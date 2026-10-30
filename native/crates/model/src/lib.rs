//! burrow's state as the room sees it: what the web server pushed as its `state` event (server.py's
//! `Hub.state()`), field for field, so the room can be fed by the Python server, by the Rust core, or by
//! a fixture alike. Every field has a default: an older or newer server leaves some out.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct State {
    pub machines: Vec<Machine>,
    pub desks: Vec<Desk>,
    pub bots: Vec<Bot>,
    /// where furniture stands: key -> [x, y] in dots (`u:<unit>` for a desk, `door:<machine>` for a door)
    pub layout: BTreeMap<String, [i32; 2]>,
    /// the pieces in the room, by key (`plant0`, `mat2`, ...)
    pub furniture: Vec<String>,
    pub pair: BTreeMap<String, PairLink>,
    /// a machine's paint, by machine id (-1 the wood)
    pub paint: BTreeMap<String, i32>,
    pub usage: Usage,
    pub chess: Option<Chess>,
    pub demo: bool,
    pub awake: Awake,
    /// the room's width in dots, as the divider was dragged
    pub width: Option<f64>,
    pub shells: Vec<ShellView>,
    pub settings: Settings,
    pub managers: Vec<Manager>,
    /// sessions being taken out (their desk burnt): they queue for the booth
    pub removing: Vec<String>,
    pub you: You,
    #[serde(rename = "pairAt")]
    pub pair_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Machine {
    /// "" for this computer
    pub id: String,
    pub name: String,
    /// the ssh host; "" here, "@demo" in the demo
    pub ssh: String,
    /// its theme is light
    pub light: bool,
    /// why it can't be reached
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Desk {
    /// `<dir>` here, `<machine id>:<dir>` elsewhere
    pub unit: String,
    pub m: String,
    pub dir: String,
    /// a name you gave it ("" for none)
    pub name: String,
    /// put down by you (it stays without sessions)
    pub kept: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Bot {
    pub id: String,
    pub name: String,
    pub cwd: String,
    /// a turn runs (tempo active, its process alive)
    pub busy: bool,
    /// a dialog is open in its session (hand up, a ?)
    pub waiting: bool,
    /// its process runs
    pub alive: bool,
    /// Claude Code's reading of its last message (working, blocked, done, ...)
    pub state: String,
    /// active, blocked, idle (Claude Code's job record)
    pub tempo: String,
    /// a prompt is open: nothing can be typed to it
    pub dialog: bool,
    /// done / failed / stopped when it is over (no process)
    pub over: String,
    #[serde(rename = "endedAt")]
    pub ended_at: i64,
    /// agent view's one-line summary
    pub detail: String,
    /// its record's last write
    pub updated: i64,
    pub started: i64,
    pub pid: i64,
    /// its conversation
    pub session: String,
    /// its Remote Control session (cse_...), if it is in the Claude app too
    pub rc: String,
    /// its machine
    pub m: String,
    /// its desk
    pub unit: String,
    /// the worktree it works in, "" for the main checkout
    pub tree: String,
    /// set on a person's entry (you), never by the server
    #[serde(skip)]
    pub human: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PairLink {
    pub url: String,
    pub room: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Limit {
    pub label: String,
    /// percent used
    pub used: i64,
    pub resets: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Usage {
    pub session: Option<Limit>,
    pub week: Option<Limit>,
    pub bars: Vec<Limit>,
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Player {
    pub id: String,
    pub name: String,
    /// "agent" or "human"
    pub kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Chess {
    pub id: i64,
    pub white: Player,
    pub black: Player,
    /// 64 squares, a1 first, "." for none (upper case white)
    pub board: String,
    pub sans: Vec<String>,
    pub last: Option<String>,
    pub check: bool,
    #[serde(rename = "toMove")]
    pub to_move: String,
    pub result: Option<String>,
    pub why: Option<String>,
    pub free: bool,
    /// your legal moves (uci), when it is your turn
    pub legal: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Awake {
    /// off, work, for
    pub mode: String,
    /// ms since the epoch, for "for"
    pub until: i64,
    /// the inhibitor runs
    pub on: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ShellView {
    pub m: String,
    pub dir: String,
    pub busy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub rc: bool,
    pub yolo: bool,
    pub hardcore: bool,
    pub sound: bool,
    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { rc: true, yolo: false, hardcore: false, sound: true, other: BTreeMap::new() }
    }
}

impl Settings {
    /// a switch kept among the others (`other`), on: halloween (the pumpkin is its switch)
    pub fn on(&self, key: &str) -> bool {
        self.other.get(key).and_then(serde_json::Value::as_bool).unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Manager {
    /// her reception's key (manager0, ...)
    pub key: String,
    pub style: String,
    pub name: String,
    pub seed: String,
    pub color: String,
    /// she is a Claude Code session of her own
    pub session: bool,
    pub id: String,
    pub error: String,
    pub busy: bool,
    pub waiting: bool,
    pub alive: bool,
    pub rc: String,
    /// shot: her reception stands empty until hired again
    pub vacant: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct You {
    pub name: String,
    pub seed: String,
    pub color: Option<String>,
    pub login: String,
}

impl State {
    pub fn from_json(s: &str) -> serde_json::Result<State> {
        serde_json::from_str(s)
    }
    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_demo_state() {
        let s = State::from_json(include_str!("../tests/fixtures/demo-state.json")).unwrap();
        assert_eq!(s.machines[0].id, "");
        assert!(s.bots.len() >= 4, "{}", s.bots.len());
        assert!(s.desks.iter().any(|d| d.kept));
        assert!(s.usage.session.is_some());
        assert!(!s.managers.is_empty());
        // and back: what is written reads the same
        let again = State::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(again, s);
    }
}
