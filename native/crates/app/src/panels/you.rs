//! YOU (#vYou, `askYou`): your face, picked as at pair's door (← another face →: pair's row of seeds
//! "<start>-<i>", so the pair rooms draw the same one), and your name (empty: the login name, greyed). Both kept
//! in room.json and put on your seat in every linked pair room.

use super::{Slot, call};
use crate::burrow::Burrow;
use crate::kit::{self, Frame, col, frame_bg};
use burrow_room::room::TERRACOTTA;
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};

/// a seed taken apart, as /^(.*)-(-?\d+)$/ does: its start, its number, whether it had one ("rembert-15": rembert, 15)
pub fn parse(seed: &str) -> (String, i64, bool) {
    if let Some((a, n)) = seed.rsplit_once('-')
        && !n.is_empty()
        && n.bytes().all(|c| c.is_ascii_digit())
        && let Ok(i) = n.parse::<i64>()
    {
        return (a.to_string(), i, true);
    }
    (if seed.is_empty() { "you".into() } else { seed.to_string() }, 0, false)
}

/// the seed for face `i` (a first face, the seed of the name, stays as it is)
pub fn seed(start: &str, i: i64, numbered: bool) -> String {
    if !numbered && i == 0 { start.to_string() } else { format!("{start}-{i}") }
}

pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let you = b.state.you.clone();
    let (start, i, numbered) = parse(&you.seed);
    let p = &mut b.views.p;
    p.you_start = start;
    p.you_i = i;
    p.you_numbered = numbered;
    let name = if you.name == you.login { String::new() } else { you.name.clone() };
    p.you_named = name.clone();
    let ph = if you.login.is_empty() { "your name".to_string() } else { you.login.clone() };
    let f = super::reset(b, Slot::YouName, &name, &ph, window, cx);
    f.update(cx, |f, _| f.max = 40);
    f.read(cx).focus(window);
}

fn turn(b: &mut Burrow, by: i64, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.you_i += by;
    let s = seed(&b.views.p.you_start, b.views.p.you_i, b.views.p.you_numbered);
    call(b, move |hub| hub.you(None, Some(&s), None), |_, _, _, _| {}, window, cx);
}

/// the name, when it changed (Enter, or the field left)
pub fn save_name(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    let name = super::text(b, Slot::YouName, cx).trim().to_string();
    if name == b.views.p.you_named {
        return;
    }
    b.views.p.you_named = name.clone();
    call(b, move |hub| hub.you(Some(&name), None, None), |_, _, _, _| {}, window, cx);
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let f = super::field(b, Slot::YouName, window, cx);
    let s = seed(&b.views.p.you_start, b.views.p.you_i, b.views.p.you_numbered);
    let (im, w, h) = kit::pic(&format!("you:{s}"), 4, window, || (*burrow_room::avatar::avatar(&s, false, Some(TERRACOTTA))).clone());
    // the face in an ink well
    let face = div()
        .relative()
        .p(px(4.))
        .child(frame_bg(Frame { border: Some(col::INK), rings: vec![(col::STEEL1, col::INK)], fill: kit::Fill::Solid(col::INK), band: None, pad: 0, pad_fill: None, clip: false, glow: None }))
        .child(div().relative().child(kit::image(im, w, h)));
    div()
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(kit::button("youprev", "←", false, false).on_click(cx.listener(|b, _, window, cx| turn(b, -1, window, cx))))
                .child(face)
                .child(kit::button("younext", "→", false, false).on_click(cx.listener(|b, _, window, cx| turn(b, 1, window, cx)))),
        )
        .child(div().mt(px(12.)).w(px(220.)).child(f))
        .when(!b.views.err.is_empty(), |d| d.child(super::err_line(b)))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_in_pairs_form() {
        assert_eq!(parse("rembert-15"), ("rembert".into(), 15, true));
        assert_eq!(parse("rembert"), ("rembert".into(), 0, false));
        assert_eq!(parse(""), ("you".into(), 0, false));
        assert_eq!(parse("a-b-3"), ("a-b".into(), 3, true));
        assert_eq!(parse("x--2"), ("x-".into(), 2, true), "as the page's regex reads it");
        assert_eq!(seed("rembert", 0, false), "rembert");
        assert_eq!(seed("rembert", -1, false), "rembert--1");

        assert_eq!(seed("rembert", 0, true), "rembert-0");
    }
}
