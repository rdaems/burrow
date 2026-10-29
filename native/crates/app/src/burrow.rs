//! The window (web/index.html): the room on the left, as wide as the divider is dragged; on the right the sessions
//! you opened, as tiles, or one view (a 16-bit panel) in their place. The room is burrow-room's buffer painted at
//! whole device pixels; the backend is burrow-core's Hub, in process: its events come in every frame, what the room
//! asks for goes to it (blocking calls off the main thread) or opens a tile or a panel here.

use crate::kit::{self, Frame, col, frame_bg, pixel_text};
use crate::tiles::{self, Side, Target, Tiles};
use crate::views::{self, ViewKind};
use burrow_core::{Event, Events, Hub};
use burrow_room::io::{DigestInfo, Input, Output, ShellInfo, ShellKey};
use burrow_room::layout::{K, MARGIN};
use burrow_room::room::Room;
use gpui::{
    AnyElement, App, Bounds, Context, Corners, CursorStyle, Entity, FocusHandle, Focusable, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, RenderImage, Styled, Window, canvas, div, prelude::*, px, rgb,
};
use serde_json::{Value, json};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// what is in a tile
pub enum TileKind {
    /// a session: `claude attach`
    Bot { m: String, id: String },
    /// your terminal at a screen
    Shell { m: String, dir: String },
    /// the office manager's notes
    Notes,
}

pub struct Tile {
    pub kind: TileKind,
    pub body: Option<Entity<crate::term::TermTile>>,
    pub notes: Option<Entity<crate::notes::Notes>>,
}

/// what is being dragged in the pane
#[derive(Clone)]
enum Drag {
    /// the divider: the room's width when it began, and where
    Divider { w0: f64, x0: f32 },
    /// a gap between tiles, the split's sizes when it began
    Gap { gap: tiles::Gap, sizes: Vec<f32>, at0: f32 },
    /// a tile by its band
    Tile { id: u64, x0: f32, y0: f32, on: bool, target: Option<Target> },
    /// the room's own pointer (a press in the room, a bot carried)
    Room,
    /// a piece carried out of the closet into the room: its kind, where the press was, whether it moved yet
    Closet { kind: String, x0: f32, y0: f32, on: bool },
}

pub struct Burrow {
    pub hub: Arc<Hub>,
    events: Events,
    pub state: burrow_model::State,
    pub digest: burrow_core::DigestView,
    pub room: Room,
    pub inp: Input,
    picture: Option<Arc<RenderImage>>,
    stale: Vec<Arc<RenderImage>>,
    room_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pane_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    started: Instant,
    last: Instant,
    dirty: bool,
    pub tiles: Tiles,
    pub tile: HashMap<u64, Tile>,
    next: u64,
    pub view: Option<ViewKind>,
    pub views: views::Views,
    drag: Option<Drag>,
    /// the tile the pointer is on, the bot (folk id) whose tile it is
    hover_tile: Option<u64>,
    lit_tiles: Vec<u64>,
    lit_shell: Option<String>,
    /// when the window lost the focus
    blur_at: Option<Instant>,
    scale: f32,
    pub focus: FocusHandle,
    pub sound: burrow_sound::Sound,
    pub outside: burrow_games::outside::Outside,
    pub pong: burrow_games::pong::Pong,
    /// the gun closet's first person, and its melt after
    pub fps: Option<crate::fpsview::FpsRun>,
    pub leaving: Option<crate::fpsview::Leaving>,
    /// pictures to drop from the GPU at the next paint
    pub stale_pictures: Vec<Arc<RenderImage>>,
}

