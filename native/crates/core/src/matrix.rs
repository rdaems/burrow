//! The matrix's backend (server.py's /api/stream and /api/tell): what the operator's view rains, the newest of what
//! each session said and did, as text; and a message typed into a session from the operator's line.

use crate::hub::{Hub, Res};
use crate::parse;
use crate::runner::run;
use crate::util::{self, sq};
use burrow_model::Bot;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

/// what the rain keeps: {session id: (its record's updatedAt, the text)}, and when each machine was last asked
#[derive(Default)]
pub struct Rain {
    pub texts: HashMap<String, (i64, String)>,
    pub asked: HashMap<String, f64>,
}

/// the newest of each transcript, a session a part after its `@@burrow@@ <session>` line
fn tails(sessions: &str) -> String {
    format!(
        r#"for s in {sessions}; do echo "@@burrow@@ $s"; f=$(ls ~/.claude/projects/*/"$s".jsonl 2>/dev/null | head -1); [ -n "$f" ] && tail -c 262144 "$f"; echo; done"#
    )
}

impl Hub {
    /// the matrix's rain (/api/stream): the newest of what each of these sessions said and did, as text. One command
    /// a machine for all of its sessions asked for, the newest 256 kB of each transcript; kept until the session's
    /// record changes, and a machine asked at most every ten seconds (odyssey's CPU)
    pub fn stream(&self, ids: &[String]) -> HashMap<String, String> {
        static SESSION: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^[A-Za-z0-9-]+$").unwrap());
        let bots: HashMap<String, Bot> = self.bots().into_iter().map(|b| (b.id.clone(), b)).collect();
        let mut out = HashMap::new();
        let mut ask: Vec<(String, Vec<Bot>)> = vec![];
        {
            let g = self.lock();
            for bid in ids.iter().take(200) {
                let Some(b) = bots.get(bid) else { continue };
                let have = g.rain.texts.get(bid);
                if let Some((at, text)) = have {
                    out.insert(bid.clone(), text.clone());
                    if *at == b.updated {
                        continue;
                    }
                }
                if SESSION.is_match(&b.session) {
                    match ask.iter_mut().find(|(m, _)| *m == b.m) {
                        Some((_, bs)) => bs.push(b.clone()),
                        None => ask.push((b.m.clone(), vec![b.clone()])),
                    }
                }
            }
        }
        let now = util::now();
        for (mid, bs) in ask {
            {
                let mut g = self.lock();
                if now - g.rain.asked.get(&mid).copied().unwrap_or(0.0) < 10.0 {
                    continue;
                }
                g.rain.asked.insert(mid.clone(), now);
            }
            let Ok(m) = self.machine(&mid) else { continue };
            if m.demo() {
                let mut g = self.lock();
                for b in bs {
                    let text = demo_stream(&b);
                    g.rain.texts.insert(b.id.clone(), (b.updated, text.clone()));
                    out.insert(b.id, text);
                }
                continue;
            }
            let sessions: Vec<String> = bs.iter().map(|b| sq(&b.session)).collect();
            let Ok(raw) = run(self.runner(), &m, &tails(&sessions.join(" ")), Duration::from_secs(30)) else { continue };
            let mut by: HashMap<&str, &str> = HashMap::new();
            for part in raw.split("@@burrow@@ ").skip(1) {
                let (sid, body) = part.split_once('\n').unwrap_or((part, ""));
                by.insert(sid.trim(), body);
            }
            let mut g = self.lock();
            for b in bs {
                let text = rain_text(by.get(b.session.as_str()).copied().unwrap_or(""));
                g.rain.texts.insert(b.id.clone(), (b.updated, text.clone()));
                out.insert(b.id, text);
            }
        }
        out
    }

    /// the matrix's `tell` (/api/tell): a message typed into a session (deliver), in the background
    pub fn tell(&self, id: &str, text: &str) -> Res<()> {
        let text: String = text.trim().chars().take(4000).collect();
        let bot = self.bots().into_iter().find(|b| b.id == id);
        let Some(bot) = bot.filter(|_| !text.is_empty()) else { return Err("tell whom what".into()) };
        if bot.dialog {
            return Err("it has a prompt open in its session: answer it there".into());
        }
        let m = self.machine(&bot.m)?;
        if m.demo() {
            let mut g = self.lock();
            for x in g.demo_bots.get_mut(&bot.m).into_iter().flatten().filter(|x| x.id == bot.id) {
                x.busy = true;
                x.waiting = false;
                x.state = "working".into();
                x.over.clear();
                x.alive = true;
            }
        } else {
            let h = self.arc();
            std::thread::spawn(move || {
                if let Err(e) = crate::term::deliver(h.runner(), &m, &bot.id, &text, Duration::from_secs(60)) {
                    eprintln!("burrow: tell {}: {e}", bot.name);
                }
            });
        }
        Ok(())
    }
}

