//! burrow, native: the room on the left (the web room's pixel art, drawn by burrow-room into a buffer and
//! painted here), the pane on the right (tiles of terminals, or a panel). GPUI draws it on the GPU.

mod kit;
mod roomview;
mod tiles;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds, WindowOptions, point, px, size};
use std::borrow::Cow;

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowPixel.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowMono-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowMono-Bold.ttf").as_slice()),
            ])
            .expect("burrow's fonts");
        let bounds = Bounds::centered(None, size(px(1280.), px(820.)), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // the room runs behind the traffic lights, as Ghostty's terminal does
            titlebar: Some(TitlebarOptions { title: Some("burrow".into()), appears_transparent: true, traffic_light_position: Some(point(px(14.), px(14.))) }),
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(720.), px(480.))),
            focus: std::env::var_os("BURROW_NO_FOCUS").is_none(),
            app_id: Some("burrow".into()),
            ..Default::default()
        };
        cx.open_window(opts, |window, cx| cx.new(|cx| roomview::Burrow::new(window, cx))).expect("a window");
        cx.activate(std::env::var_os("BURROW_NO_FOCUS").is_none());
    });
}
