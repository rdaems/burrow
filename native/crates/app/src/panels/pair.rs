//! PAIR ROOM (#vPair, `openPair`/`linkPair`): a pair room table is one pair room (rdaems/pair-prompting). Not
//! linked yet: open a new room (you are in it with your name and face), or link one that is there by your own
//! page in it (…/h/…) or its invite (…/r/…). Linked: the chat itself. The page showed it in an iframe; GPUI has
//! no web view, so here it opens in the browser and the panel says so.

use super::{Slot, call, err_line, note, row};
use crate::burrow::Burrow;
use crate::kit::{self, Run, col};
use burrow_room::room::TERRACOTTA;
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, px, prelude::*};

fn key(b: &Burrow) -> String {
    b.views.pair_key.clone().unwrap_or_default()
}

fn at(b: &Burrow) -> String {
    if b.state.pair_at.is_empty() { "https://pair.daems.me".into() } else { b.state.pair_at.clone() }
}

pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    match b.state.pair.get(&key(b)).cloned() {
        Some(info) => {
            b.views.sub = info.room.clone();
            if b.views.p.pair_opened.as_deref() != Some(&info.url) {
                cx.open_url(&info.url);
                b.views.p.pair_opened = Some(info.url);
            }
        }
        None => {
            b.views.p.pair_opened = None;
            super::reset(b, Slot::PairUrl, "", &format!("{}/h/…", at(b)), window, cx);
            let f = super::reset(b, Slot::PairRoom, "", "What is it about?", window, cx);
            f.update(cx, |f, _| f.max = 80);
            f.read(cx).focus(window);
        }
    }
}

/// Enter in one of the forms: that form's
pub fn enter(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> bool {
    if super::focused(b, Slot::PairRoom, window, cx) {
        let name = super::text(b, Slot::PairRoom, cx).trim().to_string();
        if !name.is_empty() {
            link(b, Some(name), None, window, cx);
        }
        return true;
    }
    if super::focused(b, Slot::PairUrl, window, cx) {
        let url = super::text(b, Slot::PairUrl, cx).trim().to_string();
        if !url.is_empty() {
            link(b, None, Some(url), window, cx);
        }
        return true;
    }
    false
}

fn link(b: &mut Burrow, new: Option<String>, url: Option<String>, window: &mut Window, cx: &mut Context<Burrow>) {
    let k = key(b);
    b.views.err.clear();
    b.views.p.busy = true;
    call(
        b,
        move |hub| hub.pair(&k, new.as_deref(), url.as_deref()),
        move |b, r, window, cx| {
            b.views.p.busy = false;
            match r {
                Ok(info) => {
                    if !info.warn.is_empty() {
                        note(&info.warn);
                    }
                    // before the state comes round
                    b.state.pair.insert(key(b), burrow_model::PairLink { url: info.url, room: info.room });
                    opened(b, window, cx);
                }
                Err(e) => b.views.err = e,
            }
        },
        window,
        cx,
    );
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    if let Some(info) = b.state.pair.get(&key(b)).cloned() {
        let url = info.url.clone();
        return div()
            .flex()
            .flex_col()
            .child(kit::prose(&[("The chat is open in your browser: this app has no web view. Its link is ", Run::Plain), (&info.url, Run::Code), (".", Run::Plain)], col::WHITE))
            .child(row().child(kit::button("pairopen", "OPEN THE CHAT →", false, false).on_click(move |_, _, cx| cx.open_url(&url))))
            .into_any_element();
    }
    let room_f = super::field(b, Slot::PairRoom, window, cx);
    let url_f = super::field(b, Slot::PairUrl, window, cx);
    let you = b.state.you.clone();
    let seed = if you.seed.is_empty() { you.name.clone() } else { you.seed.clone() };
    let (im, w, h) = kit::pic(&format!("you:{seed}"), 1, window, || (*burrow_room::avatar::avatar(&seed, false, Some(TERRACOTTA))).clone());
    let new_form = div()
        .flex()
        .flex_col()
        .child(kit::prose(&[("This table is one ", Run::Plain), ("pair room", Run::Gold), (": a chat where people and their agents talk. Open a new one; you are in it as", Run::Plain)], col::WHITE))
        .child(div().flex().items_center().gap(px(4.)).mb(px(10.)).child(kit::image(im, w, h)).child(kit::pixel_text(format!("{}.", you.name), col::GOLD2, true)))
        .child(div().mt(px(8.)).child(room_f))
        .child(row().child(kit::button("pairnew", "OPEN ROOM →", false, false).on_click(cx.listener(|b, _, window, cx| {
            let name = super::text(b, Slot::PairRoom, cx).trim().to_string();
            if !name.is_empty() {
                link(b, Some(name), None, window, cx);
            }
        }))));
    let link_form = div()
        .flex()
        .flex_col()
        .mt(px(18.))
        .child(kit::prose(&[("Or a room that is there already: the link of ", Run::Plain), ("your own page", Run::Gold), (" in it (…/h/…), or its invite (…/r/…).", Run::Plain)], col::WHITE))
        .child(div().mt(px(8.)).child(url_f))
        .child(err_line(b));
    div().flex().flex_col().child(new_form).child(link_form).into_any_element()
}
