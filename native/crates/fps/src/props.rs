//! The things of the 3d view (a port of web/props3d.js), made for it rather than taken from the room's pixel art: no
//! ink outlines, four texels a dot, shaded. The robots, the office managers and the dog are little models of boxes,
//! drawn from eight sides (Doom's sprite angles), each robot with its own head (its antenna, its eyes, its mouth:
//! pixel.js picks them from its seed, as for its portrait), in its own colour, in the pose the room gives it. The
//! furniture is boxes faced with textures painted here (wood with its grain, steel, plastic, the screens and lamps
//! that shine); what is round (a plant, the pair table, the flames) is a picture, the same from every side, as Doom's
//! lamps and barrels. The wall is painted here too.
//!
//! A texel whose alpha is GLOW (254) shines: the view does not light it (screens, eyes, the fire).
//! (props3d.js's floor and ceiling tiles and its `plain` wall were never drawn by fps.js: not here.)

use crate::font;
use crate::rng::jround;
use crate::scene::{Block, Face, Figure, GLOW, Light, Paint, Parts, Robot, TS, Tex, Thing, WallItem, WallKind};
use pix::{Color, Ctx, Image, Path};
use std::collections::HashMap;
use std::f64::consts::PI;

type Rgb = [f64; 3];
type Px = Option<[f64; 4]>;
const GL: f64 = GLOW as f64;
/// a screen's snow, its machine out of reach (the room's SNOW)
const SNOW: [Rgb; 7] = [[16.0, 20.0, 31.0], [16.0, 20.0, 31.0], [59.0, 58.0, 85.0], [59.0, 58.0, 85.0], [93.0, 98.0, 128.0], [139.0, 147.0, 173.0], [194.0, 201.0, 217.0]];

fn hex(h: &str) -> Rgb {
    let c = pix::css(h).0;
    [c[0] as f64, c[1] as f64, c[2] as f64]
}
fn rgb_of(c: Color) -> Rgb {
    [c.0[0] as f64, c.0[1] as f64, c.0[2] as f64]
}
fn mul(a: Rgb, k: f64) -> Rgb {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn mix(a: Rgb, b: Rgb, k: f64) -> Rgb {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}
fn c3(a: Rgb) -> Px {
    Some([a[0], a[1], a[2], 255.0])
}
fn g3(a: Rgb) -> Px {
    Some([a[0], a[1], a[2], GL])
}
/// a CSS colour as props3d.js wrote it: `rgba(${a[0] | 0}, …)`, cut, not rounded
fn css(a: Rgb, al: f64) -> Color {
    let c = |v: f64| (v as i64).clamp(0, 255) as u8;
    Color([c(a[0]), c(a[1]), c(a[2]), (al.clamp(0.0, 1.0) * 255.0).round() as u8])
}
/// a Uint8ClampedArray's store
fn u8c(v: f64) -> u8 {
    if v.is_nan() { 0 } else { v.clamp(0.0, 255.0).round_ties_even() as u8 }
}
/// JavaScript's `x | 0`
fn to_i32(x: f64) -> i32 {
    if x.is_finite() { (x.trunc() as i64) as i32 } else { 0 }
}
/// props3d.js's hash, bit for bit: 0 ≤ h < 1
pub fn hash(x: f64, y: f64, s: f64) -> f64 {
    let h = (to_i32(x) as u32)
        .wrapping_mul(374761393)
        .wrapping_add((to_i32(y) as u32).wrapping_mul(668265263))
        .wrapping_add((to_i32(s) as u32).wrapping_mul(1013904223));
    let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f64 / 4294967296.0
}
fn hashi(x: i32, y: i32, s: i32) -> f64 {
    hash(x as f64, y as f64, s as f64)
}
// Doom's grain: a texel's light varies, speckle over blotches a few texels across over stains larger still, now and
// then a pit; k how much
fn grit(i: i32, j: i32, s: i32, k: f64) -> f64 {
    let stain = hashi(i >> 3, j >> 3, s + 3);
    let blot = hashi(i >> 2, j >> 2, s) * 0.6 + hashi(i >> 1, j >> 1, s + 1) * 0.4;
    let f = hashi(i, j, s + 2);
    1.0 + k * ((stain - 0.5) * 0.1 + (blot - 0.5) * 0.18 + (f - 0.5) * 0.16 - if f < 0.035 { 0.2 } else { 0.0 })
}

/// pixel.js's colours that the models use
mod c {
    pub const INK2: &str = "#2a2140";
    pub const RED1: &str = "#e8402a";
    pub const BLUE0: &str = "#1a2448";
    pub const GREEN0: &str = "#2d8a3a";
}

/// a robot's parts, as pixel.js's robot() picks them from its seed (in its order), for a model of it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RobotParts {
    pub top: i32,
    pub ant: &'static str,
    pub eyes: &'static str,
    pub teeth: bool,
}
/// pixel.js's rng, bit for bit
fn rng(seed: &str) -> impl FnMut() -> f64 {
    let mut h: u32 = 2166136261;
    for ch in seed.chars() {
        h = (h ^ ch as u32).wrapping_mul(16777619);
    }
    move || {
        h = (h ^ (h >> 15)).wrapping_mul(2246822507);
        h = (h ^ (h >> 13)).wrapping_mul(3266489909);
        h ^= h >> 16;
        h as f64 / 4294967296.0
    }
}
pub fn robot_parts(seed: &str) -> RobotParts {
    let mut r = rng(seed);
    let mut pick = |n: usize| ((r() * n as f64) as usize).min(n - 1);
    let top = [3, 4][pick(2)];
    let ant = ["one", "two", "dish"][pick(3)];
    let eyes = ["dots", "visor", "cyclops", "happy"][pick(4)];
    let teeth = r() < 0.5;
    RobotParts { top, ant, eyes, teeth }
}

/// a picture of a figure or a round thing: where it stands in it, texels a dot
#[derive(Clone, Debug)]
pub struct Fig {
    pub tex: Tex,
    pub w: f64,
    pub h: f64,
    pub ax: f64,
    pub ay: f64,
    pub res: f64,
}

/// what a piece of furniture shows (room.js fills it from the room's state; only what its kind uses)
#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub kind: String,
    pub key: String,
    pub x: f64,
    pub y: f64,
    /// the room's clock, seconds (screens scroll by it)
    pub t_now: f64,
    /// coffee: the five-hour session's left, cooler: the week's (0..1)
    pub level: Option<f64>,
    pub brewing: bool,
    /// the printer: how far a page is out
    pub printing: Option<f64>,
    /// the manager's reception: her name, her colour, her screen (busy, asks, on, off), her stack of notes
    pub name: String,
    pub color: Option<String>,
    pub screen: Option<String>,
    pub notes: u32,
    /// the project cabinet's width
    pub w: Option<f64>,
    /// the booth: hardcore's gun closet, the guns out of it, how long it has been in use (seconds)
    pub gun: bool,
    pub out: u32,
    pub t: Option<f64>,
    /// the neuralyzer flashing
    pub flash: bool,
    /// the radio on
    pub on: bool,
    /// the chess board as it stands (64 letters), the seats at chess and the meeting table
    pub board: String,
    pub seats: Vec<(f64, f64)>,
    /// the painting on the easel (its canvas, a dot a texel), its version
    pub art: Option<Tex>,
    pub art_v: u64,
    pub variant: u32,
    /// the pair table linked to a room
    pub linked: bool,
}

/// a project's desk: room.js gives what it shows
#[derive(Clone, Debug)]
pub struct Desk {
    pub x0: f64,
    pub y: f64,
    pub w: f64,
    pub cab: f64,
    pub label: String,
    pub paint: Paint,
    pub color: String,
    pub t_now: f64,
    /// the seats' x
    pub seats: Vec<f64>,
    /// the mugs brought: by which seat's x, in whose colour
    pub mugs: Vec<(f64, String)>,
    /// off, on, busy, waits
    pub screen: String,
    pub laptops: Vec<String>,
}

// ---- materials: a face w x h dots of them, a texture of its own (kept by size)
#[derive(Clone, Copy)]
enum Mat<'a> {
    Wood(&'a str, f64),
    Plain(&'a str, f64),
}
/// the faces of a block given apart from its material
#[derive(Default)]
struct Over {
    front: Option<Face>,
    back: Option<Face>,
    side: Option<Face>,
    top: Option<Face>,
    west: Option<Face>,
    east: Option<Face>,
}

/// a box of a little model, in a figure's own frame (f forward, r to its right, z up); a painter for its front
struct MBox {
    f0: f64,
    f1: f64,
    r0: f64,
    r1: f64,
    z0: f64,
    z1: f64,
    col: Rgb,
    metal: bool,
    glow: bool,
    round: f64,
    paint: Front,
}

/// a painter for a box's front (u across it as you see it, v down)
type Front = Option<Box<dyn Fn(f64, f64) -> Px>>;
/// adds a robot's box: f0, f1, r0, r1, z0, z1, its colour, flat, glow, round, its front's painter
type Push<'a> = dyn FnMut(f64, f64, f64, f64, f64, f64, Rgb, bool, bool, f64, Front) + 'a;
/// adds a dog's box: f0, f1, r0, r1, z0, z1, its colour, its front's painter
type DogPush<'a> = dyn FnMut(f64, f64, f64, f64, f64, f64, Rgb, Front) + 'a;
const ROBOT: (f64, f64, f64) = (4.0, 9.0, 27.0); // S, R, Z
const DOG: (f64, f64, f64) = (4.0, 13.0, 14.0);
const WOODP_HI: &str = "#cfa86d";
const WOODP_TOP: &str = "#b38857";
const WOODP_D: &str = "#885c47";
const WOODP_K: &str = "#532e3d";

/// painting onto a picture in dots: r() fills, t() letters (props3d.js's `over`)
pub struct Painter<'a> {
    pub ctx: Ctx<'a>,
}
impl Painter<'_> {
    fn r(&mut self, px: f64, py: f64, pw: f64, ph: f64, col: Rgb, glow: bool) {
        let c = css(col, if glow { GL / 255.0 } else { 1.0 });
        self.ctx.fill_rect(jround(px * TS), jround(py * TS), jround(pw * TS), jround(ph * TS), c);
    }
    /// the room's letters, k texels a font pixel (a dot is TS)
    fn t(&mut self, s: &str, px: f64, py: f64, col: Rgb, k: f64) {
        let w = font::tiny_w(s);
        let (x0, y0, c) = (jround(px * TS), jround(py * TS), css(col, 1.0));
        let ctx = &mut self.ctx;
        font::tiny(s, 0, 0, |x, y| {
            if x < w {
                ctx.fill_rect(x0 + x as f64 * k, y0 + y as f64 * k, k, k, c);
            }
        });
    }
}

/// The models and textures, kept once made (props3d.js's memo).
#[derive(Default)]
pub struct Props {
    seeds: i32,
    cache: HashMap<String, Tex>,
    figs: HashMap<String, Fig>,
    wall: Option<(String, Tex)>,
    board: Option<(u64, Tex)>,
}

impl Props {
    pub fn new() -> Props {
        Props::default()
    }
    fn memo(&mut self, k: &str, f: impl FnOnce(&mut Props) -> Tex) -> Tex {
        if let Some(t) = self.cache.get(k) {
            return t.clone();
        }
        let t = f(self);
        self.cache.insert(k.to_string(), t.clone());
        t
    }
    fn face(&mut self, k: &str, f: impl FnOnce(&mut Props) -> Image) -> Face {
        Face::whole(self.memo(k, |p| Tex::new(f(p))))
    }

    // a picture texel by texel: f(i, j) gives [r, g, b, a], or nothing for clear; grained (what shines less)
    fn pix(&mut self, w: f64, h: f64, mut f: impl FnMut(i32, i32) -> Px, k: f64) -> Image {
        let (w, h) = ((w as u32).max(1), (h as u32).max(1));
        let mut im = Image::new(w, h);
        self.seeds += 1;
        let s = self.seeds * 7;
        for j in 0..h as i32 {
            for i in 0..w as i32 {
                let Some(v) = f(i, j) else { continue };
                let a = v[3];
                let g = grit(i, j, s, if a == GL { k * 0.35 } else { k });
                im.px[(j as u32 * w + i as u32) as usize] = [u8c(v[0] * g), u8c(v[1] * g), u8c(v[2] * g), u8c(a)];
            }
        }
        im
    }
    // and painted on afterwards, in dots; what was painted on gets the grain too
    fn over(&mut self, mut im: Image, paint: impl FnOnce(&mut Painter)) -> Image {
        {
            let mut p = Painter { ctx: im.ctx() };
            paint(&mut p);
        }
        self.seeds += 1;
        let s = self.seeds * 7;
        let w = im.w as i32;
        for (n, q) in im.px.iter_mut().enumerate() {
            if q[3] == 255 {
                let g = grit(n as i32 % w, n as i32 / w, s, 0.35);
                for c in q.iter_mut().take(3) {
                    *c = u8c(*c as f64 * g);
                }
            }
        }
        im
    }

