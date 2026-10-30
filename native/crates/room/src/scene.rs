//! Drawing the room (room.js's `draw`): the wall and the floor, then everything that stands back to front,
//! the figures among the furniture, then what goes over everything (beams, shots, bubbles, the tip). Hot
//! spots are collected while drawing: what is under the pointer and what a click there does.

use crate::avatar::c;
use crate::figure::{Arms, Dress, Legs, SH, SW, halo};
use crate::font::{tiny, tiny_w};
use crate::io::{Input, Output};
use crate::layout::{Layout, WALL, jround};
use crate::room::{Hot, Hovered, Room, Tip};
use crate::think::{BEAM, booth_rise, booth_t};
use crate::world::{Fig, fnv};
use pix::{Color, Ctx, Image, Op};

pub const WALLC: Color = Color::rgb(0x28374b);
pub const BASEBOARD: Color = Color::rgb(0x1b2433);
pub const PLANK: Color = Color::rgb(0x734728);
pub const SEAM: Color = Color::rgb(0x6a4224);
/// what the pointer is on: the tiles' pastel gold, on the floor as well
pub const GOLD: Color = Color::rgb(0xf6c784);
pub const RED: Color = Color::rgb(0xff6b5a);
/// a figure whose session is open
pub const OPEN: Color = Color::rgb(0x8a4a3a);

/// the wall and the wooden floor, drawn once for the room's size: Pixel Agents' planks, three dots wide, their
/// joints staggered, a shadow where the floor meets the wall
pub fn floor(w: i32, h: i32) -> Image {
    let mut im = Image::new(w as u32, h as u32);
    let mut x = im.ctx();
    x.rect(0, WALL, w, h - WALL, PLANK);
    let mut y = WALL + 3;
    let mut row = 0;
    while y < h {
        x.rect(0, y, w, 1, SEAM);
        let mut px = -((fnv(&format!("r{row}")) % 48) as i32);
        while px < w {
            x.rect(px, y + 1, 1, 3, SEAM);
            px += 40 + (fnv(&format!("{row}:{px}")) % 4) as i32 * 8;
        }
        y += 4;
        row += 1;
    }
    x.rect(0, 0, w, WALL, WALLC);
    x.rect(0, WALL - 2, w, 2, BASEBOARD);
    x.rect(0, WALL, w, 1, Color::rgba(0, 0, 0, 77));
    x.rect(0, WALL + 1, w, 1, Color::rgba(0, 0, 0, 31));
    im
}

/// a figure's colour: a bot wears its project's, you wear yours
pub fn color_of(f: &Fig) -> Color {
    f.color.unwrap_or(Color::rgb(0x8b93ad))
}

impl Room {
    /// is the pointer inside, in dots
    pub fn over(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
        matches!(self.pointer, Some((x, y)) if x >= x0 && x < x1 && y >= y0 && y < y1)
    }
    /// a hot spot: collected, and whether the pointer is on it (and free to click)
    pub fn hot(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, action: serde_json::Value) -> bool {
        self.hots.push(Hot { r: [x0, y0, x1, y1], action });
        self.over(x0, y0, x1, y1) && self.moving.is_none() && self.held.is_none() && !self.w.s.dog_held
    }

    /// the room drawn into `self.img` (W*2 x H*2: two pixels a dot)
    pub fn draw(&mut self, inp: &Input, out: &mut Output, l: &Layout) {
        let (w, h) = (self.size.w, self.size.h);
        if self.img.w != (w * 2) as u32 || self.img.h != (h * 2) as u32 {
            self.img = Image::new((w * 2) as u32, (h * 2) as u32);
        }
        if self.floor.as_ref().is_none_or(|f| f.w != w as u32 || f.h != h as u32) {
            self.floor = Some(floor(w, h));
        }
        self.hots.clear();
        self.tip = None;
        self.screen_hover = None;
        let mut img = std::mem::replace(&mut self.img, Image::new(1, 1));
        {
            let mut ctx = img.ctx();
            ctx.set_op(Op::Copy);
            ctx.set_transform(2., 0., 0., 2., 0., 0.); // drawn in dots; the canvas has two pixels a dot, for dragging
            ctx.draw_image(self.floor.as_ref().unwrap(), 0., 0.);
            ctx.set_op(Op::SourceOver);
            ctx.rect(w - 1, 0, 1, h, c::INK2);
            self.draw_room(&mut ctx, inp, out, l);
            self.draw_over(&mut ctx, inp, out, l);
        }
        self.img = img;
    }

