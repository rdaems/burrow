//! the terminal as a GPUI view: `Screen` drawn in a tile and driven by the keyboard and the mouse, as
//! xterm.js was in web/index.html. No pty here: bytes come in through `feed`, go out through the sink

use std::ops::Range;
use std::time::{Duration, Instant};

use alacritty_terminal::index::{Column, Line, Point as GridPoint, Side};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{CursorShape, Rgb};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, Font, FontFeatures, FontStyle, FontWeight, GlobalElementId,
    Hitbox, HitboxBehavior, Hsla, InputHandler, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels, Point, Render,
    ScrollWheelEvent, ShapedLine, SharedString, StrikethroughStyle, Styled, Subscription, Task, TextRun,
    UTF16Selection, UnderlineStyle, Window, div, fill, outline, point, px, relative, size,
};

use crate::boxdraw::{self, Glyph};
use crate::mouse::Button;
use crate::screen::{KeyResult, Link, Out, Screen, Style};
use crate::theme::{Theme, mix};

const FAMILY: &str = "JetBrainsMono NF";
const FONT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 1.15;
/// burrow.css's `.term` padding: top, right, bottom, left
const PAD: [f32; 4] = [10.0, 12.0, 8.0, 12.0];
const BLINK: Duration = Duration::from_millis(600);

/// what the terminal tells its owner (`cx.subscribe`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermEvent {
    /// the grid's size changed: tell the program (the pty's winsize)
    Resize { cols: u16, rows: u16 },
    Title(String),
    Bell,
    /// the person typed or pasted (never the terminal's own reports)
    Typed { enter: bool },
    /// a link was clicked: open it
    Link(String),
}

struct FontsLoaded;
impl gpui::Global for FontsLoaded {}

/// the fonts, added to the app once (the first terminal does it)
pub fn load_fonts(cx: &mut App) {
    if cx.has_global::<FontsLoaded>() {
        return;
    }
    let fonts = vec![
        std::borrow::Cow::Borrowed(&include_bytes!("../../../assets/fonts/BurrowMono-Regular.ttf")[..]),
        std::borrow::Cow::Borrowed(&include_bytes!("../../../assets/fonts/BurrowMono-Bold.ttf")[..]),
    ];
    cx.text_system().add_fonts(fonts).expect("the terminal's fonts");
    // the page's font is a subset without the Nerd Font icons (prompts are full of them): the whole font if it is
    // installed, the subset otherwise, and the icons from any Nerd Font there is
    let names = cx.text_system().all_font_names();
    let _ = INSTALLED.set(["JetBrainsMono Nerd Font Mono", "JetBrainsMono Nerd Font"].into_iter().find(|f| names.iter().any(|n| n == f)).map(String::from));
    cx.set_global(FontsLoaded);
}

static INSTALLED: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

fn font(bold: bool, italic: bool) -> Font {
    let family = INSTALLED.get().cloned().flatten().unwrap_or_else(|| FAMILY.to_string());
    Font {
        family: family.into(),
        features: FontFeatures::disable_ligatures(),
        fallbacks: Some(gpui::FontFallbacks::from_fonts(vec![
            "Symbols Nerd Font Mono".into(),
            "JetBrainsMono Nerd Font Mono".into(),
            FAMILY.into(),
            "Menlo".into(),
        ])),
        weight: if bold { FontWeight::BOLD } else { FontWeight::NORMAL },
        style: if italic { FontStyle::Italic } else { FontStyle::Normal },
    }
}

fn hsla(c: Rgb) -> Hsla {
    gpui::Rgba { r: c.r as f32 / 255.0, g: c.g as f32 / 255.0, b: c.b as f32 / 255.0, a: 1.0 }.into()
}

fn hsla_rgba(c: u32) -> Hsla {
    gpui::rgba(c).into()
}

#[derive(Clone, Copy)]
struct Metrics {
    scale: f32,
    w: Pixels,
    h: Pixels,
}

