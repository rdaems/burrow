//! The hub driven as the app drives it, on a fake outside (no claude, no ssh) or in the demo, in a
//! temporary home. Fixtures are made up after the formats server.py reads.

use burrow_core::runner::{Kept, Output};
use burrow_core::{ChessAction, Config, Event, Hub, Mach, Runner};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// the outside, made up: a script gets the reply of the first key it contains
#[derive(Default)]
struct Fake {
    replies: Mutex<Vec<(String, Vec<Result<String, String>>)>>,
    calls: Mutex<Vec<String>>,
    started: AtomicUsize,
    stopped: Arc<AtomicUsize>,
    /// pair's answers, by a part of the url
    web: Mutex<Vec<(String, u16, String)>>,
}

impl Fake {
    /// the reply from now on
    fn on(&self, key: &str, reply: Result<&str, &str>) -> &Fake {
        self.replies.lock().unwrap().retain(|(k, _)| k != key);
        self.then(key, reply)
    }
    /// a reply after the ones there are
    fn then(&self, key: &str, reply: Result<&str, &str>) -> &Fake {
        let r = reply.map(String::from).map_err(String::from);
        let mut rs = self.replies.lock().unwrap();
        match rs.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => v.push(r),
            None => rs.push((key.into(), vec![r])),
        }
        self
    }
    fn called(&self, key: &str) -> usize {
        self.calls.lock().unwrap().iter().filter(|c| c.contains(key)).count()
    }
}

struct FakeKept(bool, Arc<AtomicUsize>);

impl Kept for FakeKept {
    fn running(&mut self) -> bool {
        self.0
    }
    fn stop(&mut self) {
        self.0 = false;
        self.1.fetch_add(1, Ordering::SeqCst);
    }
}

impl Runner for Fake {
    fn exec(&self, _m: &Mach, script: &str, _input: Option<&[u8]>, _t: Duration, pid: &mut dyn FnMut(u32)) -> Result<Output, String> {
        self.calls.lock().unwrap().push(script.to_string());
        pid(4242);
        let mut rs = self.replies.lock().unwrap();
        let Some((_, v)) = rs.iter_mut().find(|(k, _)| script.contains(k.as_str())) else {
            return Ok(Output { code: 0, stdout: vec![], stderr: vec![] });
        };
        // replies are taken in turn; the last one stays
        let r = if v.len() > 1 { v.remove(0) } else { v[0].clone() };
        Ok(match r {
            Ok(s) => Output { code: 0, stdout: s.into_bytes(), stderr: vec![] },
            Err(e) => Output { code: 1, stdout: vec![], stderr: e.into_bytes() },
        })
    }
    fn argv(&self, _m: &Mach, script: &str, _tty: bool) -> Vec<String> {
        self.calls.lock().unwrap().push(format!("pty: {script}"));
        let say = if script.contains("exec claude attach") { "printf 'attached\\n'; exec cat" } else { "printf 'Do you trust it?\\n\\033[1m I trust\\033[0m this folder\\n'; sleep 1.5" };
        vec!["/bin/sh".into(), "-c".into(), say.into()]
    }
    fn shell_argv(&self, _m: &Mach, dir: &str) -> Vec<String> {
        vec!["/bin/sh".into(), "-c".into(), format!("echo ready in {dir}; while read l; do echo \"got $l\"; done")]
    }
    fn keep(&self, _argv: &[String]) -> std::io::Result<Box<dyn Kept>> {
        self.started.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(20)); // as slow as starting a program: a race shows
        Ok(Box::new(FakeKept(true, self.stopped.clone())))
    }
    fn scp(&self, _m: &Mach, _l: &Path, _t: &str) -> Result<(), String> {
        Ok(())
    }
    fn http(&self, url: &str, body: Option<&Value>) -> Result<(u16, String), String> {
        self.calls.lock().unwrap().push(format!("http: {url} {}", body.map(|b| b.to_string()).unwrap_or_default()));
        let web = self.web.lock().unwrap();
        Ok(web.iter().find(|(k, _, _)| url.contains(k.as_str())).map(|(_, c, t)| (*c, t.clone())).unwrap_or((404, String::new())))
    }
}

