// node paint-cases.mjs: writes paint-cases.json beside it, the cases for tests/painters.rs:
//   node native/tools/golden.mjs http://localhost:7791/ native/crates/room/tests/golden/paint-cases.json native/crates/room/tests/golden
// (a BURROW_DEMO=1 server). Each draws one painter of room.js with fixed parameters into a small canvas, a
// pixel a dot, over the floor or the wall where the piece stands there (so nothing is half transparent);
// the time, performance.now() and the room's T are pinned while it draws. tests/painters.rs draws the
// same, by the case's name.
import { writeFileSync } from "node:fs";

const P = `
// Pixel Agents' pieces are PNGs, loaded by the page: wait for them
for (let i = 0; i < 100 && !["PLANT", "PLANT_2", "WHITEBOARD", "CUSHIONED_CHAIR_SIDE", "WOODEN_BENCH", "SMALL_TABLE_FRONT", "COFFEE"].every((n) => sprites.get(n)); i++) await new Promise((r) => setTimeout(r, 50));
const cv = (w, h, bg) => { const c = document.createElement("canvas"); c.width = w; c.height = h; const x = c.getContext("2d"); x.imageSmoothingEnabled = false; if (bg) { x.fillStyle = bg; x.fillRect(0, 0, w, h); } return [c, x]; };
const RD = Date;
const at = (h, m, s, fn) => { const fixed = new RD(2026, 9, 29, h, m, s).getTime(); window.Date = class extends RD { constructor(...a) { super(...(a.length ? a : [fixed])); } static now() { return fixed; } }; try { return fn(fixed); } finally { window.Date = RD; } };
const frozen = (fn) => { const now = performance.now(); performance.now = () => now; try { return fn(now); } finally { delete performance.now; } };
const atT = (t, fn) => { const w = I.world(), o = w.T; w.T = t; try { return fn(); } finally { w.T = o; } };
const hover = (p, fn) => { const o = I.pointer; I.pointer = p; try { return fn(); } finally { I.pointer = o; } };
const patch = (k, v, fn) => { const o = I.app[k]; I.app[k] = v; try { return fn(); } finally { I.app[k] = o; } };
const PLANK = "#734728", WALLC = "#28374b";
// a figure's portrait loads by itself (an <img>): ask for it, wait, then draw
const prime = async (seed, color, human) => { I.face(seed, color, human); await new Promise((r) => setTimeout(r, 150)); };
const desk = (o) => {
  const cab = Math.max(I.tinyW(o.label.toUpperCase()) + 6, 22, o.trees.length ? 26 + o.trees.length * 13 : 0), n = o.seats.length;
  const folk = new Map(), mugs = new Map();
  const seats = o.seats.map((s, k) => {
    const id = "b" + k;
    folk.set(id, { id, legs: "sit", lying: false, act: { kind: "desk" }, b: { id, unit: s.unit || "u1", alive: s.alive !== false }, busy: !!s.busy, waiting: !!s.waiting });
    if (s.mug) mugs.set(id, 1e9);
    return [id, o.x0 + cab + 4 + 9 + k * 18, s.tree || ""];
  });
  const d = { x0: o.x0, y: o.y, w: n ? cab + 4 + n * 18 : cab, cab, label: o.label.toUpperCase(), dir: o.dir, unit: o.dir, m: "", color: I.projectColor(o.dir), trees: o.trees, seats };
  return [d, { folk, mugs }];
};
`;
const cases = [];
const add = (name, body) => cases.push({ name: "paint-" + name, js: `(async (I) => { ${P}\n${body} })(window.__in)` });

// every kind at rest, as the closet shows it (the door's paint depends on the machines: tested by itself)
const KINDS = ["meet", "pong", "chess", "coffee", "cooler", "printer", "armchair", "plant", "board", "window", "add", "pair", "manager",
  "booth", "neuralyzer", "doghouse", "radio", "fire", "closet", "settings", "clock"];
for (const k of KINDS) add(`icon-${k}`, `return at(14, 25, 37, () => I.icon("${k}", 1, 4));`);
for (const k of ["fire", "booth", "manager", "clock"]) add(`icon2-${k}`, `return at(14, 25, 37, () => I.icon("${k}", 2, 4));`);

