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
