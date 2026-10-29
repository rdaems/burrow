//! What the room sounds like, while a radio stands in it and is on (room.json settings.sound; the radio is the
//! switch). A port of web/sound.js. Nothing recorded: the room's sounds are made here (synth.rs), short and
//! quiet, from where they happen (left or right by where in the room: pan -1..1); Doom's come out of its WAD
//! (`doom`), in Doom's own format (DMX: a rate, then 8-bit samples).
//!
//! `Sound::set(on)`; `Sound::play(name, pan, vol, force, dur)` → a handle for `stop` (dur is the whistle's
//! length); `Sound::doom(lump, vol, pan, origin)`. Only what happens is heard, never a background. The audio
//! device is opened on the first sound that plays, never before: a Sound that only ever stays off (or a test)
//! opens nothing.

mod synth;
pub use synth::{SOUNDS, dmx, pan_gains, resample, stop_fade, synth, volume, whistle_dur};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex, mpsc};

/// the master volume (sound.js's master gain)
pub const MASTER: f64 = 0.7;

/// the sound made, in stereo, as it leaves for the speakers: the master, the sound's own volume, the caller's,
/// left or right by pan
pub fn render(
    name: &str,
    rate: u32,
    pan: f64,
    vol: f64,
    dur: Option<f64>,
    seed: u64,
) -> Option<Vec<[f32; 2]>> {
    let mono = synth(name, rate, dur, seed)?;
    let g = (MASTER * volume(name)? * vol) as f32;
    let (l, r) = pan_gains(pan);
    Some(mono.iter().map(|&v| [v * g * l, v * g * r]).collect())
}

/// a sound playing, for stopping it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle(u64);

struct Voice {
    id: u64,
    frames: Vec<[f32; 2]>,
    pos: usize,
    /// fading out once stopped: (frames gone, frames it takes)
    fade: Option<(usize, usize)>,
    /// how long its fade is, in frames (none: a stop changes nothing, it plays to its end)
    stop_fade: Option<usize>,
    origin: Option<String>,
}

#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
}
impl Mixer {
    /// the next frames, summed, into an interleaved buffer of `ch` channels
    fn fill(&mut self, out: &mut [f32], ch: usize) {
        out.fill(0.);
        for v in &mut self.voices {
            for frame in out.chunks_mut(ch) {
                if v.pos >= v.frames.len() {
                    break;
                }
                let mut s = v.frames[v.pos];
                if let Some((k, n)) = v.fade.as_mut() {
                    // exponentially down to 0.0001, as the gain's ramp in sound.js
                    let g = 0.0001f32.powf(*k as f32 / *n as f32);
                    s = [s[0] * g, s[1] * g];
                    *k += 1;
                    if *k >= *n {
                        v.pos = v.frames.len();
                    }
                }
                if ch == 1 {
                    frame[0] += (s[0] + s[1]) / 2.;
                } else {
                    frame[0] += s[0];
                    frame[1] += s[1];
                }
                v.pos += 1;
            }
        }
        self.voices.retain(|v| v.pos < v.frames.len());
    }
    fn stop(&mut self, id: u64) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.id == id)
            && let (Some(n), None) = (v.stop_fade, v.fade)
        {
            v.fade = Some((0, n.max(1)));
        }
    }
}

/// the speakers: the stream lives on a thread of its own (a cpal stream may not move between threads)
struct Out {
    rate: u32,
    mixer: Arc<Mutex<Mixer>>,
}
fn open() -> Option<Out> {
    let (tx, rx) = mpsc::channel();
    let mixer = Arc::new(Mutex::new(Mixer::default()));
    let m = mixer.clone();
    std::thread::Builder::new()
        .name("burrow-sound".into())
        .spawn(move || {
            let Some(stream) = stream(m) else {
                let _ = tx.send(None);
                return;
            };
            let _ = tx.send(Some(stream.1));
            let _keep = stream.0;
            loop {
                std::thread::park();
            }
        })
        .ok()?;
    let rate = rx.recv().ok()??;
    Some(Out { rate, mixer })
}
fn stream(m: Arc<Mutex<Mixer>>) -> Option<(cpal::Stream, u32)> {
    let dev = cpal::default_host().default_output_device()?;
    let cfg = dev.default_output_config().ok()?;
    let fmt = cfg.sample_format();
    let cfg: cpal::StreamConfig = cfg.into();
    let (rate, ch) = (cfg.sample_rate, cfg.channels as usize);
    let err = |e: cpal::Error| eprintln!("burrow-sound: {e}");
    let s = match fmt {
        cpal::SampleFormat::F32 => dev
            .build_output_stream(
                cfg,
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    m.lock().unwrap().fill(out, ch)
                },
                err,
                None,
            )
            .ok()?,
        cpal::SampleFormat::I16 => {
            let mut buf = Vec::new();
            dev.build_output_stream(
                cfg,
                move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    buf.resize(out.len(), 0.);
                    m.lock().unwrap().fill(&mut buf, ch);
                    for (o, v) in out.iter_mut().zip(&buf) {
                        *o = (v.clamp(-1., 1.) * i16::MAX as f32) as i16;
                    }
                },
                err,
                None,
            )
            .ok()?
        }
        _ => return None,
    };
    s.play().ok()?;
    Some((s, rate))
}

