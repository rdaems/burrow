//! The room as one thing (room.js's `startRoom`): what it keeps between frames, the figures coming in and going
//! out, the tick, twelve a second like a 16-bit game, and the pointer. Drawing is in `scene.rs`.

use crate::io::{Input, Output};
use crate::layout::{self, Keep, Layout, Moving, Size, WALL, foot, free_at, kind_of};
use crate::model::Bot;
use crate::think::{self, Tick, doomed};
use crate::world::*;
use pix::{Color, Image};
use serde_json::json;
use std::collections::HashSet;

/// your clothes: pair's "me"
pub const TERRACOTTA: Color = Color::rgb(0xd97757);

/// a hot spot: where a click does something (collected while drawing, front last)
#[derive(Clone, Debug)]
pub struct Hot {
    pub r: [f64; 4],
    pub action: serde_json::Value,
}

/// the pointer pressed: where, on what
#[derive(Clone, Debug)]
pub struct Press {
    pub x: f64,
    pub y: f64,
    pub action: Option<serde_json::Value>,
    pub bot: Option<String>,
    pub piece: Option<layout::PieceR>,
    pub dog: bool,
    pub shift: bool,
}

/// the bot under the pointer: its entry, and where its name goes
#[derive(Clone, Debug)]
pub struct Hovered {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub seat: bool,
}

/// a tip over something under the pointer
#[derive(Clone, Debug)]
pub struct Tip {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

pub struct Room {
    pub w: World,
    pub keep: Keep,
    pub dice: Dice,
    /// the room's width in dots (the divider), and its height (the window)
    pub size: Size,
    /// the width was set by the divider here (not the state's)
    pub width_set: bool,
    pub moving: Option<Moving>,
    /// the figure carried (its id)
    pub held: Option<String>,
    /// a piece carried in from the closet: its kind, where it would land
    pub carrying: Option<(String, Option<[i32; 2]>)>,
    /// a new machine's door, dropped from the closet, hanging while its form is open
    pub pending: Option<([i32; 2], Option<String>, String, f64)>,
    pub press: Option<Press>,
    /// the pointer, in dots
    pub pointer: Option<(f64, f64)>,
    pub hots: Vec<Hot>,
    pub tip: Option<Tip>,
    pub hovered: Option<Hovered>,
    pub hover_id: Option<String>,
    pub hover_screen: Option<String>,
    /// the screen under the pointer: its machine and directory (a shell there)
    pub screen_hover: Option<(String, String)>,
    pub dog_rect: Option<[f64; 4]>,
    pub danger_now: bool,
    pub fresh: HashSet<String>,
    /// when the room started (ms)
    pub born: f64,
    pub last: Option<Layout>,
    /// seconds waiting for the next tick
    acc: f64,
    pub tick_dt: f64,
    /// the room's picture, W*2 x H*2 (two pixels a dot)
    pub img: Image,
    /// the floor, drawn once for the room's size
    pub floor: Option<Image>,
    /// the pointer's look over the room: default, pointer, grab, grabbing
    pub cursor: &'static str,
    /// what the first person keeps of the room
    pub third: crate::scene3::Third,
}

impl Room {
    pub fn new(seed: u64, now: f64) -> Room {
        Room {
            w: World::new(),
            keep: Keep::default(),
            dice: Dice::new(seed),
            size: Size { w: 256, h: 400 },
            width_set: false,
            moving: None,
            held: None,
            carrying: None,
            pending: None,
            press: None,
            pointer: None,
            hots: Vec::new(),
            tip: None,
            hovered: None,
            hover_id: None,
            hover_screen: None,
            screen_hover: None,
            dog_rect: None,
            danger_now: false,
            fresh: HashSet::new(),
            born: now,
            last: None,
            acc: 0.0,
            tick_dt: 0.0,
            img: Image::new(512, 800),
            floor: None,
            cursor: "default",
            third: Default::default(),
        }
    }

    /// the room's size for a pane `h_px` CSS pixels tall; the width is the state's (the divider dragged) unless set here
    pub fn resize(&mut self, inp: &Input, h_px: f64) {
        if let Some(w) = inp.state.width
            && !self.width_set
        {
            self.size.w = (w.round() as i32).clamp(200, 1200);
        }
        self.size.h = (((h_px - 2.0 * layout::MARGIN as f64) / layout::K as f64).floor() as i32).max(200);
    }

    /// the divider dragged: the room this many CSS pixels wide (whole dots)
    pub fn set_width(&mut self, px: f64) -> i32 {
        self.width_set = true;
        self.size.w = ((px / layout::K as f64).round() as i32).clamp(200, 1200);
        self.size.w
    }

