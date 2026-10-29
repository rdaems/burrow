//! The room's painters (room.js's `draw*`): each piece of furniture, the props and the image effects,
//! drawn into a canvas at its place. They are pure: what room.js read from the world or the app's
//! state (the time, the usage, who is lit, the chess board) comes in as parameters; what is under the
//! pointer is the caller's (it knows where the pointer is). Names as in room.js, in snake case.

mod desk;
mod dog;
mod dream;
mod effects;
mod figures;
mod fire;
mod kind;
mod pieces;
mod wall;

pub use desk::*;
pub use dog::*;
pub use dream::*;
pub use effects::*;
pub use figures::*;
pub use fire::*;
pub use kind::*;
pub use pieces::*;
pub use wall::*;

use crate::sprites;
use pix::{Color, Ctx, Image};

/// sprites.js's palette by its letters (`SP.F` in room.js is `SP::F` here)
#[allow(non_upper_case_globals, non_snake_case)]
pub mod SP {
    use pix::Color;
    pub const s: Color = Color::rgba(0, 0, 0, 51);
    pub const h: Color = Color::rgb(0xcfa86d);
    pub const w: Color = Color::rgb(0xb38857);
    pub const d: Color = Color::rgb(0x885c47);
    pub const k: Color = Color::rgb(0x532e3d);
    pub const A: Color = Color::rgb(0xffffff);
    pub const B: Color = Color::rgb(0xe1e3e9);
    pub const C: Color = Color::rgb(0xb5bfc7);
    pub const D: Color = Color::rgb(0x757b7c);
    pub const E: Color = Color::rgb(0x595e60);
    pub const F: Color = Color::rgb(0x3f3740);
    pub const G: Color = Color::rgb(0x394449);
    pub const x: Color = Color::rgb(0x140f1e);
    pub const r: Color = Color::rgb(0xeb4c3f);
    pub const R: Color = Color::rgb(0x8c323a);
    pub const o: Color = Color::rgb(0xededed);
    pub const g: Color = Color::rgb(0x60c677);
    pub const y: Color = Color::rgb(0xf2a531);
    pub const Y: Color = Color::rgb(0xffe27a);
    pub const v: Color = Color::rgb(0x4a76b8);
    pub const M: Color = Color::rgb(0x1b6660);
    pub const m: Color = Color::rgb(0x2fa087);
}
pub use crate::avatar::c as C;

pub const INK: Color = C::INK;
pub const TERRACOTTA: Color = Color::rgb(0xd97757);
pub const RED: Color = Color::rgb(0xff6b5a);
pub const OPEN: Color = Color::rgb(0x8a4a3a);
pub const YOU: Color = TERRACOTTA;
/// what the pointer is on: the tiles' pastel gold, on the floor as well
pub const GOLD: Color = Color::rgb(0xf6c784);
pub const SHADOW: Color = SP::s;
pub const WALLC: Color = Color::rgb(0x28374b);
pub const BASEBOARD: Color = Color::rgb(0x1b2433);
pub const PLANK: Color = Color::rgb(0x734728);
pub const SEAM: Color = Color::rgb(0x6a4224);
pub const PROJECT: [Color; 8] = [
    Color::rgb(0x4a76b8),
    Color::rgb(0xe8402a),
    Color::rgb(0x2fa087),
    Color::rgb(0xf2a531),
    Color::rgb(0x9a74d6),
    Color::rgb(0x5fcf4a),
    Color::rgb(0xf06b72),
    Color::rgb(0x8fc0e8),
];
pub const WALL: i32 = 26;
pub const SEAT: i32 = 18;
pub const ADD_W: i32 = 30;
pub const ROW: i32 = 46;
pub const LAP: i32 = 13;
pub const DOOR: i32 = 20;
pub const NAME: usize = 8;

/// a machine's paint: its door and its desks (the wood is this computer's)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paint {
    pub hi: Color,
    pub top: Color,
    pub d: Color,
    pub k: Color,
    pub plate: Color,
}
impl Default for Paint {
    fn default() -> Paint {
        WOOD
    }
}
pub const WOOD: Paint = Paint { hi: SP::h, top: SP::w, d: SP::d, k: SP::k, plate: Color::rgb(0x2e1f28) };
const fn paint(hi: u32, top: u32, d: u32, k: u32, plate: u32) -> Paint {
    Paint { hi: Color::rgb(hi), top: Color::rgb(top), d: Color::rgb(d), k: Color::rgb(k), plate: Color::rgb(plate) }
}
/// the paint of another machine's door and desks, in the order of Claude Code's sshConfigs
pub const PAINT: [Paint; 8] = [
    paint(0x8fd0c4, 0x5fa89c, 0x3f7d74, 0x1f3f42, 0x142a2c),
    paint(0xf4f1ea, 0xd8d2c6, 0xa59d90, 0x4a4250, 0x2a2530),
    paint(0xa3c0e4, 0x7397c4, 0x4f6e9a, 0x263652, 0x182236),
    paint(0xe8a493, 0xc97766, 0x964c40, 0x4f2430, 0x2e151c),
    paint(0xf2d27a, 0xd6aa45, 0xa07c2c, 0x4f3a1e, 0x2e2214), // mustard
    paint(0xc9b3e6, 0xa185c9, 0x76599e, 0x3a2a52, 0x221830), // lilac
    paint(0xb9d39a, 0x8fae6c, 0x62804a, 0x2f4027, 0x1b2617), // sage
    paint(0x8a93a3, 0x646c7b, 0x454b57, 0x22262e, 0x16181d), // charcoal
];
/// a paint by its index in the doors' panel: -1 the wood
pub fn paint_at(i: i32) -> Paint {
    if i < 0 { WOOD } else { PAINT[i as usize % PAINT.len()] }
}

