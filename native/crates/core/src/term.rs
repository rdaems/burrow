//! The terminals: a session attached (`claude attach <id>` in a pty, here or over `ssh -t`), your shell at
//! a project's screen (one per screen, kept by the hub, its recent output replayed), and the ptys burrow
//! types into itself: an answer delivered, /clear (the neuralyzer), the trust prompt of a new directory.

use crate::hub::{Hub, Res};
use crate::runner::{run, Mach, Pty, Runner};
use crate::util::{self, sq};
use regex::bytes::Regex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::time::{Duration, Instant};

const COLS: u16 = 120;
const ROWS: u16 = 40;

/// a program's output, read in a thread of its own until it ends or is let go: each piece to `out`
fn pump(pty: Arc<Pty>, closed: Arc<AtomicBool>, out: Sender<Vec<u8>>) {
    std::thread::spawn(move || {
        while !closed.load(Ordering::Relaxed) {
            match pty.read_timeout(Duration::from_millis(500)) {
                Some(b) if b.is_empty() => break,
                Some(b) => {
                    if out.send(b).is_err() {
                        break;
                    }
                }
                None => {}
            }
        }
    });
}

/// a session attached: its screen's bytes in `output` (disconnected once the attach ends), keys in.
/// Letting it go only ends the attach: the session goes on
pub struct Term {
    pty: Arc<Pty>,
    closed: Arc<AtomicBool>,
    pub output: Receiver<Vec<u8>>,
}

impl Term {
    pub fn write(&self, data: &[u8]) {
        let _ = self.pty.write(data);
    }
    pub fn resize(&self, cols: u16, rows: u16) {
        self.pty.resize(cols, rows);
    }
    pub fn pid(&self) -> i32 {
        self.pty.pid()
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        self.pty.signal(libc::SIGHUP);
        self.pty.reap(false);
    }
}

/// `claude attach <id>` in a pty (an over session restarts)
pub fn attach(run: &dyn Runner, m: &Mach, id: &str, cols: u16, rows: u16) -> Res<Term> {
    let a = run.argv(m, &format!("exec claude attach {}", sq(id)), true);
    let pty = Arc::new(Pty::spawn(&a, None, cols, rows).map_err(|e| e.to_string())?);
    let (tx, rx) = channel();
    let closed = Arc::new(AtomicBool::new(false));
    pump(pty.clone(), closed.clone(), tx);
    Ok(Term { pty, closed, output: rx })
}

// ---- your terminal at a project's screen: one login shell per directory (the project's, or a worktree's
// laptop), ad hoc, for a quick look or a command (the bots do the work). It lives as long as the hub does:
// a tile only attaches to it, closing the tile leaves it running, `exit` ends it. Its recent output is kept
// and replayed to the next attach. A cd elsewhere inside it is its own business

/// bytes of output kept for the next attach
const KEEP: usize = 200_000;

struct Kept {
    buf: Vec<u8>,
    socks: Vec<(u64, Sender<Vec<u8>>)>,
}

pub struct Shell {
    pub key: (String, String),
    pty: Arc<Pty>,
    kept: Mutex<Kept>,
    last: Mutex<f64>,
    busy: AtomicBool,
    alive: AtomicBool,
    next: AtomicU64,
}

