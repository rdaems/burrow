//! How burrow reaches outside itself: a script in a shell on a machine (this one, or over ssh), a program
//! kept running (the sleep inhibitor), a program in a pty (a session attached, your terminal at a screen),
//! scp, and pair's HTTP (curl). One trait, so tests run with a fake that never calls claude or ssh.

use crate::util::sq;
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// a non-login shell over ssh has a bare PATH; claude lives here
pub const PATH: &str = "PATH=\"$HOME/.local/bin:$HOME/.claude/local:/opt/homebrew/bin:/usr/local/bin:$PATH\"; export PATH; export COLORTERM=truecolor; ";

/// a machine: this computer (id ""), or one of Claude Code's sshConfigs
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mach {
    pub id: String,
    pub name: String,
    /// the ssh host; "" here, "@demo" for the demo's made-up ones
    pub ssh: String,
    pub port: Option<String>,
    /// sshIdentityFile
    pub key: Option<String>,
}

impl Mach {
    pub fn demo(&self) -> bool {
        self.ssh == "@demo"
    }
}

pub struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// a program kept running until stopped
pub trait Kept: Send {
    fn running(&mut self) -> bool;
    fn stop(&mut self);
}

pub trait Runner: Send + Sync {
    /// `script` in a shell on the machine, to its end (killed after `timeout`). `input` goes to its stdin;
    /// `pid` hears the pid as soon as it runs (`claude -p` shows up in `claude agents` while it runs)
    fn exec(&self, m: &Mach, script: &str, input: Option<&[u8]>, timeout: Duration, pid: &mut dyn FnMut(u32)) -> Result<Output, String>;
    /// program + args that run `script` in a shell on the machine (`tty`: for a pty, over `ssh -t`)
    fn argv(&self, m: &Mach, script: &str, tty: bool) -> Vec<String>;
    /// your login shell in `dir` there, for a pty
    fn shell_argv(&self, m: &Mach, dir: &str) -> Vec<String> {
        let script = format!("cd {} 2>/dev/null; exec \"${{SHELL:-/bin/sh}}\" -l", sq(dir));
        if m.ssh.is_empty() || m.demo() {
            vec!["/bin/sh".into(), "-c".into(), format!("{PATH}{script}")]
        } else {
            self.argv(m, &script, true)
        }
    }
    /// a program kept running (the sleep inhibitor)
    fn keep(&self, argv: &[String]) -> io::Result<Box<dyn Kept>>;
    /// a local file copied to `there` on the machine
    fn scp(&self, m: &Mach, local: &Path, there: &str) -> Result<(), String>;
    /// one HTTP call: (status, body); a body given is POSTed as JSON
    fn http(&self, url: &str, body: Option<&Value>) -> Result<(u16, String), String>;
}

/// what `run` in server.py does: the script's stdout, or what it said went wrong
pub fn run(r: &dyn Runner, m: &Mach, script: &str, timeout: Duration) -> Result<String, String> {
    let o = r.exec(m, script, None, timeout, &mut |_| {})?;
    if o.code == 0 {
        return Ok(String::from_utf8_lossy(&o.stdout).into_owned());
    }
    let (e, s) = (String::from_utf8_lossy(&o.stderr).trim().to_string(), String::from_utf8_lossy(&o.stdout).trim().to_string());
    Err(if !e.is_empty() { e } else if !s.is_empty() { s } else { format!("exit {}", o.code) })
}

/// the real outside: /bin/sh here, ssh elsewhere, curl for pair
pub struct System {
    pub cache: PathBuf,
}

impl System {
    fn ssh_opts(&self, m: &Mach) -> Vec<String> {
        let _ = std::fs::create_dir_all(&self.cache);
        let mut o: Vec<String> = ["-o", "ControlMaster=auto", "-o"].iter().map(|s| s.to_string()).collect();
        o.push(format!("ControlPath={}/ssh-%C", self.cache.display()));
        o.extend(["-o", "ControlPersist=600", "-o", "ServerAliveInterval=15"].iter().map(|s| s.to_string()));
        if let Some(k) = m.key.as_ref().filter(|k| !k.is_empty()) {
            o.push("-i".into());
            o.push(expand(k));
        }
        o
    }
}

fn expand(p: &str) -> String {
    match (p.strip_prefix("~/"), std::env::var("HOME")) {
        (Some(r), Ok(h)) => format!("{h}/{r}"),
        _ => p.to_string(),
    }
}

