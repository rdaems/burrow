//! burrow: the window's view (web/outside.js). Outside, by this computer's clock: the sky by the hour, the sun
//! or the moon on their arc, stars after dark, clouds going with the wind, hills in the haze behind.
//!
//! And it is a game: on the hills two guns, an artillery duel in the manner of the old DOS games (VGABomb,
//! Gorillas): you against an idle bot drawn by lot, or two bots' duel to watch. Angle and power, a wind that
//! changes each round (the clouds, the smoke and the pennants go with it), craters where the shells come down,
//! pines blown away; a gun hit is the round. A bot knows the physics and aims less well its first shots of a round.
//! With the room's radio on it is heard (burrow-sound, through `events()`): the gun, the whistle of the shell
//! coming down, the blast, a gun hit, the gunner landing, a tune for the round.
//!
//! Everything is in dots, the canvas's pixels (256 x 144, the app scales it to the pane); S is a dot of the
//! first, coarser version, kept for the physics. The hour is the caller's (`draw(img, hour)`), so is the seed.

use crate::{Event, Player, Rng, c, clamp, jround};
use pix::{Color, Ctx, Image, Op, css};
use std::collections::HashSet;
use std::f64::consts::PI;

pub const W: usize = 256;
pub const H: usize = 144;
const WF: f64 = W as f64;
const HF: f64 = H as f64;
const S: f64 = WF / 160.;
const G: f64 = 60. * S; // gravity (dots/s²)
const VK: f64 = 1.1 * S; // dots/s a power point
const PUSH: f64 = 9. * S; // the wind's push at full strength
const DT: f64 = 1. / 120.; // a step
const BOOM: f64 = 8.; // a shell's reach

