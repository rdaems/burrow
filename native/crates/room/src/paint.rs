//! The bridge to the painters (`draw`): what the room's logic needs of the pictures. Filled in once the
//! painters are ported.

use pix::Color;

/// how far above its anchor a kind of piece is drawn (its topmost opaque row)
pub fn vis_top(kind: &str) -> i32 {
    crate::layout::foot(kind, [0, 0], None)[1]
}

/// the whiteboard dream's pixels, in the order they come in
pub fn dream_scene(_i: usize) -> Vec<(i32, i32, Color)> {
    Vec::new()
}