fn first(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// a transcript's tail as the matrix's rain: what was said, the tools' calls (a command, a file, a pattern) and the
/// start of what came back, in order, as one line; the newest 4000 characters
pub fn rain_text(raw: &str) -> String {
    let mut out: Vec<String> = vec![];
    for line in raw.lines().skip(1) {
        // (the first may be cut)
        let Ok(d) = serde_json::from_str::<Value>(line) else { continue };
        let c = d.get("message").filter(|m| m.is_object()).and_then(|m| m.get("content"));
        let items: Vec<Value> = match c {
            Some(Value::String(s)) => vec![serde_json::json!({"type": "text", "text": s})],
            Some(Value::Array(a)) => a.clone(),
            _ => vec![],
        };
        for x in items.iter().filter(|x| x.is_object()) {
            match x.get("type").and_then(Value::as_str) {
                Some("text") => out.push(first(&parse::untag(x.get("text").and_then(Value::as_str).unwrap_or("")), 1500)),
                Some("tool_use") => {
                    let inp = x.get("input");
                    let mut parts = vec![util::text(x.get("name"))];
                    for k in ["command", "file_path", "pattern", "path", "url", "description"] {
                        if let Some(v) = inp.and_then(|i| i.get(k)).filter(|v| util::truthy(v)) {
                            parts.push(first(&util::text(Some(v)), 300));
                        }
                    }
                    out.push(parts.join(" "));
                }
                Some("tool_result") => {
                    let r = match x.get("content") {
                        Some(Value::Array(a)) => a.iter().filter(|y| y.is_object()).map(|y| y.get("text").and_then(Value::as_str).unwrap_or("")).collect::<Vec<_>>().join(" "),
                        r => util::text(r),
                    };
                    out.push(first(&r, 240));
                }
                _ => {}
            }
        }
    }
    // every run of white space one space (re.sub(r"\s+", " ")), then the newest 4000
    let mut one: Vec<char> = vec![];
    for c in out.join(" ").chars() {
        if !c.is_whitespace() {
            one.push(c);
        } else if one.last() != Some(&' ') {
            one.push(' ');
        }
    }
    one[one.len().saturating_sub(4000)..].iter().collect()
}

/// the demo's rain: made up, the same for a bot each time
fn demo_stream(b: &Bot) -> String {
    const WORDS: [&str; 17] = [
        "Reading", "src/room.js", "web/server.py", "Running the tests", "14 passed", "git diff --stat", "Edited", "README.md",
        "the layout keeps its desks", "fixed the off-by-one in the seat rows", "cargo test", "all green", "Committed", "npm run build",
        "the fold of worktrees", "TypeError at line 212", "tracing the poll",
    ];
    // its own dice, from its id (fnv-1a, then a xorshift)
    let mut s = b.id.bytes().fold(0xcbf29ce484222325u64, |h, c| (h ^ c as u64).wrapping_mul(0x100000001b3)) | 1;
    let mut roll = |n: usize| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s % n as u64) as usize
    };
    (0..160)
        .map(|_| match roll(WORDS.len() + 2) {
            i if i < WORDS.len() => WORDS[i],
            i if i == WORDS.len() => b.name.as_str(),
            _ => "looks right now",
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transcript_as_rain() {
        let raw = [
            r#"{"cut off half way"#,
            r#"{"type":"user","message":{"role":"user","content":"make the <system-reminder>secret</system-reminder>rain   fall"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":"On it."},{"type":"tool_use","name":"Bash","input":{"command":"cargo test","description":"the tests","timeout":5}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":[{"type":"text","text":"14 passed\n\nok"}]}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"plain"}]}}"#,
            "not json",
        ]
        .join("\n");
        assert_eq!(rain_text(&raw), "make the rain fall On it. Bash cargo test the tests 14 passed ok plain");
        // 1500 characters of what was said, the newest 4000 of them all
        let said = |c: &str| r#"{"message":{"content":"ab"}}"#.replace("ab", &c.repeat(2000));
        let long = format!("x\n{}\n{}\n{}", said("a"), said("é"), said("z"));
        let t = rain_text(&long);
        assert_eq!(t.chars().count(), 4000);
        assert!(t.starts_with("aaa") && t.contains(&format!("a {} z", "é".repeat(1500))) && t.ends_with("zzz"));
        assert_eq!(rain_text(""), "");
    }

    #[test]
    fn the_demo_rains_the_same_each_time() {
        let b = Bot { id: "demo0003".into(), name: "kite".into(), ..Bot::default() };
        assert_eq!(demo_stream(&b), demo_stream(&b));
        assert!(demo_stream(&b).split(' ').count() >= 160);
        assert_ne!(demo_stream(&b), demo_stream(&Bot { id: "demo0004".into(), ..b.clone() }));
    }
}
