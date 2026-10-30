//! The operator's line (matrix.js's `run`, `complete`): its commands, what they find, what they print. What a command
//! does to the window or the hub is the view's (mod.rs); here are only its words, so the tests read them.

use super::words::{base, desk_of, lost, machine_name, project_of, state_of, tilde, word_of, St};
use burrow_model::{Bot, Limit, State};

/// how a printed line looks: the line's green, an echo (dim), trouble (red), what wants you (white)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Plain,
    Echo,
    Err,
    Hot,
}

pub type Lines = Vec<(String, Class)>;

/// what there is to type: the command, what it takes, what it does
pub const COMMANDS: [(&str, &str, &str); 15] = [
    ("help", "", "this"),
    ("ls", "[what]", "every agent, by project; or those whose name, project or state has it"),
    ("jack", "<agent>", "jack in: its terminal (a click on its lane does the same)"),
    ("exit", "", "out of the terminal; the session goes on"),
    ("rabbit", "", "follow the white rabbit: the next agent that needs you"),
    ("load", "<project|~/folder>[@machine]", "a new agent there, and in"),
    ("tell", "<agent> <message>", "type a message into an agent's session"),
    ("reset", "<agent>", "/clear an idle agent: it forgets its conversation"),
    ("stop", "<agent>", "stop its process; it stays, idle"),
    ("delete", "<agent>", "stop it and take it out for good (asks first; its transcript stays for --resume)"),
    ("shell", "<project|~/folder>[@machine]", "your own shell there"),
    ("limits", "", "what is left of the five hours and the week"),
    ("nodes", "", "the machines"),
    ("room", "", "back to the 16-bit room (or esc on an empty line)"),
    ("cls", "", "clear these lines"),
];

const ALIAS: [(&str, &str); 11] = [
    ("?", "help"),
    ("agents", "ls"),
    ("j", "jack"),
    ("in", "jack"),
    ("open", "jack"),
    ("new", "load"),
    ("spawn", "load"),
    ("out", "exit"),
    ("q", "exit"),
    ("rm", "delete"),
    ("clear", "reset"),
];

fn alias(verb: &str) -> String {
    ALIAS.iter().find(|(a, _)| *a == verb).map(|(_, v)| v.to_string()).unwrap_or_else(|| verb.to_string())
}

/// a line as a command and what follows it: `jack in x` is `jack x`, `jack out` is `exit`
pub fn parse(raw: &str) -> (String, Vec<String>) {
    let mut words = raw.split_whitespace().map(String::from);
    let mut verb = words.next().unwrap_or_default().to_lowercase();
    let mut rest: Vec<String> = words.collect();
    if verb == "jack" && rest.first().is_some_and(|w| w == "in") {
        rest.remove(0);
    }
    if verb == "jack" && rest.first().is_some_and(|w| w == "out") {
        verb = "exit".into();
        rest.clear();
    }
    (alias(&verb), rest)
}

/// a JS padEnd: at least `n` characters, spaces after
fn pad(s: &str, n: usize) -> String {
    format!("{s}{}", " ".repeat(n.saturating_sub(s.chars().count())))
}

/// a session by what was typed: its name, a unique start of it, or of its id
pub fn find<'a>(q: &str, st: &'a State) -> Result<&'a Bot, String> {
    if q.is_empty() {
        return Err("whom?".into());
    }
    let s = q.to_lowercase();
    let exact: Vec<&Bot> = st.bots.iter().filter(|b| b.name.to_lowercase() == s || b.id == q).collect();
    if exact.len() == 1 {
        return Ok(exact[0]);
    }
    let some: Vec<&Bot> = if exact.is_empty() { st.bots.iter().filter(|b| b.name.to_lowercase().starts_with(&s) || b.id.starts_with(q)).collect() } else { exact };
    match some.len() {
        1 => Ok(some[0]),
        0 => Err(format!("no agent called {q}")),
        n => {
            let names: Vec<String> = some.iter().take(8).map(|b| format!("{}{}", b.name, if b.m.is_empty() { String::new() } else { format!("@{}", machine_name(&b.m, st)) })).collect();
            Err(format!("which one: {}{}", names.join(", "), if n > 8 { ", …" } else { "" }))
        }
    }
}

