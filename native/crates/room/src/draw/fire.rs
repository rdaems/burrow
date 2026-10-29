//! The fireplace: a fire pit in the middle of the room, seen from above like the tables: a ring of
//! stones on the floor, embers and logs in it, the flames rising from it. What is dragged into it
//! burns. `burst` (0..1) is a shower of sparks: the pointer stirred it, or something burnt.

use super::*;

/// the half widths of an ellipse's rows, `rows` of them
pub fn ell(rx: i32, ry: i32, rows: i32) -> Vec<i32> {
    let _ = ry;
    (0..rows)
        .map(|j| {
            let k = (j as f64 + 0.5 - rows as f64 / 2.) / (rows as f64 / 2.);
            js_round(rx as f64 * (1. - k * k).max(0.).sqrt())
        })
        .collect()
}

/// something thrown in: its picture, where it was picked up (its top left), seconds since
pub struct Burning<'a> {
    pub img: &'a Image,
    pub x: f64,
    pub y: f64,
    pub t: f64,
}

/// `Pit`: the stones, the embers, the logs (flat on the floor, for the 3d view's floor); `Flames`: what
/// rises (the flames, the sparks, what is thrown in); or both
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirePart {
    All,
    Pit,
    Flames,
}

/// the fire at x0, y0 (the top left of its footprint)
#[allow(clippy::too_many_arguments)]
pub fn draw_fire(ctx: &mut Ctx, x0: i32, y0: i32, t: f64, feeding: bool, burst: f64, burning: &[Burning], part: FirePart) {
    // every measure of the 32 x 28 original scaled by FIRE.S (X), so the pit keeps its proportions
    let s = FIRE.s;
    let xx = |v: i32| js_round(v as f64 * s);
    let cx = x0 + xx(16);
    let r = |ctx: &mut Ctx, y: i32, x: i32, w: i32, c: Color| rect(ctx, cx + x, y0 + y, w, 1, c);
    let (st_k, st_m, st_hi, st_d) = (Color::rgb(0x4d4a4f), Color::rgb(0x8d8a86), Color::rgb(0xaaa59e), Color::rgb(0x6e6b6a)); // the stones
    let k = (t * 6.).floor() as i32 % 3;
    let big = if feeding { xx(4) } else if !burning.is_empty() { xx(3) } else { 0 };
    if part != FirePart::Flames {
        // the ring of stones at the bottom, lit along its far side, in shade on the near one
        let rows = xx(12);
        let top = ell(xx(15), xx(6), rows);
        let ty = xx(16);
        for (j, &hw) in top.iter().enumerate() {
            let j = j as i32;
            r(ctx, ty + j, -hw, hw * 2, st_k);
            if hw > 2 {
                let c = if (j as f64) < rows as f64 / 3. {
                    st_hi
                } else if j as f64 > rows as f64 * 2. / 3. {
                    st_d
                } else {
                    st_m
                };
                r(ctx, ty + j, -hw + 1, hw * 2 - 2, c);
            }
        }
        for (x, y) in [(-12, 20), (-6, 17), (2, 17), (9, 18), (12, 22), (-10, 25), (-1, 26), (6, 26), (11, 25), (-13, 23), (-3, 17), (14, 20), (3, 27), (-8, 27)] {
            r(ctx, xx(y), xx(x), 1, st_k);
        }
        for (j, hw) in ell(xx(12), xx(4), xx(8)).into_iter().enumerate() {
            r(ctx, xx(18) + j as i32, -hw, hw * 2, Color::rgb(0x120a0c));
        }
        r(ctx, xx(28), -xx(14), xx(28), SHADOW);
        // embers and two logs
        for (x, y) in [(-9, 21), (-7, 23), (7, 20), (9, 23), (-3, 24), (4, 24), (-11, 22), (11, 21), (0, 22)] {
            r(ctx, xx(y), xx(x), 1, if (k + x) % 2 != 0 { C::RED1 } else { Color::rgb(0x7a2a18) });
        }
        r(ctx, xx(22), -xx(7), xx(6), Color::rgb(0x3b2a1e));
        r(ctx, xx(23), -xx(8), xx(7), Color::rgb(0x5c3a26));
        r(ctx, xx(22), xx(1), xx(6), Color::rgb(0x3b2a1e));
        r(ctx, xx(23), xx(2), xx(6), Color::rgb(0x5c3a26));
    }
    if part == FirePart::Pit {
        return;
    }
    // what was thrown in: tossed over to the pit in an arc, then it sinks into it, the flames in
    // front of it (what is below the pit's floor is not seen)
    let floor_y = y0 + xx(23);
    for o in burning {
        // anything wider than the pit shrinks to fit it as it flies in (a desk would be cropped by the pit's clip)
        let (ow, oh) = (o.img.w as f64, o.img.h as f64);
        let fit = (xx(24) as f64 / ow).min(1.);
        let (sw, sh) = (js_round(ow * fit), js_round(oh * fit));
        let tx = cx - js_round(sw as f64 / 2.);
        let oy = floor_y - sh;
        if o.t < 0.35 {
            let k = o.t / 0.35;
            let e = k * (2. - k);
            let sc = 1. + (fit - 1.) * e;
            let dx = js_round(o.x + (tx as f64 - o.x) * e);
            let dy = js_round(o.y + (oy as f64 - o.y) * e - (k * std::f64::consts::PI).sin() * xx(12) as f64);
            ctx.draw_image_scaled(o.img, dx as f64, dy as f64, js_round(ow * sc) as f64, js_round(oh * sc) as f64);
        } else {
            let k = ((o.t - 0.35) / 0.8).min(1.);
            ctx.save();
            ctx.clip_rect((x0 + xx(3)) as f64, (y0 - 40) as f64, xx(26) as f64, (floor_y - (y0 - 40)) as f64);
            ctx.draw_image_scaled(o.img, tx as f64, (oy + js_round(k * k * (sh + 4) as f64)) as f64, sw as f64, sh as f64);
            ctx.restore();
        }
    }
    // the flames, up from the embers: a tongue a dot, tallest in the middle, of three heights a frame;
    // red, gold in them, yellow at the heart
    let (base, hw, hmax) = (xx(22), xx(7), xx(18));
    let flame = |ctx: &mut Ctx, x: i32, h: i32, c: Color| rect(ctx, cx + x, y0 + base - h, 1, h, c);
    let fl: Vec<(i32, i32)> = (-hw..=hw)
        .map(|x| (x, js_round(xx(15) as f64 - xx(13) as f64 * x.abs() as f64 / hw as f64) + (k + x.abs()) % 3))
        .collect();
    for &(x, h) in &fl {
        flame(ctx, x, hmax.min(h + big), C::RED1);
    }
    for &(x, h) in &fl {
        flame(ctx, x, 1.max(js_round(hmax.min(h + big) as f64 * 0.62)), C::GOLD1);
    }
    for &(x, h) in &fl {
        if x.abs() < xx(4) {
            flame(ctx, x, 1.max(js_round(hmax.min(h + big) as f64 * 0.3)), C::GOLD2);
        }
    }
    if burst >= 0. {
        // sparks, out and up, dying out
        for i in 0..14 {
            let a = (i as f64 * 2.399) % 6.283;
            let sp = 0.5 + ((i * 7919) % 100) as f64 / 200.;
            let tt = burst * sp * 1.6;
            if tt > 1. {
                continue;
            }
            let sx = js_round(a.sin() * xx(16) as f64 * tt);
            let sy = js_round(xx(22) as f64 - xx(8) as f64 * a.cos() * tt - xx(26) as f64 * tt);
            r(ctx, sy, sx, 1, if tt < 0.5 { C::GOLD2 } else if tt < 0.8 { C::GOLD1 } else { C::RED1 });
        }
    }
}
