// The gun closet, hardcore: the office in the first person, as Doom drew its bases. The whole window is the
// view: the room as it is, raycast. Its floor (planks, and the rug, the mats rolled out, the oil, the fire's pit
// on them), its wall with what hangs on it (the doors, the window, the clock, the closet, the switchboard), the
// other three walls plain, a ceiling of tiles and lamps; the desks and the furniture as blocks, what is round as
// pictures, the bots, the office managers and the dog as models seen from whichever of their eight sides faces
// you (all of it props3d.js's, made for this view: no ink outlines, four texels a dot). The light is Doom's:
// what is far sinks into the dark, pools of light under the lamps, the fire flickers on everything near it, a
// shot lights the room. You hold a shotgun, Doom's way: a click shoots (a bot shot goes to pieces, in the room
// as well: the remains, the sweepers, the undo), then the pump; E on the pieces puts it back together, E on the
// gun closet loads the gun again. Esc leaves; you stand in the room where you left off. Going in and out the
// screen melts, as Doom's did between its screens.
//
// Under the view Doom's status bar: AMMO (your shells), HEALTH (what is left of the five-hour limit, the coffee
// pot's), BOTS where Doom has ARMS (at work, waiting for you, idle, done), your face, ARMOR (what is left of the
// week, the cooler's bottle), the doors as keys (lit while their machine answers), and where Doom lists its
// ammo, the tokens left: every limit /usage has, out of a hundred.
//
// startFps(room, done): room = { scene(), start(), move(x, y, a, walking), shoot(id), unshoot(id), alarm(x, y),
// armed(), use(thing), label(thing), status(), face(), tiny(ctx, s, x, y, col), tinyW(s), hands(), snapshot() }.
// scene() is the room at its last tick (twelve a second):
//   { v, W, H, WALL, floor: the floor's overlay (2 texels a dot), wall, plain: the walls' pictures (TS a dot),
//     boxes: [{x0, y0, x1, y1, z0, z1, front, back, side, top, west?, east?}]  a face: {c, sx, sy, sw, sh, glow}
//     things: [{key, x, y, z, c, sx, sy, sw, sh, ax, ay, res, flip, bot, staff, rem, name, piece, face, fig,
//               seated, dissolve}]  a picture standing at x, y (its column ax there), its row ay at height z, res
//               texels a dot; one with a `fig` is a model, drawn from the side that faces you
//     solid: [[x0, y0, x1, y1]], lights: [{x, y, z, r, col, k, flicker}] }
// A canvas redrawn in place says so with its `v`.
function startFps(room, done) {
  const BAR = 48, FOV = 66 * Math.PI / 180, EYE = 21, WALLH = 40, R = 4, TICK = 1000 / 12, SHELLS = 8;
  const TS = Props3D.TS, GLOW = Props3D.GLOW, BW = 480;   // the bar is laid out for 480 pixels, as Doom's for 320
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const cv = document.createElement("canvas");
  cv.id = "fps"; cv.tabIndex = -1;
  document.body.append(cv);
  if (document.activeElement && document.activeElement.blur) document.activeElement.blur();   // keys are the view's now, not a terminal's
  const ctx = cv.getContext("2d");
  let RW = 0, RH = 0, VIEW = 0, img, buf, zb, idb, focal;
  function resize() {                         // square pixels: 480 to 640 across, as tall as the window's shape says
    const asp = innerWidth / Math.max(1, innerHeight);
    RW = Math.max(BW, Math.min(640, Math.round(300 * asp))); RH = Math.max(260, Math.round(RW / asp)); VIEW = RH - BAR;
    cv.width = RW; cv.height = RH;
    ctx.imageSmoothingEnabled = false;
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
  // smaller copies of a picture, each pixel the mean of four (what shines stays shining): far off the planks'
  // seams would shimmer in rings
  function mips(t) {
    if (t.mips) return t.mips;
    const out = [t];
    for (let k = 1; k < 4; k++) {
      const a = out[k - 1], w = a.w >> 1, h = a.h >> 1, p = new Uint32Array(w * h);
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
        const i = 2 * y * a.w + 2 * x, c0 = a.p[i], c1 = a.p[i + 1], c2 = a.p[i + a.w], c3 = a.p[i + a.w + 1];
        const r = (c0 & 255) + (c1 & 255) + (c2 & 255) + (c3 & 255), g = (c0 >>> 8 & 255) + (c1 >>> 8 & 255) + (c2 >>> 8 & 255) + (c3 >>> 8 & 255), b = (c0 >>> 16 & 255) + (c1 >>> 16 & 255) + (c2 >>> 16 & 255) + (c3 >>> 16 & 255);
        const glow = (c0 >>> 24 === GLOW) + (c1 >>> 24 === GLOW) + (c2 >>> 24 === GLOW) + (c3 >>> 24 === GLOW) >= 2;
        p[y * w + x] = (glow ? GLOW : 255) << 24 | (b >> 2) << 16 | (g >> 2) << 8 | r >> 2;
      }
      out.push({ w, h, p });
    }
    return (t.mips = out);
  }
  const FL = mips(tex(Props3D.floorTile())), CE = mips(tex(Props3D.ceilTile()));

  // ---- the light at a point: the room's own, a pool under each of the ceiling's lamps (64 x 96 dots apart, as
  // its tiles), darker in the corners; the lights the scene has (the fire flickering, a booth in use); a shot's
  // flash. Then, by distance, Doom's dark
  let LT = [], flashK = 0;
  const lr = new Float32Array(3);
  function light(x, y, z) {
    const dx = ((x % 64) + 64) % 64 - 32, dy = ((y % 96) + 96) % 96 - 48, q = 1 - (dx * dx / 1100 + dy * dy / 700);
    let k = 0.58 + (q > 0 ? 0.36 * q * q : 0);
    const e = Math.min(x, sc.W - x, y - sc.WALL, sc.H - y);
    if (e < 16) k *= 0.68 + 0.32 * Math.max(0, e) / 16;
    let r = k + flashK, g = k + flashK, b = k + flashK * 0.8;
    for (let n = 0; n < LT.length; n++) {
      const L = LT[n], ax = x - L.x, ay = y - L.y, az = z - L.z, d2 = ax * ax + ay * ay + az * az;
      if (d2 >= L.r2) continue;
      const f = 1 - Math.sqrt(d2) / L.r, a = f * f * L.k;
      r += a * L.c0; g += a * L.c1; b += a * L.c2;
    }
    lr[0] = r; lr[1] = g; lr[2] = b;
  }
  const dimOf = (d) => Math.max(0.08, Math.min(1, 1.2 - d / 360));
  function shade(c, k) {                      // a texel under the light in lr, k the distance's dimming
    const a = c >>> 24;
    if (a === GLOW) return c | 0xff000000;    // it shines
    const r = (c & 255) * lr[0] * k, g = ((c >>> 8) & 255) * lr[1] * k, b = ((c >>> 16) & 255) * lr[2] * k;
    return 0xff000000 | (b > 255 ? 255 : b) << 16 | (g > 255 ? 255 : g) << 8 | (r > 255 ? 255 : r);
  }

  // ---- you: where you stand, which way you look
  const me = room.start();
  let a = me.a, pitch = 0, walking = false, bob = 0, shot = -1e9, used = -1e9, shells = room.armed() ? SHELLS : 0, armedWas = room.armed(), kills0 = (room.status() || {}).kills || 0, grin = -1e9;   // pitch: how far the horizon is pushed down (you look up) or up
  const keys = new Set(), puffs = [], said = [];
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
    if (e.code === "Enter" || e.code === "KeyF") (room.armed() ? fire : use)();
  };
  const onMove = (e) => {
    if (locked) { a += e.movementX * 0.0024; look(-e.movementY * 0.3); }
    else if (dragging) { a += (e.clientX - dragging[0]) * 0.006; look(-(e.clientY - dragging[1]) * 0.6); dragging = [e.clientX, e.clientY]; }
  };
  const onDown = (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    if (locked || nolock) (room.armed() ? fire : use)();   // (no lock to be had: a click fires all the same)
    if (!locked) { dragging = [e.clientX, e.clientY]; lock(); }
  };
  const onUp = () => { dragging = null; };
  const look = (dy) => { pitch = Math.max(-VIEW * 0.4, Math.min(VIEW * 0.4, pitch + dy)); };
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

  // ---- the shotgun: the bot nearest you under the spread round the crosshair goes to pieces; a miss puffs where
  // it hits. Then the pump, before the next
  const COOL = 900;
  function fire() {
    const now = performance.now();
    if (!room.armed()) return;
    if (now - shot < COOL || !shells) return;   // (out of shells: the gun closet has more, E on it)
    shot = now; shells--;
    const cx = RW >> 1, cy = VIEW >> 1;
    let best = null, bd = Infinity;
    for (let y = cy - 9; y <= cy + 9; y++) for (let x = cx - 14; x <= cx + 14; x++) {
      const i = y * RW + x, n = idb[i], th = n && drawn[n - 1];
      if (th && th.bot && !th.staff && zb[i] < bd) { best = th; bd = zb[i]; }
    }
    const d = Math.min(best ? bd : zb[cy * RW + cx], 2000) * 0.97;
    const hx = me.x + Math.cos(a) * d, hy = me.y + Math.sin(a) * d, hz = EYE + Math.round(pitch) * d / focal;
    if (best && room.shoot(best.bot)) said.push({ text: (best.name || "a bot").toUpperCase() + " IS DELETED.", t0: now });
    else puffs.push({ x: hx, y: hy, z: Math.max(1, hz), t0: now });
    room.alarm(me.x, me.y);
  }
  // E (or a click with nothing in your hands): the pieces under the crosshair are put back together, or what is
  // under it is used (room.use); else the nearest pieces in front of you
  function use() {
    const now = performance.now();
    if (now - used < 300) return;
    used = now;
    const th = under(3);
    if (th && th.rem) return room.unshoot(th.rem);
    if (th && room.use(th) === "reload") { shells = SHELLS; said.push({ text: "PICKED UP SOME SHELLS.", t0: now }); return; }
    const r = near();
    if (r) room.unshoot(r.rem);
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
    const cx = RW >> 1, cy = VIEW >> 1;
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
    const since = now - shot;
    flashK = room.armed() && since < 160 ? 0.7 * (1 - since / 160) : 0;
    LT = (sc.lights || []).map((L) => {
      const f = L.flicker && !still ? 0.84 + 0.09 * Math.sin(now / 83) + 0.06 * Math.sin(now / 31 + 1.3) + 0.04 * Math.sin(now / 13 + 0.4) : 1;
      return { x: L.x, y: L.y, z: L.z, r: L.r, r2: L.r * L.r, k: L.k * f, c0: L.col[0], c1: L.col[1], c2: L.col[2] };
    });
    const dx = Math.cos(a), dy = Math.sin(a), tf = Math.tan(FOV / 2), plx = -dy * tf, ply = dx * tf;
    const eye = EYE + (walking && !still ? Math.sin(bob * 2) * 0.5 : 0), hor = VIEW / 2 + Math.round(pitch);
    const px = me.x, py = me.y;
    const OV = tex(sc.floor), wl = tex(sc.wall), pl = tex(sc.plain);
    // the floor and the ceiling, a row at a time; far off from the smaller copies, as many texels a pixel as the
    // row covers. The floor's planks, and on them what the overlay has
    for (let y = 0; y < VIEW; y++) {
      const below = y + 0.5 - hor, floorRow = below > 0, d = (floorRow ? eye : WALLH - eye) * focal / Math.abs(below), dk = dimOf(d);
      const sx = d * 2 * plx / RW, sy = d * 2 * ply / RW, fp = Math.max(Math.hypot(sx, sy), d / Math.abs(below)) * TS;
      const lv = fp < 1.4 ? 0 : fp < 2.8 ? 1 : fp < 5.6 ? 2 : 3, m = TS / (1 << lv), T = floorRow ? FL[lv] : CE[lv], tw = T.w, th = T.h, tp = T.p;
      let wx = px + d * (dx - plx) + sx / 2, wy = py + d * (dy - ply) + sy / 2, i = y * RW;
      for (let x = 0; x < RW; x++, i++, wx += sx, wy += sy) {
        const tx = Math.floor(wx * m), ty = Math.floor(wy * m);
        let c = tp[(((ty % th) + th) % th) * tw + (((tx % tw) + tw) % tw)];
        if (floorRow) {
          const ox = Math.floor(wx * 2), oy = Math.floor(wy * 2);
          if (ox >= 0 && oy >= 0 && ox < OV.w && oy < OV.h) {
            const o = OV.p[oy * OV.w + ox], oa = o >>> 24;
            if (oa >= 250 && oa < 255) c = (o & 0xffffff) | GLOW << 24;
            else if (oa) {
              const k = oa / 255, j = 1 - k;
              c = 0xff000000 | (((c >>> 16) & 255) * j + ((o >>> 16) & 255) * k) << 16 | (((c >>> 8) & 255) * j + ((o >>> 8) & 255) * k) << 8 | ((c & 255) * j + (o & 255) * k);
            }
          }
          light(wx, wy, 0);
        } else { light(wx, wy, WALLH); lr[0] *= 0.82; lr[1] *= 0.82; lr[2] *= 0.82; }
        buf[i] = shade(c, dk); zb[i] = d; idb[i] = 0;
      }
    }
    // the four walls: the far one is the room's own, with what hangs on it
    for (let x = 0; x < RW; x++) {
      const cam = 2 * (x + 0.5) / RW - 1, rx = dx + plx * cam, ry = dy + ply * cam;
      const tx = rx > 0 ? (W - px) / rx : rx < 0 ? -px / rx : 1e9, ty = ry > 0 ? (H - py) / ry : ry < 0 ? (WALL - py) / ry : 1e9;
      let t, u, T, side, hx, hy;
      if (ty < tx) { t = ty; u = px + rx * t; T = ry < 0 ? wl : pl; side = ry < 0 ? 1 : 0.9; hx = u; hy = ry < 0 ? WALL : H; if (ry > 0) u = W - u; }
      else { t = tx; u = py + ry * t; T = pl; side = 0.82; hx = rx > 0 ? W : 0; hy = u; if (rx < 0) u = H - u; }
      const col = T === wl ? Math.max(0, Math.min(T.w - 1, Math.floor(u * TS))) : ((Math.floor(u * TS) % T.w) + T.w) % T.w;
      const top = hor - (WALLH - eye) * focal / t, bot = hor + eye * focal / t, dk = dimOf(t) * side;
      light(hx - (hx > W / 2 ? 2 : -2), hy - (hy > H / 2 ? 2 : -2), 14);
      const l0 = lr[0], l1 = lr[1], l2 = lr[2];
      for (let y = Math.max(0, Math.ceil(top - 0.5)), y1 = Math.min(VIEW - 1, Math.floor(bot - 0.5)); y <= y1; y++) {
        const z = eye + (hor - y - 0.5) * t / focal, row = Math.max(0, Math.min(T.h - 1, Math.floor((WALLH - z) * TS))), i = y * RW + x;
        const kz = z < 5 ? 0.62 + z * 0.076 : 1;            // darker where it meets the floor
        lr[0] = l0 * kz; lr[1] = l1 * kz; lr[2] = l2 * kz;
        buf[i] = shade(T.p[row * T.w + col], dk); zb[i] = t; idb[i] = 0;
      }
    }
    // the blocks: the face the ray meets first, and their tops while you look down on them
    for (const b of sc.boxes) box(b, px, py, dx, dy, plx, ply, eye, hor);
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
    drawn = [];
    const all = sc.things.concat(puffs.filter((p) => now - p.t0 < 400).map((p) => ({ x: p.x, y: p.y, z: p.z - 3, c: PUFF, sx: 0, sy: 0, sw: PUFF.width, sh: PUFF.height, ax: PUFF.width / 2, ay: PUFF.height, res: 4 })));
    for (let n = 0; n < all.length; n++) {
      const th = all[n], o = th.key && seen.get(th.key), at = o ? place(o, now) : th;
      drawn.push(th);
      sprite3(th, at, drawn.length, px, py, dx, dy, eye, hor, o);
    }
    ctx.putImageData(img, 0, 0);
    hud(now);
  }
  const place = (o, now) => { const k = Math.min(1, (now - o.t0) / TICK); return { x: o.fx + (o.tx - o.fx) * k, y: o.fy + (o.ty - o.fy) * k, z: o.fz + (o.tz - o.fz) * k }; };
  function face(f, u, v) { const T = tex(f.c); return T.p[(f.sy + Math.min(f.sh - 1, Math.floor(v * f.sh))) * T.w + f.sx + Math.min(f.sw - 1, Math.floor(u * f.sw))]; }
  function box(b, px, py, dx, dy, plx, ply, eye, hor) {
    if (px > b.x0 && px < b.x1 && py > b.y0 && py < b.y1) return;
    let x0 = RW, x1 = -1;                      // the columns it can cover
    for (const [cx, cy] of [[b.x0, b.y0], [b.x1, b.y0], [b.x0, b.y1], [b.x1, b.y1]]) {
      const rx = cx - px, ry = cy - py, d = rx * dx + ry * dy;
      if (d < 0.5) { x0 = 0; x1 = RW - 1; break; }
      const sx = RW / 2 + (rx * -dy + ry * dx) * focal / d;
      x0 = Math.min(x0, Math.floor(sx)); x1 = Math.max(x1, Math.ceil(sx));
    }
    x0 = Math.max(0, x0); x1 = Math.min(RW - 1, x1);
    for (let x = x0; x <= x1; x++) {
      const cam = 2 * (x + 0.5) / RW - 1, rx = dx + plx * cam, ry = dy + ply * cam;
      let tn = -Infinity, tx = Infinity, ax = 0;
      if (rx) { const a1 = (b.x0 - px) / rx, a2 = (b.x1 - px) / rx; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 1; } tx = Math.min(tx, Math.max(a1, a2)); } else if (px < b.x0 || px > b.x1) continue;
      if (ry) { const a1 = (b.y0 - py) / ry, a2 = (b.y1 - py) / ry; if (Math.min(a1, a2) > tn) { tn = Math.min(a1, a2); ax = 2; } tx = Math.min(tx, Math.max(a1, a2)); } else if (py < b.y0 || py > b.y1) continue;
      if (tx < tn || tn <= 0.5) continue;
      const hx = px + rx * tn, hy = py + ry * tn;     // the side it meets, and where on it
      let f, u, sh;
      if (ax === 1) { f = (rx > 0 ? b.west : b.east) || b.side; u = (hy - b.y0) / (b.y1 - b.y0); sh = 0.8; if (rx < 0) u = 1 - u; }
      else if (ry < 0) { f = b.front; u = (hx - b.x0) / (b.x1 - b.x0); sh = 0.95; }
      else { f = b.back; u = 1 - (hx - b.x0) / (b.x1 - b.x0); sh = 0.72; }
      u = Math.max(0, Math.min(0.999, u));
      const dk = dimOf(tn) * sh, top = hor - (b.z1 - eye) * focal / tn, bot = hor - (b.z0 - eye) * focal / tn;
      light(hx, hy, (b.z0 + b.z1) / 2);
      for (let y = Math.max(0, Math.ceil(top - 0.5)), y1 = Math.min(VIEW - 1, Math.floor(bot - 0.5)); y <= y1; y++) {
        const i = y * RW + x;
        if (tn >= zb[i]) continue;
        const z = eye + (hor - y - 0.5) * tn / focal, c = face(f, u, Math.max(0, Math.min(0.999, (b.z1 - z) / (b.z1 - b.z0))));
        if (c >>> 24 < 128) continue;
        buf[i] = f.glow ? c | 0xff000000 : shade(c, dk); zb[i] = tn; idb[i] = 0;
      }
      if (eye <= b.z1) continue;
      // its top, from the far edge to the near one
      const h = eye - b.z1, ya = Math.max(0, Math.ceil(hor + h * focal / tx - 0.5)), yb = Math.min(VIEW - 1, Math.floor(hor + h * focal / tn - 0.5));
      for (let y = ya; y <= yb; y++) {
        const d = h * focal / (y + 0.5 - hor), i = y * RW + x;
        if (d < tn || d > tx || d >= zb[i]) continue;
        const wx = px + rx * d, wy = py + ry * d, c = face(b.top, Math.max(0, Math.min(0.999, (wx - b.x0) / (b.x1 - b.x0))), Math.max(0, Math.min(0.999, (wy - b.y0) / (b.y1 - b.y0))));
        if (c >>> 24 < 128) continue;
        light(wx, wy, b.z1);
        buf[i] = b.top.glow ? c | 0xff000000 : shade(c, dimOf(d) * 1.05); zb[i] = d; idb[i] = 0;
      }
    }
  }
  // a picture or a model, standing where it stands. A model is seen from the side of its eight that faces you:
  // the way it walks (standing, the way it looks) against the way to you
  const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
  function sprite3(th, at, n, px, py, dx, dy, eye, hor, o) {
    const rx = at.x - px, ry = at.y - py, d = rx * dx + ry * dy;
    if (d < 6) return;                        // right in your face (you walked into it): it would fill the view
    let pic = th.c, sx0 = th.sx, sy0 = th.sy, sw = th.sw, sh = th.sh, ax = th.ax, ay = th.ay, res = th.res || 1;
    let flip = th.face && !th.fig ? th.face * -dy < 0 : !!th.flip;
    if (th.fig) {
      const mx = o ? o.tx - o.fx : 0, my = o ? o.ty - o.fy : 0;
      const fa = th.facing !== undefined ? th.facing : !th.seated && Math.hypot(mx, my) > 0.4 ? Math.atan2(my, mx) : th.face < 0 ? Math.PI : 0;
      const k = ((Math.round((Math.atan2(-ry, -rx) - fa) / (Math.PI / 4)) % 8) + 8) % 8;
      const f = Props3D.figure(th.fig, k);
      pic = f.c; sx0 = 0; sy0 = 0; sw = f.w; sh = f.h; ax = f.ax; ay = f.ay; res = f.res; flip = false;
    }
    const s = rx * -dy + ry * dx, kd = focal / d, k = kd / res, T = tex(pic);
    const left = RW / 2 + s * kd - ax * k, top = hor - (at.z - eye) * kd - ay * k;
    const x0 = Math.max(0, Math.floor(left)), x1 = Math.min(RW - 1, Math.ceil(left + sw * k) - 1);
    const y0 = Math.max(0, Math.floor(top)), y1 = Math.min(VIEW - 1, Math.ceil(top + sh * k) - 1);
    if (x0 > x1 || y0 > y1) return;
    light(at.x, at.y, at.z + 12);
    const dk = dimOf(d) * (th.dim || 1), dis = th.dissolve || 0;
    for (let x = x0; x <= x1; x++) {
      const u = Math.floor((x + 0.5 - left) / k);
      if (u < 0 || u >= sw) continue;
      const col = sx0 + (flip ? sw - 1 - u : u);
      for (let y = y0; y <= y1; y++) {
        const v = Math.floor((y + 0.5 - top) / k);
        if (v < 0 || v >= sh) continue;
        const i = y * RW + x;
        if (d >= zb[i]) continue;
        let c = T.p[(sy0 + v) * T.w + col];
        if (c >>> 24 < 128) continue;
        if (dis) {                            // going up in the booth's light: from its feet, thinning out
          const am = Math.max(0, Math.min(1, (dis * 1.4 - (sh - 1 - v) / (sh - 1)) / 0.4));
          if (am && BAYER[(y & 3) * 4 + (x & 3)] / 16 < am) continue;
          if (am) { const m = Math.min(1, am * 1.6) * 0.85; c = GLOW << 24 | ((c >>> 16 & 255) + (214 - (c >>> 16 & 255)) * m) << 16 | ((c >>> 8 & 255) + (244 - (c >>> 8 & 255)) * m) << 8 | ((c & 255) + (255 - (c & 255)) * m); }
        }
        buf[i] = shade(c, dk); zb[i] = d; idb[i] = n;
      }
    }
  }

  // ---- the shotgun, as Doom's: seen from behind it, down the barrel toward the middle of the view, the pump on
  // it, the receiver nearest you. A shot: the flash at the muzzle (it lights the room), a kick; then the pump:
  // the gun dips and turns, the pump comes back and goes forward again. Lit by the light where you stand
  function crisp(c) {                         // hard edges: a texel is there or not
    const x = c.getContext("2d", { willReadFrequently: true }), d = x.getImageData(0, 0, c.width, c.height), p = d.data;
    for (let i = 3; i < p.length; i += 4) p[i] = p[i] < 110 ? 0 : 255;
    x.putImageData(d, 0, 0);
    return c;
  }
  function paint(w, h, fn) { const c = document.createElement("canvas"); c.width = w; c.height = h; fn(c.getContext("2d"), w, h); return crisp(c); }
  const across = (g, x0, x1, stops) => { const gr = g.createLinearGradient(x0, 0, x1, 0); stops.forEach(([k, col]) => gr.addColorStop(k, col)); return gr; };
  const trap = (g, cx, y0, w0, y1, w1, fill) => { g.fillStyle = fill; g.beginPath(); g.moveTo(cx - w0, y0); g.lineTo(cx + w0, y0); g.lineTo(cx + w1, y1); g.lineTo(cx - w1, y1); g.closePath(); g.fill(); };
  const GW = 210, GH = 170, GC = 105;
  const gun = paint(GW, GH, (g) => {
    const STEEL = [[0, "#1d1e24"], [0.28, "#3b3e48"], [0.5, "#8d93a0"], [0.62, "#575b66"], [1, "#22232a"]];
    trap(g, GC, 2, 7, 112, 15, across(g, GC - 15, GC + 15, STEEL));               // the barrel, narrowing away from you
    g.fillStyle = "#c9ced8"; g.fillRect(GC - 1.5, 4, 3, 70);                        // the light along its top
    g.fillStyle = "#f6f4ea"; g.fillRect(GC - 2, 0, 4, 5);                           // the bead
    trap(g, GC, 106, 26, 170, 42, across(g, GC - 42, GC + 42, [[0, "#15161b"], [0.3, "#33363f"], [0.48, "#6e7380"], [0.6, "#3c3f49"], [1, "#16171c"]]));   // the receiver
    g.fillStyle = "#0c0c10"; g.fillRect(GC + 9, 122, 14, 26);                       // its port
    g.fillStyle = "#9aa0ad"; g.fillRect(GC + 9, 122, 14, 2);
    g.fillStyle = "#26272e"; g.fillRect(GC - 6, 107, 12, 8); g.fillStyle = "#0c0c10"; g.fillRect(GC - 2, 107, 4, 5);   // the rear sight's notch
  });
  const pump = paint(GW, GH, (g) => {
    const WOOD = [[0, "#3a2414"], [0.25, "#6e4422"], [0.5, "#b07a42"], [0.66, "#7c4c26"], [1, "#3a2414"]];
    trap(g, GC, 52, 13, 96, 19, across(g, GC - 19, GC + 19, WOOD));                // the pump round the tube under the barrel
    g.fillStyle = "rgba(30,18,10,0.8)";
    for (const y of [62, 71, 80, 89]) { const hw = 13 + (y - 52) / 44 * 6; g.fillRect(GC - hw + 2, y, hw * 2 - 4, 2.4); }   // its grooves
    trap(g, GC, 52, 13, 55, 13.4, "#c99058");                                       // its lit front edge
  });
  const flashes = [0, 1].map((k) => paint(70, 56, (g) => {        // the muzzle flash: a ragged star, white at the heart
    const cx = 35, cy = 36, rays = k ? 7 : 9, R0 = k ? 18 : 25;
    for (const [rr, col] of [[1, "#f2a531"], [0.68, "#ffd75e"], [0.4, "#fff3c0"]]) {
      g.fillStyle = col; g.beginPath();
      for (let i = 0; i <= rays * 2; i++) { const an = (i / (rays * 2)) * Math.PI * 2 + k, rad = (i % 2 ? 0.45 : 1) * R0 * rr * (0.85 + ((i * 37) % 7) / 20); g.lineTo(cx + Math.cos(an) * rad, cy + Math.sin(an) * rad * 0.8); }
      g.fill();
    }
  }));
  const PUFF = paint(28, 28, (g) => { for (const [x, y, r, c] of [[14, 15, 10, "#8d929c"], [10, 12, 6, "#b9bec8"], [17, 11, 5, "#c9ced8"], [14, 14, 3, "#eef0f4"]]) { g.fillStyle = c; g.beginPath(); g.arc(x, y, r, 0, 7); g.fill(); } });
  PUFF.v = 1;

  // ---- over the view: the gun, the crosshair, what is in your sights; Doom's messages up left
  let t0 = performance.now();
  function hud(now) {
    const armed = room.armed();
    if (armed && !armedWas) shells = SHELLS;    // a gun from the gun closet: loaded
    armedWas = armed;
    const since = now - shot, cx = RW >> 1, cy = VIEW >> 1;
    if (armed) {
      const kick = since < 60 ? -7 : since < 140 ? -7 * (1 - (since - 60) / 80) : 0;
      const p = since > 240 && since < 860 ? (since - 240) / 620 : 0, dip = p ? Math.sin(Math.PI * p) : 0;
      const back = p > 0.22 && p < 0.78 ? Math.sin(Math.PI * (p - 0.22) / 0.56) * 18 : 0;
      const bx = walking && !still ? Math.sin(bob) * 10 : 0, by = walking && !still ? Math.abs(Math.cos(bob)) * 8 : 0;
      const gx = Math.round(cx - GC + bx + dip * 18), gy = Math.round(VIEW - GH + 34 + by + kick + dip * 52);
      light(me.x, me.y, EYE);
      const lit = Math.max(0.35, Math.min(1.5, (lr[0] + lr[1] + lr[2]) / 3));
      ctx.save();
      ctx.filter = since < 130 ? "none" : `brightness(${lit.toFixed(2)})`;
      ctx.translate(gx + GC, gy + GH); ctx.rotate(dip * 0.16); ctx.translate(-GC, -GH);
      if (since < 130) ctx.drawImage(flashes[since < 60 ? 0 : 1], GC - 35, -46);
      ctx.drawImage(gun, 0, 0);
      ctx.drawImage(pump, 0, Math.round(back));
      ctx.restore();
    }
    const th = under(2), bot = armed && th && th.bot && !th.staff, what = !armed && th && room.label(th);
    const col = bot ? "#ff6b5a" : "#e8e8ee";
    ctx.fillStyle = "rgba(10,8,16,0.6)";
    for (const [x, y, w, h] of [[cx - 6, cy, 4, 1], [cx + 3, cy, 4, 1], [cx, cy - 6, 1, 4], [cx, cy + 3, 1, 4]]) ctx.fillRect(x + 1, y + 1, w, h);
    ctx.fillStyle = col;
    for (const [x, y, w, h] of [[cx - 6, cy, 4, 1], [cx + 3, cy, 4, 1], [cx, cy - 6, 1, 4], [cx, cy + 3, 1, 4]]) ctx.fillRect(x, y, w, h);
    const rm = near(), who = bot ? th : rm || (th && th.name ? th : null);
    const mid = (text, y, c) => words(text, Math.round(cx - room.tinyW(text)), y, c);
    if (armed || rm) { if (who) mid(who.name.toUpperCase(), 22, bot ? "#ffb4a8" : "#e8e8ee"); }   // who is in your sights, up top (not over their face)
    if (armed && !who && th && room.use(th) === "reload") mid("E: RELOAD", 22, "#e8e8ee");
    else if (what) mid(what.toUpperCase(), 22, "#e8e8ee");
    if (!bot && rm) mid("E: PUT BACK TOGETHER", 36, "#e8e8ee");
    // Doom's messages, up left, a few seconds each: the keys at first, a kill, shells picked up
    const msgs = said.filter((m) => now - m.t0 < 4000).slice(-2);
    if (now - t0 < 6000) msgs.unshift({ text: armed ? "WASD WALK - MOUSE LOOK - CLICK SHOOT - E USE - ESC LEAVE" : "WASD WALK - MOUSE LOOK - CLICK USE - ESC LEAVE" });
    msgs.forEach((m, i) => words(m.text, 4, 4 + i * 13, "#e8402a"));
    statusBar(now, armed);
  }
  // the room's 3x5 letters at twice their size, with a shadow
  const letters = document.createElement("canvas"); letters.width = 640; letters.height = 6;
  function words(text, x, y, col, k = 2) {
    const lx = letters.getContext("2d"), w = Math.min(640, room.tinyW(text));
    lx.clearRect(0, 0, 640, 6); room.tiny(lx, text, 0, 0, "#0c0a12");
    ctx.drawImage(letters, 0, 0, w, 5, x + 1, y + 1, w * k, 5 * k);
    lx.clearRect(0, 0, 640, 6); room.tiny(lx, text, 0, 0, col);
    ctx.drawImage(letters, 0, 0, w, 5, x, y, w * k, 5 * k);
  }

  // ---- Doom's status bar: its stone, its sunk panels, the big red numbers, the small yellow ones
  const bar = document.createElement("canvas"); bar.width = BW; bar.height = BAR;
  const bx = bar.getContext("2d");
  const STONE = (() => {
    const c = document.createElement("canvas"); c.width = 64; c.height = BAR;
    const x = c.getContext("2d"); x.fillStyle = "#5f5a52"; x.fillRect(0, 0, 64, BAR);
    let h = 7;
    for (let i = 0; i < 64 * BAR; i++) { h = (h * 1103515245 + 12345) >>> 0; const v = h >>> 24; if (v < 70) { x.fillStyle = v < 22 ? "#4a463f" : v < 44 ? "#6c675e" : "#544f48"; x.fillRect(i % 64, (i / 64) | 0, 1, 1); } }
    x.fillStyle = "#8a847a"; x.fillRect(0, 0, 64, 1); x.fillStyle = "#3a3731"; x.fillRect(0, 1, 64, 1);
    return c;
  })();
  const sunk = (x, y, w, h) => {              // a panel sunk into the stone: dark top and left, light bottom and right
    bx.fillStyle = "#2e2b27"; bx.fillRect(x, y, w, 1); bx.fillRect(x, y, 1, h);
    bx.fillStyle = "#8f897d"; bx.fillRect(x, y + h - 1, w, 1); bx.fillRect(x + w - 1, y, 1, h);
  };
  const BIG = {
    0: [".##.", "#..#", "#..#", "#..#", "#..#", "#..#", ".##."], 1: ["..#.", ".##.", "..#.", "..#.", "..#.", "..#.", ".###"], 2: [".##.", "#..#", "...#", "..#.", ".#..", "#...", "####"],
    3: ["###.", "...#", "...#", ".##.", "...#", "...#", "###."], 4: ["#..#", "#..#", "#..#", "####", "...#", "...#", "...#"], 5: ["####", "#...", "###.", "...#", "...#", "#..#", ".##."],
    6: [".##.", "#...", "###.", "#..#", "#..#", "#..#", ".##."], 7: ["####", "...#", "..#.", "..#.", ".#..", ".#..", ".#.."], 8: [".##.", "#..#", "#..#", ".##.", "#..#", "#..#", ".##."],
    9: [".##.", "#..#", "#..#", ".###", "...#", "...#", ".##."], "%": ["#..#", "#..#", "..#.", ".#..", "#...", "#..#", "#..#"], "-": ["....", "....", "....", "####", "....", "....", "...."],
  };
  function big(text, right, y) {             // Doom's big numbers: red blocks, lit from the top left, a shadow; right-aligned
    let x = right - text.length * 15 + 3;
    for (const ch of text) {
      const g = BIG[ch];
      if (g) g.forEach((row, j) => [...row].forEach((q, i) => {
        if (q !== "#") return;
        const px = x + i * 3, py = y + j * 3;
        bx.fillStyle = "#3a0c08"; bx.fillRect(px + 1, py + 1, 3, 3);
        bx.fillStyle = "#c42a1c"; bx.fillRect(px, py, 3, 3);
        bx.fillStyle = "#f0583c"; bx.fillRect(px, py, 2, 1); bx.fillRect(px, py, 1, 2);
        bx.fillStyle = "#84160c"; bx.fillRect(px + 2, py + 2, 1, 1);
      }));
      x += 15;
    }
  }
  const small = (text, x, y, col, k = 2) => {   // the room's letters on the bar, with a shadow
    const lx = letters.getContext("2d"), w = room.tinyW(text);
    for (const [dx, c] of [[1, "#1e1c19"], [0, col]]) { lx.clearRect(0, 0, 640, 6); room.tiny(lx, text, 0, 0, c); bx.drawImage(letters, 0, 0, w, 5, x + dx, y + dx, w * k, 5 * k); }
    return w * k;
  };
  const shortLabel = (l) => (l === "session" ? "5H" : /all models/.test(l) ? "WEEK" : l.replace(/^week \(|\)$/g, "").replace(/ only/, "")).toUpperCase().slice(0, 6);
  let faceImg = null, faceSrc = null;
  function statusBar(now, armed) {
    const st = room.status() || {};
    for (let x = 0; x < BW; x += 64) bx.drawImage(STONE, x, 0);
    if ((st.kills || 0) > kills0) { kills0 = st.kills; grin = now; }
    const label = (t, x, w) => small(t, Math.round(x + (w - room.tinyW(t) * 2) / 2), BAR - 12, "#d2c6ae");
    // AMMO, HEALTH
    let x = 0;
    for (const [name, v, w] of [["AMMO", armed ? String(shells) : "-", 60], ["HEALTH", st.health == null ? "-" : st.health + "%", 80]]) {
      sunk(x + 2, 3, w - 4, BAR - 6); big(v, x + w - 8, 6); label(name, x, w); x += w;
    }
    // BOTS, where Doom has ARMS: at work, waiting for you, idle, done (lit while there are any)
    sunk(x + 2, 3, 60, BAR - 6);
    [["W", st.working], ["?", st.waiting], ["I", st.idle], ["D", st.done]].forEach(([n, v], i) => {
      const ex = x + 6 + (i % 2) * 30, ey = 6 + Math.floor(i / 2) * 12, on = (v || 0) > 0;
      small(n, ex, ey, on ? "#d2c6ae" : "#77705f"); small(String(Math.min(99, v || 0)), ex + 8, ey, on ? "#f2c84c" : "#77705f");
    });
    label("BOTS", x, 64); x += 64;
    // your face, looking about; after a kill it grins a moment (the panel goes red)
    const src = room.face();
    if (src && src !== faceSrc) { faceSrc = src; faceImg = new Image(); faceImg.src = src; }
    sunk(x + 2, 2, 44, BAR - 4);
    bx.fillStyle = now - grin < 1300 ? "#7a2a18" : "#24211d"; bx.fillRect(x + 3, 3, 42, BAR - 6);
    if (faceImg && faceImg.complete) { const lk = Math.floor(now / 1100) % 4, sx = lk === 1 ? -2 : lk === 3 ? 2 : 0; bx.imageSmoothingEnabled = false; bx.drawImage(faceImg, x + 4 + sx, 4, 40, 40); }
    x += 48;
    // ARMOR
    sunk(x + 2, 3, 76, BAR - 6); big(st.armor == null ? "-" : st.armor + "%", x + 72, 6); label("ARMOR", x, 80); x += 80;
    // the doors as keys: one a machine, in its paint, dark while it does not answer
    sunk(x + 2, 3, 17, BAR - 6);
    (st.keys || []).slice(0, 3).forEach((k, i) => {
      const ky = 7 + i * 12;
      bx.fillStyle = "#1e1c19"; bx.fillRect(x + 6, ky + 1, 10, 8);
      bx.fillStyle = k.ok ? k.color : "#3a3731"; bx.fillRect(x + 5, ky, 10, 8);
      bx.fillStyle = "#1e1c19"; bx.fillRect(x + 11, ky + 3, 2, 2);
    });
    x += 21;
    // the tokens left, where Doom lists its ammo: every limit /usage has, what is left of a hundred
    sunk(x + 2, 3, BW - x - 4, BAR - 6);
    const bars = (st.bars || []).slice(0, 4);
    if (!bars.length) small("TOKENS ?", x + 8, 8, "#d2c6ae");
    bars.forEach((b, i) => {
      const ty = 5 + i * 10.5, lab = shortLabel(b.label), v = `${b.left == null ? "-" : b.left}/100`;
      small(lab, x + 7, Math.round(ty), "#d2c6ae", 1.6); small(v, BW - 8 - room.tinyW(v) * 1.6, Math.round(ty), "#f2c84c", 1.6);
    });
    // on the view's bottom: the stone right across, the bar in the middle
    for (let sx = 0; sx < RW; sx += 64) ctx.drawImage(STONE, sx, VIEW);
    ctx.drawImage(bar, Math.round((RW - BW) / 2), VIEW);
  }

  // ---- Doom's wipe between screens: the old one melts down in columns, each starting a little after its
  // neighbour, speeding up, showing what is behind it
  let melt = null;
  function startMelt(from) {                 // from: a canvas as large as the view
    if (still) return null;
    const n = 160, y = new Float32Array(n);
    y[0] = -Math.random() * 16;
    for (let i = 1; i < n; i++) y[i] = Math.max(-15, Math.min(0, y[i - 1] + Math.floor(Math.random() * 3) - 1));
    return { from, y, n, last: performance.now() };
  }
  function drawMelt(target, m, now) {        // true while it runs
    const k = RH / 200, cw = RW / m.n;
    let tics = Math.min(6, Math.floor((now - m.last) / (1000 / 35)));
    m.last += tics * (1000 / 35);
    while (tics-- > 0) for (let i = 0; i < m.n; i++) { if (m.y[i] < 0) m.y[i]++; else m.y[i] += (m.y[i] < 16 * k ? m.y[i] + 1 : 8 * k); }
    let on = false;
    for (let i = 0; i < m.n; i++) {
      const yy = Math.max(0, Math.round(m.y[i])), x = Math.floor(i * cw), w = Math.floor((i + 1) * cw) - x;
      if (yy >= RH) continue;
      on = true;
      target.drawImage(m.from, x, 0, w, RH - yy, x, yy, w, RH - yy);
    }
    return on;
  }
  {                                           // going in: the room as it was on the page melts away
    const old = document.createElement("canvas"); old.width = RW; old.height = RH;
    const ox = old.getContext("2d"), snap = room.snapshot && room.snapshot();
    ox.fillStyle = getComputedStyle(document.body).backgroundColor || "#0c0a12"; ox.fillRect(0, 0, RW, RH);
    if (snap && snap.canvas) { const r = snap.rect, kx = RW / innerWidth, ky = RH / innerHeight; ox.imageSmoothingEnabled = false; ox.drawImage(snap.canvas, r.left * kx, r.top * ky, r.width * kx, r.height * ky); }
    melt = startMelt(old);
  }

  let raf = 0, last = performance.now(), ms = 0;   // ms: how long a frame takes to draw, smoothed
  function frame(now) {
    raf = requestAnimationFrame(frame);
    const dt = Math.min(0.05, (now - last) / 1000); last = now;
    stepYou(dt);
    const t = performance.now();
    render(now);
    if (melt && !drawMelt(ctx, melt, now)) melt = null;
    ms = ms * 0.95 + (performance.now() - t) * 0.05;
  }
  raf = requestAnimationFrame(frame);

  function leave() {
    cancelAnimationFrame(raf);
    removeEventListener("keydown", onKey, true); removeEventListener("keyup", onKey, true);
    removeEventListener("mousemove", onMove); removeEventListener("mouseup", onUp); removeEventListener("resize", resize); removeEventListener("blur", blur);
    document.removeEventListener("pointerlockchange", onLock); document.removeEventListener("pointerlockerror", onLockError);
    if (document.pointerLockElement === cv) document.exitPointerLock();
    window.__fps = null;
    // going out: this view melts away over the room
    const last3d = document.createElement("canvas"); last3d.width = RW; last3d.height = RH; last3d.getContext("2d").drawImage(cv, 0, 0);
    const out = startMelt(last3d);
    if (out) {
      cv.style.background = "transparent"; cv.style.pointerEvents = "none"; cv.style.cursor = "auto";
      const step = (now) => { ctx.clearRect(0, 0, RW, RH); if (drawMelt(ctx, out, now)) requestAnimationFrame(step); else cv.remove(); };
      requestAnimationFrame(step);
    } else cv.remove();
    done();
  }
  // for look.mjs: where you are, turn, step, fire, use, without a mouse
  window.__fps = { me, look, ms: () => ms, turn: (r) => { a += r; }, set: (x, y, ang) => { me.x = x; me.y = y; if (ang !== undefined) a = ang; }, fire: () => { shot = -1e9; fire(); }, use, leave, under: () => { const t = under(2); return t && { bot: t.bot, rem: t.rem, name: t.name, x: t.x, y: t.y, w: t.sw, h: t.sh }; }, melting: () => !!melt, size: () => ({ RW, RH }) };
  return { leave };
}
