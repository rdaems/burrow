//! What is done to a picture: a piece drawn by itself (to toss in the fire, to outline), outlined,
//! washed in a colour, dissolved in the booth's light.

use crate::figure::halo;
use pix::{Color, Ctx, Image, Op};

/// a piece drawn into a canvas of its footprint (plus a little around)
pub fn snapshot(r: [f64; 4], pad: f64, f: impl FnOnce(&mut Ctx)) -> Image {
    let mut im = Image::new((r[2] - r[0] + 2. * pad) as u32, (r[3] - r[1] + 2. * pad) as u32);
    {
        let mut o = im.ctx();
        o.translate(pad - r[0], pad - r[1]);
        f(&mut o);
    }
    im
}

/// a piece put with an outline around it (the pointer is on it, or it is in danger), or drawn straight;
/// `tint` (0..1) washes it in the outline's colour as well: red, flashing, for what is about to burn
pub fn outlined(ctx: &mut Ctx, r: [f64; 4], on: bool, col: Color, tint: f64, f: impl FnOnce(&mut Ctx)) {
    if !on {
        f(ctx);
        return;
    }
    let pad = 2.;
    let c = snapshot(r, pad, f);
    ctx.draw_image(&halo(&c, col), r[0] - pad, r[1] - pad);
    if tint != 0. {
        ctx.draw_image(&tinted(&c, col, tint), r[0] - pad, r[1] - pad);
    } else {
        ctx.draw_image(&c, r[0] - pad, r[1] - pad);
    }
}

/// a picture washed in a colour, its own pixels only (the danger's red over what is held over the fire)
pub fn tinted(im: &Image, col: Color, a: f64) -> Image {
    let mut c = im.clone();
    {
        let mut x = c.ctx();
        x.set_op(Op::SourceAtop);
        x.set_alpha(a);
        x.fill_rect(0., 0., im.w as f64, im.h as f64, col);
    }
    c
}

/// the danger's flash: the wash goes between two strengths, five times a second
pub fn flash_a(t: f64) -> f64 {
    if (t * 5.).floor() as i64 % 2 != 0 { 0.5 } else { 0.3 }
}

pub const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
/// a figure going up in the booth's light: a front of light rises from its feet (d 0 to 1); what it has passed
/// turns to light and thins out in an ordered dither until nothing is left
pub fn dissolve(im: &Image, d: f64) -> Image {
    let mut c = im.clone();
    if d <= 0. {
        return c;
    }
    let (top, foot) = (3., c.h as f64 - 1.);
    // (written through a Uint8ClampedArray: rounded half to even)
    let to = |v: u8, t: f64, m: f64| (v as f64 + (t - v as f64) * m).round_ties_even().clamp(0., 255.) as u8;
    for j in 0..c.h as i32 {
        let a = ((d * 1.4 - (foot - j as f64) / (foot - top)) / 0.4).clamp(0., 1.);
        if a == 0. {
            continue;
        }
        for i in 0..c.w as i32 {
            let mut p = c.get(i, j);
            if p[3] == 0 {
                continue;
            }
            if (BAYER[(j % 4) as usize][(i % 4) as usize] as f64) / 16. < a {
                p[3] = 0;
                c.set(i, j, p);
                continue;
            }
            let m = (a * 1.6).min(1.) * 0.85;
            p = [to(p[0], 255., m), to(p[1], 244., m), to(p[2], 214., m), p[3]];
            c.set(i, j, p);
        }
    }
    c
}
