//! The view's panels, one module each (web/index.html's #v… panels and their fill… functions). `body` draws the
//! one that is open; `opened`, `closed` and `key` are their lifecycle. What a panel keeps between frames is in
//! `State` (the view's `p`): its fields, its list, what it asked the hub and is waiting for.

pub mod board;
pub mod chess;
pub mod clock;
pub mod closet;
pub mod desk;
pub mod dir;
pub mod doors;
pub mod games;
pub mod limits;
pub mod machine;
pub mod manager;
pub mod pair;
pub mod report;
pub mod report_text;
pub mod settings;
pub mod suggest;
pub mod you;

use crate::burrow::Burrow;
use crate::field::{Field, FieldEvent, Look};
use crate::kit::{self, col};
use crate::views::ViewKind;
use burrow_core::Hub;
use gpui::{AnyElement, AppContext, Context, Entity, FocusHandle, IntoElement, KeyDownEvent, ParentElement, RenderImage, SharedString, Styled, Subscription, Window, div, prelude::*, px};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// the text fields there are, one each, kept while the app runs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Dir,
    Host,
    YouName,
    DeskName,
    PairRoom,
    PairUrl,
}

/// what the panels keep between frames
#[derive(Default)]
pub struct State {
    pub fields: HashMap<Slot, Entity<Field>>,
    subs: Vec<Subscription>,
    /// the panel's own focus: the games' keys, up and down
    pub focus: Option<FocusHandle>,
    /// a call to the hub under way ("…" under the form)
    pub busy: bool,
    /// the list under a field
    pub list: suggest::List,
    /// the new desk: its machine, that machine's home, where Claude Code ran lately there, folders by (m, dir)
    pub dir_m: String,
    pub home: String,
    pub recent: Vec<String>,
    pub kids: HashMap<(String, String), Vec<String>>,
    pub asking: Vec<(String, String)>,
    /// the ssh config's hosts without a door
    pub hosts: Vec<String>,
    /// where the new door, or the reception, was let go from the closet
    pub machine_at: Option<[i32; 2]>,
    pub manager_at: Option<[i32; 2]>,
    pub styles: Vec<burrow_core::office::StyleView>,
    /// your face: the seed's start, its number, whether the seed had one
    pub you_start: String,
    pub you_i: i64,
    pub you_numbered: bool,
    pub you_named: String,
    /// the switchboard's line explained
    pub help: Option<usize>,
    /// a door's remove pressed once: which, when
    pub door_armed: Option<(String, Instant)>,
    pub report: Option<report::Printing>,
    pub chess: chess::Ui,
    pub game: games::Pics,
    pub board: board::Painting,
    /// the pair room opened in the browser (its link)
    pub pair_opened: Option<String>,
    /// the duel at the window, or ping-pong, is yours to play (the keys' hint shows)
    pub yours: bool,
    /// how wide the view is (measured as it was drawn)
    pub width: std::rc::Rc<std::cell::Cell<f32>>,
    /// pictures that were replaced, to let go of on the GPU
    pub stale: Vec<Arc<RenderImage>>,
}

/// a field of a slot, made the first time it is wanted
pub fn field(b: &mut Burrow, slot: Slot, window: &mut Window, cx: &mut Context<Burrow>) -> Entity<Field> {
    if let Some(f) = b.views.p.fields.get(&slot) {
        return f.clone();
    }
    let f = cx.new(|cx| Field::new(false, Look::Steel, window, cx));
    let s = cx.subscribe_in(&f, window, move |b: &mut Burrow, _, ev: &FieldEvent, window, cx| on_field(b, slot, *ev, window, cx));
    b.views.p.subs.push(s);
    b.views.p.fields.insert(slot, f.clone());
    f
}

/// a field set (empty, mostly), with its placeholder
pub fn reset(b: &mut Burrow, slot: Slot, text: &str, placeholder: &str, window: &mut Window, cx: &mut Context<Burrow>) -> Entity<Field> {
    let f = field(b, slot, window, cx);
    let ph: SharedString = placeholder.to_string().into();
    f.update(cx, |f, cx| {
        f.placeholder = ph;
        f.set(text, cx);
    });
    f
}

pub fn text(b: &Burrow, slot: Slot, cx: &Context<Burrow>) -> String {
    b.views.p.fields.get(&slot).map(|f| f.read(cx).text().to_string()).unwrap_or_default()
}

