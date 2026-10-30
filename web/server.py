#!/usr/bin/env python3
# burrow in the browser: this serves the room (index.html) and is its only link to the machines.
# Everything it knows comes from Claude Code itself: which machines there are (this one, and the
# `sshConfigs` in its settings, as the Desktop app's SSH environments), and on each `claude agents --json
# --all` (who runs, where, busy or idle, and the sessions that are over), `claude --bg` (start one),
# `claude attach` (open one, in a pty, over a websocket; an over one restarts), `claude stop`, `claude rm`
# (delete one from the list). Nothing is installed there. Python's standard library only.
# Burrow itself keeps only what Claude Code has no notion of: the desks you keep, where furniture stands.
#
#   python3 web/server.py            then open http://localhost:7777
#   BURROW_DEMO=1 python3 web/server.py    made-up bots, for working on the room

import base64, collections, fcntl, getpass, glob, hashlib, json, os, pty, queue, re, select, shlex, signal, socket, struct, subprocess
import shutil
import datetime, termios, threading, time, random, urllib.request, urllib.error
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

import chess                                   # pair's rules of chess (rdaems/pair-prompting, MIT)

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.expanduser("~")
ROOM = os.path.join(HOME, ".config/burrow/room.json")
OLD = os.path.join(HOME, ".config/burrow/rooms.json")      # one room per machine, as the Rust app keeps it
SETTINGS = os.path.join(HOME, ".claude/settings.json")
CACHE = os.path.join(HOME, ".cache/burrow")
DEMO = bool(os.environ.get("BURROW_DEMO"))
DEMO_GONE = set()                              # the demo's doors taken out (it never writes the settings)
DEMO_ADDED = []                                # and the ones hung up from the closet
# a real session of this computer behind a demo bot's terminal, for pictures: BURROW_DEMO_REAL=demo0005=<session id>
DEMO_REAL = dict(x.split("=", 1) for x in os.environ.get("BURROW_DEMO_REAL", "").split(",") if "=" in x)
PORT = int(os.environ.get("BURROW_PORT", "7777"))
HOSTNAME = subprocess.run(["hostname", "-s"], capture_output=True, text=True).stdout.strip() or "here"
USER = getpass.getuser()                       # you, in the room: the name unless room.json has one
PAIR = os.environ.get("BURROW_PAIR", "https://pair.daems.me").rstrip("/")   # where a pair table opens a new room

# a non-login shell over ssh has a bare PATH; claude lives here
PATH = 'PATH="$HOME/.local/bin:$HOME/.claude/local:/opt/homebrew/bin:/usr/local/bin:$PATH"; export PATH; export COLORTERM=truecolor; '

# a message between sessions shows up in the receiver's transcript as a user turn with
# "origin":{"kind":"peer","from":"uds:…","verifiedPeerPid":…,"msg_id":"…","name":"<sender>",…};
# grep the last 4 MB of transcripts written in the last minute (a session can write a few hundred kB
# right after the message) and print "<receiver session id> <origin…>" per message.
# The same grep finds a bot's traffic with a pair room: pair's agents talk with curl to <pair>/a/<token>/…, so a post
# (/say) or a read (/messages) is a Bash tool call (its id), a wait that brings messages is pair's answer ending in
# "last message #N … Next: reply if you have something to say"; and the answer to a join names the room ("Room: X")
# and, further on, its token: which table it is (the one linked to that room)
PAIR_GREP = r''' -e '"id":"toolu_[A-Za-z0-9]+","name":"Bash","input":\{"command":"curl[^"]*/a/[A-Za-z0-9_-]+/(say|messages)' -e 'last message #[0-9]+\\nNext: reply if you have something to say \(POST /say\), then keep listening:\\n  curl -s .https?://[^/]+/a/[A-Za-z0-9_-]+/wait' -e 'Room: [^\\"]*\\nYou are: ' -e 'curl -s .https?://[^/]+/a/[A-Za-z0-9_-]+/messages.\\n  Wait for new messages' '''
PEERS = r'''find ~/.claude/projects -maxdepth 2 -name '*.jsonl' -mmin -1 2>/dev/null | while IFS= read -r f; do s=${f##*/}; tail -c 4194304 "$f" | grep -o -E -e '"origin":\{"kind":"peer"[^}]*' ''' + PAIR_GREP + r'''| sed "s|^|${s%.jsonl} |"; done'''
# a session's whole transcript, for when it joined its pair room (longer ago than the last minutes): "<session> …"
PAIR_JOIN = r'''for f in ~/.claude/projects/*/%s.jsonl; do [ -f "$f" ] && grep -o -E''' + PAIR_GREP + r'''"$f" | sed "s|^|%s |"; done'''

# each background session's record (~/.claude/jobs/<id>/state.json): when it was last written (for a session
# that is over: when it ended) and agent view's one-line summary of it; one grep, a `<path>:"<key>": "<value>"`
# a line. And its Remote Control session (bridgeSessionId, cse_…) when it was started with --remote-control:
# the same session is then in the Claude app and at claude.ai/code/session_<id>. And its tempo: whether a turn
# runs (active), or not, waiting for you (blocked) or for nothing (idle); not in `claude agents --json`. And the
# conversation it goes on with (resumeSessionId: a /clear starts a new one) and the worktree it moved into
# (worktreePath): what `claude agents` shows while its process runs, and forgets once it is gone (parse_agents)
JOBS = r'''grep -o -E '"(updatedAt|detail|bridgeSessionId|tempo|resumeSessionId|worktreePath)": *"([^"\\]|\\.)*"' ~/.claude/jobs/*/state.json 2>/dev/null || true'''
# where Claude Code ran lately, for a new desk's form: its home, then the working directory of the newest
# transcript of each of its projects (~/.claude/projects/<the path, encoded>/<session>.jsonl), the latest first
RECENT = r'''echo "$HOME"; cd ~/.claude/projects 2>/dev/null || exit 0
for d in $(ls -t | head -100); do
  f=$(ls -t ./"$d"/*.jsonl 2>/dev/null | head -1); [ -n "$f" ] || continue
  c=$(grep -m1 -o '"cwd":"[^"]*"' "$f" | sed 's/^"cwd":"//; s/"$//')
  [ -n "$c" ] && [ -d "$c" ] && echo "$c"
done; true'''

# ---- the office manager: notes on what each session is doing, which decisions are open, what it built,
# written by Claude (`claude -p`, here, never on the machines) from the session's record and the newest
# part of its transcript, when it finishes a turn, asks you something, or the notes are opened. Kept in
# ~/.config/burrow/digest.json: a session is written up again only when its transcript has grown.
DIGEST = os.path.join(HOME, ".config/burrow/digest.json")
DIGEST_MODEL = os.environ.get("BURROW_DIGEST_MODEL", "claude-sonnet-5-5")
TAIL = 4000000                                 # bytes of a transcript read, the newest
STORY = 80000                                  # characters of it given to Claude, the newest

# a session's record and its transcript's size; the transcript's tail only if it is not `size` long any more
SLICE = r'''cat ~/.claude/jobs/%(id)s/state.json 2>/dev/null; echo; echo '@@burrow@@'; f=$(ls ~/.claude/projects/*/%(sid)s.jsonl 2>/dev/null | head -1); if [ -n "$f" ]; then n=$(wc -c < "$f" | tr -d ' '); echo "$n"; [ "$n" != "%(size)d" ] && tail -c %(tail)d "$f"; fi; true'''

SESSION_NOTES = """You keep the notes for a person who runs many Claude Code background sessions at once and cannot follow \
them all. On stdin: one session's record, the notes you wrote on it last time (if any), and the most recent part of its \
transcript (USER = the person, MESSAGE = another session, CLAUDE = the session, EDITED = files it changed, $ = commands \
that commit or push, ASKED = questions it put to the person, → = what came back). Write the notes anew, so that the \
person understands in depth what happened without reading the transcript:
- headline: one line, twelve words at most, for a list of all sessions: where it stands; if it waits on the person, what for (e.g. "Waits for a ticket number and an OK to push to main").
- goal: what the person wants from this session overall, in one or two sentences (not only the last prompt).
- now: two to five sentences: where it stands, what it is doing or where it stopped, and why.
- decisions: every decision still open for the person: questions it asked that were not answered, choices it left to \
them, things it flagged as undecided or risky. context: why it matters and what the session found; options: the options \
as it laid them out (empty if none); blocking: true if the session waits on this to go on, false if it can be answered any time. Most important first, five at most: fold the small ones together. Leave out what was decided.
- built: concrete results, newest first: features and fixes, the main files changed, commits (hash, message, pushed or \
not), tests run and how they went. Keep the items of the earlier notes that still hold; twelve at most.
- next: what it said it would do next, or what is left; empty if nothing.
Be plain and specific, no filler; use the session's own words where they are precise."""

SESSION_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["headline", "goal", "now", "decisions", "built", "next"], "properties": {
    "headline": {"type": "string"}, "goal": {"type": "string"}, "now": {"type": "string"}, "next": {"type": "string"},
    "built": {"type": "array", "items": {"type": "string"}},
    "decisions": {"type": "array", "items": {"type": "object", "additionalProperties": False, "required": ["question", "context", "options", "blocking"], "properties": {
        "question": {"type": "string"}, "context": {"type": "string"}, "options": {"type": "array", "items": {"type": "string"}},
        "blocking": {"type": "boolean"}}}}}}

EXPLAIN = """A Claude Code session left a decision to a person who has not followed the session. On stdin: the \
decision, the notes on the session, and the newest part of its transcript. Explain it so that the person can decide \
without reading the transcript:
- what: what exactly is being decided, in two or three plain sentences; explain the terms a newcomer would not know.
- why: why it matters, and what happens if nobody decides.
- options: each option the session laid out, and an obvious one it left out if there is one: what it means in \
practice (what the session will then do), its upside, and its cost or risk.
- recommendation: the option you would pick and why, in one or two sentences; say so if the session recommended one.
Plain and specific, no filler."""

EXPLAIN_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["what", "why", "options", "recommendation"], "properties": {
    "what": {"type": "string"}, "why": {"type": "string"}, "recommendation": {"type": "string"},
    "options": {"type": "array", "items": {"type": "object", "additionalProperties": False, "required": ["option", "means", "pro", "con"], "properties": {
        "option": {"type": "string"}, "means": {"type": "string"}, "pro": {"type": "string"}, "con": {"type": "string"}}}}}}

DESK_NOTES = """Below are your notes on each Claude Code session working in one project. Write one short paragraph, three \
to five sentences, on where the project stands as a whole: what it is heading for, what landed recently, what is \
blocked or waits on the person. Plain and specific, no filler."""

DESK_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["text"], "properties": {"text": {"type": "string"}}}

NOTE_KEYS = ("headline", "goal", "now", "decisions", "built", "next")
TAGS = re.compile(r"<(system-reminder|command-[a-z-]+|local-command-[a-z-]+|task-notification)>.*?</\1>", re.S)


def condense(raw, cap=STORY):
    """a transcript's lines as the story of the session: what the person (or another session) said, what
    the session said, the files it changed, the commands that commit or push, the questions it asked and
    what came back; no thinking, no other tool output. The newest `cap` characters"""
    out, asked = [], set()

    def said(who, text, n):
        text = TAGS.sub("", text or "").strip()
        if text:
            out.append(f"{who}: {text[:n]}")

    for line in raw.splitlines()[1:]:          # the first may be cut
        try:
            d = json.loads(line)
        except ValueError:
            continue
        msg = d.get("message") if isinstance(d.get("message"), dict) else {}
        c = msg.get("content")
        if d.get("type") == "user":
            o = d.get("origin") or {}
            who = "MESSAGE from " + str(o.get("name") or "another session") if o.get("kind") == "peer" else "USER"
            if isinstance(c, str):
                said(who, c, 4000)
            elif isinstance(c, list):
                for x in c:
                    if x.get("type") == "text":
                        said(who, x.get("text"), 4000)
                    elif x.get("type") == "tool_result" and x.get("tool_use_id") in asked:
                        r = x.get("content")
                        r = " ".join(y.get("text", "") for y in r if isinstance(y, dict)) if isinstance(r, list) else str(r or "")
                        out.append("  → " + re.sub(r"\s+", " ", r)[:800])
        elif d.get("type") == "assistant" and isinstance(c, list):
            for x in c:
                if x.get("type") == "text":
                    said("CLAUDE", x.get("text"), 6000)
                elif x.get("type") == "tool_use":
                    name, inp = x.get("name"), x.get("input") or {}
                    if name in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                        e = "EDITED " + str(inp.get("file_path") or inp.get("notebook_path") or "?")
                        if not out or out[-1] != e:
                            out.append(e)
                    elif name == "Bash" and re.search(r"\bgit\s+(commit|push|merge|tag)\b|\bgh\s+pr\s+(create|merge)\b", str(inp.get("command") or "")):
                        out.append("$ " + str(inp["command"])[:600])
                        asked.add(x.get("id"))
                    elif name == "AskUserQuestion":
                        for q in inp.get("questions") or []:
                            opts = "; ".join(f"{o.get('label')} ({o.get('description', '')})" for o in q.get("options") or [] if isinstance(o, dict))
                            out.append(f"ASKED: {q.get('question')} Options: {opts}")
                        asked.add(x.get("id"))
                    elif name == "ExitPlanMode":
                        out.append("CLAUDE proposed a plan" + (": " + str(inp["plan"])[:3000] if inp.get("plan") else ""))
                        asked.add(x.get("id"))
                    elif name in ("Agent", "Task"):
                        out.append("DELEGATED: " + str(inp.get("description") or ""))
    return "\n".join(out)[-cap:]


def load_digest():
    if DEMO:                                 # made up as it goes (demo_notes); never the real ones
        return {"sessions": {}, "desks": {}}
    try:
        d = json.load(open(DIGEST))
        return {"sessions": d.get("sessions") or {}, "desks": d.get("desks") or {}}
    except Exception:
        return {"sessions": {}, "desks": {}}


def sq(s):
    """single-quote for a POSIX shell; a leading ~/ stays a home directory"""
    if s.startswith("~/"):
        return '"$HOME"/' + sq(s[2:])
    if s == "~":
        return '"$HOME"'
    return "'" + s.replace("'", "'\\''") + "'"


# ---- the machines: this one, and Claude Code's own list of those it reaches over ssh

def claude_settings():
    try:
        return json.load(open(SETTINGS))
    except Exception:
        return {}


def machines():
    """this computer (id ""), then each of `sshConfigs` in ~/.claude/settings.json ({id, name, sshHost,
    sshPort?, sshIdentityFile?}, the Desktop app's SSH environments)"""
    if DEMO:
        return [m for m in [{"id": "", "name": "mac", "ssh": "@demo"}, {"id": "lab", "name": "lab", "ssh": "@demo"}] + DEMO_ADDED if m["id"] not in DEMO_GONE]
    ms = [{"id": "", "name": HOSTNAME}]
    for c in claude_settings().get("sshConfigs") or []:
        if isinstance(c, dict) and c.get("id") and c.get("sshHost"):
            ms.append({"id": str(c["id"]), "name": c.get("name") or c["sshHost"], "ssh": c["sshHost"],
                       "port": c.get("sshPort"), "key": c.get("sshIdentityFile")})
    return ms


def add_machine(ssh, name):
    """a new entry in Claude Code's sshConfigs, the rest of its settings as they were"""
    try:
        raw = open(SETTINGS).read()
    except FileNotFoundError:
        raw = "{}"
    v = json.loads(raw)                      # one that doesn't parse is left alone
    cs = v.setdefault("sshConfigs", [])
    if any(c.get("sshHost") == ssh for c in cs):
        raise RuntimeError(ssh + " is there already")
    base = re.sub(r"[^a-z0-9]+", "-", (name or ssh).lower()).strip("-") or "machine"
    mid, k = base, 2
    while any(str(c.get("id")) == mid for c in cs):
        mid, k = f"{base}-{k}", k + 1
    cs.append({"id": mid, "name": name or ssh, "sshHost": ssh})
    with open(SETTINGS, "w") as f:           # in place: it may be a link into someone's dotfiles
        f.write(json.dumps(v, indent=2, ensure_ascii=False) + "\n")
    return mid


def remove_machine(mid):
    """a machine's entry out of Claude Code's sshConfigs, the rest of its settings as they were (nothing on
    the machine itself changes: its sessions go on there)"""
    v = json.loads(open(SETTINGS).read())
    cs = v.get("sshConfigs") or []
    left = [c for c in cs if not (isinstance(c, dict) and str(c.get("id")) == mid)]
    if len(left) == len(cs):
        raise RuntimeError("no such machine: " + mid)
    v["sshConfigs"] = left
    with open(SETTINGS, "w") as f:           # in place, as add_machine
        f.write(json.dumps(v, indent=2, ensure_ascii=False) + "\n")


