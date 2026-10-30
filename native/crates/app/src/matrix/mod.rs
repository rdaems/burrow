//! burrow as the operator sees it (web/matrix.html, matrix.js, matrix.css): the same hub and the same sessions as the
//! room, another face, over the whole window in the room's place (a click on the operator's console; `room`, or Esc on
//! an empty line, brings the room back). No 16 bit: black, phosphor green, one monospace. Every session a lane of
//! falling code (rain.rs); the pointer on a lane says who it is, a click jacks in: the session's own terminal (`claude
//! attach`, the room's terminal to the letter, in green) on the right, EXIT or `exit` out again. Under it all the
//! operator's line (op.rs), with tab completion and history.

mod op;
mod rain;
mod words;

use crate::burrow::{Burrow, local_time, reduce_motion};
use crate::field::{Field, FieldEvent, Look};
use crate::kit;
use crate::term::{TermTile, TileEvent};
use burrow_term::Theme;
use gpui::{
    AnyElement, BoxShadow, Bounds, Context, CursorStyle, Entity, Focusable, FontFallbacks, FontFeatures, FontId, FontStyle, FontWeight, GlyphId, HighlightStyle,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point, StyledText, Subscription, TextRun, Window, canvas, div, fill, point,
    prelude::*, px, relative, rgb,
};
use op::Class;
use rain::{FONT, Glyph, Rain};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use words::{project_of, tilde, word_of};

// matrix.css's colours
const BG: u32 = 0x000300;
const G: u32 = 0x00ff41;
const G2: u32 = 0x00c032;
const G3: u32 = 0x008f1f;
const G4: u32 = 0x004d12;
const G5: u32 = 0x0a1f0f;
const HOT: u32 = 0xe9ffee;
const WARN: u32 = 0xff5f56;
/// the terminal's side
const JACK: u32 = 0x000a02;

/// what is jacked into: a session, or your shell somewhere
enum JackKind {
    Bot { m: String, id: String },
    Shell { m: String, dir: String },
}

struct Jack {
    kind: JackKind,
    body: Entity<TermTile>,
    at: Instant,
}

/// what the rain's canvas keeps between frames: where it is, the font's measure, each glyph shaped once
#[derive(Default)]
struct Painter {
    bounds: Cell<Option<Bounds<Pixels>>>,
    /// a column's width (the font's M, rounded up, and one) and the glyphs' ascent
    metrics: Cell<Option<(f32, f32)>>,
    glyphs: RefCell<HashMap<(char, bool), Option<(FontId, GlyphId)>>>,
}

pub struct Matrix {
    rain: Rain,
    field: Entity<Field>,
    /// what was printed at the operator's line, and when
    out: Vec<(String, Class, Instant)>,
    hint: String,
    hist: Vec<String>,
    hi: usize,
    /// a delete asked: whom, until when a yes counts
    confirm: Option<(String, Instant)>,
    jack: Option<Jack>,
    glyphs: Rc<Vec<Glyph>>,
    paint: Rc<Painter>,
    /// the pointer in the rain's box
    at: Option<(f32, f32)>,
    /// the box and the column the rain was measured for
    sized: (f32, f32, f32),
    last_draw: f64,
    last_texts: f64,
    last_sweep: f64,
    /// (BURROW_OPEN: the pointer to be put on this one's lane)
    point_at: Option<String>,
    _subs: Vec<Subscription>,
}

/// the rain's font: the terminal's, the katakana from the system's
fn font(bold: bool) -> gpui::Font {
    gpui::Font {
        family: kit::MONO.into(),
        features: FontFeatures::disable_ligatures(),
        fallbacks: Some(FontFallbacks::from_fonts(vec!["Hiragino Sans".into(), "Menlo".into()])),
        weight: if bold { FontWeight::BOLD } else { FontWeight::NORMAL },
        style: FontStyle::Normal,
    }
}

fn rgba(c: u32, a: f32) -> gpui::Hsla {
    gpui::Rgba { r: (c >> 16 & 255) as f32 / 255., g: (c >> 8 & 255) as f32 / 255., b: (c & 255) as f32 / 255., a }.into()
}

/// what was typed at the line before, kept in the cache (the page kept it in localStorage); the demo keeps none
fn history_file(b: &Burrow) -> Option<std::path::PathBuf> {
    (!b.hub.cfg.demo).then(|| b.hub.cfg.cache.join("matrix-history.json"))
}

