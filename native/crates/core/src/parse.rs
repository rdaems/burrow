//! What the machines say, read: `claude agents --json --all`, the job records' grep, `/usage`, the peer
//! messages in the transcripts, the worktree fold, `~/.ssh/config`; and a transcript condensed into the
//! story the notes are written from.

use crate::util::{text, when};
use burrow_model::{Bot, Limit, Usage};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// a job's record, the fields the JOBS grep takes: updatedAt, detail, bridgeSessionId, tempo,
/// resumeSessionId, worktreePath
pub type Jobs = HashMap<String, HashMap<String, String>>;

/// the sessions `claude agents --json --all` lists: the background ones (an interactive `claude` in
/// someone's terminal is listed too, but nothing can attach to it, so it is left out, as agent view does).
/// What a bot does is its tempo (its record's), not its state: the state is Claude Code's reading of how
/// the last message ends (the same "still listening" was `working` once and `done` the next time), the
/// tempo is what happens:
///   busy     tempo active: a turn runs, its process there
///   waiting  a dialog open (`status: waiting`): a multiple-choice question, a permission prompt. A last
///            message that only reads as a question (tempo blocked, no dialog) is idle: it waits for
///            your next prompt, as a done one does
///   idle     the rest: done, a background command left running, its process stopped after an hour, a
///            session that completed 48 days ago; opening one restarts it
/// A row without a tempo (an old record) gets Claude Code's own fallback from the process's status.
/// A row without a process takes its conversation and folder from the record (resumeSessionId,
/// worktreePath): `claude agents` falls back to the ones it started with once the process is gone.
pub fn parse_agents(out: &str, jobs: &Jobs) -> Result<Vec<Bot>, String> {
    let out = out.trim();
    let rows: Vec<Value> = serde_json::from_str(if out.is_empty() { "[]" } else { out }).map_err(|e| e.to_string())?;
    let none = HashMap::new();
    let mut bots = vec![];
    for x in rows {
        let id = text(x.get("id"));
        if x.get("kind").and_then(Value::as_str) != Some("background") || id.is_empty() {
            continue;
        }
        let state = x.get("state").and_then(Value::as_str).unwrap_or("").to_string();
        let status = x.get("status").and_then(Value::as_str).unwrap_or("");
        let j = jobs.get(&id).unwrap_or(&none);
        let jget = |k: &str| j.get(k).filter(|v| !v.is_empty()).cloned();
        let alive = x.get("pid").is_some();
        let (mut session, mut cwd) = (text(x.get("sessionId")), text(x.get("cwd")));
        if !alive {
            session = jget("resumeSessionId").unwrap_or(session);
            cwd = jget("worktreePath").unwrap_or(cwd);
        }
        let tempo = jget("tempo").unwrap_or_else(|| (if status == "busy" { "active" } else if status == "waiting" { "blocked" } else { "idle" }).into());
        let waiting = alive && status == "waiting";
        let over = !alive && matches!(state.as_str(), "done" | "failed" | "stopped");
        let updated = jget("updatedAt").map(|t| when(&t)).unwrap_or(0);
        let name = text(x.get("name"));
        bots.push(Bot {
            name: if name.is_empty() { id.clone() } else { name },
            id,
            cwd,
            busy: alive && !waiting && tempo == "active",
            waiting,
            alive,
            dialog: waiting, // a prompt open in its session: nothing can be typed to it (the notes send you there)
            over: if over { state.clone() } else { String::new() },
            ended_at: if over { updated } else { 0 },
            detail: jget("detail").unwrap_or_default(),
            updated, // its record's last write: the notes on it are older, or not
            started: num(x.get("startedAt")),
            pid: num(x.get("pid")),
            session,
            rc: jget("bridgeSessionId").unwrap_or_default(), // its Remote Control session (cse_…): in the Claude app too
            state,
            tempo,
            ..Bot::default()
        });
    }
    bots.sort_by_key(|b| b.started);
    Ok(bots)
}

fn num(v: Option<&Value>) -> i64 {
    v.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0)
}

static JOB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(?:.*/)?([^/:]+)/state\.json:"(updatedAt|detail|bridgeSessionId|tempo|resumeSessionId|worktreePath)": *"(.*)"$"#).unwrap()
});

