//! props3d's pictures against the web's own (tests/web/props/*.png, from Props3D on the demo page with
//! web/look.mjs and tests/web/props/steps.json), and the parts that must be bit for bit pixel.js's and props3d.js's.

use burrow_fps::props::{hash, robot_parts};
use burrow_fps::*;
use pix::Image;

fn web(name: &str) -> Image {
    Image::from_png(&std::fs::read(format!("{}/tests/web/props/{name}.png", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap()
}
/// (pixels whose alpha differs, pixels whose colour differs and by how much at most)
fn differ(a: &Image, b: &Image) -> (usize, usize, i32) {
    assert_eq!((a.w, a.h), (b.w, b.h));
    let (mut da, mut dc, mut most) = (0, 0, 0);
    for (p, q) in a.px.iter().zip(&b.px) {
        da += (p[3] != q[3]) as usize;
        let m = (0..3).map(|k| (p[k] as i32 - q[k] as i32).abs()).max().unwrap();
        if m > 0 && p[3] > 0 {
            dc += 1;
            most = most.max(m);
        }
    }
    (da, dc, most)
}
fn robot(seed: &str, color: &str, arms: &str, legs: &str, femme: bool) -> Robot {
    Robot { seed: seed.into(), color: color.into(), arms: arms.into(), legs: legs.into(), femme, red: false }
}

#[test]
fn hash_and_robot_parts_are_the_web_s() {
    // node: props3d.js's hash, pixel.js's avatar.robot
    assert_eq!(hash(1.0, 2.0, 3.0), 0.445841187145561);
    assert_eq!(hash(-12.000000000000002, 1.0, 0.0), 0.4184437906369567);
    assert_eq!(hash(123456.0, -7.0, 11.0), 0.7344607152044773);
    assert_eq!(hash(0.0, 0.0, 0.0), 0.0);
    assert_eq!(hash(5.0, 9.0, 0.0), 0.5722004037816077);
    let p = |top, ant, eyes, teeth| props::RobotParts { top, ant, eyes, teeth };
    assert_eq!(robot_parts("clever-cleo"), p(4, "two", "cyclops", false));
    assert_eq!(robot_parts("demo0003"), p(4, "two", "dots", false));
    assert_eq!(robot_parts("manager-16"), p(4, "one", "happy", false));
    assert_eq!(robot_parts(""), p(4, "dish", "happy", false));
    assert_eq!(robot_parts("é-ü-42"), p(3, "two", "cyclops", true));
}

#[test]
fn figures_are_the_web_s() {
    let mut p = Props::new();
    let a = robot("demo0001", "#4a76b8", "fwd1", "sit", false);
    let cases = [
        ("robot_a0", Figure::Robot(a.clone()), 0),
        ("robot_a3", Figure::Robot(a.clone()), 3),
        ("robot_b1", Figure::Robot(robot("clever-cleo", "#2fa087", "wave", "walk1", false)), 1),
        ("robot_c6", Figure::Robot(robot("manager-16", "#f06b72", "gun", "stand", true)), 6),
        ("dog_2", Figure::Dog { pose: "walk2".into(), wag: true }, 2),
    ];
    for (name, fig, k) in cases {
        let f = p.figure(&fig, k);
        assert_eq!(differ(&web(name), &f.tex.img), (0, 0, 0), "{name}");
        assert_eq!((f.ax, f.ay, f.res), (f.w / 2.0, f.h - 1.0, 4.0));
    }
    let l = p.lying(&a);
    assert_eq!(differ(&web("lying"), &l.tex.img), (0, 0, 0));
    assert_eq!(differ(&web("plant1"), &p.plant(1).tex.img), (0, 0, 0));
    // the flames' grain is seeded by how many pictures were made before them: a step at most
    let (da, _, most) = differ(&web("flames2"), &p.flames(2, true).tex.img);
    assert_eq!(da, 0);
    assert!(most <= 2);
}

#[test]
fn every_kind_builds() {
    let mut p = Props::new();
    for kind in ["coffee", "cooler", "printer", "manager", "add", "booth", "neuralyzer", "doghouse", "radio", "chess", "meet", "pong", "board", "armchair", "plant", "pair"] {
        let piece = props::Piece { kind: kind.into(), key: format!("{kind}0"), x: 100.0, y: 100.0, ..Default::default() };
        let parts = p.build(&piece).unwrap_or_else(|| panic!("{kind}"));
        assert!(!parts.boxes.is_empty() || !parts.things.is_empty(), "{kind}");
        assert!(!parts.solid.is_empty(), "{kind}");
    }
    assert!(p.build(&props::Piece { kind: "fire".into(), ..Default::default() }).is_none());
    let gun = p.build(&props::Piece { kind: "booth".into(), gun: true, out: 1, ..Default::default() }).unwrap();
    assert_eq!(gun.boxes.len(), 2);
    let wall = p.wall(256.0, 40.0, 26.0, &[], "k");
    assert_eq!((wall.w(), wall.h()), (1024, 160));
    assert_eq!(p.wall(256.0, 40.0, 26.0, &[], "k").id, wall.id, "kept while its key stays");
}

#[test]
fn a_mutineer_s_eyes_are_red() {
    let mut p = Props::new();
    let a = robot("demo0001", "#4a76b8", "gun", "stand", false);
    let calm = p.figure(&Figure::Robot(a.clone()), 0).tex.img.clone();
    let red = p.figure(&Figure::Robot(Robot { red: true, ..a }), 0).tex.img.clone();
    let (da, dc, _) = differ(&calm, &red);
    assert!(da == 0 && dc > 0, "its shape the same, its eyes another colour");
    // what differs is red: its eyes, nothing else
    for (p, q) in calm.px.iter().zip(&red.px) {
        if p != q {
            assert!(q[0] > 2 * q[1].max(q[2]), "{q:?} is not red");
        }
    }
}
