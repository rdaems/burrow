//! The room's painters (burrow_room::draw) against the web room's own: tests/golden/paint-*.png, made by
//! native/tools/golden.mjs from the cases in tests/golden/paint-cases.mjs (the real room.js in headless
//! Chrome). Each case is drawn here by its name. A mismatch writes what was drawn to target/golden-out/.

use burrow_room::draw::*;
use burrow_room::figure::{self, Arms, Legs};
use pix::{Color, Ctx, Image, css};
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

/// compare every reference whose name starts with `prefix` against `draw(name)`; none may differ by more
/// than `allow(name)` pixels
fn check(prefix: &str, allow: impl Fn(&str) -> usize, draw: impl Fn(&str) -> Option<Image>) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-out");
    let mut bad = Vec::new();
    let mut n = 0;
    let mut names: Vec<_> =
        std::fs::read_dir(golden()).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
    names.sort();
    for p in names {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        let want = Image::from_png(&std::fs::read(&p).unwrap()).unwrap();
        // (a reference of nothing but its background would let anything pass: a sprite not loaded yet)
        assert!(want.px.iter().any(|q| *q != want.px[0]), "{name}: the reference is blank");
        let Some(got) = draw(&name) else {
            bad.push(format!("{name}: not drawn"));
            continue;
        };
        n += 1;
        let d = diff(&got, &want);
        if d > allow(&name) {
            std::fs::create_dir_all(&out).unwrap();
            got.save_png(out.join(format!("{name}.png"))).unwrap();
            bad.push(format!(
                "{name}: {} pixels ({}x{} vs {}x{})",
                if d == usize::MAX { -1 } else { d as i64 },
                got.w,
                got.h,
                want.w,
                want.h
            ));
        }
    }
    assert!(n > 0, "no references for {prefix}");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad.join("\n"));
}
fn exact(_: &str) -> usize {
    0
}

const PLANK: &str = "#734728";
const WALLC: &str = "#28374b";
const NOON: Clock = Clock { h: 14, m: 25, s: 37 };
const EARLY: Clock = Clock { h: 3, m: 50, s: 5 };

/// a canvas filled with the floor or the wall, and a painter on it
fn on(w: u32, h: u32, bg: &str, f: impl FnOnce(&mut Ctx)) -> Image {
    let mut im = Image::filled(w, h, css(bg));
    f(&mut im.ctx());
    im
}
fn env() -> KindEnv {
    KindEnv { clock: NOON, ..KindEnv::default() }
}
fn bot() -> Image {
    (*figure::body("demo0001", project_color("u1"), Arms::Down, Legs::Stand, false, false)).clone()
}

#[test]
fn icons_are_room_js_drawkind() {
    check("paint-icon", exact, |name| {
        let (s, kind) = if let Some(k) = name.strip_prefix("paint-icon2-") { (2, k) } else { (1, name.strip_prefix("paint-icon-")?) };
        // a new door is in the paint the next machine gets: the demo has two machines, so PAINT[1]
        let e = if kind == "door" { KindEnv { door: PAINT[1], ..env() } } else { env() };
        Some(icon(kind, s, 4, &e))
    });
}

// ---- desks, as the cases build them: a bot a seat, its tree, its state
#[derive(Default, Clone, Copy)]
struct Sit {
    tree: &'static str,
    busy: bool,
    waiting: bool,
    off: bool,
    mug: bool,
    unit: &'static str,
}
struct Spec {
    label: &'static str,
    dir: &'static str,
    trees: &'static [&'static str],
    seats: Vec<Sit>,
}
fn spec(name: &str) -> Spec {
    let s = |f: fn(&mut Sit)| {
        let mut x = Sit::default();
        f(&mut x);
        x
    };
    match name {
        "empty" => Spec { label: "burrow", dir: "/Users/x/burrow", trees: &[], seats: vec![] },
        "idle" => Spec { label: "pair", dir: "/Users/x/pair", trees: &[], seats: vec![Sit::default(), Sit::default()] },
        "busy" => Spec {
            label: "burrow",
            dir: "/Users/x/burrow",
            trees: &[],
            seats: vec![
                s(|x| x.busy = true),
                s(|x| {
                    x.mug = true;
                    x.unit = "u7"
                }),
                s(|x| x.waiting = true),
            ],
        },
        "waits" => Spec { label: "knee-id", dir: "/home/r/knee-id", trees: &[], seats: vec![s(|x| x.waiting = true)] },
        "off" => Spec {
            label: "odyssey",
            dir: "~",
            trees: &[],
            seats: vec![
                s(|x| x.off = true),
                s(|x| {
                    x.off = true;
                    x.busy = true
                }),
            ],
        },
        _ => Spec {
            label: "burrow",
            dir: "/u/burrow",
            trees: &["/u/burrow-wt1", "/u/b/.claude/worktrees/x"],
            seats: vec![
                s(|x| x.busy = true),
                s(|x| {
                    x.tree = "/u/burrow-wt1";
                    x.waiting = true
                }),
                s(|x| x.tree = "/u/b/.claude/worktrees/x"),
            ],
        },
    }
}