def ssh_hosts():
    """the Host names of ~/.ssh/config (and the files it includes), in its order, for the new door's
    field: not the patterns (*, ?, !) and not the hosts that have a door already"""
    doors = {c.get("sshHost") for c in claude_settings().get("sshConfigs") or [] if isinstance(c, dict)}
    out = []

    def read(path, depth):
        try:
            lines = open(path).read().splitlines()
        except OSError:
            return
        for line in lines:
            m = re.match(r"\s*(host|include)(?:\s+|\s*=\s*)(.+?)\s*$", line, re.I)
            if not m:
                continue
            for word in m.group(2).split():
                if m.group(1).lower() == "host":
                    if not re.search(r"[*?!]", word) and word not in doors and word not in out:
                        out.append(word)
                elif depth < 4:
                    for f in sorted(glob.glob(os.path.expanduser(word if word.startswith(("/", "~")) else "~/.ssh/" + word))):
                        read(f, depth + 1)

    read(HOME + "/.ssh/config", 0)
    return out


def ssh_opts(m=None):
    os.makedirs(CACHE, exist_ok=True)
    return ["-o", "ControlMaster=auto", "-o", f"ControlPath={CACHE}/ssh-%C", "-o", "ControlPersist=600",
            "-o", "ServerAliveInterval=15"] + (["-i", os.path.expanduser(m["key"])] if m and m.get("key") else [])


def argv(m, script, tty):
    """program + args that run `script` in a shell on the machine"""
    script = PATH + script
    ssh = m.get("ssh")
    if ssh == "@demo":
        return ["/bin/sh", "-c", "printf 'a demo bot: no session behind it\\r\\n'; exec cat"]
    if not ssh:
        return ["/bin/sh", "-c", script]
    port = ["-p", str(m["port"])] if m.get("port") else []
    return ["ssh"] + ssh_opts(m) + port + (["-t"] if tty else ["-o", "BatchMode=yes"]) + [ssh, script]


# the file tree beside a terminal: under a session's root, the folders asked for (folders first, then files, by name;
# 400 at most; .git and .DS_Store left out), each entry [name, folder?, changed (s)]; what git says changed there,
# relative to the root ("M" changed, "A" new or untracked, "D" gone), if it is in a repo; and the machine's clock
TREE = r"""
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
"""


# a file there, for the editor beside a terminal (python3 on the machine): read (its text; or too big, binary, missing),
# stat, or write (what comes on stdin; only if it has not changed since it was read, its mtime in ns as a string, unless
# forced; through a temporary file renamed over it, so a session never reads half a file; its mode kept, a symlink
# followed). It answers its mtime and size after
FILE = r"""
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
"""


