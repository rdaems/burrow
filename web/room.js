// The room: every machine's bots in one place, as pair's office (rdaems/pair-prompting, app/floor.js): its
// breaks, its robots (pixel.js), furnished as pixel art (sprites.js). Every project (a working
// directory, here or on another machine) is a desk: a cabinet with the project's screen and name at its left
// end, a long desktop to the right where its bots sit facing the screen, a chair a bot. The desk is
// handled at its screen: hover it for the path, the + beside it starts another bot there. The desks are
// the ones you put down (the project cabinet) and stay until you burn one. A bot in a
// git worktree of the project sits at the project's desk, its own copy of the files a laptop on the
// cabinet, plugged in.
// The bots are the sessions `claude agents --all` lists whose directory has a desk (the rest are not in
// the room; the cabinet's form lists their directories), and what a bot does is its tempo (Claude Code's
// record of each session: is a turn running), not its state (Claude Code's reading of its last words):
//   a turn runs         at its project's desk, typing
//   a dialog open       at its desk with its hand up and a ? in a bubble (a multiple-choice question, a
//                       permission prompt), its screen's cursor blinking
//   idle                by its desk, or on a break: coffee, the printer, the cooler, ping-pong,
//                       chess, the plants, the window, the armchair, the bench, a meeting at the
//                       whiteboard, a visit, a nap on a mat it rolls out, warming its hands at the fire,
//                       watching a game, a stretch, a coffee for a busy colleague. Whatever its state:
//                       done, completed long ago, its process stopped after an hour, a last message
//                       that reads as a question; opening one restarts it
//   sends a message     a beam from its antenna to whoever it wrote to
//   deleted             gone from the list (the fire, `claude rm`): up in flames
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
// empty door frame for a machine; drag furniture where you want it; carry a bot into the booth (gently,
// with five seconds to undo) or the fire to delete it (a desk or a piece of furniture burns there too), to
// the neuralyzer to `/clear` its conversation; drop a file on a bot to
// hand it over.
//
// startRoom(canvas, app) with app = { state(), open(), opens(), chess(), pong(), away(), hl(), hover(id), act(action) }
// (open: the bot whose tile has the focus, opens: every bot with a tile, hl: the one whose tile the pointer is on,
// carry(id, clientX|null, clientY) / carried(id, clientX, clientY): a bot held out over the pane, and let go there);
// returns { talk(from, to), typed(), size(), draw(), width(px?), color(unit) }. The room is W x H dots: W as the
// divider was dragged (room.json), H as tall as the window.

