//! the terminal without a window: alacritty's grid fed with the program's bytes, and what the person
//! does (keys, text, paste, the mouse, focus) turned into bytes for the program. What comes out is a
//! list of `Out`s, in order, for the view to act on

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::time::Instant;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Osc52, Term, TermMode};
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, NamedColor, Processor, Rgb, Color};
use gpui::{Keystroke, Modifiers};

use crate::keys::{self, KeyAction};
use crate::links;
use crate::mouse::{self, Button, Kind};
use crate::theme::{mix, Theme};

pub const SCROLLBACK: usize = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    /// bytes for the program
    Write(Vec<u8>),
    /// the person typed (or pasted): not the terminal's own answers and reports
    Typed { enter: bool },
    Title(String),
    Bell,
    /// put this on the clipboard (Cmd+C, or the program's OSC 52)
    Clipboard(String),
    /// a link was clicked
    Link(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum KeyResult {
    /// not the terminal's key
    Ignored,
    Done,
    /// Cmd+V: the view reads the clipboard and calls `paste`
    Paste,
}

/// a link under the pointer: where it goes and the cells it covers, a (viewport row, columns) a row
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub uri: String,
    pub spans: Vec<(usize, Range<usize>)>,
}

/// how a cell is drawn
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
}

#[derive(Clone, Default)]
pub(crate) struct Collect(Rc<RefCell<Vec<Event>>>);

impl EventListener for Collect {
    fn send_event(&self, ev: Event) {
        self.0.borrow_mut().push(ev);
    }
}

struct Press {
    button: Button,
    at: (usize, usize),
    moved: bool,
    report: bool,
    link: Option<String>,
}

