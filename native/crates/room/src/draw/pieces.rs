//! The furniture on the floor: the easel, the pair table, a new desk standing blank, the reception, the
//! rug, a nap's mat, the booth and the gun closet, the radio, the neuralyzer, the meeting table, the
//! chess table, the stools, the coffee corner, the cooler, the printer, the pouf, the plants, the
//! ping-pong table, the dog house.

use super::*;
use crate::figure::mat;
use crate::font::{tiny, tiny_w};
use pix::css;
use std::sync::OnceLock;

// ---- the easel: two legs splayed in front and one behind, a ledge with a few spots of paint, the canvas on it
// (upright or across, as the painting is) clamped at the top; on it the painting at a third of its size (`small`:
// art::art_small), or the bare primer

/// the easel standing at x, y (the bottom middle); `i`: which painting is on it (its size), `small` its dots
pub fn draw_easel(ctx: &mut Ctx, x: i32, y: i32, art: Option<(usize, &[(i32, i32, Color)])>) {
    let p = &crate::paintings::list()[art.map_or(0, |a| a.0)];
    let (cw, ch) = (p.w / 3, p.h / 3);
    let cx0 = x - (cw as f64 / 2.0).floor() as i32;
    let cy1 = y - 11;
    let cy0 = cy1 - ch;
    let leg = |ctx: &mut Ctx, x0: i32, y0: i32, x1: i32, y1: i32, col: Color| {
        let n = ((x1 - x0) as f64).hypot((y1 - y0) as f64).ceil() as i32;
        for k in 0..=n {
            let (fx, fy) = if n == 0 { (x0 as f64, y0 as f64) } else { (x0 as f64 + ((x1 - x0) * k) as f64 / n as f64, y0 as f64 + ((y1 - y0) * k) as f64 / n as f64) };
            rect(ctx, js_round(fx), js_round(fy), 1, 1, col);
        }
    };
    rect(ctx, x - 10, y + 1, 21, 1, SHADOW);
    leg(ctx, x, cy1, x + 1, y, WOOD.k); // the back leg, the mast
    leg(ctx, x - 1, cy0 - 3, x - 1, cy1, WOOD.k);
    leg(ctx, x, cy0 - 3, x, cy1, WOOD.d);
    for s in [-1, 1] {
        let o = if s < 0 { 1 } else { -1 };
        leg(ctx, x + s * 4, cy1 + 1, x + s * 9, y, WOOD.k);
        leg(ctx, x + s * 4 + o, cy1 + 1, x + s * 9 + o, y, WOOD.top);
    }
    rect(ctx, x - 6, y - 6, 13, 1, WOOD.k); // the crossbar
    rect(ctx, cx0 - 1, cy0 - 1, cw + 2, ch + 2, Color::rgb(0x3f3740)); // the canvas, its edge
    match art {
        Some((_, small)) => {
            for &(i, j, col) in small {
                rect(ctx, cx0 + i, cy0 + j, 1, 1, col);
            }
        }
        None => rect(ctx, cx0, cy0, cw, ch, crate::paintings::PRIMER),
    }
    rect(ctx, x - 2, cy0 - 3, 5, 2, WOOD.k); // the clamp
    rect(ctx, x - 1, cy0 - 3, 3, 1, WOOD.top);
    rect(ctx, cx0 - 3, cy1 + 1, cw + 6, 2, WOOD.k); // the ledge
    rect(ctx, cx0 - 2, cy1 + 1, cw + 4, 1, WOOD.top);
    rect(ctx, cx0, cy1 + 1, 1, 1, C::RED1);
    rect(ctx, cx0 + 2, cy1 + 1, 1, 1, C::BLUE2);
    rect(ctx, cx0 + cw - 2, cy1 + 1, 1, 1, C::GOLD2);
}

/// a pair room: a small round table with a telephone on it, its light on while a room is linked,
/// blinking while a bot is held over it (it would join the chat)
/// hand: where you hold its handset while its room is open, and the side you stand (the cradle empty, the cord
/// coiling from the base to your hand)
pub fn draw_pair(ctx: &mut Ctx, x: i32, y: i32, linked: bool, t: f64, over: bool, hand: Option<(i32, i32, i32)>) {
    let (pk, pd, pm, phi) = (Color::rgb(0x3f3740), Color::rgb(0x595e60), Color::rgb(0x757b7c), Color::rgb(0xb5bfc7));
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
    // the telephone: its base, three keys and the light along its front, the handset across the cradle on top
    rect(ctx, x - 5, y - 18, 10, 5, pk);
    rect(ctx, x - 4, y - 18, 8, 2, pd);
    rect(ctx, x - 4, y - 18, 8, 1, pm);
    for kx in [-3, -1, 1] {
        rect(ctx, x + kx, y - 15, 1, 1, phi);
    }
    let on = linked && (!over || (t * 6.).floor() as i64 % 2 != 0);
    rect(ctx, x + 3, y - 15, 1, 1, if on { SP::g } else { Color::rgb(0x2a2f33) });
    let Some((hx, hy, side)) = hand else {
        // the handset at rest: a bone across the cradle, its two ends bulging
        rect(ctx, x - 4, y - 20, 8, 2, pk);
        rect(ctx, x - 3, y - 20, 6, 1, pd);
        for ex in [x - 7, x + 4] {
            rect(ctx, ex, y - 21, 3, 4, pk);
            rect(ctx, ex + 1, y - 20, 1, 2, pm);
        }
        return;
    };
    rect(ctx, x - 5, y - 20, 1, 2, pk); // the cradle, empty
    rect(ctx, x + 4, y - 20, 1, 2, pk);
    // the cord: from the base's side down in a sag to your hand, coiled (every other dot a dot higher)
    let (x0, y0) = (x + side * 5, y - 15);
    let n = js_round(((hx - x0) as f64).hypot((hy - y0) as f64)).max(2);
    for k in 0..=n {
        let u = k as f64 / n as f64;
        let cx = js_round(x0 as f64 + (hx - x0) as f64 * u);
        let cy = js_round(y0 as f64 + (hy - y0) as f64 * u + (u * std::f64::consts::PI).sin() * 3.0);
        rect(ctx, cx, cy - k % 2, 1, 1, pk);
    }
}