impl Shell {
    pub(crate) fn start(hub: &Arc<Hub>, m: &Mach, d: &str) -> Res<Arc<Shell>> {
        let a = hub.runner().shell_argv(m, d);
        let pty = Arc::new(Pty::spawn(&a, None, 80, 24).map_err(|e| e.to_string())?);
        let sh = Arc::new(Shell {
            key: (m.id.clone(), d.to_string()),
            pty,
            kept: Mutex::new(Kept { buf: vec![], socks: vec![] }),
            last: Mutex::new(0.0),
            busy: AtomicBool::new(false),
            alive: AtomicBool::new(true),
            next: AtomicU64::new(0),
        });
        let (s, h) = (sh.clone(), Arc::downgrade(hub));
        std::thread::spawn(move || s.read(h));
        Ok(sh)
    }

    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// output in the last 1.5 s
    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    /// the pty's output: kept, and sent on to whoever is attached; busy while it comes
    fn read(self: Arc<Shell>, hub: Weak<Hub>) {
        let shells = |h: &Weak<Hub>| {
            if let Some(h) = h.upgrade() {
                h.push_shells();
            }
        };
        loop {
            match self.pty.read_timeout(Duration::from_millis(500)) {
                Some(b) if b.is_empty() => break,
                Some(b) => {
                    {
                        let mut k = self.kept.lock().unwrap();
                        k.buf.extend_from_slice(&b);
                        if k.buf.len() > KEEP {
                            let cut = k.buf.len() - KEEP;
                            k.buf.drain(..cut);
                        }
                        k.socks.retain(|(_, s)| s.send(b.clone()).is_ok());
                    }
                    *self.last.lock().unwrap() = util::now();
                    if !self.busy.swap(true, Ordering::Relaxed) {
                        shells(&hub);
                    }
                }
                None => {
                    if self.busy() && util::now() - *self.last.lock().unwrap() > 1.5 {
                        self.busy.store(false, Ordering::Relaxed);
                        shells(&hub);
                    }
                }
            }
        }
        self.alive.store(false, Ordering::Relaxed);
        self.kept.lock().unwrap().socks.clear(); // the attached tiles see it end
        self.pty.reap(false);
        if let Some(h) = hub.upgrade() {
            h.drop_shell(&self);
        }
    }

    /// a tile attached: the kept output first, then what comes
    pub fn attach(self: &Arc<Shell>) -> ShellTerm {
        let (tx, rx) = channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut k = self.kept.lock().unwrap();
        if !k.buf.is_empty() {
            let _ = tx.send(k.buf.clone());
        }
        if self.alive() {
            k.socks.push((id, tx));
        }
        ShellTerm { shell: self.clone(), id, output: rx }
    }

    fn detach(&self, id: u64) {
        self.kept.lock().unwrap().socks.retain(|(i, _)| *i != id);
    }

    pub fn write(&self, data: &[u8]) {
        let _ = self.pty.write(data);
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        self.pty.resize(cols, rows);
    }

    pub fn kill(&self) {
        self.pty.signal(libc::SIGHUP);
    }
}

/// a tile on your terminal: its output (disconnected once the shell ends), keys in; letting it go only
/// detaches
pub struct ShellTerm {
    pub shell: Arc<Shell>,
    id: u64,
    pub output: Receiver<Vec<u8>>,
}

impl ShellTerm {
    pub fn write(&self, data: &[u8]) {
        self.shell.write(data);
    }
    pub fn resize(&self, cols: u16, rows: u16) {
        self.shell.resize(cols, rows);
    }
}

impl Drop for ShellTerm {
    fn drop(&mut self) {
        self.shell.detach(self.id);
    }
}

impl Hub {
    /// a session attached for a tile (`claude attach`); the demo's office manager is real when it is
    pub fn attach(&self, mid: &str, id: &str, cols: u16, rows: u16) -> Res<Term> {
        let real_mgr = self.cfg.demo
            && self.cfg.real_office
            && self.lock().room.get("managers").and_then(|m| m.as_object()).is_some_and(|ms| ms.values().any(|m| m.get("id").and_then(|i| i.as_str()) == Some(id)));
        let m = if real_mgr { self.local() } else { self.machine(mid)? };
        attach(self.runner(), &m, id, cols, rows)
    }

    /// your terminal at that screen, started if there is none
    pub fn shell(&self, mid: &str, dir: &str) -> Res<ShellTerm> {
        let m = self.machine(mid)?;
        let sh = {
            let mut g = self.lock();
            let key = (m.id.clone(), dir.to_string());
            match g.shells.get(&key).filter(|s| s.alive()) {
                Some(s) => return Ok(s.attach()),
                None => {
                    let s = Shell::start(&self.arc(), &m, dir)?;
                    g.shells.insert(key, s.clone());
                    s
                }
            }
        };
        self.push_shells();
        Ok(sh.attach())
    }

