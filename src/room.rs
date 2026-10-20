// The room: one machine. Every project (the directory a session was started in) is a rug with a desk
// per bot. Where a bot is says what it is doing — at its desk typing = busy, up and about on its rug
// = idle, asleep at its desk = idle for a long while. A bot that sends another one a message walks
// over to it (across the room for another project) and stands there talking for a moment.
// Nothing else signals anything.

use crate::host::Bot;
use crate::sprite::*;
use egui::{Color32, Mesh, Pos2, Rect, Vec2};
use std::collections::HashMap;

const WALL: f32 = 24.0;
const DESK_W: f32 = 30.0;
const POD_H: f32 = 70.0;
const SLEEP_AFTER: f64 = 600.0;
const TALK: f32 = 4.0; // seconds standing next to the other once there
const WOOD: Color32 = hex(0x7a4a35);
const WOOD_D: Color32 = hex(0x4a2e2a);
const WALLC: Color32 = hex(0x1e1a2c);
const FLOOR: Color32 = hex(0x130f1b);
const PLANK: Color32 = hex(0x19141f);

pub enum Action {
    Open(String),
    Spawn(String),
}

struct Fig {
    x: f32,
    y: f32,
    dir: f32,
    target: Option<(f32, f32)>,
    wait: f32,
    walked: f32,
    seated: bool,
    busy: bool,
    idle_since: f64,
    visit: Option<Visit>,
}

/// on its way to another bot, or standing next to it, to say something
struct Visit {
    to: String,
    here: bool,
    stay: f32,
    patience: f32, // gives up if it can't get there
}

struct Pod {
    cwd: String,
    rect: Rect, // in dots
    color: Color32,
    desks: Vec<(String, Pos2)>,
    plus: Pos2,
}

pub struct Room {
    figs: HashMap<String, Fig>,
    sprites: Sprites,
    font: Font,
    seed: u32,
}

pub fn project_color(cwd: &str) -> Color32 {
    PROJECT[(fnv(cwd) % PROJECT.len() as u32) as usize]
}

pub fn project_label(cwd: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if cwd == home || cwd == "/root" || (cwd.starts_with("/home/") && cwd.matches('/').count() == 2) || (cwd.starts_with("/Users/") && cwd.matches('/').count() == 2) {
        return "home".into();
    }
    cwd.trim_end_matches('/').rsplit('/').next().unwrap_or(cwd).to_string()
}

impl Room {
    /// `from` sent `to` a message
    pub fn talk(&mut self, from: &str, to: &str) {
        if !self.figs.contains_key(to) {
            return;
        }
        // already together: talk a bit longer
        for (a, b) in [(from, to), (to, from)] {
            if let Some(v) = self.figs.get_mut(a).and_then(|f| f.visit.as_mut()).filter(|v| v.to == b) {
                v.stay = v.stay.max(TALK);
                return;
            }
        }
        if let Some(f) = self.figs.get_mut(from) {
            f.visit = Some(Visit { to: to.to_string(), here: false, stay: TALK, patience: 45.0 });
        }
    }

    pub fn new() -> Self {
        Room { figs: HashMap::new(), sprites: Sprites::default(), font: Font::new(), seed: 0x9e3779b9 }
    }

