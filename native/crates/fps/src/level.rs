//! A Doom level played, simply (a port of web/doom/level.js): the map from a WAD (wad.rs), the player, Doom's
//! monsters, items and specials, in Doom's units (a unit a texel) and its tics (35 a second). What it does, as Doom
//! does it, more or less:
//! - moving: against the lines (one-sided, blocking, an opening lower than you, a step higher than 24) and the solid
//!   things, sliding along walls; falling with Doom's gravity, your height the highest floor under you;
//! - specials: doors (opened by use, by walking over a line, keyed; monsters open the plain ones), lifts (down, a
//!   wait, up), floors that lower, switches (their texture flips), the exit (switch or line: the level is done); sector
//!   lights that blink, strobe, glow and flicker; floors that hurt (nukage);
//! - things: monsters wake when they see you, or when a shot's sound reaches their sector (through every opening, past
//!   one sound-blocking line at most); they chase you in eight directions, attack as often as Doom's A_Chase lets
//!   them (zombiemen, shotgun guys: hitscan; imps, cacodemons, barons: fireballs; demons, imps up close: claws and
//!   teeth), feel pain and die on Doom's frames; items are picked up as you walk over them (health, armor, ammo, keys,
//!   a backpack); barrels explode;
//! - the shotgun: seven pellets in a spread, five to fifteen each, puffs on the walls, blood on what is hit.
//! What it leaves out: the other weapons (their ammo is counted), vertical aim (autoaim, as Doom's), saving.
//!
//! `portal` (set by fps.rs): the mysterious door out to burrow's office, in a wall of the map. While it is open you
//! go through it (in its doorway), and so do, while you are in the office (`away`), the monsters' sight, bullets and
//! fireballs on their way to you, and your shots on their way out: beyond the door is the office, where no line of the
//! map is. Elsewhere behind that wall the map is the map. While you are away the level goes on (monsters see you
//! through the door, come to it, shoot through it).
//!
//! What level.js shared by reference is here by index: sectors, sides and lines in `map`, mobjs by a stable id.

use crate::rng::{Rng, jround};
use crate::wad::{Map, MapThing, Wad};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;

const RAD: f64 = PI / 180.0;

/// the player, or a thing by its id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Who {
    Player,
    Mobj(u32),
}

/// where a sound came from: one sound an origin at a time (Doom's channels)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    Player,
    Mobj(u32),
    Sector(usize),
    /// a switch, a spot of its own
    Spot(u32),
    /// the mysterious door (fps.rs)
    Door,
    /// your shotgun
    Gun,
    /// a box of shells from the gun closet
    Pickup,
}

/// a sound in the level this tic: Doom's lump, where, from what
#[derive(Clone, Debug)]
pub struct Heard {
    pub name: &'static str,
    pub x: f64,
    pub y: f64,
    pub o: Origin,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Atk {
    #[default]
    None,
    Pistol,
    Shotgun,
    Imp,
    Bite,
    Caco,
    Baron,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ammo {
    Bullets = 0,
    Shells = 1,
    Rockets = 2,
    Cells = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Blue = 0,
    Yellow = 1,
    Red = 2,
}

/// what a thing is (Doom's mobjinfo, the parts used)
#[derive(Clone, Copy, Debug, Default)]
pub struct Def {
    pub s: &'static str,
    pub frames: &'static str,
    pub mon: bool,
    pub hp: i32,
    pub speed: f64,
    pub r: f64,
    pub h: f64,
    pub pain: i32,
    pub atk: Atk,
    pub solid: bool,
    pub shootable: bool,
    pub melee: bool,
    /// a monster that never shoots from afar
    pub no_missile: bool,
    pub fly: bool,
    pub bright: bool,
    pub shadow: bool,
    pub barrel: bool,
    pub hang: bool,
    pub pick: bool,
    pub health: i32,
    pub max: i32,
    pub always: bool,
    pub armor: i32,
    pub cls: i32,
    pub ammo: Option<Ammo>,
    pub n: i32,
    pub backpack: bool,
    pub key: Option<Key>,
}

const fn mon(s: &'static str, hp: i32, speed: f64, r: f64, pain: i32, atk: Atk) -> Def {
    Def {
        s,
        frames: "",
        mon: true,
        hp,
        speed,
        r,
        h: 56.0,
        pain,
        atk,
        solid: true,
        shootable: true,
        melee: false,
        no_missile: false,
        fly: false,
        bright: false,
        shadow: false,
        barrel: false,
        hang: false,
        pick: false,
        health: 0,
        max: 0,
        always: false,
        armor: 0,
        cls: 0,
        ammo: None,
        n: 0,
        backpack: false,
        key: None,
    }
}
const fn item(s: &'static str, frames: &'static str) -> Def {
    let mut d = mon(s, 0, 0.0, 20.0, 0, Atk::None);
    d.mon = false;
    d.solid = false;
    d.shootable = false;
    d.frames = frames;
    d.pick = true;
    d.h = 16.0;
    d
}
const fn deco(s: &'static str, frames: &'static str, solid: bool) -> Def {
    let mut d = item(s, frames);
    d.pick = false;
    d.solid = solid;
    d.r = 16.0;
    d
}

/// the things a map can have: sprite, size, what it is. A monster's melee and missile attacks as Doom's mobjinfo has them
pub fn def(ty: i32) -> Option<Def> {
    use Atk::{Baron, Bite, Caco, Imp, Pistol, Shotgun};
    let health = |s, f, n, max, always, bright| Def { health: n, max, always, bright, ..item(s, f) };
    let armor = |s, f, n, cls| Def { armor: n, cls, ..item(s, f) };
    let ammo = |s, a, n| Def { ammo: Some(a), n, ..item(s, "A") };
    let key = |s, k| Def { key: Some(k), ..item(s, "AB") };
    let bright = |s, f, solid| Def { bright: true, ..deco(s, f, solid) };
    let hang = |s, f, solid| Def { hang: true, ..deco(s, f, solid) };
    Some(match ty {
        3004 => mon("POSS", 20, 8.0, 20.0, 200, Pistol),
        9 => mon("SPOS", 30, 8.0, 20.0, 170, Shotgun),
        3001 => Def { melee: true, ..mon("TROO", 60, 8.0, 20.0, 200, Imp) },
        3002 => Def { melee: true, no_missile: true, ..mon("SARG", 150, 10.0, 30.0, 180, Bite) },
        58 => Def { melee: true, no_missile: true, shadow: true, ..mon("SARG", 150, 10.0, 30.0, 180, Bite) },
        3006 => Def { fly: true, bright: true, melee: true, no_missile: true, ..mon("SKUL", 100, 8.0, 16.0, 256, Bite) },
        3005 => Def { fly: true, melee: true, ..mon("HEAD", 400, 8.0, 31.0, 128, Caco) },
        3003 => Def { melee: true, ..mon("BOSS", 1000, 8.0, 24.0, 50, Baron) },
        2035 => Def { s: "BAR1", frames: "AB", r: 10.0, h: 42.0, solid: true, shootable: true, hp: 20, barrel: true, ..deco("BAR1", "AB", true) },
        // items (Doom's amounts)
        2011 => health("STIM", "A", 10, 100, false, false),
        2012 => health("MEDI", "A", 25, 100, false, false),
        2014 => health("BON1", "ABCDCB", 1, 200, true, false),
        2015 => Def { armor: 1, max: 200, always: true, ..item("BON2", "ABCDCB") },
        2018 => armor("ARM1", "AB", 100, 1),
        2019 => armor("ARM2", "AB", 200, 2),
        2013 => health("SOUL", "ABCDCB", 100, 200, true, true),
        2023 => health("PSTR", "A", 100, 100, false, true),
        2008 => ammo("SHEL", Ammo::Shells, 4),
        2049 => ammo("SBOX", Ammo::Shells, 20),
        2001 => ammo("SHOT", Ammo::Shells, 8),
        2007 => ammo("CLIP", Ammo::Bullets, 10),
        2048 => ammo("AMMO", Ammo::Bullets, 50),
        2010 => ammo("ROCK", Ammo::Rockets, 1),
        2046 => ammo("BROK", Ammo::Rockets, 5),
        2047 => ammo("CELL", Ammo::Cells, 20),
        17 => ammo("CELP", Ammo::Cells, 100),
        8 => Def { backpack: true, ..item("BPAK", "A") },
        5 => key("BKEY", Key::Blue),
        6 => key("YKEY", Key::Yellow),
        13 => key("RKEY", Key::Red),
        40 => key("BSKU", Key::Blue),
        39 => key("YSKU", Key::Yellow),
        38 => key("RSKU", Key::Red),
        // what you cannot use here stays where it lies
        2002 => deco("MGUN", "A", false),
        2003 => deco("LAUN", "A", false),
        2004 => deco("PLAS", "A", false),
        2005 => deco("CSAW", "A", false),
        2006 => deco("BFUG", "A", false),
        2022 => bright("PINV", "ABCD", false),
        2024 => bright("PINS", "ABCD", false),
        2025 => bright("SUIT", "A", false),
        2026 => bright("PMAP", "ABCDCB", false),
        2045 => bright("PVIS", "AB", false),
        // decorations
        2028 => bright("COLU", "A", true),
        48 => deco("ELEC", "A", true),
        43 => deco("TRE1", "A", true),
        54 => Def { r: 32.0, ..deco("TRE2", "A", true) },
        47 => deco("SMIT", "A", true),
        30 => deco("COL1", "A", true),
        31 => deco("COL2", "A", true),
        32 => deco("COL3", "A", true),
        33 => deco("COL4", "A", true),
        36 => deco("COL5", "AB", true),
        37 => deco("COL6", "A", true),
        41 => bright("CEYE", "ABCB", true),
        42 => bright("FSKU", "ABC", true),
        44 => bright("TBLU", "ABCD", true),
        45 => bright("TGRN", "ABCD", true),
        46 => bright("TRED", "ABCD", true),
        55 => bright("SMBT", "ABCD", true),
        56 => bright("SMGT", "ABCD", true),
        57 => bright("SMRT", "ABCD", true),
        34 => bright("CAND", "A", false),
        35 => bright("CBRA", "A", true),
        70 => bright("FCAN", "ABC", true),
        49 => hang("GOR1", "ABCB", true),
        50 => hang("GOR2", "A", true),
        51 => hang("GOR3", "A", true),
        52 => hang("GOR4", "A", true),
        53 => hang("GOR5", "A", true),
        63 => hang("GOR1", "ABCB", false),
        59 => hang("GOR2", "A", false),
        61 => hang("GOR3", "A", false),
        60 => hang("GOR4", "A", false),
        62 => hang("GOR5", "A", false),
        24 => deco("POL5", "A", false),
        25 => deco("POL1", "A", true),
        26 => deco("POL6", "AB", true),
        27 => deco("POL4", "A", true),
        28 => deco("POL2", "A", true),
        29 => bright("POL3", "AB", true),
        15 => deco("PLAY", "N", false),
        10 => deco("PLAY", "W", false),
        12 => deco("PLAY", "W", false),
        18 => deco("POSS", "L", false),
        19 => deco("SPOS", "L", false),
        20 => deco("TROO", "M", false),
        21 => deco("SARG", "N", false),
        22 => deco("HEAD", "L", false),
        _ => return None,
    })
}
const AMMO: [i32; 4] = [200, 50, 50, 300];
const CLIPS: [i32; 4] = [10, 4, 1, 20];

// monster frames (Doom's states): (frame, tics, action); a lower-case frame is fullbright
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    No,
    Look,
    Chase,
    Face,
    Shoot,
    Pain,
    Scream,
    Fall,
    Explode,
    Gone,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum St {
    Idle,
    Walk,
    Atk,
    Pain,
    Die,
}
type Fr = (u8, i32, Act);
struct States {
    idle: &'static [Fr],
    walk: &'static [Fr],
    atk: &'static [Fr],
    pain: &'static [Fr],
    die: &'static [Fr],
}
use Act::*;
const fn w4(t: i32) -> [Fr; 8] {
    [(b'A', t, Chase), (b'A', t, Chase), (b'B', t, Chase), (b'B', t, Chase), (b'C', t, Chase), (b'C', t, Chase), (b'D', t, Chase), (b'D', t, Chase)]
}
const IDLE2: [Fr; 2] = [(b'A', 10, Look), (b'B', 10, Look)];
const POSS: States = States {
    idle: &IDLE2,
    walk: &w4(4),
    atk: &[(b'E', 10, Face), (b'f', 8, Shoot), (b'E', 8, Face)],
    pain: &[(b'G', 3, No), (b'G', 3, Pain)],
    die: &[(b'H', 5, No), (b'I', 5, Scream), (b'J', 5, Fall), (b'K', 5, No), (b'L', -1, No)],
};
const TROO: States = States {
    idle: &IDLE2,
    walk: &w4(3),
    atk: &[(b'E', 8, Face), (b'F', 8, Face), (b'G', 6, Shoot)],
    pain: &[(b'H', 2, No), (b'H', 2, Pain)],
    die: &[(b'I', 8, No), (b'J', 8, Scream), (b'K', 6, No), (b'L', 6, Fall), (b'M', -1, No)],
};
const SARG: States = States {
    idle: &IDLE2,
    walk: &w4(2),
    atk: &[(b'E', 8, Face), (b'F', 8, Face), (b'G', 8, Shoot)],
    pain: &[(b'H', 2, No), (b'H', 2, Pain)],
    die: &[(b'I', 8, No), (b'J', 8, Scream), (b'K', 4, No), (b'L', 4, Fall), (b'M', 4, No), (b'N', -1, No)],
};
const HEAD: States = States {
    idle: &[(b'A', 10, Look)],
    walk: &[(b'A', 3, Chase)],
    atk: &[(b'B', 5, Face), (b'C', 5, Face), (b'd', 5, Shoot)],
    pain: &[(b'E', 3, No), (b'E', 3, Pain), (b'F', 6, No)],
    die: &[(b'G', 8, No), (b'H', 8, Scream), (b'I', 8, No), (b'J', 8, No), (b'K', 8, Fall), (b'L', -1, No)],
};
const BOSS: States = States {
    idle: &IDLE2,
    walk: &w4(3),
    atk: &[(b'E', 8, Face), (b'F', 8, Face), (b'G', 8, Shoot)],
    pain: &[(b'H', 2, No), (b'H', 2, Pain)],
    die: &[(b'I', 8, No), (b'J', 8, Scream), (b'K', 8, No), (b'L', 8, Fall), (b'M', 8, No), (b'N', 8, No), (b'O', -1, No)],
};
const SKUL: States = States {
    idle: &[(b'a', 10, Look), (b'b', 10, Look)],
    walk: &[(b'a', 6, Chase), (b'b', 6, Chase)],
    atk: &[(b'c', 10, Face), (b'd', 4, Shoot), (b'c', 4, No), (b'd', 4, No)],
    pain: &[(b'e', 3, No), (b'e', 3, Pain)],
    die: &[(b'f', 6, No), (b'g', 6, Scream), (b'h', 6, No), (b'i', 6, Fall), (b'J', 6, No), (b'K', 6, Gone)],
};
// BAR1 then BEXP
const BAR: States = States {
    idle: &[(b'A', 6, No), (b'B', 6, No)],
    walk: &[],
    atk: &[],
    pain: &[],
    die: &[(b'c', 5, No), (b'd', 5, Explode), (b'e', 10, No), (b'f', 10, Gone)],
};
impl States {
    fn get(&self, s: St) -> &'static [Fr] {
        match s {
            St::Idle => self.idle,
            St::Walk => self.walk,
            St::Atk => self.atk,
            St::Pain => self.pain,
            St::Die => self.die,
        }
    }
}
// what each monster sounds like (Doom's mobjinfo): when it sees you, now and then while it hunts, hurt, dying
const POSIT: &[&str] = &["DSPOSIT1", "DSPOSIT2", "DSPOSIT3"];
const PODTH: &[&str] = &["DSPODTH1", "DSPODTH2", "DSPODTH3"];
fn see(s: &str) -> &'static [&'static str] {
    match s {
        "POSS" | "SPOS" => POSIT,
        "TROO" => &["DSBGSIT1", "DSBGSIT2"],
        "SARG" => &["DSSGTSIT"],
        "HEAD" => &["DSCACSIT"],
        "BOSS" => &["DSBRSSIT"],
        _ => &[],
    }
}
fn active(s: &str) -> &'static str {
    match s {
        "POSS" | "SPOS" => "DSPOSACT",
        "TROO" => "DSBGACT",
        _ => "DSDMACT",
    }
}
fn pain_snd(s: &str) -> &'static str {
    match s {
        "POSS" | "SPOS" | "TROO" => "DSPOPAIN",
        _ => "DSDMPAIN",
    }
}
fn death(s: &str) -> &'static [&'static str] {
    match s {
        "POSS" | "SPOS" => PODTH,
        "TROO" => &["DSBGDTH1", "DSBGDTH2"],
        "SARG" => &["DSSGTDTH"],
        "HEAD" => &["DSCACDTH"],
        "BOSS" => &["DSBRSDTH"],
        "SKUL" => &["DSFIRXPL"],
        _ => &[],
    }
}
#[derive(Debug)]
pub struct MissileDef {
    pub s: &'static str,
    pub speed: f64,
    pub r: f64,
    pub dmg: i32,
    pub fly: [u8; 2],
    pub die: [u8; 3],
}
const IMP_BALL: MissileDef = MissileDef { s: "BAL1", speed: 10.0, r: 6.0, dmg: 3, fly: *b"AB", die: *b"CDE" };
const CACO_BALL: MissileDef = MissileDef { s: "BAL2", speed: 10.0, r: 6.0, dmg: 5, fly: *b"AB", die: *b"CDE" };
const BARON_BALL: MissileDef = MissileDef { s: "BAL7", speed: 15.0, r: 6.0, dmg: 8, fly: *b"AB", die: *b"CDE" };