    pub(crate) fn drop_shell(&self, sh: &Arc<Shell>) {
        {
            let mut g = self.lock();
            if g.shells.get(&sh.key).is_some_and(|s| Arc::ptr_eq(s, sh)) {
                g.shells.shift_remove(&sh.key);
            }
        }
        self.push_shells();
    }

    /// a desk taken away takes its terminals with it: the project's and its worktrees'
    pub fn kill_shells(&self, unit: &str) {
        let (mid, d) = if unit.starts_with('/') { (String::new(), unit.to_string()) } else { unit.split_once(':').map(|(a, b)| (a.into(), b.into())).unwrap_or_default() };
        let g = self.lock();
        let doomed: Vec<Arc<Shell>> = g
            .shells
            .iter()
            .filter(|(k, _)| k.0 == mid && (k.1 == d || g.folds.get(*k).is_some_and(|f| f.0 == d)))
            .map(|(_, s)| s.clone())
            .collect();
        drop(g);
        for s in doomed {
            s.kill();
        }
    }

    /// a file dropped on a session: kept here, and copied over to a remote machine; its path there
    pub fn upload(&self, mid: &str, name: &str, data: &[u8]) -> Res<String> {
        let m = self.machine(mid)?;
        let name = std::path::Path::new(if name.is_empty() { "file" } else { name }).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        let stamp = util::now_ms().to_string();
        let d = self.cfg.cache.join("drop").join(&stamp);
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        let local = d.join(&name);
        std::fs::write(&local, data).map_err(|e| e.to_string())?;
        if m.ssh.is_empty() || m.demo() {
            return Ok(local.to_string_lossy().into_owned());
        }
        let there = run(self.runner(), &m, &format!("d=\"$HOME/.cache/burrow/drop/{stamp}\"; mkdir -p \"$d\" && echo \"$d\""), Duration::from_secs(60))?.trim().to_string() + "/" + &name;
        self.runner().scp(&m, &local, &there)?;
        Ok(there)
    }
}

static CSI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[A-Za-z]").unwrap());
static CSI_ALL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\s").unwrap());

/// an answer typed into a background session without a terminal: `claude attach` in a pty of its own,
/// the text pasted at its prompt once the screen has settled (a session whose process was off is
/// restarted by the attach, which takes longer), Enter, a moment for it to be taken, then the attach is
/// let go: the session goes on with it. Claude Code has no command to send a session a message
pub fn deliver(run: &dyn Runner, m: &Mach, bot: &str, text: &str, timeout: Duration) -> Res<()> {
    let a = run.argv(m, &format!("exec claude attach {}", sq(bot)), true);
    let pty = Pty::spawn(&a, None, COLS, ROWS).map_err(|e| e.to_string())?;
    let mut seen: Vec<u8> = vec![];
    let t0 = Instant::now();
    // what comes, until it has been quiet `quiet` seconds after something came, or `until` passed
    let read = |seen: &mut Vec<u8>, quiet: f64, until: Instant| -> bool {
        let (mut got, mut last) = (false, Instant::now());
        while Instant::now() < until {
            match pty.read_timeout(Duration::from_millis(200)) {
                Some(b) if b.is_empty() => return false,
                Some(b) => {
                    seen.extend(b);
                    got = true;
                    last = Instant::now();
                }
                None if got && last.elapsed().as_secs_f64() > quiet => return true,
                None => {}
            }
        }
        got
    };
    let r = (|| {
        if !read(&mut seen, 1.5, t0 + timeout) {
            return Err("the session did not come up".to_string());
        }
        let screen = String::from_utf8_lossy(&CSI.replace_all(&seen, &b""[..])).into_owned();
        if screen.contains("No job matching") || screen.contains("describe a task for a new session") {
            return Err("could not attach to the session".into());
        }
        let mut paste = b"\x1b[200~".to_vec(); // pasted: new lines stay in the message
        paste.extend(text.as_bytes());
        paste.extend(b"\x1b[201~");
        let _ = pty.write(&paste);
        std::thread::sleep(Duration::from_millis(500));
        let _ = pty.write(b"\r");
        read(&mut seen, 1.0, Instant::now() + Duration::from_secs(4));
        Ok(())
    })();
    pty.signal(libc::SIGHUP);
    if let Some(mut c) = pty.close() {
        std::thread::spawn(move || {
            let _ = c.wait();
        });
    }
    r
}

