//! The files' tile in the window (index.html's `filesLink`, `syncFiles`, `openFiles`, `showFile`'s buttons, `pathFor`
//! and the drop on a terminal): FILES on a terminal's band while a filing cabinet stands in the room and is on, opening
//! its files beside it (again: back to it, or closed if it has the focus); THEIRS, MINE, SOURCE / PREVIEW and @ on
//! the files' own band; a file dragged from the tree onto a terminal goes in as @path in a session (Claude Code reads
//! it with the message), its path in a shell. A part of burrow.rs, kept apart (files.rs is the tile itself).

use super::{Burrow, TileKind};
use crate::files::{FileDrag, Files, FilesEvent};
use crate::kit;
use burrow_room::layout::kind_of;
use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Window, prelude::*};
use std::time::{Duration, Instant};

/// FILES is on the bands: a filing cabinet stands in the room and it is on
pub fn trees_on(b: &Burrow) -> bool {
    burrow_room::stage::files_on(&b.state) && b.state.furniture.iter().any(|k| kind_of(k) == "files")
}

/// where a terminal runs: a session's working directory, a shell's folder
pub fn root_of(b: &Burrow, t: u64) -> Option<(String, String)> {
    let (m, dir) = match &b.tile.get(&t)?.kind {
        TileKind::Bot { m, id } => (m.clone(), b.state.bots.iter().find(|x| x.id == *id)?.cwd.clone()),
        TileKind::Shell { m, dir } => (m.clone(), dir.clone()),
        _ => return None,
    };
    let dir = dir.trim_end_matches('/').to_string();
    (!dir.is_empty()).then_some((m, dir))
}

/// a file as it goes into a terminal: @path in a session (relative to where it runs when it is under it), its path
/// in a shell, quoted if it must be
pub fn path_for(b: &Burrow, t: u64, abs: &str) -> String {
    let session = match b.tile.get(&t).map(|x| &x.kind) {
        Some(TileKind::Bot { id, .. }) => Some(b.state.bots.iter().find(|x| x.id == *id).map(|x| x.cwd.clone()).unwrap_or_default()),
        _ => None,
    };
    path_in(session.as_deref(), abs)
}

/// the same, for a session working in `cwd` (None: a shell)
pub fn path_in(cwd: Option<&str>, abs: &str) -> String {
    match cwd {
        Some(cwd) => {
            let root = cwd.trim_end_matches('/');
            let rel = abs.strip_prefix(&format!("{root}/")).filter(|_| !root.is_empty()).unwrap_or(abs);
            if rel.contains(char::is_whitespace) { format!("@\"{rel}\" ") } else { format!("@{rel} ") }
        }
        None if abs.contains(|c: char| c.is_whitespace() || "'\"\\$".contains(c)) => format!("'{}' ", abs.replace('\'', "'\\''")),
        None => format!("{abs} "),
    }
}

#[cfg(test)]
mod tests {
    use super::path_in;

    #[test]
    fn a_file_into_a_terminal() {
        assert_eq!(path_in(Some("/p/kite/"), "/p/kite/src/main.js"), "@src/main.js ");
        assert_eq!(path_in(Some("/p/kite"), "/p/kite/my notes.md"), "@\"my notes.md\" ");
        assert_eq!(path_in(Some("/p/kite"), "/q/x.rs"), "@/q/x.rs ", "not under it: whole");
        assert_eq!(path_in(None, "/p/a.rs"), "/p/a.rs ");
        assert_eq!(path_in(None, "/p/it's here"), "'/p/it'\\''s here' ");
    }
}

/// the files' tile of a terminal (`src`), opened beside it; there already: back to it, or closed if it has the
/// focus (`toggle`)
pub fn open(b: &mut Burrow, src: u64, toggle: bool, window: &mut Window, cx: &mut Context<Burrow>) {
    b.close_view(window, cx);
    if let Some(t) = b.tile.iter().find(|(_, x)| x.files.as_ref().is_some_and(|f| f.read(cx).src == src)).map(|(k, _)| *k) {
        if toggle && b.tiles.focused() == Some(t) {
            b.close_tile(t, window, cx);
        } else {
            b.focus_tile(t, window, cx);
        }
        return;
    }
    let Some((m, root)) = root_of(b, src) else { return };
    let src_bot = match &b.tile[&src].kind {
        TileKind::Bot { id, .. } => Some(id.clone()),
        _ => None,
    };
    let t = b.new_tile(TileKind::Files { m: m.clone(), root: root.clone() });
    let hub = b.hub.clone();
    let files: Entity<Files> = cx.new(|cx| Files::new(hub, &m, &root, src, src_bot, cx));
    // its band follows it; what you type in it is you typing at the screen; clicked into, it is the focused tile
    cx.observe(&files, |_, _, cx| cx.notify()).detach();
    cx.subscribe(&files, move |b, _, ev: &FilesEvent, cx| {
        match ev {
            FilesEvent::Typed => {
                let now = b.now_ms();
                b.room.typed(now, None, false);
            }
            FilesEvent::Focused => b.tiles.focus(t),
        }
        b.dirty = true;
        cx.notify();
    })
    .detach();
    b.tile.get_mut(&t).unwrap().files = Some(files);
    // beside its terminal
    b.tiles.focus(src);
    b.tiles.add(t);
    b.dirty = true;
    cx.notify();
}

