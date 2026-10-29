//! Any kind of furniture drawn at rest (the closet's pictures, the ghost of a piece carried in, what is
//! tossed into the fire), and the pictures for the page: the closet's icons, the printer in the pane.

use super::*;

/// what a piece at rest still takes from the world: this computer's time (the clock, the window) and the
/// paint a new door would get (the next machine's)
#[derive(Clone, Copy, Debug)]
pub struct KindEnv {
    pub clock: Clock,
    pub door: Paint,
}
impl Default for KindEnv {
    fn default() -> KindEnv {
        KindEnv { clock: Clock::default(), door: PAINT[0] }
    }
}

/// a kind of furniture at a spot, at rest (`x`, `y` as the layout keeps it: its anchor), `t` the room's time
pub fn draw_kind(ctx: &mut Ctx, kind: &str, x: i32, y: i32, t: f64, env: &KindEnv) {
    match kind {
        "plant" => draw_plant(ctx, x, y, 0, None),
        "armchair" => draw_armchair(ctx, x, y),
        "coffee" => draw_coffee(ctx, x, y, false, t, None),
        "cooler" => draw_cooler(ctx, x, y, None),
        "printer" => draw_printer(ctx, x, y, None),
        "chess" => {
            draw_chess(ctx, x, y, None);
            for d in [-13, 13] {
                draw_stool(ctx, x + d, y);
            }
        }
        "pong" => {
            draw_rug(ctx, x, y);
            draw_pong(ctx, x, y, None);
        }
        "meet" => {
            draw_table(ctx, x, y);
            for d in [-23, 23] {
                draw_stool(ctx, x + d, y);
            }
        }
        "board" => draw_board(ctx, x, y, &[]),
        "window" => draw_window(ctx, x, y, env.clock.h),
        "clock" => draw_clock(ctx, x, y, None, env.clock, false),
        "fire" => draw_fire(ctx, x, y - FIRE.h, t, false, -1., &[], FirePart::All),
        "closet" => draw_closet(ctx, x, y, false),
        "settings" => draw_switchboard(ctx, x, y, false),
        "manager" => {
            let o = Reception { n: 0, name: "", color: Color::rgb(0x9a74d6), screen: Some(RecScreen::On), t: 0. };
            draw_reception(ctx, x, y, &o, false);
            draw_reception(ctx, x, y, &o, true);
        }
        "pair" => draw_pair(ctx, x, y, true, t, false),
        "add" => draw_add(ctx, x, y - 15, false),
        "booth" => {
            draw_booth(ctx, x, y, false, -1., -1., false, t, false);
            draw_booth(ctx, x, y, true, -1., -1., false, t, false);
        }
        "neuralyzer" => draw_neuralyzer(ctx, x, y, false),
        "radio" => draw_radio(ctx, x, y, true),
        "doghouse" => draw_doghouse(ctx, x, y),
        "door" => draw_door(ctx, x, &env.door, false, false, y), // in the paint the next machine gets
        _ => {}
    }
}

/// a picture of a kind of furniture, for the closet: `s` pixels a dot, `pad` dots round its footprint
pub fn icon(kind: &str, s: u32, pad: i32, env: &KindEnv) -> Image {
    let r = foot(kind, 0., 0., None);
    let (pad, sf) = (pad as f64, s as f64);
    let mut im = Image::new(((r[2] - r[0] + 2. * pad) * sf) as u32, ((r[3] - r[1] + 2. * pad) * sf) as u32);
    {
        let mut ctx = im.ctx();
        ctx.set_transform(sf, 0., 0., sf, (pad - r[0]) * sf, (pad - r[1]) * sf);
        draw_kind(&mut ctx, kind, 0, 0, 0., env);
    }
    im
}

/// the printer the report comes out of, in the pane: a wide-carriage line printer drawn to the paper's
/// width (`paper` pixels) plus three dots a side, `s` pixels a dot: its slot (row 1, dark, one dot wider
/// than the paper each side) is where the sheet rises from; the body, a front opening, a lamp, a stand.
/// Its picture, its rows, its width in dots
pub fn printer_picture(paper: f64, s: u32) -> (Image, usize, usize) {
    let w = (paper / s as f64).ceil() as usize + 6;
    let f = |a: &str, mid: &str, b: &str| format!("{a}{}{b}", mid.repeat(w - a.len() - b.len()));
    let rows = [
        f(".", "F", "."),
        f("FB", "x", "BF"),
        f("F", "B", "F"),
        f("F", "C", "F"),
        f("F", "C", "gCF"),
        f("F", "C", "F"),
        f("FC", "x", "CF"),
        f("F", "C", "F"),
        f("F", "D", "F"),
        f(".", "F", "."),
        f(".", "d", "."),
        f("d", "h", "d"),
        f("k", "d", "k"),
        f("k", "d", "k"),
        f("k", "k", "k"),
        f(".", "s", "."),
    ];
    let mut im = Image::new((w as u32) * s, rows.len() as u32 * s);
    {
        let mut x = im.ctx();
        for (j, row) in rows.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                let c = sprites::pal(ch);
                if !c.is_none() {
                    x.rect(i as i32 * s as i32, j as i32 * s as i32, s as i32, s as i32, c);
                }
            }
        }
    }
    (im, rows.len(), w)
}
