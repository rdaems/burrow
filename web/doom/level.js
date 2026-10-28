// A Doom level played, simply: the map from a WAD (wad.js), the player, Doom's monsters, items and specials, in
// Doom's units (a unit a texel) and its tics (35 a second). What it does, as Doom does it, more or less:
// - moving: against the lines (one-sided, blocking, an opening lower than you, a step higher than 24) and the solid
//   things, sliding along walls; falling with Doom's gravity, your height the highest floor under you;
// - specials: doors (opened by use, by walking over a line, keyed; monsters open the plain ones), lifts (down, a
//   wait, up), floors that lower, switches (their texture flips), the exit (switch or line: the level is done); sector
//   lights that blink, strobe, glow and flicker; floors that hurt (nukage);
// - things: monsters wake when they see you, or when a shot's sound reaches their sector (through every opening, past
//   one sound-blocking line at most); they chase you in eight directions, attack as often as Doom's A_Chase lets
//   them (zombiemen, shotgun guys: hitscan; imps, cacodemons, barons: fireballs; demons, imps up close: claws and
//   teeth), feel pain and die on Doom's frames; items are picked up as you walk over them (health, armor, ammo, keys,
//   a backpack); barrels explode;
// - the shotgun: seven pellets in a spread, five to fifteen each, puffs on the walls, blood on what is hit.
// What it leaves out: the other weapons (their ammo is counted), vertical aim (autoaim, as Doom's), sound, saving.
//
// level.portal (set by fps.js): the mysterious door out to burrow's office, in a wall of the map. While it is open
// you, missiles, bullets and sight go through it; beyond it is the office, where no line of the map is. While you are
// in the office, level.away: the level goes on (monsters see you through the door, come to it, shoot through it).
//
// new DoomLevel(wad, mapname) → level: level.tic(cmd) a step; level.player, level.mobjs, level.map; level.done
// ("exit") once it is over; level.sounds what was heard this tic ({name, x, y, o}: Doom's sound, where, from what),
// for fps.js to play. The 3d view (fps.js) draws it.
window.DoomLevel = (() => {
  const RAD = Math.PI / 180;
  const rnd = () => Math.random() * 256 | 0;           // P_Random, near enough
  // the things a map can have: sprite, size, what it is. A monster's melee and missile attacks as Doom's mobjinfo has them
  const M = (s, hp, speed, r, pain, atk, extra) => ({ s, mon: true, hp, speed, r, h: 56, pain, atk, solid: true, shootable: true, ...extra });
  const I = (s, frames, extra) => ({ s, frames, pick: true, r: 20, h: 16, ...extra });
  const D = (s, frames, solid, extra) => ({ s, frames, solid, r: 16, h: 16, ...extra });
  const T = {
    3004: M("POSS", 20, 8, 20, 200, "pistol"), 9: M("SPOS", 30, 8, 20, 170, "shotgun"), 3001: M("TROO", 60, 8, 20, 200, "imp", { melee: true }),
    3002: M("SARG", 150, 10, 30, 180, "bite", { melee: true, missile: false }), 58: M("SARG", 150, 10, 30, 180, "bite", { melee: true, missile: false, shadow: true }),
    3006: M("SKUL", 100, 8, 16, 256, "bite", { fly: true, bright: true, melee: true, missile: false }),
    3005: M("HEAD", 400, 8, 31, 128, "caco", { fly: true, melee: true }), 3003: M("BOSS", 1000, 8, 24, 50, "baron", { melee: true }),
    2035: { s: "BAR1", frames: "AB", r: 10, h: 42, solid: true, shootable: true, hp: 20, barrel: true },
    // items (Doom's amounts)
    2011: I("STIM", "A", { health: 10, max: 100 }), 2012: I("MEDI", "A", { health: 25, max: 100 }), 2014: I("BON1", "ABCDCB", { health: 1, max: 200, always: true }),
    2015: I("BON2", "ABCDCB", { armor: 1, max: 200, always: true }), 2018: I("ARM1", "AB", { armor: 100, cls: 1 }), 2019: I("ARM2", "AB", { armor: 200, cls: 2 }),
    2013: I("SOUL", "ABCDCB", { health: 100, max: 200, always: true, bright: true }), 2023: I("PSTR", "A", { health: 100, max: 100, bright: true }),
    2008: I("SHEL", "A", { ammo: "shells", n: 4 }), 2049: I("SBOX", "A", { ammo: "shells", n: 20 }), 2001: I("SHOT", "A", { ammo: "shells", n: 8 }),
    2007: I("CLIP", "A", { ammo: "bullets", n: 10 }), 2048: I("AMMO", "A", { ammo: "bullets", n: 50 }), 2010: I("ROCK", "A", { ammo: "rockets", n: 1 }),
    2046: I("BROK", "A", { ammo: "rockets", n: 5 }), 2047: I("CELL", "A", { ammo: "cells", n: 20 }), 17: I("CELP", "A", { ammo: "cells", n: 100 }), 8: I("BPAK", "A", { backpack: true }),
    5: I("BKEY", "AB", { key: "blue" }), 6: I("YKEY", "AB", { key: "yellow" }), 13: I("RKEY", "AB", { key: "red" }),
    40: I("BSKU", "AB", { key: "blue" }), 39: I("YSKU", "AB", { key: "yellow" }), 38: I("RSKU", "AB", { key: "red" }),
    // what you cannot use here stays where it lies
    2002: D("MGUN", "A", false), 2003: D("LAUN", "A", false), 2004: D("PLAS", "A", false), 2005: D("CSAW", "A", false), 2006: D("BFUG", "A", false),
    2022: D("PINV", "ABCD", false, { bright: true }), 2024: D("PINS", "ABCD", false, { bright: true }), 2025: D("SUIT", "A", false, { bright: true }),
    2026: D("PMAP", "ABCDCB", false, { bright: true }), 2045: D("PVIS", "AB", false, { bright: true }),
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
  const AMMO = { bullets: 200, shells: 50, rockets: 50, cells: 300 }, CLIPS = { bullets: 10, shells: 4, rockets: 1, cells: 20 };
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
  // what each monster sounds like (Doom's mobjinfo): when it sees you, now and then while it hunts, hurt, dying
  const POS = ["DSPOSIT1", "DSPOSIT2", "DSPOSIT3"], PODTH = ["DSPODTH1", "DSPODTH2", "DSPODTH3"];
  const SEE = { POSS: POS, SPOS: POS, TROO: ["DSBGSIT1", "DSBGSIT2"], SARG: ["DSSGTSIT"], HEAD: ["DSCACSIT"], BOSS: ["DSBRSSIT"], SKUL: [] };
  const ACTIVE = { POSS: "DSPOSACT", SPOS: "DSPOSACT", TROO: "DSBGACT", SARG: "DSDMACT", HEAD: "DSDMACT", BOSS: "DSDMACT", SKUL: "DSDMACT" };
  const PAIN = { POSS: "DSPOPAIN", SPOS: "DSPOPAIN", TROO: "DSPOPAIN", SARG: "DSDMPAIN", HEAD: "DSDMPAIN", BOSS: "DSDMPAIN", SKUL: "DSDMPAIN" };
  const DEATH = { POSS: PODTH, SPOS: PODTH, TROO: ["DSBGDTH1", "DSBGDTH2"], SARG: ["DSSGTDTH"], HEAD: ["DSCACDTH"], BOSS: ["DSBRSDTH"], SKUL: ["DSFIRXPL"] };
  const pick = (a) => a && a.length ? a[Math.floor(Math.random() * a.length)] : null;
  const MISSILES = { imp: { s: "BAL1", speed: 10, r: 6, dmg: 3, fly: [F("A"), F("B")], die: [F("C"), F("D"), F("E")] },
    caco: { s: "BAL2", speed: 10, r: 6, dmg: 5, fly: [F("A"), F("B")], die: [F("C"), F("D"), F("E")] },
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
      this.portal = null;
      this.away = false;
      this.sounds = [];
      const m = this.map;
      for (const s of m.sectors) { s.lines = []; s.light0 = s.light; s.floor0 = s.floor; s.ceil0 = s.ceil; s.mover = null; s.phase = rnd() & 7; }
      for (const l of m.lines) {
        l.x0 = Math.min(l.v1.x, l.v2.x); l.x1 = Math.max(l.v1.x, l.v2.x); l.y0 = Math.min(l.v1.y, l.v2.y); l.y1 = Math.max(l.v1.y, l.v2.y);
        l.dx = l.v2.x - l.v1.x; l.dy = l.v2.y - l.v1.y;
        for (const sd of [l.front, l.back]) if (sd && !sd.sector.lines.includes(l)) sd.sector.lines.push(l);
      }
      for (const s of m.sectors) {                   // where a sector's sounds come from: its middle (Doom's soundorg)
        const xs = s.lines.flatMap((l) => [l.v1.x, l.v2.x]), ys = s.lines.flatMap((l) => [l.v1.y, l.v2.y]);
        s.x = xs.length ? (Math.min(...xs) + Math.max(...xs)) / 2 : 0; s.y = ys.length ? (Math.min(...ys) + Math.max(...ys)) / 2 : 0;
      }
      // a grid of the lines, 128 units a cell, for what a box or a ray meets
      const xs = m.vertexes.map((v) => v.x), ys = m.vertexes.map((v) => v.y);
      this.gx = Math.min(...xs) - 8; this.gy = Math.min(...ys) - 8;
      this.gw = Math.ceil((Math.max(...xs) - this.gx + 16) / 128); this.gh = Math.ceil((Math.max(...ys) - this.gy + 16) / 128);
      this.grid = Array.from({ length: this.gw * this.gh }, () => []);
      for (const l of m.lines) for (let cy = this.cell(l.y0, this.gy); cy <= this.cell(l.y1, this.gy); cy++) for (let cx = this.cell(l.x0, this.gx); cx <= this.cell(l.x1, this.gx); cx++) this.grid[cy * this.gw + cx].push(l);
      // things: the player's start, monsters and items for skill 3 (Hurt me plenty), not the multiplayer ones
      this.start = m.things.find((t) => t.type === 1) || { x: 0, y: 0, angle: 0 };
      this.player = this.spawnPlayer(this.start);   // first: the monsters look for you as they come
      for (const t of m.things) {
        if (t.type <= 4 || t.type === 11) continue;
        if (t.flags & 16 || !(t.flags & 2)) continue;
        const d = T[t.type];
        if (d) this.spawn(t.type, d, t.x, t.y, t.angle * RAD, !!(t.flags & 8));
      }
      this.anims = this.animations();
    }
    snd(name, o) { if (name && o) this.sounds.push({ name, x: o.x, y: o.y, o }); }
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
      return { player: true, x: t.x, y: t.y, z: s.floor, floorz: s.floor, mz: 0, angle: t.angle * RAD, mx: 0, my: 0, r: 16, h: 56, health: 100, armor: 0, armorClass: 0,
        ammo: { bullets: 0, shells: 20, rockets: 0, cells: 0 }, max: { ...AMMO }, keys: {}, viewh: 41, dviewh: 0, damagecount: 0, bonuscount: 0, attacker: null, dead: false,
        sector: s, kills: 0 };
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
    place(type, x, y) { const d = T[type]; return d ? this.spawn(type, d, x, y, 0, false) : null; }   // a thing put down (a torch by the mysterious door)
    inside(x, y) {                                    // in the map, not in the void beyond its walls: on the inside of every seg of its subsector
      const ss = this.subsector(x, y), m = this.map;
      for (let i = 0; i < ss.n; i++) { const sg = m.segs[ss.first + i]; if ((sg.v2.x - sg.v1.x) * (y - sg.v1.y) - (sg.v2.y - sg.v1.y) * (x - sg.v1.x) > 0) return false; }
      return true;
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

    // ---- the mysterious door (fps.js sets level.portal): which side of its wall a point is on (> 0 the level's), how
    // far along it from its middle; open: how high it stands open now
    beyond(x, y) { const P = this.portal; return (x - P.cx) * P.nx + (y - P.cy) * P.ny; }
    along(x, y) { const P = this.portal; return (x - P.cx) * P.tx + (y - P.cy) * P.ty; }
    officeSide(l) { return this.beyond(l.v1.x, l.v1.y) <= 0.01 && this.beyond(l.v2.x, l.v2.y) <= 0.01; }   // a line on the door's wall or behind it

    // ---- moving: can a thing of radius r stand at x, y; where its floor and ceiling are there. What stopped it is
    // left in blockLine (for the slide); the special lines it touched in spechit (a monster opens a door that way)
    check(o, x, y) {
      const r = o.r, P = this.portal;
      this.blockLine = null; this.spechit = [];
      // through the open door: under its lintel, between its jambs; the lines on the office's side are not there
      let thru = false;
      if (P && (o.player || o.missile) && P.open() > 0) {
        const b = this.beyond(x, y), fits = Math.abs(this.along(x, y)) + r <= P.w / 2 && (o.missile ? o.z >= P.floor && o.z + 8 <= P.floor + P.open() : P.open() >= o.h);
        thru = fits && b < r * (Math.abs(P.nx) + Math.abs(P.ny));
      }
      let sec = this.sectorAt(x, y);
      if (thru && this.beyond(x, y) < 0) sec = P.sector;
      let floor = sec.floor, ceil = thru ? Math.min(sec.ceil, P.floor + P.open()) : sec.ceil, drop = sec.floor, fl = null, cl = null;
      for (const l of this.linesNear(x - r, y - r, x + r, y + r)) {
        if (x + r <= l.x0 || x - r >= l.x1 || y + r <= l.y0 || y - r >= l.y1) continue;
        if (this.boxSide(l, x, y, r) !== -1) continue;   // the box is wholly on one side: it does not touch the line
        if (thru && this.officeSide(l)) continue;
        if (!l.back) { this.blockLine = l; return null; }                    // a one-sided line: a wall
        if (l.flags & 1 && !o.missile) { this.blockLine = l; return null; }   // blocking
        if (o.d && o.d.mon && l.flags & 2) { this.blockLine = l; return null; }   // blocks monsters
        if (l.special) this.spechit.push(l);
        const f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil);
        if (f > floor) { floor = f; fl = l; }
        if (c < ceil) { ceil = c; cl = l; }
        drop = Math.min(drop, l.front.sector.floor, l.back.sector.floor);
      }
      if (ceil - floor < o.h && !o.missile) { this.blockLine = cl || fl; return null; }
      if (!o.missile && !o.d?.fly && floor - o.z > 24) { this.blockLine = fl; return null; }   // too high a step
      if (o.d && o.d.mon && !o.d.fly && floor - drop > 24) { this.blockLine = fl; return null; }   // a monster does not walk off a ledge
      if (o.missile && (o.z < floor || o.z + 8 > ceil)) return null;
      for (const t of this.mobjs) {                                                  // solid things: as tall as anything (vanilla Doom's)
        if (t === o || !t.solid || t === o.owner) continue;
        if (Math.abs(t.x - x) < t.r + r && Math.abs(t.y - y) < t.r + r) {
          if (o.missile) { o.hit = t; return null; }
          return null;
        }
      }
      if (!o.player && !o.missile && this.player && !this.player.dead && !this.away && Math.abs(this.player.x - x) < this.player.r + r && Math.abs(this.player.y - y) < this.player.r + r) return null;
      if (o.missile && this.hitsPlayer(o, x, y)) { o.hit = this.player; return null; }
      return { floor, ceil, sector: sec };
    }
    hitsPlayer(o, x, y) {
      const p = this.player;
      return p && o.owner !== p && !p.dead && Math.abs(p.x - x) < p.r + o.r && Math.abs(p.y - y) < p.r + o.r && o.z < p.z + p.h && o.z + 8 > p.z;
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
      if (o.player) { if (o.z < at.floor) { o.dviewh = Math.min(0, o.z - at.floor); o.z = at.floor; } }   // up a step at once (the view follows); down: you fall
      else if (!o.missile && !(o.d && o.d.fly)) o.z = at.floor;
      if (o.player) this.crossLines(o, ox, oy);
      return true;
    }
    // Doom's slide: the whole move, else along the wall that stopped it, else along x or y. A long move goes in halves
    slide(o, dx, dy) {
      if (Math.hypot(dx, dy) > 15) { this.slide(o, dx / 2, dy / 2); if (o.mx || o.my) this.slide(o, dx / 2, dy / 2); return; }
      if (this.tryMove(o, o.x + dx, o.y + dy)) return;
      const l = this.blockLine;
      if (o.player && this.portal && l && this.portal.lines.has(l) && Math.abs(this.along(o.x, o.y)) < this.portal.w / 2) this.portal.bump();   // walked into the mysterious door: it opens
      if (l) {
        const len = Math.hypot(l.dx, l.dy) || 1, tx = l.dx / len, ty = l.dy / len, k = dx * tx + dy * ty;
        if (Math.abs(k) > 0.01 && this.tryMove(o, o.x + tx * k, o.y + ty * k)) { o.mx = tx * k; o.my = ty * k; return; }
      }
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
    // height z), within dist; for the shotgun and sight. Through the open door the lines on the office's side are not there
    trace(x, y, ang, dist, zAt) {
      const dx = Math.cos(ang), dy = Math.sin(ang), x2 = x + dx * dist, y2 = y + dy * dist, P = this.portal;
      let thru = false;
      if (P && P.open() > 0) {
        const b0 = this.beyond(x, y), b1 = this.beyond(x2, y2);
        if ((b0 > 0) !== (b1 > 0)) {
          const tc = b0 / (b0 - b1) * dist, z = zAt(tc);
          thru = Math.abs(this.along(x + dx * tc, y + dy * tc)) < P.w / 2 && z > P.floor && z < P.floor + P.open();
        }
      }
      let best = dist, line = null;
      for (const l of this.linesNear(Math.min(x, x2), Math.min(y, y2), Math.max(x, x2), Math.max(y, y2))) {
        if (thru && this.officeSide(l)) continue;
        const den = dx * l.dy - dy * l.dx;
        if (Math.abs(den) < 1e-9) continue;
        const t = ((l.v1.x - x) * l.dy - (l.v1.y - y) * l.dx) / den, u = ((l.v1.x - x) * dy - (l.v1.y - y) * dx) / den;
        if (t <= 0.01 || t >= best || u < 0 || u > 1) continue;
        if (l.back) {
          const z = zAt(t), f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil);
          if (z > f && z < c) continue;               // through the opening
        }
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
      if (def.key && by.player && !by.keys[def.key]) { this.snd("DSOOF", by); return false; }   // a locked door
      if (by.d && by.d.mon && !def.monster) return false;
      let did = false;
      if (def.door) {
        const secs = def.manual ? (l.back ? [l.back.sector] : []) : this.tagged(l.tag);
        for (const s of secs) did = this.door(s, def.door, def.fast) || did;
      } else if (def.plat) for (const s of this.tagged(l.tag)) did = this.plat(s, def.fast) || did;
      else if (def.floor) for (const s of this.tagged(l.tag)) did = this.floorMove(s, def.floor) || did;
      else if (def.exit) { this.done = "exit"; did = true; }
      if (def.once && (did || def.exit)) l.special = 0;
      if (how === "use" && did) { this.switchTex(l, !def.once); if (def.exit) this.snd("DSSWTCHX", by); }
      return did;
    }
    tagged(tag) { return tag ? this.map.sectors.filter((s) => s.tag === tag) : []; }
    neighbours(s) { return s.lines.map((l) => (l.front.sector === s ? l.back : l.front)).filter(Boolean).map((sd) => sd.sector); }
    door(s, kind, fast) {
      if (s.mover) {                                   // a door going down: back up (DR doors used again)
        if (s.mover.kind === "door" && s.mover.dir === -1 && kind === "raise") { s.mover.dir = 1; this.snd(s.mover.speed > 2 ? "DSBDOPN" : "DSDOROPN", s); return true; }
        return false;
      }
      const ns = this.neighbours(s), top = (ns.length ? Math.min(...ns.map((n) => n.ceil)) : s.ceil0) - 4;
      if (kind === "close") s.mover = { kind: "door", dir: -1, top: s.ceil, speed: fast ? 8 : 2, wait: 0, mode: "close" };
      else s.mover = { kind: "door", dir: 1, top, speed: fast ? 8 : 2, wait: 0, mode: kind };
      this.snd(kind === "close" ? (fast ? "DSBDCLS" : "DSDORCLS") : fast ? "DSBDOPN" : "DSDOROPN", s);
      return true;
    }
    plat(s, fast) {
      if (s.mover) return false;
      const ns = this.neighbours(s), low = Math.min(s.floor, ...ns.map((n) => n.floor));
      s.mover = { kind: "plat", dir: -1, low, high: s.floor, speed: fast ? 8 : 4, wait: 0 };
      this.snd("DSPSTART", s);
      return true;
    }
    floorMove(s, kind) {
      if (s.mover) return false;
      const ns = this.neighbours(s);
      if (!ns.length) return false;
      const to = kind === "lowest" ? Math.min(s.floor, ...ns.map((n) => n.floor)) : kind === "highest" || kind === "highest8" ? Math.max(...ns.map((n) => n.floor)) + (kind === "highest8" ? 8 : 0) : s.floor;
      s.mover = { kind: "floor", to, speed: 1 };
      return true;
    }
    switchTex(l, back) {
      const sd = l.front;
      this.snd("DSSWTCHN", { x: (l.v1.x + l.v2.x) / 2, y: (l.v1.y + l.v2.y) / 2 });
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
          if (m.wait > 0) { if (--m.wait === 0) { m.dir = -1; this.snd(m.speed > 2 ? "DSBDCLS" : "DSDORCLS", s); } continue; }
          if (m.dir === 1) {
            s.ceil = Math.min(m.top, s.ceil + m.speed);
            if (s.ceil >= m.top) { if (m.mode === "raise") { m.wait = 150; m.dir = 0; } else s.mover = null; }
          } else if (m.dir === -1) {
            const nc = Math.max(s.floor, s.ceil - m.speed);
            if (this.crushes(s, nc)) { if (m.mode !== "close") { m.dir = 1; this.snd(m.speed > 2 ? "DSBDOPN" : "DSDOROPN", s); } continue; }
            s.ceil = nc;
            if (s.ceil <= s.floor) s.mover = null;
          }
        } else if (m.kind === "plat") {
          if (m.wait > 0) { if (--m.wait === 0) { m.dir = m.dir === -1 ? 1 : -1; this.snd("DSPSTART", s); } continue; }
          if (m.dir === -1) { s.floor = Math.max(m.low, s.floor - m.speed); if (s.floor <= m.low) { m.wait = 105; m.dir = -1; m.next = 1; this.snd("DSPSTOP", s); } }
          else { const nf = Math.min(m.high, s.floor + m.speed); if (this.crushes(s, s.ceil, nf)) { m.dir = -1; continue; } s.floor = nf; if (s.floor >= m.high) { s.mover = null; this.snd("DSPSTOP", s); } }
          if (m.next && m.wait === 0) { m.dir = m.next; m.next = 0; }
        } else if (m.kind === "floor") {
          s.floor += Math.sign(m.to - s.floor) * Math.min(m.speed, Math.abs(m.to - s.floor));
          if (s.floor === m.to) { s.mover = null; this.snd("DSPSTOP", s); } else if (this.time % 8 === 0) this.snd("DSSTNMOV", s);
        }
        for (const o of [this.player, ...this.mobjs]) {   // what stands on a moving floor rides it (you: the highest floor under you)
          if (!o || o.missile || (o.d && (o.d.fly || o.d.hang)) || (o.player && this.away) || this.sectorAt(o.x, o.y) !== s) continue;
          const at = o.player ? this.check(o, o.x, o.y) : null;
          o.z = at ? at.floor : s.floor;
          if (o.player) o.floorz = o.z;
        }
      }
    }
    crushes(s, ceil, floor = s.floor) {               // would something no longer fit
      for (const o of [this.player, ...this.mobjs]) if (o && o.alive !== false && !o.missile && (o.player || o.solid) && !(o.player && this.away) && this.sectorAt(o.x, o.y) === s && ceil - floor < o.h) return true;
      return false;
    }
    lights() {                                       // Doom's: a flicker, strobes (fast: 15 tics dark, 5 bright; slow: 35 dark), a glow
      const t = this.time;
      for (const s of this.map.sectors) {
        const sp = s.special;
        if (!sp || sp === 9 || sp === 7 || sp === 5 || sp === 16) continue;
        const ns = this.neighbours(s), low = Math.min(s.light0, ...ns.map((n) => n.light0));
        const lo = low === s.light0 ? 0 : low, hi = s.light0, ph = sp === 12 || sp === 13 ? 0 : s.phase;
        if (sp === 1) { if (!s.blink || --s.blink <= 0) { s.light = s.light === hi ? lo : hi; s.blink = s.light === hi ? (rnd() & 64) + 1 : (rnd() & 7) + 1; } }
        else if (sp === 2 || sp === 4 || sp === 13) s.light = (t + ph) % 20 < 5 ? hi : lo;
        else if (sp === 3 || sp === 12) s.light = (t + ph) % 40 < 5 ? hi : lo;
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

    // ---- the player's turn: cmd = {forward, side, turn (radians), use, run}. Away (in the office, fps.js moves you
    // there) the level goes on without you moving in it
    tic(cmd) {
      if (this.done) return;
      this.time++;
      const p = this.player;
      if (!p.dead && !this.away) {
        p.angle += cmd.turn || 0;
        const onGround = p.z <= (p.floorz ?? p.z) + 0.01;
        const fw = (cmd.forward || 0) * (cmd.run ? 50 : 25) / 32, sd = (cmd.side || 0) * (cmd.run ? 40 : 24) / 32;   // Doom's thrust, in units a tic
        const ca = Math.cos(p.angle), sa = Math.sin(p.angle);
        if (onGround) { p.mx += ca * fw + sa * sd; p.my += sa * fw - ca * sd; }
        const sp = Math.hypot(p.mx, p.my);
        if (sp > 30) { p.mx *= 30 / sp; p.my *= 30 / sp; }
        if (p.mx || p.my) this.slide(p, p.mx, p.my);
        if (p.z > p.floorz) { p.mz = p.mz ? p.mz - 1 : -2; p.z += p.mz; if (p.z <= p.floorz) { if (p.mz < -8) { p.dviewh = p.mz / 8; this.snd("DSOOF", p); } p.z = p.floorz; p.mz = 0; } }   // Doom's gravity; a hard landing squats the view
        if (onGround) { p.mx *= 0.90625; p.my *= 0.90625; }   // friction, 0xE800 (none in the air)
        if (Math.abs(p.mx) < 0.0625 && Math.abs(p.my) < 0.0625 && !cmd.forward && !cmd.side) p.mx = p.my = 0;
        p.bob = Math.min(16, (p.mx * p.mx + p.my * p.my) / 4);
        if (p.dviewh) { p.dviewh *= 0.75; if (Math.abs(p.dviewh) < 0.5) p.dviewh = 0; }
        if (cmd.use) this.use(p);
        const s = this.sectorAt(p.x, p.y);
        p.sector = s;
        if (s.special === 9) { s.special = 0; p.secret = (p.secret || 0) + 1; }
        if ((s.special === 7 || s.special === 5 || s.special === 16 || s.special === 4) && p.z <= s.floor && this.time % 32 === 0) this.damage(p, s.special === 7 ? 5 : s.special === 5 ? 10 : 20, null);
        this.pickup(p);
      } else if (p.dead) {
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
    use(p) {                                          // the line in front of you, within 64 units, used from its front
      const ang = p.angle, x2 = p.x + Math.cos(ang) * 64, y2 = p.y + Math.sin(ang) * 64, P = this.portal;
      let best = 1e9, hit = null;
      for (const l of this.linesNear(Math.min(p.x, x2), Math.min(p.y, y2), Math.max(p.x, x2), Math.max(p.y, y2))) {
        const dx = Math.cos(ang), dy = Math.sin(ang), den = dx * l.dy - dy * l.dx;
        if (Math.abs(den) < 1e-9) continue;
        const t = ((l.v1.x - p.x) * l.dy - (l.v1.y - p.y) * l.dx) / den, u = ((l.v1.x - p.x) * dy - (l.v1.y - p.y) * dx) / den;
        if (t < 0 || t > 64 || u < 0 || u > 1 || t >= best) continue;
        if (P && P.lines.has(l) && Math.abs(this.along(p.x + dx * t, p.y + dy * t)) < P.w / 2) { best = t; hit = "door"; continue; }
        if (l.special || !l.back) { best = t; hit = l; }
        else { const f = Math.max(l.front.sector.floor, l.back.sector.floor), c = Math.min(l.front.sector.ceil, l.back.sector.ceil); if (c - f <= 0) { best = t; hit = l; } }
      }
      if (hit === "door") return P.bump && P.bump();
      if (hit && hit.special && (l2side(hit, p.x, p.y) === 0 || hit.special === 124)) return this.trigger(hit, p, "use");
      if (hit) this.snd("DSNOWAY", p);               // a wall: the grunt
      return false;
    }
    pickup(p) {
      for (const o of this.mobjs) {
        const d = o.d;
        if (!d.pick) continue;
        if (Math.abs(o.x - p.x) > p.r + o.r || Math.abs(o.y - p.y) > p.r + o.r || o.z - p.z > p.h || o.z - p.z < -8) continue;
        let took = false;
        if (d.health && (d.always || p.health < d.max)) { p.health = Math.min(d.max, p.health + d.health); took = true; }
        if (d.armor && (d.always ? p.armor < d.max : p.armor < d.armor)) { p.armor = d.always ? Math.min(d.max, p.armor + d.armor) : d.armor; p.armorClass = Math.max(p.armorClass || 1, d.cls || 1); took = true; }
        if (d.backpack) {                              // twice as much of everything, and a clip of each
          if (p.max.bullets === AMMO.bullets) for (const k in AMMO) p.max[k] = AMMO[k] * 2;
          for (const k in CLIPS) p.ammo[k] = Math.min(p.max[k], p.ammo[k] + CLIPS[k]);
          took = true;
        }
        if (d.ammo && p.ammo[d.ammo] < p.max[d.ammo]) { p.ammo[d.ammo] = Math.min(p.max[d.ammo], p.ammo[d.ammo] + d.n); took = true; }
        if (d.key && !p.keys[d.key]) { p.keys[d.key] = true; took = true; }
        if (took) { o.gone = true; p.bonuscount = Math.min(p.bonuscount + 6, 40); this.snd(d.s === "SHOT" ? "DSWPNUP" : d.s === "SOUL" || d.s === "PSTR" ? "DSGETPOW" : "DSITEMUP", p); }
      }
    }

    // ---- shooting: the shotgun's seven pellets from the player, Doom's spread
    fire(p) {
      if (p.dead || p.ammo.shells <= 0) return false;
      p.ammo.shells--;
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
      if (hit) { this.damage(hit, dmg, src); if ((!hit.d || !hit.d.barrel) && !(hit.player && this.away)) this.puffs.push({ x, y, z, t: 0, blood: true }); }
      else if (wall.line && !(this.portal && this.beyond(x, y) < 0)) this.puffs.push({ x, y, z, t: 0, blood: false });
    }
    // a shot's sound (P_NoiseAlert): from the shooter's sector through every opening, past one sound-blocking line at
    // most; a monster there wakes (one lying in ambush only if it sees you)
    noise(p) {
      const start = p.sector || this.sectorAt(p.x, p.y), heard = new Map([[start, 0]]), q = [start];   // (in the office: the room in front of the door)
      while (q.length) {
        const s = q.shift(), blocks = heard.get(s);
        for (const l of s.lines) {
          if (!l.back) continue;
          const o = l.front.sector === s ? l.back.sector : l.front.sector;
          if (Math.min(l.front.sector.ceil, l.back.sector.ceil) - Math.max(l.front.sector.floor, l.back.sector.floor) <= 0) continue;   // a closed door
          const b = blocks + (l.flags & 64 ? 1 : 0);
          if (b > 1 || (heard.has(o) && heard.get(o) <= b)) continue;
          heard.set(o, b); q.push(o);
        }
      }
      for (const o of this.mobjs) {
        if (!o.d.mon || !o.alive || o.state !== "idle" || !heard.has(o.sector)) continue;
        if (o.ambush && !this.sight(o, this.player)) continue;
        o.target = this.player; this.snd(pick(SEE[o.d.s]), o); this.setState(o, "walk");
      }
    }
    damage(o, dmg, src) {
      if (o.player) {
        if (o.dead) return;
        if (o.armor) { const save = Math.min(o.armor, Math.floor(dmg / (o.armorClass === 2 ? 2 : 3))); o.armor -= save; dmg -= save; }
        o.health -= dmg; o.damagecount = Math.min(100, o.damagecount + dmg); o.attacker = src; o.hurt = this.time;
        if (o.health <= 0) { o.health = 0; o.dead = true; o.deadAt = this.time; this.snd("DSPLDETH", o); } else this.snd("DSPLPAIN", o);
        return;
      }
      if (!o.alive) return;
      o.hp -= dmg;
      if (o.d.mon) { o.reaction = 0; if (src) o.target = this.player; }   // fight back at once
      if (o.hp <= 0) {
        o.alive = false; o.shootable = false;
        if (o.d.mon) { this.kills++; this.player.kills++; this.setState(o, "die"); }
        else if (o.d.barrel) this.setState(o, "die");
        return;
      }
      if (o.d.mon && rnd() < o.d.pain) { o.justHit = true; this.setState(o, "pain"); }
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
        if ((da < Math.PI / 2 || Math.hypot(p.x - o.x, p.y - o.y) < 64) && this.sight(o, p)) { o.target = p; this.snd(pick(SEE[o.d.s]), o); this.setState(o, "walk"); }
      } else if (a === "chase") {                     // Doom's A_Chase
        if (o.reaction) o.reaction--;
        if (rnd() < 3) this.snd(ACTIVE[o.d.s], o);
        if (o.movedir >= 0) {                         // turn towards where it walks, 45 degrees a tic
          const want = o.movedir * Math.PI / 4, d = ((want - o.angle + 3 * Math.PI) % (2 * Math.PI)) - Math.PI;
          o.angle = Math.abs(d) <= Math.PI / 4 ? want : o.angle + Math.sign(d) * Math.PI / 4;
        }
        if (p.dead) { o.target = null; return this.setState(o, "idle"); }
        if (o.justAttacked) { o.justAttacked = false; return this.newDir(o); }
        const d = Math.hypot(p.x - o.x, p.y - o.y);
        if (o.d.melee && d < 44 + p.r && this.sight(o, p)) { o.melee = true; return this.setState(o, "atk"); }
        if (o.d.missile !== false && !o.movecount && this.missileRange(o, d)) { o.melee = false; o.justAttacked = true; return this.setState(o, "atk"); }
        if (--o.movecount < 0 || !this.step(o)) this.newDir(o);
      } else if (a === "face") {
        o.angle = Math.atan2(p.y - o.y, p.x - o.x);
      } else if (a === "shoot") {
        o.angle = Math.atan2(p.y - o.y, p.x - o.x);
        const atk = o.d.atk, d = Math.hypot(p.x - o.x, p.y - o.y);
        if (o.melee || atk === "bite") {
          this.snd(o.d.s === "SKUL" ? "DSSKLATK" : atk === "bite" ? "DSSGTATK" : "DSCLAW", o);
          if (d < 44 + p.r + 20 && this.sight(o, p)) this.damage(p, atk === "bite" ? ((rnd() % 10) + 1) * 4 : atk === "baron" ? ((rnd() % 8) + 1) * 10 : atk === "caco" ? ((rnd() % 6) + 1) * 10 : ((rnd() % 8) + 1) * 3, o);
        } else if (atk === "pistol" || atk === "shotgun") {
          this.snd(atk === "shotgun" ? "DSSHOTGN" : "DSPISTOL", o);
          for (let k = 0; k < (atk === "shotgun" ? 3 : 1); k++) this.bullet(o, o.angle + ((rnd() - rnd()) / 255) * 22 * RAD, o.z + 32, p, ((rnd() % 5) + 1) * 3);
        } else { this.snd("DSFIRSHT", o); this.missile(o, MISSILES[atk]); }
      } else if (a === "pain") {
        this.snd(PAIN[o.d.s], o);
      } else if (a === "scream") {
        this.snd(pick(DEATH[o.d.s]), o);
      } else if (a === "fall") { o.solid = false; }
      else if (a === "explode") {                     // a barrel: everything within 128 units is hurt, less the further
        this.snd("DSBAREXP", o);
        for (const t of [...this.mobjs, p]) {
          if (!t || t === o || (t.player && this.away)) continue;
          const d = Math.max(0, Math.hypot(t.x - o.x, t.y - o.y) - (t.r || 16));
          if (d < 128 && (t.player || t.shootable) && this.sight(o, t)) this.damage(t, 128 - d, o);
        }
      } else if (a === "gone") o.gone = true;
    }
    missileRange(o, d) {                              // P_CheckMissileRange: in sight, then the further, the less likely
      const p = this.player;
      if (!this.sight(o, p)) return false;
      if (o.justHit) { o.justHit = false; return true; }
      if (o.reaction) return false;
      let k = d - 64;
      if (!o.d.melee) k -= 128;                       // nothing to do up close: shoots more
      if (o.d.s === "SKUL") k /= 2;
      return rnd() >= Math.min(200, k);
    }
    // eight directions, toward the player, the next best when that is blocked (P_NewChaseDir, more or less). A door
    // in the way that a monster may open, it opens (P_Move's spechit)
    step(o) {
      if (o.movedir < 0) return false;
      const a = o.movedir * Math.PI / 4, sp = o.d.speed;
      if (!this.tryMove(o, o.x + Math.cos(a) * sp, o.y + Math.sin(a) * sp)) {
        let opened = false;
        for (const l of this.spechit || []) if (this.trigger(l, o, "use")) opened = true;
        if (opened) o.movedir = -1;
        return opened;
      }
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
        mx: Math.cos(a) * m.speed, my: Math.sin(a) * m.speed, mz: (p.z + 32 - (o.z + 32)) / tics, r: m.r, h: 8, d: { s: m.s }, sprite: m.s, frame: "A", bright: true, alive: true, dying: 0, life: 0 });
    }
    fly(o) {
      if (o.dying) { o.frame = o.m.die[Math.min(2, Math.floor((o.dying++) / 6))].toUpperCase(); if (o.dying > 18) o.gone = true; return; }
      o.frame = o.m.fly[Math.floor(this.time / 4) % 2].toUpperCase();
      o.z += o.mz;
      const P = this.portal;
      if (o.office) {                                 // through the door: in the office, until it hits you or a wall there
        o.x += o.mx; o.y += o.my;
        if (this.hitsPlayer(o, o.x, o.y)) { this.damage(this.player, ((rnd() % 8) + 1) * o.m.dmg, o.owner); o.dying = 1; this.snd("DSFIRXPL", o); }
        else if (!P || !P.inside(o.x, o.y) || o.z < P.floor || o.z + 8 > P.floor + P.height || ++o.life > 200) o.gone = true;
        return;
      }
      if (!this.tryMove(o, o.x + o.mx, o.y + o.my)) {
        if (o.hit && o.hit !== o.owner) this.damage(o.hit, ((rnd() % 8) + 1) * o.m.dmg, o.owner);
        o.dying = 1; this.snd("DSFIRXPL", o);
      } else if (P && this.beyond(o.x, o.y) < 0) o.office = true;
    }
  }
  const l2side = (l, x, y) => (l.dx * (y - l.v1.y) - l.dy * (x - l.v1.x) > 0 ? 1 : 0);   // 0 in front of the line (its right), as P_PointOnLineSide
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
  // where the mysterious door goes in a level: in the wall behind where you start (a straight stretch 64 wide, as
  // high as a door, its floor level), else in the longest such wall of the room you start in. {cx, cy} its middle on
  // the wall, {nx, ny} into the level, the lines it is in, the sector in front of it
  Level.prototype.doorSpot = function (w = 64, h = 72) {
    const m = this.map, st = this.start, S = this.sectorAt(st.x, st.y), a = st.angle * RAD + Math.PI;
    const fits = (l, cx, cy) => {
      const fs = l.front.sector, len = Math.hypot(l.dx, l.dy), tx = l.dx / len, ty = l.dy / len;
      const t = (cx - l.v1.x) * tx + (cy - l.v1.y) * ty;
      if (fs.ceil - fs.floor < h || (l.back && l.back.sector.floor - fs.floor < h && l.back.sector.ceil > fs.floor)) return null;   // too low, or open below the door's height
      const c = Math.max(w / 2, Math.min(len - w / 2, t));
      if (len < w) return null;
      return { cx: l.v1.x + tx * c, cy: l.v1.y + ty * c, nx: ty, ny: -tx, lines: new Set([l]), sector: fs, floor: fs.floor };
    };
    const tr = this.trace(st.x, st.y, a, 1024, () => S.floor + 40);
    if (tr.line && l2side(tr.line, st.x, st.y) === 0) { const f = fits(tr.line, tr.x, tr.y); if (f) return f; }
    let best = null, bl = 0;
    for (const l of S.lines) {
      if (l.front.sector !== S) continue;
      const len = Math.hypot(l.dx, l.dy), f = len > bl && fits(l, (l.v1.x + l.v2.x) / 2, (l.v1.y + l.v2.y) / 2);
      if (f) { best = f; bl = len; }
    }
    return best;
  };
  return Level;
})();
