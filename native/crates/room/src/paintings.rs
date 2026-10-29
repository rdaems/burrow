//! What the robots paint on the easel (web/paintings.js): famous paintings, every person in them a robot. Each is a
//! list of dots in layers, in the order a painter would put them down (the sky and the ground first, then what stands
//! on them, the faces and the details last); the room paints them in that order, a few dots a second while a robot
//! stands at the easel. Portrait ones are 48 x 60, landscape ones 66 x 48: three times the easel's canvas in the room.
//!
//! `list()`: the paintings; `Painting::make()` its dots; `order(dots)`: the brush's order.

use crate::layout::jround;
use pix::{Color, css};
use std::collections::HashMap;
use std::f64::consts::PI;

/// the canvas before anything is painted on it
pub const PRIMER: Color = Color::rgb(0xf2ede2);

/// a dot of a painting: where, its colour, its layer
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    pub x: i32,
    pub y: i32,
    pub c: Color,
    pub l: i32,
}

pub struct Painting {
    pub title: &'static str,
    pub after: &'static str,
    pub w: i32,
    pub h: i32,
    make: fn() -> Vec<Dot>,
}

impl Painting {
    /// its dots, as they are laid out (not yet in the brush's order)
    pub fn make(&self) -> Vec<Dot> {
        (self.make)()
    }
}

static LIST: [Painting; 6] = [
    Painting { title: "Mona Lisa", after: "Leonardo", w: 48, h: 60, make: mona_lisa },
    Painting { title: "The Starry Night", after: "van Gogh", w: 66, h: 48, make: starry_night },
    Painting { title: "The Scream", after: "Munch", w: 48, h: 60, make: scream },
    Painting { title: "Girl with a Pearl Earring", after: "Vermeer", w: 48, h: 60, make: pearl },
    Painting { title: "The Great Wave", after: "Hokusai", w: 66, h: 48, make: great_wave },
    Painting { title: "American Gothic", after: "Grant Wood", w: 48, h: 60, make: gothic },
];

pub fn list() -> &'static [Painting] {
    &LIST
}

/// the brush's order: layer by layer; within one, in strokes three dots high, back and forth, a little uneven
pub fn order(dots: Vec<Dot>) -> Vec<Dot> {
    let mut keyed: Vec<(Dot, f64)> = dots.into_iter().map(|d| (d, d.x as f64 + hash(d.x, d.y, 99) * 6.)).collect();
    keyed.sort_by(|(a, ka), (b, kb)| {
        let (ra, rb) = (a.y.div_euclid(3), b.y.div_euclid(3));
        a.l.cmp(&b.l).then(ra.cmp(&rb)).then_with(|| if ra % 2 != 0 { kb.total_cmp(ka) } else { ka.total_cmp(kb) })
    });
    keyed.into_iter().map(|(d, _)| d).collect()
}

// ---- JS's arithmetic, as paintings.js gets it

/// (x, y, s) → 0..1, paintings.js's `hash` (Math.imul and >>> as JS has them)
fn hash(x: i32, y: i32, s: i32) -> f64 {
    let a = (x as i64 * 73856093) as i32 ^ (y as i64 * 19349663) as i32 ^ (s as i64 * 83492791) as i32;
    let mut h = a.wrapping_mul(2654435761u32 as i32);
    h ^= ((h as u32) >> 15) as i32;
    ((h as u32) % 10007) as f64 / 10007.
}

/// V8's Math.hypot (the largest first, a compensated sum): libm's hypot can differ in the last bit
fn hypot(a: f64, b: f64) -> f64 {
    let (a, b) = (a.abs(), b.abs());
    let max = a.max(b);
    if max == 0. {
        return 0.;
    }
    let (mut sum, mut comp) = (0f64, 0f64);
    for v in [a, b] {
        let n = v / max;
        let summand = n * n - comp;
        let pre = sum + summand;
        comp = (pre - sum) - summand;
        sum = pre;
    }
    sum.sqrt() * max
}

type S = &'static str;

