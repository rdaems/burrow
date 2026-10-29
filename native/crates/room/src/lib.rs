//! burrow's room, without a UI toolkit: its state, its logic and its pixel art, drawn into a buffer
//! (a port of web/room.js, web/pixel.js and web/sprites.js). The app shows the buffer and hands it the
//! pointer and the keys.

// the web room's own numbers are kept as they are (6.28 for a turn, where room.js has it): the same dice, the same room
#![allow(clippy::approx_constant)]

pub mod avatar;
pub mod dog;
pub mod draw;
pub mod art;
pub mod figure;
pub mod font;
pub mod io;
pub mod layout;
pub mod mutiny;
pub mod paint;
pub mod paintings;
pub mod pointer;
pub mod remains;
pub mod room;
pub mod scene;
pub mod scene3;
pub mod sprites;
pub mod stage;
#[rustfmt::skip]
pub mod sprites_gen;
pub mod think;
pub mod world;

pub use burrow_model as model;