    // wood: its grain along the length, boards a few dots tall, a bevel at the edges
    fn wood_tex(&mut self, w: f64, h: f64, base: &str, board: f64) -> Image {
        let b = hex(base);
        let (bw, ww, hh) = (board * TS, w * TS, h * TS);
        self.pix(
            ww,
            hh,
            |i, j| {
                let (fi, fj) = (i as f64, j as f64);
                let row = (fj / bw).floor();
                let y = fj % bw;
                let g = ((fj + 3.0 * (fi * 0.045 + row * 2.1).sin()) * 0.95 + row).sin() * 0.05
                    + (hash((i >> 1) as f64, fj, row) - 0.5) * 0.07
                    + (hash(row, 7.0, 0.0) - 0.5) * 0.1;
                let mut k = 0.95 + g;
                if y == 0.0 {
                    k *= 0.72;
                } else if y == 1.0 {
                    k *= 1.1;
                }
                if i < 2 || j < 2 {
                    k *= 1.12;
                } else if fi >= ww - 2.0 || fj >= hh - 2.0 {
                    k *= 0.75;
                }
                c3(mul(b, k))
            },
            0.75,
        )
    }
    // metal or plastic: smooth, a fine brushed noise, the bevel (less grain than wood)
    fn plain_tex(&mut self, w: f64, h: f64, base: &str, rough: f64) -> Image {
        let b = hex(base);
        let (ww, hh) = (w * TS, h * TS);
        self.pix(
            ww,
            hh,
            |i, j| {
                let mut k = 1.0 + (hashi(0, j, 3) - 0.5) * rough + (hashi(i, j, 5) - 0.5) * rough * 0.6;
                if i < 1 || j < 1 {
                    k *= 1.18;
                } else if i as f64 >= ww - 1.0 || j as f64 >= hh - 1.0 {
                    k *= 0.7;
                }
                c3(mul(b, k))
            },
            0.5,
        )
    }
    fn mat(&mut self, m: Mat, w: f64, h: f64) -> Face {
        match m {
            Mat::Wood(base, board) => self.face(&format!("w{base}{board}:{w}x{h}"), |p| p.wood_tex(w, h, base, board)),
            Mat::Plain(base, rough) => self.face(&format!("p{base}{rough}:{w}x{h}"), |p| p.plain_tex(w, h, base, rough)),
        }
    }
    // a material whose texels depend on a state (glass, water: little grain)
    fn tinted(&mut self, k: &str, w: f64, h: f64, f: impl Fn(i32, i32, f64, f64) -> Px) -> Face {
        self.face(&format!("b:{k}{w}x{h}"), |p| p.pix(w * TS, h * TS, |i, j| f(i, j, w, h), 0.35))
    }
    /// a box: x0..x1 (west to east), y0..y1 (north to south), z0..z1 (up); its faces from a material, any of them
    /// given apart (front: the south face, as the room is seen)
    #[allow(clippy::too_many_arguments)]
    fn blk(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, z0: f64, z1: f64, m: Mat, o: Over) -> Block {
        let (w, d, h) = ((x1 - x0).max(0.25), (y1 - y0).max(0.25), (z1 - z0).max(0.25));
        let r = |v: f64| (jround(v * 2.0) / 2.0).max(1.0);
        let front = Some(o.front.unwrap_or_else(|| self.mat(m, r(w), r(h))));
        let back = Some(o.back.unwrap_or_else(|| self.mat(m, r(w), r(h))));
        let side = Some(o.side.unwrap_or_else(|| self.mat(m, r(d), r(h))));
        let top = Some(o.top.unwrap_or_else(|| self.mat(m, r(w), r(d))));
        Block { x0, y0, x1, y1, z0, z1, front, back, side, top, west: o.west, east: o.east, ..Default::default() }
    }
    /// a block faced with one picture on every side (the pong table's net)
    fn blk_tex(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, z0: f64, z1: f64, f: Face) -> Block {
        Block { x0, y0, x1, y1, z0, z1, front: Some(f.clone()), back: Some(f.clone()), side: Some(f.clone()), top: Some(f), ..Default::default() }
    }

    // ---- the little models, drawn from eight sides into a picture each. A face's colour is its box's, or a
    // painter's (u across it as you see it, v down)
    fn render(&mut self, boxes: &[MBox], k: usize, (s, rr, zz): (f64, f64, f64)) -> Image {
        let th = k as f64 * PI / 4.0;
        let (cf, cr, sf, sr) = (th.cos(), th.sin(), th.sin(), -th.cos());
        let (w, h) = ((2.0 * rr * s).ceil() as i32, (zz * s).ceil() as i32);
        let mut zb = vec![-1e9f32; (w * h) as usize]; // (a Float32Array in props3d.js)
        let mut im = Image::new(w as u32, h as u32);
        let (lf, lr) = (0.82 * cf - 0.57 * sf, 0.82 * cr - 0.57 * sr); // the light: from the front, a little from your left
        for b in boxes {
            for (nf, nr, side) in [(1.0, 0.0, 'F'), (-1.0, 0.0, 'B'), (0.0, 1.0, 'R'), (0.0, -1.0, 'L')] {
                if nf * cf + nr * cr <= 1e-6 {
                    continue;
                }
                let pa = match side {
                    'F' => [b.f1, b.r0],
                    'B' => [b.f0, b.r1],
                    'R' => [b.f1, b.r1],
                    _ => [b.f0, b.r0],
                };
                let pb = match side {
                    'F' => [b.f1, b.r1],
                    'B' => [b.f0, b.r0],
                    'R' => [b.f0, b.r1],
                    _ => [b.f1, b.r0],
                };
                let (mut xa, mut xb, mut p0, mut p1) = (pa[0] * sf + pa[1] * sr, pb[0] * sf + pb[1] * sr, pa, pb);
                if xa > xb {
                    std::mem::swap(&mut xa, &mut xb);
                    std::mem::swap(&mut p0, &mut p1);
                }
                let lam = (nf * lf + nr * lr).max(0.0);
                let shade = 0.56 + 0.44 * lam;
                let x0 = (jround((xa + rr) * s) as i32).max(0);
                let x1 = (jround((xb + rr) * s) as i32).min(w);
                let y0 = (jround((zz - b.z1) * s) as i32).max(0);
                let y1 = (jround((zz - b.z0) * s) as i32).min(h);
                let paint = if side == 'F' { b.paint.as_ref() } else { None };
                let (rd, fw, fh) = (b.round, (pb[0] - pa[0]).hypot(pb[1] - pa[1]), b.z1 - b.z0);
                let span = (xb - xa) * s;
                for x in x0..x1 {
                    let u = ((x as f64 + 0.5 - (xa + rr) * s) / if span == 0.0 { 1.0 } else { span }).clamp(0.0, 0.999);
                    let (pf, pr) = (p0[0] + (p1[0] - p0[0]) * u, p0[1] + (p1[1] - p0[1]) * u);
                    let dep = pf * cf + pr * cr;
                    for y in y0..y1 {
                        let i = (y * w + x) as usize;
                        if dep <= zb[i] as f64 {
                            continue;
                        }
                        let v = ((y as f64 + 0.5 - y0 as f64) / if y1 == y0 { 1.0 } else { (y1 - y0) as f64 }).min(0.999);
                        if rd != 0.0 {
                            // its corners rounded
                            let (du, dv) = (rd - u.min(1.0 - u) * fw, rd - v.min(1.0 - v) * fh);
                            if du > 0.0 && dv > 0.0 && du * du + dv * dv > rd * rd {
                                continue;
                            }
                        }
                        let (mut rgb, mut glow) = (b.col, b.glow);
                        if let Some(pt) = paint {
                            let Some(q) = pt(u, v) else { continue };
                            rgb = [q[0], q[1], q[2]];
                            glow = q[3] == GL;
                        }
                        let mut k2 = if glow {
                            1.0
                        } else {
                            shade
                                * if y - y0 < 1 {
                                    1.16
                                } else if y1 - y <= 1 {
                                    0.84
                                } else {
                                    1.0
                                }
                                * if x - x0 < 1 || x1 - x <= 1 { 0.93 } else { 1.0 }
                        };
                        if !glow {
                            k2 *= grit(x, y, 11, 0.55) * if b.metal { 1.12 - 0.32 * v + if v > 0.12 && v < 0.2 { 0.14 } else { 0.0 } } else { 1.0 };
                        }
                        zb[i] = dep as f32;
                        im.px[i] = [u8c((rgb[0] * k2).min(255.0)), u8c((rgb[1] * k2).min(255.0)), u8c((rgb[2] * k2).min(255.0)), if glow { GLOW } else { 255 }];
                    }
                }
            }
        }
        im
    }

    /// a figure from one of the eight sides (k: 0 its front, counting round to its right), kept
    pub fn figure(&mut self, sp: &Figure, k: usize) -> Fig {
        let key = match sp {
            Figure::Robot(r) => format!("robot|{}|{}|{}|{}|{}|{}|{k}", r.seed, r.color, r.femme, r.arms, r.legs, r.red),
            Figure::Dog { pose, wag } => format!("dog|{pose}|{wag}|{k}"),
        };
        if let Some(v) = self.figs.get(&key) {
            return v.clone();
        }
        let (boxes, d) = match sp {
            Figure::Robot(r) => (robot_boxes(r), ROBOT),
            Figure::Dog { pose, wag } => (dog_boxes(pose, *wag), DOG),
        };
        let im = self.render(&boxes, k, d);
        let (w, h) = (im.w as f64, im.h as f64);
        let v = Fig { tex: Tex::new(im), w, h, ax: w / 2.0, ay: h - 1.0, res: d.0 };
        if self.figs.len() > 4000 {
            self.figs.clear();
        }
        self.figs.insert(key, v.clone());
        v
    }
    /// lying down (a nap): its side, turned a quarter, head first
    pub fn lying(&mut self, sp: &Robot) -> Fig {
        let key = format!("lie|{}{}{}{}", sp.seed, sp.color, sp.femme, sp.red);
        if let Some(v) = self.figs.get(&key) {
            return v.clone();
        }
        let s = self.figure(&Figure::Robot(Robot { arms: "down".into(), legs: "stand".into(), ..sp.clone() }), 2);
        let src = &s.tex.img;
        let mut im = Image::new(src.h, src.w);
        for sy in 0..src.h as i32 {
            for sx in 0..src.w as i32 {
                im.set(sy, src.w as i32 - 1 - sx, src.get(sx, sy));
            }
        }
        let (w, h) = (im.w as f64, im.h as f64);
        let v = Fig { tex: Tex::new(im), w, h, ax: w / 2.0, ay: h * 0.5 + 3.0 * ROBOT.0, res: ROBOT.0 };
        self.figs.insert(key, v.clone());
        v
    }

