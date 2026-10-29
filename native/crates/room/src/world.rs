//! Everyone in the room and what each is doing (room.js's `world()`): the figures, their acts, and the
//! room's own goings-on (beams, the ball, remains on the floor, the whiteboard's ink, ...).

use crate::model::Bot;
use pix::Image;
use std::collections::{HashMap, HashSet};

/// a spot on the floor; one at a piece says which (`on`): whoever got there rides along when it is dragged
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spot {
    pub x: f64,
    pub y: f64,
    pub on: Option<String>,
}

impl Spot {
    pub fn at(x: f64, y: f64) -> Spot {
        Spot { x, y, on: None }
    }
    pub fn on(x: f64, y: f64, key: &str) -> Spot {
        Spot { x, y, on: Some(key.to_string()) }
    }
}

/// what a figure is doing: a kind and whatever that kind keeps (room.js's `f.act`, an object of any shape;
/// here every field any kind uses, unused ones at their default)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Act {
    pub kind: &'static str,
    /// seconds left; None = until something ends it (room.js's `undefined`)
    pub left: Option<f64>,
    pub there: bool,
    pub with: Option<String>,
    pub side: i32,
    pub you: bool,
    /// a piece's key (a plant, a window, the booth, a pit's key) or a figure's id
    pub at: Option<String>,
    /// a place (the pitch, a nap's mat)
    pub spot: Option<Spot>,
    pub to: Option<String>,
    /// where to walk (loiter, hide, condemned's next step)
    pub dest: Option<Spot>,
    pub home: Option<Spot>,
    pub stage: i32,
    pub wait: f64,
    pub back: bool,
    pub v: f64,
    pub pause: Option<f64>,
    pub panic: bool,
    /// the condemned taken aback: the shots it has heard (None: not yet), when it last gave a start
    pub bang: Option<f64>,
    pub jolt: f64,
    pub n: i32,
    pub look: f64,
    pub game: Option<&'static str>,
    pub game_id: Option<i64>,
    pub dir: i32,
    pub went: bool,
    pub water: bool,
    pub t: f64,
    pub free: bool,
    pub drawing: bool,
    pub done: f64,
    pub wave: f64,
    pub team: Vec<String>,
    pub slot: usize,
    pub shell: Option<String>,
    pub seat: usize,
    pub open: Option<String>,
    pub m: Option<String>,
    pub path: Option<String>,
    pub by: Option<String>,
    pub chosen: bool,
    /// the meeting halfway: where, and from which side
    pub mx: f64,
    pub my: f64,
}

impl Act {
    pub fn new(kind: &'static str) -> Act {
        Act { kind, ..Default::default() }
    }
    pub fn left(kind: &'static str, left: f64) -> Act {
        Act { kind, left: Some(left), ..Default::default() }
    }
    pub fn is(&self, k: &str) -> bool {
        self.kind == k
    }
}

/// a hunt in progress (hardcore): the hunter's phase and the one claimed
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hunt {
    pub phase: &'static str,
    pub id: Option<String>,
    pub t: f64,
}

/// an office manager clearing one out: who, and how far
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clean {
    pub id: String,
    pub phase: &'static str,
    pub t: f64,
}

/// a figure: a bot, you, an office manager
#[derive(Clone, Debug, Default)]
pub struct Fig {
    pub id: String,
    /// the bot (or your / her entry) it stands for
    pub b: Option<Bot>,
    pub human: bool,
    pub staff: bool,
    /// the office manager's style and booth (staff only)
    pub style: String,
    pub booth: Option<String>,
    pub seed: String,
    pub color: Option<pix::Color>,
    pub x: f64,
    pub y: f64,
    /// -1 facing left, 1 right
    pub dir: f64,
    pub arms: &'static str,
    pub legs: &'static str,
    pub carry: Option<&'static str>,
    pub act: Act,
    pub step: f64,
    /// a little phase of its own, for things that should not all happen at once
    pub z: f64,
    pub chat: f64,
    pub work: bool,
    pub busy: bool,
    pub waiting: bool,
    pub was_waiting: Option<bool>,
    pub open: bool,
    pub was_open: bool,
    pub sent: bool,
    pub reply: f64,
    pub sat: bool,
    pub sat_at: Option<String>,
    pub on: Option<String>,
    pub held: bool,
    pub leaving: bool,
    pub lying: bool,
    pub gone: bool,
    pub kick: f64,
    pub moved: f64,
    pub hunt: Option<Hunt>,
    pub clean: Option<Clean>,
    pub clean_rest: f64,
    pub last_kind: &'static str,
    pub walking: bool,
    /// it came in through its door just now (seconds since)
    pub came: f64,
    /// an office manager's session: busy, waiting, its process alive
    pub staff_state: (bool, bool, bool),
}

