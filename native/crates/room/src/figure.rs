//! Figures: pixel.js's head on floor.js's body, drawn once per pose with an ink outline; the bots are
//! robots, you are a person (pair's human figure).

use crate::avatar::{self, c};
use pix::{Color, Image};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const SW: i32 = 22;
pub const SH: i32 = 29;

/// the colours of a rolled-up mat (and the mat itself)
pub mod mat {
    use pix::Color;
    pub const HI: Color = Color::rgb(0x4d5866);
    pub const TOP: Color = Color::rgb(0x394449);
    pub const D: Color = Color::rgb(0x2b3238);
    pub const K: Color = Color::rgb(0x3f3740);
}

/// what the arms do: down sw1 sw2 up wave fwd1 fwd2 reach mug gun paper book can roll
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Arms {
    #[default]
    Down,
    Sw1,
    Sw2,
    Up,
    Wave,
    Fwd1,
    Fwd2,
    Reach,
    Paint1,
    Paint2,
    Phone,
    Mug,
    Gun,
    Paper,
    Book,
    Can,
    Roll,
}

/// legs: stand walk1 walk2 sit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Legs {
    #[default]
    Stand,
    Walk1,
    Walk2,
    Sit,
}

impl Arms {
    pub fn parse(s: &str) -> Arms {
        match s {
            "sw1" => Arms::Sw1,
            "sw2" => Arms::Sw2,
            "up" => Arms::Up,
            "wave" => Arms::Wave,
            "fwd1" => Arms::Fwd1,
            "fwd2" => Arms::Fwd2,
            "reach" => Arms::Reach,
            "phone" => Arms::Phone,
            "paint1" => Arms::Paint1,
            "paint2" => Arms::Paint2,
            "mug" => Arms::Mug,
            "gun" => Arms::Gun,
            "paper" => Arms::Paper,
            "book" => Arms::Book,
            "can" => Arms::Can,
            "roll" => Arms::Roll,
            _ => Arms::Down,
        }
    }
}
impl Legs {
    pub fn parse(s: &str) -> Legs {
        match s {
            "walk1" => Legs::Walk1,
            "walk2" => Legs::Walk2,
            "sit" => Legs::Sit,
            _ => Legs::Stand,
        }
    }
}

type Key = (String, Color, Arms, Legs, bool, bool, bool);
thread_local! {
    static BODIES: RefCell<HashMap<Key, Rc<Image>>> = RefCell::new(HashMap::new());
}

/// the portrait's pixel at x, y, if it is solid
fn at(face: &Image, x: i32, y: i32) -> Option<Color> {
    let p = face.get(x, y);
    (p[3] >= 128).then_some(Color([p[0], p[1], p[2], 255]))
}

/// a full figure: `human` a person (else a robot), `femme` the office manager (a bow on her head, a skirt)
pub fn body(seed: &str, color: Color, arms: Arms, legs: Legs, human: bool, femme: bool) -> Rc<Image> {
    body_red(seed, color, arms, legs, human, femme, false)
}

