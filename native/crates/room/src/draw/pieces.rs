//! The furniture on the floor: the whiteboard, the pair table, the project cabinet, the reception, the
//! rug, a nap's mat, the booth and the gun closet, the radio, the neuralyzer, the meeting table, the
//! chess table, the stools, the coffee corner, the cooler, the printer, the pouf, the plants, the
//! ping-pong table, the dog house.

use super::*;
use crate::figure::mat;
use crate::font::{tiny, tiny_w};
use pix::css;
use std::sync::OnceLock;

// ---- the whiteboard on wheels, on the floor: its face 30 x 15, two posts, a base on four wheels. What is
// drawn on it is ink at five pixels a board dot (130 x 55): the host of a meeting draws boxes, lines and
// dots in its colour, you draw on it big in the view. A board dot is inked when any of its pixels is
// (the colour of the first)

/// where the ink goes on the board's face, in board dots
pub struct InkArea {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
pub const INK_AREA: InkArea = InkArea { x: 2, y: 2, w: 26, h: 11 };
/// pixels of ink a board dot
pub const INKS: i32 = 5;

/// a line of ink (Bresenham), `width` pixels square a step
#[allow(clippy::too_many_arguments)]
pub fn ink_line(ink: &mut Image, x0: i32, y0: i32, x1: i32, y1: i32, col: Color, width: i32) {
    let mut c = ink.ctx();
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut e, mut x, mut y) = (dx + dy, x0, y0);
    loop {
        rect(&mut c, x, y, width, width, col);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * e;
        if e2 >= dy {
            e += dy;
            x += sx;
        }
        if e2 <= dx {
            e += dx;
            y += sy;
        }
    }
}

/// the board's dots from the ink (keep them until the ink changes): a dot's middle pixel, else its first inked one
pub fn board_cells(ink: &Image) -> Vec<(i32, i32, Color)> {
    let mut out = Vec::new();
    let solid = |p: [u8; 4]| (p[3] > 128).then_some(Color::rgb(((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32));
    for j in 0..INK_AREA.h {
        for i in 0..INK_AREA.w {
            let mut col = solid(ink.get(i * INKS + 2, j * INKS + 2));
            'dot: for y in j * INKS..(j + 1) * INKS {
                for x in i * INKS..(i + 1) * INKS {
                    if col.is_some() {
                        break 'dot;
                    }
                    col = solid(ink.get(x, y));
                }
            }
            if let Some(c) = col {
                out.push((i, j, c));
            }
        }
    }
    out
}

/// the whiteboard standing at x, y (the bottom middle), with what is drawn on it
pub fn draw_board(ctx: &mut Ctx, x: i32, y: i32, cells: &[(i32, i32, Color)]) {
    let (fx, fy) = (x - 15, y - 26);
    rect(ctx, fx, fy, 30, 15, SP::F);
    rect(ctx, fx + 1, fy + 1, 28, 13, Color::rgb(0xefebe8));
    rect(ctx, fx + 1, fy + 14, 28, 1, SP::E); // the tray
    rect(ctx, fx + 3, fy + 13, 3, 1, SP::r); // markers
    rect(ctx, fx + 7, fy + 13, 3, 1, SP::v);
    for px in [fx + 3, fx + 26] {
        rect(ctx, px, fy + 15, 1, 8, SP::F);
        rect(ctx, px + 1, fy + 15, 1, 8, SP::E);
    }
    rect(ctx, fx + 1, fy + 23, 28, 2, SP::F);
    rect(ctx, fx + 2, fy + 23, 26, 1, SP::E);
    for px in [fx, fx + 5, fx + 24, fx + 29] {
        rect(ctx, px, fy + 25, 1, 2, SP::F);
    }
    rect(ctx, fx + 1, fy + 27, 28, 1, SHADOW);
    let (ix, iy) = (fx + INK_AREA.x, fy + INK_AREA.y);
    for &(i, j, col) in cells {
        rect(ctx, ix + i, iy + j, 1, 1, col);
    }
}

/// a pair room: a small round table with a speakerphone on it, its light on while a room is linked,
/// blinking while a bot is held over it (it would join the chat)
pub fn draw_pair(ctx: &mut Ctx, x: i32, y: i32, linked: bool, t: f64, over: bool) {
    let (pk, pd, pm) = (Color::rgb(0x3f3740), Color::rgb(0x595e60), Color::rgb(0x757b7c));
    for (j, hw) in ell(13, 4, 8).into_iter().enumerate() {
        let j = j as i32;
        rect(ctx, x - hw, y - 20 + j, hw * 2, 1, WOOD.k);
        if hw > 1 {
            rect(ctx, x - hw + 1, y - 20 + j, hw * 2 - 2, 1, if j < 2 { WOOD.hi } else { WOOD.top });
        }
    }
    for (yy, hw) in [(-12, 12), (-11, 11), (-10, 9)] {
        rect(ctx, x - hw, y + yy, hw * 2, 1, WOOD.k);
        rect(ctx, x - hw + 1, y + yy, hw * 2 - 2, 1, WOOD.d);
    }
    rect(ctx, x - 2, y - 9, 4, 8, WOOD.k); // the stem and the foot
    rect(ctx, x - 1, y - 9, 2, 8, WOOD.d);
    rect(ctx, x - 6, y - 1, 12, 2, WOOD.k);
    rect(ctx, x - 5, y - 1, 10, 1, WOOD.d);
    rect(ctx, x - 5, y + 1, 10, 1, SHADOW);
    // the speakerphone: a dark triangle-ish puck, a light on top
    rect(ctx, x - 4, y - 18, 8, 4, pk);
    rect(ctx, x - 3, y - 18, 6, 3, pd);
    rect(ctx, x - 3, y - 18, 6, 1, pm);
    rect(ctx, x - 6, y - 16, 2, 2, pk);
    rect(ctx, x + 4, y - 16, 2, 2, pk);
    rect(ctx, x - 1, y - 20, 2, 2, pk);
    let on = linked && (!over || (t * 6.).floor() as i64 % 2 != 0);
    rect(ctx, x, y - 17, 1, 1, if on { SP::g } else { Color::rgb(0x2a2f33) });
}

/// the empty cabinet that adds a project: a plate with a + on it (x0, y0 its top left)
pub fn draw_add(ctx: &mut Ctx, x0: i32, y0: i32, lit: bool) {
    slab(ctx, x0, y0, ADD_W, 4, 11, &WOOD);
    plate(ctx, x0 + 3, y0 + 6, ADD_W - 6, lit, &WOOD);
    tiny(ctx, "+", js_round(x0 as f64 + ADD_W as f64 / 2. - 1.) as f64, (y0 + 7) as f64, if lit { C::STEEL4 } else { C::STEEL2 });
}

/// what the reception's screen shows, for an office manager who is a session
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecScreen {
    Off,
    On,
    Busy,
    Asks,
}
/// the office manager at a reception: how many sessions have notes, her name, her colour, her screen
#[derive(Clone, Copy, Debug)]
pub struct Reception<'a> {
    pub n: usize,
    pub name: &'a str,
    pub color: Color,
    pub screen: Option<RecScreen>,
    pub t: f64,
}