// desks
const DESKS = {
  empty: `{ x0: 4, y: 46, label: "burrow", dir: "/Users/x/burrow", trees: [], seats: [] }`,
  idle: `{ x0: 4, y: 46, label: "pair", dir: "/Users/x/pair", trees: [], seats: [{}, {}] }`,
  busy: `{ x0: 4, y: 46, label: "burrow", dir: "/Users/x/burrow", trees: [], seats: [{ busy: true }, { mug: true, unit: "u7" }, { waiting: true }] }`,
  waits: `{ x0: 4, y: 46, label: "knee-id", dir: "/home/r/knee-id", trees: [], seats: [{ waiting: true }] }`,
  off: `{ x0: 4, y: 46, label: "odyssey", dir: "~", trees: [], seats: [{ alive: false }, { alive: false, busy: true }] }`,
  trees: `{ x0: 4, y: 46, label: "burrow", dir: "/u/burrow", trees: ["/u/burrow-wt1", "/u/b/.claude/worktrees/x"], seats: [{ busy: true }, { tree: "/u/burrow-wt1", waiting: true }, { tree: "/u/b/.claude/worktrees/x" }] }`,
};
const deskCase = (name, spec, T, extra = {}) => add(`desk-${name}`, `
  const [c, x] = cv(130, 50, PLANK), [d, w] = desk(${spec});
  const sc = I.screenAt(d), px = sc.sx + 22 + d.trees.length * 13 + 2;
  const P = ${JSON.stringify(extra.pointer || null)};
  const pt = P === "plate" ? { x: d.x0 + 5, y: d.y - 6 } : P === "screen" ? { x: sc.sx + 5, y: sc.sy + 5 } : P === "plus" ? { x: px + 2, y: sc.sy + 2 } : P === "laptop" ? { x: sc.sx + 26, y: d.y - 19 } : null;
  const shells = ${extra.shells || "[]"}, hs = ${extra.hlShell || "null"};
  patch("shells", () => shells, () => patch("hlShell", () => hs, () => hover(pt, () => I.drawDesk(x, d, w, ${T}, new Set(${JSON.stringify(extra.hl || [])}), ${!!extra.lit}))));
  return c;`);
deskCase("empty", DESKS.empty, 0);
deskCase("idle", DESKS.idle, 1.3);
deskCase("busy", DESKS.busy, 2.7);
deskCase("waits", DESKS.waits, 0.6);
deskCase("off", DESKS.off, 0.6);
deskCase("trees", DESKS.trees, 0.6, { lit: true, hl: ["b2"] });
deskCase("plate", DESKS.idle, 1.3, { pointer: "plate" });
deskCase("plus", DESKS.idle, 1.3, { pointer: "plus" });
deskCase("screen", DESKS.trees, 0.6, { pointer: "screen" });
deskCase("laptop", DESKS.trees, 0.6, { pointer: "laptop" });
deskCase("shell", DESKS.trees, 0.6, { shells: `[{ m: "", dir: "/u/burrow", busy: true }, { m: "", dir: "/u/burrow-wt1", busy: false }]` });
deskCase("shell-idle", DESKS.empty, 0.6, { shells: `[{ m: "", dir: "/Users/x/burrow", busy: false }]`, hlShell: `{ m: "", dir: "/Users/x/burrow" }` });
for (const lit of [0, 1]) add(`chair-${lit}`, `const [c, x] = cv(16, 22, PLANK); I.drawChair(x, 6, 18, ${!!lit}); return c;`);

// the fire
const fire = (name, T, feeding, burst, part, burning) => add(`fire-${name}`, `
  const [c, x] = cv(80, 80, PLANK);
  const plant = I.snapshot(I.foot("plant", [0, 0]), (o) => I.drawKind(o, "plant", [0, 0], null, 0));
  const board = I.snapshot(I.foot("board", [0, 0]), (o) => I.drawKind(o, "board", [0, 0], null, 0));
  frozen((now) => I.drawFire(x, { x0: 16, y0: 30 }, ${T}, ${feeding}, ${burst}, ${burning || "[]"}, "${part || "all"}"));
  return c;`);
fire("0", 0.4, false, -1);
fire("feed", 1.1, true, -1);
fire("burst3", 0.7, false, 0.3);
fire("burst8", 0.7, false, 0.8);
fire("pit", 0.4, false, -1, "pit");
fire("flames", 0.4, true, 0.5, "flames");
for (const t of [0.1, 0.3, 0.6, 1.4])
  fire(`burn-${t}`, 0.9, false, -1, "all", `[{ img: plant, x: 2, y: 2, w: plant.width, h: plant.height, t0: now - ${t * 1000} }, { img: board, x: 44, y: 4, w: board.width, h: board.height, t0: now - ${t * 1000 + 40} }]`);

