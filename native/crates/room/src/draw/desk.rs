//! A project's desk: the cabinet with the project's name and screen, the long desktop where its bots
//! sit, a laptop on the cabinet for each worktree; and the chairs.

use super::*;
use crate::figure::halo;
use crate::font::{tiny, tiny_w};

/// what one screen shows (the project's own, or a worktree's laptop), as the caller found it: whether a
/// bot sits there with its process running, whether one of them is busy, waits for you; your terminal
/// there (one per screen, kept by the server) and whether output is coming
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Screen {
    pub seated: bool,
    pub busy: bool,
    pub waits: bool,
    pub shell: bool,
    pub shell_busy: bool,
}

/// a worktree's laptop on the cabinet
#[derive(Clone, Debug, Default)]
pub struct Tree<'a> {
    pub dir: &'a str,
    pub screen: Screen,
    /// the pointer is on it
    pub on: bool,
    /// lit: its shell's tile is under the pointer, or one of its bots is lit
    pub lit: bool,
}

/// a desk as the layout gives it, and what the caller knows of its state
#[derive(Clone, Debug, Default)]
pub struct Desk<'a> {
    pub x0: i32,
    pub y: i32,
    pub w: i32,
    pub cab: i32,
    pub label: &'a str,
    pub dir: &'a str,
    pub color: Color,
    pub paint: Paint,
    /// each chair's x, in the bots' order
    pub seats: Vec<i32>,
    /// a coffee a colleague brought, by the keyboard of the seat at x, in the bringer's colour
    pub mugs: Vec<(i32, Color)>,
    pub screen: Screen,
    pub trees: Vec<Tree<'a>>,
    /// the pointer on the name plate, the screen, the + beside it
    pub on_plate: bool,
    pub on_screen: bool,
    pub on_plus: bool,
    /// the screen lit (drawDesk's `lit`), or its shell's tile under the pointer (`hsAt(d.dir)`)
    pub lit: bool,
    pub shell_lit: bool,
}

impl Default for Paint {
    fn default() -> Paint {
        WOOD
    }
}

/// a JS string's length (in UTF-16 units): what seeds a screen's rows
fn js_len(s: &str) -> i32 {
    s.encode_utf16().count() as i32
}

/// the project's screen on its cabinet
pub fn screen_at(x0: i32, cab: i32, trees: usize, y: i32) -> (i32, i32) {
    (js_round(x0 as f64 + (cab - (22 + trees as i32 * LAP)) as f64 / 2.), y - 32)
}

/// where a click on the desk does something, x0 y0 x1 y1 (room.js's `hot` calls in drawDesk)
#[derive(Clone, Debug, PartialEq)]
pub struct DeskHots {
    pub plate: [i32; 4],
    pub screen: [i32; 4],
    pub plus: [i32; 4],
    pub laptops: Vec<[i32; 4]>,
}
pub fn desk_hots(x0: i32, y: i32, cab: i32, trees: usize) -> DeskHots {
    let (sx, sy) = screen_at(x0, cab, trees, y);
    let gw = 22 + trees as i32 * LAP;
    let px = sx + gw + 2;
    DeskHots {
        plate: [x0 + 2, y - 10, x0 + cab - 2, y - 2],
        screen: [sx, sy, sx + 22, sy + 18],
        plus: [px - 3, sy - 3, px + 8, sy + 8],
        laptops: (0..trees as i32).map(|k| {
            let (px, py) = (sx + 23 + k * LAP, y - 22);
            [px, py, px + 12, py + 11]
        }).collect(),
    }
}