    fn rnd(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x % 100_000) as f32 / 100_000.0
    }

    fn layout(bots: &[Bot], w: f32) -> Vec<Pod> {
        let mut order: Vec<&str> = Vec::new();
        for b in bots {
            if !order.contains(&b.cwd.as_str()) {
                order.push(&b.cwd);
            }
        }
        let font = Font::new();
        let (mut x, mut y) = (8.0, WALL + 10.0);
        let mut pods = Vec::new();
        for cwd in order {
            let mine: Vec<&Bot> = bots.iter().filter(|b| b.cwd == cwd).collect();
            let label_w = font.width(&project_label(cwd)) as f32 + 12.0;
            let pw = ((mine.len() + 1) as f32 * DESK_W + 12.0).max(label_w).min(w - 16.0);
            if x + pw > w - 8.0 && x > 8.0 {
                x = 8.0;
                y += POD_H + 10.0;
            }
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(pw, POD_H));
            let desks = mine
                .iter()
                .enumerate()
                .map(|(i, b)| (b.id.clone(), Pos2::new(x + 18.0 + i as f32 * DESK_W, y + 40.0)))
                .collect();
            let plus = Pos2::new(x + 18.0 + mine.len() as f32 * DESK_W, y + 40.0);
            pods.push(Pod { cwd: cwd.to_string(), rect, color: project_color(cwd), desks, plus });
            x += pw + 10.0;
        }
        pods
    }

    /// advance and draw; `open` is the bot whose session is showing
    pub fn ui(&mut self, ui: &mut egui::Ui, bots: &[Bot], open: Option<&str>, title: &str, note: Option<&str>) -> Option<Action> {
        let rect = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(rect, egui::Sense::click());
        let k: f32 = if rect.width() < 520.0 { 2.0 } else { 3.0 };
        let (w, h) = (rect.width() / k, rect.height() / k);
        let ppp = ui.ctx().pixels_per_point();
        let o = Pos2::new((rect.min.x * ppp).round() / ppp, (rect.min.y * ppp).round() / ppp);
        let (now, dt) = ui.input(|i| (i.time, i.stable_dt.min(0.1)));
        let pods = Self::layout(bots, w);

        // keep a figure per bot
        self.figs.retain(|id, _| bots.iter().any(|b| &b.id == id));
        for pod in &pods {
            for (id, d) in &pod.desks {
                if !self.figs.contains_key(id) {
                    let seat = Pos2::new(d.x - 7.0, d.y + 1.0);
                    self.figs.insert(
                        id.clone(),
                        Fig { x: seat.x, y: seat.y, dir: 1.0, target: None, wait: 0.0, walked: 0.0, seated: true, busy: false, idle_since: now, visit: None },
                    );
                }
            }
        }
        // where everyone is, and who has a visitor standing next to them
        let at: HashMap<String, (f32, f32, bool)> = self.figs.iter().map(|(id, f)| (id.clone(), (f.x, f.y, f.seated))).collect();
        let visited: HashMap<String, f32> = self
            .figs
            .values()
            .filter_map(|f| f.visit.as_ref().filter(|v| v.here).map(|v| (v.to.clone(), f.x)))
            .collect();
        // walk
        for pod in &pods {
            for (id, d) in &pod.desks {
                let b = bots.iter().find(|b| &b.id == id).unwrap();
                let seat = (d.x - 7.0, d.y + 1.0);
                let floor = Rect::from_min_max(
                    Pos2::new(pod.rect.left() + 10.0, pod.rect.top() + 52.0),
                    Pos2::new(pod.rect.right() - 10.0, pod.rect.bottom() - 2.0),
                );
                let (r1, r2, r3) = (self.rnd(), self.rnd(), self.rnd());
                let f = self.figs.get_mut(id).unwrap();
                if b.busy != f.busy {
                    f.busy = b.busy;
                    f.idle_since = now;
                    f.wait = if b.busy { 0.0 } else { 0.6 + r1 * 1.5 };
                    f.target = if b.busy { Some(seat) } else { None };
                }
                let asleep = !f.busy && now - f.idle_since > SLEEP_AFTER;
                let mut fast = false;
                if let Some(v) = &mut f.visit {
                    v.patience -= dt;
                    match at.get(&v.to) {
                        Some(&(ox, oy, oseated)) if v.patience > 0.0 => {
                            // next to the other one: on the side it comes from, left of the chair if seated
                            let spot = if oseated {
                                (ox - 10.0, oy + 2.0)
                            } else if f.x < ox {
                                (ox - 11.0, oy)
                            } else {
                                (ox + 11.0, oy)
                            };
                            let near = (spot.0 - f.x).abs() < 1.5 && (spot.1 - f.y).abs() < 1.5;
                            if v.here || near {
                                v.here = true;
                                f.target = None;
                                f.dir = (ox - f.x).signum();
                                v.stay -= dt;
                                // the other one wandered off: follow it
                                if !near {
                                    v.here = false;
                                }
                            } else {
                                f.target = Some(spot);
                                fast = true;
                            }
                            if v.stay <= 0.0 {
                                f.visit = None;
                                f.wait = 0.5 + r1;
                            }
                        }
                        _ => f.visit = None,
                    }
                } else if let Some(&vx) = visited.get(id).filter(|_| !f.busy) {
                    // someone came over to talk: stand still and face them (a busy one stays at its desk)
                    if !f.seated {
                        f.target = None;
                        f.dir = (vx - f.x).signum();
                    }
                } else if f.busy || asleep {
                    if !f.seated && f.target.is_none() {
                        f.target = Some(seat);
                    }
                } else if f.target.is_none() {
                    f.wait -= dt;
                    if f.wait <= 0.0 {
                        // somewhere on the rug, now and then back to the chair for a while
                        f.target = Some(if r3 < 0.2 {
                            seat
                        } else {
                            (floor.left() + r1 * floor.width(), floor.top() + r2 * floor.height())
                        });
                        f.wait = 2.0 + r3 * 5.0;
                    }
                }
                if let Some((tx, ty)) = f.target {
                    f.seated = false;
                    let (dx, dy) = (tx - f.x, ty - f.y);
                    let dist = (dx * dx + dy * dy).sqrt();
                    let step = if fast { 34.0 } else { 22.0 } * dt;
                    if dist <= step {
                        f.x = tx;
                        f.y = ty;
                        f.target = None;
                        f.seated = (tx, ty) == seat;
                        if f.seated {
                            f.dir = 1.0;
                        }
                    } else {
                        f.x += dx / dist * step;
                        f.y += dy / dist * step;
                        f.walked += step;
                        if dx.abs() > 0.3 {
                            f.dir = dx.signum();
                        }
                    }
                }
            }
        }

        // paint, in dots
        let mut m = Mesh::default();
        let mut px = |m: &mut Mesh, x: f32, y: f32, ww: f32, hh: f32, c: Color32| {
            m.add_colored_rect(Rect::from_min_size(o + Vec2::new(x * k, y * k), Vec2::new(ww * k, hh * k)), c);
        };
        let boxed = |m: &mut Mesh, px: &mut dyn FnMut(&mut Mesh, f32, f32, f32, f32, Color32), x: f32, y: f32, ww: f32, hh: f32, fill: Color32| {
            px(m, x - 1.0, y - 1.0, ww + 2.0, hh + 2.0, INK);
            px(m, x, y, ww, hh, fill);
        };
        px(&mut m, 0.0, 0.0, w, h, FLOOR);
        let mut yy = WALL + 6.0;
        while yy < h {
            px(&mut m, 0.0, yy, w, 1.0, PLANK);
            yy += 9.0;
        }
        px(&mut m, 0.0, 0.0, w, WALL, WALLC);
        px(&mut m, 0.0, WALL, w, 2.0, INK2);
        let text = |m: &mut Mesh, s: &str, x: f32, y: f32, c: Color32| {
            let mut dots = Vec::new();
            self.font.dots(s, x as i32, y as i32, &mut dots);
            for (dx, dy) in dots {
                m.add_colored_rect(Rect::from_min_size(o + Vec2::new(dx as f32 * k, dy as f32 * k), Vec2::splat(k)), c);
            }
        };
        text(&mut m, title, 8.0, 9.0, STEEL2);
        let clock = chrono_hm();
        let cwid = self.font.width(&clock) as f32;
        text(&mut m, &clock, w - cwid - 8.0, 9.0, STEEL1);
        if let Some(n) = note {
            // a note goes on the wall, between the room's name and the clock
            let nx = (self.font.width(title) as f32 + 24.0).min(w - 8.0);
            let room_for = ((w - cwid - 16.0 - nx) / 4.0).max(0.0) as usize;
            let short: String = n.chars().take(room_for).collect();
            text(&mut m, &short, nx, 9.0, hex(0xff6b5a));
        } else if bots.is_empty() {
            text(&mut m, "no bots here yet", 8.0, WALL + 12.0, STEEL1);
        }

        let hover = resp.hover_pos().map(|p| ((p.x - o.x) / k, (p.y - o.y) / k));
        let mut hovered_fig: Option<String> = None;
        let mut hovered_plus: Option<String> = None;
        let t = now as f32;

        for pod in &pods {
            let r = pod.rect;
            boxed(&mut m, &mut px, r.left(), r.top() + 14.0, r.width(), r.height() - 14.0, shade(pod.color, 0.16));
            px(&mut m, r.left(), r.top() + 14.0, r.width(), 1.0, shade(pod.color, 0.26));
            text(&mut m, &project_label(&pod.cwd), r.left() + 1.0, r.top() + 2.0, pod.color);
            for (id, d) in &pod.desks {
                let f = &self.figs[id];
                let b = bots.iter().find(|b| &b.id == id).unwrap();
                let typing = f.seated && f.busy;
                let asleep = !f.busy && now - f.idle_since > SLEEP_AFTER;
                let (x, y) = (d.x, d.y);
                boxed(&mut m, &mut px, x - 3.0, y - 12.0, 18.0, 2.0, WOOD);
                px(&mut m, x - 3.0, y - 10.0, 18.0, 1.0, WOOD_D);
                boxed(&mut m, &mut px, x - 2.0, y - 9.0, 2.0, 9.0, WOOD_D);
                boxed(&mut m, &mut px, x + 12.0, y - 9.0, 2.0, 9.0, WOOD_D);
                boxed(&mut m, &mut px, x + 2.0, y - 22.0, 11.0, 8.0, STEEL0);
                let on = !asleep && b.background || f.busy;
                px(&mut m, x + 3.0, y - 21.0, 9.0, 6.0, if on { BLUE0 } else { NIGHT });
                if on {
                    let c = if typing { pod.color } else { shade(pod.color, 0.45) };
                    let s = if typing { (t * 5.0) as usize } else { 0 };
                    for row in 0..3 {
                        let len = 2 + ((row * 7 + s + id.len()) * 5) % 6;
                        px(&mut m, x + 4.0, y - 20.0 + row as f32 * 2.0, len as f32, 1.0, c);
                    }
                }
                px(&mut m, x + 7.0, y - 14.0, 1.0, 2.0, STEEL1);
                px(&mut m, x - 1.0, y - 13.0, 5.0, 1.0, STEEL2);
            }
            // an empty desk at the end: start another bot in this project
            let p = pod.plus;
            let over = hover.is_some_and(|(hx, hy)| hx >= p.x - 4.0 && hx <= p.x + 16.0 && hy >= p.y - 24.0 && hy <= p.y + 2.0);
            let c = if over { pod.color } else { shade(pod.color, 0.4) };
            for i in (0..18).step_by(2) {
                px(&mut m, p.x - 3.0 + i as f32, p.y - 12.0, 1.0, 1.0, c);
            }
            text(&mut m, "+", p.x + 4.0, p.y - 21.0, c);
            if over {
                hovered_plus = Some(pod.cwd.clone());
            }
        }

        // the bots, back to front
        let mut order: Vec<(&String, &Fig)> = self.figs.iter().collect();
        order.sort_by(|a, b| a.1.y.partial_cmp(&b.1.y).unwrap());
        let mut labels = Vec::new();
        for (id, f) in order {
            let Some(b) = bots.iter().find(|b| &b.id == id) else { continue };
            let color = project_color(&b.cwd);
            let asleep = !f.busy && now - f.idle_since > SLEEP_AFTER;
            let walking = f.target.is_some();
            let phase = (f.walked / 3.0) as i32 % 2 == 0;
            let (arms, legs) = if walking {
                if phase { (Arms::Swing1, Legs::Walk1) } else { (Arms::Swing2, Legs::Walk2) }
            } else if f.seated && f.busy {
                let tap = ((t * 7.0 + (fnv(id) % 7) as f32) as i32) % 2 == 0;
                (if tap { Arms::Type1 } else { Arms::Type2 }, Legs::Sit)
            } else if f.seated {
                (Arms::Down, Legs::Sit)
            } else {
                (Arms::Down, Legs::Stand)
            };
            let sprite = self.sprites.get(id, color, arms, legs);
            let ghost = !b.background;
            let x0 = f.x.round() - 11.0;
            let y0 = f.y.round() - 27.0;
            let flip = f.dir < 0.0;
            px(&mut m, f.x.round() - 5.0, f.y.round(), 10.0, 1.0, Color32::from_black_alpha(100));
            for &(sx, sy, c) in sprite.iter() {
                let sx = if flip { SW - 1 - sx as i32 } else { sx as i32 };
                let c = if ghost { mix(c, shade(color, 0.16), 0.55) } else { c };
                px(&mut m, x0 + sx as f32, y0 + sy as f32, 1.0, 1.0, c);
            }
            if asleep {
                for i in 0..2 {
                    let kk = (t * 0.4 + i as f32 * 0.5) % 1.0;
                    let zx = (f.x + 2.0 + kk * 6.0).round();
                    let zy = (y0 + 4.0 - kk * 10.0).round();
                    for (dx, dy) in [(0, 0), (1, 0), (2, 0), (1, 1), (0, 2), (1, 2), (2, 2)] {
                        px(&mut m, zx + dx as f32, zy + dy as f32, 1.0, 1.0, STEEL3);
                    }
                }
            }
            if f.visit.as_ref().is_some_and(|v| v.here) {
                // talking: a little speech bubble, no words
                let bx = (f.x + f.dir * 3.0).round() - 4.0;
                let by = y0 - 3.0;
                px(&mut m, bx - 1.0, by - 1.0, 11.0, 7.0, INK);
                px(&mut m, bx, by, 9.0, 5.0, STEEL4);
                px(&mut m, bx + if f.dir < 0.0 { 1.0 } else { 6.0 }, by + 5.0, 2.0, 1.0, STEEL4);
                let n = ((t * 3.0) as i32 % 4) as usize;
                for i in 0..n.min(3) {
                    px(&mut m, bx + 1.0 + i as f32 * 3.0, by + 2.0, 1.0, 1.0, INK);
                }
            }
            if open == Some(id.as_str()) {
                for (i, wd) in [5.0, 3.0, 1.0].iter().enumerate() {
                    px(&mut m, f.x.round() - (wd / 2.0_f32).floor(), y0 - 6.0 + i as f32, *wd, 1.0, TERRACOTTA);
                }
            }
            if hover.is_some_and(|(hx, hy)| hx >= x0 + 3.0 && hx <= x0 + 19.0 && hy >= y0 && hy <= f.y + 1.0) {
                hovered_fig = Some(id.clone());
            }
            labels.push((id.clone(), f.x, y0));
        }
        // the hovered bot's name over its head
        if let Some(hid) = &hovered_fig {
            if let Some((_, fx, fy)) = labels.iter().find(|(i, _, _)| i == hid) {
                let b = bots.iter().find(|b| &b.id == hid).unwrap();
                let mut s = b.name.clone();
                if !b.background {
                    s.push_str(" (in a terminal elsewhere)");
                }
                let tw = self.font.width(&s) as f32;
                let lx = (fx - tw / 2.0).clamp(2.0, (w - tw - 2.0).max(2.0)).round();
                let ly = (fy - 14.0).max(2.0).round();
                boxed(&mut m, &mut px, lx - 2.0, ly - 2.0, tw + 4.0, 12.0, INK2);
                text(&mut m, &s, lx, ly, STEEL4);
            }
        }
        ui.painter().with_clip_rect(rect).add(m);

        if hovered_fig.is_some() || hovered_plus.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some(id) = hovered_fig {
                if bots.iter().any(|b| b.id == id && b.background) {
                    return Some(Action::Open(id));
                }
            } else if let Some(cwd) = hovered_plus {
                return Some(Action::Spawn(cwd));
            }
        }
        None
    }
}

fn chrono_hm() -> String {
    // local time without a date crate: ask libc
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as libc_time_t;
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    unsafe { localtime_r(&t, &mut tm) };
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

#[allow(non_camel_case_types)]
type libc_time_t = i64;
#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const i8,
}
unsafe extern "C" {
    fn localtime_r(t: *const libc_time_t, tm: *mut Tm) -> *mut Tm;
}

fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}
