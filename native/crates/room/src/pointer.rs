//! The pointer, in dots (room.js's pointer section): a click acts, a drag moves furniture (or carries a bot, the
//! dog), a figure let go on a piece uses it, one carried out over the pane opens there.

use crate::io::{Input, Output};
use crate::layout::{self, WALL, fixed, foot, kind_of, on_wall};
use crate::room::{Press, Room};
use crate::think::{self, BOOTH, Tick, in_fire, resume, spot_of_fig};
use crate::world::*;
use serde_json::{Value, json};

impl Room {
    /// what can be picked up under a point
    pub fn piece_at(&self, p: Option<(f64, f64)>) -> Option<layout::PieceR> {
        let (x, y) = p?;
        let l = self.last.as_ref()?;
        l.pieces.iter().find(|q| x >= q.r[0] as f64 && x < q.r[2] as f64 && y >= q.r[1] as f64 && y < q.r[3] as f64).cloned()
    }

    pub fn pointer_down(&mut self, x: f64, y: f64, shift: bool) {
        self.pointer = Some((x, y));
        let h = self.hots.iter().rev().find(|h| self.over(h.r[0], h.r[1], h.r[2], h.r[3])).map(|h| h.action.clone());
        let dog = self.dog_rect.is_some_and(|r| self.over(r[0], r[1], r[2], r[3])) && self.hovered.is_none();
        let bot = if dog { None } else { self.hovered.as_ref().filter(|hv| !hv.seat).and_then(|hv| self.w.get(&hv.id)).filter(|f| !f.staff).map(|f| f.id.clone()) };
        let piece = if dog { None } else { self.piece_at(self.pointer) };
        self.press = Some(Press { x, y, action: h, bot, piece, dog, shift });
    }

    /// the pointer moved (dots); `client`: where it is on the page, for a bot carried out over the pane
    pub fn pointer_move(&mut self, x: f64, y: f64, client: (f64, f64), out: &mut Output) {
        self.pointer = Some((x, y));
        let (w, h) = (self.size.w as f64, self.size.h as f64);
        if let Some(p) = self.press.clone()
            && self.moving.is_none()
            && self.held.is_none()
            && !self.w.s.dog_held
            && (x - p.x).hypot(y - p.y) > 1.0
        {
            let f = p.bot.as_ref().and_then(|id| self.w.idx(id));
            if p.dog && self.w.s.dog.is_some() {
                // the dog, picked up (by the scruff): what it carried drops where it was
                let (gx, gy) = self.w.s.dog.as_ref().map(|g| (g.x, g.y)).unwrap();
                if let Some(st) = self.w.s.stick.as_mut()
                    && st.held
                {
                    st.held = false;
                    st.x = gx;
                    st.y = gy;
                    st.t = 1.0;
                    st.h = 0.0;
                }
                self.w.s.dog_held = true;
                let g = self.w.s.dog.as_mut().unwrap();
                g.held = true;
                g.act = DogAct::new("held", 0.0);
            } else if let Some(i) = f.filter(|&i| self.w.folk[i].b.is_some() && !self.w.folk[i].lying) {
                // picked up; one about to be stopped is rescued by that
                let r = self.dice.next();
                let f = &mut self.w.folk[i];
                if f.act.kind == "doom" {
                    f.act = resume(f, r);
                }
                f.held = true;
                f.sat = false;
                f.lying = false;
                self.held = Some(f.id.clone());
            } else if let Some(pc) = p.piece.clone().filter(|_| !(p.bot.is_some() && f.is_none())) {
                self.moving = Some(layout::Moving { key: pc.key.clone(), from: pc.at, u: pc.u, at: pc.at });
                // the fire picked up: whoever warmed their hands at it steps away (they would stand in the pit as it went)
                if kind_of(&pc.key) == "fire" {
                    for g in self.w.folk.iter_mut() {
                        if g.act.kind == "warm" {
                            end(g);
                        }
                    }
                }
            }
        }
        if let (Some(m), Some(p)) = (self.moving.clone(), self.press.clone()) {
            let at = [m.from[0] + (x - p.x).round() as i32, m.from[1] + (y - p.y).round() as i32];
            let r = foot(&m.key, at, m.u);
            // on the grid of dots while it moves, as pair: it steps, and lands where it is shown, over whatever is
            // there. What hangs on the wall stays on it, sliding along; only one that can burn comes off it, and only
            // over the fire
            let loose = (on_wall(&m.key) && !fixed(&m.key)).then(|| {
                [(2 - (r[0] - at[0])).max((self.size.w - 2 - (r[2] - at[0])).min(at[0])), 0.max((self.size.h - 1 - (r[3] - at[1])).min(at[1]))]
            });
            let l = self.last.as_ref();
            let new_at = match (loose, l) {
                (Some(lo), Some(l)) if in_fire(l, &m.key, lo, m.u) => lo,
                _ => self.size.clamp_at(&m.key, at, m.u),
            };
            self.moving.as_mut().unwrap().at = new_at;
        }
        if self.w.s.dog_held
            && let Some(g) = self.w.s.dog.as_mut()
        {
            // it hangs from the pointer
            g.x = x.clamp(8.0, w - 8.0);
            g.y = (y + 7.0).clamp(WALL as f64, h - 2.0);
        }
        if let Some(id) = self.held.clone()
            && let Some(f) = self.w.get_mut(&id)
        {
            // it hangs under the pointer, but never past the floor's bottom edge
            f.x = x.clamp(0.0, w);
            f.y = (y + 24.0).clamp(WALL as f64, h - 2.0);
            f.dir = -1.0;
            let outside = x >= w || x < 0.0 || y < 0.0 || y >= h;
            out.carry = Some(outside.then(|| (id.clone(), client.0, client.1)));
        }
    }

