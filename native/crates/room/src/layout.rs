//! Where everything stands (room.js's layout section): the room is always W x H dots; its furniture
//! stands where it was put (dragged by hand, kept in room.json), or where it stands by default. A
//! project's desk that has no place yet gets the first free one, and keeps it.

use crate::font::tiny_w;
use crate::model::{Bot, Desk, State};
use crate::world::{Spot, fnv};
use std::collections::HashMap;

pub const WALL: i32 = 26;
pub const SEAT: i32 = 18;
pub const ADD_W: i32 = 30;
pub const LAP: i32 = 13;
pub const DOOR: i32 = 20;
pub const NAME: usize = 8;
/// CSS pixels a dot, and the margin around the room (the tiles' OUT)
pub const K: i32 = 2;
pub const MARGIN: i32 = 6;
/// the fire pit's footprint, drawn to scale from its 32 x 28 original
pub const FIRE_W: i32 = 48;
pub const FIRE_H: i32 = 42;
/// the switchboard
pub const SB_W: i32 = 10;
pub const SB_H: i32 = 14;

pub const PROJECT: [u32; 8] = [0x4a76b8, 0xe8402a, 0x2fa087, 0xf2a531, 0x9a74d6, 0x5fcf4a, 0xf06b72, 0x8fc0e8];

pub type Rect = [i32; 4];

/// a key's kind: `plant0` is a plant, `door:<machine>` a door
pub fn kind_of(key: &str) -> &str {
    if key.starts_with("door:") {
        return "door";
    }
    key.trim_end_matches(|c: char| c.is_ascii_digit())
}

pub const FIXED: [&str; 4] = ["fire", "closet", "settings", "clock"];
pub fn fixed(key: &str) -> bool {
    FIXED.contains(&kind_of(key)) || key == "door:"
}
/// a click does something there
pub const CLICK: [&str; 20] = [
    "radio", "pumpkin", "matrix", "doghouse", "add", "closet", "chess", "pong", "printer", "pair", "board", "coffee", "cooler", "window", "armchair", "settings",
    "clock", "plant", "manager", "door",
];
pub fn name_of(kind: &str) -> &str {
    match kind {
        "meet" => "meeting table",
        "pong" => "ping-pong table",
        "chess" => "chess table",
        "coffee" => "coffee machine",
        "cooler" => "water cooler",
        "armchair" => "pouf",
        "board" => "easel",
        "add" => "project cabinet",
        "pair" => "pair room",
        "settings" => "switchboard",
        "manager" => "reception",
        "doghouse" => "dog house",
        k => k,
    }
}

pub const ON_WALL: [&str; 5] = ["closet", "window", "clock", "settings", "door"];
pub fn on_wall(key: &str) -> bool {
    ON_WALL.contains(&kind_of(key))
}
pub fn wall_y(kind: &str) -> i32 {
    match kind {
        "closet" => 2,
        "window" => 4,
        "clock" | "settings" => 5,
        _ => WALL - 20,
    }
}

/// a project's colour, by its unit
pub fn project_color(unit: &str) -> pix::Color {
    pix::Color::rgb(PROJECT[(fnv(unit) % PROJECT.len() as u32) as usize])
}

/// a desk's directory as a name: its last part, or "home"
pub fn project_label(cwd: &str) -> String {
    let t = cwd.trim_end_matches('/');
    let n: Vec<&str> = t.split('/').collect();
    if cwd == "~" || cwd == "/root" || (n.len() == 3 && (n[1] == "Users" || n[1] == "home")) {
        return "home".into();
    }
    match n.last() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => cwd.to_string(),
    }
}

/// a desk's name: its project's, cut to eight letters like a name in a 16-bit game (its machine is its
/// paint, the whole path is on its screen under the pointer)
pub fn desk_label(d: &Desk) -> String {
    let (s, n) = if d.name.is_empty() { (project_label(&d.dir), NAME) } else { (d.name.to_uppercase(), 14) };
    let mut out: String = s.chars().take(n).collect();
    // (.)[-_. ]+$ -> $1
    while out.chars().count() > 1 && out.ends_with(['-', '_', '.', ' ']) {
        out.pop();
    }
    out
}