/// a desk case: its spec, the room's T, lit, the ids lit, where the pointer is, the shells (dir, busy), the
/// shell whose tile is under the pointer
#[allow(clippy::too_many_arguments)]
fn desk_case(sp: Spec, t: f64, lit: bool, hl: &[&str], pointer: Option<&str>, shells: &[(&str, bool)], hl_shell: Option<&str>) -> Image {
    let label = sp.label.to_uppercase();
    let cab = (burrow_room::font::tiny_w(&label) + 6).max(22).max(if sp.trees.is_empty() { 0 } else { 26 + sp.trees.len() as i32 * LAP });
    let (x0, y) = (4, 46);
    let n = sp.seats.len() as i32;
    let xs: Vec<i32> = (0..n).map(|k| x0 + cab + 4 + 9 + k * 18).collect();
    let ids: Vec<String> = (0..n).map(|k| format!("b{k}")).collect();
    let hots = desk_hots(x0, y, cab, sp.trees.len());
    let (sx, sy) = screen_at(x0, cab, sp.trees.len(), y);
    let px = sx + 22 + sp.trees.len() as i32 * LAP + 2;
    let pt = match pointer {
        Some("plate") => Some((x0 + 5, y - 6)),
        Some("screen") => Some((sx + 5, sy + 5)),
        Some("plus") => Some((px + 2, sy + 2)),
        Some("laptop") => Some((sx + 26, y - 19)),
        _ => None,
    };
    let inside = |r: [i32; 4]| pt.is_some_and(|(x, y)| x >= r[0] && x < r[2] && y >= r[1] && y < r[3]);
    let on_screen = inside(hots.screen);
    let mut hl: Vec<String> = hl.iter().map(|s| s.to_string()).collect();
    if on_screen {
        hl.extend(ids.iter().cloned());
    }
    let screen = |tree: &str| {
        let mine: Vec<&Sit> = sp.seats.iter().filter(|s| s.tree == tree && !s.off).collect();
        let dir = if tree.is_empty() { sp.dir } else { tree };
        let sh = shells.iter().find(|(d, _)| *d == dir);
        Screen {
            seated: !mine.is_empty(),
            busy: mine.iter().any(|s| s.busy),
            waits: mine.iter().any(|s| s.waiting),
            shell: sh.is_some(),
            shell_busy: sh.is_some_and(|s| s.1),
        }
    };
    let trees = sp
        .trees
        .iter()
        .enumerate()
        .map(|(k, &dir)| {
            let on = inside(hots.laptops[k]);
            let mine = sp.seats.iter().enumerate().filter(|(_, s)| s.tree == dir).map(|(i, _)| ids[i].clone());
            let lit = hl_shell == Some(dir) || mine.clone().any(|id| hl.contains(&id));
            Tree { dir, screen: screen(dir), on, lit }
        })
        .collect();
    let d = Desk {
        x0,
        y,
        w: if n > 0 { cab + 4 + n * 18 } else { cab },
        cab,
        label: &label,
        dir: sp.dir,
        color: project_color(sp.dir),
        paint: WOOD,
        seats: xs.clone(),
        mugs: sp
            .seats
            .iter()
            .zip(&xs)
            .filter(|(s, _)| s.mug)
            .map(|(s, &x)| (x, project_color(if s.unit.is_empty() { "u1" } else { s.unit })))
            .collect(),
        screen: screen(""),
        trees,
        on_plate: inside(hots.plate),
        on_screen,
        on_plus: inside(hots.plus),
        lit,
        shell_lit: hl_shell == Some(sp.dir),
    };
    on(130, 50, PLANK, |c| draw_desk(c, &d, t))
}