fn hub(dir: &Path, demo: bool, fake: Arc<Fake>) -> Arc<Hub> {
    let mut cfg = Config::under(dir, demo);
    cfg.inhibitor = Some(vec!["caffeinate".into()]);
    std::fs::create_dir_all(cfg.claude_settings.parent().unwrap()).unwrap();
    Hub::new(cfg, fake)
}

/// the first state pushed that `ok` likes, within `secs`
fn wait_state(ev: &burrow_core::Events, secs: u64, ok: impl Fn(&burrow_model::State) -> bool) -> burrow_model::State {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if let Ok(Event::State(s)) = ev.recv_timeout(Duration::from_millis(100)) {
            if ok(&s) {
                return *s;
            }
        }
    }
    panic!("no such state within {secs} s");
}

fn wait(secs: u64, ok: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("not within {secs} s");
}

const AGENTS: &str = r#"[
  {"id": "b1", "kind": "background", "name": "busy-bea", "state": "working", "status": "busy", "pid": 501, "cwd": "/p/one", "sessionId": "s1", "startedAt": 1000},
  {"id": "b2", "kind": "background", "name": "tree-tom", "state": "done", "status": "idle", "pid": 502, "cwd": "/p/wt", "sessionId": "s2", "startedAt": 2000},
  {"id": "b3", "kind": "background", "name": "old-olga", "state": "done", "cwd": "/p/two", "sessionId": "s3", "startedAt": 3000},
  {"id": "b4", "kind": "background", "name": "usage-run", "status": "busy", "pid": 4242, "cwd": "/p/one", "startedAt": 4000},
  {"id": "i1", "kind": "interactive", "status": "busy", "pid": 600, "cwd": "/p/one"}
]"#;

#[test]
fn a_poll_of_this_computer() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    let jobs = "/h/.claude/jobs/b1/state.json:\"tempo\": \"active\"\n/h/.claude/jobs/b3/state.json:\"updatedAt\": \"2026-10-24T12:00:00Z\"";
    fake.on("claude agents --json --all; echo", Ok(&format!("{AGENTS}\n@@burrow@@\n{{\"theme\": \"light-daltonized\"}}\n@@burrow@@\n\n@@burrow@@\n{jobs}\n")))
        // FOLD for the new cwds, sorted: /p/one, /p/two, /p/wt (a linked worktree of /p/one)
        .on("git rev-parse", Ok("/p/one\t/p/one/.git\n\t\n/p/wt\t/p/one/.git\n"))
        .on("claude -p /usage", Ok("Current session: 41% used · resets 5pm\n"));
    let h = hub(dir.path(), false, fake.clone());
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.usage.session.is_some());
    // a round once /usage has run (its pid is burrow's own from then on)
    h.focus(true);
    let s = wait_state(&ev, 5, |s| s.bots.len() == 3);
    assert_eq!(s.machines.len(), 1);
    assert!(s.machines[0].light && s.machines[0].error.is_none());
    // the interactive row left out, and the pid of burrow's own `claude -p` (b4)
    assert_eq!(s.bots.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["b1", "b2", "b3"]);
    let b2 = &s.bots[1];
    assert_eq!((b2.unit.as_str(), b2.tree.as_str()), ("/p/one", "/p/wt"));
    assert!(s.bots[0].busy && !b2.busy && s.bots[2].over == "done" && s.bots[2].ended_at == 1792843200000);
    assert_eq!(s.desks.iter().map(|d| d.unit.as_str()).collect::<Vec<_>>(), ["/p/one", "/p/two"]);
    assert!(s.desks.iter().all(|d| !d.kept));
    assert_eq!(s.usage.session.unwrap().used, 41);
    // a desk kept from the cabinet: `pwd -P` there, then for good
    fake.on("pwd -P", Ok("/p/three\n"));
    assert_eq!(h.dir("", "~/p/three").unwrap(), "/p/three");
    let s = wait_state(&ev, 2, |s| s.desks.iter().any(|d| d.unit == "/p/three"));
    assert!(s.desks.iter().find(|d| d.unit == "/p/three").unwrap().kept);
    h.rename("/p/three", "Three").unwrap();
    assert_eq!(h.desks().iter().find(|d| d.unit == "/p/three").unwrap().name, "Three");
    assert!(h.rename("/nowhere", "x").is_err());
    // a poll that fails: the machine says why
    fake.on("claude agents --json --all; echo", Err("ssh: connect to host lab: Connection refused"));
    h.focus(true);
    let s = wait_state(&ev, 5, |s| s.machines[0].error.is_some());
    assert!(s.machines[0].error.as_deref().unwrap().contains("Connection refused"));
}

