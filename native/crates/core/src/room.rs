//! The room: the desks you keep and where the furniture stands, in ~/.config/burrow/room.json. Kept as
//! the JSON object it is (a key this port does not know survives a save: the web app reads the same file).
//! A desk is a unit: a directory here, or "<machine id>:<directory>" on another machine.

use crate::config::Config;
use crate::util::{kind_of, pair_seed, truthy, unit};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::LazyLock;

pub type Room = Map<String, Value>;

/// The furniture: what stands in the room, as keys; a key is its kind and a number (plant0, mat2;
/// the first room's pieces have no number). There can be any number of a kind. The closet in the
/// room adds a piece, the fireplace burns one; the fire, the closet and the switchboard themselves stay.
pub const FURNITURE: &[&str] = &[
    "meet", "pong", "chess", "coffee", "cooler", "printer", "armchair", "plant0", "plant1", "board", "window", "clock", "fire", "closet",
    "settings0", "manager0", "booth0", "neuralyzer0",
];
// (a new machine is a door taken from the closet and hung on the wall; the doors themselves are no furniture:
// one a machine in Claude Code's sshConfigs, their places in the layout as door:<machine id>)
pub const FIXED: &[&str] = &["fire", "closet", "settings", "clock"];
pub const KINDS: &[&str] = &[
    "meet", "pong", "chess", "coffee", "cooler", "printer", "armchair", "plant", "board", "window", "pair", "manager", "booth", "neuralyzer",
    "doghouse", "radio", "files",
];

/// Burrow's own settings: the switchboard on the wall, a switch each; kept in room.json `settings`
///   rc        a bot started here gets --remote-control: the same session is in the Claude app and at claude.ai/code
///   yolo      a bot started here gets --dangerously-skip-permissions: it never asks, it just does
///   hardcore  the booth is a gun closet: she clears them out with a shotgun, and a click on it gives you a crosshair
///   sound     the room makes its sounds while a radio stands in it (no switch: the radio is the switch)
///   files     FILES on the terminals' bands while a filing cabinet stands in the room (the cabinet is the switch)
pub const OPTIONS: &[(&str, bool)] = &[("rc", true), ("yolo", false), ("hardcore", false), ("sound", true), ("files", true)];

/// the switchboard's settings, the room's over the defaults (the ones the model has no field for in `other`)
pub fn options(room: &Room) -> burrow_model::Settings {
    let get = |k: &str| {
        let d = OPTIONS.iter().find(|(n, _)| *n == k).unwrap().1;
        room.get("settings").and_then(|s| s.get(k)).map(truthy).unwrap_or(d)
    };
    let other = OPTIONS.iter().filter(|(k, _)| !["rc", "yolo", "hardcore", "sound"].contains(k)).map(|(k, _)| (k.to_string(), json!(get(k)))).collect();
    burrow_model::Settings { rc: get("rc"), yolo: get("yolo"), hardcore: get("hardcore"), sound: get("sound"), other }
}

/// your name and face: room.json's, the login name and a face from it when none is picked
pub fn you_of(room: &Room, user: &str) -> (String, String) {
    let you = room.get("you");
    let s = |k: &str| you.and_then(|y| y.get(k)).map(|v| crate::util::text(Some(v))).unwrap_or_default();
    let name = s("name").trim().to_string();
    let name = if name.is_empty() { user.to_string() } else { name };
    let mut seed = pair_seed(&s("seed"));
    if seed.is_empty() {
        seed = pair_seed(&name);
    }
    if seed.is_empty() {
        seed = "you".into();
    }
    (name, seed)
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn furniture(room: &Room) -> Vec<String> {
    strs(room.get("furniture"))
}

pub fn desks(room: &Room) -> Vec<String> {
    strs(room.get("desks"))
}

/// a key of the room as an object, made one if it is not
pub fn obj<'a>(room: &'a mut Room, key: &str) -> &'a mut Map<String, Value> {
    if !room.get(key).is_some_and(Value::is_object) {
        room.insert(key.into(), Value::Object(Map::new()));
    }
    room.get_mut(key).unwrap().as_object_mut().unwrap()
}

pub fn layout(room: &mut Room) -> &mut Map<String, Value> {
    obj(room, "layout")
}

pub fn set_furniture(room: &mut Room, fs: Vec<String>) {
    room.insert("furniture".into(), json!(fs));
}

fn v(room: &Room) -> i64 {
    room.get("v").and_then(Value::as_i64).unwrap_or(1)
}

