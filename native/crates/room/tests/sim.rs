//! The room left to itself: ten minutes of the demo's bots at twelve ticks a second, no window. Whatever they
//! get up to, they stay in the room, a bot at work sits at its desk, and nothing breaks.

use burrow_room::io::{Input, Output};
use burrow_room::model::State;
use burrow_room::room::Room;

fn demo() -> State {
    State::from_json(include_str!("../../model/tests/fixtures/demo-state.json")).unwrap()
}

fn run(room: &mut Room, inp: &mut Input, secs: f64) -> Vec<serde_json::Value> {
    let mut acts = Vec::new();
    let frames = (secs * 60.0) as usize;
    for _ in 0..frames {
        inp.now += 1000.0 / 60.0;
        let mut out = Output::default();
        if room.advance(1.0 / 60.0) {
            room.step(inp, &mut out); // (the app steps and draws at the ticks, and when the pointer moves)
        }
        acts.extend(out.acts);
    }
    acts
}

#[test]
fn ten_minutes_of_the_demo() {
    let s = demo();
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(7, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 600.0);
    let l = room.last.clone().unwrap();
    for f in &room.w.folk {
        assert!(f.x.is_finite() && f.y.is_finite(), "{} at {}, {}", f.id, f.x, f.y);
        assert!(f.x >= 0.0 && f.x <= l.w as f64 && f.y >= 0.0 && f.y <= l.h as f64, "{} out of the room at {}, {} ({:?})", f.id, f.x, f.y, f.act.kind);
    }
    // every bot is a figure, and you
    for b in &s.bots {
        assert!(room.w.get(&b.id).is_some(), "{} has no figure", b.id);
    }
    assert!(room.w.get("you").is_some());
    // a bot at work sits at its desk, in its chair
    for b in s.bots.iter().filter(|b| b.busy) {
        let f = room.w.get(&b.id).unwrap();
        if let Some(seat) = l.seat(&b.id) {
            assert_eq!(f.act.kind, "desk", "{} works but is at {}", b.id, f.act.kind);
            assert!((f.x - seat.x).abs() < 0.5 && (f.y - seat.y).abs() < 0.5, "{} works away from its chair", b.id);
            assert_eq!(f.legs, "sit");
        }
    }
    // they did all sorts of things meanwhile
    let kinds: std::collections::HashSet<&str> = room.w.s.stats.values().flat_map(|s| s.acts.keys().copied()).collect();
    assert!(kinds.len() >= 6, "only {kinds:?}");
}

#[test]
fn a_bot_that_goes_walks_out_and_one_that_comes_walks_in() {
    let mut s = demo();
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(3, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 10.0);
    let gone = s.bots.remove(0);
    let mut new = s.bots[0].clone();
    new.id = "fresh1".into();
    new.busy = true;
    s.bots.push(new);
    inp.state = s;
    run(&mut room, &mut inp, 60.0);
    assert!(room.w.get(&gone.id).is_none(), "{} is still in the room", gone.id);
    let f = room.w.get("fresh1").expect("the newcomer");
    assert_eq!(f.act.kind, "desk");
    assert_eq!(f.legs, "sit");
}

#[test]
fn the_booth_deletes_after_five_seconds_and_the_undo_keeps() {
    let mut s = demo();
    s.furniture.push("booth0".into());
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(5, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 5.0);
    let l = room.last.clone().unwrap();
    let bo = l.booths[0].key.clone();
    let id = s.bots.iter().find(|b| !b.busy && !b.waiting).unwrap().id.clone();
    let f = room.w.get_mut(&id).unwrap();
    f.act = burrow_room::world::Act { kind: "doom", at: Some(bo.clone()), left: Some(5.0), ..Default::default() };
    let acts = run(&mut room, &mut inp, 6.0);
    assert!(acts.iter().any(|a| a["kind"] == "stop" && a["id"] == id.as_str() && a["rm"] == true), "{acts:?}");
    assert!(room.w.get(&id).is_none());
}

