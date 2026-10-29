//! The painting on the easel: the robots paint famous paintings (paintings.rs), every person in them a robot. It
//! grows only while a robot stands at the easel painting (`paint`, a break of its own, now and then), layer by layer
//! as a painter would, ART_SPEED dots a second; done, it stays, and once it has hung there ART_REST seconds the next
//! robot to come starts another on a fresh canvas. The room opens on one well begun. Who painted it is kept
//! (room.js's newArt, paintArt, artStep, artSmall).

use crate::paintings::{self, PRIMER};
use crate::world::{Art, World};
use pix::{Color, Image};

pub const ART_SPEED: f64 = 30.0;
pub const ART_REST: f64 = 600.0;

/// a fresh canvas for painting `i`, `start` of it already painted
pub fn new_art(w: &mut World, i: usize, start: f64) {
    let p = &paintings::list()[i];
    let order = paintings::order(p.make());
    let upto = (order.len() as f64 * start).floor() as usize;
    w.s.art = Some(Art { i, order, n: 0, done: None, by: Vec::new(), f: 0.0, img: Image::filled(p.w as u32, p.h as u32, PRIMER) });
    w.s.art_v += 1;
    w.s.art_small = None;
    paint_art(w, upto);
}

fn paint_art(w: &mut World, upto: usize) {
    let t = w.s.t;
    let Some(a) = w.s.art.as_mut() else { return };
    let end = upto.min(a.order.len());
    if a.n < end {
        for d in &a.order[a.n..end] {
            a.img.set(d.x, d.y, d.c.0);
        }
        a.n = end;
        w.s.art_v += 1;
        w.s.art_small = None;
    }
    if a.n >= a.order.len() && a.done.is_none() {
        a.done = Some(t);
    }
}

/// unfinished, or done a while: a robot may come and paint
pub fn art_wanted(w: &World) -> bool {
    w.s.art.as_ref().is_none_or(|a| a.done.is_none_or(|d| w.s.t - d >= ART_REST))
}

/// a tick: the painter at the easel paints. `r0`, `r1`: the dice for the first painting (which, how far along) and
/// the next one
pub fn art_step(w: &mut World, dt: f64, r0: f64, r1: f64, r2: f64) {
    let n = paintings::list().len();
    if w.s.art.is_none() {
        new_art(w, ((r0 * n as f64).floor() as usize).min(n - 1), 0.4 + r1 * 0.6);
        let t = w.s.t;
        if let Some(a) = w.s.art.as_mut()
            && a.done.is_some()
        {
            a.done = Some(t - ART_REST / 2.0);
        }
    }
    let Some(painter) = w.folk.iter().find(|f| f.act.kind == "paint" && f.act.there).map(|f| f.id.clone()) else { return };
    if !art_wanted(w) {
        return;
    }
    let a = w.s.art.as_ref().unwrap();
    if a.done.is_some() {
        let next = (a.i + 1 + (r2 * (n - 1) as f64).floor() as usize) % n; // the next one, never the same
        new_art(w, next, 0.0);
    }
    let a = w.s.art.as_mut().unwrap();
    if !a.by.contains(&painter) {
        a.by.push(painter);
    }
    a.f += dt * ART_SPEED;
    if a.f >= 1.0 {
        let k = a.f.floor();
        a.f -= k;
        let upto = a.n + k as usize;
        paint_art(w, upto);
    }
}

/// the painting at a third of its size, each dot the mean of its three by three (kept until the painting changes)
pub fn art_small(w: &mut World) -> &[(i32, i32, Color)] {
    if w.s.art_small.is_none() {
        let mut out = Vec::new();
        if let Some(a) = &w.s.art {
            let p = &paintings::list()[a.i];
            for j in 0..p.h / 3 {
                for i in 0..p.w / 3 {
                    let mut sum = [0u32; 3];
                    for y in 0..3 {
                        for x in 0..3 {
                            let q = a.img.get(i * 3 + x, j * 3 + y);
                            for (s, v) in sum.iter_mut().zip(q) {
                                *s += v as u32;
                            }
                        }
                    }
                    let m = |v: u32| (v as f64 / 9.0).round() as u8;
                    out.push((i, j, Color([m(sum[0]), m(sum[1]), m(sum[2]), 255])));
                }
            }
        }
        w.s.art_small = Some(out);
    }
    w.s.art_small.as_deref().unwrap()
}