    fn draw_over(&mut self, ctx: &mut Ctx, inp: &Input, out: &mut Output, l: &Layout) {
        // a message on its way: a beam in the sender's colour, up from its antenna, across the room over everyone's
        // heads, down into the receiver's antenna; its bright head leads, its tail follows. Its ends: a figure's
        // antenna, or the telephone on a pair table (over its handset)
        let beams = self.w.s.beams.clone();
        for bm in beams {
            let end = |id: &str, key: &Option<String>| -> Option<(f64, f64)> {
                if let Some(k) = key {
                    return l.pairs.iter().find(|p| &p.key == k).map(|p| (p.x as f64, (p.y - 22) as f64));
                }
                let h = self.w.get(id)?;
                Some(((jround(h.x) + if h.dir < 0.0 { -1 } else { 0 }) as f64, (jround(h.y) - if h.lying { 20 } else { 29 }) as f64))
            };
            let (Some((ax, ay)), Some((bx, by))) = (end(&bm.from, &bm.from_table), end(&bm.to, &bm.to_table)) else { continue };
            let Some(f) = self.w.get(if bm.from_table.is_some() { &bm.to } else { &bm.from }) else { continue };
            let up = 6.0;
            let pts = [(ax, ay), (ax, ay - up), (bx, by - up), (bx, by)];
            let len = [up, (bx - ax).hypot(by - ay), up];
            let total = len[0] + len[1] + len[2];
            let at = |mut s: f64| {
                let mut k = 0;
                while k < 2 && s > len[k] {
                    s -= len[k];
                    k += 1;
                }
                let ((x0, y0), (x1, y1)) = (pts[k], pts[k + 1]);
                let u = if len[k] != 0.0 { s / len[k] } else { 0.0 };
                (jround(x0 + (x1 - x0) * u), jround(y0 + (y1 - y0) * u))
            };
            let head = (bm.t / BEAM).min(1.0) * total;
            let tail = ((bm.t - 0.25) / BEAM).max(0.0) * total;
            let col = color_of(f);
            let mut s = tail.ceil();
            while s <= head {
                let (x, y) = at(s);
                ctx.rect(x, y + 1, 1, 1, c::INK);
                ctx.rect(x, y, 1, 1, col);
                s += 1.0;
            }
            if head < total {
                let (x, y) = at(head);
                ctx.rect(x - 1, y - 1, 3, 3, c::INK);
                ctx.rect(x - 1, y - 1, 2, 2, c::STEEL4);
            }
        }
        // shots (hardcore): a flash where it hit, a puff of smoke after
        let now = inp.now;
        self.w.s.shots.retain(|sh| now - sh.t0 < 700.0);
        for sh in self.w.s.shots.clone() {
            let k = (now - sh.t0) / 700.0;
            let (x, y) = (sh.x as i32, sh.y as i32);
            if sh.dust {
                // one gone wide: dust off the floor, two chips flying
                let u = jround(k * 5.0);
                ctx.set_alpha(1.0 - k);
                ctx.rect(x - 2 - u, y - 2 - u, 5 + 2 * u, 2 + u, Color::rgb(0xb9a48a));
                ctx.set_alpha(1.0);
                if k < 0.5 {
                    ctx.rect(x - 3 - jround(k * 12.0), y - 3 - jround(k * 10.0), 1, 1, Color::rgb(0xe8d6b8));
                    ctx.rect(x + 3 + jround(k * 10.0), y - 2 - jround(k * 12.0), 1, 1, Color::rgb(0xe8d6b8));
                }
                continue;
            }
            if k < 0.3 {
                let r = if k < 0.15 { 4 } else { 3 };
                ctx.rect(x - r, y, 2 * r + 1, 1, Color::rgb(0xfff6c0));
                ctx.rect(x, y - r, 1, 2 * r + 1, Color::rgb(0xfff6c0));
                ctx.rect(x - 1, y - 1, 3, 3, Color::rgb(0xffe27a));
            } else {
                ctx.set_alpha(1.0 - k);
                ctx.rect(x - 2, y - 2 - jround(k * 6.0), 5, 3, Color::rgb(0xc9ced8));
                ctx.set_alpha(1.0);
            }
        }
        // the neuralyzer's flash: the room goes white a moment, brightest at the flasher
        if let Some((fx, fy, t0)) = self.w.s.flash {
            let k = (now - t0) / 500.0;
            if k >= 1.0 {
                self.w.s.flash = None;
            } else {
                if !inp.still {
                    ctx.set_alpha(0.5 * (1.0 - k));
                    ctx.rect(0, 0, self.size.w, self.size.h, c::WHITE);
                }
                let r = jround(3.0 + k * 14.0);
                ctx.set_alpha(0.9 * (1.0 - k));
                ctx.rect(fx as i32 - r, fy as i32 - jround(r as f64 / 2.0), 2 * r, r, c::WHITE);
                ctx.set_alpha(1.0);
            }
        }
        // talk, over everything: dots in a bubble, never the words; a bot that needs input asks, a ? in its bubble
        for f in &self.w.folk {
            if f.waiting && !f.gone && f.act.kind != "doom" {
                let (bx, by) = (jround(f.x) + 3, jround(f.y) - 30);
                ctx.rect(bx - 1, by - 1, 9, 9, c::INK);
                ctx.rect(bx, by, 7, 7, c::STEEL4);
                ctx.rect(bx, by + 7, 1, 1, c::STEEL4);
                tiny(ctx, "?", (bx + 2) as f64, (by + 1) as f64, c::INK);
            }
        }
        for f in &self.w.folk {
            if f.chat > 0.0 && !inp.still && !f.waiting && !(f.act.kind == "chess" && !f.human) {
                let (bx, by) = (jround(f.x) + 3, jround(f.y) - if f.lying { 20 } else { 27 } - 6);
                ctx.rect(bx - 1, by - 1, 11, 7, c::INK);
                ctx.rect(bx, by, 9, 5, c::STEEL4);
                ctx.rect(bx, by + 5, 1, 1, c::STEEL4);
                for i in 0..3 {
                    if ((f.chat * 4.0).floor() as i32 + i) % 3 != 0 {
                        ctx.rect(bx + 2 + i * 2, by + 2, 1, 1, c::INK);
                    }
                }
            }
        }
        // the bot under the pointer: its name over its head, and a click opens it
        if let Some(hv) = self.hovered.clone()
            && self.moving.is_none()
            && let Some(f) = self.w.get(&hv.id)
        {
            let name = f.b.as_ref().map(|b| b.name.clone()).unwrap_or_default();
            let doomed = f.act.kind == "doom";
            if f.staff {
                self.tip = Some(Tip { text: format!("{name} - office manager"), x: hv.x, y: hv.y - 2.0 });
            } else if !doomed {
                self.tip = Some(Tip { text: format!("{name}{}", if f.waiting { " - needs input" } else { "" }), x: hv.x, y: hv.y - 2.0 });
            }
            let action = if f.staff {
                serde_json::json!({"kind": "manager", "key": f.booth})
            } else if f.human {
                serde_json::json!({"kind": "you"})
            } else if doomed {
                serde_json::json!({"kind": "undo", "id": hv.id})
            } else {
                serde_json::json!({"kind": "open", "id": hv.id})
            };
            self.hots.push(Hot { r: [-1e9, -1e9, 1e9, 1e9], action });
        }
        let dead = crate::mutiny::dead(&self.w);
        if let Some(tip) = self.tip.clone()
            && self.carrying.is_none()
            && !dead
        {
            let tw = tiny_w(&tip.text);
            let lx = jround(((self.size.w - tw - 2) as f64).min(tip.x - tw as f64 / 2.0).max(2.0));
            let ly = jround(((WALL + 2) as f64).max(tip.y - 8.0));
            ctx.rect(lx - 3, ly - 3, tw + 6, 11, c::INK);
            ctx.rect(lx - 2, ly - 2, tw + 4, 9, c::INK2);
            tiny(ctx, &tip.text, lx as f64, ly as f64, c::STEEL4);
        }
        // you died (the mutiny): the room goes red, a dark band across it with YOU DIED, and RESPAWN under it
        if dead {
            let since = now - self.w.s.mutiny.as_ref().map_or(now, |m| m.dead_at);
            let (w, h) = (self.size.w, self.size.h);
            let k = (since / 1200.0).min(1.0);
            ctx.save();
            ctx.set_alpha(k);
            ctx.set_op(Op::Multiply);
            ctx.rect(0, 0, w, h, Color::rgb(0xe0473a));
            ctx.restore();
            ctx.set_alpha(0.3 * k);
            ctx.rect(0, 0, w, h, Color::rgb(0x4a0004));
            ctx.set_alpha(1.0);
            if since > 900.0 {
                let (s, text) = (4, "YOU DIED");
                let tw = tiny_w(text) * s;
                let cy = jround((WALL + h) as f64 / 2.0) - 18;
                let x0 = jround((w - tw) as f64 / 2.0);
                ctx.set_alpha(0.72);
                ctx.rect(0, cy - 9, w, 5 * s + 18, Color::rgb(0x000000));
                ctx.set_alpha(1.0);
                for (dx, col) in [(1, Color::rgb(0x3a0306)), (0, Color::rgb(0xc8261f))] {
                    ctx.save();
                    ctx.translate((x0 + dx) as f64, (cy + dx) as f64);
                    ctx.scale(s as f64, s as f64);
                    tiny(ctx, text, 0.0, 0.0, col);
                    ctx.restore();
                }
                let bt = "RESPAWN";
                let (bw, bh) = (tiny_w(bt) + 12, 11);
                let (bx, by) = (jround((w - bw) as f64 / 2.0), cy + 5 * s + 16);
                let on = self.over((bx - 1) as f64, (by - 1) as f64, (bx + bw + 1) as f64, (by + bh + 1) as f64);
                crate::draw::box_(ctx, bx, by, bw, bh, c::STEEL3, if on { GOLD } else { c::INK });
                ctx.rect(bx, by, bw, 1, c::STEEL4);
                ctx.rect(bx, by + bh - 1, bw, 1, c::STEEL1);
                tiny(ctx, bt, (bx + 6) as f64, (by + 3) as f64, c::INK);
                self.hots.push(Hot { r: [(bx - 1) as f64, (by - 1) as f64, (bx + bw + 1) as f64, (by + bh + 1) as f64], action: serde_json::json!({"kind": "respawn"}) });
            }
        }
        let id = match &self.held {
            Some(h) => Some(h.clone()), // (a bot carried stays lit: its chair, its session)
            None => self.hovered.as_ref().map(|h| h.id.clone()),
        };
        if id != self.hover_id {
            self.hover_id = id.clone();
            out.hover = Some(id);
        }
        let on_hot = self.hots.iter().any(|h| self.over(h.r[0], h.r[1], h.r[2], h.r[3]));
        self.cursor = if self.moving.is_some() || self.held.is_some() {
            "grabbing"
        } else if on_hot {
            "pointer"
        } else if self.piece_at(self.pointer).is_some() {
            "grab"
        } else {
            "default"
        };
        out.cursor = self.cursor;
        let _ = l;
    }

