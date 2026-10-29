//! LIMITS (#vLimits, `fillLimits`): what Claude Code's /usage says, a bar a limit (the coffee pot is the first,
//! the cooler's bottle the second), and the rest of what it said.

use crate::burrow::Burrow;
use gpui::prelude::*;
use crate::kit::{self, col};
use burrow_model::Limit;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px, rgb};

/// "Oct 30 at 3pm" as the bar says it: "new Oct 30 3pm"
pub fn resets(r: &str) -> String {
    match r.rfind(" at ") {
        Some(i) => format!("{} {}", &r[..i], &r[i + 4..]),
        None => r.to_string(),
    }
}

pub fn bars(u: &burrow_model::Usage) -> Vec<Limit> {
    if !u.bars.is_empty() {
        return u.bars.clone();
    }
    let named = |l: &Option<Limit>, name: &str| l.clone().map(|mut l| {
        l.label = name.into();
        l
    });
    [named(&u.session, "session"), named(&u.week, "week (all models)")].into_iter().flatten().collect()
}

pub fn body(b: &Burrow) -> AnyElement {
    let u = &b.state.usage;
    let bars = bars(u);
    let mut d = div().flex().flex_col();
    if bars.is_empty() {
        d = d.child(kit::pixel_para("Not known yet: Claude Code's /usage is asked every five minutes.", col::WHITE));
    }
    for l in &bars {
        let num = format!("{}% left{}", 100 - l.used, if l.resets.is_empty() { String::new() } else { format!(" · new {}", resets(&l.resets)) });
        d = d.child(div().mt(px(4.)).mb(px(12.)).child(kit::pixel_text(l.label.clone(), col::GOLD2, true)).child(kit::bar(l.used as f32)).child(kit::pixel_text(num, col::STEEL3, true)));
    }
    if !u.text.is_empty() {
        d = d.child(
            div()
                .id("usage")
                .mt(px(12.))
                .px(px(10.))
                .py(px(8.))
                .bg(rgb(col::INK))
                .border_2()
                .border_color(rgb(col::INK))
                .max_h(px(300.))
                .overflow_y_scroll()
                .font_family(kit::MONO)
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(rgb(col::STEEL3))
                .child(u.text.clone()),
        );
    }
    d.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reset_said_shorter() {
        assert_eq!(resets("Oct 30 at 3pm (Europe/Brussels)"), "Oct 30 3pm (Europe/Brussels)");
        assert_eq!(resets("3pm"), "3pm");
        let u = burrow_model::Usage { week: Some(Limit { used: 20, ..Default::default() }), ..Default::default() };
        assert_eq!(bars(&u)[0].label, "week (all models)");
    }
}
