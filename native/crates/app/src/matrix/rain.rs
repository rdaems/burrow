//! The rain (matrix.js's lanes): every session a lane of falling code, made of what it said and did. How it falls is
//! how it is: fast and bright, it works; frozen and pulsing white, it needs you; a slow dim drip, it is idle; nearly
//! dark, its process is off or it is over; nothing falls on a machine out of reach. Lanes stand together by project (a
//! desk), a gap between projects; what is left over is a faint rain of nobody's. A message between two sessions
//! streaks across the top. No window here: a step gives the glyphs to paint, at their places in the rain's box.

use super::words::{lost, machine_name, project_of, state_of, St};
use burrow_model::{Bot, State};
use std::collections::HashMap;

/// the glyphs' size and a row's height (px)
pub const FONT: f32 = 14.0;
pub const CH: f32 = 16.0;
/// columns a lane, at most (as many as fit)
pub const LANE: usize = 8;
/// the bottom of the box: the projects' names
pub const NAMES: f32 = 26.0;

/// how a state rains: drops a column a second, their speed (rows a second), how fast the trail fades, how bright
struct Rule {
    spawn: f64,
    speed: [f64; 2],
    fade: f64,
    bright: f64,
}

fn rule(st: Option<St>) -> Rule {
    let r = |spawn, speed, fade, bright| Rule { spawn, speed, fade, bright };
    match st {
        Some(St::Work) => r(1.1, [14., 24.], 1.1, 1.),
        Some(St::Need) => r(0., [0., 0.], 0., 1.),
        Some(St::Idle) => r(0.1, [4., 7.], 0.7, 0.55),
        Some(St::Off) => r(0.04, [3., 5.], 0.7, 0.32),
        Some(St::Over) => r(0.025, [3., 4.], 0.8, 0.24),
        Some(St::Lost) => r(0., [0., 0.], 0.25, 0.2),
        None => r(0.12, [6., 13.], 1.2, 0.2), // nobody's
    }
}

/// half-width katakana and the digits
pub fn kana_at(i: usize) -> char {
    if i < 56 { char::from_u32(0xff66 + i as u32).unwrap_or('0') } else { (b'0' + (i - 56) as u8 % 10) as char }
}
const KANA: usize = 66;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cell {
    /// its glyph ('\0': none)
    pub c: char,
    /// how bright
    pub a: f64,
    /// how much a drop's white head it still is
    pub h: f64,
}

#[derive(Clone, Copy, Debug)]
struct Drop {
    col: usize,
    y: f64,
    v: f64,
}

pub struct Lane {
    pub id: String,
    pub b: Bot,
    /// its first column, how many it has
    pub x: usize,
    pub w: usize,
    /// [column][row]
    pub cells: Vec<Vec<Cell>>,
    drops: Vec<Drop>,
    /// what it said and did, and where the rain is in it
    pub text: Vec<char>,
    pos: usize,
    /// the record's updatedAt the text is of, and when it was last asked for
    pub text_for: Option<i64>,
    pub asked: f64,
    /// deleted: since when (it falls apart, then goes)
    pub gone: Option<f64>,
    was: Option<St>,
}

struct Ambient {
    x: usize,
    cells: Vec<Cell>,
    drops: Vec<Drop>,
}

/// a project's lanes: its columns, its name at the bottom
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub unit: String,
    pub x0: usize,
    pub x1: usize,
    pub label: String,
    pub lost: bool,
}

/// a message on its way
struct Streak {
    from: String,
    to: String,
    t0: f64,
    hit: bool,
}

/// a glyph to paint: where (px in the box, its top), its colour (rgb) and how opaque, bold (a name)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub x: f32,
    pub y: f32,
    pub c: char,
    pub rgb: u32,
    pub a: f32,
    pub bold: bool,
}

pub struct Rain {
    /// a column's width (px), how many columns and rows there are
    pub cw: f32,
    pub cols: usize,
    pub rows: usize,
    /// in the order they came
    pub lanes: Vec<Lane>,
    ambient: Vec<Ambient>,
    pub groups: Vec<Group>,
    streaks: Vec<Streak>,
    /// id → when it was jacked into (its lane flashes white)
    flashes: HashMap<String, f64>,
    /// the lane under the pointer
    pub hovered: Option<String>,
    last: Option<f64>,
    rng: u64,
}

