// node paint-cases2.mjs: writes paint-cases2.json beside it, the cases for what room.js drew since 3f3764b (Halloween's
// costumes, the pumpkin and the cobwebs; the sausage at the fire; a desk whose machine is out of reach), for
// tests/painters.rs and tests/golden.rs, kept apart from paint-cases.mjs:
//   node native/tools/golden.mjs http://localhost:7811/ native/crates/room/tests/golden/paint-cases2.json native/crates/room/tests/golden
// (a BURROW_DEMO=1 server). As paint-cases.mjs: each draws one painter of room.js with fixed parameters, a pixel a
// dot, over the floor or the wall; the bodies are body()'s pictures as they are (body-<seed>-<arms>-<legs>-<kind>-<colour>,
// kind h or r, then f femme, x red eyes, t a witch's hat, c a vampire's cape, b a skeleton's suit).
import { writeFileSync } from "node:fs";

const P = `
for (let i = 0; i < 100 && !["PLANT", "WHITEBOARD"].every((n) => sprites.get(n)); i++) await new Promise((r) => setTimeout(r, 50));
const cv = (w, h, bg) => { const c = document.createElement("canvas"); c.width = w; c.height = h; const x = c.getContext("2d"); x.imageSmoothingEnabled = false; if (bg) { x.fillStyle = bg; x.fillRect(0, 0, w, h); } return [c, x]; };
const RD = Date;
const at = (h, m, s, fn) => { const fixed = new RD(2026, 9, 29, h, m, s).getTime(); window.Date = class extends RD { constructor(...a) { super(...(a.length ? a : [fixed])); } static now() { return fixed; } }; try { return fn(fixed); } finally { window.Date = RD; } };
const hover = (p, fn) => { const o = I.pointer; I.pointer = p; try { return fn(); } finally { I.pointer = o; } };
const patch = (k, v, fn) => { const o = I.app[k]; I.app[k] = v; try { return fn(); } finally { I.app[k] = o; } };
const PLANK = "#734728", WALLC = "#28374b";
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
const add = (name, body) => cases.push({ name, js: `(async (I) => { ${P}\n${body} })(window.__in)` });

// the costumes (and the hand at its mouth, eating): body() as it is, its portrait loaded first
const DRESS = { t: "hat", c: "cape", b: "bones" };
const body = (seed, arms, legs, kind, col) => {
  const human = kind[0] === "h", femme = kind.includes("f"), red = kind.includes("x"), dress = Object.entries(DRESS).find(([k]) => kind.slice(1).includes(k));
  add(`body-${seed}-${arms}-${legs}-${kind}-${col}`, `
    I.face("${seed}", "#${col}", ${human}); await new Promise((r) => setTimeout(r, 150));
    return I.body("${seed}", "#${col}", "${arms}", "${legs}", ${human}, ${femme}, ${red}, ${dress ? `"${dress[1]}"` : "undefined"});`);
};
body("demo0001", "down", "stand", "rt", "4a76b8");
body("demo0001", "up", "walk1", "rt", "4a76b8");
body("demo0004", "fwd1", "sit", "rt", "f2a531");
body("demo0002", "sw1", "walk1", "rc", "e8402a");
body("demo0002", "gun", "stand", "rxc", "e8402a");
body("demo0003", "fwd1", "sit", "rb", "2fa087");
body("demo0003", "down", "stand", "rb", "2fa087");
body("demo0003", "sw2", "walk2", "rb", "2fa087");
body("manager-16", "down", "stand", "rfc", "9a74d6");
body("rembert-15", "down", "stand", "ht", "d97757");
body("stijnslaets-3", "wave", "walk2", "ht", "d97757");
body("demo0001", "eat", "stand", "r", "4a76b8");
body("rembert-15", "eat", "stand", "h", "d97757");

// the pumpkin: lit (its eyes and grin flickering by the time), blown out; as the closet shows it
for (const t of [0, 0.4, 1.3, 2.9]) add(`paint-pumpkin-on-${t}`, `const [c, x] = cv(20, 18, PLANK); I.drawPumpkin(x, { x: 10, y: 15 }, true, ${t}); return c;`);
add("paint-pumpkin-off", `const [c, x] = cv(20, 18, PLANK); I.drawPumpkin(x, { x: 10, y: 15 }, false, 1.3); return c;`);
add("paint-icon-pumpkin", `return at(14, 25, 37, () => I.icon("pumpkin", 1, 4));`);
add("paint-icon2-pumpkin", `return at(14, 25, 37, () => I.icon("pumpkin", 2, 4));`);

// the cobwebs in the wall's top corners and the spider between them, across the room's width (the picture's)
for (const t of [0, 1.7, 4.4]) add(`paint-cobwebs-${t}`, `const [c, x] = cv(I.W, 30, WALLC); I.drawCobwebs(x, ${t}); return c;`);

// a sausage at the fire: over the flames as it browns, turned, smoking, then eaten bite by bite (a figure at 12, 34
// facing right, at 48, 34 facing left)
for (const [a, d] of [[0.5, 1], [3, -1], [9, 1], [13, -1], [16.5, 1], [18.5, -1], [20.5, 1]])
  add(`paint-sausage-${a}-${d}`, `const [c, x] = cv(60, 40, PLANK); I.drawSausage(x, { act: { t: ${a}, cook: 16 }, dir: ${d} }, ${d > 0 ? 12 : 48}, 34, 1.3); return c;`);

// a desk whose machine is out of reach: snow on its screen and laptops, no + (this computer's door taken as out of
// reach: the desks' machine is "")
const cut = (name, spec, T, pointer) => add(`paint-desk-${name}`, `
  const [c, x] = cv(130, 50, PLANK), [d, w] = desk(${spec});
  const sc = I.screenAt(d), P = ${JSON.stringify(pointer || null)};
  const pt = P === "screen" ? { x: sc.sx + 5, y: sc.sy + 5 } : P === "plus" ? { x: sc.sx + 22 + d.trees.length * 13 + 4, y: sc.sy + 2 } : null;
  const st = I.app.state(), off = () => ({ ...st, machines: [{ id: "", name: "here", ssh: "", error: "no route to host" }] });
  patch("state", off, () => patch("shells", () => [], () => patch("hlShell", () => null, () => hover(pt, () => I.drawDesk(x, d, w, ${T}, new Set(), false)))));
  return c;`);
cut("cut", `{ x0: 4, y: 46, label: "burrow", dir: "/u/burrow", trees: ["/u/burrow-wt1", "/u/b/.claude/worktrees/x"], seats: [{ busy: true }, { tree: "/u/burrow-wt1", waiting: true }, { tree: "/u/b/.claude/worktrees/x" }] }`, 0.6);
cut("cut-later", `{ x0: 4, y: 46, label: "pair", dir: "/Users/x/pair", trees: [], seats: [{}, {}] }`, 1.3);
cut("cut-screen", `{ x0: 4, y: 46, label: "pair", dir: "/Users/x/pair", trees: [], seats: [{}, {}] }`, 1.3, "screen");
cut("cut-plus", `{ x0: 4, y: 46, label: "pair", dir: "/Users/x/pair", trees: [], seats: [{}, {}] }`, 1.3, "plus");

writeFileSync(new URL("./paint-cases2.json", import.meta.url), JSON.stringify(cases, null, 1));
console.log(cases.length + " cases");
