//! The window: the room on the left, painted from burrow-room's buffer (two pixels a dot, whole device pixels,
//! so the pixel art stays crisp), the pane on the right. The room thinks twelve times a second; the pointer
//! redraws it at once.

use burrow_room::io::{Input, Output};
use burrow_room::layout::{K, MARGIN};
use burrow_room::room::Room;
use gpui::{
    App, Bounds, Context, Corners, CursorStyle, Hsla, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Render, RenderImage, Styled, Window, canvas, div, prelude::*, px, rgb,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// the night behind everything, the room's frame
const NIGHT: u32 = 0x0b0812;

pub struct Burrow {
    pub room: Room,
    pub inp: Input,
    /// the room as last drawn, uploaded once (dropped from the GPU when a newer one comes)
    picture: Option<Arc<RenderImage>>,
    stale: Vec<Arc<RenderImage>>,
    room_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    started: Instant,
    last: Instant,
    dirty: bool,
}

impl Burrow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Burrow {
        let state = burrow_model::State::from_json(include_str!("../../model/tests/fixtures/demo-state.json")).expect("the demo state");
        let inp = Input { state, hour: 14, ..Default::default() };
        let room = Room::new(1, 0.0);
        // a frame every sixtieth of a second: the room moves at twelve, a drag at once
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(16)).await;
                if this.update(cx, |b, cx| b.frame(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let now = Instant::now();
        Burrow { room, inp, picture: None, stale: Vec::new(), room_bounds: Rc::new(Cell::new(None)), started: now, last: now, dirty: true }
    }

    fn now_ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    fn frame(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.inp.now = self.now_ms();
        let ticked = self.room.advance(dt);
        if ticked || self.dirty {
            self.redraw();
            cx.notify();
        }
    }

    /// step, draw, and turn the buffer into a picture for the GPU
    fn redraw(&mut self) {
        self.dirty = false;
        let mut out = Output::default();
        let l = self.room.step(&self.inp, &mut out);
        self.room.draw(&self.inp, &mut out, &l);
        self.handle(out);
        let scale = 2; // device pixels a canvas pixel (a retina screen); set from the window when painting
        let bytes = self.room.img.upscale_bgra(scale, [0x0b, 0x08, 0x12, 255]);
        let (w, h) = (self.room.img.w * scale, self.room.img.h * scale);
        if let Some(buf) = image::RgbaImage::from_raw(w, h, bytes) {
            if let Some(old) = self.picture.replace(Arc::new(RenderImage::new(vec![image::Frame::new(buf)]))) {
                self.stale.push(old);
            }
        }
    }

    /// what the room asked for
    fn handle(&mut self, out: Output) {
        for a in out.acts {
            let _ = a; // the backend takes them (not wired yet)
        }
    }

    fn dots(&self, p: gpui::Point<Pixels>) -> Option<(f64, f64)> {
        let b = self.room_bounds.get()?;
        Some((f64::from(p.x - b.origin.x) / K as f64, f64::from(p.y - b.origin.y) / K as f64))
    }
}

impl Render for Burrow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // the room as tall as the window, as wide as the divider says
        let wh = f64::from(window.viewport_size().height);
        let (w0, h0) = (self.room.size.w, self.room.size.h);
        self.room.resize(&self.inp, wh);
        if (w0, h0) != (self.room.size.w, self.room.size.h) {
            self.dirty = true;
        }
        for old in self.stale.drain(..) {
            let _ = window.drop_image(old);
        }
        let pic = self.picture.clone();
        let bounds = self.room_bounds.clone();
        let (rw, rh) = (self.room.size.w as f32 * K as f32, self.room.size.h as f32 * K as f32);
        let cursor = match self.room.cursor {
            "pointer" => CursorStyle::PointingHand,
            "grabbing" => CursorStyle::ClosedHand,
            "grab" => CursorStyle::OpenHand,
            _ => CursorStyle::Arrow,
        };
        let room = div()
            .id("room")
            .w(px(rw))
            .h(px(rh))
            .cursor(cursor)
            .child(
                canvas(
                    move |b, _, _| bounds.set(Some(b)),
                    move |b, _, window, _| {
                        if let Some(pic) = pic {
                            let _ = window.paint_image(b, Corners::default(), pic, 0, false);
                        }
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    if let Some((x, y)) = this.dots(e.position) {
                        this.room.pointer_down(x, y, e.modifiers.shift);
                        this.dirty = true;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, _| {
                if let Some((x, y)) = this.dots(e.position) {
                    let mut out = Output::default();
                    this.room.pointer_move(x, y, (f64::from(e.position.x), f64::from(e.position.y)), &mut out);
                    this.handle(out);
                    this.dirty = true;
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    let mut out = Output::default();
                    let inp = this.inp.clone();
                    this.room.pointer_up(&inp, &mut out, (f64::from(e.position.x), f64::from(e.position.y)));
                    this.handle(out);
                    this.dirty = true;
                    cx.notify();
                }),
            );
        div()
            .size_full()
            .bg(rgb(NIGHT))
            .flex()
            .flex_row()
            .child(div().p(px(MARGIN as f32)).pr(px(0.)).child(room))
            .child(div().flex_1().h_full().bg(Hsla::from(rgb(NIGHT))))
    }
}

#[allow(dead_code)]
fn _app(_: &mut App) {}
