//! A terminal tile's body: a session (`claude attach`) or your shell at a screen, a pty from burrow-core shown by
//! burrow-term's view. Its bytes are watched on their way in: `exit` in an attached session does not end the attach
//! but drops into Claude Code's agent view ("describe a task for a new session"); the room is that view here, so the
//! tile closes instead (the session keeps running, as in Claude Code). The attach ending closes it too.

use burrow_core::{Hub, ShellTerm, Term};
use burrow_term::{TermEvent, TermView, Theme};
use gpui::{AppContext, Context, Entity, EventEmitter, Focusable, IntoElement, ParentElement, Render, Styled, Window, div};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// what keeps the pty: dropping it closes an attach (its process is hung up) or detaches from a shell (which stays
/// with the hub)
enum Conn {
    Attach(Term),
    Shell(ShellTerm),
}

impl Conn {
    fn write(&self, b: &[u8]) {
        match self {
            Conn::Attach(t) => t.write(b),
            Conn::Shell(t) => t.write(b),
        }
    }
    fn resize(&self, c: u16, r: u16) {
        match self {
            Conn::Attach(t) => t.resize(c, r),
            Conn::Shell(t) => t.resize(c, r),
        }
    }
}

/// what a tile's terminal tells the app
pub enum TileEvent {
    /// you typed (Enter: sent)
    Typed { enter: bool },
}

pub struct TermTile {
    pub view: Option<Entity<TermView>>,
    _conn: Option<Arc<Conn>>,
    /// the program ended, or the session went to agent view: the tile should close
    pub ended: Arc<AtomicBool>,
    /// it ended at once: it never attached (what it said stays, until you close it)
    pub failed: bool,
    pub opened: Instant,
    pub shell: bool,
    /// why it could not start
    pub error: Option<String>,
}

impl EventEmitter<TileEvent> for TermTile {}

/// the agent view's words, in what an attached session prints: it was detached (`exit`)
fn agent_view(tail: &str) -> bool {
    tail.contains("describe a task for a new session") || tail.contains("enter to return · space to reply")
}

impl TermTile {
    pub fn bot(hub: Arc<Hub>, m: &str, id: &str, window: &mut Window, cx: &mut Context<crate::burrow::Burrow>) -> Entity<TermTile> {
        let (m, id) = (m.to_string(), id.to_string());
        let light = hub.state().machine(&m).is_some_and(|x| x.light);
        cx.new(|cx| match hub.attach(&m, &id, 120, 40) {
            Ok(t) => TermTile::start(Conn::Attach(t), false, light, window, cx),
            Err(e) => TermTile::broken(e, false),
        })
    }

    pub fn shell(hub: Arc<Hub>, m: &str, dir: &str, window: &mut Window, cx: &mut Context<crate::burrow::Burrow>) -> Entity<TermTile> {
        let (m, dir) = (m.to_string(), dir.to_string());
        let light = hub.state().machine(&m).is_some_and(|x| x.light);
        cx.new(|cx| match hub.shell(&m, &dir) {
            Ok(t) => TermTile::start(Conn::Shell(t), true, light, window, cx),
            Err(e) => TermTile::broken(e, true),
        })
    }

    fn broken(e: String, shell: bool) -> TermTile {
        TermTile { view: None, _conn: None, ended: Arc::new(AtomicBool::new(false)), failed: true, opened: Instant::now(), shell, error: Some(e) }
    }

