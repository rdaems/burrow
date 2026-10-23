// The room: every machine's bots in one place, as pair's office (rdaems/pair-prompting, app/floor.js): its
// breaks, its mats, its robots (pixel.js), furnished as pixel art (sprites.js). Every project (a working
// directory, here or on another machine) is a desk: a cabinet with the project's screen and name at its left
// end, a long desktop to the right where its bots sit facing the screen, a chair a bot. The desk is
// handled at its screen: hover it for the path, the + beside it starts another bot there. Desks stay
// (every directory a session ran in, and the ones you add); carry one to a door to remove it. A bot in a
// git worktree of the project sits at the project's desk, its own copy of the files a laptop on the
// cabinet, plugged in.
// The bots are the sessions `claude agents` lists, and where a bot is says what agent view would:
//   working             at its project's desk, typing
//   needs input         at its desk with its hand up (a question, a permission prompt), its screen's
//                       cursor blinking; never falls asleep
//   idle (done)         by its desk, or on a break: coffee, the printer, the cooler, ping-pong,
//                       the plants, the window, the armchair, a meeting at the whiteboard
//   idle 10 minutes,    charging on a mat (opening it wakes it: Claude Code restarts a stopped
//   or process exited   process when you attach)
//   sends a message     a beam from its antenna to whoever it wrote to
//   stopped             walks out through its machine's door (and a new one comes in through it)
// Free, two idle bots play ping-pong or chess; one alone at the chess board waits for an opponent.
// You are in the room too, as pair draws its people: beside the bot whose session is open, a bubble
// over your head while you type to it; at the chess or ping-pong table while you play; on a mat
// while the window is left alone; otherwise about the room like the others.
// The account's limits are in the room too: the coffee pot holds what's left of the five-hour
// session, the cooler's bottle what's left of the week (Claude Code's own /usage).
// Nothing else signals anything; what they say is never shown, only that they talk.
// Each machine is a door in the wall, this one's first. Another machine's door and desks are painted
// its own colour (a palette swap, as 16-bit games told kinds of a thing apart), so no desk needs its
// machine in its name; a machine out of reach has its door boarded up.
//
// What you do happens in the room as well: click a bot to open its session (shift-click keeps the
// tile that was open), the + by a desk's screen for a new bot, the empty cabinet for a project, the
// empty door frame for a machine; drag furniture where you want it; carry a bot to its door to stop
// it, a desk to a door to remove it; drop a file on a bot to hand it over.
//
// startRoom(canvas, app) with app = { state(), open(), opens(), chess(), pong(), away(), hl(), hover(id), act(action) }
// (open: the bot whose tile has the focus, opens: every bot with a tile, hl: the one whose tile the pointer is on,
// carry(id, clientX|null, clientY) / carried(id, clientX, clientY): a bot held out over the pane, and let go there);
// returns { talk(from, to), typed(), size(), draw(), width(px?), color(unit) }. The room is W x H dots: W as the
// divider was dragged (room.json), H as tall as the window.