impl Burrow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Burrow {
        let hub = burrow_core::hub_from_env();
        hub.start();
        // the office managers' curl: BURROW_PORT if given, else a free port (her CLAUDE.md is told which)
        let port = std::env::var("BURROW_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(0);
        if let Err(e) = hub.serve_office(port) {
            eprintln!("burrow: the office managers' port: {e}");
        }
        let events = hub.subscribe();
        let state = hub.state();
        let inp = Input { state: state.clone(), ..Default::default() };
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        // a frame every sixtieth of a second: the room moves at twelve, a drag at once
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(16)).await;
                if this.update_in(cx, |b, window, cx| b.frame(window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        // ⌘Q in the first person: the mouse let go before the process ends
        cx.on_app_quit(|_, _| {
            crate::fpsview::unlock();
            async {}
        })
        .detach();
        // the window's focus: away a minute, you nap (and the polls slow down)
        cx.observe_window_activation(window, |b, window, cx| {
            let on = window.is_window_active();
            if !on && b.fps.is_some() {
                b.stop_fps(cx); // the pointer let go: out of the first person, as when the browser's lock was lost
            }
            if !on {
                b.outside.blur(); // a key held while the window went: let go
                b.pong.blur();
            }
            b.blur_at = if on { None } else { b.blur_at.or(Some(Instant::now())) };
            b.hub.focus(on);
        })
        .detach();
        // BURROW_OPEN (debug, with BURROW_SHOT): what to open once the state is in: bot:<id>, shell:<dir>, view:<kind>,
        // several split by commas
        if let Ok(what) = std::env::var("BURROW_OPEN") {
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(1500)).await;
                let _ = this.update_in(cx, |b, window, cx| {
                    for w in what.split(',') {
                        let (k, v) = w.split_once(':').unwrap_or((w, ""));
                        match k {
                            "bot" => {
                                let m = b.state.bots.iter().find(|x| x.id == v).map(|x| x.m.clone()).unwrap_or_default();
                                b.open_bot(&m, v, true, None, window, cx);
                            }
                            "shell" => b.open_shell("", v, true, window, cx),
                            "notes" => b.open_notes(true, window, cx),
                            "fps" => {
                                // the demo in hardcore, at the gun closet
                                b.hub.settings(&[("hardcore", true)]);
                                b.state.settings.hardcore = true;
                                if let Some(k) = b.room.last.as_ref().and_then(|l| l.booths.first()).map(|p| p.key.clone()) {
                                    b.start_fps(&k, window, cx);
                                }
                            }
                            "view" => {
                                if let Some(kind) = ViewKind::parse(v) {
                                    b.open_view(kind, window, cx);
                                }
                            }
                            _ => {}
                        }
                    }
                });
            })
            .detach();
        }
        // BURROW_CLICKS (debug): clicks in the room, in dots, "x,y;x,y", one every 700 ms after two seconds
        if let Ok(clicks) = std::env::var("BURROW_CLICKS") {
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(2000)).await;
                let pt = |s: &str| s.split_once(',').and_then(|(x, y)| Some((x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?)));
                for c in clicks.split(';') {
                    if let Some((a, z)) = c.split_once('>') {
                        // a drag: pressed at a, moved there in steps, let go at z (held at z: "x,y>x,y!")
                        let hold = z.ends_with('!');
                        let (Some(a), Some(z)) = (pt(a), pt(z.trim_end_matches('!'))) else { continue };
                        let _ = this.update_in(cx, |b, window, cx| {
                            let mut out = Output::default();
                            b.room.pointer_move(a.0, a.1, (0.0, 0.0), &mut out);
                            b.redraw(window, cx);
                            b.room.pointer_down(a.0, a.1, false);
                            b.drag = Some(Drag::Room);
                        });
                        for k in 1..=10 {
                            let (x, y) = (a.0 + (z.0 - a.0) * k as f64 / 10.0, a.1 + (z.1 - a.1) * k as f64 / 10.0);
                            let _ = this.update_in(cx, |b, window, cx| {
                                let mut out = Output::default();
                                b.room.pointer_move(x, y, (0.0, 0.0), &mut out);
                                b.after(out, window, cx);
                                b.dirty = true;
                            });
                            cx.background_executor().timer(Duration::from_millis(60)).await;
                        }
                        if !hold {
                            let _ = this.update_in(cx, |b, window, cx| {
                                let mut out = Output::default();
                                let inp = b.input();
                                b.room.pointer_up(&inp, &mut out, (0.0, 0.0));
                                b.drag = None;
                                b.after(out, window, cx);
                                b.dirty = true;
                            });
                        }
                    } else if let Some((x, y)) = pt(c) {
                        let _ = this.update_in(cx, |b, window, cx| b.click_room(x, y, window, cx));
                    }
                    cx.background_executor().timer(Duration::from_millis(700)).await;
                }
            })
            .detach();
        }
        let now = Instant::now();
        Burrow {
            hub,
            events,
            state,
            digest: Default::default(),
            room: Room::new(seed, 0.0),
            inp,
            picture: None,
            stale: Vec::new(),
            room_bounds: Rc::new(Cell::new(None)),
            pane_bounds: Rc::new(Cell::new(None)),
            started: now,
            last: now,
            dirty: true,
            tiles: Tiles::default(),
            tile: HashMap::new(),
            next: 1,
            view: None,
            views: views::Views::default(),
            drag: None,
            hover_tile: None,
            lit_tiles: Vec::new(),
            lit_shell: None,
            blur_at: None,
            scale: 2.0,
            focus: cx.focus_handle(),
            sound: burrow_sound::Sound::new(),
            outside: burrow_games::outside::Outside::new(seed),
            pong: burrow_games::pong::Pong::new(seed ^ 0x9e37),
            fps: None,
            leaving: None,
            stale_pictures: Vec::new(),
        }
    }

    pub fn now_ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    /// the backend's news, the clock, the room's tick
    fn frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        let mut news = false;
        while let Ok(e) = self.events.try_recv() {
            news = true;
            match e {
                Event::State(s) => self.state = *s,
                Event::Digest(d) => self.digest = d,
                Event::Shells(sh) => self.state.shells = sh,
                Event::Talk { from, to } => {
                    let inp = self.input();
                    let mut out = Output::default();
                    self.room.talk(&inp, &mut out, &from, &to);
                    self.after(out, window, cx);
                }
            }
        }
        if news {
            self.on_state(window, cx);
        }
        // a terminal whose program ended (or whose session went to agent view) closes its tile
        let done: Vec<u64> = self.tile.iter().filter_map(|(k, t)| t.body.as_ref().filter(|b| b.update(cx, |b, _| b.done())).map(|_| *k)).collect();
        for t in done {
            self.close_tile(t, window, cx);
        }
        self.tick_games(dt, cx);
        let ticked = self.room.advance(dt);
        if ticked || self.dirty || news {
            self.redraw(window, cx);
            cx.notify();
        }
        if self.fps.is_some() {
            self.fps_frame(dt, ticked, window, cx);
        } else if self.leaving.is_some() {
            let now = self.now_ms();
            self.leaving_frame(now, window, cx);
        }
    }

    /// the room's view of the app: what is open, focused, lit, the clock
    pub fn input(&self) -> Input {
        let lt = local_time();
        let focused = self.tiles.focused().and_then(|t| self.tile.get(&t));
        let open = focused.and_then(|t| match &t.kind {
            TileKind::Bot { id, .. } => Some(self.as_folk(id)),
            _ => None,
        });
        let opens = self.tile.values().filter_map(|t| match &t.kind {
            TileKind::Bot { id, .. } => Some(self.as_folk(id)),
            _ => None,
        });
        let shell = focused.and_then(|t| match &t.kind {
            TileKind::Shell { m, dir } => Some(ShellKey { m: m.clone(), dir: dir.clone() }),
            _ => None,
        });
        let hl = self.hover_tile.and_then(|t| self.tile.get(&t)).and_then(|t| match &t.kind {
            TileKind::Bot { id, .. } => Some(self.as_folk(id)),
            TileKind::Notes => self.linda_folk(),
            _ => None,
        });
        let hl_shell = self.hover_tile.and_then(|t| self.tile.get(&t)).and_then(|t| match &t.kind {
            TileKind::Shell { m, dir } => Some(ShellKey { m: m.clone(), dir: dir.clone() }),
            _ => None,
        });
        let chess = (self.view == Some(ViewKind::Chess)).then(|| self.state.chess.clone()).flatten().filter(|c| c.result.is_none());
        let ids = |x: &Option<[String; 2]>| x.as_ref().map(|v| v.to_vec()).unwrap_or_default();
        let pong = (self.view == Some(ViewKind::Pong))
            .then(|| self.pong.playing())
            .flatten()
            .map(|p| burrow_room::io::PongInfo { id: Some(p.id.clone()), watch: p.watch, ball: p.ball, ids: ids(&p.ids) });
        // (the duel keeps its own clock: its shots, on the app's)
        let now_s = self.now_ms() / 1000.0;
        let o_now = self.outside.now();
        let outside = (self.view == Some(ViewKind::Outside)).then(|| self.outside.playing()).flatten().map(|p| burrow_room::io::OutsideInfo {
            id: Some(p.id.clone()),
            watch: p.watch,
            at: p.at.clone().unwrap_or_default(),
            ids: ids(&p.ids),
            fired: [p.fired[0] - o_now + now_s, p.fired[1] - o_now + now_s],
        });
        let shells = self.state.shells.iter().map(|s| ShellInfo { m: s.m.clone(), dir: s.dir.clone(), busy: s.busy }).collect();
        let digest = DigestInfo {
            on: self.digest.on,
            writing: self.digest.writing.clone(),
            explaining: self.digest.explaining.clone(),
            sessions: self.digest.sessions.keys().cloned().collect(),
        };
        Input {
            state: self.state.clone(),
            now: self.now_ms(),
            hour: lt.0,
            minute: lt.1,
            epoch_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
            open,
            opens: opens.collect(),
            chess,
            pong,
            outside,
            view: self.view.map(|v| v.name().to_string()),
            digest: Some(digest),
            away: self.blur_at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0),
            hl,
            shell,
            shells,
            hl_shell,
            still: reduce_motion(),
            fps: self.fps.is_some(),
        }
    }

    /// an office manager's session id as the room knows her: mgr:<her booth>
    pub fn as_folk(&self, id: &str) -> String {
        match self.state.managers.iter().find(|m| m.id == id && !id.is_empty()) {
            Some(m) => format!("mgr:{}", m.key),
            None => id.to_string(),
        }
    }
    fn linda_folk(&self) -> Option<String> {
        self.state.managers.iter().find(|m| m.style == "notes").map(|m| format!("mgr:{}", m.key))
    }

    /// step, draw, and turn the buffer into a picture for the GPU
    fn redraw(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dirty = false;
        self.inp = self.input();
        let mut out = Output::default();
        let l = self.room.step(&self.inp, &mut out);
        self.room.draw(&self.inp, &mut out, &l);
        let k = self.scale.round().max(1.0) as u32;
        if std::env::var_os("BURROW_DEBUG").is_some() {
            eprintln!("redraw {}x{} k={k} bots={} folk={}", self.room.img.w, self.room.img.h, self.state.bots.len(), self.room.w.folk.len());
        }
        let bytes = self.room.img.upscale_bgra(k, [0x0b, 0x08, 0x12, 255]);
        let (w, h) = (self.room.img.w * k, self.room.img.h * k);
        if let Some(buf) = image::RgbaImage::from_raw(w, h, bytes)
            && let Some(old) = self.picture.replace(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
        {
            self.stale.push(old);
        }
        self.after(out, window, cx);
    }

    /// what the room asked for: actions, sounds, the tile under a carried bot
    pub fn after(&mut self, mut out: Output, window: &mut Window, cx: &mut Context<Self>) {
        self.sound.set(out.sound_on);
        for s in &out.sounds {
            self.sound.play(s.name, s.pan, s.vol, false, None);
        }
        if let Some(h) = out.hover {
            self.lit_tiles = h.map(|id| self.tile.iter().filter(|(_, t)| matches!(&t.kind, TileKind::Bot { id: b, .. } if self.as_folk(b) == id)).map(|(k, _)| *k).collect()).unwrap_or_default();
            cx.notify();
        }
        if let Some(hs) = out.hover_shell {
            self.lit_shell = hs;
            cx.notify();
        }
        if let Some(key) = out.first_person.take() {
            self.start_fps(&key, window, cx);
        }
        for a in out.acts {
            self.act(a, window, cx);
        }
    }

    /// the room's `act`: what a click or a drop in the room asks for
    pub fn act(&mut self, a: Value, window: &mut Window, cx: &mut Context<Self>) {
        let kind = a["kind"].as_str().unwrap_or_default().to_string();
        if std::env::var_os("BURROW_DEBUG").is_some() {
            eprintln!("act {a}");
        }
        let s = |k: &str| a[k].as_str().unwrap_or_default().to_string();
        let shift = a["shift"].as_bool().unwrap_or(false);
        let hub = self.hub.clone();
        match kind.as_str() {
            "addDir" => self.open_view(ViewKind::Dir, window, cx),
            "you" => self.open_view(ViewKind::You, window, cx),
            "closet" => self.open_view(ViewKind::Closet, window, cx),
            "report" => self.open_view(ViewKind::Report, window, cx),
            "board" => self.open_view(ViewKind::Board, window, cx),
            "limits" => self.open_view(ViewKind::Limits, window, cx),
            "settings" => self.open_view(ViewKind::Settings, window, cx),
            "doors" => self.open_view(ViewKind::Doors, window, cx),
            "clock" => self.open_view(ViewKind::Clock, window, cx),
            "outside" => {
                self.views.outside_key = Some(s("key"));
                self.open_view(ViewKind::Outside, window, cx)
            }
            "pairOpen" => {
                self.views.pair_key = Some(s("key"));
                self.open_view(ViewKind::Pair, window, cx)
            }
            "desk" => {
                self.views.desk_unit = Some(s("unit"));
                self.open_view(ViewKind::Desk, window, cx)
            }
            "chess" => self.open_game(ViewKind::Chess, window, cx),
            "pong" => self.open_game(ViewKind::Pong, window, cx),
            "notes" => self.open_notes(shift, window, cx),
            "manager" => self.open_manager(s("key"), shift, window, cx),
            "sound" => {
                // the radio: the room's sound on or off (it says so as it goes)
                let on = !self.state.settings.sound;
                self.sound.play(if on { "tune" } else { "click" }, a["pan"].as_f64().unwrap_or(0.0), 1.0, true, None);
                hub.settings(&[("sound", on)]);
            }
            "kept" => {
                let _ = hub.office_do(&json!({"kept": s("id")}));
            }
            "pair" => {
                let (key, id) = (s("key"), s("id"));
                self.join_pair(key, id, window, cx);
            }
            "burn" => {
                let key = s("key");
                self.background(move || hub.remove_furniture(&key).map(|_| Value::Null), cx)
            }
            "unmachine" => {
                let m = s("m");
                self.background(move || hub.unmachine(&m).map(|_| Value::Null), cx)
            }
            "open" => {
                let id = s("id");
                if let Some(b) = self.state.bots.iter().find(|b| b.id == id).cloned() {
                    self.open_bot(&b.m, &b.id, shift, None, window, cx);
                }
            }
            "shell" => self.open_shell(&s("m"), &s("dir"), shift, window, cx),
            "chessBots" => {
                let (x, y) = (s("a"), s("b"));
                let _ = hub.chess(burrow_core::ChessAction::Bots(x, y));
            }
            "chessLeave" => {
                let _ = hub.chess(burrow_core::ChessAction::Leave(s("id")));
            }
            "forget" => hub.forget(&s("unit")),
            "layout" => {
                let lay: Vec<(String, [f64; 2])> = a["layout"]
                    .as_object()
                    .map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), [v[0].as_f64()?, v[1].as_f64()?]))).collect())
                    .unwrap_or_default();
                hub.set_layout(&lay);
            }
            "stop" => {
                let id = s("id");
                let rm = a["rm"].as_bool().unwrap_or(false);
                self.close_bot(&id, window, cx);
                if let Some(b) = self.state.bots.iter().find(|b| b.id == id).cloned() {
                    self.background(move || hub.stop(&b.m, &b.id, rm).map(|_| Value::Null), cx);
                }
            }
            "managerGone" => {
                let _ = hub.manager_gone(&s("key"));
            }
            "close" => self.close_bot(&s("id"), window, cx),
            "clear" => {
                if let Some(b) = self.state.bots.iter().find(|b| b.id == s("id")) {
                    let _ = hub.clear(&b.m, &b.id);
                }
            }
            "blank" => {
                for t in self.tiles.all() {
                    self.close_tile(t, window, cx);
                }
                self.close_view(window, cx);
            }
            "spawn" => {
                let (m, dir) = (s("m"), s("dir"));
                let task = cx.background_executor().spawn(async move { hub.spawn(&m, &dir, "").map(|id| (m, id)) });
                cx.spawn_in(window, async move |this, cx| {
                    if let Ok((m, id)) = task.await {
                        let _ = this.update_in(cx, |b, window, cx| {
                            b.room.fresh(&id); // just started: you both go to its desk
                            b.open_bot(&m, &id, shift, None, window, cx);
                        });
                    }
                })
                .detach();
            }
            _ => {}
        }
        cx.notify();
    }

    /// a call that may take a while (ssh, claude): off the main thread; trouble goes to the console, never the room
    pub fn background(&self, f: impl FnOnce() -> Result<Value, String> + Send + 'static, cx: &mut Context<Self>) {
        let task = cx.background_executor().spawn(async move { f() });
        cx.spawn(async move |_, _| {
            if let Err(e) = task.await {
                eprintln!("burrow: {e}");
            }
        })
        .detach();
    }

    fn on_state(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // a bot that gets work, or a question for you, leaves the ping-pong table, and the duel at the window
        let idle = |id: &str| self.state.bots.iter().any(|b| b.id == id && !b.busy && !b.waiting);
        if self.view == Some(ViewKind::Pong)
            && let Some(p) = self.pong.playing()
            && let Some(gone) = p.ids.iter().flatten().chain(std::iter::once(&p.id)).find(|id| !idle(id)).cloned()
        {
            self.pong.left(&gone);
        }
        if self.view == Some(ViewKind::Outside)
            && let Some(p) = self.outside.playing()
            && let Some(gone) = p.ids.iter().flatten().chain(std::iter::once(&p.id)).find(|id| !idle(id)).cloned()
        {
            self.outside.left(&gone);
        }
        cx.notify();
    }

    fn tick_games(&mut self, dt: f64, cx: &mut Context<Self>) {
        match self.view {
            Some(ViewKind::Pong) => {
                self.pong.tick(dt);
                cx.notify();
            }
            Some(ViewKind::Outside) => {
                self.outside.tick(dt);
                for e in self.outside.events() {
                    self.play_event(e);
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn play_event(&mut self, e: burrow_games::Event) {
        match e {
            burrow_games::Event::Sound { name, pan, dur, id } => {
                if let Some(h) = self.sound.play(name, pan, 1.0, false, dur) {
                    self.views.sounds.insert(id, h);
                }
            }
            burrow_games::Event::Stop(id) => {
                if let Some(h) = self.views.sounds.remove(&id) {
                    self.sound.stop(h);
                }
            }
        }
    }

    // ---- tiles

    fn new_tile(&mut self, kind: TileKind) -> u64 {
        let id = self.next;
        self.next += 1;
        self.tile.insert(id, Tile { kind, body: None, notes: None });
        id
    }

    /// a click replaces the focused tile, unless shift is held (then it splits it)
    fn make_room(&mut self, retain: bool, window: &mut Window, cx: &mut Context<Self>) {
        if retain {
            return;
        }
        if let Some(t) = self.tiles.focused() {
            self.close_tile(t, window, cx);
        }
    }

    pub fn tile_of_bot(&self, id: &str) -> Option<u64> {
        self.tile.iter().find(|(_, t)| matches!(&t.kind, TileKind::Bot { id: b, .. } if b == id)).map(|(k, _)| *k)
    }

    pub fn open_bot(&mut self, m: &str, id: &str, retain: bool, at: Option<Target>, window: &mut Window, cx: &mut Context<Self>) {
        self.close_view(window, cx);
        if let Some(t) = self.tile_of_bot(id) {
            self.tiles.focus(t);
            self.focus_tile(t, window, cx);
            return;
        }
        if at.is_none() {
            self.make_room(retain, window, cx);
        }
        let t = self.new_tile(TileKind::Bot { m: m.to_string(), id: id.to_string() });
        let body = crate::term::TermTile::bot(self.hub.clone(), m, id, window, cx);
        // what you type there: a bubble over your head in the room, the bot answers when you pause
        let folk = self.as_folk(id);
        cx.subscribe(&body, move |this, _, ev: &crate::term::TileEvent, _| {
            let crate::term::TileEvent::Typed { enter } = ev;
            let now = this.now_ms();
            this.room.typed(now, Some(&folk), *enter);
            this.dirty = true;
        })
        .detach();
        self.tile.get_mut(&t).unwrap().body = Some(body);
        match at {
            Some(at) => {
                if let Some(was) = self.tiles.add_at(t, &at) {
                    self.drop_tile(was);
                }
            }
            None => self.tiles.add(t),
        }
        self.focus_tile(t, window, cx);
    }

    pub fn open_shell(&mut self, m: &str, dir: &str, retain: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.close_view(window, cx);
        if let Some((t, _)) = self.tile.iter().find(|(_, t)| matches!(&t.kind, TileKind::Shell { m: a, dir: d } if a == m && d == dir)) {
            let t = *t;
            self.tiles.focus(t);
            self.focus_tile(t, window, cx);
            return;
        }
        self.make_room(retain, window, cx);
        let t = self.new_tile(TileKind::Shell { m: m.to_string(), dir: dir.to_string() });
        let body = crate::term::TermTile::shell(self.hub.clone(), m, dir, window, cx);
        cx.subscribe(&body, move |this, _, ev: &crate::term::TileEvent, _| {
            let crate::term::TileEvent::Typed { enter } = ev;
            let now = this.now_ms();
            this.room.typed(now, None, *enter);
            this.dirty = true;
        })
        .detach();
        self.tile.get_mut(&t).unwrap().body = Some(body);
        self.tiles.add(t);
        self.focus_tile(t, window, cx);
    }

    fn open_notes(&mut self, retain: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.close_view(window, cx);
        if let Some((t, _)) = self.tile.iter().find(|(_, t)| matches!(t.kind, TileKind::Notes)) {
            self.tiles.focus(*t);
            return;
        }
        self.make_room(retain, window, cx);
        let t = self.new_tile(TileKind::Notes);
        let me = cx.entity().downgrade();
        let notes = cx.new(|cx| crate::notes::Notes::new(self.hub.clone(), me, cx));
        self.tile.get_mut(&t).unwrap().notes = Some(notes);
        self.tiles.add(t);
        self.hub.digest_refresh();
    }

    /// an office manager's booth: her terminal (Susan, Joan), the notes (Linda), or hiring her again
    fn open_manager(&mut self, key: String, retain: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(m) = self.state.managers.iter().find(|m| m.key == key).cloned() else { return };
        if m.style == "notes" && !m.vacant {
            return self.open_notes(retain, window, cx);
        }
        let hub = self.hub.clone();
        let task = cx.background_executor().spawn(async move { hub.manager_open(&key) });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok((id, err)) = task.await {
                let _ = this.update_in(cx, |b, window, cx| {
                    if !id.is_empty() {
                        b.open_bot("", &id, retain, None, window, cx);
                    } else if !err.is_empty() {
                        eprintln!("burrow: {err}");
                    }
                });
            }
        })
        .detach();
    }

    /// a bot dropped on a pair table joins its chat: its session opened, the bring link typed in
    fn join_pair(&mut self, key: String, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let hub = self.hub.clone();
        let Some(b) = self.state.bots.iter().find(|b| b.id == id).cloned() else { return };
        self.open_bot(&b.m, &b.id, false, None, window, cx);
        let task = cx.background_executor().spawn(async move { hub.pair(&key, None, None) });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(info) = task.await
                && !info.bring.is_empty()
            {
                let bring = info.bring;
                let _ = this.update_in(cx, |b, _, cx| {
                    if let Some(t) = b.tile_of_bot(&id)
                        && let Some(body) = b.tile.get(&t).and_then(|t| t.body.clone())
                    {
                        body.update(cx, |tt, cx| tt.hand(format!("{bring}\r"), cx));
                    }
                });
            }
        })
        .detach();
    }

    fn focus_tile(&mut self, t: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.tiles.focus(t);
        if let Some(body) = self.tile.get(&t).and_then(|x| x.body.clone()) {
            body.update(cx, |b, cx| b.focus(window, cx));
        }
        self.dirty = true;
    }

    fn drop_tile(&mut self, t: u64) {
        self.tile.remove(&t);
    }

    pub fn close_tile(&mut self, t: u64, _window: &mut Window, cx: &mut Context<Self>) {
        self.tiles.remove(t);
        self.tile.remove(&t);
        if self.hover_tile == Some(t) {
            self.hover_tile = None;
        }
        self.dirty = true;
        cx.notify();
    }

    fn close_bot(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let ts: Vec<u64> = self.tile.iter().filter(|(_, t)| matches!(&t.kind, TileKind::Bot { id: b, .. } if b == id)).map(|(k, _)| *k).collect();
        for t in ts {
            self.close_tile(t, window, cx);
        }
    }

    // ---- the view

    pub fn open_view(&mut self, kind: ViewKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.view.is_some() && self.view != Some(kind) {
            self.close_view_keep(window, cx, true);
        }
        for t in self.tiles.all() {
            self.close_tile(t, window, cx);
        }
        self.view = Some(kind);
        views::opened(self, kind, window, cx);
        self.dirty = true;
        cx.notify();
    }

    fn open_game(&mut self, kind: ViewKind, window: &mut Window, cx: &mut Context<Self>) {
        self.open_view(kind, window, cx);
    }

    pub fn close_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_view_keep(window, cx, false);
    }

    fn close_view_keep(&mut self, window: &mut Window, cx: &mut Context<Self>, _keep: bool) {
        let Some(was) = self.view.take() else { return };
        views::closed(self, was, window, cx);
        self.dirty = true;
        cx.notify();
    }

    /// a piece pressed in the closet: carried while the pointer moves (panels::closet)
    pub fn closet_carry(&mut self, kind: &str, at: Point<Pixels>) {
        self.drag = Some(Drag::Closet { kind: kind.to_string(), x0: f32::from(at.x), y0: f32::from(at.y), on: false });
    }
    pub fn closet_carrying(&self) -> Option<&str> {
        match &self.drag {
            Some(Drag::Closet { kind, on: true, .. }) => Some(kind),
            _ => None,
        }
    }

    fn dots(&self, p: Point<Pixels>) -> Option<(f64, f64)> {
        let b = self.room_bounds.get()?;
        Some((f64::from(p.x - b.origin.x) / K as f64, f64::from(p.y - b.origin.y) / K as f64))
    }

    /// pane coordinates of a window point
    fn in_pane(&self, p: Point<Pixels>) -> Option<(f32, f32, f32, f32)> {
        let b = self.pane_bounds.get()?;
        Some((f32::from(p.x - b.origin.x), f32::from(p.y - b.origin.y), f32::from(b.size.width), f32::from(b.size.height)))
    }

    // ---- the pointer, all over the window while something is dragged

    fn pointer_move(&mut self, e: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        match self.drag.clone() {
            Some(Drag::Divider { w0, x0 }) => {
                let w = self.room.set_width(w0 + (f32::from(e.position.x) - x0) as f64);
                let _ = w;
                self.dirty = true;
            }
            Some(Drag::Gap { gap, sizes, at0 }) => {
                let d = if gap.dir == tiles::Dir::Row { f32::from(e.position.x) - at0 } else { f32::from(e.position.y) - at0 };
                self.tiles.drag_gap(&gap, &sizes, d);
            }
            Some(Drag::Tile { id, x0, y0, on, .. }) => {
                let (x, y) = (f32::from(e.position.x), f32::from(e.position.y));
                if on || (x - x0).hypot(y - y0) >= 4.0 {
                    let target = self.in_pane(e.position).and_then(|(px, py, pw, ph)| self.tiles.preview(px, py, pw, ph, Some(id)));
                    self.drag = Some(Drag::Tile { id, x0, y0, on: true, target });
                }
            }
            Some(Drag::Closet { kind, x0, y0, on }) => {
                let (x, y) = (f32::from(e.position.x), f32::from(e.position.y));
                if on || (x - x0).hypot(y - y0) >= 4.0 {
                    self.drag = Some(Drag::Closet { kind: kind.clone(), x0, y0, on: true });
                    if let Some((dx, dy)) = self.dots(e.position) {
                        self.room.carry_kind(Some(&kind), dx, dy);
                    }
                    self.dirty = true;
                }
            }
            Some(Drag::Room) | None => {
                if let Some((x, y)) = self.dots(e.position) {
                    let mut out = Output::default();
                    self.room.pointer_move(x, y, (f64::from(e.position.x), f64::from(e.position.y)), &mut out);
                    if let Some(c) = out.carry.take() {
                        // a bot carried out over the pane: where it would open
                        self.views.carry_target = c.and_then(|(_, cx_, cy)| {
                            self.in_pane(Point { x: px(cx_ as f32), y: px(cy as f32) }).and_then(|(px_, py, pw, ph)| self.tiles.preview(px_, py, pw, ph, None))
                        });
                    }
                    self.after(out, window, cx);
                    self.dirty = true;
                }
            }
        }
        cx.notify();
    }

    fn pointer_up(&mut self, e: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        match self.drag.take() {
            Some(Drag::Divider { .. }) => self.hub.set_width(self.room.size.w as f64),
            Some(Drag::Tile { id, on: true, target: Some(t), .. }) => self.tiles.move_to(id, &t),
            Some(Drag::Closet { kind, on: true, .. }) => {
                let at = self.dots(e.position).and_then(|(x, y)| self.room.place_kind(&kind, x, y));
                self.room.carrying = None;
                self.dirty = true;
                crate::panels::closet::put(self, &kind, at, window, cx);
            }
            Some(Drag::Room) => {
                let mut out = Output::default();
                let inp = self.input();
                self.room.pointer_up(&inp, &mut out, (f64::from(e.position.x), f64::from(e.position.y)));
                if let Some((id, _, _)) = out.carried.take() {
                    // let go over the pane: its session opens there
                    let at = self.views.carry_target.take();
                    if let Some(b) = self.state.bots.iter().find(|b| b.id == id).cloned() {
                        self.open_bot(&b.m, &b.id, false, at, window, cx);
                    }
                }
                self.views.carry_target = None;
                self.after(out, window, cx);
                self.dirty = true;
            }
            _ => {}
        }
        cx.notify();
    }

    /// files dropped from the Finder: on a bot, into its session; on a desk's screen, into your terminal there; on a
    /// tile, into its terminal. A path on another machine is copied there first (scp)
    fn drop_on_room(&mut self, paths: Vec<std::path::PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let Some((x, y)) = self.dots(window.mouse_position()) else { return };
        let mut out = Output::default();
        self.room.pointer_move(x, y, (0.0, 0.0), &mut out);
        self.redraw(window, cx);
        if let Some(h) = self.room.hovered.clone().filter(|h| !h.id.starts_with("mgr:") && h.id != "you")
            && let Some(b) = self.state.bots.iter().find(|b| b.id == h.id).cloned()
        {
            self.open_bot(&b.m, &b.id, false, None, window, cx);
            if let Some(t) = self.tile_of_bot(&b.id) {
                self.hand(t, paths, cx);
            }
        } else if let Some((m, dir)) = self.room.screen_hover.clone() {
            self.open_shell(&m, &dir, false, window, cx);
            if let Some(t) = self.tiles.focused() {
                self.hand(t, paths, cx);
            }
        }
    }

    /// files handed to a tile's terminal: their paths pasted (quoted when they need it), copied to its machine first
    pub fn hand(&mut self, t: u64, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        let Some(tile) = self.tile.get(&t) else { return };
        let Some(body) = tile.body.clone() else { return };
        let m = match &tile.kind {
            TileKind::Bot { m, .. } | TileKind::Shell { m, .. } => m.clone(),
            TileKind::Notes => return,
        };
        let hub = self.hub.clone();
        let task = cx.background_executor().spawn(async move {
            let mut out = Vec::new();
            for p in paths {
                if m.is_empty() {
                    out.push(p.to_string_lossy().to_string());
                } else if let Ok(bytes) = std::fs::read(&p) {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    match hub.upload(&m, &name, &bytes) {
                        Ok(there) => out.push(there),
                        Err(e) => eprintln!("burrow: {e}"),
                    }
                }
            }
            out
        });
        cx.spawn(async move |_, cx| {
            let ps = task.await;
            let text: String = ps.iter().map(|p| if p.contains([' ', '\'', '"', '\\', '\t', '\n']) { format!("'{}' ", p.replace('\'', "'\\''")) } else { format!("{p} ") }).collect();
            let _ = body.update(cx, |b, cx| b.hand(text, cx));
        })
        .detach();
    }

    /// a click in the room at x, y dots, as the pointer would make it (for BURROW_CLICKS)
    pub fn click_room(&mut self, x: f64, y: f64, window: &mut Window, cx: &mut Context<Self>) {
        let mut out = Output::default();
        self.room.pointer_move(x, y, (0.0, 0.0), &mut out);
        self.redraw(window, cx);
        self.room.pointer_down(x, y, false);
        let inp = self.input();
        self.room.pointer_up(&inp, &mut out, (0.0, 0.0));
        self.after(out, window, cx);
        self.dirty = true;
        cx.notify();
    }

    fn room_element(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pic = self.picture.clone();
        let bounds = self.room_bounds.clone();
        let (rw, rh) = (self.room.size.w as f32 * K as f32, self.room.size.h as f32 * K as f32);
        let cursor = match self.room.cursor {
            "pointer" => CursorStyle::PointingHand,
            "grabbing" => CursorStyle::ClosedHand,
            "grab" => CursorStyle::OpenHand,
            _ => CursorStyle::Arrow,
        };
        div()
            .id("room")
            .w(px(rw))
            .h(px(rh))
            .cursor(cursor)
            .child(
                canvas(
                    move |b, _, _| bounds.set(Some(b)),
                    move |b, _, window, _| {
                        if let Some(pic) = pic {
                            let r = window.paint_image(b, Corners::default(), pic, 0, false);
                            if std::env::var_os("BURROW_DEBUG").is_some() {
                                eprintln!("paint room {:?} -> {:?}", b.size, r.is_ok());
                            }
                        } else if std::env::var_os("BURROW_DEBUG").is_some() {
                            eprintln!("paint room: no picture");
                        }
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    if let Some((x, y)) = this.dots(e.position) {
                        this.room.pointer_down(x, y, e.modifiers.shift);
                        this.drag = Some(Drag::Room);
                        this.dirty = true;
                        window.focus(&this.focus);
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, window, cx| {
                if this.drag.is_none() {
                    this.pointer_move(e, window, cx);
                }
            }))
            .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, window, cx| this.drop_on_room(paths.paths().to_vec(), window, cx)))
            .on_hover(cx.listener(|this, on: &bool, _, _| {
                if !*on && this.drag.is_none() {
                    this.room.pointer_leave();
                    this.dirty = true;
                }
            }))
            .into_any_element()
    }

    fn tile_element(&mut self, t: u64, r: tiles::Rect, cx: &mut Context<Self>) -> AnyElement {
        let Some(tile) = self.tile.get(&t) else { return div().into_any_element() };
        let lit = self.tiles.focused() == Some(t) || self.lit_tiles.contains(&t) || self.hover_tile == Some(t) || matches!(&tile.kind, TileKind::Shell { m, dir } if self.lit_shell.as_deref() == Some(&format!("{m}:{dir}")));
        let (name, sub, rc) = match &tile.kind {
            TileKind::Bot { m, id } => {
                let on = self.state.machine(m).map(|x| format!("on {}", x.name)).unwrap_or_default();
                match self.state.managers.iter().find(|x| x.id == *id && !id.is_empty()) {
                    Some(mg) => (mg.name.clone(), "office manager".to_string(), mg.rc.clone()),
                    None => {
                        let b = self.state.bots.iter().find(|b| b.id == *id);
                        (b.map(|b| b.name.clone()).unwrap_or(id.clone()), on, b.map(|b| b.rc.clone()).unwrap_or_default())
                    }
                }
            }
            TileKind::Shell { m, dir } => {
                let on = self.state.machine(m).map(|x| format!("on {}", x.name)).unwrap_or_default();
                (dir.trim_end_matches('/').rsplit('/').next().unwrap_or(dir).to_string(), on, String::new())
            }
            TileKind::Notes => ("notes".to_string(), if self.digest.writing.is_some() { "writing…".into() } else if !self.digest.on { "switched off".into() } else { String::new() }, String::new()),
        };
        let body: AnyElement = match (&tile.body, &tile.notes) {
            (Some(b), _) => b.clone().into_any_element(),
            (_, Some(n)) => n.clone().into_any_element(),
            _ => div().into_any_element(),
        };
        let ink = if lit { col::INK } else { col::STEEL4 };
        let band = div()
            .id(("band", t))
            .h(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.))
            .pl(px(4.))
            .pr(px(8.))
            .cursor(CursorStyle::OpenHand)
            .child(kit::button(("tx", t), "✕", false, true).on_click(cx.listener(move |this, _, window, cx| this.close_tile(t, window, cx))))
            .child(pixel_text(name, ink, !lit).min_w(px(0.)).overflow_hidden())
            .child(pixel_text(sub, ink, !lit).flex_1().min_w(px(0.)).overflow_hidden().opacity(0.75))
            .when(!rc.is_empty(), |d| {
                let url = format!("https://claude.ai/code/session_{}", rc.trim_start_matches("cse_"));
                d.child(kit::button(("app", t), "APP", false, true).on_click(move |_, _, cx: &mut App| cx.open_url(&url)))
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    if this.tiles.all().len() > 1 {
                        this.drag = Some(Drag::Tile { id: t, x0: f32::from(e.position.x), y0: f32::from(e.position.y), on: false, target: None });
                    }
                    this.tiles.focus(t);
                    cx.notify();
                }),
            );
        div()
            .id(("tile", t))
            .absolute()
            .left(px(r.x))
            .top(px(r.y))
            .w(px(r.w))
            .h(px(r.h))
            .when(matches!(self.drag, Some(Drag::Tile { id, on: true, .. }) if id == t), |d| d.opacity(0.6))
            .child(frame_bg(Frame::tile(lit)))
            .child(div().absolute().inset(px(6.)).flex().flex_col().child(band).child(div().flex_1().min_h(px(0.)).bg(rgb(col::BOX)).child(body)))
            .on_drop(cx.listener(move |this, paths: &gpui::ExternalPaths, _, cx| this.hand(t, paths.paths().to_vec(), cx)))
            .on_hover(cx.listener(move |this, on: &bool, _, cx| {
                this.hover_tile = if *on { Some(t) } else if this.hover_tile == Some(t) { None } else { this.hover_tile };
                this.dirty = true;
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                if this.tiles.focused() != Some(t) {
                    this.focus_tile(t, window, cx);
                    cx.notify();
                }
            }))
            .into_any_element()
    }

    fn pane_element(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let bounds = self.pane_bounds.clone();
        if let Some(b) = self.pane_bounds.get() {
            self.tiles.layout(f32::from(b.size.width), f32::from(b.size.height));
        }
        let mut pane = div()
            .id("pane")
            .flex_1()
            .h_full()
            .relative()
            .overflow_hidden()
            .child(canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {}).absolute().size_full())
            .child(crate::stars::stars());
        let rects = self.tiles.rects.clone();
        for (t, r) in rects {
            pane = pane.child(self.tile_element(t, r, cx));
        }
        for (i, g) in self.tiles.gaps.clone().into_iter().enumerate() {
            let row = g.dir == tiles::Dir::Row;
            pane = pane.child(
                div()
                    .id(("gap", i))
                    .absolute()
                    .left(px(g.rect.x))
                    .top(px(g.rect.y))
                    .w(px(g.rect.w))
                    .h(px(g.rect.h))
                    .cursor(if row { CursorStyle::ResizeLeftRight } else { CursorStyle::ResizeUpDown })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, _, _| {
                            let sizes = this.tiles.sizes_of(&g);
                            let at0 = if row { f32::from(e.position.x) } else { f32::from(e.position.y) };
                            this.drag = Some(Drag::Gap { gap: g.clone(), sizes, at0 });
                        }),
                    ),
            );
        }
        // the drop zone of a tile dragged, or of a bot carried out of the room
        let zone = match &self.drag {
            Some(Drag::Tile { target: Some(t), .. }) => Some(t.zone),
            _ => self.views.carry_target.as_ref().map(|t| t.zone),
        };
        if let Some(z) = zone {
            pane = pane.child(div().absolute().left(px(z.x)).top(px(z.y)).w(px(z.w)).h(px(z.h)).bg(gpui::rgba(0xf2a53133)).border_2().border_color(rgb(col::GOLD1)));
        }
        if let Some(kind) = self.view {
            pane = pane.child(views::render(self, kind, window, cx));
        }
        pane.into_any_element()
    }
}

