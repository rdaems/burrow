//! The file beside a terminal, editable (index.html's CodeMirror, `mountEditor`): GPUI has no editor, so this is the
//! least that is honest to it. In the terminal's font, a pixel grid of one width: line numbers, the line you are on
//! lit, the caret and the selection, typing (through the platform's input handler, so dead keys and the character
//! palette work), the Mac's keys for moving and selecting (⌥ by word, ⌘ to the line's or the file's ends), undo and
//! redo, copy, cut and paste, Tab and Enter keeping the indentation, the wheel scrolling both ways, a click, a drag, a
//! double click for a word. The colours of what the code is: syntax.rs. ⌘S asks the tile to save.
//!   Not here (CodeMirror has them): folding, search, bracket matching and closing, several selections, lines
//! wrapped, a language's own indentation.

use crate::kit::{self, col};
use crate::syntax::{self, Lang, Tok};
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, Font, FontFeatures, FontStyle, FontWeight, GlobalElementId, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollWheelEvent, ShapedLine, SharedString, Style, TextRun, UTF16Selection, Window, div, fill, point,
    prelude::*, px, relative, rgb, rgba, size,
};
use std::ops::Range;
use std::time::{Duration, Instant};

const FONT_SIZE: f32 = 13.0;
/// 1.5 lines, as the page's `.cm-scroller`
const LINE: f32 = 19.5;
/// the content's padding at the top (`.cm-content`), the text's left of the gutter
const TOP: f32 = 10.0;
const GAP: f32 = 14.0;
/// a tab is this many columns wide
const TAB: usize = 4;

// ---- the text and what is selected, with no pixels: byte offsets on char boundaries

/// one change, to undo: what was at `at` (`gone`) and what took its place (`put`), the selection before and after
#[derive(Clone, Debug)]
struct Change {
    at: usize,
    gone: String,
    put: String,
    before: (Range<usize>, bool),
    after: (Range<usize>, bool),
    /// typing or deleting a character, when: run together with the next of its kind
    kind: Option<(char, Instant)>,
}

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub text: String,
    pub sel: Range<usize>,
    /// the caret is at the selection's start
    pub reversed: bool,
    /// where each line starts
    starts: Vec<usize>,
    undo: Vec<Change>,
    redo: Vec<Change>,
    /// the column ↑ and ↓ keep to
    goal: Option<usize>,
}

impl Doc {
    pub fn new(text: &str) -> Doc {
        let mut d = Doc { text: text.to_string(), ..Default::default() };
        d.index();
        d
    }
    fn index(&mut self) {
        self.starts = std::iter::once(0).chain(self.text.match_indices('\n').map(|(i, _)| i + 1)).collect();
    }
    pub fn lines(&self) -> usize {
        self.starts.len()
    }
    pub fn line_of(&self, i: usize) -> usize {
        self.starts.partition_point(|&s| s <= i).saturating_sub(1)
    }
    /// a line's bytes, without its newline
    pub fn line_range(&self, l: usize) -> Range<usize> {
        let a = self.starts[l.min(self.starts.len() - 1)];
        let b = self.starts.get(l + 1).map(|s| s - 1).unwrap_or(self.text.len());
        a..b
    }
    pub fn line(&self, l: usize) -> &str {
        &self.text[self.line_range(l)]
    }
    pub fn caret(&self) -> usize {
        if self.reversed { self.sel.start } else { self.sel.end }
    }
    pub fn selected(&self) -> &str {
        &self.text[self.sel.clone()]
    }

    /// the column an offset is drawn at (tabs to the next stop)
    pub fn col_of(&self, i: usize) -> usize {
        let r = self.line_range(self.line_of(i));
        col_in(&self.text[r.start..i.min(r.end).max(r.start)])
    }
    /// the offset nearest a column on a line
    pub fn at_col(&self, l: usize, want: usize) -> usize {
        let r = self.line_range(l);
        let mut c = 0;
        for (j, ch) in self.text[r.clone()].char_indices() {
            let w = if ch == '\t' { TAB - c % TAB } else { 1 };
            if c + w / 2 >= want || c + w > want {
                return r.start + j + if c + w / 2 < want { ch.len_utf8() } else { 0 };
            }
            c += w;
        }
        r.end
    }

