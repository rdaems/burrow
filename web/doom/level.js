// A Doom level played, simply: the map from a WAD (wad.js), the player, Doom's monsters, items and specials, in
// Doom's units (a unit a texel) and its tics (35 a second). What it does, as Doom does it, more or less:
// - moving: against the lines (one-sided, blocking, an opening lower than you, a step higher than 24), sliding
//   along walls; your height is the highest floor under you;
// - specials: doors (opened by use, by walking over a line, keyed), lifts (down, a wait, up), floors that lower,
//   switches (their texture flips), the exit (switch or line: the level is done); sector lights that blink, glow
//   and flicker; floors that hurt (nukage);
// - things: monsters wake when they see you (or hear a shot), chase you in eight directions, shoot (zombiemen,
//   shotgun guys), throw fireballs (imps), bite (demons); they feel pain and die on Doom's frames; items are picked
//   up as you walk over them (health, armor, shells, keys); barrels explode;
// - the shotgun: seven pellets in a spread, five to fifteen each, puffs on the walls, blood on what is hit.
// What it leaves out: the other weapons, vertical aim (autoaim, as Doom's), sound, saving, the intermission.
//
// new DoomLevel(wad, mapname) → level: level.tic(cmd) a step; level.player, level.mobjs, level.map; level.done
// ("exit" or "dead") once it is over. The 3d view (fps.js) draws it.
window.DoomLevel = (() => {
  const TIC = 1 / 35, RAD = Math.PI / 180;
  const rnd = () => Math.random() * 256 | 0;           // P_Random, near enough
  // the things a map can have: sprite, size, what it is
  const M = (s, hp, speed, r, pain, atk, extra) => ({ s, mon: true, hp, speed, r, h: 56, pain, atk, solid: true, shootable: true, ...extra });
  const I = (s, frames, pick, extra) => ({ s, frames, pick, r: 20, h: 16, ...extra });
  const D = (s, frames, solid, extra) => ({ s, frames, solid, r: 16, h: 16, ...extra });
  const T = {
    3004: M("POSS", 20, 8, 20, 200, "pistol"), 9: M("SPOS", 30, 8, 20, 170, "shotgun"), 3001: M("TROO", 60, 8, 20, 200, "imp"),
    3002: M("SARG", 150, 10, 30, 180, "bite"), 58: M("SARG", 150, 10, 30, 180, "bite", { shadow: true }), 3006: M("SKUL", 100, 8, 16, 256, "bite", { fly: true, bright: true }),
    3005: M("HEAD", 400, 8, 31, 128, "caco", { fly: true }), 3003: M("BOSS", 1000, 8, 24, 50, "baron"),
    2035: { s: "BAR1", frames: "AB", r: 10, h: 42, solid: true, shootable: true, hp: 20, barrel: true },
    // items
    2011: I("STIM", "A", { health: 10, max: 100 }), 2012: I("MEDI", "A", { health: 25, max: 100 }), 2014: I("BON1", "ABCDCB", { health: 1, max: 200, always: true }),
    2015: I("BON2", "ABCDCB", { armor: 1, max: 200, always: true }), 2018: I("ARM1", "AB", { armor: 100, cls: 1 }), 2019: I("ARM2", "AB", { armor: 200, cls: 2 }),
    2013: I("SOUL", "ABCDCB", { health: 100, max: 200, always: true, bright: true }), 2008: I("SHEL", "A", { shells: 4 }), 2049: I("SBOX", "A", { shells: 20 }),
    2001: I("SHOT", "A", { shells: 8 }), 8: I("BPAK", "A", { shells: 4, backpack: true }), 5: I("BKEY", "AB", { key: "blue" }), 6: I("YKEY", "AB", { key: "yellow" }),
    13: I("RKEY", "AB", { key: "red" }), 40: I("BSKU", "AB", { key: "blue" }), 39: I("YSKU", "AB", { key: "yellow" }), 38: I("RSKU", "AB", { key: "red" }),
    2007: I("CLIP", "A"), 2048: I("AMMO", "A"), 2010: I("ROCK", "A"), 2046: I("BROK", "A"), 2047: I("CELL", "A"), 17: I("CELP", "A"),
    2002: I("MGUN", "A"), 2003: I("LAUN", "A"), 2004: I("PLAS", "A"), 2005: I("CSAW", "A"), 2006: I("BFUG", "A"),
    2023: I("PSTR", "A", { bright: true }), 2022: I("PINV", "ABCD", { bright: true }), 2024: I("PINS", "ABCD", { bright: true }), 2025: I("SUIT", "A", { bright: true }),
    2026: I("PMAP", "ABCDCB", { bright: true }), 2045: I("PVIS", "AB", { bright: true }),
    // decorations
    2028: D("COLU", "A", true, { bright: true }), 48: D("ELEC", "A", true), 43: D("TRE1", "A", true), 54: D("TRE2", "A", true, { r: 32 }), 47: D("SMIT", "A", true),
    30: D("COL1", "A", true), 31: D("COL2", "A", true), 32: D("COL3", "A", true), 33: D("COL4", "A", true), 36: D("COL5", "AB", true), 37: D("COL6", "A", true),
    41: D("CEYE", "ABCB", true, { bright: true }), 42: D("FSKU", "ABC", true, { bright: true }), 44: D("TBLU", "ABCD", true, { bright: true }), 45: D("TGRN", "ABCD", true, { bright: true }),
    46: D("TRED", "ABCD", true, { bright: true }), 55: D("SMBT", "ABCD", true, { bright: true }), 56: D("SMGT", "ABCD", true, { bright: true }), 57: D("SMRT", "ABCD", true, { bright: true }),
    34: D("CAND", "A", false, { bright: true }), 35: D("CBRA", "A", true, { bright: true }), 70: D("FCAN", "ABC", true, { bright: true }),
    49: D("GOR1", "ABCB", true, { hang: true }), 50: D("GOR2", "A", true, { hang: true }), 51: D("GOR3", "A", true, { hang: true }), 52: D("GOR4", "A", true, { hang: true }), 53: D("GOR5", "A", true, { hang: true }),
    63: D("GOR1", "ABCB", false, { hang: true }), 59: D("GOR2", "A", false, { hang: true }), 61: D("GOR3", "A", false, { hang: true }), 60: D("GOR4", "A", false, { hang: true }), 62: D("GOR5", "A", false, { hang: true }),
    24: D("POL5", "A", false), 25: D("POL1", "A", true), 26: D("POL6", "AB", true), 27: D("POL4", "A", true), 28: D("POL2", "A", true), 29: D("POL3", "AB", true, { bright: true }),
    15: D("PLAY", "N", false), 10: D("PLAY", "W", false), 12: D("PLAY", "W", false), 18: D("POSS", "L", false), 19: D("SPOS", "L", false), 20: D("TROO", "M", false),
    21: D("SARG", "N", false), 22: D("HEAD", "L", false),
  };
  // monster frames (Doom's states): [frame, tics, action]; F() a fullbright frame
  const F = (f) => f.toLowerCase();
  const STATES = {
    POSS: { idle: [["A", 10, "look"], ["B", 10, "look"]], walk: [..."AABBCCDD"].map((f) => [f, 4, "chase"]), atk: [["E", 10, "face"], [F("F"), 8, "shoot"], ["E", 8, "face"]],
      pain: [["G", 3], ["G", 3, "pain"]], die: [["H", 5], ["I", 5, "scream"], ["J", 5, "fall"], ["K", 5], ["L", -1]] },
    TROO: { idle: [["A", 10, "look"], ["B", 10, "look"]], walk: [..."AABBCCDD"].map((f) => [f, 3, "chase"]), atk: [["E", 8, "face"], ["F", 8, "face"], ["G", 6, "shoot"]],
      pain: [["H", 2], ["H", 2, "pain"]], die: [["I", 8], ["J", 8, "scream"], ["K", 6], ["L", 6, "fall"], ["M", -1]] },
    SARG: { idle: [["A", 10, "look"], ["B", 10, "look"]], walk: [..."AABBCCDD"].map((f) => [f, 2, "chase"]), atk: [["E", 8, "face"], ["F", 8, "face"], ["G", 8, "shoot"]],
      pain: [["H", 2], ["H", 2, "pain"]], die: [["I", 8], ["J", 8, "scream"], ["K", 4], ["L", 4, "fall"], ["M", 4], ["N", -1]] },
    HEAD: { idle: [["A", 10, "look"]], walk: [["A", 3, "chase"]], atk: [["B", 5, "face"], ["C", 5, "face"], [F("D"), 5, "shoot"]],
      pain: [["E", 3], ["E", 3, "pain"], ["F", 6]], die: [["G", 8], ["H", 8, "scream"], ["I", 8], ["J", 8], ["K", 8, "fall"], ["L", -1]] },
    BOSS: { idle: [["A", 10, "look"], ["B", 10, "look"]], walk: [..."AABBCCDD"].map((f) => [f, 3, "chase"]), atk: [["E", 8, "face"], ["F", 8, "face"], ["G", 8, "shoot"]],
      pain: [["H", 2], ["H", 2, "pain"]], die: [["I", 8], ["J", 8, "scream"], ["K", 8], ["L", 8, "fall"], ["M", 8], ["N", 8], ["O", -1]] },
    SKUL: { idle: [[F("A"), 10, "look"], [F("B"), 10, "look"]], walk: [[F("A"), 6, "chase"], [F("B"), 6, "chase"]], atk: [[F("C"), 10, "face"], [F("D"), 4, "shoot"], [F("C"), 4], [F("D"), 4]],
      pain: [[F("E"), 3], [F("E"), 3, "pain"]], die: [[F("F"), 6], [F("G"), 6, "scream"], [F("H"), 6], [F("I"), 6, "fall"], ["J", 6], ["K", 6, "gone"]] },
  };
  STATES.SPOS = STATES.POSS;
  const BAR = { idle: [["A", 6], ["B", 6]], die: [[F("C"), 5], [F("D"), 5, "explode"], [F("E"), 10], [F("F"), 10, "gone"]] };   // BAR1 then BEXP
  const MISSILES = { imp: { s: "BAL1", speed: 10, r: 6, dmg: 8, fly: [F("A"), F("B")], die: [F("C"), F("D"), F("E")] },
    caco: { s: "BAL2", speed: 10, r: 6, dmg: 8, fly: [F("A"), F("B")], die: [F("C"), F("D"), F("E")] },
    baron: { s: "BAL7", speed: 15, r: 6, dmg: 8, fly: [F("A"), F("B")], die: [F("C"), F("D"), F("E")] } };

  class Level {
    constructor(wad, name) {
      this.wad = wad;
      this.map = wad.map(name);
      this.name = name;
      this.time = 0;
      this.mobjs = [];
      this.thinkers = [];
      this.puffs = [];
      this.done = null;
      this.kills = 0;
      const m = this.map;
      for (const s of m.sectors) { s.lines = []; s.light0 = s.light; s.floor0 = s.floor; s.ceil0 = s.ceil; s.mover = null; }
      for (const l of m.lines) {
        l.x0 = Math.min(l.v1.x, l.v2.x); l.x1 = Math.max(l.v1.x, l.v2.x); l.y0 = Math.min(l.v1.y, l.v2.y); l.y1 = Math.max(l.v1.y, l.v2.y);
        l.dx = l.v2.x - l.v1.x; l.dy = l.v2.y - l.v1.y;
        for (const sd of [l.front, l.back]) if (sd && !sd.sector.lines.includes(l)) sd.sector.lines.push(l);
      }
      // a grid of the lines, 128 units a cell, for what a box or a ray meets
      const xs = m.vertexes.map((v) => v.x), ys = m.vertexes.map((v) => v.y);
      this.gx = Math.min(...xs) - 8; this.gy = Math.min(...ys) - 8;
      this.gw = Math.ceil((Math.max(...xs) - this.gx + 16) / 128); this.gh = Math.ceil((Math.max(...ys) - this.gy + 16) / 128);
      this.grid = Array.from({ length: this.gw * this.gh }, () => []);
      for (const l of m.lines) for (let cy = this.cell(l.y0, this.gy); cy <= this.cell(l.y1, this.gy); cy++) for (let cx = this.cell(l.x0, this.gx); cx <= this.cell(l.x1, this.gx); cx++) this.grid[cy * this.gw + cx].push(l);
      // things: the player's start, monsters and items for skill 3 (Hurt me plenty), not the multiplayer ones
      const start = m.things.find((t) => t.type === 1);
      this.player = this.spawnPlayer(start || { x: 0, y: 0, angle: 0 });   // first: the monsters look for you as they come
      for (const t of m.things) {
        if (t.type <= 4 || t.type === 11) continue;
        if (t.flags & 16 || !(t.flags & 2)) continue;
        const d = T[t.type];
        if (d) this.spawn(t.type, d, t.x, t.y, t.angle * RAD, !!(t.flags & 8));
      }
      this.anims = this.animations();
    }
    cell(v, o) { return Math.max(0, Math.floor((v - o) / 128)); }
    linesNear(x0, y0, x1, y1) {
      const out = new Set();
      for (let cy = this.cell(y0, this.gy); cy <= Math.min(this.gh - 1, this.cell(y1, this.gy)); cy++)
        for (let cx = this.cell(x0, this.gx); cx <= Math.min(this.gw - 1, this.cell(x1, this.gx)); cx++) for (const l of this.grid[cy * this.gw + cx]) out.add(l);
      return out;
    }
    // ---- the BSP: which subsector (and sector) a point is in
    side(n, x, y) { return (x - n.x) * n.dy - (y - n.y) * n.dx > 0 ? 0 : 1; }   // 0 the front (right) side, as Doom's R_PointOnSide
    subsector(x, y) {
      const m = this.map;
      if (!m.nodes.length) return m.ssectors[0];
      let n = m.nodes.length - 1;
      for (;;) {
        const node = m.nodes[n], k = node.kids[this.side(node, x, y)];
        if (k & 0x8000) return m.ssectors[k & 0x7fff];
        n = k;
      }
    }
    sectorAt(x, y) { return this.subsector(x, y).sector; }

    spawnPlayer(t) {
      const s = this.sectorAt(t.x, t.y);
      return { player: true, x: t.x, y: t.y, z: s.floor, angle: t.angle * RAD, mx: 0, my: 0, r: 16, h: 56, health: 100, armor: 0, armorClass: 0,
        shells: 20, maxShells: 50, keys: {}, viewh: 41, dviewh: 0, damagecount: 0, bonuscount: 0, attacker: null, dead: false, sector: s, kills: 0 };
    }
    spawn(type, d, x, y, angle, ambush) {
      const s = this.sectorAt(x, y);
      const o = { type, d, x, y, z: d.hang ? s.ceil - 64 : d.fly ? s.floor + 24 : s.floor, angle, r: d.r, h: d.h, sector: s, hp: d.hp || 0, ambush,
        frame: (d.frames || "A")[0], tics: 0, si: 0, state: d.mon ? "idle" : d.barrel ? "idle" : null, target: null, movedir: -1, movecount: 0,
        reaction: 8, solid: !!d.solid, shootable: !!d.shootable, alive: true, bright: !!d.bright };
      if (d.mon || d.barrel) this.setState(o, "idle");
      this.mobjs.push(o);
      return o;
    }
    states(o) { return o.d.barrel ? BAR : STATES[o.d.s] || STATES.POSS; }
    setState(o, name, i = 0) {
      const seq = this.states(o)[name];
      if (!seq) return;
      o.state = name; o.si = i;
      const [f, tics, act] = seq[i];
      o.bright = f === f.toLowerCase() || !!o.d.bright;
      o.frame = f.toUpperCase();
      o.sprite = o.d.barrel && name === "die" ? "BEXP" : o.d.s;
      o.tics = tics < 0 ? -1 : tics;
      if (act) this.act(o, act);
    }
    nextState(o) {
      const seq = this.states(o)[o.state];
      if (o.si + 1 < seq.length) this.setState(o, o.state, o.si + 1);
      else if (o.state === "die") o.tics = -1;
      else this.setState(o, o.state === "idle" ? "idle" : "walk");
    }

    // ---- moving: can a thing of radius r stand at x, y; where its floor and ceiling are there
    check(o, x, y) {
      const r = o.r, sec = this.sectorAt(x, y);
      let floor = sec.floor, ceil = sec.ceil, drop = sec.floor;
      for (const l of this.linesNear(x - r, y - r, x + r, y + r)) {
        if (x + r <= l.x0 || x - r >= l.x1 || y + r <= l.y0 || y - r >= l.y1) continue;
        if (this.boxSide(l, x, y, r) !== -1) continue;   // the box is wholly on one side: it does not touch the line
        if (!l.back) return null;                        // a one-sided line: a wall
        if (l.flags & 1 && !o.missile) return null;       // blocking
        if (o.d && o.d.mon && l.flags & 2) return null;  // blocks monsters
        const f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil);
        floor = Math.max(floor, f); ceil = Math.min(ceil, c); drop = Math.min(drop, l.front.sector.floor, l.back.sector.floor);
      }
      if (ceil - floor < o.h && !o.missile) return null;
      if (!o.missile && !o.d?.fly && floor - o.z > 24) return null;                  // too high a step
      if (o.d && o.d.mon && !o.d.fly && floor - drop > 24) return null;            // a monster does not walk off a ledge
      if (o.missile && (o.z < floor || o.z + 8 > ceil)) return null;
      for (const t of this.mobjs) {                                                  // solid things
        if (t === o || !t.solid || t === o.owner) continue;
        if (Math.abs(t.x - x) < t.r + r && Math.abs(t.y - y) < t.r + r) {
          if (o.missile) { o.hit = t; return null; }
          if (o.player && t.z + t.h <= o.z + 24 && !t.d.mon) continue;
          return null;
        }
      }
      if (!o.player && !o.missile && this.player && !this.player.dead && Math.abs(this.player.x - x) < this.player.r + r && Math.abs(this.player.y - y) < this.player.r + r) return null;
      if (o.missile && this.player && o.owner !== this.player && Math.abs(this.player.x - x) < this.player.r + r && Math.abs(this.player.y - y) < this.player.r + r && o.z < this.player.z + this.player.h && o.z + 8 > this.player.z) { o.hit = this.player; return null; }
      return { floor, ceil, sector: sec };
    }
    boxSide(l, x, y, r) {                            // -1 if the box straddles the line
      const s = (px, py) => l.dx * (py - l.v1.y) - l.dy * (px - l.v1.x) > 0;
      const a = s(x - r, y - r), b = s(x + r, y - r), c = s(x - r, y + r), d = s(x + r, y + r);
      return a === b && b === c && c === d ? +a : -1;
    }
    tryMove(o, x, y) {
      const at = this.check(o, x, y);
      if (!at) return false;
      const ox = o.x, oy = o.y;
      o.x = x; o.y = y; o.sector = at.sector; o.floorz = at.floor; o.ceilz = at.ceil;
      if (!o.missile && !(o.d && o.d.fly)) o.z = at.floor;
      if (o.player) this.crossLines(o, ox, oy);
      return true;
    }
    // Doom's slide, simply: the whole move, else along x, else along y
    slide(o, dx, dy) {
      if (this.tryMove(o, o.x + dx, o.y + dy)) return;
      if (this.tryMove(o, o.x + dx, o.y)) { o.my = 0; return; }
      if (this.tryMove(o, o.x, o.y + dy)) { o.mx = 0; return; }
      o.mx = o.my = 0;
    }
    // walking over a line with a special
    crossLines(o, ox, oy) {
      for (const l of this.linesNear(Math.min(ox, o.x) - 1, Math.min(oy, o.y) - 1, Math.max(ox, o.x) + 1, Math.max(oy, o.y) + 1)) {
        if (!l.special || !WALK.has(l.special)) continue;
        const s0 = l.dx * (oy - l.v1.y) - l.dy * (ox - l.v1.x), s1 = l.dx * (o.y - l.v1.y) - l.dy * (o.x - l.v1.x);
        if ((s0 > 0) === (s1 > 0)) continue;
        const t0 = (o.x - ox) * (oy - l.v1.y) - (o.y - oy) * (ox - l.v1.x), t1 = (o.x - ox) * (oy - l.v2.y) - (o.y - oy) * (ox - l.v2.x);
        if ((t0 > 0) === (t1 > 0)) continue;           // crossed the line's extension, not the line
        this.trigger(l, o, "walk");
      }
    }

    // ---- what you see along a ray: the first line that stops it (one-sided, or an opening that does not take the
    // height z), within dist; for the shotgun, sight and use
    trace(x, y, ang, dist, zAt) {
      const dx = Math.cos(ang), dy = Math.sin(ang), x2 = x + dx * dist, y2 = y + dy * dist;
      let best = dist, line = null;
      for (const l of this.linesNear(Math.min(x, x2), Math.min(y, y2), Math.max(x, x2), Math.max(y, y2))) {
        const den = dx * l.dy - dy * l.dx;
        if (Math.abs(den) < 1e-9) continue;
        const t = ((l.v1.x - x) * l.dy - (l.v1.y - y) * l.dx) / den, u = ((l.v1.x - x) * dy - (l.v1.y - y) * dx) / den;
        if (t <= 0.01 || t >= best || u < 0 || u > 1) continue;
        if (zAt) {
          if (l.back) {
            const z = zAt(t), f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil);
            if (z > f && z < c) continue;               // through the opening
          }
        } else if (l.back && !l.special) continue;
        best = t; line = l;
      }
      return { t: best, line, x: x + dx * best, y: y + dy * best };
    }
    sight(a, b) {                                      // can a see b: nothing in between closes off the line from eye to middle
      const dx = b.x - a.x, dy = b.y - a.y, d = Math.hypot(dx, dy) || 1, z0 = a.z + (a.player ? a.viewh : a.h * 0.75), z1 = b.z + b.h / 2;
      return this.trace(a.x, a.y, Math.atan2(dy, dx), d, (t) => z0 + (z1 - z0) * t / d).t >= d - 1;
    }

    // ---- specials
    trigger(l, by, how) {
      const sp = l.special, def = SPECIALS[sp];
      if (!def || def.how !== how) return false;
      if (def.key && by.player && !by.keys[def.key]) return false;   // a locked door
      if (by.d && by.d.mon && !def.monster) return false;
      let did = false;
      if (def.door) {
        const secs = def.manual ? (l.back ? [l.back.sector] : []) : this.tagged(l.tag);
        for (const s of secs) did = this.door(s, def.door, def.fast) || did;
      } else if (def.plat) for (const s of this.tagged(l.tag)) did = this.plat(s, def.fast) || did;
      else if (def.floor) for (const s of this.tagged(l.tag)) did = this.floorMove(s, def.floor) || did;
      else if (def.exit) { this.done = "exit"; did = true; }
      if (def.once && (did || def.exit)) l.special = 0;
      if (how === "use" && did) this.switchTex(l, !def.once);
      return did;
    }
    tagged(tag) { return tag ? this.map.sectors.filter((s) => s.tag === tag) : []; }
    neighbours(s) { return s.lines.map((l) => (l.front.sector === s ? l.back : l.front)).filter(Boolean).map((sd) => sd.sector); }
    door(s, kind, fast) {
      if (s.mover) {                                   // a door going down: back up (DR doors used again)
        if (s.mover.kind === "door" && s.mover.dir === -1 && kind === "raise") { s.mover.dir = 1; return true; }
        return false;
      }
      const ns = this.neighbours(s), top = (ns.length ? Math.min(...ns.map((n) => n.ceil)) : s.ceil0) - 4;
      if (kind === "close") s.mover = { kind: "door", dir: -1, top: s.ceil, speed: fast ? 8 : 2, wait: 0, mode: "close" };
      else s.mover = { kind: "door", dir: 1, top, speed: fast ? 8 : 2, wait: 0, mode: kind };
      return true;
    }
    plat(s, fast) {
      if (s.mover) return false;
      const ns = this.neighbours(s), low = Math.min(s.floor, ...ns.map((n) => n.floor));
      s.mover = { kind: "plat", dir: -1, low, high: s.floor, speed: fast ? 8 : 4, wait: 0 };
      return true;
    }
    floorMove(s, kind) {
      if (s.mover) return false;
      const ns = this.neighbours(s);
      if (!ns.length) return false;
      const to = kind === "lowest" ? Math.min(s.floor, ...ns.map((n) => n.floor)) : kind === "highest" ? Math.max(...ns.map((n) => n.floor)) + (kind === "highest8" ? 8 : 0) : s.floor;
      s.mover = { kind: "floor", to, speed: 1 };
      return true;
    }
    switchTex(l, back) {
      const sd = l.front;
      for (const k of ["upper", "mid", "lower"]) {
        const t = sd[k];
        if (/^SW1/.test(t)) { sd[k] = "SW2" + t.slice(3); if (back) this.thinkers.push({ t: 35, fn: () => { sd[k] = t; } }); return; }
        if (/^SW2/.test(t)) { sd[k] = "SW1" + t.slice(3); return; }
      }
    }
    moveSectors() {
      for (const s of this.map.sectors) {
        const m = s.mover;
        if (!m) continue;
        if (m.kind === "door") {
          if (m.wait > 0) { if (--m.wait === 0) m.dir = -1; continue; }
          if (m.dir === 1) {
            s.ceil = Math.min(m.top, s.ceil + m.speed);
            if (s.ceil >= m.top) { if (m.mode === "raise") { m.wait = 150; m.dir = 0; } else s.mover = null; }
          } else if (m.dir === -1) {
            const nc = Math.max(s.floor, s.ceil - m.speed);
            if (this.crushes(s, nc)) { if (m.mode !== "close") m.dir = 1; continue; }
            s.ceil = nc;
            if (s.ceil <= s.floor) s.mover = null;
          }
        } else if (m.kind === "plat") {
          if (m.wait > 0) { if (--m.wait === 0) m.dir = m.dir === -1 ? 1 : -1; continue; }
          if (m.dir === -1) { s.floor = Math.max(m.low, s.floor - m.speed); if (s.floor <= m.low) { m.wait = 105; m.dir = -1; m.next = 1; } }
          else { const nf = Math.min(m.high, s.floor + m.speed); if (this.crushes(s, s.ceil, nf)) { m.dir = -1; continue; } s.floor = nf; if (s.floor >= m.high) s.mover = null; }
          if (m.next && m.wait === 0) { m.dir = m.next; m.next = 0; }
        } else if (m.kind === "floor") {
          s.floor += Math.sign(m.to - s.floor) * Math.min(m.speed, Math.abs(m.to - s.floor));
          if (s.floor === m.to) s.mover = null;
        }
        for (const o of [this.player, ...this.mobjs]) {   // what stands on a moving floor rides it (you: the highest floor under you)
          if (!o || o.missile || (o.d && (o.d.fly || o.d.hang)) || this.sectorAt(o.x, o.y) !== s) continue;
          const at = o.player ? this.check(o, o.x, o.y) : null;
          o.z = at ? at.floor : s.floor;
        }
      }
    }
    crushes(s, ceil, floor = s.floor) {               // would something no longer fit
      for (const o of [this.player, ...this.mobjs]) if (o && o.alive !== false && !o.missile && (o.player || o.solid) && this.sectorAt(o.x, o.y) === s && ceil - floor < o.h) return true;
      return false;
    }
    lights() {
      const t = this.time;
      for (const s of this.map.sectors) {
        const sp = s.special;
        if (!sp || sp === 9 || sp === 7 || sp === 5 || sp === 16) continue;
        const ns = this.neighbours(s), low = Math.min(s.light0, ...ns.map((n) => n.light0));
        const lo = low === s.light0 ? 0 : low, hi = s.light0;
        if (sp === 1) { if (!s.blink || --s.blink <= 0) { s.light = s.light === hi ? lo : hi; s.blink = s.light === hi ? (rnd() & 64) + 1 : (rnd() & 7) + 1; } }
        else if (sp === 2 || sp === 4 || sp === 12) s.light = (t + (sp === 12 ? 0 : s.tag * 7)) % 35 < 5 ? hi : lo;
        else if (sp === 3 || sp === 13) s.light = (t + (sp === 13 ? 0 : s.tag * 7)) % 70 < 5 ? hi : lo;
        else if (sp === 8) { const k = (t % 64) / 32; s.light = Math.round(lo + (hi - lo) * (k < 1 ? k : 2 - k)); }
        else if (sp === 17) { if (!s.blink || --s.blink <= 0) { s.light = Math.max(lo + 16, hi - (rnd() & 3) * 16); s.blink = 4; } }
      }
    }
    animations() {                                   // Doom's animated flats and textures: a frame every 8 tics
      const seqs = [["NUKAGE1", "NUKAGE2", "NUKAGE3"], ["FWATER1", "FWATER2", "FWATER3", "FWATER4"], ["SWATER1", "SWATER2", "SWATER3", "SWATER4"],
        ["LAVA1", "LAVA2", "LAVA3", "LAVA4"], ["BLOOD1", "BLOOD2", "BLOOD3"], ["RROCK05", "RROCK06", "RROCK07", "RROCK08"], ["SLIME01", "SLIME02", "SLIME03", "SLIME04"],
        ["SLIME05", "SLIME06", "SLIME07", "SLIME08"], ["SLIME09", "SLIME10", "SLIME11", "SLIME12"], ["BLODGR1", "BLODGR2", "BLODGR3", "BLODGR4"],
        ["SLADRIP1", "SLADRIP2", "SLADRIP3"], ["BLODRIP1", "BLODRIP2", "BLODRIP3", "BLODRIP4"], ["FIREBLU1", "FIREBLU2"], ["ROCKRED1", "ROCKRED2", "ROCKRED3"],
        ["BFALL1", "BFALL2", "BFALL3", "BFALL4"], ["SFALL1", "SFALL2", "SFALL3", "SFALL4"], ["WFALL1", "WFALL2", "WFALL3", "WFALL4"], ["DBRAIN1", "DBRAIN2", "DBRAIN3", "DBRAIN4"],
        ["GSTFONT1", "GSTFONT2", "GSTFONT3"], ["FIRELAVA", "FIRELAV2"], ["FIREMAG1", "FIREMAG2", "FIREMAG3"]];
      const at = new Map();
      for (const q of seqs) q.forEach((n, i) => at.set(n, { q, i }));
      return (name) => { const a = at.get(name); return a ? a.q[(a.i + Math.floor(this.time / 8)) % a.q.length] : name; };
    }

    // ---- the player's turn: cmd = {forward, side, turn (radians), fire, use, run}
    tic(cmd) {
      if (this.done) return;
      this.time++;
      const p = this.player;
      if (!p.dead) {
        p.angle += cmd.turn || 0;
        const fw = (cmd.forward || 0) * (cmd.run ? 50 : 25) / 32, sd = (cmd.side || 0) * (cmd.run ? 40 : 24) / 32;   // Doom's thrust, in units a tic
        const ca = Math.cos(p.angle), sa = Math.sin(p.angle);
        if (p.z <= (p.floorz ?? p.z) + 0.01) { p.mx += ca * fw + sa * sd; p.my += sa * fw - ca * sd; }
        const sp = Math.hypot(p.mx, p.my);
        if (sp > 30) { p.mx *= 30 / sp; p.my *= 30 / sp; }
        if (p.mx || p.my) this.slide(p, p.mx, p.my);
        p.mx *= 0.90625; p.my *= 0.90625;               // friction, 0xE800
        if (Math.abs(p.mx) < 0.0625 && Math.abs(p.my) < 0.0625 && !cmd.forward && !cmd.side) p.mx = p.my = 0;
        p.bob = Math.min(16, (p.mx * p.mx + p.my * p.my) / 4);
        if (cmd.use) this.use(p);
        const s = this.sectorAt(p.x, p.y);
        p.sector = s;
        if (s.special === 9) { s.special = 0; p.secret = (p.secret || 0) + 1; }
        if ((s.special === 7 || s.special === 5 || s.special === 16 || s.special === 4) && p.z <= s.floor && this.time % 32 === 0) this.damage(p, s.special === 7 ? 5 : s.special === 5 ? 10 : 20, null);
        this.pickup(p);
      } else {
        p.viewh = Math.max(6, p.viewh - 1);           // the view falls to the floor
        p.mx *= 0.9; p.my *= 0.9;
      }
      if (p.damagecount) p.damagecount--;
      if (p.bonuscount) p.bonuscount--;
      this.moveSectors();
      this.lights();
      for (const th of this.thinkers.splice(0)) { if (--th.t > 0) this.thinkers.push(th); else th.fn(); }
      for (const o of this.mobjs.slice()) this.think(o);
      this.mobjs = this.mobjs.filter((o) => !o.gone);
      this.puffs = this.puffs.filter((f) => ++f.t < 16);
    }
    use(p) {                                          // the line in front of you, within 64 units
      const ang = p.angle, x2 = p.x + Math.cos(ang) * 64, y2 = p.y + Math.sin(ang) * 64;
      let best = 1e9, hit = null;
      for (const l of this.linesNear(Math.min(p.x, x2), Math.min(p.y, y2), Math.max(p.x, x2), Math.max(p.y, y2))) {
        const dx = Math.cos(ang), dy = Math.sin(ang), den = dx * l.dy - dy * l.dx;
        if (Math.abs(den) < 1e-9) continue;
        const t = ((l.v1.x - p.x) * l.dy - (l.v1.y - p.y) * l.dx) / den, u = ((l.v1.x - p.x) * dy - (l.v1.y - p.y) * dx) / den;
        if (t < 0 || t > 64 || u < 0 || u > 1 || t >= best) continue;
        if (l.special || !l.back) { best = t; hit = l; }
        else { const f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil); if (c - f <= 0) { best = t; hit = l; } }
      }
      if (hit && hit.special) return this.trigger(hit, p, "use");
      return false;
    }
    pickup(p) {
      for (const o of this.mobjs) {
        const d = o.d;
        if (!d.pick && !d.health && !d.armor && !d.shells && !d.key) continue;
        if (Math.abs(o.x - p.x) > p.r + o.r || Math.abs(o.y - p.y) > p.r + o.r || Math.abs(o.z - p.z) > 24) continue;
        let took = false;
        if (d.health && (d.always || p.health < d.max)) { p.health = Math.min(d.max, p.health + d.health); took = true; }
        if (d.armor && (d.always ? p.armor < d.max : p.armor < d.armor)) { p.armor = d.always ? Math.min(d.max, p.armor + d.armor) : d.armor; p.armorClass = Math.max(p.armorClass || 1, d.cls || 1); took = true; }
        if (d.backpack) { p.maxShells = 100; took = true; }
        if (d.shells && p.shells < p.maxShells) { p.shells = Math.min(p.maxShells, p.shells + d.shells); took = true; }
        if (d.key && !p.keys[d.key]) { p.keys[d.key] = true; took = true; }
        if (took) { o.gone = true; p.bonuscount = Math.min(p.bonuscount + 6, 40); }
      }
    }

    // ---- shooting: the shotgun's seven pellets from the player, Doom's spread
    fire(p) {
      if (p.dead || p.shells <= 0) return false;
      p.shells--;
      const eye = p.z + p.viewh - 8, target = this.aim(p);
      for (let k = 0; k < 7; k++) this.bullet(p, p.angle + ((rnd() - rnd()) / 255) * 5.6 * RAD, eye, target, 5 * ((rnd() % 3) + 1));
      this.noise(p);
      return true;
    }
    aim(p) {                                          // autoaim: the nearest thing to shoot within a few degrees of where you look
      let best = null, bd = 2048;
      for (const o of this.mobjs) {
        if (!o.shootable || !o.alive) continue;
        const dx = o.x - p.x, dy = o.y - p.y, d = Math.hypot(dx, dy), da = Math.abs(((Math.atan2(dy, dx) - p.angle + 3 * Math.PI) % (2 * Math.PI)) - Math.PI);
        if (d < bd && da < Math.atan2(o.r + 4, d) + 4 * RAD && this.sight(p, o)) { best = o; bd = d; }
      }
      return best;
    }
    bullet(src, ang, z0, target, dmg) {
      const range = 2048, zAt = target ? (t) => z0 + (target.z + target.h / 2 - z0) * t / Math.max(1, Math.hypot(target.x - src.x, target.y - src.y)) : () => z0;
      const wall = this.trace(src.x, src.y, ang, range, zAt);
      let hit = null, ht = wall.t;
      const dx = Math.cos(ang), dy = Math.sin(ang);
      for (const o of [...this.mobjs, this.player]) {
        if (!o || o === src || !o.alive && !o.player || !(o.shootable || o.player) || o.dead) continue;
        const rx = o.x - src.x, ry = o.y - src.y, t = rx * dx + ry * dy;
        if (t <= 0 || t >= ht) continue;
        if (Math.abs(rx * dy - ry * dx) > o.r) continue;
        const z = zAt(t);
        if (z < o.z - 4 || z > o.z + o.h + 4) continue;
        hit = o; ht = t;
      }
      const x = src.x + dx * (ht - 2), y = src.y + dy * (ht - 2), z = zAt(ht);
      if (hit) { this.damage(hit, dmg, src); if (!hit.d || !hit.d.barrel) this.puffs.push({ x, y, z, t: 0, blood: true }); }
      else if (wall.line || ht < range) this.puffs.push({ x, y, z, t: 0, blood: false });
    }
    noise(p) {                                        // a shot wakes the monsters that are near enough to hear it
      for (const o of this.mobjs) if (o.d.mon && o.alive && o.state === "idle" && !o.ambush && Math.hypot(o.x - p.x, o.y - p.y) < 1200) { o.target = p; this.setState(o, "walk"); }
    }
    damage(o, dmg, src) {
      if (o.player) {
        if (o.dead) return;
        if (o.armor) { const save = Math.min(o.armor, Math.floor(dmg / (o.armorClass === 2 ? 2 : 3))); o.armor -= save; dmg -= save; }
        o.health -= dmg; o.damagecount = Math.min(100, o.damagecount + dmg); o.attacker = src; o.hurt = this.time;
        if (o.health <= 0) { o.health = 0; o.dead = true; this.done = null; o.deadAt = this.time; }
        return;
      }
      if (!o.alive) return;
      o.hp -= dmg;
      if (src && o.d.mon) o.target = this.player;
      if (o.hp <= 0) {
        o.alive = false; o.shootable = false;
        if (o.d.mon) { this.kills++; this.player.kills++; this.setState(o, "die"); }
        else if (o.d.barrel) this.setState(o, "die");
        return;
      }
      if (o.d.mon && rnd() < o.d.pain) this.setState(o, "pain");
      else if (o.d.mon && o.state === "idle") this.setState(o, "walk");
    }
    // ---- a thing's tic
    think(o) {
      if (o.missile) return this.fly(o);
      if (o.d.frames && !o.d.mon && !o.d.barrel) { o.frame = o.d.frames[Math.floor(this.time / 6) % o.d.frames.length]; return; }
      if (o.tics === -1) return;
      if (o.tics > 0 && --o.tics > 0) return;
      this.nextState(o);
    }
    act(o, a) {
      const p = this.player;
      if (a === "look") {
        if (p.dead) return;
        const ang = Math.atan2(p.y - o.y, p.x - o.x), da = Math.abs(((ang - o.angle + 3 * Math.PI) % (2 * Math.PI)) - Math.PI);
        if ((da < Math.PI / 2 || Math.hypot(p.x - o.x, p.y - o.y) < 64) && this.sight(o, p)) { o.target = p; this.setState(o, "walk"); }
      } else if (a === "chase") {
        if (p.dead) { o.target = null; return this.setState(o, "idle"); }
        if (o.reaction) o.reaction--;
        const d = Math.hypot(p.x - o.x, p.y - o.y);
        if (o.d.atk === "bite" || o.d.atk === "imp" || o.d.atk === "baron") {
          if (d < 64 + p.r && this.sight(o, p)) { o.melee = true; return this.setState(o, "atk"); }
        }
        if (o.d.atk !== "bite" && !o.reaction && this.sight(o, p) && rnd() > Math.min(200, d / 8 - (o.d.atk === "pistol" || o.d.atk === "shotgun" ? 0 : 20))) { o.melee = false; o.reaction = 8; return this.setState(o, "atk"); }
        if (--o.movecount < 0 || !this.step(o)) this.newDir(o);
      } else if (a === "face") {
        o.angle = Math.atan2(p.y - o.y, p.x - o.x);
      } else if (a === "shoot") {
        o.angle = Math.atan2(p.y - o.y, p.x - o.x);
        const atk = o.d.atk, d = Math.hypot(p.x - o.x, p.y - o.y);
        if (atk === "pistol" || atk === "shotgun") {
          for (let k = 0; k < (atk === "shotgun" ? 3 : 1); k++) this.bullet(o, o.angle + ((rnd() - rnd()) / 255) * 22 * RAD, o.z + 32, p, ((rnd() % 5) + 1) * 3);
        } else if (o.melee || atk === "bite") {
          if (d < 64 + p.r) this.damage(p, atk === "bite" ? ((rnd() % 10) + 1) * 4 : ((rnd() % 8) + 1) * 3, o);
        } else this.missile(o, MISSILES[atk]);
      } else if (a === "pain" || a === "scream") {
        /* no sounds */
      } else if (a === "fall") { o.solid = false; }
      else if (a === "explode") {                     // a barrel: everything within 128 units is hurt, less the further
        for (const t of [...this.mobjs, p]) {
          if (!t || t === o) continue;
          const d = Math.max(0, Math.hypot(t.x - o.x, t.y - o.y) - (t.r || 16));
          if (d < 128 && (t.player || t.shootable) && this.sight(o, t)) this.damage(t, 128 - d, o);
        }
      } else if (a === "gone") o.gone = true;
    }
    // eight directions, toward the player, the next best when that is blocked (P_NewChaseDir, more or less)
    step(o) {
      if (o.movedir < 0) return false;
      const a = o.movedir * Math.PI / 4, sp = o.d.speed;
      if (!this.tryMove(o, o.x + Math.cos(a) * sp, o.y + Math.sin(a) * sp)) return false;
      o.angle = a;
      if (o.d.fly) { const tz = this.player.z + 16; o.z += Math.sign(tz - o.z) * Math.min(4, Math.abs(tz - o.z)); o.z = Math.max(o.floorz ?? o.z, Math.min(o.z, (o.ceilz ?? o.z + o.h) - o.h)); }
      return true;
    }
    newDir(o) {
      const p = this.player, want = Math.round(Math.atan2(p.y - o.y, p.x - o.x) / (Math.PI / 4));
      const order = [0, 1, -1, 2, -2, 3, -3, 4].map((k) => (((want + k) % 8) + 8) % 8);
      if (rnd() < 64) order.sort(() => Math.random() - 0.5);
      for (const dir of order) { o.movedir = dir; if (this.step(o)) { o.movecount = rnd() & 15; return; } }
      o.movedir = -1;
    }
    missile(o, m) {
      const p = this.player, a = Math.atan2(p.y - o.y, p.x - o.x), d = Math.hypot(p.x - o.x, p.y - o.y) || 1, tics = d / m.speed;
      this.mobjs.push({ missile: true, m, owner: o, x: o.x + Math.cos(a) * (o.r + 8), y: o.y + Math.sin(a) * (o.r + 8), z: o.z + 32, angle: a,
        mx: Math.cos(a) * m.speed, my: Math.sin(a) * m.speed, mz: (p.z + 32 - (o.z + 32)) / tics, r: m.r, h: 8, d: { s: m.s }, sprite: m.s, frame: "A", bright: true, alive: true, dying: 0 });
    }
    fly(o) {
      if (o.dying) { o.frame = o.m.die[Math.min(2, Math.floor((o.dying++) / 6))].toUpperCase(); if (o.dying > 18) o.gone = true; return; }
      o.frame = o.m.fly[Math.floor(this.time / 4) % 2].toUpperCase();
      o.z += o.mz;
      if (!this.tryMove(o, o.x + o.mx, o.y + o.my)) {
        if (o.hit && o.hit !== o.owner) this.damage(o.hit, ((rnd() % 8) + 1) * o.m.dmg / 2.67 | 0, o.owner);
        o.dying = 1;
      }
    }
  }
  // line specials: how they are set off (use, walk), what they do, once or again
  const SPECIALS = {
    1: { how: "use", door: "raise", manual: true, monster: true }, 26: { how: "use", door: "raise", manual: true, key: "blue" }, 27: { how: "use", door: "raise", manual: true, key: "yellow" },
    28: { how: "use", door: "raise", manual: true, key: "red" }, 31: { how: "use", door: "open", manual: true, once: true }, 32: { how: "use", door: "open", manual: true, key: "blue", once: true },
    33: { how: "use", door: "open", manual: true, key: "red", once: true }, 34: { how: "use", door: "open", manual: true, key: "yellow", once: true },
    117: { how: "use", door: "raise", manual: true, fast: true }, 118: { how: "use", door: "open", manual: true, fast: true, once: true },
    2: { how: "walk", door: "open", once: true }, 3: { how: "walk", door: "close", once: true }, 4: { how: "walk", door: "raise", once: true, monster: true },
    86: { how: "walk", door: "open" }, 90: { how: "walk", door: "raise" }, 29: { how: "use", door: "raise", once: true }, 63: { how: "use", door: "raise" },
    103: { how: "use", door: "open", once: true }, 61: { how: "use", door: "open" }, 50: { how: "use", door: "close", once: true }, 42: { how: "use", door: "close" },
    62: { how: "use", plat: true }, 88: { how: "walk", plat: true, monster: true }, 10: { how: "walk", plat: true, once: true }, 21: { how: "use", plat: true, once: true },
    120: { how: "walk", plat: true, fast: true }, 123: { how: "use", plat: true, fast: true },
    23: { how: "use", floor: "lowest", once: true }, 38: { how: "walk", floor: "lowest", once: true }, 82: { how: "walk", floor: "lowest" }, 60: { how: "use", floor: "lowest" },
    102: { how: "use", floor: "highest", once: true }, 19: { how: "walk", floor: "highest", once: true }, 36: { how: "walk", floor: "highest8", once: true },
    11: { how: "use", exit: true }, 51: { how: "use", exit: true }, 52: { how: "walk", exit: true }, 124: { how: "walk", exit: true },
  };
  const WALK = new Set(Object.keys(SPECIALS).filter((k) => SPECIALS[k].how === "walk").map(Number));
  // the first level there is: E1M1 (Doom), else MAP01 (Doom II)
  Level.first = (wad) => (wad.has("E1M1") ? "E1M1" : wad.has("MAP01") ? "MAP01" : null);
  Level.prefixes = () => [...new Set(Object.values(T).map((d) => d.s).concat(["BEXP", "BAL1", "BAL2", "BAL7", "PUFF", "BLUD"]))];
  return Level;
})();
