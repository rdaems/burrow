// Doom's data, read from a WAD as Doom reads it: the lump directory; PLAYPAL (fourteen palettes: the normal one,
// eight reds for pain, four yellows for a pickup, a green), COLORMAP (34 maps: 32 light levels, then
// invulnerability and black); patches (columns of posts, with offsets), flats (64x64 raw), wall textures composed
// from patches (TEXTURE1/2, PNAMES), sprites by frame and rotation (POSSA1, POSSA2A8: rotation 2, mirrored as 8),
// and maps (THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS). A picture here is
// {w, h, px: Uint8Array of palette indices, op: Uint8Array, 1 where it is opaque, lo, to: its offsets}.
// Also a way into the palette for burrow's own pictures (quantize): nearest colour, through a 32x32x32 table.
//
// Wad.load(url) gives a promise of the WAD; the 3d view (fps.js) loads /doom.wad once a page.
window.Wad = (() => {
  const text = (d, o, n) => { let s = ""; for (let i = 0; i < n && d[o + i]; i++) s += String.fromCharCode(d[o + i]); return s.toUpperCase(); };

  function open(buf) {
    const d = new Uint8Array(buf), dv = new DataView(buf);
    const n = dv.getInt32(4, true), off = dv.getInt32(8, true);
    const dir = [];
    for (let i = 0; i < n; i++) dir.push({ name: text(d, off + 16 * i + 8, 8), pos: dv.getInt32(off + 16 * i, true), size: dv.getInt32(off + 16 * i + 4, true) });
    const by = new Map();
    dir.forEach((l, i) => { if (!by.has(l.name)) by.set(l.name, i); });
    const bytes = (l) => d.subarray(l.pos, l.pos + l.size);
    const lump = (name) => { const i = by.get(name.toUpperCase()); return i === undefined ? null : bytes(dir[i]); };
    const view = (u8) => new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const between = (a, b) => { const out = []; let on = false; for (const l of dir) { if (l.name === a || l.name === a.replace("_", "_")) on = true; else if (l.name === b) on = false; else if (on && l.size) out.push(l); } return out; };

    // ---- colour
    const PP = lump("PLAYPAL"), CM = lump("COLORMAP");
    const rgb = [];                                  // per palette: 256 colours as ImageData words (ABGR)
    for (let p = 0; p < PP.length / 768; p++) {
      const a = new Uint32Array(256);
      for (let i = 0; i < 256; i++) a[i] = 0xff000000 | PP[p * 768 + i * 3 + 2] << 16 | PP[p * 768 + i * 3 + 1] << 8 | PP[p * 768 + i * 3];
      rgb.push(a);
    }
    let cube = null;                                 // r>>3, g>>3, b>>3 → the nearest of the palette's colours
    function quantizer() {
      if (cube) return cube;
      cube = new Uint8Array(32768);
      const R = [], G = [], B = [];
      for (let i = 0; i < 256; i++) { R.push(PP[i * 3]); G.push(PP[i * 3 + 1]); B.push(PP[i * 3 + 2]); }
      for (let r = 0; r < 32; r++) for (let g = 0; g < 32; g++) for (let b = 0; b < 32; b++) {
        const rr = r * 8 + 4, gg = g * 8 + 4, bb = b * 8 + 4;
        let best = 0, bd = 1e9;
        for (let i = 0; i < 256; i++) {
          const dr = rr - R[i], dg = gg - G[i], db = bb - B[i], dd = 3 * dr * dr + 4 * dg * dg + 2 * db * db;
          if (dd < bd) { bd = dd; best = i; }
        }
        cube[r << 10 | g << 5 | b] = best;
      }
      return cube;
    }
    const nearest = (r, g, b) => quantizer()[(r >> 3) << 10 | (g >> 3) << 5 | b >> 3];

    // ---- pictures
    const patches = new Map();
    function patch(name) {
      name = name.toUpperCase();
      if (patches.has(name)) return patches.get(name);
      const p = lump(name);
      let out = null;
      if (p && p.length > 8) {
        const v = view(p), w = v.getInt16(0, true), h = v.getInt16(2, true);
        out = { w, h, lo: v.getInt16(4, true), to: v.getInt16(6, true), px: new Uint8Array(w * h), op: new Uint8Array(w * h) };
        for (let x = 0; x < w; x++) {
          let q = v.getUint32(8 + 4 * x, true), top = -1;
          while (q < p.length && p[q] !== 255) {
            const t = p[q], len = p[q + 1];
            top = t <= top ? top + t : t;           // tall patches: a post's top may be relative
            for (let k = 0; k < len; k++) { const y = top + k; if (y >= 0 && y < h) { out.px[y * w + x] = p[q + 3 + k]; out.op[y * w + x] = 1; } }
            q += len + 4;
          }
        }
      }
      patches.set(name, out);
      return out;
    }
    const flats = new Map();
    function flat(name) {
      name = name.toUpperCase();
      if (!flats.has(name)) { const f = lump(name); flats.set(name, f && f.length >= 4096 ? f.subarray(0, 4096) : null); }
      return flats.get(name);
    }
    // wall textures: TEXTURE1 (and TEXTURE2, Doom 1's) over PNAMES
    let texDefs = null;
    function defs() {
      if (texDefs) return texDefs;
      texDefs = new Map();
      const pn = lump("PNAMES");
      if (!pn) return texDefs;
      const pv = view(pn), names = [];
      for (let i = 0; i < pv.getInt32(0, true); i++) names.push(text(pn, 4 + 8 * i, 8));
      for (const tl of ["TEXTURE1", "TEXTURE2"]) {
        const t = lump(tl);
        if (!t) continue;
        const tv = view(t);
        for (let i = 0; i < tv.getInt32(0, true); i++) {
          const o = tv.getInt32(4 + 4 * i, true), name = text(t, o, 8);
          if (texDefs.has(name)) continue;
          const w = tv.getInt16(o + 12, true), h = tv.getInt16(o + 14, true), np = tv.getInt16(o + 20, true), ps = [];
          for (let k = 0; k < np; k++) ps.push({ x: tv.getInt16(o + 22 + 10 * k, true), y: tv.getInt16(o + 24 + 10 * k, true), name: names[tv.getInt16(o + 26 + 10 * k, true)] });
          texDefs.set(name, { w, h, ps });
        }
      }
      return texDefs;
    }
    const textures = new Map();
    function texture(name) {
      name = (name || "").toUpperCase();
      if (textures.has(name)) return textures.get(name);
      const def = defs().get(name);
      let out = null;
      if (def) {
        out = { w: def.w, h: def.h, px: new Uint8Array(def.w * def.h), op: new Uint8Array(def.w * def.h), masked: false };
        for (const p of def.ps) {
          const pt = patch(p.name);
          if (!pt) continue;
          for (let y = 0; y < pt.h; y++) for (let x = 0; x < pt.w; x++) {
            const tx = p.x + x, ty = p.y + y, i = y * pt.w + x;
            if (!pt.op[i] || tx < 0 || ty < 0 || tx >= def.w || ty >= def.h) continue;
            out.px[ty * def.w + tx] = pt.px[i]; out.op[ty * def.w + tx] = 1;
          }
        }
        for (let i = 0; i < out.op.length; i++) if (!out.op[i]) { out.masked = true; break; }
      }
      textures.set(name, out);
      return out;
    }
    // sprites: frames by letter, each with eight rotations (0 = the same from every side), some mirrored
    let spriteTable = null;
    function sprites() {
      if (spriteTable) return spriteTable;
      spriteTable = new Map();
      const add = (name, frame, rot, lname, flip) => {
        let s = spriteTable.get(name);
        if (!s) spriteTable.set(name, (s = []));
        const f = frame.charCodeAt(0) - 65;
        s[f] = s[f] || new Array(9);
        if (rot === "0") for (let r = 1; r <= 8; r++) s[f][r] = s[f][r] || { lump: lname, flip: false, all: true };
        else s[f][+rot] = { lump: lname, flip };
      };
      let on = false;
      for (const l of dir) {
        if (l.name === "S_START" || l.name === "SS_START") { on = true; continue; }
        if (l.name === "S_END" || l.name === "SS_END") { on = false; continue; }
        if (!on || !l.size || l.name.length < 6) continue;
        add(l.name.slice(0, 4), l.name[4], l.name[5], l.name, false);
        if (l.name.length >= 8) add(l.name.slice(0, 4), l.name[6], l.name[7], l.name, true);
      }
      return spriteTable;
    }
    // a sprite's picture for a frame, seen from rotation 1..8 (1 = from in front)
    function spriteFrame(name, frame, rot) {
      const s = sprites().get(name), f = s && s[frame];
      if (!f) return null;
      const e = f[rot] || f[1];
      if (!e) return null;
      return { pic: patch(e.lump), flip: e.flip };
    }

    // ---- maps
    function map(name) {
      const i = by.get(name.toUpperCase());
      if (i === undefined) return null;
      const L = {};
      for (let k = 1; k <= 10 && i + k < dir.length; k++) L[dir[i + k].name] = bytes(dir[i + k]);
      const rows = (u8, size, fn) => { const v = view(u8), out = []; for (let o = 0; o + size <= u8.length; o += size) out.push(fn(v, o, u8)); return out; };
      const vertexes = rows(L.VERTEXES, 4, (v, o) => ({ x: v.getInt16(o, true), y: v.getInt16(o + 2, true) }));
      const sectors = rows(L.SECTORS, 26, (v, o, u) => ({ floor: v.getInt16(o, true), ceil: v.getInt16(o + 2, true), ftex: text(u, o + 4, 8), ctex: text(u, o + 12, 8),
        light: v.getInt16(o + 20, true), special: v.getInt16(o + 22, true), tag: v.getInt16(o + 24, true) }));
      const sides = rows(L.SIDEDEFS, 30, (v, o, u) => ({ xo: v.getInt16(o, true), yo: v.getInt16(o + 2, true), upper: text(u, o + 4, 8), lower: text(u, o + 12, 8), mid: text(u, o + 20, 8), sector: sectors[v.getInt16(o + 28, true)] }));
      const lines = rows(L.LINEDEFS, 14, (v, o) => {
        const s1 = v.getInt16(o + 10, true), s2 = v.getInt16(o + 12, true);
        return { v1: vertexes[v.getUint16(o, true)], v2: vertexes[v.getUint16(o + 2, true)], flags: v.getInt16(o + 4, true), special: v.getInt16(o + 6, true), tag: v.getInt16(o + 8, true),
          front: s1 >= 0 ? sides[s1] : null, back: s2 >= 0 ? sides[s2] : null };
      });
      const segs = rows(L.SEGS, 12, (v, o) => {
        const line = lines[v.getInt16(o + 6, true)], dirn = v.getInt16(o + 8, true);
        return { v1: vertexes[v.getUint16(o, true)], v2: vertexes[v.getUint16(o + 2, true)], angle: v.getInt16(o + 4, true), line, side: dirn,
          offset: v.getInt16(o + 10, true), front: dirn ? line.back : line.front, back: dirn ? line.front : line.back };
      });
      const ssectors = rows(L.SSECTORS, 4, (v, o) => ({ n: v.getInt16(o, true), first: v.getInt16(o + 2, true) }));
      for (const ss of ssectors) ss.sector = segs[ss.first].front.sector;
      const nodes = rows(L.NODES, 28, (v, o) => {
        const g = (k) => v.getInt16(o + k, true);
        return { x: g(0), y: g(2), dx: g(4), dy: g(6), bbox: [[g(8), g(10), g(12), g(14)], [g(16), g(18), g(20), g(22)]], kids: [v.getUint16(o + 24, true), v.getUint16(o + 26, true)] };
      });
      const things = rows(L.THINGS, 10, (v, o) => ({ x: v.getInt16(o, true), y: v.getInt16(o + 2, true), angle: v.getInt16(o + 4, true), type: v.getInt16(o + 6, true), flags: v.getInt16(o + 8, true) }));
      return { name: name.toUpperCase(), vertexes, sectors, sides, lines, segs, ssectors, nodes, things };
    }

    return { lump, has: (n) => by.has(n.toUpperCase()), rgb, cmap: CM, nearest, quantizer, patch, flat, texture, spriteFrame, sprites, map,
      flatNames: () => between("F_START", "F_END").map((l) => l.name), textureNames: () => [...defs().keys()] };
  }

  const cache = new Map();
  return { load: (url) => { if (!cache.has(url)) cache.set(url, fetch(url).then((r) => r.arrayBuffer()).then(open)); return cache.get(url); }, open };
})();
