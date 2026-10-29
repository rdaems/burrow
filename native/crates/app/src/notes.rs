//! The office manager's notes (Linda), a tile at the screen's resolution (web/index.html's `openNotes`,
//! `renderNotes`, `decide`): the room is the overview, this is the depth. Two panes: on the left what needs you
//! (every open decision a line, the ones a session waits on first), then who works, then the rest, a session a
//! line; on the right the notes of what was clicked there, the decision picked lit up. Under each open decision:
//! an explainer (Claude, kept with the notes), its options to pick into the answer, the answer sent (typed
//! into the session by the hub).

use crate::burrow::Burrow;
use crate::field::{Field, FieldEvent, Look};
use crate::kit::{self, col};
use burrow_core::Hub;
use burrow_model::{Bot, Desk, State};
use gpui::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    Subscription, WeakEntity, Window, div, point, prelude::*, px, rgb, rgba,
};
use serde_json::{Map, Value, json};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

// ---- what the notes say, without pixels

/// a session needs you when a prompt is open in it, or when its last message asks you something (tempo
/// blocked; the room shows that one idle, as Claude Code's agent view does)
pub fn needs_you(b: &Bot) -> bool {
    b.waiting || b.tempo == "blocked"
}

/// its state as a word, and the class it is drawn with
pub fn state_of(b: &Bot) -> (&'static str, String) {
    if needs_you(b) {
        ("needs", "needs you".into())
    } else if b.busy {
        ("busy", "working".into())
    } else if !b.over.is_empty() {
        ("over", b.over.clone())
    } else if !b.alive {
        ("off", "process off".into())
    } else {
        ("idle", "idle".into())
    }
}

/// up to the first full stop (or ! or ?) that a space or the end follows
pub fn first_sentence(t: &str) -> String {
    let b = t.as_bytes();
    for (i, c) in t.char_indices() {
        if matches!(c, '.' | '!' | '?') && b.get(i + 1).is_none_or(|n| n.is_ascii_whitespace()) {
            return t[..=i].trim().to_string();
        }
    }
    t.trim().to_string()
}

fn s<'a>(e: &'a Value, k: &str) -> &'a str {
    e.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn headline(e: Option<&Value>) -> String {
    let Some(e) = e else { return String::new() };
    let h = s(e, "headline");
    if h.is_empty() { first_sentence(s(e, "now")) } else { h.to_string() }
}

/// a desk as the room names it
pub fn desk_name(d: Option<&Desk>) -> String {
    let Some(d) = d else { return "?".into() };
    if !d.name.is_empty() {
        return d.name.clone();
    }
    let t = d.dir.trim_end_matches('/');
    let home = t == "~" || t == "/root" || {
        let parts: Vec<&str> = t.split('/').collect();
        parts.len() == 3 && parts[0].is_empty() && (parts[1] == "Users" || parts[1] == "home") && !parts[2].is_empty()
    };
    if home { "home".into() } else { t.rsplit('/').next().unwrap_or(t).to_string() }
}

/// a path with the home as ~
pub fn tilde(p: &str) -> String {
    for pre in ["/Users/", "/home/"] {
        if let Some(rest) = p.strip_prefix(pre) {
            return match rest.find('/') {
                Some(i) => format!("~{}", &rest[i..]),
                None => "~".into(),
            };
        }
    }
    p.to_string()
}

pub fn ago(sec: f64) -> String {
    if sec < 90. {
        format!("{} s", sec.round())
    } else if sec < 5400. {
        format!("{} min", (sec / 60.).round())
    } else if sec < 48. * 3600. {
        format!("{:.1} h", sec / 3600.)
    } else {
        format!("{} d", (sec / 86400.).round())
    }
}

/// what a session waits on: the decision's own flag, else (notes from before the flag) whether it needs you
pub fn blocking(b: &Bot, q: &Value) -> bool {
    q.get("blocking").and_then(Value::as_bool).unwrap_or_else(|| needs_you(b))
}

/// a session with notes (or being written)
#[derive(Clone, Debug)]
pub struct Item {
    pub b: Bot,
    pub e: Option<Value>,
    pub desk: Option<Desk>,
}

/// an open decision: whose (its item), which of its decisions
#[derive(Clone, Debug)]
pub struct Dec {
    pub it: usize,
    pub i: usize,
    pub q: Value,
}

/// the notes laid out: the sessions in their order, the open decisions waited on and the rest
pub struct Model {
    pub items: Vec<Item>,
    pub now: Vec<Dec>,
    pub later: Vec<Dec>,
}

fn answered(e: &Value, q: &Value) -> bool {
    e.get("answered").and_then(|a| a.get(s(q, "question"))).is_some_and(|x| !x.is_null())
}

