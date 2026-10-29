//! burrow's pixel drawing: an RGBA buffer, and the part of the browser's 2D canvas that the web room
//! (room.js, outside.js, pong.js, fps.js) drew with: rectangles, images (nearest neighbour, as with
//! `imageSmoothingEnabled = false`), a transform, global alpha, a few composite operations, a clip
//! rectangle, filled paths. A pixel is painted when its centre is inside a shape, so edges are hard
//! (the browser blends a fractional edge; the room draws on whole dots almost everywhere).

mod color;
mod path;
pub use color::{Color, css};
pub use path::Path;

/// An image, straight (not premultiplied) RGBA, row by row: what `getImageData` returns.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub px: Vec<[u8; 4]>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}x{})", self.w, self.h)
    }
}

impl Image {
    pub fn new(w: u32, h: u32) -> Image {
        Image { w, h, px: vec![[0; 4]; (w * h) as usize] }
    }
    pub fn filled(w: u32, h: u32, c: Color) -> Image {
        Image { w, h, px: vec![c.0; (w * h) as usize] }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return [0; 4];
        }
        self.px[(y as u32 * self.w + x as u32) as usize]
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as u32 * self.w + x as u32) as usize;
        self.px[i] = c;
    }
    pub fn clear(&mut self) {
        self.px.fill([0; 4]);
    }
    /// a canvas of its own to draw on
    pub fn ctx(&mut self) -> Ctx<'_> {
        Ctx::new(self)
    }
    /// how many pixels are not fully transparent
    pub fn inked(&self) -> usize {
        self.px.iter().filter(|p| p[3] > 0).count()
    }

    pub fn from_png(bytes: &[u8]) -> Result<Image, String> {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = dec.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("png too big")?];
        let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width, info.height);
        let mut px = Vec::with_capacity((w * h) as usize);
        let n = info.color_type.samples();
        for c in buf[..info.buffer_size()].chunks(n) {
            px.push(match n {
                1 => [c[0], c[0], c[0], 255],
                2 => [c[0], c[0], c[0], c[1]],
                3 => [c[0], c[1], c[2], 255],
                _ => [c[0], c[1], c[2], c[3]],
            });
        }
        Ok(Image { w, h, px })
    }
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.w, self.h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().expect("png header");
            let flat: Vec<u8> = self.px.iter().flatten().copied().collect();
            wr.write_image_data(&flat).expect("png data");
        }
        out
    }
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_png())
    }

    /// `k` x `k` pixels a pixel, as BGRA bytes (what a GPU texture takes), over `bg` where transparent
    pub fn upscale_bgra(&self, k: u32, bg: [u8; 4]) -> Vec<u8> {
        let (w, h) = (self.w * k, self.h * k);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let row_bytes = (w * 4) as usize;
        for y in 0..self.h {
            let base = (y * k) as usize * row_bytes;
            {
                let row = &mut out[base..base + row_bytes];
                for x in 0..self.w {
                    let p = over(bg, self.px[(y * self.w + x) as usize]);
                    let q = [p[2], p[1], p[0], p[3]];
                    for i in 0..k {
                        let o = ((x * k + i) * 4) as usize;
                        row[o..o + 4].copy_from_slice(&q);
                    }
                }
            }
            for j in 1..k as usize {
                out.copy_within(base..base + row_bytes, base + j * row_bytes);
            }
        }
        out
    }
    /// scaled by a whole number, nearest neighbour
    pub fn scaled(&self, k: u32) -> Image {
        let mut out = Image::new(self.w * k, self.h * k);
        for y in 0..out.h {
            for x in 0..out.w {
                out.px[(y * out.w + x) as usize] = self.px[((y / k) * self.w + x / k) as usize];
            }
        }
        out
    }
    /// a part of it
    pub fn crop(&self, x: i32, y: i32, w: u32, h: u32) -> Image {
        let mut out = Image::new(w, h);
        for j in 0..h as i32 {
            for i in 0..w as i32 {
                out.set(i, j, self.get(x + i, y + j));
            }
        }
        out
    }
    pub fn flipped_x(&self) -> Image {
        let mut out = Image::new(self.w, self.h);
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                out.set(self.w as i32 - 1 - x, y, self.get(x, y));
            }
        }
        out
    }
}