/// JS's `Math.round`: halves go up (Rust's `round` takes -0.5 away from zero)
#[inline]
pub fn js_round(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}
/// room.js's FNV-1a over the string's code points
pub fn fnv(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for ch in s.chars() {
        h = (h ^ ch as u32).wrapping_mul(16777619);
    }
    h
}
pub fn project_color(unit: &str) -> Color {
    PROJECT[fnv(unit) as usize % PROJECT.len()]
}

/// `ctx.fillRect` in whole dots (a negative size goes the other way, as the canvas does)
#[inline]
pub fn rect(ctx: &mut Ctx, x: i32, y: i32, w: i32, h: i32, c: Color) {
    let (x, w) = if w < 0 { (x + w, -w) } else { (x, w) };
    let (y, h) = if h < 0 { (y + h, -h) } else { (y, h) };
    ctx.rect(x, y, w, h, c);
}
/// a filled box with a one-dot edge round it
pub fn box_(ctx: &mut Ctx, x: i32, y: i32, w: i32, h: i32, fill: Color, edge: Color) {
    rect(ctx, x - 1, y - 1, w + 2, h + 2, edge);
    rect(ctx, x, y, w, h, fill);
}
/// a sprite (sprites.js) with its top left at x, y; flipped, it faces the other way in the same place
pub fn put(ctx: &mut Ctx, im: &Image, x: i32, y: i32, flip: bool) {
    if flip {
        ctx.save();
        ctx.translate((x + im.w as i32) as f64, y as f64);
        ctx.scale(-1., 1.);
        ctx.draw_image(im, 0., 0.);
        ctx.restore();
    } else {
        ctx.draw_image(im, x as f64, y as f64);
    }
}
/// a sprite by its name; nothing if there is none
pub fn put_named(ctx: &mut Ctx, name: &str, x: i32, y: i32) {
    if let Some(im) = sprites::get(name) {
        put(ctx, &im, x, y, false);
    }
}

/// a block seen from above at a slant, as Pixel Agents draws its furniture: a top `top` dots deep
/// and a front `front` dots high, outlined, its back and front edges lit, a shadow under it
pub fn slab(ctx: &mut Ctx, x: i32, y: i32, w: i32, top: i32, front: i32, p: &Paint) {
    rect(ctx, x, y, w, top + front, p.k);
    rect(ctx, x + 1, y, w - 2, 1, p.d);
    rect(ctx, x + 1, y + 1, w - 2, top - 1, p.top);
    rect(ctx, x + 1, y + top - 1, w - 2, 1, p.hi);
    rect(ctx, x + 1, y + top + 1, w - 2, front - 2, p.d);
    rect(ctx, x + 1, y + top + front, w - 2, 1, SHADOW);
}
/// a name plate on a cabinet's front: seven dots high, for one line of lettering (only `k` and
/// `plate` of the paint are used)
pub fn plate(ctx: &mut Ctx, x: i32, y: i32, w: i32, lit: bool, p: &Paint) {
    rect(ctx, x, y, w, 7, p.k);
    rect(ctx, x + 1, y + 1, w - 2, 5, if lit { C::STEEL1 } else { p.plate });
}

/// a piece's kind from its key: plant0 is a plant, door:<machine> a door
pub fn kind_of(key: &str) -> &str {
    if key.starts_with("door:") {
        return "door";
    }
    key.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// the fire pit's footprint, drawn to scale from its 32 x 28 original
pub struct FireSize {
    pub w: i32,
    pub h: i32,
    pub s: f64,
}
pub const FIRE: FireSize = FireSize { w: 48, h: 42, s: 1.5 };
pub const fn sb_h() -> i32 {
    14
}
pub const fn sb_w() -> i32 {
    10
}

/// what each piece covers, from where it stands (`unit_w`: a desk's width, for a desk)
pub fn foot(key: &str, x: f64, y: f64, unit_w: Option<f64>) -> [f64; 4] {
    if let Some(w) = unit_w {
        return [x - 2., y - 35., x + w + 2., y + 2.];
    }
    let a = ADD_W as f64;
    match kind_of(key) {
        "add" => [x - 2., y - 16., x + a + 2., y + 2.],
        "meet" => [x - 28., y - 20., x + 28., y + 4.],
        "pong" => [x - 38., y - 23., x + 38., y + 8.],
        "chess" => [x - 19., y - 18., x + 19., y + 2.],
        "coffee" => [x, y - 22., x + 20., y + 1.],
        "cooler" => [x - 5., y - 26., x + 5., y + 1.],
        "printer" => [x - 7., y - 15., x + 7., y + 1.],
        "armchair" => [x - 8., y - 14., x + 8., y + 1.],
        "board" => [x - 16., y - 26., x + 16., y + 1.],
        "window" => [x, y, x + 21., y + 18.],
        "clock" => [x, y, x + 17., y + 17.],
        "fire" => [x, y - FIRE.h as f64, x + FIRE.w as f64, y + 1.],
        "closet" => [x, y, x + 22., y + 24.],
        "settings" => [x, y, x + sb_w() as f64, y + sb_h() as f64],
        "manager" => [x - 2., y - 36., x + 48., y + 2.],
        "pair" => [x - 13., y - 20., x + 13., y + 1.],
        "booth" => [x - 9., y - 32., x + 9., y + 2.],
        "door" => [x, y, x + 16., y + 20.],
        "neuralyzer" => [x - 7., y - 19., x + 7., y + 2.],
        "doghouse" => [x - 12., y - 21., x + 12., y + 2.],
        "radio" => [x - 7., y - 21., x + 7., y + 1.],
        _ => [x - 8., y - 31., x + 8., y + 1.], // a plant
    }
}