impl Fig {
    pub fn is_bot(&self) -> bool {
        self.b.is_some() && !self.human && !self.staff
    }
}

/// a message under way: a beam from one antenna to another, or between one and the telephone on a pair table
/// (`from_table`, `to_table`: its key; `from`/`to` empty then)
#[derive(Clone, Debug, Default)]
pub struct Beam {
    pub from: String,
    pub to: String,
    pub from_table: Option<String>,
    pub to_table: Option<String>,
    pub t: f64,
}

/// the football
#[derive(Clone, Debug, Default)]
pub struct Ball {
    pub x: f64,
    pub y: f64,
    pub x0: f64,
    pub y0: f64,
    pub from: String,
    pub to: String,
    pub t: f64,
    pub h: f64,
    pub wait: Option<f64>,
}

/// the meeting at the whiteboard
#[derive(Clone, Debug)]
pub struct Meeting {
    pub host: String,
    pub left: f64,
}

/// one condemned: its desk, if it goes with it
#[derive(Clone, Debug, Default)]
pub struct Condemned {
    pub unit: Option<String>,
}

/// a flying or landed piece of a shot (or burnt) bot
#[derive(Clone, Debug, Default)]
pub struct Part {
    pub kind: &'static str,
    pub img: Option<std::rc::Rc<Image>>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub rot: f64,
    pub spin: f64,
    pub col: Option<pix::Color>,
    pub w: i32,
    pub h: i32,
    pub sx: i32,
    pub sy: i32,
    pub flip: bool,
    pub charred: bool,
    pub bounced: i32,
    /// a cog: the big one
    pub big: bool,
    /// put back together: where it flew in from
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
}

/// a drop of oil on the floor
#[derive(Clone, Debug, Default)]
pub struct Drop {
    pub x: f64,
    pub y: f64,
    /// its size, 1 or 2 dots
    pub s: i32,
    /// a highlight
    pub hi: bool,
}

/// what is left of a shot bot: its pieces and the oil; a click puts it back together
#[derive(Clone, Debug, Default)]
pub struct Remains {
    pub x: f64,
    pub y: f64,
    pub t0: f64,
    pub parts: Vec<Part>,
    pub drops: Vec<Drop>,
    pub back: bool,
    pub back_t: f64,
    pub burnt: bool,
    pub b: Option<Bot>,
    pub name: String,
    pub human: bool,
    pub staff: bool,
    pub booth: Option<String>,
    pub seed: String,
    pub color: Option<pix::Color>,
    pub fade: f64,
    /// the puddle's half width, and how many parts there were (it shrinks as they are mopped up)
    pub puddle: f64,
    pub total: usize,
}

/// a shot fired: its flash
#[derive(Clone, Debug)]
pub struct Shot {
    pub x: f64,
    pub y: f64,
    pub t0: f64,
}

/// something on its way into the fire: its picture, from where it was
#[derive(Clone, Debug)]
pub struct Burning {
    pub img: std::rc::Rc<Image>,
    pub x: f64,
    pub y: f64,
    pub t0: f64,
}

/// the whiteboard's dream: the scene, the order its pixels come in, how far
#[derive(Clone, Debug, Default)]
pub struct Dream {
    pub scene: usize,
    pub order: Vec<(i32, i32, pix::Color)>,
    pub n: usize,
    pub done: Option<f64>,
    pub mine: HashMap<usize, pix::Color>,
    pub f: f64,
}

/// what the dog is up to
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DogAct {
    pub kind: &'static str,
    pub left: f64,
    /// afraid: only its eyes in the dark door
    pub hide: bool,
    /// where it trots to (wander)
    pub to: Option<Spot>,
    /// whom it visits or barks at
    pub who: Option<String>,
    pub sniff: f64,
    /// fetch: run, pick, back, wait
    pub stage: &'static str,
    pub wait: f64,
    /// after a shake: home, or what comes next
    pub next: Option<Box<DogAct>>,
    pub yap: bool,
    /// called out of its house: a few steps out first
    pub out: bool,
}