    fn prev(&self, i: usize) -> usize {
        self.text[..i].char_indices().next_back().map(|(j, _)| j).unwrap_or(0)
    }
    fn next(&self, i: usize) -> usize {
        self.text[i..].chars().next().map(|c| i + c.len_utf8()).unwrap_or(self.text.len())
    }
    /// a word back or on (⌥←, ⌥→): over spaces, then a word or a run of punctuation
    fn word_back(&self, i: usize) -> usize {
        let t = &self.text[..i];
        let t2 = t.trim_end_matches([' ', '\t']);
        let Some(c) = t2.chars().next_back() else { return 0 };
        if c == '\n' {
            return if t2.len() == t.len() { t2.len() - 1 } else { t2.len() };
        }
        let same = |ch: char| if is_word(c) { is_word(ch) } else { !is_word(ch) && !ch.is_whitespace() };
        t2.char_indices().rev().find(|(_, ch)| !same(*ch)).map(|(j, ch)| j + ch.len_utf8()).unwrap_or(0)
    }
    fn word_on(&self, i: usize) -> usize {
        let t = &self.text[i..];
        let skip = t.len() - t.trim_start_matches([' ', '\t']).len();
        let t2 = &t[skip..];
        let Some(c) = t2.chars().next() else { return self.text.len() };
        if c == '\n' {
            return i + skip + if skip == 0 { 1 } else { 0 };
        }
        let same = |ch: char| if is_word(c) { is_word(ch) } else { !is_word(ch) && !ch.is_whitespace() };
        i + skip + t2.char_indices().find(|(_, ch)| !same(*ch)).map(|(j, _)| j).unwrap_or(t2.len())
    }
    /// the word round an offset (a double click)
    pub fn word_at(&self, i: usize) -> Range<usize> {
        let a = self.text[..i].char_indices().rev().find(|(_, c)| !is_word(*c)).map(|(j, c)| j + c.len_utf8()).unwrap_or(0);
        let b = self.text[i..].char_indices().find(|(_, c)| !is_word(*c)).map(|(j, _)| i + j).unwrap_or(self.text.len());
        if a == b { i..self.next(i) } else { a..b }
    }

    pub fn move_to(&mut self, i: usize) {
        let i = i.min(self.text.len());
        self.sel = i..i;
        self.reversed = false;
    }
    /// the selection's moving end to `i`
    pub fn select_to(&mut self, i: usize) {
        let i = i.min(self.text.len());
        if self.reversed { self.sel.start = i } else { self.sel.end = i }
        if self.sel.end < self.sel.start {
            self.reversed = !self.reversed;
            self.sel = self.sel.end..self.sel.start;
        }
    }
    /// the caret to `i`, or the selection's end there (shift)
    fn go(&mut self, i: usize, shift: bool) {
        if shift { self.select_to(i) } else { self.move_to(i) }
    }

    /// a key that moves the caret: left, right, up, down, home, end, top, bottom, pageup, pagedown (`page` lines);
    /// `word`: by words (⌥)
    pub fn travel(&mut self, how: &str, shift: bool, word: bool, page: usize) {
        let c = self.caret();
        if !matches!(how, "up" | "down" | "pageup" | "pagedown") {
            self.goal = None;
        }
        let to = match how {
            "left" if !shift && !self.sel.is_empty() && !word => self.sel.start,
            "right" if !shift && !self.sel.is_empty() && !word => self.sel.end,
            "left" if word => self.word_back(c),
            "right" if word => self.word_on(c),
            "left" => self.prev(c),
            "right" => self.next(c),
            "home" => {
                // to the line's first character, then to its very start
                let r = self.line_range(self.line_of(c));
                let first = r.start + (self.text[r.clone()].len() - self.text[r.clone()].trim_start().len());
                if c == first { r.start } else { first }
            }
            "end" => self.line_range(self.line_of(c)).end,
            "top" => 0,
            "bottom" => self.text.len(),
            "up" | "down" | "pageup" | "pagedown" => {
                let goal = *self.goal.get_or_insert(self.col_of(c));
                let l = self.line_of(c);
                let n = if how.starts_with("page") { page.max(1) } else { 1 };
                let l2 = if how.ends_with("up") { l.checked_sub(n) } else { Some(l + n).filter(|x| *x < self.lines()) };
                match l2 {
                    Some(l2) => self.at_col(l2, goal),
                    None if how.ends_with("up") => 0,
                    None => self.text.len(),
                }
            }
            _ => c,
        };
        self.go(to, shift);
    }

    pub fn select_all(&mut self) {
        self.sel = 0..self.text.len();
        self.reversed = false;
    }

