/// mulberry32: a tiny seedable PRNG. `tools/oracle/run.js` uses the same algorithm, so `random`
/// produces identical sequences in the Rust interpreter and in the JS oracle.
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub fn new(seed: u64) -> Self {
        // The JS harness seeds with `seed >>> 0`, i.e. the low 32 bits.
        Mulberry32 { state: seed as u32 }
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let a = self.state;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_unit_interval_and_deterministic() {
        let mut a = Mulberry32::new(42);
        let mut b = Mulberry32::new(42);
        for _ in 0..1000 {
            let x = a.next_f64();
            assert!((0.0..1.0).contains(&x));
            assert_eq!(x, b.next_f64());
        }
    }

    #[test]
    fn matches_js_reference() {
        // Values from the JS implementation in tools/oracle/run.js with seed 42.
        let mut r = Mulberry32::new(42);
        assert_eq!(r.next_f64(), 0.6011037519201636);
        assert_eq!(r.next_f64(), 0.44829055899754167);
        assert_eq!(r.next_f64(), 0.8524657934904099);
    }
}