/// `b` drawn over `a` (straight alpha)
#[inline]
pub fn over(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
    match b[3] {
        255 => b,
        0 => a,
        sa => {
            let sa = sa as u32;
            let da = a[3] as u32;
            let oa = sa * 255 + da * (255 - sa); // x 255
            if oa == 0 {
                return [0; 4];
            }
            let ch = |s: u8, d: u8| ((s as u32 * sa * 255 + d as u32 * da * (255 - sa) + oa / 2) / oa) as u8;
            [ch(b[0], a[0]), ch(b[1], a[1]), ch(b[2], a[2]), ((oa + 127) / 255) as u8]
        }
    }
}

/// How a source pixel meets what is there (`globalCompositeOperation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    SourceOver,
    /// only where something is already, keeping its alpha (a wash over a picture)
    SourceAtop,
    /// erases where the source is
    DestinationOut,
    /// what is under the source stays only where the source is (a mask)
    DestinationIn,
    /// replaces
    Copy,
    /// adds light
    Lighter,
    /// darkens by the source's colour (canvas's "multiply")
    Multiply,
}

#[inline]
fn comp(op: Op, d: [u8; 4], s: [u8; 4]) -> [u8; 4] {
    match op {
        Op::SourceOver => over(d, s),
        Op::Copy => s,
        Op::SourceAtop => {
            if d[3] == 0 || s[3] == 0 {
                return d;
            }
            let a = s[3] as u32;
            let mix = |s: u8, d: u8| ((s as u32 * a + d as u32 * (255 - a) + 127) / 255) as u8;
            [mix(s[0], d[0]), mix(s[1], d[1]), mix(s[2], d[2]), d[3]]
        }
        Op::DestinationOut => {
            let a = d[3] as u32 * (255 - s[3] as u32) / 255;
            [d[0], d[1], d[2], a as u8]
        }
        Op::DestinationIn => {
            let a = d[3] as u32 * s[3] as u32 / 255;
            [d[0], d[1], d[2], a as u8]
        }
        Op::Multiply => {
            // the W3C blending: co = cs·as·(1 − ad) + cd·ad·(1 − as) + as·ad·cs·cd, then out of premultiplied
            let (sa, da) = (s[3] as f64 / 255.0, d[3] as f64 / 255.0);
            let a = sa + da * (1.0 - sa);
            if a <= 0.0 {
                return [0; 4];
            }
            let ch = |s: u8, d: u8| {
                let (cs, cd) = (s as f64 / 255.0, d as f64 / 255.0);
                let co = cs * sa * (1.0 - da) + cd * da * (1.0 - sa) + sa * da * cs * cd;
                (co / a * 255.0).round().clamp(0.0, 255.0) as u8
            };
            [ch(s[0], d[0]), ch(s[1], d[1]), ch(s[2], d[2]), (a * 255.0).round() as u8]
        }
        Op::Lighter => {
            let sa = s[3] as u32;
            let da = d[3] as u32;
            let ch = |s: u8, d: u8| ((s as u32 * sa + d as u32 * da) / 255).min(255);
            let a = (sa + da).min(255);
            if a == 0 {
                return [0; 4];
            }
            [
                (ch(s[0], d[0]) * 255 / a).min(255) as u8,
                (ch(s[1], d[1]) * 255 / a).min(255) as u8,
                (ch(s[2], d[2]) * 255 / a).min(255) as u8,
                a as u8,
            ]
        }
    }
}

/// x' = a x + c y + e, y' = b x + d y + f (the canvas's own order)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub const ID: Affine = Affine { a: 1., b: 0., c: 0., d: 1., e: 0., f: 0. };
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
    pub fn then(&self, m: &Affine) -> Affine {
        // self * m: m applied first, then self
        Affine {
            a: self.a * m.a + self.c * m.b,
            b: self.b * m.a + self.d * m.b,
            c: self.a * m.c + self.c * m.d,
            d: self.b * m.c + self.d * m.d,
            e: self.a * m.e + self.c * m.f + self.e,
            f: self.b * m.e + self.d * m.f + self.f,
        }
    }
    pub fn inverse(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Affine { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) })
    }
    fn axis_aligned(&self) -> bool {
        self.b == 0. && self.c == 0.
    }
}

