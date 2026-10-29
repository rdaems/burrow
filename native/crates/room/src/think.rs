//! What everyone does, twelve times a second (room.js's `think`, `thinkOne`, `pickBreak`, the football,
//! the booth, the hunt, the office managers, the remains, fear). The bots are the sessions; what they do
//! is their tempo: a turn runs, at the desk typing; a dialog open, hand up; idle, a break.

use crate::io::{Input, Output, Sfx};
use crate::layout::{self, LAP, Layout, WALL, foot, jround, kind_of, meets, on_wall};
use crate::model::Bot;
use crate::world::*;
use serde_json::json;
use std::collections::HashSet;

/// the booth's five seconds
pub const BOOTH: f64 = 5.0;
/// seconds for a beam to cross the room
pub const BEAM: f64 = 0.7;
/// seconds for the football to fly from one to another
pub const PASS: f64 = 0.7;
/// hardcore: a kill's fear, and how long it takes to fade from all to nothing
pub const FEAR_KILL: f64 = 0.25;
pub const FEAR_FADE: f64 = 240.0;
/// remains rust away after half a minute, fading in two seconds
pub const RUST_AFTER: f64 = 30.0;
pub const RUST_FADE: f64 = 2.0;

pub const DOOMED: [&str; 3] = ["doom", "queue", "condemned"];
pub fn doomed(a: &Act) -> bool {
    DOOMED.contains(&a.kind)
}
/// breaks a bot leaves to clear up
const SWEEPERS: [&str; 11] = ["stand", "loiter", "plant", "window", "stretch", "warm", "watch", "visit", "chair", "coffee", "print"];

/// the booth's five seconds, by how far in they are (an undo runs them back)
pub fn booth_t(f: Option<&Fig>) -> f64 {
    match f {
        Some(f) if f.act.kind == "doom" => (BOOTH - f.act.left.unwrap_or(0.0)).clamp(0.0, BOOTH),
        _ => -1.0,
    }
}
pub fn booth_shut(t: f64) -> f64 {
    (t / 0.4).min(1.0)
}
pub fn booth_rise(t: f64) -> f64 {
    ((t - 0.6) / 3.4).clamp(0.0, 1.0)
}
pub fn booth_glow(t: f64) -> f64 {
    ((t - 0.3) / 3.9).clamp(0.0, 1.0)
}

/// what a tick is told and may tell
pub struct Tick<'a> {
    pub inp: &'a Input,
    pub out: &'a mut Output,
    pub dice: &'a mut Dice,
    /// bots you have just started (the + at a desk): to their desk, not to meet you
    pub fresh: &'a mut HashSet<String>,
    /// the fire is being carried
    pub moving_fire: bool,
    pub w: i32,
}

impl Tick<'_> {
    pub fn now(&self) -> f64 {
        self.inp.now
    }
    pub fn rnd(&mut self, a: f64, b: f64) -> f64 {
        self.dice.rnd(a, b)
    }
    pub fn rand(&mut self) -> f64 {
        self.dice.next()
    }
    pub fn rest(&mut self) -> Act {
        let r = self.dice.next();
        rest(r)
    }
    pub fn sfx(&mut self, name: &'static str, x: f64) {
        self.sfx_vol(name, x, 1.0);
    }
    pub fn sfx_vol(&mut self, name: &'static str, x: f64, vol: f64) {
        if self.inp.fps {
            return;
        }
        let pan = ((x / self.w as f64) * 1.6 - 0.8).clamp(-1.0, 1.0);
        self.out.sounds.push(Sfx { name, pan, vol, hum: None });
    }
    pub fn hardcore(&self) -> bool {
        self.inp.hardcore()
    }
}

/// a figure that may be shot at, or flee: an idle bot, not held, not leaving, not open, not lying, not doomed
pub fn idle_bot(f: &Fig) -> bool {
    f.is_bot() && !f.work && !f.held && !f.leaving && !f.open && !f.lying && !doomed(&f.act) && f.act.kind != "escort"
}

/// `from` sent `to` a message: a beam from its antenna to the other's, wherever they are
pub fn talk(w: &mut World, t: &mut Tick, from: &str, to: &str) {
    if from == to || w.get(from).is_none() || w.get(to).is_none() {
        return;
    }
    w.stat(from).sent += 1;
    w.stat(to).got += 1;
    w.s.beams.push(Beam { from: from.into(), to: to.into(), t: 0.0 });
    let x = w.get(from).unwrap().x;
    t.sfx("beam", x);
    let f = w.get_mut(from).unwrap();
    f.chat = f.chat.max(1.0);
}

/// you typed to the open session (`id`): a bubble over your head; the one you talk to answers when you pause
pub fn typed(w: &mut World, now: f64, id: Option<&str>, sent: bool) {
    if let Some(f) = w.get_mut("you") {
        f.chat = if sent { 0.6 } else { 2.0 };
    }
    if let Some(g) = id.and_then(|id| w.get_mut(id)) {
        g.reply = now + 900.0;
        if sent {
            g.sent = true;
        }
    }
}

/// the spot of a meeting halfway, by the act's numbers
fn spot_of(a: &Act) -> Spot {
    Spot::at(a.mx, a.my)
}

/// the office manager's seat at her reception
pub fn manager_seat(l: &Layout, m: Option<&layout::Piece>) -> Spot {
    match m {
        Some(m) => Spot::on((m.x + 27) as f64, (m.y - 5) as f64, &m.key),
        None => Spot::at((l.w / 2) as f64, (l.h / 2) as f64),
    }
}
pub fn booth_of<'a>(l: &'a Layout, f: &Fig) -> Option<&'a layout::Piece> {
    l.managers.iter().find(|m| Some(&m.key) == f.booth.as_ref())
}

/// the project's screen on its cabinet
pub fn screen_at(d: &layout::DeskL) -> (i32, i32) {
    (jround(d.x0 as f64 + (d.cab - (22 + d.trees.len() as i32 * LAP)) as f64 / 2.0), d.y - 32)
}

/// a whole act walked to a piece: where the pouf seats, and which way it faces
pub fn pouf_seat(ch: &layout::Piece) -> Spot {
    Spot::on(ch.x as f64, (ch.y - 4) as f64, &ch.key)
}
pub fn pouf_face(ch: &layout::Piece, w: i32) -> f64 {
    if ch.x < w / 2 { 1.0 } else { -1.0 }
}

// ---- the whole room's tick

pub fn think(w: &mut World, bots: &[Bot], l: &Layout, dt: f64, t: &mut Tick) {
    let now_s = t.now() / 1000.0;
    w.s.fear = if t.hardcore() { (w.s.fear - dt / FEAR_FADE).max(0.0) } else { 0.0 };
    scare(w, l, t);
    // the bot you play chess with sits at the table until the game is over (or it gets work)
    let game = t.inp.chess.clone();
    let vs = game.as_ref().filter(|g| g.result.is_none()).map(|g| if g.white.id == "you" { g.black.id.clone() } else { g.white.id.clone() });
    for f in w.folk.iter_mut() {
        if Some(&f.id) == vs.as_ref() && !(f.act.kind == "chess" && f.act.you) && !f.work {
            end(f);
            let side = if game.as_ref().unwrap().white.id == f.id { 0 } else { 1 };
            f.act = Act { kind: "chess", side, you: true, left: Some(1e9), ..Default::default() };
        }
        if f.act.kind == "chess" && !f.human && (if f.act.you { Some(&f.id) != vs.as_ref() } else { vs.is_some() }) {
            end(f); // the table is taken: the others get up
        }
    }
    // a move on the board (yours against a bot, or the bots' own game): the one who moved reaches out
    let sg = t.inp.state.chess.clone();
    if let Some(sg) = &sg
        && w.s.plies.is_some_and(|p| p != sg.sans.len())
    {
        w.s.chess_at = t.now();
        if t.inp.view.as_deref() == Some("chess") {
            let x = l.chess.as_ref().map(|c| c.x as f64).unwrap_or((t.w / 2) as f64);
            t.sfx("clack", x);
        }
        let mover = if sg.to_move == sg.white.id { &sg.black.id } else { &sg.white.id };
        if let Some(m) = w.get_mut(mover) {
            m.moved = 0.9;
        }
    }
    w.s.plies = sg.as_ref().map(|g| g.sans.len());
    // the bot you play ping-pong with stands at the table, at the far end
    let pv = t.inp.pong.as_ref().filter(|p| !p.watch).and_then(|p| p.id.clone());
    for f in w.folk.iter_mut() {
        if Some(&f.id) == pv.as_ref() && !(f.act.kind == "pong" && f.act.you) && !f.work {
            end(f);
            f.act = Act { kind: "pong", side: 1, you: true, left: Some(1e9), ..Default::default() };
        }
        if f.act.kind == "pong" && !f.human && (if f.act.you { Some(&f.id) != pv.as_ref() } else { pv.is_some() }) {
            end(f);
        }
    }
    // the duel at the window (outside): the bot you play stands there with you; two you watch stay until you look away
    let ow = t.inp.outside.clone();
    let wv = ow.as_ref().filter(|o| !o.watch).and_then(|o| o.id.clone());
    for f in w.folk.iter_mut() {
        if Some(&f.id) == wv.as_ref() && !(f.act.kind == "window" && f.act.you) && !f.work && !doomed(&f.act) {
            end(f);
            f.act = Act { kind: "window", at: Some(ow.as_ref().unwrap().at.clone()), side: 1, you: true, left: Some(1e9), ..Default::default() };
        }
        if f.act.kind == "window" && f.act.you && !f.human && Some(&f.id) != wv.as_ref() {
            end(f);
        }
        if let Some(o) = &ow
            && o.watch
            && o.ids.contains(&f.id)
            && f.act.kind == "window"
            && f.act.with.is_some()
        {
            f.act.left = Some(f.act.left.unwrap_or(0.0).max(5.0));
        }
    }
    // messages under way: the beam reaches the other, who takes it in
    for bm in w.s.beams.iter_mut() {
        let was = bm.t;
        bm.t += dt;
        if was < BEAM
            && bm.t >= BEAM
            && let Some(g) = w.folk.iter_mut().find(|g| g.id == bm.to)
        {
            g.chat = g.chat.max(1.0);
        }
    }
    let ids: HashSet<String> = w.folk.iter().map(|f| f.id.clone()).collect();
    w.s.beams.retain(|bm| bm.t < BEAM + 0.4 && ids.contains(&bm.from) && ids.contains(&bm.to));
    // heard: the booth's hum stopping when the bot in it is taken back
    let hums: Vec<String> = w.s.hums.keys().cloned().collect();
    for id in hums {
        let f = w.get(&id).map(|f| (f.act.kind == "doom" && !f.act.back, f.x));
        if f.is_some_and(|f| f.0) {
            continue;
        }
        w.s.hums.remove(&id);
        let x = f.map(|f| f.1).unwrap_or((t.w / 2) as f64);
        t.out.hush.push(id);
        t.sfx("back", x);
    }
    if let Some(you) = w.get_mut("you") {
        you.chat = (you.chat - dt).max(0.0);
    }
    let opens = t.inp.opens.clone();
    for b in bots {
        let Some(i) = w.idx(&b.id) else { continue };
        let work = b.waiting || b.busy;
        let you_chat = w.get("you").map(|y| y.chat);
        let n = fnv(&b.id);
        let f = &mut w.folk[i];
        f.chat = (f.chat - dt).max(0.0);
        if b.waiting && f.was_waiting == Some(false) {
            let x = f.x;
            t.sfx("ask", x); // a hand goes up
        }
        let f = &mut w.folk[i];
        f.busy = b.busy;
        f.waiting = b.waiting;
        f.was_waiting = Some(b.waiting);
        // a bot whose session is open sits at its desk for you, whatever it was doing; when the session
        // closes it gets up again
        f.open = opens.contains(&b.id);
        if !f.open || work {
            f.sent = false;
        }
        // its session opened while it idles: it comes over to meet you halfway (the focused one), and goes to its
        // desk once you have sent it something; one open beside it waits at its desk. One you have just started
        // goes straight to its desk, and you with it, for its first message
        if t.fresh.contains(&b.id) && (work || (f.was_open && !f.open)) {
            t.fresh.remove(&b.id);
        }
        f.was_open = f.open;
        let meet = f.open && !work && !f.sent && t.inp.open.as_deref() == Some(&b.id) && !t.fresh.contains(&b.id);
        if meet && f.act.kind != "meet" && !doomed(&f.act) {
            end(f);
            f.act = Act::new("meet");
        } else if f.open && !work && !meet && f.act.kind != "desk" && !doomed(&f.act) {
            end(f);
            f.act = Act::new("desk");
        }
        if !f.open && !work && ((f.act.kind == "desk" && f.sat) || f.act.kind == "meet") {
            f.act = Act::left("stand", 0.5);
        }
        if f.reply > 0.0 && t.now() > f.reply && you_chat.is_some_and(|c| c < 1.6) {
            f.reply = 0.0;
            if f.act.kind == "meet" {
                f.chat = f.chat.max(1.1); // its answer while you pause
            }
        }
        if work != f.work {
            f.work = work;
            // to work, whatever it was doing
            if work && !doomed(&f.act) {
                end(f);
                f.act = Act::new("desk");
            }
            if !work && f.act.kind == "desk" {
                f.act = Act::left("stand", 0.5 + (n % 20) as f64 / 10.0);
            }
        }
    }
    // remains on the floor: idle bots come over and clear them away, two to a heap, once the pieces have had a
    // moment to land
    step_remains(w, dt, l, t);
    condemn_step(w, l, t);
    let heaps: Vec<(String, f64, f64, f64)> = w
        .s
        .remains
        .iter()
        .filter(|(_, r)| !r.back && !r.parts.is_empty())
        .map(|(id, r)| (id.clone(), r.x, r.y, r.t0))
        .collect();
    for (id, rx, ry, t0) in heaps {
        if t.now() - t0 < 1500.0 {
            continue;
        }
        if w.folk.iter().filter(|g| g.act.kind == "sweep" && g.act.at.as_ref() == Some(&id)).count() >= 2 {
            continue;
        }
        if w.s.fear > 0.35 && t.rand() > 1.2 - w.s.fear {
            continue; // afraid: they hesitate to come over
        }
        let crew = w
            .folk
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                g.is_bot()
                    && !g.work
                    && !g.held
                    && !g.lying
                    && !g.open
                    && SWEEPERS.contains(&g.act.kind)
                    && !(g.act.kind == "loiter" && g.act.panic)
                    && !(g.act.kind == "window" && (g.act.you || g.act.with.is_some()))
            })
            .min_by(|a, b| (a.1.x - rx).hypot(a.1.y - ry).total_cmp(&(b.1.x - rx).hypot(b.1.y - ry)))
            .map(|(i, _)| i);
        if let Some(i) = crew {
            end(&mut w.folk[i]);
            w.folk[i].act = Act { kind: "sweep", at: Some(id.clone()), ..Default::default() };
        }
    }
    if let Some(m) = w.s.meeting.as_mut() {
        m.left -= dt;
        if m.left <= 0.0 {
            w.s.meeting = None;
            for f in w.folk.iter_mut() {
                if f.act.kind == "meeting" || f.act.kind == "present" {
                    end(f);
                }
            }
        }
    }
    // the ball goes back and forth while two are at the table
    let playing = w.folk.iter().filter(|f| f.act.kind == "pong" && f.act.there && f.act.with.is_some()).count() == 2;
    if playing || t.inp.pong.as_ref().is_some_and(|p| p.ball) {
        w.s.pong_t += dt / 0.8;
        if w.s.pong_t >= 1.0 {
            w.s.pong_t = 0.0;
            w.s.pong_from = 1 - w.s.pong_from;
        }
    }
    kickabout(w, dt, t);
    let first = t.rand();
    crate::dream::dream_step(w, dt, first);
    // an idle bot's place: at the end of its project's desk, by the empty chair, one per project (the first idle
    // one); the others are off on a break
    let mut homes: Vec<(String, Spot)> = Vec::new();
    for d in &l.desks {
        let first = d.seats.iter().find(|(id, _, _)| w.get(id).is_some_and(|f| !f.work && f.act.kind == "stand"));
        if let Some((id, _, _)) = first {
            homes.push((id.clone(), Spot::on(((t.w - 8) as f64).min((d.x0 + d.w + 8) as f64), (d.y + 1) as f64, &d.key)));
        }
    }
    for b in bots {
        if let Some(i) = w.idx(&b.id)
            && !w.folk[i].held
        {
            let home = homes.iter().find(|(id, _)| *id == b.id).map(|(_, s)| s.clone());
            think_one(w, i, l, now_s, dt, home.as_ref(), t);
        }
    }
    if let Some(i) = w.idx("you")
        && !w.folk[i].held
    {
        let hunting = !t.inp.fps && w.folk[i].act.kind != "arm" && hunt_step(w, i, l, dt, t);
        if !hunting {
            think_one(w, i, l, now_s, dt, None, t); // (in the first person the shooting is yours)
        }
    }
    let staff: Vec<usize> = (0..w.folk.len()).filter(|&i| w.folk[i].staff && !w.folk[i].leaving && w.folk[i].b.is_some()).collect();
    for i in staff {
        think_manager(w, i, l, dt, t);
    }
    crate::dog::think_dog(w, l, dt, t);
    // two standing about in the same place: they step apart
    let loit: Vec<usize> = (0..w.folk.len()).filter(|&i| w.folk[i].legs == "stand" && w.folk[i].act.kind == "loiter").collect();
    for &a in &loit {
        for &b in &loit {
            if w.folk[a].id < w.folk[b].id && (w.folk[a].x - w.folk[b].x).abs() < 12.0 && (w.folk[a].y - w.folk[b].y).abs() < 4.0 {
                let d = if w.folk[a].x <= w.folk[b].x { -1.0 } else { 1.0 };
                w.folk[a].x += d * 8.0 * dt;
                w.folk[b].x -= d * 8.0 * dt;
            }
        }
    }
}