/// a project (a desk) by its name, or a folder (~/x, /x), with `@machine` after it for another machine: (machine, folder)
pub fn place(q: &str, st: &State) -> Result<(String, String), String> {
    if q.is_empty() {
        return Err("where? a project, or a folder".into());
    }
    let mut parts = q.split('@');
    let what = parts.next().unwrap_or("");
    let mname = parts.next().unwrap_or("");
    let m = if mname.is_empty() { None } else { st.machines.iter().find(|x| x.name.to_lowercase() == mname.to_lowercase() || x.id == mname) };
    if !mname.is_empty() && m.is_none() {
        return Err(format!("no machine called {mname}"));
    }
    if what.starts_with('~') || what.starts_with('/') {
        // (a / at its end goes, unless it is all there is)
        let mut dir = what.to_string();
        while dir.len() > 1 && dir.ends_with('/') {
            dir.pop();
        }
        return Ok((m.map(|m| m.id.clone()).unwrap_or_default(), dir));
    }
    let name = |d: &burrow_model::Desk| if d.name.is_empty() { base(&d.dir) } else { d.name.clone() };
    let ds: Vec<&burrow_model::Desk> = st.desks.iter().filter(|d| m.is_none_or(|m| d.m == m.id) && name(d).to_lowercase() == what.to_lowercase()).collect();
    match ds.len() {
        1 => Ok((ds[0].m.clone(), ds[0].dir.clone())),
        0 => Err(format!("no project called {what} (a folder starts with ~ or /)")),
        _ => Err(format!("which one: {}", ds.iter().map(|d| format!("{}@{}", name(d), machine_name(&d.m, st))).collect::<Vec<_>>().join(", "))),
    }
}