    pub fn pointer_leave(&mut self) {
        if self.press.is_none() {
            self.pointer = None;
        }
    }

    pub fn pointer_up(&mut self, inp: &Input, out: &mut Output, client: (f64, f64)) {
        // (the dice and the fresh ones lent to the tick, so the room stays free to look at itself)
        let mut dice = std::mem::replace(&mut self.dice, Dice::new(1));
        let mut fresh = std::mem::take(&mut self.fresh);
        let mut t = Tick { inp, out, dice: &mut dice, fresh: &mut fresh, moving_fire: false, w: self.size.w };
        self.up(inp, &mut t, client);
        self.dice = dice;
        self.fresh = fresh;
    }

    fn up(&mut self, inp: &Input, t: &mut Tick, client: (f64, f64)) {
        let ptr = self.pointer;
        let press = self.press.take();
        let (w, h) = (self.size.w as f64, self.size.h as f64);
        let click = matches!((ptr, &press), (Some((x, y)), Some(p)) if (x - p.x).hypot(y - p.y) <= 1.0);
        let l = self.last.clone();
        // on your way to the gun closet: any other click stops you (where it sends you, you go)
        if let Some(you) = self.w.get_mut("you")
            && you.act.kind == "arm"
            && click
            && self.moving.is_none()
            && self.held.is_none()
            && !press.as_ref().and_then(|p| p.action.as_ref()).is_some_and(|a| a["kind"] == "aim")
        {
            end(you);
        }
        let action = press.as_ref().and_then(|p| p.action.clone());
        let akind = action.as_ref().and_then(|a| a["kind"].as_str().map(|s| s.to_string())).unwrap_or_default();
        if let Some(m) = self.moving.take() {
            let Some(l) = l else { return };
            let d = l.desks.iter().find(|d| d.key == m.key).cloned();
            if in_fire(&l, &m.key, m.at, m.u) {
                // into the fire: a picture of it goes in, the thing itself goes (a desk's bots walk out)
                let r = foot(&m.key, m.at, m.u);
                let img = self.snapshot_of(&m.key, m.at, m.u, inp);
                self.to_fire(r, img, t);
                let mid = m.key.strip_prefix("door:").map(|s| s.to_string());
                if let Some(d) = d {
                    let ids: Vec<String> = inp.state.bots.iter().filter(|b| b.unit == d.d.unit).map(|b| b.id.clone()).collect();
                    // its bots first (condemn), then the desk
                    if think::condemn(&mut self.w, t, &ids, Some(&d.d.unit)).is_empty() {
                        t.out.act(json!({"kind": "forget", "unit": d.d.unit}));
                    }
                } else if let Some(mid) = mid {
                    t.out.act(json!({"kind": "unmachine", "m": mid}));
                } else {
                    t.out.act(json!({"kind": "burn", "key": m.key}));
                }
            } else {
                // put down on the grid of dots, where it was let go
                let at = self.size.clamp_at(&m.key, m.at, m.u);
                self.keep.dropped.insert(m.key.clone(), (at, inp.now));
                if kind_of(&m.key) == "fire" {
                    // put down where someone stands: they walk out of the flames
                    let r = foot(&m.key, at, m.u);
                    for g in self.w.folk.iter_mut() {
                        if !g.held && g.x >= r[0] as f64 && g.x < r[2] as f64 && g.y >= (r[1] + 10) as f64 && g.y < (r[3] + 4) as f64 {
                            end(g);
                            let left = t.dice.rnd(3.0, 6.0);
                            g.act = Act { kind: "loiter", dest: Some(Spot::at(g.x, l.floor.y1.min((r[3] + 8) as f64))), left: Some(left), v: 30.0, ..Default::default() };
                        }
                    }
                }
                let mut lay = serde_json::Map::new();
                for (k, v) in &inp.state.layout {
                    lay.insert(k.clone(), json!(v));
                }
                lay.insert(m.key.clone(), json!(at));
                t.out.act(json!({"kind": "layout", "layout": lay}));
            }
        } else if let Some(id) = self.held.take() {
            let Some(i) = self.w.idx(&id) else { return };
            self.w.folk[i].held = false;
            // carried out of the room, onto the pane, its session opens there; anywhere else it goes back to what it did
            let out_of = ptr.is_some_and(|(x, y)| x >= w || x < 0.0 || y < 0.0 || y >= h);
            let bot = self.w.folk[i].is_bot();
            if let (false, Some(l)) = (out_of, l.as_ref()) {
                let f = &mut self.w.folk[i];
                f.x = f.x.clamp(l.floor.x0, l.floor.x1);
                f.y = f.y.clamp(l.floor.y0, l.floor.y1);
            }
            let r = t.dice.next();
            if out_of {
                let f = &mut self.w.folk[i];
                f.act = resume(f, r);
                if bot {
                    t.out.carried = Some((id.clone(), client.0, client.1));
                }
            } else if let (true, Some(l), true) = (bot, l.as_ref(), ptr.is_some()) {
                let bo = think::booth_at(l, &self.w.folk[i]).cloned();
                if let Some(bo) = bo {
                    if t.hardcore() {
                        // by the gun closet, hands up: someone takes a gun
                        let f = &mut self.w.folk[i];
                        f.x = l.floor.x0.max((bo.x - 16) as f64);
                        f.y = l.floor.y1.min((bo.y + 3) as f64);
                        think::condemn(&mut self.w, t, &[id.clone()], None);
                    } else {
                        // into the booth: five seconds, then gone
                        let f = &mut self.w.folk[i];
                        end(f);
                        f.act = Act { kind: "doom", at: Some(bo.key.clone()), left: Some(BOOTH), ..Default::default() };
                        t.out.act(json!({"kind": "close", "id": id}));
                    }
                } else {
                    self.drop_on_piece(i, &id, bot, l, t, r);
                }
            } else if let Some(l) = l.as_ref() {
                self.drop_on_piece(i, &id, bot, l, t, r);
            }
            if let Some(f) = self.w.folk.get_mut(i) {
                f.legs = "stand";
            }
        } else if self.w.s.dog_held {
            // the dog let go of: on the floor (never in the fire: in front of it), a shake, off it goes; at its house, in
            self.w.s.dog_held = false;
            if let (Some(g), Some(l)) = (self.w.s.dog.as_mut(), l.as_ref()) {
                g.held = false;
                g.x = g.x.clamp(l.floor.x0, l.floor.x1);
                g.y = g.y.clamp(l.floor.y0, l.floor.y1);
                if let Some(fr) = &l.fire
                    && g.x >= (fr.x0 - 4) as f64
                    && g.x < (fr.x1 + 4) as f64
                    && g.y >= fr.y0 as f64
                    && g.y < (fr.y1 + 6) as f64
                {
                    g.y = l.floor.y1.min((fr.y1 + 8) as f64);
                }
                let home = l.doghouses.first().is_some_and(|dh| think::in_rect((g.x, g.y), foot("doghouse", [dh.x, dh.y + 4], None)));
                let next = home.then(|| Box::new(DogAct::new("home", t.dice.rnd(20.0, 60.0))));
                g.act = DogAct { kind: "shake", left: 0.6, next, ..Default::default() };
            }
        } else if akind == "undo" {
            // the booth runs back, it steps out
            let id = action.as_ref().unwrap()["id"].as_str().unwrap_or_default().to_string();
            if let Some(f) = self.w.get_mut(&id)
                && f.act.kind == "doom"
            {
                f.act.back = true;
            }
        } else if akind == "unshoot" {
            let id = action.as_ref().unwrap()["id"].as_str().unwrap_or_default().to_string();
            think::unshoot(&mut self.w, &id, inp.now);
        } else if akind == "aim" {
            // you walk to the gun closet and take a gun; the first person after
            let key = action.as_ref().unwrap()["key"].as_str().map(|s| s.to_string());
            let sh = inp.shell.as_ref().map(|s| format!("{}:{}", s.m, s.dir));
            if let Some(you) = self.w.get_mut("you")
                && !you.held
                && !inp.fps
            {
                you.hunt = None;
                end(you);
                you.act = Act { kind: "arm", at: key, open: inp.open.clone(), shell: sh, chosen: true, ..Default::default() };
            }
        } else if akind == "water" {
            // you go and water the plant
            let key = action.as_ref().unwrap()["key"].as_str().map(|s| s.to_string());
            if let Some(you) = self.w.get_mut("you")
                && !you.held
            {
                end(you);
                you.act = Act { kind: "plant", at: key, chosen: true, ..Default::default() };
            }
        } else if akind == "pet" {
            // the dog: you go and pet it (out of its house it comes, unless afraid); petting it, you throw the stick
            if let (Some(l), true) = (l.as_ref(), self.w.get("you").is_some_and(|y| !y.held)) {
                let dog_ok = self.w.s.dog.as_ref().is_some_and(|g| !g.held && !g.leaving);
                if dog_ok {
                    if crate::dog::can_throw(&self.w) {
                        crate::dog::throw_stick(&mut self.w, l, t);
                    } else if self.w.s.dog.as_ref().unwrap().act.kind != "fetch" {
                        let you = self.w.get_mut("you").unwrap();
                        end(you);
                        you.act = Act { kind: "pet", left: Some(6.0), chosen: true, ..Default::default() };
                        let g = self.w.s.dog.as_mut().unwrap();
                        if !g.act.hide {
                            g.act = DogAct { kind: "petted", out: g.inside, ..Default::default() };
                        }
                    }
                }
            }
        } else if akind == "sit" {
            // you take the armchair; a bot sitting in it gets up
            let key = action.as_ref().unwrap()["key"].as_str().map(|s| s.to_string());
            for g in self.w.folk.iter_mut() {
                if g.id != "you" && g.act.kind == "chair" && g.act.at == key {
                    end(g);
                }
            }
            let left = t.dice.rnd(180.0, 360.0);
            if let Some(you) = self.w.get_mut("you")
                && !you.held
            {
                end(you);
                you.act = Act { kind: "chair", at: key, left: Some(left), chosen: true, ..Default::default() };
            }
        } else if let Some(mut a) = action {
            if akind == "report" {
                self.w.s.print = inp.now;
                if let Some(pr) = l.as_ref().and_then(|l| l.printer.clone()) {
                    t.sfx("print", pr.x as f64);
                }
            }
            if let Some(o) = a.as_object_mut() {
                o.insert("shift".into(), Value::Bool(press.as_ref().is_some_and(|p| p.shift)));
            }
            t.out.act(a);
        } else if let (Some(p), Some((x, y)), true) = (press.as_ref(), ptr, click) {
            // a plain click on the floor or the wall, on nothing: what is open on the right goes away
            if p.bot.is_none() && p.piece.is_none() {
                t.out.act(json!({"kind": "blank"}));
                // and you walk there, if it is floor, and stand about a while
                if let Some(l) = l.as_ref()
                    && y >= l.floor.y0
                {
                    let left = t.dice.rnd(20.0, 45.0);
                    if let Some(you) = self.w.get_mut("you")
                        && !you.held
                    {
                        end(you);
                        you.act = Act { kind: "loiter", dest: Some(Spot::at(x.clamp(l.floor.x0, l.floor.x1), y.min(l.floor.y1))), left: Some(left), v: 36.0, ..Default::default() };
                    }
                }
            }
        }
    }

