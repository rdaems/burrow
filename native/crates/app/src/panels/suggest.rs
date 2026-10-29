//! What a field completes to (web/index.html's `completes` and `browse`), without pixels: the list under the
//! field, the row picked, what a key does to it; the new desk's folder list (as VS Code's own folder dialog,
//! Remote-SSH's "Open Folder": the field a path, the list the folders in it, ".." first, and in the folder it
//! starts in the places Claude Code ran lately with no desk now).

/// a row: what it puts in the field (a folder: where it goes), what it shows, a note beside it, a recent place
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub go: String,
    pub recent: bool,
}

impl Item {
    pub fn plain(s: &str) -> Item {
        Item { label: s.into(), go: s.into(), recent: false }
    }
}

/// what a key did to the list
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// not the list's: on to the field and the form
    Pass,
    /// the pick moved
    Moved,
    /// take this row
    Take(usize),
    /// Esc: the list is put away
    Closed,
}

/// the rows and the one picked
#[derive(Clone, Debug, Default, PartialEq)]
pub struct List {
    pub items: Vec<Item>,
    pub pick: Option<usize>,
}

impl List {
    /// new rows: none picked; what the field says already is not offered (completes)
    pub fn set(&mut self, items: Vec<Item>, text: &str) {
        self.items = items.into_iter().filter(|x| x.go != text).collect();
        self.pick = None;
    }
    pub fn browse(&mut self, items: Vec<Item>) {
        self.items = items;
        self.pick = None;
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.pick = None;
    }
    fn step(&mut self, down: bool) {
        let n = self.items.len();
        self.pick = Some(match (self.pick, down) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        });
    }

    /// a key in the field, as `completes` takes it: ↑ ↓ pick, Tab → Enter take the one picked, Esc puts it away
    pub fn key(&mut self, key: &str) -> Key {
        if self.items.is_empty() {
            return Key::Pass;
        }
        match (key, self.pick) {
            ("down", _) => {
                self.step(true);
                Key::Moved
            }
            ("up", _) => {
                self.step(false);
                Key::Moved
            }
            ("tab" | "right" | "enter", Some(i)) => Key::Take(i),
            ("escape", _) => {
                self.clear();
                Key::Closed
            }
            _ => Key::Pass,
        }
    }

    /// a key in the new desk's path, as `browse` takes it: ↑ ↓ pick, Enter or Tab go into the one picked; Tab with
    /// none picked goes into the one folder left that matches (as a shell completes); Tab is never the focus's
    pub fn browse_key(&mut self, key: &str) -> Key {
        match (key, self.pick) {
            ("down" | "up", _) if !self.items.is_empty() => {
                self.step(key == "down");
                Key::Moved
            }
            ("enter" | "tab", Some(i)) => Key::Take(i),
            ("tab", None) => {
                let only: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].label != ".." && !self.items[i].recent).collect();
                if only.len() == 1 { Key::Take(only[0]) } else { Key::Moved }
            }
            _ => Key::Pass,
        }
    }
}

/// hosts as the machine's field offers them: those starting with what was typed, case aside
pub fn hosts(all: &[String], text: &str) -> Vec<Item> {
    let t = text.to_lowercase();
    all.iter().filter(|h| h.to_lowercase().starts_with(&t)).map(|h| Item::plain(h)).collect()
}

/// a path with the home as ~
pub fn short(home: &str, d: &str) -> String {
    if !home.is_empty() && (d == home || d.starts_with(&format!("{home}/"))) { format!("~{}", &d[home.len()..]) } else { d.to_string() }
}

/// where the field starts: the home, or the root when the home is not known
pub fn start(home: &str) -> &'static str {
    if home.is_empty() { "/" } else { "~/" }
}

/// the folder a folder is in
pub fn up(home: &str, dir: &str) -> String {
    let s = dir.trim_end_matches('/');
    if s == "~" {
        if home.is_empty() {
            return "/".into();
        }
        let parent = match home.rfind('/') {
            Some(0) | None => "/",
            Some(i) => &home[..i],
        };
        return if parent == "/" { "/".into() } else { short(home, parent) + "/" };
    }
    match s.rfind('/') {
        Some(i) if i > 0 => s[..=i].to_string(),
        _ => "/".into(),
    }
}

/// the folders as the machine lists them (`ls -d …*/`), each as the field writes it: ~ for the home, a / at the end
pub fn kids(home: &str, ls: &[String]) -> Vec<String> {
    ls.iter().map(|d| short(home, d.trim_end_matches('/')) + "/").collect()
}

/// the folder the field is in (up to its last /), and what is typed after it
pub fn split(v: &str) -> (&str, &str) {
    match v.rfind('/') {
        Some(i) => (&v[..=i], &v[i + 1..]),
        None => ("", v),
    }
}