impl DogAct {
    pub fn new(kind: &'static str, left: f64) -> DogAct {
        DogAct { kind, left, ..Default::default() }
    }
}

/// the dog: where it is and what it is up to
#[derive(Clone, Debug, Default)]
pub struct Dog {
    pub x: f64,
    pub y: f64,
    pub dir: f64,
    pub act: DogAct,
    pub pose: &'static str,
    pub wag: i32,
    pub step: f64,
    pub z: f64,
    pub inside: bool,
    pub asleep: bool,
    pub held: bool,
    pub leaving: bool,
    pub barking: bool,
    /// hearts rising: x, y, when
    pub hearts: Vec<(f64, f64, f64)>,
    pub love_at: Option<f64>,
}

/// the stick you throw for the dog
#[derive(Clone, Debug, Default)]
pub struct Stick {
    pub x: f64,
    pub y: f64,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub t: f64,
    pub h: f64,
    pub held: bool,
    pub turn: f64,
}

/// what everyone has been up to since the room opened, for the printer's report
#[derive(Clone, Debug, Default)]
pub struct Stat {
    pub busy: f64,
    pub idle: f64,
    pub asleep: f64,
    pub sent: u32,
    pub got: u32,
    pub acts: HashMap<&'static str, u32>,
}

/// everything that is not a figure
#[derive(Default)]
pub struct Shared {
    pub condemned: Vec<(String, Condemned)>,
    pub razed: HashSet<String>,
    pub meeting: Option<Meeting>,
    pub last_meeting: f64,
    /// the bots' own ping-pong ball: how far across, from which side
    pub pong_t: f64,
    pub pong_from: i32,
    pub stopped: HashMap<String, f64>,
    pub flare: f64,
    pub sparks: f64,
    pub booth_flash: Option<(String, f64)>,
    pub flash: Option<(f64, f64, f64)>,
    pub shots: Vec<Shot>,
    pub spared: HashSet<String>,
    pub print: f64,
    pub burning: Vec<Burning>,
    pub stats: HashMap<String, Stat>,
    pub meetings: u32,
    pub remains: Vec<(String, Remains)>,
    pub rise_at: HashMap<String, (f64, f64)>,
    pub sway: HashMap<String, f64>,
    pub ball: Option<Ball>,
    pub mugs: HashMap<String, f64>,
    pub beams: Vec<Beam>,
    pub hums: HashMap<String, bool>,
    /// seconds the room has run
    pub t: f64,
    pub fear: f64,
    /// when the last bot was shot (the condemned still waiting start at it)
    pub bang: f64,
    pub plies: Option<usize>,
    pub chess_at: f64,
    pub chess_asked: f64,
    pub dream: Option<Dream>,
    /// the whiteboard's ink, 130 x 55 (five pixels a board dot)
    pub ink: Option<Image>,
    pub ink_v: u64,
    pub dog: Option<Dog>,
    pub stick: Option<Stick>,
    pub dog_held: bool,
    /// a newcomer the dog goes and barks at
    pub dog_news: Option<String>,
    /// the bots you shot (the face in the first person grins)
    pub kills: u32,
    /// a door ajar: when someone last went through it, by machine
    pub door_at: HashMap<String, f64>,
    pub spark_on: bool,
}

pub struct World {
    pub folk: Vec<Fig>,
    pub s: Shared,
}

impl World {
    pub fn new() -> World {
        World { folk: Vec::new(), s: Shared { last_meeting: -60.0, ..Default::default() } }
    }
    pub fn idx(&self, id: &str) -> Option<usize> {
        self.folk.iter().position(|f| f.id == id)
    }
    pub fn get(&self, id: &str) -> Option<&Fig> {
        self.folk.iter().find(|f| f.id == id)
    }
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Fig> {
        self.folk.iter_mut().find(|f| f.id == id)
    }
    pub fn remains(&self, id: &str) -> Option<&Remains> {
        self.s.remains.iter().find(|(k, _)| k == id).map(|(_, r)| r)
    }
    pub fn remains_mut(&mut self, id: &str) -> Option<&mut Remains> {
        self.s.remains.iter_mut().find(|(k, _)| k == id).map(|(_, r)| r)
    }
    pub fn is_condemned(&self, id: &str) -> bool {
        self.s.condemned.iter().any(|(k, _)| k == id)
    }
    pub fn stat(&mut self, id: &str) -> &mut Stat {
        self.s.stats.entry(id.to_string()).or_default()
    }
}

