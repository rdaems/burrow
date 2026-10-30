//! The office manager: whoever sits at the reception booth (the `manager` piece; one at a time). Her style is
//! picked when the booth comes out of the closet. The basis is a Claude Code background session of her own
//! (in the office directory, whose CLAUDE.md burrow writes: the office, what she can do through burrow's
//! /api/office, her style), a terminal you talk to her in; Joan is that, doing rounds on her own (Claude
//! Code's /loop). Linda is Stijn's notes: no session, `claude -p` writes notes on every session (the digest).
//! Kept in room.json `managers: {booth: {style, id}}`.
//!
//! Her curl is the one thing that still comes over HTTP: `serve_office` answers GET and POST /api/office.

use crate::hub::{Hub, Inner, Res};
use crate::room;
use crate::runner::run;
use crate::util::{self, kind_of, split_unit, sq, text, truthy, unit};
use burrow_model::{self as model, Bot};
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub struct Style {
    pub key: &'static str,
    pub name: &'static str,
    pub seed: &'static str,
    pub color: &'static str,
    pub session: bool,
    /// her first prompt
    pub first: &'static str,
    pub line: &'static str,
    pub how: &'static str,
    pub brief: &'static str,
}

pub const MANAGERS: &[Style] = &[
    Style {
        key: "ask",
        name: "Susan",
        seed: "susan-10",
        color: "#4fb3a9",
        session: true,
        first: "",
        line: "Ask her anything about the office, in her terminal. She looks it up, starts bots, puts up desks, clears out sessions, messages a bot.",
        how: "a Claude Code session of her own; she acts when asked",
        brief: "You act when asked. %(you)s comes to your booth (your terminal) with a question about the office or an errand:
what goes on at a desk, who is stuck and on what, start a bot on something, put up a desk, clear out old
sessions, pass a message on. Look things up yourself before you ask back.",
    },
    Style {
        key: "rounds",
        name: "Joan",
        seed: "joan-10",
        color: "#9a74d6",
        session: true,
        first: "/loop 30m Do your round of the office (CLAUDE.md, your style).",
        line: "As Susan, and every half hour she goes round by herself: who is stuck, who is done. She nudges bots and asks you what is yours.",
        how: "a Claude Code session with a /loop: a turn every half hour",
        brief: "You do the rounds: a /loop in your session starts one every 30m, and %(you)s may also talk to you
in your terminal. A round:
1. `curl -s localhost:%(port)s/api/office`; rounds.md here is your log of the earlier rounds (newest on top).
2. For the sessions that changed since your last round, read what you need: the record's state, summary and
   last output (~/.claude/jobs/<id>/state.json), the transcript's tail if that is not enough.
3. A session waiting for an answer: if it is %(you)s's to decide, ask %(you)s (AskUserQuestion: the session, the
   question in a line, the options). Never answer for %(you)s.
4. A session stuck or going in circles: message it (SendMessage) with what you see, or tell %(you)s.
5. Write the round into rounds.md (three lines at most, newest on top). End with one line for %(you)s; a quiet
   round is one line. You remove a session only when %(you)s asks you to.",
    },
    Style {
        key: "notes",
        name: "Linda",
        seed: "manager-16",
        color: "#f06b72",
        session: false,
        first: "",
        line: "Keeps the notes on every session: what it does, the decisions it leaves you, what it built. Explains a decision, sends your answer.",
        how: "Stijn's notes: Claude writes them up when a session finishes a turn (a few cents each)",
        brief: "",
    },
];

pub fn style(key: &str) -> Option<&'static Style> {
    MANAGERS.iter().find(|s| s.key == key)
}

const MANUAL: &str = "# The office

You are %(name)s, an office manager in %(you)s's office. The office is burrow: a room drawn in pixels in
%(you)s's browser (%(room)s) where %(you)s's Claude Code background sessions are bots. You are one of those
sessions yourself, at the reception booth; %(you)s talks to you in your terminal there.