#[test]
fn spawn_and_stop() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    fake.on("claude --bg", Ok("Starting background service…\nbackgrounded · 7ebbc505 (idle — send a prompt)\n"));
    let h = hub(dir.path(), false, fake.clone());
    let here = Mach::default();
    assert_eq!(h.spawn_in(&here, "/p/one", "", "-v please", "").unwrap(), "7ebbc505");
    let call = fake.calls.lock().unwrap().iter().find(|c| c.contains("claude --bg")).unwrap().clone();
    assert!(call.starts_with("cd '/p/one' && claude --bg --remote-control -n '"), "{call}");
    assert!(call.ends_with(" 'Task: -v please' 2>&1"), "{call}");
    // the switchboard: no Remote Control, yolo
    h.settings(&[("rc", false), ("yolo", true), ("nonsense", true)]);
    fake.on("claude --bg", Ok("backgrounded · abc · named (idle)\n"));
    assert_eq!(h.spawn_in(&here, "/p", "named", "", "").unwrap(), "abc");
    assert!(fake.calls.lock().unwrap().iter().any(|c| c.contains("claude --bg --dangerously-skip-permissions -n 'named' 2>&1")));
    // what claude says when it will not: the error
    fake.on("claude --bg", Err("Error: Workspace not trusted"));
    assert_eq!(h.spawn_in(&here, "/p", "x", "", "").unwrap_err(), "Error: Workspace not trusted");
}

#[test]
fn the_room_kept_in_room_json() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    let h = hub(dir.path(), false, fake.clone());
    let key = h.add_furniture("plant", [40.7, 90.0]).unwrap();
    assert_eq!(key, "plant2");
    assert!(h.add_furniture("throne", [0.0, 0.0]).is_err());
    assert!(h.add_furniture("manager", [0.0, 0.0]).is_err());
    h.remove_furniture("plant0").unwrap();
    assert_eq!(h.remove_furniture("fire").unwrap_err(), "that stays");
    h.add_furniture("radio", [1.0, 1.0]).unwrap();
    h.paint("lab", Some(3));
    h.set_width(5000.0);
    h.you(Some("  Jo  Doe "), Some("jo:4"), None);
    h.settings(&[("hardcore", true)]);
    let s = h.state();
    assert_eq!(s.layout["plant2"], [40, 90]);
    assert!(!s.furniture.contains(&"plant0".to_string()));
    assert_eq!((s.you.name.as_str(), s.you.seed.as_str(), s.you.login.as_str()), ("Jo Doe", "jo4", "tester"));
    assert_eq!(s.width, Some(1200.0));
    assert!(s.settings.hardcore && s.settings.sound && s.settings.rc);
    // a second hub reads what the first wrote, as the web app would
    let cfg = Config::under(dir.path(), false);
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(cfg.room()).unwrap()).unwrap();
    assert_eq!(raw["v"], 13);
    assert_eq!(raw["paint"]["lab"], 3);
    assert_eq!(raw["you"], json!({"name": "Jo Doe", "seed": "jo4"}));
    let again = Hub::new(cfg, fake);
    let s2 = again.state();
    assert_eq!((s2.layout, s2.furniture, s2.you, s2.width, s2.paint), (s.layout, s.furniture, s.you, s.width, s.paint));
    // a name emptied is the login name again
    again.you(Some(""), None, None);
    assert_eq!(again.state().you.name, "tester");
}

