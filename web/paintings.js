// burrow: what the robots paint on the easel (room.js): famous paintings, every person in them a robot. Each is a
// list of dots in layers, in the order a painter would put them down (the sky and the ground first, then what stands
// on them, the faces and the details last); the room paints them in that order, a few dots a second while a robot
// stands at the easel. Portrait ones are 48 x 60, landscape ones 66 x 48: three times the easel's canvas in the room.
//
// Paintings.list: [{ title, after, w, h, make() → [{x, y, c, L}] }]; Paintings.order(dots): the brush's order.
window.Paintings = (() => {
  const hash = (x, y, s = 0) => { let h = Math.imul(x * 73856093 ^ y * 19349663 ^ s * 83492791, 2654435761); h ^= h >>> 15; return ((h >>> 0) % 10007) / 10007; };
  const B4 = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
  const dith = (a, b, t, x, y) => (t * 16 > B4[(y & 3) * 4 + (x & 3)] + 0.5 ? b : a);   // from a to b by t, ordered
  const ramp = (cols, t, x, y) => {          // along a row of colours by t (0..1), dithered between neighbours
    const u = Math.max(0, Math.min(0.999, t)) * (cols.length - 1), i = Math.floor(u);
    return dith(cols[i], cols[Math.min(i + 1, cols.length - 1)], u - i, x, y);
  };
  const pick = (cols, t) => cols[Math.max(0, Math.min(cols.length - 1, Math.floor(t * cols.length)))];

  function canvas(w, h) {
    const px = new Map();
    const put = (x, y, c, L) => { x = Math.round(x); y = Math.round(y); if (c && x >= 0 && y >= 0 && x < w && y < h) px.set(L * 1e6 + y * 1000 + x, { x, y, c, L }); };
    const area = (x0, y0, x1, y1, L, fn) => { for (let y = Math.floor(y0); y < y1; y++) for (let x = Math.floor(x0); x < x1; x++) put(x, y, fn(x, y), L); };
    const oval = (cx, cy, rx, ry, L, fn) => {
      for (let y = Math.floor(cy - ry); y <= cy + ry; y++) for (let x = Math.floor(cx - rx); x <= cx + rx; x++) {
        const d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2;
        if (d <= 1) put(x, y, typeof fn === "function" ? fn(x, y, Math.sqrt(d)) : fn, L);
      }
    };
    const inside = (pts, x, y) => { let c = false; for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) { const [xi, yi] = pts[i], [xj, yj] = pts[j]; if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) c = !c; } return c; };
    const poly = (pts, L, fn) => {
      const xs = pts.map((p) => p[0]), ys = pts.map((p) => p[1]);
      for (let y = Math.floor(Math.min(...ys)); y <= Math.max(...ys); y++) for (let x = Math.floor(Math.min(...xs)); x <= Math.max(...xs); x++)
        if (inside(pts, x + 0.5, y + 0.5)) put(x, y, typeof fn === "function" ? fn(x, y) : fn, L);
    };
    const line = (x0, y0, x1, y1, L, c, th = 1) => {
      const n = Math.ceil(Math.hypot(x1 - x0, y1 - y0) * 2) || 1;
      for (let k = 0; k <= n; k++) {
        const x = x0 + ((x1 - x0) * k) / n, y = y0 + ((y1 - y0) * k) / n;
        for (let j = 0; j < th; j++) for (let i = 0; i < th; i++) put(x + i - (th - 1) / 2, y + j - (th - 1) / 2, c, L);
      }
    };
    return { put, area, oval, poly, line, dots: () => [...px.values()] };
  }

  // a robot's head as the room draws its robots (pixel.js robot()): a steel box lit along its top and left edge, shaded
  // right and below, its corners cut; across it the dark screen with the eyes glowing in it; a bolt each side where ears
  // would be; the antenna with its light. Whoever is in a painting has one. o: eye, dim (the eyes' colours), mouth
  // ("grille", "smile", "o", "lips"), turn (-1..1: three quarters, the screen slid that way), top (the screen's first
  // row, from the head's), ant (the antenna's height), light (its colour), glasses, noEars
  const STEEL = { hi: "#f4f6fb", lit: "#c2c9d9", mid: "#8b93ad", dark: "#5d6280", deep: "#3b3a55", scr: "#1a2448" };
  function head(E, x0, y0, w, h, L, o = {}) {
    const eye = o.eye || "#7fdcb0", dim = o.dim || "#2fa087", turn = o.turn || 0, x1 = x0 + w - 1, y1 = y0 + h - 1;
    E.area(x0, y0, x1 + 1, y1 + 1, L, (x, y) => {
      if ((x === x0 || x === x1) && (y === y0 || y === y1)) return null;
      if (y === y0 || x === x0) return STEEL.hi;
      if (y === y1 || x === x1) return STEEL.dark;
      return (x - x0) / w > 0.74 + turn * 0.12 ? STEEL.mid : STEEL.lit;
    });
    const m = o.glasses ? 1 : Math.max(1, Math.round(w * 0.14)), sy0 = y0 + (o.top ?? Math.max(2, Math.round(h * 0.22))), sh = Math.max(3, Math.round(h * 0.42));
    const sx0 = x0 + 1 + m + Math.round(Math.min(0, turn) * m), sx1 = x1 - 1 - m + Math.round(Math.max(0, turn) * m);
    E.area(sx0, sy0, sx1 + 1, sy0 + sh, L + 1, (x, y) => (y === sy0 ? STEEL.deep : STEEL.scr));
    const ey = sy0 + Math.floor(sh / 2) - (sh > 4 ? 1 : 0), span = sx1 - sx0, inset = Math.max(1, Math.round(span * 0.22));
    const two = w > 11 && !o.glasses && span - 2 * inset >= 5;             // eyes two dots wide when they still stand apart
    const eyes = o.glasses ? [sx0 + 1, sx1 - 1] : [sx0 + inset, sx1 - inset - (two ? 1 : 0)];
    for (const x of eyes) {
      if (o.glasses) {                       // round glasses: a ring of light round each eye, a bridge between
        for (const [i, j] of [[-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1]]) E.put(x + i, ey + j, dim, L + 1);
        E.put(x, ey, eye, L + 1);
        continue;
      }
      for (let i = 0; i < (two ? 2 : 1); i++) { E.put(x + i, ey, eye, L + 1); if (sh > 3) E.put(x + i, ey + 1, dim, L + 1); }
    }
    if (o.glasses) E.line(eyes[0] + 2, ey, eyes[1] - 2, ey, L + 1, STEEL.mid);
    const cx = Math.round((sx0 + sx1) / 2);
    if (o.mouth === "smile") { for (let x = cx - 2; x <= cx + 2; x++) E.put(x, sy0 + sh - 1 - (Math.abs(x - cx) === 2 ? 1 : 0), dim, L + 1); }   // a slight smile, lit
    if (o.mouth === "grille") for (let x = x0 + 3; x <= x1 - 3; x++) E.put(x, y1 - 2, x % 2 ? STEEL.dark : STEEL.deep, L + 1);
    if (o.mouth === "lips") for (let x = cx - 1; x <= cx + 1; x++) E.put(x, sy0 + sh + 1, x === cx ? "#e86048" : "#b0503a", L + 1);
    if (o.mouth === "o") E.oval(cx, Math.min(y1 - 2, sy0 + sh + 2), 1.4, 2.2, L + 1, (x, y, d) => (d > 0.7 ? STEEL.deep : "#0e1226"));   // wide open
    if (!o.noEars) { const ym = Math.round((sy0 + sy0 + sh) / 2); E.put(x0 - 1, ym, STEEL.dark, L); E.put(x1 + 1, ym, STEEL.dark, L); E.put(x0 - 1, ym + 1, STEEL.deep, L); E.put(x1 + 1, ym + 1, STEEL.deep, L); }
    if (o.ant) {
      const ax = x0 + Math.floor(w / 2);
      E.line(ax, y0 - 1, ax, y0 - o.ant, L, STEEL.mid);
      E.put(ax, y0 - o.ant - 1, o.light || eye, L + 1);
    }
  }

  // ---- Mona Lisa, after Leonardo: a robot in a dark dress, hands folded, the smile lit on its screen, the veil over
  // its head; the hazy land behind, its road winding on the left, a lake and a bridge on the right
  function monaLisa() {
    const E = canvas(48, 60);
    E.area(0, 0, 48, 26, 0, (x, y) => ramp(["#8e9c6c", "#a9b27e", "#c6c48e", "#d9d09a"], y / 24, x, y));
    const ridge = (x) => 15 + 3 * Math.sin(x / 3.1) + 2 * Math.sin(x / 1.7 + 1);
    E.area(0, 8, 48, 26, 1, (x, y) => (y >= ridge(x) ? ramp(["#9fb39a", "#7c937c", "#66795f"], (y - ridge(x)) / 9, x, y) : null));
    E.area(0, 22, 18, 60, 1, (x, y) => (y > 44 ? (y === 45 ? "#6a5a34" : "#4a3f22") : ramp(["#8a8450", "#6e6a3c", "#55522e"], (y - 22) / 18, x, y)));   // down to the parapet
    for (let y = 22; y < 40; y++) { const rx = 7 + 4 * Math.sin(y / 3); E.put(rx, y, "#c2a866", 1); E.put(rx + 1, y, "#b0944e", 1); }   // the road
    E.area(31, 17, 48, 22, 1, (x, y) => (hash(x, y, 3) < 0.15 ? "#c8d4b8" : "#a9bba4"));   // the lake
    E.area(30, 22, 48, 60, 1, (x, y) => (y > 44 ? (y === 45 ? "#6a5a34" : "#4a3f22") : ramp(["#7e8a58", "#66703f", "#4f552e"], (y - 22) / 18, x, y)));
    for (let x = 38; x < 47; x++) E.put(x, 30 - Math.round(1.6 * Math.sin(((x - 38) / 8) * Math.PI)), "#a89458", 1);   // the bridge
    // the dress, the hair and veil falling to the shoulders
    E.poly([[3, 60], [6, 45], [11, 39], [18, 35], [30, 35], [37, 39], [42, 45], [45, 60]], 2, (x, y) => (hash(x >> 1, y, 5) < 0.16 ? "#3d3523" : (x + y) % 9 === 0 ? "#332c1c" : "#29241a"));
    E.oval(24, 18, 10, 13, 2, (x, y) => ((x * 3 + y) % 7 === 0 ? "#3a2a1a" : "#22180f"));
    E.area(14, 20, 35, 38, 2, (x, y) => ((x * 3 + y) % 7 === 0 ? "#3a2a1a" : "#22180f"));   // the hair behind the neck
    E.poly([[14, 18], [19, 22], [19, 38], [13, 38]], 2, (x, y) => ((x + y * 2) % 5 === 0 ? "#3a2a1a" : "#22180f"));
    E.poly([[34, 18], [29, 22], [29, 38], [35, 38]], 2, (x, y) => ((x + y * 2) % 5 === 0 ? "#3a2a1a" : "#22180f"));
    // the neck in rings, the chest plate in the neckline, the head: a robot's, the smile lit, the antenna through the veil
    E.area(21, 26, 28, 36, 3, (x, y) => (x === 21 ? STEEL.hi : x === 27 ? STEEL.dark : (y - 26) % 3 === 2 ? STEEL.mid : STEEL.lit));
    E.oval(24, 35, 8, 4, 3, (x, y, d) => (y < 35 ? null : d > 0.85 ? STEEL.dark : x < 21 ? STEEL.hi : STEEL.lit));
    head(E, 18, 11, 13, 16, 3, { eye: "#f3c94a", dim: "#a8802a", mouth: "smile", turn: 0.25, ant: 5 });
    // the sleeves, the hands folded, the right over the left
    E.poly([[8, 54], [11, 49], [19, 52], [19, 56], [10, 58]], 3, (x, y) => ((x + y) % 5 === 0 ? "#7a6030" : "#5a4622"));
    E.poly([[40, 54], [37, 48], [29, 51], [28, 55], [38, 58]], 3, (x, y) => ((x - y) % 5 === 0 ? "#7a6030" : "#5a4622"));
    E.oval(20.5, 54.5, 3, 1.8, 4, (x, y, d) => (d > 0.8 ? STEEL.dark : x < 20 ? STEEL.hi : STEEL.lit));   // the hands, steel
    E.oval(26, 53.2, 3.6, 2, 4, (x, y, d) => (d > 0.8 ? STEEL.dark : STEEL.lit));
    for (const x of [24, 26, 28]) E.put(x, 54, STEEL.dark, 4);
    return E.dots();
  }

  // ---- The Starry Night, after van Gogh: the sky in flowing strokes, its great swirl, stars with halos, the moon;
  // the hills, the village and its church; the cypress flaming up; and a robot sitting on the hill, looking up
  function starryNight() {
    const E = canvas(66, 48), blues = ["#16295e", "#1d3576", "#26448e", "#3459a6", "#4a72bc"];
    E.area(0, 0, 66, 34, 0, (x, y) => pick(blues, (0.5 + 0.5 * Math.sin(x * 0.21 + 2.2 * Math.sin(y * 0.19 + x * 0.05) + y * 0.13)) * 0.9 + hash(x, y, 1) * 0.15));
    const swirl = (cx, cy, R) => E.area(cx - R, cy - R * 0.7, cx + R + 1, cy + R * 0.7 + 1, 1, (x, y) => {
      const dx = x - cx, dy = (y - cy) / 0.7, r = Math.hypot(dx, dy);
      if (r > R) return null;
      const s = ((Math.atan2(dy, dx) / (2 * Math.PI)) * 2 + r / 3.2 + 10) % 1;
      return s < 0.22 ? "#9ab8e2" : s < 0.34 ? "#dfe6cc" : s < 0.45 ? "#6f93cf" : null;
    });
    swirl(33, 13, 10); swirl(49, 16, 5); swirl(17, 8, 4);
    const star = (cx, cy, R) => {
      E.oval(cx, cy, R + 1.6, R + 1.6, 2, (x, y, d) => (d > 0.72 ? (hash(x, y, 2) < 0.6 ? "#9cbc6e" : null) : d > 0.4 ? "#f2eba0" : null));
      E.oval(cx, cy, R * 0.55 + 0.4, R * 0.55 + 0.4, 2, "#f3d54a"); E.put(cx, cy, "#fff8d0", 2);
    };
    [[5, 5, 2], [25, 3, 2], [42, 3, 2.4], [53, 9, 2], [60, 21, 2], [38, 25, 1.5], [21, 21, 1.6], [8, 16, 1.6]].forEach(([x, y, r]) => star(x, y, r));
    E.oval(59, 6, 5.6, 5.6, 2, (x, y, d) => (d > 0.72 ? "#e8d870" : null));
    E.oval(59, 6, 3.6, 3.6, 2, (x, y) => ((x - 60.6) ** 2 + (y - 4.6) ** 2 < 7.5 ? "#f2eba0" : "#f0b83a"));
    // the hills, the village with its lights, the church's spire
    const hill = (x) => 30 + 2.2 * Math.sin(x / 6.5 + 1) + 1.5 * Math.sin(x / 2.9);
    E.area(0, 24, 66, 48, 3, (x, y) => (y < hill(x) ? null : y - hill(x) < 1.5 ? "#3d5f9a" : hash(x >> 1, y, 3) < 0.2 ? "#2f5a7a" : y > 43 ? "#1a2c56" : "#22386a"));
    for (const [hx, hy, hw, hh] of [[27, 38, 4, 3], [32, 37, 3, 3], [45, 38, 4, 3], [50, 39, 4, 3], [55, 37, 5, 4], [61, 39, 4, 3], [31, 42, 4, 3], [48, 43, 5, 3], [57, 43, 4, 3], [37, 41, 3, 3]]) {
      E.area(hx, hy, hx + hw, hy + hh, 4, () => "#2c3c5e");
      E.line(hx, hy - 1, hx + hw - 1, hy - 1, 4, "#5276b0");
      if (hash(hx, hy, 4) < 0.75) E.put(hx + 1, hy + 1, "#f0c43a", 4);
    }
    E.area(41, 35, 44, 41, 4, () => "#24304e"); E.line(42.5, 35, 42.5, 26, 4, "#16203a");
    // the cypress, a dark flame from the ground into the sky
    for (let y = 2; y < 48; y++) {
      const cx = 11 + 1.6 * Math.sin(y * 0.33), half = Math.max(0.6, (y - 2) * 0.14 + 0.9 * Math.sin(y * 0.8));
      for (let x = Math.floor(cx - half); x <= cx + half; x++) {
        const e = Math.abs(x - cx) / half, h = hash(x, y >> 1, 6);
        E.put(x, y, e > 0.72 ? (h < 0.5 ? "#5a3e1e" : "#3e4a24") : h < 0.25 ? "#3e4a24" : h < 0.4 ? "#2a3a1e" : "#16241a", 5);
      }
    }
    // a robot sitting on the hill with its back to us, looking up at it all; its antenna's light on
    const bx = 23, by = Math.round(hill(25)) - 7;
    ["...y.", "...n.", ".sss.", ".ssd.", "ssssd", "ss.sd"].forEach((row, j) => [...row].forEach((ch, i) => E.put(bx + i, by + j, { y: "#f3d54a", n: "#5d6280", s: "#9aa4bc", d: "#6a7290" }[ch], 6)));
    return E.dots();
  }

  // ---- The Scream, after Munch: the sky in bands of fire, the dark fjord, the bridge going off to the left, two
  // robots far along it, and in front the one who screams, its clamps to its head
  function scream() {
    const E = canvas(48, 60), sky = ["#e88a32", "#d4502c", "#eab24a", "#c8402a", "#e06a2a"];
    E.area(0, 0, 48, 26, 0, (x, y) => { const b = Math.floor((y + 3.2 * Math.sin(x * 0.16 + y * 0.12) + 1.5 * Math.sin(x * 0.4)) / 3.6); return sky[((b % 5) + 5) % 5]; });
    E.area(0, 18, 48, 48, 1, (x, y) => {
      if (x > 30 - (y - 18) * 0.6 + 3 * Math.sin(y / 4)) return hash(x, y >> 1, 7) < 0.25 ? "#5a7a3a" : (x + y) % 6 === 0 ? "#2f4a2c" : "#3d5e36";   // the land
      const s = Math.sin(x * 0.3 + y * 0.5 + Math.sin(y * 0.3) * 2);
      return s > 0.6 ? "#3a5a94" : s > -0.2 ? "#253c72" : "#1b2c5a";
    });
    const rail = (x) => 30 + (x * 16) / 48;
    E.area(0, 28, 48, 60, 2, (x, y) => (y < rail(x) + 2 ? null : (Math.floor(y - x * 0.33) % 3 === 0 ? "#7a4a2e" : y > 54 ? "#8a5634" : "#9a6440")));
    E.line(0, 29, 48, 45, 2, "#5e3424", 2);
    for (let x = 2; x < 48; x += 7) E.line(x, rail(x), x, rail(x) + 3, 2, "#5e3424");
    for (const fx of [3, 7]) ["y.", "hs", "kk", "kk", "k."].forEach((row, j) => [...row].forEach((ch, i) => E.put(fx + i, 23 + (fx > 4 ? 1 : 0) + j, { y: "#f3d54a", h: STEEL.lit, s: STEEL.scr, k: "#2a2430" }[ch], 3)));   // two far along the bridge
    // the one who screams: the dark body, a long robot's head, the eyes wide, the mouth open; its clamps to its head
    E.poly([[13, 60], [16, 52], [19, 46], [21, 42], [28, 42], [30, 46], [33, 52], [37, 60]], 3, (x, y) => (Math.sin(x * 0.7 + y * 0.35) > 0.55 ? "#3e3a56" : "#2c2a3e"));
    E.line(19, 38, 21, 44, 3, "#2c2a3e", 2); E.line(31, 38, 28, 44, 3, "#2c2a3e", 2);
    head(E, 20, 28, 10, 14, 4, { eye: "#fff6c8", dim: "#e8c040", mouth: "o", ant: 3, light: "#e8402a", noEars: true, top: 2 });
    E.area(18, 31, 20, 39, 6, (x) => (x === 18 ? STEEL.dark : STEEL.mid)); E.area(30, 31, 32, 39, 6, (x) => (x === 31 ? STEEL.dark : STEEL.mid));
    return E.dots();
  }

  // ---- Girl with a Pearl Earring, after Vermeer: a robot turning to look at you over its shoulder, its eyes on its
  // screen, a red light for the parted lips, the blue and yellow turban (its antenna through it), the pearl from the
  // bolt where an ear would be, the white collar
  function pearl() {
    const E = canvas(48, 60);
    E.area(0, 0, 48, 60, 0, (x, y) => (hash(x >> 1, y >> 1, 8) < 0.2 ? "#1c1914" : "#12100c"));
    E.poly([[2, 60], [6, 50], [14, 44], [30, 45], [38, 52], [42, 60]], 1, (x, y) => ramp(["#a8884a", "#8a6e3a", "#6a5428", "#4a3a1c"], (x - 2) / 40 + (y - 44) / 60, x, y));
    E.poly([[21, 36], [31, 36], [31, 45], [22, 45]], 2, (x, y) => ((y - 36) % 3 === 2 ? STEEL.dark : ramp([STEEL.lit, STEEL.mid, STEEL.dark], (x - 21) / 10, x, y)));   // the neck, in rings
    E.poly([[12, 46], [19, 43], [32, 44], [34, 46], [26, 48], [16, 49]], 2, (x, y) => ramp(["#f2ede2", "#d8d0c0", "#a8a090"], (x - 12) / 22, x, y));
    head(E, 19, 18, 17, 19, 3, { eye: "#bfe3ff", dim: "#4a76b8", mouth: "lips", turn: -0.35, top: 6, noEars: true });
    E.poly([[19, 14], [27, 11], [24, 19], [18, 35], [12, 35], [14, 22]], 4, (x, y) => ramp(["#f2d27a", "#d8b04a", "#b08a30", "#7a5e20"], (x - 12) / 16 + (hash(x, y >> 2, 9) - 0.5) * 0.15, x, y));
    E.oval(25, 17, 9.5, 6.5, 4, (x, y) => (y > 21 + (x - 25) * 0.25 ? null : ramp(["#5a86cc", "#3d6ab0", "#2a4f8f", "#1c3666"], (x - 16) / 19 + (Math.sin(x * 0.9 + y * 0.6) > 0.7 ? -0.2 : 0), x, y)));
    // the bolt and its pearl, the antenna through the turban
    E.oval(19.5, 30.5, 1.1, 1.1, 5, STEEL.dark); E.put(19.5, 32, STEEL.mid, 5);
    E.oval(19.5, 34.3, 1.5, 2, 6, (x, y) => (y < 34 && x < 20 ? "#f8f8f4" : y > 35 ? "#7a7a86" : "#c8c8d0"));
    E.line(26, 11, 25, 6, 6, STEEL.mid); E.oval(25, 5, 1.1, 1.1, 6, "#bfe3ff");
    return E.dots();
  }

  // ---- The Great Wave, after Hokusai: the wave rearing up over the boats, its foam reaching out like claws, spray in
  // the sky, Fuji small in the trough; rowing in the boats, robots
  function greatWave() {
    const E = canvas(66, 48);
    E.area(0, 0, 66, 48, 0, (x, y) => ramp(["#d6c8a4", "#e2d6b8", "#ebe1c8"], y / 30, x, y));
    E.area(2, 2, 11, 10, 1, (x, y) => (x === 2 || y === 2 || x === 10 || y === 9 ? "#8a7a5a" : x % 2 === 0 && y > 3 && y < 9 && hash(x, y, 10) < 0.6 ? "#4a4030" : "#e8dcc0"));   // the cartouche
    E.poly([[36, 37], [46, 26], [56, 37]], 1, (x, y) => (y < 29 + hash(x, 0, 11) * 2 ? "#f4efe2" : "#3e4c6e"));
    E.area(0, 35, 66, 48, 2, (x, y) => ((y + Math.round(2 * Math.sin(x / 3))) % 3 === 0 ? "#8fb4d8" : y > 42 ? "#2a4f88" : "#4a78b0"));
    const inA = (x, y) => ((x - 12) / 26) ** 2 + ((y - 34) / 30) ** 2 <= 1, inLip = (x, y) => ((x - 25) / 11) ** 2 + ((y - 9) / 5.5) ** 2 <= 1;
    const rH = (x, y) => Math.hypot((x - 33) / 9.5, (y - 23) / 11), mass = (x, y) => (inA(x, y) || inLip(x, y)) && rH(x, y) >= 1;
    const body = (cx, cy, sx, sy) => (x, y) => {      // its body in bands curving round the hollow, white and pale near the edge
      const r = Math.hypot((x - cx) / sx, (y - cy) / sy), s = (r * 4.5 + Math.sin(Math.atan2(y - cy, x - cx) * 3) * 0.3) % 1;
      if (r < 1.25) return s < 0.3 ? "#f4efe2" : s < 0.6 ? "#8fb4d8" : "#4a78b0";
      return s < 0.18 ? "#8fb4d8" : r > 2.2 ? "#1a3260" : s < 0.5 ? "#2a4f88" : "#1f3a6a";
    };
    E.area(0, 0, 44, 48, 3, (x, y) => (mass(x, y) ? body(33, 23, 9.5, 11)(x, y) : null));
    E.oval(58, 48, 13, 10, 3, body(58, 52, 7, 6));
    for (let x = 6; x < 40; x++) {                   // the foam along its top, fingers of it
      let ye = -1;
      for (let y = 0; y < 48; y++) if (mass(x, y)) { ye = y; break; }
      if (ye < 0 || ye > 22) continue;
      E.put(x, ye, "#f4efe2", 4);
      if (hash(x, 1, 12) < 0.6) E.put(x, ye - 1, "#f4efe2", 4);
      if (hash(x, 2, 12) < 0.3) E.put(x + (hash(x, 3, 12) < 0.5 ? -1 : 1), ye - 2, "#f4efe2", 4);
    }
    for (let x = 27; x < 38; x++) {                  // the claws hanging from the lip into the hollow
      let yb = -1;
      for (let y = 14; y >= 4; y--) if (inLip(x, y) && rH(x, y) >= 1) { yb = y; break; }
      if (yb < 0) continue;
      const n = 1 + Math.floor(hash(x, 4, 12) * 3);
      for (let k = 1; k <= n; k++) E.put(x, yb + k, k === n ? "#d8d4c4" : "#f4efe2", 4);
    }
    for (let x = 46; x < 66; x++) if (hash(x, 5, 12) < 0.7) E.put(x, Math.round(48 - 10 * Math.sqrt(Math.max(0, 1 - ((x - 58) / 13) ** 2))), "#f4efe2", 4);
    E.area(14, 0, 44, 11, 4, (x, y) => (hash(x, y, 13) < 0.07 && !mass(x, y) ? "#f4efe2" : null));   // spray
    // the boats, robots rowing in them
    const boat = (x0, y0, x1, y1) => {
      E.line(x0, y0 + 1, x1, y1 + 1, 5, "#9a8a6a"); E.line(x0, y0, x1, y1, 5, "#e2d4b0");
      for (let k = 2; k < Math.abs(x1 - x0) - 1; k += 3) {
        const x = x0 + k, y = Math.round(y0 + ((y1 - y0) * k) / (x1 - x0));
        E.put(x, y - 1, "#7fdcb0", 5); E.put(x + 1, y - 1, STEEL.scr, 5); E.put(x, y - 2, STEEL.lit, 5); E.put(x + 1, y - 2, STEEL.mid, 5); E.put(x, y - 3, "#f3d54a", 5);
      }
    };
    boat(15, 41, 34, 36); boat(46, 45, 62, 43);
    return E.dots();
  }

  // ---- American Gothic, after Grant Wood: two robots before the white farmhouse with its pointed window, she with a
  // bow on her antenna, the cameo and the apron, he with round glasses, the overalls and the pitchfork
  function gothic() {
    const E = canvas(48, 60);
    E.area(0, 0, 48, 34, 0, (x, y) => ramp(["#b8c8d4", "#cad6de", "#d8e0e4"], y / 24, x, y));
    E.oval(4, 13, 7, 8, 1, (x, y) => (hash(x, y, 14) < 0.3 ? "#5a7a44" : "#3e5a30"));
    E.oval(10, 8, 5, 6, 1, (x, y) => (hash(x, y, 15) < 0.3 ? "#5a7a44" : "#3e5a30"));
    E.area(40, 10, 48, 30, 1, (x, y) => (y === 10 || x === 40 ? "#ece6da" : "#a0402a"));
    E.poly([[12, 32], [12, 10], [25, 1], [38, 10], [38, 32]], 2, (x, y) => (y % 2 === 0 ? "#d6d2c8" : "#ece8e0"));
    E.line(11, 10.5, 25, 0.5, 2, "#5a5a62"); E.line(25, 0.5, 39, 10.5, 2, "#5a5a62");
    E.poly([[20.5, 21], [20.5, 12], [25, 6.5], [29.5, 12], [29.5, 21]], 3, "#3a3a42");
    E.poly([[21.5, 20], [21.5, 12.5], [25, 8], [28.5, 12.5], [28.5, 20]], 3, (x, y) => (x === 25 || y === 14 ? "#3a3a42" : x < 24 ? "#8a5a5a" : "#2a2a32"));
    const steel = (x0, span) => (x, y) => ramp(["#dde2ea", "#bcc4d0", "#9aa2b0", "#747c8a"], (x - x0) / span, x, y);
    // her: the dark dress, the print apron, the white collar and the cameo, her head turned a little, her bow
    E.poly([[0, 60], [2, 42], [7, 37], [20, 37], [24, 42], [24, 60]], 4, (x, y) => ((x + y) % 7 === 0 ? "#4a362c" : "#3a2a22"));
    E.poly([[3, 60], [5, 49], [21, 49], [23, 60]], 4, (x, y) => ((x + y * 2) % 4 === 0 ? "#c8a070" : "#6a4a32"));
    E.area(12, 32, 17, 37, 4, steel(11, 7));
    E.oval(14.5, 37, 6, 2.2, 4, (x, y) => (y < 36 ? null : "#ece6da"));
    E.oval(14.5, 39.2, 1.3, 1.3, 5, (x, y, d) => (d < 0.5 ? "#8a5a3a" : "#c8a070"));
    head(E, 10, 21, 10, 12, 5, { eye: "#7fdcb0", mouth: "grille", turn: 0.4, ant: 4 });
    for (const [x, y] of [[13, 15], [14, 16], [13, 17], [16, 16], [17, 15], [17, 17]]) E.put(x, y, "#d6457a", 7);   // her bow, on the antenna
    E.put(15, 16, "#a03060", 7);
    // him: the black jacket, the striped shirt, the overalls, the long head, round glasses, the pitchfork in his hand
    E.poly([[20, 60], [22, 40], [27, 34], [40, 34], [45, 40], [48, 60]], 4, (x, y) => (x < 26 || x > 41 ? "#2e2e3a" : "#1c1c24"));
    E.poly([[30, 34], [37.5, 34], [34, 45]], 4, (x, y) => (y % 2 ? "#e8e4dc" : "#c8c4bc"));
    E.poly([[29, 43], [38.5, 43], [39.5, 60], [28, 60]], 4, (x, y) => ((x * 3 + y) % 8 === 0 ? "#6a7a9a" : "#4a5a7a"));
    E.line(30, 36, 30, 43, 4, "#4a5a7a"); E.line(37.5, 36, 37.5, 43, 4, "#4a5a7a");
    E.area(32, 29, 36, 34, 4, steel(31, 6));
    head(E, 29, 13, 11, 17, 5, { eye: "#7fdcb0", mouth: "grille", glasses: true, ant: 3 });
    E.line(27.5, 36, 27.5, 60, 7, "#8a6a4a");
    for (const tx of [24.5, 27.5, 30.5]) E.line(tx, 30, tx, 36, 7, "#9aa2a8");
    E.line(24.5, 36, 30.5, 36, 7, "#9aa2a8");
    E.oval(27.5, 48, 2, 1.8, 8, steel(25, 5));
    return E.dots();
  }

  const list = [
    { title: "Mona Lisa", after: "Leonardo", w: 48, h: 60, make: monaLisa },
    { title: "The Starry Night", after: "van Gogh", w: 66, h: 48, make: starryNight },
    { title: "The Scream", after: "Munch", w: 48, h: 60, make: scream },
    { title: "Girl with a Pearl Earring", after: "Vermeer", w: 48, h: 60, make: pearl },
    { title: "The Great Wave", after: "Hokusai", w: 66, h: 48, make: greatWave },
    { title: "American Gothic", after: "Grant Wood", w: 48, h: 60, make: gothic },
  ];
  // the brush's order: layer by layer; within one, in strokes three dots high, back and forth, a little uneven
  const order = (dots) => dots.sort((a, b) => a.L - b.L || Math.floor(a.y / 3) - Math.floor(b.y / 3)
    || (Math.floor(a.y / 3) % 2 ? -1 : 1) * (a.x + hash(a.x, a.y, 99) * 6 - (b.x + hash(b.x, b.y, 99) * 6)));
  return { list, order, PRIMER: "#f2ede2" };
})();
