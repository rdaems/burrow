// The pane as a tiling window manager lays out windows: each tile a border with a thicker band on top
// (its name, and on the left its ✕).
// A new tile splits the focused one along its longer side. Drag a tile by its band onto another: on
// a side, it goes there (the other makes room); in the middle, they swap places. Near the pane's own
// edge, as Sway does at a workspace's: it goes along that whole side, the rest beside it (two tiles side
// by side, one dragged to the bottom: it lies under both, the full width). Drag the gap between two
// tiles to resize them. Nothing is remembered: the tiles are what is open now.
//
// startTiles(el) → { add(tile), remove(tile), focus(tile), focused(), all(), relayout(),
//   preview(clientX, clientY), hide(), addAt(tile, target), forget(tile) }  (something carried in from outside)
// newTile() → { el, bar, body, name(text, sub) }; the caller fills `body` and wires the ✕
// (`el.querySelector(".tx")`).

function newTile() {
  const el = document.createElement("div");
  el.className = "tile";
  el.innerHTML = `<div class="tbar"><button class="tx" title="close">✕</button><span class="tname"></span><span class="tas"></span></div><div class="tbody"></div>`;
  const t = { el, bar: el.querySelector(".tbar"), body: el.querySelector(".tbody") };
  t.name = (text, sub) => { el.querySelector(".tname").textContent = text; el.querySelector(".tas").textContent = sub || ""; };
  return t;
}

