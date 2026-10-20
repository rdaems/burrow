// A room is a machine: this one, or one reached over ssh. Everything burrow knows about the bots comes
// from Claude Code itself on that machine: `claude agents --json` (who runs, where, busy or idle),
// `claude --bg` (start one), `claude attach` (open one), `claude stop`. Nothing is installed there.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Host {
    pub name: String,
    /// an ssh destination (anything `ssh` accepts, ~/.ssh/config applies); none = this machine
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<String>,
    /// the working directories shown as projects, also while no bot works there
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirs: Vec<String>,
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

/// BURROW_DEMO=1: made-up rooms with made-up bots, for working on the room without real sessions
const DEMO: &str = "@demo";

pub fn load_hosts() -> Vec<Host> {
    if std::env::var_os("BURROW_DEMO").is_some() {
        let dirs = vec![format!("{}/projects/romuald", home().display())];
        return ["mac", "odyssey"].map(|n| Host { name: n.into(), ssh: Some(DEMO.into()), dirs: dirs.clone() }).to_vec();
    }
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
    vec![Host { name, ..Default::default() }]
}

pub fn save_hosts(hosts: &[Host]) {
    if std::env::var_os("BURROW_DEMO").is_some() {
        return;
    }
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
const PATH: &str = "PATH=\"$HOME/.local/bin:$HOME/.claude/local:/opt/homebrew/bin:/usr/local/bin:$PATH\"; export PATH; export COLORTERM=truecolor; ";

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
        if self.ssh.as_deref() == Some(DEMO) {
            // a made-up bot's session: a shell that says so
            return ("/bin/sh".into(), vec!["-c".into(), "printf 'a demo bot: no session behind it\\n'; exec cat".into()]);
        }
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
        // (the first start of the day prints "Starting background service…" before it)
        if let Some(line) = out.lines().find(|l| l.trim_start().starts_with("backgrounded")) {
            if let Some(id) = line.split('·').nth(1) {
                return Ok(id.trim().to_string());
            }
        }
        Err(out.trim().to_string())
    }

    /// the absolute path of a directory there (and whether it is one)
    pub fn resolve(&self, dir: &str) -> Result<String, String> {
        if self.ssh.as_deref() == Some(DEMO) {
            return Ok(dir.replacen('~', &home().display().to_string(), 1));
        }
        let out = self.run(&format!("cd {} && pwd -P", sq(dir.trim())))?;
        Ok(out.trim().to_string())
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
            .arg(format!("{dest}:{remote}")) // scp speaks SFTP now: the path is taken literally, no quoting
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
    pub pid: u64,
    /// the full session id: its transcript is ~/.claude/projects/<escaped cwd>/<session>.jsonl
    pub session: String,
}

#[derive(Default)]
pub struct Feed {
    pub bots: Vec<Bot>,
    pub error: Option<String>,
    pub fresh: bool,
    /// Claude Code's theme there starts with "light"
    pub light: bool,
    /// messages one bot sent another since the UI last looked: (from, to), by bot id
    pub talks: Vec<(String, String)>,
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
                pid: x.get("pid").and_then(|v| v.as_u64()).unwrap_or(0),
                session: sid,
            }
        })
        .collect();
    bots.sort_by_key(|b| b.started);
    Ok(bots)
}

/// a message between sessions shows up in the receiver's transcript as a user turn with
/// `"origin":{"kind":"peer","from":"uds:…","verifiedPeerPid":…,"msg_id":"…","name":"<sender>",…}`;
/// the poll greps the last 4 MB of transcripts written in the last minute (a session can write a few
/// hundred kB of context right after the message) for that and prints
/// `<receiver session id> <origin…>` per message (inside a JSON string the quotes would be escaped,
/// so quoted transcripts don't match)
const PEERS: &str = r#"find ~/.claude/projects -maxdepth 2 -name '*.jsonl' -mmin -1 2>/dev/null | while IFS= read -r f; do s=${f##*/}; tail -c 4194304 "$f" | grep -o '"origin":{"kind":"peer"[^}]*' | sed "s|^|${s%.jsonl} |"; done"#;

fn field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let i = s.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = s[i..].trim_start_matches('"');
    rest.split(|c| c == '"' || c == ',' || c == '}').next()
}

