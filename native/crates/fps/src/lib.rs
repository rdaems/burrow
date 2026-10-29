//! The office in the first person, Doom's way: a port of web/fps.js (the engine), web/props3d.js (the office's
//! things as models and textures) and web/doom/ (the WAD, the level's game). No UI toolkit: the app feeds input
//! and a scene of the room, ticks it, and gets Doom's screen as pixels and what happened as events.

pub mod font;
pub mod fps;
pub mod level;
pub mod props;
pub mod rng;
pub mod scene;
pub mod wad;

pub use level::Level;
pub use wad::Wad;
pub use fps::{Event, Fps, Key, LCam, Melt, Options, Sound, Start, World, You};
pub use level::Origin;
pub use props::{Desk, Fig, Piece, Props};
pub use scene::{Block, Face, Figure, Light, Paint, Parts, Robot, Scene, Tex, Thing, WallItem, WallKind};
