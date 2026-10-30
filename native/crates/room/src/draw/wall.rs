//! What hangs on the wall: the closet, the switchboard, the doors, the clock, the windows.

use super::*;
use std::f64::consts::PI;

/// the closet, against the wall (it holds the furniture that is not in the room): a wardrobe seen
/// from the front like the doors, its right door open while its page is
pub fn draw_closet(ctx: &mut Ctx, x0: i32, y0: i32, open: bool) {
    rect(ctx, x0, y0, 22, 24, WOOD.k);
    rect(ctx, x0 + 1, y0 + 1, 20, 22, WOOD.d);
    rect(ctx, x0 + 1, y0 + 1, 20, 1, WOOD.hi);
    rect(ctx, x0 + 1, y0 + 2, 20, 1, WOOD.top);
    for dx in [2, 12] {
        rect(ctx, x0 + dx, y0 + 4, 8, 18, WOOD.top);
        rect(ctx, x0 + dx + 1, y0 + 5, 6, 16, WOOD.d);
        rect(ctx, x0 + dx + 1, y0 + 5, 6, 1, WOOD.top);
    }
    rect(ctx, x0 + 11, y0 + 3, 1, 20, WOOD.k);
    if open {
        // the right door open: shelves inside, something on them
        rect(ctx, x0 + 12, y0 + 3, 9, 20, Color::rgb(0x1c1420));
        for y in [9, 15] {
            rect(ctx, x0 + 12, y0 + y, 9, 1, WOOD.d);
        }
        rect(ctx, x0 + 14, y0 + 6, 2, 3, SP::g);
        rect(ctx, x0 + 17, y0 + 11, 3, 4, SP::C);
        rect(ctx, x0 + 14, y0 + 17, 4, 2, SP::y);
        rect(ctx, x0 + 21, y0 + 2, 1, 21, WOOD.k);
        rect(ctx, x0 + 22, y0 + 3, 1, 20, WOOD.top);
    }
    rect(ctx, x0 + 9, y0 + 13, 1, 2, SP::C);
    if !open {
        rect(ctx, x0 + 12, y0 + 13, 1, 2, SP::C);
    }
    rect(ctx, x0 + 2, y0 + 23, 2, 1, WOOD.k); // its feet
    rect(ctx, x0 + 18, y0 + 23, 2, 1, WOOD.k);
}

/// burrow's own settings: a small steel box on the wall with one big lever, its red knob up; pulled down
/// while the settings' panel is open (a click on it opens them)
pub fn draw_switchboard(ctx: &mut Ctx, x0: i32, y0: i32, open: bool) {
    let (h, wd) = (sb_h(), sb_w());
    rect(ctx, x0, y0, wd, h, SP::F);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, h - 2, SP::D);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, 1, SP::C);
    rect(ctx, x0 + 1, y0 + 1, 1, h - 2, SP::C);
    rect(ctx, x0 + 4, y0 + 3, 2, h - 6, INK); // the slot
    let ky = if open { y0 + h - 4 } else { y0 - 1 }; // the knob: up, or pulled down
    rect(ctx, x0 + 4, (ky + 2).min(y0 + 7), 2, (ky + 2 - (y0 + 7)).abs(), SP::B); // the handle, from the slot's middle
    rect(ctx, x0 + 3, ky, 4, 4, INK);
    rect(ctx, x0 + 3, ky, 3, 3, SP::r);
    rect(ctx, x0 + 3, ky, 1, 1, Color::rgb(0xff9a8a));
    rect(ctx, x0 + 2, y0 + h - 2, 1, 1, SP::F); // the screws
    rect(ctx, x0 + wd - 3, y0 + h - 2, 1, 1, SP::F);
}

/// a machine's door in its paint: ajar while someone comes or goes, boarded up while it is out of reach
pub fn draw_door(ctx: &mut Ctx, x: i32, p: &Paint, ajar: bool, boarded: bool, y0: i32) {
    let im = sprites::swap(if ajar { "doorOpen" } else { "door" }, p.hi, p.top, p.d, p.k);
    put(ctx, &im, x, y0, false);
    if boarded {
        for y in [y0 + 4, y0 + 11] {
            // two planks nailed across it
            rect(ctx, x - 1, y, 18, 3, WOOD.k);
            rect(ctx, x, y + 1, 16, 1, WOOD.top);
            rect(ctx, x, y, 16, 1, WOOD.hi);
            rect(ctx, x + 1, y + 1, 1, 1, C::STEEL1);
            rect(ctx, x + 14, y + 1, 1, 1, C::STEEL1);
        }
    }
}
/// the door's default height on the wall
pub const DOOR_Y: i32 = WALL - 20;

/// a door in a paint (-1 the wood), for the doors' panel: its picture, `s` pixels a dot
pub fn door_picture(i: i32, s: u32) -> Image {
    let p = paint_at(i);
    let mut im = Image::new(16, 20);
    put(&mut im.ctx(), &sprites::swap("door", p.hi, p.top, p.d, p.k), 0, 0, false);
    im.scaled(s)
}