fn column(rows: usize) -> Vec<Cell> {
    vec![Cell::default(); rows]
}

/// a column to `rows` rows: the bottom kept
fn fit(c: &mut Vec<Cell>, rows: usize) {
    if c.len() > rows {
        c.drain(..c.len() - rows);
    } else if c.len() < rows {
        let mut top = column(rows - c.len());
        top.append(c);
        *c = top;
    }
}

/// a name down the bottom of its lane: as many letters as fit a third of the rows, fourteen at most
pub fn name_of(l: &Lane, rows: usize) -> Vec<char> {
    let n = if l.b.name.is_empty() { &l.id } else { &l.b.name };
    n.chars().take(14.min(rows / 3)).collect()
}

/// a trail's drops fall (writing a glyph in every row they pass), drops out of sight go, every glyph fades
fn fall(cells: &mut [Vec<Cell>], drops: &mut Vec<Drop>, rows: usize, r: &Rule, dt: f64, src: &mut dyn FnMut() -> char) {
    for d in drops.iter_mut() {
        let before = d.y.floor() as i64;
        d.y += d.v * dt;
        let mut row = (before + 1).max(0);
        while row <= d.y.floor() as i64 && (row as usize) < rows {
            if let Some(cell) = cells.get_mut(d.col).and_then(|c| c.get_mut(row as usize)) {
                *cell = Cell { c: src(), a: r.bright, h: 1. };
            }
            row += 1;
        }
    }
    drops.retain(|d| d.y <= rows as f64 + 2.);
    let (k, kh) = ((-dt * r.fade).exp(), (-dt * 9.).exp());
    for col in cells.iter_mut() {
        for cell in col.iter_mut() {
            if cell.a != 0. {
                cell.a *= k;
                if cell.a < 0.015 {
                    cell.a = 0.;
                    cell.c = '\0';
                }
            }
            cell.h *= kh;
        }
    }
}

impl Rain {
    pub fn new(seed: u64) -> Rain {
        Rain {
            cw: 9.,
            cols: 1,
            rows: 1,
            lanes: vec![],
            ambient: vec![],
            groups: vec![],
            streaks: vec![],
            flashes: HashMap::new(),
            hovered: None,
            last: None,
            rng: seed | 1,
        }
    }

    /// 0 ≤ x < 1 (a xorshift: the rain's own dice)
    fn random(&mut self) -> f64 {
        let mut s = self.rng;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.rng = s;
        (s >> 11) as f64 / (1u64 << 53) as f64
    }
    fn kana(&mut self) -> char {
        kana_at((self.random() * KANA as f64) as usize)
    }

