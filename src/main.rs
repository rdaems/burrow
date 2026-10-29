// burrow: a native window around the room. The room is web/, a page burrow's server serves on localhost; this is the
// window it lives in: the system's own webview (WKWebView on a Mac), a dock icon of its own, the keys a browser keeps
// for itself (⌘W, ⌘T, ⌘N, …) reaching the page, links opening in your browser, and the server started when none runs.
// The server is started detached, so it outlives the window as your sessions and terminals do, from your login shell
// (its PATH has python3); burrow ▸ Restart Server restarts it (attached terminals close, the sessions go on).
//
//   cargo run                  the window (BURROW_PORT: another port; BURROW_WEB: another web/ folder)
//   BURROW_SHOT=<png> burrow   for tests: the room's canvas written there after a few seconds, the page's state printed
//                              (and BURROW_TEST_JS evaluated, its result printed)
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Theme, Window, WindowBuilder};
use wry::{NewWindowResponse, WebView, WebViewBuilder};

const DARK: (u8, u8, u8, u8) = (11, 8, 18, 255);   // the page's night (burrow.css --night): the title bar is this too
const BLANK: &str = r#"<!doctype html><body style="margin:0;background:#0b0812;color:#8b93ad;font:13px/1.5 ui-monospace,monospace;display:grid;place-items:center;height:100vh"><p id=m></p></body>"#;

enum Ev {
    Ready,
    Failed(String),
    Menu(muda::MenuId),
    Shoot,
    Shot(String),
}

fn port() -> u16 {
    std::env::var("BURROW_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(7777)
}

fn listening(port: u16) -> bool {
    TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(300)).is_ok()
}

fn wait_for(port: u16, up: bool, secs: u64) -> bool {
    let t = Instant::now();
    while listening(port) != up {
        if t.elapsed() > Duration::from_secs(secs) {
            return false;
        }
        thread::sleep(Duration::from_millis(150));
    }
    true
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// where the room's files are: $BURROW_WEB, the checkout this was built from, or the copy inside the app
fn web_dir() -> Option<PathBuf> {
    let mut at = Vec::new();
    if let Some(p) = std::env::var_os("BURROW_WEB") {
        at.push(PathBuf::from(p));
    }
    at.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/web")));
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("../Resources/web"))) {
        at.push(dir);
    }
    at.into_iter().find(|d| d.join("server.py").is_file())
}

fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// start burrow's server, detached, from your login shell; what it says goes to ~/.cache/burrow/server.log
fn start_server(port: u16) -> Result<(), String> {
    let web = web_dir().ok_or("burrow's web folder (server.py) is not here: set BURROW_WEB")?;
    let log = home().join(".cache/burrow/server.log");
    let _ = fs::create_dir_all(home().join(".cache/burrow"));
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let script = format!(
        "cd {} && BURROW_PORT={port} nohup python3 server.py >> {} 2>&1 &",
        quoted(&web.to_string_lossy()),
        quoted(&log.to_string_lossy())
    );
    Command::new(shell)
        .args(["-lc", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// the server that listens on the port, if it is burrow's (server.py), stopped; then started again
fn restart_server(port: u16) -> Result<(), String> {
    let out = Command::new("lsof").args(["-nP", "-t", &format!("-iTCP:{port}"), "-sTCP:LISTEN"]).output().map_err(|e| e.to_string())?;
    for pid in String::from_utf8_lossy(&out.stdout).split_whitespace() {
        let cmd = Command::new("ps").args(["-o", "command=", "-p", pid]).output().map_err(|e| e.to_string())?;
        if String::from_utf8_lossy(&cmd.stdout).contains("server.py") {
            let _ = Command::new("kill").arg(pid).status();
        }
    }
    if !wait_for(port, false, 8) {
        return Err(format!("something else holds port {port}"));
    }
    start_server(port)
}

/// a link from the page (window.open, a target="_blank"): in your browser, http(s) only
fn open_external(url: &str) {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return;
    }
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).status();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = Command::new("xdg-open").arg(url).status();
}

// where the window was, kept in ~/.config/burrow/window as "x y width height" (logical pixels)
fn window_file() -> PathBuf {
    home().join(".config/burrow/window")
}

fn saved_window() -> Option<(f64, f64, f64, f64)> {
    let s = fs::read_to_string(window_file()).ok()?;
    let v: Vec<f64> = s.split_whitespace().filter_map(|n| n.parse().ok()).collect();
    (v.len() == 4 && v[2] >= 480.0 && v[3] >= 320.0).then(|| (v[0], v[1], v[2], v[3]))
}

fn save_window(w: &Window) {
    let k = w.scale_factor();
    if let (Ok(p), s) = (w.outer_position(), w.inner_size()) {
        let _ = fs::create_dir_all(home().join(".config/burrow"));
        let _ = fs::write(window_file(), format!("{} {} {} {}\n", p.x as f64 / k, p.y as f64 / k, s.width as f64 / k, s.height as f64 / k));
    }
}

/// the menus: burrow's own (Restart Server among the usual), Edit (copy and paste reach the page), View, Window. ⌘W is
/// in none of them: it goes to the page, which closes the focused tile with it
struct Menus {
    restart: muda::MenuId,
    reload: muda::MenuId,
    _menu: muda::Menu,
}

fn menus() -> Menus {
    use muda::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem as P, Submenu};
    let key = |s: &str| muda::accelerator::Accelerator::from_str(s).ok();
    let restart = MenuItem::new("Restart Server", true, key("CmdOrCtrl+Shift+R"));
    let reload = MenuItem::new("Reload", true, key("CmdOrCtrl+R"));
    let about = AboutMetadata { name: Some("burrow".into()), version: Some(env!("CARGO_PKG_VERSION").into()), ..Default::default() };
    let menu = Menu::new();
    let app = Submenu::with_items("burrow", true, &[
        &P::about(None, Some(about)), &P::separator(), &restart, &P::separator(),
        &P::hide(None), &P::hide_others(None), &P::show_all(None), &P::separator(), &P::quit(None),
    ]);
    let edit = Submenu::with_items("Edit", true, &[&P::undo(None), &P::redo(None), &P::separator(), &P::cut(None), &P::copy(None), &P::paste(None), &P::select_all(None)]);
    let view = Submenu::with_items("View", true, &[&reload, &P::fullscreen(None)]);
    let window = Submenu::with_items("Window", true, &[&P::minimize(None)]);
    for m in [app, edit, view, window].into_iter().flatten() {
        let _ = menu.append(&m);
    }
    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();
    Menus { restart: restart.id().clone(), reload: reload.id().clone(), _menu: menu }
}

fn say(webview: &WebView, text: &str) {
    let _ = webview.evaluate_script(&format!("document.getElementById('m').textContent = {:?}", text));
}

// for tests: the page's state, and the room's canvas as a PNG (decoded here from the data URL the page gives)
const CHECKS: &str = "JSON.stringify({ title: document.title, state: typeof state !== 'undefined' && !!state, bots: typeof state !== 'undefined' && state ? state.bots.length : null, ipc: !!window.ipc, audio: !!(window.AudioContext || window.webkitAudioContext), pointerLock: 'requestPointerLock' in Element.prototype, size: [innerWidth, innerHeight] })";
fn base64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut n) = (Vec::new(), 0u32, 0);
    for v in s.bytes().filter_map(val) {
        acc = acc << 6 | v as u32;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((acc >> n) as u8);
        }
    }
    out
}

