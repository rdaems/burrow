// One bot's session: `claude attach <id>` (over ssh for a remote room) in a pty, parsed by alacritty's
// terminal and painted with egui. Closing it only ends the attach; the session keeps working.

use crate::host::Host;
use crate::sprite::{hex, INK, TERRACOTTA};
use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use egui::{Color32, FontFamily, FontId, Key, Pos2, Rect, Vec2};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const FONT: &str = "term";
pub const FONT_BOLD: &str = "term-bold";
const SIZE: f32 = 13.0;
const PAD: f32 = 10.0;

/// the terminal's colours; light or dark after the theme Claude Code is set to on that machine
#[derive(Clone, Copy)]
pub struct Scheme {
    pub bg: Color32,
    pub fg: Color32,
    base: [u32; 16],
}

pub const DARK: Scheme = Scheme {
    bg: INK,
    fg: hex(0xe6e1ef),
    base: [
        0x2a2140, 0xe8402a, 0x5fcf4a, 0xf2a531, 0x4a76b8, 0x9a74d6, 0x2fa087, 0xc2c9d9, 0x5d6280, 0xff6b5a, 0x8ee07a,
        0xffe27a, 0x8fc0e8, 0xc4a8f0, 0x7fdcb0, 0xf4f6fb,
    ],
};

pub const LIGHT: Scheme = Scheme {
    bg: hex(0xfbf8f1),
    fg: INK,
    base: [
        0x140f1e, 0xb8321f, 0x2d8a3a, 0xa8650f, 0x2c4580, 0x5a3f94, 0x1b6660, 0x8b93ad, 0x5d6280, 0xe8402a, 0x3f9e2f,
        0xb86a1e, 0x4a76b8, 0x9a74d6, 0x2fa087, 0x3b3a55,
    ],
};

impl Scheme {
    /// xterm's 256 colours, the first 16 in pair's palette
    pub fn color(&self, i: usize) -> Color32 {
        match i {
            0..=15 => hex(self.base[i]),
            16..=231 => {
                let i = i - 16;
                let v = |k: usize| if k == 0 { 0 } else { (55 + k * 40) as u8 };
                Color32::from_rgb(v(i / 36), v((i / 6) % 6), v(i % 6))
            }
            232..=255 => {
                let g = (8 + (i - 232) * 10) as u8;
                Color32::from_rgb(g, g, g)
            }
            _ => self.fg,
        }
    }

    fn named(&self, n: NamedColor) -> Color32 {
        match n {
            NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::Cursor => self.fg,
            NamedColor::Background => self.bg,
            NamedColor::DimForeground => self.fg.gamma_multiply(0.66),
            n => {
                let i = n as usize;
                if i < 16 {
                    self.color(i)
                } else if (259..267).contains(&i) {
                    self.color(i - 259).gamma_multiply(0.66) // the dim colours
                } else {
                    self.fg
                }
            }
        }
    }
}

#[derive(Clone)]
struct Listener {
    ctx: egui::Context,
    sender: Arc<Mutex<Option<EventLoopSender>>>,
    exited: Arc<AtomicBool>,
    size: Arc<Mutex<WindowSize>>,
    scheme: Arc<Mutex<Scheme>>,
}

impl Listener {
    fn write(&self, s: String) {
        if let Some(tx) = self.sender.lock().unwrap().as_ref() {
            let _ = tx.send(Msg::Input(Cow::Owned(s.into_bytes())));
        }
    }
}

impl EventListener for Listener {
    fn send_event(&self, ev: Event) {
        match ev {
            Event::PtyWrite(s) => self.write(s),
            Event::ColorRequest(i, f) => {
                // answer "what colour is your background?" and friends with our own palette
                let sc = *self.scheme.lock().unwrap();
                let c = match i {
                    257 => sc.bg,
                    256 | 258 => sc.fg,
                    i => sc.color(i),
                };
                self.write(f(Rgb { r: c.r(), g: c.g(), b: c.b() }));
            }
            Event::TextAreaSizeRequest(f) => self.write(f(*self.size.lock().unwrap())),
            Event::ChildExit(_) | Event::Exit => self.exited.store(true, Ordering::Relaxed),
            _ => {}
        }
        self.ctx.request_repaint();
    }
}