pub fn decisions(e: &Value) -> &[Value] {
    e.get("decisions").and_then(Value::as_array).map(|v| v.as_slice()).unwrap_or(&[])
}

/// the sessions that wait on you first, then those at work, then the idle, then the ones over; the freshest notes first
pub fn model(state: &State, sessions: &Map<String, Value>, writing: Option<&str>) -> Model {
    let rank = |b: &Bot| if needs_you(b) { 0 } else if b.busy { 1 } else if b.over.is_empty() { 2 } else { 3 };
    let at = |e: &Option<Value>| e.as_ref().and_then(|e| e.get("at")).and_then(Value::as_f64).unwrap_or(0.);
    let mut items: Vec<Item> = state
        .bots
        .iter()
        .filter(|b| sessions.contains_key(&b.id) || writing == Some(b.id.as_str()))
        .map(|b| Item { b: b.clone(), e: sessions.get(&b.id).cloned(), desk: state.desks.iter().find(|d| d.unit == b.unit).cloned() })
        .collect();
    items.sort_by(|x, y| rank(&x.b).cmp(&rank(&y.b)).then(at(&y.e).partial_cmp(&at(&x.e)).unwrap_or(std::cmp::Ordering::Equal)));
    let mut now = vec![];
    let mut later = vec![];
    for (k, it) in items.iter().enumerate() {
        let Some(e) = &it.e else { continue };
        for (i, q) in decisions(e).iter().enumerate() {
            if answered(e, q) {
                continue;
            }
            let d = Dec { it: k, i, q: q.clone() };
            if blocking(&it.b, q) { now.push(d) } else { later.push(d) }
        }
    }
    Model { items, now, later }
}

impl Model {
    /// what the detail shows: the session picked if it is still there, else the first decision waited on (or any),
    /// else the first session; and the decision lit (the one waited on, when that made the pick)
    pub fn pick(&self, sel: Option<&str>, dec: Option<usize>) -> Option<(usize, Option<usize>)> {
        if self.items.is_empty() {
            return None;
        }
        if let Some(k) = self.items.iter().position(|it| Some(it.b.id.as_str()) == sel) {
            return Some((k, dec));
        }
        let k = self.now.first().or(self.later.first()).map(|d| d.it).unwrap_or(0);
        let lit = self.now.first().filter(|d| d.it == k).map(|d| d.i);
        Some((k, lit))
    }
}

// ---- the tile

type Key = (String, String);

pub struct Notes {
    pub hub: Arc<Hub>,
    burrow: WeakEntity<Burrow>,
    sel: Option<String>,
    dec: Option<usize>,
    /// the answers being written, by (session, question); an option picked (its number, its text)
    drafts: HashMap<Key, Entity<Field>>,
    picked: HashMap<Key, (usize, String)>,
    /// explaining or sending, what went wrong
    busy: HashMap<Key, &'static str>,
    errs: HashMap<Key, String>,
    /// what came back before the notes were pushed again
    explained: HashMap<Key, Value>,
    answered: HashMap<Key, (String, f64)>,
    project_open: bool,
    /// the detail starts at its top, or at the decision picked
    jump: bool,
    nav: ScrollHandle,
    detail: ScrollHandle,
    width: Rc<Cell<f32>>,
    subs: Vec<Subscription>,
}

fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.)
}

const NEEDS: u32 = 0xffb3a6;

impl Notes {
    pub fn new(hub: Arc<Hub>, burrow: WeakEntity<Burrow>, _cx: &mut Context<Notes>) -> Notes {
        Notes {
            hub,
            burrow,
            sel: None,
            dec: None,
            drafts: HashMap::new(),
            picked: HashMap::new(),
            busy: HashMap::new(),
            errs: HashMap::new(),
            explained: HashMap::new(),
            answered: HashMap::new(),
            project_open: false,
            jump: false,
            nav: ScrollHandle::new(),
            detail: ScrollHandle::new(),
            width: Rc::new(Cell::new(0.)),
            subs: Vec::new(),
        }
    }

    /// a name opens the session beside the notes
    fn open(&self, id: &str, window: &mut Window, cx: &mut Context<Notes>) {
        let id = id.to_string();
        let _ = self.burrow.update(cx, |b, cx| {
            if let Some(x) = b.state.bots.iter().find(|x| x.id == id).cloned() {
                b.open_bot(&x.m, &x.id, true, None, window, cx);
            }
        });
    }

