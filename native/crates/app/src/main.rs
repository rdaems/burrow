//! burrow, native: the room on the left (the web room's pixel art, drawn by burrow-room into a buffer and
//! painted here), the pane on the right (tiles of terminals, or a panel). GPUI draws it on the GPU.

mod burrow;
mod field;
mod fpsview;
mod icon;
mod kit;
mod matrix;
mod notes;
mod panels;
mod shot;
mod stars;
mod term;
mod tiles;
mod views;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds, WindowOptions, point, px, size};
use std::borrow::Cow;

fn main() {
    fpsview::release_on_panic();
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
            // a picture of itself (BURROW_SHOT): above the others, or a covered window is never drawn
            kind: if std::env::var_os("BURROW_SHOT").is_some() { gpui::WindowKind::PopUp } else { gpui::WindowKind::Normal },
            app_id: Some("burrow".into()),
            ..Default::default()
        };
        cx.open_window(opts, |window, cx| cx.new(|cx| burrow::Burrow::new(window, cx))).expect("a window");
        // BURROW_SHOT=<png>: a picture of the window after a few seconds, then quit (see shot.rs)
        if let Ok(path) = std::env::var("BURROW_SHOT") {
            let wait: u64 = std::env::var("BURROW_SHOT_AFTER").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
            cx.spawn(async move |cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(wait)).await;
                let _ = cx.update(|cx| {
                    if let Some(w) = cx.windows().first().copied() {
                        let _ = w.update(cx, |_, window, _| eprintln!("shot: {}", shot::shoot(window, &path)));
                    }
                    cx.quit();
                });
            })
            .detach();
        }
        cx.activate(focus);
    });
}