    /// `r` replaced by `s`, the caret after it; one step to undo (typing runs together)
    pub fn replace(&mut self, r: Range<usize>, s: &str, kind: Option<char>) {
        let (a, b) = (r.start.min(self.text.len()), r.end.min(self.text.len()));
        let before = (self.sel.clone(), self.reversed);
        let gone = self.text[a..b].to_string();
        self.text.replace_range(a..b, s);
        self.index();
        self.move_to(a + s.len());
        self.goal = None;
        self.redo.clear();
        let now = Instant::now();
        let after = (self.sel.clone(), self.reversed);
        // a character typed right after the last one (or deleted right before it), within a second: the same step
        if let (Some(k), Some(last)) = (kind, self.undo.last_mut())
            && let Some((lk, t)) = last.kind
            && lk == k
            && now.duration_since(t) < Duration::from_secs(1)
        {
            if k == 'i' && gone.is_empty() && last.at + last.put.len() == a {
                last.put.push_str(s);
                last.after = after;
                last.kind = Some((k, now));
                return;
            }
            if k == 'd' && s.is_empty() && last.put.is_empty() && b == last.at {
                last.at = a;
                last.gone.insert_str(0, &gone);
                last.after = after;
                last.kind = Some((k, now));
                return;
            }
        }
        self.undo.push(Change { at: a, gone, put: s.to_string(), before, after, kind: kind.map(|k| (k, now)) });
    }
    /// what is selected replaced by `s` (typing, a paste)
    pub fn insert(&mut self, s: &str) {
        let kind = (s.chars().count() == 1).then_some('i');
        self.replace(self.sel.clone(), s, kind);
    }
    pub fn backspace(&mut self, word: bool, line: bool) {
        if self.sel.is_empty() {
            let c = self.caret();
            // (⌘⌫ to the line's start; at its start, the line break before it)
            let ls = self.line_range(self.line_of(c)).start;
            let a = if line { if c == ls { self.prev(c) } else { ls } } else if word { self.word_back(c) } else { self.prev(c) };
            self.sel = a..c;
        }
        let k = (self.sel.len() <= 4).then_some('d');
        self.replace(self.sel.clone(), "", k);
    }
    pub fn delete(&mut self, word: bool) {
        if self.sel.is_empty() {
            let c = self.caret();
            self.sel = c..if word { self.word_on(c) } else { self.next(c) };
        }
        self.replace(self.sel.clone(), "", None);
    }
    /// Enter: a new line, as indented as this one
    pub fn newline(&mut self) {
        let r = self.line_range(self.line_of(self.sel.start));
        let line = &self.text[r.start..self.sel.start.max(r.start)];
        let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        self.replace(self.sel.clone(), &format!("\n{indent}"), None);
    }
    /// Tab: the file's indentation (a tab, or as many spaces as its lines are indented by)
    pub fn indent_unit(&self) -> String {
        let mut least = usize::MAX;
        for l in self.text.lines().take(2000) {
            if l.starts_with('\t') {
                return "\t".into();
            }
            let n = l.len() - l.trim_start_matches(' ').len();
            if n > 0 && n < least && l.len() > n {
                least = n;
            }
        }
        " ".repeat(if least == usize::MAX || least > 8 { 2 } else { least })
    }

