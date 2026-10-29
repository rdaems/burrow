//! DESK (#vDesk, `openDesk`): a desk's name (empty: its folder's), where it is, the sessions at it (a click opens
//! one), RENAME, TERMINAL (your shell there), KEEP (a desk that came with its sessions stays without them).

use super::{Slot, call, err_line, row};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use gpui::{AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, px};

fn desk(b: &Burrow) -> Option<burrow_model::Desk> {
    let u = b.views.desk_unit.as_deref()?;
    b.state.desks.iter().find(|d| d.unit == u).cloned()
}

pub fn folder(dir: &str) -> String {
    let t = dir.trim_end_matches('/');
    t.rsplit('/').next().filter(|x| !x.is_empty()).unwrap_or(dir).to_string()
}

pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let Some(d) = desk(b) else {
        b.close_view(window, cx);
        return;
    };
    let f = super::reset(b, Slot::DeskName, &d.name, &folder(&d.dir), window, cx);
    f.update(cx, |f, cx| {
        f.max = 40;
        f.select_all(cx);
    });
    f.read(cx).focus(window);
}

pub fn rename(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let (Some(u), name) = (b.views.desk_unit.clone(), super::text(b, Slot::DeskName, cx).trim().to_string()) else { return };
    call(
        b,
        move |hub| hub.desk(&u, Some(&name), false),
        |b, r, window, cx| match r {
            Ok(()) => b.close_view(window, cx),
            Err(e) => b.views.err = e,
        },
        window,
        cx,
    );
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let f = super::field(b, Slot::DeskName, window, cx);
    let Some(d) = desk(b) else { return div().into_any_element() };
    let on = b.state.machine(&d.m).filter(|m| !m.id.is_empty()).map(|m| format!("on {}", m.name)).unwrap_or_default();
    let mut sessions = div().flex().flex_col().mb(px(8.));
    for (i, x) in b.state.bots.iter().filter(|x| x.unit == d.unit).enumerate() {
        let what = if x.waiting {
            "waits for you".to_string()
        } else if x.busy {
            "working".into()
        } else if !x.over.is_empty() {
            x.over.clone()
        } else {
            "idle".into()
        };
        let (m, id) = (x.m.clone(), x.id.clone());
        sessions = sessions.child(
            div()
                .id(("ses", i))
                .flex()
                .gap(px(10.))
                .py(px(3.))
                .cursor_pointer()
                .on_click(cx.listener(move |b, _, window, cx| b.open_bot(&m, &id, false, None, window, cx)))
                .child(kit::pixel_text(x.name.clone(), col::GOLD2, true))
                .child(kit::pixel_text(what, col::STEEL3, true)),
        );
    }
    let (m, dir) = (d.m.clone(), d.dir.clone());
    let u = d.unit.clone();
    let mut buttons = row()
        .child(kit::button("rename", "RENAME", false, false).on_click(cx.listener(|b, _, window, cx| rename(b, window, cx))))
        .child(kit::button("shell", "TERMINAL", false, false).on_click(cx.listener(move |b, _, window, cx| b.open_shell(&m, &dir, false, window, cx))));
    if !d.kept {
        buttons = buttons.child(kit::button("keep", "KEEP", false, false).on_click(cx.listener(move |b, _, window, cx| {
            let u = u.clone();
            call(b, move |hub| hub.desk(&u, None, true), |b, r, _, _| if let Err(e) = r { b.views.err = e }, window, cx);
        })));
    }
    div()
        .flex()
        .flex_col()
        .child(div().mt(px(8.)).child(f))
        .child(div().flex().flex_wrap().gap(px(8.)).items_baseline().mt(px(10.)).mb(px(6.)).child(kit::mono_text(d.dir.clone(), col::GOLD2, 14.)).child(kit::pixel_text(on, col::WHITE, true)))
        .child(sessions)
        .child(buttons)
        .child(err_line(b))
        .into_any_element()
}
