//! CHESS (#chess, web/chessboard.js): pair's board in the view. The rules are the hub's (chess.rs): the board
//! only offers the moves it was sent. Yours against an idle bot drawn by lot, or the bots' own game to watch.
//! A click picks a piece and another puts it; more than one move there is a promotion, which asks to what.

use super::{call, note};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use burrow_core::ChessAction;
use burrow_model::Chess;
use pix::{Color, Image};
use gpui::{AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, rgb, rgba};

pub const START: &str = "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr";

/// what the board keeps: the square picked, a promotion asked (its from-to), what went wrong, the ply it last saw
#[derive(Default)]
pub struct Ui {
    pub sel: Option<usize>,
    pub promo: Option<String>,
    pub err: String,
    pub seen: i64,
}

/// a square's name: 0 is a1
pub fn sq(s: usize) -> String {
    format!("{}{}", (b'a' + (s % 8) as u8) as char, s / 8 + 1)
}

/// the square drawn at row r, column c from the top left (flipped when you play black)
pub fn square_at(r: usize, c: usize, flip: bool) -> usize {
    if flip { r * 8 + (7 - c) } else { (7 - r) * 8 + c }
}

/// where the piece on `sel` may go
pub fn targets(legal: &[String], sel: Option<usize>) -> Vec<String> {
    let Some(s) = sel else { return vec![] };
    let from = sq(s);
    legal.iter().filter(|m| m.starts_with(&from)).map(|m| m[2..4].to_string()).collect()
}

/// what a click on a square does
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Click {
    Pick(Option<usize>),
    Move(String),
    Promote(String),
}

pub fn click(legal: &[String], sel: Option<usize>, s: usize) -> Click {
    if let Some(f) = sel {
        let ft = format!("{}{}", sq(f), sq(s));
        let hits: Vec<&String> = legal.iter().filter(|m| m.starts_with(&ft)).collect();
        if hits.len() > 1 {
            return Click::Promote(ft);
        }
        if hits.len() == 1 {
            return Click::Move(hits[0].clone());
        }
    }
    let from = sq(s);
    Click::Pick((sel != Some(s) && legal.iter().any(|m| m.starts_with(&from))).then_some(s))
}

/// what the board says under the players
pub fn status(v: Option<&Chess>, me: &str) -> String {
    let Some(v) = v else { return "The chess table is free.".into() };
    let (w, bk) = (&v.white.name, &v.black.name);
    if let Some(r) = &v.result {
        let (win, lose) = if r == "1-0" { (w, bk) } else { (bk, w) };
        let wins = |n: &str| if n == "you" { "you win".to_string() } else { format!("{n} wins") };
        return match v.why.as_deref().unwrap_or("") {
            "checkmate" => format!("Checkmate: {}.", wins(win)),
            "resignation" => format!("{lose} resigned: {}.", wins(win)),
            "left" => format!("{lose} left the table: {}.", wins(win)),
            "stalemate" => "Stalemate: a draw.".into(),
            "repetition" => "Draw by repetition.".into(),
            "fifty moves" => "Draw: fifty moves without a capture or a pawn move.".into(),
            "material" => "Draw: not enough pieces left to mate.".into(),
            "agreement" => "Drawn by agreement.".into(),
            "unfinished" => "Left unfinished.".into(),
            _ => "Over.".into(),
        };
    }
    let mover = if v.to_move == v.white.id { &v.white } else { &v.black };
    let mut s = if v.to_move == me { "Your move".to_string() } else { format!("{} to move", mover.name) };
    s += if v.check { ", check!" } else { "." };
    if v.free {
        s += " Nobody has moved for an hour, so the table is free again.";
    }
    s
}

