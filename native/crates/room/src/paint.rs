//! The bridge from the room's logic to the painters (`draw`): what the logic needs of the pictures.

use crate::draw as D;
use pix::{Color, Image};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static TOPS: RefCell<HashMap<String, i32>> = RefCell::new(HashMap::new());
}

/// how far above its anchor a kind of piece is drawn (its topmost opaque row), measured once from its picture
pub fn vis_top(kind: &str) -> i32 {
    if let Some(t) = TOPS.with(|t| t.borrow().get(kind).copied()) {
        return t;
    }
    let r = D::foot(kind, 0., 0., None);
    let (w, h) = ((r[2] - r[0]) as i32 + 8, (r[3] - r[1]) as i32 + 8);
    let mut im = Image::new(w.max(1) as u32, h.max(1) as u32);
    {
        let mut x = im.ctx();
        x.translate(4. - r[0], 4. - r[1]);
        let env = D::KindEnv { clock: D::Clock { h: 12, m: 0, s: 0 }, door: D::WOOD };
        D::draw_kind(&mut x, kind, 0, 0, 0., &env);
    }
    let mut top = r[1] as i32;
    'rows: for j in 0..h {
        for i in 0..w {
            if im.get(i, j)[3] > 128 {
                top = j - 4 + r[1] as i32;
                break 'rows;
            }
        }
    }
    TOPS.with(|t| t.borrow_mut().insert(kind.to_string(), top));
    top
}


pub fn tinted(im: &Image, col: Color, a: f64) -> Image {
    D::tinted(im, col, a)
}

pub fn dissolve(im: &Image, d: f64) -> Image {
    D::dissolve(im, d)
}

/// a picture of a piece (or a desk, or a door) as it stands, two dots round it, for the fire
pub fn snapshot_piece(room: &crate::room::Room, key: &str, at: [i32; 2], u: Option<i32>, r: crate::layout::Rect, inp: &crate::io::Input) -> Image {
    let rf = [r[0] as f64, r[1] as f64, r[2] as f64, r[3] as f64];
    let t = room.w.s.t;
    let l = room.last.as_ref();
    D::snapshot(rf, 2., |c| {
        if let Some(d) = l.and_then(|l| l.desks.iter().find(|d| d.key == key)) {
            let desk = D::Desk {
                x0: at[0],
                y: at[1],
                w: u.unwrap_or(d.w),
                cab: d.cab,
                label: &d.label,
                dir: &d.d.dir,
                color: d.color,
                paint: room.paint_of(inp, &d.d.m),
                seats: d.seats.iter().map(|s| s.1 - d.x0 + at[0]).collect(),
                trees: d.trees.iter().map(|tr| D::Tree { dir: tr, ..Default::default() }).collect(),
                ..Default::default()
            };
            D::draw_desk(c, &desk, t);
        } else if let Some(mid) = key.strip_prefix("door:") {
            D::draw_door(c, at[0], &room.paint_of(inp, mid), false, false, at[1]);
        } else {
            let env = D::KindEnv { clock: room.clock(inp), door: D::WOOD };
            D::draw_kind(c, crate::layout::kind_of(key), at[0], at[1], t, &env);
        }
    })
}