impl Matrix {
    fn print(&mut self, text: impl Into<String>, class: Class) {
        self.out.push((text.into(), class, Instant::now()));
        if self.out.len() > 40 {
            self.out.remove(0);
        }
        // no higher than six tenths of the box: a line a row, a long one wrapped
        let (w, h) = self.paint.bounds.get().map(|b| (f32::from(b.size.width), f32::from(b.size.height))).unwrap_or((960., 600.));
        let per = (((w.min(960.) - 28.) / 7.8).floor() as usize).max(20);
        let rows = |o: &[(String, Class, Instant)]| o.iter().map(|(t, _, _)| t.chars().count().div_ceil(per).max(1)).sum::<usize>() as f32;
        while self.out.len() > 1 && rows(&self.out) * 18. + 12. > h * 0.6 {
            self.out.remove(0);
        }
    }
    fn lines(&mut self, ls: op::Lines) {
        for (t, c) in ls {
            self.print(t, c);
        }
    }
    fn jacked_bot(&self) -> Option<&str> {
        match self.jack.as_ref().map(|j| &j.kind) {
            Some(JackKind::Bot { id, .. }) => Some(id),
            _ => None,
        }
    }
}

impl Burrow {
    /// the operator's console clicked: the window jacks into the matrix (the tiles and the view close, as the page
    /// left the room for /matrix)
    pub fn open_matrix(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.matrix.is_some() {
            return;
        }
        for t in self.tiles.all() {
            self.close_tile(t, window, cx);
        }
        self.close_view(window, cx);
        let field = cx.new(|cx| Field::new(false, Look::Operator, window, cx));
        let sub = cx.subscribe(&field, |this, _, ev: &FieldEvent, cx| {
            if *ev == FieldEvent::Changed
                && let Some(m) = &mut this.matrix
            {
                m.hint.clear();
                cx.notify();
            }
        });
        let hist: Vec<String> = history_file(self).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        let mut m = Matrix {
            rain: Rain::new(seed),
            field: field.clone(),
            out: vec![],
            hint: String::new(),
            hi: hist.len(),
            hist,
            confirm: None,
            jack: None,
            glyphs: Rc::new(vec![]),
            paint: Rc::new(Painter::default()),
            at: None,
            sized: (0., 0., 0.),
            last_draw: 0.,
            last_texts: f64::NEG_INFINITY,
            last_sweep: 0.,
            point_at: None,
            _subs: vec![sub],
        };
        if m.hist.is_empty() {
            m.print("type help, or click a lane to jack in; esc to leave", Class::Echo);
        }
        self.matrix = Some(m);
        field.read(cx).focus(window);
        cx.notify();
    }

    /// back to the 16-bit room (`room`, Esc on an empty line): the terminal jacked into goes with it
    pub fn close_matrix(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.matrix = None;
        window.focus(&self.focus);
        cx.notify();
    }

    /// a message between two sessions while the matrix is up: a streak across it (the room is not drawn meanwhile)
    pub fn matrix_talk(&mut self, from: &str, to: &str) -> bool {
        let now = self.now_ms();
        let Some(m) = &mut self.matrix else { return false };
        m.rain.talk(from, to, now);
        true
    }

    /// a frame while it is up: the rain thirty times a second at most, the text it rains asked for, the printed
    /// lines going after a while, a terminal that ended let go
    pub fn matrix_frame(&mut self, news: bool, window: &mut Window, cx: &mut Context<Self>) {
        let now = self.now_ms();
        let done = self.matrix.as_ref().and_then(|m| m.jack.as_ref()).is_some_and(|j| j.body.update(cx, |b, _| b.done()));
        if done {
            self.jack_out(window, cx);
        }
        let field_focused = self.matrix.as_ref().is_some_and(|m| m.field.read(cx).focused(window));
        let Some(m) = &mut self.matrix else { return };
        if let (Some(b), Some((cw, _))) = (m.paint.bounds.get(), m.paint.metrics.get()) {
            let size = (f32::from(b.size.width), f32::from(b.size.height), cw);
            if size != m.sized {
                m.sized = size;
                m.rain.measure(size.0, size.1, cw, &self.state, now);
            } else if news {
                m.rain.arrange(&self.state, now);
            }
        }
        if let Some(q) = m.point_at.clone()
            && let Ok(b) = op::find(&q, &self.state)
            && let Some(l) = m.rain.lane(&b.id)
        {
            m.at = Some(((l.x as f32 + l.w as f32 / 2.) * m.rain.cw, m.sized.1 / 2.));
            m.rain.hovered = Some(l.id.clone());
            m.point_at = None;
        }
        if now - m.last_draw >= 32. {
            m.last_draw = now;
            m.glyphs = Rc::new(m.rain.step(&self.state, now, reduce_motion()));
            cx.notify();
        }
        // what was printed goes after a while, unless the line is in use
        if now - m.last_sweep > 2000. {
            m.last_sweep = now;
            if !field_focused {
                let n = m.out.len();
                m.out.retain(|(_, _, t)| t.elapsed() < Duration::from_secs(20));
                if m.out.len() != n {
                    cx.notify();
                }
            }
        }
        // the text the lanes rain: asked for the sessions whose record changed since (the hub keeps it, and asks a
        // machine at most every ten seconds)
        if now - m.last_texts > 4000. {
            m.last_texts = now;
            let ids = m.rain.want_texts(now);
            if !ids.is_empty() {
                crate::panels::call(
                    self,
                    move |hub| hub.stream(&ids),
                    |b, texts, _, _| {
                        if let Some(m) = &mut b.matrix {
                            m.rain.got_texts(&texts);
                        }
                    },
                    window,
                    cx,
                );
            }
        }
    }