struct Size {
    cols: usize,
    rows: usize,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Session {
    term: Arc<FairMutex<Term<Listener>>>,
    tx: EventLoopSender,
    listener: Listener,
    pub exited: Arc<AtomicBool>,
    grid: (usize, usize),
    scroll_acc: f32,
    selecting: bool,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
    }
}

impl Session {
    pub fn attach(host: &Host, id: &str, ctx: &egui::Context) -> std::io::Result<Self> {
        let (prog, args) = host.argv(&format!("exec claude attach {}", crate::host::sq(id)), true);
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        let opts = tty::Options {
            shell: Some(tty::Shell::new(prog, args)),
            working_directory: None,
            drain_on_exit: false,
            env,
        };
        let (cols, rows) = (100, 30);
        let ws = WindowSize { num_lines: rows as u16, num_cols: cols as u16, cell_width: 8, cell_height: 16 };
        let pty = tty::new(&opts, ws, 0)?;
        let exited = Arc::new(AtomicBool::new(false));
        let listener = Listener {
            ctx: ctx.clone(),
            sender: Arc::new(Mutex::new(None)),
            exited: exited.clone(),
            size: Arc::new(Mutex::new(ws)),
            scheme: Arc::new(Mutex::new(DARK)),
        };
        let config = Config { scrolling_history: 10_000, ..Config::default() };
        let term = Arc::new(FairMutex::new(Term::new(config, &Size { cols, rows }, listener.clone())));
        let el = EventLoop::new(term.clone(), listener.clone(), pty, false, false)?;
        let tx = el.channel();
        *listener.sender.lock().unwrap() = Some(tx.clone());
        el.spawn();
        Ok(Session { term, tx, listener, exited, grid: (cols, rows), scroll_acc: 0.0, selecting: false })
    }

    pub fn write(&self, s: &str) {
        self.term.lock().scroll_display(Scroll::Bottom);
        self.listener.write(s.to_string());
    }

    /// type a path the way a terminal does when a file is dropped on it
    pub fn type_path(&self, p: &str) {
        let safe = p.chars().all(|c| c.is_ascii_alphanumeric() || "/._-~+,@%".contains(c));
        let q = if safe { p.to_string() } else { format!("'{}'", p.replace('\'', "'\\''")) };
        self.write(&format!("{q} "));
    }

