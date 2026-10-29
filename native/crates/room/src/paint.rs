//! The bridge to the painters (`draw`): what the room's logic needs of the pictures. Filled in once the
//! painters are ported.

use pix::Color;

/// how far above its anchor a kind of piece is drawn (its topmost opaque row)
pub fn vis_top(kind: &str) -> i32 {
    crate::layout::foot(kind, [0, 0], None)[1]
}

/// the whiteboard dream's pixels, in the order they come in
pub fn dream_scene(_i: usize) -> Vec<(i32, i32, Color)> {
    Vec::new()
}

/// a picture washed in a colour, its own pixels only (the danger's red over what is held over the fire)
pub fn tinted(im: &pix::Image, col: Color, a: f64) -> pix::Image {
    let mut out = im.clone();
    let mut x = out.ctx();
    x.set_op(pix::Op::SourceAtop);
    x.set_alpha(a);
    x.fill_rect(0., 0., im.w as f64, im.h as f64, col);
    out
}

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// a figure going up in the booth's light: a front of light rises from its feet (d 0 to 1); what it has passed
/// turns to light and thins out in an ordered dither until nothing is left
pub fn dissolve(im: &pix::Image, d: f64) -> pix::Image {
    let mut out = im.clone();
    if d <= 0.0 {
        return out;
    }
    let (top, foot) = (3.0, (im.h - 1) as f64);
    for j in 0..im.h as i32 {
        let a = ((d * 1.4 - (foot - j as f64) / (foot - top)) / 0.4).clamp(0.0, 1.0);
        if a == 0.0 {
            continue;
        }
        for i in 0..im.w as i32 {
            let mut p = out.get(i, j);
            if p[3] == 0 {
                continue;
            }
            if (BAYER[(j % 4) as usize][(i % 4) as usize] as f64) / 16.0 < a {
                p[3] = 0;
                out.set(i, j, p);
                continue;
            }
            let m = (a * 1.6).min(1.0) * 0.85;
            let mixc = |v: u8, to: f64| (v as f64 + (to - v as f64) * m).round() as u8;
            out.set(i, j, [mixc(p[0], 255.0), mixc(p[1], 244.0), mixc(p[2], 214.0), p[3]]);
        }
    }
    out
}

/// a picture of a piece as it stands (2 dots of padding), for the fire
pub fn snapshot_piece(_room: &crate::room::Room, _key: &str, _at: [i32; 2], _u: Option<i32>, r: crate::layout::Rect, _inp: &crate::io::Input) -> pix::Image {
    pix::Image::new((r[2] - r[0] + 4).max(1) as u32, (r[3] - r[1] + 4).max(1) as u32)
}
