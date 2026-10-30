//! The operator's console in the demo room (room.js's consoles): under the pointer it says "jack in", a click asks
//! for the matrix, and in the first person it stands as its picture (target/console-fps.png to look at).

use burrow_room::io::{Input, Output};
use burrow_room::model::State;
use burrow_room::room::Room;

fn demo() -> State {
    State::from_json(include_str!("../../model/tests/fixtures/demo-state.json")).unwrap()
}

fn run(room: &mut Room, inp: &mut Input, secs: f64) {
    for _ in 0..(secs * 60.0) as usize {
        inp.now += 1000.0 / 60.0;
        let mut out = Output::default();
        if room.advance(1.0 / 60.0) {
            room.step(inp, &mut out);
        }
    }
}

#[test]
fn the_operators_console() {
    let mut s = demo();
    s.furniture.push("matrix0".into());
    s.layout.insert("matrix0".into(), [150, 360]);
    let mut inp = Input { state: s, now: 0.0, hour: 14, ..Default::default() };
    let mut room = Room::new(3, 0.0);
    room.resize(&inp, 820.0);
    run(&mut room, &mut inp, 2.0);
    let l = room.last.clone().unwrap();
    assert_eq!(l.consoles.iter().map(|p| (p.key.as_str(), p.x, p.y)).collect::<Vec<_>>(), [("matrix0", 150, 360)]);
    // the pointer on it: "jack in"; a click: the matrix
    let mut out = Output::default();
    room.pointer_move(150.0, 350.0, (0.0, 0.0), &mut out);
    let l = room.step(&inp, &mut out);
    room.draw(&inp, &mut out, &l);
    assert_eq!(room.tip.as_ref().map(|t| t.text.as_str()), Some("jack in"));
    room.pointer_down(150.0, 350.0, false);
    room.pointer_up(&inp, &mut out, (0.0, 0.0));
    assert!(out.acts.iter().any(|a| a["kind"] == "matrix"), "{:?}", out.acts);
    // in the first person: its picture, where it stands, named
    use burrow_fps::{Fps, Options, Start, Wad};
    let scene = room.scene3(&inp).expect("a scene");
    let t = scene.things.iter().find(|t| t.piece.as_deref() == Some("matrix0")).expect("the console in the first person");
    assert!(t.pic.is_some() && t.label.as_deref() == Some("matrix") && (t.y - 357.0).abs() < 0.5);
    let wad = Wad::open_bytes(burrow_fps::wad::freedoom()).unwrap();
    let start = Start { x: 150.0, y: 385.0, a: -std::f64::consts::FRAC_PI_2 };
    let mut fps = Fps::new(wad, None, scene, start, Options { seed: 1, ..Default::default() }).unwrap();
    fps.tick(1000.0 / 35.0);
    let mut img = pix::Image::new(640, 400);
    fps.draw(&mut img);
    img.save_png(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/console-fps.png")).unwrap();
}
