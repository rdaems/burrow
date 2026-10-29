//! pair's avatars (web/pixel.js): a 20x20 portrait per participant, seeded, drawn after Super Atlas's
//! portraits: shaded shapes lit from the upper left, an ink outline. Humans get a face put together from
//! many parts (skin, hair, beards, glasses, clothes; deliberately no hats or head coverings, nothing
//! cultural or religious); agents a robot. Their own colour is the clothes or the robot's lights.
//! The same seed gives the same face as the web page and pair's server.

use pix::{Color, Image, css};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const N: i32 = 20;

/// pixel.js's colours
pub mod c {
    use pix::Color;
    pub const INK: Color = Color::rgb(0x140f1e);
    pub const INK2: Color = Color::rgb(0x2a2140);
    pub const STEEL0: Color = Color::rgb(0x3b3a55);
    pub const STEEL1: Color = Color::rgb(0x5d6280);
    pub const STEEL2: Color = Color::rgb(0x8b93ad);
    pub const STEEL3: Color = Color::rgb(0xc2c9d9);
    pub const STEEL4: Color = Color::rgb(0xf4f6fb);
    pub const WHITE: Color = Color::rgb(0xffffff);
    pub const GOLD0: Color = Color::rgb(0xb86a1e);
    pub const GOLD1: Color = Color::rgb(0xf2a531);
    pub const GOLD2: Color = Color::rgb(0xffe27a);
    pub const RED0: Color = Color::rgb(0x8e1f2a);
    pub const RED1: Color = Color::rgb(0xe8402a);
    pub const TISSUE2: Color = Color::rgb(0xf06b72);
    pub const TISSUE3: Color = Color::rgb(0xffb0a8);
    pub const BLUE0: Color = Color::rgb(0x1a2448);
    pub const BLUE1: Color = Color::rgb(0x2c4580);
    pub const BLUE2: Color = Color::rgb(0x4a76b8);
    pub const BLUE3: Color = Color::rgb(0x8fc0e8);
    pub const SCRUB1: Color = Color::rgb(0x1b6660);
    pub const SCRUB2: Color = Color::rgb(0x2fa087);
    pub const SCRUB3: Color = Color::rgb(0x7fdcb0);
    pub const PURPLE0: Color = Color::rgb(0x5a3f94);
    pub const PURPLE1: Color = Color::rgb(0x9a74d6);
    pub const GREEN0: Color = Color::rgb(0x2d8a3a);
    pub const GREEN1: Color = Color::rgb(0x5fcf4a);
}

const SKIN: [[u32; 3]; 6] = [
    [0xffe0bd, 0xf7c59a, 0xde8a6a],
    [0xf7c59a, 0xde8a6a, 0xa8554f],
    [0xde9d74, 0xc27a54, 0x8a4a35],
    [0xb8784e, 0x93573a, 0x6b3043],
    [0x8a5a3c, 0x6b4430, 0x4a2e2a],
    [0x5e3c2c, 0x4a2e22, 0x2e1c16],
];
const HAIR: [[u32; 2]; 10] = [
    [0x2a2140, 0x140f1e],
    [0x4a2e2a, 0x2e1c16],
    [0x7a4a35, 0x4a2e2a],
    [0xb07a4a, 0x7a4a35],
    [0xf2a531, 0xb86a1e],
    [0xa0402a, 0x6b2418],
    [0xc2c9d9, 0x8b93ad],
    [0xf4f6fb, 0xc2c9d9],
    [0x9a74d6, 0x5a3f94],
    [0xf06b72, 0xc23a55],
];

/// pixel.js's dice: FNV over the seed's characters, then a mixer (the same numbers as Math.imul's)
pub struct Rng(u32);
impl Rng {
    pub fn new(seed: &str) -> Rng {
        let mut h: u32 = 2166136261;
        for ch in seed.chars() {
            h = (h ^ ch as u32).wrapping_mul(16777619);
        }
        Rng(h)
    }
    pub fn next(&mut self) -> f64 {
        let mut h = self.0;
        h = (h ^ (h >> 15)).wrapping_mul(2246822507);
        h = (h ^ (h >> 13)).wrapping_mul(3266489909);
        h ^= h >> 16;
        self.0 = h;
        h as f64 / 4294967296.0
    }
    pub fn pick<T: Copy>(&mut self, a: &[T]) -> T {
        a[(self.next() * a.len() as f64).floor() as usize]
    }
    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }
}

