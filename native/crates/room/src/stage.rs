//! What the room draws each frame between the floor and the tip (room.js's `draw`, its middle): the wall's pieces,
//! the desks, every piece of furniture and everyone, back to front, each with its hot spot, its outline under the
//! pointer, its tip; what is held over the fire washed red.

use crate::avatar::c as C;
use crate::draw::{self as D, Burning as DBurning, FirePart, PongBall, RecScreen, Reception};
use crate::figure::{Arms, Legs, body};
use crate::font::tiny;
use crate::io::{Input, Output};
use crate::layout::{ADD_W, FIRE_H, LAP, Layout, WALL, foot, jround, kind_of, name_of, on_wall};
use crate::room::{Hot, Hovered, Room, Tip};
use crate::scene::{GOLD, RED, color_of, flash_a};
use crate::think::{self, booth_t, in_fire};
use pix::{Color, Ctx};
use serde_json::json;
use std::collections::HashSet;

/// something that stands, drawn in order of its front (y): a figure, or a painter; `covers` are its solid parts
/// (a figure behind them is not under the pointer there)
enum What {
    Fig(usize),
    Paint(Box<dyn Fn(&mut Ctx)>),
}

fn rf(r: [i32; 4]) -> [f64; 4] {
    [r[0] as f64, r[1] as f64, r[2] as f64, r[3] as f64]
}

impl Room {
    /// a machine's paint: the one picked in the doors' panel, else this machine wood and the others in their order
    pub fn paint_of(&self, inp: &Input, mid: &str) -> D::Paint {
        let door = inp.state.machines.iter().position(|m| m.id == mid).unwrap_or(0) as i32;
        let i = inp.state.paint.get(mid).copied().unwrap_or(door - 1);
        D::paint_at(i)
    }