#[test]
fn the_room_draws() {
    // a picture of the demo room after a minute, for looking at (target/room-demo.png)
    let s = demo();
    let mut inp = Input { state: s, now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(11, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 60.0);
    let mut out = Output::default();
    let l = room.step(&inp, &mut out);
    room.draw(&inp, &mut out, &l);
    assert_eq!(room.img.w, 512);
    assert!(room.img.px.iter().all(|p| p[3] == 255), "the room is opaque");
    room.img.save_png(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/room-demo.png")).unwrap();
}

#[test]
fn the_room_in_the_first_person() {
    // the demo room as the gun closet's view sees it: a scene with every desk and piece, everyone in it, and a frame
    // drawn from it (target/room-fps.png)
    use burrow_fps::{Fps, Options, Start, Wad};
    let mut s = demo();
    s.furniture.push("booth0".into());
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(13, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 20.0);
    let scene = room.scene3(&inp).expect("a scene");
    assert!(scene.boxes.len() > 20, "{} boxes", scene.boxes.len());
    assert!(scene.things.iter().filter(|t| t.bot.is_some()).count() >= s.bots.len() / 2);
    assert!(!scene.items.is_empty());
    let wad = Wad::open_bytes(burrow_fps::wad::freedoom()).unwrap();
    let door = Wad::open_bytes(burrow_fps::wad::door_wad()).ok();
    let start = Start { x: 128.0, y: 300.0, a: -std::f64::consts::FRAC_PI_2 };
    let mut fps = Fps::new(wad, door, scene, start, Options { seed: 1, ..Default::default() }).unwrap();
    for _ in 0..10 {
        fps.tick(1000.0 / 35.0);
    }
    let mut img = pix::Image::new(640, 400);
    fps.draw(&mut img);
    let distinct: std::collections::HashSet<[u8; 4]> = img.px.iter().copied().collect();
    assert!(distinct.len() > 30, "a frame with something in it ({} colours)", distinct.len());
    img.save_png(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/room-fps.png")).unwrap();
}

#[test]
fn a_condemned_bot_only_starts_until_the_gun_comes_its_way() {
    let mut s = demo();
    s.furniture.push("booth0".into());
    s.settings.hardcore = true;
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(5, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 5.0);
    let id = s.bots.iter().find(|b| !b.busy && !b.waiting).expect("an idle bot").id.clone();
    let mut out = Output::default();
    assert_eq!(room.condemn(&inp, &mut out, std::slice::from_ref(&id)), vec![id.clone()]);
    let (x0, y0) = { let f = room.w.get(&id).unwrap(); (f.x, f.y) };
    let mut panicked = false;
    for _ in 0..60 * 40 {
        run(&mut room, &mut inp, 1.0 / 60.0);
        let Some(f) = room.w.get(&id) else { break }; // shot
        let gun_coming = room.w.folk.iter().any(|g| g.carry == Some("gun") && g.hunt.as_ref().is_some_and(|h| h.id.as_deref() == Some(id.as_str())));
        if !f.act.panic {
            assert!(f.act.kind != "condemned" || ((f.x - x0).abs() < 0.01 && (f.y - y0).abs() < 0.01), "it walked off before the gun came");
        } else {
            assert!(gun_coming || panicked, "it panicked with no gun coming");
            panicked = true;
        }
    }
    assert!(panicked, "the gun came and it never panicked");
}

#[test]
fn a_pair_room_open_puts_you_on_its_phone() {
    let mut s = demo();
    s.furniture.push("pair0".into());
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(9, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 3.0);
    let pt = room.last.clone().unwrap().pairs[0].clone();
    inp.pair = Some(pt.key.clone());
    run(&mut room, &mut inp, 20.0);
    let you = room.w.get("you").unwrap();
    assert_eq!((you.act.kind, you.act.there, you.arms), ("phone", true, "phone"));
    assert!(((you.x - pt.x as f64).abs() - 20.0).abs() < 0.5 && you.dir * (you.x - pt.x as f64) < 0.0, "beside the table, facing it");
    inp.pair = None;
    run(&mut room, &mut inp, 1.0);
    assert_ne!(room.w.get("you").unwrap().act.kind, "phone", "closing the room hangs up");
}

#[test]
fn a_bot_posting_in_a_pair_room_beams_to_its_table_and_reading_beams_back() {
    let mut s = demo();
    s.furniture.push("pair0".into());
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(4, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 2.0);
    let key = room.last.clone().unwrap().pairs[0].key.clone();
    let id = s.bots[0].id.clone();
    let mut out = Output::default();
    room.pair_talk(&inp, &mut out, &id, &key, "say");
    room.pair_talk(&inp, &mut out, &id, &key, "read");
    room.pair_talk(&inp, &mut out, &id, "pair9", "say"); // no such table: nothing
    let b = &room.w.s.beams;
    assert_eq!(b.len(), 2);
    assert_eq!((b[0].from.as_str(), b[0].to_table.as_deref()), (id.as_str(), Some(key.as_str())));
    assert_eq!((b[1].from_table.as_deref(), b[1].to.as_str()), (Some(key.as_str()), id.as_str()));
    run(&mut room, &mut inp, 0.5);
    assert_eq!(room.w.s.beams.len(), 2, "under way");
    run(&mut room, &mut inp, 1.5);
    assert!(room.w.s.beams.is_empty(), "arrived");
}

#[test]
fn the_painting_grows_only_while_a_robot_paints_at_the_easel() {
    let mut s = demo();
    if !s.furniture.iter().any(|k| k.starts_with("board")) {
        s.furniture.push("board0".into());
    }
    let mut inp = Input { state: s, now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(11, 0.0);
    room.resize(&inp, 820.0);
    let mut painted = 0;
    let mut n = None;
    for _ in 0..12 * 60 * 12 {
        run(&mut room, &mut inp, 5.0 / 60.0);
        let a = room.w.s.art.as_ref().expect("a painting on the easel from the start");
        if let Some(n0) = n
            && a.n > n0
        {
            painted += 1;
            assert!(room.w.folk.iter().any(|f| f.act.kind == "paint" && f.act.there), "it grew with nobody painting");
            assert!(!a.by.is_empty());
        }
        n = Some(a.n);
    }
    assert!(painted > 0, "nobody painted in twelve minutes");
}

#[test]
fn shoot_three_and_the_rest_come_for_you_then_respawn_ends_it() {
    let mut s = demo();
    s.furniture.push("booth0".into());
    s.settings.hardcore = true;
    let mut inp = Input { state: s.clone(), now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(13, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 5.0);
    let idle: Vec<String> = s.bots.iter().filter(|b| !b.busy && !b.waiting).map(|b| b.id.clone()).collect();
    for id in &idle[..3] {
        let mut out = Output::default();
        assert!(room.shoot(&inp, &mut out, id), "{id} can be shot");
        run(&mut room, &mut inp, 2.0);
    }
    let mut seen = std::collections::HashSet::new();
    for _ in 0..12 * 180 {
        run(&mut room, &mut inp, 1.0 / 12.0);
        if let Some(m) = &room.w.s.mutiny {
            seen.insert(m.phase);
            if m.phase == "stroll" || m.phase == "close" {
                for id in &m.crew {
                    let f = room.w.get(id).unwrap();
                    assert_eq!(f.red, f.carry == Some("gun"), "{id}: red eyes with the gun, and only then");
                }
            }
        }
        if burrow_room::mutiny::dead(&room.w) {
            break;
        }
    }
    for p in ["gather", "talk", "stroll", "close"] {
        assert!(seen.contains(p), "never {p}: {seen:?}");
    }
    assert!(burrow_room::mutiny::dead(&room.w), "they never got you");
    let you = room.w.get("you").unwrap();
    assert!(you.lying && you.act.kind == "dead");
    // RESPAWN: in by the door, the fear gone
    room.respawn(&inp);
    assert!(!burrow_room::mutiny::dead(&room.w));
    assert_eq!(room.w.s.fear, 0.0);
    run(&mut room, &mut inp, 40.0);
    assert!(room.w.s.mutiny.is_none(), "the guns went back, it is over");
    assert!(room.w.folk.iter().all(|f| !f.red && (f.human || f.carry != Some("gun"))), "nobody kept a gun");
}