/// the reception: an office manager's booth, 46 dots wide. Behind: two posts and a panel. In front, the counter (a
/// slab), her name on its front on a plate in her colour; on it a bell at the right end and, for one who is
/// a session, her screen at the left (it runs while a turn runs, its cursor blinks while she asks you, dark while
/// her process is off); for Linda a stack of notes as high as there are sessions with notes (six at most) and a pen.
/// `front` draws the counter, which goes over her
pub fn draw_reception(ctx: &mut Ctx, x: i32, y: i32, o: &Reception, front: bool) {
    if !front {
        for px in [x + 2, x + 42] {
            rect(ctx, px, y - 30, 2, 16, WOOD.k); // the posts
            rect(ctx, px, y - 30, 1, 15, WOOD.d);
        }
        rect(ctx, x + 4, y - 30, 38, 15, Color::rgb(0x2b2433)); // the panel
        rect(ctx, x + 4, y - 30, 38, 1, Color::rgb(0x1f1a26));
        for px in [x + 13, x + 23, x + 33] {
            rect(ctx, px, y - 29, 1, 14, Color::rgb(0x251f2d));
        }
        return;
    }
    slab(ctx, x, y - 16, 46, 4, 12, &WOOD); // the counter
    let nm = o.name.to_uppercase();
    if !nm.is_empty() {
        // her name on its front: a desk's plate, dark, the letters in her colour
        let tw = tiny_w(&nm);
        let px = x + 23 - (tw + 6 + 1) / 2;
        plate(ctx, px, y - 10, tw + 6, false, &Paint { k: WOOD.k, plate: o.color.shade(0.26), ..WOOD });
        tiny(ctx, &nm, (px + 3) as f64, (y - 9) as f64, o.color.mix(Color::rgb(0xffffff), 0.35));
    } else {
        for px in [x + 9, x + 18, x + 27, x + 36] {
            rect(ctx, px, y - 10, 1, 7, WOOD.k);
        }
    }
    rect(ctx, x + 39, y - 18, 5, 1, SP::F); // the bell
    rect(ctx, x + 40, y - 20, 3, 2, SP::y);
    rect(ctx, x + 40, y - 20, 1, 1, SP::Y);
    rect(ctx, x + 41, y - 21, 1, 1, SP::F);
    if let Some(sc) = o.screen {
        let (gx, gy) = (x + 3, y - 27);
        rect(ctx, gx - 1, gy - 1, 14, 11, SP::F); // the screen, its stand
        rect(ctx, gx + 4, gy + 10, 4, 1, SP::F);
        rect(ctx, gx + 2, gy + 11, 8, 1, SP::F);
        rect(ctx, gx, gy, 12, 9, if sc == RecScreen::Off { Color::rgb(0x10141f) } else { C::BLUE0 });
        if sc == RecScreen::Off {
            rect(ctx, gx + 9, gy + 1, 1, 1, Color::rgb(0x232c40));
            rect(ctx, gx + 8, gy + 2, 1, 1, Color::rgb(0x232c40));
        } else {
            let t = if sc == RecScreen::Busy { (o.t * 5.).floor() as i32 } else { 0 };
            let col = if sc == RecScreen::Busy { o.color } else { o.color.shade(0.45) };
            for r in 0..3 {
                rect(ctx, gx + 1 + (r * 3) % 4, gy + 1 + r * 2, 10.min(3 + ((r * 7 + t) * 5) % 8), 1, col);
            }
            let cursor = if sc == RecScreen::Asks { (o.t * 2.).floor() as i64 % 2 != 0 } else { sc == RecScreen::On };
            if cursor {
                rect(ctx, gx + 1, gy + 7, 2, 1, C::STEEL4);
            }
        }
    } else {
        let h = 6.min(o.n as i32);
        if h > 0 {
            rect(ctx, x + 5, y - 16 - h, 8, h + 1, INK);
            for k in 0..h {
                rect(ctx, x + 6, y - 16 - h + k, 6, 1, if k % 2 != 0 { SP::B } else { SP::A });
            }
        }
        rect(ctx, x + 15, y - 15, 5, 1, C::BLUE2); // the pen
        rect(ctx, x + 19, y - 15, 1, 1, INK);
    }
}

