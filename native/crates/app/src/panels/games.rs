//! PING-PONG (#pong, web/pong.js) and OUTSIDE (#vOutside, web/outside.js): burrow-games draws them into pix
//! images, painted here at whole device pixels, with their lines in the view's lettering: pong's card (who
//! plays, the score, the choices round a game), the duel's line over the outside and its keys under it.
//! Yours against an idle bot drawn by lot, or the two bots at that table (or that window) to watch.

use crate::burrow::Burrow;
use crate::kit::{self, col};
use crate::views::ViewKind;
use burrow_games::{Player, Wears, outside, pong};
use burrow_room::room::TERRACOTTA;
use gpui::{AnyElement, App, Bounds, Context, Corners, InteractiveElement, IntoElement, Keystroke, ParentElement, Pixels, RenderImage, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, px, rgb};
use pix::Image;
use std::sync::Arc;
use std::time::Instant;

/// a game's picture on the GPU: what it was made from (to make it again only when it changed), when
#[derive(Default)]
pub struct Pics {
    last: Vec<u8>,
    pic: Option<Arc<RenderImage>>,
    at: Option<Instant>,
}

/// the picture of a game's frame, made again only when the frame changed (thirty times a second at most)
fn frame_pic(b: &mut Burrow, img: &Image, k: u32) -> Option<Arc<RenderImage>> {
    let g = &mut b.views.p.game;
    let fresh = g.at.is_none_or(|t| t.elapsed().as_millis() >= 33);
    if g.pic.is_none() || (fresh && g.last != bytes(img, k)) {
        g.last = bytes(img, k);
        g.at = Some(Instant::now());
        if let Some(old) = g.pic.replace(kit::picture(img, k)) {
            b.views.p.stale.push(old);
        }
    }
    b.views.p.game.pic.clone()
}

fn bytes(img: &Image, k: u32) -> Vec<u8> {
    let mut v: Vec<u8> = img.px.iter().flatten().copied().collect();
    v.push(k as u8);
    v
}

fn screen(pic: Option<Arc<RenderImage>>, w: f32, h: f32) -> gpui::Div {
    div().flex_none().w(px(w)).h(px(h)).child(
        canvas(
            |_, _, _| {},
            move |bd: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
                if let Some(p) = pic {
                    let _ = window.paint_image(bd, Corners::default(), p, 0, false);
                }
            },
        )
        .size_full(),
    )
}

fn bot(b: &Burrow, id: &str) -> Player {
    let x = b.state.bots.iter().find(|x| x.id == id);
    Player::bot(id, x.map(|x| x.name.as_str()).unwrap_or(id), super::color_of(b, id))
}

fn you(b: &Burrow) -> Player {
    let y = &b.state.you;
    let name = if y.name.is_empty() { "you" } else { &y.name };
    let seed = if y.seed.is_empty() { name } else { &y.seed };
    let c = y.color.as_deref().and_then(burrow_room::avatar::color_of).unwrap_or(TERRACOTTA);
    let w = burrow_room::figure::wears(seed, c);
    Player::you(name, c, Some(Wears { shirt: w.shirt, shade: w.shade, skin: w.skin, hair: w.hair }))
}

fn small(b: &Burrow, p: &Player, window: &Window) -> gpui::Div {
    let c = p.color;
    let (im, _, _) = kit::pic(&format!("bot:{}:{}", p.id, c.hex()), 1, window, || (*burrow_room::avatar::avatar(&p.id, !p.human, Some(c))).clone());
    let _ = b;
    kit::image(im, 20., 20.)
}

// ---- ping-pong

pub fn pong_opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.game = Pics::default();
    b.views.p.yours = false;
    let at = b.room.playing("pong");
    if at.len() == 2 {
        let (x, y) = (bot(b, &at[0]), bot(b, &at[1]));
        b.views.sub = format!("{} vs {}", x.name, y.name);
        b.pong.watch(x, y);
        return;
    }
    let Some(it) = super::by_lot(b) else {
        super::note("no idle bot to play with");
        b.close_view(window, cx);
        return;
    };
    let p = bot(b, &it.id);
    b.views.sub = format!("with {}", p.name);
    b.views.p.yours = true;
    b.pong.start(Some(p));
}