fn main() -> wry::Result<()> {
    let port = port();
    let url = format!("http://localhost:{port}");
    let shot = std::env::var("BURROW_SHOT").ok();
    let event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    let menus = menus();

    let saved = saved_window();
    let mut wb = WindowBuilder::new()
        .with_title("burrow")
        .with_inner_size(saved.map(|s| LogicalSize::new(s.2, s.3)).unwrap_or(LogicalSize::new(1440.0, 900.0)))
        .with_min_inner_size(LogicalSize::new(720.0, 480.0))
        .with_theme(Some(Theme::Dark))
        .with_background_color(DARK);
    if let Some(s) = saved {
        wb = wb.with_position(LogicalPosition::new(s.0, s.1));
    }
    // no title bar to be seen: it takes the window's colour, the page's night, and shows no title; the traffic lights
    // stay, and dragging it or a double click on it work as on any window
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::WindowBuilderExtMacOS;
        wb = wb.with_titlebar_transparent(true).with_title_hidden(true);
    }
    let window = wb.build(&event_loop).expect("a window");

    let page = url.clone();
    let builder = WebViewBuilder::new()
        .with_html(BLANK)
        .with_background_color(DARK)
        .with_devtools(cfg!(debug_assertions))
        .with_accept_first_mouse(true)
        .with_new_window_req_handler(|url, _| {
            open_external(&url);
            NewWindowResponse::Deny
        })
        // what the page asks of the window: a copy out of a terminal (an OSC 52 from another machine), which a
        // webview won't put on the clipboard without a click
        .with_ipc_handler(move |req| {
            if !req.uri().to_string().starts_with(&page) {
                return;
            }
            if let Some(text) = req.body().strip_prefix("clip:") {
                if let Ok(mut c) = arboard::Clipboard::new() {
                    let _ = c.set_text(text);
                }
            }
        });
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window)?;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().unwrap())?
    };

    // the server: yours, running already, or started now
    let proxy = event_loop.create_proxy();
    thread::spawn(move || {
        let ok = listening(port) || (start_server(port).is_ok() && wait_for(port, true, 25));
        let _ = proxy.send_event(if ok { Ev::Ready } else { Ev::Failed(format!("burrow's server did not start on port {port}: see ~/.cache/burrow/server.log")) });
    });
    let proxy = event_loop.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |e: muda::MenuEvent| {
        let _ = proxy.send_event(Ev::Menu(e.id().clone()));
    }));
    let proxy = event_loop.create_proxy();

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Ev::Ready) => {
                let _ = webview.load_url(&url);
                if shot.is_some() {
                    let proxy = proxy.clone();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_secs(6));
                        let _ = proxy.send_event(Ev::Shoot);
                    });
                }
            }
            Event::UserEvent(Ev::Failed(why)) => say(&webview, &why),
            Event::UserEvent(Ev::Menu(id)) if id == menus.restart => {
                let _ = webview.load_html(BLANK);
                let proxy = proxy.clone();
                thread::spawn(move || {
                    let r = restart_server(port).map(|_| wait_for(port, true, 25));
                    let _ = proxy.send_event(match r {
                        Ok(true) => Ev::Ready,
                        Ok(false) => Ev::Failed(format!("burrow's server did not come back on port {port}: see ~/.cache/burrow/server.log")),
                        Err(e) => Ev::Failed(e),
                    });
                });
            }
            Event::UserEvent(Ev::Menu(id)) if id == menus.reload => {
                let _ = webview.reload();
            }
            Event::UserEvent(Ev::Shoot) => {
                let _ = webview.evaluate_script_with_callback(CHECKS, |r| println!("page: {r}"));
                if let Ok(js) = std::env::var("BURROW_TEST_JS") {
                    let _ = webview.evaluate_script_with_callback(&js, |r| println!("test: {r}"));
                }
                let proxy = proxy.clone();
                let _ = webview.evaluate_script_with_callback("document.querySelector('#room').toDataURL()", move |r| {
                    let _ = proxy.send_event(Ev::Shot(r));
                });
            }
            Event::UserEvent(Ev::Shot(data)) => {
                let b64 = data.trim_matches('"').replace("\\/", "/");
                let png = base64(b64.split(',').nth(1).unwrap_or(""));
                if let Some(path) = &shot {
                    let _ = fs::write(path, png);
                }
                *flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                save_window(&window);
                *flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