/// JS's Math.round: halves go up
#[inline]
pub fn jround(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// a 20x20 grid of colours being drawn on
struct Cv {
    px: [[Option<Color>; 20]; 20],
}

#[derive(Clone, Copy)]
enum Ramp {
    One(Color),
    Two(Color, Color),
    Three(Color, Color, Color),
}

impl Cv {
    fn new() -> Cv {
        Cv { px: [[None; 20]; 20] }
    }
    fn set(&mut self, x: f64, y: f64, c: Color) {
        let (x, y) = (jround(x), jround(y));
        if (0..N).contains(&x) && (0..N).contains(&y) {
            self.px[y as usize][x as usize] = Some(c);
        }
    }
    fn ell(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, ramp: Ramp, clip: Option<&dyn Fn(i32, i32) -> bool>) {
        for y in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let dx = (x as f64 + 0.5 - cx) / rx;
                let dy = (y as f64 + 0.5 - cy) / ry;
                let q = dx * dx + dy * dy;
                if q > 1.0 || clip.is_some_and(|f| !f(x, y)) {
                    continue;
                }
                let l = -0.55 * dx - 0.65 * dy + if q > 0.72 { -0.35 } else { 0.0 };
                let col = match ramp {
                    Ramp::One(c) => c,
                    Ramp::Two(a, b) => {
                        if l > 0.25 {
                            a
                        } else {
                            b
                        }
                    }
                    Ramp::Three(a, b, c) => {
                        if l > 0.25 {
                            a
                        } else if l > -0.45 {
                            b
                        } else {
                            c
                        }
                    }
                };
                self.set(x as f64, y as f64, col);
            }
        }
    }
    fn rect(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: Color) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x as f64, y as f64, c);
            }
        }
    }
}

fn rgb(v: u32) -> Color {
    Color::rgb(v)
}
fn three(a: Color, b: Color, c: Color) -> Ramp {
    Ramp::Three(a, b, c)
}