#[test]
fn a_desk_fades_two_minutes_after_its_last_session() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), false, Arc::new(Fake::default()));
    h.set_layout(&[("u:/gone".into(), [10.0, 20.0]), ("u:/kept".into(), [30.0, 20.0]), ("u:lab:/x".into(), [1.0, 1.0]), ("plant0".into(), [5.0, 5.0])]);
    h.keep("/kept", true);
    h.fade("", 1000.0);
    h.fade("", 1100.0);
    assert!(h.state().layout.contains_key("u:/gone"));
    h.fade("", 1121.0);
    let l = h.state().layout;
    assert!(!l.contains_key("u:/gone") && l.contains_key("u:/kept") && l.contains_key("u:lab:/x") && l.contains_key("plant0"));
    // burnt: gone with its place and name
    h.forget("/kept");
    assert!(!h.state().layout.contains_key("u:/kept"));
}

#[test]
fn the_sleep_inhibitor_runs_once() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    let h = hub(dir.path(), false, fake.clone());
    assert!(h.awake("for", 0).is_err() && h.awake("sometimes", 1).is_err());
    let a = h.awake("for", 60).unwrap();
    assert!(a.on && a.mode == "for" && a.until > 0);
    // every machine's poll ticks it at once
    let ts: Vec<_> = (0..8).map(|_| {
        let h = h.clone();
        std::thread::spawn(move || h.awake_tick())
    }).collect();
    ts.into_iter().for_each(|t| t.join().unwrap());
    assert_eq!(fake.started.load(Ordering::SeqCst), 1);
    assert!(!h.awake("off", 0).unwrap().on);
    assert_eq!(fake.stopped.load(Ordering::SeqCst), 1);
    // "work" is kept in room.json; with no local bot at work it stays off
    h.awake("work", 0).unwrap();
    assert!(!h.awake_view().on);
    let raw = std::fs::read_to_string(Config::under(dir.path(), false).room()).unwrap();
    assert!(raw.contains("\"awake\": {\n    \"mode\": \"work\"\n  }"), "{raw}");
}

#[test]
fn her_directory_trusted_once() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    fake.on("claude --bg", Err("Error: Workspace not trusted. Run claude there first."));
    fake.then("claude --bg", Ok("backgrounded · s0san · Susan (idle)\n"));
    let h = hub(dir.path(), false, fake.clone());
    let key = h.hire("ask", Some([100.0, 120.0])).unwrap();
    assert_eq!(key, "manager1"); // Linda, the notes, is at manager0 in a new room
    assert!(h.hire("ask", None).is_err());
    wait(10, || h.state().managers.iter().any(|m| m.key == "manager1" && m.id == "s0san"));
    assert_eq!(fake.called("claude --bg"), 2);
    assert_eq!(fake.called("pty: cd "), 1); // the trust prompt, once, in her directory
    let office = Config::under(dir.path(), false).office;
    assert!(fake.calls.lock().unwrap().iter().any(|c| c.starts_with("pty: cd ") && c.contains("/office/susan") && c.ends_with("&& exec claude")));
    let manual = std::fs::read_to_string(office.join("susan/CLAUDE.md")).unwrap();
    assert!(manual.contains("You are Susan") && manual.contains("Linda (Stijn's notes"));
    // shot: her booth empty, her session let go
    h.manager_gone("manager1").unwrap();
    assert!(h.state().managers.iter().any(|m| m.key == "manager1" && m.vacant && m.id.is_empty()));
    wait(5, || fake.called("claude stop 's0san'") == 1);
}

#[test]
fn demo_state_reads_as_the_model() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    let s = wait_state(&ev, 5, |s| s.machines.len() == 2 && s.bots.len() >= 18);
    assert!(s.demo);
    assert_eq!(s.machines.iter().map(|m| m.ssh.as_str()).collect::<Vec<_>>(), ["@demo", "@demo"]);
    let tails = s.bots.iter().find(|b| b.id == "demo0004").unwrap();
    assert!(tails.unit.ends_with("/projects/kite") && tails.tree.ends_with("/projects/kite/.claude/worktrees/tails"));
    assert!(s.bots.iter().any(|b| b.m == "lab" && b.unit.starts_with("lab:")));
    assert!(s.desks.iter().any(|d| d.kept && d.dir.ends_with("/projects/pebble")));
    assert_eq!(s.usage.session.as_ref().unwrap().used, 64);
    assert_eq!(s.managers.len(), 1);
    assert_eq!((s.managers[0].name.as_str(), s.managers[0].style.as_str()), ("Linda", "notes"));
    // the JSON the web page had, read by the model
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(burrow_model::State::from_json(&json).unwrap(), s);
    // the demo never writes the person's files
    assert!(!Config::under(dir.path(), true).room().exists());
}

