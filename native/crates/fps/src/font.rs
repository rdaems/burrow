//! pair's lettering, as the room has it (room.js `tiny`, `tinyW`): 3x5 glyphs, upper case, like a Pico-8; a letter
//! pixel a dot. A copy here: the HUD's words and the names on the desks and counters are in it.

use std::collections::HashMap;
use std::sync::OnceLock;

const GLYPHS: &str = concat!(
    "A .#./#.#/###/#.#/#.# B ##./#.#/##./#.#/##. C .##/#../#../#../.## D ##./#.#/#.#/#.#/##. E ###/#../##./#../### ",
    "F ###/#../##./#../#.. G .##/#../#.#/#.#/.## H #.#/#.#/###/#.#/#.# I ###/.#./.#./.#./### J ..#/..#/..#/#.#/.#. ",
    "K #.#/#.#/##./#.#/#.# L #../#../#../#../### M #.#/###/###/#.#/#.# N ##./#.#/#.#/#.#/#.# O .#./#.#/#.#/#.#/.#. ",
    "P ##./#.#/##./#../#.. Q .#./#.#/#.#/##./.## R ##./#.#/##./#.#/#.# S .##/#../.#./..#/##. T ###/.#./.#./.#./.#. ",
    "U #.#/#.#/#.#/#.#/.## V #.#/#.#/#.#/#.#/.#. W #.#/#.#/###/###/#.# X #.#/#.#/.#./#.#/#.# Y #.#/#.#/.#./.#./.#. ",
    "Z ###/..#/.#./#../### 0 ###/#.#/#.#/#.#/### 1 .#./##./.#./.#./### 2 ##./..#/.#./#../### 3 ##./..#/.#./..#/##. ",
    "4 #.#/#.#/###/..#/..# 5 ###/#../##./..#/##. 6 .##/#../###/#.#/### 7 ###/..#/.#./.#./.#. 8 ###/#.#/###/#.#/### ",
    "9 ###/#.#/###/..#/##. . .../.../.../.../.#. , .../.../.../.#./#.. ! .#./.#./.#./.../.#. ? ##./..#/.#./.../.#. ",
    "' .#./.#./.../.../... - .../.../###/.../... : .../.#./.../.#./... ( ..#/.#./.#./.#./..# ) #../.#./.#./.#./#.. ",
    "/ ..#/..#/.#./#../#.. + .../.#./###/.#./... = .../###/.../###/... % #.#/..#/.#./#../#.# * #.#/.#./#.#/.../... ",
    "# #.#/###/#.#/###/#.# \" #.#/#.#/.../.../... & .#./#.#/.#./#.#/.## _ .../.../.../.../### ~ .../.##/##./.../...",
);

fn glyphs() -> &'static HashMap<char, [bool; 15]> {
    static F: OnceLock<HashMap<char, [bool; 15]>> = OnceLock::new();
    F.get_or_init(|| {
        let mut f = HashMap::new();
        let words: Vec<&str> = GLYPHS.split(' ').filter(|w| !w.is_empty()).collect();
        for pair in words.chunks(2) {
            let (Some(k), Some(rows)) = (pair[0].chars().next(), pair.get(1)) else { continue };
            let mut g = [false; 15];
            for (r, row) in rows.split('/').enumerate().take(5) {
                for (c, ch) in row.chars().take(3).enumerate() {
                    g[r * 3 + c] = ch == '#';
                }
            }
            f.insert(k, g);
        }
        f
    })
}

/// how wide a text is in letter pixels
pub fn tiny_w(s: &str) -> i32 {
    (s.chars().count() as i32 * 4 - 1).max(0)
}
/// each lit letter pixel of a text at x, y (upper case)
pub fn tiny(s: &str, x: i32, y: i32, mut put: impl FnMut(i32, i32)) {
    let f = glyphs();
    for (i, ch) in s.to_uppercase().chars().enumerate() {
        if let Some(g) = f.get(&ch) {
            for k in 0..15 {
                if g[k] {
                    put(x + i as i32 * 4 + (k % 3) as i32, y + (k / 3) as i32);
                }
            }
        }
    }
}