/// pair's rug under the ping-pong table, quiet, fringed at its ends
pub fn draw_rug(ctx: &mut Ctx, gx: i32, gy: i32) {
    let (x, y, w, h) = (gx - 36, gy - 22, 72, 28);
    rect(ctx, x, y, w, h, Color::rgb(0x2a2133));
    rect(ctx, x + 1, y + 1, w - 2, h - 2, Color::rgb(0x3a2d45));
    rect(ctx, x + 3, y + 3, w - 6, h - 6, Color::rgb(0x4a3a57));
    rect(ctx, x + 4, y + 4, w - 8, h - 8, Color::rgb(0x3a2d45));
    let mut j = y + 1;
    while j < y + h - 1 {
        rect(ctx, x - 1, j, 1, 1, Color::rgb(0x8a7a68));
        rect(ctx, x + w, j, 1, 1, Color::rgb(0x8a7a68));
        j += 2;
    }
}

/// a little mat rolled out on the floor for a nap: a thin flat one, 32 x 16, nothing more
pub fn draw_mat(ctx: &mut Ctx, x: i32, y: i32) {
    rect(ctx, x - 16, y - 17, 32, 16, mat::K);
    rect(ctx, x - 15, y - 16, 30, 14, mat::TOP);
    rect(ctx, x - 15, y - 16, 30, 1, mat::HI);
    rect(ctx, x - 15, y - 3, 30, 1, mat::D);
}

// ---- the booth's five seconds, by how far in they are (an undo runs them back): the door slides shut, a
// light rises through the cabin and takes the bot up with it, feet first, the cabin glowing more and more;
// then it is all light, and at the end the bot is gone. No countdown shows: the booth is the clock
pub const BOOTH: f64 = 5.;
pub fn booth_shut(t: f64) -> f64 {
    (t / 0.4).min(1.)
}
pub fn booth_rise(t: f64) -> f64 {
    ((t - 0.6) / 3.4).clamp(0., 1.)
}
pub fn booth_glow(t: f64) -> f64 {
    ((t - 0.3) / 3.9).clamp(0., 1.)
}

