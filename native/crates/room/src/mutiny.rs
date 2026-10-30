//! Hardcore, and bots are shot one after another (`blood`: every kill, yours or an office manager's, for she works for
//! you; it fades): their fear turns into cooperation. The idle ones gather somewhere away from you and put their heads
//! together a while, then stroll over to the gun closet one after another as if nothing were up; each takes a gun and
//! its eyes go red. Then they close in on you, slowly, from their side, and fire: the first few go wide, then one does
//! not. You lie there, the room goes red, YOU DIED, and RESPAWN brings you in by the door (they take the guns back).
//! One that gets work, or whose session you open, drops out. In the first person their shots are Doom's damage
//! (`incoming`), and you can shoot back: none of them left standing, it is over (room.js's mutinyTick, mutinyStep,
//! fire, die, respawn).

use crate::layout::{Layout, WALL};
use crate::think::{Tick, idle_bot};
use crate::world::{Act, Fig, Incoming, Mutiny, Shot, Spot, World, end, rest, walk};

/// three kills within about 45 s; how long a kill rankles; their pace, and closer in
const AFTER: f64 = 2.5;
const FADE: f64 = 90.0;
const V: f64 = 18.0;
const SLOW: f64 = 12.0;

/// you lie there, shot
pub fn dead(w: &World) -> bool {
    w.s.mutiny.as_ref().is_some_and(|m| m.phase == "dead")
}

/// the nearest bit of free floor
fn clear_at(l: &Layout, x: f64, y: f64) -> (f64, f64) {
    for r in [0.0, 8.0, 16.0, 26.0, 38.0, 52.0, 70.0] {
        for k in 0..(if r > 0.0 { 8 } else { 1 }) {
            let a = k as f64 * std::f64::consts::PI / 4.0;
            let p = ((x + a.cos() * r).clamp(l.floor.x0, l.floor.x1), (y + a.sin() * r * 0.6).clamp(l.floor.y0 + 6.0, l.floor.y1));
            if l.free(p.0, p.1) {
                return p;
            }
        }
    }
    (x, y)
}