/// {id: {updatedAt, detail, bridgeSessionId, tempo, resumeSessionId, worktreePath}} from the records' lines
pub fn parse_jobs(out: &str) -> Jobs {
    let mut jobs = Jobs::new();
    for line in out.lines() {
        let Some(m) = JOB.captures(line) else { continue };
        let raw = &m[3];
        let v = serde_json::from_str::<String>(&format!("\"{raw}\"")).unwrap_or_else(|_| raw.to_string());
        jobs.entry(m[1].to_string()).or_default().insert(m[2].to_string(), v);
    }
    jobs
}

static USAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*Current (.+?): (\d+)% used(?: · resets (.*?))?\s*(?:\(.*\))?$").unwrap());

/// `/usage`: "Current session: 3% used · resets Oct 21 at 3:20pm (…)" → session, week, a bar for every
/// "Current …" line, and `text`: the rest of what it says (what contributes to the limits). None when it
/// said no such line
pub fn parse_usage(out: &str) -> Option<Usage> {
    let (mut u, mut text) = (Usage::default(), vec![]);
    for line in out.lines() {
        if let Some(m) = USAGE.captures(line) {
            let bar = Limit { label: m[1].to_string(), used: m[2].parse().unwrap_or(0), resets: m.get(3).map(|x| x.as_str().to_string()).unwrap_or_default() };
            match bar.label.as_str() {
                "session" => u.session = Some(bar.clone()),
                "week (all models)" => u.week = Some(bar.clone()),
                _ => {}
            }
            u.bars.push(bar);
        } else if !line.starts_with("You are currently") {
            text.push(line.trim_end());
        }
    }
    if u.bars.is_empty() {
        return None;
    }
    u.text = text.join("\n").trim().to_string();
    Some(u)
}

