//! The room's sounds made into samples, as sound.js makes them with the Web Audio API: oscillators, a breath of
//! noise through a biquad filter, gains that rise and fall on exponential ramps (Web Audio's own arithmetic,
//! `exponentialRampToValueAtTime` from the event before). A pure function of the name, the rate and a seed.

use std::f64::consts::{PI, TAU};

/// `Math.random()` with a seed (splitmix64): the noise, and the little differences between two plays
pub(crate) struct Rng(u64);
impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub(crate) fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + self.next() * (b - a)
    }
}

/// every sound there is, and how loud it is (each one's `vol` in sound.js)
pub const SOUNDS: [(&str, f64); 23] = [
    ("ask", 0.16),
    ("beam", 0.1),
    ("door", 0.2),
    ("booth", 0.09),
    ("poof", 0.18),
    ("back", 0.12),
    ("shot", 0.42),
    ("clank", 0.13),
    ("flash", 0.18),
    ("bark", 0.22),
    ("clack", 0.2),
    ("print", 0.09),
    ("whoosh", 0.2),
    ("tune", 0.14),
    ("click", 0.2),
    ("paper", 0.1),
    ("cannon", 0.3),
    ("whistle", 0.05),
    ("blast", 0.32),
    ("wreck", 0.38),
    ("thud", 0.16),
    ("win", 0.13),
    ("lose", 0.12),
];

pub fn volume(name: &str) -> Option<f64> {
    SOUNDS.iter().find(|s| s.0 == name).map(|s| s.1)
}

/// how it fades when stopped (`h.stop`), in seconds; none: it plays to its end (sound.js's `stop() {}`)
pub fn stop_fade(name: &str) -> Option<f64> {
    match name {
        "booth" => Some(0.15),
        "whistle" => Some(0.04),
        _ => None,
    }
}

/// an AudioParam's schedule: setValueAtTime and exponentialRampToValueAtTime, in that order of time
#[derive(Clone)]
struct Param {
    init: f64,
    ev: Vec<(f64, f64, bool)>, // time, value, a ramp to it
}
impl Param {
    fn new(init: f64) -> Param {
        Param {
            init,
            ev: Vec::new(),
        }
    }
    fn set(mut self, v: f64, t: f64) -> Param {
        self.ev.push((t, v, false));
        self
    }
    fn exp(mut self, v: f64, t: f64) -> Param {
        self.ev.push((t, v, true));
        self
    }
    /// the value at t: a ramp runs from the event before it (its time and value) to its own
    fn at(&self, t: f64) -> f64 {
        let mut prev = (0., self.init);
        for &(et, ev, ramp) in &self.ev {
            if t < et {
                if ramp && et > prev.0 {
                    let k = ((t - prev.0) / (et - prev.0)).max(0.);
                    return prev.1 * (ev / prev.1).powf(k);
                }
                return prev.1;
            }
            prev = (et, ev);
        }
        prev.1
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Wave {
    Sine,
    Square,
    Saw,
    Triangle,
}
#[derive(Clone, Copy, PartialEq)]
enum Filter {
    Lowpass,
    Highpass,
    Bandpass,
}

/// polyBLEP: the corner of a square or a saw rounded off over a sample, as a band-limited oscillator has it
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let t = t / dt;
        2. * t - t * t - 1.
    } else if t > 1. - dt {
        let t = (t - 1.) / dt;
        t * t + 2. * t + 1.
    } else {
        0.
    }
}
/// Web Audio scales its band-limited square and saw so that the Gibbs overshoot at the jump (9% of the jump of
/// 2) peaks at 1: their flat parts are at 1 / 1.179
const GIBBS: f64 = 1. / 1.179;
fn wave(w: Wave, ph: f64, dt: f64) -> f64 {
    match w {
        Wave::Sine => (TAU * ph).sin(),
        Wave::Square => {
            GIBBS * ((if ph < 0.5 { 1. } else { -1. }) + blep(ph, dt) - blep((ph + 0.5) % 1., dt))
        }
        Wave::Saw => {
            let p = (ph + 0.5) % 1.; // from 0 up, as Web Audio's
            GIBBS * (2. * p - 1. - blep(p, dt))
        }
        Wave::Triangle => {
            let p = (ph + 0.25) % 1.;
            4. * (p - 0.5).abs() - 1.
        }
    }
}

