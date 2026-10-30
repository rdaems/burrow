//! The printer's report, as text (web/index.html's `fillReport`): the office's statistics since the room
//! opened, the staff list and the limits, a little funny. The pixel font is proportional, so the paper is
//! measured in pixels: every line is cut to its width, columns padded with spaces to pixel stops.

use burrow_model::{Limit, State};
use burrow_room::world::Stat;
use std::collections::HashMap;

/// what the room counted (room.js `stats()`)
pub struct Stats<'a> {
    pub bots: &'a HashMap<String, Stat>,
    pub meetings: u32,
    /// how much of the canvas on the easel is painted, 0..1 (never which painting it is)
    pub art: Option<f64>,
    /// minutes since the room opened
    pub shift: f64,
}

/// the date and time it is printed: day, month, hours, minutes
pub type When = (u32, u32, u32, u32);

fn count(s: &Stat, k: &str) -> f64 {
    match k {
        "busy" => s.busy,
        "idle" => s.idle,
        "asleep" => s.asleep,
        "sent" => s.sent as f64,
        "got" => s.got as f64,
        k => s.acts.get(k).copied().unwrap_or(0) as f64,
    }
}

fn mins(sec: f64) -> String {
    if sec < 90. {
        format!("{} s", sec.round())
    } else if sec < 5400. {
        format!("{} min", (sec / 60.).round())
    } else {
        format!("{:.1} h", sec / 3600.)
    }
}

fn n(k: u64, one: &str) -> String {
    format!("{k} {one}{}", if k == 1 { "" } else { "s" })
}

fn left(x: &Option<Limit>) -> String {
    x.as_ref().map(|x| format!("{}%", (100 - x.used).max(0))).unwrap_or("?".into())
}

