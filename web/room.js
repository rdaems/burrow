// The room: one machine, as pair's office (rdaems/pair-prompting, app/floor.js): its furniture, its
// breaks, its mats, its robots (pixel.js). Every project (a working directory) is one unit: a cabinet
// with the project's screen and name at its left end, a long desktop to the right where its bots sit
// facing the screen, and an empty chair at the end (+) that starts another bot there.
// Where a bot is says what it is doing:
//   busy                at its project's desk, typing
//   idle                beside its cabinet, or on a break: coffee, the printer, the cooler, ping-pong,
//                       the plants, the window, the armchair, a meeting at the whiteboard
//   idle 10 minutes     charging on a mat
//   sends a message     walks over to whoever it wrote to and stands there talking for a moment
// Nothing else signals anything; what they say is never shown, only that they talk.
// The machines are lettered on the wall, the current one underlined.
//
// startRoom(canvas, app) with app = { state(), cur(), open(), inset(), act(action) };
// returns { talk(room, from, to), size(), draw() }. Sizes are in dots; a dot is 2 to 4 CSS pixels.

function startRoom(cv, app) {
  const C = pixelAvatar.C, INK = C.ink, INK2 = C.ink2;
  const NIGHT = "#0d0a16", TERRACOTTA = "#d97757", RED = "#ff6b5a";
  const WOOD = "#7a4a35", WOOD_D = "#4a2e2a", WALLC = "#1e1a2c", FLOOR = "#130f1b", PLANK = "#19141f";
  const PROJECT = ["#4a76b8", "#e8402a", "#2fa087", "#f2a531", "#9a74d6", "#5fcf4a", "#f06b72", "#8fc0e8"];
  const WALL = 30, TALK = 4, SEAT = 18, ADD_W = 30, ROW = 46;
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const shade = (hex, k) => "#" + [1, 3, 5].map((i) => Math.min(255, Math.round(parseInt(hex.slice(i, i + 2), 16) * k)).toString(16).padStart(2, "0")).join("");
  const mix = (a, b, k) => "#" + [1, 3, 5].map((i) => { const x = parseInt(a.slice(i, i + 2), 16), y = parseInt(b.slice(i, i + 2), 16); return Math.round(x + (y - x) * k).toString(16).padStart(2, "0"); }).join("");
  const rnd = (a, b) => a + Math.random() * (b - a);
  const any = (a) => a[Math.floor(Math.random() * a.length)];
  const fnv = (s) => { let h = 2166136261; for (const ch of s) h = Math.imul(h ^ ch.codePointAt(0), 16777619); return h >>> 0; };
  const projectColor = (cwd) => PROJECT[fnv(cwd) % PROJECT.length];
  function projectLabel(cwd) {
    const n = cwd.replace(/\/+$/, "").split("/");
    if (cwd === "~" || cwd === "/root" || (n.length === 3 && (n[1] === "Users" || n[1] === "home"))) return "home";
    return n[n.length - 1] || cwd;
  }
  let W = 280, H = 180, K = 3;

  // ---- pair's lettering: 3x5 glyphs, upper case, like a Pico-8
  const F = {};
  for (const g of ("A .#./#.#/###/#.#/#.# B ##./#.#/##./#.#/##. C .##/#../#../#../.## D ##./#.#/#.#/#.#/##. E ###/#../##./#../### " +
    "F ###/#../##./#../#.. G .##/#../#.#/#.#/.## H #.#/#.#/###/#.#/#.# I ###/.#./.#./.#./### J ..#/..#/..#/#.#/.#. " +
    "K #.#/#.#/##./#.#/#.# L #../#../#../#../### M #.#/###/###/#.#/#.# N ##./#.#/#.#/#.#/#.# O .#./#.#/#.#/#.#/.#. " +
    "P ##./#.#/##./#../#.. Q .#./#.#/#.#/##./.## R ##./#.#/##./#.#/#.# S .##/#../.#./..#/##. T ###/.#./.#./.#./.#. " +
    "U #.#/#.#/#.#/#.#/.## V #.#/#.#/#.#/#.#/.#. W #.#/#.#/###/###/#.# X #.#/#.#/.#./#.#/#.# Y #.#/#.#/.#./.#./.#. " +
    "Z ###/..#/.#./#../### 0 ###/#.#/#.#/#.#/### 1 .#./##./.#./.#./### 2 ##./..#/.#./#../### 3 ##./..#/.#./..#/##. " +
    "4 #.#/#.#/###/..#/..# 5 ###/#../##./..#/##. 6 .##/#../###/#.#/### 7 ###/..#/.#./.#./.#. 8 ###/#.#/###/#.#/### " +
    "9 ###/#.#/###/..#/##. . .../.../.../.../.#. , .../.../.../.#./#.. ! .#./.#./.#./.../.#. ? ##./..#/.#./.../.#. " +
    "' .#./.#./.../.../... - .../.../###/.../... : .../.#./.../.#./... ( ..#/.#./.#./.#./..# ) #../.#./.#./.#./#.. " +
    "/ ..#/..#/.#./#../#.. + .../.#./###/.#./... = .../###/.../###/... % #.#/..#/.#./#../#.# * #.#/.#./#.#/.../... " +
    "# #.#/###/#.#/###/#.# \" #.#/#.#/.../.../... & .#./#.#/.#./#.#/.## _ .../.../.../.../###").match(/\S+ \S+/g)) {
    const [k, rows] = g.split(" ");
    F[k] = rows.split("/").map((r) => r.padEnd(3, ".").slice(0, 3)).join("");
  }
  const tinyW = (s) => Math.max(0, [...s].length * 4 - 1);
  function tiny(ctx, s, x, y, col) {
    ctx.fillStyle = col;
    [...s.toUpperCase()].forEach((ch, i) => {
      const gl = F[ch];
      if (gl) for (let k = 0; k < 15; k++) if (gl[k] === "#") ctx.fillRect(x + i * 4 + (k % 3), y + Math.floor(k / 3), 1, 1);
    });
  }

  // ---- robots: pixel.js's head on floor.js's body, drawn once per pose with an ink outline
  const SW = 22, SH = 29;
  const faces = new Map(), bodies = new Map();
  function face(seed, color) {
    const key = seed + color;
    if (!faces.has(key)) {
      const im = new Image(), f = { px: null };
      im.onload = () => {
        const c = document.createElement("canvas"); c.width = c.height = 20;
        const x = c.getContext("2d"); x.drawImage(im, 0, 0);
        f.px = x.getImageData(0, 0, 20, 20).data;
      };
      im.src = pixelAvatar(seed, "agent", color);
      faces.set(key, f);
    }
    return faces.get(key);
  }
  // arms: down sw1 sw2 up wave fwd1 fwd2 reach mug paper book can; legs: stand walk1 walk2 sit
  function body(seed, color, arms, legs, ghost) {
    const f = face(seed, color);
    if (!f.px) return null;
    const key = seed + color + arms + legs + (ghost ? "g" : "");
    if (bodies.has(key)) return bodies.get(key);
    const px = new Map(), line = new Set();
    const set = (x, y, c) => { if (c) px.set((y + 1) * SW + x + 1, c); };
    const rect = (x, y, w, h, c) => { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) set(x + i, y + j, c); };
    const cut = 16;                          // the portrait down to its neck
    for (let y = 0; y <= cut; y++) for (let x = 0; x < 20; x++) {
      const i = (y * 20 + x) * 4, d = f.px;
      if (d[i + 3] < 128) continue;
      const c = "#" + [d[i], d[i + 1], d[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join("");
      set(x, y, c);
      if (c === INK) line.add((y + 1) * SW + x + 1);
    }
    const c0 = C.steel3, c1 = C.steel1, skin = C.steel3, leg = C.steel1, top = cut + 1;
    rect(6, top, 8, 22 - top, c0); rect(12, top, 2, 22 - top, c1);
    set(9, top + 2, color); set(10, top + 2, color); set(9, top + 3, shade(color, 0.62));
    const arm = (x, y0, y1, hx, hy) => { rect(x, y0, 2, y1 - y0 + 1, c1); rect(hx, hy, 2, 1, skin); };
    const back = () => arm(4, 17, 20, 4, 21);
    const fwd = (hy, w = 3) => { rect(14, hy - 1, w, 2, c1); rect(14 + w, hy - 1, 2, 2, skin); };
    switch (arms) {
      case "up": arm(4, 12, 16, 4, 11); arm(14, 12, 16, 14, 11); break;
      case "wave": back(); arm(14, 11, 16, 15, 10); break;
      case "sw1": arm(4, 17, 19, 5, 20); arm(14, 17, 19, 13, 20); break;
      case "sw2": arm(4, 17, 19, 3, 20); arm(14, 17, 19, 15, 20); break;
      case "fwd1": rect(4, 17, 2, 2, c1); rect(4, 19, 2, 1, skin); fwd(19); break;
      case "fwd2": rect(4, 17, 2, 2, c1); rect(4, 19, 2, 1, skin); fwd(20); break;
      case "reach": back(); rect(14, 14, 2, 4, c1); rect(16, 13, 2, 2, c1); rect(18, 12, 2, 2, skin); break;
      case "mug": back(); fwd(19, 2); rect(17, 16, 3, 3, color); rect(19, 17, 1, 1, INK); break;
      case "paper": back(); fwd(19, 2); rect(16, 15, 4, 5, C.steel4); rect(17, 16, 2, 1, C.steel2); rect(17, 18, 2, 1, C.steel2); break;
      case "book": back(); fwd(19, 2); rect(16, 16, 3, 4, C.red1); rect(16, 16, 1, 4, C.red0); break;
      case "can": back(); fwd(19, 2); rect(16, 17, 4, 3, C.green0); rect(20, 17, 1, 1, C.green0); break;
      default: back(); arm(14, 17, 20, 14, 21);
    }
    switch (legs) {
      case "sit": rect(7, 22, 9, 2, leg); rect(14, 24, 2, 3, leg); rect(14, 26, 3, 1, INK); break;
      case "walk1": rect(7, 22, 2, 5, leg); rect(11, 22, 2, 4, leg); rect(7, 26, 3, 1, INK); rect(11, 25, 3, 1, INK); break;
      case "walk2": rect(7, 22, 2, 4, leg); rect(11, 22, 2, 5, leg); rect(7, 25, 3, 1, INK); rect(11, 26, 3, 1, INK); break;
      default: rect(7, 22, 2, 5, leg); rect(11, 22, 2, 5, leg); rect(7, 26, 3, 1, INK); rect(11, 26, 3, 1, INK);
    }
    const c = document.createElement("canvas"); c.width = SW; c.height = SH;
    const x = c.getContext("2d");
    x.fillStyle = INK;
    for (let y = 0; y < SH; y++) for (let i = 0; i < SW; i++) {
      if (px.has(y * SW + i)) continue;
      if ([[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => {
        const k = (y + dy) * SW + i + dx;
        return i + dx >= 0 && i + dx < SW && px.has(k) && !line.has(k);
      })) x.fillRect(i, y, 1, 1);
    }
    // one in a terminal elsewhere: greyed, like pair's people who are away
    for (const [k, col] of px) { x.fillStyle = ghost ? mix(col, C.steel0, 0.55) : col; x.fillRect(k % SW, Math.floor(k / SW), 1, 1); }
    bodies.set(key, c);
    return c;
  }

  // ---- the room, laid out for the window as it is now
  function layout(dirs, bots, asleep, wallLeft) {
    const order = [...dirs];
    for (const b of bots) if (!order.includes(b.cwd)) order.push(b.cwd);
    const floorY = H - 6;
    const window_ = W >= 200 ? { x0: W - 46, y0: 6, x1: W - 28, y1: 21 } : null;
    // the whiteboard in the middle of the wall, or as far right as the names make it, if it fits
    const bx = Math.round(Math.max(W / 2 - 14, wallLeft + 6)), right = (window_ ? window_.x0 : W - 18) - 6;
    const board = bx + 30 <= right ? { x0: bx, y0: 8, x1: bx + 28, y1: 20 } : null;

    // the units in columns, each as wide as its widest unit, cabinets lined up: as many columns as fit
    const sizes = order.map((cwd) => {
      const n = bots.filter((b) => b.cwd === cwd).length, cab = Math.max(tinyW(projectLabel(cwd)) + 6, 22);
      return { cwd, w: cab + 4 + (n + 1) * SEAT, cab };
    });
    let widths = sizes.map((s) => s.w).concat([ADD_W]);
    const gapX = 14;
    const columns = (ws, c) => [...Array(c)].map((_, j) => Math.max(...ws.filter((_, i) => i % c === j)));
    const total = (cw) => cw.reduce((a, b) => a + b, 0) + gapX * (cw.length - 1);
    const fit = (ws) => { for (let c = ws.length; c >= 1; c--) if (total(columns(ws, c)) <= W - 16) return c; return 1; };
    let c = fit(widths);
    // the empty cabinet doesn't get a row of its own: then it stands by the wall, in the bottom row
    const addBelow = widths.length > 1 && (widths.length - 1) % c === 0 && W >= 110;
    if (addBelow) { widths = widths.slice(0, -1); c = fit(widths); }
    const cw = columns(widths, c);
    const left = Math.max(6, Math.round((W - total(cw)) / 2));
    const colX = []; cw.reduce((x, w) => { colX.push(x); return x + w + gapX; }, left);
    const rows = Math.ceil(widths.length / c);

    // top to bottom, as in pair: the meeting table under the whiteboard, the rows of units, the
    // ping-pong table and the mats, spread evenly; what doesn't fit is left out
    const wide = W >= 200, avail = floorY - 28 - (WALL + 4);
    let used = rows * ROW;
    const fitsMats = used + 30 <= avail; if (fitsMats) used += 30;
    const fitsPong = wide ? fitsMats : used + 34 <= avail; if (fitsPong && !wide) used += 34;
    const fitsMeet = !!board && used + 40 <= avail; if (fitsMeet) used += 40;
    const parts = rows + fitsMats + (fitsPong && !wide) + fitsMeet;
    const pad = Math.max(0, (avail - used) / (parts + 1));
    let y0 = WALL + 4 + pad, meet = null;
    if (fitsMeet) { meet = { x: Math.round((board.x0 + board.x1) / 2), y: Math.round(y0 + 30) }; y0 += 40 + pad; }
    const desks = [];
    let add = null;
    widths.forEach((wi, i) => {
      const x = colX[i % c], y = Math.round(y0 + Math.floor(i / c) * (ROW + pad) + 38);
      if (i === sizes.length) { add = { x0: x, y0: y - 13, x1: x + wi, y1: y }; return; }
      const { cwd, w, cab } = sizes[i], mine = bots.filter((b) => b.cwd === cwd);
      const seat = (k) => x + cab + 4 + SEAT / 2 + k * SEAT;
      desks.push({ cwd, x0: x, w, cab, y, color: projectColor(cwd), seats: mine.map((b, k) => [b.id, seat(k)]), plus: seat(mine.length) });
    });
    y0 += rows * (ROW + pad);
    if (addBelow) { const x = Math.round((48 + (W / 2 - 6)) / 2 - ADD_W / 2); add = { x0: x, y0: floorY - 13, x1: x + ADD_W, y1: floorY }; }
    const n = Math.min(Math.max(asleep, 1), Math.max(1, Math.floor((W - 12) / 28)));
    const matRow = (cx, y, n) => [...Array(n)].map((_, i) => ({ x: Math.round(cx + (i - (n - 1) / 2) * 28), y: Math.round(y) }));
    let pong = null, mats = [];
    if (wide && fitsMats) {                   // side by side: the table on the left, the mats on the right
      pong = { x: Math.round(W * 0.3), y: Math.round(y0 + 26) };
      mats = matRow(W * 0.68, y0 + 28, Math.max(1, Math.min(n, Math.floor((W * 0.45) / 28))));
    } else {
      if (fitsPong) { pong = { x: Math.round(W / 2), y: Math.round(y0 + 26) }; y0 += 34 + pad; }
      if (fitsMats) mats = matRow(W / 2, y0 + 28, n);
    }
    // along the bottom, as in pair: the coffee corner, the cooler, the printer, an armchair, a plant
    const cooler = W >= 76 ? { x: 36, y: floorY - 1 } : null;
    const plants = [{ x: W - 8, y: floorY }];
    if (H >= 300 && left >= 24) plants.push({ x: 8, y: Math.round(H * 0.55) });
    const L = {
      desks, add, floorY, board, window: window_, meet, pong, mats, cooler, plants,
      floor: { x0: 14, y0: WALL + 8, x1: W - 14, y1: floorY - 4 },
      coffee: cooler ? { x: 30, y: floorY - 10 } : { x: 33, y: floorY },
      printer: W >= 110 ? { x: Math.round(W / 2 + 4), y: floorY } : null,
      chair: W >= 140 ? { x: Math.round(W - 34), y: floorY } : null,
    };
    L.coolerSpot = (side) => L.cooler && { x: L.cooler.x + (side ? 10 : -8), y: L.cooler.y + 1 };
    L.pongSpot = (side) => L.pong && { x: L.pong.x + (side ? 22 : -22), y: L.pong.y + 1 };
    L.meetSeat = (i) => { const s = [[-23, 0, 1], [23, 0, -1], [-8, -5, 1], [8, -5, -1]][i]; return L.meet && s && { x: L.meet.x + s[0], y: L.meet.y + s[1], dir: s[2] }; };
    L.seat = (id) => { for (const d of desks) for (const [i, x] of d.seats) if (i === id) return { x, y: d.y }; return null; };
    L.free = (x, y) => {                      // a place to stand about: not in a unit, a mat or the ping-pong table
      const inR = (x0, y0, x1, y1) => x >= x0 && x <= x1 && y >= y0 && y <= y1;
      return !desks.some((d) => inR(d.x0 - 8, d.y - 30, d.x0 + d.w + 10, d.y + 6))
        && !(add && inR(add.x0 - 8, add.y0 - 14, add.x1 + 8, add.y1 + 6))
        && !mats.some((m) => inR(m.x - 14, m.y - 26, m.x + 14, m.y + 4))
        && !(pong && inR(pong.x - 36, pong.y - 20, pong.x + 36, pong.y + 8))
        && !(meet && inR(meet.x - 28, meet.y - 14, meet.x + 28, meet.y + 4));
    };
    return L;
  }

  // ---- everyone, per machine: what each bot is doing, kept while you look at another machine
  const worlds = new Map();
  function world(i) {
    if (!worlds.has(i)) worlds.set(i, { folk: new Map(), meeting: null, lastMeeting: -60, board: [], pong: { t: 0, from: 0 }, T: 0, born: performance.now() });
    return worlds.get(i);
  }
  const rest = (r = Math.random()) => ({ kind: "stand", left: 2 + r * 5 });   // a pause, before the next break
  function end(f) { f.lying = false; f.carry = null; f.act = rest(0.06); }
  function walk(f, to, v, dt) {
    const dx = to.x - f.x, dy = to.y - f.y, d = Math.hypot(dx, dy);
    if (d <= v * dt) { f.x = to.x; f.y = to.y; if (f.legs !== "sit") f.legs = "stand"; return true; }
    f.x += (dx / d) * v * dt; f.y += (dy / d) * v * dt;
    if (Math.abs(dx) > 0.5) f.dir = Math.sign(dx);
    f.step += dt * 8;
    const k = Math.floor(f.step) % 2;
    f.legs = k ? "walk1" : "walk2";
    f.arms = f.carry || (k ? "sw1" : "sw2");
    return false;
  }

  // `from` sent `to` a message: it walks over
  function talk(ri, from, to) {
    const w = world(ri), f = w.folk.get(from), g = w.folk.get(to);
    if (!f || !g) return;
    for (const [a, b] of [[f, g], [g, f]]) if (a.act.kind === "visit" && a.act.to === b.id) { a.act.left = Math.max(a.act.left, TALK); return; }
    f.lying = false; f.carry = null;
    f.act = { kind: "visit", to, left: TALK, wave: 0, msg: true };
  }

  function think(w, bots, L, dt, sleepAfter) {
    const now = performance.now() / 1000, folk = w.folk;
    for (const b of bots) {
      const f = folk.get(b.id);
      f.chat = Math.max(0, f.chat - dt);
      if (b.busy !== f.busy) {
        f.busy = b.busy; f.idleSince = now;
        // to work, whatever it was doing (a message still gets delivered first)
        if (b.busy && !(f.act.kind === "visit" && f.act.msg)) { end(f); f.act = { kind: "desk" }; }
        if (!b.busy && f.act.kind === "desk") f.act = { kind: "stand", left: 0.5 + (fnv(b.id) % 20) / 10 };
      }
    }
    if (w.meeting && (w.meeting.left -= dt) <= 0) {
      w.meeting = null;
      for (const f of folk.values()) if (f.act.kind === "meeting" || f.act.kind === "present") end(f);
    }
    // the ball goes back and forth while two are at the table
    if ([...folk.values()].filter((f) => f.act.kind === "pong" && f.act.there).length === 2 && (w.pong.t += dt / 0.8) >= 1) { w.pong.t = 0; w.pong.from = 1 - w.pong.from; }
    // an idle bot's place: beside its project's cabinet, one per project (the first idle one);
    // the others are off on a break
    const homes = new Map();
    for (const d of L.desks) {
      const first = d.seats.find(([id]) => { const f = folk.get(id); return f && !f.busy && f.act.kind === "stand"; });
      if (first) homes.set(first[0], { x: Math.max(8, d.x0 - 10), y: d.y + 1 });
    }
    for (const b of bots) thinkOne(w, folk.get(b.id), L, now, dt, homes.get(b.id), sleepAfter);
    // two standing about in the same place: they step apart
    const loit = [...folk.values()].filter((f) => f.legs === "stand" && f.act.kind === "loiter");
    for (const a of loit) for (const b of loit) if (a.id < b.id && Math.abs(a.x - b.x) < 12 && Math.abs(a.y - b.y) < 4) {
      const d = a.x <= b.x ? -1 : 1; a.x += d * 8 * dt; b.x -= d * 8 * dt;
    }
  }

  function thinkOne(w, f, L, now, dt, home, sleepAfter) {
    const folk = w.folk, others = [...folk.values()].filter((g) => g !== f), v = 22;
    const taken = (kind) => others.some((g) => g.act.kind === kind);
    const seat = L.seat(f.id);
    const sleepy = !f.busy && now - f.idleSince > sleepAfter;
    if (sleepy && f.act.kind !== "sleep" && !(f.act.kind === "visit" && f.act.msg)) { end(f); f.act = { kind: "sleep", there: false }; }
    if (!sleepy && f.act.kind === "sleep") end(f);
    const act = f.act;
    switch (act.kind) {
      case "desk":
        if (seat && walk(f, seat, v, dt)) {
          f.dir = -1; f.legs = "sit"; f.carry = null;
          f.arms = f.busy ? (Math.floor(w.T * 6) % 2 ? "fwd2" : "fwd1") : "down";
        }
        break;
      case "stand": {
        const there = !home || walk(f, home, v, dt);   // back to its cabinet first, if that is its place
        if (!there) break;
        f.legs = "stand"; f.arms = f.carry || "down";
        if (home) f.dir = 1;
        if ((act.left -= dt) > 0) break;
        f.carry = null; f.act = pickBreak(w, f, L, others);
        break;
      }
      case "loiter":
        if (walk(f, act, v, dt)) { f.arms = f.carry || "down"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      case "coffee":
        if (act.stage === 0) { if (walk(f, L.coffee, v, dt)) { f.dir = -1; f.arms = "reach"; act.stage = 1; } }
        else { act.wait += dt; f.arms = act.wait > 2 ? "mug" : "reach"; if (act.wait > 2.8) { f.carry = "mug"; f.act = rest(); } }
        break;
      case "print":
        if (!L.printer) { f.act = rest(); break; }
        if (act.stage === 0) { if (walk(f, { x: L.printer.x - 9, y: L.printer.y }, v, dt)) { f.dir = 1; f.arms = "down"; act.stage = 1; } }
        else if ((act.wait += dt) > 2.4) { f.carry = "paper"; f.act = rest(); }
        break;
      case "cooler": {
        const s = L.coolerSpot(act.side), g = folk.get(act.with);
        if (!s || !g) { f.act = rest(); break; }
        if (walk(f, s, v, dt)) {
          f.dir = act.side ? -1 : 1; f.arms = "down";
          if (Math.abs(g.x - f.x) < 24 && Math.abs(g.y - f.y) < 4 && Math.random() < dt * 0.4) f.chat = 1.2;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "pong": {
        const s = L.pongSpot(act.side);
        if (!s || !folk.get(act.with)) { f.act = rest(); break; }
        act.there = walk(f, s, v, dt);
        if (act.there) {
          f.dir = act.side ? -1 : 1;
          f.arms = w.pong.from !== act.side && w.pong.t > 0.75 ? "fwd1" : "fwd2";
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "visit": {
        const g = folk.get(act.to);
        if (!g) { f.act = f.busy ? { kind: "desk" } : rest(); break; }
        // a seated one faces its screen (left): stand behind its chair; else beside it, on the
        // side it comes from, as in pair
        const side = f.x < g.x ? -1 : 1, seated = g.legs === "sit";
        const spot = seated ? { x: g.x + 20, y: g.y + 1 } : { x: g.x + side * 14, y: g.y + 1 };
        if (walk(f, spot, act.msg ? 34 : v + 6, dt)) {
          f.dir = spot.x > g.x ? -1 : 1;
          f.arms = (act.wave -= dt) > 0 ? "wave" : "down";
          if (f.chat <= 0 && Math.random() < dt * 1.5) f.chat = 1;
          // visited: an idle one stops and faces the visitor, and talks back now and then
          if (!g.busy && (g.act.kind === "stand" || g.act.kind === "loiter")) { g.act = { kind: "stand", left: 2 }; g.legs = "stand"; g.dir = -f.dir; }
          if (g.chat <= 0 && Math.random() < dt * 1.2) g.chat = 1;
          if ((act.left -= dt) <= 0) f.act = f.busy ? { kind: "desk" } : rest();
        }
        break;
      }
      case "plant": {
        const pl = L.plants[act.i] || L.plants[0], side = pl.x > (L.floor.x0 + L.floor.x1) / 2 ? -1 : 1;
        f.carry = "can";
        if (walk(f, { x: pl.x + side * 12, y: pl.y + 1 }, v, dt)) {
          f.dir = -side; f.arms = "can"; act.water = true;
          if ((act.wait += dt) > 3) { f.carry = null; f.act = rest(); }
        }
        break;
      }
      case "window":
        if (!L.window) { f.act = rest(); break; }
        if (walk(f, { x: Math.round((L.window.x0 + L.window.x1) / 2), y: WALL + 8 }, v, dt)) {
          f.arms = "down"; f.dir = Math.sin(w.T * 0.7 + f.z) > 0 ? 1 : -1;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      case "chair":
        if (!L.chair) { f.act = rest(); break; }
        if (walk(f, { x: L.chair.x - 2, y: L.chair.y - 1 }, v, dt)) { f.dir = -1; f.legs = "sit"; f.arms = "book"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      case "present":
        if (!L.board || !w.meeting || w.meeting.host !== f.id) { f.act = rest(); break; }
        if (walk(f, { x: L.board.x0 - 6, y: WALL + 3 }, v, dt)) {
          f.dir = 1; f.arms = Math.floor(w.T * 2) % 3 ? "reach" : "down";
          if ((act.wait += dt) > 0.6 && w.board.length < 40) { act.wait = 0; stroke(w, L); }
          if (Math.random() < dt * 0.3) f.chat = 1;
        }
        break;
      case "meeting": {
        const s = L.meetSeat(act.seat);
        if (!s || !w.meeting) { f.act = rest(); break; }
        if (walk(f, s, v, dt)) { f.dir = s.dir; f.legs = "sit"; f.arms = "down"; if (Math.random() < dt * 0.08) f.chat = 1; }
        break;
      }
      case "sleep": {
        // on a free mat if there is one, else asleep at the desk
        const mat = L.mats.find((m) => !others.some((g) => g.act.kind === "sleep" && g.lying && Math.abs(g.x - m.x) < 1 && Math.abs(g.y - m.y) < 1));
        const to = mat || seat;
        if (to && (act.there || walk(f, to, v, dt))) {
          act.there = true; f.x = to.x; f.y = to.y; f.lying = !!mat;
          f.dir = mat ? 1 : -1; f.legs = "sit"; f.arms = "down"; f.z += dt;
        }
        break;
      }
      default: f.act = rest();
    }
    if (f.legs !== "sit") f.lying = false;
  }

  // a break, chosen by a bot with nothing to do (pair's pickBreak)
  function pickBreak(w, f, L, others) {
    const r = Math.random();
    const mates = others.filter((g) => !g.busy && (g.act.kind === "stand" || g.act.kind === "loiter"));
    const mate = mates.length ? any(mates) : null;
    const taken = (kind) => others.some((g) => g.act.kind === kind);
    if (L.meet && mates.length >= 2 && !w.meeting && w.T - w.lastMeeting > 60 && r < 0.08) {
      w.meeting = { host: f.id, left: rnd(22, 38) }; w.lastMeeting = w.T; w.board = [];
      mates.slice(0, 4).forEach((g, i) => { g.carry = null; g.act = { kind: "meeting", seat: i }; });
      return { kind: "present", wait: 0 };
    }
    if (r < 0.22 && !taken("coffee")) return { kind: "coffee", stage: 0, wait: 0 };
    if (r < 0.32 && L.printer && !taken("print")) return { kind: "print", stage: 0, wait: 0 };
    if (r < 0.46 && mate && L.cooler) {
      const left = rnd(10, 22);
      mate.carry = null; mate.act = { kind: "cooler", side: 1, left, with: f.id };
      return { kind: "cooler", side: 0, left, with: mate.id };
    }
    if (r < 0.56 && mate && L.pong && !taken("pong")) {
      const left = rnd(15, 35);
      mate.carry = null; mate.act = { kind: "pong", side: 1, left, with: f.id, there: false };
      return { kind: "pong", side: 0, left, with: mate.id, there: false };
    }
    if (r < 0.62 && mate) return { kind: "visit", to: mate.id, left: rnd(6, 12), wave: 0.8, msg: false };
    const plants = L.plants.map((_, i) => i).filter((i) => !others.some((g) => g.act.kind === "plant" && g.act.i === i));
    if (r < 0.7 && plants.length) return { kind: "plant", i: any(plants), wait: 0 };
    if (r < 0.78 && L.window && !taken("window")) return { kind: "window", left: rnd(4, 9) };
    if (r < 0.86 && L.chair && !taken("chair")) return { kind: "chair", left: rnd(15, 35) };
    for (let k = 0; k < 8; k++) {             // somewhere free to stand about
      const x = rnd(L.floor.x0, L.floor.x1), y = rnd(L.floor.y0, L.floor.y1);
      if (L.free(x, y)) return { kind: "loiter", x, y, left: rnd(3, 8) };
    }
    return rest();
  }
  function stroke(w, L) {                    // the host draws on the whiteboard: boxes, arrows, dots
    const b = L.board, bw = b.x1 - b.x0 - 4, bh = b.y1 - b.y0 - 4;
    const x = Math.floor(rnd(0, bw - 6)), y = Math.floor(rnd(1, bh - 2)), k = Math.random();
    w.board.push(k < 0.35 ? { t: "box", x, y, w: Math.floor(rnd(3, 7)), h: Math.floor(rnd(2, 4)) }
      : k < 0.7 ? { t: "line", x, y, w: Math.floor(rnd(3, 10)) } : { t: "dot", x, y });
  }

  // ---- drawing, and what is under the pointer: hot spots are collected while drawing
  let pointer = null, hots = [], hovered = null;
  const over = (x0, y0, x1, y1) => !!pointer && pointer.x >= x0 && pointer.x < x1 && pointer.y >= y0 && pointer.y < y1;
  const hot = (x0, y0, x1, y1, action) => { hots.push({ x0, y0, x1, y1, action }); return over(x0, y0, x1, y1); };
  const box = (ctx, x, y, w, h, fill, edge = INK) => { ctx.fillStyle = edge; ctx.fillRect(x - 1, y - 1, w + 2, h + 2); ctx.fillStyle = fill; ctx.fillRect(x, y, w, h); };
  const rect = (ctx, x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); };

  function draw() {
    const ctx = cv.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    const s = app.state(), ri = app.cur(), room = s && s.rooms[ri];
    hots = [];
    rect(ctx, 0, 0, W, H, FLOOR);
    ctx.fillStyle = PLANK;
    for (let y = WALL + 4, k = 0; y < H; y += 6, k++) { ctx.fillRect(0, y, W, 1); for (let x = (k % 2) * 13; x < W; x += 26) ctx.fillRect(x, y - 5, 1, 5); }
    rect(ctx, 0, 0, W, WALL, WALLC);
    rect(ctx, 0, WALL - 2, W, 2, INK2);
    if (!room) return;
    const w = world(ri), bots = room.bots || [];

    // the machines, lettered on the wall
    const tabsX = Math.ceil(app.inset() / K) + 22;
    let tx = tabsX;
    s.rooms.forEach((r, i) => {
      const tw = tinyW(r.name), h = i !== ri && hot(tx - 2, 0, tx + tw + 2, 12, { kind: "room", i });
      tiny(ctx, r.name, tx, 4, i === ri ? C.steel4 : h ? C.steel3 : C.steel1);
      if (i === ri) rect(ctx, tx, 11, tw, 1, TERRACOTTA);
      tx += tw + 7;
    });
    tiny(ctx, "+", tx, 4, hot(tx - 2, 0, tx + 5, 12, { kind: "addRoom" }) ? TERRACOTTA : C.steel1);
    const wallLeft = tx + 5;
    const asleep = [...w.folk.values()].filter((f) => f.act.kind === "sleep").length;
    const L = layout(room.dirs || [], bots, asleep, wallLeft);
    drawDoor(ctx, 5); drawClock(ctx, W - 10, 13);
    if (room.error) {                        // trouble goes on the wall, in red, instead of the whiteboard
      const nx = Math.max(wallLeft + 6, 24), n = Math.max(0, Math.floor((W - 24 - nx) / 4));
      const msg = room.error.replace(/\s+/g, " ");
      tiny(ctx, msg.length > n ? msg.slice(0, n - 1) + "-" : msg, nx, 18, RED);
    } else if (L.board) drawBoard(ctx, L.board, w);
    if (L.window) drawWindow(ctx, L.window);

    // keep a figure per bot; one that turns up later comes in through the door
    const ids = new Set(bots.map((b) => b.id));
    for (const id of w.folk.keys()) if (!ids.has(id)) w.folk.delete(id);
    const late = performance.now() - w.born > 3000;
    for (const b of bots) if (!w.folk.has(b.id)) {
      const at = late ? { x: 11, y: WALL + 3 } : L.seat(b.id) || { x: W / 2, y: H / 2 };
      w.folk.set(b.id, { id: b.id, x: at.x, y: at.y, dir: -1, arms: "down", legs: late ? "stand" : "sit", step: 0,
        act: b.busy ? { kind: "desk" } : rest(), carry: null, lying: false, busy: b.busy, idleSince: performance.now() / 1000, chat: 0, z: Math.random() * 2 });
    }
    if (tickDt > 0) { w.T += tickDt; think(w, bots, L, tickDt, s.demo ? 40 : 600); tickDt = 0; }
    const T = w.T;

    // flat on the floor: the empty cabinet that adds a project, the mats, the ping-pong rug
    if (L.add) {
      const a = L.add, h = hot(a.x0 - 2, a.y0 - 10, a.x1 + 2, a.y1 + 1, { kind: "addDir" }), aw = a.x1 - a.x0;
      box(ctx, a.x0, a.y0, aw, 13, WOOD_D);
      rect(ctx, a.x0, a.y0, aw, 1, WOOD);
      rect(ctx, a.x0 + 3, a.y0 + 3, aw - 6, 7, h ? C.steel1 : C.steel0);
      tiny(ctx, "+", Math.round(a.x0 + aw / 2 - 1), a.y0 + 4, h ? C.steel4 : C.steel2);
      if (h) tiny(ctx, "PROJECT", Math.round(a.x0 + aw / 2 - tinyW("PROJECT") / 2), a.y0 - 8, C.steel3);
    }
    for (const m of L.mats) drawMat(ctx, m);
    if (L.pong) box(ctx, L.pong.x - 33, L.pong.y - 17, 66, 22, FLOOR, "#2a1d3a");

    // everything that stands, back to front
    const items = [];
    const bot = (id) => bots.find((b) => b.id === id);
    for (const f of w.folk.values()) if (!f.lying) items.push([f.y, () => drawFigure(ctx, f, bot(f.id), T)]);
    for (const d of L.desks) {
      items.push([d.y - 1, () => drawDesk(ctx, d, w, T, ri)]);
      for (const [, x] of d.seats) items.push([d.y - 0.5, () => drawChair(ctx, x, d.y, d, false)]);
      items.push([d.y - 0.5, () => drawChair(ctx, d.plus, d.y, d, true)]);
    }
    if (L.meet) {
      items.push([L.meet.y - 1, () => drawTable(ctx, L.meet)]);
      for (const i of [0, 1]) { const st = L.meetSeat(i); items.push([st.y - 0.5, () => drawStool(ctx, st)]); }
    }
    items.push([L.floorY, () => drawCoffee(ctx, 3, L.floorY, [...w.folk.values()].some((f) => f.act.kind === "coffee" && f.act.stage === 1), T)]);
    if (L.cooler) items.push([L.cooler.y, () => drawCooler(ctx, L.cooler)]);
    if (L.printer) { const job = [...w.folk.values()].find((f) => f.act.kind === "print" && f.act.stage === 1); items.push([L.printer.y, () => drawPrinter(ctx, L.printer, job && job.act.wait)]); }
    if (L.chair) items.push([L.chair.y - 0.5, () => drawArmchair(ctx, L.chair)]);
    for (const pl of L.plants) items.push([pl.y, () => drawPlant(ctx, pl)]);
    if (L.pong) items.push([L.pong.y - 0.5, () => drawPong(ctx, L.pong, w)]);
    hovered = null;
    for (const f of w.folk.values()) if (f.lying) drawSleeper(ctx, f, bot(f.id), T);
    items.sort((a, b) => a[0] - b[0]);
    for (const [, d] of items) d();

    // talk, over everything: dots in a bubble, never the words
    for (const f of w.folk.values()) if (f.chat > 0 && !still) {
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - (f.lying ? 28 : 27) - 6;
      rect(ctx, bx - 1, by - 1, 11, 7, INK); rect(ctx, bx, by, 9, 5, C.steel4); rect(ctx, bx, by + 5, 1, 1, C.steel4);
      for (let i = 0; i < 3; i++) if ((Math.floor(f.chat * 4) + i) % 3) rect(ctx, bx + 2 + i * 2, by + 2, 1, 1, INK);
    }
    // the bot under the pointer: its name over its head, and a click opens it
    if (hovered) {
      const { b, x, y } = hovered, name = b.background ? b.name : b.name + " (in a terminal)";
      const tw = tinyW(name), lx = Math.round(Math.max(2, Math.min(W - tw - 2, x - tw / 2))), ly = Math.round(Math.max(WALL + 2, y - 10));
      box(ctx, lx - 2, ly - 2, tw + 4, 9, INK2);
      tiny(ctx, name, lx, ly, C.steel4);
      if (b.background) hots.push({ x0: -1e9, y0: -1e9, x1: 1e9, y1: 1e9, action: { kind: "open", id: b.id } });
    }
    cv.style.cursor = hots.some((h) => over(h.x0, h.y0, h.x1, h.y1)) ? "pointer" : "default";
  }

  function drawFigure(ctx, f, b, T) {
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), x0 = x - 11, y0 = y - 27;
    rect(ctx, x - 5, y, 10, 1, "rgba(0,0,0,0.4)");
    const im = body(b.id, projectColor(b.cwd), f.arms, f.legs, !b.background);
    if (im) {
      ctx.save();
      if (f.dir < 0) { ctx.translate(x0 + SW, 0); ctx.scale(-1, 1); ctx.drawImage(im, 0, y0); } else ctx.drawImage(im, x0, y0);
      ctx.restore();
    }
    if (f.act.kind === "sleep" && f.act.there) zzz(ctx, f, x + 2, y0 + 4);
    if (f.act.kind === "plant" && f.act.water && Math.floor(T * 6) % 2) { rect(ctx, x + 10 * f.dir, y0 + 21, 1, 1, C.blue3); rect(ctx, x + 11 * f.dir, y0 + 24, 1, 1, C.blue3); }
    if (app.open() === f.id) for (const [i, wd] of [5, 3, 1].entries()) rect(ctx, x - Math.floor(wd / 2), y0 - 6 + i, wd, 1, TERRACOTTA);
    if (over(x0 + 4, y0, x0 + 18, y + 1)) hovered = { b, x, y: y0 };
  }
  function drawSleeper(ctx, f, b, T) {      // seen from above: the head on the pillow, a blanket, a plug
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), im = body(b.id, projectColor(b.cwd), "down", "stand", !b.background);
    if (im) ctx.drawImage(im, 0, 0, SW, 16, x - 11, y - 28, SW, 16);
    box(ctx, x - 11, y - 13, 22, 11, C.steel1);
    rect(ctx, x - 11, y - 13, 22, 2, C.steel2);
    rect(ctx, x - 11, y - 4, 22, 2, C.steel0);
    rect(ctx, x - 2, y - 9, 3, 1, C.gold2); rect(ctx, x - 1, y - 10, 1, 3, C.gold2);
    if (Math.floor(T * 2) % 2) rect(ctx, x + 6, y - 9, 2, 2, C.green1);
    zzz(ctx, f, x + 6, y - 24);
    if (over(x - 11, y - 28, x + 11, y)) hovered = { b, x, y: y - 28 };
  }
  const zed = (ctx, x, y) => { ctx.fillRect(x, y, 3, 1); ctx.fillRect(x + 1, y + 1, 1, 1); ctx.fillRect(x, y + 2, 3, 1); };
  function zzz(ctx, f, x, y) {
    ctx.fillStyle = C.steel3;
    for (let i = 0; i < 2; i++) { const k = (f.z * 0.4 + i * 0.5) % 1; zed(ctx, Math.round(x + k * 6), Math.round(y - k * 10)); }
  }

  // a unit: the cabinet with the project's name and screen, the long desktop where its bots sit
  function drawDesk(ctx, d, w, T, ri) {
    const { x0, y, w: dw, cab } = d;
    // the screen runs only while a bot sits at it typing; dim while one sits there idle
    const seated = (f) => f && f.legs === "sit" && !f.lying && (f.act.kind === "desk" || f.act.kind === "sleep");
    const busy = d.seats.some(([id]) => { const f = w.folk.get(id); return seated(f) && f.busy; });
    const here = d.seats.some(([id]) => seated(w.folk.get(id)));
    box(ctx, x0 + cab - 1, y - 9, dw - cab + 1, 2, WOOD);
    rect(ctx, x0 + cab, y - 7, dw - cab, 1, WOOD_D);
    box(ctx, x0 + dw - 3, y - 6, 2, 6, WOOD_D);
    box(ctx, x0, y - 13, cab, 13, WOOD_D);
    rect(ctx, x0, y - 13, cab, 1, WOOD);
    const label = projectLabel(d.cwd);
    tiny(ctx, label, Math.round(x0 + (cab - tinyW(label)) / 2), y - 9, C.steel3);
    const sx = Math.round(x0 + cab / 2 - 10);
    rect(ctx, sx + 9, y - 16, 2, 3, C.steel1); rect(ctx, sx + 6, y - 14, 8, 1, C.steel1);
    box(ctx, sx, y - 31, 20, 15, C.steel0);
    rect(ctx, sx + 1, y - 30, 18, 13, busy || here ? C.blue0 : NIGHT);
    if (busy || here) {
      ctx.fillStyle = busy ? d.color : shade(d.color, 0.45);
      const s = busy ? Math.floor(T * 5) : 0;
      for (let r = 0; r < 6; r++) ctx.fillRect(sx + 2 + ((r * 3) % 4), y - 29 + r * 2, 3 + ((r * 7 + s + d.cwd.length) * 5) % 12, 1);
    }
    // a project with no bots can be forgotten: an x on its cabinet
    if (!d.seats.length && over(x0, y - 32, x0 + cab, y)) {
      const h = hot(x0 + cab - 6, y - 32, x0 + cab + 2, y - 25, { kind: "forget", dir: d.cwd });
      tiny(ctx, "X", x0 + cab - 4, y - 31, h ? TERRACOTTA : C.steel2);
    }
  }
  // pair's chair, turned round: its sitter faces left, to the screen; the empty one at the end starts a bot
  function drawChair(ctx, x, y, d, plus) {
    const h = plus && hot(x - 8, y - 22, x + 9, y + 1, { kind: "spawn", dir: d.cwd });
    const dim = plus && !h, col = dim ? INK2 : C.steel0;
    box(ctx, x + 6, y - 15, 2, 9, col);
    box(ctx, x - 2, y - 7, 9, 2, col);
    rect(ctx, x + 2, y - 4, 1, 3, dim ? C.steel0 : C.steel1);
    rect(ctx, x - 1, y - 1, 7, 1, dim ? C.steel0 : C.steel1);
    if (plus) tiny(ctx, "+", x, y - 16, h ? d.color : C.steel1);
  }
  function drawBoard(ctx, b, w) {
    box(ctx, b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, C.steel3, C.steel1);
    rect(ctx, b.x0 + 2, b.y1, b.x1 - b.x0 - 4, 1, C.steel1);
    ctx.fillStyle = shade(C.blue2, 0.75);
    for (const s of w.board) {
      const x = b.x0 + 2 + s.x, y = b.y0 + 2 + s.y;
      if (s.t === "box") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x, y + s.h, s.w, 1); ctx.fillRect(x, y, 1, s.h); ctx.fillRect(x + s.w - 1, y, 1, s.h + 1); }
      else if (s.t === "line") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x + s.w - 2, y - 1, 1, 1); ctx.fillRect(x + s.w - 2, y + 1, 1, 1); }
      else ctx.fillRect(x, y, 1, 1);
    }
  }
  function drawDoor(ctx, x) {
    const y0 = 12, h = WALL - 14;
    box(ctx, x, y0, 12, h, WOOD);
    rect(ctx, x + 2, y0 + 2, 8, 5, WOOD_D); rect(ctx, x + 2, y0 + 9, 8, h - 11, WOOD_D);
    rect(ctx, x + 9, y0 + 8, 1, 1, C.gold2);
  }
  function drawClock(ctx, x, y) {
    const t = new Date();
    rect(ctx, x - 4, y - 3, 9, 7, INK); rect(ctx, x - 3, y - 4, 7, 9, INK);
    rect(ctx, x - 3, y - 2, 7, 5, C.steel4); rect(ctx, x - 2, y - 3, 5, 7, C.steel4);
    const hand = (ang, len) => { for (let i = 1; i <= len; i++) ctx.fillRect(Math.round(x + Math.sin(ang) * i), Math.round(y - Math.cos(ang) * i), 1, 1); };
    rect(ctx, x, y, 1, 1, INK);
    hand(((t.getHours() % 12) + t.getMinutes() / 60) / 12 * 2 * Math.PI, 2);
    ctx.fillStyle = C.red1; hand(t.getMinutes() / 60 * 2 * Math.PI, 3);
  }
  function drawWindow(ctx, w) {              // day or night outside, by this computer's clock
    const hour = new Date().getHours(), day = hour >= 7 && hour < 20, ww = w.x1 - w.x0, wh = w.y1 - w.y0;
    box(ctx, w.x0, w.y0, ww, wh, day ? C.blue2 : C.blue0, C.steel2);
    if (day) { rect(ctx, w.x0, w.y1 - 5, ww, 5, C.blue3); rect(ctx, w.x0 + 3, w.y0 + 3, 5, 1, C.steel4); rect(ctx, w.x0 + 2, w.y0 + 4, 7, 1, C.steel4); }
    else { rect(ctx, w.x0 + 4, w.y0 + 3, 1, 1, C.steel4); rect(ctx, w.x1 - 5, w.y0 + 8, 1, 1, C.steel4); rect(ctx, w.x1 - 6, w.y0 + 2, 2, 2, C.gold2); }
    rect(ctx, Math.round(w.x0 + ww / 2), w.y0, 1, wh, C.steel2); rect(ctx, w.x0, Math.round(w.y0 + wh / 2), ww, 1, C.steel2);
  }
  function drawMat(ctx, m) {
    box(ctx, m.x - 12, m.y - 24, 24, 23, "#1c2538");
    ctx.fillStyle = "#222d45"; for (let y = m.y - 21; y < m.y - 2; y += 3) ctx.fillRect(m.x - 12, y, 24, 1);
    box(ctx, m.x - 8, m.y - 23, 16, 6, C.steel2);
  }
  function drawTable(ctx, m) {
    box(ctx, m.x - 16, m.y - 11, 32, 4, WOOD);
    rect(ctx, m.x - 16, m.y - 8, 32, 1, WOOD_D);
    box(ctx, m.x - 13, m.y - 6, 2, 6, WOOD_D); box(ctx, m.x + 11, m.y - 6, 2, 6, WOOD_D);
    rect(ctx, m.x - 6, m.y - 10, 3, 2, C.steel4); rect(ctx, m.x + 4, m.y - 10, 3, 2, C.steel4);
  }
  function drawStool(ctx, st) {
    box(ctx, st.x - 4, st.y - 7, 8, 2, WOOD_D);
    rect(ctx, st.x - 3, st.y - 4, 1, 4, WOOD_D); rect(ctx, st.x + 2, st.y - 4, 1, 4, WOOD_D);
  }
  function drawCoffee(ctx, x0, y, brewing, T) {
    box(ctx, x0, y - 10, 20, 10, C.steel1);
    rect(ctx, x0 + 2, y - 7, 7, 6, C.steel0); rect(ctx, x0 + 11, y - 7, 7, 6, C.steel0);
    box(ctx, x0 + 6, y - 19, 8, 8, INK2);
    rect(ctx, x0 + 12, y - 17, 1, 1, C.red1);
    rect(ctx, x0 + 8, y - 14, 4, 1, C.steel3);
    if (brewing) { rect(ctx, x0 + 9, y - 13 + (Math.floor(T * 8) % 2), 1, 1, WOOD); rect(ctx, x0 + 10, y - 22 - (Math.floor(T * 4) % 2), 1, 1, C.steel3); }
    rect(ctx, x0 + 2, y - 13, 2, 2, C.white);
  }
  function drawCooler(ctx, c) {
    box(ctx, c.x - 3, c.y - 10, 6, 10, C.steel3);
    box(ctx, c.x - 2, c.y - 17, 4, 6, C.blue3);
    rect(ctx, c.x - 2, c.y - 14, 4, 3, C.blue2);
    rect(ctx, c.x - 1, c.y - 7, 2, 1, C.steel1);
  }
  function drawPrinter(ctx, pr, wait) {
    box(ctx, pr.x - 5, pr.y - 6, 10, 6, C.steel1);
    box(ctx, pr.x - 6, pr.y - 12, 12, 5, C.steel3);
    rect(ctx, pr.x - 4, pr.y - 10, 8, 1, INK2);
    rect(ctx, pr.x + 4, pr.y - 9, 1, 1, C.green1);
    if (wait !== undefined) { const k = Math.min(4, Math.floor(wait * 2)); rect(ctx, pr.x - 3, pr.y - 13 - k, 6, k + 1, C.steel4); }
  }
  function drawArmchair(ctx, c) {
    box(ctx, c.x - 8, c.y - 9, 14, 6, C.purple0);
    box(ctx, c.x + 4, c.y - 18, 4, 15, C.purple0);
    box(ctx, c.x - 9, c.y - 11, 3, 7, C.purple1);
    rect(ctx, c.x - 7, c.y - 3, 2, 3, INK); rect(ctx, c.x + 4, c.y - 3, 2, 3, INK);
    rect(ctx, c.x + 13, c.y - 18, 1, 18, C.steel1);     // a lamp beside it
    box(ctx, c.x + 10, c.y - 22, 7, 4, C.gold2);
  }
  function drawPlant(ctx, pl) {
    box(ctx, pl.x - 3, pl.y - 5, 6, 5, C.gold0);
    const leaves = [[-4, -9, C.green0], [2, -11, C.green1], [-1, -14, C.green1], [3, -7, C.green0], [-3, -12, C.green1]];
    ctx.fillStyle = INK;
    for (const [dx, dy] of leaves) ctx.fillRect(pl.x + dx - 1, pl.y + dy - 1, 4, 5);
    for (const [dx, dy, c] of leaves) { ctx.fillStyle = c; ctx.fillRect(pl.x + dx, pl.y + dy, 2, 3); }
  }
  function drawPong(ctx, g, w) {
    box(ctx, g.x - 14, g.y - 12, 28, 5, C.scrub1);
    rect(ctx, g.x - 14, g.y - 10, 28, 1, C.steel4);
    rect(ctx, g.x - 14, g.y - 7, 28, 1, "#10363f");
    rect(ctx, g.x - 1, g.y - 16, 2, 5, INK); rect(ctx, g.x, g.y - 15, 1, 4, C.steel3);
    box(ctx, g.x - 12, g.y - 6, 2, 6, C.steel1); box(ctx, g.x + 10, g.y - 6, 2, 6, C.steel1);
    // the ball, once both players are at the table
    if ([...w.folk.values()].filter((f) => f.act.kind === "pong" && f.act.there).length === 2) {
      const { t, from } = w.pong, a = from ? 1 : -1, x = g.x + a * 18 * (1 - 2 * t), k = t < 0.5 ? t * 2 : (t - 0.5) * 2;
      const y = g.y - 12 - Math.round(Math.sin(k * Math.PI) * (t < 0.5 ? 6 : 8)) - 6 * (1 - Math.min(1, t * 4));
      rect(ctx, Math.round(x) - 1, Math.round(y) - 1, 3, 3, INK); rect(ctx, Math.round(x), Math.round(y), 1, 1, C.white);
    }
  }

  // ---- the pointer, in dots
  const dots = (e) => { const r = cv.getBoundingClientRect(); return { x: (e.clientX - r.left) / K, y: (e.clientY - r.top) / K }; };
  cv.addEventListener("pointermove", (e) => { pointer = dots(e); draw(); });
  cv.addEventListener("pointerleave", () => { pointer = null; draw(); });
  cv.addEventListener("click", (e) => {
    pointer = dots(e); draw();
    const h = [...hots].reverse().find((h) => over(h.x0, h.y0, h.x1, h.y1));
    if (h) app.act(h.action);
  });

  // ---- size and run, twelve frames a second like a 16-bit game
  function size() {
    const r = cv.parentElement.getBoundingClientRect();
    // dots big enough that the room is 220 to 330 of them wide, as dense as pair's office
    K = Math.max(2, Math.min(4, Math.round(r.width / 280)));
    W = Math.ceil(r.width / K); H = Math.ceil(r.height / K);
    if (cv.width !== W || cv.height !== H) { cv.width = W; cv.height = H; }
    cv.style.width = W * K + "px"; cv.style.height = H * K + "px";
    draw();
  }
  let last = 0, acc = 0, tickDt = 0;
  function frame(ts) {
    requestAnimationFrame(frame);
    if (document.hidden) { last = ts; return; }
    const dt = Math.min(0.1, (ts - last) / 1000 || 0); last = ts; acc += dt;
    if (acc < 1 / 12) return;
    tickDt = Math.min(acc, 0.25); acc = 0;
    draw();
  }
  new ResizeObserver(size).observe(cv.parentElement);
  size();
  requestAnimationFrame(frame);
  window.__room = { worlds, size: () => ({ W, H, K }) };   // for poking at it from the console
  return { talk, size, draw };
}