const B4: [i32; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
/// from a to b by t, ordered
fn dith(a: S, b: S, t: f64, x: i32, y: i32) -> S {
    if t * 16. > B4[((y & 3) * 4 + (x & 3)) as usize] as f64 + 0.5 { b } else { a }
}
/// along a row of colours by t (0..1), dithered between neighbours
fn ramp(cols: &[S], t: f64, x: i32, y: i32) -> S {
    let u = t.clamp(0., 0.999) * (cols.len() - 1) as f64;
    let i = u.floor() as usize;
    dith(cols[i], cols[(i + 1).min(cols.len() - 1)], u - i as f64, x, y)
}
fn pick(cols: &[S], t: f64) -> S {
    cols[((t * cols.len() as f64).floor()).clamp(0., (cols.len() - 1) as f64) as usize]
}

fn f(v: i32) -> f64 {
    v as f64
}

/// a painting being laid out: its dots by layer and place, a dot put again keeping its first place in the order
struct E {
    w: i32,
    h: i32,
    px: Vec<(i32, i32, S, i32)>,
    at: HashMap<i64, usize>,
}

impl E {
    fn new(w: i32, h: i32) -> E {
        E { w, h, px: Vec::new(), at: HashMap::new() }
    }
    fn put(&mut self, x: f64, y: f64, c: Option<S>, l: i32) {
        let (x, y) = (jround(x), jround(y));
        let Some(c) = c else { return };
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let k = l as i64 * 1_000_000 + y as i64 * 1000 + x as i64;
        match self.at.get(&k) {
            Some(&i) => self.px[i] = (x, y, c, l),
            None => {
                self.at.insert(k, self.px.len());
                self.px.push((x, y, c, l));
            }
        }
    }
    fn p(&mut self, x: f64, y: f64, c: S, l: i32) {
        self.put(x, y, Some(c), l)
    }
    #[allow(clippy::too_many_arguments)]
    fn area(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, l: i32, mut fill: impl FnMut(i32, i32) -> Option<S>) {
        let mut y = y0.floor();
        while y < y1 {
            let mut x = x0.floor();
            while x < x1 {
                let c = fill(x as i32, y as i32);
                self.put(x, y, c, l);
                x += 1.;
            }
            y += 1.;
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn oval(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, l: i32, mut fill: impl FnMut(i32, i32, f64) -> Option<S>) {
        let mut y = (cy - ry).floor();
        while y <= cy + ry {
            let mut x = (cx - rx).floor();
            while x <= cx + rx {
                let d = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
                if d <= 1. {
                    let c = fill(x as i32, y as i32, d.sqrt());
                    self.put(x, y, c, l);
                }
                x += 1.;
            }
            y += 1.;
        }
    }
    fn poly(&mut self, pts: &[(f64, f64)], l: i32, mut fill: impl FnMut(i32, i32) -> Option<S>) {
        let inside = |x: f64, y: f64| {
            let mut c = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
                if (yi > y) != (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi {
                    c = !c;
                }
                j = i;
            }
            c
        };
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for &(x, y) in pts {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        let mut y = y0.floor();
        while y <= y1 {
            let mut x = x0.floor();
            while x <= x1 {
                if inside(x + 0.5, y + 0.5) {
                    let c = fill(x as i32, y as i32);
                    self.put(x, y, c, l);
                }
                x += 1.;
            }
            y += 1.;
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, l: i32, c: S, th: i32) {
        let n = (hypot(x1 - x0, y1 - y0) * 2.).ceil();
        let n = if n == 0. { 1. } else { n };
        let half = (th - 1) as f64 / 2.;
        let mut k = 0.;
        while k <= n {
            let (x, y) = (x0 + ((x1 - x0) * k) / n, y0 + ((y1 - y0) * k) / n);
            for j in 0..th {
                for i in 0..th {
                    self.p(x + i as f64 - half, y + j as f64 - half, c, l);
                }
            }
            k += 1.;
        }
    }
    fn dots(self) -> Vec<Dot> {
        self.px.into_iter().map(|(x, y, c, l)| Dot { x, y, c: css(c), l }).collect()
    }
}

// ---- a robot's head as the room draws its robots (pixel.js robot()): a steel box lit along its top and left edge,
// shaded right and below, its corners cut; across it the dark screen with the eyes glowing in it; a bolt each side
// where ears would be; the antenna with its light. Whoever is in a painting has one

mod st {
    pub const HI: &str = "#f4f6fb";
    pub const LIT: &str = "#c2c9d9";
    pub const MID: &str = "#8b93ad";
    pub const DARK: &str = "#5d6280";
    pub const DEEP: &str = "#3b3a55";
    pub const SCR: &str = "#1a2448";
}

/// eye, dim (the eyes' colours), mouth ("grille", "smile", "o", "lips"), turn (-1..1: three quarters, the screen slid
/// that way), top (the screen's first row, from the head's), ant (the antenna's height), light (its colour)
#[derive(Default)]
struct Head {
    eye: Option<S>,
    dim: Option<S>,
    mouth: S,
    turn: f64,
    top: Option<i32>,
    ant: i32,
    light: Option<S>,
    glasses: bool,
    no_ears: bool,
}

#[allow(clippy::too_many_arguments)]
fn head(e: &mut E, x0: i32, y0: i32, w: i32, h: i32, l: i32, o: Head) {
    let (eye, dim, turn, x1, y1) = (o.eye.unwrap_or("#7fdcb0"), o.dim.unwrap_or("#2fa087"), o.turn, x0 + w - 1, y0 + h - 1);
    e.area(f(x0), f(y0), f(x1 + 1), f(y1 + 1), l, |x, y| {
        if (x == x0 || x == x1) && (y == y0 || y == y1) {
            return None;
        }
        Some(if y == y0 || x == x0 {
            st::HI
        } else if y == y1 || x == x1 {
            st::DARK
        } else if f(x - x0) / f(w) > 0.74 + turn * 0.12 {
            st::MID
        } else {
            st::LIT
        })
    });
    let m = if o.glasses { 1 } else { 1.max(jround(f(w) * 0.14)) };
    let sy0 = y0 + o.top.unwrap_or_else(|| 2.max(jround(f(h) * 0.22)));
    let sh = 3.max(jround(f(h) * 0.42));
    let sx0 = x0 + 1 + m + jround(turn.min(0.) * f(m));
    let sx1 = x1 - 1 - m + jround(turn.max(0.) * f(m));
    e.area(f(sx0), f(sy0), f(sx1 + 1), f(sy0 + sh), l + 1, |_, y| Some(if y == sy0 { st::DEEP } else { st::SCR }));
    let ey = sy0 + sh / 2 - if sh > 4 { 1 } else { 0 };
    let span = sx1 - sx0;
    let inset = 1.max(jround(f(span) * 0.22));
    let two = w > 11 && !o.glasses && span - 2 * inset >= 5; // eyes two dots wide when they still stand apart
    let eyes = if o.glasses { [sx0 + 1, sx1 - 1] } else { [sx0 + inset, sx1 - inset - if two { 1 } else { 0 }] };
    for x in eyes {
        if o.glasses {
            // round glasses: a ring of light round each eye, a bridge between
            for (i, j) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                e.p(f(x + i), f(ey + j), dim, l + 1);
            }
            e.p(f(x), f(ey), eye, l + 1);
            continue;
        }
        for i in 0..if two { 2 } else { 1 } {
            e.p(f(x + i), f(ey), eye, l + 1);
            if sh > 3 {
                e.p(f(x + i), f(ey + 1), dim, l + 1);
            }
        }
    }
    if o.glasses {
        e.line(f(eyes[0] + 2), f(ey), f(eyes[1] - 2), f(ey), l + 1, st::MID, 1);
    }
    let cx = jround(f(sx0 + sx1) / 2.);
    match o.mouth {
        "smile" => {
            // a slight smile, lit
            for x in cx - 2..=cx + 2 {
                e.p(f(x), f(sy0 + sh - 1 - if (x - cx).abs() == 2 { 1 } else { 0 }), dim, l + 1);
            }
        }
        "grille" => {
            for x in x0 + 3..=x1 - 3 {
                e.p(f(x), f(y1 - 2), if x % 2 != 0 { st::DARK } else { st::DEEP }, l + 1);
            }
        }
        "lips" => {
            for x in cx - 1..=cx + 1 {
                e.p(f(x), f(sy0 + sh + 1), if x == cx { "#e86048" } else { "#b0503a" }, l + 1);
            }
        }
        "o" => {
            // wide open
            e.oval(f(cx), f((y1 - 2).min(sy0 + sh + 2)), 1.4, 2.2, l + 1, |_, _, d| Some(if d > 0.7 { st::DEEP } else { "#0e1226" }));
        }
        _ => {}
    }
    if !o.no_ears {
        let ym = jround(f(sy0 + sy0 + sh) / 2.);
        e.p(f(x0 - 1), f(ym), st::DARK, l);
        e.p(f(x1 + 1), f(ym), st::DARK, l);
        e.p(f(x0 - 1), f(ym + 1), st::DEEP, l);
        e.p(f(x1 + 1), f(ym + 1), st::DEEP, l);
    }
    if o.ant != 0 {
        let ax = f(x0 + w / 2);
        e.line(ax, f(y0 - 1), ax, f(y0 - o.ant), l, st::MID, 1);
        e.p(ax, f(y0 - o.ant - 1), o.light.unwrap_or(eye), l + 1);
    }
}

/// Mona Lisa, after Leonardo: a robot in a dark dress, hands folded, the smile lit on its screen, the veil over its
/// head; the hazy land behind, its road winding on the left, a lake and a bridge on the right
fn mona_lisa() -> Vec<Dot> {
    let mut e = E::new(48, 60);
    e.area(0., 0., 48., 26., 0, |x, y| Some(ramp(&["#8e9c6c", "#a9b27e", "#c6c48e", "#d9d09a"], f(y) / 24., x, y)));
    let ridge = |x: i32| 15. + 3. * (f(x) / 3.1).sin() + 2. * (f(x) / 1.7 + 1.).sin();
    e.area(0., 8., 48., 26., 1, |x, y| (f(y) >= ridge(x)).then(|| ramp(&["#9fb39a", "#7c937c", "#66795f"], (f(y) - ridge(x)) / 9., x, y)));
    // down to the parapet
    e.area(0., 22., 18., 60., 1, |x, y| {
        Some(if y > 44 {
            if y == 45 { "#6a5a34" } else { "#4a3f22" }
        } else {
            ramp(&["#8a8450", "#6e6a3c", "#55522e"], f(y - 22) / 18., x, y)
        })
    });
    for y in 22..40 {
        // the road
        let rx = 7. + 4. * (f(y) / 3.).sin();
        e.p(rx, f(y), "#c2a866", 1);
        e.p(rx + 1., f(y), "#b0944e", 1);
    }
    e.area(31., 17., 48., 22., 1, |x, y| Some(if hash(x, y, 3) < 0.15 { "#c8d4b8" } else { "#a9bba4" })); // the lake
    e.area(30., 22., 48., 60., 1, |x, y| {
        Some(if y > 44 {
            if y == 45 { "#6a5a34" } else { "#4a3f22" }
        } else {
            ramp(&["#7e8a58", "#66703f", "#4f552e"], f(y - 22) / 18., x, y)
        })
    });
    for x in 38..47 {
        // the bridge
        e.p(f(x), f(30 - jround(1.6 * ((f(x - 38) / 8.) * PI).sin())), "#a89458", 1);
    }
    // the dress, the hair and veil falling to the shoulders
    e.poly(&[(3., 60.), (6., 45.), (11., 39.), (18., 35.), (30., 35.), (37., 39.), (42., 45.), (45., 60.)], 2, |x, y| {
        Some(if hash(x >> 1, y, 5) < 0.16 {
            "#3d3523"
        } else if (x + y) % 9 == 0 {
            "#332c1c"
        } else {
            "#29241a"
        })
    });
    let hair = |x: i32, y: i32| Some(if (x * 3 + y) % 7 == 0 { "#3a2a1a" } else { "#22180f" });
    e.oval(24., 18., 10., 13., 2, |x, y, _| hair(x, y));
    e.area(14., 20., 35., 38., 2, hair); // the hair behind the neck
    let veil = |x: i32, y: i32| Some(if (x + y * 2) % 5 == 0 { "#3a2a1a" } else { "#22180f" });
    e.poly(&[(14., 18.), (19., 22.), (19., 38.), (13., 38.)], 2, veil);
    e.poly(&[(34., 18.), (29., 22.), (29., 38.), (35., 38.)], 2, veil);
    // the neck in rings, the chest plate in the neckline, the head: a robot's, the smile lit, the antenna through the veil
    e.area(21., 26., 28., 36., 3, |x, y| {
        Some(if x == 21 {
            st::HI
        } else if x == 27 {
            st::DARK
        } else if (y - 26) % 3 == 2 {
            st::MID
        } else {
            st::LIT
        })
    });
    e.oval(24., 35., 8., 4., 3, |x, y, d| {
        (y >= 35).then_some(if d > 0.85 {
            st::DARK
        } else if x < 21 {
            st::HI
        } else {
            st::LIT
        })
    });
    head(&mut e, 18, 11, 13, 16, 3, Head { eye: Some("#f3c94a"), dim: Some("#a8802a"), mouth: "smile", turn: 0.25, ant: 5, ..Head::default() });
    // the sleeves, the hands folded, the right over the left
    e.poly(&[(8., 54.), (11., 49.), (19., 52.), (19., 56.), (10., 58.)], 3, |x, y| Some(if (x + y) % 5 == 0 { "#7a6030" } else { "#5a4622" }));
    e.poly(&[(40., 54.), (37., 48.), (29., 51.), (28., 55.), (38., 58.)], 3, |x, y| Some(if (x - y) % 5 == 0 { "#7a6030" } else { "#5a4622" }));
    // the hands, steel
    e.oval(20.5, 54.5, 3., 1.8, 4, |x, _, d| {
        Some(if d > 0.8 {
            st::DARK
        } else if x < 20 {
            st::HI
        } else {
            st::LIT
        })
    });
    e.oval(26., 53.2, 3.6, 2., 4, |_, _, d| Some(if d > 0.8 { st::DARK } else { st::LIT }));
    for x in [24, 26, 28] {
        e.p(f(x), 54., st::DARK, 4);
    }
    e.dots()
}

/// The Starry Night, after van Gogh: the sky in flowing strokes, its great swirl, stars with halos, the moon; the
/// hills, the village and its church; the cypress flaming up; and a robot sitting on the hill, looking up
fn starry_night() -> Vec<Dot> {
    let mut e = E::new(66, 48);
    let blues = ["#16295e", "#1d3576", "#26448e", "#3459a6", "#4a72bc"];
    e.area(0., 0., 66., 34., 0, |x, y| {
        let (fx, fy) = (f(x), f(y));
        Some(pick(&blues, (0.5 + 0.5 * (fx * 0.21 + 2.2 * (fy * 0.19 + fx * 0.05).sin() + fy * 0.13).sin()) * 0.9 + hash(x, y, 1) * 0.15))
    });
    let swirl = |e: &mut E, cx: f64, cy: f64, r0: f64| {
        e.area(cx - r0, cy - r0 * 0.7, cx + r0 + 1., cy + r0 * 0.7 + 1., 1, |x, y| {
            let (dx, dy) = (f(x) - cx, (f(y) - cy) / 0.7);
            let r = hypot(dx, dy);
            if r > r0 {
                return None;
            }
            let s = ((dy.atan2(dx) / (2. * PI)) * 2. + r / 3.2 + 10.) % 1.;
            if s < 0.22 {
                Some("#9ab8e2")
            } else if s < 0.34 {
                Some("#dfe6cc")
            } else if s < 0.45 {
                Some("#6f93cf")
            } else {
                None
            }
        })
    };
    swirl(&mut e, 33., 13., 10.);
    swirl(&mut e, 49., 16., 5.);
    swirl(&mut e, 17., 8., 4.);
    for (cx, cy, r) in [(5., 5., 2.), (25., 3., 2.), (42., 3., 2.4), (53., 9., 2.), (60., 21., 2.), (38., 25., 1.5), (21., 21., 1.6), (8., 16., 1.6)] {
        e.oval(cx, cy, r + 1.6, r + 1.6, 2, |x, y, d| {
            if d > 0.72 {
                (hash(x, y, 2) < 0.6).then_some("#9cbc6e")
            } else if d > 0.4 {
                Some("#f2eba0")
            } else {
                None
            }
        });
        e.oval(cx, cy, r * 0.55 + 0.4, r * 0.55 + 0.4, 2, |_, _, _| Some("#f3d54a"));
        e.p(cx, cy, "#fff8d0", 2);
    }
    e.oval(59., 6., 5.6, 5.6, 2, |_, _, d| (d > 0.72).then_some("#e8d870"));
    e.oval(59., 6., 3.6, 3.6, 2, |x, y, _| Some(if (f(x) - 60.6).powi(2) + (f(y) - 4.6).powi(2) < 7.5 { "#f2eba0" } else { "#f0b83a" }));
    // the hills, the village with its lights, the church's spire
    let hill = |x: f64| 30. + 2.2 * (x / 6.5 + 1.).sin() + 1.5 * (x / 2.9).sin();
    e.area(0., 24., 66., 48., 3, |x, y| {
        let hl = hill(f(x));
        if f(y) < hl {
            None
        } else if f(y) - hl < 1.5 {
            Some("#3d5f9a")
        } else if hash(x >> 1, y, 3) < 0.2 {
            Some("#2f5a7a")
        } else if y > 43 {
            Some("#1a2c56")
        } else {
            Some("#22386a")
        }
    });
    for (hx, hy, hw, hh) in [(27, 38, 4, 3), (32, 37, 3, 3), (45, 38, 4, 3), (50, 39, 4, 3), (55, 37, 5, 4), (61, 39, 4, 3), (31, 42, 4, 3), (48, 43, 5, 3), (57, 43, 4, 3), (37, 41, 3, 3)] {
        e.area(f(hx), f(hy), f(hx + hw), f(hy + hh), 4, |_, _| Some("#2c3c5e"));
        e.line(f(hx), f(hy - 1), f(hx + hw - 1), f(hy - 1), 4, "#5276b0", 1);
        if hash(hx, hy, 4) < 0.75 {
            e.p(f(hx + 1), f(hy + 1), "#f0c43a", 4);
        }
    }
    e.area(41., 35., 44., 41., 4, |_, _| Some("#24304e"));
    e.line(42.5, 35., 42.5, 26., 4, "#16203a", 1);
    // the cypress, a dark flame from the ground into the sky
    for y in 2..48 {
        let cx = 11. + 1.6 * (f(y) * 0.33).sin();
        let half = 0.6f64.max(f(y - 2) * 0.14 + 0.9 * (f(y) * 0.8).sin());
        let mut x = (cx - half).floor();
        while x <= cx + half {
            let ed = (x - cx).abs() / half;
            let h = hash(x as i32, y >> 1, 6);
            let c = if ed > 0.72 {
                if h < 0.5 { "#5a3e1e" } else { "#3e4a24" }
            } else if h < 0.25 {
                "#3e4a24"
            } else if h < 0.4 {
                "#2a3a1e"
            } else {
                "#16241a"
            };
            e.p(x, f(y), c, 5);
            x += 1.;
        }
    }
    // a robot sitting on the hill with its back to us, looking up at it all; its antenna's light on
    let (bx, by) = (23, jround(hill(25.)) - 7);
    for (j, row) in ["...y.", "...n.", ".sss.", ".ssd.", "ssssd", "ss.sd"].iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let c = match ch {
                'y' => "#f3d54a",
                'n' => "#5d6280",
                's' => "#9aa4bc",
                'd' => "#6a7290",
                _ => continue,
            };
            e.p(f(bx + i as i32), f(by + j as i32), c, 6);
        }
    }
    e.dots()
}

/// The Scream, after Munch: the sky in bands of fire, the dark fjord, the bridge going off to the left, two robots
/// far along it, and in front the one who screams, its clamps to its head
fn scream() -> Vec<Dot> {
    let mut e = E::new(48, 60);
    let sky = ["#e88a32", "#d4502c", "#eab24a", "#c8402a", "#e06a2a"];
    e.area(0., 0., 48., 26., 0, |x, y| {
        let (fx, fy) = (f(x), f(y));
        let b = ((fy + 3.2 * (fx * 0.16 + fy * 0.12).sin() + 1.5 * (fx * 0.4).sin()) / 3.6).floor() as i64;
        Some(sky[(((b % 5) + 5) % 5) as usize])
    });
    e.area(0., 18., 48., 48., 1, |x, y| {
        let (fx, fy) = (f(x), f(y));
        if fx > 30. - (fy - 18.) * 0.6 + 3. * (fy / 4.).sin() {
            // the land
            return Some(if hash(x, y >> 1, 7) < 0.25 {
                "#5a7a3a"
            } else if (x + y) % 6 == 0 {
                "#2f4a2c"
            } else {
                "#3d5e36"
            });
        }
        let s = (fx * 0.3 + fy * 0.5 + (fy * 0.3).sin() * 2.).sin();
        Some(if s > 0.6 {
            "#3a5a94"
        } else if s > -0.2 {
            "#253c72"
        } else {
            "#1b2c5a"
        })
    });
    let rail = |x: f64| 30. + (x * 16.) / 48.;
    e.area(0., 28., 48., 60., 2, |x, y| {
        let (fx, fy) = (f(x), f(y));
        if fy < rail(fx) + 2. {
            None
        } else if (fy - fx * 0.33).floor() as i64 % 3 == 0 {
            Some("#7a4a2e")
        } else if y > 54 {
            Some("#8a5634")
        } else {
            Some("#9a6440")
        }
    });
    e.line(0., 29., 48., 45., 2, "#5e3424", 2);
    for x in (2..48).step_by(7) {
        e.line(f(x), rail(f(x)), f(x), rail(f(x)) + 3., 2, "#5e3424", 1);
    }
    for fx in [3, 7] {
        // two far along the bridge
        for (j, row) in ["y.", "hs", "kk", "kk", "k."].iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                let c = match ch {
                    'y' => "#f3d54a",
                    'h' => st::LIT,
                    's' => st::SCR,
                    'k' => "#2a2430",
                    _ => continue,
                };
                e.p(f(fx + i as i32), f(23 + if fx > 4 { 1 } else { 0 } + j as i32), c, 3);
            }
        }
    }
    // the one who screams: the dark body, a long robot's head, the eyes wide, the mouth open; its clamps to its head
    e.poly(&[(13., 60.), (16., 52.), (19., 46.), (21., 42.), (28., 42.), (30., 46.), (33., 52.), (37., 60.)], 3, |x, y| {
        Some(if (f(x) * 0.7 + f(y) * 0.35).sin() > 0.55 { "#3e3a56" } else { "#2c2a3e" })
    });
    e.line(19., 38., 21., 44., 3, "#2c2a3e", 2);
    e.line(31., 38., 28., 44., 3, "#2c2a3e", 2);
    let o = Head { eye: Some("#fff6c8"), dim: Some("#e8c040"), mouth: "o", ant: 3, light: Some("#e8402a"), no_ears: true, top: Some(2), ..Head::default() };
    head(&mut e, 20, 28, 10, 14, 4, o);
    e.area(18., 31., 20., 39., 6, |x, _| Some(if x == 18 { st::DARK } else { st::MID }));
    e.area(30., 31., 32., 39., 6, |x, _| Some(if x == 31 { st::DARK } else { st::MID }));
    e.dots()
}