/// the booth (Futurama's): a tall glass cabin on a plinth, a lit sign on its roof. A bot put in it (t: seconds in,
/// -1 while empty) stands behind the glass: the door slides shut, the sign goes red, a light rises from its floor
/// and takes the bot with it (dissolve), the cabin glows brighter and brighter, the sign flickers in the last
/// second; then the bot is gone and the light fades (flash, -1 when there is none). Two layers: the cabin
/// (`front` false), and its glass door in front of whoever is inside. `tt` is the room's time
#[allow(clippy::too_many_arguments)]
pub fn draw_booth(ctx: &mut Ctx, bx: i32, y: i32, front: bool, t: f64, flash: f64, armed: bool, tt: f64, still: bool) {
    let x0 = bx - 9;
    let top = y - 32;
    let gy = top + 5;
    let gh = y - 3 - gy;
    let busy = t >= 0. || armed;
    let glow = if t >= 0. { booth_glow(t) } else { 0. };
    let rise = if t >= 0. { booth_rise(t) } else { 0. };
    let lh = js_round(gh as f64 * (rise * 1.1).min(1.));
    let light = Color::rgb(0xfff4d6);
    if !front {
        rect(ctx, x0 + 1, y, 16, 1, SHADOW);
        rect(ctx, x0, top, 18, 5, SP::F); // the roof
        rect(ctx, x0 + 1, top + 1, 16, 3, SP::E);
        rect(ctx, x0 + 1, top + 1, 16, 1, SP::D);
        let flick = t > BOOTH - 1. && !still && (tt * 8.).floor() as i64 % 2 != 0;
        rect(ctx, x0 + 4, top + 1, 10, 3, INK); // its sign
        rect(ctx, x0 + 5, top + 2, 8, 1, if busy && !flick { SP::r } else { SP::Y });
        rect(ctx, x0, gy, 18, gh, SP::F); // the posts
        rect(ctx, x0 + 1, gy, 1, gh, SP::D);
        rect(ctx, x0 + 16, gy, 1, gh, SP::D);
        rect(ctx, x0 + 2, gy, 14, gh, if busy { Color::rgb(0x3b3428) } else { Color::rgb(0x222a37) }); // inside, warm while in use
        rect(ctx, x0 + 2, y - 5, 14, 2, if busy { Color::rgb(0x4a412f) } else { Color::rgb(0x2b3442) }); // its floor
        if glow != 0. {
            // the light, behind the bot
            ctx.set_alpha(0.15 + 0.3 * glow);
            rect(ctx, x0 + 2, gy, 14, gh - lh, light);
            ctx.set_alpha(0.45 + 0.5 * glow);
            rect(ctx, x0 + 2, gy + gh - lh, 14, lh, light);
            ctx.set_alpha(1.);
        }
        rect(ctx, x0, y - 3, 18, 3, SP::F); // the plinth
        rect(ctx, x0 + 1, y - 3, 16, 1, SP::C);
        rect(ctx, x0 + 1, y - 2, 16, 1, SP::D);
        return;
    }
    if glow != 0. {
        // the light, before it
        ctx.set_alpha(0.35 * glow);
        rect(ctx, x0 + 2, gy, 14, gh, light);
        ctx.set_alpha(1.);
        if rise > 0. && rise < 1. {
            rect(ctx, x0 + 2, gy + gh - lh, 14, 1, Color::rgb(0xfffaf0)); // its rising edge
        }
        if rise > 0. && !still {
            for k in 0..7 {
                // motes going up in it, only where the light has been
                let ph = (tt * 0.8 + k as f64 / 7.) % 1.;
                let my = y - 5 - js_round(ph * (gh - 3) as f64);
                if my < gy + gh - lh - 1 && rise < 1. {
                    continue;
                }
                ctx.set_alpha(1. - ph);
                rect(ctx, x0 + 3 + (k * 5 + 2) % 12, my, 1, 1, if k % 2 != 0 { Color::rgb(0xffffff) } else { SP::Y });
                ctx.set_alpha(1.);
            }
        }
    }
    let off = if t >= 0. { js_round(13. * (1. - booth_shut(t))) } else { 0 }; // the door, sliding in from the right
    let (dx, dw) = (x0 + 2 + off, 14 - off);
    ctx.set_alpha(0.25); // its glass
    rect(ctx, dx, gy, dw, gh, Color::rgb(0xbfe3ff));
    ctx.set_alpha(1.);
    for k in 0..4 {
        if 4 + k >= off + 2 {
            rect(ctx, x0 + 4 + k, gy + 4 + k, 1, 1, css("rgba(255,255,255,0.5)"));
        }
        if 10 + k >= off + 2 {
            rect(ctx, x0 + 10 + k, gy + 11 + k, 1, 1, css("rgba(255,255,255,0.3)"));
        }
    }
    rect(ctx, x0 + 2, gy, 14, 1, SP::D); // the rail; the door's edge and handle
    rect(ctx, dx, gy + 1, 1, gh - 1, SP::D);
    rect(ctx, dx + 1, gy + 10, 1, 4, SP::B);
    if (0. ..1.).contains(&flash) {
        ctx.set_alpha(0.95 * (1. - flash)); // the light, fading
        rect(ctx, x0 + 2, gy, 14, gh, light);
        ctx.set_alpha(1.);
    }
}

