// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The engine's only source of randomness.
//!
//! Everything random in Vyvræl (Zobrist keys, magic-number search, test
//! fixtures, weakened play and the `Variety` choice) draws from the generator
//! in this module, so that results are reproducible from a seed on every
//! platform.
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

/// Stream indices of [`DEFAULT_SEED`] reserved by engine subsystems.
///
/// Each subsystem that needs deterministic random numbers takes its own
/// stream with `Rng::stream(DEFAULT_SEED, stream::NAME)`, so adding a new
/// consumer never shifts the numbers an existing one receives. Indices are
/// never reused or renumbered.
pub mod stream {
    /// Zobrist hashing keys (`board::zobrist`).
    pub const ZOBRIST: u64 = 0;
    /// Magic-number search for slider attack tables (`attacks::sliders`).
    pub const MAGICS: u64 = 1;
    /// Randomness that changes how strongly the engine plays: weakened move
    /// choice under `UCI_LimitStrength` and the varied opening move under
    /// `Variety` (`uci`). Unlike the others, this stream is taken from the
    /// seed the UCI options select, which is not the default seed unless
    /// `Seed` says so.
    pub const STRENGTH: u64 = 2;
}

/// The largest seed a user can give: 2147483647, the largest signed 32-bit
/// number. Cute Chess keeps spin values as signed 32-bit numbers, so a
/// larger one could not be typed into it. Seeds taken from entropy stay
/// between 1 and this, so that any logged seed can be given back.
pub const SEED_MAX: u32 = 0x7FFF_FFFF;

/// A seed taken from the operating system's randomness, from 1 to
/// [`SEED_MAX`].
///
/// **This is the only place in the project that reads ambient entropy**, and
/// it has exactly one permitted caller: seeding strength-affecting
/// randomness when the UCI option `Seed` is 0, its default. Everything
/// else must be reproducible from [`DEFAULT_SEED`] or a seed the user gave.
/// The caller takes one seed per game and logs it, so even such a game can
/// be replayed afterwards by giving the logged value as `Seed`. It is never
/// 0, because `Seed` 0 means "take one from entropy".
///
/// The standard library's `RandomState` is the source: every new one gets
/// keys derived from operating-system randomness. That keeps the engine
/// free of dependencies.
#[must_use]
pub fn entropy_seed() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    seed_from_bits(RandomState::new().build_hasher().finish())
}

