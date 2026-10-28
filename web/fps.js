// The gun closet, hardcore: the office in the first person, drawn by a small Doom engine, and through the mysterious
// door in its south wall, Doom's first level, in the same engine and the same space: the door is in a wall of the
// level too, and open, each side is seen through it and you walk through.
//
// It starts from Doom and works towards burrow. Doom's: the screen (320 x 200, shown at 4:3, as its pixels were a
// fifth taller than wide; drawn at twice that, Crispy Doom's high resolution: S), the palette and its colormaps (the
// light comes in Doom's bands and sinks into the dark with distance: its scalelight and zlight tables), the projection
// (90 degrees), the tics (35 a second), the movement (thrust, friction, the bob), the shotgun (its frames and their
// tics, the flash that lights the room), the status bar, the melt between screens, and a level drawn from its BSP
// (walls with their upper, lower and middle textures, flats, the sky, sprites from the side that faces you, a
// spectre's fuzz). Its data comes from a WAD (doom/wad.js): Freedoom's (bundled, BSD) or id's own when one is in
// ~/.config/burrow/ (server.py's /doom.wad). A level's game is doom/level.js; its player is you in the office as well
// (health, armour, ammo, keys: the status bar is the game's).
// Burrow's: the office (its desks, furniture, bots, what hangs on its wall, brought into Doom's palette and light);
// shooting a bot deletes its session (the room's own shoot: the remains, the sweepers, the undo with E); the face on
// the status bar is your avatar, in Doom's states (bloodier as your health goes, a grin after a kill, dead); the hands
// on the shotgun have your skin.
//
// The mysterious door is only here, in the first person: the room seen from above has none. It is a Doom door: use it
// or walk into it and it rises, stays up a while, comes down again (not on you). In the level it is in the wall behind
// where you would start (DoomLevel's doorSpot).
//
// startFps(room, done): room = { scene(), start(), move(x, y, a, walking), shoot(id), unshoot(id), alarm(x, y),
// armed(), use(thing), label(thing), status(), face(), tiny(ctx, s, x, y, col), tinyW(s), snapshot() }. scene() is
// the room at its last tick:
//   { v, W, H, WALL, floor: the floor's overlay (2 texels a dot), wall: the far wall's picture (TS texels a dot),
//     items: what hangs on that wall, wkey, boxes, things, solid, lights }  (in the room's dots; here they are units)
function startFps(room, done) {
  const S = 2, SW = 320 * S, SH = 200 * S, VH = 168 * S, CX = SW / 2, CY = VH / 2, PROJ = 160 * S, U = 2.25, TICMS = 1000 / 35, WALLH = 40;
  const TS = Props3D.TS, GLOW = Props3D.GLOW;
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;

  // ---- the screen: black all round, Doom's 320 x 200 (at S) in the middle at 4:3; over it the melt
  const wrap = document.createElement("div");
  wrap.id = "fps"; wrap.tabIndex = -1;
  const cv = document.createElement("canvas"); cv.width = SW; cv.height = SH;
  const mc = document.createElement("canvas"); mc.className = "melt";
  wrap.append(cv, mc);
  document.body.append(wrap);
  if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
  const ctx = cv.getContext("2d"), mx = mc.getContext("2d");
  const img = ctx.createImageData(SW, SH), out32 = new Uint32Array(img.data.buffer);
  function fit() {
    const k = Math.min(innerWidth / 4, innerHeight / 3);
    cv.style.width = Math.floor(k * 4) + "px"; cv.style.height = Math.floor(k * 3) + "px";
    mc.width = innerWidth; mc.height = innerHeight;
  }
  fit();
  addEventListener("resize", fit);

  const fb = new Uint8Array(SW * SH), zb = new Float32Array(SW * VH), idb = new Int32Array(SW * VH);
  const ceilClip = new Int16Array(SW), floorClip = new Int16Array(SW), solid = new Uint8Array(SW);
  const kx = new Float32Array(SW);
  for (let x = 0; x < SW; x++) kx[x] = (x + 0.5 - CX) / PROJ;
  // the door's window in this frame: per column the rows between top and bot (both out) where the other side shows, and how far the door is there
  const wTop = new Int16Array(SW), wBot = new Int16Array(SW), wZ = new Float32Array(SW);
  let wad = null, cmap = null, extralight = 0, world = "office", lvl = null, P = null, levelName = null;

  // ---- Doom's light: a sector's level (0..255) >> 4, plus the gun's flash; then by distance, its tables
  const clampL = (n) => (n < 0 ? 0 : n > 15 ? 15 : n);
  const wallLevel = (ln, z) => { const l = (15 - clampL(ln)) * 4 - (Math.min(47, Math.floor(2560 / Math.max(z, 1))) >> 1); return l < 0 ? 0 : l > 31 ? 31 : l; };
  const planeLevel = (ln, z) => { const l = (15 - clampL(ln)) * 4 - Math.floor(80 / (Math.min(127, Math.floor(z / 16)) + 1)); return l < 0 ? 0 : l > 31 ? 31 : l; };
  const spriteLevel = (ln, z) => wallLevel(ln, z);

  // ---- burrow's pictures in Doom's palette: opaque or not, fullbright where they shone (alpha GLOW), kept per canvas version
  const quant = new WeakMap();
  function pic(c, sx = 0, sy = 0, sw = c.width, sh = c.height) {
    let e = quant.get(c);
    if (!e || e.v !== c.v) { e = { v: c.v, m: new Map() }; quant.set(c, e); }
    const key = sx + "," + sy + "," + sw + "," + sh;
    let p = e.m.get(key);
    if (p) return p;
    const w = Math.max(1, Math.round(sw)), h = Math.max(1, Math.round(sh));
    const d = c.getContext("2d", { willReadFrequently: true }).getImageData(Math.round(sx), Math.round(sy), w, h).data;
    p = { w, h, px: new Uint8Array(w * h), op: new Uint8Array(w * h), lo: 0, to: 0 };
    for (let i = 0; i < w * h; i++) {
      const a = d[i * 4 + 3];
      if (a < 128) continue;
      p.px[i] = wad.nearest(d[i * 4], d[i * 4 + 1], d[i * 4 + 2]);
      p.op[i] = a === GLOW ? 2 : 1;
    }
    e.m.set(key, p);
    return p;
  }

  // ---- what is drawn over the view: a WAD picture at x, y (screen pixels; by its offsets), k screen pixels a texel
  function blit(p, x, y, trans, k = 1) {
    if (!p) return;
    const x0 = x - p.lo * k, y0 = y - p.to * k;
    for (let j = 0; j < p.h * k; j++) {
      const yy = y0 + j;
      if (yy < 0 || yy >= SH) continue;
      const row = Math.floor(j / k) * p.w;
      for (let i = 0; i < p.w * k; i++) {
        const xx = x0 + i, q = row + Math.floor(i / k);
        if (xx < 0 || xx >= SW || !p.op[q]) continue;
        fb[yy * SW + xx] = trans ? trans[p.px[q]] : p.px[q];
      }
    }
  }
  const PT = (name) => wad.patch(name);
  // the room's 3x5 letters into the screen, one palette colour, k pixels a letter pixel
  const letters = new Map();
  function words(text, x, y, idx, k = S) {
    let m = letters.get(text);
    if (!m) {
      const w = Math.max(1, room.tinyW(text)), c = document.createElement("canvas"); c.width = w; c.height = 5;
      const g = c.getContext("2d"); room.tiny(g, text, 0, 0, "#ffffff");
      const d = g.getImageData(0, 0, w, 5).data;
      m = { w, a: Uint8Array.from({ length: w * 5 }, (_, i) => d[i * 4 + 3] > 100 ? 1 : 0) };
      letters.set(text, m);
    }
    for (let j = 0; j < 5 * k; j++) for (let i = 0; i < m.w * k; i++) {
      if (!m.a[Math.floor(j / k) * m.w + Math.floor(i / k)]) continue;
      const xx = x + i, yy = y + j;
      if (xx >= 0 && xx < SW && yy >= 0 && yy < SH) fb[yy * SW + xx] = idx;
    }
    return m.w * k;
  }

  // ---- sprites: a picture standing in the world, drawn from far to near, behind what is nearer (zb); seen through
  // the door, only within its window (clipTop, clipBot)
  let vis = [];
  function drawVis(v) {
    const p = v.p, s = v.scale;
    if (!p || s <= 0) return;
    const left = v.sx - p.lo * s, top = v.sy;   // a mirrored rotation keeps its offset (Doom's R_ProjectSprite)
    const x0 = Math.max(0, Math.floor(left)), x1 = Math.min(SW - 1, Math.ceil(left + p.w * s) - 1);
    const y0 = Math.max(0, Math.floor(top)), y1 = Math.min(VH - 1, Math.ceil(top + p.h * s) - 1);
    if (x0 > x1 || y0 > y1) return;
    const base = v.level * 256, inv = 1 / s;
    for (let x = x0; x <= x1; x++) {
      let u = Math.floor((x + 0.5 - left) * inv);
      if (u < 0 || u >= p.w) continue;
      if (v.flip) u = p.w - 1 - u;
      for (let y = y0; y <= y1; y++) {
        const i = y * SW + x;
        if (v.z >= zb[i] || (v.clipTop && (y <= v.clipTop[x] || y >= v.clipBot[x]))) continue;
        const t = Math.floor((y + 0.5 - top) * inv);
        if (t < 0 || t >= p.h) continue;
        const k = t * p.w + u, o = p.op[k];
        if (!o) continue;
        if (v.fuzz) { const j = Math.min(VH - 1, Math.max(0, y + (((x * 7 + y * 3 + frameNo) % 3) - 1) * S)) * SW + x; fb[i] = cmap[6 * 256 + fb[j]]; }
        else fb[i] = o === 2 || v.bright ? p.px[k] : cmap[base + p.px[k]];
        if (v.id) idb[i] = v.id;
      }
    }
  }

  // ---- the office, in units (a dot of the room is U units): the room's walls (Doom's STARTAN; the far one with
  // the doors, the window, the clock, the closet, the switchboard; the south one with the mysterious door), the
  // floor (Doom's wooden FLAT5_2, under burrow's overlay: the rug, the mats, the oil, the pit, the shadows), a ceiling
  // of CEIL3_5 with a lamp (TLITE6_4) every third tile, Doom's sector lights: brighter under a lamp, flickering near
  // the fire
  let sc = room.scene(), seen = new Map(), sv = -1, drawn = [];
  const units = () => ({ W: sc.W * U, H: sc.H * U, WALL: sc.WALL * U });
  const DOORW = 64, DOORH = 72, CEIL = WALLH * U;   // the mysterious door: Doom's 64 x 72, in the south wall, left of the middle (the printer's default spot)
  const doorX = () => Math.round(sc.W * 0.3) * U;
  // the office's light in 32-unit cells, as Doom's sectors would have it, once a frame: 192, 224 under a lamp, the fire's flicker
  let lgrid = new Uint8Array(1), lgw = 1, lgh = 1, fireFlick = 32;
  function lightGrid() {
    const { W, H } = units();
    lgw = Math.ceil(W / 32) + 1; lgh = Math.ceil(H / 32) + 1;
    if (lgrid.length !== lgw * lgh) lgrid = new Uint8Array(lgw * lgh);
    for (let cy = 0; cy < lgh; cy++) for (let cx = 0; cx < lgw; cx++) {
      const x = cx * 32 + 16, y = cy * 32 + 16;
      let l = Math.floor(x / 64) % 3 === 1 && Math.floor(y / 64) % 3 === 1 ? 224 : 192;
      for (const L of sc.lights || []) {
        const dx = x - L.x * U, dy = y - L.y * U;
        if (dx * dx + dy * dy < 9000) l = Math.max(l, L.flicker && !still ? 192 + (fireFlick & 48) : 224);
      }
      lgrid[cy * lgw + cx] = l;
    }
  }
  const officeLight = (x, y) => lgrid[Math.max(0, Math.min(lgh - 1, Math.floor(y / 32))) * lgw + Math.max(0, Math.min(lgw - 1, Math.floor(x / 32)))];
  // the far wall: STARTAN2 along it, then what hangs on it, each where the room has it (its doors Doom's DOOR3 with
  // its machine's key colour beside it, blue, yellow, red; the rest burrow's own pictures), kept until it changes
  let farTex = null, farKey = "";
  function farWall() {
    if (farTex && farKey === sc.wkey + "|" + sc.W) return farTex;
    farKey = sc.wkey + "|" + sc.W;
    const w = Math.round(sc.W * U), h = Math.round(CEIL), base = wad.texture("STARTAN2") || wad.texture("STARTAN3");
    const t = { w, h, px: new Uint8Array(w * h), op: new Uint8Array(w * h).fill(1) };
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) t.px[y * w + x] = base ? base.px[(y % base.h) * base.w + (x % base.w)] : 96;
    const zOf = (yb) => (sc.WALL - yb) * (WALLH / sc.WALL);
    const put = (src, x0, y0, ww, hh, fn) => {      // a picture into a rectangle of the wall (in units), stretched
      for (let y = 0; y < hh; y++) for (let x = 0; x < ww; x++) {
        const tx = Math.floor(x0 + x), ty = Math.floor(y0 + y);
        if (tx < 0 || ty < 0 || tx >= w || ty >= h) continue;
        const sx = Math.floor(x / ww * src.w), sy = Math.floor(y / hh * src.h), k = sy * src.w + sx;
        if (!src.op[k]) continue;
        t.px[ty * w + tx] = fn ? fn(src.px[k]) : src.px[k];
      }
    };
    const strips = ["DOORBLU", "DOORYEL", "DOORRED"];
    let n = 0;
    for (const it of sc.items || []) {
      const zt = zOf(it.y0), zb0 = Math.max(0, zOf(it.y1)), x0 = it.x0 * U, x1 = it.x1 * U, y0 = (WALLH - zt) * U, hh = (zt - zb0) * U;
      if (it.kind === "door") {
        const dr = wad.texture("DOOR3"), trak = wad.texture("DOORTRAK"), strip = wad.texture(strips[n++] || "DOORTRAK");
        if (trak) { put(trak, x0 - 8, y0, 8, hh); put(trak, x1, y0, 8, hh); }
        if (strip && n <= 3) { put(strip, x0 - 8, y0, 4, hh); put(strip, x1 + 4, y0, 4, hh); }
        if (dr) put(dr, x0, y0, x1 - x0, hh, it.ok ? null : (c) => cmap[20 * 256 + c]);   // out of reach: dark
      } else {
        const p = pic(sc.wall, it.x0 * TS, (WALLH - zt) * TS, (it.x1 - it.x0) * TS, (zt - zb0) * TS);
        put(p, x0, y0, x1 - x0, hh);
      }
    }
    return (farTex = t);
  }
  // the floor's overlay in the palette: opaque, fullbright, or a shadow (how many light levels darker)
  let ovl = null, ovlV = -1;
  function overlay() {
    const c = sc.floor;
    if (ovl && ovlV === c.v) return ovl;
    ovlV = c.v;
    const d = c.getContext("2d", { willReadFrequently: true }).getImageData(0, 0, c.width, c.height).data, n = c.width * c.height;
    ovl = { w: c.width, h: c.height, px: new Uint8Array(n), m: new Uint8Array(n) };
    for (let i = 0; i < n; i++) {
      const a = d[i * 4 + 3];
      if (!a) continue;
      if (a >= 250) { ovl.px[i] = wad.nearest(d[i * 4], d[i * 4 + 1], d[i * 4 + 2]); ovl.m[i] = a === GLOW ? 2 : 1; }
      else ovl.m[i] = 3 + Math.round(a / 255 * 10);   // a shadow: so many levels darker
    }
    return ovl;
  }

  // you, in the office
  const st0 = room.start();
  const me = { x: st0.x * U, y: st0.y * U, a: st0.a, mx: 0, my: 0, bob: 0 };
  const R = 12, PR = 16;                      // you against the office's furniture; in the doorway, Doom's player radius
  const officeBlocked = (x, y) => {
    const { W, H, WALL } = units();
    if (x < R || x > W - R || y < WALL + R) return true;
    if (y > H - R && !(P && door.open >= 56 && Math.abs(x - doorX()) <= DOORW / 2 - PR)) return true;   // the south wall, but the open door
    return sc.solid.some((r) => x > r[0] * U - R && x < r[2] * U + R && y > r[1] * U - R && y < r[3] * U + R);
  };

  // ---- the mysterious door: one door, in the office's south wall and in a wall of the level; how far it stands open
  // (Doom's: 2 units a tic up, 150 tics open, down again, back up if you are under it)
  const door = { open: 0, dir: 0, wait: 0 };
  function bump() {
    if (door.dir === 1) return true;
    if (door.open >= DOORH) { door.wait = 150; return true; }
    door.dir = 1; door.wait = 0;
    return true;
  }
  const inDoorway = () => (world === "office" ? me.y > units().H - PR - 2 : !!P && Math.abs(lvl.beyond(lvl.player.x, lvl.player.y)) < PR + 2);
  function doorTic() {
    if (door.dir === 1) { door.open = Math.min(DOORH, door.open + 2); if (door.open >= DOORH) { door.dir = 0; door.wait = 150; } }
    else if (door.wait > 0) { if (--door.wait === 0) door.dir = -1; }
    else if (door.dir === -1) { if (inDoorway()) door.dir = 1; else { door.open = Math.max(0, door.open - 2); if (!door.open) door.dir = 0; } }
  }
  // the office and the level, one space through the door: the office's south (+y) is the level's way in from the
  // door (n), the office's x along the door is the level's t (n turned left)
  const o2l = (ox, oy) => { const b = oy - units().H, s = ox - doorX(); return [P.cx + P.nx * b + P.tx * s, P.cy + P.ny * b + P.ty * s]; };
  const l2o = (lx, ly) => { const rx = lx - P.cx, ry = ly - P.cy; return [doorX() + rx * P.tx + ry * P.ty, units().H + rx * P.nx + ry * P.ny]; };
  const a2l = (a) => Math.atan2(P.ny * Math.sin(a) + P.ty * Math.cos(a), P.nx * Math.sin(a) + P.tx * Math.cos(a));
  const a2o = (al) => { const c = Math.cos(al), s = Math.sin(al); return Math.atan2(c * P.nx + s * P.ny, c * P.tx + s * P.ty); };
  function setPortal() {
    const spot = lvl.doorSpot(DOORW, DOORH);
    P = lvl.portal = spot && { ...spot, tx: -spot.ny, ty: spot.nx, w: DOORW, h: DOORH, height: CEIL, open: () => door.open, bump,
      inside: (x, y) => { const [ox, oy] = l2o(x, y), { W, H, WALL } = units(); return ox > 0 && ox < W && oy > WALL && oy < H; } };
  }
  const winOf = () => {                        // the window's bounds, or null if the door shows nothing through it
    let x0 = SW, x1 = -1, y0 = VH, y1 = -1;
    for (let x = 0; x < SW; x++) if (wBot[x] - wTop[x] > 1) { x0 = Math.min(x0, x); x1 = x; y0 = Math.min(y0, wTop[x] + 1); y1 = Math.max(y1, wBot[x] - 1); }
    return x1 < 0 ? null : { top: wTop, bot: wBot, x0, x1, y0, y1 };
  };
  const winZ = (w, z) => { for (let x = w.x0; x <= w.x1; x++) for (let y = w.top[x] + 1; y < w.bot[x]; y++) zb[y * SW + x] = z ? z[x] : 1e9; };

  // ---- the office from cam = {x, y, eye, a} (units); within win only, seen through the door from the level
  function renderOffice(now, cam, win) {
    const s = room.scene();
    if (s !== sc) sc = s;
    const { W, H, WALL } = units(), eye = cam.eye, top = CEIL;
    const dx = Math.cos(cam.a), dy = Math.sin(cam.a), rx = -dy, ry = dx, px = cam.x, py = cam.y;
    const FL = wad.flat("FLAT5_2") || wad.flat("FLOOR4_8"), CE = wad.flat("CEIL3_5"), LAMP = wad.flat("TLITE6_4") || CE, OV = overlay();
    const xs = win ? win.x0 : 0, xe = win ? win.x1 : SW - 1;
    // floor and ceiling, a row at a time
    for (let y = win ? win.y0 : 0, ye = win ? win.y1 : VH - 1; y <= ye; y++) {
      const below = y + 0.5 - CY, isFloor = below > 0, d = Math.min(4000, (isFloor ? eye : top - eye) * PROJ / Math.abs(below));
      for (let x = xs; x <= xe; x++) {
        if (win && (y <= win.top[x] || y >= win.bot[x])) continue;
        const i = y * SW + x, wx = px + (dx + rx * kx[x]) * d, wy = py + (dy + ry * kx[x]) * d, ln = (officeLight(wx, wy) >> 4) + extralight;
        let c, lv = planeLevel(ln, d);
        if (isFloor) {
          c = FL ? FL[((Math.floor(wy) & 63) << 6) + (Math.floor(wx) & 63)] : 96;
          const ox = Math.floor(wx / U * 2), oy = Math.floor(wy / U * 2);
          if (ox >= 0 && oy >= 0 && ox < OV.w && oy < OV.h) {
            const k = oy * OV.w + ox, m = OV.m[k];
            if (m === 1) c = OV.px[k]; else if (m === 2) { c = OV.px[k]; lv = 0; } else if (m >= 3) lv = Math.min(31, lv + m - 3);
          }
        } else {
          const lamp = Math.floor(wx / 64) % 3 === 1 && Math.floor(wy / 64) % 3 === 1, F = lamp ? LAMP : CE;
          c = F ? F[((Math.floor(wy) & 63) << 6) + (Math.floor(wx) & 63)] : 4;
        }
        fb[i] = cmap[lv * 256 + c]; zb[i] = d; idb[i] = 0;
      }
    }
    // the four walls; in the south one the door, risen door.open into the lintel, the level under it
    const far = farWall(), side = wad.texture("STARTAN3") || far, dt = wad.texture("BIGDOOR7") || wad.texture("DOOR3");
    if (!win) { wTop.fill(VH); wBot.fill(-1); }
    for (let x = xs; x <= xe; x++) {
      const ylo = win ? win.top[x] + 1 : 0, yhi = win ? win.bot[x] - 1 : VH - 1;
      if (ylo > yhi) continue;
      const rdx = dx + rx * kx[x], rdy = dy + ry * kx[x];
      const tx = rdx > 0 ? (W - px) / rdx : rdx < 0 ? -px / rdx : 1e9, ty = rdy > 0 ? (H - py) / rdy : rdy < 0 ? (WALL - py) / rdy : 1e9;
      let t, u, T, fake, hx, hy, isDoor = false;
      if (ty < tx) { t = ty; hx = px + rdx * t; hy = rdy < 0 ? WALL : H; u = hx; T = rdy < 0 ? far : side; fake = -1;
        if (!win && rdy > 0 && dt && P && Math.abs(hx - doorX()) < DOORW / 2) isDoor = true; }
      else { t = tx; hx = rdx > 0 ? W : 0; hy = py + rdy * t; u = hy; T = side; fake = 1; }
      if (t <= 0) continue;
      const ln = (officeLight(hx - Math.sign(hx - W / 2) * 4, hy - Math.sign(hy - (WALL + H) / 2) * 4) >> 4) + extralight + fake, base = wallLevel(ln, t) * 256;
      const yt = CY - (top - eye) * PROJ / t, yb = CY + eye * PROJ / t;
      const col = T === far ? Math.max(0, Math.min(T.w - 1, Math.floor(u))) : ((Math.floor(u) % T.w) + T.w) % T.w;
      let wy0 = VH, wy1 = -1;
      for (let y = Math.max(ylo, Math.ceil(yt - 0.5)), y1 = Math.min(yhi, Math.floor(yb - 0.5)); y <= y1; y++) {
        const z = eye - (y + 0.5 - CY) * t / PROJ, i = y * SW + x;
        let c;
        if (isDoor && z < DOORH) {
          if (z < door.open) { if (y < wy0) wy0 = y; wy1 = y; continue; }    // open: the level shows here
          const du = Math.floor((hx - (doorX() - DOORW / 2)) / DOORW * dt.w), dv = Math.floor((DOORH - (z - door.open)) / DOORH * dt.h);
          c = dt.px[Math.min(dt.h - 1, Math.max(0, dv)) * dt.w + Math.min(dt.w - 1, Math.max(0, dt.w - 1 - du))];
        } else c = T.px[((((Math.floor(top - z)) % T.h) + T.h) % T.h) * T.w + col];
        fb[i] = cmap[base + c]; zb[i] = t; idb[i] = 0;
      }
      if (wy1 >= 0) { wTop[x] = wy0 - 1; wBot[x] = wy1 + 1; wZ[x] = t; }
    }
    // through the open door: the level, as it is from here
    let w2 = null;
    if (!win && P && door.open > 0 && (w2 = winOf())) {
      const [lx, ly] = o2l(px, py);
      winZ(w2);
      renderLevel(now, { x: lx, y: ly, z: P.floor + eye, a: a2l(cam.a) }, w2);
      winZ(w2, wZ);
    }
    aimThrough = !win && wTop[CX] < CY && CY < wBot[CX];
    // the blocks (desks and what stands as one): the face a ray meets first, and their tops seen from above
    for (const b of sc.boxes) officeBox(b, px, py, dx, dy, rx, ry, eye, win);
    // the pictures and the models; who walks is on the way from one tick's place to the next
    if (sc.v !== sv) {
      sv = sc.v;
      const next = new Map();
      for (const th of sc.things) if (th.key) {
        const o = seen.get(th.key), at = o ? place(o, now) : th;
        next.set(th.key, Math.hypot(at.x - th.x, at.y - th.y) > 30 ? { fx: th.x, fy: th.y, fz: th.z, tx: th.x, ty: th.y, tz: th.z, t0: now } : { fx: at.x, fy: at.y, fz: at.z, tx: th.x, ty: th.y, tz: th.z, t0: now });
      }
      seen = next;
    }
    drawn = []; vis = [];
    for (const th of sc.things) {
      const o = th.key && seen.get(th.key), at = o ? place(o, now) : th;
      drawn.push(th);
      officeThing(th, at, drawn.length, px, py, dx, dy, rx, ry, eye, o, win);
    }
    for (const f of puffs) puffVis(f, px, py, dx, dy, rx, ry, eye, (x, y) => (officeLight(x, y) >> 4) + extralight, win);
    // the level's fireballs that came through the door
    if (lvl) for (const o of lvl.mobjs) if (o.office) {
      const [ox, oy] = l2o(o.x, o.y), vx = ox - px, vy = oy - py, z = vx * dx + vy * dy, sf = z > 4 && wad.spriteFrame(o.sprite, o.frame.charCodeAt(0) - 65, 1);
      if (sf && sf.pic) vis.push({ p: sf.pic, flip: sf.flip, sx: CX + (vx * rx + vy * ry) * PROJ / z, sy: CY - (o.z - P.floor + sf.pic.to - eye) * PROJ / z, scale: PROJ / z, z, bright: true, level: 0, clipTop: win && win.top, clipBot: win && win.bot });
    }
    vis.sort((a, b) => b.z - a.z);
    for (const v of vis) drawVis(v);
  }
  const place = (o, now) => { const k = Math.min(1, (now - o.t0) / (1000 / 12)); return { x: o.fx + (o.tx - o.fx) * k, y: o.fy + (o.ty - o.fy) * k, z: o.fz + (o.tz - o.fz) * k }; };
  function officeBox(b, px, py, dx, dy, rx, ry, eye, win) {
    const x0 = b.x0 * U, x1 = b.x1 * U, y0 = b.y0 * U, y1 = b.y1 * U, z0 = b.z0 * U, z1 = b.z1 * U;
    if (px > x0 && px < x1 && py > y0 && py < y1) return;
    let c0 = SW, c1 = -1;
    for (const [cx, cy] of [[x0, y0], [x1, y0], [x0, y1], [x1, y1]]) {
      const vx = cx - px, vy = cy - py, d = vx * dx + vy * dy;
      if (d < 0.5) { c0 = 0; c1 = SW - 1; break; }
      const sx = CX + (vx * rx + vy * ry) * PROJ / d;
      c0 = Math.min(c0, Math.floor(sx)); c1 = Math.max(c1, Math.ceil(sx));
    }
    c0 = Math.max(win ? win.x0 : 0, c0); c1 = Math.min(win ? win.x1 : SW - 1, c1);
    const facePic = (f) => f && pic(f.c, f.sx, f.sy, f.sw, f.sh);
    const F = { front: facePic(b.front), back: facePic(b.back), side: facePic(b.side), west: facePic(b.west), east: facePic(b.east), top: facePic(b.top) };
    for (let x = c0; x <= c1; x++) {
      const ylo = win ? win.top[x] + 1 : 0, yhi = win ? win.bot[x] - 1 : VH - 1;
      if (ylo > yhi) continue;
      const rdx = dx + rx * kx[x], rdy = dy + ry * kx[x];
      let tn = -Infinity, tx = Infinity, ax = 0;
      if (rdx) { const a1 = (x0 - px) / rdx, a2 = (x1 - px) / rdx; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 1; } tx = Math.min(tx, Math.max(a1, a2)); } else if (px < x0 || px > x1) continue;
      if (rdy) { const a1 = (y0 - py) / rdy, a2 = (y1 - py) / rdy; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 2; } tx = Math.min(tx, Math.max(a1, a2)); } else if (py < y0 || py > y1) continue;
      if (tx < tn || tn <= 1) continue;
      const hx = px + rdx * tn, hy = py + rdy * tn;
      let p, u, glow, fake;
      if (ax === 1) { p = (rdx > 0 ? F.west : F.east) || F.side; glow = b.side && b.side.glow; u = (hy - y0) / (y1 - y0); fake = 1; if (rdx < 0) u = 1 - u; }
      else if (rdy < 0) { p = F.front; glow = b.front && b.front.glow; u = (hx - x0) / (x1 - x0); fake = -1; }
      else { p = F.back; glow = b.back && b.back.glow; u = 1 - (hx - x0) / (x1 - x0); fake = -1; }
      u = Math.max(0, Math.min(0.999, u));
      const ln = (officeLight(hx, hy) >> 4) + extralight + fake, base = wallLevel(ln, tn) * 256;
      const yt = CY - (z1 - eye) * PROJ / tn, yb = CY - (z0 - eye) * PROJ / tn;
      if (p) for (let y = Math.max(ylo, Math.ceil(yt - 0.5)), ye = Math.min(yhi, Math.floor(yb - 0.5)); y <= ye; y++) {
        const i = y * SW + x;
        if (tn >= zb[i]) continue;
        const z = eye - (y + 0.5 - CY) * tn / PROJ, k = Math.min(p.h - 1, Math.max(0, Math.floor((z1 - z) / (z1 - z0) * p.h))) * p.w + Math.floor(u * p.w), o = p.op[k];
        if (!o) continue;
        fb[i] = glow || o === 2 ? p.px[k] : cmap[base + p.px[k]]; zb[i] = tn; idb[i] = 0;
      }
      if (eye <= z1 || !F.top) continue;
      const h = eye - z1, ya = Math.max(ylo, Math.ceil(CY + h * PROJ / tx - 0.5)), yb2 = Math.min(yhi, Math.floor(CY + h * PROJ / tn - 0.5)), tp = F.top;
      for (let y = ya; y <= yb2; y++) {
        const d = h * PROJ / (y + 0.5 - CY), i = y * SW + x;
        if (d < tn || d > tx || d >= zb[i]) continue;
        const wx = px + rdx * d, wy = py + rdy * d;
        const k = Math.min(tp.h - 1, Math.max(0, Math.floor((wy - y0) / (y1 - y0) * tp.h))) * tp.w + Math.min(tp.w - 1, Math.max(0, Math.floor((wx - x0) / (x1 - x0) * tp.w))), o = tp.op[k];
        if (!o) continue;
        fb[i] = b.top.glow || o === 2 ? tp.px[k] : cmap[planeLevel((officeLight(wx, wy) >> 4) + extralight, d) * 256 + tp.px[k]]; zb[i] = d; idb[i] = 0;
      }
    }
  }
  function officeThing(th, at, n, px, py, dx, dy, rx, ry, eye, o, win) {
    const X = at.x * U, Y = at.y * U, vx = X - px, vy = Y - py, z = vx * dx + vy * dy;
    if (z < 12) return;
    let c = th.c, sx0 = th.sx, sy0 = th.sy, sw = th.sw, sh = th.sh, ax = th.ax, ay = th.ay, res = th.res || 1;
    let flip = th.face && !th.fig ? th.face * -dy < 0 : !!th.flip;
    if (th.fig) {
      const mx2 = o ? o.tx - o.fx : 0, my2 = o ? o.ty - o.fy : 0;
      const fa = th.facing !== undefined ? th.facing : !th.seated && Math.hypot(mx2, my2) > 0.4 ? Math.atan2(my2, mx2) : th.face < 0 ? Math.PI : 0;
      const k = ((Math.round((Math.atan2(-vy, -vx) - fa) / (Math.PI / 4)) % 8) + 8) % 8, f = Props3D.figure(th.fig, k);
      c = f.c; sx0 = 0; sy0 = 0; sw = f.w; sh = f.h; ax = f.ax; ay = f.ay; res = f.res; flip = false;
    }
    if (!c) return;
    const p = pic(c, sx0, sy0, sw, sh), scale = PROJ / z * U / res, lat = vx * rx + vy * ry;
    const ln = (officeLight(X, Y) >> 4) + extralight;
    vis.push({ p: { ...p, lo: ax }, sx: CX + lat * PROJ / z, sy: CY - ((at.z * U) + ay * U / res - eye) * PROJ / z, scale, z, flip, level: spriteLevel(ln, z),
      id: win ? 0 : th.bot || th.rem || th.piece || th.name ? n : 0, fuzz: !!th.dissolve, clipTop: win && win.top, clipBot: win && win.bot });   // going up in the booth's light: Doom's fuzz
  }
  // a shot's puff: Doom's PUFF, rising, its four frames in sixteen tics
  let puffs = [];
  function puffVis(f, px, py, dx, dy, rx, ry, eye, lightAt, win) {
    const vx = f.x - px, vy = f.y - py, z = vx * dx + vy * dy;
    if (z < 8) return;
    const fr = f.blood ? wad.spriteFrame("BLUD", 2 - Math.min(2, f.t >> 2), 1) : wad.spriteFrame("PUFF", Math.min(3, f.t >> 2), 1);
    if (!fr || !fr.pic) return;
    const scale = PROJ / z;
    vis.push({ p: fr.pic, sx: CX + (vx * rx + vy * ry) * scale, sy: CY - (f.z + f.t * 0.5 + fr.pic.to - eye) * scale, scale, z, flip: false, bright: !f.blood && f.t < 4, level: spriteLevel(lightAt(f.x, f.y), z),
      clipTop: win && win.top, clipBot: win && win.bot });
  }

  // ---- a Doom level, from its BSP, from cam = {x, y, z, a}: front to back, a column at a time; flats and sky in what
  // each wall leaves open. Within win only, seen through the door from the office: what is beyond the door's wall
  function renderLevel(now, cam, win) {
    const L = lvl, m = L.map, V = cam, dx = Math.cos(V.a), dy = Math.sin(V.a), rx = dy, ry = -dx;
    let solidN = 0;
    if (win) { for (let x = 0; x < SW; x++) { ceilClip[x] = win.top[x]; floorClip[x] = win.bot[x]; solid[x] = floorClip[x] - ceilClip[x] <= 1 ? 1 : 0; solidN += solid[x]; } }
    else { ceilClip.fill(-1); floorClip.fill(VH); solid.fill(0); zb.fill(1e9); idb.fill(0); wTop.fill(VH); wBot.fill(-1); }
    const masked = [];
    const anim = L.anims;
    const angTo = (x, y) => Math.atan2(y - V.y, x - V.x) - V.a;
    const norm = (a) => { a %= 2 * Math.PI; return a < 0 ? a + 2 * Math.PI : a; };
    const CLIP = Math.PI / 4;
    const toX = (a) => { const s = a > Math.PI ? a - 2 * Math.PI : a; return Math.round(CX - Math.tan(s) * PROJ); };
    function clipAngles(a1, a2) {               // Doom's: the span, clipped to the view; null if not seen
      const span = norm(a1 - a2);
      if (span >= Math.PI) return null;
      let t = norm(a1 + CLIP);
      if (t > 2 * CLIP) { t -= 2 * CLIP; if (t >= span) return null; a1 = CLIP; }
      t = norm(CLIP - a2);
      if (t > 2 * CLIP) { t -= 2 * CLIP; if (t >= span) return null; a2 = norm(-CLIP); }
      return [toX(a1), toX(a2)];
    }
    const boxSeen = (b) => {                     // [top, bottom, left, right]
      const bx = V.x <= b[2] ? 0 : V.x < b[3] ? 1 : 2, by = V.y >= b[0] ? 0 : V.y > b[1] ? 1 : 2, pos = by * 4 + bx;
      if (pos === 5) return true;
      const cc = [[3, 0, 2, 1], [3, 0, 2, 0], [3, 1, 2, 0], null, [2, 0, 2, 1], null, [3, 1, 3, 0], null, [2, 0, 3, 1], [2, 1, 3, 1], [2, 1, 3, 0]][pos];
      if (!cc) return true;
      const r = clipAngles(norm(angTo(b[cc[0]], b[cc[1]])), norm(angTo(b[cc[2]], b[cc[3]])));
      if (!r) return false;
      for (let x = Math.max(0, r[0]); x < Math.min(SW, r[1]); x++) if (!solid[x]) return true;
      return false;
    };
    const skyTex = wad.texture("SKY1") || wad.texture("SKY2") || wad.texture("SKY3");
    function plane(x, y0, y1, sector, isFloor) {
      if (y0 > y1) return;
      const h = isFloor ? sector.floor : sector.ceil, pt = isFloor ? sector.ftex : sector.ctex;
      if (!isFloor && pt === "F_SKY1") {
        if (!skyTex) return;
        const col = ((Math.floor(norm(V.a + Math.atan(-kx[x])) / (2 * Math.PI) * 1024)) % skyTex.w + skyTex.w) % skyTex.w;
        for (let y = y0; y <= y1; y++) { const i = y * SW + x; fb[i] = skyTex.px[(((Math.floor(y / S) + 16) % skyTex.h) + skyTex.h) % skyTex.h * skyTex.w + col]; zb[i] = 1e8; }
        return;
      }
      const F = wad.flat(anim(pt)), dh = Math.abs(h - V.z), ln = (sector.light >> 4) + extralight;
      const rdx = dx + rx * kx[x], rdy = dy + ry * kx[x];
      for (let y = y0; y <= y1; y++) {
        const d = dh * PROJ / Math.abs(y + 0.5 - CY), wx = V.x + rdx * d, wy = V.y + rdy * d, i = y * SW + x;
        fb[i] = cmap[planeLevel(ln, d) * 256 + (F ? F[((Math.floor(-wy) & 63) << 6) + (Math.floor(wx) & 63)] : 96)]; zb[i] = d;
      }
    }
    function wallRun(x, y0, y1, tex, u, refTop, yo, z, base) {
      if (!tex || y0 > y1) return;
      const col = ((Math.floor(u) % tex.w) + tex.w) % tex.w, s = z / PROJ;
      for (let y = y0; y <= y1; y++) {
        const wz = V.z - (y + 0.5 - CY) * s, v = Math.floor(refTop - wz + yo), i = y * SW + x;
        fb[i] = cmap[base + tex.px[((v % tex.h) + tex.h) % tex.h * tex.w + col]]; zb[i] = z;
      }
    }
    const dt = wad.texture("BIGDOOR7") || wad.texture("DOOR3");
    function seg(sg0) {
      let sg = sg0;
      if (win) {                                 // seen through the door: what is on the office's side of its wall is not there
        const b1 = L.beyond(sg.v1.x, sg.v1.y), b2 = L.beyond(sg.v2.x, sg.v2.y);
        if (b1 <= 0.01 && b2 <= 0.01) return;
        if (b1 < 0 || b2 < 0) {
          const k = b1 / (b1 - b2), c = { x: sg.v1.x + (sg.v2.x - sg.v1.x) * k, y: sg.v1.y + (sg.v2.y - sg.v1.y) * k };
          sg = b1 < 0 ? { ...sg0, v1: c, offset: sg0.offset + Math.hypot(c.x - sg0.v1.x, c.y - sg0.v1.y) } : { ...sg0, v2: c };
        }
      }
      const a1 = norm(angTo(sg.v1.x, sg.v1.y)), a2 = norm(angTo(sg.v2.x, sg.v2.y));
      const r = clipAngles(a1, a2);
      if (!r || r[0] >= r[1]) return;
      const fs = sg.front.sector, bs = sg.back && sg.back.sector, line = sg.line, sd = sg.front;
      const closed = !bs || bs.ceil <= fs.floor || bs.floor >= fs.ceil;
      const isDoor = !win && P && P.lines.has(line);
      if (!isDoor && bs && !closed && bs.ceil === fs.ceil && bs.floor === fs.floor && bs.ftex === fs.ftex && bs.ctex === fs.ctex && bs.light === fs.light && sd.mid === "-") return;   // nothing to draw: a line within one floor
      const ex = sg.v2.x - sg.v1.x, ey = sg.v2.y - sg.v1.y, len = Math.hypot(ex, ey), wx0 = sg.v1.x - V.x, wy0 = sg.v1.y - V.y;
      const fake = sg0.v1.y === sg0.v2.y ? -1 : sg0.v1.x === sg0.v2.x ? 1 : 0, ln = (fs.light >> 4) + extralight + fake;
      const midT = closed && sd.mid !== "-" ? wad.texture(anim(sd.mid)) : null, upT = !closed && sd.upper !== "-" ? wad.texture(anim(sd.upper)) : null, loT = !closed && sd.lower !== "-" ? wad.texture(anim(sd.lower)) : null;
      const maskT = !closed && sd.mid !== "-" ? wad.texture(anim(sd.mid)) : null;
      const bothSky = bs && fs.ctex === "F_SKY1" && bs.ctex === "F_SKY1";
      for (let x = Math.max(0, r[0]); x < Math.min(SW, r[1]); x++) {
        if (solid[x]) continue;
        const rdx = dx + rx * kx[x], rdy = dy + ry * kx[x], den = rdx * ey - rdy * ex;
        if (Math.abs(den) < 1e-9) continue;
        const z = Math.max(1, (wx0 * ey - wy0 * ex) / den), t = (wx0 * rdy - wy0 * rdx) / den;
        const u = sg.offset + sd.xo + Math.max(0, Math.min(1, t)) * len, sc2 = PROJ / z, base = wallLevel(ln, z) * 256;
        const yc = CY - (fs.ceil - V.z) * sc2, yf = CY - (fs.floor - V.z) * sc2;
        const top = Math.max(ceilClip[x] + 1, Math.ceil(yc - 0.5)), bot = Math.min(floorClip[x] - 1, Math.floor(yf - 0.5));
        if (fs.ceil > V.z || fs.ctex === "F_SKY1") plane(x, ceilClip[x] + 1, Math.min(top - 1, floorClip[x] - 1), fs, false);
        if (fs.floor < V.z) plane(x, Math.max(bot + 1, ceilClip[x] + 1), floorClip[x] - 1, fs, true);
        if (closed) {
          if (midT) wallRun(x, top, bot, midT, u, line.flags & 16 ? fs.floor + midT.h : fs.ceil, sd.yo, z, base);
          if (isDoor) doorColumn(x, sg, t, z, top, bot, base);
          solid[x] = 1; solidN++;
          continue;
        }
        const ybc = CY - (bs.ceil - V.z) * sc2, ybf = CY - (bs.floor - V.z) * sc2;
        let nc = top - 1, nf = bot + 1;
        if (bs.ceil < fs.ceil && !bothSky) {
          const y1 = Math.min(bot, Math.floor(ybc - 0.5));
          if (upT) wallRun(x, top, y1, upT, u, line.flags & 8 ? fs.ceil : bs.ceil + upT.h, sd.yo, z, base);
          nc = Math.max(nc, y1);
        }
        if (bs.floor > fs.floor) {
          const y0 = Math.max(top, Math.ceil(ybf - 0.5));
          if (loT) wallRun(x, y0, bot, loT, u, line.flags & 16 ? fs.ceil : bs.floor, sd.yo, z, base);
          nf = Math.min(nf, y0);
        }
        if (isDoor) { doorColumn(x, sg, t, z, top, bot, base); nf = Math.min(nf, Math.max(top, Math.ceil(CY - (P.floor + DOORH - V.z) * sc2 - 0.5))); }
        ceilClip[x] = Math.max(ceilClip[x], nc); floorClip[x] = Math.min(floorClip[x], nf);
        if (maskT) masked.push({ x, z, u, tex: maskT, refTop: line.flags & 16 ? Math.max(fs.floor, bs.floor) + maskT.h : Math.min(fs.ceil, bs.ceil), yo: sd.yo, base, c0: ceilClip[x], c1: floorClip[x] });
        if (ceilClip[x] >= floorClip[x] - 1) { solid[x] = 1; solidN++; }
      }
    }
    // the mysterious door in its wall: its panel risen door.open; under it, the office (the window)
    function doorColumn(x, sg, t, z, top, bot, base) {
      const hx = sg.v1.x + (sg.v2.x - sg.v1.x) * Math.max(0, Math.min(1, t)), hy = sg.v1.y + (sg.v2.y - sg.v1.y) * Math.max(0, Math.min(1, t)), al = L.along(hx, hy);
      if (Math.abs(al) >= DOORW / 2 || !dt) return;
      const s = z / PROJ, du = Math.min(dt.w - 1, Math.max(0, Math.floor((al + DOORW / 2) / DOORW * dt.w)));
      let wy0 = VH, wy1 = -1;
      for (let y = Math.max(top, Math.ceil(CY - (P.floor + DOORH - V.z) * PROJ / z - 0.5)), ye = Math.min(bot, Math.floor(CY - (P.floor - V.z) * PROJ / z - 0.5)); y <= ye; y++) {
        const h = V.z - (y + 0.5 - CY) * s - P.floor, i = y * SW + x;
        if (h < door.open) { if (y < wy0) wy0 = y; wy1 = y; continue; }
        const dv = Math.min(dt.h - 1, Math.max(0, Math.floor((DOORH - (h - door.open)) / DOORH * dt.h)));
        fb[i] = cmap[base + dt.px[dv * dt.w + du]]; zb[i] = z;
      }
      if (wy1 >= 0) { wTop[x] = wy0 - 1; wBot[x] = wy1 + 1; wZ[x] = z; }
    }
    function node(n) {
      if (solidN >= SW) return;
      if (n & 0x8000) { const ss = m.ssectors[n & 0x7fff]; for (let k = 0; k < ss.n; k++) seg(m.segs[ss.first + k]); return; }
      const nd = m.nodes[n], s = L.side(nd, V.x, V.y);
      node(nd.kids[s]);
      if (boxSeen(nd.bbox[s ^ 1])) node(nd.kids[s ^ 1]);
    }
    node(m.nodes.length ? m.nodes.length - 1 : 0x8000);
    // what is left open (beyond the map's edge): black
    for (let x = 0; x < SW; x++) if (!solid[x]) for (let y = ceilClip[x] + 1; y < floorClip[x]; y++) { fb[y * SW + x] = 0; }
    // through the open door: the office, as it is from here
    let w2 = null;
    if (!win && P && door.open > 0 && (w2 = winOf())) {
      const [ox, oy] = l2o(V.x, V.y);
      winZ(w2);
      renderOffice(now, { x: ox, y: oy, eye: V.z - P.floor, a: a2o(V.a) }, w2);
      winZ(w2, wZ);
    }
    // masked middle textures (grates, bars), then the sprites, far to near
    for (const mk of masked) {
      const tx = mk.tex, col = ((Math.floor(mk.u) % tx.w) + tx.w) % tx.w, s = mk.z / PROJ;
      for (let y = Math.max(0, mk.c0 + 1); y < Math.min(VH, mk.c1); y++) {
        const wz = V.z - (y + 0.5 - CY) * s, v = Math.floor(mk.refTop - wz + mk.yo), i = y * SW + mk.x;
        if (v < 0 || v >= tx.h || mk.z >= zb[i]) continue;
        const k = v * tx.w + col;
        if (!tx.op[k]) continue;
        fb[i] = cmap[mk.base + tx.px[k]]; zb[i] = mk.z;
      }
    }
    vis = [];
    for (const o of L.mobjs) {
      if (o.office || (win && L.beyond(o.x, o.y) <= 0)) continue;
      const vx = o.x - V.x, vy = o.y - V.y, z = vx * dx + vy * dy;
      if (z < 4) continue;
      const fr = o.sprite || o.d.s, f = o.frame.charCodeAt(0) - 65;
      const ang = Math.atan2(vy, vx), rot = Math.floor(norm(ang - (o.angle || 0) + Math.PI + Math.PI / 8) / (Math.PI / 4)) % 8 + 1;
      const sf = wad.spriteFrame(fr, f, rot);
      if (!sf || !sf.pic) continue;
      const scale = PROJ / z, sec = o.sector || L.sectorAt(o.x, o.y);
      vis.push({ p: sf.pic, flip: sf.flip, sx: CX + (vx * rx + vy * ry) * scale, sy: CY - (o.z + sf.pic.to - V.z) * scale, scale, z, bright: o.bright,
        level: spriteLevel((sec.light >> 4) + extralight, z), fuzz: !!(o.d && o.d.shadow), clipTop: win && win.top, clipBot: win && win.bot });
    }
    for (const f of L.puffs) if (!win || L.beyond(f.x, f.y) > 0) puffVis(f, V.x, V.y, dx, dy, rx, ry, V.z, () => (L.sectorAt(f.x, f.y).light >> 4) + extralight, win);
    vis.sort((a, b) => b.z - a.z);
    for (const v of vis) drawVis(v);
  }

  // ---- the shotgun, Doom's: its frames and their tics, the flash (A_Light1, A_Light2), the bob while you walk;
  // your skin on its hands
  const GUN = [["A", 3], ["A", 7, "fire"], ["B", 5], ["C", 5], ["D", 4], ["C", 5], ["B", 5], ["A", 3], ["A", 7]];
  let gunState = -1, gunTics = 0, flash = -1, flashTics = 0, wsx = 1, wsy = 32, time = 0, frameNo = 0, aimThrough = false;
  let handTrans = null;
  function hands() {                          // Freedoom's and Doom's hands are the brown ramp: your skin instead
    const t = new Uint8Array(256);
    for (let i = 0; i < 256; i++) t[i] = i;
    try {
      const sk = room.hands ? room.hands().skin : null, hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
      if (!sk) return t;
      const [r, g, b] = hex(sk), lum = (q) => 0.3 * wad.rgbOf(q)[0] + 0.59 * wad.rgbOf(q)[1] + 0.11 * wad.rgbOf(q)[2], mid = lum(72) || 1;
      for (const i of [64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 236, 237, 238, 239]) {
        const k = lum(i) / mid;
        t[i] = wad.nearest(Math.min(255, r * k), Math.min(255, g * k), Math.min(255, b * k));
      }
    } catch (_) { /* the gun as it is */ }
    return t;
  }
  function gunTic() {
    if (flash >= 0 && --flashTics <= 0) { flash++; flashTics = 3; if (flash > 1) { flash = -1; extralight = 0; } else extralight = 2; }
    if (gunState < 0) return;
    if (--gunTics > 0) return;
    gunState++;
    if (gunState >= GUN.length) { gunState = -1; return; }
    gunTics = GUN[gunState][1];
    if (GUN[gunState][2] === "fire") shoot();
  }
  function pull() {                           // the trigger: the shot comes three tics in, as Doom's
    const p = lvl.player;
    if (gunState >= 0 || p.dead || p.ammo.shells <= 0) return;
    gunState = 0; gunTics = GUN[0][1];
  }
  function shoot() {
    const p = lvl.player;
    if (p.ammo.shells <= 0) return;
    flash = 0; flashTics = 4; extralight = 1;
    if (world === "level") { lvl.fire(p); return; }
    // the office: the bot nearest you within a few degrees of where you look (Doom's autoaim) goes to pieces; else,
    // through the open door, the shot goes into the level (you are there in its terms, behind its door)
    const { WALL } = units(), dx = Math.cos(me.a), dy = Math.sin(me.a);
    let best = null, bd = 1e9;
    for (const th of sc.things) {
      if (!th.bot) continue;
      const vx = th.x * U - me.x, vy = th.y * U - me.y, d = vx * dx + vy * dy;
      if (d < 8 || d > bd) continue;
      if (Math.abs(vx * -dy + vy * dx) > 10 + d * Math.tan(5 * Math.PI / 180)) continue;
      const sx = Math.round(CX + (vx * -dy + vy * dx) * PROJ / d);
      if (sx >= 0 && sx < SW && zb[CY * SW + sx] < d - 6) continue;   // behind a desk
      best = th; bd = d;
    }
    if (P && door.open > 0) lvl.noise(p);       // the door open: the level hears it
    if (!best && P && aimThrough) { lvl.fire(p); room.alarm(me.x / U, me.y / U); return; }
    p.ammo.shells--;
    if (best && room.shoot(best.bot)) return void room.alarm(me.x / U, me.y / U);
    const d = Math.min(zb[CY * SW + CX], 3000) * 0.97;
    puffs.push({ x: me.x + dx * d, y: Math.max(WALL + 2, me.y + dy * d), z: 41, t: 0 });
    room.alarm(me.x / U, me.y / U);
  }
  function drawGun() {
    const p = lvl.player;
    if (p.dead) return;
    const fr = gunState >= 0 ? GUN[gunState][0] : "A", g = wad.spriteFrame("SHTG", fr.charCodeAt(0) - 65, 1);
    const ln = world === "level" ? (p.sector.light >> 4) + extralight : (officeLight(me.x, me.y) >> 4) + extralight;
    const lv = Math.max(0, (15 - clampL(ln)) * 4 - 23);
    const draw = (q, bright, trans) => {        // a psprite, Doom's place for it, S pixels a texel
      if (!q) return;
      const x0 = Math.round((wsx - q.lo) * S), y0 = Math.round((wsy - q.to - 16) * S);
      for (let j = 0; j < q.h * S; j++) {
        const y = y0 + j;
        if (y < 0 || y >= VH) continue;
        const row = Math.floor(j / S) * q.w;
        for (let i = 0; i < q.w * S; i++) {
          const x = x0 + i, k = row + Math.floor(i / S);
          if (x < 0 || x >= SW || !q.op[k]) continue;
          const c = trans ? trans[q.px[k]] : q.px[k];
          fb[y * SW + x] = bright ? c : cmap[lv * 256 + c];
        }
      }
    };
    draw(g && g.pic, false, handTrans);
    if (flash >= 0) { const f = wad.spriteFrame("SHTF", flash, 1); draw(f && f.pic, true); }
  }

  // ---- the status bar: Doom's, from the WAD, the game's numbers; your face in it
  let faceSet = null, faceLook = 0, faceTics = 0, grinUntil = 0, kills0 = 0, ouchUntil = 0, lastHealth = 100;
  function num(n, right, y, prefix, pct) {    // Doom's big numbers, right-aligned (in 320 x 200)
    const s = String(Math.max(-99, Math.min(999, Math.round(n))));
    let x = right;
    if (pct) blit(PT("STTPRCNT"), right * S, y * S, null, S);
    for (let k = s.length - 1; k >= 0; k--) {
      const p = s[k] === "-" ? PT("STTMINUS") : PT(prefix + "NUM" + s[k]);
      if (!p) continue;
      x -= p.w; blit(p, x * S, y * S, null, S);
    }
  }
  function smallNum(n, right, y) {
    const s = String(Math.max(0, Math.min(999, Math.round(n))));
    let x = right;
    for (let k = s.length - 1; k >= 0; k--) { const p = PT("STYSNUM" + s[k]); if (!p) continue; x -= p.w; blit(p, x * S, y * S, null, S); }
  }
  // your face, Doom's way: 30 x 30 (at S) from your 20 x 20 avatar, for each state (pain 0..4, looking ahead, left,
  // right; the grin, the ouch, dead), in the palette
  function faces(src) {
    return new Promise((res) => {
      const im = new Image();
      im.onload = () => {
        const c = document.createElement("canvas"); c.width = 20; c.height = 20;
        const g = c.getContext("2d"); g.drawImage(im, 0, 0, 20, 20);
        const base = g.getImageData(0, 0, 20, 20), F = 30 * S;
        const make = (pain, look, mode) => {
          const d = new ImageData(new Uint8ClampedArray(base.data), 20, 20), p = d.data;
          const set = (x, y, col) => { if (x < 0 || y < 0 || x > 19 || y > 19) return; const i = (y * 20 + x) * 4; if (p[i + 3] < 100) return; p[i] = col[0]; p[i + 1] = col[1]; p[i + 2] = col[2]; };
          const dark = (x, y, k) => { const i = (y * 20 + x) * 4; if (p[i + 3] < 100) return; p[i] *= k; p[i + 1] *= k; p[i + 2] *= k; };
          const RED = [176, 24, 24], RED2 = [120, 12, 12], INK = [20, 15, 30], WHITE = [240, 240, 240];
          if (pain >= 1) { set(12, 5, RED); set(12, 6, RED2); set(8, 12, RED2); }
          if (pain >= 2) { dark(12, 11, 0.6); dark(13, 11, 0.6); set(13, 6, RED); set(13, 7, RED2); set(7, 5, RED); }
          if (pain >= 3) { set(9, 14, RED); for (let y = 4; y <= 8; y++) set(7, y, y % 2 ? RED : RED2); dark(6, 9, 0.55); dark(6, 10, 0.55); dark(7, 11, 0.6); }
          if (pain >= 4) { for (let y = 4; y <= 8; y++) for (let x = 10; x <= 13; x++) if ((x + y) % 2) set(x, y, RED2); for (let y = 12; y <= 14; y++) for (let x = 5; x <= 7; x++) set(x, y, RED); dark(12, 9, 0.4); dark(13, 9, 0.4); dark(12, 10, 0.5); }
          if (mode === "grin") { for (let x = 8; x <= 12; x++) set(x, 14, WHITE); set(8, 13, RED2); set(12, 13, RED2); set(9, 15, RED2); set(11, 15, RED2); }
          if (mode === "ouch") { for (let y = 13; y <= 15; y++) for (let x = 9; x <= 11; x++) set(x, y, y === 13 ? RED2 : INK); set(7, 8, WHITE); set(12, 8, WHITE); }
          if (mode === "dead") {
            for (let i = 0; i < 400; i++) { const q = i * 4; if (p[q + 3] < 100) continue; const v = 0.3 * p[q] + 0.59 * p[q + 1] + 0.11 * p[q + 2]; p[q] = v * 0.9 + 20; p[q + 1] = v * 0.85; p[q + 2] = v * 0.85; }
            for (const ex of [7, 12]) { set(ex, 9, INK); set(ex + 1, 10, INK); set(ex + 1, 9, INK); set(ex, 10, INK); }
            for (let x = 8; x <= 12; x++) set(x, 14, INK);
            set(10, 15, RED); set(10, 16, RED2);
          }
          const t = document.createElement("canvas"); t.width = 20; t.height = 20; t.getContext("2d").putImageData(d, 0, 0);
          const s = document.createElement("canvas"); s.width = F; s.height = F;
          const sg = s.getContext("2d"); sg.imageSmoothingEnabled = F % 20 !== 0;   // a whole number of pixels a pixel: crisp
          sg.drawImage(t, look, 0, 20, 20, 0, 0, F, F);
          s.v = 1;
          return pic(s);
        };
        const out = { pain: [], grin: [], ouch: [], dead: make(4, 0, "dead") };
        for (let pn = 0; pn < 5; pn++) { out.pain.push([make(pn, 0), make(pn, 1), make(pn, -1)]); out.grin.push(make(pn, 0, "grin")); out.ouch.push(make(pn, 0, "ouch")); }
        res(out);
      };
      im.onerror = () => res(null);
      im.src = src;
    });
  }
  function face(health, dead, kills) {
    const now = performance.now();
    if (kills > kills0) grinUntil = now + 2000;
    kills0 = kills;
    if (health < lastHealth - 20) ouchUntil = now + 1000;
    lastHealth = health;
    const pain = Math.max(0, Math.min(4, Math.floor((100 - Math.min(100, Math.max(0, health))) * 5 / 101)));
    if (--faceTics <= 0) { faceLook = Math.floor(Math.random() * 3); faceTics = 17; }
    if (faceSet) {
      if (dead) return [faceSet.dead, 1];
      if (now < grinUntil) return [faceSet.grin[pain], 1];
      if (now < ouchUntil) return [faceSet.ouch[pain], 1];
      return [faceSet.pain[pain][faceLook], 1];
    }
    return [PT(dead ? "STFDEAD0" : now < grinUntil ? "STFEVL" + pain : "STFST" + pain + faceLook), S];   // Doom's own while yours loads
  }
  function statusBar() {
    const p = lvl.player;
    blit(PT("STBAR"), 0, 168 * S, null, S);
    blit(PT("STARMS"), 104 * S, 168 * S, null, S);
    num(p.ammo.shells, 44, 171, "STT");
    num(p.health, 90, 171, "STT", true);
    num(p.armor, 221, 171, "STT", true);
    for (let i = 0; i < 6; i++) blit(PT((i === 1 ? "STYSNUM" : "STGNUM") + (i + 2)), (111 + (i % 3) * 12) * S, (172 + Math.floor(i / 3) * 10) * S, null, S);   // the arms: 3, the shotgun
    ["blue", "yellow", "red"].forEach((k, i) => { if (p.keys[k]) blit(PT("STKEYS" + i), 239 * S, (171 + i * 10) * S, null, S); });
    ["bullets", "shells", "rockets", "cells"].forEach((k, i) => { const y = [173, 179, 185, 191][i]; smallNum(p.ammo[k], 288, y); smallNum(p.max[k], 314, y); });
    const kills = p.kills + ((room.status() || {}).kills || 0), [f, k] = face(p.health, p.dead, kills);
    blit(f, (k === 1 ? 145 : 143) * S, (k === 1 ? 169 : 168) * S, null, k);
  }

  // ---- the frame: what you see, the gun, the bar, the palette (red while you hurt, gold when you pick up)
  const officeCam = () => { const p = lvl.player; return { x: me.x, y: me.y, eye: p.viewh + (still ? 0 : Math.sin(time * 2 * Math.PI / 20) * me.bob / 2), a: me.a }; };
  const levelCam = () => { const p = lvl.player; return { x: p.x, y: p.y, z: p.z + p.viewh + (p.dviewh || 0) + (p.dead || still ? 0 : Math.sin(lvl.time * 2 * Math.PI / 20) * (p.bob || 0) / 2), a: p.angle }; };
  function render(now) {
    frameNo++;
    lightGrid();
    if (world === "level") renderLevel(now, levelCam(), null); else renderOffice(now, officeCam(), null);
    drawGun();
    if (world === "office") officeHud();
    else if (performance.now() - t0 < 6000) help();
    statusBar();
    const p = lvl.player;
    let pal = 0;
    if (p.damagecount) pal = Math.min(8, (p.damagecount + 7) >> 3);
    else if (p.bonuscount) pal = 8 + Math.min(4, (p.bonuscount + 7) >> 3);
    const rgb = wad.rgb[pal] || wad.rgb[0];
    for (let i = 0; i < SW * SH; i++) out32[i] = rgb[fb[i]];
    ctx.putImageData(img, 0, 0);
  }
  const help = () => words("WASD WALK - MOUSE TURN - CLICK SHOOT - E USE - ESC LEAVE", 2 * S, VH - 7 * S, wad.nearest(190, 190, 200));
  function officeHud() {
    const th = under(2), bot = th && th.bot, what = !bot && th && room.label(th);
    const rm = near(), who = bot ? th : rm || null, white = wad.nearest(232, 232, 238), mid = (t, y, c) => words(t, Math.round(CX - room.tinyW(t) * S / 2), y * S, c);
    if (who && who.name) mid(who.name.toUpperCase(), 4, bot ? wad.nearest(255, 140, 120) : white);
    else if (what) mid(what.toUpperCase(), 4, white);
    if (!bot && rm) mid("E: PUT BACK TOGETHER", 11, white);
    if (performance.now() - t0 < 6000) help();
  }
  function under(r) {
    let best = null, bd = Infinity;
    r *= S;
    for (let y = CY - r; y <= CY + r; y++) for (let x = CX - r; x <= CX + r; x++) { const i = y * SW + x, n = idb[i]; if (n && zb[i] < bd) { best = drawn[n - 1]; bd = zb[i]; } }
    return best;
  }
  function near() {
    const th = under(4);
    if (th && th.rem) return th;
    let best = null, bd = 70 * U;
    for (const t of sc.things) if (t.rem) {
      const rx2 = t.x * U - me.x, ry2 = t.y * U - me.y, d = Math.hypot(rx2, ry2) || 1;
      if (d < bd && (rx2 * Math.cos(me.a) + ry2 * Math.sin(me.a)) / d > 0.5) { best = t; bd = d; }
    }
    return best;
  }

  // ---- input: WASD (or the arrows) walk, the mouse turns (the pointer locked), Shift runs, a click (or Ctrl, F,
  // Enter) fires, E (or Space) uses, Esc leaves
  const keys = new Set();
  let locked = false, dragging = null, nolock = false, turnAcc = 0, useReq = false;
  const onKey = (e) => {
    e.stopPropagation();
    if (e.metaKey) return;
    e.preventDefault();
    if (e.type === "keyup") { keys.delete(e.code); return; }
    if (e.key === "Escape") return leave();
    keys.add(e.code);
    if (e.code === "KeyE" || e.code === "Space") useReq = true;
    if (e.code === "Enter" || e.code === "KeyF" || e.code === "ControlLeft" || e.code === "ControlRight") act();
  };
  const onMove = (e) => {
    if (locked) turnAcc -= e.movementX * 0.0035;
    else if (dragging) { turnAcc -= (e.clientX - dragging[0]) * 0.008; dragging = [e.clientX, e.clientY]; }
  };
  const onDown = (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    if (locked || nolock) act();
    if (!locked) { dragging = [e.clientX, e.clientY]; lock(); }
  };
  const onUp = () => { dragging = null; };
  const onLock = () => { const now = document.pointerLockElement === wrap; if (locked && !now) leave(); locked = now; if (now) nolock = false; };
  function lock() { try { const p = wrap.requestPointerLock(); if (p && p.catch) p.catch(() => { nolock = true; }); } catch (_) { nolock = true; } }
  const onLockError = () => { nolock = true; };
  const blur = () => keys.clear();
  addEventListener("keydown", onKey, true); addEventListener("keyup", onKey, true);
  addEventListener("mousemove", onMove); wrap.addEventListener("mousedown", onDown); addEventListener("mouseup", onUp);
  document.addEventListener("pointerlockchange", onLock); document.addEventListener("pointerlockerror", onLockError);
  addEventListener("blur", blur);
  lock();
  function act() {                            // a click: fire; dead, a while after, Doom's respawn
    if (!lvl) return;
    const p = lvl.player;
    if (p.dead) { if (lvl.time - p.deadAt > 35) respawn(); return; }
    pull();
  }
  const walking = () => keys.has("KeyW") || keys.has("KeyA") || keys.has("KeyS") || keys.has("KeyD") || keys.has("ArrowUp") || keys.has("ArrowDown");
  let turnHeld = 0;
  function cmd() {                            // this tic's move, Doom's way: the keys turn slowly the first six tics
    const fw = (keys.has("KeyW") || keys.has("ArrowUp") ? 1 : 0) - (keys.has("KeyS") || keys.has("ArrowDown") ? 1 : 0);
    const sd = (keys.has("KeyD") ? 1 : 0) - (keys.has("KeyA") ? 1 : 0);
    const run = keys.has("ShiftLeft") || keys.has("ShiftRight");
    const kt = (keys.has("ArrowLeft") ? 1 : 0) - (keys.has("ArrowRight") ? 1 : 0);
    turnHeld = kt ? turnHeld + 1 : 0;
    const turn = turnAcc + kt * (turnHeld < 6 ? 320 : run ? 1280 : 640) / 65536 * 2 * Math.PI;
    turnAcc = 0;
    const use = useReq; useReq = false;
    return { forward: fw, side: sd, run, turn, use };
  }

  // ---- the tics: 35 a second, whatever the frame rate. The level goes on while you are in the office
  function officeTic(c) {
    const p = lvl.player, { H } = units();
    if (!p.dead) {
      me.a -= c.turn;                         // (the room's y runs down: right is the other way round)
      const fw = c.forward * (c.run ? 50 : 25) / 32, sd = c.side * (c.run ? 40 : 24) / 32, ca = Math.cos(me.a), sa = Math.sin(me.a);
      me.mx += ca * fw - sa * sd; me.my += sa * fw + ca * sd;
      const sp = Math.hypot(me.mx, me.my);
      if (sp > 30) { me.mx *= 30 / sp; me.my *= 30 / sp; }
      const stuck = officeBlocked(me.x, me.y);
      if (stuck || !officeBlocked(me.x + me.mx, me.y)) me.x += me.mx; else me.mx = 0;
      if (stuck || !officeBlocked(me.x, me.y + me.my)) me.y += me.my;
      else { if (me.my > 0 && me.y > H - R - 8 && Math.abs(me.x - doorX()) < DOORW / 2 && P) bump(); me.my = 0; }   // walked into the door: it opens
      me.mx *= 0.90625; me.my *= 0.90625;
      if (Math.abs(me.mx) < 0.0625 && Math.abs(me.my) < 0.0625 && !c.forward && !c.side) me.mx = me.my = 0;
      me.bob = Math.min(16, (me.mx * me.mx + me.my * me.my) / 4);
      if (c.use) use();
    }
    room.move(me.x / U, me.y / U, me.a, walking());
    if (P) {                                  // where you are in the level's terms: behind its door
      const [x, y] = o2l(me.x, me.y);
      p.x = x; p.y = y; p.z = p.floorz = P.floor; p.angle = a2l(me.a); p.sector = P.sector; p.mx = p.my = 0;
      if (me.y > H && door.open >= 56 && Math.abs(me.x - doorX()) <= DOORW / 2 - PR) throughToLevel();
    }
    if (time % 4 === 0) fireFlick = Math.random() * 64 | 0;   // the fire flickers (Doom's FireFlicker)
  }
  function use() {
    if (world !== "office") return;
    const { H } = units(), p = lvl.player;
    if (P && me.y > H - 64 - R && Math.sin(me.a) > 0.6 && Math.abs(me.x + Math.cos(me.a) * (H - me.y) / Math.sin(me.a) - doorX()) < DOORW / 2) return void bump();   // the door in front of you
    const th = under(3);
    if (th && th.rem) return room.unshoot(th.rem);
    if (th && room.use(th) === "reload") {   // the gun closet: a box of shells
      if (p.ammo.shells < p.max.shells) { p.ammo.shells = Math.min(p.max.shells, p.ammo.shells + 20); p.bonuscount = Math.min(p.bonuscount + 6, 40); }
      return;
    }
    const r = near();
    if (r) room.unshoot(r.rem);
  }
  function tic() {
    time++;
    const c = cmd();
    if (world === "level") {
      lvl.away = false;
      lvl.tic(c);
      const p = lvl.player;
      if (lvl.done === "exit") exitLevel();
      else if (P && lvl.beyond(p.x, p.y) < 0) throughToOffice();
    } else {
      officeTic(c);
      if (world === "office") { lvl.away = true; lvl.tic(c); }
    }
    doorTic();
    // the weapon: Doom's bob while ready (A_WeaponReady), then its frames
    const bob = world === "level" ? lvl.player.bob || 0 : me.bob;
    if (gunState < 0) { const ang = (128 * time) % 8192 / 8192 * 2 * Math.PI; wsx = 1 + bob * Math.cos(ang); wsy = 32 + bob * Math.abs(Math.sin(ang)); }
    gunTic();
    for (const f of puffs) f.t++;
    puffs = puffs.filter((f) => f.t < 16);
  }

  // ---- through the door, either way: the same place, the same way you look and move, no wipe
  function throughToLevel() {
    const p = lvl.player, [x, y] = o2l(me.x, me.y);
    p.x = x; p.y = y; p.z = p.floorz = P.floor; p.mz = 0; p.angle = a2l(me.a); p.sector = lvl.sectorAt(x, y);
    p.mx = P.nx * me.my + P.tx * me.mx; p.my = P.ny * me.my + P.ty * me.mx; p.bob = me.bob;
    world = "level"; lvl.away = false;
  }
  function throughToOffice() {
    const p = lvl.player, [x, y] = l2o(p.x, p.y);
    me.x = x; me.y = y; me.a = a2o(p.angle);
    me.my = p.mx * P.nx + p.my * P.ny; me.mx = p.mx * P.tx + p.my * P.ty; me.bob = p.bob || 0;
    world = "office"; lvl.away = true;
  }
  // the level from its start: dead (Doom's respawn: as you began), or out through its exit (what you carry stays);
  // either way back in the office, in front of the door, with Doom's wipe
  let melt = null, t0 = performance.now();
  function newLevel(keep) {
    const old = lvl && lvl.player;
    lvl = new DoomLevel(wad, levelName);
    setPortal();
    const p = lvl.player;
    if (keep && old) { p.health = old.health; p.armor = old.armor; p.armorClass = old.armorClass; p.ammo = old.ammo; p.max = old.max; }
    lvl.away = true;
    door.open = 0; door.dir = 0; door.wait = 0;
  }
  function backHome(keep) {
    const before = shot(), { H } = units();
    newLevel(keep);
    world = "office"; gunState = -1; flash = -1; extralight = 0;
    me.x = doorX(); me.y = H - 40; me.a = -Math.PI / 2; me.mx = me.my = 0;
    melt = startMelt(before);
  }
  const respawn = () => backHome(false), exitLevel = () => backHome(true);
  function shot(from) {                       // the screen as it is, as large as the window (the bars black)
    const c = document.createElement("canvas"); c.width = innerWidth; c.height = innerHeight;
    const g = c.getContext("2d"), r = cv.getBoundingClientRect();
    g.fillStyle = "#000"; g.fillRect(0, 0, c.width, c.height); g.imageSmoothingEnabled = false;
    if (from) g.drawImage(from.canvas, from.rect.left, from.rect.top, from.rect.width, from.rect.height);
    else g.drawImage(cv, r.left, r.top, r.width, r.height);
    return c;
  }
  function startMelt(from) {
    if (still) return null;
    const n = 160, y = new Float32Array(n);
    y[0] = -Math.random() * 16;
    for (let i = 1; i < n; i++) y[i] = Math.max(-15, Math.min(0, y[i - 1] + Math.floor(Math.random() * 3) - 1));
    return { from, y, n, last: performance.now() };
  }
  function drawMelt(m, now) {
    const H = mc.height, k = H / 200, cw = mc.width / m.n;
    let tics = Math.min(6, Math.floor((now - m.last) / TICMS));
    m.last += tics * TICMS;
    while (tics-- > 0) for (let i = 0; i < m.n; i++) { if (m.y[i] < 0) m.y[i]++; else m.y[i] += (m.y[i] < 16 * k ? m.y[i] + 1 : 8 * k); }
    mx.clearRect(0, 0, mc.width, H);
    let on = false;
    for (let i = 0; i < m.n; i++) {
      const yy = Math.max(0, Math.round(m.y[i])), x = Math.floor(i * cw), w = Math.floor((i + 1) * cw) - x;
      if (yy >= H) continue;
      on = true;
      mx.drawImage(m.from, x, 0, w, H - yy, x, yy, w, H - yy);
    }
    return on;
  }

  // ---- the loop
  let raf = 0, last = performance.now(), acc = 0, ms = 0, ready = false;
  const first = shot(room.snapshot && room.snapshot());
  mx.drawImage(first, 0, 0);                  // the room, until Doom's data is in and the melt can begin
  Wad.load("/doom.wad").then((w) => {
    wad = w; cmap = w.cmap;
    w.rgbOf = (i) => [w.rgb[0][i] & 255, (w.rgb[0][i] >> 8) & 255, (w.rgb[0][i] >> 16) & 255];
    handTrans = hands();
    levelName = DoomLevel.first(w);
    if (!levelName) throw new Error("no level in the WAD");
    newLevel(false);
    const src = room.face && room.face();
    if (src) faces(src).then((f) => { faceSet = f; });
    ready = true;
    melt = startMelt(first);
    last = performance.now();
  }).catch((e) => { console.warn("burrow: no Doom data", e); leave(); });
  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (!ready) return;
    acc += Math.min(250, now - last); last = now;
    while (acc >= TICMS) { tic(); acc -= TICMS; }
    const t = performance.now();
    render(now);
    if (melt && !drawMelt(melt, now)) { melt = null; mx.clearRect(0, 0, mc.width, mc.height); }
    ms = ms * 0.95 + (performance.now() - t) * 0.05;
  }
  raf = requestAnimationFrame(frame);

  function leave() {
    cancelAnimationFrame(raf);
    removeEventListener("keydown", onKey, true); removeEventListener("keyup", onKey, true);
    removeEventListener("mousemove", onMove); removeEventListener("mouseup", onUp); removeEventListener("resize", fit); removeEventListener("blur", blur);
    document.removeEventListener("pointerlockchange", onLock); document.removeEventListener("pointerlockerror", onLockError);
    if (document.pointerLockElement === wrap) document.exitPointerLock();
    window.__fps = null;
    const out = ready ? startMelt(shot()) : null;   // going out: this view melts away over the room
    cv.style.visibility = "hidden"; wrap.style.background = "transparent"; wrap.style.pointerEvents = "none"; wrap.style.cursor = "auto";
    if (out) { const step = (now) => { if (drawMelt(out, now)) requestAnimationFrame(step); else wrap.remove(); }; requestAnimationFrame(step); }
    else wrap.remove();
    done();
  }
  // for look.mjs: where you are, turn, step, fire, use, through the door, without a mouse
  window.__fps = { me, ms: () => ms, turn: (r) => { if (world === "level") lvl.player.angle += r; else me.a += r; },
    set: (x, y, ang) => { if (world === "level") { lvl.player.x = x; lvl.player.y = y; if (ang !== undefined) lvl.player.angle = ang; } else { me.x = x * U; me.y = y * U; if (ang !== undefined) me.a = ang; } },
    fire: () => act(), use: () => { useReq = true; }, leave, world: () => world, lvl: () => lvl, ready: () => ready, melting: () => !!melt, keys, door, units, doorX,
    level: () => { door.open = DOORH; door.dir = 0; door.wait = 150; me.x = doorX(); me.y = units().H + 1; me.a = Math.PI / 2; throughToLevel(); },   // straight through the door into the level
    under: () => { const t = under(2); return t && { bot: t.bot, rem: t.rem, name: t.name }; } };
  return { leave };
}