#[test]
fn desks_are_room_js_drawdesk() {
    check("paint-desk-", exact, |name| {
        Some(match name.strip_prefix("paint-desk-")? {
            "empty" => desk_case(spec("empty"), 0., false, &[], None, &[], None),
            "idle" => desk_case(spec("idle"), 1.3, false, &[], None, &[], None),
            "busy" => desk_case(spec("busy"), 2.7, false, &[], None, &[], None),
            "waits" => desk_case(spec("waits"), 0.6, false, &[], None, &[], None),
            "off" => desk_case(spec("off"), 0.6, false, &[], None, &[], None),
            "trees" => desk_case(spec("trees"), 0.6, true, &["b2"], None, &[], None),
            "plate" => desk_case(spec("idle"), 1.3, false, &[], Some("plate"), &[], None),
            "plus" => desk_case(spec("idle"), 1.3, false, &[], Some("plus"), &[], None),
            "screen" => desk_case(spec("trees"), 0.6, false, &[], Some("screen"), &[], None),
            "laptop" => desk_case(spec("trees"), 0.6, false, &[], Some("laptop"), &[], None),
            "shell" => desk_case(spec("trees"), 0.6, false, &[], None, &[("/u/burrow", true), ("/u/burrow-wt1", false)], None),
            "shell-idle" => desk_case(spec("empty"), 0.6, false, &[], None, &[("/Users/x/burrow", false)], Some("/Users/x/burrow")),
            _ => return None,
        })
    });
    check("paint-chair-", exact, |name| {
        let lit = name.ends_with('1');
        Some(on(16, 22, PLANK, |c| draw_chair(c, 6, 18, lit)))
    });
}

#[test]
fn the_fire_is_room_js_drawfire() {
    let e = env();
    let plant = snapshot(foot("plant", 0., 0., None), 2., |c| draw_kind(c, "plant", 0, 0, 0., &e));
    let board = snapshot(foot("board", 0., 0., None), 2., |c| draw_kind(c, "board", 0, 0, 0., &e));
    check("paint-fire-", exact, |name| {
        let n = name.strip_prefix("paint-fire-")?;
        let fire = |t: f64, feeding: bool, burst: f64, part: FirePart, burning: &[Burning]| {
            on(80, 80, PLANK, |c| draw_fire(c, 16, 30, t, feeding, burst, burning, part))
        };
        Some(match n {
            "0" => fire(0.4, false, -1., FirePart::All, &[]),
            "feed" => fire(1.1, true, -1., FirePart::All, &[]),
            "burst3" => fire(0.7, false, 0.3, FirePart::All, &[]),
            "burst8" => fire(0.7, false, 0.8, FirePart::All, &[]),
            "pit" => fire(0.4, false, -1., FirePart::Pit, &[]),
            "flames" => fire(0.4, true, 0.5, FirePart::Flames, &[]),
            _ if n.starts_with("wide-") => {
                let t: f64 = n.strip_prefix("wide-")?.parse().ok()?;
                let meet = snapshot(foot("meet", 0., 0., None), 2., |c| draw_kind(c, "meet", 0, 0, 0., &e));
                on(90, 80, PLANK, |c| draw_fire(c, 21, 30, 0.9, false, -1., &[Burning { img: &meet, x: 1., y: 2., t }], FirePart::All))
            }
            _ => {
                let t: f64 = n.strip_prefix("burn-")?.parse().ok()?;
                let b = [Burning { img: &plant, x: 2., y: 2., t }, Burning { img: &board, x: 44., y: 4., t: t + 0.04 }];
                fire(0.9, false, -1., FirePart::All, &b)
            }
        })
    });
}