/// a biquad as Web Audio's BiquadFilterNode computes it (the Audio EQ Cookbook; lowpass and highpass take
/// their Q in dB, bandpass as is)
struct Biquad {
    kind: Filter,
    q: f64,
    x: [f64; 2],
    y: [f64; 2],
}
impl Biquad {
    fn new(kind: Filter, q: f64) -> Biquad {
        Biquad {
            kind,
            q,
            x: [0.; 2],
            y: [0.; 2],
        }
    }
    fn run(&mut self, input: f64, f: f64, rate: f64) -> f64 {
        let f = f.clamp(1., rate / 2. - 1.);
        let w0 = TAU * f / rate;
        let (s, c) = w0.sin_cos();
        let (b, a) = match self.kind {
            Filter::Lowpass | Filter::Highpass => {
                let alpha = s / (2. * 10f64.powf(self.q / 20.));
                let k = if self.kind == Filter::Lowpass {
                    1. - c
                } else {
                    1. + c
                };
                let b1 = if self.kind == Filter::Lowpass { k } else { -k };
                ([k / 2., b1, k / 2.], [1. + alpha, -2. * c, 1. - alpha])
            }
            Filter::Bandpass => {
                let alpha = s / (2. * self.q);
                ([alpha, 0., -alpha], [1. + alpha, -2. * c, 1. - alpha])
            }
        };
        let out = (b[0] * input + b[1] * self.x[0] + b[2] * self.x[1]
            - a[1] * self.y[0]
            - a[2] * self.y[1])
            / a[0];
        self.x = [input, self.x[0]];
        self.y = [out, self.y[0]];
        out
    }
}

/// where a sound is made: a mono buffer that grows as its parts need, and the generator
struct Dest<'a> {
    rate: f64,
    out: &'a mut Vec<f32>,
    rng: &'a mut Rng,
}
impl Dest<'_> {
    fn span(&mut self, t0: f64, t1: f64) -> (usize, usize) {
        let (a, b) = (
            (t0 * self.rate).round() as usize,
            (t1 * self.rate).round() as usize,
        );
        if self.out.len() < b {
            self.out.resize(b, 0.);
        }
        (a, b)
    }
}

struct Tone {
    f: f64,
    f2: Option<f64>,
    dur: f64,
    wave: Wave,
    a: f64,
    vol: f64,
}
const TONE: Tone = Tone {
    f: 440.,
    f2: None,
    dur: 0.1,
    wave: Wave::Sine,
    a: 0.004,
    vol: 1.,
};

/// a tone from t: its pitch from f to f2, up in `a` seconds to vol and down by dur; through `lp` if given
/// (the bark's own lowpass)
fn tone(d: &mut Dest, t: f64, o: Tone, lp: Option<f64>) {
    let mut fq = Param::new(o.f).set(o.f, t);
    if let Some(f2) = o.f2 {
        fq = fq.exp(f2, t + o.dur);
    }
    let g = Param::new(1.)
        .set(0.0001, t)
        .exp(o.vol, t + o.a)
        .exp(0.0001, t + o.dur);
    let (a, b) = d.span(t, t + o.dur + 0.05);
    let mut ph = 0.;
    let mut filt = lp.map(|_| Biquad::new(Filter::Lowpass, 1.));
    for i in a..b {
        let tt = i as f64 / d.rate;
        let f = fq.at(tt);
        let dt = f / d.rate;
        let mut v = wave(o.wave, ph, dt);
        if let (Some(fl), Some(fc)) = (filt.as_mut(), lp) {
            v = fl.run(v, fc, d.rate);
        }
        d.out[i] += (v * g.at(tt)) as f32;
        ph = (ph + dt) % 1.;
    }
}

struct Hiss {
    dur: f64,
    kind: Filter,
    f: f64,
    f2: Option<f64>,
    q: f64,
    vol: f64,
    a: f64,
}
const HISS: Hiss = Hiss {
    dur: 0.1,
    kind: Filter::Bandpass,
    f: 2000.,
    f2: None,
    q: 1.,
    vol: 1.,
    a: 0.002,
};

/// a breath of white noise through a filter, its cutoff from f to f2
fn hiss(d: &mut Dest, t: f64, o: Hiss) {
    let mut fq = Param::new(o.f).set(o.f, t);
    if let Some(f2) = o.f2 {
        fq = fq.exp(f2, t + o.dur);
    }
    let g = Param::new(1.)
        .set(0.0001, t)
        .exp(o.vol, t + o.a)
        .exp(0.0001, t + o.dur);
    let (a, b) = d.span(t, t + o.dur + 0.05);
    let mut fl = Biquad::new(o.kind, o.q);
    for i in a..b {
        let tt = i as f64 / d.rate;
        let n = d.rng.next() * 2. - 1.;
        let v = fl.run(n, fq.at(tt), d.rate);
        d.out[i] += (v * g.at(tt)) as f32;
    }
}

