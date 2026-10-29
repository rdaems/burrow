//! The printer's report (#vReport, `fillReport`): no panel round it. A wide-carriage printer as wide as the paper
//! stands across the pane's bottom edge, its top rows showing; the fanfold sheet rises out of its slot, row by
//! row, every row letter by letter, as a line printer prints. The text is report_text's.

use super::report_text::{self, Stats};
use crate::burrow::Burrow;
use crate::kit::{self, col};
use gpui::{AnyElement, App, Bounds, Context, IntoElement, ParentElement, Pixels, SharedString, Styled, TextRun, Window, canvas, div, fill, point, px, rgb, size};
use std::time::Instant;

/// pixels a dot of the printer; its rows above the pane's edge
const DOT: f32 = 8.;
const SHOW: f32 = 12.;
const PAPER: u32 = 0xf6f1e3;

pub struct Printing {
    pub lines: Vec<String>,
    pub started: Instant,
    pub paper: f32,
}

pub fn opened(b: &mut Burrow) {
    b.views.p.report = None;
}

/// the paper's width for a pane this wide: the view's (640 at most, 48 off the pane's edges) less 48
fn paper_for(w: f32) -> f32 {
    let view = if w <= 0. { 640. } else { (w - 48.).min(640.) };
    (view - 48.).clamp(320., 520.).floor()
}

fn print(b: &mut Burrow, paper: f32, window: &mut Window) -> Printing {
    // the pixel font is proportional: lines are measured in it
    let ts = window.text_system().clone();
    let measure = move |s: &str| -> f32 {
        if s.is_empty() {
            return 0.;
        }
        let run = TextRun { len: s.len(), font: gpui::font(kit::PIXEL), color: kit::hsla(col::INK), background_color: None, underline: None, strikethrough: None };
        f32::from(ts.shape_line(SharedString::from(s.to_string()), px(16.), &[run], None).width)
    };
    let art = b.room.w.s.art.as_ref().map(|a| (burrow_room::paintings::list()[a.i].title, a.n as f64 / a.order.len().max(1) as f64));
    let shift = (b.now_ms() - b.room.born) / 60000.;
    let st = Stats { bots: &b.room.w.s.stats, meetings: b.room.w.s.meetings, art, shift };
    let (d, mo, h, mi) = today();
    // the sheet's text, inside its 40 px margins and its border
    let wide = (paper - 4. - 80.).max(240.);
    let lines = report_text::lines(&b.state, &st, (d, mo, h, mi), &measure, wide);
    Printing { lines, started: Instant::now(), paper }
}

/// this computer's date and time: day, month, hours, minutes
fn today() -> (u32, u32, u32, u32) {
    let (h, m) = crate::burrow::local_time();
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) + offset();
    let (_, mo, d) = civil(days.div_euclid(86400));
    (d, mo, h, m)
}

/// the local time's offset from UTC, seconds (the hours local_time gives against UTC's)
fn offset() -> i64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (h, m) = crate::burrow::local_time();
    let utc = now.rem_euclid(86400);
    let mut off = (h as i64 * 3600 + m as i64 * 60) - (utc - utc % 60);
    if off > 14 * 3600 {
        off -= 86400;
    } else if off < -12 * 3600 {
        off += 86400;
    }
    off
}

/// days since 1970 as (year, month, day) (Howard Hinnant's civil_from_days)
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// the fanfold's strips: sprocket holes down each side, a dashed perforation inside them
fn strips(b: Bounds<Pixels>, window: &mut Window) {
    let (x0, y0, w, h) = (f32::from(b.origin.x), f32::from(b.origin.y), f32::from(b.size.width), f32::from(b.size.height));
    let q = |x: f32, y: f32, ww: f32, hh: f32, c: u32, window: &mut Window| window.paint_quad(fill(Bounds::new(point(px(x), px(y)), size(px(ww), px(hh))), kit::hsla(c)));
    for side in [x0 + 7., x0 + w - 22. + 7.] {
        let mut y = 8.;
        while y < h {
            q(side, y0 + y, 8., 8.0f32.min(h - y), col::INK, window);
            y += 24.;
        }
    }
    for x in [x0 + 22., x0 + w - 24.] {
        let mut y = 0.;
        while y < h {
            q(x, y0 + y, 2., 6.0f32.min(h - y), 0xcfc4aa, window);
            y += 10.;
        }
    }
}

pub fn body(b: &mut Burrow, window: &mut Window, _cx: &mut Context<Burrow>) -> AnyElement {
    let paper = paper_for(b.views.p.width.get());
    if b.views.p.report.as_ref().is_none_or(|r| r.paper != paper) {
        let pr = print(b, paper, window);
        b.views.p.report = Some(pr);
    }
    let r = b.views.p.report.as_ref().unwrap();
    let ms = r.started.elapsed().as_secs_f64() * 1000.;
    let at = report_text::printed(&r.lines, ms);
    if at.0 < r.lines.len() {
        window.request_animation_frame();
    }
    let text = report_text::sheet(&r.lines, at);
    let rows = text.split('\n').count();
    let height = if text.is_empty() { 0. } else { 14. + rows as f32 * 22. + 24. };
    let (im, w, h) = kit::pic(&format!("printer:{paper}"), 1, window, || burrow_room::draw::printer_picture(paper as f64, DOT as u32).0);
    let printer_rows = h / DOT;
    let sheet = div()
        .relative()
        .w(px(paper))
        .h(px(height))
        .overflow_hidden()
        .bg(rgb(PAPER))
        .border_2()
        .border_color(rgb(col::INK))
        .child(canvas(|_, _, _| {}, |bd, _, window: &mut Window, _: &mut App| strips(bd, window)).absolute().size_full())
        .child(div().absolute().right_0().top_0().bottom_0().w(px(2.)).bg(rgb(col::STEEL2)))
        .child(div().absolute().left_0().right_0().bottom_0().h(px(2.)).bg(rgb(col::STEEL2)))
        .child(
            div()
                .relative()
                .pt(px(14.))
                .px(px(38.))
                .font_family(kit::PIXEL)
                .text_size(px(16.))
                .line_height(px(22.))
                .whitespace_nowrap()
                .text_color(rgb(col::INK))
                .children(text.split('\n').map(|l| div().h(px(22.)).child(l.to_string()))),
        );
    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .child(div().absolute().left_0().right_0().bottom(px((SHOW - 3.) * DOT)).flex().justify_center().child(sheet))
        .child(div().absolute().left_0().right_0().bottom(px(-(printer_rows - SHOW) * DOT)).flex().justify_center().child(kit::image(im, w, h)))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paper_fits_the_pane() {
        assert_eq!(paper_for(1200.), 520.);
        assert_eq!(paper_for(500.), 404.);
        assert_eq!(paper_for(300.), 320.);
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_755), (2026, 10, 29));
    }
}