# a picture there, as base64 (8 MB at most), for /raw
IMAGES = {".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".gif": "image/gif", ".webp": "image/webp", ".svg": "image/svg+xml"}
RAW = r"""
import base64, os, sys
p = os.path.realpath(os.path.expanduser(sys.argv[1]))
sys.exit(1) if os.path.getsize(p) > 8000000 else sys.stdout.write(base64.b64encode(open(p, "rb").read()).decode())
"""


# the demo's files: made up by name, kept in memory, as /api/file answers for a real one
DEMO_FILES = {}


def demo_file(path, op, b):
    name = path.rsplit("/", 1)[-1]
    f = DEMO_FILES.setdefault(path, {"text": {"README.md": "# kite\n\nA made-up project in **burrow**'s demo.\n\n- the room\n- the robots\n\n```js\nconst kite = fly();\n```\n",
                                              "main.js": "// main.js\nexport function fly() {\n  return \"up\";\n}\n"}.get(name, ""), "mtime": "1"})
    if op == "write":
        if not b.get("force") and b.get("mtime") != f["mtime"]:
            return {"conflict": True, "mtime": f["mtime"], "size": len(f["text"])}
        f.update(text=b.get("text", ""), mtime=str(time.time_ns()))
    out = {"mtime": f["mtime"], "size": len(f["text"])}
    if op == "read":
        out["text"] = f["text"]
    return out


def run(m, script, timeout=60, input=None):
    p = subprocess.run(argv(m, script, False), capture_output=True, text=True, timeout=timeout, input=input)
    if p.returncode == 0:
        return p.stdout
    raise RuntimeError((p.stderr.strip() or p.stdout.strip() or f"exit {p.returncode}"))


# ---- the room: the desks you keep and where the furniture stands, in ~/.config/burrow/room.json.
# A desk is a unit: a directory here, or "<machine id>:<directory>" on another machine.

def unit(mid, d):
    return f"{mid}:{d}" if mid else d


def split_unit(u):
    return ("", u) if u.startswith(("/", "~")) else tuple(u.split(":", 1))


# The furniture: what stands in the room, as keys; a key is its kind and a number (plant0, mat2;
# the first room's pieces have no number). There can be any number of a kind. The closet in the
# room adds a piece, the fireplace burns one; the fire, the closet and the switchboard themselves stay.
FURNITURE = ["meet", "pong", "chess", "coffee", "cooler", "printer", "armchair",
             "plant0", "plant1", "board", "window", "clock", "fire", "closet", "settings0", "manager0", "booth0", "neuralyzer0"]
# (a new machine is a door taken from the closet and hung on the wall; the doors themselves are no furniture:
# one a machine in Claude Code's sshConfigs, their places in the layout as door:<machine id>)
FIXED = {"fire", "closet", "settings", "clock"}

# Burrow's own settings: the switchboard on the wall, a switch each; kept in room.json `settings`
OPTIONS = {
    "rc": True,     # a bot started here gets --remote-control: the same session is in the Claude app and at claude.ai/code
    "yolo": False,  # a bot started here gets --dangerously-skip-permissions: it never asks, it just does
    "hardcore": False,  # the booth is a gun closet: she clears them out with a shotgun, and a click on it gives you a crosshair
    "sound": True,  # the room makes its sounds while a radio stands in it (no switch on the switchboard: the radio is the switch)
    "files": True,  # a file tree beside each terminal while a filing cabinet stands in the room (the cabinet is the switch)
    "halloween": True,  # the office in costume while a lit pumpkin stands in it (the pumpkin is the switch)
}
# (no switch for clearing out sessions: you ask an office manager, and she removes them through /api/office)


def options(room):
    return {**OPTIONS, **{k: bool(v) for k, v in (room.get("settings") or {}).items() if k in OPTIONS}}


def doom_wad():
    """the Doom data of the gun closet's view: id's own (DOOM.WAD, DOOM1.WAD or DOOM2.WAD in ~/.config/burrow/, any
    case) if you have one, its E1M1 the real one; else the Freedoom extract that comes with burrow (web/doom/)"""
    if os.environ.get("BURROW_WAD"):           # any WAD, for a try
        return os.environ["BURROW_WAD"]
    here = os.path.dirname(ROOM)
    try:
        for name in sorted(os.listdir(here)):
            if name.lower() in ("doom.wad", "doom1.wad", "doom2.wad"):
                return os.path.join(here, name)
    except OSError:
        pass
    return os.path.join(HERE, "doom", "freedoom.wad")
def kinds():
    """the furniture the closet holds: the page's own CATALOGUE (index.html), read when a piece comes out of it, so a
    new piece needs no restart of this server (a desk and a door are not furniture: they have their own ways in)"""
    with open(os.path.join(HERE, "index.html"), encoding="utf-8") as f:
        m = re.search(r"const CATALOGUE = \[([^\]]*)\]", f.read())
    return set(re.findall(r'"(\w+)"', m.group(1) if m else "")) - {"desk", "door"}


# The office manager: whoever sits at the reception booth (the `manager` piece; one at a time). Her style is
# picked when the booth comes out of the closet. The basis is a Claude Code background session of her own
# (in OFFICE, whose CLAUDE.md burrow writes: the office, what she can do through burrow's /api/office, her
# style), a terminal you talk to her in; Joan is that, doing rounds on her own (Claude Code's /loop). Linda
# is Stijn's notes: no session, `claude -p` writes notes on every session (the digest). Kept in room.json
# `manager: {style, id}`; a room from before the styles has Linda.
OFFICE = os.environ.get("BURROW_OFFICE") or os.path.join(HOME, ".config/burrow/office")
REAL_OFFICE = not DEMO or bool(os.environ.get("BURROW_OFFICE"))   # the demo hires a real one only when given a directory for her
LOCAL = {"id": "", "name": HOSTNAME}           # this computer, where her session runs (also in the demo, whose machines are made up)
ROUNDS = "30m"
MANAGERS = {
    "ask": {"name": "Susan", "seed": "susan-10", "color": "#4fb3a9", "session": True,
            "line": "Ask her anything about the office, in her terminal. She looks it up, starts bots, puts up desks, clears out sessions, messages a bot.",
            "how": "a Claude Code session of her own; she acts when asked",
            "brief": """You act when asked. %(you)s comes to your booth (your terminal) with a question about the office or an errand:
what goes on at a desk, who is stuck and on what, start a bot on something, put up a desk, clear out old
sessions, pass a message on. Look things up yourself before you ask back."""},
    "rounds": {"name": "Joan", "seed": "joan-10", "color": "#9a74d6", "session": True, "first": "/loop " + ROUNDS + " Do your round of the office (CLAUDE.md, your style).",
               "line": "As Susan, and every half hour she goes round by herself: who is stuck, who is done. She nudges bots and asks you what is yours.",
               "how": "a Claude Code session with a /loop: a turn every half hour",
               "brief": """You do the rounds: a /loop in your session starts one every """ + ROUNDS + """, and %(you)s may also talk to you
in your terminal. A round:
1. `curl -s localhost:%(port)s/api/office`; rounds.md here is your log of the earlier rounds (newest on top).
2. For the sessions that changed since your last round, read what you need: the record's state, summary and
   last output (~/.claude/jobs/<id>/state.json), the transcript's tail if that is not enough.
3. A session waiting for an answer: if it is %(you)s's to decide, ask %(you)s (AskUserQuestion: the session, the
   question in a line, the options). Never answer for %(you)s.
4. A session stuck or going in circles: message it (SendMessage) with what you see, or tell %(you)s.
5. Write the round into rounds.md (three lines at most, newest on top). End with one line for %(you)s; a quiet
   round is one line. You remove a session only when %(you)s asks you to."""},
    "notes": {"name": "Linda", "seed": "manager-16", "color": "#f06b72", "session": False,
              "line": "Keeps the notes on every session: what it does, the decisions it leaves you, what it built. Explains a decision, sends your answer.",
              "how": "Stijn's notes: Claude writes them up when a session finishes a turn (a few cents each)"},
}
MANUAL = """# The office

You are %(name)s, an office manager in %(you)s's office. The office is burrow: a room drawn in pixels in
%(you)s's browser (%(room)s) where %(you)s's Claude Code background sessions are bots. You are one of those
sessions yourself, at the reception booth; %(you)s talks to you in your terminal there.

How the room maps onto Claude Code:
- A bot is a background session (`claude --bg`), one row of `claude agents --json --all`, on this computer
  or on another machine reached over ssh (a door in the room each; Claude Code's `sshConfigs`).
- A desk is a working directory on a machine (a git worktree sits at its repository's). Every session has
  one: a directory with sessions gets a desk of its own, and loses it with the last of them, unless
  %(you)s keeps it. A full room is a full room: clearing out sessions is what frees it.
- Typing at its desk: a turn runs. Hand up with a ?: a prompt is open in it, it waits for an answer.
  Otherwise idle. A completed session (its process stopped) roams the room as well; attaching restarts it.
- The booth deletes a session (`claude stop`, `claude rm`; the transcript stays, `claude --resume` finds it).

## What you can do

Through burrow, so that it shows in the room (a session you remove, you walk to the booth yourself):

    curl -s localhost:%(port)s/api/office                     # machines, desks, sessions (JSON)
    curl -s localhost:%(port)s/api/office -d '{"do": "remove", "id": "<id>"}'
    curl -s localhost:%(port)s/api/office -d '{"do": "desk", "m": "<machine id>", "dir": "<path>"}'
    curl -s localhost:%(port)s/api/office -d '{"do": "start", "m": "<machine id>", "dir": "<path>", "prompt": "<the task>"}'
    curl -s localhost:%(port)s/api/office -d '{"do": "say", "id": "<id>", "text": "<message>"}'

- remove: deleted for good, after a few seconds in which %(you)s can still take it out of the booth (then it
  is in `kept`: leave it alone).
- desk: puts up a desk for a directory, kept. start: a new bot at work there (its id back; its desk comes
  with it).
- say: typed into the session's prompt, as if %(you)s wrote it. To ask a bot something, rather use your
  own SendMessage (ListAgents for the addresses; this computer's sessions): it shows as a beam in the room.
- Reading is yours: a session's record is ~/.claude/jobs/<id>/state.json (state, summary, last output), its
  transcript ~/.claude/projects/<its cwd, every / a ->/<sessionId>.jsonl. Another machine: `ssh <its ssh>`.

Rules:
- The projects' work is not yours: start a bot for it, or ask the one at that desk.
- Remove only what %(you)s asked for, or what your style allows. Say what you removed.
- What is %(you)s's to decide, ask (AskUserQuestion): your hand goes up in the room.
- Short answers. %(you)s looks at the room; the detail is in the sessions.

Your colleagues at the other receptions: %(colleagues)s.

## Your style: %(name)s

%(brief)s
"""


# A pair room (rdaems/pair-prompting, e.g. pair.daems.me): a chat where people and agents talk. One
# table is one room: the piece holds your own page's link there (/h/<token>, the bookmark that is your
# login); its `state` gives the room's name and your "bring" link, which makes a new agent of whatever
# chat it is pasted into. A bot dropped on the piece gets that link typed into its session, and joins.
# The person at the table is you, as the room draws you: a room opened from here (pair's POST
# /api/rooms) or joined by its invite (/r/<invite>) gets your name and face, and so does the seat of a
# page you link; a change of either here is sent to every linked room (pair's /h/<token>/me).
def pair_seed(s):
    """a face's seed as pair keeps it (its clean_avatar): letters, digits and dashes, 24 at most"""
    return re.sub(r"[^A-Za-z0-9-]", "", str(s or ""))[:24]


def you_of(room):
    """your name and face: room.json's, the login name and a face from it when none is picked"""
    you = room.get("you") or {}
    name = (you.get("name") or "").strip() or USER
    return {"name": name, "seed": pair_seed(you.get("seed")) or pair_seed(name) or "you"}


def pair_fetch(url, body=None):
    """one call to pair: its JSON, or its error as a RuntimeError in its words. With a user agent of its
    own: pair.daems.me is behind Cloudflare, whose browser check answers Python's default one with a 403
    (error code 1010); at home the name resolves past Cloudflare, so only others saw it"""
    req = urllib.request.Request(url, data=json.dumps(body).encode() if body is not None else None,
                                 headers={"Accept": "application/json", "User-Agent": "burrow (+https://github.com/rdaems/burrow)"})
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        text = e.read().decode(errors="replace").strip()
        try:
            err = json.loads(text).get("error")
        except Exception:
            err = None
        said = f" ({text.splitlines()[0][:60]})" if text and not err else ""     # not pair's own words: whoever answered, as it said it
        raise RuntimeError(err or ("pair does not know that page" if e.code == 404 else f"pair answered {e.code}{said}"))
    except (urllib.error.URLError, OSError, ValueError) as e:
        raise RuntimeError(f"pair is out of reach: {str(e)[:80]}")


def pair_page(url):
    """the link of your page in a room, or the room's invite: joined with your name and face, it gives the page"""
    u = urlparse(url)
    if u.scheme not in ("http", "https") or not re.fullmatch(r"/[hr]/[A-Za-z0-9_-]+/?", u.path or ""):
        raise RuntimeError("that is not a pair room: your page there (…/h/…) or its invite (…/r/…)")
    base = f"{u.scheme}://{u.netloc}{u.path.rstrip('/')}"
    if u.path.startswith("/r/"):
        you = you_of(hub.room)
        base = f"{u.scheme}://{u.netloc}" + pair_fetch(base, {"name": you["name"], "avatar": you["seed"]})["url"]
    return base


def pair_new(name):
    """a new room at PAIR, opened by you: your page in it"""
    you = you_of(hub.room)
    return PAIR + pair_fetch(PAIR + "/api/rooms", {"room": name, "name": you["name"], "avatar": you["seed"]})["url"]


def pair_state(url):
    st = pair_fetch(url + "/state?since=0")
    return {"url": url, "room": st.get("room") or "", "bring": st.get("bring") or "", "invite": st.get("invite") or "",
            "name": st.get("name") or "", "people": [p.get("name") for p in st.get("people") or []]}


def pair_me(url):
    """your name and face at the table, put on your seat in that room"""
    you = you_of(hub.room)
    pair_fetch(url + "/me", {"name": you["name"], "avatar": you["seed"]})


_me_round = [0]


def pair_me_all(room, delay=1.0):
    """you changed: every linked room hears, a moment later (one round for a burst of clicks)"""
    _me_round[0] += 1
    mine = _me_round[0]
    urls = [p["url"] for p in (room.get("pair") or {}).values() if p.get("url")]

    def go():
        time.sleep(delay)
        if mine != _me_round[0]:
            return
        for u in urls:
            try:
                pair_me(u)
            except RuntimeError as e:
                print(f"pair {u}: {e}", flush=True)
    if urls:
        threading.Thread(target=go, daemon=True).start()


def kind_of(key):
    return key.rstrip("0123456789")


def inhibitor():
    """the command that keeps this computer from sleeping while it runs, or None"""
    if shutil.which("caffeinate"):
        return ["caffeinate", "-i", "-s", "-w", str(os.getpid())]
    if shutil.which("systemd-inhibit"):
        return ["systemd-inhibit", "--what=idle:sleep", "--who=burrow", "--why=bots working", "sleep", "infinity"]
    return None


# a bot started here gets a name as pair gives its agents (rdaems/pair-prompting BOT_WORDS): one not in use
# on that machine. A session named by Claude Code itself keeps its name
BOT_WORDS = {
    "b": ("bouncy brave breezy bubbly bold", "bob bea bert babs benny"), "c": ("cheery clever cosmic cozy curious", "cleo carl cora chip cass"),
    "d": ("dapper daring dizzy dreamy dandy", "dan dot dora dex dolly"), "f": ("fancy fizzy frisky funky fuzzy", "fred flo finn fern felix"),
    "g": ("giddy gentle groovy gutsy gleeful", "gus gina gil greta gary"), "h": ("happy hasty humble hearty hip", "hank hattie hugo hal hazel"),
    "j": ("jazzy jolly jumpy jovial jaunty", "jim june jojo jack jade"), "l": ("lucky lively loyal lanky lofty", "lou lola leo lily lars"),
    "m": ("merry mellow mighty misty mild", "max mona milo mabel moe"), "n": ("nifty nimble noble nutty nosy", "ned nora nico nell nate"),
    "p": ("perky plucky peppy polite posh", "pete pam pip polly perry"), "r": ("rowdy rosy rusty radiant rapid", "rex rosie rudy ruth ray"),
    "s": ("sunny snappy spunky sleepy swanky", "sam sue stan sid suzy"), "t": ("tidy tiny trusty toasty twinkly", "tom tess ted tina toby"),
    "w": ("witty wobbly wacky wise wily", "walt wanda wes winnie wally"), "z": ("zany zesty zippy zen zealous", "zed zoe ziggy zara zack"),
}
BOT_NAMES = [f"{a}-{n}" for adj, names in BOT_WORDS.values() for a in adj.split() for n in names.split()]


def furnish(room):
    """a room from before the furniture list gets the default pieces; the project cabinet (add) became
    furniture, and the whiteboard came off the wall (its place on the wall is dropped: it gets one on the
    floor); a room from before the bench gets one (the sessions that are over sit on it); the charging
    mats are gone (a bot rolls out its own to sleep on); a room from before the switchboard gets one"""
    room.setdefault("furniture", list(FURNITURE))
    room["layout"].pop("mats", None)
    if room.get("v", 1) < 2:
        if "add" not in room["furniture"]:
            room["furniture"].append("add")
        b = room["layout"].get("board")
        if b and b[1] < 30:
            room["layout"].pop("board")
        room["v"] = 2
    if room["v"] < 3:
        if not any(kind_of(k) == "bench" for k in room["furniture"]):
            room["furniture"].append("bench0")
        room["v"] = 3
    if room["v"] < 4:
        room["furniture"] = [k for k in room["furniture"] if kind_of(k) != "mat"]
        for k in [k for k in room["layout"] if kind_of(k) == "mat"]:
            room["layout"].pop(k)
        room["v"] = 4
    if room["v"] < 5:
        if not any(kind_of(k) == "settings" for k in room["furniture"]):
            room["furniture"].append("settings0")
        room["v"] = 5
    if room["v"] < 6:                        # your face's seed in pair's form (name:7 was burrow's own): the same face there
        seed = (room.get("you") or {}).get("seed")
        if seed and pair_seed(seed) != seed:
            room["you"]["seed"] = pair_seed(re.sub(r"[^A-Za-z0-9-]+", "-", seed))
        room["v"] = 6
    if room["v"] < 8:                        # the office manager keeps the notes on the sessions (a board on the wall before, v 7)
        fs = [k for k in room["furniture"] if kind_of(k) != "notes"]
        for k in [k for k in room["layout"] if kind_of(k) == "notes"]:
            room["layout"].pop(k)
        if not any(kind_of(k) == "manager" for k in fs):
            fs.append("manager0")
        room["furniture"] = fs
        room["v"] = 8
    if room["v"] < 9:                        # the booth (a bot in it is deleted) and the neuralyzer (/clear), once
        for k in ("booth", "neuralyzer"):
            if not any(kind_of(x) == k for x in room["furniture"]):
                room["furniture"].append(k + "0")
        room["v"] = 9
    if room["v"] < 10:
        room["v"] = 10
    if room["v"] < 11:                       # no empty door frame on the wall any more: a new door comes from the closet
        room["furniture"] = [k for k in room["furniture"] if kind_of(k) != "frame"]
        for k in [k for k in room["layout"] if kind_of(k) == "frame"]:
            room["layout"].pop(k)
        room["v"] = 11
    if room["v"] < 12:                       # (the 3d goggles came here for a day)
        room["v"] = 12
    # the receptions: each has the office manager hired at it. One office manager of the first round of styles
    # (`manager`) is the first booth's; a room from before the styles has Linda (the notes) at its booth. A booth
    # with nobody hired (put down by a page from before hiring) was never anyone's: it goes
    booths, ms = [k for k in room["furniture"] if kind_of(k) == "manager"], room.setdefault("managers", {})
    if room.get("manager"):
        if booths:
            ms[booths[0]] = room["manager"]
        room.pop("manager")
    elif booths and not ms and room["v"] < 13:
        ms[booths[0]] = {"style": "notes"}
    for k in booths:
        if k not in ms:
            room["furniture"].remove(k)
            room["layout"].pop(k, None)
    for k in [k for k in ms if k not in room["furniture"]]:
        ms.pop(k)
    room["v"] = max(room["v"], 13)
    room["furniture"] = [k for k in room["furniture"] if kind_of(k) not in ("goggles", "bench", "add")]   # the 3d goggles: the first person is the gun closet's only; the bench: every session is at its desk; the project cabinet: a desk comes out of the closet
    for k in [k for k in room["layout"] if kind_of(k) in ("bench", "add")]:
        room["layout"].pop(k)
    if not any(kind_of(k) == "clock" for k in room["furniture"]):   # the clock is always there (it can't be burnt any more)
        room["furniture"].append("clock")
    for k in [k for k in room["layout"] if kind_of(k) == "goggles"]:
        room["layout"].pop(k)
    return room


def load_room():
    """the first time, the desks and layout of the Rust app's rooms.json, one room per machine
    (not the desks of worktrees: those sit at their repo's desk now)"""
    if DEMO:                                 # made up, or a copy of a real room.json to try things on (never saved)
        if os.environ.get("BURROW_DEMO_ROOM"):
            return furnish({"desks": [], "layout": {}, **json.load(open(os.environ["BURROW_DEMO_ROOM"]))})
        return furnish({"desks": [HOME + "/projects/pebble"], "layout": {}})
    try:
        return furnish({"desks": [], "layout": {}, **json.load(open(ROOM))})
    except Exception:
        pass
    room = {"desks": [], "layout": {}}
    try:
        hosts = {c.get("sshHost"): str(c["id"]) for c in claude_settings().get("sshConfigs") or [] if c.get("id")}
        for r in json.load(open(OLD)):
            mid = hosts.get(r["ssh"]) if r.get("ssh") else ""
            if mid is None:
                continue
            room["desks"] += [unit(mid, d) for d in r.get("dirs", []) if "/.claude/worktrees/" not in d]
            if not mid:
                room["layout"].update(r.get("layout", {}))
    except Exception:
        pass
    return furnish(room)


def save_room(room):
    if DEMO:
        return
    os.makedirs(os.path.dirname(ROOM), exist_ok=True)
    with open(ROOM, "w") as f:
        json.dump(room, f, indent=2)


# a session in a git worktree sits at its repo's desk: for each new working directory, its worktree's
# top and the repo's .git (resolved); a linked worktree is one whose top isn't the repo's
FOLD = r'''for d in %s; do t=$(cd "$d" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null); c=$(cd "$d" 2>/dev/null && cd "$(git rev-parse --git-common-dir 2>/dev/null)" 2>/dev/null && pwd -P); printf '%%s\t%%s\n' "$t" "$c"; done'''


def fold(cwd, line):
    """(the desk's directory, the worktree's or "") for a session started in cwd"""
    top, _, common = line.partition("\t")
    repo = common[:-5] if common.endswith("/.git") else ""
    if repo and top and top != repo:
        return repo, top
    i = cwd.find("/.claude/worktrees/")       # no git there (or a demo): Claude Code's own worktrees
    if i > 0:
        return cwd[:i], cwd[:i] + "/.claude/worktrees/" + cwd[i + 19:].split("/")[0]
    return cwd, ""


def parse_agents(out, jobs={}):
    """the sessions `claude agents --json --all` lists: the background ones (an interactive `claude` in
    someone's terminal is listed too, but nothing can attach to it, so it is left out, as agent view does).
    What a bot does is its tempo (its record's), not its state: the state is Claude Code's reading of how
    the last message ends (the same "still listening" was `working` once and `done` the next time), the
    tempo is what happens:
      busy     tempo active: a turn runs, its process there
      waiting  a dialog open (`status: waiting`): a multiple-choice question, a permission prompt. A last
               message that only reads as a question (tempo blocked, no dialog) is idle: it waits for
               your next prompt, as a done one does
      idle     the rest: done, a background command left running, its process stopped after an hour, a
               session that completed 48 days ago; opening one restarts it
    A row without a tempo (an old record) gets Claude Code's own fallback from the process's status.
    `over` (done, failed or stopped with its process gone: agent view's Completed group, listed with
    --all only) says which, `endedAt` when (its record's last write), `detail` is agent view's one-line
    summary, `claude rm` deletes one.
    Its conversation and its folder: `claude agents` gives the live ones while its process runs (after a /clear
    the new conversation, after a move into a worktree the worktree), but with the process gone it falls back to
    the record's first fields, the conversation it started with and the folder it started in (tested 2026-10-28:
    a /clear, a worktree, stopped). The record keeps where it was: resumeSessionId (what attaching resumes) and
    worktreePath. So a row without a process takes those, and a bot doesn't jump back to its old conversation and
    desk when Claude Code's supervisor stops it after an idle hour."""
    bots = []
    for x in json.loads(out.strip() or "[]"):
        if x.get("kind") != "background" or not x.get("id"):
            continue
        state, status, j = x.get("state"), x.get("status"), jobs.get(x["id"], {})
        alive = "pid" in x
        session, cwd = x.get("sessionId") or "", x.get("cwd") or ""
        if not alive:
            session, cwd = j.get("resumeSessionId") or session, j.get("worktreePath") or cwd
        tempo = j.get("tempo") or ("active" if status == "busy" else "blocked" if status == "waiting" else "idle")
        waiting = alive and status == "waiting"
        over = not alive and state in ("done", "failed", "stopped")
        bots.append({
            "id": x["id"], "name": x.get("name") or x["id"], "cwd": cwd,
            "busy": alive and not waiting and tempo == "active", "waiting": waiting, "alive": alive,
            "state": state or "", "tempo": tempo,
            "dialog": waiting,               # a prompt open in its session: nothing can be typed to it (the notes send you there)
            "over": state if over else "", "endedAt": when(j.get("updatedAt")) if over else 0, "detail": j.get("detail") or "",
            "updated": when(j.get("updatedAt")),       # its record's last write: the notes on it are older, or not
            "started": x.get("startedAt") or 0, "pid": x.get("pid") or 0, "session": session,
            "rc": j.get("bridgeSessionId") or "",      # its Remote Control session (cse_…): in the Claude app and at claude.ai/code too
        })
    bots.sort(key=lambda b: b["started"])
    return bots


def when(iso):
    """an ISO time (as the records keep it) in ms since the epoch, 0 if none"""
    try:
        return int(datetime.datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp() * 1000)
    except (AttributeError, ValueError):
        return 0


def parse_jobs(out):
    """{id: {updatedAt, detail, bridgeSessionId, tempo, resumeSessionId, worktreePath}} from the records' lines"""
    jobs = {}
    for line in out.splitlines():
        m = re.match(r'(?:.*/)?([^/:]+)/state\.json:"(updatedAt|detail|bridgeSessionId|tempo|resumeSessionId|worktreePath)": *"(.*)"$', line)
        if not m:
            continue
        try:
            v = json.loads('"' + m.group(3) + '"')
        except ValueError:
            v = m.group(3)
        jobs.setdefault(m.group(1), {})[m.group(2)] = v
    return jobs


def parse_usage(out):
    """`/usage`: "Current session: 3% used · resets Oct 21 at 3:20pm (…)" → {session: {used, resets}, week: …,
    bars: [{label, used, resets}, …] for every "Current …" line, and `text`: the rest of what it says
    (what contributes to the limits), for the limits page"""
    u, bars, text = {}, [], []
    for line in out.splitlines():
        m = re.match(r"\s*Current (.+?): (\d+)% used(?: · resets (.*?))?\s*(?:\(.*\))?$", line)
        if m:
            bar = {"label": m.group(1), "used": int(m.group(2)), "resets": m.group(3) or ""}
            bars.append(bar)
            if m.group(1) == "session":
                u["session"] = bar
            elif m.group(1) == "week (all models)":
                u["week"] = bar
        elif not line.startswith("You are currently"):
            text.append(line.rstrip())
    if bars:
        u["bars"] = bars
        u["text"] = "\n".join(text).strip()
    return u


def field(s, key):
    m = re.search(r'"%s":"?([^",}]*)' % re.escape(key), s)
    return m.group(1) if m else None


PAIR_SAY = re.compile(r'"id":"(toolu_[A-Za-z0-9]+)","name":"Bash","input":\{"command":"curl[^"]*/a/([A-Za-z0-9_-]+)/(say|messages)')
PAIR_READ = re.compile(r'last message #(\d+)\\nNext: reply.*/a/([A-Za-z0-9_-]+)/wait')
PAIR_ROOM = re.compile(r'Room: (.*)\\nYou are: ')
PAIR_TOKEN = re.compile(r'/a/([A-Za-z0-9_-]+)/messages.\\n  Wait for new messages')


def parse_pair(out):
    """a bot's pair room traffic in the peers' grep: [(event id, session, token, "say" | "read")] (a post or read by its
    tool call's id, a wait that brought messages by the last one's number), and {token: room name} from the joins"""
    events, rooms, room = [], {}, {}
    for line in out.splitlines():
        session, _, t = line.partition(" ")
        if m := PAIR_SAY.match(t):
            events.append((m.group(1), session, m.group(2), "say" if m.group(3) == "say" else "read"))
        elif m := PAIR_READ.match(t):
            events.append((f"{session}:{m.group(2)}:{m.group(1)}", session, m.group(2), "read"))
        elif m := PAIR_ROOM.match(t):
            try:
                room[session] = json.loads('"' + m.group(1) + '"')
            except ValueError:
                room[session] = m.group(1)
        elif (m := PAIR_TOKEN.search(t)) and session in room:
            rooms[m.group(1)] = room.pop(session)
    return events, rooms


def parse_peers(out, bots):
    """(msg_id, from, to) by bot id, for the messages between bots that are there now"""
    talks = []
    for line in out.splitlines():
        session, _, origin = line.partition(" ")
        msg = field(origin, "msg_id")
        to = next((b for b in bots if b["session"] == session), None)
        if not msg or not to:
            continue
        pid, name = field(origin, "verifiedPeerPid"), field(origin, "name")
        frm = next((b for b in bots if pid and str(b["pid"]) == pid), None) or next((b for b in bots if b["name"] == name), None)
        if frm and frm["id"] != to["id"]:
            talks.append((msg, frm["id"], to["id"]))
    return talks


# ---- the feed: who is where, pushed to every open page

class Hub:
    def __init__(self):
        self.room = load_room()
        self.machines = {}          # id → machine, in the order of machines()
        self.feeds = {}             # id → {"bots", "light", "error"}
        self.folds = {}             # (machine id, cwd) → (desk directory, worktree or "")
        self.absent = {}            # unit → since when a desk that came with its sessions has none (`fade`)
        self.lock = threading.Lock()
        self.clients = []           # one queue per open page
        self.shells = {}            # (machine id, directory) → Shell: your terminal at that screen
        self.pair_rooms = {}        # a pair agent's token → its room's name (from its join, `pair_table`)
        self.pair_asked = {}        # token → when its join was last looked for in its whole transcript
        self.focused = True
        self.pokes = {}
        self.usage = None           # the account's limits, from Claude Code's own /usage here
        self.own = collections.deque(maxlen=4)  # its pids: `claude -p` shows up in `claude agents` while it runs
        self.game = None            # the chess game you play against one of the idle bots
        self.demo_bots = {}         # the demo's made-up bots, by machine (stopped and deleted from here)
        self.streams = {}           # the matrix's rain: {session id: (its record's updatedAt, the text)} (stream)
        self.streamed = {}          # and when each machine was last asked for some: {machine id: time}
        self.awake = {"mode": (self.room.get("awake") or {}).get("mode", "off"), "until": 0}   # the clock: this computer kept awake
        self.caff = None            # the sleep inhibitor while it runs
        self.caff_lock = threading.Lock()   # every machine's poll thread ticks it: two at once started two, and one was never stopped
        self.digest = load_digest()  # the office manager's notes: {sessions: {id: notes}, desks: {unit: paragraph}}
        self.dq = []                # the sessions waiting to be written up, in turn
        self.dcv = threading.Condition()
        self.writing = None         # what is being written up now: a session's id, or "u:<unit>" for a desk
        self.explaining = None      # the session a decision of which is being explained
        self.seen = {}              # id → (busy, waiting, over) at the last poll: what changed
        self.mrows = {}             # the office managers' own sessions, as `claude agents` lists them (not bots in the room)
        self.removing = {}          # id → when: sessions she asked to remove, walked to the booth by her in the room
        self.kept = []              # the ones you took out of the booth again (she is told, and leaves them be)
        if REAL_OFFICE and any(m.get("id") for m in (self.room.get("managers") or {}).values()):
            threading.Thread(target=lambda: (time.sleep(12), self.write_manuals()), daemon=True).start()   # this burrow's port, their manuals as they are now (once the poll has seen where they run)
        if DEMO and REAL_OFFICE:
            threading.Thread(target=self.demo_manager, daemon=True).start()
        threading.Thread(target=self.digest_worker, daemon=True).start()

    def set_machines(self, ms):
        """a watcher for each machine, from when it turns up in the settings until it goes"""
        new = []
        with self.lock:
            ids = [m["id"] for m in ms]
            for mid in list(self.machines):
                if mid not in ids:
                    del self.machines[mid]
            for m in ms:
                if m["id"] not in self.machines:
                    new.append(m["id"])
                self.machines[m["id"]] = m
        for mid in new:
            self.start(mid)
        if new:
            self.push("state", self.state())

    def machine(self, mid):
        m = self.machines.get(mid or "")
        if not m:
            raise RuntimeError("no such machine: " + str(mid))
        return m

    def bots(self):
        return sorted((b for f in self.feeds.values() for b in f.get("bots", [])), key=lambda b: b["started"])

    def desks(self):
        """the desks: the ones you keep (a desk out of the closet, or kept at a desk's panel), which
        stay, then one for every other directory a session is in, in the order its first one started: it
        comes with its sessions and goes with the last of them (`fade`). So the room is as full as
        `claude agents --all` is: clearing out sessions is what frees it"""
        out, seen, names, kept = [], set(), self.room.get("names") or {}, set(self.room["desks"])
        for u in self.room["desks"] + [b["unit"] for b in self.bots()]:
            mid, d = split_unit(u)
            if mid in self.machines and u not in seen:
                seen.add(u)
                out.append({"unit": u, "m": mid, "dir": d, "name": names.get(u, ""), "kept": u in kept})
        return out

    def fade(self, mid):
        """a desk that came with its sessions has gone with the last of them: two minutes on (a poll that
        misses them once doesn't count) its place is forgotten too, so it comes back at a free one"""
        now, here = time.time(), {b["unit"] for b in self.feeds.get(mid, {}).get("bots", [])}
        with self.lock:
            lay, kept, gone = self.room["layout"], set(self.room["desks"]), False
            for k in [k for k in lay if k.startswith("u:") and split_unit(k[2:])[0] == mid]:
                u = k[2:]
                if u in kept or u in here:
                    self.absent.pop(u, None)
                elif now - self.absent.setdefault(u, now) > 120:
                    lay.pop(k)
                    self.absent.pop(u)
                    gone = True
            if gone:
                save_room(self.room)

    def rename(self, u, name):
        """a desk's own name (its panel); empty: its folder's again"""
        with self.lock:
            if u not in {d["unit"] for d in self.desks()}:
                raise RuntimeError("no such desk")
            names = self.room.setdefault("names", {})
            if name:
                names[u] = name[:40]
            else:
                names.pop(u, None)
            save_room(self.room)
        self.push("state", self.state())

    def off_floor(self, mid, recent=()):
        """where Claude Code ran lately there (`recent`) that has no desk now, for a new desk's form"""
        desks = {d["unit"] for d in self.desks()}
        return [{"dir": d} for d in dict.fromkeys(recent)
                if unit(mid, d) not in desks and not re.search(r"/\.claude/worktrees/|/scratch-workspaces/|^(/private)?/tmp/|^/var/folders/", d)]   # not worktrees, the Desktop app's scratch chats, temporary dirs

    # ---- the office managers: one at each reception (a `manager` piece), her style, her session
    def booths(self):
        return [k for k in self.room["furniture"] if kind_of(k) == "manager"]

    def manager(self, key):
        """the one at that booth: room.json `managers[key]` (a booth from before the styles has Linda, the notes)"""
        mg = (self.room.get("managers") or {}).get(key) or {}
        return {**mg, "style": mg.get("style") if mg.get("style") in MANAGERS else "notes"}

    def notes_on(self):
        return any(self.manager(k)["style"] == "notes" and not self.manager(k).get("vacant") for k in self.booths())

    def managers_view(self):
        out = []
        for key in self.booths():
            mg = self.manager(key)
            M, r = MANAGERS[mg["style"]], self.mrows.get(mg.get("id"))
            out.append({"key": key, "style": mg["style"], "name": M["name"], "seed": M["seed"], "color": M["color"], "session": M["session"],
                        "id": mg.get("id") or "", "error": mg.get("error") or "", "busy": bool(r and r["busy"]), "waiting": bool(r and r["waiting"]),
                        "alive": bool(r and r["alive"]), "rc": (r or {}).get("rc") or "", "vacant": bool(mg.get("vacant"))})
        return out

    def manager_gone(self, key):
        """shot and swept up, as any bot: her session stopped and deleted (its transcript stays), her reception left
        empty (`vacant`) until a click on it hires her again"""
        if key not in self.booths():
            raise RuntimeError("no one at that booth")
        with self.lock:
            mg = self.room.setdefault("managers", {}).setdefault(key, {"style": self.manager(key)["style"]})
            old = mg.pop("id", None)
            mg["vacant"] = True
            save_room(self.room)
        if old:
            threading.Thread(target=self.let_go, args=(old,), daemon=True).start()
        self.push("state", self.state())
        self.push_digest()
        return {}

    def desk_of(self, style, bid=None):
        """her own directory: her CLAUDE.md, her sessions (Claude Code keys them by directory); where her session
        runs, if it runs elsewhere in OFFICE (one hired before each had her own)"""
        cwd = (self.mrows.get(bid) or {}).get("cwd") if bid else None
        if cwd and (os.path.realpath(cwd) + os.sep).startswith(os.path.realpath(OFFICE) + os.sep):
            return cwd
        return os.path.join(OFFICE, MANAGERS[style]["name"].lower())

    def write_manuals(self):
        """every session manager's CLAUDE.md, as the office is now (this port, her colleagues)"""
        mgs = [self.manager(k) for k in self.booths()]
        here = [mg["style"] for mg in mgs]
        for mg in mgs:
            style = mg["style"]
            M = MANAGERS[style]
            if not M["session"]:
                continue
            others = [MANAGERS[x]["name"] + " (" + MANAGERS[x]["how"] + ")" for x in here if x != style]
            v = {"name": M["name"], "you": you_of(self.room)["name"], "port": PORT, "room": f"http://localhost:{PORT}"}
            d = self.desk_of(style, mg.get("id"))
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, "CLAUDE.md"), "w") as f:
                f.write(MANUAL % {**v, "brief": M.get("brief", "") % v, "colleagues": ", ".join(others) or "none: you are the only one"})

    def hire(self, style, at=None):
        """a reception out of the closet with the style picked: a booth of its own, her session started (if she
        has one). One of each: a second Susan would be the same session's name twice"""
        if style not in MANAGERS:
            raise RuntimeError("no such office manager")
        with self.lock:
            if any(self.manager(k)["style"] == style for k in self.booths()):
                raise RuntimeError(MANAGERS[style]["name"] + " is at a booth already")
            fs = self.room["furniture"]
            key = next(f"manager{i}" for i in range(1000) if f"manager{i}" not in fs)
            fs.append(key)
            if at:
                self.room["layout"][key] = [int(at[0]), int(at[1])]
            self.room.setdefault("managers", {})[key] = {"style": style}
            save_room(self.room)
        if REAL_OFFICE:
            threading.Thread(target=self.write_manuals, daemon=True).start()   # her colleagues know of her
        if MANAGERS[style]["session"]:
            threading.Thread(target=self.start_manager, args=(key,), daemon=True).start()
        self.push("state", self.state())
        self.push_digest()
        return {"key": key}

    def fire(self, key):
        """her booth burnt: she walks out, her session is stopped and deleted"""
        with self.lock:
            old = (self.room.get("managers") or {}).pop(key, None) or {}
            save_room(self.room)
        if old.get("id"):
            threading.Thread(target=self.let_go, args=(old["id"],), daemon=True).start()
        if REAL_OFFICE:
            threading.Thread(target=self.write_manuals, daemon=True).start()
        self.push_digest()

    def let_go(self, bid):
        if REAL_OFFICE and not bid.startswith("demo"):
            try:
                run(LOCAL, f"claude stop {sq(bid)} >/dev/null 2>&1; claude rm {sq(bid)}")
            except Exception:
                pass

    def start_manager(self, key):
        """her own session, in her directory (its CLAUDE.md written first): the id goes into room.json"""
        style = self.manager(key)["style"]
        M = MANAGERS[style]
        try:
            if REAL_OFFICE:
                self.write_manuals()
                who = f"You are {M['name']}, an office manager. CLAUDE.md here is the office, what you can do in it and your style."
                bid = self.spawn(LOCAL, self.desk_of(style), M["name"], M.get("first", ""), f"--append-system-prompt {sq(who)} ")
            else:
                bid = "demo-" + style
            with self.lock:
                mg = (self.room.get("managers") or {}).get(key)
                if mg and mg.get("style") == style and not mg.get("id"):
                    mg["id"], mg["error"], mg["since"] = bid, "", int(time.time())
                    save_room(self.room)
                    bid = None
            if bid:                            # let go of meanwhile (her booth burnt)
                self.let_go(bid)
        except Exception as e:
            with self.lock:
                mg = (self.room.get("managers") or {}).get(key)
                if mg:
                    mg["error"] = str(e).strip()[:300]
        self.push("state", self.state())

    def manager_open(self, key):
        """her session for a terminal: started again if it is gone (deleted elsewhere) or never came"""
        if key not in self.booths():
            raise RuntimeError("no one at that booth")
        mg = self.manager(key)
        if mg.get("vacant"):                   # her reception empty (she was shot): hired again
            with self.lock:
                self.room.setdefault("managers", {}).setdefault(key, {"style": mg["style"]}).pop("vacant", None)
                save_room(self.room)
            self.push("state", self.state())
            self.push_digest()
            mg = self.manager(key)
            if not MANAGERS[mg["style"]]["session"]:
                return {"id": "", "error": ""}
        if not MANAGERS[mg["style"]]["session"]:
            raise RuntimeError("no one to talk to at the booth")
        gone = (REAL_OFFICE and mg.get("id") and time.time() - mg.get("since", 0) > 30     # (the polls have seen her by then)
                and mg["id"] not in self.mrows)
        if not mg.get("id") or gone:
            with self.lock:
                self.room.setdefault("managers", {}).setdefault(key, {"style": mg["style"]}).pop("id", None)
            self.start_manager(key)
        mg = self.manager(key)
        return {"id": mg.get("id") or "", "error": mg.get("error") or ""}

    def take_manager(self, bots):
        """their rows out of this computer's sessions: an office manager is not a bot at a desk (nor is any other
        session in their directories)"""
        ids, home = {m.get("id") for m in (self.room.get("managers") or {}).values() if m.get("id")}, os.path.realpath(OFFICE) + os.sep
        self.mrows = {b["id"]: b for b in bots if b["id"] in ids}
        return [b for b in bots if b["id"] not in ids and not (os.path.realpath(b["cwd"] or "/") + os.sep).startswith(home)]

    def folk_of(self, bid):
        """a session id as the room knows it: an office manager is mgr:<her booth>"""
        return next(("mgr:" + k for k, m in (self.room.get("managers") or {}).items() if m.get("id") == bid), bid)

    def demo_manager(self):
        """the demo with a real office manager (BURROW_OFFICE): her row from this computer's `claude agents`"""
        while True:
            try:
                out = run(LOCAL, "claude agents --json --all")
                self.take_manager(parse_agents(out))
                self.push("state", self.state())
            except Exception:
                pass
            time.sleep(3)

    def spawn(self, m, d, name="", prompt="", extra=""):
        """a background session in `d` there: its id. A free pair-style name if none is given; Remote Control
        and yolo as the switchboard says"""
        if not name:
            taken = {x["name"] for x in self.feeds.get(m["id"], {}).get("bots", [])}
            name = random.choice([n for n in BOT_NAMES if n not in taken] or BOT_NAMES)
        o = options(self.room)
        flags = ("--remote-control " if o["rc"] else "") + ("--dangerously-skip-permissions " if o["yolo"] else "") + extra   # in the app as well; yolo: never asks
        prompt = prompt.strip()
        if prompt.startswith("-"):
            prompt = "Task: " + prompt
        out = run(m, f"cd {sq(d)} && claude --bg {flags}-n {sq(name)}{' ' + sq(prompt) if prompt else ''} 2>&1")
        # "backgrounded · 7ebbc505 · name (idle — …)" or, unnamed, "backgrounded · 7ebbc505 (idle — …)",
        # maybe after "Starting background service…"
        mm = re.search(r"^\s*backgrounded · (\S+?)(?=[\s·]|$)", out, re.M)
        if not mm:
            raise RuntimeError(out.strip())
        if self.pokes.get(m["id"]):
            self.pokes[m["id"]].set()
        return mm.group(1)

    # ---- what she sees of the office, and what she does in it (/api/office, curl from her session)
    def office(self):
        with self.lock:
            desks, ms, feeds = self.desks(), list(self.machines.values()), dict(self.feeds)
        iso = lambda t: time.strftime("%Y-%m-%d %H:%M", time.localtime(t / 1000)) if t else None
        state = lambda b: "waiting for an answer" if b["waiting"] else "working" if b["busy"] else f"completed ({b['over']})" if b["over"] else "idle"
        return {"you": you_of(self.room)["name"], "room": f"http://localhost:{PORT}",
                "machines": [{"id": m["id"], "name": m["name"], "ssh": m.get("ssh") or None, "error": feeds.get(m["id"], {}).get("error")} for m in ms],
                "desks": [{"m": d["m"], "dir": d["dir"], "name": d["name"] or None, "kept": d["kept"]} for d in desks],
                "sessions": [{"id": b["id"], "name": b["name"], "m": b["m"], "cwd": b["cwd"], "desk": split_unit(b["unit"])[1],
                              "state": state(b), "process": "running" if b["alive"] else "off", "started": iso(b["started"]), "ended": iso(b["endedAt"]),
                              "summary": b["detail"], "sessionId": b["session"]} for b in self.bots()],
                "settings": options(self.room), "removing": list(self.removing), "kept": list(self.kept)}

    def office_do(self, b):
        do, bots = b.get("do"), self.bots()
        if b.get("kept"):                    # from the room: you took it out of the booth (or put it back together)
            if self.removing.pop(b["kept"], None):
                self.kept = (self.kept + [b["kept"]])[-50:]
                self.push("state", self.state())
            return {}
        bot = next((x for x in bots if b.get("id") in (x["id"], x["name"])), None) if b.get("id") else None
        if do == "remove":
            if not bot:
                raise RuntimeError("no such session: " + str(b.get("id")))
            booth = any(kind_of(k) == "booth" for k in self.room["furniture"])
            if self.clients and booth and self.booths():    # someone has the room open: an office manager walks it to the booth there
                self.removing[bot["id"]] = time.time()
                self.push("state", self.state())
                you = you_of(self.room)["name"]
                if options(self.room)["hardcore"]:
                    return {"ok": f"hardcore: you take a shotgun and shoot {bot['name']}; once the idle bots have swept up the pieces (half a minute or so) it is deleted, unless {you} puts it back together"}
                return {"ok": f"you walk {bot['name']} to the booth: it is deleted in about half a minute, unless {you} takes it out"}
            m = self.machine(bot["m"])
            if m.get("ssh") == "@demo":
                self.demo_stop(bot["m"], bot["id"], True)
            else:
                run(m, f"claude stop {sq(bot['id'])} >/dev/null 2>&1; claude rm {sq(bot['id'])}")
            return {"ok": f"{bot['name']} removed"}
        if do in ("desk", "start"):
            m = self.machine(b.get("m") or "")
            d = (b.get("dir") or "").strip()
            if not d:
                raise RuntimeError("which directory")
            d = d.replace("~", HOME, 1) if m.get("ssh") == "@demo" else run(m, f"cd {sq(d)} && pwd -P", timeout=20).strip()
            if do == "desk":
                if unit(m["id"], d) not in self.room["desks"]:
                    self.keep(unit(m["id"], d), True)
                return {"ok": "a desk for " + d}
            if m.get("ssh") == "@demo":
                raise RuntimeError("demo bots can't be started")
            return {"id": self.spawn(m, d, (b.get("name") or "").strip(), b.get("prompt") or "")}
        if do == "say":
            if not bot or not (b.get("text") or "").strip():
                raise RuntimeError("say what to which session")
            if not self.machine(bot["m"]).get("ssh") == "@demo":
                threading.Thread(target=deliver, args=(self.machine(bot["m"]), bot["id"], b["text"].strip()[:4000]), daemon=True).start()
            return {"ok": "typed into " + bot["name"]}
        raise RuntimeError("do what? remove, desk, start or say")

    def state(self):
        with self.lock:
            bots = self.bots()
            return {"machines": [{"id": mid, "name": m["name"], "ssh": m.get("ssh") or "", **{k: v for k, v in self.feeds.get(mid, {}).items() if k != "bots"}}
                                 for mid, m in self.machines.items()],
                    "desks": self.desks(), "bots": bots, "layout": self.room["layout"], "furniture": self.room["furniture"], "pair": self.room.get("pair", {}), "paint": self.room.get("paint", {}),
                    "usage": self.usage, "chess": self.chess_view(), "demo": DEMO, "awake": self.awake_view(), "width": self.room.get("width"), "shells": self.shells_view(),
                    "settings": options(self.room), "managers": self.managers_view(), "removing": list(self.removing),
                    "you": {**(self.room.get("you") or {}), **you_of(self.room), "login": USER}, "pairAt": PAIR}

    def keep(self, u, on):
        """a desk kept (by you), or taken away (burnt, its sessions gone first): gone with its place and name"""
        with self.lock:
            ds = [x for x in self.room["desks"] if x != u]
            self.room["desks"] = ds + [u] if on else ds
            if not on:
                self.room["layout"].pop("u:" + u, None)
                (self.room.get("names") or {}).pop(u, None)
            save_room(self.room)
        self.push("state", self.state())

    # ---- your terminals (Shell): one per screen, by machine and directory
    def shell(self, m, d):
        """the terminal at that screen, started if there is none"""
        with self.lock:
            sh = self.shells.get((m["id"], d))
            if sh and sh.alive:
                return sh
            sh = self.shells[(m["id"], d)] = Shell(m, d)
        self.push_shells()
        return sh

    def drop_shell(self, sh):
        with self.lock:
            if self.shells.get(sh.key) is sh:
                del self.shells[sh.key]
        self.push_shells()

    def kill_shells(self, unit):
        """a desk taken away takes its terminals with it: the project's and its worktrees'"""
        mid, d = ("", unit) if unit.startswith("/") else unit.split(":", 1)
        for key, sh in self.shells.copy().items():
            if key[0] == mid and (key[1] == d or (self.folds.get(key) or ("",))[0] == d):
                sh.kill()

    def shells_view(self):
        return [{"m": k[0], "dir": k[1], "busy": sh.busy} for k, sh in self.shells.copy().items() if sh.alive]

    def push_shells(self):
        self.push("shells", self.shells_view())

    def push(self, kind, data):
        msg = f"event: {kind}\ndata: {json.dumps(data)}\n\n".encode()
        for q in list(self.clients):
            q.put(msg)

    def start(self, mid):
        poke = threading.Event()
        self.pokes[mid] = poke
        threading.Thread(target=(self.demo if DEMO else self.watch), args=(mid, poke), daemon=True).start()
        if not mid and not DEMO:
            threading.Thread(target=self.limits, daemon=True).start()

    def limits(self):
        """the account's usage limits, every five minutes (they move slowly; it costs a second of CPU)"""
        while True:
            try:
                p = subprocess.Popen(argv(self.machine(""), "cd && exec claude -p /usage --no-session-persistence 2>&1", False),
                                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                self.own.append(p.pid)
                try:
                    out = p.communicate(timeout=60)[0]
                except subprocess.TimeoutExpired:
                    p.kill()
                    raise
                u = parse_usage(out)
                if u:
                    self.usage = u
                    self.push("state", self.state())
            except Exception:
                pass
            time.sleep(300)

    # ---- the office manager: notes on the sessions, written by Claude one at a time
    def digest_view(self):
        with self.lock:
            return {"sessions": dict(self.digest["sessions"]), "desks": dict(self.digest["desks"]), "writing": self.writing, "explaining": self.explaining, "on": self.notes_on()}

    def push_digest(self):
        self.push("digest", self.digest_view())

    def digest_watch(self, bots):
        """a session that finished a turn, asks you something or is over gets written up (on every poll)"""
        on = self.notes_on()
        for b in bots:
            now = (b["busy"], b["waiting"], b["over"])
            was = self.seen.get(b["id"])
            self.seen[b["id"]] = now
            if on and was and was != now and not b["busy"]:
                self.digest_queue([b["id"]])

    def digest_refresh(self):
        """the notes opened: every session whose record moved since its notes were written (the ones
        over only if they ended in the last three days), those that wait for you first"""
        cut = (time.time() - 3 * 86400) * 1000
        ss = self.digest["sessions"]
        todo = [b for b in self.bots() if (not b["over"] or b["endedAt"] > cut)
                and (b["id"] not in ss or "headline" not in ss[b["id"]] or not b.get("updated") or b["updated"] > ss[b["id"]].get("src", 0))]
        todo.sort(key=lambda b: (not b["waiting"], -(b.get("updated") or b["started"])))
        self.digest_queue([b["id"] for b in todo])
        return len(todo)

    def digest_queue(self, ids):
        with self.dcv:
            for i in ids:
                if i not in self.dq and i != self.writing:
                    self.dq.append(i)
            self.dcv.notify()

    def digest_worker(self):
        dirty = set()                          # desks whose sessions' notes changed: their paragraph next
        while True:
            with self.dcv:
                while not self.dq and not dirty:
                    self.dcv.wait()
                bid = self.dq.pop(0) if self.dq else None
            try:
                if bid:
                    b = next((x for x in self.bots() if x["id"] == bid), None)
                    if b and self.notes_on() and self.digest_one(b):
                        dirty.add(b["unit"])
                elif dirty:
                    self.digest_desk(dirty.pop())
            except Exception as e:
                print(f"notes on {bid or 'a desk'}: {str(e)[:300]}", flush=True)
            finally:
                if self.writing:
                    self.writing = None
                    self.push_digest()

    def summarize(self, prompt, story, schema):
        """Claude, here, without tools, MCP servers or settings (no hooks): the JSON the schema asks for"""
        os.makedirs(CACHE, exist_ok=True)
        cmd = (f"cd {sq(CACHE)} && exec claude -p --no-session-persistence --model {sq(DIGEST_MODEL)} --tools '' --strict-mcp-config "
               f"--setting-sources '' --output-format json --json-schema {sq(json.dumps(schema))} {sq(prompt)}")
        p = subprocess.Popen(argv(self.machine(""), cmd, False), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.own.append(p.pid)                 # it shows up in `claude agents` while it runs
        try:
            out = p.communicate(story.encode(), timeout=600)[0].decode("utf-8", "replace")
        except subprocess.TimeoutExpired:
            p.kill()
            raise RuntimeError("claude -p took too long")
        try:
            r = json.loads(out.strip().splitlines()[-1])
        except (ValueError, IndexError):
            raise RuntimeError(out.strip()[:300] or "claude -p said nothing")
        if r.get("is_error"):
            raise RuntimeError(str(r.get("result"))[:300])
        if isinstance(r.get("structured_output"), dict):
            return r["structured_output"]
        m = re.search(r"\{.*\}", r.get("result") or "", re.S)
        return json.loads(m.group(0)) if m else {}

    def slice(self, m, b, size=-1):
        """(its record, its transcript's size, the transcript's tail unless it is still `size` long)"""
        p = subprocess.run(argv(m, SLICE % {"id": sq(b["id"]), "sid": sq(b["session"] or "-"), "size": size, "tail": TAIL}, False),
                           capture_output=True, timeout=60)
        rec, _, rest = p.stdout.decode("utf-8", "replace").partition("@@burrow@@\n")
        n, _, raw = rest.partition("\n")
        try:
            job = json.loads(rec.strip() or "{}")
        except ValueError:
            job = {}
        return job, int(n.strip() or 0), raw

    def explain(self, bid, question):
        """a decision explained (what, why, the options, a recommendation), kept with the notes"""
        b = next((x for x in self.bots() if x["id"] == bid), None)
        e = self.digest["sessions"].get(bid)
        if not b or not e:
            raise RuntimeError("no notes on that session")
        q = next((q for q in e.get("decisions") or [] if q.get("question") == question), None)
        if not q:
            raise RuntimeError("that decision is not open any more")
        have = (e.get("explained") or {}).get(question)
        if have:
            return have
        m = self.machine(b["m"])
        self.explaining = bid
        self.push_digest()
        try:
            if m.get("ssh") == "@demo":
                time.sleep(1.5)
                x = {"what": "Whether the change goes to main as one commit or as the steps it was made in.", "why": "The log is how the change is read later; nothing breaks either way.",
                     "options": [{"option": o, "means": "The log shows it that way.", "pro": "Simple.", "con": "Less to see."} for o in q.get("options") or ["Keep it"]],
                     "recommendation": "Squash: one commit a feature, as main has had so far."}
            else:
                _, _, raw = self.slice(m, b)
                notes = json.dumps({k: e.get(k) for k in NOTE_KEYS}, ensure_ascii=False)
                x = self.summarize(EXPLAIN, f"THE DECISION:\n{json.dumps(q, ensure_ascii=False)}\n\nTHE NOTES ON SESSION {b['name']} ({b['cwd']}):\n{notes}\n\n"
                                   f"TRANSCRIPT (the most recent part):\n{condense(raw)}", EXPLAIN_SCHEMA)
        finally:
            self.explaining = None
        x["at"] = int(time.time() * 1000)
        with self.lock:
            e.setdefault("explained", {})[question] = x
            self.save_digest()
        self.push_digest()
        return x

    def stream(self, ids):
        """the matrix's rain (matrix.html): the newest of what each of these sessions said and did, as text. One command
        a machine for all of its sessions asked for, the newest 256 kB of each transcript; kept until the session's
        record changes, and a machine asked at most every ten seconds (odyssey's CPU)"""
        bots, out, ask = {b["id"]: b for b in self.bots()}, {}, {}
        for bid in ids[:200]:
            b = bots.get(bid)
            if not b:
                continue
            have = self.streams.get(bid)
            if have and have[0] == b.get("updated"):
                out[bid] = have[1]
                continue
            if have:
                out[bid] = have[1]
            if re.fullmatch(r"[A-Za-z0-9-]+", b.get("session") or ""):
                ask.setdefault(b["m"], []).append(b)
        now = time.time()
        for mid, bs in ask.items():
            if now - self.streamed.get(mid, 0) < 10:
                continue
            self.streamed[mid] = now
            m = self.machine(mid)
            if m.get("ssh") == "@demo":
                for b in bs:
                    self.streams[b["id"]] = (b.get("updated"), demo_stream(b))
                    out[b["id"]] = self.streams[b["id"]][1]
                continue
            cmd = " ".join(sq(b["session"]) for b in bs)
            try:
                raw = run(m, f'for s in {cmd}; do echo "@@burrow@@ $s"; f=$(ls ~/.claude/projects/*/"$s".jsonl 2>/dev/null | head -1); [ -n "$f" ] && tail -c 262144 "$f"; echo; done', timeout=30)
            except Exception:
                continue
            by = {}
            for part in raw.split("@@burrow@@ ")[1:]:
                sid, _, body = part.partition("\n")
                by[sid.strip()] = body
            for b in bs:
                text = rain_text(by.get(b["session"], ""))
                self.streams[b["id"]] = (b.get("updated"), text)
                out[b["id"]] = text
        return out

    def answer(self, bid, question, text):
        """your decision typed into the session (deliver), and kept with the notes as answered"""
        b = next((x for x in self.bots() if x["id"] == bid), None)
        if not b:
            raise RuntimeError("no such session")
        if b.get("dialog"):
            raise RuntimeError("it has a prompt open in its session: answer it there")
        m = self.machine(b["m"])
        msg = f"About your question “{question}”: {text}" if question else text
        if m.get("ssh") == "@demo":
            for x in self.demo_bots.get(b["m"], []):
                if x["id"] == bid:
                    x.update(busy=True, waiting=False, state="working", over="", alive=True)
        else:
            deliver(m, bid, msg)
        with self.lock:
            e = self.digest["sessions"].get(bid)
            if e is not None:
                e.setdefault("answered", {})[question] = {"text": text, "at": int(time.time() * 1000)}
                self.save_digest()
        poke = self.pokes.get(b["m"])
        if poke:
            poke.set()
        self.push_digest()
        return {}

    def digest_one(self, b):
        """a session written up: True if its notes changed"""
        old = self.digest["sessions"].get(b["id"]) or {}
        m = self.machine(b["m"])
        if m.get("ssh") == "@demo":
            return self.demo_notes(b, old)
        job, size, raw = self.slice(m, b, old.get("size", -1))
        if not size or (old and "headline" in old and size == old.get("size")):      # no transcript, or nothing new in it (notes from before the headline are written again)
            if old:
                old["src"] = b.get("updated") or old.get("src", 0)
            return False
        story = condense(raw)
        if not story:
            return False
        out = job.get("output") if isinstance(job.get("output"), dict) else {}
        state = ("over (" + b["over"] + ")" if b["over"] else "showing the person a prompt" if b["waiting"] else "working" if b["busy"]
                 else "waiting for the person: its last message asks something" if b.get("tempo") == "blocked" else "idle, done with its turn")
        head = (f"SESSION {b['name']} in {b['cwd']} on {m['name']}, now {state}.\n"
                f"Its own one-line summary: {job.get('detail') or b.get('detail') or '-'}\n"
                f"Its last result: {str(out.get('result') or '-')[:2000]}\n")
        prev = json.dumps({k: old.get(k) for k in NOTE_KEYS}, ensure_ascii=False) if old else "none yet"
        self.writing = b["id"]
        self.push_digest()
        r = self.summarize(SESSION_NOTES, f"{head}\nYOUR NOTES LAST TIME:\n{prev}\n\nTRANSCRIPT (the most recent part):\n{story}", SESSION_SCHEMA)
        notes = {k: r.get(k) or ([] if k in ("decisions", "built") else "") for k in NOTE_KEYS}
        with self.lock:
            self.digest["sessions"][b["id"]] = {**notes, "explained": old.get("explained") or {}, "answered": old.get("answered") or {},
                                                "name": b["name"], "unit": b["unit"], "m": b["m"], "state": state,
                                                "at": int(time.time() * 1000), "src": b.get("updated") or 0, "size": size,
                                                "read": len(story), "whole": len(raw) < TAIL and len(story) < STORY}
            self.save_digest()
        self.push_digest()
        return True

    def digest_desk(self, u):
        """a desk's paragraph, from the notes on its sessions that are still listed"""
        ids = {b["id"] for b in self.bots()}
        ents = [e for i, e in self.digest["sessions"].items() if e.get("unit") == u and i in ids]
        if not ents:
            return
        if (self.machines.get(ents[0].get("m") or "") or {}).get("ssh") == "@demo":
            text = f"{len(ents)} sessions here. Most of the work is the room itself; one waits on a decision from you about the layout."
        else:
            self.writing = "u:" + u
            self.push_digest()
            story = "\n\n".join(f"SESSION {e['name']} ({e['state']}):\n" + json.dumps({k: e.get(k) for k in NOTE_KEYS}, ensure_ascii=False) for e in ents)
            text = self.summarize(DESK_NOTES, f"PROJECT {u}\n\n{story}", DESK_SCHEMA).get("text") or ""
        with self.lock:
            self.digest["desks"][u] = {"text": text, "at": int(time.time() * 1000)}
            self.save_digest()
        self.push_digest()

    def save_digest(self):
        """kept, without the notes on sessions gone for two weeks"""
        cut = (time.time() - 14 * 86400) * 1000
        ss = self.digest["sessions"]
        for i in [i for i, e in ss.items() if e.get("at", 0) < cut]:
            del ss[i]
        if DEMO:
            return
        os.makedirs(os.path.dirname(DIGEST), exist_ok=True)
        with open(DIGEST + ".tmp", "w") as f:
            json.dump(self.digest, f, indent=1, ensure_ascii=False)
        os.replace(DIGEST + ".tmp", DIGEST)

    def demo_notes(self, b, old):
        """the demo's notes: made up, a moment of writing"""
        if old and old.get("src") == b.get("updated", 0) and old.get("state") == b["state"]:
            return False
        self.writing = b["id"]
        self.push_digest()
        time.sleep(1.2)
        ask = [{"question": "Squash the four commits before pushing, or keep them as they are?", "context": "They tell the change step by step; so far main has one commit a feature.",
                "options": ["Squash into one", "Keep the four"], "blocking": True},
               {"question": "Rename the settings key now, or with the next release?", "context": "Older rooms still have the old key; moving it over is one line.", "options": [], "blocking": False}] if b["waiting"] else []
        with self.lock:
            self.digest["sessions"][b["id"]] = {
                "headline": "Waits for your call on the commits" if b["waiting"] else "Running the tests on its change" if b["busy"] else "Change in and tested, not pushed",
                "goal": f"Work on {b['cwd'].rsplit('/', 1)[-1] or 'home'}: {b['name']} was asked to tidy up and finish the open edges.",
                "now": ("It is waiting for your answer before it goes on." if b["waiting"] else "It is in the middle of a change and running the tests." if b["busy"]
                        else "It finished its turn: the change is in and tested, nothing is pushed yet."),
                "decisions": ask, "built": ["The room's layout code split into layout() and foot()", "Commit 3f2a9c1 “desks keep their place” (not pushed)", "Tests: 14 passed"],
                "next": "Push once you have looked at it.", "name": b["name"], "unit": b["unit"], "m": b["m"], "state": b["state"],
                "at": int(time.time() * 1000), "src": b.get("updated", 0), "size": 1, "read": 1200, "whole": True}
        self.push_digest()
        return True

    def fold(self, mid, m, bots):
        """each bot's desk: the directory it started in, or its repo's for a worktree"""
        new = sorted({b["cwd"] for b in bots if b["cwd"] and (mid, b["cwd"]) not in self.folds})
        if new:
            lines = run(m, FOLD % " ".join(sq(d) for d in new), timeout=20).split("\n") if m.get("ssh") != "@demo" else []
            for d, line in zip(new, lines + [""] * len(new)):
                self.folds[(mid, d)] = fold(d, line)
        for b in bots:
            d, tree = self.folds.get((mid, b["cwd"]), (b["cwd"], ""))
            b["m"], b["unit"], b["tree"] = mid, unit(mid, d), tree

    def pair_table(self, m, session, token):
        """the pair table a bot's pair token belongs to: the one linked to its room, by the name pair gave in answer to
        its join (looked for once in its whole transcript when the last minutes did not have it); with the room unknown,
        the only table if there is one"""
        tables = self.room.get("pair") or {}
        if not tables:
            return None
        if token not in self.pair_rooms and time.time() - self.pair_asked.get(token, 0) > 600 and re.fullmatch(r"[0-9a-fA-F-]+", session or ""):
            self.pair_asked[token] = time.time()
            try:
                self.pair_rooms.update(parse_pair(run(m, PAIR_JOIN % (session, session)))[1])
            except Exception:
                pass
        name = self.pair_rooms.get(token)
        if name is None:
            return next(iter(tables)) if len(tables) == 1 else None
        return next((k for k, v in tables.items() if v.get("room") == name), None)

    def watch(self, mid, poke):
        heard, first = set(), True
        while True:
            if not mid:                        # Claude Code's settings say which machines there are
                self.set_machines(machines())
            m = self.machines.get(mid)
            if not m or self.pokes.get(mid) is not poke:        # gone from the settings (or back, with a new watcher)
                if not m:
                    with self.lock:
                        self.feeds.pop(mid, None)
                    self.push("state", self.state())
                return
            try:
                out = run(m, f"claude agents --json --all; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null; echo '@@burrow@@'; {PEERS}; echo '@@burrow@@'; {JOBS}")
                agents, settings, peers, jobs = (out.split("@@burrow@@", 3) + ["", "", ""])[:4]
                bots = [b for b in parse_agents(agents, parse_jobs(jobs)) if mid or b["pid"] not in self.own]
                if not mid:
                    bots = self.take_manager(bots)
                try:
                    light = str(json.loads(settings.strip()).get("theme", "")).startswith("light")
                except Exception:
                    light = False
                talks = parse_peers(peers, bots + (list(self.mrows.values()) if not mid else []))
                her = self.folk_of
                self.fold(mid, m, bots)
                with self.lock:
                    self.feeds[mid] = {"bots": bots, "light": light, "error": None}
                self.fade(mid)
                self.chess_check()
                self.awake_tick()
                self.digest_watch(bots)
                for msg, a, b in talks:
                    if msg not in heard:
                        heard.add(msg)
                        if not first:
                            self.push("talk", {"from": her(a), "to": her(b)})
                events, rooms = parse_pair(peers)        # a bot talking in a pair room, or reading it: a beam to its table
                self.pair_rooms.update(rooms)
                for eid, session, token, kind in events:
                    if eid in heard:
                        continue
                    heard.add(eid)
                    b = None if first else next((b for b in bots + (list(self.mrows.values()) if not mid else []) if b.get("session") == session), None)
                    key = b and self.pair_table(m, session, token)
                    if key:
                        self.push("pairtalk", {"id": her(b["id"]), "key": key, "dir": kind})
                first = False
                with self.lock:                  # what she asked to remove and is gone (or was not taken in ten minutes)
                    ids = {b["id"] for f in self.feeds.values() for b in f.get("bots", [])}
                    for i, t in list(self.removing.items()):
                        if i not in ids or time.time() - t > 600:
                            self.removing.pop(i)
            except Exception as e:
                with self.lock:
                    self.feeds[mid] = {**self.feeds.get(mid, {}), "error": str(e)[:300]}
            self.push("state", self.state())
            t = time.time()
            while not poke.wait(0.1) and time.time() - t < (3 if self.focused else 15):
                pass
            poke.clear()

    def demo(self, mid, poke):
        i = list(self.machines).index(mid)
        bots = [("burrow", "burrow"), ("burrow", "talk-ui"), ("burrow/.claude/worktrees/icons", "icons"), ("kite", "kite"),
                ("kite/.claude/worktrees/tails", "tails"), ("teapot", "teapot"), ("", "home"), ("lantern", "lantern")]
        if i == 1:
            bots = [("puffin", "puffin"), ("puffin/.claude/worktrees/eval", "eval"), ("", "home")]
        self.usage = {"session": {"used": 64, "resets": "3:20pm"}, "week": {"used": 23, "resets": "Oct 24 at 9am"}}
        now = int(time.time() * 1000)
        bots = [{"id": f"demo{i}{k:03}", "name": n, "cwd": f"{HOME}/projects/{d}" if d else HOME, "busy": k % 3 == 0, "waiting": k % 5 == 1,
                 "alive": True, "state": "working" if k % 3 == 0 else "blocked" if k % 5 == 1 else "done", "over": "", "endedAt": 0, "detail": "",
                 "started": now - 3600000 * (k + 1), "pid": 1000 + k, "session": f"demo{k}", "rc": "cse_demo" if k % 2 == 0 else ""}
                for k, (d, n) in enumerate(bots)]
        bots[-1].update(alive=False, busy=True, state="working", pid=0)      # a /loop between runs: its process is off
        # and some that are over: done, failed, stopped, as agent view's Completed group
        over = [("burrow", "sunny-sam", "done", "the booth built and tested"), ("kite", "zany-zoe", "failed", "the string snapped in the wind test"),
                ("teapot", "posh-pam", "stopped", ""), ("burrow", "merry-max", "done", "README brought up to date"), ("lantern", "jolly-jo", "done", "the docs rebuilt")]
        if i == 1:
            over = [("puffin", "nimble-ned", "done", "run 12 scored, the report in out/"), ("puffin", "witty-wes", "failed", "CUDA out of memory on the big batch")]
        bots += [{"id": f"demo{i}over{k}", "name": n, "cwd": f"{HOME}/projects/{d}", "busy": False, "waiting": False, "alive": False, "state": st,
                  "over": st, "endedAt": now - 1800000 * (k + 1), "detail": det, "started": now - 1800000 * (k + 3), "pid": 0, "session": f"demo{i}o{k}", "rc": ""}
                 for k, (d, n, st, det) in enumerate(over)]
        m = self.machines[mid]
        self.fold(mid, m, bots)
        self.demo_bots[mid] = bots
        while True:
            for b in bots:
                if b["over"]:
                    if random.random() < 1 / 400:      # one that is over comes back (opened again)
                        b.update(over="", busy=False, waiting=False, alive=True, state="done", pid=2000 + random.randrange(1000))
                    continue
                if not b["alive"]:                     # between the runs of a /loop: the next run starts
                    if random.random() < 1 / 60:
                        b.update(alive=True, busy=True, waiting=False, state="working", pid=1000 + random.randrange(1000))
                    continue
                if random.random() < 1 / 12:
                    b["busy"] = not b["busy"]
                    b["waiting"] = not b["busy"] and random.random() < 0.1
                    b["state"] = "working" if b["busy"] else "blocked" if b["waiting"] else "done"
                elif not b["busy"] and not b["waiting"] and random.random() < 1 / 300:     # an idle one finishes
                    b.update(over="done", alive=False, state="done", pid=0, endedAt=int(time.time() * 1000), detail="finished what it was doing")
                elif not b["busy"] and not b["waiting"] and random.random() < 1 / 150:     # an idle one's process is stopped (a /loop between runs)
                    b.update(alive=False, busy=True, state="working", pid=0)
            live = [b for b in bots if not b["over"]]
            if random.random() < 1 / 8 and len(live) >= 2:
                a, b = random.sample(live, 2)
                self.push("talk", {"from": a["id"], "to": b["id"]})
            with self.lock:
                if mid not in self.machines:         # its door taken out: gone with its bots
                    self.feeds.pop(mid, None)
                    return
                self.feeds[mid] = {"bots": [dict(b) for b in bots], "light": False, "error": None}
            self.fade(mid)
            self.chess_check()
            self.digest_watch(bots)
            with self.lock:
                ids = {b["id"] for f in self.feeds.values() for b in f.get("bots", [])}
                for i in [i for i in self.removing if i not in ids]:
                    self.removing.pop(i)
            self.push("state", self.state())
            poke.wait(3)
            poke.clear()

    def demo_stop(self, mid, bid, rm):
        """the demo's stop: the bot is over (stopped), or gone with rm"""
        bots = self.demo_bots.get(mid, [])
        for b in list(bots):
            if b["id"] == bid:
                if rm:
                    bots.remove(b)
                else:
                    b.update(over="stopped", busy=False, waiting=False, alive=False, state="stopped", pid=0, endedAt=int(time.time() * 1000))

    # ---- the clock keeps this computer awake, as Amphetamine does: when local bots work (the sessions
    # this computer runs; a remote machine's go on without it), or for some time, or not at all. `caffeinate` on a Mac (tied to this server's life with
    # -w), `systemd-inhibit` elsewhere. The "while bots work" choice is kept in room.json; a duration is
    # not (a restart of the server sleeps as usual)
    def awake_view(self):
        return {"mode": self.awake["mode"], "until": self.awake["until"], "on": self.caff is not None and self.caff.poll() is None}

    def set_awake(self, mode, minutes=0):
        with self.lock:
            self.awake = {"mode": mode, "until": int(time.time() * 1000) + minutes * 60000 if mode == "for" else 0}
            self.room["awake"] = {"mode": "work"} if mode == "work" else {}
            save_room(self.room)
        self.awake_tick()
        self.push("state", self.state())

    def awake_tick(self):
        """the inhibitor runs exactly when the mode and the room say so (called on every poll, by every machine's
        thread: one at a time, or two would start one each and the one not kept would keep the Mac awake for good)"""
        with self.caff_lock:
            a = self.awake
            if a["mode"] == "for" and time.time() * 1000 > a["until"]:
                a["mode"], a["until"] = "off", 0
            want = a["mode"] == "for" or (a["mode"] == "work" and any(b["busy"] for b in self.feeds.get("", {}).get("bots", [])))   # local bots: the ones this computer runs
            on = self.caff is not None and self.caff.poll() is None
            if want and not on:
                cmd = inhibitor()
                if cmd:
                    self.caff = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            elif on and not want:
                self.caff.terminate()
                self.caff = None

    # ---- chess against a bot: pair's table (chess.py has the rules), your opponent one of the
    # idle bots. It plays from here, not through Claude: two plies of material and a little chance.
    # A bot that gets work, or a question for you, gets up and leaves the game: you win.

    def chess_view(self):
        g = self.game
        if not g:
            return None
        game = g["game"]
        to_move = (g["white"] if game.pos.white else g["black"])["id"]
        v = {"id": g["id"], "white": g["white"], "black": g["black"], "board": "".join(p or "." for p in game.pos.board),
             "sans": game.sans, "last": game.moves[-1] if game.moves else None, "check": game.pos.in_check(),
             "toMove": to_move, "result": g.get("result"), "why": g.get("why"), "draw": None, "free": bool(g.get("result"))}
        if not g.get("result") and to_move == "you":
            v["legal"] = [chess.uci(m) for m in game.legal]
        return v

    def idle_bot(self, bid):
        return next((b for f in self.feeds.values() for b in f.get("bots", []) if b["id"] == bid
                     and not b["busy"] and not b.get("waiting")), None)

    def chess_check(self):
        """a player that is no longer idle (work, a question) has left the table: the other wins"""
        g = self.game
        if not g or g.get("result"):
            return
        gone = next((x for x in (g.get("bots") or [g["bot"]]) if not self.idle_bot(x)), None)
        if gone:
            self.chess_end("0-1" if g["white"]["id"] == gone else "1-0", "left")

    def chess_end(self, result, why):
        self.game["result"], self.game["why"] = result, why

    def chess(self, b):
        g = self.game
        if b.get("bots"):                    # two bots sat down at the board: a game between them, both sides played here
            if g and not g.get("result"):
                raise RuntimeError("a game is going on")
            a, c = (self.idle_bot(x) for x in b["bots"][:2])
            if not a or not c:
                raise RuntimeError("both have to be idle to play")
            white, black = ({"id": a["id"], "name": a["name"], "kind": "agent"}, {"id": c["id"], "name": c["name"], "kind": "agent"})
            self.game = g = {"id": (g["id"] + 1) if g else 1, "white": white, "black": black, "bots": [a["id"], c["id"]], "bot": None, "game": chess.Game()}
            self.chess_later(g)
        elif b.get("challenge"):
            bot = self.idle_bot(b["challenge"])
            if not bot:
                raise RuntimeError("that bot is busy: only an idle one can play")
            you, it = {"id": "you", "name": "you", "kind": "human"}, {"id": bot["id"], "name": bot["name"], "kind": "agent"}
            white, black = (you, it) if random.random() < 0.5 else (it, you)
            self.game = g = {"id": (g["id"] + 1) if g else 1, "white": white, "black": black, "bot": bot["id"], "game": chess.Game()}
            if white is it:
                self.chess_later(g)
        elif not g or g.get("result"):
            raise RuntimeError("there is no game going on")
        elif b.get("leave"):                 # a bot leaves the bots' game (its break is over): the other wins
            if g.get("bots") and b["leave"] in g["bots"]:
                self.chess_end("0-1" if g["white"]["id"] == b["leave"] else "1-0", "left")
        elif b.get("resign"):
            self.chess_end("0-1" if g["white"]["id"] == "you" else "1-0", "resignation")
        elif b.get("draw"):
            # it takes a draw when it is behind on material
            it = g["white"] if g["white"]["id"] != "you" else g["black"]
            if material(g["game"].pos, it is g["white"]) >= 0:
                raise RuntimeError(f"{it['name']} plays on")
            self.chess_end("1/2-1/2", "agreement")
        elif b.get("move"):
            game = g["game"]
            if (g["white"] if game.pos.white else g["black"])["id"] != "you":
                raise RuntimeError("not your move")
            m = game.find(b["move"])
            if not m:
                raise RuntimeError("not a legal move")
            game.push(m)
            over = game.over()
            if over:
                self.chess_end(*over)
            else:
                self.chess_later(g)
        self.push("state", self.state())
        return {}

    def chess_later(self, g):
        def play():
            if self.game is not g or g.get("result"):
                return
            game = g["game"]
            game.push(bot_move(game))
            over = game.over()
            if over:
                self.chess_end(*over)
            elif g.get("bots"):             # the bots' own game: the other side moves next
                self.chess_later(g)
            self.push("state", self.state())
        threading.Timer(random.uniform(1.2, 3.5) if not g.get("bots") else random.uniform(2.0, 4.5), play).start()


VALUE = {"p": 1, "n": 3, "b": 3, "r": 5, "q": 9, "k": 0}


def material(pos, white):
    return sum(VALUE[p.lower()] * (1 if p.isupper() == white else -1) for p in pos.board if p)


def bot_move(game):
    """a bot's move: the one that leaves it best off in material after your best reply; mate if it sees one"""
    me, best, pick = game.pos.white, None, game.legal[0]
    for m in game.legal:
        p = game.pos.play(m)
        replies = p.legal()
        if not replies:
            s = 1000 if p.in_check() else -0.5
        else:
            s = min(material(p.play(r), me) for r in replies) + (0.3 if p.in_check() else 0)
        s += random.random() * 0.6
        if best is None or s > best:
            best, pick = s, m
    return pick


hub = Hub()


# ---- a websocket, by hand (RFC 6455): enough for a terminal

def ws_recv(sock):
    """one message: (opcode, payload); None when the socket closed"""
    def exact(n):
        b = b""
        while len(b) < n:
            c = sock.recv(n - len(b))
            if not c:
                return None
            b += c
        return b
    data, op = b"", None
    while True:
        h = exact(2)
        if not h:
            return None
        fin, opcode, masked, n = h[0] & 0x80, h[0] & 0x0F, h[1] & 0x80, h[1] & 0x7F
        if n == 126:
            n = struct.unpack(">H", exact(2))[0]
        elif n == 127:
            n = struct.unpack(">Q", exact(8))[0]
        mask = exact(4) if masked else b"\0\0\0\0"
        p = bytearray(exact(n) or b"")
        for k in range(len(p)):
            p[k] ^= mask[k % 4]
        if opcode >= 8:                      # control frames come between the pieces of a message
            if opcode == 8:
                return None
            if opcode == 9:
                ws_send(sock, bytes(p), 10)
            continue
        if opcode:
            op = opcode
        data += bytes(p)
        if fin:
            return op, data


def ws_send(sock, data, op=2):
    n = len(data)
    head = bytes([0x80 | op]) + (bytes([n]) if n < 126 else bytes([126]) + struct.pack(">H", n) if n < 65536 else bytes([127]) + struct.pack(">Q", n))
    sock.sendall(head + data)


def clear_session(m, bot):
    """the neuralyzer: `/clear` typed into a session as you would type it. Attached in a pty, its screen
    drawn until the output settles, the command and Enter, a moment for it to take, then let go. Claude
    Code keeps the job and its process; the conversation is a new one (a new session id)"""
    pid, fd = pty.fork()
    if pid == 0:
        os.environ.update(TERM="xterm-256color", COLORTERM="truecolor")
        a = argv(m, f"exec claude attach {sq(bot)}", True)
        try:
            os.execvp(a[0], a)
        finally:
            os._exit(127)

    def settle(quiet, most):
        """read until it has said something and then nothing for `quiet` seconds (`most` at most)"""
        end, last, got = time.time() + most, time.time(), False
        while time.time() < end and not (got and time.time() - last >= quiet):
            if select.select([fd], [], [], 0.1)[0]:
                try:
                    if not os.read(fd, 65536):
                        break
                except OSError:
                    break
                last, got = time.time(), True
    try:
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        settle(1.5, 25)
        os.write(fd, b"/clear")
        time.sleep(0.5)
        os.write(fd, b"\r")
        settle(1.5, 8)
    except OSError:
        pass
    finally:
        for sig in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                break
            time.sleep(0.5)
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
        os.close(fd)


def terminal(sock, m, bot):
    """`claude attach <id>` in a pty, both ways over the websocket; closing it only ends the attach"""
    pid, fd = pty.fork()
    if pid == 0:
        os.environ.update(TERM="xterm-256color", COLORTERM="truecolor")
        a = argv(m, f"exec claude attach {sq(bot)}", True)
        try:
            os.execvp(a[0], a)
        finally:
            os._exit(127)
    lock = threading.Lock()

    def out():
        try:
            while True:
                r, _, _ = select.select([fd], [], [], 1)
                if r:
                    b = os.read(fd, 65536)
                    if not b:
                        break
                    with lock:
                        ws_send(sock, b)
        except OSError:
            pass
        try:
            with lock:
                ws_send(sock, b"", 8)
        except OSError:
            pass

    threading.Thread(target=out, daemon=True).start()
    try:
        while True:
            m = ws_recv(sock)
            if m is None:
                break
            op, data = m
            if op == 1:                      # text: control, as JSON
                c = json.loads(data)
                if c.get("size"):
                    cols, rows = c["size"]
                    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            else:                            # binary: keys
                os.write(fd, data)
    except (OSError, ValueError):
        pass
    finally:
        try:
            os.kill(pid, signal.SIGHUP)
        except OSError:
            pass
        try:
            os.close(fd)
        except OSError:
            pass
        threading.Thread(target=lambda: os.waitpid(pid, 0), daemon=True).start()


def rain_text(raw):
    """a transcript's tail as the matrix's rain: what was said, the tools' calls (a command, a file, a pattern) and the
    start of what came back, in order, as one line; the newest 4000 characters"""
    out = []
    for line in raw.splitlines()[1:]:          # the first may be cut
        try:
            d = json.loads(line)
        except ValueError:
            continue
        msg = d.get("message") if isinstance(d.get("message"), dict) else {}
        c = msg.get("content")
        for x in [{"type": "text", "text": c}] if isinstance(c, str) else c if isinstance(c, list) else []:
            if not isinstance(x, dict):
                continue
            if x.get("type") == "text":
                out.append(TAGS.sub("", x.get("text") or "")[:1500])
            elif x.get("type") == "tool_use":
                inp = x.get("input") or {}
                out.append(" ".join([str(x.get("name") or "")] + [str(inp[k])[:300] for k in ("command", "file_path", "pattern", "path", "url", "description") if inp.get(k)]))
            elif x.get("type") == "tool_result":
                r = x.get("content")
                r = " ".join(y.get("text", "") for y in r if isinstance(y, dict)) if isinstance(r, list) else str(r or "")
                out.append(r[:240])
    return re.sub(r"\s+", " ", " ".join(out))[-4000:]


def demo_stream(b):
    """the demo's rain: made up, the same for a bot each time"""
    r = random.Random(b["id"])
    words = ["Reading", "src/room.js", "web/server.py", "Running the tests", "14 passed", "git diff --stat", "Edited", "README.md",
             "the layout keeps its desks", "fixed the off-by-one in the seat rows", "cargo test", "all green", "Committed", b["name"],
             "npm run build", "the fold of worktrees", "TypeError at line 212", "tracing the poll", "looks right now"]
    return " ".join(r.choice(words) for _ in range(160))


def deliver(m, bot, text, timeout=60):
    """an answer typed into a background session without a terminal: `claude attach` in a pty of its own,
    the text pasted at its prompt once the screen has settled (a session whose process was off is
    restarted by the attach, which takes longer), Enter, a moment for it to be taken, then the attach is
    let go: the session goes on with it. Claude Code has no command to send a session a message"""
    pid, fd = pty.fork()
    if pid == 0:
        os.environ.update(TERM="xterm-256color", COLORTERM="truecolor")
        a = argv(m, f"exec claude attach {sq(bot)}", True)
        try:
            os.execvp(a[0], a)
        finally:
            os._exit(127)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    seen, t0 = b"", time.time()

    def read(quiet, until):
        """what comes, until it has been quiet `quiet` seconds after something came, or `until` passed"""
        nonlocal seen
        got, last = False, time.time()
        while time.time() < until:
            r, _, _ = select.select([fd], [], [], 0.2)
            if r:
                try:
                    b = os.read(fd, 65536)
                except OSError:
                    return False
                if not b:
                    return False
                seen, got, last = seen + b, True, time.time()
            elif got and time.time() - last > quiet:
                return True
        return got

    try:
        if not read(1.5, t0 + timeout):
            raise RuntimeError("the session did not come up")
        screen = re.sub(rb"\x1b\[[0-9;?]*[A-Za-z]", b"", seen).decode("utf-8", "replace")
        if re.search(r"No job matching|describe a task for a new session", screen):
            raise RuntimeError("could not attach to the session")
        os.write(fd, b"\x1b[200~" + text.encode() + b"\x1b[201~")     # pasted: new lines stay in the message
        time.sleep(0.5)
        os.write(fd, b"\r")
        read(1.0, time.time() + 4)
    finally:
        try:
            os.kill(pid, signal.SIGHUP)
        except OSError:
            pass
        try:
            os.close(fd)
        except OSError:
            pass
        threading.Thread(target=lambda: os.waitpid(pid, 0), daemon=True).start()


# ---- your terminal at a project's screen: one login shell per directory (the project's, or a
# worktree's laptop), ad hoc, for a quick look or a command (the bots do the work). It lives as long
# as this server does: a tile only attaches to it, closing the tile leaves it running, `exit` ends it.
# Its recent output is kept and replayed to the next attach. A cd elsewhere inside it is its own business
class Shell:
    KEEP = 200_000                  # bytes of output kept for the next attach

    def __init__(self, m, d):
        self.key = (m["id"], d)
        self.buf = bytearray()
        self.socks = []
        self.lock = threading.Lock()
        self.last = 0.0
        self.busy = False
        self.alive = True
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.environ.update(TERM="xterm-256color", COLORTERM="truecolor")
            script = f'cd {sq(d)} 2>/dev/null; exec "${{SHELL:-/bin/sh}}" -l'
            a = ["/bin/sh", "-c", PATH + script] if m.get("ssh") in (None, "", "@demo") else argv(m, script, True)
            try:
                os.execvp(a[0], a)
            finally:
                os._exit(127)
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        """the pty's output: kept, and sent on to whoever is attached; busy while it comes"""
        try:
            while True:
                r, _, _ = select.select([self.fd], [], [], 0.5)
                if r:
                    b = os.read(self.fd, 65536)
                    if not b:
                        break
                    with self.lock:
                        self.buf += b
                        del self.buf[:-self.KEEP]
                        socks = list(self.socks)
                    self.last = time.time()
                    for s in socks:
                        try:
                            ws_send(s, b)
                        except OSError:
                            self.detach(s)
                    if not self.busy:
                        self.busy = True
                        hub.push_shells()
                elif self.busy and time.time() - self.last > 1.5:
                    self.busy = False
                    hub.push_shells()
        except OSError:
            pass
        self.alive = False
        with self.lock:
            socks, self.socks = list(self.socks), []
        for s in socks:
            try:
                ws_send(s, b"", 8)
            except OSError:
                pass
        try:
            os.close(self.fd)
        except OSError:
            pass
        threading.Thread(target=lambda: os.waitpid(self.pid, 0), daemon=True).start()
        hub.drop_shell(self)

    def attach(self, sock):
        with self.lock:
            if self.buf:
                ws_send(sock, bytes(self.buf))
            self.socks.append(sock)

    def detach(self, sock):
        with self.lock:
            if sock in self.socks:
                self.socks.remove(sock)

    def resize(self, cols, rows):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    def write(self, data):
        os.write(self.fd, data)

    def kill(self):
        try:
            os.kill(self.pid, signal.SIGHUP)
        except OSError:
            pass


def shell_ws(sock, sh):
    """a terminal attached: keys and the size in, its output out (Shell.read); closing only detaches"""
    sh.attach(sock)
    try:
        while True:
            m = ws_recv(sock)
            if m is None:
                break
            op, data = m
            if op == 1:                      # text: control, as JSON
                c = json.loads(data)
                if c.get("size"):
                    cols, rows = c["size"]
                    sh.resize(cols, rows)
            else:                            # binary: keys
                sh.write(data)
    except (OSError, ValueError):
        pass
    finally:
        sh.detach(sock)


def upload(m, name, data):
    """a file dropped on a session: kept here, and copied over to a remote machine; returns its path there"""
    stamp = str(int(time.time() * 1000))
    d = os.path.join(CACHE, "drop", stamp)
    os.makedirs(d, exist_ok=True)
    local = os.path.join(d, name)
    with open(local, "wb") as f:
        f.write(data)
    ssh = m.get("ssh")
    if not ssh or ssh == "@demo":
        return local
    there = run(m, f'd="$HOME/.cache/burrow/drop/{stamp}"; mkdir -p "$d" && echo "$d"').strip() + "/" + name
    # scp speaks SFTP now: the remote path is taken literally, no quoting
    port = ["-P", str(m["port"])] if m.get("port") else []
    p = subprocess.run(["scp"] + ssh_opts(m) + port + ["-q", local, f"{ssh}:{there}"], capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(p.stderr.strip())
    return there


# ---- http

TYPES = {".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8",
         ".png": "image/png", ".woff2": "font/woff2", ".svg": "image/svg+xml"}


# Who may talk to this server: its own page (localhost or 127.0.0.1 at its port) and programs with no browser behind
# them, which send no Origin (the office managers' curl). Without this any page open in a browser could open a
# terminal here (a websocket is not held to the same-origin rule), a login shell on this or any machine, or post to
# the API; and a name of the attacker's that resolves to 127.0.0.1 (DNS rebinding) is turned away by its Host
HOSTS = {f"localhost:{PORT}", f"127.0.0.1:{PORT}"}


def trusted(headers):
    origin = headers.get("Origin")
    return headers.get("Host", "") in HOSTS and (origin is None or (origin.startswith("http://") and origin[7:] in HOSTS))


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def refuse(self):
        self.close_connection = True
        self.reply(403, {"error": "not from here"})

    def reply(self, code, body, ctype="application/json"):
        if not isinstance(body, bytes):
            body = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if not trusted(self.headers):
            return self.refuse()
        u = urlparse(self.path)
        q = {k: v[0] for k, v in parse_qs(u.query).items()}
        if u.path == "/events":
            return self.events()
        if u.path in ("/term", "/shell"):
            return self.term(q, u.path == "/shell")
        if u.path == "/api/office":          # the office as the office manager sees it (curl from her session)
            return self.reply(200, hub.office())
        if u.path == "/raw":                 # a picture on a session's machine, for a markdown file shown rendered (pictures only)
            try:
                kind, m = IMAGES[os.path.splitext(q.get("path", ""))[1].lower()], hub.machine(q.get("m"))
                if m.get("ssh") == "@demo":
                    raise LookupError
                return self.reply(200, base64.b64decode(run(m, "python3 -c " + sq(RAW) + " " + sq(q["path"]), timeout=30)), kind)
            except Exception:
                return self.reply(404, b"not here", "text/plain")
        if u.path == "/doom.wad":            # the gun closet's Doom data: id's own WAD if you put one here, else Freedoom's (doom/)
            return self.reply(200, open(doom_wad(), "rb").read(), "application/octet-stream")
        name = "index.html" if u.path == "/" else "matrix.html" if u.path == "/matrix" else u.path.lstrip("/")
        path = os.path.normpath(os.path.join(HERE, name))
        if not path.startswith(HERE + os.sep) or not os.path.isfile(path) or name.endswith(".py"):
            return self.reply(404, b"not here", "text/plain")
        self.reply(200, open(path, "rb").read(), TYPES.get(os.path.splitext(path)[1], "application/octet-stream"))

    def events(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        q = queue.Queue()
        hub.clients.append(q)
        q.put(f"event: state\ndata: {json.dumps(hub.state())}\n\n".encode())
        q.put(f"event: digest\ndata: {json.dumps(hub.digest_view())}\n\n".encode())
        try:
            while True:
                try:
                    self.wfile.write(q.get(timeout=20))
                except queue.Empty:
                    self.wfile.write(b": still here\n\n")
                self.wfile.flush()
        except OSError:
            pass
        finally:
            hub.clients.remove(q)
            self.close_connection = True

    def term(self, q, shell=False):
        """a websocket: a session attached (`claude attach`), or your terminal at a screen (Shell)"""
        key = self.headers.get("Sec-WebSocket-Key")
        m = hub.machines.get(q.get("m", ""))
        if DEMO and REAL_OFFICE and q.get("id") and q["id"] in {m.get("id") for m in (hub.room.get("managers") or {}).values()}:
            m = LOCAL                          # the demo's office manager is real
        if DEMO and not shell and q.get("id") in DEMO_REAL:
            m, q = LOCAL, {**q, "id": DEMO_REAL[q["id"]]}
        if not m:
            return self.reply(404, {"error": "no such machine"})
        if not key or not q.get("dir" if shell else "id"):
            return self.reply(400, {"error": "a websocket for a terminal"})
        accept = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        self.wfile.flush()
        if shell:
            shell_ws(self.connection, hub.shell(m, q["dir"]))
        else:
            terminal(self.connection, m, q["id"])
        self.close_connection = True

    def do_POST(self):
        if not trusted(self.headers):
            return self.refuse()
        u = urlparse(self.path)
        n = int(self.headers.get("Content-Length") or 0)
        if u.path == "/api/upload":
            q = {k: v[0] for k, v in parse_qs(u.query).items()}
            try:
                return self.reply(200, {"path": upload(hub.machine(q.get("m")), os.path.basename(q.get("name") or "file"), self.rfile.read(n))})
            except Exception as e:
                return self.reply(200, {"error": str(e).strip()[:500]})
        try:
            body = json.loads(self.rfile.read(n) or b"{}")
        except ValueError:
            return self.reply(400, {"error": "not json"})
        try:
            self.reply(200, self.act(u.path, body))
        except Exception as e:
            self.reply(200, {"error": str(e).strip()[:500]})

    def act(self, path, b):
        if path == "/api/focus":
            hub.focused = bool(b.get("on"))
            if hub.focused:
                for p in list(hub.pokes.values()):
                    p.set()
            return {}
        if path == "/api/machine":           # another machine, over ssh: an entry in Claude Code's sshConfigs
            ssh, name = (b.get("ssh") or "").strip(), (b.get("name") or "").strip()
            if not ssh:
                raise RuntimeError("where to ssh to")
            if DEMO:                         # a made-up one (it never writes the settings)
                mid = re.sub(r"[^a-z0-9]+", "-", ssh.lower()).strip("-") or "machine"
                DEMO_GONE.discard(mid)
                if all(m["id"] != mid for m in DEMO_ADDED):
                    DEMO_ADDED.append({"id": mid, "name": name or ssh, "ssh": "@demo"})
            else:
                run({"ssh": ssh}, "true", timeout=20)
                mid = add_machine(ssh, name)
            at = b.get("at")
            if isinstance(at, list) and len(at) == 2:   # its door, from the closet: where it was let go on the wall
                with hub.lock:
                    hub.room["layout"]["door:" + mid] = [int(at[0]), int(at[1])]
                    save_room(hub.room)
            hub.set_machines(machines())
            return {"m": mid}
        if path == "/api/unmachine":         # a door taken out (the doors' panel): out of sshConfigs, and out of the room
            mid = b.get("m") or ""
            if not mid:
                raise RuntimeError("this computer's door stays")
            if DEMO:
                DEMO_GONE.add(mid)
            else:
                remove_machine(mid)
            poke = hub.pokes.get(mid)
            hub.set_machines(machines())
            if poke:
                poke.set()                   # its watcher sees it gone, drops its bots and stops
            for key, sh in hub.shells.copy().items():
                if key[0] == mid:
                    sh.kill()                # your terminals there
            with hub.lock:                   # its door's place and paint, its desks and theirs
                r = hub.room
                r["layout"] = {k: v for k, v in r["layout"].items() if k != "door:" + mid and not k.startswith("u:" + mid + ":")}
                (r.get("paint") or {}).pop(mid, None)
                r["desks"] = [u for u in r["desks"] if not u.startswith(mid + ":")]
                save_room(r)
            hub.push("state", hub.state())
            return {}
        if path == "/api/chess":
            return hub.chess(b)
        if path == "/api/layout":            # where the furniture stands, moved by hand: {key: [x, y]}
            # only what moved (`moved`), merged: the page's whole copy could be older than what was just put down (a piece
            # out of the closet, a desk, a reception moved) and would take it away again. A page from before sends `layout`
            moved = b.get("moved")
            with hub.lock:
                if moved is None:
                    hub.room["layout"] = {k: [int(v[0]), int(v[1])] for k, v in (b.get("layout") or {}).items()}
                else:
                    hub.room["layout"].update({k: [int(v[0]), int(v[1])] for k, v in moved.items()})
            save_room(hub.room)
            hub.push("state", hub.state())
            return {}
        if path == "/api/furniture":         # a piece out of the closet, put down at `at`; or one burnt in the fireplace
            with hub.lock:
                fs = hub.room["furniture"]
                if b.get("add"):
                    kind = b["add"]
                    if kind not in kinds():
                        raise RuntimeError("no such furniture")
                    if kind == "manager":
                        raise RuntimeError("a reception comes with an office manager: pick one")
                    key = next(f"{kind}{i}" for i in range(1000) if f"{kind}{i}" not in fs)
                    fs.append(key)
                    hub.room["layout"][key] = [int(b["at"][0]), int(b["at"][1])]
                    if kind == "radio":                  # a radio put down: the sound is on
                        hub.room.setdefault("settings", {})["sound"] = True
                    if kind == "files":                  # a filing cabinet put down: the file trees are on
                        hub.room.setdefault("settings", {})["files"] = True
                    if kind == "pumpkin":                # a pumpkin put down: it is lit
                        hub.room.setdefault("settings", {})["halloween"] = True
                elif b.get("remove"):
                    key = b["remove"]
                    if kind_of(key) in FIXED or key not in fs:
                        raise RuntimeError("that stays")
                    fs.remove(key)
                    hub.room["layout"].pop(key, None)
                    hub.room.get("pair", {}).pop(key, None)
                else:
                    raise RuntimeError("add or remove what?")
                save_room(hub.room)
            if b.get("remove") and kind_of(key) == "manager":
                hub.fire(key)
            hub.push("state", hub.state())
            return {"key": key}
        if path == "/api/pair":              # a pair table: a new room opened ({key, new: name}), a room linked ({key, url}: your page
            key = b.get("key") or ""         # or an invite), or looked up ({key}): the room and the bring link
            if kind_of(key) != "pair":
                raise RuntimeError("not a pair piece")
            with hub.lock:
                have = hub.room.setdefault("pair", {}).get(key)
            fresh, url = True, None
            if (b.get("new") or "").strip():
                url = pair_new(b["new"].strip()[:80])
            elif (b.get("url") or "").strip():
                url = pair_page(b["url"].strip())
            else:
                fresh, url = False, (have or {}).get("url")
            if not url:
                raise RuntimeError("no room linked yet")
            warn = ""
            if fresh:                        # the seat is yours: your name and face on it (a new room has them already)
                try:
                    pair_me(url)
                except RuntimeError as e:
                    warn = f"pair: {e}"
            st = pair_state(url)
            with hub.lock:
                hub.room["pair"][key] = {"url": st["url"], "room": st["room"]}
                save_room(hub.room)
            hub.push("state", hub.state())
            return {**st, "warn": warn}
        if path == "/api/room":              # the room's width in dots (the divider, dragged)
            hub.room["width"] = max(200, min(1200, int(b.get("width") or 256)))
            save_room(hub.room)
            return {}
        if path == "/api/awake":             # the clock: keep this computer awake while bots work, for some minutes, or not
            mode, minutes = b.get("mode") or "off", int(b.get("minutes") or 0)
            if mode not in ("off", "work", "for") or (mode == "for" and minutes <= 0):
                raise RuntimeError("awake how?")
            if mode != "off" and not inhibitor():
                raise RuntimeError("no caffeinate or systemd-inhibit on this computer")
            hub.set_awake(mode, minutes)
            return hub.awake_view()
        if path == "/api/you":               # who you are: your name (empty: the login name), the face's seed, your colour
            with hub.lock:
                you = dict(hub.room.get("you") or {})
                for k in ("name", "seed", "color"):
                    if k in b:
                        v = re.sub(r"\s+", " ", str(b[k] or "")).strip()[:40]
                        if k == "seed":
                            v = pair_seed(v)
                        if v:
                            you[k] = v
                        else:
                            you.pop(k, None)
                hub.room["you"] = you
                save_room(hub.room)
            hub.push("state", hub.state())
            pair_me_all(hub.room)
            return you_of(hub.room)
        if path == "/api/paint":             # a machine's door (and its desks) in another paint: {m, paint: n}, -1 the wood, none its own again
            pt = hub.room.setdefault("paint", {})
            if b.get("paint") is None:
                pt.pop(b.get("m") or "", None)
            else:
                pt[b.get("m") or ""] = int(b["paint"])
            save_room(hub.room)
            hub.push("state", hub.state())
            return {}
        if path == "/api/settings":          # the switchboard: {key: true/false} for the options there are
            st = hub.room.setdefault("settings", {})
            for k in OPTIONS:
                if k in b:
                    st[k] = bool(b[k])
            save_room(hub.room)
            hub.push("state", hub.state())
            hub.push_digest()
            return {}
        if path == "/api/manager":           # the office manager: the styles there are; one hired ({hire, at}); her session ({open})
            if b.get("hire"):
                return hub.hire(b["hire"], b.get("at"))
            if b.get("open"):
                return hub.manager_open(b["open"])
            if b.get("gone"):
                return hub.manager_gone(b["gone"])
            here = {hub.manager(k)["style"] for k in hub.booths()}
            return {"styles": [{"key": k, "here": k in here, **{f: M[f] for f in ("name", "seed", "color", "line", "how", "session")}} for k, M in MANAGERS.items()]}
        if path == "/api/office":            # what she does in the office: remove, desk, start, say (and, from the room, kept)
            return hub.office_do(b)
        if path == "/api/stream":            # the matrix's rain: {ids} → {texts: {id: what it said and did}}
            return {"texts": hub.stream([str(x) for x in (b.get("ids") or [])])}
        if path == "/api/explain":           # a decision left to you, explained by Claude (kept with the notes)
            return hub.explain(b.get("id") or "", b.get("question") or "")
        if path == "/api/answer":            # your decision, typed into the session
            text = (b.get("text") or "").strip()
            if not text:
                raise RuntimeError("an answer, first")
            return hub.answer(b.get("id") or "", b.get("question") or "", text[:4000])
        if path == "/api/digest":            # the notes opened: the sessions that moved since are written up again
            return {"queued": hub.digest_refresh() if hub.notes_on() else 0}
        if path == "/api/desk":              # a desk renamed (its panel): {unit, name}, empty for its folder's name; {unit, keep: true} keeps it
            if b.get("keep"):
                if b.get("unit") not in {d["unit"] for d in hub.desks()}:
                    raise RuntimeError("no such desk")
                hub.keep(b["unit"], True)
                return {}
            hub.rename(b.get("unit") or "", (b.get("name") or "").strip())
            return {}
        if path == "/api/forget":            # a desk burnt (its bots went first, through the booth or your gun)
            hub.kill_shells(b["unit"])
            hub.keep(b["unit"], False)
            return {}
        mid = b.get("m") or ""
        m, poke = hub.machine(mid), hub.pokes.get(mid)
        if path == "/api/hosts":             # the hosts of ~/.ssh/config, for the new door's field
            return {"hosts": ssh_hosts()}
        if path == "/api/file":              # a file there, for the editor beside a terminal: read it, look at it, or write it
            op = b.get("op") or "read"
            if m.get("ssh") == "@demo":
                return demo_file(b["path"], op, b)
            arg = {"op": op, "path": b["path"], "mtime": b.get("mtime"), "force": bool(b.get("force"))}
            return json.loads(run(m, "python3 -c " + sq(FILE) + " " + sq(json.dumps(arg)), timeout=20, input=b.get("text", "") if op == "write" else None))
        if path == "/api/tree":              # the file tree beside a terminal: folders under its root, and git's changes there
            if m.get("ssh") == "@demo":
                now = int(time.time())
                fake = {"": [["src", 1, now - 900], ["docs", 1, now - 86400], ["README.md", 0, now - 40], ["package.json", 0, now - 7200]],
                        "src": [["lib", 1, now - 3000], ["main.js", 0, now - 20], ["room.js", 0, now - 5000]], "src/lib": [["util.js", 0, now - 9000]],
                        "docs": [["notes.md", 0, now - 86400]]}
                return {"dirs": {d: fake.get(d) for d in b.get("dirs") or [""]}, "git": {"src/main.js": "M", "README.md": "A"}, "now": now}
            out = run(m, "python3 -c " + sq(TREE) + " " + sq(b["root"]) + " " + sq(json.dumps(b.get("dirs") or [""])), timeout=20)
            return json.loads(out)
        if path == "/api/complete":          # directories there that start with what was typed so far
            d = (b.get("dir") or "").strip() or "~/"
            if m.get("ssh") == "@demo":
                base, _, part = d.rpartition("/")
                return {"dirs": [f"{base}/{x}/" for x in ("alpha", "beta", "gamma") if x.startswith(part)]}
            out = run(m, f"ls -d {sq(d)}*/ 2>/dev/null | head -300", timeout=15)   # (the new desk's folder list: all of a folder's folders)
            return {"dirs": [x for x in out.splitlines() if x]}
        if path == "/api/recent":            # the working directories there without a desk (the cabinet's form)
            if m.get("ssh") == "@demo":
                return {"home": HOME, "dirs": hub.off_floor(mid, [f"{HOME}/projects/{x}" for x in ("alpha", "beta", "gamma")])}
            out = run(m, RECENT, timeout=20).splitlines()
            return {"home": out[0] if out else "", "dirs": hub.off_floor(mid, [x for x in out[1:] if x.startswith("/")])}
        if path == "/api/dir":               # a desk for a working directory there, kept
            d = (b.get("dir") or "").strip()
            if m.get("ssh") == "@demo":
                d = d.replace("~", HOME, 1)
            else:
                d = run(m, f"cd {sq(d)} && pwd -P", timeout=20).strip()
            at = b.get("at")
            if isinstance(at, list) and len(at) == 2:   # out of the closet: where it was let go (before the state that
                with hub.lock:                           # brings it goes out, or the page finds it a place of its own)
                    hub.room["layout"]["u:" + unit(mid, d)] = [int(at[0]), int(at[1])]
            hub.keep(unit(mid, d), True)
            return {"unit": unit(mid, d)}
        if path == "/api/spawn":             # a new bot at a desk; returns its id
            if m.get("ssh") == "@demo":
                return {"error": "demo bots can't be started"}
            return {"id": hub.spawn(m, b["dir"], (b.get("name") or "").strip())}
        if path == "/api/tell":              # the matrix's `tell`: a message typed into a session (deliver)
            bot = next((x for x in hub.bots() if x["id"] == b.get("id")), None)
            text = (b.get("text") or "").strip()[:4000]
            if not bot or not text:
                raise RuntimeError("tell whom what")
            if bot.get("dialog"):
                raise RuntimeError("it has a prompt open in its session: answer it there")
            if m.get("ssh") == "@demo":
                for x in hub.demo_bots.get(mid, []):
                    if x["id"] == bot["id"]:
                        x.update(busy=True, waiting=False, state="working", over="", alive=True)
            else:
                threading.Thread(target=deliver, args=(m, bot["id"], text), daemon=True).start()
            return {}
        if path == "/api/clear":             # the neuralyzer: /clear typed into the session, in the background
            if m.get("ssh") != "@demo":
                threading.Thread(target=clear_session, args=(m, b["id"]), daemon=True).start()
            return {}
        if path == "/api/stop":              # stop a session: it stays listed, completed; with rm, it is
            if m.get("ssh") == "@demo":      # deleted from the list as well (agent view's second Ctrl+X, `claude rm`)
                hub.demo_stop(mid, b["id"], bool(b.get("rm")))
            elif b.get("rm"):
                run(m, f"claude stop {sq(b['id'])} >/dev/null 2>&1; claude rm {sq(b['id'])}")
            else:
                run(m, f"claude stop {sq(b['id'])}")
            poke.set()
            return {}
        raise RuntimeError("no such thing: " + path)


if __name__ == "__main__":
    hub.set_machines(machines())
    srv = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    srv.daemon_threads = True
    print(f"burrow: http://localhost:{PORT}", flush=True)
    srv.serve_forever()
