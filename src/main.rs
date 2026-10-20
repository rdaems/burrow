// burrow: persistent Claude Code sessions on your machines, shown as bots in a room.
// The sessions are Claude Code's own background sessions; burrow only watches, starts and attaches.

mod glyphs;
mod host;
mod room;
mod sprite;
mod term;

use egui::{Color32, FontData, FontDefinitions, FontFamily, RichText, UiBuilder, Vec2};
use host::{Host, Watch};
use sprite::*;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

enum Done {
    Spawned(usize, String),
    Failed(usize, String),
    Uploaded(usize, String, String),
    Stopped(usize),
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
    start: Option<(String, String)>,    // the "start a bot" dialog: directory, name
    add_room: Option<(String, String)>, // the "add a machine" dialog: name, ssh destination
    confirm_stop: bool,
    busy: usize, // spawns and uploads under way
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = &cc.egui_ctx;
        fonts(ctx);
        style(ctx);
        let places = host::load_hosts().into_iter().map(|h| Place::new(h, ctx)).collect();
        App { places, cur: 0, done: Arc::new(Mutex::new(Vec::new())), start: None, add_room: None, confirm_stop: false, busy: 0 }
    }

    fn later(&mut self, ctx: &egui::Context, job: impl FnOnce() -> Done + Send + 'static) {
        let (done, ctx) = (self.done.clone(), ctx.clone());
        self.busy += 1;
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

    fn finished(&mut self, ctx: &egui::Context) {
        let jobs: Vec<Done> = std::mem::take(&mut *self.done.lock().unwrap());
        for d in jobs {
            self.busy = self.busy.saturating_sub(1);
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
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.finished(&ctx);
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
        // forget sessions whose bot has gone
        {
            let p = &mut self.places[ci];
            p.sessions.retain(|id, _| bots.iter().any(|b| &b.id == id));
            if p.open.as_ref().is_some_and(|id| !bots.iter().any(|b| &b.id == id) && !p.sessions.contains_key(id)) {
                p.open = None;
            }
        }

        let full = ui.available_rect_before_wrap();
        ui.painter().rect_filled(full, 0.0, INK);

        // the bar: machines, and starting a bot
        let bar = egui::Rect::from_min_size(full.min, Vec2::new(full.width(), 30.0));
        ui.scope_builder(UiBuilder::new().max_rect(bar.shrink2(Vec2::new(8.0, 4.0))), |ui| {
            ui.horizontal_centered(|ui| {
                for (i, p) in self.places.iter().enumerate() {
                    if ui.selectable_label(i == self.cur, RichText::new(&p.host.name).color(if i == self.cur { STEEL4 } else { STEEL2 })).clicked() {
                        self.cur = i;
                    }
                }
                if ui.button(RichText::new("+ machine").color(STEEL1)).clicked() {
                    self.add_room = Some((String::new(), String::new()));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("+ bot").color(TERRACOTTA)).clicked() {
                        let dir = bots.last().map(|b| b.cwd.clone()).unwrap_or_else(|| "~".into());
                        self.start = Some((dir, String::new()));
                    }
                    if self.busy > 0 {
                        ui.spinner();
                    }
                });
            });
        });
        let body = egui::Rect::from_min_max(egui::pos2(full.left(), bar.bottom()), full.max);

        let open = self.places[ci].open.clone();
        let room_rect = if open.is_some() {
            egui::Rect::from_min_size(body.min, Vec2::new((body.width() * 0.36).clamp(300.0, 560.0), body.height()))
        } else {
            body
        };
        let note = error.clone().or_else(|| self.places[ci].note.clone());
        let title = self.places[ci].host.name.clone();
        let mut action = None;
        let mut clicked_floor = false;
        ui.scope_builder(UiBuilder::new().max_rect(room_rect), |ui| {
            let p = &mut self.places[ci];
            action = p.room.ui(ui, &bots, open.as_deref(), &title, note.as_deref());
            clicked_floor = ui.response().clicked();
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
            None => {}
        }

        // the open bot's session
        if let Some(id) = self.places[ci].open.clone() {
            let tr = egui::Rect::from_min_max(egui::pos2(room_rect.right(), body.top()), body.max);
            ui.painter().rect_filled(egui::Rect::from_min_size(tr.min, Vec2::new(1.0, tr.height())), 0.0, INK2);
            let head = egui::Rect::from_min_size(tr.min + Vec2::new(1.0, 0.0), Vec2::new(tr.width() - 1.0, 28.0));
            let bot = bots.iter().find(|b| b.id == id).cloned();
            let mut close = false;
            let mut stop = false;
            let exited = self.places[ci].sessions.get(&id).is_some_and(|s| s.exited.load(Ordering::Relaxed));
            ui.scope_builder(UiBuilder::new().max_rect(head.shrink2(Vec2::new(10.0, 2.0))), |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(b) = &bot {
                        let c = room::project_color(&b.cwd);
                        ui.label(RichText::new(&b.name).color(c).strong());
                        ui.label(RichText::new(&b.cwd).color(STEEL1));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✕").color(STEEL2)).on_hover_text("close (the bot keeps working)").clicked() {
                            close = true;
                        }
                        let label = if self.confirm_stop { "really stop?" } else { "stop" };
                        if ui.button(RichText::new(label).color(if self.confirm_stop { TERRACOTTA } else { STEEL1 })).clicked() {
                            if self.confirm_stop {
                                stop = true;
                            }
                            self.confirm_stop = !self.confirm_stop;
                        }
                        if exited && ui.button(RichText::new("reconnect").color(TERRACOTTA)).clicked() {
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
                {
                    let path = f.path().to_path_buf();
                    let (h, id2) = (self.places[ci].host.clone(), id.clone());
                    self.later(&ctx, move || match h.upload(&path) {
                        Ok(remote) => Done::Uploaded(ci, id2, remote),
                        Err(e) => Done::Failed(ci, format!("upload: {e}")),
                    });
                }
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
        let _ = clicked_floor;

        self.dialogs(&ctx, &bots);
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

impl App {
    fn dialogs(&mut self, ctx: &egui::Context, bots: &[host::Bot]) {
        let mut go = None;
        if let Some((dir, name)) = &mut self.start {
            let mut keep = true;
            egui::Window::new("start a bot").collapsible(false).resizable(false).open(&mut keep).show(ctx, |ui| {
                let mut dirs: Vec<&str> = bots.iter().map(|b| b.cwd.as_str()).collect();
                dirs.dedup();
                ui.label(RichText::new("directory (its project)").color(STEEL2));
                ui.add(egui::TextEdit::singleline(dir).desired_width(320.0));
                ui.horizontal_wrapped(|ui| {
                    let mut seen = Vec::new();
                    for d in dirs {
                        if !seen.contains(&d) {
                            seen.push(d);
                            if ui.small_button(room::project_label(d)).on_hover_text(d).clicked() {
                                *dir = d.to_string();
                            }
                        }
                    }
                });
                ui.label(RichText::new("name (optional)").color(STEEL2));
                ui.add(egui::TextEdit::singleline(name).desired_width(320.0));
                ui.add_space(6.0);
                if ui.button(RichText::new("start").color(TERRACOTTA)).clicked() {
                    go = Some((dir.clone(), name.clone()));
                }
            });
            if !keep || go.is_some() {
                self.start = None;
            }
        }
        if let Some((dir, name)) = go {
            self.spawn(ctx, dir, name);
        }
        let mut add = None;
        if let Some((name, dest)) = &mut self.add_room {
            let mut keep = true;
            egui::Window::new("add a machine").collapsible(false).resizable(false).open(&mut keep).show(ctx, |ui| {
                ui.label(RichText::new("ssh destination (as you'd type it after ssh)").color(STEEL2));
                ui.add(egui::TextEdit::singleline(dest).desired_width(280.0).hint_text("odyssey"));
                ui.label(RichText::new("name").color(STEEL2));
                ui.add(egui::TextEdit::singleline(name).desired_width(280.0));
                ui.add_space(6.0);
                if ui.button(RichText::new("add").color(TERRACOTTA)).clicked() && !dest.trim().is_empty() {
                    let n = if name.trim().is_empty() { dest.trim() } else { name.trim() };
                    add = Some(Host { name: n.to_string(), ssh: Some(dest.trim().to_string()) });
                }
            });
            if !keep || add.is_some() {
                self.add_room = None;
            }
        }
        if let Some(h) = add {
            self.places.push(Place::new(h, ctx));
            self.cur = self.places.len() - 1;
            host::save_hosts(&self.places.iter().map(|p| p.host.clone()).collect::<Vec<_>>());
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
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| s.spacing.item_spacing = Vec2::new(8.0, 6.0));
    let _ = Color32::WHITE;
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("burrow").with_inner_size([1280.0, 800.0]).with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native("burrow", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