pub struct TermView {
    screen: Screen,
    sink: Box<dyn FnMut(&[u8])>,
    focus: FocusHandle,
    focused: bool,
    metrics: Option<Metrics>,
    /// the grid's top left in the window, where the last frame put it
    origin: Point<Pixels>,
    hover: Option<Link>,
    blink_on: bool,
    blink: Option<Task<()>>,
    sync: Option<Task<()>>,
    reader: Option<Task<()>>,
    /// text being composed in an input method, shown at the cursor
    marked: Option<String>,
    wheel: f32,
    _subs: Vec<Subscription>,
}

impl EventEmitter<TermEvent> for TermView {}

impl Focusable for TermView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TermView {
    /// a terminal writing what the person does to `sink` (called on the main thread: don't block)
    pub fn new(sink: impl FnMut(&[u8]) + 'static, window: &mut Window, cx: &mut Context<Self>) -> Self {
        load_fonts(cx);
        let focus = cx.focus_handle();
        let subs = vec![
            cx.on_focus(&focus, window, |v, _, cx| {
                v.focused = true;
                v.screen.focus(true);
                v.restart_blink(cx);
                v.flush(cx);
            }),
            cx.on_blur(&focus, window, |v, _, cx| {
                v.focused = false;
                v.screen.focus(false);
                v.blink = None;
                v.blink_on = true;
                v.flush(cx);
            }),
        ];
        TermView {
            screen: Screen::new(80, 24),
            sink: Box::new(sink),
            focus,
            focused: false,
            metrics: None,
            origin: Point::default(),
            hover: None,
            blink_on: true,
            blink: None,
            sync: None,
            reader: None,
            marked: None,
            wheel: 0.0,
            _subs: subs,
        }
    }

    /// bytes from the program
    pub fn feed(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.screen.feed(bytes);
        self.arm_sync(cx);
        self.flush(cx);
    }

    /// feed everything that comes through `rx` (from the pty's reader thread, say)
    pub fn feed_from(&mut self, mut rx: UnboundedReceiver<Vec<u8>>, cx: &mut Context<Self>) {
        self.reader = Some(cx.spawn(async move |this, cx| {
            while let Some(b) = rx.next().await {
                if this.update(cx, |v, cx| v.feed(&b, cx)).is_err() {
                    break;
                }
            }
        }));
    }

    /// the same from a std channel (a thread hands the bytes over; it ends when the sender goes)
    pub fn feed_from_std(&mut self, rx: std::sync::mpsc::Receiver<Vec<u8>>, cx: &mut Context<Self>) {
        let (tx, frx) = futures::channel::mpsc::unbounded();
        std::thread::spawn(move || {
            while let Ok(b) = rx.recv() {
                if tx.unbounded_send(b).is_err() {
                    break;
                }
            }
        });
        self.feed_from(frx, cx);
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        if self.screen.theme != theme {
            self.screen.theme = theme;
            cx.notify();
        }
    }

