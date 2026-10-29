//! A text field (GPUI has none): one line, or several for the notes' answers (a textarea). After gpui's
//! examples/input.rs, without its actions: the keys that edit are taken in the field's own key listener, typed
//! text comes through the platform's input handler (so dead keys and the character palette work), and what
//! the field does not take (↑ ↓ Tab Enter Esc on one line) goes on to the panel around it.

use crate::kit::{self, Frame, col, frame_bg};
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable,
    GlobalElementId, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Style, Subscription, TextAlign,
    TextRun, UTF16Selection, Window, WrappedLine, div, fill, point, prelude::*, px, relative, rgb, rgba, size,
};
use std::ops::Range;

/// what is being typed and where the caret is, with no pixels: byte offsets on char boundaries
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edit {
    pub text: String,
    pub sel: Range<usize>,
    /// the caret is at the selection's start
    pub reversed: bool,
}

impl Edit {
    pub fn caret(&self) -> usize {
        if self.reversed { self.sel.start } else { self.sel.end }
    }
    fn prev(&self, i: usize) -> usize {
        self.text[..i].char_indices().next_back().map(|(j, _)| j).unwrap_or(0)
    }
    fn next(&self, i: usize) -> usize {
        self.text[i..].chars().next().map(|c| i + c.len_utf8()).unwrap_or(self.text.len())
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
    pub fn left(&mut self, shift: bool) {
        let to = self.prev(self.caret());
        if shift {
            self.select_to(to)
        } else if self.sel.is_empty() {
            self.move_to(to)
        } else {
            self.move_to(self.sel.start)
        }
    }
    pub fn right(&mut self, shift: bool) {
        let to = self.next(self.caret());
        if shift {
            self.select_to(to)
        } else if self.sel.is_empty() {
            self.move_to(to)
        } else {
            self.move_to(self.sel.end)
        }
    }
    /// the line the caret is on: where it starts and ends
    fn line_at(&self, i: usize) -> (usize, usize) {
        let a = self.text[..i].rfind('\n').map(|j| j + 1).unwrap_or(0);
        let b = self.text[i..].find('\n').map(|j| i + j).unwrap_or(self.text.len());
        (a, b)
    }
    pub fn home(&mut self, shift: bool) {
        let to = self.line_at(self.caret()).0;
        if shift { self.select_to(to) } else { self.move_to(to) }
    }
    pub fn end(&mut self, shift: bool) {
        let to = self.line_at(self.caret()).1;
        if shift { self.select_to(to) } else { self.move_to(to) }
    }
    pub fn select_all(&mut self) {
        self.sel = 0..self.text.len();
        self.reversed = false;
    }
    /// what is selected replaced by `s` (typing, a paste), the caret after it
    pub fn insert(&mut self, s: &str) {
        self.replace(self.sel.clone(), s);
    }
    pub fn replace(&mut self, r: Range<usize>, s: &str) {
        let (a, b) = (r.start.min(self.text.len()), r.end.min(self.text.len()));
        self.text.replace_range(a..b, s);
        self.move_to(a + s.len());
    }
    pub fn backspace(&mut self) {
        if self.sel.is_empty() {
            let c = self.caret();
            self.sel = self.prev(c)..c;
        }
        self.insert("");
    }
    pub fn delete(&mut self) {
        if self.sel.is_empty() {
            let c = self.caret();
            self.sel = c..self.next(c);
        }
        self.insert("");
    }
    /// ⌘⌫: the line up to the caret
    pub fn kill_line(&mut self) {
        let c = self.caret();
        self.sel = self.line_at(c).0..c;
        self.insert("");
    }
    pub fn selected(&self) -> &str {
        &self.text[self.sel.clone()]
    }
}

/// what a field says to the panel it is in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldEvent {
    Changed,
    Focus,
    Blur,
}

/// how it looks: the 16-bit panels' steel field in the terminal's font, or the notes' quiet box
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Steel,
    Notes,
}

