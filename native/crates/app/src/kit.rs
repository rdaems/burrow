//! The page's 16-bit chrome (web/burrow.css after pair's door.css, web/pixel.js's dithers), for GPUI: panels with
//! an ink ring, two bevels and a dithered fill, steel buttons, fields, the tiles' bands. What has pixels in it
//! (bevels, dithers, clipped corners) is drawn with `pix` into a picture the size of the element, at the screen's
//! own pixels, and cached by size; text is GPUI's, in the pixel font or the terminal's.

use gpui::{App, Bounds, Corners, Div, ElementId, FontWeight, Hsla, IntoElement, ParentElement, Pixels, RenderImage, SharedString, Styled, Window, canvas, div, prelude::*, px, rgb};
use pix::{Color, Image};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// the page's colours (burrow.css :root, pixel.js)
pub mod col {
    pub const NIGHT: u32 = 0x0b0812;
    pub const INK: u32 = 0x140f1e;
    pub const INK2: u32 = 0x2a2140;
    pub const BOX: u32 = 0x1a1626;
    pub const BOX2: u32 = 0x221d32;
    pub const LINE: u32 = 0x3b3a55;
    pub const CODE: u32 = 0x100c19;
    pub const TEXT: u32 = 0xe8e5ef;
    pub const DIM: u32 = 0x8b93ad;
    pub const FAINT: u32 = 0x5d6280;
    pub const ME: u32 = 0xd97757;
    pub const STEEL0: u32 = 0x3b3a55;
    pub const STEEL1: u32 = 0x5d6280;
    pub const STEEL2: u32 = 0x8b93ad;
    pub const STEEL3: u32 = 0xc2c9d9;
    pub const STEEL4: u32 = 0xf4f6fb;
    pub const WHITE: u32 = 0xffffff;
    pub const GOLD0: u32 = 0xb86a1e;
    pub const GOLD1: u32 = 0xf2a531;
    pub const GOLD2: u32 = 0xffe27a;
    pub const BLUE0: u32 = 0x1a2448;
    pub const BLUE1: u32 = 0x2c4580;
    pub const BLUE2: u32 = 0x4a76b8;
    pub const SCRUB1: u32 = 0x1b6660;
    pub const SCRUB2: u32 = 0x2fa087;
    pub const TISSUE2: u32 = 0xf06b72;
    /// the focused tile's pastel gold, and the room's hover colour
    pub const PALE0: u32 = 0xf6c784;
    pub const PALE1: u32 = 0xffe9a4;
}

pub const PIXEL: &str = "Pair Pixel";
pub const MONO: &str = "JetBrainsMono NF";

fn c(v: u32) -> Color {
    Color::rgb(v)
}

/// pixel.js's dithered fills: solid `c0` on top, Bayer-dithering into `c1` downwards from `from` percent, in three
/// bands of 2x2 dots (a quarter, half, three quarters of `c1`); `w` x `h` CSS pixels
pub fn dither(w: u32, h: u32, c0: u32, c1: u32, from: f32) -> Image {
    let mut im = Image::filled(w, h, c(c0));
    let band = (100.0 - from) / 3.0;
    for y in 0..h {
        let pct = y as f32 / h.max(1) as f32 * 100.0;
        if pct < from {
            continue;
        }
        let k = (((pct - from) / band).floor() as i32).clamp(0, 2);
        for x in 0..w {
            let (cx, cy) = ((x % 4) / 2, (y % 4) / 2);
            let on = match k {
                0 => cx == 0 && cy == 0,
                1 => (cx == 0 && cy == 0) || (cx == 1 && cy == 1),
                _ => !(cx == 0 && cy == 1),
            };
            if on {
                im.set(x as i32, y as i32, c(c1).0);
            }
        }
    }
    im
}

/// the fills, by name
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fill {
    /// the panels' blue
    Window,
    /// steel buttons
    Button,
    /// a hovered button
    Hot,
    /// the focused tile's band, a pressed choice
    Gold,
    Green,
    Solid(u32),
}

impl Fill {
    pub fn image(self, w: u32, h: u32) -> Image {
        match self {
            Fill::Window => dither(w, h, col::BLUE1, col::BLUE0, 40.0),
            Fill::Button => dither(w, h, col::STEEL4, col::STEEL3, 30.0),
            Fill::Hot => dither(w, h, col::GOLD2, col::GOLD1, 30.0),
            Fill::Gold => dither(w, h, 0xffe9a4, 0xf6c784, 30.0),
            Fill::Green => dither(w, h, col::SCRUB2, col::SCRUB1, 30.0),
            Fill::Solid(v) => Image::filled(w, h, c(v)),
        }
    }
}

/// a frame: an ink border, rings of bevel inside it (top-left colour, bottom-right colour, each `2` CSS px), a fill,
/// the corners clipped
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Frame {
    pub border: Option<u32>,
    /// (top-left, bottom-right) per ring, outside in
    pub rings: Vec<(u32, u32)>,
    pub fill: Fill,
    /// the band on top (a tile's), its height in CSS px and its fill
    pub band: Option<(u32, Fill)>,
    /// space between the rings and the fill (a tile's padding: the ring's colour shows... no: the box)
    pub pad: u32,
    pub pad_fill: Option<u32>,
    pub clip: bool,
    /// a glow ring outside (a focused field)
    pub glow: Option<u32>,
}

