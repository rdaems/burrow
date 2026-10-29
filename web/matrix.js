// burrow as the operator sees it (matrix.html). The same server and the same sessions as the room (index.html), another
// face: every session a lane of falling code, made of what it said and did (/api/stream). How it falls is how it is:
// fast and bright, it works; frozen and pulsing white, it needs you; a slow dim drip, it is idle; nearly dark, its
// process is off or it is over. Lanes stand together by project (a desk), a machine out of reach is "signal lost".
// A message from one session to another streaks across. A click on a lane jacks in: the session's own terminal
// (`claude attach`, as in the room) on the right. Under it all the operator's line: ls, jack, load, tell, stop, ...
"use strict";

const $ = (s) => document.querySelector(s);
const api = async (path, body) => (await fetch(path, { method: "POST", body: JSON.stringify(body || {}) })).json();
const esc = (t) => String(t ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
const enc = new TextEncoder(), dec = new TextDecoder();

let state = null, digest = null;

// ---- what a session is, in words
const machineOf = (mid) => ((state && state.machines) || []).find((m) => m.id === (mid || ""));
const lost = (b) => { const m = machineOf(b.m); return !!(m && m.error); };
function stateOf(b) {
  if (lost(b)) return "lost";
  if (b.waiting) return "need";
  if (b.busy) return "work";
  if (b.over) return "over";
  if (!b.alive) return "off";
  return "idle";
}
const WORDS = { work: "WORKING", need: "NEEDS YOU", idle: "IDLE", off: "IDLE, PROCESS OFF", over: "OVER", lost: "SIGNAL LOST" };
const wordOf = (b) => (b.over && !lost(b) ? b.over.toUpperCase() : WORDS[stateOf(b)]);
const deskOf = (unit) => ((state && state.desks) || []).find((d) => d.unit === unit);
// a folder's last part; the home folder is "home", as the room calls it
const base = (d) => { const n = String(d || "").replace(/\/+$/, "").split("/"); return d === "~" || d === "/root" || (n.length === 3 && (n[1] === "Users" || n[1] === "home")) ? "home" : n.pop() || d || "~"; };
const tilde = (d) => String(d || "").replace(/^\/(Users|home)\/[^/]+/, "~");
const projectOf = (b) => { const d = deskOf(b.unit); return (d && d.name) || base(d ? d.dir : b.cwd); };
const machineName = (mid) => { const m = machineOf(mid); return m ? m.name : mid; };
const ago = (ms) => { const s = Math.max(0, (Date.now() - ms) / 1000); return s < 90 ? Math.round(s) + "s" : s < 5400 ? Math.round(s / 60) + "m" : s < 172800 ? Math.round(s / 3600) + "h" : Math.round(s / 86400) + "d"; };

// ---- the rain
const cv = $("#rain"), box = $("#rainbox"), labels = $("#labels"), info = $("#info");
const ctx = cv.getContext("2d");
const FONT = 14, CH = 16;                 // the glyphs' size and a row's height (CSS pixels)
let CW = 9, cols = 0, rows = 0, dpr = 1;  // a column's width, and how many there are
let F1 = "14px monospace", F7 = F1;       // the glyphs' font, and the names' (bold)
const KANA = Array.from({ length: 56 }, (_, i) => String.fromCharCode(0xff66 + i)).join("") + "0123456789";
const kana = () => KANA[(Math.random() * KANA.length) | 0];
// how each state rains: drops a column a second, their speed (rows a second), how fast the trail fades, how bright
const RAIN = {
  work: { spawn: 1.1, speed: [14, 24], fade: 1.1, bright: 1 },
  need: { spawn: 0, speed: [0, 0], fade: 0, bright: 1 },
  idle: { spawn: 0.1, speed: [4, 7], fade: 0.7, bright: 0.55 },
  off: { spawn: 0.04, speed: [3, 5], fade: 0.7, bright: 0.32 },
  over: { spawn: 0.025, speed: [3, 4], fade: 0.8, bright: 0.24 },
  lost: { spawn: 0, speed: [0, 0], fade: 0.25, bright: 0.2 },
  ambient: { spawn: 0.12, speed: [6, 13], fade: 1.2, bright: 0.2 },
};
const LANE = 8;                           // columns a lane, at most (as many as fit)
const lanes = new Map();                  // session id → {b, x (its first column), cells: [col][row] {c, a, h}, drops, text, pos, ...}
let ambient = [];                         // the columns that are nobody's: [{x, cells, drops}]
let groups = [];                          // [{unit, x0, x1, label, lost}]
const streaks = [];                       // messages on their way: {from, to, t0}
const flashes = new Map();                // id → when it was jacked into (its lane flashes white)

function measure() {
  dpr = window.devicePixelRatio || 1;
  const w = box.clientWidth, h = box.clientHeight;
  cv.width = Math.round(w * dpr); cv.height = Math.round(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const mono = getComputedStyle(document.body).getPropertyValue("--mono");
  F1 = `${FONT}px ${mono}`; F7 = `700 ${FONT}px ${mono}`;
  ctx.font = F1;
  CW = Math.ceil(ctx.measureText("M").width) + 1;
  cols = Math.max(1, Math.floor(w / CW)); rows = Math.max(1, Math.floor((h - 26) / CH));   // (the bottom row: the projects' names)
  for (const l of lanes.values()) l.cells = l.cells.map((c) => fit(c));
  arrange();
}
const column = () => Array.from({ length: rows }, () => ({ c: "", a: 0, h: 0 }));
const fit = (c) => (c.length === rows ? c : c.length > rows ? c.slice(c.length - rows) : [...Array.from({ length: rows - c.length }, () => ({ c: "", a: 0, h: 0 })), ...c]);

// where every lane stands: by project (the desks' order), a gap between projects; what is left over is ambient rain.
// Too many for the width: the gaps shrink, then a lane is one column
function arrange() {
  if (!state) return;
  const bots = state.bots || [], order = (state.desks || []).map((d) => d.unit);
  const units = [...new Set([...order.filter((u) => bots.some((b) => b.unit === u)), ...bots.map((b) => b.unit)])];
  const byUnit = units.map((u) => [u, bots.filter((b) => b.unit === u)]);
  let gap = 2, ggap = 5;
  const room = () => cols - 4 - (bots.length - byUnit.length) * gap - Math.max(0, byUnit.length - 1) * ggap;
  if (room() < bots.length * 3) { ggap = 3; gap = 1; }
  if (room() < bots.length * 2) { gap = 0; ggap = 1; }
  const lane = Math.max(1, Math.min(LANE, Math.floor(room() / Math.max(1, bots.length))));
  const need = () => bots.length * lane + (bots.length - byUnit.length) * gap + Math.max(0, byUnit.length - 1) * ggap;
  const used = Math.min(cols, need()), x0 = Math.max(0, Math.floor((cols - used) / 2));
  groups = [];
  let x = x0;
  for (const [u, bs] of byUnit) {
    const g = { unit: u, x0: x, x1: x, label: "", lost: false };
    for (const b of bs) {
      let l = lanes.get(b.id);
      const fresh = !l;
      if (!l) { l = { id: b.id, cells: [], drops: [], text: "", pos: 0, textFor: undefined, asked: 0, born: performance.now() }; lanes.set(b.id, l); }
      l.b = b; l.x = x; l.w = lane; l.gone = 0;
      if (fresh) burst(l);
      while (l.cells.length < lane) l.cells.push(column());
      l.cells.length = lane;
      l.cells = l.cells.map((c) => fit(c));
      x += lane + gap;
    }
    g.x1 = x - gap;
    const b0 = bs[0], mine = (b0.m || "") === "";
    g.label = projectOf(b0) + (mine ? "" : " @" + machineName(b0.m));
    g.lost = lost(b0);
    groups.push(g);
    x += ggap - gap;
  }
  // the gone: fall apart where they stood, then go
  for (const [id, l] of lanes) if (!bots.some((b) => b.id === id)) { if (!l.gone) l.gone = performance.now(); }
  const taken = new Set();
  for (const l of lanes.values()) if (!l.gone) for (let i = 0; i < l.w; i++) taken.add(l.x + i);
  for (const g of groups) for (let c = g.x0 - 1; c <= g.x1; c++) taken.add(c);
  const keep = new Map(ambient.map((a) => [a.x, a]));
  ambient = [];
  for (let c = 0; c < cols; c++) if (!taken.has(c)) ambient.push(keep.has(c) ? { ...keep.get(c), cells: fit(keep.get(c).cells) } : { x: c, cells: column(), drops: [] });
  labels.innerHTML = groups.map((g) => `<div class="grp${g.lost ? " lost" : ""}" style="left:${g.x0 * CW}px;width:${Math.max(1, g.x1 - g.x0) * CW}px" title="${esc(g.label)}">${esc(g.lost ? g.label + " · signal lost" : g.label)}</div>`).join("");
}
// a lane comes in, or is jacked into: a drop down each of its columns at once
function burst(l) { for (let i = 0; i < (l.w || LANE); i++) l.drops.push({ col: i, y: -Math.random() * 4, v: 26 + Math.random() * 10 }); }

// its next glyph: what it said and did, letter by letter; a space is a glyph of the rain
function next(l) {
  if (!l.text) return kana();
  const ch = l.text[l.pos % l.text.length];
  l.pos++;
  return /\s/.test(ch) ? kana() : ch;
}

function rain(cells, drops, R, dt, src) {
  for (const d of drops) {
    const before = Math.floor(d.y);
    d.y += d.v * dt;
    for (let r = Math.max(0, before + 1); r <= Math.floor(d.y) && r < rows; r++) {
      const cell = (cells[d.col || 0] || [])[r];
      if (cell) { cell.c = src(); cell.a = R.bright; cell.h = 1; }
    }
  }
  for (let i = drops.length - 1; i >= 0; i--) if (drops[i].y > rows + 2) drops.splice(i, 1);
  const k = Math.exp(-dt * R.fade), kh = Math.exp(-dt * 9);
  for (const col of cells) for (const cell of col) { if (cell.a) { cell.a *= k; if (cell.a < 0.015) { cell.a = 0; cell.c = ""; } } cell.h *= kh; }
}

let last = performance.now();
function frame(now) {
  const dt = Math.min(0.1, (now - last) / 1000) * (still ? 0.25 : 1);
  last = now;
  const W = box.clientWidth, H = box.clientHeight;
  ctx.fillStyle = "#000300"; ctx.fillRect(0, 0, W, H);
  ctx.textBaseline = "top";
  // nobody's columns: faint, so the screen is never quite still
  for (const a of ambient) {
    const R = RAIN.ambient;
    if (Math.random() < R.spawn * dt) a.drops.push({ col: 0, y: -1, v: R.speed[0] + Math.random() * (R.speed[1] - R.speed[0]) });
    rain([a.cells], a.drops, R, dt, kana);
    paint(a.x, a.cells[0], 0, 0);
  }
  const hov = hovered && lanes.get(hovered);
  for (const l of lanes.values()) {
    const st = l.gone ? "over" : stateOf(l.b), R = RAIN[st];
    if (l.gone) {                            // deleted: its glyphs drop out, and it goes
      const t = (now - l.gone) / 1200;
      if (t > 1) { lanes.delete(l.id); arrange(); continue; }
      for (const col of l.cells) for (const cell of col) if (Math.random() < dt * 3) { cell.a = 0; cell.c = ""; }
    } else if (R.spawn) for (let i = 0; i < l.w; i++) if (Math.random() < R.spawn * dt) l.drops.push({ col: i, y: -1, v: R.speed[0] + Math.random() * (R.speed[1] - R.speed[0]) });
    if (st === "need") {                     // frozen: a page of what it last said stands still in its lane, white
      if (l.was !== "need") { l.drops = []; for (const col of l.cells) for (const cell of col) { cell.c = next(l); cell.a = 0.6 + Math.random() * 0.4; cell.h = 0; } }
    } else rain(l.cells, l.drops, R, dt, () => next(l));
    l.was = st;
    const pulse = st === "need" ? 0.5 + 0.5 * Math.sin(now / 260) : 0;
    const fl = flashes.has(l.id) ? Math.max(0, 1 - (now - flashes.get(l.id)) / 500) : 0;
    const lit = (hov === l ? 0.35 : 0) + fl;
    const nc = Math.floor((l.w - 1) / 2), nz = rows - nameOf(l).length - 1;
    l.cells.forEach((col, i) => paint(l.x + i, col, Math.max(pulse, lit), st === "need" ? 1 : 0, i === nc ? nz : rows));
    // its name, down the bottom of its lane, as the rain writes
    nameDown(l, st, hov === l);
  }
  for (let i = streaks.length - 1; i >= 0; i--) if (!streak(streaks[i], now)) streaks.splice(i, 1);
  requestAnimationFrame(frame);
}
// a column's glyphs: green, the head of a drop white, a lane that needs you pulsing towards white
function paint(cx, col, white, need, upto = rows) {
  const x = cx * CW;
  for (let r = 0; r < Math.min(upto, col.length); r++) {
    const cell = col[r];
    if (!cell.c || cell.a < 0.02) continue;
    const w = Math.min(1, Math.max(cell.h, need ? 0.75 + 0.25 * white : white * 0.6));
    const a = cell.a;              // green, towards white as w goes up
    ctx.fillStyle = `rgb(${Math.round(w * 233 * a)},${Math.round(255 * a)},${Math.round(((1 - w) * 65 + w * 238) * a)})`;
    ctx.fillText(cell.c, x, r * CH, CW);
  }
}
const nameOf = (l) => (l.b.name || l.id).slice(0, Math.min(14, Math.floor(rows / 3)));
function nameDown(l, st, hov) {
  const name = nameOf(l), y0 = (rows - name.length) * CH, x = (l.x + Math.floor((l.w - 1) / 2)) * CW;
  const need = st === "need", a = hov ? 1 : need ? 0.6 + 0.4 * Math.sin(performance.now() / 260) : st === "work" ? 0.9 : st === "idle" ? 0.6 : 0.4;
  ctx.fillStyle = need ? `rgba(233,255,238,${a})` : st === "lost" ? `rgba(255,95,86,${a})` : `rgba(0,255,65,${a})`;
  ctx.font = F7;
  for (let i = 0; i < name.length; i++) ctx.fillText(name[i], x, y0 + i * CH, CW);
  ctx.font = F1;
}
// a message: bright glyphs leap from the sender's lane to the receiver's, along the top
function streak(s, now) {
  const a = lanes.get(s.from), b = lanes.get(s.to), t = (now - s.t0) / 900;
  if (!a || !b || t > 1.4) return false;
  const x0 = (a.x + 0.5) * CW, x1 = (b.x + 0.5) * CW, y = CH * 2;
  const head = Math.min(1, t), tail = Math.max(0, t - 0.4);
  const n = Math.max(1, Math.round(Math.abs(x1 - x0) / CW));
  for (let i = Math.floor(tail * n); i <= Math.floor(head * n); i++) {
    const k = i / n, fade = 1 - (head - k) * 2.5;
    if (fade <= 0) continue;
    ctx.fillStyle = `rgba(${i === Math.floor(head * n) ? 233 : 0},255,${i === Math.floor(head * n) ? 238 : 65},${Math.min(1, fade)})`;
    ctx.fillText(kana(), x0 + (x1 - x0) * k, y + Math.sin(k * Math.PI) * -CH, CW);
  }
  if (t >= 1 && !s.hit) { s.hit = true; flashes.set(s.to, now); burst(b); }
  return true;
}

// ---- pointing at the rain: a lane under the pointer shows who it is; a click jacks in
let hovered = null;
function laneAt(px) {
  const c = Math.floor(px / CW);
  for (const l of lanes.values()) if (!l.gone && c >= l.x - (l.w === 1 ? 0 : 0) && c < l.x + l.w) return l;
  return null;
}
box.addEventListener("pointermove", (e) => {
  const r = box.getBoundingClientRect(), l = laneAt(e.clientX - r.left);
  hovered = l ? l.id : null;
  box.style.cursor = l ? "pointer" : "default";
  if (!l) { info.hidden = true; return; }
  showInfo(l.b, e.clientX - r.left, e.clientY - r.top);
});
box.addEventListener("pointerleave", () => { hovered = null; info.hidden = true; });
box.addEventListener("click", (e) => {
  const r = box.getBoundingClientRect(), l = laneAt(e.clientX - r.left);
  if (l) jackIn(l.b);
  else cmd.focus();
});
function showInfo(b, x, y) {
  const n = digest && digest.sessions && digest.sessions[b.id], st = stateOf(b);
  info.innerHTML = `<div class="nm">${esc(b.name)}</div><div class="st${st === "need" ? " need" : ""}">${esc(wordOf(b))}</div>` +
    `<div><span class="k">project</span> ${esc(projectOf(b))} <span class="k">on</span> ${esc(machineName(b.m))}</div>` +
    `<div><span class="k">folder</span> ${esc(tilde(b.cwd))}</div>` +
    `<div><span class="k">since</span> ${esc(ago(b.started))} ago${b.rc ? ' <span class="k">· in the Claude app too</span>' : ""}</div>` +
    (n && n.headline ? `<p>${esc(n.headline)}</p>` : b.detail ? `<p>${esc(b.detail)}</p>` : "");
  info.hidden = false;
  const W = box.clientWidth, H = box.clientHeight, w = info.offsetWidth, h = info.offsetHeight;
  info.style.left = Math.max(6, Math.min(W - w - 6, x + 18)) + "px";
  info.style.top = Math.max(6, Math.min(H - h - 40, y - 20)) + "px";
}

// ---- jacking in: the session's own terminal (claude attach), the room's terminal to the letter
const MATRIX = { background: "#000a02", foreground: "#b6f7c4", cursor: "#00ff41", cursorAccent: "#000a02", selectionBackground: "#00ff4144",
  black: "#0a1f0f", red: "#ff5f56", green: "#00ff41", yellow: "#d7ff5f", blue: "#3fbf8f", magenta: "#8fe3b0", cyan: "#5fffd7", white: "#c8f7d0",
  brightBlack: "#2f6b3c", brightRed: "#ff8a80", brightGreen: "#7dff9e", brightYellow: "#efffa0", brightBlue: "#7fe0b8", brightMagenta: "#c0f5d2", brightCyan: "#a0ffe8", brightWhite: "#e9ffee" };
const jack = $("#jack"), jterm = $("#jterm");
let cur = null;                           // what is jacked into: {kind: bot | shell, m, id | dir, term, ws, fit, ...}
const REPORT = /^(?:\x1b\[[IO]|\x1b\[<[\d;]*[Mm]|\x1b\[M[\s\S]{3}|\x1b\][\d;]*[^\x07\x1b]*(?:\x07|\x1b\\)|\x1b\[[\d;?]*[Rcn])+$/;
const CMD_KEYS = { ArrowLeft: "\x01", ArrowRight: "\x05", Backspace: "\x15" };
function jackIn(b) {
  if (cur && cur.kind === "bot" && cur.id === b.id) { cur.term.focus(); return cur; }
  flashes.set(b.id, performance.now());
  const l = lanes.get(b.id);
  if (l) burst(l);
  return open({ kind: "bot", m: b.m || "", id: b.id });
}
function jackShell(m, dir) {
  if (cur && cur.kind === "shell" && cur.m === m && cur.dir === dir) { cur.term.focus(); return cur; }
  const fresh = !((state && state.shells) || []).some((s) => s.m === m && s.dir === dir);
  return open({ kind: "shell", m, dir, fresh });
}
function open(t) {
  jackOut();
  Object.assign(t, { ws: null, typed: [], tail: "" });
  jack.hidden = false; jack.classList.remove("in"); void jack.offsetWidth; jack.classList.add("in");
  measure();
  // links open in a new tab on a click: the ones a program marks (OSC 8) and plain URLs in the text
  const openLink = (_, uri) => { if (/^https?:\/\//i.test(uri)) window.open(uri, "_blank", "noopener"); };
  const term = t.term = new Terminal({ fontFamily: '"Burrow Mono", "JetBrains Mono", Menlo, monospace', fontSize: 13, lineHeight: 1.15,
    cursorBlink: true, allowProposedApi: true, theme: MATRIX, scrollback: 5000, macOptionIsMeta: true, macOptionClickForcesSelection: true,
    linkHandler: { activate: openLink } });
  const fit = t.fit = new FitAddon.FitAddon();
  term.loadAddon(fit);
  term.loadAddon(new WebLinksAddon.WebLinksAddon(openLink));
  // copying: Claude Code copies a drag itself (pbcopy here, OSC 52 from another machine: put on the clipboard here,
  // never read from it); Option+drag selects in the terminal itself
  term.loadAddon(new ClipboardAddon.ClipboardAddon(undefined, {
    readText: async () => "",
    writeText: async (_, text) => { try { if (window.ipc) window.ipc.postMessage("clip:" + text); else await navigator.clipboard.writeText(text); } catch {} },
  }));
  term.onData((d) => { if (t.ws && t.ws.readyState === 1) t.ws.send(enc.encode(d)); if (!REPORT.test(d)) typing(t); });
  // shift+enter a new line (ESC CR); ⌘← ⌘→ ⌘⌫ as Ghostty sends them: Ctrl+A, Ctrl+E, Ctrl+U
  term.attachCustomKeyEventHandler((e) => {
    if (e.metaKey && e.key.toLowerCase() === "w") return false;
    const c = e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey && CMD_KEYS[e.key];
    if (c) { e.preventDefault(); if (e.type === "keydown") term.input(c); return false; }
    if (e.key !== "Enter" || !e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) return true;
    e.preventDefault();
    if (e.type === "keydown") term.input("\x1b\r");
    return false;
  });
  term.onBinary((d) => t.ws && t.ws.readyState === 1 && t.ws.send(Uint8Array.from(d, (c) => c.charCodeAt(0))));
  term.onResize(({ cols, rows }) => t.ws && t.ws.readyState === 1 && t.ws.send(JSON.stringify({ size: [cols, rows] })));
  const el = document.createElement("div");
  el.className = "term";
  jterm.innerHTML = "";
  jterm.appendChild(el);
  term.open(el);
  cur = t;
  who();
  t.ro = new ResizeObserver(() => { if (t.ws) fit.fit(); });
  t.ro.observe(el);
  requestAnimationFrame(() => { fit.fit(); connect(t); term.focus(); });
  return t;
}
// `exit` in an attached session drops into Claude Code's agent view: here the terminal closes instead, the session goes on
const agentView = (t, b) => {
  t.tail = (t.tail + dec.decode(b, { stream: true })).replace(/\x1b\[[0-9;?]*[A-Za-z]/g, "").slice(-300);
  return /describe a task for a new session|enter to return · space to reply/.test(t.tail);
};
function connect(t) {
  const shell = t.kind === "shell", term = t.term;
  const s = new WebSocket(shell ? `ws://${location.host}/shell?m=${encodeURIComponent(t.m)}&dir=${encodeURIComponent(t.dir)}`
    : `ws://${location.host}/term?m=${encodeURIComponent(t.m)}&id=${encodeURIComponent(t.id)}`);
  s.binaryType = "arraybuffer";
  const since = performance.now();
  const flush = (n) => {
    if (t.ws !== s) return;
    if (!term.modes.bracketedPasteMode && n < 18) return setTimeout(flush, 500, n + 1);
    for (const x of t.typed.splice(0)) s.send(enc.encode(x));
  };
  s.onopen = () => { s.send(JSON.stringify({ size: [term.cols, term.rows] })); setTimeout(flush, 1000, 0); };
  s.onmessage = (e) => { const b = new Uint8Array(e.data); if (!shell && agentView(t, b)) jackOut(); else term.write(b); };
  // the attach ended (the session or the shell stopped): out with it; one that ends at once failed, and stays to be read
  s.onclose = () => { if (t.ws === s) { t.ws = null; if ((shell && !t.fresh) || performance.now() - since > 3000) { if (cur === t) jackOut(); } } };
  t.ws = s;
}
function jackOut() {
  const t = cur;
  if (!t) return;
  cur = null;
  if (t.ro) t.ro.disconnect();
  if (t.ws) { const s = t.ws; t.ws = null; s.close(); }
  if (t.term) t.term.dispose();
  jterm.innerHTML = "";
  jack.hidden = true;
  measure();
}
$("#jexit").onclick = () => { jackOut(); cmd.focus(); };
function who() {
  if (!cur) return;
  const m = machineName(cur.m), b = cur.kind === "bot" && byId(cur.id);
  $("#jname").textContent = cur.kind === "shell" ? base(cur.dir) : b ? b.name : cur.id;
  $("#jsub").textContent = cur.kind === "shell" ? `your shell in ${tilde(cur.dir)} · ${m}` : b ? `${wordOf(b).toLowerCase()} · ${projectOf(b)} · ${m}` : m;
  const app = $("#japp");
  app.hidden = !(b && b.rc);
  app.onclick = () => window.open("https://claude.ai/code/session_" + b.rc.replace(/^cse_/, ""), "_blank", "noopener");
}
let typedAt = 0;
function typing(t) { typedAt = performance.now(); if (t.kind === "bot") { const l = lanes.get(t.id); if (l && Math.random() < 0.3) l.drops.push({ col: 0, y: -1, v: 30 }); } }
const say = (t, x) => { if (t.ws && t.ws.readyState === 1) t.ws.send(enc.encode(x)); else t.typed.push(x); };

// ---- the operator's line
const cmd = $("#cmd"), out = $("#out"), hint = $("#hint");
const HIST = "burrow.matrix.history";
let hist = [];
try { hist = JSON.parse(localStorage.getItem(HIST) || "[]"); } catch {}
let hi = hist.length;
function print(text, cls) {
  const d = document.createElement("div");
  if (cls) d.className = cls;
  d.textContent = text;
  out.appendChild(d);
  while (out.children.length > 40) out.firstChild.remove();
  while (out.offsetHeight > box.clientHeight * 0.6 && out.children.length > 1) out.firstChild.remove();
  d.dataset.t = Date.now();
}
setInterval(() => {                       // what was printed goes after a while, unless the line is in use
  if (document.activeElement === cmd) return;
  for (const d of [...out.children]) if (Date.now() - d.dataset.t > 20000) d.remove();
}, 2000);
const bots = () => (state && state.bots) || [];
const byId = (id) => bots().find((b) => b.id === id);
// a session by what was typed: its name, a unique start of it, or of its id
function find(q) {
  if (!q) return [null, "whom?"];
  const s = q.toLowerCase(), bs = bots();
  const exact = bs.filter((b) => b.name.toLowerCase() === s || b.id === q);
  if (exact.length === 1) return [exact[0]];
  const some = (exact.length ? exact : bs.filter((b) => b.name.toLowerCase().startsWith(s) || b.id.startsWith(q)));
  if (some.length === 1) return [some[0]];
  if (!some.length) return [null, `no agent called ${q}`];
  return [null, `which one: ${some.slice(0, 8).map((b) => b.name + (b.m ? "@" + machineName(b.m) : "")).join(", ")}${some.length > 8 ? ", …" : ""}`];
}
// a project (a desk) by its name, or a folder (~/x, /x) with `@machine` after it for another machine
function where(q) {
  if (!q) return [null, "where? a project, or a folder"];
  let [what, mname] = q.split("@");
  const ms = (state && state.machines) || [];
  const m = mname ? ms.find((x) => x.name.toLowerCase() === mname.toLowerCase() || x.id === mname) : null;
  if (mname && !m) return [null, `no machine called ${mname}`];
  if (/^[~/]/.test(what)) return [{ m: m ? m.id : "", dir: what.replace(/(.)\/+$/, "$1") }];
  const ds = ((state && state.desks) || []).filter((d) => (!m || d.m === m.id) && ((d.name || base(d.dir)).toLowerCase() === what.toLowerCase()));
  if (ds.length === 1) return [{ m: ds[0].m, dir: ds[0].dir }];
  if (ds.length > 1) return [null, `which one: ${ds.map((d) => (d.name || base(d.dir)) + "@" + machineName(d.m)).join(", ")}`];
  return [null, `no project called ${what} (a folder starts with ~ or /)`];
}
const COMMANDS = {
  help: { args: "", say: "this" },
  ls: { args: "[what]", say: "every agent, by project; or those whose name, project or state has it" },
  jack: { args: "<agent>", say: "jack in: its terminal (a click on its lane does the same)" },
  exit: { args: "", say: "out of the terminal; the session goes on" },
  rabbit: { args: "", say: "follow the white rabbit: the next agent that needs you" },
  load: { args: "<project|~/folder>[@machine]", say: "a new agent there, and in" },
  tell: { args: "<agent> <message>", say: "type a message into an agent's session" },
  reset: { args: "<agent>", say: "/clear an idle agent: it forgets its conversation" },
  stop: { args: "<agent>", say: "stop its process; it stays, idle" },
  delete: { args: "<agent>", say: "stop it and take it out for good (asks first; its transcript stays for --resume)" },
  shell: { args: "<project|~/folder>[@machine]", say: "your own shell there" },
  limits: { args: "", say: "what is left of the five hours and the week" },
  nodes: { args: "", say: "the machines" },
  room: { args: "", say: "back to the 16-bit room" },
  cls: { args: "", say: "clear these lines" },
};
const ALIAS = { "?": "help", agents: "ls", j: "jack", in: "jack", open: "jack", new: "load", spawn: "load", out: "exit", q: "exit", rm: "delete", clear: "reset" };
let confirm = null;                       // a delete asked: {id, until}
async function run(line) {
  const raw = line.trim();
  if (!raw) return;
  print("> " + raw, "echo");
  if (confirm) {
    const c = confirm; confirm = null;
    if (/^y(es)?$/i.test(raw) && Date.now() < c.until) {
      const b = byId(c.id);
      if (!b) return print("it is gone already");
      if (cur && cur.kind === "bot" && cur.id === b.id) jackOut();
      const r = await api("/api/stop", { m: b.m || "", id: b.id, rm: true });
      return r.error ? print(r.error, "err") : print(`${b.name} is deleted`);
    }
    if (/^n(o)?$/i.test(raw)) return print("kept");
  }
  let [verb, ...rest] = raw.split(/\s+/);
  verb = verb.toLowerCase();
  if (verb === "jack" && rest[0] === "in") rest.shift();
  if (verb === "jack" && rest[0] === "out") { verb = "exit"; rest = []; }
  verb = ALIAS[verb] || verb;
  const arg = rest.join(" ");
  switch (verb) {
    case "help":
      for (const [k, c] of Object.entries(COMMANDS)) print(`${(k + " " + c.args).padEnd(38)} ${c.say}`);
      return print("tab completes, ↑ ↓ for what was typed before; a click on a lane jacks in", "echo");
    case "ls": {
      const q = arg.toLowerCase(), bs = bots().filter((b) => !q || [b.name, projectOf(b), wordOf(b), machineName(b.m)].some((x) => String(x).toLowerCase().includes(q)));
      if (!bs.length) return print(q ? "none" : "no agents yet: load <project>");
      const units = [...new Set(bs.map((b) => b.unit))];
      for (const u of units) {
        const g = bs.filter((b) => b.unit === u), b0 = g[0];
        print(`${projectOf(b0)}${b0.m ? " @" + machineName(b0.m) : ""}  ${tilde(deskOf(u) ? deskOf(u).dir : b0.cwd)}`, "echo");
        for (const b of g) print(`  ${b.name.padEnd(20)} ${wordOf(b).padEnd(18)} ${b.detail || ""}`.trimEnd(), stateOf(b) === "need" ? "hot" : "");
      }
      return;
    }
    case "jack": { const [b, e] = find(arg); return b ? jackIn(b) : print(e, "err"); }
    case "exit": if (!cur) return print("not jacked in"); jackOut(); return;
    case "rabbit": {
      const need = bots().filter((b) => b.waiting && !lost(b));
      if (!need.length) return print("no rabbit: nobody needs you");
      const i = cur && cur.kind === "bot" ? need.findIndex((b) => b.id === cur.id) : -1;
      const b = need[(i + 1) % need.length];
      print(`follow the white rabbit: ${b.name}`, "hot");
      return jackIn(b);
    }
    case "load": {
      const [w, e] = where(arg);
      if (!w) return print(e, "err");
      print(`loading an agent in ${tilde(w.dir)}${w.m ? " on " + machineName(w.m) : ""}…`);
      const r = await api("/api/spawn", { m: w.m, dir: w.dir });
      if (r.error && /not trusted/i.test(r.error)) return trustFirst(w);
      if (r.error) return print(r.error, "err");
      print("loaded", "hot");
      const b = byId(r.id) || { id: r.id, m: w.m, name: r.id };
      return jackIn(b);
    }
    case "tell": {
      const [who_, ...msg] = rest, [b, e] = find(who_);
      if (!b) return print(e, "err");
      if (!msg.length) return print("tell it what?", "err");
      const r = await api("/api/tell", { m: b.m || "", id: b.id, text: msg.join(" ") });
      return r.error ? print(r.error, "err") : print(`told ${b.name}`);
    }
    case "reset": {
      const [b, e] = find(arg);
      if (!b) return print(e, "err");
      if (b.busy || b.waiting) return print(`${b.name} is busy: only an idle one forgets`, "err");
      const r = await api("/api/clear", { m: b.m || "", id: b.id });
      return r.error ? print(r.error, "err") : print(`${b.name}: /clear`);
    }
    case "stop": {
      const [b, e] = find(arg);
      if (!b) return print(e, "err");
      if (cur && cur.kind === "bot" && cur.id === b.id) jackOut();
      const r = await api("/api/stop", { m: b.m || "", id: b.id, rm: false });
      return r.error ? print(r.error, "err") : print(`${b.name} stopped`);
    }
    case "delete": {
      const [b, e] = find(arg);
      if (!b) return print(e, "err");
      confirm = { id: b.id, until: Date.now() + 15000 };
      return print(`delete ${b.name} for good? y / n`, "hot");
    }
    case "shell": { const [w, e] = where(arg); return w ? jackShell(w.m, w.dir) : print(e, "err"); }
    case "limits": {
      const u = (state && state.usage) || {};
      const bar = (x, what) => x ? `${what.padEnd(10)} [${"#".repeat(Math.round((100 - x.used) / 5)).padEnd(20, ".")}] ${100 - x.used}% left${x.resets ? ", new at " + x.resets : ""}` : `${what.padEnd(10)} not known yet`;
      print(bar(u.session, "five hours")); return print(bar(u.week, "the week"));
    }
    case "nodes":
      for (const m of (state && state.machines) || []) print(`${(m.name || "this computer").padEnd(20)} ${m.error ? "signal lost: " + m.error : "online"}  ${bots().filter((b) => (b.m || "") === m.id).length} agents`, m.error ? "err" : "");
      return;
    case "room": location.href = "/"; return;
    case "cls": out.innerHTML = ""; return;
    default: return print(`${verb}: no such command (help)`, "err");
  }
}
// load in a folder Claude Code does not trust yet: your shell there with `claude` in it, its trust prompt on the screen;
// the agent is asked for again every three seconds while that shell is up, then takes its place (as the room does)
async function trustFirst(w) {
  print("that folder is not trusted yet: answer claude's question in the terminal", "hot");
  const t = jackShell(w.m, w.dir);
  say(t, "claude\r");
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let k = 0; k < 100; k++) {
    await wait(3000);
    if (cur !== t) return;
    const s = await api("/api/spawn", { m: w.m, dir: w.dir });
    if (s.error && /not trusted/i.test(s.error)) continue;
    if (s.error) return print(s.error, "err");
    for (let i = 0; i < 3; i++) { say(t, "\x03"); await wait(300); }
    print("loaded", "hot");
    return jackIn(byId(s.id) || { id: s.id, m: w.m, name: s.id });
  }
}
// tab: the command, then an agent's or a project's name
function complete() {
  const v = cmd.value, parts = v.split(/\s+/), last = parts[parts.length - 1].toLowerCase();
  let pool;
  if (parts.length === 1) pool = [...Object.keys(COMMANDS), "jack in"];
  else {
    const verb = ALIAS[parts[0].toLowerCase()] || parts[0].toLowerCase();
    pool = ["load", "shell"].includes(verb) ? [...new Set(((state && state.desks) || []).map((d) => d.name || base(d.dir)))] : bots().map((b) => b.name);
    if (verb === "tell" && parts.length > 2) return;
  }
  const hits = pool.filter((x) => x.toLowerCase().startsWith(last));
  if (!hits.length) return;
  let pre = hits[0];
  for (const h of hits) while (!h.toLowerCase().startsWith(pre.toLowerCase())) pre = pre.slice(0, -1);
  parts[parts.length - 1] = hits.length === 1 ? hits[0] + " " : pre;
  cmd.value = parts.join(" ");
  if (hits.length > 1) hint.textContent = hits.slice(0, 12).join("  ");
}
cmd.addEventListener("keydown", (e) => {
  if (e.key === "Tab") { e.preventDefault(); complete(); return; }
  hint.textContent = "";
  if (e.key === "Enter") {
    const v = cmd.value;
    cmd.value = "";
    if (v.trim() && hist[hist.length - 1] !== v.trim()) { hist.push(v.trim()); hist = hist.slice(-200); try { localStorage.setItem(HIST, JSON.stringify(hist)); } catch {} }
    hi = hist.length;
    run(v).catch((err) => print(String(err && err.message || err), "err"));
  } else if (e.key === "ArrowUp") { e.preventDefault(); if (hi > 0) cmd.value = hist[--hi]; }
  else if (e.key === "ArrowDown") { e.preventDefault(); hi = Math.min(hist.length, hi + 1); cmd.value = hist[hi] || ""; }
  else if (e.key === "Escape") { cmd.value = ""; }
});
cmd.addEventListener("input", () => { hint.textContent = ""; });
// typing anywhere that is not the terminal goes to the line
document.addEventListener("keydown", (e) => {
  if (document.activeElement === cmd || (cur && jack.contains(document.activeElement))) return;
  if (e.metaKey || e.ctrlKey || e.altKey || e.key.length !== 1) return;
  cmd.focus();
});

// ---- the feed: the room's own (server.py), the same as index.html's
const es = new EventSource("/events");
es.addEventListener("state", (e) => {
  const was = state;
  state = JSON.parse(e.data);
  if (!was) measure(); else arrange();
  counts();
  who();
  if (!was && !hist.length) print("type help, or click a lane to jack in", "echo");
});
es.addEventListener("shells", (e) => { if (state) state.shells = JSON.parse(e.data); });
es.addEventListener("digest", (e) => { digest = JSON.parse(e.data); });
es.addEventListener("talk", (e) => { const t = JSON.parse(e.data); streaks.push({ from: t.from, to: t.to, t0: performance.now() }); });
const focus = () => api("/api/focus", { on: document.hasFocus() && !document.hidden });
window.addEventListener("focus", focus); window.addEventListener("blur", focus); document.addEventListener("visibilitychange", focus);
focus();

function counts() {
  const bs = bots(), n = (f) => bs.filter(f).length, ms = (state && state.machines) || [];
  const need = n((b) => b.waiting && !lost(b));
  $("#counts").innerHTML = `${bs.length} agents · ${n((b) => b.busy && !lost(b))} working · ${need ? `<b>${need} need you</b>` : "0 need you"} · ${ms.filter((m) => !m.error).length}/${ms.length} nodes`;
  const u = state.usage || {}, p = (x) => (x ? `${100 - x.used}%` : "?");
  $("#limits").textContent = `5h ${p(u.session)} left · week ${p(u.week)} left`;
}
setInterval(() => { const d = new Date(); $("#clock").textContent = String(d.getHours()).padStart(2, "0") + ":" + String(d.getMinutes()).padStart(2, "0"); }, 1000);

// the text the lanes rain: asked for the sessions whose record changed since (the server keeps it, and asks a machine at
// most every ten seconds)
async function texts() {
  const now = Date.now(), ask = [];
  for (const l of lanes.values()) if (!l.gone && l.b && l.textFor !== (l.b.updated || 0) && now - l.asked > 12000) { ask.push(l.id); l.asked = now; }
  if (!ask.length) return;
  try {
    const r = await api("/api/stream", { ids: ask });
    for (const [id, text] of Object.entries(r.texts || {})) {
      const l = lanes.get(id);
      if (l && text) { if (l.text !== text) { l.text = text; l.pos = Math.max(0, text.length - 600); } l.textFor = l.b.updated || 0; }
    }
  } catch {}
}
setInterval(texts, 4000);

window.addEventListener("resize", measure);
requestAnimationFrame(frame);
window.__matrix = { lanes, run, get state() { return state; }, jackIn, jackOut };
