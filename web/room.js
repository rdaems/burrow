// The room: one machine, as pair's office (rdaems/pair-prompting, app/floor.js): its furniture, its
// breaks, its mats, its robots (pixel.js). Every project (a working directory) is one unit: a cabinet
// with the project's screen and name at its left end, a long desktop to the right where its bots sit
// facing the screen, and an empty chair at the end (+) that starts another bot there.
// Where a bot is says what it is doing:
//   busy                at its project's desk, typing
//   idle                by its desk, or on a break: coffee, the printer, the cooler, ping-pong,
//                       the plants, the window, the armchair, a meeting at the whiteboard
//   idle 10 minutes     charging on a mat
//   sends a message     walks over to whoever it wrote to and stands there talking for a moment
//   waits for you       at its desk with its hand up (a permission prompt), its screen's cursor blinking
//   stopped             walks out through the door (and a new one comes in through it)
// Free, two idle bots play ping-pong or chess; one alone at the chess board waits for an opponent.
// The account's limits are in the room too: the coffee pot holds what's left of the five-hour
// session, the cooler's bottle what's left of the week (Claude Code's own /usage).
// Nothing else signals anything; what they say is never shown, only that they talk.
// The machines are lettered on the wall, the current one underlined.
//
// What you do happens in the room as well: click a bot to open its session, the + chair for a new
// bot, the empty cabinet for a project; drag furniture where you want it; carry a bot to the door
// to stop it; drop a file on a bot to hand it over.
//
// startRoom(canvas, app) with app = { state(), cur(), open(), hl(), hover(id), act(action) };
// returns { talk(room, from, to), size(), draw() }. The room is always W x H dots.

