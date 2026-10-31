/// xorshift64*: small, seedable and free of platform dependencies.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // The state must not be zero; the multiplier also spreads small seeds.
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `0..bound`.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.unit() * bound as f64) as usize
    }

    /// Approximately normal with mean 0 and unit deviation.
    pub fn gaussian(&mut self) -> f64 {
        // The sum of 12 uniforms has variance 1.
        (0..12).map(|_| self.unit()).sum::<f64>() - 6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_in_range_and_is_reproducible() {
        let mut a = Rng::new(0);
        let mut b = Rng::new(0);
        for _ in 0..1000 {
            let unit = a.unit();
            assert!((0.0..1.0).contains(&unit));
            assert_eq!(unit, b.unit());
            assert!(a.below(7) < 7);
            b.below(7);
        }
    }

    #[test]
    fn gaussian_is_centered_with_unit_deviation() {
        let mut rng = Rng::new(42);
        let samples: Vec<f64> = (0..20_000).map(|_| rng.gaussian()).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance = samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64;
        assert!(mean.abs() < 0.05);
        assert!((variance - 1.0).abs() < 0.1);
    }
}