    pub fn lane(&self, id: &str) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.id == id)
    }

    /// the box is `w` x `h` px, a column `cw` wide
    pub fn measure(&mut self, w: f32, h: f32, cw: f32, st: &State, now: f64) {
        self.cw = cw;
        self.cols = ((w / cw).floor() as usize).max(1);
        self.rows = (((h - NAMES) / CH).floor().max(0.) as usize).max(1);
        let rows = self.rows;
        for l in &mut self.lanes {
            for c in &mut l.cells {
                fit(c, rows);
            }
        }
        self.arrange(st, now);
    }

    /// where every lane stands: by project (the desks' order), a gap between projects; what is left over is
    /// nobody's. Too many for the width: the gaps shrink, then a lane is one column
    pub fn arrange(&mut self, st: &State, now: f64) {
        let bots = &st.bots;
        let mut units: Vec<&str> = vec![];
        for u in st.desks.iter().map(|d| d.unit.as_str()).filter(|u| bots.iter().any(|b| b.unit == *u)).chain(bots.iter().map(|b| b.unit.as_str())) {
            if !units.contains(&u) {
                units.push(u);
            }
        }
        let (n, g) = (bots.len() as i64, units.len() as i64);
        let (mut gap, mut ggap) = (2i64, 5i64);
        let cols = self.cols as i64;
        let room = |gap: i64, ggap: i64| cols - 4 - (n - g) * gap - (g - 1).max(0) * ggap;
        if room(gap, ggap) < n * 3 {
            ggap = 3;
            gap = 1;
        }
        if room(gap, ggap) < n * 2 {
            gap = 0;
            ggap = 1;
        }
        let lane = (room(gap, ggap) as f64 / n.max(1) as f64).floor().clamp(1., LANE as f64) as i64;
        let need = n * lane + (n - g) * gap + (g - 1).max(0) * ggap;
        let used = cols.min(need);
        let mut x = ((cols - used) / 2).max(0);
        let rows = self.rows;
        self.groups.clear();
        for u in units {
            let bs: Vec<&Bot> = bots.iter().filter(|b| b.unit == u).collect();
            let x0 = x;
            for b in &bs {
                let i = match self.lanes.iter().position(|l| l.id == b.id) {
                    Some(i) => i,
                    None => {
                        self.lanes.push(Lane {
                            id: b.id.clone(),
                            b: (*b).clone(),
                            x: 0,
                            w: 0,
                            cells: vec![],
                            drops: vec![],
                            text: vec![],
                            pos: 0,
                            text_for: None,
                            asked: f64::NEG_INFINITY,
                            gone: None,
                            was: None,
                        });
                        let i = self.lanes.len() - 1;
                        self.lanes[i].w = lane as usize;
                        self.burst(i);
                        i
                    }
                };
                let l = &mut self.lanes[i];
                l.b = (*b).clone();
                l.x = x as usize;
                l.w = lane as usize;
                l.gone = None;
                l.cells.resize_with(lane as usize, || column(rows));
                for c in &mut l.cells {
                    fit(c, rows);
                }
                x += lane + gap;
            }
            let b0 = bs[0];
            let label = format!("{}{}", project_of(b0, st), if b0.m.is_empty() { String::new() } else { format!(" @{}", machine_name(&b0.m, st)) });
            self.groups.push(Group { unit: u.to_string(), x0: x0 as usize, x1: (x - gap).max(0) as usize, label, lost: lost(b0, st) });
            x += ggap - gap;
        }
        // the gone: they fall apart where they stood, then go
        for l in &mut self.lanes {
            if l.gone.is_none() && !bots.iter().any(|b| b.id == l.id) {
                l.gone = Some(now);
            }
        }
        let mut taken = vec![false; self.cols + 1];
        let mut take = |c: i64| {
            if c >= 0 && (c as usize) < taken.len() {
                taken[c as usize] = true;
            }
        };
        for l in self.lanes.iter().filter(|l| l.gone.is_none()) {
            for i in 0..l.w {
                take((l.x + i) as i64);
            }
        }
        for g in &self.groups {
            for c in g.x0 as i64 - 1..=g.x1 as i64 {
                take(c);
            }
        }
        let mut keep: HashMap<usize, Ambient> = self.ambient.drain(..).map(|a| (a.x, a)).collect();
        for c in 0..self.cols {
            if taken[c] {
                continue;
            }
            let a = match keep.remove(&c) {
                Some(mut a) => {
                    fit(&mut a.cells, rows);
                    a
                }
                None => Ambient { x: c, cells: column(rows), drops: vec![] },
            };
            self.ambient.push(a);
        }
    }

    /// a lane comes in, or is jacked into: a drop down each of its columns at once
    fn burst(&mut self, i: usize) {
        let w = if self.lanes[i].w == 0 { LANE } else { self.lanes[i].w };
        for col in 0..w {
            let (y, v) = (-self.random() * 4., 26. + self.random() * 10.);
            self.lanes[i].drops.push(Drop { col, y, v });
        }
    }

    /// jacked into: its lane flashes white, a burst down it
    pub fn jacked(&mut self, id: &str, now: f64) {
        self.flashes.insert(id.to_string(), now);
        if let Some(i) = self.lanes.iter().position(|l| l.id == id) {
            self.burst(i);
        }
    }

    /// you typed into its session: now and then a drop
    pub fn typed(&mut self, id: &str) {
        if self.random() < 0.3
            && let Some(l) = self.lanes.iter_mut().find(|l| l.id == id)
        {
            l.drops.push(Drop { col: 0, y: -1., v: 30. });
        }
    }

    /// a message from one session to another
    pub fn talk(&mut self, from: &str, to: &str, now: f64) {
        self.streaks.push(Streak { from: from.into(), to: to.into(), t0: now, hit: false });
    }

    /// the lane under the pointer, `px` from the box's left
    pub fn lane_at(&self, px: f32) -> Option<&Lane> {
        let c = (px / self.cw).floor();
        if c < 0. {
            return None;
        }
        let c = c as usize;
        self.lanes.iter().find(|l| l.gone.is_none() && c >= l.x && c < l.x + l.w)
    }

    /// the sessions whose text is older than their record (asked again after twelve seconds at the soonest)
    pub fn want_texts(&mut self, now: f64) -> Vec<String> {
        let mut ask = vec![];
        for l in self.lanes.iter_mut().filter(|l| l.gone.is_none()) {
            if l.text_for != Some(l.b.updated) && now - l.asked > 12000. {
                ask.push(l.id.clone());
                l.asked = now;
            }
        }
        ask
    }

    /// what they said and did came: the rain goes on from near its end
    pub fn got_texts(&mut self, texts: &HashMap<String, String>) {
        for (id, text) in texts {
            let Some(l) = self.lanes.iter_mut().find(|l| l.id == *id) else { continue };
            if text.is_empty() {
                continue;
            }
            let t: Vec<char> = text.chars().collect();
            if l.text != t {
                l.pos = t.len().saturating_sub(600);
                l.text = t;
            }
            l.text_for = Some(l.b.updated);
        }
    }

    /// a frame: everything falls for the time since the last (a tenth of a second at most; a quarter as fast when
    /// the system asks for less motion), and what is to be painted
    pub fn step(&mut self, st: &State, now: f64, still: bool) -> Vec<Glyph> {
        let dt = ((now - self.last.unwrap_or(now)) / 1000.).clamp(0., 0.1) * if still { 0.25 } else { 1. };
        self.last = Some(now);
        let (rows, cw) = (self.rows, self.cw);
        let mut out = Vec::new();
        // nobody's columns: faint, so the screen is never quite still
        let r = rule(None);
        let mut ambient = std::mem::take(&mut self.ambient);
        for a in &mut ambient {
            if self.random() < r.spawn * dt {
                let v = r.speed[0] + self.random() * (r.speed[1] - r.speed[0]);
                a.drops.push(Drop { col: 0, y: -1., v });
            }
            let mut cells = [std::mem::take(&mut a.cells)];
            fall(&mut cells, &mut a.drops, rows, &r, dt, &mut || self.kana());
            [a.cells] = cells;
            paint(&mut out, a.x, &a.cells, 0., false, rows, cw);
        }
        self.ambient = ambient;
        let hov = self.hovered.clone();
        let mut gone = vec![];
        let mut lanes = std::mem::take(&mut self.lanes);
        for l in &mut lanes {
            let s = if l.gone.is_some() { St::Over } else { state_of(&l.b, st) };
            let r = rule(Some(s));
            if let Some(g) = l.gone {
                // deleted: its glyphs drop out, and it goes
                if (now - g) / 1200. > 1. {
                    gone.push(l.id.clone());
                    continue;
                }
                for col in &mut l.cells {
                    for cell in col.iter_mut() {
                        if self.random() < dt * 3. {
                            *cell = Cell { h: cell.h, ..Cell::default() };
                        }
                    }
                }
            } else if r.spawn > 0. {
                for col in 0..l.w {
                    if self.random() < r.spawn * dt {
                        let v = r.speed[0] + self.random() * (r.speed[1] - r.speed[0]);
                        l.drops.push(Drop { col, y: -1., v });
                    }
                }
            }
            if s == St::Need {
                // frozen: a page of what it last said stands still in its lane, white
                if l.was != Some(St::Need) {
                    l.drops.clear();
                    for k in 0..l.cells.len() {
                        for j in 0..l.cells[k].len() {
                            let c = self.next(l);
                            let a = 0.6 + self.random() * 0.4;
                            l.cells[k][j] = Cell { c, a, h: 0. };
                        }
                    }
                }
            } else {
                let Lane { cells, drops, text, pos, .. } = l;
                let rng = &mut self.rng;
                fall(cells, drops, rows, &r, dt, &mut || next_of(text, pos, rng));
            }
            l.was = Some(s);
            let pulse = if s == St::Need { 0.5 + 0.5 * (now / 260.).sin() } else { 0. };
            let fl = self.flashes.get(&l.id).map(|t| (1. - (now - t) / 500.).max(0.)).unwrap_or(0.);
            let lit = if hov.as_deref() == Some(&l.id) { 0.35 } else { 0. } + fl;
            let name = name_of(l, rows);
            let (nc, nz) = ((l.w.max(1) - 1) / 2, rows.saturating_sub(name.len() + 1));
            for (i, col) in l.cells.iter().enumerate() {
                paint(&mut out, l.x + i, col, pulse.max(lit), s == St::Need, if i == nc { nz } else { rows }, cw);
            }
            // its name, down the bottom of its lane, as the rain writes
            let a = if hov.as_deref() == Some(&l.id) {
                1.
            } else {
                match s {
                    St::Need => 0.6 + 0.4 * (now / 260.).sin(),
                    St::Work => 0.9,
                    St::Idle => 0.6,
                    _ => 0.4,
                }
            };
            let rgb = match s {
                St::Need => 0xe9ffee,
                St::Lost => 0xff5f56,
                _ => 0x00ff41,
            };
            let (x, y0) = ((l.x + nc) as f32 * cw, (rows - name.len().min(rows)) as f32 * CH);
            for (i, c) in name.iter().enumerate() {
                out.push(Glyph { x, y: y0 + i as f32 * CH, c: *c, rgb, a: a as f32, bold: true });
            }
        }
        self.lanes = lanes;
        if !gone.is_empty() {
            self.lanes.retain(|l| !gone.contains(&l.id));
            self.arrange(st, now);
        }
        // the messages on their way
        let mut streaks = std::mem::take(&mut self.streaks);
        streaks.retain_mut(|s| self.streak(s, now, &mut out));
        self.streaks = streaks;
        out
    }

    /// its next glyph: what it said and did, letter by letter; a space is a glyph of the rain
    fn next(&mut self, l: &mut Lane) -> char {
        next_of(&l.text, &mut l.pos, &mut self.rng)
    }

    /// a message: bright glyphs leap from the sender's lane to the receiver's, along the top
    fn streak(&mut self, s: &mut Streak, now: f64, out: &mut Vec<Glyph>) -> bool {
        let t = (now - s.t0) / 900.;
        let (Some(a), Some(b)) = (self.lane(&s.from).map(|l| l.x), self.lane(&s.to).map(|l| l.x)) else { return false };
        if t > 1.4 {
            return false;
        }
        let cw = self.cw as f64;
        let (x0, x1, y) = ((a as f64 + 0.5) * cw, (b as f64 + 0.5) * cw, CH as f64 * 2.);
        let (head, tail) = (t.min(1.), (t - 0.4).max(0.));
        let n = (((x1 - x0).abs() / cw).round()).max(1.);
        let hi = (head * n).floor() as i64;
        for i in (tail * n).floor() as i64..=hi {
            let k = i as f64 / n;
            let fade = 1. - (head - k) * 2.5;
            if fade <= 0. {
                continue;
            }
            let c = self.kana();
            let rgb = if i == hi { 0xe9ffee } else { 0x00ff41 };
            out.push(Glyph { x: (x0 + (x1 - x0) * k) as f32, y: (y - (k * std::f64::consts::PI).sin() * CH as f64) as f32, c, rgb, a: fade.min(1.) as f32, bold: false });
        }
        if t >= 1. && !s.hit {
            s.hit = true;
            self.jacked(&s.to.clone(), now);
        }
        true
    }
}

