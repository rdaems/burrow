//! The office in the first person (room.js's `scene3`): what is in the room right now, for the gun closet's view
//! (burrow-fps) to make its 3d things of: the floor and what lies on it (an overlay, two texels a dot), what hangs on
//! the wall, every desk and piece of furniture with what it shows (the coffee left, a screen at work, the game on the
//! chess board, the painting on the easel), the lights (the fire, a booth in use), and everyone in the pose the room
//! gives them.

use crate::draw as D;
use crate::io::Input;
use crate::layout::{self, Layout, WALL, foot, kind_of, name_of, on_wall};
use crate::room::Room;
use crate::scene::color_of;
use crate::think::{booth_rise, booth_t, manager_seat};
use burrow_fps::{Desk, Face, Figure, Light, Paint, Piece, Robot, Scene, Tex, Thing, WallItem, WallKind};
use pix::{Color, Image};
use std::sync::Arc;

fn paint(p: D::Paint) -> Paint {
    Paint { hi: p.hi, top: p.top, d: p.d, k: p.k, plate: p.plate }
}

/// what the view keeps of the room between scenes: its builder, the floor overlay and what it was made of
#[derive(Default)]
pub struct Third {
    pub props: burrow_fps::Props,
    pub v: u64,
    floor_key: String,
    floor: Option<Tex>,
}

impl Room {
    /// the scene as it is now
    pub fn scene3(&mut self, inp: &Input) -> Option<Arc<Scene>> {
        let l = self.last.clone()?;
        let mut third = std::mem::take(&mut self.third);
        let s = self.build_scene(inp, &l, &mut third);
        self.third = third;
        Some(Arc::new(s))
    }