/// a desk out of the closet, and standing where it was let go while its folder is picked: the cabinet with a blank
/// plate, its screen dark, in its machine's paint `p` (draw_desk draws it once it is a desk); x0, y as a desk's
pub fn draw_blank_desk(ctx: &mut Ctx, x0: i32, y: i32, p: &Paint) {
    let (cab, sx, sy) = (26, x0 + 2, y - 32);
    slab(ctx, x0, y - 15, cab, 4, 11, p);
    plate(ctx, x0 + 2, y - 9, cab - 4, false, p);
    rect(ctx, sx + 2, sy + 2, 18, 11, css("#10141f"));
    put_named(ctx, "monitor", sx, sy);
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
/// the filing cabinet: steel, three drawers, a card and a handle on each; while FILES is on, its top drawer is
/// pulled out, coloured folders' tabs standing up in it
pub fn draw_filing(ctx: &mut Ctx, x: i32, y: i32, on: bool) {
    let (x0, top) = (x - 7, y - 24);
    rect(ctx, x0 - 1, y, 16, 1, SHADOW);
    rect(ctx, x0, top, 14, 24, SP::F);
    rect(ctx, x0 + 1, top + 1, 12, 22, SP::C);
    rect(ctx, x0 + 1, top + 1, 12, 1, SP::B);
    rect(ctx, x0 + 1, top + 1, 1, 22, SP::B);
    rect(ctx, x0 + 12, top + 2, 1, 21, SP::D);
    let drawer = |ctx: &mut Ctx, dy: i32, dx: i32, w: i32| {
        rect(ctx, x0 + 2 + dx, dy, w, 6, SP::B);
        rect(ctx, x0 + 2 + dx, dy, w, 1, SP::A);
        rect(ctx, x0 + 2 + dx, dy + 5, w, 1, SP::D);
        let c = x0 + 2 + dx + w / 2;
        rect(ctx, c - 2, dy + 1, 4, 2, css("#f4ecd8")); // its card
        rect(ctx, c - 1, dy + 2, 2, 1, SP::D);
        rect(ctx, c - 2, dy + 4, 4, 1, SP::F); // its handle
    };
    drawer(ctx, top + 9, 0, 10);
    drawer(ctx, top + 16, 0, 10);
    if !on {
        drawer(ctx, top + 2, 0, 10);
        return;
    }
    rect(ctx, x0 + 1, top + 1, 12, 8, SP::x); // the open drawer: dark inside, the folders standing in it
    for (tx, c) in [(x0 + 2, SP::y), (x0 + 6, SP::g), (x0 + 10, SP::v)] {
        rect(ctx, tx, top + 3, 2, 5, c);
        rect(ctx, tx, top + 3, 2, 1, SP::A);
    }
    rect(ctx, x0, top + 15, 14, 1, SHADOW); // its front pulled out toward you, a little wider, its shadow under it
    rect(ctx, x0 - 1, top + 8, 16, 8, SP::F);
    drawer(ctx, top + 9, -2, 14);
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

/// the pumpkin: carved, a candle in it. Lit (Halloween), its eyes, nose and grin glow and flicker by the room's time
/// `t`; blown out, they are dark
pub fn draw_pumpkin(ctx: &mut Ctx, x: i32, y: i32, on: bool, t: f64) {
    let (hi, top, d) = (Color::rgb(0xf6a03c), Color::rgb(0xe07424), Color::rgb(0xa8481c));
    let rows = [(-3, 3), (-5, 5), (-6, 6), (-6, 6), (-6, 6), (-6, 6), (-6, 6), (-5, 5), (-4, 4)]; // its rows' ends, top down
    rect(ctx, x - 6, y, 13, 1, SHADOW);
    for (j, &(a, b)) in rows.iter().enumerate() {
        rect(ctx, x + a - 1, y - 9 + j as i32, b - a + 3, 1, INK);
    }
    rect(ctx, x - 3, y - 10, 7, 1, INK);
    rect(ctx, x - 2, y - 1, 5, 1, INK);
    for (j, &(a, b)) in rows.iter().enumerate() {
        let ry = y - 9 + j as i32;
        rect(ctx, x + a, ry, b - a + 1, 1, top);
        for rx in [-3, 3] {
            if rx > a && rx < b {
                rect(ctx, x + rx, ry, 1, 1, d); // its ribs
            }
        }
        rect(ctx, x + b, ry, 1, 1, d);
    }
    rect(ctx, x - 4, y - 8, 2, 1, hi);
    rect(ctx, x - 5, y - 7, 1, 2, hi);
    rect(ctx, x - 1, y - 12, 2, 3, INK); // the stem
    rect(ctx, x, y - 12, 1, 3, WOOD.d);
    rect(ctx, x + 1, y - 12, 1, 1, INK);
    let k = (t * 9. + (t * 23.).sin()).floor() as i64 % 3;
    let glow = if !on {
        C::INK2
    } else if k != 0 {
        SP::Y
    } else {
        C::GOLD1
    };
    let deep = if !on { INK } else { C::GOLD1 };
    for ex in [-4, 2] {
        rect(ctx, ex + x, y - 6, 3, 1, glow); // the eyes, triangles
        rect(ctx, ex + x + 1, y - 7, 1, 1, glow);
    }
    rect(ctx, x, y - 5, 1, 1, deep); // the nose
    rect(ctx, x - 4, y - 3, 9, 1, glow); // the grin, two teeth
    rect(ctx, x - 3, y - 2, 7, 1, deep);
    rect(ctx, x - 2, y - 3, 1, 1, top);
    rect(ctx, x + 2, y - 2, 1, 1, top);
}

/// the operator's console's rain: a drop's bright head, then its tail fading
const CONSOLE_RAIN: [Color; 4] = [Color::rgb(0xd6ffd0), Color::rgb(0x60c677), Color::rgb(0x2f8a46), Color::rgb(0x164a26)];
/// the operator's console (the matrix): a terminal on a metal stand, its keyboard in front, the code raining down its
/// screen, a drop a column, each at its own speed, a bright head and a fading tail; `t` the room's time
pub fn draw_console(ctx: &mut Ctx, x: i32, y: i32, t: f64) {
    rect(ctx, x - 11, y, 24, 1, SHADOW);
    rect(ctx, x - 10, y - 11, 22, 11, SP::F); // the stand: its top, its front, a slot, the power light
    rect(ctx, x - 9, y - 10, 20, 3, SP::C);
    rect(ctx, x - 9, y - 10, 20, 1, SP::B);
    rect(ctx, x - 9, y - 7, 20, 6, SP::E);
    rect(ctx, x - 9, y - 7, 20, 1, SP::D);
    rect(ctx, x - 7, y - 4, 8, 1, SP::F);
    rect(ctx, x + 8, y - 4, 1, 1, SP::g);
    rect(ctx, x - 3, y - 12, 8, 2, SP::F); // the terminal's neck
    rect(ctx, x - 9, y - 25, 20, 13, SP::F); // its case, lit from the top left
    rect(ctx, x - 8, y - 24, 18, 11, SP::D);
    rect(ctx, x - 8, y - 24, 18, 1, SP::C);
    rect(ctx, x - 8, y - 24, 1, 11, SP::C);
    rect(ctx, x + 9, y - 23, 1, 10, SP::E);
    rect(ctx, x - 7, y - 14, 17, 1, SP::E);
    rect(ctx, x - 6, y - 22, 14, 8, Color::rgb(0x071208)); // the screen
    for i in 0..14i64 {
        let head = (t * (4 + (i * 7) % 5) as f64 + ((i * 37) % 17) as f64).floor() as i64 % 13 - 2;
        for (k, &c) in CONSOLE_RAIN.iter().enumerate() {
            let r = head - k as i64;
            if (0..8).contains(&r) {
                rect(ctx, x - 6 + i as i32, y - 22 + r as i32, 1, 1, c);
            }
        }
    }
    rect(ctx, x - 6, y - 9, 13, 2, SP::F); // the keyboard
    for i in 0..6 {
        rect(ctx, x - 5 + i * 2, y - 9, 1, 1, SP::D);
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
            let Some(&p) = b.get((r * 8 + f) as usize) else {
                continue;
            };
            if p == b'.' {
                continue;
            }
            let white = p.is_ascii_uppercase();
            let tall = if b"KQkq".contains(&p) {
                3
            } else if b"Pp".contains(&p) {
                1
            } else {
                2
            };
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
const POUF: &[(char, u32)] =
    &[('k', 0x173115), ('h', 0x8cbf8c), ('t', 0x6fa374), ('s', 0x487841), ('d', 0x355f31), ('w', 0x885c47), ('x', 0x532e3d)];
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
    let Some(im) = sprites::get(if i != 0 { "PLANT_2" } else { "PLANT" }) else {
        return;
    };
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
        let y = gy as f64
            - 12.
            - js_round((k * std::f64::consts::PI).sin() * if t < 0.5 { 6. } else { 8. }) as f64
            - 6. * (1. - (t * 4.).min(1.));
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