/// each state: a session's turn over, its files read again
pub fn sync(b: &mut Burrow, cx: &mut Context<Burrow>) {
    let fs: Vec<Entity<Files>> = b.tile.values().filter_map(|x| x.files.clone()).collect();
    for f in fs {
        let Some(id) = f.read(cx).src_bot.clone() else { continue };
        let busy = b.state.bots.iter().find(|x| x.id == id).is_some_and(|x| x.busy);
        f.update(cx, |f, cx| f.turn_over(busy, cx));
    }
}

/// ✕ on a file with unsaved edits: it says so once, and a second ✕ within three seconds drops them
pub fn refuse_close(f: &Entity<Files>, cx: &mut Context<Burrow>) -> bool {
    let (dirty, armed) = {
        let f = f.read(cx);
        (f.dirty, f.closing.is_some_and(|c| c > Instant::now()))
    };
    if dirty && !armed {
        f.update(cx, |f, cx| {
            f.closing = Some(Instant::now() + Duration::from_secs(3));
            f.say("unsaved: ✕ again drops it", true, cx);
        });
        return true;
    }
    false
}

/// a click on the filing cabinet: FILES off or on
pub fn switch(b: &mut Burrow) {
    let on = !burrow_room::stage::files_on(&b.state);
    b.sound.play(if on { "paper" } else { "click" }, 0.0, 1.0, false, None);
    b.hub.settings(&[("files", on)]);
}

/// a file of a tree let go on a terminal: in it as @path (its path in a shell)
pub fn dropped(b: &mut Burrow, t: u64, d: &FileDrag, window: &mut Window, cx: &mut Context<Burrow>) {
    let Some(tile) = b.tile.get(&t) else { return };
    let (Some(body), m) = (tile.body.clone(), match &tile.kind {
        TileKind::Bot { m, .. } | TileKind::Shell { m, .. } => m.clone(),
        _ => return,
    }) else {
        return;
    };
    if d.m != m {
        return crate::panels::note("that file is on another machine"); // (its path means nothing there)
    }
    let text = path_for(b, t, &d.path);
    body.update(cx, |tt, cx| tt.hand(text, cx));
    b.focus_tile(t, window, cx);
}

/// the buttons on a tile's band: FILES on a terminal's; THEIRS, MINE, SOURCE / PREVIEW and @ on the files'
pub fn band_buttons(b: &Burrow, t: u64, cx: &mut Context<Burrow>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    let Some(tile) = b.tile.get(&t) else { return out };
    if matches!(tile.kind, TileKind::Bot { .. } | TileKind::Shell { .. }) {
        if trees_on(b) && root_of(b, t).is_some() {
            out.push(kit::button(("files", t), "FILES", false, true).on_click(cx.listener(move |b, _, window, cx| open(b, t, true, window, cx))).into_any_element());
        }
        return out;
    }
    let Some(f) = tile.files.clone() else { return out };
    let (theirs, md, preview, path, src) = {
        let r = f.read(cx);
        (r.theirs.is_some(), r.md && r.editor.is_some(), r.preview, r.path.clone(), r.src)
    };
    if theirs {
        let f2 = f.clone();
        out.push(kit::button(("theirs", t), "THEIRS", false, true).on_click(cx.listener(move |_, _, _, cx| f2.update(cx, |f, cx| f.load_file(cx)))).into_any_element());
        let f2 = f.clone();
        out.push(kit::button(("mine", t), "MINE", false, true).on_click(cx.listener(move |_, _, _, cx| f2.update(cx, |f, cx| f.save(true, cx)))).into_any_element());
    }
    if md {
        let f2 = f.clone();
        out.push(kit::button(("md", t), if preview { "SOURCE" } else { "PREVIEW" }, false, true).on_click(cx.listener(move |_, _, _, cx| f2.update(cx, |f, cx| f.toggle_preview(cx)))).into_any_element());
    }
    // @: into its terminal, while that is open
    if let Some(path) = path
        && b.tile.get(&src).is_some_and(|x| x.body.is_some())
    {
        out.push(
            kit::button(("at", t), "@", false, true)
                .on_click(cx.listener(move |b, _, window, cx| {
                    let text = path_for(b, src, &path);
                    if let Some(body) = b.tile.get(&src).and_then(|x| x.body.clone()) {
                        body.update(cx, |tt, cx| tt.hand(text, cx));
                        b.focus_tile(src, window, cx);
                    }
                }))
                .into_any_element(),
        );
    }
    out
}