pub struct Field {
    focus: FocusHandle,
    pub e: Edit,
    pub placeholder: SharedString,
    marked: Option<Range<usize>>,
    multi: bool,
    look: Look,
    /// at most this many characters (an input's maxlength)
    pub max: usize,
    lines: Vec<(WrappedLine, usize, Pixels)>,
    bounds: Option<Bounds<Pixels>>,
    /// one line scrolled sideways to keep the caret in sight
    scroll: Pixels,
    rows: usize,
    selecting: bool,
    _subs: Vec<Subscription>,
}

impl EventEmitter<FieldEvent> for Field {}

impl Field {
    pub fn new(multi: bool, look: Look, window: &mut Window, cx: &mut Context<Field>) -> Field {
        let focus = cx.focus_handle();
        let subs = vec![
            cx.on_focus(&focus, window, |_, _, cx| {
                cx.emit(FieldEvent::Focus);
                cx.notify()
            }),
            cx.on_blur(&focus, window, |_, _, cx| {
                cx.emit(FieldEvent::Blur);
                cx.notify()
            }),
        ];
        Field { focus, e: Edit::default(), placeholder: "".into(), marked: None, multi, look, max: 4000, lines: Vec::new(), bounds: None, scroll: px(0.), rows: 1, selecting: false, _subs: subs }
    }