static SEED_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9-]+").unwrap());

/// a room from before the furniture list gets the default pieces; the project cabinet (add) became
/// furniture, and the whiteboard came off the wall (its place on the wall is dropped: it gets one on the
/// floor); a room from before the bench gets one (later gone again); the charging mats are gone (a bot
/// rolls out its own to sleep on); a room from before the switchboard gets one; and so on, to v 13: the
/// receptions, each with the office manager hired at it
pub fn furnish(mut room: Room) -> Room {
    if !room.contains_key("furniture") {
        set_furniture(&mut room, FURNITURE.iter().map(|s| s.to_string()).collect());
    }
    layout(&mut room).shift_remove("mats");
    let has = |fs: &[String], kind: &str| fs.iter().any(|k| kind_of(k) == kind);
    let drop_layout = |room: &mut Room, kind: &str| layout(room).retain(|k, _| kind_of(k) != kind);
    let mut fs = furniture(&room);
    if v(&room) < 2 {
        if !fs.iter().any(|k| k == "add") {
            fs.push("add".into());
        }
        let low = room.get("layout").and_then(|l| l.get("board")).filter(|b| truthy(b)).and_then(|b| b.get(1)).and_then(Value::as_f64).is_some_and(|y| y < 30.0);
        if low {
            layout(&mut room).shift_remove("board");
        }
        room.insert("v".into(), json!(2));
    }
    if v(&room) < 3 {
        if !has(&fs, "bench") {
            fs.push("bench0".into());
        }
        room.insert("v".into(), json!(3));
    }
    if v(&room) < 4 {
        fs.retain(|k| kind_of(k) != "mat");
        drop_layout(&mut room, "mat");
        room.insert("v".into(), json!(4));
    }
    if v(&room) < 5 {
        if !has(&fs, "settings") {
            fs.push("settings0".into());
        }
        room.insert("v".into(), json!(5));
    }
    if v(&room) < 6 {
        // your face's seed in pair's form (name:7 was burrow's own): the same face there
        if let Some(seed) = room.get("you").and_then(|y| y.get("seed")).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from)
            && pair_seed(&seed) != seed {
                obj(&mut room, "you").insert("seed".into(), json!(pair_seed(&SEED_RUN.replace_all(&seed, "-"))));
            }
        room.insert("v".into(), json!(6));
    }
    if v(&room) < 8 {
        // the office manager keeps the notes on the sessions (a board on the wall before, v 7)
        fs.retain(|k| kind_of(k) != "notes");
        drop_layout(&mut room, "notes");
        if !has(&fs, "manager") {
            fs.push("manager0".into());
        }
        room.insert("v".into(), json!(8));
    }
    if v(&room) < 9 {
        // the booth (a bot in it is deleted) and the neuralyzer (/clear), once
        for k in ["booth", "neuralyzer"] {
            if !has(&fs, k) {
                fs.push(format!("{k}0"));
            }
        }
        room.insert("v".into(), json!(9));
    }
    if v(&room) < 10 {
        room.insert("v".into(), json!(10));
    }
    if v(&room) < 11 {
        // no empty door frame on the wall any more: a new door comes from the closet
        fs.retain(|k| kind_of(k) != "frame");
        drop_layout(&mut room, "frame");
        room.insert("v".into(), json!(11));
    }
    if v(&room) < 12 {
        // (the 3d goggles came here for a day)
        room.insert("v".into(), json!(12));
    }
    // the receptions: each has the office manager hired at it. One office manager of the first round of styles
    // (`manager`) is the first booth's; a room from before the styles has Linda (the notes) at its booth. A booth
    // with nobody hired (put down by a page from before hiring) was never anyone's: it goes
    let booths: Vec<String> = fs.iter().filter(|k| kind_of(k) == "manager").cloned().collect();
    obj(&mut room, "managers");
    let old = room.get("manager").filter(|m| truthy(m)).cloned();
    if let Some(old) = old {
        if let Some(b) = booths.first() {
            obj(&mut room, "managers").insert(b.clone(), old);
        }
        room.shift_remove("manager");
    } else if !booths.is_empty() && obj(&mut room, "managers").is_empty() && v(&room) < 13 {
        obj(&mut room, "managers").insert(booths[0].clone(), json!({"style": "notes"}));
    }
    for k in &booths {
        if !obj(&mut room, "managers").contains_key(k) {
            if let Some(i) = fs.iter().position(|x| x == k) {
                fs.remove(i);
            }
            layout(&mut room).shift_remove(k);
        }
    }
    obj(&mut room, "managers").retain(|k, _| fs.contains(k));
    let v13 = v(&room).max(13);
    room.insert("v".into(), json!(v13));
    // the 3d goggles: the first person is the gun closet's only; the bench: every session is at its desk; the
    // project cabinet: a desk comes out of the closet
    fs.retain(|k| kind_of(k) != "goggles" && kind_of(k) != "bench" && kind_of(k) != "add");
    drop_layout(&mut room, "bench");
    drop_layout(&mut room, "add");
    if !has(&fs, "clock") {
        // the clock is always there (it can't be burnt any more)
        fs.push("clock".into());
    }
    drop_layout(&mut room, "goggles");
    set_furniture(&mut room, fs);
    room
}

