//! What the room hands the first person, at its last tick (room.js `scene3`): the floor and what lies on it, what
//! hangs on the far wall, every desk and piece of furniture as blocks faced with pictures, everyone as a model in the
//! pose the room gives them, the lights. In the room's dots (the view makes them units: `U` a dot). props.rs makes
//! the blocks and pictures from what the room knows (`Props::desk`, `Props::build`, …); the room puts them in here.

use pix::{Color, Image};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A picture the view is given, with an identity: the view keeps its copy in Doom's palette by it (fps.js kept one
/// per canvas and its version). Changed content is a new `Tex`.
#[derive(Clone)]
pub struct Tex {
    pub id: u64,
    pub img: Arc<Image>,
}
impl Tex {
    pub fn new(img: Image) -> Tex {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Tex { id: NEXT.fetch_add(1, Ordering::Relaxed), img: Arc::new(img) }
    }
    pub fn w(&self) -> u32 {
        self.img.w
    }
    pub fn h(&self) -> u32 {
        self.img.h
    }
}
impl std::fmt::Debug for Tex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tex#{}({}x{})", self.id, self.img.w, self.img.h)
    }
}

/// A texel whose alpha is GLOW shines: the view does not light it (screens, eyes, the ceiling's lights, the fire).
pub const GLOW: u8 = 254;
/// texels a dot in props' pictures
pub const TS: f64 = 4.0;

/// a face of a block: a part of a picture; `glow`: all of it shines
#[derive(Clone, Debug)]
pub struct Face {
    pub tex: Tex,
    pub sx: f64,
    pub sy: f64,
    pub sw: f64,
    pub sh: f64,
    pub glow: bool,
}
impl Face {
    pub fn whole(tex: Tex) -> Face {
        let (sw, sh) = (tex.w() as f64, tex.h() as f64);
        Face { tex, sx: 0.0, sy: 0.0, sw, sh, glow: false }
    }
}

/// A block: x0..x1 (west to east), y0..y1 (north to south), z0..z1 (up), in dots. `front` is its south face (as
/// the room is seen), `side` the east and west ones unless `west`/`east` are given apart.
#[derive(Clone, Debug, Default)]
pub struct Block {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub z0: f64,
    pub z1: f64,
    pub front: Option<Face>,
    pub back: Option<Face>,
    pub side: Option<Face>,
    pub west: Option<Face>,
    pub east: Option<Face>,
    pub top: Option<Face>,
}

/// a robot (a bot, an office manager) in the room's pose
#[derive(Clone, Debug, PartialEq)]
pub struct Robot {
    /// its seed (pixel.js picks its head from it)
    pub seed: String,
    /// its own colour, "#rrggbb"
    pub color: String,
    /// an office manager: her skirt and bow
    pub femme: bool,
    /// down, up, wave, sw1, sw2, fwd1, fwd2, reach, book, mug, paper, can, roll, gun
    pub arms: String,
    /// stand, sit, walk1, walk2
    pub legs: String,
}
/// what a figure is: a model the view turns to the side that faces you
#[derive(Clone, Debug, PartialEq)]
pub enum Figure {
    Robot(Robot),
    /// the dog: stand, walk1, walk2, sit, lie; its tail one way or the other
    Dog { pose: String, wag: bool },
}

