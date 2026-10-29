//! burrow's backend, in process: a port of web/server.py (and chess.py) for the native app. The `Hub` knows
//! the machines (this one and Claude Code's sshConfigs), polls `claude agents --json --all` on each, keeps
//! room.json, starts, stops and attaches sessions, keeps your terminals, writes the notes; the app calls its
//! methods where the page called /api/*, and hears its `Event`s where the page had server-sent events.
//! Only the office managers' curl (`/api/office`) still comes over HTTP (`Hub::serve_office`).

pub mod chess;
pub mod config;
pub mod parse;
pub mod room;
pub mod runner;
pub mod util;

pub use config::Config;
pub use runner::{Mach, Pty, Runner, System};
