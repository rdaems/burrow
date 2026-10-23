#!/usr/bin/env python3
# burrow in the browser: this serves the room (index.html) and is its only link to the machines.
# Everything it knows comes from Claude Code itself: which machines there are (this one, and the
# `sshConfigs` in its settings, as the Desktop app's SSH environments), and on each `claude agents --json`
# (who runs, where, busy or idle), `claude --bg` (start one), `claude attach` (open one, in a pty, over
# a websocket), `claude stop`. Nothing is installed there. Python's standard library only.
# Burrow itself keeps only what Claude Code has no notion of: the desks you keep, where furniture stands.
#
#   python3 web/server.py            then open http://localhost:7777
#   BURROW_DEMO=1 python3 web/server.py    made-up bots, for working on the room

import base64, collections, fcntl, getpass, hashlib, json, os, pty, queue, re, select, shlex, signal, socket, struct, subprocess
import termios, threading, time, random
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
PORT = int(os.environ.get("BURROW_PORT", "7777"))
HOSTNAME = subprocess.run(["hostname", "-s"], capture_output=True, text=True).stdout.strip() or "here"
USER = getpass.getuser()                       # you, in the room

# a non-login shell over ssh has a bare PATH; claude lives here
PATH = 'PATH="$HOME/.local/bin:$HOME/.claude/local:/opt/homebrew/bin:/usr/local/bin:$PATH"; export PATH; export COLORTERM=truecolor; '

# a message between sessions shows up in the receiver's transcript as a user turn with
# "origin":{"kind":"peer","from":"uds:…","verifiedPeerPid":…,"msg_id":"…","name":"<sender>",…};
# grep the last 4 MB of transcripts written in the last minute (a session can write a few hundred kB
# right after the message) and print "<receiver session id> <origin…>" per message
PEERS = r'''find ~/.claude/projects -maxdepth 2 -name '*.jsonl' -mmin -1 2>/dev/null | while IFS= read -r f; do s=${f##*/}; tail -c 4194304 "$f" | grep -o '"origin":{"kind":"peer"[^}]*' | sed "s|^|${s%.jsonl} |"; done'''


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
        return [{"id": "", "name": "mac", "ssh": "@demo"}, {"id": "odyssey", "name": "odyssey", "ssh": "@demo"}]
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


def run(m, script, timeout=60):
    p = subprocess.run(argv(m, script, False), capture_output=True, text=True, timeout=timeout)
    if p.returncode == 0:
        return p.stdout
    raise RuntimeError((p.stderr.strip() or p.stdout.strip() or f"exit {p.returncode}"))


# ---- the room: the desks you keep and where the furniture stands, in ~/.config/burrow/room.json.
# A desk is a unit: a directory here, or "<machine id>:<directory>" on another machine.

def unit(mid, d):
    return f"{mid}:{d}" if mid else d


def split_unit(u):
    return ("", u) if u.startswith(("/", "~")) else tuple(u.split(":", 1))


def load_room():
    """the first time, the desks and layout of the Rust app's rooms.json, one room per machine
    (not the desks of worktrees: those sit at their repo's desk now)"""
    if DEMO:
        return {"desks": [HOME + "/projects/romuald"], "layout": {}}
    try:
        return {"desks": [], "layout": {}, **json.load(open(ROOM))}
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
    return room


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


def parse_agents(out):
    """the sessions `claude agents` shows, with their state as agent view groups them: the background
    ones (an interactive `claude` in someone's terminal is listed too, but nothing can attach to it, so
    it is left out, as agent view does). `state` is the session's (working, blocked, done, failed,
    stopped), `status` its process's (busy, waiting, idle) while that runs:
      needs    blocked, or a prompt open (status waiting): it waits for you, whatever its process does
      busy     a turn runs
      idle     done, or nothing to do, its process still there
    A session whose process has exited is listed only while it works or waits for you; the supervisor
    restarts it when it is opened."""
    bots = []
    for x in json.loads(out.strip() or "[]"):
        if x.get("kind") != "background" or not x.get("id"):
            continue
        state, status = x.get("state"), x.get("status")
        needs = state == "blocked" or status == "waiting"
        bots.append({
            "id": x["id"], "name": x.get("name") or x["id"], "cwd": x.get("cwd") or "",
            "busy": not needs and (status == "busy" or state == "working"),
            "waiting": needs, "alive": "pid" in x, "state": state or "",
            "started": x.get("startedAt") or 0, "pid": x.get("pid") or 0, "session": x.get("sessionId") or "",
        })
    bots.sort(key=lambda b: b["started"])
    return bots