    /// time passes: twelve ticks a second at most (dt in seconds, as the app's frames come)
    pub fn advance(&mut self, dt: f64) -> bool {
        self.acc += dt.min(0.1);
        if self.acc < 1.0 / 12.0 {
            return false;
        }
        self.tick_dt = self.acc.min(0.25);
        self.acc = 0.0;
        true
    }

    /// a bot's figure as the room keeps its entry: bots by id; you as "you"; office managers as mgr:<booth>
    fn entries(&self, inp: &Input, l: &Layout) -> Vec<(Bot, bool, bool, Option<String>, String, Option<Color>, String, (bool, bool, bool))> {
        let s = &inp.state;
        let mut out = Vec::new();
        for b in &s.bots {
            out.push((b.clone(), false, false, None, b.id.clone(), Some(layout::project_color(&b.unit)), String::new(), (false, false, false)));
        }
        let you_name = if s.you.name.is_empty() { "you".to_string() } else { s.you.name.clone() };
        let you = Bot { id: "you".into(), name: you_name.clone(), ..Default::default() };
        let you_seed = if s.you.seed.is_empty() { you_name } else { s.you.seed.clone() };
        let you_col = s.you.color.as_deref().and_then(crate::avatar::color_of).unwrap_or(TERRACOTTA);
        out.push((you, true, false, None, you_seed, Some(you_col), String::new(), (false, false, false)));
        // the office managers, one at each reception
        for mb in &l.managers {
            if let Some(m) = s.managers.iter().find(|m| m.key == mb.key && !m.vacant) {
                let b = Bot { id: format!("mgr:{}", m.key), name: m.name.clone(), rc: m.rc.clone(), alive: m.alive, ..Default::default() };
                out.push((b, false, true, Some(m.key.clone()), m.seed.clone(), crate::avatar::color_of(&m.color), m.style.clone(), (m.busy, m.waiting, m.alive)));
            }
        }
        out
    }

