//! The matrix's backend as the operator's view drives it (/api/stream, /api/tell): on a made-up outside (no claude,
//! no ssh) or in the demo, in a temporary home.

use burrow_core::runner::{Kept, Output};
use burrow_core::{Config, Event, Hub, Mach, Runner};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// the outside: the poll lists two sessions; the transcripts' tails are made up after the formats server.py reads
#[derive(Default)]
struct Fake {
    calls: Mutex<Vec<String>>,
}

const AGENTS: &str = r#"[
  {"id": "b1", "kind": "background", "name": "busy-bea", "state": "working", "status": "busy", "pid": 501, "cwd": "/p/one", "sessionId": "s-1", "startedAt": 1000},
  {"id": "b2", "kind": "background", "name": "odd-otto", "state": "done", "status": "idle", "pid": 502, "cwd": "/p/one", "sessionId": "not a session; rm -rf", "startedAt": 2000}
]"#;

impl Runner for Fake {
    fn exec(&self, _m: &Mach, script: &str, _input: Option<&[u8]>, _t: Duration, pid: &mut dyn FnMut(u32)) -> Result<Output, String> {
        self.calls.lock().unwrap().push(script.to_string());
        pid(4242);
        let out = if script.starts_with("claude agents --json --all; echo") {
            format!("{AGENTS}\n@@burrow@@\n{{}}\n@@burrow@@\n\n@@burrow@@\n/h/.claude/jobs/b1/state.json:\"updatedAt\": \"2026-10-24T12:00:00Z\"\n")
        } else if script.starts_with("for s in ") {
            let line = r#"{"message":{"content":[{"type":"text","text":"the rain falls"},{"type":"tool_use","name":"Read","input":{"file_path":"/p/one/a.rs"}}]}}"#;
            format!("@@burrow@@ s-1\n(cut)\n{line}\n\n")
        } else {
            String::new()
        };
        Ok(Output { code: 0, stdout: out.into_bytes(), stderr: vec![] })
    }
    fn argv(&self, _m: &Mach, _script: &str, _tty: bool) -> Vec<String> {
        vec!["/bin/sh".into(), "-c".into(), "true".into()]
    }
    fn keep(&self, _argv: &[String]) -> std::io::Result<Box<dyn Kept>> {
        Err(std::io::Error::other("no"))
    }
    fn scp(&self, _m: &Mach, _l: &Path, _t: &str) -> Result<(), String> {
        Ok(())
    }
    fn http(&self, _url: &str, _body: Option<&Value>) -> Result<(u16, String), String> {
        Ok((404, String::new()))
    }
}

fn hub(dir: &Path, demo: bool, fake: Arc<Fake>) -> Arc<Hub> {
    let cfg = Config::under(dir, demo);
    std::fs::create_dir_all(cfg.claude_settings.parent().unwrap()).unwrap();
    Hub::new(cfg, fake)
}

fn wait_state(ev: &burrow_core::Events, secs: u64, ok: impl Fn(&burrow_model::State) -> bool) -> burrow_model::State {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if let Ok(Event::State(s)) = ev.recv_timeout(Duration::from_millis(100))
            && ok(&s)
        {
            return *s;
        }
    }
    panic!("no such state within {secs} s");
}

#[test]
fn the_rain_is_each_transcripts_tail() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(Fake::default());
    let h = hub(dir.path(), false, fake.clone());
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.bots.len() == 2);
    let ids = vec!["b1".to_string(), "b2".to_string(), "nobody".to_string()];
    let t = h.stream(&ids);
    // one command for the machine, for the one whose session id is safe to put in it
    let asked: Vec<String> = fake.calls.lock().unwrap().iter().filter(|c| c.starts_with("for s in ")).cloned().collect();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].starts_with("for s in 's-1'; do") && !asked[0].contains("rm -rf"), "{}", asked[0]);
    assert_eq!(t.get("b1").map(String::as_str), Some("the rain falls Read /p/one/a.rs"));
    assert!(!t.contains_key("b2") && !t.contains_key("nobody"));
    // kept while its record is the same: asked again, nothing runs
    let t = h.stream(&ids[..1]);
    assert_eq!(t["b1"], "the rain falls Read /p/one/a.rs");
    assert_eq!(fake.calls.lock().unwrap().iter().filter(|c| c.starts_with("for s in ")).count(), 1);
}

#[test]
fn the_demo_rains_and_is_told() {
    let dir = tempfile::tempdir().unwrap();
    let h = hub(dir.path(), true, Arc::new(Fake::default()));
    let ev = h.subscribe();
    h.start();
    wait_state(&ev, 5, |s| s.bots.iter().any(|b| b.id == "demo0003") && s.bots.iter().any(|b| b.id == "demo1000"));
    let t = h.stream(&["demo0003".into(), "demo1000".into()]);
    assert!(t["demo0003"].split(' ').count() >= 160 && t["demo0003"].contains("kite"), "{}", t["demo0003"]);
    assert!(t.contains_key("demo1000"));
    // a machine asked at most every ten seconds: another of its sessions waits its turn
    let t = h.stream(&["demo0002".into()]);
    assert!(t.is_empty());
    // tell: the one told gets to work (the demo has no session to type into)
    assert_eq!(h.tell("demo0003", "  ").unwrap_err(), "tell whom what");
    assert_eq!(h.tell("nobody", "hi").unwrap_err(), "tell whom what");
    h.tell("demo0004", "carry on").unwrap();
    wait_state(&ev, 5, |s| s.bots.iter().any(|b| b.id == "demo0004" && b.busy && !b.waiting && b.state == "working"));
}
