//! Frames: the web version's (tests/web/*.png, taken from fps.js on the demo page with web/look.mjs and
//! tests/web/steps.json: a stub room, reduced motion, the level's lights at their base, its time 244) against ours;
//! a furnished office drawn the same twice; what shooting, the door and Esc do. FPS_SHOTS=<dir> saves the frames.

use burrow_fps::props::{Desk, Piece};
use burrow_fps::*;
use pix::{Image, css};
use std::f64::consts::PI;
use std::sync::Arc;

fn wads() -> (Wad, Option<Wad>) {
    (Wad::open_bytes(wad::freedoom()).unwrap(), Wad::open_bytes(wad::door_wad()).ok())
}
fn save(name: &str, im: &Image) {
    if let Ok(dir) = std::env::var("FPS_SHOTS") {
        im.save_png(format!("{dir}/{name}.png")).unwrap();
    }
}
/// how many pixels differ, leaving out what `skip` says
fn differ(name: &str, mine: &Image, skip: impl Fn(u32, u32) -> bool) -> usize {
    let r = Image::from_png(&std::fs::read(format!("{}/tests/web/{name}.png", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap();
    assert_eq!((r.w, r.h), (mine.w, mine.h), "{name}");
    save(&format!("mine_{name}"), mine);
    (0..r.px.len()).filter(|&i| !skip(i as u32 % r.w, i as u32 / r.w) && r.px[i][..3] != mine.px[i][..3]).count()
}
/// the stub room of tests/web/steps.json
fn stub(still: bool) -> Fps {
    let (w, dw) = wads();
    let sc = Arc::new(Scene::empty(256.0, 180.0, 26.0));
    Fps::new(w, dw, sc, Start { x: 128.0, y: 150.0, a: -PI / 2.0 }, Options { seed: 7, still, ..Default::default() }).unwrap()
}

#[test]
fn the_level_as_the_web_draws_it() {
    let mut f = stub(true);
    for s in &mut f.level_mut().map.sectors {
        s.light = s.light0;
    }
    f.level_mut().time = 244;
    let st = f.level().start.clone();
    let a = st.angle.to_radians();
    let one = f.snap(LCam { x: st.x, y: st.y, z: 41.0, a });
    let two = f.snap(LCam { x: st.x + 300.0, y: st.y + 120.0, z: 41.0, a: a + 2.2 });
    assert_eq!(differ("level2", &two, |_, _| false), 0);
    // two bonus potions there were on another frame of theirs
    assert!(differ("level", &one, |_, _| false) < 100);
}

#[test]
fn the_office_as_the_web_draws_it() {
    let mut f = stub(true);
    f.tick(30.0);
    let mut im = Image::new(640, 400);
    f.draw(&mut im);
    // not the face (it looks about at random) nor the help (the stub room had no letters)
    let n = differ("office", &im, |x, y| ((286..334).contains(&x) && y >= 336) || (320..334).contains(&y));
    assert_eq!(n, 0);
}

fn wood() -> Paint {
    Paint { hi: css("#cfa86d"), top: css("#b38857"), d: css("#885c47"), k: css("#532e3d"), plate: css("#2e1f28") }
}
fn robot(seed: &str, color: &str, arms: &str, legs: &str) -> Figure {
    Figure::Robot(Robot { seed: seed.into(), color: color.into(), femme: false, arms: arms.into(), legs: legs.into() })
}
/// an office with a desk, bots, furniture, the wall's things and a fire pit on the floor
fn furnished(props: &mut Props, label: &str) -> Scene {
    let mut sc = Scene::empty(256.0, 180.0, 26.0);
    sc.items = vec![
        WallItem { kind: WallKind::Door { paint: wood(), ok: true }, x0: 10.0, x1: 26.0, y0: 4.0, y1: 26.0 },
        WallItem { kind: WallKind::Window { hour: 14 }, x0: 60.0, x1: 84.0, y0: 6.0, y1: 18.0 },
        WallItem { kind: WallKind::Clock { hours: 10, minutes: 10, awake: true }, x0: 100.0, x1: 117.0, y0: 4.0, y1: 21.0 },
        WallItem { kind: WallKind::Closet, x0: 150.0, x1: 170.0, y0: 2.0, y1: 26.0 },
        WallItem { kind: WallKind::Settings, x0: 180.0, x1: 194.0, y0: 6.0, y1: 20.0 },
    ];
    sc.wkey = "furnished".into();
    sc.wall_pic = props.wall(256.0, 40.0, 26.0, &sc.items, &sc.wkey);
    let mut floor = Image::new(512, 360);
    {
        let mut g = floor.ctx();
        g.fill_circle(300.0, 200.0, 20.0, css("rgba(0,0,0,0.3)"));
        let pit = props.pit(44.0, 18.0);
        g.draw_image_scaled(&pit.img, 360.0, 220.0, 88.0, 36.0);
    }
    sc.floor = Tex::new(floor);
    sc.add(props.desk(&Desk {
        x0: 30.0,
        y: 100.0,
        w: 76.0,
        cab: 30.0,
        label: label.into(),
        paint: wood(),
        color: "#d97757".into(),
        t_now: 1.0,
        seats: vec![78.0],
        mugs: vec![(78.0, "#4a76b8".into())],
        screen: "busy".into(),
        laptops: vec!["on".into()],
    }));
    let piece = |kind: &str, x: f64, y: f64| Piece { kind: kind.into(), key: format!("{kind}0"), x, y, ..Default::default() };
    for p in [
        Piece { level: Some(0.6), ..piece("coffee", 130.0, 50.0) },
        Piece { level: Some(0.3), ..piece("cooler", 165.0, 50.0) },
        Piece { gun: true, ..piece("booth", 230.0, 60.0) },
        Piece { board: "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr".into(), seats: vec![(166.0, 112.0), (194.0, 112.0)], ..piece("chess", 180.0, 115.0) },
        piece("plant", 210.0, 90.0),
        piece("armchair", 60.0, 140.0),
        Piece { on: true, ..piece("radio", 112.0, 72.0) },
        Piece { linked: true, ..piece("pair", 140.0, 140.0) },
    ] {
        sc.add(props.build(&p).unwrap_or_else(|| panic!("{}", p.kind)));
    }
    let fl = props.flames(1, false);
    sc.things.push(Thing { key: Some("fire".into()), x: 192.0, y: 128.0, pic: Some(Face::whole(fl.tex)), ax: fl.ax, ay: fl.ay, res: fl.res, piece: Some("fire0".into()), ..Default::default() });
    sc.lights.push(Light { x: 192.0, y: 128.0, z: 9.0, r: 140.0, flicker: true, ..Default::default() });
    for (key, x, y, fig, seated) in [
        ("demo0001", 78.0, 99.0, robot("demo0001", "#4a76b8", "fwd1", "sit"), true),
        ("demo0002", 120.0, 120.0, robot("demo0002", "#2fa087", "wave", "stand"), false),
        ("demo0003", 150.0, 95.0, robot("demo0003", "#f2a531", "mug", "walk1"), false),
        ("dog", 100.0, 150.0, Figure::Dog { pose: "sit".into(), wag: true }, false),
    ] {
        let bot = (key != "dog").then(|| key.to_string());
        sc.things.push(Thing { key: Some(key.into()), x, y, fig: Some(fig), seated, face: 1.0, name: bot.clone(), bot, ..Default::default() });
    }
    sc
}

fn hash(im: &Image) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for p in &im.px {
        for &b in p {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    h
}
/// the furnished office from near the door, looking a little left of north; yours: your face and skin
fn office_frame(yours: bool) -> Image {
    let (w, dw) = wads();
    let mut props = Props::new();
    let sc = Arc::new(furnished(&mut props, if yours { "BURROW" } else { "" }));
    let face = {
        let mut im = Image::new(20, 20);
        im.ctx().fill_circle(10.0, 10.0, 7.0, css("#f7c59a"));
        im
    };
    let opt = if yours { Options { seed: 3, still: true, face: Some(face), skin: Some(css("#c27a54")), ..Default::default() } } else { Options { seed: 3, still: true, ..Default::default() } };
    let mut f = Fps::new(w, dw, sc, Start { x: 128.0, y: 172.0, a: -PI / 2.0 - 0.3 }, opt).unwrap();
    f.tick(200.0);
    let mut im = Image::new(640, 400);
    f.draw(&mut im);
    im
}

#[test]
fn a_furnished_office_as_the_web_draws_it() {
    // the same scene made with the page's Props3D (tests/web/furnished.js): the textures' grain is seeded by how many
    // were made before them, so the faces of the boxes differ a little; the rest is the same
    let n = differ("furnished", &office_frame(false), |x, y| ((286..334).contains(&x) && y >= 336) || (320..334).contains(&y));
    assert!(n < 640 * 336 / 10, "{n}");
}

#[test]
fn a_furnished_office_is_drawn_the_same_every_time() {
    let (a, b) = (office_frame(true), office_frame(true));
    save("furnished", &a);
    assert!(a == b, "two runs differ");
    // something of everything is in it: wood, the bots' colours, the fire's light
    let lit = a.px.iter().filter(|p| p[0] > 200 && p[1] > 120).count();
    assert!(lit > 200, "{lit}");
    // and as it was when this was written (change it, after looking, when the drawing changes on purpose)
    assert_eq!(hash(&a), GOLDEN, "the furnished office changed: look at it (FPS_SHOTS)");
}
const GOLDEN: u64 = 9218361961550039637;

#[test]
fn shooting_the_bot_in_front_of_you() {
    let (w, dw) = wads();
    let mut sc = Scene::empty(256.0, 180.0, 26.0);
    sc.things.push(Thing { key: Some("demo0002".into()), x: 128.0, y: 100.0, fig: Some(robot("demo0002", "#2fa087", "down", "stand")), bot: Some("demo0002".into()), name: Some("Tidy-tess".into()), ..Default::default() });
    let mut f = Fps::new(w, dw, Arc::new(sc), Start { x: 128.0, y: 150.0, a: -PI / 2.0 }, Options { seed: 1, still: true, ..Default::default() }).unwrap();
    let mut im = Image::new(640, 400);
    f.tick(30.0);
    f.draw(&mut im);
    let shells = f.level().player.ammo[1];
    f.click();
    for _ in 0..10 {
        f.tick(30.0);
    }
    let ev = f.events();
    assert!(ev.contains(&Event::Shot { bot: "demo0002".into() }), "{ev:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Sound(Sound::Doom { name: "DSSHOTGN", .. }))));
    assert!(ev.iter().any(|e| matches!(e, Event::Alarm { .. })));
    assert_eq!(f.level().player.ammo[1], shells - 1);
}

#[test]
fn through_the_door_into_the_level_and_back() {
    let mut f = stub(true);
    let mut im = Image::new(640, 400);
    // in front of the door (round(256 * 0.3) = 77 dots along), facing it: walk into it, it opens, walk on
    f.set(77.0, 168.0, Some(PI / 2.0));
    f.key_down(Key::W);
    let mut opened = false;
    for _ in 0..120 {
        f.tick(1000.0 / 35.0);
        opened |= f.door_open() > 0.0;
        if f.world() == World::Level {
            break;
        }
    }
    assert!(opened);
    assert_eq!(f.world(), World::Level);
    f.draw(&mut im);
    save("through", &im);
    // turn round and walk back
    f.key_up(Key::W);
    f.turn(PI);
    f.key_down(Key::W);
    for _ in 0..70 {
        f.tick(1000.0 / 35.0);
        if f.world() == World::Office {
            break;
        }
    }
    assert_eq!(f.world(), World::Office);
    f.tick(1000.0 / 35.0); // (where you are goes to the room at the office's tics)
    assert!(f.you().y < 180.0);
}

#[test]
fn esc_leaves_and_the_view_melts_away() {
    let mut f = stub(false);
    let mut win = Image::new(800, 500);
    f.tick(30.0);
    f.draw_window(&mut win);
    // the window is letterboxed at 4:3: black at the sides
    assert_eq!(win.get(10, 250), [0, 0, 0, 255]);
    assert_ne!(win.get(400, 100), [0, 0, 0, 255]);
    f.key_down(Key::Escape);
    assert!(f.left());
    assert!(f.events().contains(&Event::Leave));
    let mut m = f.take_leave_melt().expect("a melt");
    let mut room = Image::filled(800, 500, css("#336699"));
    let mut t = f.now();
    let mut n = 0;
    while m.draw(&mut room, t) {
        t += 1000.0 / 35.0;
        n += 1;
        assert!(n < 200, "it never ends");
    }
    assert!(n > 5);
}

/// how long a frame takes (cargo test --release --test frame timing -- --ignored --nocapture)
#[test]
#[ignore]
fn timing() {
    let (w, dw) = wads();
    let mut props = Props::new();
    let sc = Arc::new(furnished(&mut props, "BURROW"));
    let mut f = Fps::new(w, dw, sc, Start { x: 128.0, y: 172.0, a: -PI / 2.0 - 0.3 }, Options { seed: 3, still: true, ..Default::default() }).unwrap();
    let mut im = Image::new(640, 400);
    f.draw(&mut im);
    let t = std::time::Instant::now();
    for _ in 0..100 {
        f.tick(1000.0 / 60.0);
        f.draw(&mut im);
    }
    println!("office: {:.2} ms a frame", t.elapsed().as_secs_f64() * 10.0);
    f.into_level();
    f.draw(&mut im);
    let t = std::time::Instant::now();
    for _ in 0..100 {
        f.tick(1000.0 / 60.0);
        f.draw(&mut im);
    }
    println!("level: {:.2} ms a frame", t.elapsed().as_secs_f64() * 10.0);
}
