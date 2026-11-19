// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Tapered scores: a middlegame value and an endgame value, blended by the
//! game phase.

use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use super::{MAX_PHASE, Score};

/// A pair of scores for the same feature: what it is worth in the
/// middlegame and what it is worth in the endgame.
///
/// Many features change value as pieces come off. A king in the centre is a
/// liability with queens on and an asset in a pawn ending. Keeping both
/// numbers and blending them by the game phase lets the evaluation change
/// smoothly instead of jumping at an arbitrary "endgame starts here" line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tapered {
    /// Value with all pieces on the board.
    pub mg: Score,
    /// Value with only kings and pawns left.
    pub eg: Score,
}

impl Tapered {
    /// Worth nothing in either phase.
    pub const ZERO: Tapered = Tapered::new(0, 0);

    /// Creates a pair from its middlegame and endgame values.
    #[must_use]
    pub const fn new(mg: Score, eg: Score) -> Tapered {
        Tapered { mg, eg }
    }

    /// A pair worth the same in both phases.
    #[must_use]
    pub const fn flat(value: Score) -> Tapered {
        Tapered::new(value, value)
    }

    /// The single score for a position with game phase `phase` (see
    /// [`game_phase`]): a weighted average that is exactly `mg` at
    /// [`MAX_PHASE`] and exactly `eg` at phase 0.
    ///
    /// The division rounds towards zero, so negating the pair negates the
    /// result exactly. That keeps the evaluation symmetric between White and
    /// Black.
    ///
    /// # Panics
    ///
    /// Panics if `phase` is outside `0..=MAX_PHASE`.
    ///
    /// [`game_phase`]: super::game_phase
    #[must_use]
    pub const fn blend(self, phase: i32) -> Score {
        assert!(
            phase >= 0 && phase <= MAX_PHASE,
            "Tapered::blend: phase out of range"
        );
        (self.mg * phase + self.eg * (MAX_PHASE - phase)) / MAX_PHASE
    }
}

impl Add for Tapered {
    type Output = Tapered;

    fn add(self, rhs: Tapered) -> Tapered {
        Tapered::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }
}

impl Sub for Tapered {
    type Output = Tapered;

    fn sub(self, rhs: Tapered) -> Tapered {
        Tapered::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl Neg for Tapered {
    type Output = Tapered;

    fn neg(self) -> Tapered {
        Tapered::new(-self.mg, -self.eg)
    }
}

impl AddAssign for Tapered {
    fn add_assign(&mut self, rhs: Tapered) {
        *self = *self + rhs;
    }
}

impl SubAssign for Tapered {
    fn sub_assign(&mut self, rhs: Tapered) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_hits_the_endpoints_exactly() {
        let pair = Tapered::new(137, -42);
        assert_eq!(pair.blend(MAX_PHASE), 137);
        assert_eq!(pair.blend(0), -42);
    }

    #[test]
    fn blend_is_a_weighted_average() {
        let pair = Tapered::new(240, 0);
        assert_eq!(pair.blend(12), 120);
        assert_eq!(pair.blend(6), 60);
        assert_eq!(pair.blend(18), 180);
        assert_eq!(pair.blend(1), 10);
        // (100 * 8 + 40 * 16) / 24 = 60.
        assert_eq!(Tapered::new(100, 40).blend(8), 60);
    }

    #[test]
    fn a_flat_pair_blends_to_itself_at_every_phase() {
        for phase in 0..=MAX_PHASE {
            assert_eq!(Tapered::flat(77).blend(phase), 77);
            assert_eq!(Tapered::ZERO.blend(phase), 0);
        }
    }

    #[test]
    fn blend_moves_monotonically_between_the_endpoints() {
        let rising = Tapered::new(90, -30);
        for phase in 0..MAX_PHASE {
            assert!(rising.blend(phase) <= rising.blend(phase + 1));
        }
    }

    /// Rounding towards zero makes blending odd: `blend(-x) == -blend(x)`,
    /// including when the average is not a whole number.
    #[test]
    fn negation_commutes_with_blending() {
        let pairs = [
            Tapered::new(1, 0),
            Tapered::new(0, 1),
            Tapered::new(7, -5),
            Tapered::new(-13, 29),
            Tapered::new(101, 100),
        ];
        for pair in pairs {
            for phase in 0..=MAX_PHASE {
                assert_eq!((-pair).blend(phase), -pair.blend(phase), "{pair:?} {phase}");
            }
        }
        // 1 * 1 / 24 rounds to 0 from both sides.
        assert_eq!(Tapered::new(1, 0).blend(1), 0);
        assert_eq!(Tapered::new(-1, 0).blend(1), 0);
    }

    #[test]
    fn arithmetic_is_componentwise() {
        let a = Tapered::new(10, 20);
        let b = Tapered::new(3, -4);
        assert_eq!(a + b, Tapered::new(13, 16));
        assert_eq!(a - b, Tapered::new(7, 24));
        assert_eq!(-a, Tapered::new(-10, -20));
        let mut c = a;
        c += b;
        c -= a;
        assert_eq!(c, b);
        assert_eq!(Tapered::default(), Tapered::ZERO);
    }

    #[test]
    #[should_panic(expected = "phase out of range")]
    fn blend_rejects_a_phase_above_the_maximum() {
        let _ = Tapered::flat(1).blend(MAX_PHASE + 1);
    }

    #[test]
    #[should_panic(expected = "phase out of range")]
    fn blend_rejects_a_negative_phase() {
        let _ = Tapered::flat(1).blend(-1);
    }
}