/// the sound, mono, at `rate`, before its volume; `dur` is the whistle's length (until the shell lands).
/// None for a name there is no sound for.
pub fn synth(name: &str, rate: u32, dur: Option<f64>, seed: u64) -> Option<Vec<f32>> {
    volume(name)?;
    let mut out = Vec::new();
    let mut rng = Rng::new(seed);
    let mut d = Dest {
        rate: rate as f64,
        out: &mut out,
        rng: &mut rng,
    };
    let d = &mut d;
    let t = 0.;
    match name {
        // a hand goes up: "?"
        "ask" => {
            tone(
                d,
                t,
                Tone {
                    f: 659.,
                    dur: 0.18,
                    wave: Wave::Triangle,
                    vol: 0.8,
                    ..TONE
                },
                None,
            );
            tone(
                d,
                t + 0.13,
                Tone {
                    f: 988.,
                    dur: 0.3,
                    wave: Wave::Triangle,
                    vol: 0.8,
                    ..TONE
                },
                None,
            );
        }
        "beam" => {
            tone(
                d,
                t,
                Tone {
                    f: 1900.,
                    f2: Some(420.),
                    dur: 0.3,
                    vol: 0.9,
                    ..TONE
                },
                None,
            );
            tone(
                d,
                t,
                Tone {
                    f: 950.,
                    f2: Some(260.),
                    dur: 0.22,
                    wave: Wave::Square,
                    vol: 0.1,
                    ..TONE
                },
                None,
            );
        }
        "door" => {
            tone(
                d,
                t,
                Tone {
                    f: 120.,
                    f2: Some(70.),
                    dur: 0.14,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.05,
                    kind: Filter::Lowpass,
                    f: 900.,
                    vol: 0.6,
                    ..HISS
                },
            );
            tone(
                d,
                t + 0.09,
                Tone {
                    f: 2200.,
                    dur: 0.03,
                    wave: Wave::Square,
                    vol: 0.05,
                    ..TONE
                },
                None,
            );
        }
        // the light rising in the booth: a shimmer that climbs, four seconds
        "booth" => {
            let g = Param::new(1.)
                .set(0.0001, t)
                .exp(1., t + 0.6)
                .set(1., t + 3.6)
                .exp(0.0001, t + 4.4);
            let (a, b) = d.span(t, t + 4.5);
            for (f, f2, w, vol) in [
                (220., 880., Wave::Sine, 0.6),
                (330., 1320., Wave::Triangle, 0.25),
                (441., 1764., Wave::Sine, 0.15),
            ] {
                let fq = Param::new(f).set(f, t).exp(f2, t + 4.2);
                let mut ph = 0.;
                for i in a..b {
                    let tt = i as f64 / d.rate;
                    let dt = fq.at(tt) / d.rate;
                    // the 9 Hz wobble is added to the gain: lg (0.35) into g.gain
                    let gain = g.at(tt) + 0.35 * (TAU * 9. * (tt - t)).sin();
                    d.out[i] += (wave(w, ph, dt) * vol * gain) as f32;
                    ph = (ph + dt) % 1.;
                }
            }
        }
        "poof" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.4,
                    kind: Filter::Highpass,
                    f: 5000.,
                    f2: Some(700.),
                    vol: 0.8,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 1300.,
                    f2: Some(160.),
                    dur: 0.32,
                    vol: 0.4,
                    ..TONE
                },
                None,
            );
        }
        // the booth gives one back
        "back" => tone(
            d,
            t,
            Tone {
                f: 1000.,
                f2: Some(300.),
                dur: 0.5,
                vol: 0.8,
                ..TONE
            },
            None,
        ),
        "shot" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.5,
                    kind: Filter::Lowpass,
                    f: 3200.,
                    f2: Some(260.),
                    vol: 1.,
                    a: 0.001,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 95.,
                    f2: Some(38.),
                    dur: 0.32,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
        }
        "clank" => {
            let k = d.rng.range(0.85, 1.2);
            for (f, dur) in [(640., 0.22), (1590., 0.14), (2870., 0.09)] {
                tone(
                    d,
                    t,
                    Tone {
                        f: f * k,
                        dur,
                        vol: 0.5,
                        ..TONE
                    },
                    None,
                );
            }
        }
        "flash" => {
            tone(
                d,
                t,
                Tone {
                    f: 2400.,
                    f2: Some(3800.),
                    dur: 0.16,
                    vol: 0.6,
                    ..TONE
                },
                None,
            );
            hiss(
                d,
                t + 0.04,
                Hiss {
                    dur: 0.3,
                    kind: Filter::Highpass,
                    f: 6500.,
                    vol: 0.4,
                    ..HISS
                },
            );
        }
        "bark" => {
            for at in [0., 0.19] {
                tone(
                    d,
                    t + at,
                    Tone {
                        f: 540.,
                        f2: Some(360.),
                        dur: 0.12,
                        wave: Wave::Saw,
                        vol: 0.7,
                        ..TONE
                    },
                    Some(1700.),
                );
                hiss(
                    d,
                    t + at,
                    Hiss {
                        dur: 0.09,
                        f: 950.,
                        q: 1.5,
                        vol: 0.5,
                        ..HISS
                    },
                );
            }
        }
        // a chessman put down
        "clack" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.045,
                    f: 1900.,
                    q: 3.,
                    vol: 0.9,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 720.,
                    dur: 0.05,
                    vol: 0.4,
                    ..TONE
                },
                None,
            );
        }
        // a dot-matrix printer: lines of needles, a line feed after each
        "print" => {
            for line in 0..6 {
                let l = line as f64;
                for k in 0..16 {
                    let f = d.rng.range(2400., 3200.);
                    hiss(
                        d,
                        t + l * 0.34 + k as f64 * 0.016,
                        Hiss {
                            dur: 0.011,
                            f,
                            q: 4.,
                            vol: 0.8,
                            ..HISS
                        },
                    );
                }
                tone(
                    d,
                    t + l * 0.34 + 0.27,
                    Tone {
                        f: 85.,
                        dur: 0.06,
                        wave: Wave::Square,
                        vol: 0.25,
                        ..TONE
                    },
                    None,
                );
            }
        }
        "whoosh" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.8,
                    kind: Filter::Lowpass,
                    f: 280.,
                    f2: Some(2600.),
                    vol: 0.9,
                    a: 0.15,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 70.,
                    dur: 0.6,
                    vol: 0.4,
                    a: 0.1,
                    ..TONE
                },
                None,
            );
        }
        // the radio on
        "tune" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.35,
                    f: 500.,
                    f2: Some(3000.),
                    q: 8.,
                    vol: 0.7,
                    ..HISS
                },
            );
            tone(
                d,
                t + 0.28,
                Tone {
                    f: 784.,
                    dur: 0.2,
                    wave: Wave::Triangle,
                    vol: 0.6,
                    ..TONE
                },
                None,
            );
            tone(
                d,
                t + 0.4,
                Tone {
                    f: 1047.,
                    dur: 0.35,
                    wave: Wave::Triangle,
                    vol: 0.6,
                    ..TONE
                },
                None,
            );
        }
        // the radio off
        "click" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.02,
                    kind: Filter::Highpass,
                    f: 3000.,
                    vol: 0.7,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 190.,
                    dur: 0.04,
                    vol: 0.5,
                    ..TONE
                },
                None,
            );
        }
        "paper" => hiss(
            d,
            t,
            Hiss {
                dur: 0.25,
                f: 4500.,
                f2: Some(2500.),
                q: 0.7,
                vol: 0.6,
                a: 0.04,
                ..HISS
            },
        ),
        // the duel outside the window (burrow-games' outside): a gun fired, the whistle of a shell coming down
        // (dur: until it lands), where it lands, a gun hit, a gunner landing on its back, the round won or lost
        "cannon" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.7,
                    kind: Filter::Lowpass,
                    f: 2400.,
                    f2: Some(140.),
                    vol: 1.,
                    a: 0.001,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 72.,
                    f2: Some(34.),
                    dur: 0.5,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
            tone(
                d,
                t,
                Tone {
                    f: 190.,
                    f2: Some(70.),
                    dur: 0.12,
                    wave: Wave::Square,
                    vol: 0.25,
                    ..TONE
                },
                None,
            );
        }
        "whistle" => {
            let dur = whistle_dur(dur);
            let fq = Param::new(1750.).set(1750., t).exp(620., t + dur);
            let g = Param::new(1.)
                .set(0.0001, t)
                .exp(1., t + (dur / 3.).min(0.25))
                .set(1., t + dur - 0.05)
                .exp(0.0001, t + dur);
            let (a, b) = d.span(t, t + dur + 0.05);
            let mut ph = 0.;
            for i in a..b {
                let tt = i as f64 / d.rate;
                // a 7 Hz warble of 18 Hz on the pitch
                let f = fq.at(tt) + 18. * (TAU * 7. * (tt - t)).sin();
                d.out[i] += ((TAU * ph).sin() * g.at(tt)) as f32;
                ph = (ph + f / d.rate) % 1.;
            }
        }
        "blast" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.9,
                    kind: Filter::Lowpass,
                    f: 1500.,
                    f2: Some(90.),
                    vol: 1.,
                    a: 0.002,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 58.,
                    f2: Some(30.),
                    dur: 0.65,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
            hiss(
                d,
                t + 0.05,
                Hiss {
                    dur: 0.35,
                    f: 700.,
                    q: 0.8,
                    vol: 0.35,
                    ..HISS
                },
            );
        }
        "wreck" => {
            hiss(
                d,
                t,
                Hiss {
                    dur: 1.4,
                    kind: Filter::Lowpass,
                    f: 1800.,
                    f2: Some(70.),
                    vol: 1.,
                    a: 0.002,
                    ..HISS
                },
            );
            tone(
                d,
                t,
                Tone {
                    f: 50.,
                    f2: Some(26.),
                    dur: 1.,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
            for (at, f) in [(0.18, 830.), (0.32, 1270.), (0.5, 640.)] {
                let f = f * d.rng.range(0.9, 1.1); // the gun's iron
                tone(
                    d,
                    t + at,
                    Tone {
                        f,
                        dur: 0.2,
                        vol: 0.18,
                        ..TONE
                    },
                    None,
                );
            }
        }
        "thud" => {
            tone(
                d,
                t,
                Tone {
                    f: 120.,
                    f2: Some(52.),
                    dur: 0.12,
                    vol: 1.,
                    ..TONE
                },
                None,
            );
            hiss(
                d,
                t,
                Hiss {
                    dur: 0.07,
                    kind: Filter::Lowpass,
                    f: 600.,
                    vol: 0.6,
                    ..HISS
                },
            );
        }
        "win" => {
            for (i, f) in [523., 659., 784., 1047.].into_iter().enumerate() {
                let dur = if i == 3 { 0.45 } else { 0.14 };
                tone(
                    d,
                    t + i as f64 * 0.11,
                    Tone {
                        f,
                        dur,
                        wave: Wave::Square,
                        vol: 0.35,
                        ..TONE
                    },
                    None,
                );
            }
        }
        "lose" => {
            for (i, f) in [392., 349., 262.].into_iter().enumerate() {
                let (f2, dur) = if i == 2 {
                    (Some(200.), 0.6)
                } else {
                    (None, 0.2)
                };
                tone(
                    d,
                    t + i as f64 * 0.22,
                    Tone {
                        f,
                        f2,
                        dur,
                        wave: Wave::Triangle,
                        vol: 0.8,
                        ..TONE
                    },
                    None,
                );
            }
        }
        _ => return None,
    }
    Some(out)
}