/// a fireball's own part
#[derive(Clone, Debug)]
pub struct Missile {
    pub m: &'static MissileDef,
    pub owner: u32,
    pub mx: f64,
    pub my: f64,
    pub mz: f64,
    pub dying: i32,
    pub life: i32,
}

/// a thing in the level: a monster, an item, a decoration, a barrel, a fireball
#[derive(Clone, Debug)]
pub struct Mobj {
    pub id: u32,
    pub ty: i32,
    pub d: Def,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub angle: f64,
    pub r: f64,
    pub h: f64,
    pub sector: usize,
    pub hp: i32,
    pub ambush: bool,
    /// the frame's letter (b'A' ..)
    pub frame: u8,
    pub sprite: &'static str,
    pub tics: i32,
    pub si: usize,
    pub state: Option<St>,
    pub target: bool,
    pub movedir: i32,
    pub movecount: i32,
    pub reaction: i32,
    pub solid: bool,
    pub shootable: bool,
    pub alive: bool,
    pub bright: bool,
    pub floorz: Option<f64>,
    pub ceilz: Option<f64>,
    pub gone: bool,
    pub just_hit: bool,
    pub just_attacked: bool,
    pub melee: bool,
    pub missile: Option<Missile>,
    /// what a fireball ran into
    pub hit: Option<Who>,
    /// a fireball through the door, in the office
    pub office: bool,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub floorz: f64,
    pub mz: f64,
    pub angle: f64,
    pub mx: f64,
    pub my: f64,
    pub r: f64,
    pub h: f64,
    pub health: i32,
    pub armor: i32,
    pub armor_class: i32,
    /// bullets, shells, rockets, cells
    pub ammo: [i32; 4],
    pub max: [i32; 4],
    /// blue, yellow, red
    pub keys: [bool; 3],
    pub viewh: f64,
    pub dviewh: f64,
    pub damagecount: i32,
    pub bonuscount: i32,
    pub dead: bool,
    pub dead_at: i32,
    pub sector: usize,
    pub kills: i32,
    pub bob: f64,
    pub secret: i32,
    pub hurt: i32,
}

/// a shot's puff (or blood), rising
#[derive(Clone, Debug)]
pub struct Puff {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub t: i32,
    pub blood: bool,
}

/// the mysterious door, seen from the level: its middle on the wall, `n` into the level, `t` along it (n turned left)
#[derive(Clone, Debug)]
pub struct Portal {
    pub cx: f64,
    pub cy: f64,
    pub nx: f64,
    pub ny: f64,
    pub tx: f64,
    pub ty: f64,
    pub w: f64,
    pub h: f64,
    /// the office's height (its ceiling)
    pub height: f64,
    pub floor: f64,
    pub lines: Vec<usize>,
    pub sector: usize,
    /// how high it stands open now (fps.rs keeps it)
    pub open: f64,
    /// the office in units, for what is inside it: the door's x, its width, its depth, its wall
    pub office: [f64; 4],
}
/// where the door may go: {cx, cy} its middle on the wall, {nx, ny} into the level, the lines it is in, the sector in front of it
#[derive(Clone, Debug)]
pub struct Spot {
    pub cx: f64,
    pub cy: f64,
    pub nx: f64,
    pub ny: f64,
    pub lines: Vec<usize>,
    pub sector: usize,
    pub floor: f64,
}