/// Turns 64 random bits into a seed from 1 to [`SEED_MAX`]: mixed, then
/// reduced. The reduction's bias is about one in four billion, far too
/// small to matter for choosing a seed.
const fn seed_from_bits(bits: u64) -> u32 {
    (mix64(bits) % SEED_MAX as u64) as u32 + 1
}

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

    /// Creates an independent generator for stream `index` of `base_seed`.
    ///
    /// Use this wherever several generators are needed (threads, self-play
    /// games, subsystems) instead of sharing one generator. Distinct indices
    /// always give distinct starting states for the same base seed: the state
    /// is `mix64(k ^ mix64(index + GOLDEN_GAMMA))`, where `k` is the first
    /// output of `Rng::new(base_seed)`, and `mix64` is a bijection.
    #[must_use]
    pub const fn stream(base_seed: u32, index: u64) -> Self {
        let key = Self::new(base_seed).next_u64();
        Self::from_state(mix64(key ^ mix64(index.wrapping_add(GOLDEN_GAMMA))))
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
    /// Returns a uniformly distributed integer in `0..bound`.
    ///
    /// Uses Lemire's multiply-and-reject method, so the result is unbiased
    /// for every bound. Usually consumes one 64-bit output, occasionally more.
    ///
    /// # Panics
    ///
    /// Panics if `bound` is zero.
    pub const fn next_below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "next_below: bound must be non-zero");
        let mut product = (self.next_u64() as u128) * (bound as u128);
        if (product as u64) < bound {
            // 2^64 mod bound: low halves below this are over-represented.
            let threshold = bound.wrapping_neg() % bound;
            while (product as u64) < threshold {
                product = (self.next_u64() as u128) * (bound as u128);
            }
        }
        (product >> 64) as u64
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

    /// Golden values for derived streams of the default seed, computed with
    /// an independent implementation.
    #[test]
    fn golden_streams() {
        let golden: [(u64, [u64; 3]); 4] = [
            (
                0,
                [
                    0xCCC9_1CE0_CC50_1A61,
                    0x0A28_A258_6506_921E,
                    0x48E0_21E3_BC01_D377,
                ],
            ),
            (
                1,
                [
                    0xD119_0E69_43F2_5DBC,
                    0x89C2_D801_6FB9_0F9B,
                    0x6D8A_B0AE_5D5A_2A23,
                ],
            ),
            (
                2,
                [
                    0x924E_45DE_572A_5660,
                    0x506D_A39B_AB71_4DD7,
                    0x2A91_45FE_1058_8711,
                ],
            ),
            (
                u64::MAX,
                [
                    0x00BC_B3C7_C27B_9FBF,
                    0x743B_C185_E621_063D,
                    0xBDE4_9162_AD40_6CE2,
                ],
            ),
        ];
        for (index, expected) in golden {
            let mut rng = Rng::stream(DEFAULT_SEED, index);
            assert_eq!(take(&mut rng, 3), expected, "stream {index}");
        }
    }

    #[test]
    fn streams_are_distinct_from_each_other_and_from_the_base() {
        let mut states = vec![Rng::new(DEFAULT_SEED)];
        states.extend((0..1000).map(|i| Rng::stream(DEFAULT_SEED, i)));
        let mut firsts: Vec<u64> = states.iter_mut().map(Rng::next_u64).collect();
        firsts.sort_unstable();
        firsts.dedup();
        assert_eq!(firsts.len(), 1001);
    }

    #[test]
    fn stream_depends_on_base_seed() {
        assert_ne!(
            Rng::stream(DEFAULT_SEED, 0),
            Rng::stream(DEFAULT_SEED + 1, 0)
        );
    }

    /// Golden values computed with an independent implementation.
    #[test]
    fn golden_next_below() {
        let mut rng = Rng::default();
        let dice: Vec<u64> = (0..12).map(|_| rng.next_below(6)).collect();
        assert_eq!(dice, [4, 5, 1, 5, 3, 1, 3, 2, 1, 2, 4, 5]);

        let mut rng = Rng::default();
        let draws: Vec<u64> = (0..6).map(|_| rng.next_below(1000)).collect();
        assert_eq!(draws, [676, 981, 277, 983, 558, 218]);
    }

    #[test]
    fn next_below_stays_in_range() {
        let mut rng = Rng::default();
        let bounds = [1, 2, 3, 7, 64, 1000, (1 << 63) + 1, u64::MAX];
        for bound in bounds {
            for _ in 0..2000 {
                assert!(rng.next_below(bound) < bound);
            }
        }
    }

    #[test]
    fn next_below_one_is_always_zero() {
        let mut rng = Rng::default();
        assert!((0..100).all(|_| rng.next_below(1) == 0));
    }

    #[test]
    fn next_below_is_roughly_uniform() {
        let mut rng = Rng::default();
        let mut buckets = [0u32; 6];
        for _ in 0..60_000 {
            buckets[rng.next_below(6) as usize] += 1;
        }
        // Expected 10 000 each; 4 % is about 4.4 standard deviations.
        assert!(buckets.iter().all(|&n| (9_600..=10_400).contains(&n)));
    }

    #[test]
    #[should_panic(expected = "bound must be non-zero")]
    fn next_below_zero_panics() {
        Rng::default().next_below(0);
    }

    #[test]
    fn usable_in_const_context() {
        const FIRST: u64 = {
            let mut rng = Rng::new(DEFAULT_SEED);
            rng.next_u64()
        };
        assert_eq!(FIRST, 0xAD20_D299_6597_755F);
    }

    #[test]
    fn stream_indices_are_distinct() {
        let indices = [stream::ZOBRIST, stream::MAGICS, stream::STRENGTH];
        assert_eq!(indices, [0, 1, 2], "indices are never renumbered");
    }

    /// The strength stream of a seed is pinned like the others, so that a
    /// game replayed from a logged seed stays replayable.
    #[test]
    fn golden_strength_stream() {
        let mut rng = Rng::stream(DEFAULT_SEED, stream::STRENGTH);
        // Computed with an independent implementation of SplitMix64.
        let golden = [
            0x924E_45DE_572A_5660,
            0x506D_A39B_AB71_4DD7,
            0x2A91_45FE_1058_8711,
        ];
        assert_eq!(take(&mut rng, 3), golden);
    }

    /// Entropy seeds are not reproducible by design, and no test may rely on
    /// two of them differing. What can be tested is that each lies where a
    /// user's `Seed` can: from 1 to `SEED_MAX`, never the 0 that means
    /// "from entropy".
    #[test]
    fn entropy_seeds_are_valid_seeds() {
        for _ in 0..32 {
            let seed = entropy_seed();
            assert!((1..=SEED_MAX).contains(&seed), "{seed}");
        }
    }

    /// The mapping from random bits to a seed: whatever the bits, the seed
    /// is from 1 to `SEED_MAX`, and the same bits always give the same seed.
    #[test]
    fn random_bits_become_a_valid_seed() {
        assert_eq!(SEED_MAX, 2_147_483_647);
        assert_eq!(SEED_MAX, i32::MAX as u32);
        for bits in [
            0,
            1,
            2,
            u64::from(SEED_MAX),
            u64::MAX,
            u64::from(DEFAULT_SEED),
        ] {
            let seed = seed_from_bits(bits);
            assert!((1..=SEED_MAX).contains(&seed), "{bits}: {seed}");
            assert_eq!(seed_from_bits(bits), seed);
        }
    }
}