    /// a figure let go on a piece: a pair table (a bot joins its chat), else whatever the piece is used for
    fn drop_on_piece(&mut self, i: usize, id: &str, bot: bool, l: &layout::Layout, t: &mut Tick, r: f64) {
        let p = self.piece_at(Some(spot_of_fig(&self.w.folk[i]))); // what it stands on, not what the pointer is on
        let f = &self.w.folk[i];
        if let Some(p) = p.as_ref().filter(|p| kind_of(&p.key) == "pair") {
            if bot {
                t.out.act(json!({"kind": "pair", "key": p.key, "id": id}));
            } else {
                t.out.act(json!({"kind": "pairOpen", "key": p.key}));
            }
            let f = &mut self.w.folk[i];
            f.act = resume(f, r);
            return;
        }
        let used = if !f.work { p.as_ref().and_then(|p| think::use_at(f, &p.key, l, t)) } else { None };
        let f = &mut self.w.folk[i];
        f.act = used.unwrap_or_else(|| resume(f, r));
    }

    /// something goes into the fire: a picture of it, to be tossed in and sink (the painters' drawFire)
    pub fn to_fire(&mut self, r: layout::Rect, img: pix::Image, t: &mut Tick) {
        t.sfx("whoosh", r[0] as f64);
        self.w.s.burning.push(Burning { img: std::rc::Rc::new(img), x: (r[0] - 2) as f64, y: (r[1] - 2) as f64, t0: t.now() });
        self.w.s.flare = t.now();
    }

    /// a picture of a piece (or desk, or door) as it stands, for the fire
    pub fn snapshot_of(&self, key: &str, at: [i32; 2], u: Option<i32>, inp: &Input) -> pix::Image {
        let r = foot(key, at, u);
        crate::paint::snapshot_piece(self, key, at, u, r, inp)
    }
}