#[test]
fn the_booth_is_room_js_drawbooth() {
    check("paint-booth-", exact, |name| {
        let (t, flash, armed) = match name.strip_prefix("paint-booth-")? {
            "armed" => (-1., -1., true),
            "flash" => (-1., 0.4, false),
            n => (n.strip_prefix('t')?.parse().ok()?, -1., false),
        };
        Some(on(30, 44, PLANK, |c| {
            draw_booth(c, 15, 40, false, t, flash, armed, 3.4, false);
            draw_booth(c, 15, 40, true, t, flash, armed, 3.4, false);
        }))
    });
    check("paint-gun-", exact, |name| {
        let out: i32 = name.strip_prefix("paint-gun-")?.parse().ok()?;
        Some(on(30, 44, PLANK, |c| {
            draw_gun_closet(c, 15, 40, false, out);
            draw_gun_closet(c, 15, 40, true, out);
        }))
    });
}

#[test]
fn the_wall_is_room_js() {
    check("paint-clock-", exact, |name| {
        let aw = |mode, left_ms, on| Some(Awake { mode, left_ms, on });
        let (time, a) = match name.strip_prefix("paint-clock-")? {
            "a" => (NOON, None),
            "b" => (EARLY, None),
            "work-on" => (NOON, aw(AwakeMode::Work, 0., true)),
            "work-off" => (NOON, aw(AwakeMode::Work, 0., false)),
            "for-2h" => (NOON, aw(AwakeMode::For, 2. * 3600e3, false)),
            "for-20m" => (EARLY, aw(AwakeMode::For, 20. * 60e3, false)),
            "for-8h" => (EARLY, aw(AwakeMode::For, 8. * 3600e3, false)),
            "for-gone" => (EARLY, aw(AwakeMode::For, -1000., false)),
            _ => return None,
        };
        Some(on(19, 19, WALLC, |c| draw_clock(c, 1, 1, a.as_ref(), time, false)))
    });
    check("paint-window-", exact, |name| {
        let hour = if name.ends_with("day") { 14 } else { 22 };
        Some(on(23, 20, WALLC, |c| draw_window(c, 1, 1, hour)))
    });
    check("paint-door-", exact, |name| {
        if let Some(i) = name.strip_prefix("paint-door-pic-") {
            return Some(door_picture(i.parse().ok()?, 3));
        }
        let (p, ajar, boarded) = match name.strip_prefix("paint-door-")? {
            "wood" => (WOOD, false, false),
            "p0-ajar" => (PAINT[0], true, false),
            "p3-boarded" => (PAINT[3], false, true),
            "p7-both" => (PAINT[7], true, true),
            _ => return None,
        };
        Some(on(20, 24, WALLC, |c| draw_door(c, 2, &p, ajar, boarded, 2)))
    });
    check("paint-floor", exact, |name| {
        let want = Image::from_png(&std::fs::read(golden().join(format!("{name}.png"))).ok()?).ok()?;
        Some(floor(want.w as i32, want.h as i32))
    });
    check("paint-closet-", exact, |name| Some(on(26, 28, WALLC, |c| draw_closet(c, 2, 2, name.ends_with('1')))));
    check("paint-switch-", exact, |name| Some(on(14, 20, WALLC, |c| draw_switchboard(c, 2, 3, name.ends_with('1')))));
}

