// The bots and the lettering, after pair's office (app/pixel.js, app/floor.js): a robot's head from its
// session id, a body in its project's colour, and Super Atlas's bitmap font. Everything is drawn in
// "dots" (one logical pixel of the room), which the room scales to screen pixels.

use egui::Color32;
use std::collections::HashMap;

pub const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub const INK: Color32 = hex(0x140f1e);
pub const INK2: Color32 = hex(0x2a2140);
pub const STEEL0: Color32 = hex(0x3b3a55);
pub const STEEL1: Color32 = hex(0x5d6280);
pub const STEEL2: Color32 = hex(0x8b93ad);
pub const STEEL3: Color32 = hex(0xc2c9d9);
pub const STEEL4: Color32 = hex(0xf4f6fb);
pub const BLUE0: Color32 = hex(0x1a2448);
pub const NIGHT: Color32 = hex(0x0d0a16);
pub const TERRACOTTA: Color32 = hex(0xd97757);

// a project's colour: the clothes and robot lights of pair's palette
pub const PROJECT: [Color32; 8] = [
    hex(0x4a76b8),
    hex(0xe8402a),
    hex(0x2fa087),
    hex(0xf2a531),
    hex(0x9a74d6),
    hex(0x5fcf4a),
    hex(0xf06b72),
    hex(0x8fc0e8),
];