    /// a paste (bracketed when the program turned that on): a dropped file's path, say
    pub fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        self.screen.paste(text);
        self.flush(cx);
    }

    /// bytes as if typed
    pub fn input(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.screen.input(bytes);
        self.flush(cx);
    }

    pub fn is_bracketed_paste(&self) -> bool {
        self.screen.is_bracketed_paste()
    }

    /// the grid's size in cells
    pub fn size(&self) -> (u16, u16) {
        (self.screen.cols() as u16, self.screen.rows() as u16)
    }

    /// what is on it (`contents()`, `row_text`), to look for a program's prompt, say
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    fn flush(&mut self, cx: &mut Context<Self>) {
        self.send(cx);
        cx.notify();
    }

    // what came out, without a new frame (a mouse report changes nothing on the screen)
    fn send(&mut self, cx: &mut Context<Self>) {
        for o in self.screen.drain() {
            match o {
                Out::Write(b) => (self.sink)(&b),
                Out::Typed { enter } => {
                    self.restart_blink(cx);
                    cx.emit(TermEvent::Typed { enter })
                }
                Out::Title(t) => cx.emit(TermEvent::Title(t)),
                Out::Bell => cx.emit(TermEvent::Bell),
                Out::Clipboard(s) => cx.write_to_clipboard(ClipboardItem::new_string(s)),
                Out::Link(u) => cx.emit(TermEvent::Link(u)),
            }
        }
    }

    // a synchronized update that never ends must not freeze the terminal: show it when it runs out
    fn arm_sync(&mut self, cx: &mut Context<Self>) {
        let Some(at) = self.screen.sync_deadline() else { return };
        if self.sync.is_some() {
            return;
        }
        self.sync = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(at.saturating_duration_since(Instant::now())).await;
            let _ = this.update(cx, |v, cx| {
                v.sync = None;
                match v.screen.sync_deadline() {
                    Some(d) if d <= Instant::now() => {
                        v.screen.end_sync();
                        v.flush(cx);
                    }
                    Some(_) => v.arm_sync(cx),
                    None => {}
                }
            });
        }));
    }

    fn restart_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_on = true;
        if !self.focused {
            return;
        }
        self.blink = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let alive = this.update(cx, |v, cx| {
                    v.blink_on = !v.blink_on;
                    if v.screen.cursor().is_some_and(|c| c.2) {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
    }

    fn key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match self.screen.key(&ev.keystroke) {
            KeyResult::Ignored => return, // plain text comes through the input handler
            KeyResult::Done => {}
            KeyResult::Paste => {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    self.screen.paste(&t);
                }
            }
        }
        cx.stop_propagation();
        self.flush(cx);
    }

    // ---- the mouse

    fn cell_at(&self, p: Point<Pixels>) -> ((usize, usize), Side) {
        let Some(m) = self.metrics else { return ((0, 0), Side::Left) };
        let x = ((p.x - self.origin.x) / m.w).max(0.0);
        let y = ((p.y - self.origin.y) / m.h).max(0.0);
        let col = (x as usize).min(self.screen.cols() - 1);
        let row = (y as usize).min(self.screen.rows() - 1);
        // past the last column is the right side of the last cell
        let side = if x as usize >= self.screen.cols() || x.fract() >= 0.5 { Side::Right } else { Side::Left };
        ((col, row), side)
    }

    fn mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        let Some(b) = button(ev.button) else { return };
        let (at, side) = self.cell_at(ev.position);
        self.screen.mouse_down(b, at, side, &ev.modifiers, ev.click_count);
        self.flush(cx);
    }

    fn mouse_move(&mut self, ev: &MouseMoveEvent, hovered: bool, cx: &mut Context<Self>) {
        let (at, side) = self.cell_at(ev.position);
        if self.screen.pressed() {
            self.screen.mouse_move(at, side, &ev.modifiers);
            if self.screen.selecting() {
                self.flush(cx);
            } else {
                self.send(cx);
            }
            return;
        }
        let link = if hovered { self.screen.link_at(at) } else { None };
        if link != self.hover {
            self.hover = link;
            cx.notify();
        }
        if hovered && self.screen.mode().contains(TermMode::MOUSE_MOTION) {
            self.screen.mouse_move(at, side, &ev.modifiers);
            self.send(cx);
        }
    }

    fn mouse_up(&mut self, ev: &MouseUpEvent, cx: &mut Context<Self>) {
        let Some(b) = button(ev.button) else { return };
        let (at, _) = self.cell_at(ev.position);
        self.screen.mouse_up(b, at, &ev.modifiers);
        self.flush(cx);
    }

    fn scroll(&mut self, ev: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some(m) = self.metrics else { return };
        self.wheel += ev.delta.pixel_delta(m.h).y / m.h;
        let lines = self.wheel.trunc();
        self.wheel -= lines;
        if lines != 0.0 {
            let (at, _) = self.cell_at(ev.position);
            self.screen.wheel(lines as i32, at, &ev.modifiers);
            self.flush(cx);
        }
    }

    // ---- a frame

    fn layout(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Frame {
        let scale = window.scale_factor();
        let m = match self.metrics {
            Some(m) if m.scale == scale => m,
            _ => {
                let ts = window.text_system();
                let id = ts.resolve_font(&font(false, false));
                let w = ts.advance(id, px(FONT_SIZE), 'M').map(|s| s.width).unwrap_or(px(FONT_SIZE * 0.6));
                // whole device pixels a row, so the lines between rows stay sharp
                let h = px((FONT_SIZE * LINE_HEIGHT * scale).round() / scale);
                let m = Metrics { scale, w, h };
                self.metrics = Some(m);
                m
            }
        };
        let origin = point(bounds.origin.x + px(PAD[3]), bounds.origin.y + px(PAD[0]));
        self.origin = origin;
        let inner_w = bounds.size.width - px(PAD[1] + PAD[3]);
        let inner_h = bounds.size.height - px(PAD[0] + PAD[2]);
        let cols = ((inner_w / m.w).floor() as usize).max(2);
        let rows = ((inner_h / m.h).floor() as usize).max(2);
        if self.screen.resize(cols, rows, f32::from(m.w) * scale, f32::from(m.h) * scale) {
            self.hover = None;
            cx.emit(TermEvent::Resize { cols: cols as u16, rows: rows as u16 });
        }
        let focused = self.focus.is_focused(window);
        build(&self.screen, m, origin, focused, self.blink_on, self.hover.as_ref(), self.marked.as_deref(), window)
    }

    fn cursor_style(&self, alt: bool) -> CursorStyle {
        if self.hover.is_some() {
            CursorStyle::PointingHand
        } else if self.screen.mode().intersects(TermMode::MOUSE_MODE) && !alt {
            CursorStyle::Arrow
        } else {
            CursorStyle::IBeam
        }
    }
}

