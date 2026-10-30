//! The files beside a terminal, through the hub: the tree and the file scripts run for real (python3 here, in a
//! folder of the test's own) by a runner that runs nothing else, so no claude and no ssh; the demo's made-up ones.

use burrow_core::files::FileOp;
use burrow_core::runner::{Kept, Output};
use burrow_core::{Config, Hub, Mach, Runner};
use serde_json::Value;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// python3 scripts run here; anything else (the polls: claude) answers nothing
#[derive(Default)]
struct Py {
    calls: Mutex<Vec<String>>,
}

impl Runner for Py {
    fn exec(&self, _m: &Mach, script: &str, input: Option<&[u8]>, _t: Duration, _pid: &mut dyn FnMut(u32)) -> Result<Output, String> {
        if !script.starts_with("python3 -c ") {
            return Ok(Output { code: 0, stdout: vec![], stderr: vec![] });
        }
        self.calls.lock().unwrap().push(script.to_string());
        let mut c = Command::new("/bin/sh").arg("-c").arg(script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        if let Some(i) = input {
            c.stdin.take().unwrap().write_all(i).unwrap();
        }
        drop(c.stdin.take());
        let o = c.wait_with_output().map_err(|e| e.to_string())?;
        Ok(Output { code: o.status.code().unwrap_or(1), stdout: o.stdout, stderr: o.stderr })
    }
    fn argv(&self, _m: &Mach, _s: &str, _t: bool) -> Vec<String> {
        vec!["/bin/sh".into(), "-c".into(), "true".into()]
    }
    fn keep(&self, _a: &[String]) -> std::io::Result<Box<dyn Kept>> {
        Err(std::io::Error::other("no"))
    }
    fn scp(&self, _m: &Mach, _l: &Path, _t: &str) -> Result<(), String> {
        Err("no".into())
    }
    fn http(&self, _u: &str, _b: Option<&Value>) -> Result<(u16, String), String> {
        Err("no".into())
    }
}

fn hub(dir: &Path, demo: bool) -> Arc<Hub> {
    let mut cfg = Config::under(dir, demo);
    cfg.inhibitor = Some(vec!["true".into()]);
    std::fs::create_dir_all(cfg.claude_settings.parent().unwrap()).unwrap();
    let h = Hub::new(cfg, Arc::new(Py::default()));
    h.set_machines(vec![Mach { id: "".into(), name: "here".into(), ..Mach::default() }]);
    h
}

#[test]
fn the_tree_of_a_folder_with_gits_changes() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("proj it's");
    std::fs::create_dir_all(root.join("src/deep")).unwrap();
    std::fs::write(root.join("README.md"), "# hi\n").unwrap();
    std::fs::write(root.join("src/a.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join(".DS_Store"), "x").unwrap();
    let git = |a: &[&str]| Command::new("git").arg("-C").arg(&root).args(a).output().unwrap();
    let have_git = git(&["init", "-q"]).status.success();
    if have_git {
        git(&["-c", "user.email=t@t", "-c", "user.name=t", "add", "README.md"]);
        git(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "one"]);
        std::fs::write(root.join("README.md"), "# changed\n").unwrap();
    }
    let h = hub(home.path(), false);
    let r = root.to_string_lossy().to_string();
    let t = h.tree("", &r, &["".into(), "src".into(), "../escape".into(), "gone".into()]).unwrap();
    let top: Vec<(&str, bool)> = t.dirs[""].as_ref().unwrap().iter().map(|e| (e.name.as_str(), e.dir)).collect();
    assert_eq!(top, [("src", true), ("README.md", false)], "folders first, .git and .DS_Store left out");
    assert_eq!(t.dirs["src"].as_ref().unwrap().iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["deep", "a.rs"]);
    assert!(!t.dirs.contains_key("../escape"), "nothing outside its root");
    assert!(t.dirs["gone"].is_none());
    assert!(t.now > 1_700_000_000);
    if have_git {
        assert_eq!(t.git.get("README.md").map(String::as_str), Some("M"));
        assert_eq!(t.git.get("src/a.rs").map(String::as_str), Some("A"), "untracked is new");
    }
}

#[test]
fn a_save_only_over_the_version_read() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join("p");
    std::fs::create_dir_all(&dir).unwrap();
    let real = dir.join("run.sh");
    std::fs::write(&real, "echo one\n").unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();
    let link = dir.join("link.sh");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let h = hub(home.path(), false);
    let p = link.to_string_lossy().to_string();
    let r = h.file("", &p, FileOp::Read).unwrap();
    assert_eq!(r.text.as_deref(), Some("echo one\n"));
    let m0 = r.mtime.clone().unwrap();
    // someone else writes it meanwhile: a save over it is refused
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&real, "echo theirs\n").unwrap();
    let st = h.file("", &p, FileOp::Stat).unwrap();
    assert_ne!(st.mtime.as_deref(), Some(m0.as_str()));
    assert!(st.text.is_none());
    let no = h.file("", &p, FileOp::Write { text: "echo mine\n".into(), mtime: Some(m0), force: false }).unwrap();
    assert!(no.conflict);
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "echo theirs\n");
    // MINE: forced, through the link, its mode kept, the link still a link
    let yes = h.file("", &p, FileOp::Write { text: "echo mine 'quoted' \"too\"\n".into(), mtime: None, force: true }).unwrap();
    assert!(!yes.conflict && yes.mtime.is_some());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "echo mine 'quoted' \"too\"\n");
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::metadata(&real).unwrap().permissions().mode() & 0o7777, 0o755);
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().starts_with(".burrow-")), "no temporary file left");
    // a new version read again: a save over it goes
    let again = h.file("", &p, FileOp::Read).unwrap();
    let ok = h.file("", &p, FileOp::Write { text: "x".into(), mtime: again.mtime, force: false }).unwrap();
    assert!(!ok.conflict);
    // not text, missing
    std::fs::write(dir.join("b.bin"), [0u8, 1, 2]).unwrap();
    assert!(h.file("", &dir.join("b.bin").to_string_lossy(), FileOp::Read).unwrap().binary);
    assert!(h.file("", &dir.join("nope").to_string_lossy(), FileOp::Read).unwrap().missing);
}

#[test]
fn a_picture_from_the_machine() {
    let home = tempfile::tempdir().unwrap();
    let png = home.path().join("a b.png");
    let bytes: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
    std::fs::write(&png, &bytes).unwrap();
    let h = hub(home.path(), false);
    assert_eq!(h.raw("", &png.to_string_lossy()).unwrap(), bytes);
    assert!(h.raw("", &home.path().join("x.md").to_string_lossy()).is_err(), "pictures only");
}

#[test]
fn the_demos_tree_and_files() {
    let home = tempfile::tempdir().unwrap();
    let h = hub(home.path(), true);
    h.set_machines(vec![Mach { id: "".into(), name: "mac".into(), ssh: "@demo".into(), ..Mach::default() }]);
    let t = h.tree("", "/demo/kite", &["".into(), "src".into()]).unwrap();
    assert!(t.dirs[""].as_ref().unwrap().iter().any(|e| e.name == "README.md"));
    assert_eq!(t.git.get("src/main.js").map(String::as_str), Some("M"));
    let r = h.file("", "/demo/kite/src/main.js", FileOp::Read).unwrap();
    assert!(r.text.unwrap().contains("fly"));
}
