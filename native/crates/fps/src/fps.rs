//! The gun closet, hardcore: the office in the first person, drawn by a small Doom engine, and through the mysterious
//! door in its south wall, Doom's first level, in the same engine and the same space: the door is in a wall of the
//! level too, and open, each side is seen through it and you walk through (a port of web/fps.js).
//!
//! It starts from Doom and works towards burrow. Doom's: the screen (320 x 200, shown at 4:3, as its pixels were a
//! fifth taller than wide; drawn at twice that, Crispy Doom's high resolution: S), the palette and its colormaps (the
//! light comes in Doom's bands and sinks into the dark with distance: its scalelight and zlight tables), the
//! projection (90 degrees), the tics (35 a second), the movement (thrust, friction, the bob), the shotgun (its frames
//! and their tics, the flash that lights the room), the status bar, the melt between screens, and a level drawn from
//! its BSP (walls with their upper, lower and middle textures, flats, the sky, sprites from the side that faces you, a
//! spectre's fuzz). Its data comes from a WAD (wad.rs). A level's game is level.rs; its player is you in the office
//! as well (health, armour, ammo, keys: the status bar is the game's).
//! Burrow's: the office (its desks, furniture, bots, what hangs on its wall, brought into Doom's palette and light);
//! shooting a bot deletes its session (the room's own shoot: the remains, the sweepers, the undo with E); the face on
//! the status bar is your avatar, in Doom's states (bloodier as your health goes, a grin after a kill, dead); the
//! hands on the shotgun have your skin.
//!
//! Sound: what is heard comes out as events (Doom's lumps by name, placed as Doom places them; the level's heard
//! where you are: from the office, through the door; muffled while it is shut).
//!
//! The mysterious door is only here, in the first person: the room seen from above has none. It is a Doom door: use
//! it or walk into it and it rises, stays up a while, comes down again (not on you). In the level it is in the wall
//! behind where you would start (`Level::door_spot`).
//!
//! The app: `Fps::new` with the WAD and the room's scene, `set_scene` at every room tick, the input as it comes,
//! `tick(dt)`, `draw` (Doom's screen) or `draw_window` (letterboxed at 4:3, with the melt), `events()` after each
//! tick, `you()` for where you stand in the room.

use crate::level::{Cmd, Level, Origin, Portal, Puff, Who};
use crate::props::Props;
use crate::rng::{Rng, jround};
use crate::scene::{Face, Scene, TS, Tex, Thing, WallKind};
use crate::wad::{Pic, Wad};
use pix::{Color, Image};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::Arc;

pub const S: i32 = 2;
/// Doom's screen at S: 640 x 400, the view 640 x 336 over the status bar
pub const SW: i32 = 320 * S;
pub const SH: i32 = 200 * S;
pub const VH: i32 = 168 * S;
const CX: f64 = (SW / 2) as f64;
const CY: f64 = (VH / 2) as f64;
const CXI: i32 = SW / 2;
const CYI: i32 = VH / 2;
const PROJ: f64 = (160 * S) as f64;
/// a dot of the room is U units
pub const U: f64 = 2.25;
pub const TICMS: f64 = 1000.0 / 35.0;
const WALLH: f64 = 40.0;
/// the mysterious door: Doom's 64 x 72, in the south wall, left of the middle (the printer's default spot)
const DOORW: f64 = 64.0;
const DOORH: f64 = 72.0;
const CEIL: f64 = WALLH * U;
/// you against the office's furniture; in the doorway, Doom's player radius
const R: f64 = 12.0;
const PR: f64 = 16.0;

/// the keys the first person knows (the browser's codes)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    W,
    A,
    S,
    D,
    Up,
    Down,
    Left,
    Right,
    ShiftLeft,
    ShiftRight,
    E,
    Space,
    Enter,
    F,
    ControlLeft,
    ControlRight,
    Escape,
}

/// a sound to play
#[derive(Clone, Debug, PartialEq)]
pub enum Sound {
    /// Doom's, a DMX lump of the WAD (`Fps::wad().lump(name)`): one at a time an origin (Doom's channels)
    Doom { name: &'static str, vol: f64, pan: f64, origin: Origin },
    /// one of burrow's own (sound.js `play`), after a while
    Synth { name: &'static str, vol: f64, pan: f64, delay_ms: f64 },
}

/// what happened, for the room and the app
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// a bot shot: the room's own shoot (its remains, the sweepers, its session deleted after)
    Shot { bot: String },
    /// E on remains: put back together
    Unshoot { rem: String },
    /// a shot fired here (room dots): the idle bots near it run off
    Alarm { x: f64, y: f64 },
    Sound(Sound),
    /// Esc: out of the first person (take the melt with `take_leave_melt`)
    Leave,
    /// you are dead (the room's mutiny: you lie there, they stand round you)
    Died,
    /// Doom's respawn after you died: in the office again (the room's mutiny ends)
    Respawn,
}

/// a shot at you from one of the room's mutineers (room.js `incoming`): from where it stands (room dots); `hit` or
/// wide, Doom's damage if it hits
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incoming {
    pub x: f64,
    pub y: f64,
    pub hit: bool,
    pub dmg: i32,
}

/// where you start in the office: room dots, the angle you look (radians, the room's y down)
#[derive(Clone, Copy, Debug)]
pub struct Start {
    pub x: f64,
    pub y: f64,
    pub a: f64,
}
/// where you stand in the room now (room.js `move`)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct You {
    pub x: f64,
    pub y: f64,
    pub a: f64,
    pub walking: bool,
}

#[derive(Clone, Default)]
pub struct Options {
    /// prefers reduced motion: no melt, no bob, no flicker
    pub still: bool,
    /// for Math.random's place
    pub seed: u64,
    /// your avatar's face, 20 x 20 (the status bar's), else Doom's own
    pub face: Option<Image>,
    /// your skin: the hands on the shotgun
    pub skin: Option<Color>,
    /// the room as it was, as large as the window: what melts away at the start
    pub snapshot: Option<Image>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum World {
    Office,
    Level,
}

/// Doom's wipe: the old screen's columns slide down, each a little after the other
pub struct Melt {
    from: Image,
    y: Vec<f64>,
    last: f64,
}
impl Melt {
    fn start(from: Image, rng: &mut Rng, now: f64) -> Melt {
        let n = 160;
        let mut y = vec![0.0; n];
        y[0] = -rng.random() * 16.0;
        for i in 1..n {
            y[i] = (-15f64).max(0f64.min(y[i - 1] + (rng.random() * 3.0).floor() - 1.0));
        }
        Melt { from, y, last: now }
    }
    /// over what is in `out` (as large as the window), at `now` (ms): false once it has all gone
    pub fn draw(&mut self, out: &mut Image, now: f64) -> bool {
        let (h, w) = (out.h as f64, out.w as f64);
        let k = h / 200.0;
        let cw = w / self.y.len() as f64;
        let mut tics = (((now - self.last) / TICMS).floor() as i32).clamp(0, 6);
        self.last += tics as f64 * TICMS;
        while tics > 0 {
            for y in self.y.iter_mut() {
                if *y < 0.0 {
                    *y += 1.0;
                } else {
                    *y += if *y < 16.0 * k { *y + 1.0 } else { 8.0 * k };
                }
            }
            tics -= 1;
        }
        let mut on = false;
        for (i, &y) in self.y.iter().enumerate() {
            let yy = jround(y).max(0.0) as i32;
            let x = (i as f64 * cw).floor() as i32;
            let ww = ((i + 1) as f64 * cw).floor() as i32 - x;
            if yy >= out.h as i32 {
                continue;
            }
            on = true;
            for r in 0..out.h as i32 - yy {
                for c in x..x + ww {
                    let p = self.from.get(c, r);
                    if p[3] > 0 {
                        out.set(c, r + yy, p);
                    }
                }
            }
        }
        on
    }
}

// ---- Doom's light: a sector's level (0..255) >> 4, plus the gun's flash; then by distance, its tables
fn clamp_l(n: i32) -> i32 {
    n.clamp(0, 15)
}
fn wall_level(ln: i32, z: f64) -> i32 {
    let l = (15 - clamp_l(ln)) * 4 - (47.min((2560.0 / z.max(1.0)).floor() as i32) >> 1);
    l.clamp(0, 31)
}
fn plane_level(ln: i32, z: f64) -> i32 {
    let l = (15 - clamp_l(ln)) * 4 - (80.0 / (127.0f64.min((z / 16.0).floor()) + 1.0)).floor() as i32;
    l.clamp(0, 31)
}
fn sprite_level(ln: i32, z: f64) -> i32 {
    wall_level(ln, z)
}
fn norm(a: f64) -> f64 {
    let a = a % (2.0 * PI);
    if a < 0.0 { a + 2.0 * PI } else { a }
}
#[inline]
fn m64(v: f64) -> usize {
    ((v.floor() as i64) & 63) as usize
}
fn modi(a: i64, n: i64) -> i64 {
    ((a % n) + n) % n
}

/// the door's window in this frame: per column the rows between top and bot (both out) where the other side shows
#[derive(Clone)]
struct Win {
    top: Vec<i16>,
    bot: Vec<i16>,
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
}
impl Win {
    fn rows(&self, x: i32) -> (i32, i32) {
        (self.top[x as usize] as i32 + 1, self.bot[x as usize] as i32 - 1)
    }
    fn out(&self, x: i32, y: i32) -> bool {
        y <= self.top[x as usize] as i32 || y >= self.bot[x as usize] as i32
    }
}

/// a sprite standing in the world, drawn from far to near, behind what is nearer
struct Vis {
    p: Arc<Pic>,
    lo: f64,
    sx: f64,
    sy: f64,
    scale: f64,
    z: f64,
    flip: bool,
    level: i32,
    id: i32,
    fuzz: bool,
    bright: bool,
}

/// who walks: from one scene's place to the next
#[derive(Clone, Copy)]
struct Seen {
    fx: f64,
    fy: f64,
    fz: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    t0: f64,
}
fn place(o: &Seen, now: f64) -> (f64, f64, f64) {
    let k = ((now - o.t0) / (1000.0 / 12.0)).min(1.0);
    (o.fx + (o.tx - o.fx) * k, o.fy + (o.ty - o.fy) * k, o.fz + (o.tz - o.fz) * k)
}

/// the floor's overlay in the palette: opaque, fullbright, or a shadow (how many light levels darker)
struct Ovl {
    w: i32,
    h: i32,
    px: Vec<u8>,
    m: Vec<u8>,
}

/// office camera: x, y in units, eye height, angle (the room's y down)
#[derive(Clone, Copy, Debug)]
struct OCam {
    x: f64,
    y: f64,
    eye: f64,
    a: f64,
}
/// level camera
#[derive(Clone, Copy, Debug)]
pub struct LCam {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub a: f64,
}

/// what the renderers read, this frame
struct Ctx3<'a> {
    wad: &'a Wad,
    door_wad: Option<&'a Wad>,
    lvl: &'a Level,
    sc: &'a Scene,
    door_open: f64,
    time: i32,
    puffs: &'a [Puff],
    now: f64,
}
impl Ctx3<'_> {
    fn units(&self) -> (f64, f64, f64) {
        (self.sc.w * U, self.sc.h * U, self.sc.wall * U)
    }
    fn door_x(&self) -> f64 {
        jround(self.sc.w * 0.3) * U
    }
    fn p(&self) -> Option<&Portal> {
        self.lvl.portal.as_ref()
    }
    // the door with the skulls (BIGDOOR7: id's in the full game, else Freedoom's, doom/door.wad)
    fn skull_door(&self) -> Option<Arc<Pic>> {
        self.wad.texture("BIGDOOR7").or_else(|| self.door_wad.and_then(|d| d.texture("BIGDOOR7"))).or_else(|| self.wad.texture("DOOR3"))
    }
    // either side of it a tall red torch (Doom's TRED), the light round it flickering
    fn torches(&self) -> [(f64, f64); 2] {
        let (_, h, _) = self.units();
        let dx = self.door_x();
        [(dx - (DOORW / 2.0 + 22.0), h - 14.0), (dx + (DOORW / 2.0 + 22.0), h - 14.0)]
    }
    // the office and the level, one space through the door: the office's south (+y) is the level's way in from the
    // door (n), the office's x along the door is the level's t (n turned left)
    fn o2l(&self, ox: f64, oy: f64) -> (f64, f64) {
        let p = self.p().unwrap();
        let (b, s) = (oy - self.units().1, ox - self.door_x());
        (p.cx + p.nx * b + p.tx * s, p.cy + p.ny * b + p.ty * s)
    }
    fn l2o(&self, lx: f64, ly: f64) -> (f64, f64) {
        let p = self.p().unwrap();
        let (rx, ry) = (lx - p.cx, ly - p.cy);
        (self.door_x() + rx * p.tx + ry * p.ty, self.units().1 + rx * p.nx + ry * p.ny)
    }
    fn a2l(&self, a: f64) -> f64 {
        let p = self.p().unwrap();
        (p.ny * a.sin() + p.ty * a.cos()).atan2(p.nx * a.sin() + p.tx * a.cos())
    }
    fn a2o(&self, al: f64) -> f64 {
        let p = self.p().unwrap();
        let (c, s) = (al.cos(), al.sin());
        (c * p.nx + s * p.ny).atan2(c * p.tx + s * p.ty)
    }
}

/// the buffers and what is kept for drawing
struct View {
    fb: Vec<u8>,
    zb: Vec<f32>,
    idb: Vec<i32>,
    ceil_clip: Vec<i16>,
    floor_clip: Vec<i16>,
    solid: Vec<u8>,
    kx: Vec<f64>,
    w_top: Vec<i16>,
    w_bot: Vec<i16>,
    w_z: Vec<f64>,
    vis: Vec<Vis>,
    extralight: i32,
    frame_no: u32,
    aim_through: bool,
    still: bool,
    // the office's
    quant: HashMap<(u64, i32, i32, i32, i32), Arc<Pic>>,
    far: Option<(String, Arc<Pic>)>,
    ovl: Option<(u64, Arc<Ovl>)>,
    lgrid: Vec<u8>,
    lgw: i32,
    lgh: i32,
    fire_flick: i32,
    torch_flick: i32,
    seen: HashMap<String, Seen>,
    sv: u64,
    /// the scene the last frame drew (what is under the crosshair is in it)
    drawn: Option<Arc<Scene>>,
    props: Props,
}

struct Masked {
    x: i32,
    z: f64,
    u: f64,
    tex: Arc<Pic>,
    ref_top: f64,
    yo: f64,
    base: usize,
    c0: i32,
    c1: i32,
}

impl View {
    fn new(still: bool) -> View {
        let n = (SW * VH) as usize;
        View {
            fb: vec![0; (SW * SH) as usize],
            zb: vec![0.0; n],
            idb: vec![0; n],
            ceil_clip: vec![0; SW as usize],
            floor_clip: vec![0; SW as usize],
            solid: vec![0; SW as usize],
            kx: (0..SW).map(|x| ((x as f64 + 0.5 - CX) / PROJ) as f32 as f64).collect(), // (a Float32Array in fps.js)
            w_top: vec![0; SW as usize],
            w_bot: vec![0; SW as usize],
            w_z: vec![0.0; SW as usize],
            vis: Vec::new(),
            extralight: 0,
            frame_no: 0,
            aim_through: false,
            still,
            quant: HashMap::new(),
            far: None,
            ovl: None,
            lgrid: vec![0; 1],
            lgw: 1,
            lgh: 1,
            fire_flick: 32,
            torch_flick: 0,
            seen: HashMap::new(),
            sv: u64::MAX,
            drawn: None,
            props: Props::new(),
        }
    }

