// The room: one machine, ported from pair's office (app/floor.js): its furniture, its breaks, its mats.
// Every project (a working directory) is one desk: a cabinet with the project's screen and name at its
// left end, a long desktop to the right where its bots sit facing the screen, and an empty chair at the
// end (+) that starts another bot there.
// Where a bot is says what it is doing:
//   busy                at its project's desk, typing
//   idle                on a break: coffee, the printer, the cooler, ping-pong, the plants, the window,
//                       the armchair, a meeting at the whiteboard, or just standing about
//   idle 10 minutes     charging on a mat
//   sends a message     walks over to whoever it wrote to and stands there talking for a moment
// Nothing else signals anything; what they say is never shown, only that they talk.
//
// The machines are lettered on the wall, the current one underlined; the wall is also where the window
// is dragged, as it runs behind the title bar.

use crate::host::Bot;
use crate::sprite::*;
use egui::{Color32, Mesh, Pos2, Rect, Vec2};
use std::collections::HashMap;

pub const WALL: f32 = 30.0;
const TALK: f32 = 4.0; // seconds standing next to the other once there
const SEAT: f32 = 18.0; // between two chairs at a desk
const ADD_W: f32 = 30.0; // the empty cabinet that adds a project
const WOOD: Color32 = hex(0x7a4a35);
const WOOD_D: Color32 = hex(0x4a2e2a);
const WALLC: Color32 = hex(0x1e1a2c);
const FLOOR: Color32 = hex(0x130f1b);
const PLANK: Color32 = hex(0x19141f);

pub enum Action {
    Open(String),
    Spawn(String),
    AddDir,
    Forget(String),
    Room(usize),
    AddRoom,
}

#[derive(Clone, PartialEq)]
enum Act {
    Desk,
    Stand { left: f32 },
    Loiter { x: f32, y: f32, left: f32 },
    Coffee { stage: u8, wait: f32 },
    Print { stage: u8, wait: f32 },
    Cooler { side: usize, left: f32, with: String },
    Pong { side: usize, left: f32, with: String, there: bool },
    Visit { to: String, left: f32, wave: f32, msg: bool },
    Plant { i: usize, wait: f32 },
    Window { left: f32 },
    Chair { left: f32 },
    Present { wait: f32 },
    Meeting { seat: usize },
    Sleep { there: bool },
}

struct Fig {
    x: f32,
    y: f32,
    dir: f32,
    arms: Arms,
    legs: Legs,
    step: f32,
    act: Act,
    carry: Option<Arms>,
    lying: bool,
    busy: bool,
    idle_since: f64,
    chat: f32, // talking: dots in a bubble
    z: f32,
}

struct Desk {
    cwd: String,
    x0: f32,
    w: f32,
    cab: f32, // the cabinet with the screen, at the left end
    y: f32,   // where it stands on the floor
    color: Color32,
    seats: Vec<(String, f32)>,
    plus: f32,
}

struct Layout {
    desks: Vec<Desk>,
    add: Rect,
    floor: Rect,
    floor_y: f32,
    board: Option<Rect>,
    window: Option<Rect>,
    meet: Option<Pos2>,
    coffee: Pos2, // where you stand to get one
    cooler: Option<Pos2>,
    printer: Option<Pos2>,
    chair: Option<Pos2>,
    plants: Vec<Pos2>,
    pong: Option<Pos2>,
    mats: Vec<Pos2>,
}

impl Layout {
    fn cooler_spot(&self, side: usize) -> Option<Pos2> {
        self.cooler.map(|c| Pos2::new(c.x + if side == 0 { -8.0 } else { 10.0 }, c.y + 1.0))
    }
    fn pong_spot(&self, side: usize) -> Option<Pos2> {
        self.pong.map(|g| Pos2::new(g.x + if side == 0 { -22.0 } else { 22.0 }, g.y + 1.0))
    }
    fn meet_seat(&self, i: usize) -> Option<(Pos2, f32)> {
        let m = self.meet?;
        let s = [(-23.0, 0.0, 1.0), (23.0, 0.0, -1.0), (-8.0, -5.0, 1.0), (8.0, -5.0, -1.0)];
        s.get(i).map(|(dx, dy, d)| (Pos2::new(m.x + dx, m.y + dy), *d))
    }
    /// a place to stand about: not in a desk, a mat or the ping-pong table
    fn free(&self, x: f32, y: f32) -> bool {
        let inside = |x0: f32, y0: f32, x1: f32, y1: f32| x >= x0 && x <= x1 && y >= y0 && y <= y1;
        !self.desks.iter().any(|d| inside(d.x0 - 8.0, d.y - 30.0, d.x0 + d.w + 10.0, d.y + 6.0))
            && !inside(self.add.left() - 8.0, self.add.top() - 14.0, self.add.right() + 8.0, self.add.bottom() + 6.0)
            && !self.mats.iter().any(|m| inside(m.x - 14.0, m.y - 26.0, m.x + 14.0, m.y + 4.0))
            && !self.pong.is_some_and(|g| inside(g.x - 36.0, g.y - 20.0, g.x + 36.0, g.y + 8.0))
            && !self.meet.is_some_and(|m| inside(m.x - 28.0, m.y - 14.0, m.x + 28.0, m.y + 4.0))
    }
    fn seat(&self, id: &str) -> Option<Pos2> {
        self.desks.iter().find_map(|d| d.seats.iter().find(|(i, _)| i == id).map(|(_, x)| Pos2::new(*x, d.y)))
    }
}

pub struct Room {
    figs: HashMap<String, Fig>,
    sprites: Sprites,
    seed: u32,
    meeting: Option<(String, f32)>, // who presents, how long still
    last_meeting: f32,
    board: Vec<(u8, f32, f32, f32, f32)>, // what is on the whiteboard: kind, x, y, w, h
    board_by: Color32,
    pong: (f32, usize), // the ball: how far across, from which side
    t: f32,
    born: f64,
    sleep_after: f64, // idle this long: off to a mat
}

pub fn project_color(cwd: &str) -> Color32 {
    PROJECT[(fnv(cwd) % PROJECT.len() as u32) as usize]
}

pub fn project_label(cwd: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if cwd == home || cwd == "~" || cwd == "/root" || (cwd.starts_with("/home/") && cwd.matches('/').count() == 2) || (cwd.starts_with("/Users/") && cwd.matches('/').count() == 2) {
        return "home".into();
    }
    cwd.trim_end_matches('/').rsplit('/').next().unwrap_or(cwd).to_string()
}

/// dots on the mesh, `k` screen points per dot from `o`
struct Paint {
    m: Mesh,
    o: Pos2,
    k: f32,
}

impl Paint {
    fn r(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color32) {
        if w > 0.0 && h > 0.0 {
            let k = self.k;
            self.m.add_colored_rect(Rect::from_min_size(self.o + Vec2::new(x.round() * k, y.round() * k), Vec2::new(w.round() * k, h.round() * k)), c);
        }
    }
    /// a box with an ink outline
    fn b(&mut self, x: f32, y: f32, w: f32, h: f32, fill: Color32) {
        self.e(x, y, w, h, fill, INK);
    }
    fn e(&mut self, x: f32, y: f32, w: f32, h: f32, fill: Color32, edge: Color32) {
        self.r(x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
        self.r(x, y, w, h, fill);
    }
    /// pair's small lettering, 3x5, upper case
    fn tiny(&mut self, s: &str, x: f32, y: f32, c: Color32) {
        let mut dots = Vec::new();
        tiny(s, x.round() as i32, y.round() as i32, &mut dots);
        for (dx, dy) in dots {
            self.r(dx as f32, dy as f32, 1.0, 1.0, c);
        }
    }
    fn sprite(&mut self, s: &Sprite, x0: f32, y0: f32, flip: bool, rows: i8, tint: Option<Color32>) {
        for &(sx, sy, c) in s.iter() {
            if sy >= rows {
                continue;
            }
            let sx = if flip { SW - 1 - sx as i32 } else { sx as i32 };
            let c = tint.map(|t| mix(c, t, 0.55)).unwrap_or(c);
            self.r(x0 + sx as f32, y0 + sy as f32, 1.0, 1.0, c);
        }
    }
}

impl Room {
    pub fn new() -> Self {
        Room {
            figs: HashMap::new(),
            sprites: Sprites::default(),
            seed: 0x9e3779b9,
            meeting: None,
            last_meeting: -60.0,
            board: Vec::new(),
            board_by: BLUE2,
            pong: (0.0, 0),
            t: 0.0,
            born: -1.0,
            // (the made-up bots fall asleep sooner, to see it)
            sleep_after: if std::env::var_os("BURROW_DEMO").is_some() { 40.0 } else { 600.0 },
        }
    }