/// the report's lines for paper `wide` pixels wide, `px` measuring a line in the paper's font
pub fn lines(state: &State, st: &Stats, at: When, px: &dyn Fn(&str) -> f32, wide: f32) -> Vec<String> {
    let name = |id: &str| state.bots.iter().find(|b| b.id == id).map(|b| b.name.clone()).unwrap_or(id.to_string());
    let bots: Vec<_> = state.bots.iter().filter(|b| b.over.is_empty()).collect();
    let over: Vec<_> = state.bots.iter().filter(|b| !b.over.is_empty()).collect();
    let get = |id: &str, k: &str| st.bots.get(id).map(|s| count(s, k)).unwrap_or(0.);
    let sum = |k: &str| bots.iter().map(|b| get(&b.id, k)).sum::<f64>() as u64;
    let top = |k: &str| {
        let mut best: Option<(&str, f64)> = None;
        for b in &bots {
            let v = get(&b.id, k);
            if v > best.map(|x| x.1).unwrap_or(0.) {
                best = Some((&b.id, v));
            }
        }
        best
    };
    let sp = px(" ");
    let cut = |s: &str| -> String {
        if px(s) <= wide {
            return s.to_string();
        }
        let mut s = s.to_string();
        while !s.is_empty() && px(&format!("{s}-")) > wide {
            s.pop();
        }
        s + "-"
    };
    let pad_to = |s: String, at: f32| {
        let mut s = s;
        while px(&s) < at - sp / 2. {
            s.push(' ');
        }
        s
    };
    // columns at pixel stops; the right end flush with the margin
    let row = |cells: &[String]| {
        let stops = [wide * 0.36, wide * 0.6];
        let mut line = String::new();
        for (i, c) in cells.iter().enumerate() {
            line = if i + 1 < cells.len() { pad_to(line + c, stops.get(i).copied().unwrap_or(0.)) } else { line + c };
        }
        line
    };
    let two = |a: &str, b: &str| pad_to(format!("{a} "), wide - px(b)) + b;
    let (day, month, h, m) = at;
    let u = &state.usage;
    let shift = st.shift.round();
    let mut l = vec![
        two("BURROW OFFICE REPORT", &format!("{day:02}/{month:02} {h:02}:{m:02}")),
        format!("since {} ago, approximate", if shift < 2. { "a moment".to_string() } else { format!("{shift} min") }),
        String::new(),
    ];
    let working = bots.iter().filter(|b| b.busy).count();
    let waiting = bots.iter().filter(|b| b.waiting).count();
    l.push(format!("STAFF  {}, {working} working, {waiting} waiting", n(bots.len() as u64, "session")));
    for mc in &state.machines {
        for b in bots.iter().filter(|b| b.m == mc.id) {
            let what = if b.waiting { "needs you" } else if b.busy { "working" } else { "idle" };
            l.push(row(&[format!("  {}", b.name.chars().take(16).collect::<String>()), mc.name.chars().take(10).collect(), what.into()]));
        }
    }
    if bots.is_empty() {
        l.push("  nobody. the plants are thriving".into());
    }
    let done = over.iter().filter(|b| b.over == "done").count();
    let failed = over.iter().filter(|b| b.over == "failed").count();
    if !over.is_empty() {
        l.push(format!("  completed, idling about: {done} done, {failed} failed, {} stopped", over.len() - done - failed));
    }
    let (coffee, cooler, naps, prints) = (sum("coffee"), sum("cooler"), sum("sleep"), sum("print"));
    l.push(String::new());
    l.push("BREAK ROOM".into());
    l.push(if coffee > 0 { format!("  coffee: {}, the pot at {}", n(coffee, "cup"), left(&u.session)) } else { format!("  coffee: none fetched, the pot at {}", left(&u.session)) });
    l.push(if cooler > 0 { format!("  cooler: {}, topics undisclosed", n(cooler, "chat")) } else { "  cooler: no chats, the bottle is lonely".into() });
    l.push(if naps > 0 {
        let longest = top("asleep").map(|(id, s)| format!(", longest {} ({})", name(id), mins(s))).unwrap_or_default();
        format!("  naps: {naps}{longest}")
    } else {
        "  naps: none, batteries are a state of mind".into()
    });
    l.push(if prints > 0 { format!("  printed: {} by the bots, plus this one", n(prints, "page")) } else { "  printed: this page only".into() });
    l.push(String::new());
    l.push(format!("GAMES  ping-pong {}, chess {}", sum("pong"), sum("chess")));
    let present = top("present").map(|(id, _)| format!(", {} by the easel", name(id))).unwrap_or_default();
    l.push(format!("MEETINGS  {}", if st.meetings > 0 { format!("{} held{present}", st.meetings) } else { "none".into() }));
    l.push(format!(
        "EASEL     {}",
        match st.art {
            Some(k) if k >= 1. => "a canvas done".to_string(),
            Some(k) => format!("{}% painted", (k * 100.).round()),
            None => "bare".into(),
        }
    ));
    let beams = sum("sent");
    let talker = top("sent").map(|(id, _)| format!(", most by {}", name(id))).unwrap_or_default();
    l.push(format!("MESSAGES  {}", if beams > 0 { format!("{beams} beamed{talker}") } else { "none, everyone minds their desk".into() }));
    if let Some((id, s)) = top("busy").filter(|x| x.1 > 30.) {
        l.push(String::new());
        l.push(format!("EMPLOYEE OF THE SHIFT  {}, {} at the keys", name(id), mins(s)));
    }
    l.push(String::new());
    l.push(format!("LIMITS  5h {} left, week {} left", left(&u.session), left(&u.week)));
    l.push(String::new());
    l.push("recycled pixels, keep away from the fire".into());
    l.iter().map(|x| cut(x)).collect()
}

/// how far the printing is after `ms`: as a line printer, every line letter by letter (two every 12 ms), 90 ms
/// between lines. The line being printed, and how much of it
pub fn printed(lines: &[String], ms: f64) -> (usize, usize) {
    let mut t = ms;
    for (i, line) in lines.iter().enumerate() {
        let n = line.chars().count();
        let steps = n.div_ceil(2);
        let span = steps as f64 * 12.;
        if t < span {
            return (i, ((t / 12.).floor() as usize * 2).min(n));
        }
        t -= span;
        if t < 90. {
            return (i, n);
        }
        t -= 90.;
    }
    (lines.len(), 0)
}

