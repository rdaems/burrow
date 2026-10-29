//! The pane as a tiling window manager lays out windows (web/tiles.js): each tile a border with a thicker
//! band on top. A new tile splits the focused one along its longer side. Drag a tile by its band onto
//! another: on a side, it goes there (the other makes room); in the middle, they swap places. Drag the gap
//! between two tiles to resize them. Nothing is remembered: the tiles are what is open now.
//! This is only the tree and the geometry; the app draws the tiles where `layout` puts them.

/// the gap between tiles, around them, the least a tile gets
pub const G: f32 = 6.0;
pub const OUT: f32 = 6.0;
pub const MIN: f32 = 90.0;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// side by side
    Row,
    /// one over the other
    Col,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
    /// in its place
    Swap,
    /// the whole pane (nothing open)
    All,
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Leaf(u64),
    Split { dir: Dir, kids: Vec<Node>, sizes: Vec<f32> },
}

/// a draggable gap: between kid `i` and `i + 1` of the split at `path`
#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    pub rect: Rect,
    pub dir: Dir,
    path: Vec<usize>,
    i: usize,
    /// the two sides' extent and their share, when the drag starts
    span: f32,
}

/// where a drop would go
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub tile: Option<u64>,
    pub side: Side,
    /// the zone to show
    pub zone: Rect,
}

#[derive(Default, Debug, Clone)]
pub struct Tiles {
    root: Option<Node>,
    focused: Option<u64>,
    /// the tiles' rects and the gaps, as last laid out
    pub rects: Vec<(u64, Rect)>,
    pub gaps: Vec<Gap>,
}

fn leaves(n: &Node, out: &mut Vec<u64>) {
    match n {
        Node::Leaf(t) => out.push(*t),
        Node::Split { kids, .. } => kids.iter().for_each(|k| leaves(k, out)),
    }
}

fn path_of(n: &Node, t: u64) -> Option<Vec<usize>> {
    match n {
        Node::Leaf(x) => (*x == t).then(Vec::new),
        Node::Split { kids, .. } => kids.iter().enumerate().find_map(|(i, k)| path_of(k, t).map(|mut p| {
            p.insert(0, i);
            p
        })),
    }
}

fn at_mut<'a>(n: &'a mut Node, path: &[usize]) -> &'a mut Node {
    match (n, path.split_first()) {
        (n, None) => n,
        (Node::Split { kids, .. }, Some((i, rest))) => at_mut(&mut kids[*i], rest),
        (n, _) => n,
    }
}

fn norm(sizes: &mut [f32]) {
    let s: f32 = sizes.iter().sum();
    if s > 0.0 {
        sizes.iter_mut().for_each(|v| *v /= s);
    }
}

impl Tiles {
    pub fn all(&self) -> Vec<u64> {
        let mut v = Vec::new();
        if let Some(r) = &self.root {
            leaves(r, &mut v);
        }
        v
    }
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
    pub fn focused(&self) -> Option<u64> {
        self.focused
    }
    pub fn focus(&mut self, t: u64) {
        if self.all().contains(&t) {
            self.focused = Some(t);
        }
    }
    pub fn rect(&self, t: u64) -> Option<Rect> {
        self.rects.iter().find(|(x, _)| *x == t).map(|(_, r)| *r)
    }

    /// put `tile` beside the leaf at `path`, on `side`
    fn split_leaf(&mut self, path: &[usize], tile: u64, side: Side) {
        let dir = if matches!(side, Side::Left | Side::Right) { Dir::Row } else { Dir::Col };
        let before = matches!(side, Side::Left | Side::Top);
        let root = self.root.as_mut().unwrap();
        if let Some((i, parent)) = path.split_last().map(|(i, p)| (*i, p)) {
            if let Node::Split { dir: d, kids, sizes } = at_mut(root, parent)
                && *d == dir
            {
                // a sibling, half of the other's room
                let s = sizes[i] / 2.0;
                kids.insert(if before { i } else { i + 1 }, Node::Leaf(tile));
                sizes.splice(i..=i, [s, s]);
                return;
            }
        }
        let at = at_mut(root, path);
        let old = std::mem::replace(at, Node::Leaf(0));
        *at = Node::Split { dir, kids: if before { vec![Node::Leaf(tile), old] } else { vec![old, Node::Leaf(tile)] }, sizes: vec![0.5, 0.5] };
    }

    /// take a leaf out; a split with one kid left folds away
    fn unlink(&mut self, t: u64) {
        let Some(root) = self.root.as_mut() else { return };
        let Some(path) = path_of(root, t) else { return };
        let Some((i, parent)) = path.split_last() else {
            self.root = None;
            return;
        };
        let p = at_mut(root, parent);
        if let Node::Split { kids, sizes, .. } = p {
            kids.remove(*i);
            sizes.remove(*i);
            norm(sizes);
            if kids.len() == 1 {
                let only = kids.remove(0);
                *p = only;
            }
        }
    }

