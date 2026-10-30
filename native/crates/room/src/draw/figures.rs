//! What a figure leaves or wears besides its body: lying asleep on a mat, the z's over a sleeper, and the
//! remains of a shot bot (its picture in pieces, cogs, bolts, a spring).

use super::*;
use crate::figure::{SW, halo};
use std::rc::Rc;

/// a picture turned a quarter left (its top goes left): what `rotate(-PI / 2)` draws, without the
/// rotation's float error at the pixel edges
fn quarter_left(im: &Image) -> Image {
    let mut out = Image::new(im.h, im.w);
    for j in 0..im.h as i32 {
        for i in 0..im.w as i32 {
            out.set(j, im.w as i32 - 1 - i, im.get(i, j));
        }
    }
    out
}

/// lying on its back on the mat, head to the left: the standing figure (`im`, body(…, "down", "stand"))
/// turned on its side; `ring` its outline (lit, or its session open), `z` its phase for the z's (a
/// person's: a robot sleeps without them)
pub fn draw_sleeper(ctx: &mut Ctx, x: i32, y: i32, im: &Image, ring: Option<Color>, human: bool, z: f64) {
    // translate(x, y - 9), rotate(-PI / 2), drawImage(im, -SW / 2, -14 - up): image pixel (i, j) lands at
    // (x - 14 - up + j, y + 1 - i), `up` what a witch's hat adds to its height
    let up = im.h as i32 - crate::figure::SH;
    let (dx, dy) = (x - 14 - up, y + 2 - im.w as i32);
    debug_assert_eq!(im.w as i32, SW);
    if let Some(col) = ring {
        put(ctx, &quarter_left(&halo(im, col)), dx, dy, false);
    }
    put(ctx, &quarter_left(im), dx, dy, false);
    if human {
        zzz(ctx, z, x - 12, y - 22);
    }
}
/// where the pointer is on a sleeper
pub fn sleeper_hot(x: i32, y: i32) -> [i32; 4] {
    [x - 15, y - 17, x + 15, y - 1]
}

pub fn zed(ctx: &mut Ctx, x: i32, y: i32, col: Color) {
    rect(ctx, x, y, 3, 1, col);
    rect(ctx, x + 1, y + 1, 1, 1, col);
    rect(ctx, x, y + 2, 3, 1, col);
}
/// two z's rising from a sleeper's head (`z`: the figure's own phase)
pub fn zzz(ctx: &mut Ctx, z: f64, x: i32, y: i32) {
    for i in 0..2 {
        let k = (z * 0.4 + i as f64 * 0.5) % 1.;
        zed(ctx, js_round(x as f64 + k * 6.), js_round(y as f64 - k * 10.), C::STEEL3);
    }
}

/// a sausage, held by a figure at x, y facing `dir`, `at` seconds into a grill that takes `cook`: on its stick over the
/// fire, raw pink browning to a crisp (turned now and then: its lit side flips), smoking a little near the end (`t` the
/// room's time); then at its mouth, shorter with every bite
pub fn draw_sausage(ctx: &mut Ctx, x: i32, y: i32, dir: f64, at: f64, cook: f64, t: f64) {
    let d = if dir < 0. { -1 } else { 1 };
    let k = (at / cook).min(1.);
    let (col, hi) = if k < 0.35 {
        (Color::rgb(0xe8968a), Color::rgb(0xf8c4b4))
    } else if k < 0.75 {
        (Color::rgb(0xc8643c), Color::rgb(0xe8905c))
    } else {
        (Color::rgb(0x8a4426), Color::rgb(0xb8643a))
    };
    // outlined, as the room draws things
    let link = |ctx: &mut Ctx, sx: i32, sy: i32, n: i32| {
        rect(ctx, sx - 1, sy - 1, n + 2, 4, Color::rgb(0x3a1a10));
        rect(ctx, sx, sy, n, 2, col);
    };
    if at < cook {
        let (hx, hy) = (x + 8 * d, y - 8);
        for i in 0..17 {
            // the stick, up over the flames
            rect(ctx, hx + i * d, hy - (i as f64 / 2.5).floor() as i32, 1, 1, if i < 3 { Color::rgb(0x7a4a2a) } else { Color::rgb(0xd8b07a) });
        }
        let (sx, sy, turn) = ((hx + 13 * d).min(hx + 17 * d), hy - 8, (at / 2.5).floor() as i32 % 2);
        link(ctx, sx, sy, 5);
        rect(ctx, sx + 1, sy + turn, 3, 1, hi);
        if k > 0.55 && (t * 2.).floor() as i64 % 3 != 0 {
            rect(ctx, sx + 2, sy - 3 - ((t * 4.).floor() as i64 % 3) as i32, 1, 1, Color::rgba(206, 210, 220, 153)); // a wisp
        }
        return;
    }
    // bites at 1.2, 3.2, 5.2 s
    let e = at - cook;
    let left = if e < 1.2 { 5 } else { 5 - 2 * ((e - 1.2) / 2.).ceil() as i32 };
    if left > 0 {
        link(ctx, if d > 0 { x + 4 - left } else { x - 4 }, y - 12, left);
    }
}