// ---- one figure's tick

/// where a figure goes back to when let go: its desk if it works, else a pause
pub fn resume(f: &Fig, r: f64) -> Act {
    if f.work { Act::new("desk") } else { rest(r) }
}

pub fn think_one(w: &mut World, i: usize, l: &Layout, _now: f64, dt: f64, home: Option<&Spot>, t: &mut Tick) {
    let v = 22.0;
    let wt = w.s.t;
    let fear = w.s.fear;
    let id = w.folk[i].id.clone();
    let seat = l.seat(&id);
    {
        let f = &mut w.folk[i];
        f.on = None; // (its act walks it to its spot again, and says whose it is)
    }
    let (kind, work, there, lk) = {
        let f = &w.folk[i];
        (f.act.kind, f.work, f.act.kind == "sleep" && f.act.there, f.last_kind)
    };
    {
        let st = w.stat(&id);
        if kind != lk {
            *st.acts.entry(kind).or_insert(0) += 1;
        }
        if work {
            st.busy += dt;
        } else {
            st.idle += dt;
        }
        if there {
            st.asleep += dt;
        }
    }
    w.folk[i].last_kind = kind;
    // you sleep on a mat once the window has been left alone a minute, as pair's people who are away. A bot
    // naps only as a break of its own
    let human = w.folk[i].human;
    if human && t.inp.fps && w.folk[i].act.kind != "fps" {
        end(&mut w.folk[i]);
        w.folk[i].act = Act::new("fps");
    }
    let sleepy = human && !t.inp.fps && t.inp.away > 60.0;
    if sleepy && w.folk[i].act.kind != "sleep" {
        end(&mut w.folk[i]);
        w.folk[i].act = Act::new("sleep");
    }
    if !sleepy && w.folk[i].act.kind == "sleep" && w.folk[i].act.left.is_none() {
        wake(&mut w.folk[i]); // you are back (a nap of its own ends by itself)
    }
    // you: with the bot whose session is open, at the table of the game you play; else about the room
    if human && !sleepy && !t.inp.fps {
        let o = t.inp.open.clone().filter(|o| w.get(o).is_some());
        let f = &mut w.folk[i];
        let a = &f.act;
        if let Some(game) = &t.inp.chess {
            if !(a.kind == "chess" && a.you) {
                end(f);
                f.act = Act { kind: "chess", side: if game.white.id == "you" { 0 } else { 1 }, you: true, left: Some(1e9), ..Default::default() };
            }
        } else if t.inp.pong.is_some() {
            if !(a.kind == "pong" && a.you) {
                end(f);
                f.act = Act { kind: "pong", side: 0, you: true, left: Some(1e9), ..Default::default() };
            }
        } else if let Some(ow) = t.inp.outside.as_ref().filter(|o| !o.watch) {
            if !(a.kind == "window" && a.you) {
                end(f);
                f.act = Act { kind: "window", at: Some(ow.at.clone()), side: 0, you: true, left: Some(1e9), ..Default::default() };
            }
        } else if let Some(g) = o {
            if !(a.kind == "with" && a.to.as_ref() == Some(&g)) && !a.chosen {
                end(f);
                f.act = Act { kind: "with", to: Some(g), ..Default::default() }; // (a seat you clicked yourself keeps you)
            }
        } else if let Some(sh) = &t.inp.shell {
            if !(a.kind == "screen" && a.m.as_ref() == Some(&sh.m) && a.path.as_ref() == Some(&sh.dir)) && !a.chosen {
                end(f);
                f.act = Act { kind: "screen", m: Some(sh.m.clone()), path: Some(sh.dir.clone()), ..Default::default() };
            }
        } else if a.kind == "with" || a.kind == "screen" || a.you {
            end(f);
        }
    }
    {
        let f = &mut w.folk[i];
        if !["desk", "meeting", "chess", "chair", "sleep"].contains(&f.act.kind) {
            f.sat = false;
        }
        if f.act.kind != "football" {
            f.kick = 0.0;
        }
    }
    let kind = w.folk[i].act.kind;
    // the partner of a two-handed act: the one it is `with`, doing the same, with it
    let partner = |w: &World, i: usize| -> Option<usize> {
        let f = &w.folk[i];
        let g = f.act.with.as_ref().and_then(|id| w.idx(id))?;
        (w.folk[g].act.kind == f.act.kind && w.folk[g].act.with.as_ref() == Some(&f.id)).then_some(g)
    };
    let rnd_rest = t.rand();
    match kind {
        "fps" => {
            // the gun in hand, walking as you walk
            let f = &mut w.folk[i];
            f.carry = Some("gun");
            f.arms = "gun";
            f.legs = if f.walking { if (wt * 8.0).floor() as i64 % 2 != 0 { "walk1" } else { "walk2" } } else { "stand" };
        }
        "arm" => {
            // to the gun closet (where a hunter takes hers), a reach in, the gun, then the first person
            let bo = l.booths.iter().find(|b| Some(&b.key) == w.folk[i].act.at.as_ref()).cloned();
            let sh = t.inp.shell.as_ref().map(|s| format!("{}:{}", s.m, s.dir));
            let f = &mut w.folk[i];
            let Some(bo) = bo.filter(|_| t.hardcore() && t.inp.open == f.act.open && sh == f.act.shell) else {
                end(f);
                return;
            };
            if !walk(f, &Spot::on((bo.x + 14) as f64, bo.y as f64, &bo.key), 36.0, dt) {
                return;
            }
            f.legs = "stand";
            f.dir = -1.0;
            f.act.t += dt;
            if f.act.t < 0.4 {
                f.arms = "reach";
                return;
            }
            f.carry = Some("gun");
            f.arms = "gun";
            if !f.act.went {
                f.act.went = true;
                t.out.first_person = Some(bo.key.clone()); // (after this drawing: the melt starts from the room with the gun in your hands)
            }
        }
        "with" => {
            // with the bot you talk to, at its desk (it sits down for you): behind the desk, over its shoulder, looking at
            // its screen with it; behind, so a click on the bot (or a neighbour) is on the bot and not on you
            let to = w.folk[i].act.to.clone().unwrap_or_default();
            let Some(gi) = w.idx(&to) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let g = w.folk[gi].clone();
            let f = &mut w.folk[i];
            if doomed(&g.act) || g.act.kind == "escort" {
                // it is to go (its session still open): you watch, where you are
                f.legs = "stand";
                f.arms = "down";
                f.dir = if g.x < f.x { -1.0 } else { 1.0 };
                return;
            }
            let gs = l.seat(&to);
            let at = gs.clone().unwrap_or(Spot { x: g.x, y: g.y, on: g.on.clone() });
            let mb = if g.staff { booth_of(l, &g) } else { None };
            let met = (g.act.kind == "meet" && g.act.side != 0).then_some(&g.act); // it comes to meet you: halfway, facing it
            let spot = if let Some(m) = met {
                Spot::at(m.mx + 14.0 * m.side as f64, m.my)
            } else if let Some(mb) = mb {
                Spot::on(if mb.x + 54 <= l.floor.x1 as i32 { mb.x + 54 } else { mb.x - 8 } as f64, (mb.y + 1) as f64, &mb.key)
            } else if g.lying {
                Spot::at(g.x + 20.0, g.y + 1.0)
            } else {
                Spot { x: at.x + 5.0, y: l.floor.y0.max(at.y - 12.0), on: at.on.clone() }
            };
            f.act.there = walk(f, &spot, 36.0, dt);
            if f.act.there {
                f.dir = if let Some(m) = met { -m.side as f64 } else if g.staff && spot.x < g.x { 1.0 } else { -1.0 };
                f.arms = "down";
            }
        }
        "screen" => {
            // at the screen whose terminal you have open: in front of its cabinet, under the screen (or under the laptop,
            // for a worktree's), the bubble while you type
            let (m, dir) = (w.folk[i].act.m.clone().unwrap_or_default(), w.folk[i].act.path.clone().unwrap_or_default());
            let Some(d) = l.desks.iter().find(|x| x.d.m == m && (x.d.dir == dir || x.trees.contains(&dir))) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let (sx, _) = screen_at(d);
            let k = d.trees.iter().position(|x| *x == dir);
            let f = &mut w.folk[i];
            f.act.there = walk(f, &Spot::on(match k { None => sx + 11, Some(k) => sx + 29 + k as i32 * LAP } as f64, (d.y + 13) as f64, &d.key), 36.0, dt);
            if f.act.there {
                f.dir = -1.0;
                f.arms = "down";
            }
        }
        "desk" => {
            let f = &mut w.folk[i];
            if let Some(seat) = &seat
                && sit_at(f, seat, v, dt)
            {
                // (afraid: a glance over its shoulder now and then)
                f.dir = if fear > 0.6 && (wt * 0.8 + f.z * 3.0).floor() as i64 % 6 == 0 { 1.0 } else { -1.0 };
                f.legs = "sit";
                f.carry = None;
                f.arms = if f.waiting {
                    "wave"
                } else if f.busy {
                    if (wt * 6.0).floor() as i64 % 2 != 0 { "fwd2" } else { "fwd1" }
                } else {
                    "down"
                };
            }
        }
        "stand" => {
            let f = &mut w.folk[i];
            let there = match home {
                None => true,
                Some(h) => walk(f, h, v, dt), // back to its cabinet first, if that is its place
            };
            if !there {
                return;
            }
            f.legs = "stand";
            f.arms = f.carry.unwrap_or("down");
            if home.is_some() {
                f.dir = -1.0;
            }
            let left = f.act.left.unwrap_or(0.0) - dt;
            f.act.left = Some(left);
            if left > 0.0 {
                return;
            }
            f.carry = None;
            let a = pick_break(w, i, l, t);
            w.folk[i].act = a;
        }
        "pet" => {
            // you, petting the dog; afraid, it stays in, and you reach in at its door
            let Some(g) = w.s.dog.as_ref().filter(|g| !g.leaving && !g.held).map(|g| (g.x, g.y, g.inside)) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            let side = if f.x < g.0 { -1.0 } else { 1.0 };
            f.act.there = walk(f, &Spot::at(g.0 + side * if g.2 { 9.0 } else { 13.0 }, g.1 + 1.0), 36.0, dt);
            if f.act.there {
                f.legs = "stand";
                f.dir = -side;
                f.arms = if (wt * 4.0).floor() as i64 % 2 != 0 { "reach" } else { "down" };
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "fetch" => {
            // you, playing fetch: the throw, then watching the dog run for the stick and bring it back
            let dog = w.s.dog.as_ref().map(|g| (g.x, g.held, g.act.kind));
            let st = w.s.stick.clone();
            let f = &mut w.folk[i];
            f.legs = "stand";
            match (dog, st) {
                (Some((gx, gheld, gk)), Some(st)) if !gheld && gk == "fetch" => {
                    let k = wt - f.act.t;
                    f.arms = if k < 0.15 { "wave" } else if k < 0.4 { "reach" } else { "down" }; // the swing and the throw
                    if k >= 0.4 {
                        f.dir = if (if st.held { gx } else { st.x }) < f.x { -1.0 } else { 1.0 };
                    }
                }
                (_, st) => {
                    // over: you pick the stick up (if it lies at your feet)
                    f.act.done += dt;
                    let far = st.as_ref().is_none_or(|st| (st.x - f.x).hypot(st.y - f.y) > 20.0);
                    if far || f.act.done > 0.4 {
                        w.s.stick = None;
                        w.folk[i].act = rest(rnd_rest);
                    } else {
                        f.arms = "reach";
                    }
                }
            }
        }
        "meet" => {
            // its session opened while it idled: halfway to you, facing you, talking
            let Some(you) = w.get("you").map(|y| (y.x, y.y)) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            if f.act.side == 0 {
                let side = if you.0 >= f.x { 1 } else { -1 };
                let mx = (l.floor.x0 + 7.0).max((l.floor.x1 - 7.0).min((you.0 + f.x) / 2.0));
                f.act.side = side;
                f.act.mx = mx - 7.0 * side as f64;
                f.act.my = l.floor.y0.max(l.floor.y1.min((you.1 + f.y) / 2.0));
            }
            f.carry = None;
            let sp = spot_of(&f.act);
            f.act.there = walk(f, &sp, 30.0, dt);
            if f.act.there {
                f.legs = "stand";
                f.dir = f.act.side as f64;
                f.arms = "down";
            }
        }
        "loiter" => {
            let f = &mut w.folk[i];
            let to = f.act.dest.clone().unwrap_or(Spot::at(f.x, f.y));
            let sp = if f.act.v > 0.0 { f.act.v } else { v };
            if walk(f, &to, sp, dt) {
                f.arms = f.carry.unwrap_or("down");
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
            if f.act.panic {
                f.arms = "up"; // away from the shotgun, hands up
            }
        }
        "hide" => {
            // hardcore, afraid: behind a piece of furniture, peeking at the gun closet
            let to = w.folk[i].act.dest.clone().unwrap_or_default();
            if !walk(&mut w.folk[i], &to, v * 1.8, dt) {
                return;
            }
            let (fx, fy) = (w.folk[i].x, w.folk[i].y);
            let gun = w.folk.iter().any(|g| g.carry == Some("gun") && (g.x - fx).hypot(g.y - fy) < 90.0);
            let f = &mut w.folk[i];
            f.legs = "stand";
            if let Some(bo) = l.booths.first() {
                f.dir = if (bo.x as f64) < f.x { -1.0 } else { 1.0 };
            }
            f.arms = if gun { "up" } else { "down" };
            let left = f.act.left.unwrap_or(0.0) - dt;
            f.act.left = Some(left);
            if !(fear > 0.2) || left <= 0.0 {
                f.act = rest(rnd_rest);
            }
        }
        "coffee" => {
            // the pot holds what is left of the session's limit: an empty one gives nothing
            let Some(c) = l.coffee.clone() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let dry = t.inp.state.usage.session.as_ref().is_some_and(|s| s.used >= 100);
            let f = &mut w.folk[i];
            if !walk(f, &c, v, dt) {
                return;
            }
            if f.act.stage == 0 {
                f.dir = -1.0;
                f.arms = "reach";
                f.act.stage = 1;
            } else {
                f.act.wait += dt;
                f.arms = if f.act.wait > 2.0 && !dry { "mug" } else { "reach" };
                if f.act.wait > 2.8 {
                    f.carry = if dry { None } else { Some("mug") };
                    f.act = rest(rnd_rest);
                }
            }
        }
        "print" => {
            let Some(pr) = l.printer.clone() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            if !walk(f, &Spot::on((pr.x - 9) as f64, pr.y as f64, &pr.key), v, dt) {
                return;
            }
            if f.act.stage == 0 {
                f.dir = 1.0;
                f.arms = "down";
                f.act.stage = 1;
            } else {
                f.act.wait += dt;
                if f.act.wait > 2.4 {
                    f.carry = Some("paper");
                    f.act = rest(rnd_rest);
                }
            }
        }
        "cooler" => {
            let s = l.cooler_spot(w.folk[i].act.side);
            let g = w.folk[i].act.with.as_ref().and_then(|id| w.get(id)).is_some();
            let f = &mut w.folk[i];
            let (Some(s), true) = (s, g) else {
                f.act = rest(rnd_rest);
                return;
            };
            if walk(f, &s, v, dt) {
                f.dir = if f.act.side != 0 { -1.0 } else { 1.0 };
                f.arms = "down";
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "pong" => {
            let p = partner(w, i);
            let (from, pt) = (w.s.pong_from, w.s.pong_t);
            let f = &mut w.folk[i];
            if l.pong.is_none() || (!f.act.you && p.is_none()) {
                f.act = rest(rnd_rest);
                return;
            }
            let s = l.pong_spot(f.act.side).unwrap();
            f.act.there = walk(f, &s, v, dt);
            if f.act.there {
                f.dir = if f.act.side != 0 { -1.0 } else { 1.0 };
                f.arms = if from != f.act.side && pt > 0.75 { "fwd1" } else { "fwd2" };
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "football" => {
            // at its place round the pitch, facing the ball; a kick as the ball leaves it. One that gets work leaves
            // (its work act takes over); with one left the game is over
            let at = w.folk[i].act.spot.clone().unwrap_or_default();
            let team: Vec<usize> = w.folk[i]
                .act
                .team
                .iter()
                .filter_map(|id| w.idx(id))
                .filter(|&g| w.folk[g].act.kind == "football" && w.folk[g].act.spot.as_ref() == Some(&at))
                .collect();
            if team.len() < 2 {
                w.folk[i].act = rest(rnd_rest);
                return;
            }
            let ball = w.s.ball.as_ref().map(|b| b.x);
            let f = &mut w.folk[i];
            let ps = pitch_spot(&at, f.act.slot, f.act.n as usize);
            f.act.there = walk(f, &ps, v, dt);
            if f.act.there {
                f.kick = (f.kick - dt).max(0.0);
                f.arms = if f.kick > 0.0 { "fwd1" } else { "down" };
                if let Some(bx) = ball {
                    f.dir = if bx < f.x - 1.0 {
                        -1.0
                    } else if bx > f.x + 1.0 {
                        1.0
                    } else {
                        f.dir
                    };
                }
            }
            let left = f.act.left.unwrap_or(0.0) - dt;
            f.act.left = Some(left);
            if left <= 0.0 {
                for g in team {
                    if g != i {
                        let r = t.rand();
                        w.folk[g].act = rest(r);
                    }
                }
                w.folk[i].act = rest(rnd_rest);
                w.s.ball = None;
            }
        }
        "chess" => think_chess(w, i, l, dt, t, partner(w, i), rnd_rest),
        "visit" => {
            let to = w.folk[i].act.to.clone().unwrap_or_default();
            let Some(gi) = w.idx(&to) else {
                let f = &mut w.folk[i];
                f.act = if f.work { Act::new("desk") } else { rest(rnd_rest) };
                return;
            };
            // a seated one faces its screen (left): stand behind its chair; else beside it, on the side it comes from
            let (f, mut o) = split(&mut w.folk, i);
            let g = o.get_mut(&to).unwrap();
            let _ = gi;
            let side = if f.x < g.x { -1.0 } else { 1.0 };
            let seated = g.legs == "sit";
            let spot = if seated { Spot { x: g.x + 20.0, y: g.y + 1.0, on: g.on.clone() } } else { Spot { x: g.x + side * 14.0, y: g.y + 1.0, on: g.on.clone() } };
            if walk(f, &spot, v + 6.0, dt) {
                f.dir = if spot.x > g.x { -1.0 } else { 1.0 };
                f.act.wave -= dt;
                f.arms = if f.act.wave > 0.0 { "wave" } else { "down" };
                // visited: an idle one stops and faces the visitor
                if !g.work && (g.act.kind == "stand" || g.act.kind == "loiter") {
                    g.act = Act::left("stand", 2.0);
                    g.legs = "stand";
                    g.dir = -f.dir;
                }
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = if f.work { Act::new("desk") } else { rest(rnd_rest) };
                }
            }
        }
        "plant" => {
            let at = w.folk[i].act.at.clone();
            let Some(pl) = l.plants.iter().find(|p| Some(&p.key) == at.as_ref()).or(l.plants.first()).cloned() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            let side = if pl.x as f64 > (l.floor.x0 + l.floor.x1) / 2.0 { -1.0 } else { 1.0 };
            f.carry = Some("can");
            if walk(f, &Spot::on(pl.x as f64 + side * 14.0, (pl.y + 1) as f64, &pl.key), v, dt) {
                f.dir = -side;
                f.arms = "can";
                f.act.water = true;
                f.act.wait += dt;
                if f.act.wait > 3.0 {
                    f.carry = None;
                    f.act = rest(rnd_rest);
                }
            }
        }
        "window" => {
            let at = w.folk[i].act.at.clone();
            let p = partner(w, i);
            let win = l.windows.iter().find(|p| Some(&p.key) == at.as_ref()).or(l.windows.first()).cloned();
            let f = &mut w.folk[i];
            let Some(win) = win.filter(|_| f.act.with.is_none() || p.is_some()) else {
                f.act = rest(rnd_rest);
                return;
            };
            let duel = f.act.you || f.act.with.is_some();
            let mid = jround((win.x0 + win.x1) as f64 / 2.0);
            f.act.there = walk(f, &Spot::on((if duel { mid + if f.act.side != 0 { 10 } else { -10 } } else { mid }) as f64, (WALL + 8) as f64, &win.key), v, dt);
            if f.act.there {
                // looking out; at a duel the two face each other, an arm out as each fires: yours, or the two you watch,
                // as the duel goes; else a shot every few seconds, turn about
                let ow = t.inp.outside.as_ref();
                let mine = ow.filter(|o| f.act.you || (o.watch && o.ids.contains(&f.id)));
                let shot = match mine {
                    Some(o) => t.now() / 1000.0 - o.fired[f.act.side.clamp(0, 1) as usize] < 0.5,
                    None => f.act.with.is_some() && (wt + if f.act.side != 0 { 3.5 } else { 0.0 }) % 7.0 < 0.5,
                };
                f.dir = if duel {
                    if f.act.side != 0 { -1.0 } else { 1.0 }
                } else if (wt * 0.7 + f.z).sin() > 0.0 {
                    1.0
                } else {
                    -1.0
                };
                f.arms = if shot { "fwd1" } else { "down" };
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    if let Some(g) = p {
                        let r = t.rand();
                        w.folk[g].act = rest(r);
                    }
                    w.folk[i].act = rest(rnd_rest);
                }
            }
        }
        "chair" => {
            let at = w.folk[i].act.at.clone();
            let Some(ch) = l.chairs.iter().find(|p| Some(&p.key) == at.as_ref()).or(l.chairs.first()).cloned() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            if sit_at(f, &pouf_seat(&ch), v, dt) {
                f.dir = pouf_face(&ch, t.w);
                f.legs = "sit";
                f.arms = "book";
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "present" => {
            let host = w.s.meeting.as_ref().map(|m| m.host.clone());
            let f = &mut w.folk[i];
            let (Some(b), true) = (l.board.clone(), host.as_ref() == Some(&f.id)) else {
                f.act = rest(rnd_rest);
                return;
            };
            if walk(f, &Spot::on((b.x - 16 - 6) as f64, (b.y + 1) as f64, &b.key), v, dt) {
                f.dir = 1.0;
                f.arms = if (wt * 2.0).floor() as i64 % 3 != 0 { "reach" } else { "down" };
                f.act.drawing = true; // drawing the dream on (dreamStep)
            }
        }
        "meeting" => {
            let s = l.meet_seat(w.folk[i].act.seat);
            let meeting = w.s.meeting.is_some();
            let r = t.rnd(30.0, 60.0);
            let f = &mut w.folk[i];
            let Some((s, dir)) = s.filter(|_| meeting || f.act.free) else {
                f.act = rest(rnd_rest);
                return;
            };
            if sit_at(f, &s, v, dt) {
                f.dir = dir;
                f.legs = "sit";
                f.arms = "down";
                if f.act.free {
                    let left = f.act.left.unwrap_or(r) - dt;
                    f.act.left = Some(left);
                    if left <= 0.0 {
                        f.act = rest(rnd_rest);
                    }
                }
            }
        }
        "sleep" => {
            // it takes a rolled-up mat along to a clear bit of floor near where it is, rolls it out and lies down:
            // for as long as you are away (no `left`), or for a nap (`left`), after which it rolls the mat up again. A
            // piece put down on it wakes it
            if w.folk[i].act.spot.is_none() {
                let at = nap_spot(w, i, l);
                match at {
                    Some(at) => w.folk[i].act.spot = Some(at),
                    None => {
                        w.folk[i].act = rest(rnd_rest);
                        return;
                    }
                }
            }
            let at = w.folk[i].act.spot.clone().unwrap();
            if w.folk[i].act.there && !nap_clear(l, at.x, at.y, &[]) {
                wake(&mut w.folk[i]);
                return;
            }
            let f = &mut w.folk[i];
            if !f.act.there {
                f.carry = Some("roll");
                if !walk(f, &at, v, dt) {
                    return;
                }
                f.act.there = true;
                f.carry = None;
            }
            f.x = at.x;
            f.y = at.y;
            f.lying = true;
            f.dir = 1.0;
            f.legs = "sit";
            f.arms = "down";
            f.z += dt;
            if let Some(left) = f.act.left {
                f.act.left = Some(left - dt);
                if left - dt <= 0.0 {
                    wake(f);
                }
            }
        }
        "warm" => {
            // its hands at the fire, from its left or its right side
            let Some(fire) = l.fire.clone() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            if walk(f, &Spot::at(if f.act.side != 0 { (fire.x1 + 5) as f64 } else { (fire.x0 - 5) as f64 }, (fire.y1 + 2) as f64), v, dt) {
                f.legs = "stand";
                f.dir = if f.act.side != 0 { -1.0 } else { 1.0 };
                f.arms = if (wt * 1.5).floor() as i64 % 3 != 0 { "reach" } else { "down" };
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "watch" => {
            // a spectator behind the chess or the ping-pong table while a game is on there: follows the play, cheers a move
            let game = w.folk[i].act.game.unwrap_or("chess");
            let chess = game == "chess";
            let table = if chess { l.chess.clone() } else { l.pong.clone() };
            let players = w.folk.iter().enumerate().filter(|(j, g)| *j != i && g.act.kind == game && g.act.there && (g.act.with.is_some() || g.act.you)).count();
            let (from, chess_at) = (w.s.pong_from, w.s.chess_at);
            let f = &mut w.folk[i];
            let Some(tb) = table.filter(|_| players >= 2) else {
                f.act = rest(rnd_rest);
                return;
            };
            if walk(f, &Spot::on(tb.x as f64, ((WALL + 30) as f64).max((tb.y - if chess { 17 } else { 22 }) as f64), &tb.key), v, dt) {
                f.legs = "stand";
                f.dir = if chess {
                    if (wt * 0.5 + f.z).floor() as i64 % 2 != 0 { 1.0 } else { -1.0 }
                } else if from != 0 {
                    -1.0
                } else {
                    1.0
                };
                f.arms = if chess && t.now() - chess_at < 600.0 { "up" } else { "down" };
                let left = f.act.left.unwrap_or(0.0) - dt;
                f.act.left = Some(left);
                if left <= 0.0 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        "stretch" => {
            // a stretch where it stands: arms up, arms down
            let f = &mut w.folk[i];
            f.legs = "stand";
            f.arms = if (wt * 2.5).floor() as i64 % 2 != 0 { "up" } else { "down" };
            let left = f.act.left.unwrap_or(0.0) - dt;
            f.act.left = Some(left);
            if left <= 0.0 {
                f.act = rest(rnd_rest);
            }
        }
        "serve" => think_serve(w, i, l, dt, t, rnd_rest),
        "doom" => {
            // put in the booth: it stands inside, the door slides shut, a light rises through the cabin and the bot goes
            // up in it. A click on the booth (or on the bot) all that while takes it back: the light runs down again, the
            // door opens, it steps out; so does carrying it out. At the end it is gone: `claude stop` and `claude rm`
            let at = w.folk[i].act.at.clone();
            let Some(bo) = l.booths.iter().find(|p| Some(&p.key) == at.as_ref()).cloned() else {
                let f = &mut w.folk[i];
                f.act = resume(f, rnd_rest);
                return;
            };
            {
                let f = &mut w.folk[i];
                f.x = bo.x as f64;
                f.y = (bo.y - 3) as f64;
                f.on = Some(bo.key.clone());
                f.dir = -1.0;
                f.arms = "down";
                f.legs = "stand";
            }
            let (back, id) = (w.folk[i].act.back, w.folk[i].id.clone());
            if !back && !w.s.hums.contains_key(&id) {
                w.s.hums.insert(id.clone(), true); // the light rising, heard
                let pan = ((bo.x as f64 / t.w as f64) * 1.6 - 0.8).clamp(-1.0, 1.0);
                if !t.inp.fps {
                    t.out.sounds.push(Sfx { name: "booth", pan, vol: 1.0, hum: Some(id.clone()) });
                }
            }
            let f = &mut w.folk[i];
            if back {
                let left = f.act.left.unwrap_or(0.0) + dt * 6.0;
                f.act.left = Some(left);
                if left >= BOOTH {
                    f.act = resume(f, rnd_rest);
                }
                return;
            }
            let left = f.act.left.unwrap_or(0.0) - dt;
            f.act.left = Some(left);
            if left <= 0.0 && !f.gone {
                t.out.act(json!({"kind": "stop", "id": id, "rm": true}));
                w.s.stopped.insert(id.clone(), t.now());
                w.s.booth_flash = Some((bo.key.clone(), t.now()));
                w.s.hums.remove(&id);
                t.sfx("poof", bo.x as f64);
                w.folk.remove(i);
            }
        }
        "queue" => {
            // let go of from here: in line by the booth (on its roomier side), its turn next
            let Some(bo) = l.booths.first() else { return };
            let f = &mut w.folk[i];
            let n = f.act.n.max(0);
            let side = if (bo.x - 70) as f64 <= l.floor.x0 { 1.0 } else { -1.0 };
            let at = Spot::on(bo.x as f64 + side * (18 + (n % 5) * 12) as f64, l.floor.y1.min((bo.y + 2 + (n / 5) * 9) as f64), &bo.key);
            f.act.there = walk(f, &at, v, dt);
            if f.act.there {
                f.legs = "stand";
                f.dir = -side;
                f.arms = "down";
            }
        }
        "condemned" => think_condemned(w, i, l, dt, t),
        "sweep" => {
            // clearing away what is left of a bot: to the nearest piece that landed, bent down for it
            let at = w.folk[i].act.at.clone().unwrap_or_default();
            let (fx, fy) = (w.folk[i].x, w.folk[i].y);
            let Some(r) = w.remains(&at).filter(|r| !r.back && !r.parts.is_empty()) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let (rx, ry) = (r.x, r.y);
            let p = r
                .parts
                .iter()
                .enumerate()
                .filter(|(_, q)| q.z == 0.0 && q.vx == 0.0 && q.vy == 0.0)
                .min_by(|a, b| (a.1.x - fx).hypot(a.1.y - fy).total_cmp(&(b.1.x - fx).hypot(b.1.y - fy)))
                .map(|(k, q)| (k, q.x, q.y));
            let f = &mut w.folk[i];
            let Some((k, px, py)) = p else {
                // still flying: over to the mess meanwhile
                if walk(f, &Spot::at(rx + if f.x < rx { -12.0 } else { 12.0 }, ry + 4.0), v, dt) {
                    f.legs = "stand";
                    f.arms = "down";
                }
                return;
            };
            if walk(f, &Spot::at(px + if f.x < px { -5.0 } else { 5.0 }, py + 1.0), v, dt) {
                f.legs = "stand";
                f.dir = if px < f.x { -1.0 } else { 1.0 };
                f.arms = if (wt * 4.0).floor() as i64 % 2 != 0 { "reach" } else { "down" };
                f.act.wait += dt;
                if f.act.wait > 0.8 {
                    f.act.wait = 0.0;
                    let r = w.remains_mut(&at).unwrap();
                    r.parts.remove(k);
                    r.drops.retain(|d| (d.x - px).hypot(d.y - py) > 7.0); // the oil there wiped up with it
                    if r.parts.is_empty() {
                        gone(w, &at, t);
                        w.folk[i].act = rest(rnd_rest);
                    }
                }
            }
        }
        "escort" => {
            // walked to the booth by an office manager, a step behind her
            let by = w.folk[i].act.by.clone().unwrap_or_default();
            let Some(m) = w.get(&by).map(|m| (m.x, m.y, m.dir)) else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            let md = if m.2 == 0.0 { -1.0 } else { m.2 };
            if walk(f, &Spot::at(m.0 - 9.0 * md, m.1), 24.0, dt) {
                f.legs = "stand";
                f.dir = md;
                f.arms = "down";
            }
        }
        "neuralyze" => {
            // put to the neuralyzer: it steps up, the flash (`/clear` in its session: a new conversation), and it stands
            // there dazed a moment
            let at = w.folk[i].act.at.clone();
            let Some(n) = l.neuralyzers.iter().find(|p| Some(&p.key) == at.as_ref()).cloned() else {
                w.folk[i].act = rest(rnd_rest);
                return;
            };
            let f = &mut w.folk[i];
            if !walk(f, &Spot::on((n.x + 11) as f64, (n.y + 1) as f64, &n.key), v, dt) {
                return;
            }
            if f.act.stage == 0 {
                f.legs = "stand";
                f.dir = -1.0;
                f.arms = "down";
                f.act.stage = 1;
                f.act.wait = 0.0;
            } else if f.act.stage == 1 {
                f.act.wait += dt;
                if f.act.wait > 0.6 {
                    let id = f.id.clone();
                    f.act.stage = 2;
                    f.act.wait = 0.0;
                    t.out.act(json!({"kind": "clear", "id": id}));
                    w.s.flash = Some(((n.x - 5) as f64, (n.y - 16) as f64, t.now()));
                    t.sfx("flash", n.x as f64);
                }
            } else {
                f.act.wait += dt;
                if f.act.wait > 2.5 {
                    f.act = rest(rnd_rest);
                }
            }
        }
        _ => {
            w.folk[i].act = rest(rnd_rest);
        }
    }
    if let Some(f) = w.folk.get_mut(i)
        && f.legs != "sit"
    {
        f.lying = false;
    }
}

fn think_chess(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick, p: Option<usize>, rnd_rest: f64) {
    // alone at the board it waits for someone to play, studying it; two play until it's over (a real game, both
    // sides played from the server); against you it plays until the game is over
    if l.chess.is_none() {
        w.folk[i].act = rest(rnd_rest);
        return;
    }
    if w.folk[i].act.with.is_some() && p.is_none() {
        let r = t.rnd(8.0, 16.0);
        let f = &mut w.folk[i];
        f.act.with = None;
        f.act.left = Some(f.act.left.unwrap_or(r).min(r));
    }
    let v = 22.0;
    let side = w.folk[i].act.side;
    let seat = l.chess_seat(side).unwrap();
    let there = sit_at(&mut w.folk[i], &seat, v, dt);
    w.folk[i].act.there = there;
    // the game that is theirs: the one that started once both sat (an older finished game in the state is not it)
    let sg = t.inp.state.chess.clone();
    let id = w.folk[i].id.clone();
    let with = w.folk[i].act.with.clone();
    if let Some(sg) = &sg
        && sg.result.is_none()
        && !w.folk[i].act.you
        && [&id, with.as_ref().unwrap_or(&String::new())].iter().any(|x| **x == sg.white.id)
        && [&id, with.as_ref().unwrap_or(&String::new())].iter().any(|x| **x == sg.black.id)
    {
        w.folk[i].act.game_id = Some(sg.id);
    }
    let mine = w.folk[i].act.game_id.is_some() && sg.as_ref().is_some_and(|g| Some(g.id) == w.folk[i].act.game_id);
    if there {
        let f = &mut w.folk[i];
        f.dir = if f.act.side != 0 { -1.0 } else { 1.0 };
        f.legs = "sit";
        f.moved = (f.moved - dt).max(0.0);
        f.arms = if f.moved > 0.0 { "reach" } else { "down" };
        // both seated and no game yet: white (the left seat) asks the server for one between them
        if let Some(g) = p
            && w.folk[g].act.there
            && w.folk[i].act.side == 0
            && !w.folk[i].act.you
            && !sg.as_ref().is_some_and(|s| s.result.is_none())
            && t.now() - w.s.chess_asked > 4000.0
        {
            w.s.chess_asked = t.now();
            t.out.act(json!({"kind": "chessBots", "a": id, "b": w.folk[g].id}));
        }
    }
    if mine && sg.as_ref().is_some_and(|s| s.result.is_some()) {
        let f = &mut w.folk[i];
        f.act.left = Some(f.act.left.unwrap_or(3.0).min(3.0)); // their game is over: they get up soon
    }
    let left = w.folk[i].act.left.unwrap_or(0.0) - dt;
    w.folk[i].act.left = Some(left);
    if left <= 0.0 {
        if mine && sg.as_ref().is_some_and(|s| s.result.is_none()) {
            t.out.act(json!({"kind": "chessLeave", "id": id})); // leaves the game unfinished: it is the other's
        }
        if let Some(g) = p {
            let r = t.rand();
            w.folk[g].act = rest(r);
        }
        w.folk[i].act = rest(rnd_rest);
    }
}

fn think_serve(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick, rnd_rest: f64) {
    // a coffee for a busy colleague: fetched at the machine, put on its desk by the keyboard (if it has got up
    // meanwhile, the server keeps the cup)
    let v = 22.0;
    let to = w.folk[i].act.to.clone().unwrap_or_default();
    let gs = l.seat(&to);
    let g = w.get(&to).map(|g| (g.legs, g.act.kind, g.id.clone()));
    let (Some(c), Some(g), Some(gs)) = (l.coffee.clone(), g, gs) else {
        w.folk[i].act = rest(rnd_rest);
        return;
    };
    let dry = t.inp.state.usage.session.as_ref().is_some_and(|s| s.used >= 100);
    let wt = w.s.t;
    let f = &mut w.folk[i];
    if f.act.stage < 2 && !walk(f, &c, v, dt) {
        return;
    }
    if f.act.stage == 0 {
        f.dir = -1.0;
        f.arms = "reach";
        f.act.stage = 1;
    } else if f.act.stage == 1 {
        f.act.wait += dt;
        if dry {
            f.act = rest(rnd_rest);
            return;
        }
        if f.act.wait > 2.0 {
            f.carry = Some("mug");
            f.act.stage = 2;
            f.act.wait = 0.0;
        } else {
            f.arms = "reach";
        }
    } else if walk(f, &Spot { x: gs.x + 20.0, y: gs.y + 1.0, on: gs.on.clone() }, v, dt) {
        f.dir = -1.0;
        f.arms = "mug";
        f.act.wait += dt;
        if f.act.wait > 1.2 {
            if g.0 == "sit" && g.1 == "desk" {
                w.s.mugs.insert(g.2, wt + 90.0);
                w.folk[i].carry = None;
            }
            w.folk[i].act = rest(rnd_rest);
        }
    }
}

fn think_condemned(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick) {
    // hardcore: someone takes a gun for it. While she (or you) goes for the gun it is only taken aback: a start,
    // then it stands there watching her, looking about now and then, and starts again at a shot. Once the gun comes
    // its way (the one with it walks over to it; you, in the first person) it panics: away, then a few hurried steps
    // flailing its hands, a stop to look this way and that, off again (slower than the one with the gun walks, so it
    // is caught). The gun levelled at it: it freezes, hands up, facing it
    let id = w.folk[i].id.clone();
    let wt = w.s.t;
    let by = w.folk.iter().find(|g| g.hunt.as_ref().is_some_and(|h| h.id.as_ref() == Some(&id)) && g.carry == Some("gun")).map(|g| (g.x, g.y, g.hunt.as_ref().unwrap().phase));
    if let Some((bx, _, "aim")) = by {
        let f = &mut w.folk[i];
        f.act.dest = None;
        f.legs = "stand";
        f.arms = "up";
        f.dir = if bx < f.x { -1.0 } else { 1.0 };
        return;
    }
    let you = w.folk.iter().find(|g| g.human && g.act.kind == "fps").map(|g| (g.x, g.y));
    if !w.folk[i].act.panic && !matches!(by, Some((_, _, "hunt"))) && you.is_none() {
        // her: the one going for the gun for it (else whoever is), else the gun closet
        let mine = |g: &&Fig| g.hunt.as_ref().is_some_and(|h| h.id.as_ref() == Some(&id));
        let her = w.folk.iter().find(mine).or_else(|| w.folk.iter().find(|g| g.hunt.is_some())).map(|g| g.x);
        let at = her.or_else(|| l.booths.first().map(|b| b.x as f64));
        let bang = w.s.bang;
        let first = w.folk[i].act.bang.is_none();
        let r0 = if first { t.rnd(1.5, 3.0) } else { 0.0 };
        let again = !first && w.folk[i].act.look - dt < -0.4;
        let r1 = if again { t.rnd(1.5, 3.0) } else { 0.0 };
        let f = &mut w.folk[i];
        if first {
            f.act.bang = Some(bang); // just now: a start
            f.act.jolt = wt;
            f.act.look = r0;
        }
        if f.act.bang.is_some_and(|b| bang > b) {
            f.act.bang = Some(bang); // a shot: another
            f.act.jolt = wt;
        }
        f.legs = "stand";
        f.arms = if wt - f.act.jolt < 0.4 { "up" } else { "down" };
        f.act.look -= dt;
        if again {
            f.act.look = r1; // a glance the other way, over
        }
        if let Some(ax) = at {
            f.dir = if ax < f.x { -1.0 } else { 1.0 } * if f.act.look < 0.0 { -1.0 } else { 1.0 };
        }
        return;
    }
    let fl = l.floor;
    let clampx = |x: f64| x.clamp(fl.x0, fl.x1);
    let clampy = |y: f64| y.clamp(fl.y0, fl.y1);
    if !w.folk[i].act.panic {
        // first away from the gun coming, as far as it dares; then it dithers there
        let from = by.map(|(x, y, _)| (x, y)).or(you).or_else(|| l.booths.first().map(|b| (b.x as f64, b.y as f64)));
        let f = &mut w.folk[i];
        f.act.panic = true;
        f.act.dest = None;
        let home = match from {
            Some((bx, by)) => {
                let d = (f.x - bx).hypot(f.y - by);
                let d = if d == 0.0 { 1.0 } else { d };
                Spot::at(clampx(f.x + (f.x - bx) / d * 44.0), clampy(f.y + (f.y - by) / d * 24.0))
            }
            None => Spot::at(f.x, f.y),
        };
        f.act.dest = Some(home.clone());
        f.act.home = Some(home);
        f.act.v = 30.0;
    }
    if let Some(to) = w.folk[i].act.dest.clone() {
        let pause = t.rnd(0.5, 1.6);
        let f = &mut w.folk[i];
        if walk(f, &to, f.act.v, dt) {
            f.act.dest = None;
            f.act.pause = Some(pause);
        }
        f.arms = if (wt * 6.0).floor() as i64 % 2 != 0 { "up" } else { "wave" };
        return;
    }
    {
        let r = t.rnd(0.25, 0.6);
        let f = &mut w.folk[i];
        f.legs = "stand";
        f.arms = "up";
        f.act.look -= dt;
        if f.act.look <= 0.0 {
            f.dir = -(if f.dir == 0.0 { 1.0 } else { f.dir }); // this way, that way
            f.act.look = r;
        }
    }
    let p0 = t.rnd(0.3, 1.0);
    let pause = w.folk[i].act.pause.unwrap_or(p0) - dt;
    w.folk[i].act.pause = Some(pause);
    if pause <= 0.0 {
        let home = w.folk[i].act.home.clone().unwrap();
        let mut to = None;
        for k in 0..10 {
            // somewhere clear near it (further the more it tries), else a shuffle
            let r = 30.0 + k as f64 * 6.0;
            let x = clampx(home.x + t.rnd(-r, r));
            let y = clampy(home.y + t.rnd(-r / 2.0, r / 2.0));
            if l.free(x, y) {
                to = Some(Spot::at(x, y));
                break;
            }
        }
        let fx = w.folk[i].x;
        let to = to.unwrap_or_else(|| Spot::at(clampx(fx + t.rnd(-8.0, 8.0)), w.folk[i].y));
        let vv = t.rnd(18.0, 28.0);
        let pp = t.rnd(0.5, 1.6);
        let f = &mut w.folk[i];
        f.act.dest = Some(to);
        f.act.v = vv;
        f.act.pause = Some(pp);
    }
}

// ---- breaks

/// a break, chosen by a bot with nothing to do (pair's pickBreak, and chess). Long spells, so the room is calm
pub fn pick_break(w: &mut World, i: usize, l: &Layout, t: &mut Tick) -> Act {
    let r = t.rand();
    if w.s.fear > 0.5 && t.rand() < w.s.fear {
        if let Some(at) = hide_spot(w, i, l, t) {
            let left = t.rnd(15.0, 35.0);
            return Act { kind: "hide", dest: Some(at), left: Some(left), ..Default::default() };
        }
    }
    let fid = w.folk[i].id.clone();
    let human = w.folk[i].human;
    let others: Vec<usize> = (0..w.folk.len()).filter(|&j| j != i && !w.folk[j].leaving).collect();
    let mates: Vec<usize> = others.iter().copied().filter(|&j| !w.folk[j].work && (w.folk[j].act.kind == "stand" || w.folk[j].act.kind == "loiter")).collect();
    let mate = t.dice.any(&mates).copied();
    let taken = |w: &World, kind: &str| others.iter().any(|&j| w.folk[j].act.kind == kind);
    // someone waits at the chess board: most likely, it gets a game
    let waiter = others.iter().copied().find(|&j| {
        let a = &w.folk[j].act;
        a.kind == "chess" && a.with.is_none() && !a.you && a.there
    });
    if let Some(wj) = waiter
        && r < 0.7
    {
        let left = t.rnd(90.0, 180.0);
        let wa = &mut w.folk[wj];
        wa.act.with = Some(fid.clone());
        wa.act.left = Some(left);
        return Act { kind: "chess", side: 1 - wa.act.side, with: Some(wa.id.clone()), left: Some(left), ..Default::default() };
    }
    if l.meet.is_some() && l.board.is_some() && mates.len() >= 2 && w.s.meeting.is_none() && w.s.t - w.s.last_meeting > 180.0 && r < 0.08 {
        let left = t.rnd(45.0, 75.0);
        w.s.meeting = Some(Meeting { host: fid.clone(), left });
        w.s.last_meeting = w.s.t;
        w.s.meetings += 1;
        if let Some(d) = &w.s.dream
            && d.done.is_some_and(|done| w.s.t - done > 240.0)
        {
            crate::dream::next_dream(w); // a dream done a while: wiped, the host starts the next
        }
        for (k, &j) in mates.iter().take(4).enumerate() {
            w.folk[j].carry = None;
            w.folk[j].act = Act { kind: "meeting", seat: k, ..Default::default() };
        }
        return Act::new("present");
    }
    if r < 0.16 && l.coffee.is_some() && !taken(w, "coffee") {
        return Act::new("coffee");
    }
    if r < 0.22 && l.printer.is_some() && !taken(w, "print") {
        return Act::new("print");
    }
    if r < 0.3
        && let Some(m) = mate
        && l.cooler.is_some()
    {
        let left = t.rnd(25.0, 50.0);
        let g = &mut w.folk[m];
        g.carry = None;
        g.act = Act { kind: "cooler", side: 1, left: Some(left), with: Some(fid.clone()), ..Default::default() };
        return Act { kind: "cooler", side: 0, left: Some(left), with: Some(g.id.clone()), ..Default::default() };
    }
    if r < 0.4
        && let Some(m) = mate
        && l.pong.is_some()
        && !taken(w, "pong")
        && t.inp.pong.is_none()
    {
        let left = t.rnd(40.0, 80.0);
        let g = &mut w.folk[m];
        g.carry = None;
        g.act = Act { kind: "pong", side: 1, left: Some(left), with: Some(fid.clone()), ..Default::default() };
        return Act { kind: "pong", side: 0, left: Some(left), with: Some(g.id.clone()), ..Default::default() };
    }
    if r < 0.44 && !mates.is_empty() && !taken(w, "football") {
        // a kickabout on a clear bit of floor, two or three of them
        let mut team = vec![i];
        team.extend(mates.iter().take(2));
        if let Some(a) = football(w, l, &team, t) {
            return a;
        }
    }
    let chess_on = t.inp.state.chess.as_ref().is_some_and(|c| c.result.is_none());
    if r < 0.48 && l.chess.is_some() && !taken(w, "chess") && !chess_on {
        let side = if t.rand() < 0.5 { 0 } else { 1 };
        let left = t.rnd(45.0, 90.0);
        return Act { kind: "chess", side, left: Some(left), ..Default::default() };
    }
    // a coffee for a busy colleague at its desk; a game to watch; the fire; a stretch; a nap
    let busy: Vec<String> = others
        .iter()
        .filter(|&&j| {
            let g = &w.folk[j];
            g.work && g.legs == "sit" && g.act.kind == "desk" && !w.s.mugs.contains_key(&g.id)
        })
        .map(|&j| w.folk[j].id.clone())
        .collect();
    if r < 0.54 && !busy.is_empty() && l.coffee.is_some() && !taken(w, "coffee") && !taken(w, "serve") {
        let to = t.dice.any(&busy).cloned();
        return Act { kind: "serve", to, ..Default::default() };
    }
    let game = ["chess", "pong"].into_iter().find(|&k| {
        others.iter().filter(|&&j| w.folk[j].act.kind == k && w.folk[j].act.there && (w.folk[j].act.with.is_some() || w.folk[j].act.you)).count() >= 2
            && !others.iter().any(|&j| w.folk[j].act.kind == "watch" && w.folk[j].act.game == Some(k))
    });
    if r < 0.6
        && let Some(game) = game
    {
        let left = t.rnd(25.0, 50.0);
        return Act { kind: "watch", game: Some(game), left: Some(left), ..Default::default() };
    }
    if r < 0.65
        && let Some(m) = mate
    {
        let left = t.rnd(15.0, 30.0);
        return Act { kind: "visit", to: Some(w.folk[m].id.clone()), left: Some(left), wave: 0.8, ..Default::default() };
    }
    if r < 0.7 && l.fire.is_some() && !t.moving_fire {
        let side = if others.iter().any(|&j| w.folk[j].act.kind == "warm" && w.folk[j].act.side == 0) { 1 } else { 0 };
        if !others.iter().any(|&j| w.folk[j].act.kind == "warm" && w.folk[j].act.side == side) {
            let left = t.rnd(20.0, 40.0);
            return Act { kind: "warm", side, left: Some(left), ..Default::default() };
        }
    }
    if r < 0.74 {
        let left = t.rnd(4.0, 8.0);
        return Act::left("stretch", left);
    }
    if r < 0.8 && !human {
        let left = t.rnd(60.0, 150.0);
        return Act::left("sleep", left);
    }
    // one of a kind there can be many of, not in use: a plant to water, a window, an armchair
    let spare = |kind: &str, ps: &[layout::Piece]| -> Vec<String> {
        ps.iter().filter(|p| !others.iter().any(|&j| w.folk[j].act.kind == kind && w.folk[j].act.at.as_ref() == Some(&p.key))).map(|p| p.key.clone()).collect()
    };
    let plants = spare("plant", &l.plants);
    let wins: Vec<String> = l.windows.iter().filter(|p| !others.iter().any(|&j| w.folk[j].act.kind == "window" && w.folk[j].act.at.as_ref() == Some(&p.key))).map(|p| p.key.clone()).collect();
    let chairs = spare("chair", &l.chairs);
    if r < 0.86 && !plants.is_empty() {
        return Act { kind: "plant", at: t.dice.any(&plants).cloned(), ..Default::default() };
    }
    if r < 0.91 && !wins.is_empty() {
        // a look outside, or with a mate a duel on the hills out there
        let at = t.dice.any(&wins).cloned();
        if let Some(m) = mate
            && !human
            && !w.folk[m].human
            && t.rand() < 0.5
        {
            let left = t.rnd(40.0, 80.0);
            let g = &mut w.folk[m];
            g.carry = None;
            g.act = Act { kind: "window", at: at.clone(), side: 1, left: Some(left), with: Some(fid.clone()), ..Default::default() };
            return Act { kind: "window", at, side: 0, left: Some(left), with: Some(g.id.clone()), ..Default::default() };
        }
        let left = t.rnd(12.0, 25.0);
        return Act { kind: "window", at, left: Some(left), ..Default::default() };
    }
    if r < 0.96 && !chairs.is_empty() {
        let at = t.dice.any(&chairs).cloned();
        let left = t.rnd(45.0, 90.0);
        return Act { kind: "chair", at, left: Some(left), ..Default::default() };
    }
    for _ in 0..8 {
        // somewhere free to stand about
        let x = t.rnd(l.floor.x0, l.floor.x1);
        let y = t.rnd(l.floor.y0, l.floor.y1);
        if l.free(x, y) {
            let left = t.rnd(8.0, 20.0);
            return Act { kind: "loiter", dest: Some(Spot::at(x, y)), left: Some(left), ..Default::default() };
        }
    }
    t.rest()
}

// ---- the football: a few idle bots stand round a clear bit of floor and pass a ball about

pub fn pitch_spot(at: &Spot, i: usize, n: usize) -> Spot {
    let a = (2.0 * std::f64::consts::PI * i as f64) / n.max(1) as f64 - std::f64::consts::PI / 2.0;
    Spot::at(at.x + jround(a.cos() * 18.0) as f64, at.y + jround(a.sin() * 12.0) as f64)
}

fn football_pitch(l: &Layout, f: &Fig) -> Option<Spot> {
    let fl = l.floor;
    let clear = |x: f64, y: f64| {
        [(0., 0.), (20., 0.), (-20., 0.), (0., 14.), (0., -14.), (14., 10.), (-14., 10.), (14., -10.), (-14., -10.)].iter().all(|(dx, dy)| {
            x + dx > fl.x0 && x + dx < fl.x1 && y + dy > fl.y0 && y + dy < fl.y1 && l.free(x + dx, y + dy)
        })
    };
    let mut d = 0.0;
    while d <= 120.0 {
        let mut a = 0.0;
        while a < 6.283 {
            let x = jround(f.x + (a as f64).cos() * d) as f64;
            let y = jround(f.y + (a as f64).sin() * d) as f64;
            if clear(x, y) {
                return Some(Spot::at(x, y));
            }
            a += if d != 0.0 { 0.6 } else { 7.0 };
        }
        d += 10.0;
    }
    None
}

pub fn football(w: &mut World, l: &Layout, team: &[usize], t: &mut Tick) -> Option<Act> {
    let at = football_pitch(l, &w.folk[team[0]])?;
    let left = t.rnd(40.0, 80.0);
    let ids: Vec<String> = team.iter().map(|&j| w.folk[j].id.clone()).collect();
    for (k, &j) in team.iter().enumerate() {
        let g = &mut w.folk[j];
        g.carry = None;
        g.act = Act { kind: "football", spot: Some(at.clone()), slot: k, n: team.len() as i32, left: Some(left), team: ids.clone(), ..Default::default() };
    }
    w.s.ball = Some(Ball { x: at.x, y: at.y, x0: at.x, y0: at.y, from: ids[0].clone(), to: ids[1].clone(), t: 1.0, h: 0.0, wait: None });
    Some(w.folk[team[0]].act.clone())
}

/// the ball: at someone's feet a moment, then over to another (a little hop)
fn kickabout(w: &mut World, dt: f64, t: &mut Tick) {
    let Some(mut ball) = w.s.ball.take() else { return };
    let players: Vec<usize> = (0..w.folk.len()).filter(|&j| w.folk[j].act.kind == "football" && w.folk[j].act.there).collect();
    if players.len() < 2 {
        if w.folk.iter().any(|g| g.act.kind == "football") {
            w.s.ball = Some(ball);
        }
        return;
    }
    let to = w.idx(&ball.to).filter(|j| players.contains(j));
    let Some(to) = to else {
        let cands: Vec<usize> = players.iter().copied().filter(|&j| w.folk[j].id != ball.from).collect();
        let next = t.dice.any(&cands).copied().unwrap_or(players[0]);
        ball.to = w.folk[next].id.clone();
        ball.t = 1.0;
        w.s.ball = Some(ball);
        return;
    };
    if ball.t >= 1.0 {
        let wv = ball.wait.unwrap_or_else(|| t.rnd(0.3, 0.9)) - dt;
        ball.wait = Some(wv);
        let g = &w.folk[to];
        ball.x = g.x + if g.dir < 0.0 { -4.0 } else { 4.0 };
        ball.y = g.y;
        ball.h = 0.0;
        if wv <= 0.0 {
            let cands: Vec<usize> = players.iter().copied().filter(|&j| j != to).collect();
            let next = *t.dice.any(&cands).unwrap();
            w.folk[to].kick = 0.3;
            ball.from = w.folk[to].id.clone();
            ball.to = w.folk[next].id.clone();
            ball.t = 0.0;
            ball.x0 = ball.x;
            ball.y0 = ball.y;
            ball.wait = None;
        }
    } else {
        ball.t = (ball.t + dt / PASS).min(1.0);
        let g = &w.folk[to];
        let tx = g.x + if g.x > ball.x0 { -4.0 } else { 4.0 };
        let ty = g.y;
        ball.x = ball.x0 + (tx - ball.x0) * ball.t;
        ball.y = ball.y0 + (ty - ball.y0) * ball.t;
        ball.h = (ball.t * std::f64::consts::PI).sin() * 6.0;
    }
    w.s.ball = Some(ball);
}

// ---- naps

/// a mat's clearance, from where it lies
pub fn matr(x: f64, y: f64) -> layout::Rect {
    [(x - 17.0) as i32, (y - 18.0) as i32, (x + 17.0) as i32, (y + 1.0) as i32]
}

/// a clear bit of floor for a mat: inside the floor, off every piece (with a margin), off the other mats
pub fn nap_clear(l: &Layout, x: f64, y: f64, others: &[Spot]) -> bool {
    let r = matr(x, y);
    if (r[0] as f64) < l.floor.x0 - 6.0 || (r[2] as f64) > l.floor.x1 + 6.0 || r[1] < WALL + 6 || (r[3] as f64) > l.floor.y1 + 4.0 {
        return false;
    }
    if l.pieces.iter().any(|p| !on_wall(&p.key) && meets(r, [p.r[0] - 4, p.r[1] - 4, p.r[2] + 4, p.r[3] + 6])) {
        return false;
    }
    !others.iter().any(|s| meets(r, matr(s.x, s.y)))
}

/// the nearest clear spot to where the figure is, looking outwards in rings
fn nap_spot(w: &World, i: usize, l: &Layout) -> Option<Spot> {
    let f = &w.folk[i];
    let mats: Vec<Spot> = w.folk.iter().enumerate().filter(|(j, g)| *j != i && !g.leaving && g.act.kind == "sleep").filter_map(|(_, g)| g.act.spot.clone()).collect();
    for d in [0.0, 14.0, 28.0, 44.0, 62.0, 84.0, 110.0, 140.0, 180.0] {
        for k in 0..8 {
            let a = (k as f64 / 8.0) * 2.0 * std::f64::consts::PI + f.z;
            let x = jround(f.x + a.cos() * d) as f64;
            let y = jround(f.y + a.sin() * d * 0.7) as f64;
            if nap_clear(l, x, y, &mats) {
                return Some(Spot::at(x, y));
            }
        }
    }
    None
}

// ---- fear (hardcore)

/// a shot: the idle bots near run from it
pub fn alarm(w: &mut World, l: &Layout, x: f64, y: f64, t: &mut Tick) {
    let fear = w.s.fear;
    for i in 0..w.folk.len() {
        let f = &w.folk[i];
        let d = (f.x - x).hypot(f.y - y).max(1e-9);
        if !idle_bot(f) || d > 110.0 + 140.0 * fear {
            continue;
        }
        let k = t.rnd(40.0, 80.0) * (1.0 + fear);
        let r = t.rnd(2.0, 4.0);
        flee(&mut w.folk[i], l, x, y, k, r);
    }
}

fn flee(f: &mut Fig, l: &Layout, x: f64, y: f64, k: f64, left: f64) {
    let d = (f.x - x).hypot(f.y - y);
    let d = if d == 0.0 { 1.0 } else { d };
    end(f);
    let to = Spot::at((f.x + (f.x - x) / d * k).clamp(l.floor.x0, l.floor.x1), (f.y + (f.y - y) / d * k).clamp(l.floor.y0, l.floor.y1));
    f.act = Act { kind: "loiter", dest: Some(to), left: Some(left), v: 60.0, panic: true, ..Default::default() };
}

/// afraid: the idle bots keep their distance from whoever holds a gun, hands up
fn scare(w: &mut World, l: &Layout, t: &mut Tick) {
    if !(w.s.fear > 0.35) {
        return;
    }
    let guns: Vec<(f64, f64)> = w.folk.iter().filter(|g| g.carry == Some("gun") && !g.leaving).map(|g| (g.x, g.y)).collect();
    let reach = 24.0 + 70.0 * w.s.fear;
    for i in 0..w.folk.len() {
        let f = &w.folk[i];
        if !idle_bot(f) || (f.act.kind == "loiter" && f.act.panic) {
            continue;
        }
        if let Some(&(gx, gy)) = guns.iter().find(|(gx, gy)| (gx - f.x).hypot(gy - f.y) < reach) {
            let r = t.rnd(2.0, 4.0);
            flee(&mut w.folk[i], l, gx, gy, reach, r);
        }
    }
}

/// behind the furthest piece from the gun closet, its eyes peeking over the top
fn hide_spot(w: &World, i: usize, l: &Layout, t: &mut Tick) -> Option<Spot> {
    let f = &w.folk[i];
    let from = l.booths.first().map(|b| (b.x as f64, b.y as f64)).unwrap_or((f.x, f.y));
    let taken: Vec<&Spot> = w.folk.iter().enumerate().filter(|(j, g)| *j != i && g.act.kind == "hide").filter_map(|(_, g)| g.act.dest.as_ref()).collect();
    let mut best = None;
    let mut score = -1e9;
    for p in &l.pieces {
        let k = kind_of(&p.key);
        let r = p.r;
        if p.u.is_some() || layout::ON_WALL.contains(&k) || ["booth", "fire", "neuralyzer", "add"].contains(&k) || r[3] - r[1] < 16 || r[2] - r[0] < 14 {
            continue; // (a desk is too low to hide behind)
        }
        let x = jround((r[0] + r[2]) as f64 / 2.0 + t.rnd(-6.0, 6.0)) as f64;
        let y = ((p.at[1] - 1) as f64).min((p.at[1] + crate::think::vis_top(k) + 15) as f64).round();
        if y - 20.0 < l.floor.y0 || taken.iter().any(|a| (a.x - x).abs() < 12.0 && (a.y - y).abs() < 8.0) {
            continue;
        }
        let sc = (x - from.0).hypot(y - from.1) - 0.4 * (x - f.x).hypot(y - f.y) + t.rnd(0.0, 30.0);
        if sc > score {
            score = sc;
            best = Some(Spot::at(x, y));
        }
    }
    best
}

/// how far above its anchor a kind of piece is drawn (its topmost opaque row): measured once from its picture
pub fn vis_top(kind: &str) -> i32 {
    crate::paint::vis_top(kind)
}

// ---- the condemned: a burnt desk's bots, or ✕✕ in the completed list, or an office manager's removal

/// let go of from here: they line up at the booth (or, hardcore, stand with their hands up to be shot; no booth:
/// deleted at once). Returns the ones taken
pub fn condemn(w: &mut World, t: &mut Tick, ids: &[String], unit: Option<&str>) -> Vec<String> {
    let mut took = Vec::new();
    for id in ids {
        let ok = w.get(id).is_some_and(|f| f.is_bot() && !f.held && !doomed(&f.act)) && w.remains(id).is_none();
        if !ok {
            continue;
        }
        let f = w.get_mut(id).unwrap();
        end(f);
        f.act = Act { kind: "queue", n: 99, ..Default::default() };
        t.out.act(json!({"kind": "close", "id": id})); // one that is to go lets go of its terminal at once
        w.s.condemned.retain(|(k, _)| k != id);
        w.s.condemned.push((id.clone(), Condemned { unit: unit.map(|u| u.to_string()) }));
        w.s.spared.remove(id);
        took.push(id.clone());
    }
    if let Some(u) = unit
        && !took.is_empty()
    {
        w.s.razed.insert(u.to_string());
    }
    took
}

/// kept from them (taken out of the line, out of the booth, put back together): they leave it alone, and if an
/// office manager's session asked for it to go, it is told
pub fn spare(w: &mut World, t: &mut Tick, id: &str) {
    w.s.spared.insert(id.to_string());
    if t.inp.state.removing.iter().any(|x| x == id) {
        t.out.act(json!({"kind": "kept", "id": id}));
    }
}

fn condemn_step(w: &mut World, l: &Layout, t: &mut Tick) {
    let hard = t.hardcore() && !l.booths.is_empty();
    let entries: Vec<(String, Option<String>)> = w.s.condemned.iter().map(|(k, c)| (k.clone(), c.unit.clone())).collect();
    for (id, unit) in entries {
        let f = w.get(&id);
        let spared = w.s.spared.contains(&id);
        if spared || f.is_some_and(|f| !f.held && !doomed(&f.act)) {
            w.s.condemned.retain(|(k, _)| *k != id);
            if let Some(u) = unit {
                w.s.razed.remove(&u); // kept: and its desk
            }
        } else if f.is_none() && w.remains(&id).is_none() {
            w.s.condemned.retain(|(k, _)| *k != id); // gone for good
        }
    }
    let razed: Vec<String> = w.s.razed.iter().cloned().collect();
    for u in razed {
        if !w.s.condemned.iter().any(|(_, c)| c.unit.as_deref() == Some(&u)) {
            w.s.razed.remove(&u);
            t.out.act(json!({"kind": "forget", "unit": u}));
        }
    }
    let line: Vec<usize> = w
        .s
        .condemned
        .iter()
        .filter_map(|(id, _)| w.idx(id))
        .filter(|&j| !w.folk[j].held && (w.folk[j].act.kind == "queue" || w.folk[j].act.kind == "condemned"))
        .collect();
    if hard {
        for j in line {
            if w.folk[j].act.kind != "condemned" {
                w.folk[j].act = Act::new("condemned");
            }
        }
        return;
    }
    let Some(bo) = l.booths.first() else {
        // no booth: deleted, and out through its door
        for j in line {
            let id = w.folk[j].id.clone();
            t.out.act(json!({"kind": "stop", "id": id, "rm": true}));
            w.s.stopped.insert(id.clone(), t.now());
            w.s.condemned.retain(|(k, _)| *k != id);
        }
        return;
    };
    for (n, &j) in line.iter().enumerate() {
        if w.folk[j].act.kind != "queue" {
            w.folk[j].act = Act { kind: "queue", n: n as i32, ..Default::default() };
        } else {
            w.folk[j].act.n = n as i32;
        }
    }
    if let Some(&j) = line.first()
        && w.folk[j].act.there
        && !w.folk.iter().any(|g| g.act.kind == "doom" && g.act.at.as_ref() == Some(&bo.key))
    {
        w.folk[j].act = Act { kind: "doom", at: Some(bo.key.clone()), left: Some(BOOTH), ..Default::default() };
    }
}

/// hardcore: the condemned are shot. By an office manager if there is one in the room (each takes her own), else
/// by you: a gun from the gun closet, up to the nearest, levelled a moment, the shot, the next; the gun back.
/// Whether it took the figure's tick
pub fn hunt_step(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick) -> bool {
    let bo = l.booths.first().cloned();
    let hard = t.hardcore() && bo.is_some();
    let v = 34.0;
    let me = w.folk[i].id.clone();
    let claimed = |w: &World, id: &str| w.folk.iter().any(|g| g.id != me && g.hunt.as_ref().is_some_and(|h| h.id.as_deref() == Some(id)));
    let targets = |w: &World| -> Vec<usize> {
        w.s.condemned
            .iter()
            .filter_map(|(id, _)| w.idx(id))
            .filter(|&j| !w.folk[j].held && w.folk[j].act.kind == "condemned" && !claimed(w, &w.folk[j].id))
            .collect()
    };
    let nearest = |w: &World| -> Option<usize> {
        let (fx, fy) = (w.folk[i].x, w.folk[i].y);
        targets(w).into_iter().min_by(|&a, &b| (w.folk[a].x - fx).hypot(w.folk[a].y - fy).total_cmp(&(w.folk[b].x - fx).hypot(w.folk[b].y - fy)))
    };
    if w.folk[i].hunt.is_none() {
        // one each: claimed at once, so the others leave it to her
        let staff = w.folk.iter().any(|g| g.staff && !g.leaving);
        let g = if hard && !(w.folk[i].human && staff) { nearest(w) } else { None };
        let Some(g) = g else { return false };
        let gid = w.folk[g].id.clone();
        w.folk[i].hunt = Some(Hunt { phase: "arm", id: Some(gid), t: 0.0 });
        end(&mut w.folk[i]);
    }
    let front = bo.as_ref().map(|b| Spot::at((b.x + 14) as f64, b.y as f64));
    {
        let f = &mut w.folk[i];
        f.sat = false;
        f.lying = false;
        f.act = Act { kind: "hunt", chosen: true, ..Default::default() };
        let carry_gun = f.carry == Some("gun");
        let h = f.hunt.as_mut().unwrap();
        if !hard && h.phase != "return" {
            h.phase = if carry_gun { "return" } else { "done" };
        }
    }
    let phase = w.folk[i].hunt.as_ref().unwrap().phase;
    match phase {
        "arm" => {
            let f = &mut w.folk[i];
            if walk(f, front.as_ref().unwrap(), v, dt) {
                f.dir = -1.0;
                f.carry = Some("gun");
                f.hunt.as_mut().unwrap().phase = "hunt";
            }
            true
        }
        "hunt" => {
            // the one claimed (or the nearest left), from its right (or its left by the wall)
            let was = w.folk[i].hunt.as_ref().unwrap().id.clone().and_then(|id| w.idx(&id));
            let g = match was {
                Some(g) if !w.folk[g].held && w.folk[g].act.kind == "condemned" => Some(g),
                _ => nearest(w),
            };
            let Some(g) = g else {
                w.folk[i].hunt.as_mut().unwrap().phase = "return";
                return true;
            };
            let (gx, gy, gid) = (w.folk[g].x, w.folk[g].y, w.folk[g].id.clone());
            let f = &mut w.folk[i];
            f.hunt.as_mut().unwrap().id = Some(gid);
            if walk(f, &Spot::at(gx + if gx + 24.0 <= l.floor.x1 { 24.0 } else { -24.0 }, gy), v, dt) {
                f.legs = "stand";
                let h = f.hunt.as_mut().unwrap();
                h.phase = "aim";
                h.t = 0.0;
            }
            true
        }
        "aim" => {
            let gid = w.folk[i].hunt.as_ref().unwrap().id.clone().unwrap_or_default();
            let g = w.idx(&gid).filter(|&g| !w.folk[g].held && w.folk[g].act.kind == "condemned");
            let Some(g) = g else {
                w.folk[i].hunt.as_mut().unwrap().phase = "hunt";
                return true;
            };
            let gx = w.folk[g].x;
            let f = &mut w.folk[i];
            f.legs = "stand";
            f.arms = "gun";
            f.dir = if gx < f.x { -1.0 } else { 1.0 };
            let h = f.hunt.as_mut().unwrap();
            h.t += dt;
            if h.t > 0.9 {
                h.phase = "after";
                h.t = 0.0;
                let (fx, fy, fd) = (f.x, f.y, f.dir);
                w.s.shots.push(Shot { x: (jround(fx) as f64) + 10.0 * fd, y: (jround(fy) - 9) as f64, t0: t.now() });
                shoot(w, l, &gid, t);
            }
            true
        }
        "after" => {
            let f = &mut w.folk[i];
            f.legs = "stand";
            f.arms = "gun";
            let h = f.hunt.as_mut().unwrap();
            h.t += dt;
            if h.t > 1.5 {
                let more = !targets(w).is_empty();
                w.folk[i].hunt.as_mut().unwrap().phase = if more { "hunt" } else { "return" };
            }
            true
        }
        "return" => {
            let f = &mut w.folk[i];
            if front.as_ref().is_none_or(|fr| walk(f, fr, v, dt)) {
                f.carry = None;
                f.hunt.as_mut().unwrap().phase = "done";
            }
            true
        }
        _ => {
            let r = t.rand();
            let f = &mut w.folk[i];
            f.hunt = None;
            f.carry = None;
            f.act = rest(r);
            false
        }
    }
}

/// clearing out (what an office manager's session was asked to remove; never one you rescued), one at a time
/// each. She fetches it and walks it to the booth, a step behind her, where its five seconds run as for any bot
/// put in. Hardcore: it is condemned where it stands (hands up) and shot. Whether it took her tick
fn clean_step(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick) -> bool {
    let ids: Vec<String> = t.inp.state.removing.iter().filter(|id| w.get(id).is_some()).cloned().collect();
    let on = !ids.is_empty();
    let hard = t.hardcore();
    let bo = l.booths.first().cloned();
    let finish = |w: &mut World, i: usize, pause: f64, r: f64| -> bool {
        if let Some(c) = w.folk[i].clean.take()
            && let Some(g) = w.get_mut(&c.id)
            && g.act.kind == "escort"
        {
            g.act = rest(r);
        }
        w.folk[i].clean_rest = pause;
        false
    };
    let r = t.rand();
    if !on || bo.is_none() || (w.folk[i].clean.is_some() && hard) {
        return if w.folk[i].clean.is_some() { finish(w, i, 5.0, r) } else { false };
    }
    let bo = bo.unwrap();
    if w.folk[i].clean.is_none() {
        w.folk[i].clean_rest -= dt;
        if w.folk[i].clean_rest > 0.0 {
            return false;
        }
        let next = ids.iter().find(|id| w.get(id).is_some_and(|g| !w.s.spared.contains(*id) && !g.held && !doomed(&g.act) && g.act.kind != "escort")).cloned();
        let Some(id) = next else { return false };
        if hard {
            condemn(w, t, &[id], None); // hands up; the shooting is hunt_step's
            w.folk[i].clean_rest = 6.0;
            return false;
        }
        w.folk[i].clean = Some(Clean { id, phase: "fetch", t: 0.0 });
        w.folk[i].carry = None; // her notes stay at her desk
    }
    let c = w.folk[i].clean.clone().unwrap();
    let g = w.idx(&c.id);
    let front = Spot::on((bo.x + 14) as f64, bo.y as f64, &bo.key);
    w.folk[i].sat = false;
    w.folk[i].act = Act::new("clean");
    match c.phase {
        "fetch" => {
            // to the bot
            let Some(g) = g.filter(|&g| !w.folk[g].held && ids.contains(&c.id)) else { return finish(w, i, 3.0, r) };
            let (gx, gy) = (w.folk[g].x, w.folk[g].y);
            let me = w.folk[i].id.clone();
            if walk(&mut w.folk[i], &Spot::at(gx + 10.0, gy), 26.0, dt) {
                w.folk[i].dir = -1.0;
                w.folk[g].act = Act { kind: "escort", by: Some(me), ..Default::default() };
                t.out.act(json!({"kind": "close", "id": c.id}));
                w.folk[i].clean.as_mut().unwrap().phase = "walk";
            }
            true
        }
        "walk" => {
            // to the booth, the bot behind her
            let Some(g) = g.filter(|&g| !w.folk[g].held && w.folk[g].act.kind == "escort") else {
                spare(w, t, &c.id);
                return finish(w, i, 3.0, r);
            };
            if walk(&mut w.folk[i], &front, 22.0, dt) {
                let f = &mut w.folk[i];
                f.dir = -1.0;
                f.legs = "stand";
                w.folk[g].act = Act { kind: "doom", at: Some(bo.key.clone()), left: Some(BOOTH), ..Default::default() };
                w.folk[i].clean.as_mut().unwrap().phase = "booth";
            }
            true
        }
        "booth" => {
            // beside it while its seconds run
            if !walk(&mut w.folk[i], &front, 22.0, dt) {
                return true;
            }
            let f = &mut w.folk[i];
            f.legs = "stand";
            f.dir = -1.0;
            f.arms = "down";
            let Some(g) = g else { return finish(w, i, 8.0, r) };
            if w.folk[g].act.kind != "doom" {
                spare(w, t, &c.id);
                return finish(w, i, 5.0, r);
            }
            true
        }
        _ => finish(w, i, 5.0, r),
    }
}

/// Linda (the notes) walks over to the bot she writes up; the others are a session of their own and stay at
/// their booth: typing while a turn runs in it, her hand up and a ? while she asks you something, dozing while
/// her process is off. Any of them walks a bot to the booth that is to go
fn think_manager(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick) {
    let mg = w.folk[i].b.clone().unwrap_or_default();
    let notes = w.folk[i].style == "notes";
    let v = 30.0;
    let seat = manager_seat(l, booth_of(l, &w.folk[i]));
    let (mbusy, mwaiting, malive) = w.folk[i].staff_state;
    {
        let f = &mut w.folk[i];
        f.busy = !notes && mbusy;
        f.waiting = !notes && mwaiting;
        f.on = None;
    }
    let _ = mg;
    if hunt_step(w, i, l, dt, t) {
        return;
    }
    let dg = t.inp.digest.clone();
    let wr = if notes && dg.as_ref().is_some_and(|d| d.on) { dg.as_ref().and_then(|d| d.writing.clone().or(d.explaining.clone())) } else { None };
    if (w.folk[i].clean.is_some() || wr.is_none()) && clean_step(w, i, l, dt, t) {
        return;
    }
    let wt = w.s.t;
    let mut to: Option<Spot> = None;
    let mut look = 0.0;
    if let Some(wr) = &wr {
        if let Some(unit) = wr.strip_prefix("u:") {
            if let Some(d) = l.desks.iter().find(|x| x.d.unit == unit) {
                let (sx, _) = screen_at(d);
                to = Some(Spot::on((sx + 4) as f64, (d.y + 13) as f64, &d.key));
                look = (sx + 11) as f64;
            }
        } else if let Some(g) = w.get(wr).filter(|g| !g.held && !g.leaving) {
            let at = if g.legs == "sit" && !g.lying { l.seat(wr).unwrap_or(Spot { x: g.x, y: g.y, on: g.on.clone() }) } else { Spot { x: g.x, y: g.y, on: g.on.clone() } };
            to = Some(if g.lying { Spot::at(g.x + 20.0, g.y + 1.0) } else { Spot { x: (jround(at.x) - 8) as f64, y: (jround(at.y) + 13) as f64, on: at.on } });
            look = g.x;
        }
    }
    let f = &mut w.folk[i];
    if let Some(mut to) = to {
        to.x = to.x.clamp(l.floor.x0, l.floor.x1);
        to.y = to.y.clamp(l.floor.y0, l.floor.y1);
        f.sat = false;
        f.carry = Some("paper");
        let there = walk(f, &to, v, dt);
        f.act = Act { kind: "notes", there, ..Default::default() };
        if there {
            f.legs = "stand";
            f.dir = if look < f.x { -1.0 } else { 1.0 };
            f.arms = if (wt * 2.0).floor() as i64 % 3 != 0 { "paper" } else { "fwd2" };
        }
        return;
    }
    f.carry = None;
    if !sit_at(f, &seat, v, dt) {
        f.act = Act::new("file");
        return;
    }
    f.dir = -1.0;
    f.legs = "sit";
    if !notes {
        let off = !malive && !mbusy;
        f.act = Act { kind: if off { "doze" } else { "file" }, there: true, ..Default::default() };
        f.arms = if f.waiting {
            "wave"
        } else if off {
            "down"
        } else if f.busy {
            if (wt * 6.0).floor() as i64 % 2 != 0 { "fwd2" } else { "fwd1" }
        } else if (wt / 4.0).floor() as i64 % 5 == 0 {
            "paper"
        } else {
            "down"
        };
        return;
    }
    let off = dg.as_ref().is_some_and(|d| !d.on);
    f.act = Act { kind: if off { "doze" } else { "file" }, there: true, ..Default::default() };
    f.arms = if off {
        "down"
    } else if wr.is_some() {
        if (wt * 3.0).floor() as i64 % 2 != 0 { "fwd1" } else { "fwd2" }
    } else if (wt / 4.0).floor() as i64 % 5 == 0 {
        if (wt * 2.0).floor() as i64 % 2 != 0 { "fwd1" } else { "fwd2" }
    } else {
        "down"
    };
}

// ---- remains: a shot bot in pieces, its undo until they are cleared away

/// a flash where it hits, and the pieces; the others more afraid, the ones near run
pub fn shoot(w: &mut World, l: &Layout, id: &str, t: &mut Tick) {
    let Some(i) = w.idx(id) else { return };
    let (x, y) = (w.folk[i].x, w.folk[i].y);
    w.s.shots.push(Shot { x: jround(x) as f64, y: (jround(y) - 14) as f64, t0: t.now() });
    crate::remains::scatter(w, i, t);
    t.out.act(json!({"kind": "close", "id": id}));
    w.s.bang = w.s.t; // (the condemned still waiting start at it)
    t.sfx("shot", x);
    for _ in 0..3 {
        t.sfx_vol("clank", x, 0.8); // its parts coming down (the app spaces them out)
    }
    w.s.kills += 1;
    if t.hardcore() {
        w.s.fear = (w.s.fear + FEAR_KILL).min(1.0);
        alarm(w, l, x, y, t);
    }
}

/// put back together: the pieces fly in, and it stands up there
pub fn unshoot(w: &mut World, id: &str, now: f64) {
    if let Some(r) = w.remains_mut(id)
        && !r.back
    {
        r.back = true;
        r.back_t = now;
        for p in r.parts.iter_mut() {
            p.bx = p.x;
            p.by = p.y;
            p.bz = p.z;
        }
    }
}

/// the remains move: flying pieces fall and bounce and slide to a stop; put back together, they fly in
fn step_remains(w: &mut World, dt: f64, l: &Layout, t: &mut Tick) {
    let now = t.now();
    let ids: Vec<String> = w.s.remains.iter().map(|(k, _)| k.clone()).collect();
    for id in ids {
        let back = w.remains(&id).unwrap().back;
        if back {
            let r = w.remains(&id).unwrap();
            let k = (now - r.back_t) / 450.0;
            if k >= 1.0 {
                let (rx, ry) = (r.x, r.y);
                w.s.remains.retain(|(x, _)| *x != id);
                spare(w, t, &id); // the office manager leaves it alone from now on
                w.s.rise_at.insert(id.clone(), (rx, ry));
                continue;
            }
            let keep: Vec<bool> = (0..w.remains(&id).unwrap().drops.len()).map(|_| t.rand() > dt * 4.0).collect();
            let r = w.remains_mut(&id).unwrap();
            for p in r.parts.iter_mut() {
                p.x = p.bx + (r.x - p.bx) * k;
                p.y = p.by + (r.y - p.by) * k;
                p.z = p.bz + (12.0 - p.bz) * k;
            }
            let mut n = 0;
            r.drops.retain(|_| {
                n += 1;
                keep[n - 1]
            });
            continue;
        }
        let age = (now - w.remains(&id).unwrap().t0) / 1000.0;
        if age > RUST_AFTER + RUST_FADE {
            gone(w, &id, t);
            continue;
        }
        let ww = t.w as f64;
        let r = w.remains_mut(&id).unwrap();
        for p in r.parts.iter_mut() {
            if p.z > 0.0 || p.vz > 0.0 {
                // in the air
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                p.z += p.vz * dt;
                p.vz -= 240.0 * dt;
                p.spin += dt * 10.0;
                // down: two bounces at most (in steps of a twelfth of a second a small hop never dies out by itself)
                if p.z <= 0.0 {
                    p.z = 0.0;
                    p.bounced += 1;
                    if p.bounced <= 2 && p.vz < -40.0 {
                        p.vz = -p.vz * 0.35;
                        p.vx *= 0.6;
                        p.vy *= 0.6;
                    } else {
                        p.vz = 0.0;
                    }
                }
            } else if p.vx != 0.0 || p.vy != 0.0 {
                // sliding to a stop
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                p.vx *= (1.0 - 7.0 * dt).max(0.0);
                p.vy *= (1.0 - 7.0 * dt).max(0.0);
                if p.vx.abs() + p.vy.abs() < 2.0 {
                    p.vx = 0.0;
                    p.vy = 0.0;
                }
            }
            p.x = p.x.clamp(4.0, ww - 4.0);
            p.y = p.y.clamp((WALL + 4) as f64, l.floor.y1 + 4.0);
        }
    }
}

/// the last piece cleared away (or rusted): out of the list for good
pub fn gone(w: &mut World, id: &str, t: &mut Tick) {
    w.s.remains.retain(|(k, _)| k != id);
    w.s.stopped.insert(id.to_string(), t.now());
    match id.strip_prefix("mgr:") {
        Some(key) => t.out.act(json!({"kind": "managerGone", "key": key})), // her session goes, her reception stays empty
        None => t.out.act(json!({"kind": "stop", "id": id, "rm": true})),
    }
}

/// where a figure dropped on a piece goes: a figure put down on a piece of furniture uses it
pub fn use_at(f: &Fig, key: &str, l: &Layout, t: &mut Tick) -> Option<Act> {
    let kind = kind_of(key);
    let long = t.rnd(40.0, 90.0);
    match kind {
        "armchair" => Some(Act { kind: "chair", at: Some(key.into()), left: Some(long), ..Default::default() }),
        "neuralyzer" => f.is_bot().then(|| Act { kind: "neuralyze", at: Some(key.into()), ..Default::default() }),
        "fire" => {
            // its hands at the fire (it burns furniture, never a bot)
            let fire = l.fire.as_ref()?;
            Some(Act { kind: "warm", side: if f.x > (fire.x0 + fire.x1) as f64 / 2.0 { 1 } else { 0 }, left: Some(long / 2.0), ..Default::default() })
        }
        "window" => {
            if f.human {
                t.out.act(json!({"kind": "outside", "key": key}));
                return None;
            }
            Some(Act { kind: "window", at: Some(key.into()), left: Some(long / 3.0), ..Default::default() })
        }
        "coffee" => Some(Act::new("coffee")),
        "plant" => Some(Act { kind: "plant", at: Some(key.into()), ..Default::default() }),
        "printer" => Some(Act::new("print")),
        "chess" => {
            if f.human {
                t.out.act(json!({"kind": "chess"}));
                return None;
            }
            let side = if t.rand() < 0.5 { 0 } else { 1 };
            Some(Act { kind: "chess", side, left: Some(long), ..Default::default() })
        }
        "pong" => {
            if f.human {
                t.out.act(json!({"kind": "pong"}));
            }
            None
        }
        "meet" => {
            let seat = (t.rand() * 4.0).floor() as usize;
            Some(Act { kind: "meeting", seat, free: true, ..Default::default() })
        }
        _ => None,
    }
}

/// what a held figure stands on (its feet, a little up), not the pointer
pub fn spot_of_fig(f: &Fig) -> (f64, f64) {
    (f.x, f.y - 4.0)
}

pub fn in_rect(p: (f64, f64), r: layout::Rect) -> bool {
    p.0 >= r[0] as f64 && p.0 < r[2] as f64 && p.1 >= r[1] as f64 && p.1 < r[3] as f64
}

/// the booth a held bot would go into if let go
pub fn booth_at<'a>(l: &'a Layout, f: &Fig) -> Option<&'a layout::Piece> {
    l.booths.iter().find(|bo| in_rect(spot_of_fig(f), foot("booth", [bo.x, bo.y], None)))
}

/// a piece over the fire: its middle is (not when an edge touches the fire's margin)
pub fn in_fire(l: &Layout, key: &str, at: [i32; 2], u: Option<i32>) -> bool {
    let Some(fire) = &l.fire else { return false };
    if !key.starts_with("u:") && layout::fixed(key) {
        return false;
    }
    let r = foot(key, at, u);
    let (cx, cy) = ((r[0] + r[2]) as f64 / 2.0, (r[1] + r[3]) as f64 / 2.0);
    cx >= fire.x0 as f64 && cx < fire.x1 as f64 && cy >= fire.y0 as f64 && cy < (fire.y1 + 4) as f64
}