/// the room's sounds: on or off (the radio), and the speakers once something has played
pub struct Sound {
    on: bool,
    out: Option<Option<Out>>, // not opened yet; opened, or it could not be
    next: u64,
    seed: u64,
    live: Vec<(u64, Option<String>)>,
}

impl Default for Sound {
    fn default() -> Self {
        Self::new()
    }
}

impl Sound {
    pub fn new() -> Sound {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Sound {
            on: false,
            out: None,
            next: 0,
            seed,
            live: Vec::new(),
        }
    }
    pub fn on(&self) -> bool {
        self.on
    }
    /// the radio on or off; off stops what can be stopped (the booth's light, a whistle)
    pub fn set(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        self.on = on;
        if !on {
            if let Some(Some(o)) = &self.out {
                let mut m = o.mixer.lock().unwrap();
                for (id, _) in &self.live {
                    m.stop(*id);
                }
            }
            self.live.clear();
        }
    }
    fn out(&mut self) -> Option<&Out> {
        if self.out.is_none() {
            self.out = Some(open());
        }
        self.out.as_ref().unwrap().as_ref()
    }
    fn add(
        &mut self,
        frames: Vec<[f32; 2]>,
        stop_fade: Option<f64>,
        origin: Option<String>,
    ) -> Option<Handle> {
        self.next += 1;
        let id = self.next;
        let o = self.out()?;
        let fade = stop_fade.map(|s| (s * o.rate as f64) as usize);
        let mut m = o.mixer.lock().unwrap();
        if let Some(org) = &origin {
            m.voices.retain(|v| v.origin.as_ref() != Some(org)); // one sound an origin at a time (Doom's channels)
        }
        m.voices.push(Voice {
            id,
            frames,
            pos: 0,
            fade: None,
            stop_fade: fade,
            origin: origin.clone(),
        });
        drop(m);
        self.live.retain(|(_, o)| o.is_none() || *o != origin);
        self.live.push((id, origin));
        if self.live.len() > 64 {
            self.live.remove(0);
        }
        Some(Handle(id))
    }
    /// a sound, while the radio is on (or forced: the radio's own click and tune); pan -1..1, vol on top of the
    /// sound's own, dur the whistle's length
    pub fn play(
        &mut self,
        name: &str,
        pan: f64,
        vol: f64,
        force: bool,
        dur: Option<f64>,
    ) -> Option<Handle> {
        volume(name)?;
        if !(self.on || force) {
            return None;
        }
        let rate = self.out()?.rate;
        self.seed = self.seed.wrapping_add(1);
        let frames = render(name, rate, pan, vol, dur, self.seed)?;
        self.add(frames, stop_fade(name), None)
    }
    /// stop it: the booth and the whistle fade out, the rest play to their end (as sound.js's handles)
    pub fn stop(&mut self, h: Handle) {
        if let Some(Some(o)) = &self.out {
            o.mixer.lock().unwrap().stop(h.0);
        }
    }
    /// a Doom sound lump, one sound an origin at a time
    pub fn doom(
        &mut self,
        lump: &[u8],
        vol: f64,
        pan: f64,
        origin: Option<&str>,
    ) -> Option<Handle> {
        if !self.on || vol <= 0.01 {
            return None;
        }
        let (from, s) = dmx(lump)?;
        let rate = self.out()?.rate;
        let g = (MASTER * 0.55 * vol) as f32;
        let (l, r) = pan_gains(pan);
        let frames = resample(&s, from, rate)
            .into_iter()
            .map(|v| [v * g * l, v * g * r])
            .collect();
        self.add(frames, None, origin.map(String::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48000;

    /// when each sound's last part ends (its stop: dur + 0.05 after it starts)
    fn ends(name: &str) -> f64 {
        match name {
            "ask" => 0.13 + 0.3 + 0.05,
            "beam" => 0.35,
            "door" => 0.19,
            "booth" => 4.5,
            "poof" => 0.45,
            "back" => 0.55,
            "shot" => 0.55,
            "clank" => 0.27,
            "flash" => 0.04 + 0.3 + 0.05,
            "bark" => 0.19 + 0.12 + 0.05,
            "clack" => 0.1,
            "print" => 5. * 0.34 + 0.27 + 0.06 + 0.05,
            "whoosh" => 0.85,
            "tune" => 0.4 + 0.35 + 0.05,
            "click" => 0.09,
            "paper" => 0.3,
            "cannon" => 0.75,
            "whistle" => 2.5 + 0.05,
            "blast" => 0.95,
            "wreck" => 1.45,
            "thud" => 0.17,
            "win" => 3. * 0.11 + 0.45 + 0.05,
            "died" => 0.05 + 1.8 + 0.05,
            "lose" => 2. * 0.22 + 0.6 + 0.05,
            _ => unreachable!(),
        }
    }

    #[test]
    fn every_sound_is_made_and_heard() {
        assert_eq!(SOUNDS.len(), 24);
        for (name, _) in SOUNDS {
            let s = synth(name, RATE, Some(2.5), 7).unwrap();
            let want = (ends(name) * RATE as f64).round() as usize;
            assert!(
                s.len().abs_diff(want) <= 1,
                "{name}: {} samples, not {want}",
                s.len()
            );
            assert!(s.iter().all(|v| v.is_finite()), "{name}");
            let peak = s.iter().fold(0f32, |a, v| a.max(v.abs()));
            assert!(peak > 0.05 && peak < 4., "{name}: peak {peak}");
            let rms = (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt();
            assert!(rms > 0.005, "{name}: rms {rms}");
            // quiet at the end, as each one dies away
            let tail = &s[s.len() - RATE as usize / 100..];
            assert!(
                tail.iter().all(|v| v.abs() < 0.4 * peak.max(0.1)),
                "{name} ends loud"
            );
        }
        assert!(synth("nothing", RATE, None, 1).is_none());
    }

    #[test]
    fn the_whistle_lasts_until_the_shell_lands() {
        for (dur, want) in [
            (Some(1.0), 1.0),
            (Some(9.), 4.),
            (Some(0.1), 0.3),
            (None, 1.5),
            (Some(0.), 1.5),
        ] {
            let s = synth("whistle", RATE, dur, 1).unwrap();
            assert_eq!(s.len(), ((want + 0.05) * RATE as f64).round() as usize);
        }
    }

    #[test]
    fn a_tone_is_at_its_pitch() {
        // "back" glides 1000 to 300 Hz over half a second: count its zero crossings in the first 50 ms
        let s = synth("back", RATE, None, 1).unwrap();
        let n = RATE as usize / 20;
        let zc = s[1..n]
            .iter()
            .zip(&s[..n - 1])
            .filter(|(a, b)| (**a > 0.) != (**b > 0.))
            .count();
        let f = zc as f64 / 2. / 0.05; // about 1000 · (0.3)^(t/0.5) on average over the first 50 ms: ~ 943
        assert!((880. ..1000.).contains(&f), "{f}");
    }

    #[test]
    fn stereo_is_panned_as_web_audio_pans() {
        let mid = render("thud", RATE, 0., 1., None, 3).unwrap();
        assert!(mid.iter().all(|f| f[0] == f[1]));
        let left = render("thud", RATE, -1., 1., None, 3).unwrap();
        assert!(left.iter().all(|f| f[1].abs() < 1e-6));
        let (l, r) = pan_gains(0.5);
        assert!((l * l + r * r - 1.).abs() < 1e-6 && r > l);
        let peak = |v: &[[f32; 2]]| v.iter().fold(0f32, |a, f| a.max(f[0].abs()));
        let one = synth("thud", RATE, None, 3)
            .unwrap()
            .iter()
            .fold(0f32, |a, v| a.max(v.abs()));
        assert!((peak(&mid) - one * (MASTER * 0.16) as f32).abs() < 1e-4);
    }

    #[test]
    fn a_stopped_whistle_fades_and_others_play_on() {
        let mut m = Mixer::default();
        m.voices.push(Voice {
            id: 1,
            frames: vec![[1., 1.]; 1000],
            pos: 0,
            fade: None,
            stop_fade: Some(100),
            origin: None,
        });
        m.voices.push(Voice {
            id: 2,
            frames: vec![[1., 1.]; 1000],
            pos: 0,
            fade: None,
            stop_fade: None,
            origin: None,
        });
        let mut out = vec![0f32; 20];
        m.fill(&mut out, 2);
        assert_eq!(out[0], 2.);
        m.stop(1);
        m.stop(2);
        let mut out = vec![0f32; 400];
        m.fill(&mut out, 2);
        assert!(out[0] > 1.9 && out[2 * 99] < 1.01 && out[2 * 150] == 1.);
        assert_eq!(m.voices.len(), 1);
    }

    #[test]
    fn doom_lumps_are_read() {
        let mut lump = vec![3, 0, 0x11, 0x2b, 0, 0, 0, 0];
        let n = 32 + 100;
        lump[4] = n as u8;
        lump.extend(std::iter::repeat_n(128u8, 16));
        lump.extend((0..100).map(|i| if i % 2 == 0 { 255 } else { 1 }));
        lump.extend(std::iter::repeat_n(128u8, 16));
        let (rate, s) = dmx(&lump).unwrap();
        assert_eq!((rate, s.len()), (11025, 100));
        assert!((s[0] - 127. / 128.).abs() < 1e-6);
        assert_eq!(resample(&s, 11025, 44100).len(), 400);
        assert!(dmx(&lump[..30]).is_none());
    }

    #[test]
    fn off_it_plays_nothing_and_opens_nothing() {
        let mut s = Sound::new();
        assert!(s.play("ask", 0., 1., false, None).is_none());
        assert!(s.doom(&[3, 0], 1., 0., None).is_none());
        s.set(false);
        assert!(s.out.is_none(), "no device opened");
    }

    /// the page's own sounds, rendered offline in headless Chrome (tests/web/sound.py): how loud each is every
    /// 10 ms. Noise is noise and Chrome's oscillators and filters are its own, so: the loudness over time, near
    #[test]
    fn sounds_are_the_pages() {
        let web: std::collections::HashMap<String, Vec<f64>> = parse_rms(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/web/sound-rms.json"
            ))
            .unwrap(),
        );
        let mut worst = Vec::new();
        for (name, _) in SOUNDS {
            let w = &web[name];
            // Chrome's start 10 ms in: one window later
            let mine: Vec<f64> = std::iter::once(0.)
                .chain(
                    render(name, RATE, 0., 1., Some(2.5), 5)
                        .unwrap()
                        .chunks(480)
                        .filter(|c| c.len() == 480)
                        .map(|c| {
                            (c.iter().map(|f| (f[0] as f64).powi(2)).sum::<f64>() / 480.).sqrt()
                        }),
                )
                .collect();
            let total = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
            let ratio = total(&mine) / total(w);
            // in 30 ms, three windows: an 11 ms needle of noise is too short a thing to compare on its own
            let thirds = |v: &[f64]| {
                v.chunks(3)
                    .map(|c| (c.iter().map(|x| x * x).sum::<f64>() / c.len() as f64).sqrt())
                    .collect::<Vec<_>>()
            };
            let (w, mine) = (&thirds(w), thirds(&mine));
            let peak = w.iter().cloned().fold(0., f64::max);
            let dbs: Vec<f64> = w
                .iter()
                .zip(&mine)
                .filter(|(a, _)| **a > peak * 0.1)
                .map(|(a, b)| (20. * (b.max(1e-9) / a).log10()).abs())
                .collect();
            let off = dbs.iter().cloned().fold(0., f64::max);
            let mean = dbs.iter().sum::<f64>() / dbs.len() as f64;
            if std::env::var("BURROW_DIFF").as_deref() == Ok(name) {
                eprintln!("{w:?}\n{mine:?}");
            }
            if std::env::var("BURROW_DIFF").is_ok() {
                eprintln!(
                    "{name:8} loudness x{ratio:.2}  30 ms: {mean:.1} dB off on average, {off:.1} at worst"
                );
            }
            worst.push((name, ratio, mean, off));
        }
        for (name, ratio, mean, off) in worst {
            assert!(
                (0.8..1.25).contains(&ratio),
                "{name}: {ratio:.2} times as loud as the page"
            );
            assert!(
                mean < 2. && off < 8.,
                "{name}: {mean:.1} dB off the page's on average, {off:.1} at worst"
            );
        }
    }
    /// {"name": [numbers], …}, as sound.py writes it
    fn parse_rms(s: &str) -> std::collections::HashMap<String, Vec<f64>> {
        let mut out = std::collections::HashMap::new();
        for part in s
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .split("]")
        {
            let Some((k, v)) = part.split_once(": [") else {
                continue;
            };
            let k = k
                .trim_start_matches(", ")
                .trim()
                .trim_matches('"')
                .to_string();
            out.insert(
                k,
                v.split(',').filter_map(|x| x.trim().parse().ok()).collect(),
            );
        }
        out
    }
}