    // ---- what is round: pictures, the same from every side
    // the pouf's top (the pouf is a box, square as in the room): its green, a button in the middle
    fn pouf_top(&mut self) -> Face {
        self.face("poufTop", |p| {
            let b = p.plain_tex(7.0, 7.0, "#6fa374", 0.06);
            p.over(b, |r| r.r(3.0, 3.0, 1.0, 1.0, hex("#355f31"), false))
        })
    }
    /// a potted plant: a terracotta pot, long leaves (a snake plant) or broad ones on stems
    pub fn plant(&mut self, kind: u32) -> Fig {
        let s = TS;
        let (w, h) = (18.0 * s, 32.0 * s);
        let tex = self.memo(&format!("plant{kind}"), |_| {
            let mut im = Image::new(w as u32, h as u32);
            {
                let mut x = im.ctx();
                let leaf = |x: &mut Ctx, bx: f64, by: f64, tx: f64, ty: f64, wid: f64, col: Rgb| {
                    // a leaf from its base to its tip, a lighter vein down it
                    let n = (tx - bx).hypot(ty - by).ceil();
                    let mut k = 0.0;
                    while k <= n {
                        let t = k / n;
                        let (cx, cy) = (bx + (tx - bx) * t, by + (ty - by) * t);
                        let ww = wid * (PI * (t * 1.15 + 0.05).min(1.0)).sin() + 0.5;
                        x.fill_rect(jround(cx - ww), jround(cy), jround(2.0 * ww), 1.0, css(mul(col, 0.8 + 0.3 * t), 1.0));
                        x.fill_rect(jround(cx), jround(cy), 1.0, 1.0, css(mul(col, 1.25), 1.0));
                        k += 1.0;
                    }
                };
                if kind != 0 {
                    // broad leaves on stems
                    let g = [72.0, 150.0, 80.0];
                    for (a, l) in [(-1.2f64, 15.0), (-0.5, 19.0), (0.2, 20.0), (0.8, 17.0), (1.3, 13.0), (-0.1, 12.0)] {
                        let (ex, ey) = (w / 2.0 + a.sin() * l * s, h - 9.0 * s - a.cos() * l * s);
                        for k in 0..20 {
                            let t = k as f64 / 20.0;
                            x.fill_rect(jround(w / 2.0 + (ex - w / 2.0) * t * 0.7), jround(h - 9.0 * s + (ey - h + 9.0 * s) * t * 0.7), 2.0, 2.0, css([60.0, 110.0, 60.0], 1.0));
                        }
                        let k = 0.85 + hash(a * 10.0, 1.0, 0.0) * 0.3;
                        leaf(&mut x, w / 2.0 + (ex - w / 2.0) * 0.55, h - 9.0 * s + (ey - h + 9.0 * s) * 0.55, ex, ey, 4.5 * s * 0.5, mul(g, k));
                    }
                } else {
                    // a snake plant: tall pointed leaves
                    let g = [58.0, 128.0, 72.0];
                    for (dx, l, lean) in [(-2.5f64, 20.0, -0.25), (-0.8, 24.0, -0.08), (0.8, 22.0, 0.1), (2.4, 18.0, 0.28), (0.0, 16.0, 0.0), (-1.6, 14.0, -0.4), (1.8, 13.0, 0.45)] {
                        let k = 0.8 + hash(dx * 10.0, 3.0, 0.0) * 0.35;
                        leaf(&mut x, w / 2.0 + dx * s, h - 9.0 * s, w / 2.0 + (dx + lean * l) * s, h - 9.0 * s - l * s, 1.4 * s, mul(g, k));
                    }
                }
                let y0 = h - 9.0 * s;
                for j in 0..(9.0 * s) as i32 {
                    let jf = j as f64;
                    let hw = (6.5 - jf / s * 0.25) * s;
                    let k = 1.0 - jf / (9.0 * s) * 0.25;
                    let mut i = jround(w / 2.0 - hw);
                    while i < w / 2.0 + hw {
                        let l = 1.08 - (i - w / 2.0).abs() / hw * 0.35;
                        x.fill_rect(i, y0 + jf, 1.0, 1.0, css(mul([196.0, 112.0, 74.0], k * l), 1.0));
                        i += 1.0;
                    }
                }
                x.fill_rect(w / 2.0 - 7.0 * s, y0, 14.0 * s, s, css([150.0, 80.0, 52.0], 1.0));
                x.fill_rect(w / 2.0 - 6.0 * s, y0 - s * 0.5, 12.0 * s, s, css([74.0, 52.0, 40.0], 1.0));
            }
            for p in im.px.iter_mut() {
                p[3] = if p[3] < 100 { 0 } else { 255 }; // crisp: no half-covered texels
            }
            Tex::new(im)
        });
        Fig { tex, w, h, ax: w / 2.0, ay: h - 1.0, res: TS }
    }
    /// the pair table: round, on a pedestal, a speakerphone on it (lit while a room is linked)
    pub fn pair_table(&mut self, linked: bool) -> Fig {
        let (w, h) = (26.0 * TS, 13.0 * TS);
        let wd = hex("#b38857");
        let tex = self.memo(&format!("pair{linked}"), |p| {
            Tex::new(p.pix(
                w,
                h,
                |i, j| {
                    let (fi, fj) = (i as f64, j as f64);
                    let u = (fi + 0.5 - w / 2.0) / (w / 2.0 - 2.0);
                    if (4.0 * TS..6.0 * TS).contains(&fj) && u.abs() <= 1.0 {
                        return c3(mul(wd, if fj < 4.5 * TS { 1.15 } else { 0.95 } - u.abs() * 0.2)); // the top's edge
                    }
                    if fj >= 6.0 * TS && (fi - w / 2.0).abs() < 1.2 * TS {
                        return c3(mul(wd, 0.7 - (fi - w / 2.0) / TS * 0.05)); // the pedestal
                    }
                    if fj >= h - 1.5 * TS && (fi - w / 2.0).abs() < 6.0 * TS {
                        return c3(mul(wd, 0.6)); // its foot
                    }
                    if (1.6 * TS..4.0 * TS).contains(&fj) && (fi - w / 2.0).abs() < 3.0 * TS {
                        // the speakerphone
                        return if fj < 2.2 * TS && (fi - w / 2.0).abs() < TS {
                            if linked { g3([130.0, 255.0, 160.0]) } else { c3([70.0, 80.0, 74.0]) }
                        } else {
                            c3([62.0, 66.0, 70.0])
                        };
                    }
                    None
                },
                1.0,
            ))
        });
        Fig { tex, w, h, ax: w / 2.0, ay: h - 1.0, res: TS }
    }
    /// the fire's flames: tongues rising from the pit, in four frames; they shine
    pub fn flames(&mut self, frame: u32, big: bool) -> Fig {
        let (w, h) = (20.0 * TS, if big { 22.0 } else { 15.0 } * TS);
        let tex = self.memo(&format!("fl{frame}{big}"), |p| {
            Tex::new(p.pix(
                w,
                h,
                |i, j| {
                    let u = (i as f64 - w / 2.0) / (w / 2.0);
                    let up = (h - j as f64) / h;
                    let t = frame as f64 * 1.7;
                    let tongue = 0.5 + 0.22 * (u * 11.0 + t).sin() + 0.2 * (u * 23.0 - t * 1.3).sin() + 0.08 * (u * 41.0 + t * 2.0).sin() + (hashi(i >> 2, frame as i32, 9) - 0.5) * 0.12;
                    let reach = (1.0 - u * u) * tongue;
                    if up > reach || u.abs() > 0.95 {
                        return None;
                    }
                    let q = up / reach.max(0.01);
                    let col = if q < 0.35 {
                        [255.0, 246.0, 196.0]
                    } else if q < 0.6 {
                        [255.0, 214.0, 92.0]
                    } else if q < 0.82 {
                        [244.0, 140.0, 40.0]
                    } else {
                        [196.0, 62.0, 28.0]
                    };
                    g3(col)
                },
                1.0,
            ))
        });
        Fig { tex, w, h, ax: w / 2.0, ay: h - 1.0, res: TS }
    }

    // ---- the screens: a screen's picture by what it shows (off, on, busy, waits, snow: its machine out of reach), in
    // the desk's colour
    fn screen_tex(&mut self, w: f64, h: f64, state: &str, color: &str, t: f64) -> Face {
        let frame = match state {
            "busy" => (t * 5.0).floor() as i64 % 26,
            "waits" => (t * 2.0).floor() as i64 % 2,
            "snow" => (t * 8.0).floor() as i64 % 6,
            _ => 0,
        };
        let (state, color) = (state.to_string(), color.to_string());
        self.face(&format!("scr{w}x{h}{state}{color}{frame}"), move |p| p.screen_paint(w, h, &state, &color, frame))
    }
    fn screen_paint(&mut self, w: f64, h: f64, state: &str, color: &str, frame: i64) -> Image {
        let col = hex(color);
        let on = state != "off" && state != "snow";
        let c = self.pix(
            w * TS,
            h * TS,
            |i, j| {
                let (fi, fj, bz) = (i as f64, j as f64, 1.2 * TS);
                if fi < bz || fj < bz || fi >= w * TS - bz || fj >= h * TS - bz {
                    return c3([40.0, 42.0, 52.0]);
                }
                if state == "snow" {
                    // a dot of the room's screen a texel block, as the room draws it
                    let ts = TS as i32;
                    let mut n = (i / ts + 1).wrapping_mul(374761393).wrapping_add((j / ts + 1).wrapping_mul(668265263)).wrapping_add((frame as i32 + 1).wrapping_mul(1274126177));
                    n = (n ^ ((n as u32) >> 13) as i32).wrapping_mul(1103515245);
                    let u = ((n ^ ((n as u32) >> 16) as i32) as u32) as f64 / 4294967296.0;
                    return g3(SNOW[(u * SNOW.len() as f64) as usize]);
                }
                if !on {
                    return c3(if (i - j) % 9 == 0 && fi > w * TS * 0.6 && fj < h * TS * 0.4 { [44.0, 54.0, 78.0] } else { [16.0, 20.0, 31.0] });
                }
                g3([22.0, 32.0, 68.0])
            },
            1.0,
        );
        if !on {
            return c;
        }
        self.over(c, |r| {
            let rows = ((h - 3.0) / 1.6).floor() as i64;
            let s = if state == "busy" { frame } else { 0 };
            for k in 0..rows {
                let kf = k as f64;
                let len = (w - 4.0).min(2.0 + (((k * 7 + s) * 5) as f64 % (w - 5.0)));
                r.r(1.8 + ((k * 3 % 4) as f64) * 0.4, 1.8 + kf * 1.6, len, 0.6, if state == "busy" { col } else { mul(col, 0.5) }, true);
            }
            if state == "waits" && frame != 0 {
                r.r(1.8, h - 2.6, 1.6, 0.8, [244.0, 246.0, 251.0], true);
            }
        })
    }

