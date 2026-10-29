//! Where things are, and how this burrow runs: server.py's top-level constants, read from the environment
//! once. Tests build one rooted in a temporary directory (`Config::under`), so nothing of the person's is read.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    /// made-up bots, for working on the room (BURROW_DEMO)
    pub demo: bool,
    /// the office managers' curl port (BURROW_PORT): what her CLAUDE.md says
    pub port: u16,
    /// where a pair table opens a new room (BURROW_PAIR)
    pub pair: String,
    /// the model that writes the notes (BURROW_DIGEST_MODEL)
    pub digest_model: String,
    pub home: PathBuf,
    /// room.json, rooms.json (the Rust app's, one room per machine), digest.json, the office (BURROW_CONFIG_DIR)
    pub config: PathBuf,
    /// ssh's control sockets, files dropped on sessions, where `claude -p` runs (BURROW_CACHE_DIR)
    pub cache: PathBuf,
    /// Claude Code's settings: the machines (sshConfigs), the theme
    pub claude_settings: PathBuf,
    pub ssh_config: PathBuf,
    /// the office managers' own directories (BURROW_OFFICE)
    pub office: PathBuf,
    /// the demo hires a real office manager only when given a directory for her
    pub real_office: bool,
    /// a copy of a real room.json to try things on in the demo (never saved; BURROW_DEMO_ROOM)
    pub demo_room: Option<PathBuf>,
    pub hostname: String,
    /// the login name: you, in the room, unless room.json has one
    pub user: String,
    /// the command that keeps this computer from sleeping while it runs, if there is one
    pub inhibitor: Option<Vec<String>>,
}

impl Config {
    pub fn from_env() -> Config {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let home = var("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        let config = var("BURROW_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".config/burrow"));
        let demo = var("BURROW_DEMO").is_some();
        let office = var("BURROW_OFFICE");
        Config {
            demo,
            port: var("BURROW_PORT").and_then(|p| p.parse().ok()).unwrap_or(7777),
            pair: var("BURROW_PAIR").unwrap_or_else(|| "https://pair.daems.me".into()).trim_end_matches('/').to_string(),
            digest_model: var("BURROW_DIGEST_MODEL").unwrap_or_else(|| "claude-sonnet-5-5".into()),
            cache: var("BURROW_CACHE_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".cache/burrow")),
            claude_settings: home.join(".claude/settings.json"),
            ssh_config: home.join(".ssh/config"),
            real_office: !demo || office.is_some(),
            office: office.map(PathBuf::from).unwrap_or_else(|| config.join("office")),
            demo_room: var("BURROW_DEMO_ROOM").map(PathBuf::from),
            hostname: crate::util::hostname(),
            user: crate::util::login(),
            inhibitor: inhibitor(),
            config,
            home,
        }
    }

    /// everything under `root` (its own home, config, cache): for tests
    pub fn under(root: &Path, demo: bool) -> Config {
        let home = root.join("home");
        let config = home.join(".config/burrow");
        Config {
            demo,
            port: 0,
            pair: "http://pair.invalid".into(),
            digest_model: "claude-test".into(),
            cache: home.join(".cache/burrow"),
            claude_settings: home.join(".claude/settings.json"),
            ssh_config: home.join(".ssh/config"),
            office: config.join("office"),
            real_office: !demo,
            demo_room: None,
            hostname: "here".into(),
            user: "tester".into(),
            inhibitor: None,
            config,
            home,
        }
    }

    pub fn room(&self) -> PathBuf {
        self.config.join("room.json")
    }
    pub fn old_rooms(&self) -> PathBuf {
        self.config.join("rooms.json")
    }
    pub fn digest(&self) -> PathBuf {
        self.config.join("digest.json")
    }
    pub fn home_str(&self) -> String {
        self.home.to_string_lossy().into_owned()
    }
}

/// `caffeinate` on a Mac (tied to this process's life with -w), `systemd-inhibit` elsewhere
fn inhibitor() -> Option<Vec<String>> {
    let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect();
    if crate::util::which("caffeinate") {
        let pid = std::process::id().to_string();
        return Some(v(&["caffeinate", "-i", "-s", "-w", &pid]));
    }
    if crate::util::which("systemd-inhibit") {
        return Some(v(&["systemd-inhibit", "--what=idle:sleep", "--who=burrow", "--why=bots working", "sleep", "infinity"]));
    }
    None
}
