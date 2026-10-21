#!/usr/bin/env python3
# burrow in the browser: this serves the room (index.html) and is its only link to the machines.
# Everything it knows comes from Claude Code itself on each machine: `claude agents --json` (who runs,
# where, busy or idle), `claude --bg` (start one), `claude attach` (open one, in a pty, over a
# websocket), `claude stop`. Nothing is installed there. Python's standard library only.
#
#   python3 web/server.py            then open http://localhost:7777
#   BURROW_DEMO=1 python3 web/server.py    made-up bots, for working on the room

import base64, fcntl, hashlib, json, os, pty, queue, re, select, shlex, signal, socket, struct, subprocess
import termios, threading, time, random
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.expanduser("~")
CONFIG = os.path.join(HOME, ".config/burrow/rooms.json")
CACHE = os.path.join(HOME, ".cache/burrow")
DEMO = bool(os.environ.get("BURROW_DEMO"))
PORT = int(os.environ.get("BURROW_PORT", "7777"))

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


# ---- rooms: one per machine, kept in ~/.config/burrow/rooms.json (shared with the Rust app)

def load_rooms():
    if DEMO:
        return [{"name": n, "ssh": "@demo", "dirs": [HOME + "/projects/romuald"]} for n in ("mac", "odyssey")]
    try:
        v = json.load(open(CONFIG))
        if v:
            return v
    except Exception:
        pass
    name = subprocess.run(["hostname", "-s"], capture_output=True, text=True).stdout.strip() or "here"
    return [{"name": name}]


def save_rooms(rooms):
    if DEMO:
        return
    os.makedirs(os.path.dirname(CONFIG), exist_ok=True)
    keep = [{k: r[k] for k in ("name", "ssh", "dirs") if r.get(k)} for r in rooms]
    with open(CONFIG, "w") as f:
        json.dump(keep, f, indent=2)


def ssh_opts():
    os.makedirs(CACHE, exist_ok=True)
    return ["-o", "ControlMaster=auto", "-o", f"ControlPath={CACHE}/ssh-%C", "-o", "ControlPersist=600",
            "-o", "ServerAliveInterval=15"]


def argv(room, script, tty):
    """program + args that run `script` in a shell on the room's machine"""
    script = PATH + script
    ssh = room.get("ssh")
    if ssh == "@demo":
        return ["/bin/sh", "-c", "printf 'a demo bot: no session behind it\\r\\n'; exec cat"]
    if not ssh:
        return ["/bin/sh", "-c", script]
    return ["ssh"] + ssh_opts() + (["-t"] if tty else ["-o", "BatchMode=yes"]) + [ssh, script]


def run(room, script, timeout=60):
    p = subprocess.run(argv(room, script, False), capture_output=True, text=True, timeout=timeout)
    if p.returncode == 0:
        return p.stdout
    raise RuntimeError((p.stderr.strip() or p.stdout.strip() or f"exit {p.returncode}"))


def parse_agents(out):
    bots = []
    for x in json.loads(out.strip() or "[]"):
        if "pid" not in x:
            continue
        sid = x.get("sessionId") or ""
        bid = x.get("id") or sid[:8]
        bots.append({
            "id": bid, "name": x.get("name") or bid, "cwd": x.get("cwd") or "",
            "busy": x.get("status") == "busy" or x.get("state") == "working",
            "background": x.get("kind") == "background",
            "started": x.get("startedAt") or 0, "pid": x.get("pid") or 0, "session": sid,
        })
    bots.sort(key=lambda b: b["started"])
    return bots


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
        self.rooms = load_rooms()
        self.feeds = [{} for _ in self.rooms]
        self.lock = threading.Lock()
        self.clients = []           # one queue per open page
        self.focused = True
        self.pokes = []

    def state(self):
        with self.lock:
            return {"rooms": [{"name": r["name"], "ssh": r.get("ssh") or "", "dirs": r.get("dirs", []), **f}
                              for r, f in zip(self.rooms, self.feeds)], "demo": DEMO}

    def push(self, kind, data):
        msg = f"event: {kind}\ndata: {json.dumps(data)}\n\n".encode()
        for q in list(self.clients):
            q.put(msg)

    def start(self, i):
        poke = threading.Event()
        self.pokes.append(poke)
        threading.Thread(target=(self.demo if DEMO else self.watch), args=(i, poke), daemon=True).start()

    def watch(self, i, poke):
        heard, first = set(), True
        while True:
            room = self.rooms[i]
            try:
                out = run(room, f"claude agents --json; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null; echo '@@burrow@@'; {PEERS}")
                agents, settings, peers = (out.split("@@burrow@@", 2) + ["", ""])[:3]
                bots = parse_agents(agents)
                try:
                    light = str(json.loads(settings.strip()).get("theme", "")).startswith("light")
                except Exception:
                    light = False
                talks = parse_peers(peers, bots)
                for b in bots:
                    if b["background"] and b["cwd"] and b["cwd"] not in room.setdefault("dirs", []):
                        room["dirs"].append(b["cwd"])
                        save_rooms(self.rooms)
                with self.lock:
                    self.feeds[i] = {"bots": bots, "light": light, "error": None}
                for msg, a, b in talks:
                    if msg not in heard:
                        heard.add(msg)
                        if not first:
                            self.push("talk", {"room": i, "from": a, "to": b})
                first = False
            except Exception as e:
                with self.lock:
                    self.feeds[i] = {**self.feeds[i], "error": str(e)[:300]}
            self.push("state", self.state())
            t = time.time()
            while not poke.wait(0.1) and time.time() - t < (3 if self.focused else 15):
                pass
            poke.clear()

    def demo(self, i, poke):
        bots = [("burrow", "burrow"), ("burrow", "talk-ui"), ("burrow", "icons"), ("hexant", "hexant"),
                ("hexant", "ants-2"), ("website", "website"), ("", "home"), ("football", "football")]
        if i == 1:
            bots = [("odyssey", "train"), ("odyssey", "eval"), ("", "home")]
        bots = [{"id": f"demo{i}{k:03}", "name": n, "cwd": f"{HOME}/projects/{d}" if d else HOME, "busy": k % 3 == 0,
                 "background": not (i == 0 and k == 6), "started": k, "pid": 1000 + k, "session": f"demo{k}"}
                for k, (d, n) in enumerate(bots)]
        while True:
            for b in bots:
                if random.random() < 1 / 12:
                    b["busy"] = not b["busy"]
            if random.random() < 1 / 8:
                a, b = random.sample(bots, 2)
                self.push("talk", {"room": i, "from": a["id"], "to": b["id"]})
            with self.lock:
                self.feeds[i] = {"bots": [dict(b) for b in bots], "light": False, "error": None}
            self.push("state", self.state())
            poke.wait(3)
            poke.clear()


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