pub fn pong_body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let k = window.scale_factor().round().max(1.0) as u32 * 3;
    let mut img = Image::new(pong::W, pong::H);
    b.pong.draw(&mut img);
    let card = b.pong.card();
    let pic = frame_pic(b, &img, k);
    let w = b.views.p.width.get().max(300.).min(680.);
    let h = w * pong::H as f32 / pong::W as f32;
    let side = |s: usize, b: &Burrow| {
        let mut d = div().flex_1().flex().items_center().gap(px(7.)).min_w(px(0.)).overflow_hidden().whitespace_nowrap().when(s == 1, |d| d.justify_end());
        if let Some(p) = &card.players[s] {
            let name = div().font_weight(kit::bold()).text_color(rgb(kit::hex(p.color))).child(if p.human { "You".to_string() } else { p.name.clone() });
            let serve = card.serving.filter(|(x, _)| *x == s).map(|(_, w)| div().text_size(px(12.)).text_color(rgb(col::GOLD2)).child(w));
            d = d.child(small(b, p, window)).child(name).children(serve);
        }
        d
    };
    let head = div()
        .flex()
        .items_center()
        .gap(px(10.))
        .min_h(px(26.))
        .mb(px(8.))
        .child(side(0, b))
        .child(div().font_family(kit::PIXEL).text_size(px(16.)).text_color(rgb(col::GOLD2)).whitespace_nowrap().child(card.score.clone()))
        .child(side(1, b));
    let mut msg = div().flex().flex_col();
    if let Some(over) = &card.over {
        msg = msg.child(div().my(px(8.)).text_color(rgb(col::DIM)).child(over.clone())).child(
            div()
                .flex()
                .gap(px(8.))
                .mt(px(10.))
                .child(kit::button("again", "play again", false, false).on_click(cx.listener(|b, _, _, cx| {
                    b.pong.again();
                    cx.notify();
                })))
                .child(kit::button("choose", "someone else", false, false).on_click(cx.listener(|b, _, _, cx| {
                    b.pong.choose();
                    cx.notify();
                }))),
        );
    }
    let playing = !card.score.is_empty();
    if !playing {
        // choosing: against one of the bots with nothing to do
        let idle = super::idle_bots(b);
        let mut who = div().flex().flex_wrap().gap(px(8.)).my(px(8.));
        for (i, x) in idle.iter().enumerate() {
            let p = bot(b, &x.id);
            let c = kit::hex(p.color);
            let pp = p.clone();
            who = who.child(
                div()
                    .id(("pwho", i))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .pl(px(3.))
                    .pr(px(8.))
                    .py(px(3.))
                    .border_1()
                    .rounded(px(6.))
                    .border_color(rgb(col::LINE))
                    .hover(|s| s.border_color(rgb(c)))
                    .cursor_pointer()
                    .text_color(rgb(c))
                    .child(small(b, &p, window))
                    .child(p.name.clone())
                    .on_click(cx.listener(move |b, _, _, cx| {
                        b.views.sub = format!("with {}", pp.name);
                        b.views.p.yours = true;
                        b.pong.start(Some(pp.clone()));
                        cx.notify();
                    })),
            );
        }
        msg = msg.child(div().text_color(rgb(col::DIM)).child("Against one of the bots with nothing to do:"));
        msg = if idle.is_empty() { msg.child(div().my(px(8.)).text_color(rgb(col::DIM)).child("Every bot here is at work just now.")) } else { msg.child(who) };
    }
    div()
        .flex()
        .flex_col()
        .font_family(kit::MONO)
        .text_size(px(14.))
        .line_height(px(21.))
        .text_color(rgb(col::TEXT))
        .child(head)
        .when(playing, |d| d.child(screen(pic, w, h)))
        .child(msg)
        .when(card.hint, |d| d.child(div().mt(px(8.)).text_size(px(12.)).text_color(rgb(col::FAINT)).flex().justify_center().child("move with ↑ ↓ · to 11, two clear")))
        .into_any_element()
}

