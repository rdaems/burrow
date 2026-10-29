//! The office in the first person, Doom's way: a port of web/fps.js (the engine), web/props3d.js (the office's
//! things as models and textures) and web/doom/ (the WAD, the level's game). No UI toolkit: the app feeds input
//! and a scene of the room, ticks it, and gets Doom's screen as pixels and what happened as events.

pub mod level;
pub mod rng;
pub mod wad;

pub use level::Level;
pub use wad::Wad;