/// the whistle's length: the shell's time in the air, 0.3 to 4 s (1.5 when not said)
pub fn whistle_dur(dur: Option<f64>) -> f64 {
    dur.filter(|&d| d != 0. && !d.is_nan())
        .unwrap_or(1.5)
        .clamp(0.3, 4.)
}

/// left and right of a mono sound: Web Audio's StereoPanner (equal power); at pan 0 sound.js has no panner,
/// and the mono sound goes to both sides as it is
pub fn pan_gains(pan: f64) -> (f32, f32) {
    if pan == 0. {
        return (1., 1.);
    }
    let x = (pan.clamp(-1., 1.) + 1.) / 2. * PI / 2.;
    (x.cos() as f32, x.sin() as f32)
}

/// a Doom sound lump in DMX format (format 3, rate, count; the samples, 16 of padding each side): its rate
/// and its samples, -1..1
pub fn dmx(d: &[u8]) -> Option<(u32, Vec<f32>)> {
    if d.len() <= 40 || u16::from_le_bytes([d[0], d[1]]) != 3 {
        return None;
    }
    let rate = u16::from_le_bytes([d[2], d[3]]) as u32;
    let count = d[4] as usize | (d[5] as usize) << 8 | (d[6] as usize) << 16;
    let n = count.min(d.len() - 8) as i64 - 32;
    if n <= 0 || rate < 3000 {
        return None;
    }
    Some((
        rate,
        d[24..24 + n as usize]
            .iter()
            .map(|&v| (v as f32 - 128.) / 128.)
            .collect(),
    ))
}

/// samples at one rate to another, linearly between them
pub fn resample(s: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || s.is_empty() {
        return s.to_vec();
    }
    let n = (s.len() as u64 * to as u64 / from as u64) as usize;
    let k = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * k;
            let j = x.floor() as usize;
            let f = (x - j as f64) as f32;
            let a = s[j.min(s.len() - 1)];
            let b = s[(j + 1).min(s.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}