How the room maps onto Claude Code:
- A bot is a background session (`claude --bg`), one row of `claude agents --json --all`, on this computer
  or on another machine reached over ssh (a door in the room each; Claude Code's `sshConfigs`).
- A desk is a working directory on a machine (a git worktree sits at its repository's). Every session has
  one: a directory with sessions gets a desk of its own, and loses it with the last of them, unless
  %(you)s keeps it. A full room is a full room: clearing out sessions is what frees it.
- Typing at its desk: a turn runs. Hand up with a ?: a prompt is open in it, it waits for an answer.
  Otherwise idle. A completed session (its process stopped) roams the room as well; attaching restarts it.
- The booth deletes a session (`claude stop`, `claude rm`; the transcript stays, `claude --resume` finds it).

## What you can do

Through burrow, so that it shows in the room (a session you remove, you walk to the booth yourself):

    curl -s localhost:%(port)s/api/office                     # machines, desks, sessions (JSON)
    curl -s localhost:%(port)s/api/office -d '{\"do\": \"remove\", \"id\": \"<id>\"}'
    curl -s localhost:%(port)s/api/office -d '{\"do\": \"desk\", \"m\": \"<machine id>\", \"dir\": \"<path>\"}'
    curl -s localhost:%(port)s/api/office -d '{\"do\": \"start\", \"m\": \"<machine id>\", \"dir\": \"<path>\", \"prompt\": \"<the task>\"}'
    curl -s localhost:%(port)s/api/office -d '{\"do\": \"say\", \"id\": \"<id>\", \"text\": \"<message>\"}'

- remove: deleted for good, after a few seconds in which %(you)s can still take it out of the booth (then it
  is in `kept`: leave it alone).
- desk: puts up a desk for a directory, kept. start: a new bot at work there (its id back; its desk comes
  with it).
- say: typed into the session's prompt, as if %(you)s wrote it. To ask a bot something, rather use your
  own SendMessage (ListAgents for the addresses; this computer's sessions): it shows as a beam in the room.
- Reading is yours: a session's record is ~/.claude/jobs/<id>/state.json (state, summary, last output), its
  transcript ~/.claude/projects/<its cwd, every / a ->/<sessionId>.jsonl. Another machine: `ssh <its ssh>`.

Rules:
- The projects' work is not yours: start a bot for it, or ask the one at that desk.
- Remove only what %(you)s asked for, or what your style allows. Say what you removed.
- What is %(you)s's to decide, ask (AskUserQuestion): your hand goes up in the room.
- Short answers. %(you)s looks at the room; the detail is in the sessions.

Your colleagues at the other receptions: %(colleagues)s.

## Your style: %(name)s

%(brief)s
";

/// Python's `%` with a dict: %(key)s replaced
fn fill(t: &str, v: &[(&str, &str)]) -> String {
    let mut s = t.to_string();
    for (k, x) in v {
        s = s.replace(&format!("%({k})s"), x);
    }
    s
}

/// the manual of an office manager's session, as the office is now
pub fn manual(st: &Style, you: &str, port: u16, colleagues: &str) -> String {
    let (port, room) = (port.to_string(), format!("http://localhost:{port}"));
    let v = [("name", st.name), ("you", you), ("port", port.as_str()), ("room", room.as_str())];
    let brief = fill(st.brief, &v);
    let mut all = v.to_vec();
    all.push(("brief", &brief));
    all.push(("colleagues", if colleagues.is_empty() { "none: you are the only one" } else { colleagues }));
    // the brief first: it has the others' keys in it
    let t = MANUAL.replace("%(brief)s", &brief);
    fill(&t, &all)
}

fn realpath(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn inside(p: &Path, dir: &Path) -> bool {
    let (p, d) = (realpath(p), realpath(dir));
    p == d || p.starts_with(&d)
}

/// an office manager's style, as the closet lists them
#[derive(Clone, Debug, serde::Serialize)]
pub struct StyleView {
    pub key: String,
    pub here: bool,
    pub name: String,
    pub seed: String,
    pub color: String,
    pub line: String,
    pub how: String,
    pub session: bool,
}

impl Hub {
    pub(crate) fn managers_view(&self, g: &Inner) -> Vec<model::Manager> {
        g.booths()
            .into_iter()
            .map(|key| {
                let mg = g.manager(&key);
                let st = style(mg["style"].as_str().unwrap()).unwrap();
                let id = text(mg.get("id"));
                let r = g.mrows.get(&id);
                model::Manager {
                    key,
                    style: st.key.into(),
                    name: st.name.into(),
                    seed: st.seed.into(),
                    color: st.color.into(),
                    session: st.session,
                    error: text(mg.get("error")),
                    busy: r.is_some_and(|r| r.busy),
                    waiting: r.is_some_and(|r| r.waiting),
                    alive: r.is_some_and(|r| r.alive),
                    rc: r.map(|r| r.rc.clone()).unwrap_or_default(),
                    vacant: mg.get("vacant").is_some_and(truthy),
                    id,
                }
            })
            .collect()
    }

    /// the styles there are, and which are at a booth
    pub fn manager_styles(&self) -> Vec<StyleView> {
        let g = self.lock();
        let here: Vec<String> = g.booths().iter().map(|k| text(g.manager(k).get("style"))).collect();
        MANAGERS
            .iter()
            .map(|s| StyleView { key: s.key.into(), here: here.iter().any(|h| h == s.key), name: s.name.into(), seed: s.seed.into(), color: s.color.into(), line: s.line.into(), how: s.how.into(), session: s.session })
            .collect()
    }

    fn manager_entry<'a>(g: &'a mut Inner, key: &str) -> &'a mut Map<String, Value> {
        let style = g.manager(key)["style"].clone();
        let ms = room::obj(&mut g.room, "managers");
        if !ms.get(key).is_some_and(Value::is_object) {
            ms.insert(key.into(), json!({"style": style}));
        }
        ms.get_mut(key).unwrap().as_object_mut().unwrap()
    }

    /// shot and swept up, as any bot: her session stopped and deleted (its transcript stays), her reception left
    /// empty (`vacant`) until a click on it hires her again
    pub fn manager_gone(&self, key: &str) -> Res<()> {
        let old = {
            let mut g = self.lock();
            if !g.booths().iter().any(|k| k == key) {
                return Err("no one at that booth".into());
            }
            let mg = Hub::manager_entry(&mut g, key);
            let old = mg.shift_remove("id");
            mg.insert("vacant".into(), json!(true));
            g.save(&self.cfg);
            old
        };
        if let Some(old) = old.filter(truthy) {
            let (h, id) = (self.arc(), text(Some(&old)));
            std::thread::spawn(move || h.let_go(&id));
        }
        self.push_state();
        self.push_digest();
        Ok(())
    }

    /// her own directory: her CLAUDE.md, her sessions (Claude Code keys them by directory); where her session
    /// runs, if it runs elsewhere in the office (one hired before each had her own)
    pub(crate) fn desk_of(&self, st: &Style, bid: Option<&str>) -> PathBuf {
        let cwd = bid.and_then(|b| self.lock().mrows.get(b).map(|r| r.cwd.clone())).filter(|c| !c.is_empty());
        if let Some(c) = cwd {
            let p = PathBuf::from(&c);
            if inside(&p, &self.cfg.office) {
                return p;
            }
        }
        self.cfg.office.join(st.name.to_lowercase())
    }

    /// every session manager's CLAUDE.md, as the office is now (this port, her colleagues)
    pub fn write_manuals(&self) {
        let (mgs, you) = {
            let g = self.lock();
            (g.booths().iter().map(|k| g.manager(k)).collect::<Vec<_>>(), room::you_of(&g.room, &self.cfg.user).0)
        };
        let here: Vec<String> = mgs.iter().map(|m| text(m.get("style"))).collect();
        for mg in &mgs {
            let st = style(mg["style"].as_str().unwrap()).unwrap();
            if !st.session {
                continue;
            }
            let others: Vec<String> = here.iter().filter(|x| *x != st.key).map(|x| style(x).unwrap()).map(|o| format!("{} ({})", o.name, o.how)).collect();
            let d = self.desk_of(st, mg.get("id").and_then(Value::as_str));
            let _ = std::fs::create_dir_all(&d);
            let _ = std::fs::write(d.join("CLAUDE.md"), manual(st, &you, self.port(), &others.join(", ")));
        }
    }

    /// a reception out of the closet with the style picked: a booth of its own, her session started (if she
    /// has one). One of each: a second Susan would be the same session's name twice
    pub fn hire(&self, style_key: &str, at: Option<[f64; 2]>) -> Res<String> {
        let st = style(style_key).ok_or("no such office manager")?;
        let key = {
            let mut g = self.lock();
            if g.booths().iter().any(|k| g.manager(k)["style"] == style_key) {
                return Err(format!("{} is at a booth already", st.name));
            }
            let mut fs = room::furniture(&g.room);
            let key = (0..1000).map(|i| format!("manager{i}")).find(|k| !fs.contains(k)).ok_or("no room")?;
            fs.push(key.clone());
            room::set_furniture(&mut g.room, fs);
            if let Some([x, y]) = at {
                room::layout(&mut g.room).insert(key.clone(), json!([x as i64, y as i64]));
            }
            room::obj(&mut g.room, "managers").insert(key.clone(), json!({"style": style_key}));
            g.save(&self.cfg);
            key
        };
        if self.cfg.real_office {
            let h = self.arc();
            std::thread::spawn(move || h.write_manuals()); // her colleagues know of her
        }
        if st.session {
            let (h, k) = (self.arc(), key.clone());
            std::thread::spawn(move || h.start_manager(&k));
        }
        self.push_state();
        self.push_digest();
        Ok(key)
    }

    /// her booth burnt: she walks out, her session is stopped and deleted
    pub(crate) fn fire(&self, key: &str) {
        let old = {
            let mut g = self.lock();
            let old = g.room.get_mut("managers").and_then(Value::as_object_mut).and_then(|m| m.shift_remove(key));
            g.save(&self.cfg);
            old
        };
        if let Some(id) = old.as_ref().and_then(|o| o.get("id")).filter(|i| truthy(i)).map(|i| text(Some(i))) {
            let h = self.arc();
            std::thread::spawn(move || h.let_go(&id));
        }
        if self.cfg.real_office {
            let h = self.arc();
            std::thread::spawn(move || h.write_manuals());
        }
        self.push_digest();
    }

    pub(crate) fn let_go(&self, bid: &str) {
        if self.cfg.real_office && !bid.starts_with("demo") {
            let _ = run(self.runner(), &self.local(), &format!("claude stop {} >/dev/null 2>&1; claude rm {}", sq(bid), sq(bid)), Duration::from_secs(60));
        }
    }

    /// her own session, in her directory (its CLAUDE.md written first): the id goes into room.json. Her
    /// directory new and not under a trusted one: trusted, once, and started again
    pub(crate) fn start_manager(&self, key: &str) {
        let st = style(self.lock().manager(key)["style"].as_str().unwrap()).unwrap();
        let r: Res<String> = (|| {
            if !self.cfg.real_office {
                return Ok(format!("demo-{}", st.key));
            }
            self.write_manuals();
            let who = format!("You are {}, an office manager. CLAUDE.md here is the office, what you can do in it and your style.", st.name);
            let d = self.desk_of(st, None).to_string_lossy().into_owned();
            let extra = format!("--append-system-prompt {} ", sq(&who));
            match self.spawn_in(&self.local(), &d, st.name, st.first, &extra) {
                Err(e) if e.contains("not trusted") => {
                    crate::term::trust(self.runner(), &self.local(), &d);
                    self.spawn_in(&self.local(), &d, st.name, st.first, &extra)
                }
                r => r,
            }
        })();
        match r {
            Ok(bid) => {
                let taken = {
                    let mut g = self.lock();
                    let ms = room::obj(&mut g.room, "managers");
                    match ms.get_mut(key).and_then(Value::as_object_mut) {
                        Some(mg) if mg.get("style").and_then(Value::as_str) == Some(st.key) && !mg.get("id").is_some_and(truthy) => {
                            mg.insert("id".into(), json!(bid));
                            mg.insert("error".into(), json!(""));
                            mg.insert("since".into(), json!(util::now() as i64));
                            g.save(&self.cfg);
                            true
                        }
                        _ => false,
                    }
                };
                if !taken {
                    // let go of meanwhile (her booth burnt)
                    self.let_go(&bid);
                }
            }
            Err(e) => {
                let mut g = self.lock();
                if let Some(mg) = g.room.get_mut("managers").and_then(|m| m.get_mut(key)).and_then(Value::as_object_mut) {
                    mg.insert("error".into(), json!(e.trim().chars().take(300).collect::<String>()));
                }
            }
        }
        self.push_state();
    }

    /// her session for a terminal: started again if it is gone (deleted elsewhere) or never came. (id, error)
    pub fn manager_open(&self, key: &str) -> Res<(String, String)> {
        let mut mg = {
            let g = self.lock();
            if !g.booths().iter().any(|k| k == key) {
                return Err("no one at that booth".into());
            }
            g.manager(key)
        };
        if mg.get("vacant").is_some_and(truthy) {
            // her reception empty (she was shot): hired again
            {
                let mut g = self.lock();
                Hub::manager_entry(&mut g, key).shift_remove("vacant");
                g.save(&self.cfg);
            }
            self.push_state();
            self.push_digest();
            mg = self.lock().manager(key);
            if !style(mg["style"].as_str().unwrap()).unwrap().session {
                return Ok((String::new(), String::new()));
            }
        }
        if !style(mg["style"].as_str().unwrap()).unwrap().session {
            return Err("no one to talk to at the booth".into());
        }
        let id = text(mg.get("id"));
        let since = mg.get("since").and_then(Value::as_f64).unwrap_or(0.0);
        // (the polls have seen her by then)
        let gone = self.cfg.real_office && !id.is_empty() && util::now() - since > 30.0 && !self.lock().mrows.contains_key(&id);
        if id.is_empty() || gone {
            {
                let mut g = self.lock();
                Hub::manager_entry(&mut g, key).shift_remove("id");
            }
            self.start_manager(key);
        }
        let mg = self.lock().manager(key);
        Ok((text(mg.get("id")), text(mg.get("error"))))
    }

    /// their rows out of this computer's sessions: an office manager is not a bot at a desk (nor is any other
    /// session in their directories)
    pub(crate) fn take_manager(&self, bots: Vec<Bot>) -> Vec<Bot> {
        let mut g = self.lock();
        let ids: Vec<String> = g.room.get("managers").and_then(Value::as_object).map(|ms| ms.values().map(|m| text(m.get("id"))).filter(|i| !i.is_empty()).collect()).unwrap_or_default();
        g.mrows = bots.iter().filter(|b| ids.contains(&b.id)).map(|b| (b.id.clone(), b.clone())).collect();
        let office = self.cfg.office.clone();
        drop(g);
        bots.into_iter().filter(|b| !ids.contains(&b.id) && !inside(Path::new(if b.cwd.is_empty() { "/" } else { &b.cwd }), &office)).collect()
    }

    /// a session id as the room knows it: an office manager is mgr:<her booth>
    pub(crate) fn folk_of(&self, bid: &str) -> String {
        let g = self.lock();
        g.room
            .get("managers")
            .and_then(Value::as_object)
            .and_then(|ms| ms.iter().find(|(_, m)| m.get("id").and_then(Value::as_str) == Some(bid)).map(|(k, _)| format!("mgr:{k}")))
            .unwrap_or_else(|| bid.to_string())
    }

    /// the demo with a real office manager (BURROW_OFFICE): her row from this computer's `claude agents`
    pub(crate) fn demo_manager(&self) {
        loop {
            if let Ok(out) = run(self.runner(), &self.local(), "claude agents --json --all", Duration::from_secs(60))
                && let Ok(bots) = crate::parse::parse_agents(&out, &Default::default()) {
                    self.take_manager(bots);
                    self.push_state();
                }
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    // ---- what she sees of the office, and what she does in it (/api/office, curl from her session)

    pub fn office(&self) -> Value {
        let g = self.lock();
        let iso = |t: i64| if t != 0 { json!(util::local_minute(t)) } else { Value::Null };
        let state = |b: &Bot| {
            if b.waiting {
                "waiting for an answer".to_string()
            } else if b.busy {
                "working".into()
            } else if !b.over.is_empty() {
                format!("completed ({})", b.over)
            } else {
                "idle".into()
            }
        };
        let none = |s: &str| if s.is_empty() { Value::Null } else { json!(s) };
        let s = room::options(&g.room);
        json!({
            "you": room::you_of(&g.room, &self.cfg.user).0, "room": format!("http://localhost:{}", self.port()),
            "machines": g.machines.values().map(|m| json!({"id": m.id, "name": m.name, "ssh": none(&m.ssh), "error": g.feeds.get(&m.id).and_then(|f| f.error.clone())})).collect::<Vec<_>>(),
            "desks": g.desks().iter().map(|d| json!({"m": d.m, "dir": d.dir, "name": none(&d.name), "kept": d.kept})).collect::<Vec<_>>(),
            "sessions": g.bots().iter().map(|b| json!({"id": b.id, "name": b.name, "m": b.m, "cwd": b.cwd, "desk": split_unit(&b.unit).1,
                "state": state(b), "process": if b.alive { "running" } else { "off" }, "started": iso(b.started), "ended": iso(b.ended_at),
                "summary": b.detail, "sessionId": b.session})).collect::<Vec<_>>(),
            "settings": {"rc": s.rc, "yolo": s.yolo, "hardcore": s.hardcore, "sound": s.sound},
            "removing": g.removing.keys().collect::<Vec<_>>(), "kept": g.kept,
        })
    }

    pub fn office_do(&self, b: &Value) -> Res<Value> {
        let do_ = b.get("do").and_then(Value::as_str).unwrap_or("");
        let bots = self.bots();
        if let Some(k) = b.get("kept").filter(|k| truthy(k)) {
            // from the room: you took it out of the booth (or put it back together)
            let k = text(Some(k));
            let mut g = self.lock();
            if g.removing.shift_remove(&k).is_some() {
                g.kept.push(k);
                let n = g.kept.len();
                if n > 50 {
                    g.kept.drain(..n - 50);
                }
                drop(g);
                self.push_state();
            }
            return Ok(json!({}));
        }
        let id = text(b.get("id"));
        let bot = if id.is_empty() { None } else { bots.iter().find(|x| x.id == id || x.name == id).cloned() };
        match do_ {
            "remove" => {
                let bot = bot.ok_or_else(|| format!("no such session: {}", if id.is_empty() { "None" } else { &id }))?;
                let (booth, booths, hardcore, you) = {
                    let g = self.lock();
                    (room::furniture(&g.room).iter().any(|k| kind_of(k) == "booth"), !g.booths().is_empty(), room::options(&g.room).hardcore, room::you_of(&g.room, &self.cfg.user).0)
                };
                if self.viewers() > 0 && booth && booths {
                    // someone has the room open: an office manager walks it to the booth there
                    self.lock().removing.insert(bot.id.clone(), util::now());
                    self.push_state();
                    if hardcore {
                        return Ok(json!({"ok": format!("hardcore: you take a shotgun and shoot {}; once the idle bots have swept up the pieces (half a minute or so) it is deleted, unless {you} puts it back together", bot.name)}));
                    }
                    return Ok(json!({"ok": format!("you walk {} to the booth: it is deleted in about half a minute, unless {you} takes it out", bot.name)}));
                }
                let m = self.machine(&bot.m)?;
                if m.demo() {
                    self.demo_stop(&bot.m, &bot.id, true);
                } else {
                    run(self.runner(), &m, &format!("claude stop {} >/dev/null 2>&1; claude rm {}", sq(&bot.id), sq(&bot.id)), Duration::from_secs(60))?;
                }
                Ok(json!({"ok": format!("{} removed", bot.name)}))
            }
            "desk" | "start" => {
                let m = self.machine(&text(b.get("m")))?;
                let d = text(b.get("dir")).trim().to_string();
                if d.is_empty() {
                    return Err("which directory".into());
                }
                let d = if m.demo() { d.replacen('~', &self.cfg.home_str(), 1) } else { run(self.runner(), &m, &format!("cd {} && pwd -P", sq(&d)), Duration::from_secs(20))?.trim().to_string() };
                if do_ == "desk" {
                    let u = unit(&m.id, &d);
                    if !room::desks(&self.lock().room).contains(&u) {
                        self.keep(&u, true);
                    }
                    return Ok(json!({"ok": format!("a desk for {d}")}));
                }
                if m.demo() {
                    return Err("demo bots can't be started".into());
                }
                Ok(json!({"id": self.spawn_in(&m, &d, text(b.get("name")).trim(), &text(b.get("prompt")), "")?}))
            }
            "say" => {
                let t = text(b.get("text")).trim().to_string();
                let Some(bot) = bot.filter(|_| !t.is_empty()) else { return Err("say what to which session".into()) };
                let m = self.machine(&bot.m)?;
                if !m.demo() {
                    let (h, id, t) = (self.arc(), bot.id.clone(), t.chars().take(4000).collect::<String>());
                    std::thread::spawn(move || {
                        let _ = crate::term::deliver(h.runner(), &m, &id, &t, Duration::from_secs(60));
                    });
                }
                Ok(json!({"ok": format!("typed into {}", bot.name)}))
            }
            _ => Err("do what? remove, desk, start or say".into()),
        }
    }

    /// the port her CLAUDE.md tells her to curl: the one served on, else BURROW_PORT's
    pub fn port(&self) -> u16 {
        self.port.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// the office managers' curl: GET and POST /api/office on 127.0.0.1:`port` (0: any free one), the port
    /// their manuals give from then on. The port it listens on. While the web app runs too, give the app a
    /// port of its own (BURROW_PORT): 7777 is the web server's
    pub fn serve_office(&self, port: u16) -> std::io::Result<u16> {
        let l = TcpListener::bind(("127.0.0.1", port))?;
        let at = l.local_addr()?.port();
        self.port.store(at, std::sync::atomic::Ordering::Relaxed);
        let h = self.arc();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let h = h.clone();
                std::thread::spawn(move || {
                    let _ = serve(&h, s);
                });
            }
        });
        Ok(at)
    }
}

fn reply(s: &mut TcpStream, code: u16, body: &[u8], ctype: &str) -> std::io::Result<()> {
    let why = match code {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        _ => "Not Found",
    };
    write!(s, "HTTP/1.1 {code} {why}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len())?;
    s.write_all(body)
}

/// Who may talk to the office port (server.py's `trusted`): programs with no browser behind them, which send no
/// Origin (the office managers' curl), and by its own name only (localhost or 127.0.0.1 at its port). Without it any
/// page open in a browser could post to it (start a bot, type into a session), and a name of the attacker's that
/// resolves to 127.0.0.1 (DNS rebinding) could read it: it is turned away by its Host
pub fn trusted(host: Option<&str>, origin: Option<&str>, port: u16) -> bool {
    let ours = |h: &str| h == format!("localhost:{port}") || h == format!("127.0.0.1:{port}");
    host.is_some_and(ours) && origin.is_none_or(|o| o.strip_prefix("http://").is_some_and(ours))
}

fn serve(h: &Arc<Hub>, mut s: TcpStream) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let path = path.split('?').next().unwrap_or("");
    let mut n = 0usize;
    let (mut host, mut origin) = (None, None);
    loop {
        let mut hl = String::new();
        if r.read_line(&mut hl)? == 0 || hl.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = hl.split_once(':') {
            let (k, v) = (k.trim(), v.trim().to_string());
            if k.eq_ignore_ascii_case("content-length") {
                n = v.parse().unwrap_or(0);
            } else if k.eq_ignore_ascii_case("host") {
                host = Some(v);
            } else if k.eq_ignore_ascii_case("origin") {
                origin = Some(v);
            }
        }
    }
    if !trusted(host.as_deref(), origin.as_deref(), h.port()) {
        return reply(&mut s, 403, json!({"error": "not from here"}).to_string().as_bytes(), "application/json");
    }
    let json_reply = |s: &mut TcpStream, v: &Value| reply(s, 200, v.to_string().as_bytes(), "application/json");
    let err = |e: String| json!({"error": e.trim().chars().take(500).collect::<String>()});
    match (method, path) {
        ("GET", "/api/office") => json_reply(&mut s, &h.office()),
        ("POST", "/api/office") => {
            let mut body = vec![0u8; n];
            r.read_exact(&mut body)?;
            let b: Value = match serde_json::from_slice(if body.is_empty() { b"{}" } else { &body }) {
                Ok(b) => b,
                Err(_) => return reply(&mut s, 400, json!({"error": "not json"}).to_string().as_bytes(), "application/json"),
            };
            let v = h.office_do(&b).unwrap_or_else(err);
            json_reply(&mut s, &v)
        }
        _ => reply(&mut s, 404, b"not here", "text/plain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manual() {
        let joan = style("rounds").unwrap();
        let m = manual(joan, "Rembert", 7777, "Linda (Stijn's notes)");
        assert!(m.starts_with("# The office\n\nYou are Joan, an office manager in Rembert's office."));
        assert!(m.contains("1. `curl -s localhost:7777/api/office`"));
        assert!(m.contains("(http://localhost:7777)") && m.contains("colleagues at the other receptions: Linda (Stijn's notes)."));
        assert!(!m.contains("%("), "{m}");
        assert!(manual(style("ask").unwrap(), "x", 1, "").contains("none: you are the only one"));
    }

    #[test]
    fn only_its_own_name_and_no_page() {
        assert!(trusted(Some("localhost:7790"), None, 7790), "her curl");
        assert!(trusted(Some("127.0.0.1:7790"), None, 7790));
        assert!(trusted(Some("localhost:7790"), Some("http://127.0.0.1:7790"), 7790), "a page of its own");
        assert!(!trusted(Some("localhost:7790"), Some("https://evil.example"), 7790), "another site's page");
        assert!(!trusted(Some("localhost:7790"), Some("null"), 7790));
        assert!(!trusted(Some("evil.example:7790"), None, 7790), "a name resolving here (DNS rebinding)");
        assert!(!trusted(Some("localhost:7777"), None, 7790), "another port");
        assert!(!trusted(None, None, 7790));
    }
}