    fn build_scene(&mut self, inp: &Input, l: &Layout, th: &mut Third) -> Scene {
        let (w, h) = (self.size.w as f64, self.size.h as f64);
        let t = self.w.s.t;
        let now = inp.now;
        let st = &inp.state;
        let mut sc = Scene::empty(w, h, WALL as f64);
        th.v += 1;
        sc.v = th.v;
        sc.kills = self.w.s.kills as i32;
        let pieces: Vec<(String, String, i32, i32)> = l
            .pieces
            .iter()
            .filter(|p| p.u.is_none() && !on_wall(&p.key))
            .map(|p| (p.key.clone(), kind_of(&p.key).to_string(), p.at[0], p.at[1]))
            .collect();
        // the floor's overlay: the rug, the mats rolled out for a nap, the oil of the shot ones, the fire's pit, a soft
        // shadow under each piece; redrawn only when one of them changes
        let mats: Vec<(i32, i32)> = self.w.folk.iter().filter(|f| f.act.kind == "sleep" && f.act.there).filter_map(|f| f.act.spot.as_ref()).map(|s| (s.x as i32, s.y as i32)).collect();
        let fkey = format!(
            "{w}x{h}|{:?}|{mats:?}|{:?}|{:?}|{:?}|{:?}",
            l.pong.as_ref().map(|p| (p.x, p.y)),
            self.w.s.remains.iter().map(|(_, r)| (r.x as i32, r.y as i32, r.parts.len(), r.drops.len())).collect::<Vec<_>>(),
            l.fire.as_ref().map(|f| (f.x0, f.y0)),
            pieces.iter().map(|p| (&p.1, p.2, p.3)).collect::<Vec<_>>(),
            l.desks.iter().map(|d| (d.x0, d.y, d.w)).collect::<Vec<_>>(),
        );
        if th.floor.is_none() || fkey != th.floor_key {
            th.floor_key = fkey;
            let mut im = Image::new((w * 2.0) as u32, (h * 2.0) as u32);
            {
                let mut fx = im.ctx();
                fx.set_transform(2., 0., 0., 2., 0., 0.);
                let shade = Color::rgba(0, 0, 0, 77);
                let blob = |fx: &mut pix::Ctx, cx: f64, cy: f64, rx: f64, ry: f64| {
                    let mut p = pix::Path::new();
                    p.ellipse(cx, cy, rx, ry, 0., std::f64::consts::TAU);
                    fx.fill_path(&p, shade);
                };
                for (_, kind, x, y) in &pieces {
                    if kind != "fire" {
                        let r = foot(kind, [*x, *y], None);
                        blob(&mut fx, (r[0] + r[2]) as f64 / 2.0, (*y - 3) as f64, (r[2] - r[0]) as f64 / 2.0 + 1.0, 5.0);
                    }
                }
                for d in &l.desks {
                    blob(&mut fx, d.x0 as f64 + d.w as f64 / 2.0, (d.y - 6) as f64, d.w as f64 / 2.0 + 1.0, 7.0);
                }
                if let Some(p) = &l.pong {
                    D::draw_rug(&mut fx, p.x, p.y);
                }
                for (x, y) in &mats {
                    D::draw_mat(&mut fx, *x, *y);
                }
                for (_, r) in &self.w.s.remains {
                    let rx = (r.puddle * r.parts.len() as f64 / r.total.max(1) as f64).round() as i32;
                    if rx > 1 {
                        for (j, hw) in D::ell(rx, 3, 5).into_iter().enumerate() {
                            D::rect(&mut fx, r.x as i32 - hw, r.y as i32 - 2 + j as i32, 2 * hw, 1, Color::rgb(0x17151f));
                        }
                    }
                    for d in &r.drops {
                        D::rect(&mut fx, d.x as i32, d.y as i32, d.s, d.s, if d.hi { Color::rgb(0x4a4766) } else { Color::rgb(0x17151f) });
                    }
                }
                if let Some(f) = &l.fire {
                    let pit = th.props.pit(44.0, 18.0);
                    fx.draw_image_scaled(&pit.img, (f.x0 + 2) as f64, (f.y1 - 21) as f64, 44., 18.);
                }
            }
            th.floor = Some(Tex::new(im));
        }
        sc.floor = th.floor.clone().unwrap();
        // the wall: the doors in their paint (boarded while their machine is out of reach), the window, the clock, the
        // closet, the switchboard, where they hang
        let mut items: Vec<WallItem> = Vec::new();
        for d in &l.doors {
            let ok = st.machine(&d.mid).is_some_and(|m| m.error.is_none());
            items.push(WallItem { kind: WallKind::Door { paint: paint(self.paint_of(inp, &d.mid)), ok }, x0: d.x0 as f64, x1: (d.x0 + 16) as f64, y0: d.y0 as f64, y1: WALL as f64 });
        }
        for p in &l.windows {
            items.push(WallItem { kind: WallKind::Window { hour: inp.hour }, x0: p.x0 as f64, x1: p.x1 as f64, y0: p.y0 as f64, y1: p.y1 as f64 });
        }
        if let Some(c) = &l.clock {
            items.push(WallItem { kind: WallKind::Clock { hours: inp.hour, minutes: inp.minute, awake: st.awake.on }, x0: c.x as f64, x1: (c.x + 17) as f64, y0: c.y as f64, y1: (c.y + 17) as f64 });
        }
        for (b, k) in [(&l.closet, WallKind::Closet), (&l.settings, WallKind::Settings)] {
            if let Some(b) = b {
                items.push(WallItem { kind: k, x0: b.x0 as f64, x1: b.x1 as f64, y0: b.y0 as f64, y1: b.y1 as f64 });
            }
        }
        let wkey = format!("{:?}{}:{}{}", items.iter().map(|i| (format!("{:?}", i.kind).chars().take(40).collect::<String>(), i.x0, i.y0)).collect::<Vec<_>>(), inp.hour, inp.minute, st.awake.on);
        sc.wall_pic = th.props.wall(w, 40.0, WALL as f64, &items, &wkey);
        sc.items = items;
        sc.wkey = wkey;
        // the desks: their cabinet, desktop, screen (and laptops) and chairs, what each screen shows
        for d in &l.desks {
            let cut = crate::think::cut_off(st, &d.d.m); // (its machine out of reach: snow)
            let screen_of = |tree: &str| -> String {
                if cut {
                    return "snow".into();
                }
                let seated: Vec<&crate::world::Fig> = d
                    .seats
                    .iter()
                    .filter(|s| s.2 == tree)
                    .filter_map(|s| self.w.get(&s.0))
                    .filter(|f| f.legs == "sit" && !f.lying && (f.act.kind == "desk" || f.act.kind == "sleep") && f.b.as_ref().is_some_and(|b| b.alive))
                    .collect();
                if seated.iter().any(|f| f.busy) {
                    "busy".into()
                } else if seated.iter().any(|f| f.waiting) {
                    "waits".into()
                } else if !seated.is_empty() || inp.shells.iter().any(|x| x.m == d.d.m && x.dir == if tree.is_empty() { d.d.dir.as_str() } else { tree }) {
                    "on".into()
                } else {
                    "off".into()
                }
            };
            let desk = Desk {
                x0: d.x0 as f64,
                y: d.y as f64,
                w: d.w as f64,
                cab: d.cab as f64,
                label: d.label.clone(),
                paint: paint(self.paint_of(inp, &d.d.m)),
                color: d.color.hex(),
                t_now: t,
                seats: d.seats.iter().map(|s| s.1 as f64).collect(),
                mugs: d.seats.iter().filter(|(id, _, _)| self.w.s.mugs.get(id).is_some_and(|&m| m > t)).filter_map(|(id, x, _)| self.w.get(id).map(|f| (*x as f64, color_of(f).hex()))).collect(),
                screen: screen_of(""),
                laptops: d.trees.iter().map(|tr| screen_of(tr)).collect(),
            };
            // whose its blocks are: the desk's (what a shot hits, what the crosshair names); an empty one breaks
            let mut parts = th.props.desk(&desk);
            let breaks = inp.hardcore() && d.seats.is_empty();
            for b in parts.boxes.iter_mut() {
                (b.desk, b.label, b.breaks) = (Some(d.d.unit.clone()), Some(d.label.clone()), breaks);
            }
            for x in parts.things.iter_mut().filter(|x| x.bot.is_none() && x.rem.is_none()) {
                (x.desk, x.label, x.breaks) = (Some(d.d.unit.clone()), Some(d.label.clone()), breaks);
            }
            sc.add(parts);
        }
        // every piece of furniture with what it shows; the fire's flames (a light of their own), its pit on the floor
        let left = |u: Option<&crate::model::Limit>| u.map(|u| (1.0 - u.used as f64 / 100.0).max(0.0)).unwrap_or(0.0);
        for (key, kind, x, y) in &pieces {
            if kind == "fire" {
                let f = l.fire.as_ref().unwrap();
                let fl = th.props.flames((t * 8.0).floor() as u32 % 4, !self.w.s.burning.is_empty() || now - self.w.s.flare < 1500.0);
                sc.things.push(Thing { x: (f.x0 + 24) as f64, y: (f.y1 - 12) as f64, pic: Some(Face::whole(fl.tex.clone())), ax: fl.ax, ay: fl.ay, res: fl.res, piece: Some(key.clone()), label: Some("fire".into()), ..Default::default() });
                sc.lights.push(Light { x: (f.x0 + 24) as f64, y: (f.y1 - 12) as f64, z: 9.0, r: 140.0, col: [1.0, 0.8, 0.58], k: 0.9, flicker: true, muzzle: false });
                sc.solid.push([(f.x0 + 3) as f64, (f.y1 - 20) as f64, (f.x1 - 3) as f64, (f.y1 - 3) as f64]);
                continue;
            }
            let mut p = Piece { kind: kind.clone(), key: key.clone(), x: *x as f64, y: *y as f64, t_now: t, ..Default::default() };
            match kind.as_str() {
                "coffee" => {
                    p.level = Some(left(st.usage.session.as_ref()));
                    p.brewing = self.w.folk.iter().any(|f| f.act.kind == "coffee" && f.act.stage == 1);
                }
                "cooler" => p.level = Some(left(st.usage.week.as_ref())),
                "printer" => {
                    let job = self.w.folk.iter().find(|f| f.act.kind == "print" && f.act.stage == 1).map(|f| f.act.wait);
                    let since = (now - self.w.s.print) / 1000.0;
                    p.printing = job.or((since < 3.0).then_some(since));
                }
                "manager" => {
                    let m = st.managers.iter().find(|m| m.key == *key).cloned().unwrap_or_default();
                    let live: Vec<&str> = st.bots.iter().map(|b| b.id.as_str()).collect();
                    p.color = Some(m.color.clone());
                    if m.vacant {
                        p.screen = m.session.then(|| "off".into());
                    } else {
                        p.name = m.name.clone();
                        p.screen = m.session.then(|| (if m.busy { "busy" } else if m.waiting { "asks" } else if m.alive { "on" } else { "off" }).to_string());
                        p.notes = if m.style == "notes" { inp.digest.as_ref().map(|d| d.sessions.iter().filter(|id| live.contains(&id.as_str())).count() as u32).unwrap_or(0) } else { 0 };
                    }
                }
                "booth" => {
                    let inside = self.w.folk.iter().find(|f| f.act.kind == "doom" && f.act.at.as_ref() == Some(key));
                    p.gun = inp.hardcore();
                    p.out = self.w.folk.iter().filter(|f| f.carry == Some("gun")).count() as u32 + 1;
                    p.t = Some(booth_t(inside));
                }
                "neuralyzer" => p.flash = self.w.s.flash.is_some_and(|f| now - f.2 < 400.0),
                "radio" => p.on = st.settings.sound,
                "chess" => {
                    p.board = st.chess.as_ref().filter(|g| g.result.is_none()).map(|g| g.board.clone()).unwrap_or_else(|| D::START.to_string());
                    p.seats = [0, 1].iter().filter_map(|&i| l.chess_seat(i)).map(|s| (s.x, s.y)).collect();
                }
                "meet" => p.seats = [0, 1].iter().filter_map(|&i| l.meet_seat(i)).map(|(s, _)| (s.x, s.y)).collect(),
                "board" => {
                    if let Some(a) = &self.w.s.art {
                        p.art = Some(Tex::new(a.img.clone()));
                        p.art_v = self.w.s.art_v;
                    }
                }
                "plant" => p.variant = key.trim_start_matches(|c: char| !c.is_ascii_digit()).parse::<u32>().unwrap_or(0) % 2,
                "pair" => p.linked = st.pair.contains_key(key),
                "pumpkin" if self.spooky => {
                    // its candle
                    sc.lights.push(Light { x: *x as f64, y: (*y - 5) as f64, z: 5.0, r: 70.0, col: [1.0, 0.78, 0.5], k: 0.5, flicker: true, muzzle: false });
                }
                _ => {}
            }
            // whose its blocks and pictures are: the piece's (a shot breaks what the fire would take, not the gun closet
            // your shells come from)
            let breaks = inp.hardcore() && !layout::fixed(key) && kind != "booth";
            if let Some(parts) = th.props.build(&p) {
                let label = if kind == "booth" && inp.hardcore() { "E: reload".to_string() } else { name_of(kind).to_string() };
                let mut parts = parts;
                for th_ in parts.things.iter_mut() {
                    th_.label.get_or_insert(label.clone());
                    th_.reload = kind == "booth";
                    th_.piece.get_or_insert(key.clone());
                    th_.breaks = breaks;
                }
                for b in parts.boxes.iter_mut() {
                    (b.piece, b.label, b.reload, b.breaks) = (Some(key.clone()), Some(label.clone()), kind == "booth", breaks);
                }
                sc.add(parts);
                continue;
            }
            // anything else: the room's own picture of it, standing where it stands
            let r = foot(kind, [*x, *y], None);
            let pad = 1;
            let (cw, ch) = (r[2] - r[0] + 2 * pad, r[3] - r[1] + 2 * pad);
            let mut im = Image::new(cw.max(1) as u32, ch.max(1) as u32);
            {
                let mut c = im.ctx();
                c.translate((pad - r[0]) as f64, (pad - r[1]) as f64);
                let env = D::KindEnv { clock: self.clock(inp), door: D::WOOD };
                match kind.as_str() {
                    "pumpkin" => D::draw_pumpkin(&mut c, *x, *y, self.spooky, t), // (as it stands, lit or not)
                    _ => D::draw_kind(&mut c, kind, *x, *y, t, &env),
                }
            }
            let cx = (r[0] + r[2]) as f64 / 2.0;
            sc.things.push(Thing {
                piece: Some(key.clone()),
                x: cx,
                y: (*y - 3) as f64,
                pic: Some(Face::whole(Tex::new(im))),
                ax: cx - r[0] as f64 + pad as f64,
                ay: (*y + 1 - r[1] + pad) as f64,
                res: 1.0,
                label: Some(name_of(kind).to_string()),
                breaks,
                ..Default::default()
            });
            sc.solid.push([(r[0] + 2) as f64, r[1].max(*y - 10) as f64, (r[2] - 2) as f64, (*y + 2) as f64]);
        }
        // everyone but you (you are the eyes): a model of each, in the pose the room gives it, seen from whichever of its
        // eight sides faces you; lying on a mat; in the booth going up in its light; a ? over one that waits
        for f in &self.w.folk {
            if f.id == "you" || f.held || f.b.is_none() {
                continue;
            }
            let b = f.b.as_ref().unwrap();
            let rob = Robot { seed: f.seed.clone(), color: color_of(f).hex(), femme: f.staff, arms: f.arms.to_string(), legs: f.legs.to_string(), red: f.red };
            let name = Some(b.name.clone());
            let bot = (!f.human).then(|| f.id.clone());
            if f.lying {
                let lf = th.props.lying(&rob);
                sc.things.push(Thing { key: Some(f.id.clone()), bot, staff: f.staff, name: name.clone(), label: name, x: f.x, y: f.y - 9.0, pic: Some(Face::whole(lf.tex)), ax: lf.ax, ay: lf.ay, res: lf.res, ..Default::default() });
                continue;
            }
            let bt = booth_t(Some(f));
            let seat = manager_seat(l, crate::think::booth_of(l, f));
            let at_desk = f.staff && f.sat && (f.x - seat.x).hypot(f.y - seat.y) < 2.0; // an office manager at her counter faces the room
            sc.things.push(Thing {
                key: Some(f.id.clone()),
                bot,
                staff: f.staff,
                name: name.clone(),
                label: name,
                x: f.x,
                y: f.y,
                fig: Some(Figure::Robot(rob)),
                face: if f.dir < 0.0 { -1.0 } else { 1.0 },
                facing: at_desk.then_some(std::f64::consts::FRAC_PI_2),
                seated: f.legs == "sit",
                dissolve: bt > 0.0 && booth_rise(bt) > 0.0,
                ..Default::default()
            });
            if f.waiting && f.act.kind != "doom" {
                sc.things.push(Thing { key: Some(format!("?{}", f.id)), x: f.x, y: f.y + 0.1, z: 28.0, pic: Some(Face::whole(question())), ax: 13.5, ay: 30.0, res: 3.0, ..Default::default() });
            }
        }
        // what is left of a shot bot: its pieces where they fly and land, cut from its model seen from the front
        for (id, r) in &self.w.s.remains {
            for (i, p) in r.parts.iter().enumerate() {
                let mut th_ = Thing { key: Some(format!("{id}/{i}")), rem: (!r.back).then(|| id.clone()), name: Some(r.name.clone()), x: p.x, y: p.y, z: p.z, flip: (p.spin.floor() as i64) % 2 == 1, ..Default::default() };
                if p.img.is_some() {
                    let rob = Robot { seed: r.seed.clone(), color: r.color.unwrap_or(Color::rgb(0x8b93ad)).hex(), femme: r.staff, arms: "down".into(), legs: "stand".into(), red: false };
                    let fr = th.props.figure(&Figure::Robot(rob), 0);
                    let s = fr.res;
                    let sx = (fr.ax + (p.sx as f64 - 11.0) * s).round().max(0.0);
                    let sy = (fr.ay + (p.sy as f64 - 27.0) * s).round().max(0.0);
                    th_.pic = Some(Face { tex: fr.tex.clone(), sx, sy, sw: (fr.w - sx).min(p.w as f64 * s), sh: (fr.h - sy).min(p.h as f64 * s), glow: false });
                    th_.ax = p.w as f64 * s / 2.0;
                    th_.ay = p.h as f64 * s;
                    th_.res = s;
                } else {
                    let mut im = Image::new(8, 8);
                    {
                        let mut c = im.ctx();
                        let part = D::Part {
                            kind: match p.kind {
                                "cog" => D::PartKind::Cog { big: p.big },
                                "bolt" => D::PartKind::Bolt,
                                _ => D::PartKind::Spring,
                            },
                            x: 4.0,
                            y: 8.0,
                            z: 0.0,
                            spin: p.spin,
                            charred: p.charred,
                        };
                        D::draw_part(&mut c, &part, 0.0);
                    }
                    th_.pic = Some(Face::whole(Tex::new(im)));
                    th_.ax = 4.0;
                    th_.ay = 8.0;
                    th_.res = 1.0;
                }
                sc.things.push(th_);
            }
        }
        if let Some(dg) = &self.w.s.dog
            && !dg.inside
            && !dg.leaving
            && !dg.held
        {
            sc.things.push(Thing { key: Some("dog".into()), x: dg.x, y: dg.y, fig: Some(Figure::Dog { pose: dg.pose.to_string(), wag: dg.wag != 0 }), face: if dg.dir < 0.0 { -1.0 } else { 1.0 }, ..Default::default() });
        }
        if let Some(bl) = &self.w.s.ball {
            sc.things.push(Thing { key: Some("ball".into()), x: bl.x, y: bl.y, z: bl.h, pic: Some(Face::whole(ball())), ax: 6.0, ay: 12.0, res: 4.0, ..Default::default() });
        }
        for sh in self.w.s.shots.iter().filter(|sh| !sh.hit && !sh.dust && inp.now - sh.t0 < 140.0) {
            // a mutineer's shot lights up round it
            sc.lights.push(Light { x: sh.x, y: sh.y + 9.0, z: 12.0, muzzle: true, ..Default::default() });
        }
        sc
    }
}

/// the ? over one that waits, three texels a dot
fn question() -> Tex {
    thread_local!(static Q: Tex = {
        let mut im = Image::new(27, 30);
        {
            let mut x = im.ctx();
            x.scale(3., 3.);
            x.rect(0, 0, 9, 9, Color::rgb(0x2b2433));
            x.rect(1, 1, 7, 7, Color::rgb(0xf4f6fb));
            x.rect(1, 8, 1, 1, Color::rgb(0xf4f6fb));
            x.rect(0, 9, 3, 1, Color::rgb(0x2b2433));
            crate::font::tiny(&mut x, "?", 3., 2., Color::rgb(0x2b2433));
        }
        Tex::new(im)
    });
    Q.with(|q| q.clone())
}

/// the football, four texels a dot
fn ball() -> Tex {
    thread_local!(static B: Tex = {
        let mut im = Image::new(12, 12);
        {
            let mut x = im.ctx();
            x.fill_circle(6., 6., 5.5, Color::rgb(0xe8ecf0));
            x.fill_circle(7., 7.5, 3.5, Color::rgb(0xb5bfc7));
            x.rect(3, 3, 2, 2, Color::rgb(0xffffff));
        }
        Tex::new(im)
    });
    B.with(|b| b.clone())
}
