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
    // translate(x, y - 9), rotate(-PI / 2), drawImage(im, -SW / 2, -14): image pixel (i, j) lands at
    // (x - 14 + j, y + 1 - i)
    let (dx, dy) = (x - 14, y + 2 - im.w as i32);
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