    // ---- the furniture, by kind: boxes (and pictures, and what you can't walk through, and lights) at its place
    /// a piece of furniture as the view has it; None: a kind made otherwise (the room's own picture of it)
    pub fn build(&mut self, p: &Piece) -> Option<Parts> {
        let mut out = Parts::default();
        let (x, y) = (p.x, p.y);
        let steel = Mat::Plain("#8b93ad", 0.06);
        let dark_steel = Mat::Plain("#3f4452", 0.08);
        let black = Mat::Plain("#26242c", 0.08);
        let white = Mat::Plain("#e8ecf0", 0.03);
        let wood = Mat::Wood(WOODP_TOP, 4.0);
        let wood_d = Mat::Wood(WOODP_D, 4.0);
        let wood_hi = Mat::Wood(WOODP_HI, 6.0);
        macro_rules! b {
            ($x0:expr, $y0:expr, $x1:expr, $y1:expr, $z0:expr, $z1:expr, $m:expr) => {{
                let bl = self.blk($x0, $y0, $x1, $y1, $z0, $z1, $m, Over::default());
                out.boxes.push(bl);
            }};
            ($x0:expr, $y0:expr, $x1:expr, $y1:expr, $z0:expr, $z1:expr, $m:expr, $o:expr) => {{
                let bl = self.blk($x0, $y0, $x1, $y1, $z0, $z1, $m, $o);
                out.boxes.push(bl);
            }};
        }
        let key = p.key.clone();
        let pic = |out: &mut Parts, f: Fig, px: f64, py: f64, z: f64| {
            out.things.push(Thing {
                x: px,
                y: py,
                z,
                pic: Some(Face::whole(f.tex.clone())),
                ax: f.ax,
                ay: f.ay,
                res: f.res,
                piece: Some(key.clone()),
                ..Default::default()
            })
        };
        let wd = hex(WOODP_D);
        match p.kind.as_str() {
            "coffee" => {
                // a counter with doors, the brewer, the pot (the five-hour session's limit), mugs
                let front = self.face("b:coffee", |s| {
                    let t = s.wood_tex(20.0, 10.0, WOODP_TOP, 4.0);
                    s.over(t, |r| {
                        for dx in [0.6, 10.2] {
                            r.r(dx, 1.0, 9.2, 8.0, mul(wd, 1.05), false);
                            r.r(dx + 0.4, 1.4, 8.4, 7.2, hex(WOODP_TOP), false);
                        }
                        r.r(9.0, 4.5, 0.8, 1.6, [214.0, 198.0, 160.0], false);
                        r.r(10.6, 4.5, 0.8, 1.6, [214.0, 198.0, 160.0], false);
                    })
                });
                b!(x, y - 9.0, x + 20.0, y, 0.0, 10.0, wood, Over { front: Some(front), ..Default::default() });
                b!(x - 0.5, y - 9.5, x + 20.5, y + 0.5, 10.0, 11.0, wood_hi);
                let brewing = p.brewing;
                let brewer = self.face(&format!("b:brewer{brewing}"), |s| {
                    let t = s.plain_tex(7.0, 9.0, "#2b2a31", 0.06);
                    s.over(t, |r| {
                        r.r(1.0, 1.0, 5.0, 2.2, [70.0, 72.0, 84.0], false);
                        r.r(4.6, 1.6, 0.9, 0.9, if brewing { [255.0, 120.0, 80.0] } else { [235.0, 76.0, 63.0] }, true);
                        r.r(1.6, 4.0, 3.8, 3.6, [16.0, 14.0, 18.0], false);
                        r.r(2.4, 7.0, 2.2, 0.6, [90.0, 92.0, 100.0], false);
                    })
                });
                b!(x + 2.0, y - 8.0, x + 9.0, y - 3.0, 11.0, 20.0, black, Over { front: Some(brewer), ..Default::default() });
                let level = jround(p.level.unwrap_or(0.0) * 20.0) / 20.0;
                let pot = |s: &mut Props, w: f64, h: f64| {
                    s.tinted(&format!("pot{level}"), w, h, move |i, j, _w, h| {
                        let v = j as f64 / (h * TS);
                        if v < 0.15 {
                            return c3([30.0, 28.0, 34.0]);
                        }
                        if 1.0 - v < level * 0.8 {
                            return c3(if 1.0 - v > level * 0.8 - 0.06 { [176.0, 112.0, 60.0] } else { [92.0, 52.0, 30.0] });
                        }
                        c3(if i < 2 { [214.0, 228.0, 238.0] } else { [178.0, 196.0, 212.0] })
                    })
                };
                let bl = self.tinted_blk(x + 11.0, y - 7.0, x + 17.0, y - 3.0, 11.0, 17.5, &pot);
                out.boxes.push(bl);
                b!(x + 17.6, y - 4.0, x + 19.2, y - 2.4, 11.0, 12.6, white);
                out.solid.push([x, y - 9.0, x + 20.0, y]);
            }
            "cooler" => {
                // the water cooler: a white base with its taps, the bottle (the week's limit)
                let front = self.face("b:cooler", |s| {
                    let t = s.plain_tex(10.0, 14.0, "#dfe5ea", 0.04);
                    s.over(t, |r| {
                        r.r(2.0, 3.0, 6.0, 4.0, [200.0, 210.0, 218.0], false);
                        r.r(3.0, 4.2, 1.2, 1.2, [232.0, 80.0, 70.0], true);
                        r.r(5.8, 4.2, 1.2, 1.2, [80.0, 140.0, 232.0], true);
                        r.r(2.6, 7.6, 4.8, 0.8, [90.0, 96.0, 110.0], false);
                        r.r(1.0, 12.0, 8.0, 1.0, [150.0, 160.0, 170.0], false);
                    })
                });
                b!(x - 5.0, y - 8.0, x + 5.0, y, 0.0, 14.0, Mat::Plain("#dfe5ea", 0.04), Over { front: Some(front), ..Default::default() });
                let level = jround(p.level.unwrap_or(0.0) * 20.0) / 20.0;
                let bottle = |s: &mut Props, w: f64, h: f64| {
                    s.tinted(&format!("bottle{level}"), w, h, move |i, j, w, h| {
                        let v = j as f64 / (h * TS);
                        let edge = i < 2 || i as f64 >= w * TS - 2.0;
                        if v < 0.08 {
                            return c3([60.0, 120.0, 190.0]);
                        }
                        if 1.0 - v < level * 0.9 {
                            return c3(if edge { [90.0, 160.0, 220.0] } else { [74.0, 118.0, 184.0] });
                        }
                        c3(if edge { [222.0, 236.0, 246.0] } else { [196.0, 218.0, 234.0] })
                    })
                };
                let bl = self.tinted_blk(x - 4.0, y - 7.0, x + 4.0, y - 1.0, 14.0, 24.0, &bottle);
                out.boxes.push(bl);
                out.solid.push([x - 5.0, y - 8.0, x + 5.0, y]);
            }
            "printer" => {
                // on its stand: a grey printer, its slot, a lamp; a page coming out while it prints
                let stand = self.face("b:stand", |s| {
                    let t = s.plain_tex(14.0, 7.0, "#3f4452", 0.05);
                    s.over(t, |r| r.r(1.0, 3.0, 12.0, 0.6, [30.0, 32.0, 40.0], false))
                });
                b!(x - 7.0, y - 9.0, x + 7.0, y, 0.0, 7.0, dark_steel, Over { front: Some(stand), ..Default::default() });
                let front = self.face("b:printer", |s| {
                    let t = s.plain_tex(12.0, 5.0, "#cfd4dc", 0.04);
                    s.over(t, |r| {
                        r.r(1.5, 1.6, 9.0, 0.8, [40.0, 42.0, 50.0], false);
                        r.r(9.6, 3.2, 0.9, 0.9, [96.0, 220.0, 120.0], true);
                        r.r(1.4, 3.3, 3.0, 0.6, [150.0, 156.0, 168.0], false);
                    })
                });
                b!(x - 6.0, y - 8.0, x + 6.0, y - 1.0, 7.0, 12.0, Mat::Plain("#cfd4dc", 0.04), Over { front: Some(front), ..Default::default() });
                if let Some(pr) = p.printing {
                    let o = (pr * 2.0).min(4.0);
                    b!(x - 3.5, y - 1.0, x + 3.5, y - 1.0 + o + 0.2, 9.4, 9.8, white);
                }
                out.solid.push([x - 7.0, y - 9.0, x + 7.0, y]);
            }
            "manager" => {
                // her reception: a panel behind her on two posts, the counter with her name, her screen, a bell
                let color = p.color.clone().unwrap_or_else(|| "#9a74d6".into());
                let col = hex(&color);
                let panel = self.face(&format!("b:panel{color}"), |s| {
                    let t = s.plain_tex(38.0, 22.0, "#2b2433", 0.05);
                    s.over(t, |r| {
                        for dx in [9.0, 19.0, 29.0] {
                            r.r(dx, 0.0, 0.4, 22.0, [34.0, 28.0, 42.0], false);
                        }
                        r.r(0.0, 0.0, 38.0, 1.0, mul(col, 0.5), false);
                    })
                });
                b!(x + 4.0, y - 16.0, x + 42.0, y - 14.0, 0.0, 22.0, Mat::Plain("#2b2433", 0.05), Over { front: Some(panel), ..Default::default() });
                for px in [x + 2.0, x + 42.0] {
                    b!(px, y - 16.5, px + 2.0, y - 13.5, 0.0, 23.5, wood_d);
                }
                let name = p.name.to_uppercase();
                let front = self.face(&format!("b:counter{name}{color}"), |s| {
                    let t = s.wood_tex(46.0, 12.0, WOODP_TOP, 4.0);
                    s.over(t, |r| {
                        if name.is_empty() {
                            return;
                        }
                        let tw = font::tiny_w(&name) as f64;
                        let pw = tw * 0.5 + 2.0;
                        r.r(23.0 - pw / 2.0, 4.0, pw, 4.2, [40.0, 30.0, 36.0], false);
                        r.r(23.0 - pw / 2.0 + 0.4, 4.4, pw - 0.8, 3.4, mul(col, 0.26), false);
                        r.t(&name, 23.0 - pw / 2.0 + 1.0, 4.9, mix(col, [255.0; 3], 0.35), 2.0);
                    })
                });
                b!(x, y - 5.0, x + 46.0, y, 0.0, 12.0, wood, Over { front: Some(front), ..Default::default() });
                b!(x - 1.0, y - 6.0, x + 47.0, y + 0.6, 12.0, 13.0, wood_hi);
                if let Some(scr) = &p.screen {
                    let st = if scr == "asks" { "waits" } else { scr.as_str() };
                    let f = self.screen_tex(12.0, 8.2, st, &color, p.t_now);
                    b!(x + 3.0, y - 4.4, x + 15.0, y - 3.4, 13.8, 22.0, dark_steel, Over { front: Some(f), ..Default::default() });
                    b!(x + 8.0, y - 4.6, x + 10.0, y - 3.2, 13.0, 13.8, dark_steel);
                } else if p.notes > 0 {
                    b!(x + 5.0, y - 4.5, x + 13.0, y - 1.0, 13.0, 13.0 + p.notes.min(6) as f64 * 0.7, white);
                }
                b!(x + 39.0, y - 4.0, x + 43.0, y - 1.0, 13.0, 13.6, Mat::Plain("#c9a43c", 0.04));
                b!(x + 40.5, y - 3.0, x + 41.5, y - 2.0, 13.6, 14.6, Mat::Plain("#e6c65a", 0.04));
                out.solid.push([x, y - 16.5, x + 46.0, y]);
            }
            "add" => {
                // the project cabinet: a low cabinet, a + on its front
                let w = p.w.unwrap_or(30.0);
                let front = self.face(&format!("b:add{w}"), |s| {
                    let t = s.wood_tex(w, 9.0, WOODP_TOP, 4.0);
                    s.over(t, |r| {
                        r.r(w / 2.0 - 6.0, 2.0, 12.0, 5.0, [46.0, 31.0, 40.0], false);
                        r.r(w / 2.0 - 0.5, 3.0, 1.0, 3.0, [194.0, 201.0, 217.0], false);
                        r.r(w / 2.0 - 1.5, 4.0, 3.0, 1.0, [194.0, 201.0, 217.0], false);
                    })
                });
                b!(x, y - 8.0, x + w, y, 0.0, 9.0, wood, Over { front: Some(front), ..Default::default() });
                b!(x - 0.4, y - 8.4, x + w + 0.4, y + 0.4, 9.0, 10.0, wood_hi);
                out.solid.push([x, y - 8.0, x + w, y]);
            }
            "booth" if p.gun => {
                // hardcore: the gun closet, its glass doors, the shotguns in it (one gone a gun out), a red lamp
                let out_n = p.out.min(2);
                let front = self.face(&format!("b:guns{out_n}"), |s| {
                    let t = s.plain_tex(18.0, 30.0, "#4f5844", 0.06);
                    s.over(t, |r| {
                        r.r(1.4, 2.0, 15.2, 25.0, [26.0, 30.0, 22.0], false);
                        for (k, gx) in [(0, 4.6), (1, 12.2)] {
                            if k < 2 - out_n {
                                r.r(gx, 3.0, 0.8, 15.0, [167.0, 174.0, 187.0], false);
                                r.r(gx + 0.8, 3.0, 0.6, 15.0, [108.0, 115.0, 128.0], false);
                                r.r(gx - 0.8, 17.0, 2.4, 8.0, [122.0, 74.0, 42.0], false);
                            }
                        }
                        r.r(8.6, 2.0, 0.8, 25.0, [63.0, 55.0, 64.0], false);
                        r.r(7.6, 13.0, 0.6, 3.0, [200.0, 200.0, 210.0], false);
                        r.r(9.8, 13.0, 0.6, 3.0, [200.0, 200.0, 210.0], false);
                    })
                });
                b!(x - 9.0, y - 8.0, x + 9.0, y, 0.0, 30.0, Mat::Plain("#4f5844", 0.06), Over { front: Some(front), ..Default::default() });
                let lamp = self.face("b:lamp", |s| {
                    let t = s.plain_tex(6.0, 2.0, "#26242c", 0.05);
                    s.over(t, |r| r.r(1.0, 0.4, 4.0, 1.2, [235.0, 76.0, 63.0], true))
                });
                b!(x - 3.0, y - 6.0, x + 3.0, y - 2.0, 30.0, 32.0, black, Over { front: Some(lamp), ..Default::default() });
                out.solid.push([x - 9.0, y - 8.0, x + 9.0, y]);
            }
            "booth" => {
                // the booth: a glass cabin on a plinth, its sign on the roof (red while in use); in use its back shines more and more
                let t = p.t.unwrap_or(-1.0);
                let glow = if t >= 0.0 { jround(((t + 0.3) / 4.0).min(1.0) * 20.0) / 20.0 } else { 0.0 };
                b!(x - 9.0, y - 9.0, x + 9.0, y, 0.0, 2.0, steel);
                let cabin = move |_i: i32, _j: i32, _w: f64, _h: f64| if glow != 0.0 { g3(mix([59.0, 52.0, 40.0], [255.0, 244.0, 214.0], glow)) } else { c3([34.0, 42.0, 55.0]) };
                let back16 = self.tinted(&format!("cabin{glow}"), 16.0, 28.0, cabin);
                let bl = self.tinted_blk_o(x - 8.0, y - 9.0, x + 8.0, y - 8.0, 2.0, 30.0, &format!("cabin{glow}"), &cabin, Over { front: Some(back16), ..Default::default() });
                out.boxes.push(bl);
                for px in [x - 9.0, x + 8.0] {
                    for py in [y - 9.0, y - 1.0] {
                        b!(px, py, px + 1.0, py + 1.0, 2.0, 30.0, steel);
                    }
                }
                let glass = |i: i32, j: i32, w: f64, h: f64| {
                    if (i + j) % 23 == 0 && (j as f64) < h * TS * 0.6 {
                        c3([255.0; 3])
                    } else if i < 2 || j < 2 || i as f64 >= w * TS - 2.0 {
                        c3([117.0, 123.0, 124.0])
                    } else {
                        None
                    }
                };
                let bl = self.tinted_blk_o(x - 9.0, y - 8.0, x - 8.8, y - 1.0, 2.0, 30.0, "glass", &glass, Over::default());
                out.boxes.push(bl);
                let bl = self.tinted_blk_o(x + 8.8, y - 8.0, x + 9.0, y - 1.0, 2.0, 30.0, "glass", &glass, Over::default());
                out.boxes.push(bl);
                let shut = if t < 0.0 { 1.0 } else { (t / 0.4).min(1.0) };
                if shut > 0.05 {
                    let bl = self.tinted_blk_o(x + 8.0 - 16.0 * shut, y - 0.4, x + 8.0, y, 2.0, 30.0, "glass", &glass, Over::default());
                    out.boxes.push(bl);
                }
                let on = t >= 0.0;
                let sign = self.face(&format!("b:sign{on}19x3"), |s| {
                    let b = s.plain_tex(19.0, 3.0, "#3f3740", 0.05);
                    s.over(b, |r| r.r(3.0, 0.8, 13.0, 1.4, if on { [235.0, 76.0, 63.0] } else { [255.0, 226.0, 122.0] }, true))
                });
                b!(x - 9.5, y - 9.5, x + 9.5, y + 0.5, 30.0, 33.0, steel, Over { front: Some(sign), ..Default::default() });
                if glow != 0.0 {
                    out.lights.push(Light { x, y: y - 5.0, z: 16.0, r: 50.0, col: [1.0, 0.95, 0.8], k: 0.9 * glow, flicker: false, muzzle: false });
                }
                out.solid.push([x - 9.0, y - 9.0, x + 9.0, y]);
            }
            "neuralyzer" => {
                // a black pedestal, the silver flasher on it, its bulb
                b!(x - 4.0, y - 4.0, x + 4.0, y, 0.0, 12.0, black);
                b!(x - 6.0, y - 5.0, x + 6.0, y + 1.0, 12.0, 14.0, dark_steel);
                let flash = p.flash;
                let front = self.face(&format!("b:flasher{flash}"), |s| {
                    let t = s.plain_tex(10.0, 4.0, "#b5bfc7", 0.04);
                    s.over(t, |r| {
                        r.r(0.4, 1.2, 1.4, 1.6, if flash { [255.0, 255.0, 240.0] } else { [255.0, 226.0, 122.0] }, true);
                        r.r(7.0, 1.4, 1.0, 1.0, [235.0, 76.0, 63.0], true);
                    })
                });
                b!(x - 5.0, y - 4.0, x + 5.0, y - 1.0, 14.0, 18.0, Mat::Plain("#b5bfc7", 0.04), Over { front: Some(front), ..Default::default() });
                if flash {
                    out.lights.push(Light { x, y: y - 3.0, z: 16.0, r: 160.0, col: [1.0, 1.0, 1.0], k: 2.2, flicker: false, muzzle: false });
                }
                out.solid.push([x - 4.0, y - 4.0, x + 4.0, y]);
            }
            "doghouse" => {
                // plank walls, an arched dark door; the roof in red steps; its bowl
                let front = self.face("b:kennel", |s| {
                    let t = s.wood_tex(18.0, 12.0, WOODP_TOP, 3.0);
                    s.over(t, |r| {
                        r.r(5.0, 4.0, 8.0, 8.0, [30.0, 21.0, 18.0], false);
                        r.r(6.0, 3.0, 6.0, 1.0, [30.0, 21.0, 18.0], false);
                    })
                });
                b!(x - 9.0, y - 12.0, x + 9.0, y, 0.0, 12.0, Mat::Wood(WOODP_TOP, 3.0), Over { front: Some(front), ..Default::default() });
                let red = Mat::Plain("#a8433a", 0.08);
                b!(x - 10.5, y - 13.0, x + 10.5, y + 1.0, 12.0, 14.0, red);
                b!(x - 7.5, y - 13.0, x + 7.5, y + 1.0, 14.0, 16.4, Mat::Plain("#c9574b", 0.08));
                b!(x - 4.5, y - 13.0, x + 4.5, y + 1.0, 16.4, 18.4, red);
                b!(x - 1.5, y - 13.0, x + 1.5, y + 1.0, 18.4, 19.6, Mat::Plain("#7a2a26", 0.08));
                b!(x - 12.5, y - 3.0, x - 8.5, y, 0.0, 1.4, steel);
                out.solid.push([x - 9.0, y - 12.0, x + 9.0, y]);
            }
            "radio" => {
                // a midcentury set on four legs: the dial across its top (lit while the sound is on), the speaker's
                // cloth behind three slats, two knobs
                let on = p.on;
                let front = self.face(&format!("b:radio{on}"), |s| {
                    let t = s.wood_tex(14.0, 15.0, WOODP_TOP, 15.0);
                    s.over(t, |r| {
                        r.r(1.0, 1.5, 12.0, 3.0, if on { [255.0, 226.0, 122.0] } else { [58.0, 44.0, 38.0] }, on);
                        r.r(7.6, 1.5, 0.6, 3.0, if on { [235.0, 76.0, 63.0] } else { [42.0, 32.0, 38.0] }, on);
                        r.r(1.0, 5.5, 12.0, 6.0, [216.0, 196.0, 154.0], false);
                        for sx in [3.8, 6.6, 9.4] {
                            r.r(sx, 5.5, 0.8, 6.0, wd, false);
                        }
                        r.r(1.2, 12.3, 2.0, 2.0, [63.0, 55.0, 64.0], false);
                        r.r(10.8, 12.3, 2.0, 2.0, [63.0, 55.0, 64.0], false);
                    })
                });
                b!(x - 7.0, y - 5.0, x + 7.0, y, 4.0, 19.0, Mat::Wood(WOODP_TOP, 15.0), Over { front: Some(front), ..Default::default() });
                for (lx, ly) in [(x - 6.0, y - 5.0), (x + 5.0, y - 5.0), (x - 6.0, y - 1.0), (x + 5.0, y - 1.0)] {
                    b!(lx, ly, lx + 1.0, ly + 1.0, 0.0, 4.0, Mat::Wood(WOODP_D, 4.0));
                }
                if on {
                    out.lights.push(Light { x, y: y - 3.0, z: 15.0, r: 60.0, col: [1.0, 0.85, 0.5], k: 0.8, flicker: false, muzzle: false });
                }
                out.solid.push([x - 7.0, y - 5.0, x + 7.0, y]);
            }
            "chess" => {
                // a table with the game on it as it stands (white plays from the west), a stool each side
                let board: Vec<char> = p.board.chars().collect();
                let top = self.face(&format!("b:chess{}", p.board), |s| {
                    s.pix(
                        16.0 * TS,
                        14.0 * TS,
                        |i, j| {
                            let (bx, by, cell) = (i as f64 - 2.0 * TS, j as f64 - TS, 1.5 * TS);
                            if bx < 0.0 || by < 0.0 || bx >= 8.0 * cell || by >= 8.0 * cell {
                                return c3(mul(hex(WOODP_TOP), 0.95 + (hashi(i >> 1, j, 0) - 0.5) * 0.08));
                            }
                            let r = (bx / cell).floor() as i32;
                            let f = 7 - (by / cell).floor() as i32;
                            let pc = board.get((r * 8 + f) as usize).copied();
                            let (cx, cy) = (bx % cell - cell / 2.0, by % cell - cell / 2.0);
                            if let Some(pc) = pc
                                && pc != '.' && cx * cx + cy * cy < (cell * 0.38).powi(2) {
                                    let wht = pc.is_uppercase() || !pc.is_alphabetic();
                                    return c3(mul(if wht { [240.0, 236.0, 226.0] } else { [40.0, 34.0, 44.0] }, if cx + cy < -1.0 { 1.25 } else { 1.0 }));
                                }
                            c3(if (r + f) % 2 != 0 { [232.0, 210.0, 160.0] } else { [168.0, 120.0, 76.0] })
                        },
                        1.0,
                    )
                });
                b!(x - 8.0, y - 15.0, x + 8.0, y - 1.0, 9.0, 10.0, wood, Over { top: Some(top), ..Default::default() });
                b!(x - 1.0, y - 9.0, x + 1.0, y - 7.0, 0.0, 9.0, wood_d);
                b!(x - 4.0, y - 10.0, x + 4.0, y - 6.0, 0.0, 1.0, wood_d);
                for &(sx, sy) in &p.seats {
                    b!(sx - 3.0, sy - 3.0, sx + 3.0, sy, 0.0, 6.0, wood);
                }
                out.solid.push([x - 8.0, y - 15.0, x + 8.0, y - 1.0]);
            }
            "meet" => {
                // the meeting table: papers and a mug on it, four legs; two stools
                let top = self.face("b:meet", |s| {
                    let t = s.wood_tex(32.0, 11.0, WOODP_TOP, 11.0);
                    s.over(t, |r| {
                        r.r(5.0, 2.0, 7.0, 5.0, [232.0, 236.0, 240.0], false);
                        r.r(6.0, 3.0, 5.0, 0.5, [150.0, 156.0, 168.0], false);
                        r.r(6.0, 4.5, 4.0, 0.5, [150.0, 156.0, 168.0], false);
                        r.r(14.0, 3.0, 6.0, 4.0, [240.0, 240.0, 236.0], false);
                        r.r(24.0, 4.0, 2.4, 2.4, [235.0, 76.0, 63.0], false);
                    })
                });
                b!(x - 16.0, y - 14.0, x + 16.0, y - 3.0, 9.0, 10.5, wood, Over { top: Some(top), ..Default::default() });
                for lx in [x - 15.0, x + 13.6] {
                    for ly in [y - 13.0, y - 4.4] {
                        b!(lx, ly, lx + 1.4, ly + 1.4, 0.0, 9.0, wood_d);
                    }
                }
                for &(sx, sy) in &p.seats {
                    b!(sx - 3.0, sy - 3.0, sx + 3.0, sy, 0.0, 6.0, wood);
                }
                out.solid.push([x - 16.0, y - 14.0, x + 16.0, y - 3.0]);
            }
            "pong" => {
                // the ping-pong table: green, its white lines, the net across, on legs
                let top = self.face("b:pong", |s| {
                    s.pix(
                        32.0 * TS,
                        13.0 * TS,
                        |i, j| {
                            let (w2, h2) = (32.0 * TS, 13.0 * TS);
                            let (fi, fj) = (i as f64, j as f64);
                            if i < 2 || j < 2 || fi >= w2 - 2.0 || fj >= h2 - 2.0 || (fj - h2 / 2.0).abs() < 1.0 {
                                return c3([244.0, 246.0, 251.0]);
                            }
                            c3(mul([38.0, 128.0, 92.0], 0.96 + (hashi(i >> 2, j >> 2, 0) - 0.5) * 0.06))
                        },
                        1.0,
                    )
                });
                b!(x - 16.0, y - 18.0, x + 16.0, y - 5.0, 7.0, 8.0, Mat::Plain("#2a5c48", 0.04), Over { top: Some(top), ..Default::default() });
                let net = self.face("b:net", |s| s.pix(13.0 * TS, 2.0 * TS, |i, j| if j < 2 { c3([244.0, 246.0, 251.0]) } else if (i + j) % 2 != 0 { c3([220.0, 224.0, 232.0]) } else { None }, 1.0));
                let bl = self.blk_tex(x - 0.3, y - 18.0, x + 0.3, y - 5.0, 8.0, 10.0, net);
                out.boxes.push(bl);
                for lx in [x - 14.0, x + 13.0] {
                    for ly in [y - 16.0, y - 7.0] {
                        b!(lx, ly, lx + 1.0, ly + 1.0, 0.0, 7.0, dark_steel);
                    }
                }
                out.solid.push([x - 16.0, y - 18.0, x + 16.0, y - 5.0]);
            }
            "board" => {
                // the easel: legs splayed in steps, one behind, a ledge, the painting on it (a third of its dots a dot)
                let (pw, ph) = p.art.as_ref().map_or((16.0, 20.0), |a| (a.img.w as f64 / 3.0, a.img.h as f64 / 3.0));
                for s in [-1.0, 1.0] {
                    for k in 0..4 {
                        let (lx, k) = (x + s * (8.5 - k as f64 * 1.2), k as f64);
                        b!(lx - 0.5, y - 1.2, lx + 0.5, y - 0.2, k * 2.75, (k + 1.0) * 2.75, wood);
                    }
                }
                b!(x - 0.5, y - 6.0, x + 0.5, y - 5.0, 0.0, 11.0, wood_d);
                b!(x - 0.5, y - 4.0, x + 0.5, y - 3.0, 11.0, 14.0 + ph, wood_d); // the mast, behind the canvas
                b!(x - pw / 2.0 - 2.0, y - 3.5, x + pw / 2.0 + 2.0, y - 0.8, 10.0, 11.0, wood_hi);
                if let Some(art) = &p.art {
                    let c = match &self.board {
                        Some((v, t)) if *v == p.art_v => t.clone(),
                        _ => {
                            let mut c = Image::new(art.img.w + 2, art.img.h + 2);
                            {
                                let mut g = c.ctx();
                                g.fill_rect(0.0, 0.0, art.img.w as f64 + 2.0, art.img.h as f64 + 2.0, pix::css("#3f3740"));
                                g.draw_image(&art.img, 1.0, 1.0);
                            }
                            let t = Tex::new(c);
                            self.board = Some((p.art_v, t.clone()));
                            t
                        }
                    };
                    b!(x - pw / 2.0, y - 2.6, x + pw / 2.0, y - 2.0, 11.0, 11.0 + ph, Mat::Plain("#e8e2d4", 0.03), Over { front: Some(Face::whole(c)), ..Default::default() });
                }
                b!(x - 1.5, y - 3.6, x + 1.5, y - 1.8, 11.0 + ph, 12.5 + ph, wood_d); // the clamp over its top edge
                out.solid.push([x - 9.0, y - 6.0, x + 9.0, y]);
            }
            "armchair" => {
                // under whoever sits on it (at y - 4), as high as a seated robot's thighs (its shins hang over the
                // edge), on four wooden feet
                let sy = y - 4.0;
                let top = self.pouf_top();
                b!(x - 3.5, sy - 3.5, x + 3.5, sy + 3.5, 1.0, 3.4, Mat::Plain("#487841", 0.06), Over { top: Some(top), ..Default::default() });
                for (fx, fy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    b!(x + fx * 2.8 - 0.4, sy + fy * 2.8 - 0.4, x + fx * 2.8 + 0.4, sy + fy * 2.8 + 0.4, 0.0, 1.0, Mat::Wood("#885c47", 4.0));
                }
                out.solid.push([x - 3.5, sy - 3.5, x + 3.5, sy + 3.5]);
            }
            "plant" => {
                let f = self.plant(p.variant);
                pic(&mut out, f, x, y - 3.0, 0.0);
                out.solid.push([x - 6.0, y - 7.0, x + 6.0, y]);
            }
            "pair" => {
                let f = self.pair_table(p.linked);
                pic(&mut out, f, x, y - 4.0, 0.0);
                out.solid.push([x - 12.0, y - 8.0, x + 12.0, y]);
            }
            _ => return None,
        }
        Some(out)
    }
    /// a block of a tinted material on every side (its faces painted by state)
    fn tinted_blk(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, z0: f64, z1: f64, m: &dyn Fn(&mut Props, f64, f64) -> Face) -> Block {
        let (w, d, h) = ((x1 - x0).max(0.25), (y1 - y0).max(0.25), (z1 - z0).max(0.25));
        let r = |v: f64| (jround(v * 2.0) / 2.0).max(1.0);
        let (front, back, side, top) = (m(self, r(w), r(h)), m(self, r(w), r(h)), m(self, r(d), r(h)), m(self, r(w), r(d)));
        Block { x0, y0, x1, y1, z0, z1, front: Some(front), back: Some(back), side: Some(side), top: Some(top), ..Default::default() }
    }
    #[allow(clippy::too_many_arguments)]
    fn tinted_blk_o(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, z0: f64, z1: f64, k: &str, f: &(impl Fn(i32, i32, f64, f64) -> Px + Copy), o: Over) -> Block {
        let (w, d, h) = ((x1 - x0).max(0.25), (y1 - y0).max(0.25), (z1 - z0).max(0.25));
        let r = |v: f64| (jround(v * 2.0) / 2.0).max(1.0);
        let front = o.front.unwrap_or_else(|| self.tinted(k, r(w), r(h), *f));
        let back = o.back.unwrap_or_else(|| self.tinted(k, r(w), r(h), *f));
        let side = o.side.unwrap_or_else(|| self.tinted(k, r(d), r(h), *f));
        let top = o.top.unwrap_or_else(|| self.tinted(k, r(w), r(d), *f));
        Block { x0, y0, x1, y1, z0, z1, front: Some(front), back: Some(back), side: Some(side), top: Some(top), ..Default::default() }
    }