/// hardcore: the booth is a gun closet: an olive steel cabinet, two shotguns behind its glass doors (one
/// gone while it is in someone's hands: `out` of them), a red lamp on top
pub fn draw_gun_closet(ctx: &mut Ctx, bx: i32, y: i32, front: bool, out: i32) {
    let x0 = bx - 9;
    let top = y - 32;
    let gy = top + 5;
    let gh = y - 3 - gy;
    if !front {
        rect(ctx, x0 + 1, y, 16, 1, SHADOW);
        rect(ctx, x0, top, 18, y - top, SP::F);
        rect(ctx, x0 + 1, top + 1, 16, 3, Color::rgb(0x4f5844));
        rect(ctx, x0 + 1, top + 1, 16, 1, Color::rgb(0x6b7560));
        rect(ctx, x0 + 6, top + 1, 6, 3, INK); // the lamp
        rect(ctx, x0 + 7, top + 2, 4, 1, SP::r);
        rect(ctx, x0 + 1, gy, 16, gh, Color::rgb(0x3c4434)); // the cabinet, inside
        rect(ctx, x0 + 2, gy, 14, gh, Color::rgb(0x1a1e16));
        for (k, gx) in [(0, x0 + 5), (1, x0 + 12)] {
            if k < 2 - out.clamp(0, 2) {
                rect(ctx, gx, gy + 2, 1, 13, Color::rgb(0xa7aebb)); // a barrel
                rect(ctx, gx + 1, gy + 2, 1, 13, Color::rgb(0x6c7380));
                rect(ctx, gx - 1, gy + 14, 3, 7, Color::rgb(0x7a4a2a)); // its stock
                rect(ctx, gx - 1, gy + 14, 1, 7, Color::rgb(0x9a6a3a));
            }
        }
        rect(ctx, x0, y - 3, 18, 3, SP::F); // the plinth
        rect(ctx, x0 + 1, y - 3, 16, 1, Color::rgb(0x6b7560));
        rect(ctx, x0 + 1, y - 2, 16, 1, Color::rgb(0x4f5844));
        return;
    }
    ctx.set_alpha(0.18); // the glass
    rect(ctx, x0 + 2, gy, 14, gh, Color::rgb(0xbfe3ff));
    ctx.set_alpha(1.);
    for k in 0..4 {
        rect(ctx, x0 + 3 + k, gy + 3 + k, 1, 1, css("rgba(255,255,255,0.45)"));
    }
    rect(ctx, x0 + 8, gy, 2, gh, SP::F); // where the doors meet, the handles
    rect(ctx, x0 + 7, gy + 10, 1, 3, SP::B);
    rect(ctx, x0 + 10, gy + 10, 1, 3, SP::B);
}