/// Girl with a Pearl Earring, after Vermeer: a robot turning to look at you over its shoulder, its eyes on its screen,
/// a red light for the parted lips, the blue and yellow turban (its antenna through it), the pearl from the bolt
/// where an ear would be, the white collar
fn pearl() -> Vec<Dot> {
    let mut e = E::new(48, 60);
    e.area(0., 0., 48., 60., 0, |x, y| Some(if hash(x >> 1, y >> 1, 8) < 0.2 { "#1c1914" } else { "#12100c" }));
    e.poly(&[(2., 60.), (6., 50.), (14., 44.), (30., 45.), (38., 52.), (42., 60.)], 1, |x, y| {
        Some(ramp(&["#a8884a", "#8a6e3a", "#6a5428", "#4a3a1c"], f(x - 2) / 40. + f(y - 44) / 60., x, y))
    });
    // the neck, in rings
    e.poly(&[(21., 36.), (31., 36.), (31., 45.), (22., 45.)], 2, |x, y| {
        Some(if (y - 36) % 3 == 2 { st::DARK } else { ramp(&[st::LIT, st::MID, st::DARK], f(x - 21) / 10., x, y) })
    });
    e.poly(&[(12., 46.), (19., 43.), (32., 44.), (34., 46.), (26., 48.), (16., 49.)], 2, |x, y| {
        Some(ramp(&["#f2ede2", "#d8d0c0", "#a8a090"], f(x - 12) / 22., x, y))
    });
    let o = Head { eye: Some("#bfe3ff"), dim: Some("#4a76b8"), mouth: "lips", turn: -0.35, top: Some(6), no_ears: true, ..Head::default() };
    head(&mut e, 19, 18, 17, 19, 3, o);
    e.poly(&[(19., 14.), (27., 11.), (24., 19.), (18., 35.), (12., 35.), (14., 22.)], 4, |x, y| {
        Some(ramp(&["#f2d27a", "#d8b04a", "#b08a30", "#7a5e20"], f(x - 12) / 16. + (hash(x, y >> 2, 9) - 0.5) * 0.15, x, y))
    });
    e.oval(25., 17., 9.5, 6.5, 4, |x, y, _| {
        let (fx, fy) = (f(x), f(y));
        if fy > 21. + (fx - 25.) * 0.25 {
            return None;
        }
        let k = if (fx * 0.9 + fy * 0.6).sin() > 0.7 { -0.2 } else { 0. };
        Some(ramp(&["#5a86cc", "#3d6ab0", "#2a4f8f", "#1c3666"], (fx - 16.) / 19. + k, x, y))
    });
    // the bolt and its pearl, the antenna through the turban
    e.oval(19.5, 30.5, 1.1, 1.1, 5, |_, _, _| Some(st::DARK));
    e.p(19.5, 32., st::MID, 5);
    e.oval(19.5, 34.3, 1.5, 2., 6, |x, y, _| {
        Some(if y < 34 && x < 20 {
            "#f8f8f4"
        } else if y > 35 {
            "#7a7a86"
        } else {
            "#c8c8d0"
        })
    });
    e.line(26., 11., 25., 6., 6, st::MID, 1);
    e.oval(25., 5., 1.1, 1.1, 6, |_, _, _| Some("#bfe3ff"));
    e.dots()
}