#[test]
fn the_floor_is_room_js() {
    let used = |n: &str| n.parse::<f64>().ok();
    check("paint-coffee-", exact, |name| {
        let n = name.strip_prefix("paint-coffee-")?;
        Some(on(26, 26, PLANK, |c| match n {
            "brew" => draw_coffee(c, 3, 24, true, 0.3, Some(20.)),
            _ => draw_coffee(c, 3, 24, false, 0., used(n)),
        }))
    });
    check("paint-cooler-", exact, |name| {
        Some(on(14, 30, PLANK, |c| draw_cooler(c, 7, 28, used(name.strip_prefix("paint-cooler-").unwrap()))))
    });
    check("paint-printer-", exact, |name| {
        let n = name.strip_prefix("paint-printer-")?;
        if n == "pic" {
            return Some(printer_picture(100., 4).0);
        }
        Some(on(18, 20, PLANK, |c| draw_printer(c, 9, 17, n.parse().ok())))
    });
    check("paint-plant-", exact, |name| {
        let (i, k) = name.strip_prefix("paint-plant-")?.split_once('-')?;
        let stir = k.parse::<f64>().ok().map(|k| k * 125. + 60.);
        let i: u32 = i.parse().ok()?;
        Some(on(20, 34, PLANK, |c| draw_plant(c, 10, 32, i, stir)))
    });
    check("paint-rec-", exact, |name| {
        let n = name.strip_prefix("paint-rec-")?;
        let susan = |screen, t| Reception { n: 0, name: "Susan", color: css("#4a76b8"), screen: Some(screen), t };
        let o = match n {
            "linda" => Reception { n: 3, name: "Linda", color: css("#f06b72"), screen: None, t: 0. },
            "blank" => Reception { n: 8, name: "", color: css("#9a74d6"), screen: None, t: 0. },
            "off" => susan(RecScreen::Off, 1.3),
            "on" => susan(RecScreen::On, 1.3),
            "busy" => susan(RecScreen::Busy, 1.3),
            "asks" => susan(RecScreen::Asks, 0.6),
            _ => return None,
        };
        Some(on(50, 42, PLANK, |c| {
            draw_reception(c, 2, 40, &o, false);
            draw_reception(c, 2, 40, &o, true);
        }))
    });
    check("paint-pair-", exact, |name| {
        let (linked, t, over) = match name.strip_prefix("paint-pair-")? {
            "unlinked" => (false, 0., false),
            "linked" => (true, 0., false),
            "over-off" => (true, 0.1, true),
            "over-on" => (true, 0.2, true),
            _ => return None,
        };
        Some(on(30, 26, PLANK, |c| draw_pair(c, 15, 22, linked, t, over)))
    });
    check("paint-add-", exact, |name| Some(on(34, 20, PLANK, |c| draw_add(c, 2, 2, name.ends_with('1')))));
    check("paint-radio-", exact, |name| Some(on(18, 26, PLANK, |c| draw_radio(c, 9, 23, name.ends_with('1')))));
    check("paint-notes-", exact, |name| {
        let t: f64 = name.strip_prefix("paint-notes-")?.parse().ok()?;
        Some(on(18, 46, PLANK, |c| {
            draw_radio(c, 9, 43, true);
            draw_notes(c, 9, 43, t);
        }))
    });
    check("paint-neur-", exact, |name| Some(on(16, 22, PLANK, |c| draw_neuralyzer(c, 8, 20, name.ends_with('1')))));
    check("paint-table", exact, |_| Some(on(36, 34, PLANK, |c| draw_table(c, 18, 32))));
    check("paint-stool", exact, |_| Some(on(18, 18, PLANK, |c| draw_stool(c, 9, 16))));
    check("paint-chess-", exact, |name| {
        const MID: &str = "RNBQKB.RPPPP.PPP.....N......P......p...............pppp.ppprnbqkbnr";
        let b = if name.ends_with("mid") { Some(MID) } else { None };
        Some(on(26, 22, PLANK, |c| draw_chess(c, 13, 19, b)))
    });
    check("paint-pong-", exact, |name| {
        let ball = match name.strip_prefix("paint-pong-")? {
            "rest" => None,
            "0.2-0" => Some(PongBall { t: 0.2, from: false }),
            "0.7-1" => Some(PongBall { t: 0.7, from: true }),
            _ => return None,
        };
        Some(on(80, 34, PLANK, |c| {
            draw_rug(c, 40, 26);
            draw_pong(c, 40, 26, ball);
        }))
    });
    check("paint-mat", exact, |_| Some(on(36, 20, PLANK, |c| draw_mat(c, 18, 18))));
    check("paint-doghouse", exact, |_| Some(on(28, 26, PLANK, |c| draw_doghouse(c, 14, 23))));
    check("paint-board-ink", exact, |_| {
        let mut ink = Image::new(130, 55);
        {
            let mut k = ink.ctx();
            k.rect(0, 0, 130, 3, css("#2b2433"));
            k.rect(10, 10, 4, 30, css("#2b2433"));
            k.rect(60, 20, 12, 12, css("#e8505b"));
            k.rect(101, 41, 1, 1, css("#4a76b8"));
        }
        let cells = board_cells(&ink);
        Some(on(34, 30, PLANK, |c| draw_board(c, 17, 28, &cells)))
    });
}

