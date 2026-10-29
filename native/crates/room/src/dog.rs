//! The dog (the dog house, from the closet): a dachshund, a badger dog, as burrows go. It lives on the floor
//! while its house is in the room: naps in it, its head out of the door; trots about and sniffs; visits a bot or
//! you and sits by them wagging (a bot that is idle pets it); follows you a while; chases the ball while bots play
//! football; barks at whoever comes in through a door; and a click on it sends you over to pet it (hearts). At
//! night (23 to 7 here), while you are away, and in hardcore while the bots are afraid (only its eyes in the dark
//! door), it stays in its house. Its house burnt, it trots out through the door.

use crate::layout::{Layout, WALL, free_at, jround};
use crate::think::{Tick, doomed};
use crate::world::*;

/// you can throw the stick: petting it, or it sits at your feet with the stick
pub fn can_throw(w: &World) -> bool {
    let (Some(you), Some(g)) = (w.get("you"), w.s.dog.as_ref()) else { return false };
    !g.inside
        && !g.held
        && !g.leaving
        && ((you.act.kind == "pet" && you.act.there) || (you.act.kind == "fetch" && g.act.kind == "fetch" && g.act.stage == "wait"))
}

/// out into the room, somewhere clear 40 to 80 dots off; the dog runs for it
pub fn throw_stick(w: &mut World, l: &Layout, t: &mut Tick) {
    let fl = l.floor;
    let mid = (fl.x0 + fl.x1) / 2.0;
    let Some(you) = w.get("you").map(|y| (y.x, y.y)) else { return };
    let mut to = (jround(mid) as f64, jround((fl.y0 + fl.y1) / 2.0) as f64);
    for _ in 0..40 {
        let a = (if you.0 > mid { std::f64::consts::PI } else { 0.0 }) + t.rnd(-0.9, 0.9);
        let d = t.rnd(40.0, 80.0);
        let x = jround(you.0 + a.cos() * d) as f64;
        let y = jround(you.1 + a.sin() * d * 0.7) as f64;
        if x > fl.x0 + 6.0 && x < fl.x1 - 6.0 && y > fl.y0 + 8.0 && y < fl.y1 - 2.0 && l.free(x, y) {
            to = (x, y);
            break;
        }
    }
    let wt = w.s.t;
    let f = w.get_mut("you").unwrap();
    end(f);
    f.dir = if to.0 < f.x { -1.0 } else { 1.0 };
    f.act = Act { kind: "fetch", t: wt, chosen: true, ..Default::default() };
    let (sx, sy) = (f.x + f.dir * 6.0, f.y);
    w.s.stick = Some(Stick { x: sx, y: sy, x0: sx, y0: sy, x1: to.0, y1: to.1, t: 0.0, h: 12.0, held: false, turn: 0.0 });
    if let Some(g) = w.s.dog.as_mut() {
        g.act = DogAct { kind: "fetch", stage: "run", ..Default::default() };
    }
}

fn walk_dog(g: &mut Dog, to: (f64, f64), v: f64, dt: f64) -> bool {
    let (dx, dy) = (to.0 - g.x, to.1 - g.y);
    let d = dx.hypot(dy);
    if d <= v * dt {
        g.x = to.0;
        g.y = to.1;
        return true;
    }
    g.x += dx / d * v * dt;
    g.y += dy / d * v * dt;
    if dx.abs() > 0.5 {
        g.dir = dx.signum();
    }
    g.step += dt * 10.0;
    g.pose = if g.step.floor() as i64 % 2 != 0 { "walk1" } else { "walk2" };
    false
}

fn next_dog(w: &World, l: &Layout, t: &mut Tick) -> DogAct {
    let r = t.rand();
    let folk: Vec<&Fig> = w.folk.iter().filter(|f| f.b.is_some() && !f.held && !f.leaving && !f.lying && !f.staff && !doomed(&f.act)).collect();
    let bots: Vec<String> = folk.iter().filter(|f| !f.human).map(|f| f.id.clone()).collect();
    let you = folk.iter().any(|f| f.human);
    if r < 0.3 {
        let p = free_at(l, &format!("dog{}", w.s.t.floor() as i64));
        let sniff = t.rnd(1.5, 4.0);
        return DogAct { kind: "wander", to: Some(p), sniff, ..Default::default() };
    }
    if r < 0.5 && !bots.is_empty() {
        let who = t.dice.any(&bots).cloned();
        let left = t.rnd(6.0, 14.0);
        return DogAct { kind: "visit", who, left, ..Default::default() };
    }
    if r < 0.65 && you {
        let left = t.rnd(6.0, 12.0);
        return DogAct { kind: "visit", who: Some("you".into()), left, ..Default::default() };
    }
    if r < 0.8 && you {
        let left = t.rnd(15.0, 30.0);
        return DogAct::new("follow", left);
    }
    let left = t.rnd(20.0, 60.0);
    DogAct::new("home", left)
}