/// the radio: a midcentury set on splayed legs, a woven cloth over its speaker, a dial lit (and notes rising from it,
/// draw_notes) while the room's sound is on
pub fn draw_radio(ctx: &mut Ctx, x: i32, y: i32, on: bool) {
    rect(ctx, x - 6, y, 12, 1, SHADOW);
    for (lx, d) in [(-5, -1), (4, 1)] {
        rect(ctx, x + lx, y - 4, 1, 2, WOOD.k); // the legs
        rect(ctx, x + lx + d, y - 2, 1, 2, WOOD.k);
    }
    rect(ctx, x - 7, y - 19, 14, 16, WOOD.k); // the cabinet, its top corners rounded
    rect(ctx, x - 6, y - 20, 12, 1, WOOD.k);
    rect(ctx, x - 6, y - 19, 12, 15, WOOD.top);
    rect(ctx, x - 5, y - 19, 10, 1, WOOD.hi);
    rect(ctx, x - 6, y - 5, 12, 1, WOOD.d);
    rect(ctx, x - 5, y - 17, 10, 3, if on { SP::Y } else { Color::rgb(0x3a2c26) }); // the dial across its top: its marks, the needle
    for mx in [x - 4, x - 2, x, x + 3] {
        rect(ctx, mx, y - 17, 1, 1, if on { Color::rgb(0xc98a2c) } else { Color::rgb(0x2a2026) });
    }
    rect(ctx, x + 1, y - 17, 1, 3, if on { SP::r } else { Color::rgb(0x2a2026) });
    for j in 0..6 {
        for i in 0..10 {
            rect(ctx, x - 5 + i, y - 13 + j, 1, 1, if (i + j) % 2 != 0 { Color::rgb(0xd8c49a) } else { Color::rgb(0xbfa97e) }); // the speaker's cloth
        }
    }
    for sx in [x - 3, x, x + 3] {
        rect(ctx, sx, y - 13, 1, 6, WOOD.d); // behind three slats
    }
    for kx in [x - 5, x + 3] {
        rect(ctx, kx, y - 7, 2, 2, SP::F); // the knobs
        rect(ctx, kx, y - 7, 1, 1, SP::C);
    }
}
/// two notes drifting up from the radio, fading
pub fn draw_notes(ctx: &mut Ctx, x: i32, y: i32, t: f64) {
    for k in [0, 1] {
        let ph = (t * 0.45 + k as f64 * 0.5) % 1.;
        let a = if ph < 0.15 { ph / 0.15 } else { 1. - (ph - 0.15) / 0.85 };
        let nx = js_round(x as f64 - 3. + k as f64 * 5. + (ph * 5. + k as f64 * 2.).sin() * 2.);
        let ny = js_round(y as f64 - 25. - ph * 14.);
        ctx.set_alpha(a.max(0.));
        rect(ctx, nx, ny + 3, 2, 2, INK);
        rect(ctx, nx + 1, ny, 1, 4, INK);
        rect(ctx, nx + 2, ny, 1, 1, INK);
        ctx.set_alpha(1.);
    }
}

/// the neuralyzer (Men in Black's): a black pedestal, the silver flasher on it; its bulb lit while it flashes
pub fn draw_neuralyzer(ctx: &mut Ctx, x: i32, y: i32, lit: bool) {
    rect(ctx, x - 5, y, 10, 1, SHADOW);
    rect(ctx, x - 4, y - 12, 8, 12, SP::F); // the pedestal
    rect(ctx, x - 3, y - 12, 6, 11, Color::rgb(0x2a2e35));
    rect(ctx, x - 3, y - 12, 1, 11, SP::E);
    rect(ctx, x - 6, y - 14, 12, 2, SP::F); // its top
    rect(ctx, x - 5, y - 14, 10, 1, SP::C);
    rect(ctx, x - 5, y - 18, 10, 4, SP::F); // the flasher
    rect(ctx, x - 4, y - 17, 8, 2, SP::B);
    rect(ctx, x - 4, y - 17, 8, 1, SP::A);
    rect(ctx, x + 2, y - 16, 1, 1, SP::r); // its button
    rect(ctx, x - 5, y - 17, 1, 2, if lit { SP::A } else { SP::Y }); // its bulb
}

/// Pixel Agents' table, papers and a mug on it
pub fn draw_table(ctx: &mut Ctx, x: i32, y: i32) {
    put_named(ctx, "SMALL_TABLE_FRONT", x - 16, y - 30);
    rect(ctx, x - 11, y - 16, 7, 5, SP::C);
    rect(ctx, x - 11, y - 16, 6, 4, SP::A);
    rect(ctx, x - 10, y - 15, 4, 1, SP::C);
    rect(ctx, x - 10, y - 13, 3, 1, SP::C);
    put_named(ctx, "COFFEE", x - 4, y - 16);
}

/// the chess position the game begins with, a file of eight a rank (room.js's START)
pub const START: &str = "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr";
/// a small table, the game on it as it stands (`board`, START if none): white plays from the left
pub fn draw_chess(ctx: &mut Ctx, x: i32, y: i32, board: Option<&str>) {
    let b = board.unwrap_or(START).as_bytes();
    let (x0, y0) = (x - 10, y - 17);
    put_named(ctx, "chess", x0, y0);
    for r in 0..8 {
        for f in 0..8 {
            rect(ctx, x0 + 2 + r * 2, y0 + 9 - f, 2, 1, if (r + f) % 2 != 0 { Color::rgb(0xe8d2a0) } else { Color::rgb(0xa8784c) });
        }
    }
    for f in (0..8).rev() {
        for r in 0..8 {
            // the pieces stand up from their squares, back to front
            let Some(&p) = b.get((r * 8 + f) as usize) else { continue };
            if p == b'.' {
                continue;
            }
            let white = p.is_ascii_uppercase();
            let tall = if b"KQkq".contains(&p) { 3 } else if b"Pp".contains(&p) { 1 } else { 2 };
            let px = x0 + 2 + r * 2 + if white { 0 } else { 1 };
            let py = y0 + 9 - f;
            rect(ctx, px, py - tall + 1, 1, tall, if white { SP::A } else { SP::x });
        }
    }
}

