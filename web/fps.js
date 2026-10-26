// The gun closet, hardcore: the office in the first person, as Wolf3D drew its castle. The whole window is
// the view: the room as it is, raycast. Its floor (the planks, the rug, the mats rolled out, the oil), its
// wall with what hangs on it (the doors, the window, the clock, the closet), the other three walls plain,
// a ceiling of tiles and lights; the desks as blocks, their screens, chairs and everything else as sprites
// (their own pictures from the room), and the bots, walking about as they do there. You hold a shotgun:
// a bot shot goes to pieces, in the room as well (room.js: the remains, the sweepers, the undo); E on the
// pieces puts it back together. Esc leaves; you stand in the room where you left off.
//
// startFps(room, done): room = { scene(), start(), move(x, y, a, walking), shoot(id), unshoot(id), alarm(x, y),
// tiny(ctx, s, x, y, col), tinyW(s), hands() }. scene() is the room at its last tick (twelve a second):
//   { v, W, H, WALL, floor, wall, plain: canvases (the floor W x H, the wall W x WALL, a plain wall),
//     boxes: [{x0, y0, x1, y1, z0, z1, front, back, side, top}]  a face: {c, sx, sy, sw, sh}, a crop of a canvas
//     things: [{key, x, y, z, c, sx, sy, sw, sh, ax, ay, flip, bot, staff, rem, name}]  a sprite standing at
//             x, y (its column ax there), its row ay at height z; a bot to shoot, or a remains' piece
//     solid: [[x0, y0, x1, y1]] }                   what you can't walk through
// A canvas redrawn in place says so with its `v`.
function startFps(room, done) {
  const RH = 200, FOV = 66 * Math.PI / 180, EYE = 21, WALLH = 40, R = 4, TICK = 1000 / 12;
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const cv = document.createElement("canvas");
  cv.id = "fps"; cv.tabIndex = -1;
  document.body.append(cv);
  if (document.activeElement && document.activeElement.blur) document.activeElement.blur();   // keys are the view's now, not a terminal's
  const ctx = cv.getContext("2d");
  let RW = 0, img, buf, zb, idb, focal;
  function resize() {
    RW = Math.max(240, Math.min(440, Math.round(RH * innerWidth / Math.max(1, innerHeight))));
    cv.width = RW; cv.height = RH;
    img = ctx.createImageData(RW, RH); buf = new Uint32Array(img.data.buffer);
    zb = new Float32Array(RW * RH); idb = new Int32Array(RW * RH);
    focal = RW / 2 / Math.tan(FOV / 2);
  }
  resize();
  addEventListener("resize", resize);

  // pictures as pixels (little-endian: ABGR), kept per canvas until it is redrawn
  const texs = new WeakMap();
  function tex(c) {
    let t = texs.get(c);
    if (!t || t.v !== c.v) {
      const d = c.getContext("2d", { willReadFrequently: true }).getImageData(0, 0, c.width, c.height);
      t = { v: c.v, w: c.width, h: c.height, p: new Uint32Array(d.data.buffer) };
      texs.set(c, t);
    }
    return t;
  }
  // smaller copies of a picture, each pixel the mean of four: the planks' seams far off would shimmer in rings
  function mips(t) {
    if (t.mips) return t.mips;
    const out = [t];
    for (let k = 1; k < 3; k++) {
      const a = out[k - 1], w = a.w >> 1, h = a.h >> 1, p = new Uint32Array(w * h);
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
        const i = 2 * y * a.w + 2 * x, c0 = a.p[i], c1 = a.p[i + 1], c2 = a.p[i + a.w], c3 = a.p[i + a.w + 1];
        const r = (c0 & 255) + (c1 & 255) + (c2 & 255) + (c3 & 255), g = (c0 >>> 8 & 255) + (c1 >>> 8 & 255) + (c2 >>> 8 & 255) + (c3 >>> 8 & 255), b = (c0 >>> 16 & 255) + (c1 >>> 16 & 255) + (c2 >>> 16 & 255) + (c3 >>> 16 & 255);
        p[y * w + x] = 0xff000000 | (b >> 2) << 16 | (g >> 2) << 8 | r >> 2;
      }
      out.push({ w, h, p });
    }
    return (t.mips = out);
  }
  // the light: what is far off sinks into the room's dusk (a dark navy), by distance; `k` 1 is full light
  const fogk = (d) => (d < 60 ? 1 : Math.max(0.42, 1 - (d - 60) / 620));
  const lit = (c, k) => {
    const j = 1 - k;
    return 0xff000000 | (((c >>> 16) & 255) * k + 26 * j) << 16 | (((c >>> 8) & 255) * k + 16 * j) << 8 | ((c & 255) * k + 12 * j);
  };
  const rgb = (hex) => { const n = parseInt(hex.slice(1), 16); return 0xff000000 | (n & 255) << 16 | (n & 0xff00) | (n >> 16); };

  // the ceiling: tiles a little lighter than the wall, a light panel every few of them (64 x 96 dots, repeated)
  const CW = 64, CH = 96, ceil = new Uint32Array(CW * CH);
  {
    const tile = rgb("#34405a"), grid = rgb("#28324a"), frame = rgb("#8d95a6"), light = rgb("#eef0e6"), mid = rgb("#dde4ef");
    for (let y = 0; y < CH; y++) for (let x = 0; x < CW; x++) {
      let c = x % 16 === 0 || y % 16 === 0 ? grid : tile;
      if (x >= 16 && x <= 48 && y >= 40 && y <= 56) c = x === 16 || x === 48 || y === 40 || y === 56 ? frame : y === 48 ? mid : light;
      ceil[y * CW + x] = c;
    }
  }
  const ceils = mips({ w: CW, h: CH, p: ceil });

  // ---- your shotgun, seen from behind it: a pump gun, your hands on it (your skin, your sleeves); the pump
  // apart from the rest, so it can go back and forth
  const INK = rgb("#1a1626");
  function sprite(w, h, paint) {
    const c = document.createElement("canvas"); c.width = w; c.height = h;
    const x = c.getContext("2d", { willReadFrequently: true });
    const r = (px, py, pw, ph, col) => { x.fillStyle = col; x.fillRect(px, py, pw, ph); };
    const poly = (col, ...pts) => { x.fillStyle = col; x.beginPath(); pts.forEach(([px, py], i) => (i ? x.lineTo(px, py) : x.moveTo(px, py))); x.fill(); };
    paint(r, poly);
    // hard pixels only (a polygon's edge comes out half covered), then an ink outline round them, as the room's figures have
    const d = x.getImageData(0, 0, w, h), p = new Uint32Array(d.data.buffer);
    for (let i = 0; i < p.length; i++) if (p[i] >>> 24 < 128) p[i] = 0; else p[i] |= 0xff000000;
    const q = p.slice(), on = (i, j) => i >= 0 && j >= 0 && i < w && j < h && q[j * w + i] >>> 24 > 128;
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) if (!on(i, j) && (on(i - 1, j) || on(i + 1, j) || on(i, j - 1) || on(i, j + 1))) p[j * w + i] = INK;
    x.putImageData(d, 0, 0);
    return c;
  }
  const { skin, clothes } = room.hands();
  const shadeHex = (hex, k) => "#" + [1, 3, 5].map((i) => Math.min(255, Math.round(parseInt(hex.slice(i, i + 2), 16) * k)).toString(16).padStart(2, "0")).join("");
  const GW = 96, GH = 96, GX = 48;
  const STEEL = ["#e3e6ec", "#b6bcc8", "#8a909c", "#5d6370", "#3c414b"], WOODS = ["#c48a52", "#a06a3a", "#7a4a2a", "#55301a"];
  const SKIN = [shadeHex(skin, 1.08), skin, shadeHex(skin, 0.8)], CLOTH = [shadeHex(clothes, 1.15), clothes, shadeHex(clothes, 0.72)];
  const gun = sprite(GW, GH, (r, poly) => {
    poly(CLOTH[1], [GX + 14, 82], [GX + 26, 76], [GW, 86], [GW, GH], [GX + 20, GH]);                         // your right sleeve
    poly(CLOTH[2], [GX + 34, 80], [GW, 86], [GW, GH], [GX + 44, GH]);
    r(GX - 14, 84, 28, 12, WOODS[1]); r(GX - 14, 84, 4, 12, WOODS[0]); r(GX + 9, 84, 5, 12, WOODS[2]);          // the stock
    r(GX - 12, 58, 24, 27, STEEL[3]); r(GX - 12, 58, 4, 27, STEEL[2]); r(GX - 11, 58, 1, 27, STEEL[1]); r(GX + 8, 58, 4, 27, STEEL[4]);   // the receiver
    r(GX - 8, 58, 16, 2, STEEL[2]); r(GX - 8, 58, 16, 1, STEEL[1]);                                              // its rail
    r(GX + 2, 65, 6, 11, "#15131c"); r(GX + 3, 74, 4, 1, STEEL[4]);                                              // its port
    for (let y = 0; y < 60; y++) {                                                                               // the barrel, wider as it nears you
      const hw = 3 + Math.floor(y / 22);
      r(GX - hw, y, 2 * hw, 1, STEEL[2]); r(GX - hw, y, 1, 1, STEEL[0]); r(GX - hw + 1, y, 2, 1, STEEL[1]); r(GX + hw - 2, y, 2, 1, STEEL[3]);
    }
    r(GX - 1, 0, 2, 2, "#f6f4ea");                                                                               // the bead
    r(GX + 9, 74, 14, 14, SKIN[1]); r(GX + 9, 74, 14, 2, SKIN[0]);                                               // your right hand on the grip
    for (let k = 0; k < 3; k++) r(GX + 10, 78 + k * 3, 12, 1, SKIN[2]);
  });
  const pump = sprite(GW, GH, (r, poly) => {
    poly(CLOTH[1], [GX - 24, 52], [GX - 12, 60], [GX - 26, GH], [0, GH], [0, 78]);                               // your left sleeve
    poly(CLOTH[0], [GX - 24, 52], [GX - 20, 55], [0, 84], [0, 78]);
    r(GX - 11, 34, 22, 28, WOODS[1]); r(GX - 11, 34, 4, 28, WOODS[0]); r(GX + 7, 34, 4, 28, WOODS[2]);           // the pump
    for (let y = 37; y < 61; y += 4) r(GX - 10, y, 20, 1, WOODS[3]);                                             // its grooves
    r(GX - 24, 40, 14, 18, SKIN[1]); r(GX - 24, 40, 14, 2, SKIN[0]); r(GX - 24, 56, 14, 2, SKIN[2]);             // your left hand round it
    for (let k = 0; k < 4; k++) { r(GX - 11, 41 + k * 4, 7, 3, SKIN[1]); r(GX - 11, 43 + k * 4, 7, 1, SKIN[2]); }   // its fingers over the wood
    r(GX - 13, 36, 6, 4, SKIN[1]);                                                                               // the thumb
  });
  const flash = sprite(40, 30, (r) => {
    r(13, 8, 14, 16, "#f2a531"); r(4, 13, 32, 6, "#f2a531"); r(17, 0, 6, 30, "#f2a531"); r(8, 6, 4, 4, "#f2a531"); r(28, 6, 4, 4, "#f2a531");
    r(15, 10, 10, 12, "#ffe27a"); r(9, 14, 22, 4, "#ffe27a"); r(18, 3, 4, 24, "#ffe27a");
    r(17, 13, 6, 6, "#fff6c0");
  });
  const PUFF = sprite(7, 7, (r) => { r(1, 1, 5, 5, "#c9ced8"); r(0, 2, 7, 3, "#c9ced8"); r(2, 0, 3, 7, "#c9ced8"); r(2, 2, 2, 2, "#eef0f4"); });
  PUFF.v = 1;

  // ---- you: where you stand, which way you look
  const me = room.start();
  let a = me.a, pitch = 0, walking = false, bob = 0, shot = -1e9, used = -1e9;   // pitch: how far the horizon is pushed down (you look up) or up
  const keys = new Set(), puffs = [];
  let sc = room.scene(), seen = new Map(), sv = -1;
  const blocked = (x, y) => x < R || x > sc.W - R || y < sc.WALL + R || y > sc.H - R ||
    sc.solid.some((r) => x > r[0] - R && x < r[2] + R && y > r[1] - R && y < r[3] + R);
  function stepYou(dt) {
    const turn = (keys.has("ArrowLeft") ? -1 : 0) + (keys.has("ArrowRight") ? 1 : 0);
    a += turn * 2.4 * dt;
    const fw = (keys.has("KeyW") || keys.has("ArrowUp") ? 1 : 0) - (keys.has("KeyS") || keys.has("ArrowDown") ? 1 : 0);
    const st = (keys.has("KeyD") ? 1 : 0) - (keys.has("KeyA") ? 1 : 0);
    const v = (keys.has("ShiftLeft") || keys.has("ShiftRight") ? 130 : 78) * dt, n = Math.hypot(fw, st) || 1;
    const dx = Math.cos(a), dy = Math.sin(a), mx = (fw * dx - st * dy) / n * v, my = (fw * dy + st * dx) / n * v;
    walking = !!(fw || st);
    if (walking) {
      const stuck = blocked(me.x, me.y);        // inside something (it was put down on you): walk out of it freely
      if (stuck || !blocked(me.x + mx, me.y)) me.x += mx;
      if (stuck || !blocked(me.x, me.y + my)) me.y += my;
      me.x = Math.max(R, Math.min(sc.W - R, me.x)); me.y = Math.max(sc.WALL + R, Math.min(sc.H - R, me.y));
      bob += dt * 9;
    }
    room.move(me.x, me.y, a, walking);
  }

  // ---- input: the mouse looks (the pointer locked to the view), a click fires, WASD walks, Esc leaves
  const MOVE = new Set(["KeyW", "KeyA", "KeyS", "KeyD", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ShiftLeft", "ShiftRight"]);
  let locked = false, dragging = null, nolock = false;   // nolock: the browser would not lock the pointer, a click fires and a drag looks
  const onKey = (e) => {
    e.stopPropagation();                      // nothing gets to the page (a terminal) while you are in here
    if (e.metaKey || e.ctrlKey) return;       // the browser's own shortcuts still work
    e.preventDefault();
    if (e.type === "keyup") { keys.delete(e.code); return; }
    if (e.key === "Escape") return leave();
    if (MOVE.has(e.code)) keys.add(e.code);
    if (e.code === "KeyE" || e.code === "Space") use();
    if (e.code === "Enter" || e.code === "KeyF") fire();
  };
  const onMove = (e) => {
    if (locked) { a += e.movementX * 0.0024; look(-e.movementY * 0.3); }
    else if (dragging) { a += (e.clientX - dragging[0]) * 0.006; look(-(e.clientY - dragging[1]) * 0.6); dragging = [e.clientX, e.clientY]; }
  };
  const onDown = (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    if (locked || nolock) fire();                // (no lock to be had: a click fires all the same)
    if (!locked) { dragging = [e.clientX, e.clientY]; lock(); }
  };
  const onUp = () => { dragging = null; };
  const look = (dy) => { pitch = Math.max(-RH * 0.4, Math.min(RH * 0.4, pitch + dy)); };
  const onLock = () => {
    const now = document.pointerLockElement === cv;
    if (locked && !now) leave();               // Esc, as the browser takes it while the pointer is locked
    locked = now;
    if (now) nolock = false;
  };
  function lock() { try { const p = cv.requestPointerLock(); if (p && p.catch) p.catch(() => { nolock = true; }); } catch (_) { nolock = true; } }
  addEventListener("keydown", onKey, true); addEventListener("keyup", onKey, true);
  addEventListener("mousemove", onMove); cv.addEventListener("mousedown", onDown); addEventListener("mouseup", onUp);
  document.addEventListener("pointerlockchange", onLock);
  const onLockError = () => { nolock = true; };
  document.addEventListener("pointerlockerror", onLockError);
  const blur = () => keys.clear();
  addEventListener("blur", blur);
  lock();

  // ---- the shotgun: the bot nearest you under the spread round the crosshair goes to pieces; a miss
  // puffs where it hits. Then the pump, before the next
  const COOL = 800;
  function fire() {
    const now = performance.now();
    if (now - shot < COOL) return;
    shot = now;
    const cx = RW >> 1, cy = RH >> 1;
    let best = null, bd = Infinity;
    for (let y = cy - 6; y <= cy + 6; y++) for (let x = cx - 10; x <= cx + 10; x++) {
      const i = y * RW + x, n = idb[i], th = n && drawn[n - 1];
      if (th && th.bot && !th.staff && zb[i] < bd) { best = th; bd = zb[i]; }
    }
    const d = Math.min(best ? bd : zb[cy * RW + cx], 2000) * 0.97;
    const hx = me.x + Math.cos(a) * d, hy = me.y + Math.sin(a) * d, hz = EYE + Math.round(pitch) * d / focal;
    if (!(best && room.shoot(best.bot))) puffs.push({ x: hx, y: hy, z: Math.max(1, hz), t0: now });
    room.alarm(me.x, me.y);
  }
  // E: the pieces under the crosshair are put back together
  function use() {
    const now = performance.now();
    if (now - used < 300) return;
    used = now;
    const th = near();
    if (th) room.unshoot(th.rem);
  }
  function near() {                           // the pieces under the crosshair, or the nearest in front of you within reach
    const th = under(4);
    if (th && th.rem) return th;
    let best = null, bd = 70;
    for (const t of sc.things) if (t.rem) {
      const rx = t.x - me.x, ry = t.y - me.y, d = Math.hypot(rx, ry) || 1;
      if (d < bd && (rx * Math.cos(a) + ry * Math.sin(a)) / d > 0.5) { best = t; bd = d; }
    }
    return best;
  }
  function under(r) {                         // the nearest thing round the crosshair
    const cx = RW >> 1, cy = RH >> 1;
    let best = null, bd = Infinity;
    for (let y = cy - r; y <= cy + r; y++) for (let x = cx - r; x <= cx + r; x++) {
      const i = y * RW + x, n = idb[i];
      if (n && zb[i] < bd) { best = drawn[n - 1]; bd = zb[i]; }
    }
    return best;
  }

  // ---- drawing
  let drawn = [];
  function render(now) {
    const s = room.scene();
    if (s !== sc) sc = s;
    const W = sc.W, H = sc.H, WALL = sc.WALL;
    const dx = Math.cos(a), dy = Math.sin(a), tf = Math.tan(FOV / 2), plx = -dy * tf, ply = dx * tf;
    const eye = EYE + (walking && !still ? Math.sin(bob * 2) * 0.5 : 0), hor = RH / 2 + Math.round(pitch);
    const px = me.x, py = me.y;
    const fls = mips(tex(sc.floor)), wl = tex(sc.wall), pl = tex(sc.plain);
    // the floor and the ceiling, a row at a time; far off from the smaller copies, as many dots a pixel as
    // the row covers
    for (let y = 0; y < RH; y++) {
      const below = y + 0.5 - hor, floorRow = below > 0, d = (floorRow ? eye : WALLH - eye) * focal / Math.abs(below), k = fogk(d);
      const sx = d * 2 * plx / RW, sy = d * 2 * ply / RW, fp = Math.max(Math.hypot(sx, sy), d / Math.abs(below));
      const lv = fp < 1.4 ? 0 : fp < 2.8 ? 1 : 2, m = 1 / (1 << lv), T = floorRow ? fls[lv] : ceils[lv], tw = T.w, th = T.h, tp = T.p;
      let wx = (px + d * (dx - plx) + sx / 2) * m, wy = (py + d * (dy - ply) + sy / 2) * m, i = y * RW;
      const ux = sx * m, uy = sy * m;
      for (let x = 0; x < RW; x++, i++, wx += ux, wy += uy) {
        let c;
        if (floorRow) { const tx = Math.floor(wx), ty = Math.floor(wy); c = tx >= 0 && ty >= 0 && tx < tw && ty < th ? tp[ty * tw + tx] : 0xff141018; }
        else c = tp[(((Math.floor(wy) % th) + th) % th) * tw + (((Math.floor(wx) % tw) + tw) % tw)];
        buf[i] = lit(c, k); zb[i] = d; idb[i] = 0;
      }
    }
    // the four walls: the far one is the room's own, with what hangs on it
    for (let x = 0; x < RW; x++) {
      const cam = 2 * (x + 0.5) / RW - 1, rx = dx + plx * cam, ry = dy + ply * cam;
      const tx = rx > 0 ? (W - px) / rx : rx < 0 ? -px / rx : 1e9, ty = ry > 0 ? (H - py) / ry : ry < 0 ? (WALL - py) / ry : 1e9;
      let t, u, T, side;
      if (ty < tx) { t = ty; u = px + rx * t; T = ry < 0 ? wl : pl; side = ry < 0 ? 1 : 0.88; if (ry > 0) u = W - u; }
      else { t = tx; u = py + ry * t; T = pl; side = 0.78; if (rx < 0) u = H - u; }
      const col = T === wl ? Math.max(0, Math.min(T.w - 1, Math.floor(u))) : ((Math.floor(u) % T.w) + T.w) % T.w;
      const top = hor - (WALLH - eye) * focal / t, bot = hor + eye * focal / t, k = fogk(t) * side;
      for (let y = Math.max(0, Math.ceil(top - 0.5)), y1 = Math.min(RH - 1, Math.floor(bot - 0.5)); y <= y1; y++) {
        const z = eye + (hor - y - 0.5) * t / focal, row = Math.max(0, Math.min(T.h - 1, Math.floor((WALLH - z) / WALLH * T.h))), i = y * RW + x;
        buf[i] = lit(T.p[row * T.w + col], k); zb[i] = t; idb[i] = 0;
      }
    }
    // the desks: blocks, the face the ray meets first, and their tops while you look down on them
    for (const b of sc.boxes) box(b, px, py, dx, dy, plx, ply, eye, hor);
    // the sprites; bots between two ticks of the room are on their way from one place to the next
    if (sc.v !== sv) {
      sv = sc.v;
      const next = new Map();
      for (const th of sc.things) if (th.key) {
        const o = seen.get(th.key), at = o ? place(o, now) : th;
        next.set(th.key, Math.hypot(at.x - th.x, at.y - th.y) > 30 ? { fx: th.x, fy: th.y, fz: th.z, tx: th.x, ty: th.y, tz: th.z, t0: now } : { fx: at.x, fy: at.y, fz: at.z, tx: th.x, ty: th.y, tz: th.z, t0: now });
      }
      seen = next;
    }
    drawn = [];
    const all = sc.things.concat(puffs.filter((p) => now - p.t0 < 350).map((p) => ({ x: p.x, y: p.y, z: p.z - 3, c: PUFF, sx: 0, sy: 0, sw: 7, sh: 7, ax: 3.5, ay: 7 })));
    for (let n = 0; n < all.length; n++) {
      const th = all[n], o = th.key && seen.get(th.key), at = o ? place(o, now) : th;
      drawn.push(th);
      sprite3(th, at, drawn.length, px, py, dx, dy, eye, hor);
    }
    ctx.putImageData(img, 0, 0);
    hud(now);
  }
  const place = (o, now) => { const k = Math.min(1, (now - o.t0) / TICK); return { x: o.fx + (o.tx - o.fx) * k, y: o.fy + (o.ty - o.fy) * k, z: o.fz + (o.tz - o.fz) * k }; };
  function face(f, u, v) { const T = tex(f.c); return T.p[(f.sy + Math.min(f.sh - 1, Math.floor(v * f.sh))) * T.w + f.sx + Math.min(f.sw - 1, Math.floor(u * f.sw))]; }
  function box(b, px, py, dx, dy, plx, ply, eye, hor) {
    if (px > b.x0 && px < b.x1 && py > b.y0 && py < b.y1) return;
    // the columns it can cover
    let x0 = RW, x1 = -1;
    for (const [cx, cy] of [[b.x0, b.y0], [b.x1, b.y0], [b.x0, b.y1], [b.x1, b.y1]]) {
      const rx = cx - px, ry = cy - py, d = rx * dx + ry * dy;
      if (d < 0.5) { x0 = 0; x1 = RW - 1; break; }
      const sx = RW / 2 + (rx * -dy + ry * dx) * focal / d;
      x0 = Math.min(x0, Math.floor(sx)); x1 = Math.max(x1, Math.ceil(sx));
    }
    x0 = Math.max(0, x0); x1 = Math.min(RW - 1, x1);
    const tf = Math.tan(FOV / 2);
    for (let x = x0; x <= x1; x++) {
      const cam = 2 * (x + 0.5) / RW - 1, rx = dx + plx * cam, ry = dy + ply * cam;
      let tn = -Infinity, tx = Infinity, ax = 0;
      if (rx) { const a1 = (b.x0 - px) / rx, a2 = (b.x1 - px) / rx; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 1; } tx = Math.min(tx, Math.max(a1, a2)); } else if (px < b.x0 || px > b.x1) continue;
      if (ry) { const a1 = (b.y0 - py) / ry, a2 = (b.y1 - py) / ry; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 2; } tx = Math.min(tx, Math.max(a1, a2)); } else if (py < b.y0 || py > b.y1) continue;
      if (tx < tn || tn <= 0.5) continue;
      // the side it meets, and where on it
      const hx = px + rx * tn, hy = py + ry * tn;
      let f, u, sh;
      if (ax === 1) { f = b.side; u = (hy - b.y0) / (b.y1 - b.y0); sh = 0.78; if (rx < 0) u = 1 - u; }
      else if (ry < 0) { f = b.front; u = (hx - b.x0) / (b.x1 - b.x0); sh = 0.94; }
      else { f = b.back; u = 1 - (hx - b.x0) / (b.x1 - b.x0); sh = 0.72; }
      u = Math.max(0, Math.min(0.999, u));
      const k = fogk(tn) * sh, top = hor - (b.z1 - eye) * focal / tn, bot = hor - (b.z0 - eye) * focal / tn;
      for (let y = Math.max(0, Math.ceil(top - 0.5)), y1 = Math.min(RH - 1, Math.floor(bot - 0.5)); y <= y1; y++) {
        const i = y * RW + x;
        if (tn >= zb[i]) continue;
        const z = eye + (hor - y - 0.5) * tn / focal, c = face(f, u, Math.max(0, Math.min(0.999, (b.z1 - z) / (b.z1 - b.z0))));
        if (c >>> 24 < 128) continue;
        buf[i] = lit(c, k); zb[i] = tn; idb[i] = 0;
      }
      if (eye <= b.z1) continue;
      // its top, from the far edge to the near one
      const h = eye - b.z1, ya = Math.max(0, Math.ceil(hor + h * focal / tx - 0.5)), yb = Math.min(RH - 1, Math.floor(hor + h * focal / tn - 0.5));
      for (let y = ya; y <= yb; y++) {
        const d = h * focal / (y + 0.5 - hor), i = y * RW + x;
        if (d < tn || d > tx || d >= zb[i]) continue;
        const c = face(b.top, Math.max(0, Math.min(0.999, (px + rx * d - b.x0) / (b.x1 - b.x0))), Math.max(0, Math.min(0.999, (py + ry * d - b.y0) / (b.y1 - b.y0))));
        if (c >>> 24 < 128) continue;
        buf[i] = lit(c, fogk(d)); zb[i] = d; idb[i] = 0;
      }
    }
  }
  function sprite3(th, at, n, px, py, dx, dy, eye, hor) {
    const rx = at.x - px, ry = at.y - py, d = rx * dx + ry * dy;
    if (d < 6) return;                        // right in your face (you walked into it): it would fill the view
    const s = rx * -dy + ry * dx, k = focal / d, T = tex(th.c);
    const left = RW / 2 + s * k - th.ax * k, top = hor - (at.z + th.ay - eye) * k;
    const x0 = Math.max(0, Math.floor(left)), x1 = Math.min(RW - 1, Math.ceil(left + th.sw * k) - 1);
    const y0 = Math.max(0, Math.floor(top)), y1 = Math.min(RH - 1, Math.ceil(top + th.sh * k) - 1);
    if (x0 > x1 || y0 > y1) return;
    // which way it faces, as you see it: the picture faces right, a figure facing the other way is flipped
    const flip = th.face ? th.face * -dy < 0 : !!th.flip;
    const L = fogk(d) * (th.dim || 1);
    for (let x = x0; x <= x1; x++) {
      const u = Math.floor((x + 0.5 - left) / k);
      if (u < 0 || u >= th.sw) continue;
      const col = th.sx + (flip ? th.sw - 1 - u : u);
      for (let y = y0; y <= y1; y++) {
        const v = Math.floor((y + 0.5 - top) / k);
        if (v < 0 || v >= th.sh) continue;
        const i = y * RW + x;
        if (d >= zb[i]) continue;
        const c = T.p[(th.sy + v) * T.w + col];
        if (c >>> 24 < 128) continue;
        buf[i] = lit(c, L); zb[i] = d; idb[i] = n;
      }
    }
  }
  // over it: the shotgun (bobbing as you walk, kicking and pumped after a shot), the crosshair, who is under it
  const t0 = performance.now();
  function hud(now) {
    const since = now - shot, cx = RW >> 1, cy = RH >> 1;
    const kick = since < 90 ? since / 90 : since < 320 ? 1 - (since - 90) / 230 : 0;
    const pumpY = since > 380 && since < 720 ? Math.sin((since - 380) / 340 * Math.PI) * 9 : 0;
    const bx = walking && !still ? Math.sin(bob) * 5 : 0, by = walking && !still ? Math.abs(Math.cos(bob)) * 4 : 0;
    const gx = Math.round(cx - GX + bx), gy = Math.round(RH - GH + 8 + by + kick * 10);
    if (since < 70) {                          // the flash lights the room a moment
      ctx.fillStyle = "rgba(255,226,150,0.14)"; ctx.fillRect(0, 0, RW, RH);
      ctx.drawImage(flash, gx + GX - 20, gy - 24);
    }
    ctx.drawImage(gun, gx, gy); ctx.drawImage(pump, gx, Math.round(gy + pumpY));
    const th = under(2), bot = th && th.bot && !th.staff, col = bot ? "#ff6b5a" : "#e8e8ee";
    ctx.fillStyle = "rgba(10,8,16,0.6)";
    for (const [x, y, w, h] of [[cx - 5, cy, 3, 1], [cx + 3, cy, 3, 1], [cx, cy - 5, 1, 3], [cx, cy + 3, 1, 3]]) ctx.fillRect(x + 1, y + 1, w, h);
    ctx.fillStyle = col;
    for (const [x, y, w, h] of [[cx - 5, cy, 3, 1], [cx + 3, cy, 3, 1], [cx, cy - 5, 1, 3], [cx, cy + 3, 1, 3]]) ctx.fillRect(x, y, w, h);
    const tag = (text, y, c) => {
      const w = room.tinyW(text), x = Math.round(cx - w / 2);
      ctx.fillStyle = "rgba(10,8,16,0.75)"; ctx.fillRect(x - 2, y - 2, w + 4, 9);
      room.tiny(ctx, text, x, y, c);
    };
    const rm = near(), who = bot ? th : rm || (th && th.name ? th : null);
    if (who) tag(who.name, 18, bot ? "#ffb4a8" : "#e8e8ee");                  // who is in your sights, up top (not over their face)
    if (!bot && rm) tag("E: put back together", 28, "#e8e8ee");
    if (now - t0 < 6000) tag("wasd walk - mouse look - click shoot - esc leave", 6, "#c9ced8");
  }

  let raf = 0, last = performance.now(), ms = 0;   // ms: how long a frame takes to draw, smoothed
  function frame(now) {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(0.05, (now - last) / 1000); last = now;
    stepYou(dt);
    const t = performance.now();
    render(now);
    ms = ms * 0.95 + (performance.now() - t) * 0.05;
  }
  raf = requestAnimationFrame(frame);

  function leave() {
    cancelAnimationFrame(raf);
    removeEventListener("keydown", onKey, true); removeEventListener("keyup", onKey, true);
    removeEventListener("mousemove", onMove); removeEventListener("mouseup", onUp); removeEventListener("resize", resize); removeEventListener("blur", blur);
    document.removeEventListener("pointerlockchange", onLock); document.removeEventListener("pointerlockerror", onLockError);
    if (document.pointerLockElement === cv) document.exitPointerLock();
    cv.remove();
    window.__fps = null;
    done();
  }
  // for look.mjs: where you are, turn, step, fire, use, without a mouse
  window.__fps = { me, look, ms: () => ms, turn: (r) => { a += r; }, set: (x, y, ang) => { me.x = x; me.y = y; if (ang !== undefined) a = ang; }, fire: () => { shot = -1e9; fire(); }, use, leave, under: () => { const t = under(2); return t && { bot: t.bot, rem: t.rem, name: t.name, x: t.x, y: t.y, w: t.sw, h: t.sh }; } };
  return { leave };
}
