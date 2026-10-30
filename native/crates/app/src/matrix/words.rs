//! What a session is, in words (matrix.js's top): its state, its project, its machine, its folder, how long ago.

use burrow_model::{Bot, Desk, State};

/// a session's state as the rain shows it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum St {
    Work,
    Need,
    Idle,
    /// idle, its process off
    Off,
    Over,
    /// its machine out of reach
    Lost,
}

pub fn lost(b: &Bot, st: &State) -> bool {
    st.machine(&b.m).is_some_and(|m| m.error.is_some())
}

pub fn state_of(b: &Bot, st: &State) -> St {
    if lost(b, st) {
        St::Lost
    } else if b.waiting {
        St::Need
    } else if b.busy {
        St::Work
    } else if !b.over.is_empty() {
        St::Over
    } else if !b.alive {
        St::Off
    } else {
        St::Idle
    }
}

/// WORKING, NEEDS YOU, …; one that is over says how (DONE, FAILED, STOPPED)
pub fn word_of(b: &Bot, st: &State) -> String {
    if !b.over.is_empty() && !lost(b, st) {
        return b.over.to_uppercase();
    }
    match state_of(b, st) {
        St::Work => "WORKING",
        St::Need => "NEEDS YOU",
        St::Idle => "IDLE",
        St::Off => "IDLE, PROCESS OFF",
        St::Over => "OVER",
        St::Lost => "SIGNAL LOST",
    }
    .into()
}

pub fn desk_of<'a>(unit: &str, st: &'a State) -> Option<&'a Desk> {
    st.desks.iter().find(|d| d.unit == unit)
}

/// a folder's last part; the home folder is "home", as the room calls it
pub fn base(d: &str) -> String {
    let n: Vec<&str> = d.trim_end_matches('/').split('/').collect();
    if d == "~" || d == "/root" || (n.len() == 3 && (n[1] == "Users" || n[1] == "home")) {
        return "home".into();
    }
    match n.last() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ if !d.is_empty() => d.to_string(),
        _ => "~".into(),
    }
}

/// /Users/x/... as ~/...
pub fn tilde(d: &str) -> String {
    for top in ["/Users/", "/home/"] {
        if let Some(rest) = d.strip_prefix(top) {
            let user = rest.split('/').next().unwrap_or("");
            if !user.is_empty() {
                return format!("~{}", &rest[user.len()..]);
            }
        }
    }
    d.to_string()
}

/// its desk's name, or its folder's
pub fn project_of(b: &Bot, st: &State) -> String {
    match desk_of(&b.unit, st) {
        Some(d) if !d.name.is_empty() => d.name.clone(),
        Some(d) => base(&d.dir),
        None => base(&b.cwd),
    }
}

pub fn machine_name(mid: &str, st: &State) -> String {
    st.machine(mid).map(|m| m.name.clone()).unwrap_or_else(|| mid.to_string())
}

/// how long ago a time (ms since the epoch) was: 40s, 12m, 3h, 2d
pub fn ago(ms: i64, now_ms: i64) -> String {
    let s = ((now_ms - ms) as f64 / 1000.).max(0.);
    let r = |x: f64| (x + 0.5).floor() as i64;
    if s < 90. {
        format!("{}s", r(s))
    } else if s < 5400. {
        format!("{}m", r(s / 60.))
    } else if s < 172800. {
        format!("{}h", r(s / 3600.))
    } else {
        format!("{}d", r(s / 86400.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burrow_model::Machine;

    #[test]
    fn words() {
        assert_eq!(base("/Users/r/projects/burrow/"), "burrow");
        assert_eq!(base("/Users/r"), "home");
        assert_eq!(base("~"), "home");
        assert_eq!(base("/"), "/");
        assert_eq!(tilde("/Users/r/projects/x"), "~/projects/x");
        assert_eq!(tilde("/home/r"), "~");
        assert_eq!(tilde("/srv/x"), "/srv/x");
        assert_eq!((ago(0, 30_000), ago(0, 600_000), ago(0, 7_200_000), ago(0, 400_000_000)), ("30s".into(), "10m".into(), "2h".into(), "5d".into()));
        let mut st = State { machines: vec![Machine { id: "lab".into(), name: "lab".into(), ..Default::default() }], ..Default::default() };
        let mut b = Bot { m: "lab".into(), over: "failed".into(), cwd: "/p/kite".into(), unit: "lab:/p/kite".into(), ..Default::default() };
        assert_eq!((word_of(&b, &st), project_of(&b, &st)), ("FAILED".into(), "kite".into()));
        b.over.clear();
        assert_eq!(word_of(&b, &st), "IDLE, PROCESS OFF");
        st.machines[0].error = Some("gone".into());
        assert_eq!((state_of(&b, &st), word_of(&b, &st)), (St::Lost, "SIGNAL LOST".into()));
    }
}
