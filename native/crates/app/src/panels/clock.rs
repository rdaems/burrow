//! CLOCK (#vClock, `fillClock`): this computer kept awake (caffeinate): while bots work, for a while, or not. The
//! hub runs the inhibitor; the buttons show the choice, the clock's face and tip what it does.

use super::{call, note, row};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, prelude::*};

/// the choices: mode, label, minutes
pub const AWAKE: [(&str, &str, i64); 5] = [("off", "off", 0), ("work", "when local bots work", 0), ("for", "1 h", 60), ("for", "3 h", 180), ("for", "8 h", 480)];

/// which choice is lit: the mode, and for a while the shortest duration that still covers what is left
pub fn lit(mode: &str, until: i64, now: i64) -> [bool; 5] {
    let left = if mode == "for" { (until - now).max(0) } else { 0 };
    let chosen = AWAKE.iter().find(|(m, _, n)| *m == "for" && n * 60000 >= left).map(|x| x.2);
    AWAKE.map(|(m, _, n)| mode == m && (m != "for" || chosen == Some(n)))
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut gpui::Context<Burrow>) -> AnyElement {
    let _ = window;
    let a = &b.state.awake;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let on = lit(if a.mode.is_empty() { "off" } else { &a.mode }, a.until, now);
    let mut r = row();
    for (i, (mode, label, min)) in AWAKE.iter().enumerate() {
        r = r.child(kit::button(("awake", i), *label, on[i], false).on_click(cx.listener(move |b: &mut Burrow, _, window, cx: &mut Context<Burrow>| {
            call(b, move |hub| hub.awake(mode, *min), |_, r, _, _| if let Err(e) = r { note(&e) }, window, cx);
        })));
    }
    div().flex().flex_col().child(div().mb(gpui::px(4.)).child(kit::pixel_text("Keep this computer awake", col::WHITE, true))).child(r).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_that_holds() {
        assert_eq!(lit("off", 0, 0), [true, false, false, false, false]);
        assert_eq!(lit("work", 0, 0), [false, true, false, false, false]);
        assert_eq!(lit("for", 100 * 60000, 0), [false, false, false, true, false], "100 min left: the 3 h");
        assert_eq!(lit("for", 50 * 60000, 0), [false, false, true, false, false]);
        assert_eq!(lit("for", 0, 99), [false, false, true, false, false], "run out: the shortest");
    }
}
