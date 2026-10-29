//! What the room is told each frame by the app around it (room.js's `app`), and what it tells the app
//! back: actions (the page's `act`), sounds, which bot or screen is under the pointer.

use crate::model::{Chess, State};
use serde_json::Value;

/// ping-pong as the app has it open: the bot you play, or two you watch
#[derive(Clone, Debug, Default)]
pub struct PongInfo {
    pub id: Option<String>,
    pub watch: bool,
    /// the ball is in play (you against the machine)
    pub ball: bool,
    pub ids: Vec<String>,
}

/// the duel at the window (outside), as the app has it open
#[derive(Clone, Debug, Default)]
pub struct OutsideInfo {
    pub id: Option<String>,
    pub watch: bool,
    /// the window's key
    pub at: String,
    pub ids: Vec<String>,
    /// when each side last fired (seconds, the app's clock)
    pub fired: [f64; 2],
}

/// the notes (Linda): on, and what she is writing or explaining (a bot's id, or `u:<unit>` for a desk)
#[derive(Clone, Debug, Default)]
pub struct DigestInfo {
    pub on: bool,
    pub writing: Option<String>,
    pub explaining: Option<String>,
    /// sessions she has notes on
    pub sessions: Vec<String>,
}

/// your terminal at a screen
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShellKey {
    pub m: String,
    pub dir: String,
}

/// a shell the server keeps, and whether output comes
#[derive(Clone, Debug, Default)]
pub struct ShellInfo {
    pub m: String,
    pub dir: String,
    pub busy: bool,
}

/// what the room is told each frame
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub state: State,
    /// milliseconds, monotonic (performance.now())
    pub now: f64,
    /// this computer's clock: hours and minutes (the window's sky, the clock, the dog's night)
    pub hour: u32,
    pub minute: u32,
    /// ms since the epoch (the clock's awake time)
    pub epoch_ms: i64,
    /// the bot (folk id) whose tile has the focus: you stand with it
    pub open: Option<String>,
    /// every bot with a tile
    pub opens: Vec<String>,
    /// the chess game you play (open, not over)
    pub chess: Option<Chess>,
    pub pong: Option<PongInfo>,
    pub outside: Option<OutsideInfo>,
    /// the view (panel) that is open: closet, settings, chess, ...
    pub view: Option<String>,
    pub digest: Option<DigestInfo>,
    /// seconds since the window lost the focus (0 while it has it)
    pub away: f64,
    /// the bot (folk id) whose tile the pointer is on
    pub hl: Option<String>,
    pub shell: Option<ShellKey>,
    pub shells: Vec<ShellInfo>,
    pub hl_shell: Option<ShellKey>,
    /// reduced motion
    pub still: bool,
    /// in the first person (fps)
    pub fps: bool,
}

impl Input {
    pub fn hardcore(&self) -> bool {
        self.state.settings.hardcore
    }
}

/// a sound to play: its name, where it comes from (-1 left .. 1 right), how loud; a booth's hum is kept by
/// its bot's id until it is stopped
#[derive(Clone, Debug, PartialEq)]
pub struct Sfx {
    pub name: &'static str,
    pub pan: f64,
    pub vol: f64,
    pub hum: Option<String>,
}

/// what the room tells the app
#[derive(Default, Debug)]
pub struct Output {
    /// the page's `act` calls: {kind, ...}
    pub acts: Vec<Value>,
    pub sounds: Vec<Sfx>,
    /// hums to stop (a bot taken back out of the booth)
    pub hush: Vec<String>,
    /// the bot under the pointer changed: light its tile
    pub hover: Option<Option<String>>,
    /// the screen under the pointer changed (`m:dir`): light its shell's tile
    pub hover_shell: Option<Option<String>>,
    /// a bot carried out over the pane (client coordinates), or not
    pub carry: Option<Option<(String, f64, f64)>>,
    /// a bot let go over the pane
    pub carried: Option<(String, f64, f64)>,
    /// the pointer's look: default, pointer, grab, grabbing
    pub cursor: &'static str,
    /// start the first person at the gun closet with this key
    pub first_person: Option<String>,
    /// leave the first person (hardcore went off)
    pub leave_fps: bool,
}

impl Output {
    pub fn act(&mut self, v: Value) {
        self.acts.push(v);
    }
}