// the booth, the gun closet
const booth = (name, t, flash, armed) => add(`booth-${name}`, `
  const [c, x] = cv(30, 44, PLANK);
  atT(3.4, () => { I.drawBooth(x, { x: 15, y: 40 }, false, ${t}, ${flash}, ${armed}); I.drawBooth(x, { x: 15, y: 40 }, true, ${t}, ${flash}, ${armed}); });
  return c;`);
for (const t of [-1, 0.2, 1, 2.5, 4.6]) booth(`t${t}`, t, -1, false);
booth("armed", -1, -1, true);
booth("flash", -1, 0.4, false);
for (const out of [0, 1, 2]) add(`gun-${out}`, `const [c, x] = cv(30, 44, PLANK); I.drawGunCloset(x, { x: 15, y: 40 }, false, ${out}); I.drawGunCloset(x, { x: 15, y: 40 }, true, ${out}); return c;`);

// on the wall
const clock = (name, h, m, s, a) => add(`clock-${name}`, `const [c, x] = cv(19, 19, WALLC); at(${h}, ${m}, ${s}, (now) => I.drawClock(x, 1, 1, ${a || "undefined"})); return c;`);
clock("a", 14, 25, 37);
clock("b", 3, 50, 5);
clock("work-on", 14, 25, 37, `{ mode: "work", on: true }`);
clock("work-off", 14, 25, 37, `{ mode: "work", on: false }`);
clock("for-2h", 14, 25, 37, `{ mode: "for", until: now + 2 * 3600e3 }`);
clock("for-20m", 3, 50, 5, `{ mode: "for", until: now + 20 * 60e3 }`);
clock("for-8h", 3, 50, 5, `{ mode: "for", until: now + 8 * 3600e3 }`);
clock("for-gone", 3, 50, 5, `{ mode: "for", until: now - 1000 }`);
for (const [n, h] of [["day", 14], ["night", 22]]) add(`window-${n}`, `const [c, x] = cv(23, 20, WALLC); at(${h}, 0, 0, () => I.drawWindow(x, { x0: 1, y0: 1 })); return c;`);
for (const [n, p, ajar, boarded] of [["wood", "I.WOOD", false, false], ["p0-ajar", "I.PAINT[0]", true, false], ["p3-boarded", "I.PAINT[3]", false, true], ["p7-both", "I.PAINT[7]", true, true]])
  add(`door-${n}`, `const [c, x] = cv(20, 24, WALLC); I.drawDoor(x, 2, ${p}, ${ajar}, ${boarded}, 2); return c;`);
for (const open of [0, 1]) add(`closet-${open}`, `const [c, x] = cv(26, 28, WALLC); I.drawCloset(x, { x0: 2, y0: 2 }, ${!!open}); return c;`);
for (const open of [0, 1]) add(`switch-${open}`, `const [c, x] = cv(14, 20, WALLC); I.drawSwitchboard(x, { x0: 2, y0: 3 }, {}, ${!!open}); return c;`);

// the floor's pieces
for (const [n, brewing, T, u] of [["none", false, 0, "null"], ["0", false, 0, "{ used: 0 }"], ["40", false, 0, "{ used: 40 }"], ["100", false, 0, "{ used: 100 }"], ["brew", true, 0.3, "{ used: 20 }"]])
  add(`coffee-${n}`, `const [c, x] = cv(26, 26, PLANK); I.drawCoffee(x, { x: 3, y: 24 }, ${brewing}, ${T}, ${u}); return c;`);
for (const [n, u] of [["none", "null"], ["0", "{ used: 0 }"], ["55", "{ used: 55 }"], ["100", "{ used: 100 }"]])
  add(`cooler-${n}`, `const [c, x] = cv(14, 30, PLANK); I.drawCooler(x, { x: 7, y: 28 }, ${u}); return c;`);
for (const wait of ["undefined", 0.3, 1.2, 3]) add(`printer-${wait}`, `const [c, x] = cv(18, 20, PLANK); I.drawPrinter(x, { x: 9, y: 17 }, ${wait}); return c;`);
for (const i of [0, 1]) for (const k of ["none", 0, 1, 2, 3])
  add(`plant-${i}-${k}`, `const [c, x] = cv(20, 34, PLANK); frozen((now) => I.drawPlant(x, { x: 10, y: 32 }, ${i}, ${k === "none" ? 0 : `now - ${k * 125 + 60}`})); return c;`);