    /// a project's desk: its cabinet (the name on its plate), the desktop on legs with a keyboard at each seat and
    /// the mugs brought, the screen on the cabinet (laptops for the worktrees beside it), a chair a bot
    pub fn desk(&mut self, d: &Desk) -> Parts {
        let mut out = Parts::default();
        let (x0, y, w, cab, pt) = (d.x0, d.y, d.w, d.cab, d.paint);
        let (top_hex, d_hex, hi_hex) = (pt.top.hex(), pt.d.hex(), pt.hi.hex());
        let wd = Mat::Wood(&top_hex, 6.0);
        let wdk = Mat::Wood(&d_hex, 6.0);
        let steel = Mat::Plain("#8b93ad", 0.06);
        let dark_steel = Mat::Plain("#3f4452", 0.08);
        let black = Mat::Plain("#26242c", 0.08);
        let label = d.label.clone();
        let front = self.face(&format!("b:desk{label}{cab}{top_hex}"), |s| {
            let t = s.wood_tex(cab, 11.0, &top_hex, 11.0);
            s.over(t, |r| {
                let tw = font::tiny_w(&label) as f64;
                let pw = (cab - 3.0).min(tw * 0.5 + 2.4);
                r.r((cab - pw) / 2.0, 2.2, pw, 4.4, rgb_of(pt.k), false);
                r.r((cab - pw) / 2.0 + 0.4, 2.6, pw - 0.8, 3.6, rgb_of(pt.plate), false);
                r.t(&label, (cab - tw * 0.5) / 2.0, 3.2, [194.0, 201.0, 217.0], 2.0);
                r.r(1.5, 8.2, cab - 3.0, 0.4, mul(rgb_of(pt.top), 0.7), false);
            })
        });
        let bl = self.blk(x0, y - 12.0, x0 + cab, y, 0.0, 11.0, wd, Over { front: Some(front), ..Default::default() });
        out.boxes.push(bl);
        let bl = self.blk(x0 - 0.4, y - 12.4, x0 + cab + 0.4, y + 0.4, 11.0, 12.0, Mat::Wood(&hi_hex, 6.0), Over::default());
        out.boxes.push(bl);
        if !d.seats.is_empty() {
            let tx = x0 + cab;
            let dw = x0 + w - tx;
            let seats_k: Vec<String> = d.seats.iter().map(|s| format!("{}", s - tx)).collect();
            let mugs_k: Vec<String> = d.mugs.iter().map(|(mx, c)| format!("{}{c}", mx - tx)).collect();
            let (seats, mugs) = (d.seats.clone(), d.mugs.clone());
            let top = self.face(&format!("b:desktop{top_hex}{dw}{}{}", seats_k.join(","), mugs_k.join(",")), |s| {
                let t = s.wood_tex(dw, 7.0, &top_hex, 7.0);
                s.over(t, |r| {
                    for sx in &seats {
                        r.r(sx - 9.0 - tx, 1.6, 7.0, 2.2, [63.0, 55.0, 64.0], false);
                        for k in 0..6 {
                            r.r(sx - 8.6 - tx + k as f64 * 1.1, 2.0, 0.6, 0.5, [181.0, 191.0, 199.0], false);
                        }
                    }
                    for (mx, c) in &mugs {
                        r.r(mx - 13.0 - tx, 0.6, 2.0, 2.0, hex(c), false);
                    }
                })
            });
            let bl = self.blk(tx, y - 11.0, x0 + w, y - 4.0, 7.0, 9.0, wd, Over { top: Some(top), ..Default::default() });
            out.boxes.push(bl);
            let mut legs = vec![tx + 1.0];
            legs.extend(d.seats.iter().map(|s| s + 11.0));
            legs.push(x0 + w - 2.0);
            for lx in legs {
                for ly in [y - 10.5, y - 5.5] {
                    let bl = self.blk(lx, ly, lx + 1.2, ly + 1.0, 0.0, 7.0, wdk, Over::default());
                    out.boxes.push(bl);
                }
            }
            for &cx in &d.seats {
                // the office chair, its back behind its sitter (east)
                let cy = y - 1.6;
                let seat = Mat::Plain("#3f3740", 0.08);
                for (a, b2, c2, d2, e, f, m) in [
                    (cx - 3.0, cy - 2.4, cx + 3.0, cy + 2.4, 4.4, 5.6, seat),
                    (cx + 2.6, cy - 2.4, cx + 3.6, cy + 2.4, 5.6, 13.0, seat),
                    (cx - 0.4, cy - 0.4, cx + 0.4, cy + 0.4, 1.0, 4.4, steel),
                    (cx - 2.6, cy - 0.5, cx + 2.6, cy + 0.5, 0.0, 1.0, black),
                    (cx - 0.5, cy - 2.6, cx + 0.5, cy + 2.6, 0.0, 1.0, black),
                ] {
                    let bl = self.blk(a, b2, c2, d2, e, f, m, Over::default());
                    out.boxes.push(bl);
                }
            }
        }
        let cx = x0 + cab / 2.0;
        let scr = self.screen_tex(18.0, 11.0, if d.screen.is_empty() { "off" } else { &d.screen }, &d.color, d.t_now);
        let bl = self.blk(cx - 9.0, y - 7.6, cx + 9.0, y - 6.6, 14.0, 25.0, Mat::Plain("#3f3740", 0.06), Over { front: Some(scr), ..Default::default() });
        out.boxes.push(bl);
        let bl = self.blk(cx - 1.0, y - 8.0, cx + 1.0, y - 7.0, 12.0, 14.0, steel, Over::default());
        out.boxes.push(bl);
        let bl = self.blk(cx - 3.5, y - 9.5, cx + 3.5, y - 5.5, 12.0, 12.6, dark_steel, Over::default());
        out.boxes.push(bl);
        for (k, st) in d.laptops.iter().enumerate() {
            let lx = cx + 10.0 + k as f64 * 9.0;
            let bl = self.blk(lx, y - 8.6, lx + 8.0, y - 4.0, 12.0, 12.5, dark_steel, Over::default());
            out.boxes.push(bl);
            let f = self.screen_tex(8.0, 5.5, st, &d.color, d.t_now);
            let bl = self.blk(lx, y - 9.0, lx + 8.0, y - 8.6, 12.5, 18.0, Mat::Plain("#757b7c", 0.05), Over { front: Some(f), ..Default::default() });
            out.boxes.push(bl);
        }
        if matches!(d.screen.as_str(), "busy" | "waits" | "on") {
            let c = hex(&d.color).map(|v| v / 255.0);
            out.lights.push(Light { x: cx, y: y - 4.0, z: 19.0, r: 34.0, col: mix([0.5, 0.6, 1.0], c, 0.5), k: if d.screen == "busy" { 0.5 } else { 0.25 }, flicker: false, muzzle: false });
        }
        out.solid.push([x0, y - 12.0, x0 + w, y - 3.0]);
        out
    }