fn button(b: MouseButton) -> Option<Button> {
    match b {
        MouseButton::Left => Some(Button::Left),
        MouseButton::Middle => Some(Button::Middle),
        MouseButton::Right => Some(Button::Right),
        _ => None,
    }
}

impl Render for TermView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key_down))
            .child(TermElement { view: cx.entity() })
    }
}

impl EntityInputHandler for TermView {
    fn text_for_range(&mut self, _: Range<usize>, _: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        None
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: 0..0, reversed: false })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|m| 0..m.encode_utf16().count())
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(&mut self, _: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        self.screen.text(text);
        self.flush(cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked = (!text.is_empty()).then(|| text.to_string());
        cx.notify();
    }
    // where the input method puts its window: at the cursor
    fn bounds_for_range(&mut self, _: Range<usize>, _: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let m = self.metrics?;
        let ((col, row), _, _) = self.screen.cursor_cell()?;
        Some(Bounds::new(point(self.origin.x + m.w * col as f32, self.origin.y + m.h * row as f32), size(m.w, m.h)))
    }
    fn character_index_for_point(&mut self, _: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        None
    }
}

// the view's input handler, but a held key repeats (as in a terminal) instead of opening macOS's
// accent menu
struct TermInput(ElementInputHandler<TermView>);

impl InputHandler for TermInput {
    fn selected_text_range(&mut self, ignore: bool, window: &mut Window, cx: &mut App) -> Option<UTF16Selection> {
        self.0.selected_text_range(ignore, window, cx)
    }
    fn marked_text_range(&mut self, window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.0.marked_text_range(window, cx)
    }
    fn text_for_range(&mut self, r: Range<usize>, adj: &mut Option<Range<usize>>, window: &mut Window, cx: &mut App) -> Option<String> {
        self.0.text_for_range(r, adj, window, cx)
    }
    fn replace_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, window: &mut Window, cx: &mut App) {
        self.0.replace_text_in_range(r, text, window, cx)
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        text: &str,
        sel: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.replace_and_mark_text_in_range(r, text, sel, window, cx)
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut App) {
        self.0.unmark_text(window, cx)
    }
    fn bounds_for_range(&mut self, r: Range<usize>, window: &mut Window, cx: &mut App) -> Option<Bounds<Pixels>> {
        self.0.bounds_for_range(r, window, cx)
    }
    fn character_index_for_point(&mut self, p: Point<Pixels>, window: &mut Window, cx: &mut App) -> Option<usize> {
        self.0.character_index_for_point(p, window, cx)
    }
    fn apple_press_and_hold_enabled(&mut self) -> bool {
        false
    }
}

// ---- what a frame paints: built in prepaint, painted in paint

struct Frame {
    bg: Hsla,
    quads: Vec<(Bounds<Pixels>, Hsla)>,
    lines: Vec<(Point<Pixels>, ShapedLine)>,
    /// rounded corners: (cell origin, opening right, opening down, colour)
    rounds: Vec<(Point<Pixels>, bool, bool, Hsla)>,
    /// the selection, under the text
    sel: Vec<(Bounds<Pixels>, Hsla)>,
    /// drawn over the text: beam and underline cursors, a link's underline
    over: Vec<(Bounds<Pixels>, Hsla)>,
    hollow: Option<(Bounds<Pixels>, Hsla)>,
    m: Metrics,
}