/// the player's turn: turn in radians
#[derive(Clone, Copy, Debug, Default)]
pub struct Cmd {
    pub forward: f64,
    pub side: f64,
    pub run: bool,
    pub turn: f64,
    pub use_: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DoorMode {
    Raise,
    Open,
    Close,
}
#[derive(Clone, Copy, Debug)]
enum Mover {
    Door { dir: i32, top: f64, speed: f64, wait: i32, mode: DoorMode },
    Plat { dir: i32, low: f64, high: f64, speed: f64, wait: i32, next: i32 },
    Floor { to: f64, speed: f64 },
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum How {
    Use,
    Walk,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Does {
    Door(DoorMode),
    Plat,
    Floor(FloorTo),
    Exit,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum FloorTo {
    Lowest,
    Highest,
    Highest8,
}
#[derive(Clone, Copy, Debug)]
struct Special {
    how: How,
    does: Does,
    manual: bool,
    monster: bool,
    key: Option<Key>,
    once: bool,
    fast: bool,
}
// line specials: how they are set off (use, walk), what they do, once or again
fn special(sp: i32) -> Option<Special> {
    use DoorMode::*;
    use How::*;
    let s = |how, does| Special { how, does, manual: false, monster: false, key: None, once: false, fast: false };
    let man = |how, m, key, once, fast| Special { manual: true, key, once, fast, ..s(how, Does::Door(m)) };
    Some(match sp {
        1 => Special { monster: true, ..man(Use, Raise, None, false, false) },
        26 => man(Use, Raise, Some(Key::Blue), false, false),
        27 => man(Use, Raise, Some(Key::Yellow), false, false),
        28 => man(Use, Raise, Some(Key::Red), false, false),
        31 => man(Use, Open, None, true, false),
        32 => man(Use, Open, Some(Key::Blue), true, false),
        33 => man(Use, Open, Some(Key::Red), true, false),
        34 => man(Use, Open, Some(Key::Yellow), true, false),
        117 => man(Use, Raise, None, false, true),
        118 => man(Use, Open, None, true, true),
        2 => Special { once: true, ..s(Walk, Does::Door(Open)) },
        3 => Special { once: true, ..s(Walk, Does::Door(Close)) },
        4 => Special { once: true, monster: true, ..s(Walk, Does::Door(Raise)) },
        86 => s(Walk, Does::Door(Open)),
        90 => s(Walk, Does::Door(Raise)),
        29 => Special { once: true, ..s(Use, Does::Door(Raise)) },
        63 => s(Use, Does::Door(Raise)),
        103 => Special { once: true, ..s(Use, Does::Door(Open)) },
        61 => s(Use, Does::Door(Open)),
        50 => Special { once: true, ..s(Use, Does::Door(Close)) },
        42 => s(Use, Does::Door(Close)),
        62 => s(Use, Does::Plat),
        88 => Special { monster: true, ..s(Walk, Does::Plat) },
        10 => Special { once: true, ..s(Walk, Does::Plat) },
        21 => Special { once: true, ..s(Use, Does::Plat) },
        120 => Special { fast: true, ..s(Walk, Does::Plat) },
        123 => Special { fast: true, ..s(Use, Does::Plat) },
        23 => Special { once: true, ..s(Use, Does::Floor(FloorTo::Lowest)) },
        38 => Special { once: true, ..s(Walk, Does::Floor(FloorTo::Lowest)) },
        82 => s(Walk, Does::Floor(FloorTo::Lowest)),
        60 => s(Use, Does::Floor(FloorTo::Lowest)),
        102 => Special { once: true, ..s(Use, Does::Floor(FloorTo::Highest)) },
        19 => Special { once: true, ..s(Walk, Does::Floor(FloorTo::Highest)) },
        36 => Special { once: true, ..s(Walk, Does::Floor(FloorTo::Highest8)) },
        11 | 51 => s(Use, Does::Exit),
        52 | 124 => s(Walk, Does::Exit),
        _ => return None,
    })
}

// Doom's animated flats and textures: a frame every 8 tics
const ANIMS: &[&[&str]] = &[
    &["NUKAGE1", "NUKAGE2", "NUKAGE3"],
    &["FWATER1", "FWATER2", "FWATER3", "FWATER4"],
    &["SWATER1", "SWATER2", "SWATER3", "SWATER4"],
    &["LAVA1", "LAVA2", "LAVA3", "LAVA4"],
    &["BLOOD1", "BLOOD2", "BLOOD3"],
    &["RROCK05", "RROCK06", "RROCK07", "RROCK08"],
    &["SLIME01", "SLIME02", "SLIME03", "SLIME04"],
    &["SLIME05", "SLIME06", "SLIME07", "SLIME08"],
    &["SLIME09", "SLIME10", "SLIME11", "SLIME12"],
    &["BLODGR1", "BLODGR2", "BLODGR3", "BLODGR4"],
    &["SLADRIP1", "SLADRIP2", "SLADRIP3"],
    &["BLODRIP1", "BLODRIP2", "BLODRIP3", "BLODRIP4"],
    &["FIREBLU1", "FIREBLU2"],
    &["ROCKRED1", "ROCKRED2", "ROCKRED3"],
    &["BFALL1", "BFALL2", "BFALL3", "BFALL4"],
    &["SFALL1", "SFALL2", "SFALL3", "SFALL4"],
    &["WFALL1", "WFALL2", "WFALL3", "WFALL4"],
    &["DBRAIN1", "DBRAIN2", "DBRAIN3", "DBRAIN4"],
    &["GSTFONT1", "GSTFONT2", "GSTFONT3"],
    &["FIRELAVA", "FIRELAV2"],
    &["FIREMAG1", "FIREMAG2", "FIREMAG3"],
];

/// a switch flipped back after a second
#[derive(Clone, Debug)]
struct Revert {
    t: i32,
    side: usize,
    part: u8,
    tex: String,
}

/// what moves: the player, or a thing (a snapshot of what check() needs)
#[derive(Clone, Copy, Debug)]
struct Body {
    who: Who,
    r: f64,
    h: f64,
    z: f64,
    missile: bool,
    mon: bool,
    fly: bool,
    owner: Option<u32>,
}
struct At {
    floor: f64,
    ceil: f64,
    sector: usize,
}
/// one end of a sight line
#[derive(Clone, Copy)]
struct Eye {
    x: f64,
    y: f64,
    z: f64,
    h: f64,
    viewh: Option<f64>,
}
pub struct Trace {
    pub t: f64,
    pub line: Option<usize>,
    pub x: f64,
    pub y: f64,
}

pub struct Level {
    pub map: Map,
    pub name: String,
    pub time: i32,
    pub mobjs: Vec<Mobj>,
    pub puffs: Vec<Puff>,
    /// once the exit is taken
    pub done: bool,
    pub kills: i32,
    pub portal: Option<Portal>,
    /// you are in the office
    pub away: bool,
    /// what was heard this tic, for fps.rs to play
    pub sounds: Vec<Heard>,
    pub start: MapThing,
    pub player: Player,
    /// you walked into or used the mysterious door this tic (fps.rs opens it)
    pub bumped: bool,
    pub rng: Rng,
    movers: Vec<Option<Mover>>,
    thinkers: Vec<Revert>,
    gx: f64,
    gy: f64,
    gw: usize,
    gh: usize,
    grid: Vec<Vec<usize>>,
    block_line: Option<usize>,
    spechit: Vec<usize>,
    next_id: u32,
    spots: u32,
    seen: RefCell<(Vec<u32>, u32)>,
    anims: HashMap<&'static str, (usize, usize)>,
}

fn cell(v: f64, o: f64) -> usize {
    ((v - o) / 128.0).floor().max(0.0) as usize
}
/// JavaScript's `%` (the remainder keeps the dividend's sign): Rust's on f64 is the same
#[inline]
fn wrap(a: f64) -> f64 {
    ((a + 3.0 * PI) % (2.0 * PI)) - PI
}
fn sgn(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}
/// 0 in front of the line (its right), as P_PointOnLineSide
pub fn l2side(l: &crate::wad::Line, x: f64, y: f64) -> i32 {
    if l.dx * (y - l.v1.y) - l.dy * (x - l.v1.x) > 0.0 { 1 } else { 0 }
}

impl Level {
    pub fn new(wad: &Wad, name: &str, seed: u64) -> Option<Level> {
        let mut map = wad.map(name)?;
        let mut rng = Rng::new(seed);
        for s in &mut map.sectors {
            s.lines.clear();
            s.light0 = s.light;
            s.floor0 = s.floor;
            s.ceil0 = s.ceil;
            s.phase = rng.rnd() & 7;
        }
        for (li, l) in map.lines.iter().enumerate() {
            for sd in [Some(l.front), l.back].into_iter().flatten() {
                let s = map.sides[sd].sector;
                if !map.sectors[s].lines.contains(&li) {
                    map.sectors[s].lines.push(li);
                }
            }
        }
        for si in 0..map.sectors.len() {
            // where a sector's sounds come from: its middle (Doom's soundorg)
            let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for &li in &map.sectors[si].lines {
                let l = &map.lines[li];
                for v in [l.v1, l.v2] {
                    x0 = x0.min(v.x);
                    x1 = x1.max(v.x);
                    y0 = y0.min(v.y);
                    y1 = y1.max(v.y);
                }
            }
            let s = &mut map.sectors[si];
            if x0 <= x1 {
                s.x = (x0 + x1) / 2.0;
                s.y = (y0 + y1) / 2.0;
            }
        }
        // a grid of the lines, 128 units a cell, for what a box or a ray meets
        let gx = map.vertexes.iter().map(|v| v.x).fold(f64::MAX, f64::min) - 8.0;
        let gy = map.vertexes.iter().map(|v| v.y).fold(f64::MAX, f64::min) - 8.0;
        let mx = map.vertexes.iter().map(|v| v.x).fold(f64::MIN, f64::max);
        let my = map.vertexes.iter().map(|v| v.y).fold(f64::MIN, f64::max);
        let gw = ((mx - gx + 16.0) / 128.0).ceil().max(1.0) as usize;
        let gh = ((my - gy + 16.0) / 128.0).ceil().max(1.0) as usize;
        let mut grid = vec![Vec::new(); gw * gh];
        for (li, l) in map.lines.iter().enumerate() {
            for cy in cell(l.y0, gy)..=cell(l.y1, gy).min(gh - 1) {
                for cx in cell(l.x0, gx)..=cell(l.x1, gx).min(gw - 1) {
                    grid[cy * gw + cx].push(li);
                }
            }
        }
        let start = map.things.iter().find(|t| t.ty == 1).cloned().unwrap_or_default();
        let nlines = map.lines.len();
        let nsec = map.sectors.len();
        let mut anims = HashMap::new();
        for (qi, q) in ANIMS.iter().enumerate() {
            for (i, n) in q.iter().enumerate() {
                anims.insert(*n, (qi, i));
            }
        }
        let player = Player {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            floorz: 0.0,
            mz: 0.0,
            angle: 0.0,
            mx: 0.0,
            my: 0.0,
            r: 16.0,
            h: 56.0,
            health: 100,
            armor: 0,
            armor_class: 0,
            ammo: [0, 20, 0, 0],
            max: AMMO,
            keys: [false; 3],
            viewh: 41.0,
            dviewh: 0.0,
            damagecount: 0,
            bonuscount: 0,
            dead: false,
            dead_at: 0,
            sector: 0,
            kills: 0,
            bob: 0.0,
            secret: 0,
            hurt: 0,
        };
        let mut lv = Level {
            map,
            name: name.to_ascii_uppercase(),
            time: 0,
            mobjs: Vec::new(),
            puffs: Vec::new(),
            done: false,
            kills: 0,
            portal: None,
            away: false,
            sounds: Vec::new(),
            start: start.clone(),
            player,
            bumped: false,
            rng,
            movers: vec![None; nsec],
            thinkers: Vec::new(),
            gx,
            gy,
            gw,
            gh,
            grid,
            block_line: None,
            spechit: Vec::new(),
            next_id: 1,
            spots: 0,
            seen: RefCell::new((vec![0; nlines], 0)),
            anims,
        };
        // the player first: the monsters look for you as they come
        let s = lv.sector_at(start.x, start.y);
        let fl = lv.map.sectors[s].floor;
        lv.player.x = start.x;
        lv.player.y = start.y;
        lv.player.z = fl;
        lv.player.floorz = fl;
        lv.player.angle = start.angle * RAD;
        lv.player.sector = s;
        // things: monsters and items for skill 3 (Hurt me plenty), not the multiplayer ones
        let things = lv.map.things.clone();
        for t in things {
            if t.ty <= 4 || t.ty == 11 || t.flags & 16 != 0 || t.flags & 2 == 0 {
                continue;
            }
            if let Some(d) = def(t.ty) {
                lv.spawn(t.ty, d, t.x, t.y, t.angle * RAD, t.flags & 8 != 0);
            }
        }
        Some(lv)
    }
    /// the first level there is: E1M1 (Doom), else MAP01 (Doom II)
    pub fn first(wad: &Wad) -> Option<&'static str> {
        if wad.has("E1M1") {
            Some("E1M1")
        } else if wad.has("MAP01") {
            Some("MAP01")
        } else {
            None
        }
    }

    fn snd(&mut self, name: Option<&'static str>, x: f64, y: f64, o: Origin) {
        if let Some(name) = name {
            self.sounds.push(Heard { name, x, y, o });
        }
    }
    fn snd_who(&mut self, name: Option<&'static str>, who: Who) {
        let (x, y, o) = match who {
            Who::Player => (self.player.x, self.player.y, Origin::Player),
            Who::Mobj(id) => match self.find(id) {
                Some(i) => (self.mobjs[i].x, self.mobjs[i].y, Origin::Mobj(id)),
                None => return,
            },
        };
        self.snd(name, x, y, o);
    }
    fn snd_sector(&mut self, name: &'static str, s: usize) {
        let (x, y) = (self.map.sectors[s].x, self.map.sectors[s].y);
        self.snd(Some(name), x, y, Origin::Sector(s));
    }
    fn pick(&mut self, a: &'static [&'static str]) -> Option<&'static str> {
        if a.is_empty() { None } else { Some(a[self.rng.below(a.len())]) }
    }
    pub fn find(&self, id: u32) -> Option<usize> {
        self.mobjs.iter().position(|o| o.id == id)
    }
    pub fn lines_near(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<usize> {
        let mut out = Vec::new();
        let mut seen = self.seen.borrow_mut();
        seen.1 = seen.1.wrapping_add(1);
        if seen.1 == 0 {
            seen.0.fill(0);
            seen.1 = 1;
        }
        let ep = seen.1;
        let (cy1, cx1) = (cell(y1, self.gy).min(self.gh - 1), cell(x1, self.gx).min(self.gw - 1));
        for cy in cell(y0, self.gy)..=cy1 {
            for cx in cell(x0, self.gx)..=cx1 {
                for &l in &self.grid[cy * self.gw + cx] {
                    if seen.0[l] != ep {
                        seen.0[l] = ep;
                        out.push(l);
                    }
                }
            }
        }
        out
    }
    pub fn anim<'a>(&self, name: &'a str) -> &'a str {
        match self.anims.get(name) {
            Some(&(q, i)) => {
                let q = ANIMS[q];
                q[(i + (self.time / 8) as usize) % q.len()]
            }
            None => name,
        }
    }
    fn front_sector(&self, l: usize) -> usize {
        self.map.sides[self.map.lines[l].front].sector
    }
    fn back_sector(&self, l: usize) -> Option<usize> {
        self.map.lines[l].back.map(|b| self.map.sides[b].sector)
    }