    // ---- jacking in: the session's own terminal (claude attach), the room's terminal to the letter, in green

    fn jack_in(&mut self, m: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let now = self.now_ms();
        let Some(mx) = &mut self.matrix else { return };
        if mx.jacked_bot() == Some(id) {
            let body = mx.jack.as_ref().unwrap().body.clone();
            body.update(cx, |b, cx| b.focus(window, cx));
            return;
        }
        mx.rain.jacked(id, now);
        let body = TermTile::bot(self.hub.clone(), m, id, window, cx);
        let lane = id.to_string();
        let sub = cx.subscribe(&body, move |this, _, _: &TileEvent, _| {
            if let Some(m) = &mut this.matrix {
                m.rain.typed(&lane);
            }
        });
        self.jack_with(JackKind::Bot { m: m.into(), id: id.into() }, body, sub, window, cx);
    }

    fn jack_shell(&mut self, m: &str, dir: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mx) = &mut self.matrix else { return };
        if let Some(j) = &mx.jack
            && matches!(&j.kind, JackKind::Shell { m: a, dir: d } if a == m && d == dir)
        {
            let body = j.body.clone();
            body.update(cx, |b, cx| b.focus(window, cx));
            return;
        }
        let body = TermTile::shell(self.hub.clone(), m, dir, window, cx);
        let sub = cx.subscribe(&body, |_, _, _: &TileEvent, _| {});
        self.jack_with(JackKind::Shell { m: m.into(), dir: dir.into() }, body, sub, window, cx);
    }

    fn jack_with(&mut self, kind: JackKind, body: Entity<TermTile>, sub: Subscription, window: &mut Window, cx: &mut Context<Self>) {
        self.jack_out(window, cx);
        if let Some(v) = body.read(cx).view.clone() {
            v.update(cx, |v, cx| v.set_theme(Theme::Matrix, cx));
        }
        body.update(cx, |b, cx| b.focus(window, cx));
        if let Some(m) = &mut self.matrix {
            m.jack = Some(Jack { kind, body, at: Instant::now() });
            m._subs.push(sub);
        }
        cx.notify();
    }

    /// out of the terminal: the attach let go, the session goes on
    fn jack_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(m) = &mut self.matrix
            && m.jack.take().is_some()
        {
            m._subs.truncate(1);
            cx.notify();
        }
    }

    // ---- the operator's line

    /// a line typed at the operator's line
    pub fn matrix_run(&mut self, line: &str, window: &mut Window, cx: &mut Context<Self>) {
        let raw = line.trim().to_string();
        let Some(m) = &mut self.matrix else { return };
        if raw.is_empty() {
            return;
        }
        m.print(format!("> {raw}"), Class::Echo);
        if let Some((id, until)) = m.confirm.take() {
            let low = raw.to_lowercase();
            if (low == "y" || low == "yes") && Instant::now() < until {
                let Some(b) = self.state.bots.iter().find(|b| b.id == id).cloned() else { return m.print("it is gone already", Class::Plain) };
                if m.jacked_bot() == Some(&b.id) {
                    self.jack_out(window, cx);
                }
                return self.matrix_call(move |hub| hub.stop(&b.m, &b.id, true).map(|_| format!("{} is deleted", b.name)), window, cx);
            }
            if low == "n" || low == "no" {
                return m.print("kept", Class::Plain);
            }
        }
        let (verb, rest) = op::parse(&raw);
        let arg = rest.join(" ");
        let st = &self.state;
        match verb.as_str() {
            "help" => m.lines(op::help()),
            "ls" => m.lines(op::ls(&arg, st)),
            "jack" => match op::find(&arg, st) {
                Ok(b) => {
                    let (bm, bid) = (b.m.clone(), b.id.clone());
                    self.jack_in(&bm, &bid, window, cx);
                }
                Err(e) => m.print(e, Class::Err),
            },
            "exit" => {
                if m.jack.is_none() {
                    m.print("not jacked in", Class::Plain);
                } else {
                    self.jack_out(window, cx);
                }
            }
            "rabbit" => match op::rabbit(st, m.jacked_bot()) {
                None => m.print("no rabbit: nobody needs you", Class::Plain),
                Some(b) => {
                    let (bm, bid) = (b.m.clone(), b.id.clone());
                    m.print(format!("follow the white rabbit: {}", b.name), Class::Hot);
                    self.jack_in(&bm, &bid, window, cx);
                }
            },
            "load" => match op::place(&arg, st) {
                Err(e) => m.print(e, Class::Err),
                Ok((mid, dir)) => {
                    let on = if mid.is_empty() { String::new() } else { format!(" on {}", words::machine_name(&mid, st)) };
                    m.print(format!("loading an agent in {}{on}…", tilde(&dir)), Class::Plain);
                    let (m2, d2) = (mid.clone(), dir.clone());
                    crate::panels::call(
                        self,
                        move |hub| hub.spawn(&m2, &d2, ""),
                        move |b, r, window, cx| match r {
                            Ok(id) => {
                                if let Some(m) = &mut b.matrix {
                                    m.print("loaded", Class::Hot);
                                }
                                b.jack_in(&mid, &id, window, cx);
                            }
                            Err(e) if e.to_lowercase().contains("not trusted") => b.matrix_trust_first(mid, dir, window, cx),
                            Err(e) => {
                                if let Some(m) = &mut b.matrix {
                                    m.print(e, Class::Err);
                                }
                            }
                        },
                        window,
                        cx,
                    );
                }
            },
            "tell" => {
                let who = rest.first().map(String::as_str).unwrap_or("");
                match op::find(who, st) {
                    Err(e) => m.print(e, Class::Err),
                    Ok(_) if rest.len() < 2 => m.print("tell it what?", Class::Err),
                    Ok(b) => {
                        let (id, name, text) = (b.id.clone(), b.name.clone(), rest[1..].join(" "));
                        self.matrix_call(move |hub| hub.tell(&id, &text).map(|_| format!("told {name}")), window, cx);
                    }
                }
            }
            "reset" => match op::find(&arg, st) {
                Err(e) => m.print(e, Class::Err),
                Ok(b) if b.busy || b.waiting => m.print(format!("{} is busy: only an idle one forgets", b.name), Class::Err),
                Ok(b) => {
                    let b = b.clone();
                    self.matrix_call(move |hub| hub.clear(&b.m, &b.id).map(|_| format!("{}: /clear", b.name)), window, cx);
                }
            },
            "stop" => match op::find(&arg, st) {
                Err(e) => m.print(e, Class::Err),
                Ok(b) => {
                    let b = b.clone();
                    if m.jacked_bot() == Some(&b.id) {
                        self.jack_out(window, cx);
                    }
                    self.matrix_call(move |hub| hub.stop(&b.m, &b.id, false).map(|_| format!("{} stopped", b.name)), window, cx);
                }
            },
            "delete" => match op::find(&arg, st) {
                Err(e) => m.print(e, Class::Err),
                Ok(b) => {
                    m.confirm = Some((b.id.clone(), Instant::now() + Duration::from_secs(15)));
                    m.print(format!("delete {} for good? y / n", b.name), Class::Hot);
                }
            },
            "shell" => match op::place(&arg, st) {
                Ok((mid, dir)) => self.jack_shell(&mid, &dir, window, cx),
                Err(e) => m.print(e, Class::Err),
            },
            "limits" => m.lines(op::limits(st)),
            "nodes" => m.lines(op::nodes(st)),
            "room" => self.close_matrix(window, cx),
            "cls" => m.out.clear(),
            _ => m.print(format!("{verb}: no such command (help)"), Class::Err),
        }
        cx.notify();
    }

    /// a call to the hub that may take a while, off the main thread: what it says, or what went wrong, printed
    fn matrix_call(&mut self, f: impl FnOnce(&burrow_core::Hub) -> Result<String, String> + Send + 'static, window: &mut Window, cx: &mut Context<Self>) {
        crate::panels::call(
            self,
            move |hub| f(&hub),
            |b, r, _, _| {
                if let Some(m) = &mut b.matrix {
                    match r {
                        Ok(s) => m.print(s, Class::Plain),
                        Err(e) => m.print(e, Class::Err),
                    }
                }
            },
            window,
            cx,
        );
    }

    /// load in a folder Claude Code does not trust yet: your shell there with `claude` in it, its trust prompt on the
    /// screen; the agent is asked for again every three seconds while that shell is jacked into, then takes its place
    /// (as the room does)
    fn matrix_trust_first(&mut self, m: String, dir: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mx) = &mut self.matrix else { return };
        mx.print("that folder is not trusted yet: answer claude's question in the terminal", Class::Hot);
        self.jack_shell(&m, &dir, window, cx);
        let shell = |b: &Burrow| match b.matrix.as_ref().and_then(|x| x.jack.as_ref()) {
            Some(j) if matches!(&j.kind, JackKind::Shell { .. }) => Some(j.body.clone()),
            _ => None,
        };
        let Some(t) = shell(self) else { return };
        t.read(cx).say(b"claude\r");
        let hub = self.hub.clone();
        cx.spawn_in(window, async move |this, cx| {
            for _ in 0..100 {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                // (out of that shell, or out of the matrix: nothing starts)
                let Ok(still) = this.update(cx, |b, _| shell(b).is_some_and(|x| x == t)) else { return };
                if !still {
                    return;
                }
                let (h, m2, d2) = (hub.clone(), m.clone(), dir.clone());
                let id = match cx.background_executor().spawn(async move { h.spawn(&m2, &d2, "") }).await {
                    Ok(id) => id,
                    Err(e) if e.to_lowercase().contains("not trusted") => continue,
                    Err(e) => {
                        let _ = this.update(cx, |b, cx| {
                            if let Some(x) = &mut b.matrix {
                                x.print(e, Class::Err);
                                cx.notify();
                            }
                        });
                        return;
                    }
                };
                for _ in 0..3 {
                    let _ = this.update(cx, |_, cx| t.read(cx).say(b"\x03"));
                    cx.background_executor().timer(Duration::from_millis(300)).await;
                }
                let _ = this.update_in(cx, |b, window, cx| {
                    if let Some(x) = &mut b.matrix {
                        x.print("loaded", Class::Hot);
                    }
                    b.jack_in(&m, &id, window, cx);
                });
                return;
            }
        })
        .detach();
    }

    /// keys while it is up: the line's Enter, Tab, ↑ ↓ and Esc; typing anywhere else goes to the line; in the
    /// terminal every key is Claude Code's (Esc too)
    pub fn matrix_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(m) = &mut self.matrix else { return };
        let term = m.jack.as_ref().and_then(|j| j.body.read(cx).view.clone());
        if term.is_some_and(|v| v.focus_handle(cx).contains_focused(window, cx)) {
            return;
        }
        let k = e.keystroke.key.as_str();
        let field = m.field.clone();
        let text = field.read(cx).text().to_string();
        if field.read(cx).focused(window) {
            if k == "tab" {
                if let Some((v, hint)) = op::complete(&text, &self.state) {
                    field.update(cx, |f, cx| f.set(&v, cx));
                    m.hint = hint.unwrap_or_default();
                }
                cx.stop_propagation();
                return cx.notify();
            }
            m.hint.clear();
            match k {
                "enter" => {
                    field.update(cx, |f, cx| f.set("", cx));
                    let v = text.trim().to_string();
                    if !v.is_empty() && m.hist.last() != Some(&v) {
                        m.hist.push(v);
                        let n = m.hist.len();
                        m.hist.drain(..n.saturating_sub(200));
                        if let Some(p) = history_file(self) {
                            let m = self.matrix.as_ref().unwrap();
                            let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p)).and_then(|_| std::fs::write(&p, serde_json::to_string(&m.hist).unwrap_or_default()));
                        }
                    }
                    let m = self.matrix.as_mut().unwrap();
                    m.hi = m.hist.len();
                    self.matrix_run(&text, window, cx);
                }
                "up" => {
                    if m.hi > 0 {
                        m.hi -= 1;
                        let v = m.hist[m.hi].clone();
                        field.update(cx, |f, cx| f.set(&v, cx));
                    }
                }
                "down" => {
                    m.hi = (m.hi + 1).min(m.hist.len());
                    let v = m.hist.get(m.hi).cloned().unwrap_or_default();
                    field.update(cx, |f, cx| f.set(&v, cx));
                }
                // an empty line: out, back to the room (the window has no back button)
                "escape" if text.is_empty() => self.close_matrix(window, cx),
                "escape" => field.update(cx, |f, cx| f.set("", cx)),
                _ => return cx.notify(),
            }
            cx.stop_propagation();
            return cx.notify();
        }
        if k == "escape" && text.is_empty() {
            return self.close_matrix(window, cx);
        }
        let mo = e.keystroke.modifiers;
        let typed = e.keystroke.key_char.clone().filter(|c| c.chars().count() == 1 && !c.starts_with(char::is_control));
        if mo.platform || mo.control || mo.alt || typed.is_none() {
            return;
        }
        let v = format!("{text}{}", typed.unwrap_or_default());
        field.update(cx, |f, cx| f.set(&v, cx));
        field.read(cx).focus(window);
        cx.stop_propagation();
        cx.notify();
    }

    /// the pointer over the rain: the lane under it shows who it is
    fn matrix_pointer(&mut self, p: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        let Some(m) = &mut self.matrix else { return };
        let rel = p.zip(m.paint.bounds.get()).map(|(p, b)| (f32::from(p.x - b.origin.x), f32::from(p.y - b.origin.y)));
        let lane = rel.and_then(|(x, _)| m.rain.lane_at(x)).map(|l| l.id.clone());
        if lane != m.rain.hovered || rel != m.at {
            m.rain.hovered = lane;
            m.at = rel;
            cx.notify();
        }
    }

    /// BURROW_OPEN's matrix:<line> (typed at the line), matrix:@<agent> (the pointer on its lane, half way down, once
    /// the lanes stand), matrix:key:<keystroke> (pressed, as the keyboard would)
    pub fn matrix_debug(&mut self, v: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(k) = v.strip_prefix("key:") {
            if let Ok(ks) = gpui::Keystroke::parse(k) {
                window.defer(cx, move |window, cx| {
                    window.dispatch_keystroke(ks, cx);
                });
            }
            return;
        }
        let Some(q) = v.strip_prefix('@') else { return self.matrix_run(v, window, cx) };
        if let Some(m) = &mut self.matrix {
            m.point_at = Some(q.to_string());
        }
    }

    /// a click on the rain: on a lane, in; elsewhere, the line
    fn matrix_click(&mut self, p: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(m) = &mut self.matrix else { return };
        let b = m.paint.bounds.get().and_then(|b| m.rain.lane_at(f32::from(p.x - b.origin.x))).map(|l| l.b.clone());
        match b {
            Some(b) => self.jack_in(&b.m, &b.id, window, cx),
            None => m.field.read(cx).focus(window),
        }
    }

    // ---- what it looks like

    pub fn matrix_element(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.matrix.as_ref()?;
        let st = &self.state;
        // the header: the counts, the limits, the clock (the brand clear of the window's buttons)
        let (n, work, need, on, all) = op::counts(st);
        let head = format!("{n} agents · {work} working · ");
        let needs = if need > 0 { format!("{need} need you") } else { "0 need you".into() };
        let counts = format!("{head}{needs} · {on}/{all} nodes");
        let bold = HighlightStyle { color: Some(rgb(HOT).into()), font_weight: Some(FontWeight::BOLD), ..Default::default() };
        let counts = StyledText::new(counts).with_highlights(if need > 0 { vec![(head.len()..head.len() + needs.len(), bold)] } else { vec![] });
        let (h, mi) = local_time();
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .pl(px(88.))
            .pr(px(14.))
            .py(px(6.))
            // (the gaps as margins: a gap between them did not show here)
            .border_b_1()
            .border_color(rgb(G5))
            .whitespace_nowrap()
            .overflow_hidden()
            .child(div().flex().flex_none().gap(px(9.)).font_weight(FontWeight::BOLD).child("BURROW").child(div().text_color(rgb(G3)).child("//")).child("OPERATOR"))
            .child(div().flex_none().ml(px(24.)).text_color(rgb(G2)).child(counts))
            .child(div().flex_none().ml_auto().text_color(rgb(G3)).child(format!("5h {} left · week {} left", op::left(&st.usage.session), op::left(&st.usage.week))))
            .child(div().flex_none().ml(px(24.)).text_color(rgb(G3)).child(format!("{h:02}:{mi:02}")));
        let rain = self.rain_element(window, cx);
        let jack = self.jack_element(cx);
        let m = self.matrix.as_ref()?;
        let field = m.field.clone();
        let footer = div()
            .id("op")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .px(px(14.))
            .pt(px(5.))
            .pb(px(7.))
            .border_t_1()
            .border_color(rgb(G5))
            .bg(rgb(0x000500))
            .cursor(CursorStyle::IBeam)
            .child(div().flex_none().text_color(rgb(G2)).whitespace_nowrap().child("operator@burrow:~$"))
            .child(div().flex_1().min_w(px(0.)).child(field.clone()))
            .child(div().flex_none().max_w(relative(0.5)).text_color(rgb(G4)).whitespace_nowrap().overflow_hidden().child(m.hint.clone()))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| field.read(cx).focus(window));
        Some(
            div()
                .id("matrix")
                .absolute()
                .inset_0()
                .occlude()
                .bg(rgb(BG))
                .flex()
                .flex_col()
                .font_family(kit::MONO)
                .text_size(px(13.))
                .line_height(px(18.))
                .text_color(rgb(G))
                .child(header)
                .child(div().flex().flex_1().min_h(px(0.)).child(rain).children(jack))
                .child(footer)
                .into_any_element(),
        )
    }

    /// the rain: its canvas, the projects' names under it, who a lane is, what was printed
    fn rain_element(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let m = self.matrix.as_ref().unwrap();
        let st = &self.state;
        let (painter, glyphs) = (m.paint.clone(), m.glyphs.clone());
        let p2 = painter.clone();
        let cw = m.rain.cw;
        let canvas = canvas(
            move |b, window, _| {
                painter.bounds.set(Some(b));
                if painter.metrics.get().is_none() {
                    let ts = window.text_system();
                    let id = ts.resolve_font(&font(false));
                    let w = ts.advance(id, px(FONT), 'M').map(|s| f32::from(s.width)).unwrap_or(FONT * 0.6);
                    painter.metrics.set(Some((w.ceil() + 1., f32::from(ts.ascent(id, px(FONT))))));
                }
            },
            move |b, _, window, _| {
                window.paint_quad(fill(b, rgb(BG)));
                let Some((_, ascent)) = p2.metrics.get() else { return };
                window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
                    for g in glyphs.iter() {
                        let shaped = *p2.glyphs.borrow_mut().entry((g.c, g.bold)).or_insert_with(|| {
                            let s = g.c.to_string();
                            let run = TextRun { len: s.len(), font: font(g.bold), color: rgb(G).into(), background_color: None, underline: None, strikethrough: None };
                            let line = window.text_system().layout_line(&s, px(FONT), &[run], None);
                            line.runs.first().and_then(|r| r.glyphs.first().map(|x| (r.font_id, x.id)))
                        });
                        if let Some((f, id)) = shaped {
                            let _ = window.paint_glyph(point(b.origin.x + px(g.x), b.origin.y + px(g.y + ascent)), f, id, px(FONT), rgba(g.rgb, g.a));
                        }
                    }
                });
            },
        )
        .absolute()
        .size_full();
        // the projects' names along the bottom; a machine out of reach in red
        let labels = m.rain.groups.iter().map(|g| {
            let text = if g.lost { format!("{} · signal lost", g.label) } else { g.label.clone() };
            div()
                .absolute()
                .bottom(px(4.))
                .left(px(g.x0 as f32 * cw))
                .w(px((g.x1.saturating_sub(g.x0)).max(1) as f32 * cw))
                .h(px(16.))
                .truncate()
                .text_center()
                .text_size(px(11.))
                .line_height(px(14.))
                .text_color(rgb(if g.lost { WARN } else { G3 }))
                .border_t_1()
                .border_color(rgb(if g.lost { 0x4d1210 } else { G4 }))
                .pt(px(1.))
                .child(text)
        });
        // who the lane under the pointer is
        let size = m.paint.bounds.get().map(|b| (f32::from(b.size.width), f32::from(b.size.height))).unwrap_or((0., 0.));
        let info = m.rain.hovered.as_ref().and_then(|id| m.rain.lane(id)).zip(m.at).map(|(l, (x, y))| {
            let b = &l.b;
            let need = words::state_of(b, st) == words::St::Need;
            let k = |parts: &[(&str, bool)]| {
                let (mut s, mut hl) = (String::new(), vec![]);
                for (t, key) in parts {
                    if *key {
                        hl.push((s.len()..s.len() + t.len(), HighlightStyle { color: Some(rgb(G3).into()), ..Default::default() }));
                    }
                    s.push_str(t);
                }
                StyledText::new(s).with_highlights(hl)
            };
            let note = self.digest.sessions.get(&b.id).and_then(|n| n.get("headline")).and_then(|h| h.as_str()).filter(|h| !h.is_empty()).map(String::from).or_else(|| (!b.detail.is_empty()).then(|| b.detail.clone()));
            let ago = words::ago(b.started, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0));
            let mname = words::machine_name(&b.m, st);
            // (how tall it is, to keep it in the box: a row a line, the note wrapped at about 43 letters)
            let h = 20. + 18. + 26. + 3. * 18. + note.as_ref().map_or(0., |n| 6. + n.chars().count().div_ceil(43) as f32 * 18.);
            div()
                .absolute()
                .left(px((x + 18.).min(size.0 - 360. - 6.).max(6.)))
                .top(px((y - 20.).min(size.1 - h - 40.).max(6.)))
                .w(px(360.))
                .px(px(12.))
                .py(px(10.))
                .bg(gpui::rgba(0x000802f0))
                .border_1()
                .border_color(rgb(G3))
                .shadow(vec![BoxShadow { color: rgba(G, 0.12), offset: point(px(0.), px(0.)), blur_radius: px(24.), spread_radius: px(0.) }])
                .child(div().text_color(rgb(HOT)).font_weight(FontWeight::BOLD).text_size(px(14.)).child(b.name.clone()))
                .child(div().mt(px(2.)).mb(px(6.)).when(need, |d| d.text_color(rgb(HOT))).child(word_of(b, st)))
                .child(k(&[("project", true), (&format!(" {} ", project_of(b, st)), false), ("on", true), (&format!(" {mname}"), false)]))
                .child(k(&[("folder", true), (&format!(" {}", tilde(&b.cwd)), false)]))
                .child(k(&[("since", true), (&format!(" {ago} ago"), false), (if b.rc.is_empty() { "" } else { " · in the Claude app too" }, true)]))
                .when_some(note, |d, n| d.child(div().mt(px(6.)).text_color(rgb(G2)).child(n)))
        });
        // what was printed at the line, above the projects' names
        let out = (!m.out.is_empty()).then(|| {
            div()
                .absolute()
                .left(px(0.))
                .bottom(px(24.))
                .max_w(px(960.))
                .px(px(14.))
                .py(px(6.))
                .bg(gpui::rgba(0x000401f0))
                .border_t_1()
                .border_r_1()
                .border_color(rgb(G4))
                .children(m.out.iter().map(|(t, c, _)| {
                    let col = match c {
                        Class::Plain => G,
                        Class::Echo => G3,
                        Class::Err => WARN,
                        Class::Hot => HOT,
                    };
                    div().text_color(rgb(col)).child(t.clone())
                }))
        });
        let hovered = m.rain.hovered.is_some();
        div()
            .id("rainbox")
            .relative()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .overflow_hidden()
            .cursor(if hovered { CursorStyle::PointingHand } else { CursorStyle::Arrow })
            .child(canvas)
            .children(labels)
            .children(info)
            .children(out)
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| this.matrix_pointer(Some(e.position), cx)))
            .on_hover(cx.listener(|this, on: &bool, _, cx| {
                if !*on {
                    this.matrix_pointer(None, cx);
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, window, cx| this.matrix_click(e.position, window, cx)))
            .into_any_element()
    }

    /// the terminal jacked into, on the right: its name, what it is, APP, EXIT
    fn jack_element(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let m = self.matrix.as_ref()?;
        let j = m.jack.as_ref()?;
        let st = &self.state;
        let (name, sub, rc) = match &j.kind {
            JackKind::Shell { m, dir } => (words::base(dir), format!("your shell in {} · {}", tilde(dir), words::machine_name(m, st)), String::new()),
            JackKind::Bot { m, id } => match st.bots.iter().find(|b| b.id == *id) {
                Some(b) => (b.name.clone(), format!("{} · {} · {}", word_of(b, st).to_lowercase(), project_of(b, st), words::machine_name(m, st)), b.rc.clone()),
                None => (id.clone(), words::machine_name(m, st), String::new()),
            },
        };
        // jacking in: out of the dark, in a third of a second (the frames go on at thirty a second meanwhile)
        let k = (j.at.elapsed().as_secs_f32() / 0.35).min(1.);
        let btn = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .flex_none()
                .px(px(8.))
                .py(px(1.))
                .border_1()
                .border_color(rgb(G3))
                .text_color(rgb(G))
                .cursor(CursorStyle::PointingHand)
                .hover(|s| s.bg(rgb(G)).text_color(rgb(BG)).border_color(rgb(G)))
                .child(label)
        };
        let url = format!("https://claude.ai/code/session_{}", rc.trim_start_matches("cse_"));
        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .px(px(10.))
            .py(px(5.))
            .border_b_1()
            .border_color(rgb(G4))
            .whitespace_nowrap()
            .overflow_hidden()
            .child(div().flex_none().text_color(rgb(HOT)).font_weight(FontWeight::BOLD).child(name))
            .child(div().min_w(px(0.)).overflow_hidden().text_ellipsis().text_color(rgb(G3)).child(sub))
            .child(div().flex_1())
            .when(!rc.is_empty(), |d| d.child(btn("japp", "APP").on_click(move |_, _, cx| cx.open_url(&url))))
            .child(btn("jexit", "EXIT").on_click(cx.listener(|this, _, window, cx| {
                this.jack_out(window, cx);
                if let Some(m) = &this.matrix {
                    m.field.read(cx).focus(window);
                }
            })));
        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .w(relative(0.58))
                .min_w(px(420.))
                .h_full()
                .border_l_1()
                .border_color(rgb(G3))
                .bg(rgb(JACK))
                .opacity(k)
                .child(bar)
                .child(div().flex_1().min_h(px(0.)).pt(px(6.)).pr(px(4.)).pb(px(4.)).pl(px(8.)).child(j.body.clone()))
                .into_any_element(),
        )
    }
}