    /// a new tile: splits the focused one along its longer side (the first is the whole pane)
    pub fn add(&mut self, tile: u64) {
        match &self.root {
            None => self.root = Some(Node::Leaf(tile)),
            Some(root) => {
                let all = self.all();
                let at = self.focused.filter(|f| all.contains(f)).unwrap_or(*all.last().unwrap());
                let path = path_of(root, at).unwrap();
                let r = self.rect(at).unwrap_or(Rect { w: 1.0, h: 0.0, ..Default::default() });
                self.split_leaf(&path, tile, if r.w >= r.h { Side::Right } else { Side::Bottom });
            }
        }
        self.focused = Some(tile);
    }

    pub fn remove(&mut self, tile: u64) {
        self.unlink(tile);
        if self.focused == Some(tile) {
            self.focused = self.all().last().copied();
        }
    }

    /// a new tile put where a preview pointed: beside the tile there, or in its place (the one it replaces is
    /// returned, for the caller to close), or as the only one
    pub fn add_at(&mut self, tile: u64, target: &Target) -> Option<u64> {
        let Some(at) = target.tile.filter(|_| self.root.is_some()) else {
            self.add(tile);
            return None;
        };
        let path = path_of(self.root.as_ref().unwrap(), at)?;
        let mut was = None;
        if target.side == Side::Swap {
            *at_mut(self.root.as_mut().unwrap(), &path) = Node::Leaf(tile);
            was = Some(at);
        } else {
            self.split_leaf(&path, tile, target.side);
        }
        self.focused = Some(tile);
        was
    }

    /// a tile dragged by its band and let go on a target: swapped with the other, or moved beside it
    pub fn move_to(&mut self, tile: u64, target: &Target) {
        let Some(other) = target.tile else { return };
        if other == tile || self.root.is_none() {
            return;
        }
        let root = self.root.as_mut().unwrap();
        if target.side == Side::Swap {
            let (Some(a), Some(b)) = (path_of(root, tile), path_of(root, other)) else { return };
            *at_mut(root, &a) = Node::Leaf(other);
            *at_mut(self.root.as_mut().unwrap(), &b) = Node::Leaf(tile);
        } else {
            self.unlink(tile);
            if let Some(p) = path_of(self.root.as_ref().unwrap(), other) {
                self.split_leaf(&p, tile, target.side);
            }
        }
    }

    /// where everything goes, in a pane `w` x `h`
    pub fn layout(&mut self, w: f32, h: f32) {
        self.rects.clear();
        self.gaps.clear();
        if let Some(root) = self.root.clone() {
            self.place(&root, Rect { x: OUT, y: OUT, w: w - 2.0 * OUT, h: h - 2.0 * OUT }, &mut Vec::new());
        }
    }

    fn place(&mut self, n: &Node, r: Rect, path: &mut Vec<usize>) {
        match n {
            Node::Leaf(t) => self.rects.push((*t, r)),
            Node::Split { dir, kids, sizes } => {
                let row = *dir == Dir::Row;
                let total = (if row { r.w } else { r.h }) - G * (kids.len() as f32 - 1.0);
                let mut at = if row { r.x } else { r.y };
                for (i, k) in kids.iter().enumerate() {
                    let s = (total * sizes[i]).round();
                    let kr = if row { Rect { x: at, y: r.y, w: s, h: r.h } } else { Rect { x: r.x, y: at, w: r.w, h: s } };
                    path.push(i);
                    self.place(k, kr, path);
                    path.pop();
                    if i < kids.len() - 1 {
                        let g = if row { Rect { x: at + s, y: r.y, w: G, h: r.h } } else { Rect { x: r.x, y: at + s, w: r.w, h: G } };
                        self.gaps.push(Gap { rect: g, dir: *dir, path: path.clone(), i, span: total + G * (kids.len() as f32 - 1.0) });
                    }
                    at += s + G;
                }
            }
        }
    }

    /// a gap dragged by `d` pixels from where it was (`sizes0`, the split's sizes when the drag began): the two
    /// beside it trade room
    pub fn sizes_of(&self, gap: &Gap) -> Vec<f32> {
        match self.root.as_ref().map(|r| at_ref(r, &gap.path)) {
            Some(Node::Split { sizes, .. }) => sizes.clone(),
            _ => Vec::new(),
        }
    }
    pub fn drag_gap(&mut self, gap: &Gap, sizes0: &[f32], d: f32) {
        let Some(root) = self.root.as_mut() else { return };
        if let Node::Split { sizes, .. } = at_mut(root, &gap.path) {
            let i = gap.i;
            if sizes0.len() != sizes.len() || i + 1 >= sizes.len() {
                return;
            }
            let both = sizes0[i] + sizes0[i + 1];
            // the two kids' extent: their share of the split's (less the gaps)
            let span = gap.span * both;
            let dd = d / span * both;
            let lo = MIN / span * both;
            sizes[i] = (sizes0[i] + dd).clamp(lo, both - lo);
            sizes[i + 1] = both - sizes[i];
        }
    }

