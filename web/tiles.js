// The pane as a tiling window manager lays out windows: each tile a border with a thicker band on top
// (its name, and on the left its buttons: ✕, and a pin that says it is kept when another is opened).
// A new tile splits the focused one along its longer side. Drag a tile by its band onto another: on
// a side, it goes there (the other makes room); in the middle, they swap places. Drag the gap between
// two tiles to resize them. Nothing is remembered: the tiles are what is open now.
//
// startTiles(el) → { add(tile), remove(tile), focus(tile), focused(), all(), relayout() }
// newTile() → { el, bar, body, pinned, name(text, sub) }; the caller fills `body` and wires the
// buttons (`el.querySelector(".tx")`, `.tpin`).

function newTile() {
  const el = document.createElement("div");
  el.className = "tile";
  el.innerHTML = `<div class="tbar"><button class="tx" title="close">✕</button><button class="tpin" title="keep this tile when another opens">◦</button>` +
    `<span class="tname"></span><span class="tas"></span></div><div class="tbody"></div>`;
  const t = { el, bar: el.querySelector(".tbar"), body: el.querySelector(".tbody"), pinned: false };
  t.name = (text, sub) => { el.querySelector(".tname").textContent = text; el.querySelector(".tas").textContent = sub || ""; };
  t.pin = (on) => { t.pinned = on; const b = el.querySelector(".tpin"); b.textContent = on ? "•" : "◦"; b.classList.toggle("on", on); };
  return t;
}