/// Pixel Agents' bench
pub fn draw_stool(ctx: &mut Ctx, x: i32, y: i32) {
    put_named(ctx, "WOODEN_BENCH", x - 8, y - 15);
}

/// the coffee corner; its pot is the five-hour session's limit (`used`, percent): full when nothing is used yet
pub fn draw_coffee(ctx: &mut Ctx, x0: i32, y: i32, brewing: bool, t: f64, used: Option<f64>) {
    put_named(ctx, "counter", x0, y - 12);
    put_named(ctx, "brewer", x0 + 1, y - 20);
    if brewing {
        rect(ctx, x0 + 5, y - 15 + ((t * 8.).floor() as i32 % 2), 1, 1, Color::rgb(0x8a5030));
        rect(ctx, x0 + 4, y - 22 - ((t * 4.).floor() as i32 % 2), 1, 1, C::STEEL3);
    }
    // the pot: glass, the coffee in it with a lighter top, one dot = a seventh
    let (px, py) = (x0 + 11, y - 20);
    rect(ctx, px + 1, py + 3, 5, 7, Color::rgb(0xb9c6d2));
    let left = used.map(|u| js_round(7. * (1. - u / 100.)).clamp(0, 7)).unwrap_or(0);
    if left > 0 {
        rect(ctx, px + 1, py + 10 - left, 5, left, Color::rgb(0x6b3a22));
        rect(ctx, px + 1, py + 10 - left, 5, 1, Color::rgb(0xb0703c));
    }
    put_named(ctx, "pot", px, py);
}

/// the water cooler; its bottle is the week's limit
pub fn draw_cooler(ctx: &mut Ctx, x: i32, y: i32, used: Option<f64>) {
    put_named(ctx, "cooler", x - 5, y - 14);
    let (bx, by) = (x - 4, y - 25);
    rect(ctx, bx + 2, by + 2, 4, 1, Color::rgb(0xcfe0ec));
    rect(ctx, bx + 1, by + 3, 6, 8, Color::rgb(0xcfe0ec));
    let left = used.map(|u| js_round(8. * (1. - u / 100.)).clamp(0, 8)).unwrap_or(0);
    if left > 0 {
        rect(ctx, bx + 1, by + 11 - left, 6, left, C::BLUE2);
        rect(ctx, bx + 1, by + 11 - left, 6, 1, C::BLUE3);
    }
    put_named(ctx, "bottle", bx, by);
}

/// the printer on its stand; a page comes out of the front while it prints (`wait`: seconds into it)
pub fn draw_printer(ctx: &mut Ctx, x: i32, y: i32, wait: Option<f64>) {
    put_named(ctx, "stand", x - 7, y - 6);
    put_named(ctx, "printer", x - 7, y - 14);
    if let Some(w) = wait {
        let k = 4.min((w * 2.).floor() as i32);
        box_(ctx, x - 4, y - 8, 8, k + 1, C::STEEL4, SP::D);
    }
}

// the pouf (the kind is still "armchair"): a square green cushion seen from above at a slant, as the furniture
// is, so it is the same from each of its four sides; its corners rounded, its top lit along the front edge, a
// button in the middle, on two little splayed wooden feet. Pixel Agents' armchair, its seat without the back
const POUF: &[(char, u32)] = &[('k', 0x173115), ('h', 0x8cbf8c), ('t', 0x6fa374), ('s', 0x487841), ('d', 0x355f31), ('w', 0x885c47), ('x', 0x532e3d)];
const POUF_ROWS: [&str; 14] = [
    "..kkkkkkkkkkkk..",
    ".kttttttttttttk.",
    "kttttttttttttttk",
    "kttttttttttttttk",
    "kttttttssttttttk",
    "kttttttttttttttk",
    "kttttttttttttttk",
    "khhhhhhhhhhhhhhk",
    "kssssssssssssssk",
    "kssssssssssssssk",
    ".kddddddddddddk.",
    "..kkkkkkkkkkkk..",
    "..wx........wx..",
    ".wx..........wx.",
];
pub fn pouf() -> &'static Image {
    static PIC: OnceLock<Image> = OnceLock::new();
    PIC.get_or_init(|| {
        let mut im = Image::new(16, POUF_ROWS.len() as u32);
        for (j, r) in POUF_ROWS.iter().enumerate() {
            for (i, ch) in r.chars().enumerate() {
                if let Some(&(_, c)) = POUF.iter().find(|(k, _)| *k == ch) {
                    im.set(i as i32, j as i32, Color::rgb(c).0);
                }
            }
        }
        im
    })
}
pub fn draw_armchair(ctx: &mut Ctx, x: i32, y: i32) {
    rect(ctx, x - 7, y, 14, 1, SHADOW);
    put(ctx, pouf(), x - 8, y - 14, false);
}

