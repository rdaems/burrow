//! The room's furniture as pixel art (web/sprites.js): letter grids a dot a letter in Pixel Agents'
//! palette (`sprites_gen.rs`, generated from sprites.js), and Pixel Agents' own pieces as their PNGs.
//! What changes with the state of things (screens, coffee, water, chess, clock) is painted in by the room.

use crate::sprites_gen as g;
use pix::{Color, Image, css};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// a palette letter's colour (sprites.js's PAL)
pub fn pal(ch: char) -> Color {
    g::PAL.iter().find(|(k, _)| *k == ch).map(|(_, v)| css(v)).unwrap_or(Color::NONE)
}

/// Pixel Agents' wood and the rest, by name, for the drawing code
pub mod sp {
    use pix::Color;
    pub const S: Color = Color::rgba(0, 0, 0, 51);
    pub const H: Color = Color::rgb(0xcfa86d);
    pub const W: Color = Color::rgb(0xb38857);
    pub const D: Color = Color::rgb(0x885c47);
    pub const K: Color = Color::rgb(0x532e3d);
    pub const A: Color = Color::rgb(0xffffff);
    pub const B: Color = Color::rgb(0xe1e3e9);
    pub const C: Color = Color::rgb(0xb5bfc7);
    pub const D2: Color = Color::rgb(0x757b7c); // 'D'
    pub const E: Color = Color::rgb(0x595e60);
    pub const F: Color = Color::rgb(0x3f3740);
    pub const G: Color = Color::rgb(0x394449);
    pub const X: Color = Color::rgb(0x140f1e); // 'x'
    pub const R: Color = Color::rgb(0xeb4c3f); // 'r'
    pub const R2: Color = Color::rgb(0x8c323a); // 'R'
    pub const O: Color = Color::rgb(0xededed); // 'o'
    pub const GR: Color = Color::rgb(0x60c677); // 'g'
    pub const Y: Color = Color::rgb(0xf2a531); // 'y'
    pub const Y2: Color = Color::rgb(0xffe27a); // 'Y'
    pub const V: Color = Color::rgb(0x4a76b8); // 'v'
    pub const M: Color = Color::rgb(0x1b6660); // 'M'
    pub const M2: Color = Color::rgb(0x2fa087); // 'm'
}

/// a letter grid drawn: each letter its palette colour (`over` replaces some), '.' and unknown letters clear
pub fn art(rows: &[&str], over: &[(char, Color)]) -> Image {
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
    let mut im = Image::new(w, rows.len() as u32);
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let c = over.iter().find(|(k, _)| *k == ch).map(|(_, c)| *c).unwrap_or_else(|| pal(ch));
            if !c.is_none() {
                im.set(i as i32, j as i32, c.0);
            }
        }
    }
    im
}

/// the same art from rows the drawing code builds (owned strings)
pub fn art_owned(rows: &[String], over: &[(char, Color)]) -> Image {
    let v: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    art(&v, over)
}

const PNGS: &[(&str, &[u8])] = &[
    ("PLANT", include_bytes!("../assets/PLANT.png")),
    ("PLANT_2", include_bytes!("../assets/PLANT_2.png")),
    ("WHITEBOARD", include_bytes!("../assets/WHITEBOARD.png")),
    ("CUSHIONED_CHAIR_SIDE", include_bytes!("../assets/CUSHIONED_CHAIR_SIDE.png")),
    ("WOODEN_BENCH", include_bytes!("../assets/WOODEN_BENCH.png")),
    ("SMALL_TABLE_FRONT", include_bytes!("../assets/SMALL_TABLE_FRONT.png")),
    ("COFFEE", include_bytes!("../assets/COFFEE.png")),
];

thread_local! {
    static CACHE: RefCell<HashMap<String, Rc<Image>>> = RefCell::new(HashMap::new());
}

fn cached(key: String, make: impl FnOnce() -> Option<Image>) -> Option<Rc<Image>> {
    if let Some(im) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(im);
    }
    let im = Rc::new(make()?);
    CACHE.with(|c| c.borrow_mut().insert(key, im.clone()));
    Some(im)
}

/// a sprite by its web name (`chair`, `monitor`, `PLANT`, ...)
pub fn get(name: &str) -> Option<Rc<Image>> {
    cached(name.to_string(), || {
        if let Some((_, rows)) = g::ALL.iter().find(|(n, _)| *n == name) {
            return Some(art(rows, &[]));
        }
        let (_, bytes) = PNGS.iter().find(|(n, _)| *n == name)?;
        Image::from_png(bytes).ok()
    })
}

/// a door in another paint (a palette swap, h w d k as the wood)
pub fn swap(name: &str, h: Color, w: Color, d: Color, k: Color) -> Rc<Image> {
    let rows = g::ALL.iter().find(|(n, _)| *n == name).map(|(_, r)| *r).unwrap_or(g::DOOR);
    cached(format!("{name}{h:?}{w:?}{d:?}{k:?}"), || Some(art(rows, &[('h', h), ('w', w), ('d', d), ('k', k)]))).unwrap()
}