    fn draft(&mut self, k: &Key, placeholder: &str, window: &mut Window, cx: &mut Context<Notes>) -> Entity<Field> {
        if let Some(f) = self.drafts.get(k) {
            return f.clone();
        }
        let f = cx.new(|cx| Field::new(true, Look::Notes, window, cx));
        let ph: SharedString = placeholder.to_string().into();
        f.update(cx, |f, _| f.placeholder = ph);
        let key = k.clone();
        // typing over a picked option un-picks it
        self.subs.push(cx.subscribe(&f, move |n: &mut Notes, f, ev: &FieldEvent, cx| {
            if *ev == FieldEvent::Changed && n.picked.get(&key).is_some_and(|(_, t)| t != f.read(cx).text()) {
                n.picked.remove(&key);
                cx.notify();
            }
        }));
        self.drafts.insert(k.clone(), f.clone());
        f
    }

    fn explain(&mut self, id: String, q: String, cx: &mut Context<Notes>) {
        let k = (id.clone(), q.clone());
        self.busy.insert(k.clone(), "explain");
        self.errs.remove(&k);
        let hub = self.hub.clone();
        let task = cx.background_executor().spawn(async move { hub.explain(&id, &q) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            let _ = this.update(cx, |n, cx| {
                n.busy.remove(&k);
                match r {
                    Ok(x) => {
                        n.explained.insert(k, x);
                    }
                    Err(e) => {
                        n.errs.insert(k, e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn send(&mut self, b: &Bot, q: String, cx: &mut Context<Notes>) {
        let k = (b.id.clone(), q.clone());
        let text = self.drafts.get(&k).map(|f| f.read(cx).text().trim().to_string()).unwrap_or_default();
        if text.is_empty() {
            self.errs.insert(k, "Write an answer, or pick an option.".into());
            cx.notify();
            return;
        }
        self.busy.insert(k.clone(), "send");
        self.errs.remove(&k);
        let (hub, id, t2) = (self.hub.clone(), b.id.clone(), text.clone());
        let task = cx.background_executor().spawn(async move { hub.answer(&id, &q, &t2) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            let _ = this.update(cx, |n, cx| {
                n.busy.remove(&k);
                match r {
                    Ok(()) => {
                        n.drafts.remove(&k);
                        n.picked.remove(&k);
                        n.answered.insert(k, (text, now_ms()));
                    }
                    Err(e) => {
                        n.errs.insert(k, e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// the notes with what came back meanwhile laid over them
    fn sessions(&self, d: &burrow_core::DigestView) -> Map<String, Value> {
        let mut ss = d.sessions.clone();
        for ((id, q), x) in &self.explained {
            if let Some(e) = ss.get_mut(id).and_then(Value::as_object_mut)
                && let Some(o) = e.entry("explained").or_insert(json!({})).as_object_mut()
            {
                o.entry(q.clone()).or_insert(x.clone());
            }
        }
        for ((id, q), (t, at)) in &self.answered {
            if let Some(e) = ss.get_mut(id).and_then(Value::as_object_mut)
                && let Some(o) = e.entry("answered").or_insert(json!({})).as_object_mut()
            {
                o.entry(q.clone()).or_insert(json!({"text": t, "at": at}));
            }
        }
        ss
    }
}

/// a project's name in its colour, in a hairline box
fn proj(it: &Item) -> gpui::Div {
    let c = kit::hex(burrow_room::layout::project_color(&it.b.unit));
    div()
        .flex_none()
        .font_family(kit::MONO)
        .text_size(px(11.))
        .line_height(px(13.))
        .text_color(rgb(c))
        .border_1()
        .border_color(rgb(kit::mix(col::BOX, c, 0.55)))
        .rounded(px(3.))
        .px(px(4.))
        .py(px(1.))
        .whitespace_nowrap()
        .child(desk_name(it.desk.as_ref()))
}

fn writing(on: bool) -> Option<gpui::Div> {
    // blinking, as the page's steps(2)
    let lit = (now_ms() / 600.) as i64 % 2 == 0;
    on.then(|| div().text_size(px(12.)).text_color(rgb(col::GOLD2)).opacity(if lit { 1. } else { 0.35 }).child("writing…"))
}

fn h2(title: &str, needs: bool, count: Option<usize>) -> gpui::Div {
    div()
        .flex()
        .gap(px(8.))
        .items_center()
        .mb(px(4.))
        .ml(px(6.))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(if needs { NEEDS } else { col::DIM }))
        .child(title.to_uppercase())
        .children(count.map(|n| div().text_color(rgb(col::INK)).bg(rgb(NEEDS)).rounded(px(8.)).px(px(6.)).font_weight(kit::bold()).child(n.to_string())))
}

fn h4(title: &str, color: u32) -> gpui::Div {
    div().mt(px(12.)).mb(px(2.)).text_size(px(12.)).text_color(rgb(color)).child(title.to_uppercase())
}

fn para(t: impl Into<SharedString>, color: u32) -> gpui::Div {
    div().mb(px(6.)).text_color(rgb(color)).child(t.into())
}

/// a line in the left pane: lit when picked
fn li(id: impl Into<gpui::ElementId>, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x(px(8.))
        .gap_y(px(3.))
        .px(px(8.))
        .py(px(7.))
        .rounded(px(5.))
        .cursor_pointer()
        .when(on, |d| d.bg(rgb(col::BOX2)).child(div().absolute().left_0().top_0().bottom_0().w(px(3.)).bg(rgb(col::GOLD2))))
        .when(!on, |d| d.hover(|s| s.bg(rgba(0xffffff0d))))
}

fn button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, solid: bool, off: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(5.))
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .border_1()
        .when(solid, |d| d.bg(rgb(col::GOLD2)).text_color(rgb(col::INK)).border_color(rgb(col::GOLD2)).hover(|s| s.bg(rgb(0xfff0a8))))
        .when(!solid, |d| d.text_color(rgb(col::GOLD2)).border_color(rgb(kit::mix(col::BOX, col::GOLD2, 0.45))).hover(|s| s.bg(rgba(0xffe27a1f))))
        .when(off, |d| d.opacity(0.6))
        .when(!off, |d| d.cursor_pointer())
        .child(label.into())
}

impl Render for Notes {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(bw) = self.burrow.upgrade() else { return div().into_any_element() };
        let (state, digest) = {
            let b = bw.read(cx);
            (b.state.clone(), b.digest.clone())
        };
        let ss = self.sessions(&digest);
        let m = model(&state, &ss, digest.writing.as_deref());
        let width = self.width.clone();
        let base = div()
            .size_full()
            .relative()
            .font_family(kit::SANS)
            .text_size(px(15.))
            .line_height(px(23.))
            .text_color(rgb(col::TEXT))
            .child(gpui::canvas(move |bd, _, _| width.set(f32::from(bd.size.width)), |_, _, _, _| {}).absolute().size_full());
        let Some((k, dec)) = m.pick(self.sel.as_deref(), self.dec) else {
            let t = if !digest.on {
                "Notes are switched off at the switchboard."
            } else if digest.writing.is_some() {
                "Writing the first notes…"
            } else {
                "No notes yet. They are written when a session finishes a turn or asks you something, and when this opens."
            };
            return base.child(div().px(px(24.)).py(px(18.)).text_color(rgb(col::DIM)).child(t)).into_any_element();
        };
        if self.sel.as_deref() != Some(m.items[k].b.id.as_str()) {
            self.sel = Some(m.items[k].b.id.clone());
            self.dec = dec;
        }
        let wr = digest.writing.clone();
        let nav = self.nav_pane(&m, wr.as_deref(), cx);
        let (out, lit_at) = self.detail_pane(&m.items[k], &state, &digest, wr.as_deref(), window, cx);
        // (the page's 76ch)
        let out = out.into_iter().map(|el| div().max_w(px(660.)).child(el));
        let jump = std::mem::take(&mut self.jump);
        // a narrow tile: the list, then the notes under it
        let narrow = self.width.get() > 0. && self.width.get() < 620.;
        let body = if narrow {
            if jump {
                self.detail.set_offset(point(px(0.), px(0.)));
            }
            div()
                .id("nv")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.detail)
                .child(nav.border_b_1().border_color(rgb(col::LINE)))
                .child(div().flex().flex_col().pt(px(16.)).px(px(24.)).pb(px(32.)).children(out))
                .into_any_element()
        } else {
            // a new pick: the detail at its top, or at the decision picked (its lines are the scroller's own children)
            if jump {
                match lit_at {
                    Some(i) => self.detail.scroll_to_top_of_item(i),
                    None => self.detail.set_offset(point(px(0.), px(0.))),
                }
            }
            let w = (self.width.get() * 0.4).max(260.);
            div()
                .flex()
                .size_full()
                .child(div().id("nav").w(px(w)).flex_none().h_full().overflow_y_scroll().track_scroll(&self.nav).border_r_1().border_color(rgb(col::LINE)).child(nav))
                .child(div().id("detail").flex_1().min_w(px(0.)).h_full().flex().flex_col().pt(px(16.)).px(px(24.)).pb(px(32.)).overflow_y_scroll().track_scroll(&self.detail).children(out))
                .into_any_element()
        };
        base.child(body).into_any_element()
    }
}

impl Notes {
    fn nav_pane(&mut self, m: &Model, wr: Option<&str>, cx: &mut Context<Notes>) -> gpui::Div {
        let mut nav = div().flex().flex_col().pt(px(14.)).pr(px(10.)).pb(px(24.)).pl(px(14.));
        let dec_row = |d: &Dec, later: bool, n: &Notes, cx: &mut Context<Notes>| {
            let it = &m.items[d.it];
            let on = n.sel.as_deref() == Some(it.b.id.as_str()) && n.dec == Some(d.i);
            let (id, i) = (it.b.id.clone(), d.i);
            li(SharedString::from(format!("dq:{}:{}", it.b.id, d.i)), on)
                .on_click(cx.listener(move |n, _, _, cx| {
                    n.sel = Some(id.clone());
                    n.dec = Some(i);
                    n.jump = true;
                    cx.notify();
                }))
                .child(
                    div()
                        .w_full()
                        .text_size(px(14.))
                        .line_height(px(19.))
                        .when(!later, |d| d.font_weight(gpui::FontWeight::SEMIBOLD))
                        .when(later, |d| d.text_color(rgb(col::DIM)))
                        .child(s(&d.q, "question").to_string()),
                )
                .child(div().flex().gap(px(6.)).items_center().text_size(px(12.)).text_color(rgb(col::DIM)).child(proj(it)).child(it.b.name.clone()).children(writing(wr == Some(it.b.id.as_str()))))
        };
        let ses_row = |it: &Item, n: &Notes, cx: &mut Context<Notes>| {
            let on = n.sel.as_deref() == Some(it.b.id.as_str()) && n.dec.is_none();
            let dot = match state_of(&it.b).0 {
                "needs" => 0xff8a7a,
                "busy" => 0x7fdc8a,
                _ => col::FAINT,
            };
            let id = it.b.id.clone();
            li(SharedString::from(format!("ses:{}", it.b.id)), on)
                .on_click(cx.listener(move |n, _, _, cx| {
                    n.sel = Some(id.clone());
                    n.dec = None;
                    n.jump = true;
                    cx.notify();
                }))
                .child(div().w(px(8.)).h(px(8.)).rounded(px(4.)).bg(rgb(dot)).flex_none())
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(it.b.name.clone()))
                .child(proj(it))
                .children(writing(wr == Some(it.b.id.as_str())))
                .child(div().w_full().pl(px(16.)).text_size(px(13.)).line_height(px(18.)).text_color(rgb(col::DIM)).line_clamp(2).child(headline(it.e.as_ref())))
        };
        let mut needs = div().flex().flex_col().mb(px(16.)).child(h2("Needs you", true, (!m.now.is_empty()).then_some(m.now.len())));
        for d in &m.now {
            needs = needs.child(dec_row(d, false, self, cx));
        }
        if m.now.is_empty() && m.later.is_empty() {
            needs = needs.child(div().mx(px(6.)).mt(px(2.)).mb(px(8.)).text_size(px(14.)).text_color(rgb(col::DIM)).child("Nothing waits on you."));
        }
        nav = nav.child(needs);
        if !m.later.is_empty() {
            let mut later = div().flex().flex_col().mb(px(16.)).child(div().mt(px(8.)).mb(px(4.)).ml(px(6.)).text_size(px(12.)).text_color(rgb(col::FAINT)).child("Also open, no hurry"));
            for d in &m.later {
                later = later.child(dec_row(d, true, self, cx));
            }
            nav = nav.child(later);
        }
        let groups: [(&str, fn(&Bot) -> bool); 3] = [("Working", |b| b.busy && !needs_you(b)), ("Waiting for your answer", needs_you), ("Idle and done", |b| !b.busy && !needs_you(b))];
        for (title, f) in groups {
            let rows: Vec<&Item> = m.items.iter().filter(|it| f(&it.b)).collect();
            if rows.is_empty() {
                continue;
            }
            let mut sec = div().flex().flex_col().mb(px(16.)).child(h2(title, false, None));
            for it in rows {
                sec = sec.child(ses_row(it, self, cx));
            }
            nav = nav.child(sec);
        }
        nav
    }

    fn detail_pane(&mut self, it: &Item, state: &State, digest: &burrow_core::DigestView, wr: Option<&str>, window: &mut Window, cx: &mut Context<Notes>) -> (Vec<AnyElement>, Option<usize>) {
        let b = &it.b;
        let (cls, label) = state_of(b);
        let mut out: Vec<AnyElement> = Vec::new();
        let mut lit_at = None;
        let id = b.id.clone();
        let chip = |bg: u32, fg: u32| div().text_size(px(11.)).font_weight(gpui::FontWeight::SEMIBOLD).px(px(6.)).py(px(1.)).rounded(px(3.)).bg(rgb(bg)).text_color(rgb(fg)).whitespace_nowrap();
        let st = match cls {
            "needs" => chip(0x5a2a26, NEEDS),
            "busy" => chip(0x22402c, 0x9fe0a8),
            "over" if b.over == "failed" => chip(col::INK2, 0xff9c8c),
            "over" if b.over == "done" => chip(col::INK2, 0x9fe0a8),
            _ => chip(col::INK2, col::DIM),
        }
        .child(label.to_uppercase());
        out.push(
            div()
                .flex()
                .flex_wrap()
                .items_baseline()
                .gap_x(px(10.))
                .gap_y(px(4.))
                .child(
                    div()
                        .id("name")
                        .text_size(px(20.))
                        .line_height(px(28.))
                        .font_weight(kit::bold())
                        .cursor_pointer()
                        .hover(|s| s.text_color(rgb(col::GOLD2)))
                        .child(b.name.clone())
                        .on_click(cx.listener(move |n, _, window, cx| n.open(&id, window, cx))),
                )
                .child(st)
                .children(writing(wr == Some(b.id.as_str())))
                .into_any_element(),
        );
        let dir = it.desk.as_ref().map(|d| d.dir.clone()).unwrap_or(b.cwd.clone());
        let on = state.machine(&b.m).filter(|_| state.machines.len() > 1).map(|m| format!(" on {}", m.name)).unwrap_or_default();
        out.push(div().mt(px(2.)).mb(px(12.)).font_family(kit::MONO).text_size(px(12.)).text_color(rgb(col::DIM)).child(format!("{} · {}{on}", desk_name(it.desk.as_ref()), tilde(&dir))).into_any_element());
        // the project's paragraph, folded
        if let Some(t) = digest.desks.get(&b.unit).map(|d| s(d, "text").to_string()).filter(|t| !t.is_empty()) {
            let open = self.project_open;
            out.push(
                div()
                    .id("project")
                    .mb(px(12.))
                    .px(px(10.))
                    .py(px(8.))
                    .bg(rgb(col::BOX2))
                    .rounded(px(5.))
                    .text_size(px(14.))
                    .cursor_pointer()
                    .on_click(cx.listener(|n, _, _, cx| {
                        n.project_open = !n.project_open;
                        cx.notify();
                    }))
                    .child(div().text_color(rgb(col::DIM)).child(format!("{} The project: {}", if open { "▾" } else { "▸" }, first_sentence(&t))))
                    .when(open, |d| d.child(div().mt(px(6.)).child(t.clone())))
                    .into_any_element(),
            );
        }
        let Some(e) = &it.e else {
            out.push(div().text_color(rgb(col::DIM)).child(if wr == Some(b.id.as_str()) { "Being written…" } else { "No notes yet." }).into_any_element());
            return (out, None);
        };
        if !s(e, "goal").is_empty() {
            out.push(div().mt(px(2.)).mb(px(6.)).italic().text_color(rgb(col::DIM)).child(s(e, "goal").to_string()).into_any_element());
        }
        if !s(e, "now").is_empty() {
            out.push(h4("Now", col::DIM).into_any_element());
            out.push(para(s(e, "now").to_string(), col::TEXT).into_any_element());
        }
        let open: Vec<(usize, Value)> = decisions(e).iter().cloned().enumerate().filter(|(_, q)| !answered(e, q)).collect();
        if !open.is_empty() {
            out.push(h4("Open decisions", NEEDS).into_any_element());
            for (i, q) in open {
                if self.dec == Some(i) {
                    lit_at = Some(out.len());
                }
                let el = self.decision(b, e, &q, i, window, cx);
                out.push(el);
            }
        }
        let done: Vec<&Value> = decisions(e).iter().filter(|q| answered(e, q)).collect();
        if !done.is_empty() {
            out.push(h4("Answered", col::DIM).into_any_element());
            for q in done {
                let a = &e["answered"][s(q, "question")];
                let when = ago((now_ms() - a.get("at").and_then(Value::as_f64).unwrap_or(0.)) / 1000.);
                out.push(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(col::DIM))
                        .child(div().flex().gap(px(6.)).child(div().text_color(rgb(0x9fe0a8)).child("✓")).child(s(q, "question").to_string()))
                        .child(div().ml(px(20.)).mb(px(6.)).text_size(px(13.)).child(format!("You answered: {} · {when} ago", s(a, "text"))))
                        .into_any_element(),
                );
            }
        }
        let built: Vec<String> = e.get("built").and_then(Value::as_array).into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
        if !built.is_empty() {
            out.push(h4("Built", col::DIM).into_any_element());
            for x in built {
                out.push(div().flex().gap(px(8.)).pl(px(6.)).child(div().text_color(rgb(col::DIM)).child("•")).child(x).into_any_element());
            }
        }
        if !s(e, "next").is_empty() {
            out.push(h4("Next", col::DIM).into_any_element());
            out.push(para(s(e, "next").to_string(), col::TEXT).into_any_element());
        }
        let at = e.get("at").and_then(Value::as_f64).unwrap_or(0.);
        let read = if e.get("whole").and_then(Value::as_bool).unwrap_or(false) {
            "the whole transcript".to_string()
        } else {
            format!("its newest {}k characters", (e.get("read").and_then(Value::as_f64).unwrap_or(0.) / 1000.).round())
        };
        let id = b.id.clone();
        out.push(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.))
                .mt(px(12.))
                .text_size(px(12.))
                .text_color(rgb(col::FAINT))
                .child(div().id("src").text_color(rgb(col::GOLD2)).cursor_pointer().child("open the session →").on_click(cx.listener(move |n, _, window, cx| n.open(&id, window, cx))))
                .child(format!("· written {} ago from {read}", ago((now_ms() - at) / 1000.)))
                .into_any_element(),
        );
        (out, lit_at)
    }

    fn decision(&mut self, b: &Bot, e: &Value, q: &Value, i: usize, window: &mut Window, cx: &mut Context<Notes>) -> AnyElement {
        let question = s(q, "question").to_string();
        let k: Key = (b.id.clone(), question.clone());
        let mut d = div().relative().flex().flex_col().my(px(6.)).px(px(8.)).py(px(6.)).rounded(px(5.));
        if self.dec == Some(i) {
            d = d.bg(rgb(col::BOX2)).child(div().absolute().left_0().top_0().bottom_0().w(px(3.)).bg(rgb(col::GOLD2)));
        }
        if blocking(b, q) {
            d = d.child(div().text_size(px(11.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(NEEDS)).child("IT WAITS ON THIS"));
        }
        d = d.child(div().font_weight(kit::bold()).child(question.clone()));
        if !s(q, "context").is_empty() {
            d = d.child(div().my(px(2.)).text_color(rgb(col::DIM)).child(s(q, "context").to_string()));
        }
        // a prompt open in its session: nothing can be typed to it
        if b.dialog {
            let id = b.id.clone();
            return d
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.))
                        .my(px(6.))
                        .text_size(px(14.))
                        .text_color(rgb(col::DIM))
                        .child("It shows this as a prompt in its session.")
                        .child(div().id(("there", i)).text_color(rgb(col::GOLD2)).cursor_pointer().child("Answer it there →").on_click(cx.listener(move |n, _, window, cx| n.open(&id, window, cx)))),
                )
                .into_any_element();
        }
        let busy = self.busy.get(&k).copied();
        let mut box_ = div().flex().flex_col().gap(px(8.)).mt(px(8.)).mb(px(2.));
        match e.get("explained").and_then(|x| x.get(&question)).filter(|x| !x.is_null()) {
            Some(x) => {
                let bold = |t: &str| div().font_weight(kit::bold()).child(t.to_string());
                let mut ex = div().flex().flex_col().px(px(12.)).py(px(10.)).bg(rgb(col::BOX2)).rounded(px(6.)).text_size(px(14.)).line_height(px(21.));
                ex = ex.child(div().mb(px(6.)).child(bold("What is decided.")).child(s(x, "what").to_string())).child(div().mb(px(6.)).child(bold("Why it matters.")).child(s(x, "why").to_string()));
                for o in x.get("options").and_then(Value::as_array).into_iter().flatten() {
                    ex = ex.child(
                        div()
                            .my(px(8.))
                            .pl(px(10.))
                            .border_l_2()
                            .border_color(rgb(col::LINE))
                            .child(bold(s(o, "option")))
                            .child(div().child(s(o, "means").to_string()))
                            .child(div().text_color(rgb(0x9fe0a8)).child(format!("+ {}", s(o, "pro"))))
                            .child(div().text_color(rgb(NEEDS)).child(format!("− {}", s(o, "con")))),
                    );
                }
                ex = ex.child(div().mt(px(8.)).child(bold("Recommendation.")).child(s(x, "recommendation").to_string()));
                box_ = box_.child(ex);
            }
            None => {
                let (id, qq) = (b.id.clone(), question.clone());
                let label = if busy == Some("explain") { "Explaining…" } else { "Explain this decision" };
                box_ = box_.child(div().flex().child(button(("explain", i), label, false, busy.is_some()).when(busy.is_none(), |d| d.on_click(cx.listener(move |n, _, _, cx| n.explain(id.clone(), qq.clone(), cx))))));
            }
        }
        let options: Vec<String> = q.get("options").and_then(Value::as_array).into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
        let field = self.draft(&k, if options.is_empty() { "Your answer" } else { "Your answer, or pick an option above" }, window, cx);
        if !options.is_empty() {
            let mut opts = div().flex().flex_wrap().gap(px(6.));
            for (n_, o) in options.iter().enumerate() {
                let on = self.picked.get(&k).is_some_and(|(p, _)| *p == n_);
                let (k2, o2, f2) = (k.clone(), o.clone(), field.clone());
                opts = opts.child(
                    div()
                        .id(SharedString::from(format!("opt:{i}:{n_}")))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(5.))
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .cursor_pointer()
                        .bg(rgb(col::INK2))
                        .when(on, |d| d.border_1().border_color(rgb(col::GOLD2)).text_color(rgb(col::GOLD2)))
                        .hover(|s| s.bg(rgb(kit::mix(col::INK2, col::WHITE, 0.3))))
                        .child(o.clone())
                        .on_click(cx.listener(move |n, _, window, cx| {
                            f2.update(cx, |f, cx| f.set(&o2, cx));
                            f2.read(cx).focus(window);
                            n.picked.insert(k2.clone(), (n_, o2.clone()));
                            cx.notify();
                        })),
                );
            }
            box_ = box_.child(opts);
        }
        box_ = box_.child(field);
        let (bb, qq) = (b.clone(), question.clone());
        let label = if busy == Some("send") { "Sending…".to_string() } else { format!("Send to {}", b.name) };
        let mut send = div().flex().gap(px(10.)).items_center().child(button(("send", i), label, true, busy.is_some()).when(busy.is_none(), |d| d.on_click(cx.listener(move |n, _, _, cx| n.send(&bb, qq.clone(), cx)))));
        if let Some(err) = self.errs.get(&k) {
            send = send.child(div().text_size(px(13.)).text_color(rgb(0xff9c8c)).child(err.clone()));
        }
        d.child(box_.child(send)).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(id: &str, busy: bool, waiting: bool, over: &str) -> Bot {
        Bot { id: id.into(), name: id.into(), busy, waiting, over: over.into(), alive: true, unit: "/p".into(), ..Default::default() }
    }

    #[test]
    fn words_for_the_notes() {
        assert_eq!(first_sentence("It builds. Then it tests."), "It builds.");
        assert_eq!(first_sentence("v1.2 is out! yes"), "v1.2 is out!");
        assert_eq!(first_sentence("no stop"), "no stop");
        assert_eq!(desk_name(Some(&Desk { dir: "/Users/rem".into(), ..Default::default() })), "home");
        assert_eq!(desk_name(Some(&Desk { dir: "/Users/rem/p/burrow/".into(), ..Default::default() })), "burrow");
        assert_eq!(desk_name(Some(&Desk { dir: "/x".into(), name: "mine".into(), ..Default::default() })), "mine");
        assert_eq!(desk_name(None), "?");
        assert_eq!(tilde("/Users/rem/p"), "~/p");
        assert_eq!(tilde("/opt/x"), "/opt/x");
        assert_eq!(ago(30.), "30 s");
        assert_eq!(ago(3600.), "60 min");
        assert_eq!(ago(7200.), "2.0 h");
        let mut b = bot("a", false, false, "");
        b.tempo = "blocked".into();
        assert!(needs_you(&b));
        assert_eq!(state_of(&b).0, "needs");
        b.tempo = "idle".into();
        b.alive = false;
        assert_eq!(state_of(&b).1, "process off");
    }

    #[test]
    fn the_notes_in_their_order() {
        let state = State { bots: vec![bot("idle", false, false, ""), bot("done", false, false, "done"), bot("work", true, false, ""), bot("asks", false, true, ""), bot("none", false, false, "")], ..Default::default() };
        let mut ss = Map::new();
        ss.insert("idle".into(), json!({"at": 5, "decisions": [{"question": "later?", "blocking": false}]}));
        ss.insert("done".into(), json!({"at": 9}));
        ss.insert("work".into(), json!({"at": 1, "decisions": [{"question": "old one"}, {"question": "done one"}], "answered": {"done one": {"text": "x", "at": 1}}}));
        ss.insert("asks".into(), json!({"at": 2, "decisions": [{"question": "now?"}]}));
        let m = model(&state, &ss, Some("none"));
        let order: Vec<&str> = m.items.iter().map(|it| it.b.id.as_str()).collect();
        assert_eq!(order, vec!["asks", "work", "idle", "none", "done"], "needs you, working, idle (freshest first), over");
        assert_eq!(m.now.len(), 1, "the waiting one's decision (no flag: it needs you)");
        assert_eq!(m.items[m.now[0].it].b.id, "asks");
        let later: Vec<&str> = m.later.iter().map(|d| s(&d.q, "question")).collect();
        assert_eq!(later, vec!["old one", "later?"], "answered ones are not open");
        assert_eq!(m.pick(None, None), Some((0, Some(0))), "the first decision waited on, lit");
        assert_eq!(m.pick(Some("idle"), None), Some((2, None)));
        assert_eq!(m.pick(Some("gone"), Some(3)), Some((0, Some(0))));
        assert!(model(&State::default(), &Map::new(), None).pick(None, None).is_none());
    }
}