pub fn think_dog(w: &mut World, l: &Layout, dt: f64, t: &mut Tick) {
    let Some(h) = l.doghouses.first().cloned() else {
        // its house is gone: out through the door
        let door = l.door_x("", 0);
        if let Some(g) = w.s.dog.as_mut() {
            if g.held {
                g.held = false;
                w.s.dog_held = false;
            }
            g.inside = false;
            g.leaving = true;
            g.barking = false;
            if walk_dog(g, (door, (WALL + 2) as f64), 40.0, dt) {
                w.s.dog = None;
            }
        }
        return;
    };
    let door = (h.x as f64, (h.y + 3) as f64);
    if w.s.dog.as_ref().is_none_or(|g| g.leaving) {
        let left = t.rnd(3.0, 10.0);
        w.s.dog = Some(Dog { x: door.0, y: door.1, dir: 1.0, pose: "stand", inside: true, act: DogAct::new("home", left), ..Default::default() });
    }
    let now = t.now();
    let wt = w.s.t;
    let sleepy = t.inp.hour >= 23 || t.inp.hour < 7 || t.inp.away > 60.0;
    let afraid = w.s.fear > 0.4;
    {
        let g = w.s.dog.as_mut().unwrap();
        g.z += dt;
        g.barking = false;
        g.hearts.retain(|x| now - x.2 < 1200.0);
    }
    // the stick in the air: from your hand, in an arc, down on the floor
    if let Some(st) = w.s.stick.as_mut()
        && !st.held
        && st.t < 1.0
    {
        st.t = (st.t + dt / 0.8).min(1.0);
        st.x = st.x0 + (st.x1 - st.x0) * st.t;
        st.y = st.y0 + (st.y1 - st.y0) * st.t;
        st.h = 12.0 * (1.0 - st.t) + (std::f64::consts::PI * st.t).sin() * 22.0;
    }
    if w.s.dog.as_ref().unwrap().held {
        // in your hand: dangling, wagging
        let g = w.s.dog.as_mut().unwrap();
        g.pose = "stand";
        g.wag = (wt * 8.0).floor() as i32 % 2;
        g.inside = false;
        return;
    }
    // with you (petted, a game of fetch) it stays up at night as well; afraid, it stays in its house whatever you do
    {
        let r1 = t.rnd(1.0, 4.0);
        let g = w.s.dog.as_mut().unwrap();
        let with_you = !afraid && ["petted", "fetch", "shake"].contains(&g.act.kind);
        if (sleepy || afraid) && !with_you && !(g.act.kind == "home" && g.act.left > 1e8) {
            g.act = DogAct { kind: "home", left: 1e9, hide: afraid, ..Default::default() };
        } else if !sleepy && !afraid && g.act.kind == "home" && g.act.left > 1e8 {
            g.act = DogAct::new("home", r1);
        }
        if !sleepy && !afraid && !with_you {
            if w.s.ball.is_some() && g.act.kind != "chase" {
                g.act = DogAct::new("chase", 0.0);
            } else if let Some(n) = w.s.dog_news.clone()
                && g.act.kind != "bark"
                && w.folk.iter().any(|f| f.id == n)
            {
                g.act = DogAct { kind: "bark", who: Some(n), left: 3.0, ..Default::default() };
            }
        }
    }
    w.s.dog_news = None;
    let mut love: Vec<(f64, f64)> = Vec::new();
    let mut next = false;
    let kind = w.s.dog.as_ref().unwrap().act.kind;
    {
        let g = w.s.dog.as_mut().unwrap();
        if kind != "home" {
            g.inside = false;
            g.asleep = false;
        }
        g.wag = (wt * if ["visit", "petted", "follow", "chase", "fetch"].contains(&kind) { 8.0 } else { 1.5 }).floor() as i32 % 2;
    }
    match kind {
        "home" => {
            // in its house: in through the door, a nap there; you reaching in to it, afraid: hearts
            let you_petting = w.get("you").is_some_and(|p| p.act.kind == "pet" && p.act.there);
            let g = w.s.dog.as_mut().unwrap();
            if !g.inside && !walk_dog(g, door, 34.0, dt) {
            } else {
                g.inside = true;
                g.x = door.0;
                g.y = door.1;
                g.pose = "lie";
                g.dir = 1.0;
                g.asleep = g.act.left > 1e8 || sleepy;
                if g.act.hide && you_petting {
                    love.push((h.x as f64, (h.y - 12) as f64));
                }
                g.act.left -= dt;
                if g.act.left <= 0.0 {
                    g.inside = false;
                    g.asleep = false;
                    next = true;
                }
            }
        }
        "shake" => {
            // put down: a shake, then about its business (into its house, if it was put down there)
            let g = w.s.dog.as_mut().unwrap();
            g.pose = "stand";
            g.act.left -= dt;
            if g.act.left <= 0.0 {
                match g.act.next.take() {
                    Some(n) => g.act = *n,
                    None => next = true,
                }
            }
        }
        "fetch" => {
            // after the stick you threw, back with it to your feet, then sitting there for the next throw
            let you = w.get("you").filter(|f| !f.held && f.act.kind == "fetch").map(|f| (f.x, f.y));
            let (Some(f), true) = (you, w.s.stick.is_some()) else {
                let g = w.s.dog.as_ref().unwrap();
                let (gx, gy) = (g.x, g.y);
                if let Some(st) = w.s.stick.as_mut()
                    && st.held
                {
                    st.held = false;
                    st.x = gx;
                    st.y = gy;
                    st.t = 1.0;
                    st.h = 0.0;
                }
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let st = w.s.stick.as_mut().unwrap();
            let g = w.s.dog.as_mut().unwrap();
            match g.act.stage {
                "run" => {
                    // off as soon as it flies, to where it comes down; there first, it watches it come
                    let to = (st.x1 + if g.x < st.x1 { -7.0 } else { 7.0 }, st.y1 + 1.0);
                    let there = walk_dog(g, to, 62.0, dt);
                    if there && st.t >= 1.0 {
                        g.act.stage = "pick";
                        g.act.wait = 0.0;
                    } else if there {
                        g.pose = "stand";
                        g.dir = if st.x < g.x { -1.0 } else { 1.0 };
                    }
                }
                "pick" => {
                    g.pose = "stand";
                    g.dir = if st.x < g.x { -1.0 } else { 1.0 };
                    g.act.wait += dt;
                    if g.act.wait > 0.3 {
                        st.held = true;
                        g.act.stage = "back";
                    }
                }
                "back" => {
                    // to your feet, the stick across its mouth; dropped there
                    let side = if g.x < f.0 { -1.0 } else { 1.0 };
                    if walk_dog(g, (f.0 + side * 13.0, f.1 + 1.0), 48.0, dt) {
                        st.held = false;
                        st.x = g.x - side * 6.0;
                        st.y = g.y + 1.0;
                        st.t = 1.0;
                        st.h = 0.0;
                        g.act.stage = "wait";
                        g.act.left = 12.0;
                    }
                }
                _ => {
                    // at your feet: sitting, wagging, now and then a heart; another throw, or it wanders off
                    g.pose = "sit";
                    g.dir = if f.0 < g.x { -1.0 } else { 1.0 };
                    if t.dice.next() < dt * 0.7 {
                        love.push((g.x, g.y - 10.0));
                    }
                    g.act.left -= dt;
                    if g.act.left <= 0.0 {
                        next = true;
                    }
                }
            }
        }
        "wander" => {
            // somewhere on the floor, a sniff there
            let g = w.s.dog.as_mut().unwrap();
            let to = g.act.to.clone().map(|s| (s.x, s.y)).unwrap_or((g.x, g.y));
            if walk_dog(g, to, 26.0, dt) {
                g.pose = "stand";
                g.act.sniff -= dt;
                if g.act.sniff <= 0.0 {
                    next = true;
                }
            }
        }
        "visit" => {
            // beside a bot or you, sitting, wagging; an idle bot pets it
            let who = w.s.dog.as_ref().unwrap().act.who.clone().unwrap_or_default();
            let Some(fi) = w.idx(&who).filter(|&i| !w.folk[i].leaving && !w.folk[i].held) else {
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let (fx, fy) = (w.folk[fi].x, w.folk[fi].y);
            let side = if fx > (l.floor.x0 + l.floor.x1) / 2.0 { -1.0 } else { 1.0 };
            let spot = (fx + side * 13.0, fy + 1.0);
            let g = w.s.dog.as_mut().unwrap();
            if (spot.0 - g.x).hypot(spot.1 - g.y) > 3.0 {
                walk_dog(g, spot, 32.0, dt);
            } else {
                g.pose = "sit";
                g.dir = -side;
                let (gx, gy) = (g.x, g.y);
                g.act.left -= dt;
                if g.act.left <= 0.0 {
                    next = true;
                }
                let f = &mut w.folk[fi];
                if !f.human && !f.work && f.legs == "stand" && !f.sat {
                    f.dir = side;
                    f.arms = if (wt * 3.0).floor() as i64 % 2 != 0 { "reach" } else { "down" };
                    love.push((gx, gy - 10.0));
                }
                if f.human && t.dice.next() < dt * 0.6 {
                    love.push((gx, gy - 10.0));
                }
            }
        }
        "follow" => {
            // at your heels a while
            let Some(f) = w.get("you").filter(|f| !f.held).map(|f| (f.x, f.y, f.dir)) else {
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let g = w.s.dog.as_mut().unwrap();
            let fd = if f.2 == 0.0 { 1.0 } else { f.2 };
            let spot = ((f.0 - fd * 14.0).clamp(l.floor.x0, l.floor.x1), f.1 + 2.0);
            if (spot.0 - g.x).hypot(spot.1 - g.y) > 6.0 {
                walk_dog(g, spot, 38.0, dt);
            } else {
                g.pose = "sit";
                g.dir = if f.0 < g.x { -1.0 } else { 1.0 };
            }
            g.act.left -= dt;
            if g.act.left <= 0.0 {
                next = true;
            }
        }
        "chase" => {
            // after the ball the bots kick about
            let Some(b) = w.s.ball.as_ref().map(|b| (b.x, b.y)) else {
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let g = w.s.dog.as_mut().unwrap();
            if (b.0 - g.x).hypot(b.1 - g.y) > 5.0 {
                walk_dog(g, (b.0 + if g.x < b.0 { -5.0 } else { 5.0 }, b.1 + 1.0), 55.0, dt);
            } else {
                g.pose = "stand";
                g.dir = if b.0 < g.x { -1.0 } else { 1.0 };
                g.barking = (wt * 1.5).floor() as i64 % 3 == 0;
            }
        }
        "bark" => {
            // at a newcomer, from a little way off
            let who = w.s.dog.as_ref().unwrap().act.who.clone().unwrap_or_default();
            let Some(f) = w.get(&who).map(|f| (f.x, f.y)) else {
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let g = w.s.dog.as_mut().unwrap();
            if (f.0 - g.x).hypot(f.1 - g.y) > 24.0 {
                walk_dog(g, (f.0 + if g.x < f.0 { -20.0 } else { 20.0 }, f.1 + 1.0), 50.0, dt);
            } else {
                g.pose = "stand";
                g.dir = if f.0 < g.x { -1.0 } else { 1.0 };
                g.barking = true;
                let x = g.x;
                if !g.act.yap {
                    g.act.yap = true; // once: a newcomer is the event
                    t.sfx("bark", x);
                }
                let g = w.s.dog.as_mut().unwrap();
                g.act.left -= dt;
                if g.act.left <= 0.0 {
                    next = true;
                }
            }
        }
        "petted" => {
            // sitting for you while you pet it
            let Some(f) = w.get("you").filter(|f| f.act.kind == "pet").map(|f| (f.x, f.y)) else {
                let a = next_dog(w, l, t);
                w.s.dog.as_mut().unwrap().act = a;
                return;
            };
            let g = w.s.dog.as_mut().unwrap();
            if g.act.out && !walk_dog(g, (door.0, l.floor.y1.min(door.1 + 10.0)), 30.0, dt) {
                // called out of its house: a few steps out first
            } else {
                g.act.out = false;
                g.pose = "sit";
                g.dir = if f.0 < g.x { -1.0 } else { 1.0 };
                if (f.0 - g.x).hypot(f.1 - g.y) < 16.0 {
                    love.push((g.x, g.y - 10.0));
                }
            }
        }
        _ => next = true,
    }
    for (x, y) in love {
        let dx = t.rnd(-3.0, 3.0);
        let g = w.s.dog.as_mut().unwrap();
        if g.love_at.is_none_or(|a| wt - a > 0.7) {
            g.love_at = Some(wt);
            g.hearts.push((x + dx, y, now));
        }
    }
    if next {
        let a = next_dog(w, l, t);
        w.s.dog.as_mut().unwrap().act = a;
    }
}