    pub fn text(&self) -> &str {
        &self.e.text
    }
    /// set what it says, the caret at the end
    pub fn set(&mut self, s: &str, cx: &mut Context<Field>) {
        let s = if self.multi { s.to_string() } else { s.replace('\n', " ") };
        self.e = Edit { text: s, ..Default::default() };
        self.e.move_to(self.e.text.len());
        self.marked = None;
        cx.emit(FieldEvent::Changed);
        cx.notify();
    }
    pub fn select_all(&mut self, cx: &mut Context<Field>) {
        self.e.select_all();
        cx.notify();
    }
    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus);
    }
    pub fn focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    fn changed(&mut self, cx: &mut Context<Field>) {
        if self.e.text.chars().count() > self.max {
            let cut = self.e.text.char_indices().nth(self.max).map(|(i, _)| i).unwrap_or(self.e.text.len());
            self.e.text.truncate(cut);
            self.e.move_to(self.e.caret().min(cut));
        }
        cx.emit(FieldEvent::Changed);
        cx.notify();
    }

    fn key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Field>) {
        let k = &ev.keystroke;
        let (m, shift, cmd, alt) = (k.modifiers, k.modifiers.shift, k.modifiers.platform, k.modifiers.alt);
        let edit = |f: &mut Field, cx: &mut Context<Field>, g: &dyn Fn(&mut Edit)| {
            g(&mut f.e);
            f.marked = None;
            f.changed(cx);
        };
        let moved = |f: &mut Field, cx: &mut Context<Field>, g: &dyn Fn(&mut Edit)| {
            g(&mut f.e);
            cx.notify();
        };
        match (k.key.as_str(), cmd, m.control) {
            ("backspace", true, _) => edit(self, cx, &|e| e.kill_line()),
            ("backspace", _, _) if alt => edit(self, cx, &|e| {
                // a word back, as a Mac field does
                let c = e.caret();
                let t = &e.text[..c];
                let a = t.trim_end().rfind(|ch: char| ch.is_whitespace() || ch == '/').map(|j| j + 1).unwrap_or(0);
                e.sel = a..c;
                e.insert("");
            }),
            ("backspace", _, _) => edit(self, cx, &|e| e.backspace()),
            ("delete", _, _) => edit(self, cx, &|e| e.delete()),
            ("left", true, _) | ("home", _, _) => moved(self, cx, &|e| e.home(shift)),
            ("right", true, _) | ("end", _, _) => moved(self, cx, &|e| e.end(shift)),
            ("left", _, _) => moved(self, cx, &|e| e.left(shift)),
            ("right", _, _) => moved(self, cx, &|e| e.right(shift)),
            ("a", true, _) => moved(self, cx, &|e| e.select_all()),
            ("c", true, _) | ("x", true, _) => {
                if !self.e.sel.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(self.e.selected().to_string()));
                    if k.key == "x" {
                        edit(self, cx, &|e| e.insert(""));
                    }
                }
            }
            ("v", true, _) => {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    let t = if self.multi { t } else { t.replace('\n', " ") };
                    edit(self, cx, &|e| e.insert(&t));
                }
            }
            ("enter", false, false) if self.multi => edit(self, cx, &|e| e.insert("\n")),
            ("up", false, _) | ("down", false, _) if self.multi => {
                let down = k.key == "down";
                if let Some(i) = self.vertical(down) {
                    moved(self, cx, &|e| if shift { e.select_to(i) } else { e.move_to(i) });
                }
            }
            _ => return, // on to the panel (↑ ↓ Tab Enter Esc), or typed (the input handler)
        }
        cx.stop_propagation();
    }

    /// the place a row up or down from the caret (a textarea's ↑ ↓)
    fn vertical(&self, down: bool) -> Option<usize> {
        let lh = self.line_height();
        let p = self.pos(self.e.caret())?;
        let y = if down { p.y + lh * 1.5 } else { p.y - lh * 0.5 };
        if y < px(0.) {
            return Some(0);
        }
        Some(self.index_at(point(p.x, y)))
    }

    fn line_height(&self) -> Pixels {
        match self.look {
            Look::Steel => px(20.),
            Look::Notes => px(20.),
        }
    }

    /// where an offset is drawn, relative to the text's origin
    fn pos(&self, i: usize) -> Option<Point<Pixels>> {
        if self.e.text.is_empty() {
            return Some(point(px(0.), px(0.))); // (what is shown is the placeholder)
        }
        let lh = self.line_height();
        for (line, start, y) in &self.lines {
            if i >= *start && i <= start + line.len() {
                let p = line.position_for_index(i - start, lh)?;
                return Some(point(p.x - self.scroll, *y + p.y));
            }
        }
        None
    }

    /// the offset nearest a point relative to the text's origin
    fn index_at(&self, p: Point<Pixels>) -> usize {
        if self.e.text.is_empty() {
            return 0;
        }
        let lh = self.line_height();
        let p = point(p.x + self.scroll, p.y);
        for (k, (line, start, y)) in self.lines.iter().enumerate() {
            let h = line.size(lh).height;
            if p.y < *y + h || k + 1 == self.lines.len() {
                let q = point(p.x.max(px(0.)), (p.y - *y).clamp(px(0.), h - px(1.)));
                return start + line.closest_index_for_position(q, lh).unwrap_or_else(|i| i);
            }
        }
        self.e.text.len()
    }

    fn mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Field>) {
        window.focus(&self.focus);
        let Some(b) = self.bounds else { return };
        let i = self.index_at(point(ev.position.x - b.left(), ev.position.y - b.top()));
        if ev.click_count >= 2 {
            self.e.select_all();
        } else if ev.modifiers.shift {
            self.e.select_to(i);
        } else {
            self.e.move_to(i);
        }
        self.selecting = true;
        cx.notify();
    }
    fn mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Field>) {
        if !self.selecting {
            return;
        }
        let Some(b) = self.bounds else { return };
        let i = self.index_at(point(ev.position.x - b.left(), ev.position.y - b.top()));
        self.e.select_to(i);
        cx.notify();
    }
    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Field>) {
        self.selecting = false;
    }

    // utf-16, for the platform
    fn to16(&self, i: usize) -> usize {
        self.e.text[..i.min(self.e.text.len())].encode_utf16().count()
    }
    fn from16(&self, n: usize) -> usize {
        let mut c = 0;
        for (i, ch) in self.e.text.char_indices() {
            if c >= n {
                return i;
            }
            c += ch.len_utf16();
        }
        self.e.text.len()
    }
    fn range16(&self, r: &Range<usize>) -> Range<usize> {
        self.to16(r.start)..self.to16(r.end)
    }
    fn range8(&self, r: &Range<usize>) -> Range<usize> {
        self.from16(r.start)..self.from16(r.end)
    }
}