    // ---- burrow's pictures in Doom's palette: opaque or not, fullbright where they shone (alpha GLOW), kept per picture
    fn pic(&mut self, wad: &Wad, t: &Tex, sx: f64, sy: f64, sw: f64, sh: f64) -> Arc<Pic> {
        let (w, h) = ((jround(sw) as i32).max(1), (jround(sh) as i32).max(1));
        let (x0, y0) = (jround(sx) as i32, jround(sy) as i32);
        let key = (t.id, x0, y0, w, h);
        if let Some(p) = self.quant.get(&key) {
            return p.clone();
        }
        if self.quant.len() > 20000 {
            self.quant.clear();
        }
        let n = (w * h) as usize;
        let mut p = Pic { w, h, lo: 0, to: 0, px: vec![0; n], op: vec![0; n], masked: false };
        for j in 0..h {
            for i in 0..w {
                let c = t.img.get(x0 + i, y0 + j);
                if c[3] < 128 {
                    continue;
                }
                let k = (j * w + i) as usize;
                p.px[k] = wad.nearest(c[0], c[1], c[2]);
                p.op[k] = if c[3] == crate::scene::GLOW { 2 } else { 1 };
            }
        }
        let p = Arc::new(p);
        self.quant.insert(key, p.clone());
        p
    }
    fn face_pic(&mut self, wad: &Wad, f: &Option<Face>) -> Option<Arc<Pic>> {
        f.as_ref().map(|f| self.pic(wad, &f.tex, f.sx, f.sy, f.sw, f.sh))
    }

    fn draw_vis(&mut self, v: &Vis, cmap: &[u8], win: Option<&Win>) {
        let (p, s) = (&v.p, v.scale);
        if s <= 0.0 {
            return;
        }
        let left = v.sx - v.lo * s; // a mirrored rotation keeps its offset (Doom's R_ProjectSprite)
        let top = v.sy;
        let x0 = (left.floor() as i32).max(0);
        let x1 = ((left + p.w as f64 * s).ceil() as i32 - 1).min(SW - 1);
        let y0 = (top.floor() as i32).max(0);
        let y1 = ((top + p.h as f64 * s).ceil() as i32 - 1).min(VH - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }
        let base = v.level as usize * 256;
        let inv = 1.0 / s;
        for x in x0..=x1 {
            let mut u = ((x as f64 + 0.5 - left) * inv).floor() as i32;
            if u < 0 || u >= p.w {
                continue;
            }
            if v.flip {
                u = p.w - 1 - u;
            }
            for y in y0..=y1 {
                let i = (y * SW + x) as usize;
                if v.z >= self.zb[i] as f64 || win.is_some_and(|w| w.out(x, y)) {
                    continue;
                }
                let t = ((y as f64 + 0.5 - top) * inv).floor() as i32;
                if t < 0 || t >= p.h {
                    continue;
                }
                let k = (t * p.w + u) as usize;
                let o = p.op[k];
                if o == 0 {
                    continue;
                }
                if v.fuzz {
                    let jy = (y + ((((x * 7 + y * 3) as u32).wrapping_add(self.frame_no) % 3) as i32 - 1) * S).clamp(0, VH - 1);
                    let j = (jy * SW + x) as usize;
                    self.fb[i] = cmap[6 * 256 + self.fb[j] as usize];
                } else {
                    self.fb[i] = if o == 2 || v.bright { p.px[k] } else { cmap[base + p.px[k] as usize] };
                }
                if v.id != 0 {
                    self.idb[i] = v.id;
                }
            }
        }
    }
    fn draw_vis_all(&mut self, cmap: &[u8], win: Option<&Win>) {
        let mut vis = std::mem::take(&mut self.vis);
        vis.sort_by(|a, b| b.z.total_cmp(&a.z));
        for v in &vis {
            self.draw_vis(v, cmap, win);
        }
        self.vis = vis;
    }

    // the office's light in 32-unit cells, as Doom's sectors would have it, once a frame: 192, 224 under a lamp, the
    // fire's flicker; by the door the torches' (Doom's FireFlicker: up to three steps of 16 down from 208, every 4 tics)
    fn light_grid(&mut self, r: &Ctx3) {
        let (w, h, _) = r.units();
        self.lgw = (w / 32.0).ceil() as i32 + 1;
        self.lgh = (h / 32.0).ceil() as i32 + 1;
        let n = (self.lgw * self.lgh) as usize;
        if self.lgrid.len() != n {
            self.lgrid = vec![0; n];
        }
        let dx = r.door_x();
        for cy in 0..self.lgh {
            for cx in 0..self.lgw {
                let (x, y) = ((cx * 32 + 16) as f64, (cy * 32 + 16) as f64);
                let mut l: i32 = if (x / 64.0).floor() as i64 % 3 == 1 && (y / 64.0).floor() as i64 % 3 == 1 { 224 } else { 192 };
                for li in &r.sc.lights {
                    let (ddx, ddy) = (x - li.x * U, y - li.y * U);
                    if ddx * ddx + ddy * ddy < 9000.0 {
                        l = l.max(if li.muzzle { 255 } else if li.flicker && !self.still { 192 + (self.fire_flick & 48) } else { 224 });
                    }
                }
                if r.p().is_some() && (x - dx).abs() < 112.0 && y > h - 112.0 {
                    l = 208 - if self.still { 0 } else { self.torch_flick * 16 };
                }
                self.lgrid[(cy * self.lgw + cx) as usize] = l as u8;
            }
        }
    }
    fn office_light(&self, x: f64, y: f64) -> i32 {
        let cy = ((y / 32.0).floor() as i32).clamp(0, self.lgh - 1);
        let cx = ((x / 32.0).floor() as i32).clamp(0, self.lgw - 1);
        self.lgrid[(cy * self.lgw + cx) as usize] as i32
    }
    // the far wall: STARTAN2 along it, then what hangs on it, each where the room has it (its doors Doom's DOOR3 with
    // its machine's key colour beside it, blue, yellow, red; the rest burrow's own pictures), kept until it changes
    fn far_wall(&mut self, r: &Ctx3) -> Arc<Pic> {
        let sc = r.sc;
        let key = format!("{}|{}", sc.wkey, sc.w);
        if let Some((k, t)) = &self.far
            && *k == key {
                return t.clone();
            }
        let wad = r.wad;
        let (w, h) = (jround(sc.w * U) as i32, jround(CEIL) as i32);
        let base = wad.texture("STARTAN2").or_else(|| wad.texture("STARTAN3"));
        let n = (w.max(0) * h) as usize;
        let mut t = Pic { w, h, lo: 0, to: 0, px: vec![0; n], op: vec![1; n], masked: false };
        for y in 0..h {
            for x in 0..w {
                t.px[(y * w + x) as usize] = base.as_ref().map_or(96, |b| b.px[((y % b.h) * b.w + (x % b.w)) as usize]);
            }
        }
        let z_of = |yb: f64| (sc.wall - yb) * (WALLH / sc.wall);
        // a picture into a rectangle of the wall (in units), stretched
        let put = |t: &mut Pic, src: &Pic, x0: f64, y0: f64, ww: f64, hh: f64, f: Option<&dyn Fn(u8) -> u8>| {
            let mut y = 0.0;
            while y < hh {
                let mut x = 0.0;
                while x < ww {
                    let (tx, ty) = ((x0 + x).floor() as i32, (y0 + y).floor() as i32);
                    if tx >= 0 && ty >= 0 && tx < t.w && ty < t.h {
                        let (sx, sy) = ((x / ww * src.w as f64).floor() as i32, (y / hh * src.h as f64).floor() as i32);
                        let k = (sy * src.w + sx) as usize;
                        if src.op.get(k).is_some_and(|&o| o != 0) {
                            t.px[(ty * t.w + tx) as usize] = f.map_or(src.px[k], |f| f(src.px[k]));
                        }
                    }
                    x += 1.0;
                }
                y += 1.0;
            }
        };
        let strips = ["DOORBLU", "DOORYEL", "DOORRED"];
        let mut n = 0;
        let cmap = &wad.cmap;
        for it in &sc.items {
            let (zt, zb0) = (z_of(it.y0), z_of(it.y1).max(0.0));
            let (x0, x1, y0, hh) = (it.x0 * U, it.x1 * U, (WALLH - zt) * U, (zt - zb0) * U);
            if let WallKind::Door { ok, .. } = it.kind {
                let dr = wad.texture("DOOR3");
                let trak = wad.texture("DOORTRAK");
                let strip = wad.texture(strips.get(n).copied().unwrap_or("DOORTRAK"));
                n += 1;
                if let Some(tr) = &trak {
                    put(&mut t, tr, x0 - 8.0, y0, 8.0, hh, None);
                    put(&mut t, tr, x1, y0, 8.0, hh, None);
                }
                if let (Some(st), true) = (&strip, n <= 3) {
                    put(&mut t, st, x0 - 8.0, y0, 4.0, hh, None);
                    put(&mut t, st, x1 + 4.0, y0, 4.0, hh, None);
                }
                if let Some(d) = &dr {
                    let dark = |c: u8| cmap[20 * 256 + c as usize]; // out of reach: dark
                    put(&mut t, d, x0, y0, x1 - x0, hh, if ok { None } else { Some(&dark) });
                }
            } else {
                let p = self.pic(wad, &sc.wall_pic, it.x0 * TS, (WALLH - zt) * TS, (it.x1 - it.x0) * TS, (zt - zb0) * TS);
                put(&mut t, &p, x0, y0, x1 - x0, hh, None);
            }
        }
        let t = Arc::new(t);
        self.far = Some((key, t.clone()));
        t
    }
    fn overlay(&mut self, r: &Ctx3) -> Arc<Ovl> {
        let c = &r.sc.floor;
        if let Some((id, o)) = &self.ovl
            && *id == c.id {
                return o.clone();
            }
        let (w, h) = (c.img.w as i32, c.img.h as i32);
        let n = (w * h) as usize;
        let mut o = Ovl { w, h, px: vec![0; n], m: vec![0; n] };
        for (i, p) in c.img.px.iter().enumerate() {
            let a = p[3];
            if a == 0 {
                continue;
            }
            if a >= 250 {
                o.px[i] = r.wad.nearest(p[0], p[1], p[2]);
                o.m[i] = if a == crate::scene::GLOW { 2 } else { 1 };
            } else {
                o.m[i] = 3 + jround(a as f64 / 255.0 * 10.0) as u8; // a shadow: so many levels darker
            }
        }
        let o = Arc::new(o);
        self.ovl = Some((c.id, o.clone()));
        o
    }
    fn win_of(&self) -> Option<Win> {
        let (mut x0, mut x1, mut y0, mut y1) = (SW, -1, VH, -1);
        for x in 0..SW {
            let (t, b) = (self.w_top[x as usize] as i32, self.w_bot[x as usize] as i32);
            if b - t > 1 {
                x0 = x0.min(x);
                x1 = x;
                y0 = y0.min(t + 1);
                y1 = y1.max(b - 1);
            }
        }
        if x1 < 0 {
            return None;
        }
        Some(Win { top: self.w_top.clone(), bot: self.w_bot.clone(), x0, x1, y0, y1 })
    }
    fn win_z(&mut self, w: &Win, z: Option<&[f64]>) {
        for x in w.x0..=w.x1 {
            let (a, b) = w.rows(x);
            for y in a..=b {
                self.zb[(y * SW + x) as usize] = z.map_or(1e9, |z| z[x as usize] as f32);
            }
        }
    }

