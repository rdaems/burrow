/// A path as the canvas builds one (moveTo, lineTo, rect, arc, closePath), kept as polygons; arcs
/// become short straight pieces.
#[derive(Default, Clone, Debug)]
pub struct Path {
    polys: Vec<Vec<(f64, f64)>>,
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.polys.push(vec![(x, y)]);
    }
    pub fn line_to(&mut self, x: f64, y: f64) {
        match self.polys.last_mut() {
            Some(p) => p.push((x, y)),
            None => self.polys.push(vec![(x, y)]),
        }
    }
    pub fn close(&mut self) {
        if let Some(&first) = self.polys.last().and_then(|p| p.first()) {
            self.polys.push(vec![first]);
        }
    }
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.polys.push(vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]);
    }
    /// an arc from angle a0 to a1 (radians, clockwise on screen as the canvas draws), joined to what was
    pub fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64) {
        self.ellipse(cx, cy, r, r, a0, a1);
    }
    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, a0: f64, a1: f64) {
        let sweep = (a1 - a0).clamp(-std::f64::consts::TAU, std::f64::consts::TAU);
        let n = ((sweep.abs() * rx.max(ry)).ceil() as usize).clamp(8, 256);
        let pts = (0..=n).map(|i| {
            let a = a0 + sweep * i as f64 / n as f64;
            (cx + rx * a.cos(), cy + ry * a.sin())
        });
        match self.polys.last_mut() {
            Some(p) if !p.is_empty() && p.len() > 1 => p.extend(pts),
            _ => self.polys.push(pts.collect()),
        }
    }
    pub fn polys(&self) -> &[Vec<(f64, f64)>] {
        &self.polys
    }
}