// ---- the pieces: 16 x 16, '#' is the piece; holes inside it come out as ink, like the eye of the knight
const ART: [(char, &[&str]); 6] = [
    ('p', &["", "", "", "", "......####", ".....######", ".....######", "......####", ".....######", "......####", "......####", ".....######", "....########", "...##########", "...##########"]),
    ('r', &["", "", "...##..##..##", "...##########", "...##########", "....########", ".....######", ".....######", ".....######", ".....######", ".....######", "....########", "...##########", "..############", "..############"]),
    ('n', &["", "", ".......#.#", "......#####", ".....#######", "....###.#####", "...##########", "..####..######", "..###...######", "........######", ".......######", "......#######", ".....########", "....#########", "...##########"]),
    ('b', &["", ".......##", "......####", ".....###.##", ".....##.###", ".....######", "......####", ".......##", "......####", "......####", ".....######", "......####", "....########", "...##########", "...##########"]),
    ('q', &[".......##", "..#....##....#", "..##..####..##", "...##.####.##", "...##########", "....########", ".....######", "......####", "......####", ".....######", "......####", "......####", ".....######", "....########", "...##########", "...##########"]),
    ('k', &[".......##", ".....######", ".......##", "...####..####", "..############", "..############", "...##########", "....########", ".....######", "......####", ".....######", "......####", "......####", "....########", "...##########", "...##########"]),
];

/// a piece's picture, 18 x 18: lit on its top-left edges, shaded on the others, outlined
pub fn sprite(ch: char) -> Image {
    use burrow_room::avatar::c as C;
    let white = ch.is_ascii_uppercase();
    let rows = ART.iter().find(|(k, _)| *k == ch.to_ascii_lowercase()).map(|x| x.1).unwrap_or(&[]);
    let (base, lit, dark) = if white { (C::STEEL4, C::WHITE, C::STEEL3) } else { (C::INK2, C::STEEL0, C::INK) };
    let on = |x: i32, y: i32| y >= 0 && x >= 0 && rows.get(y as usize).and_then(|r| r.as_bytes().get(x as usize)) == Some(&b'#');
    let mut im = Image::new(18, 18);
    for y in -1..=16 {
        for x in -1..=16 {
            let c: Color = if on(x, y) {
                if !on(x - 1, y) || !on(x, y - 1) {
                    lit
                } else if !on(x + 1, y) || !on(x, y + 1) {
                    dark
                } else {
                    base
                }
            } else if on(x - 1, y) || on(x + 1, y) || on(x, y - 1) || on(x, y + 1) {
                if white { C::INK } else { C::STEEL1 }
            } else {
                continue;
            };
            im.set(x + 1, y + 1, c.0);
        }
    }
    im
}

// ---- the view

fn you_play(v: &Chess) -> bool {
    v.white.id == "you" || v.black.id == "you"
}

/// a game in the view: the bots' game to watch while there is one, else yours against an idle bot by lot
pub fn opened(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.chess = Ui { seen: -1, ..Default::default() };
    if let Some(v) = b.state.chess.as_ref().filter(|v| v.result.is_none() && !you_play(v)) {
        b.views.sub = format!("{} vs {}", v.white.name, v.black.name);
        return;
    }
    let Some(it) = super::by_lot(b) else {
        note("no idle bot to play with");
        b.close_view(window, cx);
        return;
    };
    challenge(b, it.id, window, cx);
}

fn challenge(b: &mut Burrow, id: String, window: &mut Window, cx: &mut Context<Burrow>) {
    call(
        b,
        move |hub| hub.chess(ChessAction::Challenge(id)),
        |b, r, window, cx| {
            if let Err(e) = r {
                note(&e);
                if b.state.chess.is_none() {
                    b.close_view(window, cx);
                } else {
                    b.views.p.chess.err = e;
                }
            }
        },
        window,
        cx,
    );
}

/// closing the view gives your game up; the bots' goes on
pub fn closed(b: &mut Burrow) {
    if let Some(v) = &b.state.chess
        && v.result.is_none()
        && you_play(v)
    {
        let _ = b.hub.chess(ChessAction::Resign);
    }
}

fn send(b: &mut Burrow, a: ChessAction, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.p.chess.err.clear();
    call(b, move |hub| hub.chess(a), |b, r, _, _| if let Err(e) = r { b.views.p.chess.err = e }, window, cx);
}