function startTiles(el) {
  const G = 6, MIN = 90;                     // the gap between tiles, the least a tile gets
  let root = null, focusedTile = null;       // a node: {leaf: tile, parent} | {dir: "row"|"col", kids: [...], sizes: [...], parent}
  const gaps = [];                           // the draggable gaps, remade on each layout
  const drop = document.createElement("div");
  drop.className = "tdrop"; drop.hidden = true;
  el.appendChild(drop);

  const leaves = (n, out = []) => { if (!n) return out; if (n.leaf) out.push(n); else n.kids.forEach((k) => leaves(k, out)); return out; };
  const nodeOf = (tile) => leaves(root).find((n) => n.leaf === tile);
  const replace = (old, nu) => {
    nu.parent = old.parent;
    if (!old.parent) root = nu; else old.parent.kids[old.parent.kids.indexOf(old)] = nu;
  };
  const even = (n) => { n.sizes = n.kids.map(() => 1 / n.kids.length); };
  const norm = (n) => { const s = n.sizes.reduce((a, b) => a + b, 0); n.sizes = n.sizes.map((v) => v / s); };

  // put `tile` beside the leaf `at`, on `side` (left right top bottom)
  function splitLeaf(at, tile, side) {
    const dir = side === "left" || side === "right" ? "row" : "col", before = side === "left" || side === "top";
    const leaf = { leaf: tile }, p = at.parent;
    if (p && p.dir === dir) {                // a sibling, half of the other's room
      const i = p.kids.indexOf(at), s = p.sizes[i] / 2;
      p.kids.splice(before ? i : i + 1, 0, leaf); p.sizes.splice(i, 1, s, s);
      leaf.parent = p;
    } else {
      const split = { dir, kids: before ? [leaf, at] : [at, leaf], sizes: [0.5, 0.5] };
      replace(at, split);
      leaf.parent = split; at.parent = split;
    }
  }
  function unlink(n) {                       // take a leaf out; a split with one kid left folds away
    const p = n.parent;
    if (!p) { root = null; return; }
    const i = p.kids.indexOf(n);
    p.kids.splice(i, 1); p.sizes.splice(i, 1); norm(p);
    if (p.kids.length === 1) replace(p, p.kids[0]);
  }

  // ---- where everything goes
  const rects = new Map();
  function place(n, r) {
    if (n.leaf) {
      rects.set(n, r);
      Object.assign(n.leaf.el.style, { left: r.x + "px", top: r.y + "px", width: r.w + "px", height: r.h + "px" });
      return;
    }
    const row = n.dir === "row", total = (row ? r.w : r.h) - G * (n.kids.length - 1);
    let at = row ? r.x : r.y;
    n.kids.forEach((k, i) => {
      const s = Math.round(total * n.sizes[i]);
      place(k, row ? { x: at, y: r.y, w: s, h: r.h } : { x: r.x, y: at, w: r.w, h: s });
      if (i < n.kids.length - 1) {
        const g = document.createElement("div");
        g.className = "tgap " + (row ? "row" : "col");
        Object.assign(g.style, row ? { left: at + s + "px", top: r.y + "px", width: G + "px", height: r.h + "px" }
          : { left: r.x + "px", top: at + s + "px", width: r.w + "px", height: G + "px" });
        g.onpointerdown = (e) => resize(e, n, i, g);
        el.appendChild(g); gaps.push(g);
      }
      at += s + G;
    });
  }
  function relayout() {
    for (const g of gaps.splice(0)) g.remove();
    rects.clear();
    if (root) place(root, { x: 0, y: 0, w: el.clientWidth, h: el.clientHeight });
    el.classList.toggle("empty", !root);
  }
  function resize(e, n, i, g) {              // drag the gap: the two beside it trade room
    e.preventDefault();
    const row = n.dir === "row", r0 = rects.get(leaves(n.kids[i])[0]), r1 = rects.get(leaves(n.kids[i + 1]).pop());
    const span = row ? r1.x + r1.w - r0.x : r1.y + r1.h - r0.y, both = n.sizes[i] + n.sizes[i + 1], from = row ? e.clientX : e.clientY, s0 = n.sizes[i];
    try { g.setPointerCapture(e.pointerId); } catch {}
    g.onpointermove = (ev) => {
      const d = ((row ? ev.clientX : ev.clientY) - from) / span * both;
      n.sizes[i] = Math.max(MIN / span * both, Math.min(both - MIN / span * both, s0 + d));
      n.sizes[i + 1] = both - n.sizes[i];
      relayout();
    };
    g.onpointerup = () => { g.onpointermove = g.onpointerup = null; };
  }

  // ---- the tiles
  function add(tile) {
    el.appendChild(tile.el);
    if (!root) root = { leaf: tile };
    else {
      const at = nodeOf(focusedTile) || leaves(root).pop(), r = rects.get(at) || { w: 1, h: 0 };
      splitLeaf(at, tile, r.w >= r.h ? "right" : "bottom");
    }
    tile.el.addEventListener("pointerdown", () => focus(tile), true);
    tile.bar.addEventListener("pointerdown", (e) => lift(e, tile));
    relayout();
    focus(tile);
  }
  function remove(tile) {
    const n = nodeOf(tile);
    if (!n) return;
    unlink(n);
    tile.el.remove();
    if (focusedTile === tile) { focusedTile = null; const last = leaves(root).pop(); if (last) focus(last.leaf); }
    relayout();
  }
  function focus(tile) {
    if (focusedTile === tile) return;
    if (focusedTile) focusedTile.el.classList.remove("focus");
    focusedTile = tile;
    if (tile) tile.el.classList.add("focus");
  }

  // ---- dragging a tile by its band: a side of another tile puts it there, the middle swaps
  function lift(e, tile) {
    if (e.button !== 0 || e.target.tagName === "BUTTON" || leaves(root).length < 2) return;
    e.preventDefault();
    const x0 = e.clientX, y0 = e.clientY, bar = tile.bar;
    let on = false, target = null;
    try { bar.setPointerCapture(e.pointerId); } catch {}
    const zone = (ev) => {
      drop.hidden = true;
      const under = document.elementsFromPoint(ev.clientX, ev.clientY).find((x) => x.classList && x.classList.contains("tile") && x !== tile.el);
      const n = under && leaves(root).find((k) => k.leaf.el === under);
      if (!n) return null;
      const r = rects.get(n), px = (ev.clientX - el.getBoundingClientRect().left - r.x) / r.w, py = (ev.clientY - el.getBoundingClientRect().top - r.y) / r.h;
      const edge = Math.min(px, 1 - px, py, 1 - py);
      const side = edge > 0.28 ? "swap" : px === edge ? "left" : 1 - px === edge ? "right" : py === edge ? "top" : "bottom";
      const z = side === "swap" ? r : side === "left" ? { ...r, w: r.w / 2 } : side === "right" ? { ...r, x: r.x + r.w / 2, w: r.w / 2 }
        : side === "top" ? { ...r, h: r.h / 2 } : { ...r, y: r.y + r.h / 2, h: r.h / 2 };
      Object.assign(drop.style, { left: z.x + "px", top: z.y + "px", width: z.w + "px", height: z.h + "px" });
      drop.hidden = false;
      return { n, side };
    };
    bar.onpointermove = (ev) => {
      if (!on && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 4) return;
      on = true; tile.el.classList.add("lift");
      target = zone(ev);
    };
    bar.onpointerup = () => {
      bar.onpointermove = bar.onpointerup = null;
      drop.hidden = true; tile.el.classList.remove("lift");
      if (!on || !target) return;
      const me = nodeOf(tile);
      if (target.side === "swap") { const o = target.n.leaf; target.n.leaf = tile; me.leaf = o; }
      else { unlink(me); splitLeaf(target.n, tile, target.side); }
      relayout();
    };
  }

  new ResizeObserver(relayout).observe(el);
  relayout();
  return { add, remove, focus, focused: () => focusedTile, all: () => leaves(root).map((n) => n.leaf), relayout };
}