/// Something standing in the office: a picture (the same from every side, as Doom's lamps), or a figure.
#[derive(Clone, Debug, Default)]
pub struct Thing {
    /// who or what it is from one scene to the next (it walks between them)
    pub key: Option<String>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// its picture (not for a figure)
    pub pic: Option<Face>,
    /// where in the picture it stands (its anchor), in texels
    pub ax: f64,
    pub ay: f64,
    /// texels a dot (1 when 0)
    pub res: f64,
    pub flip: bool,
    /// which way it faces in the room, -1 west, 1 east, 0 not given
    pub face: f64,
    pub fig: Option<Figure>,
    /// the way it faces, an angle (an office manager at her counter faces the room)
    pub facing: Option<f64>,
    pub seated: bool,
    /// going up in the booth's light: Doom's fuzz
    pub dissolve: bool,
    /// a bot that can be shot: its session's id (the room leaves it off what it would not let you shoot)
    pub bot: Option<String>,
    /// the remains of a shot one: its id (E puts it back together)
    pub rem: Option<String>,
    /// a piece of furniture: its key
    pub piece: Option<String>,
    pub name: Option<String>,
    pub staff: bool,
    /// what the HUD says it is, under the crosshair (room.js `label`)
    pub label: Option<String>,
    /// E on it loads the gun (the gun closet: room.js `use` → "reload")
    pub reload: bool,
}

/// a light in the office: the fire's flickers
#[derive(Clone, Debug, Default)]
pub struct Light {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
    pub col: [f64; 3],
    pub k: f64,
    pub flicker: bool,
}

/// a desk's (or a door's) paint
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub hi: Color,
    pub top: Color,
    pub d: Color,
    pub k: Color,
    pub plate: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WallKind {
    /// a machine's door in its paint (boarded while out of reach)
    Door { paint: Paint, ok: bool },
    /// the sky by the hour
    Window { hour: u32 },
    /// its hands at this computer's time; awake: kept awake
    Clock { hours: u32, minutes: u32, awake: bool },
    Closet,
    Settings,
}
/// what hangs on the far wall, in the room's wall band (y0..y1 of `wall` dots, the floor at `wall`)
#[derive(Clone, Debug, PartialEq)]
pub struct WallItem {
    pub kind: WallKind,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

/// what props.rs makes of a desk or a piece: its blocks, its pictures, what you can't walk through, its lights
#[derive(Clone, Debug, Default)]
pub struct Parts {
    pub boxes: Vec<Block>,
    pub things: Vec<Thing>,
    pub solid: Vec<[f64; 4]>,
    pub lights: Vec<Light>,
}

/// The office as the room has it now. `v` goes up with every scene (who walks walks between two of them).
#[derive(Clone, Debug)]
pub struct Scene {
    pub v: u64,
    /// the room's width and height, its wall's depth, in dots
    pub w: f64,
    pub h: f64,
    pub wall: f64,
    /// the floor's overlay (the rug, the mats, the oil, the pit, the shadows), two texels a dot: opaque (alpha ≥ 250;
    /// GLOW shines), or a shadow (`3 + round(a / 255 * 10)` levels darker)
    pub floor: Tex,
    /// the far wall's picture (`Props::wall`), TS texels a dot: what is not a door is taken from it
    pub wall_pic: Tex,
    pub items: Vec<WallItem>,
    /// changes when what hangs on the wall does
    pub wkey: String,
    pub boxes: Vec<Block>,
    pub things: Vec<Thing>,
    /// [x0, y0, x1, y1] in dots
    pub solid: Vec<[f64; 4]>,
    pub lights: Vec<Light>,
    /// the bots you shot (the face grins)
    pub kills: i32,
}
impl Scene {
    /// an empty office w x h dots, its wall `wall` deep
    pub fn empty(w: f64, h: f64, wall: f64) -> Scene {
        Scene {
            v: 1,
            w,
            h,
            wall,
            floor: Tex::new(Image::new((w * 2.0) as u32, (h * 2.0) as u32)),
            wall_pic: Tex::new(Image::new(1, 1)),
            items: Vec::new(),
            wkey: String::new(),
            boxes: Vec::new(),
            things: Vec::new(),
            solid: Vec::new(),
            lights: Vec::new(),
            kills: 0,
        }
    }
    pub fn add(&mut self, p: Parts) {
        self.boxes.extend(p.boxes);
        self.things.extend(p.things);
        self.solid.extend(p.solid);
        self.lights.extend(p.lights);
    }
}