    // ---- the office from cam (units); within win only, seen through the door from the level
    fn render_office(&mut self, r: &Ctx3, cam: OCam, win: Option<&Win>) {
        let sc = r.sc;
        let wad = r.wad;
        let cmap = &wad.cmap[..];
        let (w, h, wall) = r.units();
        let (eye, top) = (cam.eye, CEIL);
        let (dx, dy) = (cam.a.cos(), cam.a.sin());
        let (rx, ry, px, py) = (-dy, dx, cam.x, cam.y);
        let fl = wad.flat("FLAT5_2").or_else(|| wad.flat("FLOOR4_8"));
        let ce = wad.flat("CEIL3_5");
        let lamp = wad.flat("TLITE6_4").or_else(|| ce.clone());
        let ov = self.overlay(r);
        let (xs, xe) = win.map_or((0, SW - 1), |w| (w.x0, w.x1));
        let extralight = self.extralight;
        // floor and ceiling, a row at a time
        let (ys, ye) = win.map_or((0, VH - 1), |w| (w.y0, w.y1));
        for y in ys..=ye {
            let below = y as f64 + 0.5 - CY;
            let is_floor = below > 0.0;
            let d = 4000f64.min((if is_floor { eye } else { top - eye }) * PROJ / below.abs());
            for x in xs..=xe {
                if win.is_some_and(|w| w.out(x, y)) {
                    continue;
                }
                let i = (y * SW + x) as usize;
                let kx = self.kx[x as usize];
                let (wx, wy) = (px + (dx + rx * kx) * d, py + (dy + ry * kx) * d);
                let ln = (self.office_light(wx, wy) >> 4) + extralight;
                let mut lv = plane_level(ln, d);
                let c;
                if is_floor {
                    let mut cc = fl.as_ref().map_or(96, |f| f[(m64(wy) << 6) + m64(wx)]);
                    let (ox, oy) = ((wx / U * 2.0).floor() as i32, (wy / U * 2.0).floor() as i32);
                    if ox >= 0 && oy >= 0 && ox < ov.w && oy < ov.h {
                        let k = (oy * ov.w + ox) as usize;
                        let m = ov.m[k];
                        if m == 1 {
                            cc = ov.px[k];
                        } else if m == 2 {
                            cc = ov.px[k];
                            lv = 0;
                        } else if m >= 3 {
                            lv = 31.min(lv + m as i32 - 3);
                        }
                    }
                    c = cc;
                } else {
                    let lit = (wx / 64.0).floor() as i64 % 3 == 1 && (wy / 64.0).floor() as i64 % 3 == 1;
                    let f = if lit { &lamp } else { &ce };
                    c = f.as_ref().map_or(4, |f| f[(m64(wy) << 6) + m64(wx)]);
                }
                self.fb[i] = cmap[lv as usize * 256 + c as usize];
                self.zb[i] = d as f32;
                self.idb[i] = 0;
            }
        }
        // the four walls; in the south one the door, risen door.open into the lintel, the level under it
        let far = self.far_wall(r);
        let side = wad.texture("STARTAN3").unwrap_or_else(|| far.clone());
        let dt = r.skull_door();
        let door_x = r.door_x();
        let has_p = r.p().is_some();
        if win.is_none() {
            self.w_top.fill(VH as i16);
            self.w_bot.fill(-1);
        }
        for x in xs..=xe {
            let (ylo, yhi) = win.map_or((0, VH - 1), |w| w.rows(x));
            if ylo > yhi {
                continue;
            }
            let kx = self.kx[x as usize];
            let (rdx, rdy) = (dx + rx * kx, dy + ry * kx);
            let tx = if rdx > 0.0 {
                (w - px) / rdx
            } else if rdx < 0.0 {
                -px / rdx
            } else {
                1e9
            };
            let ty = if rdy > 0.0 {
                (h - py) / rdy
            } else if rdy < 0.0 {
                (wall - py) / rdy
            } else {
                1e9
            };
            let (t, u, tt, fake, hx, hy);
            let mut is_door = false;
            let far_side;
            if ty < tx {
                t = ty;
                hx = px + rdx * t;
                hy = if rdy < 0.0 { wall } else { h };
                u = hx;
                far_side = rdy < 0.0;
                tt = if far_side { far.clone() } else { side.clone() };
                fake = -1;
                if win.is_none() && rdy > 0.0 && dt.is_some() && has_p && (hx - door_x).abs() < DOORW / 2.0 {
                    is_door = true;
                }
            } else {
                t = tx;
                hx = if rdx > 0.0 { w } else { 0.0 };
                hy = py + rdy * t;
                u = hy;
                tt = side.clone();
                far_side = false;
                fake = 1;
            }
            if t <= 0.0 {
                continue;
            }
            let sg = |v: f64| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
            let ln = (self.office_light(hx - sg(hx - w / 2.0) * 4.0, hy - sg(hy - (wall + h) / 2.0) * 4.0) >> 4) + extralight + fake;
            let base = wall_level(ln, t) as usize * 256;
            let yt = CY - (top - eye) * PROJ / t;
            let yb = CY + eye * PROJ / t;
            let col = if far_side { (u.floor() as i32).clamp(0, tt.w - 1) } else { modi(u.floor() as i64, tt.w as i64) as i32 };
            let (mut wy0, mut wy1) = (VH, -1);
            let y1 = yhi.min((yb - 0.5).floor() as i32);
            let mut y = ylo.max((yt - 0.5).ceil() as i32);
            while y <= y1 {
                let z = eye - (y as f64 + 0.5 - CY) * t / PROJ;
                let i = (y * SW + x) as usize;
                let c;
                if is_door && z < DOORH {
                    if z < r.door_open {
                        // open: the level shows here
                        if y < wy0 {
                            wy0 = y;
                        }
                        wy1 = y;
                        y += 1;
                        continue;
                    }
                    let d = dt.as_ref().unwrap();
                    let du = ((hx - (door_x - DOORW / 2.0)) / DOORW * d.w as f64).floor() as i32;
                    let dv = ((DOORH - (z - r.door_open)) / DOORH * d.h as f64).floor() as i32;
                    c = d.px[(dv.clamp(0, d.h - 1) * d.w + (d.w - 1 - du).clamp(0, d.w - 1)) as usize];
                } else {
                    c = tt.px[(modi((top - z).floor() as i64, tt.h as i64) as i32 * tt.w + col) as usize];
                }
                self.fb[i] = cmap[base + c as usize];
                self.zb[i] = t as f32;
                self.idb[i] = 0;
                y += 1;
            }
            if wy1 >= 0 {
                self.w_top[x as usize] = (wy0 - 1) as i16;
                self.w_bot[x as usize] = (wy1 + 1) as i16;
                self.w_z[x as usize] = t;
            }
        }
        // through the open door: the level, as it is from here
        if win.is_none() && has_p && r.door_open > 0.0
            && let Some(w2) = self.win_of() {
                let (lx, ly) = r.o2l(px, py);
                self.win_z(&w2, None);
                let floor = r.p().unwrap().floor;
                self.render_level(r, LCam { x: lx, y: ly, z: floor + eye, a: r.a2l(cam.a) }, Some(&w2));
                let wz = self.w_z.clone();
                self.win_z(&w2, Some(&wz));
            }
        self.aim_through = win.is_none() && (self.w_top[CXI as usize] as i32) < CYI && CYI < self.w_bot[CXI as usize] as i32;
        // the blocks (desks and what stands as one): the face a ray meets first, and their tops seen from above
        for b in &sc.boxes {
            self.office_box(r, b, px, py, dx, dy, rx, ry, eye, win);
        }
        // the pictures and the models; who walks is on the way from one tick's place to the next
        if sc.v != self.sv {
            self.sv = sc.v;
            let mut next = HashMap::new();
            for th in &sc.things {
                let Some(k) = &th.key else { continue };
                let at = self.seen.get(k).map_or((th.x, th.y, th.z), |o| place(o, r.now));
                let s = if (at.0 - th.x).hypot(at.1 - th.y) > 30.0 {
                    Seen { fx: th.x, fy: th.y, fz: th.z, tx: th.x, ty: th.y, tz: th.z, t0: r.now }
                } else {
                    Seen { fx: at.0, fy: at.1, fz: at.2, tx: th.x, ty: th.y, tz: th.z, t0: r.now }
                };
                next.insert(k.clone(), s);
            }
            self.seen = next;
        }
        self.vis.clear();
        for (n, th) in sc.things.iter().enumerate() {
            let o = th.key.as_ref().and_then(|k| self.seen.get(k)).copied();
            let at = o.map_or((th.x, th.y, th.z), |o| place(&o, r.now));
            self.office_thing(r, th, at, n as i32 + 1, px, py, dx, dy, rx, ry, eye, o, win);
        }
        for f in r.puffs {
            let ln = (self.office_light(f.x, f.y) >> 4) + extralight;
            self.puff_vis(r, f, px, py, dx, dy, rx, ry, eye, ln);
        }
        if has_p {
            // the torches by the door, their flames burning (four frames, fullbright)
            for (tx, ty) in r.torches() {
                let (vx, vy) = (tx - px, ty - py);
                let z = vx * dx + vy * dy;
                if z <= 8.0 {
                    continue;
                }
                let Some(sf) = wad.sprite_frame("TRED", ((r.time / 4) % 4) as usize, 1) else { continue };
                let Some(p) = sf.pic else { continue };
                self.vis.push(Vis { lo: p.lo as f64, sx: CX + (vx * rx + vy * ry) * PROJ / z, sy: CY - (p.to as f64 - eye) * PROJ / z, scale: PROJ / z, z, flip: sf.flip, level: 0, id: 0, fuzz: false, bright: true, p });
            }
        }
        // the level's fireballs that came through the door
        for o in &r.lvl.mobjs {
            if !o.office || !has_p {
                continue;
            }
            let (ox, oy) = r.l2o(o.x, o.y);
            let (vx, vy) = (ox - px, oy - py);
            let z = vx * dx + vy * dy;
            if z <= 4.0 {
                continue;
            }
            let Some(sf) = wad.sprite_frame(o.sprite, (o.frame - b'A') as usize, 1) else { continue };
            let Some(p) = sf.pic else { continue };
            let fl = r.p().unwrap().floor;
            self.vis.push(Vis { lo: p.lo as f64, sx: CX + (vx * rx + vy * ry) * PROJ / z, sy: CY - (o.z - fl + p.to as f64 - eye) * PROJ / z, scale: PROJ / z, z, flip: sf.flip, level: 0, id: 0, fuzz: false, bright: true, p });
        }
        self.draw_vis_all(cmap, win);
    }
    #[allow(clippy::too_many_arguments)]
    fn office_box(&mut self, r: &Ctx3, b: &crate::scene::Block, px: f64, py: f64, dx: f64, dy: f64, rx: f64, ry: f64, eye: f64, win: Option<&Win>) {
        let wad = r.wad;
        let cmap = &wad.cmap[..];
        let (x0, x1, y0, y1, z0, z1) = (b.x0 * U, b.x1 * U, b.y0 * U, b.y1 * U, b.z0 * U, b.z1 * U);
        if px > x0 && px < x1 && py > y0 && py < y1 {
            return;
        }
        let (mut c0, mut c1) = (SW, -1);
        for (cx, cy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
            let (vx, vy) = (cx - px, cy - py);
            let d = vx * dx + vy * dy;
            if d < 0.5 {
                c0 = 0;
                c1 = SW - 1;
                break;
            }
            let sx = CX + (vx * rx + vy * ry) * PROJ / d;
            c0 = c0.min(sx.floor() as i32);
            c1 = c1.max(sx.ceil() as i32);
        }
        c0 = c0.max(win.map_or(0, |w| w.x0));
        c1 = c1.min(win.map_or(SW - 1, |w| w.x1));
        if c0 > c1 {
            return;
        }
        let ff = self.face_pic(wad, &b.front);
        let fbk = self.face_pic(wad, &b.back);
        let fs = self.face_pic(wad, &b.side);
        let fw = self.face_pic(wad, &b.west);
        let fe = self.face_pic(wad, &b.east);
        let ft = self.face_pic(wad, &b.top);
        let g = |f: &Option<Face>| f.as_ref().is_some_and(|f| f.glow);
        let extralight = self.extralight;
        for x in c0..=c1 {
            let (ylo, yhi) = win.map_or((0, VH - 1), |w| w.rows(x));
            if ylo > yhi {
                continue;
            }
            let kx = self.kx[x as usize];
            let (rdx, rdy) = (dx + rx * kx, dy + ry * kx);
            let (mut tn, mut tx, mut ax) = (f64::NEG_INFINITY, f64::INFINITY, 0);
            if rdx != 0.0 {
                let (a1, a2) = ((x0 - px) / rdx, (x1 - px) / rdx);
                if a1.min(a2) > tn {
                    tn = a1.min(a2);
                    ax = 1;
                }
                tx = tx.min(a1.max(a2));
            } else if px < x0 || px > x1 {
                continue;
            }
            if rdy != 0.0 {
                let (a1, a2) = ((y0 - py) / rdy, (y1 - py) / rdy);
                if a1.min(a2) > tn {
                    tn = a1.min(a2);
                    ax = 2;
                }
                tx = tx.min(a1.max(a2));
            } else if py < y0 || py > y1 {
                continue;
            }
            if tx < tn || tn <= 1.0 {
                continue;
            }
            let (hx, hy) = (px + rdx * tn, py + rdy * tn);
            let (p, glow, mut u, fake);
            if ax == 1 {
                p = if rdx > 0.0 { fw.as_ref() } else { fe.as_ref() }.or(fs.as_ref());
                glow = g(&b.side);
                u = (hy - y0) / (y1 - y0);
                fake = 1;
                if rdx < 0.0 {
                    u = 1.0 - u;
                }
            } else if rdy < 0.0 {
                p = ff.as_ref();
                glow = g(&b.front);
                u = (hx - x0) / (x1 - x0);
                fake = -1;
            } else {
                p = fbk.as_ref();
                glow = g(&b.back);
                u = 1.0 - (hx - x0) / (x1 - x0);
                fake = -1;
            }
            let u = u.clamp(0.0, 0.999);
            let ln = (self.office_light(hx, hy) >> 4) + extralight + fake;
            let base = wall_level(ln, tn) as usize * 256;
            let yt = CY - (z1 - eye) * PROJ / tn;
            let yb = CY - (z0 - eye) * PROJ / tn;
            if let Some(p) = p {
                let ye = yhi.min((yb - 0.5).floor() as i32);
                for y in ylo.max((yt - 0.5).ceil() as i32)..=ye {
                    let i = (y * SW + x) as usize;
                    if tn >= self.zb[i] as f64 {
                        continue;
                    }
                    let z = eye - (y as f64 + 0.5 - CY) * tn / PROJ;
                    let k = (((z1 - z) / (z1 - z0) * p.h as f64).floor() as i32).clamp(0, p.h - 1) * p.w + (u * p.w as f64).floor() as i32;
                    let o = p.op[k as usize];
                    if o == 0 {
                        continue;
                    }
                    let c = p.px[k as usize];
                    self.fb[i] = if glow || o == 2 { c } else { cmap[base + c as usize] };
                    self.zb[i] = tn as f32;
                    self.idb[i] = 0;
                }
            }
            let Some(tp) = ft.as_ref() else { continue };
            if eye <= z1 {
                continue;
            }
            let hh = eye - z1;
            let ya = ylo.max((CY + hh * PROJ / tx - 0.5).ceil() as i32);
            let yb2 = yhi.min((CY + hh * PROJ / tn - 0.5).floor() as i32);
            let tglow = g(&b.top);
            for y in ya..=yb2 {
                let d = hh * PROJ / (y as f64 + 0.5 - CY);
                let i = (y * SW + x) as usize;
                if d < tn || d > tx || d >= self.zb[i] as f64 {
                    continue;
                }
                let (wx, wy) = (px + rdx * d, py + rdy * d);
                let k = (((wy - y0) / (y1 - y0) * tp.h as f64).floor() as i32).clamp(0, tp.h - 1) * tp.w + (((wx - x0) / (x1 - x0) * tp.w as f64).floor() as i32).clamp(0, tp.w - 1);
                let o = tp.op[k as usize];
                if o == 0 {
                    continue;
                }
                let c = tp.px[k as usize];
                self.fb[i] = if tglow || o == 2 { c } else { cmap[plane_level((self.office_light(wx, wy) >> 4) + extralight, d) as usize * 256 + c as usize] };
                self.zb[i] = d as f32;
                self.idb[i] = 0;
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn office_thing(&mut self, r: &Ctx3, th: &Thing, at: (f64, f64, f64), n: i32, px: f64, py: f64, dx: f64, dy: f64, rx: f64, ry: f64, eye: f64, o: Option<Seen>, win: Option<&Win>) {
        let (x, y) = (at.0 * U, at.1 * U);
        let (vx, vy) = (x - px, y - py);
        let z = vx * dx + vy * dy;
        if z < 12.0 {
            return;
        }
        let (tex, mut sx0, mut sy0, mut sw, mut sh, mut ax, mut ay, mut res);
        let mut flip = if th.face != 0.0 && th.fig.is_none() { th.face * -dy < 0.0 } else { th.flip };
        if let Some(fig) = &th.fig {
            let (mx2, my2) = o.map_or((0.0, 0.0), |o| (o.tx - o.fx, o.ty - o.fy));
            let fa = th.facing.unwrap_or(if !th.seated && mx2.hypot(my2) > 0.4 {
                my2.atan2(mx2)
            } else if th.face < 0.0 {
                PI
            } else {
                0.0
            });
            let k = modi(jround(((-vy).atan2(-vx) - fa) / (PI / 4.0)) as i64 % 8, 8) as usize;
            let f = self.props.figure(fig, k);
            tex = f.tex;
            sx0 = 0.0;
            sy0 = 0.0;
            sw = f.w;
            sh = f.h;
            ax = f.ax;
            ay = f.ay;
            res = f.res;
            flip = false;
        } else {
            let Some(pc) = &th.pic else { return };
            tex = pc.tex.clone();
            sx0 = pc.sx;
            sy0 = pc.sy;
            sw = pc.sw;
            sh = pc.sh;
            ax = th.ax;
            ay = th.ay;
            res = if th.res > 0.0 { th.res } else { 1.0 };
        }
        let _ = (&mut sx0, &mut sy0, &mut sw, &mut sh, &mut ax, &mut ay, &mut res);
        let p = self.pic(r.wad, &tex, sx0, sy0, sw, sh);
        let scale = PROJ / z * U / res;
        let lat = vx * rx + vy * ry;
        let ln = (self.office_light(x, y) >> 4) + self.extralight;
        let shootable = th.bot.is_some() || th.rem.is_some() || th.piece.is_some() || th.name.is_some();
        self.vis.push(Vis {
            p,
            lo: ax,
            sx: CX + lat * PROJ / z,
            sy: CY - ((at.2 * U) + ay * U / res - eye) * PROJ / z,
            scale,
            z,
            flip,
            level: sprite_level(ln, z),
            id: if win.is_none() && shootable { n } else { 0 },
            fuzz: th.dissolve, // going up in the booth's light: Doom's fuzz
            bright: false,
        });
    }
    // a shot's puff: Doom's PUFF, rising, its four frames in sixteen tics
    #[allow(clippy::too_many_arguments)]
    fn puff_vis(&mut self, r: &Ctx3, f: &Puff, px: f64, py: f64, dx: f64, dy: f64, rx: f64, ry: f64, eye: f64, ln: i32) {
        let (vx, vy) = (f.x - px, f.y - py);
        let z = vx * dx + vy * dy;
        if z < 8.0 {
            return;
        }
        let fr = if f.blood { r.wad.sprite_frame("BLUD", (2 - 2.min(f.t >> 2)) as usize, 1) } else { r.wad.sprite_frame("PUFF", 3.min(f.t >> 2) as usize, 1) };
        let Some(fr) = fr else { return };
        let Some(p) = fr.pic else { return };
        let scale = PROJ / z;
        self.vis.push(Vis {
            lo: p.lo as f64,
            sx: CX + (vx * rx + vy * ry) * scale,
            sy: CY - (f.z + f.t as f64 * 0.5 + p.to as f64 - eye) * scale,
            scale,
            z,
            flip: false,
            level: sprite_level(ln, z),
            id: 0,
            fuzz: false,
            bright: !f.blood && f.t < 4,
            p,
        });
    }

    // ---- a Doom level, from its BSP, from cam: front to back, a column at a time; flats and sky in what each wall
    // leaves open. Within win only, seen through the door from the office: what is beyond the door's wall
    fn render_level(&mut self, r: &Ctx3, v: LCam, win: Option<&Win>) {
        let l = r.lvl;
        let (dx, dy) = (v.a.cos(), v.a.sin());
        let (rx, ry) = (dy, -dx);
        let mut solid_n = 0;
        if let Some(w) = win {
            for x in 0..SW as usize {
                self.ceil_clip[x] = w.top[x];
                self.floor_clip[x] = w.bot[x];
                self.solid[x] = if self.floor_clip[x] as i32 - self.ceil_clip[x] as i32 <= 1 { 1 } else { 0 };
                solid_n += self.solid[x] as i32;
            }
        } else {
            // (what nothing draws on is black, not the last frame)
            self.ceil_clip.fill(-1);
            self.floor_clip.fill(VH as i16);
            self.solid.fill(0);
            self.zb.fill(1e9);
            self.idb.fill(0);
            self.w_top.fill(VH as i16);
            self.w_bot.fill(-1);
            self.fb[..(SW * VH) as usize].fill(0);
        }
        let mut pass = LvlPass {
            v: self,
            r,
            cam: v,
            dx,
            dy,
            rx,
            ry,
            win,
            masked: Vec::new(),
            solid_n,
            sky: r.wad.texture("SKY1").or_else(|| r.wad.texture("SKY2")).or_else(|| r.wad.texture("SKY3")),
            dt: r.skull_door(),
        };
        let root = if l.map.nodes.is_empty() { 0x8000 } else { (l.map.nodes.len() - 1) as u16 };
        pass.node(root);
        let masked = std::mem::take(&mut pass.masked);
        // what is left open (beyond the map's edge): black
        for x in 0..SW {
            let xu = x as usize;
            if self.solid[xu] == 0 {
                for y in self.ceil_clip[xu] as i32 + 1..self.floor_clip[xu] as i32 {
                    self.fb[(y * SW + x) as usize] = 0;
                }
            }
        }
        // through the open door: the office, as it is from here
        if win.is_none() && r.p().is_some() && r.door_open > 0.0
            && let Some(w2) = self.win_of() {
                let (ox, oy) = r.l2o(v.x, v.y);
                self.win_z(&w2, None);
                let fl = r.p().unwrap().floor;
                self.render_office(r, OCam { x: ox, y: oy, eye: v.z - fl, a: r.a2o(v.a) }, Some(&w2));
                let wz = self.w_z.clone();
                self.win_z(&w2, Some(&wz));
            }
        // masked middle textures (grates, bars), then the sprites, far to near
        let cmap = &r.wad.cmap[..];
        for mk in &masked {
            let tx = &mk.tex;
            let col = modi(mk.u.floor() as i64, tx.w as i64) as i32;
            let s = mk.z / PROJ;
            for y in 0.max(mk.c0 + 1)..VH.min(mk.c1) {
                let wz = v.z - (y as f64 + 0.5 - CY) * s;
                let vv = (mk.ref_top - wz + mk.yo).floor() as i32;
                let i = (y * SW + mk.x) as usize;
                if vv < 0 || vv >= tx.h || mk.z >= self.zb[i] as f64 {
                    continue;
                }
                let k = (vv * tx.w + col) as usize;
                if tx.op[k] == 0 {
                    continue;
                }
                self.fb[i] = cmap[mk.base + tx.px[k] as usize];
                self.zb[i] = mk.z as f32;
            }
        }
        self.vis.clear();
        for o in &l.mobjs {
            if o.office || (win.is_some() && l.beyond(o.x, o.y) <= 0.0) {
                continue;
            }
            let (vx, vy) = (o.x - v.x, o.y - v.y);
            let z = vx * dx + vy * dy;
            if z < 4.0 {
                continue;
            }
            let fr = if o.sprite.is_empty() { o.d.s } else { o.sprite };
            let f = (o.frame - b'A') as usize;
            let ang = vy.atan2(vx);
            let rot = ((norm(ang - o.angle + PI + PI / 8.0) / (PI / 4.0)).floor() as usize) % 8 + 1;
            let Some(sf) = r.wad.sprite_frame(fr, f, rot) else { continue };
            let Some(p) = sf.pic else { continue };
            let scale = PROJ / z;
            let sec = &l.map.sectors[if o.missile.is_some() { l.sector_at(o.x, o.y) } else { o.sector }];
            self.vis.push(Vis {
                lo: p.lo as f64,
                sx: CX + (vx * rx + vy * ry) * scale,
                sy: CY - (o.z + p.to as f64 - v.z) * scale,
                scale,
                z,
                flip: sf.flip,
                level: sprite_level((sec.light >> 4) + self.extralight, z),
                id: 0,
                fuzz: o.d.shadow,
                bright: o.bright,
                p,
            });
        }
        for f in &l.puffs {
            if win.is_some() && l.beyond(f.x, f.y) <= 0.0 {
                continue;
            }
            let ln = (l.map.sectors[l.sector_at(f.x, f.y)].light >> 4) + self.extralight;
            self.puff_vis(r, f, v.x, v.y, dx, dy, rx, ry, v.z, ln);
        }
        self.draw_vis_all(cmap, win);
    }
}

/// one walk of the BSP for one view
struct LvlPass<'a, 'b> {
    v: &'a mut View,
    r: &'a Ctx3<'b>,
    cam: LCam,
    dx: f64,
    dy: f64,
    rx: f64,
    ry: f64,
    win: Option<&'a Win>,
    masked: Vec<Masked>,
    solid_n: i32,
    sky: Option<Arc<Pic>>,
    dt: Option<Arc<Pic>>,
}
const CLIP: f64 = PI / 4.0;
fn to_x(a: f64) -> i32 {
    let s = if a > PI { a - 2.0 * PI } else { a };
    jround(CX - s.tan() * PROJ) as i32
}
/// Doom's: the span, clipped to the view; None if not seen
fn clip_angles(mut a1: f64, mut a2: f64) -> Option<(i32, i32)> {
    let span = norm(a1 - a2);
    if span >= PI {
        return None;
    }
    let mut t = norm(a1 + CLIP);
    if t > 2.0 * CLIP {
        t -= 2.0 * CLIP;
        if t >= span {
            return None;
        }
        a1 = CLIP;
    }
    t = norm(CLIP - a2);
    if t > 2.0 * CLIP {
        t -= 2.0 * CLIP;
        if t >= span {
            return None;
        }
        a2 = norm(-CLIP);
    }
    Some((to_x(a1), to_x(a2)))
}
const BOX_CORNERS: [Option<[usize; 4]>; 11] = [
    Some([3, 0, 2, 1]),
    Some([3, 0, 2, 0]),
    Some([3, 1, 2, 0]),
    None,
    Some([2, 0, 2, 1]),
    None,
    Some([3, 1, 3, 0]),
    None,
    Some([2, 0, 3, 1]),
    Some([2, 1, 3, 1]),
    Some([2, 1, 3, 0]),
];
impl LvlPass<'_, '_> {
    fn ang_to(&self, x: f64, y: f64) -> f64 {
        (y - self.cam.y).atan2(x - self.cam.x) - self.cam.a
    }
    /// [top, bottom, left, right]
    fn box_seen(&self, b: &[f64; 4]) -> bool {
        let (vx, vy) = (self.cam.x, self.cam.y);
        let bx = if vx <= b[2] {
            0
        } else if vx < b[3] {
            1
        } else {
            2
        };
        let by = if vy >= b[0] {
            0
        } else if vy > b[1] {
            1
        } else {
            2
        };
        let pos = by * 4 + bx;
        if pos == 5 {
            return true;
        }
        let Some(cc) = BOX_CORNERS.get(pos).copied().flatten() else { return true };
        let a1 = norm(self.ang_to(b[cc[0]], b[cc[1]]));
        let a2 = norm(self.ang_to(b[cc[2]], b[cc[3]]));
        if norm(a1 - a2) >= PI {
            return true; // you are at its edge: seen (as Doom's; a wall's back is another matter)
        }
        let Some((x0, x1)) = clip_angles(a1, a2) else { return false };
        (x0.max(0)..x1.min(SW)).any(|x| self.v.solid[x as usize] == 0)
    }
    fn plane(&mut self, x: i32, y0: i32, y1: i32, sector: usize, is_floor: bool) {
        if y0 > y1 {
            return;
        }
        let l = self.r.lvl;
        let sec = &l.map.sectors[sector];
        let (h, pt) = if is_floor { (sec.floor, &sec.ftex) } else { (sec.ceil, &sec.ctex) };
        let v = &mut *self.v;
        if !is_floor && pt == "F_SKY1" {
            let Some(sky) = &self.sky else { return };
            let col = modi((norm(self.cam.a + (-v.kx[x as usize]).atan()) / (2.0 * PI) * 1024.0).floor() as i64, sky.w as i64) as i32;
            for y in y0..=y1 {
                let i = (y * SW + x) as usize;
                v.fb[i] = sky.px[(modi(((y / S) + 16) as i64, sky.h as i64) as i32 * sky.w + col) as usize];
                v.zb[i] = 1e8;
            }
            return;
        }
        let f = self.r.wad.flat(l.anim(pt));
        let dh = (h - self.cam.z).abs();
        let ln = (sec.light >> 4) + v.extralight;
        let kx = v.kx[x as usize];
        let (rdx, rdy) = (self.dx + self.rx * kx, self.dy + self.ry * kx);
        let cmap = &self.r.wad.cmap[..];
        for y in y0..=y1 {
            let d = dh * PROJ / (y as f64 + 0.5 - CY).abs();
            let (wx, wy) = (self.cam.x + rdx * d, self.cam.y + rdy * d);
            let i = (y * SW + x) as usize;
            let c = f.as_ref().map_or(96, |f| f[(m64(-wy) << 6) + m64(wx)]);
            v.fb[i] = cmap[plane_level(ln, d) as usize * 256 + c as usize];
            v.zb[i] = d as f32;
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn wall_run(&mut self, x: i32, y0: i32, y1: i32, tex: &Option<Arc<Pic>>, u: f64, ref_top: f64, yo: f64, z: f64, base: usize) {
        let Some(tex) = tex else { return };
        if y0 > y1 {
            return;
        }
        let col = modi(u.floor() as i64, tex.w as i64) as i32;
        let s = z / PROJ;
        let cmap = &self.r.wad.cmap[..];
        for y in y0..=y1 {
            let wz = self.cam.z - (y as f64 + 0.5 - CY) * s;
            let vv = (ref_top - wz + yo).floor() as i64;
            let i = (y * SW + x) as usize;
            self.v.fb[i] = cmap[base + tex.px[(modi(vv, tex.h as i64) as i32 * tex.w + col) as usize] as usize];
            self.v.zb[i] = z as f32;
        }
    }
    fn seg(&mut self, si: usize) {
        let l = self.r.lvl;
        let m = &l.map;
        let sg0 = &m.segs[si];
        let (mut v1, mut v2, mut offset) = (sg0.v1, sg0.v2, sg0.offset);
        if self.win.is_some() {
            // seen through the door: what is on the office's side of its wall is not there
            let (b1, b2) = (l.beyond(v1.x, v1.y), l.beyond(v2.x, v2.y));
            if b1 <= 0.01 && b2 <= 0.01 {
                return;
            }
            if b1 < 0.0 || b2 < 0.0 {
                let k = b1 / (b1 - b2);
                let c = crate::wad::V { x: v1.x + (v2.x - v1.x) * k, y: v1.y + (v2.y - v1.y) * k };
                if b1 < 0.0 {
                    offset = sg0.offset + (c.x - sg0.v1.x).hypot(c.y - sg0.v1.y);
                    v1 = c;
                } else {
                    v2 = c;
                }
            }
        }
        let a1 = norm(self.ang_to(v1.x, v1.y));
        let a2 = norm(self.ang_to(v2.x, v2.y));
        let Some((r0, r1)) = clip_angles(a1, a2) else { return };
        if r0 >= r1 {
            return;
        }
        let sd = &m.sides[sg0.front];
        let fsi = sd.sector;
        let fs = &m.sectors[fsi];
        let bsi = sg0.back.map(|b| m.sides[b].sector);
        let bs = bsi.map(|b| &m.sectors[b]);
        let line = &m.lines[sg0.line];
        let closed = bs.is_none_or(|bs| bs.ceil <= fs.floor || bs.floor >= fs.ceil);
        let is_door = self.win.is_none() && self.r.p().is_some_and(|p| p.lines.contains(&sg0.line));
        if let Some(bs) = bs
            && !is_door && !closed && bs.ceil == fs.ceil && bs.floor == fs.floor && bs.ftex == fs.ftex && bs.ctex == fs.ctex && bs.light == fs.light && sd.mid == "-" {
                return; // nothing to draw: a line within one floor
            }
        let (ex, ey) = (v2.x - v1.x, v2.y - v1.y);
        let len = ex.hypot(ey);
        let (wx0, wy0) = (v1.x - self.cam.x, v1.y - self.cam.y);
        let fake = if sg0.v1.y == sg0.v2.y {
            -1
        } else if sg0.v1.x == sg0.v2.x {
            1
        } else {
            0
        };
        let ln = (fs.light >> 4) + self.v.extralight + fake;
        let wad = self.r.wad;
        let tex = |n: &str| wad.texture(l.anim(n));
        let mid_t = if bs.is_none() && sd.mid != "-" { tex(&sd.mid) } else { None };
        let up_t = if bs.is_some() && sd.upper != "-" { tex(&sd.upper) } else { None };
        let lo_t = if bs.is_some() && sd.lower != "-" { tex(&sd.lower) } else { None };
        let mask_t = if !closed && sd.mid != "-" { tex(&sd.mid) } else { None };
        let both_sky = bs.is_some_and(|bs| fs.ctex == "F_SKY1" && bs.ctex == "F_SKY1");
        let (flags, xo, yo) = (line.flags, sd.xo, sd.yo);
        let (fs_floor, fs_ceil, fs_sky) = (fs.floor, fs.ceil, fs.ctex == "F_SKY1");
        let (bs_floor, bs_ceil) = bs.map_or((0.0, 0.0), |b| (b.floor, b.ceil));
        let p_floor = self.r.p().map_or(0.0, |p| p.floor);
        let vz = self.cam.z;
        for x in r0.max(0)..r1.min(SW) {
            let xu = x as usize;
            if self.v.solid[xu] != 0 {
                continue;
            }
            let kx = self.v.kx[xu];
            let (rdx, rdy) = (self.dx + self.rx * kx, self.dy + self.ry * kx);
            let den = rdx * ey - rdy * ex;
            if den.abs() < 1e-9 {
                continue;
            }
            let z = ((wx0 * ey - wy0 * ex) / den).max(1.0);
            let t = (wx0 * rdy - wy0 * rdx) / den;
            let u = offset + xo + t.clamp(0.0, 1.0) * len;
            let sc2 = PROJ / z;
            let base = wall_level(ln, z) as usize * 256;
            // sky on both sides: the sky comes down to the other side's ceiling (Doom's outdoor hack)
            let yc = CY - ((if both_sky { bs_ceil } else { fs_ceil }) - vz) * sc2;
            let yf = CY - (fs_floor - vz) * sc2;
            let (cc, fc) = (self.v.ceil_clip[xu] as i32, self.v.floor_clip[xu] as i32);
            let top = (cc + 1).max((yc - 0.5).ceil() as i32);
            let bot = (fc - 1).min((yf - 0.5).floor() as i32);
            if fs_ceil > vz || fs_sky {
                self.plane(x, cc + 1, (top - 1).min(fc - 1), fsi, false);
            }
            if fs_floor < vz {
                self.plane(x, (bot + 1).max(cc + 1), fc - 1, fsi, true);
            }
            if closed {
                if let Some(mt) = &mid_t {
                    let rt = if flags & 16 != 0 { fs_floor + mt.h as f64 } else { fs_ceil };
                    self.wall_run(x, top, bot, &mid_t, u, rt, yo, z, base);
                } else if bs.is_some() {
                    // two-sided but shut (a closed door, a lift up): its upper and lower textures fill it (R_StoreWallRange)
                    if bs_ceil < fs_ceil && !both_sky
                        && let Some(ut) = &up_t {
                            let rt = if flags & 8 != 0 { fs_ceil } else { bs_ceil + ut.h as f64 };
                            self.wall_run(x, top, bot.min((CY - (bs_ceil - vz) * sc2 - 0.5).floor() as i32), &up_t, u, rt, yo, z, base);
                        }
                    if bs_floor > fs_floor && lo_t.is_some() {
                        let rt = if flags & 16 != 0 { fs_ceil } else { bs_floor };
                        self.wall_run(x, top.max((CY - (bs_floor - vz) * sc2 - 0.5).ceil() as i32), bot, &lo_t, u, rt, yo, z, base);
                    }
                }
                if is_door {
                    self.door_column(x, v1, v2, t, z, top, bot, base);
                }
                self.v.solid[xu] = 1;
                self.solid_n += 1;
                continue;
            }
            let (ybc, ybf) = (CY - (bs_ceil - vz) * sc2, CY - (bs_floor - vz) * sc2);
            let (mut nc, mut nf) = (top - 1, bot + 1);
            if bs_ceil < fs_ceil && !both_sky {
                let y1 = bot.min((ybc - 0.5).floor() as i32);
                if let Some(ut) = &up_t {
                    let rt = if flags & 8 != 0 { fs_ceil } else { bs_ceil + ut.h as f64 };
                    self.wall_run(x, top, y1, &up_t, u, rt, yo, z, base);
                }
                nc = nc.max(y1);
            }
            if bs_floor > fs_floor {
                let y0 = top.max((ybf - 0.5).ceil() as i32);
                if lo_t.is_some() {
                    let rt = if flags & 16 != 0 { fs_ceil } else { bs_floor };
                    self.wall_run(x, y0, bot, &lo_t, u, rt, yo, z, base);
                }
                nf = nf.min(y0);
            }
            if is_door {
                self.door_column(x, v1, v2, t, z, top, bot, base);
                nf = nf.min(top.max((CY - (p_floor + DOORH - vz) * sc2 - 0.5).ceil() as i32));
            }
            self.v.ceil_clip[xu] = (self.v.ceil_clip[xu] as i32).max(nc) as i16;
            self.v.floor_clip[xu] = (self.v.floor_clip[xu] as i32).min(nf) as i16;
            if let Some(mt) = &mask_t {
                let rt = if flags & 16 != 0 { fs_floor.max(bs_floor) + mt.h as f64 } else { fs_ceil.min(bs_ceil) };
                self.masked.push(Masked { x, z, u, tex: mt.clone(), ref_top: rt, yo, base, c0: self.v.ceil_clip[xu] as i32, c1: self.v.floor_clip[xu] as i32 });
            }
            if self.v.ceil_clip[xu] as i32 >= self.v.floor_clip[xu] as i32 - 1 {
                self.v.solid[xu] = 1;
                self.solid_n += 1;
            }
        }
    }
    // the mysterious door in its wall: its panel risen door.open; under it, the office (the window)
    #[allow(clippy::too_many_arguments)]
    fn door_column(&mut self, x: i32, v1: crate::wad::V, v2: crate::wad::V, t: f64, z: f64, top: i32, bot: i32, base: usize) {
        let l = self.r.lvl;
        let tc = t.clamp(0.0, 1.0);
        let (hx, hy) = (v1.x + (v2.x - v1.x) * tc, v1.y + (v2.y - v1.y) * tc);
        let al = l.along(hx, hy);
        let Some(dt) = &self.dt else { return };
        if al.abs() >= DOORW / 2.0 {
            return;
        }
        let pf = self.r.p().unwrap().floor;
        let open = self.r.door_open;
        let s = z / PROJ;
        let du = (((al + DOORW / 2.0) / DOORW * dt.w as f64).floor() as i32).clamp(0, dt.w - 1);
        let (mut wy0, mut wy1) = (VH, -1);
        let vz = self.cam.z;
        let cmap = &self.r.wad.cmap[..];
        let ys = top.max((CY - (pf + DOORH - vz) * PROJ / z - 0.5).ceil() as i32);
        let ye = bot.min((CY - (pf - vz) * PROJ / z - 0.5).floor() as i32);
        for y in ys..=ye {
            let h = vz - (y as f64 + 0.5 - CY) * s - pf;
            let i = (y * SW + x) as usize;
            if h < open {
                if y < wy0 {
                    wy0 = y;
                }
                wy1 = y;
                continue;
            }
            let dv = (((DOORH - (h - open)) / DOORH * dt.h as f64).floor() as i32).clamp(0, dt.h - 1);
            self.v.fb[i] = cmap[base + dt.px[(dv * dt.w + du) as usize] as usize];
            self.v.zb[i] = z as f32;
        }
        if wy1 >= 0 {
            self.v.w_top[x as usize] = (wy0 - 1) as i16;
            self.v.w_bot[x as usize] = (wy1 + 1) as i16;
            self.v.w_z[x as usize] = z;
        }
    }
    fn node(&mut self, n: u16) {
        if self.solid_n >= SW {
            return;
        }
        let m = &self.r.lvl.map;
        if n & 0x8000 != 0 {
            let Some(ss) = m.ssectors.get((n & 0x7fff) as usize) else { return };
            for k in 0..ss.n {
                self.seg(ss.first + k);
            }
            return;
        }
        let nd = &m.nodes[n as usize];
        let s = Level::side(nd, self.cam.x, self.cam.y);
        self.node(nd.kids[s]);
        if self.box_seen(&nd.bbox[s ^ 1]) {
            self.node(nd.kids[s ^ 1]);
        }
    }
}

// ---- the shotgun, Doom's: its frames and their tics, the flash (A_Light1, A_Light2), the bob while you walk
const GUN: [(u8, i32, bool); 9] = [(b'A', 3, false), (b'A', 7, true), (b'B', 5, false), (b'C', 5, false), (b'D', 4, false), (b'C', 5, false), (b'B', 5, false), (b'A', 3, false), (b'A', 7, false)];

/// your face, Doom's way: 30 x 30 (at S) from your 20 x 20 avatar, for each state (pain 0..4, looking ahead, left,
/// right; the grin, the ouch, dead), in the palette
struct FaceSet {
    pain: Vec<[Arc<Pic>; 3]>,
    grin: Vec<Arc<Pic>>,
    ouch: Vec<Arc<Pic>>,
    dead: Arc<Pic>,
}

/// the office in the first person, and the level through its door
pub struct Fps {
    wad: Wad,
    door_wad: Option<Wad>,
    view: View,
    rng: Rng,
    still: bool,
    world: World,
    lvl: Level,
    level_name: &'static str,
    sc: Arc<Scene>,
    me: Me,
    door: Door,
    puffs: Vec<Puff>,
    gun_state: i32,
    gun_tics: i32,
    flash: i32,
    flash_tics: i32,
    wsx: f64,
    wsy: f64,
    time: i32,
    hand_trans: [u8; 256],
    face_set: Option<FaceSet>,
    face_look: i32,
    face_tics: i32,
    grin_until: f64,
    kills0: i32,
    ouch_until: f64,
    last_health: i32,
    keys: HashSet<Key>,
    turn_acc: f64,
    use_req: bool,
    turn_held: i32,
    melt: Option<Melt>,
    leave_melt: Option<Melt>,
    t0: f64,
    now: f64,
    acc: f64,
    events: Vec<Event>,
    /// the mutiny's shots at you, for the next tic; whether you were dead at the last
    incoming: Vec<Incoming>,
    dead_seen: bool,
    letters: HashMap<String, (i32, Vec<bool>)>,
    frame: Image,
    win: (u32, u32),
    left: bool,
    you: You,
}
#[derive(Clone, Copy, Debug, Default)]
struct Me {
    x: f64,
    y: f64,
    a: f64,
    mx: f64,
    my: f64,
    bob: f64,
}
/// the mysterious door: how far it stands open (Doom's: 2 units a tic up, 150 tics open, down again, back up if you
/// are under it)
#[derive(Clone, Copy, Debug, Default)]
struct Door {
    open: f64,
    dir: i32,
    wait: i32,
}

impl Fps {
    /// the office from `start`, Doom's data from `wad` (and the skull door's from `door_wad`, for a WAD without it)
    pub fn new(wad: Wad, door_wad: Option<Wad>, scene: Arc<Scene>, start: Start, opt: Options) -> Result<Fps, String> {
        let level_name = Level::first(&wad).ok_or("no level in the WAD")?;
        let lvl = Level::new(&wad, level_name, opt.seed.wrapping_add(1)).ok_or("the level would not load")?;
        let mut f = Fps {
            view: View::new(opt.still),
            rng: Rng::new(opt.seed),
            still: opt.still,
            world: World::Office,
            lvl,
            level_name,
            me: Me { x: start.x * U, y: start.y * U, a: start.a, ..Default::default() },
            door: Door::default(),
            puffs: Vec::new(),
            gun_state: -1,
            gun_tics: 0,
            flash: -1,
            flash_tics: 0,
            wsx: 1.0,
            wsy: 32.0,
            time: 0,
            hand_trans: std::array::from_fn(|i| i as u8),
            face_set: None,
            face_look: 0,
            face_tics: 0,
            grin_until: 0.0,
            kills0: 0,
            ouch_until: 0.0,
            last_health: 100,
            keys: HashSet::new(),
            turn_acc: 0.0,
            use_req: false,
            turn_held: 0,
            melt: None,
            leave_melt: None,
            t0: 0.0,
            now: 0.0,
            acc: 0.0,
            events: Vec::new(),
            incoming: Vec::new(),
            dead_seen: false,
            letters: HashMap::new(),
            frame: Image::new(SW as u32, SH as u32),
            win: (0, 0),
            left: false,
            you: You { x: start.x, y: start.y, a: start.a, walking: false },
            sc: scene,
            wad,
            door_wad,
        };
        f.hand_trans = f.hands(opt.skin);
        f.new_level(false);
        if let Some(src) = &opt.face {
            f.face_set = Some(f.faces(src));
        }
        if let Some(first) = opt.snapshot {
            f.win = (first.w, first.h);
            if !f.still {
                f.melt = Some(Melt::start(first, &mut f.rng, 0.0));
            }
        }
        Ok(f)
    }
    /// the WAD it plays (its sound lumps: `wad().lump(name)`)
    pub fn wad(&self) -> &Wad {
        &self.wad
    }
    /// the room at its last tick
    pub fn set_scene(&mut self, scene: Arc<Scene>) {
        self.sc = scene;
    }
    pub fn world(&self) -> World {
        self.world
    }
    pub fn level(&self) -> &Level {
        &self.lvl
    }
    pub fn level_mut(&mut self) -> &mut Level {
        &mut self.lvl
    }
    /// the level's name (E1M1)
    pub fn level_name(&self) -> &str {
        self.level_name
    }
    /// where you stand in the room (dots), the way you look, whether you walk (in the office)
    pub fn you(&self) -> You {
        self.you
    }
    /// the room's mutiny: shots at you since the last call (taken at the next tic)
    pub fn incoming(&mut self, shots: Vec<Incoming>) {
        self.incoming.extend(shots);
    }
    /// how many puffs of dust are in the air (a shot that hit a wall or went wide)
    pub fn puffs(&self) -> usize {
        self.puffs.len()
    }
    /// through the mysterious door, in the level (the room's mutineers wait for you at it)
    pub fn in_level(&self) -> bool {
        self.world == World::Level
    }
    /// what happened since the last call
    pub fn events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// the melt that takes this view away over the room, once you left
    pub fn take_leave_melt(&mut self) -> Option<Melt> {
        self.leave_melt.take()
    }
    pub fn left(&self) -> bool {
        self.left
    }
    /// the door: how far it stands open (0..72)
    pub fn door_open(&self) -> f64 {
        self.door.open
    }
    pub fn melting(&self) -> bool {
        self.melt.is_some()
    }

    fn units(&self) -> (f64, f64, f64) {
        (self.sc.w * U, self.sc.h * U, self.sc.wall * U)
    }
    fn door_x(&self) -> f64 {
        jround(self.sc.w * 0.3) * U
    }
    fn ctx3(&self) -> Ctx3<'_> {
        Ctx3 { wad: &self.wad, door_wad: self.door_wad.as_ref(), lvl: &self.lvl, sc: &self.sc, door_open: self.door.open, time: self.time, puffs: &self.puffs, now: self.now }
    }
    fn snd(&mut self, name: &'static str, vol: f64, pan: f64, origin: Origin) {
        self.events.push(Event::Sound(Sound::Doom { name, vol, pan, origin }));
    }

    // Freedoom's and Doom's hands are the brown ramp: your skin instead
    fn hands(&self, skin: Option<Color>) -> [u8; 256] {
        let mut t: [u8; 256] = std::array::from_fn(|i| i as u8);
        let Some(sk) = skin else { return t };
        let (r, g, b) = (sk.0[0] as f64, sk.0[1] as f64, sk.0[2] as f64);
        let lum = |q: u8| {
            let c = self.wad.rgb_of(q);
            0.3 * c[0] as f64 + 0.59 * c[1] as f64 + 0.11 * c[2] as f64
        };
        let mid = if lum(72) == 0.0 { 1.0 } else { lum(72) };
        for i in (64u8..=79).chain(236..=239) {
            let k = lum(i) / mid;
            t[i as usize] = self.wad.nearest((r * k).min(255.0) as u8, (g * k).min(255.0) as u8, (b * k).min(255.0) as u8);
        }
        t
    }
    fn faces(&mut self, src: &Image) -> FaceSet {
        let mut base = Image::new(20, 20);
        base.ctx().draw_image_scaled(src, 0.0, 0.0, 20.0, 20.0);
        let f = 30 * S;
        let make = |me: &mut Fps, pain: i32, look: i32, mode: &str| -> Arc<Pic> {
            let mut d = base.clone();
            let set = |d: &mut Image, x: i32, y: i32, c: [u8; 3]| {
                if !(0..20).contains(&x) || !(0..20).contains(&y) {
                    return;
                }
                let p = d.get(x, y);
                if p[3] < 100 {
                    return;
                }
                d.set(x, y, [c[0], c[1], c[2], p[3]]);
            };
            let dark = |d: &mut Image, x: i32, y: i32, k: f64| {
                let p = d.get(x, y);
                if p[3] < 100 {
                    return;
                }
                d.set(x, y, [u8c(p[0] as f64 * k), u8c(p[1] as f64 * k), u8c(p[2] as f64 * k), p[3]]);
            };
            const RED: [u8; 3] = [176, 24, 24];
            const RED2: [u8; 3] = [120, 12, 12];
            const INK: [u8; 3] = [20, 15, 30];
            const WHITE: [u8; 3] = [240, 240, 240];
            if pain >= 1 {
                set(&mut d, 12, 5, RED);
                set(&mut d, 12, 6, RED2);
                set(&mut d, 8, 12, RED2);
            }
            if pain >= 2 {
                dark(&mut d, 12, 11, 0.6);
                dark(&mut d, 13, 11, 0.6);
                set(&mut d, 13, 6, RED);
                set(&mut d, 13, 7, RED2);
                set(&mut d, 7, 5, RED);
            }
            if pain >= 3 {
                set(&mut d, 9, 14, RED);
                for y in 4..=8 {
                    set(&mut d, 7, y, if y % 2 != 0 { RED } else { RED2 });
                }
                dark(&mut d, 6, 9, 0.55);
                dark(&mut d, 6, 10, 0.55);
                dark(&mut d, 7, 11, 0.6);
            }
            if pain >= 4 {
                for y in 4..=8 {
                    for x in 10..=13 {
                        if (x + y) % 2 != 0 {
                            set(&mut d, x, y, RED2);
                        }
                    }
                }
                for y in 12..=14 {
                    for x in 5..=7 {
                        set(&mut d, x, y, RED);
                    }
                }
                dark(&mut d, 12, 9, 0.4);
                dark(&mut d, 13, 9, 0.4);
                dark(&mut d, 12, 10, 0.5);
            }
            if mode == "grin" {
                for x in 8..=12 {
                    set(&mut d, x, 14, WHITE);
                }
                set(&mut d, 8, 13, RED2);
                set(&mut d, 12, 13, RED2);
                set(&mut d, 9, 15, RED2);
                set(&mut d, 11, 15, RED2);
            }
            if mode == "ouch" {
                for y in 13..=15 {
                    for x in 9..=11 {
                        set(&mut d, x, y, if y == 13 { RED2 } else { INK });
                    }
                }
                set(&mut d, 7, 8, WHITE);
                set(&mut d, 12, 8, WHITE);
            }
            if mode == "dead" {
                for q in d.px.iter_mut() {
                    if q[3] < 100 {
                        continue;
                    }
                    let v = 0.3 * q[0] as f64 + 0.59 * q[1] as f64 + 0.11 * q[2] as f64;
                    *q = [u8c(v * 0.9 + 20.0), u8c(v * 0.85), u8c(v * 0.85), q[3]];
                }
                for ex in [7, 12] {
                    set(&mut d, ex, 9, INK);
                    set(&mut d, ex + 1, 10, INK);
                    set(&mut d, ex + 1, 9, INK);
                    set(&mut d, ex, 10, INK);
                }
                for x in 8..=12 {
                    set(&mut d, x, 14, INK);
                }
                set(&mut d, 10, 15, RED);
                set(&mut d, 10, 16, RED2);
            }
            // a whole number of pixels a pixel: crisp; the source shifted by where it looks
            let n = (f * f) as usize;
            let mut p = Pic { w: f, h: f, lo: 0, to: 0, px: vec![0; n], op: vec![0; n], masked: false };
            for y in 0..f {
                for x in 0..f {
                    let c = d.get(x * 20 / f + look, y * 20 / f);
                    if c[3] < 128 {
                        continue;
                    }
                    let k = (y * f + x) as usize;
                    p.px[k] = me.wad.nearest(c[0], c[1], c[2]);
                    p.op[k] = 1;
                }
            }
            Arc::new(p)
        };
        let dead = make(self, 4, 0, "dead");
        let (mut pain, mut grin, mut ouch) = (Vec::new(), Vec::new(), Vec::new());
        for pn in 0..5 {
            pain.push([make(self, pn, 0, ""), make(self, pn, 1, ""), make(self, pn, -1, "")]);
            grin.push(make(self, pn, 0, "grin"));
            ouch.push(make(self, pn, 0, "ouch"));
        }
        FaceSet { pain, grin, ouch, dead }
    }

    // ---- what is drawn over the view: a WAD picture at x, y (screen pixels; by its offsets), k screen pixels a texel
    fn blit(&mut self, p: Option<Arc<Pic>>, x: i32, y: i32, trans: Option<&[u8; 256]>, k: i32) {
        let Some(p) = p else { return };
        let (x0, y0) = (x - p.lo * k, y - p.to * k);
        for j in 0..p.h * k {
            let yy = y0 + j;
            if !(0..SH).contains(&yy) {
                continue;
            }
            let row = (j / k) * p.w;
            for i in 0..p.w * k {
                let xx = x0 + i;
                let q = (row + i / k) as usize;
                if !(0..SW).contains(&xx) || p.op[q] == 0 {
                    continue;
                }
                self.view.fb[(yy * SW + xx) as usize] = trans.map_or(p.px[q], |t| t[p.px[q] as usize]);
            }
        }
    }
    fn pt(&self, name: &str) -> Option<Arc<Pic>> {
        self.wad.patch(name)
    }
    /// the room's 3x5 letters into the screen, one palette colour, k pixels a letter pixel
    fn words(&mut self, text: &str, x: i32, y: i32, idx: u8, k: i32) -> i32 {
        if !self.letters.contains_key(text) {
            let w = crate::font::tiny_w(text).max(1);
            let mut a = vec![false; (w * 5) as usize];
            crate::font::tiny(text, 0, 0, |fx, fy| {
                if fx < w && (0..5).contains(&fy) {
                    a[(fy * w + fx) as usize] = true;
                }
            });
            self.letters.insert(text.to_string(), (w, a));
        }
        let (w, a) = &self.letters[text];
        let w = *w;
        for j in 0..5 * k {
            for i in 0..w * k {
                if !a[((j / k) * w + i / k) as usize] {
                    continue;
                }
                let (xx, yy) = (x + i, y + j);
                if (0..SW).contains(&xx) && (0..SH).contains(&yy) {
                    self.view.fb[(yy * SW + xx) as usize] = idx;
                }
            }
        }
        w * k
    }

    fn office_blocked(&self, x: f64, y: f64) -> bool {
        let (w, h, wall) = self.units();
        if x < R || x > w - R || y < wall + R {
            return true;
        }
        let p = self.lvl.portal.is_some();
        if y > h - R && !(p && self.door.open >= 56.0 && (x - self.door_x()).abs() <= DOORW / 2.0 - PR) {
            return true; // the south wall, but the open door
        }
        if p && self.ctx3().torches().iter().any(|t| (x - t.0).abs() < 16.0 + R && (y - t.1).abs() < 16.0 + R) {
            return true;
        }
        self.sc.solid.iter().any(|r| x > r[0] * U - R && x < r[2] * U + R && y > r[1] * U - R && y < r[3] * U + R)
    }

    fn bump(&mut self) {
        if self.door.dir == 1 {
            return;
        }
        if self.door.open >= DOORH {
            self.door.wait = 150;
            return;
        }
        self.door.dir = 1;
        self.door.wait = 0;
        self.door_snd("DSBDOPN");
    }
    fn door_snd(&mut self, name: &'static str) {
        if let Some(p) = &self.lvl.portal {
            let (x, y) = (p.cx, p.cy);
            self.heard(name, x, y, Origin::Door);
        }
    }
    fn in_doorway(&self) -> bool {
        if self.world == World::Office {
            return self.me.y > self.units().1 - PR - 2.0;
        }
        let l = &self.lvl;
        l.portal.is_some() && l.beyond(l.player.x, l.player.y).abs() < PR + 2.0 && l.along(l.player.x, l.player.y).abs() < DOORW / 2.0 + PR
    }
    fn door_tic(&mut self) {
        let d = &mut self.door;
        if d.dir == 1 {
            d.open = DOORH.min(d.open + 2.0);
            if d.open >= DOORH {
                d.dir = 0;
                d.wait = 150;
            }
        } else if d.wait > 0 {
            d.wait -= 1;
            if d.wait == 0 {
                d.dir = -1;
                self.door_snd("DSBDCLS");
            }
        } else if d.dir == -1 {
            if self.in_doorway() {
                self.door.dir = 1;
                self.door_snd("DSBDOPN");
            } else {
                let d = &mut self.door;
                d.open = 0f64.max(d.open - 2.0);
                if d.open == 0.0 {
                    d.dir = 0;
                }
            }
        }
        self.sync_portal();
    }
    /// the level's portal follows the door and the office
    fn sync_portal(&mut self) {
        let (w, h, wall) = self.units();
        let dx = self.door_x();
        if let Some(p) = &mut self.lvl.portal {
            p.open = self.door.open;
            p.office = [dx, w, h, wall];
        }
    }
    fn set_portal(&mut self) {
        let spot = self.lvl.door_spot(DOORW, DOORH);
        self.lvl.portal = spot.map(|s| Portal {
            cx: s.cx,
            cy: s.cy,
            nx: s.nx,
            ny: s.ny,
            tx: -s.ny,
            ty: s.nx,
            w: DOORW,
            h: DOORH,
            height: CEIL,
            floor: s.floor,
            lines: s.lines,
            sector: s.sector,
            open: self.door.open,
            office: [0.0; 4],
        });
        self.sync_portal();
        let Some(p) = self.lvl.portal.clone() else { return };
        for k in [-1.0, 1.0] {
            // in the level too, a torch either side of it where there is room (not in a niche)
            let x = p.cx + p.tx * k * (DOORW / 2.0 + 22.0) + p.nx * 20.0;
            let y = p.cy + p.ty * k * (DOORW / 2.0 + 22.0) + p.ny * 20.0;
            if self.lvl.inside(x, y) && self.lvl.map.sectors[self.lvl.sector_at(x, y)].floor == p.floor && self.lvl.room_for(16.0, 56.0, p.floor, x, y) {
                self.lvl.place(46, x, y);
            }
        }
    }

    // ---- input: WASD (or the arrows) walk, the mouse turns (the pointer locked), Shift runs, a click (or Ctrl, F,
    // Enter) fires, E (or Space) uses, Esc leaves
    pub fn key_down(&mut self, k: Key) {
        if k == Key::Escape {
            return self.leave();
        }
        self.keys.insert(k);
        if matches!(k, Key::E | Key::Space) {
            self.use_req = true;
        }
        if matches!(k, Key::Enter | Key::F | Key::ControlLeft | Key::ControlRight) {
            self.act();
        }
    }
    pub fn key_up(&mut self, k: Key) {
        self.keys.remove(&k);
    }
    /// the mouse moved while the pointer is locked (its movementX, in pixels)
    pub fn mouse_moved(&mut self, dx: f64) {
        self.turn_acc -= dx * 0.0035;
    }
    /// dragged without the pointer locked (pixels)
    pub fn mouse_dragged(&mut self, dx: f64) {
        self.turn_acc -= dx * 0.008;
    }
    /// a click: fire; dead, a while after, Doom's respawn
    pub fn click(&mut self) {
        self.act();
    }
    /// the window lost the focus
    pub fn blur(&mut self) {
        self.keys.clear();
    }
    /// E, as if pressed (for tests)
    pub fn use_(&mut self) {
        self.use_req = true;
    }
    fn act(&mut self) {
        let p = &self.lvl.player;
        if p.dead {
            if self.lvl.time - p.dead_at > 35 {
                self.respawn();
            }
            return;
        }
        self.pull();
    }
    /// the trigger: the shot comes three tics in, as Doom's
    fn pull(&mut self) {
        let p = &self.lvl.player;
        if self.gun_state >= 0 || p.dead || p.ammo[1] <= 0 {
            return;
        }
        self.gun_state = 0;
        self.gun_tics = GUN[0].1;
    }
    fn walking(&self) -> bool {
        [Key::W, Key::A, Key::S, Key::D, Key::Up, Key::Down].iter().any(|k| self.keys.contains(k))
    }
    /// this tic's move, Doom's way: the keys turn slowly the first six tics
    fn cmd(&mut self) -> Cmd {
        let k = |a: Key, b: Option<Key>| self.keys.contains(&a) || b.is_some_and(|b| self.keys.contains(&b));
        let fw = k(Key::W, Some(Key::Up)) as i32 - k(Key::S, Some(Key::Down)) as i32;
        let sd = k(Key::D, None) as i32 - k(Key::A, None) as i32;
        let run = k(Key::ShiftLeft, Some(Key::ShiftRight));
        let kt = k(Key::Left, None) as i32 - k(Key::Right, None) as i32;
        self.turn_held = if kt != 0 { self.turn_held + 1 } else { 0 };
        let speed = if self.turn_held < 6 {
            320.0
        } else if run {
            1280.0
        } else {
            640.0
        };
        let turn = self.turn_acc + kt as f64 * speed / 65536.0 * 2.0 * PI;
        self.turn_acc = 0.0;
        let use_ = std::mem::take(&mut self.use_req);
        Cmd { forward: fw as f64, side: sd as f64, run, turn, use_ }
    }

    // ---- the tics: 35 a second, whatever the frame rate. The level goes on while you are in the office
    fn office_tic(&mut self, c: &Cmd) {
        let (_, h, _) = self.units();
        let door_x = self.door_x();
        if !self.lvl.player.dead {
            self.me.a -= c.turn; // (the room's y runs down: right is the other way round)
            let fw = c.forward * if c.run { 50.0 } else { 25.0 } / 32.0;
            let sd = c.side * if c.run { 40.0 } else { 24.0 } / 32.0;
            let (ca, sa) = (self.me.a.cos(), self.me.a.sin());
            self.me.mx += ca * fw - sa * sd;
            self.me.my += sa * fw + ca * sd;
            let sp = self.me.mx.hypot(self.me.my);
            if sp > 30.0 {
                self.me.mx *= 30.0 / sp;
                self.me.my *= 30.0 / sp;
            }
            let (x, y, mx, my) = (self.me.x, self.me.y, self.me.mx, self.me.my);
            let stuck = self.office_blocked(x, y);
            if stuck || !self.office_blocked(x + mx, y) {
                self.me.x += mx;
            } else {
                self.me.mx = 0.0;
            }
            let x = self.me.x;
            if stuck || !self.office_blocked(x, y + my) {
                self.me.y += my;
            } else {
                // walked into the door: it opens
                if my > 0.0 && y > h - R - 8.0 && (x - door_x).abs() < DOORW / 2.0 && self.lvl.portal.is_some() {
                    self.bump();
                }
                self.me.my = 0.0;
            }
            let m = &mut self.me;
            m.mx *= 0.90625;
            m.my *= 0.90625;
            if m.mx.abs() < 0.0625 && m.my.abs() < 0.0625 && c.forward == 0.0 && c.side == 0.0 {
                m.mx = 0.0;
                m.my = 0.0;
            }
            m.bob = 16f64.min((m.mx * m.mx + m.my * m.my) / 4.0);
            if c.use_ {
                self.use_thing();
            }
        }
        self.you = You { x: self.me.x / U, y: self.me.y / U, a: self.me.a, walking: self.walking() };
        if self.lvl.portal.is_some() {
            // where you are in the level's terms: behind its door
            let r = self.ctx3();
            let (x, y) = r.o2l(self.me.x, self.me.y);
            let a = r.a2l(self.me.a);
            let p = self.lvl.portal.as_ref().unwrap();
            let (fl, sec) = (p.floor, p.sector);
            let pl = &mut self.lvl.player;
            pl.x = x;
            pl.y = y;
            pl.z = fl;
            pl.floorz = fl;
            pl.angle = a;
            pl.sector = sec;
            pl.mx = 0.0;
            pl.my = 0.0;
            if self.me.y > h && self.door.open >= 56.0 && (self.me.x - door_x).abs() <= DOORW / 2.0 - PR {
                self.through_to_level();
            }
        }
        if self.time % 4 == 0 {
            // the fire and the torches flicker (Doom's FireFlicker)
            self.view.fire_flick = (self.rng.random() * 64.0) as i32;
            self.view.torch_flick = (self.rng.random() * 4.0) as i32;
        }
    }
    fn use_thing(&mut self) {
        if self.world != World::Office {
            return;
        }
        let (_, h, _) = self.units();
        let (mx, my, ma) = (self.me.x, self.me.y, self.me.a);
        if self.lvl.portal.is_some() && my > h - 64.0 - R && ma.sin() > 0.6 && (mx + ma.cos() * (h - my) / ma.sin() - self.door_x()).abs() < DOORW / 2.0 {
            return self.bump(); // the door in front of you
        }
        let th = self.under(3);
        if let Some(rem) = th.as_ref().and_then(|t| t.rem.clone()) {
            self.events.push(Event::Unshoot { rem });
            return;
        }
        if th.as_ref().is_some_and(|t| t.reload) {
            // the gun closet: a box of shells
            let p = &mut self.lvl.player;
            if p.ammo[1] < p.max[1] {
                p.ammo[1] = p.max[1].min(p.ammo[1] + 20);
                p.bonuscount = (p.bonuscount + 6).min(40);
                self.snd("DSWPNUP", 1.0, 0.0, Origin::Pickup);
            }
            return;
        }
        if let Some(rem) = self.near().and_then(|t| t.rem) {
            self.events.push(Event::Unshoot { rem });
        }
    }
    fn tic(&mut self) {
        self.time += 1;
        let c = self.cmd();
        for s in std::mem::take(&mut self.incoming) {
            self.take_shot(s);
        }
        self.sync_portal();
        if self.world == World::Level {
            self.lvl.away = false;
            let (px, py) = (self.lvl.player.x, self.lvl.player.y);
            let has_p = self.lvl.portal.is_some();
            let b0 = if has_p { self.lvl.beyond(px, py) } else { 0.0 };
            self.lvl.tic(&c);
            self.level_heard();
            if std::mem::take(&mut self.lvl.bumped) {
                self.bump();
            }
            let (px, py) = (self.lvl.player.x, self.lvl.player.y);
            if self.lvl.done {
                self.exit_level();
            } else if has_p && b0 >= 0.0 && self.lvl.beyond(px, py) < 0.0 && self.lvl.along(px, py).abs() < DOORW / 2.0 {
                self.through_to_office(); // through the doorway (not anywhere behind its wall)
            }
        } else {
            self.office_tic(&c);
            if self.world == World::Office {
                self.lvl.away = true;
                self.lvl.tic(&c);
                self.level_heard();
                self.lvl.bumped = false;
            }
        }
        self.door_tic();
        // the weapon: Doom's bob while ready (A_WeaponReady), then its frames
        let bob = if self.world == World::Level { self.lvl.player.bob } else { self.me.bob };
        if self.gun_state < 0 {
            let ang = ((128 * self.time) % 8192) as f64 / 8192.0 * 2.0 * PI;
            self.wsx = 1.0 + bob * ang.cos();
            self.wsy = 32.0 + bob * ang.sin().abs();
        }
        if self.lvl.player.dead && !self.dead_seen {
            self.events.push(Event::Died); // (the room's mutiny: you lie there, they stand round you)
        }
        self.dead_seen = self.lvl.player.dead;
        self.gun_tic();
        self.puffs.retain_mut(|f| {
            f.t += 1;
            f.t < 16
        });
    }
    // the mutiny (room.js, hardcore): a shot at you from one of them, from where it stands. Doom's shotgun from there;
    // wide, a puff past you; else Doom's damage (the pain, the face, the red)
    fn take_shot(&mut self, s: Incoming) {
        let (w, h, wall) = self.units();
        let (sx, sy) = (s.x * U, s.y * U);
        let (dx, dy) = (self.me.x - sx, self.me.y - sy);
        let d = dx.hypot(dy);
        let d = if d == 0.0 { 1.0 } else { d };
        let office = self.world == World::Office;
        let vol = if office { if d < 160.0 { 1.0 } else { 0.15f64.max((1200.0 - d) / 1040.0) } } else { 0.3 };
        let pan = 0.75 * ((sy - self.me.y).atan2(sx - self.me.x) - self.me.a).sin();
        self.snd("DSSHOTGN", vol, pan, Origin::Mutiny((s.x * 1000.0).round() as i64));
        if !office || self.lvl.player.dead {
            return;
        }
        if !s.hit {
            let k = (d + 40.0 + self.rng.random() * 60.0) / d;
            let x = (sx + dx * k + (self.rng.random() - 0.5) * 30.0).clamp(4.0, w - 4.0);
            let y = (sy + dy * k + (self.rng.random() - 0.5) * 30.0).clamp(wall + 2.0, h - 4.0);
            let z = 14.0 + self.rng.random() * 36.0;
            self.puffs.push(Puff { x, y, z, t: 0, blood: false });
            return;
        }
        self.lvl.damage(Who::Player, s.dmg, None);
    }
    fn level_heard(&mut self) {
        for h in std::mem::take(&mut self.lvl.sounds) {
            self.heard(h.name, h.x, h.y, h.o);
        }
    }
    // a sound in the level, as Doom places it (S_AdjustSoundParams): from what made it to you (in the office: you
    // behind the door, the door shut muffles it); loud within 160 units, gone at 1200, to the side it is on. Yours,
    // full and in the middle
    fn heard(&mut self, name: &'static str, x: f64, y: f64, o: Origin) {
        let p = &self.lvl.player;
        let (mut vol, mut pan) = (1.0, 0.0);
        if o != Origin::Player {
            let (dx, dy) = ((x - p.x).abs(), (y - p.y).abs());
            let d = dx + dy - dx.min(dy) / 2.0;
            if d > 1200.0 {
                return;
            }
            vol = if d < 160.0 { 1.0 } else { (1200.0 - d) / 1040.0 };
            pan = if d < 1.0 { 0.0 } else { -0.75 * ((y - p.y).atan2(x - p.x) - p.angle).sin() };
        }
        if self.world == World::Office && o != Origin::Door && self.door.open < 8.0 {
            vol *= 0.3;
        }
        self.snd(name, vol, pan, o);
    }
    fn gun_tic(&mut self) {
        if self.flash >= 0 {
            self.flash_tics -= 1;
            if self.flash_tics <= 0 {
                self.flash += 1;
                self.flash_tics = 3;
                if self.flash > 1 {
                    self.flash = -1;
                    self.view.extralight = 0;
                } else {
                    self.view.extralight = 2;
                }
            }
        }
        if self.gun_state < 0 {
            return;
        }
        self.gun_tics -= 1;
        if self.gun_tics > 0 {
            return;
        }
        self.gun_state += 1;
        if self.gun_state as usize >= GUN.len() {
            self.gun_state = -1;
            return;
        }
        let (_, tics, fire) = GUN[self.gun_state as usize];
        self.gun_tics = tics;
        if fire {
            self.shoot();
        }
    }
    fn shoot(&mut self) {
        if self.lvl.player.ammo[1] <= 0 {
            return;
        }
        self.flash = 0;
        self.flash_tics = 4;
        self.view.extralight = 1;
        self.snd("DSSHOTGN", 1.0, 0.0, Origin::Gun);
        if self.world == World::Level {
            self.lvl.fire();
            return;
        }
        // the office: the bot nearest you within a few degrees of where you look (Doom's autoaim) goes to pieces;
        // else, through the open door, the shot goes into the level (you are there in its terms, behind its door)
        let (_, _, wall) = self.units();
        let (dx, dy) = (self.me.a.cos(), self.me.a.sin());
        let (mut best, mut bd) = (None, 1e9);
        for th in &self.sc.things {
            let Some(bot) = &th.bot else { continue };
            let (vx, vy) = (th.x * U - self.me.x, th.y * U - self.me.y);
            let d = vx * dx + vy * dy;
            if d < 8.0 || d > bd {
                continue;
            }
            if (vx * -dy + vy * dx).abs() > 10.0 + d * (5.0 * PI / 180.0).tan() {
                continue;
            }
            let sx = jround(CX + (vx * -dy + vy * dx) * PROJ / d) as i32;
            if (0..SW).contains(&sx) && (self.view.zb[(CYI * SW + sx) as usize] as f64) < d - 6.0 {
                continue; // behind a desk
            }
            best = Some(bot.clone());
            bd = d;
        }
        let has_p = self.lvl.portal.is_some();
        if has_p && self.door.open > 0.0 {
            self.lvl.noise(); // the door open: the level hears it
        }
        let alarm = Event::Alarm { x: self.me.x / U, y: self.me.y / U };
        if best.is_none() && has_p && self.view.aim_through {
            self.lvl.fire();
            self.events.push(alarm);
            return;
        }
        self.lvl.player.ammo[1] -= 1;
        if let Some(bot) = best {
            // in pieces: its parts come down
            self.events.push(Event::Shot { bot });
            for k in 0..3 {
                let delay_ms = 200.0 + k as f64 * 160.0 + self.rng.random() * 120.0;
                self.events.push(Event::Sound(Sound::Synth { name: "clank", vol: 0.9, pan: 0.0, delay_ms }));
            }
            self.events.push(alarm);
            return;
        }
        let d = (self.view.zb[(CYI * SW + CXI) as usize] as f64).min(3000.0) * 0.97;
        self.puffs.push(Puff { x: self.me.x + dx * d, y: (wall + 2.0).max(self.me.y + dy * d), z: 41.0, t: 0, blood: false });
        self.events.push(alarm);
    }

    // ---- through the door, either way: the same place, the same way you look and move, no wipe
    fn through_to_level(&mut self) {
        let r = self.ctx3();
        let (x, y) = r.o2l(self.me.x, self.me.y);
        let a = r.a2l(self.me.a);
        let sec = self.lvl.sector_at(x, y);
        let p = self.lvl.portal.clone().unwrap();
        let pl = &mut self.lvl.player;
        pl.x = x;
        pl.y = y;
        pl.z = p.floor;
        pl.floorz = p.floor;
        pl.mz = 0.0;
        pl.angle = a;
        pl.sector = sec;
        pl.mx = p.nx * self.me.my + p.tx * self.me.mx;
        pl.my = p.ny * self.me.my + p.ty * self.me.mx;
        pl.bob = self.me.bob;
        self.world = World::Level;
        self.lvl.away = false;
    }
    fn through_to_office(&mut self) {
        let r = self.ctx3();
        let pl = &self.lvl.player;
        let (x, y) = r.l2o(pl.x, pl.y);
        let a = r.a2o(pl.angle);
        let p = self.lvl.portal.as_ref().unwrap();
        self.me.x = x;
        self.me.y = y;
        self.me.a = a;
        self.me.my = pl.mx * p.nx + pl.my * p.ny;
        self.me.mx = pl.mx * p.tx + pl.my * p.ty;
        self.me.bob = pl.bob;
        self.world = World::Office;
        self.lvl.away = true;
    }
    // the level from its start: dead (Doom's respawn: as you began), or out through its exit (what you carry stays);
    // either way back in the office, in front of the door, with Doom's wipe
    fn new_level(&mut self, keep: bool) {
        let old = self.lvl.player.clone();
        let seed = (self.rng.random() * 1e15) as u64;
        if let Some(l) = Level::new(&self.wad, self.level_name, seed) {
            self.lvl = l;
        }
        self.set_portal();
        let p = &mut self.lvl.player;
        if keep {
            p.health = old.health;
            p.armor = old.armor;
            p.armor_class = old.armor_class;
            p.ammo = old.ammo;
            p.max = old.max;
        }
        self.lvl.away = true;
        self.door = Door::default();
        self.sync_portal();
    }
    fn back_home(&mut self, keep: bool) {
        let before = self.shot();
        let (_, h, _) = self.units();
        self.new_level(keep);
        self.world = World::Office;
        self.gun_state = -1;
        self.flash = -1;
        self.view.extralight = 0;
        self.me = Me { x: self.door_x(), y: h - 40.0, a: -PI / 2.0, ..Default::default() };
        if !self.still
            && let Some(b) = before {
                self.melt = Some(Melt::start(b, &mut self.rng, self.now));
            }
    }
    fn respawn(&mut self) {
        self.back_home(false);
        self.events.push(Event::Respawn);
    }
    fn exit_level(&mut self) {
        self.back_home(true)
    }
    /// the screen as it is, as large as the window (the bars black)
    fn shot(&self) -> Option<Image> {
        let (w, h) = self.win;
        if w == 0 || h == 0 {
            return None;
        }
        let mut c = Image::filled(w, h, Color::rgb(0));
        self.letterbox(&mut c);
        Some(c)
    }
    fn letterbox(&self, out: &mut Image) {
        let k = (out.w as f64 / 4.0).min(out.h as f64 / 3.0);
        let (cw, ch) = ((k * 4.0).floor(), (k * 3.0).floor());
        let (x0, y0) = (((out.w as f64 - cw) / 2.0).floor(), ((out.h as f64 - ch) / 2.0).floor());
        out.ctx().draw_image_scaled(&self.frame, x0, y0, cw, ch);
    }
    /// out of the first person: the view melts away over the room (`take_leave_melt`)
    pub fn leave(&mut self) {
        if self.left {
            return;
        }
        self.left = true;
        if !self.still
            && let Some(s) = self.shot() {
                self.leave_melt = Some(Melt::start(s, &mut self.rng, self.now));
            }
        self.events.push(Event::Leave);
    }

    /// time goes on: dt in ms (35 tics a second, at most a quarter second at once)
    pub fn tick(&mut self, dt: f64) {
        if self.left {
            return;
        }
        self.acc += dt.clamp(0.0, 250.0);
        self.now += dt.max(0.0);
        while self.acc >= TICMS {
            self.tic();
            self.acc -= TICMS;
            if self.left {
                break;
            }
        }
    }
    /// ms since the start (the clock the melts run on)
    pub fn now(&self) -> f64 {
        self.now
    }

    // ---- the frame: what you see, the gun, the bar, the palette (red while you hurt, gold when you pick up)
    fn office_cam(&self) -> OCam {
        let p = &self.lvl.player;
        let bob = if self.still { 0.0 } else { (self.time as f64 * 2.0 * PI / 20.0).sin() * self.me.bob / 2.0 };
        OCam { x: self.me.x, y: self.me.y, eye: p.viewh + bob, a: self.me.a }
    }
    /// where the level's camera is (in the level)
    pub fn level_cam(&self) -> LCam {
        let p = &self.lvl.player;
        let bob = if p.dead || self.still { 0.0 } else { (self.lvl.time as f64 * 2.0 * PI / 20.0).sin() * p.bob / 2.0 };
        LCam { x: p.x, y: p.y, z: p.z + p.viewh + p.dviewh + bob, a: p.angle }
    }
    fn render(&mut self) {
        self.view.frame_no = self.view.frame_no.wrapping_add(1);
        let sc = self.sc.clone();
        {
            let r = Ctx3 { wad: &self.wad, door_wad: self.door_wad.as_ref(), lvl: &self.lvl, sc: &sc, door_open: self.door.open, time: self.time, puffs: &self.puffs, now: self.now };
            self.view.light_grid(&r);
            if self.world == World::Level {
                let cam = self.level_cam();
                self.view.render_level(&r, cam, None);
            } else {
                let cam = self.office_cam();
                self.view.render_office(&r, cam, None);
            }
        }
        self.view.drawn = Some(sc);
        self.draw_gun();
        if self.world == World::Office {
            self.office_hud();
        } else if self.now - self.t0 < 6000.0 {
            self.help();
        }
        self.status_bar();
    }
    /// the frame into self.frame, in the palette of the moment
    fn present(&mut self) {
        self.render();
        let p = &self.lvl.player;
        let mut pal = 0;
        if p.damagecount > 0 {
            pal = 7.min((p.damagecount + 7) >> 3) + 1; // (Doom's STARTREDPALS)
        } else if p.bonuscount > 0 {
            pal = 8 + 4.min((p.bonuscount + 7) >> 3);
        }
        if p.dead {
            pal = pal.max(3);
            self.died();
        }
        let rgb = self.wad.rgb.get(pal as usize).unwrap_or(&self.wad.rgb[0]);
        for (o, &i) in self.frame.px.iter_mut().zip(self.view.fb.iter()) {
            *o = rgb[i as usize];
        }
    }
    /// Doom's screen, 640 x 400 (to be shown at 4:3)
    pub fn draw(&mut self, out: &mut Image) {
        self.present();
        if out.w != SW as u32 || out.h != SH as u32 {
            *out = Image::new(SW as u32, SH as u32);
        }
        out.px.copy_from_slice(&self.frame.px);
    }
    /// the window: black, Doom's screen in the middle at 4:3, the melt over it
    pub fn draw_window(&mut self, out: &mut Image) {
        self.win = (out.w, out.h);
        self.present();
        out.px.fill([0, 0, 0, 255]);
        self.letterbox(out);
        let now = self.now;
        if let Some(m) = &mut self.melt
            && !m.draw(out, now) {
                self.melt = None;
            }
    }
    /// dead: a dark band across the view, YOU DIED in it, then how to come back
    fn died(&mut self) {
        let t = self.lvl.time - self.lvl.player.dead_at;
        if t < 18 {
            return;
        }
        let (k, txt) = (6 * S, "YOU DIED");
        let y0 = jround(VH as f64 / 2.0 - 5.0 * k as f64 / 2.0) as i32 - 8 * S;
        for y in (y0 - 7 * S)..(y0 + 5 * k + if t > 35 { 19 } else { 7 } * S) {
            if !(0..SH).contains(&y) {
                continue;
            }
            for x in 0..SW {
                let i = (y * SW + x) as usize;
                self.view.fb[i] = self.wad.cmap[27 * 256 + self.view.fb[i] as usize];
            }
        }
        let x0 = jround(CX - crate::font::tiny_w(txt) as f64 * k as f64 / 2.0) as i32;
        let (shadow, red) = (self.wad.nearest(40, 0, 0), self.wad.nearest(200, 30, 24));
        self.words(txt, x0 + S, y0 + S, shadow, k);
        self.words(txt, x0, y0, red, k);
        if t > 35 {
            let again = "CLICK TO RESPAWN";
            let white = self.wad.nearest(232, 232, 238);
            self.words(again, jround(CX - crate::font::tiny_w(again) as f64 * S as f64 / 2.0) as i32, y0 + 5 * k + 7 * S, white, S);
        }
    }
    fn help(&mut self) {
        let c = self.wad.nearest(190, 190, 200);
        self.words("WASD WALK - MOUSE TURN - CLICK SHOOT - E USE - ESC LEAVE", 2 * S, VH - 7 * S, c, S);
    }
    fn office_hud(&mut self) {
        let th = self.under(2);
        let bot = th.as_ref().is_some_and(|t| t.bot.is_some());
        let what = if bot { None } else { th.as_ref().and_then(|t| t.label.clone()) };
        let rm = self.near();
        let who = if bot { th.clone() } else { rm.clone() };
        let white = self.wad.nearest(232, 232, 238);
        let mid = |me: &mut Fps, t: &str, y: i32, c: u8| {
            me.words(t, jround(CX - crate::font::tiny_w(t) as f64 * S as f64 / 2.0) as i32, y * S, c, S);
        };
        if let Some(name) = who.as_ref().and_then(|w| w.name.clone()).filter(|n| !n.is_empty()) {
            let c = if bot { self.wad.nearest(255, 140, 120) } else { white };
            mid(self, &name.to_uppercase(), 4, c);
        } else if let Some(w) = what {
            mid(self, &w.to_uppercase(), 4, white);
        }
        if !bot && rm.is_some() {
            mid(self, "E: PUT BACK TOGETHER", 11, white);
        }
        if self.now - self.t0 < 6000.0 {
            self.help();
        }
    }
    /// what is under the crosshair (within r pixels at 320 x 200), nearest first, as the last frame drew it
    fn under(&self, r: i32) -> Option<Thing> {
        let sc = self.view.drawn.as_ref()?;
        let r = r * S;
        let (mut best, mut bd) = (None, f32::INFINITY);
        for y in CYI - r..=CYI + r {
            for x in CXI - r..=CXI + r {
                let i = (y * SW + x) as usize;
                let n = self.view.idb[i];
                if n != 0 && self.view.zb[i] < bd {
                    best = sc.things.get(n as usize - 1);
                    bd = self.view.zb[i];
                }
            }
        }
        best.cloned()
    }
    fn near(&self) -> Option<Thing> {
        if let Some(th) = self.under(4).filter(|t| t.rem.is_some()) {
            return Some(th);
        }
        let (mut best, mut bd) = (None, 70.0 * U);
        for t in &self.sc.things {
            if t.rem.is_none() {
                continue;
            }
            let (rx2, ry2) = (t.x * U - self.me.x, t.y * U - self.me.y);
            let d = rx2.hypot(ry2);
            let d = if d == 0.0 { 1.0 } else { d };
            if d < bd && (rx2 * self.me.a.cos() + ry2 * self.me.a.sin()) / d > 0.5 {
                best = Some(t);
                bd = d;
            }
        }
        best.cloned()
    }
    fn draw_gun(&mut self) {
        if self.lvl.player.dead {
            return;
        }
        let fr = if self.gun_state >= 0 { GUN[self.gun_state as usize].0 } else { b'A' };
        let g = self.wad.sprite_frame("SHTG", (fr - b'A') as usize, 1).and_then(|s| s.pic);
        let ln = if self.world == World::Level {
            (self.lvl.map.sectors[self.lvl.player.sector].light >> 4) + self.view.extralight
        } else {
            (self.view.office_light(self.me.x, self.me.y) >> 4) + self.view.extralight
        };
        let lv = 0.max((15 - clamp_l(ln)) * 4 - 23) as usize;
        let (wsx, wsy) = (self.wsx, self.wsy);
        let hand = self.hand_trans;
        // a psprite, Doom's place for it, S pixels a texel
        let draw = |fb: &mut [u8], cmap: &[u8], q: Option<Arc<Pic>>, bright: bool, trans: Option<&[u8; 256]>| {
            let Some(q) = q else { return };
            let x0 = jround((wsx - q.lo as f64) * S as f64) as i32;
            let y0 = jround((wsy - q.to as f64 - 16.0) * S as f64) as i32;
            for j in 0..q.h * S {
                let y = y0 + j;
                if !(0..VH).contains(&y) {
                    continue;
                }
                let row = (j / S) * q.w;
                for i in 0..q.w * S {
                    let x = x0 + i;
                    let k = (row + i / S) as usize;
                    if !(0..SW).contains(&x) || q.op[k] == 0 {
                        continue;
                    }
                    let c = trans.map_or(q.px[k], |t| t[q.px[k] as usize]);
                    fb[(y * SW + x) as usize] = if bright { c } else { cmap[lv * 256 + c as usize] };
                }
            }
        };
        draw(&mut self.view.fb, &self.wad.cmap, g, false, Some(&hand));
        if self.flash >= 0 {
            let f = self.wad.sprite_frame("SHTF", self.flash as usize, 1).and_then(|s| s.pic);
            draw(&mut self.view.fb, &self.wad.cmap, f, true, None);
        }
    }

    // ---- the status bar: Doom's, from the WAD, the game's numbers; your face in it
    /// Doom's big numbers, right-aligned (in 320 x 200)
    fn num(&mut self, n: i32, right: i32, y: i32, prefix: &str, pct: bool) {
        let s = n.clamp(-99, 999).to_string();
        let mut x = right;
        if pct {
            let p = self.pt("STTPRCNT");
            self.blit(p, right * S, y * S, None, S);
        }
        for ch in s.chars().rev() {
            let p = if ch == '-' { self.pt("STTMINUS") } else { self.pt(&format!("{prefix}NUM{ch}")) };
            let Some(p) = p else { continue };
            x -= p.w;
            self.blit(Some(p), x * S, y * S, None, S);
        }
    }
    fn small_num(&mut self, n: i32, right: i32, y: i32) {
        let s = n.clamp(0, 999).to_string();
        let mut x = right;
        for ch in s.chars().rev() {
            let Some(p) = self.pt(&format!("STYSNUM{ch}")) else { continue };
            x -= p.w;
            self.blit(Some(p), x * S, y * S, None, S);
        }
    }
    fn face(&mut self, health: i32, dead: bool, kills: i32) -> (Option<Arc<Pic>>, i32) {
        let now = self.now;
        if kills > self.kills0 {
            self.grin_until = now + 2000.0;
        }
        self.kills0 = kills;
        if health < self.last_health - 20 {
            self.ouch_until = now + 1000.0;
        }
        self.last_health = health;
        let pain = ((100 - health.clamp(0, 100)) * 5 / 101).clamp(0, 4) as usize;
        self.face_tics -= 1;
        if self.face_tics <= 0 {
            self.face_look = (self.rng.random() * 3.0) as i32;
            self.face_tics = 17;
        }
        if let Some(f) = &self.face_set {
            if dead {
                return (Some(f.dead.clone()), 1);
            }
            if now < self.grin_until {
                return (Some(f.grin[pain].clone()), 1);
            }
            if now < self.ouch_until {
                return (Some(f.ouch[pain].clone()), 1);
            }
            return (Some(f.pain[pain][self.face_look as usize].clone()), 1);
        }
        // Doom's own while there is none of yours
        let name = if dead {
            "STFDEAD0".to_string()
        } else if now < self.grin_until {
            format!("STFEVL{pain}")
        } else {
            format!("STFST{pain}{}", self.face_look)
        };
        (self.pt(&name), S)
    }
    fn status_bar(&mut self) {
        let bar = self.pt("STBAR");
        self.blit(bar, 0, 168 * S, None, S);
        let arms = self.pt("STARMS");
        self.blit(arms, 104 * S, 168 * S, None, S);
        let p = self.lvl.player.clone();
        self.num(p.ammo[1], 44, 171, "STT", false);
        self.num(p.health, 90, 171, "STT", true);
        self.num(p.armor, 221, 171, "STT", true);
        for i in 0..6 {
            // the arms: 3, the shotgun
            let n = self.pt(&format!("{}{}", if i == 1 { "STYSNUM" } else { "STGNUM" }, i + 2));
            self.blit(n, (111 + (i % 3) * 12) * S, (172 + (i / 3) * 10) * S, None, S);
        }
        for i in 0..3 {
            if p.keys[i] {
                let k = self.pt(&format!("STKEYS{i}"));
                self.blit(k, 239 * S, (171 + i as i32 * 10) * S, None, S);
            }
        }
        for (i, y) in [173, 179, 185, 191].into_iter().enumerate() {
            self.small_num(p.ammo[i], 288, y);
            self.small_num(p.max[i], 314, y);
        }
        let kills = p.kills + self.sc.kills;
        let (f, k) = self.face(p.health, p.dead, kills);
        self.blit(f, if k == 1 { 145 } else { 143 } * S, if k == 1 { 169 } else { 168 } * S, None, k);
    }

    // ---- for tests (fps.js's __fps): where you are, turn, step, through the door, the level from a camera
    /// put you somewhere: in the office in dots, in the level in its units
    pub fn set(&mut self, x: f64, y: f64, a: Option<f64>) {
        if self.world == World::Level {
            let p = &mut self.lvl.player;
            p.x = x;
            p.y = y;
            if let Some(a) = a {
                p.angle = a;
            }
        } else {
            self.me.x = x * U;
            self.me.y = y * U;
            if let Some(a) = a {
                self.me.a = a;
            }
        }
    }
    pub fn turn(&mut self, r: f64) {
        if self.world == World::Level {
            self.lvl.player.angle += r;
        } else {
            self.me.a += r;
        }
    }
    /// straight through the door into the level
    pub fn into_level(&mut self) {
        if self.lvl.portal.is_none() {
            return;
        }
        self.door = Door { open: DOORH, dir: 0, wait: 150 };
        self.sync_portal();
        let (_, h, _) = self.units();
        self.me = Me { x: self.door_x(), y: h + 1.0, a: PI / 2.0, ..self.me };
        self.through_to_level();
    }
    /// the level from there, as a picture (the view only, the normal palette)
    pub fn snap(&mut self, cam: LCam) -> Image {
        let sc = self.sc.clone();
        let r = Ctx3 { wad: &self.wad, door_wad: self.door_wad.as_ref(), lvl: &self.lvl, sc: &sc, door_open: self.door.open, time: self.time, puffs: &self.puffs, now: self.now };
        self.view.light_grid(&r);
        self.view.render_level(&r, cam, None);
        let mut im = Image::new(SW as u32, VH as u32);
        for (o, &i) in im.px.iter_mut().zip(self.view.fb.iter()) {
            *o = self.wad.rgb[0][i as usize];
        }
        im
    }
    /// the level from there: how many pixels nothing drew on (a hole in the view), and the first
    pub fn hom(&mut self, cam: LCam) -> (usize, i32, i32) {
        self.snap(cam);
        let (mut n, mut fx, mut fy) = (0, -1, -1);
        for (i, &z) in self.view.zb.iter().enumerate() {
            if z >= 1e9 {
                n += 1;
                if fx < 0 {
                    fx = i as i32 % SW;
                    fy = i as i32 / SW;
                }
            }
        }
        (n, fx, fy)
    }
}

fn u8c(v: f64) -> u8 {
    if v.is_nan() { 0 } else { v.clamp(0.0, 255.0).round_ties_even() as u8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection() {
        // 90 degrees across 640 pixels: straight ahead the middle, 45 degrees each side the edges
        assert_eq!(to_x(0.0), 320);
        assert_eq!(to_x(PI / 4.0), 0);
        assert_eq!(to_x(2.0 * PI - PI / 4.0), 640);
        // a span wholly in view, one cut at the left edge, one behind
        assert_eq!(clip_angles(0.2, 2.0 * PI - 0.2), Some((to_x(0.2), to_x(2.0 * PI - 0.2))));
        assert_eq!(clip_angles(1.2, 2.0 * PI - 0.2), Some((0, to_x(2.0 * PI - 0.2))));
        assert_eq!(clip_angles(PI - 0.1, PI + 0.1), None);
        assert!((norm(-0.5) - (2.0 * PI - 0.5)).abs() < 1e-12);
    }

    #[test]
    fn light_bands() {
        // full light up close: the brightest map; far off, darker; no light: the darkest
        assert_eq!(wall_level(15, 1.0), 0);
        assert_eq!(wall_level(15, 2560.0), 0);
        assert_eq!(wall_level(10, 100.0), 8);
        assert_eq!(wall_level(0, 10000.0), 31);
        assert_eq!(plane_level(12, 16.0), 0);
        assert_eq!(plane_level(12, 1000.0), 11);
        assert!(plane_level(8, 4000.0) > plane_level(8, 100.0));
    }

    #[test]
    fn the_door_joins_office_and_level() {
        let w = Wad::open_bytes(crate::wad::freedoom()).unwrap();
        let f = Fps::new(w, None, Arc::new(Scene::empty(256.0, 180.0, 26.0)), Start { x: 128.0, y: 150.0, a: 0.0 }, Options::default()).unwrap();
        let r = f.ctx3();
        assert!(r.p().is_some(), "E1M1 has a place for the door");
        for (x, y) in [(100.0, 300.0), (173.25, 405.0), (400.0, 120.0)] {
            let (lx, ly) = r.o2l(x, y);
            let (ox, oy) = r.l2o(lx, ly);
            assert!((ox - x).abs() < 1e-9 && (oy - y).abs() < 1e-9);
        }
        for a in [0.0, 1.0, -2.5, 3.0] {
            let b = r.a2o(r.a2l(a));
            assert!((a - b).sin().abs() < 1e-9 && (a - b).cos() > 0.0);
        }
        // the door's middle in the office is the portal's middle in the level
        let (lx, ly) = r.o2l(r.door_x(), r.units().1);
        let p = r.p().unwrap();
        assert!((lx - p.cx).abs() < 1e-9 && (ly - p.cy).abs() < 1e-9);
    }
}
