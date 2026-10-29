//! The view's panels, one module each (web/index.html's #v… panels and their fill… functions). `body` draws the
//! one that is open; `opened`, `closed` and `key` are their lifecycle.

use crate::burrow::Burrow;
use crate::kit::{col, pixel_text};
use crate::views::ViewKind;
use gpui::{AnyElement, Context, IntoElement, KeyDownEvent, ParentElement, Window, div};

pub fn opened(_b: &mut Burrow, _kind: ViewKind, _window: &mut Window, _cx: &mut Context<Burrow>) {}

pub fn closed(_b: &mut Burrow, _kind: ViewKind, _window: &mut Window, _cx: &mut Context<Burrow>) {}

pub fn key(_b: &mut Burrow, _kind: ViewKind, _e: &KeyDownEvent, _window: &mut Window, _cx: &mut Context<Burrow>) {}

/// the printer's report and a linked pair room have no panel round them
pub fn frameless(_b: &Burrow, kind: ViewKind) -> bool {
    kind == ViewKind::Report
}

pub fn body(_b: &mut Burrow, kind: ViewKind, _window: &mut Window, _cx: &mut Context<Burrow>) -> AnyElement {
    div().child(pixel_text(format!("{} (soon)", kind.title()), col::STEEL3, true)).into_any_element()
}