/// the next of `text` (a space a glyph of the rain), or the rain's own with none
fn next_of(text: &[char], pos: &mut usize, rng: &mut u64) -> char {
    let kana = |rng: &mut u64| {
        let mut s = *rng;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *rng = s;
        kana_at((((s >> 11) as f64 / (1u64 << 53) as f64) * KANA as f64) as usize)
    };
    if text.is_empty() {
        return kana(rng);
    }
    let c = text[*pos % text.len()];
    *pos += 1;
    if c.is_whitespace() { kana(rng) } else { c }
}

/// a column's glyphs: green, the head of a drop white, a lane that needs you pulsing towards white
fn paint(out: &mut Vec<Glyph>, cx: usize, col: &[Cell], white: f64, need: bool, upto: usize, cw: f32) {
    let x = cx as f32 * cw;
    for (r, cell) in col.iter().enumerate().take(upto) {
        if cell.c == '\0' || cell.a < 0.02 {
            continue;
        }
        let w = cell.h.max(if need { 0.75 + 0.25 * white } else { white * 0.6 }).min(1.);
        let a = cell.a;
        let ch = |v: f64| (v.round().clamp(0., 255.)) as u32;
        let rgb = (ch(w * 233. * a) << 16) | (ch(255. * a) << 8) | ch(((1. - w) * 65. + w * 238.) * a);
        out.push(Glyph { x, y: r as f32 * CH, c: cell.c, rgb, a: 1., bold: false });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burrow_model::{Desk, Machine};

    fn bot(id: &str, name: &str, unit: &str, f: impl Fn(&mut Bot)) -> Bot {
        let mut b = Bot { id: id.into(), name: name.into(), unit: unit.into(), cwd: unit.into(), alive: true, ..Bot::default() };
        f(&mut b);
        b
    }
    fn state() -> State {
        State {
            machines: vec![Machine { id: "".into(), name: "mac".into(), ..Default::default() }, Machine { id: "lab".into(), name: "lab".into(), ..Default::default() }],
            desks: vec![
                Desk { unit: "/p/kite".into(), dir: "/p/kite".into(), ..Default::default() },
                Desk { unit: "/p/burrow".into(), dir: "/p/burrow".into(), name: "Burrow".into(), ..Default::default() },
                Desk { unit: "lab:/p/puffin".into(), m: "lab".into(), dir: "/p/puffin".into(), ..Default::default() },
            ],
            bots: vec![
                bot("b1", "burrow", "/p/burrow", |b| b.busy = true),
                bot("k1", "kite", "/p/kite", |_| {}),
                bot("b2", "talk-ui", "/p/burrow", |b| b.waiting = true),
                bot("p1", "puffin", "lab:/p/puffin", |b| b.m = "lab".into()),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn lanes_stand_by_project_in_the_desks_order() {
        let st = state();
        let mut r = Rain::new(7);
        r.measure(1280., 700., 10., &st, 0.);
        assert_eq!((r.cols, r.rows), (128, 42));
        // kite's desk first, then burrow's two, then puffin on lab: eight columns a lane, two between a project's, five
        // between projects, the whole in the middle
        let at: Vec<(&str, usize, usize)> = r.lanes.iter().map(|l| (l.id.as_str(), l.x, l.w)).collect();
        let need = 4 * 8 + 2 + 2 * 5;
        let x0 = (128 - need) / 2;
        let x = |id: &str| at.iter().find(|a| a.0 == id).unwrap().1;
        assert_eq!(x("k1"), x0);
        assert_eq!((x("b1"), x("b2")), (x0 + 13, x0 + 23));
        assert_eq!(x("p1"), x0 + 36);
        assert!(at.iter().all(|a| a.2 == 8));
        assert_eq!(r.groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>(), ["kite", "Burrow", "puffin @lab"]);
        assert_eq!((r.groups[1].x0, r.groups[1].x1), (x0 + 13, x0 + 31));
        // every lane's columns as tall as the box; the rest of the width nobody's, a column clear round each project
        assert!(r.lanes.iter().all(|l| l.cells.len() == 8 && l.cells.iter().all(|c| c.len() == 42)));
        assert!(r.ambient.iter().all(|a| r.groups.iter().all(|g| a.x + 1 < g.x0 || a.x > g.x1)));
        assert_eq!(r.ambient.len(), 128 - r.groups.iter().map(|g| g.x1 - g.x0 + 2).sum::<usize>());
        // narrow: the gaps shrink, then a lane is a column
        r.measure(60., 700., 10., &st, 0.);
        assert!(r.lanes.iter().all(|l| l.w == 1));
        assert_eq!(r.lane_at(r.lanes[0].x as f32 * 10. + 5.).map(|l| l.id.as_str()), Some(r.lanes[0].id.as_str()));
    }

    #[test]
    fn how_it_falls_is_how_it_is() {
        let st = state();
        let mut r = Rain::new(11);
        r.measure(1280., 700., 10., &st, 0.);
        r.got_texts(&HashMap::from([("b1".to_string(), "cargo test".to_string())]));
        let mut now = 0.;
        let mut glyphs = vec![];
        for _ in 0..300 {
            now += 33.;
            glyphs = r.step(&st, now, false);
        }
        let lit = |id: &str| r.lane(id).unwrap().cells.iter().flatten().filter(|c| c.c != '\0').count();
        // working: its lane full of what it did; idle: a drip; needs you: a whole page, standing still
        assert!(lit("b1") > 100 && lit("k1") < lit("b1"), "{} {}", lit("b1"), lit("k1"));
        assert_eq!(lit("b2"), 8 * 42);
        assert!(r.lane("b1").unwrap().cells.iter().flatten().filter(|c| c.c != '\0').all(|c| "cargotest".contains(c.c) || (c.c as u32) >= 0xff66 || c.c.is_ascii_digit()));
        let page: Vec<char> = r.lane("b2").unwrap().cells.iter().flatten().map(|c| c.c).collect();
        r.step(&st, now + 33., false);
        assert_eq!(page, r.lane("b2").unwrap().cells.iter().flatten().map(|c| c.c).collect::<Vec<_>>());
        // the names down the bottom of their lanes, bold: white for the one that needs you
        let name: String = glyphs.iter().filter(|g| g.bold && g.rgb == 0xe9ffee).map(|g| g.c).collect();
        assert_eq!(name, "talk-ui");
        assert!(glyphs.iter().any(|g| g.bold && g.rgb == 0x00ff41));
        // a machine out of reach: nothing new falls there, its name red
        let mut st2 = st.clone();
        st2.machines[1].error = Some("ssh: no route".into());
        let glyphs = r.step(&st2, now + 66., false);
        let red: String = glyphs.iter().filter(|g| g.bold && g.rgb == 0xff5f56).map(|g| g.c).collect();
        assert_eq!(red, "puffin");
    }

    #[test]
    fn the_gone_fall_apart_and_a_message_streaks_across() {
        let mut st = state();
        let mut r = Rain::new(3);
        r.measure(1280., 700., 10., &st, 0.);
        r.talk("k1", "p1", 0.);
        let g = r.step(&st, 450., false);
        assert!(g.iter().any(|g| !g.bold && g.y <= CH * 2. && g.x % 10. != 0.), "the streak along the top, off the columns");
        r.step(&st, 1000., false);
        assert!(r.flashes.contains_key("p1"), "the receiver flashes once it is hit");
        // a session gone: its lane stays a moment, out of the way of the pointer, then goes; the others close up
        st.bots.retain(|b| b.id != "k1");
        r.arrange(&st, 1000.);
        assert!(r.lane("k1").unwrap().gone.is_some() && r.lane_at(r.lane("k1").unwrap().x as f32 * 10.).is_none_or(|l| l.id != "k1"));
        r.step(&st, 1500., false);
        assert!(r.lane("k1").is_some());
        r.step(&st, 2300., false);
        assert!(r.lane("k1").is_none() && r.lanes.len() == 3);
        // text is asked for once a record changes, not more than every twelve seconds
        assert_eq!(r.want_texts(3000.).len(), 3);
        assert!(r.want_texts(4000.).is_empty());
        assert_eq!(r.want_texts(16000.).len(), 3);
        r.got_texts(&HashMap::from([("b1".to_string(), "x".to_string())]));
        assert_eq!(r.want_texts(40000.).len(), 2);
    }
}