pub fn focused(b: &Burrow, slot: Slot, window: &Window, cx: &Context<Burrow>) -> bool {
    b.views.p.fields.get(&slot).is_some_and(|f| f.read(cx).focused(window))
}

fn on_field(b: &mut Burrow, slot: Slot, ev: FieldEvent, window: &mut Window, cx: &mut Context<Burrow>) {
    match (slot, ev) {
        (Slot::Dir, FieldEvent::Changed) if b.view == Some(ViewKind::Dir) => dir::refresh(b, window, cx),
        (Slot::Host, FieldEvent::Changed | FieldEvent::Focus) if b.view == Some(ViewKind::Machine) => machine::look(b, cx),
        (Slot::YouName, FieldEvent::Blur) if b.view == Some(ViewKind::You) => you::save_name(b, window, cx),
        _ => {}
    }
    cx.notify();
}

/// a call to the hub that may take a while (ssh, claude): off the main thread, then `then` with what it said
pub fn call<T: Send + 'static>(
    b: &Burrow,
    f: impl FnOnce(Arc<Hub>) -> T + Send + 'static,
    then: impl FnOnce(&mut Burrow, T, &mut Window, &mut Context<Burrow>) + 'static,
    window: &mut Window,
    cx: &mut Context<Burrow>,
) {
    let hub = b.hub.clone();
    let task = cx.background_executor().spawn(async move { f(hub) });
    cx.spawn_in(window, async move |this, cx| {
        let r = task.await;
        let _ = this.update_in(cx, |b, window, cx| {
            then(b, r, window, cx);
            cx.notify();
        });
    })
    .detach();
}

/// trouble: the console's, never the room's
pub fn note(msg: &str) {
    eprintln!("burrow: {msg}");
}

/// the panel's own focus handle, made once
pub fn own_focus(b: &mut Burrow, cx: &mut Context<Burrow>) -> FocusHandle {
    b.views.p.focus.get_or_insert_with(|| cx.focus_handle()).clone()
}

pub fn opened(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.busy = false;
    b.views.p.list.clear();
    let f = own_focus(b, cx);
    window.focus(&f);
    match kind {
        ViewKind::Dir => dir::opened(b, window, cx),
        ViewKind::Machine => machine::opened(b, window, cx),
        ViewKind::You => you::opened(b, window, cx),
        ViewKind::Settings => b.views.p.help = None,
        ViewKind::Desk => desk::opened(b, window, cx),
        ViewKind::Manager => manager::opened(b),
        ViewKind::Report => report::opened(b),
        ViewKind::Pair => pair::opened(b, window, cx),
        ViewKind::Chess => chess::opened(b, window, cx),
        ViewKind::Pong => games::pong_opened(b, window, cx),
        ViewKind::Outside => games::outside_opened(b),
        _ => {}
    }
}

pub fn closed(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) {
    let now = b.now_ms();
    match kind {
        ViewKind::Chess => chess::closed(b),
        ViewKind::Pong => b.pong.close(),
        ViewKind::Outside => b.outside.close(),
        // the new door goes back into the closet, the booth too with nobody picked
        ViewKind::Machine => {
            if b.views.p.machine_at.take().is_some() {
                b.room.pend(None, None, "door", now);
            }
        }
        ViewKind::Manager => {
            if b.views.p.manager_at.take().is_some() {
                b.room.pend(None, None, "manager", now);
            }
        }
        ViewKind::Report => b.views.p.report = None,
        // (opened again, the chat opens in the browser again)
        ViewKind::Pair => b.views.p.pair_opened = None,
        // the name as it was left (the page's onchange): the field's blur comes once the view is gone
        ViewKind::You => you::save_name(b, window, cx),
        _ => {}
    }
    b.views.p.list.clear();
    // the keys go back to the window
    window.focus(&b.focus);
}

/// keys while the view is open (the root's, after the panel's own): the games take theirs
pub fn key(b: &mut Burrow, kind: ViewKind, e: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Burrow>) {
    if matches!(kind, ViewKind::Pong | ViewKind::Outside) {
        games::key(b, kind, &e.keystroke, true, e.is_held, cx);
    }
}

/// the printer's report has no panel round it
pub fn frameless(_b: &Burrow, kind: ViewKind) -> bool {
    kind == ViewKind::Report
}