/// the same, `red`: a robot with its eyes red (a mutineer, gun in hand)
pub fn body_red(seed: &str, color: Color, arms: Arms, legs: Legs, human: bool, femme: bool, red: bool) -> Rc<Image> {
    let key = (seed.to_string(), color, arms, legs, human, femme, red);
    if let Some(im) = BODIES.with(|b| b.borrow().get(&key).cloned()) {
        return im;
    }
    let f = avatar::avatar(seed, !human, Some(color));
    let mut px: Vec<Option<Color>> = vec![None; (SW * SH) as usize];
    let mut line = vec![false; (SW * SH) as usize];
    let idx = |x: i32, y: i32| ((y + 1) * SW + x + 1) as usize;
    let set = |px: &mut Vec<Option<Color>>, x: i32, y: i32, c: Color| {
        let i = (y + 1) * SW + x + 1;
        if i >= 0 && (i as usize) < px.len() {
            px[i as usize] = Some(c);
        }
    };
    let rect = |px: &mut Vec<Option<Color>>, x: i32, y: i32, w: i32, h: i32, c: Color| {
        for j in 0..h {
            for i in 0..w {
                let k = (y + j + 1) * SW + x + i + 1;
                if k >= 0 && (k as usize) < px.len() {
                    px[k as usize] = Some(c);
                }
            }
        }
    };
    let cut = if human { 14 } else { 16 }; // the portrait down to the chin (a robot's down to its neck)
    for y in 0..=cut {
        for x in 0..20 {
            if let Some(col) = at(&f, x, y) {
                set(&mut px, x, y, col);
                if col == c::INK {
                    line[idx(x, y)] = true;
                }
            }
        }
    }
    if red && !human {
        // the eyes: what is not screen on its face's screen (pixel.js robot(): x 7..12, four rows under its top)
        let top = avatar::robot_parts(seed).top + 6;
        let scr = at(&f, 7, top);
        let lum = |c: Color| c.0[0] as f64 * 0.3 + c.0[1] as f64 * 0.59 + c.0[2] as f64 * 0.11;
        let mut eyes = Vec::new();
        for y in top..top + 4 {
            for x in 7..13 {
                if let Some(c) = at(&f, x, y).filter(|&c| Some(c) != scr) {
                    eyes.push((x, y, lum(c)));
                }
            }
        }
        let hi = eyes.iter().map(|e| e.2).fold(f64::NEG_INFINITY, f64::max);
        for (x, y, l) in eyes {
            set(&mut px, x, y, if l >= hi * 0.85 { Color::rgb(0xff3b2f) } else { Color::rgb(0xa3170f) });
        }
    }
    // a person wears what their portrait wears (a coat is white, a suit grey): its shoulders' colours
    let c0 = if human { at(&f, 6, 19).unwrap_or(color) } else { c::STEEL3 };
    let c1 = if human { at(&f, 15, 19).filter(|&x| x != c0).unwrap_or(c0.shade(0.62)) } else { c::STEEL1 };
    let skin = if human { avatar::skin(seed) } else { c::STEEL3 };
    let leg = if human { c::INK2 } else { c::STEEL1 };
    let top = cut + 1;
    if human {
        rect(&mut px, 9, top, 2, 1, skin.shade(0.8));
    }
    let t0 = top + if human { 1 } else { 0 };
    rect(&mut px, 6, t0, 8, 22 - t0, c0);
    rect(&mut px, 12, t0, 2, 22 - t0, c1);
    if !human {
        set(&mut px, 9, top + 2, color);
        set(&mut px, 10, top + 2, color);
        set(&mut px, 9, top + 3, color.shade(0.62));
    }
    if femme {
        let mut ht = 0;
        while ht < cut && at(&f, 7, ht).is_none() {
            ht += 1; // the top of her head (her antenna stands further right)
        }
        let (bow, knot) = (color, color.shade(0.62));
        rect(&mut px, 4, ht - 3, 2, 3, bow);
        rect(&mut px, 7, ht - 3, 2, 3, bow);
        set(&mut px, 6, ht - 2, knot);
        rect(&mut px, 5, 18, 10, 3, bow); // the skirt
        rect(&mut px, 5, 20, 10, 1, knot);
    }
    let arm = |px: &mut Vec<Option<Color>>, x: i32, y0: i32, y1: i32, hx: i32, hy: i32| {
        rect(px, x, y0, 2, y1 - y0 + 1, c1);
        rect(px, hx, hy, 2, 1, skin);
    };
    let back = |px: &mut Vec<Option<Color>>| arm(px, 4, 17, 20, 4, 21);
    let fwd = |px: &mut Vec<Option<Color>>, hy: i32, w: i32| {
        rect(px, 14, hy - 1, w, 2, c1);
        rect(px, 14 + w, hy - 1, 2, 2, skin);
    };
    match arms {
        Arms::Up => {
            arm(&mut px, 4, 12, 16, 4, 11);
            arm(&mut px, 14, 12, 16, 14, 11);
        }
        Arms::Wave => {
            back(&mut px);
            arm(&mut px, 14, 11, 16, 15, 10);
        }
        Arms::Sw1 => {
            arm(&mut px, 4, 17, 19, 5, 20);
            arm(&mut px, 14, 17, 19, 13, 20);
        }
        Arms::Sw2 => {
            arm(&mut px, 4, 17, 19, 3, 20);
            arm(&mut px, 14, 17, 19, 15, 20);
        }
        Arms::Fwd1 | Arms::Fwd2 => {
            rect(&mut px, 4, 17, 2, 2, c1);
            rect(&mut px, 4, 19, 2, 1, skin);
            fwd(&mut px, if arms == Arms::Fwd1 { 19 } else { 20 }, 3);
        }
        Arms::Reach => {
            back(&mut px);
            rect(&mut px, 14, 14, 2, 4, c1);
            rect(&mut px, 16, 13, 2, 2, c1);
            rect(&mut px, 18, 12, 2, 2, skin);
        }
        Arms::Paint1 | Arms::Paint2 => {
            // a palette on the back hand, a brush in the front one, up at the canvas and down
            let brush = Color::rgb(0x8a5a32);
            rect(&mut px, 4, 17, 2, 3, c1);
            rect(&mut px, 1, 19, 5, 3, Color::rgb(0xd8b07a));
            rect(&mut px, 2, 20, 1, 1, c::RED1);
            rect(&mut px, 3, 19, 1, 1, c::BLUE2);
            rect(&mut px, 4, 21, 1, 1, c::GOLD2);
            if arms == Arms::Paint1 {
                rect(&mut px, 14, 14, 2, 4, c1);
                rect(&mut px, 16, 13, 2, 2, c1);
                rect(&mut px, 18, 12, 2, 2, skin);
                rect(&mut px, 20, 11, 1, 1, brush);
            } else {
                rect(&mut px, 14, 16, 3, 2, c1);
                rect(&mut px, 17, 16, 2, 2, skin);
                rect(&mut px, 19, 15, 1, 1, brush);
                rect(&mut px, 20, 14, 1, 1, c::RED1);
            }
        }
        Arms::Phone => {
            // a handset held to the ear
            back(&mut px);
            let (d, hi) = (Color::rgb(0x595e60), Color::rgb(0xb5bfc7));
            rect(&mut px, 14, 13, 2, 5, c1);
            rect(&mut px, 16, 7, 3, 2, d);
            rect(&mut px, 16, 9, 2, 3, d);
            rect(&mut px, 16, 12, 3, 2, d);
            rect(&mut px, 17, 7, 1, 1, hi);
            rect(&mut px, 17, 12, 1, 1, hi);
            rect(&mut px, 15, 10, 2, 2, skin);
        }
        Arms::Mug => {
            back(&mut px);
            fwd(&mut px, 19, 2);
            rect(&mut px, 17, 16, 3, 3, color);
            rect(&mut px, 19, 17, 1, 1, c::INK);
        }
        Arms::Gun => {
            // a shotgun, levelled
            back(&mut px);
            fwd(&mut px, 19, 1);
            rect(&mut px, 13, 18, 3, 2, Color::rgb(0x7a4a2a));
            rect(&mut px, 15, 17, 6, 1, Color::rgb(0xa7aebb));
            rect(&mut px, 15, 18, 5, 1, Color::rgb(0x6c7380));
        }
        Arms::Paper => {
            back(&mut px);
            fwd(&mut px, 19, 2);
            rect(&mut px, 16, 15, 4, 5, c::STEEL4);
            rect(&mut px, 17, 16, 2, 1, c::STEEL2);
            rect(&mut px, 17, 18, 2, 1, c::STEEL2);
        }
        Arms::Book => {
            back(&mut px);
            fwd(&mut px, 19, 2);
            rect(&mut px, 16, 16, 3, 4, c::RED1);
            rect(&mut px, 16, 16, 1, 4, c::RED0);
        }
        Arms::Can => {
            back(&mut px);
            fwd(&mut px, 19, 2);
            rect(&mut px, 16, 17, 4, 3, c::GREEN0);
            rect(&mut px, 20, 17, 1, 1, c::GREEN0);
        }
        Arms::Roll => {
            // a rolled-up mat
            back(&mut px);
            fwd(&mut px, 19, 2);
            rect(&mut px, 16, 15, 4, 5, mat::TOP);
            rect(&mut px, 16, 15, 4, 1, mat::HI);
            rect(&mut px, 17, 16, 2, 3, mat::D);
        }
        Arms::Down => {
            back(&mut px);
            arm(&mut px, 14, 17, 20, 14, 21);
        }
    }
    match legs {
        Legs::Sit => {
            rect(&mut px, 7, 22, 9, 2, leg);
            rect(&mut px, 14, 24, 2, 3, leg);
            rect(&mut px, 14, 26, 3, 1, c::INK);
        }
        Legs::Walk1 => {
            rect(&mut px, 7, 22, 2, 5, leg);
            rect(&mut px, 11, 22, 2, 4, leg);
            rect(&mut px, 7, 26, 3, 1, c::INK);
            rect(&mut px, 11, 25, 3, 1, c::INK);
        }
        Legs::Walk2 => {
            rect(&mut px, 7, 22, 2, 4, leg);
            rect(&mut px, 11, 22, 2, 5, leg);
            rect(&mut px, 7, 25, 3, 1, c::INK);
            rect(&mut px, 11, 26, 3, 1, c::INK);
        }
        Legs::Stand => {
            rect(&mut px, 7, 22, 2, 5, leg);
            rect(&mut px, 11, 22, 2, 5, leg);
            rect(&mut px, 7, 26, 3, 1, c::INK);
            rect(&mut px, 11, 26, 3, 1, c::INK);
        }
    }
    let mut im = Image::new(SW as u32, SH as u32);
    let has = |k: i32| k >= 0 && (k as usize) < px.len() && px[k as usize].is_some() && !line[k as usize];
    for y in 0..SH {
        for i in 0..SW {
            let k = y * SW + i;
            if px[k as usize].is_some() {
                continue;
            }
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dy)| i + dx >= 0 && i + dx < SW && has((y + dy) * SW + i + dx));
            if near {
                im.set(i, y, c::INK.0);
            }
        }
    }
    for (k, col) in px.iter().enumerate() {
        if let Some(col) = col {
            im.set(k as i32 % SW, k as i32 / SW, col.0);
        }
    }
    let im = Rc::new(im);
    BODIES.with(|b| b.borrow_mut().insert(key, im.clone()));
    im
}