/// what each piece covers, from where it stands (a desk: its width `u`)
pub fn foot(key: &str, at: [i32; 2], u: Option<i32>) -> Rect {
    let [x, y] = at;
    if let Some(w) = u {
        return [x - 2, y - 35, x + w + 2, y + 2];
    }
    match kind_of(key) {
        "add" => [x - 2, y - 16, x + ADD_W + 2, y + 2],
        "meet" => [x - 28, y - 20, x + 28, y + 4],
        "pong" => [x - 38, y - 23, x + 38, y + 8],
        "chess" => [x - 19, y - 18, x + 19, y + 2],
        "coffee" => [x, y - 22, x + 20, y + 1],
        "cooler" => [x - 5, y - 26, x + 5, y + 1],
        "printer" => [x - 7, y - 15, x + 7, y + 1],
        "armchair" => [x - 8, y - 14, x + 8, y + 1],
        "board" => [x - 12, y - 34, x + 12, y + 1], // the easel
        "window" => [x, y, x + 21, y + 18],
        "clock" => [x, y, x + 17, y + 17],
        "fire" => [x, y - FIRE_H, x + FIRE_W, y + 1],
        "closet" => [x, y, x + 22, y + 24],
        "settings" => [x, y, x + SB_W, y + SB_H],
        "manager" => [x - 2, y - 36, x + 48, y + 2],
        "pair" => [x - 13, y - 20, x + 13, y + 1],
        "booth" => [x - 9, y - 32, x + 9, y + 2],
        "door" => [x, y, x + 16, y + 20],
        "neuralyzer" => [x - 7, y - 19, x + 7, y + 2],
        "doghouse" => [x - 12, y - 21, x + 12, y + 2],
        "radio" => [x - 7, y - 21, x + 7, y + 1],
        "pumpkin" => [x - 8, y - 13, x + 8, y + 1],
        "matrix" => [x - 11, y - 25, x + 12, y + 1],
        _ => [x - 8, y - 31, x + 8, y + 1], // a plant
    }
}

pub fn meets(a: Rect, b: Rect) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// JS's Math.round
pub fn jround(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// a piece on the floor (or a desk)
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub key: String,
    pub x: i32,
    pub y: i32,
}

/// a desk as laid out: its project, its cabinet's width, its seats
#[derive(Clone, Debug)]
pub struct DeskL {
    pub key: String,
    pub d: Desk,
    pub label: String,
    /// its worktrees with bots, a laptop each
    pub trees: Vec<String>,
    pub x0: i32,
    pub w: i32,
    pub cab: i32,
    pub y: i32,
    pub color: pix::Color,
    /// (bot id, chair x, its worktree)
    pub seats: Vec<(String, i32, String)>,
    pub at: [i32; 2],
}

impl DeskL {
    pub fn unit(&self) -> &str {
        &self.d.unit
    }
}

#[derive(Clone, Debug)]
pub struct Box4 {
    pub key: String,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Clone, Debug)]
pub struct DoorL {
    pub key: String,
    pub mid: String,
    pub x0: i32,
    pub y0: i32,
}