fn base(file: Value) -> Option<Room> {
    let Value::Object(file) = file else { return None };
    let mut room = Room::new();
    room.insert("desks".into(), json!([]));
    room.insert("layout".into(), json!({}));
    room.extend(file);
    Some(room)
}

/// the room as room.json has it; the first time, the desks and layout of the Rust app's rooms.json, one
/// room per machine (not the desks of worktrees: those sit at their repo's desk now)
pub fn load_room(cfg: &Config) -> Room {
    if cfg.demo {
        // made up, or a copy of a real room.json to try things on (never saved)
        if let Some(r) = cfg.demo_room.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).and_then(base) {
            return furnish(r);
        }
        let mut r = Room::new();
        r.insert("desks".into(), json!([format!("{}/projects/pebble", cfg.home_str())]));
        r.insert("layout".into(), json!({}));
        return furnish(r);
    }
    if let Some(r) = std::fs::read_to_string(cfg.room()).ok().and_then(|s| serde_json::from_str(&s).ok()).and_then(base)
        && r.get("layout").is_some_and(Value::is_object) && r.get("desks").is_some_and(Value::is_array) {
            return furnish(r);
        }
    let mut room = Room::new();
    let (mut ds, mut lay) = (vec![], Map::new());
    let settings: Value = std::fs::read_to_string(&cfg.claude_settings).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let hosts: Vec<(String, String)> = settings
        .get("sshConfigs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|c| c.get("id").is_some_and(truthy))
        .map(|c| (crate::util::text(c.get("sshHost")), id_str(c.get("id"))))
        .collect();
    if let Some(Value::Array(rs)) = std::fs::read_to_string(cfg.old_rooms()).ok().and_then(|s| serde_json::from_str(&s).ok()) {
        for r in rs {
            let ssh = crate::util::text(r.get("ssh"));
            let mid = if ssh.is_empty() { Some(String::new()) } else { hosts.iter().find(|(h, _)| *h == ssh).map(|(_, id)| id.clone()) };
            let Some(mid) = mid else { continue };
            ds.extend(strs(r.get("dirs")).into_iter().filter(|d| !d.contains("/.claude/worktrees/")).map(|d| json!(unit(&mid, &d))));
            if mid.is_empty()
                && let Some(Value::Object(l)) = r.get("layout") {
                    lay.extend(l.clone());
                }
        }
    }
    room.insert("desks".into(), Value::Array(ds));
    room.insert("layout".into(), Value::Object(lay));
    furnish(room)
}

/// a machine id as str() makes it (sshConfigs may have a number)
pub fn id_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => "None".into(),
        Some(v) => v.to_string(),
    }
}

