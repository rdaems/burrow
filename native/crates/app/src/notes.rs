//! The office manager's notes (Linda), a tile at the screen's resolution: what each session is doing, the decisions
//! still open (explain one, answer it there), what was built.

use burrow_core::Hub;
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div};
use std::sync::Arc;

pub struct Notes {
    pub hub: Arc<Hub>,
}

impl Notes {
    pub fn new(hub: Arc<Hub>, _cx: &mut Context<Notes>) -> Notes {
        Notes { hub }
    }
}

impl Render for Notes {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("notes")
    }
}
