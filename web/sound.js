// What the room sounds like, while a radio stands in it and is on (room.json settings.sound; the radio is the switch,
// room.js). Nothing recorded: the room's sounds are made here with the Web Audio API, short and quiet, from where they
// happen (left or right by where in the room: pan -1..1); Doom's come out of its WAD (fps.js), in Doom's own format
// (DMX: a rate, then 8-bit samples), placed as Doom places them.
//
// Sound.set(on); Sound.play(name, pan, vol, force, opt) → a handle {stop()} or null (opt goes to the sound, the whistle's
// length); Sound.doom(wad, lump, vol, pan, origin). Only what happens is heard, never a background: no keyboards, no fire.
// A browser lets a page make sound only after a click or a key on it: until then nothing plays (and nothing is kept).
window.Sound = (() => {
  let ac = null, master = null, on = false, noiseBuf = null;
  const live = new Set();
  const woken = () => !navigator.userActivation || navigator.userActivation.hasBeenActive;
  function ctx() {
    if (!on || !woken()) return null;
    if (!ac) {
      const A = window.AudioContext || window.webkitAudioContext;
      if (!A) return null;
      ac = new A(); master = ac.createGain(); master.gain.value = 0.7; master.connect(ac.destination);
    }
    if (ac.state === "suspended") ac.resume().catch(() => {});
    return ac;
  }
  const wake = () => { if (on) ctx(); };
  addEventListener("pointerdown", wake, true); addEventListener("keydown", wake, true);
  function set(v) {
    v = !!v;
    if (v === on) return;
    on = v;
    if (!on) { for (const h of live) h.stop(); live.clear(); }
  }

  // ---- the parts: an output (its volume, where it is), a tone, a breath of noise through a filter
  function out(pan, vol) {
    const g = ac.createGain();
    g.gain.value = vol;
    if (pan && ac.createStereoPanner) { const p = ac.createStereoPanner(); p.pan.value = Math.max(-1, Math.min(1, pan)); g.connect(p); p.connect(master); }
    else g.connect(master);
    return g;
  }
  function tone(dest, t, { f, f2, dur, type = "sine", a = 0.004, vol = 1 }) {
    const o = ac.createOscillator(), g = ac.createGain();
    o.type = type; o.frequency.setValueAtTime(f, t);
    if (f2) o.frequency.exponentialRampToValueAtTime(f2, t + dur);
    g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(vol, t + a); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g); g.connect(dest); o.start(t); o.stop(t + dur + 0.05);
    return o;
  }
  function hiss(dest, t, { dur, type = "bandpass", f = 2000, f2, q = 1, vol = 1, a = 0.002 }) {
    if (!noiseBuf) { noiseBuf = ac.createBuffer(1, ac.sampleRate, ac.sampleRate); const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1; }
    const s = ac.createBufferSource(), fl = ac.createBiquadFilter(), g = ac.createGain();
    s.buffer = noiseBuf; s.loop = true;
    fl.type = type; fl.frequency.setValueAtTime(f, t); fl.Q.value = q;
    if (f2) fl.frequency.exponentialRampToValueAtTime(f2, t + dur);
    g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(vol, t + a); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    s.connect(fl); fl.connect(g); g.connect(dest); s.start(t, Math.random() * 0.9); s.stop(t + dur + 0.05);
    return s;
  }
  const rnd = (a, b) => a + Math.random() * (b - a);

  // ---- the room's sounds: each draws into dest from t, says how loud it is, and how long it lasts
  const SOUNDS = {
    ask: { vol: 0.16, make: (d, t) => { tone(d, t, { f: 659, dur: 0.18, type: "triangle", vol: 0.8 }); tone(d, t + 0.13, { f: 988, dur: 0.3, type: "triangle", vol: 0.8 }); } },   // a hand goes up: "?"
    beam: { vol: 0.1, make: (d, t) => { tone(d, t, { f: 1900, f2: 420, dur: 0.3, vol: 0.9 }); tone(d, t, { f: 950, f2: 260, dur: 0.22, type: "square", vol: 0.1 }); } },
    door: { vol: 0.2, make: (d, t) => { tone(d, t, { f: 120, f2: 70, dur: 0.14, vol: 1 }); hiss(d, t, { dur: 0.05, type: "lowpass", f: 900, vol: 0.6 }); tone(d, t + 0.09, { f: 2200, dur: 0.03, type: "square", vol: 0.05 }); } },
    booth: { vol: 0.09, make: (d, t, h) => {   // the light rising in the booth: a shimmer that climbs, four seconds
      const g = ac.createGain(), lfo = ac.createOscillator(), lg = ac.createGain();
      g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(1, t + 0.6); g.gain.setValueAtTime(1, t + 3.6); g.gain.exponentialRampToValueAtTime(0.0001, t + 4.4);
      lfo.frequency.value = 9; lg.gain.value = 0.35; lfo.connect(lg); lg.connect(g.gain); lfo.start(t); lfo.stop(t + 4.5);
      g.connect(d);
      const parts = [[220, 880, "sine", 0.6], [330, 1320, "triangle", 0.25], [441, 1764, "sine", 0.15]].map(([f, f2, type, vol]) => {
        const o = ac.createOscillator(), og = ac.createGain();
        o.type = type; o.frequency.setValueAtTime(f, t); o.frequency.exponentialRampToValueAtTime(f2, t + 4.2); og.gain.value = vol;
        o.connect(og); og.connect(g); o.start(t); o.stop(t + 4.5);
        return o;
      });
      h.stop = () => { const n = ac.currentTime; g.gain.cancelScheduledValues(n); g.gain.setValueAtTime(g.gain.value, n); g.gain.exponentialRampToValueAtTime(0.0001, n + 0.15); for (const o of [...parts, lfo]) { try { o.stop(n + 0.2); } catch (_) { /* over already */ } } };
      return 4.5;
    } },
    poof: { vol: 0.18, make: (d, t) => { hiss(d, t, { dur: 0.4, type: "highpass", f: 5000, f2: 700, vol: 0.8 }); tone(d, t, { f: 1300, f2: 160, dur: 0.32, vol: 0.4 }); } },
    back: { vol: 0.12, make: (d, t) => { tone(d, t, { f: 1000, f2: 300, dur: 0.5, vol: 0.8 }); } },   // the booth gives one back
    shot: { vol: 0.42, make: (d, t) => { hiss(d, t, { dur: 0.5, type: "lowpass", f: 3200, f2: 260, vol: 1, a: 0.001 }); tone(d, t, { f: 95, f2: 38, dur: 0.32, vol: 1 }); } },
    clank: { vol: 0.13, make: (d, t) => { const k = rnd(0.85, 1.2); for (const [f, dur] of [[640, 0.22], [1590, 0.14], [2870, 0.09]]) tone(d, t, { f: f * k, dur, vol: 0.5 }); } },
    flash: { vol: 0.18, make: (d, t) => { tone(d, t, { f: 2400, f2: 3800, dur: 0.16, vol: 0.6 }); hiss(d, t + 0.04, { dur: 0.3, type: "highpass", f: 6500, vol: 0.4 }); } },
    bark: { vol: 0.22, make: (d, t) => {
      for (const at of [0, 0.19]) {
        const fl = ac.createBiquadFilter(); fl.type = "lowpass"; fl.frequency.value = 1700; fl.connect(d);
        tone(fl, t + at, { f: 540, f2: 360, dur: 0.12, type: "sawtooth", vol: 0.7 }); hiss(d, t + at, { dur: 0.09, f: 950, q: 1.5, vol: 0.5 });
      }
      return 0.4;
    } },
    clack: { vol: 0.2, make: (d, t) => { hiss(d, t, { dur: 0.045, f: 1900, q: 3, vol: 0.9 }); tone(d, t, { f: 720, dur: 0.05, vol: 0.4 }); } },   // a chessman put down
    print: { vol: 0.09, make: (d, t) => {   // a dot-matrix printer: lines of needles, a line feed after each
      for (let line = 0; line < 6; line++) {
        for (let k = 0; k < 16; k++) hiss(d, t + line * 0.34 + k * 0.016, { dur: 0.011, f: rnd(2400, 3200), q: 4, vol: 0.8 });
        tone(d, t + line * 0.34 + 0.27, { f: 85, dur: 0.06, type: "square", vol: 0.25 });
      }
      return 2.1;
    } },
    whoosh: { vol: 0.2, make: (d, t) => { hiss(d, t, { dur: 0.8, type: "lowpass", f: 280, f2: 2600, vol: 0.9, a: 0.15 }); tone(d, t, { f: 70, dur: 0.6, vol: 0.4, a: 0.1 }); } },
    tune: { vol: 0.14, make: (d, t) => { hiss(d, t, { dur: 0.35, f: 500, f2: 3000, q: 8, vol: 0.7 }); tone(d, t + 0.28, { f: 784, dur: 0.2, type: "triangle", vol: 0.6 }); tone(d, t + 0.4, { f: 1047, dur: 0.35, type: "triangle", vol: 0.6 }); } },   // the radio on
    click: { vol: 0.2, make: (d, t) => { hiss(d, t, { dur: 0.02, type: "highpass", f: 3000, vol: 0.7 }); tone(d, t, { f: 190, dur: 0.04, vol: 0.5 }); } },   // the radio off
    paper: { vol: 0.1, make: (d, t) => { hiss(d, t, { dur: 0.25, f: 4500, f2: 2500, q: 0.7, vol: 0.6, a: 0.04 }); } },
    // the duel outside the window (outside.js): a gun fired, the whistle of a shell coming down (o.dur: until it lands),
    // where it lands, a gun hit, a gunner landing on its back, the round won or lost
    cannon: { vol: 0.3, make: (d, t) => { hiss(d, t, { dur: 0.7, type: "lowpass", f: 2400, f2: 140, vol: 1, a: 0.001 }); tone(d, t, { f: 72, f2: 34, dur: 0.5, vol: 1 }); tone(d, t, { f: 190, f2: 70, dur: 0.12, type: "square", vol: 0.25 }); } },
    whistle: { vol: 0.05, make: (d, t, h, o) => {
      const dur = Math.max(0.3, Math.min(4, (o && o.dur) || 1.5)), osc = ac.createOscillator(), g = ac.createGain(), lfo = ac.createOscillator(), lg = ac.createGain();
      osc.type = "sine"; osc.frequency.setValueAtTime(1750, t); osc.frequency.exponentialRampToValueAtTime(620, t + dur);
      lfo.frequency.value = 7; lg.gain.value = 18; lfo.connect(lg); lg.connect(osc.frequency);
      g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(1, t + Math.min(0.25, dur / 3)); g.gain.setValueAtTime(1, t + dur - 0.05); g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      osc.connect(g); g.connect(d); osc.start(t); lfo.start(t); osc.stop(t + dur + 0.05); lfo.stop(t + dur + 0.05);
      h.stop = () => { const n = ac.currentTime; g.gain.cancelScheduledValues(n); g.gain.setValueAtTime(g.gain.value, n); g.gain.exponentialRampToValueAtTime(0.0001, n + 0.04); try { osc.stop(n + 0.06); lfo.stop(n + 0.06); } catch (_) { /* over already */ } };
      return dur;
    } },
    blast: { vol: 0.32, make: (d, t) => { hiss(d, t, { dur: 0.9, type: "lowpass", f: 1500, f2: 90, vol: 1, a: 0.002 }); tone(d, t, { f: 58, f2: 30, dur: 0.65, vol: 1 }); hiss(d, t + 0.05, { dur: 0.35, f: 700, q: 0.8, vol: 0.35 }); } },
    wreck: { vol: 0.38, make: (d, t) => {
      hiss(d, t, { dur: 1.4, type: "lowpass", f: 1800, f2: 70, vol: 1, a: 0.002 }); tone(d, t, { f: 50, f2: 26, dur: 1, vol: 1 });
      for (const [at, f] of [[0.18, 830], [0.32, 1270], [0.5, 640]]) tone(d, t + at, { f: f * rnd(0.9, 1.1), dur: 0.2, vol: 0.18 });   // the gun's iron
      return 1.5;
    } },
    thud: { vol: 0.16, make: (d, t) => { tone(d, t, { f: 120, f2: 52, dur: 0.12, vol: 1 }); hiss(d, t, { dur: 0.07, type: "lowpass", f: 600, vol: 0.6 }); } },
    win: { vol: 0.13, make: (d, t) => { [523, 659, 784, 1047].forEach((f, i) => tone(d, t + i * 0.11, { f, dur: i === 3 ? 0.45 : 0.14, type: "square", vol: 0.35 })); return 0.9; } },
    lose: { vol: 0.12, make: (d, t) => { [392, 349, 262].forEach((f, i) => tone(d, t + i * 0.22, { f, f2: i === 2 ? 200 : undefined, dur: i === 2 ? 0.6 : 0.2, type: "triangle", vol: 0.8 })); return 1.1; } },
  };
  function play(name, pan = 0, vol = 1, force, opt) {
    const s = SOUNDS[name];
    if (!s || !(on || force)) return null;
    const was = on; on = true; const a = ctx(); on = was;
    if (!a) return null;
    const h = { stop() {} }, t = a.currentTime + 0.01, dest = out(pan, s.vol * vol);
    const len = s.make(dest, t, h, opt) || 0.6;
    live.add(h);
    setTimeout(() => live.delete(h), (len + 0.2) * 1000);
    return h;
  }

  // ---- Doom's: a lump in DMX format (format 3, rate, count; the samples, 16 of padding each side), one sound an
  // origin at a time (Doom's channels)
  const dmx = new WeakMap(), byOrigin = new Map();
  function doom(wad, name, vol = 1, pan = 0, origin) {
    const a = ctx();
    if (!a || !wad || vol <= 0.01) return;
    let m = dmx.get(wad);
    if (!m) dmx.set(wad, (m = new Map()));
    let b = m.get(name);
    if (b === undefined) {
      b = null;
      const d = wad.lump(name);
      if (d && d.length > 40 && (d[0] | (d[1] << 8)) === 3) {
        const rate = d[2] | (d[3] << 8), n = Math.min((d[4] | (d[5] << 8) | (d[6] << 16)) >>> 0, d.length - 8) - 32;
        if (n > 0 && rate >= 3000) { b = a.createBuffer(1, n, rate); const ch = b.getChannelData(0); for (let i = 0; i < n; i++) ch[i] = (d[8 + 16 + i] - 128) / 128; }
      }
      m.set(name, b);
    }
    if (!b) return;
    if (origin !== undefined && byOrigin.has(origin)) { try { byOrigin.get(origin).stop(); } catch (_) { /* done already */ } }
    const s = a.createBufferSource();
    s.buffer = b; s.connect(out(pan, 0.55 * vol)); s.start();
    if (origin !== undefined) { byOrigin.set(origin, s); s.onended = () => { if (byOrigin.get(origin) === s) byOrigin.delete(origin); }; }
  }
  return { set, play, doom, on: () => on };
})();