    pub(crate) fn draw_room(&mut self, ctx: &mut Ctx, inp: &Input, out: &mut Output, l: &Layout) {
        let t = self.w.s.t;
        let now = inp.now;
        let s = &inp.state;
        let mut hl: HashSet<String> = inp.hl.iter().cloned().collect();
        let mut tip: Option<Tip> = None;
        let set_tip = |tip: &mut Option<Tip>, text: String, x: f64, y: f64| *tip = Some(Tip { text, x, y });
        let clip = |t: &str, room: i32| {
            let n = ((room + 1) / 4).max(0) as usize;
            let m: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
            if m.chars().count() > n { format!("{}…", m.chars().take(n.saturating_sub(1)).collect::<String>()) } else { m }
        };
        let say = |u: Option<&crate::model::Limit>, what: &str| match u {
            Some(u) => format!("{}% {what}", (100 - u.used).max(0)),
            None => what.to_string(),
        };
        let heldbot = self.held.as_ref().and_then(|id| self.w.get(id)).filter(|f| f.is_bot() && self.pointer.is_some()).map(|f| f.id.clone());
        let booth_under = heldbot.as_ref().and_then(|id| think::booth_at(l, self.w.get(id).unwrap())).cloned();
        let moving = self.moving.clone();
        self.danger_now = booth_under.is_some() || moving.as_ref().is_some_and(|m| l.fire.is_some() && in_fire(l, &m.key, m.at, m.u));
        let danger_now = self.danger_now;
        let danger = |key: &str| danger_now && moving.as_ref().is_some_and(|m| m.key == key);
        let tint_of = |key: &str| if danger(key) { flash_a(t) } else { 0.0 };
        let hp = if self.moving.is_none() && self.held.is_none() && self.carrying.is_none() && !self.w.s.dog_held && !self.dog_rect.is_some_and(|r| self.over(r[0], r[1], r[2], r[3])) {
            self.piece_at(self.pointer)
        } else {
            None
        };
        let on = |key: &str| hp.as_ref().is_some_and(|p| p.key == key);
        let lit = |key: &str| on(key) && crate::layout::CLICK.contains(&kind_of(key));
        // whoever is at what is dragged rides along: done in step

        // the wall: the doors, one a machine, where they hang; ajar while someone goes through, boarded up while out
        // of reach. One being carried is drawn over everything else, washed red over the fire
        let mut aloft: Option<Box<dyn Fn(&mut Ctx)>> = None;
        for dr in &l.doors {
            let m = s.machines.iter().find(|m| m.id == dr.mid).cloned().unwrap_or_default();
            let (x0, y0) = (dr.x0, dr.y0);
            let h = self.hot(x0 as f64, y0 as f64, (x0 + 16) as f64, (y0 + 20) as f64, json!({"kind": "doors"}));
            let ajar = now - self.w.s.door_at.get(&m.id).copied().unwrap_or(-1e9) < 900.0
                || self.w.folk.iter().any(|f| f.b.as_ref().is_some_and(|b| b.m == m.id) && f.leaving && f.y < (WALL + 12) as f64);
            let p = self.paint_of(inp, &m.id);
            let boarded = m.error.is_some();
            let r = rf(foot("door", [x0, y0], None));
            let paint = move |c: &mut Ctx| D::draw_door(c, x0, &p, ajar && !boarded, boarded, y0);
            if moving.as_ref().is_some_and(|mv| mv.key == dr.key) {
                let (dg, tn) = (danger(&dr.key), tint_of(&dr.key));
                aloft = Some(Box::new(move |c: &mut Ctx| D::outlined(c, r, dg, RED, tn, &paint)));
            } else {
                D::outlined(ctx, r, h, GOLD, 0.0, paint);
            }
            if h {
                for b in &s.bots {
                    if b.m == m.id {
                        hl.insert(b.id.clone());
                    }
                }
                let text = match &m.error {
                    Some(e) => format!("{}: {e}", m.name),
                    None => m.name.clone(),
                };
                set_tip(&mut tip, clip(&text, self.size.w - 8), (x0 + 8) as f64, 20.0);
            }
        }
        if let Some((at, mid, kind, t0)) = self.pending.clone() {
            if mid.as_ref().is_some_and(|mid| s.machines.iter().any(|m| m.id == *mid)) || now - t0 > 10000.0 {
                self.pending = None;
            } else {
                let env = D::KindEnv { clock: self.clock(inp), door: D::paint_at(s.machines.len() as i32 - 1) };
                D::draw_kind(ctx, &kind, at[0], at[1], t, &env);
            }
        }
        for win in &l.windows {
            let h = self.hot(win.x0 as f64, win.y0 as f64, win.x1 as f64, win.y1 as f64, json!({"kind": "outside", "key": win.key}));
            let (x0, y0, hour) = (win.x0, win.y0, inp.hour);
            let r = [win.x0 as f64, win.y0 as f64, win.x1 as f64, win.y1 as f64];
            let paint = move |c: &mut Ctx| D::draw_window(c, x0, y0, hour);
            if moving.as_ref().is_some_and(|mv| mv.key == win.key) {
                let (dg, tn) = (danger(&win.key), tint_of(&win.key));
                aloft = Some(Box::new(move |c: &mut Ctx| D::outlined(c, r, dg, RED, tn, &paint)));
            } else {
                D::outlined(ctx, r, h, GOLD, 0.0, paint);
            }
            if h {
                set_tip(&mut tip, "outside".into(), (win.x0 + win.x1) as f64 / 2.0, (win.y1 + 10) as f64);
            }
        }
        if let Some(ck) = l.clock.clone() {
            // the clock keeps this computer awake, as Amphetamine does: its face says how, yellowed
            let h = self.hot(ck.x as f64, ck.y as f64, (ck.x + 17) as f64, (ck.y + 17) as f64, json!({"kind": "clock"}));
            let a = &s.awake;
            let awake = D::Awake {
                mode: match a.mode.as_str() {
                    "work" => D::AwakeMode::Work,
                    "for" => D::AwakeMode::For,
                    _ => D::AwakeMode::Off,
                },
                left_ms: (a.until - inp.epoch_ms) as f64,
                on: a.on,
            };
            let time = self.clock(inp);
            let (x0, y0, still) = (ck.x, ck.y, inp.still);
            let r = rf(foot("clock", [x0, y0], None));
            let paint = move |c: &mut Ctx| D::draw_clock(c, x0, y0, Some(&awake), time, still);
            if moving.as_ref().is_some_and(|mv| mv.key == ck.key) {
                let (dg, tn) = (danger(&ck.key), tint_of(&ck.key));
                aloft = Some(Box::new(move |c: &mut Ctx| D::outlined(c, r, dg, RED, tn, &paint)));
            } else {
                D::outlined(ctx, r, h, GOLD, 0.0, paint);
            }
            if h {
                let tm = ((a.until - inp.epoch_ms).max(0) as f64 / 60000.0).round() as i64;
                let (hh, mm) = (tm / 60, tm % 60);
                let hm = format!("{:02}:{:02}", inp.hour, inp.minute);
                let more = match a.mode.as_str() {
                    "for" => format!(" - awake {}{mm}m more", if hh > 0 { format!("{hh}h ") } else { String::new() }),
                    "work" if a.on => " - awake, local bots at work".into(),
                    "work" => " - awake when local bots work".into(),
                    _ => String::new(),
                };
                set_tip(&mut tip, hm + &more, (ck.x + 8) as f64, (WALL + 10) as f64);
            }
        }
        if let Some(cl) = l.closet.clone() {
            // the closet, against the wall: the furniture that is not in the room; a click opens it (on the right)
            let h = self.hot(cl.x0 as f64, cl.y0 as f64, cl.x1 as f64, cl.y1 as f64, json!({"kind": "closet"}));
            let open = inp.view.as_deref() == Some("closet");
            D::outlined(ctx, rf([cl.x0, cl.y0, cl.x1, cl.y1]), h, GOLD, 0.0, |c| D::draw_closet(c, cl.x0, cl.y0, open));
            if h {
                set_tip(&mut tip, "furniture".into(), (cl.x0 + 11) as f64, (cl.y0 + 4) as f64);
            }
        }
        if let Some(b) = l.settings.clone() {
            // the switchboard on the wall: burrow's own settings; a click opens them
            let h = self.hot(b.x0 as f64, b.y0 as f64, b.x1 as f64, b.y1 as f64, json!({"kind": "settings"}));
            let open = inp.view.as_deref() == Some("settings");
            D::outlined(ctx, rf([b.x0, b.y0, b.x1, b.y1]), h, GOLD, 0.0, |c| D::draw_switchboard(c, b.x0, b.y0, open));
            if h {
                set_tip(&mut tip, "settings".into(), (b.x0 + 7) as f64, (b.y0 + 4) as f64);
            }
        }
        // a bot and its seat at its project's desk are one thing: the pointer on either lights up both (and its
        // session), and a click on either opens it
        if let Some(id) = &self.hover_id {
            hl.insert(id.clone());
        }
        let mut seat_hover: Option<(String, i32, i32)> = None;
        if self.moving.is_none() && self.held.is_none() {
            for d in &l.desks {
                for (id, x, _) in &d.seats {
                    if self.over((x - 3) as f64, (d.y - 16) as f64, (x + 9) as f64, (d.y + 1) as f64) {
                        seat_hover = Some((id.clone(), *x, d.y));
                    }
                }
            }
        }
        if let Some((id, _, _)) = &seat_hover {
            hl.insert(id.clone());
        }
        // flat on the floor: the empty cabinet that adds a project, the mats rolled out for a nap, the ping-pong rug
        if let Some(a) = l.add.clone() {
            let h = self.hot((a.x0 - 2) as f64, (a.y0 - 10) as f64, (a.x1 + 2) as f64, (a.y1 + 1) as f64, json!({"kind": "addDir"}));
            let dg = danger(&a.key);
            D::outlined(ctx, rf(foot("add", [a.x0, a.y1], None)), h || dg, if dg { RED } else { GOLD }, tint_of(&a.key), |c| D::draw_add(c, a.x0, a.y0, h));
            if h {
                set_tip(&mut tip, "project".into(), (a.x0 + ADD_W / 2) as f64, (a.y0 - 2) as f64);
            }
        }
        for f in &self.w.folk {
            if f.act.kind == "sleep" && f.act.there
                && let Some(at) = &f.act.spot
            {
                D::draw_mat(ctx, at.x as i32, at.y as i32);
            }
        }
        if let Some(p) = &l.pong {
            D::draw_rug(ctx, p.x, p.y);
        }

        // everything that stands, back to front
        let mut items: Vec<(f64, What, Vec<[f64; 4]>)> = Vec::new();
        if let Some(a) = aloft {
            items.push((1e9, What::Paint(a), vec![]));
        }
        for (i, f) in self.w.folk.iter().enumerate() {
            if !f.lying && !f.held {
                items.push((f.y, What::Fig(i), vec![]));
            }
        }
        let shells = &inp.shells;
        let hs = inp.hl_shell.clone();
        for d in &l.desks {
            let p = self.paint_of(inp, &d.d.m);
            let hots = D::desk_hots(d.x0, d.y, d.cab, d.trees.len());
            let on_plate = self.hot_r(hots.plate, json!({"kind": "desk", "unit": d.d.unit}));
            let on_screen = self.hot_r(hots.screen, json!({"kind": "shell", "m": d.d.m, "dir": d.d.dir}));
            let (sx, sy) = D::screen_at(d.x0, d.cab, d.trees.len(), d.y);
            let gw = 22 + d.trees.len() as i32 * LAP;
            let where_ = |dir: &str| {
                let home = regex_home(dir);
                let m = s.machines.iter().find(|m| m.id == d.d.m && !d.d.m.is_empty()).map(|m| format!("{}:", m.name)).unwrap_or_default();
                format!("{m}{}", D::abbrev(&home))
            };
            if on_screen {
                for (id, _, _) in &d.seats {
                    hl.insert(id.clone());
                }
                set_tip(&mut tip, where_(&d.d.dir), (sx + 11) as f64, (sy - 1) as f64);
                self.screen_hover = Some((d.d.m.clone(), d.d.dir.clone()));
            }
            let on_plus = self.hot_r(hots.plus, json!({"kind": "spawn", "m": d.d.m, "dir": d.d.dir}));
            if on_plus {
                set_tip(&mut tip, "new bot".into(), (sx + gw + 4) as f64, (sy - 2) as f64);
            }
            if on_plate {
                set_tip(&mut tip, if d.d.name.is_empty() { d.label.clone() } else { d.d.name.clone() }, (d.x0 + d.cab / 2) as f64, (d.y - 11) as f64);
            }
            let mut trees = Vec::new();
            for (k, tr) in d.trees.iter().enumerate() {
                let on = self.hot_r(hots.laptops[k], json!({"kind": "shell", "m": d.d.m, "dir": tr}));
                let mine: Vec<&String> = d.seats.iter().filter(|st| st.2 == *tr).map(|st| &st.0).collect();
                if on {
                    for id in &mine {
                        hl.insert((*id).clone());
                    }
                    set_tip(&mut tip, format!("{}: a worktree", tr.rsplit('/').next().unwrap_or(tr)), (hots.laptops[k][0] + 6) as f64, (hots.laptops[k][1] - 1) as f64);
                    self.screen_hover = Some((d.d.m.clone(), tr.clone()));
                }
                trees.push((tr.clone(), on, mine.iter().map(|x| (*x).clone()).collect::<Vec<_>>()));
            }
            let screen_of = |tree: &str| -> D::Screen {
                let seated: Vec<&crate::world::Fig> = d
                    .seats
                    .iter()
                    .filter(|st| st.2 == tree)
                    .filter_map(|st| self.w.get(&st.0))
                    .filter(|f| f.legs == "sit" && !f.lying && (f.act.kind == "desk" || f.act.kind == "sleep") && f.b.as_ref().is_some_and(|b| b.alive))
                    .collect();
                let sh = shells.iter().find(|x| x.m == d.d.m && x.dir == if tree.is_empty() { d.d.dir.as_str() } else { tree });
                D::Screen {
                    seated: !seated.is_empty(),
                    busy: seated.iter().any(|f| f.busy),
                    waits: seated.iter().any(|f| f.waiting),
                    shell: sh.is_some(),
                    shell_busy: sh.is_some_and(|x| x.busy),
                }
            };
            let screen = screen_of("");
            let tree_screens: Vec<D::Screen> = d.trees.iter().map(|tr| screen_of(tr)).collect();
            let mugs: Vec<(i32, Color)> = d.seats.iter().filter(|(id, _, _)| self.w.s.mugs.get(id).is_some_and(|&m| m > t) && self.w.get(id).is_some()).map(|(id, x, _)| (*x, color_of(self.w.get(id).unwrap()))).collect();
            let shell_lit = hs.as_ref().is_some_and(|h| h.m == d.d.m && h.dir == d.d.dir);
            let (x0, y, w, cab, label, dir, color) = (d.x0, d.y, d.w, d.cab, d.label.clone(), d.d.dir.clone(), d.color);
            let seats: Vec<i32> = d.seats.iter().map(|s| s.1).collect();
            let tree_lit: Vec<bool> = trees.iter().map(|(tr, _, mine)| hs.as_ref().is_some_and(|h| h.m == d.d.m && h.dir == *tr) || mine.iter().any(|id| hl.contains(id))).collect();
            let r = rf(foot(&d.key, d.at, Some(d.w)));
            let (dg, tn) = (danger(&d.key), tint_of(&d.key));
            let screen_lit = false; // (room.js draws a desk with its `lit` left out)
            let paint = move |c: &mut Ctx| {
                let td: Vec<D::Tree> = trees.iter().enumerate().map(|(k, (tr, on, _))| D::Tree { dir: tr.as_str(), screen: tree_screens[k], on: *on, lit: tree_lit[k] }).collect();
                let desk = D::Desk {
                    x0,
                    y,
                    w,
                    cab,
                    label: &label,
                    dir: &dir,
                    color,
                    paint: p,
                    seats: seats.clone(),
                    mugs: mugs.clone(),
                    screen,
                    trees: td,
                    on_plate,
                    on_screen,
                    on_plus,
                    lit: screen_lit,
                    shell_lit,
                };
                D::outlined(c, r, dg, RED, tn, |c| D::draw_desk(c, &desk, t));
            };
            items.push((
                if dg { 1e9 } else { (d.y - 1) as f64 },
                What::Paint(Box::new(paint)),
                vec![
                    [sx as f64, sy as f64, (sx + gw) as f64, (sy + 18) as f64],
                    [d.x0 as f64, (d.y - 15) as f64, (d.x0 + d.cab) as f64, (d.y + 1) as f64],
                    [(d.x0 + d.cab - 2) as f64, (d.y - 12) as f64, (d.x0 + d.w) as f64, (d.y - 3) as f64],
                ],
            ));
            for (id, x, _) in &d.seats {
                let (x, y, lit) = (*x, d.y, hl.contains(id));
                items.push(((d.y as f64) - 0.5, What::Paint(Box::new(move |c: &mut Ctx| D::draw_chair(c, x, y, lit))), vec![]));
            }
        }
        // every piece: outlined under the pointer; what it does, on top of that. Held over the fire it is drawn over
        // everything (the flames would hide it), washed red
        let piece = |items: &mut Vec<(f64, What, Vec<[f64; 4]>)>, key: &str, kind: &str, x: i32, y: i32, f: Box<dyn Fn(&mut Ctx)>, dz: f64| {
            let r = rf(foot(kind, [x, y], None));
            let (dg, ln, tn) = (danger(key), lit(key), tint_of(key));
            items.push((if dg { 1e9 } else { y as f64 - dz }, What::Paint(Box::new(move |c: &mut Ctx| D::outlined(c, r, ln || dg, if dg { RED } else { GOLD }, tn, &f))), vec![r]));
        };
        if let Some(m) = l.meet.clone() {
            let seats: Vec<(i32, i32)> = [0, 1].iter().filter_map(|&i| l.meet_seat(i)).map(|(s, _)| (s.x as i32, s.y as i32)).collect();
            piece(&mut items, &m.key, "meet", m.x, m.y, Box::new(move |c| {
                D::draw_table(c, m.x, m.y);
                for (x, y) in &seats {
                    D::draw_stool(c, *x, *y);
                }
            }), 0.5);
        }
        if let Some(ch) = l.chess.clone() {
            let g = s.chess.as_ref().filter(|g| g.result.is_none()).cloned();
            let top = [(ch.x - 10) as f64, (ch.y - 17) as f64, (ch.x + 10) as f64, (ch.y - 5) as f64];
            let h = self.hot(top[0], top[1], top[2], top[3], json!({"kind": "chess"}));
            if h {
                let text = match &g {
                    None => "chess".to_string(),
                    Some(g) if g.white.id == "you" || g.black.id == "you" => format!("chess with {}", if g.white.id == "you" { &g.black.name } else { &g.white.name }),
                    Some(g) => format!("{} vs {}", g.white.name, g.black.name),
                };
                set_tip(&mut tip, text, ch.x as f64, (ch.y - 18) as f64);
            }
            let seats: Vec<(i32, i32)> = [0, 1].iter().filter_map(|&i| l.chess_seat(i)).map(|s| (s.x as i32, s.y as i32)).collect();
            let board = g.map(|g| g.board).unwrap_or_else(|| D::START.to_string());
            items.push(((ch.y as f64) - 0.5, What::Paint(Box::new(move |c: &mut Ctx| {
                for (x, y) in &seats {
                    D::draw_stool(c, *x, *y);
                }
                D::outlined(c, top, h, GOLD, 0.0, |c| D::draw_chess(c, ch.x, ch.y, Some(&board)));
            })), vec![]));
        }
        let usage = &s.usage;
        if let Some(cf) = l.coffee_at.clone() {
            let brewing = self.w.folk.iter().any(|f| f.act.kind == "coffee" && f.act.stage == 1);
            let used = usage.session.as_ref().map(|u| u.used as f64);
            piece(&mut items, &cf.key, "coffee", cf.x, cf.y, Box::new(move |c| D::draw_coffee(c, cf.x, cf.y, brewing, t, used)), 0.5);
        }
        if let Some(co) = l.cooler.clone() {
            let used = usage.week.as_ref().map(|u| u.used as f64);
            piece(&mut items, &co.key, "cooler", co.x, co.y, Box::new(move |c| D::draw_cooler(c, co.x, co.y, used)), 0.5);
        }
        if let Some(pr) = l.printer.clone() {
            // a click prints a report of what goes on (the page comes out of it)
            let job = self.w.folk.iter().find(|f| f.act.kind == "print" && f.act.stage == 1).map(|f| f.act.wait);
            let printing = (now - self.w.s.print) / 1000.0;
            let h = self.hot((pr.x - 7) as f64, (pr.y - 14) as f64, (pr.x + 7) as f64, (pr.y + 1) as f64, json!({"kind": "report"}));
            let wait = job.or(if printing < 3.0 { Some(printing) } else { None });
            piece(&mut items, &pr.key, "printer", pr.x, pr.y, Box::new(move |c| D::draw_printer(c, pr.x, pr.y, wait)), 0.5);
            if h {
                set_tip(&mut tip, "report".into(), pr.x as f64, (pr.y - 16) as f64);
            }
        }
        for mg in &l.managers {
            // a reception, an office manager's booth: a click (on it or on her) is hers: her terminal, or the notes
            let m = s.managers.iter().find(|x| x.key == mg.key).cloned().unwrap_or_default();
            let live: HashSet<&str> = s.bots.iter().map(|b| b.id.as_str()).collect();
            let notes_n = if m.style == "notes" { inp.digest.as_ref().map(|d| d.sessions.iter().filter(|id| live.contains(id.as_str())).count()).unwrap_or(0) } else { 0 };
            let color = crate::avatar::color_of(&m.color).unwrap_or(crate::sprites::sp::R);
            let screen = if !m.session {
                None
            } else if m.vacant {
                Some(RecScreen::Off)
            } else if m.busy {
                Some(RecScreen::Busy)
            } else if m.waiting {
                Some(RecScreen::Asks)
            } else if m.alive {
                Some(RecScreen::On)
            } else {
                Some(RecScreen::Off)
            };
            let name = if m.vacant { String::new() } else { m.name.clone() };
            let r = rf(foot("manager", [mg.x, mg.y], None));
            let h = self.hot(r[0], r[1], r[2], r[3], json!({"kind": "manager", "key": mg.key}));
            let (dg, tn) = (danger(&mg.key), tint_of(&mg.key));
            let lit_her = h || hl.contains(&format!("mgr:{}", mg.key)) || dg;
            let (x, y, n2, name2) = (mg.x, mg.y, if m.vacant { 0 } else { notes_n }, name.clone());
            items.push((if dg { 1e9 } else { (mg.y - 20) as f64 }, What::Paint(Box::new(move |c: &mut Ctx| {
                let o = Reception { n: n2, name: &name2, color, screen, t };
                D::outlined(c, r, lit_her, if dg { RED } else { GOLD }, tn, |c| {
                    D::draw_reception(c, x, y, &o, false);
                    D::draw_reception(c, x, y, &o, true);
                });
            })), vec![[(mg.x + 2) as f64, (mg.y - 30) as f64, (mg.x + 44) as f64, (mg.y - 14) as f64]]));
            if !dg {
                let name3 = name.clone();
                items.push(((mg.y as f64) - 0.5, What::Paint(Box::new(move |c: &mut Ctx| {
                    let o = Reception { n: n2, name: &name3, color, screen, t };
                    D::draw_reception(c, x, y, &o, true);
                })), vec![[mg.x as f64, (mg.y - 16) as f64, (mg.x + 46) as f64, (mg.y + 1) as f64]]));
            }
            if h {
                let text = if m.vacant {
                    format!("empty - a click hires {} again", if m.name.is_empty() { "her" } else { &m.name })
                } else {
                    format!(
                        "{}{}",
                        if m.name.is_empty() { "office manager" } else { &m.name },
                        if !m.error.is_empty() {
                            format!(" - {}", m.error)
                        } else if m.style == "notes" {
                            " - notes".into()
                        } else if m.session && m.id.is_empty() {
                            " - on her way".into()
                        } else {
                            " - office manager".into()
                        }
                    )
                };
                set_tip(&mut tip, text, (mg.x + 23) as f64, (mg.y - 30) as f64);
            }
        }
        for ch in l.chairs.clone() {
            // a click on one: you go and sit in it (a bot in it gets up)
            let r = foot("armchair", [ch.x, ch.y], None);
            let h = self.hot_r(r, json!({"kind": "sit", "key": ch.key}));
            piece(&mut items, &ch.key, "armchair", ch.x, ch.y, Box::new(move |c| D::draw_armchair(c, ch.x, ch.y)), 4.5);
            if h {
                set_tip(&mut tip, "have a seat".into(), ch.x as f64, (ch.y - 14) as f64);
            }
        }
        if let Some(b) = l.board.clone() {
            // the easel: a click shows the painting; the tip says which it is
            let h = self.hot((b.x - 11) as f64, (b.y - 33) as f64, (b.x + 11) as f64, (b.y - 8) as f64, json!({"kind": "board"}));
            let art = self.w.s.art.as_ref().map(|a| a.i);
            let small = crate::art::art_small(&mut self.w).to_vec();
            piece(&mut items, &b.key, "board", b.x, b.y, Box::new(move |c| D::draw_easel(c, b.x, b.y, art.map(|i| (i, &small[..])))), 0.5);
            if h {
                let title = art.map_or("easel", |i| crate::paintings::list()[i].title);
                set_tip(&mut tip, title.into(), b.x as f64, (b.y - 36) as f64);
            }
        }
        if let Some(fr) = l.fire.clone() {
            // the fireplace: what is held over it (a desk, a piece of furniture, a bot) is about to burn: the fire flares.
            // A desk with bots can't burn. The pointer on it stirs sparks
            let feeding = moving.as_ref().is_some_and(|m| in_fire(l, &m.key, m.at, m.u));
            let fd = if feeding { l.desks.iter().find(|d| Some(&d.key) == moving.as_ref().map(|m| &m.key)).cloned() } else { None };
            let on_it = self.moving.is_none() && self.held.is_none() && self.over(fr.x0 as f64, fr.y0 as f64, fr.x1 as f64, fr.y1 as f64);
            if on_it && !self.w.s.spark_on {
                self.w.s.spark_on = true;
                self.w.s.sparks = now;
            } else if !on_it {
                self.w.s.spark_on = false;
            }
            let since = (now - self.w.s.sparks).min(now - self.w.s.flare);
            let burst = if since < 1500.0 { since / 1500.0 } else { -1.0 };
            self.w.s.burning.retain(|o| now - o.t0 < 1400.0);
            let burning: Vec<(std::rc::Rc<pix::Image>, f64, f64, f64)> = self.w.s.burning.iter().map(|b| (b.img.clone(), b.x, b.y, (now - b.t0) / 1000.0)).collect();
            items.push((fr.y1 as f64, What::Paint(Box::new(move |c: &mut Ctx| {
                let bs: Vec<DBurning> = burning.iter().map(|(img, x, y, tt)| DBurning { img, x: *x, y: *y, t: *tt }).collect();
                D::draw_fire(c, fr.x0, fr.y0, t, feeding, burst, &bs, FirePart::All);
            })), vec![]));
            let (tx, ty) = ((fr.x0 + 16) as f64, (fr.y0 - 2) as f64);
            if let Some(fd) = fd {
                let has = s.bots.iter().any(|b| b.unit == fd.d.unit);
                let tail = if !has {
                    ""
                } else if l.booths.is_empty() {
                    ", delete its bots"
                } else if inp.hardcore() {
                    ", shoot its bots"
                } else {
                    ", its bots to the booth"
                };
                set_tip(&mut tip, format!("burn {}{tail}", fd.label), tx, ty);
            } else if feeding && moving.as_ref().is_some_and(|m| kind_of(&m.key) == "door") {
                let key = moving.as_ref().unwrap().key.clone();
                let m = s.machines.iter().find(|m| format!("door:{}", m.id) == key);
                set_tip(&mut tip, format!("take out {}", m.map(|m| m.name.as_str()).unwrap_or("the door")), tx, ty);
            } else if feeding {
                let k = kind_of(&moving.as_ref().unwrap().key).to_string();
                set_tip(&mut tip, format!("burn the {}", name_of(&k)), tx, ty);
            }
        }
        // the booth: a bot let go over it goes in, and after five seconds out of the list for good
        for bo in l.booths.clone() {
            let r = rf(foot("booth", [bo.x, bo.y], None));
            if inp.hardcore() {
                // hardcore: a gun closet; a click takes a gun and you see the office through your own eyes
                let out_n = self.w.folk.iter().filter(|f| f.carry == Some("gun")).count() as i32 + if inp.fps && self.w.get("you").is_some_and(|y| y.carry != Some("gun")) { 1 } else { 0 };
                let h = self.hot(r[0], r[1], r[2], r[3], json!({"kind": "aim", "key": bo.key}));
                let aimed = booth_under.as_ref().is_some_and(|b| b.key == bo.key);
                let (dg, tn) = (danger(&bo.key), tint_of(&bo.key));
                items.push(((bo.y as f64) - 3.5, What::Paint(Box::new(move |c: &mut Ctx| {
                    D::outlined(c, r, h || dg || aimed, if dg || aimed { RED } else { GOLD }, tn, |c| D::draw_gun_closet(c, bo.x, bo.y, false, out_n))
                })), vec![]));
                items.push(((bo.y as f64) - 2.5, What::Paint(Box::new(move |c: &mut Ctx| D::draw_gun_closet(c, bo.x, bo.y, true, out_n))), vec![]));
                if aimed {
                    let name = heldbot.as_ref().and_then(|id| self.w.get(id)).and_then(|f| f.b.as_ref()).map(|b| b.name.clone()).unwrap_or_default();
                    set_tip(&mut tip, format!("shoot {name}"), bo.x as f64, (bo.y - 34) as f64);
                } else if h {
                    set_tip(&mut tip, "gun closet: take a gun".into(), bo.x as f64, (bo.y - 34) as f64);
                }
                continue;
            }
            // a bot in it: the whole of its five seconds a click on the booth (or on the bot) takes it back out
            let inside = self.w.folk.iter().find(|f| f.act.kind == "doom" && f.act.at.as_ref() == Some(&bo.key) && f.b.is_some() && !f.gone).cloned();
            let bt = booth_t(inside.as_ref());
            let fl = match &self.w.s.booth_flash {
                Some((k, t0)) if *k == bo.key => (now - t0) / 1200.0,
                _ => -1.0,
            };
            let flagged = booth_under.as_ref().is_some_and(|b| b.key == bo.key);
            let h = inside.as_ref().is_some_and(|f| !f.act.back) && self.hot(r[0], r[1], r[2], r[3], json!({"kind": "undo", "id": inside.as_ref().unwrap().id}));
            let (dg, tn, still) = (danger(&bo.key), tint_of(&bo.key), inp.still);
            items.push(((bo.y as f64) - 3.5, What::Paint(Box::new(move |c: &mut Ctx| {
                D::outlined(c, r, dg || flagged || h, if dg || flagged { RED } else { GOLD }, tn, |c| D::draw_booth(c, bo.x, bo.y, false, bt, fl, flagged, t, still))
            })), vec![]));
            items.push(((bo.y as f64) - 2.5, What::Paint(Box::new(move |c: &mut Ctx| D::draw_booth(c, bo.x, bo.y, true, bt, fl, flagged, t, still))), vec![]));
            let bname = |id: &str| self.w.get(id).and_then(|f| f.b.as_ref()).map(|b| b.name.clone()).unwrap_or_default();
            if flagged {
                set_tip(&mut tip, format!("bye {}", bname(heldbot.as_deref().unwrap_or(""))), bo.x as f64, (bo.y - 34) as f64);
            } else if h {
                set_tip(&mut tip, format!("keep {}", bname(&inside.as_ref().unwrap().id)), bo.x as f64, (bo.y - 34) as f64);
            } else if on(&bo.key) && inside.is_none() {
                set_tip(&mut tip, "booth: a bot in it is deleted".into(), bo.x as f64, (bo.y - 34) as f64);
            }
        }
        // the dog: a click on it (or on its house, where it naps) and you go and pet it; it can be picked up
        let dog_tip = |w: &crate::world::World| -> Option<String> {
            let dog = w.s.dog.as_ref()?;
            Some(if crate::dog::can_throw(w) {
                "throw the stick".into()
            } else if !dog.inside {
                if dog.act.kind == "fetch" {
                    return None;
                }
                "pet the dog".into()
            } else if dog.act.hide {
                "comfort the dog".into()
            } else if dog.asleep {
                "wake the dog".into()
            } else {
                "call the dog".into()
            })
        };
        let mut dog_hots: Vec<([f64; 4], f64, f64)> = Vec::new();
        self.dog_rect = None;
        for (k, dh) in l.doghouses.clone().into_iter().enumerate() {
            let r = rf(foot("doghouse", [dh.x, dh.y], None));
            piece(&mut items, &dh.key, "doghouse", dh.x, dh.y, Box::new(move |c| D::draw_doghouse(c, dh.x, dh.y)), 0.5);
            if k == 0 && self.w.s.dog.as_ref().is_some_and(|g| !g.leaving && !g.held) {
                dog_hots.push((r, dh.x as f64, (dh.y - 23) as f64));
            }
        }
        if let Some(dog) = self.w.s.dog.clone() {
            if !dog.leaving && dog.inside && !l.doghouses.is_empty() {
                // at home: its head out of the door (only its eyes, afraid), z's asleep
                let dh = l.doghouses[0].clone();
                let im = D::dog_pic("lie", false);
                items.push(((dh.y as f64) - 0.4, What::Paint(Box::new(move |c: &mut Ctx| {
                    if dog.act.hide {
                        D::rect(c, dh.x - 2, dh.y - 5, 1, 1, C::STEEL4);
                        D::rect(c, dh.x + 1, dh.y - 5, 1, 1, C::STEEL4);
                        return;
                    }
                    c.draw_image_sub(&im, 10., 0., 8., 6., (dh.x - 4) as f64, (dh.y - 6) as f64, 8., 6.);
                    if dog.act.left > 1e8 || dog.asleep {
                        D::zzz(c, dog.z, dh.x + 3, dh.y - 9);
                    }
                })), vec![]));
            } else {
                let im = D::dog_pic(dog.pose, dog.wag != 0);
                let shake = if dog.act.kind == "shake" && !inp.still { if (t * 20.0).floor() as i64 % 2 != 0 { 1 } else { -1 } } else { 0 };
                let x0 = jround(dog.x - im.w as f64 / 2.0) + shake;
                let y0 = jround(dog.y - im.h as f64 + 1.0);
                let r = [x0 as f64, y0 as f64, (x0 + im.w as i32) as f64, (y0 + im.h as i32 + 1) as f64];
                let h = !dog.leaving && !dog.held && self.over(r[0], r[1], r[2], r[3]) && self.moving.is_none() && self.held.is_none();
                let by = match self.w.get("you") {
                    Some(you) if (you.x - dog.x).abs() < 24.0 && (you.y - dog.y).abs() < 10.0 => (jround(you.y.min(dog.y)) - 29) as f64,
                    _ => (y0 - 2) as f64,
                };
                if !dog.leaving && !dog.held {
                    self.dog_rect = Some(r);
                    dog_hots.push((r, dog.x, by));
                }
                let mouth = self.w.s.stick.as_ref().filter(|st| st.held).map(|_| D::dog_mouth(dog.pose));
                let (iw, ih) = (im.w as i32, im.h as i32);
                let (dir, held, barking) = (dog.dir, dog.held, dog.barking && (t * 5.0).floor() as i64 % 2 != 0);
                items.push((if held { 1e9 } else { dog.y }, What::Paint(Box::new(move |c: &mut Ctx| {
                    if !held {
                        D::rect(c, x0 + 2, y0 + im.h as i32 - 1, im.w as i32 - 4, 1, Color::rgba(0, 0, 0, 77));
                    }
                    c.save();
                    if dir < 0.0 {
                        c.translate((x0 * 2 + im.w as i32) as f64, 0.);
                        c.scale(-1., 1.);
                    }
                    if h {
                        c.draw_image(&crate::figure::halo(&im, GOLD), x0 as f64, y0 as f64);
                    }
                    c.draw_image(&im, x0 as f64, y0 as f64);
                    if let Some((mx, my)) = mouth {
                        D::rect(c, x0 + mx - 2, y0 + my, 10, 2, C::INK);
                        D::rect(c, x0 + mx - 1, y0 + my, 8, 1, D::STICK);
                    }
                    c.restore();
                    if barking {
                        // a bark: three short lines off its snout
                        let (sx, d) = if dir < 0.0 { (x0 - 1, -1) } else { (x0 + im.w as i32, 1) };
                        for (ox, oy) in [(1, -2), (2, 0), (1, 2)] {
                            D::rect(c, sx + d * ox, y0 + 3 + oy, 1, 1, C::INK);
                        }
                    }
                })), if held { vec![] } else { vec![[(x0 + 1) as f64, (y0 + 1) as f64, (x0 + iw - 1) as f64, (y0 + ih) as f64]] }));
            }
            for (hx, hy, t0) in dog.hearts.clone() {
                let k = (now - t0) / 1200.0;
                if k >= 1.0 {
                    continue;
                }
                items.push((1e8, What::Paint(Box::new(move |c: &mut Ctx| {
                    c.set_alpha(1.0 - k);
                    D::heart(c, jround(hx), jround(hy - k * 10.0));
                    c.set_alpha(1.0);
                })), vec![]));
            }
        }
        if let Some(st) = self.w.s.stick.clone().filter(|st| !st.held) {
            // the stick: tumbling through the air over its shadow, or lying on the floor
            let (sx, sy, sh) = (jround(st.x), jround(st.y), jround(st.h));
            let turn = if st.t < 1.0 && !inp.still { (st.t * 10.0).floor() as usize % 4 } else { 0 };
            items.push((st.y + 0.2, What::Paint(Box::new(move |c: &mut Ctx| {
                if sh > 0 {
                    D::rect(c, sx - 2, sy, 5, 1, Color::rgba(0, 0, 0, 77));
                }
                D::draw_stick(c, sx, sy - sh - 1, turn);
            })), vec![]));
        }
        // the radio: the room's sound, while one stands here and it is on; a click switches it
        let sound_on = s.settings.sound && !l.radios.is_empty();
        out.sound_on = sound_on;
        for ra in l.radios.clone() {
            let r = foot("radio", [ra.x, ra.y], None);
            let pan = ((ra.x as f64 / self.size.w as f64) * 1.6 - 0.8).clamp(-1.0, 1.0);
            let h = self.hot_r(r, json!({"kind": "sound", "pan": pan}));
            piece(&mut items, &ra.key, "radio", ra.x, ra.y, Box::new(move |c| D::draw_radio(c, ra.x, ra.y, sound_on)), 0.5);
            if sound_on && !inp.still {
                items.push(((ra.y as f64) + 0.1, What::Paint(Box::new(move |c: &mut Ctx| D::draw_notes(c, ra.x, ra.y, t))), vec![]));
            }
            if h {
                set_tip(&mut tip, if sound_on { "sound on".into() } else { "sound off".into() }, ra.x as f64, (ra.y - 22) as f64);
            }
        }
        for n in l.neuralyzers.clone() {
            let fl = self.w.s.flash.is_some_and(|f| now - f.2 < 400.0);
            piece(&mut items, &n.key, "neuralyzer", n.x, n.y, Box::new(move |c| D::draw_neuralyzer(c, n.x, n.y, fl)), 0.5);
            let held_on = heldbot.as_ref().and_then(|id| self.w.get(id)).filter(|f| think::in_rect(think::spot_of_fig(f), foot("neuralyzer", [n.x, n.y], None)));
            if let Some(f) = held_on {
                let name = f.b.as_ref().map(|b| b.name.clone()).unwrap_or_default();
                set_tip(&mut tip, if f.work { format!("{name} is busy") } else { format!("/clear {name}") }, n.x as f64, (n.y - 21) as f64);
            } else if on(&n.key) {
                set_tip(&mut tip, "neuralyzer: /clear a bot".into(), n.x as f64, (n.y - 21) as f64);
            }
        }
        for pl in l.plants.clone() {
            // the pointer on a plant stirs its leaves: two movements, then it is still until the pointer comes back
            if on(&pl.key) {
                self.w.s.sway.entry(pl.key.clone()).or_insert(now);
            } else {
                self.w.s.sway.remove(&pl.key);
            }
            let r = foot("plant", [pl.x, pl.y], None);
            let h = self.hot_r(r, json!({"kind": "water", "key": pl.key}));
            let i = pl.key.trim_start_matches(|c: char| !c.is_ascii_digit()).parse::<u32>().unwrap_or(0) % 2;
            let stir = self.w.s.sway.get(&pl.key).map(|t0| now - t0);
            piece(&mut items, &pl.key, "plant", pl.x, pl.y, Box::new(move |c| D::draw_plant(c, pl.x, pl.y, i, stir)), 0.5);
            if h {
                set_tip(&mut tip, "water".into(), pl.x as f64, (pl.y - 32) as f64);
            }
        }
        for pr in l.pairs.clone() {
            // a pair room (a chat of people and agents, pair.daems.me): a click opens the chat; a bot dropped on it
            // joins the chat. Its light is on once a room is linked
            let info = s.pair.get(&pr.key).cloned();
            let h = self.hot((pr.x - 13) as f64, (pr.y - 20) as f64, (pr.x + 13) as f64, (pr.y + 1) as f64, json!({"kind": "pairOpen", "key": pr.key}));
            let over_ = heldbot.as_ref().and_then(|id| self.w.get(id)).is_some_and(|f| think::in_rect(think::spot_of_fig(f), foot("pair", [pr.x, pr.y], None)));
            let linked = info.is_some();
            // you on its phone: the handset's foot (body()'s "phone")
            let hand = self.w.get("you").filter(|me| me.act.kind == "phone" && me.act.at.as_ref() == Some(&pr.key) && me.act.there && !me.held).map(|me| (jround(me.x) + if me.dir < 0.0 { -8 } else { 7 }, jround(me.y) - 12, me.act.side));
            piece(&mut items, &pr.key, "pair", pr.x, pr.y, Box::new(move |c| D::draw_pair(c, pr.x, pr.y, linked, t, over_, hand)), 0.5);
            if h {
                set_tip(&mut tip, info.as_ref().map(|i| if i.room.is_empty() { "pair".to_string() } else { i.room.clone() }).unwrap_or("pair: open a room".into()), pr.x as f64, (pr.y - 22) as f64);
            } else if over_ {
                let name = heldbot.as_ref().and_then(|id| self.w.get(id)).and_then(|f| f.b.as_ref()).map(|b| b.name.clone()).unwrap_or_default();
                set_tip(&mut tip, match &info { Some(i) => format!("{name} joins {}", if i.room.is_empty() { "the room" } else { &i.room }), None => "no room linked yet".into() }, pr.x as f64, (pr.y - 22) as f64);
            }
        }
        if let Some(bl) = self.w.s.ball.clone() {
            // the football: a small ball, its shadow on the floor, a hop as it flies
            let (bx, by, h) = (jround(bl.x), jround(bl.y), jround(bl.h));
            items.push((bl.y + 0.2, What::Paint(Box::new(move |c: &mut Ctx| {
                D::rect(c, bx - 1, by, 3, 1, Color::rgba(0, 0, 0, 77));
                D::rect(c, bx - 1, by - h - 3, 3, 3, C::INK);
                D::rect(c, bx, by - h - 2, 1, 1, C::WHITE);
            })), vec![]));
        }
        // what is left of a killed bot: the oil under everything, each piece at its own depth; a click on any of it
        // puts the bot back together
        for (id, r) in self.w.s.remains.clone() {
            let age = (now - r.t0) / 1000.0;
            let fade = if age > think::RUST_AFTER { ((age - think::RUST_AFTER) / think::RUST_FADE).min(1.0) } else { 0.0 };
            let rx = jround(r.puddle * r.parts.len() as f64 / r.total.max(1) as f64);
            let (px, py, drops) = (r.x as i32, r.y as i32, r.drops.clone());
            items.push((-1e9, What::Paint(Box::new(move |c: &mut Ctx| {
                if fade > 0.0 {
                    c.set_alpha(1.0 - fade);
                }
                if rx > 1 {
                    for (j, hw) in D::ell(rx, 3, 5).into_iter().enumerate() {
                        D::rect(c, px - hw, py - 2 + j as i32, 2 * hw, 1, Color::rgb(0x17151f));
                        if j == 1 && hw > 2 {
                            D::rect(c, px - hw + 2, py - 1, 3, 1, Color::rgb(0x4a4766));
                        }
                    }
                }
                for d in &drops {
                    D::rect(c, d.x as i32, d.y as i32, d.s, d.s, if d.hi { Color::rgb(0x4a4766) } else { Color::rgb(0x17151f) });
                }
                c.set_alpha(1.0);
            })), vec![]));
            for p in &r.parts {
                let part = D::Part {
                    kind: match p.kind {
                        "cog" => D::PartKind::Cog { big: p.big },
                        "bolt" => D::PartKind::Bolt,
                        "spring" => D::PartKind::Spring,
                        _ => D::PartKind::Cut { img: p.img.clone().unwrap(), sx: p.sx, sy: p.sy, sw: p.w, sh: p.h },
                    },
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    spin: p.spin,
                    charred: p.charred,
                };
                items.push((p.y, What::Paint(Box::new(move |c: &mut Ctx| D::draw_part(c, &part, fade))), vec![]));
            }
            if r.parts.is_empty() || r.back {
                continue;
            }
            let xs = r.parts.iter().map(|p| p.x);
            let ys = r.parts.iter().map(|p| p.y);
            let (x0, x1) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
            let (y0, y1) = (ys.clone().fold(f64::MAX, f64::min), ys.fold(f64::MIN, f64::max));
            if self.hot(x0 - 4.0, y0 - 10.0, x1 + 4.0, y1 + 3.0, json!({"kind": "unshoot", "id": id})) {
                set_tip(&mut tip, format!("{} - put back together", r.name), r.x, y0 - 12.0);
            }
        }
        if let Some(pg) = l.pong.clone() {
            let playing = self.w.folk.iter().filter(|f| f.act.kind == "pong" && f.act.there).count() == 2 || inp.pong.as_ref().is_some_and(|p| p.ball);
            let ball = playing.then_some(PongBall { t: self.w.s.pong_t, from: self.w.s.pong_from != 0 });
            piece(&mut items, &pg.key, "pong", pg.x, pg.y, Box::new(move |c| D::draw_pong(c, pg.x, pg.y, ball)), 0.5);
            if self.hot((pg.x - 16) as f64, (pg.y - 17) as f64, (pg.x + 16) as f64, pg.y as f64, json!({"kind": "pong"})) {
                set_tip(&mut tip, "ping-pong".into(), pg.x as f64, (pg.y - 18) as f64);
            }
        }
        // the limits, in the coffee pot (this five-hour session) and the cooler's bottle (this week); a click on either
        // opens the whole picture
        if let Some(c0) = l.coffee_at.clone()
            && self.hot(c0.x as f64, (c0.y - 22) as f64, (c0.x + 20) as f64, (c0.y + 1) as f64, json!({"kind": "limits"}))
        {
            set_tip(&mut tip, say(usage.session.as_ref(), "5h"), (c0.x + 10) as f64, (c0.y - 24) as f64);
        }
        if let Some(co) = l.cooler.clone()
            && self.hot((co.x - 5) as f64, (co.y - 26) as f64, (co.x + 5) as f64, (co.y + 1) as f64, json!({"kind": "limits"}))
        {
            set_tip(&mut tip, say(usage.week.as_ref(), "1w"), co.x as f64, (co.y - 24) as f64);
        }
        for (r, x, y) in dog_hots {
            if self.hot(r[0], r[1], r[2], r[3], json!({"kind": "pet"}))
                && let Some(text) = dog_tip(&self.w)
            {
                set_tip(&mut tip, text, x, y); // the dog's last: it stands in front of the pieces
            }
        }
        self.hovered = None;
        let hlv: Vec<String> = hl.iter().cloned().collect();
        for i in 0..self.w.folk.len() {
            if self.w.folk[i].lying {
                self.draw_sleeper(ctx, i, inp, &hlv);
            }
        }
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        // back to front; a figure under the pointer stops being the one under it when something solid is drawn over
        // it there (a bot behind a desk: a click on the desk's screen is the screen's)
        for (_, what, cover) in items {
            let before = self.hovered.clone().map(|h| h.id);
            match what {
                What::Fig(i) => self.draw_figure_at(ctx, i, inp, &hlv, t),
                What::Paint(f) => f(ctx),
            }
            if before.is_some() && self.hovered.as_ref().map(|h| &h.id) == before.as_ref() && cover.iter().any(|r| self.over(r[0], r[1], r[2], r[3])) {
                self.hovered = None;
            }
        }
        // a bot being carried, over everything; a piece carried in from the closet, where it would stand
        if let Some(i) = self.held.as_ref().and_then(|id| self.w.idx(id)) {
            self.draw_figure_at(ctx, i, inp, &hlv, t);
        }
        if let Some((kind, Some(at))) = self.carrying.clone() {
            ctx.set_alpha(0.7);
            let env = D::KindEnv { clock: self.clock(inp), door: D::paint_at(s.machines.len() as i32 - 1) };
            D::draw_kind(ctx, &kind, at[0], at[1], t, &env);
            ctx.set_alpha(1.0);
        }
        if self.held.is_some() {
            self.hovered = None;
        }
        if self.hovered.is_none()
            && self.held.is_none()
            && let Some((id, x, y)) = seat_hover
            && self.w.get(&id).is_some_and(|f| f.b.is_some())
        {
            self.hovered = Some(Hovered { id, x: (x + 3) as f64, y: (y - 16) as f64, seat: true });
        }
        self.tip = tip;
        let _ = (FIRE_H, on_wall("x"), RED);
    }