pub fn body(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    for old in b.views.p.stale.drain(..) {
        let _ = window.drop_image(old);
    }
    let focus = own_focus(b, cx);
    let width = b.views.p.width.clone();
    let inner: AnyElement = match kind {
        ViewKind::Dir => dir::body(b, window, cx),
        ViewKind::Machine => machine::body(b, window, cx),
        ViewKind::You => you::body(b, window, cx),
        ViewKind::Closet => closet::body(b, window, cx),
        ViewKind::Report => report::body(b, window, cx),
        ViewKind::Limits => limits::body(b),
        ViewKind::Settings => settings::body(b, cx),
        ViewKind::Doors => doors::body(b, window, cx),
        ViewKind::Desk => desk::body(b, window, cx),
        ViewKind::Manager => manager::body(b, window, cx),
        ViewKind::Clock => clock::body(b, window, cx),
        ViewKind::Outside => games::outside_body(b, window, cx),
        ViewKind::Board => board::body(b, window, cx),
        ViewKind::Pair => pair::body(b, window, cx),
        ViewKind::Chess => chess::body(b, window, cx),
        ViewKind::Pong => games::pong_body(b, window, cx),
    };
    div()
        .id("panel")
        .track_focus(&focus)
        .relative()
        .flex()
        .flex_col()
        .when(kind == ViewKind::Report, |d| d.size_full())
        .capture_key_down(cx.listener(move |b, e: &KeyDownEvent, window, cx| form_key(b, kind, e, window, cx)))
        .on_key_up(cx.listener(move |b, e: &gpui::KeyUpEvent, _, cx| {
            if matches!(kind, ViewKind::Pong | ViewKind::Outside) {
                games::key(b, kind, &e.keystroke, false, false, cx);
            }
        }))
        .child(gpui::canvas(move |bd, _, _| width.set(f32::from(bd.size.width)), |_, _, _, _| {}).absolute().size_full())
        .child(inner)
        .into_any_element()
}

/// a key in a form, before its field has it: the list under the field, Enter that adds
fn form_key(b: &mut Burrow, kind: ViewKind, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Burrow>) {
    let k = &e.keystroke;
    if k.modifiers.platform || k.modifiers.control || k.modifiers.alt {
        return;
    }
    let key = k.key.as_str();
    let done = match kind {
        ViewKind::Dir if focused(b, Slot::Dir, window, cx) => dir::key(b, key, window, cx),
        ViewKind::Machine if focused(b, Slot::Host, window, cx) => machine::key(b, key, window, cx),
        ViewKind::You if key == "enter" => {
            you::save_name(b, window, cx);
            true
        }
        ViewKind::Desk if key == "enter" => {
            desk::rename(b, window, cx);
            true
        }
        ViewKind::Pair if key == "enter" => pair::enter(b, window, cx),
        _ => false,
    };
    if done {
        cx.stop_propagation();
        cx.notify();
    }
}

/// the line under a form: what went wrong, or "…" while it is asked
pub fn err_line(b: &Burrow) -> gpui::Div {
    let s = if b.views.p.busy && b.views.err.is_empty() { "…".to_string() } else { b.views.err.clone() };
    div().min_h(px(22.)).mt(px(8.)).child(kit::pixel_para(s, col::TISSUE2))
}

/// a row of buttons
pub fn row() -> gpui::Div {
    div().flex().flex_wrap().gap(px(8.)).mt(px(10.))
}

/// the idle bots, for a game: not at work, not waiting for you
pub fn idle_bots(b: &Burrow) -> Vec<burrow_model::Bot> {
    b.state.bots.iter().filter(|x| !x.busy && !x.waiting).cloned().collect()
}

/// one of them by lot
pub fn by_lot(b: &mut Burrow) -> Option<burrow_model::Bot> {
    let who = idle_bots(b);
    if who.is_empty() {
        return None;
    }
    let i = ((b.room.dice.next() * who.len() as f64) as usize).min(who.len() - 1);
    Some(who[i].clone())
}

/// a bot's project colour
pub fn color_of(b: &Burrow, id: &str) -> pix::Color {
    burrow_room::layout::project_color(b.state.bots.iter().find(|x| x.id == id).map(|x| x.unit.as_str()).unwrap_or(""))
}
