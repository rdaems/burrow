//! The view: what is not a session shows here, in place of the tiles, one at a time: a 16-bit panel in the
//! middle of the pane (pair's door panels): the closet, a game, a project or machine to add, your face, the
//! settings, the clock, the doors, a desk, the limits, the printer's report, the whiteboard, outside, a pair
//! room, an office manager to hire. Each panel is its own module in `panels/`.

use crate::burrow::Burrow;
use crate::kit::{self, Frame, col, frame_bg, pixel_text};
use gpui::{AnyElement, Context, IntoElement, KeyDownEvent, ParentElement, Styled, Window, div, prelude::*, px};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViewKind {
    Dir,
    Machine,
    You,
    Closet,
    Report,
    Limits,
    Settings,
    Doors,
    Desk,
    Manager,
    Clock,
    Outside,
    Board,
    Pair,
    Chess,
    Pong,
}

impl ViewKind {
    /// the room knows the view by these names
    pub fn name(self) -> &'static str {
        match self {
            ViewKind::Dir => "dir",
            ViewKind::Machine => "machine",
            ViewKind::You => "you",
            ViewKind::Closet => "closet",
            ViewKind::Report => "report",
            ViewKind::Limits => "limits",
            ViewKind::Settings => "settings",
            ViewKind::Doors => "doors",
            ViewKind::Desk => "desk",
            ViewKind::Manager => "manager",
            ViewKind::Clock => "clock",
            ViewKind::Outside => "outside",
            ViewKind::Board => "board",
            ViewKind::Pair => "pair",
            ViewKind::Chess => "chess",
            ViewKind::Pong => "pong",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            ViewKind::Dir => "NEW DESK",
            ViewKind::Machine => "A MACHINE",
            ViewKind::You => "YOU",
            ViewKind::Closet => "THE CLOSET",
            ViewKind::Chess => "CHESS",
            ViewKind::Pong => "PING-PONG",
            ViewKind::Report => "REPORT",
            ViewKind::Pair => "PAIR ROOM",
            ViewKind::Board => "WHITEBOARD",
            ViewKind::Limits => "LIMITS",
            ViewKind::Outside => "OUTSIDE",
            ViewKind::Settings => "SETTINGS",
            ViewKind::Clock => "CLOCK",
            ViewKind::Doors => "DOORS",
            ViewKind::Manager => "OFFICE MANAGER",
            ViewKind::Desk => "DESK",
        }
    }
    /// how wide its panel is at most (CSS px)
    pub fn width(self) -> f32 {
        match self {
            ViewKind::Outside => 1068.0,
            ViewKind::You => 280.0,
            _ => 640.0,
        }
    }
}

/// what the views keep between frames
#[derive(Default)]
pub struct Views {
    pub desk_unit: Option<String>,
    pub pair_key: Option<String>,
    pub outside_key: Option<String>,
    /// a bot carried out of the room over the pane: where it would open
    pub carry_target: Option<crate::tiles::Target>,
    /// a game's sounds still playing, by the game's id for them
    pub sounds: HashMap<u32, burrow_sound::Handle>,
    /// a line under a form: what went wrong
    pub err: String,
    /// the subtitle on the band
    pub sub: String,
}

/// a view was opened: what it needs set up
pub fn opened(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) {
    b.views.err.clear();
    b.views.sub.clear();
    crate::panels::opened(b, kind, window, cx);
}

/// a view was closed: your chess game resigns, a game stops
pub fn closed(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) {
    crate::panels::closed(b, kind, window, cx);
}

/// keys while the view is open: Esc closes it, the games take theirs
pub fn key(b: &mut Burrow, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Burrow>) {
    if e.keystroke.key == "escape" {
        // the view goes; with none, the focused tile
        if b.view.is_some() {
            b.close_view(window, cx);
        } else if let Some(t) = b.tiles.focused() {
            b.close_tile(t, window, cx);
        }
        return;
    }
    if let Some(kind) = b.view {
        crate::panels::key(b, kind, e, window, cx);
    }
}

/// the panel: an ink ring, two bevels, the blue; its title, ✕; its body
pub fn render(b: &mut Burrow, kind: ViewKind, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let body = crate::panels::body(b, kind, window, cx);
    if crate::panels::frameless(b, kind) {
        return div().absolute().inset_0().child(body).into_any_element();
    }
    let head = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(10.))
        .mb(px(10.))
        .child(pixel_text(kind.title(), col::GOLD2, true))
        .child(pixel_text(b.views.sub.clone(), col::STEEL3, true).flex_1().min_w(px(0.)).overflow_hidden())
        .child(kit::button("vx", "✕", false, true).on_click(cx.listener(|this, _, window, cx| this.close_view(window, cx))));
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(24.))
        .child(
            div()
                .id("view")
                .relative()
                .w_full()
                .max_w(px(kind.width()))
                .max_h_full()
                .when(kind == ViewKind::You, |d| d.w_auto())
                .child(frame_bg(Frame::panel()))
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .pt(px(16.))
                        .px(px(20.))
                        .pb(px(18.))
                        .font_family(kit::PIXEL)
                        .text_size(px(16.))
                        .line_height(px(22.))
                        .child(head)
                        .child(div().id("vbody").flex_1().min_h(px(0.)).overflow_y_scroll().child(body)),
                )
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation()),
        )
        .into_any_element()
}
