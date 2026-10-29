//! A MACHINE (#vMachine): a new door, reached over ssh; its host from the Host entries of ~/.ssh/config that have
//! no door yet, completing as you type. It goes into Claude Code's own sshConfigs.

use super::suggest::{self, Key};
use super::{Slot, call, err_line, row};
use crate::burrow::Burrow;
use crate::kit::{self, Run, col};
use gpui::{AnyElement, Context, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, Window, div, prelude::*, px};

pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.hosts = b.hub.hosts();
    let f = super::reset(b, Slot::Host, "", "odyssey", window, cx);
    f.read(cx).focus(window);
    // its completions open at once
    look(b, cx);
}

/// the hosts for what the field says
pub fn look(b: &mut Burrow, cx: &mut Context<Burrow>) {
    let text = super::text(b, Slot::Host, cx);
    let items = suggest::hosts(&b.views.p.hosts, &text);
    b.views.p.list.set(items, &text);
}

fn take(b: &mut Burrow, i: usize, window: &mut Window, cx: &mut Context<Burrow>) {
    if let Some(it) = b.views.p.list.items.get(i).cloned() {
        let f = super::reset(b, Slot::Host, &it.go, "odyssey", window, cx);
        f.read(cx).focus(window);
    }
}

pub fn key(b: &mut Burrow, key: &str, window: &mut Window, cx: &mut Context<Burrow>) -> bool {
    match b.views.p.list.key(key) {
        Key::Take(i) => {
            take(b, i, window, cx);
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

fn add(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let ssh = super::text(b, Slot::Host, cx).trim().to_string();
    if b.views.p.busy {
        return;
    }
    b.views.err.clear();
    b.views.p.busy = true;
    b.views.p.list.clear();
    let at = b.views.p.machine_at.map(|a| [a[0] as i64, a[1] as i64]);
    call(
        b,
        move |hub| hub.add_machine(&ssh, "", at),
        |b, r, window, cx| {
            b.views.p.busy = false;
            match r {
                Ok(mid) => {
                    // its door stays up until the state brings it
                    let at = b.views.p.machine_at.take();
                    let now = b.now_ms();
                    b.room.pend(at, Some(mid), "door", now);
                    b.close_view(window, cx);
                }
                Err(e) => b.views.err = e,
            }
        },
        window,
        cx,
    );
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let f = super::field(b, Slot::Host, window, cx);
    let focused = f.read(cx).focused(window);
    let p = &b.views.p;
    let mut list = kit::steel_list("hosts", true).max_h(px(200.));
    for (i, it) in p.list.items.iter().enumerate() {
        list = list.child(kit::steel_row(("host", i), it.label.clone(), p.list.pick == Some(i), false, None).on_mouse_down(
            MouseButton::Left,
            cx.listener(move |b, e: &MouseDownEvent, window, cx| {
                take(b, i, window, cx);
                if e.click_count >= 2 {
                    add(b, window, cx);
                }
                cx.stop_propagation();
            }),
        ));
    }
    let prose = kit::prose(
        &[
            ("Reached over ssh, as ", Run::Plain),
            ("ssh", Run::Code),
            (" would (your ", Run::Plain),
            ("~/.ssh/config", Run::Code),
            (" applies). It goes into Claude Code's own list, ", Run::Plain),
            ("sshConfigs", Run::Code),
            (" in ", Run::Plain),
            ("~/.claude/settings.json", Run::Code),
            (", which the Desktop app has too. Nothing gets installed there.", Run::Plain),
        ],
        col::WHITE,
    );
    div()
        .flex()
        .flex_col()
        .child(div().mb(px(10.)).child(prose))
        .child(div().mt(px(8.)).child(f))
        .when(focused && !p.list.items.is_empty(), |d| d.child(list))
        .child(row().child(kit::button("addm", "ADD →", false, false).on_click(cx.listener(|b, _, window, cx| add(b, window, cx)))))
        .child(err_line(b))
        .into_any_element()
}