    /// the frame's work before drawing: lay out, keep a figure per bot (in by the door, out by it), think
    pub fn step(&mut self, inp: &Input, out: &mut Output) -> Layout {
        let s = &inp.state;
        let now = inp.now;
        let gone_ids: HashSet<String> = self
            .w
            .s
            .remains
            .iter()
            .map(|(k, _)| k.clone())
            .chain(self.w.s.stopped.keys().cloned())
            .chain(self.w.folk.iter().filter(|f| f.act.kind == "doom").map(|f| f.id.clone()))
            .collect();
        // a burnt desk whose bots are on their way out is not drawn; nor one that came with its sessions once the last
        // of them is deleted from here (the feed still lists it a moment)
        let shown: Vec<_> = s
            .desks
            .iter()
            .filter(|d| !self.w.s.razed.contains(&d.unit) && (d.kept || s.bots.iter().any(|b| b.unit == d.unit && !self.w.s.stopped.contains_key(&b.id))))
            .cloned()
            .collect();
        let l = layout::layout(s, &shown, &self.size, &mut self.keep, self.moving.as_ref(), &|id| gone_ids.contains(id), now);
        if let Some(m) = self.moving.as_ref().map(|m| m.key.clone()) {
            ride(&mut self.w, &l, self.last.as_ref(), &m); // whoever is at what is being dragged comes along, this very frame
        }
        if !l.placed.is_empty() {
            let mut lay = serde_json::Map::new();
            for (k, v) in &s.layout {
                lay.insert(k.clone(), json!(v));
            }
            for (k, v) in &l.placed {
                lay.insert(k.clone(), json!(v));
            }
            out.act(json!({"kind": "layout", "layout": lay}));
        }
        // keep a figure per bot; one that turns up later comes in through its machine's door, one that is gone from
        // the list (deleted elsewhere) walks out of it
        let entries = self.entries(inp, &l);
        let ids: HashSet<String> = entries.iter().map(|e| e.0.id.clone()).collect();
        for f in self.w.folk.iter_mut() {
            if !ids.contains(&f.id) && !f.leaving {
                f.leaving = true;
                f.held = false;
                f.lying = false;
                f.sat = false;
                f.legs = "stand";
            }
        }
        self.w.s.stopped.retain(|id, t| ids.contains(id) && now - *t <= 30000.0);
        let late = now - self.born > 3000.0;
        let machines: Vec<String> = s.machines.iter().map(|m| m.id.clone()).collect();
        for (b, human, staff, booth, seed, color, style, staff_state) in entries {
            if self.w.s.stopped.contains_key(&b.id) || self.w.remains(&b.id).is_some() {
                continue; // stopped from here (the feed still lists it for a moment), or in pieces
            }
            if self.w.idx(&b.id).is_none() {
                let door = l.door_x(&b.m, machines.iter().position(|m| *m == b.m).unwrap_or(0));
                let at = if staff && !late {
                    think::manager_seat(&l, l.managers.iter().find(|m| Some(&m.key) == booth.as_ref()))
                } else if late || human {
                    // back from its pieces, or in by the door; at first, at its desk or anywhere
                    match self.w.s.rise_at.get(&b.id) {
                        Some(&(x, y)) => Spot::at(x, y),
                        None => Spot::at(door, (WALL + 3) as f64),
                    }
                } else {
                    l.seat(&b.id).unwrap_or_else(|| free_at(&l, &b.id))
                };
                self.w.s.rise_at.remove(&b.id);
                if late && !human && !staff {
                    self.w.s.dog_news = Some(b.id.clone()); // a newcomer: the dog goes and barks at it
                    let pan = ((at.x / self.size.w as f64) * 1.6 - 0.8).clamp(-1.0, 1.0);
                    out.sounds.push(crate::io::Sfx { name: "door", pan, vol: 1.0, hum: None });
                }
                let work = !human && (b.busy || b.waiting);
                let stand = late || (human && !staff);
                let z = self.dice.next() * 2.0;
                let r = self.dice.next();
                self.w.folk.push(Fig {
                    id: b.id.clone(),
                    x: at.x,
                    y: at.y,
                    dir: -1.0,
                    arms: "down",
                    legs: if stand { "stand" } else { "sit" },
                    sat: !stand,
                    act: if work { Act::new("desk") } else { rest(r) },
                    busy: b.busy,
                    waiting: b.waiting,
                    work,
                    human,
                    staff,
                    z,
                    ..Default::default()
                });
            }
            let f = self.w.get_mut(&b.id).unwrap();
            f.human = human;
            f.staff = staff;
            f.booth = booth;
            f.seed = seed;
            f.color = color;
            f.style = style;
            f.staff_state = staff_state;
            f.b = Some(b);
        }
        if self.tick_dt > 0.0 {
            let dt = self.tick_dt;
            self.w.s.t += dt;
            let bots: Vec<Bot> = s.bots.clone();
            let moving_fire = self.moving.as_ref().is_some_and(|m| kind_of(&m.key) == "fire");
            let mut t = Tick { inp, out, dice: &mut self.dice, fresh: &mut self.fresh, moving_fire, w: self.size.w };
            think::think(&mut self.w, &bots, &l, dt, &mut t);
            let mut left_now = Vec::new();
            for f in self.w.folk.iter_mut() {
                let m = f.b.as_ref().map(|b| b.m.clone()).unwrap_or_default();
                let door = l.door_x(&m, machines.iter().position(|x| *x == m).unwrap_or(0));
                if f.leaving && walk(f, &Spot::at(door, (WALL + 1) as f64), 30.0, dt) {
                    left_now.push((f.id.clone(), m, f.x));
                }
            }
            for (id, m, x) in left_now {
                t.sfx("door", x);
                self.w.folk.retain(|f| f.id != id);
                self.w.s.door_at.insert(m, now);
            }
            self.tick_dt = 0.0;
        }
        self.last = Some(l.clone());
        l
    }

    // ---- what the app tells the room