/// Pixel Agents' plants (`i` 0 or 1: which); stirred (`stir_ms`: how long since the pointer came), the leaves sway twice
pub fn draw_plant(ctx: &mut Ctx, x: i32, y: i32, i: u32, stir_ms: Option<f64>) {
    let Some(im) = sprites::get(if i != 0 { "PLANT_2" } else { "PLANT" }) else { return };
    let k = stir_ms.map(|ms| (ms / 125.).floor() as i64).unwrap_or(4);
    if k >= 4 {
        put(ctx, &im, x - 8, y - 30, false);
        return;
    }
    let sway = [-1, 0, 1, 0][k.clamp(0, 3) as usize];
    let (w, h) = (im.w as f64, im.h as f64);
    let cut = h - 12.;
    ctx.draw_image_sub(&im, 0., 0., w, cut, (x - 8 + sway) as f64, (y - 30) as f64, w, cut);
    ctx.draw_image_sub(&im, 0., cut, w, h - cut, (x - 8) as f64, y as f64 - 30. + cut, w, h - cut);
}

/// the ball in a game of ping-pong: `t` 0..1 across, from the left (`from` false) or the right
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PongBall {
    pub t: f64,
    pub from: bool,
}
/// the ping-pong table; the ball, once both players are at it
pub fn draw_pong(ctx: &mut Ctx, gx: i32, gy: i32, ball: Option<PongBall>) {
    put_named(ctx, "pong", gx - 16, gy - 19);
    if let Some(PongBall { t, from }) = ball {
        let a = if from { 1. } else { -1. };
        let x = gx as f64 + a * 18. * (1. - 2. * t);
        let k = if t < 0.5 { t * 2. } else { (t - 0.5) * 2. };
        let y = gy as f64 - 12. - js_round((k * std::f64::consts::PI).sin() * if t < 0.5 { 6. } else { 8. }) as f64 - 6. * (1. - (t * 4.).min(1.));
        let (rx, ry) = (js_round(x), js_round(y));
        rect(ctx, rx - 1, ry - 1, 3, 3, INK);
        rect(ctx, rx, ry, 1, 1, C::WHITE);
    }
}

/// the dog house: a wooden kennel, a red gabled roof, a dark arched door; its bowl by the corner
pub fn draw_doghouse(ctx: &mut Ctx, x: i32, y: i32) {
    rect(ctx, x - 10, y, 20, 1, SHADOW);
    rect(ctx, x - 9, y - 12, 18, 12, WOOD.k);
    rect(ctx, x - 8, y - 11, 16, 11, WOOD.top);
    for j in [y - 8, y - 5, y - 2] {
        rect(ctx, x - 8, j, 16, 1, WOOD.d);
    }
    for j in 0..9 {
        // the roof, a gable, overhanging the walls
        let hw = 2. + j as f64 * 1.25;
        let ry = y - 20 + j;
        rect(ctx, js_round(x as f64 - hw - 1.), ry, js_round(hw * 2. + 2.), 1, Color::rgb(0x5a2420));
        if j > 0 {
            let c = if j < 3 {
                Color::rgb(0xc9574b)
            } else if j < 7 {
                Color::rgb(0xa8433a)
            } else {
                Color::rgb(0x8a3530)
            };
            rect(ctx, js_round(x as f64 - hw), ry, js_round(hw * 2.), 1, c);
        }
    }
    let door = Color::rgb(0x1e1512); // the door
    rect(ctx, x - 4, y - 8, 8, 8, door);
    rect(ctx, x - 3, y - 9, 6, 1, door);
    rect(ctx, x - 5, y - 8, 1, 8, WOOD.k);
    rect(ctx, x + 4, y - 8, 1, 8, WOOD.k);
    rect(ctx, x - 12, y - 2, 5, 2, SP::F); // the bowl
    rect(ctx, x - 11, y - 2, 3, 1, Color::rgb(0x8a5a32));
}
