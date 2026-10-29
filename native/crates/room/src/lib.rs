//! burrow's room, without a UI toolkit: its state, its logic and its pixel art, drawn into a buffer
//! (a port of web/room.js, web/pixel.js and web/sprites.js). The app shows the buffer and hands it the
//! pointer and the keys.

pub mod avatar;
pub mod figure;
pub mod font;
pub mod sprites;
#[rustfmt::skip]
pub mod sprites_gen;

pub use burrow_model as model;