fn human(cv: &mut Cv, r: &mut Rng, col: Color) {
    let skin = r.pick(&SKIN).map(rgb);
    let n = if r.chance(0.15) { 10 } else { 8 };
    let [hair, hair_d] = r.pick(&HAIR[..n]).map(rgb);
    let style = r.pick(&["short", "short", "long", "long", "bun", "ponytail", "afro", "curly", "spiky", "bald", "prof", "mohawk", "side"]);
    let grey = if style == "prof" { Some(r.pick(&[[c::STEEL4, c::STEEL3], [c::STEEL3, c::STEEL2]])) } else { None };
    let outfit = r.pick(&["tee", "tee", "coat", "suit", "hoodie", "sweater"]);
    let (c0, c1) = (col, col.shade(0.62));
    let hr = three(hair, hair, hair_d);
    // behind the head: big hair
    if style == "afro" {
        cv.ell(10., 7.5, 7.6, 6.8, hr, None);
    }
    if style == "long" {
        cv.ell(10., 8., 6.6, 5.5, hr, None);
        cv.rect(3, 9, 3, 7, hair_d);
        cv.rect(14, 9, 3, 7, hair_d);
    }
    // shoulders and clothes
    match outfit {
        "coat" => {
            cv.ell(10., 21., 9.5, 5.5, three(c::WHITE, c::STEEL4, c::STEEL3), None);
            cv.rect(9, 16, 2, 4, c0);
            cv.set(9., 16., c1);
            cv.set(10., 19., c1);
            cv.set(7., 16., c::STEEL2);
            cv.set(8., 17., c::STEEL2);
            cv.set(12., 16., c::STEEL2);
            cv.set(11., 17., c::STEEL2);
        }
        "suit" => {
            cv.ell(10., 21., 9.5, 5.5, three(c::STEEL1, c::STEEL0, c::INK2), None);
            cv.rect(8, 16, 4, 4, c::WHITE);
            cv.rect(9, 16, 2, 4, c0);
            cv.set(10., 17., c1);
        }
        "hoodie" => {
            cv.ell(10., 21., 9.5, 5.5, three(c0, c0, c1), None);
            cv.ell(10., 16.5, 5., 2., Ramp::One(c1), None);
            cv.set(8., 18., c::STEEL4);
            cv.set(12., 18., c::STEEL4);
        }
        "sweater" => {
            cv.ell(10., 21., 9.5, 5.5, three(c0, c0, c1), None);
            cv.rect(7, 16, 6, 1, c::WHITE);
            cv.set(6., 17., c::WHITE);
            cv.set(13., 17., c::WHITE);
        }
        _ => cv.ell(10., 21., 9.5, 5.5, three(c0, c0, c1), None),
    }
    // neck, ears, head
    cv.rect(8, 13, 4, 3, skin[2]);
    cv.ell(4.6, 10., 1.4, 1.8, three(skin[1], skin[1], skin[2]), None);
    cv.ell(15.4, 10., 1.4, 1.8, three(skin[1], skin[1], skin[2]), None);
    cv.ell(10., 9.6, 5.4, 6.1, three(skin[0], skin[1], skin[2]), None);
    // hair on top
    let cap = |cv: &mut Cv, y_top: f64, clip_y: i32| cv.ell(10., y_top, 5.9, 4.4, hr, Some(&move |_x, y| y <= clip_y));
    if matches!(style, "short" | "long" | "side" | "spiky" | "ponytail" | "bun") {
        cap(cv, 6., 6);
        cv.set(4.6, 7., hair);
        cv.set(15.4, 7., hair);
        cv.set(4.6, 8., hair_d);
        cv.set(15.4, 8., hair_d);
        if style == "side" {
            for x in 5..11 {
                cv.set(x as f64, 7., hair);
            }
            cv.set(5., 8., hair);
        }
        if style == "spiky" {
            for x in (5..=15).step_by(2) {
                cv.set(x as f64, 1., hair);
                cv.set(x as f64, 2., hair);
            }
        }
        if style == "bun" {
            cv.ell(10., 1.6, 2.4, 1.8, hr, None);
        }
        if style == "ponytail" {
            cv.ell(16.6, 9., 1.6, 4., hr, None);
        }
    }
    if style == "afro" {
        cap(cv, 6., 5);
    }
    if style == "curly" {
        for (x, y) in [(6., 4.), (8.5, 2.8), (11.5, 2.8), (14., 4.), (5., 6.5), (15., 6.5), (10., 3.5)] {
            cv.ell(x, y, 2., 1.9, hr, None);
        }
    }
    if style == "mohawk" {
        cv.rect(9, 1, 3, 5, hair);
        cv.rect(9, 5, 3, 1, hair_d);
    }
    if let (Some(g), "prof") = (grey, style) {
        cv.ell(4.6, 9., 1.8, 2.8, Ramp::Two(g[0], g[1]), None);
        cv.ell(15.4, 9., 1.8, 2.8, Ramp::Two(g[0], g[1]), None);
        cv.set(9., 4., skin[0]);
        cv.set(10., 4., skin[0]);
    }
    if style == "bald" && r.chance(0.5) {
        cv.set(8., 5., skin[0]);
        cv.set(9., 5., skin[0]);
    }
    // face: brows, eyes, nose, mouth
    let brow = match grey {
        Some(g) => g[0],
        None if style == "bald" => skin[2],
        None => hair_d,
    };
    let ey = 10.;
    if r.chance(0.6) {
        for x in [7., 8., 12., 13.] {
            cv.set(x, ey - 3., brow);
        }
    }
    // eyes: a 2x2 pupil with a catchlight, readable on every skin
    for ex in [7., 12.] {
        cv.set(ex, ey, c::INK);
        cv.set(ex + 1., ey, c::INK);
        cv.set(ex, ey - 1., c::STEEL4);
        cv.set(ex + 1., ey - 1., c::INK);
    }
    cv.set(10., ey + 2., skin[2]);
    let my = 14.;
    let mouth = r.pick(&["smile", "smile", "flat", "open", "grin"]);
    match mouth {
        "smile" => {
            for x in [9., 10., 11.] {
                cv.set(x, my, c::RED0);
            }
            cv.set(8., my - 1., c::RED0);
            cv.set(12., my - 1., c::RED0);
        }
        "flat" => {
            for x in [9., 10., 11.] {
                cv.set(x, my, skin[2]);
            }
        }
        "open" => {
            cv.set(10., my, c::RED0);
            cv.set(10., my - 1., c::RED0);
        }
        _ => {
            for x in 8..=12 {
                cv.set(x as f64, my, c::WHITE);
            }
            cv.set(8., my - 1., c::RED0);
            cv.set(12., my - 1., c::RED0);
        }
    }
    if r.chance(0.25) {
        cv.set(6., 12., c::TISSUE2);
        cv.set(14., 12., c::TISSUE2);
    }
    if r.chance(0.15) {
        cv.set(7., 12., skin[2]);
        cv.set(13., 11., skin[2]);
        cv.set(12., 12., skin[2]);
    }
    // facial hair
    let beard = if grey.is_some() {
        r.pick(&["stache", "beard", "none"])
    } else if r.chance(0.35) {
        r.pick(&["beard", "stache", "goatee", "stubble"])
    } else {
        "none"
    };
    let bc = grey.unwrap_or([hair, hair_d]);
    if beard == "beard" {
        cv.ell(10., 13.4, 5., 3.2, three(bc[0], bc[0], bc[1]), Some(&|_x, y| y >= 12));
        for x in [9., 10., 11.] {
            cv.set(x, my, c::RED0);
        }
    }
    if beard == "stache" || beard == "beard" {
        for x in 8..=12 {
            cv.set(x as f64, 13., bc[0]);
        }
        cv.set(7., 14., bc[1]);
        cv.set(13., 14., bc[1]);
    }
    if beard == "goatee" {
        cv.set(10., 15., bc[0]);
        cv.set(9., 15., bc[0]);
        cv.set(11., 15., bc[0]);
        cv.set(10., 16., bc[1]);
    }
    if beard == "stubble" {
        for x in (6..=14).step_by(2) {
            cv.set(x as f64, 14. + if x % 4 != 0 { 0. } else { 1. }, skin[2]);
        }
    }
    // glasses
    let specs = if grey.is_some() {
        r.pick(&["round", "none"])
    } else if r.chance(0.18) {
        r.pick(&["round", "square", "shades"])
    } else {
        "none"
    };
    if specs == "round" || specs == "square" {
        let ey = ey as i32;
        for ex in [7, 12] {
            for y in ey - 2..=ey + 1 {
                for x in ex - 1..=ex + 2 {
                    let edge = y == ey - 2 || y == ey + 1 || x == ex - 1 || x == ex + 2;
                    let corner = (y == ey - 2 || y == ey + 1) && (x == ex - 1 || x == ex + 2);
                    if edge && !(specs == "round" && corner) {
                        cv.set(x as f64, y as f64, c::INK);
                    }
                }
            }
        }
        cv.set(10., ey as f64 - 1., c::INK);
    }
    if specs == "shades" {
        cv.rect(6, ey as i32 - 1, 9, 2, c::INK);
        cv.set(7., ey - 1., c::STEEL1);
        cv.set(12., ey - 1., c::STEEL1);
        cv.set(10., ey, skin[1]);
    }
    if style != "long" && r.chance(0.2) {
        cv.set(4.6, 12., c::GOLD2);
        cv.set(15.4, 12., c::GOLD2);
    }
}

