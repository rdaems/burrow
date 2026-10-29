//! The gun closet's first person (hardcore), over the whole window (web/index.html's #fps and room.js's firstPerson):
//! burrow-fps's Doom engine drawing the office as the room has it, letterboxed at 4:3 in big pixels; the pointer
//! locked as the browser locked it (the cursor hidden and held still, the mouse's own movement turning you); WASD,
//! the shotgun, E. What happens in there happens in the room: a bot shot is shot, you walk where you walk. Esc, or
//! the window losing the focus, and you are back, standing where you went.

use crate::burrow::Burrow;
use burrow_fps::{Event as FEv, Fps, Key, Melt, Options, Sound as FSound, Start, Wad};
use burrow_room::io::Output;
use gpui::{Context, KeyDownEvent, KeyUpEvent, ModifiersChangedEvent, Window};
use pix::Image;
use std::sync::Arc;

pub struct FpsRun {
    pub fps: Fps,
    pub img: Image,
    pub picture: Option<Arc<gpui::RenderImage>>,
    shift: bool,
    ctrl: bool,
}

/// the pointer is let go whatever way the first person ends (left, dropped, the app quitting)
impl Drop for FpsRun {
    fn drop(&mut self) {
        lock(false);
    }
}

/// a panic must not leave the mouse held and hidden for the rest of the session: let it go first
pub fn release_on_panic() {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if LOCKED.load(std::sync::atomic::Ordering::Relaxed) {
            lock(false);
        }
        before(info);
    }));
}

/// let the pointer go, if it is held (the app quitting: no destructors run then)
pub fn unlock() {
    lock(false);
}

static LOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// the melt over the room once you are out
pub struct Leaving {
    pub melt: Melt,
    pub img: Image,
    pub picture: Option<Arc<gpui::RenderImage>>,
}

unsafe extern "C" {
    fn CGAssociateMouseAndMouseCursorPosition(connected: u32) -> i32;
    fn CGGetLastMouseDelta(dx: *mut i32, dy: *mut i32);
}

fn lock(on: bool) {
    if std::env::var_os("BURROW_SHOT").is_some() {
        return; // a picture of itself: the mouse is someone else's
    }
    if !on && !LOCKED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        return; // let go already
    }
    if on {
        LOCKED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    unsafe {
        use objc::{class, msg_send, sel, sel_impl};
        let _: () = if on { msg_send![class!(NSCursor), hide] } else { msg_send![class!(NSCursor), unhide] };
        CGAssociateMouseAndMouseCursorPosition(if on { 0 } else { 1 });
        if on {
            let (mut dx, mut dy) = (0, 0);
            CGGetLastMouseDelta(&mut dx, &mut dy); // what moved before does not count
        }
    }
}

/// how far the mouse moved since it was last asked
pub fn mouse_delta() -> f64 {
    let (mut dx, mut dy) = (0, 0);
    unsafe { CGGetLastMouseDelta(&mut dx, &mut dy) };
    dx as f64
}

fn key_of(k: &str) -> Option<Key> {
    Some(match k {
        "w" => Key::W,
        "a" => Key::A,
        "s" => Key::S,
        "d" => Key::D,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "e" => Key::E,
        "space" => Key::Space,
        "enter" => Key::Enter,
        "f" => Key::F,
        "escape" => Key::Escape,
        _ => return None,
    })
}

impl Burrow {
    /// into the first person at the gun closet `key`: your face on the status bar, your hands on the gun
    pub fn start_fps(&mut self, key: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if self.fps.is_some() || !self.state.settings.hardcore {
            return;
        }
        let wad = self.hub.cfg.doom_wad().and_then(|p| Wad::load(p).ok()).or_else(|| Wad::open_bytes(burrow_fps::wad::freedoom()).ok());
        let Some(wad) = wad else { return };
        let door = Wad::open_bytes(burrow_fps::wad::door_wad()).ok();
        let inp = self.input();
        let Some(scene) = self.room.scene3(&inp) else { return };
        let (x, y, a) = self.room.fps_start(key);
        let you = self.room.w.get("you").cloned();
        let seed = you.as_ref().map(|f| f.seed.clone()).unwrap_or_else(|| "you".into());
        let color = you.as_ref().and_then(|f| f.color);
        let face = (*burrow_room::avatar::avatar(&seed, false, color)).clone();
        let opts = Options { still: inp.still, seed: inp.now as u64 ^ 0x5eed, face: Some(face), skin: Some(burrow_room::avatar::skin(&seed)), snapshot: None };
        match Fps::new(wad, door, scene, Start { x, y, a }, opts) {
            Ok(fps) => {
                self.fps = Some(FpsRun { fps, img: Image::new(1, 1), picture: None, shift: false, ctrl: false });
                lock(true);
                cx.notify();
            }
            Err(e) => eprintln!("burrow: the first person: {e}"),
        }
    }

    /// out of it: the room as it is, the melt over it, you where you went
    pub fn stop_fps(&mut self, cx: &mut Context<Self>) {
        let Some(mut run) = self.fps.take() else { return };
        lock(false); // (its drop would too)
        self.room.left_fps();
        if let Some(melt) = run.fps.take_leave_melt() {
            self.leaving = Some(Leaving { melt, img: Image::new(1, 1), picture: None });
        }
        if let Some(p) = run.picture.take() {
            self.stale_pictures.push(p);
        }
        cx.notify();
    }

