//! burrow, native: the room on the left (the web room's pixel art, drawn by burrow-room into a buffer and
//! painted here), the pane on the right (tiles of terminals, or a panel). GPUI draws it on the GPU.

mod burrow;
mod icon;
mod kit;
mod notes;
mod panels;
mod stars;
mod term;
mod tiles;
mod views;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds, WindowOptions, point, px, size};
use std::borrow::Cow;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--icon") {
        // for macos/bundle.sh
        let out = args.get(i + 1).map(String::as_str).unwrap_or("icon.png");
        icon::icon().save_png(out).expect("the icon");
        return;
    }
    Application::new().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowPixel.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowMono-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../../../assets/fonts/BurrowMono-Bold.ttf").as_slice()),
            ])
            .expect("burrow's fonts");
        let bounds = Bounds::centered(None, size(px(1280.), px(820.)), cx);
        let focus = std::env::var_os("BURROW_NO_FOCUS").is_none();
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // the room runs behind the traffic lights, as Ghostty's terminal does
            titlebar: Some(TitlebarOptions { title: Some("burrow".into()), appears_transparent: true, traffic_light_position: Some(point(px(14.), px(14.))) }),
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(720.), px(480.))),
            focus,
            app_id: Some("burrow".into()),
            ..Default::default()
        };
        cx.open_window(opts, |window, cx| cx.new(|cx| burrow::Burrow::new(window, cx))).expect("a window");
        cx.activate(focus);
    });
}
