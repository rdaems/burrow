// burrow: the window's view. Outside, by this computer's clock: the sky by the hour, the sun or the moon on
// their arc, stars after dark, clouds going with the wind; in the meadow in front the other machines (each a
// door in the room) as cottages in their door's paint: lit and smoking while their bots work, one lamp while
// they idle, dark with none, boarded up while out of reach; fireflies at night.
//
// And it is a game: on the hills two guns, an artillery duel in the manner of the old DOS games (VGABomb,
// Gorillas): you against an idle bot drawn by lot, or two bots' duel to watch. Angle and power, a wind that
// changes each round (the clouds, the smoke and the pennants go with it), craters where the shells come down;
// a gun hit is the round. A bot knows the physics and aims less well its first shots of a round.
//
// outsideView(canvas, app), app = { state(), room, shown(), wears(seed) }; returns
// { open(a, b, at), close(), playing(), left(id) }. a and b are { id, name, color, human, seed }; with b null
// it is only the outside. Everything is in dots, the canvas's pixels (the page shows four pixels a dot).
function outsideView(cv, app) {
  const W = cv.width, H = cv.height, c = cv.getContext("2d"), C = pixelAvatar.C;
  const M = 78, BASE = 89;                   // the meadow's top (the hills come down to it); the cottages' ground
  const G = 60, VK = 1.1, PUSH = 9, DT = 1 / 120, BOOM = 5;   // gravity (dots/s²), dots/s a power point, the wind's push at full strength, a step, a shell's reach
  const rnd = (a, b) => a + Math.random() * (b - a);
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const gauss = () => Math.sqrt(-2 * Math.log(1 - Math.random())) * Math.cos(2 * Math.PI * Math.random());
  const hash = (s) => { let h = 2166136261; for (const ch of String(s)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619); return (h >>> 0) / 4294967296; };
  const mixc = (a, b, k) => "#" + [1, 3, 5].map((i) => { const x = parseInt(a.slice(i, i + 2), 16), y = parseInt(b.slice(i, i + 2), 16); return Math.round(x + (y - x) * k).toString(16).padStart(2, "0"); }).join("");
  const r = (x, y, w, h, col) => { c.fillStyle = col; c.fillRect(x, y, w, h); };
  const disc = (x, y, rad, col) => { x = Math.round(x); y = Math.round(y); for (let j = -Math.floor(rad); j <= rad; j++) { const h = Math.floor(Math.sqrt(rad * rad - j * j)); r(x - h, y + j, 2 * h + 1, 1, col); } };

  let land = hills(false), wind = 0.3, duel = null, at = null, last = 0, acc = 0, T = 0;
  const clouds = [0, 1, 2].map((i) => ({ x: i * 57 + 10, y: 8 + i * 11, k: 0.8 + 0.2 * i }));
  const held = {};                           // the arrow keys held down: since when, and what of a step is owed

  // ---- the land: rolling hills at two levels, a mountain between them (a gentle rise without a duel)
  function hills(mountain) {
    const g = new Float32Array(W), a = rnd(0, 7), b = rnd(0, 7), l = rnd(-7, 7), rt = rnd(-7, 7);
    const xm = rnd(66, 94), hm = mountain ? rnd(16, 32) : rnd(4, 9), wm = rnd(14, 24);
    for (let x = 0; x < W; x++) {
      const u = x / (W - 1), lvl = 62 + l + (rt - l) * u * u * (3 - 2 * u);
      g[x] = clamp(lvl + 3 * Math.sin(x / 11 + a) + 2 * Math.sin(x / 5.3 + b) - hm * Math.exp(-(((x - xm) / wm) ** 2)), 22, M - 2);
    }
    return { g, scar: new Uint8Array(W) };
  }
  const ground = (x) => land.g[clamp(Math.round(x), 0, W - 1)];
  function crater(x, y, rad) {
    for (let i = Math.ceil(x - rad); i <= x + rad; i++) {
      if (i < 0 || i >= W) continue;
      const low = y + Math.sqrt(Math.max(0, rad * rad - (i - x) ** 2));
      if (land.g[i] < low) { land.g[i] = Math.min(M - 1, low); land.scar[i] = 1; }
    }
  }

  // ---- a duel: two guns, a turn each, the round to whoever hits the other's gun
  function gun(p, side) { return { ...p, side, f: side ? -1 : 1, x: 0, ang: 45, pow: 50, shots: 0, kick: 0, flash: 0, dead: false, body: null, trail: [], plan: null, skill: 0.75 + 0.6 * hash(p.id) }; }
  function round(starter) {
    land = hills(true);
    const s = Math.random() < 0.5 ? -1 : 1;
    wind = s * Math.pow(Math.random(), 1.4);
    for (const P of duel.p) {
      P.x = P.f > 0 ? Math.round(rnd(14, 32)) : W - 1 - Math.round(rnd(14, 32));
      const span = [P.x - 9 * P.f, P.x + 3 * P.f].sort((a, b) => a - b), h = Math.round(land.g.slice(span[0], span[1] + 1).reduce((a, b) => a + b, 0) / (span[1] - span[0] + 1));
      for (let x = span[0]; x <= span[1]; x++) land.g[x] = h;   // a level pad for the gun, its gunner and its pennant
      Object.assign(P, { shots: 0, kick: 0, flash: 0, dead: false, body: null, trail: [], plan: null });
    }
    Object.assign(duel, { turn: starter, phase: "aim", t: 0, shell: null, booms: [], bits: [] });
  }
  // a shell from P's muzzle: angle a in degrees over the ground toward the other side, power p
  function launch(P, a, p) {
    const rad = a * Math.PI / 180, dx = P.f * Math.cos(rad), dy = -Math.sin(rad), y0 = ground(P.x) - 3;
    return { x: P.x + dx * 6, y: y0 + dy * 6, vx: dx * p * VK, vy: dy * p * VK, by: P.side, age: 0 };
  }
  function fly(s) { s.vx += wind * PUSH * DT; s.vy += G * DT; s.x += s.vx * DT; s.y += s.vy * DT; s.age += DT; }
  // where it comes down: { x, y, gun } (gun: the side whose gun it hits), { out } off the side; null in the air
  function down(s) {
    if (s.x < -4 || s.x > W + 3 || s.y > H + 4) return { out: true };
    for (const Q of duel.p) {
      if (Q.dead || (Q.side === s.by && s.age < 0.25)) continue;
      const gy = ground(Q.x);
      if (Math.abs(s.x - Q.x) < 2.5 && s.y > gy - 5 && s.y < gy + 1) return { x: s.x, y: s.y, gun: Q.side };
    }
    if (s.x >= 0 && s.x < W && s.y >= ground(s.x)) return { x: s.x, y: ground(s.x) };
    return null;
  }
  function sim(P, a, p) { const s = launch(P, a, p); for (let k = 0; k < 1500; k++) { fly(s); const d = down(s); if (d) return d; } return { out: true }; }
  // a bot's aim: for every angle the power that comes closest, then its own way of shooting (an angle it
  // likes, seeded by its id), then the hand: far off the first shot of a round, closer with every shot
  function plan(P) {
    const Q = duel.p[1 - P.side], tx = Q.x, ty = ground(Q.x) - 2;
    const miss = (d) => (d.out ? 1e9 : d.gun === Q.side ? 0 : Math.hypot(d.x - tx, d.y - ty) + (d.gun === P.side ? 80 : 0));
    const best = [];
    for (let a = 20; a <= 80; a += 4) {
      let b = null;
      for (let p = 10; p <= 100; p += 1.5) { const d = miss(sim(P, a, p)); if (!b || d < b.d) b = { a, p, d }; }
      for (let p = b.p - 1.5; p <= b.p + 1.5; p += 0.25) { const d = miss(sim(P, a, p)); if (d < b.d) b = { a, p, d }; }
      best.push(b);
    }
    const likes = 30 + 40 * hash(P.id + "a"), ok = best.filter((b) => b.d < 3);
    const b = (ok.length ? ok : best).reduce((x, y) => (ok.length ? (Math.abs(y.a - likes) < Math.abs(x.a - likes) ? y : x) : y.d < x.d ? y : x));
    const k = P.shots, sp = (9 * Math.pow(0.55, k) + 0.5) * P.skill, sa = 2 * Math.pow(0.6, k) * P.skill;
    return { a: clamp(Math.round(b.a + gauss() * sa), 5, 85), p: clamp(Math.round(b.p + gauss() * sp), 5, 100), wait: rnd(0.6, 1.3) };
  }
  function fire(P) {
    duel.shell = launch(P, P.ang, P.pow);
    duel.phase = "fly"; P.shots++; P.kick = 1; P.flash = 0.12; P.trail = [];
    duel.fired[P.side] = performance.now() / 1000;
  }
  function boom(d) {
    const s = duel.shell, P = duel.p[s.by];
    duel.shell = null; duel.phase = "boom"; duel.t = 0.9;
    if (d.out) return;
    const hit = duel.p.filter((Q) => !Q.dead && (d.gun === Q.side || Math.hypot(d.x - Q.x, d.y - (ground(Q.x) - 2)) <= BOOM));
    duel.booms.push({ x: d.x, y: d.y, t: 0, R: hit.length ? 8 : 5 });
    for (let k = 0; k < (hit.length ? 26 : 14); k++) {
      const a = rnd(Math.PI * 1.05, Math.PI * 1.95), v = rnd(18, 46);
      duel.bits.push({ x: d.x, y: d.y - 1, vx: Math.cos(a) * v, vy: Math.sin(a) * v, t: 0, col: k % 3 ? (land.scar[Math.round(d.x)] ? "#6b4a2b" : "#4a7a3e") : "#3f3740" });
    }
    for (const Q of hit) { Q.dead = true; Q.body = { x: Q.x - 5 * Q.f, y: ground(Q.x) - 1, vx: -Q.f * rnd(14, 24), vy: -rnd(30, 42), down: false }; }
    crater(d.x, d.y, BOOM);
    if (hit.length) {
      const win = hit.length === 2 ? null : 1 - hit[0].side;
      if (win !== null) duel.score[win]++;
      duel.phase = "over"; duel.t = 3.6; duel.next = hit.length === 2 ? 1 - P.side : hit[0].side;   // the loser starts the next round
    }
  }

  // ---- the time going by: a step of the duel, then the clouds and the bits in the air
  function update(dt) {
    T += dt;
    for (const cl of clouds) { cl.x += wind * 8 * cl.k * dt; if (cl.x > W + 20) cl.x -= W + 40; if (cl.x < -20) cl.x += W + 40; }
    if (!duel) return;
    for (const P of duel.p) {
      P.kick = Math.max(0, P.kick - dt * 5); P.flash = Math.max(0, P.flash - dt);
      const b = P.body;
      if (b && !b.down) { b.vy += G * dt; b.x += b.vx * dt; b.y += b.vy * dt; b.x = clamp(b.x, 2, W - 3); if (b.vy > 0 && b.y >= ground(b.x) - 1) { b.y = ground(b.x) - 1; b.down = true; } }
      if (P.leaving) P.walk = (P.walk || 0) + dt * 14;
    }
    for (const bm of duel.booms) bm.t += dt;
    duel.booms = duel.booms.filter((bm) => bm.t < 1.8);
    for (const b of duel.bits) { b.t += dt; b.vy += G * dt; b.x += b.vx * dt; b.y += b.vy * dt; }
    duel.bits = duel.bits.filter((b) => b.t < 2 && b.x > -2 && b.x < W + 2 && b.y < ground(b.x));
    if (duel.shell) {
      acc += dt;
      while (duel.shell && acc >= DT) {
        acc -= DT; fly(duel.shell);
        const P = duel.p[duel.shell.by];
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
  const STARS = [...Array(64)].map((_, i) => [(i * 37) % W, (i * 53) % 62, i % 3]);
  // the gunners, a letter a dot, facing right: you as the portrait wears it, a bot a robot with its colour on
  // its antenna and chest
  const PERSON = [".hh.", "hss.", ".cc.", "cccs", ".cd.", ".ll.", ".l.l"];
  const ROBOT = ["..a.", "..m.", ".mm.", ".me.", "mmmm", "mkmd", ".mm.", ".m.m"];
  function figure(P, x, y, lying, hop) {     // x: its middle, y: the ground under it
    const w = app.wears && P.human ? app.wears(P.seed) : null;
    const pal = P.human ? { h: (w && w.hair) || C.ink2, s: (w && w.skin) || "#f7c59a", c: (w && w.shirt) || P.color, d: (w && w.shade) || P.color, l: C.ink2 }
      : { a: P.color, m: C.steel2, e: C.steel4, k: P.color, d: C.steel1 };
    const rows = P.human ? PERSON : ROBOT, h = rows.length, x0 = Math.round(x), y0 = Math.round(y) - hop;
    for (let j = 0; j < h; j++) for (let i = 0; i < 4; i++) {
      const ch = rows[j][i];
      if (ch === ".") continue;
      if (!lying) r(x0 - 2 + (P.f > 0 ? i : 3 - i), y0 - h + j, 1, 1, pal[ch]);   // facing its gun
      else r(x0 - (h >> 1) + (P.f > 0 ? j : h - 1 - j), y0 - 1 - i, 1, 1, pal[ch]);   // on its back, the head away from its gun
    }
  }
  function drawGun(P) {
    const gy = Math.round(ground(P.x)), f = P.f, x = P.x - Math.round(f * P.kick * 1.5);
    if (P.dead) {                            // the wreck, smoking
      r(x - 3, gy - 1, 7, 1, "#2a2420"); r(x - 1, gy - 2, 4, 1, "#3f3740"); r(x + 2 * f, gy - 3, 1, 1, "#5a3a22");
      for (let k = 0; k < 4; k++) { const ph = (T * 0.35 + k / 4) % 1; c.globalAlpha = 0.6 * (1 - ph); r(Math.round(x + ph * wind * 8 + Math.sin(ph * 6 + k)), Math.round(gy - 3 - ph * 16), 2, 2, "#5d6280"); c.globalAlpha = 1; }
      return;
    }
    r(x - 4 * f, gy - 1, 1, 1, "#5a3a22"); r(x - 3 * f, gy - 2, 1, 1, "#5a3a22"); r(x - 2 * f, gy - 2, 1, 1, "#5a3a22");   // the trail
    const rad = P.ang * Math.PI / 180, dx = f * Math.cos(rad), dy = -Math.sin(rad), px = f * Math.sin(rad), py = Math.cos(rad);
    for (let s = 0; s <= 6; s += 0.5) {      // the barrel, thicker at the breech
      const bx = Math.round(x + dx * s), by = Math.round(gy - 3 + dy * s);
      r(bx, by, 1, 1, s > 5 ? C.ink2 : "#3f3740");
      if (s < 4) r(Math.round(x + dx * s + px), Math.round(gy - 3 + dy * s + py), 1, 1, "#2a2530");
    }
    r(x - 1, gy - 3, 3, 3, "#5a3a22"); r(x, gy - 2, 1, 1, C.steel3);   // the wheel and its hub
    if (P.flash > 0) { const mx = Math.round(x + dx * 7.5), my = Math.round(gy - 3 + dy * 7.5); r(mx - 1, my, 3, 1, C.gold2); r(mx, my - 1, 1, 3, C.gold2); r(mx, my, 1, 1, C.white); }
  }
  function pennant(P, col) {                 // on a pole behind the gunner, streaming with the wind
    const gy = Math.round(ground(P.x)), x = P.x - 9 * P.f;
    r(x, gy - 9, 1, 9, C.steel2);
    const n = Math.abs(wind) < 0.08 ? 0 : 1 + Math.round(Math.abs(wind) * 5), s = Math.sign(wind), wave = Math.floor(T * (3 + Math.abs(wind) * 6)) % 2;
    if (!n) { r(x + 1, gy - 9, 1, 3, col); return; }
    for (let k = 0; k < n; k++) { const hh = Math.max(1, 3 - Math.floor((k * 3) / (n + 1))); r(x + s * (k + 1), gy - 9 + (k === n - 1 && wave ? 1 : 0), 1, hh, col); }
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
  function draw() {
    const now = new Date(), h = now.getHours() + now.getMinutes() / 60, sky = skyAt(h);
    const dark = h < 6 || h > 20.5 ? 1 : h < 8 ? (8 - h) / 2 : h > 18.5 ? (h - 18.5) / 2 : 0;
    for (let b = 0; b < 5; b++) r(0, b * 16, W, 16, mixc(sky, "#ffffff", (0.03 * b) * (1 - dark) + 0.012 * b));   // the sky, lighter toward the horizon
    if (dark > 0) for (const [x, y, k] of STARS) if (((Math.floor(T * 2) + k) % 5) || k === 0) r(x, y, 1, 1, dark > 0.5 ? "#f4f6fb" : "#8b93ad");
    // the sun from 6:30 to 20:30, the moon the rest of the night, on an arc over the hills
    const day = h >= 6.5 && h <= 20.5, p = day ? (h - 6.5) / 14 : ((h + 24 - 20.5) % 24) / 10;
    const sx = Math.round(12 + p * (W - 24)), sy = Math.round(60 - Math.sin(p * Math.PI) * 50);
    if (day) { r(sx - 3, sy - 2, 7, 5, "#ffe27a"); r(sx - 2, sy - 3, 5, 7, "#ffe27a"); r(sx - 1, sy - 1, 3, 3, "#fff6c0"); }
    else { r(sx - 3, sy - 2, 7, 5, "#e8e5ef"); r(sx - 2, sy - 3, 5, 7, "#e8e5ef"); r(sx, sy - 2, 4, 5, sky); }
    for (const cl of clouds) {               // clouds, going with the wind
      const cx = Math.round(cl.x) - 15, cy = cl.y, col = mixc("#f4f6fb", sky, 0.25 + dark * 0.6);
      r(cx, cy + 2, 18, 3, col); r(cx + 3, cy, 8, 2, col); r(cx + 12, cy + 1, 4, 1, col);
    }
    // the hills: grass along the top (earth where a shell came down), darker further in
    const grass = mixc("#4a7a3e", "#142019", dark), turf = mixc("#35603a", "#101a16", dark), earth = mixc("#6b4a2b", "#1a1410", dark);
    for (let x = 0; x < W; x++) {
      const top = Math.round(land.g[x]);
      for (let y = top; y < M; y += 4) r(x, y, 1, Math.min(4, M - y), mixc(mixc("#2b4a3a", "#1d3328", Math.min(1, (y - top) / 22)), "#0c1418", dark * 0.8));
      r(x, top, 1, 1, land.scar[x] ? earth : grass);
      r(x, top + 1, 1, 1, land.scar[x] ? mixc(earth, "#000000", 0.25) : turf);
    }
    if (duel) {
      // the last shot of each, a dotted line in its colour until its next; the shell in the air
      for (const P of duel.p) { c.globalAlpha = 0.45; for (const [x, y] of P.trail) r(Math.round(x), Math.round(y), 1, 1, P.color); c.globalAlpha = 1; }
      for (const P of duel.p) {
        pennant(P, P.color); drawGun(P);
        const lying = P.body && P.body.down, out = P.leaving && P.walk, bx = P.body ? P.body.x : P.x - 5 * P.f - (out ? P.f * P.walk : 0);
        const won = duel.phase === "over" && !P.dead ? Math.floor(T * 6) % 2 : 0;
        if (!(out && (bx < -4 || bx > W + 4))) figure(P, bx, P.body ? P.body.y + 1 : ground(bx), lying, won);
      }
      const s = duel.shell;                  // the shell; over the top of the picture, a dot on its edge where it is
      if (s) r(Math.round(s.x), Math.max(0, Math.round(s.y)), 1, 1, s.y < 0 ? C.gold2 : dark > 0.5 ? C.steel4 : C.ink);
      for (const b of duel.bits) r(Math.round(b.x), Math.round(b.y), 1, 1, b.col);
      for (const bm of duel.booms) {         // a flash, a ball of fire, then its smoke rising with the wind
        const t = bm.t, R = bm.R;
        if (t > 0.25) { c.globalAlpha = Math.max(0, 0.7 * (1 - (t - 0.25) / 1.55)); disc(bm.x + wind * 6 * (t - 0.25), bm.y - 2 - (t - 0.25) * 7, R * 0.6 * (1 - (t - 0.25) / 2.5), mixc("#8b93ad", "#3b3a55", dark)); c.globalAlpha = 1; }
        if (t < 0.5) {
          const k = Math.min(1, t / 0.12), fade = t > 0.3 ? 1 - (t - 0.3) / 0.2 : 1;
          disc(bm.x, bm.y, R * (0.5 + 0.5 * k) * (0.6 + 0.4 * fade), t < 0.08 ? C.white : C.red1);
          disc(bm.x, bm.y, R * (0.5 + 0.3 * k) * fade, t < 0.08 ? C.white : C.gold1);
          disc(bm.x, bm.y, R * 0.35 * fade, C.gold2);
        }
      }
    }
    // the meadow in front, a few pines, the other machines as cottages
    r(0, M, W, H - M, mixc("#1f3a2c", "#0c1418", dark)); r(0, M, W, 1, mixc("#2f5a3c", "#101a16", dark));
    const st = app.state(), others = ((st && st.machines) || []).filter((m) => m.id), span = (W - 24) / Math.max(1, others.length);
    const homes = others.map((m, i) => Math.round(12 + span * (i + 0.5)));
    const pine = mixc("#122619", "#060c09", dark);
    for (const [x, hh] of [[5, 16], [12, 12], [W / 2, 13], [W - 13, 15], [W - 6, 11]]) if (!homes.some((hx) => Math.abs(hx - x) < 13)) for (let k = 0; k < hh; k++) r(x - Math.floor(k / 3), BASE - hh + k, 1 + 2 * Math.floor(k / 3), 1, pine);
    const warm = "#ffd97a", glow = "rgba(255,217,122,0.35)";
    others.forEach((m, i) => {
      const p = app.room.paint(m.id), mine = st.bots.filter((b) => (b.m || "") === m.id);
      const busy = mine.some((b) => b.busy || b.waiting), lamp = mine.length > 0, cx = homes[i], x = cx - 8, base = BASE;
      const wall = mixc(p.top, "#0c1418", dark * 0.55), roof = mixc(p.d, "#0c1418", dark * 0.5), line = p.k;
      r(x + 11, base - 17, 3, 6, line); r(x + 12, base - 17, 1, 5, roof);                                  // the chimney
      for (let k = 0; k < 6; k++) r(x - 1 + k, base - 10 - k, 18 - 2 * k, 1, k === 5 ? line : roof);     // the roof
      r(x - 1, base - 10, 18, 1, line);
      r(x, base - 9, 16, 9, line); r(x + 1, base - 9, 14, 8, wall);                                     // the walls
      r(x + 6, base - 6, 4, 6, line); r(x + 7, base - 5, 2, 5, mixc(p.d, "#000000", 0.3));              // the door
      for (const [wx, on] of [[x + 2, busy || lamp], [x + 11, busy]]) {
        if (on && dark > 0.3) r(wx - 1, base - 8, 4, 4, glow);
        r(wx, base - 7, 2, 2, on ? warm : mixc("#26303c", "#0c1418", dark));
      }
      if (m.error) { r(x + 1, base - 7, 14, 1, "#6b4a2b"); r(x + 5, base - 4, 6, 1, "#6b4a2b"); }       // boarded up
      if (busy && !m.error) for (let k = 0; k < 6; k++) {                                                 // smoke, rising and going with the wind
        const ph = (T * 0.3 + k / 6) % 1, s = 2 + Math.round(ph * 3), sx = Math.round(x + 12 + ph * (2 + wind * 8) + Math.sin(ph * 5 + k) * 1.5) - (s >> 1), sy = Math.round(base - 18 - ph * 18);
        c.globalAlpha = 0.75 * (1 - ph); r(sx, sy, s, s - 1, mixc("#f1f3f7", "#8a93a3", dark)); c.globalAlpha = 1;
      }
      const name = m.name.slice(0, Math.max(1, Math.floor((span + 1) / 4))).toUpperCase(), tw = app.room.tinyW(name);
      app.room.tiny(c, name, Math.round(cx - tw / 2), base + 2, mixc("#f4f6fb", "#8b93ad", dark * 0.5));
    });
    if (dark > 0.5) for (let k = 0; k < 6; k++) {
      const fx = Math.round((k * 29 + Math.sin(T * 0.7 + k) * 6 + 7) % W), fy = Math.round(M + 4 + (k % 3) * 3 + Math.sin(T * 1.3 + k * 2) * 2);
      if (Math.floor(T * 1.5 + k) % 3) r(fx, fy, 1, 1, "#e8f59a");
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
    close() { duel = null; at = null; },
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
