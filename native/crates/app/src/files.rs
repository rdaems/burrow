//! The files of a session or a shell (index.html's `openFiles`, `makeTree`, `openFile`, `loadFile`, `saveFile`,
//! `watchFile`): one tile beside the terminal, the tree on its left and the file you pick on its right. FILES on the
//! terminal's band opens it (burrow.rs), while a filing cabinet stands in the room and is on.
//!   The tree is rooted where the terminal runs (a session's working directory, a shell's folder): folders open and
//! close with a click, what git says changed in colour (gold changed, green new, a dot on a folder that holds
//! changes), what changed in the last two minutes bold with a dot: where the bot is at work. A file dragged from it
//! onto a terminal goes in as @path (its path in a shell). Read again every 15 s, and when the bot's turn ends (the
//! app says). A README at its top opens first.
//!   The file: editor.rs, ⌘S saves; a markdown file opens rendered (markdown.rs, its pictures from the machine),
//! SOURCE on the band to edit it. What a session changes on disk comes in by itself while you have nothing unsaved
//! (looked at every 4 s); with unsaved edits the band says so in red, and THEIRS (load it) or MINE (save over it)
//! settles it. A save goes through only if the file has not changed since it was read (burrow-core's files.rs).

use crate::editor::{Editor, EditorEvent};
use crate::kit::{self, col};
use crate::markdown::{self, Block, Pic};
use burrow_core::Hub;
use burrow_core::files::{Entry, FileOp};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, Styled, Task,
    WeakEntity, Window, div, prelude::*, px, rgb,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// a file of the tree, dragged: onto a terminal it goes in as @path (its path in a shell)
#[derive(Clone, Debug)]
pub struct FileDrag {
    pub m: String,
    pub path: String,
    pub name: String,
}

/// what is under the pointer while a file is dragged
pub struct DragLabel(String);

impl Render for DragLabel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().px(px(8.)).py(px(2.)).bg(rgb(col::INK2)).border_1().border_color(rgb(col::GOLD1)).font_family(kit::MONO).text_size(px(12.)).text_color(rgb(col::STEEL4)).child(self.0.clone())
    }
}

/// what the tile tells the app
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesEvent {
    /// you typed in the file (the room shows you typing at the screen)
    Typed,
    /// the file took the keyboard: the tile is the focused one
    Focused,
}

pub fn is_md(p: &str) -> bool {
    let p = p.to_lowercase();
    p.ends_with(".md") || p.ends_with(".markdown")
}

/// a link in a/b.md to ../c.md: c.md (None: out of its root)
pub fn resolve_rel(from: &str, href: &str) -> Option<String> {
    let mut out: Vec<String> = from.split('/').map(String::from).collect();
    out.pop();
    for part in percent_decode(href).split('/') {
        match part {
            ".." => {
                out.pop()?;
            }
            "" | "." => {}
            p => out.push(p.to_string()),
        }
    }
    Some(out.join("/"))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// a row of the tree, as it is shown
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// relative to the root
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub dir: bool,
    pub open: bool,
    /// git's mark (M, A, D), a folder holding changes, changed in the last two minutes, a dotfile, the file open
    pub git: Option<String>,
    pub has: bool,
    pub fresh: bool,
    pub dot: bool,
    pub cur: bool,
    /// a folder open that is not read yet: "…"
    pub waiting: bool,
}

/// the tree's rows: the open folders walked, folders first as the machine sorted them
pub fn rows(dirs: &HashMap<String, Vec<Entry>>, open: &BTreeSet<String>, git: &BTreeMap<String, String>, now: i64, cur: Option<&str>) -> Vec<Row> {
    let mut out = Vec::new();
    fn walk(rel: &str, d: usize, dirs: &HashMap<String, Vec<Entry>>, open: &BTreeSet<String>, git: &BTreeMap<String, String>, now: i64, cur: Option<&str>, out: &mut Vec<Row>) {
        let Some(es) = dirs.get(rel) else {
            out.push(Row { path: String::new(), name: "…".into(), depth: d, dir: false, open: false, git: None, has: false, fresh: false, dot: false, cur: false, waiting: true });
            return;
        };
        for e in es {
            let p = if rel.is_empty() { e.name.clone() } else { format!("{rel}/{}", e.name) };
            let is_open = e.dir && open.contains(&p);
            out.push(Row {
                path: p.clone(),
                name: e.name.clone(),
                depth: d,
                dir: e.dir,
                open: is_open,
                git: (!e.dir).then(|| git.get(&p).cloned()).flatten(),
                has: e.dir && git.keys().any(|c| c.starts_with(&format!("{p}/"))),
                fresh: !e.dir && now - e.mtime < 120,
                dot: e.name.starts_with('.'),
                cur: cur == Some(p.as_str()),
                waiting: false,
            });
            if is_open {
                walk(&p, d + 1, dirs, open, git, now, cur, out);
            }
        }
    }
    walk("", 0, dirs, open, git, now, cur, &mut out);
    out
}