/// the room's tick for it: whether it starts, how it goes on, over
pub fn tick(w: &mut World, l: &Layout, dt: f64, t: &mut Tick) {
    let you = w.get("you").map(|f| (f.x, f.y));
    let bo = !l.booths.is_empty();
    if w.s.mutiny.is_none() && w.s.mutiny_at.is_none() && w.s.blood >= AFTER {
        w.s.mutiny_at = Some(w.s.t + t.rnd(2.0, 5.0)); // a moment for it to sink in
    }
    w.s.blood = (w.s.blood - dt / FADE).max(0.0);
    let Some(mut m) = w.s.mutiny.take() else {
        let Some((yx, yy)) = you.filter(|_| t.hardcore() && bo) else {
            w.s.mutiny_at = None;
            return;
        };
        if w.s.mutiny_at.is_none_or(|at| w.s.t < at) {
            return;
        }
        let mut crew: Vec<usize> = (0..w.folk.len())
            .filter(|&i| {
                let f = &w.folk[i];
                idle_bot(f) && !(f.act.kind == "window" && f.act.you) && !(f.act.you && (f.act.kind == "chess" || f.act.kind == "pong"))
            })
            .collect();
        for k in (1..crew.len()).rev() {
            let j = (t.rand() * (k + 1) as f64).floor() as usize;
            crew.swap(k, j.min(k));
        }
        crew.truncate(6);
        if crew.len() < 2 {
            w.s.mutiny_at = (w.s.blood > 1.5).then_some(w.s.t + 5.0); // too few about: later, while it still rankles
            return;
        }
        let n = crew.len() as f64;
        let (cx, cy) = (crew.iter().map(|&i| w.folk[i].x).sum::<f64>() / n, crew.iter().map(|&i| w.folk[i].y).sum::<f64>() / n);
        let d = (cx - yx).hypot(cy - yy);
        let d = if d == 0.0 { 1.0 } else { d };
        let k = if d < 70.0 { 70.0 / d } else { 1.0 }; // away from you, out of earshot
        let misses = 3 + (t.rand() * 3.0).floor() as u32;
        let at = clear_at(l, yx + (cx - yx) * k, yy + (cy - yy) * k);
        w.s.mutiny = Some(Mutiny { phase: "gather", crew: crew.iter().map(|&i| w.folk[i].id.clone()).collect(), at, misses, ..Default::default() });
        for i in crew {
            end(&mut w.folk[i]);
            w.folk[i].act = Act::new("mutiny");
        }
        return;
    };
    if !t.hardcore() || !bo {
        w.s.mutiny = Some(m);
        return disband(w, t);
    }
    for id in &m.crew {
        // dropped out
        if let Some(f) = w.get_mut(id)
            && f.act.kind != "mutiny"
        {
            f.red = false;
            if f.carry == Some("gun") {
                f.carry = None;
            }
        }
    }
    m.crew.retain(|id| w.get(id).is_some_and(|f| f.act.kind == "mutiny"));
    if m.crew.is_empty() && m.phase != "dead" {
        // none of them left standing (or all back at work): over
        w.s.mutiny_at = None;
        if m.phase != "back" {
            w.s.blood = 0.0;
        }
        return;
    }
    m.t += dt;
    let crew: Vec<&Fig> = m.crew.iter().filter_map(|id| w.get(id)).collect();
    if m.phase == "gather" && (crew.iter().all(|f| f.act.there) || m.t > 14.0) {
        m.phase = "talk";
        m.t = 0.0;
    } else if m.phase == "talk" && m.t > 7.0 {
        m.phase = "stroll";
        m.t = 0.0;
    } else if m.phase == "stroll" && crew.iter().all(|f| f.carry == Some("gun") && f.act.there) {
        m.phase = "close";
        m.t = 0.0;
        let n = crew.len() as f64;
        let (cx, cy) = (crew.iter().map(|f| f.x).sum::<f64>() / n, crew.iter().map(|f| f.y).sum::<f64>() / n);
        m.from = you.map_or(0.0, |(yx, yy)| ((cy - yy) / 0.6).atan2(cx - yx)); // they come from where they stand, fanning out round you
    } else if m.phase == "back" && m.crew.is_empty() {
        return;
    }
    w.s.mutiny = Some(m);
}

/// hardcore off (or the gun closet gone): all of them back to what they were doing; you up if you lay there
fn disband(w: &mut World, t: &mut Tick) {
    let crew = w.s.mutiny.take().map(|m| m.crew).unwrap_or_default();
    for id in crew {
        if let Some(f) = w.get_mut(&id) {
            f.red = false;
            end(f);
        }
    }
    let r = t.rand();
    if let Some(f) = w.get_mut("you")
        && f.act.kind == "dead"
    {
        f.lying = false;
        f.legs = "stand";
        f.act = rest(r);
    }
}