#[derive(Clone, Copy)]
struct State {
    t: Affine,
    alpha: f64,
    op: Op,
    clip: Option<[i32; 4]>,
}

/// A 2D canvas over an image: the transform, globalAlpha, the composite operation and a clip rectangle
/// are kept as the browser keeps them (save/restore).
pub struct Ctx<'a> {
    pub img: &'a mut Image,
    st: State,
    stack: Vec<State>,
}

impl<'a> Ctx<'a> {
    pub fn new(img: &'a mut Image) -> Ctx<'a> {
        Ctx { img, st: State { t: Affine::ID, alpha: 1.0, op: Op::SourceOver, clip: None }, stack: Vec::new() }
    }
    pub fn save(&mut self) {
        self.stack.push(self.st);
    }
    pub fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.st = s;
        }
    }
    pub fn transform(&self) -> Affine {
        self.st.t
    }
    pub fn set_transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.st.t = Affine { a, b, c, d, e, f };
    }
    pub fn translate(&mut self, x: f64, y: f64) {
        self.st.t = self.st.t.then(&Affine { e: x, f: y, ..Affine::ID });
    }
    pub fn scale(&mut self, x: f64, y: f64) {
        self.st.t = self.st.t.then(&Affine { a: x, d: y, ..Affine::ID });
    }
    pub fn rotate(&mut self, r: f64) {
        let (s, c) = r.sin_cos();
        self.st.t = self.st.t.then(&Affine { a: c, b: s, c: -s, d: c, e: 0., f: 0. });
    }
    pub fn alpha(&self) -> f64 {
        self.st.alpha
    }
    pub fn set_alpha(&mut self, a: f64) {
        if a.is_finite() && (0.0..=1.0).contains(&a) {
            self.st.alpha = a;
        }
    }
    pub fn set_op(&mut self, op: Op) {
        self.st.op = op;
    }
    /// clip to a rectangle (in the current transform), within the clip there is
    pub fn clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let r = self.device_rect(x, y, w, h);
        self.st.clip = Some(match self.st.clip {
            Some(c) => [r[0].max(c[0]), r[1].max(c[1]), r[2].min(c[2]), r[3].min(c[3])],
            None => r,
        });
    }

    /// the device pixels a rectangle covers: those whose centre is inside it
    fn device_rect(&self, x: f64, y: f64, w: f64, h: f64) -> [i32; 4] {
        let (x0, y0) = self.st.t.apply(x, y);
        let (x1, y1) = self.st.t.apply(x + w, y + h);
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (y0, y1) = (y0.min(y1), y0.max(y1));
        [(x0 - 0.5).ceil() as i32, (y0 - 0.5).ceil() as i32, (x1 - 0.5).ceil() as i32, (y1 - 0.5).ceil() as i32]
    }
    fn bounds(&self, r: [i32; 4]) -> [i32; 4] {
        let mut r = [r[0].max(0), r[1].max(0), r[2].min(self.img.w as i32), r[3].min(self.img.h as i32)];
        if let Some(c) = self.st.clip {
            r = [r[0].max(c[0]), r[1].max(c[1]), r[2].min(c[2]), r[3].min(c[3])];
        }
        r
    }
    #[inline]
    fn put(&mut self, x: i32, y: i32, mut c: [u8; 4]) {
        if self.st.alpha < 1.0 {
            c[3] = (c[3] as f64 * self.st.alpha).round() as u8;
        }
        let i = (y as u32 * self.img.w + x as u32) as usize;
        self.img.px[i] = comp(self.st.op, self.img.px[i], c);
    }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
        if w <= 0. || h <= 0. {
            return;
        }
        if self.st.t.axis_aligned() {
            let r = self.bounds(self.device_rect(x, y, w, h));
            for py in r[1]..r[3] {
                for px in r[0]..r[2] {
                    self.put(px, py, c.0);
                }
            }
        } else {
            let mut p = Path::new();
            p.rect(x, y, w, h);
            self.fill_path(&p, c);
        }
    }
    /// a rectangle in whole numbers (the common case)
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        self.fill_rect(x as f64, y as f64, w as f64, h as f64, c);
    }
    pub fn clear_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let r = self.bounds(self.device_rect(x, y, w, h));
        for py in r[1]..r[3] {
            for px in r[0]..r[2] {
                self.img.set(px, py, [0; 4]);
            }
        }
    }

    pub fn draw_image(&mut self, im: &Image, x: f64, y: f64) {
        self.draw_image_sub(im, 0., 0., im.w as f64, im.h as f64, x, y, im.w as f64, im.h as f64);
    }
    pub fn draw_image_scaled(&mut self, im: &Image, x: f64, y: f64, w: f64, h: f64) {
        self.draw_image_sub(im, 0., 0., im.w as f64, im.h as f64, x, y, w, h);
    }
    /// `drawImage(im, sx, sy, sw, sh, dx, dy, dw, dh)`, nearest neighbour
    #[allow(clippy::too_many_arguments)]
    pub fn draw_image_sub(&mut self, im: &Image, sx: f64, sy: f64, sw: f64, sh: f64, dx: f64, dy: f64, dw: f64, dh: f64) {
        if sw <= 0. || sh <= 0. || dw == 0. || dh == 0. {
            return;
        }
        // dest rect in device space -> source pixel
        let m = self.st.t.then(&Affine { a: dw / sw, b: 0., c: 0., d: dh / sh, e: dx - sx * dw / sw, f: dy - sy * dh / sh });
        let Some(inv) = m.inverse() else { return };
        let corners = [(sx, sy), (sx + sw, sy), (sx, sy + sh), (sx + sw, sy + sh)].map(|(x, y)| m.apply(x, y));
        let xs = corners.iter().map(|c| c.0);
        let ys = corners.iter().map(|c| c.1);
        let (x0, x1) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
        let (y0, y1) = (ys.clone().fold(f64::MAX, f64::min), ys.fold(f64::MIN, f64::max));
        let r = self.bounds([(x0 - 0.5).ceil() as i32, (y0 - 0.5).ceil() as i32, (x1 - 0.5).ceil() as i32, (y1 - 0.5).ceil() as i32]);
        let (sx1, sy1) = (sx + sw, sy + sh);
        for py in r[1]..r[3] {
            for px in r[0]..r[2] {
                let (u, v) = inv.apply(px as f64 + 0.5, py as f64 + 0.5);
                if u < sx || v < sy || u >= sx1 || v >= sy1 {
                    continue;
                }
                let c = im.get(u.floor() as i32, v.floor() as i32);
                if c[3] == 0 && self.st.op != Op::Copy && self.st.op != Op::DestinationIn {
                    continue;
                }
                self.put(px, py, c);
            }
        }
    }

    /// a path filled (non-zero), by pixel centres
    pub fn fill_path(&mut self, p: &Path, c: Color) {
        let polys: Vec<Vec<(f64, f64)>> =
            p.polys().iter().map(|poly| poly.iter().map(|&(x, y)| self.st.t.apply(x, y)).collect()).collect();
        let (mut y0, mut y1) = (f64::MAX, f64::MIN);
        for poly in &polys {
            for &(_, y) in poly {
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
        if y0 > y1 {
            return;
        }
        let b = self.bounds([0, (y0 - 0.5).ceil() as i32, self.img.w as i32, (y1 - 0.5).ceil() as i32]);
        let mut xs: Vec<(f64, i32)> = Vec::new();
        for py in b[1]..b[3] {
            let cy = py as f64 + 0.5;
            xs.clear();
            for poly in &polys {
                let n = poly.len();
                for i in 0..n {
                    let (ax, ay) = poly[i];
                    let (bx, by) = poly[(i + 1) % n];
                    if (ay <= cy && by > cy) || (by <= cy && ay > cy) {
                        let x = ax + (cy - ay) / (by - ay) * (bx - ax);
                        xs.push((x, if by > ay { 1 } else { -1 }));
                    }
                }
            }
            xs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut wind = 0;
            for i in 0..xs.len() {
                let before = wind;
                wind += xs[i].1;
                if before == 0 && wind != 0 {
                    let start = xs[i].0;
                    let mut j = i + 1;
                    let mut w2 = wind;
                    while j < xs.len() {
                        w2 += xs[j].1;
                        if w2 == 0 {
                            break;
                        }
                        j += 1;
                    }
                    if j < xs.len() {
                        let end = xs[j].0;
                        let pa = ((start - 0.5).ceil() as i32).max(b[0]);
                        let pb = ((end - 0.5).ceil() as i32).min(b[2]);
                        for px in pa..pb {
                            self.put(px, py, c.0);
                        }
                    }
                }
            }
        }
    }
    /// a filled circle (an arc all round)
    pub fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color) {
        let mut p = Path::new();
        p.arc(cx, cy, r, 0., std::f64::consts::TAU);
        self.fill_path(&p, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_and_scale_are_whole_pixels() {
        let mut im = Image::new(8, 8);
        let mut c = im.ctx();
        c.set_transform(2., 0., 0., 2., 0., 0.);
        c.rect(1, 1, 2, 1, css("#ff0000"));
        assert_eq!(im.get(1, 2), [0; 4]);
        assert_eq!(im.get(2, 2), [255, 0, 0, 255]);
        assert_eq!(im.get(5, 3), [255, 0, 0, 255]);
        assert_eq!(im.get(6, 3), [0; 4]);
        assert_eq!(im.inked(), 8);
    }

    #[test]
    fn half_alpha_over_white() {
        let mut im = Image::filled(1, 1, css("#ffffff"));
        let mut c = im.ctx();
        c.set_alpha(0.5);
        c.rect(0, 0, 1, 1, css("#000000"));
        let p = im.get(0, 0);
        assert!((126..=129).contains(&p[0]) && p[3] == 255, "{p:?}");
    }

    #[test]
    fn image_flipped_by_scale() {
        let mut src = Image::new(2, 1);
        src.set(0, 0, [1, 2, 3, 255]);
        src.set(1, 0, [4, 5, 6, 255]);
        let mut im = Image::new(4, 1);
        let mut c = im.ctx();
        c.translate(2., 0.);
        c.scale(-1., 1.);
        c.draw_image(&src, 0., 0.);
        assert_eq!(im.get(1, 0), [1, 2, 3, 255]);
        assert_eq!(im.get(0, 0), [4, 5, 6, 255]);
    }

    #[test]
    fn source_atop_keeps_alpha() {
        let mut im = Image::new(2, 1);
        im.set(0, 0, [0, 0, 0, 255]);
        let mut c = im.ctx();
        c.set_op(Op::SourceAtop);
        c.rect(0, 0, 2, 1, css("#ff0000"));
        assert_eq!(im.get(0, 0), [255, 0, 0, 255]);
        assert_eq!(im.get(1, 0), [0; 4]);
    }

    #[test]
    fn circle_is_round() {
        let mut im = Image::new(12, 12);
        im.ctx().fill_circle(6., 6., 5.5, css("#ffffff"));
        assert_eq!(im.get(6, 1)[3], 255);
        assert_eq!(im.get(0, 0)[3], 0);
        assert_eq!(im.get(6, 6)[3], 255);
    }

    #[test]
    fn png_round_trip() {
        let mut im = Image::new(3, 2);
        im.set(2, 1, [10, 20, 30, 40]);
        let back = Image::from_png(&im.to_png()).unwrap();
        assert_eq!(back, im);
    }

    #[test]
    fn upscale_is_bgra_blocks() {
        let mut im = Image::new(1, 1);
        im.set(0, 0, [1, 2, 3, 255]);
        let b = im.upscale_bgra(2, [0, 0, 0, 255]);
        assert_eq!(b.len(), 16);
        assert_eq!(&b[12..16], &[3, 2, 1, 255]);
    }
}