fn face(b: &Burrow, id: &str, size: f32, window: &Window) -> Option<gpui::Div> {
    if id == "you" {
        return None;
    }
    let c = super::color_of(b, id);
    let (im, _, _) = kit::pic(&format!("bot:{id}:{}", c.hex()), 1, window, || (*burrow_room::avatar::avatar(id, true, Some(c))).clone());
    Some(kit::image(im, size, size))
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let v = b.state.chess.clone();
    // a new ply, a new game: what was picked goes
    let ply = v.as_ref().map(|v| v.id * 1000 + v.sans.len() as i64 + if v.result.is_some() { 500 } else { 0 }).unwrap_or(-1);
    if ply != b.views.p.chess.seen {
        b.views.p.chess = Ui { seen: ply, ..Default::default() };
    }
    let me = "you";
    let board = v.as_ref().map(|v| v.board.clone()).unwrap_or(START.into());
    let flip = v.as_ref().is_some_and(|v| v.black.id == me);
    let legal = v.as_ref().map(|v| v.legal.clone()).unwrap_or_default();
    let turn = !legal.is_empty();
    let ui = &b.views.p.chess;
    let tg = targets(&legal, ui.sel);
    let last: Vec<String> = v.as_ref().and_then(|v| v.last.clone()).filter(|l| l.len() >= 4).map(|l| vec![l[0..2].to_string(), l[2..4].to_string()]).unwrap_or_default();
    let check_sq = v.as_ref().filter(|v| v.check && v.result.is_none()).and_then(|v| board.find(if v.to_move == v.white.id { 'K' } else { 'k' }));
    let idle = v.as_ref().is_none_or(|v| v.result.is_some());
    let cell = 46.0;
    let mut grid = div().flex().flex_col().border_2().border_color(rgb(col::INK)).flex_none().when(idle, |d| d.opacity(0.85));
    for r in 0..8 {
        let mut line = div().flex();
        for c in 0..8 {
            let s = square_at(r, c, flip);
            let p = board.as_bytes().get(s).copied().unwrap_or(b'.') as char;
            let n = sq(s);
            let light = (r + c) % 2 == 0;
            let mut d = div()
                .id(("sq", s))
                .relative()
                .w(px(cell))
                .h(px(cell))
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(if light { 0xd9c49a } else { 0xa08670 }))
                .when(turn, |d| d.cursor_pointer());
            if Some(s) == check_sq {
                d = d.bg(rgb(0xc8352a));
            }
            if last.contains(&n) {
                d = d.child(div().absolute().inset_0().bg(rgba(0xf2a53161)));
            }
            if ui.sel == Some(s) {
                d = d.child(div().absolute().inset_0().border_3().border_color(rgb(col::ME)));
            }
            if tg.contains(&n) {
                d = if p != '.' { d.child(div().absolute().inset_0().border_3().border_color(rgba(0x140f1e8c))) } else { d.child(div().absolute().w(px(12.)).h(px(12.)).bg(rgba(0x140f1e73))) };
            }
            if p != '.' {
                let (im, _, _) = kit::pic(&format!("piece:{p}"), 2, window, || sprite(p));
                d = d.child(div().relative().child(kit::image(im, 36., 36.)));
            }
            if c == 0 {
                d = d.child(div().absolute().left(px(2.)).top(px(1.)).text_size(px(9.)).line_height(px(10.)).text_color(rgba(0x140f1e8c)).child(n[1..].to_string()));
            }
            if r == 7 {
                d = d.child(div().absolute().right(px(2.)).bottom(px(1.)).text_size(px(9.)).line_height(px(10.)).text_color(rgba(0x140f1e8c)).child(n[..1].to_string()));
            }
            let legal2 = legal.clone();
            d = d.on_click(cx.listener(move |b, _, window, cx| {
                if legal2.is_empty() {
                    return;
                }
                match click(&legal2, b.views.p.chess.sel, s) {
                    Click::Pick(x) => {
                        b.views.p.chess.sel = x;
                        b.views.p.chess.promo = None;
                    }
                    Click::Promote(ft) => b.views.p.chess.promo = Some(ft),
                    Click::Move(m) => {
                        b.views.p.chess.sel = None;
                        b.views.p.chess.promo = None;
                        send(b, ChessAction::Move(m), window, cx);
                    }
                }
                cx.notify();
            }));
            line = line.child(d);
        }
        grid = grid.child(line);
    }
    // the side: who moves, a promotion, what went wrong, the moves, your buttons
    let mut side = div().flex().flex_col().flex_1().min_w(px(200.)).child(div().mb(px(8.)).text_color(rgb(if turn { col::GOLD2 } else { col::TEXT })).when(turn, |d| d.font_weight(kit::bold())).child(status(v.as_ref(), me)));
    if let Some(ft) = ui.promo.clone() {
        let mut pr = div().flex().items_center().gap(px(6.)).mb(px(8.)).text_color(rgb(col::DIM)).child("promote to");
        for (i, k) in "qrbn".chars().enumerate() {
            let ch = if v.as_ref().is_some_and(|v| v.white.id == me) { k.to_ascii_uppercase() } else { k };
            let (im, _, _) = kit::pic(&format!("piece:{ch}"), 2, window, || sprite(ch));
            let m = format!("{ft}{k}");
            pr = pr.child(kit::button_with(("promo", i), kit::image(im, 32., 32.), false, true).on_click(cx.listener(move |b, _, window, cx| {
                b.views.p.chess.promo = None;
                b.views.p.chess.sel = None;
                send(b, ChessAction::Move(m.clone()), window, cx);
            })));
        }
        side = side.child(pr);
    }
    if !ui.err.is_empty() {
        side = side.child(div().mb(px(8.)).text_color(rgb(col::TISSUE2)).child(ui.err.clone()));
    }
    if let Some(v) = v.as_ref().filter(|v| !v.sans.is_empty()) {
        let mut moves = div().id("moves").flex().flex_col().flex_wrap().max_h(px(190.)).overflow_y_scroll().mb(px(10.)).px(px(8.)).py(px(6.)).bg(rgb(col::CODE)).border_1().border_color(rgb(col::LINE)).rounded(px(6.)).text_size(px(13.));
        for (i, pair) in v.sans.chunks(2).enumerate() {
            moves = moves.child(div().flex().gap(px(4.)).child(div().w(px(32.)).text_color(rgb(col::FAINT)).child(format!("{}.", i + 1))).child(format!("{} {}", pair[0], pair.get(1).cloned().unwrap_or_default())));
        }
        side = side.child(moves);
    }
    if let Some(v) = v.as_ref().filter(|v| v.result.is_none() && you_play(v)) {
        let _ = v;
        side = side.child(
            div()
                .flex()
                .gap(px(8.))
                .child(kit::button("draw", "offer draw", false, false).on_click(cx.listener(|b, _, window, cx| send(b, ChessAction::Draw, window, cx))))
                .child(kit::button("resign", "resign", false, false).on_click(cx.listener(|b, _, window, cx| send(b, ChessAction::Resign, window, cx)))),
        );
    }
    // who plays
    let who = v.as_ref().map(|v| {
        let pl = |p: &burrow_model::Player, colour: &str, b: &Burrow| {
            let on = v.to_move == p.id && v.result.is_none();
            let c = if p.id == "you" { col::ME } else { kit::hex(super::color_of(b, &p.id)) };
            div()
                .flex()
                .items_center()
                .gap(px(7.))
                .pl(px(2.))
                .pr(px(8.))
                .py(px(2.))
                .border_1()
                .rounded(px(6.))
                .border_color(if on { rgb(c) } else { gpui::transparent_black().into() })
                .children(face(b, &p.id, 28., window))
                .child(div().font_weight(kit::bold()).text_color(rgb(c)).child(p.name.clone()))
                .child(div().text_size(px(12.)).text_color(rgb(col::DIM)).child(colour.to_string()))
        };
        div().flex().flex_wrap().items_center().gap(px(10.)).mb(px(10.)).child(pl(&v.white, "white", b)).child(div().text_color(rgb(col::FAINT)).child("vs")).child(pl(&v.black, "black", b))
    });
    // after a game, or while the table is free: someone to play with
    let again = (v.is_none() || v.as_ref().is_some_and(|v| v.free || v.result.is_some())).then(|| {
        let mut d = div().flex().flex_wrap().items_center().gap(px(10.)).mt(px(12.)).pt(px(10.)).border_t_1().border_color(rgb(col::LINE)).child(div().text_color(rgb(col::DIM)).child(if v.is_some() { "Play again, or with someone else:" } else { "Sit down with someone (colours by lot):" }));
        let idle = super::idle_bots(b);
        if idle.is_empty() {
            d = d.child(div().text_color(rgb(col::DIM)).child("nobody else is here yet"));
        }
        for (i, x) in idle.iter().enumerate() {
            let id = x.id.clone();
            let c = kit::hex(super::color_of(b, &x.id));
            d = d.child(
                div()
                    .id(("play", i))
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
                    .children(face(b, &x.id, 20., window))
                    .child(x.name.clone())
                    .on_click(cx.listener(move |b, _, window, cx| challenge(b, id.clone(), window, cx))),
            );
        }
        d
    });
    // the chess card keeps pair's room look: the terminal's font
    div()
        .flex()
        .flex_col()
        .font_family(kit::MONO)
        .text_size(px(14.))
        .line_height(px(21.))
        .text_color(rgb(col::TEXT))
        .children(who)
        .child(div().flex().flex_wrap().gap(px(16.)).items_start().child(grid).child(side))
        .children(again)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_and_their_names() {
        assert_eq!(sq(0), "a1");
        assert_eq!(sq(63), "h8");
        assert_eq!(square_at(0, 0, false), 56, "a8 top left for white");
        assert_eq!(square_at(7, 7, false), 7);
        assert_eq!(square_at(0, 0, true), 7, "h1 top left for black");
        assert_eq!(&START[12..13], "P");
    }

    #[test]
    fn a_click_picks_and_one_more_moves() {
        let legal: Vec<String> = ["e2e4", "e2e3", "g1f3"].iter().map(|s| s.to_string()).collect();
        let e2 = 12;
        assert_eq!(click(&legal, None, e2), Click::Pick(Some(e2)));
        assert_eq!(targets(&legal, Some(e2)), vec!["e4", "e3"]);
        assert_eq!(click(&legal, Some(e2), 28), Click::Move("e2e4".into()));
        assert_eq!(click(&legal, Some(e2), e2), Click::Pick(None), "the same square again: put down");
        assert_eq!(click(&legal, Some(e2), 6), Click::Pick(Some(6)), "another piece of yours");
        assert_eq!(click(&legal, None, 40), Click::Pick(None), "nothing to move there");
    }

    #[test]
    fn a_promotion_asks() {
        let legal: Vec<String> = ["a7a8q", "a7a8r", "a7a8b", "a7a8n"].iter().map(|s| s.to_string()).collect();
        assert_eq!(click(&legal, Some(48), 56), Click::Promote("a7a8".into()));
    }

    #[test]
    fn it_says_how_it_stands() {
        let mut v = Chess { white: burrow_model::Player { id: "you".into(), name: "you".into(), kind: "human".into() }, black: burrow_model::Player { id: "b".into(), name: "bo".into(), kind: "agent".into() }, to_move: "you".into(), ..Default::default() };
        assert_eq!(status(Some(&v), "you"), "Your move.");
        v.to_move = "b".into();
        v.check = true;
        assert_eq!(status(Some(&v), "you"), "bo to move, check!");
        v.result = Some("1-0".into());
        v.why = Some("resignation".into());
        assert_eq!(status(Some(&v), "you"), "bo resigned: you win.");
        assert_eq!(status(None, "you"), "The chess table is free.");
    }

    #[test]
    fn a_knight_has_an_eye() {
        let n = sprite('N');
        assert_eq!(n.get(8, 6), burrow_room::avatar::c::INK.0, "the hole is ink");
        assert_eq!(n.get(0, 0)[3], 0);
    }
}
