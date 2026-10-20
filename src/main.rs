// burrow: persistent Claude Code sessions on your machines, shown as bots in a room.
// The sessions are Claude Code's own background sessions; burrow only watches, starts and attaches.

mod host;
mod room;
mod shot;
mod sprite;
mod term;

use egui::{Color32, FontData, FontDefinitions, FontFamily, RichText, UiBuilder, Vec2};
use host::{Host, Watch};
use sprite::*;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// the title bar's buttons sit over the room's wall, this wide (points)
const TRAFFIC_LIGHTS: f32 = 76.0;
const HEAD: f32 = 30.0;

enum Done {
    Spawned(usize, String),
    Failed(usize, String),
    Uploaded(usize, String, String),
    Stopped(usize),
    DirAdded(usize, String),
}

struct Place {
    host: Host,
    watch: Watch,
    room: room::Room,
    sessions: HashMap<String, term::Session>,
    open: Option<String>,
    note: Option<String>,
}

impl Place {
    fn new(host: Host, ctx: &egui::Context) -> Self {
        let watch = host::watch(host.clone(), ctx.clone());
        Place { host, watch, room: room::Room::new(), sessions: HashMap::new(), open: None, note: None }
    }
}

struct App {
    places: Vec<Place>,
    cur: usize,
    done: Arc<Mutex<Vec<Done>>>,
    add_dir: Option<String>,            // the "add a project" dialog: its directory
    add_room: Option<(String, String)>, // the "add a machine" dialog: name, ssh destination
    confirm_stop: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = &cc.egui_ctx;
        fonts(ctx);
        style(ctx);
        let places = host::load_hosts().into_iter().map(|h| Place::new(h, ctx)).collect();
        App { places, cur: 0, done: Arc::new(Mutex::new(Vec::new())), add_dir: None, add_room: None, confirm_stop: false }
    }

    fn later(&mut self, ctx: &egui::Context, job: impl FnOnce() -> Done + Send + 'static) {
        let (done, ctx) = (self.done.clone(), ctx.clone());
        std::thread::spawn(move || {
            let d = job();
            done.lock().unwrap().push(d);
            ctx.request_repaint();
        });
    }

    fn spawn(&mut self, ctx: &egui::Context, dir: String, name: String) {
        let (i, h) = (self.cur, self.places[self.cur].host.clone());
        self.later(ctx, move || match h.spawn(&dir, &name) {
            Ok(id) => Done::Spawned(i, id),
            Err(e) => Done::Failed(i, e),
        });
    }

    fn open(&mut self, ctx: &egui::Context, i: usize, id: &str) {
        let p = &mut self.places[i];
        let stale = p.sessions.get(id).is_some_and(|s| s.exited.load(Ordering::Relaxed));
        if stale {
            p.sessions.remove(id);
        }
        if !p.sessions.contains_key(id) {
            match term::Session::attach(&p.host, id, ctx) {
                Ok(s) => {
                    p.sessions.insert(id.to_string(), s);
                }
                Err(e) => {
                    p.note = Some(format!("could not attach: {e}"));
                    return;
                }
            }
        }
        p.open = Some(id.to_string());
        self.confirm_stop = false;
    }

    fn keep_hosts(&self) {
        host::save_hosts(&self.places.iter().map(|p| p.host.clone()).collect::<Vec<_>>());
    }

    fn finished(&mut self, ctx: &egui::Context) {
        let jobs: Vec<Done> = std::mem::take(&mut *self.done.lock().unwrap());
        for d in jobs {
            match d {
                Done::Spawned(i, id) => {
                    self.places[i].watch.poke.store(true, Ordering::Relaxed);
                    self.places[i].note = None;
                    self.open(ctx, i, &id);
                }
                Done::Failed(i, e) => self.places[i].note = Some(e.lines().next().unwrap_or("failed").to_string()),
                Done::Uploaded(i, id, path) => {
                    if let Some(s) = self.places[i].sessions.get(&id) {
                        s.type_path(&path);
                    }
                }
                Done::Stopped(i) => self.places[i].watch.poke.store(true, Ordering::Relaxed),
                Done::DirAdded(i, dir) => {
                    self.places[i].note = None;
                    if !self.places[i].host.dirs.contains(&dir) {
                        self.places[i].host.dirs.push(dir);
                        self.keep_hosts();
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.finished(&ctx);
        if let Ok(dir) = std::env::var("BURROW_SPAWN") {
            // SAFETY: only a test hook, read once before anything else uses the environment this frame
            unsafe { std::env::remove_var("BURROW_SPAWN") };
            self.spawn(&ctx, dir, "burrow-spawn-test".into());
        }
        if let Ok(id) = std::env::var("BURROW_OPEN") {
            if self.places[0].open.is_none() && self.places[0].watch.feed.lock().unwrap().bots.iter().any(|b| b.id == id) {
                // SAFETY: single-threaded at this point of the frame; only a test hook
                unsafe { std::env::remove_var("BURROW_OPEN") };
                self.open(&ctx, 0, &id);
            }
        }
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        for p in &self.places {
            p.watch.fast.store(focused, Ordering::Relaxed);
        }
        let ci = self.cur;
        let (bots, error, light) = {
            let f = self.places[ci].watch.feed.lock().unwrap();
            (f.bots.clone(), f.error.clone(), f.light)
        };
        for p in &mut self.places {
            let talks = std::mem::take(&mut p.watch.feed.lock().unwrap().talks);
            for (from, to) in talks {
                p.room.talk(&from, &to);
            }
        }
        // a project a bot was started in stays, also once its bots are gone
        let new: Vec<String> = bots.iter().filter(|b| b.background && !self.places[ci].host.dirs.contains(&b.cwd)).map(|b| b.cwd.clone()).collect();
        if !new.is_empty() {
            for d in new {
                if !self.places[ci].host.dirs.contains(&d) {
                    self.places[ci].host.dirs.push(d);
                }
            }
            self.keep_hosts();
        }
        // forget sessions whose bot has gone
        {
            let p = &mut self.places[ci];
            // (a session still attached stays: a bot just started isn't in the feed until the next poll)
            p.sessions.retain(|id, s| bots.iter().any(|b| &b.id == id) || !s.exited.load(Ordering::Relaxed));
            if p.open.as_ref().is_some_and(|id| !bots.iter().any(|b| &b.id == id) && !p.sessions.contains_key(id)) {
                p.open = None;
            }
        }

        let full = ui.available_rect_before_wrap();
        ui.painter().rect_filled(full, 0.0, INK);

        let open = self.places[ci].open.clone();
        let room_rect = if open.is_some() {
            egui::Rect::from_min_size(full.min, Vec2::new((full.width() * 0.36).clamp(300.0, 560.0), full.height()))
        } else {
            full
        };
        let note = error.clone().or_else(|| self.places[ci].note.clone());
        let rooms: Vec<String> = self.places.iter().map(|p| p.host.name.clone()).collect();
        let mut action = None;
        ui.scope_builder(UiBuilder::new().max_rect(room_rect), |ui| {
            let p = &mut self.places[ci];
            action = p.room.ui(ui, &bots, &p.host.dirs, open.as_deref(), &rooms, ci, TRAFFIC_LIGHTS, note.as_deref());
        });
        match action {
            Some(room::Action::Open(id)) => {
                if open.as_deref() == Some(id.as_str()) {
                    self.places[ci].open = None;
                } else {
                    self.open(&ctx, ci, &id);
                }
            }
            Some(room::Action::Spawn(cwd)) => self.spawn(&ctx, cwd, String::new()),
            Some(room::Action::AddDir) => self.add_dir = Some("~/".into()),
            Some(room::Action::Forget(cwd)) => {
                self.places[ci].host.dirs.retain(|d| d != &cwd);
                self.keep_hosts();
            }
            Some(room::Action::Room(i)) => self.cur = i,
            Some(room::Action::AddRoom) => self.add_room = Some((String::new(), String::new())),
            None => {}
        }

        // the open bot's session
        if let Some(id) = self.places[ci].open.clone() {
            let tr = egui::Rect::from_min_max(egui::pos2(room_rect.right(), full.top()), full.max);
            ui.painter().rect_filled(egui::Rect::from_min_size(tr.min, Vec2::new(1.0, tr.height())), 0.0, INK2);
            let head = egui::Rect::from_min_size(tr.min + Vec2::new(1.0, 0.0), Vec2::new(tr.width() - 1.0, HEAD));
            ui.painter().rect_filled(head, 0.0, hex(0x1e1a2c));
            // the header is title bar too
            let drag = ui.interact(head, ui.scope_id().with("head"), egui::Sense::click_and_drag());
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            let bot = bots.iter().find(|b| b.id == id).cloned();
            let mut close = false;
            let mut stop = false;
            let exited = self.places[ci].sessions.get(&id).is_some_and(|s| s.exited.load(Ordering::Relaxed));
            ui.scope_builder(UiBuilder::new().max_rect(head.shrink2(Vec2::new(12.0, 2.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(b) = &bot {
                        let c = room::project_color(&b.cwd);
                        ui.label(RichText::new(&b.name).color(c).strong());
                        ui.label(RichText::new(&b.cwd).color(STEEL1));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if flat(ui, "✕", STEEL2).on_hover_text("close (the bot keeps working)").clicked() {
                            close = true;
                        }
                        let (label, c) = if self.confirm_stop { ("really stop?", TERRACOTTA) } else { ("stop", STEEL1) };
                        if flat(ui, label, c).clicked() {
                            if self.confirm_stop {
                                stop = true;
                            }
                            self.confirm_stop = !self.confirm_stop;
                        }
                        if exited && flat(ui, "reconnect", TERRACOTTA).clicked() {
                            self.places[ci].sessions.remove(&id);
                            self.open(&ctx, ci, &id);
                        }
                    });
                });
            });
            let area = egui::Rect::from_min_max(egui::pos2(tr.left() + 1.0, head.bottom()), tr.max);
            ui.scope_builder(UiBuilder::new().max_rect(area).id_salt(("term", &id)), |ui| {
                if let Some(s) = self.places[ci].sessions.get_mut(&id) {
                    s.ui(ui, if light { term::LIGHT } else { term::DARK });
                }
            });
            // files dropped anywhere go to the open bot
            let (hovering, dropped) = ctx.input(|i| (!i.raw.hovered_files.is_empty(), i.raw.dropped_files.clone()));
            if hovering {
                ui.painter().rect_filled(area, 0.0, TERRACOTTA.gamma_multiply(0.12));
                ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, "drop to give it to this bot", egui::FontId::proportional(16.0), STEEL4);
            }
            for f in dropped {
                let path = f.path().to_path_buf();
                let (h, id2) = (self.places[ci].host.clone(), id.clone());
                self.later(&ctx, move || match h.upload(&path) {
                    Ok(remote) => Done::Uploaded(ci, id2, remote),
                    Err(e) => Done::Failed(ci, format!("upload: {e}")),
                });
            }
            if close {
                self.places[ci].open = None;
            }
            if stop {
                let h = self.places[ci].host.clone();
                self.places[ci].open = None;
                self.places[ci].sessions.remove(&id);
                self.later(&ctx, move || match h.stop(&id) {
                    Ok(_) => Done::Stopped(ci),
                    Err(e) => Done::Failed(ci, e),
                });
            }
        }

        self.dialogs(&ctx);
        if let Ok(path) = std::env::var("BURROW_SHOT") {
            shot::tick(&ctx, &path);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// a button that is only its word, like everything else in the room
fn flat(ui: &mut egui::Ui, s: &str, c: Color32) -> egui::Response {
    let r = ui.add(egui::Button::new(RichText::new(s).color(c)).frame(false));
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    r
}

/// a small dialog in the room's colours, without a title bar; false once it is closed
fn dialog(ctx: &egui::Context, title: &str, body: impl FnOnce(&mut egui::Ui)) -> bool {
    let mut keep = true;
    egui::Window::new(title)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(egui::Frame::new().fill(INK2).stroke(egui::Stroke::new(1.0, STEEL0)).inner_margin(14.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).color(STEEL4).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if flat(ui, "✕", STEEL2).clicked() {
                        keep = false;
                    }
                });
            });
            ui.add_space(6.0);
            body(ui);
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        keep = false;
    }
    keep
}

