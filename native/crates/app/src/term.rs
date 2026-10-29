//! A terminal tile's body: a session (`claude attach`) or your shell at a screen, from burrow-core's ptys, shown
//! by burrow-term's view.

use burrow_core::Hub;
use gpui::{AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div};
use std::sync::Arc;

pub struct TermTile {
    pub hub: Arc<Hub>,
    pub m: String,
    pub what: String,
    pub shell: bool,
}

impl TermTile {
    pub fn bot(hub: Arc<Hub>, m: &str, id: &str, _window: &mut Window, cx: &mut Context<crate::burrow::Burrow>) -> gpui::Entity<TermTile> {
        cx.new(|_| TermTile { hub, m: m.into(), what: id.into(), shell: false })
    }
    pub fn shell(hub: Arc<Hub>, m: &str, dir: &str, _window: &mut Window, cx: &mut Context<crate::burrow::Burrow>) -> gpui::Entity<TermTile> {
        cx.new(|_| TermTile { hub, m: m.into(), what: dir.into(), shell: true })
    }
    /// focus the terminal
    pub fn focus(&mut self, _window: &mut Window, _cx: &mut Context<TermTile>) {}
    /// text handed to the program once it is up (a dropped file's path, a pair link)
    pub fn hand(&mut self, _text: String, _cx: &mut Context<TermTile>) {}
}

impl Render for TermTile {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.what.clone())
    }
}