    pub fn undo(&mut self) -> bool {
        let Some(c) = self.undo.pop() else { return false };
        self.text.replace_range(c.at..c.at + c.put.len(), &c.gone);
        self.index();
        (self.sel, self.reversed) = c.before.clone();
        self.clamp();
        self.redo.push(Change { kind: None, ..c });
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(c) = self.redo.pop() else { return false };
        self.text.replace_range(c.at..c.at + c.gone.len(), &c.put);
        self.index();
        (self.sel, self.reversed) = c.after.clone();
        self.clamp();
        self.undo.push(c);
        true
    }
    /// all of it replaced (what came in from disk), where you were kept as near as can be; undo goes back past it
    pub fn reload(&mut self, text: &str) {
        let c = self.caret();
        let n = self.text.len();
        self.replace(0..n, text, None);
        let c = c.min(self.text.len());
        let c = (0..=c).rev().find(|&i| self.text.is_char_boundary(i)).unwrap_or(0);
        self.move_to(c);
    }
    fn clamp(&mut self) {
        let n = self.text.len();
        let fix = |i: usize, t: &str| (0..=i.min(n)).rev().find(|&j| t.is_char_boundary(j)).unwrap_or(0);
        self.sel = fix(self.sel.start, &self.text)..fix(self.sel.end, &self.text);
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// how many columns a stretch of a line takes, tabs to the next stop
fn col_in(s: &str) -> usize {
    let mut c = 0;
    for ch in s.chars() {
        c += if ch == '\t' { TAB - c % TAB } else { 1 };
    }
    c
}

/// a line as it is drawn: tabs as spaces to the next stop; and, for each byte of the line, where it went in that
fn expand(line: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(line.len());
    let mut map = Vec::with_capacity(line.len() + 1);
    let mut c = 0;
    for ch in line.chars() {
        for _ in 0..ch.len_utf8() {
            map.push(out.len());
        }
        if ch == '\t' {
            let w = TAB - c % TAB;
            out.extend(std::iter::repeat_n(' ', w));
            c += w;
        } else {
            out.push(ch);
            c += 1;
        }
    }
    map.push(out.len());
    (out, map)
}

// ---- the view

/// what the editor tells the tile it is in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorEvent {
    /// the text changed (you typed, pasted, undid; or it came in from disk)
    Changed,
    /// a key typed or a deletion: you, typing (the room shows it)
    Typed,
    /// ⌘S
    Save,
    /// it was clicked into (it has the keyboard)
    Focused,
}

pub struct Editor {
    pub focus: FocusHandle,
    pub doc: Doc,
    lang: Option<Lang>,
    /// per line: it starts inside a block comment (kept until the text changes)
    blocks: Option<Vec<bool>>,
    marked: Option<Range<usize>>,
    /// how far down and across it is scrolled, in pixels
    pub scroll: Point<Pixels>,
    /// a column's width and the text's bounds, as last drawn
    cw: Pixels,
    bounds: Option<Bounds<Pixels>>,
    gutter: Pixels,
    selecting: bool,
    /// bring the caret into view at the next drawing
    reveal: bool,
}

impl EventEmitter<EditorEvent> for Editor {}

fn font(bold: bool, italic: bool) -> Font {
    Font {
        family: kit::MONO.into(),
        features: FontFeatures::disable_ligatures(),
        fallbacks: Some(gpui::FontFallbacks::from_fonts(vec!["Menlo".into()])),
        weight: if bold { FontWeight::BOLD } else { FontWeight::NORMAL },
        style: if italic { FontStyle::Italic } else { FontStyle::Normal },
    }
}

impl Editor {
    pub fn new(text: &str, path: &str, cx: &mut Context<Editor>) -> Editor {
        let focus = cx.focus_handle();
        Editor {
            focus,
            doc: Doc::new(text),
            lang: syntax::lang_of(path),
            blocks: None,
            marked: None,
            scroll: point(px(0.), px(0.)),
            cw: px(7.8),
            bounds: None,
            gutter: px(40.),
            selecting: false,
            reveal: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.doc.text
    }
    /// what is on disk now, taken in (nothing unsaved): where you were stays
    pub fn reload(&mut self, text: &str, cx: &mut Context<Editor>) {
        self.doc.reload(text);
        self.blocks = None;
        cx.notify();
    }
    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus);
    }
    /// keys typed, as the platform hands them over (BURROW_OPEN's type:, to look at it)
    pub fn type_in(&mut self, text: &str, window: &mut Window, cx: &mut Context<Editor>) {
        window.focus(&self.focus);
        for ch in text.chars() {
            self.replace_text_in_range(None, &ch.to_string(), window, cx);
        }
    }

    fn changed(&mut self, typed: bool, cx: &mut Context<Editor>) {
        self.blocks = None;
        self.marked = None;
        self.reveal = true;
        cx.emit(EditorEvent::Changed);
        if typed {
            cx.emit(EditorEvent::Typed);
        }
        cx.notify();
    }
    fn moved(&mut self, cx: &mut Context<Editor>) {
        self.reveal = true;
        cx.notify();
    }

    /// how many lines a page is
    fn page(&self) -> usize {
        self.bounds.map(|b| (f32::from(b.size.height) / LINE) as usize).unwrap_or(20).saturating_sub(2)
    }