impl App {
    fn dialogs(&mut self, ctx: &egui::Context) {
        let mut go = None;
        if let Some(dir) = &mut self.add_dir {
            let keep = dialog(ctx, "a project", |ui| {
                ui.label(RichText::new("its directory on this machine").color(STEEL2));
                let r = ui.add(egui::TextEdit::singleline(dir).desired_width(320.0));
                r.request_focus();
                ui.add_space(6.0);
                if flat(ui, "add", TERRACOTTA).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    go = Some(dir.clone());
                }
            });
            if !keep || go.is_some() {
                self.add_dir = None;
            }
        }
        if let Some(dir) = go.filter(|d| !d.trim().is_empty()) {
            let (i, h) = (self.cur, self.places[self.cur].host.clone());
            self.later(ctx, move || match h.resolve(&dir) {
                Ok(abs) => Done::DirAdded(i, abs),
                Err(e) => Done::Failed(i, e),
            });
        }
        let mut add = None;
        if let Some((name, dest)) = &mut self.add_room {
            let keep = dialog(ctx, "a machine", |ui| {
                ui.label(RichText::new("ssh destination (as you'd type it after ssh)").color(STEEL2));
                ui.add(egui::TextEdit::singleline(dest).desired_width(280.0).hint_text("odyssey"));
                ui.label(RichText::new("name").color(STEEL2));
                ui.add(egui::TextEdit::singleline(name).desired_width(280.0));
                ui.add_space(6.0);
                if flat(ui, "add", TERRACOTTA).clicked() && !dest.trim().is_empty() {
                    let n = if name.trim().is_empty() { dest.trim() } else { name.trim() };
                    add = Some(Host { name: n.to_string(), ssh: Some(dest.trim().to_string()), ..Default::default() });
                }
            });
            if !keep || add.is_some() {
                self.add_room = None;
            }
        }
        if let Some(h) = add {
            self.places.push(Place::new(h, ctx));
            self.cur = self.places.len() - 1;
            self.keep_hosts();
        }
    }
}