/// a child's output read to its end, killed once `timeout` is past
fn finish(mut child: Child, input: Option<&[u8]>, timeout: Duration) -> Result<Output, String> {
    if let (Some(mut i), Some(data)) = (child.stdin.take(), input) {
        let data = data.to_vec();
        std::thread::spawn(move || {
            let _ = i.write_all(&data);
        });
    }
    let (mut so, mut se) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let a = std::thread::spawn(move || {
        let mut b = vec![];
        let _ = so.read_to_end(&mut b);
        b
    });
    let e = std::thread::spawn(move || {
        let mut b = vec![];
        let _ = se.read_to_end(&mut b);
        b
    });
    let end = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out".into());
            }
        }
    };
    Ok(Output { code: status.code().unwrap_or(-1), stdout: a.join().unwrap_or_default(), stderr: e.join().unwrap_or_default() })
}

struct KeptChild(Child);

impl Kept for KeptChild {
    fn running(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
    fn stop(&mut self) {
        unsafe {
            libc::kill(self.0.id() as i32, libc::SIGTERM);
        }
        let _ = self.0.wait();
    }
}

impl Runner for System {
    fn exec(&self, m: &Mach, script: &str, input: Option<&[u8]>, timeout: Duration, pid: &mut dyn FnMut(u32)) -> Result<Output, String> {
        let a = self.argv(m, script, false);
        let child = Command::new(&a[0])
            .args(&a[1..])
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        pid(child.id());
        finish(child, input, timeout)
    }

    fn argv(&self, m: &Mach, script: &str, tty: bool) -> Vec<String> {
        let script = format!("{PATH}{script}");
        if m.demo() {
            return vec!["/bin/sh".into(), "-c".into(), "printf 'a demo bot: no session behind it\\r\\n'; exec cat".into()];
        }
        if m.ssh.is_empty() {
            return vec!["/bin/sh".into(), "-c".into(), script];
        }
        let mut a = vec!["ssh".to_string()];
        a.extend(self.ssh_opts(m));
        if let Some(p) = m.port.as_ref().filter(|p| !p.is_empty()) {
            a.extend(["-p".into(), p.clone()]);
        }
        if tty {
            a.push("-t".into());
        } else {
            a.extend(["-o".into(), "BatchMode=yes".into()]);
        }
        a.extend([m.ssh.clone(), script]);
        a
    }

    fn keep(&self, argv: &[String]) -> io::Result<Box<dyn Kept>> {
        let c = Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
        Ok(Box::new(KeptChild(c)))
    }

    fn scp(&self, m: &Mach, local: &Path, there: &str) -> Result<(), String> {
        // scp speaks SFTP now: the remote path is taken literally, no quoting
        let mut c = Command::new("scp");
        c.args(self.ssh_opts(m));
        if let Some(p) = m.port.as_ref().filter(|p| !p.is_empty()) {
            c.args(["-P", p]);
        }
        let o = c.arg("-q").arg(local).arg(format!("{}:{}", m.ssh, there)).output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
        }
        Ok(())
    }

    fn http(&self, url: &str, body: Option<&Value>) -> Result<(u16, String), String> {
        // with a user agent of its own: pair.daems.me is behind Cloudflare, whose browser check answers a
        // library's default one with a 403 (error code 1010)
        let mut c = Command::new("curl");
        c.args(["-sSL", "-m", "15", "-H", "Accept: application/json", "-H", "User-Agent: burrow (+https://github.com/rdaems/burrow)", "-w", "\n%{http_code}"]);
        if body.is_some() {
            c.args(["-H", "Content-Type: application/json", "--data-binary", "@-"]);
        }
        let child = c.arg("--").arg(url).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        let input = body.map(|b| b.to_string().into_bytes());
        let o = finish(child, Some(input.as_deref().unwrap_or(b"")), Duration::from_secs(20))?;
        let out = String::from_utf8_lossy(&o.stdout).into_owned();
        if o.code != 0 {
            return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
        }
        let (text, code) = out.rsplit_once('\n').unwrap_or(("", &out));
        Ok((code.trim().parse().unwrap_or(0), text.to_string()))
    }
}

// ---- a program in a pty

/// a pty with a program in it: its output read from `reader()`, keys written, its size set, signalled
pub struct Pty {
    master: OwnedFd,
    child: std::sync::Mutex<Option<Child>>,
    pid: i32,
}