    fn start(mut conn: Conn, shell: bool, light: bool, window: &mut Window, cx: &mut Context<TermTile>) -> TermTile {
        // the pty's output, watched on its way to the view on a thread of its own
        let (_keep, dummy) = std::sync::mpsc::channel();
        let rx = match &mut conn {
            Conn::Attach(t) => std::mem::replace(&mut t.output, dummy),
            Conn::Shell(t) => std::mem::replace(&mut t.output, dummy),
        };
        let conn = Arc::new(conn);
        let ended = Arc::new(AtomicBool::new(false));
        let (tx, frx) = futures::channel::mpsc::unbounded();
        {
            let ended = ended.clone();
            std::thread::spawn(move || {
                let mut tail = String::new();
                while let Ok(b) = rx.recv() {
                    if !shell {
                        tail.push_str(&strip(&String::from_utf8_lossy(&b)));
                        if tail.len() > 600 {
                            let cut = tail.len() - 300;
                            let cut = (cut..tail.len()).find(|&i| tail.is_char_boundary(i)).unwrap_or(0);
                            tail = tail[cut..].to_string();
                        }
                        if agent_view(&tail) {
                            break;
                        }
                    }
                    if tx.unbounded_send(b).is_err() {
                        break;
                    }
                }
                ended.store(true, Ordering::Relaxed);
            });
        }
        let c2 = conn.clone();
        let view = cx.new(|cx| {
            let mut v = TermView::new(move |b: &[u8]| c2.write(b), window, cx);
            v.feed_from(frx, cx);
            v.set_theme(if light { Theme::Light } else { Theme::Dark }, cx);
            v
        });
        let c3 = conn.clone();
        cx.subscribe(&view, move |_, _, ev: &TermEvent, cx| match ev {
            TermEvent::Resize { cols, rows } => c3.resize(*cols, *rows),
            TermEvent::Link(url) => {
                if url.starts_with("http://") || url.starts_with("https://") {
                    cx.open_url(url);
                }
            }
            TermEvent::Typed { enter } => cx.emit(TileEvent::Typed { enter: *enter }),
            TermEvent::Title(_) | TermEvent::Bell => {}
        })
        .detach();
        TermTile { view: Some(view), _conn: Some(conn), ended, failed: false, opened: Instant::now(), shell, error: None }
    }

    /// the program is gone (or went to agent view): an attach that ended at once never attached, and stays to say so
    pub fn done(&mut self) -> bool {
        if self.failed || !self.ended.load(Ordering::Relaxed) {
            return false;
        }
        if self.opened.elapsed() < Duration::from_secs(3) && !self.shell {
            self.failed = true;
            return false;
        }
        true
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<TermTile>) {
        if let Some(v) = &self.view {
            window.focus(&v.focus_handle(cx));
        }
    }

    /// keys typed into it as they are (no paste)
    pub fn say(&self, b: &[u8]) {
        if let Some(c) = &self._conn {
            c.write(b);
        }
    }

    /// text handed to the program once it is up (a dropped file's path, a pair link): pasted as a terminal pastes
    /// (bracketed once the program turned that on; a session being restarted takes a few seconds: one at least,
    /// ten at most)
    pub fn hand(&mut self, text: String, cx: &mut Context<TermTile>) {
        let Some(view) = self.view.clone() else { return };
        let started = self.opened;
        cx.spawn(async move |_, cx| {
            if started.elapsed() < Duration::from_secs(1) {
                cx.background_executor().timer(Duration::from_secs(1) - started.elapsed()).await;
            }
            for _ in 0..18 {
                if view.read_with(cx, |v, _| v.is_bracketed_paste()).unwrap_or(true) {
                    break;
                }
                cx.background_executor().timer(Duration::from_millis(500)).await;
            }
            let _ = view.update(cx, |v, cx| v.paste(&text, cx));
        })
        .detach();
    }
}

/// escape sequences out, for reading what the program says
fn strip(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(&d) = it.peek() {
                    it.next();
                    if d.is_ascii_alphabetic() || d == '~' {
                        break;
                    }
                }
            } else if it.peek() == Some(&']') {
                for d in it.by_ref() {
                    if d == '\x07' {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

impl Render for TermTile {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        match (&self.view, &self.error) {
            (Some(v), _) => div().size_full().child(v.clone()),
            (None, e) => div()
                .size_full()
                .p_3()
                .font_family(crate::kit::MONO)
                .text_color(crate::kit::hsla(crate::kit::col::TISSUE2))
                .child(e.clone().unwrap_or_default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_view_is_seen_through_the_escapes() {
        let raw = "\x1b[2J\x1b[1;1H\x1b[38;5;246mdescribe a task\x1b[0m for a new session";
        assert!(!agent_view(raw));
        assert!(agent_view(&strip(raw)));
        assert!(!agent_view(&strip("all good")));
    }
}