// ---- outside

pub fn outside_opened(b: &mut Burrow) {
    b.views.p.game = Pics::default();
    b.views.p.yours = false;
    let key = b.views.outside_key.clone();
    let at = b.room.playing("window");
    if at.len() == 2 {
        let (x, y) = (bot(b, &at[0]), bot(b, &at[1]));
        b.views.sub = format!("{} vs {}", x.name, y.name);
        b.outside.open(x, Some(y), key);
        return;
    }
    let it = super::by_lot(b).map(|x| bot(b, &x.id));
    if let Some(p) = &it {
        b.views.sub = format!("with {}", p.name);
        b.views.p.yours = true;
    }
    let me = you(b);
    b.outside.open(me, it, key);
}

pub fn outside_body(b: &mut Burrow, window: &mut Window, _cx: &mut Context<Burrow>) -> AnyElement {
    // as big as the pane lets it be, four pixels a dot at most, whole pixels
    let room_h = f32::from(window.viewport_size().height) - 210.;
    let avail = b.views.p.width.get().max(256.);
    let s = ((avail / outside::W as f32).floor().min(room_h * 16. / 9. / outside::W as f32).floor()).clamp(1., 4.);
    let k = window.scale_factor().round().max(1.0) as u32 * s as u32;
    let (h, m) = crate::burrow::local_time();
    let mut img = Image::new(outside::W as u32, outside::H as u32);
    b.outside.draw(&mut img, h as f64 + m as f64 / 60.);
    let pic = frame_pic(b, &img, k);
    let hud = b.outside.hud().map(|hud| {
        let side = |i: usize| {
            let p = &hud.sides[i];
            let name = kit::pixel_text(p.name.clone(), if p.on { col::GOLD2 } else { col::STEEL3 }, true);
            let say = p.say.clone().map(|x| div().mx(px(10.)).child(kit::pixel_text(x, col::WHITE, true)));
            let d = div().flex().flex_1().min_w(px(0.)).overflow_hidden();
            if i == 1 { d.justify_end().children(say).child(name) } else { d.child(name).children(say) }
        };
        div().flex().gap(px(12.)).mb(px(8.)).child(side(0)).child(kit::pixel_text(format!("{} – {}", hud.score[0], hud.score[1]), col::WHITE, true)).child(side(1))
    });
    let (w, hh) = (outside::W as f32 * s, outside::H as f32 * s);
    let hint = b.views.p.yours.then(|| {
        kit::prose(
            &[("↑ ↓", kit::Run::Code), (" angle · ", kit::Run::Plain), ("← →", kit::Run::Code), (" power · ", kit::Run::Plain), ("space", kit::Run::Code), (" fires", kit::Run::Plain)],
            col::STEEL3,
        )
    });
    div()
        .flex()
        .flex_col()
        .items_center()
        .children(hud.map(|h| h.w_full()))
        .child(div().border_2().border_color(rgb(col::INK)).child(screen(pic, w, hh)))
        .children(hint.map(|h| div().mt(px(10.)).child(h)))
        .into_any_element()
}

/// a key down or up while a game is shown (presses with ⌘ ⌃ ⌥ are not the game's)
pub fn key(b: &mut Burrow, kind: ViewKind, k: &Keystroke, down: bool, held: bool, cx: &mut Context<Burrow>) {
    if k.modifiers.platform || k.modifiers.control || k.modifiers.alt {
        return;
    }
    match kind {
        ViewKind::Pong => {
            let key = match k.key.as_str() {
                "up" => pong::Key::Up,
                "down" => pong::Key::Down,
                _ => return,
            };
            b.pong.key(key, down);
        }
        ViewKind::Outside => {
            let key = match k.key.as_str() {
                "up" => outside::Key::Up,
                "down" => outside::Key::Down,
                "left" => outside::Key::Less,
                "right" => outside::Key::More,
                "space" | "enter" => outside::Key::Fire,
                _ => return,
            };
            if down { b.outside.key_down(key, held) } else { b.outside.key_up(key) }
        }
        _ => return,
    }
    cx.stop_propagation();
    cx.notify();
}