    /// a frame of it: time, what happened in there, the picture
    pub fn fps_frame(&mut self, dt: f64, ticked: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.fps.is_some() && !self.state.settings.hardcore {
            return self.stop_fps(cx); // no guns outside hardcore
        }
        let Some(run) = self.fps.as_mut() else { return };
        let dx = mouse_delta();
        if dx != 0.0 {
            run.fps.mouse_moved(dx);
        }
        let shots = self.room.take_incoming().into_iter().map(|s| burrow_fps::Incoming { x: s.x, y: s.y, hit: s.hit, dmg: s.dmg }).collect();
        let run = self.fps.as_mut().unwrap();
        run.fps.incoming(shots);
        run.fps.tick(dt * 1000.0);
        let events = run.fps.events();
        let you = run.fps.you();
        self.room.move_you(you.x, you.y, you.a, you.walking);
        if ticked {
            let inp = self.input();
            if let Some(scene) = self.room.scene3(&inp) {
                self.fps.as_mut().unwrap().fps.set_scene(scene);
            }
        }
        let mut leave = false;
        for e in events {
            let inp = self.input();
            let mut out = Output::default();
            match e {
                FEv::Shot { bot } => {
                    self.room.shoot(&inp, &mut out, &bot);
                }
                FEv::Unshoot { rem } => self.room.unshoot(inp.now, &rem),
                FEv::Alarm { x, y } => self.room.alarm(&inp, &mut out, x, y),
                FEv::Sound(FSound::Doom { name, vol, pan, origin }) => {
                    if let Some(lump) = self.fps.as_ref().and_then(|r| r.fps.wad().lump(name)).map(|l| l.to_vec()) {
                        self.sound.doom(&lump, vol, pan, Some(&format!("{origin:?}")));
                    }
                }
                FEv::Sound(FSound::Synth { name, vol, pan, .. }) => {
                    self.sound.play(name, pan, vol, false, None);
                }
                FEv::Leave => leave = true,
                // the mutiny: you dead of their shots, then Doom's respawn (the room's too)
                FEv::Died => self.room.died(&inp, &mut out),
                FEv::Respawn => self.room.respawn(&inp),
            }
            self.after(out, window, cx);
        }
        if leave {
            return self.stop_fps(cx);
        }
        // the frame at the window's size (CSS pixels), then whole device pixels: big, crisp pixels
        let size = window.viewport_size();
        let (w, h) = (f32::from(size.width).round() as u32, f32::from(size.height).round() as u32);
        let k = window.scale_factor().round().max(1.0) as u32;
        let run = self.fps.as_mut().unwrap();
        if run.img.w != w || run.img.h != h {
            run.img = Image::new(w.max(1), h.max(1));
        }
        run.fps.draw_window(&mut run.img);
        let pic = crate::kit::picture_opaque(&run.img, k);
        if let Some(old) = run.picture.replace(pic) {
            self.stale_pictures.push(old);
        }
        cx.notify();
    }

    /// the melt over the room after leaving
    pub fn leaving_frame(&mut self, now: f64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(l) = self.leaving.as_mut() else { return };
        let size = window.viewport_size();
        let (w, h) = (f32::from(size.width).round() as u32, f32::from(size.height).round() as u32);
        let k = window.scale_factor().round().max(1.0) as u32;
        if l.img.w != w || l.img.h != h {
            l.img = Image::new(w.max(1), h.max(1));
        }
        l.img.clear();
        if !l.melt.draw(&mut l.img, now) {
            if let Some(p) = l.picture.take() {
                self.stale_pictures.push(p);
            }
            self.leaving = None;
        } else {
            let pic = crate::kit::picture(&l.img, k);
            if let Some(old) = l.picture.replace(pic) {
                self.stale_pictures.push(old);
            }
        }
        cx.notify();
    }

    pub fn fps_key(&mut self, e: &KeyDownEvent) -> bool {
        let Some(run) = self.fps.as_mut() else { return false };
        if let Some(k) = key_of(e.keystroke.key.as_str())
            && !e.is_held
        {
            run.fps.key_down(k);
        }
        true
    }
    pub fn fps_key_up(&mut self, e: &KeyUpEvent) -> bool {
        let Some(run) = self.fps.as_mut() else { return false };
        if let Some(k) = key_of(e.keystroke.key.as_str()) {
            run.fps.key_up(k);
        }
        true
    }
    pub fn fps_modifiers(&mut self, e: &ModifiersChangedEvent) {
        let Some(run) = self.fps.as_mut() else { return };
        if e.modifiers.shift != run.shift {
            run.shift = e.modifiers.shift;
            if run.shift { run.fps.key_down(Key::ShiftLeft) } else { run.fps.key_up(Key::ShiftLeft) }
        }
        if e.modifiers.control != run.ctrl {
            run.ctrl = e.modifiers.control;
            if run.ctrl { run.fps.key_down(Key::ControlLeft) } else { run.fps.key_up(Key::ControlLeft) }
        }
    }
    pub fn fps_click(&mut self) {
        if let Some(run) = self.fps.as_mut() {
            run.fps.click();
        }
    }
}