impl Frame {
    pub fn panel() -> Frame {
        // .pn.window: ink, then steel4/steel2, steel3/steel1, an ink ring, the blue
        Frame { border: Some(col::INK), rings: vec![(col::STEEL4, col::STEEL2), (col::STEEL3, col::STEEL1), (col::INK, col::INK)], fill: Fill::Window, band: None, pad: 0, pad_fill: None, clip: true, glow: None }
    }
    pub fn button(fill: Fill, tl: u32, br: u32) -> Frame {
        Frame { border: Some(col::INK), rings: vec![(tl, br)], fill, band: None, pad: 0, pad_fill: None, clip: true, glow: None }
    }
    pub fn field(focused: bool) -> Frame {
        Frame { border: Some(col::INK), rings: vec![(col::STEEL2, col::STEEL4)], fill: Fill::Solid(col::STEEL4), band: None, pad: 0, pad_fill: None, clip: false, glow: focused.then_some(col::GOLD1) }
    }
    /// a tile: an ink ring with a steel bevel (pastel gold focused), padding on the box colour, its band
    pub fn tile(lit: bool) -> Frame {
        let (tl, br) = if lit { (col::PALE1, col::PALE0) } else { (col::STEEL3, col::STEEL1) };
        Frame {
            border: Some(col::INK),
            rings: vec![(tl, br), (col::INK, col::INK)],
            fill: Fill::Solid(col::BOX),
            band: Some((28, if lit { Fill::Gold } else { Fill::Window })),
            pad: 0,
            pad_fill: None,
            clip: true,
            glow: None,
        }
    }

    /// the frame drawn `w` x `h` CSS px
    pub fn image(&self, w: u32, h: u32) -> Image {
        let (wi, hi) = (w as i32, h as i32);
        let mut im = Image::new(w, h);
        let g = if self.glow.is_some() { 2 } else { 0 };
        {
            let mut x = im.ctx();
            if let Some(gc) = self.glow {
                x.rect(0, 0, wi, hi, c(gc));
            }
            let mut d = g;
            if let Some(b) = self.border {
                x.rect(d, d, wi - 2 * d, hi - 2 * d, c(b));
                d += 2;
            }
            // the fill first, then the rings over its edge, outside in
            let rings = self.rings.len() as i32 * 2;
            let (fx, fy, fw, fh) = (d + rings, d + rings, wi - 2 * (d + rings), hi - 2 * (d + rings));
            if fw > 0 && fh > 0 {
                let fill = self.fill.image(fw as u32, fh as u32);
                x.draw_image(&fill, fx as f64, fy as f64);
                if let Some((bh, bf)) = self.band {
                    let band = bf.image(fw as u32, (bh as i32).min(fh) as u32);
                    x.draw_image(&band, fx as f64, fy as f64);
                }
            }
            for (k, (tl, br)) in self.rings.iter().enumerate() {
                let o = d + k as i32 * 2;
                let (rw, rh) = (wi - 2 * o, hi - 2 * o);
                // bottom-right first: the top-left one is over it where they meet
                x.rect(o, o + rh - 2, rw, 2, c(*br));
                x.rect(o + rw - 2, o, 2, rh, c(*br));
                x.rect(o, o, rw, 2, c(*tl));
                x.rect(o, o, 2, rh, c(*tl));
            }
        }
        if self.clip {
            // clip-path: a 2 px corner cut
            let o = g;
            for (cx, cy) in [(o, o), (wi - 1 - o, o), (o, hi - 1 - o), (wi - 1 - o, hi - 1 - o)] {
                im.set(cx, cy, [0; 4]);
            }
        }
        im
    }
}

thread_local! {
    static PICS: RefCell<HashMap<(Frame, u32, u32, u32), Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// a pix image as a GPUI picture at `scale` device pixels a CSS pixel (whole pixels, crisp)
pub fn picture(im: &Image, scale: u32) -> Arc<RenderImage> {
    let bgra = premultiplied_bgra(im, scale);
    let buf = image::RgbaImage::from_raw(im.w * scale, im.h * scale, bgra).expect("a picture");
    Arc::new(RenderImage::new(vec![image::Frame::new(buf)]))
}

/// an opaque picture (the room, Doom): no alpha to premultiply
pub fn picture_opaque(im: &Image, scale: u32) -> Arc<RenderImage> {
    let bgra = im.upscale_bgra(scale, [0, 0, 0, 255]);
    let buf = image::RgbaImage::from_raw(im.w * scale, im.h * scale, bgra).expect("a picture");
    Arc::new(RenderImage::new(vec![image::Frame::new(buf)]))
}

/// GPUI wants premultiplied BGRA
fn premultiplied_bgra(im: &Image, k: u32) -> Vec<u8> {
    let (w, h) = (im.w * k, im.h * k);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let p = im.px[((y / k) * im.w + x / k) as usize];
            let a = p[3] as u32;
            let i = ((y * w + x) * 4) as usize;
            out[i] = (p[2] as u32 * a / 255) as u8;
            out[i + 1] = (p[1] as u32 * a / 255) as u8;
            out[i + 2] = (p[0] as u32 * a / 255) as u8;
            out[i + 3] = p[3];
        }
    }
    out
}