#[test]
fn demo_chess_with_you() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.bots.iter().any(|b| b.id == "demo0over0"));
    assert!(h.chess(ChessAction::Resign).is_err());
    h.chess(ChessAction::Challenge("demo0over0".into())).unwrap();
    let c = h.state().chess.unwrap();
    assert!(c.white.id == "you" || c.black.id == "you");
    // the bot opens if it is white; then it is your move, with the legal ones
    let mine = wait_state(&ev, 6, |s| s.chess.as_ref().is_some_and(|c| c.to_move == "you"));
    let c = mine.chess.unwrap();
    assert_eq!(c.legal.len(), 20);
    assert!(h.chess(ChessAction::Move("Ke5".into())).is_err());
    h.chess(ChessAction::Move(c.legal[0].clone())).unwrap();
    wait_state(&ev, 6, |s| s.chess.as_ref().is_some_and(|c| c.to_move == "you" && c.sans.len() >= 2));
    h.chess(ChessAction::Resign).unwrap();
    let c = h.state().chess.unwrap();
    assert_eq!(c.why.as_deref(), Some("resignation"));
    assert!(c.free);
    // two bots: a game of their own, until one leaves
    h.chess(ChessAction::Bots("demo0over0".into(), "demo0over1".into())).unwrap();
    assert!(h.chess(ChessAction::Bots("demo0over0".into(), "demo0over1".into())).is_err());
    h.chess(ChessAction::Leave("demo0over1".into())).unwrap();
    let c = h.state().chess.unwrap();
    assert_eq!((c.result.as_deref(), c.why.as_deref()), (Some("1-0"), Some("left")));
}

#[test]
fn demo_terminals() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.machines.len() == 2);
    // your shell at a screen: kept by the hub, its output replayed to the next tile
    let t = h.shell("", "/p/one").unwrap();
    let read = |rx: &std::sync::mpsc::Receiver<Vec<u8>>, want: &str| {
        let (mut seen, end) = (String::new(), Instant::now() + Duration::from_secs(5));
        while !seen.contains(want) && Instant::now() < end {
            if let Ok(b) = rx.recv_timeout(Duration::from_millis(100)) {
                seen += &String::from_utf8_lossy(&b);
            }
        }
        assert!(seen.contains(want), "{seen:?}");
    };
    read(&t.output, "ready in /p/one");
    t.write(b"hello\n");
    read(&t.output, "got hello");
    assert!(h.state().shells.iter().any(|s| s.dir == "/p/one"));
    drop(t);
    let t2 = h.shell("", "/p/one").unwrap();
    read(&t2.output, "got hello");
    // burnt with its desk: the tile sees it end
    h.forget("/p/one");
    let end = Instant::now() + Duration::from_secs(5);
    while t2.output.recv_timeout(Duration::from_millis(100)).is_ok() && Instant::now() < end {}
    wait(5, || h.state().shells.is_empty());
    // a session attached
    let a = h.attach("", "demo0003", 100, 30).unwrap();
    read(&a.output, "attached");
    // a file dropped on a session here: its path
    let p = h.upload("", "../notes.txt", b"hi").unwrap();
    assert!(p.ends_with("/notes.txt") && p.contains("/.cache/burrow/drop/"));
    assert_eq!(std::fs::read(&p).unwrap(), b"hi");
}

#[test]
fn demo_notes_written() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.bots.len() > 10);
    assert!(h.digest_view().on);
    assert!(h.digest() > 5);
    let end = Instant::now() + Duration::from_secs(6);
    let mut got = None;
    while got.is_none() && Instant::now() < end {
        if let Ok(Event::Digest(d)) = ev.recv_timeout(Duration::from_millis(100)) {
            got = d.sessions.values().next().cloned();
        }
    }
    let n = got.expect("a session's notes");
    assert!(n["headline"].as_str().is_some_and(|s| !s.is_empty()) && n["built"].as_array().is_some());
}

