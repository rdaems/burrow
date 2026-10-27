// The things of the 3d view (fps.js), made for it rather than taken from the room's pixel art: no ink
// outlines, four texels a dot, shaded. The robots, the office managers and the dog are little models of boxes,
// drawn from eight sides (Doom's sprite angles), each robot with its own head (its antenna, its eyes, its
// mouth: pixel.js picks them from its seed, as for its portrait), in its own colour, in the pose the room
// gives it. The furniture is boxes faced with textures painted here (wood with its grain, steel, plastic, the
// screens and lamps that shine); what is round (the pouf, a plant, the pair table, the flames) is a picture,
// the same from every side, as Doom's lamps and barrels. The wall, the floor and the ceiling are painted here too.
//
// A texel whose alpha is GLOW (254) shines: the view does not light it (screens, eyes, the ceiling's lights,
// the fire).
window.Props3D = (() => {
  const TS = 4, GLOW = 254, C = pixelAvatar.C;
  const hex = (h) => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
  const mul = (a, k) => [a[0] * k, a[1] * k, a[2] * k];
  const mix = (a, b, k) => [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k];
  const css = (a, al = 1) => `rgba(${a[0] | 0},${a[1] | 0},${a[2] | 0},${al})`;
  const hash = (x, y, s = 0) => { let h = Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263) + Math.imul(s | 0, 1013904223) | 0; h = Math.imul(h ^ (h >>> 13), 1274126177); return ((h ^ (h >>> 16)) >>> 0) / 4294967296; };
  let font = null;                           // the room's 3x5 letters: { tiny(ctx, s, x, y, col), tinyW(s) }
  const cache = new Map(), memo = (k, f) => { let v = cache.get(k); if (v === undefined) { v = f(); cache.set(k, v); } return v; };

  // a picture texel by texel: fn(i, j) gives [r, g, b] (or [r, g, b, a]), or nothing for clear
  function pix(w, h, fn) {
    const c = document.createElement("canvas"); c.width = Math.max(1, w); c.height = Math.max(1, h); c.v = 1;
    const x = c.getContext("2d", { willReadFrequently: true }), d = x.createImageData(c.width, c.height), p = d.data;
    for (let j = 0; j < c.height; j++) for (let i = 0; i < c.width; i++) {
      const v = fn(i, j);
      if (!v) continue;
      const k = (j * c.width + i) * 4;
      p[k] = v[0]; p[k + 1] = v[1]; p[k + 2] = v[2]; p[k + 3] = v[3] === undefined ? 255 : v[3];
    }
    x.putImageData(d, 0, 0);
    return c;
  }
  // and painted on afterwards, in dots: r(x, y, w, h, rgb, glow) fills, t(text, x, y, rgb, scale) letters
  function over(c, paint) {
    const x = c.getContext("2d", { willReadFrequently: true });
    const r = (px, py, pw, ph, col, glow) => { x.fillStyle = css(col, glow ? GLOW / 255 : 1); x.fillRect(Math.round(px * TS), Math.round(py * TS), Math.round(pw * TS), Math.round(ph * TS)); };
    const t = (s, px, py, col, k = 1) => {     // the room's letters, k texels a font pixel (a dot is TS)
      if (!font) return;
      const w = font.tinyW(s), b = document.createElement("canvas"); b.width = w + 1; b.height = 6;
      font.tiny(b.getContext("2d"), s, 0, 0, css(col));
      x.imageSmoothingEnabled = false;
      x.drawImage(b, 0, 0, w, 5, Math.round(px * TS), Math.round(py * TS), w * k, 5 * k);
    };
    paint(r, t, x);
    c.v++;
    return c;
  }
  const face = (c, glow) => ({ c, sx: 0, sy: 0, sw: c.width, sh: c.height, glow });

  // ---- materials: a face w x h dots of them, a texture of its own (kept by size)
  // wood: its grain along the length, boards a few dots tall, a bevel at the edges
  function woodTex(w, h, base, board = 4) {
    const b = hex(base);
    return pix(w * TS, h * TS, (i, j) => {
      const row = Math.floor(j / (board * TS)), y = j % (board * TS);
      const g = Math.sin((j + 3 * Math.sin(i * 0.045 + row * 2.1)) * 0.95 + row) * 0.05 + (hash(i >> 1, j, row) - 0.5) * 0.07 + (hash(row, 7) - 0.5) * 0.1;
      let k = 0.95 + g;
      if (y === 0) k *= 0.72; else if (y === 1) k *= 1.1;
      if (i < 2 || j < 2) k *= 1.12; else if (i >= w * TS - 2 || j >= h * TS - 2) k *= 0.75;
      return mul(b, k);
    });
  }
  // metal or plastic: smooth, a fine brushed noise, the bevel
  function plainTex(w, h, base, rough = 0.05) {
    const b = hex(base);
    return pix(w * TS, h * TS, (i, j) => {
      let k = 1 + (hash(0, j, 3) - 0.5) * rough + (hash(i, j, 5) - 0.5) * rough * 0.6;
      if (i < 1 || j < 1) k *= 1.18; else if (i >= w * TS - 1 || j >= h * TS - 1) k *= 0.7;
      return mul(b, k);
    });
  }
  const M = {
    wood: (base, board) => (w, h) => face(memo(`w${base}${board}:${w}x${h}`, () => woodTex(w, h, base, board))),
    plain: (base, rough) => (w, h) => face(memo(`p${base}${rough}:${w}x${h}`, () => plainTex(w, h, base, rough))),
  };
  // a box: x0..x1 (west to east), y0..y1 (north to south), z0..z1 (up); its faces from a material, any of them
  // given apart (front: the south face, as the room is seen)
  function blk(x0, y0, x1, y1, z0, z1, m, o = {}) {
    const w = Math.max(0.25, x1 - x0), d = Math.max(0.25, y1 - y0), h = Math.max(0.25, z1 - z0), r = (v) => Math.max(1, Math.round(v * 2) / 2);
    return { x0, y0, x1, y1, z0, z1, front: o.front || m(r(w), r(h)), back: o.back || m(r(w), r(h)), side: o.side || m(r(d), r(h)), top: o.top || m(r(w), r(d)), west: o.west, east: o.east };
  }

  // ---- the little models: boxes in a figure's own frame (f forward, r to its right, z up), drawn from eight
  // sides into a picture each. A face's colour is its box's, or a painter's (u across it as you see it, v down)
  function render(boxes, k, S, R, Z) {
    const th = (k * Math.PI) / 4, cf = Math.cos(th), cr = Math.sin(th), sf = Math.sin(th), sr = -Math.cos(th);
    const w = Math.ceil(2 * R * S), h = Math.ceil(Z * S), zb = new Float32Array(w * h).fill(-1e9);
    const c = document.createElement("canvas"); c.width = w; c.height = h;
    const x = c.getContext("2d", { willReadFrequently: true }), d = x.createImageData(w, h), p = d.data;
    const lf = 0.82 * cf - 0.57 * sf, lr = 0.82 * cr - 0.57 * sr;   // the light: from the front, a little from your left
    for (const b of boxes) for (const [nf, nr, side] of [[1, 0, "F"], [-1, 0, "B"], [0, 1, "R"], [0, -1, "L"]]) {
      if (nf * cf + nr * cr <= 1e-6) continue;
      const pa = side === "F" ? [b.f1, b.r0] : side === "B" ? [b.f0, b.r1] : side === "R" ? [b.f1, b.r1] : [b.f0, b.r0];
      const pb = side === "F" ? [b.f1, b.r1] : side === "B" ? [b.f0, b.r0] : side === "R" ? [b.f0, b.r1] : [b.f1, b.r0];
      let xa = pa[0] * sf + pa[1] * sr, xb = pb[0] * sf + pb[1] * sr, p0 = pa, p1 = pb;
      if (xa > xb) { [xa, xb] = [xb, xa]; [p0, p1] = [pb, pa]; }
      const lam = Math.max(0, nf * lf + nr * lr), shade = 0.56 + 0.44 * lam;
      const X0 = Math.max(0, Math.round((xa + R) * S)), X1 = Math.min(w, Math.round((xb + R) * S));
      const Y0 = Math.max(0, Math.round((Z - b.z1) * S)), Y1 = Math.min(h, Math.round((Z - b.z0) * S));
      const paint = b.paint && b.paint[side], col = b.col, rd = b.round || 0, fw = Math.hypot(pb[0] - pa[0], pb[1] - pa[1]), fh = b.z1 - b.z0;
      for (let X = X0; X < X1; X++) {
        const u = Math.min(0.999, Math.max(0, (X + 0.5 - (xa + R) * S) / ((xb - xa) * S || 1)));
        const pf = p0[0] + (p1[0] - p0[0]) * u, pr = p0[1] + (p1[1] - p0[1]) * u, dep = pf * cf + pr * cr;
        for (let Y = Y0; Y < Y1; Y++) {
          const i = Y * w + X;
          if (dep <= zb[i]) continue;
          const v = Math.min(0.999, (Y + 0.5 - Y0) / (Y1 - Y0 || 1));
          if (rd) { const du = rd - Math.min(u, 1 - u) * fw, dv = rd - Math.min(v, 1 - v) * fh; if (du > 0 && dv > 0 && du * du + dv * dv > rd * rd) continue; }   // its corners rounded
          let rgb = col, glow = b.glow;
          if (paint) { const q = paint(u, v); if (!q) continue; rgb = q; glow = q[3] === GLOW; }
          let k2 = glow ? 1 : shade * (Y - Y0 < 1 ? 1.16 : Y1 - Y <= 1 ? 0.84 : 1) * (X - X0 < 1 || X1 - X <= 1 ? 0.93 : 1);
          if (!glow) k2 *= 0.97 + hash(X, Y, 11) * 0.06;
          zb[i] = dep;
          p[i * 4] = Math.min(255, rgb[0] * k2); p[i * 4 + 1] = Math.min(255, rgb[1] * k2); p[i * 4 + 2] = Math.min(255, rgb[2] * k2); p[i * 4 + 3] = glow ? GLOW : 255;
        }
      }
    }
    x.putImageData(d, 0, 0);
    c.v = 1;
    return c;
  }

  // a robot: legs, its torso in its colour (her skirt), arms in the pose, its head with its face on the front
  const ROBOT = { S: 4, R: 9, Z: 27 };
  function robotBoxes(sp) {
    const P = pixelAvatar.robot(sp.seed), col = hex(sp.color), light = col, lightD = mul(col, 0.62);
    const s1 = hex(C.steel1), s2 = hex(C.steel2), s3 = hex(C.steel3), s4 = hex(C.steel4), dark = hex(C.steel0), screen = hex(C.blue0), ink2 = hex(C.ink2);
    const B = [], box = (f0, f1, r0, r1, z0, z1, c, o = {}) => B.push({ f0, f1, r0, r1, z0, z1, col: c, ...o });
    const legs = sp.legs, sit = legs === "sit";
    if (sit) for (const r of [-2.5, 0.7]) { box(-1.2, 4.6, r, r + 1.8, 3.4, 5.2, s1); box(3.2, 5, r, r + 1.8, 0.8, 3.6, s1); box(3.2, 6, r - 0.1, r + 1.9, 0, 0.8, dark); }
    else {
      const o = legs === "walk1" ? 1.3 : legs === "walk2" ? -1.3 : 0;
      [[-2.5, o], [0.7, -o]].forEach(([r, df]) => { box(-0.9 + df, 0.9 + df, r, r + 1.8, 0.8, 5.2, s1); box(-1 + df, 1.7 + df, r - 0.1, r + 1.9, 0, 0.8, dark); });
    }
    box(-2.1, 2.1, -3.6, 3.6, 5, 12.4, col, { round: 0.9, paint: { F: (u, v) => (v > 0.25 && v < 0.55 && u > 0.36 && u < 0.64 ? (u > 0.43 && u < 0.57 && v > 0.32 && v < 0.48 ? [...mix(light, [255, 255, 255], 0.5), GLOW] : s3) : col) } });   // a chest plate with its light
    if (sp.femme) box(-2.7, 2.7, -4.3, 4.3, 4.6, 8.2, mul(col, 0.82));
    box(-1, 1, -1.5, 1.5, 12.4, 13.2, s2);
    const rows = 12 - P.top, top = 13 + rows;     // the head: as tall as its portrait's (8 or 9 dots), ten wide
    const faceAt = (u, v) => {                   // its front: the portrait's cells (x 3..12, y top..11), as robot() paints them
      const cx = Math.floor(u * 10), cy = Math.floor(v * rows), gx = cx + 3, gy = cy + P.top, ey = P.top + 3;
      let c = cy === 0 || cx === 0 ? s4 : cy === rows - 1 || cx === 9 ? s1 : s3;
      if (gx >= 5 && gx < 11 && gy >= P.top + 2 && gy < P.top + 6) c = screen;
      const lit = (x0, y0) => gx === x0 && gy === y0;
      const e = P.eyes;
      if (e === "dots" && (lit(6, ey) || lit(9, ey))) return [...light, GLOW];
      if (e === "dots" && (lit(6, ey + 1) || lit(9, ey + 1))) return [...lightD, GLOW];
      if (e === "visor" && gy === ey && gx >= 5 && gx < 11) return [...(gx % 2 ? light : lightD), GLOW];
      if (e === "cyclops" && (lit(7, ey) || lit(8, ey) || lit(7, ey + 1))) return [...light, GLOW];
      if (e === "cyclops" && lit(8, ey + 1)) return [...lightD, GLOW];
      if (e === "happy" && (lit(6, ey + 1) || lit(5, ey) || lit(7, ey) || lit(9, ey + 1) || lit(8, ey) || lit(10, ey))) return [...light, GLOW];
      if (gy === 10 && gx >= 6 && gx < 10) c = P.teeth && gx % 2 === 0 ? ink2 : s1;
      return c;
    };
    box(-3.6, 3.6, -5, 5, 13.2, top, s3, { paint: { F: faceAt }, round: 1.4 });
    for (const r of [-5.6, 5]) box(-1.6, -0.2, r, r + 0.6, 16.4, 18.2, s2);     // its ears, small bolts toward the back
    const tip = (f0, r0, z0, w) => box(f0, f0 + w, r0, r0 + w, z0, z0 + w, light, { glow: true });
    if (P.ant === "one") { box(-0.35, 0.35, -0.35, 0.35, top, top + 2.2, s2); tip(-0.6, -0.6, top + 2.2, 1.2); }
    if (P.ant === "two") for (const r of [-2.8, 2.2]) { box(-0.35, 0.35, r, r + 0.6, top, top + 1.4, s2); tip(-0.55, r - 0.25, top + 1.4, 1.1); }
    if (P.ant === "dish") { box(-0.5, 0.5, -2, 2, top, top + 0.9, s3); tip(-0.5, -0.5, top + 0.9, 1); }
    if (sp.femme) { box(-0.9, 0.9, -3.4, -1.1, top + 0.2, top + 2.3, col); box(-0.9, 0.9, 1.1, 3.4, top + 0.2, top + 2.3, col); box(-0.7, 0.7, -1.1, 1.1, top + 0.6, top + 1.9, mul(col, 0.62)); }
    // the arms, by the pose; what is held
    const arm = (side, f0, f1, z0, z1) => { const r0 = side < 0 ? -5.6 : 3.6; box(f0, f1, r0, r0 + 2, z0, z1, s2); };
    const hand = (side, f, z) => { const r0 = side < 0 ? -5.6 : 3.6; box(f - 0.9, f + 0.9, r0, r0 + 2, z, z + 1.2, s1); };
    const down = (side, df = 0) => { arm(side, -1 + df, 1 + df, 6.4, 12); hand(side, df, 5.4); };
    const fwd = (side, z) => { arm(side, -1, 1, z + 1, 12); box(0.8, 5.4, side < 0 ? -5.6 : 3.6, side < 0 ? -3.6 : 5.6, z, z + 1.8, s2); hand(side, 6, z + 0.3); };
    const a = sp.arms || "down";
    if (a === "up") for (const s of [-1, 1]) { arm(s, -1, 1, 12, 18.6); hand(s, 0, 18.6); }
    else if (a === "wave") { down(-1); arm(1, -1, 1, 12, 18.6); hand(1, 0, 18.6); }
    else if (a === "sw1" || a === "sw2") { const o = a === "sw1" ? 1.2 : -1.2; down(-1, o); down(1, -o); }
    else if (a === "fwd1" || a === "fwd2") { fwd(-1, a === "fwd1" ? 9.4 : 8.8); fwd(1, a === "fwd1" ? 8.8 : 9.4); }
    else if (a === "reach") { down(-1); arm(1, -1, 1, 11, 12.4); box(0.8, 5.4, 3.6, 5.6, 11.2, 13, s2); hand(1, 6, 11.4); }
    else if (["book", "mug", "paper", "can", "roll", "gun"].includes(a)) {
      fwd(-1, 9); fwd(1, 9);
      if (a === "book") box(4.2, 5.8, -2.2, 2.2, 9.4, 13.6, hex(C.red1));
      if (a === "mug") box(5.4, 7.2, 2.4, 4.2, 10, 12.2, col);
      if (a === "paper") box(4.6, 5, -2.4, 2.4, 9.4, 15, s4);
      if (a === "can") box(5.2, 7.4, -1.2, 1.2, 9.6, 12, hex(C.green0));
      if (a === "roll") box(4, 6.6, -3, 3, 8.6, 11, [77, 88, 102]);
      if (a === "gun") { box(1, 9, -0.6, 0.6, 10.2, 11.4, s1); box(-1.5, 1.5, -0.7, 0.7, 9.2, 11.2, [122, 74, 42]); }
    } else { down(-1); down(1); }
    return B;
  }
  // the dog, a dachshund: a long body, short legs, its ears, its tail
  const DOG = { S: 4, R: 13, Z: 14 };
  function dogBoxes(sp) {
    const b = hex("#c27a3e"), d = hex("#7a4424"), w = hex("#efd8b4"), n = hex("#1a1626");
    const B = [], box = (f0, f1, r0, r1, z0, z1, c, o = {}) => B.push({ f0, f1, r0, r1, z0, z1, col: c, ...o });
    const headAt = (f, z) => {
      box(f, f + 4, -2, 2, z, z + 4.2, b, { paint: { F: (u, v) => (v > 0.3 && v < 0.5 && (Math.abs(u - 0.25) < 0.08 || Math.abs(u - 0.75) < 0.08) ? n : b) } });
      box(f + 4, f + 6.2, -1.2, 1.2, z + 0.6, z + 2.6, w); box(f + 6.2, f + 6.9, -0.6, 0.6, z + 1.6, z + 2.6, n);
      for (const r of [-2.7, 2]) box(f + 0.6, f + 2.6, r, r + 0.7, z + 0.8, z + 4.4, d);
    };
    const pose = sp.pose || "stand", o = pose === "walk1" ? 0.8 : pose === "walk2" ? -0.8 : 0, wag = sp.wag ? 0.8 : -0.8;
    if (pose === "lie") {
      box(-6, 6, -2.2, 2.2, 0.4, 4, b); box(-6, 6, -2, 2, 0, 0.6, w); headAt(5.5, 0.6);
      box(-8, -6, -0.4 + wag, 0.4 + wag, 1.2, 2, d);
    } else if (pose === "sit") {
      box(-5, 1, -2.2, 2.2, 1, 5.5, b); box(0, 5.4, -2.2, 2.2, 3, 8, b);
      for (const r of [-2, 1]) { box(2.6, 4, r, r + 1.2, 0, 3.4, b); box(-4.5, -1.5, r - 0.2, r + 1.4, 0, 1.6, b); }
      headAt(4, 7.4);
      box(-6.4, -5, -0.4 + wag, 0.4 + wag, 1.4, 2.4, d);
    } else {
      box(-6, 6, -2.2, 2.2, 3, 7, b); box(-5, 5, -1.8, 1.8, 2.6, 3.2, w);
      for (const [f, r, s] of [[4.2, -2, 1], [4.2, 0.8, -1], [-4.6, -2, -1], [-4.6, 0.8, 1]]) box(f - 0.7 + s * o, f + 0.7 + s * o, r, r + 1.2, 0, 3.2, b);
      headAt(5, 5);
      box(-7.8, -6, -0.4 + wag, 0.4 + wag, 6.2, 7.2, d); box(-8.6, -7.6, -0.4 + wag * 1.6, 0.4 + wag * 1.6, 7, 8.6, d);
    }
    return B;
  }
  // a figure from one of the eight sides (k: 0 its front, counting round to its right), kept
  function figure(sp, k) {
    const key = [sp.kind, sp.seed, sp.color, sp.femme, sp.arms, sp.legs, sp.pose, sp.wag, k].join("|");
    let v = cache.get(key);
    if (v) return v;
    const D = sp.kind === "dog" ? DOG : ROBOT, c = render(sp.kind === "dog" ? dogBoxes(sp) : robotBoxes(sp), k, D.S, D.R, D.Z);
    v = { c, w: c.width, h: c.height, ax: c.width / 2, ay: c.height - 1, res: D.S };
    if (cache.size > 4000) cache.clear();
    cache.set(key, v);
    return v;
  }
  // lying down (a nap): its side, turned a quarter, head first
  function lying(sp) {
    return memo("lie|" + sp.seed + sp.color + sp.femme, () => {
      const s = figure({ ...sp, arms: "down", legs: "stand" }, 2), c = document.createElement("canvas");
      c.width = s.h; c.height = s.w; c.v = 1;
      const x = c.getContext("2d"); x.translate(0, s.w); x.rotate(-Math.PI / 2); x.drawImage(s.c, 0, 0);
      return { c, w: c.width, h: c.height, ax: c.width / 2, ay: c.height * 0.5 + 3 * ROBOT.S, res: ROBOT.S };
    });
  }

  // ---- what is round: pictures, the same from every side
  // the pouf: a short green cylinder, its top lit
  const pouf = () => memo("pouf", () => {
    const w = 20 * TS, h = 9 * TS, cx = w / 2, rx = w / 2 - 2, ry = 2.6 * TS, top = 2.8 * TS, side = 5 * TS, g = [[140, 191, 140], [111, 163, 116], [72, 120, 65], [53, 95, 49]];
    return { c: pix(w, h, (i, j) => {
      const u = (i + 0.5 - cx) / rx, e = ry * Math.sqrt(Math.max(0, 1 - u * u));
      if (Math.abs(u) > 1) return null;
      const n = (hash(i, j, 2) - 0.5) * 0.05;
      if (Math.abs(j - top) <= e) { const k = 1 - (j - top + e) / (2 * e + 1) * 0.15 - Math.abs(u) * 0.12 + n; const b = (i - cx) ** 2 + (j - top) ** 2 < 5 ? 0.75 : 1; return mul(g[1], k * b); }
      if (j > top && j <= top + side + e) { const k = 0.92 - 0.25 * ((j - top) / (side + e)) - u * 0.25 + n; return mul(g[2], k); }
      return null;
    }), ax: w / 2, ay: h - 1, res: TS };
  });
  // a potted plant: a terracotta pot, long leaves (a snake plant) or broad ones on stems
  function plant(kind) {
    return memo("plant" + kind, () => {
      const w = 18 * TS, h = 32 * TS, c = document.createElement("canvas"); c.width = w; c.height = h;
      const x = c.getContext("2d"), S = TS;
      const pot = (y0) => {
        for (let j = 0; j < 9 * S; j++) { const hw = (6.5 - j / S * 0.25) * S, k = 1 - j / (9 * S) * 0.25; for (let i = Math.round(w / 2 - hw); i < w / 2 + hw; i++) { const l = 1.08 - Math.abs(i - w / 2) / hw * 0.35; x.fillStyle = css(mul([196, 112, 74], k * l)); x.fillRect(i, y0 + j, 1, 1); } }
        x.fillStyle = css([150, 80, 52]); x.fillRect(w / 2 - 7 * S, y0, 14 * S, S);
        x.fillStyle = css([74, 52, 40]); x.fillRect(w / 2 - 6 * S, y0 - S * 0.5, 12 * S, S);
      };
      const leaf = (bx, by, tx, ty, wid, col) => {     // a leaf from its base to its tip, a lighter vein down it
        const n = Math.ceil(Math.hypot(tx - bx, ty - by));
        for (let k = 0; k <= n; k++) {
          const t = k / n, cx = bx + (tx - bx) * t, cy = by + (ty - by) * t, ww = wid * Math.sin(Math.PI * Math.min(1, t * 1.15 + 0.05)) + 0.5;
          x.fillStyle = css(mul(col, 0.8 + 0.3 * t)); x.fillRect(Math.round(cx - ww), Math.round(cy), Math.round(2 * ww), 1);
          x.fillStyle = css(mul(col, 1.25)); x.fillRect(Math.round(cx), Math.round(cy), 1, 1);
        }
      };
      if (kind) {                                   // broad leaves on stems
        const g = [72, 150, 80];
        for (const [a, l] of [[-1.2, 15], [-0.5, 19], [0.2, 20], [0.8, 17], [1.3, 13], [-0.1, 12]]) {
          const ex = w / 2 + Math.sin(a) * l * S, ey = h - 9 * S - Math.cos(a) * l * S;
          x.fillStyle = css([60, 110, 60]); for (let k = 0; k < 20; k++) { const t = k / 20; x.fillRect(Math.round(w / 2 + (ex - w / 2) * t * 0.7), Math.round(h - 9 * S + (ey - h + 9 * S) * t * 0.7), 2, 2); }
          leaf(w / 2 + (ex - w / 2) * 0.55, h - 9 * S + (ey - h + 9 * S) * 0.55, ex, ey, 4.5 * S * 0.5, mul(g, 0.85 + hash(a * 10, 1) * 0.3));
        }
      } else {                                      // a snake plant: tall pointed leaves
        const g = [58, 128, 72];
        for (const [dx, l, lean] of [[-2.5, 20, -0.25], [-0.8, 24, -0.08], [0.8, 22, 0.1], [2.4, 18, 0.28], [0, 16, 0], [-1.6, 14, -0.4], [1.8, 13, 0.45]])
          leaf(w / 2 + dx * S, h - 9 * S, w / 2 + (dx + lean * l) * S, h - 9 * S - l * S, 1.4 * S, mul(g, 0.8 + hash(dx * 10, 3) * 0.35));
      }
      pot(h - 9 * S);
      const d = x.getImageData(0, 0, w, h), p = d.data;          // crisp: no half-covered texels
      for (let i = 3; i < p.length; i += 4) p[i] = p[i] < 100 ? 0 : 255;
      x.putImageData(d, 0, 0); c.v = 1;
      return { c, ax: w / 2, ay: h - 1, res: TS };
    });
  }
  // the pair table: round, on a pedestal, a speakerphone on it (lit while a room is linked)
  function pairTable(linked) {
    return memo("pair" + linked, () => {
      const w = 26 * TS, h = 13 * TS, wd = hex("#b38857");
      const c = pix(w, h, (i, j) => {
        const u = (i + 0.5 - w / 2) / (w / 2 - 2);
        if (j >= 4 * TS && j < 6 * TS && Math.abs(u) <= 1) return mul(wd, (j < 4.5 * TS ? 1.15 : 0.95) - Math.abs(u) * 0.2);   // the top's edge
        if (j >= 6 * TS && Math.abs(i - w / 2) < 1.2 * TS) return mul(wd, 0.7 - (i - w / 2) / TS * 0.05);                      // the pedestal
        if (j >= h - 1.5 * TS && Math.abs(i - w / 2) < 6 * TS) return mul(wd, 0.6);                                               // its foot
        if (j >= 1.6 * TS && j < 4 * TS && Math.abs(i - w / 2) < 3 * TS) return j < 2.2 * TS && Math.abs(i - w / 2) < TS ? (linked ? [130, 255, 160, GLOW] : [70, 80, 74]) : [62, 66, 70];   // the speakerphone
        return null;
      });
      return { c, ax: w / 2, ay: h - 1, res: TS };
    });
  }
  // the fire's flames: tongues rising from the pit, in four frames; they shine
  function flames(frame, big) {
    return memo("fl" + frame + big, () => {
      const w = 20 * TS, h = (big ? 22 : 15) * TS;
      const c = pix(w, h, (i, j) => {
        const u = (i - w / 2) / (w / 2), up = (h - j) / h, t = frame * 1.7;
        const tongue = 0.5 + 0.22 * Math.sin(u * 11 + t) + 0.2 * Math.sin(u * 23 - t * 1.3) + 0.08 * Math.sin(u * 41 + t * 2) + (hash(i >> 2, frame, 9) - 0.5) * 0.12;
        const reach = (1 - u * u) * tongue;
        if (up > reach || Math.abs(u) > 0.95) return null;
        const q = up / Math.max(0.01, reach);
        const col = q < 0.35 ? [255, 246, 196] : q < 0.6 ? [255, 214, 92] : q < 0.82 ? [244, 140, 40] : [196, 62, 28];
        return [...col, GLOW];
      });
      return { c, ax: w / 2, ay: h - 1, res: TS };
    });
  }

  // ---- the screens: a screen's picture by what it shows (off, on, busy, waits), in the desk's colour
  function screenTex(w, h, state, color, T) {
    const frame = state === "busy" ? Math.floor(T * 5) % 26 : state === "waits" ? Math.floor(T * 2) % 2 : 0;
    return memo(`scr${w}x${h}${state}${color}${frame}`, () => screenPaint(w, h, state, color, frame / 5, frame));
  }
  function screenPaint(w, h, state, color, T, frame) {
    const col = hex(color), on = state !== "off";
    const c = pix(w * TS, h * TS, (i, j) => {
      const bz = 1.2 * TS;
      if (i < bz || j < bz || i >= w * TS - bz || j >= h * TS - bz) return [40, 42, 52];
      if (!on) return (i - j) % 9 === 0 && i > w * TS * 0.6 && j < h * TS * 0.4 ? [44, 54, 78] : [16, 20, 31];
      return [22, 32, 68, GLOW];
    });
    if (on) over(c, (r) => {
      const rows = Math.floor((h - 3) / 1.6), s = state === "busy" ? frame : 0;
      for (let k = 0; k < rows; k++) r(1.8 + (k * 3 % 4) * 0.4, 1.8 + k * 1.6, Math.min(w - 4, 2 + ((k * 7 + s) * 5) % (w - 5)), 0.6, state === "busy" ? col : mul(col, 0.5), true);
      if (state === "waits" && frame) r(1.8, h - 2.6, 1.6, 0.8, [244, 246, 251], true);
    });
    return face(c);
  }

  // ---- the furniture, by kind: boxes (and pictures, and what you can't walk through, and lights) at its place
  const WOODP = { hi: "#cfa86d", top: "#b38857", d: "#885c47", k: "#532e3d" };
  const once = (k, f) => memo("b:" + k, f);                  // a face painted once (for what it shows)
  const tinted = (k, fn) => (w, h) => once(k + w + "x" + h, () => face(pix(w * TS, h * TS, (i, j) => fn(i, j, w, h))));   // a material whose texels depend on a state
  const steel = M.plain("#8b93ad", 0.06), darkSteel = M.plain("#3f4452", 0.08), black = M.plain("#26242c", 0.08), white = M.plain("#e8ecf0", 0.03);
  const wood = M.wood(WOODP.top), woodD = M.wood(WOODP.d), woodHi = M.wood(WOODP.hi, 6);
  function build(p) {
    const out = { boxes: [], things: [], solid: [], lights: [] }, B = (...a) => { out.boxes.push(blk(...a)); }, { x, y } = p;
    const solid = (x0, y0, x1, y1) => out.solid.push([x0, y0, x1, y1]);
    const pic = (pc, px, py, z = 0, o = {}) => out.things.push({ x: px, y: py, z, c: pc.c, sx: 0, sy: 0, sw: pc.c.width, sh: pc.c.height, ax: pc.ax, ay: pc.ay, res: pc.res, piece: p.key, ...o });
    switch (p.kind) {
      case "coffee": {                           // a counter with doors, the brewer, the pot (the five-hour session's limit), mugs
        const front = once("coffee", () => face(over(woodTex(20, 10, WOODP.top), (r) => { for (const dx of [0.6, 10.2]) { r(dx, 1, 9.2, 8, mul(hex(WOODP.d), 1.05)); r(dx + 0.4, 1.4, 8.4, 7.2, hex(WOODP.top)); } r(9, 4.5, 0.8, 1.6, [214, 198, 160]); r(10.6, 4.5, 0.8, 1.6, [214, 198, 160]); })));
        B(x, y - 9, x + 20, y, 0, 10, wood, { front });
        B(x - 0.5, y - 9.5, x + 20.5, y + 0.5, 10, 11, woodHi);
        const brewer = once("brewer" + !!p.brewing, () => face(over(plainTex(7, 9, "#2b2a31", 0.06), (r) => { r(1, 1, 5, 2.2, [70, 72, 84]); r(4.6, 1.6, 0.9, 0.9, p.brewing ? [255, 120, 80] : [235, 76, 63], true); r(1.6, 4, 3.8, 3.6, [16, 14, 18]); r(2.4, 7, 2.2, 0.6, [90, 92, 100]); })));
        B(x + 2, y - 8, x + 9, y - 3, 11, 20, black, { front: brewer });
        const level = p.level == null ? 0 : Math.round(p.level * 20) / 20, pot = tinted("pot" + level, (i, j, w, h) => {
          const v = j / (h * TS);
          if (v < 0.15) return [30, 28, 34];
          if (1 - v < level * 0.8) return 1 - v > level * 0.8 - 0.06 ? [176, 112, 60] : [92, 52, 30];
          return i < 2 ? [214, 228, 238] : [178, 196, 212];
        });
        B(x + 11, y - 7, x + 17, y - 3, 11, 17.5, pot);
        B(x + 17.6, y - 4, x + 19.2, y - 2.4, 11, 12.6, white);
        solid(x, y - 9, x + 20, y);
        break;
      }
      case "cooler": {                           // the water cooler: a white base with its taps, the bottle (the week's limit)
        const front = once("cooler", () => face(over(plainTex(10, 14, "#dfe5ea", 0.04), (r) => { r(2, 3, 6, 4, [200, 210, 218]); r(3, 4.2, 1.2, 1.2, [232, 80, 70], true); r(5.8, 4.2, 1.2, 1.2, [80, 140, 232], true); r(2.6, 7.6, 4.8, 0.8, [90, 96, 110]); r(1, 12, 8, 1, [150, 160, 170]); })));
        B(x - 5, y - 8, x + 5, y, 0, 14, M.plain("#dfe5ea", 0.04), { front });
        const level = p.level == null ? 0 : Math.round(p.level * 20) / 20, bottle = tinted("bottle" + level, (i, j, w, h) => {
          const v = j / (h * TS), edge = i < 2 || i >= w * TS - 2;
          if (v < 0.08) return [60, 120, 190];
          if (1 - v < level * 0.9) return edge ? [90, 160, 220] : [74, 118, 184];
          return edge ? [222, 236, 246] : [196, 218, 234];
        });
        B(x - 4, y - 7, x + 4, y - 1, 14, 24, bottle);
        solid(x - 5, y - 8, x + 5, y);
        break;
      }
      case "printer": {                          // on its stand: a grey printer, its slot, a lamp; a page coming out while it prints
        const stand = once("stand", () => face(over(plainTex(14, 7, "#3f4452"), (r) => { r(1, 3, 12, 0.6, [30, 32, 40]); })));
        B(x - 7, y - 9, x + 7, y, 0, 7, darkSteel, { front: stand });
        const front = once("printer", () => face(over(plainTex(12, 5, "#cfd4dc", 0.04), (r) => { r(1.5, 1.6, 9, 0.8, [40, 42, 50]); r(9.6, 3.2, 0.9, 0.9, [96, 220, 120], true); r(1.4, 3.3, 3, 0.6, [150, 156, 168]); })));
        B(x - 6, y - 8, x + 6, y - 1, 7, 12, M.plain("#cfd4dc", 0.04), { front });
        if (p.printing !== undefined) { const out_ = Math.min(4, p.printing * 2); B(x - 3.5, y - 1, x + 3.5, y - 1 + out_ + 0.2, 9.4, 9.8, white); }
        solid(x - 7, y - 9, x + 7, y);
        break;
      }
      case "manager": {                          // her reception: a panel behind her on two posts, the counter with her name, her screen, a bell
        const col = hex(p.color || "#9a74d6");
        const panel = once("panel" + p.color, () => face(over(plainTex(38, 22, "#2b2433", 0.05), (r) => { for (const dx of [9, 19, 29]) r(dx, 0, 0.4, 22, [34, 28, 42]); r(0, 0, 38, 1, mul(col, 0.5)); })));
        B(x + 4, y - 16, x + 42, y - 14, 0, 22, M.plain("#2b2433", 0.05), { front: panel });
        for (const px of [x + 2, x + 42]) B(px, y - 16.5, px + 2, y - 13.5, 0, 23.5, woodD);
        const name = (p.name || "").toUpperCase(), front = once("counter" + name + p.color, () => face(over(woodTex(46, 12, WOODP.top), (r, t) => {
          if (!name) return;
          const tw = font ? font.tinyW(name) : name.length * 4, pw = tw * 0.5 + 2;
          r(23 - pw / 2, 4, pw, 4.2, [40, 30, 36]); r(23 - pw / 2 + 0.4, 4.4, pw - 0.8, 3.4, mul(col, 0.26));
          t(name, 23 - pw / 2 + 1, 4.9, mix(col, [255, 255, 255], 0.35), 2);
        })));
        B(x, y - 5, x + 46, y, 0, 12, wood, { front });
        B(x - 1, y - 6, x + 47, y + 0.6, 12, 13, woodHi);
        if (p.screen) {
          B(x + 3, y - 4.4, x + 15, y - 3.4, 13.8, 22, darkSteel, { front: screenTex(12, 8.2, p.screen === "asks" ? "waits" : p.screen, p.color || "#9a74d6", p.T || 0) });
          B(x + 8, y - 4.6, x + 10, y - 3.2, 13, 13.8, darkSteel);
        } else if (p.notes) B(x + 5, y - 4.5, x + 13, y - 1, 13, 13 + Math.min(6, p.notes) * 0.7, white);
        B(x + 39, y - 4, x + 43, y - 1, 13, 13.6, M.plain("#c9a43c", 0.04)); B(x + 40.5, y - 3, x + 41.5, y - 2, 13.6, 14.6, M.plain("#e6c65a", 0.04));
        solid(x, y - 16.5, x + 46, y);
        break;
      }
      case "add": {                              // the project cabinet: a low cabinet, a + on its front
        const w = p.w || 30, front = once("add" + w, () => face(over(woodTex(w, 9, WOODP.top), (r) => { r(w / 2 - 6, 2, 12, 5, [46, 31, 40]); r(w / 2 - 0.5, 3, 1, 3, [194, 201, 217]); r(w / 2 - 1.5, 4, 3, 1, [194, 201, 217]); })));
        B(x, y - 8, x + w, y, 0, 9, wood, { front });
        B(x - 0.4, y - 8.4, x + w + 0.4, y + 0.4, 9, 10, woodHi);
        solid(x, y - 8, x + w, y);
        break;
      }
      case "booth": {
        if (p.gun) {                             // hardcore: the gun closet, its glass doors, the shotguns in it (one gone a gun out), a red lamp
          const front = once("guns" + Math.min(2, p.out || 0), () => face(over(plainTex(18, 30, "#4f5844", 0.06), (r) => {
            r(1.4, 2, 15.2, 25, [26, 30, 22]);
            for (const [k, gx] of [[0, 4.6], [1, 12.2]]) if (k < 2 - Math.min(2, p.out || 0)) { r(gx, 3, 0.8, 15, [167, 174, 187]); r(gx + 0.8, 3, 0.6, 15, [108, 115, 128]); r(gx - 0.8, 17, 2.4, 8, [122, 74, 42]); }
            r(8.6, 2, 0.8, 25, [63, 55, 64]); r(7.6, 13, 0.6, 3, [200, 200, 210]); r(9.8, 13, 0.6, 3, [200, 200, 210]);
          })));
          B(x - 9, y - 8, x + 9, y, 0, 30, M.plain("#4f5844", 0.06), { front });
          B(x - 3, y - 6, x + 3, y - 2, 30, 32, black, { front: once("lamp", () => face(over(plainTex(6, 2, "#26242c"), (r) => r(1, 0.4, 4, 1.2, [235, 76, 63], true)))) });
          solid(x - 9, y - 8, x + 9, y);
          break;
        }
        // the booth: a glass cabin on a plinth, its sign on the roof (red while in use); in use its back shines more and more
        const t = p.t == null ? -1 : p.t, glow = t >= 0 ? Math.round(Math.min(1, (t + 0.3) / 4) * 20) / 20 : 0;
        B(x - 9, y - 9, x + 9, y, 0, 2, steel);
        const back = tinted("cabin" + glow, () => (glow ? [...mix([59, 52, 40], [255, 244, 214], glow), GLOW] : [34, 42, 55]));
        B(x - 8, y - 9, x + 8, y - 8, 2, 30, back, { front: back(16, 28) });
        for (const px of [x - 9, x + 8]) for (const py of [y - 9, y - 1]) B(px, py, px + 1, py + 1, 2, 30, steel);
        const glass = tinted("glass", (i, j, w, h) => ((i + j) % 23 === 0 && j < h * TS * 0.6 ? [255, 255, 255] : i < 2 || j < 2 || i >= w * TS - 2 ? [117, 123, 124] : null));
        B(x - 9, y - 8, x - 8.8, y - 1, 2, 30, glass); B(x + 8.8, y - 8, x + 9, y - 1, 2, 30, glass);
        const shut = t < 0 ? 1 : Math.min(1, t / 0.4);
        if (shut > 0.05) B(x + 8 - 16 * shut, y - 0.4, x + 8, y, 2, 30, glass);
        const sign = (w, h) => once("sign" + (t >= 0) + w + h, () => face(over(plainTex(w, h, "#3f3740"), (r) => r(3, 0.8, w - 6, 1.4, t >= 0 ? [235, 76, 63] : [255, 226, 122], true))));
        B(x - 9.5, y - 9.5, x + 9.5, y + 0.5, 30, 33, steel, { front: sign(19, 3) });
        if (glow) out.lights.push({ x, y: y - 5, z: 16, r: 50, col: [1, 0.95, 0.8], k: 0.9 * glow });
        solid(x - 9, y - 9, x + 9, y);
        break;
      }
      case "neuralyzer": {                       // a black pedestal, the silver flasher on it, its bulb
        B(x - 4, y - 4, x + 4, y, 0, 12, black);
        B(x - 6, y - 5, x + 6, y + 1, 12, 14, darkSteel);
        const front = once("flasher" + !!p.flash, () => face(over(plainTex(10, 4, "#b5bfc7", 0.04), (r) => { r(0.4, 1.2, 1.4, 1.6, p.flash ? [255, 255, 240] : [255, 226, 122], true); r(7, 1.4, 1, 1, [235, 76, 63], true); })));
        B(x - 5, y - 4, x + 5, y - 1, 14, 18, M.plain("#b5bfc7", 0.04), { front });
        if (p.flash) out.lights.push({ x, y: y - 3, z: 16, r: 160, col: [1, 1, 1], k: 2.2 });
        solid(x - 4, y - 4, x + 4, y);
        break;
      }
      case "doghouse": {                         // plank walls, an arched dark door; the roof in red steps; its bowl
        const front = once("kennel", () => face(over(woodTex(18, 12, WOODP.top, 3), (r) => { r(5, 4, 8, 8, [30, 21, 18]); r(6, 3, 6, 1, [30, 21, 18]); })));
        B(x - 9, y - 12, x + 9, y, 0, 12, M.wood(WOODP.top, 3), { front });
        const red = M.plain("#a8433a", 0.08);
        B(x - 10.5, y - 13, x + 10.5, y + 1, 12, 14, red); B(x - 7.5, y - 13, x + 7.5, y + 1, 14, 16.4, M.plain("#c9574b", 0.08)); B(x - 4.5, y - 13, x + 4.5, y + 1, 16.4, 18.4, red); B(x - 1.5, y - 13, x + 1.5, y + 1, 18.4, 19.6, M.plain("#7a2a26", 0.08));
        B(x - 12.5, y - 3, x - 8.5, y, 0, 1.4, steel);
        solid(x - 9, y - 12, x + 9, y);
        break;
      }
      case "chess": {                            // a table with the game on it as it stands (white plays from the west), a stool each side
        const board = p.board || "", top = once("chess" + board, () => face(pix(16 * TS, 14 * TS, (i, j) => {
          const bx = i - 2 * TS, by = j - TS, cell = 1.5 * TS;
          if (bx < 0 || by < 0 || bx >= 8 * cell || by >= 8 * cell) return mul(hex(WOODP.top), 0.95 + (hash(i >> 1, j) - 0.5) * 0.08);
          const r = Math.floor(bx / cell), f = 7 - Math.floor(by / cell), pc = board[r * 8 + f], cx = bx % cell - cell / 2, cy = by % cell - cell / 2;
          if (pc && pc !== "." && cx * cx + cy * cy < (cell * 0.38) ** 2) { const wht = pc === pc.toUpperCase(); return mul(wht ? [240, 236, 226] : [40, 34, 44], cx + cy < -1 ? 1.25 : 1); }
          return (r + f) % 2 ? [232, 210, 160] : [168, 120, 76];
        })));
        B(x - 8, y - 15, x + 8, y - 1, 9, 10, wood, { top });
        B(x - 1, y - 9, x + 1, y - 7, 0, 9, woodD); B(x - 4, y - 10, x + 4, y - 6, 0, 1, woodD);
        for (const s of p.seats || []) B(s.x - 3, s.y - 3, s.x + 3, s.y, 0, 6, wood);
        solid(x - 8, y - 15, x + 8, y - 1);
        break;
      }
      case "meet": {                             // the meeting table: papers and a mug on it, four legs; two stools
        const top = once("meet", () => face(over(woodTex(32, 11, WOODP.top, 11), (r) => { r(5, 2, 7, 5, [232, 236, 240]); r(6, 3, 5, 0.5, [150, 156, 168]); r(6, 4.5, 4, 0.5, [150, 156, 168]); r(14, 3, 6, 4, [240, 240, 236]); r(24, 4, 2.4, 2.4, [235, 76, 63]); })));
        B(x - 16, y - 14, x + 16, y - 3, 9, 10.5, wood, { top });
        for (const lx of [x - 15, x + 13.6]) for (const ly of [y - 13, y - 4.4]) B(lx, ly, lx + 1.4, ly + 1.4, 0, 9, woodD);
        for (const s of p.seats || []) B(s.x - 3, s.y - 3, s.x + 3, s.y, 0, 6, wood);
        solid(x - 16, y - 14, x + 16, y - 3);
        break;
      }
      case "pong": {                             // the ping-pong table: green, its white lines, the net across, on legs
        const top = once("pong", () => face(pix(32 * TS, 13 * TS, (i, j) => {
          const W2 = 32 * TS, H2 = 13 * TS;
          if (i < 2 || j < 2 || i >= W2 - 2 || j >= H2 - 2 || Math.abs(j - H2 / 2) < 1) return [244, 246, 251];
          return mul([38, 128, 92], 0.96 + (hash(i >> 2, j >> 2) - 0.5) * 0.06);
        })));
        B(x - 16, y - 18, x + 16, y - 5, 7, 8, M.plain("#2a5c48", 0.04), { top });
        const net = once("net", () => face(pix(13 * TS, 2 * TS, (i, j) => (j < 2 ? [244, 246, 251] : (i + j) % 2 ? [220, 224, 232] : null))));
        out.boxes.push({ x0: x - 0.3, y0: y - 18, x1: x + 0.3, y1: y - 5, z0: 8, z1: 10, front: net, back: net, side: net, top: net });
        for (const lx of [x - 14, x + 13]) for (const ly of [y - 16, y - 7]) B(lx, ly, lx + 1, ly + 1, 0, 7, darkSteel);
        solid(x - 16, y - 18, x + 16, y - 5);
        break;
      }
      case "board": {                            // the whiteboard on wheels: two posts, the board with the dream on it
        for (const px of [x - 15, x + 14]) B(px, y - 3.5, px + 1, y - 2, 0, 28, steel);
        B(x - 15, y - 4, x + 15, y - 2, 1, 2, darkSteel);
        for (const px of [x - 15, x - 10, x + 9, x + 14]) B(px, y - 4.5, px + 1.2, y - 1.5, 0, 1, black);
        if (p.ink) {
          const c = memo("boardc", () => { const c = document.createElement("canvas"); c.width = 150; c.height = 75; c.v = 0; return c; });
          if (c.inkV !== p.inkV) {
            const g = c.getContext("2d"); g.imageSmoothingEnabled = false;
            g.fillStyle = "#3f3740"; g.fillRect(0, 0, 150, 75); g.fillStyle = "#efebe8"; g.fillRect(8, 8, 134, 59);
            g.drawImage(p.ink, 10, 10); g.fillStyle = "#595e60"; g.fillRect(8, 67, 134, 5);
            g.fillStyle = "#eb4c3f"; g.fillRect(20, 67, 12, 3); g.fillStyle = "#4a76b8"; g.fillRect(36, 67, 12, 3);
            c.inkV = p.inkV; c.v++;
          }
          out.boxes.push({ ...blk(x - 15, y - 2.8, x + 15, y - 2, 13, 28, M.plain("#d8d4cf", 0.03)), front: face(c) });
        }
        solid(x - 15, y - 4.5, x + 15, y - 1.5);
        break;
      }
      case "armchair": pic(pouf(), x, y - 4, 0); solid(x - 9, y - 8, x + 9, y); break;
      case "plant": pic(plant(p.variant || 0), x, y - 3, 0); solid(x - 6, y - 7, x + 6, y); break;
      case "pair": pic(pairTable(!!p.linked), x, y - 4, 0); solid(x - 12, y - 8, x + 12, y); break;
      default: return null;
    }
    return out;
  }

  // a project's desk: its cabinet (the name on its plate), the desktop on legs with a keyboard at each seat
  // and the mugs brought, the screen on the cabinet (laptops for the worktrees beside it), a chair a bot
  function desk(d) {
    const out = { boxes: [], things: [], solid: [], lights: [] }, B = (...a) => out.boxes.push(blk(...a));
    const { x0, y, w, cab } = d, P = d.paint, wd = M.wood(P.top, 6), wdk = M.wood(P.d, 6);
    const front = once("desk" + d.label + cab + P.top, () => face(over(woodTex(cab, 11, P.top, 11), (r, t) => {
      const tw = font ? font.tinyW(d.label) : d.label.length * 4, pw = Math.min(cab - 3, tw * 0.5 + 2.4);
      r((cab - pw) / 2, 2.2, pw, 4.4, hex(P.k)); r((cab - pw) / 2 + 0.4, 2.6, pw - 0.8, 3.6, hex(P.plate));
      t(d.label, (cab - tw * 0.5) / 2, 3.2, [194, 201, 217], 2);
      r(1.5, 8.2, cab - 3, 0.4, mul(hex(P.top), 0.7));
    })));
    B(x0, y - 12, x0 + cab, y, 0, 11, wd, { front });
    B(x0 - 0.4, y - 12.4, x0 + cab + 0.4, y + 0.4, 11, 12, M.wood(P.hi, 6));
    if (d.seats.length) {
      const tx = x0 + cab, dw = x0 + w - tx, top = once("desktop" + P.top + dw + d.seats.map((s) => s.x - tx).join() + (d.mugs || []).map((m) => m.x - tx + m.color).join(), () => face(over(woodTex(dw, 7, P.top, 7), (r) => {
        for (const s of d.seats) { r(s.x - 9 - tx, 1.6, 7, 2.2, [63, 55, 64]); for (let k = 0; k < 6; k++) r(s.x - 8.6 - tx + k * 1.1, 2, 0.6, 0.5, [181, 191, 199]); }
        for (const m of d.mugs || []) r(m.x - 13 - tx, 0.6, 2, 2, hex(m.color));
      })));
      B(tx, y - 11, x0 + w, y - 4, 7, 9, wd, { top });
      for (const lx of [tx + 1, ...d.seats.map((s) => s.x + 11), x0 + w - 2]) for (const ly of [y - 10.5, y - 5.5]) B(lx, ly, lx + 1.2, ly + 1, 0, 7, wdk);
      for (const s of d.seats) {               // the office chair, its back behind its sitter (east)
        const cx = s.x, cy = y - 1.6, seat = M.plain("#3f3740", 0.08);
        B(cx - 3, cy - 2.4, cx + 3, cy + 2.4, 4.4, 5.6, seat);
        B(cx + 2.6, cy - 2.4, cx + 3.6, cy + 2.4, 5.6, 13, seat);
        B(cx - 0.4, cy - 0.4, cx + 0.4, cy + 0.4, 1, 4.4, steel);
        B(cx - 2.6, cy - 0.5, cx + 2.6, cy + 0.5, 0, 1, black); B(cx - 0.5, cy - 2.6, cx + 0.5, cy + 2.6, 0, 1, black);
      }
    }
    const cx = x0 + cab / 2;
    B(cx - 9, y - 7.6, cx + 9, y - 6.6, 14, 25, M.plain("#3f3740", 0.06), { front: screenTex(18, 11, d.screen || "off", d.color, d.T || 0) });
    B(cx - 1, y - 8, cx + 1, y - 7, 12, 14, steel); B(cx - 3.5, y - 9.5, cx + 3.5, y - 5.5, 12, 12.6, darkSteel);
    (d.laptops || []).forEach((st, k) => {
      const lx = cx + 10 + k * 9;
      B(lx, y - 8.6, lx + 8, y - 4, 12, 12.5, darkSteel);
      B(lx, y - 9, lx + 8, y - 8.6, 12.5, 18, M.plain("#757b7c", 0.05), { front: screenTex(8, 5.5, st, d.color, d.T || 0) });
    });
    if (d.screen === "busy" || d.screen === "waits" || d.screen === "on") out.lights.push({ x: cx, y: y - 4, z: 19, r: 34, col: mix([0.5, 0.6, 1], hex(d.color).map((v) => v / 255), 0.5), k: d.screen === "busy" ? 0.5 : 0.25 });
    out.solid.push([x0, y - 12, x0 + w, y - 3]);
    return out;
  }

  // ---- the walls: wainscot panels under a rail, the plaster over it, a cornice; on the room's own wall the
  // doors (in their paint), the window (outside by the hour), the clock (telling the time), the closet and the
  // switchboard. WH dots high; `items` in the room's wall band (y0..y1 of `band` dots, the floor at band)
  function wallBase(w, WH) {
    const navy = hex("#28374b"), pan = hex("#1f2b3c"), rail = hex(WOODP.d);
    return pix(w * TS, WH * TS, (i, j) => {
      const z = WH - j / TS, n = (hash(i >> 1, j >> 1, 4) - 0.5) * 0.05;
      if (z < 1) return mul([18, 22, 30], 1 + n);                                    // the skirting
      if (z < 13) {                                                                  // the panels
        const px = (i / TS) % 16, inPanel = px > 1.2 && px < 14.8 && z > 2 && z < 11.6;
        if (!inPanel) return mul(pan, 1.05 + n);
        const e = Math.min(px - 1.2, 14.8 - px, z - 2, 11.6 - z);
        return mul(pan, (e < 0.5 ? (px - 1.2 < 0.5 || 11.6 - z < 0.5 ? 0.78 : 1.25) : 0.92) + n);
      }
      if (z < 14.6) return mul(rail, (z > 14.1 ? 1.2 : z < 13.4 ? 0.75 : 1) + n);   // the rail
      if (z > WH - 1.4) return mul([52, 66, 88], (z > WH - 0.5 ? 0.7 : 1) + n);      // the cornice
      const stripe = Math.floor(i / TS / 4) % 2 ? 1.03 : 0.97;
      return mul(navy, stripe + n);
    });
  }
  const plain = (WH) => memo("plain" + WH, () => { const c = wallBase(32, WH); return c; });
  let wallKey = null, wallC = null;
  function wall(w, WH, band, items, key) {
    if (wallKey === w + "|" + key) return wallC;
    wallKey = w + "|" + key;
    return (wallC = (() => {
      const c = wallBase(w, WH), zOf = (yb) => (band - yb) * (WH / band);
      return over(c, (r, t, x) => {
        for (const it of items) {
          const x0 = it.x0, x1 = it.x1, zt = zOf(it.y0), zb = Math.max(0, zOf(it.y1)), wd = x1 - x0, ht = zt - zb, y = WH - zt;
          if (it.kind === "door") {               // its frame, the door in its paint, two sunk panels, a knob, a plate
            const p = it.paint, k = hex(p.k), tp = hex(p.top), hi = hex(p.hi), dd = hex(p.d);
            r(x0 - 0.8, y - 0.8, wd + 1.6, ht + 0.8, k); r(x0, y, wd, ht, tp);
            for (const [py, ph] of [[2, ht * 0.38], [ht * 0.48, ht * 0.4]]) { r(x0 + 2, y + py, wd - 4, ph, dd); r(x0 + 2.4, y + py + 0.4, wd - 4.8, ph - 0.8, tp); r(x0 + 2.4, y + py + 0.4, wd - 4.8, 0.4, hi); }
            r(x0 + wd - 3, y + ht * 0.5, 1.2, 1.2, [214, 180, 90]); r(x0 + 1, y + ht - 2.4, wd - 2, 1.6, mul(dd, 0.9));
            if (!it.ok) for (const py of [ht * 0.3, ht * 0.62]) { r(x0 - 1.4, y + py, wd + 2.8, 2.2, [94, 60, 36]); r(x0 - 1.4, y + py, wd + 2.8, 0.5, [140, 98, 60]); }
          } else if (it.kind === "window") {      // its frame and sill, four panes, the sky by the hour
            const h = it.hour, day = h >= 7 && h < 19, dusk = h >= 18 && h < 21 || h >= 6 && h < 8;
            const sky0 = day ? [120, 176, 230] : dusk ? [232, 150, 110] : [20, 28, 60], sky1 = day ? [190, 222, 246] : dusk ? [250, 200, 150] : [36, 44, 86];
            r(x0 - 0.6, y - 0.6, wd + 1.2, ht + 1.2, [216, 220, 228]);
            for (let k = 0; k < 8; k++) r(x0, y + k * ht / 8, wd, ht / 8 + 0.1, mix(sky0, sky1, k / 7), true);
            if (!day && !dusk) for (let k = 0; k < 9; k++) r(x0 + 1 + hash(k, 1) * (wd - 2), y + 1 + hash(k, 2) * ht * 0.6, 0.4, 0.4, [240, 240, 220], true);
            else r(x0 + wd * 0.65, y + ht * 0.2, 2.4, 2.4, day ? [255, 240, 170] : [255, 200, 120], true);
            r(x0 + wd / 2 - 0.4, y, 0.8, ht, [216, 220, 228]); r(x0, y + ht / 2 - 0.4, wd, 0.8, [216, 220, 228]);
            r(x0 - 1.2, y + ht, wd + 2.4, 1.2, [190, 194, 204]);
          } else if (it.kind === "clock") {       // round, its hours, the hands at this computer's time
            const cx = x0 + wd / 2, cy = y + ht / 2, R = Math.min(wd, ht) / 2, now = it.time;
            x.fillStyle = "#3f3740"; x.beginPath(); x.arc(cx * TS, cy * TS, R * TS, 0, 7); x.fill();
            x.fillStyle = "#c83c30"; x.beginPath(); x.arc(cx * TS, cy * TS, (R - 0.3) * TS, 0, 7); x.fill();
            x.fillStyle = "#f4f1ea"; x.beginPath(); x.arc(cx * TS, cy * TS, (R - 1.4) * TS, 0, 7); x.fill();
            for (let hh = 0; hh < 12; hh++) { const a = hh * Math.PI / 6; r(cx + Math.sin(a) * (R - 2.4) - 0.3, cy - Math.cos(a) * (R - 2.4) - 0.3, 0.6, 0.6, hh % 3 ? [150, 150, 160] : [40, 40, 50]); }
            const hand = (a, len, wdt, col) => { x.strokeStyle = col; x.lineWidth = wdt * TS; x.lineCap = "round"; x.beginPath(); x.moveTo(cx * TS, cy * TS); x.lineTo((cx + Math.sin(a) * len) * TS, (cy - Math.cos(a) * len) * TS); x.stroke(); };
            const hrs = now.getHours() % 12 + now.getMinutes() / 60, mins = now.getMinutes();
            hand(hrs * Math.PI / 6, R * 0.45, 0.8, "#1a1626"); hand(mins * Math.PI / 30, R * 0.7, 0.55, "#1a1626");
            if (it.awake) { x.fillStyle = "rgba(255,226,122,0.35)"; x.beginPath(); x.arc(cx * TS, cy * TS, (R - 1.4) * TS, 0, 7); x.fill(); }
          } else if (it.kind === "closet") {      // a wardrobe: two doors, their handles
            const wk = hex(WOODP.k), wt = hex(WOODP.top), wdd = hex(WOODP.d);
            r(x0 - 0.5, y - 0.5, wd + 1, ht + 0.5, wk); r(x0, y, wd, ht, wt);
            for (const dx of [0.8, wd / 2 + 0.4]) { r(x0 + dx, y + 1.5, wd / 2 - 1.2, ht - 2.5, wdd); r(x0 + dx + 0.4, y + 1.9, wd / 2 - 2, ht - 3.3, wt); }
            r(x0 + wd / 2 - 1.4, y + ht * 0.45, 0.6, 2, [214, 198, 160]); r(x0 + wd / 2 + 0.8, y + ht * 0.45, 0.6, 2, [214, 198, 160]);
          } else if (it.kind === "settings") {    // the switchboard: a steel box, one big lever, its red knob
            r(x0 - 0.4, y - 0.4, wd + 0.8, ht + 0.8, [40, 42, 50]); r(x0, y, wd, ht, [139, 147, 173]); r(x0 + 0.4, y + 0.4, wd - 0.8, 0.5, [194, 201, 217]);
            r(x0 + wd / 2 - 0.5, y + 2, 1, ht * 0.5, [60, 64, 74]); r(x0 + wd / 2 - 1.4, y + 1, 2.8, 2.4, [235, 76, 63]);
          }
        }
      });
    })());
  }

  // ---- the floor: planks five dots wide, their joints staggered, a grain; the ceiling: tiles, a light panel
  // among them (it shines). Both repeat
  const floorTile = () => memo("floor", () => pix(64 * TS, 60 * TS, (i, j) => {
    const row = Math.floor(j / (5 * TS)), yy = j % (5 * TS), off = hash(row, 1) * 64 * TS, xx = (i + off) % (64 * TS), plank = Math.floor(xx / (32 * TS)), px = xx % (32 * TS);
    const tone = 0.9 + hash(row, plank, 2) * 0.2, g = Math.sin((j * 0.9 + 4 * Math.sin(i * 0.03 + row)) * 0.7) * 0.05 + (hash(i >> 1, j, 6) - 0.5) * 0.06;
    let k = tone + g;
    if (yy === 0) k *= 0.62; else if (yy === 1) k *= 1.1;
    if (px === 0) k *= 0.7;
    return mul([115, 71, 40], k);
  }));
  const ceilTile = () => memo("ceil", () => pix(64 * TS, 96 * TS, (i, j) => {
    const x = i / TS, y = j / TS, n = (hash(i >> 1, j >> 1, 8) - 0.5) * 0.06;
    if (x >= 16 && x < 48 && y >= 40 && y < 56) {
      if (x < 16.6 || x >= 47.4 || y < 40.6 || y >= 55.4) return [141, 149, 166];
      return [...mul([238, 240, 230], 0.96 + (Math.floor(x) % 4 === 0 ? -0.06 : 0)), GLOW];
    }
    if (x % 16 < 0.5 || y % 16 < 0.5) return mul([40, 50, 74], 1 + n);
    return mul([56, 68, 94], 1 + n + ((x % 16) < 1 || (y % 16) < 1 ? 0.12 : 0));
  }));
  // the fire's pit on the floor: a ring of stones, the embers inside (they shine)
  function pit(w, h) {
    return memo("pit" + w + "x" + h, () => pix(w * TS, h * TS, (i, j) => {
      const u = (i + 0.5) / (w * TS) * 2 - 1, v = (j + 0.5) / (h * TS) * 2 - 1, d = Math.hypot(u, v);
      if (d > 1) return null;
      if (d > 0.72) { const s = hash(Math.floor(Math.atan2(v, u) * 5), 3); return mul([128, 124, 132], 0.75 + s * 0.4 - (d - 0.72) * 0.8); }
      if (d > 0.66) return [40, 32, 30];
      const e = hash(i >> 2, j >> 2, 5);
      return e > 0.55 ? [...mul([255, 120, 40], 0.6 + e * 0.5), GLOW] : [46, 30, 26];
    }));
  }

  return { TS, GLOW, init: (f) => { font = f; }, figure, lying, build, desk, wall, plain, floorTile, ceilTile, flames, pit, plant, pouf };
})();