/// this computer's time, as the clock and the window read it
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clock {
    pub h: u32,
    pub m: u32,
    pub s: u32,
}

/// what keeps the computer awake (the clock's panel): off, while local bots work, or for a while
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwakeMode {
    Off,
    Work,
    For,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Awake {
    pub mode: AwakeMode,
    /// for a while: how long is left (until - now), in milliseconds
    pub left_ms: f64,
    /// the inhibitor runs (a local bot at work)
    pub on: bool,
}

/// the clock's cells, by their distance from the middle (8, 8): its face, its rim
pub struct ClockCells {
    pub face: Vec<(i32, i32)>,
    pub rim: Vec<(i32, i32, f64)>,
}
pub fn clock_cells() -> &'static ClockCells {
    static CELLS: std::sync::OnceLock<ClockCells> = std::sync::OnceLock::new();
    CELLS.get_or_init(|| {
        let (mut face, mut rim) = (Vec::new(), Vec::new());
        for j in 0..17 {
            for i in 0..17 {
                let d = ((i - 8) as f64).hypot((j - 8) as f64);
                if d <= 6.6 {
                    face.push((i, j));
                } else if d <= 8.4 {
                    rim.push((i, j, d));
                }
            }
        }
        ClockCells { face, rim }
    })
}

/// The clock, always on the wall, telling this computer's time: a round face 17 dots across in
/// Pixel Agents' colours (its rim red above, slate below), the hours as dots, the quarters darker; the hour hand
/// short and black, the minute hand long, the seconds' red one ticking. While it keeps the computer awake its face
/// is yellowed: for a while, a wedge from the hour hand to the hour it ends (shrinking as the time runs out, as a
/// kitchen timer); while local bots work, all of it; set to wait for them and none at work, hatched
pub fn draw_clock(ctx: &mut Ctx, x0: i32, y0: i32, a: Option<&Awake>, time: Clock, still: bool) {
    let (cx, cy) = (x0 + 8, y0 + 8);
    let cells = clock_cells();
    for &(i, j, d) in &cells.rim {
        let c = if j < 8 {
            if d < 7.4 { SP::r } else { SP::R }
        } else if d < 7.4 {
            SP::D
        } else {
            SP::G
        };
        rect(ctx, x0 + i, y0 + j, 1, 1, c);
    }
    for &(i, j) in &cells.face {
        rect(ctx, x0 + i, y0 + j, 1, 1, Color::rgb(0xededed));
    }
    let hour = (((time.h % 12) as f64 + time.m as f64 / 60.) / 12.) * 2. * PI;
    if let Some(a) = a.filter(|a| a.mode != AwakeMode::Off) {
        let left = if a.mode == AwakeMode::For { a.left_ms } else { 0. };
        let span = if left > 0. { (PI / 12.).max(left / 432e5 * 2. * PI) } else { 0. }; // 12 h round; never under half an hour, or the hands hide it
        for &(i, j) in &cells.face {
            // a cell the wedge reaches at all (its half width in angle past the end), so the last minutes still show
            let (dx, dy) = ((i - 8) as f64, (8 - j) as f64);
            let on = if a.mode == AwakeMode::For {
                span > 0. && (dx.atan2(dy) - hour + 4. * PI) % (2. * PI) < span + 0.5f64.atan2(dx.hypot(dy))
            } else {
                a.on || (i + j) % 3 == 0
            };
            if on {
                rect(ctx, x0 + i, y0 + j, 1, 1, SP::Y);
            }
        }
    }
    for k in 0..12 {
        // the hours
        let ang = k as f64 / 12. * 2. * PI;
        rect(
            ctx,
            js_round(cx as f64 + ang.sin() * 5.6),
            js_round(cy as f64 - ang.cos() * 5.6),
            1,
            1,
            if k % 3 != 0 { SP::C } else { SP::E },
        );
    }
    let hand = |ctx: &mut Ctx, ang: f64, len: f64, col: Color| {
        // a line from the middle, half a dot a step
        let mut r = 1.;
        while r <= len {
            rect(ctx, js_round(cx as f64 + ang.sin() * r), js_round(cy as f64 - ang.cos() * r), 1, 1, col);
            r += 0.5;
        }
    };
    hand(ctx, time.m as f64 / 60. * 2. * PI + time.s as f64 / 3600. * 2. * PI, 5.5, SP::F);
    hand(ctx, hour, 3.5, INK);
    if !still {
        hand(ctx, time.s as f64 / 60. * 2. * PI, 5., SP::r);
    }
    rect(ctx, cx, cy, 1, 1, SP::r);
}

