//! The room's pictures against the web room's own (tests/golden, made by native/tools/golden.mjs from
//! the real room.js in headless Chrome). A mismatch writes what was drawn to target/golden-out/ to look at.

use burrow_room::{avatar, figure, font};
use pix::{Image, css};
use std::path::PathBuf;

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// how many pixels differ (any channel by more than 2, alpha compared as solid or not)
fn diff(a: &Image, b: &Image) -> usize {
    if a.w != b.w || a.h != b.h {
        return usize::MAX;
    }
    a.px.iter()
        .zip(&b.px)
        .filter(|(p, q)| {
            let (sa, sb) = (p[3] > 0, q[3] > 0);
            sa != sb || (sa && (0..3).any(|i| (p[i] as i32 - q[i] as i32).abs() > 2))
        })
        .count()
}

/// compare every reference whose name starts with `prefix` against `draw(name)`; none may differ
fn check(prefix: &str, draw: impl Fn(&str) -> Option<Image>) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-out");
    let mut bad = Vec::new();
    let mut n = 0;
    for e in std::fs::read_dir(golden()).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        let want = Image::from_png(&std::fs::read(&p).unwrap()).unwrap();
        let Some(got) = draw(&name) else { continue };
        n += 1;
        let d = diff(&got, &want);
        if d > 0 {
            std::fs::create_dir_all(&out).unwrap();
            got.save_png(out.join(format!("{name}.png"))).unwrap();
            bad.push(format!("{name}: {} pixels ({}x{} vs {}x{})", if d == usize::MAX { -1 } else { d as i64 }, got.w, got.h, want.w, want.h));
        }
    }
    assert!(n > 0, "no references for {prefix}");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn avatars_are_pairs() {
    // avatar-agent-<seed> / avatar-human-<seed>, the colour by the seed's place in the list
    let seeds = ["demo0001", "demo0002", "demo0003", "8f2a1c9e", "manager-16", "susan-10", "rembert-15", "you", "stijnslaets-3", "a", "zz-99", "linda-4"];
    let cols = ["#4a76b8", "#e8402a", "#2fa087", "#f2a531", "#9a74d6", "#d97757"];
    check("avatar-", |name| {
        let rest = name.strip_prefix("avatar-")?;
        let (kind, seed) = rest.split_once('-')?;
        let i = seeds.iter().position(|s| *s == seed)?;
        Some((*avatar::avatar(seed, kind == "agent", Some(css(cols[i % cols.len()])))).clone())
    });
}

#[test]
fn figures_are_room_js_bodies() {
    // body-<seed>-<arms>-<legs>-<h|r><f?>-<colour>
    check("body-", |name| {
        let parts: Vec<&str> = name.strip_prefix("body-")?.rsplitn(5, '-').collect();
        let [col, kind, legs, arms, seed] = parts[..] else { return None };
        let human = kind.starts_with('h');
        let femme = kind.ends_with('f');
        Some((*figure::body(seed, css(&format!("#{col}")), figure::Arms::parse(arms), figure::Legs::parse(legs), human, femme)).clone())
    });
}

#[test]
fn lettering_is_pairs() {
    check("tiny-alphabet", |_| {
        let mut im = Image::new(260, 14);
        let mut c = im.ctx();
        font::tiny(&mut c, "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789", 0., 0., css("#ffffff"));
        font::tiny(&mut c, ".,!?'-:()/+=%*#\"&_~ burrow", 0., 7., css("#d97757"));
        Some(im)
    });
}

#[test]
fn layout_is_room_js_layout() {
    // the web room's own layout of a demo state (tools/golden.mjs, case layout-demo): desks, seats, doors, pieces
    use burrow_room::layout::{Keep, Size, layout};
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(golden().join("layout-demo.json")).unwrap()).unwrap();
    let s: burrow_room::model::State = serde_json::from_value(v["state"].clone()).unwrap();
    let size = Size { w: v["W"].as_i64().unwrap() as i32, h: v["H"].as_i64().unwrap() as i32 };
    let l = layout(&s, &s.desks, &size, &mut Keep::default(), None, &|_| false, 0.0);
    for (d, want) in l.desks.iter().zip(v["desks"].as_array().unwrap()) {
        assert_eq!(d.key, want["key"].as_str().unwrap());
        assert_eq!(serde_json::json!(d.at), want["at"], "{}", d.key);
        assert_eq!(d.w as i64, want["w"].as_i64().unwrap(), "{}", d.key);
        assert_eq!(d.label, want["label"].as_str().unwrap());
        let seats: Vec<serde_json::Value> = d.seats.iter().map(|(id, x, _)| serde_json::json!([id, x])).collect();
        assert_eq!(serde_json::json!(seats), want["seats"], "{}", d.key);
    }
    let doors: Vec<serde_json::Value> = l.doors.iter().map(|d| serde_json::json!([d.key, d.x0, d.y0])).collect();
    assert_eq!(serde_json::json!(doors), v["doors"]);
    let pieces: Vec<serde_json::Value> = l.pieces.iter().map(|p| serde_json::json!([p.key, p.at, p.r])).collect();
    assert_eq!(serde_json::json!(pieces), v["pieces"]);
}