#[test]
fn the_dog_is_room_js() {
    check("paint-dog-", exact, |name| {
        let (pose, wag) = name.strip_prefix("paint-dog-")?.split_once('-')?;
        Some((*dog_pic(pose, wag == "1")).clone())
    });
    check("paint-heart", exact, |_| Some(on(9, 7, PLANK, |c| heart(c, 4, 2))));
    check("paint-stick-", exact, |name| {
        let turn: usize = name.strip_prefix("paint-stick-")?.parse().ok()?;
        Some(on(11, 11, PLANK, |c| draw_stick(c, 5, 5, turn)))
    });
}

#[test]
fn figures_and_effects_are_room_js() {
    check("paint-sleeper-", exact, |name| {
        Some(if name.ends_with("bot") {
            let im = figure::body("demo0002", project_color("u1"), Arms::Down, Legs::Stand, false, false);
            on(40, 30, PLANK, |c| draw_sleeper(c, 20, 26, &im, Some(GOLD), false, 0.3))
        } else {
            let im = figure::body("rembert-15", css("#d97757"), Arms::Down, Legs::Stand, true, false);
            on(40, 40, PLANK, |c| draw_sleeper(c, 20, 36, &im, None, true, 0.3))
        })
    });
    let img = std::rc::Rc::new(bot());
    check("paint-part-", exact, |name| {
        let p = |kind, x, y, z, spin| Part { kind, x, y, z, spin, charred: false };
        let cut = |sx, sy, sw, sh| PartKind::Cut { img: img.clone(), sx, sy, sw, sh };
        let (part, fade) = match name.strip_prefix("paint-part-")? {
            "cog" => (p(PartKind::Cog { big: false }, 8., 20., 0., 0.5), 0.),
            "cog7" => (p(PartKind::Cog { big: true }, 15., 20., 4., 1.5), 0.),
            "cogc" => (Part { charred: true, ..p(PartKind::Cog { big: false }, 15., 20., 0., 2.2) }, 0.),
            "bolt" => (p(PartKind::Bolt, 15., 20., 3., 0.), 0.),
            "spring" => (p(PartKind::Spring, 15., 20., 0., 0.), 0.),
            "head" => (p(cut(1, 1, 20, 17), 15., 26., 2., 0.2), 0.),
            "head-flip" => (p(cut(1, 1, 20, 17), 15., 26., 2., 1.2), 0.),
            "leg" => (p(cut(7, 23, 4, 6), 15., 26., 0., 3.7), 0.),
            "fade" => (p(PartKind::Cog { big: false }, 15., 20., 0., 0.5), 0.4),
            _ => return None,
        };
        Some(on(30, 30, PLANK, |c| draw_part(c, &part, fade)))
    });
    check("paint-tinted", exact, |name| Some(if name.ends_with("gold") { tinted(&bot(), GOLD, 0.3) } else { tinted(&bot(), RED, 0.5) }));
    check("paint-dissolve-", exact, |name| Some(dissolve(&bot(), name.strip_prefix("paint-dissolve-")?.parse().ok()?)));
    check("paint-outlined-", exact, |name| {
        Some(match name.strip_prefix("paint-outlined-")? {
            "plant" => {
                on(24, 40, PLANK, |c| outlined(c, foot("plant", 12., 34., None), true, GOLD, 0., |o| draw_plant(o, 12, 34, 0, None)))
            }
            "booth" => on(26, 44, PLANK, |c| {
                outlined(c, foot("booth", 13., 38., None), true, RED, 0.5, |o| draw_booth(o, 13, 38, false, -1., -1., false, 0., false))
            }),
            "off" => on(24, 40, PLANK, |c| outlined(c, foot("plant", 12., 34., None), false, GOLD, 0., |o| draw_plant(o, 12, 34, 1, None))),
            _ => return None,
        })
    });
}

#[test]
fn dreams_are_room_js_dreamscene() {
    check("paint-dream-", exact, |name| {
        let (i, n) = name.strip_prefix("paint-dream-")?.split_once('-')?;
        let o = dream_scene(i.parse().ok()?);
        let n = n.parse().unwrap_or(o.len()).min(o.len());
        Some(on(65, 27, "#efebe8", |c| {
            for p in &o[..n] {
                c.rect(p.x, p.y, 1, 1, dream_pal(p.c).unwrap_or(Color::NONE));
            }
        }))
    });
}
