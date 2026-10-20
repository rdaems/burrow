// A room is a machine: this one, or one reached over ssh. Everything burrow knows about the bots comes
// from Claude Code itself on that machine: `claude agents --json` (who runs, where, busy or idle),
// `claude --bg` (start one), `claude attach` (open one), `claude stop`. Nothing is installed there.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Host {
    pub name: String,
    /// an ssh destination (anything `ssh` accepts, ~/.ssh/config applies); none = this machine
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<String>,
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn config_path() -> PathBuf {
    home().join(".config/burrow/rooms.json")
}

fn cache_dir() -> PathBuf {
    let d = home().join(".cache/burrow");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn load_hosts() -> Vec<Host> {
    if let Ok(s) = std::fs::read_to_string(config_path()) {
        if let Ok(v) = serde_json::from_str::<Vec<Host>>(&s) {
            if !v.is_empty() {
                return v;
            }
        }
    }
    let name = Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "here".into());
    vec![Host { name, ssh: None }]
}

pub fn save_hosts(hosts: &[Host]) {
    let p = config_path();
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    let _ = std::fs::write(p, serde_json::to_string_pretty(hosts).unwrap());
}

/// single-quote for a POSIX shell; a leading ~/ stays a home directory
pub fn sq(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        return format!("\"$HOME\"/{}", sq(rest));
    }
    if s == "~" {
        return "\"$HOME\"".into();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

// a non-login shell over ssh (and an app started from the Dock) has a bare PATH; claude lives here
const PATH: &str = "PATH=\"$HOME/.local/bin:$HOME/.claude/local:/opt/homebrew/bin:/usr/local/bin:$PATH\"; export PATH; ";

impl Host {
    fn ssh_opts(&self) -> Vec<String> {
        let ctl = cache_dir().join("ssh-%C");
        vec![
            "-o".into(),
            "ControlMaster=auto".into(),
            "-o".into(),
            format!("ControlPath={}", ctl.display()),
            "-o".into(),
            "ControlPersist=600".into(),
            "-o".into(),
            "ServerAliveInterval=15".into(),
        ]
    }

    /// program + args that run `script` in a shell there; `tty` for an interactive one
    pub fn argv(&self, script: &str, tty: bool) -> (String, Vec<String>) {
        let script = format!("{PATH}{script}");
        match &self.ssh {
            None => ("/bin/sh".into(), vec!["-c".into(), script]),
            Some(dest) => {
                let mut a = self.ssh_opts();
                if tty {
                    a.push("-t".into());
                } else {
                    a.extend(["-o".into(), "BatchMode=yes".into()]);
                }
                a.push(dest.clone());
                a.push(script);
                ("ssh".into(), a)
            }
        }
    }

    pub fn run(&self, script: &str) -> Result<String, String> {
        let (p, a) = self.argv(script, false);
        let out = Command::new(p).args(a).output().map_err(|e| e.to_string())?;
        let so = String::from_utf8_lossy(&out.stdout).to_string();
        if out.status.success() {
            Ok(so)
        } else {
            let se = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if se.is_empty() { so.trim().to_string() } else { se })
        }
    }

    /// start a background session in `dir`; returns its short id
    pub fn spawn(&self, dir: &str, name: &str) -> Result<String, String> {
        let n = if name.trim().is_empty() { String::new() } else { format!(" -n {}", sq(name.trim())) };
        let out = self.run(&format!("cd {} && claude --bg{n} 2>&1", sq(dir)))?;
        // "backgrounded · 7ebbc505 · name (idle — send a prompt to start)"
        let first = out.lines().next().unwrap_or("");
        if first.starts_with("backgrounded") {
            if let Some(id) = first.split('·').nth(1) {
                return Ok(id.trim().to_string());
            }
        }
        Err(out.trim().to_string())
    }

    pub fn stop(&self, id: &str) -> Result<String, String> {
        self.run(&format!("claude stop {}", sq(id)))
    }

    /// copy a dropped file there; returns the path to type into the session
    pub fn upload(&self, local: &std::path::Path) -> Result<String, String> {
        let Some(dest) = &self.ssh else {
            return Ok(local.display().to_string());
        };
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        let dir = self.run(&format!("d=\"$HOME/.cache/burrow/drop/{stamp}\"; mkdir -p \"$d\" && echo \"$d\""))?;
        let name = local.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
        let remote = format!("{}/{}", dir.trim(), name);
        let out = Command::new("scp")
            .args(self.ssh_opts())
            .arg("-q")
            .arg(local)
            .arg(format!("{dest}:{}", sq(&remote)))
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(remote)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bot {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub busy: bool,
    /// a `claude --bg` session; the others run in a terminal somewhere else and can only be watched
    pub background: bool,
    pub started: u64,
}

#[derive(Default)]
pub struct Feed {
    pub bots: Vec<Bot>,
    pub error: Option<String>,
    pub fresh: bool,
    /// Claude Code's theme there starts with "light"
    pub light: bool,
}

pub fn parse_agents(json: &str) -> Result<Vec<Bot>, String> {
    let v: serde_json::Value = serde_json::from_str(json.trim()).map_err(|e| format!("claude agents: {e}"))?;
    let s = |x: &serde_json::Value, k: &str| x.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut bots: Vec<Bot> = v
        .as_array()
        .ok_or("claude agents: not a list")?
        .iter()
        .filter(|x| x.get("pid").is_some())
        .map(|x| {
            let sid = s(x, "sessionId");
            let id = Some(s(x, "id")).filter(|i| !i.is_empty()).unwrap_or_else(|| sid.chars().take(8).collect());
            Bot {
                name: Some(s(x, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| id.clone()),
                id,
                cwd: s(x, "cwd"),
                busy: s(x, "status") == "busy" || s(x, "state") == "working",
                background: s(x, "kind") == "background",
                started: x.get("startedAt").and_then(|v| v.as_u64()).unwrap_or(0),
            }
        })
        .collect();
    bots.sort_by_key(|b| b.started);
    Ok(bots)
}

pub struct Watch {
    pub feed: Arc<Mutex<Feed>>,
    pub fast: Arc<AtomicBool>,
    pub poke: Arc<AtomicBool>,
}

/// ask the machine who is there: every 3 s while burrow has focus, every 15 s while it hasn't
/// (`claude agents` costs the machine about half a second of CPU)
pub fn watch(host: Host, ctx: egui::Context) -> Watch {
    let w = Watch {
        feed: Arc::new(Mutex::new(Feed::default())),
        fast: Arc::new(AtomicBool::new(true)),
        poke: Arc::new(AtomicBool::new(false)),
    };
    let (feed, fast, poke) = (w.feed.clone(), w.fast.clone(), w.poke.clone());
    std::thread::spawn(move || loop {
        let r = host.run("claude agents --json; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null");
        let r = r.and_then(|o| {
            let (agents, settings) = o.split_once("@@burrow@@").unwrap_or((&o, ""));
            let light = serde_json::from_str::<serde_json::Value>(settings.trim())
                .ok()
                .and_then(|v| v.get("theme").and_then(|t| t.as_str()).map(|t| t.starts_with("light")))
                .unwrap_or(false);
            parse_agents(agents).map(|b| (b, light))
        });
        {
            let mut f = feed.lock().unwrap();
            match r {
                Ok((b, light)) => {
                    f.light = light;
                    f.bots = b;
                    f.error = None;
                }
                Err(e) => f.error = Some(e),
            }
            f.fresh = true;
        }
        ctx.request_repaint();
        let t = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let every = if fast.load(Ordering::Relaxed) { 3 } else { 15 };
            if poke.swap(false, Ordering::Relaxed) || t.elapsed() >= Duration::from_secs(every) {
                break;
            }
        }
    });
    w
}