pub struct Files {
    hub: Arc<Hub>,
    pub m: String,
    pub root: String,
    /// the terminal it is beside (its tile), and the session there if it is one
    pub src: u64,
    pub src_bot: Option<String>,
    /// the session was at work at the last state (its turn's end reads the tree again)
    pub busy: bool,
    // the tree
    open: BTreeSet<String>,
    dirs: HashMap<String, Vec<Entry>>,
    git: BTreeMap<String, String>,
    now: i64,
    /// a README looked for at the first reading
    first: bool,
    // the file
    pub rel: Option<String>,
    pub path: Option<String>,
    mtime: Option<String>,
    /// the text as it was read (or saved): unsaved is the editor's differing from it
    text: String,
    pub dirty: bool,
    pub md: bool,
    pub preview: bool,
    /// the version on disk that is not yours ("gone": deleted)
    pub theirs: Option<String>,
    pub note: String,
    pub warn: bool,
    /// ✕ pressed once on unsaved edits: a second within this drops them
    pub closing: Option<Instant>,
    saving: bool,
    pub editor: Option<Entity<Editor>>,
    /// instead of the file: "…", too big, not text
    msg: Option<String>,
    /// the markdown parsed, for the text it was parsed from
    blocks: Option<(String, Vec<Block>)>,
    pics: HashMap<String, Pic>,
    me: WeakEntity<Files>,
    _ticks: Vec<Task<()>>,
}

impl EventEmitter<FilesEvent> for Files {}

