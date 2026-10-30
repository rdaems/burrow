//! The files beside a terminal (server.py's /api/tree, /api/file and /raw): the tree of a folder on a machine with
//! what git says changed there, a file read, looked at or written for the editor, a picture for a markdown file shown
//! rendered. Each is one round to the machine, a python3 script there (as server.py's TREE, FILE and RAW); the demo's
//! machines have made-up files, kept in memory.

use crate::hub::{Hub, Res};
use crate::runner::run;
use crate::util::sq;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// the file tree beside a terminal: under a session's root, the folders asked for (folders first, then files, by
/// name; 400 at most; .git and .DS_Store left out), each entry [name, folder?, changed (s)]; what git says changed
/// there, relative to the root ("M" changed, "A" new or untracked, "D" gone), if it is in a repo; and the machine's
/// clock
pub const TREE: &str = r#"
import json, os, subprocess, sys, time
root = os.path.abspath(os.path.expanduser(sys.argv[1])); dirs = {}
for d in json.loads(sys.argv[2]):
    p = os.path.normpath(os.path.join(root, d))
    if p != root and not p.startswith(root + os.sep):
        continue
    try:
        es = []
        with os.scandir(p) as it:
            for e in it:
                if e.name in (".git", ".DS_Store"):
                    continue
                try:
                    es.append([e.name, 1 if e.is_dir() else 0, int(e.stat(follow_symlinks=False).st_mtime)])
                except OSError:
                    es.append([e.name, 0, 0])
        es.sort(key=lambda x: (-x[1], x[0].lower()))
        dirs[d] = es[:400]
    except OSError:
        dirs[d] = None
git = {}
try:
    g = lambda *a: subprocess.run(["git", "-C", root] + list(a), capture_output=True, text=True, timeout=5)
    pre = g("rev-parse", "--show-prefix")
    if pre.returncode == 0:
        pre = pre.stdout.strip()
        for line in g("status", "--porcelain=v1", "-uall", "--no-renames").stdout.splitlines():
            code, path = line[:2], line[3:]
            if path.startswith(chr(34)):
                path = json.loads(path)
            if path.startswith(pre):
                git[path[len(pre):]] = "A" if "?" in code or "A" in code else "D" if "D" in code else "M"
except Exception:
    pass
print(json.dumps({"dirs": dirs, "git": git, "now": int(time.time())}))
"#;

/// a file there, for the editor beside a terminal: read (its text; or too big, binary, missing), stat, or write (what
/// comes on stdin; only if it has not changed since it was read, its mtime in ns as a string, unless forced; through a
/// temporary file renamed over it, so a session never reads half a file; its mode kept, a symlink followed). It
/// answers its mtime and size after
pub const FILE: &str = r#"
import json, os, sys, tempfile
a = json.loads(sys.argv[1]); p = os.path.realpath(os.path.expanduser(a["path"]))
def st():
    try:
        s = os.stat(p)
        return {"mtime": str(s.st_mtime_ns), "size": s.st_size}
    except FileNotFoundError:
        return {"missing": True}
out = st()
if a["op"] == "read" and not out.get("missing"):
    if out["size"] > 2000000:
        out["big"] = True
    else:
        data = open(p, "rb").read()
        try:
            if b"\0" in data[:8192]:
                raise ValueError
            out["text"] = data.decode("utf-8")
        except ValueError:
            out["binary"] = True
elif a["op"] == "write":
    if not a.get("force") and out.get("mtime") != a.get("mtime"):
        out["conflict"] = True
    else:
        data = sys.stdin.buffer.read()
        mode = None if out.get("missing") else os.stat(p).st_mode & 0o7777
        fd, tmp = tempfile.mkstemp(dir=os.path.dirname(p), prefix=".burrow-")
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        os.chmod(tmp, mode if mode is not None else 0o644)
        os.replace(tmp, p)
        out = st()
print(json.dumps(out))
"#;

/// a picture there, as base64 (8 MB at most)
pub const RAW: &str = r#"
import base64, os, sys
p = os.path.realpath(os.path.expanduser(sys.argv[1]))
sys.exit(1) if os.path.getsize(p) > 8000000 else sys.stdout.write(base64.b64encode(open(p, "rb").read()).decode())
"#;