function startRoom(cv, app) {
  const C = pixelAvatar.C, INK = C.ink, INK2 = C.ink2;
  const TERRACOTTA = "#d97757", RED = "#ff6b5a", OPEN = "#8a4a3a", YOU = TERRACOTTA;   // your clothes: pair's "me"
  const SP = sprites.PAL, WOOD = { hi: SP.h, top: SP.w, d: SP.d, k: SP.k, plate: "#2e1f28" }, SHADOW = SP.s;
  // the paint of another machine's door and desks, in the order of Claude Code's sshConfigs
  const PAINT = [
    { hi: "#8fd0c4", top: "#5fa89c", d: "#3f7d74", k: "#1f3f42", plate: "#142a2c" },
    { hi: "#f4f1ea", top: "#d8d2c6", d: "#a59d90", k: "#4a4250", plate: "#2a2530" },
    { hi: "#a3c0e4", top: "#7397c4", d: "#4f6e9a", k: "#263652", plate: "#182236" },
    { hi: "#e8a493", top: "#c97766", d: "#964c40", k: "#4f2430", plate: "#2e151c" },
  ];
  const WALLC = "#28374b", BASEBOARD = "#1b2433";
  const PLANK = "#734728", SEAM = "#6a4224";
  const PROJECT = ["#4a76b8", "#e8402a", "#2fa087", "#f2a531", "#9a74d6", "#5fcf4a", "#f06b72", "#8fc0e8"];
  const WALL = 26, SEAT = 18, ADD_W = 30, ROW = 46, LAP = 13, DOOR = 20, NAME = 8;
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const shade = (hex, k) => "#" + [1, 3, 5].map((i) => Math.min(255, Math.round(parseInt(hex.slice(i, i + 2), 16) * k)).toString(16).padStart(2, "0")).join("");
  const mix = (a, b, k) => "#" + [1, 3, 5].map((i) => { const x = parseInt(a.slice(i, i + 2), 16), y = parseInt(b.slice(i, i + 2), 16); return Math.round(x + (y - x) * k).toString(16).padStart(2, "0"); }).join("");
  const rnd = (a, b) => a + Math.random() * (b - a);
  const any = (a) => a[Math.floor(Math.random() * a.length)];
  const fnv = (s) => { let h = 2166136261; for (const ch of s) h = Math.imul(h ^ ch.codePointAt(0), 16777619); return h >>> 0; };
  const projectColor = (unit) => PROJECT[fnv(unit) % PROJECT.length];
  function projectLabel(cwd) {
    const n = cwd.replace(/\/+$/, "").split("/");
    if (cwd === "~" || cwd === "/root" || (n.length === 3 && (n[1] === "Users" || n[1] === "home"))) return "home";
    return n[n.length - 1] || cwd;
  }
  const machineName = (id) => { const m = id && app.state().machines.find((x) => x.id === id); return m ? m.name : ""; };
  // a desk's name: its project's, cut to eight letters like a name in a 16-bit game (its machine is
  // its paint, the whole path is on its screen under the pointer)
  const deskLabel = (d) => [...projectLabel(d.dir)].slice(0, NAME).join("").replace(/(.)[-_. ]+$/, "$1");
  // a machine's door: the first is this one's; its bots come in and go out at its middle
  const doorOf = (mid) => Math.max(0, app.state().machines.findIndex((m) => m.id === (mid || "")));
  const doorX = (mid) => 10 + doorOf(mid) * DOOR;
  const paintOf = (mid) => { const i = doorOf(mid); return i ? PAINT[(i - 1) % PAINT.length] : WOOD; };
  const wallFrom = () => 24 + app.state().machines.length * DOOR;    // what hangs on the wall: past the doors

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
    "# #.#/###/#.#/###/#.# \" #.#/#.#/.../.../... & .#./#.#/.#./#.#/.## _ .../.../.../.../### ~ .../.##/##./.../...").match(/\S+ \S+/g)) {
    const [k, rows] = g.split(" ");
    F[k] = rows.split("/").map((r) => r.padEnd(3, ".").slice(0, 3)).join("");
  }
  // pair's lettering, a letter pixel a dot (he tried it smaller and in the terminal's font: worse)
  const tinyW = (s) => Math.max(0, [...s].length * 4 - 1);
  function tiny(ctx, s, x, y, col) {
    ctx.fillStyle = col;
    [...s.toUpperCase()].forEach((ch, i) => {
      const gl = F[ch];
      if (gl) for (let k = 0; k < 15; k++) if (gl[k] === "#") ctx.fillRect(x + i * 4 + (k % 3), y + Math.floor(k / 3), 1, 1);
    });
  }

  // ---- figures: pixel.js's head on floor.js's body, drawn once per pose with an ink outline; the
  // bots are robots, you are a person (pair's human figure)
  const SW = 22, SH = 29;
  const faces = new Map(), bodies = new Map();
  function face(seed, color, human) {
    const key = seed + color + (human ? "h" : "");
    if (!faces.has(key)) {
      const im = new Image(), f = { px: null };
      im.onload = () => {
        const c = document.createElement("canvas"); c.width = c.height = 20;
        const x = c.getContext("2d"); x.drawImage(im, 0, 0);
        f.px = x.getImageData(0, 0, 20, 20).data;
      };
      im.src = pixelAvatar(seed, human ? "human" : "agent", color);
      faces.set(key, f);
    }
    return faces.get(key);
  }
  // arms: down sw1 sw2 up wave fwd1 fwd2 reach mug paper book can; legs: stand walk1 walk2 sit
  function body(seed, color, arms, legs, human) {
    const f = face(seed, color, human);
    if (!f.px) return null;
    const key = seed + color + arms + legs + (human ? "h" : "");
    if (bodies.has(key)) return bodies.get(key);
    const px = new Map(), line = new Set();
    const set = (x, y, c) => { if (c) px.set((y + 1) * SW + x + 1, c); };
    const rect = (x, y, w, h, c) => { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) set(x + i, y + j, c); };
    const cut = human ? 14 : 16;             // the portrait down to the chin (a robot's down to its neck)
    for (let y = 0; y <= cut; y++) for (let x = 0; x < 20; x++) {
      const i = (y * 20 + x) * 4, d = f.px;
      if (d[i + 3] < 128) continue;
      const c = "#" + [d[i], d[i + 1], d[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join("");
      set(x, y, c);
      if (c === INK) line.add((y + 1) * SW + x + 1);
    }
    const at = (x, y) => { const i = (y * 20 + x) * 4, d = f.px; return d[i + 3] < 128 ? null : "#" + [d[i], d[i + 1], d[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join(""); };
    // a person wears what their portrait wears (a coat is white, a suit grey): its shoulders' colours
    const c0 = human ? at(6, 19) || color : C.steel3, c1 = human ? (at(15, 19) !== c0 && at(15, 19)) || shade(c0, 0.62) : C.steel1;
    const skin = human ? pixelAvatar.skin(seed) : C.steel3, leg = human ? INK2 : C.steel1, top = cut + 1;
    if (human) rect(9, top, 2, 1, shade(skin, 0.8));
    const t0 = top + (human ? 1 : 0);
    rect(6, t0, 8, 22 - t0, c0); rect(12, t0, 2, 22 - t0, c1);
    if (!human) { set(9, top + 2, color); set(10, top + 2, color); set(9, top + 3, shade(color, 0.62)); }
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
    for (const [k, col] of px) { x.fillStyle = col; x.fillRect(k % SW, Math.floor(k / SW), 1, 1); }
    bodies.set(key, c);
    return c;
  }
  // a figure's colour: a bot wears its project's, you wear yours
  const colorOf = (b) => b.human ? (b.color || YOU) : projectColor(b.unit);
  const seedOf = (b) => b.human ? (b.seed || b.name) : b.id;

  // halo for a highlighted bot (or chair, or screen): one dot around its outline (not its shadow)
  const halos = new Map();
  function halo(im, col) {
    if (!halos.has(col)) halos.set(col, new WeakMap());
    const hm = halos.get(col);
    if (hm.has(im)) return hm.get(im);
    const w = im.width, h = im.height, d = im.getContext("2d").getImageData(0, 0, w, h).data, c = document.createElement("canvas");
    c.width = w; c.height = h;
    const x = c.getContext("2d"), on = (i, j) => i >= 0 && j >= 0 && i < w && j < h && d[(j * w + i) * 4 + 3] > 128;
    x.fillStyle = col;
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++)
      if (!on(i, j) && (on(i - 1, j) || on(i + 1, j) || on(i, j - 1) || on(i, j + 1))) x.fillRect(i, j, 1, 1);
    hm.set(im, c);
    return c;
  }
  // a sprite (sprites.js) with its top left at x, y; flipped, it faces the other way in the same place
  function put(ctx, name, x, y, flip) {
    const im = typeof name === "string" ? sprites.get(name) : name;
    if (!im) return null;
    if (flip) { ctx.save(); ctx.translate(x + im.width, y); ctx.scale(-1, 1); ctx.drawImage(im, 0, 0); ctx.restore(); }
    else ctx.drawImage(im, x, y);
    return im;
  }
  // a block seen from above at a slant, as Pixel Agents draws its furniture: a top `top` dots deep
  // and a front `front` dots high, outlined, its back and front edges lit, a shadow under it
  function slab(ctx, x, y, w, top, front, p) {
    rect(ctx, x, y, w, top + front, p.k);
    rect(ctx, x + 1, y, w - 2, 1, p.d);
    rect(ctx, x + 1, y + 1, w - 2, top - 1, p.top);
    rect(ctx, x + 1, y + top - 1, w - 2, 1, p.hi);
    rect(ctx, x + 1, y + top + 1, w - 2, front - 2, p.d);
    rect(ctx, x + 1, y + top + front, w - 2, 1, SHADOW);
  }

  // ---- the room: always the same size; its furniture stands where it was put (dragged by hand,
  // kept in room.json), or where it stands by default. A project's desk that has no place yet gets
  // the first free one, and keeps it.
  // as dense as pair's office: two CSS pixels a dot, a room as wide as pair's margin; as tall as the
  // window lets it be (what stands at the bottom stays at the bottom)
  let W = 256;
  const K = 2, MARGIN = 6;                                 // CSS pixels a dot; the margin around the room, the tiles' (tiles.js OUT)
  let H = 400, FLOORY = H - 6, DEF = {};
  function defaults() {
    DEF = {
      meet: [W / 2, WALL + 34], pong: [60, 62], chess: [70, FLOORY - 32], mat0: [104, FLOORY - 30], mat1: [132, FLOORY - 30], mat2: [160, FLOORY - 30],
      coffee: [3, FLOORY], cooler: [36, FLOORY - 1], printer: [W / 2 + 4, FLOORY], armchair: [W - 32, FLOORY],
      plant0: [W - 8, FLOORY], plant1: [W - 12, 64], board: [W / 2 + 46, WALL + 34], window: [W - 40, 4], clock: [W - 15, 5], fire: [W - 74, FLOORY],
      closet: [W - 72, 2],
    };
  }
  // The furniture is what the server lists (room.json `furniture`): keys, a kind and maybe a number
  // (plant0, mat2). The closet adds a piece, the fireplace burns one; the fire and the closet stay
  const KIND = (key) => key.replace(/\d+$/, "");
  const FIXED = new Set(["fire", "closet"]);
  const NAMES = { meet: "meeting table", pong: "ping-pong table", chess: "chess table", mat: "charging mat", coffee: "coffee machine", cooler: "water cooler",
    printer: "printer", armchair: "armchair", plant: "plant", board: "whiteboard", window: "window", clock: "clock", add: "project cabinet", pair: "pair room" };
  // the default places for desks: columns of them, as many as the room is wide
  const SLOTS = () => [104, 150, 196, 242, 288, 334].flatMap((y) => [...Array(Math.max(1, Math.floor((W - 10) / 124)))].map((_, i) => [10 + i * 124, y]));
  const ON_WALL = new Set(["closet", "window", "clock"]), WALL_Y = { closet: 2, window: 4, clock: 5 };
  const onWall = (key) => ON_WALL.has(KIND(key));
  // what each piece covers, from where it stands
  function foot(key, [x, y], u) {
    if (u) return [x - 2, y - 35, x + u.w + 2, y + 2];
    switch (KIND(key)) {
      case "add": return [x - 2, y - 16, x + ADD_W + 2, y + 2];
      case "meet": return [x - 28, y - 20, x + 28, y + 4];
      case "pong": return [x - 38, y - 23, x + 38, y + 8];
      case "chess": return [x - 19, y - 18, x + 19, y + 2];
      case "mat": return [x - 12, y - 29, x + 14, y + 1];
      case "coffee": return [x, y - 22, x + 20, y + 1];
      case "cooler": return [x - 5, y - 26, x + 5, y + 1];
      case "printer": return [x - 7, y - 15, x + 7, y + 1];
      case "armchair": return [x - 6, y - 23, x + 18, y + 1];
      case "board": return [x - 16, y - 26, x + 16, y + 1];
      case "window": return [x, y, x + 21, y + 18];
      case "clock": return [x, y, x + 12, y + 10];
      case "fire": return [x, y - 28, x + 32, y + 1];
      case "closet": return [x, y, x + 22, y + 24];
      case "pair": return [x - 13, y - 20, x + 13, y + 1];
      default: return [x - 8, y - 31, x + 8, y + 1];     // a plant
    }
  }
  const meets = (a, b) => a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
  // keep a piece in the room: on the floor, or along the wall for what hangs there
  // (on a grid of `step` dots)
  function clampAt(key, at, u, step = 1) {
    const q = (v) => Math.round(v / step) * step;
    const r = foot(key, at, u), [x, y] = at;
    if (onWall(key)) return [q(Math.max(wallFrom() - (r[0] - x), Math.min(W - 2 - (r[2] - x), x))), WALL_Y[KIND(key)]];
    return [q(Math.max(2 - (r[0] - x), Math.min(W - 2 - (r[2] - x), x))),
            q(Math.max(WALL + 2 - (r[1] - y), Math.min(H - 1 - (r[3] - y), y)))];
  }

  const dropped = new Map();
  // a desk's bots: those in its own directory first, then those in each worktree
  const sitters = (bots, d) => bots.filter((b) => b.unit === d.unit).sort((a, b) => (a.tree > b.tree) - (a.tree < b.tree));
  function layout(s, bots, moving) {
    const saved = s.layout || {}, placed = {};
    const units = s.desks.map((d) => {
      const n = sitters(bots, d).length, label = deskLabel(d);
      const trees = [...new Set(sitters(bots, d).map((b) => b.tree).filter(Boolean))];
      const cab = Math.max(tinyW(label) + 6, 22, trees.length ? 26 + trees.length * LAP : 0);
      return { key: "u:" + d.unit, d, label, trees, cab, w: cab + 4 + Math.max(1, n) * SEAT };   // a chair a bot; one when there is none
    });
    const where = (key, u) => {
      if (moving && moving.key === key) return moving.at;
      const just = dropped.get(key);          // just put down: stays there while the server hears of it
      if (just && performance.now() - just.t < 5000) return just.at;
      if (saved[key]) return clampAt(key, saved[key], u);       // the window may be shorter than when it was put there
      return DEF[key] && !u ? DEF[key] : null;
    };
    const keys = s.furniture || Object.keys(DEF), at0 = {};
    const taken = [];
    for (const k of keys) { const at = where(k); at0[k] = at; if (at && !onWall(k)) taken.push(foot(k, at)); }
    for (const u of units) { u.at = where(u.key, u); if (u.at) taken.push(foot(u.key, u.at, u)); }
    // a free place, for a unit (with room to grow by a chair) or a piece without one
    const free = (key, u) => {
      const wide = u && { ...u, w: u.w + SEAT };
      const cands = SLOTS().concat([...Array(40)].flatMap((_, j) => [...Array(30)].map((_, i) => [6 + i * 8, 70 + j * 6])));
      for (const c of cands) {
        const r = foot(key, c, wide);
        if (r[0] >= 2 && r[2] <= W - 2 && r[1] >= WALL + 2 && r[3] <= H && !taken.some((t) => meets(t, r))) { taken.push(r); return c; }
      }
      return SLOTS()[0];
    };
    for (const u of units) if (!u.at) { u.at = free(u.key, u); placed[u.key] = u.at; }
    for (const k of keys) if (!at0[k]) { at0[k] = onWall(k) ? [wallFrom(), KIND(k) === "closet" ? 2 : 5] : free(k); placed[k] = at0[k]; }   // a piece without a place yet

    const P = (k) => at0[k];
    // the pieces by kind, in the order of their keys; most kinds are one at most, some can be many
    const by = {};
    for (const k of keys) { const [x, y] = P(k); (by[KIND(k)] = by[KIND(k)] || []).push({ key: k, x, y }); }
    const one = (kind) => (by[kind] || [])[0] || null, all = (kind) => by[kind] || [];
    const desks = units.map((u) => {
      const [x, y] = u.at, mine = sitters(bots, u.d);
      const seat = (k) => x + u.cab + 4 + SEAT / 2 + k * SEAT;
      return { key: u.key, ...u.d, label: u.label, trees: u.trees, x0: x, w: u.w, cab: u.cab, y, color: projectColor(u.d.unit),
        seats: mine.map((b, k) => [b.id, seat(k), b.tree]), empty: mine.length ? null : seat(0), at: u.at, u };
    });
    const b0 = one("board"), c0 = one("clock"), f0 = one("fire"), cl = one("closet"), a0 = one("add");
    const L = {
      desks, add: a0 && { key: a0.key, x0: a0.x, y0: a0.y - 15, x1: a0.x + ADD_W, y1: a0.y },
      floorY: FLOORY, board: b0 && { key: b0.key, x: b0.x, y: b0.y, x0: b0.x - 16, y0: b0.y - 26, x1: b0.x + 16, y1: b0.y }, clock: c0 && { key: c0.key, x0: c0.x, y0: c0.y },
      windows: all("window").map((p) => ({ key: p.key, x0: p.x, y0: p.y, x1: p.x + 21, y1: p.y + 18 })),
      fire: f0 && { key: f0.key, x0: f0.x, y0: f0.y - 28, x1: f0.x + 32, y1: f0.y }, closet: cl && { key: cl.key, x0: cl.x, y0: cl.y, x1: cl.x + 22, y1: cl.y + 24 },
      meet: one("meet"), pong: one("pong"), chess: one("chess"), mats: all("mat"), pairs: all("pair"),
      cooler: one("cooler"), plants: all("plant"), printer: one("printer"), chairs: all("armchair"),
      coffeeAt: one("coffee"), floor: { x0: 14, y0: WALL + 8, x1: W - 14, y1: FLOORY - 4 }, placed,
    };
    L.coffee = L.coffeeAt && { x: L.coffeeAt.x + 27, y: L.coffeeAt.y - 10 };
    // what can be picked up and moved, front to back
    L.pieces = [
      ...desks.map((d) => ({ key: d.key, at: d.at, u: d.u, r: foot(d.key, d.at, d.u) })),
      ...keys.map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
    ].sort((a, b) => b.r[3] - a.r[3]);
    L.pieceOf = (key) => L.pieces.find((p) => p.key === key) || null;
    L.coolerSpot = (side) => L.cooler && { x: L.cooler.x + (side ? 10 : -8), y: L.cooler.y + 1 };
    L.pongSpot = (side) => L.pong && { x: L.pong.x + (side ? 22 : -22), y: L.pong.y + 1 };
    L.chessSeat = (side) => L.chess && { x: L.chess.x + (side ? 13 : -13), y: L.chess.y };
    L.meetSeat = (i) => { const s = L.meet && [[-23, 0, 1], [23, 0, -1], [-8, -5, 1], [8, -5, -1]][i]; return s && { x: L.meet.x + s[0], y: L.meet.y + s[1], dir: s[2] }; };
    L.seat = (id) => { for (const d of desks) for (const [i, x] of d.seats) if (i === id) return { x, y: d.y }; return null; };
    L.free = (x, y) => !L.pieces.some((p) => !onWall(p.key) && x >= p.r[0] - 6 && x <= p.r[2] + 6 && y >= p.r[1] && y <= p.r[3] + 4);
    return L;
  }

  // ---- everyone: what each bot is doing
  let everyone = null;
  function world() {
    if (!everyone) everyone = { folk: new Map(), meeting: null, lastMeeting: -60, board: [], pong: { t: 0, from: 0 }, doorAt: {}, stopped: new Map(), flare: 0, sparks: 0, print: 0,
      chess: { board: newBoard(), t: 0, turn: 0 }, beams: [], T: 0, born: performance.now() };
    return everyone;
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

  // `from` sent `to` a message: a beam from its antenna to the other's, wherever they are
  const BEAM = 0.7;                          // seconds for it to cross the room
  function talk(from, to) {
    const w = world(), f = w.folk.get(from), g = w.folk.get(to);
    if (!f || !g || f === g) return;
    w.beams.push({ from, to, t: 0 });
    f.chat = Math.max(f.chat, 1);
  }
  // you typed to the open session: a bubble over your head
  function typed() {
    const f = world().folk.get("you");
    if (f) f.chat = 2;
  }

  function think(w, bots, L, dt, sleepAfter, usage, game) {
    const now = performance.now() / 1000, folk = w.folk;
    // the bot you play chess with sits at the table until the game is over (or it gets work)
    const vs = game && !game.result ? (game.white.id === "you" ? game.black.id : game.white.id) : null;
    for (const f of folk.values()) {
      if (f.id === vs && !(f.act.kind === "chess" && f.act.you) && !f.work) { end(f); f.act = { kind: "chess", side: game.white.id === vs ? 0 : 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "chess" && !f.human && (f.act.you ? f.id !== vs : !!vs)) end(f);   // the table is taken: the others get up
      if (f.id === vs && game.sans.length !== w.plies) { if (w.plies !== undefined && (game.sans.length % 2 === 1) === (game.white.id === vs)) f.moved = 0.9; }
    }
    w.plies = game ? game.sans.length : undefined;
    // the bot you play ping-pong with stands at the table, at the far end
    const pp = app.pong(), pv = pp && pp.id;
    for (const f of folk.values()) {
      if (f.id === pv && !(f.act.kind === "pong" && f.act.you) && !f.work) { end(f); f.act = { kind: "pong", side: 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "pong" && !f.human && (f.act.you ? f.id !== pv : !!pv)) end(f);
    }
    // messages under way: the beam reaches the other, who takes it in
    for (const bm of w.beams) { const was = bm.t; bm.t += dt; if (was < BEAM && bm.t >= BEAM) { const g = folk.get(bm.to); if (g) g.chat = Math.max(g.chat, 1); } }
    w.beams = w.beams.filter((bm) => bm.t < BEAM + 0.4 && folk.has(bm.from) && folk.has(bm.to));
    const you = folk.get("you");
    if (you) you.chat = Math.max(0, you.chat - dt);
    const opens = app.opens();
    for (const b of bots) {
      const f = folk.get(b.id), work = b.waiting || (b.busy && b.alive !== false);
      if (!f) continue;                       // stopped from here, still in the feed
      f.chat = Math.max(0, f.chat - dt);
      f.busy = b.busy; f.waiting = b.waiting;
      // a bot whose session is open sits at its desk for you, whatever it was doing; when the
      // session closes it gets up again
      f.open = opens.includes(b.id);
      if (f.open && !work && !["desk", "doom"].includes(f.act.kind)) { end(f); f.act = { kind: "desk" }; }
      if (!f.open && !work && f.act.kind === "desk" && f.sat) f.act = { kind: "stand", left: 0.5 };
      if (work !== f.work) {
        f.work = work; f.idleSince = now;
        // to work, whatever it was doing (a bot by the door stays)
        if (work && f.act.kind !== "doom") { end(f); f.act = { kind: "desk" }; }
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
    for (const b of bots) { const f = folk.get(b.id); if (f && !f.held) thinkOne(w, f, L, now, dt, homes.get(b.id), sleepAfter, usage); }
    if (you) thinkOne(w, you, L, now, dt, null, sleepAfter, usage);
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
    // waiting for an answer never falls asleep: it sits at its desk with its hand up. A bot whose
    // process Claude Code stopped charges too (it comes back when opened). You, when the window
    // has been left alone a minute, as pair's people who are away
    const sleepy = f.human ? app.away() > 60 : !f.work && !f.open && (now - f.idleSince > sleepAfter || (f.b && f.b.alive === false));
    if (sleepy && f.act.kind !== "sleep" && f.act.kind !== "doom") { end(f); f.act = { kind: "sleep", there: false }; }
    if (!sleepy && f.act.kind === "sleep") end(f);
    // you: with the bot whose session is open, at the table of the game you play; else about the
    // room like the others
    if (f.human && !sleepy) {
      const o = app.open(), g = o && folk.get(o), game = app.chess(), pp = app.pong(), a = f.act;
      if (game) { if (!(a.kind === "chess" && a.you)) { end(f); f.act = { kind: "chess", side: game.white.id === "you" ? 0 : 1, you: true, left: 1e9, there: false }; } }
      else if (pp) { if (!(a.kind === "pong" && a.you)) { end(f); f.act = { kind: "pong", side: 0, you: true, left: 1e9, there: false }; } }
      else if (g) { if (!(a.kind === "with" && a.to === g.id)) { end(f); f.act = { kind: "with", to: g.id }; } }
      else if (a.kind === "with" || a.you) end(f);
    }
    const act = f.act, partner = () => { const g = act.with && folk.get(act.with); return g && g.act.kind === act.kind && g.act.with === f.id ? g : null; };
    if (!["desk", "meeting", "chess", "chair", "sleep"].includes(act.kind)) f.sat = false;
    switch (act.kind) {
      case "with": {
        // beside the bot you talk to, at its desk (it sits down for you): right of its chair,
        // unless another sits there, then in front of it (the bubble is yours while you type)
        const g = folk.get(act.to), gs = L.seat(act.to);
        if (!g) { f.act = rest(); break; }
        const seated = g.legs === "sit" && !g.lying, at = gs || g;
        const taken = seated && others.some((h) => h !== g && h.legs === "sit" && Math.abs(h.x - (at.x + SEAT)) < 4 && Math.abs(h.y - at.y) < 4);
        const spot = g.lying ? { x: g.x + 16, y: g.y + 1 } : taken ? { x: at.x + 2, y: at.y + 13 } : { x: at.x + 20, y: at.y + 1 };
        act.there = walk(f, spot, 36, dt);
        if (act.there) { f.dir = spot.x > g.x ? -1 : spot.x < g.x ? 1 : -1; f.arms = "down"; }
        break;
      }
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
        if (!L.coffee) { f.act = rest(); break; }
        if (act.stage === 0) { if (walk(f, L.coffee, v, dt)) { f.dir = -1; f.arms = "reach"; act.stage = 1; } }
        else {
          act.wait += dt; const dry = usage.session && usage.session.used >= 100;
          f.arms = act.wait > 2 && !dry ? "mug" : "reach";
          if (act.wait > 2.8) { f.carry = dry ? null : "mug"; f.act = rest(); }
        }
        break;
      case "print":
        if (!L.printer) { f.act = rest(); break; }
        if (act.stage === 0) { if (walk(f, { x: L.printer.x - 9, y: L.printer.y }, v, dt)) { f.dir = 1; f.arms = "down"; act.stage = 1; } }
        else if ((act.wait += dt) > 2.4) { f.carry = "paper"; f.act = rest(); }
        break;
      case "cooler": {
        const s = L.coolerSpot(act.side), g = folk.get(act.with);
        if (!g || !s) { f.act = rest(); break; }
        if (walk(f, s, v, dt)) {
          f.dir = act.side ? -1 : 1; f.arms = "down";
          if (Math.abs(g.x - f.x) < 24 && Math.abs(g.y - f.y) < 4 && Math.random() < dt * 0.4) f.chat = 1.2;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "pong": {
        if (!L.pong || (!act.you && !partner())) { f.act = rest(); break; }
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
        if (!L.chess) { f.act = rest(); break; }
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
        if (walk(f, spot, v + 6, dt)) {
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
        const pl = L.plants.find((p) => p.key === act.at) || L.plants[0];
        if (!pl) { f.act = rest(); break; }
        const side = pl.x > (L.floor.x0 + L.floor.x1) / 2 ? -1 : 1;
        f.carry = "can";
        if (walk(f, { x: pl.x + side * 14, y: pl.y + 1 }, v, dt)) {
          f.dir = -side; f.arms = "can"; act.water = true;
          if ((act.wait += dt) > 3) { f.carry = null; f.act = rest(); }
        }
        break;
      }
      case "window": {
        const win = L.windows.find((p) => p.key === act.at) || L.windows[0];
        if (!win) { f.act = rest(); break; }
        if (walk(f, { x: Math.round((win.x0 + win.x1) / 2), y: WALL + 8 }, v, dt)) {
          f.arms = "down"; f.dir = Math.sin(w.T * 0.7 + f.z) > 0 ? 1 : -1;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "chair": {
        const ch = L.chairs.find((p) => p.key === act.at) || L.chairs[0];
        if (!ch) { f.act = rest(); break; }
        if (sitAt(f, { x: ch.x - 2, y: ch.y - 1 }, v, dt)) { f.dir = -1; f.legs = "sit"; f.arms = "book"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      }
      case "present":
        if (!w.meeting || w.meeting.host !== f.id || !L.board) { f.act = rest(); break; }
        if (walk(f, { x: L.board.x0 - 6, y: L.board.y + 1 }, v, dt)) {
          f.dir = 1; f.arms = Math.floor(w.T * 2) % 3 ? "reach" : "down";
          if ((act.wait += dt) > 0.6 && w.board.length < 40) { act.wait = 0; stroke(w); }
          if (Math.random() < dt * 0.3) f.chat = 1;
        }
        break;
      case "meeting": {
        const s = L.meetSeat(act.seat);
        if (!s || (!w.meeting && !act.free)) { f.act = rest(); break; }
        if (sitAt(f, s, v, dt)) { f.dir = s.dir; f.legs = "sit"; f.arms = "down"; if (Math.random() < dt * 0.08) f.chat = 1; if (act.free && (act.left = (act.left ?? rnd(30, 60)) - dt) <= 0) f.act = rest(); }
        break;
      }
      case "sleep": {
        // on a free mat if there is one, else asleep at the desk (a mat burnt from under it: the desk)
        const mine = act.mat ? L.mats.find((m) => m.key === act.mat) : null;
        if (act.mat && !mine) { act.mat = null; act.there = false; }
        const mat = mine || L.mats.find((m) => !others.some((g) => g.act.kind === "sleep" && g.act.mat === m.key));
        const to = act.there ? mine || seat : mat || seat;
        if (!to) { f.act = rest(); break; }
        if (act.there || walk(f, to, v, dt)) {
          if (!act.there) act.mat = mat ? mat.key : null;
          act.there = true; f.x = to.x; f.y = to.y; f.lying = !!act.mat;
          f.dir = f.lying ? 1 : -1; f.legs = "sit"; f.arms = "down"; f.z += dt;
        }
        break;
      }
      case "doom": {
        // put at its door or into the fire: it is about to be stopped; a sign over it counts down,
        // a click on it is the undo. Then `claude stop`: out through the door, or up in flames
        const spot = act.where === "fire" && L.fire ? { x: L.fire.x0 + 16, y: L.fire.y1 - 10 } : { x: doorX(f.b && f.b.m), y: WALL + 3 };
        f.x = spot.x; f.y = spot.y; f.dir = -1; f.arms = act.where === "fire" ? "up" : "down"; f.legs = "stand";
        if ((act.left -= dt) <= 0 && !f.gone) {
          f.gone = true; w.stopped.set(f.id, performance.now());
          app.act({ kind: "stop", id: f.id });
          if (act.where === "fire") { w.flare = performance.now(); w.folk.delete(f.id); }
          else { f.leaving = true; f.gone = false; }
        }
        break;
      }
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
    if (L.meet && L.board && mates.length >= 2 && !w.meeting && w.T - w.lastMeeting > 60 && r < 0.08) {
      w.meeting = { host: f.id, left: rnd(22, 38) }; w.lastMeeting = w.T; w.board = [];
      mates.slice(0, 4).forEach((g, i) => { g.carry = null; g.act = { kind: "meeting", seat: i }; });
      return { kind: "present", wait: 0 };
    }
    if (r < 0.2 && L.coffee && !taken("coffee")) return { kind: "coffee", stage: 0, wait: 0 };
    if (r < 0.28 && L.printer && !taken("print")) return { kind: "print", stage: 0, wait: 0 };
    if (r < 0.38 && mate && L.cooler) {
      const left = rnd(10, 22);
      mate.carry = null; mate.act = { kind: "cooler", side: 1, left, with: f.id };
      return { kind: "cooler", side: 0, left, with: mate.id };
    }
    if (r < 0.5 && mate && L.pong && !taken("pong") && !app.pong()) {
      const left = rnd(15, 35);
      mate.carry = null; mate.act = { kind: "pong", side: 1, left, with: f.id, there: false };
      return { kind: "pong", side: 0, left, with: mate.id, there: false };
    }
    if (r < 0.6 && L.chess && !taken("chess") && !(app.state().chess && !app.state().chess.result)) return { kind: "chess", side: Math.random() < 0.5 ? 0 : 1, with: null, left: rnd(20, 45), there: false };
    if (r < 0.65 && mate) return { kind: "visit", to: mate.id, left: rnd(6, 12), wave: 0.8 };
    // one of a kind there can be many of, not in use: a plant to water, a window, an armchair
    const spare = (kind, ps) => ps.filter((p) => !others.some((g) => g.act.kind === kind && g.act.at === p.key));
    const plants = spare("plant", L.plants), wins = spare("window", L.windows), chairs = spare("chair", L.chairs);
    if (r < 0.72 && plants.length) return { kind: "plant", at: any(plants).key, wait: 0 };
    if (r < 0.79 && wins.length) return { kind: "window", at: any(wins).key, left: rnd(4, 9) };
    if (r < 0.87 && chairs.length) return { kind: "chair", at: any(chairs).key, left: rnd(15, 35) };
    for (let k = 0; k < 8; k++) {             // somewhere free to stand about
      const x = rnd(L.floor.x0, L.floor.x1), y = rnd(L.floor.y0, L.floor.y1);
      if (L.free(x, y)) return { kind: "loiter", x, y, left: rnd(3, 8) };
    }
    return rest();
  }
  function stroke(w) {                       // the host draws on the whiteboard: boxes, arrows, dots
    const k = Math.random(), sw = k < 0.35 ? Math.floor(rnd(3, 7)) : k < 0.7 ? Math.floor(rnd(3, 10)) : 1;
    const sh = k < 0.35 ? Math.floor(rnd(2, 4)) : 1;
    const x = Math.floor(rnd(1, INK_AREA.w - sw)), y = Math.floor(rnd(1, INK_AREA.h - sh - 1));
    w.board.push(k < 0.35 ? { t: "box", x, y, w: sw, h: sh } : k < 0.7 ? { t: "line", x, y, w: sw } : { t: "dot", x, y });
  }

  // ---- drawing, and what is under the pointer: hot spots are collected while drawing
  let pointer = null, hots = [], hovered = null, tip = null, lastL = null, moving = null, held = null, hoverId = null;
  const over = (x0, y0, x1, y1) => !!pointer && pointer.x >= x0 && pointer.x < x1 && pointer.y >= y0 && pointer.y < y1;
  const hot = (x0, y0, x1, y1, action) => { hots.push({ x0, y0, x1, y1, action }); return over(x0, y0, x1, y1) && !moving && !held; };
  const box = (ctx, x, y, w, h, fill, edge = INK) => { ctx.fillStyle = edge; ctx.fillRect(x - 1, y - 1, w + 2, h + 2); ctx.fillStyle = fill; ctx.fillRect(x, y, w, h); };
  const rect = (ctx, x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); };
  const clip = (t, room) => { const n = Math.max(0, Math.floor((room + 1) / 4)), m = t.replace(/\s+/g, " "); return m.length > n ? m.slice(0, n - 1) + "-" : m; };
  const say = (u, what) => (u ? `${Math.max(0, 100 - u.used)}% ` : "") + what;     // what is left of a limit: "63% 5h"

  function draw() {
    const ctx = cv.getContext("2d");
    ctx.setTransform(2, 0, 0, 2, 0, 0);      // drawn in dots; the canvas has two pixels a dot, for dragging
    ctx.imageSmoothingEnabled = false;
    const s = app.state();
    hots = []; tip = null;
    ctx.drawImage(floor(), 0, 0);
    rect(ctx, W - 1, 0, 1, H, INK2);
    if (!s) return;
    const w = world(), bots = s.bots || [], usage = s.usage || {};
    const L = layout(s, bots, moving);
    const hp = !moving && !held && !hovered && !carrying ? pieceAt(pointer) : null, lit = (key) => !!hp && hp.key === key;
    lastL = L;
    if (Object.keys(L.placed).length) app.act({ kind: "layout", layout: { ...(s.layout || {}), ...L.placed } });
    s.layout = { ...(s.layout || {}), ...L.placed };

    // keep a figure per bot; one that turns up later comes in through its machine's door, one that
    // is stopped walks out of it
    const ids = new Set(bots.map((b) => b.id)), you = { id: "you", name: (s.you && s.you.name) || "you", seed: s.you && s.you.seed, color: s.you && s.you.color, human: true, m: "" };
    ids.add("you");
    for (const f of w.folk.values()) if (!ids.has(f.id) && !f.leaving) { f.leaving = true; f.held = false; f.lying = false; f.sat = false; f.legs = "stand"; }
    for (const [id, t] of w.stopped) if (!ids.has(id) || performance.now() - t > 30000) w.stopped.delete(id);
    const late = performance.now() - w.born > 3000;
    for (const b of [...bots, you]) {
      if (w.stopped.has(b.id)) continue;      // stopped from here: the feed still lists it for a moment
      if (!w.folk.has(b.id)) {
        const at = late || b.human ? { x: doorX(b.m), y: WALL + 3 } : L.seat(b.id) || { x: W / 2, y: H / 2 };
        const work = !b.human && (b.busy || b.waiting), stand = late || b.human;
        w.folk.set(b.id, { id: b.id, x: at.x, y: at.y, dir: -1, arms: "down", legs: stand ? "stand" : "sit", step: 0, sat: !stand,
          act: work ? { kind: "desk" } : rest(), carry: null, lying: false, busy: b.busy, waiting: b.waiting, work, human: !!b.human,
          idleSince: performance.now() / 1000, chat: 0, z: Math.random() * 2 });
      }
      w.folk.get(b.id).b = b;
    }
    if (tickDt > 0) {
      w.T += tickDt; think(w, bots, L, tickDt, s.demo ? 40 : 600, usage, s.chess);
      for (const f of w.folk.values()) {
        const m = (f.b && f.b.m) || "";
        if (f.leaving && walk(f, { x: doorX(m), y: WALL + 1 }, 30, tickDt)) { w.folk.delete(f.id); w.doorAt[m] = performance.now(); }
      }
      tickDt = 0;
    }
    const T = w.T;
    // who is highlighted: the one under the pointer, the one whose session the pointer is over,
    // a project's bots while the pointer is on its screen, a machine's while it is on its door
    const hl = new Set();
    if (app.hl()) hl.add(app.hl());

    // the doors, one a machine; its name and its bots lit under the pointer. A door is ajar while
    // someone goes in or out; boarded up while its machine is out of reach (why, under the pointer).
    // After them an empty door frame, for another machine
    s.machines.forEach((m, i) => {
      const x0 = 2 + i * DOOR, h = over(x0, WALL - 20, x0 + 16, WALL);
      const ajar = performance.now() - (w.doorAt[m.id] || 0) < 900 ||
        [...w.folk.values()].some((f) => f.b && (f.b.m || "") === m.id && ((f.act.kind === "doom" && f.act.where === "door") || (f.leaving && f.y < WALL + 12)));
      drawDoor(ctx, x0, paintOf(m.id), ajar && !m.error, !!m.error, h);
      if (h) { for (const b of bots) if ((b.m || "") === m.id) hl.add(b.id); tip = { text: clip(m.error ? m.name + ": " + m.error : m.name, W - 8), x: x0 + 8, y: 20 }; }
    });
    {
      const x0 = 2 + s.machines.length * DOOR, h = hot(x0, WALL - 20, x0 + 16, WALL, { kind: "addMachine" });
      drawFrame(ctx, x0, h);
      if (h) tip = { text: "machine", x: x0 + 8, y: 20 };
    }
    // trouble here goes on the wall, in red, instead of the whiteboard
    const here = s.machines.find((m) => !m.id), trouble = s.note || (here && here.error);
    if (trouble) { const nx = wallFrom(); tiny(ctx, clip(trouble, W - 24 - nx), nx, 18, RED); }
    for (const win of L.windows) outlined(ctx, [win.x0, win.y0, win.x1, win.y1], lit(win.key), (c) => drawWindow(c, win));
    if (L.clock) outlined(ctx, foot("clock", [L.clock.x0, L.clock.y0]), lit(L.clock.key), (c) => drawClock(c, L.clock.x0 + 6, L.clock.y0 + 5));
    if (L.closet) {
      // the closet, against the wall: the furniture that is not in the room; a click opens it (on the right)
      const c = L.closet, h = hot(c.x0, c.y0, c.x1, c.y1, { kind: "closet" });
      outlined(ctx, [c.x0, c.y0, c.x1, c.y1], h, (x) => drawCloset(x, c, app.view() === "closet"));
      if (h) tip = { text: "furniture", x: c.x0 + 11, y: c.y0 + 4 };
    }
    // a bot and its seat at its project's desk are one thing: where the bot is only says what it
    // does. The pointer on either lights up both (and its session), and a click on either opens it
    if (hoverId) hl.add(hoverId);
    let seatHover = null;
    if (!moving && !held) for (const d of L.desks) for (const [id, x] of d.seats) if (over(x - 3, d.y - 16, x + 9, d.y + 1)) seatHover = { id, x, y: d.y };
    if (seatHover) hl.add(seatHover.id);

    // flat on the floor: the empty cabinet that adds a project, the mats, the ping-pong rug
    if (L.add) {
      const a = L.add, h = hot(a.x0 - 2, a.y0 - 10, a.x1 + 2, a.y1 + 1, { kind: "addDir" });
      outlined(ctx, foot("add", [a.x0, a.y1]), h, (c) => drawAdd(c, a, h));
      if (h) tip = { text: "project", x: a.x0 + ADD_W / 2, y: a.y0 - 2 };
    }
    for (const m of L.mats) outlined(ctx, foot("mat", [m.x, m.y]), lit(m.key), (c) => drawMat(c, m, [...w.folk.values()].some((f) => f.act.kind === "sleep" && f.act.there && f.act.mat === m.key), w.T));
    if (L.pong) drawRug(ctx, L.pong);

    // everything that stands, back to front
    const items = [];
    for (const f of w.folk.values()) if (!f.lying && !f.held) items.push([f.y, () => drawFigure(ctx, f, f.b, T, hl)]);
    for (const d of L.desks) {
      items.push([d.y - 1, () => drawDesk(ctx, d, w, T, hl)]);
      for (const [id, x] of d.seats) items.push([d.y - 0.5, () => drawChair(ctx, x, d.y, hl.has(id))]);
    }
    // every piece: outlined under the pointer, like a desk's chairs; what it does, on top of that
    const piece = (p, kind, fn) => items.push([p.y - 0.5, () => outlined(ctx, foot(kind, [p.x, p.y]), lit(p.key), fn)]);
    if (L.meet) piece(L.meet, "meet", (c) => { drawTable(c, L.meet); for (const i of [0, 1]) drawStool(c, L.meetSeat(i)); });
    if (L.chess) {
      const c = L.chess, g = s.chess && !s.chess.result ? s.chess : null;
      if (hot(c.x - 12, c.y - 17, c.x + 12, c.y, { kind: "chess" }))
        tip = { text: g ? "chess with " + (g.white.id === "you" ? g.black.name : g.white.name) : "chess", x: c.x, y: c.y - 18 };
      piece(c, "chess", (x) => { drawChess(x, c, g ? g.board : w.chess.board); for (const i of [0, 1]) drawStool(x, L.chessSeat(i)); });
    }
    if (L.coffeeAt) {
      const brewing = [...w.folk.values()].some((f) => f.act.kind === "coffee" && f.act.stage === 1);
      piece(L.coffeeAt, "coffee", (c) => drawCoffee(c, L.coffeeAt, brewing, T, usage.session));
    }
    if (L.cooler) piece(L.cooler, "cooler", (c) => drawCooler(c, L.cooler, usage.week));
    if (L.printer) {
      // a click prints a report of what goes on (the page comes out of it)
      const pr = L.printer, job = [...w.folk.values()].find((f) => f.act.kind === "print" && f.act.stage === 1);
      const printing = (performance.now() - w.print) / 1000, h = hot(pr.x - 7, pr.y - 14, pr.x + 7, pr.y + 1, { kind: "report" });
      piece(pr, "printer", (c) => drawPrinter(c, pr, job ? job.act.wait : printing < 3 ? printing : undefined));
      if (h) tip = { text: "report", x: pr.x, y: pr.y - 16 };
    }
    for (const ch of L.chairs) piece(ch, "armchair", (c) => drawArmchair(c, ch));
    if (L.board) piece(L.board, "board", (c) => drawBoard(c, L.board, w));
    if (L.fire) {
      // the fireplace: what is held over it (a desk, a piece of furniture, a bot) is about to burn:
      // the fire flares and the fireplace lights up red. A desk with bots can't burn. The pointer
      // on it stirs sparks
      const F = L.fire, r = [F.x0, F.y0, F.x1, F.y1];
      const feeding = (moving && inFire(L, moving.key, moving.at, moving.u)) || (held && held.b && !held.b.human && pointer && inRect(pointer, r));
      const fd = moving && feeding && L.desks.find((d) => d.key === moving.key);
      const onIt = !moving && !held && over(...r);
      if (onIt && !w.sparkOn) { w.sparkOn = true; w.sparks = performance.now(); } else if (!onIt) w.sparkOn = false;
      const since = Math.min(performance.now() - w.sparks, performance.now() - w.flare), burst = since < 1500 ? since / 1500 : -1;
      items.push([F.y1, () => outlined(ctx, r, !!feeding || onIt, (c) => drawFire(c, F, T, !!feeding, burst), feeding ? RED : TERRACOTTA)]);
      const tx = F.x0 + 16, ty = F.y0 - 2;
      if (fd) tip = { text: fd.seats.length ? fd.label + " has bots" : "burn " + fd.label, x: tx, y: ty };
      else if (feeding && moving) tip = { text: "burn the " + (NAMES[KIND(moving.key)] || KIND(moving.key)), x: tx, y: ty };
      else if (feeding) tip = { text: "stop " + held.b.name, x: tx, y: ty };
    }
    L.plants.forEach((pl) => piece(pl, "plant", (c) => drawPlant(c, pl, +(pl.key.match(/\d+$/) || [0])[0] % 2, lit(pl.key), T)));
    for (const pr of L.pairs) {
      // a pair room (a chat of people and agents, pair.daems.me): a click opens the chat; a bot
      // dropped on it joins the chat. Its light is on once a room is linked
      const info = (s.pair || {})[pr.key], h = hot(pr.x - 13, pr.y - 20, pr.x + 13, pr.y + 1, { kind: "pairOpen", key: pr.key });
      const over_ = held && held.b && !held.b.human && pointer && inRect(pointer, foot("pair", [pr.x, pr.y]));
      piece(pr, "pair", (c) => drawPair(c, pr, !!info, T, over_));
      if (h) tip = { text: info ? info.room || "pair" : "pair: link a room", x: pr.x, y: pr.y - 22 };
      else if (over_) tip = { text: info ? held.b.name + " joins " + (info.room || "the room") : "no room linked yet", x: pr.x, y: pr.y - 22 };
    }
    if (L.pong) {
      piece(L.pong, "pong", (c) => drawPong(c, L.pong, w));
      if (hot(L.pong.x - 16, L.pong.y - 17, L.pong.x + 16, L.pong.y, { kind: "pong" })) tip = { text: "ping-pong", x: L.pong.x, y: L.pong.y - 18 };
    }
    // the limits, in the coffee pot (this five-hour session) and the cooler's bottle (this week)
    const c0 = L.coffeeAt;
    if (c0 && over(c0.x, c0.y - 22, c0.x + 20, c0.y + 1)) tip = { text: say(usage.session, "5h"), x: c0.x + 10, y: c0.y - 24 };
    if (L.cooler && over(L.cooler.x - 4, L.cooler.y - 22, L.cooler.x + 4, L.cooler.y + 1)) tip = { text: say(usage.week, "1w"), x: L.cooler.x, y: L.cooler.y - 24 };
    hovered = null;
    for (const f of w.folk.values()) if (f.lying) drawSleeper(ctx, f, f.b, T, hl);
    items.sort((a, b) => a[0] - b[0]);
    for (const [, d] of items) d();
    // a bot being carried, over everything; a piece carried in from the closet, where it would stand
    for (const f of w.folk.values()) if (f.held) drawFigure(ctx, f, f.b, T, hl);
    if (carrying && carrying.at) { ctx.globalAlpha = 0.7; drawKind(ctx, carrying.kind, carrying.at, w, T); ctx.globalAlpha = 1; }
    if (held) hovered = null;
    if (!hovered && seatHover && !held) { const f = w.folk.get(seatHover.id); if (f && f.b) hovered = { b: f.b, x: seatHover.x + 3, y: seatHover.y - 16, seat: true }; }

    // a message on its way: a beam in the sender's colour, up from its antenna, across the room over
    // everyone's heads, down into the receiver's antenna; its bright head leads, its tail follows
    for (const bm of w.beams) {
      const f = w.folk.get(bm.from), g = w.folk.get(bm.to);
      if (!f || !g) continue;
      const top = (h) => Math.round(h.y) - (h.lying ? 27 : 29), UP = 6;
      const ax = Math.round(f.x) + (f.dir < 0 ? -1 : 0), ay = top(f), bx = Math.round(g.x) + (g.dir < 0 ? -1 : 0), by = top(g);
      const pts = [[ax, ay], [ax, ay - UP], [bx, by - UP], [bx, by]];
      const len = [UP, Math.hypot(bx - ax, by - ay), UP], total = len[0] + len[1] + len[2];
      const at = (s) => {                    // the point s dots along the beam
        let k = 0; while (k < 2 && s > len[k]) { s -= len[k]; k++; }
        const [x0, y0] = pts[k], [x1, y1] = pts[k + 1], u = len[k] ? s / len[k] : 0;
        return [Math.round(x0 + (x1 - x0) * u), Math.round(y0 + (y1 - y0) * u)];
      };
      const head = Math.min(1, bm.t / BEAM) * total, tail = Math.max(0, (bm.t - 0.25) / BEAM) * total, col = colorOf(f.b);
      for (let s = Math.ceil(tail); s <= head; s++) { const [x, y] = at(s); rect(ctx, x, y + 1, 1, 1, INK); rect(ctx, x, y, 1, 1, col); }
      if (head < total) { const [x, y] = at(head); rect(ctx, x - 1, y - 1, 3, 3, INK); rect(ctx, x - 1, y - 1, 2, 2, C.steel4); }
    }
    // talk, over everything: dots in a bubble, never the words
    for (const f of w.folk.values()) if (f.chat > 0 && !still) {
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - (f.lying ? 28 : 27) - 6;
      rect(ctx, bx - 1, by - 1, 11, 7, INK); rect(ctx, bx, by, 9, 5, C.steel4); rect(ctx, bx, by + 5, 1, 1, C.steel4);
      for (let i = 0; i < 3; i++) if ((Math.floor(f.chat * 4) + i) % 3) rect(ctx, bx + 2 + i * 2, by + 2, 1, 1, INK);
    }
    // about to be stopped (put at its door, or into the fire): a sign over it counts the seconds
    // down; a click on it is the undo
    for (const f of w.folk.values()) if (f.act.kind === "doom" && f.b && !f.gone) {
      const text = f.b.name + " stops - undo", tw = tinyW(text), lx = Math.round(Math.max(2, Math.min(W - tw - 2, f.x - tw / 2))), ly = Math.round(f.y) - 36;
      const h = hot(lx - 2, ly - 2, lx + tw + 2, ly + 9, { kind: "undo", id: f.id });
      box(ctx, lx - 2, ly - 2, tw + 4, 11, h ? TERRACOTTA : INK2);
      tiny(ctx, text, lx, ly, h ? INK : C.steel4);
      rect(ctx, lx, ly + 7, Math.round(tw * Math.max(0, f.act.left) / 5), 1, h ? INK : RED);
    }
    // the bot under the pointer: its name over its head, and a click opens it
    if (hovered && !moving) {
      const { b, x, y } = hovered;
      tip = { text: b.human ? b.name : b.name + (b.waiting ? " - needs input" : b.alive === false ? " - stopped, opens again" : ""), x, y: y - 2 };
      hots.push({ x0: -1e9, y0: -1e9, x1: 1e9, y1: 1e9, action: b.human ? { kind: "you" } : { kind: "open", id: b.id } });
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

  // the wall and the wooden floor, drawn once for the room's height: Pixel Agents' planks, three
  // dots wide, their joints staggered, a shadow where the floor meets the wall
  let floorC = null;
  function floor() {
    if (floorC && floorC.height === H && floorC.width === W) return floorC;
    floorC = document.createElement("canvas"); floorC.width = W; floorC.height = H;
    const ctx = floorC.getContext("2d");
    rect(ctx, 0, WALL, W, H - WALL, PLANK);
    for (let y = WALL + 3, row = 0; y < H; y += 4, row++) {
      rect(ctx, 0, y, W, 1, SEAM);
      for (let x = -(fnv("r" + row) % 48); x < W; x += 40 + (fnv(row + ":" + x) % 4) * 8) rect(ctx, x, y + 1, 1, 3, SEAM);
    }
    rect(ctx, 0, 0, W, WALL, WALLC);
    rect(ctx, 0, WALL - 2, W, 2, BASEBOARD);
    rect(ctx, 0, WALL, W, 1, "rgba(0,0,0,0.3)"); rect(ctx, 0, WALL + 1, W, 1, "rgba(0,0,0,0.12)");
    return floorC;
  }

  function drawFigure(ctx, f, b, T, hl) {
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), x0 = x - 11, y0 = y - 27;
    if (!f.held) rect(ctx, x - 5, y, 10, 1, "rgba(0,0,0,0.4)");
    const im = body(seedOf(b), colorOf(b), f.held ? "up" : f.arms, f.held ? "walk1" : f.legs, b.human);
    // its outline: terracotta while the pointer is on it or on its session, faint while its session is open
    const lit = hl.has(f.id) || (hovered && hovered.b.id === f.id) || (!hovered && over(x0 + 4, y0, x0 + 18, y + 1));
    const ring = f.leaving ? null : lit ? TERRACOTTA : app.opens().includes(f.id) ? OPEN : null;
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
    if (!f.leaving && !f.held && f.act.kind !== "doom" && over(x0 + 4, y0, x0 + 18, y + 1)) hovered = { b, x, y: y0 };
  }
  function drawSleeper(ctx, f, b, T, hl) {  // seen from above: the head on the pillow, a blanket, a plug
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), im = body(seedOf(b), colorOf(b), "down", "stand", b.human);
    const ring = hl.has(f.id) || over(x - 11, y - 28, x + 11, y) ? TERRACOTTA : app.opens().includes(f.id) ? OPEN : null;
    if (im) { if (ring) ctx.drawImage(halo(im, ring), 0, 0, SW, 16, x - 11, y - 28, SW, 16); ctx.drawImage(im, 0, 0, SW, 16, x - 11, y - 28, SW, 16); }
    const c0 = b.human ? colorOf(b) : C.steel2;   // a blanket: grey for a charging bot, yours in your colour
    box(ctx, x - 10, y - 13, 20, 9, c0, MAT.k);
    rect(ctx, x - 10, y - 13, 20, 2, shade(c0, 1.25));
    rect(ctx, x - 10, y - 6, 20, 2, shade(c0, 0.7));
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
    const { x0, y, w: dw, cab } = d, p = paintOf(d.m);
    // a screen runs only while a bot sits at it typing; dim while one sits there idle; a blinking
    // cursor while one waits for you. The big one is the project's own directory, a laptop a worktree's
    const seated = (f) => f && f.legs === "sit" && !f.lying && (f.act.kind === "desk" || f.act.kind === "sleep");
    const at = (tree) => d.seats.filter((st) => st[2] === tree).map(([id]) => w.folk.get(id)).filter(seated);
    function paint(g, gw, gh, tree, seed) {
      // off while nobody sits there, or while the sitter's process is stopped
      const f = at(tree).filter((x) => x.b && x.b.alive !== false), busy = f.some((x) => x.busy), waits = f.some((x) => x.waiting);
      rect(ctx, g.x, g.y, gw, gh, f.length ? C.blue0 : "#10141f");
      if (!f.length) { rect(ctx, g.x + gw - 3, g.y + 1, 1, 1, "#232c40"); rect(ctx, g.x + gw - 4, g.y + 2, 1, 1, "#232c40"); return; }   // off: a glint
      ctx.fillStyle = busy ? d.color : shade(d.color, 0.45);
      const s = busy ? Math.floor(T * 5) : 0;
      for (let r = 0; r < Math.floor(gh / 2); r++) ctx.fillRect(g.x + 1 + ((r * 3) % 4), g.y + 1 + r * 2, Math.min(gw - 2, 3 + ((r * 7 + s + seed) * 5) % (gw > 10 ? 11 : 4)), 1);
      if (waits && !busy && Math.floor(T * 2) % 2) rect(ctx, g.x + 2, g.y + gh - 2, 2, 1, C.steel4);
    }
    // the desktop, lower than the cabinet: a leg between each two chairs, a keyboard at each seat
    slab(ctx, x0 + cab - 2, y - 12, dw - cab + 2, 5, 4, p);
    for (const x of [x0 + cab + 1, ...d.seats.map(([, x]) => x + 11), x0 + dw - 4]) {
      rect(ctx, x, y - 4, 2, 4, p.k); rect(ctx, x, y - 4, 1, 3, p.d); rect(ctx, x, y, 2, 1, SHADOW);
    }
    for (const x of [...d.seats.map(([, x]) => x), ...(d.empty ? [d.empty] : [])]) { rect(ctx, x - 9, y - 11, 7, 2, SP.F); rect(ctx, x - 8, y - 11, 5, 1, SP.B); }
    // the cabinet, the project's name on a plate
    slab(ctx, x0, y - 15, cab, 4, 11, p);
    const lx = Math.round(x0 + (cab - tinyW(d.label)) / 2);
    plate(ctx, x0 + 2, y - 9, cab - 4, false, p); tiny(ctx, d.label, lx, y - 8, C.steel3);
    // on it the project's screen, and a laptop for each worktree, plugged in; the pointer on one
    // lights up the bots working there. The desk is handled at its screen: a + beside it starts a
    // bot here
    const gw = 22 + d.trees.length * LAP, sx = Math.round(x0 + (cab - gw) / 2), sy = y - 32;
    const where = (dir) => (d.m ? machineName(d.m) + ":" : "") + dir.replace(/^\/(Users|home)\/[^/]+/, "~");
    const onScreen = over(sx, sy, sx + 22, sy + 18);
    if (onScreen) { for (const [id] of d.seats) hl.add(id); tip = { text: where(d.dir), x: sx + 11, y: sy - 1 }; }
    {
      const px = sx + gw + 1, h = hot(px - 2, sy - 2, px + 5, sy + 7, { kind: "spawn", m: d.m, dir: d.dir });
      tiny(ctx, "+", px, sy, h ? d.color : C.steel2);
      if (h) tip = { text: "new bot", x: px + 1, y: sy - 1 };
    }
    paint({ x: sx + 2, y: sy + 2 }, 18, 11, "", d.dir.length);
    const mon = sprites.get("monitor");
    if (onScreen) put(ctx, halo(mon, C.steel3), sx, sy);
    put(ctx, mon, sx, sy);
    if (d.trees.length) rect(ctx, sx + 17, y - 13, gw - 22, 1, SP.F);       // the cable, along the top
    d.trees.forEach((tree, k) => {
      const px = sx + 23 + k * LAP, py = y - 22, lap = sprites.get("laptop");
      const on = over(px, py, px + 12, py + 11), mine = d.seats.filter((st) => st[2] === tree);
      if (on) { for (const [id] of mine) hl.add(id); tip = { text: tree.split("/").pop() + ": a worktree", x: px + 6, y: py - 1 }; }
      paint({ x: px + 2, y: py + 2 }, 8, 5, tree, tree.length);
      if (on || mine.some(([id]) => hl.has(id))) put(ctx, halo(lap, on ? C.steel3 : TERRACOTTA), px, py);
      put(ctx, lap, px, py);
    });
  }
  // a name plate on a cabinet's front: seven dots high, for one line of lettering
  function plate(ctx, x, y, w, lit, p = WOOD) {
    rect(ctx, x, y, w, 7, p.k);
    rect(ctx, x + 1, y + 1, w - 2, 5, lit ? C.steel1 : p.plate);
  }
  // an office chair at a project's desk, its sitter facing the screen; none where no bot is yet
  function drawChair(ctx, x, y, lit) {
    const im = sprites.get("chair");
    if (lit) put(ctx, halo(im, TERRACOTTA), x - 4, y - 16);
    put(ctx, im, x - 4, y - 16);
  }
  // a desk carried to a door is on its way out: a sign over the door asks; a click on it takes
  // the desk away (its bots must be gone first)
  // the fireplace: a fire pit in the middle of the room, seen from above like the tables: a ring of
  // stones on the floor, embers and logs in it, the flames rising from it. What is dragged into it
  // burns: a desk (its bots must be gone first), a piece of furniture, a bot (stopped, after a
  // countdown). `burst` (0..1) is a shower of sparks: the pointer stirred it, or something burnt
  const inFire = (L, key, at, u) => L.fire && (key.startsWith("u:") || !FIXED.has(KIND(key))) &&
    meets(foot(key, at, u), [L.fire.x0, L.fire.y0, L.fire.x1, L.fire.y1 + 4]);
  const ELL = (rx, ry, rows) => [...Array(rows)].map((_, j) => Math.round(rx * Math.sqrt(Math.max(0, 1 - ((j + 0.5 - rows / 2) / (rows / 2)) ** 2))));
  function drawFire(ctx, f, T, feeding, burst = -1) {
    const { x0, y0 } = f, cx = x0 + 16, r = (y, x, w, c) => rect(ctx, cx + x, y0 + y, w, 1, c);
    const ST = { k: "#4d4a4f", m: "#8d8a86", hi: "#aaa59e", d: "#6e6b6a" };    // the stones
    // the ring of stones, 32 x 12 at the bottom, lit along its far side, in shade on the near one
    const top = ELL(15, 6, 12);
    top.forEach((hw, j) => { r(16 + j, -hw, hw * 2, ST.k); if (hw > 2) r(16 + j, -hw + 1, hw * 2 - 2, j < 4 ? ST.hi : j > 7 ? ST.d : ST.m); });
    for (const [x, y] of [[-12, 20], [-6, 17], [2, 17], [9, 18], [12, 22], [-10, 25], [-1, 26], [6, 26], [11, 25], [-13, 23]]) r(y, x, 1, ST.k);
    ELL(12, 4, 8).forEach((hw, j) => r(18 + j, -hw, hw * 2, "#120a0c"));
    r(28, -14, 28, SHADOW);
    const k = Math.floor(T * 6) % 3, big = feeding ? 4 : 0;
    // embers and two logs
    for (const [x, y] of [[-9, 21], [-7, 23], [7, 20], [9, 23], [-3, 24], [4, 24]]) r(y, x, 1, (k + x) % 2 ? C.red1 : "#7a2a18");
    r(22, -7, 6, "#3b2a1e"); r(23, -8, 7, "#5c3a26"); r(22, 1, 6, "#3b2a1e"); r(23, 2, 6, "#5c3a26");
    // the flames, up from the embers: tongues of three heights a frame, red, gold in them, yellow at the heart
    const flame = (x, h, c) => rect(ctx, cx + x, y0 + 22 - h, 1, h, c);
    const F = [[-7, 2 + k], [-6, 4 + (k + 1) % 3], [-5, 6 + k], [-4, 8 + (k + 2) % 3], [-3, 10 + k], [-2, 12 + (k + 1) % 3], [-1, 14 + k], [0, 15 + (k + 2) % 3],
      [1, 14 + k], [2, 12 + (k + 1) % 3], [3, 10 + k], [4, 8 + (k + 2) % 3], [5, 6 + k], [6, 4 + (k + 1) % 3], [7, 2 + k]];
    for (const [x, h] of F) flame(x, Math.min(18, h + big), C.red1);
    for (const [x, h] of F) flame(x, Math.max(1, Math.round(Math.min(18, h + big) * 0.62)), C.gold1);
    for (const [x, h] of F) if (x > -4 && x < 4) flame(x, Math.max(1, Math.round(Math.min(18, h + big) * 0.3)), C.gold2);
    if (burst >= 0) for (let i = 0; i < 14; i++) {      // sparks, out and up, dying out
      const a = (i * 2.399) % 6.283, sp = 0.5 + ((i * 7919) % 100) / 200, t = burst * sp * 1.6;
      if (t > 1) continue;
      const sx = Math.round(Math.sin(a) * 16 * t), sy = Math.round(22 - 8 * Math.cos(a) * t - 26 * t);
      r(sy, sx, 1, t < 0.5 ? C.gold2 : t < 0.8 ? C.gold1 : C.red1);
    }
  }
  // the closet, against the wall (it holds the furniture that is not in the room): a wardrobe seen
  // from the front like the doors, its right door open while its page is
  function drawCloset(ctx, c, open) {
    const { x0, y0 } = c;
    rect(ctx, x0, y0, 22, 24, WOOD.k);
    rect(ctx, x0 + 1, y0 + 1, 20, 22, WOOD.d);
    rect(ctx, x0 + 1, y0 + 1, 20, 1, WOOD.hi); rect(ctx, x0 + 1, y0 + 2, 20, 1, WOOD.top);
    for (const dx of [2, 12]) { rect(ctx, x0 + dx, y0 + 4, 8, 18, WOOD.top); rect(ctx, x0 + dx + 1, y0 + 5, 6, 16, WOOD.d); rect(ctx, x0 + dx + 1, y0 + 5, 6, 1, WOOD.top); }
    rect(ctx, x0 + 11, y0 + 3, 1, 20, WOOD.k);
    if (open) {                               // the right door open: shelves inside, something on them
      rect(ctx, x0 + 12, y0 + 3, 9, 20, "#1c1420");
      for (const y of [9, 15]) rect(ctx, x0 + 12, y0 + y, 9, 1, WOOD.d);
      rect(ctx, x0 + 14, y0 + 6, 2, 3, SP.g); rect(ctx, x0 + 17, y0 + 11, 3, 4, SP.C); rect(ctx, x0 + 14, y0 + 17, 4, 2, SP.y);
      rect(ctx, x0 + 21, y0 + 2, 1, 21, WOOD.k); rect(ctx, x0 + 22, y0 + 3, 1, 20, WOOD.top);
    }
    rect(ctx, x0 + 9, y0 + 13, 1, 2, SP.C); if (!open) rect(ctx, x0 + 12, y0 + 13, 1, 2, SP.C);
    rect(ctx, x0 + 2, y0 + 23, 2, 1, WOOD.k); rect(ctx, x0 + 18, y0 + 23, 2, 1, WOOD.k);     // its feet
  }
  // the whiteboard on wheels, on the floor: its face 30 x 15 (the host draws on it), two posts, a
  // base on four wheels
  const INK_AREA = { x: 2, y: 2, w: 26, h: 11 };
  function drawBoard(ctx, b, w) {
    const fx = b.x - 15, fy = b.y - 26;
    rect(ctx, fx, fy, 30, 15, SP.F); rect(ctx, fx + 1, fy + 1, 28, 13, "#efebe8");
    rect(ctx, fx + 1, fy + 14, 28, 1, SP.E);                      // the tray
    rect(ctx, fx + 3, fy + 13, 3, 1, SP.r); rect(ctx, fx + 7, fy + 13, 3, 1, SP.v);   // markers
    for (const px of [fx + 3, fx + 26]) { rect(ctx, px, fy + 15, 1, 8, SP.F); rect(ctx, px + 1, fy + 15, 1, 8, SP.E); }
    rect(ctx, fx + 1, fy + 23, 28, 2, SP.F); rect(ctx, fx + 2, fy + 23, 26, 1, SP.E);
    for (const px of [fx, fx + 5, fx + 24, fx + 29]) rect(ctx, px, fy + 25, 1, 2, SP.F);
    rect(ctx, fx + 1, fy + 27, 28, 1, SHADOW);
    const ix = fx + INK_AREA.x, iy = fy + INK_AREA.y;
    ctx.fillStyle = shade(C.blue2, 0.75);
    for (const s of w.board || []) {
      const x = ix + s.x, y = iy + s.y;
      if (s.t === "box") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x, y + s.h, s.w, 1); ctx.fillRect(x, y, 1, s.h); ctx.fillRect(x + s.w - 1, y, 1, s.h + 1); }
      else if (s.t === "line") { ctx.fillRect(x, y, s.w, 1); ctx.fillRect(x + s.w - 2, y - 1, 1, 1); ctx.fillRect(x + s.w - 2, y + 1, 1, 1); }
      else ctx.fillRect(x, y, 1, 1);
    }
  }
  // a pair room: a small round table with a speakerphone on it, its light on while a room is linked,
  // blinking while a bot is held over it (it would join the chat)
  function drawPair(ctx, p, linked, T, over) {
    const { x, y } = p, PL = { k: "#3f3740", d: "#595e60", m: "#757b7c", hi: "#b5bfc7" };
    ELL(13, 4, 8).forEach((hw, j) => { rect(ctx, x - hw, y - 20 + j, hw * 2, 1, WOOD.k); if (hw > 1) rect(ctx, x - hw + 1, y - 20 + j, hw * 2 - 2, 1, j < 2 ? WOOD.hi : WOOD.top); });
    for (const [yy, hw] of [[-12, 12], [-11, 11], [-10, 9]]) { rect(ctx, x - hw, y + yy, hw * 2, 1, WOOD.k); rect(ctx, x - hw + 1, y + yy, hw * 2 - 2, 1, WOOD.d); }
    rect(ctx, x - 2, y - 9, 4, 8, WOOD.k); rect(ctx, x - 1, y - 9, 2, 8, WOOD.d);                       // the stem and the foot
    rect(ctx, x - 6, y - 1, 12, 2, WOOD.k); rect(ctx, x - 5, y - 1, 10, 1, WOOD.d); rect(ctx, x - 5, y + 1, 10, 1, SHADOW);
    // the speakerphone: a dark triangle-ish puck, a light on top
    rect(ctx, x - 4, y - 18, 8, 4, PL.k); rect(ctx, x - 3, y - 18, 6, 3, PL.d); rect(ctx, x - 3, y - 18, 6, 1, PL.m);
    rect(ctx, x - 6, y - 16, 2, 2, PL.k); rect(ctx, x + 4, y - 16, 2, 2, PL.k); rect(ctx, x - 1, y - 20, 2, 2, PL.k);
    const on = linked && (!over || Math.floor(T * 6) % 2);
    rect(ctx, x, y - 17, 1, 1, on ? SP.g : "#2a2f33");
  }
  // the empty cabinet that adds a project: a plate with a + on it
  function drawAdd(ctx, a, lit) {
    slab(ctx, a.x0, a.y0, ADD_W, 4, 11, WOOD);
    plate(ctx, a.x0 + 3, a.y0 + 6, ADD_W - 6, lit);
    tiny(ctx, "+", Math.round(a.x0 + ADD_W / 2 - 1), a.y0 + 7, lit ? C.steel4 : C.steel2);
  }
  // a piece drawn into a canvas of its footprint and put with an outline around it (the pointer is
  // on it, or it is in danger), or drawn straight
  function outlined(ctx, r, on, fn, col = TERRACOTTA) {
    if (!on) { fn(ctx); return; }
    const pad = 2, w = r[2] - r[0] + 2 * pad, h = r[3] - r[1] + 2 * pad, c = document.createElement("canvas");
    c.width = w; c.height = h;
    const o = c.getContext("2d");
    o.imageSmoothingEnabled = false; o.translate(pad - r[0], pad - r[1]);
    fn(o);
    ctx.drawImage(halo(c, col), r[0] - pad, r[1] - pad); ctx.drawImage(c, r[0] - pad, r[1] - pad);
  }
  // any kind of furniture drawn at a spot, at rest: for the closet's pictures and for the ghost of
  // a piece carried into the room (`at` as the layout keeps it: its anchor)
  function drawKind(ctx, kind, [x, y], w, T) {
    const pt = { x, y };
    switch (kind) {
      case "plant": drawPlant(ctx, pt, 0); break;
      case "armchair": drawArmchair(ctx, pt); break;
      case "mat": drawMat(ctx, pt, false, T); break;
      case "coffee": drawCoffee(ctx, pt, false, T, null); break;
      case "cooler": drawCooler(ctx, pt, null); break;
      case "printer": drawPrinter(ctx, pt, undefined); break;
      case "chess": drawChess(ctx, pt, null); for (const d of [-13, 13]) drawStool(ctx, { x: x + d, y }); break;
      case "pong": drawRug(ctx, pt); drawPong(ctx, pt, w); break;
      case "meet": drawTable(ctx, pt); for (const d of [-23, 23]) drawStool(ctx, { x: x + d, y }); break;
      case "board": drawBoard(ctx, pt, w); break;
      case "window": drawWindow(ctx, { x0: x, y0: y, x1: x + 21, y1: y + 18 }); break;
      case "clock": drawClock(ctx, x + 6, y + 5); break;
      case "fire": drawFire(ctx, { x0: x, y0: y - 28, x1: x + 32, y1: y }, T, false); break;
      case "closet": drawCloset(ctx, { x0: x, y0: y }, false); break;
      case "pair": drawPair(ctx, pt, true, T, false); break;
      case "add": drawAdd(ctx, { x0: x, y0: y - 15, x1: x + ADD_W, y1: y }, false); break;
    }
  }
  // a picture of a kind of furniture, for the closet: a canvas, two pixels a dot as the room
  function icon(kind) {
    const r = foot(kind, [0, 0]), pad = 4, c = document.createElement("canvas"), S = K;
    c.width = (r[2] - r[0] + 2 * pad) * S; c.height = (r[3] - r[1] + 2 * pad) * S;
    const ctx = c.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    ctx.setTransform(S, 0, 0, S, (pad - r[0]) * S, (pad - r[1]) * S);
    drawKind(ctx, kind, [0, 0], { board: [], folk: new Map(), pong: { t: 0, from: 0 } }, 0);
    return c.toDataURL();
  }
  // a piece carried in from the closet (client coordinates): its ghost follows the pointer while
  // it is over the room; put down, its place is returned (null outside the room)
  let carrying = null;
  function carryKind(kind, cx, cy) {
    if (!kind) { carrying = null; draw(); return null; }
    const b = cv.getBoundingClientRect(), x = (cx - b.left) / K, y = (cy - b.top) / K;
    const inside = x >= 0 && x < W && y >= 0 && y < H;
    carrying = { kind, at: inside ? clampAt(kind, [Math.round(x), Math.round(y)]) : null };
    draw();
    return carrying.at;
  }
  function placeKind(kind, cx, cy) { const at = carryKind(kind, cx, cy); carrying = null; draw(); return at; }
  function drawDoor(ctx, x, p, ajar, boarded, lit) {
    if (lit) rect(ctx, x - 1, WALL - 21, 18, 21, C.steel3);
    put(ctx, sprites.swap(ajar ? "doorOpen" : "door", { h: p.hi, w: p.top, d: p.d, k: p.k }), x, WALL - 20);
    if (boarded) for (const y of [WALL - 16, WALL - 9]) {       // two planks nailed across it
      rect(ctx, x - 1, y, 18, 3, WOOD.k); rect(ctx, x, y + 1, 16, 1, WOOD.top); rect(ctx, x, y, 16, 1, WOOD.hi);
      rect(ctx, x + 1, y + 1, 1, 1, C.steel1); rect(ctx, x + 14, y + 1, 1, 1, C.steel1);
    }
  }
  function drawFrame(ctx, x, lit) {         // where another door could go: its outline, a + in it
    ctx.fillStyle = lit ? TERRACOTTA : "#3d4d66";
    for (let i = 0; i < 16; i += 2) ctx.fillRect(x + i, WALL - 20, 1, 1);
    for (let j = 0; j < 18; j += 2) { ctx.fillRect(x, WALL - 20 + j, 1, 1); ctx.fillRect(x + 15, WALL - 20 + j, 1, 1); }
    tiny(ctx, "+", x + 6, WALL - 14, lit ? TERRACOTTA : C.steel1);
  }
  function drawClock(ctx, x, y) {           // Pixel Agents' clock, telling this computer's time
    put(ctx, "clock", x - 6, y - 5);
    const t = new Date();
    const hand = (ang, len) => { for (let i = 1; i <= len; i++) ctx.fillRect(Math.round(x - 1 + Math.sin(ang) * i), Math.round(y - Math.cos(ang) * i * 0.75), 1, 1); };
    ctx.fillStyle = SP.G; hand(((t.getHours() % 12) + t.getMinutes() / 60) / 12 * 2 * Math.PI, 2);
    ctx.fillStyle = SP.r; hand(t.getMinutes() / 60 * 2 * Math.PI, 3);
    rect(ctx, x - 1, y, 1, 1, SP.F);
  }
  function drawWindow(ctx, w) {              // day or night outside, by this computer's clock
    const hour = new Date().getHours(), day = hour >= 7 && hour < 20, gx = w.x0 + 3, gy = w.y0 + 2;
    rect(ctx, gx, gy, 15, 11, day ? "#7fb2e0" : C.blue0);
    if (day) {
      rect(ctx, gx, gy + 8, 15, 3, "#b7d9f2");
      rect(ctx, gx + 2, gy + 2, 3, 1, C.steel4); rect(ctx, gx + 1, gy + 3, 6, 1, C.steel4); rect(ctx, gx + 10, gy + 5, 4, 1, C.steel4);
    } else {
      rect(ctx, gx + 2, gy + 2, 1, 1, C.steel3); rect(ctx, gx + 5, gy + 8, 1, 1, C.steel2); rect(ctx, gx + 9, gy + 6, 1, 1, C.steel3);
      rect(ctx, gx + 11, gy + 1, 3, 3, C.gold2); rect(ctx, gx + 12, gy + 1, 2, 2, C.blue0);
    }
    put(ctx, "window", w.x0, w.y0);
    ctx.fillStyle = "rgba(255,255,255,0.3)";            // the glass catches the light
    for (const px of [gx + 1, gx + 9]) { ctx.fillRect(px + 3, gy + 1, 1, 1); ctx.fillRect(px + 2, gy + 2, 1, 1); ctx.fillRect(px + 1, gy + 3, 1, 1); }
  }
  function drawRug(ctx, g) {                // pair's rug under the ping-pong table, quiet, fringed at its ends
    const x = g.x - 36, y = g.y - 22, w = 72, h = 28;
    rect(ctx, x, y, w, h, "#2a2133");
    rect(ctx, x + 1, y + 1, w - 2, h - 2, "#3a2d45");
    rect(ctx, x + 3, y + 3, w - 6, h - 6, "#4a3a57");
    rect(ctx, x + 4, y + 4, w - 8, h - 8, "#3a2d45");
    ctx.fillStyle = "#8a7a68";
    for (let j = y + 1; j < y + h - 1; j += 2) { ctx.fillRect(x - 1, j, 1, 1); ctx.fillRect(x + w, j, 1, 1); }
  }
  const MAT = { hi: "#4d5866", top: "#394449", d: "#2b3238", k: SP.F };
  // a mat to charge on: a pillow at its head, the charger beside it, its light on while a bot charges
  function drawMat(ctx, m, charging, T) {
    slab(ctx, m.x - 11, m.y - 23, 22, 20, 2, MAT);
    for (let y = m.y - 15; y < m.y - 5; y += 4) rect(ctx, m.x - 10, y, 20, 1, MAT.d);
    rect(ctx, m.x - 7, m.y - 22, 14, 5, MAT.k); rect(ctx, m.x - 6, m.y - 22, 12, 3, C.steel3); rect(ctx, m.x - 6, m.y - 19, 12, 1, C.steel2);
    box(ctx, m.x + 8, m.y - 28, 5, 8, SP.G, SP.F);
    ctx.fillStyle = C.gold1;                  // a bolt on it
    for (const [dx, dy] of [[3, 0], [2, 1], [1, 2], [2, 2], [3, 2], [2, 3], [1, 4]]) ctx.fillRect(m.x + 8 + dx, m.y - 27 + dy, 1, 1);
    rect(ctx, m.x + 10, m.y - 21, 1, 1, charging && Math.floor(T * 2) % 2 ? SP.g : SP.E);
  }
  function drawTable(ctx, m) {              // Pixel Agents' table, papers and a mug on it
    put(ctx, "SMALL_TABLE_FRONT", m.x - 16, m.y - 30);
    rect(ctx, m.x - 11, m.y - 16, 7, 5, SP.C); rect(ctx, m.x - 11, m.y - 16, 6, 4, SP.A);
    rect(ctx, m.x - 10, m.y - 15, 4, 1, SP.C); rect(ctx, m.x - 10, m.y - 13, 3, 1, SP.C);
    put(ctx, "COFFEE", m.x - 4, m.y - 16);
  }
  function drawChess(ctx, c, board) {       // a small table, the game on it as it stands: white plays from the left
    const b = board || START, x0 = c.x - 10, y0 = c.y - 17;
    put(ctx, "chess", x0, y0);
    for (let r = 0; r < 8; r++) for (let f = 0; f < 8; f++) rect(ctx, x0 + 2 + r * 2, y0 + 9 - f, 2, 1, (r + f) % 2 ? "#e8d2a0" : "#a8784c");
    for (let f = 7; f >= 0; f--) for (let r = 0; r < 8; r++) {      // the pieces stand up from their squares, back to front
      const p = b[r * 8 + f];
      if (!p || p === ".") continue;
      const white = p === p.toUpperCase(), tall = "KQkq".includes(p) ? 3 : "Pp".includes(p) ? 1 : 2;
      const px = x0 + 2 + r * 2 + (white ? 0 : 1), py = y0 + 9 - f;
      rect(ctx, px, py - tall + 1, 1, tall, white ? SP.A : SP.x);
    }
  }
  function drawStool(ctx, st) {             // Pixel Agents' bench
    put(ctx, "WOODEN_BENCH", st.x - 8, st.y - 15);
  }
  // the coffee corner; its pot is the five-hour session's limit: full when nothing is used yet
  function drawCoffee(ctx, at, brewing, T, u) {
    const x0 = at.x, y = at.y;
    put(ctx, "counter", x0, y - 12);
    put(ctx, "brewer", x0 + 1, y - 20);
    if (brewing) { rect(ctx, x0 + 5, y - 15 + (Math.floor(T * 8) % 2), 1, 1, "#8a5030"); rect(ctx, x0 + 4, y - 22 - (Math.floor(T * 4) % 2), 1, 1, C.steel3); }
    // the pot: glass, the coffee in it with a lighter top, one dot = a seventh
    const px = x0 + 11, py = y - 20;
    rect(ctx, px + 1, py + 3, 5, 7, "#b9c6d2");
    const left = u ? Math.max(0, Math.min(7, Math.round(7 * (1 - u.used / 100)))) : 0;
    if (left) { rect(ctx, px + 1, py + 10 - left, 5, left, "#6b3a22"); rect(ctx, px + 1, py + 10 - left, 5, 1, "#b0703c"); }
    put(ctx, "pot", px, py);
  }
  // the water cooler; its bottle is the week's limit
  function drawCooler(ctx, c, u) {
    put(ctx, "cooler", c.x - 5, c.y - 14);
    const bx = c.x - 4, by = c.y - 25;
    rect(ctx, bx + 2, by + 2, 4, 1, "#cfe0ec"); rect(ctx, bx + 1, by + 3, 6, 8, "#cfe0ec");
    const left = u ? Math.max(0, Math.min(8, Math.round(8 * (1 - u.used / 100)))) : 0;
    if (left) { rect(ctx, bx + 1, by + 11 - left, 6, left, C.blue2); rect(ctx, bx + 1, by + 11 - left, 6, 1, C.blue3); }
    put(ctx, "bottle", bx, by);
  }
  function drawPrinter(ctx, pr, wait) {     // on its stand; a page comes out of the front
    put(ctx, "stand", pr.x - 7, pr.y - 6);
    put(ctx, "printer", pr.x - 7, pr.y - 14);
    if (wait !== undefined) { const k = Math.min(4, Math.floor(wait * 2)); box(ctx, pr.x - 4, pr.y - 8, 8, k + 1, C.steel4, SP.D); }
  }
  function drawArmchair(ctx, c) {            // Pixel Agents' armchair, turned to face left, a lamp beside it
    put(ctx, "CUSHIONED_CHAIR_SIDE", c.x - 8, c.y - 15, true);
    put(ctx, "lamp", c.x + 9, c.y - 22);
  }
  function drawPlant(ctx, pl, i, lit, T = 0) {   // Pixel Agents' plants; the leaves sway under the pointer
    const im = sprites.get(i ? "PLANT_2" : "PLANT");
    if (!im) return;
    if (!lit) { ctx.drawImage(im, pl.x - 8, pl.y - 30); return; }
    const sway = Math.floor(T * 5) % 2 ? 1 : -1, cut = im.height - 12;
    ctx.drawImage(im, 0, 0, im.width, cut, pl.x - 8 + sway, pl.y - 30, im.width, cut);
    ctx.drawImage(im, 0, cut, im.width, im.height - cut, pl.x - 8, pl.y - 30 + cut, im.width, im.height - cut);
  }
  function drawPong(ctx, g, w) {
    put(ctx, "pong", g.x - 16, g.y - 19);
    // the ball, once both players are at the table
    const pp = app.pong();
    if ([...w.folk.values()].filter((f) => f.act.kind === "pong" && f.act.there).length === 2 || (pp && pp.ball)) {
      const { t, from } = w.pong, a = from ? 1 : -1, x = g.x + a * 18 * (1 - 2 * t), k = t < 0.5 ? t * 2 : (t - 0.5) * 2;
      const y = g.y - 12 - Math.round(Math.sin(k * Math.PI) * (t < 0.5 ? 6 : 8)) - 6 * (1 - Math.min(1, t * 4));
      rect(ctx, Math.round(x) - 1, Math.round(y) - 1, 3, 3, INK); rect(ctx, Math.round(x), Math.round(y), 1, 1, C.white);
    }
  }

  // ---- the pointer, in dots: a click acts, a drag moves furniture (or carries a bot: to its
  // door, to stop it)
  const dots = (e) => { const r = cv.getBoundingClientRect(); return { x: (e.clientX - r.left) / K, y: (e.clientY - r.top) / K }; };
  const inRect = (p, r) => p.x >= r[0] && p.x < r[2] && p.y >= r[1] && p.y < r[3];
  // a figure put down on a piece of furniture uses it: the armchair, a mat, the window, the coffee, a
  // plant, the chess board or the ping-pong table (a game for you), the meeting table, the printer
  function useAt(f, p) {
    if (!p) return null;
    const kind = KIND(p.key), long = rnd(40, 90);
    switch (kind) {
      case "armchair": return { kind: "chair", at: p.key, left: long };
      case "mat": return { kind: "sleep", mat: p.key, there: false };
      case "window": return { kind: "window", at: p.key, left: long / 3 };
      case "coffee": return { kind: "coffee", stage: 0, wait: 0 };
      case "plant": return { kind: "plant", at: p.key, wait: 0 };
      case "printer": return { kind: "print", stage: 0, wait: 0 };
      case "chess": if (f.human) app.act({ kind: "chess" }); else return { kind: "chess", side: Math.random() < 0.5 ? 0 : 1, with: null, left: long, there: false }; return null;
      case "pong": if (f.human) app.act({ kind: "pong" }); return null;
      case "meet": return { kind: "meeting", seat: Math.floor(Math.random() * 4), free: true };
      default: return null;
    }
  }
  function pieceAt(p) {
    if (!p || !lastL) return null;
    return lastL.pieces.find((q) => p.x >= q.r[0] && p.x < q.r[2] && p.y >= q.r[1] && p.y < q.r[3]) || null;
  }
  let press = null;
  cv.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    pointer = dots(e); draw();
    const h = [...hots].reverse().find((h) => over(h.x0, h.y0, h.x1, h.y1));
    press = { x: pointer.x, y: pointer.y, action: h && h.action, bot: hovered && !hovered.seat && hovered.b.id, piece: pieceAt(pointer), shift: e.shiftKey };
    cv.setPointerCapture(e.pointerId);
  });
  cv.addEventListener("pointermove", (e) => {
    pointer = dots(e);
    if (press && !moving && !held && Math.hypot(pointer.x - press.x, pointer.y - press.y) > 1) {
      const f = press.bot && world().folk.get(press.bot);
      if (f && f.b && !f.lying && f.act.kind !== "doom") { held = f; f.held = true; f.sat = false; f.lying = false; }
      else if (press.piece && !(press.bot && !f)) moving = { key: press.piece.key, from: press.piece.at, u: press.piece.u, at: press.piece.at };
    }
    if (moving) {
      const at = [moving.from[0] + pointer.x - press.x, moving.from[1] + (ON_WALL.has(moving.key) ? 0 : pointer.y - press.y)];
      moving.at = clampAt(moving.key, at, moving.u);         // on the grid of dots while it moves, as pair: it steps
    }
    if (held) {
      held.x = Math.max(0, Math.min(W, pointer.x)); held.y = Math.max(WALL, Math.min(H, pointer.y + 24)); held.dir = -1;
      app.carry(held.id, pointer.x >= W || pointer.x < 0 || pointer.y < 0 || pointer.y >= H ? e.clientX : null, e.clientY);
    }
    draw();
  });
  cv.addEventListener("pointerup", (e) => {
    if (moving) {
      const s = app.state(), d = lastL && lastL.desks.find((d) => d.key === moving.key);
      const burn = lastL && inFire(lastL, moving.key, moving.at, moving.u);
      if (burn && d && !d.seats.length) app.act({ kind: "forget", unit: d.unit });   // into the fire
      else if (burn && !d) app.act({ kind: "burn", key: moving.key });
      else {
        const at = clampAt(moving.key, moving.at, moving.u);       // put down on the grid of dots
        dropped.set(moving.key, { at, t: performance.now() });
        s.layout = { ...(s.layout || {}), [moving.key]: at };
        app.act({ kind: "layout", layout: s.layout });
      }
      moving = null;
    } else if (held) {
      const f = held; held = null; f.held = false;
      // dropped at its door: it waits there to be stopped; carried out of the room, onto the pane,
      // its session opens there; anywhere else it goes back to what it did
      const out = pointer && (pointer.x >= W || pointer.x < 0 || pointer.y < 0 || pointer.y >= H), bot = f.b && !f.b.human;
      const back = () => (f.work ? { kind: "desk" } : rest());
      if (out) { f.act = back(); if (bot) app.carried(f.id, e.clientX, e.clientY); }
      else if (bot && pointer && Math.abs(pointer.x - doorX(f.b.m)) < 11 && pointer.y < WALL + 14) f.act = { kind: "doom", where: "door", left: 5 };
      else if (bot && pointer && lastL && lastL.fire && inRect(pointer, [lastL.fire.x0, lastL.fire.y0, lastL.fire.x1, lastL.fire.y1])) f.act = { kind: "doom", where: "fire", left: 5 };
      else {
        const p = pointer && pieceAt(pointer);
        if (p && KIND(p.key) === "pair") { if (bot) app.act({ kind: "pair", key: p.key, id: f.id }); else app.act({ kind: "pairOpen", key: p.key }); f.act = back(); }
        else f.act = (pointer && !f.work && useAt(f, p)) || back();
      }
      f.legs = "stand";
    } else if (press && press.action && press.action.kind === "undo") { const f = world().folk.get(press.action.id); if (f && f.act.kind === "doom") f.act = f.work ? { kind: "desk" } : rest(); }
    else if (press && press.action) { if (press.action.kind === "report") world().print = performance.now(); app.act({ ...press.action, shift: press.shift }); }
    // a plain click on the floor or the wall, on nothing: what is open on the right goes away
    else if (press && !press.bot && !press.piece && pointer && Math.hypot(pointer.x - press.x, pointer.y - press.y) <= 1) app.act({ kind: "blank" });
    press = null;
    draw();
  });
  cv.addEventListener("pointerleave", () => { if (!press) { pointer = null; draw(); } });

  // a file dropped on a bot goes into its session
  cv.addEventListener("dragover", (e) => { e.preventDefault(); pointer = dots(e); draw(); });
  cv.addEventListener("drop", (e) => {
    e.preventDefault(); pointer = dots(e); draw();
    if (hovered && !hovered.b.human && e.dataTransfer.files.length) app.act({ kind: "drop", id: hovered.b.id, files: [...e.dataTransfer.files] });
  });

  // ---- size and run, twelve frames a second like a 16-bit game. The room is always W x H dots;
  // only how big a dot is follows the window: whole pixels, as big as fits
  function size() {
    const s = app.state();
    if (s && s.width && !widthSet) W = Math.max(200, Math.min(1200, Math.round(s.width)));
    H = Math.max(200, Math.floor((cv.parentElement.clientHeight - 2 * MARGIN) / K)); FLOORY = H - 6; defaults();
    if (cv.width !== W * 2 || cv.height !== H * 2) { cv.width = W * 2; cv.height = H * 2; }
    cv.style.width = W * K + "px"; cv.style.height = H * K + "px";
    cv.parentElement.style.width = W * K + MARGIN + "px";      // its margin on the left; the pane's tiles keep theirs
    draw();
  }
  // the divider dragged: the room this many pixels wide (whole dots)
  let widthSet = false;
  function width(px) {
    if (px === undefined) return W * K;
    widthSet = true; W = Math.max(200, Math.min(1200, Math.round(px / K)));
    size();
    return W;
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
  window.__room = { world, size: () => ({ W, H, K }), layout: () => lastL };   // for poking at it from the console
  // at(id): where a figure stands, in CSS pixels of the page (for look.mjs, to drive the room from outside)
  const at = (id) => { const f = world().folk.get(id), b = cv.getBoundingClientRect(); return f ? [b.left + f.x * K, b.top + (f.y - 12) * K] : null; };
  return { talk, typed, size, draw, width, color: projectColor, icon, carry: carryKind, place: placeKind, names: NAMES, at, hovered: () => hovered && { id: hovered.b.id, seat: !!hovered.seat } };
}