/// read until it has said something and then nothing for `quiet` seconds (`most` at most)
fn settle(pty: &Pty, quiet: f64, most: f64) {
    let end = Instant::now() + Duration::from_secs_f64(most);
    let (mut last, mut got) = (Instant::now(), false);
    while Instant::now() < end && !(got && last.elapsed().as_secs_f64() >= quiet) {
        match pty.read_timeout(Duration::from_millis(100)) {
            Some(b) if b.is_empty() => break,
            Some(_) => {
                last = Instant::now();
                got = true;
            }
            None => {}
        }
    }
}

/// a program in a pty let go: each signal (stopping once it is gone), then reaped and the pty closed,
/// in that order or the other (`close_first`: a killed claude hangs in its exit while its output is unread, macOS)
fn let_go(pty: Pty, sigs: &[i32], close_first: bool) {
    for &s in sigs {
        if !pty.signal(s) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if close_first {
        if let Some(mut c) = pty.close() {
            let _ = c.wait();
        }
    } else {
        pty.reap(true);
        pty.close();
    }
}

/// the neuralyzer: `/clear` typed into a session as you would type it. Attached in a pty, its screen
/// drawn until the output settles, the command and Enter, a moment for it to take, then let go. Claude
/// Code keeps the job and its process; the conversation is a new one (a new session id)
pub fn clear_session(run: &dyn Runner, m: &Mach, bot: &str) {
    let a = run.argv(m, &format!("exec claude attach {}", sq(bot)), true);
    let Ok(pty) = Pty::spawn(&a, None, COLS, ROWS) else { return };
    settle(&pty, 1.5, 25.0);
    let _ = pty.write(b"/clear");
    std::thread::sleep(Duration::from_millis(500));
    let _ = pty.write(b"\r");
    settle(&pty, 1.5, 8.0);
    let_go(pty, &[libc::SIGTERM, libc::SIGKILL], false);
}

/// Claude Code's trust prompt answered yes for a directory of this computer, as you would in `claude` there:
/// `claude --bg` refuses a directory nobody has trusted ("Workspace not trusted"). Only for burrow's own office
/// directories (nothing in them but the CLAUDE.md burrow writes). The prompt's cursor starts on "No, exit": ↓,
/// Enter; the session that starts then is let go. Without the calling session's CLAUDE_* variables, or the
/// answer is not kept
pub fn trust(run: &dyn Runner, local: &Mach, d: &str) {
    let a = run.argv(local, &format!("cd {} && exec claude", sq(d)), true);
    let Ok(pty) = Pty::spawn(&a, Some("CLAUDE"), COLS, ROWS) else { return };
    let mut seen: Vec<u8> = vec![];
    let mut upto = |want: Option<&[u8]>, most: f64| -> bool {
        let end = Instant::now() + Duration::from_secs_f64(most);
        while Instant::now() < end {
            match pty.read_timeout(Duration::from_millis(200)) {
                Some(b) if b.is_empty() => return false,
                Some(b) => {
                    seen.extend(b);
                    if let Some(w) = want
                        && CSI_ALL.replace_all(&seen, &b""[..]).windows(w.len()).any(|x| x == w) {
                            return true;
                        }
                }
                None => {}
            }
        }
        false
    };
    if upto(Some(b"Itrustthisfolder"), 20.0) {
        std::thread::sleep(Duration::from_secs(1)); // it takes keys a moment after drawing the prompt
        let _ = pty.write(b"\x1b[B");
        std::thread::sleep(Duration::from_millis(400));
        let _ = pty.write(b"\r");
        upto(None, 4.0);
    }
    let_go(pty, &[libc::SIGHUP, libc::SIGKILL], true);
}