impl Focusable for Burrow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Burrow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.scale = window.scale_factor();
        // the room as tall as the window, as wide as the divider says
        let wh = f64::from(window.viewport_size().height);
        let (w0, h0) = (self.room.size.w, self.room.size.h);
        self.room.resize(&self.inp, wh);
        if (w0, h0) != (self.room.size.w, self.room.size.h) {
            self.dirty = true;
        }
        for old in self.stale.drain(..).chain(self.stale_pictures.drain(..)) {
            let _ = window.drop_image(old);
        }
        let room = self.room_element(cx);
        let pane = self.pane_element(window, cx);
        let dragging = self.drag.is_some();
        let this = cx.entity();
        div()
            .id("burrow")
            .track_focus(&self.focus)
            .size_full()
            .bg(rgb(col::NIGHT))
            .flex()
            .flex_row()
            .font_family(kit::MONO)
            .text_color(rgb(col::TEXT))
            .child(div().flex_none().h_full().pt(px(MARGIN as f32)).pb(px(MARGIN as f32)).pl(px(MARGIN as f32)).child(room))
            .child(
                div().id("divider").flex_none().w(px(6.)).h_full().cursor(CursorStyle::ResizeLeftRight).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, e: &MouseDownEvent, _, _| this.drag = Some(Drag::Divider { w0: (this.room.size.w * K) as f64, x0: f32::from(e.position.x) })),
                ),
            )
            .child(pane)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if !this.fps_key(e) {
                    views::key(this, e, window, cx)
                }
            }))
            .on_key_up(cx.listener(|this, e: &gpui::KeyUpEvent, _, _| {
                this.fps_key_up(e);
            }))
            .on_modifiers_changed(cx.listener(|this, e: &gpui::ModifiersChangedEvent, _, _| this.fps_modifiers(e)))
            .when_some(self.fps.as_ref().and_then(|r| r.picture.clone()).or(self.leaving.as_ref().and_then(|l| l.picture.clone())), |d, pic| {
                // the first person over everything (or its melt over the room, on the way out)
                let black = self.fps.is_some();
                d.child(
                    div()
                        .id("fps")
                        .absolute()
                        .inset_0()
                        .when(black, |d| d.bg(rgb(0x000000)))
                        .child(canvas(|_, _, _| {}, move |b, _, window, _| {
                            let _ = window.paint_image(b, Corners::default(), pic, 0, false);
                        }).size_full())
                        .when(black, |d| d.on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, _| this.fps_click()))),
                )
            })
            // while something is dragged, the pointer is followed over the whole window
            .when(dragging, |d| {
                let (a, b) = (this.clone(), this.clone());
                d.child(
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, _| {
                            let a = a.clone();
                            window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                                if phase == gpui::DispatchPhase::Capture {
                                    a.update(cx, |this, cx| this.pointer_move(e, window, cx));
                                }
                            });
                            let b = b.clone();
                            window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
                                if phase == gpui::DispatchPhase::Capture {
                                    b.update(cx, |this, cx| this.pointer_up(e, window, cx));
                                }
                            });
                        },
                    )
                    .absolute()
                    .size_0(),
                )
            })
    }
}

/// macOS's Reduce Motion (the page's prefers-reduced-motion): asked at most once a second
pub fn reduce_motion() -> bool {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static AT: AtomicU64 = AtomicU64::new(0);
    static ON: AtomicBool = AtomicBool::new(false);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if AT.swap(now, Ordering::Relaxed) != now {
        let on: bool = unsafe {
            use objc::{class, msg_send, sel, sel_impl};
            let ws: *mut objc::runtime::Object = msg_send![class!(NSWorkspace), sharedWorkspace];
            let v: objc::runtime::BOOL = msg_send![ws, accessibilityDisplayShouldReduceMotion];
            v != objc::runtime::NO
        };
        ON.store(on, Ordering::Relaxed);
    }
    ON.load(Ordering::Relaxed)
}

/// this computer's clock: hours and minutes, local time
pub fn local_time() -> (u32, u32) {
    // libc's localtime: the hour the window's sky, the clock and the dog go by
    unsafe {
        let t = libc_time();
        let mut tm: Tm = std::mem::zeroed();
        localtime_r(&t, &mut tm);
        (tm.tm_hour as u32, tm.tm_min as u32)
    }
}

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
    fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm;
}

fn libc_time() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[allow(dead_code)]
fn _unused(_: Side) {}