impl Files {
    pub fn new(hub: Arc<Hub>, m: &str, root: &str, src: u64, src_bot: Option<String>, cx: &mut Context<Files>) -> Files {
        // read again every 15 s, the file looked at every 4 s
        let tree = cx.spawn(async move |this: WeakEntity<Files>, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(15)).await;
                if this.update(cx, |f, cx| f.load_tree(None, cx)).is_err() {
                    break;
                }
            }
        });
        let watch = cx.spawn(async move |this: WeakEntity<Files>, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(4)).await;
                if this.update(cx, |f, cx| f.watch(cx)).is_err() {
                    break;
                }
            }
        });
        let mut f = Files {
            hub,
            m: m.to_string(),
            root: root.to_string(),
            src,
            src_bot,
            busy: false,
            open: BTreeSet::from([String::new()]),
            dirs: HashMap::new(),
            git: BTreeMap::new(),
            now: 0,
            first: true,
            rel: None,
            path: None,
            mtime: None,
            text: String::new(),
            dirty: false,
            md: false,
            preview: false,
            theirs: None,
            note: String::new(),
            warn: false,
            closing: None,
            saving: false,
            editor: None,
            msg: None,
            blocks: None,
            pics: HashMap::new(),
            me: cx.entity().downgrade(),
            _ticks: vec![tree, watch],
        };
        f.load_tree(None, cx);
        f
    }

    /// what the band says: the file's name (or the folder's), and under it a note, unsaved, or the machine
    pub fn name(&self) -> String {
        let p = self.rel.as_deref().unwrap_or(&self.root);
        p.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(&self.root).to_string()
    }

    pub fn say(&mut self, note: &str, warn: bool, cx: &mut Context<Files>) {
        self.note = note.to_string();
        self.warn = warn;
        cx.notify();
    }
    /// a note that goes again after a moment, if nothing else was said meanwhile
    fn say_briefly(&mut self, note: &'static str, cx: &mut Context<Files>) {
        self.say(note, false, cx);
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            cx.background_executor().timer(Duration::from_millis(1500)).await;
            let _ = this.update(cx, |f, cx| {
                if f.note == note {
                    f.say("", false, cx)
                }
            });
        })
        .detach();
    }

    /// the folders open (or one just opened), and git
    pub fn load_tree(&mut self, only: Option<String>, cx: &mut Context<Files>) {
        let dirs: Vec<String> = match &only {
            Some(d) => vec![d.clone()],
            None => self.open.iter().cloned().collect(),
        };
        let (hub, m, root) = (self.hub.clone(), self.m.clone(), self.root.clone());
        let task = cx.background_executor().spawn(async move { hub.tree(&m, &root, &dirs) });
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            let r = task.await;
            let _ = this.update(cx, |f, cx| {
                match r {
                    Ok(t) => {
                        for (d, es) in t.dirs {
                            match es {
                                Some(es) => {
                                    f.dirs.insert(d, es);
                                }
                                None if !d.is_empty() => {
                                    f.dirs.remove(&d);
                                    f.open.remove(&d);
                                }
                                None => {}
                            }
                        }
                        f.git = t.git;
                        f.now = t.now;
                    }
                    Err(e) => eprintln!("burrow: the tree: {e}"),
                }
                // a README at its top opens first, rendered
                if std::mem::take(&mut f.first)
                    && f.rel.is_none()
                    && let Some(n) = f.dirs.get("").and_then(|es| es.iter().find(|e| !e.dir && matches!(e.name.to_lowercase().as_str(), "readme.md" | "readme.markdown"))).map(|e| e.name.clone())
                {
                    f.open_file(n, None, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle(&mut self, p: String, cx: &mut Context<Files>) {
        if !self.open.remove(&p) {
            self.open.insert(p.clone());
            if !self.dirs.contains_key(&p) {
                self.load_tree(Some(p), cx);
            }
        }
        cx.notify();
    }

    /// the file you picked in the tree (or a link in a markdown file): opened beside it
    pub fn open_file(&mut self, rel: String, window: Option<&mut Window>, cx: &mut Context<Files>) {
        let path = format!("{}/{rel}", self.root);
        if self.path.as_deref() == Some(path.as_str()) {
            if let (Some(e), Some(w)) = (&self.editor, window) {
                e.read(cx).focus(w);
            }
            return;
        }
        if self.dirty {
            return self.say("unsaved: ⌘S first", true, cx);
        }
        self.editor = None;
        let md = is_md(&rel);
        self.rel = Some(rel);
        self.path = Some(path);
        self.mtime = None;
        self.dirty = false;
        self.md = md;
        self.preview = md;
        self.note.clear();
        self.warn = false;
        self.theirs = None;
        self.closing = None;
        self.msg = Some("…".into());
        self.load_file(cx);
    }

    /// the file as it is on disk (THEIRS too: your edits go)
    pub fn load_file(&mut self, cx: &mut Context<Files>) {
        let Some(path) = self.path.clone() else { return };
        let (hub, m, p) = (self.hub.clone(), self.m.clone(), path.clone());
        let task = cx.background_executor().spawn(async move { hub.file(&m, &p, FileOp::Read) });
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            let r = task.await;
            let _ = this.update(cx, |f, cx| {
                if f.path.as_deref() != Some(path.as_str()) {
                    return;
                }
                let r = match r {
                    Ok(r) => r,
                    Err(e) => return f.say(&e, true, cx),
                };
                if r.missing {
                    return f.say("not there any more", true, cx);
                }
                if r.big || r.binary {
                    f.editor = None;
                    f.msg = Some(format!("{} ({} kB)", if r.big { "too big to open here" } else { "not a text file" }, (r.size as f64 / 1024.0).round()));
                    return f.say("", false, cx);
                }
                let text = r.text.unwrap_or_default();
                f.mtime = r.mtime;
                f.dirty = false;
                f.theirs = None;
                f.note.clear();
                f.warn = false;
                f.msg = None;
                f.closing = None;
                f.mount(&text, cx);
                f.text = text;
                cx.notify();
            });
        })
        .detach();
    }

    /// the editor, on the text read
    fn mount(&mut self, text: &str, cx: &mut Context<Files>) {
        let path = self.rel.clone().unwrap_or_default();
        let ed = cx.new(|cx| Editor::new(text, &path, cx));
        cx.subscribe(&ed, |f: &mut Files, ed, ev: &EditorEvent, cx| match ev {
            EditorEvent::Changed => {
                let d = ed.read(cx).text() != f.text;
                if d != f.dirty {
                    f.dirty = d;
                    if !f.warn {
                        f.note.clear();
                    }
                }
                cx.notify();
            }
            EditorEvent::Typed => cx.emit(FilesEvent::Typed),
            EditorEvent::Save => f.save(false, cx),
            EditorEvent::Focused => cx.emit(FilesEvent::Focused),
        })
        .detach();
        self.editor = Some(ed);
    }

    /// ⌘S, or MINE (`force`: over what is on disk)
    pub fn save(&mut self, force: bool, cx: &mut Context<Files>) {
        let (Some(ed), Some(path)) = (self.editor.clone(), self.path.clone()) else { return };
        if self.saving {
            return;
        }
        self.saving = true;
        let text = ed.read(cx).text().to_string();
        let (hub, m, t2, mtime) = (self.hub.clone(), self.m.clone(), text.clone(), self.mtime.clone());
        let task = cx.background_executor().spawn(async move { hub.file(&m, &path, FileOp::Write { text: t2, mtime, force }) });
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            let r = task.await;
            let _ = this.update(cx, |f, cx| {
                f.saving = false;
                let r = match r {
                    Ok(r) => r,
                    Err(e) => return f.say(&e, true, cx),
                };
                if r.conflict {
                    f.theirs = Some(r.mtime.clone().unwrap_or_else(|| "gone".into()));
                    return f.say(if r.missing { "deleted on disk" } else { "changed on disk" }, true, cx);
                }
                f.mtime = r.mtime;
                f.theirs = None;
                f.closing = None;
                f.dirty = f.editor.as_ref().is_some_and(|e| e.read(cx).text() != text);
                f.text = text;
                f.say_briefly("saved", cx);
                f.load_tree(None, cx);
            });
        })
        .detach();
    }

    /// what changed on disk: taken in while you have nothing unsaved (where you were stays), else the band asks
    pub fn watch(&mut self, cx: &mut Context<Files>) {
        if self.editor.is_none() || self.saving || self.theirs.is_some() {
            return;
        }
        let Some(path) = self.path.clone() else { return };
        let (hub, m, p) = (self.hub.clone(), self.m.clone(), path.clone());
        let task = cx.background_executor().spawn(async move { hub.file(&m, &p, FileOp::Stat) });
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            let Ok(r) = task.await else { return };
            let again = this
                .update(cx, |f, cx| {
                    if f.editor.is_none() || f.saving || f.path.as_deref() != Some(path.as_str()) {
                        return false;
                    }
                    if r.missing {
                        if f.mtime.as_deref() != Some("gone") && f.theirs.is_none() {
                            f.theirs = Some("gone".into());
                            f.say("deleted on disk", true, cx);
                        }
                        return false;
                    }
                    if r.mtime == f.mtime {
                        return false;
                    }
                    if f.dirty {
                        f.theirs = r.mtime.clone();
                        f.say("changed on disk", true, cx);
                        return false;
                    }
                    true
                })
                .unwrap_or(false);
            if !again {
                return;
            }
            let hub = this.update(cx, |f, _| (f.hub.clone(), f.m.clone())).ok();
            let Some((hub, m)) = hub else { return };
            let p = path.clone();
            let Ok(g) = cx.background_executor().spawn(async move { hub.file(&m, &p, FileOp::Read) }).await else { return };
            let _ = this.update(cx, |f, cx| {
                let Some(text) = g.text else { return };
                if f.dirty || f.path.as_deref() != Some(path.as_str()) {
                    return;
                }
                let Some(ed) = f.editor.clone() else { return };
                f.text = text.clone();
                f.mtime = g.mtime;
                ed.update(cx, |e, cx| e.reload(&text, cx));
                f.say_briefly("updated", cx);
            });
        })
        .detach();
    }

    pub fn toggle_preview(&mut self, cx: &mut Context<Files>) {
        self.preview = !self.preview;
        cx.notify();
    }

    /// the session's turn ended: its files read again
    pub fn turn_over(&mut self, busy: bool, cx: &mut Context<Files>) {
        if self.busy && !busy {
            self.load_tree(None, cx);
        }
        self.busy = busy;
    }

    /// a picture of the markdown file, asked of the machine once (the web's and data: ones are not fetched)
    fn want_picture(&mut self, src: &str, cx: &mut Context<Files>) {
        if self.pics.contains_key(src) {
            return;
        }
        let rel = self.rel.clone().unwrap_or_default();
        let abs = if src.starts_with('/') {
            Some(src.to_string())
        } else if src.contains("://") || src.starts_with("data:") {
            None
        } else {
            resolve_rel(&rel, src.split(['?', '#']).next().unwrap_or("")).map(|r| format!("{}/{r}", self.root))
        };
        let kind = abs.as_deref().and_then(burrow_core::files::image_kind);
        let (Some(abs), Some(kind)) = (abs, kind) else {
            self.pics.insert(src.to_string(), Pic::Missing);
            return;
        };
        self.pics.insert(src.to_string(), Pic::Loading);
        let (hub, m, key) = (self.hub.clone(), self.m.clone(), src.to_string());
        let task = cx.background_executor().spawn(async move { hub.raw(&m, &abs) });
        cx.spawn(async move |this: WeakEntity<Files>, cx| {
            let r = task.await;
            let _ = this.update(cx, |f, cx| {
                let fmt = match kind {
                    "png" => gpui::ImageFormat::Png,
                    "jpeg" => gpui::ImageFormat::Jpeg,
                    "gif" => gpui::ImageFormat::Gif,
                    "webp" => gpui::ImageFormat::Webp,
                    _ => gpui::ImageFormat::Svg,
                };
                f.pics.insert(key, r.map(|b| Pic::Here(Arc::new(gpui::Image::from_bytes(fmt, b)))).unwrap_or(Pic::Missing));
                cx.notify();
            });
        })
        .detach();
    }

    /// a link in the rendered file: the web in your browser, another file of it here
    fn follow(&mut self, href: &str, window: &mut Window, cx: &mut Context<Files>) {
        if href.starts_with("http://") || href.starts_with("https://") {
            return cx.open_url(href);
        }
        if href.starts_with('#') || href.split_once(':').is_some_and(|(s, _)| !s.contains('/') && s.chars().all(|c| c.is_ascii_alphabetic())) {
            return;
        }
        let rel = self.rel.clone().unwrap_or_default();
        if let Some(r) = resolve_rel(&rel, href.split('#').next().unwrap_or("")) {
            self.open_file(r, Some(window), cx);
        }
    }

    fn tree_el(&mut self, cx: &mut Context<Files>) -> AnyElement {
        let name = self.root.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(&self.root).to_string();
        let rs = rows(&self.dirs, &self.open, &self.git, self.now, self.rel.as_deref());
        let mut list = div()
            .id("tree")
            .flex_none()
            .w(px(220.))
            .h_full()
            .overflow_y_scroll()
            .pt(px(8.))
            .pb(px(10.))
            .border_r_2()
            .border_color(rgb(col::INK2))
            .font_family(kit::MONO)
            .text_size(px(12.))
            .line_height(px(18.))
            .text_color(rgb(col::STEEL3))
            .child(div().px(px(10.)).pb(px(4.)).text_color(rgb(col::STEEL4)).font_weight(FontWeight::BOLD).whitespace_nowrap().overflow_hidden().child(name));
        for (i, r) in rs.into_iter().enumerate() {
            let pad = 10. + r.depth as f32 * 12. + if r.dir || r.waiting { 0. } else { 14. };
            let color = match (r.dir, r.git.as_deref()) {
                (true, _) => col::STEEL4,
                (_, Some("M")) => col::PALE0,
                (_, Some("A")) => 0x7fdcb0,
                _ => col::STEEL3,
            };
            let mut row = div()
                .id(("row", i))
                .flex()
                .pl(px(pad))
                .pr(px(8.))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_color(rgb(color))
                .when(r.dot, |d| d.opacity(0.55))
                .when(r.fresh, |d| d.font_weight(FontWeight::BOLD))
                .when(r.cur, |d| d.bg(rgb(col::LINE)).text_color(rgb(col::STEEL4)))
                .when(!r.waiting, |d| d.cursor_pointer().hover(|s| s.bg(rgb(col::INK2)).text_color(rgb(col::STEEL4))));
            if r.waiting {
                row = row.text_color(rgb(col::STEEL1)).child("…");
                list = list.child(row);
                continue;
            }
            if r.dir {
                row = row.child(div().text_color(rgb(col::STEEL1)).child(if r.open { "▾ " } else { "▸ " }));
            }
            row = row.child(r.name.clone());
            if r.fresh || r.has {
                row = row.child(div().text_color(rgb(col::PALE0)).font_weight(FontWeight::NORMAL).child(" •"));
            }
            let p = r.path.clone();
            if r.dir {
                row = row.on_click(cx.listener(move |f, _, _, cx| f.toggle(p.clone(), cx)));
            } else {
                let drag = FileDrag { m: self.m.clone(), path: format!("{}/{}", self.root, r.path), name: r.name.clone() };
                row = row
                    .on_click(cx.listener(move |f, _, window, cx| f.open_file(p.clone(), Some(window), cx)))
                    .on_drag(drag, |d: &FileDrag, _, _, cx: &mut App| cx.new(|_| DragLabel(d.name.clone())));
            }
            list = list.child(row);
        }
        list.into_any_element()
    }

    fn main_el(&mut self, cx: &mut Context<Files>) -> AnyElement {
        let note = |s: String| div().px(px(20.)).py(px(18.)).font_family(kit::MONO).text_size(px(13.)).text_color(rgb(col::STEEL2)).child(s).into_any_element();
        if let Some(m) = &self.msg {
            return note(m.clone());
        }
        let Some(ed) = self.editor.clone() else { return div().into_any_element() };
        if !(self.md && self.preview) {
            return ed.into_any_element();
        }
        // rendered: parsed again when the text changed
        let text = ed.read(cx).text().to_string();
        if self.blocks.as_ref().is_none_or(|(t, _)| *t != text) {
            self.blocks = Some((text.clone(), markdown::parse(&text)));
        }
        let blocks = self.blocks.as_ref().map(|(_, b)| b.clone()).unwrap_or_default();
        for src in markdown::pictures(&blocks) {
            self.want_picture(&src, cx);
        }
        let me = self.me.clone();
        let on_link: Rc<dyn Fn(&str, &mut Window, &mut App)> = Rc::new(move |href: &str, window: &mut Window, cx: &mut App| {
            let href = href.to_string();
            let _ = me.update(cx, |f, cx| f.follow(&href, window, cx));
        });
        let d = markdown::Draw { pics: &self.pics, on_link, id: "md" };
        div().id("fview").size_full().overflow_y_scroll().px(px(28.)).pt(px(8.)).pb(px(48.)).child(markdown::render(&blocks, &d)).into_any_element()
    }
}