/// what a person's portrait wears, for a small figure of them elsewhere (the gunner outside the window):
/// the shoulders' colours as body() takes them, the skin, the hair (down the middle from the top)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wears {
    pub shirt: Color,
    pub shade: Color,
    pub skin: Color,
    pub hair: Option<Color>,
}

pub fn wears(seed: &str, you: Color) -> Wears {
    let f = avatar::avatar(seed, false, Some(you));
    let shirt = at(&f, 6, 19).unwrap_or(you);
    let skin = avatar::skin(seed);
    let mut hair = None;
    for y in 0..12 {
        if let Some(c) = at(&f, 10, y)
            && c != c::INK
            && c != c::INK2
            && c != skin
        {
            hair = Some(c);
            break;
        }
    }
    Wears { shirt, shade: at(&f, 15, 19).filter(|&x| x != shirt).unwrap_or(shirt.shade(0.62)), skin, hair }
}

/// one dot around an image's outline (not its shadow): a highlighted bot, chair, screen
pub fn halo(im: &Image, col: Color) -> Image {
    let (w, h) = (im.w as i32, im.h as i32);
    let on = |i: i32, j: i32| i >= 0 && j >= 0 && i < w && j < h && im.get(i, j)[3] > 128;
    let mut out = Image::new(im.w, im.h);
    for j in 0..h {
        for i in 0..w {
            if !on(i, j) && (on(i - 1, j) || on(i + 1, j) || on(i, j - 1) || on(i, j + 1)) {
                out.set(i, j, col.0);
            }
        }
    }
    out
}