    pub(crate) fn hot_r(&mut self, r: [i32; 4], action: serde_json::Value) -> bool {
        self.hot(r[0] as f64, r[1] as f64, r[2] as f64, r[3] as f64, action)
    }

    /// this computer's clock, for the clock and the window
    pub fn clock(&self, inp: &Input) -> D::Clock {
        D::Clock { h: inp.hour, m: inp.minute, s: ((inp.epoch_ms / 1000) % 60) as u32 }
    }

    fn draw_sleeper(&mut self, ctx: &mut Ctx, i: usize, inp: &Input, hl: &[String]) {
        // lying on its back on the mat, head to the left: the standing figure turned on its side
        let f = &self.w.folk[i];
        if f.b.is_none() {
            return;
        }
        let (x, y) = (jround(f.x), jround(f.y));
        let im = body(&f.seed, color_of(f), Arms::Down, Legs::Stand, f.human, false);
        let hr = D::sleeper_hot(x, y);
        let on = self.over(hr[0] as f64, hr[1] as f64, hr[2] as f64, hr[3] as f64);
        let ring = if hl.contains(&f.id) || on {
            Some(GOLD)
        } else if inp.opens.contains(&f.id) {
            Some(crate::scene::OPEN)
        } else {
            None
        };
        D::draw_sleeper(ctx, x, y, &im, ring, f.human, f.z);
        if on {
            self.hovered = Some(Hovered { id: f.id.clone(), x: x as f64, y: (y - 20) as f64, seat: false });
        }
        let _ = Hot { r: [0.; 4], action: json!(null) };
        let _ = tiny;
    }
}

/// the home directory as ~
fn regex_home(dir: &str) -> String {
    for pre in ["/Users/", "/home/"] {
        if let Some(rest) = dir.strip_prefix(pre) {
            return match rest.find('/') {
                Some(i) => format!("~{}", &rest[i..]),
                None => "~".into(),
            };
        }
    }
    dir.to_string()
}