    fn key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Editor>) {
        let k = &ev.keystroke;
        let (shift, cmd, alt, ctrl) = (k.modifiers.shift, k.modifiers.platform, k.modifiers.alt, k.modifiers.control);
        let page = self.page();
        let d = &mut self.doc;
        match k.key.as_str() {
            "s" if cmd => cx.emit(EditorEvent::Save),
            "z" if cmd && shift => {
                if d.redo() {
                    self.changed(false, cx)
                }
            }
            "z" if cmd => {
                if d.undo() {
                    self.changed(false, cx)
                }
            }
            "y" if cmd => {
                if d.redo() {
                    self.changed(false, cx)
                }
            }
            "a" if cmd => {
                d.select_all();
                cx.notify()
            }
            "c" | "x" if cmd => {
                if !d.sel.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(d.selected().to_string()));
                    if k.key == "x" {
                        d.insert("");
                        self.changed(true, cx);
                    }
                }
            }
            "v" if cmd => {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    self.doc.insert(&t);
                    self.changed(true, cx);
                }
            }
            "backspace" => {
                d.backspace(alt || ctrl, cmd);
                self.changed(true, cx)
            }
            "delete" => {
                d.delete(alt);
                self.changed(true, cx)
            }
            "enter" if !cmd => {
                d.newline();
                self.changed(true, cx)
            }
            "tab" if !cmd && !ctrl => {
                if shift {
                    // the line's indentation, one step less
                    let l = d.line_of(d.caret());
                    let r = d.line_range(l);
                    let unit = d.indent_unit();
                    let line = d.line(l).to_string();
                    let n = if line.starts_with('\t') { 1 } else { line.len() - line.trim_start_matches(' ').len() }.min(unit.len().max(1));
                    if n > 0 {
                        let keep = d.sel.clone();
                        d.replace(r.start..r.start + n, "", None);
                        let back = |i: usize| if i >= r.start + n { i - n } else { r.start.max(i.min(r.start)) };
                        d.sel = back(keep.start)..back(keep.end);
                        self.changed(true, cx);
                    }
                } else {
                    let u = d.indent_unit();
                    d.insert(&u);
                    self.changed(true, cx);
                }
            }
            "left" | "right" | "up" | "down" | "home" | "end" | "pageup" | "pagedown" => {
                let how = match (k.key.as_str(), cmd) {
                    ("left", true) => "home",
                    ("right", true) => "end",
                    ("up", true) => "top",
                    ("down", true) => "bottom",
                    (h, _) => h,
                };
                d.travel(how, shift, alt, page);
                self.moved(cx);
            }
            _ => return, // typed (the input handler), or not the editor's
        }
        cx.stop_propagation();
    }

    /// the offset nearest a point in the window
    fn index_at(&self, p: Point<Pixels>) -> usize {
        let Some(b) = self.bounds else { return 0 };
        let y = f32::from(p.y - b.top() + self.scroll.y) - TOP;
        let l = ((y / LINE).floor().max(0.0) as usize).min(self.doc.lines() - 1);
        let x = f32::from(p.x - b.left() - self.gutter + self.scroll.x).max(0.0);
        let col = (x / f32::from(self.cw)).round() as usize;
        self.doc.at_col(l, col)
    }

    fn mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Editor>) {
        window.focus(&self.focus);
        cx.emit(EditorEvent::Focused);
        let i = self.index_at(ev.position);
        if ev.click_count == 2 {
            let w = self.doc.word_at(i);
            self.doc.sel = w;
            self.doc.reversed = false;
        } else if ev.click_count >= 3 {
            let l = self.doc.line_of(i);
            let r = self.doc.line_range(l);
            self.doc.sel = r.start..(r.end + 1).min(self.doc.text.len());
            self.doc.reversed = false;
        } else if ev.modifiers.shift {
            self.doc.select_to(i);
        } else {
            self.doc.move_to(i);
        }
        self.selecting = true;
        cx.notify();
    }
    fn mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Editor>) {
        if !self.selecting || ev.pressed_button != Some(MouseButton::Left) {
            self.selecting = false;
            return;
        }
        let i = self.index_at(ev.position);
        self.doc.select_to(i);
        self.reveal = true;
        cx.notify();
    }
    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Editor>) {
        self.selecting = false;
    }
    fn wheel(&mut self, ev: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Editor>) {
        let d = ev.delta.pixel_delta(px(LINE));
        self.scroll.y -= d.y;
        self.scroll.x -= d.x;
        self.clamp_scroll(window);
        cx.notify();
    }
    fn clamp_scroll(&mut self, _window: &Window) {
        let Some(b) = self.bounds else { return };
        let h = TOP * 2.0 + self.doc.lines() as f32 * LINE;
        let max_y = (h - f32::from(b.size.height) + LINE * 3.0).max(0.0);
        let widest = (0..self.doc.lines()).map(|l| col_in(self.doc.line(l))).max().unwrap_or(0) as f32 * f32::from(self.cw);
        let max_x = (widest + 40.0 - f32::from(b.size.width - self.gutter)).max(0.0);
        self.scroll.y = self.scroll.y.clamp(px(0.), px(max_y));
        self.scroll.x = self.scroll.x.clamp(px(0.), px(max_x));
    }

    /// the caret's place in the text's bounds' own pixels (unscrolled)
    fn caret_at(&self, i: usize) -> Point<Pixels> {
        let l = self.doc.line_of(i);
        point(self.cw * self.doc.col_of(i) as f32, px(TOP + l as f32 * LINE))
    }

    // utf-16, for the platform
    fn to16(&self, i: usize) -> usize {
        self.doc.text[..i.min(self.doc.text.len())].encode_utf16().count()
    }
    fn from16(&self, n: usize) -> usize {
        let mut c = 0;
        for (i, ch) in self.doc.text.char_indices() {
            if c >= n {
                return i;
            }
            c += ch.len_utf16();
        }
        self.doc.text.len()
    }
    fn range16(&self, r: &Range<usize>) -> Range<usize> {
        self.to16(r.start)..self.to16(r.end)
    }
    fn range8(&self, r: &Range<usize>) -> Range<usize> {
        self.from16(r.start)..self.from16(r.end)
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(&mut self, r: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let r = self.range8(&r);
        actual.replace(self.range16(&r));
        Some(self.doc.text[r].to_string())
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.range16(&self.doc.sel), reversed: self.doc.reversed })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|r| self.range16(r))
    }
    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
    }
    fn replace_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let r = r.map(|r| self.range8(&r)).or(self.marked.clone()).unwrap_or(self.doc.sel.clone());
        let kind = (text.chars().count() == 1).then_some('i');
        self.doc.replace(r, text, kind);
        self.changed(true, cx);
    }
    fn replace_and_mark_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, sel: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        let r = r.map(|r| self.range8(&r)).or(self.marked.clone()).unwrap_or(self.doc.sel.clone());
        self.doc.replace(r.clone(), text, None);
        let marked = (!text.is_empty()).then(|| r.start..r.start + text.len());
        if let Some(s) = sel {
            let s = self.range8(&s);
            let n = self.doc.text.len();
            self.doc.sel = (r.start + s.start).min(n)..(r.start + s.end).min(n);
        }
        self.changed(false, cx);
        self.marked = marked;
    }
    fn bounds_for_range(&mut self, r: Range<usize>, b: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let r = self.range8(&r);
        let (a, z) = (self.caret_at(r.start), self.caret_at(r.end));
        let o = point(b.left() + self.gutter - self.scroll.x, b.top() - self.scroll.y);
        Some(Bounds::from_corners(point(o.x + a.x, o.y + a.y), point(o.x + z.x, o.y + z.y + px(LINE))))
    }
    fn character_index_for_point(&mut self, p: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let i = self.index_at(p);
        Some(self.to16(i))
    }
}