#[derive(Clone, Debug)]
pub struct PieceR {
    pub key: String,
    pub at: [i32; 2],
    /// a desk's width
    pub u: Option<i32>,
    pub r: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct Floor {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// the room laid out for this frame (room.js's `L`)
#[derive(Clone, Debug)]
pub struct Layout {
    pub w: i32,
    pub h: i32,
    pub desks: Vec<DeskL>,
    pub add: Option<Box4>,
    pub floor_y: i32,
    pub board: Option<Piece>,
    pub clock: Option<Piece>,
    pub windows: Vec<Box4>,
    pub fire: Option<Box4>,
    pub closet: Option<Box4>,
    pub settings: Option<Box4>,
    pub managers: Vec<Piece>,
    pub meet: Option<Piece>,
    pub pong: Option<Piece>,
    pub chess: Option<Piece>,
    pub pairs: Vec<Piece>,
    pub booths: Vec<Piece>,
    pub neuralyzers: Vec<Piece>,
    pub doghouses: Vec<Piece>,
    pub radios: Vec<Piece>,
    pub pumpkins: Vec<Piece>,
    /// the operator's consoles (the matrix)
    pub consoles: Vec<Piece>,
    pub cooler: Option<Piece>,
    pub plants: Vec<Piece>,
    pub printer: Option<Piece>,
    pub chairs: Vec<Piece>,
    pub coffee_at: Option<Piece>,
    pub coffee: Option<Spot>,
    pub floor: Floor,
    /// pieces given a place this time (to be saved)
    pub placed: HashMap<String, [i32; 2]>,
    pub doors: Vec<DoorL>,
    /// what can be picked up and moved, front to back
    pub pieces: Vec<PieceR>,
}

impl Layout {
    pub fn piece_of(&self, key: &str) -> Option<&PieceR> {
        self.pieces.iter().find(|p| p.key == key)
    }
    pub fn cooler_spot(&self, side: i32) -> Option<Spot> {
        self.cooler.as_ref().map(|c| Spot::on((c.x + if side != 0 { 10 } else { -8 }) as f64, (c.y + 1) as f64, &c.key))
    }
    pub fn pong_spot(&self, side: i32) -> Option<Spot> {
        self.pong.as_ref().map(|c| Spot::on((c.x + if side != 0 { 22 } else { -22 }) as f64, (c.y + 1) as f64, &c.key))
    }
    pub fn chess_seat(&self, side: i32) -> Option<Spot> {
        self.chess.as_ref().map(|c| Spot::on((c.x + if side != 0 { 13 } else { -13 }) as f64, c.y as f64, &c.key))
    }
    /// a seat at the meeting table, and which way it faces
    pub fn meet_seat(&self, i: usize) -> Option<(Spot, f64)> {
        let m = self.meet.as_ref()?;
        let s = [(-23, 0, 1), (23, 0, -1), (-8, -5, 1), (8, -5, -1)].get(i)?;
        Some((Spot::on((m.x + s.0) as f64, (m.y + s.1) as f64, &m.key), s.2 as f64))
    }
    /// a bot's chair at its desk
    pub fn seat(&self, id: &str) -> Option<Spot> {
        for d in &self.desks {
            for (i, x, _) in &d.seats {
                if i == id {
                    return Some(Spot::on(*x as f64, d.y as f64, &d.key));
                }
            }
        }
        None
    }
    /// clear floor to stand on: off every piece on the floor (with a margin)
    pub fn free(&self, x: f64, y: f64) -> bool {
        !self.pieces.iter().any(|p| {
            !on_wall(&p.key)
                && x >= (p.r[0] - 6) as f64
                && x <= (p.r[2] + 6) as f64
                && y >= p.r[1] as f64
                && y <= (p.r[3] + 4) as f64
        })
    }
    pub fn door_x(&self, mid: &str, machines_index: usize) -> f64 {
        match self.doors.iter().find(|d| d.mid == mid) {
            Some(d) => (d.x0 + 8) as f64,
            None => (10 + machines_index as i32 * DOOR) as f64,
        }
    }
}

/// the room's size and its default places, which follow it
#[derive(Clone, Debug)]
pub struct Size {
    pub w: i32,
    pub h: i32,
}

impl Size {
    pub fn floor_y(&self) -> i32 {
        self.h - 6
    }
    /// the default places, as the room is now
    pub fn defaults(&self, key: &str) -> Option<[i32; 2]> {
        let (w, fy) = (self.w, self.floor_y());
        Some(match key {
            "meet" => [w / 2, WALL + 34],
            "pong" => [60, 62],
            "chess" => [70, fy - 32],
            "coffee" => [3, fy],
            "cooler" => [36, fy - 1],
            "printer" => [w / 2 + 4, fy],
            "armchair" => [w - 32, fy],
            "plant0" => [w - 8, fy],
            "plant1" => [w - 12, 64],
            "board" => [w / 2 + 46, WALL + 42],
            "window" => [w - 40, 4],
            "clock" => [w - 15, 5],
            "fire" => [w - 92, fy],
            "closet" => [w - 72, 2],
            "settings0" => [w - 92, 5],
            _ => return None,
        })
    }
    /// the default places for desks: columns of them, as many as the room is wide
    pub fn slots(&self) -> Vec<[i32; 2]> {
        let n = ((self.w - 10) / 124).max(1);
        [104, 150, 196, 242, 288, 334].iter().flat_map(|&y| (0..n).map(move |i| [10 + i * 124, y])).collect()
    }
    /// keep a piece in the room: on the floor, or along the wall for what hangs there
    pub fn clamp_at(&self, key: &str, at: [i32; 2], u: Option<i32>) -> [i32; 2] {
        let r = foot(key, at, u);
        let [x, y] = at;
        if on_wall(key) {
            return [(2 - (r[0] - x)).max((self.w - 2 - (r[2] - x)).min(x)), wall_y(kind_of(key))];
        }
        [(2 - (r[0] - x)).max((self.w - 2 - (r[2] - x)).min(x)), (WALL + 2 - (r[1] - y)).max((self.h - 1 - (r[3] - y)).min(y))]
    }
}

/// what the layout keeps between frames: a desk's bots in the order they came (a new one takes the chair at
/// the far end of the row, the others keep theirs), and pieces just put down (they stay there while the
/// server hears of it)
#[derive(Default)]
pub struct Keep {
    pub seat_rows: HashMap<String, Vec<String>>,
    pub dropped: HashMap<String, ([i32; 2], f64)>,
}

impl Keep {
    /// a desk's sitters, in their order; a killed one (in pieces, in the booth, gone) gives up its chair at once
    /// but keeps its place in the row: put back together, it sits where it sat
    pub fn sitters<'a>(&mut self, bots: &'a [Bot], unit: &str, gone: &dyn Fn(&str) -> bool) -> Vec<&'a Bot> {
        let here: Vec<&Bot> = bots.iter().filter(|b| b.unit == unit).collect();
        let row = self.seat_rows.entry(unit.to_string()).or_default();
        row.retain(|id| here.iter().any(|b| &b.id == id));
        for b in &here {
            if !row.contains(&b.id) {
                row.push(b.id.clone());
            }
        }
        row.iter().filter(|id| !gone(id)).filter_map(|id| here.iter().find(|b| &b.id == id).copied()).collect()
    }
}