/// outside.js's `hash`: FNV-1a over the UTF-16 units, 0..1
fn hash(s: &str) -> f64 {
    let mut h: u32 = 2166136261;
    for ch in s.encode_utf16() {
        h = (h ^ ch as u32).wrapping_mul(16777619);
    }
    h as f64 / 4294967296.
}
fn mix(a: Color, b: Color, k: f64) -> Color {
    a.mix(b, k)
}
fn hex(s: &str) -> Color {
    css(s)
}
fn r(g: &mut Ctx, x: f64, y: f64, w: f64, h: f64, col: Color) {
    g.fill_rect(x, y, w, h, col);
}
fn disc(g: &mut Ctx, x: f64, y: f64, rad: f64, col: Color) {
    let (x, y) = (jround(x), jround(y));
    let mut j = -rad.floor();
    while j <= rad {
        let h = (rad * rad - j * j).sqrt().floor();
        g.fill_rect(x - h, y + j, 2. * h + 1., 1., col);
        j += 1.;
    }
}
type Pt = (f64, f64, Color);
/// dots with an ink outline round them; the outline first, so it never covers a dot
fn inked(g: &mut Ctx, pts: &[Pt]) {
    for &(x, y, _) in pts {
        g.fill_rect(x - 1., y, 3., 1., c::INK);
        g.fill_rect(x, y - 1., 1., 3., c::INK);
    }
    for &(x, y, col) in pts {
        g.fill_rect(x, y, 1., 1., col);
    }
}
/// a letter grid at x0, y0: flip mirrors it, turn lays it on its back (its head where its top-left was)
fn grid(
    rows: &[&str],
    pal: &dyn Fn(u8) -> Option<Color>,
    x0: f64,
    y0: f64,
    flip: bool,
    turn: bool,
    out: &mut Vec<Pt>,
) {
    let h = rows.len();
    let w = rows[0].len();
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if ch == b'.' {
                continue;
            }
            let Some(col) = pal(ch) else { continue };
            let (i, j) = (i as f64, j as f64);
            out.push(if turn {
                (
                    x0 + if flip { (h - 1) as f64 - j } else { j },
                    y0 + (w - 1) as f64 - i,
                    col,
                )
            } else {
                (x0 + if flip { (w - 1) as f64 - i } else { i }, y0 + j, col)
            });
        }
    }
}
/// a thick line from (x0, y0) to (x1, y1), th dots across, with a lighter row along its top
#[allow(clippy::too_many_arguments)]
fn beam(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    th: i32,
    col: Color,
    hi: Option<Color>,
    out: &mut Vec<Pt>,
) {
    let n = ((x1 - x0).hypot(y1 - y0) * 2.).ceil();
    if n == 0. {
        return; // 0/0 in the page: nothing drawn
    }
    let mut seen = HashSet::new();
    let thf = th as f64;
    for k in 0..=n as i64 {
        let k = k as f64;
        let x = x0 + (x1 - x0) * k / n;
        let y = y0 + (y1 - y0) * k / n;
        for j in 0..th {
            for i in 0..th {
                let px = jround(x - thf / 2. + i as f64 + 0.5);
                let py = jround(y - thf / 2. + j as f64 + 0.5);
                if seen.insert((px as i64, py as i64)) {
                    out.push((px, py, if j == 0 { hi.unwrap_or(col) } else { col }));
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Pine {
    x: f64,
    h: f64,
}
/// the land: rolling hills at two levels, a mountain between them (a gentle rise without a duel), the far
/// hills in the haze behind, a few pines (the page keeps the heights in Float32Arrays: f32 here too)
#[derive(Clone, Debug)]
struct Land {
    g: Vec<f32>,
    far: Vec<f32>,
    pines: Vec<Pine>,
    scar: Vec<u8>,
    /// a change, for the picture's cache (a draw of the generator, as the page's `Math.random()`)
    v: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Aim,
    Fly,
    Boom,
    Over,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// ↑ ↓ the angle
    Up,
    Down,
    /// ← → the power
    Less,
    More,
    /// space or Enter
    Fire,
}

#[derive(Clone, Copy, Debug)]
struct Plan {
    a: f64,
    p: f64,
    wait: f64,
    hold: f64,
}
#[derive(Clone, Copy, Debug)]
struct Body {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    down: bool,
}
#[derive(Clone, Debug)]
struct Gun {
    p: Player,
    side: usize,
    f: f64,
    x: f64,
    ang: f64,
    pow: f64,
    shots: i32,
    kick: f64,
    flash: f64,
    dead: bool,
    body: Option<Body>,
    trail: Vec<(f64, f64)>,
    plan: Option<Plan>,
    skill: f64,
    leaving: bool,
    walk: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Shell {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    by: usize,
    age: f64,
    /// heard on its way down: the sound's id, for stopping it where it lands
    whistle: Option<u32>,
}
/// where a shell comes down
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Down {
    /// off the side
    Out,
    /// gun: the side whose gun it hits
    At { x: f64, y: f64, gun: Option<usize> },
}
#[derive(Clone, Copy, Debug)]
struct Boom {
    x: f64,
    y: f64,
    t: f64,
    r: f64,
}
#[derive(Clone, Copy, Debug)]
struct Bit {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    t: f64,
    col: Color,
}
#[derive(Clone, Debug)]
struct Duel {
    p: [Gun; 2],
    score: [u32; 2],
    fired: [f64; 2],
    watch: bool,
    turn: usize,
    phase: Phase,
    t: f64,
    shell: Option<Shell>,
    booms: Vec<Boom>,
    bits: Vec<Bit>,
    won: Option<usize>,
    tune: f64,
    next: usize,
    gone: Option<usize>,
}

/// the duel at the window for the room: the bot you play (or the two you watch), when each last fired (in
/// this game's clock: compare with `now()`)
#[derive(Clone, Debug, PartialEq)]
pub struct Playing {
    pub id: String,
    pub ids: Option<[String; 2]>,
    pub watch: bool,
    pub at: Option<String>,
    pub fired: [f64; 2],
}

/// the duel's line over the picture: the two, the one whose turn it is lit with its angle and power, the
/// rounds won between them
#[derive(Clone, Debug, PartialEq)]
pub struct Hud {
    pub sides: [HudSide; 2],
    pub score: [u32; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct HudSide {
    /// "you" for you
    pub name: String,
    /// its turn: the name in gold
    pub on: bool,
    /// "angle 45 · power 50" while its turn, "left" once it walked off
    pub say: Option<String>,
}

/// for tests: the duel as window.__outside.get() gives it
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub phase: Phase,
    pub turn: usize,
    pub score: [u32; 2],
    pub wind: f64,
    pub gone: Option<usize>,
    pub guns: [GunState; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct GunState {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ang: f64,
    pub pow: f64,
    pub dead: bool,
    pub shots: i32,
}

struct Cloud {
    x: f64,
    y: f64,
    k: f64,
    s: bool,
}
struct Held {
    t: f64,
    owe: f64,
}

pub struct Outside {
    rng: Rng,
    land: Land,
    wind: f64,
    duel: Option<Duel>,
    at: Option<String>,
    acc: f64,
    t: f64,
    clouds: Vec<Cloud>,
    held: Vec<(Key, Held)>,
    events: Vec<Event>,
    sounds: u32,
    stars: Vec<(f64, f64, i32)>,
    // the sky and the land are drawn into images of their own, again only when they change
    sky_img: Image,
    land_img: Image,
    sky_was: Option<Color>,
    land_was: Option<(u64, Color, usize)>,
}

const SKY: [(f64, u32); 9] = [
    (0., 0x0d1430),
    (5., 0x141d3f),
    (6.5, 0xd9775a),
    (8., 0x7fb2e0),
    (17., 0x7fb2e0),
    (19., 0xe07a4a),
    (20.5, 0x2a2550),
    (22., 0x0d1430),
    (24., 0x0d1430),
];
pub fn sky_at(h: f64) -> Color {
    for i in 1..SKY.len() {
        if h <= SKY[i].0 {
            let ((h0, c0), (h1, c1)) = (SKY[i - 1], SKY[i]);
            return mix(Color::rgb(c0), Color::rgb(c1), (h - h0) / (h1 - h0));
        }
    }
    Color::rgb(SKY[0].1)
}
/// how dark it is at an hour, 0..1
pub fn dark_at(h: f64) -> f64 {
    if !(6. ..=20.5).contains(&h) {
        1.
    } else if h < 8. {
        (8. - h) / 2.
    } else if h > 18.5 {
        (h - 18.5) / 2.
    } else {
        0.
    }
}

/// a pine h dots high: three tiers on a trunk, lit from the left
fn pine_rows(h: f64) -> Vec<String> {
    let mut rows = Vec::new();
    let tier = (h - 2.) / 3.;
    let mut k = 0.;
    while k < h - 2. {
        let t = (k / tier).floor();
        let q = k - t * tier;
        let half = 4f64.min((1. + t + q * 0.55).floor()) as usize;
        rows.push(format!(
            "{}l{}{}",
            ".".repeat(5 - half),
            "d".repeat(2 * half - 1),
            ".".repeat(5 - half)
        ));
        k += 1.;
    }
    rows.push(".....t....".into());
    rows.push(".....t....".into());
    rows
}

// the gunners, facing right: you as the portrait wears it, a bot a robot with its colour on its antenna and chest
const PERSON: [&str; 13] = [
    "..hhh..", ".hhhhh.", ".hhsks.", ".hssss.", "..sss..", ".ccccc.", "cccccss", "ccdcc..",
    ".cdcc..", ".cccc..", ".ll.l..", ".ll.l..", ".bb.bb.",
];
const ROBOT: [&str; 13] = [
    "....a..", "....n..", ".mmmmm.", ".mmmee.", ".mmmmm.", "..nnn..", ".mmmmm.", "mmkkmmh",
    ".mkkmn.", ".mmmmn.", "..n.n..", "..n.n..", ".mm.mm.",
];
const WHEEL: [&str; 7] = [
    "..rrr..", ".rbsbr.", "rbbsbbr", "rssxssr", "rbbsbbr", ".rbsbr.", "..rrr..",
];

impl Outside {
    pub fn new(seed: u64) -> Outside {
        let mut rng = Rng::new(seed);
        let land = hills(&mut rng, false);
        let stars = (0..110i64)
            .map(|i| {
                (
                    ((i * 37 + (i >> 3) * 11) % W as i64) as f64,
                    ((i * 53) % jround(HF * 0.62) as i64) as f64,
                    (i % 4) as i32,
                )
            })
            .collect();
        Outside {
            rng,
            land,
            wind: 0.3,
            duel: None,
            at: None,
            acc: 0.,
            t: 0.,
            clouds: (0..4)
                .map(|i| Cloud {
                    x: (i * 71 + 20) as f64,
                    y: (12 + (i % 3) * 15) as f64,
                    k: 0.7 + 0.15 * i as f64,
                    s: i % 2 == 1,
                })
                .collect(),
            held: Vec::new(),
            events: Vec::new(),
            sounds: 0,
            stars,
            sky_img: Image::new(W as u32, H as u32),
            land_img: Image::new(W as u32, H as u32),
            sky_was: None,
            land_was: None,
        }
    }

    /// open the view: a and b at the guns (b none: only the outside); `key` is the window's furniture key
    pub fn open(&mut self, a: Player, b: Option<Player>, key: Option<String>) {
        self.at = key;
        self.held.clear();
        if let Some(b) = b {
            let watch = !a.human;
            let p = [self.gun(a, 0), self.gun(b, 1)];
            self.duel = Some(Duel {
                p,
                score: [0, 0],
                fired: [f64::NEG_INFINITY; 2],
                watch,
                turn: 0,
                phase: Phase::Aim,
                t: 0.,
                shell: None,
                booms: Vec::new(),
                bits: Vec::new(),
                won: None,
                tune: 0.,
                next: 0,
                gone: None,
            });
            self.round(0);
        } else {
            self.duel = None;
            self.new_land(false);
            self.wind = 0.3;
        }
    }
    pub fn close(&mut self) {
        if let Some(id) = self
            .duel
            .as_ref()
            .and_then(|d| d.shell)
            .and_then(|s| s.whistle)
        {
            self.events.push(Event::Stop(id));
        }
        self.duel = None;
        self.at = None;
    }
    pub fn playing(&self) -> Option<Playing> {
        let d = self.duel.as_ref()?;
        if d.phase == Phase::Left {
            return None;
        }
        Some(Playing {
            id: d.p[1].p.id.clone(),
            ids: d.watch.then(|| [d.p[0].p.id.clone(), d.p[1].p.id.clone()]),
            watch: d.watch,
            at: self.at.clone(),
            fired: d.fired,
        })
    }
    /// a bot that gets work walks off; the duel is over
    pub fn left(&mut self, id: &str) {
        let Some(d) = self.duel.as_mut() else { return };
        if d.gone.is_some() {
            return;
        }
        let Some(side) = d.p.iter().position(|q| q.p.id == id && !q.p.human) else {
            return;
        };
        d.gone = Some(side);
        d.p[side].leaving = true;
        d.p[side].walk = 0.;
        if d.phase == Phase::Aim {
            d.phase = Phase::Left;
        }
    }
    /// the sounds since the last call
    pub fn events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// this game's clock, seconds
    pub fn now(&self) -> f64 {
        self.t
    }

    fn new_land(&mut self, mountain: bool) {
        self.land = hills(&mut self.rng, mountain);
    }
    fn hear(&mut self, name: &'static str, x: f64, dur: Option<f64>) -> u32 {
        self.sounds += 1;
        let pan = clamp((x / WF) * 1.6 - 0.8, -1., 1.);
        self.events.push(Event::Sound {
            name,
            pan,
            dur,
            id: self.sounds,
        });
        self.sounds
    }
    fn ground(&self, x: f64) -> f64 {
        ground(&self.land, x)
    }
    fn crater(&mut self, x: f64, y: f64, rad: f64) {
        let mut i = (x - rad).ceil();
        while i <= x + rad {
            if i >= 0. && i < WF {
                let low = y + (rad * rad - (i - x).powi(2)).max(0.).sqrt();
                let iu = i as usize;
                if (self.land.g[iu] as f64) < low {
                    self.land.g[iu] = (HF - 2.).min(low) as f32;
                    self.land.scar[iu] = 1;
                }
            }
            i += 1.;
        }
        self.land.v = (self.rng.next() * 9007199254740992.) as u64;
    }

    // ---- a duel: two guns, a turn each, the round to whoever hits the other's gun
    fn gun(&self, p: Player, side: usize) -> Gun {
        let skill = 0.75 + 0.6 * hash(&p.id);
        Gun {
            p,
            side,
            f: if side == 1 { -1. } else { 1. },
            x: 0.,
            ang: 45.,
            pow: 50.,
            shots: 0,
            kick: 0.,
            flash: 0.,
            dead: false,
            body: None,
            trail: Vec::new(),
            plan: None,
            skill,
            leaving: false,
            walk: 0.,
        }
    }
    fn round(&mut self, starter: usize) {
        self.new_land(true);
        let s = if self.rng.next() < 0.5 { -1. } else { 1. };
        self.wind = s * self.rng.next().powf(1.4);
        for k in 0..2 {
            let f = self.duel.as_ref().unwrap().p[k].f;
            let x = if f > 0. {
                jround(self.rng.range(24., 50.))
            } else {
                WF - 1. - jround(self.rng.range(24., 50.))
            };
            let (a, b) = {
                let (u, v) = (x - 23. * f, x + 7. * f);
                (u.min(v) as usize, u.max(v) as usize)
            };
            let g = &mut self.land.g;
            let sum: f64 = g[a..=b].iter().map(|&v| v as f64).sum();
            let h = jround(sum / (b - a + 1) as f64);
            for v in &mut g[a..=b] {
                *v = h as f32; // a level pad for the gun, its gunner and its pennant,
            }
            for d in 1..=14i64 {
                for xx in [a as i64 - d, b as i64 + d] {
                    // sloping into the hill round it
                    if xx < 0 || xx >= W as i64 {
                        continue;
                    }
                    let k = d as f64 / 15.;
                    let s = k * k * (3. - 2. * k);
                    let xx = xx as usize;
                    g[xx] = (h + (g[xx] as f64 - h) * s) as f32;
                }
            }
            let (a, b) = (a as f64, b as f64);
            self.land.pines.retain(|p| p.x < a - 5. || p.x > b + 5.);
            let gun = &mut self.duel.as_mut().unwrap().p[k];
            gun.x = x;
            gun.shots = 0;
            gun.kick = 0.;
            gun.flash = 0.;
            gun.dead = false;
            gun.body = None;
            gun.trail.clear();
            gun.plan = None;
        }
        let d = self.duel.as_mut().unwrap();
        d.turn = starter;
        d.phase = Phase::Aim;
        d.t = 0.;
        d.shell = None;
        d.booms.clear();
        d.bits.clear();
    }
    /// the gun's pivot
    fn pivot(&self, side: usize) -> (f64, f64) {
        let x = self.duel.as_ref().unwrap().p[side].x;
        (x, self.ground(x) - 6.)
    }
    /// a shell leaving the muzzle at angle a in degrees over the ground toward the other side, power p
    pub fn launch(&self, side: usize, a: f64, p: f64) -> Shell {
        let f = self.duel.as_ref().unwrap().p[side].f;
        let rad = a * PI / 180.;
        let (dx, dy) = (f * rad.cos(), -rad.sin());
        let (x0, y0) = self.pivot(side);
        Shell {
            x: x0 + dx * 11.,
            y: y0 + dy * 11.,
            vx: dx * p * VK,
            vy: dy * p * VK,
            by: side,
            age: 0.,
            whistle: None,
        }
    }
    /// a step of a shell in the air
    pub fn fly(&self, s: &mut Shell) {
        fly(self.wind, s);
    }
    /// where it comes down; none: still in the air
    pub fn down(&self, s: &Shell) -> Option<Down> {
        if s.x < -6. || s.x > WF + 5. || s.y > HF + 6. {
            return Some(Down::Out);
        }
        for q in &self.duel.as_ref().unwrap().p {
            if q.dead || (q.side == s.by && s.age < 0.25) {
                continue;
            }
            let gy = self.ground(q.x);
            if (s.x - q.x).abs() < 5. && s.y > gy - 9. && s.y < gy + 1. {
                return Some(Down::At {
                    x: s.x,
                    y: s.y,
                    gun: Some(q.side),
                });
            }
        }
        if s.x >= 0. && s.x < WF && s.y >= self.ground(s.x) {
            return Some(Down::At {
                x: s.x,
                y: self.ground(s.x),
                gun: None,
            });
        }
        None
    }
    /// a shot played out
    pub fn sim(&self, side: usize, a: f64, p: f64) -> Down {
        let mut s = self.launch(side, a, p);
        for _ in 0..1500 {
            fly(self.wind, &mut s);
            if let Some(d) = self.down(&s) {
                return d;
            }
        }
        Down::Out
    }
    /// a bot's aim: for every angle the power that comes closest, then its own way of shooting (an angle it
    /// likes, seeded by its id), then the hand: far off the first shot of a round, closer with every shot
    fn plan(&mut self, side: usize, shots: i32, skill: f64) -> Plan {
        let d = self.duel.as_ref().unwrap();
        let q = &d.p[1 - side];
        let (qside, tx) = (q.side, q.x);
        let ty = self.ground(tx) - 4.;
        let miss = |d: Down| match d {
            Down::Out => 1e9,
            Down::At { gun: Some(g), .. } if g == qside => 0.,
            Down::At { x, y, gun } => {
                (x - tx).hypot(y - ty) + if gun == Some(side) { 120. } else { 0. }
            }
        };
        let mut best: Vec<(f64, f64, f64)> = Vec::new();
        let mut a = 20.;
        while a <= 80. {
            let mut b: Option<(f64, f64, f64)> = None;
            let mut p = 10.;
            while p <= 100. {
                let dd = miss(self.sim(side, a, p));
                if b.is_none_or(|b| dd < b.2) {
                    b = Some((a, p, dd));
                }
                p += 1.5;
            }
            let mut b = b.unwrap();
            let mut p = b.1 - 1.5;
            let hi = b.1 + 1.5;
            while p <= hi {
                let dd = miss(self.sim(side, a, p));
                if dd < b.2 {
                    b = (a, p, dd);
                }
                p += 0.25;
            }
            best.push(b);
            a += 4.;
        }
        let id = &self.duel.as_ref().unwrap().p[side].p.id;
        let likes = 30. + 40. * hash(&format!("{id}a"));
        let ok: Vec<_> = best.iter().copied().filter(|b| b.2 < 5.).collect();
        let b = if !ok.is_empty() {
            ok.iter()
                .copied()
                .reduce(|x, y| {
                    if (y.0 - likes).abs() < (x.0 - likes).abs() {
                        y
                    } else {
                        x
                    }
                })
                .unwrap()
        } else {
            best.iter()
                .copied()
                .reduce(|x, y| if y.2 < x.2 { y } else { x })
                .unwrap()
        };
        let k = shots as f64;
        let sp = (9. * 0.55f64.powf(k) + 0.5) * skill;
        let sa = 2. * 0.6f64.powf(k) * skill;
        let ga = self.rng.gauss();
        let gp = self.rng.gauss();
        Plan {
            a: clamp(jround(b.0 + ga * sa), 5., 85.),
            p: clamp(jround(b.1 + gp * sp), 5., 100.),
            wait: self.rng.range(0.6, 1.3),
            hold: 0.,
        }
    }
    fn fire(&mut self, side: usize) {
        let (ang, pow) = {
            let g = &self.duel.as_ref().unwrap().p[side];
            (g.ang, g.pow)
        };
        let shell = self.launch(side, ang, pow);
        let t = self.t;
        let d = self.duel.as_mut().unwrap();
        d.shell = Some(shell);
        d.phase = Phase::Fly;
        let g = &mut d.p[side];
        g.shots += 1;
        g.kick = 1.;
        g.flash = 0.12;
        g.trail.clear();
        let x = g.x;
        d.fired[side] = t;
        self.hear("cannon", x, None);
    }
    fn boom(&mut self, dn: Down) {
        let d = self.duel.as_mut().unwrap();
        let s = d.shell.take().unwrap();
        let by = s.by;
        d.phase = Phase::Boom;
        d.t = 0.9;
        if let Some(id) = s.whistle {
            self.events.push(Event::Stop(id));
        }
        let Down::At { x: dx, y: dy, gun } = dn else {
            return;
        };
        let hit: Vec<usize> = (0..2)
            .filter(|&k| {
                let q = &self.duel.as_ref().unwrap().p[k];
                !q.dead
                    && (gun == Some(q.side)
                        || (dx - q.x).hypot(dy - (self.ground(q.x) - 4.)) <= BOOM)
            })
            .collect();
        let rr = if hit.is_empty() { 8. } else { 13. };
        self.duel.as_mut().unwrap().booms.push(Boom {
            x: dx,
            y: dy,
            t: 0.,
            r: rr,
        });
        let xi = clamp(jround(dx), 0., WF - 1.) as usize;
        let soil = if self.land.scar[xi] != 0 {
            [hex("#8a6239"), hex("#6b4a2b")]
        } else {
            [hex("#6aa84f"), hex("#6b4a2b")]
        };
        let mut bits = Vec::new();
        for k in 0..(if hit.is_empty() { 22 } else { 40 }) {
            let a = self.rng.range(PI * 1.05, PI * 1.95);
            let v = self.rng.range(26., 70.);
            bits.push(Bit {
                x: dx,
                y: dy - 1.,
                vx: a.cos() * v,
                vy: a.sin() * v,
                t: 0.,
                col: if k % 4 == 0 {
                    hex("#3f3740")
                } else {
                    soil[k % 2]
                },
            });
        }
        // pines nearby go up in the blast
        let near: Vec<Pine> = self
            .land
            .pines
            .iter()
            .filter(|p| (p.x - dx).abs() < BOOM + 2. && (self.ground(p.x) - dy).abs() < BOOM + 4.)
            .cloned()
            .collect();
        for p in &near {
            for k in 0..10 {
                let y = self.ground(p.x) - self.rng.range(2., p.h);
                let vx = self.rng.range(-30., 30.);
                let vy = -self.rng.range(20., 50.);
                bits.push(Bit {
                    x: p.x,
                    y,
                    vx,
                    vy,
                    t: 0.,
                    col: if k % 3 != 0 {
                        hex("#1f4a2e")
                    } else {
                        hex("#5a3a22")
                    },
                });
            }
        }
        self.land.pines.retain(|p| !near.iter().any(|n| n.x == p.x));
        for &k in &hit {
            let (x, f) = {
                let q = &self.duel.as_ref().unwrap().p[k];
                (q.x, q.f)
            };
            let y = self.ground(x) - 1.;
            let vx = -f * self.rng.range(22., 38.);
            let vy = -self.rng.range(48., 66.);
            let q = &mut self.duel.as_mut().unwrap().p[k];
            q.dead = true;
            q.body = Some(Body {
                x: x - 15. * f,
                y,
                vx,
                vy,
                down: false,
            });
        }
        self.duel.as_mut().unwrap().bits.extend(bits);
        self.crater(dx, dy, BOOM);
        self.hear(if hit.is_empty() { "blast" } else { "wreck" }, dx, None);
        if !hit.is_empty() {
            let d = self.duel.as_mut().unwrap();
            let win = if hit.len() == 2 {
                None
            } else {
                Some(1 - d.p[hit[0]].side)
            };
            if let Some(w) = win {
                d.score[w] += 1;
            }
            d.won = win;
            d.tune = 1.1;
            d.phase = Phase::Over;
            d.t = 3.6;
            d.next = if hit.len() == 2 {
                1 - by
            } else {
                d.p[hit[0]].side
            }; // the loser starts the next round
        }
    }

    /// time going by (the frame's dt, at most 0.05 s): your keys, a step of the duel, the clouds and the bits
    pub fn tick(&mut self, dt: f64) {
        let dt = dt.clamp(0., 0.05);
        self.t += dt;
        self.aim(dt);
        self.update(dt);
    }
    fn update(&mut self, dt: f64) {
        let wind = self.wind;
        for cl in &mut self.clouds {
            cl.x += wind * 13. * cl.k * dt;
            if cl.x > WF + 30. {
                cl.x -= WF + 60.;
            }
            if cl.x < -30. {
                cl.x += WF + 60.;
            }
        }
        if self.duel.is_none() {
            return;
        }
        let mut thuds = Vec::new();
        {
            let land = &self.land;
            let d = self.duel.as_mut().unwrap();
            for p in &mut d.p {
                p.kick = (p.kick - dt * 5.).max(0.);
                p.flash = (p.flash - dt).max(0.);
                if let Some(b) = p.body.as_mut().filter(|b| !b.down) {
                    b.vy += G * dt;
                    b.x += b.vx * dt;
                    b.y += b.vy * dt;
                    b.x = clamp(b.x, 7., WF - 8.);
                    if b.vy > 0. && b.y >= ground(land, b.x) - 1. {
                        b.y = ground(land, b.x) - 1.;
                        b.down = true;
                        thuds.push(b.x);
                    }
                }
                if p.leaving {
                    p.walk += dt * 22.;
                }
            }
        }
        for x in thuds {
            self.hear("thud", x, None);
        }
        let d = self.duel.as_mut().unwrap();
        let mut tune = None;
        if d.tune > 0. {
            d.tune -= dt;
            if d.tune <= 0. {
                if let Some(won) = d.won {
                    // the round's tune: yours won, or lost (a duel you watch: won)
                    let mine = d.p.iter().find(|p| p.p.human).map(|p| p.side);
                    tune = Some(if mine.is_none_or(|s| s == won) {
                        "win"
                    } else {
                        "lose"
                    });
                }
            }
        }
        if let Some(name) = tune {
            self.hear(name, WF / 2., None);
        }
        let land = &self.land;
        let d = self.duel.as_mut().unwrap();
        for bm in &mut d.booms {
            bm.t += dt;
        }
        d.booms.retain(|b| b.t < 1.8);
        for b in &mut d.bits {
            b.t += dt;
            b.vy += G * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
        }
        d.bits
            .retain(|b| b.t < 2.5 && b.x > -2. && b.x < WF + 2. && b.y < ground(land, b.x));
        if d.shell.is_some() {
            self.acc += dt;
            while self.acc >= DT {
                let Some(mut sh) = self.duel.as_ref().unwrap().shell else {
                    break;
                };
                self.acc -= DT;
                fly(self.wind, &mut sh);
                if sh.whistle.is_none() && sh.vy > 0. {
                    // over the top: it whistles on the way down, until it lands
                    let mut cp = sh;
                    let mut n = 0;
                    while n < 1500 && self.down(&cp).is_none() {
                        fly(self.wind, &mut cp);
                        n += 1;
                    }
                    sh.whistle = Some(self.hear("whistle", sh.x, Some(n as f64 * DT)));
                }
                let d = self.duel.as_mut().unwrap();
                d.shell = Some(sh);
                if jround(sh.age / DT) as i64 % 3 == 0 {
                    d.p[sh.by].trail.push((sh.x, sh.y));
                }
                if let Some(dn) = self.down(&sh) {
                    self.boom(dn);
                }
            }
            return;
        }
        self.acc = 0.;
        let d = self.duel.as_mut().unwrap();
        if d.phase == Phase::Boom {
            d.t -= dt;
            if d.t <= 0. {
                if d.gone.is_some() {
                    d.phase = Phase::Left;
                } else {
                    d.turn = 1 - d.turn;
                    d.phase = Phase::Aim;
                }
            }
        }
        if d.phase == Phase::Over {
            d.t -= dt;
            if d.t <= 0. {
                if d.gone.is_some() {
                    d.phase = Phase::Left;
                } else {
                    let next = d.next;
                    self.round(next);
                }
            }
        }
        let d = self.duel.as_ref().unwrap();
        if d.phase != Phase::Aim {
            return;
        }
        let side = d.turn;
        let g = &d.p[side];
        if g.p.human {
            return; // yours: the keys (aim, below)
        }
        if g.plan.is_none() {
            let (shots, skill) = (g.shots, g.skill);
            let plan = self.plan(side, shots, skill);
            self.duel.as_mut().unwrap().p[side].plan = Some(plan);
        }
        let g = &mut self.duel.as_mut().unwrap().p[side];
        let pl = g.plan.as_mut().unwrap();
        pl.wait -= dt;
        if pl.wait > 0. {
            return;
        }
        // the barrel swings round to its angle, the power comes up to its mark, then it fires
        g.ang += clamp(pl.a - g.ang, -40. * dt, 40. * dt);
        g.pow += clamp(pl.p - g.pow, -50. * dt, 50. * dt);
        if (g.ang - pl.a).abs() < 0.01 && (g.pow - pl.p).abs() < 0.01 {
            g.ang = pl.a;
            g.pow = pl.p;
            pl.hold += dt;
            if pl.hold > 0.35 {
                g.plan = None;
                self.fire(side);
            }
        }
    }

    // you: ↑ ↓ the angle, ← → the power, a step a press and faster while held; space or Enter fires
    fn you(&self) -> Option<usize> {
        let d = self.duel.as_ref()?;
        if d.phase == Phase::Left || d.phase == Phase::Over {
            return None;
        }
        d.p.iter().position(|p| p.p.human)
    }
    fn aim(&mut self, dt: f64) {
        let Some(side) = self.you() else { return };
        let now = self.t;
        let mut steps = Vec::new();
        for (k, h) in &mut self.held {
            let t = now - h.t;
            if t < 0.3 {
                continue;
            }
            h.owe += dt * (10. + 40. * ((t - 0.3) / 1.2).min(1.));
            let n = h.owe.floor();
            h.owe -= n;
            if n != 0. {
                steps.push((*k, n));
            }
        }
        for (k, n) in steps {
            self.nudge(side, k, n);
        }
    }
    fn nudge(&mut self, side: usize, k: Key, n: f64) {
        let g = &mut self.duel.as_mut().unwrap().p[side];
        match k {
            Key::Up | Key::Down => {
                g.ang = clamp(jround(g.ang) + if k == Key::Up { n } else { -n }, 0., 90.)
            }
            Key::Less | Key::More => {
                g.pow = clamp(
                    jround(g.pow) + if k == Key::More { n } else { -n },
                    0.,
                    100.,
                )
            }
            Key::Fire => {}
        }
    }
    /// a key pressed while the view is shown (the app leaves out presses with ⌘, ⌃ or ⌥); `repeat`: the
    /// keyboard's own repeat
    pub fn key_down(&mut self, k: Key, repeat: bool) {
        let Some(d) = self.duel.as_ref() else { return };
        let me = self.you();
        if k == Key::Fire {
            if let Some(side) = me {
                if !repeat && d.phase == Phase::Aim && d.turn == side {
                    self.fire(side);
                }
            }
            return;
        }
        if let Some(side) = me {
            if !self.held.iter().any(|(h, _)| *h == k) {
                self.held.push((k, Held { t: self.t, owe: 0. }));
                self.nudge(side, k, 1.);
            }
        }
    }
    pub fn key_up(&mut self, k: Key) {
        self.held.retain(|(h, _)| *h != k);
    }
    /// the window lost the keys
    pub fn blur(&mut self) {
        self.held.clear();
    }

    pub fn hud(&self) -> Option<Hud> {
        let d = self.duel.as_ref()?;
        let side = |p: &Gun| {
            let on = d.turn == p.side && matches!(d.phase, Phase::Aim | Phase::Fly | Phase::Boom);
            let say = if on {
                Some(format!("angle {} · power {}", jround(p.ang), jround(p.pow)))
            } else if d.gone == Some(p.side) {
                Some("left".into())
            } else {
                None
            };
            HudSide {
                name: if p.p.human {
                    "you".into()
                } else {
                    p.p.name.clone()
                },
                on,
                say,
            }
        };
        Some(Hud {
            sides: [side(&d.p[0]), side(&d.p[1])],
            score: d.score,
        })
    }

    // ---- the picture
    fn paint_sky(&mut self, sky: Color, dark: f64) {
        if self.sky_was == Some(sky) {
            return;
        }
        self.sky_was = Some(sky);
        let mut g = self.sky_img.ctx();
        let n = 7.;
        let bh = (HF * 0.8 / n).ceil();
        let band = |b: f64| mix(sky, c::WHITE, b.min(n - 1.) * (0.028 * (1. - dark) + 0.01)); // lighter toward the horizon
        for b in 0..=7 {
            let b = b as f64;
            r(
                &mut g,
                0.,
                b * bh,
                WF,
                if b == n { HF } else { bh },
                band(b),
            );
        }
        for b in 1..=7 {
            let b = b as f64;
            // dithered where two bands meet, as a 16-bit sky would be
            let y = b * bh;
            let col = band(b - 1.);
            for x in 0..W {
                if x % 2 == 0 {
                    r(&mut g, x as f64, y, 1., 1., col);
                }
                if x % 4 == 1 {
                    r(&mut g, x as f64, y + 1., 1., 1., col);
                }
            }
        }
    }
    fn paint_land(&mut self, sky: Color, dark: f64) {
        let key = (
            self.land.v,
            self.sky_was.unwrap_or(sky),
            self.land.pines.len(),
        );
        if self.land_was == Some(key) {
            return;
        }
        self.land_was = Some(key);
        self.land_img.clear();
        let land = &self.land;
        let mut g = self.land_img.ctx();
        let haze = mix(mix(hex("#3d6b55"), sky, 0.5), hex("#0c1418"), dark * 0.55);
        let haze2 = mix(haze, c::WHITE, 0.12 * (1. - dark));
        for x in 0..W {
            let t = jround(land.far[x] as f64);
            let xf = x as f64;
            r(&mut g, xf, t, 1., HF - t, haze);
            r(&mut g, xf, t, 1., 1., haze2);
        }
        // the hills: grass along the top (earth where a shell came down), darker strata further in, dithered between
        let bands: Vec<Color> = ["#2f5a3c", "#2b4a3a", "#24402f", "#1d3328", "#182b22"]
            .iter()
            .map(|b| mix(hex(b), hex("#0c1418"), dark * 0.8))
            .collect();
        let grass = [
            mix(hex("#7cc05a"), hex("#1a2a1e"), dark),
            mix(hex("#4a7a3e"), hex("#122019"), dark),
        ];
        let earth = [
            mix(hex("#a77a4a"), hex("#1a1410"), dark),
            mix(hex("#6b4a2b"), hex("#140f0c"), dark),
        ];
        for x in 0..W {
            let top = jround(land.g[x] as f64);
            let soil = if land.scar[x] != 0 { earth } else { grass };
            let xf = x as f64;
            let xi = x as i64;
            let mut b = 0usize;
            loop {
                let y0 = top + 3. + b as f64 * 10.;
                if y0 >= HF {
                    break;
                }
                r(
                    &mut g,
                    xf,
                    y0,
                    1.,
                    10f64.min(HF - y0),
                    bands[b.min(bands.len() - 1)],
                );
                if b > 0 && b < bands.len() {
                    let s = xi + y0 as i64;
                    if s.rem_euclid(2) != 0 {
                        r(&mut g, xf, y0, 1., 1., bands[b - 1]);
                    }
                    if s.rem_euclid(4) == 1 {
                        r(&mut g, xf, y0 + 1., 1., 1., bands[b - 1]);
                    }
                }
                b += 1;
            }
            r(&mut g, xf, top, 1., 1., soil[0]);
            r(&mut g, xf, top + 1., 1., 2., soil[1]);
            if land.scar[x] == 0 && (xi * 7 + top as i64).rem_euclid(6) == 0 {
                r(&mut g, xf, top - 1., 1., 1., soil[1]); // tufts
            }
        }
        let (pl, pd, pt) = (
            mix(hex("#3f8a52"), hex("#0c1a12"), dark),
            mix(hex("#1f4a2e"), hex("#08110c"), dark),
            mix(hex("#5a3a22"), hex("#140f0c"), dark),
        );
        let pal = move |ch: u8| match ch {
            b'l' => Some(pl),
            b'd' => Some(pd),
            b't' => Some(pt),
            _ => None,
        };
        for pn in &land.pines {
            let rows = pine_rows(pn.h);
            let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
            let mut pts = Vec::new();
            grid(
                &rows,
                &pal,
                pn.x - 4.,
                jround(ground(land, pn.x)) - rows.len() as f64,
                false,
                false,
                &mut pts,
            );
            inked(&mut g, &pts);
        }
    }

    /// the picture at this hour of the day (local, 0..24 with the minutes)
    pub fn draw(&mut self, img: &mut Image, hour: f64) {
        let h = hour;
        let sky = sky_at(h);
        let dark = dark_at(h);
        self.paint_sky(sky, dark);
        self.paint_land(sky, dark);
        let tt = self.t;
        let wind = self.wind;
        let mut c = img.ctx();
        c.save();
        c.set_op(Op::Copy);
        c.draw_image(&self.sky_img, 0., 0.);
        c.restore();
        if dark > 0. {
            for &(x, y, k) in &self.stars {
                if ((tt * 2.).floor() as i64 + k as i64) % 6 != 0 || k == 0 {
                    r(
                        &mut c,
                        x,
                        y,
                        1.,
                        1.,
                        if k == 3 {
                            hex("#ffe9a4")
                        } else if dark > 0.5 {
                            hex("#f4f6fb")
                        } else {
                            hex("#8b93ad")
                        },
                    );
                }
            }
        }
        // the sun from 6:30 to 20:30, the moon the rest of the night, on an arc over the hills
        let day = (6.5..=20.5).contains(&h);
        let p = if day {
            (h - 6.5) / 14.
        } else {
            ((h + 24. - 20.5) % 24.) / 10.
        };
        let sx = jround(20. + p * (WF - 40.));
        let sy = jround(0.62 * HF - (p * PI).sin() * 0.5 * HF);
        if day {
            c.set_alpha(0.25);
            disc(&mut c, sx, sy, 10., hex("#fff6c0"));
            c.set_alpha(1.);
            disc(&mut c, sx, sy, 6.5, hex("#ffd34d"));
            disc(&mut c, sx, sy, 5.2, hex("#ffe27a"));
            disc(&mut c, sx - 1., sy - 1., 2.5, hex("#fff6c0"));
        } else {
            // the moon, a crescent: its disc less the one beside it
            for j in -6i32..=6 {
                for i in -6i32..=6 {
                    if i * i + j * j > 36 || (i - 3).pow(2) + (j + 2).pow(2) <= 30 {
                        continue;
                    }
                    let col = if (i == -4 && j == 1) || (i == -2 && j == -3) {
                        hex("#c2c9d9")
                    } else {
                        hex("#e8e5ef")
                    };
                    r(&mut c, sx + i as f64, sy + j as f64, 1., 1., col);
                }
            }
        }
        let light = mix(hex("#f4f6fb"), sky, 0.2 + dark * 0.62);
        let shade = mix(hex("#c2c9d9"), sky, 0.35 + dark * 0.5);
        for cl in &self.clouds {
            // puffs on a flat bottom, its underside in shade; going with the wind
            let (x, y) = (jround(cl.x), cl.y);
            let puffs: &[(f64, f64, f64)] = if cl.s {
                &[(0., 2., 4.), (6., -1., 6.), (13., 0., 5.), (19., 2., 4.)]
            } else {
                &[(0., 1., 5.), (7., -2., 7.), (15., 1., 5.)]
            };
            for &(px, py, pr) in puffs {
                disc(&mut c, x + px, y + py + 1., pr, shade);
            }
            for &(px, py, pr) in puffs {
                disc(&mut c, x + px, y + py, pr - 0.6, light);
            }
            r(
                &mut c,
                x - 3.,
                y + 4.,
                if cl.s { 25. } else { 22. },
                2.,
                shade,
            );
        }
        c.draw_image(&self.land_img, 0., 0.);
        let Some(d) = self.duel.as_ref() else { return };
        let land = &self.land;
        // the last shot of each, a dotted line in its colour until its next; the guns; the shell in the air
        for p in &d.p {
            c.set_alpha(0.5);
            for &(x, y) in &p.trail {
                r(&mut c, jround(x), jround(y), 1., 1., p.p.color);
            }
            c.set_alpha(1.);
        }
        for p in &d.p {
            pennant(&mut c, land, p, wind, tt);
            draw_gun(&mut c, land, p, wind, tt);
            let lying = p.body.is_some_and(|b| b.down);
            let out = p.leaving && p.walk != 0.;
            let bx = match p.body {
                Some(b) => b.x,
                None => p.x - 15. * p.f - if out { p.f * p.walk } else { 0. },
            };
            let won = if d.phase == Phase::Over && !p.dead {
                ((tt * 6.).floor() as i64 % 2) as f64
            } else {
                0.
            };
            if !(out && (bx < -6. || bx > WF + 6.)) {
                let y = match p.body {
                    Some(b) => b.y + 1.,
                    None => ground(land, bx),
                };
                gunner(&mut c, p, bx, y, lying, won);
            }
        }
        if let Some(s) = d.shell {
            // over the top of the picture, a mark on its edge where it is
            if s.y < 0. {
                r(&mut c, jround(s.x) - 1., 0., 3., 1., c::GOLD2);
            } else {
                let (x, y) = (jround(s.x), jround(s.y));
                inked(
                    &mut c,
                    &[
                        (x, y, c::STEEL1),
                        (x + 1., y, c::STEEL0),
                        (x, y + 1., c::STEEL0),
                        (x + 1., y + 1., c::INK2),
                    ],
                );
                r(&mut c, x, y, 1., 1., c::STEEL4);
            }
        }
        for b in &d.bits {
            let n = if b.t < 0.6 { 2. } else { 1. };
            r(&mut c, jround(b.x), jround(b.y), n, n, b.col);
        }
        for bm in &d.booms {
            // its smoke rising with the wind, a flash, a ball of fire
            let (t, rr) = (bm.t, bm.r);
            if t > 0.25 {
                c.set_alpha((0.7 * (1. - (t - 0.25) / 1.55)).max(0.));
                disc(
                    &mut c,
                    bm.x + wind * 10. * (t - 0.25),
                    bm.y - 3. - (t - 0.25) * 11.,
                    rr * 0.6 * (1. - (t - 0.25) / 2.5),
                    mix(hex("#8b93ad"), hex("#3b3a55"), dark),
                );
                c.set_alpha(1.);
            }
            if t < 0.5 {
                let k = (t / 0.12).min(1.);
                let fade = if t > 0.3 { 1. - (t - 0.3) / 0.2 } else { 1. };
                disc(
                    &mut c,
                    bm.x,
                    bm.y,
                    rr * (0.5 + 0.5 * k) * (0.6 + 0.4 * fade),
                    if t < 0.08 { c::WHITE } else { c::RED1 },
                );
                disc(
                    &mut c,
                    bm.x,
                    bm.y,
                    rr * (0.5 + 0.3 * k) * fade,
                    if t < 0.08 { c::WHITE } else { c::GOLD1 },
                );
                disc(&mut c, bm.x, bm.y, rr * 0.35 * fade, c::GOLD2);
            }
        }
    }

    // ---- for tests (window.__outside)
    pub fn state(&self) -> Option<State> {
        let d = self.duel.as_ref()?;
        let gs = |p: &Gun| GunState {
            id: p.p.id.clone(),
            x: p.x,
            y: self.ground(p.x),
            ang: p.ang,
            pow: p.pow,
            dead: p.dead,
            shots: p.shots,
        };
        Some(State {
            phase: d.phase,
            turn: d.turn,
            score: d.score,
            wind: self.wind,
            gone: d.gone,
            guns: [gs(&d.p[0]), gs(&d.p[1])],
        })
    }
    /// set your angle and power
    pub fn set_aim(&mut self, a: f64, p: f64) {
        if let Some(side) = self.you() {
            let g = &mut self.duel.as_mut().unwrap().p[side];
            g.ang = a;
            g.pow = p;
        }
    }
    /// the bots' aim with a perfect hand, for your gun: (angle, power)
    pub fn solve(&mut self) -> Option<(f64, f64)> {
        let side = self.you()?;
        let p = self.plan(side, 99, 0.);
        Some((p.a, p.p))
    }
    pub fn set_wind(&mut self, v: f64) {
        self.wind = v;
    }
}

fn ground(land: &Land, x: f64) -> f64 {
    land.g[clamp(jround(x), 0., WF - 1.) as usize] as f64
}
fn fly(wind: f64, s: &mut Shell) {
    s.vx += wind * PUSH * DT;
    s.vy += G * DT;
    s.x += s.vx * DT;
    s.y += s.vy * DT;
    s.age += DT;
}

fn hills(rng: &mut Rng, mountain: bool) -> Land {
    let (a, b, l, rt) = (
        rng.range(0., 7.),
        rng.range(0., 7.),
        rng.range(-11., 11.),
        rng.range(-11., 11.),
    );
    let xm = rng.range(0.41, 0.59) * WF;
    let hm = if mountain {
        rng.range(32., 58.)
    } else {
        rng.range(6., 14.)
    };
    let (wm, fa, fb) = (rng.range(22., 38.), rng.range(0., 7.), rng.range(0., 7.));
    let mut g = vec![0f32; W];
    let mut far = vec![0f32; W];
    for x in 0..W {
        let xf = x as f64;
        let u = xf / (WF - 1.);
        let lvl = 0.7 * HF + l + (rt - l) * u * u * (3. - 2. * u);
        g[x] = clamp(
            lvl + 5. * (xf / 17. + a).sin() + 3. * (xf / 8.5 + b).sin()
                - hm * (-((xf - xm) / wm).powi(2)).exp(),
            0.24 * HF,
            HF - 4.,
        ) as f32;
        far[x] = (0.58 * HF + 9. * (xf / 29. + fa).sin() + 5. * (xf / 11. + fb).sin()) as f32;
    }
    let mut pines: Vec<Pine> = Vec::new();
    let mut k = 0;
    while k < 40 && pines.len() < 6 {
        let x = jround(rng.range(4., WF - 5.));
        if !pines.iter().any(|p| (p.x - x).abs() < 14.) {
            pines.push(Pine {
                x,
                h: jround(rng.range(13., 20.)),
            });
        }
        k += 1;
    }
    Land {
        g,
        far,
        pines,
        scar: vec![0; W],
        v: (rng.next() * 9007199254740992.) as u64,
    }
}

fn gunner(c: &mut Ctx, p: &Gun, x: f64, y: f64, lying: bool, hop: f64) {
    // x: its middle, y: the ground under it
    let pl = &p.p;
    let w = if pl.human { pl.wears } else { None };
    let col = pl.color;
    let pal: Box<dyn Fn(u8) -> Option<Color>> = if pl.human {
        let hair = w.and_then(|w| w.hair).unwrap_or(c::INK2);
        let skin = w.map_or(hex("#f7c59a"), |w| w.skin);
        let shirt = w.map_or(col, |w| w.shirt);
        let shade = w.map_or(col, |w| w.shade);
        Box::new(move |ch| match ch {
            b'h' => Some(hair),
            b's' => Some(skin),
            b'k' => Some(c::INK),
            b'c' => Some(shirt),
            b'd' => Some(shade),
            b'l' => Some(c::INK2),
            b'b' => Some(c::INK),
            _ => None,
        })
    } else {
        Box::new(move |ch| match ch {
            b'a' | b'k' => Some(col),
            b'n' => Some(c::STEEL1),
            b'm' => Some(c::STEEL2),
            b'e' => Some(c::STEEL4),
            b'h' => Some(c::STEEL3),
            _ => None,
        })
    };
    let rows: &[&str] = if pl.human { &PERSON } else { &ROBOT };
    let (h, wd) = (rows.len() as i64, rows[0].len() as i64);
    let flip = p.f < 0.;
    let x0 = jround(x) - (if lying { h >> 1 } else { wd >> 1 }) as f64;
    let y0 = jround(y) - (if lying { wd } else { h }) as f64 - hop;
    let mut pts = Vec::new();
    grid(rows, &*pal, x0, y0, flip, lying, &mut pts);
    inked(c, &pts);
}

fn draw_gun(c: &mut Ctx, land: &Land, p: &Gun, wind: f64, tt: f64) {
    let gy = jround(ground(land, p.x));
    let f = p.f;
    let x = p.x - jround(f * p.kick * 2.);
    let py = ground(land, p.x) - 6.;
    if p.dead {
        // the wreck, smoking
        let mut pts = Vec::new();
        beam(
            x - 6.,
            gy - 2.,
            x + 5.,
            gy - 2.,
            2,
            hex("#2a2420"),
            None,
            &mut pts,
        );
        let pal = |ch: u8| match ch {
            b'r' => Some(hex("#2a2420")),
            b'b' | b's' => Some(hex("#3f3740")),
            b'x' => Some(hex("#5d6280")),
            _ => None,
        };
        grid(
            &WHEEL,
            &pal,
            x + 3. * f - 3.,
            gy - 6.,
            false,
            false,
            &mut pts,
        );
        inked(c, &pts);
        for k in 0..5 {
            let ph = (tt * 0.35 + k as f64 / 5.) % 1.;
            c.set_alpha(0.6 * (1. - ph));
            disc(
                c,
                x + ph * wind * 14. + (ph * 6. + k as f64).sin() * 2.,
                gy - 6. - ph * 26.,
                1.5 + ph * 2.5,
                hex("#5d6280"),
            );
            c.set_alpha(1.);
        }
        return;
    }
    let rad = p.ang * PI / 180.;
    let (dx, dy) = (f * rad.cos(), -rad.sin());
    let mut pts = Vec::new();
    beam(
        x,
        py + 2.,
        x - 12. * f,
        gy - 1.,
        2,
        hex("#734728"),
        Some(hex("#885c47")),
        &mut pts,
    ); // the trail
    beam(
        x - dx * 2.,
        py - dy * 2.,
        x + dx * 9.,
        py + dy * 9.,
        3,
        hex("#3f3740"),
        Some(hex("#6b6470")),
        &mut pts,
    ); // the barrel
    beam(
        x + dx * 9.,
        py + dy * 9.,
        x + dx * 11.,
        py + dy * 11.,
        2,
        c::INK2,
        None,
        &mut pts,
    ); // its muzzle
    let pal = |ch: u8| match ch {
        b'r' => Some(hex("#4a2e1c")),
        b'b' => Some(hex("#885c47")),
        b's' => Some(hex("#b07a4a")),
        b'x' => Some(c::STEEL3),
        _ => None,
    };
    grid(&WHEEL, &pal, x - 3., gy - 7., false, false, &mut pts); // the wheel
    inked(c, &pts);
    if p.flash > 0. {
        let (mx, my) = (jround(x + dx * 13.), jround(py + dy * 13.));
        disc(c, mx, my, 3., c::GOLD1);
        disc(c, mx, my, 1.6, c::GOLD2);
        r(c, mx, my, 1., 1., c::WHITE);
    }
}

/// on a pole behind the gunner, streaming with the wind
fn pennant(c: &mut Ctx, land: &Land, p: &Gun, wind: f64, tt: f64) {
    let gy = jround(ground(land, p.x));
    let x = p.x - 21. * p.f;
    let top = gy - 17.;
    let mut pts = Vec::new();
    beam(x, top, x, gy - 1., 1, c::STEEL2, None, &mut pts);
    let n = if wind.abs() < 0.08 {
        0.
    } else {
        2. + jround(wind.abs() * 8.)
    };
    let s = if wind > 0. {
        1.
    } else if wind < 0. {
        -1.
    } else {
        0.
    };
    let col = p.p.color;
    let lo = mix(col, Color::rgb(0), 0.25);
    if n == 0. {
        for j in 0..5 {
            pts.push((
                x + 1.,
                top + 1. + j as f64,
                if j % 2 == 1 { lo } else { col },
            )); // calm: it hangs down the pole
        }
    }
    let mut k = 0.;
    while k < n {
        let hh = 1f64.max(jround(5. * (1. - k / (n + 1.))));
        let wave = jround((tt * (4. + wind.abs() * 8.) - k * 0.9).sin() * (k / n) * 1.4);
        for j in 0..hh as i64 {
            let j = j as f64;
            pts.push((
                x + s * (k + 1.),
                top + 1. + j + wave + ((5. - hh) / 2.).floor(),
                if j == hh - 1. { lo } else { col },
            ));
        }
        k += 1.;
    }
    inked(c, &pts);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(id: &str) -> Player {
        Player::bot(id, id, css("#2fa087"))
    }
    fn you() -> Player {
        Player::you("you", css("#d97757"), None)
    }
    fn run(o: &mut Outside, secs: f64) {
        for _ in 0..(secs * 60.) as usize {
            o.tick(1. / 60.);
        }
    }

    #[test]
    fn the_sky_follows_the_hour() {
        assert_eq!(sky_at(12.), Color::rgb(0x7fb2e0));
        assert_eq!(sky_at(0.), Color::rgb(0x0d1430));
        assert_eq!(sky_at(23.), Color::rgb(0x0d1430));
        assert_eq!(sky_at(6.5), Color::rgb(0xd9775a));
        assert_eq!(dark_at(12.), 0.);
        assert_eq!(dark_at(7.), 0.5);
        assert_eq!(dark_at(23.), 1.);
        assert_eq!(hash("a"), 3826002220. / 4294967296.); // FNV-1a of "a", as the page has it
    }

    #[test]
    fn a_shell_flies_as_the_physics_say() {
        let mut o = Outside::new(1);
        o.open(you(), Some(bot("b")), None);
        o.set_wind(0.);
        let s0 = o.launch(0, 45., 50.);
        let (px, py) = o.pivot(0);
        assert!((s0.x - (px + 11. * (PI / 4.).cos())).abs() < 1e-9);
        assert!((s0.y - (py - 11. * (PI / 4.).sin())).abs() < 1e-9);
        assert!((s0.vx - 50. * VK * (PI / 4.).cos()).abs() < 1e-9);
        // without wind and ground: after t seconds the closed form, x0 + vx t, y0 + vy t + g t²/2 (the steps'
        // own error is a step's g·dt·t/2 at most)
        let mut s = s0;
        for _ in 0..120 {
            fly(0., &mut s);
        }
        assert!((s.x - (s0.x + s0.vx)).abs() < 1e-9);
        assert!((s.y - (s0.y + s0.vy + G / 2.)).abs() < G * DT);
        // the top of its arc: where vy turns
        let mut s = s0;
        while s.vy < 0. {
            fly(0., &mut s);
        }
        assert!((s.age - (-s0.vy / G)).abs() <= DT + 1e-9);
        // the wind pushes it along
        let mut a = s0;
        let mut b = s0;
        for _ in 0..120 {
            fly(1., &mut a);
            fly(-1., &mut b);
        }
        assert!(a.x > s.x.max(s0.x) && b.x < a.x);
        assert!((a.x - b.x - PUSH * (1. + DT)).abs() < 1e-6, "{}", a.x - b.x);
    }

    #[test]
    fn a_shell_on_a_gun_hits_it() {
        let mut o = Outside::new(2);
        o.open(you(), Some(bot("b")), None);
        let gx = o.state().unwrap().guns[1].x;
        let gy = o.state().unwrap().guns[1].y;
        let s = Shell {
            x: gx + 3.,
            y: gy - 4.,
            vx: 0.,
            vy: 1.,
            by: 0,
            age: 1.,
            whistle: None,
        };
        assert_eq!(
            o.down(&s),
            Some(Down::At {
                x: gx + 3.,
                y: gy - 4.,
                gun: Some(1)
            })
        );
        let s = Shell {
            x: gx + 6.,
            y: gy - 4.,
            ..s
        };
        assert_ne!(
            o.down(&s).and_then(|d| if let Down::At { gun, .. } = d {
                gun
            } else {
                None
            }),
            Some(1)
        );
        // its own gun only after a quarter of a second
        let s = Shell {
            x: o.state().unwrap().guns[0].x,
            y: o.state().unwrap().guns[0].y - 4.,
            vx: 0.,
            vy: 1.,
            by: 0,
            age: 0.1,
            whistle: None,
        };
        assert!(!matches!(o.down(&s), Some(Down::At { gun: Some(0), .. })));
        assert_eq!(o.down(&Shell { x: -7., ..s }), Some(Down::Out));
    }

    #[test]
    fn a_perfect_shot_wins_the_round() {
        let mut o = Outside::new(3);
        o.open(you(), Some(bot("b")), Some("window0".into()));
        let (a, p) = o.solve().expect("a shot");
        assert!(
            matches!(o.sim(0, a, p), Down::At { gun: Some(1), .. }),
            "{a} {p}"
        );
        o.set_aim(a, p);
        o.events();
        o.key_down(Key::Fire, false);
        assert_eq!(o.state().unwrap().phase, Phase::Fly);
        assert_eq!(o.playing().unwrap().fired[0], o.now());
        let mut heard = Vec::new();
        for _ in 0..60 * 8 {
            o.tick(1. / 60.);
            heard.extend(o.events());
            if o.state().unwrap().phase == Phase::Over {
                break;
            }
        }
        let st = o.state().unwrap();
        assert_eq!(st.phase, Phase::Over);
        assert_eq!(st.score, [1, 0]);
        assert!(st.guns[1].dead);
        let names: Vec<_> = heard
            .iter()
            .filter_map(|e| {
                if let Event::Sound { name, .. } = e {
                    Some(*name)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(names[0], "cannon");
        assert!(
            names.contains(&"whistle") && names.contains(&"wreck"),
            "{names:?}"
        );
        let wid = heard
            .iter()
            .find_map(|e| {
                if let Event::Sound {
                    name: "whistle",
                    id,
                    ..
                } = e
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            heard.contains(&Event::Stop(wid)),
            "the whistle stops where it lands"
        );
        run(&mut o, 4.);
        let st = o.state().unwrap();
        assert_eq!(st.phase, Phase::Aim);
        assert_eq!(st.turn, 1, "the loser starts the next round");
    }

    #[test]
    fn a_bot_aims_and_fires_on_its_turn() {
        let mut o = Outside::new(4);
        o.open(you(), Some(bot("b")), None);
        o.key_down(Key::Fire, false); // your turn first: 45° at 50
        run(&mut o, 6.);
        let st = o.state().unwrap();
        assert!(st.phase == Phase::Over || st.guns[1].shots == 1, "{st:?}");
        assert!(o.hud().unwrap().sides[0].name == "you");
    }

    #[test]
    fn held_keys_go_faster() {
        let mut o = Outside::new(5);
        o.open(you(), Some(bot("b")), None);
        o.key_down(Key::Up, false);
        assert_eq!(o.state().unwrap().guns[0].ang, 46.);
        o.tick(0.02);
        assert_eq!(o.state().unwrap().guns[0].ang, 46.);
        run(&mut o, 1.);
        let a1 = o.state().unwrap().guns[0].ang;
        assert!(a1 > 50. && a1 < 70., "{a1}");
        o.key_up(Key::Up);
        run(&mut o, 0.5);
        assert_eq!(o.state().unwrap().guns[0].ang, a1);
        assert_eq!(
            o.hud().unwrap().sides[0].say.as_deref(),
            Some(format!("angle {a1} · power 50").as_str())
        );
    }

    #[test]
    fn two_bots_duel_on_their_own_and_one_leaving_ends_it() {
        let mut o = Outside::new(6);
        o.open(bot("a"), Some(bot("b")), Some("window0".into()));
        let pl = o.playing().unwrap();
        assert!(
            pl.watch
                && pl.ids == Some(["a".into(), "b".into()])
                && pl.at.as_deref() == Some("window0")
        );
        run(&mut o, 40.);
        let st = o.state().unwrap();
        assert!(
            st.guns[0].shots + st.guns[1].shots >= 3 || st.score[0] + st.score[1] > 0,
            "{st:?}"
        );
        o.left("b");
        run(&mut o, 8.);
        assert_eq!(o.state().unwrap().phase, Phase::Left);
        assert!(o.playing().is_none());
        assert_eq!(o.hud().unwrap().sides[1].say.as_deref(), Some("left"));
    }

    #[test]
    fn pines_have_three_tiers_on_a_trunk() {
        let rows = pine_rows(14.);
        assert_eq!(rows.len(), 14);
        assert!(rows.iter().all(|r| r.len() == 10));
        assert_eq!(rows[0], "....ld....");
        assert_eq!(rows[11], ".lddddddd.");
        assert_eq!(rows[13], ".....t....");
    }

    /// the picture for a seed and an hour is always the same; tests/golden/*.png are what it was when
    /// ported (BURROW_BLESS=1 writes them again)
    #[test]
    fn frames_are_the_same_every_time() {
        for (name, hour, duel) in [
            ("noon", 12.25, true),
            ("night", 23.5, true),
            ("dusk", 19.6, false),
        ] {
            let mut o = Outside::new(42);
            if duel {
                o.open(you(), Some(bot("b")), None);
            }
            run(&mut o, 0.5);
            let mut im = Image::new(W as u32, H as u32);
            o.draw(&mut im, hour);
            assert_eq!(im.inked(), W * H);
            let mut again = Outside::new(42);
            if duel {
                again.open(you(), Some(bot("b")), None);
            }
            run(&mut again, 0.5);
            let mut im2 = Image::new(W as u32, H as u32);
            again.draw(&mut im2, hour);
            assert!(im == im2);
            let path = format!(
                "{}/tests/golden/outside-{name}.png",
                env!("CARGO_MANIFEST_DIR")
            );
            if std::env::var("BURROW_BLESS").is_ok() || !std::path::Path::new(&path).exists() {
                std::fs::create_dir_all(format!("{}/tests/golden", env!("CARGO_MANIFEST_DIR")))
                    .unwrap();
                im.save_png(&path).unwrap();
            }
            let want = Image::from_png(&std::fs::read(&path).unwrap()).unwrap();
            assert!(want == im, "{name} differs from {path}");
        }
    }

    /// the page itself, rendered with the same seed (Math.random as this crate's Rng) and hour in headless
    /// Chrome: tests/web/*.png, made by tests/web/outside.py. The canvas blends a translucent colour a
    /// unit off now and then (the sun's halo), so: within 2 a channel
    #[test]
    fn frames_are_the_pages() {
        for (name, hour, duel) in [
            ("noon", 12.25, true),
            ("night", 23.5, true),
            ("dusk", 19.6, false),
        ] {
            let mut o = Outside::new(42);
            o.open(you(), duel.then(|| bot("b")), None);
            o.tick(0.);
            let mut im = Image::new(W as u32, H as u32);
            o.draw(&mut im, hour);
            let path = format!(
                "{}/tests/web/outside-{name}.png",
                env!("CARGO_MANIFEST_DIR")
            );
            let web = Image::from_png(&std::fs::read(&path).unwrap()).unwrap();
            let off: Vec<(usize, [u8; 4], [u8; 4])> = (0..W * H)
                .filter(|&i| (0..3).any(|k| (web.px[i][k] as i32 - im.px[i][k] as i32).abs() > 2))
                .map(|i| (i, im.px[i], web.px[i]))
                .collect();
            if std::env::var("BURROW_DIFF").is_ok() {
                im.save_png(format!("/tmp/outside-{name}-rust.png")).ok();
                for (i, a, b) in off.iter().take(30) {
                    eprintln!("{name} ({}, {}) rust {a:?} web {b:?}", i % W, i / W);
                }
            }
            assert!(
                off.is_empty(),
                "{name}: {} pixels differ from the page",
                off.len()
            );
        }
    }
}