    // ---- the walls: wainscot panels under a rail, the plaster over it, a cornice; on the room's own wall the doors
    // (in their paint), the window (outside by the hour), the clock (telling the time), the closet and the
    // switchboard. wh dots high; `items` in the room's wall band (y0..y1 of `band` dots, the floor at band)
    fn wall_base(&mut self, w: f64, wh: f64) -> Image {
        let (navy, pan, rail) = (hex("#28374b"), hex("#1f2b3c"), hex(WOODP_D));
        self.pix(
            w * TS,
            wh * TS,
            |i, j| {
                let z = wh - j as f64 / TS;
                let n = (hashi(i >> 1, j >> 1, 4) - 0.5) * 0.05;
                if z < 1.0 {
                    return c3(mul([18.0, 22.0, 30.0], 1.0 + n)); // the skirting
                }
                if z < 13.0 {
                    // the panels
                    let px = (i as f64 / TS) % 16.0;
                    let in_panel = px > 1.2 && px < 14.8 && z > 2.0 && z < 11.6;
                    if !in_panel {
                        return c3(mul(pan, 1.05 + n));
                    }
                    let e = (px - 1.2).min(14.8 - px).min(z - 2.0).min(11.6 - z);
                    return c3(mul(pan, if e < 0.5 { if px - 1.2 < 0.5 || 11.6 - z < 0.5 { 0.78 } else { 1.25 } } else { 0.92 } + n));
                }
                if z < 14.6 {
                    return c3(mul(rail, if z > 14.1 { 1.2 } else if z < 13.4 { 0.75 } else { 1.0 } + n)); // the rail
                }
                if z > wh - 1.4 {
                    return c3(mul([52.0, 66.0, 88.0], if z > wh - 0.5 { 0.7 } else { 1.0 } + n)); // the cornice
                }
                let stripe = if ((i as f64 / TS / 4.0).floor() as i64) % 2 != 0 { 1.03 } else { 0.97 };
                c3(mul(navy, stripe + n))
            },
            1.0,
        )
    }
    /// the far wall's picture, w dots wide, wh high; kept while `key` (room.js `wkey`) stays the same
    pub fn wall(&mut self, w: f64, wh: f64, band: f64, items: &[WallItem], key: &str) -> Tex {
        let k = format!("{w}|{key}");
        if let Some((wk, t)) = &self.wall
            && *wk == k {
                return t.clone();
            }
        let c = self.wall_base(w, wh);
        let z_of = |yb: f64| (band - yb) * (wh / band);
        let c = self.over(c, |r| {
            for it in items {
                let (x0, x1, zt, zb) = (it.x0, it.x1, z_of(it.y0), z_of(it.y1).max(0.0));
                let (wd, ht, y) = (x1 - x0, zt - zb, wh - zt);
                match &it.kind {
                    WallKind::Door { paint: p, ok } => {
                        // its frame, the door in its paint, two sunk panels, a knob, a plate
                        let (k, tp, hi, dd) = (rgb_of(p.k), rgb_of(p.top), rgb_of(p.hi), rgb_of(p.d));
                        r.r(x0 - 0.8, y - 0.8, wd + 1.6, ht + 0.8, k, false);
                        r.r(x0, y, wd, ht, tp, false);
                        for (py, ph) in [(2.0, ht * 0.38), (ht * 0.48, ht * 0.4)] {
                            r.r(x0 + 2.0, y + py, wd - 4.0, ph, dd, false);
                            r.r(x0 + 2.4, y + py + 0.4, wd - 4.8, ph - 0.8, tp, false);
                            r.r(x0 + 2.4, y + py + 0.4, wd - 4.8, 0.4, hi, false);
                        }
                        r.r(x0 + wd - 3.0, y + ht * 0.5, 1.2, 1.2, [214.0, 180.0, 90.0], false);
                        r.r(x0 + 1.0, y + ht - 2.4, wd - 2.0, 1.6, mul(dd, 0.9), false);
                        if !ok {
                            for py in [ht * 0.3, ht * 0.62] {
                                r.r(x0 - 1.4, y + py, wd + 2.8, 2.2, [94.0, 60.0, 36.0], false);
                                r.r(x0 - 1.4, y + py, wd + 2.8, 0.5, [140.0, 98.0, 60.0], false);
                            }
                        }
                    }
                    WallKind::Window { hour } => {
                        // its frame and sill, four panes, the sky by the hour
                        let h = *hour;
                        let day = (7..19).contains(&h);
                        let dusk = (18..21).contains(&h) || (6..8).contains(&h);
                        let sky0 = if day { [120.0, 176.0, 230.0] } else if dusk { [232.0, 150.0, 110.0] } else { [20.0, 28.0, 60.0] };
                        let sky1 = if day { [190.0, 222.0, 246.0] } else if dusk { [250.0, 200.0, 150.0] } else { [36.0, 44.0, 86.0] };
                        r.r(x0 - 0.6, y - 0.6, wd + 1.2, ht + 1.2, [216.0, 220.0, 228.0], false);
                        for k in 0..8 {
                            r.r(x0, y + k as f64 * ht / 8.0, wd, ht / 8.0 + 0.1, mix(sky0, sky1, k as f64 / 7.0), true);
                        }
                        if !day && !dusk {
                            for k in 0..9 {
                                r.r(x0 + 1.0 + hashi(k, 1, 0) * (wd - 2.0), y + 1.0 + hashi(k, 2, 0) * ht * 0.6, 0.4, 0.4, [240.0, 240.0, 220.0], true);
                            }
                        } else {
                            r.r(x0 + wd * 0.65, y + ht * 0.2, 2.4, 2.4, if day { [255.0, 240.0, 170.0] } else { [255.0, 200.0, 120.0] }, true);
                        }
                        r.r(x0 + wd / 2.0 - 0.4, y, 0.8, ht, [216.0, 220.0, 228.0], false);
                        r.r(x0, y + ht / 2.0 - 0.4, wd, 0.8, [216.0, 220.0, 228.0], false);
                        r.r(x0 - 1.2, y + ht, wd + 2.4, 1.2, [190.0, 194.0, 204.0], false);
                    }
                    WallKind::Clock { hours, minutes, awake } => {
                        // round, its hours, the hands at this computer's time
                        let (cx, cy, rr) = (x0 + wd / 2.0, y + ht / 2.0, wd.min(ht) / 2.0);
                        let x = &mut r.ctx;
                        x.fill_circle(cx * TS, cy * TS, rr * TS, pix::css("#3f3740"));
                        x.fill_circle(cx * TS, cy * TS, (rr - 0.3) * TS, pix::css("#c83c30"));
                        x.fill_circle(cx * TS, cy * TS, (rr - 1.4) * TS, pix::css("#f4f1ea"));
                        for hh in 0..12 {
                            let a = hh as f64 * PI / 6.0;
                            r.r(cx + a.sin() * (rr - 2.4) - 0.3, cy - a.cos() * (rr - 2.4) - 0.3, 0.6, 0.6, if hh % 3 != 0 { [150.0, 150.0, 160.0] } else { [40.0, 40.0, 50.0] }, false);
                        }
                        let hrs = (hours % 12) as f64 + *minutes as f64 / 60.0;
                        let x = &mut r.ctx;
                        hand(x, cx, cy, hrs * PI / 6.0, rr * 0.45, 0.8, pix::css("#1a1626"));
                        hand(x, cx, cy, *minutes as f64 * PI / 30.0, rr * 0.7, 0.55, pix::css("#1a1626"));
                        if *awake {
                            x.fill_circle(cx * TS, cy * TS, (rr - 1.4) * TS, pix::css("rgba(255,226,122,0.35)"));
                        }
                    }
                    WallKind::Closet => {
                        // a wardrobe: two doors, their handles
                        let (wk, wt, wdd) = (hex(WOODP_K), hex(WOODP_TOP), hex(WOODP_D));
                        r.r(x0 - 0.5, y - 0.5, wd + 1.0, ht + 0.5, wk, false);
                        r.r(x0, y, wd, ht, wt, false);
                        for dx in [0.8, wd / 2.0 + 0.4] {
                            r.r(x0 + dx, y + 1.5, wd / 2.0 - 1.2, ht - 2.5, wdd, false);
                            r.r(x0 + dx + 0.4, y + 1.9, wd / 2.0 - 2.0, ht - 3.3, wt, false);
                        }
                        r.r(x0 + wd / 2.0 - 1.4, y + ht * 0.45, 0.6, 2.0, [214.0, 198.0, 160.0], false);
                        r.r(x0 + wd / 2.0 + 0.8, y + ht * 0.45, 0.6, 2.0, [214.0, 198.0, 160.0], false);
                    }
                    WallKind::Settings => {
                        // the switchboard: a steel box, one big lever, its red knob
                        r.r(x0 - 0.4, y - 0.4, wd + 0.8, ht + 0.8, [40.0, 42.0, 50.0], false);
                        r.r(x0, y, wd, ht, [139.0, 147.0, 173.0], false);
                        r.r(x0 + 0.4, y + 0.4, wd - 0.8, 0.5, [194.0, 201.0, 217.0], false);
                        r.r(x0 + wd / 2.0 - 0.5, y + 2.0, 1.0, ht * 0.5, [60.0, 64.0, 74.0], false);
                        r.r(x0 + wd / 2.0 - 1.4, y + 1.0, 2.8, 2.4, [235.0, 76.0, 63.0], false);
                    }
                }
            }
        });
        let t = Tex::new(c);
        self.wall = Some((k, t.clone()));
        t
    }