/// tab: the command, then an agent's or a project's name. What the line becomes, and the choices when there are some
pub fn complete(v: &str, st: &State) -> Option<(String, Option<String>)> {
    // (split as JS splits on /\s+/: a run of spaces is one, one at either end leaves an empty part)
    let mut parts = vec![String::new()];
    let mut space = false;
    for c in v.chars() {
        if !c.is_whitespace() {
            parts.last_mut().unwrap().push(c);
        } else if !space {
            parts.push(String::new());
        }
        space = c.is_whitespace();
    }
    let last = parts.last().cloned().unwrap_or_default().to_lowercase();
    let pool: Vec<String> = if parts.len() == 1 {
        COMMANDS.iter().map(|c| c.0.to_string()).chain(std::iter::once("jack in".to_string())).collect()
    } else {
        let verb = alias(&parts[0].to_lowercase());
        if verb == "tell" && parts.len() > 2 {
            return None;
        }
        if verb == "load" || verb == "shell" {
            let mut names: Vec<String> = vec![];
            for d in &st.desks {
                let n = if d.name.is_empty() { base(&d.dir) } else { d.name.clone() };
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            names
        } else {
            st.bots.iter().map(|b| b.name.clone()).collect()
        }
    };
    let hits: Vec<&String> = pool.iter().filter(|x| x.to_lowercase().starts_with(&last)).collect();
    let first = hits.first()?;
    let mut pre: String = first.to_string();
    for h in &hits {
        while !h.to_lowercase().starts_with(&pre.to_lowercase()) {
            pre.pop();
        }
    }
    let n = parts.len();
    parts[n - 1] = if hits.len() == 1 { format!("{first} ") } else { pre };
    let hint = (hits.len() > 1).then(|| hits.iter().take(12).map(|h| h.as_str()).collect::<Vec<_>>().join("  "));
    Some((parts.join(" "), hint))
}

pub fn help() -> Lines {
    let mut out: Lines = COMMANDS.iter().map(|(k, args, say)| (format!("{} {say}", pad(&format!("{k} {args}"), 38)), Class::Plain)).collect();
    out.push(("tab completes, ↑ ↓ for what was typed before; a click on a lane jacks in".into(), Class::Echo));
    out
}

/// every agent by project; with `q`, those whose name, project, state or machine has it
pub fn ls(q: &str, st: &State) -> Lines {
    let q = q.to_lowercase();
    let bs: Vec<&Bot> = st
        .bots
        .iter()
        .filter(|b| q.is_empty() || [b.name.clone(), project_of(b, st), word_of(b, st), machine_name(&b.m, st)].iter().any(|x| x.to_lowercase().contains(&q)))
        .collect();
    if bs.is_empty() {
        return vec![((if q.is_empty() { "no agents yet: load <project>" } else { "none" }).into(), Class::Plain)];
    }
    let mut units: Vec<&str> = vec![];
    for b in &bs {
        if !units.contains(&b.unit.as_str()) {
            units.push(&b.unit);
        }
    }
    let mut out = vec![];
    for u in units {
        let g: Vec<&&Bot> = bs.iter().filter(|b| b.unit == u).collect();
        let b0 = g[0];
        let dir = desk_of(u, st).map(|d| d.dir.clone()).unwrap_or_else(|| b0.cwd.clone());
        let on = if b0.m.is_empty() { String::new() } else { format!(" @{}", machine_name(&b0.m, st)) };
        out.push((format!("{}{on}  {}", project_of(b0, st), tilde(&dir)), Class::Echo));
        for b in g {
            let line = format!("  {} {} {}", pad(&b.name, 20), pad(&word_of(b, st), 18), b.detail);
            out.push((line.trim_end().to_string(), if state_of(b, st) == St::Need { Class::Hot } else { Class::Plain }));
        }
    }
    out
}

pub fn limits(st: &State) -> Lines {
    let bar = |x: &Option<Limit>, what: &str| match x {
        Some(x) => {
            let left = 100 - x.used;
            let hashes = "#".repeat(((left as f64 / 5.) + 0.5).floor().max(0.) as usize);
            format!("{} [{}] {left}% left{}", pad(what, 10), pad(&hashes, 20).replace(' ', "."), if x.resets.is_empty() { String::new() } else { format!(", new at {}", x.resets) })
        }
        None => format!("{} not known yet", pad(what, 10)),
    };
    vec![(bar(&st.usage.session, "five hours"), Class::Plain), (bar(&st.usage.week, "the week"), Class::Plain)]
}

pub fn nodes(st: &State) -> Lines {
    st.machines
        .iter()
        .map(|m| {
            let name = if m.name.is_empty() { "this computer" } else { &m.name };
            let n = st.bots.iter().filter(|b| b.m == m.id).count();
            let how = match &m.error {
                Some(e) => format!("signal lost: {e}"),
                None => "online".into(),
            };
            (format!("{} {how}  {n} agents", pad(name, 20)), if m.error.is_some() { Class::Err } else { Class::Plain })
        })
        .collect()
}

/// the header's counts: agents, working, needing you, machines in reach of all
pub fn counts(st: &State) -> (usize, usize, usize, usize, usize) {
    let n = |f: &dyn Fn(&Bot) -> bool| st.bots.iter().filter(|b| f(b)).count();
    (
        st.bots.len(),
        n(&|b| b.busy && !lost(b, st)),
        n(&|b| b.waiting && !lost(b, st)),
        st.machines.iter().filter(|m| m.error.is_none()).count(),
        st.machines.len(),
    )
}

/// what is left of a limit, for the header: 36%, or ? not known yet
pub fn left(x: &Option<Limit>) -> String {
    x.as_ref().map(|x| format!("{}%", 100 - x.used)).unwrap_or_else(|| "?".into())
}

/// the next that needs you after the one jacked into (follow the white rabbit)
pub fn rabbit<'a>(st: &'a State, cur: Option<&str>) -> Option<&'a Bot> {
    let need: Vec<&Bot> = st.bots.iter().filter(|b| b.waiting && !lost(b, st)).collect();
    if need.is_empty() {
        return None;
    }
    let i = cur.and_then(|c| need.iter().position(|b| b.id == c)).map(|i| i as i64).unwrap_or(-1);
    Some(need[((i + 1) as usize) % need.len()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use burrow_model::{Desk, Machine, Usage};

    fn st() -> State {
        let b = |id: &str, name: &str, m: &str, unit: &str| Bot { id: id.into(), name: name.into(), m: m.into(), unit: unit.into(), cwd: unit.trim_start_matches("lab:").into(), alive: true, ..Default::default() };
        let mut s = State {
            machines: vec![Machine { id: "".into(), name: "mac".into(), ..Default::default() }, Machine { id: "lab".into(), name: "lab".into(), ..Default::default() }],
            desks: vec![
                Desk { unit: "/Users/r/kite".into(), dir: "/Users/r/kite".into(), ..Default::default() },
                Desk { unit: "lab:/home/r/kite".into(), m: "lab".into(), dir: "/home/r/kite".into(), ..Default::default() },
                Desk { unit: "/Users/r/b".into(), dir: "/Users/r/b".into(), name: "Burrow".into(), ..Default::default() },
            ],
            bots: vec![b("a1", "kite", "", "/Users/r/kite"), b("a2", "kite-two", "", "/Users/r/kite"), b("c3", "talk-ui", "", "/Users/r/b"), b("l4", "kite", "lab", "lab:/home/r/kite")],
            usage: Usage { session: Some(Limit { used: 64, resets: "3:20pm".into(), ..Default::default() }), ..Default::default() },
            ..Default::default()
        };
        s.bots[2].waiting = true;
        s.bots[3].detail = "the string snapped".into();
        s
    }

    #[test]
    fn lines_as_commands() {
        assert_eq!(parse("  Jack in  kite "), ("jack".into(), vec!["kite".into()]));
        assert_eq!(parse("jack out"), ("exit".into(), vec![]));
        assert_eq!(parse("rm x"), ("delete".into(), vec!["x".into()]));
        assert_eq!(parse("tell kite go on then"), ("tell".into(), vec!["kite".into(), "go".into(), "on".into(), "then".into()]));
    }

    #[test]
    fn whom_and_where() {
        let s = st();
        assert_eq!(find("", &s).unwrap_err(), "whom?");
        assert_eq!(find("talk", &s).unwrap().id, "c3");
        assert_eq!(find("c3", &s).unwrap().id, "c3");
        assert_eq!(find("kite", &s).unwrap_err(), "which one: kite, kite@lab");
        assert_eq!(find("kite-", &s).unwrap().id, "a2");
        assert_eq!(find("zz", &s).unwrap_err(), "no agent called zz");
        assert_eq!(place("burrow", &s).unwrap(), ("".into(), "/Users/r/b".into()));
        assert_eq!(place("kite@lab", &s).unwrap(), ("lab".into(), "/home/r/kite".into()));
        assert_eq!(place("kite", &s).unwrap_err(), "which one: kite@mac, kite@lab");
        assert_eq!(place("~/x/@LAB", &s).unwrap(), ("lab".into(), "~/x".into()));
        assert_eq!(place("/", &s).unwrap(), ("".into(), "/".into()));
        assert_eq!(place("x@moon", &s).unwrap_err(), "no machine called moon");
        assert_eq!(place("x", &s).unwrap_err(), "no project called x (a folder starts with ~ or /)");
    }

    #[test]
    fn tab_completes() {
        let s = st();
        assert_eq!(complete("he", &s), Some(("help ".into(), None)));
        assert_eq!(complete("ja", &s), Some(("jack".into(), Some("jack  jack in".into()))));
        assert_eq!(complete("jack ta", &s), Some(("jack talk-ui ".into(), None)));
        assert_eq!(complete("jack ki", &s), Some(("jack kite".into(), Some("kite  kite-two  kite".into()))));
        assert_eq!(complete("load b", &s), Some(("load Burrow ".into(), None)));
        assert_eq!(complete("tell kite go", &s), None);
        assert_eq!(complete("zz", &s), None);
    }

    #[test]
    fn what_it_prints() {
        let s = st();
        let ls = ls("", &s);
        assert_eq!(ls[0], ("kite  ~/kite".into(), Class::Echo));
        assert_eq!(ls[1].0, format!("  kite{}IDLE", " ".repeat(17)));
        assert!(ls.iter().any(|l| l.1 == Class::Hot && l.0.contains("talk-ui") && l.0.contains("NEEDS YOU")));
        assert!(ls.iter().any(|l| l.0 == "kite @lab  ~/kite" && l.1 == Class::Echo));
        assert!(ls.iter().any(|l| l.0.ends_with(&format!("IDLE{}the string snapped", " ".repeat(15)))));
        assert_eq!(super::ls("needs", &s).len(), 2);
        assert_eq!(super::ls("nothing like it", &s), vec![("none".to_string(), Class::Plain)]);
        let lim = limits(&s);
        assert_eq!(lim[0].0, "five hours [#######.............] 36% left, new at 3:20pm");
        assert_eq!(lim[1].0, "the week   not known yet");
        assert_eq!(help()[2].0, format!("{} jack in: its terminal (a click on its lane does the same)", pad("jack <agent>", 38)));
        assert_eq!(nodes(&s)[1].0, format!("{} online  1 agents", pad("lab", 20)));
        assert_eq!(counts(&s), (4, 0, 1, 2, 2));
        assert_eq!((left(&s.usage.session), left(&s.usage.week)), ("36%".into(), "?".into()));
        assert_eq!(rabbit(&s, None).unwrap().id, "c3");
        assert_eq!(rabbit(&s, Some("c3")).unwrap().id, "c3");
    }
}