    pub(crate) fn draw_figure_at(&mut self, ctx: &mut Ctx, i: usize, inp: &Input, hl: &[String], t: f64) {
        let f = &self.w.folk[i];
        if f.b.is_none() {
            return;
        }
        let (x, y) = (jround(f.x), jround(f.y));
        // taken aback (condemned, the gun not yet coming): a little hop
        let j = if f.act.kind == "condemned" && !f.act.panic && !f.held { t - f.act.jolt } else { 1.0 };
        let hop = if (0.0..0.3).contains(&j) { jround(2.0 * (j / 0.3 * std::f64::consts::PI).sin()) } else { 0 };
        let (x0, y0) = (x - 11, y - 27 - hop);
        if !f.held {
            ctx.rect(x - 5, y, 10, 1, Color::rgba(0, 0, 0, 102));
        }
        let im = crate::figure::body_dress(
            &f.seed,
            color_of(f),
            if f.held { Arms::Up } else { Arms::parse(f.arms) },
            if f.held { Legs::Walk1 } else { Legs::parse(f.legs) },
            f.human,
            f.staff,
            f.red,
            dress_of(f, self.spooky),
        );
        let up = im.h as i32 - SH; // (a witch's hat: the picture is taller, the figure where it was)
        // its outline: gold while the pointer is on it or on its session, faint while its session is open
        let over_it = self.over((x0 + 4) as f64, y0 as f64, (x0 + 18) as f64, (y + 1) as f64);
        let lit = hl.contains(&f.id) || self.hovered.as_ref().is_some_and(|h| h.id == f.id) || (self.hovered.is_none() && over_it);
        let bt = if f.held { -1.0 } else { booth_t(Some(f)) };
        let ring = if f.leaving || bt >= 0.0 {
            None
        } else if f.held && self.danger_now {
            Some(RED)
        } else if lit {
            Some(GOLD)
        } else if inp.opens.contains(&f.id) {
            Some(OPEN)
        } else {
            None
        };
        let fear = if !f.human && !f.staff && !f.held { if f.act.kind == "condemned" && f.act.panic { 1.0 } else if f.act.kind == "mutiny" { 0.0 } else { self.w.s.fear } } else { 0.0 };
        let shake = if fear > 0.2 && (t * 15.0 + f.z * 7.0).floor() as i64 % (if fear > 0.6 { 2 } else { 4 }) == 0 {
            if (t * 30.0).floor() as i64 % 2 != 0 { 1.0 } else { -1.0 }
        } else {
            0.0
        };
        ctx.save();
        if shake != 0.0 {
            ctx.translate(shake, 0.);
        }
        if f.dir < 0.0 {
            ctx.translate((x0 + SW) as f64, 0.);
            ctx.scale(-1., 1.);
        }
        let dx = if f.dir < 0.0 { 0 } else { x0 };
        let iy = (y0 - up) as f64;
        if let Some(col) = ring {
            ctx.draw_image(&halo(&im, col), dx as f64, iy);
        }
        if bt > 0.0 {
            ctx.draw_image(&crate::paint::dissolve(&im, booth_rise(bt)), dx as f64, iy);
        } else if f.held && self.danger_now {
            ctx.draw_image(&crate::paint::tinted(&im, RED, flash_a(t)), dx as f64, iy);
        } else {
            ctx.draw_image(&im, dx as f64, iy);
        }
        ctx.restore();
        if fear > 0.45 {
            // a drop of sweat running down beside its head
            let ph = (t * 1.4 + f.z) % 1.0;
            let sx = x + if f.dir < 0.0 { -8 } else { 7 };
            if ph < 0.8 {
                ctx.rect(sx, y0 + 5 + (ph * 6.0).floor() as i32, 1, 2, c::BLUE3);
                ctx.rect(sx, y0 + 5 + (ph * 6.0).floor() as i32, 1, 1, c::STEEL4);
            }
        }
        if f.act.kind == "neuralyze" && f.act.stage == 2 {
            // dazed: little stars round its head
            for k in 0..3 {
                let a = t * 4.0 + k as f64 * 2.1;
                ctx.rect(x - 1 + jround(a.cos() * 6.0), y0 + 3 + jround(a.sin() * 2.0), 1, 1, crate::sprites::sp::Y);
            }
        }
        if f.act.kind == "grill" && f.act.t > 0.0 && !f.held {
            crate::draw::draw_sausage(ctx, x, y, f.dir, f.act.t, f.act.cook, t);
        }
        if f.act.kind == "plant" && f.act.water && (t * 6.0).floor() as i64 % 2 != 0 {
            let d = f.dir as i32;
            ctx.rect(x + 10 * d, y0 + 21, 1, 1, c::BLUE3);
            ctx.rect(x + 11 * d, y0 + 24, 1, 1, c::BLUE3);
        }
        if !f.leaving && !f.held && over_it {
            self.hovered = Some(Hovered { id: f.id.clone(), x: x as f64, y: y0 as f64, seat: false });
        }
    }
}

/// Halloween (a lit pumpkin in the room, `spooky`): everyone in costume, a bot's by its seed; you the witch's hat, an
/// office manager the cape (her bow stays)
pub fn dress_of(f: &Fig, spooky: bool) -> Option<Dress> {
    if !spooky {
        None
    } else if f.human {
        Some(Dress::Hat)
    } else if f.staff {
        Some(Dress::Cape)
    } else {
        Some([Dress::Hat, Dress::Cape, Dress::Bones][fnv(&f.seed) as usize % 3])
    }
}

/// the danger's flash: the wash goes between two strengths, five times a second
pub fn flash_a(t: f64) -> f64 {
    if (t * 5.0).floor() as i64 % 2 != 0 { 0.5 } else { 0.3 }
}