function startTiles(el) {
  const G = 6, OUT = 6, MIN = 90;            // the gap between tiles, around them, the least a tile gets
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
  // put `tile` along a whole side of the pane: beside everything else
  function splitOuter(tile, side) {
    const dir = side === "left" || side === "right" ? "row" : "col", before = side === "left" || side === "top";
    const leaf = { leaf: tile };
    if (!root) { root = leaf; return; }
    if (root.dir === dir) {                  // the pane is split that way already: one more along it
      const s = 1 / (root.kids.length + 1);
      root.sizes = root.sizes.map((v) => v * (1 - s));
      if (before) { root.kids.unshift(leaf); root.sizes.unshift(s); } else { root.kids.push(leaf); root.sizes.push(s); }
      leaf.parent = root;
      return;
    }
    const old = root, split = { dir, kids: before ? [leaf, old] : [old, leaf], sizes: before ? [0.5, 0.5] : [0.5, 0.5] };
    old.parent = split; leaf.parent = split; root = split;
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
        g.onpointerdown = (e) => resize(e, n, i);
        el.appendChild(g); gaps.push(g);
      }
      at += s + G;
    });
  }
  function relayout() {
    for (const g of gaps.splice(0)) g.remove();
    rects.clear();
    if (root) place(root, { x: OUT, y: OUT, w: el.clientWidth - 2 * OUT, h: el.clientHeight - 2 * OUT });
    el.classList.toggle("empty", !root);
  }
  // drag the gap: the two beside it trade room. Followed on the window, not on the gap: every move lays the
  // tiles out again, which makes new gaps (the one pressed would be gone, and the drag with it, after a step)
  function resize(e, n, i) {
    if (e.button !== 0) return;
    e.preventDefault();
    const row = n.dir === "row", r0 = rects.get(leaves(n.kids[i])[0]), r1 = rects.get(leaves(n.kids[i + 1]).pop());
    const span = row ? r1.x + r1.w - r0.x : r1.y + r1.h - r0.y, both = n.sizes[i] + n.sizes[i + 1], from = row ? e.clientX : e.clientY, s0 = n.sizes[i];
    el.classList.add(row ? "resizing-row" : "resizing-col");   // (the cursor stays while the gap is remade, and terminals under it don't take the pointer)
    const move = (ev) => {
      const d = ((row ? ev.clientX : ev.clientY) - from) / span * both;
      n.sizes[i] = Math.max(MIN / span * both, Math.min(both - MIN / span * both, s0 + d));
      n.sizes[i + 1] = both - n.sizes[i];
      relayout();
    };
    const up = () => { removeEventListener("pointermove", move); removeEventListener("pointerup", up); el.classList.remove("resizing-row", "resizing-col"); };
    addEventListener("pointermove", move); addEventListener("pointerup", up);
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

  // the drop zone under a point (client coordinates), shown: near the pane's edge, that whole side; else a
  // side of a tile, or its middle; with nothing open, the whole pane. `not` is the tile being dragged, which
  // is no target
  function preview(cx, cy, not) {
    drop.hidden = true;
    const box = el.getBoundingClientRect();
    if (cx < box.left || cx >= box.right || cy < box.top || cy >= box.bottom) return null;
    const others = leaves(root).filter((k) => !not || k.leaf !== not);
    if (others.length) {                     // the pane's own edge: a band as wide as Sway's, a little wider on a big pane
      const W = el.clientWidth, H = el.clientHeight, x = cx - box.left, y = cy - box.top;
      const bx = Math.max(30, W * 0.08), by = Math.max(30, H * 0.08);
      const d = { left: x / bx, right: (W - x) / bx, top: y / by, bottom: (H - y) / by };
      const side = Object.keys(d).sort((a, b) => d[a] - d[b])[0];
      if (d[side] < 1) {
        const iw = W - 2 * OUT, ih = H - 2 * OUT;
        const z = side === "left" ? { x: OUT, y: OUT, w: iw / 2, h: ih } : side === "right" ? { x: OUT + iw / 2, y: OUT, w: iw / 2, h: ih }
          : side === "top" ? { x: OUT, y: OUT, w: iw, h: ih / 2 } : { x: OUT, y: OUT + ih / 2, w: iw, h: ih / 2 };
        Object.assign(drop.style, { left: z.x + "px", top: z.y + "px", width: z.w + "px", height: z.h + "px" });
        drop.hidden = false;
        return { n: null, side, outer: true };
      }
    }
    const under = document.elementsFromPoint(cx, cy).find((x) => x.classList && x.classList.contains("tile") && (!not || x !== not.el));
    const n = under && leaves(root).find((k) => k.leaf.el === under);
    if (!n) {
      if (root) return null;
      Object.assign(drop.style, { left: OUT + "px", top: OUT + "px", width: el.clientWidth - 2 * OUT + "px", height: el.clientHeight - 2 * OUT + "px" });
      drop.hidden = false;
      return { n: null, side: "all" };
    }
    const r = rects.get(n), px = (cx - box.left - r.x) / r.w, py = (cy - box.top - r.y) / r.h;
    const edge = Math.min(px, 1 - px, py, 1 - py);
    const side = edge > 0.28 ? "swap" : px === edge ? "left" : 1 - px === edge ? "right" : py === edge ? "top" : "bottom";
    const z = side === "swap" ? r : side === "left" ? { ...r, w: r.w / 2 } : side === "right" ? { ...r, x: r.x + r.w / 2, w: r.w / 2 }
      : side === "top" ? { ...r, h: r.h / 2 } : { ...r, y: r.y + r.h / 2, h: r.h / 2 };
    Object.assign(drop.style, { left: z.x + "px", top: z.y + "px", width: z.w + "px", height: z.h + "px" });
    drop.hidden = false;
    return { n, side };
  }
  const hide = () => { drop.hidden = true; };
  // a new tile put where a preview pointed: beside the tile there, or in its place (the caller
  // closes what was there; it is returned), or as the only one
  function addAt(tile, target) {
    if (target && target.outer) {
      el.appendChild(tile.el);
      splitOuter(tile, target.side);
      tile.el.addEventListener("pointerdown", () => focus(tile), true);
      tile.bar.addEventListener("pointerdown", (e) => lift(e, tile));
      relayout(); focus(tile);
      return null;
    }
    if (!target || !target.n) { add(tile); return null; }
    el.appendChild(tile.el);
    let was = null;
    if (target.side === "swap") { was = target.n.leaf; target.n.leaf = tile; }
    else splitLeaf(target.n, tile, target.side);
    tile.el.addEventListener("pointerdown", () => focus(tile), true);
    tile.bar.addEventListener("pointerdown", (e) => lift(e, tile));
    relayout();
    focus(tile);
    return was;
  }
  // a tile taken out of the tree without its place folding away (another took it)
  function drop_(tile) { tile.el.remove(); if (focusedTile === tile) focusedTile = null; }

  // ---- dragging a tile by its band: a side of another tile puts it there, the middle swaps
  function lift(e, tile) {
    if (e.button !== 0 || e.target.tagName === "BUTTON" || leaves(root).length < 2) return;
    e.preventDefault();
    const x0 = e.clientX, y0 = e.clientY, bar = tile.bar;
    let on = false, target = null;
    try { bar.setPointerCapture(e.pointerId); } catch {}
    const zone = (ev) => preview(ev.clientX, ev.clientY, tile);
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
      if (target.outer) { unlink(me); splitOuter(tile, target.side); }
      else if (target.side === "swap") { const o = target.n.leaf; target.n.leaf = tile; me.leaf = o; }
      else { unlink(me); splitLeaf(target.n, tile, target.side); }
      relayout();
    };
  }

  new ResizeObserver(relayout).observe(el);
  relayout();
  return { add, remove, focus, focused: () => focusedTile, all: () => leaves(root).map((n) => n.leaf), relayout, preview, hide, addAt, forget: drop_ };
}