/// what is being dragged: its key, where it was, a desk's width, where it is now; `out`: the pointer has been off the
/// fire since it was picked up (one picked up over the pit burns only once it has left it and come back)
#[derive(Clone, Debug)]
pub struct Moving {
    pub key: String,
    pub from: [i32; 2],
    pub u: Option<i32>,
    pub at: [i32; 2],
    pub out: bool,
}

pub fn layout(s: &State, desks: &[Desk], size: &Size, keep: &mut Keep, moving: Option<&Moving>, gone: &dyn Fn(&str) -> bool, now: f64) -> Layout {
    let (w, h) = (size.w, size.h);
    let mut placed = HashMap::new();
    struct Unit {
        key: String,
        d: Desk,
        label: String,
        trees: Vec<String>,
        cab: i32,
        w: i32,
        at: Option<[i32; 2]>,
    }
    let mut units: Vec<Unit> = desks
        .iter()
        .map(|d| {
            let mine = keep.sitters(&s.bots, &d.unit, gone);
            let n = mine.len() as i32;
            let label = desk_label(d);
            let mut trees: Vec<String> = Vec::new();
            for b in &mine {
                if !b.tree.is_empty() && !trees.contains(&b.tree) {
                    trees.push(b.tree.clone());
                }
            }
            let cab = (tiny_w(&label) + 6).max(22).max(if trees.is_empty() { 0 } else { 26 + trees.len() as i32 * LAP });
            // a chair a bot; none, and no desktop either, without one
            Unit { key: format!("u:{}", d.unit), d: d.clone(), label, trees, cab, w: if n > 0 { cab + 4 + n * SEAT } else { cab }, at: None }
        })
        .collect();
    let where_ = |key: &str, u: Option<i32>, keep: &Keep| -> Option<[i32; 2]> {
        if let Some(m) = moving
            && m.key == key
        {
            return Some(m.at);
        }
        if let Some((at, t)) = keep.dropped.get(key)
            && now - t < 5000.0
        {
            return Some(*at);
        }
        if let Some(at) = s.layout.get(key) {
            return Some(size.clamp_at(key, *at, u)); // the window may be shorter than when it was put there
        }
        if u.is_none() { size.defaults(key) } else { None }
    };
    let keys: Vec<String> = if s.furniture.is_empty() {
        ["meet", "pong", "chess", "coffee", "cooler", "printer", "armchair", "plant0", "plant1", "board", "window", "clock", "fire", "closet", "settings0"]
            .iter()
            .map(|k| k.to_string())
            .collect()
    } else {
        s.furniture.iter().filter(|k| kind_of(k) != "mat").cloned().collect()
    };
    let mut at0: HashMap<String, [i32; 2]> = HashMap::new();
    let mut taken: Vec<Rect> = Vec::new();
    for k in &keys {
        if let Some(at) = where_(k, None, keep) {
            at0.insert(k.clone(), at);
            if !on_wall(k) {
                taken.push(foot(k, at, None));
            }
        }
    }
    for u in units.iter_mut() {
        u.at = where_(&u.key, Some(u.w), keep);
        if let Some(at) = u.at {
            taken.push(foot(&u.key, at, Some(u.w)));
        }
    }
    // a free place, for a unit (with room to grow by a chair) or a piece without one
    let free = |key: &str, uw: Option<i32>, taken: &mut Vec<Rect>| -> [i32; 2] {
        let wide = uw.map(|w| w + SEAT);
        let mut cands = size.slots();
        for j in 0..40 {
            for i in 0..30 {
                cands.push([6 + i * 8, 70 + j * 6]);
            }
        }
        for c in cands {
            let r = foot(key, c, wide);
            if r[0] >= 2 && r[2] <= w - 2 && r[1] >= WALL + 2 && r[3] <= h && !taken.iter().any(|t| meets(*t, r)) {
                taken.push(r);
                return c;
            }
        }
        size.slots()[0]
    };
    for u in units.iter_mut() {
        if u.at.is_none() {
            let at = free(&u.key, Some(u.w), &mut taken);
            u.at = Some(at);
            placed.insert(u.key.clone(), at);
        }
    }
    // along the wall: the doors (one a machine, in their order unless slid elsewhere) and what hangs there;
    // one without a place yet gets the stretch of wall nearest its default that nothing else covers
    let door_keys: Vec<String> = s.machines.iter().map(|m| format!("door:{}", m.id)).collect();
    let mut the_wall: Vec<Rect> = Vec::new();
    let wall_spot = |k: &str, x0: i32, the_wall: &Vec<Rect>| -> [i32; 2] {
        let y = wall_y(kind_of(k));
        let fits = |x: i32| {
            let r = foot(k, [x, y], None);
            r[0] >= 2 && r[2] <= w - 2 && !the_wall.iter().any(|t| meets(*t, r))
        };
        let mut d = 0;
        while d < w {
            for x in [x0 + d, x0 - d] {
                if fits(x) {
                    return [x, y];
                }
            }
            d += 2;
        }
        [x0, y]
    };
    for k in &door_keys {
        if let Some(at) = where_(k, None, keep) {
            at0.insert(k.clone(), at);
            the_wall.push(foot(k, at, None));
        }
    }
    for k in &keys {
        if let Some(at) = at0.get(k)
            && on_wall(k)
        {
            the_wall.push(foot(k, *at, None));
        }
    }
    for (i, k) in door_keys.iter().enumerate() {
        if !at0.contains_key(k) {
            let at = wall_spot(k, 2 + i as i32 * DOOR, &the_wall);
            at0.insert(k.clone(), at);
            the_wall.push(foot(k, at, None));
            placed.insert(k.clone(), at);
        }
    }
    for k in &keys {
        if !at0.contains_key(k) {
            let at = if on_wall(k) { wall_spot(k, 2 + door_keys.len() as i32 * DOOR, &the_wall) } else { free(k, None, &mut taken) };
            if on_wall(k) {
                the_wall.push(foot(k, at, None));
            }
            at0.insert(k.clone(), at);
            placed.insert(k.clone(), at);
        }
    }
    // the pieces by kind, in the order of their keys; most kinds are one at most, some can be many
    let mut by: HashMap<&str, Vec<Piece>> = HashMap::new();
    for k in &keys {
        let [x, y] = at0[k];
        by.entry(kind_of(k)).or_default().push(Piece { key: k.clone(), x, y });
    }
    let one = |kind: &str| by.get(kind).and_then(|v| v.first().cloned());
    let all = |kind: &str| by.get(kind).cloned().unwrap_or_default();
    let desks_l: Vec<DeskL> = units
        .iter()
        .map(|u| {
            let [x, y] = u.at.unwrap();
            let mine = keep.sitters(&s.bots, &u.d.unit, gone);
            let seat = |k: usize| x + u.cab + 4 + SEAT / 2 + k as i32 * SEAT;
            DeskL {
                key: u.key.clone(),
                d: u.d.clone(),
                label: u.label.clone(),
                trees: u.trees.clone(),
                x0: x,
                w: u.w,
                cab: u.cab,
                y,
                color: project_color(&u.d.unit),
                seats: mine.iter().enumerate().map(|(k, b)| (b.id.clone(), seat(k), b.tree.clone())).collect(),
                at: [x, y],
            }
        })
        .collect();
    let bx = |p: &Piece, w: i32, h: i32| Box4 { key: p.key.clone(), x0: p.x, y0: p.y, x1: p.x + w, y1: p.y + h };
    let coffee_at = one("coffee");
    let mut pieces: Vec<PieceR> = desks_l.iter().map(|d| PieceR { key: d.key.clone(), at: d.at, u: Some(d.w), r: foot(&d.key, d.at, Some(d.w)) }).collect();
    for k in keys.iter().chain(door_keys.iter()) {
        let at = at0[k];
        pieces.push(PieceR { key: k.clone(), at, u: None, r: foot(k, at, None) });
    }
    pieces.sort_by(|a, b| b.r[3].cmp(&a.r[3]));
    Layout {
        w,
        h,
        add: one("add").map(|a| Box4 { key: a.key.clone(), x0: a.x, y0: a.y - 15, x1: a.x + ADD_W, y1: a.y }),
        floor_y: size.floor_y(),
        board: one("board"),
        clock: one("clock"),
        windows: all("window").iter().map(|p| bx(p, 21, 18)).collect(),
        fire: one("fire").map(|f| Box4 { key: f.key.clone(), x0: f.x, y0: f.y - FIRE_H, x1: f.x + FIRE_W, y1: f.y }),
        closet: one("closet").map(|c| bx(&c, 22, 24)),
        settings: one("settings").map(|c| bx(&c, SB_W, SB_H)),
        managers: all("manager"),
        meet: one("meet"),
        pong: one("pong"),
        chess: one("chess"),
        pairs: all("pair"),
        booths: all("booth"),
        neuralyzers: all("neuralyzer"),
        doghouses: all("doghouse"),
        radios: all("radio"),
        pumpkins: all("pumpkin"),
        consoles: all("matrix"),
        cooler: one("cooler"),
        plants: all("plant"),
        printer: one("printer"),
        chairs: all("armchair"),
        coffee: coffee_at.as_ref().map(|c| Spot::on((c.x + 27) as f64, (c.y - 10) as f64, &c.key)),
        coffee_at,
        // where a figure may stand: right up to the bottom edge, its body inside the side walls
        floor: Floor { x0: 11., y0: (WALL + 8) as f64, x1: (w - 11) as f64, y1: (h - 2) as f64 },
        placed,
        doors: s
            .machines
            .iter()
            .enumerate()
            .map(|(i, m)| DoorL { key: door_keys[i].clone(), mid: m.id.clone(), x0: at0[&door_keys[i]][0], y0: at0[&door_keys[i]][1] })
            .collect(),
        desks: desks_l,
        pieces,
    }
}