/// what a picture is, by its name (only pictures come through /raw)
pub fn image_kind(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
    Some(match ext.as_str() {
        "png" => "png",
        "jpg" | "jpeg" => "jpeg",
        "gif" => "gif",
        "webp" => "webp",
        "svg" => "svg",
        _ => return None,
    })
}

/// a thing in a folder: its name, whether it is a folder, when it changed (seconds)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
    pub mtime: i64,
}

/// a round of the tree: the folders asked for (None: not there any more), git's changes, the machine's clock
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub dirs: BTreeMap<String, Option<Vec<Entry>>>,
    pub git: BTreeMap<String, String>,
    pub now: i64,
}

#[derive(Deserialize)]
struct TreeJson {
    #[serde(default)]
    dirs: BTreeMap<String, Option<Vec<(String, i64, i64)>>>,
    #[serde(default)]
    git: BTreeMap<String, String>,
    #[serde(default)]
    now: i64,
}

/// what the machine says of a file: its mtime (ns, as a string) and size, its text when read; or that it is missing,
/// too big, not text, or changed since it was read (a write not done)
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FileAnswer {
    pub mtime: Option<String>,
    pub size: u64,
    pub text: Option<String>,
    pub missing: bool,
    pub big: bool,
    pub binary: bool,
    pub conflict: bool,
}

/// what to do with a file: read it, look at it, or write it (over the version of `mtime`, unless forced)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileOp {
    Read,
    Stat,
    Write { text: String, mtime: Option<String>, force: bool },
}

/// the demo's files: made up by name, kept in memory (server.py's DEMO_FILES)
static DEMO_FILES: LazyLock<Mutex<HashMap<String, (String, String)>>> = LazyLock::new(Default::default);

fn demo_file(path: &str, op: &FileOp) -> FileAnswer {
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut fs = DEMO_FILES.lock().unwrap();
    let f = fs.entry(path.to_string()).or_insert_with(|| {
        let text = match name {
            "README.md" => "# kite\n\nA made-up project in **burrow**'s demo.\n\n- the room\n- the robots\n\n```js\nconst kite = fly();\n```\n",
            "main.js" => "// main.js\nexport function fly() {\n  return \"up\";\n}\n",
            _ => "",
        };
        (text.to_string(), "1".to_string())
    });
    if let FileOp::Write { text, mtime, force } = op {
        if !force && mtime.as_deref() != Some(f.1.as_str()) {
            return FileAnswer { conflict: true, mtime: Some(f.1.clone()), size: f.0.len() as u64, ..Default::default() };
        }
        let ns = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        *f = (text.clone(), ns.to_string());
    }
    FileAnswer { mtime: Some(f.1.clone()), size: f.0.len() as u64, text: (*op == FileOp::Read).then(|| f.0.clone()), ..Default::default() }
}