/// the lines in view: shaped, the gutter's numbers, the selection, the caret
struct Lines {
    ed: Entity<Editor>,
}

impl IntoElement for Lines {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// what was shaped for the lines in view: each line's number, its text and its gutter's number
struct Shaped {
    first: usize,
    lines: Vec<(ShapedLine, ShapedLine, Vec<usize>)>,
}

impl Element for Lines {
    type RequestLayoutState = ();
    type PrepaintState = Shaped;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) -> Shaped {
        let ts = window.text_system().clone();
        let size_ = px(FONT_SIZE);
        let run = |len: usize, c: u32, bold: bool, italic: bool| TextRun { len, font: font(bold, italic), color: kit::hsla(c), background_color: None, underline: None, strikethrough: None };
        // a column's width: the font's own
        let cw = ts.shape_line("0000000000".into(), size_, &[run(10, 0, false, false)], None).width / 10.;
        self.ed.update(cx, |e, _| {
            e.cw = cw;
            let digits = e.doc.lines().to_string().len().max(2);
            e.gutter = cw * (digits as f32 + 1.0) + px(GAP);
            e.bounds = Some(b);
            if e.blocks.is_none()
                && let Some(l) = &e.lang
            {
                e.blocks = Some(syntax::block_state(&e.doc.text, l));
            }
            if std::mem::take(&mut e.reveal) {
                // the caret in sight
                let c = e.caret_at(e.doc.caret());
                let (h, w) = (b.size.height, b.size.width - e.gutter);
                if c.y - e.scroll.y < px(0.) {
                    e.scroll.y = c.y - px(TOP);
                } else if c.y + px(LINE) - e.scroll.y > h {
                    e.scroll.y = c.y + px(LINE * 2.) - h;
                }
                if c.x - e.scroll.x > w - cw * 2. {
                    e.scroll.x = c.x - w + cw * 4.;
                } else if c.x < e.scroll.x {
                    e.scroll.x = (c.x - cw * 4.).max(px(0.));
                }
                e.scroll.y = e.scroll.y.max(px(0.));
            }
        });
        let e = self.ed.read(cx);
        let first = ((f32::from(e.scroll.y) - TOP) / LINE).floor().max(0.0) as usize;
        let n = (f32::from(b.size.height) / LINE).ceil() as usize + 2;
        let caret_line = e.doc.line_of(e.doc.caret());
        let mut lines = Vec::new();
        for l in first..(first + n).min(e.doc.lines()) {
            let raw = e.doc.line(l);
            let (shown, map) = expand(raw);
            // its colours: what the code is, the stretches between plain
            let mut runs = Vec::new();
            let mut at = 0;
            if let Some(lang) = &e.lang {
                let inside = e.blocks.as_ref().and_then(|b| b.get(l).copied()).unwrap_or(false);
                for (r, tok) in syntax::spans(raw, lang, inside) {
                    let (a, z) = (map[r.start], map[r.end.min(raw.len())]);
                    if a > at {
                        runs.push(run(a - at, Tok::Plain.look().0, false, false));
                    }
                    if z > a {
                        let (c, bold, italic) = tok.look();
                        runs.push(run(z - a, c, bold, italic));
                    }
                    at = z.max(at);
                }
            }
            if shown.len() > at || runs.is_empty() {
                runs.push(run(shown.len() - at, Tok::Plain.look().0, false, false));
            }
            let text = ts.shape_line(SharedString::from(shown), size_, &runs, None);
            let num = (l + 1).to_string();
            let numc = if l == caret_line { col::STEEL3 } else { col::STEEL1 };
            let gutter = ts.shape_line(SharedString::from(num.clone()), size_, &[run(num.len(), numc, false, false)], None);
            lines.push((text, gutter, map));
        }
        Shaped { first, lines }
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), sh: &mut Shaped, window: &mut Window, cx: &mut App) {
        let focus = self.ed.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(b, self.ed.clone()), cx);
        let focused = focus.is_focused(window);
        let e = self.ed.read(cx);
        let (cw, gutter, scroll) = (e.cw, e.gutter, e.scroll);
        let sel = e.doc.sel.clone();
        let caret = e.doc.caret();
        let caret_line = e.doc.line_of(caret);
        let caret_x = e.caret_at(caret).x;
        let ranges: Vec<Range<usize>> = (0..sh.lines.len()).map(|k| e.doc.line_range(sh.first + k)).collect();
        let lh = px(LINE);
        let y_of = |l: usize| b.top() + px(TOP + l as f32 * LINE) - scroll.y;
        let tx = b.left() + gutter - scroll.x;
        // the text's lines, behind the gutter
        window.with_content_mask(Some(gpui::ContentMask { bounds: Bounds::from_corners(point(b.left() + gutter - px(GAP / 2.), b.top()), b.bottom_right()) }), |window| {
            for (k, (line, _, map)) in sh.lines.iter().enumerate() {
                let l = sh.first + k;
                let y = y_of(l);
                if l == caret_line && sel.is_empty() {
                    window.paint_quad(fill(Bounds::new(point(b.left(), y), size(b.size.width, lh)), rgba(0xffffff08)));
                }
                // the selection on this line
                let r = ranges[k].clone();
                let (a, z) = (sel.start.max(r.start), sel.end.min(r.end));
                if !sel.is_empty() && sel.start <= r.end && sel.end >= r.start && a <= z {
                    let (xa, xz) = (map[a - r.start], map[z - r.start]);
                    let eol = if sel.end > r.end { 1 } else { 0 };
                    let w = cw * ((xz - xa) as f32 + eol as f32 * 0.6);
                    window.paint_quad(fill(Bounds::new(point(tx + cw * xa as f32, y), size(w.max(px(2.)), lh)), rgb(col::LINE)));
                }
                let _ = line.paint(point(tx, y), lh, window, cx);
            }
            if focused && sel.is_empty() {
                window.paint_quad(fill(Bounds::new(point(tx + caret_x, y_of(caret_line) + px(2.)), size(px(2.), lh - px(4.))), rgb(col::PALE0)));
            }
        });
        // the gutter: the numbers, right-aligned
        for (k, (_, num, _)) in sh.lines.iter().enumerate() {
            let y = y_of(sh.first + k);
            if y + lh < b.top() || y > b.bottom() {
                continue;
            }
            let _ = num.paint(point(b.left() + gutter - px(GAP) - num.width, y), lh, window, cx);
        }
    }
}