/// a robot's parts, as robot() picks them (in its order), for a model of it (the 3d view)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RobotParts {
    pub top: i32,
    pub ant: &'static str,
    pub eyes: &'static str,
    pub teeth: bool,
}

pub fn robot_parts(seed: &str) -> RobotParts {
    let mut r = Rng::new(seed);
    let top = r.pick(&[3, 4]);
    let ant = r.pick(&["one", "two", "dish"]);
    let eyes = r.pick(&["dots", "visor", "cyclops", "happy"]);
    RobotParts { top, ant, eyes, teeth: r.next() < 0.5 }
}

fn robot(cv: &mut Cv, r: &mut Rng, col: Color) {
    let (light, light_d) = (col, col.shade(0.62));
    // drawn on the old 16 grid, moved into the 20
    let mut g = |x: i32, y: i32, c: Color| cv.set((x + 2) as f64, (y + 4) as f64, c);
    for y in 13..16 {
        for x in 3..13 {
            g(x, y, if x == 3 || y == 13 { c::STEEL3 } else if x > 10 { c::STEEL1 } else { c::STEEL2 });
        }
    }
    g(7, 14, light);
    g(8, 14, light);
    for x in 6..10 {
        g(x, 12, c::STEEL1);
    }
    let top = r.pick(&[3, 4]);
    for y in top..12 {
        for x in 3..13 {
            g(x, y, if y == top || x == 3 { c::STEEL4 } else if y == 11 || x == 12 { c::STEEL1 } else { c::STEEL3 });
        }
    }
    match r.pick(&["one", "two", "dish"]) {
        "one" => {
            g(8, top - 1, c::STEEL2);
            g(8, top - 2, c::STEEL2);
            g(8, top - 3, light);
        }
        "two" => {
            g(5, top - 1, c::STEEL2);
            g(10, top - 1, c::STEEL2);
            g(5, top - 2, light);
            g(10, top - 2, light);
        }
        _ => {
            g(7, top - 1, c::STEEL2);
            g(8, top - 1, c::STEEL2);
            g(6, top - 2, c::STEEL3);
            g(9, top - 2, c::STEEL3);
            g(7, top - 2, light);
            g(8, top - 2, light);
        }
    }
    for y in 6..9 {
        g(2, y, c::STEEL1);
        g(13, y, c::STEEL1);
    }
    for y in top + 2..top + 6 {
        for x in 5..11 {
            g(x, y, c::BLUE0);
        }
    }
    let ey = top + 3;
    match r.pick(&["dots", "visor", "cyclops", "happy"]) {
        "dots" => {
            g(6, ey, light);
            g(9, ey, light);
            g(6, ey + 1, light_d);
            g(9, ey + 1, light_d);
        }
        "visor" => {
            for x in 5..11 {
                g(x, ey, if x % 2 != 0 { light } else { light_d });
            }
        }
        "cyclops" => {
            g(7, ey, light);
            g(8, ey, light);
            g(7, ey + 1, light);
            g(8, ey + 1, light_d);
        }
        _ => {
            g(6, ey + 1, light);
            g(5, ey, light);
            g(7, ey, light);
            g(9, ey + 1, light);
            g(8, ey, light);
            g(10, ey, light);
        }
    }
    if r.next() < 0.5 {
        for x in 6..10 {
            g(x, 10, if x % 2 != 0 { c::STEEL1 } else { c::INK2 });
        }
    } else {
        for x in 6..10 {
            g(x, 10, c::STEEL1);
        }
    }
}