function startRoom(cv, app) {
  const C = pixelAvatar.C, INK = C.ink, INK2 = C.ink2;
  const NIGHT = "#0d0a16", TERRACOTTA = "#d97757", RED = "#ff6b5a", OPEN = "#8a4a3a", GLASS = "#1a1f2e";
  const WOOD = "#7a4a35", WOOD_D = "#4a2e2a", WALLC = "#1e1a2c", FLOOR = "#130f1b", PLANK = "#19141f";
  const PROJECT = ["#4a76b8", "#e8402a", "#2fa087", "#f2a531", "#9a74d6", "#5fcf4a", "#f06b72", "#8fc0e8"];
  const WALL = 26, TALK = 4, SEAT = 18, ADD_W = 30, ROW = 46;
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

  // halo for a highlighted bot: one dot around its outline
  const halos = new Map();
  function halo(im, col) {
    if (!halos.has(col)) halos.set(col, new WeakMap());
    const hm = halos.get(col);
    if (hm.has(im)) return hm.get(im);
    const d = im.getContext("2d").getImageData(0, 0, SW, SH).data, c = document.createElement("canvas");
    c.width = SW; c.height = SH;
    const x = c.getContext("2d"), on = (i, j) => i >= 0 && j >= 0 && i < SW && j < SH && d[(j * SW + i) * 4 + 3] > 0;
    x.fillStyle = col;
    for (let j = 0; j < SH; j++) for (let i = 0; i < SW; i++)
      if (!on(i, j) && (on(i - 1, j) || on(i + 1, j) || on(i, j - 1) || on(i, j + 1))) x.fillRect(i, j, 1, 1);
    hm.set(im, c);
    return c;
  }

  // ---- the room: always the same size; its furniture stands where it was put (dragged by hand,
  // kept per machine in rooms.json), or where it stands by default. A project's unit that has no
  // place yet gets the first free one, and keeps it.
  // as dense as pair's office: two CSS pixels a dot, a room as wide as pair's margin; as tall as the
  // window lets it be (what stands at the bottom stays at the bottom)
  const W = 256, K = 2;
  let H = 400, FLOORY = H - 6, DEF = {};
  function defaults() {
    DEF = {
      meet: [W / 2, WALL + 34], pong: [60, 62], chess: [70, FLOORY - 32], mats: [176, FLOORY - 30],
      coffee: [3, FLOORY], cooler: [36, FLOORY - 1], printer: [W / 2 + 4, FLOORY], armchair: [W - 34, FLOORY],
      plant0: [W - 8, FLOORY], plant1: [W - 12, 64], board: [W / 2 - 16, 5], window: [W - 38, 5],
    };
  }
  const SLOTS = [104, 150, 196, 242, 288, 334].flatMap((y) => [[10, y], [134, y]]);
  const ON_WALL = new Set(["board", "window"]);
  // what each piece covers, from where it stands
  function foot(key, [x, y], u) {
    if (u) return [x - 2, y - 32, x + u.w + 2, y + 2];
    switch (key) {
      case "add": return [x - 2, y - 14, x + ADD_W + 2, y + 2];
      case "meet": return [x - 28, y - 14, x + 28, y + 4];
      case "pong": return [x - 36, y - 20, x + 36, y + 8];
      case "chess": return [x - 19, y - 17, x + 19, y + 2];
      case "mats": return [x - 42, y - 26, x + 42, y + 3];
      case "coffee": return [x, y - 22, x + 20, y + 1];
      case "cooler": return [x - 4, y - 22, x + 4, y + 1];
      case "printer": return [x - 7, y - 14, x + 7, y + 1];
      case "armchair": return [x - 10, y - 23, x + 18, y + 1];
      case "board": return [x, y, x + 32, y + 14];
      case "window": return [x, y, x + 18, y + 14];
      default: return [x - 5, y - 16, x + 5, y + 1];     // a plant
    }
  }
  const meets = (a, b) => a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
  // keep a piece in the room: on the floor, or along the wall for what hangs there
  function clampAt(key, at, u) {
    const r = foot(key, at, u), [x, y] = at;
    if (ON_WALL.has(key)) return [Math.round(Math.max(24 - (r[0] - x), Math.min(W - 18 - (r[2] - x), x))), y];
    return [Math.round(Math.max(2 - (r[0] - x), Math.min(W - 2 - (r[2] - x), x))),
            Math.round(Math.max(WALL + 2 - (r[1] - y), Math.min(H - 1 - (r[3] - y), y)))];
  }

  function layout(room, bots, moving) {
    const saved = room.layout || {}, placed = {};
    const order = [...(room.dirs || [])];
    for (const b of bots) if (!order.includes(b.cwd)) order.push(b.cwd);
    const units = order.map((cwd) => {
      const n = bots.filter((b) => b.cwd === cwd).length, cab = Math.max(tinyW(projectLabel(cwd)) + 6, 22);
      return { key: "u:" + cwd, cwd, cab, w: cab + 4 + (n + 1) * SEAT };
    });
    const where = (key, u) => {
      if (moving && moving.key === key) return moving.at;
      if (saved[key]) return clampAt(key, saved[key], u);       // the window may be shorter than when it was put there
      return DEF[key] && !u ? DEF[key] : null;
    };
    const taken = [];
    for (const k of Object.keys(DEF)) { const at = where(k); if (!ON_WALL.has(k)) taken.push(foot(k, at)); }
    for (const u of units) { u.at = where(u.key, u); if (u.at) taken.push(foot(u.key, u.at, u)); }
    let addAt = where("add");
    if (addAt) taken.push(foot("add", addAt));
    // a free place, for a unit (with room to grow by a chair) or the add cabinet
    const free = (key, u) => {
      const wide = u && { ...u, w: u.w + SEAT };
      const cands = SLOTS.concat([...Array(40)].flatMap((_, j) => [...Array(30)].map((_, i) => [6 + i * 8, 70 + j * 6])));
      for (const c of cands) {
        const r = foot(key, c, wide);
        if (r[0] >= 2 && r[2] <= W - 2 && r[1] >= WALL + 2 && r[3] <= H && !taken.some((t) => meets(t, r))) { taken.push(r); return c; }
      }
      return SLOTS[0];
    };
    for (const u of units) if (!u.at) { u.at = free(u.key, u); placed[u.key] = u.at; }
    if (!addAt) { addAt = free("add"); placed.add = addAt; }

    const P = (k) => where(k);
    const desks = units.map((u) => {
      const [x, y] = u.at, mine = bots.filter((b) => b.cwd === u.cwd);
      const seat = (k) => x + u.cab + 4 + SEAT / 2 + k * SEAT;
      return { key: u.key, cwd: u.cwd, x0: x, w: u.w, cab: u.cab, y, color: projectColor(u.cwd),
        seats: mine.map((b, k) => [b.id, seat(k)]), plus: seat(mine.length), at: u.at, u };
    });
    const [bx, by] = P("board"), [wx, wy] = P("window");
    const xy = (k) => { const [x, y] = P(k); return { x, y }; };
    const matsAt = xy("mats");
    const L = {
      desks, add: { x0: addAt[0], y0: addAt[1] - 13, x1: addAt[0] + ADD_W, y1: addAt[1] },
      floorY: FLOORY, board: { x0: bx, y0: by, x1: bx + 32, y1: by + 14 }, window: { x0: wx, y0: wy, x1: wx + 18, y1: wy + 14 },
      meet: xy("meet"), pong: xy("pong"), chess: xy("chess"), mats: [-28, 0, 28].map((d) => ({ x: matsAt.x + d, y: matsAt.y })),
      cooler: xy("cooler"), plants: [xy("plant0"), xy("plant1")], printer: xy("printer"), chair: xy("armchair"),
      coffeeAt: xy("coffee"), floor: { x0: 14, y0: WALL + 8, x1: W - 14, y1: FLOORY - 4 }, placed,
    };
    L.coffee = { x: L.coffeeAt.x + 27, y: L.coffeeAt.y - 10 };
    // what can be picked up and moved, front to back
    L.pieces = [
      ...desks.map((d) => ({ key: d.key, at: d.at, u: d.u, r: foot(d.key, d.at, d.u) })),
      { key: "add", at: addAt, r: foot("add", addAt) },
      ...Object.keys(DEF).map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
    ].sort((a, b) => b.r[3] - a.r[3]);
    L.coolerSpot = (side) => ({ x: L.cooler.x + (side ? 10 : -8), y: L.cooler.y + 1 });
    L.pongSpot = (side) => ({ x: L.pong.x + (side ? 22 : -22), y: L.pong.y + 1 });
    L.chessSeat = (side) => ({ x: L.chess.x + (side ? 13 : -13), y: L.chess.y });
    L.meetSeat = (i) => { const s = [[-23, 0, 1], [23, 0, -1], [-8, -5, 1], [8, -5, -1]][i]; return s && { x: L.meet.x + s[0], y: L.meet.y + s[1], dir: s[2] }; };
    L.seat = (id) => { for (const d of desks) for (const [i, x] of d.seats) if (i === id) return { x, y: d.y }; return null; };
    L.free = (x, y) => !L.pieces.some((p) => !ON_WALL.has(p.key) && x >= p.r[0] - 6 && x <= p.r[2] + 6 && y >= p.r[1] && y <= p.r[3] + 4);
    return L;
  }

  // ---- everyone, per machine: what each bot is doing, kept while you look at another machine
  const worlds = new Map();
  function world(i) {
    if (!worlds.has(i)) worlds.set(i, { folk: new Map(), meeting: null, lastMeeting: -60, board: [], pong: { t: 0, from: 0 },
      chess: { board: newBoard(), t: 0, turn: 0 }, T: 0, born: performance.now() });
    return worlds.get(i);
  }
  // the chess board as pair keeps it: 64 squares from a1, upper case white, "." empty
  const START = "RNBQKBNRPPPPPPPP" + ".".repeat(32) + "pppppppprnbqkbnr";
  const newBoard = () => START;
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
  // seated somewhere that can be moved: stays on its seat
  function sitAt(f, s, v, dt) {
    if (f.legs === "sit" && Math.hypot(s.x - f.x, s.y - f.y) < 40 && f.sat) { f.x = s.x; f.y = s.y; return true; }
    f.sat = walk(f, s, v, dt);
    return f.sat;
  }

  // `from` sent `to` a message: it walks over
  function talk(ri, from, to) {
    const w = world(ri), f = w.folk.get(from), g = w.folk.get(to);
    if (!f || !g) return;
    for (const [a, b] of [[f, g], [g, f]]) if (a.act.kind === "visit" && a.act.to === b.id) { a.act.left = Math.max(a.act.left, TALK); return; }
    f.lying = false; f.carry = null; f.sat = false;
    f.act = { kind: "visit", to, left: TALK, wave: 0, msg: true };
  }

  function think(w, bots, L, dt, sleepAfter, usage, game) {
    const now = performance.now() / 1000, folk = w.folk;
    // the bot you play chess with sits at the table until the game is over (or it gets work)
    const vs = game && !game.result ? (game.white.id === "you" ? game.black.id : game.white.id) : null;
    for (const f of folk.values()) {
      if (f.id === vs && !(f.act.kind === "chess" && f.act.you) && !f.work) { end(f); f.act = { kind: "chess", side: game.white.id === vs ? 0 : 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "chess" && (f.act.you ? f.id !== vs : !!vs)) end(f);   // the table is taken: the others get up
      if (f.id === vs && game.sans.length !== w.plies) { if (w.plies !== undefined && (game.sans.length % 2 === 1) === (game.white.id === vs)) f.moved = 0.9; }
    }
    w.plies = game ? game.sans.length : undefined;
    // the bot you play ping-pong with stands at the table, at the far end
    const pp = app.pong(), pv = pp && pp.id;
    for (const f of folk.values()) {
      if (f.id === pv && !(f.act.kind === "pong" && f.act.you) && !f.work) { end(f); f.act = { kind: "pong", side: 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "pong" && (f.act.you ? f.id !== pv : !!pv)) end(f);
    }
    for (const b of bots) {
      const f = folk.get(b.id), work = b.busy || b.waiting;
      f.chat = Math.max(0, f.chat - dt);
      f.busy = b.busy; f.waiting = b.waiting;
      if (work !== f.work) {
        f.work = work; f.idleSince = now;
        // to work, whatever it was doing (a message still gets delivered first, a bot by the door stays)
        if (work && !(f.act.kind === "visit" && f.act.msg) && f.act.kind !== "door") { end(f); f.act = { kind: "desk" }; }
        if (!work && f.act.kind === "desk") f.act = { kind: "stand", left: 0.5 + (fnv(b.id) % 20) / 10 };
      }
    }
    if (w.meeting && (w.meeting.left -= dt) <= 0) {
      w.meeting = null;
      for (const f of folk.values()) if (f.act.kind === "meeting" || f.act.kind === "present") end(f);
    }
    const playing = (kind) => [...folk.values()].filter((f) => f.act.kind === kind && f.act.there && f.act.with).length === 2;
    // the ball goes back and forth while two are at the table
    if ((playing("pong") || (app.pong() && app.pong().ball)) && (w.pong.t += dt / 0.8) >= 1) { w.pong.t = 0; w.pong.from = 1 - w.pong.from; }
    // a chess move every few seconds while two sit at the board
    if (playing("chess") && (w.chess.t += dt) > 3.5) { w.chess.t = 0; chessMove(w.chess); }
    // an idle bot's place: at the end of its project's desk, by the empty chair, one per project
    // (the first idle one); the others are off on a break
    const homes = new Map();
    for (const d of L.desks) {
      const first = d.seats.find(([id]) => { const f = folk.get(id); return f && !f.work && f.act.kind === "stand"; });
      if (first) homes.set(first[0], { x: Math.min(W - 8, d.x0 + d.w + 8), y: d.y + 1 });
    }
    for (const b of bots) if (!folk.get(b.id).held) thinkOne(w, folk.get(b.id), L, now, dt, homes.get(b.id), sleepAfter, usage);
    // two standing about in the same place: they step apart
    const loit = [...folk.values()].filter((f) => f.legs === "stand" && f.act.kind === "loiter");
    for (const a of loit) for (const b of loit) if (a.id < b.id && Math.abs(a.x - b.x) < 12 && Math.abs(a.y - b.y) < 4) {
      const d = a.x <= b.x ? -1 : 1; a.x += d * 8 * dt; b.x -= d * 8 * dt;
    }
  }
  function chessMove(g) {
    // two bots between themselves: whoever's turn it is moves a piece forward to a free square or
    // takes one diagonally ahead (it is a game for them, not chess); you play the real thing
    const b = [...g.board], white = g.turn === 0, ahead = white ? 8 : -8, moves = [];
    const mine = (p) => p !== "." && (p === p.toUpperCase()) === white;
    b.forEach((p, s) => {
      if (!mine(p)) return;
      const f = s % 8;
      if (b[s + ahead] === ".") moves.push([s, s + ahead, 1]);
      for (const d of [-1, 1]) { const t = s + ahead + d; if (f + d >= 0 && f + d < 8 && b[t] && b[t] !== "." && !mine(b[t])) moves.push([s, t, 4]); }
    });
    if (moves.length) {
      let k = Math.random() * moves.reduce((a, m) => a + m[2], 0), m = moves[0];
      for (const c of moves) if ((k -= c[2]) < 0) { m = c; break; }
      b[m[1]] = b[m[0]]; b[m[0]] = ".";
    }
    g.board = moves.length && b.filter((p) => p !== ".").length > 12 ? b.join("") : newBoard();
    g.turn = 1 - g.turn;
    g.last = performance.now();
  }

  function thinkOne(w, f, L, now, dt, home, sleepAfter, usage) {
    const folk = w.folk, others = [...folk.values()].filter((g) => g !== f && !g.leaving), v = 22;
    const seat = L.seat(f.id);
    // waiting for an answer never falls asleep: it sits at its desk with its hand up
    const sleepy = !f.work && now - f.idleSince > sleepAfter;
    if (sleepy && f.act.kind !== "sleep" && !(f.act.kind === "visit" && f.act.msg) && f.act.kind !== "door") { end(f); f.act = { kind: "sleep", there: false }; }
    if (!sleepy && f.act.kind === "sleep") end(f);
    const act = f.act, partner = () => { const g = act.with && folk.get(act.with); return g && g.act.kind === act.kind && g.act.with === f.id ? g : null; };
    if (!["desk", "meeting", "chess", "chair", "sleep"].includes(act.kind)) f.sat = false;
    switch (act.kind) {
      case "desk":
        if (seat && sitAt(f, seat, v, dt)) {
          f.dir = -1; f.legs = "sit"; f.carry = null;
          f.arms = f.waiting ? "wave" : f.busy ? (Math.floor(w.T * 6) % 2 ? "fwd2" : "fwd1") : "down";
        }
        break;
      case "stand": {
        const there = !home || walk(f, home, v, dt);   // back to its cabinet first, if that is its place
        if (!there) break;
        f.legs = "stand"; f.arms = f.carry || "down";
        if (home) f.dir = -1;
        if ((act.left -= dt) > 0) break;
        f.carry = null; f.act = pickBreak(w, f, L, others);
        break;
      }
      case "loiter":
        if (walk(f, act, v, dt)) { f.arms = f.carry || "down"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      case "coffee":
        // the pot holds what is left of the session's limit: an empty one gives nothing
        if (act.stage === 0) { if (walk(f, L.coffee, v, dt)) { f.dir = -1; f.arms = "reach"; act.stage = 1; } }
        else {
          act.wait += dt; const dry = usage.session && usage.session.used >= 100;
          f.arms = act.wait > 2 && !dry ? "mug" : "reach";
          if (act.wait > 2.8) { f.carry = dry ? null : "mug"; f.act = rest(); }
        }
        break;
      case "print":
        if (act.stage === 0) { if (walk(f, { x: L.printer.x - 9, y: L.printer.y }, v, dt)) { f.dir = 1; f.arms = "down"; act.stage = 1; } }
        else if ((act.wait += dt) > 2.4) { f.carry = "paper"; f.act = rest(); }
        break;
      case "cooler": {
        const s = L.coolerSpot(act.side), g = folk.get(act.with);
        if (!g) { f.act = rest(); break; }
        if (walk(f, s, v, dt)) {
          f.dir = act.side ? -1 : 1; f.arms = "down";
          if (Math.abs(g.x - f.x) < 24 && Math.abs(g.y - f.y) < 4 && Math.random() < dt * 0.4) f.chat = 1.2;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "pong": {
        if (!act.you && !partner()) { f.act = rest(); break; }
        act.there = walk(f, L.pongSpot(act.side), v, dt);
        if (act.there) {
          f.dir = act.side ? -1 : 1;
          f.arms = w.pong.from !== act.side && w.pong.t > 0.75 ? "fwd1" : "fwd2";
          if ((act.left -= dt) <= 0) { f.act = rest(); }
        }
        break;
      }
      case "chess": {
        // alone at the board it waits for someone to play, studying it; two play until it's over; against
        // you it plays until the game is over
        if (act.with && !partner()) { act.with = null; act.left = Math.min(act.left, rnd(8, 16)); }
        act.there = sitAt(f, L.chessSeat(act.side), v, dt);
        if (act.there) {
          f.dir = act.side ? -1 : 1; f.legs = "sit";
          const moved = act.you ? (f.moved = Math.max(0, (f.moved || 0) - dt)) > 0 : act.with && w.chess.turn !== act.side && performance.now() - (w.chess.last || 0) < 500;
          f.arms = moved ? "reach" : "down";
          if (!act.with && Math.random() < dt * 0.05) f.chat = 0.8;
        }
        if ((act.left -= dt) <= 0) { const g = partner(); if (g) g.act = rest(); f.act = rest(); }
        break;
      }
      case "visit": {
        const g = folk.get(act.to);
        if (!g) { f.act = f.work ? { kind: "desk" } : rest(); break; }
        // a seated one faces its screen (left): stand behind its chair; else beside it, on the
        // side it comes from, as in pair
        const side = f.x < g.x ? -1 : 1, seated = g.legs === "sit";
        const spot = seated ? { x: g.x + 20, y: g.y + 1 } : { x: g.x + side * 14, y: g.y + 1 };
        if (walk(f, spot, act.msg ? 34 : v + 6, dt)) {
          f.dir = spot.x > g.x ? -1 : 1;
          f.arms = (act.wave -= dt) > 0 ? "wave" : "down";
          if (f.chat <= 0 && Math.random() < dt * 1.5) f.chat = 1;
          // visited: an idle one stops and faces the visitor, and talks back now and then
          if (!g.work && (g.act.kind === "stand" || g.act.kind === "loiter")) { g.act = { kind: "stand", left: 2 }; g.legs = "stand"; g.dir = -f.dir; }
          if (g.chat <= 0 && Math.random() < dt * 1.2) g.chat = 1;
          if ((act.left -= dt) <= 0) f.act = f.work ? { kind: "desk" } : rest();
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
        if (walk(f, { x: Math.round((L.window.x0 + L.window.x1) / 2), y: WALL + 8 }, v, dt)) {
          f.arms = "down"; f.dir = Math.sin(w.T * 0.7 + f.z) > 0 ? 1 : -1;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      case "chair":
        if (sitAt(f, { x: L.chair.x - 2, y: L.chair.y - 1 }, v, dt)) { f.dir = -1; f.legs = "sit"; f.arms = "book"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      case "present":
        if (!w.meeting || w.meeting.host !== f.id) { f.act = rest(); break; }
        if (walk(f, { x: L.board.x0 - 6, y: WALL + 3 }, v, dt)) {
          f.dir = 1; f.arms = Math.floor(w.T * 2) % 3 ? "reach" : "down";
          if ((act.wait += dt) > 0.6 && w.board.length < 40) { act.wait = 0; stroke(w, L); }
          if (Math.random() < dt * 0.3) f.chat = 1;
        }
        break;
      case "meeting": {
        const s = L.meetSeat(act.seat);
        if (!s || !w.meeting) { f.act = rest(); break; }
        if (sitAt(f, s, v, dt)) { f.dir = s.dir; f.legs = "sit"; f.arms = "down"; if (Math.random() < dt * 0.08) f.chat = 1; }
        break;
      }
      case "sleep": {
        // on a free mat if there is one, else asleep at the desk
        const mat = L.mats.find((m) => !others.some((g) => g.act.kind === "sleep" && g.act.mat === L.mats.indexOf(m)));
        const to = act.there ? (act.mat >= 0 ? L.mats[act.mat] : seat) : mat || seat;
        if (to && (act.there || walk(f, to, v, dt))) {
          if (!act.there) act.mat = mat ? L.mats.indexOf(mat) : -1;
          act.there = true; f.x = to.x; f.y = to.y; f.lying = act.mat >= 0;
          f.dir = f.lying ? 1 : -1; f.legs = "sit"; f.arms = "down"; f.z += dt;
        }
        break;
      }
      case "door":
        // put by the door to be stopped: it waits there a moment for a click on it
        if (walk(f, { x: 10, y: WALL + 3 }, v, dt)) {
          f.dir = -1; f.arms = "down";
          if ((act.left -= dt) <= 0) f.act = f.work ? { kind: "desk" } : rest();
        }
        break;
      default: f.act = rest();
    }
    if (f.legs !== "sit") f.lying = false;
  }

  // a break, chosen by a bot with nothing to do (pair's pickBreak, and chess)
  function pickBreak(w, f, L, others) {
    const r = Math.random();
    const mates = others.filter((g) => !g.work && (g.act.kind === "stand" || g.act.kind === "loiter"));
    const mate = mates.length ? any(mates) : null;
    const taken = (kind) => others.some((g) => g.act.kind === kind);
    // someone waits at the chess board: most likely, it gets a game
    const waiter = others.find((g) => g.act.kind === "chess" && !g.act.with && !g.act.you && g.act.there);
    if (waiter && r < 0.6) {
      const left = rnd(40, 90);
      waiter.act.with = f.id; waiter.act.left = left;
      return { kind: "chess", side: 1 - waiter.act.side, with: waiter.id, left, there: false };
    }
    if (mates.length >= 2 && !w.meeting && w.T - w.lastMeeting > 60 && r < 0.08) {
      w.meeting = { host: f.id, left: rnd(22, 38) }; w.lastMeeting = w.T; w.board = [];
      mates.slice(0, 4).forEach((g, i) => { g.carry = null; g.act = { kind: "meeting", seat: i }; });
      return { kind: "present", wait: 0 };
    }
    if (r < 0.2 && !taken("coffee")) return { kind: "coffee", stage: 0, wait: 0 };
    if (r < 0.28 && !taken("print")) return { kind: "print", stage: 0, wait: 0 };
    if (r < 0.38 && mate) {
      const left = rnd(10, 22);
      mate.carry = null; mate.act = { kind: "cooler", side: 1, left, with: f.id };
      return { kind: "cooler", side: 0, left, with: mate.id };
    }
    if (r < 0.5 && mate && !taken("pong") && !app.pong()) {
      const left = rnd(15, 35);
      mate.carry = null; mate.act = { kind: "pong", side: 1, left, with: f.id, there: false };
      return { kind: "pong", side: 0, left, with: mate.id, there: false };
    }
    if (r < 0.6 && !taken("chess") && !(app.state().rooms[app.cur()].chess && !app.state().rooms[app.cur()].chess.result)) return { kind: "chess", side: Math.random() < 0.5 ? 0 : 1, with: null, left: rnd(20, 45), there: false };
    if (r < 0.65 && mate) return { kind: "visit", to: mate.id, left: rnd(6, 12), wave: 0.8, msg: false };
    const plants = L.plants.map((_, i) => i).filter((i) => !others.some((g) => g.act.kind === "plant" && g.act.i === i));
    if (r < 0.72 && plants.length) return { kind: "plant", i: any(plants), wait: 0 };
    if (r < 0.79 && !taken("window")) return { kind: "window", left: rnd(4, 9) };
    if (r < 0.87 && !taken("chair")) return { kind: "chair", left: rnd(15, 35) };
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
  let pointer = null, hots = [], hovered = null, tip = null, lastL = null, moving = null, held = null, hoverId = null;
  const over = (x0, y0, x1, y1) => !!pointer && pointer.x >= x0 && pointer.x < x1 && pointer.y >= y0 && pointer.y < y1;
  const hot = (x0, y0, x1, y1, action) => { hots.push({ x0, y0, x1, y1, action }); return over(x0, y0, x1, y1) && !moving && !held; };
  const box = (ctx, x, y, w, h, fill, edge = INK) => { ctx.fillStyle = edge; ctx.fillRect(x - 1, y - 1, w + 2, h + 2); ctx.fillStyle = fill; ctx.fillRect(x, y, w, h); };
  const rect = (ctx, x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); };
  const say = (u, what) => u ? `${Math.max(0, 100 - u.used)}% of the ${what} left${u.resets ? ", new at " + u.resets.replace(/^.* at /, "") : ""}` : `the ${what}'s limit: not known yet`;

  function draw() {
    const ctx = cv.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    const s = app.state(), ri = app.cur(), room = s && s.rooms[ri];
    hots = []; tip = null;
    rect(ctx, 0, 0, W, H, FLOOR);
    ctx.fillStyle = PLANK;
    for (let y = WALL + 4, k = 0; y < H; y += 6, k++) { ctx.fillRect(0, y, W, 1); for (let x = (k % 2) * 13; x < W; x += 26) ctx.fillRect(x, y - 5, 1, 5); }
    rect(ctx, 0, 0, W, WALL, WALLC);
    rect(ctx, 0, WALL - 2, W, 2, INK2);
    rect(ctx, W - 1, 0, 1, H, INK2);
    if (!room) return;
    const w = world(ri), bots = room.bots || [], usage = room.usage || {};

    // the machines, lettered on the wall
    let tx = 22;
    s.rooms.forEach((r, i) => {
      const tw = tinyW(r.name), h = i !== ri && hot(tx - 2, 0, tx + tw + 2, 12, { kind: "room", i });
      tiny(ctx, r.name, tx, 4, i === ri ? C.steel4 : h ? C.steel3 : C.steel1);
      tx += tw + 7;
    });
    tiny(ctx, "+", tx, 4, hot(tx - 2, 0, tx + 5, 12, { kind: "addRoom" }) ? TERRACOTTA : C.steel1);
    const L = layout(room, bots, moving);
    lastL = L;
    if (Object.keys(L.placed).length) app.act({ kind: "layout", layout: { ...(room.layout || {}), ...L.placed } });
    room.layout = { ...(room.layout || {}), ...L.placed };

    // keep a figure per bot; one that turns up later comes in through the door, one that is
    // stopped walks out of it
    const ids = new Set(bots.map((b) => b.id));
    for (const f of w.folk.values()) if (!ids.has(f.id) && !f.leaving) { f.leaving = true; f.held = false; f.lying = false; f.sat = false; f.legs = "stand"; }
    const late = performance.now() - w.born > 3000;
    for (const b of bots) {
      if (!w.folk.has(b.id)) {
        const at = late ? { x: 10, y: WALL + 3 } : L.seat(b.id) || { x: W / 2, y: H / 2 };
        const work = b.busy || b.waiting;
        w.folk.set(b.id, { id: b.id, x: at.x, y: at.y, dir: -1, arms: "down", legs: late ? "stand" : "sit", step: 0, sat: !late,
          act: work ? { kind: "desk" } : rest(), carry: null, lying: false, busy: b.busy, waiting: b.waiting, work,
          idleSince: performance.now() / 1000, chat: 0, z: Math.random() * 2 });
      }
      w.folk.get(b.id).b = b;
    }
    const door = performance.now() - (w.doorAt || 0) < 900;
    if (tickDt > 0) {
      w.T += tickDt; think(w, bots, L, tickDt, s.demo ? 40 : 600, usage, room.chess);
      for (const f of w.folk.values()) if (f.leaving && walk(f, { x: 10, y: WALL + 1 }, 30, tickDt)) { w.folk.delete(f.id); w.doorAt = performance.now(); }
      tickDt = 0;
    }
    const T = w.T;
    drawDoor(ctx, 4, door || [...w.folk.values()].some((f) => f.act.kind === "door" || (f.leaving && f.y < WALL + 12))); drawClock(ctx, W - 9, 10);
    if (room.error) {                        // trouble goes on the wall, in red, instead of the whiteboard
      const nx = Math.max(tx + 11, 24), n = Math.max(0, Math.floor((W - 24 - nx) / 4));
      const msg = room.error.replace(/\s+/g, " ");
      tiny(ctx, msg.length > n ? msg.slice(0, n - 1) + "-" : msg, nx, 18, RED);
    } else drawBoard(ctx, L.board, w);
    drawWindow(ctx, L.window);

    // who is highlighted: the one under the pointer, the one whose session the pointer is over,
    // a project's bots while the pointer is on its screen
    const hl = new Set();
    if (app.hl()) hl.add(app.hl());
    // a bot and its seat at its project's desk are one thing: where the bot is only says what it
    // does. The pointer on either lights up both (and its session), and a click on either opens it
    if (hoverId) hl.add(hoverId);
    let seatHover = null;
    if (!moving && !held) for (const d of L.desks) for (const [id, x] of d.seats) if (over(x - 3, d.y - 16, x + 9, d.y + 1)) seatHover = { id, x, y: d.y };
    if (seatHover) hl.add(seatHover.id);

    // flat on the floor: the empty cabinet that adds a project, the mats, the ping-pong rug
    {
      const a = L.add, h = hot(a.x0 - 2, a.y0 - 10, a.x1 + 2, a.y1 + 1, { kind: "addDir" }), aw = a.x1 - a.x0;
      box(ctx, a.x0, a.y0, aw, 13, WOOD_D);
      rect(ctx, a.x0, a.y0, aw, 1, WOOD);
      rect(ctx, a.x0 + 3, a.y0 + 3, aw - 6, 7, h ? C.steel1 : C.steel0);
      tiny(ctx, "+", Math.round(a.x0 + aw / 2 - 1), a.y0 + 4, h ? C.steel4 : C.steel2);
      if (h) tip = { text: "project", x: a.x0 + aw / 2, y: a.y0 - 2 };
    }
    for (const m of L.mats) drawMat(ctx, m);
    drawRug(ctx, L.pong);

    // everything that stands, back to front
    const items = [];
    for (const f of w.folk.values()) if (!f.lying && !f.held) items.push([f.y, () => drawFigure(ctx, f, f.b, T, hl)]);
    for (const d of L.desks) {
      items.push([d.y - 1, () => drawDesk(ctx, d, w, T, hl)]);
      for (const [id, x] of d.seats) items.push([d.y - 0.5, () => drawChair(ctx, x, d.y, d, false, hl.has(id))]);
      items.push([d.y - 0.5, () => drawChair(ctx, d.plus, d.y, d, true)]);
    }
    items.push([L.meet.y - 1, () => drawTable(ctx, L.meet)]);
    for (const i of [0, 1]) { const st = L.meetSeat(i); items.push([st.y - 0.5, () => drawStool(ctx, st)]); }
    {
      const c = L.chess, g = room.chess && !room.chess.result ? room.chess : null;
      if (hot(c.x - 12, c.y - 17, c.x + 12, c.y, { kind: "chess" }))
        tip = { text: g ? "chess with " + (g.white.id === "you" ? g.black.name : g.white.name) : "chess: play an idle bot", x: c.x, y: c.y - 18 };
    }
    items.push([L.chess.y + 0.5, () => drawChess(ctx, L.chess, room.chess && !room.chess.result ? room.chess.board : w.chess.board)]);
    for (const i of [0, 1]) { const st = L.chessSeat(i); items.push([st.y - 0.5, () => drawStool(ctx, st)]); }
    const brewing = [...w.folk.values()].some((f) => f.act.kind === "coffee" && f.act.stage === 1);
    items.push([L.coffeeAt.y, () => drawCoffee(ctx, L.coffeeAt, brewing, T, usage.session)]);
    items.push([L.cooler.y, () => drawCooler(ctx, L.cooler, usage.week)]);
    const job = [...w.folk.values()].find((f) => f.act.kind === "print" && f.act.stage === 1);
    items.push([L.printer.y, () => drawPrinter(ctx, L.printer, job && job.act.wait)]);
    items.push([L.chair.y - 0.5, () => drawArmchair(ctx, L.chair)]);
    for (const pl of L.plants) items.push([pl.y, () => drawPlant(ctx, pl)]);
    items.push([L.pong.y - 0.5, () => drawPong(ctx, L.pong, w)]);
    if (hot(L.pong.x - 16, L.pong.y - 17, L.pong.x + 16, L.pong.y, { kind: "pong" })) tip = { text: "ping-pong: play an idle bot", x: L.pong.x, y: L.pong.y - 18 };
    // the limits, in the coffee pot (this five-hour session) and the cooler's bottle (this week)
    const c0 = L.coffeeAt;
    if (over(c0.x, c0.y - 22, c0.x + 20, c0.y + 1)) tip = { text: say(usage.session, "session"), x: c0.x + 10, y: c0.y - 24 };
    if (over(L.cooler.x - 4, L.cooler.y - 22, L.cooler.x + 4, L.cooler.y + 1)) tip = { text: say(usage.week, "week"), x: L.cooler.x, y: L.cooler.y - 24 };
    hovered = null;
    for (const f of w.folk.values()) if (f.lying) drawSleeper(ctx, f, f.b, T, hl);
    items.sort((a, b) => a[0] - b[0]);
    for (const [, d] of items) d();
    // a bot being carried, over everything
    for (const f of w.folk.values()) if (f.held) drawFigure(ctx, f, f.b, T, hl);
    if (held) hovered = null;
    if (!hovered && seatHover && !held) { const f = w.folk.get(seatHover.id); if (f && f.b) hovered = { b: f.b, x: seatHover.x + 3, y: seatHover.y - 16, seat: true }; }

    // talk, over everything: dots in a bubble, never the words
    for (const f of w.folk.values()) if (f.chat > 0 && !still) {
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - (f.lying ? 28 : 27) - 6;
      rect(ctx, bx - 1, by - 1, 11, 7, INK); rect(ctx, bx, by, 9, 5, C.steel4); rect(ctx, bx, by + 5, 1, 1, C.steel4);
      for (let i = 0; i < 3; i++) if ((Math.floor(f.chat * 4) + i) % 3) rect(ctx, bx + 2 + i * 2, by + 2, 1, 1, INK);
    }
    // by the door to be stopped: a click on the sign over it stops it
    for (const f of w.folk.values()) if (f.act.kind === "door" && f.b && Math.hypot(f.x - 10, f.y - WALL - 3) < 2) {
      const text = "stop " + f.b.name + "?", tw = tinyW(text), lx = 4, ly = WALL + 7;
      const h = hot(lx - 2, ly - 2, lx + tw + 2, ly + 7, { kind: "stop", id: f.id });
      box(ctx, lx - 2, ly - 2, tw + 4, 9, h ? TERRACOTTA : INK2);
      tiny(ctx, text, lx, ly, h ? INK : C.steel4);
    }
    // the bot under the pointer: its name over its head, and a click opens it
    if (hovered && !moving) {
      const { b, x, y } = hovered;
      tip = { text: b.background ? b.name + (b.waiting ? " - waits for you" : "") : b.name + " (in a terminal)", x, y: y - 2 };
      if (b.background) hots.push({ x0: -1e9, y0: -1e9, x1: 1e9, y1: 1e9, action: { kind: "open", id: b.id } });
    }
    if (tip && !held && !moving) {
      const t = tip.text, tw = tinyW(t), lx = Math.round(Math.max(2, Math.min(W - tw - 2, tip.x - tw / 2))), ly = Math.round(Math.max(WALL + 2, tip.y - 8));
      box(ctx, lx - 2, ly - 2, tw + 4, 9, INK2);
      tiny(ctx, t, lx, ly, C.steel4);
    }
    const id = hovered && !held ? hovered.b.id : null;
    if (id !== hoverId) { hoverId = id; app.hover(id); }
    const onHot = hots.some((h) => over(h.x0, h.y0, h.x1, h.y1));
    cv.style.cursor = moving || held ? "grabbing" : onHot ? "pointer" : pieceAt(pointer) ? "grab" : "default";
  }

  function drawFigure(ctx, f, b, T, hl) {
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), x0 = x - 11, y0 = y - 27;
    if (!f.held) rect(ctx, x - 5, y, 10, 1, "rgba(0,0,0,0.4)");
    const im = body(b.id, projectColor(b.cwd), f.held ? "up" : f.arms, f.held ? "walk1" : f.legs, !b.background);
    // its outline: terracotta while the pointer is on it or on its session, faint while its session is open
    const lit = hl.has(f.id) || (hovered && hovered.b.id === f.id) || (!hovered && over(x0 + 4, y0, x0 + 18, y + 1));
    const ring = f.leaving ? null : lit ? TERRACOTTA : app.open() === f.id ? OPEN : null;
    if (im) {
      ctx.save();
      if (f.dir < 0) { ctx.translate(x0 + SW, 0); ctx.scale(-1, 1); }
      const dx = f.dir < 0 ? 0 : x0;
      if (ring) ctx.drawImage(halo(im, ring), dx, y0);
      ctx.drawImage(im, dx, y0);
      ctx.restore();
    }
    if (f.act.kind === "sleep" && f.act.there) zzz(ctx, f, x + 2, y0 + 4);
    if (f.act.kind === "plant" && f.act.water && Math.floor(T * 6) % 2) { rect(ctx, x + 10 * f.dir, y0 + 21, 1, 1, C.blue3); rect(ctx, x + 11 * f.dir, y0 + 24, 1, 1, C.blue3); }
    if (!f.leaving && !f.held && over(x0 + 4, y0, x0 + 18, y + 1)) hovered = { b, x, y: y0 };
  }
  function drawSleeper(ctx, f, b, T, hl) {  // seen from above: the head on the pillow, a blanket, a plug
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), im = body(b.id, projectColor(b.cwd), "down", "stand", !b.background);
    const ring = hl.has(f.id) || over(x - 11, y - 28, x + 11, y) ? TERRACOTTA : app.open() === f.id ? OPEN : null;
    if (im) { if (ring) ctx.drawImage(halo(im, ring), 0, 0, SW, 16, x - 11, y - 28, SW, 16); ctx.drawImage(im, 0, 0, SW, 16, x - 11, y - 28, SW, 16); }
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
  function drawDesk(ctx, d, w, T, hl) {
    const { x0, y, w: dw, cab } = d;
    // the screen runs only while a bot sits at it typing; dim while one sits there idle; a
    // blinking cursor while one waits for you
    const seated = (f) => f && f.legs === "sit" && !f.lying && (f.act.kind === "desk" || f.act.kind === "sleep");
    const at = d.seats.map(([id]) => w.folk.get(id)).filter(seated);
    const busy = at.some((f) => f.busy), waits = at.some((f) => f.waiting), here = at.length > 0;
    box(ctx, x0 + cab - 1, y - 9, dw - cab + 1, 2, WOOD);
    rect(ctx, x0 + cab, y - 7, dw - cab, 1, WOOD_D);
    box(ctx, x0 + dw - 3, y - 6, 2, 6, WOOD_D);
    box(ctx, x0, y - 13, cab, 13, WOOD_D);
    rect(ctx, x0, y - 13, cab, 1, WOOD);
    const label = projectLabel(d.cwd);
    tiny(ctx, label, Math.round(x0 + (cab - tinyW(label)) / 2), y - 9, C.steel3);
    const sx = Math.round(x0 + cab / 2 - 10);
    rect(ctx, sx + 9, y - 16, 2, 3, C.steel1); rect(ctx, sx + 6, y - 14, 8, 1, C.steel1);
    // the pointer on the screen lights up the project's bots
    const onScreen = over(sx - 1, y - 32, sx + 21, y - 15);
    if (onScreen) { for (const [id] of d.seats) hl.add(id); tip = { text: d.cwd.replace(/^\/(Users|home)\/[^/]+/, "~"), x: sx + 10, y: y - 33 }; }
    box(ctx, sx, y - 31, 20, 15, C.steel0, onScreen ? C.steel3 : INK);
    rect(ctx, sx + 1, y - 30, 18, 13, here ? C.blue0 : "#161b28");
    if (!here) { rect(ctx, sx + 14, y - 29, 3, 1, C.steel0); rect(ctx, sx + 16, y - 28, 1, 2, C.steel0); }   // off: a glint
    if (here) {
      ctx.fillStyle = busy ? d.color : shade(d.color, 0.45);
      const s = busy ? Math.floor(T * 5) : 0;
      for (let r = 0; r < 6; r++) ctx.fillRect(sx + 2 + ((r * 3) % 4), y - 29 + r * 2, 3 + ((r * 7 + s + d.cwd.length) * 5) % 12, 1);
      if (waits && !busy && Math.floor(T * 2) % 2) rect(ctx, sx + 3, y - 19, 2, 1, C.steel4);
    }
    // a project with no bots can be forgotten: an x on its cabinet
    if (!d.seats.length && over(x0, y - 32, x0 + cab, y)) {
      const h = hot(x0 + cab - 6, y - 32, x0 + cab + 2, y - 25, { kind: "forget", dir: d.cwd });
      tiny(ctx, "X", x0 + cab - 4, y - 31, h ? TERRACOTTA : C.steel2);
    }
  }
  // pair's chair, turned round: its sitter faces left, to the screen; the empty one at the end starts a bot
  function drawChair(ctx, x, y, d, plus, lit) {
    const h = plus && hot(x - 8, y - 22, x + 9, y + 1, { kind: "spawn", dir: d.cwd });
    if (h) tip = { text: "new bot", x, y: y - 18 };
    const dim = plus && !h, col = dim ? INK2 : lit ? C.steel1 : C.steel0;
    box(ctx, x + 6, y - 15, 2, 9, col, lit ? TERRACOTTA : INK);
    box(ctx, x - 2, y - 7, 9, 2, col, lit ? TERRACOTTA : INK);
    rect(ctx, x + 2, y - 4, 1, 3, dim ? C.steel0 : C.steel1);
    rect(ctx, x - 1, y - 1, 7, 1, dim ? C.steel0 : C.steel1);
    if (plus) tiny(ctx, "+", x, y - 16, h ? d.color : C.steel1);
  }
  function drawBoard(ctx, b, w) {
    box(ctx, b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, C.steel4, C.steel1);
    rect(ctx, b.x0 + 2, b.y1, b.x1 - b.x0 - 4, 1, C.steel1);
    ctx.fillStyle = shade(C.blue2, 0.75);
    for (const s of w.board) {
      const x = b.x0 + 2 + s.x, y = b.y0 + 2 + s.y;
      if (s.t === "box") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x, y + s.h, s.w, 1); ctx.fillRect(x, y, 1, s.h); ctx.fillRect(x + s.w - 1, y, 1, s.h + 1); }
      else if (s.t === "line") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x + s.w - 2, y - 1, 1, 1); ctx.fillRect(x + s.w - 2, y + 1, 1, 1); }
      else ctx.fillRect(x, y, 1, 1);
    }
  }
  function drawDoor(ctx, x, ajar) {         // pair's door; ajar while someone goes in or out
    const y0 = 6, h = WALL - y0 - 2;
    if (ajar) { box(ctx, x, y0, 12, h, INK); rect(ctx, x, y0, 3, h, WOOD); return; }
    box(ctx, x, y0, 12, h, WOOD);
    rect(ctx, x + 2, y0 + 2, 8, 6, WOOD_D); rect(ctx, x + 2, y0 + 10, 8, 6, WOOD_D);
    rect(ctx, x + 9, y0 + 9, 1, 1, C.gold2);
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
  function drawRug(ctx, g) {                // pair's ping-pong rug
    box(ctx, g.x - 34, g.y - 18, 68, 24, "#2a1d3a", "#3a2a4f");
    ctx.fillStyle = "#3a2a4f";
    for (let x = g.x - 30; x < g.x + 32; x += 6) { ctx.fillRect(x, g.y - 15, 2, 1); ctx.fillRect(x + 3, g.y + 2, 2, 1); }
  }
  function drawMat(ctx, m) {
    box(ctx, m.x - 12, m.y - 24, 24, 23, "#26324f");
    ctx.fillStyle = "#2f3e62"; for (let y = m.y - 21; y < m.y - 2; y += 3) ctx.fillRect(m.x - 12, y, 24, 1);
    box(ctx, m.x - 8, m.y - 23, 16, 6, C.steel3);
  }
  function drawTable(ctx, m) {
    box(ctx, m.x - 16, m.y - 11, 32, 4, WOOD);
    rect(ctx, m.x - 16, m.y - 8, 32, 1, WOOD_D);
    box(ctx, m.x - 13, m.y - 6, 2, 6, WOOD_D); box(ctx, m.x + 11, m.y - 6, 2, 6, WOOD_D);
    rect(ctx, m.x - 6, m.y - 10, 3, 2, C.steel4); rect(ctx, m.x + 4, m.y - 10, 3, 2, C.steel4);
  }
  // a small table with the board on it, seen from above at a slant; the pieces stand on their squares
  function drawChess(ctx, c, board) {       // pair's: a small table with the board on it, the position as it stands
    const b = board || START;
    box(ctx, c.x - 2, c.y - 9, 4, 9, WOOD_D);
    box(ctx, c.x - 6, c.y - 1, 12, 1, WOOD_D);
    box(ctx, c.x - 9, c.y - 12, 18, 3, WOOD);
    // the board seen from the side: eight files, the back half and the front half
    for (let f = 0; f < 8; f++) for (let r = 0; r < 2; r++) rect(ctx, c.x - 8 + f * 2, c.y - 13 + r, 2, 1, (f + r) % 2 ? "#a08670" : "#d9c49a");
    for (let f = 0; f < 8; f++) {           // a piece on a file stands up from the board, white in front
      const col = [...Array(8)].map((_, r) => b[r * 8 + f]);
      const wh = col.some((p) => p !== "." && p === p.toUpperCase()), bl = col.some((p) => p !== "." && p === p.toLowerCase());
      if (bl) rect(ctx, c.x - 8 + f * 2, c.y - 15, 1, 2, C.ink2);
      if (wh) rect(ctx, c.x - 7 + f * 2, c.y - 14, 1, 2, C.steel4);
    }
  }
  function drawStool(ctx, st) {
    box(ctx, st.x - 4, st.y - 7, 8, 2, WOOD_D);
    rect(ctx, st.x - 3, st.y - 4, 1, 4, WOOD_D); rect(ctx, st.x + 2, st.y - 4, 1, 4, WOOD_D);
  }
  // the coffee machine; its pot is the five-hour session's limit: full when nothing is used yet
  function drawCoffee(ctx, at, brewing, T, u) {
    const x0 = at.x, y = at.y;
    box(ctx, x0, y - 10, 20, 10, C.steel1);
    rect(ctx, x0 + 2, y - 7, 7, 6, C.steel0); rect(ctx, x0 + 11, y - 7, 7, 6, C.steel0);
    box(ctx, x0 + 2, y - 19, 8, 8, INK2);
    rect(ctx, x0 + 8, y - 17, 1, 1, C.red1);
    rect(ctx, x0 + 4, y - 14, 4, 1, C.steel3);
    if (brewing) { rect(ctx, x0 + 5, y - 13 + (Math.floor(T * 8) % 2), 1, 1, WOOD); rect(ctx, x0 + 6, y - 22 - (Math.floor(T * 4) % 2), 1, 1, C.steel3); }
    // the pot: glass with a light rim, the coffee in it with a lighter top, one dot = an eighth
    box(ctx, x0 + 12, y - 20, 6, 9, GLASS, C.steel2);
    rect(ctx, x0 + 19, y - 18, 1, 4, C.steel2);
    const left = u ? Math.max(0, Math.min(9, Math.round(9 * (1 - u.used / 100)))) : 0;
    if (left) { rect(ctx, x0 + 12, y - 11 - left, 6, left, "#8a5030"); rect(ctx, x0 + 12, y - 11 - left, 6, 1, "#d08a4a"); }
  }
  // the water cooler; its bottle is the week's limit
  function drawCooler(ctx, c, u) {
    box(ctx, c.x - 3, c.y - 10, 6, 10, C.steel3);
    rect(ctx, c.x - 1, c.y - 7, 2, 1, C.steel1);
    box(ctx, c.x - 3, c.y - 21, 6, 10, GLASS, C.steel2);
    const left = u ? Math.max(0, Math.min(10, Math.round(10 * (1 - u.used / 100)))) : 0;
    if (left) { rect(ctx, c.x - 3, c.y - 11 - left, 6, left, C.blue2); rect(ctx, c.x - 3, c.y - 11 - left, 6, 1, C.blue3); }
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
    const pp = app.pong();
    if ([...w.folk.values()].filter((f) => f.act.kind === "pong" && f.act.there).length === 2 || (pp && pp.ball)) {
      const { t, from } = w.pong, a = from ? 1 : -1, x = g.x + a * 18 * (1 - 2 * t), k = t < 0.5 ? t * 2 : (t - 0.5) * 2;
      const y = g.y - 12 - Math.round(Math.sin(k * Math.PI) * (t < 0.5 ? 6 : 8)) - 6 * (1 - Math.min(1, t * 4));
      rect(ctx, Math.round(x) - 1, Math.round(y) - 1, 3, 3, INK); rect(ctx, Math.round(x), Math.round(y), 1, 1, C.white);
    }
  }

  // ---- the pointer, in dots: a click acts, a drag moves furniture (or carries a bot: to the
  // door, to stop it)
  const dots = (e) => { const r = cv.getBoundingClientRect(); return { x: (e.clientX - r.left) / K, y: (e.clientY - r.top) / K }; };
  function pieceAt(p) {
    if (!p || !lastL) return null;
    return lastL.pieces.find((q) => p.x >= q.r[0] && p.x < q.r[2] && p.y >= q.r[1] && p.y < q.r[3]) || null;
  }
  let press = null;
  cv.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    pointer = dots(e); draw();
    const h = [...hots].reverse().find((h) => over(h.x0, h.y0, h.x1, h.y1));
    press = { x: pointer.x, y: pointer.y, action: h && h.action, bot: hovered && !hovered.seat && hovered.b.id, piece: pieceAt(pointer) };
    cv.setPointerCapture(e.pointerId);
  });
  cv.addEventListener("pointermove", (e) => {
    pointer = dots(e);
    if (press && !moving && !held && Math.hypot(pointer.x - press.x, pointer.y - press.y) > 2) {
      const f = press.bot && world(app.cur()).folk.get(press.bot);
      if (f && f.b && f.b.background && !f.lying) { held = f; f.held = true; f.sat = false; f.lying = false; }
      else if (press.piece && !(press.bot && !f)) moving = { key: press.piece.key, from: press.piece.at, u: press.piece.u, at: press.piece.at };
    }
    if (moving) {
      const at = [moving.from[0] + Math.round(pointer.x - press.x), moving.from[1] + (ON_WALL.has(moving.key) ? 0 : Math.round(pointer.y - press.y))];
      moving.at = clampAt(moving.key, at, moving.u);
    }
    if (held) { held.x = pointer.x; held.y = pointer.y + 24; held.dir = -1; }
    draw();
  });
  cv.addEventListener("pointerup", () => {
    if (moving) {
      const room = app.state().rooms[app.cur()];
      room.layout = { ...(room.layout || {}), [moving.key]: moving.at };
      app.act({ kind: "layout", layout: room.layout });
      moving = null;
    } else if (held) {
      const f = held; held = null; f.held = false;
      // dropped at the door: it waits there to be stopped; anywhere else it goes back to what it did
      if (pointer && pointer.x < 26 && pointer.y < WALL + 14) f.act = { kind: "door", left: 6 };
      else f.act = f.work ? { kind: "desk" } : rest();
      f.legs = "stand";
    } else if (press && press.action) app.act(press.action);
    press = null;
    draw();
  });
  cv.addEventListener("pointerleave", () => { if (!press) { pointer = null; draw(); } });

  // a file dropped on a bot goes into its session
  cv.addEventListener("dragover", (e) => { e.preventDefault(); pointer = dots(e); draw(); });
  cv.addEventListener("drop", (e) => {
    e.preventDefault(); pointer = dots(e); draw();
    if (hovered && hovered.b.background && e.dataTransfer.files.length) app.act({ kind: "drop", id: hovered.b.id, files: [...e.dataTransfer.files] });
  });

  // ---- size and run, twelve frames a second like a 16-bit game. The room is always W x H dots;
  // only how big a dot is follows the window: whole pixels, as big as fits
  function size() {
    H = Math.max(200, Math.floor(cv.parentElement.clientHeight / K)); FLOORY = H - 6; defaults();
    if (cv.width !== W || cv.height !== H) { cv.width = W; cv.height = H; }
    cv.style.width = W * K + "px"; cv.style.height = H * K + "px";
    cv.parentElement.style.width = W * K + "px";
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
  window.__room = { worlds, size: () => ({ W, H, K }), layout: () => lastL };   // for poking at it from the console
  return { talk, size, draw, color: projectColor };
}