    fn resize(&mut self, cols: usize, rows: usize, cw: f32, ch: f32) {
        if (cols, rows) == self.grid || cols < 2 || rows < 2 {
            return;
        }
        self.grid = (cols, rows);
        let ws = WindowSize { num_lines: rows as u16, num_cols: cols as u16, cell_width: cw as u16, cell_height: ch as u16 };
        *self.listener.size.lock().unwrap() = ws;
        self.term.lock().resize(Size { cols, rows });
        let _ = self.tx.send(Msg::Resize(ws));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, sc: Scheme) {
        *self.listener.scheme.lock().unwrap() = sc;
        let rect = ui.available_rect_before_wrap();
        let id = ui.scope_id().with("terminal");
        let resp = ui.interact(rect, id, egui::Sense::click_and_drag());
        ui.painter().rect_filled(rect, 0.0, sc.bg);
        let font = FontId::new(SIZE, FontFamily::Name(FONT.into()));
        let bold = FontId::new(SIZE, FontFamily::Name(FONT_BOLD.into()));
        let (cw, ch) = ui.ctx().fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let inner = rect.shrink(PAD);
        let cols = (inner.width() / cw).floor().max(2.0) as usize;
        let rows = (inner.height() / ch).floor().max(2.0) as usize;
        self.resize(cols, rows, cw, ch);

        // the keyboard belongs to the session, Tab and Esc and arrows included
        if resp.clicked() || !ui.ctx().memory(|m| m.focused().is_some()) {
            resp.request_focus();
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
            )
        });
        let at = |pos: Pos2, off: usize| -> (Point, Side) {
            let x = ((pos.x - inner.left()) / cw).clamp(0.0, cols as f32 - 0.01);
            let y = ((pos.y - inner.top()) / ch).floor().clamp(0.0, rows as f32 - 1.0);
            let side = if x.fract() < 0.5 { Side::Left } else { Side::Right };
            (Point::new(Line(y as i32 - off as i32), Column(x as usize)), side)
        };
        {
            let mut term = self.term.lock();
            let off = term.grid().display_offset();
            // select with the mouse, copy with the usual shortcut
            if resp.drag_started() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let (pt, side) = at(p, off);
                    term.selection = Some(Selection::new(SelectionType::Simple, pt, side));
                    self.selecting = true;
                }
            } else if resp.dragged() && self.selecting {
                if let Some(p) = resp.interact_pointer_pos() {
                    let (pt, side) = at(p, off);
                    if let Some(s) = term.selection.as_mut() {
                        s.update(pt, side);
                    }
                }
            } else if resp.drag_stopped() {
                self.selecting = false;
            } else if resp.clicked() {
                term.selection = None;
            }
            if resp.hovered() {
                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                if dy != 0.0 {
                    self.scroll_acc += dy / ch;
                    let n = self.scroll_acc.trunc() as i32;
                    self.scroll_acc -= n as f32;
                    if n != 0 {
                        let mode = *term.mode();
                        if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
                            let k = if n > 0 { "\x1bOA" } else { "\x1bOB" };
                            self.listener.write(k.repeat(n.unsigned_abs() as usize));
                        } else {
                            term.scroll_display(Scroll::Delta(n));
                        }
                    }
                }
            }
        }
        if resp.has_focus() {
            self.keys(ui);
        }
        self.paint(ui, inner, cw, ch, &font, &bold, resp.has_focus(), sc);
    }

    fn keys(&mut self, ui: &mut egui::Ui) {
        let mac = cfg!(target_os = "macos");
        let events = ui.input(|i| i.events.clone());
        let mode = *self.term.lock().mode();
        let mut out = String::new();
        for ev in events {
            match ev {
                egui::Event::Text(t) => out.push_str(&t),
                egui::Event::Paste(t) => {
                    if mode.contains(TermMode::BRACKETED_PASTE) {
                        out.push_str(&format!("\x1b[200~{}\x1b[201~", t.replace('\x1b', "")));
                    } else {
                        out.push_str(&t.replace("\r\n", "\r").replace('\n', "\r"));
                    }
                }
                egui::Event::Copy => {
                    let sel = self.term.lock().selection_to_string().filter(|s| !s.is_empty());
                    match sel {
                        Some(s) => ui.ctx().copy_text(s),
                        None if !mac => out.push('\x03'),
                        None => {}
                    }
                }
                egui::Event::Cut if !mac => out.push('\x18'),
                egui::Event::Key { key, pressed: true, modifiers: m, .. } => {
                    let app = mode.contains(TermMode::APP_CURSOR);
                    let arrow = |c: char| if app { format!("\x1bO{c}") } else { format!("\x1b[{c}") };
                    let s: Option<String> = match key {
                        Key::Enter if m.shift || m.alt => Some("\x1b\r".into()),
                        Key::Enter => Some("\r".into()),
                        Key::Tab if m.shift => Some("\x1b[Z".into()),
                        Key::Tab => Some("\t".into()),
                        Key::Backspace if m.alt => Some("\x1b\x7f".into()),
                        Key::Backspace if m.mac_cmd => Some("\x15".into()),
                        Key::Backspace => Some("\x7f".into()),
                        Key::Escape => Some("\x1b".into()),
                        Key::Delete => Some("\x1b[3~".into()),
                        Key::ArrowUp => Some(arrow('A')),
                        Key::ArrowDown => Some(arrow('B')),
                        Key::ArrowLeft if m.alt => Some("\x1bb".into()),
                        Key::ArrowRight if m.alt => Some("\x1bf".into()),
                        Key::ArrowLeft if m.mac_cmd => Some("\x01".into()),
                        Key::ArrowRight if m.mac_cmd => Some("\x05".into()),
                        Key::ArrowRight => Some(arrow('C')),
                        Key::ArrowLeft => Some(arrow('D')),
                        Key::Home => Some(arrow('H')),
                        Key::End => Some(arrow('F')),
                        Key::PageUp if m.shift => {
                            self.term.lock().scroll_display(Scroll::PageUp);
                            None
                        }
                        Key::PageDown if m.shift => {
                            self.term.lock().scroll_display(Scroll::PageDown);
                            None
                        }
                        Key::PageUp => Some("\x1b[5~".into()),
                        Key::PageDown => Some("\x1b[6~".into()),
                        _ if m.ctrl && !m.alt => {
                            // ctrl+letter; on Linux egui turns ctrl+c/x/v into copy/cut/paste itself
                            let name = key.name();
                            let c = name.chars().next().filter(|_| name.len() == 1).unwrap_or('\0');
                            let taken = !mac && matches!(c, 'C' | 'X' | 'V');
                            if c.is_ascii_uppercase() && !taken {
                                Some(((c as u8 - b'A' + 1) as char).to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some(s) = s {
                        out.push_str(&s);
                    }
                }
                _ => {}
            }
        }
        if !out.is_empty() {
            self.term.lock().selection = None;
            self.write(&out);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(&self, ui: &egui::Ui, inner: Rect, cw: f32, ch: f32, font: &FontId, bold: &FontId, focused: bool, sc: Scheme) {
        let painter = ui.painter_at(inner.expand(PAD));
        let term = self.term.lock();
        let content = term.renderable_content();
        let colors = content.colors;
        let off = content.display_offset as i32;
        let sel = content.selection;
        let resolve = |c: Color| -> Color32 {
            match c {
                Color::Spec(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
                Color::Indexed(i) => colors[i as usize].map(rgb32).unwrap_or_else(|| sc.color(i as usize)),
                Color::Named(n) => colors[n].map(rgb32).unwrap_or_else(|| sc.named(n)),
            }
        };
        let mut bg_mesh = egui::Mesh::default();
        // runs of plain ASCII in one style are laid out as one string; anything else gets its own cell
        let mut runs: Vec<(Pos2, String, Color32, bool)> = Vec::new();
        let mut singles: Vec<(Pos2, char, Color32, bool, f32)> = Vec::new();
        let mut blocks = egui::Mesh::default();
        let mut cur: Option<(i32, usize, Color32, bool)> = None; // row, end column, colour, bold
        for cell in content.display_iter {
            let row = cell.point.line.0 + off;
            let col = cell.point.column.0;
            let flags = cell.flags;
            if flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                cur = None;
                continue;
            }
            let mut fg = resolve(cell.fg);
            let mut bg = resolve(cell.bg);
            if flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if flags.contains(Flags::DIM) {
                fg = fg.gamma_multiply(0.66);
            }
            if sel.is_some_and(|s| s.contains(cell.point)) {
                bg = TERRACOTTA.gamma_multiply(0.45);
            }
            let w = if flags.contains(Flags::WIDE_CHAR) { 2.0 } else { 1.0 };
            let pos = Pos2::new(inner.left() + col as f32 * cw, inner.top() + row as f32 * ch);
            if bg != sc.bg {
                bg_mesh.add_colored_rect(Rect::from_min_size(pos, Vec2::new(cw * w, ch)), bg);
            }
            let c = cell.c;
            let is_bold = flags.contains(Flags::BOLD);
            if flags.contains(Flags::HIDDEN) || c == ' ' || c == '\0' {
                cur = None;
                continue;
            }
            if block(&mut blocks, c, Rect::from_min_size(pos, Vec2::new(cw, ch)), fg) {
                cur = None;
                continue;
            }
            if c.is_ascii() && w == 1.0 {
                match (&mut cur, runs.last_mut()) {
                    (Some((r, end, fc, b)), Some(run)) if *r == row && *end == col && *fc == fg && *b == is_bold => {
                        run.1.push(c);
                        *end += 1;
                    }
                    _ => {
                        runs.push((pos, c.to_string(), fg, is_bold));
                        cur = Some((row, col + 1, fg, is_bold));
                    }
                }
            } else {
                cur = None;
                singles.push((pos, c, fg, is_bold, w));
            }
            if flags.intersects(Flags::ALL_UNDERLINES) {
                blocks.add_colored_rect(
                    Rect::from_min_size(pos + Vec2::new(0.0, ch - 2.0), Vec2::new(cw * w, 1.0)),
                    fg,
                );
            }
        }
        painter.add(bg_mesh);
        painter.add(blocks);
        for (pos, s, c, b) in runs {
            painter.text(pos, egui::Align2::LEFT_TOP, s, if b { bold.clone() } else { font.clone() }, c);
        }
        for (pos, ch_, c, b, w) in singles {
            let f = if b { bold.clone() } else { font.clone() };
            painter.text(pos + Vec2::new(cw * w / 2.0, 0.0), egui::Align2::CENTER_TOP, ch_, f, c);
        }
        // the cursor, where the program wants one
        if content.mode.contains(TermMode::SHOW_CURSOR) && off == 0 {
            let p = content.cursor.point;
            let pos = Pos2::new(inner.left() + p.column.0 as f32 * cw, inner.top() + (p.line.0 + off) as f32 * ch);
            let r = Rect::from_min_size(pos, Vec2::new(cw, ch));
            if focused {
                painter.rect_filled(r, 0.0, sc.fg.gamma_multiply(0.7));
            } else {
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, sc.fg.gamma_multiply(0.5)), egui::StrokeKind::Inside);
            }
        }
    }
}

fn rgb32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

// block elements and the lines Claude Code draws with, as rectangles, so they meet across cells
fn block(m: &mut egui::Mesh, c: char, r: Rect, col: Color32) -> bool {
    let (x0, y0, w, h) = (r.left(), r.top(), r.width(), r.height());
    let mut q = |fx0: f32, fy0: f32, fx1: f32, fy1: f32| {
        m.add_colored_rect(
            Rect::from_min_max(Pos2::new(x0 + fx0 * w, y0 + fy0 * h), Pos2::new(x0 + fx1 * w, y0 + fy1 * h)),
            col,
        )
    };
    let quads = |q: &mut dyn FnMut(f32, f32, f32, f32), bits: u8| {
        if bits & 1 != 0 {
            q(0.0, 0.0, 0.5, 0.5)
        }
        if bits & 2 != 0 {
            q(0.5, 0.0, 1.0, 0.5)
        }
        if bits & 4 != 0 {
            q(0.0, 0.5, 0.5, 1.0)
        }
        if bits & 8 != 0 {
            q(0.5, 0.5, 1.0, 1.0)
        }
    };
    let t = (1.0 / h).max(0.07); // a hairline, one point thick
    match c {
        '█' => q(0.0, 0.0, 1.0, 1.0),
        '▀' => q(0.0, 0.0, 1.0, 0.5),
        '▄' => q(0.0, 0.5, 1.0, 1.0),
        '▌' => q(0.0, 0.0, 0.5, 1.0),
        '▐' => q(0.5, 0.0, 1.0, 1.0),
        '▘' => quads(&mut q, 1),
        '▝' => quads(&mut q, 2),
        '▖' => quads(&mut q, 4),
        '▗' => quads(&mut q, 8),
        '▚' => quads(&mut q, 1 | 8),
        '▞' => quads(&mut q, 2 | 4),
        '▛' => quads(&mut q, 1 | 2 | 4),
        '▜' => quads(&mut q, 1 | 2 | 8),
        '▙' => quads(&mut q, 1 | 4 | 8),
        '▟' => quads(&mut q, 2 | 4 | 8),
        '─' => q(0.0, 0.5 - t / 2.0, 1.0, 0.5 + t / 2.0),
        '│' => q(0.5 - 0.06, 0.0, 0.5 + 0.06, 1.0),
        _ => return false,
    }
    true
}