/// The Great Wave, after Hokusai: the wave rearing up over the boats, its foam reaching out like claws, spray in the
/// sky, Fuji small in the trough; rowing in the boats, robots
fn great_wave() -> Vec<Dot> {
    let mut e = E::new(66, 48);
    e.area(0., 0., 66., 48., 0, |x, y| Some(ramp(&["#d6c8a4", "#e2d6b8", "#ebe1c8"], f(y) / 30., x, y)));
    // the cartouche
    e.area(2., 2., 11., 10., 1, |x, y| {
        Some(if x == 2 || y == 2 || x == 10 || y == 9 {
            "#8a7a5a"
        } else if x % 2 == 0 && y > 3 && y < 9 && hash(x, y, 10) < 0.6 {
            "#4a4030"
        } else {
            "#e8dcc0"
        })
    });
    e.poly(&[(36., 37.), (46., 26.), (56., 37.)], 1, |x, y| Some(if f(y) < 29. + hash(x, 0, 11) * 2. { "#f4efe2" } else { "#3e4c6e" }));
    e.area(0., 35., 66., 48., 2, |x, y| {
        Some(if (y + jround(2. * (f(x) / 3.).sin())) % 3 == 0 {
            "#8fb4d8"
        } else if y > 42 {
            "#2a4f88"
        } else {
            "#4a78b0"
        })
    });
    let in_a = |x: f64, y: f64| ((x - 12.) / 26.).powi(2) + ((y - 34.) / 30.).powi(2) <= 1.;
    let in_lip = |x: f64, y: f64| ((x - 25.) / 11.).powi(2) + ((y - 9.) / 5.5).powi(2) <= 1.;
    let r_h = |x: f64, y: f64| hypot((x - 33.) / 9.5, (y - 23.) / 11.);
    let mass = |x: i32, y: i32| (in_a(f(x), f(y)) || in_lip(f(x), f(y))) && r_h(f(x), f(y)) >= 1.;
    // its body in bands curving round the hollow, white and pale near the edge
    let body = |cx: f64, cy: f64, sx: f64, sy: f64| {
        move |x: i32, y: i32| {
            let (fx, fy) = (f(x), f(y));
            let r = hypot((fx - cx) / sx, (fy - cy) / sy);
            let s = (r * 4.5 + ((fy - cy).atan2(fx - cx) * 3.).sin() * 0.3) % 1.;
            if r < 1.25 {
                return if s < 0.3 {
                    "#f4efe2"
                } else if s < 0.6 {
                    "#8fb4d8"
                } else {
                    "#4a78b0"
                };
            }
            if s < 0.18 {
                "#8fb4d8"
            } else if r > 2.2 {
                "#1a3260"
            } else if s < 0.5 {
                "#2a4f88"
            } else {
                "#1f3a6a"
            }
        }
    };
    let b1 = body(33., 23., 9.5, 11.);
    e.area(0., 0., 44., 48., 3, |x, y| mass(x, y).then(|| b1(x, y)));
    let b2 = body(58., 52., 7., 6.);
    e.oval(58., 48., 13., 10., 3, |x, y, _| Some(b2(x, y)));
    for x in 6..40 {
        // the foam along its top, fingers of it
        let Some(ye) = (0..48).find(|&y| mass(x, y)) else { continue };
        if ye > 22 {
            continue;
        }
        e.p(f(x), f(ye), "#f4efe2", 4);
        if hash(x, 1, 12) < 0.6 {
            e.p(f(x), f(ye - 1), "#f4efe2", 4);
        }
        if hash(x, 2, 12) < 0.3 {
            e.p(f(x + if hash(x, 3, 12) < 0.5 { -1 } else { 1 }), f(ye - 2), "#f4efe2", 4);
        }
    }
    for x in 27..38 {
        // the claws hanging from the lip into the hollow
        let Some(yb) = (4..=14).rev().find(|&y| in_lip(f(x), f(y)) && r_h(f(x), f(y)) >= 1.) else { continue };
        let n = 1 + (hash(x, 4, 12) * 3.).floor() as i32;
        for k in 1..=n {
            e.p(f(x), f(yb + k), if k == n { "#d8d4c4" } else { "#f4efe2" }, 4);
        }
    }
    for x in 46..66 {
        if hash(x, 5, 12) < 0.7 {
            e.p(f(x), f(jround(48. - 10. * (0f64.max(1. - ((f(x) - 58.) / 13.).powi(2))).sqrt())), "#f4efe2", 4);
        }
    }
    e.area(14., 0., 44., 11., 4, |x, y| (hash(x, y, 13) < 0.07 && !mass(x, y)).then_some("#f4efe2")); // spray
    // the boats, robots rowing in them
    let boat = |e: &mut E, x0: i32, y0: i32, x1: i32, y1: i32| {
        e.line(f(x0), f(y0 + 1), f(x1), f(y1 + 1), 5, "#9a8a6a", 1);
        e.line(f(x0), f(y0), f(x1), f(y1), 5, "#e2d4b0", 1);
        let mut k = 2;
        while k < (x1 - x0).abs() - 1 {
            let x = x0 + k;
            let y = jround(f(y0) + (f(y1 - y0) * f(k)) / f(x1 - x0));
            e.p(f(x), f(y - 1), "#7fdcb0", 5);
            e.p(f(x + 1), f(y - 1), st::SCR, 5);
            e.p(f(x), f(y - 2), st::LIT, 5);
            e.p(f(x + 1), f(y - 2), st::MID, 5);
            e.p(f(x), f(y - 3), "#f3d54a", 5);
            k += 3;
        }
    };
    boat(&mut e, 15, 41, 34, 36);
    boat(&mut e, 46, 45, 62, 43);
    e.dots()
}