/// a clear spot on the floor for a figure that has no chair when the room opens (spread by its id, the same
/// each time)
pub fn free_at(l: &Layout, id: &str) -> Spot {
    let mut h = fnv(id);
    let (fx0, fy0, fx1, fy1) = (l.floor.x0 as u32, l.floor.y0 as u32, l.floor.x1 as u32, l.floor.y1 as u32);
    for _ in 0..60 {
        h = h.wrapping_mul(1103515245).wrapping_add(12345);
        let x = fx0 + h % (fx1 - fx0);
        let y = fy0 + 20 + (h >> 11) % (fy1.saturating_sub(fy0 + 20)).max(1);
        if l.free(x as f64, y as f64) {
            return Spot::at(x as f64, y as f64);
        }
    }
    Spot::at((l.w / 2) as f64, (l.h / 2) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> State {
        State::from_json(include_str!("../../model/tests/fixtures/demo-state.json")).unwrap()
    }

    #[test]
    fn kinds_and_labels() {
        assert_eq!(kind_of("plant12"), "plant");
        assert_eq!(kind_of("door:lab"), "door");
        assert!(fixed("door:") && !fixed("door:lab") && fixed("fire"));
        let d = Desk { dir: "/Users/you/projects/burrow-gpui".into(), ..Default::default() };
        assert_eq!(desk_label(&d), "burrow-g");
        assert_eq!(project_label("/Users/you"), "home");
        let named = Desk { name: "my thing".into(), ..Default::default() };
        assert_eq!(desk_label(&named), "MY THING");
    }

    #[test]
    fn the_demo_room_lays_out_without_overlaps() {
        let s = demo();
        let size = Size { w: 256, h: 400 };
        let mut keep = Keep::default();
        let l = layout(&s, &s.desks, &size, &mut keep, None, &|_| false, 0.0);
        assert_eq!(l.desks.len(), s.desks.len());
        assert_eq!(l.doors.len(), s.machines.len());
        // every bot of a desk has a chair there
        for b in &s.bots {
            if s.desks.iter().any(|d| d.unit == b.unit) {
                assert!(l.seat(&b.id).is_some(), "{} has no chair", b.id);
            }
        }
        // desks given places don't overlap each other
        for (i, a) in l.desks.iter().enumerate() {
            for b in &l.desks[i + 1..] {
                assert!(!meets(foot(&a.key, a.at, Some(a.w)), foot(&b.key, b.at, Some(b.w))), "{} and {}", a.key, b.key);
            }
        }
        // the same room twice is laid out the same
        let l2 = layout(&s, &s.desks, &size, &mut keep, None, &|_| false, 0.0);
        assert_eq!(l.desks.iter().map(|d| d.at).collect::<Vec<_>>(), l2.desks.iter().map(|d| d.at).collect::<Vec<_>>());
    }

    #[test]
    fn clamp_keeps_pieces_in() {
        let size = Size { w: 256, h: 400 };
        assert_eq!(size.clamp_at("plant0", [-50, 9999], None), [10, 398]);
        assert_eq!(size.clamp_at("window0", [100, 300], None)[1], 4);
    }
}
