//! Math.random's place: a seeded generator, so a frame or a game can be played again exactly (tests)

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    /// 0 ≤ x < 1, as Math.random
    pub fn random(&mut self) -> f64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
    /// 0..255, Doom's P_Random near enough (`Math.random() * 256 | 0`)
    pub fn rnd(&mut self) -> i32 {
        (self.random() * 256.0) as i32
    }
    /// 0..n
    pub fn below(&mut self, n: usize) -> usize {
        ((self.random() * n as f64) as usize).min(n.saturating_sub(1))
    }
}

/// JavaScript's Math.round: halves go up (Rust's round takes them away from zero)
#[inline]
pub fn jround(x: f64) -> f64 {
    (x + 0.5).floor()
}