const rec = (n, o) => add(`rec-${n}`, `const [c, x] = cv(50, 42, PLANK), o = ${o}; I.drawReception(x, { x: 2, y: 40 }, o, false); I.drawReception(x, { x: 2, y: 40 }, o, true); return c;`);
rec("linda", `{ n: 3, name: "Linda", color: "#f06b72", screen: null, T: 0 }`);
rec("blank", `{ n: 8, name: "", color: "#9a74d6", screen: null, T: 0 }`);
for (const [s, T] of [["off", 1.3], ["on", 1.3], ["busy", 1.3], ["asks", 0.6]]) rec(s, `{ n: 0, name: "Susan", color: "#4a76b8", screen: "${s}", T: ${T} }`);
for (const [n, linked, T, over] of [["unlinked", false, 0, false], ["linked", true, 0, false], ["over-off", true, 0.1, true], ["over-on", true, 0.2, true]])
  add(`pair-${n}`, `const [c, x] = cv(30, 26, PLANK); I.drawPair(x, { x: 15, y: 22 }, ${linked}, ${T}, ${over}); return c;`);
for (const lit of [0, 1]) add(`add-${lit}`, `const [c, x] = cv(34, 20, PLANK); I.drawAdd(x, { x0: 2, y0: 2 }, ${!!lit}); return c;`);
for (const on of [0, 1]) add(`radio-${on}`, `const [c, x] = cv(18, 26, PLANK); I.drawRadio(x, { x: 9, y: 23 }, ${!!on}); return c;`);
for (const T of [0.3, 1.7]) add(`notes-${T}`, `const [c, x] = cv(18, 46, PLANK); I.drawRadio(x, { x: 9, y: 43 }, true); I.drawNotes(x, { x: 9, y: 43 }, ${T}); return c;`);
for (const lit of [0, 1]) add(`neur-${lit}`, `const [c, x] = cv(16, 22, PLANK); I.drawNeuralyzer(x, { x: 8, y: 20 }, ${!!lit}); return c;`);
add("table", `const [c, x] = cv(36, 34, PLANK); I.drawTable(x, { x: 18, y: 32 }); return c;`);
add("stool", `const [c, x] = cv(18, 18, PLANK); I.drawStool(x, { x: 9, y: 16 }); return c;`);
const MID = "RNBQKB.RPPPP.PPP.....N......P......p...............pppp.ppprnbqkbnr";
for (const [n, b] of [["start", "null"], ["mid", JSON.stringify(MID)]]) add(`chess-${n}`, `const [c, x] = cv(26, 22, PLANK); I.drawChess(x, { x: 13, y: 19 }, ${b}); return c;`);
for (const [n, ball] of [["rest", null], ["0.2-0", [0.2, 0]], ["0.7-1", [0.7, 1]]])
  add(`pong-${n}`, `
    const [c, x] = cv(80, 34, PLANK);
    const w = ${ball ? `{ folk: new Map([[1, { act: { kind: "pong", there: true } }], [2, { act: { kind: "pong", there: true } }]]), pong: { t: ${ball[0]}, from: ${ball[1]} } }` : `{ folk: new Map(), pong: { t: 0, from: 0 } }`};
    I.drawRug(x, { x: 40, y: 26 }); I.drawPong(x, { x: 40, y: 26 }, w); return c;`);
add("mat", `const [c, x] = cv(36, 20, PLANK); I.drawMat(x, { x: 18, y: 18 }); return c;`);
add("doghouse", `const [c, x] = cv(28, 26, PLANK); I.drawDoghouse(x, { x: 14, y: 23 }); return c;`);
add("board-ink", `
  const [c, x] = cv(34, 30, PLANK), [ink, k] = cv(130, 55);
  k.fillStyle = "#2b2433"; k.fillRect(0, 0, 130, 3); k.fillRect(10, 10, 4, 30);
  k.fillStyle = "#e8505b"; k.fillRect(60, 20, 12, 12);
  k.fillStyle = "#4a76b8"; k.fillRect(101, 41, 1, 1);
  I.drawBoard(x, { x: 17, y: 28 }, { ink }); return c;`);

// the dog
for (const pose of ["stand", "walk1", "walk2", "sit", "lie"]) for (const wag of [0, 1]) add(`dog-${pose}-${wag}`, `return I.dogPic("${pose}", ${wag});`);
add("heart", `const [c, x] = cv(9, 7, PLANK); I.heart(x, 4, 2); return c;`);
for (const turn of [0, 1, 2, 3]) add(`stick-${turn}`, `const [c, x] = cv(11, 11, PLANK); I.drawStick(x, 5, 5, ${turn}); return c;`);

