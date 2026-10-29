//! OFFICE MANAGER (#vManager, `askManager`): who sits at the reception just let go from the closet. A card each in
//! her own colour: her figure on a swatch of it, her name, what she does; one at a reception already is greyed.

use super::{call, note};
use crate::burrow::Burrow;
use crate::kit::{self, Frame, col, frame_bg, mix};
use burrow_room::figure::{Arms, Legs, body as figure};
use gpui::{AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, rgb};

pub fn opened(b: &mut Burrow) {
    b.views.p.styles = b.hub.manager_styles();
}

fn hire(b: &mut Burrow, key: String, window: &mut Window, cx: &mut Context<Burrow>) {
    let at = b.views.p.manager_at.take().map(|a| [a[0] as f64, a[1] as f64]);
    let now = b.now_ms();
    b.room.pend(None, None, "manager", now);
    call(b, move |hub| hub.hire(&key, at), |_, r, _, _| if let Err(e) = r { note(&e) }, window, cx);
    b.close_view(window, cx);
}

pub fn body(b: &mut Burrow, window: &mut Window, cx: &mut Context<Burrow>) -> AnyElement {
    let mut cards = div().flex().flex_col().gap(px(10.));
    for (i, st) in b.views.p.styles.clone().into_iter().enumerate() {
        let c = burrow_room::avatar::color_of(&st.color).unwrap_or(pix::Color::rgb(0x9a74d6));
        let cv = kit::hex(c);
        let (seed, color) = (st.seed.clone(), c);
        let (im, w, h) = kit::pic(&format!("mgr:{}:{}", st.seed, st.color), 3, window, || (*figure(&seed, color, Arms::Down, Legs::Stand, false, true)).clone());
        let cream = 0xfff6e2;
        let (bg, hot) = (mix(cream, cv, 0.28), mix(cream, cv, 0.45));
        let (tl, br) = (mix(0xffffff, cv, 0.08), mix(col::BOX, cv, 0.75));
        let card = |fill: u32| Frame { border: Some(col::INK), rings: vec![(tl, br)], fill: kit::Fill::Solid(fill), band: None, pad: 0, pad_fill: None, clip: false, glow: None };
        let key = st.key.clone();
        let who = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(3.))
            .flex_1()
            .min_w(px(0.))
            .child(kit::pixel_text(st.name.to_uppercase(), cv, true).text_size(px(22.)).line_height(px(26.)))
            .child(div().font_family(kit::PIXEL).text_size(px(16.)).line_height(px(22.)).text_color(rgb(col::INK)).child(st.line.clone()))
            .child(div().font_family(kit::PIXEL).text_size(px(13.)).text_color(rgb(mix(0x2a2230, cv, 0.55))).child(if st.here { "at a reception already".to_string() } else { st.how.clone() }));
        cards = cards.child(
            div()
                .id(("style", i))
                .group("card")
                .relative()
                .flex()
                .gap(px(16.))
                .items_center()
                .px(px(14.))
                .py(px(10.))
                .when(st.here, |d| d.opacity(0.6))
                .when(!st.here, |d| d.cursor_pointer().on_click(cx.listener(move |b, _, window, cx| hire(b, key.clone(), window, cx))))
                .child(div().absolute().size_full().child(frame_bg(card(bg))))
                .when(!st.here, |d| d.child(div().absolute().size_full().invisible().group_hover("card", |s| s.visible()).child(frame_bg(card(hot)))))
                .child(div().relative().flex_none().px(px(8.)).py(px(4.)).bg(rgb(mix(0xffffff, cv, 0.65))).border_2().border_color(rgb(col::INK)).child(kit::image(im, w, h)))
                .child(who),
        );
    }
    div()
        .flex()
        .flex_col()
        .child(div().mb(px(10.)).child(kit::pixel_para("Who sits at the reception? Pick one; burn her booth to let her go.", col::WHITE)))
        .child(cards)
        .into_any_element()
}