pub fn save_room(cfg: &Config, room: &Room) {
    if cfg.demo {
        return;
    }
    let _ = std::fs::create_dir_all(&cfg.config);
    if let Ok(s) = serde_json::to_string_pretty(room) {
        let _ = std::fs::write(cfg.room(), s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(v: Value) -> Room {
        base(v).unwrap()
    }

    #[test]
    fn a_new_room() {
        let r = furnish(room(json!({})));
        assert_eq!(r["v"], 13);
        let fs = furniture(&r);
        assert_eq!(fs, FURNITURE); // bench0 came at v 3 and went again; the rest were there
        assert!(fs.contains(&"manager0".into()) && fs.contains(&"clock".into()));
        assert_eq!(r["managers"], json!({"manager0": {"style": "notes"}}));
    }

    #[test]
    fn a_first_room() {
        let r = furnish(room(json!({
            "desks": ["/a"], "layout": {"board": [40, 10], "mats": [1, 2], "mat0": [3, 4], "u:/a": [5, 6], "add": [7, 8]},
            "furniture": ["meet", "board", "mat0", "plant"], "you": {"seed": "rembert:15"}, "names": {"/a": "A"}
        })));
        let fs = furniture(&r);
        // (the project cabinet came at v 2 and went again: a desk comes out of the closet)
        assert_eq!(fs, ["meet", "board", "plant", "settings0", "manager0", "booth0", "neuralyzer0", "clock"]);
        let lay = r["layout"].as_object().unwrap();
        assert!(!lay.contains_key("board") && !lay.contains_key("mats") && !lay.contains_key("mat0") && !lay.contains_key("add") && lay.contains_key("u:/a"));
        assert_eq!(r["you"]["seed"], "rembert-15");
        assert_eq!(r["names"]["/a"], "A"); // what it doesn't touch stays
        assert_eq!(r["managers"]["manager0"]["style"], "notes");
    }

    #[test]
    fn a_room_with_the_notes_board() {
        let r = furnish(room(json!({"layout": {"notes0": [1, 1], "board": [10, 80]}, "furniture": ["notes0", "board", "settings0"], "v": 7})));
        let fs = furniture(&r);
        assert_eq!(fs, ["board", "settings0", "manager0", "booth0", "neuralyzer0", "clock"]);
        assert!(r["layout"].get("notes0").is_none() && r["layout"].get("board").is_some());
    }

    #[test]
    fn the_first_office_manager_and_empty_booths() {
        let r = furnish(room(json!({
            "furniture": ["manager0", "manager1", "clock", "bench0", "goggles0"], "layout": {"manager1": [1, 1], "bench0": [2, 2]},
            "manager": {"style": "ask", "id": "x1"}, "v": 12
        })));
        assert_eq!(r["managers"], json!({"manager0": {"style": "ask", "id": "x1"}}));
        assert!(r.get("manager").is_none());
        assert_eq!(furniture(&r), ["manager0", "clock"]);
        assert!(r["layout"].as_object().unwrap().is_empty());
        // a booth nobody was hired at, in a room of v 13: it goes
        let r = furnish(room(json!({"furniture": ["manager0", "clock"], "managers": {"gone9": {"style": "ask"}}, "v": 13})));
        assert_eq!(furniture(&r), ["clock"]);
        assert!(r["managers"].as_object().unwrap().is_empty());
    }

    #[test]
    fn settings_merge() {
        let r = room(json!({"settings": {"yolo": 1, "rc": false, "nonsense": true}}));
        let o = options(&r);
        assert!(!o.rc && o.yolo && !o.hardcore && o.sound);
        assert_eq!(o.other.keys().collect::<Vec<_>>(), ["files"], "nonsense is not a setting");
        assert_eq!(o.other["files"], json!(true));
        let o = options(&room(json!({})));
        assert!(o.rc && !o.yolo && o.sound);
    }

    #[test]
    fn you() {
        assert_eq!(you_of(&room(json!({})), "rembert"), ("rembert".into(), "rembert".into()));
        assert_eq!(you_of(&room(json!({"you": {"name": " Jo ", "seed": "jo:3"}})), "x"), ("Jo".into(), "jo3".into()));
        assert_eq!(you_of(&room(json!({"you": {"name": "!!"}})), "x"), ("!!".into(), "you".into()));
    }

    #[test]
    fn from_the_rust_apps_rooms() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::under(dir.path(), false);
        std::fs::create_dir_all(&cfg.config).unwrap();
        std::fs::create_dir_all(cfg.claude_settings.parent().unwrap()).unwrap();
        std::fs::write(&cfg.claude_settings, r#"{"sshConfigs": [{"id": "ody", "sshHost": "odyssey"}]}"#).unwrap();
        std::fs::write(
            cfg.old_rooms(),
            r#"[{"ssh": "", "dirs": ["/p/a", "/p/a/.claude/worktrees/w"], "layout": {"u:/p/a": [10, 20]}},
                {"ssh": "odyssey", "dirs": ["/q"], "layout": {"u:/q": [1, 1]}},
                {"ssh": "unknown", "dirs": ["/r"]}]"#,
        )
        .unwrap();
        let r = load_room(&cfg);
        assert_eq!(desks(&r), ["/p/a", "ody:/q"]);
        assert_eq!(r["layout"], json!({"u:/p/a": [10, 20]}));
        // and room.json, once there, is what is read
        save_room(&cfg, &r);
        let again = load_room(&cfg);
        assert_eq!(again, r);
    }
}