/// American Gothic, after Grant Wood: two robots before the white farmhouse with its pointed window, she with a bow
/// on her antenna, the cameo and the apron, he with round glasses, the overalls and the pitchfork
fn gothic() -> Vec<Dot> {
    let mut e = E::new(48, 60);
    e.area(0., 0., 48., 34., 0, |x, y| Some(ramp(&["#b8c8d4", "#cad6de", "#d8e0e4"], f(y) / 24., x, y)));
    e.oval(4., 13., 7., 8., 1, |x, y, _| Some(if hash(x, y, 14) < 0.3 { "#5a7a44" } else { "#3e5a30" }));
    e.oval(10., 8., 5., 6., 1, |x, y, _| Some(if hash(x, y, 15) < 0.3 { "#5a7a44" } else { "#3e5a30" }));
    e.area(40., 10., 48., 30., 1, |x, y| Some(if y == 10 || x == 40 { "#ece6da" } else { "#a0402a" }));
    e.poly(&[(12., 32.), (12., 10.), (25., 1.), (38., 10.), (38., 32.)], 2, |_, y| Some(if y % 2 == 0 { "#d6d2c8" } else { "#ece8e0" }));
    e.line(11., 10.5, 25., 0.5, 2, "#5a5a62", 1);
    e.line(25., 0.5, 39., 10.5, 2, "#5a5a62", 1);
    e.poly(&[(20.5, 21.), (20.5, 12.), (25., 6.5), (29.5, 12.), (29.5, 21.)], 3, |_, _| Some("#3a3a42"));
    e.poly(&[(21.5, 20.), (21.5, 12.5), (25., 8.), (28.5, 12.5), (28.5, 20.)], 3, |x, y| {
        Some(if x == 25 || y == 14 {
            "#3a3a42"
        } else if x < 24 {
            "#8a5a5a"
        } else {
            "#2a2a32"
        })
    });
    let steel = |x0: f64, span: f64| move |x: i32, y: i32| Some(ramp(&["#dde2ea", "#bcc4d0", "#9aa2b0", "#747c8a"], (f(x) - x0) / span, x, y));
    // her: the dark dress, the print apron, the white collar and the cameo, her head turned a little, her bow
    e.poly(&[(0., 60.), (2., 42.), (7., 37.), (20., 37.), (24., 42.), (24., 60.)], 4, |x, y| Some(if (x + y) % 7 == 0 { "#4a362c" } else { "#3a2a22" }));
    e.poly(&[(3., 60.), (5., 49.), (21., 49.), (23., 60.)], 4, |x, y| Some(if (x + y * 2) % 4 == 0 { "#c8a070" } else { "#6a4a32" }));
    e.area(12., 32., 17., 37., 4, steel(11., 7.));
    e.oval(14.5, 37., 6., 2.2, 4, |_, y, _| (y >= 36).then_some("#ece6da"));
    e.oval(14.5, 39.2, 1.3, 1.3, 5, |_, _, d| Some(if d < 0.5 { "#8a5a3a" } else { "#c8a070" }));
    head(&mut e, 10, 21, 10, 12, 5, Head { eye: Some("#7fdcb0"), mouth: "grille", turn: 0.4, ant: 4, ..Head::default() });
    for (x, y) in [(13, 15), (14, 16), (13, 17), (16, 16), (17, 15), (17, 17)] {
        e.p(f(x), f(y), "#d6457a", 7); // her bow, on the antenna
    }
    e.p(15., 16., "#a03060", 7);
    // him: the black jacket, the striped shirt, the overalls, the long head, round glasses, the pitchfork in his hand
    e.poly(&[(20., 60.), (22., 40.), (27., 34.), (40., 34.), (45., 40.), (48., 60.)], 4, |x, _| {
        Some(if !(26..=41).contains(&x) { "#2e2e3a" } else { "#1c1c24" })
    });
    e.poly(&[(30., 34.), (37.5, 34.), (34., 45.)], 4, |_, y| Some(if y % 2 != 0 { "#e8e4dc" } else { "#c8c4bc" }));
    e.poly(&[(29., 43.), (38.5, 43.), (39.5, 60.), (28., 60.)], 4, |x, y| Some(if (x * 3 + y) % 8 == 0 { "#6a7a9a" } else { "#4a5a7a" }));
    e.line(30., 36., 30., 43., 4, "#4a5a7a", 1);
    e.line(37.5, 36., 37.5, 43., 4, "#4a5a7a", 1);
    e.area(32., 29., 36., 34., 4, steel(31., 6.));
    head(&mut e, 29, 13, 11, 17, 5, Head { eye: Some("#7fdcb0"), mouth: "grille", glasses: true, ant: 3, ..Head::default() });
    e.line(27.5, 36., 27.5, 60., 7, "#8a6a4a", 1);
    for tx in [24.5, 27.5, 30.5] {
        e.line(tx, 30., tx, 36., 7, "#9aa2a8", 1);
    }
    e.line(24.5, 36., 30.5, 36., 7, "#9aa2a8", 1);
    let s = steel(25., 5.);
    e.oval(27.5, 48., 2., 1.8, 8, |x, y, _| s(x, y));
    e.dots()
}