#[derive(Clone, Copy, PartialEq)]
struct RunStyle {
    fg: Rgb,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}

#[allow(clippy::too_many_arguments)]
fn build(
    screen: &Screen,
    m: Metrics,
    origin: Point<Pixels>,
    focused: bool,
    blink_on: bool,
    hover: Option<&Link>,
    marked: Option<&str>,
    window: &mut Window,
) -> Frame {
    let pal = screen.theme.palette();
    let term_bg = screen.background();
    let cursor_rgb = crate::theme::rgb(pal.cursor);
    let mut f = Frame {
        bg: hsla(term_bg),
        quads: Vec::new(),
        lines: Vec::new(),
        rounds: Vec::new(),
        sel: Vec::new(),
        over: Vec::new(),
        hollow: None,
        m,
    };
    let at = |col: usize, row: usize| point(origin.x + m.w * col as f32, origin.y + m.h * row as f32);
    let cell_bounds = |col: usize, row: usize, w: usize| Bounds::new(at(col, row), size(m.w * w as f32, m.h));
    let ts = window.text_system().clone();
    let font_size = px(FONT_SIZE);
    let shape = |text: String, s: RunStyle| -> ShapedLine {
        let run = TextRun {
            len: text.len(),
            font: font(s.bold, s.italic),
            color: hsla(s.fg),
            background_color: None,
            underline: s.underline.then(|| UnderlineStyle { thickness: px(1.0), color: None, wavy: false }),
            strikethrough: s.strike.then(|| StrikethroughStyle { thickness: px(1.0), color: None }),
        };
        ts.shape_line(SharedString::from(text), font_size, &[run], None)
    };

    // the cursor: a block takes the cell's colours over; the others are drawn on top
    let cursor = screen.cursor().filter(|_| marked.is_none());
    let mut block_at = None;
    if let Some(((col, row), shape_, blinks)) = cursor {
        let w = if screen.cell(col, row).flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 };
        let b = cell_bounds(col, row, w);
        let c = hsla(cursor_rgb);
        if !focused {
            f.hollow = Some((b, c));
        } else if !blinks || blink_on {
            match shape_ {
                CursorShape::Block => block_at = Some((col, row)),
                CursorShape::HollowBlock => f.hollow = Some((b, c)),
                CursorShape::Underline => f.over.push((Bounds::new(point(b.origin.x, b.origin.y + m.h - px(2.0)), size(b.size.width, px(2.0))), c)),
                CursorShape::Beam => f.over.push((Bounds::new(b.origin, size(px(1.0), m.h)), c)),
                CursorShape::Hidden => {}
            }
        }
    }

    let term = screen.term();
    let selection = term.selection.as_ref().and_then(|s| s.to_range(term));
    let off = screen.display_offset() as i32;
    let sel_color = hsla_rgba(pal.selection);
    let (cols, rows) = (screen.cols(), screen.rows());
    let t = (1.0 / m.scale).max(1.0); // a light line: one point

    for row in 0..rows {
        // runs of plain ASCII in one style are shaped as one line; anything else is shaped on its own and
        // put at its column, so a glyph from a fallback font can't push the rest out of the grid
        let mut run: Option<(usize, String, RunStyle)> = None;
        let mut bg_run: Option<(usize, usize, Rgb)> = None;
        let mut sel_run: Option<Range<usize>> = None;
        let flush_run = |run: &mut Option<(usize, String, RunStyle)>, f: &mut Frame| {
            if let Some((col, text, s)) = run.take() {
                let text = if s.underline || s.strike { text } else { text.trim_end().to_string() };
                if !text.is_empty() {
                    f.lines.push((at(col, row), shape(text, s)));
                }
            }
        };
        let line = Line(row as i32 - off);
        let mut col = 0;
        while col < cols {
            let cell = screen.cell(col, row);
            let flags = cell.flags;
            if flags.contains(Flags::WIDE_CHAR_SPACER) {
                col += 1;
                continue;
            }
            let w = if flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 };
            let mut st: Style = screen.style(cell);
            if block_at == Some((col, row)) {
                st.bg = cursor_rgb;
                st.fg = term_bg;
            }
            // backgrounds, merged along the row
            match &mut bg_run {
                Some((_, end, c)) if *c == st.bg && *end == col => *end += w,
                _ => {
                    if let Some((s, e, c)) = bg_run.take()
                        && c != term_bg
                    {
                        f.quads.push((Bounds::new(at(s, row), size(m.w * (e - s) as f32, m.h)), hsla(c)));
                    }
                    bg_run = Some((col, col + w, st.bg));
                }
            }
            let selected = selection.is_some_and(|s| s.contains(GridPoint::new(line, Column(col))));
            match (&mut sel_run, selected) {
                (Some(r), true) if r.end == col => r.end += w,
                (_, true) => {
                    if let Some(r) = sel_run.take() {
                        f.sel.push((Bounds::new(at(r.start, row), size(m.w * r.len() as f32, m.h)), sel_color));
                    }
                    sel_run = Some(col..col + w);
                }
                (_, false) => {
                    if let Some(r) = sel_run.take() {
                        f.sel.push((Bounds::new(at(r.start, row), size(m.w * r.len() as f32, m.h)), sel_color));
                    }
                }
            }
            let rs = RunStyle { fg: st.fg, bold: st.bold, italic: st.italic, underline: st.underline, strike: st.strike };
            let c = if flags.intersects(Flags::HIDDEN | Flags::LEADING_WIDE_CHAR_SPACER) || cell.c == '\0' { ' ' } else { cell.c };
            let zero = cell.zerowidth();
            if let Some(g) = boxdraw::glyph(c).filter(|_| w == 1) {
                flush_run(&mut run, &mut f);
                let b = cell_bounds(col, row, 1);
                let col_ = hsla(st.fg);
                let (x, y, cw, ch) = (f32::from(b.origin.x), f32::from(b.origin.y), f32::from(m.w), f32::from(m.h));
                match g {
                    Glyph::Arms(a) => {
                        for (x0, y0, x1, y1) in boxdraw::arms(a, x, y, cw, ch, t) {
                            f.quads.push((Bounds::from_corners(point(px(x0), px(y0)), point(px(x1), px(y1))), col_));
                        }
                    }
                    Glyph::Round { right, down } => f.rounds.push((b.origin, right, down, col_)),
                    Glyph::Blocks(q, a) => {
                        let c = if a < 1.0 { hsla(mix(st.fg, st.bg, a)) } else { col_ };
                        for (x0, y0, x1, y1) in q {
                            let p0 = point(px(x + x0 * cw), px(y + y0 * ch));
                            let p1 = point(px(x + x1 * cw), px(y + y1 * ch));
                            f.quads.push((Bounds::from_corners(p0, p1), c));
                        }
                    }
                }
                if st.underline || st.strike {
                    run = Some((col, " ".into(), rs)); // the decoration still goes under it
                    flush_run(&mut run, &mut f);
                }
            } else if (c == ' ' || c.is_ascii_graphic()) && w == 1 && zero.is_none() {
                match &mut run {
                    Some((start, text, s)) if *s == rs && *start + text.len() == col => text.push(c),
                    _ => {
                        flush_run(&mut run, &mut f);
                        if c != ' ' || st.underline || st.strike {
                            run = Some((col, c.to_string(), rs));
                        }
                    }
                }
            } else {
                flush_run(&mut run, &mut f);
                let mut s = c.to_string();
                if let Some(z) = zero {
                    s.extend(z);
                }
                f.lines.push((at(col, row), shape(s, rs)));
            }
            col += w;
        }
        flush_run(&mut run, &mut f);
        if let Some((s, e, c)) = bg_run
            && c != term_bg
        {
            f.quads.push((Bounds::new(at(s, row), size(m.w * (e - s) as f32, m.h)), hsla(c)));
        }
        if let Some(r) = sel_run {
            f.sel.push((Bounds::new(at(r.start, row), size(m.w * r.len() as f32, m.h)), sel_color));
        }
    }

    // a hovered link is underlined
    if let Some(l) = hover {
        for (row, cs) in &l.spans {
            let fg = hsla(screen.style(screen.cell(cs.start, *row)).fg);
            let p = at(cs.start, *row);
            f.over.push((Bounds::new(point(p.x, p.y + m.h - px(2.0)), size(m.w * cs.len() as f32, px(1.0))), fg));
        }
    }

    // text being composed, over the cursor's cell
    if let Some(text) = marked
        && let Some(((col, row), _, _)) = screen.cursor_cell()
    {
        let s = RunStyle { fg: crate::theme::rgb(pal.foreground), bold: false, italic: false, underline: true, strike: false };
        let l = shape(text.to_string(), s);
        f.quads.push((Bounds::new(at(col, row), size(l.width.max(m.w), m.h)), f.bg));
        f.lines.push((at(col, row), l));
    }
    f
}

