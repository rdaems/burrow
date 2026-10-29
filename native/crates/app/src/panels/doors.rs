//! DOORS (#vDoors, `fillDoors`): a line a machine, its door in its paint, its name, how it is reached and how it
//! is, and the paints to pick from (its door and its desks wear it: the wood or one of the room's paints).
//! Another machine's door can be taken out of Claude Code's sshConfigs (the machine and its sessions are left as
//! they are), asked twice: "remove", then "sure? remove" within three seconds.

use super::{call, note};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use burrow_room::draw::{PAINT, WOOD, door_picture};
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use std::time::{Duration, Instant};

/// the paint a machine wears: the one picked, else this machine the wood and the others in their order
pub fn paint_of(s: &burrow_model::State, mid: &str) -> i32 {
    let i = s.machines.iter().position(|m| m.id == mid).unwrap_or(0) as i32;
    s.paint.get(mid).copied().unwrap_or(i - 1)
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let armed = b.views.p.door_armed.as_ref().filter(|(_, t)| t.elapsed() < Duration::from_secs(3)).map(|(m, _)| m.clone());
    if armed.is_some() {
        window.request_animation_frame(); // (to disarm on time)
    }
    let mut d = div().flex().flex_col();
    for (k, m) in b.state.machines.clone().iter().enumerate() {
        let cur = paint_of(&b.state, &m.id);
        let n = b.state.bots.iter().filter(|x| x.m == m.id).count();
        let how = format!(
            "{} · {}",
            if m.id.is_empty() { "this computer".to_string() } else { format!("over ssh, {}", if m.ssh.is_empty() { &m.id } else { &m.ssh }) },
            m.error.clone().unwrap_or_else(|| format!("{n} session{}", if n == 1 { "" } else { "s" }))
        );
        let (im, w, h) = kit::pic(&format!("door:{cur}"), 3, window, || door_picture(cur, 1));
        let mut sw = div().flex().flex_wrap().gap(px(6.)).mt(px(8.));
        for i in -1..PAINT.len() as i32 {
            let c = if i < 0 { WOOD.top } else { PAINT[i as usize].top };
            let mid = m.id.clone();
            sw = sw.child(kit::swatch(("sw", k * 100 + (i + 1) as usize), kit::hex(c), i == cur).on_click(cx.listener(move |b, _, _, cx| {
                b.hub.paint(&mid, Some(i as i64));
                cx.notify();
            })));
        }
        let rm = (!m.id.is_empty()).then(|| {
            let sure = armed.as_deref() == Some(m.id.as_str());
            let mid = m.id.clone();
            let label = if sure { "sure? remove" } else { "remove" };
            let btn = if sure {
                // armed: a second click takes the door out
                div()
                    .id(("rm", k))
                    .relative()
                    .flex_none()
                    .cursor_pointer()
                    .child(kit::frame_bg(kit::Frame::button(kit::Fill::Solid(0xc9473a), col::STEEL1, col::WHITE)))
                    .child(div().relative().px(px(14.)).py(px(7.)).child(kit::pixel_text(label, col::WHITE, false)))
            } else {
                kit::button(("rm", k), label, false, false)
            };
            btn.ml_auto().on_click(cx.listener(move |b, _, window, cx| {
                if sure {
                    b.views.p.door_armed = None;
                    let mid = mid.clone();
                    call(b, move |hub| hub.unmachine(&mid), |_, r, _, _| if let Err(e) = r { note(&e) }, window, cx);
                } else {
                    b.views.p.door_armed = Some((mid.clone(), Instant::now()));
                }
                cx.notify();
            }))
        });
        d = d.child(
            div()
                .flex()
                .gap(px(14.))
                .items_center()
                .py(px(8.))
                .child(kit::image(im, w, h))
                .child(div().flex().flex_col().flex_1().min_w(px(0.)).child(kit::pixel_text(m.name.clone(), col::WHITE, true)).child(kit::pixel_para(how, col::STEEL3)).child(sw))
                .children(rm),
        );
    }
    d.into_any_element()
}
