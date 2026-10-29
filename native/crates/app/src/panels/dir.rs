//! NEW DESK (#vDir, `ask("dir")` and `browse`): which machine (a steel button each, when there is a choice), the
//! folder's path, the folders in it under the field as a file dialog lists them, ADD DESK.

use super::suggest::{self, Item, Key};
use super::{Slot, call, err_line, row};
use crate::burrow::Burrow;
use crate::kit;
use gpui::{AnyElement, Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, Window, div, prelude::*, px};

pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let first = b.state.machines.first().map(|m| m.id.clone()).unwrap_or_default();
    pick(b, first, window, cx);
}

/// a machine picked: its home and its recent places asked, the field at its home
fn pick(b: &mut Burrow, m: String, window: &mut Window, cx: &mut Context<Burrow>) {
    let p = &mut b.views.p;
    p.dir_m = m.clone();
    p.home.clear();
    p.recent.clear();
    p.list.clear();
    let f = super::reset(b, Slot::Dir, "", "", window, cx);
    f.read(cx).focus(window);
    let mid = m.clone();
    call(
        b,
        move |hub| hub.recent(&mid),
        move |b, r, window, cx| {
            if b.views.p.dir_m != m || b.view != Some(crate::views::ViewKind::Dir) {
                return;
            }
            let (home, dirs) = r.unwrap_or_default();
            b.views.p.recent = dirs.iter().map(|d| suggest::short(&home, d)).collect();
            let start = suggest::start(&home);
            b.views.p.home = home;
            super::reset(b, Slot::Dir, start, "", window, cx);
        },
        window,
        cx,
    );
}

/// the list again for what the field says: the folder's folders, asked of the machine once each
pub fn refresh(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let v = super::text(b, Slot::Dir, cx);
    let (dir, _) = suggest::split(&v);
    let p = &b.views.p;
    let key = (p.dir_m.clone(), dir.to_string());
    if dir.is_empty() {
        let items = suggest::browse(&v, &p.home, &p.recent, &[]);
        b.views.p.list.browse(items);
        return;
    }
    if let Some(kids) = p.kids.get(&key) {
        let items = suggest::browse(&v, &p.home, &p.recent, kids);
        b.views.p.list.browse(items);
        return;
    }
    if p.asking.contains(&key) {
        return;
    }
    b.views.p.asking.push(key.clone());
    let (m, d) = key.clone();
    call(
        b,
        move |hub| hub.complete(&m, &d),
        move |b, r, window, cx| {
            b.views.p.asking.retain(|k| *k != key);
            let ls = r.unwrap_or_default();
            let kids = suggest::kids(&b.views.p.home, &ls);
            b.views.p.kids.insert(key, kids);
            if b.view == Some(crate::views::ViewKind::Dir) {
                refresh(b, window, cx);
            }
        },
        window,
        cx,
    );
}

/// into a folder (a click, Enter or Tab on the one picked)
fn go(b: &mut Burrow, it: &Item, window: &mut Window, cx: &mut Context<Burrow>) {
    let f = super::reset(b, Slot::Dir, &it.go, "", window, cx);
    f.read(cx).focus(window);
}

pub fn key(b: &mut Burrow, key: &str, window: &mut Window, cx: &mut Context<Burrow>) -> bool {
    match b.views.p.list.browse_key(key) {
        Key::Take(i) => {
            if let Some(it) = b.views.p.list.items.get(i).cloned() {
                go(b, &it, window, cx);
            }
            true
        }
        Key::Moved | Key::Closed => true,
        Key::Pass if key == "enter" => {
            add(b, window, cx);
            true
        }
        Key::Pass => false,
    }
}

/// ADD DESK: the folder in the field, kept as a desk there
fn add(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let dir = suggest::trimmed(&super::text(b, Slot::Dir, cx));
    if dir.is_empty() || b.views.p.busy {
        return;
    }
    let m = b.views.p.dir_m.clone();
    b.views.err.clear();
    b.views.p.busy = true;
    call(
        b,
        move |hub| hub.dir(&m, &dir),
        |b, r, window, cx| {
            b.views.p.busy = false;
            match r {
                Ok(_) => b.close_view(window, cx),
                Err(e) => b.views.err = e,
            }
        },
        window,
        cx,
    );
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let f = super::field(b, Slot::Dir, window, cx);
    let p = &b.views.p;
    let mut machines = row().mt(px(0.)).mb(px(4.));
    if b.state.machines.len() >= 2 {
        for (i, m) in b.state.machines.iter().enumerate() {
            let id = m.id.clone();
            machines = machines.child(kit::button(("dm", i), m.name.clone(), p.dir_m == m.id, false).on_click(cx.listener(move |b, _, window, cx| pick(b, id.clone(), window, cx))));
        }
    }
    // the folders: always there under the path, as a file dialog's list; the recent ones first, set apart
    let mut list = kit::steel_list("dirs", false).mt(px(6.)).h(px(300.));
    let mut was_recent = false;
    for (i, it) in p.list.items.iter().enumerate() {
        let it2 = it.clone();
        let rowel = kit::steel_row(("dir", i), it.label.clone(), p.list.pick == Some(i), it.recent, None)
            .when(was_recent && !it.recent, |d| d.border_t_2().border_color(gpui::rgb(kit::col::STEEL3)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |b, e: &MouseDownEvent, window, cx| {
                    // a click goes into the folder; a second one adds the folder it went into
                    if e.click_count >= 2 { add(b, window, cx) } else { go(b, &it2, window, cx) }
                    cx.stop_propagation();
                }),
            );
        was_recent = it.recent;
        list = list.child(rowel);
    }
    div()
        .flex()
        .flex_col()
        .when(b.state.machines.len() >= 2, |d| d.child(machines))
        .child(div().mt(px(8.)).child(f))
        .child(list)
        .child(row().child(kit::button("adddesk", "ADD DESK →", false, false).on_click(cx.listener(|b, _, window, cx| add(b, window, cx)))))
        .child(err_line(b))
        .into_any_element()
}