/// a frame behind an element, filling it: drawn at its size, at the screen's pixels, kept for the next time
pub fn frame_bg(f: Frame) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |b: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let scale = window.scale_factor().round().max(1.0) as u32;
            let (w, h) = (f32::from(b.size.width).round() as u32, f32::from(b.size.height).round() as u32);
            if w == 0 || h == 0 {
                return;
            }
            let key = (f.clone(), w, h, scale);
            let pic = PICS.with(|p| {
                let mut p = p.borrow_mut();
                if p.len() > 400 {
                    p.clear(); // (sizes come and go as windows resize)
                }
                p.entry(key).or_insert_with(|| picture(&f.image(w, h), scale)).clone()
            });
            let _ = window.paint_image(b, Corners::default(), pic, 0, false);
        },
    )
    .absolute()
    .size_full()
}

/// text in the pixel font, 16 px on 22, with the page's ink shadow (2 px down and right) when `shadow`
pub fn pixel_text(s: impl Into<SharedString>, color: u32, shadow: bool) -> Div {
    let s: SharedString = s.into();
    let t = div().font_family(PIXEL).text_size(px(16.)).line_height(px(22.)).whitespace_nowrap();
    if !shadow {
        return t.text_color(rgb(color)).child(s);
    }
    t.relative()
        .child(div().absolute().left(px(2.)).top(px(2.)).text_color(rgb(col::INK)).child(s.clone()))
        .child(div().relative().text_color(rgb(color)).child(s))
}

/// text in the terminal's font (paths, flags: the pixel font has no ~)
pub fn mono_text(s: impl Into<SharedString>, color: u32, size: f32) -> Div {
    div().font_family(MONO).text_size(px(size)).text_color(rgb(color)).child(s.into())
}

/// how a steel button looks
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Plain,
    Hover,
    Pressed,
    /// the choice that holds: pressed in, gold
    On,
}

pub fn button_frame(look: Look) -> Frame {
    match look {
        Look::Plain => Frame::button(Fill::Button, col::WHITE, col::STEEL1),
        Look::Hover => Frame::button(Fill::Hot, col::WHITE, col::GOLD0),
        Look::Pressed => Frame::button(Fill::Solid(col::STEEL2), col::STEEL1, col::WHITE),
        Look::On => Frame::button(Fill::Gold, col::STEEL1, col::WHITE),
    }
}

/// a steel button with a label in the pixel font; `small` for a band's ✕
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, on: bool, small: bool) -> gpui::Stateful<Div> {
    let label: SharedString = label.into();
    let (py, px_) = if small { (1.0, 8.0) } else { (5.0, 12.0) };
    div()
        .id(id)
        .relative()
        .flex_none()
        .cursor_pointer()
        .child(frame_bg(button_frame(if on { Look::On } else { Look::Plain })))
        .child(div().relative().px(px(px_ + 2.)).py(px(py + 2.)).child(pixel_text(label, col::INK, false)))
}

/// a hairline of `Hsla`
pub fn hsla(v: u32) -> Hsla {
    rgb(v).into()
}

pub fn bold() -> FontWeight {
    FontWeight::BOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dithers_go_from_solid_to_three_quarters() {
        let im = dither(40, 100, 0x000000, 0xffffff, 40.0);
        let share = |y0: i32, y1: i32| {
            let mut n = 0;
            for y in y0..y1 {
                for x in 0..40 {
                    if im.get(x, y)[0] == 255 {
                        n += 1;
                    }
                }
            }
            n as f64 / ((y1 - y0) * 40) as f64
        };
        assert_eq!(share(0, 40), 0.0);
        assert!((share(40, 60) - 0.25).abs() < 0.05);
        assert!((share(84, 100) - 0.75).abs() < 0.05);
    }

    #[test]
    fn a_panel_has_its_rings() {
        let im = Frame::panel().image(40, 30);
        assert_eq!(im.get(0, 0)[3], 0, "the corner is cut");
        assert_eq!(im.get(1, 5), Color::rgb(col::INK).0, "the border");
        assert_eq!(im.get(3, 5), Color::rgb(col::STEEL4).0, "the light ring, left");
        assert_eq!(im.get(36, 5), Color::rgb(col::STEEL2).0, "the light ring's shadow, right");
        assert_eq!(im.get(5, 10), Color::rgb(col::STEEL3).0);
        assert_eq!(im.get(7, 10), Color::rgb(col::INK).0, "the inner ink ring");
        assert_eq!(im.get(10, 10), Color::rgb(col::BLUE1).0, "the blue");
    }
}