impl Focusable for Field {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EntityInputHandler for Field {
    fn text_for_range(&mut self, r: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let r = self.range8(&r);
        actual.replace(self.range16(&r));
        Some(self.e.text[r].to_string())
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.range16(&self.e.sel), reversed: self.e.reversed })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|r| self.range16(r))
    }
    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
    }
    fn replace_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let r = r.map(|r| self.range8(&r)).or(self.marked.clone()).unwrap_or(self.e.sel.clone());
        let text = if self.multi { text.to_string() } else { text.replace('\n', " ") };
        self.e.replace(r, &text);
        self.marked = None;
        self.changed(cx);
    }
    fn replace_and_mark_text_in_range(&mut self, r: Option<Range<usize>>, text: &str, sel: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        let r = r.map(|r| self.range8(&r)).or(self.marked.clone()).unwrap_or(self.e.sel.clone());
        self.e.replace(r.clone(), text);
        self.marked = (!text.is_empty()).then(|| r.start..r.start + text.len());
        if let Some(s) = sel {
            let s = self.range8(&s);
            self.e.sel = (r.start + s.start).min(self.e.text.len())..(r.start + s.end).min(self.e.text.len());
        }
        self.changed(cx);
    }
    fn bounds_for_range(&mut self, r: Range<usize>, b: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let r = self.range8(&r);
        let (a, z) = (self.pos(r.start)?, self.pos(r.end)?);
        Some(Bounds::from_corners(point(b.left() + a.x, b.top() + a.y), point(b.left() + z.x, b.top() + z.y + self.line_height())))
    }
    fn character_index_for_point(&mut self, p: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let b = self.bounds?;
        let i = self.index_at(point(p.x - b.left(), p.y - b.top()));
        Some(self.to16(i))
    }
}

/// the text itself: shaped, its caret and selection, the platform's input handler
struct Text {
    field: Entity<Field>,
}

impl IntoElement for Text {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Text {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let f = self.field.read(cx);
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (f.line_height() * f.rows.max(if f.multi { 2 } else { 1 }) as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) {
        let (text, placeholder, look, multi, caret) = {
            let f = self.field.read(cx);
            (f.e.text.clone(), f.placeholder.clone(), f.look, f.multi, f.e.caret())
        };
        let (family, size_, ink, faint) = match look {
            Look::Steel => (kit::MONO, px(14.), col::INK, col::STEEL1),
            Look::Notes => (kit::SANS, px(14.), col::TEXT, col::FAINT),
        };
        let shown: SharedString = if text.is_empty() { placeholder } else { text.clone().into() };
        let color = kit::hsla(if text.is_empty() { faint } else { ink });
        let run = TextRun { len: shown.len(), font: gpui::font(family), color, background_color: None, underline: None, strikethrough: None };
        let wrap = multi.then_some(b.size.width);
        let lh = self.field.read(cx).line_height();
        let lines = window.text_system().shape_text(shown.clone(), size_, &[run], wrap, None).map(|v| v.into_vec()).unwrap_or_default();
        let mut placed = Vec::new();
        let (mut start, mut y) = (0usize, px(0.));
        for line in lines {
            let n = line.len();
            let h = line.size(lh).height;
            placed.push((line, start, y));
            start += n + 1;
            y += h;
        }
        let rows = (y / lh).round().max(1.0) as usize;
        self.field.update(cx, |f, cx| {
            f.lines = placed;
            if f.rows != rows && multi {
                f.rows = rows;
                cx.notify();
            }
            f.bounds = Some(b);
            if !multi && !text.is_empty() {
                // keep the caret in sight
                let x = f.lines.first().and_then(|(l, _, _)| l.position_for_index(caret, lh)).map(|p| p.x).unwrap_or(px(0.));
                let w = b.size.width - px(2.);
                if x - f.scroll > w {
                    f.scroll = x - w;
                } else if x < f.scroll {
                    f.scroll = x;
                }
            } else {
                f.scroll = px(0.);
            }
        });
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        let focus = self.field.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(b, self.field.clone()), cx);
        let (lines, sel, caret, scroll, empty, look) = {
            let f = self.field.read(cx);
            (f.lines.clone(), f.e.sel.clone(), f.e.caret(), f.scroll, f.e.text.is_empty(), f.look)
        };
        let lh = self.field.read(cx).line_height();
        let focused = focus.is_focused(window);
        window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
            let o = point(b.left() - scroll, b.top());
            // the selection: a band per row it covers
            if !sel.is_empty() && !empty {
                let selc = match look {
                    Look::Steel => rgba(0xffe27aff),
                    Look::Notes => rgba(0xffe27a55),
                };
                for (line, start, y) in &lines {
                    let (a, z) = (sel.start.max(*start), sel.end.min(start + line.len()));
                    if a > z || (a == z && sel.end != a) {
                        continue;
                    }
                    let (Some(pa), Some(pz)) = (line.position_for_index(a - start, lh), line.position_for_index(z - start, lh)) else { continue };
                    let w = line.size(lh).width.max(b.size.width + scroll);
                    let mut band = |x0: Pixels, x1: Pixels, row: Pixels| window.paint_quad(fill(Bounds::from_corners(point(o.x + x0, o.y + *y + row), point(o.x + x1, o.y + *y + row + lh)), selc));
                    if pa.y == pz.y {
                        band(pa.x, pz.x.max(pa.x + px(4.)), pa.y);
                    } else {
                        band(pa.x, w, pa.y);
                        let mut r = pa.y + lh;
                        while r < pz.y {
                            band(px(0.), w, r);
                            r += lh;
                        }
                        band(px(0.), pz.x, pz.y);
                    }
                }
            }
            for (line, _, y) in &lines {
                let _ = line.paint(point(o.x, o.y + *y), lh, TextAlign::Left, None, window, cx);
            }
            if focused && sel.is_empty() {
                let p = if empty { Some(point(px(0.), px(0.))) } else { self.field.read(cx).pos(caret).map(|p| point(p.x + scroll, p.y)) };
                if let Some(p) = p {
                    let c = match look {
                        Look::Steel => col::INK,
                        Look::Notes => col::GOLD2,
                    };
                    window.paint_quad(fill(Bounds::new(point(o.x + p.x, o.y + p.y + px(2.)), size(px(2.), lh - px(4.))), kit::hsla(c)));
                }
            }
        });
    }
}