    /// `from` sent `to` a message: it walks over
    pub fn talk(&mut self, from: &str, to: &str) {
        if !self.figs.contains_key(to) {
            return;
        }
        // already together: talk a bit longer
        for (a, b) in [(from, to), (to, from)] {
            if let Some(f) = self.figs.get_mut(a) {
                if let Act::Visit { to: t, left, .. } = &mut f.act {
                    if t == b {
                        *left = left.max(TALK);
                        return;
                    }
                }
            }
        }
        if let Some(f) = self.figs.get_mut(from) {
            f.lying = false;
            f.carry = None;
            f.act = Act::Visit { to: to.to_string(), left: TALK, wave: 0.0, msg: true };
        }
    }

    fn rnd(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x % 100_000) as f32 / 100_000.0
    }

    /// `wall_left`: where the wall is free, after the machines' names
    fn layout(&self, dirs: &[String], bots: &[Bot], w: f32, h: f32, asleep: usize, wall_left: f32) -> Layout {
        let mut order: Vec<&str> = dirs.iter().map(|d| d.as_str()).collect();
        for b in bots {
            if !order.contains(&b.cwd.as_str()) {
                order.push(&b.cwd);
            }
        }
        let floor_y = h - 6.0;
        let window = (w >= 200.0).then(|| Rect::from_min_max(Pos2::new(w - 46.0, 6.0), Pos2::new(w - 28.0, 21.0)));
        // the whiteboard in the middle of the wall, or as far right as the names make it, if it fits
        let bx = (w / 2.0 - 14.0).max(wall_left + 6.0).round();
        let right = window.map(|r| r.left()).unwrap_or(w - 18.0) - 6.0;
        let board = (bx + 30.0 <= right).then(|| Rect::from_min_max(Pos2::new(bx, 8.0), Pos2::new(bx + 28.0, 20.0)));

        // the desks in columns, each as wide as its widest desk, cabinets lined up: as many columns as fit
        let sizes: Vec<(String, f32, f32)> = order
            .iter()
            .map(|cwd| {
                let n = bots.iter().filter(|b| b.cwd == *cwd).count();
                let cab = (tiny_width(&project_label(cwd)) as f32 + 6.0).max(22.0);
                (cwd.to_string(), cab + 4.0 + (n + 1) as f32 * SEAT, cab)
            })
            .collect();
        let mut widths: Vec<f32> = sizes.iter().map(|s| s.1).chain(std::iter::once(ADD_W)).collect();
        let gap_x = 14.0;
        let columns = |ws: &[f32], c: usize| -> Vec<f32> { (0..c).map(|j| ws.iter().skip(j).step_by(c).fold(0.0, |a: f32, b| a.max(*b))).collect() };
        let total = |cw: &[f32]| cw.iter().sum::<f32>() + gap_x * (cw.len() as f32 - 1.0);
        let mut c = (1..=widths.len()).rev().find(|&c| total(&columns(&widths, c)) <= w - 16.0).unwrap_or(1);
        // the empty cabinet doesn't get a row of its own: then it stands by the wall, in the bottom row
        let add_below = widths.len() > 1 && (widths.len() - 1) % c == 0 && w >= 110.0;
        if add_below {
            widths.pop();
            c = (1..=widths.len()).rev().find(|&c| total(&columns(&widths, c)) <= w - 16.0).unwrap_or(1);
        }
        let cw = columns(&widths, c);
        let left = ((w - total(&cw)) / 2.0).max(6.0).round();
        let col_x: Vec<f32> = cw.iter().scan(left, |x, cwj| {
            let here = *x;
            *x += cwj + gap_x;
            Some(here)
        }).collect();
        let rows = widths.len().div_ceil(c);

        // top to bottom, as in pair: the meeting table under the whiteboard, the rows of desks, the
        // ping-pong table and the mats, spread evenly over what the wall and the bottom row leave;
        // what doesn't fit is left out (the mats last, the meeting table first)
        const ROW: f32 = 46.0;
        let wide = w >= 200.0;
        let avail = floor_y - 28.0 - (WALL + 4.0);
        let mut used = rows as f32 * ROW;
        let mats_h = 30.0;
        let pong_h = 34.0;
        let fits_mats = used + mats_h <= avail;
        if fits_mats {
            used += mats_h;
        }
        let fits_pong = if wide { fits_mats } else { used + pong_h <= avail };
        if fits_pong && !wide {
            used += pong_h;
        }
        let fits_meet = board.is_some() && used + 40.0 <= avail;
        if fits_meet {
            used += 40.0;
        }
        let parts = rows + fits_mats as usize + (fits_pong && !wide) as usize + fits_meet as usize;
        let pad = ((avail - used) / (parts + 1) as f32).max(0.0);
        let mut y0 = WALL + 4.0 + pad;
        let mut meet = None;
        if fits_meet {
            meet = Some(Pos2::new(board.unwrap().center().x.round(), (y0 + 30.0).round()));
            y0 += 40.0 + pad;
        }
        let mut desks = Vec::new();
        let mut add = Rect::NOTHING;
        for (i, wi) in widths.iter().enumerate() {
            let (r, j) = (i / c, i % c);
            let x = col_x[j];
            let y = (y0 + r as f32 * (ROW + pad) + 38.0).round();
            if i == sizes.len() {
                add = Rect::from_min_max(Pos2::new(x, y - 13.0), Pos2::new(x + wi, y));
                continue;
            }
            let (cwd, dw, cab) = &sizes[i];
            let mine: Vec<&Bot> = bots.iter().filter(|b| &b.cwd == cwd).collect();
            let seat = |k: usize| x + cab + 4.0 + SEAT / 2.0 + k as f32 * SEAT;
            desks.push(Desk {
                cwd: cwd.clone(),
                x0: x,
                w: *dw,
                cab: *cab,
                y,
                color: project_color(cwd),
                seats: mine.iter().enumerate().map(|(k, b)| (b.id.clone(), seat(k))).collect(),
                plus: seat(mine.len()),
            });
        }
        y0 += rows as f32 * (ROW + pad);
        if add_below {
            let x = ((48.0 + (w / 2.0 - 6.0)) / 2.0 - ADD_W / 2.0).round();
            add = Rect::from_min_max(Pos2::new(x, floor_y - 13.0), Pos2::new(x + ADD_W, floor_y));
        }
        let n = asleep.max(1).min(((w - 12.0) / 28.0).max(1.0) as usize);
        let mut pong = None;
        let mut mats = Vec::new();
        let mat_row = |cx: f32, y: f32, n: usize| (0..n).map(|i| Pos2::new((cx + (i as f32 - (n - 1) as f32 / 2.0) * 28.0).round(), y.round())).collect::<Vec<_>>();
        if wide && fits_mats {
            // side by side: the table on the left, the mats on the right
            pong = Some(Pos2::new((w * 0.3).round(), (y0 + 26.0).round()));
            mats = mat_row(w * 0.68, y0 + 28.0, n.min(((w * 0.45) / 28.0) as usize).max(1));
        } else {
            if fits_pong {
                pong = Some(Pos2::new((w / 2.0).round(), (y0 + 26.0).round()));
                y0 += pong_h + pad;
            }
            if fits_mats {
                mats = mat_row(w / 2.0, y0 + 28.0, n);
            }
        }

        // along the bottom, as in pair: the coffee corner, the cooler, the printer, an armchair, a plant
        let cooler = (w >= 76.0).then(|| Pos2::new(36.0, floor_y - 1.0));
        let coffee = if cooler.is_some() { Pos2::new(30.0, floor_y - 10.0) } else { Pos2::new(33.0, floor_y) };
        let printer = (w >= 110.0).then(|| Pos2::new((w / 2.0 + 4.0).round(), floor_y));
        let chair = (w >= 140.0).then(|| Pos2::new((w - 34.0).round(), floor_y));
        let mut plants = vec![Pos2::new(w - 8.0, floor_y)];
        if h >= 300.0 && left >= 24.0 {
            plants.push(Pos2::new(8.0, (h * 0.55).round()));
        }
        Layout {
            desks,
            add,
            floor: Rect::from_min_max(Pos2::new(14.0, WALL + 8.0), Pos2::new(w - 14.0, floor_y - 4.0)),
            floor_y,
            board,
            window,
            meet,
            coffee,
            cooler,
            printer,
            chair,
            plants,
            pong,
            mats,
        }
    }

