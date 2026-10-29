//! box drawing and block elements drawn as shapes, so they meet across cells (the font's glyphs leave
//! seams at a line height of 1.15; xterm.js draws them itself too). Claude Code's UI is boxes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W {
    No,
    Light,
    Heavy,
    Double,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    /// lines from the centre to the left, right, top and bottom edges
    Arms([W; 4]),
    /// a rounded corner: ╭ ╮ ╯ ╰, its arms (left or right, up or down)
    Round { right: bool, down: bool },
    /// rectangles in fractions of the cell, at an alpha
    Blocks(&'static [(f32, f32, f32, f32)], f32),
}

use W::*;

pub fn glyph(c: char) -> Option<Glyph> {
    let a = |l, r, u, d| Some(Glyph::Arms([l, r, u, d]));
    let b = |q: &'static [(f32, f32, f32, f32)]| Some(Glyph::Blocks(q, 1.0));
    const FULL: &[(f32, f32, f32, f32)] = &[(0.0, 0.0, 1.0, 1.0)];
    match c {
        '─' => a(Light, Light, No, No),
        '━' => a(Heavy, Heavy, No, No),
        '│' => a(No, No, Light, Light),
        '┃' => a(No, No, Heavy, Heavy),
        '┌' => a(No, Light, No, Light),
        '┐' => a(Light, No, No, Light),
        '└' => a(No, Light, Light, No),
        '┘' => a(Light, No, Light, No),
        '├' => a(No, Light, Light, Light),
        '┤' => a(Light, No, Light, Light),
        '┬' => a(Light, Light, No, Light),
        '┴' => a(Light, Light, Light, No),
        '┼' => a(Light, Light, Light, Light),
        '┏' => a(No, Heavy, No, Heavy),
        '┓' => a(Heavy, No, No, Heavy),
        '┗' => a(No, Heavy, Heavy, No),
        '┛' => a(Heavy, No, Heavy, No),
        '┣' => a(No, Heavy, Heavy, Heavy),
        '┫' => a(Heavy, No, Heavy, Heavy),
        '┳' => a(Heavy, Heavy, No, Heavy),
        '┻' => a(Heavy, Heavy, Heavy, No),
        '╋' => a(Heavy, Heavy, Heavy, Heavy),
        '═' => a(Double, Double, No, No),
        '║' => a(No, No, Double, Double),
        '╔' => a(No, Double, No, Double),
        '╗' => a(Double, No, No, Double),
        '╚' => a(No, Double, Double, No),
        '╝' => a(Double, No, Double, No),
        '╠' => a(No, Double, Double, Double),
        '╣' => a(Double, No, Double, Double),
        '╦' => a(Double, Double, No, Double),
        '╩' => a(Double, Double, Double, No),
        '╬' => a(Double, Double, Double, Double),
        '╴' => a(Light, No, No, No),
        '╵' => a(No, No, Light, No),
        '╶' => a(No, Light, No, No),
        '╷' => a(No, No, No, Light),
        '╸' => a(Heavy, No, No, No),
        '╹' => a(No, No, Heavy, No),
        '╺' => a(No, Heavy, No, No),
        '╻' => a(No, No, No, Heavy),
        '╭' => Some(Glyph::Round { right: true, down: true }),
        '╮' => Some(Glyph::Round { right: false, down: true }),
        '╯' => Some(Glyph::Round { right: false, down: false }),
        '╰' => Some(Glyph::Round { right: true, down: false }),
        '█' => b(FULL),
        '▀' => b(&[(0.0, 0.0, 1.0, 0.5)]),
        '▄' => b(&[(0.0, 0.5, 1.0, 1.0)]),
        '▌' => b(&[(0.0, 0.0, 0.5, 1.0)]),
        '▐' => b(&[(0.5, 0.0, 1.0, 1.0)]),
        '▔' => b(&[(0.0, 0.0, 1.0, 0.125)]),
        '▁' => b(&[(0.0, 0.875, 1.0, 1.0)]),
        '▂' => b(&[(0.0, 0.75, 1.0, 1.0)]),
        '▃' => b(&[(0.0, 0.625, 1.0, 1.0)]),
        '▅' => b(&[(0.0, 0.375, 1.0, 1.0)]),
        '▆' => b(&[(0.0, 0.25, 1.0, 1.0)]),
        '▇' => b(&[(0.0, 0.125, 1.0, 1.0)]),
        '▉' => b(&[(0.0, 0.0, 0.875, 1.0)]),
        '▊' => b(&[(0.0, 0.0, 0.75, 1.0)]),
        '▋' => b(&[(0.0, 0.0, 0.625, 1.0)]),
        '▍' => b(&[(0.0, 0.0, 0.375, 1.0)]),
        '▎' => b(&[(0.0, 0.0, 0.25, 1.0)]),
        '▏' => b(&[(0.0, 0.0, 0.125, 1.0)]),
        '▕' => b(&[(0.875, 0.0, 1.0, 1.0)]),
        '▘' => b(&[Q[0]]),
        '▝' => b(&[Q[1]]),
        '▖' => b(&[Q[2]]),
        '▗' => b(&[Q[3]]),
        '▚' => b(&[Q[0], Q[3]]),
        '▞' => b(&[Q[1], Q[2]]),
        '▛' => b(&[Q[0], Q[1], Q[2]]),
        '▜' => b(&[Q[0], Q[1], Q[3]]),
        '▙' => b(&[Q[0], Q[2], Q[3]]),
        '▟' => b(&[Q[1], Q[2], Q[3]]),
        '░' => Some(Glyph::Blocks(FULL, 0.25)),
        '▒' => Some(Glyph::Blocks(FULL, 0.5)),
        '▓' => Some(Glyph::Blocks(FULL, 0.75)),
        _ => None,
    }
}