pub fn shade(c: Color32, k: f32) -> Color32 {
    let f = |v: u8| ((v as f32 * k).round()).min(255.0) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

pub fn fnv(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for ch in s.chars() {
        h = (h ^ ch as u32).wrapping_mul(16777619);
    }
    h
}

// pixel.js's rng: FNV-1a of the seed, then a murmur-style finaliser per draw
struct Rng(u32);
impl Rng {
    fn new(seed: &str) -> Self {
        Rng(fnv(seed))
    }
    fn next(&mut self) -> f32 {
        let mut h = self.0;
        h = (h ^ (h >> 15)).wrapping_mul(2246822507);
        h = (h ^ (h >> 13)).wrapping_mul(3266489909);
        h ^= h >> 16;
        self.0 = h;
        h as f32 / 4294967296.0
    }
    fn pick<T: Copy>(&mut self, a: &[T]) -> T {
        a[((self.next() * a.len() as f32) as usize).min(a.len() - 1)]
    }
}

pub const SW: i32 = 22; // a figure's sprite, outline included
pub const SH: i32 = 29;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Arms {
    Down,
    Swing1,
    Swing2,
    Type1,
    Type2,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Legs {
    Stand,
    Walk1,
    Walk2,
    Sit,
}

pub type Sprite = Vec<(i8, i8, Color32)>;

#[derive(Default)]
pub struct Sprites {
    cache: HashMap<(String, Color32, Arms, Legs), std::rc::Rc<Sprite>>,
}

impl Sprites {
    pub fn get(&mut self, seed: &str, col: Color32, arms: Arms, legs: Legs) -> std::rc::Rc<Sprite> {
        let key = (seed.to_string(), col, arms, legs);
        self.cache.entry(key).or_insert_with(|| std::rc::Rc::new(robot(seed, col, arms, legs))).clone()
    }
}

// the head of pixel.js's robot() on its 20-grid, the body of floor.js's body() for an agent
fn robot(seed: &str, light: Color32, arms: Arms, legs: Legs) -> Sprite {
    let mut px: HashMap<(i32, i32), Color32> = HashMap::new();
    let mut r = Rng::new(seed);
    let light_d = shade(light, 0.62);
    {
        let mut g = |x: i32, y: i32, c: Color32| {
            px.insert((x + 2, y + 4), c);
        };
        for y in 13..16 {
            for x in 3..13 {
                g(x, y, if x == 3 || y == 13 { STEEL3 } else if x > 10 { STEEL1 } else { STEEL2 });
            }
        }
        g(7, 14, light);
        g(8, 14, light);
        for x in 6..10 {
            g(x, 12, STEEL1);
        }
        let top = r.pick(&[3, 4]);
        for y in top..12 {
            for x in 3..13 {
                let c = if y == top || x == 3 {
                    STEEL4
                } else if y == 11 || x == 12 {
                    STEEL1
                } else {
                    STEEL3
                };
                g(x, y, c);
            }
        }
        match r.pick(&[0, 1, 2]) {
            0 => {
                g(8, top - 1, STEEL2);
                g(8, top - 2, STEEL2);
                g(8, top - 3, light);
            }
            1 => {
                g(5, top - 1, STEEL2);
                g(10, top - 1, STEEL2);
                g(5, top - 2, light);
                g(10, top - 2, light);
            }
            _ => {
                g(7, top - 1, STEEL2);
                g(8, top - 1, STEEL2);
                g(6, top - 2, STEEL3);
                g(9, top - 2, STEEL3);
                g(7, top - 2, light);
                g(8, top - 2, light);
            }
        }
        for y in 6..9 {
            g(2, y, STEEL1);
            g(13, y, STEEL1);
        }
        for y in top + 2..top + 6 {
            for x in 5..11 {
                g(x, y, BLUE0);
            }
        }
        let ey = top + 3;
        match r.pick(&[0, 1, 2, 3]) {
            0 => {
                g(6, ey, light);
                g(9, ey, light);
                g(6, ey + 1, light_d);
                g(9, ey + 1, light_d);
            }
            1 => {
                for x in 5..11 {
                    g(x, ey, if x % 2 == 1 { light } else { light_d });
                }
            }
            2 => {
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
        let grille = r.next() < 0.5;
        for x in 6..10 {
            g(x, 10, if grille && x % 2 == 1 { INK2 } else { STEEL1 });
        }
    }
    // the portrait down to the neck, shifted by one for the outline, then the body
    let mut out: HashMap<(i32, i32), Color32> = HashMap::new();
    for ((x, y), c) in px {
        if y <= 16 {
            out.insert((x + 1, y + 1), c);
        }
    }
    let mut set = |x: i32, y: i32, c: Color32| {
        out.insert((x + 1, y + 1), c);
    };
    let rect = |set: &mut dyn FnMut(i32, i32, Color32), x: i32, y: i32, w: i32, h: i32, c: Color32| {
        for j in 0..h {
            for i in 0..w {
                set(x + i, y + j, c);
            }
        }
    };
    let (c0, c1, skin, leg) = (STEEL3, STEEL1, STEEL3, STEEL1);
    let t0 = 17;
    rect(&mut set, 6, t0, 8, 22 - t0, c0);
    rect(&mut set, 12, t0, 2, 22 - t0, c1);
    set(9, 19, light);
    set(10, 19, light);
    set(9, 20, light_d);
    let arm = |set: &mut dyn FnMut(i32, i32, Color32), x: i32, y0: i32, y1: i32, hx: i32, hy: i32| {
        rect(set, x, y0, 2, y1 - y0 + 1, c1);
        rect(set, hx, hy, 2, 1, skin);
    };
    let fwd = |set: &mut dyn FnMut(i32, i32, Color32), hy: i32| {
        rect(set, 4, 17, 2, 2, c1);
        rect(set, 4, 19, 2, 1, skin);
        rect(set, 14, hy - 1, 3, 2, c1);
        rect(set, 17, hy - 1, 2, 2, skin);
    };
    match arms {
        Arms::Swing1 => {
            arm(&mut set, 4, 17, 19, 5, 20);
            arm(&mut set, 14, 17, 19, 13, 20);
        }
        Arms::Swing2 => {
            arm(&mut set, 4, 17, 19, 3, 20);
            arm(&mut set, 14, 17, 19, 15, 20);
        }
        Arms::Type1 => fwd(&mut set, 19),
        Arms::Type2 => fwd(&mut set, 20),
        Arms::Down => {
            arm(&mut set, 4, 17, 20, 4, 21);
            arm(&mut set, 14, 17, 20, 14, 21);
        }
    }
    match legs {
        Legs::Sit => {
            rect(&mut set, 7, 22, 9, 2, leg);
            rect(&mut set, 14, 24, 2, 3, leg);
            rect(&mut set, 14, 26, 3, 1, INK);
        }
        Legs::Walk1 => {
            rect(&mut set, 7, 22, 2, 5, leg);
            rect(&mut set, 11, 22, 2, 4, leg);
            rect(&mut set, 7, 26, 3, 1, INK);
            rect(&mut set, 11, 25, 3, 1, INK);
        }
        Legs::Walk2 => {
            rect(&mut set, 7, 22, 2, 4, leg);
            rect(&mut set, 11, 22, 2, 5, leg);
            rect(&mut set, 7, 25, 3, 1, INK);
            rect(&mut set, 11, 26, 3, 1, INK);
        }
        Legs::Stand => {
            rect(&mut set, 7, 22, 2, 5, leg);
            rect(&mut set, 11, 22, 2, 5, leg);
            rect(&mut set, 7, 26, 3, 1, INK);
            rect(&mut set, 11, 26, 3, 1, INK);
        }
    }
    // the ink outline: every empty dot next to a filled one
    let mut s: Sprite = Vec::new();
    for y in 0..SH {
        for x in 0..SW {
            if out.contains_key(&(x, y)) {
                continue;
            }
            let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| out.get(&(x + dx, y + dy)).is_some_and(|c| *c != INK));
            if edge {
                s.push((x as i8, y as i8, INK));
            }
        }
    }
    for ((x, y), c) in out {
        s.push((x as i8, y as i8, c));
    }
    s
}

// ---- lettering -------------------------------------------------------------------------------

pub struct Font {
    glyphs: HashMap<char, (i32, Vec<(i32, i32)>)>,
}

impl Font {
    pub fn new() -> Self {
        let mut glyphs = HashMap::new();
        for (ch, spec) in crate::glyphs::GLYPHS {
            let rows: Vec<&str> = spec.split('|').collect();
            let w = rows[0].chars().count() as i32;
            let mut dots = Vec::new();
            for (y, row) in rows.iter().enumerate() {
                for (x, c) in row.chars().enumerate() {
                    if c == '#' {
                        dots.push((x as i32, y as i32));
                    }
                }
            }
            glyphs.insert(*ch, (w, dots));
        }
        Font { glyphs }
    }

    fn glyph(&self, ch: char) -> &(i32, Vec<(i32, i32)>) {
        self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?')).unwrap()
    }

    pub fn width(&self, s: &str) -> i32 {
        let w: i32 = s.chars().map(|c| self.glyph(c).0 + 1).sum();
        (w - 1).max(0)
    }

    // the dots of a line of text, top-left at (x, y)
    pub fn dots(&self, s: &str, x: i32, y: i32, out: &mut Vec<(i32, i32)>) {
        let mut cx = x;
        for ch in s.chars() {
            let (w, d) = self.glyph(ch);
            out.extend(d.iter().map(|(dx, dy)| (cx + dx, y + dy)));
            cx += w + 1;
        }
    }
}