/// a key's value in a JSON fragment cut off anywhere (`"key":"value` or `"key":123`)
pub fn field(s: &str, key: &str) -> Option<String> {
    let re = Regex::new(&format!(r#""{}":"?([^",}}]*)"#, regex::escape(key))).ok()?;
    re.captures(s).map(|m| m[1].to_string())
}

/// (msg_id, from, to) by bot id, for the messages between bots that are there now
pub fn parse_peers(out: &str, bots: &[Bot]) -> Vec<(String, String, String)> {
    let mut talks = vec![];
    for line in out.lines() {
        let (session, origin) = line.split_once(' ').unwrap_or((line, ""));
        let (Some(msg), Some(to)) = (field(origin, "msg_id"), bots.iter().find(|b| b.session == session)) else { continue };
        if msg.is_empty() {
            continue;
        }
        let (pid, name) = (field(origin, "verifiedPeerPid").filter(|p| !p.is_empty()), field(origin, "name"));
        let frm = pid
            .as_ref()
            .and_then(|p| bots.iter().find(|b| &b.pid.to_string() == p))
            .or_else(|| bots.iter().find(|b| Some(&b.name) == name.as_ref()));
        if let Some(frm) = frm {
            if frm.id != to.id {
                talks.push((msg, frm.id.clone(), to.id.clone()));
            }
        }
    }
    talks
}

/// (the desk's directory, the worktree's or "") for a session started in cwd, from FOLD's line for it
/// ("<worktree top>\t<the repo's .git, resolved>")
pub fn fold(cwd: &str, line: &str) -> (String, String) {
    let (top, common) = line.split_once('\t').unwrap_or((line, ""));
    let repo = common.strip_suffix("/.git").unwrap_or("");
    if !repo.is_empty() && !top.is_empty() && top != repo {
        return (repo.to_string(), top.to_string());
    }
    // no git there (or a demo): Claude Code's own worktrees
    if let Some(i) = cwd.find("/.claude/worktrees/").filter(|&i| i > 0) {
        let name = cwd[i + 19..].split('/').next().unwrap_or("");
        return (cwd[..i].to_string(), format!("{}/.claude/worktrees/{}", &cwd[..i], name));
    }
    (cwd.to_string(), String::new())
}

// ---- the story of a session

/// characters of the story given to Claude, the newest
pub const STORY: usize = 80000;

static GIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bgit\s+(commit|push|merge|tag)\b|\bgh\s+pr\s+(create|merge)\b").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// the tags Claude Code wraps its own additions in, cut out with what they hold (TAGS in server.py:
/// `<(system-reminder|command-…|local-command-…|task-notification)>.*?</\1>`)
fn untag(s: &str) -> String {
    let (mut out, mut i) = (String::new(), 0);
    while let Some(off) = s[i..].find('<') {
        let at = i + off;
        let name_end = s[at + 1..].find('>').map(|e| at + 1 + e);
        let name = name_end.map(|e| &s[at + 1..e]).unwrap_or("");
        let ok = name == "system-reminder"
            || name == "task-notification"
            || [("command-", 8), ("local-command-", 14)].iter().any(|(p, n)| {
                name.starts_with(p) && name.len() > *n && name[*n..].bytes().all(|c| c.is_ascii_lowercase() || c == b'-')
            });
        if ok {
            let close = format!("</{name}>");
            if let Some(c) = s[name_end.unwrap() + 1..].find(&close) {
                out += &s[i..at];
                i = name_end.unwrap() + 1 + c + close.len();
                continue;
            }
        }
        out += &s[i..at + 1];
        i = at + 1;
    }
    out + &s[i..]
}

fn first(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Python's str() of a value from a record (None for a missing one)
fn pystr(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => (if *b { "True" } else { "False" }).into(),
        Some(v) => v.to_string(),
    }
}

/// a transcript's lines as the story of the session: what the person (or another session) said, what
/// the session said, the files it changed, the commands that commit or push, the questions it asked and
/// what came back; no thinking, no other tool output. The newest `cap` characters
pub fn condense(raw: &str, cap: usize) -> String {
    let mut out: Vec<String> = vec![];
    let mut asked: Vec<Value> = vec![];
    let said = |out: &mut Vec<String>, who: &str, t: Option<&Value>, n: usize| {
        let t = untag(t.and_then(Value::as_str).unwrap_or(""));
        let t = t.trim();
        if !t.is_empty() {
            out.push(format!("{who}: {}", first(t, n)));
        }
    };
    for line in raw.lines().skip(1) {
        // the first may be cut
        let Ok(d) = serde_json::from_str::<Value>(line) else { continue };
        let msg = d.get("message").filter(|m| m.is_object());
        let c = msg.and_then(|m| m.get("content"));
        let kind = d.get("type").and_then(Value::as_str);
        if kind == Some("user") {
            let o = d.get("origin");
            let peer = o.and_then(|o| o.get("kind")).and_then(Value::as_str) == Some("peer");
            let who = if peer {
                let n = text(o.and_then(|o| o.get("name")));
                format!("MESSAGE from {}", if n.is_empty() { "another session".into() } else { n })
            } else {
                "USER".into()
            };
            match c {
                Some(Value::String(_)) => said(&mut out, &who, c, 4000),
                Some(Value::Array(xs)) => {
                    for x in xs {
                        match x.get("type").and_then(Value::as_str) {
                            Some("text") => said(&mut out, &who, x.get("text"), 4000),
                            Some("tool_result") if asked.contains(x.get("tool_use_id").unwrap_or(&Value::Null)) => {
                                let r = match x.get("content") {
                                    Some(Value::Array(ys)) => ys.iter().filter(|y| y.is_object()).map(|y| text(y.get("text"))).collect::<Vec<_>>().join(" "),
                                    r => text(r),
                                };
                                out.push(format!("  → {}", first(&SPACES.replace_all(&r, " "), 800)));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        } else if kind == Some("assistant") {
            let Some(Value::Array(xs)) = c else { continue };
            for x in xs {
                match x.get("type").and_then(Value::as_str) {
                    Some("text") => said(&mut out, "CLAUDE", x.get("text"), 6000),
                    Some("tool_use") => {
                        let name = x.get("name").and_then(Value::as_str).unwrap_or("");
                        let empty = Value::Object(Default::default());
                        let inp = x.get("input").filter(|v| crate::util::truthy(v)).unwrap_or(&empty);
                        let id = x.get("id").cloned().unwrap_or(Value::Null);
                        match name {
                            "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => {
                                let mut p = text(inp.get("file_path"));
                                if p.is_empty() {
                                    p = text(inp.get("notebook_path"));
                                }
                                let e = format!("EDITED {}", if p.is_empty() { "?" } else { &p });
                                if out.last() != Some(&e) {
                                    out.push(e);
                                }
                            }
                            "Bash" if GIT.is_match(&text(inp.get("command"))) => {
                                out.push(format!("$ {}", first(&pystr(inp.get("command")), 600)));
                                asked.push(id);
                            }
                            "AskUserQuestion" => {
                                for q in inp.get("questions").and_then(Value::as_array).into_iter().flatten() {
                                    let opts: Vec<String> = q
                                        .get("options")
                                        .and_then(Value::as_array)
                                        .into_iter()
                                        .flatten()
                                        .filter(|o| o.is_object())
                                        .map(|o| format!("{} ({})", pystr(o.get("label")), o.get("description").map(|d| pystr(Some(d))).unwrap_or_default()))
                                        .collect();
                                    out.push(format!("ASKED: {} Options: {}", pystr(q.get("question")), opts.join("; ")));
                                }
                                asked.push(id);
                            }
                            "ExitPlanMode" => {
                                let plan = inp.get("plan").filter(|p| crate::util::truthy(p));
                                out.push(format!("CLAUDE proposed a plan{}", plan.map(|p| format!(": {}", first(&pystr(Some(p)), 3000))).unwrap_or_default()));
                                asked.push(id);
                            }
                            "Agent" | "Task" => out.push(format!("DELEGATED: {}", text(inp.get("description")))),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let all = out.join("\n");
    let n = all.chars().count();
    all.chars().skip(n.saturating_sub(cap)).collect()
}

// ---- ~/.ssh/config

static HOSTLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*(host|include)(?:\s+|\s*=\s*)(.+?)\s*$").unwrap());

/// the Host names of an ssh config (and the files it includes), in its order, for the new door's
/// field: not the patterns (*, ?, !) and not the hosts that have a door already (`doors`)
pub fn ssh_hosts(config: &Path, home: &Path, doors: &[String]) -> Vec<String> {
    let mut out = vec![];
    read_hosts(config, home, doors, 0, &mut out);
    out
}

fn read_hosts(path: &Path, home: &Path, doors: &[String], depth: usize, out: &mut Vec<String>) {
    let Ok(raw) = std::fs::read_to_string(path) else { return };
    for line in raw.lines() {
        let Some(m) = HOSTLINE.captures(line) else { continue };
        let host = m[1].eq_ignore_ascii_case("host");
        for word in m[2].split_whitespace() {
            if host {
                if !word.contains(['*', '?', '!']) && !doors.iter().any(|d| d == word) && !out.iter().any(|o| o == word) {
                    out.push(word.to_string());
                }
            } else if depth < 4 {
                let p = if word.starts_with('/') {
                    PathBuf::from(word)
                } else if let Some(r) = word.strip_prefix("~/") {
                    home.join(r)
                } else if word == "~" {
                    home.to_path_buf()
                } else {
                    home.join(".ssh").join(word)
                };
                for f in glob(&p) {
                    read_hosts(&f, home, doors, depth + 1, out);
                }
            }
        }
    }
}

/// the files a path with * and ? in it names, sorted (glob.glob)
pub fn glob(p: &Path) -> Vec<PathBuf> {
    let mut found = vec![PathBuf::from(if p.is_absolute() { "/" } else { "" })];
    for part in p.components().skip(if p.is_absolute() { 1 } else { 0 }) {
        let part = part.as_os_str().to_string_lossy().to_string();
        if !part.contains(['*', '?', '[']) {
            found = found.into_iter().map(|d| d.join(&part)).collect();
            continue;
        }
        let re = Regex::new(&format!(
            "^{}$",
            regex::escape(&part).replace(r"\*", "[^/]*").replace(r"\?", "[^/]").replace(r"\[", "[").replace(r"\]", "]")
        ));
        let Ok(re) = re else { return vec![] };
        let mut next = vec![];
        for d in found {
            let Ok(rd) = std::fs::read_dir(if d.as_os_str().is_empty() { Path::new(".") } else { &d }) else { continue };
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if re.is_match(&n) && (!n.starts_with('.') || part.starts_with('.')) {
                    next.push(d.join(n));
                }
            }
        }
        found = next;
    }
    let mut out: Vec<PathBuf> = found.into_iter().filter(|f| f.exists()).collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // made up after the formats server.py reads; no real session's data
    const AGENTS: &str = r#"[
      {"id": "a1", "kind": "background", "name": "busy-bea", "state": "working", "status": "busy", "pid": 501, "cwd": "/p/one", "sessionId": "s1", "startedAt": 3000},
      {"id": "a2", "kind": "background", "name": "asking-al", "state": "blocked", "status": "waiting", "waitingFor": "input needed", "pid": 502, "cwd": "/p/one", "sessionId": "s2", "startedAt": 2000},
      {"id": "a3", "kind": "background", "state": "blocked", "status": "idle", "pid": 503, "cwd": "/p/two", "sessionId": "s3", "startedAt": 1000},
      {"id": "a4", "kind": "background", "name": "old-olga", "state": "done", "cwd": "/p/two", "sessionId": "s4-first", "startedAt": 500},
      {"id": "a5", "kind": "background", "name": "still-sid", "state": "working", "status": "busy", "pid": 0, "cwd": "/p/three", "sessionId": "s5", "startedAt": 4000},
      {"id": "a6", "kind": "background", "name": "fallback-fay", "state": "working", "status": "busy", "pid": 506, "cwd": "/p/three", "sessionId": "s6", "startedAt": 5000},
      {"id": "i1", "kind": "interactive", "name": "a terminal", "status": "busy", "pid": 600, "cwd": "/p"},
      {"kind": "background", "name": "no id"}
    ]"#;
    const JOBS: &str = r#"/home/u/.claude/jobs/a1/state.json:"tempo": "active"
/home/u/.claude/jobs/a1/state.json:"bridgeSessionId": "cse_01abc"
/home/u/.claude/jobs/a2/state.json:"tempo": "blocked"
/home/u/.claude/jobs/a3/state.json:"tempo": "blocked"
/home/u/.claude/jobs/a3/state.json:"detail": "asks \"shall I build it?\""
/home/u/.claude/jobs/a4/state.json:"tempo": "idle"
/home/u/.claude/jobs/a4/state.json:"updatedAt": "2026-10-24T12:00:00.000Z"
/home/u/.claude/jobs/a4/state.json:"resumeSessionId": "s4-after-clear"
/home/u/.claude/jobs/a4/state.json:"worktreePath": "/p/two/.claude/worktrees/w"
/home/u/.claude/jobs/a5/state.json:"tempo": "idle"
not a record line"#;

    #[test]
    fn jobs_read() {
        let j = parse_jobs(JOBS);
        assert_eq!(j["a1"]["bridgeSessionId"], "cse_01abc");
        assert_eq!(j["a3"]["detail"], "asks \"shall I build it?\"");
        assert_eq!(j.len(), 5);
    }

    #[test]
    fn agents_by_tempo() {
        let bots = parse_agents(AGENTS, &parse_jobs(JOBS)).unwrap();
        let by = |id: &str| bots.iter().find(|b| b.id == id).unwrap().clone();
        assert_eq!(bots.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["a4", "a3", "a2", "a1", "a5", "a6"]);
        // busy: a live process and tempo active
        let a1 = by("a1");
        assert!(a1.busy && !a1.waiting && a1.alive && a1.rc == "cse_01abc");
        // waiting: a dialog open
        let a2 = by("a2");
        assert!(a2.waiting && a2.dialog && !a2.busy);
        // tempo blocked without a dialog: idle; unnamed: its id
        let a3 = by("a3");
        assert!(!a3.busy && !a3.waiting && a3.name == "a3" && a3.tempo == "blocked");
        // over: no process, done; its conversation and folder from the record
        let a4 = by("a4");
        assert!(!a4.alive && a4.over == "done" && a4.ended_at == 1792843200000);
        assert_eq!((a4.session.as_str(), a4.cwd.as_str()), ("s4-after-clear", "/p/two/.claude/worktrees/w"));
        // pid 0 is a pid: alive; its record says idle, so not busy whatever its status
        let a5 = by("a5");
        assert!(a5.alive && !a5.busy);
        // no record: Claude Code's own fallback from the status
        let a6 = by("a6");
        assert!(a6.busy && a6.tempo == "active");
        assert!(parse_agents("", &Jobs::new()).unwrap().is_empty());
    }

    #[test]
    fn usage_read() {
        let out = "You are currently on the Max plan\n\n  Current session: 3% used · resets Oct 21 at 3:20pm (Europe/Brussels)\n  Current week (all models): 77% used · resets Oct 24 at 9am\n  Current week (Opus): 12% used\n\nWhat contributes: long sessions.\n";
        let u = parse_usage(out).unwrap();
        assert_eq!(u.session.as_ref().unwrap().used, 3);
        assert_eq!(u.session.as_ref().unwrap().resets, "Oct 21 at 3:20pm");
        assert_eq!(u.week.as_ref().unwrap().resets, "Oct 24 at 9am");
        assert_eq!(u.bars.len(), 3);
        assert_eq!(u.bars[2].label, "week (Opus)");
        assert_eq!(u.bars[2].resets, "");
        assert_eq!(u.text, "What contributes: long sessions.");
        assert!(parse_usage("Error: not logged in").is_none());
    }

    #[test]
    fn peers_read() {
        let bots = parse_agents(AGENTS, &Jobs::new()).unwrap();
        let out = "s2 \"origin\":{\"kind\":\"peer\",\"from\":\"uds:/tmp/cc-socks/501.sock\",\"verifiedPeerPid\":501,\"msg_id\":\"m1\",\"name\":\"busy-bea\"\n\
                   s3 \"origin\":{\"kind\":\"peer\",\"msg_id\":\"m2\",\"name\":\"asking-al\"\n\
                   s1 \"origin\":{\"kind\":\"peer\",\"msg_id\":\"m3\",\"name\":\"busy-bea\"\n\
                   nobody \"origin\":{\"kind\":\"peer\",\"msg_id\":\"m4\",\"name\":\"busy-bea\"";
        let t = parse_peers(out, &bots);
        assert_eq!(t, [("m1".into(), "a1".into(), "a2".into()), ("m2".into(), "a2".into(), "a3".into())]);
    }

    #[test]
    fn worktrees_fold() {
        assert_eq!(fold("/r/wt", "/r/wt\t/r/main/.git"), ("/r/main".into(), "/r/wt".into()));
        assert_eq!(fold("/r/main/src", "/r/main\t/r/main/.git"), ("/r/main/src".into(), "".into()));
        assert_eq!(fold("/r/k/.claude/worktrees/tails/src", ""), ("/r/k".into(), "/r/k/.claude/worktrees/tails".into()));
        assert_eq!(fold("/plain", ""), ("/plain".into(), "".into()));
    }

    #[test]
    fn the_story() {
        let lines = [
            r#"{"cut off"#,
            r#"{"type":"user","message":{"content":"Build the <system-reminder>secret</system-reminder>booth"}}"#,
            r#"{"type":"user","origin":{"kind":"peer","name":"tidy-tess"},"message":{"content":[{"type":"text","text":"hello from next door"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":"On it."},{"type":"tool_use","id":"t1","name":"Edit","input":{"file_path":"/p/a.rs"}},{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"/p/a.rs"}},{"type":"tool_use","id":"t3","name":"Bash","input":{"command":"cargo test"}},{"type":"tool_use","id":"t4","name":"Bash","input":{"command":"git commit -m x"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t3","content":"ok"},{"type":"tool_result","tool_use_id":"t4","content":[{"type":"text","text":"[main 3f2a9c1]\n x"}]}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t5","name":"AskUserQuestion","input":{"questions":[{"question":"Which colour?","options":[{"label":"Red","description":"warm"},{"label":"Blue"}]}]}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t5","content":"Blue"}]}}"#,
            r#"{"type":"user","message":{"content":"<command-name>/clear</command-name><local-command-stdout></local-command-stdout>"}}"#,
        ];
        let s = condense(&lines.join("\n"), STORY);
        assert_eq!(
            s,
            "USER: Build the booth\nMESSAGE from tidy-tess: hello from next door\nCLAUDE: On it.\nEDITED /p/a.rs\n$ git commit -m x\n  → [main 3f2a9c1] x\nASKED: Which colour? Options: Red (warm); Blue ()\n  → Blue"
        );
        assert_eq!(condense(&lines.join("\n"), 4), "Blue");
    }

    #[test]
    fn ssh_config_hosts() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        std::fs::create_dir_all(home.join(".ssh/conf.d")).unwrap();
        std::fs::write(home.join(".ssh/config"), "Host odyssey arthroid\n  HostName 10.0.0.2\nHost *.lan !bad\nInclude conf.d/*\nhost=lab\nHost odyssey\n").unwrap();
        std::fs::write(home.join(".ssh/conf.d/b"), "Host beta\n").unwrap();
        std::fs::write(home.join(".ssh/conf.d/a"), "Host alpha\n").unwrap();
        let hosts = ssh_hosts(&home.join(".ssh/config"), home, &["arthroid".into()]);
        assert_eq!(hosts, ["odyssey", "alpha", "beta", "lab"]);
    }
}