fn http(port: u16, req: &str) -> (u16, Value) {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let code = out[9..12].parse().unwrap();
    let body = out.split_once("\r\n\r\n").unwrap().1;
    (code, serde_json::from_str(body).unwrap_or(Value::Null))
}

fn post(port: u16, body: &str) -> (u16, Value) {
    http(port, &format!("POST /api/office HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}", body.len()))
}

#[test]
fn the_office_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.bots.iter().any(|b| b.id == "demo0over3"));
    let port = h.serve_office(0).unwrap();
    let (code, o) = http(port, "GET /api/office HTTP/1.1\r\nHost: x\r\n\r\n");
    assert_eq!(code, 200);
    assert_eq!(o["room"], format!("http://localhost:{port}"));
    for k in ["you", "room", "machines", "desks", "sessions", "settings", "removing", "kept"] {
        assert!(o.get(k).is_some(), "{k}");
    }
    assert_eq!(o["you"], "tester");
    let s = o["sessions"].as_array().unwrap().iter().find(|s| s["id"] == "demo0over0").unwrap();
    assert_eq!((s["state"].as_str(), s["process"].as_str()), (Some("completed (done)"), Some("off")));
    assert!(s["ended"].as_str().is_some_and(|t| t.len() == 16) && s["desk"].as_str().unwrap().ends_with("/projects/burrow"));
    assert_eq!(post(port, "{\"do\": \"dance\"}").1["error"], "do what? remove, desk, start or say");
    assert_eq!(post(port, "not json").0, 400);
    assert_eq!(http(port, "GET /elsewhere HTTP/1.1\r\n\r\n").0, 404);
    // someone has the room open: she walks it to the booth; you take it out again
    let (_, r) = post(port, "{\"do\": \"remove\", \"id\": \"merry-max\"}");
    assert!(r["ok"].as_str().unwrap().starts_with("you walk merry-max to the booth"), "{r}");
    assert_eq!(h.state().removing, ["demo0over3"]);
    post(port, "{\"kept\": \"demo0over3\"}");
    assert!(h.state().removing.is_empty());
    assert_eq!(h.office()["kept"], json!(["demo0over3"]));
    // nobody looking: removed at once
    drop(ev);
    let (_, r) = post(port, "{\"do\": \"remove\", \"id\": \"demo0over3\"}");
    assert_eq!(r["ok"], "merry-max removed");
    let (_, r) = post(port, "{\"do\": \"desk\", \"m\": \"\", \"dir\": \"~/projects/new\"}");
    assert!(r["ok"].as_str().unwrap().ends_with("/projects/new"));
    assert!(h.desks().iter().any(|d| d.kept && d.dir.ends_with("/projects/new")));
    assert_eq!(post(port, "{\"do\": \"start\", \"m\": \"\", \"dir\": \"/x\"}").1["error"], "demo bots can't be started");
}

#[test]
fn claude_settings_round_trip_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("settings.json");
    // as Python's json.dumps(indent=2, ensure_ascii=False) left it
    let before = "{\n  \"theme\": \"dark\",\n  \"model\": \"opus\",\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ],\n    \"deny\": []\n  },\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"echo été\"\n  },\n  \"env\": {},\n  \"sshConfigs\": [\n    {\n      \"id\": \"odyssey\",\n      \"name\": \"odyssey\",\n      \"sshHost\": \"odyssey\",\n      \"sshPort\": 22\n    }\n  ],\n  \"zeta\": 1.5\n}\n";
    let after = "{\n  \"theme\": \"dark\",\n  \"model\": \"opus\",\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ],\n    \"deny\": []\n  },\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"echo été\"\n  },\n  \"env\": {},\n  \"sshConfigs\": [\n    {\n      \"id\": \"odyssey\",\n      \"name\": \"odyssey\",\n      \"sshHost\": \"odyssey\",\n      \"sshPort\": 22\n    },\n    {\n      \"id\": \"lab-box\",\n      \"name\": \"Lab Box\",\n      \"sshHost\": \"lab\"\n    }\n  ],\n  \"zeta\": 1.5\n}\n";
    std::fs::write(&p, before).unwrap();
    assert_eq!(burrow_core::hub::add_ssh_config(&p, "lab", "Lab Box").unwrap(), "lab-box");
    assert_eq!(std::fs::read_to_string(&p).unwrap(), after);
    assert!(burrow_core::hub::add_ssh_config(&p, "lab", "").is_err());
    assert_eq!(burrow_core::hub::add_ssh_config(&p, "lab2", "Lab Box").unwrap(), "lab-box-2");
    burrow_core::hub::remove_ssh_config(&p, "lab-box-2").unwrap();
    burrow_core::hub::remove_ssh_config(&p, "lab-box").unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), before);
    assert!(burrow_core::hub::remove_ssh_config(&p, "nope").is_err());
}

