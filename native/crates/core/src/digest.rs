//! The office manager's notes: what each session is doing, which decisions are open, what it built,
//! written by Claude (`claude -p`, here, never on the machines) from the session's record and the newest
//! part of its transcript, when it finishes a turn, asks you something, or the notes are opened. Kept in
//! ~/.config/burrow/digest.json: a session is written up again only when its transcript has grown.

use crate::config::Config;
use crate::hub::{DigestView, Event, Hub, Res};
use crate::parse::{condense, STORY};
use crate::runner::Mach;
use crate::scripts::{self, TAIL};
use crate::util::{self, sq, text, truthy};
use burrow_model::Bot;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;

pub const SESSION_NOTES: &str = "You keep the notes for a person who runs many Claude Code background sessions at once and cannot follow \
them all. On stdin: one session's record, the notes you wrote on it last time (if any), and the most recent part of its \
transcript (USER = the person, MESSAGE = another session, CLAUDE = the session, EDITED = files it changed, $ = commands \
that commit or push, ASKED = questions it put to the person, → = what came back). Write the notes anew, so that the \
person understands in depth what happened without reading the transcript:
- headline: one line, twelve words at most, for a list of all sessions: where it stands; if it waits on the person, what for (e.g. \"Waits for a ticket number and an OK to push to main\").
- goal: what the person wants from this session overall, in one or two sentences (not only the last prompt).
- now: two to five sentences: where it stands, what it is doing or where it stopped, and why.
- decisions: every decision still open for the person: questions it asked that were not answered, choices it left to \
them, things it flagged as undecided or risky. context: why it matters and what the session found; options: the options \
as it laid them out (empty if none); blocking: true if the session waits on this to go on, false if it can be answered any time. Most important first, five at most: fold the small ones together. Leave out what was decided.
- built: concrete results, newest first: features and fixes, the main files changed, commits (hash, message, pushed or \
not), tests run and how they went. Keep the items of the earlier notes that still hold; twelve at most.
- next: what it said it would do next, or what is left; empty if nothing.
Be plain and specific, no filler; use the session's own words where they are precise.";

pub const EXPLAIN: &str = "A Claude Code session left a decision to a person who has not followed the session. On stdin: the \
decision, the notes on the session, and the newest part of its transcript. Explain it so that the person can decide \
without reading the transcript:
- what: what exactly is being decided, in two or three plain sentences; explain the terms a newcomer would not know.
- why: why it matters, and what happens if nobody decides.
- options: each option the session laid out, and an obvious one it left out if there is one: what it means in \
practice (what the session will then do), its upside, and its cost or risk.
- recommendation: the option you would pick and why, in one or two sentences; say so if the session recommended one.
Plain and specific, no filler.";

pub const DESK_NOTES: &str = "Below are your notes on each Claude Code session working in one project. Write one short paragraph, three \
to five sentences, on where the project stands as a whole: what it is heading for, what landed recently, what is \
blocked or waits on the person. Plain and specific, no filler.";

pub fn session_schema() -> Value {
    json!({"type": "object", "additionalProperties": false, "required": ["headline", "goal", "now", "decisions", "built", "next"], "properties": {
        "headline": {"type": "string"}, "goal": {"type": "string"}, "now": {"type": "string"}, "next": {"type": "string"},
        "built": {"type": "array", "items": {"type": "string"}},
        "decisions": {"type": "array", "items": {"type": "object", "additionalProperties": false, "required": ["question", "context", "options", "blocking"], "properties": {
            "question": {"type": "string"}, "context": {"type": "string"}, "options": {"type": "array", "items": {"type": "string"}},
            "blocking": {"type": "boolean"}}}}}})
}

pub fn explain_schema() -> Value {
    json!({"type": "object", "additionalProperties": false, "required": ["what", "why", "options", "recommendation"], "properties": {
        "what": {"type": "string"}, "why": {"type": "string"}, "recommendation": {"type": "string"},
        "options": {"type": "array", "items": {"type": "object", "additionalProperties": false, "required": ["option", "means", "pro", "con"], "properties": {
            "option": {"type": "string"}, "means": {"type": "string"}, "pro": {"type": "string"}, "con": {"type": "string"}}}}}})
}

pub fn desk_schema() -> Value {
    json!({"type": "object", "additionalProperties": false, "required": ["text"], "properties": {"text": {"type": "string"}}})
}

pub const NOTE_KEYS: [&str; 6] = ["headline", "goal", "now", "decisions", "built", "next"];

/// digest.json's sessions and desks; the demo's are made up as it goes, never the real ones
pub fn load_digest(cfg: &Config) -> (Map<String, Value>, Map<String, Value>) {
    if cfg.demo {
        return Default::default();
    }
    let d: Value = std::fs::read_to_string(cfg.digest()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let part = |k: &str| d.get(k).and_then(Value::as_object).cloned().unwrap_or_default();
    (part("sessions"), part("desks"))
}

/// what claude -p printed, read: the JSON the schema asked for
pub fn read_answer(out: &str) -> Res<Value> {
    let last = out.trim().lines().last().unwrap_or("");
    let r: Value = serde_json::from_str(last).map_err(|_| {
        let t: String = out.trim().chars().take(300).collect();
        if t.is_empty() { "claude -p said nothing".to_string() } else { t }
    })?;
    if r.get("is_error").is_some_and(truthy) {
        return Err(text(r.get("result")).chars().take(300).collect());
    }
    if let Some(o) = r.get("structured_output").filter(|o| o.is_object()) {
        return Ok(o.clone());
    }
    static OBJ: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)\{.*\}").unwrap());
    let res = text(r.get("result"));
    match OBJ.find(&res) {
        Some(m) => serde_json::from_str(m.as_str()).map_err(|e| e.to_string()),
        None => Ok(json!({})),
    }
}

fn notes_of(e: &Value) -> String {
    let m: Map<String, Value> = NOTE_KEYS.iter().map(|k| (k.to_string(), e.get(*k).cloned().unwrap_or(Value::Null))).collect();
    Value::Object(m).to_string()
}

impl Hub {
    pub fn digest_view(&self) -> DigestView {
        let g = self.lock();
        DigestView { sessions: g.digest.0.clone(), desks: g.digest.1.clone(), writing: g.writing.clone(), explaining: g.explaining.clone(), on: g.notes_on() }
    }

    pub(crate) fn push_digest(&self) {
        let v = self.digest_view();
        self.push(Event::Digest(v));
    }

    /// a session that finished a turn, asks you something or is over gets written up (on every poll)
    pub(crate) fn digest_watch(&self, bots: &[Bot]) {
        let mut todo = vec![];
        {
            let mut g = self.lock();
            let on = g.notes_on();
            for b in bots {
                let now = (b.busy, b.waiting, b.over.clone());
                let was = g.seen.insert(b.id.clone(), now.clone());
                if on && was.is_some_and(|w| w != now) && !b.busy {
                    todo.push(b.id.clone());
                }
            }
        }
        if !todo.is_empty() {
            self.digest_queue(&todo);
        }
    }

    /// the notes opened (/api/digest): every session whose record moved since its notes were written (the
    /// ones over only if they ended in the last three days), those that wait for you first. How many
    pub fn digest(&self) -> usize {
        if !self.lock().notes_on() {
            return 0;
        }
        self.digest_refresh()
    }

    pub fn digest_refresh(&self) -> usize {
        let cut = ((util::now() - 3.0 * 86400.0) * 1000.0) as i64;
        let mut todo: Vec<Bot> = {
            let g = self.lock();
            let ss = &g.digest.0;
            g.bots()
                .into_iter()
                .filter(|b| {
                    let e = ss.get(&b.id);
                    (b.over.is_empty() || b.ended_at > cut)
                        && (e.is_none_or(|e| e.get("headline").is_none()) || b.updated == 0 || b.updated > e.and_then(|e| e.get("src")).and_then(Value::as_i64).unwrap_or(0))
                })
                .collect()
        };
        todo.sort_by_key(|b| (!b.waiting, -(if b.updated != 0 { b.updated } else { b.started })));
        let ids: Vec<String> = todo.iter().map(|b| b.id.clone()).collect();
        self.digest_queue(&ids);
        ids.len()
    }

    pub(crate) fn digest_queue(&self, ids: &[String]) {
        let writing = self.lock().writing.clone();
        let mut q = self.dq.lock().unwrap();
        for i in ids {
            if !q.contains(i) && Some(i) != writing.as_ref() {
                q.push(i.clone());
            }
        }
        self.dcv.notify_all();
    }

    pub(crate) fn digest_worker(&self) {
        let mut dirty: Vec<String> = vec![]; // desks whose sessions' notes changed: their paragraph next
        loop {
            let bid = {
                let mut q = self.dq.lock().unwrap();
                while q.is_empty() && dirty.is_empty() {
                    q = self.dcv.wait(q).unwrap();
                }
                if q.is_empty() { None } else { Some(q.remove(0)) }
            };
            let r = match &bid {
                Some(id) => {
                    let b = self.bots().into_iter().find(|x| &x.id == id);
                    match b {
                        Some(b) if self.lock().notes_on() => self.digest_one(&b).map(|changed| {
                            if changed && !dirty.contains(&b.unit) {
                                dirty.push(b.unit.clone());
                            }
                        }),
                        _ => Ok(()),
                    }
                }
                None => {
                    let u = dirty.pop().unwrap();
                    self.digest_desk(&u)
                }
            };
            if let Err(e) = r {
                eprintln!("notes on {}: {}", bid.as_deref().unwrap_or("a desk"), e.chars().take(300).collect::<String>());
            }
            if self.lock().writing.take().is_some() {
                self.push_digest();
            }
        }
    }

    /// Claude, here, without tools, MCP servers or settings (no hooks): the JSON the schema asks for
    pub(crate) fn summarize(&self, prompt: &str, story: &str, schema: &Value) -> Res<Value> {
        let _ = std::fs::create_dir_all(&self.cfg.cache);
        let cmd = format!(
            "cd {} && exec claude -p --no-session-persistence --model {} --tools '' --strict-mcp-config --setting-sources '' --output-format json --json-schema {} {} 2>&1",
            sq(&self.cfg.cache.to_string_lossy()),
            sq(&self.cfg.digest_model),
            sq(&schema.to_string()),
            sq(prompt)
        );
        let o = self
            .runner()
            .exec(&self.local(), &cmd, Some(story.as_bytes()), Duration::from_secs(600), &mut |pid| self.own(pid))
            .map_err(|e| if e == "timed out" { "claude -p took too long".to_string() } else { e })?;
        read_answer(&String::from_utf8_lossy(&o.stdout))
    }

    /// (its record, its transcript's size, the transcript's tail unless it is still `size` long)
    pub(crate) fn slice(&self, m: &Mach, b: &Bot, size: i64) -> Res<(Value, i64, String)> {
        let o = self.runner().exec(m, &scripts::slice(&b.id, &b.session, size), None, Duration::from_secs(60), &mut |_| {})?;
        let out = String::from_utf8_lossy(&o.stdout).into_owned();
        let (rec, rest) = out.split_once("@@burrow@@\n").unwrap_or((&out, ""));
        let (n, raw) = rest.split_once('\n').unwrap_or((rest, ""));
        let job = serde_json::from_str(if rec.trim().is_empty() { "{}" } else { rec.trim() }).unwrap_or(json!({}));
        Ok((job, n.trim().parse().unwrap_or(0), raw.to_string()))
    }

    /// a decision explained (what, why, the options, a recommendation), kept with the notes
    pub fn explain(&self, bid: &str, question: &str) -> Res<Value> {
        let (b, e) = {
            let g = self.lock();
            (g.bots().into_iter().find(|x| x.id == bid), g.digest.0.get(bid).cloned())
        };
        let (Some(b), Some(e)) = (b, e) else { return Err("no notes on that session".into()) };
        let q = e.get("decisions").and_then(Value::as_array).into_iter().flatten().find(|q| q.get("question").and_then(Value::as_str) == Some(question)).cloned();
        let Some(q) = q else { return Err("that decision is not open any more".into()) };
        if let Some(have) = e.get("explained").and_then(|x| x.get(question)).filter(|h| truthy(h)) {
            return Ok(have.clone());
        }
        let m = self.machine(&b.m)?;
        self.lock().explaining = Some(bid.to_string());
        self.push_digest();
        let x = (|| {
            if m.demo() {
                std::thread::sleep(Duration::from_millis(1500));
                let opts: Vec<String> = q.get("options").and_then(Value::as_array).filter(|o| !o.is_empty()).map(|o| o.iter().map(|x| text(Some(x))).collect()).unwrap_or_else(|| vec!["Keep it".into()]);
                return Ok(json!({"what": "Whether the change goes to main as one commit or as the steps it was made in.", "why": "The log is how the change is read later; nothing breaks either way.",
                    "options": opts.iter().map(|o| json!({"option": o, "means": "The log shows it that way.", "pro": "Simple.", "con": "Less to see."})).collect::<Vec<_>>(),
                    "recommendation": "Squash: one commit a feature, as main has had so far."}));
            }
            let (_, _, raw) = self.slice(&m, &b, -1)?;
            self.summarize(
                EXPLAIN,
                &format!("THE DECISION:\n{}\n\nTHE NOTES ON SESSION {} ({}):\n{}\n\nTRANSCRIPT (the most recent part):\n{}", q, b.name, b.cwd, notes_of(&e), condense(&raw, STORY)),
                &explain_schema(),
            )
        })();
        self.lock().explaining = None;
        let mut x = x?;
        if let Some(o) = x.as_object_mut() {
            o.insert("at".into(), json!(util::now_ms()));
        }
        {
            let mut g = self.lock();
            if let Some(e) = g.digest.0.get_mut(bid).and_then(Value::as_object_mut) {
                let ex = e.entry("explained").or_insert(json!({}));
                if let Some(ex) = ex.as_object_mut() {
                    ex.insert(question.to_string(), x.clone());
                }
            }
            self.save_digest(&mut g.digest);
        }
        self.push_digest();
        Ok(x)
    }

    /// your decision typed into the session (deliver), and kept with the notes as answered. It blocks
    /// while it types (a few seconds; longer for a session whose process was off)
    pub fn answer(&self, bid: &str, question: &str, text: &str) -> Res<()> {
        let text: String = text.trim().chars().take(4000).collect();
        if text.is_empty() {
            return Err("an answer, first".into());
        }
        let b = self.bots().into_iter().find(|x| x.id == bid).ok_or("no such session")?;
        if b.dialog {
            return Err("it has a prompt open in its session: answer it there".into());
        }
        let m = self.machine(&b.m)?;
        let msg = if question.is_empty() { text.clone() } else { format!("About your question “{question}”: {text}") };
        if m.demo() {
            let mut g = self.lock();
            for x in g.demo_bots.get_mut(&b.m).into_iter().flatten() {
                if x.id == bid {
                    x.busy = true;
                    x.waiting = false;
                    x.state = "working".into();
                    x.over = String::new();
                    x.alive = true;
                }
            }
        } else {
            crate::term::deliver(self.runner(), &m, bid, &msg, Duration::from_secs(60))?;
        }
        {
            let mut g = self.lock();
            if let Some(e) = g.digest.0.get_mut(bid).and_then(Value::as_object_mut) {
                let a = e.entry("answered").or_insert(json!({}));
                if let Some(a) = a.as_object_mut() {
                    a.insert(question.to_string(), json!({"text": text, "at": util::now_ms()}));
                }
                self.save_digest(&mut g.digest);
            }
        }
        self.poke(&b.m);
        self.push_digest();
        Ok(())
    }

    /// a session written up: true if its notes changed
    pub(crate) fn digest_one(&self, b: &Bot) -> Res<bool> {
        let old: Map<String, Value> = self.lock().digest.0.get(&b.id).and_then(Value::as_object).cloned().unwrap_or_default();
        let m = self.machine(&b.m)?;
        if m.demo() {
            return Ok(self.demo_notes(b, &old));
        }
        let (job, size, raw) = self.slice(&m, b, old.get("size").and_then(Value::as_i64).unwrap_or(-1))?;
        // no transcript, or nothing new in it (notes from before the headline are written again)
        if size == 0 || (!old.is_empty() && old.contains_key("headline") && Some(size) == old.get("size").and_then(Value::as_i64)) {
            if !old.is_empty() {
                let mut g = self.lock();
                if let Some(e) = g.digest.0.get_mut(&b.id).and_then(Value::as_object_mut) {
                    let src = if b.updated != 0 { json!(b.updated) } else { e.get("src").cloned().unwrap_or(json!(0)) };
                    e.insert("src".into(), src);
                }
            }
            return Ok(false);
        }
        let story = condense(&raw, STORY);
        if story.is_empty() {
            return Ok(false);
        }
        let out = job.get("output").filter(|o| o.is_object()).cloned().unwrap_or(json!({}));
        let state = if !b.over.is_empty() {
            format!("over ({})", b.over)
        } else if b.waiting {
            "showing the person a prompt".into()
        } else if b.busy {
            "working".into()
        } else if b.tempo == "blocked" {
            "waiting for the person: its last message asks something".into()
        } else {
            "idle, done with its turn".into()
        };
        let or_dash = |s: String| if s.is_empty() { "-".to_string() } else { s };
        let detail = { let d = text(job.get("detail")); if d.is_empty() { b.detail.clone() } else { d } };
        let head = format!(
            "SESSION {} in {} on {}, now {state}.\nIts own one-line summary: {}\nIts last result: {}\n",
            b.name,
            b.cwd,
            m.name,
            or_dash(detail),
            or_dash(text(out.get("result"))).chars().take(2000).collect::<String>()
        );
        let prev = if old.is_empty() { "none yet".to_string() } else { notes_of(&Value::Object(old.clone())) };
        self.lock().writing = Some(b.id.clone());
        self.push_digest();
        let r = self.summarize(SESSION_NOTES, &format!("{head}\nYOUR NOTES LAST TIME:\n{prev}\n\nTRANSCRIPT (the most recent part):\n{story}"), &session_schema())?;
        let mut notes = Map::new();
        for k in NOTE_KEYS {
            let v = r.get(k).filter(|v| truthy(v)).cloned().unwrap_or(if k == "decisions" || k == "built" { json!([]) } else { json!("") });
            notes.insert(k.into(), v);
        }
        let keep = |k: &str| old.get(k).filter(|v| truthy(v)).cloned().unwrap_or(json!({}));
        notes.insert("explained".into(), keep("explained"));
        notes.insert("answered".into(), keep("answered"));
        for (k, v) in [
            ("name", json!(b.name)),
            ("unit", json!(b.unit)),
            ("m", json!(b.m)),
            ("state", json!(state)),
            ("at", json!(util::now_ms())),
            ("src", json!(b.updated)),
            ("size", json!(size)),
            ("read", json!(story.chars().count())),
            ("whole", json!(raw.len() < TAIL && story.chars().count() < STORY)),
        ] {
            notes.insert(k.into(), v);
        }
        {
            let mut g = self.lock();
            g.digest.0.insert(b.id.clone(), Value::Object(notes));
            self.save_digest(&mut g.digest);
        }
        self.push_digest();
        Ok(true)
    }

    /// a desk's paragraph, from the notes on its sessions that are still listed
    pub(crate) fn digest_desk(&self, u: &str) -> Res<()> {
        let (ents, demo) = {
            let g = self.lock();
            let ids: HashSet<String> = g.bots().into_iter().map(|b| b.id).collect();
            let ents: Vec<Value> = g.digest.0.iter().filter(|(i, e)| e.get("unit").and_then(Value::as_str) == Some(u) && ids.contains(*i)).map(|(_, e)| e.clone()).collect();
            let demo = ents.first().and_then(|e| g.machines.get(&text(e.get("m")))).is_some_and(|m| m.demo());
            (ents, demo)
        };
        if ents.is_empty() {
            return Ok(());
        }
        let t = if demo {
            format!("{} sessions here. Most of the work is the room itself; one waits on a decision from you about the layout.", ents.len())
        } else {
            self.lock().writing = Some(format!("u:{u}"));
            self.push_digest();
            let story: Vec<String> = ents.iter().map(|e| format!("SESSION {} ({}):\n{}", text(e.get("name")), text(e.get("state")), notes_of(e))).collect();
            text(self.summarize(DESK_NOTES, &format!("PROJECT {u}\n\n{}", story.join("\n\n")), &desk_schema())?.get("text"))
        };
        {
            let mut g = self.lock();
            g.digest.1.insert(u.to_string(), json!({"text": t, "at": util::now_ms()}));
            self.save_digest(&mut g.digest);
        }
        self.push_digest();
        Ok(())
    }

    /// kept, without the notes on sessions gone for two weeks
    pub(crate) fn save_digest(&self, d: &mut (Map<String, Value>, Map<String, Value>)) {
        let cut = ((util::now() - 14.0 * 86400.0) * 1000.0) as i64;
        d.0.retain(|_, e| e.get("at").and_then(Value::as_i64).unwrap_or(0) >= cut);
        if self.cfg.demo {
            return;
        }
        let _ = std::fs::create_dir_all(&self.cfg.config);
        let v = json!({"sessions": d.0, "desks": d.1});
        let tmp = self.cfg.digest().with_extension("json.tmp");
        if std::fs::write(&tmp, indent1(&v)).is_ok() {
            let _ = std::fs::rename(&tmp, self.cfg.digest());
        }
    }

    /// the demo's notes: made up, a moment of writing
    pub(crate) fn demo_notes(&self, b: &Bot, old: &Map<String, Value>) -> bool {
        if !old.is_empty() && old.get("src").and_then(Value::as_i64) == Some(b.updated) && old.get("state").and_then(Value::as_str) == Some(b.state.as_str()) {
            return false;
        }
        self.lock().writing = Some(b.id.clone());
        self.push_digest();
        std::thread::sleep(Duration::from_millis(1200));
        let ask = if b.waiting {
            json!([{"question": "Squash the four commits before pushing, or keep them as they are?", "context": "They tell the change step by step; so far main has one commit a feature.",
                    "options": ["Squash into one", "Keep the four"], "blocking": true},
                   {"question": "Rename the settings key now, or with the next release?", "context": "Older rooms still have the old key; moving it over is one line.", "options": [], "blocking": false}])
        } else {
            json!([])
        };
        let place = b.cwd.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("home");
        let notes = json!({
            "headline": if b.waiting { "Waits for your call on the commits" } else if b.busy { "Running the tests on its change" } else { "Change in and tested, not pushed" },
            "goal": format!("Work on {place}: {} was asked to tidy up and finish the open edges.", b.name),
            "now": if b.waiting { "It is waiting for your answer before it goes on." } else if b.busy { "It is in the middle of a change and running the tests." } else { "It finished its turn: the change is in and tested, nothing is pushed yet." },
            "decisions": ask, "built": ["The room's layout code split into layout() and foot()", "Commit 3f2a9c1 “desks keep their place” (not pushed)", "Tests: 14 passed"],
            "next": "Push once you have looked at it.", "name": b.name, "unit": b.unit, "m": b.m, "state": b.state,
            "at": util::now_ms(), "src": b.updated, "size": 1, "read": 1200, "whole": true});
        {
            let mut g = self.lock();
            g.digest.0.insert(b.id.clone(), notes);
            self.save_digest(&mut g.digest);
        }
        self.push_digest();
        true
    }
}

/// JSON as Python's json.dump(indent=1, ensure_ascii=False) writes it
fn indent1(v: &Value) -> Vec<u8> {
    let mut out = vec![];
    let f = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut s = serde_json::Serializer::with_formatter(&mut out, f);
    let _ = serde::Serialize::serialize(v, &mut s);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_p_read() {
        let ok = r#"{"type":"result","is_error":false,"result":"","structured_output":{"text":"fine"}}"#;
        assert_eq!(read_answer(&format!("a warning\n{ok}\n")).unwrap(), json!({"text": "fine"}));
        let loose = r#"{"is_error":false,"result":"Here: {\"text\": \"x\"} done"}"#;
        assert_eq!(read_answer(loose).unwrap(), json!({"text": "x"}));
        assert_eq!(read_answer(r#"{"is_error":true,"result":"overloaded"}"#).unwrap_err(), "overloaded");
        assert_eq!(read_answer("").unwrap_err(), "claude -p said nothing");
        assert_eq!(read_answer("Error: no auth").unwrap_err(), "Error: no auth");
    }
}
