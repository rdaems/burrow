//! A shot bot leaves remains, not a gravestone: its own picture cut into head, chest, arms and legs, cogs,
//! bolts and a spring, flying apart with a hop and bouncing to a stop on a splash of oil. They are its undo:
//! a click on any of it puts it back together. Idle bots come over and clear them away, a piece at a time;
//! with the last piece gone the session is stopped and deleted. With nobody to clear them they rust away
//! after half a minute.

use crate::figure::{Arms, Legs, body};
use crate::layout::{WALL, jround};
use crate::think::Tick;
use crate::world::{Drop, Part, Remains, World};

/// a figure in pieces: they fly from where it stood; the figure goes
pub fn scatter(w: &mut World, i: usize, t: &mut Tick) {
    let f = w.folk.remove(i);
    let (x, y) = (jround(f.x) as f64, ((WALL + 14) as f64).max(jround(f.y) as f64));
    let im = body(&f.seed, f.color.unwrap_or(pix::Color::rgb(0x8b93ad)), Arms::Down, Legs::Stand, false, f.staff);
    let cut = |sx: i32, sy: i32, sw: i32, sh: i32, kind: &'static str| Part { kind, img: Some(im.clone()), sx, sy, w: sw, h: sh, ..Default::default() };
    let mut kinds = vec![cut(1, 1, 20, 17, "head"), cut(6, 18, 10, 5, "chest"), cut(4, 18, 3, 5, "arm"), cut(15, 18, 3, 5, "arm"), cut(7, 23, 4, 6, "leg"), cut(11, 23, 4, 6, "leg")];
    kinds.push(Part { kind: "cog", big: true, ..Default::default() });
    for k in ["cog", "cog", "bolt", "bolt", "bolt", "cog", "spring"] {
        kinds.push(Part { kind: k, ..Default::default() });
    }
    let parts: Vec<Part> = kinds
        .into_iter()
        .map(|mut p| {
            let a = t.rand() * std::f64::consts::TAU;
            let sp = 16.0 + t.rand() * 30.0;
            p.x = x + (t.rand() - 0.5) * 4.0;
            p.y = y - 1.0 + (t.rand() - 0.5) * 3.0;
            p.z = 8.0 + t.rand() * 10.0;
            p.vx = a.cos() * sp;
            p.vy = a.sin() * sp * 0.55;
            p.vz = 45.0 + t.rand() * 45.0;
            p.spin = t.rand() * 4.0;
            if t.inp.still {
                p.x += p.vx * 0.4;
                p.y += p.vy * 0.4;
                p.z = 0.0;
                p.vx = 0.0;
                p.vy = 0.0;
                p.vz = 0.0;
            }
            p
        })
        .collect();
    // the oil: a puddle where it stood, splashed all round, a few streaks flung out
    let mut drops: Vec<Drop> = (0..60)
        .map(|_| {
            let r = t.rand().powf(0.6) * 22.0;
            let s = if t.rand() < 0.35 { 2 } else { 1 };
            let a = t.rand() * 6.28;
            let hi = t.rand() < 0.15;
            Drop { x: jround(x + a.cos() * r) as f64, y: jround(y + a.sin() * r * 0.5) as f64, s, hi }
        })
        .collect();
    for _ in 0..5 {
        let a = t.rand() * 6.28;
        for k in 0..6 {
            let d = 10.0 + k as f64 * 3.0;
            drops.push(Drop { x: jround(x + a.cos() * d) as f64, y: jround(y + a.sin() * d * 0.5) as f64, s: if k < 3 { 2 } else { 1 }, hi: false });
        }
    }
    let puddle = 7.0 + jround(t.rand() * 3.0) as f64;
    let total = parts.len();
    w.s.remains.retain(|(k, _)| *k != f.id);
    w.s.remains.push((
        f.id.clone(),
        Remains {
            x,
            y,
            t0: t.now(),
            parts,
            drops,
            name: f.b.as_ref().map(|b| b.name.clone()).unwrap_or_default(),
            b: f.b.clone(),
            human: f.human,
            staff: f.staff,
            booth: f.booth.clone(),
            seed: f.seed.clone(),
            color: f.color,
            puddle,
            total,
            ..Default::default()
        },
    ));
}
