//! Deterministic RNG: SplitMix64.
//!
//! The connectome generator draws motif-scoped streams from this so that a
//! given `(scale, seed)` pair reproduces bit-identical graphs on every
//! platform — the same property the repository's genesis-reproducibility
//! gate demands. Not a CSPRNG; used only for topology sampling and stimulus
//! expansion, never for keys.

/// SplitMix64 state (Stafford / Vigna splittable generator).
#[derive(Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Seeds the stream. `const` so region tags can be composed at compile time.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform draw in `0..n`. Callers guarantee `n > 0` (region sizes are
    /// all `>= 1` by construction; debug-asserted here).
    pub fn below(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0);
        (self.next_u64() % u64::from(n)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_reference_sequence() {
        // First outputs for seed 0, from the reference C implementation.
        let mut r = SplitMix64::new(0);
        let want: [u64; 4] = [
            0xe220_a839_7b1d_cdaf,
            0x6e78_9e6a_a1b9_65f4,
            0x06c4_5d18_8009_454f,
            0xf88b_b8a8_724c_81ec,
        ];
        for w in want {
            assert_eq!(r.next_u64(), w);
        }
    }

    #[test]
    fn below_is_in_range_and_deterministic() {
        let mut a = SplitMix64::new(0xB0DF17);
        let mut b = SplitMix64::new(0xB0DF17);
        for _ in 0..10_000 {
            let x = a.below(97);
            assert!(x < 97);
            assert_eq!(x, b.below(97));
        }
    }
}