/// the path as a shell prompt shortens it (fish, starship): every directory but the last to its first
/// letter (two for a dotted one), the home to ~
pub fn abbrev(p: &str) -> String {
    let s: Vec<&str> = p.split('/').collect();
    s.iter()
        .enumerate()
        .map(|(i, x)| {
            if i < s.len() - 1 && x.chars().count() > 1 && *x != "~" {
                x.chars().take(if x.starts_with('.') { 2 } else { 1 }).collect()
            } else {
                x.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn draw_desk(ctx: &mut Ctx, d: &Desk, t: f64) {
    let (x0, y, dw, cab) = (d.x0, d.y, d.w, d.cab);
    let p = &d.paint;
    // a screen runs only while a bot sits at it typing; dim while one sits there idle; a blinking
    // cursor while one waits for you. The big one is the project's own directory, a laptop a worktree's;
    // your terminal there: its prompt on the screen's last row, the cursor blinking, a grey line running
    // while output comes
    let paint = |ctx: &mut Ctx, gx: i32, gy: i32, gw: i32, gh: i32, s: &Screen, seed: i32| {
        let py = gy + gh - 2;
        rect(ctx, gx, gy, gw, gh, if s.seated || s.shell { C::BLUE0 } else { Color::rgb(0x10141f) });
        if !s.seated && !s.shell {
            rect(ctx, gx + gw - 3, gy + 1, 1, 1, Color::rgb(0x232c40)); // off: a glint
            rect(ctx, gx + gw - 4, gy + 2, 1, 1, Color::rgb(0x232c40));
            return;
        }
        let col = if s.busy { d.color } else { d.color.shade(0.45) };
        let sc = if s.busy { (t * 5.).floor() as i32 } else { 0 };
        if s.seated {
            for r in 0..gh / 2 - if s.shell { 1 } else { 0 } {
                let w = (gw - 2).min(3 + ((r * 7 + sc + seed) * 5) % if gw > 10 { 11 } else { 4 });
                rect(ctx, gx + 1 + (r * 3) % 4, gy + 1 + r * 2, w, 1, col);
            }
        }
        if s.waits && !s.busy && (t * 2.).floor() as i64 % 2 != 0 {
            rect(ctx, gx + 2, py - if s.shell { 2 } else { 0 }, 2, 1, C::STEEL4);
        }
        if !s.shell {
            return;
        }
        rect(ctx, gx + 1, py, 2, 1, C::STEEL3);
        if s.shell_busy {
            rect(ctx, gx + 4, py, (gw - 6).min(2 + (((t * 6.).floor() as i32 * 7) % if gw > 10 { 9 } else { 3 })), 1, C::STEEL2);
        } else if (t * 2.).floor() as i64 % 2 != 0 {
            rect(ctx, gx + 4, py, 1, 1, C::STEEL4);
        }
    };
    // the desktop, lower than the cabinet: a leg between each two chairs, a keyboard at each seat; none until the
    // first bot comes
    if !d.seats.is_empty() {
        slab(ctx, x0 + cab - 2, y - 12, dw - cab + 2, 5, 4, p);
        let legs = std::iter::once(x0 + cab + 1).chain(d.seats.iter().map(|x| x + 11)).chain(std::iter::once(x0 + dw - 4));
        for x in legs {
            rect(ctx, x, y - 4, 2, 4, p.k);
            rect(ctx, x, y - 4, 1, 3, p.d);
            rect(ctx, x, y, 2, 1, SHADOW);
        }
        for &x in &d.seats {
            rect(ctx, x - 9, y - 11, 7, 2, SP::F);
            rect(ctx, x - 8, y - 11, 5, 1, SP::B);
        }
    }
    for &(x, col) in &d.mugs {
        // a coffee a colleague brought, by the keyboard, for a while
        rect(ctx, x - 13, y - 14, 3, 3, col);
        rect(ctx, x - 11, y - 13, 1, 1, INK);
    }
    // the cabinet, the project's name on a plate (under the pointer only its letters light up)
    slab(ctx, x0, y - 15, cab, 4, 11, p);
    let lx = js_round(x0 as f64 + (cab - tiny_w(d.label)) as f64 / 2.);
    plate(ctx, x0 + 2, y - 9, cab - 4, false, p);
    tiny(ctx, d.label, lx as f64, (y - 8) as f64, if d.on_plate { GOLD } else { C::STEEL3 });
    // on it the project's screen, and a laptop for each worktree, plugged in. A + beside it starts a bot here
    let gw = 22 + d.trees.len() as i32 * LAP;
    let (sx, sy) = screen_at(x0, cab, d.trees.len(), y);
    {
        let px = sx + gw + 2;
        let c = if d.on_plus { d.color } else { C::STEEL2 };
        rect(ctx, px + 2, sy, 1, 5, c); // a 5 x 5 plus
        rect(ctx, px, sy + 2, 5, 1, c);
    }
    paint(ctx, sx + 2, sy + 2, 18, 11, &d.screen, js_len(d.dir));
    if let Some(mon) = sprites::get("monitor") {
        if d.on_screen || d.lit || d.shell_lit {
            put(ctx, &halo(&mon, if d.on_screen { C::STEEL3 } else { GOLD }), sx, sy, false);
        }
        put(ctx, &mon, sx, sy, false);
    }
    if !d.trees.is_empty() {
        rect(ctx, sx + 17, y - 13, gw - 22, 1, SP::F); // the cable, along the top
    }
    let lap = sprites::get("laptop");
    for (k, tree) in d.trees.iter().enumerate() {
        let (px, py) = (sx + 23 + k as i32 * LAP, y - 22);
        paint(ctx, px + 2, py + 2, 8, 5, &tree.screen, js_len(tree.dir));
        if let Some(lap) = &lap {
            if tree.on || tree.lit {
                put(ctx, &halo(lap, if tree.on { C::STEEL3 } else { GOLD }), px, py, false);
            }
            put(ctx, lap, px, py, false);
        }
    }
}

/// an office chair at a project's desk, its sitter facing the screen; none where no bot is yet
pub fn draw_chair(ctx: &mut Ctx, x: i32, y: i32, lit: bool) {
    let Some(im) = sprites::get("chair") else { return };
    if lit {
        put(ctx, &halo(&im, GOLD), x - 4, y - 16, false);
    }
    put(ctx, &im, x - 4, y - 16, false);
}