def parse_usage(out):
    """`/usage`: "Current session: 3% used · resets Oct 21 at 3:20pm (…)" → {session: {used, resets}, week: …}"""
    u = {}
    for line in out.splitlines():
        m = re.match(r"\s*Current (session|week \(all models\)): (\d+)% used(?: · resets (.*?))?\s*(?:\(.*\))?$", line)
        if m:
            u["session" if m.group(1) == "session" else "week"] = {"used": int(m.group(2)), "resets": m.group(3) or ""}
    return u


def field(s, key):
    m = re.search(r'"%s":"?([^",}]*)' % re.escape(key), s)
    return m.group(1) if m else None


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
        self.lock = threading.Lock()
        self.clients = []           # one queue per open page
        self.focused = True
        self.pokes = {}
        self.usage = None           # the account's limits, from Claude Code's own /usage here
        self.own = collections.deque(maxlen=4)  # its pids: `claude -p` shows up in `claude agents` while it runs
        self.game = None            # the chess game you play against one of the idle bots

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

    def desks(self, bots):
        """the desks you keep, then one wherever a session runs that has none"""
        out, seen = [], set()
        for u in self.room["desks"]:
            mid, d = split_unit(u)
            if mid in self.machines and u not in seen:
                seen.add(u)
                out.append({"unit": u, "m": mid, "dir": d, "kept": True})
        for b in bots:
            if b["unit"] not in seen:
                seen.add(b["unit"])
                out.append({"unit": b["unit"], "m": b["m"], "dir": split_unit(b["unit"])[1], "kept": False})
        return out

    def state(self):
        with self.lock:
            bots = self.bots()
            return {"machines": [{"id": mid, "name": m["name"], "ssh": m.get("ssh") or "", **{k: v for k, v in self.feeds.get(mid, {}).items() if k != "bots"}}
                                 for mid, m in self.machines.items()],
                    "desks": self.desks(bots), "bots": bots, "layout": self.room["layout"],
                    "usage": self.usage, "chess": self.chess_view(), "demo": DEMO, "you": {"name": USER}}

    def keep(self, u, on):
        with self.lock:
            ds = [x for x in self.room["desks"] if x != u]
            self.room["desks"] = ds + [u] if on else ds
            save_room(self.room)
        self.push("state", self.state())

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
                out = run(m, f"claude agents --json; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null; echo '@@burrow@@'; {PEERS}")
                agents, settings, peers = (out.split("@@burrow@@", 2) + ["", ""])[:3]
                bots = [b for b in parse_agents(agents) if mid or b["pid"] not in self.own]
                try:
                    light = str(json.loads(settings.strip()).get("theme", "")).startswith("light")
                except Exception:
                    light = False
                talks = parse_peers(peers, bots)
                self.fold(mid, m, bots)
                with self.lock:
                    self.feeds[mid] = {"bots": bots, "light": light, "error": None}
                self.chess_check()
                for msg, a, b in talks:
                    if msg not in heard:
                        heard.add(msg)
                        if not first:
                            self.push("talk", {"from": a, "to": b})
                first = False
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
        bots = [("burrow", "burrow"), ("burrow", "talk-ui"), ("burrow/.claude/worktrees/icons", "icons"), ("hexant", "hexant"),
                ("hexant/.claude/worktrees/ants-2", "ants-2"), ("website", "website"), ("", "home"), ("football", "football")]
        if i == 1:
            bots = [("train", "train"), ("train/.claude/worktrees/eval", "eval"), ("", "home")]
        self.usage = {"session": {"used": 64, "resets": "3:20pm"}, "week": {"used": 23, "resets": "Oct 24 at 9am"}}
        bots = [{"id": f"demo{i}{k:03}", "name": n, "cwd": f"{HOME}/projects/{d}" if d else HOME, "busy": k % 3 == 0, "waiting": k % 5 == 1,
                 "alive": True, "state": "working" if k % 3 == 0 else "blocked" if k % 5 == 1 else "done",
                 "started": k, "pid": 1000 + k, "session": f"demo{k}"}
                for k, (d, n) in enumerate(bots)]
        m = self.machines[mid]
        self.fold(mid, m, bots)
        while True:
            for b in bots:
                if random.random() < 1 / 12:
                    b["busy"] = not b["busy"]
                    b["waiting"] = not b["busy"] and random.random() < 0.1
                    b["state"] = "working" if b["busy"] else "blocked" if b["waiting"] else "done"
            if random.random() < 1 / 8:
                a, b = random.sample(bots, 2)
                self.push("talk", {"from": a["id"], "to": b["id"]})
            with self.lock:
                self.feeds[mid] = {"bots": [dict(b) for b in bots], "light": False, "error": None}
            self.chess_check()
            self.push("state", self.state())
            poke.wait(3)
            poke.clear()

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
        g = self.game
        if g and not g.get("result") and not self.idle_bot(g["bot"]):
            self.chess_end("1-0" if g["white"]["id"] == "you" else "0-1", "left")

    def chess_end(self, result, why):
        self.game["result"], self.game["why"] = result, why

    def chess(self, b):
        g = self.game
        if b.get("challenge"):
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
            self.push("state", self.state())
        threading.Timer(random.uniform(1.2, 3.5), play).start()


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


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

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
        u = urlparse(self.path)
        q = {k: v[0] for k, v in parse_qs(u.query).items()}
        if u.path == "/events":
            return self.events()
        if u.path == "/term":
            return self.term(q)
        name = "index.html" if u.path == "/" else u.path.lstrip("/")
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

    def term(self, q):
        key = self.headers.get("Sec-WebSocket-Key")
        m = hub.machines.get(q.get("m", ""))
        if not m:
            return self.reply(404, {"error": "no such machine"})
        if not key or not q.get("id"):
            return self.reply(400, {"error": "a websocket for a bot"})
        accept = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        self.wfile.flush()
        terminal(self.connection, m, q["id"])
        self.close_connection = True

    def do_POST(self):
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
            if DEMO:
                raise RuntimeError("not in the demo")
            run({"ssh": ssh}, "true", timeout=20)
            mid = add_machine(ssh, name)
            hub.set_machines(machines())
            return {"m": mid}
        if path == "/api/chess":
            return hub.chess(b)
        if path == "/api/layout":            # where the furniture stands, moved by hand: {key: [x, y]}
            hub.room["layout"] = {k: [int(v[0]), int(v[1])] for k, v in (b.get("layout") or {}).items()}
            save_room(hub.room)
            hub.push("state", hub.state())
            return {}
        if path in ("/api/keep", "/api/forget"):    # a desk you keep, or not any more
            hub.keep(b["unit"], path == "/api/keep")
            return {}
        mid = b.get("m") or ""
        m, poke = hub.machine(mid), hub.pokes.get(mid)
        if path == "/api/dir":               # a desk for a working directory there, kept
            d = (b.get("dir") or "").strip()
            if m.get("ssh") == "@demo":
                d = d.replace("~", HOME, 1)
            else:
                d = run(m, f"cd {sq(d)} && pwd -P", timeout=20).strip()
            hub.keep(unit(mid, d), True)
            return {"unit": unit(mid, d)}
        if path == "/api/spawn":             # a new bot at a desk; returns its id
            if m.get("ssh") == "@demo":
                return {"error": "demo bots can't be started"}
            name = (b.get("name") or "").strip()
            out = run(m, f"cd {sq(b['dir'])} && claude --bg{' -n ' + sq(name) if name else ''} 2>&1")
            # "backgrounded · 7ebbc505 · name (idle — …)" or, unnamed, "backgrounded · 7ebbc505 (idle — …)",
            # maybe after "Starting background service…"
            mm = re.search(r"^\s*backgrounded · (\S+?)(?=[\s·]|$)", out, re.M)
            if not mm:
                raise RuntimeError(out.strip())
            poke.set()
            return {"id": mm.group(1)}
        if path == "/api/stop":
            if m.get("ssh") == "@demo":
                return {}
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