def terminal(sock, room, bot):
    """`claude attach <id>` in a pty, both ways over the websocket; closing it only ends the attach"""
    pid, fd = pty.fork()
    if pid == 0:
        os.environ.update(TERM="xterm-256color", COLORTERM="truecolor")
        a = argv(room, f"exec claude attach {sq(bot)}", True)
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


def upload(room, name, data):
    """a file dropped on a session: kept here, and copied over to a remote machine; returns its path there"""
    stamp = str(int(time.time() * 1000))
    d = os.path.join(CACHE, "drop", stamp)
    os.makedirs(d, exist_ok=True)
    local = os.path.join(d, name)
    with open(local, "wb") as f:
        f.write(data)
    ssh = room.get("ssh")
    if not ssh or ssh == "@demo":
        return local
    there = run(room, f'd="$HOME/.cache/burrow/drop/{stamp}"; mkdir -p "$d" && echo "$d"').strip() + "/" + name
    # scp speaks SFTP now: the remote path is taken literally, no quoting
    p = subprocess.run(["scp"] + ssh_opts() + ["-q", local, f"{ssh}:{there}"], capture_output=True, text=True)
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
        try:
            room = hub.rooms[int(q.get("room", 0))]
        except (ValueError, IndexError):
            return self.reply(404, {"error": "no such room"})
        if not key or not q.get("id"):
            return self.reply(400, {"error": "a websocket for a bot"})
        accept = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        self.wfile.flush()
        terminal(self.connection, room, q["id"])
        self.close_connection = True

    def do_POST(self):
        u = urlparse(self.path)
        n = int(self.headers.get("Content-Length") or 0)
        if u.path == "/api/upload":
            q = {k: v[0] for k, v in parse_qs(u.query).items()}
            try:
                return self.reply(200, {"path": upload(hub.rooms[int(q.get("room", 0))], os.path.basename(q.get("name") or "file"), self.rfile.read(n))})
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
                for p in hub.pokes:
                    p.set()
            return {}
        if path == "/api/room":              # a machine: a name, and an ssh destination if it isn't this one
            name, ssh = (b.get("name") or "").strip(), (b.get("ssh") or "").strip()
            if not (name or ssh):
                raise RuntimeError("a name, or where to ssh to")
            r = {"name": name or ssh, "dirs": []}
            if ssh:
                r["ssh"] = ssh
                run(r, "true", timeout=20)
            with hub.lock:
                hub.rooms.append(r)
                hub.feeds.append({})
            save_rooms(hub.rooms)
            hub.start(len(hub.rooms) - 1)
            return {"room": len(hub.rooms) - 1}
        room = hub.rooms[int(b.get("room", 0))]
        poke = hub.pokes[int(b.get("room", 0))]
        if path == "/api/dir":               # a project: a working directory there
            d = (b.get("dir") or "").strip()
            if room.get("ssh") == "@demo":
                d = d.replace("~", HOME, 1)
            else:
                d = run(room, f"cd {sq(d)} && pwd -P", timeout=20).strip()
            if d not in room.setdefault("dirs", []):
                room["dirs"].append(d)
                save_rooms(hub.rooms)
            poke.set()
            return {"dir": d}
        if path == "/api/forget":
            d = b.get("dir")
            room["dirs"] = [x for x in room.get("dirs", []) if x != d]
            save_rooms(hub.rooms)
            poke.set()
            return {}
        if path == "/api/spawn":             # a new bot in a project; returns its id
            if room.get("ssh") == "@demo":
                return {"error": "demo bots can't be started"}
            name = (b.get("name") or "").strip()
            out = run(room, f"cd {sq(b['dir'])} && claude --bg{' -n ' + sq(name) if name else ''} 2>&1")
            # "backgrounded · 7ebbc505 · name (idle — send a prompt to start)", maybe after "Starting background service…"
            line = next((l for l in out.splitlines() if l.strip().startswith("backgrounded")), None)
            if not line or len(line.split("·")) < 2:
                raise RuntimeError(out.strip())
            poke.set()
            return {"id": line.split("·")[1].strip()}
        if path == "/api/stop":
            if room.get("ssh") == "@demo":
                return {}
            run(room, f"claude stop {sq(b['id'])}")
            poke.set()
            return {}
        raise RuntimeError("no such thing: " + path)


if __name__ == "__main__":
    for i in range(len(hub.rooms)):
        hub.start(i)
    srv = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    srv.daemon_threads = True
    print(f"burrow: http://localhost:{PORT}", flush=True)
    srv.serve_forever()
