// the view in gpui's test platform (no window on the screen): it lays out, resizes, paints, and keys
// and the mouse reach the program

use std::cell::RefCell;
use std::rc::Rc;

use burrow_term::{TermEvent, TermView};
use gpui::{Focusable, Modifiers, TestAppContext, point, px, size};

#[test]
fn lays_out_types_and_clicks() {
    let mut cx = TestAppContext::single();
    let sent = Rc::new(RefCell::new(Vec::<u8>::new()));
    let s = sent.clone();
    let (view, vcx) = cx.add_window_view(move |window, cx| TermView::new(move |b: &[u8]| s.borrow_mut().extend_from_slice(b), window, cx));
    let events = Rc::new(RefCell::new(Vec::new()));
    let e = events.clone();
    vcx.update(|_, cx| cx.subscribe(&view, move |_, ev: &TermEvent, _| e.borrow_mut().push(ev.clone())).detach());
    vcx.simulate_resize(size(px(800.), px(600.)));
    vcx.run_until_parked();
    let (cols, rows) = vcx.update(|_, cx| view.read(cx).size());
    assert!(cols > 20 && rows > 10, "{cols}x{rows}");
    assert!(events.borrow().iter().any(|e| matches!(e, TermEvent::Resize { .. })));

    // a screenful with every kind of cell the painter knows
    vcx.update(|_, cx| {
        view.update(cx, |v, cx| {
            v.feed("╭─╮ \x1b[1;31mbold\x1b[0m \x1b[4mhttps://example.com\x1b[0m 日本 é ▛█▜ ░\r\n╰─╯\x1b[?2004h".as_bytes(), cx)
        })
    });
    vcx.run_until_parked();
    assert!(vcx.update(|_, cx| view.read(cx).screen().row_text(0).starts_with("╭─╮ bold")));

    vcx.update(|window, cx| window.focus(&view.focus_handle(cx)));
    vcx.run_until_parked();
    sent.borrow_mut().clear();
    vcx.simulate_keystrokes("enter shift-enter up ctrl-c");
    vcx.run_until_parked();
    assert_eq!(&*sent.borrow(), b"\r\x1b\r\x1b[A\x03");
    assert!(events.borrow().contains(&TermEvent::Typed { enter: true }));

    // a click, with the program asking for the mouse
    vcx.update(|_, cx| view.update(cx, |v, cx| v.feed(b"\x1b[?1000h\x1b[?1006h", cx)));
    vcx.run_until_parked();
    sent.borrow_mut().clear();
    vcx.simulate_click(point(px(30.), px(30.)), Modifiers::default());
    vcx.run_until_parked();
    let got = String::from_utf8(sent.borrow().clone()).unwrap();
    assert!(got.starts_with("\x1b[<0;") && got.ends_with('m'), "{got:?}");
}
