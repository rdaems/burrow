//! pair's lettering: 3x5 glyphs, upper case, like a Pico-8; a letter pixel a dot (he tried it smaller and
//! in the terminal's font: worse).

use pix::{Color, Ctx};

const GLYPHS: &str = "A .#./#.#/###/#.#/#.# B ##./#.#/##./#.#/##. C .##/#../#../#../.## D ##./#.#/#.#/#.#/##. E ###/#../##./#../### \
F ###/#../##./#../#.. G .##/#../#.#/#.#/.## H #.#/#.#/###/#.#/#.# I ###/.#./.#./.#./### J ..#/..#/..#/#.#/.#. \
K #.#/#.#/##./#.#/#.# L #../#../#../#../### M #.#/###/###/#.#/#.# N ##./#.#/#.#/#.#/#.# O .#./#.#/#.#/#.#/.#. \
P ##./#.#/##./#../#.. Q .#./#.#/#.#/##./.## R ##./#.#/##./#.#/#.# S .##/#../.#./..#/##. T ###/.#./.#./.#./.#. \
U #.#/#.#/#.#/#.#/.## V #.#/#.#/#.#/#.#/.#. W #.#/#.#/###/###/#.# X #.#/#.#/.#./#.#/#.# Y #.#/#.#/.#./.#./.#. \
Z ###/..#/.#./#../### 0 ###/#.#/#.#/#.#/### 1 .#./##./.#./.#./### 2 ##./..#/.#./#../### 3 ##./..#/.#./..#/##. \
4 #.#/#.#/###/..#/..# 5 ###/#../##./..#/##. 6 .##/#../###/#.#/### 7 ###/..#/.#./.#./.#. 8 ###/#.#/###/#.#/### \
9 ###/#.#/###/..#/##. . .../.../.../.../.#. , .../.../.../.#./#.. ! .#./.#./.#./.../.#. ? ##./..#/.#./.../.#. \
' .#./.#./.../.../... - .../.../###/.../... : .../.#./.../.#./... ( ..#/.#./.#./.#./..# ) #../.#./.#./.#./#.. \
/ ..#/..#/.#./#../#.. + .../.#./###/.#./... = .../###/.../###/... % #.#/..#/.#./#../#.# * #.#/.#./#.#/.../... \
# #.#/###/#.#/###/#.# \" #.#/#.#/.../.../... & .#./#.#/.#./#.#/.## _ .../.../.../.../### ~ .../.##/##./.../...";

/// a glyph's 15 bits, row by row, or none
pub fn glyph(ch: char) -> Option<[bool; 15]> {
    let mut it = GLYPHS.split_whitespace();
    while let (Some(k), Some(rows)) = (it.next(), it.next()) {
        if k.chars().next() == Some(ch) && k.chars().count() == 1 {
            let mut out = [false; 15];
            for (j, r) in rows.split('/').enumerate().take(5) {
                for (i, c) in r.chars().take(3).enumerate() {
                    out[j * 3 + i] = c == '#';
                }
            }
            return Some(out);
        }
    }
    None
}

/// how wide a text is in dots
pub fn tiny_w(s: &str) -> i32 {
    (s.chars().count() as i32 * 4 - 1).max(0)
}

/// upper-cased into 3x5 glyphs at x, y, a dot a letter pixel
pub fn tiny(ctx: &mut Ctx, s: &str, x: f64, y: f64, col: Color) {
    for (i, ch) in s.to_uppercase().chars().enumerate() {
        if let Some(g) = glyph(ch) {
            for (k, on) in g.iter().enumerate() {
                if *on {
                    ctx.fill_rect(x + (i * 4 + k % 3) as f64, y + (k / 3) as f64, 1., 1., col);
                }
            }
        }
    }
}