impl Frame {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.paint_quad(fill(bounds, self.bg));
        for (b, c) in &self.quads {
            window.paint_quad(fill(*b, *c));
        }
        let (w, h) = (self.m.w, self.m.h);
        let t = px((1.0 / self.m.scale).max(1.0));
        for (o, right, down, c) in &self.rounds {
            // from the edge the corner opens to, a quarter curve through the centre's corner, to the other edge
            let cx_ = px((f32::from(o.x + w / 2.0) - f32::from(t) / 2.0).round()) + t / 2.0;
            let cy_ = px((f32::from(o.y + h / 2.0) - f32::from(t) / 2.0).round()) + t / 2.0;
            let r = w / 2.0;
            let ex = if *right { o.x + w } else { o.x };
            let ey = if *down { o.y + h } else { o.y };
            let sx = if *right { 1.0 } else { -1.0 };
            let sy = if *down { 1.0 } else { -1.0 };
            let mut pb = PathBuilder::stroke(t);
            pb.move_to(point(ex, cy_));
            pb.line_to(point(cx_ + r * sx, cy_));
            pb.curve_to(point(cx_, cy_ + r * sy), point(cx_, cy_));
            pb.line_to(point(cx_, ey));
            if let Ok(path) = pb.build() {
                window.paint_path(path, *c);
            }
        }
        for (b, c) in &self.sel {
            window.paint_quad(fill(*b, *c));
        }
        for (o, l) in &self.lines {
            let _ = l.paint(*o, h, window, cx);
        }
        for (b, c) in &self.over {
            window.paint_quad(fill(*b, *c));
        }
        if let Some((b, c)) = self.hollow {
            window.paint_quad(outline(b, c, gpui::BorderStyle::Solid));
        }
    }
}

