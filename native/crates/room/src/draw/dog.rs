//! The dog (the dog house, from the closet): a dachshund, a badger dog, as burrows go; its hearts and the
//! stick you throw for it.

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// its colours by letter
pub fn dog_color(ch: char) -> Option<Color> {
    Some(match ch {
        'b' => Color::rgb(0xc27a3e),
        'd' => Color::rgb(0x7a4424),
        'w' => Color::rgb(0xefd8b4),
        'r' => Color::rgb(0xd9473a),
        'n' | 'e' => Color::rgb(0x1a1626),
        _ => return None,
    })
}
/// its poses, facing right; its tail is put on by the wag
pub fn dog_rows(pose: &str) -> &'static [&'static str] {
    match pose {
        "walk1" => &["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "...bb......bb..."],
        "walk2" => &["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "..b.b.....b.b..."],
        "sit" => &[".......bbb..", "......bbbebb", "......dbbbbn", "......rbb...", "...bbbbbb...", "..bbbbbbw...", "..bb..bb...."],
        "lie" => &["...........bbb..", "..........bbbdbb", ".bbbbbbbbbdbbbbn", ".bbbbbbbbbbbbw.."],
        _ => &["...........bbb..", "..........bbbebb", "..........dbbbbn", "..........rbb...", "..bbbbbbbbbbb...", "..bbbbbbbbbbw...", "..bb......bb...."],
    }
}
/// where its tail is, still and wagging
pub fn dog_tail(pose: &str, wag: bool) -> &'static [(i32, i32)] {
    match (pose, wag) {
        ("sit", false) => &[(1, 5), (0, 6)],
        ("sit", true) => &[(1, 5), (0, 5)],
        ("lie", false) => &[(0, 3)],
        ("lie", true) => &[(0, 2)],
        (_, false) => &[(1, 3), (0, 2)],
        (_, true) => &[(1, 3), (1, 2)],
    }
}

thread_local! {
    static DOG_PICS: RefCell<HashMap<(String, bool), Rc<Image>>> = RefCell::new(HashMap::new());
}
/// the dog in a pose, outlined in ink
pub fn dog_pic(pose: &str, wag: bool) -> Rc<Image> {
    let key = (pose.to_string(), wag);
    if let Some(im) = DOG_PICS.with(|m| m.borrow().get(&key).cloned()) {
        return im;
    }
    let rows = dog_rows(pose);
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32 + 2;
    let h = rows.len() as i32 + 2;
    let mut px: HashMap<i32, Color> = HashMap::new();
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.chars().enumerate() {
            if let Some(c) = dog_color(ch) {
                px.insert((y as i32 + 1) * w + x as i32 + 1, c);
            }
        }
    }
    for &(x, y) in dog_tail(pose, wag) {
        px.insert((y + 1) * w + x + 1, dog_color('d').unwrap());
    }
    let mut im = Image::new(w as u32, h as u32);
    for j in 0..h {
        for i in 0..w {
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| i + dx >= 0 && i + dx < w && px.contains_key(&((j + dy) * w + i + dx)));
            if !px.contains_key(&(j * w + i)) && near {
                im.set(i, j, INK.0);
            }
        }
    }
    for (k, col) in &px {
        im.set(k % w, k / w, col.0);
    }
    let im = Rc::new(im);
    DOG_PICS.with(|m| m.borrow_mut().insert(key, im.clone()));
    im
}

/// a heart over someone petting it
pub fn heart(ctx: &mut Ctx, x: i32, y: i32) {
    rect(ctx, x - 2, y - 1, 2, 1, RED);
    rect(ctx, x + 1, y - 1, 2, 1, RED);
    rect(ctx, x - 2, y, 5, 1, RED);
    rect(ctx, x - 1, y + 1, 3, 1, RED);
    rect(ctx, x, y + 2, 1, 1, RED);
}

/// the stick you throw for it: tumbling (four turns) on the way, lying on the floor, across its mouth on the way back
pub const STICK: Color = Color::rgb(0xe0bd84);
/// where the stick is in its mouth, by pose
pub fn dog_mouth(pose: &str) -> (i32, i32) {
    if pose == "sit" { (8, 4) } else { (12, 4) }
}
pub fn draw_stick(ctx: &mut Ctx, x: i32, y: i32, turn: usize) {
    const PTS: [&[(i32, i32)]; 4] = [
        &[(-3, 0), (-2, 0), (-1, 0), (0, 0), (1, 0), (2, 0)],
        &[(-2, -2), (-1, -1), (0, 0), (1, 1), (2, 2)],
        &[(0, -3), (0, -2), (0, -1), (0, 0), (0, 1), (0, 2)],
        &[(-2, 2), (-1, 1), (0, 0), (1, -1), (2, -2)],
    ];
    let pts = PTS[turn % 4];
    for &(i, j) in pts {
        rect(ctx, x + i - 1, y + j - 1, 3, 3, INK);
    }
    for &(i, j) in pts {
        rect(ctx, x + i, y + j, 1, 1, STICK);
    }
}