    /// the fire's pit on the floor: a ring of stones, the embers inside (they shine); room.js stamps it into the
    /// floor's overlay
    pub fn pit(&mut self, w: f64, h: f64) -> Tex {
        self.memo(&format!("pit{w}x{h}"), |p| {
            Tex::new(p.pix(
                w * TS,
                h * TS,
                |i, j| {
                    let u = (i as f64 + 0.5) / (w * TS) * 2.0 - 1.0;
                    let v = (j as f64 + 0.5) / (h * TS) * 2.0 - 1.0;
                    let d = u.hypot(v);
                    if d > 1.0 {
                        return None;
                    }
                    if d > 0.72 {
                        let s = hash((v.atan2(u) * 5.0).floor(), 3.0, 0.0);
                        return c3(mul([128.0, 124.0, 132.0], 0.75 + s * 0.4 - (d - 0.72) * 0.8));
                    }
                    if d > 0.66 {
                        return c3([40.0, 32.0, 30.0]);
                    }
                    let e = hashi(i >> 2, j >> 2, 5);
                    if e > 0.55 { g3(mul([255.0, 120.0, 40.0], 0.6 + e * 0.5)) } else { c3([46.0, 30.0, 26.0]) }
                },
                1.0,
            ))
        })
    }
}

/// a clock's hand: a line with round ends (the canvas's stroke, lineCap round), in dots
fn hand(x: &mut Ctx, cx: f64, cy: f64, a: f64, len: f64, wdt: f64, col: Color) {
    let (x0, y0) = (cx * TS, cy * TS);
    let (x1, y1) = ((cx + a.sin() * len) * TS, (cy - a.cos() * len) * TS);
    let hw = wdt * TS / 2.0;
    let (dx, dy) = (x1 - x0, y1 - y0);
    let l = dx.hypot(dy);
    if l > 0.0 {
        let (nx, ny) = (-dy / l * hw, dx / l * hw);
        let mut p = Path::new();
        p.move_to(x0 + nx, y0 + ny);
        p.line_to(x1 + nx, y1 + ny);
        p.line_to(x1 - nx, y1 - ny);
        p.line_to(x0 - nx, y0 - ny);
        p.close();
        x.fill_path(&p, col);
    }
    x.fill_circle(x0, y0, hw, col);
    x.fill_circle(x1, y1, hw, col);
}

