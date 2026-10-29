//! The whiteboard's dreams: two pictures (the hill, the beach) painted a dot at a time while the bots
//! nap, layer by layer, outlines before colours, left to right (room.js's dreamScene).

use pix::Color;
use std::collections::HashMap;

pub const DREAM_W: i32 = 65;
pub const DREAM_H: i32 = 27;
pub const DREAMS: usize = 2;

/// the dreams' colours by letter
pub fn dream_pal(ch: char) -> Option<Color> {
    Some(Color::rgb(match ch {
        'k' => 0x2b2433,
        'S' => 0xe9f3ff,
        'A' => 0x4fb3a9,
        'B' => 0xf06b72,
        'p' => 0xf59ab0,
        'q' => 0xd6457a,
        'r' => 0xe8505b,
        'y' => 0xf7d154,
        'o' => 0xf29b38,
        'g' => 0x7cc96a,
        'G' => 0x3f8f45,
        'c' => 0x9fd3f2,
        'b' => 0x5aa9e6,
        'w' => 0xd6efff,
        'n' => 0xf3d99b,
        'N' => 0xd9b26a,
        't' => 0x8a5a32,
        'm' => 0xb8bcc6,
        _ => return None,
    }))
}
/// the letters that are outlines (painted before the colours of their layer)
fn is_line(c: char) -> bool {
    matches!(c, 'k' | 'G' | 'o' | 'N' | 't' | 'q')
}
const BOT: [&str; 12] = ["....k....", "....k....", "..kkkkk..", ".kSSSSSk.", ".kSkSkSk.", ".kpSSSpk.", "..kkkkk..", ".kCCCCCk.", "kCCCCCCCk", ".kCCCCCk.", "..kk.kk..", ".k.....k."];
const BOW: [&str; 2] = ["..qq.qq..", "...qkq..."]; // hers: a bow on her antenna
const HEART: [&str; 6] = [".kk.kk.", "krrkrrk", "krrrrrk", ".krrrk.", "..krk..", "...k..."];
const SMALL: [&str; 3] = ["p.p", "ppp", ".p."];
const BIRD: [&str; 3] = ["k...k", ".k.k.", "..k.."];
const CLOUD: [&str; 4] = ["..cccc...", ".cccccccc", "ccccccccc", ".ccccccc."];

/// a dot of a dream: where, its colour's letter, its layer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DreamDot {
    pub x: i32,
    pub y: i32,
    pub c: char,
    pub layer: i32,
}