    // ---- the BSP: which subsector (and sector) a point is in
    /// 0 the front (right) side, as Doom's R_PointOnSide
    pub fn side(n: &crate::wad::Node, x: f64, y: f64) -> usize {
        if (x - n.x) * n.dy - (y - n.y) * n.dx > 0.0 { 0 } else { 1 }
    }
    pub fn subsector(&self, x: f64, y: f64) -> usize {
        let m = &self.map;
        if m.nodes.is_empty() {
            return 0;
        }
        let mut n = m.nodes.len() - 1;
        loop {
            let node = &m.nodes[n];
            let k = node.kids[Level::side(node, x, y)];
            if k & 0x8000 != 0 {
                return (k & 0x7fff) as usize;
            }
            n = k as usize;
        }
    }
    pub fn sector_at(&self, x: f64, y: f64) -> usize {
        self.map.ssectors.get(self.subsector(x, y)).map_or(0, |s| s.sector)
    }

    fn spawn(&mut self, ty: i32, d: Def, x: f64, y: f64, angle: f64, ambush: bool) -> u32 {
        let s = self.sector_at(x, y);
        let sec = &self.map.sectors[s];
        let z = if d.hang { sec.ceil - 64.0 } else if d.fly { sec.floor + 24.0 } else { sec.floor };
        let id = self.next_id;
        self.next_id += 1;
        let frame = d.frames.as_bytes().first().copied().unwrap_or(b'A');
        self.mobjs.push(Mobj {
            id,
            ty,
            d,
            x,
            y,
            z,
            angle,
            r: d.r,
            h: d.h,
            sector: s,
            hp: d.hp,
            ambush,
            frame,
            sprite: d.s,
            tics: 0,
            si: 0,
            state: if d.mon || d.barrel { Some(St::Idle) } else { None },
            target: false,
            movedir: -1,
            movecount: 0,
            reaction: 8,
            solid: d.solid,
            shootable: d.shootable,
            alive: true,
            bright: d.bright,
            floorz: None,
            ceilz: None,
            gone: false,
            just_hit: false,
            just_attacked: false,
            melee: false,
            missile: None,
            hit: None,
            office: false,
        });
        if d.mon || d.barrel {
            let i = self.mobjs.len() - 1;
            self.set_state(i, St::Idle, 0);
        }
        id
    }
    /// a thing put down (a torch by the mysterious door)
    pub fn place(&mut self, ty: i32, x: f64, y: f64) -> Option<u32> {
        def(ty).map(|d| self.spawn(ty, d, x, y, 0.0, false))
    }
    /// in the map, not in the void beyond its walls: on the inside of every seg of its subsector
    pub fn inside(&self, x: f64, y: f64) -> bool {
        let Some(ss) = self.map.ssectors.get(self.subsector(x, y)) else { return false };
        for i in 0..ss.n {
            let Some(sg) = self.map.segs.get(ss.first + i) else { continue };
            if (sg.v2.x - sg.v1.x) * (y - sg.v1.y) - (sg.v2.y - sg.v1.y) * (x - sg.v1.x) > 0.0 {
                return false;
            }
        }
        true
    }
    fn states(o: &Mobj) -> &'static States {
        if o.d.barrel {
            return &BAR;
        }
        match o.d.s {
            "TROO" => &TROO,
            "SARG" => &SARG,
            "HEAD" => &HEAD,
            "BOSS" => &BOSS,
            "SKUL" => &SKUL,
            _ => &POSS,
        }
    }
    fn set_state(&mut self, i: usize, name: St, si: usize) {
        let seq = Level::states(&self.mobjs[i]).get(name);
        let Some(&(f, tics, act)) = seq.get(si) else { return };
        let o = &mut self.mobjs[i];
        o.state = Some(name);
        o.si = si;
        o.bright = f.is_ascii_lowercase() || o.d.bright;
        o.frame = f.to_ascii_uppercase();
        o.sprite = if o.d.barrel && name == St::Die { "BEXP" } else { o.d.s };
        o.tics = if tics < 0 { -1 } else { tics };
        if act != No {
            self.act(i, act);
        }
    }
    fn next_state(&mut self, i: usize) {
        let o = &self.mobjs[i];
        let Some(st) = o.state else { return };
        let seq = Level::states(o).get(st);
        if o.si + 1 < seq.len() {
            self.set_state(i, st, o.si + 1);
        } else if st == St::Die {
            self.mobjs[i].tics = -1;
        } else {
            self.set_state(i, if st == St::Idle { St::Idle } else { St::Walk }, 0);
        }
    }

    // ---- the mysterious door (fps.rs sets portal): which side of its wall a point is on (> 0 the level's), how
    // far along it from its middle
    pub fn beyond(&self, x: f64, y: f64) -> f64 {
        self.portal.as_ref().map_or(0.0, |p| (x - p.cx) * p.nx + (y - p.cy) * p.ny)
    }
    pub fn along(&self, x: f64, y: f64) -> f64 {
        self.portal.as_ref().map_or(0.0, |p| (x - p.cx) * p.tx + (y - p.cy) * p.ty)
    }
    /// a line on the door's wall or behind it
    fn office_side(&self, l: usize) -> bool {
        let l = &self.map.lines[l];
        self.beyond(l.v1.x, l.v1.y) <= 0.01 && self.beyond(l.v2.x, l.v2.y) <= 0.01
    }
    fn portal_open(&self) -> f64 {
        self.portal.as_ref().map_or(0.0, |p| p.open)
    }
    fn portal_has(&self, l: usize) -> bool {
        self.portal.as_ref().is_some_and(|p| p.lines.contains(&l))
    }
    /// a point of the level in the office's terms, inside the office
    pub fn portal_inside(&self, x: f64, y: f64) -> bool {
        let Some(p) = &self.portal else { return false };
        let (rx, ry) = (x - p.cx, y - p.cy);
        let [door_x, w, h, wall] = p.office;
        let (ox, oy) = (door_x + rx * p.tx + ry * p.ty, h + rx * p.nx + ry * p.ny);
        ox > 0.0 && ox < w && oy > wall && oy < h
    }

    fn body(&self, who: Who) -> Body {
        match who {
            Who::Player => Body { who, r: self.player.r, h: self.player.h, z: self.player.z, missile: false, mon: false, fly: false, owner: None },
            Who::Mobj(id) => {
                let o = &self.mobjs[self.find(id).expect("a mobj")];
                Body { who, r: o.r, h: o.h, z: o.z, missile: o.missile.is_some(), mon: o.d.mon, fly: o.d.fly, owner: o.missile.as_ref().map(|m| m.owner) }
            }
        }
    }

    // ---- moving: can a thing of radius r stand at x, y; where its floor and ceiling are there. What stopped it is
    // left in block_line (for the slide); the special lines it touched in spechit (a monster opens a door that way)
    fn check(&mut self, o: &Body, x: f64, y: f64) -> Option<At> {
        let r = o.r;
        self.block_line = None;
        self.spechit.clear();
        let player = o.who == Who::Player;
        // through the open door: under its lintel, between its jambs; the lines on the office's side are not there
        let mut thru = false;
        if let Some(p) = &self.portal {
            if p.open > 0.0 && (if player { !self.away } else { o.missile && self.away }) {
                let b = self.beyond(x, y);
                let k = r * (p.nx.abs() + p.ny.abs());
                let fits = self.along(x, y).abs() + r <= p.w / 2.0 && if o.missile { o.z >= p.floor && o.z + 8.0 <= p.floor + p.open } else { p.open >= o.h };
                thru = fits && b < k && b > -(k + 16.0);
            }
        }
        let mut sec = self.sector_at(x, y);
        if thru && self.beyond(x, y) < 0.0 {
            sec = self.portal.as_ref().unwrap().sector;
        }
        let s = &self.map.sectors[sec];
        let mut floor = s.floor;
        let mut ceil = if thru { s.ceil.min(self.portal.as_ref().unwrap().floor + self.portal_open()) } else { s.ceil };
        let mut drop = s.floor;
        let (mut fl, mut cl) = (None, None);
        for li in self.lines_near(x - r, y - r, x + r, y + r) {
            let l = &self.map.lines[li];
            if x + r <= l.x0 || x - r >= l.x1 || y + r <= l.y0 || y - r >= l.y1 {
                continue;
            }
            if Level::box_side(l, x, y, r) != -1 {
                continue; // the box is wholly on one side: it does not touch the line
            }
            if thru && self.office_side(li) {
                continue;
            }
            let Some(back) = l.back else {
                self.block_line = Some(li); // a one-sided line: a wall
                return None;
            };
            if l.flags & 1 != 0 && !o.missile {
                self.block_line = Some(li); // blocking
                return None;
            }
            if o.mon && l.flags & 2 != 0 {
                self.block_line = Some(li); // blocks monsters
                return None;
            }
            if l.special != 0 {
                self.spechit.push(li);
            }
            let (fs, bs) = (&self.map.sectors[self.map.sides[l.front].sector], &self.map.sectors[self.map.sides[back].sector]);
            let (f, c) = (fs.floor.max(bs.floor), fs.ceil.min(bs.ceil));
            if f > floor {
                floor = f;
                fl = Some(li);
            }
            if c < ceil {
                ceil = c;
                cl = Some(li);
            }
            drop = drop.min(fs.floor).min(bs.floor);
        }
        if ceil - floor < o.h && !o.missile {
            self.block_line = cl.or(fl);
            return None;
        }
        if !o.missile && !o.fly && floor - o.z > 24.0 {
            self.block_line = fl; // too high a step
            return None;
        }
        if o.mon && !o.fly && floor - drop > 24.0 {
            self.block_line = fl; // a monster does not walk off a ledge
            return None;
        }
        if o.missile && (o.z < floor || o.z + 8.0 > ceil) {
            return None;
        }
        // solid things: as tall as anything (vanilla Doom's)
        let blocker = self.mobjs.iter().find(|t| Who::Mobj(t.id) != o.who && t.solid && Some(t.id) != o.owner && (t.x - x).abs() < t.r + r && (t.y - y).abs() < t.r + r).map(|t| t.id);
        if let Some(t) = blocker {
            if let (true, Who::Mobj(id)) = (o.missile, o.who) {
                if let Some(i) = self.find(id) {
                    self.mobjs[i].hit = Some(Who::Mobj(t));
                }
            }
            return None;
        }
        let p = &self.player;
        if !player && !o.missile && !p.dead && !self.away && (p.x - x).abs() < p.r + r && (p.y - y).abs() < p.r + r {
            return None;
        }
        if o.missile && self.hits_player(o.z, o.r, x, y) {
            if let Who::Mobj(id) = o.who {
                if let Some(i) = self.find(id) {
                    self.mobjs[i].hit = Some(Who::Player);
                }
            }
            return None;
        }
        Some(At { floor, ceil, sector: sec })
    }
    fn hits_player(&self, z: f64, r: f64, x: f64, y: f64) -> bool {
        let p = &self.player;
        !p.dead && (p.x - x).abs() < p.r + r && (p.y - y).abs() < p.r + r && z < p.z + p.h && z + 8.0 > p.z
    }
    /// -1 if the box straddles the line
    fn box_side(l: &crate::wad::Line, x: f64, y: f64, r: f64) -> i32 {
        let s = |px: f64, py: f64| l.dx * (py - l.v1.y) - l.dy * (px - l.v1.x) > 0.0;
        let (a, b, c, d) = (s(x - r, y - r), s(x + r, y - r), s(x - r, y + r), s(x + r, y + r));
        if a == b && b == c && c == d { a as i32 } else { -1 }
    }
    fn try_move(&mut self, who: Who, x: f64, y: f64) -> bool {
        let b = self.body(who);
        let Some(at) = self.check(&b, x, y) else { return false };
        match who {
            Who::Player => {
                let p = &mut self.player;
                let (ox, oy) = (p.x, p.y);
                p.x = x;
                p.y = y;
                p.sector = at.sector;
                p.floorz = at.floor;
                if p.z < at.floor {
                    // up a step at once (the view follows); down: you fall
                    p.dviewh = (p.z - at.floor).min(0.0);
                    p.z = at.floor;
                }
                self.cross_lines(ox, oy);
            }
            Who::Mobj(id) => {
                let i = self.find(id).unwrap();
                let o = &mut self.mobjs[i];
                o.x = x;
                o.y = y;
                o.sector = at.sector;
                o.floorz = Some(at.floor);
                o.ceilz = Some(at.ceil);
                if o.missile.is_none() && !o.d.fly {
                    o.z = at.floor;
                }
            }
        }
        true
    }
    // Doom's slide: the whole move, else along the wall that stopped it, else along x or y. A long move goes in halves
    fn slide(&mut self, dx: f64, dy: f64) {
        if dx.hypot(dy) > 15.0 {
            self.slide(dx / 2.0, dy / 2.0);
            if self.player.mx != 0.0 || self.player.my != 0.0 {
                self.slide(dx / 2.0, dy / 2.0);
            }
            return;
        }
        let (px, py) = (self.player.x, self.player.y);
        if self.try_move(Who::Player, px + dx, py + dy) {
            return;
        }
        let bl = self.block_line;
        if let Some(l) = bl {
            if self.portal_has(l) && self.along(px, py).abs() < self.portal.as_ref().unwrap().w / 2.0 {
                self.bumped = true; // walked into the mysterious door: it opens
            }
            let l = &self.map.lines[l];
            let len = l.dx.hypot(l.dy);
            let len = if len == 0.0 { 1.0 } else { len };
            let (tx, ty) = (l.dx / len, l.dy / len);
            let k = dx * tx + dy * ty;
            if k.abs() > 0.01 && self.try_move(Who::Player, px + tx * k, py + ty * k) {
                self.player.mx = tx * k;
                self.player.my = ty * k;
                return;
            }
        }
        if self.try_move(Who::Player, px + dx, py) {
            self.player.my = 0.0;
            return;
        }
        if self.try_move(Who::Player, px, py + dy) {
            self.player.mx = 0.0;
            return;
        }
        self.player.mx = 0.0;
        self.player.my = 0.0;
    }
    // walking over a line with a special
    fn cross_lines(&mut self, ox: f64, oy: f64) {
        let (x, y) = (self.player.x, self.player.y);
        for li in self.lines_near(ox.min(x) - 1.0, oy.min(y) - 1.0, ox.max(x) + 1.0, oy.max(y) + 1.0) {
            let l = &self.map.lines[li];
            if l.special == 0 || !special(l.special).is_some_and(|s| s.how == How::Walk) {
                continue;
            }
            let s0 = l.dx * (oy - l.v1.y) - l.dy * (ox - l.v1.x);
            let s1 = l.dx * (y - l.v1.y) - l.dy * (x - l.v1.x);
            if (s0 > 0.0) == (s1 > 0.0) {
                continue;
            }
            let t0 = (x - ox) * (oy - l.v1.y) - (y - oy) * (ox - l.v1.x);
            let t1 = (x - ox) * (oy - l.v2.y) - (y - oy) * (ox - l.v2.x);
            if (t0 > 0.0) == (t1 > 0.0) {
                continue; // crossed the line's extension, not the line
            }
            self.trigger(li, Who::Player, How::Walk);
        }
    }

    // ---- what you see along a ray: the first line that stops it (one-sided, or an opening that does not take the
    // height z), within dist; for the shotgun and sight. Through the open door the lines on the office's side are not there
    pub fn trace(&self, x: f64, y: f64, ang: f64, dist: f64, z_at: &dyn Fn(f64) -> f64, office: bool) -> Trace {
        let (dx, dy) = (ang.cos(), ang.sin());
        let (x2, y2) = (x + dx * dist, y + dy * dist);
        let mut thru = false;
        if office {
            if let Some(p) = &self.portal {
                if p.open > 0.0 {
                    let (b0, b1) = (self.beyond(x, y), self.beyond(x2, y2));
                    if (b0 > 0.0) != (b1 > 0.0) {
                        let tc = b0 / (b0 - b1) * dist;
                        let z = z_at(tc);
                        thru = self.along(x + dx * tc, y + dy * tc).abs() < p.w / 2.0 && z > p.floor && z < p.floor + p.open;
                    }
                }
            }
        }
        let (mut best, mut line) = (dist, None);
        for li in self.lines_near(x.min(x2), y.min(y2), x.max(x2), y.max(y2)) {
            if thru && self.office_side(li) {
                continue;
            }
            let l = &self.map.lines[li];
            let den = dx * l.dy - dy * l.dx;
            if den.abs() < 1e-9 {
                continue;
            }
            let t = ((l.v1.x - x) * l.dy - (l.v1.y - y) * l.dx) / den;
            let u = ((l.v1.x - x) * dy - (l.v1.y - y) * dx) / den;
            if t <= 0.01 || t >= best || !(0.0..=1.0).contains(&u) {
                continue;
            }
            if let Some(b) = l.back {
                let (fs, bs) = (&self.map.sectors[self.map.sides[l.front].sector], &self.map.sectors[self.map.sides[b].sector]);
                let z = z_at(t);
                if z > fs.floor.max(bs.floor) && z < fs.ceil.min(bs.ceil) {
                    continue; // through the opening
                }
            }
            best = t;
            line = Some(li);
        }
        Trace { t: best, line, x: x + dx * best, y: y + dy * best }
    }
    fn eye(&self, w: Who) -> Option<Eye> {
        match w {
            Who::Player => Some(Eye { x: self.player.x, y: self.player.y, z: self.player.z, h: self.player.h, viewh: Some(self.player.viewh) }),
            Who::Mobj(id) => self.find(id).map(|i| {
                let o = &self.mobjs[i];
                Eye { x: o.x, y: o.y, z: o.z, h: o.h, viewh: None }
            }),
        }
    }
    /// can a see b: nothing in between closes off the line from eye to middle
    fn sight(&self, a: Who, b: Who) -> bool {
        let (Some(ea), Some(eb)) = (self.eye(a), self.eye(b)) else { return false };
        let (dx, dy) = (eb.x - ea.x, eb.y - ea.y);
        let d = dx.hypot(dy);
        let d = if d == 0.0 { 1.0 } else { d };
        let z0 = ea.z + ea.viewh.unwrap_or(ea.h * 0.75);
        let z1 = eb.z + eb.h / 2.0;
        let office = self.away && (a == Who::Player || b == Who::Player);
        self.trace(ea.x, ea.y, dy.atan2(dx), d, &|t| z0 + (z1 - z0) * t / d, office).t >= d - 1.0
    }

    // ---- specials
    fn trigger(&mut self, li: usize, by: Who, how: How) -> bool {
        let sp = self.map.lines[li].special;
        let Some(def) = special(sp) else { return false };
        if def.how != how {
            return false;
        }
        if let (Some(k), Who::Player) = (def.key, by) {
            if !self.player.keys[k as usize] {
                self.snd_who(Some("DSOOF"), by); // a locked door
                return false;
            }
        }
        if let Who::Mobj(id) = by {
            if self.find(id).is_some_and(|i| self.mobjs[i].d.mon) && !def.monster {
                return false;
            }
        }
        let mut did = false;
        let tag = self.map.lines[li].tag;
        match def.does {
            Does::Door(kind) => {
                let secs = if def.manual { self.back_sector(li).into_iter().collect() } else { self.tagged(tag) };
                for s in secs {
                    did = self.door(s, kind, def.fast) || did;
                }
            }
            Does::Plat => {
                for s in self.tagged(tag) {
                    did = self.plat(s, def.fast) || did;
                }
            }
            Does::Floor(to) => {
                for s in self.tagged(tag) {
                    did = self.floor_move(s, to) || did;
                }
            }
            Does::Exit => {
                self.done = true;
                did = true;
            }
        }
        let exit = def.does == Does::Exit;
        if def.once && (did || exit) {
            self.map.lines[li].special = 0;
        }
        if how == How::Use && did {
            self.switch_tex(li, !def.once);
            if exit {
                self.snd_who(Some("DSSWTCHX"), by);
            }
        }
        did
    }
    fn tagged(&self, tag: i32) -> Vec<usize> {
        if tag == 0 {
            return Vec::new();
        }
        (0..self.map.sectors.len()).filter(|&s| self.map.sectors[s].tag == tag).collect()
    }
    fn neighbours(&self, s: usize) -> Vec<usize> {
        self.map.sectors[s]
            .lines
            .iter()
            .filter_map(|&li| {
                let l = &self.map.lines[li];
                let sd = if self.map.sides[l.front].sector == s { l.back } else { Some(l.front) };
                sd.map(|sd| self.map.sides[sd].sector)
            })
            .collect()
    }
    fn door(&mut self, s: usize, kind: DoorMode, fast: bool) -> bool {
        if let Some(m) = self.movers[s] {
            // a door going down: back up (DR doors used again)
            if let Mover::Door { dir: -1, top, speed, wait, mode } = m {
                if kind == DoorMode::Raise {
                    self.movers[s] = Some(Mover::Door { dir: 1, top, speed, wait, mode });
                    self.snd_sector(if speed > 2.0 { "DSBDOPN" } else { "DSDOROPN" }, s);
                    return true;
                }
            }
            return false;
        }
        let ns = self.neighbours(s);
        let top = if ns.is_empty() { self.map.sectors[s].ceil0 } else { ns.iter().map(|&n| self.map.sectors[n].ceil).fold(f64::MAX, f64::min) } - 4.0;
        let speed = if fast { 8.0 } else { 2.0 };
        self.movers[s] = Some(if kind == DoorMode::Close {
            Mover::Door { dir: -1, top: self.map.sectors[s].ceil, speed, wait: 0, mode: kind }
        } else {
            Mover::Door { dir: 1, top, speed, wait: 0, mode: kind }
        });
        let snd = if kind == DoorMode::Close {
            if fast { "DSBDCLS" } else { "DSDORCLS" }
        } else if fast {
            "DSBDOPN"
        } else {
            "DSDOROPN"
        };
        self.snd_sector(snd, s);
        true
    }
    fn plat(&mut self, s: usize, fast: bool) -> bool {
        if self.movers[s].is_some() {
            return false;
        }
        let f = self.map.sectors[s].floor;
        let low = self.neighbours(s).iter().map(|&n| self.map.sectors[n].floor).fold(f, f64::min);
        self.movers[s] = Some(Mover::Plat { dir: -1, low, high: f, speed: if fast { 8.0 } else { 4.0 }, wait: 0, next: 0 });
        self.snd_sector("DSPSTART", s);
        true
    }
    fn floor_move(&mut self, s: usize, kind: FloorTo) -> bool {
        if self.movers[s].is_some() {
            return false;
        }
        let ns = self.neighbours(s);
        if ns.is_empty() {
            return false;
        }
        let f = self.map.sectors[s].floor;
        let hi = ns.iter().map(|&n| self.map.sectors[n].floor).fold(f64::MIN, f64::max);
        let to = match kind {
            FloorTo::Lowest => ns.iter().map(|&n| self.map.sectors[n].floor).fold(f, f64::min),
            FloorTo::Highest => hi,
            FloorTo::Highest8 => hi + 8.0,
        };
        self.movers[s] = Some(Mover::Floor { to, speed: 1.0 });
        true
    }
    fn switch_tex(&mut self, li: usize, back: bool) {
        let l = &self.map.lines[li];
        let (x, y, sd) = ((l.v1.x + l.v2.x) / 2.0, (l.v1.y + l.v2.y) / 2.0, l.front);
        self.spots += 1;
        self.snd(Some("DSSWTCHN"), x, y, Origin::Spot(self.spots));
        for part in [b'u', b'm', b'l'] {
            let side = &mut self.map.sides[sd];
            let t = match part {
                b'u' => &mut side.upper,
                b'm' => &mut side.mid,
                _ => &mut side.lower,
            };
            if let Some(rest) = t.strip_prefix("SW1") {
                let old = t.clone();
                *t = format!("SW2{rest}");
                if back {
                    self.thinkers.push(Revert { t: 35, side: sd, part, tex: old });
                }
                return;
            }
            if let Some(rest) = t.strip_prefix("SW2") {
                *t = format!("SW1{rest}");
                return;
            }
        }
    }
    fn move_sectors(&mut self) {
        for s in 0..self.map.sectors.len() {
            let Some(mut m) = self.movers[s] else { continue };
            let mut keep = true;
            match &mut m {
                Mover::Door { dir, top, speed, wait, mode } => {
                    if *wait > 0 {
                        *wait -= 1;
                        if *wait == 0 {
                            *dir = -1;
                            let snd = if *speed > 2.0 { "DSBDCLS" } else { "DSDORCLS" };
                            self.snd_sector(snd, s);
                        }
                        self.movers[s] = Some(m);
                        continue;
                    }
                    let sec = &mut self.map.sectors[s];
                    if *dir == 1 {
                        sec.ceil = top.min(sec.ceil + *speed);
                        if sec.ceil >= *top {
                            if *mode == DoorMode::Raise {
                                *wait = 150;
                                *dir = 0;
                            } else {
                                keep = false;
                            }
                        }
                    } else if *dir == -1 {
                        let nc = sec.floor.max(sec.ceil - *speed);
                        if self.crushes(s, nc, None) {
                            if *mode != DoorMode::Close {
                                *dir = 1;
                                let snd = if *speed > 2.0 { "DSBDOPN" } else { "DSDOROPN" };
                                self.snd_sector(snd, s);
                            }
                            self.movers[s] = Some(m);
                            continue;
                        }
                        let sec = &mut self.map.sectors[s];
                        sec.ceil = nc;
                        if sec.ceil <= sec.floor {
                            keep = false;
                        }
                    }
                }
                Mover::Plat { dir, low, high, speed, wait, next } => {
                    if *wait > 0 {
                        *wait -= 1;
                        if *wait == 0 {
                            *dir = if *dir == -1 { 1 } else { -1 };
                            self.snd_sector("DSPSTART", s);
                        }
                        self.movers[s] = Some(m);
                        continue;
                    }
                    if *dir == -1 {
                        let sec = &mut self.map.sectors[s];
                        sec.floor = low.max(sec.floor - *speed);
                        if sec.floor <= *low {
                            *wait = 105;
                            *dir = -1;
                            *next = 1;
                            self.snd_sector("DSPSTOP", s);
                        }
                    } else {
                        let nf = high.min(self.map.sectors[s].floor + *speed);
                        let ceil = self.map.sectors[s].ceil;
                        if self.crushes(s, ceil, Some(nf)) {
                            *dir = -1;
                            self.movers[s] = Some(m);
                            continue;
                        }
                        self.map.sectors[s].floor = nf;
                        if nf >= *high {
                            keep = false;
                            self.snd_sector("DSPSTOP", s);
                        }
                    }
                    if *next != 0 && *wait == 0 {
                        *dir = *next;
                        *next = 0;
                    }
                }
                Mover::Floor { to, speed } => {
                    let sec = &mut self.map.sectors[s];
                    sec.floor += sgn(*to - sec.floor) * speed.min((*to - sec.floor).abs());
                    if sec.floor == *to {
                        keep = false;
                        self.snd_sector("DSPSTOP", s);
                    } else if self.time % 8 == 0 {
                        self.snd_sector("DSSTNMOV", s);
                    }
                }
            }
            self.movers[s] = if keep { Some(m) } else { None };
            // what stands on a moving floor rides it (you: the highest floor under you)
            if !self.away && self.sector_at(self.player.x, self.player.y) == s {
                let b = self.body(Who::Player);
                let (px, py) = (self.player.x, self.player.y);
                let z = self.check(&b, px, py).map_or(self.map.sectors[s].floor, |a| a.floor);
                self.player.z = z;
                self.player.floorz = z;
            }
            let fl = self.map.sectors[s].floor;
            for i in 0..self.mobjs.len() {
                let o = &self.mobjs[i];
                if o.missile.is_some() || o.d.fly || o.d.hang || self.sector_at(o.x, o.y) != s {
                    continue;
                }
                self.mobjs[i].z = fl;
            }
        }
    }
    /// would something no longer fit
    fn crushes(&self, s: usize, ceil: f64, floor: Option<f64>) -> bool {
        let floor = floor.unwrap_or(self.map.sectors[s].floor);
        let p = &self.player;
        if !self.away && self.sector_at(p.x, p.y) == s && ceil - floor < p.h {
            return true;
        }
        self.mobjs.iter().any(|o| o.alive && o.missile.is_none() && o.solid && self.sector_at(o.x, o.y) == s && ceil - floor < o.h)
    }
    /// Doom's: a flicker, strobes (fast: 15 tics dark, 5 bright; slow: 35 dark), a glow
    fn lights(&mut self) {
        let t = self.time;
        for si in 0..self.map.sectors.len() {
            let sp = self.map.sectors[si].special;
            if sp == 0 || sp == 9 || sp == 7 || sp == 5 || sp == 16 {
                continue;
            }
            let l0 = self.map.sectors[si].light0;
            let low = self.neighbours(si).iter().map(|&n| self.map.sectors[n].light0).fold(l0, i32::min);
            let lo = if low == l0 { 0 } else { low };
            let hi = l0;
            let ph = if sp == 12 || sp == 13 { 0 } else { self.map.sectors[si].phase };
            match sp {
                1 => {
                    let s = &self.map.sectors[si];
                    if s.blink == 0 || s.blink - 1 <= 0 {
                        let light = if s.light == hi { lo } else { hi };
                        let blink = if light == hi { (self.rng.rnd() & 64) + 1 } else { (self.rng.rnd() & 7) + 1 };
                        let s = &mut self.map.sectors[si];
                        s.light = light;
                        s.blink = blink;
                    } else {
                        self.map.sectors[si].blink -= 1;
                    }
                }
                2 | 4 | 13 => self.map.sectors[si].light = if (t + ph) % 20 < 5 { hi } else { lo },
                3 | 12 => self.map.sectors[si].light = if (t + ph) % 40 < 5 { hi } else { lo },
                8 => {
                    let k = (t % 64) as f64 / 32.0;
                    self.map.sectors[si].light = jround(lo as f64 + (hi - lo) as f64 * if k < 1.0 { k } else { 2.0 - k }) as i32;
                }
                17 => {
                    let s = &self.map.sectors[si];
                    if s.blink == 0 || s.blink - 1 <= 0 {
                        let light = (lo + 16).max(hi - (self.rng.rnd() & 3) * 16);
                        let s = &mut self.map.sectors[si];
                        s.light = light;
                        s.blink = 4;
                    } else {
                        self.map.sectors[si].blink -= 1;
                    }
                }
                _ => {}
            }
        }
    }

    // ---- the player's turn. Away (in the office, fps.rs moves you there) the level goes on without you moving in it
    pub fn tic(&mut self, cmd: &Cmd) {
        if self.done {
            return;
        }
        self.time += 1;
        if !self.player.dead && !self.away {
            let p = &mut self.player;
            p.angle += cmd.turn;
            let on_ground = p.z <= p.floorz + 0.01;
            // Doom's thrust, in units a tic
            let fw = cmd.forward * if cmd.run { 50.0 } else { 25.0 } / 32.0;
            let sd = cmd.side * if cmd.run { 40.0 } else { 24.0 } / 32.0;
            let (ca, sa) = (p.angle.cos(), p.angle.sin());
            if on_ground {
                p.mx += ca * fw + sa * sd;
                p.my += sa * fw - ca * sd;
            }
            let sp = p.mx.hypot(p.my);
            if sp > 30.0 {
                p.mx *= 30.0 / sp;
                p.my *= 30.0 / sp;
            }
            if p.mx != 0.0 || p.my != 0.0 {
                let (mx, my) = (p.mx, p.my);
                self.slide(mx, my);
            }
            let p = &mut self.player;
            let mut oof = false;
            if p.z > p.floorz {
                // Doom's gravity; a hard landing squats the view
                p.mz = if p.mz != 0.0 { p.mz - 1.0 } else { -2.0 };
                p.z += p.mz;
                if p.z <= p.floorz {
                    if p.mz < -8.0 {
                        p.dviewh = p.mz / 8.0;
                        oof = true;
                    }
                    p.z = p.floorz;
                    p.mz = 0.0;
                }
            }
            if on_ground {
                p.mx *= 0.90625; // friction, 0xE800 (none in the air)
                p.my *= 0.90625;
            }
            if p.mx.abs() < 0.0625 && p.my.abs() < 0.0625 && cmd.forward == 0.0 && cmd.side == 0.0 {
                p.mx = 0.0;
                p.my = 0.0;
            }
            p.bob = 16f64.min((p.mx * p.mx + p.my * p.my) / 4.0);
            if p.dviewh != 0.0 {
                p.dviewh *= 0.75;
                if p.dviewh.abs() < 0.5 {
                    p.dviewh = 0.0;
                }
            }
            if oof {
                self.snd_who(Some("DSOOF"), Who::Player);
            }
            if cmd.use_ {
                self.use_line();
            }
            let s = self.sector_at(self.player.x, self.player.y);
            self.player.sector = s;
            let sp = self.map.sectors[s].special;
            if sp == 9 {
                self.map.sectors[s].special = 0;
                self.player.secret += 1;
            }
            if matches!(sp, 7 | 5 | 16 | 4) && self.player.z <= self.map.sectors[s].floor && self.time % 32 == 0 {
                self.damage(Who::Player, if sp == 7 { 5 } else if sp == 5 { 10 } else { 20 }, None);
            }
            self.pickup();
        } else if self.player.dead {
            let p = &mut self.player;
            p.viewh = 6f64.max(p.viewh - 1.0); // the view falls to the floor
            p.mx *= 0.9;
            p.my *= 0.9;
        }
        let p = &mut self.player;
        if p.damagecount > 0 {
            p.damagecount -= 1;
        }
        if p.bonuscount > 0 {
            p.bonuscount -= 1;
        }
        self.move_sectors();
        self.lights();
        for mut th in std::mem::take(&mut self.thinkers) {
            th.t -= 1;
            if th.t > 0 {
                self.thinkers.push(th);
            } else {
                let sd = &mut self.map.sides[th.side];
                match th.part {
                    b'u' => sd.upper = th.tex,
                    b'm' => sd.mid = th.tex,
                    _ => sd.lower = th.tex,
                }
            }
        }
        let n = self.mobjs.len();
        for i in 0..n {
            self.think(i);
        }
        self.mobjs.retain(|o| !o.gone);
        self.puffs.retain_mut(|f| {
            f.t += 1;
            f.t < 16
        });
    }
    /// the line in front of you, within 64 units, used from its front
    fn use_line(&mut self) {
        let p = &self.player;
        let (ang, px, py) = (p.angle, p.x, p.y);
        let (x2, y2) = (px + ang.cos() * 64.0, py + ang.sin() * 64.0);
        let (mut best, mut hit, mut door) = (1e9, None, false);
        let (dx, dy) = (ang.cos(), ang.sin());
        for li in self.lines_near(px.min(x2), py.min(y2), px.max(x2), py.max(y2)) {
            let l = &self.map.lines[li];
            let den = dx * l.dy - dy * l.dx;
            if den.abs() < 1e-9 {
                continue;
            }
            let t = ((l.v1.x - px) * l.dy - (l.v1.y - py) * l.dx) / den;
            let u = ((l.v1.x - px) * dy - (l.v1.y - py) * dx) / den;
            if !(0.0..=64.0).contains(&t) || !(0.0..=1.0).contains(&u) || t >= best {
                continue;
            }
            if self.portal_has(li) && self.along(px + dx * t, py + dy * t).abs() < self.portal.as_ref().unwrap().w / 2.0 {
                best = t;
                door = true;
                hit = None;
                continue;
            }
            if l.special != 0 || l.back.is_none() {
                best = t;
                hit = Some(li);
                door = false;
            } else {
                let (fs, bs) = (self.front_sector(li), self.back_sector(li).unwrap());
                let (fs, bs) = (&self.map.sectors[fs], &self.map.sectors[bs]);
                if fs.ceil.min(bs.ceil) - fs.floor.max(bs.floor) <= 0.0 {
                    best = t;
                    hit = Some(li);
                    door = false;
                }
            }
        }
        if door {
            self.bumped = true;
            return;
        }
        if let Some(li) = hit {
            let l = &self.map.lines[li];
            if l.special != 0 && (l2side(l, px, py) == 0 || l.special == 124) {
                self.trigger(li, Who::Player, How::Use);
                return;
            }
            self.snd_who(Some("DSNOWAY"), Who::Player); // a wall: the grunt
        }
    }
    fn pickup(&mut self) {
        for i in 0..self.mobjs.len() {
            let o = &self.mobjs[i];
            let d = o.d;
            let p = &self.player;
            if !d.pick || o.gone {
                continue;
            }
            if (o.x - p.x).abs() > p.r + o.r || (o.y - p.y).abs() > p.r + o.r || o.z - p.z > p.h || o.z - p.z < -8.0 {
                continue;
            }
            let p = &mut self.player;
            let mut took = false;
            if d.health != 0 && (d.always || p.health < d.max) {
                p.health = d.max.min(p.health + d.health);
                took = true;
            }
            if d.armor != 0 && (if d.always { p.armor < d.max } else { p.armor < d.armor }) {
                p.armor = if d.always { d.max.min(p.armor + d.armor) } else { d.armor };
                p.armor_class = p.armor_class.max(1).max(d.cls.max(1));
                took = true;
            }
            if d.backpack {
                // twice as much of everything, and a clip of each
                if p.max[0] == AMMO[0] {
                    for k in 0..4 {
                        p.max[k] = AMMO[k] * 2;
                    }
                }
                for k in 0..4 {
                    p.ammo[k] = p.max[k].min(p.ammo[k] + CLIPS[k]);
                }
                took = true;
            }
            if let Some(a) = d.ammo {
                let a = a as usize;
                if p.ammo[a] < p.max[a] {
                    p.ammo[a] = p.max[a].min(p.ammo[a] + d.n);
                    took = true;
                }
            }
            if let Some(k) = d.key {
                if !p.keys[k as usize] {
                    p.keys[k as usize] = true;
                    took = true;
                }
            }
            if took {
                self.mobjs[i].gone = true;
                p.bonuscount = (p.bonuscount + 6).min(40);
                let snd = if d.s == "SHOT" {
                    "DSWPNUP"
                } else if d.s == "SOUL" || d.s == "PSTR" {
                    "DSGETPOW"
                } else {
                    "DSITEMUP"
                };
                self.snd_who(Some(snd), Who::Player);
            }
        }
    }

    // ---- shooting: the shotgun's seven pellets from the player, Doom's spread
    pub fn fire(&mut self) -> bool {
        let p = &self.player;
        if p.dead || p.ammo[1] <= 0 {
            return false;
        }
        let eye = p.z + p.viewh - 8.0;
        self.player.ammo[1] -= 1;
        let target = self.aim();
        for _ in 0..7 {
            let (a, b) = (self.rng.rnd(), self.rng.rnd());
            let ang = self.player.angle + ((a - b) as f64 / 255.0) * 5.6 * RAD;
            let dmg = 5 * ((self.rng.rnd() % 3) + 1);
            self.bullet(Who::Player, ang, eye, target, dmg);
        }
        self.noise();
        true
    }
    /// autoaim: the nearest thing to shoot within a few degrees of where you look
    fn aim(&self) -> Option<Who> {
        let p = &self.player;
        let (mut best, mut bd) = (None, 2048.0);
        for o in &self.mobjs {
            if !o.shootable || !o.alive {
                continue;
            }
            let (dx, dy) = (o.x - p.x, o.y - p.y);
            let d = dx.hypot(dy);
            let da = wrap(dy.atan2(dx) - p.angle).abs();
            if d < bd && da < (o.r + 4.0).atan2(d) + 4.0 * RAD && self.sight(Who::Player, Who::Mobj(o.id)) {
                best = Some(Who::Mobj(o.id));
                bd = d;
            }
        }
        best
    }
    fn xyz(&self, w: Who) -> Option<(f64, f64, f64, f64)> {
        match w {
            Who::Player => Some((self.player.x, self.player.y, self.player.z, self.player.h)),
            Who::Mobj(id) => self.find(id).map(|i| (self.mobjs[i].x, self.mobjs[i].y, self.mobjs[i].z, self.mobjs[i].h)),
        }
    }
    fn bullet(&mut self, src: Who, ang: f64, z0: f64, target: Option<Who>, dmg: i32) {
        let range = 2048.0;
        let (sx, sy, _, _) = self.xyz(src).unwrap();
        let tg = target.and_then(|t| self.xyz(t));
        let z_at = move |t: f64| match tg {
            Some((tx, ty, tz, th)) => z0 + (tz + th / 2.0 - z0) * t / 1f64.max((tx - sx).hypot(ty - sy)),
            None => z0,
        };
        let office = self.away && (src == Who::Player || target == Some(Who::Player));
        let wall = self.trace(sx, sy, ang, range, &z_at, office);
        let (mut hit, mut ht) = (None, wall.t);
        let (dx, dy) = (ang.cos(), ang.sin());
        let mut test = |who: Who, x: f64, y: f64, z: f64, h: f64, r: f64| {
            let (rx, ry) = (x - sx, y - sy);
            let t = rx * dx + ry * dy;
            if t <= 0.0 || t >= ht || (rx * dy - ry * dx).abs() > r {
                return;
            }
            let zz = z_at(t);
            if zz < z - 4.0 || zz > z + h + 4.0 {
                return;
            }
            hit = Some(who);
            ht = t;
        };
        for o in &self.mobjs {
            if Who::Mobj(o.id) == src || !o.alive || !o.shootable {
                continue;
            }
            test(Who::Mobj(o.id), o.x, o.y, o.z, o.h, o.r);
        }
        let p = &self.player;
        if src != Who::Player && !p.dead {
            test(Who::Player, p.x, p.y, p.z, p.h, p.r);
        }
        let (x, y, z) = (sx + dx * (ht - 2.0), sy + dy * (ht - 2.0), z_at(ht));
        if let Some(h) = hit {
            self.damage(h, dmg, Some(src));
            let barrel = matches!(h, Who::Mobj(id) if self.find(id).is_some_and(|i| self.mobjs[i].d.barrel));
            if !barrel && !(h == Who::Player && self.away) {
                self.puffs.push(Puff { x, y, z, t: 0, blood: true });
            }
        } else if wall.line.is_some() {
            self.puffs.push(Puff { x, y, z, t: 0, blood: false });
        }
    }
    /// a shot's sound (P_NoiseAlert): from the shooter's sector through every opening, past one sound-blocking line
    /// at most; a monster there wakes (one lying in ambush only if it sees you). In the office: the room in front of the door
    pub fn noise(&mut self) {
        let start = self.player.sector;
        let mut heard: HashMap<usize, i32> = HashMap::new();
        heard.insert(start, 0);
        let mut q = std::collections::VecDeque::from([start]);
        while let Some(s) = q.pop_front() {
            let blocks = heard[&s];
            for &li in &self.map.sectors[s].lines {
                let l = &self.map.lines[li];
                let Some(b) = l.back else { continue };
                let (fs, bs) = (self.map.sides[l.front].sector, self.map.sides[b].sector);
                let o = if fs == s { bs } else { fs };
                let (f, bk) = (&self.map.sectors[fs], &self.map.sectors[bs]);
                if f.ceil.min(bk.ceil) - f.floor.max(bk.floor) <= 0.0 {
                    continue; // a closed door
                }
                let b = blocks + if l.flags & 64 != 0 { 1 } else { 0 };
                if b > 1 || heard.get(&o).is_some_and(|&h| h <= b) {
                    continue;
                }
                heard.insert(o, b);
                q.push_back(o);
            }
        }
        for i in 0..self.mobjs.len() {
            let o = &self.mobjs[i];
            if !o.d.mon || !o.alive || o.state != Some(St::Idle) || !heard.contains_key(&o.sector) {
                continue;
            }
            if o.ambush && !self.sight(Who::Mobj(o.id), Who::Player) {
                continue;
            }
            let (id, s) = (o.id, o.d.s);
            self.mobjs[i].target = true;
            let snd = self.pick(see(s));
            self.snd_who(snd, Who::Mobj(id));
            self.set_state(i, St::Walk, 0);
        }
    }
    pub fn damage(&mut self, w: Who, mut dmg: i32, src: Option<Who>) {
        match w {
            Who::Player => {
                let time = self.time;
                let o = &mut self.player;
                if o.dead {
                    return;
                }
                if o.armor > 0 {
                    let save = o.armor.min(dmg / if o.armor_class == 2 { 2 } else { 3 });
                    o.armor -= save;
                    dmg -= save;
                }
                o.health -= dmg;
                o.damagecount = 100.min(o.damagecount + dmg);
                o.hurt = time;
                if o.health <= 0 {
                    o.health = 0;
                    o.dead = true;
                    o.dead_at = time;
                    self.snd_who(Some("DSPLDETH"), Who::Player);
                } else {
                    self.snd_who(Some("DSPLPAIN"), Who::Player);
                }
            }
            Who::Mobj(id) => {
                let Some(i) = self.find(id) else { return };
                let o = &mut self.mobjs[i];
                if !o.alive {
                    return;
                }
                o.hp -= dmg;
                if o.d.mon {
                    o.reaction = 0; // fight back at once
                    if src.is_some() {
                        o.target = true;
                    }
                }
                if o.hp <= 0 {
                    o.alive = false;
                    o.shootable = false;
                    if o.d.mon {
                        self.kills += 1;
                        self.player.kills += 1;
                        self.set_state(i, St::Die, 0);
                    } else if o.d.barrel {
                        self.set_state(i, St::Die, 0);
                    }
                    return;
                }
                if o.d.mon && self.rng.rnd() < self.mobjs[i].d.pain {
                    self.mobjs[i].just_hit = true;
                    self.set_state(i, St::Pain, 0);
                } else if self.mobjs[i].d.mon && self.mobjs[i].state == Some(St::Idle) {
                    self.set_state(i, St::Walk, 0);
                }
            }
        }
    }
    // ---- a thing's tic
    fn think(&mut self, i: usize) {
        if self.mobjs[i].missile.is_some() {
            return self.fly(i);
        }
        let time = self.time;
        let o = &mut self.mobjs[i];
        if !o.d.frames.is_empty() && !o.d.mon && !o.d.barrel {
            let f = o.d.frames.as_bytes();
            o.frame = f[(time / 6) as usize % f.len()];
            return;
        }
        if o.tics == -1 {
            return;
        }
        if o.tics > 0 {
            o.tics -= 1;
            if o.tics > 0 {
                return;
            }
        }
        self.next_state(i);
    }
    fn act(&mut self, i: usize, a: Act) {
        let o = &self.mobjs[i];
        let (id, ox, oy, s) = (o.id, o.x, o.y, o.d.s);
        let me = Who::Mobj(id);
        let (px, py, pr) = (self.player.x, self.player.y, self.player.r);
        match a {
            Look => {
                if self.player.dead {
                    return;
                }
                let ang = (py - oy).atan2(px - ox);
                let da = wrap(ang - o.angle).abs();
                if (da < PI / 2.0 || (px - ox).hypot(py - oy) < 64.0) && self.sight(me, Who::Player) {
                    self.mobjs[i].target = true;
                    let snd = self.pick(see(s));
                    self.snd_who(snd, me);
                    self.set_state(i, St::Walk, 0);
                }
            }
            Chase => {
                // Doom's A_Chase
                if self.mobjs[i].reaction > 0 {
                    self.mobjs[i].reaction -= 1;
                }
                if self.rng.rnd() < 3 {
                    self.snd_who(Some(active(s)), me);
                }
                let o = &mut self.mobjs[i];
                if o.movedir >= 0 {
                    // turn towards where it walks, 45 degrees a tic
                    let want = o.movedir as f64 * PI / 4.0;
                    let d = wrap(want - o.angle);
                    o.angle = if d.abs() <= PI / 4.0 { want } else { o.angle + sgn(d) * PI / 4.0 };
                }
                if self.player.dead {
                    self.mobjs[i].target = false;
                    return self.set_state(i, St::Idle, 0);
                }
                if self.mobjs[i].just_attacked {
                    self.mobjs[i].just_attacked = false;
                    return self.new_dir(i);
                }
                let d = (px - ox).hypot(py - oy);
                let dd = self.mobjs[i].d;
                if dd.melee && d < 44.0 + pr && self.sight(me, Who::Player) {
                    self.mobjs[i].melee = true;
                    return self.set_state(i, St::Atk, 0);
                }
                if !dd.no_missile && self.mobjs[i].movecount == 0 && self.missile_range(i, d) {
                    self.mobjs[i].melee = false;
                    self.mobjs[i].just_attacked = true;
                    return self.set_state(i, St::Atk, 0);
                }
                self.mobjs[i].movecount -= 1;
                if self.mobjs[i].movecount < 0 || !self.step(i) {
                    self.new_dir(i);
                }
            }
            Face => self.mobjs[i].angle = (py - oy).atan2(px - ox),
            Shoot => {
                let ang = (py - oy).atan2(px - ox);
                self.mobjs[i].angle = ang;
                let (atk, melee, oz) = (self.mobjs[i].d.atk, self.mobjs[i].melee, self.mobjs[i].z);
                let d = (px - ox).hypot(py - oy);
                if melee || atk == Atk::Bite {
                    let snd = if s == "SKUL" {
                        "DSSKLATK"
                    } else if atk == Atk::Bite {
                        "DSSGTATK"
                    } else {
                        "DSCLAW"
                    };
                    self.snd_who(Some(snd), me);
                    if d < 44.0 + pr + 20.0 && self.sight(me, Who::Player) {
                        let r = self.rng.rnd();
                        let dmg = match atk {
                            Atk::Bite => ((r % 10) + 1) * 4,
                            Atk::Baron => ((r % 8) + 1) * 10,
                            Atk::Caco => ((r % 6) + 1) * 10,
                            _ => ((r % 8) + 1) * 3,
                        };
                        self.damage(Who::Player, dmg, Some(me));
                    }
                } else if atk == Atk::Pistol || atk == Atk::Shotgun {
                    self.snd_who(Some(if atk == Atk::Shotgun { "DSSHOTGN" } else { "DSPISTOL" }), me);
                    for _ in 0..if atk == Atk::Shotgun { 3 } else { 1 } {
                        let (a, b) = (self.rng.rnd(), self.rng.rnd());
                        let ba = ang + ((a - b) as f64 / 255.0) * 22.0 * RAD;
                        let dmg = ((self.rng.rnd() % 5) + 1) * 3;
                        self.bullet(me, ba, oz + 32.0, Some(Who::Player), dmg);
                    }
                } else {
                    self.snd_who(Some("DSFIRSHT"), me);
                    let m = match atk {
                        Atk::Caco => &CACO_BALL,
                        Atk::Baron => &BARON_BALL,
                        _ => &IMP_BALL,
                    };
                    self.missile(i, m);
                }
            }
            Pain => self.snd_who(Some(pain_snd(s)), me),
            Scream => {
                let snd = self.pick(death(s));
                self.snd_who(snd, me);
            }
            Fall => self.mobjs[i].solid = false,
            Explode => {
                // a barrel: everything within 128 units is hurt, less the further
                self.snd_who(Some("DSBAREXP"), me);
                let mut hurt = Vec::new();
                for t in &self.mobjs {
                    if t.id == id || !t.shootable {
                        continue;
                    }
                    let d = 0f64.max((t.x - ox).hypot(t.y - oy) - if t.r != 0.0 { t.r } else { 16.0 });
                    if d < 128.0 {
                        hurt.push((Who::Mobj(t.id), d));
                    }
                }
                if !self.away {
                    let d = 0f64.max((px - ox).hypot(py - oy) - pr);
                    if d < 128.0 {
                        hurt.push((Who::Player, d));
                    }
                }
                for (t, d) in hurt {
                    if self.sight(me, t) {
                        self.damage(t, (128.0 - d) as i32, Some(me));
                    }
                }
            }
            Gone => self.mobjs[i].gone = true,
            No => {}
        }
    }
    /// P_CheckMissileRange: in sight, then the further, the less likely
    fn missile_range(&mut self, i: usize, d: f64) -> bool {
        let id = self.mobjs[i].id;
        if !self.sight(Who::Mobj(id), Who::Player) {
            return false;
        }
        let o = &mut self.mobjs[i];
        if o.just_hit {
            o.just_hit = false;
            return true;
        }
        if o.reaction > 0 {
            return false;
        }
        let mut k = d - 64.0;
        if !o.d.melee {
            k -= 128.0; // nothing to do up close: shoots more
        }
        if o.d.s == "SKUL" {
            k /= 2.0;
        }
        self.rng.rnd() as f64 >= k.min(200.0)
    }
    // eight directions, toward the player, the next best when that is blocked (P_NewChaseDir, more or less). A door
    // in the way that a monster may open, it opens (P_Move's spechit)
    fn step(&mut self, i: usize) -> bool {
        let o = &self.mobjs[i];
        if o.movedir < 0 {
            return false;
        }
        let (id, a, sp) = (o.id, o.movedir as f64 * PI / 4.0, o.d.speed);
        let (x, y) = (o.x + a.cos() * sp, o.y + a.sin() * sp);
        if !self.try_move(Who::Mobj(id), x, y) {
            let mut opened = false;
            for l in std::mem::take(&mut self.spechit) {
                if self.trigger(l, Who::Mobj(id), How::Use) {
                    opened = true;
                }
            }
            if opened {
                self.mobjs[i].movedir = -1;
            }
            return opened;
        }
        let pz = self.player.z;
        let o = &mut self.mobjs[i];
        if o.d.fly {
            let tz = pz + 16.0;
            o.z += sgn(tz - o.z) * 4f64.min((tz - o.z).abs());
            let ceil = o.ceilz.unwrap_or(o.z + o.h);
            o.z = o.floorz.unwrap_or(o.z).max(o.z.min(ceil - o.h));
        }
        true
    }
    fn new_dir(&mut self, i: usize) {
        let o = &self.mobjs[i];
        let want = jround((self.player.y - o.y).atan2(self.player.x - o.x) / (PI / 4.0)) as i32;
        let mut order: Vec<i32> = [0, 1, -1, 2, -2, 3, -3, 4].iter().map(|k| (((want + k) % 8) + 8) % 8).collect();
        if self.rng.rnd() < 64 {
            for k in (1..order.len()).rev() {
                let j = self.rng.below(k + 1);
                order.swap(k, j);
            }
        }
        for dir in order {
            self.mobjs[i].movedir = dir;
            if self.step(i) {
                self.mobjs[i].movecount = self.rng.rnd() & 15;
                return;
            }
        }
        self.mobjs[i].movedir = -1;
    }
    fn missile(&mut self, i: usize, m: &'static MissileDef) {
        let o = &self.mobjs[i];
        let p = &self.player;
        let a = (p.y - o.y).atan2(p.x - o.x);
        let d = (p.x - o.x).hypot(p.y - o.y);
        let d = if d == 0.0 { 1.0 } else { d };
        let tics = d / m.speed;
        let (x, y, z) = (o.x + a.cos() * (o.r + 8.0), o.y + a.sin() * (o.r + 8.0), o.z + 32.0);
        let mz = (p.z + 32.0 - (o.z + 32.0)) / tics;
        let owner = o.id;
        let id = self.next_id;
        self.next_id += 1;
        let sector = self.sector_at(x, y);
        let d = Def { s: m.s, r: m.r, h: 8.0, ..Def::default() };
        self.mobjs.push(Mobj {
            id,
            ty: 0,
            d,
            x,
            y,
            z,
            angle: a,
            r: m.r,
            h: 8.0,
            sector,
            hp: 0,
            ambush: false,
            frame: b'A',
            sprite: m.s,
            tics: 0,
            si: 0,
            state: None,
            target: false,
            movedir: -1,
            movecount: 0,
            reaction: 0,
            solid: false,
            shootable: false,
            alive: true,
            bright: true,
            floorz: None,
            ceilz: None,
            gone: false,
            just_hit: false,
            just_attacked: false,
            melee: false,
            missile: Some(Missile { m, owner, mx: a.cos() * m.speed, my: a.sin() * m.speed, mz, dying: 0, life: 0 }),
            hit: None,
            office: false,
        });
    }
    fn fly(&mut self, i: usize) {
        let time = self.time;
        let o = &mut self.mobjs[i];
        let id = o.id;
        let ms = o.missile.as_mut().unwrap();
        if ms.dying > 0 {
            o.frame = ms.m.die[2.min(ms.dying / 6) as usize];
            ms.dying += 1;
            if ms.dying > 18 {
                o.gone = true;
            }
            return;
        }
        o.frame = ms.m.fly[(time / 4) as usize % 2];
        o.z += ms.mz;
        let (mx, my, dmg, owner) = (ms.mx, ms.my, ms.m.dmg, ms.owner);
        if o.office {
            // through the door: in the office, until it hits you or a wall there
            o.x += mx;
            o.y += my;
            let (x, y, z, r) = (o.x, o.y, o.z, o.r);
            if self.hits_player(z, r, x, y) {
                let d = ((self.rng.rnd() % 8) + 1) * dmg;
                self.damage(Who::Player, d, Some(Who::Mobj(owner)));
                self.mobjs[i].missile.as_mut().unwrap().dying = 1;
                self.snd_who(Some("DSFIRXPL"), Who::Mobj(id));
            } else {
                let gone = match &self.portal {
                    None => true,
                    Some(p) => !self.portal_inside(x, y) || z < p.floor || z + 8.0 > p.floor + p.height || {
                        let ms = self.mobjs[i].missile.as_mut().unwrap();
                        ms.life += 1;
                        ms.life > 200
                    },
                };
                if gone {
                    self.mobjs[i].gone = true;
                }
            }
            return;
        }
        let (x, y) = (o.x, o.y);
        let b0 = self.beyond(x, y);
        if !self.try_move(Who::Mobj(id), x + mx, y + my) {
            if let Some(h) = self.mobjs[i].hit {
                if h != Who::Mobj(owner) {
                    let d = ((self.rng.rnd() % 8) + 1) * dmg;
                    self.damage(h, d, Some(Who::Mobj(owner)));
                }
            }
            if let Some(i) = self.find(id) {
                self.mobjs[i].missile.as_mut().unwrap().dying = 1;
            }
            self.snd_who(Some("DSFIRXPL"), Who::Mobj(id));
        } else if let Some(p) = &self.portal {
            let (x, y) = (self.mobjs[i].x, self.mobjs[i].y);
            if self.away && b0 >= 0.0 && self.beyond(x, y) < 0.0 && self.along(x, y).abs() < p.w / 2.0 {
                self.mobjs[i].office = true; // through the door, to you
            }
        }
    }

    /// where the mysterious door goes in a level: in the wall behind where you start (a straight stretch w wide, as
    /// high as a door, its floor level), else in the longest such wall of the room you start in
    pub fn door_spot(&self, w: f64, h: f64) -> Option<Spot> {
        let st = &self.start;
        let s0 = self.sector_at(st.x, st.y);
        let a = st.angle * RAD + PI;
        let fits = |li: usize, cx: f64, cy: f64| -> Option<Spot> {
            let l = &self.map.lines[li];
            let fsi = self.map.sides[l.front].sector;
            let fs = &self.map.sectors[fsi];
            let len = l.dx.hypot(l.dy);
            let (tx, ty) = (l.dx / len, l.dy / len);
            let t = (cx - l.v1.x) * tx + (cy - l.v1.y) * ty;
            if fs.ceil - fs.floor < h {
                return None;
            }
            if let Some(b) = self.back_sector(li) {
                let bs = &self.map.sectors[b];
                if bs.floor - fs.floor < h && bs.ceil > fs.floor {
                    return None; // too low, or open below the door's height
                }
            }
            let c = (w / 2.0).max((len - w / 2.0).min(t));
            if len < w {
                return None;
            }
            Some(Spot { cx: l.v1.x + tx * c, cy: l.v1.y + ty * c, nx: ty, ny: -tx, lines: vec![li], sector: fsi, floor: fs.floor })
        };
        let fl = self.map.sectors[s0].floor;
        let tr = self.trace(st.x, st.y, a, 1024.0, &|_| fl + 40.0, false);
        if let Some(li) = tr.line {
            if l2side(&self.map.lines[li], st.x, st.y) == 0 {
                if let Some(f) = fits(li, tr.x, tr.y) {
                    return Some(f);
                }
            }
        }
        let (mut best, mut bl) = (None, 0.0);
        for &li in &self.map.sectors[s0].lines {
            let l = &self.map.lines[li];
            if self.map.sides[l.front].sector != s0 {
                continue;
            }
            let len = l.dx.hypot(l.dy);
            if len > bl {
                if let Some(f) = fits(li, (l.v1.x + l.v2.x) / 2.0, (l.v1.y + l.v2.y) / 2.0) {
                    best = Some(f);
                    bl = len;
                }
            }
        }
        best
    }
}
