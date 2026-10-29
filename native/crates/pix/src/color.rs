/// A colour, straight RGBA. `css("#d97757")`, `css("rgba(0,0,0,0.2)")`, or `Color::rgb(0xd97757)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const NONE: Color = Color([0, 0, 0, 0]);
    pub const fn rgb(v: u32) -> Color {
        Color([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255])
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color([r, g, b, a])
    }
    pub fn with_alpha(self, a: f64) -> Color {
        let mut c = self.0;
        c[3] = (a.clamp(0., 1.) * 255.).round() as u8;
        Color(c)
    }
    /// each channel times `k`, at most 255 (room.js's `shade`)
    pub fn shade(self, k: f64) -> Color {
        let f = |v: u8| ((v as f64 * k).round()).min(255.) as u8;
        Color([f(self.0[0]), f(self.0[1]), f(self.0[2]), self.0[3]])
    }
    /// `a` toward `b` by `k` (room.js's `mix`)
    pub fn mix(self, b: Color, k: f64) -> Color {
        let f = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * k).round().clamp(0., 255.) as u8;
        Color([f(self.0[0], b.0[0]), f(self.0[1], b.0[1]), f(self.0[2], b.0[2]), 255])
    }
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }
    pub fn is_none(self) -> bool {
        self.0[3] == 0
    }
}

/// a CSS colour as the web room wrote them: #rgb, #rrggbb, #rrggbbaa, rgb(), rgba(); anything else is none
pub fn css(s: &str) -> Color {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#') {
        let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
        return match h.len() {
            3 => match (v(0, 1), v(1, 1), v(2, 1)) {
                (Some(r), Some(g), Some(b)) => Color([r * 17, g * 17, b * 17, 255]),
                _ => Color::NONE,
            },
            6 | 8 => match (v(0, 2), v(2, 2), v(4, 2)) {
                (Some(r), Some(g), Some(b)) => Color([r, g, b, if h.len() == 8 { v(6, 2).unwrap_or(255) } else { 255 }]),
                _ => Color::NONE,
            },
            _ => Color::NONE,
        };
    }
    if let Some(body) = s.strip_prefix("rgba(").or_else(|| s.strip_prefix("rgb(")) {
        let parts: Vec<f64> = body.trim_end_matches(')').split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if parts.len() >= 3 {
            let a = parts.get(3).copied().unwrap_or(1.0);
            return Color([parts[0] as u8, parts[1] as u8, parts[2] as u8, (a * 255.).round() as u8]);
        }
    }
    match s {
        "white" => Color::rgb(0xffffff),
        "black" => Color::rgb(0x000000),
        _ => Color::NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses() {
        assert_eq!(css("#d97757"), Color::rgb(0xd97757));
        assert_eq!(css("#fff"), Color::rgb(0xffffff));
        assert_eq!(css("rgba(0,0,0,0.2)"), Color([0, 0, 0, 51]));
        assert_eq!(Color::rgb(0x808080).shade(0.5), Color::rgb(0x404040));
        assert_eq!(css("#8b93ad").shade(0.62).hex(), "#565b6b");
    }
}