    /// the drop zone under a point (pane coordinates): a side of a tile, or its middle; with nothing open, the
    /// whole pane. `not` is the tile being dragged, which is no target
    pub fn preview(&self, x: f32, y: f32, w: f32, h: f32, not: Option<u64>) -> Option<Target> {
        if x < 0.0 || y < 0.0 || x >= w || y >= h {
            return None;
        }
        let Some((t, r)) = self.rects.iter().find(|(t, r)| Some(*t) != not && r.contains(x, y)).copied() else {
            if self.root.is_some() {
                return None;
            }
            return Some(Target { tile: None, side: Side::All, zone: Rect { x: OUT, y: OUT, w: w - 2.0 * OUT, h: h - 2.0 * OUT } });
        };
        let (px, py) = ((x - r.x) / r.w, (y - r.y) / r.h);
        let edge = px.min(1.0 - px).min(py).min(1.0 - py);
        let side = if edge > 0.28 {
            Side::Swap
        } else if px == edge {
            Side::Left
        } else if 1.0 - px == edge {
            Side::Right
        } else if py == edge {
            Side::Top
        } else {
            Side::Bottom
        };
        let zone = match side {
            Side::Left => Rect { w: r.w / 2.0, ..r },
            Side::Right => Rect { x: r.x + r.w / 2.0, w: r.w / 2.0, ..r },
            Side::Top => Rect { h: r.h / 2.0, ..r },
            Side::Bottom => Rect { y: r.y + r.h / 2.0, h: r.h / 2.0, ..r },
            _ => r,
        };
        Some(Target { tile: Some(t), side, zone })
    }
}

fn at_ref<'a>(n: &'a Node, path: &[usize]) -> &'a Node {
    match (n, path.split_first()) {
        (n, None) => n,
        (Node::Split { kids, .. }, Some((i, rest))) => at_ref(&kids[*i], rest),
        (n, _) => n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_tile_splits_the_focused_along_its_longer_side() {
        let mut t = Tiles::default();
        t.add(1);
        t.layout(1000.0, 600.0);
        assert_eq!(t.rect(1).unwrap(), Rect { x: 6.0, y: 6.0, w: 988.0, h: 588.0 });
        t.add(2); // wide: side by side
        t.layout(1000.0, 600.0);
        let (a, b) = (t.rect(1).unwrap(), t.rect(2).unwrap());
        assert_eq!(a.y, b.y);
        assert!(b.x > a.x && (a.w - b.w).abs() <= 1.0);
        assert_eq!(b.x - (a.x + a.w), G);
        t.add(3); // the focused (2) is tall now: under it
        t.layout(1000.0, 600.0);
        let (b, c) = (t.rect(2).unwrap(), t.rect(3).unwrap());
        assert_eq!(b.x, c.x);
        assert!(c.y > b.y);
        assert_eq!(t.focused(), Some(3));
    }

    #[test]
    fn removing_folds_the_split() {
        let mut t = Tiles::default();
        for i in 1..=3 {
            t.add(i);
        }
        t.remove(3);
        t.remove(2);
        t.layout(800.0, 600.0);
        assert_eq!(t.all(), vec![1]);
        assert_eq!(t.rect(1).unwrap().w, 788.0);
        assert_eq!(t.focused(), Some(1));
        t.remove(1);
        assert!(t.is_empty());
    }

    #[test]
    fn drop_zones_and_moves() {
        let mut t = Tiles::default();
        t.add(1);
        t.add(2);
        t.layout(1000.0, 600.0);
        let r1 = t.rect(1).unwrap();
        let mid = t.preview(r1.x + r1.w / 2.0, r1.y + r1.h / 2.0, 1000.0, 600.0, None).unwrap();
        assert_eq!((mid.tile, mid.side), (Some(1), Side::Swap));
        let top = t.preview(r1.x + r1.w / 2.0, r1.y + 5.0, 1000.0, 600.0, None).unwrap();
        assert_eq!(top.side, Side::Top);
        t.move_to(2, &top); // 2 over 1
        t.layout(1000.0, 600.0);
        let (a, b) = (t.rect(1).unwrap(), t.rect(2).unwrap());
        assert!(b.y < a.y && a.w == b.w);
        let was = t.add_at(3, &Target { tile: Some(1), side: Side::Swap, zone: a });
        assert_eq!(was, Some(1));
        assert_eq!(t.all().len(), 2);
        // nothing open: the whole pane
        let empty = Tiles::default();
        assert_eq!(empty.preview(10.0, 10.0, 500.0, 400.0, None).unwrap().side, Side::All);
    }

    #[test]
    fn a_gap_trades_room_but_leaves_a_minimum() {
        let mut t = Tiles::default();
        t.add(1);
        t.add(2);
        t.layout(1000.0, 600.0);
        let gap = t.gaps[0].clone();
        let s0 = t.sizes_of(&gap);
        t.drag_gap(&gap, &s0, 200.0);
        t.layout(1000.0, 600.0);
        assert!(t.rect(1).unwrap().w > t.rect(2).unwrap().w + 300.0);
        t.drag_gap(&gap, &s0, 5000.0);
        t.layout(1000.0, 600.0);
        assert!(t.rect(2).unwrap().w >= MIN - 1.0);
    }
}