// the quadrants: top left, top right, bottom left, bottom right
const Q: [(f32, f32, f32, f32); 4] = [(0.0, 0.0, 0.5, 0.5), (0.5, 0.0, 1.0, 0.5), (0.0, 0.5, 0.5, 1.0), (0.5, 0.5, 1.0, 1.0)];

/// the rectangles of a glyph's arms in a cell at (x, y) of w by h, in pixels (x0, y0, x1, y1);
/// `t` is a light line's thickness, a heavy one twice that, a double two light ones `t` apart
pub fn arms(a: [W; 4], x: f32, y: f32, w: f32, h: f32, t: f32) -> Vec<(f32, f32, f32, f32)> {
    let (cx, cy) = ((x + w / 2.0 - t / 2.0).round(), (y + h / 2.0 - t / 2.0).round());
    let mut out = Vec::new();
    // each line of an arm: its offset across the arm and its thickness
    let lines = |k: W| -> &'static [(f32, f32)] {
        match k {
            No => &[],
            Light => &[(0.0, 1.0)],
            Heavy => &[(-0.5, 2.0)],
            Double => &[(-1.0, 1.0), (1.0, 1.0)],
        }
    };
    // a double arm meeting a crossing double arm stops at the near line, so the gaps stay open
    let reach = |across: [W; 2]| if across.contains(&Double) { 1.0 } else { 0.0 };
    let [l, r, u, d] = a;
    for (o, th) in lines(l) {
        out.push((x, cy + o * t, cx + t + reach([u, d]) * t, cy + (o + th) * t));
    }
    for (o, th) in lines(r) {
        out.push((cx - reach([u, d]) * t, cy + o * t, x + w, cy + (o + th) * t));
    }
    for (o, th) in lines(u) {
        out.push((cx + o * t, y, cx + (o + th) * t, cy + t + reach([l, r]) * t));
    }
    for (o, th) in lines(d) {
        out.push((cx + o * t, cy - reach([l, r]) * t, cx + (o + th) * t, y + h));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_reach_the_edges() {
        // │ spans the whole cell's height, ─ its width: that is what makes them meet
        let Some(Glyph::Arms(v)) = glyph('│') else { panic!() };
        let r = arms(v, 10.0, 20.0, 8.0, 15.0, 1.0);
        assert!(r.iter().any(|q| q.1 == 20.0) && r.iter().any(|q| q.3 == 35.0));
        assert!(r.iter().all(|q| q.2 - q.0 == 1.0));
        let Some(Glyph::Arms(h)) = glyph('─') else { panic!() };
        let r = arms(h, 10.0, 20.0, 8.0, 15.0, 1.0);
        assert!(r.iter().any(|q| q.0 == 10.0) && r.iter().any(|q| q.2 == 18.0));
        assert_eq!(glyph('a'), None);
        assert!(matches!(glyph('▛'), Some(Glyph::Blocks(q, _)) if q.len() == 3));
    }
}