/// the demo's tree: the same made-up folders under any root
fn demo_tree(dirs: &[String]) -> Tree {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let e = |n: &str, d: bool, ago: i64| Entry { name: n.into(), dir: d, mtime: now - ago };
    let fake = |d: &str| match d {
        "" => Some(vec![e("src", true, 900), e("docs", true, 86400), e("README.md", false, 40), e("package.json", false, 7200)]),
        "src" => Some(vec![e("lib", true, 3000), e("main.js", false, 20), e("room.js", false, 5000)]),
        "src/lib" => Some(vec![e("util.js", false, 9000)]),
        "docs" => Some(vec![e("notes.md", false, 86400)]),
        _ => None,
    };
    let ask: Vec<String> = if dirs.is_empty() { vec![String::new()] } else { dirs.to_vec() };
    Tree {
        dirs: ask.iter().map(|d| (d.clone(), fake(d))).collect(),
        git: [("src/main.js", "M"), ("README.md", "A")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        now,
    }
}

/// the tree script's answer
pub fn parse_tree(out: &str) -> Res<Tree> {
    let t: TreeJson = serde_json::from_str(out.trim()).map_err(|e| format!("the tree: {e}"))?;
    let dirs = t.dirs.into_iter().map(|(d, es)| (d, es.map(|es| es.into_iter().map(|(name, dir, mtime)| Entry { name, dir: dir != 0, mtime }).collect()))).collect();
    Ok(Tree { dirs, git: t.git, now: t.now })
}

impl Hub {
    /// the tree beside a terminal: the folders `dirs` under `root` (relative; "" the root itself), and git's changes
    pub fn tree(&self, mid: &str, root: &str, dirs: &[String]) -> Res<Tree> {
        let m = self.machine(mid)?;
        if m.demo() {
            return Ok(demo_tree(dirs));
        }
        let ask: Vec<&str> = if dirs.is_empty() { vec![""] } else { dirs.iter().map(String::as_str).collect() };
        let script = format!("python3 -c {} {} {}", sq(TREE), sq(root), sq(&json!(ask).to_string()));
        parse_tree(&run(self.runner(), &m, &script, Duration::from_secs(20))?)
    }

    /// a file there: read, looked at, or written (only over the version read, unless forced)
    pub fn file(&self, mid: &str, path: &str, op: FileOp) -> Res<FileAnswer> {
        let m = self.machine(mid)?;
        if m.demo() {
            return Ok(demo_file(path, &op));
        }
        let (name, input, mtime, force) = match &op {
            FileOp::Read => ("read", None, None, false),
            FileOp::Stat => ("stat", None, None, false),
            FileOp::Write { text, mtime, force } => ("write", Some(text.as_bytes()), mtime.clone(), *force),
        };
        let arg = json!({"op": name, "path": path, "mtime": mtime, "force": force});
        let script = format!("python3 -c {} {}", sq(FILE), sq(&arg.to_string()));
        let o = self.runner().exec(&m, &script, input, Duration::from_secs(20), &mut |_| {})?;
        let out = String::from_utf8_lossy(&o.stdout);
        if o.code != 0 {
            let e = String::from_utf8_lossy(&o.stderr).trim().to_string();
            return Err(if e.is_empty() { format!("exit {}", o.code) } else { e.lines().last().unwrap_or_default().to_string() });
        }
        serde_json::from_str(out.trim()).map_err(|e| format!("the file: {e}"))
    }

    /// a picture there, for a markdown file shown rendered: its bytes (pictures only, 8 MB at most)
    pub fn raw(&self, mid: &str, path: &str) -> Res<Vec<u8>> {
        image_kind(path).ok_or("not a picture")?;
        let m = self.machine(mid)?;
        if m.demo() {
            return Err("not here".into());
        }
        let out = run(self.runner(), &m, &format!("python3 -c {} {}", sq(RAW), sq(path)), Duration::from_secs(30))?;
        b64(out.trim()).ok_or_else(|| "not a picture".to_string())
    }
}

/// base64, as RAW writes it
fn b64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let s = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for ch in s.chunks(4) {
        let mut n = 0u32;
        for (i, c) in ch.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..ch.len().saturating_sub(1)]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_as_python_writes_it() {
        assert_eq!(b64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(b64("aGk=").unwrap(), b"hi");
        assert_eq!(b64("YWJj").unwrap(), b"abc");
        assert!(b64("a$c").is_none());
    }

    #[test]
    fn the_trees_answer() {
        let t = parse_tree(r#"{"dirs": {"": [["src", 1, 10], ["a.md", 0, 20]], "gone": null}, "git": {"a.md": "M"}, "now": 30}"#).unwrap();
        assert_eq!(t.dirs[""].as_ref().unwrap()[0], Entry { name: "src".into(), dir: true, mtime: 10 });
        assert!(t.dirs["gone"].is_none());
        assert_eq!((t.git["a.md"].as_str(), t.now), ("M", 30));
    }

    #[test]
    fn the_demos_files_keep_what_is_saved() {
        let p = "/demo/x/README.md";
        let a = demo_file(p, &FileOp::Read);
        assert!(a.text.as_deref().unwrap().starts_with("# kite"));
        let stale = demo_file(p, &FileOp::Write { text: "new".into(), mtime: Some("0".into()), force: false });
        assert!(stale.conflict, "not over a version it did not read");
        let w = demo_file(p, &FileOp::Write { text: "new".into(), mtime: a.mtime.clone(), force: false });
        assert!(!w.conflict && w.mtime != a.mtime);
        assert_eq!(demo_file(p, &FileOp::Read).text.as_deref(), Some("new"));
        assert_eq!(image_kind("a/b.PNG"), Some("png"));
        assert_eq!(image_kind("a/b.md"), None);
    }
}