impl Render for Field {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let body = div()
            .id("field")
            .relative()
            .w_full()
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move));
        let text = div().relative().line_height(self.line_height()).child(Text { field: cx.entity() });
        match self.look {
            // #view .field: steel, an ink line, a sunken top-left, gold round it while it has the focus
            Look::Steel => body.child(frame_bg(Frame::field(focused))).child(text.px(px(12.)).py(px(8.))),
            // the notes' textarea: on --code, a hairline, gold while it has the focus
            Look::Notes => body
                .rounded(px(5.))
                .bg(rgb(col::CODE))
                .border_1()
                .border_color(rgb(if focused { col::GOLD2 } else { col::LINE }))
                .min_h(px(52.))
                .child(text.px(px(9.)).py(px(7.))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(s: &str) -> Edit {
        let mut e = Edit { text: s.into(), ..Default::default() };
        e.move_to(s.len());
        e
    }

    #[test]
    fn typing_and_deleting() {
        let mut e = ed("~/pro");
        e.insert("j");
        assert_eq!(e.text, "~/proj");
        e.backspace();
        e.backspace();
        assert_eq!(e.text, "~/pr");
        e.left(false);
        e.delete();
        assert_eq!(e.text, "~/p");
        assert_eq!(e.caret(), 3);
    }

    #[test]
    fn selection_goes_both_ways() {
        let mut e = ed("hello");
        e.left(true);
        e.left(true);
        assert_eq!(e.selected(), "lo");
        assert!(e.reversed);
        e.right(true);
        e.right(true);
        e.right(true);
        assert_eq!(e.selected(), "");
        e.home(true);
        assert_eq!(e.selected(), "hello");
        e.insert("x");
        assert_eq!(e.text, "x");
    }

    #[test]
    fn many_bytes_a_char() {
        let mut e = ed("héé");
        e.backspace();
        assert_eq!(e.text, "hé");
        e.left(false);
        e.left(false);
        e.insert("→");
        assert_eq!(e.text, "→hé");
        assert_eq!(e.caret(), "→".len());
    }

    #[test]
    fn lines_have_their_own_home() {
        let mut e = ed("one\ntwo");
        e.home(false);
        assert_eq!(e.caret(), 4);
        e.kill_line();
        assert_eq!(e.text, "one\ntwo");
        e.end(false);
        e.kill_line();
        assert_eq!(e.text, "one\n");
    }
}
