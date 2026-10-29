//! WHITEBOARD (#vBoard, `openBoard`): the whiteboard big, the same ink the bots draw on (burrow-room's dream),
//! four pixels an ink pixel on pair's paper white; draw on it with the pointer, a black marker; wipe.

use crate::burrow::Burrow;
use crate::kit::{self, col};
use burrow_room::dream::{INK_H, INK_W, INKS, ink, ink_line, wipe};
use gpui::{AnyElement, App, Bounds, Context, Corners, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, Point, RenderImage, Styled, Window, canvas, div, px, rgb, prelude::*};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// what the board keeps: the marker's last point, the picture of the ink (and the ink's version it shows), where
/// the board was drawn
#[derive(Default)]
pub struct Ink {
    pub last: Option<(i32, i32)>,
    pic: Option<(u64, u32, Arc<RenderImage>)>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

const SCALE: f32 = 4.;

fn at(b: &Burrow, p: Point<Pixels>) -> Option<(i32, i32)> {
    let r = b.views.p.board.bounds.get()?;
    let (w, h) = ((INK_W * INKS) as f32, (INK_H * INKS) as f32);
    let x = (f32::from(p.x - r.origin.x) * w / f32::from(r.size.width)).floor() as i32;
    let y = (f32::from(p.y - r.origin.y) * h / f32::from(r.size.height)).floor() as i32;
    Some((x, y))
}

fn line(b: &mut Burrow, to: (i32, i32)) {
    let from = b.views.p.board.last.unwrap_or(to);
    ink_line(&mut b.room.w, from.0, from.1, to.0, to.1, pix::Color::rgb(col::INK), 1);
    b.views.p.board.last = Some(to);
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let v = b.room.w.s.ink_v;
    let k = window.scale_factor().round().max(1.0) as u32 * SCALE as u32;
    if b.views.p.board.pic.as_ref().is_none_or(|(pv, pk, _)| *pv != v || *pk != k) {
        let pic = kit::picture(ink(&mut b.room.w), k);
        if let Some((_, _, old)) = b.views.p.board.pic.replace((v, k, pic)) {
            b.views.p.stale.push(old);
        }
    }
    let pic = b.views.p.board.pic.as_ref().map(|x| x.2.clone());
    let bounds = b.views.p.board.bounds.clone();
    let (w, h) = ((INK_W * INKS) as f32 * SCALE, (INK_H * INKS) as f32 * SCALE);
    let board = div()
        .id("board")
        .w(px(w + 4.))
        .h(px(h + 4.))
        .max_w_full()
        .bg(rgb(0xefebe8))
        .border_2()
        .border_color(rgb(col::INK))
        .cursor_crosshair()
        .child(
            canvas(
                move |bd, _, _| bounds.set(Some(bd)),
                move |bd: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
                    if let Some(p) = pic {
                        let _ = window.paint_image(bd, Corners::default(), p, 0, false);
                    }
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|b, e: &MouseDownEvent, _, cx| {
                b.views.p.board.last = None;
                if let Some(p) = at(b, e.position) {
                    line(b, p);
                }
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .on_mouse_move(cx.listener(|b, e: &MouseMoveEvent, _, cx| {
            if e.pressed_button != Some(MouseButton::Left) {
                b.views.p.board.last = None;
                return;
            }
            if b.views.p.board.last.is_some()
                && let Some(p) = at(b, e.position)
            {
                line(b, p);
                cx.notify();
            }
        }))
        .on_mouse_up(MouseButton::Left, cx.listener(|b, _, _, _| b.views.p.board.last = None))
        .on_mouse_up_out(MouseButton::Left, cx.listener(|b, _, _, _| b.views.p.board.last = None));
    div()
        .flex()
        .flex_col()
        .child(board)
        .child(super::row().child(kit::button("wipe", "wipe", false, false).on_click(cx.listener(|b, _, _, cx| {
            wipe(&mut b.room.w);
            cx.notify();
        }))))
        .into_any_element()
}
