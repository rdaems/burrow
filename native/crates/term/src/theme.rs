//! the terminal's colours: DARK and LIGHT from web/index.html, xterm's 256 colours past the first 16

use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

/// light or dark after the theme Claude Code is set to on that machine
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

pub struct Palette {
    pub background: u32,
    pub foreground: u32,
    pub cursor: u32,
    /// rgba: drawn over the cells
    pub selection: u32,
    pub ansi: [u32; 16],
}

pub const DARK: Palette = Palette {
    background: 0x1a1626,
    foreground: 0xe6e1ef,
    cursor: 0xe6e1ef,
    selection: 0xd9775766,
    ansi: [
        0x2a2140, 0xe8402a, 0x5fcf4a, 0xf2a531, 0x4a76b8, 0x9a74d6, 0x2fa087, 0xc2c9d9, //
        0x5d6280, 0xff6b5a, 0x8ee07a, 0xffe27a, 0x8fc0e8, 0xc4a8f0, 0x7fdcb0, 0xf4f6fb,
    ],
};

pub const LIGHT: Palette = Palette {
    background: 0xfbf8f1,
    foreground: 0x140f1e,
    cursor: 0x140f1e,
    selection: 0xd9775755,
    ansi: [
        0x140f1e, 0xb8321f, 0x2d8a3a, 0xa8650f, 0x2c4580, 0x5a3f94, 0x1b6660, 0x8b93ad, //
        0x5d6280, 0xe8402a, 0x3f9e2f, 0xb86a1e, 0x4a76b8, 0x9a74d6, 0x2fa087, 0x3b3a55,
    ],
};

impl Theme {
    pub fn palette(self) -> &'static Palette {
        match self {
            Theme::Dark => &DARK,
            Theme::Light => &LIGHT,
        }
    }
}

pub fn rgb(c: u32) -> Rgb {
    Rgb { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8 }
}

impl Palette {
    /// xterm's 256 colours, the first 16 the theme's
    pub fn indexed(&self, i: usize) -> Rgb {
        match i {
            0..=15 => rgb(self.ansi[i]),
            16..=231 => {
                let i = i - 16;
                let v = |k: usize| if k == 0 { 0 } else { (55 + k * 40) as u8 };
                Rgb { r: v(i / 36), g: v((i / 6) % 6), b: v(i % 6) }
            }
            232..=255 => {
                let g = (8 + (i - 232) * 10) as u8;
                Rgb { r: g, g, b: g }
            }
            _ => rgb(self.foreground),
        }
    }

    /// a colour as the program asked for it; `colors` holds what it set itself (OSC 4, 10, 11)
    pub fn resolve(&self, c: Color, colors: &Colors) -> Rgb {
        match c {
            Color::Spec(rgb) => rgb,
            Color::Indexed(i) => colors[i as usize].unwrap_or_else(|| self.indexed(i as usize)),
            Color::Named(n) => colors[n].unwrap_or_else(|| self.named(n)),
        }
    }

    pub fn named(&self, n: NamedColor) -> Rgb {
        match n {
            NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => rgb(self.foreground),
            NamedColor::Background => rgb(self.background),
            NamedColor::Cursor => rgb(self.cursor),
            n => {
                let i = n as usize;
                if i < 16 {
                    self.indexed(i)
                } else if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize).contains(&i) {
                    self.indexed(i - NamedColor::DimBlack as usize)
                } else {
                    rgb(self.foreground)
                }
            }
        }
    }

    /// the answer to "what is colour i?" (OSC 4/10/11/12 queries)
    pub fn answer(&self, i: usize, colors: &Colors) -> Rgb {
        if let Some(c) = colors[i] {
            return c;
        }
        match i {
            256 => rgb(self.foreground),
            257 => rgb(self.background),
            258 => rgb(self.cursor),
            i => self.indexed(i),
        }
    }
}

/// a colour `a` of the way from `bg` to `fg` (dim text is drawn half-transparent, as xterm.js does)
pub fn mix(fg: Rgb, bg: Rgb, a: f32) -> Rgb {
    let m = |f: u8, b: u8| (b as f32 + (f as f32 - b as f32) * a).round() as u8;
    Rgb { r: m(fg.r, bg.r), g: m(fg.g, bg.g), b: m(fg.b, bg.b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_256_colours() {
        let p = Theme::Dark.palette();
        assert_eq!(p.indexed(1), rgb(0xe8402a));
        assert_eq!(p.indexed(16), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(p.indexed(196), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(p.indexed(231), Rgb { r: 255, g: 255, b: 255 });
        assert_eq!(p.indexed(232), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(p.indexed(255), Rgb { r: 238, g: 238, b: 238 });
        assert_eq!(Theme::Light.palette().named(NamedColor::Background), rgb(0xfbf8f1));
    }
}