/// the dots (a Map in room.js: a dot painted again keeps its place in the order, takes the new colour)
struct Dots {
    order: Vec<DreamDot>,
    at: HashMap<(i32, i32, i32), usize>,
}
impl Dots {
    fn put(&mut self, x: i32, y: i32, c: char, layer: i32) {
        if c == '.' || !(0..DREAM_W).contains(&x) || !(0..DREAM_H).contains(&y) {
            return;
        }
        let d = DreamDot { x, y, c, layer };
        match self.at.get(&(layer, x, y)) {
            Some(&i) => self.order[i] = d,
            None => {
                self.at.insert((layer, x, y), self.order.len());
                self.order.push(d);
            }
        }
    }
    fn sprite(&mut self, rows: &[&str], x0: i32, y0: i32, layer: i32, cc: char) {
        for (j, r) in rows.iter().enumerate() {
            for (i, ch) in r.chars().enumerate() {
                self.put(x0 + i as i32, y0 + j as i32, if ch == 'C' { cc } else { ch }, layer);
            }
        }
    }
    fn sun(&mut self, cx: i32, cy: i32) {
        for y in -3..=3 {
            for x in -3..=3 {
                let d = (x as f64).hypot(y as f64);
                if d <= 3.3 {
                    self.put(cx + x, cy + y, if d > 2.4 { 'o' } else { 'y' }, 0);
                }
            }
        }
        for k in 0..8 {
            let a = k as f64 * std::f64::consts::PI / 4.;
            for r in [5., 6.] {
                self.put(cx + js(a.cos() * r), cy + js(a.sin() * r), 'o', 0);
            }
        }
    }
    /// him, her (the bow), holding hands; running lines behind, the heart over them
    fn lovers(&mut self, x0: i32, ground: i32) {
        let y0 = ground - 12;
        for dy in [5, 8] {
            for x in x0 - 5..x0 - 2 {
                self.put(x, y0 + dy, 'm', 3);
            }
        }
        self.sprite(&BOT, x0, y0, 3, 'A');
        self.sprite(&BOT, x0 + 9, y0, 3, 'B');
        self.sprite(&BOW, x0 + 9, y0, 3, '.');
        self.sprite(&HEART, x0 + 6, y0 - 7, 4, '.');
        self.sprite(&SMALL, x0 + 15, y0 - 7, 4, '.');
        self.sprite(&SMALL, x0 + 19, y0 - 3, 4, '.');
    }
}
fn js(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// dream `i`'s dots in the order they are painted
pub fn dream_scene(i: usize) -> Vec<DreamDot> {
    let mut p = Dots { order: Vec::new(), at: HashMap::new() };
    if i == 0 {
        // the hill: the sun, two clouds, birds; the grass, flowers, a tree
        p.sun(7, 6);
        p.sprite(&CLOUD, 47, 1, 0, '.');
        p.sprite(&CLOUD, 15, 7, 0, '.');
        p.sprite(&BIRD, 16, 2, 0, '.');
        p.sprite(&BIRD, 46, 8, 0, '.');
        let s = |x: i32| js(26. - (2. + 4.5 * (-((x as f64 - 33.) / 17.).powi(2)).exp()));
        for x in 0..DREAM_W {
            for y in s(x)..DREAM_H {
                p.put(x, y, if y == s(x) { 'G' } else { 'g' }, 1);
            }
        }
        for (k, x) in [4, 9, 14, 46, 50, 61].into_iter().enumerate() {
            p.put(x, s(x) + 2, if k % 2 != 0 { 'y' } else { 'r' }, 2);
            p.put(x, s(x) + 1, 'G', 2);
        }
        for y in s(56) - 4..s(56) {
            p.put(56, y, 't', 2);
        }
        for y in -3..=3 {
            for x in -3..=3 {
                let d = (x as f64).hypot(y as f64);
                if d <= 3.3 {
                    p.put(56 + x, s(56) - 7 + y, if d > 2.4 { 'G' } else { 'g' }, 2);
                }
            }
        }
        p.lovers(24, s(28));
    } else {
        // the beach: the sun over the sea, waves, the sand, a palm, a crab, gulls
        p.sun(56, 6);
        p.sprite(&BIRD, 16, 3, 0, '.');
        p.sprite(&BIRD, 23, 1, 0, '.');
        let shore = |x: i32| js(19. + 1.5 * (x as f64 / 6.).sin());
        for x in 0..DREAM_W {
            for y in 12..shore(x) {
                let wave = y > 12 && ((x * 7 + y * 3) % 11 == 0 || ((x - 1) * 7 + y * 3) % 11 == 0); // little waves
                p.put(x, y, if wave { 'w' } else { 'b' }, 1);
            }
            for y in shore(x)..DREAM_H {
                p.put(x, y, if y == shore(x) { 'N' } else { 'n' }, 1);
            }
        }
        for k in 0..15 {
            // the palm's trunk, bending
            let y = 25 - k;
            let x = 6 + js(4. * ((k as f64 / 15.) * 1.6).sin());
            p.put(x, y, 't', 2);
            p.put(x + 1, y, 't', 2);
        }
        p.sprite(&["...GGG...GGG...", ".GGgggG.GgggGG.", "Ggg...GgG...ggG", "G....G.t.G....G", "....G.ttt.G...."], 3, 9, 2, '.'); // its leaves, coconuts under them
        p.sprite(&["rk...kr", ".rrrrr.", "r.r.r.r"], 50, 22, 2, '.'); // a crab
        p.lovers(24, 25);
    }
    let hash = |x: i32, y: i32| {
        let a = (x as i64 * 73856093) as i32 ^ (y as i64 * 19349663) as i32;
        (a.wrapping_mul(2654435761u32 as i32) as u32 % 1000) as f64 / 1000.
    };
    let mut out = p.order;
    out.sort_by(|a, b| {
        let l = a.layer - b.layer;
        if l != 0 {
            return l.cmp(&0);
        }
        let k = (!is_line(a.c)) as i32 - (!is_line(b.c)) as i32;
        if k != 0 {
            return k.cmp(&0);
        }
        let d = (a.x as f64 + hash(a.x, a.y) * 6.) - (b.x as f64 + hash(b.x, b.y) * 6.);
        d.partial_cmp(&0.).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}