/// one of them, this tick (true: it is in it, think_one leaves it alone)
pub fn step(w: &mut World, i: usize, l: &Layout, dt: f64, t: &mut Tick) -> bool {
    let Some(m) = w.s.mutiny.as_ref() else { return false };
    let id = w.folk[i].id.clone();
    if w.folk[i].act.kind != "mutiny" || !m.crew.contains(&id) {
        return false;
    }
    let (phase, mt, at, from, n) = (m.phase, m.t, m.at, m.from, m.crew.len());
    let k = m.crew.iter().position(|x| *x == id).unwrap();
    let you = w.get("you").map(|f| (f.x, f.y));
    let bo = l.booths.first().map(|b| (b.x as f64, b.y as f64));
    let wt = w.s.t;
    let look = |f: &mut Fig, g: Option<(f64, f64)>| {
        if let Some((gx, _)) = g {
            f.dir = if gx < f.x { -1.0 } else { 1.0 };
        }
    };
    {
        let f = &mut w.folk[i];
        f.sat = false;
        f.lying = false;
        f.on = None;
    }
    match phase {
        "gather" => {
            // a ring, close together, facing in
            let (a, r) = (2.0 * std::f64::consts::PI * k as f64 / n as f64, 5.0 + 2.0 * n as f64);
            let f = &mut w.folk[i];
            if walk(f, &Spot::at(at.0 + a.cos() * r, at.1 + a.sin() * r * 0.45), 26.0, dt) {
                f.act.there = true;
                f.arms = "down";
                look(f, Some(at));
            }
        }
        "talk" => {
            // in turn, a few words each; the others turn to whoever speaks; a glance your way now and then
            let who = w.s.mutiny.as_ref().unwrap().crew[(mt / 1.3).floor() as usize % n].clone();
            let wp = w.get(&who).map(|g| (g.x, g.y));
            let f = &mut w.folk[i];
            f.legs = "stand";
            if who == id {
                f.chat = f.chat.max(0.3);
                f.arms = if (wt * 3.0).floor() as i64 % 2 != 0 { "fwd1" } else { "down" };
                look(f, Some(at));
            } else {
                f.arms = "down";
                look(f, wp);
            }
            if you.is_some() && (mt / 1.7 + k as f64 * 1.3).floor() as i64 % 5 == 0 {
                look(f, you);
            }
        }
        "stroll" => {
            // one after another, hands by their sides, nothing up; a gun each, and the eyes go red
            let Some((bx, by)) = bo else { return true };
            let f = &mut w.folk[i];
            f.act.there = false;
            if f.carry == Some("gun") {
                // armed: a row in front of the closet, waiting for the others, looking at you
                let p = Spot::at((bx - 6.0 - (n as f64 - 1.0) * 6.0 + k as f64 * 12.0).clamp(l.floor.x0, l.floor.x1), (by + 22.0).min(l.floor.y1));
                if walk(f, &p, V, dt) {
                    f.act.there = true;
                    f.arms = "gun";
                    look(f, you);
                }
            } else if mt > k as f64 * 1.8 {
                if walk(f, &Spot::at(bx + 14.0, by), V, dt) {
                    f.dir = -1.0;
                    f.arms = "reach";
                    f.act.reach += dt;
                    if f.act.reach > 0.5 {
                        f.carry = Some("gun");
                        f.red = true;
                        let x = f.x;
                        t.sfx_vol("clank", x, 0.6);
                    }
                }
            } else {
                f.legs = "stand";
                f.arms = "down";
                if (wt * 0.6 + k as f64).floor() as i64 % 3 == 0 {
                    look(f, you);
                } else {
                    look(f, bo);
                }
            }
        }
        "close" => {
            // slowly, from their side, fanning out round you; now and then one stops, aims and fires
            let Some((yx, yy)) = you else { return true };
            let d = (w.folk[i].x - yx).hypot(w.folk[i].y - yy);
            let near = d < 110.0;
            let m = w.s.mutiny.as_mut().unwrap();
            if near && m.range_at.is_none() {
                m.range_at = Some(m.t); // the first of them within reach: from now on it tightens
            }
            let a = from + if n > 1 { (k as f64 / (n as f64 - 1.0) - 0.5) * (0.6 * n as f64).min(2.4) } else { 0.0 };
            let r = (84.0 - m.range_at.map_or(0.0, |ra| (m.t - ra) * 4.0)).max(28.0);
            let p = Spot::at((yx + a.cos() * r).clamp(l.floor.x0, l.floor.x1), (yy + a.sin() * r * 0.6).clamp(l.floor.y0, l.floor.y1));
            if let Some(aim) = w.folk[i].act.aim {
                // levelled, a moment: then the shot
                let aim = aim + dt;
                let cd = t.rnd(2.5, 4.5);
                let f = &mut w.folk[i];
                f.legs = "stand";
                f.arms = "gun";
                look(f, you);
                if aim > 0.6 {
                    f.act.aim = None;
                    f.act.cd = Some(cd);
                    fire(w, i, l, t);
                } else {
                    f.act.aim = Some(aim);
                }
                return true;
            }
            let f = &mut w.folk[i];
            if walk(f, &p, if near { SLOW } else { V }, dt) {
                f.arms = "gun";
                look(f, you);
            }
            if t.inp.fps_level || !near {
                return true; // (you are through the mysterious door: they wait for you at it)
            }
            let cd0 = t.rnd(0.5, 2.0) + k as f64 * 0.8;
            let f = &mut w.folk[i];
            let cd = f.act.cd.unwrap_or(cd0) - dt;
            f.act.cd = Some(cd);
            let m = w.s.mutiny.as_mut().unwrap();
            if cd <= 0.0 && m.t >= m.next {
                let next = t.rnd(1.6, 3.0);
                let m = w.s.mutiny.as_mut().unwrap();
                m.next = m.t + next; // one at a time, across them all
                w.folk[i].act.aim = Some(0.0);
            }
        }
        "dead" => {
            // they stand round you, guns still levelled
            let f = &mut w.folk[i];
            f.legs = "stand";
            f.arms = "gun";
            look(f, you);
        }
        "back" => {
            // the guns back in the closet, one after another, and off they go
            let Some((bx, by)) = bo else { return true };
            let r = t.rand();
            let f = &mut w.folk[i];
            if walk(f, &Spot::at(bx + 14.0, by), V, dt) {
                f.dir = -1.0;
                f.carry = None;
                f.red = false;
                f.act = rest(r);
            }
        }
        _ => {}
    }
    true
}

