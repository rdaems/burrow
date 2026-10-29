//! SETTINGS (#vSettings, `fillSettings`): burrow's own settings, a slider and a name a line; what one does is in
//! the box under them while the pointer is on it, as a game's menu explains the line its cursor is on: the name
//! lit gold, the text kept there until another line is pointed at. Kept in room.json `settings`.

use crate::burrow::Burrow;
use crate::kit::{self, Run, col};
use gpui::{AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, px, rgb};

/// the switches: key, name, what it does (a flag as code)
pub const OPTIONS: [(&str, &str, &[(&str, Run)]); 3] = [
    ("rc", "Remote Control", &[("New bots start with ", Run::Plain), ("--remote-control", Run::Code), (": also in the Claude app and at claude.ai/code.", Run::Plain)]),
    ("yolo", "YOLO mode", &[("New bots start with ", Run::Plain), ("--dangerously-skip-permissions", Run::Code), (": bypass permissions, no prompts.", Run::Plain)]),
    ("hardcore", "Hardcore", &[("Not for the faint of heart.", Run::Plain)]),
];

fn on(b: &Burrow, key: &str) -> bool {
    let s = &b.state.settings;
    match key {
        "rc" => s.rc,
        "yolo" => s.yolo,
        "hardcore" => s.hardcore,
        k => s.other.get(k).and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

pub fn body(b: &mut Burrow, cx: &mut Context<Burrow>) -> AnyElement {
    let mut d = div().flex().flex_col();
    for (i, (key, name, _)) in OPTIONS.iter().enumerate() {
        let v = on(b, key);
        let cur = b.views.p.help == Some(i);
        d = d.child(
            div()
                .id(("opt", i))
                .flex()
                .gap(px(14.))
                .items_center()
                .py(px(8.))
                .cursor_pointer()
                .on_hover(cx.listener(move |b, h: &bool, _, cx| {
                    if *h {
                        b.views.p.help = Some(i);
                        cx.notify();
                    }
                }))
                // it slides at once; the push that follows agrees
                .on_click(cx.listener(move |b, _, _, cx| {
                    let v = !on(b, key);
                    b.hub.settings(&[(key, v)]);
                    cx.notify();
                }))
                .child(kit::slider(("slide", i), v))
                .child(kit::pixel_text(*name, if cur { col::GOLD2 } else { col::WHITE }, true)),
        );
    }
    let help = b.views.p.help.map(|i| kit::prose(OPTIONS[i].2, col::STEEL3));
    d.child(div().w(px(440.)).max_w_full().min_h(px(56.)).mt(px(10.)).pt(px(10.)).border_t_2().border_color(rgb(col::STEEL0)).children(help)).into_any_element()
}