// ---- the element: lays the grid out in the tile, paints it, takes the mouse

struct TermElement {
    view: Entity<TermView>,
}

impl IntoElement for TermElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TermElement {
    type RequestLayoutState = ();
    type PrepaintState = (Frame, Hitbox);

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = gpui::Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> (Frame, Hitbox) {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        let frame = self.view.update(cx, |v, cx| v.layout(bounds, window, cx));
        (frame, hitbox)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        (frame, hitbox): &mut (Frame, Hitbox),
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = self.view.read(cx);
        let focus = view.focus.clone();
        let style = view.cursor_style(window.modifiers().alt);
        window.set_cursor_style(style, hitbox);
        window.handle_input(&focus, TermInput(ElementInputHandler::new(bounds, self.view.clone())), cx);
        frame.paint(bounds, window, cx);

        let (v, hb) = (self.view.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && hb.is_hovered(window) {
                v.update(cx, |v, cx| v.mouse_down(e, window, cx));
                cx.stop_propagation();
            }
        });
        let (v, hb) = (self.view.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble {
                let hovered = hb.is_hovered(window);
                if hovered || v.read(cx).screen.pressed() || v.read(cx).hover.is_some() {
                    v.update(cx, |v, cx| v.mouse_move(e, hovered, cx));
                }
            }
        });
        let v = self.view.clone();
        window.on_mouse_event(move |e: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble && v.read(cx).screen.pressed() {
                v.update(cx, |v, cx| v.mouse_up(e, cx));
            }
        });
        let (v, hb) = (self.view.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &ScrollWheelEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && hb.is_hovered(window) {
                v.update(cx, |v, cx| v.scroll(e, cx));
                cx.stop_propagation();
            }
        });
    }
}