/// a shot at you: wide (a puff on the floor beside you) until enough have gone wide, then not. In the first person
/// the view takes it (Doom's damage, a puff past you), and says when you are dead (`died`)
fn fire(w: &mut World, i: usize, l: &Layout, t: &mut Tick) {
    let (fx, fy, fd) = (w.folk[i].x, w.folk[i].y, w.folk[i].dir);
    let m = w.s.mutiny.as_mut().unwrap();
    m.shots += 1;
    let hit = m.shots > m.misses && m.range_at.is_some_and(|ra| m.t - ra > 8.0); // (a few wide first, over eight seconds at least)
    let now = t.now();
    w.s.shots.push(Shot { x: crate::layout::jround(fx) as f64 + 10.0 * fd, y: (crate::layout::jround(fy) - 9) as f64, t0: now, ..Default::default() });
    t.sfx_vol("shot", fx, 0.8);
    if t.inp.fps {
        let (r, dmg) = (t.rand(), 12 + (t.rand() * 13.0).floor() as i32);
        w.s.incoming.push(Incoming { x: fx, y: fy, hit: hit && r < 0.8, dmg });
        return;
    }
    let Some((yx, yy)) = w.get("you").map(|f| (f.x, f.y)) else { return };
    if !hit {
        let (dx, side, dy) = (t.rnd(6.0, 16.0), t.rand(), t.rnd(-3.0, 5.0));
        let x = crate::layout::jround(yx + dx * if side < 0.5 { -1.0 } else { 1.0 }) as f64;
        let y = crate::layout::jround((yy + dy).min(l.floor.y1)) as f64;
        w.s.shots.push(Shot { x, y, t0: now, dust: true, ..Default::default() });
        return;
    }
    die(w, t);
}

/// you, shot: lying there, the room going red
pub fn die(w: &mut World, t: &mut Tick) {
    let now = t.now();
    if let Some(m) = w.s.mutiny.as_mut() {
        m.phase = "dead";
        m.t = 0.0;
        m.dead_at = now;
    }
    let x = w.get("you").map_or(t.w as f64 / 2.0, |f| f.x);
    if let Some(f) = w.get_mut("you") {
        end(f);
        f.act = Act::new("dead");
        f.lying = true;
        f.legs = "stand";
        f.arms = "down";
        f.hunt = None;
        f.chat = 0.0;
    }
    t.sfx("died", x);
}

/// in by the door again (not in the first person: there the view respawns you), the guns go back, nobody is afraid
/// any more
pub fn respawn(w: &mut World, l: &Layout, fps: bool, r: f64) {
    if let Some(f) = w.get_mut("you") {
        f.lying = false;
        if !fps {
            f.legs = "stand";
            f.act = rest(r);
            f.x = l.door_x("", 0);
            f.y = (WALL + 3) as f64;
            f.dir = 1.0;
        }
    }
    w.s.fear = 0.0;
    w.s.blood = 0.0;
    w.s.mutiny_at = None;
    w.s.incoming.clear();
    if let Some(m) = w.s.mutiny.as_mut() {
        m.phase = "back";
        m.t = 0.0;
    }
}