function startRoom(cv, app) {
  const C = pixelAvatar.C, INK = C.ink, INK2 = C.ink2;
  const TERRACOTTA = "#d97757", RED = "#ff6b5a", OPEN = "#8a4a3a", YOU = TERRACOTTA;   // your clothes: pair's "me"
  const GOLD = "#f6c784";                     // what the pointer is on: the tiles' pastel gold (burrow.css --pale0), on the floor as well; terracotta stays yours
  const SP = sprites.PAL, WOOD = { hi: SP.h, top: SP.w, d: SP.d, k: SP.k, plate: "#2e1f28" }, SHADOW = SP.s;
  // the paint of another machine's door and desks, in the order of Claude Code's sshConfigs
  const PAINT = [
    { hi: "#8fd0c4", top: "#5fa89c", d: "#3f7d74", k: "#1f3f42", plate: "#142a2c" },
    { hi: "#f4f1ea", top: "#d8d2c6", d: "#a59d90", k: "#4a4250", plate: "#2a2530" },
    { hi: "#a3c0e4", top: "#7397c4", d: "#4f6e9a", k: "#263652", plate: "#182236" },
    { hi: "#e8a493", top: "#c97766", d: "#964c40", k: "#4f2430", plate: "#2e151c" },
    { hi: "#f2d27a", top: "#d6aa45", d: "#a07c2c", k: "#4f3a1e", plate: "#2e2214" },   // mustard
    { hi: "#c9b3e6", top: "#a185c9", d: "#76599e", k: "#3a2a52", plate: "#221830" },   // lilac
    { hi: "#b9d39a", top: "#8fae6c", d: "#62804a", k: "#2f4027", plate: "#1b2617" },   // sage
    { hi: "#8a93a3", top: "#646c7b", d: "#454b57", k: "#22262e", plate: "#16181d" },   // charcoal
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
  // a machine's door: the first is this one's; its bots come in and go out at its middle, wherever it was
  // slid along the wall. Its paint (and its desks'): the one picked in the doors' panel (room.json `paint`,
  // -1 the wood), else this machine wood and the others PAINT in their order
  const doorOf = (mid) => Math.max(0, app.state().machines.findIndex((m) => m.id === (mid || "")));
  const doorX = (mid) => { const d = lastL && lastL.doors.find((d) => d.mid === (mid || "")); return d ? d.x0 + 8 : 10 + doorOf(mid) * DOOR; };
  const paintIdx = (mid) => { const o = (app.state().paint || {})[mid || ""]; return Number.isInteger(o) ? o : doorOf(mid) - 1; };
  const paintOf = (mid) => { const i = paintIdx(mid); return i < 0 ? WOOD : PAINT[i % PAINT.length]; };
  const wallFrom = () => 2;                 // what hangs on the wall: anywhere along it (it settles off the others)

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
  // femme: the office manager, a robot with a bow on her head and a skirt
  function body(seed, color, arms, legs, human, femme) {
    const f = face(seed, color, human);
    if (!f.px) return null;
    const key = seed + color + arms + legs + (human ? "h" : "") + (femme ? "f" : "");
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
    if (femme) {
      let ht = 0;
      while (ht < cut && !at(7, ht)) ht++;            // the top of her head (her antenna stands further right)
      const bow = color, knot = shade(color, 0.62);
      rect(4, ht - 3, 2, 3, bow); rect(7, ht - 3, 2, 3, bow); set(6, ht - 2, knot);
      rect(5, 18, 10, 3, bow); rect(5, 20, 10, 1, knot);   // the skirt
    }
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
      case "gun": back(); fwd(19, 1); rect(13, 18, 3, 2, "#7a4a2a"); rect(15, 17, 6, 1, "#a7aebb"); rect(15, 18, 5, 1, "#6c7380"); break;   // a shotgun, levelled
      case "paper": back(); fwd(19, 2); rect(16, 15, 4, 5, C.steel4); rect(17, 16, 2, 1, C.steel2); rect(17, 18, 2, 1, C.steel2); break;
      case "book": back(); fwd(19, 2); rect(16, 16, 3, 4, C.red1); rect(16, 16, 1, 4, C.red0); break;
      case "can": back(); fwd(19, 2); rect(16, 17, 4, 3, C.green0); rect(20, 17, 1, 1, C.green0); break;
      case "roll": back(); fwd(19, 2); rect(16, 15, 4, 5, MAT.top); rect(16, 15, 4, 1, MAT.hi); rect(17, 16, 2, 3, MAT.d); break;   // a rolled-up mat
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
  const colorOf = (b) => b.staff ? b.color : b.human ? (b.color || YOU) : projectColor(b.unit);
  const seedOf = (b) => b.staff ? b.seed : b.human ? (b.seed || b.name) : b.id;

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
      plant0: [W - 8, FLOORY], plant1: [W - 12, 64], board: [W / 2 + 46, WALL + 34], window: [W - 40, 4], clock: [W - 15, 5], fire: [W - 92, FLOORY],
      closet: [W - 72, 2], settings0: [W - 92, 5],
    };
  }
  // The furniture is what the server lists (room.json `furniture`): keys, a kind and maybe a number
  // (plant0, window1). The closet adds a piece, the fireplace burns one; the fire, the closet and the
  // switchboard (burrow's settings) stay. A machine's door is a piece too (`door:<machine id>`, not in the
  // list): it hangs on the wall like the window, comes off it while carried, and in the fire it takes its
  // machine out of Claude Code's sshConfigs (as the doors' panel does). A new one comes from the closet.
  // This computer's door (`door:`) only slides along the wall
  const KIND = (key) => key.startsWith("door:") ? "door" : key.replace(/\d+$/, "");
  const FIXED = new Set(["fire", "closet", "settings"]), fixed = (key) => FIXED.has(KIND(key)) || key === "door:";
  const CLICK = new Set(["add", "closet", "chess", "pong", "printer", "pair", "board", "coffee", "cooler", "window", "bench", "armchair", "settings", "clock", "plant", "manager", "door"]);   // a click does something there
  const NAMES = { meet: "meeting table", pong: "ping-pong table", chess: "chess table", coffee: "coffee machine", cooler: "water cooler",
    printer: "printer", armchair: "armchair", plant: "plant", board: "whiteboard", window: "window", clock: "clock", add: "project cabinet", pair: "pair room", bench: "bench", settings: "switchboard", manager: "office manager's desk",
    booth: "booth", neuralyzer: "neuralyzer", door: "door" };
  // the bench seats three, side by side, idle bots on a break (a click on it lists the completed sessions)
  const BENCH_SEATS = 3, benchSeat = (bn, k) => ({ x: bn.x - 14 + k * 14, y: bn.y });
  // a clear spot on the floor for a figure that has no chair when the page opens (spread by its id, the same each time)
  function freeAt(L, id) {
    let h = fnv(id);
    for (let k = 0; k < 60; k++) {
      h = (Math.imul(h, 1103515245) + 12345) >>> 0;
      const x = L.floor.x0 + (h % (L.floor.x1 - L.floor.x0)), y = L.floor.y0 + 20 + ((h >>> 11) % Math.max(1, L.floor.y1 - L.floor.y0 - 20));
      if (L.free(x, y)) return { x, y };
    }
    return { x: W / 2, y: H / 2 };
  }
  const benchFree = (bn, f) => [...Array(BENCH_SEATS).keys()].filter((k) => ![...world().folk.values()].some((g) => g !== f && g.act.kind === "bench" && g.act.at === bn.key && g.act.seat === k)).map((seat) => ({ at: bn.key, seat }));
  const FIRE = { w: 48, h: 42, S: 1.5 };     // the fire pit's footprint, drawn to scale from its 32 x 28 original
  // the default places for desks: columns of them, as many as the room is wide
  const SLOTS = () => [104, 150, 196, 242, 288, 334].flatMap((y) => [...Array(Math.max(1, Math.floor((W - 10) / 124)))].map((_, i) => [10 + i * 124, y]));
  const ON_WALL = new Set(["closet", "window", "clock", "settings", "door"]), WALL_Y = { closet: 2, window: 4, clock: 5, settings: 5, door: WALL - 20 };
  const onWall = (key) => ON_WALL.has(KIND(key));
  // what each piece covers, from where it stands
  function foot(key, [x, y], u) {
    if (u) return [x - 2, y - 35, x + u.w + 2, y + 2];
    switch (KIND(key)) {
      case "add": return [x - 2, y - 16, x + ADD_W + 2, y + 2];
      case "meet": return [x - 28, y - 20, x + 28, y + 4];
      case "pong": return [x - 38, y - 23, x + 38, y + 8];
      case "chess": return [x - 19, y - 18, x + 19, y + 2];
      case "coffee": return [x, y - 22, x + 20, y + 1];
      case "cooler": return [x - 5, y - 26, x + 5, y + 1];
      case "printer": return [x - 7, y - 15, x + 7, y + 1];
      case "armchair": return [x - 6, y - 23, x + 18, y + 1];
      case "board": return [x - 16, y - 26, x + 16, y + 1];
      case "window": return [x, y, x + 21, y + 18];
      case "clock": return [x, y, x + 12, y + 10];
      case "fire": return [x, y - FIRE.h, x + FIRE.w, y + 1];
      case "closet": return [x, y, x + 22, y + 24];
      case "settings": return [x, y, x + SB_W(), y + SB_H()];
      case "manager": return [x - 2, y - 17, x + 30, y + 2];
      case "pair": return [x - 13, y - 20, x + 13, y + 1];
      case "bench": return [x - 29, y - 18, x + 29, y + 2];
      case "booth": return [x - 9, y - 32, x + 9, y + 2];
      case "door": return [x, y, x + 16, y + 20];
      case "neuralyzer": return [x - 7, y - 19, x + 7, y + 2];
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
  // a desk's bots, in the order they came: a new one (or one whose process comes back) takes the chair at
  // the far end of the row, the others keep theirs; when the page opens, in the order they started
  const seatRows = new Map();                 // unit → ids, chair by chair from the screen
  const sitters = (bots, d) => {
    const here = new Map(bots.filter((b) => b.unit === d.unit).map((b) => [b.id, b]));
    const row = (seatRows.get(d.unit) || []).filter((id) => here.has(id));
    for (const id of here.keys()) if (!row.includes(id)) row.push(id);
    seatRows.set(d.unit, row);
    return row.map((id) => here.get(id));
  };
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
    const keys = (s.furniture || Object.keys(DEF)).filter((k) => KIND(k) !== "mat"), at0 = {};   // the charging mats of older rooms (a server from before drops them)
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
    // along the wall: the doors (one a machine, in their order unless slid elsewhere) and what hangs there;
    // one without a place yet gets the stretch of wall nearest its default that nothing else covers
    const doorKeys = (s.machines || []).map((m) => "door:" + m.id), onTheWall = [];
    const wallSpot = (k, x0) => {
      const y = WALL_Y[KIND(k)], fits = (x) => { const r = foot(k, [x, y]); return r[0] >= 2 && r[2] <= W - 2 && !onTheWall.some((t) => meets(t, r)); };
      for (let d = 0; d < W; d += 2) for (const x of [x0 + d, x0 - d]) if (fits(x)) return [x, y];
      return [x0, y];
    };
    for (const k of doorKeys) { at0[k] = where(k); if (at0[k]) onTheWall.push(foot(k, at0[k])); }
    for (const k of keys) if (at0[k] && onWall(k)) onTheWall.push(foot(k, at0[k]));
    doorKeys.forEach((k, i) => { if (!at0[k]) { at0[k] = wallSpot(k, 2 + i * DOOR); onTheWall.push(foot(k, at0[k])); placed[k] = at0[k]; } });
    for (const k of keys) if (!at0[k]) {      // a piece without a place yet
      at0[k] = onWall(k) ? wallSpot(k, 2 + doorKeys.length * DOOR) : free(k);
      if (onWall(k)) onTheWall.push(foot(k, at0[k]));
      placed[k] = at0[k];
    }

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
    const b0 = one("board"), c0 = one("clock"), f0 = one("fire"), cl = one("closet"), a0 = one("add"), sw = one("settings");
    const L = {
      desks, add: a0 && { key: a0.key, x0: a0.x, y0: a0.y - 15, x1: a0.x + ADD_W, y1: a0.y },
      floorY: FLOORY, board: b0 && { key: b0.key, x: b0.x, y: b0.y, x0: b0.x - 16, y0: b0.y - 26, x1: b0.x + 16, y1: b0.y }, clock: c0 && { key: c0.key, x0: c0.x, y0: c0.y },
      windows: all("window").map((p) => ({ key: p.key, x0: p.x, y0: p.y, x1: p.x + 21, y1: p.y + 18 })),
      fire: f0 && { key: f0.key, x0: f0.x, y0: f0.y - FIRE.h, x1: f0.x + FIRE.w, y1: f0.y }, closet: cl && { key: cl.key, x0: cl.x, y0: cl.y, x1: cl.x + 22, y1: cl.y + 24 },
      settings: sw && { key: sw.key, x0: sw.x, y0: sw.y, x1: sw.x + SB_W(), y1: sw.y + SB_H() },
      manager: one("manager"), meet: one("meet"), pong: one("pong"), chess: one("chess"), pairs: all("pair"), benches: all("bench"), booths: all("booth"), neuralyzers: all("neuralyzer"),
      cooler: one("cooler"), plants: all("plant"), printer: one("printer"), chairs: all("armchair"),
      coffeeAt: one("coffee"), floor: { x0: 14, y0: WALL + 8, x1: W - 14, y1: FLOORY - 4 }, placed,
    };
    L.coffee = L.coffeeAt && { x: L.coffeeAt.x + 27, y: L.coffeeAt.y - 10 };
    L.doors = (s.machines || []).map((m, i) => ({ key: doorKeys[i], mid: m.id, m, x0: at0[doorKeys[i]][0], y0: at0[doorKeys[i]][1] }));
    // what can be picked up and moved, front to back
    L.pieces = [
      ...desks.map((d) => ({ key: d.key, at: d.at, u: d.u, r: foot(d.key, d.at, d.u) })),
      ...keys.map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
      ...doorKeys.map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
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
    if (!everyone) everyone = { condemned: new Map(), razed: new Set(), hunt: null, folk: new Map(), meeting: null, lastMeeting: -60, board: [], pong: { t: 0, from: 0 }, doorAt: {}, stopped: new Map(), flare: 0, sparks: 0, boothFlash: null, flash: null, shots: [], spared: new Set(), clean: null, cleanRest: 0, print: 0, burning: [], strokes: 0, stats: new Map(), meetings: 0,
      remains: new Map(), riseAt: new Map(), sway: new Map(), ball: null, mugs: new Map(),
      beams: [], T: 0, born: performance.now() };
    return everyone;
  }
  // the chess board as pair keeps it: 64 squares from a1, upper case white, "." empty
  const START = "RNBQKBNRPPPPPPPP" + ".".repeat(32) + "pppppppprnbqkbnr";
  const newBoard = () => START;
  const rest = (r = Math.random()) => ({ kind: "stand", left: 2 + r * 5 });   // a pause, before the next break
  function end(f) { f.lying = false; f.carry = null; f.act = rest(0.06); }
  function wake(f) { f.lying = false; f.legs = "stand"; f.carry = "roll"; f.act = { kind: "stand", left: 1.5 }; }   // up, the mat rolled up under its arm
  // a clear bit of floor for a mat: inside the floor, off every piece (with a margin), off the other mats
  const MATR = (x, y) => [x - 17, y - 18, x + 17, y + 1];
  function napClear(L, x, y, others) {
    const r = MATR(x, y);
    if (r[0] < L.floor.x0 - 6 || r[2] > L.floor.x1 + 6 || r[1] < WALL + 6 || r[3] > L.floor.y1 + 4) return false;
    if (L.pieces.some((p) => !onWall(p.key) && meets(r, [p.r[0] - 4, p.r[1] - 4, p.r[2] + 4, p.r[3] + 6]))) return false;
    return !others.some((g) => g.act.kind === "sleep" && g.act.at && meets(r, MATR(g.act.at.x, g.act.at.y)));
  }
  // the nearest clear spot to where the figure is, looking outwards in rings
  function napSpot(f, L, others) {
    for (const d of [0, 14, 28, 44, 62, 84, 110, 140, 180]) for (let k = 0; k < 8; k++) {
      const a = (k / 8) * 2 * Math.PI + f.z, x = Math.round(f.x + Math.cos(a) * d), y = Math.round(f.y + Math.sin(a) * d * 0.7);
      if (napClear(L, x, y, others)) return { x, y };
    }
    return null;
  }
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
    stat(w, from).sent++; stat(w, to).got++;
    w.beams.push({ from, to, t: 0 });
    f.chat = Math.max(f.chat, 1);
  }
  // you typed to the open session: a bubble over your head
  function typed() {
    const f = world().folk.get("you");
    if (f) f.chat = 2;
  }

  function think(w, bots, L, dt, usage, game) {
    const now = performance.now() / 1000, folk = w.folk;
    // the bot you play chess with sits at the table until the game is over (or it gets work)
    const vs = game && !game.result ? (game.white.id === "you" ? game.black.id : game.white.id) : null;
    for (const f of folk.values()) {
      if (f.id === vs && !(f.act.kind === "chess" && f.act.you) && !f.work) { end(f); f.act = { kind: "chess", side: game.white.id === vs ? 0 : 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "chess" && !f.human && (f.act.you ? f.id !== vs : !!vs)) end(f);   // the table is taken: the others get up
    }
    // a move on the board (yours against a bot, or the bots' own game): the one who moved reaches out
    const sg = app.state().chess;
    if (sg && sg.sans.length !== w.plies && w.plies !== undefined) { w.chessAt = performance.now(); const mover = folk.get(sg.toMove === sg.white.id ? sg.black.id : sg.white.id); if (mover) mover.moved = 0.9; }
    w.plies = sg ? sg.sans.length : undefined;
    // the bot you play ping-pong with stands at the table, at the far end
    const pp = app.pong(), pv = pp && !pp.watch && pp.id;   // the bot you play (none while you watch two)
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
      const f = folk.get(b.id), work = b.waiting || b.busy;
      if (!f) continue;
      f.chat = Math.max(0, f.chat - dt);
      f.busy = b.busy; f.waiting = b.waiting;
      // a bot whose session is open sits at its desk for you, whatever it was doing; when the
      // session closes it gets up again
      f.open = opens.includes(b.id);
      if (f.open && !work && f.act.kind !== "desk" && !DOOMED.has(f.act.kind)) { end(f); f.act = { kind: "desk" }; }
      if (!f.open && !work && f.act.kind === "desk" && f.sat) f.act = { kind: "stand", left: 0.5 };
      if (work !== f.work) {
        f.work = work;
        // to work, whatever it was doing (a bot by the door stays)
        if (work && !DOOMED.has(f.act.kind)) { end(f); f.act = { kind: "desk" }; }
        if (!work && f.act.kind === "desk") f.act = { kind: "stand", left: 0.5 + (fnv(b.id) % 20) / 10 };
      }
    }
    // remains on the floor: idle bots come over and clear them away, two to a heap, once the pieces have
    // had a moment to land (whoever is on an ordinary break; not one at work, in a game, asleep or held)
    stepRemains(w, dt, L);
    condemnStep(w, L);
    for (const [id, r] of w.remains) {
      if (r.back || !r.parts.length || performance.now() - r.t0 < 1500) continue;
      if ([...folk.values()].filter((g) => g.act.kind === "sweep" && g.act.at === id).length >= 2) continue;
      const crew = [...folk.values()].filter((g) => g.b && !g.b.human && !g.b.staff && !g.work && !g.held && !g.lying && !g.open && SWEEPERS.has(g.act.kind))
        .sort((a, b) => Math.hypot(a.x - r.x, a.y - r.y) - Math.hypot(b.x - r.x, b.y - r.y))[0];
      if (crew) { end(crew); crew.act = { kind: "sweep", at: id, wait: 0 }; }
    }
    if (w.meeting && (w.meeting.left -= dt) <= 0) {
      w.meeting = null;
      for (const f of folk.values()) if (f.act.kind === "meeting" || f.act.kind === "present") end(f);
    }
    const playing = (kind) => [...folk.values()].filter((f) => f.act.kind === kind && f.act.there && f.act.with).length === 2;
    // the ball goes back and forth while two are at the table
    if ((playing("pong") || (app.pong() && app.pong().ball)) && (w.pong.t += dt / 0.8) >= 1) { w.pong.t = 0; w.pong.from = 1 - w.pong.from; }
    // the football, if one is in play
    kickabout(w, dt);
    // an idle bot's place: at the end of its project's desk, by the empty chair, one per project
    // (the first idle one); the others are off on a break
    const homes = new Map();
    for (const d of L.desks) {
      const first = d.seats.find(([id]) => { const f = folk.get(id); return f && !f.work && f.act.kind === "stand"; });
      if (first) homes.set(first[0], { x: Math.min(W - 8, d.x0 + d.w + 8), y: d.y + 1 });
    }
    for (const b of bots) { const f = folk.get(b.id); if (f && !f.held) thinkOne(w, f, L, now, dt, homes.get(b.id), usage); }
    if (you && !you.held && (fps || !huntStep(w, you, L, dt))) thinkOne(w, you, L, now, dt, null, usage);   // (in the first person the shooting is yours)
    const mg = folk.get("manager");
    if (mg && !mg.leaving) thinkManager(w, mg, L, dt);
    // two standing about in the same place: they step apart
    const loit = [...folk.values()].filter((f) => f.legs === "stand" && f.act.kind === "loiter");
    for (const a of loit) for (const b of loit) if (a.id < b.id && Math.abs(a.x - b.x) < 12 && Math.abs(a.y - b.y) < 4) {
      const d = a.x <= b.x ? -1 : 1; a.x += d * 8 * dt; b.x -= d * 8 * dt;
    }
  }
  // the office manager keeps the notes on the sessions (server.py has Claude write them): while one is
  // written up she stands by that bot, a sheet in hand, writing (by that desk's screen for a desk's
  // paragraph); otherwise she sits at her desk with the stack of notes, filing now and then. With the
  // notes switched off at the switchboard she dozes in her chair
  const MANAGER = { seed: "manager-16", color: "#f06b72" };   // a robot: one antenna, happy eyes; a pink bow and skirt (body's femme)
  const managerSeat = (L) => L.manager ? { x: L.manager.x + 24, y: L.manager.y } : { x: W / 2, y: H / 2 };
  // a bot let go of from here (its desk burnt, or ✕✕ in the completed list) is not just deleted: it walks to
  // the booth and waits in line, then its five seconds inside as for one put in by hand; in hardcore it stands
  // with its hands up and you fetch a gun from the gun closet and shoot it (its remains are the undo, until
  // they are cleared). With no booth it walks into the fire. Taking one out of the line, or its undo, keeps it,
  // and a burnt desk with it: the desk goes (`forget`) only once the last of its bots has
  const DOOMED = new Set(["doom", "queue", "condemned", "pyre"]);
  function condemn(ids, unit = null) {
    const w = world(), took = [];
    for (const id of ids) {
      const f = w.folk.get(id);
      if (!f || !f.b || f.b.human || f.b.staff || f.held || DOOMED.has(f.act.kind) || w.remains.has(id)) continue;
      end(f); f.act = { kind: "queue", n: 99 };
      w.condemned.set(id, { unit }); w.spared.delete(id); took.push(id);
    }
    if (unit && took.length) w.razed.add(unit);
    return took;
  }
  function condemnStep(w, L) {
    const folk = w.folk, bo = L.booths[0], hard = hardcore() && !!bo;
    for (const [id, c] of w.condemned) {
      const f = folk.get(id);
      if (w.spared.has(id) || (f && !f.held && !DOOMED.has(f.act.kind))) { w.condemned.delete(id); if (c.unit) w.razed.delete(c.unit); }   // kept: and its desk
      else if (!f && !w.remains.has(id)) w.condemned.delete(id);                                                                       // gone for good
    }
    for (const u of w.razed) if (![...w.condemned.values()].some((c) => c.unit === u)) { w.razed.delete(u); app.act({ kind: "forget", unit: u }); }
    const line = [...w.condemned.keys()].map((id) => folk.get(id)).filter((f) => f && !f.held && ["queue", "condemned"].includes(f.act.kind));
    if (hard) { for (const f of line) if (f.act.kind !== "condemned") f.act = { kind: "condemned" }; return; }
    if (!bo) { for (const f of line) f.act = { kind: "pyre" }; return; }
    line.forEach((f, n) => { if (f.act.kind !== "queue") f.act = { kind: "queue", n }; else f.act.n = n; });
    if (line[0] && line[0].act.there && ![...folk.values()].some((g) => g.act.kind === "doom" && g.act.at === bo.key)) line[0].act = { kind: "doom", at: bo.key, left: 5 };
  }
  // you, in hardcore: a gun from the gun closet, up to each of them, levelled a moment, the shot; the gun back
  function huntStep(w, f, L, dt) {
    const bo = L.booths[0], hard = hardcore() && !!bo, v = 34;
    const targets = () => [...w.condemned.keys()].map((id) => w.folk.get(id)).filter((g) => g && !g.held && g.act.kind === "condemned");
    let h = w.hunt;
    if (!h) { if (!hard || !targets().length) return false; h = w.hunt = { phase: "arm", id: null, t: 0 }; end(f); }
    const front = bo && { x: bo.x + 14, y: bo.y };
    f.sat = false; f.lying = false; f.act = { kind: "hunt", chosen: true };
    if (!hard && h.phase !== "return") h.phase = f.carry === "gun" ? "return" : "done";
    switch (h.phase) {
      case "arm":
        if (walk(f, front, v, dt)) { f.dir = -1; f.carry = "gun"; h.phase = "hunt"; }
        return true;
      case "hunt": {                         // the nearest, from its right (or its left by the wall)
        const g = targets().sort((a, b) => Math.hypot(a.x - f.x, a.y - f.y) - Math.hypot(b.x - f.x, b.y - f.y))[0];
        if (!g) { h.phase = "return"; return true; }
        h.id = g.id;
        if (walk(f, { x: g.x + (g.x + 24 <= L.floor.x1 ? 24 : -24), y: g.y }, v, dt)) { f.legs = "stand"; h.phase = "aim"; h.t = 0; }
        return true;
      }
      case "aim": {
        const g = w.folk.get(h.id);
        if (!g || g.held || g.act.kind !== "condemned") { h.phase = "hunt"; return true; }
        f.legs = "stand"; f.arms = "gun"; f.dir = g.x < f.x ? -1 : 1;
        if ((h.t += dt) > 0.9) { w.shots.push({ x: Math.round(f.x) + 10 * f.dir, y: Math.round(f.y) - 9, t0: performance.now(), hit: false }); shoot(g); h.phase = "after"; h.t = 0; }
        return true;
      }
      case "after":
        f.legs = "stand"; f.arms = "gun";
        if ((h.t += dt) > 1.5) h.phase = targets().length ? "hunt" : "return";
        return true;
      case "return":
        if (!front || walk(f, front, v, dt)) { f.carry = null; h.phase = "done"; }
        return true;
    }
    w.hunt = null; f.carry = null; f.act = rest();
    return false;
  }
  // clearing out (switchboard: cleanup): one finished session at a time (app.cleanup(): completed, nothing
  // open in her notes, an hour gone; never one you rescued). She fetches it and walks it to the booth, a step
  // behind her, where its five seconds run as for any bot put in. Hardcore: a shotgun from the gun closet,
  // up to it, levelled, the shot (its remains, the undo until they are cleared), the next one, the gun back. Taking
  // one from her, or its undo, spares it for good. A pause between them, so it can be watched
  function cleanStep(w, f, L, dt) {
    const cu = app.cleanup(), bo = L.booths[0];
    const next = () => cu.ids.find((id) => { const g = w.folk.get(id); return g && !w.spared.has(id) && !g.held && !DOOMED.has(g.act.kind) && g.act.kind !== "escort"; });
    let c = w.clean;
    const finish = (pause) => {
      const g = c && w.folk.get(c.id);
      if (g && g.act.kind === "escort") g.act = rest();
      if (c && c.armed) f.carry = null;
      w.clean = null; w.cleanRest = pause;
      return false;
    };
    if (!cu.on || !bo || (c && c.hard !== cu.hardcore)) return c ? finish(5) : false;
    if (!c) {
      if ((w.cleanRest -= dt) > 0) return false;
      const id = next();
      if (!id) return false;
      c = w.clean = { id, hard: cu.hardcore, phase: cu.hardcore ? "arm" : "fetch", t: 0 };
      f.carry = null;                        // her notes stay at her desk
    }
    const g = w.folk.get(c.id), front = { x: bo.x + 14, y: bo.y };
    f.sat = false; f.act = { kind: "clean" };
    switch (c.phase) {
      case "fetch":                          // to the bot
        if (!g || g.held || !cu.ids.includes(c.id)) return finish(3);
        if (walk(f, { x: g.x + 10, y: g.y }, 26, dt)) { f.dir = -1; g.act = { kind: "escort" }; c.phase = "walk"; }
        return true;
      case "walk":                           // to the booth, the bot behind her
        if (!g || g.held || g.act.kind !== "escort") { w.spared.add(c.id); return finish(3); }
        if (walk(f, front, 22, dt)) { f.dir = -1; f.legs = "stand"; g.act = { kind: "doom", at: bo.key, left: 5 }; c.phase = "booth"; }
        return true;
      case "booth":                          // beside it while its seconds run
        f.legs = "stand"; f.dir = -1; f.arms = "down";
        if (!g) return finish(8);
        if (g.act.kind !== "doom") { w.spared.add(c.id); return finish(5); }
        return true;
      case "arm":                            // hardcore: a shotgun from the gun closet
        if (walk(f, front, 30, dt)) { f.dir = -1; c.armed = true; f.carry = "gun"; c.phase = "hunt"; }
        return true;
      case "hunt":                           // up to the bot, to the right of it
        if (!g || g.held || w.remains.has(c.id)) { const id = next(); if (id) c.id = id; else c.phase = "return"; return true; }
        if (walk(f, { x: Math.min(L.floor.x1, g.x + 24), y: g.y }, 30, dt)) { f.legs = "stand"; c.phase = "aim"; c.t = 0; }
        return true;
      case "aim":                            // levelled a moment, then the shot (the flash at the barrel)
        if (!g || g.held) { c.phase = "hunt"; return true; }
        f.legs = "stand"; f.arms = "gun"; f.dir = g.x < f.x ? -1 : 1;
        if ((c.t += dt) > 0.9) { w.shots.push({ x: Math.round(f.x) + 10 * f.dir, y: Math.round(f.y) - 9, t0: performance.now(), hit: false }); shoot(g); c.phase = "after"; c.t = 0; }
        return true;
      case "after":                          // the next one, or done
        f.legs = "stand"; f.arms = "gun";
        if ((c.t += dt) > 2.5) { const id = next(); if (id) { c.id = id; c.phase = "hunt"; } else c.phase = "return"; }
        return true;
      case "return":                         // the gun back in the closet
        if (walk(f, front, 30, dt)) return finish(20);
        return true;
    }
    return finish(5);
  }
  function thinkManager(w, f, L, dt) {
    const dg = app.digest(), wr = dg && dg.on ? dg.writing || dg.explaining : null, v = 30;   // writing notes, or explaining a decision
    if ((w.clean || !wr) && cleanStep(w, f, L, dt)) return;
    let to = null, look = null;
    if (wr && wr.startsWith("u:")) {
      const d = L.desks.find((x) => x.unit === wr.slice(2));
      if (d) { const { sx } = screenAt(d); to = { x: sx + 4, y: d.y + 13 }; look = sx + 11; }
    } else if (wr) {
      const g = w.folk.get(wr);
      if (g && !g.held && !g.leaving) {
        const at = g.legs === "sit" && !g.lying ? L.seat(wr) || g : g;
        to = g.lying ? { x: g.x + 20, y: g.y + 1 } : { x: Math.round(at.x) - 8, y: Math.round(at.y) + 13 };
        look = g.x;
      }
    }
    if (to) {
      to.x = Math.max(L.floor.x0, Math.min(L.floor.x1, to.x)); to.y = Math.max(L.floor.y0, Math.min(L.floor.y1, to.y));
      f.sat = false; f.carry = "paper";
      const there = walk(f, to, v, dt);
      f.act = { kind: "notes", there };
      if (there) { f.legs = "stand"; f.dir = look < f.x ? -1 : 1; f.arms = Math.floor(w.T * 2) % 3 ? "paper" : "fwd2"; }
      return;
    }
    f.carry = null;
    if (!sitAt(f, managerSeat(L), v, dt)) { f.act = { kind: "file" }; return; }
    f.dir = -1; f.legs = "sit";
    const off = !!dg && !dg.on;
    f.act = { kind: off ? "doze" : "file", there: true };
    f.arms = off ? "down" : wr ? (Math.floor(w.T * 3) % 2 ? "fwd1" : "fwd2") : Math.floor(w.T / 4) % 5 === 0 ? (Math.floor(w.T * 2) % 2 ? "fwd1" : "fwd2") : "down";
  }

  // what everyone has been up to since the page opened, for the printer's report: acts begun, by
  // kind; seconds at work, idle, asleep; messages sent and got
  const stat = (w, id) => w.stats.get(id) || (w.stats.set(id, { busy: 0, idle: 0, asleep: 0, sent: 0, got: 0 }), w.stats.get(id));
  function thinkOne(w, f, L, now, dt, home, usage) {
    const folk = w.folk, others = [...folk.values()].filter((g) => g !== f && !g.leaving), v = 22;
    const seat = L.seat(f.id), st = stat(w, f.id);
    if (f.act.kind !== f.lastKind) { f.lastKind = f.act.kind; st[f.act.kind] = (st[f.act.kind] || 0) + 1; }
    if (f.work) st.busy += dt; else st.idle += dt;
    if (f.act.kind === "sleep" && f.act.there) st.asleep += dt;
    // you sleep on a mat once the window has been left alone a minute, as pair's people who are away. A
    // bot naps only as a break of its own: whether its process runs does not show (opening one restarts it)
    if (f.human && fps) { if (f.act.kind !== "fps") { end(f); f.act = { kind: "fps" }; } }   // you are where the view says (move)
    const sleepy = f.human && !fps && app.away() > 60;
    if (sleepy && f.act.kind !== "sleep") { end(f); f.act = { kind: "sleep", there: false }; }
    if (!sleepy && f.act.kind === "sleep" && f.act.left === undefined) wake(f);   // you are back (a nap of its own ends by itself)
    // you: with the bot whose session is open, at the table of the game you play; else about the
    // room like the others
    if (f.human && !sleepy && !fps) {
      const o = app.open(), g = o && folk.get(o), game = app.chess(), pp = app.pong(), sh = app.shell(), a = f.act;
      if (game) { if (!(a.kind === "chess" && a.you)) { end(f); f.act = { kind: "chess", side: game.white.id === "you" ? 0 : 1, you: true, left: 1e9, there: false }; } }
      else if (pp) { if (!(a.kind === "pong" && a.you)) { end(f); f.act = { kind: "pong", side: 0, you: true, left: 1e9, there: false }; } }
      else if (g) { if (!(a.kind === "with" && a.to === g.id) && !a.chosen) { end(f); f.act = { kind: "with", to: g.id }; } }      // (a seat you clicked yourself keeps you)
      else if (sh) { if (!(a.kind === "screen" && a.m === sh.m && a.dir === sh.dir) && !a.chosen) { end(f); f.act = { kind: "screen", m: sh.m, dir: sh.dir }; } }
      else if (a.kind === "with" || a.kind === "screen" || a.you) end(f);
    }
    const act = f.act, partner = () => { const g = act.with && folk.get(act.with); return g && g.act.kind === act.kind && g.act.with === f.id ? g : null; };
    if (!["desk", "meeting", "chess", "chair", "sleep", "bench"].includes(act.kind)) f.sat = false;
    if (act.kind !== "football") f.kick = 0;
    switch (act.kind) {
      case "fps":                            // the gun in hand, walking as you walk
        f.carry = "gun"; f.arms = "gun"; f.legs = f.walking ? (Math.floor(w.T * 8) % 2 ? "walk1" : "walk2") : "stand";
        break;
      case "with": {
        // beside the bot you talk to, at its desk (it sits down for you): right of its chair,
        // unless another sits there, then in front of it (the bubble is yours while you type)
        const g = folk.get(act.to), gs = L.seat(act.to);
        if (!g) { f.act = rest(); break; }
        const seated = g.legs === "sit" && !g.lying, at = gs || g;
        const taken = seated && others.some((h) => h !== g && h.legs === "sit" && Math.abs(h.x - (at.x + SEAT)) < 4 && Math.abs(h.y - at.y) < 4);
        const spot = g.lying ? { x: g.x + 20, y: g.y + 1 } : taken ? { x: at.x + 2, y: at.y + 13 } : { x: at.x + 20, y: at.y + 1 };
        act.there = walk(f, spot, 36, dt);
        if (act.there) { f.dir = spot.x > g.x ? -1 : spot.x < g.x ? 1 : -1; f.arms = "down"; }
        break;
      }
      case "screen": {
        // at the screen whose terminal you have open: in front of its cabinet, under the screen (or
        // under the laptop, for a worktree's), the bubble while you type
        const d = L.desks.find((x) => (x.m || "") === act.m && (x.dir === act.dir || x.trees.includes(act.dir)));
        if (!d) { f.act = rest(); break; }
        const { sx } = screenAt(d), k = d.trees.indexOf(act.dir);
        act.there = walk(f, { x: k < 0 ? sx + 11 : sx + 29 + k * LAP, y: d.y + 13 }, 36, dt);
        if (act.there) { f.dir = -1; f.arms = "down"; }
        break;
      }
      case "desk":
        if (seat && moving && f.sat && L.desks.some((d) => d.key === moving.key && d.seats.some(([id]) => id === f.id))) { f.x = seat.x; f.y = seat.y; }   // the desk is being moved: it rides along
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
        if (walk(f, act, act.v || v, dt)) { f.arms = f.carry || "down"; if ((act.left -= dt) <= 0) f.act = rest(); }
        if (act.panic) f.arms = "up";        // away from the shotgun, hands up
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
      case "football": {
        // at its place round the pitch, facing the ball; a kick as the ball leaves it. One that gets work
        // leaves (its work act takes over); with one left the game is over
        const team = act.team.map((id) => folk.get(id)).filter((g) => g && g.act.kind === "football" && g.act.at === act.at);
        if (team.length < 2) { f.act = rest(); break; }
        act.there = walk(f, pitchSpot(act.at, act.slot, act.n), v, dt);
        if (act.there) {
          f.kick = Math.max(0, (f.kick || 0) - dt); f.arms = f.kick > 0 ? "fwd1" : "down";
          if (w.ball) f.dir = w.ball.x < f.x - 1 ? -1 : w.ball.x > f.x + 1 ? 1 : f.dir;
        }
        if ((act.left -= dt) <= 0) { for (const g of team) if (g !== f) g.act = rest(); f.act = rest(); w.ball = null; }
        break;
      }
      case "chess": {
        // alone at the board it waits for someone to play, studying it; two play until it's over (a real
        // game, both sides played from the server); against you it plays until the game is over
        if (!L.chess) { f.act = rest(); break; }
        if (act.with && !partner()) { act.with = null; act.left = Math.min(act.left, rnd(8, 16)); }
        act.there = sitAt(f, L.chessSeat(act.side), v, dt);
        // the game that is theirs: the one that started once both sat (an older finished game in the state is not it)
        const sg = app.state().chess, pair = [f.id, act.with];
        if (sg && !sg.result && !act.you && pair.includes(sg.white.id) && pair.includes(sg.black.id)) act.gameId = sg.id;
        const mine = act.gameId !== undefined && sg && sg.id === act.gameId;
        if (act.there) {
          f.dir = act.side ? -1 : 1; f.legs = "sit";
          f.arms = (f.moved = Math.max(0, (f.moved || 0) - dt)) > 0 ? "reach" : "down";
          if (!act.with && Math.random() < dt * 0.05) f.chat = 0.8;
          // both seated and no game yet: white (the left seat) asks the server for one between them
          const g = partner();
          if (g && g.act.there && act.side === 0 && !act.you && !(sg && !sg.result) && performance.now() - (w.chessAsked || 0) > 4000) { w.chessAsked = performance.now(); app.act({ kind: "chessBots", a: f.id, b: g.id }); }
        }
        if (mine && sg.result) act.left = Math.min(act.left, 3);               // their game is over: they get up soon
        if ((act.left -= dt) <= 0) {
          if (mine && !sg.result) app.act({ kind: "chessLeave", id: f.id });   // leaves the game unfinished: it is the other's
          const g = partner(); if (g) g.act = rest(); f.act = rest();
        }
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
          if ((act.wait += dt) > 0.6 && w.strokes < 40) { act.wait = 0; stroke(w, INK); }   // a black marker, as yours
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
        // it takes a rolled-up mat along to a clear bit of floor near where it is, rolls it out and
        // lies down: for as long as its process is off (no `left`), or for a nap (`left`), after
        // which it rolls the mat up again. A piece put down on it wakes it
        if (!act.at) { act.at = napSpot(f, L, others); if (!act.at) { f.act = rest(); break; } }
        if (act.there && !napClear(L, act.at.x, act.at.y, [])) { wake(f); break; }
        if (!act.there) { f.carry = "roll"; if (!walk(f, act.at, v, dt)) break; act.there = true; f.carry = null; }
        f.x = act.at.x; f.y = act.at.y; f.lying = true; f.dir = 1; f.legs = "sit"; f.arms = "down"; f.z += dt;
        if (act.left !== undefined && (act.left -= dt) <= 0) wake(f);
        break;
      }
      case "warm": {
        // its hands at the fire, from its left or its right side
        if (!L.fire) { f.act = rest(); break; }
        if (walk(f, { x: act.side ? L.fire.x1 + 5 : L.fire.x0 - 5, y: L.fire.y1 + 2 }, v, dt)) {
          f.legs = "stand"; f.dir = act.side ? -1 : 1; f.arms = Math.floor(w.T * 1.5) % 3 ? "reach" : "down";
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "watch": {
        // a spectator behind the chess or the ping-pong table while a game is on there: follows the
        // play, cheers a move, says something now and then
        const chess = act.game === "chess", t = chess ? L.chess : L.pong;
        const players = others.filter((g) => g.act.kind === act.game && g.act.there && (g.act.with || g.act.you));
        if (!t || players.length < 2) { f.act = rest(); break; }
        if (walk(f, { x: t.x, y: Math.max(WALL + 30, t.y - (chess ? 17 : 22)) }, v, dt)) {
          f.legs = "stand";
          f.dir = chess ? (Math.floor(w.T * 0.5 + f.z) % 2 ? 1 : -1) : w.pong.from ? -1 : 1;
          f.arms = chess && performance.now() - (w.chessAt || 0) < 600 ? "up" : "down";
          if (Math.random() < dt * 0.1) f.chat = 0.8;
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "stretch":
        // a stretch where it stands: arms up, arms down
        f.legs = "stand"; f.arms = Math.floor(w.T * 2.5) % 2 ? "up" : "down";
        if ((act.left -= dt) <= 0) f.act = rest();
        break;
      case "serve": {
        // a coffee for a busy colleague: fetched at the machine, put on its desk by the keyboard
        // (if it has got up meanwhile, the server keeps the cup)
        const g = folk.get(act.to), gs = L.seat(act.to);
        if (!L.coffee || !g || !gs) { f.act = rest(); break; }
        if (act.stage === 0) { if (walk(f, L.coffee, v, dt)) { f.dir = -1; f.arms = "reach"; act.stage = 1; } }
        else if (act.stage === 1) {
          act.wait += dt; const dry = usage.session && usage.session.used >= 100;
          if (dry) { f.act = rest(); break; }
          if (act.wait > 2) { f.carry = "mug"; act.stage = 2; act.wait = 0; } else f.arms = "reach";
        } else if (walk(f, { x: gs.x + 20, y: gs.y + 1 }, v, dt)) {
          f.dir = -1; f.arms = "mug";
          if ((act.wait += dt) > 1.2) { if (g.legs === "sit" && g.act.kind === "desk") { w.mugs.set(g.id, w.T + 90); f.carry = null; } f.act = rest(); }
        }
        break;
      }
      case "doom": {
        // put in the booth: it stands inside, behind the glass; the sign over the booth counts down, a click
        // on it (or on the bot) is the undo, and so is carrying it out. Then the light, and it is gone:
        // `claude stop` and `claude rm` (its transcript stays, for `claude --resume`). The fire does the
        // same at once, leaving charred remains (bury)
        const bo = L.booths.find((p) => p.key === act.at);
        if (!bo) { f.act = resume(f); break; }
        f.x = bo.x; f.y = bo.y - 3; f.dir = -1; f.arms = "down"; f.legs = "stand";
        if ((act.left -= dt) <= 0 && !f.gone) {
          app.act({ kind: "stop", id: f.id, rm: true });
          w.stopped.set(f.id, performance.now()); w.boothFlash = { key: bo.key, t0: performance.now() };
          folk.delete(f.id);
        }
        break;
      }
      case "queue": {                        // let go of from here: in line by the booth (on its roomier side), its turn next
        const bo = L.booths[0];
        if (!bo) break;
        const n = act.n || 0, side = bo.x - 70 < L.floor.x0 ? 1 : -1;
        act.there = walk(f, { x: bo.x + side * (18 + (n % 5) * 12), y: Math.min(L.floor.y1, bo.y + 2 + Math.floor(n / 5) * 9) }, v, dt);
        if (act.there) { f.legs = "stand"; f.dir = -side; f.arms = "down"; }
        break;
      }
      case "condemned":                      // hardcore: where it is, hands up, until you get to it
        f.legs = "stand"; f.arms = "up";
        break;
      case "pyre": {                         // no booth: into the fire (and with no fire, gone at once)
        if (!L.fire) { app.act({ kind: "stop", id: f.id, rm: true }); w.stopped.set(f.id, performance.now()); folk.delete(f.id); break; }
        if (walk(f, { x: (L.fire.x0 + L.fire.x1) / 2, y: L.fire.y1 - 8 }, v, dt)) bury(f, L);
        break;
      }
      case "sweep": {                        // clearing away what is left of a bot: to the nearest piece that landed, bent down for it
        const r = w.remains.get(act.at);
        if (!r || r.back || !r.parts.length) { f.act = rest(); break; }
        const p = r.parts.filter((q) => !q.z && !q.vx && !q.vy).sort((a, b) => Math.hypot(a.x - f.x, a.y - f.y) - Math.hypot(b.x - f.x, b.y - f.y))[0];
        if (!p) { if (walk(f, { x: r.x + (f.x < r.x ? -12 : 12), y: r.y + 4 }, v, dt)) { f.legs = "stand"; f.arms = "down"; } break; }   // still flying: over to the mess meanwhile
        if (walk(f, { x: p.x + (f.x < p.x ? -5 : 5), y: p.y + 1 }, v, dt)) {
          f.legs = "stand"; f.dir = p.x < f.x ? -1 : 1; f.arms = Math.floor(w.T * 4) % 2 ? "reach" : "down";
          if ((act.wait += dt) > 0.8) {
            act.wait = 0;
            r.parts.splice(r.parts.indexOf(p), 1);
            r.drops = r.drops.filter((d) => Math.hypot(d.x - p.x, d.y - p.y) > 7);   // the oil there wiped up with it
            if (!r.parts.length) { gone(w, act.at); f.act = rest(); }
          }
        }
        break;
      }
      case "escort": {                       // walked to the booth by the office manager, a step behind her
        const m = folk.get("manager");
        if (!m) { f.act = rest(); break; }
        if (walk(f, { x: m.x - 9 * (m.dir || -1), y: m.y }, 24, dt)) { f.legs = "stand"; f.dir = m.dir || -1; f.arms = "down"; }
        break;
      }
      case "neuralyze": {
        // put to the neuralyzer: it steps up, the flash (`/clear` in its session: a new conversation), and
        // it stands there dazed a moment
        const n = L.neuralyzers.find((p) => p.key === act.at);
        if (!n) { f.act = rest(); break; }
        if (act.stage === 0) { if (walk(f, { x: n.x + 11, y: n.y + 1 }, v, dt)) { f.legs = "stand"; f.dir = -1; f.arms = "down"; act.stage = 1; act.wait = 0; } }
        else if (act.stage === 1) {
          if ((act.wait += dt) > 0.6) { app.act({ kind: "clear", id: f.id }); w.flash = { x: n.x - 5, y: n.y - 16, t0: performance.now() }; act.stage = 2; act.wait = 0; }
        } else if ((act.wait += dt) > 2.5) f.act = rest();
        break;
      }
      case "bench": {                        // a sit on the bench, a while
        const bn = L.benches.find((b) => b.key === act.at);
        if (!bn) { f.act = rest(); break; }
        if (sitAt(f, benchSeat(bn, act.seat), v, dt)) { f.dir = -1; f.legs = "sit"; f.arms = "down"; if ((act.left -= dt) <= 0) f.act = rest(); }
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
    if (waiter && r < 0.7) {
      const left = rnd(40, 90);
      waiter.act.with = f.id; waiter.act.left = left;
      return { kind: "chess", side: 1 - waiter.act.side, with: waiter.id, left, there: false };
    }
    if (L.meet && L.board && mates.length >= 2 && !w.meeting && w.T - w.lastMeeting > 60 && r < 0.08) {
      w.meeting = { host: f.id, left: rnd(22, 38) }; w.lastMeeting = w.T; w.meetings++;
      if (w.ink && inked(w) > 0.3) wipe(w);                 // a full board is wiped for the meeting
      mates.slice(0, 4).forEach((g, i) => { g.carry = null; g.act = { kind: "meeting", seat: i }; });
      return { kind: "present", wait: 0 };
    }
    if (r < 0.16 && L.coffee && !taken("coffee")) return { kind: "coffee", stage: 0, wait: 0 };
    if (r < 0.22 && L.printer && !taken("print")) return { kind: "print", stage: 0, wait: 0 };
    if (r < 0.3 && mate && L.cooler) {
      const left = rnd(10, 22);
      mate.carry = null; mate.act = { kind: "cooler", side: 1, left, with: f.id };
      return { kind: "cooler", side: 0, left, with: mate.id };
    }
    if (r < 0.4 && mate && L.pong && !taken("pong") && !app.pong()) {
      const left = rnd(15, 35);
      mate.carry = null; mate.act = { kind: "pong", side: 1, left, with: f.id, there: false };
      return { kind: "pong", side: 0, left, with: mate.id, there: false };
    }
    if (r < 0.44 && mates.length >= 1 && !taken("football")) {   // a kickabout on a clear bit of floor, two or three of them
      const game = football(w, L, [f, ...mates.slice(0, 2)]);
      if (game) return game;
    }
    if (r < 0.48 && L.chess && !taken("chess") && !(app.state().chess && !app.state().chess.result)) return { kind: "chess", side: Math.random() < 0.5 ? 0 : 1, with: null, left: rnd(20, 45), there: false };
    // a coffee for a busy colleague at its desk; a game to watch; the fire; a stretch; a nap
    const busy = others.filter((g) => g.work && g.legs === "sit" && g.act.kind === "desk" && !w.mugs.has(g.id));
    if (r < 0.54 && busy.length && L.coffee && !taken("coffee") && !taken("serve")) return { kind: "serve", to: any(busy).id, stage: 0, wait: 0 };
    const game = ["chess", "pong"].find((k) => others.filter((g) => g.act.kind === k && g.act.there && (g.act.with || g.act.you)).length >= 2 && !others.some((g) => g.act.kind === "watch" && g.act.game === k));
    if (r < 0.6 && game) return { kind: "watch", game, left: rnd(10, 25) };
    if (r < 0.65 && mate) return { kind: "visit", to: mate.id, left: rnd(6, 12), wave: 0.8 };
    if (r < 0.7 && L.fire && !(moving && KIND(moving.key) === "fire")) { const side = others.some((g) => g.act.kind === "warm" && !g.act.side) ? 1 : 0; if (!others.some((g) => g.act.kind === "warm" && g.act.side === side)) return { kind: "warm", side, left: rnd(8, 16) }; }
    if (r < 0.74) return { kind: "stretch", left: rnd(4, 8) };
    if (r < 0.8 && !f.human) return { kind: "sleep", there: false, left: rnd(25, 50) };
    // one of a kind there can be many of, not in use: a plant to water, a window, an armchair
    const spare = (kind, ps) => ps.filter((p) => !others.some((g) => g.act.kind === kind && g.act.at === p.key));
    const plants = spare("plant", L.plants), wins = spare("window", L.windows), chairs = spare("chair", L.chairs);
    if (r < 0.86 && plants.length) return { kind: "plant", at: any(plants).key, wait: 0 };
    if (r < 0.91 && wins.length) return { kind: "window", at: any(wins).key, left: rnd(4, 9) };
    if (r < 0.96 && chairs.length) return { kind: "chair", at: any(chairs).key, left: rnd(15, 35) };
    const bs = L.benches.flatMap((bn) => benchFree(bn, f));
    if (r < 0.99 && bs.length) return { kind: "bench", ...any(bs), left: rnd(20, 50) };
    for (let k = 0; k < 8; k++) {             // somewhere free to stand about
      const x = rnd(L.floor.x0, L.floor.x1), y = rnd(L.floor.y0, L.floor.y1);
      if (L.free(x, y)) return { kind: "loiter", x, y, left: rnd(3, 8) };
    }
    return rest();
  }
  // football: a few idle bots stand round a clear bit of floor and pass a ball about; the one it comes
  // to kicks it on to another. The pitch is found near the one who starts it (rings outward, a round
  // clear patch); each player has a place round it. The ball is the world's (`w.ball`)
  const PASS = 0.7;
  function pitchSpot(at, i, n) { const a = (2 * Math.PI * i) / n - Math.PI / 2; return { x: at.x + Math.round(Math.cos(a) * 18), y: at.y + Math.round(Math.sin(a) * 12) }; }
  function footballPitch(L, f) {
    const clear = (x, y) => [[0, 0], [20, 0], [-20, 0], [0, 14], [0, -14], [14, 10], [-14, 10], [14, -10], [-14, -10]]
      .every(([dx, dy]) => x + dx > L.floor.x0 && x + dx < L.floor.x1 && y + dy > L.floor.y0 && y + dy < L.floor.y1 && L.free(x + dx, y + dy));
    for (let d = 0; d <= 120; d += 10) for (let a = 0; a < 6.283; a += d ? 0.6 : 7) {
      const x = Math.round(f.x + Math.cos(a) * d), y = Math.round(f.y + Math.sin(a) * d);
      if (clear(x, y)) return { x, y };
    }
    return null;
  }
  function football(w, L, team) {
    const at = footballPitch(L, team[0]);
    if (!at) return null;
    const left = rnd(25, 50), ids = team.map((g) => g.id);
    team.forEach((g, i) => { g.carry = null; g.act = { kind: "football", at, slot: i, n: team.length, left, there: false, team: ids }; });
    w.ball = { x: at.x, y: at.y, x0: at.x, y0: at.y, from: ids[0], to: ids[1], t: 1, h: 0 };
    return team[0].act;
  }
  function kickabout(w, dt) {                // the ball: at someone's feet a moment, then over to another (a little hop)
    const ball = w.ball;
    if (!ball) return;
    const folk = w.folk, players = [...folk.values()].filter((g) => g.act.kind === "football" && g.act.there);
    const to = folk.get(ball.to);
    if (players.length < 2) { if (![...folk.values()].some((g) => g.act.kind === "football")) w.ball = null; return; }
    if (!to || !players.includes(to)) { const next = any(players.filter((g) => g.id !== ball.from)) || players[0]; ball.to = next.id; ball.t = 1; return; }
    if (ball.t >= 1) {
      ball.wait = (ball.wait ?? rnd(0.3, 0.9)) - dt;
      ball.x = to.x + (to.dir < 0 ? -4 : 4); ball.y = to.y; ball.h = 0;
      if (ball.wait <= 0) {
        const next = any(players.filter((g) => g !== to));
        to.kick = 0.3; ball.from = to.id; ball.to = next.id; ball.t = 0; ball.x0 = ball.x; ball.y0 = ball.y; ball.wait = undefined;
      }
    } else {
      ball.t = Math.min(1, ball.t + dt / PASS);
      const tx = to.x + (to.x > ball.x0 ? -4 : 4), ty = to.y;
      ball.x = ball.x0 + (tx - ball.x0) * ball.t; ball.y = ball.y0 + (ty - ball.y0) * ball.t; ball.h = Math.sin(ball.t * Math.PI) * 6;
    }
  }
  function stroke(w, col) {                  // the host draws on the whiteboard: boxes, arrows, dots, in its colour
    const k = Math.random(), S = INKS, sw = k < 0.35 ? Math.floor(rnd(3, 7)) : k < 0.7 ? Math.floor(rnd(3, 10)) : 1;
    const sh = k < 0.35 ? Math.floor(rnd(2, 4)) : 1;
    const x = Math.floor(rnd(1, INK_AREA.w - sw)) * S + 2, y = Math.floor(rnd(1, INK_AREA.h - sh - 1)) * S + 2, X = x + sw * S - 2, Y = y + sh * S - 2;
    if (k < 0.35) { inkLine(w, x, y, X, y, col, 2); inkLine(w, x, Y, X, Y, col, 2); inkLine(w, x, y, x, Y, col, 2); inkLine(w, X, y, X, Y, col, 2); }
    else if (k < 0.7) { inkLine(w, x, y, X, y, col, 2); inkLine(w, X - 3, y - 3, X, y, col, 2); inkLine(w, X - 3, y + 3, X, y, col, 2); }
    else inkLine(w, x, y, x + 1, y + 1, col, 2);
    w.strokes++;
  }

  // ---- drawing, and what is under the pointer: hot spots are collected while drawing
  let pointer = null, hots = [], hovered = null, tip = null, lastL = null, moving = null, held = null, hoverId = null, lastHp = null, dangerNow = false;
  let fps = null, view = null;              // hardcore: the gun taken from the gun closet, the office in the first person (fps.js); the scene it renders
  let pics = [], wallPics = [];              // what was just drawn, each piece by itself, for that view (scene3)
  const hardcore = () => !!app.settings().hardcore;
  let screenHover = null, hoverScreen = null;    // the screen (or laptop) under the pointer: {m, dir}; the one told to the page
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
    hots = []; tip = null; screenHover = null; pics = []; wallPics = [];
    ctx.drawImage(floor(), 0, 0);
    rect(ctx, W - 1, 0, 1, H, INK2);
    if (!s) return;
    const w = world(), all = s.bots || [], usage = s.usage || {};
    // every session is a bot about the room; a chair at its desk only while its process runs (a completed one
    // would otherwise keep a chair for good: a desk a dozen chairs long). Opening one restarts it, and its chair comes
    const bots = all;
    const L = layout(w.razed.size ? { ...s, desks: s.desks.filter((d) => !w.razed.has(d.unit)) } : s, all.filter((b) => !b.over), moving);   // (a burnt desk whose bots are on their way out)
    // the piece under the pointer: outlined if a click does something there (the rest only reacts in
    // its own way, a plant sways)
    const hp = !moving && !held && !carrying ? pieceAt(pointer) : null;
    lastHp = hp;
    const on = (key) => !!hp && hp.key === key, lit = (key) => on(key) && CLICK.has(KIND(key));
    // what is being dragged over the fire (a desk without bots, a piece, a bot) is about to burn, a bot
    // held at its door is about to be stopped: it is outlined red and washed in red, flashing (tinted)
    const heldBot = held && held.b && !held.b.human && pointer ? held : null, botDrop = heldBot && dropAt(L, heldBot);
    const boothUnder = botDrop === "booth" ? boothAt(L, heldBot) : null;   // a bot held over the booth: let go and it goes in
    dangerNow = !!boothUnder || botDrop === "fire" || !!(L.fire && moving && inFire(L, moving.key, moving.at, moving.u));
    const danger = (key) => dangerNow && !!moving && moving.key === key, tintOf = (key) => (danger(key) ? flashA(w.T) : 0);
    // a desk being dragged takes its seated bots along, this very frame
    if (moving && moving.key.startsWith("u:")) {
      const d = L.desks.find((d) => d.key === moving.key);
      if (d) for (const [id, x] of d.seats) { const f = w.folk.get(id); if (f && f.sat && !f.held) { f.x = x; f.y = d.y; } }
    }
    // and an armchair whoever reads in it (you too)
    if (moving && KIND(moving.key) === "armchair") {
      const ch = L.chairs.find((c) => c.key === moving.key);
      if (ch) for (const f of w.folk.values()) if (f.act.kind === "chair" && f.act.at === ch.key && f.sat && !f.held) { f.x = ch.x - 2; f.y = ch.y - 1; }
    }
    lastL = L;
    if (Object.keys(L.placed).length) app.act({ kind: "layout", layout: { ...(s.layout || {}), ...L.placed } });
    s.layout = { ...(s.layout || {}), ...L.placed };

    // keep a figure per bot; one that turns up later comes in through its machine's door, one that is
    // gone from the list (deleted elsewhere) walks out of it
    const ids = new Set(bots.map((b) => b.id)), you ={ id: "you", name: (s.you && s.you.name) || "you", seed: s.you && s.you.seed, color: s.you && s.you.color, human: true, m: "" };
    ids.add("you");
    // the office manager, while her desk is in the room: the notes on the sessions are hers (thinkManager)
    const mgr = L.manager ? { id: "manager", name: "office manager", seed: MANAGER.seed, color: MANAGER.color, human: false, staff: true, m: "" } : null;
    if (mgr) ids.add("manager");
    for (const f of w.folk.values()) if (!ids.has(f.id) && !f.leaving) { f.leaving = true; f.held = false; f.lying = false; f.sat = false; f.legs = "stand"; }
    for (const [id, t] of w.stopped) if (!ids.has(id) || performance.now() - t > 30000) w.stopped.delete(id);
    const late = performance.now() - w.born > 3000;
    for (const b of [...bots, you, ...(mgr ? [mgr] : [])]) {
      if (w.stopped.has(b.id) || w.remains.has(b.id)) continue;      // stopped from here (the feed still lists it for a moment), or in pieces
      if (!w.folk.has(b.id)) {
        const at = b.staff && !late ? managerSeat(L) : late || b.human ? w.riseAt.get(b.id) || { x: doorX(b.m), y: WALL + 3 } : L.seat(b.id) || freeAt(L, b.id);   // back from its grave, or in by the door; at first, at its desk or anywhere
        w.riseAt.delete(b.id);
        const work = !b.human && (b.busy || b.waiting), stand = late || (b.human && !b.staff);
        w.folk.set(b.id, { id: b.id, x: at.x, y: at.y, dir: -1, arms: "down", legs: stand ? "stand" : "sit", step: 0, sat: !stand,
          act: work ? { kind: "desk" } : rest(), carry: null, lying: false, busy: b.busy, waiting: b.waiting, work, human: !!b.human, staff: !!b.staff,
          chat: 0, z: Math.random() * 2 });
      }
      w.folk.get(b.id).b = b;
    }
    if (tickDt > 0) {
      w.T += tickDt; think(w, bots, L, tickDt, usage, s.chess);
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

    // what hangs on the wall, outlined under the pointer; one being carried is drawn over everything
    // else (it is in your hand), washed red over the fire
    let aloft = null;
    const onTheWall = (key, r, h, fn) => {
      wallPics.push(fn);
      if (moving && moving.key === key) aloft = () => outlined(ctx, r, danger(key), fn, RED, tintOf(key));
      else outlined(ctx, r, h, fn);
    };
    // the doors, one a machine, where they hang on the wall; its name and its bots lit under the
    // pointer, a click lists them all (names, paints). A door is ajar while someone goes in or out; boarded
    // up while its machine is out of reach (why, under the pointer). A new machine's door, dropped from the
    // closet, hangs where it was let go while its form is open (`pending`)
    for (const dr of L.doors) {
      const m = dr.m, x0 = dr.x0, y0 = dr.y0, h = hot(x0, y0, x0 + 16, y0 + 20, { kind: "doors" });
      const ajar = performance.now() - (w.doorAt[m.id] || 0) < 900 ||
        [...w.folk.values()].some((f) => f.b && (f.b.m || "") === m.id && f.leaving && f.y < WALL + 12);
      onTheWall(dr.key, foot("door", [x0, y0]), h, (c) => drawDoor(c, x0, paintOf(m.id), ajar && !m.error, !!m.error, y0));
      if (h) { for (const b of bots) if ((b.m || "") === m.id) hl.add(b.id); tip = { text: clip(m.error ? m.name + ": " + m.error : m.name, W - 8), x: x0 + 8, y: 20 }; }
    }
    if (pending && pending.mid != null && (s.machines.some((m) => m.id === pending.mid) || performance.now() - pending.t > 10000)) pending = null;
    if (pending) drawKind(ctx, "door", pending.at, w, T);
    // trouble here goes on the wall, in red, instead of the whiteboard
    const here = s.machines.find((m) => !m.id), trouble = s.note || (here && here.error);
    if (trouble) { const nx = Math.max(24, ...L.doors.map((d) => d.x0 + 20)); tiny(ctx, clip(trouble, W - 24 - nx), nx, 18, RED); }   // past the last door
    for (const win of L.windows) {
      const h = hot(win.x0, win.y0, win.x1, win.y1, { kind: "outside" });
      onTheWall(win.key, [win.x0, win.y0, win.x1, win.y1], h, (c) => drawWindow(c, win));
      if (h) tip = { text: "outside", x: (win.x0 + win.x1) / 2, y: win.y1 + 10 };
    }
    if (L.clock) {
      // the clock keeps this computer awake, as Amphetamine does: a click chooses (while bots work, for
      // a while, not at all); its face says which, yellowed (drawClock)
      const ck = L.clock, a = s.awake || { mode: "off" }, h = hot(ck.x0, ck.y0, ck.x0 + 12, ck.y0 + 10, { kind: "clock" });
      onTheWall(ck.key, foot("clock", [ck.x0, ck.y0]), h, (c) => drawClock(c, ck.x0 + 6, ck.y0 + 5, a));
      if (h) {
        const tm = Math.round(Math.max(0, a.until - Date.now()) / 60000), hh = Math.floor(tm / 60), mm = tm % 60;
        tip = { text: a.mode === "for" ? `awake ${hh ? hh + "h " : ""}${mm}m more` : a.mode === "work" ? (a.on ? "awake, local bots at work" : "awake when local bots work") : "clock", x: ck.x0 + 6, y: WALL + 10 };
      }
    }
    if (L.closet) {
      // the closet, against the wall: the furniture that is not in the room; a click opens it (on the right)
      const c = L.closet, h = hot(c.x0, c.y0, c.x1, c.y1, { kind: "closet" });
      outlined(ctx, [c.x0, c.y0, c.x1, c.y1], h, (x) => drawCloset(x, c, app.view() === "closet"));
      wallPics.push((x) => drawCloset(x, c, false));
      if (h) tip = { text: "furniture", x: c.x0 + 11, y: c.y0 + 4 };
    }
    if (L.settings) {
      // the switchboard on the wall: burrow's own settings, a switch each; a click opens them
      const b = L.settings, h = hot(b.x0, b.y0, b.x1, b.y1, { kind: "settings" });
      outlined(ctx, [b.x0, b.y0, b.x1, b.y1], h, (x) => drawSwitchboard(x, b, app.settings(), app.view() === "settings"));
      wallPics.push((x) => drawSwitchboard(x, b, app.settings(), false));
      if (h) tip = { text: "settings", x: b.x0 + 7, y: b.y0 + 4 };
    }
    // a bot and its seat at its project's desk are one thing: where the bot is only says what it
    // does. The pointer on either lights up both (and its session), and a click on either opens it
    if (hoverId) hl.add(hoverId);
    let seatHover = null;
    if (!moving && !held) for (const d of L.desks) for (const [id, x] of d.seats) if (over(x - 3, d.y - 16, x + 9, d.y + 1)) seatHover = { id, x, y: d.y };
    if (seatHover) hl.add(seatHover.id);

    // flat on the floor: the empty cabinet that adds a project, the mats rolled out for a nap, the ping-pong rug
    if (L.add) {
      const a = L.add, h = hot(a.x0 - 2, a.y0 - 10, a.x1 + 2, a.y1 + 1, { kind: "addDir" });
      outlined(ctx, foot("add", [a.x0, a.y1]), h || danger(a.key), (c) => drawAdd(c, a, h), danger(a.key) ? RED : GOLD, tintOf(a.key));
      pics.push({ key: a.key, kind: "add", x: a.x0, y: a.y1, fn: (c) => drawAdd(c, a, false) });
      if (h) tip = { text: "project", x: a.x0 + ADD_W / 2, y: a.y0 - 2 };
    }
    for (const f of w.folk.values()) if (f.act.kind === "sleep" && f.act.there && f.act.at) drawMat(ctx, f.act.at);
    if (L.pong) drawRug(ctx, L.pong);

    // everything that stands, back to front
    const items = [];
    if (aloft) items.push([1e9, aloft]);
    for (const f of w.folk.values()) if (!f.lying && !f.held) items.push([f.y, () => drawFigure(ctx, f, f.b, T, hl)]);
    for (const d of L.desks) {
      const sc = screenAt(d);
      if (!moving && !held && over(sc.sx, sc.sy, sc.sx + 22, sc.sy + 18)) for (const b of bots) if (b.over && b.unit === d.unit) hl.add(b.id);   // its completed ones too (no chair)
      items.push([danger(d.key) ? 1e9 : d.y - 1, () => outlined(ctx, foot(d.key, d.at, d.u), danger(d.key), (c) => drawDesk(c, d, w, T, hl), RED, tintOf(d.key))]);
      for (const [id, x] of d.seats) items.push([d.y - 0.5, () => drawChair(ctx, x, d.y, hl.has(id))]);
    }
    // every piece: outlined under the pointer, like a desk's chairs; what it does, on top of that. Held over
    // the fire it is drawn over everything (the flames would hide it), washed red
    const piece = (p, kind, fn) => {
      pics.push({ key: p.key, kind, x: p.x, y: p.y, fn });
      items.push([danger(p.key) ? 1e9 : p.y - 0.5, () => outlined(ctx, foot(kind, [p.x, p.y]), lit(p.key) || danger(p.key), fn, danger(p.key) ? RED : GOLD, tintOf(p.key))]);
    };
    if (L.meet) piece(L.meet, "meet", (c) => { drawTable(c, L.meet); for (const i of [0, 1]) drawStool(c, L.meetSeat(i)); });
    if (L.chess) {
      const c = L.chess, g = s.chess && !s.chess.result ? s.chess : null, top = [c.x - 10, c.y - 17, c.x + 10, c.y - 5];
      const h = hot(...top, { kind: "chess" });
      if (h) tip = { text: !g ? "chess" : [g.white.id, g.black.id].includes("you") ? "chess with " + (g.white.id === "you" ? g.black.name : g.white.name) : g.white.name + " vs " + g.black.name, x: c.x, y: c.y - 18 };   // yours, or theirs to watch
      items.push([c.y - 0.5, () => { for (const i of [0, 1]) drawStool(ctx, L.chessSeat(i)); outlined(ctx, top, h, (x) => drawChess(x, c, g ? g.board : START)); }]);
      pics.push({ key: c.key, kind: "chess", x: c.x, y: c.y, fn: (x) => { for (const i of [0, 1]) drawStool(x, L.chessSeat(i)); drawChess(x, c, g ? g.board : START); } });
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
    if (L.manager) {
      // the office manager's desk: a click (on it or on her) opens the notes beside the room
      const mg = L.manager, dg = app.digest(), live = new Set(all.map((b) => b.id));
      const n = dg ? Object.keys(dg.sessions || {}).filter((id) => live.has(id)).length : 0;
      const h = hot(mg.x - 2, mg.y - 17, mg.x + 30, mg.y + 2, { kind: "notes" });
      piece(mg, "manager", (c) => drawManagerDesk(c, mg, n, h || hl.has("manager")));
      if (h) tip = { text: "notes", x: mg.x + 12, y: mg.y - 18 };
    }
    for (const ch of L.chairs) {              // a click on one: you go and sit in it (a bot in it gets up)
      const h = hot(ch.x - 6, ch.y - 23, ch.x + 18, ch.y + 1, { kind: "sit", key: ch.key });
      piece(ch, "armchair", (c) => drawArmchair(c, ch));
      if (h) tip = { text: "have a seat", x: ch.x + 6, y: ch.y - 24 };
    }
    if (L.board) {
      const b = L.board, h = hot(b.x - 15, b.y - 26, b.x + 15, b.y - 11, { kind: "board" });
      piece(b, "board", (c) => drawBoard(c, b, w));
      if (h) tip = { text: "whiteboard", x: b.x, y: b.y - 28 };
    }
    if (L.fire) {
      // the fireplace: what is held over it (a desk, a piece of furniture, a bot) is about to burn:
      // the fire flares and the fireplace lights up red. A desk with bots can't burn. The pointer
      // on it stirs sparks
      const F = L.fire, r = [F.x0, F.y0, F.x1, F.y1];
      const feeding = (moving && inFire(L, moving.key, moving.at, moving.u)) || botDrop === "fire";
      const fd = moving && feeding && L.desks.find((d) => d.key === moving.key);
      const onIt = !moving && !held && over(...r);
      if (onIt && !w.sparkOn) { w.sparkOn = true; w.sparks = performance.now(); } else if (!onIt) w.sparkOn = false;
      const since = Math.min(performance.now() - w.sparks, performance.now() - w.flare), burst = since < 1500 ? since / 1500 : -1;
      w.burning = w.burning.filter((o) => performance.now() - o.t0 < 1400);
      items.push([F.y1, () => drawFire(ctx, F, T, !!feeding, burst, w.burning)]);
      pics.push({ key: F.key, kind: "fire", x: F.x0, y: F.y1, fn: (x) => drawFire(x, F, T, false, burst, w.burning) });
      const tx = F.x0 + 16, ty = F.y0 - 2;
      if (fd) tip = { text: "burn " + fd.label + (!s.bots.some((b) => b.unit === fd.unit) ? "" : !L.booths.length ? " and its bots" : hardcore() ? ", shoot its bots" : ", its bots to the booth"), x: tx, y: ty };
      else if (feeding && moving && KIND(moving.key) === "door") { const m = s.machines.find((m) => "door:" + m.id === moving.key); tip = { text: "take out " + (m ? m.name : "the door"), x: tx, y: ty }; }
      else if (feeding && moving) tip = { text: "burn the " + (NAMES[KIND(moving.key)] || KIND(moving.key)), x: tx, y: ty };
      else if (feeding) tip = { text: "delete " + held.b.name, x: tx, y: ty };
    }
    // the booth: a bot let go over it goes in, and after five seconds out of the list for good; the
    // neuralyzer: a bot let go by it gets `/clear` (the drop is useAt's)
    for (const bo of L.booths) {
      if (hardcore()) {                     // hardcore: a gun closet; a click takes a gun and you see the office through your own eyes (fps.js)
        const out = !!fps || (w.clean && w.clean.armed) || (w.folk.get("you") || {}).carry === "gun", h = hot(bo.x - 9, bo.y - 32, bo.x + 9, bo.y + 2, { kind: "aim", key: bo.key });
        items.push([bo.y - 3.5, () => outlined(ctx, foot("booth", [bo.x, bo.y]), h || danger(bo.key), (c) => drawGunCloset(c, bo, false, out), danger(bo.key) ? RED : GOLD, tintOf(bo.key))]);
        items.push([bo.y - 2.5, () => drawGunCloset(ctx, bo, true, out)]);
        pics.push({ key: bo.key, kind: "booth", x: bo.x, y: bo.y, fn: (c) => { drawGunCloset(c, bo, false, out); drawGunCloset(c, bo, true, out); } });
        if (h) tip = { text: "gun closet: take a gun", x: bo.x, y: bo.y - 34 };
        continue;
      }
      const inside = [...w.folk.values()].some((f) => f.act.kind === "doom" && f.act.at === bo.key);
      const fl = w.boothFlash && w.boothFlash.key === bo.key ? (performance.now() - w.boothFlash.t0) / 1200 : -1, flagged = !!boothUnder && boothUnder.key === bo.key;
      items.push([bo.y - 3.5, () => outlined(ctx, foot("booth", [bo.x, bo.y]), danger(bo.key) || flagged, (c) => drawBooth(c, bo, false, inside || flagged, fl), RED, tintOf(bo.key))]);
      items.push([bo.y - 2.5, () => drawBooth(ctx, bo, true, inside, fl)]);
      pics.push({ key: bo.key, kind: "booth", x: bo.x, y: bo.y, fn: (c) => { drawBooth(c, bo, false, inside, fl); drawBooth(c, bo, true, inside, fl); } });
      if (flagged) tip = { text: "bye " + heldBot.b.name, x: bo.x, y: bo.y - 34 };
      else if (on(bo.key) && !inside) tip = { text: "booth: a bot in it is deleted", x: bo.x, y: bo.y - 34 };
    }
    for (const n of L.neuralyzers) {
      const fl = w.flash && performance.now() - w.flash.t0 < 400;
      piece(n, "neuralyzer", (c) => drawNeuralyzer(c, n, fl));
      if (heldBot && inRect(spot(heldBot), foot("neuralyzer", [n.x, n.y]))) tip = { text: heldBot.work ? heldBot.b.name + " is busy" : "/clear " + heldBot.b.name, x: n.x, y: n.y - 21 };
      else if (on(n.key)) tip = { text: "neuralyzer: /clear a bot", x: n.x, y: n.y - 21 };
    }
    L.plants.forEach((pl) => {
      // the pointer on a plant stirs its leaves: two movements, then it is still until the pointer comes back
      if (on(pl.key)) { if (!w.sway.has(pl.key)) w.sway.set(pl.key, performance.now()); } else w.sway.delete(pl.key);
      const h = hot(...foot("plant", [pl.x, pl.y]), { kind: "water", key: pl.key });   // a click: you go and water it
      piece(pl, "plant", (c) => drawPlant(c, pl, +(pl.key.match(/\d+$/) || [0])[0] % 2, w.sway.get(pl.key)));
      if (h) tip = { text: "water", x: pl.x, y: pl.y - 32 };
    });
    for (const pr of L.pairs) {
      // a pair room (a chat of people and agents, pair.daems.me): a click opens the chat; a bot
      // dropped on it joins the chat. Its light is on once a room is linked
      const info = (s.pair || {})[pr.key], h = hot(pr.x - 13, pr.y - 20, pr.x + 13, pr.y + 1, { kind: "pairOpen", key: pr.key });
      const over_ = !!heldBot && inRect(spot(heldBot), foot("pair", [pr.x, pr.y]));
      piece(pr, "pair", (c) => drawPair(c, pr, !!info, T, over_));
      if (h) tip = { text: info ? info.room || "pair" : "pair: open a room", x: pr.x, y: pr.y - 22 };
      else if (over_) tip = { text: info ? held.b.name + " joins " + (info.room || "the room") : "no room linked yet", x: pr.x, y: pr.y - 22 };
    }
    if (w.ball) {                           // the football: a small ball, its shadow on the floor, a hop as it flies
      const bl = w.ball, bx = Math.round(bl.x), by = Math.round(bl.y), h = Math.round(bl.h);
      items.push([bl.y + 0.2, () => { rect(ctx, bx - 1, by, 3, 1, "rgba(0,0,0,0.3)"); rect(ctx, bx - 1, by - h - 3, 3, 3, INK); rect(ctx, bx, by - h - 2, 1, 1, C.white); }]);
    }
    // what is left of a killed bot: the oil under everything, each piece at its own depth; a click on any of
    // it puts the bot back together, its name over it under the pointer
    for (const [id, r] of w.remains) {
      const age = (performance.now() - r.t0) / 1000, fade = age > RUST.after ? Math.min(1, (age - RUST.after) / RUST.fade) : 0;
      items.push([-1e9, () => {
        if (fade) ctx.globalAlpha = 1 - fade;
        const rx = Math.round(r.puddle * r.parts.length / r.total);         // the puddle shrinks as it is mopped up
        if (rx > 1) ELL(rx, 3, 5).forEach((hw, j) => { rect(ctx, r.x - hw, r.y - 2 + j, 2 * hw, 1, "#17151f"); if (j === 1 && hw > 2) rect(ctx, r.x - hw + 2, r.y - 1, 3, 1, "#4a4766"); });
        for (const d of r.drops) rect(ctx, d.x, d.y, d.s, d.s, d.hi ? "#4a4766" : "#17151f");
        ctx.globalAlpha = 1;
      }]);
      for (const p of r.parts) items.push([p.y, () => drawPart(ctx, p, fade)]);
      if (!r.parts.length || r.back) continue;
      const xs = r.parts.map((p) => p.x), ys = r.parts.map((p) => p.y);
      const h = hot(Math.min(...xs) - 4, Math.min(...ys) - 10, Math.max(...xs) + 4, Math.max(...ys) + 3, { kind: "unshoot", id });
      if (h) tip = { text: r.b.name + " - put back together", x: r.x, y: Math.min(...ys) - 12 };
    }
    if (L.pong) {
      piece(L.pong, "pong", (c) => drawPong(c, L.pong, w));
      if (hot(L.pong.x - 16, L.pong.y - 17, L.pong.x + 16, L.pong.y, { kind: "pong" })) tip = { text: "ping-pong", x: L.pong.x, y: L.pong.y - 18 };
    }
    // the bench: idle bots sit on it a while (a bot is clicked as anywhere else); a click on the bench
    // itself lists the completed sessions (agent view's Completed group), to open one or delete it
    const completed = bots.filter((b) => b.over).length;
    for (const bn of L.benches) {
      const h = hot(bn.x - 29, bn.y - 18, bn.x + 29, bn.y + 1, { kind: "completed" });
      items.push([bn.y - 0.5, () => outlined(ctx, foot("bench", [bn.x, bn.y]), lit(bn.key) || danger(bn.key), (c) => drawBench(c, bn), danger(bn.key) ? RED : GOLD, tintOf(bn.key))]);
      pics.push({ key: bn.key, kind: "bench", x: bn.x, y: bn.y, fn: (c) => drawBench(c, bn) });
      if (h) tip = { text: completed ? `completed ${completed}` : "completed", x: bn.x, y: bn.y - 20 };
    }
    // the limits, in the coffee pot (this five-hour session) and the cooler's bottle (this week); a
    // click on either opens the whole picture
    const c0 = L.coffeeAt;
    if (c0 && hot(c0.x, c0.y - 22, c0.x + 20, c0.y + 1, { kind: "limits" })) tip = { text: say(usage.session, "5h"), x: c0.x + 10, y: c0.y - 24 };
    if (L.cooler && hot(L.cooler.x - 5, L.cooler.y - 26, L.cooler.x + 5, L.cooler.y + 1, { kind: "limits" })) tip = { text: say(usage.week, "1w"), x: L.cooler.x, y: L.cooler.y - 24 };
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
      const top = (h) => Math.round(h.y) - (h.lying ? 20 : 29), UP = 6;
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
    // shots (hardcore): a flash where it hit (or at the barrel), a puff of smoke after
    if (fps && !hardcore()) fps.leave();
    w.shots = w.shots.filter((sh) => performance.now() - sh.t0 < 700);
    for (const sh of w.shots) {
      const t = (performance.now() - sh.t0) / 700;
      if (t < 0.3) { const r = t < 0.15 ? 4 : 3; rect(ctx, sh.x - r, sh.y, 2 * r + 1, 1, "#fff6c0"); rect(ctx, sh.x, sh.y - r, 1, 2 * r + 1, "#fff6c0"); rect(ctx, sh.x - 1, sh.y - 1, 3, 3, "#ffe27a"); }
      else { ctx.globalAlpha = 1 - t; rect(ctx, sh.x - 2, sh.y - 2 - Math.round(t * 6), 5, 3, "#c9ced8"); ctx.globalAlpha = 1; }
    }
    // the neuralyzer's flash: the room goes white a moment, brightest at the flasher (only that glow with
    // reduced motion)
    if (w.flash) {
      const t = (performance.now() - w.flash.t0) / 500;
      if (t >= 1) w.flash = null;
      else {
        if (!still) { ctx.globalAlpha = 0.5 * (1 - t); rect(ctx, 0, 0, W, H, "#ffffff"); }
        const r = Math.round(3 + t * 14);
        ctx.globalAlpha = 0.9 * (1 - t); rect(ctx, w.flash.x - r, w.flash.y - Math.round(r / 2), 2 * r, r, "#ffffff"); ctx.globalAlpha = 1;
      }
    }
    // talk, over everything: dots in a bubble, never the words; a bot that needs input asks, a ? in
    // its bubble for as long as it waits (the one glyph a bubble holds)
    for (const f of w.folk.values()) if (f.waiting && !f.gone && f.act.kind !== "doom") {
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - 30;      // a dot over its antenna, right of it
      rect(ctx, bx - 1, by - 1, 9, 9, INK); rect(ctx, bx, by, 7, 7, C.steel4); rect(ctx, bx, by + 7, 1, 1, C.steel4);
      tiny(ctx, "?", bx + 2, by + 1, INK);
    }
    for (const f of w.folk.values()) if (f.chat > 0 && !still && !f.waiting) {
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - (f.lying ? 20 : 27) - 6;
      rect(ctx, bx - 1, by - 1, 11, 7, INK); rect(ctx, bx, by, 9, 5, C.steel4); rect(ctx, bx, by + 5, 1, 1, C.steel4);
      for (let i = 0; i < 3; i++) if ((Math.floor(f.chat * 4) + i) % 3) rect(ctx, bx + 2 + i * 2, by + 2, 1, 1, INK);
    }
    // in the booth: a sign over it counts the seconds down; a click on it (or on the bot) is the undo, and
    // so is carrying the bot out
    for (const f of w.folk.values()) if (f.act.kind === "doom" && f.b && !f.gone) {
      const text = "bye " + f.b.name + " - undo", tw = tinyW(text), lx = Math.round(Math.max(2, Math.min(W - tw - 2, f.x - tw / 2)));
      const ly = f.y - 42 < 2 ? Math.round(f.y) + 5 : Math.round(f.y) - 42;     // over the booth's roof, or under it at the top of the room
      const h = hot(lx - 2, ly - 2, lx + tw + 2, ly + 9, { kind: "undo", id: f.id }) || (!!hovered && hovered.b.id === f.id);
      box(ctx, lx - 2, ly - 2, tw + 4, 11, h ? GOLD : INK2);
      tiny(ctx, text, lx, ly, h ? INK : C.steel4);
      rect(ctx, lx, ly + 7, Math.round(tw * Math.max(0, f.act.left) / 5), 1, h ? INK : RED);
    }
    // the bot under the pointer: its name over its head, and a click opens it
    // (one about to be stopped has its sign instead of a name, and a click on it is the undo)
    if (hovered && !moving) {
      const { b, x, y } = hovered, hf = w.folk.get(b.id), doomed = !!hf && hf.act.kind === "doom";
      const dg = app.digest(), wb = b.staff && dg && dg.on && dg.writing && !dg.writing.startsWith("u:") ? (all.find((x) => x.id === dg.writing) || {}).name : null;
      const cl = b.staff && w.clean && w.folk.get(w.clean.id);
      if (cl) tip = { text: b.name + (w.clean.hard ? " - after " : " - seeing out ") + cl.b.name, x, y: y - 2 };
      else if (b.staff) tip = { text: b.name + (dg && !dg.on ? " - off duty" : wb ? " - notes on " + wb : dg && dg.writing ? " - a project's notes" : " - notes"), x, y: y - 2 };
      else if (!doomed) tip = { text: b.name + (b.waiting ? " - needs input" : ""), x, y: y - 2 };
      hots.push({ x0: -1e9, y0: -1e9, x1: 1e9, y1: 1e9, action: b.staff ? { kind: "notes" } : b.human ? { kind: "you" } : doomed ? { kind: "undo", id: b.id } : { kind: "open", id: b.id } });
    }
    // (while something is dragged, the only tips are the fire's, the door's and the pair table's: what the drop does)
    if (tip && !carrying) {                  // no tips under a piece carried in from the closet
      const t = tip.text, tw = tinyW(t), lx = Math.round(Math.max(2, Math.min(W - tw - 2, tip.x - tw / 2))), ly = Math.round(Math.max(WALL + 2, tip.y - 8));
      box(ctx, lx - 2, ly - 2, tw + 4, 9, INK2);
      tiny(ctx, t, lx, ly, C.steel4);
    }
    const id = hovered && !held ? hovered.b.id : null;
    if (id !== hoverId) { hoverId = id; app.hover(id); }
    const sk = screenHover && !held && !moving ? screenHover.m + ":" + screenHover.dir : null;
    if (sk !== hoverScreen) { hoverScreen = sk; app.hoverShell(sk); }
    if (fps) view = scene3(L, w, s, T);
    const onHot = hots.some((h) => over(h.x0, h.y0, h.x1, h.y1));
    cv.style.cursor = moving || held ? "grabbing" : onHot ? "pointer" : pieceAt(pointer) ? "grab" : "default";
  }

  // ---- the office in the first person (fps.js), from what the room has just drawn: the floor and what lies on
  // it, the wall and what hangs on it, the desks as blocks (their faces cut from their own pictures), a sprite
  // for every piece of furniture, chair, screen, bot and bit of a shot bot, where it is in the room
  const canvases = new Map();
  function canvas(name, w, h) {              // one per name, redrawn in place (its `v` says it was)
    let c = canvases.get(name);
    if (!c || c.width !== w || c.height !== h) { c = document.createElement("canvas"); c.width = w; c.height = h; c.v = 0; canvases.set(name, c); }
    const x = c.getContext("2d", { willReadFrequently: true });
    x.setTransform(1, 0, 0, 1, 0, 0); x.clearRect(0, 0, w, h); x.imageSmoothingEnabled = false;
    c.v++;
    return [c, x];
  }
  const sides = new Map();                   // a desk's ends and back: its paint, outlined
  function sideOf(p) {
    if (!sides.has(p)) {
      const c = document.createElement("canvas"); c.width = c.height = 8;
      const x = c.getContext("2d"); rect(x, 0, 0, 8, 8, p.k); rect(x, 1, 1, 6, 6, p.d); rect(x, 1, 1, 6, 1, p.hi);
      sides.set(p, c);
    }
    return { c: sides.get(p), sx: 0, sy: 0, sw: 8, sh: 8 };
  }
  const backs = new WeakMap();               // a figure on its back, head to the left, as drawSleeper turns it
  function onItsBack(im) {
    if (!backs.has(im)) {
      const c = document.createElement("canvas"); c.width = SH; c.height = SW;
      const x = c.getContext("2d"); x.translate(0, SW); x.rotate(-Math.PI / 2); x.drawImage(im, 0, 0);
      backs.set(im, c);
    }
    return backs.get(im);
  }
  const smalls = new Map();                  // cogs, bolts, a spring, the ball, the ? of a bot that waits
  function small(k, w, h, paint) {
    if (!smalls.has(k)) { const c = document.createElement("canvas"); c.width = w; c.height = h; paint(c.getContext("2d")); smalls.set(k, c); }
    return smalls.get(k);
  }
  function bitOf(p) {
    const turn = Math.floor(p.spin) % 2, g = p.kind === "cog" ? (p.big ? COG7 : COG) : null, n = g ? g.length : 4;
    return small(p.kind + (p.big ? 7 : "") + (p.char ? "c" : "") + turn, n, n, (x) => {
      if (g) g.forEach((row, j) => [...row].forEach((ch, i) => { if (ch === "#") rect(x, turn ? n - 1 - i : i, j, 1, 1, i === n >> 1 && j === n >> 1 ? INK : p.char ? "#3a3438" : j < 2 ? C.steel3 : C.steel2); }));
      else if (p.kind === "bolt") { rect(x, 0, 1, 3, 1, C.steel3); rect(x, 1, 2, 1, 2, C.steel2); }
      else for (let q = 0; q < 4; q++) rect(x, q % 2, q, 2, 1, C.steel3);
    });
  }
  // the walls as you see them up close: a rail a third of the way up, panels under it, seams over it
  function panelled(x, w) {
    rect(x, 0, 0, w, 1, "#33445c");
    rect(x, 0, 15, w, 9, "#243245"); rect(x, 0, 14, w, 1, "#3d4f68"); rect(x, 0, 15, w, 1, "#1d2737");
    for (let i = 0; i < w; i += 32) { rect(x, i, 1, 1, 13, "#223042"); rect(x, i + 16, 16, 1, 8, "#1d2737"); }
  }
  function scene3(L, w, s, T) {
    const things = [], boxes = [], solid = [];
    // the floor: the planks, the rug, the mats rolled out for a nap, the oil of the shot ones
    const [fc, fx] = canvas("floor", W, H);
    fx.drawImage(floor(), 0, 0);
    if (L.pong) drawRug(fx, L.pong);
    for (const f of w.folk.values()) if (f.act.kind === "sleep" && f.act.there && f.act.at) drawMat(fx, f.act.at);
    for (const r of w.remains.values()) {
      const rx = Math.round(r.puddle * r.parts.length / r.total);
      if (rx > 1) ELL(rx, 3, 5).forEach((hw, j) => rect(fx, r.x - hw, r.y - 2 + j, 2 * hw, 1, "#17151f"));
      for (const d of r.drops) rect(fx, d.x, d.y, d.s, d.s, d.hi ? "#4a4766" : "#17151f");
    }
    // the wall with the doors, the window, the clock, the closet, the switchboard; the other three plain
    const [wc, wx] = canvas("wall", W, WALL);
    wx.drawImage(floor(), 0, 0, W, WALL, 0, 0, W, WALL); panelled(wx, W);
    for (const fn of wallPics) fn(wx);
    const pc = canvases.get("plain") || (() => { const [c, x] = canvas("plain", 32, WALL); x.drawImage(floor(), 0, 0, 32, WALL, 0, 0, 32, WALL); panelled(x, 32); return c; })();
    // a desk: its cabinet and its desktop on legs as blocks, faced with what the room draws of them; its
    // screen (and laptops) standing on the cabinet, a chair a bot
    const chair = sprites.get("chair");
    for (const d of L.desks) {
      const { x0, y, w: dw, cab } = d, ox = x0 - 2, oy = y - 35;
      const [c, x] = canvas("d:" + d.key, dw + 4, 37);
      x.translate(-ox, -oy); drawDesk(x, d, w, T, new Set());
      const crop = (cx, cy, cw, ch) => ({ c, sx: cx - ox, sy: cy - oy, sw: cw, sh: ch }), side = sideOf(paintOf(d.m)), leg = (lx) => crop(lx, y - 4, 2, 4);
      boxes.push({ x0, y0: y - 12, x1: x0 + cab, y1: y, z0: 0, z1: 11, front: crop(x0, y - 11, cab, 11), top: crop(x0, y - 15, cab, 4), side, back: side });
      const tx = x0 + cab - 2;
      boxes.push({ x0: tx, y0: y - 11, x1: x0 + dw, y1: y - 4, z0: 5, z1: 9, front: crop(tx, y - 7, x0 + dw - tx, 4), top: crop(tx, y - 12, x0 + dw - tx, 5), side, back: side });
      for (const lx of [x0 + cab + 1, ...d.seats.map(([, sx]) => sx + 11), x0 + dw - 4]) for (const ly of [y - 6, y - 11])
        boxes.push({ x0: lx, y0: ly, x1: lx + 2, y1: ly + 2, z0: 0, z1: 5, front: leg(lx), back: leg(lx), side: leg(lx), top: side });
      solid.push([x0, y - 12, x0 + dw, y - 3]);
      const gw = 22 + d.trees.length * LAP, { sx, sy } = screenAt(d);
      things.push({ x: sx + gw / 2, y: y - 6, z: 10, ...crop(sx, sy, gw, 18), ax: gw / 2, ay: 18 });
      if (chair) for (const [, cx] of d.seats) things.push({ x: cx - 4 + chair.width / 2, y: y - 0.7, z: 0, c: chair, sx: 0, sy: 0, sw: chair.width, sh: chair.height, ax: chair.width / 2, ay: 17 });
    }
    // every other piece standing on the floor, as the room drew it a moment ago
    for (const p of pics) {
      const r = foot(p.kind, [p.x, p.y]), P = 1, cw = r[2] - r[0] + 2 * P, ch = r[3] - r[1] + 2 * P;
      const [c, x] = canvas("p:" + p.key, cw, ch);
      x.translate(P - r[0], P - r[1]); p.fn(x);
      const cx = (r[0] + r[2]) / 2;
      things.push({ x: cx, y: p.y - 3, z: 0, c, sx: 0, sy: 0, sw: cw, sh: ch, ax: cx - r[0] + P, ay: p.y + 1 - r[1] + P });
      solid.push([r[0] + 2, Math.max(r[1], p.y - 10), r[2] - 2, p.y + 2]);
    }
    // everyone but you (you are the eyes), walking, sitting, lying on a mat; a ? over one that waits for you
    for (const f of w.folk.values()) {
      if (f.id === "you" || f.held || !f.b) continue;
      const b = f.b, im = body(seedOf(b), colorOf(b), f.arms, f.legs, b.human, b.staff);
      if (!im) continue;
      const who = { key: f.id, bot: b.human ? null : b.id, staff: !!b.staff, name: b.name };
      if (f.lying) { things.push({ ...who, x: f.x, y: f.y - 9, z: 0, c: onItsBack(im), sx: 0, sy: 0, sw: SH, sh: SW, ax: SH / 2, ay: SW - 4 }); continue; }
      things.push({ ...who, x: f.x, y: f.y, z: 0, c: im, sx: 0, sy: 0, sw: SW, sh: SH, ax: 11, ay: 28, face: f.dir < 0 ? -1 : 1 });
      if (f.waiting && f.act.kind !== "doom") {
        const q = small("?", 9, 10, (x) => { rect(x, 0, 0, 9, 9, INK); rect(x, 1, 1, 7, 7, C.steel4); rect(x, 1, 8, 1, 1, C.steel4); rect(x, 0, 9, 3, 1, INK); tiny(x, "?", 3, 2, INK); });
        things.push({ key: "?" + f.id, x: f.x, y: f.y + 0.1, z: 28, c: q, sx: 0, sy: 0, sw: 9, sh: 10, ax: 4.5, ay: 10 });
      }
    }
    // what is left of a shot bot: its pieces where they fly and land
    for (const [id, r] of w.remains) r.parts.forEach((p, i) => {
      const who = { key: id + "/" + i, rem: r.back ? null : id, name: r.b.name, x: p.x, y: p.y, z: p.z, flip: Math.floor(p.spin) % 2 === 1 };
      if (p.img) things.push({ ...who, c: p.img, sx: p.sx, sy: p.sy, sw: p.sw, sh: p.sh, ax: p.sw / 2, ay: p.sh });
      else { const c = bitOf(p); things.push({ ...who, c, sx: 0, sy: 0, sw: c.width, sh: c.height, ax: c.width / 2, ay: c.height }); }
    });
    if (w.ball) things.push({ key: "ball", x: w.ball.x, y: w.ball.y, z: w.ball.h, c: small("ball", 3, 3, (x) => { rect(x, 0, 0, 3, 3, INK); rect(x, 1, 1, 1, 1, C.white); }), sx: 0, sy: 0, sw: 3, sh: 3, ax: 1.5, ay: 3 });
    return { v: (view ? view.v : 0) + 1, W, H, WALL, floor: fc, wall: wc, plain: pc, boxes, things, solid };
  }
  // a shot fired: the idle bots near it run off, hands up (the ones at work work on)
  function alarm(x, y) {
    const L = lastL;
    for (const f of world().folk.values()) {
      const d = Math.hypot(f.x - x, f.y - y) || 1;
      if (!L || !f.b || f.b.human || f.b.staff || f.work || f.held || f.leaving || f.open || DOOMED.has(f.act.kind) || f.act.kind === "escort" || d > 110) continue;   // (the ones let go of stay put)
      const k = rnd(40, 80);
      end(f);
      f.act = { kind: "loiter", x: Math.max(L.floor.x0, Math.min(L.floor.x1, f.x + (f.x - x) / d * k)), y: Math.max(L.floor.y0, Math.min(L.floor.y1, f.y + (f.y - y) / d * k)), left: rnd(2, 4), v: 60, panic: true };
    }
  }
  // the gun closet clicked (hardcore): a gun, and the office through your own eyes, from in front of the
  // closet, looking into the room; Esc and you are back, standing where you went
  function firstPerson(key) {
    if (fps || !lastL) return;
    const bo = lastL.booths.find((b) => b.key === key) || lastL.booths[0];
    pointer = null; world().hunt = null;      // a hunt under way in the room is yours to finish in here
    view = scene3(lastL, world(), app.state(), world().T);
    fps = startFps({
      scene: () => view,
      start: () => { const x = bo ? Math.max(14, Math.min(W - 14, bo.x)) : W / 2, y = bo ? Math.min(H - 8, bo.y + 24) : H / 2; return { x, y, a: Math.atan2((WALL + H) / 2 - y, W / 2 - x) }; },
      move: (x, y, a, walking) => { const f = world().folk.get("you"); if (f && !f.held) { f.x = x; f.y = y; f.dir = Math.cos(a) < 0 ? -1 : 1; f.walking = walking; } },
      shoot: (id) => { const f = world().folk.get(id); if (!f || f.held || !f.b || f.b.human || f.b.staff) return false; shoot(f); return true; },
      unshoot, alarm, tiny, tinyW,
      hands: () => {                         // your skin and your sleeves, as your figure wears them (its portrait's shoulders)
        const f = world().folk.get("you"), b = f && f.b, seed = b ? seedOf(b) : "you", col = b ? colorOf(b) : YOU, px = face(seed, col, true).px, i = (19 * 20 + 6) * 4;
        return { skin: pixelAvatar.skin(seed), clothes: px && px[i + 3] > 128 ? "#" + [px[i], px[i + 1], px[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join("") : col };
      },
    }, () => { fps = null; view = null; world().hunt = null; const f = world().folk.get("you"); if (f) end(f); });   // (drawn at the next tick: this can come from inside draw)
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
    const im = body(seedOf(b), colorOf(b), f.held ? "up" : f.arms, f.held ? "walk1" : f.legs, b.human, b.staff);
    // its outline: terracotta while the pointer is on it or on its session, faint while its session is open;
    // held over the fire or at its door (let go and it stops): a red outline and a red flashing wash
    const lit = hl.has(f.id) || (hovered && hovered.b.id === f.id) || (!hovered && over(x0 + 4, y0, x0 + 18, y + 1));
    const doomed = f.held && dangerNow;
    const ring = f.leaving ? null : doomed ? RED : lit ? GOLD : app.opens().includes(f.id) ? OPEN : null;
    if (im) {
      ctx.save();
      if (f.dir < 0) { ctx.translate(x0 + SW, 0); ctx.scale(-1, 1); }
      const dx = f.dir < 0 ? 0 : x0;
      if (ring) ctx.drawImage(halo(im, ring), dx, y0);
      ctx.drawImage(doomed ? tinted(im, RED, flashA(T)) : im, dx, y0);
      ctx.restore();
    }
    if (b.rc && !f.held && Math.floor(T) % 2 === 0) rect(ctx, f.dir < 0 ? x - 1 : x, y0 + 3, 1, 1, C.steel4);   // in the Claude app too: a signal light over its antenna, blinking slowly
    if ((f.act.kind === "sleep" || f.act.kind === "doze") && f.act.there) zzz(ctx, f, x + 2, y0 + 4);
    if (f.act.kind === "neuralyze" && f.act.stage === 2) for (let k = 0; k < 3; k++) {   // dazed: little stars round its head
      const a = T * 4 + k * 2.1; rect(ctx, x - 1 + Math.round(Math.cos(a) * 6), y0 + 3 + Math.round(Math.sin(a) * 2), 1, 1, SP.Y);
    }
    if (f.act.kind === "plant" && f.act.water && Math.floor(T * 6) % 2) { rect(ctx, x + 10 * f.dir, y0 + 21, 1, 1, C.blue3); rect(ctx, x + 11 * f.dir, y0 + 24, 1, 1, C.blue3); }
    if (!f.leaving && !f.held && over(x0 + 4, y0, x0 + 18, y + 1)) hovered = { b, x, y: y0 };
  }
  function drawSleeper(ctx, f, b, T, hl) {  // lying on its back on the mat, head to the left: the standing figure turned on its side
    if (!b) return;
    const x = Math.round(f.x), y = Math.round(f.y), im = body(seedOf(b), colorOf(b), "down", "stand", b.human);
    const on = over(x - 15, y - 17, x + 15, y - 1), ring = hl.has(f.id) || on ? GOLD : app.opens().includes(f.id) ? OPEN : null;
    if (im) {
      ctx.save(); ctx.translate(x, y - 9); ctx.rotate(-Math.PI / 2);      // a quarter turn: the sprite's top goes left
      if (ring) ctx.drawImage(halo(im, ring), -SW / 2, -14);
      ctx.drawImage(im, -SW / 2, -14);
      ctx.restore();
    }
    zzz(ctx, f, x - 12, y - 22);
    if (on) hovered = { b, x, y: y - 20 };
  }
  const zed = (ctx, x, y) => { ctx.fillRect(x, y, 3, 1); ctx.fillRect(x + 1, y + 1, 1, 1); ctx.fillRect(x, y + 2, 3, 1); };
  function zzz(ctx, f, x, y) {
    ctx.fillStyle = C.steel3;
    for (let i = 0; i < 2; i++) { const k = (f.z * 0.4 + i * 0.5) % 1; zed(ctx, Math.round(x + k * 6), Math.round(y - k * 10)); }
  }

  // a unit: the cabinet with the project's name and screen, the long desktop where its bots sit
  const screenAt = (d) => ({ sx: Math.round(d.x0 + (d.cab - (22 + d.trees.length * LAP)) / 2), sy: d.y - 32 });   // the project's screen on its cabinet
  function drawDesk(ctx, d, w, T, hl, lit) {
    const { x0, y, w: dw, cab } = d, p = paintOf(d.m);
    // a screen runs only while a bot sits at it typing; dim while one sits there idle; a blinking
    // cursor while one waits for you. The big one is the project's own directory, a laptop a worktree's
    const seated = (f) => f && f.legs === "sit" && !f.lying && (f.act.kind === "desk" || f.act.kind === "sleep");
    const at = (tree) => d.seats.filter((st) => st[2] === tree).map(([id]) => w.folk.get(id)).filter(seated);
    // your terminal there (app.shells: one per screen, kept by the server): its prompt on the
    // screen's last row, the cursor blinking, a grey line running while output comes
    const shells = app.shells(), shellAt = (dir) => shells.find((s) => s.m === (d.m || "") && s.dir === dir);
    const hs = app.hlShell(), hsAt = (dir) => !!hs && hs.m === (d.m || "") && hs.dir === dir;   // its tile under the pointer
    function paint(g, gw, gh, tree, seed) {
      // off while nobody sits there, or while the sitter's process is stopped
      const f = at(tree).filter((x) => x.b && x.b.alive !== false), busy = f.some((x) => x.busy), waits = f.some((x) => x.waiting);
      const sh = shellAt(tree || d.dir), py = g.y + gh - 2;
      rect(ctx, g.x, g.y, gw, gh, f.length || sh ? C.blue0 : "#10141f");
      if (!f.length && !sh) { rect(ctx, g.x + gw - 3, g.y + 1, 1, 1, "#232c40"); rect(ctx, g.x + gw - 4, g.y + 2, 1, 1, "#232c40"); return; }   // off: a glint
      ctx.fillStyle = busy ? d.color : shade(d.color, 0.45);
      const s = busy ? Math.floor(T * 5) : 0;
      if (f.length) for (let r = 0; r < Math.floor(gh / 2) - (sh ? 1 : 0); r++) ctx.fillRect(g.x + 1 + ((r * 3) % 4), g.y + 1 + r * 2, Math.min(gw - 2, 3 + ((r * 7 + s + seed) * 5) % (gw > 10 ? 11 : 4)), 1);
      if (waits && !busy && Math.floor(T * 2) % 2) rect(ctx, g.x + 2, py - (sh ? 2 : 0), 2, 1, C.steel4);
      if (!sh) return;
      rect(ctx, g.x + 1, py, 2, 1, C.steel3);
      if (sh.busy) rect(ctx, g.x + 4, py, Math.min(gw - 6, 2 + ((Math.floor(T * 6) * 7) % (gw > 10 ? 9 : 3))), 1, C.steel2);
      else if (Math.floor(T * 2) % 2) rect(ctx, g.x + 4, py, 1, 1, C.steel4);
    }
    // the desktop, lower than the cabinet: a leg between each two chairs, a keyboard at each seat
    slab(ctx, x0 + cab - 2, y - 12, dw - cab + 2, 5, 4, p);
    for (const x of [x0 + cab + 1, ...d.seats.map(([, x]) => x + 11), x0 + dw - 4]) {
      rect(ctx, x, y - 4, 2, 4, p.k); rect(ctx, x, y - 4, 1, 3, p.d); rect(ctx, x, y, 2, 1, SHADOW);
    }
    for (const x of [...d.seats.map(([, x]) => x), ...(d.empty ? [d.empty] : [])]) { rect(ctx, x - 9, y - 11, 7, 2, SP.F); rect(ctx, x - 8, y - 11, 5, 1, SP.B); }
    for (const [id, x] of d.seats) {          // a coffee a colleague brought, by the keyboard, for a while
      const until = w.mugs.get(id), f = w.folk.get(id);
      if (until === undefined) continue;
      if (until < T || !f) { w.mugs.delete(id); continue; }
      rect(ctx, x - 13, y - 14, 3, 3, colorOf(f.b)); rect(ctx, x - 11, y - 13, 1, 1, INK);
    }
    // the cabinet, the project's name on a plate
    slab(ctx, x0, y - 15, cab, 4, 11, p);
    const lx = Math.round(x0 + (cab - tinyW(d.label)) / 2);
    plate(ctx, x0 + 2, y - 9, cab - 4, false, p); tiny(ctx, d.label, lx, y - 8, C.steel3);
    // on it the project's screen, and a laptop for each worktree, plugged in; the pointer on one
    // lights up the bots working there. The desk is handled at its screen: a + beside it starts a
    // bot here, a click on the screen (or a laptop) opens your terminal there
    const gw = 22 + d.trees.length * LAP, { sx, sy } = screenAt(d);
    // the path as a shell prompt shortens it (fish, starship): every directory but the last to its first
    // letter (two for a dotted one), the home to ~
    const abbrev = (p) => { const s = p.split("/"); return s.map((x, i) => (i < s.length - 1 && x.length > 1 && x !== "~" ? x.slice(0, x.startsWith(".") ? 2 : 1) : x)).join("/"); };
    const where = (dir) => (d.m ? machineName(d.m) + ":" : "") + abbrev(dir.replace(/^\/(Users|home)\/[^/]+/, "~"));
    const onScreen = hot(sx, sy, sx + 22, sy + 18, { kind: "shell", m: d.m || "", dir: d.dir });
    if (onScreen) { for (const [id] of d.seats) hl.add(id); tip = { text: where(d.dir), x: sx + 11, y: sy - 1 }; screenHover = { m: d.m || "", dir: d.dir }; }
    {
      const px = sx + gw + 2, h = hot(px - 3, sy - 3, px + 8, sy + 8, { kind: "spawn", m: d.m, dir: d.dir });
      ctx.fillStyle = h ? d.color : C.steel2; ctx.fillRect(px + 2, sy, 1, 5); ctx.fillRect(px, sy + 2, 5, 1);   // a 5 x 5 plus
      if (h) tip = { text: "new bot", x: px + 2, y: sy - 2 };
    }
    paint({ x: sx + 2, y: sy + 2 }, 18, 11, "", d.dir.length);
    const mon = sprites.get("monitor");
    if (onScreen || lit || hsAt(d.dir)) put(ctx, halo(mon, onScreen ? C.steel3 : GOLD), sx, sy);
    put(ctx, mon, sx, sy);
    if (d.trees.length) rect(ctx, sx + 17, y - 13, gw - 22, 1, SP.F);       // the cable, along the top
    d.trees.forEach((tree, k) => {
      const px = sx + 23 + k * LAP, py = y - 22, lap = sprites.get("laptop");
      const on = hot(px, py, px + 12, py + 11, { kind: "shell", m: d.m || "", dir: tree }), mine = d.seats.filter((st) => st[2] === tree);
      if (on) { for (const [id] of mine) hl.add(id); tip = { text: tree.split("/").pop() + ": a worktree", x: px + 6, y: py - 1 }; screenHover = { m: d.m || "", dir: tree }; }
      paint({ x: px + 2, y: py + 2 }, 8, 5, tree, tree.length);
      if (on || hsAt(tree) || mine.some(([id]) => hl.has(id))) put(ctx, halo(lap, on ? C.steel3 : GOLD), px, py);
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
    if (lit) put(ctx, halo(im, GOLD), x - 4, y - 16);
    put(ctx, im, x - 4, y - 16);
  }
  // a desk carried to a door is on its way out: a sign over the door asks; a click on it takes
  // the desk away (its bots must be gone first)
  // the fireplace: a fire pit in the middle of the room, seen from above like the tables: a ring of
  // stones on the floor, embers and logs in it, the flames rising from it. What is dragged into it
  // burns: a desk (its bots must be gone first), a piece of furniture, a bot (stopped, after a
  // countdown). `burst` (0..1) is a shower of sparks: the pointer stirred it, or something burnt
  // what is carried is over the fire when its middle is: a piece only touching it at an edge (put down
  // beside the pit, and so picked up again) is not about to burn
  const inFire = (L, key, at, u) => {
    if (!L.fire || (!key.startsWith("u:") && fixed(key))) return false;
    const r = foot(key, at, u), cx = (r[0] + r[2]) / 2, cy = (r[1] + r[3]) / 2;
    return cx >= L.fire.x0 && cx < L.fire.x1 && cy >= L.fire.y0 && cy < L.fire.y1 + 4;
  };
  // a held figure hangs 24 dots under the pointer, so what it is dropped on is found by where it
  // stands, not by the pointer: `spot` is its feet, a little up; `dropAt` is where a bot would be
  // stopped if let go: in the fire pit (standing in it or just in front of it), in its own doorway, or on
  // the bench (a running one; one that is over goes nowhere but the fire)
  const spot = (f) => ({ x: f.x, y: f.y - 4 });
  function dropAt(L, f) {
    if (!L || !f.b) return null;
    if (L.fire && f.x >= L.fire.x0 && f.x < L.fire.x1 && f.y >= L.fire.y0 && f.y < L.fire.y1 + 6) return "fire";
    return !hardcore() && boothAt(L, f) ? "booth" : null;      // the gun closet takes no one in
  }
  const boothAt = (L, f) => L.booths.find((bo) => inRect(spot(f), foot("booth", [bo.x, bo.y]))) || null;
  const ELL = (rx, ry, rows) => [...Array(rows)].map((_, j) => Math.round(rx * Math.sqrt(Math.max(0, 1 - ((j + 0.5 - rows / 2) / (rows / 2)) ** 2))));
  function drawFire(ctx, f, T, feeding, burst = -1, burning = []) {
    // every measure of the 32 x 28 original scaled by FIRE.S (X), so the pit keeps its proportions
    const { x0, y0 } = f, S = FIRE.S, X = (v) => Math.round(v * S), cx = x0 + X(16), r = (y, x, w, c) => rect(ctx, cx + x, y0 + y, w, 1, c);
    const ST = { k: "#4d4a4f", m: "#8d8a86", hi: "#aaa59e", d: "#6e6b6a" };    // the stones
    // the ring of stones at the bottom, lit along its far side, in shade on the near one
    const rows = X(12), top = ELL(X(15), X(6), rows), ty = X(16);
    top.forEach((hw, j) => { r(ty + j, -hw, hw * 2, ST.k); if (hw > 2) r(ty + j, -hw + 1, hw * 2 - 2, j < rows / 3 ? ST.hi : j > rows * 2 / 3 ? ST.d : ST.m); });
    for (const [x, y] of [[-12, 20], [-6, 17], [2, 17], [9, 18], [12, 22], [-10, 25], [-1, 26], [6, 26], [11, 25], [-13, 23], [-3, 17], [14, 20], [3, 27], [-8, 27]]) r(X(y), X(x), 1, ST.k);
    ELL(X(12), X(4), X(8)).forEach((hw, j) => r(X(18) + j, -hw, hw * 2, "#120a0c"));
    r(X(28), -X(14), X(28), SHADOW);
    // what was thrown in: tossed over to the pit in an arc, then it sinks into it, the flames in
    // front of it (what is below the pit's floor is not seen)
    const floorY = y0 + X(23);
    for (const o of burning) {
      // anything wider than the pit shrinks to fit it as it flies in (a desk would be cropped by the pit's clip)
      const t = (performance.now() - o.t0) / 1000, fit = Math.min(1, X(24) / o.w), sw = Math.round(o.w * fit), sh = Math.round(o.h * fit), tx = cx - Math.round(sw / 2), oy = floorY - sh;
      if (t < 0.35) {
        const k = t / 0.35, e = k * (2 - k), s = 1 + (fit - 1) * e;
        ctx.drawImage(o.img, Math.round(o.x + (tx - o.x) * e), Math.round(o.y + (oy - o.y) * e - Math.sin(k * Math.PI) * X(12)), Math.round(o.w * s), Math.round(o.h * s));
      } else {
        const k = Math.min(1, (t - 0.35) / 0.8);
        ctx.save(); ctx.beginPath(); ctx.rect(x0 + X(3), y0 - 40, X(26), floorY - (y0 - 40)); ctx.clip();
        ctx.drawImage(o.img, tx, oy + Math.round(k * k * (sh + 4)), sw, sh);
        ctx.restore();
      }
    }
    const k = Math.floor(T * 6) % 3, big = feeding ? X(4) : burning.length ? X(3) : 0;
    // embers and two logs
    for (const [x, y] of [[-9, 21], [-7, 23], [7, 20], [9, 23], [-3, 24], [4, 24], [-11, 22], [11, 21], [0, 22]]) r(X(y), X(x), 1, (k + x) % 2 ? C.red1 : "#7a2a18");
    r(X(22), -X(7), X(6), "#3b2a1e"); r(X(23), -X(8), X(7), "#5c3a26"); r(X(22), X(1), X(6), "#3b2a1e"); r(X(23), X(2), X(6), "#5c3a26");
    // the flames, up from the embers: a tongue a dot, tallest in the middle, of three heights a frame;
    // red, gold in them, yellow at the heart
    const base = X(22), hw = X(7), hmax = X(18), flame = (x, h, c) => rect(ctx, cx + x, y0 + base - h, 1, h, c);
    const F = [];
    for (let x = -hw; x <= hw; x++) F.push([x, Math.round(X(15) - X(13) * Math.abs(x) / hw) + (k + Math.abs(x)) % 3]);
    for (const [x, h] of F) flame(x, Math.min(hmax, h + big), C.red1);
    for (const [x, h] of F) flame(x, Math.max(1, Math.round(Math.min(hmax, h + big) * 0.62)), C.gold1);
    for (const [x, h] of F) if (Math.abs(x) < X(4)) flame(x, Math.max(1, Math.round(Math.min(hmax, h + big) * 0.3)), C.gold2);
    if (burst >= 0) for (let i = 0; i < 14; i++) {      // sparks, out and up, dying out
      const a = (i * 2.399) % 6.283, sp = 0.5 + ((i * 7919) % 100) / 200, t = burst * sp * 1.6;
      if (t > 1) continue;
      const sx = Math.round(Math.sin(a) * X(16) * t), sy = Math.round(X(22) - X(8) * Math.cos(a) * t - X(26) * t);
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
  // the whiteboard on wheels, on the floor: its face 30 x 15, two posts, a base on four wheels. What is
  // drawn on it is ink at five pixels a board dot (`w.ink`, 130 x 55): the host of a meeting draws
  // boxes, lines and dots in its colour, you draw on it big in the view. A board dot is inked when any
  // of its pixels is (the colour of the first)
  const INK_AREA = { x: 2, y: 2, w: 26, h: 11 }, INKS = 5;
  function ink(w) {
    if (!w.ink) { w.ink = document.createElement("canvas"); w.ink.width = INK_AREA.w * INKS; w.ink.height = INK_AREA.h * INKS; w.inkV = 0; }
    return w.ink;
  }
  function inkLine(w, x0, y0, x1, y1, col, width = 1) {
    const c = ink(w).getContext("2d");
    c.fillStyle = col;
    const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    let e = dx + dy, x = x0, y = y0;
    for (;;) {
      c.fillRect(x, y, width, width);
      if (x === x1 && y === y1) break;
      const e2 = 2 * e;
      if (e2 >= dy) { e += dy; x += sx; }
      if (e2 <= dx) { e += dx; y += sy; }
    }
    w.inkV++; w.cells = null;
  }
  function wipe(w) { ink(w).getContext("2d").clearRect(0, 0, ink(w).width, ink(w).height); w.inkV++; w.cells = null; w.strokes = 0; }
  const inked = (w) => { const d = ink(w).getContext("2d").getImageData(0, 0, ink(w).width, ink(w).height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i] > 128) n++; return n / (d.length / 4); };
  function cells(w) {                        // the board's dots from the ink, kept until the ink changes
    if (w.cells) return w.cells;
    const im = ink(w), d = im.getContext("2d").getImageData(0, 0, im.width, im.height).data, out = [];
    for (let j = 0; j < INK_AREA.h; j++) for (let i = 0; i < INK_AREA.w; i++) {
      let col = null;
      for (let y = j * INKS; y < (j + 1) * INKS && !col; y++) for (let x = i * INKS; x < (i + 1) * INKS; x++) {
        const k = (y * im.width + x) * 4;
        if (d[k + 3] > 128) { col = "#" + [d[k], d[k + 1], d[k + 2]].map((v) => v.toString(16).padStart(2, "0")).join(""); break; }
      }
      if (col) out.push([i, j, col]);
    }
    return (w.cells = out);
  }
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
    if (w.ink) for (const [i, j, col] of cells(w)) rect(ctx, ix + i, iy + j, 1, 1, col);
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
  // a piece drawn into a canvas of its footprint (plus a little around)
  function snapshot(r, fn, pad = 2) {
    const c = document.createElement("canvas");
    c.width = r[2] - r[0] + 2 * pad; c.height = r[3] - r[1] + 2 * pad;
    const o = c.getContext("2d");
    o.imageSmoothingEnabled = false; o.translate(pad - r[0], pad - r[1]);
    fn(o);
    return c;
  }
  // a piece put with an outline around it (the pointer is on it, or it is in danger), or drawn straight;
  // `tint` (0..1) washes it in the outline's colour as well: red, flashing, for what is about to burn
  function outlined(ctx, r, on, fn, col = GOLD, tint = 0) {
    if (!on) { fn(ctx); return; }
    const pad = 2, c = snapshot(r, fn, pad);
    ctx.drawImage(halo(c, col), r[0] - pad, r[1] - pad); ctx.drawImage(tint ? tinted(c, col, tint) : c, r[0] - pad, r[1] - pad);
  }
  // A killed bot (shot: your crosshair or the office manager's shotgun; or put in the fire) leaves remains,
  // not a gravestone: its own picture cut into head, chest, arms and legs, cogs, bolts and a spring, flying
  // apart with a hop and bouncing to a stop on a splash of oil (a burnt one: a few charred cogs and bolts by
  // the pit). They are its undo: a click on any of it puts it back together (the pieces fly back in). Idle
  // bots come over and clear them away, a piece at a time (`sweep`); with the last piece gone the session is
  // stopped and deleted (`claude stop`, `claude rm`). With nobody to clear them they rust away after half a
  // minute, and it goes then
  const RUST = { after: 30, fade: 2 };
  const SWEEPERS = new Set(["stand", "loiter", "plant", "window", "stretch", "warm", "watch", "visit", "chair", "bench", "coffee", "print"]);   // breaks a bot leaves to clear up
  const COG = [".#.#.", "#####", "##.##", "#####", ".#.#."], COG7 = ["..#.#..", ".#####.", "###.###", "##...##", "###.###", ".#####.", "..#.#.."];
  function scatter(f, at, charred) {
    const w = world(), x = Math.round(at[0]), y = Math.max(WALL + 14, Math.round(at[1]));
    const im = body(seedOf(f.b), colorOf(f.b), "down", "stand", false), cut = (sx, sy, sw, sh, kind) => ({ kind, img: im, sx, sy, sw, sh });
    const kinds = (charred || !im ? [] : [cut(1, 1, 20, 17, "head"), cut(6, 18, 10, 5, "chest"), cut(4, 18, 3, 5, "arm"), cut(15, 18, 3, 5, "arm"), cut(7, 23, 4, 6, "leg"), cut(11, 23, 4, 6, "leg")])
      .concat([{ kind: "cog", big: true }, { kind: "cog" }, { kind: "cog" }, { kind: "bolt" }, { kind: "bolt" }, { kind: "bolt" }], charred ? [] : [{ kind: "cog" }, { kind: "spring" }]);
    const parts = kinds.map((p) => {
      const a = Math.random() * Math.PI * 2, sp = charred ? 8 + Math.random() * 14 : 16 + Math.random() * 30;
      return { ...p, char: !!charred, x: x + (Math.random() - 0.5) * 4, y: y - 1 + (Math.random() - 0.5) * 3, z: charred ? 6 : 8 + Math.random() * 10,
        vx: Math.cos(a) * sp, vy: Math.sin(a) * sp * 0.55, vz: (charred ? 25 : 45) + Math.random() * 45, spin: Math.random() * 4 };
    });
    // the oil: a puddle where it stood, splashed all round, a few streaks flung out (none from a burnt one)
    const drop = (r, s) => { const a = Math.random() * 6.28; return { x: Math.round(x + Math.cos(a) * r), y: Math.round(y + Math.sin(a) * r * 0.5), s, hi: Math.random() < 0.15 }; };
    const drops = charred ? [] : [...Array(60)].map(() => drop(Math.random() ** 0.6 * 22, Math.random() < 0.35 ? 2 : 1))
      .concat([...Array(5)].flatMap(() => { const a = Math.random() * 6.28; return [...Array(6)].map((_, k) => ({ x: Math.round(x + Math.cos(a) * (10 + k * 3)), y: Math.round(y + Math.sin(a) * (10 + k * 3) * 0.5), s: k < 3 ? 2 : 1 })); }));
    if (still) for (const p of parts) { p.x += p.vx * 0.4; p.y += p.vy * 0.4; p.z = p.vx = p.vy = p.vz = 0; }
    w.remains.set(f.id, { b: f.b, x, y, t0: performance.now(), parts, drops, puddle: charred ? 0 : 7 + Math.round(Math.random() * 3), total: parts.length, back: 0 });
    w.folk.delete(f.id);
  }
  function bury(f, L) {                       // into the fire: its picture into the flames, charred bits beside the pit
    const F = L.fire, im = body(seedOf(f.b), colorOf(f.b), "up", "walk1", false);
    if (im) toFire([Math.round(f.x) - 11 + 2, Math.round(f.y) - 27 + 2, 0, 0], im);
    scatter(f, [F.x1 > W - 30 ? F.x0 - 8 : F.x1 + 8, F.y1 - 4], true);
  }
  function shoot(f) {                         // a flash where it hits, and the pieces
    world().shots.push({ x: Math.round(f.x), y: Math.round(f.y) - 14, t0: performance.now(), hit: true });
    scatter(f, [f.x, f.y], false);
  }
  function unshoot(id) {                      // put back together: the pieces fly in, and it stands up there
    const r = world().remains.get(id);
    if (!r || r.back) return;
    r.back = performance.now();
    for (const p of r.parts) { p.bx = p.x; p.by = p.y; p.bz = p.z; }
  }
  // the remains move: flying pieces fall and bounce and slide to a stop; put back together, they fly in
  function stepRemains(w, dt, L) {
    const now = performance.now();
    for (const [id, r] of w.remains) {
      if (r.back) {
        const t = (now - r.back) / 450;
        if (t >= 1) { w.remains.delete(id); w.spared.add(id); w.riseAt.set(id, { x: r.x, y: r.y }); continue; }   // the office manager leaves it alone from now on
        for (const p of r.parts) { p.x = p.bx + (r.x - p.bx) * t; p.y = p.by + (r.y - p.by) * t; p.z = p.bz + (12 - p.bz) * t; }
        r.drops = r.drops.filter(() => Math.random() > dt * 4);
        continue;
      }
      const age = (now - r.t0) / 1000;
      if (age > RUST.after + RUST.fade) { gone(w, id); continue; }
      for (const p of r.parts) {
        if (p.z > 0 || p.vz > 0) {             // in the air
          p.x += p.vx * dt; p.y += p.vy * dt; p.z += p.vz * dt; p.vz -= 240 * dt; p.spin += dt * 10;
          // down: two bounces at most (in steps of a twelfth of a second a small hop never dies out by itself)
          if (p.z <= 0) { p.z = 0; p.hops = (p.hops || 0) + 1; if (p.hops <= 2 && p.vz < -40) { p.vz = -p.vz * 0.35; p.vx *= 0.6; p.vy *= 0.6; } else p.vz = 0; }
        } else if (p.vx || p.vy) {             // sliding to a stop
          p.x += p.vx * dt; p.y += p.vy * dt; p.vx *= Math.max(0, 1 - 7 * dt); p.vy *= Math.max(0, 1 - 7 * dt);
          if (Math.abs(p.vx) + Math.abs(p.vy) < 2) p.vx = p.vy = 0;
        }
        p.x = Math.max(4, Math.min(W - 4, p.x)); p.y = Math.max(WALL + 4, Math.min(L.floor.y1 + 4, p.y));
      }
    }
  }
  function gone(w, id) {                      // the last piece cleared away (or rusted): out of the list for good
    w.remains.delete(id); w.stopped.set(id, performance.now()); app.act({ kind: "stop", id, rm: true });
  }
  function drawPart(ctx, p, fade) {
    const x = Math.round(p.x), y = Math.round(p.y), z = Math.round(p.z);
    if (fade) ctx.globalAlpha = 1 - fade;
    if (z > 1) rect(ctx, x - 2, y, 4, 1, SHADOW);
    if (p.img) {
      const fl = Math.floor(p.spin) % 2 === 1;
      ctx.save(); ctx.translate(x, y - z);
      if (fl) ctx.scale(-1, 1);
      ctx.drawImage(p.img, p.sx, p.sy, p.sw, p.sh, -Math.round(p.sw / 2), -p.sh, p.sw, p.sh);
      ctx.restore();
    } else if (p.kind === "cog") {
      const g = p.big ? COG7 : COG, n = g.length, turn = Math.floor(p.spin) % 2;
      g.forEach((row, j) => [...row].forEach((ch, i) => { if (ch === "#") rect(ctx, x - (n >> 1) + (turn ? n - 1 - i : i), y - z - n + j, 1, 1, i === (n >> 1) && j === (n >> 1) ? INK : p.char ? "#3a3438" : j < 2 ? C.steel3 : C.steel2); }));
    } else if (p.kind === "bolt") { rect(ctx, x - 1, y - z - 3, 3, 1, C.steel3); rect(ctx, x, y - z - 2, 1, 2, C.steel2); }
    else if (p.kind === "spring") for (let k = 0; k < 4; k++) rect(ctx, x - 1 + (k % 2), y - z - 4 + k, 2, 1, C.steel3);
    ctx.globalAlpha = 1;
  }
  // a picture washed in a colour, its own pixels only (the danger's red over what is held over the fire)
  function tinted(im, col, a) {
    const c = document.createElement("canvas"); c.width = im.width; c.height = im.height;
    const x = c.getContext("2d");
    x.drawImage(im, 0, 0);
    x.globalCompositeOperation = "source-atop"; x.globalAlpha = a; x.fillStyle = col; x.fillRect(0, 0, c.width, c.height);
    return c;
  }
  // the danger's flash: the wash goes between two strengths, five times a second
  function flashA(T) { return Math.floor(T * 5) % 2 ? 0.5 : 0.3; }
  // something goes into the fire: a picture of it, to be tossed in and sink (drawFire)
  function toFire(r, img) { world().burning.push({ img, x: r[0] - 2, y: r[1] - 2, w: img.width, h: img.height, t0: performance.now() }); world().flare = performance.now(); }
  // any kind of furniture drawn at a spot, at rest: for the closet's pictures and for the ghost of
  // a piece carried into the room (`at` as the layout keeps it: its anchor)
  function drawKind(ctx, kind, [x, y], w, T) {
    const pt = { x, y };
    switch (kind) {
      case "plant": drawPlant(ctx, pt, 0, 0); break;
      case "armchair": drawArmchair(ctx, pt); break;
      case "coffee": drawCoffee(ctx, pt, false, T, null); break;
      case "cooler": drawCooler(ctx, pt, null); break;
      case "printer": drawPrinter(ctx, pt, undefined); break;
      case "chess": drawChess(ctx, pt, null); for (const d of [-13, 13]) drawStool(ctx, { x: x + d, y }); break;
      case "pong": drawRug(ctx, pt); drawPong(ctx, pt, w); break;
      case "meet": drawTable(ctx, pt); for (const d of [-23, 23]) drawStool(ctx, { x: x + d, y }); break;
      case "board": drawBoard(ctx, pt, { ink: null }); break;
      case "window": drawWindow(ctx, { x0: x, y0: y, x1: x + 21, y1: y + 18 }); break;
      case "clock": drawClock(ctx, x + 6, y + 5); break;
      case "fire": drawFire(ctx, { x0: x, y0: y - FIRE.h, x1: x + FIRE.w, y1: y }, T, false); break;
      case "closet": drawCloset(ctx, { x0: x, y0: y }, false); break;
      case "settings": drawSwitchboard(ctx, { x0: x, y0: y }, {}, false); break;
      case "manager": drawManagerDesk(ctx, pt, 3, false); break;
      case "pair": drawPair(ctx, pt, true, T, false); break;
      case "add": drawAdd(ctx, { x0: x, y0: y - 15, x1: x + ADD_W, y1: y }, false); break;
      case "bench": drawBench(ctx, pt); break;
      case "booth": drawBooth(ctx, pt, false, false, -1); drawBooth(ctx, pt, true, false, -1); break;
      case "neuralyzer": drawNeuralyzer(ctx, pt, false); break;
      case "door": { const ms = (app.state() || {}).machines || []; drawDoor(ctx, x, PAINT[Math.max(0, ms.length - 1) % PAINT.length], false, false, y); break; }   // in the paint the next machine gets
    }
  }
  // the printer the report comes out of, in the pane: a wide-carriage line printer drawn to the paper's
  // width (`paper` pixels) plus three dots a side, `S` pixels a dot: its slot (row 1, dark, one dot wider
  // than the paper each side) is where the sheet rises from; the body, a front opening, a lamp, a stand
  // (the page shows its top rows, the rest crosses the pane's bottom edge)
  function printerPicture(paper, S) {
    const w = Math.ceil(paper / S) + 6, f = (a, mid, b) => a + mid.repeat(w - a.length - b.length) + b;
    const rows = [f(".", "F", "."), f("FB", "x", "BF"), f("F", "B", "F"), f("F", "C", "F"), f("F", "C", "gCF"), f("F", "C", "F"), f("FC", "x", "CF"), f("F", "C", "F"), f("F", "D", "F"), f(".", "F", "."),
      f(".", "d", "."), f("d", "h", "d"), f("k", "d", "k"), f("k", "d", "k"), f("k", "k", "k"), f(".", "s", ".")];
    const c = document.createElement("canvas"); c.width = w * S; c.height = rows.length * S;
    const x = c.getContext("2d");
    rows.forEach((row, j) => [...row].forEach((ch, i) => { if (SP[ch]) { x.fillStyle = SP[ch]; x.fillRect(i * S, j * S, S, S); } }));
    return { src: c.toDataURL(), rows: rows.length, w };
  }
  // a picture of a kind of furniture, for the closet: a canvas, two pixels a dot as the room
  // a door in a paint (-1 the wood), for the doors' panel: its picture, S pixels a dot
  function doorPicture(i, S = 3) {
    const p = i < 0 ? WOOD : PAINT[i % PAINT.length], c = document.createElement("canvas");
    c.width = 16 * S; c.height = 20 * S;
    const ctx = c.getContext("2d");
    ctx.imageSmoothingEnabled = false; ctx.setTransform(S, 0, 0, S, 0, 0);
    put(ctx, sprites.swap("door", { h: p.hi, w: p.top, d: p.d, k: p.k }), 0, 0);
    return c.toDataURL();
  }
  function icon(kind, S = K, pad = 4) {
    const r = foot(kind, [0, 0]), c = document.createElement("canvas");
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
  // a new machine's door, let go on the wall from the closet: it hangs there while its form is open
  // (its machine added, until the state brings its door: `mid`)
  let pending = null;
  const pend = (at, mid) => { pending = at ? { at, mid, t: performance.now() } : null; draw(); };
  function carryKind(kind, cx, cy) {
    if (!kind) { carrying = null; draw(); return null; }
    const b = cv.getBoundingClientRect(), x = (cx - b.left) / K, y = (cy - b.top) / K;
    const inside = x >= 0 && x < W && y >= 0 && y < H;
    pointer = inside ? { x, y } : null;      // the room's pointer follows the carried piece (the canvas gets no events: the closet holds the pointer)
    carrying = { kind, at: inside ? settle(lastL, "new", clampAt(kind, [Math.round(x), Math.round(y)]), undefined, kind) : null };   // where it will land
    draw();
    return carrying.at;
  }
  function placeKind(kind, cx, cy) { const at = carryKind(kind, cx, cy); carrying = null; draw(); return at; }
  // a piece put down settles on clear floor: the nearest spot around `at` (rings outward) whose
  // footprint meets no other piece's, inside the room; a wall piece slides along
  // the wall. One rule for a piece moved, a desk moved and a piece out of the closet, so nothing ever
  // lands behind something else (where it could neither be seen nor picked up)
  function settle(L, key, at, u, kind = key) {
    if (!L) return at;
    const fits = (spot) => { const r = foot(kind, spot, u); return !L.pieces.some((p) => p.key !== key && meets(r, p.r)); };
    const wall = onWall(kind);
    // as far as the room goes (two dots a step close by, four further out): in a crowded corner nothing
    // within sixty dots may be clear, and giving up there put it on top of what was there
    for (let d = 0; d <= Math.max(W, H); d += d < 60 ? 2 : 4) {
      const s = d < 60 ? 2 : 4, ring = d ? [] : [[0, 0]];
      if (d && wall) ring.push([-d, 0], [d, 0]);
      else if (d) for (let t = -d; t < d; t += s) ring.push([t, -d], [d, t], [-t, d], [-d, -t]);
      ring.sort((a, b) => Math.hypot(...a) - Math.hypot(...b));
      for (const [dx, dy] of ring) { const spot = clampAt(kind, [at[0] + dx, at[1] + dy], u); if (fits(spot)) return spot; }
    }
    return clampAt(kind, at, u);
  }
  // burrow's own settings: a steel switchboard on the wall, a toggle a setting, its lever up and its
  // lamp lit while the setting is on (the first: new bots in the Claude app too); the lid stands open
  // while the panel is
  // three levers a column, a column more when there are more (the board stays above the floor)
  const SWITCHES = ["rc", "yolo", "digest", "cleanup", "hardcore"], SB_COLS = () => Math.ceil(SWITCHES.length / 3);
  const SB_H = () => 6 + 4 * Math.min(3, SWITCHES.length), SB_W = () => 4 + 10 * SB_COLS();
  function drawSwitchboard(ctx, b, cfg, open) {
    const { x0, y0 } = b, h = SB_H(), wd = SB_W();
    if (open) { rect(ctx, x0, y0 - 3, wd, 4, SP.F); rect(ctx, x0 + 1, y0 - 2, wd - 2, 2, SP.C); }   // the lid, flipped up
    rect(ctx, x0, y0, wd, h, SP.F);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, h - 2, SP.D);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, 1, SP.C); rect(ctx, x0 + 1, y0 + 1, 1, h - 2, SP.C);
    rect(ctx, x0 + 3, y0 + 3, wd - 6, h - 6, SP.E);
    SWITCHES.forEach((k, i) => {
      const on = !!cfg[k], x = x0 + 10 * Math.floor(i / 3), y = y0 + 3 + (i % 3) * 4;   // a lever every four dots
      rect(ctx, x + 4, y, 2, 4, SP.F); rect(ctx, x + 4, on ? y : y + 2, 2, 2, on ? SP.B : SP.D);   // the lever: up when on
      rect(ctx, x + 8, y + 1, 2, 2, on ? (k === "hardcore" ? SP.r : SP.g) : SP.F);                 // its lamp (red for hardcore)
    });
    rect(ctx, x0 + 2, y0 + h - 2, 1, 1, SP.F); rect(ctx, x0 + wd - 3, y0 + h - 2, 1, 1, SP.F);       // the screws
  }
  // the office manager's desk: a small writing desk, a stack of notes on it as high as there are
  // sessions with notes (six sheets at most), a pen; her chair to the right, where she sits facing it
  function drawManagerDesk(ctx, m, n, lit) {
    const x = m.x, y = m.y;
    slab(ctx, x, y - 12, 22, 5, 4, WOOD);
    for (const lx of [x + 1, x + 19]) { rect(ctx, lx, y - 4, 2, 4, WOOD.k); rect(ctx, lx, y - 4, 1, 3, WOOD.d); rect(ctx, lx, y, 2, 1, SHADOW); }
    const h = Math.min(6, n);
    if (h) { rect(ctx, x + 3, y - 12 - h, 8, h + 1, INK); for (let k = 0; k < h; k++) rect(ctx, x + 4, y - 12 - h + k, 6, 1, k % 2 ? SP.B : SP.A); }
    rect(ctx, x + 13, y - 11, 5, 1, C.blue2); rect(ctx, x + 17, y - 11, 1, 1, INK);      // the pen
    drawChair(ctx, x + 24, y, lit);
  }
  function drawDoor(ctx, x, p, ajar, boarded, y0 = WALL - 20) {
    put(ctx, sprites.swap(ajar ? "doorOpen" : "door", { h: p.hi, w: p.top, d: p.d, k: p.k }), x, y0);
    if (boarded) for (const y of [y0 + 4, y0 + 11]) {       // two planks nailed across it
      rect(ctx, x - 1, y, 18, 3, WOOD.k); rect(ctx, x, y + 1, 16, 1, WOOD.top); rect(ctx, x, y, 16, 1, WOOD.hi);
      rect(ctx, x + 1, y + 1, 1, 1, C.steel1); rect(ctx, x + 14, y + 1, 1, 1, C.steel1);
    }
  }
  // the clock's face: the sprite's light cells, found once
  let dial = null;
  function dialCells() {
    if (dial) return dial;
    const s = sprites.get("clock"), d = s.getContext("2d").getImageData(0, 0, s.width, s.height).data;
    dial = [];
    for (let j = 0; j < s.height; j++) for (let i = 0; i < s.width; i++) {
      const k = (j * s.width + i) * 4;
      if (d[k] === 0xed && d[k + 1] === 0xed && d[k + 2] === 0xed && d[k + 3] === 255) dial.push([i, j]);
    }
    return dial;
  }
  // Pixel Agents' clock, telling this computer's time. While it keeps the computer awake its face is
  // yellowed: for a while, a wedge from the hour hand to the hour it ends (shrinking as the time runs
  // out, as a kitchen timer); while local bots work, all of it; set to wait for them and none at work,
  // hatched
  function drawClock(ctx, x, y, a) {
    put(ctx, "clock", x - 6, y - 5);
    const t = new Date(), hour = ((t.getHours() % 12) + t.getMinutes() / 60) / 12 * 2 * Math.PI;
    if (a && a.mode !== "off") {
      const left = a.mode === "for" ? a.until - Date.now() : 0;
      const span = left > 0 ? Math.max(Math.PI / 12, left / 432e5 * 2 * Math.PI) : 0;   // 12 h round; never under half an hour, or the hands hide it
      ctx.fillStyle = SP.Y;
      for (const [i, j] of dialCells()) {
        // a cell the wedge reaches at all (its half width in angle past the end), so the last minutes still show
        const px = x - 6 + i, py = y - 5 + j, dx = px - (x - 1), dy = (y - py) / 0.75;
        const on = a.mode === "for" ? span > 0 && (Math.atan2(dx, dy) - hour + 4 * Math.PI) % (2 * Math.PI) < span + Math.atan2(0.5, Math.hypot(dx, dy))
          : a.on || (i + j) % 3 === 0;
        if (on) ctx.fillRect(px, py, 1, 1);
      }
    }
    const hand = (ang, len) => { for (let i = 1; i <= len; i++) ctx.fillRect(Math.round(x - 1 + Math.sin(ang) * i), Math.round(y - Math.cos(ang) * i * 0.75), 1, 1); };
    ctx.fillStyle = SP.G; hand(hour, 2);
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
  // a little mat rolled out on the floor for a nap: a thin flat one, 32 x 16, nothing more
  function drawMat(ctx, m) {
    rect(ctx, m.x - 16, m.y - 17, 32, 16, MAT.k);
    rect(ctx, m.x - 15, m.y - 16, 30, 14, MAT.top);
    rect(ctx, m.x - 15, m.y - 16, 30, 1, MAT.hi);
    rect(ctx, m.x - 15, m.y - 3, 30, 1, MAT.d);
  }
  // the booth (Futurama's): a tall glass cabin on a plinth, a lit sign on its roof. A bot put in it stands
  // behind the glass while its sign counts down (red while in use); then a soft light fills the cabin and it
  // is gone (`claude stop`, `claude rm`). Two layers: the cabin, and its glass in front of whoever is inside
  function drawBooth(ctx, bo, front, busy, flash) {
    const x0 = bo.x - 9, y = bo.y, top = y - 32, gy = top + 5, gh = y - 3 - gy;
    if (!front) {
      rect(ctx, x0 + 1, y, 16, 1, SHADOW);
      rect(ctx, x0, top, 18, 5, SP.F); rect(ctx, x0 + 1, top + 1, 16, 3, SP.E); rect(ctx, x0 + 1, top + 1, 16, 1, SP.D);   // the roof
      rect(ctx, x0 + 4, top + 1, 10, 3, INK); rect(ctx, x0 + 5, top + 2, 8, 1, busy ? SP.r : SP.Y);                      // its sign
      rect(ctx, x0, gy, 18, gh, SP.F); rect(ctx, x0 + 1, gy, 1, gh, SP.D); rect(ctx, x0 + 16, gy, 1, gh, SP.D);              // the posts
      rect(ctx, x0 + 2, gy, 14, gh, busy ? "#3b3428" : "#222a37");                                                       // inside, warm while in use
      rect(ctx, x0 + 2, y - 5, 14, 2, busy ? "#4a412f" : "#2b3442");                                                     // its floor
      rect(ctx, x0, y - 3, 18, 3, SP.F); rect(ctx, x0 + 1, y - 3, 16, 1, SP.C); rect(ctx, x0 + 1, y - 2, 16, 1, SP.D);       // the plinth
      return;
    }
    ctx.globalAlpha = 0.25; rect(ctx, x0 + 2, gy, 14, gh, "#bfe3ff"); ctx.globalAlpha = 1;                                // the glass
    for (let k = 0; k < 4; k++) { rect(ctx, x0 + 4 + k, gy + 4 + k, 1, 1, "rgba(255,255,255,0.5)"); rect(ctx, x0 + 10 + k, gy + 11 + k, 1, 1, "rgba(255,255,255,0.3)"); }
    rect(ctx, x0 + 2, gy, 14, 1, SP.D); rect(ctx, x0 + 13, gy + 10, 1, 4, SP.B);                                          // the door's rail and handle
    if (flash >= 0 && flash < 1) { ctx.globalAlpha = 0.95 * (1 - flash); rect(ctx, x0 + 2, gy, 14, gh, "#fff4d6"); ctx.globalAlpha = 1; }   // the light, fading
  }
  // the neuralyzer (Men in Black's): a black pedestal, the silver flasher on it. A bot put to it is flashed:
  // `/clear` in its session, a new conversation; it stands there dazed a moment
  // hardcore: the booth is a gun closet: an olive steel cabinet, two shotguns behind its glass doors (one
  // gone while it is in someone's hands), a red lamp on top
  function drawGunCloset(ctx, bo, front, out) {
    const x0 = bo.x - 9, y = bo.y, top = y - 32, gy = top + 5, gh = y - 3 - gy;
    if (!front) {
      rect(ctx, x0 + 1, y, 16, 1, SHADOW);
      rect(ctx, x0, top, 18, y - top, SP.F); rect(ctx, x0 + 1, top + 1, 16, 3, "#4f5844"); rect(ctx, x0 + 1, top + 1, 16, 1, "#6b7560");
      rect(ctx, x0 + 6, top + 1, 6, 3, INK); rect(ctx, x0 + 7, top + 2, 4, 1, SP.r);                     // the lamp
      rect(ctx, x0 + 1, gy, 16, gh, "#3c4434"); rect(ctx, x0 + 2, gy, 14, gh, "#1a1e16");                // the cabinet, inside
      for (const [k, gx] of [[0, x0 + 5], [1, x0 + 12]]) if (!(out && k === 1)) {
        rect(ctx, gx, gy + 2, 1, 13, "#a7aebb"); rect(ctx, gx + 1, gy + 2, 1, 13, "#6c7380");               // a barrel
        rect(ctx, gx - 1, gy + 14, 3, 7, "#7a4a2a"); rect(ctx, gx - 1, gy + 14, 1, 7, "#9a6a3a");           // its stock
      }
      rect(ctx, x0, y - 3, 18, 3, SP.F); rect(ctx, x0 + 1, y - 3, 16, 1, "#6b7560"); rect(ctx, x0 + 1, y - 2, 16, 1, "#4f5844");   // the plinth
      return;
    }
    ctx.globalAlpha = 0.18; rect(ctx, x0 + 2, gy, 14, gh, "#bfe3ff"); ctx.globalAlpha = 1;                // the glass
    for (let k = 0; k < 4; k++) rect(ctx, x0 + 3 + k, gy + 3 + k, 1, 1, "rgba(255,255,255,0.45)");
    rect(ctx, x0 + 8, gy, 2, gh, SP.F); rect(ctx, x0 + 7, gy + 10, 1, 3, SP.B); rect(ctx, x0 + 10, gy + 10, 1, 3, SP.B);   // where the doors meet, the handles
  }
  function drawNeuralyzer(ctx, n, lit) {
    const x = n.x, y = n.y;
    rect(ctx, x - 5, y, 10, 1, SHADOW);
    rect(ctx, x - 4, y - 12, 8, 12, SP.F); rect(ctx, x - 3, y - 12, 6, 11, "#2a2e35"); rect(ctx, x - 3, y - 12, 1, 11, SP.E);   // the pedestal
    rect(ctx, x - 6, y - 14, 12, 2, SP.F); rect(ctx, x - 5, y - 14, 10, 1, SP.C);                                          // its top
    rect(ctx, x - 5, y - 18, 10, 4, SP.F); rect(ctx, x - 4, y - 17, 8, 2, SP.B); rect(ctx, x - 4, y - 17, 8, 1, SP.A);       // the flasher
    rect(ctx, x + 2, y - 16, 1, 1, SP.r);                                                                                 // its button
    rect(ctx, x - 5, y - 17, 1, 2, lit ? SP.A : SP.Y);                                                                     // its bulb
  }
  // the bench: a wooden slat bench with a back rail, on legs
  function drawBench(ctx, bn) {
    const x = bn.x - 28, y = bn.y;
    rect(ctx, x, y - 16, 56, 4, WOOD.k); rect(ctx, x + 1, y - 16, 54, 1, WOOD.d); rect(ctx, x + 1, y - 15, 54, 2, WOOD.top); rect(ctx, x + 1, y - 13, 54, 1, WOOD.hi);   // the back rail
    for (const px of [x + 2, x + 52]) { rect(ctx, px, y - 12, 2, 4, WOOD.k); rect(ctx, px, y - 12, 1, 3, WOOD.d); }                                                     // its posts
    rect(ctx, x, y - 8, 56, 6, WOOD.k); rect(ctx, x + 1, y - 8, 54, 1, WOOD.d); rect(ctx, x + 1, y - 7, 54, 2, WOOD.top); rect(ctx, x + 1, y - 5, 54, 1, WOOD.hi); rect(ctx, x + 1, y - 4, 54, 1, WOOD.d);   // the seat
    for (const px of [x + 2, x + 27, x + 52]) { rect(ctx, px, y - 2, 2, 2, WOOD.k); rect(ctx, px, y - 2, 1, 1, WOOD.d); }                                              // the legs
    rect(ctx, x + 1, y, 54, 1, SHADOW);
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
  function drawPlant(ctx, pl, i, since) {   // Pixel Agents' plants; stirred (since: when the pointer came), the leaves sway twice
    const im = sprites.get(i ? "PLANT_2" : "PLANT");
    if (!im) return;
    const k = since ? Math.floor((performance.now() - since) / 125) : 4;
    if (k >= 4) { ctx.drawImage(im, pl.x - 8, pl.y - 30); return; }
    const sway = [-1, 0, 1, 0][k], cut = im.height - 12;
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
  // a figure put down on a piece of furniture uses it: the armchair, the window, the coffee, a
  // plant, the chess board or the ping-pong table (a game for you), the meeting table, the printer
  // where a figure goes back to when let go: its desk if it works, else a pause
  const resume = (f) => (f.work ? { kind: "desk" } : rest());
  function useAt(f, p) {
    if (!p) return null;
    const kind = KIND(p.key), long = rnd(40, 90);
    switch (kind) {
      case "armchair": return { kind: "chair", at: p.key, left: long };
      case "neuralyzer": return f.b && !f.b.human && !f.b.staff ? { kind: "neuralyze", at: p.key, stage: 0, wait: 0 } : null;
      case "bench": { const s = benchFree(lastL.benches.find((bn) => bn.key === p.key), f)[0]; return s ? { kind: "bench", ...s, left: long } : null; }
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
    press = { x: pointer.x, y: pointer.y, action: h && h.action, bot: hovered && !hovered.seat && !hovered.b.staff && hovered.b.id, piece: pieceAt(pointer), shift: e.shiftKey };
    cv.setPointerCapture(e.pointerId);
  });
  cv.addEventListener("pointermove", (e) => {
    pointer = dots(e);
    if (press && !moving && !held && Math.hypot(pointer.x - press.x, pointer.y - press.y) > 1) {
      const f = press.bot && world().folk.get(press.bot);
      if (f && f.b && !f.lying) {                      // picked up; one about to be stopped is rescued by that
        if (f.act.kind === "doom") f.act = resume(f);
        held = f; f.held = true; f.sat = false; f.lying = false;
      }
      else if (press.piece && !(press.bot && !f)) {
        moving = { key: press.piece.key, from: press.piece.at, u: press.piece.u, at: press.piece.at };
        // the fire picked up: whoever warmed their hands at it steps away (they would stand in the pit as it went)
        if (KIND(moving.key) === "fire") for (const g of world().folk.values()) if (g.act.kind === "warm") end(g);
      }
    }
    if (moving) {
      const at = [moving.from[0] + pointer.x - press.x, moving.from[1] + pointer.y - press.y], r = foot(moving.key, at, moving.u);
      // on the grid of dots while it moves, as pair: it steps. It shows where it will land: on clear floor
      // (settle), or over the fire, to burn. A wall piece that can burn comes off the wall while it is
      // carried (to the fire, or back up: it is hung again where it is let go); a fixed one slides along it
      const off = onWall(moving.key) && !fixed(moving.key), raw = off
        ? [Math.round(Math.max(2 - (r[0] - at[0]), Math.min(W - 2 - (r[2] - at[0]), at[0]))), Math.round(Math.max(0, Math.min(H - 1 - (r[3] - at[1]), at[1])))]
        : clampAt(moving.key, at, moving.u);
      moving.at = off || (lastL && inFire(lastL, moving.key, raw, moving.u)) ? raw : settle(lastL, moving.key, raw, moving.u);
    }
    if (held) {
      held.x = Math.max(0, Math.min(W, pointer.x)); held.y = Math.max(WALL, Math.min(FLOORY - 4, pointer.y + 24)); held.dir = -1;   // it hangs under the pointer, but never past the floor's bottom edge
      app.carry(held.id, pointer.x >= W || pointer.x < 0 || pointer.y < 0 || pointer.y >= H ? e.clientX : null, e.clientY);
    }
    draw();
  });
  cv.addEventListener("pointerup", (e) => {
    if (moving) {
      const s = app.state(), d = lastL && lastL.desks.find((d) => d.key === moving.key);
      const burn = lastL && inFire(lastL, moving.key, moving.at, moving.u);
      if (burn) {                            // into the fire: a picture of it goes in, the thing itself goes (a desk's bots walk out)
        const w = world(), r = foot(moving.key, moving.at, moving.u);
        const mid = KIND(moving.key) === "door" ? moving.key.slice(5) : null;      // a door: its machine goes out of the room (and of sshConfigs)
        toFire(r, snapshot(r, (c) => (d ? drawDesk(c, d, w, w.T, new Set()) : mid !== null ? drawDoor(c, moving.at[0], paintOf(mid), false, false, moving.at[1]) : drawKind(c, KIND(moving.key), moving.at, w, w.T))));
        if (d) { const ids = s.bots.filter((b) => b.unit === d.unit).map((b) => b.id); if (!condemn(ids, d.unit).length) app.act({ kind: "forget", unit: d.unit }); }   // its bots first (condemn), then the desk
        else app.act(mid !== null ? { kind: "unmachine", m: mid } : { kind: "burn", key: moving.key });
      } else {
        const at = settle(lastL, moving.key, clampAt(moving.key, moving.at, moving.u), moving.u);       // put down on the grid of dots, on clear floor
        dropped.set(moving.key, { at, t: performance.now() });
        if (KIND(moving.key) === "fire") {        // put down where someone stands: they walk out of the flames
          const r = foot(moving.key, at, moving.u);
          for (const g of world().folk.values()) if (!g.held && g.x >= r[0] && g.x < r[2] && g.y >= r[1] + 10 && g.y < r[3] + 4) {
            end(g); g.act = { kind: "loiter", x: g.x, y: Math.min(lastL.floor.y1, r[3] + 8), left: rnd(3, 6), v: 30 };
          }
        }
        s.layout = { ...(s.layout || {}), [moving.key]: at };
        app.act({ kind: "layout", layout: s.layout });
      }
      moving = null;
    } else if (held) {
      const f = held; held = null; f.held = false;
      // dropped at its door: it waits there to be stopped; carried out of the room, onto the pane,
      // its session opens there; anywhere else it goes back to what it did
      const out = pointer && (pointer.x >= W || pointer.x < 0 || pointer.y < 0 || pointer.y >= H), bot = f.b && !f.b.human;
      if (!out && lastL) { f.x = Math.max(lastL.floor.x0, Math.min(lastL.floor.x1, f.x)); f.y = Math.max(lastL.floor.y0, Math.min(lastL.floor.y1, f.y)); }   // let go in the room: on the floor, never past its bottom edge (it hung under the pointer)
      const back = () => resume(f);
      if (out) { f.act = back(); if (bot) app.carried(f.id, e.clientX, e.clientY); }
      else if (bot && pointer && dropAt(lastL, f) === "booth") { end(f); f.act = { kind: "doom", at: boothAt(lastL, f).key, left: 5 }; }   // into the booth: five seconds, then gone
      else if (bot && pointer && dropAt(lastL, f) === "fire") bury(f, lastL);   // at once: the flames, charred remains for the undo
      else {
        const p = pointer && pieceAt(spot(f));             // what it stands on, not what the pointer is on
        if (p && KIND(p.key) === "pair") { if (bot) app.act({ kind: "pair", key: p.key, id: f.id }); else app.act({ kind: "pairOpen", key: p.key }); f.act = back(); }
        else f.act = (pointer && !f.work && useAt(f, p)) || back();
      }
      f.legs = "stand";
    } else if (press && press.action && press.action.kind === "undo") { const f = world().folk.get(press.action.id); if (f && f.act.kind === "doom") f.act = resume(f); }
    else if (press && press.action && press.action.kind === "unshoot") unshoot(press.action.id);
    else if (press && press.action && press.action.kind === "aim") firstPerson(press.action.key);     // a gun from the gun closet
    else if (press && press.action && press.action.kind === "water") {     // you go and water the plant
      const you = world().folk.get("you");
      if (you && !you.held) { end(you); you.act = { kind: "plant", at: press.action.key, wait: 0, chosen: true }; }
    }
    else if (press && press.action && press.action.kind === "sit") {       // you take the armchair; a bot sitting in it gets up
      const w = world(), you = w.folk.get("you");
      for (const g of w.folk.values()) if (g !== you && g.act.kind === "chair" && g.act.at === press.action.key) end(g);
      if (you && !you.held) { end(you); you.act = { kind: "chair", at: press.action.key, left: rnd(180, 360), chosen: true }; }
    }
    else if (press && press.action) { if (press.action.kind === "report") world().print = performance.now(); app.act({ ...press.action, shift: press.shift }); }
    // a plain click on the floor or the wall, on nothing: what is open on the right goes away
    else if (press && !press.bot && !press.piece && pointer && Math.hypot(pointer.x - press.x, pointer.y - press.y) <= 1) {
      app.act({ kind: "blank" });
      // and you walk there, if it is floor, and stand about a while
      const you = world().folk.get("you"), L = lastL;
      if (you && L && !you.held && pointer.y >= L.floor.y0) {
        end(you); you.act = { kind: "loiter", x: Math.max(L.floor.x0, Math.min(L.floor.x1, pointer.x)), y: Math.min(L.floor.y1, pointer.y), left: rnd(20, 45), v: 36 };
      }
    }
    press = null;
    draw();
  });
  cv.addEventListener("pointerleave", () => { if (!press) { pointer = null; draw(); } });

  // a file dropped on a bot goes into its session
  cv.addEventListener("dragover", (e) => { e.preventDefault(); pointer = dots(e); draw(); });
  cv.addEventListener("drop", (e) => {
    e.preventDefault(); pointer = dots(e); draw();
    if (hovered && !hovered.b.human && !hovered.b.staff && e.dataTransfer.files.length) app.act({ kind: "drop", id: hovered.b.id, files: [...e.dataTransfer.files] });
    else if (screenHover && e.dataTransfer.files.length) app.act({ kind: "dropShell", ...screenHover, files: [...e.dataTransfer.files] });   // on a screen: into your terminal there
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
  window.__room = { world, size: () => ({ W, H, K }), layout: () => lastL, hp: () => lastHp, settle: (...a) => settle(lastL, ...a), fps: (key) => firstPerson(key) };   // for poking at it from the console
  const stats = () => { const w = world(); return { since: w.born, bots: Object.fromEntries(w.stats), meetings: w.meetings, inked: w.ink ? inked(w) : 0 }; };
  // at(id): where a figure stands, in CSS pixels of the page (for look.mjs, to drive the room from outside)
  const at = (id) => { const f = world().folk.get(id), b = cv.getBoundingClientRect(); return f ? [b.left + f.x * K, b.top + (f.y - 12) * K] : null; };
  // the whiteboard's ink, for the view: the canvas itself, a line on it (in ink pixels), a wipe
  const board = { ink: () => ink(world()), line: (x0, y0, x1, y1, col) => inkLine(world(), x0, y0, x1, y1, col), wipe: () => wipe(world()), version: () => world().inkV || 0 };
  // the two at a game table right now (chess, pong), white or the left side first: the page lets you watch them
  const playingAt = (kind) => [...world().folk.values()].filter((f) => f.act.kind === kind && f.act.there && f.act.with && !f.act.you).sort((a, b) => a.act.side - b.act.side).map((f) => f.id);
  // for look.mjs: two bots sent to a game, a kickabout for some
  window.__room.game = (kind, a, b) => { const w = world(), fa = w.folk.get(a), fb = w.folk.get(b); if (!fa || !fb) return false; end(fa); end(fb); fa.act = { kind, side: 0, with: b, left: 90, there: false }; fb.act = { kind, side: 1, with: a, left: 90, there: false }; return true; };
  window.__room.football = (ids) => { const w = world(), team = ids.map((id) => w.folk.get(id)).filter(Boolean); return team.length >= 2 && !!football(w, lastL, team); };
  window.__room.clock = (a) => { const c = document.createElement("canvas"); c.width = 12; c.height = 10; drawClock(c.getContext("2d"), 6, 5, a); return c.toDataURL(); };   // the clock's face for an awake state
  return { talk, typed, size, draw, width, color: projectColor, icon, door: doorPicture, paints: () => ({ wood: WOOD.top, list: PAINT.map((p) => p.top) }), paintOf: paintIdx, paint: paintOf, tiny, tinyW, printer: printerPicture, carry: carryKind, place: placeKind, pend, condemn: (ids) => condemn(ids), playing: playingAt, names: NAMES, at, board, stats, hovered: () => hovered && { id: hovered.b.id, seat: !!hovered.seat } };
}