    /// advance and draw; `open` is the bot whose session is showing, `rooms` the machines (`cur` is this
    /// one), `inset` the width of the title bar's buttons over the wall, in points
    #[allow(clippy::too_many_arguments)]
    pub fn ui(&mut self, ui: &mut egui::Ui, bots: &[Bot], dirs: &[String], open: Option<&str>, rooms: &[String], cur: usize, inset: f32, note: Option<&str>) -> Option<Action> {
        let rect = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        // dots big enough that the room is 220 to 330 of them wide, as dense as pair's office
        let k: f32 = (rect.width() / 280.0).round().clamp(2.0, 4.0);
        let (w, h) = ((rect.width() / k).floor(), (rect.height() / k).floor());
        let ppp = ui.ctx().pixels_per_point();
        let o = Pos2::new((rect.min.x * ppp).round() / ppp, (rect.min.y * ppp).round() / ppp);
        let (now, dt) = ui.input(|i| (i.time, i.stable_dt.min(0.1)));
        if self.born < 0.0 {
            self.born = now;
        }
        self.t += dt;
        let t = self.t;

        // keep a figure per bot; one that turns up later comes in through the door
        self.figs.retain(|id, _| bots.iter().any(|b| &b.id == id));
        let asleep_n = self.figs.values().filter(|f| matches!(f.act, Act::Sleep { .. })).count();
        let tabs_x = (inset / k).ceil() + 2.0;
        let wall_left = tabs_x + rooms.iter().map(|r| tiny_width(r) as f32 + 7.0).sum::<f32>() + 5.0;
        let lay = self.layout(dirs, bots, w, h, asleep_n, wall_left);
        let late = now - self.born > 3.0;
        for b in bots {
            if !self.figs.contains_key(&b.id) {
                let at = if late { Pos2::new(12.0, WALL + 3.0) } else { lay.seat(&b.id).unwrap_or(Pos2::new(w / 2.0, h / 2.0)) };
                let z = self.rnd() * 2.0;
                self.figs.insert(
                    b.id.clone(),
                    Fig {
                        x: at.x,
                        y: at.y,
                        dir: 1.0,
                        arms: Arms::Down,
                        legs: if late { Legs::Stand } else { Legs::Sit },
                        step: 0.0,
                        // idle ones get up a moment after burrow opens
                        act: if b.busy { Act::Desk } else { rest(z) },
                        carry: None,
                        lying: false,
                        busy: b.busy,
                        idle_since: now,
                        chat: 0.0,
                        z,
                    },
                );
            }
        }

        self.think(bots, &lay, now, dt);

        // ---- paint
        let hover = resp.hover_pos().map(|p| ((p.x - o.x) / k, (p.y - o.y) / k));
        let over = |x0: f32, y0: f32, x1: f32, y1: f32| hover.is_some_and(|(hx, hy)| hx >= x0 && hx < x1 && hy >= y0 && hy < y1);
        let mut clicked: Option<Action> = None;
        let mut pointing = false;
        let mut p = Paint { m: Mesh::default(), o, k };

        // the floor, the wall, its skirting
        p.r(0.0, 0.0, w, h, FLOOR);
        let mut yy = WALL + 4.0;
        let mut row = 0;
        while yy < h {
            p.r(0.0, yy, w, 1.0, PLANK);
            let mut xx = if row % 2 == 0 { 0.0 } else { 13.0 };
            while xx < w {
                p.r(xx, yy - 5.0, 1.0, 5.0, PLANK);
                xx += 26.0;
            }
            yy += 6.0;
            row += 1;
        }
        p.r(0.0, 0.0, w, WALL, WALLC);
        p.r(0.0, WALL - 2.0, w, 2.0, INK2);

        // the machines, lettered on the wall after the title bar's buttons
        let mut tx = tabs_x;
        let ty = 4.0;
        for (i, name) in rooms.iter().enumerate() {
            let tw = tiny_width(name) as f32;
            let hot = over(tx - 2.0, 0.0, tx + tw + 2.0, ty + 8.0);
            p.tiny(name, tx, ty, if i == cur { STEEL4 } else if hot { STEEL3 } else { STEEL1 });
            if i == cur {
                p.r(tx, ty + 7.0, tw, 1.0, TERRACOTTA);
            } else if hot {
                pointing = true;
                if resp.clicked() {
                    clicked = Some(Action::Room(i));
                }
            }
            tx += tw + 7.0;
        }
        {
            let hot = over(tx - 2.0, 0.0, tx + 5.0, ty + 8.0);
            p.tiny("+", tx, ty, if hot { TERRACOTTA } else { STEEL1 });
            if hot {
                pointing = true;
                if resp.clicked() {
                    clicked = Some(Action::AddRoom);
                }
            }
        }
        draw_door(&mut p, 5.0);
        draw_clock(&mut p, w - 10.0, 13.0);
        if let Some(n) = note {
            // a note goes on the wall, in red, instead of the whiteboard
            let nx = 22.0;
            let room_for = ((w - 60.0 - nx) / 4.0).max(0.0) as usize;
            let short: String = n.chars().take(room_for).collect();
            p.tiny(&short, nx, 16.0, hex(0xff6b5a));
        } else if let Some(b) = lay.board {
            p.e(b.left(), b.top(), b.width(), b.height(), STEEL3, STEEL1);
            p.r(b.left() + 2.0, b.bottom(), b.width() - 4.0, 1.0, STEEL1);
            let c = shade(self.board_by, 0.75);
            for &(kind, x, y, sw, sh) in &self.board {
                let (x, y) = (b.left() + 2.0 + x, b.top() + 2.0 + y);
                match kind {
                    0 => {
                        p.r(x, y, sw, 1.0, c);
                        p.r(x, y + sh, sw, 1.0, c);
                        p.r(x, y, 1.0, sh, c);
                        p.r(x + sw - 1.0, y, 1.0, sh + 1.0, c);
                    }
                    1 => {
                        p.r(x, y, sw, 1.0, c);
                        p.r(x + sw - 2.0, y - 1.0, 1.0, 1.0, c);
                        p.r(x + sw - 2.0, y + 1.0, 1.0, 1.0, c);
                    }
                    _ => p.r(x, y, 1.0, 1.0, c),
                }
            }
        }
        if let Some(wd) = lay.window {
            draw_window(&mut p, wd);
        }

        // flat on the floor: the mats, the ping-pong rug
        // a new project: an empty cabinet at the end, a + where the name goes
        {
            let a = lay.add;
            let hot = over(a.left() - 2.0, a.top() - 10.0, a.right() + 2.0, a.bottom() + 1.0);
            p.b(a.left(), a.top(), a.width(), a.height(), STEEL0);
            p.r(a.left(), a.top(), a.width(), 1.0, STEEL1);
            p.tiny("+", (a.center().x - 1.0).round(), a.top() + 4.0, if hot { STEEL4 } else { STEEL2 });
            if hot {
                let s = "PROJECT";
                p.tiny(s, (a.center().x - tiny_width(s) as f32 / 2.0).round(), a.top() - 8.0, STEEL3);
                pointing = true;
                if resp.clicked() {
                    clicked = Some(Action::AddDir);
                }
            }
        }
        for m in &lay.mats {
            draw_mat(&mut p, m.x, m.y);
        }
        if let Some(g) = lay.pong {
            let mut x = g.x - 34.0;
            while x < g.x + 34.0 {
                p.r(x, g.y - 18.0, 2.0, 1.0, INK2);
                p.r(x, g.y + 6.0, 2.0, 1.0, INK2);
                x += 4.0;
            }
            let mut y = g.y - 18.0;
            while y < g.y + 6.0 {
                p.r(g.x - 34.0, y, 1.0, 2.0, INK2);
                p.r(g.x + 33.0, y, 1.0, 2.0, INK2);
                y += 4.0;
            }
        }

        // everything that stands, back to front
        enum Item<'a> {
            Fig(&'a str),
            Chair(f32, f32, Color32, bool),
            Desk(&'a Desk),
            Table(Pos2),
            Stool(Pos2),
            Coffee,
            Cooler(Pos2),
            Printer(Pos2),
            Armchair(Pos2),
            Plant(Pos2),
            Pong(Pos2),
        }
        let mut items: Vec<(f32, Item)> = Vec::new();
        for b in bots {
            if let Some(f) = self.figs.get(&b.id) {
                if !f.lying {
                    items.push((f.y, Item::Fig(&b.id)));
                }
            }
        }
        for d in &lay.desks {
            // the desk behind its bots, the chairs behind them too
            items.push((d.y - 1.0, Item::Desk(d)));
            for (_, sx) in &d.seats {
                items.push((d.y - 0.5, Item::Chair(*sx, d.y, d.color, false)));
            }
            items.push((d.y - 0.5, Item::Chair(d.plus, d.y, d.color, true)));
        }
        if let Some(m) = lay.meet {
            items.push((m.y - 1.0, Item::Table(m)));
            for i in 0..2 {
                if let Some((s, _)) = lay.meet_seat(i) {
                    items.push((s.y - 0.5, Item::Stool(s)));
                }
            }
        }
        items.push((lay.floor_y, Item::Coffee));
        if let Some(c) = lay.cooler {
            items.push((c.y, Item::Cooler(c)));
        }
        if let Some(x) = lay.printer {
            items.push((x.y, Item::Printer(x)));
        }
        if let Some(c) = lay.chair {
            items.push((c.y - 0.5, Item::Armchair(c)));
        }
        for pl in &lay.plants {
            items.push((pl.y, Item::Plant(*pl)));
        }
        if let Some(g) = lay.pong {
            items.push((g.y - 0.5, Item::Pong(g)));
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut hovered: Option<(String, f32, f32)> = None;
        let brewing = self.figs.values().any(|f| matches!(f.act, Act::Coffee { stage: 1, .. }));
        let printing = self.figs.values().find_map(|f| if let Act::Print { stage: 1, wait } = f.act { Some(wait) } else { None });
        let playing: Vec<&Fig> = self.figs.values().filter(|f| matches!(f.act, Act::Pong { there: true, .. })).collect();
        let ball = (playing.len() == 2).then_some(self.pong);

        // the lying ones first: flat on their mats
        for b in bots {
            let Some(f) = self.figs.get(&b.id) else { continue };
            if !f.lying {
                continue;
            }
            let color = project_color(&b.cwd);
            let (fx, fy) = (f.x.round(), f.y.round());
            let sprite = self.sprites.get(&b.id, color, Arms::Down, Legs::Stand);
            p.sprite(&sprite, fx - 11.0, fy - 28.0, false, 16, None);
            p.b(fx - 11.0, fy - 13.0, 22.0, 11.0, STEEL1);
            p.r(fx - 11.0, fy - 13.0, 22.0, 2.0, STEEL2);
            p.r(fx - 11.0, fy - 4.0, 22.0, 2.0, STEEL0);
            p.r(fx - 2.0, fy - 9.0, 3.0, 1.0, GOLD2);
            p.r(fx - 1.0, fy - 10.0, 1.0, 3.0, GOLD2);
            if (t * 2.0) as i32 % 2 == 0 {
                p.r(fx + 6.0, fy - 9.0, 2.0, 2.0, GREEN1);
            }
            zzz(&mut p, fx + 6.0, fy - 24.0, f.z);
            if over(fx - 11.0, fy - 28.0, fx + 11.0, fy) {
                hovered = Some((b.id.clone(), fx, fy - 28.0));
            }
        }
        for (_, it) in &items {
            match it {
                Item::Fig(id) => {
                    let (Some(f), Some(b)) = (self.figs.get(*id), bots.iter().find(|b| b.id == *id)) else { continue };
                    let color = project_color(&b.cwd);
                    let (fx, fy) = (f.x.round(), f.y.round());
                    let sprite = self.sprites.get(id, color, f.arms, f.legs);
                    let (x0, y0) = (fx - 11.0, fy - 27.0);
                    p.r(fx - 5.0, fy, 10.0, 1.0, Color32::from_black_alpha(100));
                    // one in a terminal elsewhere: greyed, like pair's people who are away
                    p.sprite(&sprite, x0, y0, f.dir < 0.0, SH as i8, (!b.background).then_some(STEEL0));
                    if matches!(f.act, Act::Sleep { .. }) {
                        zzz(&mut p, fx + 2.0, y0 + 4.0, f.z);
                    }
                    if matches!(f.act, Act::Plant { .. }) && f.arms == Arms::Can && (t * 6.0) as i32 % 2 == 1 {
                        p.r(fx + 10.0 * f.dir, y0 + 21.0, 1.0, 1.0, BLUE3);
                        p.r(fx + 11.0 * f.dir, y0 + 24.0, 1.0, 1.0, BLUE3);
                    }
                    if open == Some(*id) {
                        for (i, wd) in [5.0, 3.0, 1.0].iter().enumerate() {
                            p.r(fx - (wd / 2.0_f32).floor(), y0 - 6.0 + i as f32, *wd, 1.0, TERRACOTTA);
                        }
                    }
                    if over(x0 + 4.0, y0, x0 + 18.0, fy + 1.0) {
                        hovered = Some((id.to_string(), fx, y0));
                    }
                }
                Item::Chair(x, y, c, plus) => {
                    // pair's chair, turned round: its sitter faces left, to the screen
                    let (x, y) = (*x, *y);
                    let hot = *plus && over(x - 8.0, y - 22.0, x + 9.0, y + 1.0);
                    let col = if *plus && !hot { INK2 } else { STEEL0 };
                    p.b(x + 6.0, y - 15.0, 2.0, 9.0, col);
                    p.b(x - 2.0, y - 7.0, 9.0, 2.0, col);
                    p.r(x + 2.0, y - 4.0, 1.0, 3.0, if *plus && !hot { STEEL0 } else { STEEL1 });
                    p.r(x - 1.0, y - 1.0, 7.0, 1.0, if *plus && !hot { STEEL0 } else { STEEL1 });
                    if *plus {
                        p.tiny("+", x, y - 16.0, if hot { *c } else { STEEL1 });
                        if hot {
                            pointing = true;
                            if resp.clicked() {
                                let d = lay.desks.iter().find(|d| d.plus == x && d.y == y).unwrap();
                                clicked = Some(Action::Spawn(d.cwd.clone()));
                            }
                        }
                    }
                }
                Item::Desk(d) => {
                    draw_desk(&mut p, d, &self.figs, t);
                    // a project with no bots can be forgotten: an x on its cabinet
                    if d.seats.is_empty() && over(d.x0, d.y - 32.0, d.x0 + d.cab, d.y) {
                        let hot = over(d.x0 + d.cab - 6.0, d.y - 32.0, d.x0 + d.cab + 2.0, d.y - 25.0);
                        p.tiny("X", d.x0 + d.cab - 4.0, d.y - 31.0, if hot { TERRACOTTA } else { STEEL2 });
                        if hot {
                            pointing = true;
                            if resp.clicked() {
                                clicked = Some(Action::Forget(d.cwd.clone()));
                            }
                        }
                    }
                }
                Item::Table(m) => draw_table(&mut p, *m),
                Item::Stool(st) => {
                    p.b(st.x - 4.0, st.y - 7.0, 8.0, 2.0, WOOD_D);
                    p.r(st.x - 3.0, st.y - 4.0, 1.0, 4.0, WOOD_D);
                    p.r(st.x + 2.0, st.y - 4.0, 1.0, 4.0, WOOD_D);
                }
                Item::Coffee => draw_coffee(&mut p, 3.0, lay.floor_y, brewing, t),
                Item::Cooler(c) => draw_cooler(&mut p, c.x, c.y),
                Item::Printer(pr) => draw_printer(&mut p, pr.x, pr.y, printing),
                Item::Armchair(c) => draw_armchair(&mut p, c.x, c.y),
                Item::Plant(pl) => draw_plant(&mut p, pl.x, pl.y),
                Item::Pong(g) => draw_pong(&mut p, *g, ball),
            }
        }

        // talk, over everything: dots in a bubble, never the words
        for f in self.figs.values() {
            if f.chat > 0.0 {
                let (ax, ay) = (f.x.round() + 1.0, if f.lying { f.y.round() - 28.0 } else { f.y.round() - 27.0 });
                let (bx, by) = (ax + 2.0, ay - 6.0);
                p.r(bx - 1.0, by - 1.0, 11.0, 7.0, INK);
                p.r(bx, by, 9.0, 5.0, STEEL4);
                p.r(bx, by + 5.0, 1.0, 1.0, STEEL4);
                for i in 0..3 {
                    if ((f.chat * 4.0) as i32 + i) % 3 != 0 {
                        p.r(bx + 2.0 + i as f32 * 2.0, by + 2.0, 1.0, 1.0, INK);
                    }
                }
            }
        }

        // the hovered bot's name over its head
        if let Some((id, fx, fy)) = &hovered {
            let b = bots.iter().find(|b| &b.id == id).unwrap();
            let mut s = b.name.clone();
            if !b.background {
                s.push_str(" (in a terminal)");
            }
            let tw = tiny_width(&s) as f32;
            let lx = (fx - tw / 2.0).clamp(2.0, (w - tw - 2.0).max(2.0)).round();
            let ly = (fy - 10.0).max(WALL + 2.0).round();
            p.b(lx - 2.0, ly - 2.0, tw + 4.0, 9.0, INK2);
            p.tiny(&s, lx, ly, STEEL4);
            if b.background {
                pointing = true;
                if resp.clicked() {
                    clicked = Some(Action::Open(id.clone()));
                }
            }
        }
        let mesh = p.m;
        ui.painter().with_clip_rect(rect).add(mesh);

        // the wall is the title bar: drag the window by it, double-click to zoom
        if resp.drag_started() && resp.interact_pointer_pos().is_some_and(|q| (q.y - o.y) / k < WALL) && !pointing {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if resp.double_clicked() && hover.is_some_and(|(_, hy)| hy < WALL) && !pointing {
            let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
        }
        if pointing {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        clicked
    }

    // ---- what everyone does, after floor.js's think()

    fn think(&mut self, bots: &[Bot], lay: &Layout, now: f64, dt: f32) {
        let ids: Vec<String> = bots.iter().map(|b| b.id.clone()).collect();
        for b in bots {
            if let Some(f) = self.figs.get_mut(&b.id) {
                f.chat = (f.chat - dt).max(0.0);
                if b.busy != f.busy {
                    f.busy = b.busy;
                    f.idle_since = now;
                    // to work: whatever it was doing (a message still gets delivered first)
                    if b.busy && !matches!(f.act, Act::Visit { msg: true, .. }) {
                        end(f);
                        f.act = Act::Desk;
                    }
                    if !b.busy && f.act == Act::Desk {
                        f.act = Act::Stand { left: 0.5 + (fnv(&b.id) % 20) as f32 / 10.0 };
                    }
                }
            }
        }
        if let Some((_, left)) = &mut self.meeting {
            *left -= dt;
            if *left <= 0.0 {
                self.meeting = None;
                for f in self.figs.values_mut() {
                    if matches!(f.act, Act::Meeting { .. } | Act::Present { .. }) {
                        end(f);
                    }
                }
            }
        }
        // the ball goes back and forth while two are at the table
        if self.figs.values().filter(|f| matches!(f.act, Act::Pong { there: true, .. })).count() == 2 {
            self.pong.0 += dt / 0.8;
            if self.pong.0 >= 1.0 {
                self.pong = (0.0, 1 - self.pong.1);
            }
        }
        for id in &ids {
            self.think_one(id, lay, now, dt);
        }
        // two standing about in the same place: they step apart
        let still: Vec<(String, f32, f32)> = self
            .figs
            .iter()
            .filter(|(_, f)| f.legs == Legs::Stand && matches!(f.act, Act::Stand { .. } | Act::Loiter { .. }))
            .map(|(k, f)| (k.clone(), f.x, f.y))
            .collect();
        for (a, ax, ay) in &still {
            for (b, bx, by) in &still {
                if a < b && (ax - bx).abs() < 12.0 && (ay - by).abs() < 4.0 {
                    let d = if ax <= bx { -1.0 } else { 1.0 };
                    if let Some(f) = self.figs.get_mut(a) {
                        f.x += d * 8.0 * dt;
                    }
                    if let Some(f) = self.figs.get_mut(b) {
                        f.x -= d * 8.0 * dt;
                    }
                }
            }
        }
    }

    fn think_one(&mut self, id: &str, lay: &Layout, now: f64, dt: f32) {
        let pos: HashMap<String, (f32, f32, bool)> = self.figs.iter().map(|(k, f)| (k.clone(), (f.x, f.y, f.legs == Legs::Sit))).collect();
        let idle_mates: Vec<String> = self
            .figs
            .iter()
            .filter(|(k, f)| k.as_str() != id && !f.busy && matches!(f.act, Act::Stand { .. } | Act::Loiter { .. }))
            .map(|(k, _)| k.clone())
            .collect();
        let rs: [f32; 6] = std::array::from_fn(|_| self.rnd());
        let taken_chair = self.figs.values().any(|f| matches!(f.act, Act::Chair { .. }));
        // one at a time at the coffee machine, the printer, the window, a plant
        let taken = |m: fn(&Act) -> bool| self.figs.iter().any(|(k, f)| k != id && m(&f.act));
        let (coffee_taken, print_taken, window_taken) =
            (taken(|a| matches!(a, Act::Coffee { .. })), taken(|a| matches!(a, Act::Print { .. })), taken(|a| matches!(a, Act::Window { .. })));
        let plants_taken: Vec<usize> = self.figs.iter().filter(|(k, _)| k.as_str() != id).filter_map(|(_, f)| if let Act::Plant { i, .. } = f.act { Some(i) } else { None }).collect();
        // somewhere free to stand about
        let mut spot = None;
        for _ in 0..8 {
            let (a, b) = (self.rnd(), self.rnd());
            let (x, y) = (lay.floor.left() + a * lay.floor.width(), lay.floor.top() + b * lay.floor.height());
            if lay.free(x, y) {
                spot = Some((x, y));
                break;
            }
        }
        let pong_busy = self.figs.values().any(|f| matches!(f.act, Act::Pong { .. }));
        let mats_taken: Vec<(f32, f32)> = self.figs.iter().filter(|(k, f)| k.as_str() != id && matches!(f.act, Act::Sleep { there: true }) && f.lying).map(|(_, f)| (f.x, f.y)).collect();
        let meeting = self.meeting.clone();
        let (t, last_meeting, ball, self_sleep) = (self.t, self.last_meeting, self.pong, self.sleep_after);
        let seat = lay.seat(id);
        let mut partner: Option<(String, Act)> = None;
        let mut start_meeting: Option<Vec<String>> = None;
        let mut stroke = false;

        let Some(f) = self.figs.get_mut(id) else { return };
        let v = 22.0;
        let sleepy = !f.busy && now - f.idle_since > self_sleep;
        if sleepy && !matches!(f.act, Act::Sleep { .. } | Act::Visit { msg: true, .. }) {
            end(f);
            f.act = Act::Sleep { there: false };
        }
        if !sleepy && matches!(f.act, Act::Sleep { .. }) {
            end(f);
        }
        let mut act = f.act.clone();
        match &mut act {
            Act::Desk => {
                if let Some(s) = seat {
                    if walk(f, s, v, dt) {
                        f.dir = -1.0;
                        f.legs = Legs::Sit;
                        f.carry = None;
                        f.arms = if f.busy { if (t * 6.0) as i32 % 2 == 0 { Arms::Type1 } else { Arms::Type2 } } else { Arms::Down };
                    }
                }
            }
            Act::Stand { left } => {
                f.legs = Legs::Stand;
                f.arms = f.carry.unwrap_or(Arms::Down);
                *left -= dt;
                if *left <= 0.0 {
                    // a break, chosen by someone with nothing to do (pair's pickBreak)
                    let r = rs[0];
                    let mate = (!idle_mates.is_empty()).then(|| idle_mates[((rs[1] * idle_mates.len() as f32) as usize).min(idle_mates.len() - 1)].clone());
                    f.carry = None;
                    act = if lay.meet.is_some() && idle_mates.len() >= 2 && meeting.is_none() && t - last_meeting > 60.0 && r < 0.08 {
                        start_meeting = Some(idle_mates.iter().take(4).cloned().collect());
                        Act::Present { wait: 0.0 }
                    } else if r < 0.22 && !coffee_taken {
                        Act::Coffee { stage: 0, wait: 0.0 }
                    } else if r < 0.32 && lay.printer.is_some() && !print_taken {
                        Act::Print { stage: 0, wait: 0.0 }
                    } else if r < 0.46 && mate.is_some() && lay.cooler.is_some() {
                        let left = 10.0 + rs[2] * 12.0;
                        let m = mate.clone().unwrap();
                        partner = Some((m.clone(), Act::Cooler { side: 1, left, with: id.to_string() }));
                        Act::Cooler { side: 0, left, with: m }
                    } else if r < 0.56 && mate.is_some() && lay.pong.is_some() && !pong_busy {
                        let left = 15.0 + rs[2] * 20.0;
                        let m = mate.clone().unwrap();
                        partner = Some((m.clone(), Act::Pong { side: 1, left, with: id.to_string(), there: false }));
                        Act::Pong { side: 0, left, with: m, there: false }
                    } else if r < 0.62 && mate.is_some() {
                        Act::Visit { to: mate.unwrap(), left: 6.0 + rs[2] * 6.0, wave: 0.8, msg: false }
                    } else if r < 0.70 && plants_taken.len() < lay.plants.len() {
                        let free: Vec<usize> = (0..lay.plants.len()).filter(|i| !plants_taken.contains(i)).collect();
                        Act::Plant { i: free[((rs[2] * free.len() as f32) as usize).min(free.len() - 1)], wait: 0.0 }
                    } else if r < 0.78 && lay.window.is_some() && !window_taken {
                        Act::Window { left: 4.0 + rs[2] * 5.0 }
                    } else if r < 0.86 && lay.chair.is_some() && !taken_chair {
                        Act::Chair { left: 15.0 + rs[2] * 20.0 }
                    } else if let Some((x, y)) = spot {
                        Act::Loiter { x, y, left: 3.0 + rs[5] * 5.0 }
                    } else {
                        rest(rs[5])
                    };
                }
            }
            Act::Loiter { x, y, left } => {
                if walk(f, Pos2::new(*x, *y), v, dt) {
                    f.arms = f.carry.unwrap_or(Arms::Down);
                    *left -= dt;
                    if *left <= 0.0 {
                        act = rest(rs[0]);
                    }
                }
            }
            Act::Coffee { stage, wait } => {
                if *stage == 0 {
                    if walk(f, lay.coffee, v, dt) {
                        f.dir = -1.0;
                        f.arms = Arms::Reach;
                        *stage = 1;
                    }
                } else {
                    *wait += dt;
                    f.arms = if *wait > 2.0 { Arms::Mug } else { Arms::Reach };
                    if *wait > 2.8 {
                        f.carry = Some(Arms::Mug);
                        act = rest(rs[0]);
                    }
                }
            }
            Act::Print { stage, wait } => match lay.printer {
                None => act = rest(rs[0]),
                Some(pr) => {
                    if *stage == 0 {
                        if walk(f, Pos2::new(pr.x - 9.0, pr.y), v, dt) {
                            f.dir = 1.0;
                            f.arms = Arms::Down;
                            *stage = 1;
                        }
                    } else {
                        *wait += dt;
                        if *wait > 2.4 {
                            f.carry = Some(Arms::Paper);
                            act = rest(rs[0]);
                        }
                    }
                }
            },
            Act::Cooler { side, left, with } => match (lay.cooler_spot(*side), pos.get(with.as_str())) {
                (Some(s), Some(&(ox, oy, _))) => {
                    if walk(f, s, v, dt) {
                        f.dir = if *side == 0 { 1.0 } else { -1.0 };
                        f.arms = Arms::Down;
                        let both = (ox - f.x).abs() < 24.0 && (oy - f.y).abs() < 4.0;
                        if both && rs[0] < dt * 0.4 {
                            f.chat = 1.2;
                        }
                        *left -= dt;
                        if *left <= 0.0 {
                            act = rest(rs[1]);
                        }
                    }
                }
                _ => act = rest(rs[1]),
            },
            Act::Pong { side, left, with, there } => match (lay.pong_spot(*side), pos.get(with.as_str())) {
                (Some(s), Some(_)) => {
                    *there = walk(f, s, v, dt);
                    if *there {
                        f.dir = if *side == 0 { 1.0 } else { -1.0 };
                        let (bt, from) = ball;
                        f.arms = if from != *side && bt > 0.75 { Arms::Type1 } else { Arms::Type2 };
                        *left -= dt;
                        if *left <= 0.0 {
                            act = rest(rs[1]);
                        }
                    }
                }
                _ => act = rest(rs[1]),
            },
            Act::Visit { to, left, wave, msg } => match pos.get(to.as_str()) {
                Some(&(ox, oy, oseated)) => {
                    // next to the other one, on the side it comes from; beside the chair if seated
                    // a seated one faces its screen (left): stand behind its chair; else beside it, on the
                    // side it comes from, as in pair
                    let side = if f.x < ox { -1.0 } else { 1.0 };
                    let spot = if oseated { Pos2::new(ox + 20.0, oy + 1.0) } else { Pos2::new(ox + side * 14.0, oy + 1.0) };
                    let speed = if *msg { 34.0 } else { v + 6.0 };
                    if walk(f, spot, speed, dt) {
                        f.dir = if spot.x > ox { -1.0 } else { 1.0 };
                        if *wave > 0.0 {
                            *wave -= dt;
                            f.arms = Arms::Wave;
                        } else {
                            f.arms = Arms::Down;
                        }
                        if f.chat <= 0.0 && rs[0] < dt * 1.5 {
                            f.chat = 1.0;
                        }
                        partner = Some((to.clone(), Act::Stand { left: -1.0 })); // marker: face me, chat back
                        *left -= dt;
                        if *left <= 0.0 {
                            act = if f.busy { Act::Desk } else { rest(rs[1]) };
                        }
                    }
                }
                None => act = if f.busy { Act::Desk } else { rest(rs[1]) },
            },
            Act::Plant { i, wait } => {
                let pl = lay.plants.get(*i).copied().unwrap_or(lay.plants[0]);
                let side = if pl.x > lay.floor.center().x { -1.0 } else { 1.0 };
                f.carry = Some(Arms::Can);
                if walk(f, Pos2::new(pl.x + side * 12.0, pl.y + 1.0), v, dt) {
                    f.dir = -side;
                    f.arms = Arms::Can;
                    *wait += dt;
                    if *wait > 3.0 {
                        f.carry = None;
                        act = rest(rs[0]);
                    }
                }
            }
            Act::Window { left } => match lay.window {
                None => act = rest(rs[0]),
                Some(wd) => {
                    if walk(f, Pos2::new(wd.center().x.round(), WALL + 5.0), v, dt) {
                        f.arms = Arms::Down;
                        f.dir = if (t * 0.7 + f.z).sin() > 0.0 { 1.0 } else { -1.0 };
                        *left -= dt;
                        if *left <= 0.0 {
                            act = rest(rs[0]);
                        }
                    }
                }
            },
            Act::Chair { left } => match lay.chair {
                None => act = rest(rs[0]),
                Some(c) => {
                    if walk(f, Pos2::new(c.x - 2.0, c.y - 1.0), v, dt) {
                        f.dir = -1.0;
                        f.legs = Legs::Sit;
                        f.arms = Arms::Book;
                        *left -= dt;
                        if *left <= 0.0 {
                            act = rest(rs[0]);
                        }
                    }
                }
            },
            Act::Present { wait } => match (lay.board, &meeting) {
                (Some(b), Some((host, _))) if host == id => {
                    if walk(f, Pos2::new(b.left() - 6.0, WALL + 3.0), v, dt) {
                        f.dir = 1.0;
                        f.arms = if (t * 2.0) as i32 % 3 != 0 { Arms::Reach } else { Arms::Down };
                        *wait += dt;
                        if *wait > 0.6 {
                            *wait = 0.0;
                            stroke = true;
                        }
                        if rs[0] < dt * 0.3 {
                            f.chat = 1.0;
                        }
                    }
                }
                (Some(_), None) if start_meeting.is_some() => {}
                _ => act = rest(rs[0]),
            },
            Act::Meeting { seat: s } => match (lay.meet_seat(*s), &meeting) {
                (Some((p, d)), Some(_)) => {
                    if walk(f, p, v, dt) {
                        f.dir = d;
                        f.legs = Legs::Sit;
                        f.arms = Arms::Down;
                        if rs[0] < dt * 0.08 {
                            f.chat = 1.0;
                        }
                    }
                }
                _ => act = rest(rs[0]),
            },
            Act::Sleep { there } => {
                // on a free mat if there is one, else asleep at the desk
                let mat = lay.mats.iter().find(|m| !mats_taken.iter().any(|(x, y)| (x - m.x).abs() < 1.0 && (y - m.y).abs() < 1.0)).copied();
                let to = mat.or(seat);
                if let Some(to) = to {
                    if *there || walk(f, to, v, dt) {
                        *there = true;
                        f.x = to.x;
                        f.y = to.y;
                        f.lying = mat.is_some();
                        f.dir = 1.0;
                        f.legs = Legs::Sit;
                        f.arms = Arms::Down;
                        f.z += dt;
                    }
                }
            }
        }
        f.act = act;
        if f.legs != Legs::Sit {
            f.lying = false;
        }

        if let Some((other, a)) = partner {
            if let Some(g) = self.figs.get_mut(&other) {
                match a {
                    // visited: an idle one stops and faces the visitor, and talks back now and then
                    Act::Stand { left } if left < 0.0 => {
                        if !g.busy && matches!(g.act, Act::Stand { .. } | Act::Loiter { .. }) {
                            g.act = Act::Stand { left: 2.0 };
                            g.legs = Legs::Stand;
                        }
                        if g.chat <= 0.0 && rs[2] < dt * 1.2 {
                            g.chat = 1.0;
                        }
                    }
                    a => {
                        if matches!(g.act, Act::Stand { .. } | Act::Loiter { .. }) {
                            g.carry = None;
                            g.act = a;
                        }
                    }
                }
            }
        }
        if let Some(mates) = start_meeting {
            self.meeting = Some(id.to_string()).map(|h| (h, 22.0 + rs[3] * 16.0));
            self.last_meeting = t;
            self.board.clear();
            self.board_by = self.figs.get(id).map(|_| BLUE2).unwrap_or(BLUE2);
            for (i, m) in mates.iter().enumerate() {
                if let Some(g) = self.figs.get_mut(m) {
                    g.carry = None;
                    g.act = Act::Meeting { seat: i };
                }
            }
        }
        if stroke && self.board.len() < 40 {
            if let Some(b) = lay.board {
                let (bw, bh) = (b.width() - 4.0, b.height() - 4.0);
                let (x, y, k) = ((rs[3] * (bw - 6.0)).floor(), (1.0 + rs[4] * (bh - 3.0)).floor(), rs[5]);
                self.board.push(if k < 0.35 {
                    (0, x, y, (3.0 + rs[0] * 4.0).floor(), (2.0 + rs[1] * 2.0).floor())
                } else if k < 0.7 {
                    (1, x, y, (3.0 + rs[0] * 7.0).floor(), 0.0)
                } else {
                    (2, x, y, 0.0, 0.0)
                });
            }
        }
    }
}

/// a pause where it is, standing (with whatever it carries), before the next break
fn rest(r: f32) -> Act {
    Act::Stand { left: 2.0 + r * 5.0 }
}

fn end(f: &mut Fig) {
    f.lying = false;
    f.carry = None;
    f.act = rest(0.3);
}

fn walk(f: &mut Fig, to: Pos2, v: f32, dt: f32) -> bool {
    let (dx, dy) = (to.x - f.x, to.y - f.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d <= v * dt {
        f.x = to.x;
        f.y = to.y;
        if f.legs != Legs::Sit {
            f.legs = Legs::Stand;
        }
        return true;
    }
    f.x += dx / d * v * dt;
    f.y += dy / d * v * dt;
    if dx.abs() > 0.5 {
        f.dir = dx.signum();
    }
    f.step += dt * 8.0;
    let k = f.step as i32 % 2 == 1;
    f.legs = if k { Legs::Walk1 } else { Legs::Walk2 };
    f.arms = f.carry.unwrap_or(if k { Arms::Swing1 } else { Arms::Swing2 });
    false
}

// ---- the furniture, after pair's floor.js

fn draw_desk(p: &mut Paint, d: &Desk, figs: &HashMap<String, Fig>, t: f32) {
    let (x0, y, w, cab) = (d.x0, d.y, d.w, d.cab);
    let busy = d.seats.iter().any(|(id, _)| figs.get(id).is_some_and(|f| f.busy));
    let here = d.seats.iter().any(|(id, _)| figs.get(id).is_some_and(|f| matches!(f.act, Act::Desk)));
    // the long desktop, low, where the bots' hands are
    p.b(cab - 1.0 + x0, y - 9.0, w - cab + 1.0, 2.0, WOOD);
    p.r(cab + x0, y - 7.0, w - cab, 1.0, WOOD_D);
    p.b(x0 + w - 3.0, y - 6.0, 2.0, 6.0, WOOD_D);
    // the cabinet, the project's name on it
    p.b(x0, y - 13.0, cab, 13.0, WOOD_D);
    p.r(x0, y - 13.0, cab, 1.0, WOOD);
    let label = project_label(&d.cwd);
    let lw = tiny_width(&label) as f32;
    p.tiny(&label, (x0 + (cab - lw) / 2.0).round(), y - 9.0, STEEL3);
    // the screen on it: the project's work scrolls by in its colour while a bot types
    let sx = (x0 + cab / 2.0 - 10.0).round();
    p.r(sx + 9.0, y - 16.0, 2.0, 3.0, STEEL1);
    p.r(sx + 6.0, y - 14.0, 8.0, 1.0, STEEL1);
    p.b(sx, y - 31.0, 20.0, 15.0, STEEL0);
    p.r(sx + 1.0, y - 30.0, 18.0, 13.0, if busy || here { BLUE0 } else { NIGHT });
    if busy || here {
        let col = if busy { d.color } else { shade(d.color, 0.45) };
        let s = if busy { (t * 5.0) as usize } else { 0 };
        for row in 0..6 {
            let len = 3 + ((row * 7 + s + d.cwd.len()) * 5) % 12;
            p.r(sx + 2.0 + ((row * 3) % 4) as f32, y - 29.0 + row as f32 * 2.0, len as f32, 1.0, col);
        }
    }
}

fn draw_table(p: &mut Paint, m: Pos2) {
    p.b(m.x - 16.0, m.y - 11.0, 32.0, 4.0, WOOD);
    p.r(m.x - 16.0, m.y - 8.0, 32.0, 1.0, WOOD_D);
    p.b(m.x - 13.0, m.y - 6.0, 2.0, 6.0, WOOD_D);
    p.b(m.x + 11.0, m.y - 6.0, 2.0, 6.0, WOOD_D);
    p.r(m.x - 6.0, m.y - 10.0, 3.0, 2.0, STEEL4);
    p.r(m.x + 4.0, m.y - 10.0, 3.0, 2.0, STEEL4);
}

fn draw_mat(p: &mut Paint, x: f32, y: f32) {
    p.b(x - 12.0, y - 24.0, 24.0, 23.0, hex(0x1c2538));
    let mut yy = y - 21.0;
    while yy < y - 2.0 {
        p.r(x - 12.0, yy, 24.0, 1.0, hex(0x222d45));
        yy += 3.0;
    }
    p.b(x - 8.0, y - 23.0, 16.0, 6.0, STEEL2);
}

fn zzz(p: &mut Paint, x: f32, y: f32, z: f32) {
    for i in 0..2 {
        let kk = (z * 0.4 + i as f32 * 0.5) % 1.0;
        let (zx, zy) = ((x + kk * 6.0).round(), (y - kk * 10.0).round());
        p.r(zx, zy, 3.0, 1.0, STEEL3);
        p.r(zx + 1.0, zy + 1.0, 1.0, 1.0, STEEL3);
        p.r(zx, zy + 2.0, 3.0, 1.0, STEEL3);
    }
}

fn draw_door(p: &mut Paint, x: f32) {
    let (y0, h) = (12.0, WALL - 14.0);
    p.b(x, y0, 12.0, h, WOOD);
    p.r(x + 2.0, y0 + 2.0, 8.0, 5.0, WOOD_D);
    p.r(x + 2.0, y0 + 9.0, 8.0, h - 11.0, WOOD_D);
    p.r(x + 9.0, y0 + 8.0, 1.0, 1.0, GOLD2);
}

fn draw_clock(p: &mut Paint, x: f32, y: f32) {
    p.r(x - 4.0, y - 3.0, 9.0, 7.0, INK);
    p.r(x - 3.0, y - 4.0, 7.0, 9.0, INK);
    p.r(x - 3.0, y - 2.0, 7.0, 5.0, STEEL4);
    p.r(x - 2.0, y - 3.0, 5.0, 7.0, STEEL4);
    let (hh, mm) = local_hm();
    let hand = |p: &mut Paint, ang: f32, len: i32, c: Color32| {
        for i in 1..=len {
            p.r((x + ang.sin() * i as f32).round(), (y - ang.cos() * i as f32).round(), 1.0, 1.0, c);
        }
    };
    p.r(x, y, 1.0, 1.0, INK);
    hand(p, ((hh % 12) as f32 + mm as f32 / 60.0) / 12.0 * std::f32::consts::TAU, 2, INK);
    hand(p, mm as f32 / 60.0 * std::f32::consts::TAU, 3, RED1);
}

fn draw_window(p: &mut Paint, r: Rect) {
    let (hh, _) = local_hm();
    let day = (7..20).contains(&hh);
    let (x0, y0, ww, wh) = (r.left(), r.top(), r.width(), r.height());
    p.e(x0, y0, ww, wh, if day { BLUE2 } else { BLUE0 }, STEEL2);
    if day {
        p.r(x0, y0 + wh - 5.0, ww, 5.0, BLUE3);
        p.r(x0 + 3.0, y0 + 3.0, 5.0, 1.0, STEEL4);
        p.r(x0 + 2.0, y0 + 4.0, 7.0, 1.0, STEEL4);
    } else {
        p.r(x0 + 4.0, y0 + 3.0, 1.0, 1.0, STEEL4);
        p.r(x0 + ww - 5.0, y0 + 8.0, 1.0, 1.0, STEEL4);
        p.r(x0 + ww - 6.0, y0 + 2.0, 2.0, 2.0, GOLD2);
    }
    p.r((x0 + ww / 2.0).round(), y0, 1.0, wh, STEEL2);
    p.r(x0, (y0 + wh / 2.0).round(), ww, 1.0, STEEL2);
}

fn draw_coffee(p: &mut Paint, x0: f32, y: f32, brewing: bool, t: f32) {
    p.b(x0, y - 10.0, 20.0, 10.0, STEEL1);
    p.r(x0 + 2.0, y - 7.0, 7.0, 6.0, STEEL0);
    p.r(x0 + 11.0, y - 7.0, 7.0, 6.0, STEEL0);
    p.b(x0 + 6.0, y - 19.0, 8.0, 8.0, INK2);
    p.r(x0 + 12.0, y - 17.0, 1.0, 1.0, RED1);
    p.r(x0 + 8.0, y - 14.0, 4.0, 1.0, STEEL3);
    if brewing {
        p.r(x0 + 9.0, y - 13.0 + ((t * 8.0) as i32 % 2) as f32, 1.0, 1.0, WOOD);
        p.r(x0 + 10.0, y - 22.0 - ((t * 4.0) as i32 % 2) as f32, 1.0, 1.0, STEEL3);
    }
    p.r(x0 + 2.0, y - 13.0, 2.0, 2.0, WHITE);
}

fn draw_cooler(p: &mut Paint, x: f32, y: f32) {
    p.b(x - 3.0, y - 10.0, 6.0, 10.0, STEEL3);
    p.b(x - 2.0, y - 17.0, 4.0, 6.0, BLUE3);
    p.r(x - 2.0, y - 14.0, 4.0, 3.0, BLUE2);
    p.r(x - 1.0, y - 7.0, 2.0, 1.0, STEEL1);
}

fn draw_printer(p: &mut Paint, x: f32, y: f32, printing: Option<f32>) {
    p.b(x - 5.0, y - 6.0, 10.0, 6.0, STEEL1);
    p.b(x - 6.0, y - 12.0, 12.0, 5.0, STEEL3);
    p.r(x - 4.0, y - 10.0, 8.0, 1.0, INK2);
    p.r(x + 4.0, y - 9.0, 1.0, 1.0, GREEN1);
    if let Some(wait) = printing {
        let k = (wait * 2.0).floor().min(4.0);
        p.r(x - 3.0, y - 13.0 - k, 6.0, k + 1.0, STEEL4);
    }
}

fn draw_armchair(p: &mut Paint, x: f32, y: f32) {
    p.b(x - 8.0, y - 9.0, 14.0, 6.0, PURPLE0);
    p.b(x + 4.0, y - 18.0, 4.0, 15.0, PURPLE0);
    p.b(x - 9.0, y - 11.0, 3.0, 7.0, PURPLE1);
    p.r(x - 7.0, y - 3.0, 2.0, 3.0, INK);
    p.r(x + 4.0, y - 3.0, 2.0, 3.0, INK);
    // a lamp beside it
    p.r(x + 13.0, y - 18.0, 1.0, 18.0, STEEL1);
    p.b(x + 10.0, y - 22.0, 7.0, 4.0, GOLD2);
}

fn draw_plant(p: &mut Paint, x: f32, y: f32) {
    p.b(x - 3.0, y - 5.0, 6.0, 5.0, GOLD0);
    let leaves = [(-4.0, -9.0, GREEN0), (2.0, -11.0, GREEN1), (-1.0, -14.0, GREEN1), (3.0, -7.0, GREEN0), (-3.0, -12.0, GREEN1)];
    for (dx, dy, _) in leaves {
        p.r(x + dx - 1.0, y + dy - 1.0, 4.0, 5.0, INK);
    }
    for (dx, dy, c) in leaves {
        p.r(x + dx, y + dy, 2.0, 3.0, c);
    }
}

fn draw_pong(p: &mut Paint, g: Pos2, ball: Option<(f32, usize)>) {
    let scrub1 = hex(0x1b6660);
    p.b(g.x - 14.0, g.y - 12.0, 28.0, 5.0, scrub1);
    p.r(g.x - 14.0, g.y - 10.0, 28.0, 1.0, STEEL4);
    p.r(g.x - 14.0, g.y - 7.0, 28.0, 1.0, hex(0x10363f));
    p.r(g.x - 1.0, g.y - 16.0, 2.0, 5.0, INK);
    p.r(g.x, g.y - 15.0, 1.0, 4.0, STEEL3);
    p.b(g.x - 12.0, g.y - 6.0, 2.0, 6.0, STEEL1);
    p.b(g.x + 10.0, g.y - 6.0, 2.0, 6.0, STEEL1);
    if let Some((bt, from)) = ball {
        let a = if from == 1 { 1.0 } else { -1.0 };
        let x = g.x + a * 18.0 * (1.0 - 2.0 * bt);
        let kk = if bt < 0.5 { bt * 2.0 } else { (bt - 0.5) * 2.0 };
        let y = g.y - 12.0 - ((kk * std::f32::consts::PI).sin() * if bt < 0.5 { 6.0 } else { 8.0 }).round() - 6.0 * (1.0 - (bt * 4.0).min(1.0));
        p.r(x.round() - 1.0, y - 1.0, 3.0, 3.0, INK);
        p.r(x.round(), y, 1.0, 1.0, WHITE);
    }
}

fn local_hm() -> (i32, i32) {
    // local time without a date crate: ask libc
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as libc_time_t;
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    unsafe { localtime_r(&t, &mut tm) };
    (tm.tm_hour, tm.tm_min)
}

#[allow(non_camel_case_types)]
type libc_time_t = i64;
#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const i8,
}
unsafe extern "C" {
    fn localtime_r(t: *const libc_time_t, tm: *mut Tm) -> *mut Tm;
}

fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}