impl Render for Editor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("editor")
            .size_full()
            .overflow_hidden()
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .font_family(kit::MONO)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .child(Lines { ed: cx.entity() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Doc {
        // "|" marks the caret
        let i = s.find('|').unwrap();
        let mut d = Doc::new(&s.replace('|', ""));
        d.move_to(i);
        d
    }
    fn show(d: &Doc) -> String {
        let mut s = d.text.clone();
        s.insert(d.caret(), '|');
        s
    }

    #[test]
    fn lines_and_columns() {
        let d = Doc::new("ab\n\tc\n");
        assert_eq!(d.lines(), 3);
        assert_eq!(d.line(1), "\tc");
        assert_eq!((d.line_of(0), d.line_of(3), d.line_of(6)), (0, 1, 2));
        assert_eq!(d.col_of(5), 5, "a tab to the next stop");
        assert_eq!(d.at_col(1, 4), 4);
        assert_eq!(d.at_col(1, 1), 3, "inside a tab: its nearer side");
        assert_eq!(d.at_col(0, 99), 2);
        assert_eq!(expand("\tx").0, "    x");
    }

    #[test]
    fn moving_as_a_mac_field_does() {
        let mut d = at("one two|.three\nfour");
        d.travel("left", false, true, 1);
        assert_eq!(show(&d), "one |two.three\nfour");
        d.travel("right", false, true, 1);
        d.travel("right", false, true, 1);
        assert_eq!(show(&d), "one two.|three\nfour");
        d.travel("down", false, false, 1);
        assert_eq!(show(&d), "one two.three\nfour|", "down keeps the column as near as it can");
        d.travel("up", false, false, 1);
        assert_eq!(show(&d), "one two.|three\nfour", "and comes back to it");
        d.travel("home", true, false, 1);
        assert_eq!(d.selected(), "one two.");
        d.travel("bottom", false, false, 1);
        assert_eq!(d.caret(), d.text.len());
        let mut d = at("  x|");
        d.travel("home", false, false, 1);
        assert_eq!(d.caret(), 2, "home: to the first character, then the start");
        d.travel("home", false, false, 1);
        assert_eq!(d.caret(), 0);
    }

    #[test]
    fn typing_undo_and_redo() {
        let mut d = at("fn |()");
        for c in "main".chars() {
            d.insert(&c.to_string());
        }
        d.insert("!!");
        assert_eq!(d.text, "fn main!!()");
        assert!(d.undo());
        assert_eq!(d.text, "fn main()", "a paste is its own step");
        assert!(d.undo());
        assert_eq!(show(&d), "fn |()", "typing runs together");
        assert!(d.redo() && d.redo());
        assert_eq!(d.text, "fn main!!()");
        d.backspace(false, false);
        d.backspace(false, false);
        assert_eq!(d.text, "fn main()");
        d.undo();
        assert_eq!(d.text, "fn main!!()", "deleting runs together");
        assert!(d.redo());
        assert_eq!(d.text, "fn main()");
    }

    #[test]
    fn enter_keeps_the_indent_and_tab_is_the_files() {
        let mut d = at("fn a() {\n    x|\n}");
        d.newline();
        assert_eq!(show(&d), "fn a() {\n    x\n    |\n}");
        assert_eq!(d.indent_unit(), "    ");
        assert_eq!(Doc::new("a\n\tb").indent_unit(), "\t");
        let mut d = at("a b|");
        d.backspace(true, false);
        assert_eq!(show(&d), "a |");
        let mut d = at("ab\ncd|");
        d.backspace(false, true);
        assert_eq!(show(&d), "ab\n|", "⌘⌫: to the line's start");
    }

    #[test]
    fn what_comes_in_from_disk_keeps_the_caret() {
        let mut d = at("abc|def");
        d.reload("abcXdef");
        assert_eq!(d.text, "abcXdef");
        assert_eq!(d.caret(), 3);
        d.reload("é");
        assert!(d.text.is_char_boundary(d.caret()));
        assert_eq!(d.word_at(0), 0..2);
    }
}
