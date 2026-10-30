//! CANVAS (#vBoard, `openBoard`): the canvas on the easel big (burrow-room's art), seven pixels a dot, in a gilded
//! frame lit along its top and left, in shadow along the others, and who has painted at it under it; never what it
//! is after.

use crate::burrow::Burrow;
use crate::kit::{self, col};
use gpui::{AnyElement, App, Bounds, Context, Corners, IntoElement, ParentElement, Pixels, RenderImage, Styled, Window, canvas, div, px, rgb};
use std::sync::Arc;

/// the picture of the painting, and the painting's version (and the scale) it shows
#[derive(Default)]
pub struct Painting {
    pic: Option<(u64, u32, Arc<RenderImage>)>,
}

const SCALE: f32 = 7.;

/// "a", "a and b", "a, b and c"
fn and(xs: &[String]) -> String {
    match xs {
        [] => String::new(),
        [x] => x.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

pub fn body(b: &mut Burrow, window: &mut Window, _cx: &mut Context<Burrow>) -> AnyElement {
    let Some(a) = b.room.w.s.art.as_ref() else { return div().into_any_element() };
    let p = &burrow_room::paintings::list()[a.i];
    let (v, k) = (b.room.w.s.art_v, window.scale_factor().round().max(1.0) as u32 * SCALE as u32);
    if b.views.p.board.pic.as_ref().is_none_or(|(pv, pk, _)| *pv != v || *pk != k) {
        let pic = kit::picture(&a.img, k);
        if let Some((_, _, old)) = b.views.p.board.pic.replace((v, k, pic)) {
            b.views.p.stale.push(old);
        }
    }
    // never what it is after: only who painted at it, by the names they had then
    let names: Vec<String> = a.by.iter().map(|(_, n)| n.clone()).collect();
    let by = if names.is_empty() { String::new() } else { format!("By {}.", and(&names)) };
    let pic = b.views.p.board.pic.as_ref().map(|x| x.2.clone());
    let (w, h) = (p.w as f32 * SCALE, p.h as f32 * SCALE);
    let art = div().w(px(w + 4.)).h(px(h + 4.)).border_2().border_color(rgb(0x4a3410)).child(
        canvas(
            |_, _, _| {},
            move |bd: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
                if let Some(p) = pic {
                    let _ = window.paint_image(bd, Corners::default(), p, 0, false);
                }
            },
        )
        .size_full(),
    );
    // the gilt: lit along the top and left, in shadow along the others (the page's inset shadows)
    let bar = |c: u32| div().absolute().bg(rgb(c));
    let frame = div()
        .relative()
        .mx_auto()
        .p(px(10.))
        .bg(rgb(0xc8962e))
        .border_2()
        .border_color(rgb(col::INK))
        .child(bar(0x7a5418).bottom_0().left_0().right_0().h(px(3.)))
        .child(bar(0x7a5418).top_0().bottom_0().right_0().w(px(3.)))
        .child(bar(0xf2d27a).top_0().left_0().right_0().h(px(3.)))
        .child(bar(0xf2d27a).top_0().bottom_0().left_0().w(px(3.)))
        .child(art);
    div()
        .flex()
        .flex_col()
        .items_center()
        .child(frame)
        .child(div().mt(px(10.)).min_h(px(22.)).text_color(rgb(col::STEEL3)).child(by))
        .into_any_element()
}