// figures: asleep, the remains, the effects
add("sleeper-bot", `await prime("demo0002", I.projectColor("u1"), false); const [c, x] = cv(40, 30, PLANK); I.drawSleeper(x, { id: "f1", x: 20, y: 26, z: 0.3 }, { id: "demo0002", unit: "u1" }, 0, new Set(["f1"])); return c;`);
add("sleeper-you", `await prime("rembert-15", "#d97757", true); const [c, x] = cv(40, 40, PLANK); I.drawSleeper(x, { id: "f2", x: 20, y: 36, z: 0.3 }, { id: "you", human: true, name: "rembert", seed: "rembert-15", color: "#d97757" }, 0, new Set()); return c;`);
const part = (n, p, fade = 0) => add(`part-${n}`, `
  await prime("demo0001", I.projectColor("u1"), false);
  const [c, x] = cv(30, 30, PLANK), img = I.body("demo0001", I.projectColor("u1"), "down", "stand", false, false);
  I.drawPart(x, ${p}, ${fade}); return c;`);
part("cog", `{ kind: "cog", x: 8, y: 20, z: 0, spin: 0.5 }`);
part("cog7", `{ kind: "cog", big: true, x: 15, y: 20, z: 4, spin: 1.5 }`);
part("cogc", `{ kind: "cog", char: true, x: 15, y: 20, z: 0, spin: 2.2 }`);
part("bolt", `{ kind: "bolt", x: 15, y: 20, z: 3, spin: 0 }`);
part("spring", `{ kind: "spring", x: 15, y: 20, z: 0, spin: 0 }`);
part("head", `{ kind: "head", img, sx: 1, sy: 1, sw: 20, sh: 17, x: 15, y: 26, z: 2, spin: 0.2 }`);
part("head-flip", `{ kind: "head", img, sx: 1, sy: 1, sw: 20, sh: 17, x: 15, y: 26, z: 2, spin: 1.2 }`);
part("leg", `{ kind: "leg", img, sx: 7, sy: 23, sw: 4, sh: 6, x: 15, y: 26, z: 0, spin: 3.7 }`);
part("fade", `{ kind: "cog", x: 15, y: 20, z: 0, spin: 0.5 }`, 0.4);
const BODY = `I.body("demo0001", I.projectColor("u1"), "down", "stand", false, false)`;
add("tinted", `await prime("demo0001", I.projectColor("u1"), false); return I.tinted(${BODY}, I.RED, 0.5);`);
add("tinted-gold", `await prime("demo0001", I.projectColor("u1"), false); return I.tinted(${BODY}, I.GOLD, 0.3);`);
for (const d of [0.2, 0.5, 0.7, 0.8]) add(`dissolve-${d}`, `await prime("demo0001", I.projectColor("u1"), false); return I.dissolve(${BODY}, ${d});`);
add("outlined-plant", `const [c, x] = cv(24, 40, PLANK); I.outlined(x, I.foot("plant", [12, 34]), true, (o) => I.drawPlant(o, { x: 12, y: 34 }, 0, 0), I.GOLD); return c;`);
add("outlined-booth", `const [c, x] = cv(26, 44, PLANK); I.outlined(x, I.foot("booth", [13, 38]), true, (o) => I.drawBooth(o, { x: 13, y: 38 }, false, -1, -1), I.RED, 0.5); return c;`);
add("outlined-off", `const [c, x] = cv(24, 40, PLANK); I.outlined(x, I.foot("plant", [12, 34]), false, (o) => I.drawPlant(o, { x: 12, y: 34 }, 1, 0)); return c;`);

// the whiteboard's dreams, whole and begun
for (const i of [0, 1]) for (const n of [300, 1000, -1]) add(`dream-${i}-${n < 0 ? "all" : n}`, `
  const [c, x] = cv(65, 27, "#efebe8"), o = I.dreamScene(${i});
  for (const p of o.slice(0, ${n < 0 ? "o.length" : n})) { x.fillStyle = I.DREAM_PAL[p.c]; x.fillRect(p.x, p.y, 1, 1); }
  return c;`);

// pictures for the page
add("printer-pic", `return I.printerPicture(100, 4).src;`);
for (const i of [-1, 2]) add(`door-pic-${i}`, `return I.doorPicture(${i}, 3);`);

writeFileSync(new URL("./paint-cases.json", import.meta.url), JSON.stringify(cases, null, 1) + "\n");
console.log(cases.length, "cases");