/// a window: day or night outside, by this computer's clock (`hour`)
pub fn draw_window(ctx: &mut Ctx, x0: i32, y0: i32, hour: u32) {
    let day = (7..20).contains(&hour);
    let (gx, gy) = (x0 + 3, y0 + 2);
    rect(ctx, gx, gy, 15, 11, if day { Color::rgb(0x7fb2e0) } else { C::BLUE0 });
    if day {
        rect(ctx, gx, gy + 8, 15, 3, Color::rgb(0xb7d9f2));
        rect(ctx, gx + 2, gy + 2, 3, 1, C::STEEL4);
        rect(ctx, gx + 1, gy + 3, 6, 1, C::STEEL4);
        rect(ctx, gx + 10, gy + 5, 4, 1, C::STEEL4);
    } else {
        rect(ctx, gx + 2, gy + 2, 1, 1, C::STEEL3);
        rect(ctx, gx + 5, gy + 8, 1, 1, C::STEEL2);
        rect(ctx, gx + 9, gy + 6, 1, 1, C::STEEL3);
        rect(ctx, gx + 11, gy + 1, 3, 3, C::GOLD2);
        rect(ctx, gx + 12, gy + 1, 2, 2, C::BLUE0);
    }
    put_named(ctx, "window", x0, y0);
    let glint = Color::rgba(255, 255, 255, 77); // the glass catches the light (rgba(255,255,255,0.3))
    for px in [gx + 1, gx + 9] {
        rect(ctx, px + 3, gy + 1, 1, 1, glint);
        rect(ctx, px + 2, gy + 2, 1, 1, glint);
        rect(ctx, px + 1, gy + 3, 1, 1, glint);
    }
}

/// Halloween: cobwebs in the wall's top corners (a room `w` dots wide), a spider letting itself down from the left one
/// and climbing back by the room's time `t` (still: it hangs)
pub fn draw_cobwebs(ctx: &mut Ctx, w: i32, t: f64, still: bool) {
    ctx.set_alpha(0.45);
    for (cx, d) in [(0, 1), (w - 2, -1)] {
        let at = |r: f64, a: f64| (cx + d * js_round(a.cos() * r), js_round(a.sin() * r));
        // (a dot once, however many threads cross it: it is see-through)
        let mut dots: Vec<(i32, i32)> = Vec::new();
        let mut dot = |p: (i32, i32)| {
            if !dots.contains(&p) {
                dots.push(p);
            }
        };
        for a in [1., 3., 5.] {
            for r in 2..=13 {
                dot(at(r as f64, a * PI / 12.)); // its threads out of the corner
            }
        }
        for r in [5, 10] {
            for k in 0..4 {
                // and round, from the ceiling to the wall, sagging
                for i in 1..r {
                    let a0 = if k == 0 { 0. } else { (2 * k - 1) as f64 * PI / 12. };
                    let a1 = if k == 3 { PI / 2. } else { (2 * k + 1) as f64 * PI / 12. };
                    let (rf, u) = (r as f64, i as f64 / r as f64);
                    let ((ax, ay), (bx, by)) = (at(rf, a0), at(rf, a1));
                    let sag = (u * PI).sin() * rf / 8.;
                    dot((
                        js_round(ax as f64 + (bx - ax) as f64 * u - d as f64 * sag * 0.7),
                        js_round(ay as f64 + (by - ay) as f64 * u - sag * 0.7),
                    ));
                }
            }
        }
        for (px, py) in dots {
            rect(ctx, px, py, 1, 1, C::STEEL3);
        }
    }
    ctx.set_alpha(1.);
    let (sx, sy) = (9, js_round(13. + if still { 0. } else { 5. * (t * 0.6).sin() }));
    rect(ctx, sx, 7, 1, sy - 7, pix::css("rgba(194,201,217,0.5)")); // its thread
    rect(ctx, sx - 1, sy, 2, 2, INK); // its body
    for (lx, ly) in [(-2, -1), (-2, 1), (1, -1), (1, 1)] {
        rect(ctx, sx + lx, sy + ly, 1, 1, INK); // its legs
    }
}

/// the floor and the wall behind everything, `w` x `h` dots: Pixel Agents' planks in rows, their seams
/// staggered, the navy wall, its baseboard, a soft shadow along its foot
pub fn floor(w: i32, h: i32) -> Image {
    let mut im = Image::new(w as u32, h as u32);
    {
        let ctx = &mut im.ctx();
        rect(ctx, 0, WALL, w, h - WALL, PLANK);
        let (mut y, mut row) = (WALL + 3, 0);
        while y < h {
            rect(ctx, 0, y, w, 1, SEAM);
            let mut x = -((fnv(&format!("r{row}")) % 48) as i32);
            while x < w {
                rect(ctx, x, y + 1, 1, 3, SEAM);
                x += 40 + (fnv(&format!("{row}:{x}")) % 4) as i32 * 8;
            }
            y += 4;
            row += 1;
        }
        rect(ctx, 0, 0, w, WALL, WALLC);
        rect(ctx, 0, WALL - 2, w, 2, BASEBOARD);
        rect(ctx, 0, WALL, w, 1, pix::css("rgba(0,0,0,0.3)"));
        rect(ctx, 0, WALL + 1, w, 1, pix::css("rgba(0,0,0,0.12)"));
    }
    im
}
