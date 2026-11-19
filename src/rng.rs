// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The engine's only source of randomness.
//!
//! Everything random in Vyvræl (Zobrist keys, magic-number search, test
//! fixtures, and later any randomised play) draws from the generator in this
//! module, so that results are reproducible from a seed on every platform.
//! The rules are in `AGENTS.md` ("Randomness") and the design is in
//! `docs/ARCHITECTURE.md` ("RNG design").
//!
//! The generator is SplitMix64 (Steele, Lea and Flood, 2014). It is
//! implemented here rather than taken from a crate so that its output can
//! never change underneath generated tables or golden tests.

/// The default 32-bit seed for all randomness in the project.
///
/// Reference this constant; never repeat the literal elsewhere.
pub const DEFAULT_SEED: u32 = 0x3103_1996;

/// SplitMix64's state increment (the 64-bit golden-ratio constant).
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// SplitMix64's output mixing function: a bijection on `u64`.
///
/// Useful on its own for hashing an integer into a well-distributed value.
#[must_use]
pub const fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A SplitMix64 pseudo-random number generator.
///
/// Deterministic: the same seed always yields the same sequence. All methods
/// are `const fn`, so tables can be generated at compile time.
///
/// This type is deliberately not `Copy`, so a generator cannot be duplicated
/// by accident and replay the same numbers twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Creates a generator from a 32-bit seed (zero-extended to the state).
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self::from_state(seed as u64)
    }

    /// Creates a generator from a full 64-bit state.
    #[must_use]
    pub const fn from_state(state: u64) -> Self {
        Self { state }
    }

    /// Returns the next 64 random bits.
    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix64(self.state)
    }

    /// Returns the next 32 random bits (the high half of a 64-bit output).
    pub const fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

impl Default for Rng {
    /// A generator seeded with [`DEFAULT_SEED`].
    fn default() -> Self {
        Self::new(DEFAULT_SEED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(rng: &mut Rng, n: usize) -> Vec<u64> {
        (0..n).map(|_| rng.next_u64()).collect()
    }

    #[test]
    fn default_seed_value() {
        assert_eq!(DEFAULT_SEED, 0x3103_1996);
    }

    /// The published SplitMix64 reference vector for seed 1234567.
    #[test]
    fn golden_reference_vector() {
        let mut rng = Rng::from_state(1_234_567);
        assert_eq!(
            take(&mut rng, 5),
            [
                6_457_827_717_110_365_317,
                3_203_168_211_198_807_973,
                9_817_491_932_198_370_423,
                4_593_380_528_125_082_431,
                16_408_922_859_458_223_821,
            ]
        );
    }

    #[test]
    fn golden_seed_zero() {
        let mut rng = Rng::new(0);
        assert_eq!(
            take(&mut rng, 4),
            [
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F,
                0xF88B_B8A8_724C_81EC,
            ]
        );
    }

    /// Golden values for the project's default seed, computed with an
    /// independent implementation of SplitMix64.
    #[test]
    fn golden_default_seed() {
        let mut rng = Rng::default();
        assert_eq!(
            take(&mut rng, 6),
            [
                0xAD20_D299_6597_755F,
                0xFB44_9459_5280_B961,
                0x4726_29A4_BE12_259C,
                0xFBC1_6F1B_974F_7D0C,
                0x8EE1_13F8_5DBC_A720,
                0x37FE_3B5F_60D3_581E,
            ]
        );
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(DEFAULT_SEED);
        let mut b = Rng::new(DEFAULT_SEED);
        assert_eq!(take(&mut a, 1000), take(&mut b, 1000));
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(DEFAULT_SEED);
        let mut b = Rng::new(DEFAULT_SEED + 1);
        assert_ne!(take(&mut a, 8), take(&mut b, 8));
    }

    #[test]
    fn new_zero_extends_the_seed() {
        assert_eq!(
            Rng::new(DEFAULT_SEED),
            Rng::from_state(u64::from(DEFAULT_SEED))
        );
        assert_eq!(Rng::new(u32::MAX), Rng::from_state(0xFFFF_FFFF));
    }

    #[test]
    fn next_u32_is_high_half() {
        let mut a = Rng::default();
        let mut b = Rng::default();
        for _ in 0..16 {
            assert_eq!(u64::from(a.next_u32()), b.next_u64() >> 32);
        }
    }

    #[test]
    fn usable_in_const_context() {
        const FIRST: u64 = {
            let mut rng = Rng::new(DEFAULT_SEED);
            rng.next_u64()
        };
        assert_eq!(FIRST, 0xAD20_D299_6597_755F);
    }
}
