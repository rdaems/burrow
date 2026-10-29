//! pair's starfield where nothing is open (pixel.js's --stars): a 240 px tile of eight steel dots, repeated.

use crate::kit::{self, col};
use gpui::{App, Bounds, Corners, IntoElement, Pixels, Styled, Window, canvas, point, px, size};

const DOTS: [(f32, f32, u32); 8] = [
    (14., 22., col::STEEL1),
    (92., 8., col::STEEL2),
    (170., 40., col::STEEL0),
    (210., 120., col::STEEL1),
    (56., 150., col::STEEL0),
    (130., 196., col::STEEL2),
    (20., 218., col::STEEL1),
    (150., 98., col::STEEL0),
];

pub fn stars() -> impl IntoElement {
    canvas(
        |_, _, _| {},
        |b: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
            let mut ty = 0.0;
            while ty < h {
                let mut tx = 0.0;
                while tx < w {
                    for (x, y, c) in DOTS {
                        let at = point(b.origin.x + px(tx + x), b.origin.y + px(ty + y));
                        window.paint_quad(gpui::fill(Bounds::new(at, size(px(2.), px(2.))), kit::hsla(c)));
                    }
                    tx += 240.0;
                }
                ty += 240.0;
            }
            let _ = Corners::<Pixels>::default();
        },
    )
    .absolute()
    .size_full()
}
