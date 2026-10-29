//! The robots' dream, on the whiteboard: a pixel-art picture that appears bit by bit, two happy robots in love,
//! hand in hand, running over a sunny hill, or along a beach. It grows by itself, slowly; faster for every robot
//! taking a nap (it is their dream) and while the host of a meeting draws at the board. Done, it stays; a meeting
//! a while later starts the next one on a wiped board. Your own marker is never drawn over.
//! Also the board's ink: what you and the dream draw on it, 130 x 55 (five pixels a board dot).

use crate::world::{Dream, World};
use pix::{Color, Image};
use std::collections::HashMap;

pub const INK_W: i32 = 26;
pub const INK_H: i32 = 11;
pub const INKS: i32 = 5;
pub const DREAM_W: i32 = 65;
pub const DREAM_H: i32 = 27;
pub const DREAMS: usize = 2;

/// the board's ink, made when first wanted
pub fn ink(w: &mut World) -> &mut Image {
    w.s.ink.get_or_insert_with(|| Image::new((INK_W * INKS) as u32, (INK_H * INKS) as u32))
}

/// a line on the board (ink pixels), Bresenham's
pub fn ink_line(w: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, col: Color, width: i32) {
    let im = ink(w);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut e, mut x, mut y) = (dx + dy, x0, y0);
    loop {
        for j in 0..width {
            for i in 0..width {
                im.set(x + i, y + j, col.0);
            }
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * e;
        if e2 >= dy {
            e += dy;
            x += sx;
        }
        if e2 <= dx {
            e += dx;
            y += sy;
        }
    }
    w.s.ink_v += 1;
}

/// the board wiped: the dream begins again
pub fn wipe(w: &mut World) {
    ink(w).clear();
    w.s.ink_v += 1;
    if let Some(d) = w.s.dream.as_mut() {
        d.n = 0;
        d.done = None;
        d.mine.clear();
    }
}

/// how much of the board is inked, 0..1
pub fn inked(w: &mut World) -> f64 {
    let im = ink(w);
    im.px.iter().filter(|p| p[3] > 128).count() as f64 / im.px.len() as f64
}

/// the board's dots from the ink: a dot's middle pixel, else its first inked one
pub fn cells(w: &mut World) -> Vec<(i32, i32, Color)> {
    let im = ink(w);
    let mut out = Vec::new();
    for j in 0..INK_H {
        for i in 0..INK_W {
            let m = im.get(i * INKS + 2, j * INKS + 2);
            let mut col = (m[3] > 128).then(|| Color([m[0], m[1], m[2], 255]));
            'find: for y in j * INKS..(j + 1) * INKS {
                if col.is_some() {
                    break;
                }
                for x in i * INKS..(i + 1) * INKS {
                    let k = im.get(x, y);
                    if k[3] > 128 {
                        col = Some(Color([k[0], k[1], k[2], 255]));
                        break 'find;
                    }
                }
            }
            if let Some(c) = col {
                out.push((i, j, c));
            }
        }
    }
    out
}

/// a scene's pixels in the order they are drawn in (room.js's dreamScene)
pub fn scene(i: usize) -> Vec<(i32, i32, Color)> {
    crate::paint::dream_scene(i)
}

pub fn new_dream(w: &mut World, scene_i: usize, start: f64) {
    let order = scene(scene_i);
    let upto = (order.len() as f64 * start).floor() as usize;
    w.s.dream = Some(Dream { scene: scene_i, order, n: 0, done: None, mine: HashMap::new(), f: 0.0 });
    paint_dream(w, upto);
}

fn paint_dream(w: &mut World, upto: usize) {
    let t = w.s.t;
    let Some(mut d) = w.s.dream.take() else { return };
    let mut painted = false;
    {
        let im = ink(w);
        while d.n < upto.min(d.order.len()) {
            let (x, y, c) = d.order[d.n];
            d.n += 1;
            let cell = (y * DREAM_W + x) as usize;
            let a = [im.get(x * 2, y * 2), im.get(x * 2 + 1, y * 2), im.get(x * 2, y * 2 + 1), im.get(x * 2 + 1, y * 2 + 1)];
            let was = Color([a[0][0], a[0][1], a[0][2], 255]);
            if a.iter().any(|p| p[3] != 0) && !(a[0][3] != 0 && d.mine.get(&cell) == Some(&was)) {
                continue; // your marker is there: left as it is
            }
            for (i, j) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                im.set(x * 2 + i, y * 2 + j, c.0);
            }
            d.mine.insert(cell, c);
            painted = true;
        }
    }
    if painted {
        w.s.ink_v += 1;
    }
    if d.n >= d.order.len() && d.done.is_none() {
        d.done = Some(t);
    }
    w.s.dream = Some(d);
}

pub fn dream_step(w: &mut World, dt: f64, first: f64) {
    if w.s.dream.is_none() {
        new_dream(w, (first * DREAMS as f64).floor() as usize % DREAMS, 0.3); // the room opens on a dream half begun
    }
    let sleepers = w.folk.iter().filter(|f| !f.human && (f.act.kind == "sleep" || f.act.kind == "doze") && f.act.there).count() as f64;
    let drawing = w.folk.iter().any(|f| f.act.kind == "present" && f.act.drawing);
    let (n, done) = {
        let d = w.s.dream.as_mut().unwrap();
        d.f += dt * (0.6 + 1.5 * sleepers + if drawing { 8.0 } else { 0.0 });
        let mut k = 0;
        if d.f >= 1.0 {
            k = d.f.floor() as usize;
            d.f -= k as f64;
        }
        (d.n + k, d.done)
    };
    paint_dream(w, n);
    if done.is_some_and(|done| w.s.t - done > 1200.0) {
        next_dream(w); // done twenty minutes and no meeting came to start another: the next one
    }
}

pub fn next_dream(w: &mut World) {
    let s = w.s.dream.as_ref().map(|d| (d.scene + 1) % DREAMS).unwrap_or(0);
    wipe(w);
    new_dream(w, s, 0.0);
}
