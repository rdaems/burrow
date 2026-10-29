//! THE CLOSET (#vCloset, `fillCloset`, `carryOut`): the furniture there is, drawn as it stands in the room (two
//! pixels a dot); drag a piece into the room to put it there, as many as you like. A door is a machine (which one,
//! then it hangs there), a reception comes with an office manager (who); the fireplace takes a piece back out.

use super::{call, note};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use crate::views::ViewKind;
use burrow_room::draw::{Clock, KindEnv, icon, paint_at};
use gpui::{AnyElement, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, Window, div, prelude::*, px, rgb};

pub const CATALOGUE: [&str; 18] = ["plant", "armchair", "window", "board", "meet", "chess", "pong", "coffee", "cooler", "printer", "add", "pair", "manager", "booth", "neuralyzer", "doghouse", "radio", "door"];

/// a piece let go in the room (`at`: where it stands; none: outside the room)
pub fn put(b: &mut Burrow, kind: &str, at: Option<[i32; 2]>, window: &mut Window, cx: &mut Context<Burrow>) {
    let Some(at) = at else { return };
    let now = b.now_ms();
    match kind {
        "door" => {
            b.open_view(ViewKind::Machine, window, cx);
            b.views.p.machine_at = Some(at);
            b.room.pend(Some(at), None, "door", now);
        }
        "manager" => {
            b.views.p.manager_at = Some(at);
            b.room.pend(Some(at), None, "manager", now);
            b.open_view(ViewKind::Manager, window, cx);
        }
        _ => {
            let kind = kind.to_string();
            call(b, move |hub| hub.add_furniture(&kind, [at[0] as f64, at[1] as f64]), |_, r, _, _| if let Err(e) = r { note(&e) }, window, cx);
        }
    }
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let (h, m) = crate::burrow::local_time();
    let n = b.state.machines.len() as i32;
    let env = KindEnv { clock: Clock { h, m, s: 0 }, door: paint_at(n - 1) };
    let carrying = b.closet_carrying().map(String::from);
    let mut d = div().flex().flex_wrap().gap_x(px(18.)).gap_y(px(14.)).items_end();
    for (i, kind) in CATALOGUE.iter().enumerate() {
        // (what changes with the hour or the doors: the window, the clock, a door's paint)
        let (im, w, hh) = kit::pic(&format!("closet:{kind}:{h}:{n}"), 2, window, || icon(kind, 1, 4, &env));
        let lift = carrying.as_deref() == Some(*kind);
        d = d.child(
            div()
                .id(("item", i))
                .group("item")
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .px(px(2.))
                .py(px(4.))
                .cursor_grab()
                .when(lift, |d| d.opacity(0.5))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |b, e: &MouseDownEvent, _, cx| {
                        b.closet_carry(kind, e.position);
                        cx.notify();
                    }),
                )
                .child(kit::image(im, w, hh))
                .child(
                    div()
                        .font_family(kit::PIXEL)
                        .text_size(px(16.))
                        .line_height(px(22.))
                        .whitespace_nowrap()
                        .text_color(rgb(col::STEEL3))
                        .group_hover("item", |s| s.text_color(rgb(col::GOLD2)))
                        .child(burrow_room::layout::name_of(kind).to_string()),
                ),
        );
    }
    d.into_any_element()
}