thread_local! {
    static CACHE: RefCell<HashMap<(String, bool, Color), Rc<Image>>> = RefCell::new(HashMap::new());
}

/// the portrait: `agent` a robot, else a person; `color` their clothes or lights
pub fn avatar(seed: &str, agent: bool, color: Option<Color>) -> Rc<Image> {
    let color = color.unwrap_or(c::STEEL2);
    let key = (seed.to_string(), agent, color);
    if let Some(im) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return im;
    }
    let mut cv = Cv::new();
    let mut r = Rng::new(seed);
    if agent {
        robot(&mut cv, &mut r, color);
    } else {
        human(&mut cv, &mut r, color);
    }
    let mut im = Image::new(N as u32, N as u32);
    let has = |x: i32, y: i32| (0..N).contains(&x) && (0..N).contains(&y) && cv.px[y as usize][x as usize].is_some();
    for y in 0..N {
        for x in 0..N {
            match cv.px[y as usize][x as usize] {
                Some(c) => im.set(x, y, c.0),
                None if has(x + 1, y) || has(x - 1, y) || has(x, y + 1) || has(x, y - 1) => im.set(x, y, c::INK.0),
                None => {}
            }
        }
    }
    let im = Rc::new(im);
    CACHE.with(|c| c.borrow_mut().insert(key, im.clone()));
    im
}

/// the skin a human's face was drawn with (skin is the first thing human() picks), for the hands of a figure
pub fn skin(seed: &str) -> Color {
    rgb(Rng::new(seed).pick(&SKIN)[1])
}

/// a colour given as CSS, or none
pub fn color_of(s: &str) -> Option<Color> {
    let c = css(s);
    (!c.is_none()).then_some(c)
}