fn fonts(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    f.font_data.insert("jbm".into(), Arc::new(FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf"))));
    f.font_data.insert("jbm-bold".into(), Arc::new(FontData::from_static(include_bytes!("../assets/JetBrainsMono-Bold.ttf"))));
    let fallback = f.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let with = |first: &str| std::iter::once(first.to_string()).chain(fallback.iter().cloned()).collect::<Vec<_>>();
    f.families.insert(FontFamily::Name(term::FONT.into()), with("jbm"));
    f.families.insert(FontFamily::Name(term::FONT_BOLD.into()), with("jbm-bold"));
    f.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "jbm".into());
    f.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "jbm".into());
    ctx.set_fonts(f);
}

fn style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = INK;
    v.window_fill = INK2;
    v.extreme_bg_color = INK;
    v.window_stroke = egui::Stroke::new(1.0, STEEL0);
    v.selection.bg_fill = TERRACOTTA.gamma_multiply(0.5);
    v.override_text_color = Some(STEEL3);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = 0.into();
    }
    v.widgets.inactive.weak_bg_fill = INK2;
    v.widgets.inactive.bg_fill = INK2;
    v.widgets.hovered.weak_bg_fill = STEEL0;
    v.widgets.active.weak_bg_fill = STEEL1;
    v.window_corner_radius = 0.into();
    v.window_shadow = egui::Shadow::NONE;
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| s.spacing.item_spacing = Vec2::new(8.0, 6.0));
}

fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--icon-dots") {
        // for macos/bundle.sh: the app icon is one of the room's own robots, standing
        for &(x, y, c) in Sprites::default().get("burrow", TERRACOTTA, Arms::Down, Legs::Stand).iter() {
            println!("{x} {y} {:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
        }
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
            .with_title("burrow")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([480.0, 320.0])
            // the room's wall is the title bar, as in a terminal: only the buttons stay
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false);
    if std::env::var_os("BURROW_SHOT").is_some() {
        // looking at itself: a covered window isn't drawn, so stay on top while that runs
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }
    let opts = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("burrow", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