pub const COG: [&str; 5] = [".#.#.", "#####", "##.##", "#####", ".#.#."];
pub const COG7: [&str; 7] = ["..#.#..", ".#####.", "###.###", "##...##", "###.###", ".#####.", "..#.#.."];

/// a piece of a shot bot: its own picture cut up (head, chest, arms, legs), or a cog, a bolt, a spring
#[derive(Clone, Debug)]
pub enum PartKind {
    Cut { img: Rc<Image>, sx: i32, sy: i32, sw: i32, sh: i32 },
    Cog { big: bool },
    Bolt,
    Spring,
}
/// where a piece is: on the floor at x, y, `z` up in the air, turning (`spin`); a burnt bot's are charred
#[derive(Clone, Debug)]
pub struct Part {
    pub kind: PartKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub spin: f64,
    pub charred: bool,
}

/// a piece of the remains, `fade` 0..1 as they rust away
pub fn draw_part(ctx: &mut Ctx, p: &Part, fade: f64) {
    let (x, y, z) = (js_round(p.x), js_round(p.y), js_round(p.z));
    if fade != 0. {
        ctx.set_alpha(1. - fade);
    }
    if z > 1 {
        rect(ctx, x - 2, y, 4, 1, SHADOW);
    }
    match &p.kind {
        PartKind::Cut { img, sx, sy, sw, sh } => {
            let fl = p.spin.floor() as i64 % 2 == 1;
            ctx.save();
            ctx.translate(x as f64, (y - z) as f64);
            if fl {
                ctx.scale(-1., 1.);
            }
            let dx = -js_round(*sw as f64 / 2.);
            ctx.draw_image_sub(img, *sx as f64, *sy as f64, *sw as f64, *sh as f64, dx as f64, -sh as f64, *sw as f64, *sh as f64);
            ctx.restore();
        }
        PartKind::Cog { big } => {
            let g: &[&str] = if *big { &COG7 } else { &COG };
            let n = g.len() as i32;
            let turn = p.spin.floor() as i64 % 2 != 0;
            for (j, row) in g.iter().enumerate() {
                for (i, ch) in row.chars().enumerate() {
                    if ch != '#' {
                        continue;
                    }
                    let (i, j) = (i as i32, j as i32);
                    let col = if i == n >> 1 && j == n >> 1 {
                        INK
                    } else if p.charred {
                        Color::rgb(0x3a3438)
                    } else if j < 2 {
                        C::STEEL3
                    } else {
                        C::STEEL2
                    };
                    rect(ctx, x - (n >> 1) + if turn { n - 1 - i } else { i }, y - z - n + j, 1, 1, col);
                }
            }
        }
        PartKind::Bolt => {
            rect(ctx, x - 1, y - z - 3, 3, 1, C::STEEL3);
            rect(ctx, x, y - z - 2, 1, 2, C::STEEL2);
        }
        PartKind::Spring => {
            for k in 0..4 {
                rect(ctx, x - 1 + (k % 2), y - z - 4 + k, 2, 1, C::STEEL3);
            }
        }
    }
    ctx.set_alpha(1.);
}