impl Render for Files {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tree = self.tree_el(cx);
        let main = self.main_el(cx);
        div().size_full().flex().flex_row().child(tree).child(div().relative().flex_1().min_w(px(0.)).h_full().child(main))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(n: &str, dir: bool, mtime: i64) -> Entry {
        Entry { name: n.into(), dir, mtime }
    }

    #[test]
    fn the_trees_rows() {
        let dirs = HashMap::from([
            (String::new(), vec![e("src", true, 0), e("docs", true, 0), e(".env", false, 0), e("README.md", false, 990)]),
            ("src".to_string(), vec![e("main.js", false, 100)]),
        ]);
        let open = BTreeSet::from([String::new(), "src".to_string(), "docs".to_string()]);
        let git = BTreeMap::from([("src/main.js".to_string(), "M".to_string()), ("README.md".to_string(), "A".to_string())]);
        let r = rows(&dirs, &open, &git, 1000, Some("README.md"));
        let shown: Vec<(&str, usize)> = r.iter().map(|x| (x.name.as_str(), x.depth)).collect();
        assert_eq!(shown, [("src", 0), ("main.js", 1), ("docs", 0), ("…", 1), (".env", 0), ("README.md", 0)]);
        assert!(r[0].open && r[0].has, "a folder holding a change has its dot");
        assert_eq!(r[1].git.as_deref(), Some("M"));
        assert!(!r[1].fresh, "changed 900 s ago");
        assert!(r[3].waiting, "a folder open but not read yet");
        assert!(r[4].dot);
        assert!(r[5].fresh && r[5].cur && r[5].git.as_deref() == Some("A"));
    }

    #[test]
    fn links_between_files() {
        assert_eq!(resolve_rel("docs/a.md", "../README.md").as_deref(), Some("README.md"));
        assert_eq!(resolve_rel("docs/a.md", "b%20c.md").as_deref(), Some("docs/b c.md"));
        assert_eq!(resolve_rel("a.md", "./x/../y.md").as_deref(), Some("y.md"));
        assert_eq!(resolve_rel("a.md", "../out.md"), None, "out of its root");
        assert!(is_md("x/README.MD") && !is_md("a.rs"));
    }
}
