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
//                       chess, the plants, the window, the armchair, a meeting at the
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
  const deskLabel = (d) => [...(d.name ? d.name.toUpperCase() : projectLabel(d.dir))].slice(0, d.name ? 14 : NAME).join("").replace(/(.)[-_. ]+$/, "$1");   // its own name, or its folder's
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
  // what a person's portrait wears, for a small figure of them elsewhere (the gunner outside the window): the
  // shoulders' colours as body() takes them, the skin, the hair (down the middle from the top)
  function wears(seed) {
    const f = face(seed, YOU, true), d = f.px;
    if (!d) return null;
    const at = (x, y) => { const i = (y * 20 + x) * 4; return d[i + 3] < 128 ? null : "#" + [d[i], d[i + 1], d[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join(""); };
    const shirt = at(6, 19) || YOU, skin = pixelAvatar.skin(seed);
    let hair = null;
    for (let y = 0; y < 12 && !hair; y++) { const c = at(10, y); if (c && c !== INK && c !== INK2 && c !== skin) hair = c; }
    return { shirt, shade: (at(15, 19) !== shirt && at(15, 19)) || shade(shirt, 0.62), skin, hair };
  }

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
  const FIXED = new Set(["fire", "closet", "settings", "clock"]), fixed = (key) => FIXED.has(KIND(key)) || key === "door:";
  const CLICK = new Set(["radio", "doghouse", "add", "closet", "chess", "pong", "printer", "pair", "board", "coffee", "cooler", "window", "armchair", "settings", "clock", "plant", "manager", "door"]);   // a click does something there
  const NAMES = { meet: "meeting table", pong: "ping-pong table", chess: "chess table", coffee: "coffee machine", cooler: "water cooler",
    printer: "printer", armchair: "pouf", plant: "plant", board: "whiteboard", window: "window", clock: "clock", add: "project cabinet", pair: "pair room", settings: "switchboard", manager: "reception",
    booth: "booth", neuralyzer: "neuralyzer", door: "door", doghouse: "dog house", radio: "radio" };
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
      case "armchair": return [x - 8, y - 14, x + 8, y + 1];
      case "board": return [x - 16, y - 26, x + 16, y + 1];
      case "window": return [x, y, x + 21, y + 18];
      case "clock": return [x, y, x + 17, y + 17];
      case "fire": return [x, y - FIRE.h, x + FIRE.w, y + 1];
      case "closet": return [x, y, x + 22, y + 24];
      case "settings": return [x, y, x + SB_W(), y + SB_H()];
      case "manager": return [x - 2, y - 36, x + 48, y + 2];
      case "pair": return [x - 13, y - 20, x + 13, y + 1];
      case "booth": return [x - 9, y - 32, x + 9, y + 2];
      case "door": return [x, y, x + 16, y + 20];
      case "neuralyzer": return [x - 7, y - 19, x + 7, y + 2];
      case "doghouse": return [x - 12, y - 21, x + 12, y + 2];
      case "radio": return [x - 7, y - 21, x + 7, y + 1];
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
  // A killed one (in pieces, in the booth, gone) gives up its chair at once but keeps its place in the
  // row: put back together, it sits where it sat
  const sitters = (bots, d, gone) => {
    const here = new Map(bots.filter((b) => b.unit === d.unit).map((b) => [b.id, b]));
    const row = (seatRows.get(d.unit) || []).filter((id) => here.has(id));
    for (const id of here.keys()) if (!row.includes(id)) row.push(id);
    seatRows.set(d.unit, row);
    return row.filter((id) => !(gone && gone.has(id))).map((id) => here.get(id));
  };
  function layout(s, bots, moving, gone) {
    const saved = s.layout || {}, placed = {};
    const units = s.desks.map((d) => {
      const n = sitters(bots, d, gone).length, label = deskLabel(d);
      const trees = [...new Set(sitters(bots, d, gone).map((b) => b.tree).filter(Boolean))];
      const cab = Math.max(tinyW(label) + 6, 22, trees.length ? 26 + trees.length * LAP : 0);
      return { key: "u:" + d.unit, d, label, trees, cab, w: n ? cab + 4 + n * SEAT : cab };   // a chair a bot; none, and no desktop either, without one
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
      const [x, y] = u.at, mine = sitters(bots, u.d, gone);
      const seat = (k) => x + u.cab + 4 + SEAT / 2 + k * SEAT;
      return { key: u.key, ...u.d, label: u.label, trees: u.trees, x0: x, w: u.w, cab: u.cab, y, color: projectColor(u.d.unit),
        seats: mine.map((b, k) => [b.id, seat(k), b.tree]), at: u.at, u };
    });
    const b0 = one("board"), c0 = one("clock"), f0 = one("fire"), cl = one("closet"), a0 = one("add"), sw = one("settings");
    const L = {
      desks, add: a0 && { key: a0.key, x0: a0.x, y0: a0.y - 15, x1: a0.x + ADD_W, y1: a0.y },
      floorY: FLOORY, board: b0 && { key: b0.key, x: b0.x, y: b0.y, x0: b0.x - 16, y0: b0.y - 26, x1: b0.x + 16, y1: b0.y }, clock: c0 && { key: c0.key, x0: c0.x, y0: c0.y },
      windows: all("window").map((p) => ({ key: p.key, x0: p.x, y0: p.y, x1: p.x + 21, y1: p.y + 18 })),
      fire: f0 && { key: f0.key, x0: f0.x, y0: f0.y - FIRE.h, x1: f0.x + FIRE.w, y1: f0.y }, closet: cl && { key: cl.key, x0: cl.x, y0: cl.y, x1: cl.x + 22, y1: cl.y + 24 },
      settings: sw && { key: sw.key, x0: sw.x, y0: sw.y, x1: sw.x + SB_W(), y1: sw.y + SB_H() },
      managers: all("manager"), meet: one("meet"), pong: one("pong"), chess: one("chess"), pairs: all("pair"), booths: all("booth"), neuralyzers: all("neuralyzer"), doghouses: all("doghouse"), radios: all("radio"),
      cooler: one("cooler"), plants: all("plant"), printer: one("printer"), chairs: all("armchair"),
      coffeeAt: one("coffee"), floor: { x0: 11, y0: WALL + 8, x1: W - 11, y1: H - 2 }, placed,   // (where a figure may stand: right up to the bottom edge, its body inside the side walls)
    };
    L.coffee = L.coffeeAt && { x: L.coffeeAt.x + 27, y: L.coffeeAt.y - 10, on: L.coffeeAt.key };
    L.doors = (s.machines || []).map((m, i) => ({ key: doorKeys[i], mid: m.id, m, x0: at0[doorKeys[i]][0], y0: at0[doorKeys[i]][1] }));
    // what can be picked up and moved, front to back
    L.pieces = [
      ...desks.map((d) => ({ key: d.key, at: d.at, u: d.u, r: foot(d.key, d.at, d.u) })),
      ...keys.map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
      ...doorKeys.map((k) => ({ key: k, at: P(k), r: foot(k, P(k)) })),
    ].sort((a, b) => b.r[3] - a.r[3]);
    L.pieceOf = (key) => L.pieces.find((p) => p.key === key) || null;
    // the spots at a piece carry its key (`on`): whoever is there rides along when it is dragged (ride)
    L.coolerSpot = (side) => L.cooler && { x: L.cooler.x + (side ? 10 : -8), y: L.cooler.y + 1, on: L.cooler.key };
    L.pongSpot = (side) => L.pong && { x: L.pong.x + (side ? 22 : -22), y: L.pong.y + 1, on: L.pong.key };
    L.chessSeat = (side) => L.chess && { x: L.chess.x + (side ? 13 : -13), y: L.chess.y, on: L.chess.key };
    L.meetSeat = (i) => { const s = L.meet && [[-23, 0, 1], [23, 0, -1], [-8, -5, 1], [8, -5, -1]][i]; return s && { x: L.meet.x + s[0], y: L.meet.y + s[1], dir: s[2], on: L.meet.key }; };
    L.seat = (id) => { for (const d of desks) for (const [i, x] of d.seats) if (i === id) return { x, y: d.y, on: d.key }; return null; };
    L.free = (x, y) => !L.pieces.some((p) => !onWall(p.key) && x >= p.r[0] - 6 && x <= p.r[2] + 6 && y >= p.r[1] && y <= p.r[3] + 4);
    return L;
  }

  // ---- everyone: what each bot is doing
  let everyone = null;
  function world() {
    if (!everyone) everyone = { condemned: new Map(), razed: new Set(), hunt: null, folk: new Map(), meeting: null, lastMeeting: -60, board: [], pong: { t: 0, from: 0 }, doorAt: {}, stopped: new Map(), flare: 0, sparks: 0, boothFlash: null, flash: null, shots: [], spared: new Set(), clean: null, cleanRest: 0, print: 0, burning: [], stats: new Map(), meetings: 0,
      remains: new Map(), riseAt: new Map(), sway: new Map(), ball: null, mugs: new Map(),
      beams: [], hums: new Map(), T: 0, born: performance.now() };
    return everyone;
  }
  // the chess board as pair keeps it: 64 squares from a1, upper case white, "." empty
  const START = "RNBQKBNRPPPPPPPP" + ".".repeat(32) + "pppppppprnbqkbnr";
  const newBoard = () => START;
  const rest = (r = Math.random()) => ({ kind: "stand", left: 6 + r * 10 });   // a pause, before the next break
  function end(f) { f.lying = false; f.carry = null; f.on = null; f.act = rest(0.06); }
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
  // to a spot; one at a piece says so (`on`: its key), and who has got there rides along when the piece is dragged
  // (`ride`). Every act walks to its spot each tick, so `on` is always the piece the figure is at now (thinkOne clears it)
  function walk(f, to, v, dt) {
    const dx = to.x - f.x, dy = to.y - f.y, d = Math.hypot(dx, dy);
    if (d <= v * dt) { f.x = to.x; f.y = to.y; f.on = to.on || null; if (f.legs !== "sit") f.legs = "stand"; return true; }
    f.on = null;
    f.x += (dx / d) * v * dt; f.y += (dy / d) * v * dt;
    if (Math.abs(dx) > 0.5) f.dir = Math.sign(dx);
    f.step += dt * 8;
    const k = Math.floor(f.step) % 2;
    f.legs = k ? "walk1" : "walk2";
    f.arms = f.carry || (k ? "sw1" : "sw2");
    return false;
  }
  // seated somewhere that can be moved: stays on its seat (a desk's chairs close up when one leaves). A seat at
  // another piece (its directory changed, even to a desk close by) is walked to
  function sitAt(f, s, v, dt) {
    if (f.legs === "sit" && f.sat && f.satAt === s.on && Math.hypot(s.x - f.x, s.y - f.y) < 40) { f.x = s.x; f.y = s.y; f.on = s.on || null; return true; }
    f.sat = walk(f, s, v, dt);
    if (f.sat) f.satAt = s.on;
    return f.sat;
  }
  // a piece being dragged takes along whoever is at it, this very frame: think runs twelve times a second, a drag
  // draws with the pointer. Seated at a desk or standing by it, at the reception, in the pouf, at the chess or the
  // ping-pong table, at the coffee machine, in the booth: whoever got to a spot of it (walk's `on`), by as much as it moved
  function ride(w, L, before, key) {
    const p = L.pieceOf(key), q = before && before.pieceOf(key);
    const dx = p && q ? p.at[0] - q.at[0] : 0, dy = p && q ? p.at[1] - q.at[1] : 0;
    if (dx || dy) for (const f of w.folk.values()) if (f.on === key && !f.held) { f.x += dx; f.y += dy; }
  }

  // `from` sent `to` a message: a beam from its antenna to the other's, wherever they are
  const BEAM = 0.7;                          // seconds for it to cross the room
  function talk(from, to) {
    const w = world(), f = w.folk.get(from), g = w.folk.get(to);
    if (!f || !g || f === g) return;
    stat(w, from).sent++; stat(w, to).got++;
    w.beams.push({ from, to, t: 0 });
    sfx("beam", f.x);
    f.chat = Math.max(f.chat, 1);
  }
  const fresh = new Set();                    // bots you have just started (the + at a desk): to their desk, not to meet you
  // you typed to the open session (`id`, a bot's or an office manager's): a bubble over your head; the one you
  // talk to answers when you pause (dots in its bubble). `sent`: Enter, your message is off, it goes to work
  function typed(id, sent) {
    const w = world(), f = w.folk.get("you"), g = id && w.folk.get(id);
    if (f) f.chat = sent ? 0.6 : 2;
    if (g) { g.reply = performance.now() + 900; if (sent) g.sent = true; }
  }

  function think(w, bots, L, dt, usage, game) {
    const now = performance.now() / 1000, folk = w.folk;
    w.fear = hardcore() ? Math.max(0, (w.fear || 0) - dt / FEAR.fade) : 0;
    scare(w, L);
    // the bot you play chess with sits at the table until the game is over (or it gets work)
    const vs = game && !game.result ? (game.white.id === "you" ? game.black.id : game.white.id) : null;
    for (const f of folk.values()) {
      if (f.id === vs && !(f.act.kind === "chess" && f.act.you) && !f.work) { end(f); f.act = { kind: "chess", side: game.white.id === vs ? 0 : 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "chess" && !f.human && (f.act.you ? f.id !== vs : !!vs)) end(f);   // the table is taken: the others get up
    }
    // a move on the board (yours against a bot, or the bots' own game): the one who moved reaches out
    const sg = app.state().chess;
    if (sg && sg.sans.length !== w.plies && w.plies !== undefined) { w.chessAt = performance.now(); if (app.view() === "chess") sfx("clack", L.chess ? L.chess.x : W / 2); const mover = folk.get(sg.toMove === sg.white.id ? sg.black.id : sg.white.id); if (mover) mover.moved = 0.9; }
    w.plies = sg ? sg.sans.length : undefined;
    // the bot you play ping-pong with stands at the table, at the far end
    const pp = app.pong(), pv = pp && !pp.watch && pp.id;   // the bot you play (none while you watch two)
    for (const f of folk.values()) {
      if (f.id === pv && !(f.act.kind === "pong" && f.act.you) && !f.work) { end(f); f.act = { kind: "pong", side: 1, you: true, left: 1e9, there: false }; }
      if (f.act.kind === "pong" && !f.human && (f.act.you ? f.id !== pv : !!pv)) end(f);
    }
    // the duel at the window (outside.js): the bot you play stands there with you; two you watch stay until you look away
    const ow = app.outside(), wv = ow && !ow.watch && ow.id;
    for (const f of folk.values()) {
      if (f.id === wv && !(f.act.kind === "window" && f.act.you) && !f.work && !DOOMED.has(f.act.kind)) { end(f); f.act = { kind: "window", at: ow.at, side: 1, you: true, left: 1e9 }; }
      if (f.act.kind === "window" && f.act.you && !f.human && f.id !== wv) end(f);
      if (ow && ow.watch && ow.ids.includes(f.id) && f.act.kind === "window" && f.act.with) f.act.left = Math.max(f.act.left, 5);
    }
    // messages under way: the beam reaches the other, who takes it in
    for (const bm of w.beams) { const was = bm.t; bm.t += dt; if (was < BEAM && bm.t >= BEAM) { const g = folk.get(bm.to); if (g) g.chat = Math.max(g.chat, 1); } }
    w.beams = w.beams.filter((bm) => bm.t < BEAM + 0.4 && folk.has(bm.from) && folk.has(bm.to));
    // heard: the booth's hum stopping when the bot in it is taken back
    for (const [id, h] of w.hums) {
      const f = folk.get(id);
      if (f && f.act.kind === "doom" && !f.act.back) continue;
      w.hums.delete(id);
      if (h) { h.stop(); sfx("back", f ? f.x : W / 2); }
    }
    const you = folk.get("you");
    if (you) you.chat = Math.max(0, you.chat - dt);
    const opens = app.opens();
    for (const b of bots) {
      const f = folk.get(b.id), work = b.waiting || b.busy;
      if (!f) continue;
      f.chat = Math.max(0, f.chat - dt);
      if (b.waiting && f.waiting === false) sfx("ask", f.x);   // a hand goes up
      f.busy = b.busy; f.waiting = b.waiting;
      // a bot whose session is open sits at its desk for you, whatever it was doing; when the
      // session closes it gets up again
      f.open = opens.includes(b.id);
      if (!f.open || work) f.sent = false;
      // its session opened while it idles: it comes over to meet you halfway (the focused one), and goes to its desk
      // once you have sent it something; one open beside it waits at its desk. One you have just started goes
      // straight to its desk, and you with it, for its first message (until it has had one, or its tile closes)
      if (fresh.has(b.id) && (work || (f.wasOpen && !f.open))) fresh.delete(b.id);
      f.wasOpen = f.open;
      const meet = f.open && !work && !f.sent && app.open() === b.id && !fresh.has(b.id);
      if (meet && f.act.kind !== "meet" && !DOOMED.has(f.act.kind)) { end(f); f.act = { kind: "meet" }; }
      else if (f.open && !work && !meet && f.act.kind !== "desk" && !DOOMED.has(f.act.kind)) { end(f); f.act = { kind: "desk" }; }
      if (!f.open && !work && ((f.act.kind === "desk" && f.sat) || f.act.kind === "meet")) f.act = { kind: "stand", left: 0.5 };
      if (f.reply && performance.now() > f.reply && you && you.chat < 1.6) { f.reply = 0; if (f.act.kind === "meet") f.chat = Math.max(f.chat, 1.1); }   // its answer while you pause
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
      if (w.fear > 0.35 && Math.random() > 1.2 - w.fear) continue;     // afraid: they hesitate to come over
      const crew = [...folk.values()].filter((g) => g.b && !g.b.human && !g.b.staff && !g.work && !g.held && !g.lying && !g.open && SWEEPERS.has(g.act.kind) && !(g.act.kind === "loiter" && g.act.panic) && !(g.act.kind === "window" && (g.act.you || g.act.with)))
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
    dreamStep(w, dt);
    // an idle bot's place: at the end of its project's desk, by the empty chair, one per project
    // (the first idle one); the others are off on a break
    const homes = new Map();
    for (const d of L.desks) {
      const first = d.seats.find(([id]) => { const f = folk.get(id); return f && !f.work && f.act.kind === "stand"; });
      if (first) homes.set(first[0], { x: Math.min(W - 8, d.x0 + d.w + 8), y: d.y + 1, on: d.key });
    }
    for (const b of bots) { const f = folk.get(b.id); if (f && !f.held) thinkOne(w, f, L, now, dt, homes.get(b.id), usage); }
    if (you && !you.held && (fps || you.act.kind === "arm" || !huntStep(w, you, L, dt))) thinkOne(w, you, L, now, dt, null, usage);   // (in the first person the shooting is yours)
    for (const f of folk.values()) if (f.staff && !f.leaving && f.b) thinkManager(w, f, L, dt);
    thinkDog(w, L, dt);
    // two standing about in the same place: they step apart
    const loit = [...folk.values()].filter((f) => f.legs === "stand" && f.act.kind === "loiter");
    for (const a of loit) for (const b of loit) if (a.id < b.id && Math.abs(a.x - b.x) < 12 && Math.abs(a.y - b.y) < 4) {
      const d = a.x <= b.x ? -1 : 1; a.x += d * 8 * dt; b.x -= d * 8 * dt;
    }
  }
  // The office managers, one at each reception (server.py MANAGERS: Susan, Joan, Linda; the state's `managers`, one
  // a booth): a robot with a bow on her head and a skirt in her colour (body's femme), seated at her booth, her head
  // and shoulders over its counter. In the room she is "mgr:<booth's key>"
  const boothOf = (L, f) => L.managers.find((m) => m.key === (f.b && f.b.booth)) || null;
  const managerSeat = (m) => m ? { x: m.x + 27, y: m.y - 5, on: m.key } : { x: W / 2, y: H / 2 };
  // a bot let go of from here (its desk burnt, or ✕✕ in the completed list) is not just deleted: it walks to
  // the booth and waits in line, then its five seconds inside as for one put in by hand; in hardcore it stands
  // with its hands up and you fetch a gun from the gun closet and shoot it (its remains are the undo, until
  // they are cleared). With no booth it walks into the fire. Taking one out of the line, or its undo, keeps it,
  // and a burnt desk with it: the desk goes (`forget`) only once the last of its bots has
  const DOOMED = new Set(["doom", "queue", "condemned"]);
  // the booth's five seconds, by how far in they are (an undo runs them back): the door slides shut, a light rises
  // through the cabin and takes the bot up with it, feet first, the cabin glowing more and more; then it is all
  // light, and at the end the bot is gone. No countdown shows: the booth is the clock
  const BOOTH = 5;
  const boothT = (f) => (f && f.act.kind === "doom" ? Math.max(0, Math.min(BOOTH, BOOTH - f.act.left)) : -1);
  const boothShut = (t) => Math.min(1, t / 0.4), boothRise = (t) => Math.max(0, Math.min(1, (t - 0.6) / 3.4)), boothGlow = (t) => Math.max(0, Math.min(1, (t - 0.3) / 3.9));
  // kept from them (taken from her on the way, out of the booth, put back together): they leave it alone, and if
  // an office manager's session asked for it to go, it is told (the server's `kept`)
  function spare(w, id) {
    w.spared.add(id);
    if (((app.state() || {}).removing || []).includes(id)) app.act({ kind: "kept", id });
  }
  // one that is to go lets go of its terminal at once (the booth or the gun take a while; kept, it stays closed).
  // After this frame: closing a tile draws the room, and this may be in the middle of thinking
  const unopen = (id) => queueMicrotask(() => app.act({ kind: "close", id }));
  function condemn(ids, unit = null) {
    const w = world(), took = [];
    for (const id of ids) {
      const f = w.folk.get(id);
      if (!f || !f.b || f.b.human || f.b.staff || f.held || DOOMED.has(f.act.kind) || w.remains.has(id)) continue;
      end(f); f.act = { kind: "queue", n: 99 }; unopen(id);
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
    if (!bo) {                               // no booth: deleted, and out through its door
      for (const f of line) { app.act({ kind: "stop", id: f.id, rm: true }); w.stopped.set(f.id, performance.now()); w.condemned.delete(f.id); }
      return;
    }
    line.forEach((f, n) => { if (f.act.kind !== "queue") f.act = { kind: "queue", n }; else f.act.n = n; });
    if (line[0] && line[0].act.there && ![...folk.values()].some((g) => g.act.kind === "doom" && g.act.at === bo.key)) line[0].act = { kind: "doom", at: bo.key, left: BOOTH };
  }
  // hardcore: the condemned are shot. By an office manager if there is one in the room (each takes her own), else by
  // you: a gun from the gun closet, up to the nearest, levelled a moment, the shot, the next; the gun back
  function huntStep(w, f, L, dt) {
    const bo = L.booths[0], hard = hardcore() && !!bo, v = 34;
    const mine = (id) => ![...w.folk.values()].some((g) => g !== f && g.hunt && g.hunt.id === id);
    const targets = () => [...w.condemned.keys()].map((id) => w.folk.get(id)).filter((g) => g && !g.held && g.act.kind === "condemned" && mine(g.id));
    const nearest = () => targets().sort((a, b) => Math.hypot(a.x - f.x, a.y - f.y) - Math.hypot(b.x - f.x, b.y - f.y))[0];
    let h = f.hunt;
    if (!h) {                                // one each: claimed at once, so the others leave it to her
      const staff = [...w.folk.values()].some((g) => g.staff && !g.leaving), g = hard && !(f.human && staff) && nearest();
      if (!g) return false;
      h = f.hunt = { phase: "arm", id: g.id, t: 0 }; end(f);
    }
    const front = bo && { x: bo.x + 14, y: bo.y };
    f.sat = false; f.lying = false; f.act = { kind: "hunt", chosen: true };
    if (!hard && h.phase !== "return") h.phase = f.carry === "gun" ? "return" : "done";
    switch (h.phase) {
      case "arm":
        if (walk(f, front, v, dt)) { f.dir = -1; f.carry = "gun"; h.phase = "hunt"; }
        return true;
      case "hunt": {                         // the one claimed (or the nearest left), from its right (or its left by the wall)
        const was = h.id && w.folk.get(h.id), g = was && !was.held && was.act.kind === "condemned" ? was : nearest();
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
    f.hunt = null; f.carry = null; f.act = rest();
    return false;
  }
  // clearing out (app.cleanup(): what an office manager's session was asked to remove; never one you rescued), one
  // at a time each. She
  // fetches it and walks it to the booth, a step behind her, where its five seconds run as for any bot put in.
  // Hardcore: it is condemned where it stands (hands up) and shot (huntStep). Taking one from her, or its undo,
  // spares it for good. A pause between them, so it can be watched
  function cleanStep(w, f, L, dt) {
    const cu = app.cleanup(), bo = L.booths[0];
    const next = () => cu.ids.find((id) => { const g = w.folk.get(id); return g && !w.spared.has(id) && !g.held && !DOOMED.has(g.act.kind) && g.act.kind !== "escort"; });
    let c = f.clean;
    const finish = (pause) => {
      const g = c && w.folk.get(c.id);
      if (g && g.act.kind === "escort") g.act = rest();
      f.clean = null; f.cleanRest = pause;
      return false;
    };
    if (!cu.on || !bo || (c && cu.hardcore)) return c ? finish(5) : false;
    if (!c) {
      if ((f.cleanRest = (f.cleanRest || 0) - dt) > 0) return false;
      const id = next();
      if (!id) return false;
      if (cu.hardcore) { condemn([id]); f.cleanRest = 6; return false; }   // hands up; the shooting is huntStep's
      c = f.clean = { id, phase: "fetch", t: 0 };
      f.carry = null;                        // her notes stay at her desk
    }
    const g = w.folk.get(c.id), front = { x: bo.x + 14, y: bo.y, on: bo.key };
    f.sat = false; f.act = { kind: "clean" };
    switch (c.phase) {
      case "fetch":                          // to the bot
        if (!g || g.held || !cu.ids.includes(c.id)) return finish(3);
        if (walk(f, { x: g.x + 10, y: g.y }, 26, dt)) { f.dir = -1; g.act = { kind: "escort", by: f.id }; unopen(g.id); c.phase = "walk"; }
        return true;
      case "walk":                           // to the booth, the bot behind her
        if (!g || g.held || g.act.kind !== "escort") { spare(w, c.id); return finish(3); }
        if (walk(f, front, 22, dt)) { f.dir = -1; f.legs = "stand"; g.act = { kind: "doom", at: bo.key, left: BOOTH }; c.phase = "booth"; }
        return true;
      case "booth":                          // beside it while its seconds run
        if (!walk(f, front, 22, dt)) return true;
        f.legs = "stand"; f.dir = -1; f.arms = "down";
        if (!g) return finish(8);
        if (g.act.kind !== "doom") { spare(w, c.id); return finish(5); }
        return true;
    }
    return finish(5);
  }
  // Linda (the notes) walks over to the bot she writes up; the others are a session of their own and stay at their
  // booth: typing while a turn runs in it (her screen runs), her hand up and a ? while she asks you something, dozing
  // while her process is off (an hour idle; a click wakes her). Any of them walks a bot to the booth that is to go
  function thinkManager(w, f, L, dt) {
    const mg = f.b, notes = mg.style === "notes", dg = app.digest(), v = 30, seat = managerSeat(boothOf(L, f));
    f.busy = !notes && !!mg.busy; f.waiting = !notes && !!mg.waiting; f.on = null;
    if (huntStep(w, f, L, dt)) return;
    const wr = notes && dg && dg.on ? dg.writing || dg.explaining : null;   // writing notes, or explaining a decision
    if ((f.clean || !wr) && cleanStep(w, f, L, dt)) return;
    let to = null, look = null;
    if (wr && wr.startsWith("u:")) {
      const d = L.desks.find((x) => x.unit === wr.slice(2));
      if (d) { const { sx } = screenAt(d); to = { x: sx + 4, y: d.y + 13, on: d.key }; look = sx + 11; }
    } else if (wr) {
      const g = w.folk.get(wr);
      if (g && !g.held && !g.leaving) {
        const at = g.legs === "sit" && !g.lying ? L.seat(wr) || g : g;
        to = g.lying ? { x: g.x + 20, y: g.y + 1 } : { x: Math.round(at.x) - 8, y: Math.round(at.y) + 13, on: at.on };
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
    if (!sitAt(f, seat, v, dt)) { f.act = { kind: "file" }; return; }
    f.dir = -1; f.legs = "sit";
    if (!notes) {
      const off = !mg.alive && !mg.busy;
      f.act = { kind: off ? "doze" : "file", there: true };
      f.arms = f.waiting ? "wave" : off ? "down" : f.busy ? (Math.floor(w.T * 6) % 2 ? "fwd2" : "fwd1") : Math.floor(w.T / 4) % 5 === 0 ? "paper" : "down";
      return;
    }
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
    f.on = null;                             // (its act walks it to its spot again, and says whose it is)
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
      const o = app.open(), g = o && folk.get(o), game = app.chess(), pp = app.pong(), ow = app.outside(), sh = app.shell(), a = f.act;
      if (game) { if (!(a.kind === "chess" && a.you)) { end(f); f.act = { kind: "chess", side: game.white.id === "you" ? 0 : 1, you: true, left: 1e9, there: false }; } }
      else if (pp) { if (!(a.kind === "pong" && a.you)) { end(f); f.act = { kind: "pong", side: 0, you: true, left: 1e9, there: false }; } }
      else if (ow && !ow.watch) { if (!(a.kind === "window" && a.you)) { end(f); f.act = { kind: "window", at: ow.at, side: 0, you: true, left: 1e9 }; } }
      else if (g) { if (!(a.kind === "with" && a.to === g.id) && !a.chosen) { end(f); f.act = { kind: "with", to: g.id }; } }      // (a seat you clicked yourself keeps you)
      else if (sh) { if (!(a.kind === "screen" && a.m === sh.m && a.dir === sh.dir) && !a.chosen) { end(f); f.act = { kind: "screen", m: sh.m, dir: sh.dir }; } }
      else if (a.kind === "with" || a.kind === "screen" || a.you) end(f);
    }
    const act = f.act, partner = () => { const g = act.with && folk.get(act.with); return g && g.act.kind === act.kind && g.act.with === f.id ? g : null; };
    if (!["desk", "meeting", "chess", "chair", "sleep"].includes(act.kind)) f.sat = false;
    if (act.kind !== "football") f.kick = 0;
    switch (act.kind) {
      case "fps":                            // the gun in hand, walking as you walk
        f.carry = "gun"; f.arms = "gun"; f.legs = f.walking ? (Math.floor(w.T * 8) % 2 ? "walk1" : "walk2") : "stand";
        break;
      case "arm": {                          // to the gun closet (where a hunter takes hers), a reach in, the gun, then the first person.
        // Stopped by hardcore going off, the closet going, a session or a terminal opening (or any other click: pointerup)
        const bo = L.booths.find((b) => b.key === act.at), sh = app.shell();
        if (!hardcore() || !bo || app.open() !== act.open || (sh ? sh.m + ":" + sh.dir : null) !== act.shell) { end(f); break; }
        if (!walk(f, { x: bo.x + 14, y: bo.y, on: bo.key }, 36, dt)) break;
        f.legs = "stand"; f.dir = -1;
        if ((act.t = (act.t || 0) + dt) < 0.4) { f.arms = "reach"; break; }
        f.carry = "gun"; f.arms = "gun";
        if (!act.went) { act.went = true; const key = bo.key; queueMicrotask(() => firstPerson(key)); }   // (after this drawing: the melt starts from the room with the gun in your hands)
        break;
      }
      case "with": {
        // with the bot you talk to, at its desk (it sits down for you): behind the desk, over its shoulder,
        // looking at its screen with it; behind, so a click on the bot (or a neighbour) is on the bot and not
        // on you (the bubble is yours while you type)
        const g = folk.get(act.to), gs = L.seat(act.to);
        if (!g) { f.act = rest(); break; }
        if (DOOMED.has(g.act.kind) || g.act.kind === "escort") { f.legs = "stand"; f.arms = "down"; f.dir = g.x < f.x ? -1 : 1; break; }   // it is to go (its session still open): you watch, where you are
        const at = gs || g;
        const mb = g.staff && boothOf(L, g);
        const met = g.act.kind === "meet" && g.act.side ? g.act : null;   // it comes to meet you: halfway, facing it
        const spot = met ? { x: met.x + 14 * met.side, y: met.y } : mb ? { x: mb.x + 54 <= L.floor.x1 ? mb.x + 54 : mb.x - 8, y: mb.y + 1, on: mb.key }   // at her reception: by the counter
          : g.lying ? { x: g.x + 20, y: g.y + 1 } : { x: at.x + 5, y: Math.max(L.floor.y0, at.y - 12), on: at.on };
        act.there = walk(f, spot, 36, dt);
        if (act.there) { f.dir = met ? -met.side : g.staff && spot.x < g.x ? 1 : -1; f.arms = "down"; }
        break;
      }
      case "screen": {
        // at the screen whose terminal you have open: in front of its cabinet, under the screen (or
        // under the laptop, for a worktree's), the bubble while you type
        const d = L.desks.find((x) => (x.m || "") === act.m && (x.dir === act.dir || x.trees.includes(act.dir)));
        if (!d) { f.act = rest(); break; }
        const { sx } = screenAt(d), k = d.trees.indexOf(act.dir);
        act.there = walk(f, { x: k < 0 ? sx + 11 : sx + 29 + k * LAP, y: d.y + 13, on: d.key }, 36, dt);
        if (act.there) { f.dir = -1; f.arms = "down"; }
        break;
      }
      case "desk":
        if (seat && sitAt(f, seat, v, dt)) {
          f.dir = w.fear > 0.6 && Math.floor(w.T * 0.8 + f.z * 3) % 6 === 0 ? 1 : -1; f.legs = "sit"; f.carry = null;   // (afraid: a glance over its shoulder now and then)
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
      case "pet": {                          // you, petting the dog (a click on it or its house); afraid, it stays in, and you reach in at its door
        const g = w.dog;
        if (!g || g.leaving || g.held) { f.act = rest(); break; }
        const side = f.x < g.x ? -1 : 1;
        act.there = walk(f, { x: g.x + side * (g.inside ? 9 : 13), y: g.y + 1 }, 36, dt);
        if (act.there) {
          f.legs = "stand"; f.dir = -side; f.arms = Math.floor(w.T * 4) % 2 ? "reach" : "down";
          if ((act.left -= dt) <= 0) f.act = rest();
        }
        break;
      }
      case "fetch": {                        // you, playing fetch: the throw, then watching the dog run for the stick and bring it back
        const g = w.dog, st = w.stick;
        f.legs = "stand";
        if (!g || !st || g.held || g.act.kind !== "fetch") {      // over: you pick the stick up (if it lies at your feet)
          if (!st || Math.hypot(st.x - f.x, st.y - f.y) > 20 || (act.done = (act.done || 0) + dt) > 0.4) { w.stick = null; f.act = rest(); }
          else f.arms = "reach";
          break;
        }
        const k = w.T - act.at;
        f.arms = k < 0.15 ? "wave" : k < 0.4 ? "reach" : "down";      // the swing and the throw
        if (k >= 0.4) f.dir = (st.held ? g.x : st.x) < f.x ? -1 : 1;
        break;
      }
      case "meet": {                         // its session opened while it idled: halfway to you, facing you, talking
        const you = folk.get("you");
        if (!you) { f.act = rest(); break; }
        if (!act.side) {
          const side = you.x >= f.x ? 1 : -1, mx = Math.max(L.floor.x0 + 7, Math.min(L.floor.x1 - 7, (you.x + f.x) / 2));
          Object.assign(act, { side, x: mx - 7 * side, y: Math.max(L.floor.y0, Math.min(L.floor.y1, (you.y + f.y) / 2)) });
        }
        f.carry = null;
        act.there = walk(f, act, 30, dt);
        if (act.there) { f.legs = "stand"; f.dir = act.side; f.arms = "down"; }
        break;
      }
      case "loiter":
        if (walk(f, act, act.v || v, dt)) { f.arms = f.carry || "down"; if ((act.left -= dt) <= 0) f.act = rest(); }
        if (act.panic) f.arms = "up";        // away from the shotgun, hands up
        break;
      case "hide": {                         // hardcore, afraid: behind a piece of furniture, peeking at the gun closet
        if (!walk(f, act, v * 1.8, dt)) break;
        const bo = L.booths[0], gun = [...folk.values()].some((g) => g.carry === "gun" && Math.hypot(g.x - f.x, g.y - f.y) < 90);
        f.legs = "stand"; f.dir = bo ? (bo.x < f.x ? -1 : 1) : f.dir; f.arms = gun ? "up" : "down";
        if (!(w.fear > 0.2) || (act.left -= dt) <= 0) f.act = rest();
        break;
      }
      case "coffee":
        // the pot holds what is left of the session's limit: an empty one gives nothing
        if (!L.coffee) { f.act = rest(); break; }
        if (!walk(f, L.coffee, v, dt)) break;    // (there, it stays there: after the machine, if it is moved)
        if (act.stage === 0) { f.dir = -1; f.arms = "reach"; act.stage = 1; }
        else {
          act.wait += dt; const dry = usage.session && usage.session.used >= 100;
          f.arms = act.wait > 2 && !dry ? "mug" : "reach";
          if (act.wait > 2.8) { f.carry = dry ? null : "mug"; f.act = rest(); }
        }
        break;
      case "print":
        if (!L.printer) { f.act = rest(); break; }
        if (!walk(f, { x: L.printer.x - 9, y: L.printer.y, on: L.printer.key }, v, dt)) break;
        if (act.stage === 0) { f.dir = 1; f.arms = "down"; act.stage = 1; }
        else if ((act.wait += dt) > 2.4) { f.carry = "paper"; f.act = rest(); }
        break;
      case "cooler": {
        const s = L.coolerSpot(act.side), g = folk.get(act.with);
        if (!g || !s) { f.act = rest(); break; }
        if (walk(f, s, v, dt)) {
          f.dir = act.side ? -1 : 1; f.arms = "down";
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
        const spot = seated ? { x: g.x + 20, y: g.y + 1, on: g.on } : { x: g.x + side * 14, y: g.y + 1, on: g.on };
        if (walk(f, spot, v + 6, dt)) {
          f.dir = spot.x > g.x ? -1 : 1;
          f.arms = (act.wave -= dt) > 0 ? "wave" : "down";
          // visited: an idle one stops and faces the visitor
          if (!g.work && (g.act.kind === "stand" || g.act.kind === "loiter")) { g.act = { kind: "stand", left: 2 }; g.legs = "stand"; g.dir = -f.dir; }
          if ((act.left -= dt) <= 0) f.act = f.work ? { kind: "desk" } : rest();
        }
        break;
      }
      case "plant": {
        const pl = L.plants.find((p) => p.key === act.at) || L.plants[0];
        if (!pl) { f.act = rest(); break; }
        const side = pl.x > (L.floor.x0 + L.floor.x1) / 2 ? -1 : 1;
        f.carry = "can";
        if (walk(f, { x: pl.x + side * 14, y: pl.y + 1, on: pl.key }, v, dt)) {
          f.dir = -side; f.arms = "can"; act.water = true;
          if ((act.wait += dt) > 3) { f.carry = null; f.act = rest(); }
        }
        break;
      }
      case "window": {
        const win = L.windows.find((p) => p.key === act.at) || L.windows[0];
        if (!win || (act.with && !partner())) { f.act = rest(); break; }
        const duel = act.you || act.with, mid = Math.round((win.x0 + win.x1) / 2);
        act.there = walk(f, { x: duel ? mid + (act.side ? 10 : -10) : mid, y: WALL + 8, on: win.key }, v, dt);
        if (act.there) {
          // looking out; at a duel (outside.js) the two face each other, an arm out as each fires: yours, or the
          // two you watch, as the duel goes; else a shot every few seconds, turn about
          const ow = app.outside(), mine = ow && (act.you || (ow.watch && ow.ids.includes(f.id)));
          const shot = mine ? performance.now() / 1000 - ow.fired[act.side] < 0.5 : !!act.with && (w.T + (act.side ? 3.5 : 0)) % 7 < 0.5;
          f.dir = duel ? (act.side ? -1 : 1) : Math.sin(w.T * 0.7 + f.z) > 0 ? 1 : -1;
          f.arms = shot ? "fwd1" : "down";
          if ((act.left -= dt) <= 0) { const g = partner(); if (g) g.act = rest(); f.act = rest(); }
        }
        break;
      }
      case "chair": {
        const ch = L.chairs.find((p) => p.key === act.at) || L.chairs[0];
        if (!ch) { f.act = rest(); break; }
        if (sitAt(f, poufSeat(ch), v, dt)) { f.dir = poufFace(ch); f.legs = "sit"; f.arms = "book"; if ((act.left -= dt) <= 0) f.act = rest(); }
        break;
      }
      case "present":
        if (!w.meeting || w.meeting.host !== f.id || !L.board) { f.act = rest(); break; }
        if (walk(f, { x: L.board.x0 - 6, y: L.board.y + 1, on: L.board.key }, v, dt)) {
          f.dir = 1; f.arms = Math.floor(w.T * 2) % 3 ? "reach" : "down"; act.drawing = true;   // drawing the dream on (dreamStep)
        }
        break;
      case "meeting": {
        const s = L.meetSeat(act.seat);
        if (!s || (!w.meeting && !act.free)) { f.act = rest(); break; }
        if (sitAt(f, s, v, dt)) { f.dir = s.dir; f.legs = "sit"; f.arms = "down"; if (act.free && (act.left = (act.left ?? rnd(30, 60)) - dt) <= 0) f.act = rest(); }
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
        if (walk(f, { x: t.x, y: Math.max(WALL + 30, t.y - (chess ? 17 : 22)), on: t.key }, v, dt)) {
          f.legs = "stand";
          f.dir = chess ? (Math.floor(w.T * 0.5 + f.z) % 2 ? 1 : -1) : w.pong.from ? -1 : 1;
          f.arms = chess && performance.now() - (w.chessAt || 0) < 600 ? "up" : "down";
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
        if (act.stage < 2 && !walk(f, L.coffee, v, dt)) break;
        if (act.stage === 0) { f.dir = -1; f.arms = "reach"; act.stage = 1; }
        else if (act.stage === 1) {
          act.wait += dt; const dry = usage.session && usage.session.used >= 100;
          if (dry) { f.act = rest(); break; }
          if (act.wait > 2) { f.carry = "mug"; act.stage = 2; act.wait = 0; } else f.arms = "reach";
        } else if (walk(f, { x: gs.x + 20, y: gs.y + 1, on: gs.on }, v, dt)) {
          f.dir = -1; f.arms = "mug";
          if ((act.wait += dt) > 1.2) { if (g.legs === "sit" && g.act.kind === "desk") { w.mugs.set(g.id, w.T + 90); f.carry = null; } f.act = rest(); }
        }
        break;
      }
      case "doom": {
        // put in the booth: it stands inside, the door slides shut, a light rises through the cabin and the
        // bot goes up in it (boothT, drawFigure). A click on the booth (or on the bot) all that while takes it
        // back: the light runs down again, the door opens, it steps out; so does carrying it out. At the end it
        // is gone: `claude stop` and `claude rm` (its transcript stays, for `claude --resume`)
        const bo = L.booths.find((p) => p.key === act.at);
        if (!bo) { f.act = resume(f); break; }
        f.x = bo.x; f.y = bo.y - 3; f.on = bo.key; f.dir = -1; f.arms = "down"; f.legs = "stand";
        if (!act.back && !w.hums.has(f.id)) w.hums.set(f.id, sfx("booth", bo.x));   // the light rising, heard
        if (act.back) { if ((act.left += dt * 6) >= BOOTH) f.act = resume(f); break; }
        if ((act.left -= dt) <= 0 && !f.gone) {
          app.act({ kind: "stop", id: f.id, rm: true });
          w.stopped.set(f.id, performance.now()); w.boothFlash = { key: bo.key, t0: performance.now() };
          w.hums.delete(f.id); sfx("poof", bo.x);
          folk.delete(f.id);
        }
        break;
      }
      case "queue": {                        // let go of from here: in line by the booth (on its roomier side), its turn next
        const bo = L.booths[0];
        if (!bo) break;
        const n = act.n || 0, side = bo.x - 70 < L.floor.x0 ? 1 : -1;
        act.there = walk(f, { x: bo.x + side * (18 + (n % 5) * 12), y: Math.min(L.floor.y1, bo.y + 2 + Math.floor(n / 5) * 9), on: bo.key }, v, dt);
        if (act.there) { f.legs = "stand"; f.dir = -side; f.arms = "down"; }
        break;
      }
      case "condemned": {                    // hardcore: someone takes a gun for it. Meanwhile it wanders about where it was put, at a
        // loss: a few hurried steps flailing its hands, a stop to look this way and that, off again (slower than the one with
        // the gun walks, so it is caught). The gun levelled at it: it freezes, hands up, facing it
        const by = [...folk.values()].find((g) => g.hunt && g.hunt.id === f.id && g.carry === "gun");
        if (by && by.hunt.phase === "aim") { act.to = null; f.legs = "stand"; f.arms = "up"; f.dir = by.x < f.x ? -1 : 1; break; }
        if (!act.home) {                     // first away from the gun closet, as far as it dares; then it dithers there
          const bo = L.booths[0], d = bo ? Math.hypot(f.x - bo.x, f.y - bo.y) || 1 : 1;
          act.home = bo ? { x: Math.max(L.floor.x0, Math.min(L.floor.x1, f.x + (f.x - bo.x) / d * 44)), y: Math.max(L.floor.y0, Math.min(L.floor.y1, f.y + (f.y - bo.y) / d * 24)) } : { x: f.x, y: f.y };
          act.to = { ...act.home }; act.v = 30;
        }
        if (act.to) {
          if (walk(f, act.to, act.v, dt)) { act.to = null; act.pause = rnd(0.5, 1.6); }
          f.arms = Math.floor(w.T * 6) % 2 ? "up" : "wave";
          break;
        }
        f.legs = "stand"; f.arms = "up";
        if ((act.look = (act.look || 0) - dt) <= 0) { f.dir = -(f.dir || 1); act.look = rnd(0.25, 0.6); }   // this way, that way
        if ((act.pause = (act.pause ?? rnd(0.3, 1)) - dt) <= 0) {
          for (let k = 0; k < 10 && !act.to; k++) {          // somewhere clear near it (further the more it tries), else a shuffle
            const r = 30 + k * 6, x = Math.max(L.floor.x0, Math.min(L.floor.x1, act.home.x + rnd(-r, r))), y = Math.max(L.floor.y0, Math.min(L.floor.y1, act.home.y + rnd(-r / 2, r / 2)));
            if (L.free(x, y)) act.to = { x, y };
          }
          if (!act.to) act.to = { x: Math.max(L.floor.x0, Math.min(L.floor.x1, f.x + rnd(-8, 8))), y: f.y };
          act.v = rnd(18, 28); act.pause = rnd(0.5, 1.6);
        }
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
      case "escort": {                       // walked to the booth by an office manager, a step behind her
        const m = folk.get(act.by);
        if (!m) { f.act = rest(); break; }
        if (walk(f, { x: m.x - 9 * (m.dir || -1), y: m.y }, 24, dt)) { f.legs = "stand"; f.dir = m.dir || -1; f.arms = "down"; }
        break;
      }
      case "neuralyze": {
        // put to the neuralyzer: it steps up, the flash (`/clear` in its session: a new conversation), and
        // it stands there dazed a moment
        const n = L.neuralyzers.find((p) => p.key === act.at);
        if (!n) { f.act = rest(); break; }
        if (!walk(f, { x: n.x + 11, y: n.y + 1, on: n.key }, v, dt)) break;
        if (act.stage === 0) { f.legs = "stand"; f.dir = -1; f.arms = "down"; act.stage = 1; act.wait = 0; }
        else if (act.stage === 1) {
          if ((act.wait += dt) > 0.6) { app.act({ kind: "clear", id: f.id }); w.flash = { x: n.x - 5, y: n.y - 16, t0: performance.now() }; sfx("flash", n.x); act.stage = 2; act.wait = 0; }
        } else if ((act.wait += dt) > 2.5) f.act = rest();
        break;
      }
      default: f.act = rest();
    }
    if (f.legs !== "sit") f.lying = false;
  }

  // a break, chosen by a bot with nothing to do (pair's pickBreak, and chess). Long spells, so the room is calm: a bot
  // keeps at one thing a while (a minute in the pouf, a long nap) rather than walking from one to the next; and no
  // chatter: a bubble is a real message (talk) or you
  function pickBreak(w, f, L, others) {
    const r = Math.random();
    if (w.fear > 0.5 && Math.random() < w.fear) { const at = hideSpot(w, f, L); if (at) return { kind: "hide", ...at, left: rnd(15, 35) }; }
    const mates = others.filter((g) => !g.work && (g.act.kind === "stand" || g.act.kind === "loiter"));
    const mate = mates.length ? any(mates) : null;
    const taken = (kind) => others.some((g) => g.act.kind === kind);
    // someone waits at the chess board: most likely, it gets a game
    const waiter = others.find((g) => g.act.kind === "chess" && !g.act.with && !g.act.you && g.act.there);
    if (waiter && r < 0.7) {
      const left = rnd(90, 180);
      waiter.act.with = f.id; waiter.act.left = left;
      return { kind: "chess", side: 1 - waiter.act.side, with: waiter.id, left, there: false };
    }
    if (L.meet && L.board && mates.length >= 2 && !w.meeting && w.T - w.lastMeeting > 180 && r < 0.08) {
      w.meeting = { host: f.id, left: rnd(45, 75) }; w.lastMeeting = w.T; w.meetings++;
      if (w.dream && w.dream.done !== null && w.T - w.dream.done > 240) nextDream(w);   // a dream done a while: wiped, the host starts the next
      mates.slice(0, 4).forEach((g, i) => { g.carry = null; g.act = { kind: "meeting", seat: i }; });
      return { kind: "present", wait: 0 };
    }
    if (r < 0.16 && L.coffee && !taken("coffee")) return { kind: "coffee", stage: 0, wait: 0 };
    if (r < 0.22 && L.printer && !taken("print")) return { kind: "print", stage: 0, wait: 0 };
    if (r < 0.3 && mate && L.cooler) {
      const left = rnd(25, 50);
      mate.carry = null; mate.act = { kind: "cooler", side: 1, left, with: f.id };
      return { kind: "cooler", side: 0, left, with: mate.id };
    }
    if (r < 0.4 && mate && L.pong && !taken("pong") && !app.pong()) {
      const left = rnd(40, 80);
      mate.carry = null; mate.act = { kind: "pong", side: 1, left, with: f.id, there: false };
      return { kind: "pong", side: 0, left, with: mate.id, there: false };
    }
    if (r < 0.44 && mates.length >= 1 && !taken("football")) {   // a kickabout on a clear bit of floor, two or three of them
      const game = football(w, L, [f, ...mates.slice(0, 2)]);
      if (game) return game;
    }
    if (r < 0.48 && L.chess && !taken("chess") && !(app.state().chess && !app.state().chess.result)) return { kind: "chess", side: Math.random() < 0.5 ? 0 : 1, with: null, left: rnd(45, 90), there: false };
    // a coffee for a busy colleague at its desk; a game to watch; the fire; a stretch; a nap
    const busy = others.filter((g) => g.work && g.legs === "sit" && g.act.kind === "desk" && !w.mugs.has(g.id));
    if (r < 0.54 && busy.length && L.coffee && !taken("coffee") && !taken("serve")) return { kind: "serve", to: any(busy).id, stage: 0, wait: 0 };
    const game = ["chess", "pong"].find((k) => others.filter((g) => g.act.kind === k && g.act.there && (g.act.with || g.act.you)).length >= 2 && !others.some((g) => g.act.kind === "watch" && g.act.game === k));
    if (r < 0.6 && game) return { kind: "watch", game, left: rnd(25, 50) };
    if (r < 0.65 && mate) return { kind: "visit", to: mate.id, left: rnd(15, 30), wave: 0.8 };
    if (r < 0.7 && L.fire && !(moving && KIND(moving.key) === "fire")) { const side = others.some((g) => g.act.kind === "warm" && !g.act.side) ? 1 : 0; if (!others.some((g) => g.act.kind === "warm" && g.act.side === side)) return { kind: "warm", side, left: rnd(20, 40) }; }
    if (r < 0.74) return { kind: "stretch", left: rnd(4, 8) };
    if (r < 0.8 && !f.human) return { kind: "sleep", there: false, left: rnd(60, 150) };
    // one of a kind there can be many of, not in use: a plant to water, a window, an armchair
    const spare = (kind, ps) => ps.filter((p) => !others.some((g) => g.act.kind === kind && g.act.at === p.key));
    const plants = spare("plant", L.plants), wins = spare("window", L.windows), chairs = spare("chair", L.chairs);
    if (r < 0.86 && plants.length) return { kind: "plant", at: any(plants).key, wait: 0 };
    if (r < 0.91 && wins.length) {             // a look outside, or with a mate a duel on the hills out there (outside.js)
      const at = any(wins).key;
      if (mate && !f.human && !mate.human && Math.random() < 0.5) { const left = rnd(40, 80); mate.carry = null; mate.act = { kind: "window", at, side: 1, left, with: f.id }; return { kind: "window", at, side: 0, left, with: mate.id }; }
      return { kind: "window", at, left: rnd(12, 25) };
    }
    if (r < 0.96 && chairs.length) return { kind: "chair", at: any(chairs).key, left: rnd(45, 90) };
    for (let k = 0; k < 8; k++) {             // somewhere free to stand about
      const x = rnd(L.floor.x0, L.floor.x1), y = rnd(L.floor.y0, L.floor.y1);
      if (L.free(x, y)) return { kind: "loiter", x, y, left: rnd(8, 20) };
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
    const left = rnd(40, 80), ids = team.map((g) => g.id);
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
  // The robots' dream, on the whiteboard: a pixel-art picture that appears bit by bit, two happy robots in love,
  // hand in hand, running over a sunny hill, or along a beach. A scene is 65 x 27 dream pixels, each two by two of
  // the board's ink; it comes in as a drawing would, layer by layer (the sky, the ground, the things on it, the two
  // of them, the heart last), outlines before colours, left to right. It grows by itself, slowly; faster for every
  // robot taking a nap (it is their dream) and while the host of a meeting draws at the board. Done, it stays; a
  // meeting a while later starts the next one on a wiped board. Your own marker is never drawn over
  const DREAM_PAL = { k: "#2b2433", S: "#e9f3ff", A: "#4fb3a9", B: "#f06b72", p: "#f59ab0", q: "#d6457a", r: "#e8505b", y: "#f7d154", o: "#f29b38",
    g: "#7cc96a", G: "#3f8f45", c: "#9fd3f2", b: "#5aa9e6", w: "#d6efff", n: "#f3d99b", N: "#d9b26a", t: "#8a5a32", m: "#b8bcc6" };
  const DREAM_W = 65, DREAM_H = 27, DREAM_LINE = new Set(["k", "G", "o", "N", "t", "q"]);
  const DREAM_BOT = ["....k....", "....k....", "..kkkkk..", ".kSSSSSk.", ".kSkSkSk.", ".kpSSSpk.", "..kkkkk..", ".kCCCCCk.", "kCCCCCCCk", ".kCCCCCk.", "..kk.kk..", ".k.....k."];
  const DREAM_BOW = ["..qq.qq..", "...qkq..."];        // hers: a bow on her antenna
  const DREAM_HEART = [".kk.kk.", "krrkrrk", "krrrrrk", ".krrrk.", "..krk..", "...k..."];
  const DREAM_SMALL = ["p.p", "ppp", ".p."], DREAM_BIRD = ["k...k", ".k.k.", "..k.."], DREAM_CLOUD = ["..cccc...", ".cccccccc", "ccccccccc", ".ccccccc."];
  function dreamScene(i) {
    const px = new Map();
    const put = (x, y, c, layer) => { if (c !== "." && x >= 0 && x < DREAM_W && y >= 0 && y < DREAM_H) px.set(layer + ":" + x + "," + y, { x, y, c, layer }); };   // (a layer over another paints over it later)
    const sprite = (rows, x0, y0, layer, C) => rows.forEach((r, j) => [...r].forEach((ch, i) => put(x0 + i, y0 + j, ch === "C" ? C : ch, layer)));
    const sun = (cx, cy) => {
      for (let y = -3; y <= 3; y++) for (let x = -3; x <= 3; x++) { const d = Math.hypot(x, y); if (d <= 3.3) put(cx + x, cy + y, d > 2.4 ? "o" : "y", 0); }
      for (let k = 0; k < 8; k++) { const a = (k * Math.PI) / 4; for (const r of [5, 6]) put(cx + Math.round(Math.cos(a) * r), cy + Math.round(Math.sin(a) * r), "o", 0); }
    };
    const lovers = (x0, ground) => {                 // him, her (the bow), holding hands; running lines behind, the heart over them
      const y0 = ground - 12;
      for (const dy of [5, 8]) for (let x = x0 - 5; x < x0 - 2; x++) put(x, y0 + dy, "m", 3);
      sprite(DREAM_BOT, x0, y0, 3, "A");
      sprite(DREAM_BOT, x0 + 9, y0, 3, "B"); sprite(DREAM_BOW, x0 + 9, y0, 3);
      sprite(DREAM_HEART, x0 + 6, y0 - 7, 4);
      sprite(DREAM_SMALL, x0 + 15, y0 - 7, 4); sprite(DREAM_SMALL, x0 + 19, y0 - 3, 4);
    };
    if (i === 0) {                                   // the hill: the sun, two clouds, birds; the grass, flowers, a tree
      sun(7, 6);
      sprite(DREAM_CLOUD, 47, 1, 0); sprite(DREAM_CLOUD, 15, 7, 0);
      sprite(DREAM_BIRD, 16, 2, 0); sprite(DREAM_BIRD, 46, 8, 0);
      const s = (x) => Math.round(26 - (2 + 4.5 * Math.exp(-(((x - 33) / 17) ** 2))));
      for (let x = 0; x < DREAM_W; x++) for (let y = s(x); y < DREAM_H; y++) put(x, y, y === s(x) ? "G" : "g", 1);
      [4, 9, 14, 46, 50, 61].forEach((x, k) => { put(x, s(x) + 2, k % 2 ? "y" : "r", 2); put(x, s(x) + 1, "G", 2); });
      for (let y = s(56) - 4; y < s(56); y++) put(56, y, "t", 2);
      for (let y = -3; y <= 3; y++) for (let x = -3; x <= 3; x++) { const d = Math.hypot(x, y); if (d <= 3.3) put(56 + x, s(56) - 7 + y, d > 2.4 ? "G" : "g", 2); }
      lovers(24, s(28));
    } else {                                         // the beach: the sun over the sea, waves, the sand, a palm, a crab, gulls
      sun(56, 6);
      sprite(DREAM_BIRD, 16, 3, 0); sprite(DREAM_BIRD, 23, 1, 0);
      const shore = (x) => Math.round(19 + 1.5 * Math.sin(x / 6));
      for (let x = 0; x < DREAM_W; x++) {
        for (let y = 12; y < shore(x); y++) put(x, y, y > 12 && ((x * 7 + y * 3) % 11 === 0 || ((x - 1) * 7 + y * 3) % 11 === 0) ? "w" : "b", 1);   // little waves
        for (let y = shore(x); y < DREAM_H; y++) put(x, y, y === shore(x) ? "N" : "n", 1);
      }
      for (let k = 0; k < 15; k++) { const y = 25 - k, x = 6 + Math.round(4 * Math.sin((k / 15) * 1.6)); put(x, y, "t", 2); put(x + 1, y, "t", 2); }   // the palm's trunk, bending
      sprite(["...GGG...GGG...", ".GGgggG.GgggGG.", "Ggg...GgG...ggG", "G....G.t.G....G", "....G.ttt.G...."], 3, 9, 2);   // its leaves, coconuts under them
      sprite(["rk...kr", ".rrrrr.", "r.r.r.r"], 50, 22, 2);                                // a crab
      lovers(24, 25);
    }
    const hash = (x, y) => ((Math.imul(x * 73856093 ^ y * 19349663, 2654435761) >>> 0) % 1000) / 1000;
    return [...px.values()].sort((a, b) => a.layer - b.layer || +!DREAM_LINE.has(a.c) - +!DREAM_LINE.has(b.c) || (a.x + hash(a.x, a.y) * 6) - (b.x + hash(b.x, b.y) * 6));
  }
  const DREAMS = 2;
  function newDream(w, scene, start) {
    w.dream = { scene, order: dreamScene(scene), n: 0, done: null, mine: new Map() };
    paintDream(w, Math.floor(w.dream.order.length * start));
  }
  function paintDream(w, upto) {
    const d = w.dream, c = ink(w).getContext("2d");
    let painted = false;
    for (; d.n < Math.min(upto, d.order.length); d.n++) {
      const p = d.order[d.n], a = c.getImageData(p.x * 2, p.y * 2, 2, 2).data, cell = p.y * DREAM_W + p.x;
      const was = "#" + [a[0], a[1], a[2]].map((v) => v.toString(16).padStart(2, "0")).join("");
      if ((a[3] || a[7] || a[11] || a[15]) && !(a[3] && d.mine.get(cell) === was)) continue;   // your marker is there: left as it is
      c.fillStyle = DREAM_PAL[p.c]; c.fillRect(p.x * 2, p.y * 2, 2, 2); d.mine.set(cell, DREAM_PAL[p.c]); painted = true;
    }
    if (painted) { w.inkV++; w.cells = null; }
    if (d.n >= d.order.length && d.done === null) d.done = w.T;
  }
  function dreamStep(w, dt) {
    if (!w.dream) newDream(w, Math.floor(Math.random() * DREAMS), 0.3);    // the page opens on a dream half begun
    const d = w.dream, folk = [...w.folk.values()];
    const sleepers = folk.filter((f) => !f.human && (f.act.kind === "sleep" || f.act.kind === "doze") && f.act.there).length;
    const drawing = folk.some((f) => f.act.kind === "present" && f.act.drawing);
    d.f = (d.f || 0) + dt * (0.6 + 1.5 * sleepers + (drawing ? 8 : 0));
    if (d.f >= 1) { const k = Math.floor(d.f); d.f -= k; paintDream(w, d.n + k); }
    if (d.done !== null && w.T - d.done > 1200) nextDream(w);     // done twenty minutes and no meeting came to start another: the next one
  }
  const nextDream = (w) => { const s = w.dream ? (w.dream.scene + 1) % DREAMS : 0; wipe(w); newDream(w, s, 0); };

  // ---- drawing, and what is under the pointer: hot spots are collected while drawing
  let pointer = null, hots = [], hovered = null, tip = null, lastL = null, moving = null, held = null, hoverId = null, lastHp = null, dangerNow = false;
  let dogHeld = false, dogRect = null;      // the dog in your hand; where it stands (a press there is on the dog, not on what is under it)
  let fps = null, view = null;              // hardcore: the gun taken from the gun closet, the office in the first person (fps.js); the scene it renders
  let pics = [];                             // what was just drawn, each piece by itself, for that view (scene3)
  const hardcore = () => !!app.settings().hardcore;
  // the room's sounds (sound.js), while a radio stands in it and is on, from where they happen (left to right); not
  // while you are in the first person (fps.js has Doom's there)
  const sfx = (name, x, vol = 1) => (Sound.on() && !fps ? Sound.play(name, Math.max(-1, Math.min(1, (x / W) * 1.6 - 0.8)), vol) : null);
  let screenHover = null, hoverScreen = null;    // the screen (or laptop) under the pointer: {m, dir}; the one told to the page
  const over = (x0, y0, x1, y1) => !!pointer && pointer.x >= x0 && pointer.x < x1 && pointer.y >= y0 && pointer.y < y1;
  const hot = (x0, y0, x1, y1, action) => { hots.push({ x0, y0, x1, y1, action }); return over(x0, y0, x1, y1) && !moving && !held && !dogHeld; };
  const box = (ctx, x, y, w, h, fill, edge = INK) => { ctx.fillStyle = edge; ctx.fillRect(x - 1, y - 1, w + 2, h + 2); ctx.fillStyle = fill; ctx.fillRect(x, y, w, h); };
  const rect = (ctx, x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); };
  const clip = (t, room) => { const n = Math.max(0, Math.floor((room + 1) / 4)), m = t.replace(/\s+/g, " "); return m.length > n ? m.slice(0, n - 1) + "-" : m; };
  const say = (u, what) => (u ? `${Math.max(0, 100 - u.used)}% ` : "") + what;     // what is left of a limit: "63% 5h"

  function draw() {
    const ctx = cv.getContext("2d");
    ctx.setTransform(2, 0, 0, 2, 0, 0);      // drawn in dots; the canvas has two pixels a dot, for dragging
    ctx.imageSmoothingEnabled = false;
    const s = app.state();
    hots = []; tip = null; screenHover = null; pics = [];
    ctx.drawImage(floor(), 0, 0);
    rect(ctx, W - 1, 0, 1, H, INK2);
    if (!s) return;
    const w = world(), all = s.bots || [], usage = s.usage || {};
    // every session is a bot about the room, with a chair at its desk, whether its process runs or not (Claude
    // Code's supervisor stops an idle one after an hour; opening it starts it again). Many finished sessions make a
    // long desk: a full room is a full room, the booth clears it
    const bots = all;
    const gone = new Set([...w.remains.keys(), ...w.stopped.keys(), ...[...w.folk.values()].filter((f) => f.act.kind === "doom").map((f) => f.id)]);   // killed: no chair
    // a burnt desk whose bots are on their way out is not drawn; nor one that came with its sessions once the
    // last of them is deleted from here (the feed still lists it a moment)
    const shown = s.desks.filter((d) => !w.razed.has(d.unit) && (d.kept || all.some((b) => b.unit === d.unit && !w.stopped.has(b.id))));
    const L = layout(shown.length < s.desks.length ? { ...s, desks: shown } : s, all, moving, gone);
    // the piece under the pointer: outlined if a click does something there (the rest only reacts in
    // its own way, a plant sways)
    const hp = !moving && !held && !carrying && !dogHeld && !(dogRect && over(...dogRect)) ? pieceAt(pointer) : null;   // (the dog in front of it: the dog)
    lastHp = hp;
    const on = (key) => !!hp && hp.key === key, lit = (key) => on(key) && CLICK.has(KIND(key));
    // what is being dragged over the fire (a desk without bots, a piece, a bot) is about to burn, a bot
    // held at its door is about to be stopped: it is outlined red and washed in red, flashing (tinted)
    const heldBot = held && held.b && !held.b.human && pointer ? held : null, botDrop = heldBot && dropAt(L, heldBot);
    const boothUnder = botDrop === "booth" ? boothAt(L, heldBot) : null;   // a bot held over the booth: let go and it goes in (hardcore: is shot)
    dangerNow = !!boothUnder || !!(L.fire && moving && inFire(L, moving.key, moving.at, moving.u));
    const danger = (key) => dangerNow && !!moving && moving.key === key, tintOf = (key) => (danger(key) ? flashA(w.T) : 0);
    if (moving) ride(w, L, lastL, moving.key);      // whoever is at what is being dragged comes along, this very frame
    lastL = L;
    if (Object.keys(L.placed).length) app.act({ kind: "layout", layout: { ...(s.layout || {}), ...L.placed } });
    s.layout = { ...(s.layout || {}), ...L.placed };

    // keep a figure per bot; one that turns up later comes in through its machine's door, one that is
    // gone from the list (deleted elsewhere) walks out of it
    const ids = new Set(bots.map((b) => b.id)), you ={ id: "you", name: (s.you && s.you.name) || "you", seed: s.you && s.you.seed, color: s.you && s.you.color, human: true, m: "" };
    ids.add("you");
    // the office managers, one at each reception (thinkManager)
    const mgrs = L.managers.map((mb) => (s.managers || []).find((M) => M.key === mb.key)).filter((M) => M && !M.vacant).map((M) => ({ id: "mgr:" + M.key, booth: M.key, name: M.name, seed: M.seed,
      color: M.color, style: M.style, session: M.session, busy: M.busy, waiting: M.waiting, alive: M.alive, rc: M.rc, human: false, staff: true, m: "" }));
    for (const g of mgrs) ids.add(g.id);
    for (const f of w.folk.values()) if (!ids.has(f.id) && !f.leaving) { f.leaving = true; f.held = false; f.lying = false; f.sat = false; f.legs = "stand"; }
    for (const [id, t] of w.stopped) if (!ids.has(id) || performance.now() - t > 30000) w.stopped.delete(id);
    const late = performance.now() - w.born > 3000;
    for (const b of [...bots, you, ...mgrs]) {
      if (w.stopped.has(b.id) || w.remains.has(b.id)) continue;      // stopped from here (the feed still lists it for a moment), or in pieces
      if (!w.folk.has(b.id)) {
        const at = b.staff && !late ? managerSeat(L.managers.find((m) => m.key === b.booth)) : late || b.human ? w.riseAt.get(b.id) || { x: doorX(b.m), y: WALL + 3 } : L.seat(b.id) || freeAt(L, b.id);   // back from its grave, or in by the door; at first, at its desk or anywhere
        w.riseAt.delete(b.id);
        if (late && !b.human && !b.staff) { w.dogNews = b.id; sfx("door", at.x); }    // a newcomer: the dog goes and barks at it
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
        if (f.leaving && walk(f, { x: doorX(m), y: WALL + 1 }, 30, tickDt)) { sfx("door", f.x); w.folk.delete(f.id); w.doorAt[m] = performance.now(); }
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
    if (pending) drawKind(ctx, pending.kind, pending.at, w, T);
    for (const win of L.windows) {
      const h = hot(win.x0, win.y0, win.x1, win.y1, { kind: "outside", key: win.key });
      onTheWall(win.key, [win.x0, win.y0, win.x1, win.y1], h, (c) => drawWindow(c, win));
      if (h) tip = { text: "outside", x: (win.x0 + win.x1) / 2, y: win.y1 + 10 };
    }
    if (L.clock) {
      // the clock keeps this computer awake, as Amphetamine does: a click chooses (while bots work, for
      // a while, not at all); its face says which, yellowed (drawClock)
      const ck = L.clock, a = s.awake || { mode: "off" }, h = hot(ck.x0, ck.y0, ck.x0 + 17, ck.y0 + 17, { kind: "clock" });
      onTheWall(ck.key, foot("clock", [ck.x0, ck.y0]), h, (c) => drawClock(c, ck.x0, ck.y0, a));
      if (h) {
        const tm = Math.round(Math.max(0, a.until - Date.now()) / 60000), hh = Math.floor(tm / 60), mm = tm % 60;
        const now = new Date(), hm = String(now.getHours()).padStart(2, "0") + ":" + String(now.getMinutes()).padStart(2, "0");
        tip = { text: hm + (a.mode === "for" ? ` - awake ${hh ? hh + "h " : ""}${mm}m more` : a.mode === "work" ? (a.on ? " - awake, local bots at work" : " - awake when local bots work") : ""), x: ck.x0 + 8, y: WALL + 10 };
      }
    }
    if (L.closet) {
      // the closet, against the wall: the furniture that is not in the room; a click opens it (on the right)
      const c = L.closet, h = hot(c.x0, c.y0, c.x1, c.y1, { kind: "closet" });
      outlined(ctx, [c.x0, c.y0, c.x1, c.y1], h, (x) => drawCloset(x, c, app.view() === "closet"));
      if (h) tip = { text: "furniture", x: c.x0 + 11, y: c.y0 + 4 };
    }
    if (L.settings) {
      // the switchboard on the wall: burrow's own settings, a switch each; a click opens them
      const b = L.settings, h = hot(b.x0, b.y0, b.x1, b.y1, { kind: "settings" });
      outlined(ctx, [b.x0, b.y0, b.x1, b.y1], h, (x) => drawSwitchboard(x, b, app.settings(), app.view() === "settings"));
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
      const gw = 22 + d.trees.length * LAP;     // what of it is solid: its screen (and laptops), the cabinet, the desktop
      items.push([danger(d.key) ? 1e9 : d.y - 1, () => outlined(ctx, foot(d.key, d.at, d.u), danger(d.key), (c) => drawDesk(c, d, w, T, hl), RED, tintOf(d.key)),
        [[sc.sx, sc.sy, sc.sx + gw, sc.sy + 18], [d.x0, d.y - 15, d.x0 + d.cab, d.y + 1], [d.x0 + d.cab - 2, d.y - 12, d.x0 + d.w, d.y - 3]]]);
      for (const [id, x] of d.seats) items.push([d.y - 0.5, () => drawChair(ctx, x, d.y, hl.has(id))]);
    }
    // every piece: outlined under the pointer, like a desk's chairs; what it does, on top of that. Held over
    // the fire it is drawn over everything (the flames would hide it), washed red
    const piece = (p, kind, fn, dz = 0.5) => {         // (dz: how far behind its front it is drawn: the pouf under who sits on it)
      pics.push({ key: p.key, kind, x: p.x, y: p.y, fn });
      items.push([danger(p.key) ? 1e9 : p.y - dz, () => outlined(ctx, foot(kind, [p.x, p.y]), lit(p.key) || danger(p.key), fn, danger(p.key) ? RED : GOLD, tintOf(p.key)), [foot(kind, [p.x, p.y])]]);
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
    for (const mg of L.managers) {
      // a reception, an office manager's booth: a click (on it or on her) is hers: her terminal, or the notes
      // (Linda). Two layers, she sits between them: the back wall, then the counter with her name on it
      const M = (s.managers || []).find((x) => x.key === mg.key) || {}, dg = app.digest(), live = new Set(all.map((b) => b.id)), r = foot("manager", [mg.x, mg.y]);
      const o = M.vacant ? { name: "", color: M.color || SP.r, n: 0, screen: M.session ? "off" : null, T } : { name: M.name || "", color: M.color || SP.r, n: M.style === "notes" && dg ? Object.keys(dg.sessions || {}).filter((id) => live.has(id)).length : 0,
        screen: M.session ? (M.busy ? "busy" : M.waiting ? "asks" : M.alive ? "on" : "off") : null, T };
      const h = hot(...r, { kind: "manager", key: mg.key }), both = (c) => { drawReception(c, mg, o, false); drawReception(c, mg, o, true); };
      pics.push({ key: mg.key, kind: "manager", x: mg.x, y: mg.y, fn: both });
      items.push([danger(mg.key) ? 1e9 : mg.y - 20, () => outlined(ctx, r, h || hl.has("mgr:" + mg.key) || danger(mg.key), both, danger(mg.key) ? RED : GOLD, tintOf(mg.key)), [[mg.x + 2, mg.y - 30, mg.x + 44, mg.y - 14]]]);
      if (!danger(mg.key)) items.push([mg.y - 0.5, () => drawReception(ctx, mg, o, true), [[mg.x, mg.y - 16, mg.x + 46, mg.y + 1]]]);
      if (h) tip = { text: M.vacant ? "empty - a click hires " + (M.name || "her") + " again" : (M.name || "office manager") + (M.error ? " - " + M.error : M.style === "notes" ? " - notes" : M.session && !M.id ? " - on her way" : " - office manager"), x: mg.x + 23, y: mg.y - 30 };
    }
    for (const ch of L.chairs) {              // a click on one: you go and sit in it (a bot in it gets up)
      const h = hot(...foot("armchair", [ch.x, ch.y]), { kind: "sit", key: ch.key });
      piece(ch, "armchair", (c) => drawArmchair(c, ch), 4.5);
      if (h) tip = { text: "have a seat", x: ch.x, y: ch.y - 14 };
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
      const feeding = moving && inFire(L, moving.key, moving.at, moving.u);
      const fd = moving && feeding && L.desks.find((d) => d.key === moving.key);
      const onIt = !moving && !held && over(...r);
      if (onIt && !w.sparkOn) { w.sparkOn = true; w.sparks = performance.now(); } else if (!onIt) w.sparkOn = false;
      const since = Math.min(performance.now() - w.sparks, performance.now() - w.flare), burst = since < 1500 ? since / 1500 : -1;
      w.burning = w.burning.filter((o) => performance.now() - o.t0 < 1400);
      items.push([F.y1, () => drawFire(ctx, F, T, !!feeding, burst, w.burning)]);
      pics.push({ key: F.key, kind: "fire", x: F.x0, y: F.y1, fn: (x) => drawFire(x, F, T, false, burst, w.burning),
        pit: (x) => drawFire(x, F, T, false, -1, [], "pit"), flames: (x) => drawFire(x, F, T, !!feeding, burst, w.burning, "flames"), F });
      const tx = F.x0 + 16, ty = F.y0 - 2;
      if (fd) tip = { text: "burn " + fd.label + (!s.bots.some((b) => b.unit === fd.unit) ? "" : !L.booths.length ? ", delete its bots" : hardcore() ? ", shoot its bots" : ", its bots to the booth"), x: tx, y: ty };
      else if (feeding && moving && KIND(moving.key) === "door") { const m = s.machines.find((m) => "door:" + m.id === moving.key); tip = { text: "take out " + (m ? m.name : "the door"), x: tx, y: ty }; }
      else if (feeding && moving) tip = { text: "burn the " + (NAMES[KIND(moving.key)] || KIND(moving.key)), x: tx, y: ty };
    }
    // the booth: a bot let go over it goes in, and after five seconds out of the list for good; the
    // neuralyzer: a bot let go by it gets `/clear` (the drop is useAt's)
    for (const bo of L.booths) {
      if (hardcore()) {                     // hardcore: a gun closet; a click takes a gun and you see the office through your own eyes (fps.js)
        const out = [...w.folk.values()].filter((f) => f.carry === "gun").length + (fps && (w.folk.get("you") || {}).carry !== "gun" ? 1 : 0), h = hot(bo.x - 9, bo.y - 32, bo.x + 9, bo.y + 2, { kind: "aim", key: bo.key });
        const aimed = !!boothUnder && boothUnder.key === bo.key;    // a bot held over it: let go and it is shot
        items.push([bo.y - 3.5, () => outlined(ctx, foot("booth", [bo.x, bo.y]), h || danger(bo.key) || aimed, (c) => drawGunCloset(c, bo, false, out), danger(bo.key) || aimed ? RED : GOLD, tintOf(bo.key))]);
        items.push([bo.y - 2.5, () => drawGunCloset(ctx, bo, true, out)]);
        pics.push({ key: bo.key, kind: "booth", x: bo.x, y: bo.y, fn: (c) => { drawGunCloset(c, bo, false, out); drawGunCloset(c, bo, true, out); } });
        if (aimed) tip = { text: "shoot " + heldBot.b.name, x: bo.x, y: bo.y - 34 };
        else if (h) tip = { text: "gun closet: take a gun", x: bo.x, y: bo.y - 34 };
        continue;
      }
      // a bot in it: the whole of its five seconds a click on the booth (or on the bot) takes it back out
      const inside = [...w.folk.values()].find((f) => f.act.kind === "doom" && f.act.at === bo.key && f.b && !f.gone), t = boothT(inside);
      const fl = w.boothFlash && w.boothFlash.key === bo.key ? (performance.now() - w.boothFlash.t0) / 1200 : -1, flagged = !!boothUnder && boothUnder.key === bo.key;
      const h = !!inside && !inside.act.back && hot(bo.x - 9, bo.y - 32, bo.x + 9, bo.y + 2, { kind: "undo", id: inside.id });
      items.push([bo.y - 3.5, () => outlined(ctx, foot("booth", [bo.x, bo.y]), danger(bo.key) || flagged || h, (c) => drawBooth(c, bo, false, t, fl, flagged), danger(bo.key) || flagged ? RED : GOLD, tintOf(bo.key))]);
      items.push([bo.y - 2.5, () => drawBooth(ctx, bo, true, t, fl, flagged)]);
      pics.push({ key: bo.key, kind: "booth", x: bo.x, y: bo.y, fn: (c) => { drawBooth(c, bo, false, t, fl, false); drawBooth(c, bo, true, t, fl, false); } });
      if (flagged) tip = { text: "bye " + heldBot.b.name, x: bo.x, y: bo.y - 34 };
      else if (h) tip = { text: "keep " + inside.b.name, x: bo.x, y: bo.y - 34 };
      else if (on(bo.key) && !inside) tip = { text: "booth: a bot in it is deleted", x: bo.x, y: bo.y - 34 };
    }
    // the dog: a click on it (or on its house, where it naps) and you go and pet it, out of its house it comes to you;
    // a click while you pet it, or while it sits at your feet with the stick, throws the stick. It can be picked up
    const dog = w.dog, dogHots = [], dogTip = () => (canThrow(w) ? "throw the stick" : !dog.inside ? (dog.act.kind === "fetch" ? null : "pet the dog") : dog.act.hide ? "comfort the dog" : dog.asleep ? "wake the dog" : "call the dog");
    dogRect = null;
    for (const dh of L.doghouses) {
      const r = foot("doghouse", [dh.x, dh.y]);
      piece(dh, "doghouse", (c) => drawDoghouse(c, dh));
      if (dog && !dog.leaving && !dog.held && dh === L.doghouses[0]) dogHots.push([r, () => ({ text: dogTip(), x: dh.x, y: dh.y - 23 })]);
    }
    if (dog && !dog.leaving && dog.inside && L.doghouses[0]) {   // at home: its head out of the door (only its eyes, afraid), z's asleep
      const dh = L.doghouses[0];
      items.push([dh.y - 0.4, () => {
        if (dog.act.hide) { rect(ctx, dh.x - 2, dh.y - 5, 1, 1, C.steel4); rect(ctx, dh.x + 1, dh.y - 5, 1, 1, C.steel4); return; }
        const im = dogPic("lie", 0);
        ctx.drawImage(im, 10, 0, 8, 6, dh.x - 4, dh.y - 6, 8, 6);
        if (dog.act.left > 1e8 || dog.asleep) zzz(ctx, dog, dh.x + 3, dh.y - 9);
      }]);
    } else if (dog) {
      const im = dogPic(dog.pose, dog.wag), x0 = Math.round(dog.x - im.width / 2) + (dog.act.kind === "shake" && !still ? (Math.floor(w.T * 20) % 2 ? 1 : -1) : 0), y0 = Math.round(dog.y - im.height + 1);
      const r = [x0, y0, x0 + im.width, y0 + im.height + 1], h = !dog.leaving && !dog.held && over(...r) && !moving && !held;
      const you = w.folk.get("you"), by = you && Math.abs(you.x - dog.x) < 24 && Math.abs(you.y - dog.y) < 10 ? Math.round(Math.min(you.y, dog.y)) - 29 : y0 - 2;   // over your head when you are by it
      if (!dog.leaving && !dog.held) { dogRect = r; dogHots.push([r, () => ({ text: dogTip(), x: dog.x, y: by })]); }
      const st = w.stick, mouth = st && st.held && MOUTH[dog.pose];
      items.push([dog.held ? 1e9 : dog.y, () => {
        if (!dog.held) rect(ctx, x0 + 2, y0 + im.height - 1, im.width - 4, 1, "rgba(0,0,0,0.3)");
        ctx.save();
        if (dog.dir < 0) { ctx.translate(x0 * 2 + im.width, 0); ctx.scale(-1, 1); }
        if (h) ctx.drawImage(halo(im, GOLD), x0, y0);
        ctx.drawImage(im, x0, y0);
        if (mouth) { rect(ctx, x0 + mouth[0] - 2, y0 + mouth[1], 10, 2, INK); rect(ctx, x0 + mouth[0] - 1, y0 + mouth[1], 8, 1, STICK); }   // the stick, across its mouth
        ctx.restore();
        if (dog.barking && Math.floor(w.T * 5) % 2) {        // a bark: three short lines off its snout
          const sx = dog.dir < 0 ? x0 - 1 : x0 + im.width, d = dog.dir < 0 ? -1 : 1;
          ctx.fillStyle = INK; for (const [ox, oy] of [[1, -2], [2, 0], [1, 2]]) ctx.fillRect(sx + d * ox, y0 + 3 + oy, 1, 1);
        }
      }, dog.held ? null : [[x0 + 1, y0 + 1, x0 + im.width - 1, y0 + im.height]]]);
    }
    if (w.stick && !w.stick.held) {           // the stick: tumbling through the air over its shadow, or lying on the floor
      const st = w.stick, sx = Math.round(st.x), sy = Math.round(st.y), sh = Math.round(st.h || 0), turn = st.t < 1 && !still ? Math.floor(st.t * 10) % 4 : 0;
      items.push([st.y + 0.2, () => { if (sh > 0) rect(ctx, sx - 2, sy, 5, 1, "rgba(0,0,0,0.3)"); drawStick(ctx, sx, sy - sh - 1, turn); }]);
    }
    if (dog) for (const ht of dog.hearts) {                 // hearts, rising and gone
      const k = (performance.now() - ht.t0) / 1200;
      if (k >= 1) continue;
      items.push([1e8, () => { ctx.globalAlpha = 1 - k; heart(ctx, Math.round(ht.x), Math.round(ht.y - k * 10)); ctx.globalAlpha = 1; }]);
    }
    // the radio: the room's sound, while one stands here and it is on; a click switches it (its dial lit, notes rising)
    const soundOn = !!app.settings().sound && L.radios.length > 0;
    Sound.set(soundOn);
    for (const ra of L.radios) {
      const h = hot(...foot("radio", [ra.x, ra.y]), { kind: "sound", pan: Math.max(-1, Math.min(1, (ra.x / W) * 1.6 - 0.8)) });
      piece(ra, "radio", (c) => drawRadio(c, ra, soundOn));
      if (soundOn && !still) items.push([ra.y + 0.1, () => drawNotes(ctx, ra, T)]);
      if (h) tip = { text: soundOn ? "sound on" : "sound off", x: ra.x, y: ra.y - 22 };
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
    // the limits, in the coffee pot (this five-hour session) and the cooler's bottle (this week); a
    // click on either opens the whole picture
    const c0 = L.coffeeAt;
    if (c0 && hot(c0.x, c0.y - 22, c0.x + 20, c0.y + 1, { kind: "limits" })) tip = { text: say(usage.session, "5h"), x: c0.x + 10, y: c0.y - 24 };
    if (L.cooler && hot(L.cooler.x - 5, L.cooler.y - 26, L.cooler.x + 5, L.cooler.y + 1, { kind: "limits" })) tip = { text: say(usage.week, "1w"), x: L.cooler.x, y: L.cooler.y - 24 };
    for (const [r, t] of dogHots) if (hot(...r, { kind: "pet" })) { const tt = t(); if (tt.text) tip = tt; }   // the dog's last: it stands in front of the pieces
    hovered = null;
    for (const f of w.folk.values()) if (f.lying) drawSleeper(ctx, f, f.b, T, hl);
    items.sort((a, b) => a[0] - b[0]);
    // back to front; a figure under the pointer stops being the one under it when something solid is drawn over
    // it there (a bot behind a desk: a click on the desk's screen is the screen's)
    for (const [, d, cover] of items) {
      const before = hovered;
      d();
      if (before && hovered === before && cover && cover.some((r) => over(...r))) hovered = null;
    }
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
    if (fps && !hardcore()) fps.leave();      // no guns outside hardcore
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
    for (const f of w.folk.values()) if (f.chat > 0 && !still && !f.waiting && !(f.act.kind === "chess" && !f.human)) {   // (a robot at the chess board plays in silence)
      const bx = Math.round(f.x) + 3, by = Math.round(f.y) - (f.lying ? 20 : 27) - 6;
      rect(ctx, bx - 1, by - 1, 11, 7, INK); rect(ctx, bx, by, 9, 5, C.steel4); rect(ctx, bx, by + 5, 1, 1, C.steel4);
      for (let i = 0; i < 3; i++) if ((Math.floor(f.chat * 4) + i) % 3) rect(ctx, bx + 2 + i * 2, by + 2, 1, 1, INK);
    }
    // the bot under the pointer: its name over its head, and a click opens it
    // (one about to be stopped has its sign instead of a name, and a click on it is the undo)
    if (hovered && !moving) {
      const { b, x, y } = hovered, hf = w.folk.get(b.id), doomed = !!hf && hf.act.kind === "doom";
      const dg = app.digest(), wb = b.staff && dg && dg.on && dg.writing && !dg.writing.startsWith("u:") ? (all.find((x) => x.id === dg.writing) || {}).name : null;
      const hf2 = b.staff && w.folk.get(b.id), job = hf2 && (hf2.hunt && hf2.hunt.id ? ["after", hf2.hunt.id] : hf2.clean ? ["seeing out", hf2.clean.id] : null), cl = job && w.folk.get(job[1]);
      if (cl && cl.b) tip = { text: b.name + " - " + job[0] + " " + cl.b.name, x, y: y - 2 };
      else if (b.staff) tip = { text: b.name + (b.style !== "notes" ? (b.waiting ? " - asks you" : b.busy ? " - at work" : !b.alive ? " - dozing" : " - office manager") : wb ? " - notes on " + wb : dg && dg.writing ? " - a project's notes" : " - notes"), x, y: y - 2 };
      else if (!doomed) tip = { text: b.name + (b.waiting ? " - needs input" : ""), x, y: y - 2 };
      hots.push({ x0: -1e9, y0: -1e9, x1: 1e9, y1: 1e9, action: b.staff ? { kind: "manager", key: b.booth } : b.human ? { kind: "you" } : doomed ? { kind: "undo", id: b.id } : { kind: "open", id: b.id } });
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

  // ---- the office in the first person (fps.js): what is in the room right now, for props3d.js to make its 3d
  // things of: the floor and what lies on it (an overlay, two texels a dot), what hangs on the wall, every desk and
  // piece of furniture with what it shows (the coffee left, a screen at work, the game on the chess board, the
  // dream on the whiteboard), the lights (the fire, a booth in use), and everyone in the pose the room gives them
  const canvases = new Map();
  function canvas(name, w, h) {              // one per name, redrawn in place (its `v` says it was)
    let c = canvases.get(name);
    if (!c || c.width !== w || c.height !== h) { c = document.createElement("canvas"); c.width = w; c.height = h; c.v = 0; canvases.set(name, c); }
    const x = c.getContext("2d", { willReadFrequently: true });
    x.setTransform(1, 0, 0, 1, 0, 0); x.clearRect(0, 0, w, h); x.imageSmoothingEnabled = false;
    c.v++;
    return [c, x];
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
  // what a desk's screen shows (as drawDesk paints it): off, on (its sitter idles), busy, waits; a shell keeps it on
  function screenOf(d, w, tree) {
    const seated = (f) => f && f.legs === "sit" && !f.lying && (f.act.kind === "desk" || f.act.kind === "sleep") && f.b && f.b.alive !== false;
    const f = d.seats.filter((st) => st[2] === tree).map(([id]) => w.folk.get(id)).filter(seated);
    if (f.some((x) => x.busy)) return "busy";
    if (f.some((x) => x.waiting)) return "waits";
    return f.length || app.shells().some((s) => s.m === (d.m || "") && s.dir === (tree || d.dir)) ? "on" : "off";
  }
  let overlayKey = null;
  function scene3(L, w, s, T) {
    const things = [], boxes = [], solid = [], lights = [];
    const add = (o) => { if (!o) return false; boxes.push(...o.boxes); things.push(...o.things); solid.push(...o.solid); lights.push(...o.lights); return true; };
    // the floor's overlay: the rug, the mats rolled out for a nap, the oil of the shot ones, the fire's pit, a soft
    // shadow under each piece; redrawn only when one of them changes
    const mats = [...w.folk.values()].filter((f) => f.act.kind === "sleep" && f.act.there && f.act.at).map((f) => f.act.at);
    const pieces = pics.filter((p) => !ON_WALL.has(p.kind));
    const okey = JSON.stringify([W, H, L.pong && [L.pong.x, L.pong.y], mats, [...w.remains.values()].map((r) => [r.x, r.y, r.parts.length, r.drops.length]), L.fire && [L.fire.x0, L.fire.y0], pieces.map((p) => [p.kind, p.x, p.y]), L.desks.map((d) => [d.x0, d.y, d.w])]);
    let fc = canvases.get("floor");
    if (!fc || okey !== overlayKey) {
      overlayKey = okey;
      const [c, fx] = canvas("floor", W * 2, H * 2);
      fc = c; fx.setTransform(2, 0, 0, 2, 0, 0);
      fx.fillStyle = "rgba(0,0,0,0.3)";
      const blob = (cx, cy, rx, ry) => { fx.beginPath(); fx.ellipse(cx, cy, rx, ry, 0, 0, 7); fx.fill(); };
      for (const p of pieces) if (p.kind !== "fire") { const r = foot(p.kind, [p.x, p.y]); blob((r[0] + r[2]) / 2, p.y - 3, (r[2] - r[0]) / 2 + 1, 5); }
      for (const d of L.desks) blob(d.x0 + d.w / 2, d.y - 6, d.w / 2 + 1, 7);
      if (L.pong) drawRug(fx, L.pong);
      for (const m of mats) drawMat(fx, m);
      for (const r of w.remains.values()) {
        const rx = Math.round(r.puddle * r.parts.length / r.total);
        if (rx > 1) ELL(rx, 3, 5).forEach((hw, j) => rect(fx, r.x - hw, r.y - 2 + j, 2 * hw, 1, "#17151f"));
        for (const d of r.drops) rect(fx, d.x, d.y, d.s, d.s, d.hi ? "#4a4766" : "#17151f");
      }
      if (L.fire) fx.drawImage(Props3D.pit(44, 18), L.fire.x0 + 2, L.fire.y1 - 21, 44, 18);
    }
    // the wall: the doors in their paint (boarded while their machine is out of reach), the window, the clock, the
    // closet, the switchboard, where they hang
    const now = new Date(), aw = s.awake || {};
    const items = [
      ...L.doors.map((d) => ({ kind: "door", x0: d.x0, x1: d.x0 + 16, y0: d.y0, y1: WALL, paint: paintOf(d.mid), ok: !d.m.error })),
      ...L.windows.map((p) => ({ kind: "window", x0: p.x0, x1: p.x1, y0: p.y0, y1: p.y1, hour: now.getHours() })),
      ...(L.clock ? [{ kind: "clock", x0: L.clock.x0, x1: L.clock.x0 + 17, y0: L.clock.y0, y1: L.clock.y0 + 17, time: now, awake: !!aw.on }] : []),
      ...[L.closet && "closet", L.settings && "settings"].filter(Boolean).map((k) => ({ kind: k, x0: L[k].x0, x1: L[k].x1, y0: L[k].y0, y1: L[k].y1 })),
    ];
    const wkey = JSON.stringify(items.map((it) => [it.kind, it.x0, it.y0, it.paint && it.paint.top, it.ok])) + now.getHours() + ":" + now.getMinutes() + !!aw.on;
    // the desks: their cabinet, desktop, screen (and laptops) and chairs, what each screen shows
    for (const d of L.desks) add(Props3D.desk({ x0: d.x0, y: d.y, w: d.w, cab: d.cab, label: d.label, paint: paintOf(d.m), color: d.color, T,
      seats: d.seats.map(([id, x, tree]) => ({ x, tree })), mugs: d.seats.filter(([id]) => w.mugs.get(id) > T && w.folk.get(id)).map(([id, x]) => ({ x, color: colorOf(w.folk.get(id).b) })),
      screen: screenOf(d, w, ""), laptops: d.trees.map((t) => screenOf(d, w, t)) }));
    // every piece of furniture with what it shows; the fire's flames (a light of their own), its pit on the floor
    const usage = s.usage || {}, left = (u) => (u && u.used !== undefined ? Math.max(0, 1 - u.used / 100) : 0);
    for (const p of pics) {
      if (ON_WALL.has(p.kind)) continue;
      if (p.kind === "fire") {
        const F = L.fire, fl = Props3D.flames(Math.floor(T * 8) % 4, w.burning.length > 0 || performance.now() - w.flare < 1500);
        things.push({ x: F.x0 + 24, y: F.y1 - 12, z: 0, c: fl.c, sx: 0, sy: 0, sw: fl.c.width, sh: fl.c.height, ax: fl.ax, ay: fl.ay, res: fl.res, piece: p.key });
        lights.push({ x: F.x0 + 24, y: F.y1 - 12, z: 9, r: 140, col: [1, 0.8, 0.58], k: 0.9, flicker: true });   // warm, not orange: Doom's light has hardly a colour
        solid.push([F.x0 + 3, F.y1 - 20, F.x1 - 3, F.y1 - 3]);
        continue;
      }
      const st = { kind: p.kind, key: p.key, x: p.x, y: p.y, T };
      if (p.kind === "coffee") { st.level = left(usage.session); st.brewing = [...w.folk.values()].some((f) => f.act.kind === "coffee" && f.act.stage === 1); }
      if (p.kind === "cooler") st.level = left(usage.week);
      if (p.kind === "printer") { const job = [...w.folk.values()].find((f) => f.act.kind === "print" && f.act.stage === 1), since = (performance.now() - w.print) / 1000; st.printing = job ? job.act.wait : since < 3 ? since : undefined; }
      if (p.kind === "manager") {
        const M = (s.managers || []).find((x) => x.key === p.key) || {}, dg = app.digest(), live = new Set((s.bots || []).map((b) => b.id));
        Object.assign(st, M.vacant ? { name: "", color: M.color, screen: M.session ? "off" : null, notes: 0 } : { name: M.name, color: M.color, screen: M.session ? (M.busy ? "busy" : M.waiting ? "asks" : M.alive ? "on" : "off") : null, notes: M.style === "notes" && dg ? Object.keys(dg.sessions || {}).filter((id) => live.has(id)).length : 0 });
      }
      if (p.kind === "add") st.w = ADD_W;
      if (p.kind === "booth") {
        const inside = [...w.folk.values()].find((f) => f.act.kind === "doom" && f.act.at === p.key);
        Object.assign(st, { gun: hardcore(), out: [...w.folk.values()].filter((f) => f.carry === "gun").length + (fps ? 1 : 0), t: boothT(inside) });
      }
      if (p.kind === "neuralyzer") st.flash = !!w.flash && performance.now() - w.flash.t0 < 400;
      if (p.kind === "radio") st.on = !!app.settings().sound;
      if (p.kind === "chess") { const g = s.chess && !s.chess.result ? s.chess : null; st.board = g ? g.board : START; st.seats = [L.chessSeat(0), L.chessSeat(1)]; }
      if (p.kind === "meet") st.seats = [L.meetSeat(0), L.meetSeat(1)];
      if (p.kind === "board") { st.ink = ink(w); st.inkV = w.inkV; }
      if (p.kind === "plant") st.variant = +(p.key.match(/\d+$/) || [0])[0] % 2;
      if (p.kind === "pair") st.linked = !!(s.pair || {})[p.key];
      if (add(Props3D.build(st))) continue;
      // anything else: the room's own picture of it, standing where it stands
      const r = foot(p.kind, [p.x, p.y]), P = 1, cw = r[2] - r[0] + 2 * P, ch = r[3] - r[1] + 2 * P;
      const [c, x] = canvas("p:" + p.key, cw, ch);
      x.translate(P - r[0], P - r[1]); p.fn(x);
      const cx = (r[0] + r[2]) / 2;
      things.push({ piece: p.key, x: cx, y: p.y - 3, z: 0, c, sx: 0, sy: 0, sw: cw, sh: ch, ax: cx - r[0] + P, ay: p.y + 1 - r[1] + P });
      solid.push([r[0] + 2, Math.max(r[1], p.y - 10), r[2] - 2, p.y + 2]);
    }
    // everyone but you (you are the eyes): a model of each, in the pose the room gives it, seen from whichever of
    // its eight sides faces you; lying on a mat; in the booth going up in its light; a ? over one that waits
    for (const f of w.folk.values()) {
      if (f.id === "you" || f.held || !f.b) continue;
      const b = f.b, fig = { kind: "robot", seed: seedOf(b), color: colorOf(b), femme: !!b.staff, arms: f.arms, legs: f.legs };
      const who = { key: f.id, bot: b.human ? null : b.id, staff: !!b.staff, booth: b.booth, name: b.name };
      if (f.lying) { const l = Props3D.lying(fig); things.push({ ...who, x: f.x, y: f.y - 9, z: 0, c: l.c, sx: 0, sy: 0, sw: l.w, sh: l.h, ax: l.ax, ay: l.ay, res: l.res }); continue; }
      const bt = boothT(f);
      const atDesk = b.staff && f.sat && Math.hypot(f.x - managerSeat(boothOf(L, f)).x, f.y - managerSeat(boothOf(L, f)).y) < 2;   // an office manager at her counter faces the room
      things.push({ ...who, x: f.x, y: f.y, z: 0, fig, face: f.dir < 0 ? -1 : 1, facing: atDesk ? Math.PI / 2 : undefined, seated: f.legs === "sit", dissolve: bt > 0 ? boothRise(bt) : 0 });
      if (f.waiting && f.act.kind !== "doom") {
        const q = small("?3", 27, 30, (x) => { x.scale(3, 3); rect(x, 0, 0, 9, 9, "#2b2433"); rect(x, 1, 1, 7, 7, "#f4f6fb"); rect(x, 1, 8, 1, 1, "#f4f6fb"); rect(x, 0, 9, 3, 1, "#2b2433"); tiny(x, "?", 3, 2, "#2b2433"); });
        things.push({ key: "?" + f.id, x: f.x, y: f.y + 0.1, z: 28, c: q, sx: 0, sy: 0, sw: 27, sh: 30, ax: 13.5, ay: 30, res: 3 });
      }
    }
    // what is left of a shot bot: its pieces where they fly and land, cut from its model seen from the front
    for (const [id, r] of w.remains) r.parts.forEach((p, i) => {
      const who = { key: id + "/" + i, rem: r.back ? null : id, name: r.b.name, x: p.x, y: p.y, z: p.z, flip: Math.floor(p.spin) % 2 === 1 };
      if (p.img) {
        const fr = Props3D.figure({ kind: "robot", seed: seedOf(r.b), color: colorOf(r.b), femme: !!r.b.staff, arms: "down", legs: "stand" }, 0), S = fr.res;
        const sx = Math.max(0, Math.round(fr.ax + (p.sx - 11) * S)), sy = Math.max(0, Math.round(fr.ay + (p.sy - 27) * S));
        things.push({ ...who, c: fr.c, sx, sy, sw: Math.min(fr.w - sx, p.sw * S), sh: Math.min(fr.h - sy, p.sh * S), ax: p.sw * S / 2, ay: p.sh * S, res: S });
      } else { const c = bitOf(p); things.push({ ...who, c, sx: 0, sy: 0, sw: c.width, sh: c.height, ax: c.width / 2, ay: c.height }); }
    });
    const dg = w.dog;
    if (dg && !dg.inside && !dg.leaving && !dg.held) things.push({ key: "dog", x: dg.x, y: dg.y, z: 0, fig: { kind: "dog", pose: dg.pose, wag: dg.wag }, face: dg.dir < 0 ? -1 : 1 });
    if (w.ball) things.push({ key: "ball", x: w.ball.x, y: w.ball.y, z: w.ball.h, c: small("ball4", 12, 12, (x) => { x.fillStyle = "#e8ecf0"; x.beginPath(); x.arc(6, 6, 5.5, 0, 7); x.fill(); x.fillStyle = "#b5bfc7"; x.beginPath(); x.arc(7, 7.5, 3.5, 0, 7); x.fill(); x.fillStyle = "#ffffff"; x.fillRect(3, 3, 2, 2); }), sx: 0, sy: 0, sw: 12, sh: 12, ax: 6, ay: 12, res: 4 });
    return { v: (view ? view.v : 0) + 1, W, H, WALL, floor: fc, wall: Props3D.wall(W, 40, WALL, items, wkey), plain: Props3D.plain(40), items, wkey, boxes, things, solid, lights };
  }
  // a shot fired: the idle bots near it run off, hands up (the ones at work work on); the more afraid, the
  // further off they hear it and the further they run
  function alarm(x, y) {
    const L = lastL, fear = world().fear || 0;
    for (const f of world().folk.values()) {
      const d = Math.hypot(f.x - x, f.y - y) || 1;
      if (!L || !idleBot(f) || d > 110 + 140 * fear) continue;   // (the ones let go of stay put)
      flee(f, L, x, y, rnd(40, 80) * (1 + fear));
    }
  }
  // Hardcore: the bots are afraid, more with every kill (FEAR.kill a shot, 1 at most), less as minutes go by
  // without one (FEAR.fade seconds from 1 to nothing). It shows only in what they do: a little afraid they
  // tremble; more, and they sweat and keep their distance from whoever holds a gun, hands up; more still, a
  // break is spent hiding behind the furniture furthest from the gun closet, peeking out (hide), and the ones
  // at work glance over their shoulder now and then. Nobody likes to come and sweep up then
  const FEAR = { kill: 0.25, fade: 240 };
  const idleBot = (f) => !!f.b && !f.b.human && !f.b.staff && !f.work && !f.held && !f.leaving && !f.open && !f.lying && !DOOMED.has(f.act.kind) && f.act.kind !== "escort";
  function flee(f, L, x, y, k) {
    const d = Math.hypot(f.x - x, f.y - y) || 1;
    end(f);
    f.act = { kind: "loiter", x: Math.max(L.floor.x0, Math.min(L.floor.x1, f.x + (f.x - x) / d * k)), y: Math.max(L.floor.y0, Math.min(L.floor.y1, f.y + (f.y - y) / d * k)), left: rnd(2, 4), v: 60, panic: true };
  }
  function scare(w, L) {
    if (!(w.fear > 0.35)) return;
    const guns = [...w.folk.values()].filter((g) => g.carry === "gun" && !g.leaving), reach = 24 + 70 * w.fear;
    for (const f of w.folk.values()) {
      if (!idleBot(f) || (f.act.kind === "loiter" && f.act.panic)) continue;
      const g = guns.find((g) => Math.hypot(g.x - f.x, g.y - f.y) < reach);
      if (g) flee(f, L, g.x, g.y, reach);
    }
  }
  // how far above its anchor a kind of piece is drawn (its topmost opaque row), measured once from its picture
  const tops = new Map();
  function visTop(kind) {
    if (tops.has(kind)) return tops.get(kind);
    const r = foot(kind, [0, 0]), c = document.createElement("canvas"), w = r[2] - r[0] + 8, h = r[3] - r[1] + 8;
    c.width = w; c.height = h;
    const x = c.getContext("2d", { willReadFrequently: true });
    x.translate(4 - r[0], 4 - r[1]); drawKind(x, kind, [0, 0], { board: [], folk: new Map(), pong: { t: 0, from: 0 } }, 0);
    const d = x.getImageData(0, 0, w, h).data;
    let top = r[1];
    for (let j = 0; j < h; j++) if ([...Array(w).keys()].some((i) => d[(j * w + i) * 4 + 3] > 128)) { top = j - 4 + r[1]; break; }
    if (sprites.get("PLANT")) tops.set(kind, top);         // (once the sprites are in)
    return top;
  }
  // The dog (the dog house, from the closet): a dachshund, a badger dog, as burrows go. It lives on the floor while
  // its house is in the room: naps in it, its head out of the door; trots about and sniffs; visits a bot or you and
  // sits by them wagging (a bot that is idle pets it); follows you a while; chases the ball while bots play football;
  // barks at whoever comes in through a door; and a click on it sends you over to pet it (hearts). At night (23 to
  // 7 here), while you are away, and in hardcore while the bots are afraid (only its eyes in the dark door), it
  // stays in its house. Its house burnt, it trots out through the door
  const DOG = { b: "#c27a3e", d: "#7a4424", w: "#efd8b4", r: "#d9473a", n: "#1a1626", e: "#1a1626" };
  const DOGS = {                             // facing right; its tail is put on by `wag`
    stand: ["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "..bb......bb...."],
    walk1: ["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "...bb......bb..."],
    walk2: ["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "..b.b.....b.b..."],
    sit: [".......bbb..", "......bbbebb", "......dbbbbn", "......rbb...", "...bbbbbb...", "..bbbbbbw...", "..bb..bb...."],
    lie: ["...........bbb..", "..........bbbdbb", ".bbbbbbbbbdbbbbn", ".bbbbbbbbbbbbw.."],
  };
  const TAIL = { stand: [[[1, 3], [0, 2]], [[1, 3], [1, 2]]], walk1: [[[1, 3], [0, 2]], [[1, 3], [1, 2]]], walk2: [[[1, 3], [0, 2]], [[1, 3], [1, 2]]],
    sit: [[[1, 5], [0, 6]], [[1, 5], [0, 5]]], lie: [[[0, 3]], [[0, 2]]] };
  const dogPics = new Map();
  function dogPic(pose, wag) {
    const key = pose + (wag ? 1 : 0);
    if (dogPics.has(key)) return dogPics.get(key);
    const rows = DOGS[pose], w = Math.max(...rows.map((r) => r.length)) + 2, h = rows.length + 2, px = new Map();
    rows.forEach((r, y) => [...r].forEach((ch, x) => { if (DOG[ch]) px.set((y + 1) * w + x + 1, DOG[ch]); }));
    for (const [x, y] of TAIL[pose][wag ? 1 : 0]) px.set((y + 1) * w + x + 1, DOG.d);
    const c = document.createElement("canvas"); c.width = w; c.height = h;
    const x = c.getContext("2d");
    x.fillStyle = INK;
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++)
      if (!px.has(j * w + i) && [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => i + dx >= 0 && i + dx < w && px.has((j + dy) * w + i + dx))) x.fillRect(i, j, 1, 1);
    for (const [k, col] of px) { x.fillStyle = col; x.fillRect(k % w, Math.floor(k / w), 1, 1); }
    dogPics.set(key, c);
    return c;
  }
  function heart(ctx, x, y) {
    rect(ctx, x - 2, y - 1, 2, 1, RED); rect(ctx, x + 1, y - 1, 2, 1, RED); rect(ctx, x - 2, y, 5, 1, RED); rect(ctx, x - 1, y + 1, 3, 1, RED); rect(ctx, x, y + 2, 1, 1, RED);
  }
  // the stick you throw for it: tumbling (four turns) on the way, lying on the floor, across its mouth on the way back
  const STICK = "#e0bd84", MOUTH = { stand: [12, 4], walk1: [12, 4], walk2: [12, 4], sit: [8, 4], lie: [12, 4] };
  function drawStick(ctx, x, y, turn) {
    const pts = [[[-3, 0], [-2, 0], [-1, 0], [0, 0], [1, 0], [2, 0]], [[-2, -2], [-1, -1], [0, 0], [1, 1], [2, 2]], [[0, -3], [0, -2], [0, -1], [0, 0], [0, 1], [0, 2]], [[-2, 2], [-1, 1], [0, 0], [1, -1], [2, -2]]][turn];
    for (const [i, j] of pts) rect(ctx, x + i - 1, y + j - 1, 3, 3, INK);
    for (const [i, j] of pts) rect(ctx, x + i, y + j, 1, 1, STICK);
  }
  // a throw: when you are petting it, or it sits at your feet with the stick it brought back
  function canThrow(w) {
    const you = w.folk.get("you"), g = w.dog;
    return !!you && !!g && !g.inside && !g.held && !g.leaving && ((you.act.kind === "pet" && you.act.there) || (you.act.kind === "fetch" && g.act.kind === "fetch" && g.act.stage === "wait"));
  }
  function throwStick(w, you, g, L) {
    const fl = L.floor, mid = (fl.x0 + fl.x1) / 2;
    let to = { x: Math.round(mid), y: Math.round((fl.y0 + fl.y1) / 2) };
    for (let k = 0; k < 40; k++) {           // out into the room, somewhere clear 40 to 80 dots off
      const a = (you.x > mid ? Math.PI : 0) + rnd(-0.9, 0.9), d = rnd(40, 80);
      const x = Math.round(you.x + Math.cos(a) * d), y = Math.round(you.y + Math.sin(a) * d * 0.7);
      if (x > fl.x0 + 6 && x < fl.x1 - 6 && y > fl.y0 + 8 && y < fl.y1 - 2 && L.free(x, y)) { to = { x, y }; break; }
    }
    end(you); you.dir = to.x < you.x ? -1 : 1; you.act = { kind: "fetch", at: w.T, chosen: true };
    w.stick = { x: you.x + you.dir * 6, y: you.y, x0: you.x + you.dir * 6, y0: you.y, x1: to.x, y1: to.y, t: 0, h: 12, held: false };
    g.act = { kind: "fetch", stage: "run" };
  }
  function walkDog(g, to, v, dt) {
    const dx = to.x - g.x, dy = to.y - g.y, d = Math.hypot(dx, dy);
    if (d <= v * dt) { g.x = to.x; g.y = to.y; return true; }
    g.x += (dx / d) * v * dt; g.y += (dy / d) * v * dt;
    if (Math.abs(dx) > 0.5) g.dir = Math.sign(dx);
    g.step += dt * 10; g.pose = Math.floor(g.step) % 2 ? "walk1" : "walk2";
    return false;
  }
  function nextDog(w, g, L) {
    const r = Math.random(), folk = [...w.folk.values()].filter((f) => f.b && !f.held && !f.leaving && !f.lying && !f.b.staff && !DOOMED.has(f.act.kind));
    const bots = folk.filter((f) => !f.human), you = folk.find((f) => f.human);
    if (r < 0.3) { const p = freeAt(L, "dog" + Math.floor(w.T)); return { kind: "wander", to: p, sniff: rnd(1.5, 4) }; }
    if (r < 0.5 && bots.length) return { kind: "visit", to: any(bots).id, left: rnd(6, 14) };
    if (r < 0.65 && you) return { kind: "visit", to: "you", left: rnd(6, 12) };
    if (r < 0.8 && you) return { kind: "follow", left: rnd(15, 30) };
    return { kind: "home", left: rnd(20, 60) };
  }
  function thinkDog(w, L, dt) {
    const h = L.doghouses[0];
    let g = w.dog;
    if (!h) {                                // its house is gone: out through the door
      if (!g) return;
      if (g.held) { g.held = false; dogHeld = false; }
      g.inside = false; g.leaving = true; g.barking = false;
      if (walkDog(g, { x: doorX(""), y: WALL + 2 }, 40, dt)) w.dog = null;
      return;
    }
    const door = { x: h.x, y: h.y + 3 };
    if (!g || g.leaving) g = w.dog = { x: door.x, y: door.y, dir: 1, pose: "stand", wag: 0, step: 0, z: 0, inside: true, hearts: [], act: { kind: "home", left: rnd(3, 10) } };
    g.z += dt; g.barking = false;
    g.hearts = g.hearts.filter((x) => performance.now() - x.t0 < 1200);
    const folk = w.folk, T = w.T, hour = new Date().getHours();
    const sleepy = hour >= 23 || hour < 7 || app.away() > 60, afraid = (w.fear || 0) > 0.4;
    const love = (x, y) => { if (!g.loveAt || T - g.loveAt > 0.7) { g.loveAt = T; g.hearts.push({ x: x + rnd(-3, 3), y: y, t0: performance.now() }); } };
    const st = w.stick;
    if (st && !st.held && st.t < 1) {         // the stick in the air: from your hand, in an arc, down on the floor
      st.t = Math.min(1, st.t + dt / 0.8);
      st.x = st.x0 + (st.x1 - st.x0) * st.t; st.y = st.y0 + (st.y1 - st.y0) * st.t; st.h = 12 * (1 - st.t) + Math.sin(Math.PI * st.t) * 22;
    }
    if (g.held) { g.pose = "stand"; g.wag = Math.floor(T * 8) % 2; g.inside = false; return; }   // in your hand: dangling, wagging
    // with you (petted, a game of fetch) it stays up at night as well; afraid, it stays in its house whatever you do
    const withYou = !afraid && ["petted", "fetch", "shake"].includes(g.act.kind);
    if ((sleepy || afraid) && !withYou && !(g.act.kind === "home" && g.act.left > 1e8)) g.act = { kind: "home", left: 1e9, hide: afraid };
    else if (!sleepy && !afraid && g.act.kind === "home" && g.act.left > 1e8) g.act = { kind: "home", left: rnd(1, 4) };
    if (!sleepy && !afraid && !withYou) {
      if (w.ball && g.act.kind !== "chase") g.act = { kind: "chase" };
      else if (w.dogNews && g.act.kind !== "bark") { const n = folk.get(w.dogNews); if (n) g.act = { kind: "bark", to: n.id, left: 3 }; }
    }
    w.dogNews = null;
    const a = g.act;
    if (a.kind !== "home") { g.inside = false; g.asleep = false; }
    g.wag = Math.floor(T * (["visit", "petted", "follow", "chase", "fetch"].includes(a.kind) ? 8 : 1.5)) % 2;
    switch (a.kind) {
      case "home": {                         // in its house: in through the door, a nap there; you reaching in to it, afraid: hearts
        if (!g.inside && !walkDog(g, door, 34, dt)) break;
        g.inside = true; g.x = door.x; g.y = door.y; g.pose = "lie"; g.dir = 1; g.asleep = a.left > 1e8 || sleepy;   // (with its house, if that is moved)
        const p = folk.get("you");
        if (a.hide && p && p.act.kind === "pet" && p.act.there) love(h.x, h.y - 12);
        if ((a.left -= dt) <= 0) { g.inside = false; g.asleep = false; g.act = nextDog(w, g, L); }
        break;
      }
      case "shake":                          // put down: a shake, then about its business (into its house, if it was put down there)
        g.pose = "stand";
        if ((a.left -= dt) <= 0) g.act = a.next || nextDog(w, g, L);
        break;
      case "fetch": {                        // after the stick you threw, back with it to your feet, then sitting there for the next throw
        const f = folk.get("you");
        if (!f || !st || f.held || f.act.kind !== "fetch") { if (st && st.held) { st.held = false; st.x = g.x; st.y = g.y; st.t = 1; st.h = 0; } g.act = nextDog(w, g, L); break; }
        if (a.stage === "run") {             // off as soon as it flies, to where it comes down; there first, it watches it come
          const to = { x: st.x1 + (g.x < st.x1 ? -7 : 7), y: st.y1 + 1 }, there = walkDog(g, to, 62, dt);
          if (there && st.t >= 1) { a.stage = "pick"; a.wait = 0; }
          else if (there) { g.pose = "stand"; g.dir = st.x < g.x ? -1 : 1; }
        } else if (a.stage === "pick") {
          g.pose = "stand"; g.dir = st.x < g.x ? -1 : 1;
          if ((a.wait += dt) > 0.3) { st.held = true; a.stage = "back"; }
        } else if (a.stage === "back") {     // to your feet, the stick across its mouth; dropped there
          const side = g.x < f.x ? -1 : 1;
          if (walkDog(g, { x: f.x + side * 13, y: f.y + 1 }, 48, dt)) { st.held = false; st.x = g.x - side * 6; st.y = g.y + 1; st.t = 1; st.h = 0; a.stage = "wait"; a.left = 12; }
        } else {                             // at your feet: sitting, wagging, now and then a heart; another throw, or it wanders off
          g.pose = "sit"; g.dir = f.x < g.x ? -1 : 1;
          if (Math.random() < dt * 0.7) love(g.x, g.y - 10);
          if ((a.left -= dt) <= 0) g.act = nextDog(w, g, L);
        }
        break;
      }
      case "wander":                         // somewhere on the floor, a sniff there
        if (!walkDog(g, a.to, 26, dt)) break;
        g.pose = "stand";
        if ((a.sniff -= dt) <= 0) g.act = nextDog(w, g, L);
        break;
      case "visit": {                        // beside a bot or you, sitting, wagging; an idle bot pets it
        const f = folk.get(a.to);
        if (!f || f.leaving || f.held) { g.act = nextDog(w, g, L); break; }
        const side = f.x > (L.floor.x0 + L.floor.x1) / 2 ? -1 : 1, spot = { x: f.x + side * 13, y: f.y + 1 };
        if (Math.hypot(spot.x - g.x, spot.y - g.y) > 3) { walkDog(g, spot, 32, dt); break; }
        g.pose = "sit"; g.dir = -side;
        if (!f.human && !f.work && f.legs === "stand" && !f.sat) { f.dir = side; f.arms = Math.floor(T * 3) % 2 ? "reach" : "down"; love(g.x, g.y - 10); }
        if (f.human && Math.random() < dt * 0.6) love(g.x, g.y - 10);
        if ((a.left -= dt) <= 0) g.act = nextDog(w, g, L);
        break;
      }
      case "follow": {                       // at your heels a while
        const f = folk.get("you");
        if (!f || f.held) { g.act = nextDog(w, g, L); break; }
        const spot = { x: Math.max(L.floor.x0, Math.min(L.floor.x1, f.x - (f.dir || 1) * 14)), y: f.y + 2 };
        if (Math.hypot(spot.x - g.x, spot.y - g.y) > 6) walkDog(g, spot, 38, dt);
        else { g.pose = "sit"; g.dir = f.x < g.x ? -1 : 1; }
        if ((a.left -= dt) <= 0) g.act = nextDog(w, g, L);
        break;
      }
      case "chase": {                        // after the ball the bots kick about
        if (!w.ball) { g.act = nextDog(w, g, L); break; }
        const b = w.ball;
        if (Math.hypot(b.x - g.x, b.y - g.y) > 5) walkDog(g, { x: b.x + (g.x < b.x ? -5 : 5), y: b.y + 1 }, 55, dt);
        else { g.pose = "stand"; g.dir = b.x < g.x ? -1 : 1; g.barking = Math.floor(T * 1.5) % 3 === 0; }
        break;
      }
      case "bark": {                         // at a newcomer, from a little way off
        const f = folk.get(a.to);
        if (!f) { g.act = nextDog(w, g, L); break; }
        if (Math.hypot(f.x - g.x, f.y - g.y) > 24) { walkDog(g, { x: f.x + (g.x < f.x ? -20 : 20), y: f.y + 1 }, 50, dt); break; }
        g.pose = "stand"; g.dir = f.x < g.x ? -1 : 1; g.barking = true;
        if (!a.yap) { sfx("bark", g.x); a.yap = true; }   // once: a newcomer is the event
        if ((a.left -= dt) <= 0) g.act = nextDog(w, g, L);
        break;
      }
      case "petted": {                       // sitting for you while you pet it
        const f = folk.get("you");
        if (!f || f.act.kind !== "pet") { g.act = nextDog(w, g, L); break; }
        if (a.out && !walkDog(g, { x: door.x, y: Math.min(L.floor.y1, door.y + 10) }, 30, dt)) break;   // called out of its house: a few steps out first
        a.out = false;
        g.pose = "sit"; g.dir = f.x < g.x ? -1 : 1;
        if (Math.hypot(f.x - g.x, f.y - g.y) < 16) love(g.x, g.y - 10);
        break;
      }
      default: g.act = nextDog(w, g, L);
    }
  }
  // the dog house: a wooden kennel, a red gabled roof, a dark arched door; its bowl by the corner
  function drawDoghouse(ctx, h) {
    const x = h.x, y = h.y;
    rect(ctx, x - 10, y, 20, 1, SHADOW);
    rect(ctx, x - 9, y - 12, 18, 12, WOOD.k); rect(ctx, x - 8, y - 11, 16, 11, WOOD.top);
    for (const j of [y - 8, y - 5, y - 2]) rect(ctx, x - 8, j, 16, 1, WOOD.d);
    for (let j = 0; j < 9; j++) {           // the roof, a gable, overhanging the walls
      const hw = 2 + j * 1.25, ry = y - 20 + j;
      rect(ctx, Math.round(x - hw - 1), ry, Math.round(hw * 2 + 2), 1, "#5a2420");
      if (j > 0) rect(ctx, Math.round(x - hw), ry, Math.round(hw * 2), 1, j < 3 ? "#c9574b" : j < 7 ? "#a8433a" : "#8a3530");
    }
    rect(ctx, x - 4, y - 8, 8, 8, "#1e1512"); rect(ctx, x - 3, y - 9, 6, 1, "#1e1512"); rect(ctx, x - 5, y - 8, 1, 8, WOOD.k); rect(ctx, x + 4, y - 8, 1, 8, WOOD.k);   // the door
    rect(ctx, x - 12, y - 2, 5, 2, SP.F); rect(ctx, x - 11, y - 2, 3, 1, "#8a5a32");    // the bowl
  }
  // somewhere to hide: behind a piece that stands tall enough, far from the gun closet, not where another hides
  function hideSpot(w, f, L) {
    const bo = L.booths[0], from = bo ? { x: bo.x, y: bo.y } : { x: f.x, y: f.y };
    const taken = [...w.folk.values()].filter((g) => g !== f && g.act.kind === "hide").map((g) => g.act);
    let best = null, score = -1e9;
    for (const p of L.pieces) {
      const k = KIND(p.key), r = p.r;
      if (p.u || ON_WALL.has(k) || ["booth", "fire", "neuralyzer", "add"].includes(k) || r[3] - r[1] < 16 || r[2] - r[0] < 14) continue;   // (a desk is too low to hide behind)
      const x = Math.round((r[0] + r[2]) / 2 + rnd(-6, 6)), y = Math.round(Math.min(p.at[1] - 1, p.at[1] + visTop(k) + 15));   // its eyes peek over the top

      if (y - 20 < L.floor.y0 || taken.some((a) => Math.abs(a.x - x) < 12 && Math.abs(a.y - y) < 8)) continue;
      const sc = Math.hypot(x - from.x, y - from.y) - 0.4 * Math.hypot(x - f.x, y - f.y) + rnd(0, 30);
      if (sc > score) { score = sc; best = { x, y }; }
    }
    return best;
  }
  // the gun closet clicked (hardcore): you walk there and take a shotgun (the `arm` act), then the office through
  // your own eyes (fps.js), from where you stand, looking into the room; in there E on the closet loads the gun
  // again. Esc and you are back, standing where you went
  function firstPerson(key) {
    if (fps || !lastL || !hardcore()) return;
    const p = lastL.pieceOf(key) || lastL.booths[0];
    pointer = null; const me0 = world().folk.get("you"); if (me0) me0.hunt = null;      // a hunt of yours under way in the room is yours to finish in here
    Props3D.init({ tiny, tinyW });
    view = scene3(lastL, world(), app.state(), world().T);
    fps = startFps({
      scene: () => view,
      start: () => {                         // where you stand (by the closet, the gun just taken), looking into the room
        const me = world().folk.get("you"), x = me ? me.x : p ? Math.max(14, Math.min(W - 14, (p.r[0] + p.r[2]) / 2)) : W / 2, y = me ? me.y : p ? Math.min(H - 8, p.r[3] + 22) : H / 2;
        return { x, y, a: Math.atan2((WALL + H) / 2 - y, W / 2 - x) };
      },
      move: (x, y, a, walking) => { const f = world().folk.get("you"); if (f && !f.held) { f.x = x; f.y = y; f.dir = Math.cos(a) < 0 ? -1 : 1; f.walking = walking; } },
      shoot: (id) => { const f = world().folk.get(id); if (!f || f.held || !f.b || f.b.human) return false; shoot(f); return true; },   // an office manager too: a bot like the others
      status: () => ({ kills: world().kills || 0 }),   // the bots you shot (the face grins)
      face: () => { const f = world().folk.get("you"), b = f && f.b; return b ? pixelAvatar(seedOf(b), "human", colorOf(b)) : null; },
      // what E does to what is under the crosshair (the gun closet: more shells), and what it says it would do
      use: (th) => !!th && !!th.piece && KIND(th.piece) === "booth" && "reload",
      label: (th) => {
        const k = th && th.piece && KIND(th.piece);
        if (k === "booth") return "E: reload";
        if (th && (th.bot || th.staff)) return th.name;
        return k ? NAMES[k] || k : null;
      },
      unshoot, alarm, tiny, tinyW,
      snapshot: () => ({ canvas: cv, rect: cv.getBoundingClientRect() }),   // the room as it is on the page: what melts away (Doom's wipe)
      hands: () => {                         // your skin and your sleeves, as your figure wears them (its portrait's shoulders)
        const f = world().folk.get("you"), b = f && f.b, seed = b ? seedOf(b) : "you", col = b ? colorOf(b) : YOU, px = face(seed, col, true).px, i = (19 * 20 + 6) * 4;
        return { skin: pixelAvatar.skin(seed), clothes: px && px[i + 3] > 128 ? "#" + [px[i], px[i + 1], px[i + 2]].map((v) => v.toString(16).padStart(2, "0")).join("") : col };
      },
    }, () => { fps = null; view = null; const f = world().folk.get("you"); if (f) { f.hunt = null; end(f); f.carry = null; } });   // (drawn at the next tick: this can come from inside draw)
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
    const doomed = f.held && dangerNow, bt = f.held ? -1 : boothT(f);    // in the booth: the booth lights up for it, not the bot
    const ring = f.leaving || bt >= 0 ? null : doomed ? RED : lit ? GOLD : app.opens().includes(f.id) ? OPEN : null;
    const fear = !b.human && !b.staff && !f.held ? (f.act.kind === "condemned" ? 1 : world().fear || 0) : 0;   // (one waiting to be shot: all of it)
    const shake = fear > 0.2 && Math.floor(T * 15 + f.z * 7) % (fear > 0.6 ? 2 : 4) === 0 ? (Math.floor(T * 30) % 2 ? 1 : -1) : 0;   // trembling
    if (im) {
      ctx.save();
      if (shake) ctx.translate(shake, 0);
      if (f.dir < 0) { ctx.translate(x0 + SW, 0); ctx.scale(-1, 1); }
      const dx = f.dir < 0 ? 0 : x0;
      if (ring) ctx.drawImage(halo(im, ring), dx, y0);
      ctx.drawImage(doomed ? tinted(im, RED, flashA(T)) : bt > 0 ? dissolve(im, boothRise(bt)) : im, dx, y0);
      ctx.restore();
    }
    if (fear > 0.45) {                        // a drop of sweat running down beside its head
      const ph = (T * 1.4 + f.z) % 1, sx = x + (f.dir < 0 ? -8 : 7);
      if (ph < 0.8) { rect(ctx, sx, y0 + 5 + Math.floor(ph * 6), 1, 2, C.blue3); rect(ctx, sx, y0 + 5 + Math.floor(ph * 6), 1, 1, C.steel4); }
    }
    if (b.human && (f.act.kind === "sleep" || f.act.kind === "doze") && f.act.there) zzz(ctx, f, x + 2, y0 + 4);   // (a robot sleeps without z's)
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
    if (b.human) zzz(ctx, f, x - 12, y - 22);
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
    // the desktop, lower than the cabinet: a leg between each two chairs, a keyboard at each seat; none until the
    // first bot comes
    if (d.seats.length) {
      slab(ctx, x0 + cab - 2, y - 12, dw - cab + 2, 5, 4, p);
      for (const x of [x0 + cab + 1, ...d.seats.map(([, x]) => x + 11), x0 + dw - 4]) {
        rect(ctx, x, y - 4, 2, 4, p.k); rect(ctx, x, y - 4, 1, 3, p.d); rect(ctx, x, y, 2, 1, SHADOW);
      }
      for (const [, x] of d.seats) { rect(ctx, x - 9, y - 11, 7, 2, SP.F); rect(ctx, x - 8, y - 11, 5, 1, SP.B); }
    }
    for (const [id, x] of d.seats) {          // a coffee a colleague brought, by the keyboard, for a while
      const until = w.mugs.get(id), f = w.folk.get(id);
      if (until === undefined) continue;
      if (until < T || !f) { w.mugs.delete(id); continue; }
      rect(ctx, x - 13, y - 14, 3, 3, colorOf(f.b)); rect(ctx, x - 11, y - 13, 1, 1, INK);
    }
    // the cabinet, the project's name on a plate; a click on the plate opens the desk's panel (its name)
    slab(ctx, x0, y - 15, cab, 4, 11, p);
    const lx = Math.round(x0 + (cab - tinyW(d.label)) / 2), onPlate = hot(x0 + 2, y - 10, x0 + cab - 2, y - 2, { kind: "desk", unit: d.unit });
    plate(ctx, x0 + 2, y - 9, cab - 4, false, p);
    tiny(ctx, d.label, lx, y - 8, onPlate ? GOLD : C.steel3);   // under the pointer only its letters light up
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
  // stands, not by the pointer: `spot` is its feet, a little up; `dropAt` is where a bot would go if let go:
  // the booth (in hardcore the gun closet)
  const spot = (f) => ({ x: f.x, y: f.y - 4 });
  function dropAt(L, f) {
    if (!L || !f.b) return null;
    return boothAt(L, f) ? "booth" : null;      // the booth; in hardcore the gun closet: someone takes a gun and shoots it
  }
  const boothAt = (L, f) => L.booths.find((bo) => inRect(spot(f), foot("booth", [bo.x, bo.y]))) || null;
  const ELL = (rx, ry, rows) => [...Array(rows)].map((_, j) => Math.round(rx * Math.sqrt(Math.max(0, 1 - ((j + 0.5 - rows / 2) / (rows / 2)) ** 2))));
  // `part`: "pit" (the stones, the embers, the logs: flat on the floor, for the 3d view's floor), "flames" (what
  // rises: the flames, the sparks, what is thrown in), or both
  function drawFire(ctx, f, T, feeding, burst = -1, burning = [], part = "all") {
    // every measure of the 32 x 28 original scaled by FIRE.S (X), so the pit keeps its proportions
    const { x0, y0 } = f, S = FIRE.S, X = (v) => Math.round(v * S), cx = x0 + X(16), r = (y, x, w, c) => rect(ctx, cx + x, y0 + y, w, 1, c);
    const ST = { k: "#4d4a4f", m: "#8d8a86", hi: "#aaa59e", d: "#6e6b6a" };    // the stones
    const k = Math.floor(T * 6) % 3, big = feeding ? X(4) : burning.length ? X(3) : 0;
    if (part !== "flames") {
    // the ring of stones at the bottom, lit along its far side, in shade on the near one
    const rows = X(12), top = ELL(X(15), X(6), rows), ty = X(16);
    top.forEach((hw, j) => { r(ty + j, -hw, hw * 2, ST.k); if (hw > 2) r(ty + j, -hw + 1, hw * 2 - 2, j < rows / 3 ? ST.hi : j > rows * 2 / 3 ? ST.d : ST.m); });
    for (const [x, y] of [[-12, 20], [-6, 17], [2, 17], [9, 18], [12, 22], [-10, 25], [-1, 26], [6, 26], [11, 25], [-13, 23], [-3, 17], [14, 20], [3, 27], [-8, 27]]) r(X(y), X(x), 1, ST.k);
    ELL(X(12), X(4), X(8)).forEach((hw, j) => r(X(18) + j, -hw, hw * 2, "#120a0c"));
    r(X(28), -X(14), X(28), SHADOW);
    // embers and two logs
    for (const [x, y] of [[-9, 21], [-7, 23], [7, 20], [9, 23], [-3, 24], [4, 24], [-11, 22], [11, 21], [0, 22]]) r(X(y), X(x), 1, (k + x) % 2 ? C.red1 : "#7a2a18");
    r(X(22), -X(7), X(6), "#3b2a1e"); r(X(23), -X(8), X(7), "#5c3a26"); r(X(22), X(1), X(6), "#3b2a1e"); r(X(23), X(2), X(6), "#5c3a26");
    }
    if (part === "pit") return;
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
  function wipe(w) { ink(w).getContext("2d").clearRect(0, 0, ink(w).width, ink(w).height); w.inkV++; w.cells = null; if (w.dream) { w.dream.n = 0; w.dream.done = null; w.dream.mine.clear(); } }   // the dream begins again
  const inked = (w) => { const d = ink(w).getContext("2d").getImageData(0, 0, ink(w).width, ink(w).height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i] > 128) n++; return n / (d.length / 4); };
  function cells(w) {                        // the board's dots from the ink, kept until the ink changes: a dot's middle pixel, else its first inked one
    if (w.cells) return w.cells;
    const im = ink(w), d = im.getContext("2d").getImageData(0, 0, im.width, im.height).data, out = [];
    for (let j = 0; j < INK_AREA.h; j++) for (let i = 0; i < INK_AREA.w; i++) {
      let col = null;
      const m = ((j * INKS + 2) * im.width + i * INKS + 2) * 4;
      if (d[m + 3] > 128) col = "#" + [d[m], d[m + 1], d[m + 2]].map((v) => v.toString(16).padStart(2, "0")).join("");
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
  // A shot bot (your shotgun or an office manager's) leaves remains, not a gravestone: its own picture cut into
  // head, chest, arms and legs, cogs, bolts and a spring, flying apart with a hop and bouncing to a stop on a
  // splash of oil. They are its undo: a click on any of it puts it back together (the pieces fly back in). Idle
  // bots come over and clear them away, a piece at a time (`sweep`); with the last piece gone the session is
  // stopped and deleted (`claude stop`, `claude rm`). With nobody to clear them they rust away after half a
  // minute, and it goes then
  const RUST = { after: 30, fade: 2 };
  const SWEEPERS = new Set(["stand", "loiter", "plant", "window", "stretch", "warm", "watch", "visit", "chair", "coffee", "print"]);   // breaks a bot leaves to clear up
  const COG = [".#.#.", "#####", "##.##", "#####", ".#.#."], COG7 = ["..#.#..", ".#####.", "###.###", "##...##", "###.###", ".#####.", "..#.#.."];
  function scatter(f, at) {
    const charred = false;
    const w = world(), x = Math.round(at[0]), y = Math.max(WALL + 14, Math.round(at[1]));
    const im = body(seedOf(f.b), colorOf(f.b), "down", "stand", false, !!f.b.staff), cut = (sx, sy, sw, sh, kind) => ({ kind, img: im, sx, sy, sw, sh });
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
  function shoot(f) {                         // a flash where it hits, and the pieces; the others more afraid, the ones near run
    const w = world();
    w.shots.push({ x: Math.round(f.x), y: Math.round(f.y) - 14, t0: performance.now(), hit: true });
    scatter(f, [f.x, f.y]); unopen(f.id);
    sfx("shot", f.x);
    for (let k = 0; k < 3; k++) setTimeout(() => sfx("clank", f.x, 0.8), 220 + k * 170 + Math.random() * 120);   // its parts coming down
    w.kills = (w.kills || 0) + 1;
    if (hardcore()) { w.fear = Math.min(1, (w.fear || 0) + FEAR.kill); alarm(f.x, f.y); }
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
        if (t >= 1) { w.remains.delete(id); spare(w, id); w.riseAt.set(id, { x: r.x, y: r.y }); continue; }   // the office manager leaves it alone from now on
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
    w.remains.delete(id); w.stopped.set(id, performance.now());
    app.act(id.startsWith("mgr:") ? { kind: "managerGone", key: id.slice(4) } : { kind: "stop", id, rm: true });   // an office manager: her session, her reception left empty
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
  // a figure going up in the booth's light: a front of light rises from its feet (d 0 to 1); what it has passed
  // turns to light and thins out in an ordered dither until nothing is left
  const BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
  function dissolve(im, d) {
    const c = document.createElement("canvas"); c.width = im.width; c.height = im.height;
    const x = c.getContext("2d");
    x.drawImage(im, 0, 0);
    if (d <= 0) return c;
    const img = x.getImageData(0, 0, c.width, c.height), p = img.data, top = 3, foot = c.height - 1;
    for (let j = 0; j < c.height; j++) {
      const a = Math.max(0, Math.min(1, (d * 1.4 - (foot - j) / (foot - top)) / 0.4));
      if (!a) continue;
      for (let i = 0; i < c.width; i++) {
        const k = (j * c.width + i) * 4;
        if (!p[k + 3]) continue;
        if (BAYER[j % 4][i % 4] / 16 < a) { p[k + 3] = 0; continue; }
        const m = Math.min(1, a * 1.6) * 0.85;
        p[k] += (255 - p[k]) * m; p[k + 1] += (244 - p[k + 1]) * m; p[k + 2] += (214 - p[k + 2]) * m;
      }
    }
    x.putImageData(img, 0, 0);
    return c;
  }
  // the danger's flash: the wash goes between two strengths, five times a second
  function flashA(T) { return Math.floor(T * 5) % 2 ? 0.5 : 0.3; }
  // something goes into the fire: a picture of it, to be tossed in and sink (drawFire)
  function toFire(r, img) { sfx("whoosh", r[0]); world().burning.push({ img, x: r[0] - 2, y: r[1] - 2, w: img.width, h: img.height, t0: performance.now() }); world().flare = performance.now(); }
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
      case "clock": drawClock(ctx, x, y); break;
      case "fire": drawFire(ctx, { x0: x, y0: y - FIRE.h, x1: x + FIRE.w, y1: y }, T, false); break;
      case "closet": drawCloset(ctx, { x0: x, y0: y }, false); break;
      case "settings": drawSwitchboard(ctx, { x0: x, y0: y }, {}, false); break;
      case "manager": { const o = { n: 0, name: "", color: "#9a74d6", screen: "on", T: 0 }; drawReception(ctx, pt, o, false); drawReception(ctx, pt, o, true); break; }
      case "pair": drawPair(ctx, pt, true, T, false); break;
      case "add": drawAdd(ctx, { x0: x, y0: y - 15, x1: x + ADD_W, y1: y }, false); break;
      case "booth": drawBooth(ctx, pt, false, -1, -1); drawBooth(ctx, pt, true, -1, -1); break;
      case "neuralyzer": drawNeuralyzer(ctx, pt, false); break;
      case "radio": drawRadio(ctx, pt, true); break;
      case "doghouse": drawDoghouse(ctx, pt); break;
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
  const pend = (at, mid, kind = "door") => { pending = at ? { at, mid, kind, t: performance.now() } : null; draw(); };
  function carryKind(kind, cx, cy) {
    if (!kind) { carrying = null; draw(); return null; }
    const b = cv.getBoundingClientRect(), x = (cx - b.left) / K, y = (cy - b.top) / K;
    const inside = x >= 0 && x < W && y >= 0 && y < H;
    pointer = inside ? { x, y } : null;      // the room's pointer follows the carried piece (the canvas gets no events: the closet holds the pointer)
    carrying = { kind, at: inside ? clampAt(kind, [Math.round(x), Math.round(y)]) : null };   // where it will land: there
    draw();
    return carrying.at;
  }
  function placeKind(kind, cx, cy) { const at = carryKind(kind, cx, cy); carrying = null; draw(); return at; }
  // burrow's own settings: a small steel box on the wall with one big lever, its red knob up; pulled down
  // while the settings' panel is open (a click on it opens them)
  const SB_H = () => 14, SB_W = () => 10;
  function drawSwitchboard(ctx, b, cfg, open) {
    const { x0, y0 } = b, h = SB_H(), wd = SB_W();
    rect(ctx, x0, y0, wd, h, SP.F);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, h - 2, SP.D);
    rect(ctx, x0 + 1, y0 + 1, wd - 2, 1, SP.C); rect(ctx, x0 + 1, y0 + 1, 1, h - 2, SP.C);
    rect(ctx, x0 + 4, y0 + 3, 2, h - 6, INK);                                                          // the slot
    const ky = open ? y0 + h - 4 : y0 - 1;                                                            // the knob: up, or pulled down
    rect(ctx, x0 + 4, Math.min(ky + 2, y0 + 7), 2, Math.abs(ky + 2 - (y0 + 7)), SP.B);                 // the handle, from the slot's middle
    rect(ctx, x0 + 3, ky, 4, 4, INK); rect(ctx, x0 + 3, ky, 3, 3, SP.r); rect(ctx, x0 + 3, ky, 1, 1, "#ff9a8a");
    rect(ctx, x0 + 2, y0 + h - 2, 1, 1, SP.F); rect(ctx, x0 + wd - 3, y0 + h - 2, 1, 1, SP.F);       // the screws
  }
  // the reception: an office manager's booth, 46 dots wide. Behind: two posts and a panel. In front, the counter (a
  // slab), her name on its front on a plate in her colour; on it a bell at the right end and, for one who is
  // a session, her screen at the left (it runs while a turn runs, its cursor blinks while she asks you, dark while
  // her process is off); for Linda a stack of notes as high as there are sessions with notes (six at most) and a pen.
  // `front` draws the counter, which goes over her
  function drawReception(ctx, m, o, front) {
    const x = m.x, y = m.y;
    if (!front) {
      for (const px of [x + 2, x + 42]) { rect(ctx, px, y - 30, 2, 16, WOOD.k); rect(ctx, px, y - 30, 1, 15, WOOD.d); }                       // the posts
      rect(ctx, x + 4, y - 30, 38, 15, "#2b2433"); rect(ctx, x + 4, y - 30, 38, 1, "#1f1a26");                                                // the panel
      for (const px of [x + 13, x + 23, x + 33]) rect(ctx, px, y - 29, 1, 14, "#251f2d");
      return;
    }
    slab(ctx, x, y - 16, 46, 4, 12, WOOD);                                                                                                  // the counter
    const nm = (o.name || "").toUpperCase();
    if (nm) {                                                                     // her name on its front: a desk's plate, dark, the letters in her colour
      const tw = tinyW(nm), px = x + 23 - Math.ceil((tw + 6) / 2);
      plate(ctx, px, y - 10, tw + 6, false, { k: WOOD.k, plate: shade(o.color, 0.26) });
      tiny(ctx, nm, px + 3, y - 9, mix(o.color, "#ffffff", 0.35));
    } else for (const px of [x + 9, x + 18, x + 27, x + 36]) rect(ctx, px, y - 10, 1, 7, WOOD.k);
    rect(ctx, x + 39, y - 18, 5, 1, SP.F); rect(ctx, x + 40, y - 20, 3, 2, SP.y); rect(ctx, x + 40, y - 20, 1, 1, SP.Y); rect(ctx, x + 41, y - 21, 1, 1, SP.F);   // the bell
    if (o.screen) {
      const gx = x + 3, gy = y - 27;
      rect(ctx, gx - 1, gy - 1, 14, 11, SP.F); rect(ctx, gx + 4, gy + 10, 4, 1, SP.F); rect(ctx, gx + 2, gy + 11, 8, 1, SP.F);              // the screen, its stand
      rect(ctx, gx, gy, 12, 9, o.screen === "off" ? "#10141f" : C.blue0);
      if (o.screen === "off") { rect(ctx, gx + 9, gy + 1, 1, 1, "#232c40"); rect(ctx, gx + 8, gy + 2, 1, 1, "#232c40"); }
      else {
        const t = o.screen === "busy" ? Math.floor(o.T * 5) : 0;
        ctx.fillStyle = o.screen === "busy" ? o.color : shade(o.color, 0.45);
        for (let r = 0; r < 3; r++) ctx.fillRect(gx + 1 + ((r * 3) % 4), gy + 1 + r * 2, Math.min(10, 3 + ((r * 7 + t) * 5) % 8), 1);
        if (o.screen === "asks" ? Math.floor(o.T * 2) % 2 : o.screen === "on") rect(ctx, gx + 1, gy + 7, 2, 1, C.steel4);
      }
    } else {
      const h = Math.min(6, o.n);
      if (h) { rect(ctx, x + 5, y - 16 - h, 8, h + 1, INK); for (let k = 0; k < h; k++) rect(ctx, x + 6, y - 16 - h + k, 6, 1, k % 2 ? SP.B : SP.A); }
      rect(ctx, x + 15, y - 15, 5, 1, C.blue2); rect(ctx, x + 19, y - 15, 1, 1, INK);                                                     // the pen
    }
  }
  function drawDoor(ctx, x, p, ajar, boarded, y0 = WALL - 20) {
    put(ctx, sprites.swap(ajar ? "doorOpen" : "door", { h: p.hi, w: p.top, d: p.d, k: p.k }), x, y0);
    if (boarded) for (const y of [y0 + 4, y0 + 11]) {       // two planks nailed across it
      rect(ctx, x - 1, y, 18, 3, WOOD.k); rect(ctx, x, y + 1, 16, 1, WOOD.top); rect(ctx, x, y, 16, 1, WOOD.hi);
      rect(ctx, x + 1, y + 1, 1, 1, C.steel1); rect(ctx, x + 14, y + 1, 1, 1, C.steel1);
    }
  }
  // The clock, always on the wall (it can't be burnt), telling this computer's time: a round face 17 dots across in
  // Pixel Agents' colours (its rim red above, slate below), the hours as dots, the quarters darker; the hour hand
  // short and black, the minute hand long, the seconds' red one ticking. While it keeps the computer awake its face
  // is yellowed: for a while, a wedge from the hour hand to the hour it ends (shrinking as the time runs out, as a
  // kitchen timer); while local bots work, all of it; set to wait for them and none at work, hatched
  const CLOCK = (() => {                    // its cells, by their distance from the middle (8, 8)
    const face = [], rim = [];
    for (let j = 0; j < 17; j++) for (let i = 0; i < 17; i++) {
      const d = Math.hypot(i - 8, j - 8);
      if (d <= 6.6) face.push([i, j]); else if (d <= 8.4) rim.push([i, j, d]);
    }
    return { face, rim };
  })();
  function drawClock(ctx, x0, y0, a) {
    const cx = x0 + 8, cy = y0 + 8, t = new Date();
    for (const [i, j, d] of CLOCK.rim) rect(ctx, x0 + i, y0 + j, 1, 1, j < 8 ? (d < 7.4 ? SP.r : SP.R) : d < 7.4 ? SP.D : SP.G);
    for (const [i, j] of CLOCK.face) rect(ctx, x0 + i, y0 + j, 1, 1, "#ededed");
    const hour = ((t.getHours() % 12) + t.getMinutes() / 60) / 12 * 2 * Math.PI;
    if (a && a.mode !== "off") {
      const left = a.mode === "for" ? a.until - Date.now() : 0;
      const span = left > 0 ? Math.max(Math.PI / 12, left / 432e5 * 2 * Math.PI) : 0;   // 12 h round; never under half an hour, or the hands hide it
      ctx.fillStyle = SP.Y;
      for (const [i, j] of CLOCK.face) {
        // a cell the wedge reaches at all (its half width in angle past the end), so the last minutes still show
        const dx = i - 8, dy = 8 - j;
        const on = a.mode === "for" ? span > 0 && (Math.atan2(dx, dy) - hour + 4 * Math.PI) % (2 * Math.PI) < span + Math.atan2(0.5, Math.hypot(dx, dy))
          : a.on || (i + j) % 3 === 0;
        if (on) ctx.fillRect(x0 + i, y0 + j, 1, 1);
      }
    }
    for (let k = 0; k < 12; k++) {          // the hours
      const ang = k / 12 * 2 * Math.PI;
      rect(ctx, Math.round(cx + Math.sin(ang) * 5.6), Math.round(cy - Math.cos(ang) * 5.6), 1, 1, k % 3 ? SP.C : SP.E);
    }
    const hand = (ang, len, col) => {       // a line from the middle, half a dot a step
      ctx.fillStyle = col;
      for (let r = 1; r <= len; r += 0.5) ctx.fillRect(Math.round(cx + Math.sin(ang) * r), Math.round(cy - Math.cos(ang) * r), 1, 1);
    };
    hand(t.getMinutes() / 60 * 2 * Math.PI + t.getSeconds() / 3600 * 2 * Math.PI, 5.5, SP.F);
    hand(hour, 3.5, INK);
    if (!still) hand(t.getSeconds() / 60 * 2 * Math.PI, 5, SP.r);
    rect(ctx, cx, cy, 1, 1, SP.r);
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
  // the booth (Futurama's): a tall glass cabin on a plinth, a lit sign on its roof. A bot put in it (t: seconds in,
  // -1 while empty) stands behind the glass: the door slides shut, the sign goes red, a light rises from its floor
  // and takes the bot with it (dissolve), the cabin glows brighter and brighter, the sign flickers in the last
  // second; then the bot is gone (`claude stop`, `claude rm`) and the light fades (flash). Two layers: the cabin,
  // and its glass door in front of whoever is inside
  function drawBooth(ctx, bo, front, t, flash, armed) {
    const x0 = bo.x - 9, y = bo.y, top = y - 32, gy = top + 5, gh = y - 3 - gy, busy = t >= 0 || armed;
    const glow = t >= 0 ? boothGlow(t) : 0, rise = t >= 0 ? boothRise(t) : 0, lh = Math.round(gh * Math.min(1, rise * 1.1)), T = world().T;
    if (!front) {
      rect(ctx, x0 + 1, y, 16, 1, SHADOW);
      rect(ctx, x0, top, 18, 5, SP.F); rect(ctx, x0 + 1, top + 1, 16, 3, SP.E); rect(ctx, x0 + 1, top + 1, 16, 1, SP.D);   // the roof
      const flick = t > BOOTH - 1 && !still && Math.floor(T * 8) % 2;
      rect(ctx, x0 + 4, top + 1, 10, 3, INK); rect(ctx, x0 + 5, top + 2, 8, 1, busy && !flick ? SP.r : SP.Y);           // its sign
      rect(ctx, x0, gy, 18, gh, SP.F); rect(ctx, x0 + 1, gy, 1, gh, SP.D); rect(ctx, x0 + 16, gy, 1, gh, SP.D);              // the posts
      rect(ctx, x0 + 2, gy, 14, gh, busy ? "#3b3428" : "#222a37");                                                       // inside, warm while in use
      rect(ctx, x0 + 2, y - 5, 14, 2, busy ? "#4a412f" : "#2b3442");                                                     // its floor
      if (glow) {                                                                                                        // the light, behind the bot
        ctx.globalAlpha = 0.15 + 0.3 * glow; rect(ctx, x0 + 2, gy, 14, gh - lh, "#fff4d6");
        ctx.globalAlpha = 0.45 + 0.5 * glow; rect(ctx, x0 + 2, gy + gh - lh, 14, lh, "#fff4d6"); ctx.globalAlpha = 1;
      }
      rect(ctx, x0, y - 3, 18, 3, SP.F); rect(ctx, x0 + 1, y - 3, 16, 1, SP.C); rect(ctx, x0 + 1, y - 2, 16, 1, SP.D);       // the plinth
      return;
    }
    if (glow) {                                                                                                          // the light, before it
      ctx.globalAlpha = 0.35 * glow; rect(ctx, x0 + 2, gy, 14, gh, "#fff4d6"); ctx.globalAlpha = 1;
      if (rise > 0 && rise < 1) rect(ctx, x0 + 2, gy + gh - lh, 14, 1, "#fffaf0");                                      // its rising edge
      if (rise > 0 && !still) for (let k = 0; k < 7; k++) {                                                             // motes going up in it
        const ph = (T * 0.8 + k / 7) % 1, my = y - 5 - Math.round(ph * (gh - 3));
        if (my < gy + gh - lh - 1 && rise < 1) continue;                                                                 // only where the light has been
        ctx.globalAlpha = 1 - ph; rect(ctx, x0 + 3 + ((k * 5 + 2) % 12), my, 1, 1, k % 2 ? "#ffffff" : SP.Y); ctx.globalAlpha = 1;
      }
    }
    const off = t >= 0 ? Math.round(13 * (1 - boothShut(t))) : 0, dx = x0 + 2 + off, dw = 14 - off;                     // the door, sliding in from the right
    ctx.globalAlpha = 0.25; rect(ctx, dx, gy, dw, gh, "#bfe3ff"); ctx.globalAlpha = 1;                                    // its glass
    for (let k = 0; k < 4; k++) { if (4 + k >= off + 2) rect(ctx, x0 + 4 + k, gy + 4 + k, 1, 1, "rgba(255,255,255,0.5)"); if (10 + k >= off + 2) rect(ctx, x0 + 10 + k, gy + 11 + k, 1, 1, "rgba(255,255,255,0.3)"); }
    rect(ctx, x0 + 2, gy, 14, 1, SP.D); rect(ctx, dx, gy + 1, 1, gh - 1, SP.D); rect(ctx, dx + 1, gy + 10, 1, 4, SP.B);  // the rail; the door's edge and handle
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
      for (const [k, gx] of [[0, x0 + 5], [1, x0 + 12]]) if (k < 2 - Math.min(2, +out || 0)) {
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
  // the radio: a midcentury set on splayed legs, a woven cloth over its speaker, a dial lit (and notes rising from it,
  // drawNotes) while the room's sound is on
  function drawRadio(ctx, p, on) {
    const x = p.x, y = p.y;
    rect(ctx, x - 6, y, 12, 1, SHADOW);
    for (const [lx, d] of [[-5, -1], [4, 1]]) { rect(ctx, x + lx, y - 4, 1, 2, WOOD.k); rect(ctx, x + lx + d, y - 2, 1, 2, WOOD.k); }   // the legs
    rect(ctx, x - 7, y - 19, 14, 16, WOOD.k); rect(ctx, x - 6, y - 20, 12, 1, WOOD.k);   // the cabinet, its top corners rounded
    rect(ctx, x - 6, y - 19, 12, 15, WOOD.top); rect(ctx, x - 5, y - 19, 10, 1, WOOD.hi); rect(ctx, x - 6, y - 5, 12, 1, WOOD.d);
    rect(ctx, x - 5, y - 17, 10, 3, on ? SP.Y : "#3a2c26");                              // the dial across its top: its marks, the needle
    for (const mx of [x - 4, x - 2, x, x + 3]) rect(ctx, mx, y - 17, 1, 1, on ? "#c98a2c" : "#2a2026");
    rect(ctx, x + 1, y - 17, 1, 3, on ? SP.r : "#2a2026");
    for (let j = 0; j < 6; j++) for (let i = 0; i < 10; i++) rect(ctx, x - 5 + i, y - 13 + j, 1, 1, (i + j) % 2 ? "#d8c49a" : "#bfa97e");   // the speaker's cloth
    for (const sx of [x - 3, x, x + 3]) rect(ctx, sx, y - 13, 1, 6, WOOD.d);           // behind three slats
    for (const kx of [x - 5, x + 3]) { rect(ctx, kx, y - 7, 2, 2, SP.F); rect(ctx, kx, y - 7, 1, 1, SP.C); }   // the knobs
  }
  function drawNotes(ctx, p, T) {              // two notes drifting up from it, fading
    for (const k of [0, 1]) {
      const ph = (T * 0.45 + k * 0.5) % 1, a = ph < 0.15 ? ph / 0.15 : 1 - (ph - 0.15) / 0.85;
      const nx = Math.round(p.x - 3 + k * 5 + Math.sin(ph * 5 + k * 2) * 2), ny = Math.round(p.y - 25 - ph * 14);
      ctx.globalAlpha = Math.max(0, a);
      rect(ctx, nx, ny + 3, 2, 2, INK); rect(ctx, nx + 1, ny, 1, 4, INK); rect(ctx, nx + 2, ny, 1, 1, INK);
      ctx.globalAlpha = 1;
    }
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
  // the pouf (the kind is still "armchair"): a square green cushion seen from above at a slant, as the furniture
  // is, so it is the same from each of its four sides; its corners rounded, its top lit along the front edge, a
  // button in the middle, on two little splayed wooden feet. Pixel Agents' armchair, its seat without the back
  const POUF = { k: "#173115", h: "#8cbf8c", t: "#6fa374", s: "#487841", d: "#355f31", w: "#885c47", x: "#532e3d" };
  const POUF_ROWS = [
    "..kkkkkkkkkkkk..",
    ".kttttttttttttk.",
    "kttttttttttttttk",
    "kttttttttttttttk",
    "kttttttssttttttk",
    "kttttttttttttttk",
    "kttttttttttttttk",
    "khhhhhhhhhhhhhhk",
    "kssssssssssssssk",
    "kssssssssssssssk",
    ".kddddddddddddk.",
    "..kkkkkkkkkkkk..",
    "..wx........wx..",
    ".wx..........wx.",
  ];
  let poufPic = null;
  function pouf() {
    if (poufPic) return poufPic;
    const c = document.createElement("canvas"); c.width = 16; c.height = POUF_ROWS.length;
    const x = c.getContext("2d");
    POUF_ROWS.forEach((r, j) => [...r].forEach((ch, i) => { if (POUF[ch]) { x.fillStyle = POUF[ch]; x.fillRect(i, j, 1, 1); } }));
    return (poufPic = c);
  }
  function drawArmchair(ctx, c) {
    rect(ctx, c.x - 7, c.y, 14, 1, SHADOW);
    ctx.drawImage(pouf(), c.x - 8, c.y - 14);
  }
  const poufSeat = (ch) => ({ x: ch.x, y: ch.y - 4, on: ch.key });      // on its top, drawn over it (piece's dz)
  const poufFace = (ch) => (ch.x < W / 2 ? 1 : -1);          // facing the middle of the room
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
      case "fire": return { kind: "warm", side: f.x > (lastL.fire.x0 + lastL.fire.x1) / 2 ? 1 : 0, left: long / 2 };   // its hands at the fire (it burns furniture, never a bot)
      case "window": if (f.human) { app.act({ kind: "outside", key: p.key }); return null; } return { kind: "window", at: p.key, left: long / 3 };
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
    const dog = !!dogRect && over(...dogRect) && !hovered;      // on the dog (a figure drawn in front of it would be hovered)
    press = { x: pointer.x, y: pointer.y, action: h && h.action, bot: !dog && hovered && !hovered.seat && !hovered.b.staff && hovered.b.id, piece: dog ? null : pieceAt(pointer), dog, shift: e.shiftKey };
    cv.setPointerCapture(e.pointerId);
  });
  cv.addEventListener("pointermove", (e) => {
    pointer = dots(e);
    if (press && !moving && !held && !dogHeld && Math.hypot(pointer.x - press.x, pointer.y - press.y) > 1) {
      const f = press.bot && world().folk.get(press.bot), g = press.dog && world().dog;
      if (g) {                                         // the dog, picked up (by the scruff): what it carried drops where it was
        const st = world().stick;
        if (st && st.held) { st.held = false; st.x = g.x; st.y = g.y; st.t = 1; st.h = 0; }
        dogHeld = true; g.held = true; g.act = { kind: "held" };
      }
      else if (f && f.b && !f.lying) {                      // picked up; one about to be stopped is rescued by that
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
      // on the grid of dots while it moves, as pair: it steps, and lands where it is shown, over whatever is there
      // (keeping the room tidy is yours). What hangs on the wall stays on it, sliding along; only one that can
      // burn comes off it, and only over the fire
      const loose = onWall(moving.key) && !fixed(moving.key) && [Math.round(Math.max(2 - (r[0] - at[0]), Math.min(W - 2 - (r[2] - at[0]), at[0]))), Math.round(Math.max(0, Math.min(H - 1 - (r[3] - at[1]), at[1])))];
      moving.at = loose && lastL && inFire(lastL, moving.key, loose, moving.u) ? loose : clampAt(moving.key, at, moving.u);
    }
    if (dogHeld && world().dog) { const g = world().dog; g.x = Math.max(8, Math.min(W - 8, pointer.x)); g.y = Math.max(WALL, Math.min(H - 2, pointer.y + 7)); }   // it hangs from the pointer
    if (held) {
      held.x = Math.max(0, Math.min(W, pointer.x)); held.y = Math.max(WALL, Math.min(H - 2, pointer.y + 24)); held.dir = -1;   // it hangs under the pointer, but never past the floor's bottom edge
      app.carry(held.id, pointer.x >= W || pointer.x < 0 || pointer.y < 0 || pointer.y >= H ? e.clientX : null, e.clientY);
    }
    draw();
  });
  cv.addEventListener("pointerup", (e) => {
    {                                        // on your way to the gun closet: any other click stops you (where it sends you, you go)
      const you = world().folk.get("you"), a = press && press.action;
      if (you && you.act.kind === "arm" && press && !moving && !held && pointer && Math.hypot(pointer.x - press.x, pointer.y - press.y) <= 1 && !(a && a.kind === "aim")) end(you);
    }
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
        const at = clampAt(moving.key, moving.at, moving.u);       // put down on the grid of dots, where it was let go
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
      else if (bot && pointer && dropAt(lastL, f) === "booth") {
        const bo = boothAt(lastL, f);
        if (hardcore()) { f.x = Math.max(lastL.floor.x0, bo.x - 16); f.y = Math.min(lastL.floor.y1, bo.y + 3); condemn([f.id]); }   // by the gun closet, hands up: someone takes a gun
        else { end(f); f.act = { kind: "doom", at: bo.key, left: BOOTH }; unopen(f.id); }   // into the booth: five seconds, then gone
      }
      else {
        const p = pointer && pieceAt(spot(f));             // what it stands on, not what the pointer is on
        if (p && KIND(p.key) === "pair") { if (bot) app.act({ kind: "pair", key: p.key, id: f.id }); else app.act({ kind: "pairOpen", key: p.key }); f.act = back(); }
        else f.act = (pointer && !f.work && useAt(f, p)) || back();
      }
      f.legs = "stand";
    } else if (dogHeld) {                    // the dog let go of: on the floor (never in the fire: in front of it), a shake, off it goes; at its house, in
      dogHeld = false;
      const g = world().dog, L = lastL;
      if (g && L) {
        g.held = false;
        g.x = Math.max(L.floor.x0, Math.min(L.floor.x1, g.x)); g.y = Math.max(L.floor.y0, Math.min(L.floor.y1, g.y));
        const F = L.fire, dh = L.doghouses[0];
        if (F && g.x >= F.x0 - 4 && g.x < F.x1 + 4 && g.y >= F.y0 && g.y < F.y1 + 6) g.y = Math.min(L.floor.y1, F.y1 + 8);
        const home = dh && inRect(g, foot("doghouse", [dh.x, dh.y + 4]));
        g.act = { kind: "shake", left: 0.6, next: home ? { kind: "home", left: rnd(20, 60) } : null };
      }
    } else if (press && press.action && press.action.kind === "undo") { const f = world().folk.get(press.action.id); if (f && f.act.kind === "doom") f.act.back = true; }   // the booth runs back, it steps out
    else if (press && press.action && press.action.kind === "unshoot") unshoot(press.action.id);
    else if (press && press.action && press.action.kind === "aim") {      // you walk to the gun closet and take a gun; the first person after
      const you = world().folk.get("you"), sh = app.shell();
      if (you && !you.held && !fps) { you.hunt = null; end(you); you.act = { kind: "arm", at: press.action.key, open: app.open(), shell: sh && sh.m + ":" + sh.dir, chosen: true }; }
    }
    else if (press && press.action && press.action.kind === "water") {     // you go and water the plant
      const you = world().folk.get("you");
      if (you && !you.held) { end(you); you.act = { kind: "plant", at: press.action.key, wait: 0, chosen: true }; }
    }
    else if (press && press.action && press.action.kind === "pet") {       // the dog: you go and pet it (out of its house it comes, unless afraid); petting it, you throw the stick
      const w = world(), you = w.folk.get("you"), g = w.dog;
      if (you && !you.held && g && !g.held && !g.leaving && lastL) {
        if (canThrow(w)) throwStick(w, you, g, lastL);
        else if (g.act.kind !== "fetch") { end(you); you.act = { kind: "pet", left: 6, chosen: true }; if (!g.act.hide) g.act = { kind: "petted", out: g.inside }; }
      }
    }
    else if (press && press.action && press.action.kind === "sit") {       // you take the armchair; a bot sitting in it gets up
      const w = world(), you = w.folk.get("you");
      for (const g of w.folk.values()) if (g !== you && g.act.kind === "chair" && g.act.at === press.action.key) end(g);
      if (you && !you.held) { end(you); you.act = { kind: "chair", at: press.action.key, left: rnd(180, 360), chosen: true }; }
    }
    else if (press && press.action) { if (press.action.kind === "report") { world().print = performance.now(); if (lastL && lastL.printer) sfx("print", lastL.printer.x); } app.act({ ...press.action, shift: press.shift }); }
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
  window.__room = { world, size: () => ({ W, H, K }), layout: () => lastL, hp: () => lastHp, fps: (key) => firstPerson(key),
    shoot: (id) => { const f = world().folk.get(id); if (f) shoot(f); }, unshoot, dogPic, dog: () => world().dog, stick: () => world().stick, dream: (scene, k) => { const w = world(); wipe(w); newDream(w, scene, k); return w.dream.order.length; }, fear: (v) => { if (v !== undefined) world().fear = v; return world().fear; } };   // for poking at it from the console
  const stats = () => { const w = world(); return { since: w.born, bots: Object.fromEntries(w.stats), meetings: w.meetings, inked: w.ink ? inked(w) : 0 }; };
  // at(id): where a figure stands, in CSS pixels of the page (for look.mjs, to drive the room from outside)
  const at = (id) => { const f = world().folk.get(id), b = cv.getBoundingClientRect(); return f ? [b.left + f.x * K, b.top + (f.y - 12) * K] : null; };
  // the whiteboard's ink, for the view: the canvas itself, a line on it (in ink pixels), a wipe
  const board = { ink: () => ink(world()), line: (x0, y0, x1, y1, col) => inkLine(world(), x0, y0, x1, y1, col), wipe: () => wipe(world()), version: () => world().inkV || 0 };
  // the two at a game table right now (chess, pong), white or the left side first: the page lets you watch them
  const playingAt = (kind) => [...world().folk.values()].filter((f) => f.act.kind === kind && f.act.there && f.act.with && !f.act.you && !f.human).sort((a, b) => a.act.side - b.act.side).map((f) => f.id);
  // for look.mjs: two bots sent to a game, a kickabout for some
  window.__room.game = (kind, a, b) => { const w = world(), fa = w.folk.get(a), fb = w.folk.get(b); if (!fa || !fb) return false; end(fa); end(fb); fa.act = { kind, side: 0, with: b, left: 90, there: false }; fb.act = { kind, side: 1, with: a, left: 90, there: false }; return true; };
  window.__room.football = (ids) => { const w = world(), team = ids.map((id) => w.folk.get(id)).filter(Boolean); return team.length >= 2 && !!football(w, lastL, team); };
  window.__room.clock = (a) => { const c = document.createElement("canvas"); c.width = 17; c.height = 17; drawClock(c.getContext("2d"), 0, 0, a); return c.toDataURL(); };   // the clock's face for an awake state
  // a figure as the room draws it (the office managers' cards): a data URL, S pixels a dot; null until its face has loaded
  const figure = (seed, color, S = 4, femme = true) => {
    const im = body(seed, color, "down", "stand", false, femme);
    if (!im) return null;
    const c = document.createElement("canvas"); c.width = im.width * S; c.height = im.height * S;
    const x = c.getContext("2d"); x.imageSmoothingEnabled = false; x.drawImage(im, 0, 0, c.width, c.height);
    return c.toDataURL();
  };
  return { talk, typed, size, draw, figure, wears, fresh: (id) => fresh.add(id), width, color: projectColor, icon, door: doorPicture, paints: () => ({ wood: WOOD.top, list: PAINT.map((p) => p.top) }), paintOf: paintIdx, paint: paintOf, tiny, tinyW, printer: printerPicture, carry: carryKind, place: placeKind, pend, condemn: (ids) => condemn(ids), playing: playingAt, names: NAMES, at, board, stats, hovered: () => hovered && { id: hovered.b.id, seat: !!hovered.seat } };
}