    /// `from` sent `to` a message: a beam
    pub fn talk(&mut self, inp: &Input, out: &mut Output, from: &str, to: &str) {
        let mut t = Tick { inp, out, dice: &mut self.dice, fresh: &mut self.fresh, moving_fire: false, w: self.size.w };
        think::talk(&mut self.w, &mut t, from, to);
    }
    /// you typed to the open session
    pub fn typed(&mut self, now: f64, id: Option<&str>, sent: bool) {
        think::typed(&mut self.w, now, id, sent);
    }
    /// a bot you have just started: to its desk, and you with it
    pub fn fresh(&mut self, id: &str) {
        self.fresh.insert(id.to_string());
    }
    /// let these go (a burnt desk's bots, ✕✕ in the completed list): to the booth
    pub fn condemn(&mut self, inp: &Input, out: &mut Output, ids: &[String]) -> Vec<String> {
        let mut t = Tick { inp, out, dice: &mut self.dice, fresh: &mut self.fresh, moving_fire: false, w: self.size.w };
        think::condemn(&mut self.w, &mut t, ids, None)
    }
    /// the first person shot a bot: its pieces fly (false if it cannot be shot: held, gone, you)
    pub fn shoot(&mut self, inp: &Input, out: &mut Output, id: &str) -> bool {
        let ok = self.w.get(id).is_some_and(|f| !f.held && f.b.is_some() && !f.human);
        if ok {
            let l = self.last.clone();
            let mut t = Tick { inp, out, dice: &mut self.dice, fresh: &mut self.fresh, moving_fire: false, w: self.size.w };
            if let Some(l) = l {
                think::shoot(&mut self.w, &l, id, &mut t);
            }
        }
        ok
    }
    /// put back together (E on its remains, in the first person)
    pub fn unshoot(&mut self, now: f64, id: &str) {
        think::unshoot(&mut self.w, id, now);
    }
    /// a shot heard: the idle bots near it run off
    pub fn alarm(&mut self, inp: &Input, out: &mut Output, x: f64, y: f64) {
        let Some(l) = self.last.clone() else { return };
        let mut t = Tick { inp, out, dice: &mut self.dice, fresh: &mut self.fresh, moving_fire: false, w: self.size.w };
        think::alarm(&mut self.w, &l, x, y, &mut t);
    }
    /// you are where the first person says
    pub fn move_you(&mut self, x: f64, y: f64, a: f64, walking: bool) {
        if let Some(f) = self.w.get_mut("you")
            && !f.held
        {
            f.x = x;
            f.y = y;
            f.dir = if a.cos() < 0.0 { -1.0 } else { 1.0 };
            f.walking = walking;
        }
    }
    /// out of the first person: you stand where you went, the gun put away
    pub fn left_fps(&mut self) {
        if let Some(f) = self.w.get_mut("you") {
            f.hunt = None;
            end(f);
            f.carry = None;
        }
    }
    /// where the first person starts: where you stand (by the closet, the gun just taken), looking into the room
    pub fn fps_start(&self, key: &str) -> (f64, f64, f64) {
        let (w, h) = (self.size.w as f64, self.size.h as f64);
        let p = self.last.as_ref().and_then(|l| l.piece_of(key).or(l.booths.first().and_then(|b| l.piece_of(&b.key)))).cloned();
        let (x, y) = match (self.w.get("you"), p) {
            (Some(me), _) => (me.x, me.y),
            (None, Some(p)) => (((p.r[0] + p.r[2]) as f64 / 2.0).clamp(14.0, w - 14.0), ((p.r[3] + 22) as f64).min(h - 8.0)),
            _ => (w / 2.0, h / 2.0),
        };
        (x, y, (((WALL as f64) + h) / 2.0 - y).atan2(w / 2.0 - x))
    }

    /// the two at a game table right now (chess, pong), white or the left side first: the app lets you watch them
    pub fn playing(&self, kind: &str) -> Vec<String> {
        let mut v: Vec<&Fig> = self.w.folk.iter().filter(|f| f.act.kind == kind && f.act.there && f.act.with.is_some() && !f.act.you && !f.human).collect();
        v.sort_by_key(|f| f.act.side);
        v.iter().map(|f| f.id.clone()).collect()
    }
    /// a figure, where it stands (dots)
    pub fn at(&self, id: &str) -> Option<(f64, f64)> {
        self.w.get(id).map(|f| (f.x, f.y))
    }
    /// is anything in the room being dragged or carried
    pub fn busy_hand(&self) -> bool {
        self.moving.is_some() || self.held.is_some() || self.w.s.dog_held
    }
}

/// a piece being dragged takes along whoever is at it, this very frame: think runs twelve times a second, a drag
/// draws with the pointer. Whoever got to a spot of it (walk's `on`), by as much as it moved
pub fn ride(w: &mut World, l: &Layout, before: Option<&Layout>, key: &str) {
    let (Some(p), Some(q)) = (l.piece_of(key), before.and_then(|b| b.piece_of(key))) else { return };
    let (dx, dy) = ((p.at[0] - q.at[0]) as f64, (p.at[1] - q.at[1]) as f64);
    if dx != 0.0 || dy != 0.0 {
        for f in w.folk.iter_mut() {
            if f.on.as_deref() == Some(key) && !f.held {
                f.x += dx;
                f.y += dy;
            }
        }
    }
}

/// a bot that is about to go (in the booth, in line, condemned): not to be opened
pub fn is_doomed(f: &Fig) -> bool {
    doomed(&f.act)
}

pub fn footprint(key: &str, at: [i32; 2], u: Option<i32>) -> layout::Rect {
    foot(key, at, u)
}
