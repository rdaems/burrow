//! The window's outside with its artillery duel (web/outside.js) and ping-pong (web/pong.js), drawn into pix
//! images at the canvases' own sizes. No clock and no randomness of their own: each game keeps its time from
//! `tick(dt)` and draws from a seeded generator, so a test can play it back. What the page drew as HTML (the
//! duel's line, the ping-pong card) is data here, for the app to set in its own lettering.

pub mod outside;
pub mod pong;

use pix::Color;

/// pixel.js's palette (`pixelAvatar.C`), the part these two use
pub(crate) mod c {
    use pix::Color;
    pub const INK: Color = Color::rgb(0x140f1e);
    pub const INK2: Color = Color::rgb(0x2a2140);
    pub const STEEL0: Color = Color::rgb(0x3b3a55);
    pub const STEEL1: Color = Color::rgb(0x5d6280);
    pub const STEEL2: Color = Color::rgb(0x8b93ad);
    pub const STEEL3: Color = Color::rgb(0xc2c9d9);
    pub const STEEL4: Color = Color::rgb(0xf4f6fb);
    pub const WHITE: Color = Color::rgb(0xffffff);
    pub const GOLD1: Color = Color::rgb(0xf2a531);
    pub const GOLD2: Color = Color::rgb(0xffe27a);
    pub const RED1: Color = Color::rgb(0xe8402a);
    pub const SCRUB1: Color = Color::rgb(0x1b6660);
}

/// `Math.round`: halves go up (Rust's `round` takes them away from zero)
#[inline]
pub(crate) fn jround(v: f64) -> f64 {
    (v + 0.5).floor()
}

#[inline]
pub(crate) fn clamp(v: f64, a: f64, b: f64) -> f64 {
    a.max(b.min(v))
}

/// `Math.random()` with a seed: splitmix64
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    /// 0 <= r < 1
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + self.next() * (b - a)
    }
    /// a normal deviate (Box-Muller)
    pub fn gauss(&mut self) -> f64 {
        let u = self.next();
        let v = self.next();
        (-2.0 * (1.0 - u).ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
    }
}

/// someone at a game: a bot, or you
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: String,
    pub name: String,
    /// a bot's project colour; yours
    pub color: Color,
    pub human: bool,
    /// what your portrait wears (room.js `wears(seed)`), for your gunner outside; none: the default person
    pub wears: Option<Wears>,
}

impl Player {
    pub fn bot(id: &str, name: &str, color: Color) -> Player {
        Player {
            id: id.into(),
            name: name.into(),
            color,
            human: false,
            wears: None,
        }
    }
    pub fn you(name: &str, color: Color, wears: Option<Wears>) -> Player {
        Player {
            id: "you".into(),
            name: name.into(),
            color,
            human: true,
            wears,
        }
    }
}

/// the colours of a portrait, for a small figure of that person
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wears {
    pub shirt: Color,
    pub shade: Color,
    pub skin: Color,
    pub hair: Option<Color>,
}

/// a sound to make (burrow-sound's names), left or right by where it happens (-1..1)
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// `id` is for a `Stop` later (only the whistle gets one)
    Sound {
        name: &'static str,
        pan: f64,
        dur: Option<f64>,
        id: u32,
    },
    Stop(u32),
}