pub struct Screen {
    term: Term<Collect>,
    parser: Processor,
    events: Collect,
    pub theme: Theme,
    size: WindowSize,
    out: Vec<Out>,
    press: Option<Press>,
    last_cell: Option<(usize, usize)>,
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        let events = Collect::default();
        let config = Config {
            scrolling_history: SCROLLBACK,
            // copies are taken; a read gets an empty answer, never the clipboard (see `pump`)
            osc52: Osc52::CopyPaste,
            default_cursor_style: CursorStyle { shape: CursorShape::Block, blinking: true },
            ..Config::default()
        };
        let term = Term::new(config, &TermSize::new(cols, rows), events.clone());
        let size = WindowSize { num_lines: rows as u16, num_cols: cols as u16, cell_width: 8, cell_height: 15 };
        Screen { term, parser: Processor::new(), events, theme: Theme::Dark, size, out: Vec::new(), press: None, last_cell: None }
    }

    /// bytes from the program
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
        self.pump();
    }

    /// a synchronized update (DEC 2026) holds its bytes back until it ends, or until this moment
    pub fn sync_deadline(&self) -> Option<Instant> {
        self.parser.sync_timeout().sync_timeout()
    }

    /// the update ran out of time: show what came so far
    pub fn end_sync(&mut self) {
        self.parser.stop_sync(&mut self.term);
        self.pump();
    }

    /// what happened since the last call, in order
    pub fn drain(&mut self) -> Vec<Out> {
        std::mem::take(&mut self.out)
    }

    fn pump(&mut self) {
        let evs = std::mem::take(&mut *self.events.0.borrow_mut());
        let p = self.theme.palette();
        for ev in evs {
            match ev {
                Event::PtyWrite(s) => self.out.push(Out::Write(s.into_bytes())),
                Event::ColorRequest(i, f) => self.out.push(Out::Write(f(p.answer(i, self.term.colors())).into_bytes())),
                Event::TextAreaSizeRequest(f) => self.out.push(Out::Write(f(self.size).into_bytes())),
                Event::ClipboardStore(_, s) => self.out.push(Out::Clipboard(s)),
                Event::ClipboardLoad(_, f) => self.out.push(Out::Write(f("").into_bytes())),
                Event::Title(s) => self.out.push(Out::Title(s)),
                Event::ResetTitle => self.out.push(Out::Title(String::new())),
                Event::Bell => self.out.push(Out::Bell),
                _ => {}
            }
        }
    }

    pub fn cols(&self) -> usize {
        self.term.columns()
    }
    pub fn rows(&self) -> usize {
        self.term.screen_lines()
    }
    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }
    pub fn is_bracketed_paste(&self) -> bool {
        self.mode().contains(TermMode::BRACKETED_PASTE)
    }
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }
    pub(crate) fn term(&self) -> &Term<Collect> {
        &self.term
    }

    /// the grid's new size, and the cell's in pixels (a program may ask for both); true if it changed
    pub fn resize(&mut self, cols: usize, rows: usize, cell_w: f32, cell_h: f32) -> bool {
        let (cols, rows) = (cols.max(2), rows.max(2));
        self.size.cell_width = cell_w as u16;
        self.size.cell_height = cell_h as u16;
        if (cols, rows) == (self.cols(), self.rows()) {
            return false;
        }
        self.size.num_cols = cols as u16;
        self.size.num_lines = rows as u16;
        self.term.resize(TermSize::new(cols, rows));
        true
    }

    // ---- what the person does

    fn typed(&mut self, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        self.term.scroll_display(Scroll::Bottom);
        self.term.selection = None;
        let enter = bytes.contains(&b'\r');
        self.out.push(Out::Write(bytes));
        self.out.push(Out::Typed { enter });
    }

    pub fn key(&mut self, ks: &Keystroke) -> KeyResult {
        match keys::encode(ks, self.mode().contains(TermMode::APP_CURSOR)) {
            None => return KeyResult::Ignored,
            Some(KeyAction::Send(b)) => self.typed(b),
            Some(KeyAction::Paste) => return KeyResult::Paste,
            Some(KeyAction::Copy) => {
                if let Some(s) = self.selection_text() {
                    self.out.push(Out::Clipboard(s));
                }
            }
            Some(KeyAction::SelectAll) => self.select_all(),
            Some(KeyAction::PageUp) => self.term.scroll_display(Scroll::PageUp),
            Some(KeyAction::PageDown) => self.term.scroll_display(Scroll::PageDown),
        }
        KeyResult::Done
    }

    /// text typed (through the input method: plain keys, dead keys, IME)
    pub fn text(&mut self, s: &str) {
        self.typed(s.as_bytes().to_vec());
    }

    /// bytes as if typed
    pub fn input(&mut self, b: &[u8]) {
        self.typed(b.to_vec());
    }

    /// a paste: new lines as Enter, bracketed when the program asked for it
    pub fn paste(&mut self, s: &str) {
        let s = s.replace("\r\n", "\r").replace('\n', "\r");
        let b = if self.is_bracketed_paste() {
            format!("\x1b[200~{}\x1b[201~", s.replace('\x1b', ""))
        } else {
            s
        };
        self.typed(b.into_bytes());
    }

    pub fn focus(&mut self, on: bool) {
        if self.mode().contains(TermMode::FOCUS_IN_OUT) {
            self.out.push(Out::Write(if on { b"\x1b[I".to_vec() } else { b"\x1b[O".to_vec() }));
        }
    }

    // ---- the mouse, at cells of the viewport (col, row)

    // the program gets the mouse when it asked for it, unless Option is held (that forces a selection,
    // as macOptionClickForcesSelection did)
    fn reporting(&self, mods: &Modifiers) -> bool {
        self.mode().intersects(TermMode::MOUSE_MODE) && !mods.alt
    }

    fn report(&mut self, b: Button, k: Kind, at: (usize, usize), mods: &Modifiers) {
        if let Some(r) = mouse::report(b, k, at.0, at.1, mods, self.mode()) {
            self.out.push(Out::Write(r));
        }
    }

    fn point(&self, at: (usize, usize)) -> Point {
        let row = at.1.min(self.rows() - 1) as i32 - self.display_offset() as i32;
        Point::new(Line(row), Column(at.0.min(self.cols() - 1)))
    }

    pub fn mouse_down(&mut self, button: Button, at: (usize, usize), side: Side, mods: &Modifiers, clicks: usize) {
        let report = self.reporting(mods);
        let link = if button == Button::Left { self.link_at(at).map(|l| l.uri) } else { None };
        if report {
            self.report(button, Kind::Press, at, mods);
        } else if button == Button::Left {
            let ty = match clicks {
                2 => SelectionType::Semantic,
                n if n >= 3 => SelectionType::Lines,
                _ => SelectionType::Simple,
            };
            self.term.selection = Some(Selection::new(ty, self.point(at), side));
        }
        self.press = Some(Press { button, at, moved: false, report, link });
        self.last_cell = Some(at);
    }

    pub fn mouse_move(&mut self, at: (usize, usize), side: Side, mods: &Modifiers) {
        let moved = self.last_cell != Some(at);
        self.last_cell = Some(at);
        let Some(p) = self.press.as_mut() else {
            if moved && self.mode().contains(TermMode::MOUSE_MOTION) && !mods.alt {
                self.report(Button::None, Kind::Motion, at, mods);
            }
            return;
        };
        p.moved |= p.at != at;
        let (button, report) = (p.button, p.report);
        if report {
            if moved {
                self.report(button, Kind::Motion, at, mods);
            }
        } else if button == Button::Left {
            let pt = self.point(at);
            if let Some(s) = self.term.selection.as_mut() {
                s.update(pt, side);
            }
        }
    }

    pub fn mouse_up(&mut self, button: Button, at: (usize, usize), mods: &Modifiers) {
        let Some(p) = self.press.take_if(|p| p.button == button) else { return };
        if p.report {
            self.report(button, Kind::Release, at, mods);
        } else if !p.moved && self.term.selection.as_ref().is_some_and(|s| s.ty == SelectionType::Simple) {
            self.term.selection = None; // a click, not a drag
        }
        if !p.moved
            && let Some(uri) = p.link
            && self.link_at(at).is_some_and(|l| l.uri == uri)
            && (uri.starts_with("https://") || uri.starts_with("http://"))
        {
            self.out.push(Out::Link(uri));
        }
    }

    /// is a selection being dragged out
    pub fn selecting(&self) -> bool {
        self.press.as_ref().is_some_and(|p| !p.report && p.button == Button::Left)
    }

    /// is a button down (a drag or a selection going on)
    pub fn pressed(&self) -> bool {
        self.press.is_some()
    }

    /// the wheel, `lines` up (positive) or down
    pub fn wheel(&mut self, lines: i32, at: (usize, usize), mods: &Modifiers) {
        if lines == 0 {
            return;
        }
        if self.reporting(mods) {
            let b = if lines > 0 { Button::WheelUp } else { Button::WheelDown };
            for _ in 0..lines.unsigned_abs() {
                self.report(b, Kind::Press, at, mods);
            }
        } else if self.mode().contains(TermMode::ALT_SCREEN) {
            // no scrollback there: the program gets arrows, as xterm does
            if self.mode().contains(TermMode::ALTERNATE_SCROLL) {
                let app = self.mode().contains(TermMode::APP_CURSOR);
                let k = match (lines > 0, app) {
                    (true, true) => "\x1bOA",
                    (true, false) => "\x1b[A",
                    (false, true) => "\x1bOB",
                    (false, false) => "\x1b[B",
                };
                self.out.push(Out::Write(k.repeat(lines.unsigned_abs() as usize).into_bytes()));
            }
        } else {
            self.term.scroll_display(Scroll::Delta(lines));
        }
    }

    // ---- selection

    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }

    pub fn has_selection(&self) -> bool {
        self.term.selection.as_ref().is_some_and(|s| !s.is_empty())
    }

    pub fn select_all(&mut self) {
        let start = Point::new(self.term.topmost_line(), Column(0));
        let end = Point::new(self.term.bottommost_line(), self.term.last_column());
        let mut s = Selection::new(SelectionType::Simple, start, Side::Left);
        s.update(end, Side::Right);
        self.term.selection = Some(s);
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    // ---- links: the program's own (OSC 8) and plain URLs in the text, also across a wrapped line

    pub fn link_at(&self, at: (usize, usize)) -> Option<Link> {
        if at.0 >= self.cols() || at.1 >= self.rows() {
            return None;
        }
        let p = self.point(at);
        let grid = self.term.grid();
        let off = self.display_offset() as i32;
        if let Some(h) = grid[p].hyperlink() {
            let mut spans: Vec<(usize, Range<usize>)> = Vec::new();
            for row in 0..self.rows() {
                let line = Line(row as i32 - off);
                for col in 0..self.cols() {
                    if grid[line][Column(col)].hyperlink().as_ref() == Some(&h) {
                        match spans.last_mut() {
                            Some((r, cs)) if *r == row && cs.end == col => cs.end += 1,
                            _ => spans.push((row, col..col + 1)),
                        }
                    }
                }
            }
            return Some(Link { uri: h.uri().to_string(), spans });
        }
        // the logical line: rows joined where a row wrapped
        let last = self.cols() - 1;
        let wraps = |l: Line| grid[l][Column(last)].flags.contains(Flags::WRAPLINE);
        let (mut top, mut bottom) = (p.line, p.line);
        while top > self.term.topmost_line() && wraps(top - 1) && p.line.0 - top.0 < 50 {
            top -= 1;
        }
        while bottom < self.term.bottommost_line() && wraps(bottom) && bottom.0 - p.line.0 < 50 {
            bottom += 1;
        }
        let mut chars = Vec::new();
        let mut points = Vec::new();
        let mut l = top;
        while l <= bottom {
            for col in 0..self.cols() {
                let cell = &grid[l][Column(col)];
                if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                    continue;
                }
                chars.push(cell.c);
                points.push(Point::new(l, Column(col)));
            }
            l += 1;
        }
        let i = points.iter().position(|q| q.line == p.line && q.column >= p.column)?;
        let i = if points[i].column > p.column { i.checked_sub(1)? } else { i };
        let r = links::urls(&chars).into_iter().find(|r| r.contains(&i))?;
        let uri: String = chars[r.clone()].iter().collect();
        let mut spans: Vec<(usize, Range<usize>)> = Vec::new();
        for q in &points[r] {
            let row = q.line.0 + off;
            if row < 0 || row as usize >= self.rows() {
                continue;
            }
            let (row, col) = (row as usize, q.column.0);
            let w = if grid[q.line][q.column].flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 };
            match spans.last_mut() {
                Some((rr, cs)) if *rr == row && cs.end == col => cs.end += w,
                _ => spans.push((row, col..col + w)),
            }
        }
        Some(Link { uri, spans })
    }

    // ---- what is on the screen

    /// a viewport row's text, without its trailing blanks
    pub fn row_text(&self, row: usize) -> String {
        let line = Line(row as i32 - self.display_offset() as i32);
        let mut s = String::new();
        for col in 0..self.cols() {
            let cell = &self.term.grid()[line][Column(col)];
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            s.push(cell.c);
            if let Some(z) = cell.zerowidth() {
                s.extend(z);
            }
        }
        s.trim_end().to_string()
    }

    /// the whole viewport as text, a line a row
    pub fn contents(&self) -> String {
        (0..self.rows()).map(|r| self.row_text(r)).collect::<Vec<_>>().join("\n")
    }

    /// the cursor: its cell and shape, if it shows (not while scrolled back or hidden)
    pub fn cursor(&self) -> Option<((usize, usize), CursorShape, bool)> {
        let c = self.term.renderable_content().cursor;
        let row = c.point.line.0 + self.display_offset() as i32;
        if c.shape == CursorShape::Hidden || row < 0 || row as usize >= self.rows() {
            return None;
        }
        Some(((c.point.column.0, row as usize), c.shape, self.term.cursor_style().blinking))
    }

    /// the cursor's cell, shown or not (where an input method composes)
    pub fn cursor_cell(&self) -> Option<((usize, usize), CursorShape, bool)> {
        let c = self.term.grid().cursor.point;
        let row = c.line.0 + self.display_offset() as i32;
        (row >= 0 && (row as usize) < self.rows()).then(|| ((c.column.0, row as usize), CursorShape::Block, false))
    }

    /// a viewport cell
    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        &self.term.grid()[Line(row as i32 - self.display_offset() as i32)][Column(col)]
    }

    /// how a cell is drawn: the theme's colours, bold ones bright (as xterm.js draws them), inverse
    /// swapped, dim ones halfway to the background
    pub fn style(&self, cell: &Cell) -> Style {
        let p = self.theme.palette();
        let colors = self.term.colors();
        let f = cell.flags;
        let fgc = match cell.fg {
            Color::Named(n) if f.contains(Flags::BOLD) && (n as usize) < 8 => Color::Named(n.to_bright()),
            Color::Indexed(i) if f.contains(Flags::BOLD) && i < 8 => Color::Indexed(i + 8),
            c => c,
        };
        let mut fg = p.resolve(fgc, colors);
        let mut bg = p.resolve(cell.bg, colors);
        if f.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if f.contains(Flags::DIM) {
            fg = mix(fg, bg, 0.5);
        }
        if f.contains(Flags::HIDDEN) {
            fg = bg;
        }
        Style {
            fg,
            bg,
            bold: f.contains(Flags::BOLD),
            italic: f.contains(Flags::ITALIC),
            underline: f.intersects(Flags::ALL_UNDERLINES),
            strike: f.contains(Flags::STRIKEOUT),
        }
    }

    pub fn background(&self) -> Rgb {
        self.theme.palette().resolve(Color::Named(NamedColor::Background), self.term.colors())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::rgb;

    const NO: Modifiers = Modifiers { control: false, alt: false, shift: false, platform: false, function: false };
    const ALT: Modifiers = Modifiers { control: false, alt: true, shift: false, platform: false, function: false };

    fn screen(s: &str) -> Screen {
        let mut t = Screen::new(40, 10);
        t.feed(s.as_bytes());
        t
    }
    fn writes(t: &mut Screen) -> Vec<String> {
        t.drain()
            .into_iter()
            .filter_map(|o| if let Out::Write(b) = o { Some(String::from_utf8_lossy(&b).into_owned()) } else { None })
            .collect()
    }

    #[test]
    fn text_colours_cursor() {
        let t = screen("hello\r\n\x1b[31mred\x1b[0m \x1b[1;32mB\x1b[0m\x1b[38;2;1;2;3mX\x1b[48;5;196mY\x1b[0m\x1b[7mI");
        assert_eq!(t.row_text(0), "hello");
        assert_eq!(t.row_text(1), "red BXYI");
        let p = Theme::Dark.palette();
        assert_eq!(t.style(t.cell(0, 1)).fg, rgb(p.ansi[1]));
        assert_eq!(t.style(t.cell(0, 0)).fg, rgb(p.foreground));
        assert_eq!(t.style(t.cell(0, 0)).bg, rgb(p.background));
        let b = t.style(t.cell(4, 1));
        assert!(b.bold);
        assert_eq!(b.fg, rgb(p.ansi[10]), "bold is bright");
        assert_eq!(t.style(t.cell(5, 1)).fg, Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(t.style(t.cell(6, 1)).bg, Rgb { r: 255, g: 0, b: 0 });
        let inv = t.style(t.cell(7, 1));
        assert_eq!((inv.fg, inv.bg), (rgb(p.background), rgb(p.foreground)));
        assert_eq!(t.cursor().map(|c| c.0), Some((8, 1)));
    }

    #[test]
    fn wide_chars_and_light_theme() {
        let mut t = screen("日本x");
        assert_eq!(t.row_text(0), "日本x");
        assert!(t.cell(0, 0).flags.contains(Flags::WIDE_CHAR));
        assert!(t.cell(1, 0).flags.contains(Flags::WIDE_CHAR_SPACER));
        assert_eq!(t.cell(4, 0).c, 'x');
        t.theme = Theme::Light;
        assert_eq!(t.style(t.cell(4, 0)).fg, rgb(0x140f1e));
    }

    #[test]
    fn answers_are_not_typing() {
        let mut t = screen("\x1b[2;3H\x1b[6n");
        let out = t.drain();
        assert_eq!(out, vec![Out::Write(b"\x1b[2;3R".to_vec())]);
        // the background colour, asked by Claude Code to pick its theme
        t.feed(b"\x1b]11;?\x07");
        assert_eq!(writes(&mut t), ["\x1b]11;rgb:1a1a/1616/2626\x07"]);
        t.feed(b"\x1b[?1004h");
        t.focus(true);
        t.focus(false);
        assert_eq!(t.drain(), vec![Out::Write(b"\x1b[I".to_vec()), Out::Write(b"\x1b[O".to_vec())]);
        t.feed(b"\x1b[?1000h\x1b[?1006h");
        t.mouse_down(Button::Left, (2, 3), Side::Left, &NO, 1);
        t.mouse_up(Button::Left, (2, 3), &NO);
        assert!(t.drain().iter().all(|o| matches!(o, Out::Write(_))));
    }

    #[test]
    fn typing() {
        let mut t = screen("");
        t.key(&Keystroke::parse("enter").unwrap());
        t.text("a");
        assert_eq!(
            t.drain(),
            vec![
                Out::Write(b"\r".to_vec()),
                Out::Typed { enter: true },
                Out::Write(b"a".to_vec()),
                Out::Typed { enter: false }
            ]
        );
        assert_eq!(t.key(&Keystroke::parse("a").unwrap()), KeyResult::Ignored);
        assert_eq!(t.key(&Keystroke::parse("cmd-v").unwrap()), KeyResult::Paste);
        t.paste("a\nb\r\nc");
        assert_eq!(writes(&mut t), ["a\rb\rc"]);
        t.feed(b"\x1b[?2004h");
        assert!(t.is_bracketed_paste());
        t.paste("x\n\x1by");
        assert_eq!(writes(&mut t), ["\x1b[200~x\ry\x1b[201~"]);
    }

    #[test]
    fn osc52_writes_but_never_reads() {
        let mut t = screen("\x1b]52;c;aGVsbG8=\x07");
        assert_eq!(t.drain(), vec![Out::Clipboard("hello".into())]);
        t.feed(b"\x1b]52;c;?\x07");
        assert_eq!(t.drain(), vec![Out::Write(b"\x1b]52;c;\x07".to_vec())]);
    }

    #[test]
    fn title_and_bell() {
        let mut t = screen("\x1b]0;claude\x07\x07");
        assert_eq!(t.drain(), vec![Out::Title("claude".into()), Out::Bell]);
    }

    #[test]
    fn selecting() {
        let mut t = screen("hello world\r\nsecond");
        t.mouse_down(Button::Left, (0, 0), Side::Left, &NO, 1);
        t.mouse_move((4, 0), Side::Right, &NO);
        t.mouse_up(Button::Left, (4, 0), &NO);
        assert_eq!(t.selection_text().as_deref(), Some("hello"));
        t.key(&Keystroke::parse("cmd-c").unwrap());
        assert_eq!(t.drain(), vec![Out::Clipboard("hello".into())]);
        t.mouse_down(Button::Left, (8, 0), Side::Left, &NO, 2);
        t.mouse_up(Button::Left, (8, 0), &NO);
        assert_eq!(t.selection_text().as_deref(), Some("world"));
        t.mouse_down(Button::Left, (8, 0), Side::Left, &NO, 3);
        t.mouse_up(Button::Left, (8, 0), &NO);
        assert_eq!(t.selection_text().as_deref(), Some("hello world\n"));
        // a plain click clears it
        t.mouse_down(Button::Left, (3, 1), Side::Left, &NO, 1);
        t.mouse_up(Button::Left, (3, 1), &NO);
        assert!(!t.has_selection());
        t.select_all();
        assert_eq!(t.selection_text().unwrap().trim_end(), "hello world\nsecond");
        t.text("x");
        assert!(!t.has_selection(), "typing clears the selection");
    }

    #[test]
    fn option_forces_a_selection() {
        let mut t = screen("hello world\x1b[?1002h\x1b[?1006h");
        t.mouse_down(Button::Left, (0, 0), Side::Left, &NO, 1);
        t.mouse_move((4, 0), Side::Right, &NO);
        t.mouse_up(Button::Left, (4, 0), &NO);
        assert_eq!(writes(&mut t), ["\x1b[<0;1;1M", "\x1b[<32;5;1M", "\x1b[<0;5;1m"]);
        assert!(!t.has_selection());
        t.mouse_down(Button::Left, (0, 0), Side::Left, &ALT, 1);
        t.mouse_move((4, 0), Side::Right, &ALT);
        t.mouse_up(Button::Left, (4, 0), &ALT);
        assert!(writes(&mut t).is_empty());
        assert_eq!(t.selection_text().as_deref(), Some("hello"));
    }

    #[test]
    fn the_wheel() {
        let mut t = Screen::new(20, 5);
        for i in 0..30 {
            t.feed(format!("line {i}\r\n").as_bytes());
        }
        t.wheel(3, (0, 0), &NO);
        assert_eq!(t.display_offset(), 3);
        assert!(writes(&mut t).is_empty());
        t.text("x"); // typing goes back to the bottom
        assert_eq!(t.display_offset(), 0);
        t.drain();
        t.feed(b"\x1b[?1049h");
        t.wheel(2, (0, 0), &NO);
        t.wheel(-1, (0, 0), &NO);
        assert_eq!(writes(&mut t), ["\x1b[A\x1b[A", "\x1b[B"]);
        assert_eq!(t.display_offset(), 0);
        t.feed(b"\x1b[?1000h\x1b[?1006h");
        t.wheel(-2, (3, 4), &NO);
        assert_eq!(writes(&mut t), ["\x1b[<65;4;5M", "\x1b[<65;4;5M"]);
    }

    #[test]
    fn links() {
        let mut t = screen("go to https://example.com/x now\r\n\x1b]8;;https://a.b/\x1b\\link\x1b]8;;\x1b\\ plain");
        let l = t.link_at((8, 0)).unwrap();
        assert_eq!(l.uri, "https://example.com/x");
        assert_eq!(l.spans, vec![(0, 6..27)]);
        assert_eq!(t.link_at((3, 0)), None);
        let l = t.link_at((1, 1)).unwrap();
        assert_eq!(l.uri, "https://a.b/");
        assert_eq!(l.spans, vec![(1, 0..4)]);
        assert_eq!(t.link_at((6, 1)), None);
        // a click opens it, with the mouse captured too; a drag does not
        t.feed(b"\x1b[?1000h\x1b[?1006h");
        t.drain();
        t.mouse_down(Button::Left, (2, 1), Side::Left, &NO, 1);
        t.mouse_up(Button::Left, (2, 1), &NO);
        assert!(t.drain().contains(&Out::Link("https://a.b/".into())));
        t.mouse_down(Button::Left, (10, 0), Side::Left, &NO, 1);
        t.mouse_move((12, 0), Side::Left, &NO);
        t.mouse_up(Button::Left, (12, 0), &NO);
        assert!(!t.drain().iter().any(|o| matches!(o, Out::Link(_))));
    }

    #[test]
    fn a_wrapped_url() {
        let mut t = Screen::new(20, 5);
        t.feed(b"see https://example.com/a/long/path ok");
        let l = t.link_at((2, 1)).unwrap();
        assert_eq!(l.uri, "https://example.com/a/long/path");
        assert_eq!(l.spans, vec![(0, 4..20), (1, 0..15)]);
        assert_eq!(t.link_at((6, 0)).unwrap().uri, l.uri);
    }

    #[test]
    fn synchronized_updates() {
        let mut t = screen("\x1b[?2026habc");
        assert_eq!(t.row_text(0), "");
        assert!(t.sync_deadline().is_some());
        t.end_sync();
        assert_eq!(t.row_text(0), "abc");
        t.feed(b"\x1b[?2026hd\x1b[?2026l");
        assert_eq!(t.row_text(0), "abcd");
        assert!(t.sync_deadline().is_none());
    }

    #[test]
    fn resizing() {
        let mut t = screen("abc");
        assert!(!t.resize(40, 10, 8.0, 15.0));
        assert!(t.resize(80, 24, 8.0, 15.0));
        assert_eq!((t.cols(), t.rows()), (80, 24));
        t.drain();
        t.feed(b"\x1b[18t"); // the text area in characters
        assert_eq!(writes(&mut t), ["\x1b[8;24;80t"]);
    }
}