impl Default for World {
    fn default() -> World {
        World::new()
    }
}

/// the figure at `i` to change, and the others to look at (and change: a partner is told too)
pub struct Others<'a> {
    pub before: &'a mut [Fig],
    pub after: &'a mut [Fig],
}

impl<'a> Others<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &Fig> {
        self.before.iter().chain(self.after.iter())
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Fig> {
        self.before.iter_mut().chain(self.after.iter_mut())
    }
    pub fn get(&self, id: &str) -> Option<&Fig> {
        self.iter().find(|g| g.id == id)
    }
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Fig> {
        self.iter_mut().find(|g| g.id == id)
    }
}

pub fn split(folk: &mut [Fig], i: usize) -> (&mut Fig, Others<'_>) {
    let (before, rest) = folk.split_at_mut(i);
    let (f, after) = rest.split_first_mut().expect("a figure there");
    (f, Others { before, after })
}

/// the dice of the room (Math.random); seeded, so a test sees the same room every time
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Dice {
        Dice(seed.max(1))
    }
    pub fn next(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64) / (1u64 << 53) as f64
    }
    pub fn rnd(&mut self, a: f64, b: f64) -> f64 {
        a + self.next() * (b - a)
    }
    pub fn any<'a, T>(&mut self, a: &'a [T]) -> Option<&'a T> {
        if a.is_empty() {
            None
        } else {
            Some(&a[(self.next() * a.len() as f64) as usize % a.len()])
        }
    }
}

/// a pause before the next break
pub fn rest(r: f64) -> Act {
    Act::left("stand", 6.0 + r * 10.0)
}

/// whatever it was doing, over: up, nothing in its hands
pub fn end(f: &mut Fig) {
    f.lying = false;
    f.carry = None;
    f.on = None;
    f.act = rest(0.06);
}

/// up from a nap, the mat rolled up under its arm
pub fn wake(f: &mut Fig) {
    f.lying = false;
    f.legs = "stand";
    f.carry = Some("roll");
    f.act = Act::left("stand", 1.5);
}

/// to a spot; one at a piece says so, and who has got there rides along when the piece is dragged (`ride`).
/// Every act walks to its spot each tick, so `on` is always the piece the figure is at now
pub fn walk(f: &mut Fig, to: &Spot, v: f64, dt: f64) -> bool {
    let (dx, dy) = (to.x - f.x, to.y - f.y);
    let d = dx.hypot(dy);
    if d <= v * dt {
        f.x = to.x;
        f.y = to.y;
        f.on = to.on.clone();
        if f.legs != "sit" {
            f.legs = "stand";
        }
        f.walking = false;
        return true;
    }
    f.on = None;
    f.x += dx / d * v * dt;
    f.y += dy / d * v * dt;
    if dx.abs() > 0.5 {
        f.dir = dx.signum();
    }
    f.step += dt * 8.0;
    let k = f.step.floor() as i64 % 2;
    f.legs = if k != 0 { "walk1" } else { "walk2" };
    f.arms = f.carry.unwrap_or(if k != 0 { "sw1" } else { "sw2" });
    f.walking = true;
    false
}

/// seated somewhere that can be moved: stays on its seat (a desk's chairs close up when one leaves). A seat
/// at another piece (its directory changed, even to a desk close by) is walked to
pub fn sit_at(f: &mut Fig, s: &Spot, v: f64, dt: f64) -> bool {
    if f.legs == "sit" && f.sat && f.sat_at == s.on && (s.x - f.x).hypot(s.y - f.y) < 40.0 {
        f.x = s.x;
        f.y = s.y;
        f.on = s.on.clone();
        return true;
    }
    f.sat = walk(f, s, v, dt);
    if f.sat {
        f.sat_at = s.on.clone();
    }
    f.sat
}

/// room.js's fnv: a string's 32-bit FNV-1a, as unsigned
pub fn fnv(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for ch in s.chars() {
        h = (h ^ ch as u32).wrapping_mul(16777619);
    }
    h
}