#[test]
fn machines_from_claude_settings() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), false, Arc::new(Fake::default()));
    std::fs::write(
        &h.cfg.claude_settings,
        r#"{"sshConfigs": [{"id": "ody", "sshHost": "odyssey", "sshPort": 2222, "sshIdentityFile": "~/.ssh/k"}, {"id": "", "sshHost": "x"}, {"id": 7, "name": "Seven", "sshHost": "seven"}, "junk"]}"#,
    )
    .unwrap();
    let ms = h.machines();
    assert_eq!(ms.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["", "ody", "7"]);
    assert_eq!((ms[1].name.as_str(), ms[1].port.as_deref(), ms[1].key.as_deref()), ("odyssey", Some("2222"), Some("~/.ssh/k")));
    assert_eq!(ms[2].name, "Seven");
    std::fs::create_dir_all(dir.path().join("home/.ssh")).unwrap();
    std::fs::write(&h.cfg.ssh_config, "Host odyssey lab\nHost *\n").unwrap();
    assert_eq!(h.hosts(), ["lab"]);
}

#[test]
fn a_pair_table() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    let h = hub(dir.path(), false, fake.clone());
    let key = h.add_furniture("pair", [10.0, 10.0]).unwrap();
    assert_eq!(h.pair("plant0", None, Some("x")).unwrap_err(), "not a pair piece");
    assert_eq!(h.pair(&key, None, None).unwrap_err(), "no room linked yet");
    assert!(h.pair(&key, None, Some("https://pair.example/a/b")).unwrap_err().starts_with("that is not a pair room"));
    assert_eq!(h.pair(&key, None, Some("https://pair.example/h/nope")).unwrap_err(), "pair does not know that page");
    // Cloudflare's check, not pair's words: as it said it
    fake.web.lock().unwrap().push(("cf.example".into(), 403, "error code: 1010\n<html>".into()));
    assert_eq!(h.pair(&key, None, Some("https://cf.example/h/tok")).unwrap_err(), "pair answered 403 (error code: 1010)");
    // an invite: joined with your name and face, then your seat refused (the name is taken there), the room linked
    let mut web = fake.web.lock().unwrap();
    web.push(("/r/inv".into(), 200, r#"{"url": "/h/mine"}"#.into()));
    web.push(("/h/mine/me".into(), 409, r#"{"error": "that name is taken"}"#.into()));
    web.push(("/h/mine/state".into(), 200, r#"{"room": "Lounge", "bring": "https://pair.example/n/b1", "people": [{"name": "tester"}]}"#.into()));
    drop(web);
    let p = h.pair(&key, None, Some("https://pair.example/r/inv/")).unwrap();
    assert_eq!((p.url.as_str(), p.room.as_str(), p.bring.as_str()), ("https://pair.example/h/mine", "Lounge", "https://pair.example/n/b1"));
    assert_eq!(p.warn, "pair: that name is taken");
    assert_eq!(p.people, ["tester"]);
    assert!(fake.called(r#"http: https://pair.example/r/inv {"name":"tester","avatar":"tester"}"#) == 1);
    assert_eq!(h.state().pair[&key].room, "Lounge");
    // looked up again: no seat taken
    assert_eq!(h.pair(&key, None, None).unwrap().bring, "https://pair.example/n/b1");
    assert_eq!(fake.called("/h/mine/me"), 1);
}