impl Pty {
    /// `argv` in a new pty, TERM and COLORTERM set; `drop_env`: variables with that prefix left out
    pub fn spawn(argv: &[String], drop_env: Option<&str>, cols: u16, rows: u16) -> io::Result<Pty> {
        let (mut m, mut s) = (0, 0);
        let mut ws = libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 };
        if unsafe { libc::openpty(&mut m, &mut s, std::ptr::null_mut(), std::ptr::null_mut(), &mut ws) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let (master, slave) = unsafe { (OwnedFd::from_raw_fd(m), OwnedFd::from_raw_fd(s)) };
        unsafe {
            libc::fcntl(m, libc::F_SETFD, libc::FD_CLOEXEC);
        }
        let mut c = Command::new(&argv[0]);
        c.args(&argv[1..]);
        if let Some(p) = drop_env {
            for (k, _) in std::env::vars_os() {
                if k.to_string_lossy().starts_with(p) {
                    c.env_remove(&k);
                }
            }
        }
        c.env("TERM", "xterm-256color").env("COLORTERM", "truecolor");
        c.stdin(Stdio::from(slave.try_clone()?)).stdout(Stdio::from(slave.try_clone()?)).stderr(Stdio::from(slave));
        unsafe {
            c.pre_exec(|| {
                // its own session, the pty its controlling terminal
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = c.spawn()?;
        drop(c); // the slave's copies close with the command
        let pid = child.id() as i32;
        Ok(Pty { master, child: std::sync::Mutex::new(Some(child)), pid })
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// a reader of its output (a copy of the master); ends when the program does
    pub fn reader(&self) -> io::Result<std::fs::File> {
        Ok(std::fs::File::from(self.master.try_clone()?))
    }

    pub fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut off = 0;
        while off < data.len() {
            let n = unsafe { libc::write(self.master.as_raw_fd(), data[off..].as_ptr() as *const libc::c_void, data.len() - off) };
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            off += n as usize;
        }
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let ws = libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 };
        unsafe {
            libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws);
        }
    }

    /// a signal to the program; false if it is gone
    pub fn signal(&self, sig: i32) -> bool {
        unsafe { libc::kill(self.pid, sig) == 0 }
    }

    /// what is there to read within `timeout`: Some(bytes), Some(empty) at its end, None if nothing came
    pub fn read_timeout(&self, timeout: Duration) -> Option<Vec<u8>> {
        let mut p = libc::pollfd { fd: self.master.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let n = unsafe { libc::poll(&mut p, 1, timeout.as_millis() as i32) };
        if n <= 0 {
            return None;
        }
        let mut buf = vec![0u8; 65536];
        let k = unsafe { libc::read(self.master.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if k <= 0 {
            return Some(vec![]);
        }
        buf.truncate(k as usize);
        Some(buf)
    }

    /// its end waited for (it is reaped); in a thread of its own unless `here`
    pub fn reap(&self, here: bool) {
        let Some(mut c) = self.child.lock().unwrap().take() else { return };
        if here {
            let _ = c.wait();
        } else {
            std::thread::spawn(move || {
                let _ = c.wait();
            });
        }
    }

    /// the master closed (the pty goes when every reader's copy has gone too)
    pub fn close(self) -> Option<Child> {
        let c = self.child.lock().unwrap().take();
        drop(self.master);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_for_each_kind_of_machine() {
        let sys = System { cache: PathBuf::from("/c") };
        let here = Mach::default();
        assert_eq!(sys.argv(&here, "true", false), ["/bin/sh", "-c", &format!("{PATH}true")]);
        let lab = Mach { id: "lab".into(), name: "lab".into(), ssh: "lab.local".into(), port: Some("2222".into()), key: Some("/k".into()) };
        let a = sys.argv(&lab, "true", true);
        assert_eq!(a[0], "ssh");
        assert!(a.windows(2).any(|w| w == ["-p", "2222"]) && a.windows(2).any(|w| w == ["-i", "/k"]) && a.contains(&"-t".to_string()));
        assert_eq!(a[a.len() - 2], "lab.local");
        assert!(sys.argv(&lab, "true", false).windows(2).any(|w| w == ["-o", "BatchMode=yes"]));
        let demo = Mach { ssh: "@demo".into(), ..Mach::default() };
        assert!(sys.argv(&demo, "claude attach x", true)[2].contains("a demo bot"));
    }

    #[test]
    fn a_pty_both_ways() {
        let p = Pty::spawn(&["/bin/sh".into(), "-c".into(), "stty -echo; read x; echo got:$x; stty size".into()], None, 90, 30).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        p.write(b"hello\n").unwrap();
        let mut seen = vec![];
        let end = Instant::now() + Duration::from_secs(5);
        while Instant::now() < end {
            match p.read_timeout(Duration::from_millis(200)) {
                Some(b) if b.is_empty() => break,
                Some(b) => seen.extend(b),
                None => {}
            }
        }
        let s = String::from_utf8_lossy(&seen);
        assert!(s.contains("got:hello") && s.contains("30 90"), "{s}");
        p.reap(true);
    }
}