/// the list for what the field says, given the folders of its folder (`kids` of `split(v).0`): "..", the folders
/// that start with what is typed after the last / (case aside), and at the start the recent places first
pub fn browse(v: &str, home: &str, recent: &[String], kids: &[String]) -> Vec<Item> {
    let (dir, tail) = split(v);
    let tail = tail.to_lowercase();
    let mut out = Vec::new();
    if v == start(home) {
        out.extend(recent.iter().map(|p| Item { label: p.clone(), go: format!("{p}/"), recent: true }));
    }
    if !dir.is_empty() {
        if dir != "/" {
            out.push(Item { label: "..".into(), go: up(home, dir), recent: false });
        }
        for k in kids {
            if let Some(rest) = k.strip_prefix(dir)
                && rest.to_lowercase().starts_with(&tail)
            {
                out.push(Item { label: rest.into(), go: k.clone(), recent: false });
            }
        }
    }
    out
}

/// what ADD DESK sends: the path without a / at its end (but "/" itself)
pub fn trimmed(dir: &str) -> String {
    let t = dir.trim_end_matches('/');
    if t.is_empty() && dir.starts_with('/') { "/".into() } else { t.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(n: usize) -> List {
        let mut l = List::default();
        l.set((0..n).map(|i| Item::plain(&format!("h{i}"))).collect(), "");
        l
    }

    #[test]
    fn the_pick_wraps_round() {
        let mut l = list(3);
        assert_eq!(l.key("down"), Key::Moved);
        assert_eq!(l.pick, Some(0));
        l.key("up");
        assert_eq!(l.pick, Some(2));
        l.key("down");
        assert_eq!(l.pick, Some(0));
        assert_eq!(l.key("tab"), Key::Take(0));
        assert_eq!(l.key("right"), Key::Take(0));
    }

    #[test]
    fn nothing_picked_nothing_taken() {
        let mut l = list(2);
        assert_eq!(l.key("enter"), Key::Pass, "Enter adds what the field says");
        assert_eq!(l.key("tab"), Key::Pass);
        assert_eq!(l.key("escape"), Key::Closed);
        assert!(l.items.is_empty());
        assert_eq!(l.key("down"), Key::Pass, "an empty list takes no keys");
    }

    #[test]
    fn what_is_typed_is_not_offered() {
        let mut l = List::default();
        l.set(hosts(&["odyssey".into(), "Arthroid".into(), "ody".into()], "ody"), "ody");
        assert_eq!(l.items, vec![Item::plain("odyssey")]);
        assert_eq!(hosts(&["Arthroid".into()], "ar").len(), 1);
    }

    #[test]
    fn paths_go_home_and_up() {
        let h = "/Users/rem";
        assert_eq!(short(h, "/Users/rem/p/burrow"), "~/p/burrow");
        assert_eq!(short(h, "/Users/remy"), "/Users/remy");
        assert_eq!(up(h, "~/p/burrow/"), "~/p/");
        assert_eq!(up(h, "~/p/"), "~/");
        assert_eq!(up(h, "~/"), "/Users/");
        assert_eq!(up(h, "/Users/"), "/");
        assert_eq!(up("/root", "~/"), "/");
        assert_eq!(kids(h, &["/Users/rem/p/".into(), "/opt/".into()]), vec!["~/p/", "/opt/"]);
        assert_eq!(trimmed("~/p/burrow/"), "~/p/burrow");
        assert_eq!(trimmed("/"), "/");
    }

    #[test]
    fn the_folder_list() {
        let kids = vec!["~/Projects/".to_string(), "~/pictures/".into(), "~/Music/".into()];
        let recent = vec!["~/Projects/burrow".to_string()];
        let at_home = browse("~/", "/Users/rem", &recent, &kids);
        assert_eq!(at_home[0], Item { label: "~/Projects/burrow".into(), go: "~/Projects/burrow/".into(), recent: true });
        assert_eq!(at_home[1].label, "..");
        assert_eq!(at_home.len(), 5);
        let typed = browse("~/p", "/Users/rem", &recent, &kids);
        assert_eq!(typed.iter().map(|x| x.label.as_str()).collect::<Vec<_>>(), vec!["..", "Projects/", "pictures/"]);
        let root = browse("/", "", &[], &["/opt/".into()]);
        assert_eq!(root, vec![Item { label: "opt/".into(), go: "/opt/".into(), recent: false }]);
    }

    #[test]
    fn tab_goes_into_the_only_match() {
        let mut l = List::default();
        l.browse(browse("~/Pro", "/h", &[], &["~/Projects/".into(), "~/Music/".into()]));
        assert_eq!(l.browse_key("tab"), Key::Take(1));
        assert_eq!(l.items[1].go, "~/Projects/");
        assert_eq!(l.browse_key("enter"), Key::Pass);
        l.browse_key("down");
        assert_eq!(l.browse_key("enter"), Key::Take(0));
    }
}