/// the text on the sheet at that point
pub fn sheet(lines: &[String], at: (usize, usize)) -> String {
    let (i, j) = at;
    let mut s: Vec<String> = lines[..i.min(lines.len())].to_vec();
    if i < lines.len() {
        s.push(lines[i].chars().take(j).collect());
    }
    s.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use burrow_model::{Bot, Machine, Usage};

    fn state() -> State {
        State {
            machines: vec![Machine { id: "".into(), name: "mac".into(), ..Default::default() }],
            bots: vec![
                Bot { id: "a".into(), name: "ada".into(), busy: true, ..Default::default() },
                Bot { id: "b".into(), name: "bo".into(), ..Default::default() },
                Bot { id: "c".into(), name: "cy".into(), over: "done".into(), ..Default::default() },
            ],
            usage: Usage { session: Some(Limit { label: "s".into(), used: 37, resets: "".into() }), ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn the_report_counts_and_jokes() {
        let mut stats = HashMap::new();
        let mut a = Stat { busy: 400., ..Default::default() };
        a.acts.insert("coffee", 2);
        a.sent = 3;
        stats.insert("a".to_string(), a);
        let mut b = Stat::default();
        b.acts.insert("coffee", 1);
        b.acts.insert("sleep", 1);
        b.asleep = 50.;
        stats.insert("b".to_string(), b);
        let st = Stats { bots: &stats, meetings: 0, art: Some(0.25), shift: 12. };
        let px = |s: &str| s.chars().count() as f32 * 8.;
        let l = lines(&state(), &st, (3, 7, 9, 5), &px, 800.);
        assert!(l[0].starts_with("BURROW OFFICE REPORT") && l[0].ends_with("03/07 09:05"), "{}", l[0]);
        assert_eq!(px(&l[0]), 800., "the date flush right");
        assert_eq!(l[1], "since 12 min ago, approximate");
        assert!(l.contains(&"STAFF  2 sessions, 1 working, 0 waiting".to_string()));
        assert!(l.iter().any(|x| x.starts_with("  ada") && x.ends_with("working")));
        assert!(l.contains(&"  completed, idling about: 1 done, 0 failed, 0 stopped".to_string()));
        assert!(l.contains(&"  coffee: 3 cups, the pot at 63%".to_string()));
        assert!(l.contains(&"  naps: 1, longest bo (50 s)".to_string()));
        assert!(l.contains(&"MESSAGES  3 beamed, most by ada".to_string()));
        assert!(l.contains(&"MEETINGS  none".to_string()));
        assert!(l.contains(&"EASEL     25% painted".to_string()), "never which painting");
        let done = Stats { bots: &stats, meetings: 0, art: Some(1.), shift: 12. };
        assert!(lines(&state(), &done, (3, 7, 9, 5), &px, 800.).contains(&"EASEL     a canvas done".to_string()));
        assert!(l.contains(&"EMPLOYEE OF THE SHIFT  ada, 7 min at the keys".to_string()));
        assert!(l.contains(&"LIMITS  5h 63% left, week ? left".to_string()));
    }

    #[test]
    fn lines_are_cut_to_the_paper() {
        let stats = HashMap::new();
        let st = Stats { bots: &stats, meetings: 0, art: None, shift: 0. };
        let px = |s: &str| s.chars().count() as f32 * 10.;
        for line in lines(&state(), &st, (1, 1, 0, 0), &px, 240.) {
            assert!(px(&line) <= 240., "{line}");
        }
    }

    #[test]
    fn it_prints_letter_by_letter() {
        let l = vec!["abcd".to_string(), "xy".to_string()];
        assert_eq!(printed(&l, 0.), (0, 0));
        assert_eq!(printed(&l, 13.), (0, 2));
        assert_eq!(printed(&l, 24. + 50.), (0, 4), "the line done, the pause");
        assert_eq!(printed(&l, 24. + 90. + 1.), (1, 0));
        assert_eq!(printed(&l, 10_000.), (2, 0));
        assert_eq!(sheet(&l, (1, 1)), "abcd\nx");
        assert_eq!(sheet(&l, (2, 0)), "abcd\nxy");
    }
}
