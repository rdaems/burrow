// burrow: the window's view. Outside, by this computer's clock: the sky by the hour, the sun or the moon on
// their arc, stars after dark, clouds going with the wind, hills in the haze behind.
//
// And it is a game: on the hills two guns, an artillery duel in the manner of the old DOS games (VGABomb,
// Gorillas): you against an idle bot drawn by lot, or two bots' duel to watch. Angle and power, a wind that
// changes each round (the clouds, the smoke and the pennants go with it), craters where the shells come down,
// pines blown away; a gun hit is the round. A bot knows the physics and aims less well its first shots of a round.
// With the room's radio on it is heard (sound.js): the gun, the whistle of the shell coming down, the blast, a gun hit,
// the gunner landing, a tune for the round.
//
// outsideView(canvas, app), app = { shown(), wears(seed) }; returns { open(a, b, at), close(), playing(), left(id) }.
// a and b are { id, name, color, human, seed }; with b null it is only the outside. Everything is in dots, the
// canvas's pixels (the page scales it to the pane); S is a dot of the first, coarser version, kept for the physics.
function outsideView(cv, app) {
  const W = cv.width, H = cv.height, c = cv.getContext("2d"), C = pixelAvatar.C, S = W / 160;
  const G = 60 * S, VK = 1.1 * S, PUSH = 9 * S, DT = 1 / 120, BOOM = 8;   // gravity (dots/s²), dots/s a power point, the wind's push at full strength, a step, a shell's reach
  const rnd = (a, b) => a + Math.random() * (b - a);
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const gauss = () => Math.sqrt(-2 * Math.log(1 - Math.random())) * Math.cos(2 * Math.PI * Math.random());
  const hash = (s) => { let h = 2166136261; for (const ch of String(s)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619); return (h >>> 0) / 4294967296; };
  const mixc = (a, b, k) => "#" + [1, 3, 5].map((i) => { const x = parseInt(a.slice(i, i + 2), 16), y = parseInt(b.slice(i, i + 2), 16); return Math.round(x + (y - x) * k).toString(16).padStart(2, "0"); }).join("");
  const r = (x, y, w, h, col) => { c.fillStyle = col; c.fillRect(x, y, w, h); };
  const hear = (name, x, opt) => (window.Sound ? Sound.play(name, clamp((x / W) * 1.6 - 0.8, -1, 1), 1, false, opt) : null);   // left or right by where
  const disc = (x, y, rad, col, g = c) => { x = Math.round(x); y = Math.round(y); g.fillStyle = col; for (let j = -Math.floor(rad); j <= rad; j++) { const h = Math.floor(Math.sqrt(rad * rad - j * j)); g.fillRect(x - h, y + j, 2 * h + 1, 1); } };
  // dots with an ink outline round them: [x, y, colour] each; the outline first, so it never covers a dot
  function inked(pts, g = c) {
    g.fillStyle = C.ink;
    for (const [x, y] of pts) { g.fillRect(x - 1, y, 3, 1); g.fillRect(x, y - 1, 1, 3); }
    for (const [x, y, col] of pts) { g.fillStyle = col; g.fillRect(x, y, 1, 1); }
  }
  // a letter grid at x0, y0: flip mirrors it, turn lays it on its back (its head where its top-left was)
  function grid(rows, pal, x0, y0, flip, turn, out = []) {
    const h = rows.length, w = rows[0].length;
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) {
      const ch = rows[j][i];
      if (ch === "." || !pal[ch]) continue;
      out.push(turn ? [x0 + (flip ? h - 1 - j : j), y0 + w - 1 - i, pal[ch]] : [x0 + (flip ? w - 1 - i : i), y0 + j, pal[ch]]);
    }
    return out;
  }
  // a thick line from (x0, y0) to (x1, y1), th dots across, with a lighter row along its top
  function beam(x0, y0, x1, y1, th, col, hi, out = []) {
    const n = Math.ceil(Math.hypot(x1 - x0, y1 - y0) * 2), seen = new Set();
    for (let k = 0; k <= n; k++) {
      const x = x0 + (x1 - x0) * k / n, y = y0 + (y1 - y0) * k / n;
      for (let j = 0; j < th; j++) for (let i = 0; i < th; i++) {
        const px = Math.round(x - th / 2 + i + 0.5), py = Math.round(y - th / 2 + j + 0.5), key = px + "," + py;
        if (!seen.has(key)) { seen.add(key); out.push([px, py, j === 0 && hi ? hi : col]); }
      }
    }
    return out;
  }

  let land = hills(false), wind = 0.3, duel = null, at = null, last = 0, acc = 0, T = 0;
  const clouds = [0, 1, 2, 3].map((i) => ({ x: i * 71 + 20, y: 12 + (i % 3) * 15, k: 0.7 + 0.15 * i, s: i % 2 }));
  const held = {};                           // the arrow keys held down: since when, and what of a step is owed

  // ---- the land: rolling hills at two levels, a mountain between them (a gentle rise without a duel), the far
  // hills in the haze behind, a few pines
  function hills(mountain) {
    const g = new Float32Array(W), far = new Float32Array(W), a = rnd(0, 7), b = rnd(0, 7), l = rnd(-11, 11), rt = rnd(-11, 11);
    const xm = rnd(0.41, 0.59) * W, hm = mountain ? rnd(32, 58) : rnd(6, 14), wm = rnd(22, 38), fa = rnd(0, 7), fb = rnd(0, 7);
    for (let x = 0; x < W; x++) {
      const u = x / (W - 1), lvl = 0.7 * H + l + (rt - l) * u * u * (3 - 2 * u);
      g[x] = clamp(lvl + 5 * Math.sin(x / 17 + a) + 3 * Math.sin(x / 8.5 + b) - hm * Math.exp(-(((x - xm) / wm) ** 2)), 0.24 * H, H - 4);
      far[x] = 0.58 * H + 9 * Math.sin(x / 29 + fa) + 5 * Math.sin(x / 11 + fb);
    }
    const pines = [];
    for (let k = 0; k < 40 && pines.length < 6; k++) {
      const x = Math.round(rnd(4, W - 5));
      if (!pines.some((p) => Math.abs(p.x - x) < 14)) pines.push({ x, h: Math.round(rnd(13, 20)) });
    }
    return { g, far, pines, scar: new Uint8Array(W), v: Math.random() };
  }
  const ground = (x) => land.g[clamp(Math.round(x), 0, W - 1)];
  function crater(x, y, rad) {
    for (let i = Math.ceil(x - rad); i <= x + rad; i++) {
      if (i < 0 || i >= W) continue;
      const low = y + Math.sqrt(Math.max(0, rad * rad - (i - x) ** 2));
      if (land.g[i] < low) { land.g[i] = Math.min(H - 2, low); land.scar[i] = 1; }
    }
    land.v = Math.random();
  }

  // ---- a duel: two guns, a turn each, the round to whoever hits the other's gun
  function gun(p, side) { return { ...p, side, f: side ? -1 : 1, x: 0, ang: 45, pow: 50, shots: 0, kick: 0, flash: 0, dead: false, body: null, trail: [], plan: null, skill: 0.75 + 0.6 * hash(p.id) }; }
  function round(starter) {
    land = hills(true);
    const s = Math.random() < 0.5 ? -1 : 1;
    wind = s * Math.pow(Math.random(), 1.4);
    for (const P of duel.p) {
      P.x = P.f > 0 ? Math.round(rnd(24, 50)) : W - 1 - Math.round(rnd(24, 50));
      const span = [P.x - 23 * P.f, P.x + 7 * P.f].sort((a, b) => a - b), h = Math.round(land.g.slice(span[0], span[1] + 1).reduce((a, b) => a + b, 0) / (span[1] - span[0] + 1));
      for (let x = span[0]; x <= span[1]; x++) land.g[x] = h;   // a level pad for the gun, its gunner and its pennant,
      for (let d = 1; d <= 14; d++) for (const x of [span[0] - d, span[1] + d]) {   // sloping into the hill round it
        if (x < 0 || x >= W) continue;
        const k = d / 15, s = k * k * (3 - 2 * k);
        land.g[x] = h + (land.g[x] - h) * s;
      }
      land.pines = land.pines.filter((p) => p.x < span[0] - 5 || p.x > span[1] + 5);
      Object.assign(P, { shots: 0, kick: 0, flash: 0, dead: false, body: null, trail: [], plan: null });
    }
    Object.assign(duel, { turn: starter, phase: "aim", t: 0, shell: null, booms: [], bits: [] });
  }
  // the gun's pivot, and its muzzle at angle a in degrees over the ground toward the other side
  const pivot = (P) => [P.x, ground(P.x) - 6];
  function launch(P, a, p) {
    const rad = a * Math.PI / 180, dx = P.f * Math.cos(rad), dy = -Math.sin(rad), [x0, y0] = pivot(P);
    return { x: x0 + dx * 11, y: y0 + dy * 11, vx: dx * p * VK, vy: dy * p * VK, by: P.side, age: 0 };
  }
  function fly(s) { s.vx += wind * PUSH * DT; s.vy += G * DT; s.x += s.vx * DT; s.y += s.vy * DT; s.age += DT; }
  // where it comes down: { x, y, gun } (gun: the side whose gun it hits), { out } off the side; null in the air
  function down(s) {
    if (s.x < -6 || s.x > W + 5 || s.y > H + 6) return { out: true };
    for (const Q of duel.p) {
      if (Q.dead || (Q.side === s.by && s.age < 0.25)) continue;
      const gy = ground(Q.x);
      if (Math.abs(s.x - Q.x) < 5 && s.y > gy - 9 && s.y < gy + 1) return { x: s.x, y: s.y, gun: Q.side };
    }
    if (s.x >= 0 && s.x < W && s.y >= ground(s.x)) return { x: s.x, y: ground(s.x) };
    return null;
  }
  function sim(P, a, p) { const s = launch(P, a, p); for (let k = 0; k < 1500; k++) { fly(s); const d = down(s); if (d) return d; } return { out: true }; }
  // a bot's aim: for every angle the power that comes closest, then its own way of shooting (an angle it
  // likes, seeded by its id), then the hand: far off the first shot of a round, closer with every shot
  function plan(P) {
    const Q = duel.p[1 - P.side], tx = Q.x, ty = ground(Q.x) - 4;
    const miss = (d) => (d.out ? 1e9 : d.gun === Q.side ? 0 : Math.hypot(d.x - tx, d.y - ty) + (d.gun === P.side ? 120 : 0));
    const best = [];
    for (let a = 20; a <= 80; a += 4) {
      let b = null;
      for (let p = 10; p <= 100; p += 1.5) { const d = miss(sim(P, a, p)); if (!b || d < b.d) b = { a, p, d }; }
      for (let p = b.p - 1.5; p <= b.p + 1.5; p += 0.25) { const d = miss(sim(P, a, p)); if (d < b.d) b = { a, p, d }; }
      best.push(b);
    }
    const likes = 30 + 40 * hash(P.id + "a"), ok = best.filter((b) => b.d < 5);
    const b = (ok.length ? ok : best).reduce((x, y) => (ok.length ? (Math.abs(y.a - likes) < Math.abs(x.a - likes) ? y : x) : y.d < x.d ? y : x));
    const k = P.shots, sp = (9 * Math.pow(0.55, k) + 0.5) * P.skill, sa = 2 * Math.pow(0.6, k) * P.skill;
    return { a: clamp(Math.round(b.a + gauss() * sa), 5, 85), p: clamp(Math.round(b.p + gauss() * sp), 5, 100), wait: rnd(0.6, 1.3) };
  }
  function fire(P) {
    duel.shell = launch(P, P.ang, P.pow);
    duel.phase = "fly"; P.shots++; P.kick = 1; P.flash = 0.12; P.trail = [];
    duel.fired[P.side] = performance.now() / 1000;
    hear("cannon", P.x);
  }
  function boom(d) {
    const s = duel.shell, P = duel.p[s.by];
    duel.shell = null; duel.phase = "boom"; duel.t = 0.9;
    if (s.whistle && s.whistle.stop) s.whistle.stop();
    if (d.out) return;
    const hit = duel.p.filter((Q) => !Q.dead && (d.gun === Q.side || Math.hypot(d.x - Q.x, d.y - (ground(Q.x) - 4)) <= BOOM));
    duel.booms.push({ x: d.x, y: d.y, t: 0, R: hit.length ? 13 : 8 });
    const xi = clamp(Math.round(d.x), 0, W - 1), soil = land.scar[xi] ? ["#8a6239", "#6b4a2b"] : ["#6aa84f", "#6b4a2b"];
    for (let k = 0; k < (hit.length ? 40 : 22); k++) {
      const a = rnd(Math.PI * 1.05, Math.PI * 1.95), v = rnd(26, 70);
      duel.bits.push({ x: d.x, y: d.y - 1, vx: Math.cos(a) * v, vy: Math.sin(a) * v, t: 0, col: k % 4 === 0 ? "#3f3740" : soil[k % 2] });
    }
    // pines nearby go up in the blast
    for (const p of land.pines.filter((p) => Math.abs(p.x - d.x) < BOOM + 2 && Math.abs(ground(p.x) - d.y) < BOOM + 4)) {
      for (let k = 0; k < 10; k++) duel.bits.push({ x: p.x, y: ground(p.x) - rnd(2, p.h), vx: rnd(-30, 30), vy: -rnd(20, 50), t: 0, col: k % 3 ? "#1f4a2e" : "#5a3a22" });
      p.gone = true;
    }
    land.pines = land.pines.filter((p) => !p.gone);
    for (const Q of hit) { Q.dead = true; Q.body = { x: Q.x - 15 * Q.f, y: ground(Q.x) - 1, vx: -Q.f * rnd(22, 38), vy: -rnd(48, 66), down: false }; }
    crater(d.x, d.y, BOOM);
    hear(hit.length ? "wreck" : "blast", d.x);
    if (hit.length) {
      const win = hit.length === 2 ? null : 1 - hit[0].side;
      if (win !== null) duel.score[win]++;
      duel.won = win; duel.tune = 1.1;
      duel.phase = "over"; duel.t = 3.6; duel.next = hit.length === 2 ? 1 - P.side : hit[0].side;   // the loser starts the next round
    }
  }

  // ---- the time going by: a step of the duel, then the clouds and the bits in the air
  function update(dt) {
    T += dt;
    for (const cl of clouds) { cl.x += wind * 13 * cl.k * dt; if (cl.x > W + 30) cl.x -= W + 60; if (cl.x < -30) cl.x += W + 60; }
    if (!duel) return;
    for (const P of duel.p) {
      P.kick = Math.max(0, P.kick - dt * 5); P.flash = Math.max(0, P.flash - dt);
      const b = P.body;
      if (b && !b.down) { b.vy += G * dt; b.x += b.vx * dt; b.y += b.vy * dt; b.x = clamp(b.x, 7, W - 8); if (b.vy > 0 && b.y >= ground(b.x) - 1) { b.y = ground(b.x) - 1; b.down = true; hear("thud", b.x); } }
      if (P.leaving) P.walk = (P.walk || 0) + dt * 22;
    }
    if (duel.tune > 0 && (duel.tune -= dt) <= 0 && duel.won !== null) {   // the round's tune: yours won, or lost (a duel you watch: won)
      const mine = duel.p.find((P) => P.human);
      hear(!mine || mine.side === duel.won ? "win" : "lose", W / 2);
    }
    for (const bm of duel.booms) bm.t += dt;
    duel.booms = duel.booms.filter((bm) => bm.t < 1.8);
    for (const b of duel.bits) { b.t += dt; b.vy += G * dt; b.x += b.vx * dt; b.y += b.vy * dt; }
    duel.bits = duel.bits.filter((b) => b.t < 2.5 && b.x > -2 && b.x < W + 2 && b.y < ground(b.x));
    if (duel.shell) {
      acc += dt;
      while (duel.shell && acc >= DT) {
        acc -= DT; fly(duel.shell);
        const P = duel.p[duel.shell.by], sh = duel.shell;
        if (!sh.whistle && sh.vy > 0) {      // over the top: it whistles on the way down, until it lands
          const cp = { ...sh };
          let n = 0;
          while (n < 1500 && !down(cp)) { fly(cp); n++; }
          sh.whistle = hear("whistle", sh.x, { dur: n * DT }) || true;
        }
        if (Math.round(duel.shell.age / DT) % 3 === 0) P.trail.push([duel.shell.x, duel.shell.y]);
        const d = down(duel.shell);
        if (d) boom(d);
      }
      return;
    }
    acc = 0;
    if (duel.phase === "boom" && (duel.t -= dt) <= 0) {
      if (duel.gone !== undefined) duel.phase = "left";
      else { duel.turn = 1 - duel.turn; duel.phase = "aim"; }
    }
    if (duel.phase === "over" && (duel.t -= dt) <= 0) { if (duel.gone !== undefined) duel.phase = "left"; else round(duel.next); }
    if (duel.phase !== "aim") return;
    const P = duel.p[duel.turn];
    if (P.human) return;                     // yours: the keys (aim, below)
    if (!P.plan) P.plan = plan(P);
    if ((P.plan.wait -= dt) > 0) return;
    // the barrel swings round to its angle, the power comes up to its mark, then it fires
    P.ang += clamp(P.plan.a - P.ang, -40 * dt, 40 * dt); P.pow += clamp(P.plan.p - P.pow, -50 * dt, 50 * dt);
    if (Math.abs(P.ang - P.plan.a) < 0.01 && Math.abs(P.pow - P.plan.p) < 0.01) { P.ang = P.plan.a; P.pow = P.plan.p; if ((P.plan.hold = (P.plan.hold || 0) + dt) > 0.35) { P.plan = null; fire(P); } }
  }
  // you: ↑ ↓ the angle, ← → the power, a step a press and faster while held; space or Enter fires
  const you = () => duel && duel.phase !== "left" && duel.phase !== "over" ? duel.p.find((P) => P.human) : null;
  function aim(dt) {
    const P = you();
    if (!P) return;
    const now = performance.now() / 1000;
    for (const [k, h] of Object.entries(held)) {
      const t = now - h.t;
      if (t < 0.3) continue;
      h.owe += dt * (10 + 40 * Math.min(1, (t - 0.3) / 1.2));
      const n = Math.floor(h.owe); h.owe -= n;
      if (n) nudge(P, k, n);
    }
  }
  function nudge(P, k, n) {
    if (k === "up" || k === "down") P.ang = clamp(Math.round(P.ang) + (k === "up" ? n : -n), 0, 90);
    else P.pow = clamp(Math.round(P.pow) + (k === "more" ? n : -n), 0, 100);
  }
  const KEYS = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "less", ArrowRight: "more" };
  addEventListener("keydown", (e) => {
    if (!app.shown() || !duel || e.metaKey || e.ctrlKey || e.altKey) return;
    const k = KEYS[e.key], P = you();
    if (k) { e.preventDefault(); if (P && !held[k]) { held[k] = { t: performance.now() / 1000, owe: 0 }; nudge(P, k, 1); } }
    else if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      if (!e.repeat && P && duel.phase === "aim" && duel.p[duel.turn] === P) fire(P);
    }
  });
  addEventListener("keyup", (e) => { const k = KEYS[e.key]; if (k) delete held[k]; });
  addEventListener("blur", () => { for (const k in held) delete held[k]; });

  // ---- the picture
  const SKY = [[0, "#0d1430"], [5, "#141d3f"], [6.5, "#d9775a"], [8, "#7fb2e0"], [17, "#7fb2e0"], [19, "#e07a4a"], [20.5, "#2a2550"], [22, "#0d1430"], [24, "#0d1430"]];
  function skyAt(h) { for (let i = 1; i < SKY.length; i++) if (h <= SKY[i][0]) { const [h0, c0] = SKY[i - 1], [h1, c1] = SKY[i]; return mixc(c0, c1, (h - h0) / (h1 - h0)); } return SKY[0][1]; }
  const STARS = [...Array(110)].map((_, i) => [(i * 37 + (i >> 3) * 11) % W, (i * 53) % Math.round(H * 0.62), i % 4]);
  // the sky and the land are drawn into canvases of their own, again only when they change
  const skyCv = document.createElement("canvas"), landCv = document.createElement("canvas");
  skyCv.width = landCv.width = W; skyCv.height = landCv.height = H;
  let skyWas = "", landWas = "";
  function paintSky(sky, dark) {
    if (skyWas === sky) return;
    skyWas = sky;
    const g = skyCv.getContext("2d"), n = 7, bh = Math.ceil(H * 0.8 / n);
    const band = (b) => mixc(sky, "#ffffff", Math.min(b, n - 1) * (0.028 * (1 - dark) + 0.01));   // lighter toward the horizon
    for (let b = 0; b <= n; b++) { g.fillStyle = band(b); g.fillRect(0, b * bh, W, b === n ? H : bh); }
    for (let b = 1; b <= n; b++) for (let x = 0; x < W; x++) {   // dithered where two bands meet, as a 16-bit sky would be
      const y = b * bh;
      g.fillStyle = band(b - 1);
      if (x % 2 === 0) g.fillRect(x, y, 1, 1);
      if (x % 4 === 1) g.fillRect(x, y + 1, 1, 1);
    }
  }
  const PINE = (h) => {                      // a pine h dots high: three tiers on a trunk, lit from the left
    const rows = [];
    for (let k = 0; k < h - 2; k++) { const t = Math.floor(k / ((h - 2) / 3)), q = k - t * ((h - 2) / 3), half = Math.min(4, Math.floor(1 + t + q * 0.55)); rows.push(".".repeat(5 - half) + "l" + "d".repeat(2 * half - 1) + ".".repeat(5 - half)); }
    rows.push(".....t....", ".....t....");
    return rows;
  };
  function paintLand(sky, dark) {
    const key = land.v + ":" + skyWas + ":" + land.pines.length;
    if (landWas === key) return;
    landWas = key;
    const g = landCv.getContext("2d");
    g.clearRect(0, 0, W, H);
    const p = (x, y, w, h, col) => { g.fillStyle = col; g.fillRect(x, y, w, h); };
    const haze = mixc(mixc("#3d6b55", sky, 0.5), "#0c1418", dark * 0.55), haze2 = mixc(haze, "#ffffff", 0.12 * (1 - dark));
    for (let x = 0; x < W; x++) { const t = Math.round(land.far[x]); p(x, t, 1, H - t, haze); p(x, t, 1, 1, haze2); }
    // the hills: grass along the top (earth where a shell came down), darker strata further in, dithered between
    const bands = ["#2f5a3c", "#2b4a3a", "#24402f", "#1d3328", "#182b22"].map((b) => mixc(b, "#0c1418", dark * 0.8));
    const grass = [mixc("#7cc05a", "#1a2a1e", dark), mixc("#4a7a3e", "#122019", dark)], earth = [mixc("#a77a4a", "#1a1410", dark), mixc("#6b4a2b", "#140f0c", dark)];
    for (let x = 0; x < W; x++) {
      const top = Math.round(land.g[x]), soil = land.scar[x] ? earth : grass;
      for (let b = 0; ; b++) {
        const y0 = top + 3 + b * 10;
        if (y0 >= H) break;
        p(x, y0, 1, Math.min(10, H - y0), bands[Math.min(b, bands.length - 1)]);
        if (b > 0 && b < bands.length) { if ((x + y0) % 2) p(x, y0, 1, 1, bands[b - 1]); if ((x + y0) % 4 === 1) p(x, y0 + 1, 1, 1, bands[b - 1]); }
      }
      p(x, top, 1, 1, soil[0]); p(x, top + 1, 1, 2, soil[1]);
      if (!land.scar[x] && (x * 7 + top) % 6 === 0) p(x, top - 1, 1, 1, soil[1]);   // tufts
    }
    const pine = { l: mixc("#3f8a52", "#0c1a12", dark), d: mixc("#1f4a2e", "#08110c", dark), t: mixc("#5a3a22", "#140f0c", dark) };
    for (const pn of land.pines) { const rows = PINE(pn.h); inked(grid(rows, pine, pn.x - 4, Math.round(ground(pn.x)) - rows.length), g); }
  }
  // the gunners, facing right: you as the portrait wears it, a bot a robot with its colour on its antenna and chest
  const PERSON = ["..hhh..", ".hhhhh.", ".hhsks.", ".hssss.", "..sss..", ".ccccc.", "cccccss", "ccdcc..", ".cdcc..", ".cccc..", ".ll.l..", ".ll.l..", ".bb.bb."];
  const ROBOT = ["....a..", "....n..", ".mmmmm.", ".mmmee.", ".mmmmm.", "..nnn..", ".mmmmm.", "mmkkmmh", ".mkkmn.", ".mmmmn.", "..n.n..", "..n.n..", ".mm.mm."];
  function gunner(P, x, y, lying, hop) {     // x: its middle, y: the ground under it
    const w = app.wears && P.human ? app.wears(P.seed) : null;
    const pal = P.human ? { h: (w && w.hair) || C.ink2, s: (w && w.skin) || "#f7c59a", k: C.ink, c: (w && w.shirt) || P.color, d: (w && w.shade) || P.color, l: C.ink2, b: C.ink }
      : { a: P.color, n: C.steel1, m: C.steel2, e: C.steel4, k: P.color, h: C.steel3 };
    const rows = P.human ? PERSON : ROBOT, h = rows.length, wd = rows[0].length, flip = P.f < 0;
    const x0 = Math.round(x) - (lying ? h >> 1 : wd >> 1), y0 = Math.round(y) - (lying ? wd : h) - hop;
    inked(grid(rows, pal, x0, y0, flip, lying));
  }
  const WHEEL = ["..rrr..", ".rbsbr.", "rbbsbbr", "rssxssr", "rbbsbbr", ".rbsbr.", "..rrr.."];
  function drawGun(P) {
    const gy = Math.round(ground(P.x)), f = P.f, x = P.x - Math.round(f * P.kick * 2), [, py] = pivot(P);
    if (P.dead) {                            // the wreck, smoking
      inked([...beam(x - 6, gy - 2, x + 5, gy - 2, 2, "#2a2420"), ...grid(WHEEL, { r: "#2a2420", b: "#3f3740", s: "#3f3740", x: "#5d6280" }, x + 3 * f - 3, gy - 6)]);
      for (let k = 0; k < 5; k++) { const ph = (T * 0.35 + k / 5) % 1; c.globalAlpha = 0.6 * (1 - ph); disc(x + ph * wind * 14 + Math.sin(ph * 6 + k) * 2, gy - 6 - ph * 26, 1.5 + ph * 2.5, "#5d6280"); c.globalAlpha = 1; }
      return;
    }
    const rad = P.ang * Math.PI / 180, dx = f * Math.cos(rad), dy = -Math.sin(rad);
    const pts = [
      ...beam(x, py + 2, x - 12 * f, gy - 1, 2, "#734728", "#885c47"),                                   // the trail
      ...beam(x - dx * 2, py - dy * 2, x + dx * 9, py + dy * 9, 3, "#3f3740", "#6b6470"),              // the barrel
      ...beam(x + dx * 9, py + dy * 9, x + dx * 11, py + dy * 11, 2, C.ink2),                          // its muzzle
      ...grid(WHEEL, { r: "#4a2e1c", b: "#885c47", s: "#b07a4a", x: C.steel3 }, x - 3, gy - 7),         // the wheel
    ];
    inked(pts);
    if (P.flash > 0) { const mx = Math.round(x + dx * 13), my = Math.round(py + dy * 13); disc(mx, my, 3, C.gold1); disc(mx, my, 1.6, C.gold2); r(mx, my, 1, 1, C.white); }
  }
  function pennant(P) {                      // on a pole behind the gunner, streaming with the wind
    const gy = Math.round(ground(P.x)), x = P.x - 21 * P.f, top = gy - 17, pts = beam(x, top, x, gy - 1, 1, C.steel2);
    const n = Math.abs(wind) < 0.08 ? 0 : 2 + Math.round(Math.abs(wind) * 8), s = Math.sign(wind), lo = mixc(P.color, "#000000", 0.25);
    if (!n) for (let j = 0; j < 5; j++) pts.push([x + 1, top + 1 + j, j % 2 ? lo : P.color]);   // calm: it hangs down the pole
    for (let k = 0; k < n; k++) {
      const hh = Math.max(1, Math.round(5 * (1 - k / (n + 1)))), wave = Math.round(Math.sin(T * (4 + Math.abs(wind) * 8) - k * 0.9) * (k / n) * 1.4);
      for (let j = 0; j < hh; j++) pts.push([x + s * (k + 1), top + 1 + j + wave + Math.floor((5 - hh) / 2), j === hh - 1 ? lo : P.color]);
    }
    inked(pts);
  }
  // the duel's line over the picture, in the view's lettering: the two, the one whose turn it is in gold with
  // its angle and power, the rounds won between them (rewritten only when it changes)
  const bar = document.createElement("div"), esc = (s) => String(s).replace(/[&<>]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[ch]));
  bar.className = "duel"; bar.hidden = true; cv.before(bar);
  let barWas = "";
  function hud() {
    let h = "";
    if (duel) {
      const side = (P) => {
        const on = duel.turn === P.side && (duel.phase === "aim" || duel.phase === "fly" || duel.phase === "boom");
        const say = on ? `<i>angle ${Math.round(P.ang)} · power ${Math.round(P.pow)}</i>` : duel.gone === P.side ? "<i>left</i>" : "";
        const name = `<b${on ? ' class="on"' : ""}>${esc(P.human ? "you" : P.name)}</b>`;
        return `<span class="pl${P.side ? " r" : ""}">${P.side && on ? say + name : name + say}</span>`;
      };
      h = side(duel.p[0]) + `<span class="sc">${duel.score[0]} – ${duel.score[1]}</span>` + side(duel.p[1]);
    }
    if (h !== barWas) { barWas = h; bar.innerHTML = h; bar.hidden = !duel; }
  }
  function cloud(cl, light, shade) {         // puffs on a flat bottom, its underside in shade
    const x = Math.round(cl.x), y = cl.y, puffs = cl.s ? [[0, 2, 4], [6, -1, 6], [13, 0, 5], [19, 2, 4]] : [[0, 1, 5], [7, -2, 7], [15, 1, 5]];
    for (const [px, py, pr] of puffs) disc(x + px, y + py + 1, pr, shade);
    for (const [px, py, pr] of puffs) disc(x + px, y + py, pr - 0.6, light);
    r(x - 3, y + 4, (cl.s ? 25 : 22), 2, shade);
  }
  function draw() {
    const now = new Date(), h = now.getHours() + now.getMinutes() / 60, sky = skyAt(h);
    const dark = h < 6 || h > 20.5 ? 1 : h < 8 ? (8 - h) / 2 : h > 18.5 ? (h - 18.5) / 2 : 0;
    paintSky(sky, dark); c.drawImage(skyCv, 0, 0);
    if (dark > 0) for (const [x, y, k] of STARS) if (((Math.floor(T * 2) + k) % 6) || k === 0) r(x, y, 1, 1, k === 3 ? "#ffe9a4" : dark > 0.5 ? "#f4f6fb" : "#8b93ad");
    // the sun from 6:30 to 20:30, the moon the rest of the night, on an arc over the hills
    const day = h >= 6.5 && h <= 20.5, p = day ? (h - 6.5) / 14 : ((h + 24 - 20.5) % 24) / 10;
    const sx = Math.round(20 + p * (W - 40)), sy = Math.round(0.62 * H - Math.sin(p * Math.PI) * 0.5 * H);
    if (day) { c.globalAlpha = 0.25; disc(sx, sy, 10, "#fff6c0"); c.globalAlpha = 1; disc(sx, sy, 6.5, "#ffd34d"); disc(sx, sy, 5.2, "#ffe27a"); disc(sx - 1, sy - 1, 2.5, "#fff6c0"); }
    else for (let j = -6; j <= 6; j++) for (let i = -6; i <= 6; i++) {   // the moon, a crescent: its disc less the one beside it
      if (i * i + j * j > 36 || (i - 3) ** 2 + (j + 2) ** 2 <= 30) continue;
      r(sx + i, sy + j, 1, 1, (i === -4 && j === 1) || (i === -2 && j === -3) ? "#c2c9d9" : "#e8e5ef");
    }
    const light = mixc("#f4f6fb", sky, 0.2 + dark * 0.62), shade = mixc("#c2c9d9", sky, 0.35 + dark * 0.5);
    for (const cl of clouds) cloud(cl, light, shade);   // going with the wind
    paintLand(sky, dark); c.drawImage(landCv, 0, 0);
    if (duel) {
      // the last shot of each, a dotted line in its colour until its next; the guns; the shell in the air
      for (const P of duel.p) { c.globalAlpha = 0.5; for (const [x, y] of P.trail) r(Math.round(x), Math.round(y), 1, 1, P.color); c.globalAlpha = 1; }
      for (const P of duel.p) {
        pennant(P); drawGun(P);
        const lying = P.body && P.body.down, out = P.leaving && P.walk, bx = P.body ? P.body.x : P.x - 15 * P.f - (out ? P.f * P.walk : 0);
        const won = duel.phase === "over" && !P.dead ? Math.floor(T * 6) % 2 : 0;
        if (!(out && (bx < -6 || bx > W + 6))) gunner(P, bx, P.body ? P.body.y + 1 : ground(bx), lying, won);
      }
      const s = duel.shell;                  // over the top of the picture, a mark on its edge where it is
      if (s && s.y < 0) r(Math.round(s.x) - 1, 0, 3, 1, C.gold2);
      else if (s) { const x = Math.round(s.x), y = Math.round(s.y); inked([[x, y, C.steel1], [x + 1, y, C.steel0], [x, y + 1, C.steel0], [x + 1, y + 1, C.ink2]]); r(x, y, 1, 1, C.steel4); }
      for (const b of duel.bits) r(Math.round(b.x), Math.round(b.y), b.t < 0.6 ? 2 : 1, b.t < 0.6 ? 2 : 1, b.col);
      for (const bm of duel.booms) {         // its smoke rising with the wind, a flash, a ball of fire
        const t = bm.t, R = bm.R;
        if (t > 0.25) { c.globalAlpha = Math.max(0, 0.7 * (1 - (t - 0.25) / 1.55)); disc(bm.x + wind * 10 * (t - 0.25), bm.y - 3 - (t - 0.25) * 11, R * 0.6 * (1 - (t - 0.25) / 2.5), mixc("#8b93ad", "#3b3a55", dark)); c.globalAlpha = 1; }
        if (t < 0.5) {
          const k = Math.min(1, t / 0.12), fade = t > 0.3 ? 1 - (t - 0.3) / 0.2 : 1;
          disc(bm.x, bm.y, R * (0.5 + 0.5 * k) * (0.6 + 0.4 * fade), t < 0.08 ? C.white : C.red1);
          disc(bm.x, bm.y, R * (0.5 + 0.3 * k) * fade, t < 0.08 ? C.white : C.gold1);
          disc(bm.x, bm.y, R * 0.35 * fade, C.gold2);
        }
      }
    }
    hud();
  }
  function frame(ts) {
    if (!app.shown()) { last = 0; return; }
    const dt = last ? Math.min(0.05, (ts - last) / 1000) : 0;
    last = ts;
    aim(dt); update(dt); draw();
    requestAnimationFrame(frame);
  }

  window.__outside = {
    get: () => duel && { phase: duel.phase, turn: duel.turn, score: duel.score.slice(), wind, gone: duel.gone, p: duel.p.map((P) => ({ id: P.id, x: P.x, y: ground(P.x), ang: P.ang, pow: P.pow, dead: P.dead, shots: P.shots })) },
    aim: (a, p) => { const P = you(); if (P) { P.ang = a; P.pow = p; } },
    fire: () => { const P = you(); if (P && duel.phase === "aim" && duel.p[duel.turn] === P) fire(P); },
    solve: () => { const P = you(); return P ? plan({ ...P, shots: 99, skill: 0 }) : null; },   // the bots' aim with a perfect hand, for tests
    wind: (v) => { wind = v; },
  };
  return {
    open(a, b, key) {
      at = key || null;
      for (const k in held) delete held[k];
      if (b) { duel = { p: [gun(a, 0), gun(b, 1)], score: [0, 0], fired: [0, 0], watch: !a.human }; round(0); }
      else { duel = null; land = hills(false); wind = 0.3; }
      if (!last) requestAnimationFrame(frame);
    },
    close() { const w = duel && duel.shell && duel.shell.whistle; if (w && w.stop) w.stop(); duel = null; at = null; },
    // the duel at the window for the room: the bot you play (or the two you watch), when each last fired
    playing: () => duel && duel.phase !== "left" ? { id: duel.p[1].id, ids: duel.watch ? [duel.p[0].id, duel.p[1].id] : null, watch: duel.watch, at, fired: duel.fired.slice() } : null,
    left(id) {                               // a bot that gets work walks off; the duel is over
      if (!duel || duel.gone !== undefined) return;
      const P = duel.p.find((Q) => Q.id === id && !Q.human);
      if (!P) return;
      duel.gone = P.side; P.leaving = true; P.walk = 0;
      if (duel.phase === "aim") duel.phase = "left";
    },
  };
}