/// (msg_id, from, to) by bot id, for the messages between bots that are there now
pub fn parse_peers(out: &str, bots: &[Bot]) -> Vec<(String, String, String)> {
    out.lines()
        .filter_map(|l| {
            let (session, origin) = l.split_once(' ')?;
            let msg = field(origin, "msg_id")?;
            let to = bots.iter().find(|b| b.session == session)?;
            let pid = field(origin, "verifiedPeerPid").and_then(|p| p.parse::<u64>().ok());
            let name = field(origin, "name");
            let from = bots.iter().find(|b| Some(b.pid) == pid).or_else(|| bots.iter().find(|b| Some(b.name.as_str()) == name))?;
            (from.id != to.id).then(|| (msg.to_string(), from.id.clone(), to.id.clone()))
        })
        .collect()
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
    if host.ssh.as_deref() == Some(DEMO) {
        std::thread::spawn(move || demo(feed, ctx));
        return w;
    }
    // messages seen already; what is in the transcripts when burrow starts is old news
    let mut heard: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut first = true;
    std::thread::spawn(move || loop {
        let r = host.run(&format!("claude agents --json; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null; echo '@@burrow@@'; {PEERS}"));
        let r = r.and_then(|o| {
            let mut parts = o.splitn(3, "@@burrow@@");
            let (agents, settings, peers) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            let light = serde_json::from_str::<serde_json::Value>(settings.trim())
                .ok()
                .and_then(|v| v.get("theme").and_then(|t| t.as_str()).map(|t| t.starts_with("light")))
                .unwrap_or(false);
            parse_agents(agents).map(|b| {
                let talks = parse_peers(peers, &b);
                (b, light, talks)
            })
        });
        {
            let mut f = feed.lock().unwrap();
            match r {
                Ok((b, light, talks)) => {
                    for (msg, from, to) in talks {
                        if heard.insert(msg) && !first {
                            f.talks.push((from, to));
                        }
                    }
                    first = false;
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

fn demo(feed: Arc<Mutex<Feed>>, ctx: egui::Context) {
    let h = home().display().to_string();
    let bots = [
        ("burrow", "burrow"), ("burrow", "talk-ui"), ("burrow", "icons"), ("hexant", "hexant"),
        ("hexant", "ants-2"), ("website", "website"), ("", "home"), ("football", "football"),
    ];
    let mut bots: Vec<Bot> = bots
        .iter()
        .enumerate()
        .map(|(i, (dir, name))| Bot {
            id: format!("demo{i:04}"),
            name: name.to_string(),
            cwd: if dir.is_empty() { h.clone() } else { format!("{h}/projects/{dir}") },
            busy: i % 3 == 0,
            background: i != 6,
            started: i as u64,
            pid: 1000 + i as u64,
            session: format!("demo{i:04}"),
        })
        .collect();
    let mut seed = 0x2545f491u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    loop {
        {
            let mut f = feed.lock().unwrap();
            for b in bots.iter_mut() {
                if rnd() % 12 == 0 {
                    b.busy = !b.busy;
                }
            }
            if rnd() % 3 == 0 {
                let (a, b) = (rnd() as usize % bots.len(), rnd() as usize % bots.len());
                if a != b {
                    f.talks.push((bots[a].id.clone(), bots[b].id.clone()));
                }
            }
            f.bots = bots.clone();
            f.error = None;
            f.fresh = true;
        }
        ctx.request_repaint();
        std::thread::sleep(Duration::from_secs(3));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(id: &str, name: &str, pid: u64, session: &str) -> Bot {
        Bot { id: id.into(), name: name.into(), cwd: "/x".into(), busy: false, background: true, started: 0, pid, session: session.into() }
    }

    #[test]
    fn peers() {
        // as the poll prints it, from a real transcript (Claude Code 2.1.304)
        let out = r#"b10ba1ba-6fb5-4f3d-b231-a394dd4add37 "origin":{"kind":"peer","from":"uds:/tmp/cc-socks/27312.sock","verifiedPeerPid":27312,"msg_id":"c363f3ab-b3d0-4dc2-99ab-11fe507efcd8","name":"burrow-msg-test","hopChain":["9707c300dd0b4b521fd65bc1"],"fromMode":"prompting","body":"pong from b — got your \"ping from a\"."
"#;
        let a = bot("b10ba1ba", "burrow-msg-a", 27327, "b10ba1ba-6fb5-4f3d-b231-a394dd4add37");
        let b = bot("627daf32", "burrow-msg-test", 27312, "627daf32-638a-49dd-a3eb-307149ac3198");
        let t = parse_peers(out, &[a.clone(), b.clone()]);
        assert_eq!(t, vec![("c363f3ab-b3d0-4dc2-99ab-11fe507efcd8".into(), "627daf32".into(), "b10ba1ba".into())]);
        // a restarted sender has another pid: fall back to its name
        let b2 = Bot { pid: 1, ..b };
        assert_eq!(parse_peers(out, &[a.clone(), b2]).len(), 1);
        // a sender that isn't there any more: nothing to show
        assert!(parse_peers(out, &[a]).is_empty());
    }
}