// a robot: legs, its torso in its colour (her skirt), arms in the pose, its head with its face on the front
fn robot_boxes(sp: &Robot) -> Vec<MBox> {
    let p = robot_parts(&sp.seed);
    let col = hex(&sp.color);
    let (light, light_d) = (col, mul(col, 0.62));
    // its eyes: its own light, or red (a mutineer)
    let (eye, eye_d) = if sp.red { ([255.0, 52.0, 40.0], [170.0, 20.0, 14.0]) } else { (light, light_d) };
    // its casing a cool steel, darker than the room's steel3 (lit warm, that went the colour of skin)
    let (s1, s2, s3, s4, dark, screen, ink2) = (hex("#3f4660"), hex("#6c7590"), hex("#8f9ab2"), hex("#c3cbdb"), hex("#2a2d3d"), hex(c::BLUE0), hex(c::INK2));
    let mut b: Vec<MBox> = Vec::new();
    let mut bx = |f0: f64, f1: f64, r0: f64, r1: f64, z0: f64, z1: f64, c: Rgb, flat: bool, glow: bool, round: f64, paint: Front| {
        b.push(MBox { f0, f1, r0, r1, z0, z1, col: c, metal: !flat && c != col, glow, round, paint })
    };
    macro_rules! bx {
        ($f0:expr, $f1:expr, $r0:expr, $r1:expr, $z0:expr, $z1:expr, $c:expr) => {
            bx($f0, $f1, $r0, $r1, $z0, $z1, $c, false, false, 0.0, None)
        };
    }
    let sit = sp.legs == "sit";
    if sit {
        for r in [-2.5, 0.7] {
            bx!(-1.2, 4.6, r, r + 1.8, 3.4, 5.2, s1);
            bx!(3.2, 5.0, r, r + 1.8, 0.8, 3.6, s1);
            bx!(3.2, 6.0, r - 0.1, r + 1.9, 0.0, 0.8, dark);
        }
    } else {
        let o = match sp.legs.as_str() {
            "walk1" => 1.3,
            "walk2" => -1.3,
            _ => 0.0,
        };
        for (r, df) in [(-2.5, o), (0.7, -o)] {
            bx!(-0.9 + df, 0.9 + df, r, r + 1.8, 0.8, 5.2, s1);
            bx!(-1.0 + df, 1.7 + df, r - 0.1, r + 1.9, 0.0, 0.8, dark);
        }
    }
    // a chest plate with its light
    let chest_hi = mix(light, [255.0; 3], 0.5);
    bx(-2.1, 2.1, -3.6, 3.6, 5.0, 12.4, col, false, false, 0.9, Some(Box::new(move |u, v| {
        if v > 0.25 && v < 0.55 && u > 0.36 && u < 0.64 {
            if u > 0.43 && u < 0.57 && v > 0.32 && v < 0.48 { g3(chest_hi) } else { c3(s3) }
        } else {
            c3(col)
        }
    })));
    if sp.femme {
        bx!(-2.7, 2.7, -4.3, 4.3, 4.6, 8.2, mul(col, 0.82));
    }
    bx!(-1.0, 1.0, -1.5, 1.5, 12.4, 13.2, s2);
    let rows = (12 - p.top) as f64;
    let top = 13.0 + rows; // the head: as tall as its portrait's (8 or 9 dots), ten wide
    let pt = p;
    // its front: the portrait's cells (x 3..12, y top..11), as robot() paints them
    let face_at = move |u: f64, v: f64| -> Px {
        let (cx, cy) = ((u * 10.0).floor() as i32, (v * rows).floor() as i32);
        let (gx, gy, ey) = (cx + 3, cy + pt.top, pt.top + 3);
        let mut c = if cy == 0 || cx == 0 {
            s4
        } else if cy == rows as i32 - 1 || cx == 9 {
            s1
        } else {
            s3
        };
        if (5..11).contains(&gx) && gy >= pt.top + 2 && gy < pt.top + 6 {
            c = screen;
        }
        let lit = |x0: i32, y0: i32| gx == x0 && gy == y0;
        let e = pt.eyes;
        if e == "dots" && (lit(6, ey) || lit(9, ey)) {
            return g3(eye);
        }
        if e == "dots" && (lit(6, ey + 1) || lit(9, ey + 1)) {
            return g3(eye_d);
        }
        if e == "visor" && gy == ey && (5..11).contains(&gx) {
            return g3(if gx % 2 != 0 { eye } else { eye_d });
        }
        if e == "cyclops" && (lit(7, ey) || lit(8, ey) || lit(7, ey + 1)) {
            return g3(eye);
        }
        if e == "cyclops" && lit(8, ey + 1) {
            return g3(eye_d);
        }
        if e == "happy" && (lit(6, ey + 1) || lit(5, ey) || lit(7, ey) || lit(9, ey + 1) || lit(8, ey) || lit(10, ey)) {
            return g3(eye);
        }
        if gy == 10 && (6..10).contains(&gx) {
            c = if pt.teeth && gx % 2 == 0 { ink2 } else { s1 };
        }
        c3(c)
    };
    bx(-3.6, 3.6, -5.0, 5.0, 13.2, top, s3, false, false, 1.4, Some(Box::new(face_at)));
    for r in [-5.6, 5.0] {
        bx!(-1.6, -0.2, r, r + 0.6, 16.4, 18.2, s2); // its ears, small bolts toward the back
    }
    let tip = |b: &mut Push, f0: f64, r0: f64, z0: f64, w: f64| b(f0, f0 + w, r0, r0 + w, z0, z0 + w, light, false, true, 0.0, None);
    match p.ant {
        "one" => {
            bx!(-0.35, 0.35, -0.35, 0.35, top, top + 2.2, s2);
            tip(&mut bx, -0.6, -0.6, top + 2.2, 1.2);
        }
        "two" => {
            for r in [-2.8, 2.2] {
                bx!(-0.35, 0.35, r, r + 0.6, top, top + 1.4, s2);
                tip(&mut bx, -0.55, r - 0.25, top + 1.4, 1.1);
            }
        }
        _ => {
            bx!(-0.5, 0.5, -2.0, 2.0, top, top + 0.9, s3);
            tip(&mut bx, -0.5, -0.5, top + 0.9, 1.0);
        }
    }
    if sp.femme {
        bx!(-0.9, 0.9, -3.4, -1.1, top + 0.2, top + 2.3, col);
        bx!(-0.9, 0.9, 1.1, 3.4, top + 0.2, top + 2.3, col);
        bx!(-0.7, 0.7, -1.1, 1.1, top + 0.6, top + 1.9, mul(col, 0.62));
    }
    // the arms, by the pose; what is held
    let r0 = |side: f64| if side < 0.0 { -5.6 } else { 3.6 };
    let arm = |bx: &mut Push, side: f64, f0: f64, f1: f64, z0: f64, z1: f64| bx(f0, f1, r0(side), r0(side) + 2.0, z0, z1, s2, false, false, 0.0, None);
    let hand = |bx: &mut Push, side: f64, f: f64, z: f64| bx(f - 0.9, f + 0.9, r0(side), r0(side) + 2.0, z, z + 1.2, s1, false, false, 0.0, None);
    let a = sp.arms.as_str();
    {
        let bx: &mut Push = &mut bx;
        let down = |bx: &mut Push, side: f64, df: f64| {
            arm(bx, side, -1.0 + df, 1.0 + df, 6.4, 12.0);
            hand(bx, side, df, 5.4);
        };
        let fwd = |bx: &mut Push, side: f64, z: f64| {
            bx(-1.0, 1.0, r0(side), r0(side) + 2.0, z + 1.0, 12.0, s2, false, false, 0.0, None);
            bx(0.8, 5.4, if side < 0.0 { -5.6 } else { 3.6 }, if side < 0.0 { -3.6 } else { 5.6 }, z, z + 1.8, s2, false, false, 0.0, None);
            hand(bx, side, 6.0, z + 0.3);
        };
        let pl = |bx: &mut Push, f0: f64, f1: f64, r0: f64, r1: f64, z0: f64, z1: f64, c: Rgb| bx(f0, f1, r0, r1, z0, z1, c, false, false, 0.0, None);
        match a {
            "up" => {
                for s in [-1.0, 1.0] {
                    bx(-1.0, 1.0, r0(s), r0(s) + 2.0, 12.0, 18.6, s2, false, false, 0.0, None);
                    hand(bx, s, 0.0, 18.6);
                }
            }
            "wave" => {
                down(bx, -1.0, 0.0);
                bx(-1.0, 1.0, r0(1.0), r0(1.0) + 2.0, 12.0, 18.6, s2, false, false, 0.0, None);
                hand(bx, 1.0, 0.0, 18.6);
            }
            "sw1" | "sw2" => {
                let o = if a == "sw1" { 1.2 } else { -1.2 };
                down(bx, -1.0, o);
                down(bx, 1.0, -o);
            }
            "fwd1" | "fwd2" => {
                fwd(bx, -1.0, if a == "fwd1" { 9.4 } else { 8.8 });
                fwd(bx, 1.0, if a == "fwd1" { 8.8 } else { 9.4 });
            }
            "reach" => {
                down(bx, -1.0, 0.0);
                bx(-1.0, 1.0, r0(1.0), r0(1.0) + 2.0, 11.0, 12.4, s2, false, false, 0.0, None);
                pl(bx, 0.8, 5.4, 3.6, 5.6, 11.2, 13.0, s2);
                hand(bx, 1.0, 6.0, 11.4);
            }
            "book" | "mug" | "paper" | "can" | "roll" | "gun" => {
                fwd(bx, -1.0, 9.0);
                fwd(bx, 1.0, 9.0);
                match a {
                    "book" => pl(bx, 4.2, 5.8, -2.2, 2.2, 9.4, 13.6, hex(c::RED1)),
                    "mug" => pl(bx, 5.4, 7.2, 2.4, 4.2, 10.0, 12.2, col),
                    "paper" => pl(bx, 4.6, 5.0, -2.4, 2.4, 9.4, 15.0, s4),
                    "can" => pl(bx, 5.2, 7.4, -1.2, 1.2, 9.6, 12.0, hex(c::GREEN0)),
                    "roll" => pl(bx, 4.0, 6.6, -3.0, 3.0, 8.6, 11.0, [77.0, 88.0, 102.0]),
                    _ => {
                        pl(bx, 1.0, 9.0, -0.6, 0.6, 10.2, 11.4, s1);
                        pl(bx, -1.5, 1.5, -0.7, 0.7, 9.2, 11.2, [122.0, 74.0, 42.0]);
                    }
                }
            }
            _ => {
                down(bx, -1.0, 0.0);
                down(bx, 1.0, 0.0);
            }
        }
    }
    b
}
// the dog, a dachshund: a long body, short legs, its ears, its tail
fn dog_boxes(pose: &str, wag: bool) -> Vec<MBox> {
    let (b, d, w, n) = (hex("#c27a3e"), hex("#7a4424"), hex("#efd8b4"), hex("#1a1626"));
    let mut out: Vec<MBox> = Vec::new();
    let mut bx = |f0: f64, f1: f64, r0: f64, r1: f64, z0: f64, z1: f64, c: Rgb, paint: Front| {
        out.push(MBox { f0, f1, r0, r1, z0, z1, col: c, metal: false, glow: false, round: 0.0, paint })
    };
    let head_at = |bx: &mut DogPush, f: f64, z: f64| {
        bx(f, f + 4.0, -2.0, 2.0, z, z + 4.2, b, Some(Box::new(move |u, v| c3(if v > 0.3 && v < 0.5 && ((u - 0.25).abs() < 0.08 || (u - 0.75).abs() < 0.08) { n } else { b }))));
        bx(f + 4.0, f + 6.2, -1.2, 1.2, z + 0.6, z + 2.6, w, None);
        bx(f + 6.2, f + 6.9, -0.6, 0.6, z + 1.6, z + 2.6, n, None);
        for r in [-2.7, 2.0] {
            bx(f + 0.6, f + 2.6, r, r + 0.7, z + 0.8, z + 4.4, d, None);
        }
    };
    let o = match pose {
        "walk1" => 0.8,
        "walk2" => -0.8,
        _ => 0.0,
    };
    let wag = if wag { 0.8 } else { -0.8 };
    let bx: &mut DogPush = &mut bx;
    match pose {
        "lie" => {
            bx(-6.0, 6.0, -2.2, 2.2, 0.4, 4.0, b, None);
            bx(-6.0, 6.0, -2.0, 2.0, 0.0, 0.6, w, None);
            head_at(bx, 5.5, 0.6);
            bx(-8.0, -6.0, -0.4 + wag, 0.4 + wag, 1.2, 2.0, d, None);
        }
        "sit" => {
            bx(-5.0, 1.0, -2.2, 2.2, 1.0, 5.5, b, None);
            bx(0.0, 5.4, -2.2, 2.2, 3.0, 8.0, b, None);
            for r in [-2.0, 1.0] {
                bx(2.6, 4.0, r, r + 1.2, 0.0, 3.4, b, None);
                bx(-4.5, -1.5, r - 0.2, r + 1.4, 0.0, 1.6, b, None);
            }
            head_at(bx, 4.0, 7.4);
            bx(-6.4, -5.0, -0.4 + wag, 0.4 + wag, 1.4, 2.4, d, None);
        }
        _ => {
            bx(-6.0, 6.0, -2.2, 2.2, 3.0, 7.0, b, None);
            bx(-5.0, 5.0, -1.8, 1.8, 2.6, 3.2, w, None);
            for (f, r, s) in [(4.2, -2.0, 1.0), (4.2, 0.8, -1.0), (-4.6, -2.0, -1.0), (-4.6, 0.8, 1.0)] {
                bx(f - 0.7 + s * o, f + 0.7 + s * o, r, r + 1.2, 0.0, 3.2, b, None);
            }
            head_at(bx, 5.0, 5.0);
            bx(-7.8, -6.0, -0.4 + wag, 0.4 + wag, 6.2, 7.2, d, None);
            bx(-8.6, -7.6, -0.4 + wag * 1.6, 0.4 + wag * 1.6, 7.0, 8.6, d, None);
        }
    }
    out
}
