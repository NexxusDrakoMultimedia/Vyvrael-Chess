// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! A score shown as chances of a win, a draw and a loss, for the UCI option
//! `UCI_ShowWDL`.
//!
//! **This is a model, not a measurement.** The three numbers are a fixed
//! function of the centipawn score, of the shape engines usually use for
//! the purpose: one logistic curve for a win and its mirror image for a
//! loss, the draw being what is left.
//!
//! ```text
//! win  = 1000 / (1 + e^((HALF - score) / SPREAD))
//! loss = 1000 / (1 + e^((HALF + score) / SPREAD))
//! draw = 1000 - win - loss
//! ```
//!
//! The two constants were chosen, not fitted: nobody has counted how
//! Vyvræl's games end from each score, so the numbers say nothing measured
//! about its results against any opponent, and they would be off in scale
//! after any change to what a centipawn means in the evaluation. They are
//! there because GUIs and lichess-bot can show them.

use crate::eval::Score;
use crate::search::moves_to_mate;

/// The score, in centipawns, at which the model gives a win half the time.
pub const WDL_HALF: Score = 200;

/// How many centipawns stretch the model's curves: at `WDL_HALF` plus this
/// much a win is given about 73% of the time, at `WDL_HALF` minus it 27%.
pub const WDL_SPREAD: Score = 100;

/// `score` (for the side to move) as chances per thousand of a win, a draw
/// and a loss for that side, in that order. They add up to 1000.
///
/// A forced mate is a certain win or loss. Everything else follows the
/// model in this module's description, which is a fixed mapping and not a
/// measured win rate.
#[must_use]
pub fn wdl(score: Score) -> (u32, u32, u32) {
    if moves_to_mate(score).is_some() {
        return if score > 0 {
            (1000, 0, 0)
        } else {
            (0, 0, 1000)
        };
    }
    let win = per_mille(f64::from(score));
    let loss = per_mille(-f64::from(score));
    (win, 1000_u32.saturating_sub(win + loss), loss)
}

/// The model's chance per thousand of a win at `score`, rounded to the
/// nearest whole number.
fn per_mille(score: f64) -> u32 {
    // Beyond 40 either way the curve is flat to far more digits than are
    // shown, and the clamp keeps `exp` within the range it is exact for.
    let exponent = ((f64::from(WDL_HALF) - score) / f64::from(WDL_SPREAD)).clamp(-40.0, 40.0);
    (1000.0 / (1.0 + exp(exponent))).round() as u32
}

/// `e` to the power `x`, for `x` from -40 to 40.
///
/// The standard library's `exp` may differ in its last digit from one
/// platform to another, and a figure that sits on a rounding boundary would
/// then be shown differently. This one uses only additions, multiplications
/// and divisions, which give the same result everywhere: the power is
/// divided by 1024, the series for `e^x` is summed (twelve terms are more
/// than enough for so small a power), and the sum is squared ten times.
fn exp(x: f64) -> f64 {
    let small = x / 1024.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=12 {
        term *= small / f64::from(n);
        sum += term;
    }
    for _ in 0..10 {
        sum *= sum;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{MATE, mate_in, mated_in};

    #[test]
    fn the_exponential_agrees_with_the_standard_one() {
        for tenths in -400..=400 {
            let x = f64::from(tenths) / 10.0;
            let (ours, theirs) = (exp(x), x.exp());
            assert!(
                ((ours - theirs) / theirs).abs() < 1e-12,
                "{x}: {ours} {theirs}"
            );
        }
    }

    /// The model's figures at a few scores, worked out by hand from the
    /// formula: 1000 / (1 + e^2) is 119.2, 1000 / (1 + e) is 268.9, and so
    /// on.
    #[test]
    fn the_model_at_some_scores() {
        assert_eq!(wdl(0), (119, 762, 119));
        assert_eq!(wdl(100), (269, 684, 47));
        assert_eq!(wdl(200), (500, 482, 18));
        assert_eq!(wdl(300), (731, 262, 7));
        assert_eq!(wdl(600), (982, 18, 0));
        assert_eq!(wdl(-100), (47, 684, 269));
    }

    #[test]
    fn the_three_always_add_up_and_mirror_each_other() {
        let mut last_win = 0;
        for score in -3_000..=3_000 {
            let (win, draw, loss) = wdl(score);
            assert_eq!(win + draw + loss, 1000, "{score}");
            assert_eq!(wdl(-score), (loss, draw, win), "{score}");
            // A better score never gives a smaller chance of winning.
            assert!(win >= last_win, "{score}");
            last_win = win;
        }
        // Far from zero the curves are flat, with nothing left over.
        assert_eq!(wdl(3_000), (1000, 0, 0));
        assert_eq!(wdl(MATE / 2), (1000, 0, 0));
        assert_eq!(wdl(-MATE / 2), (0, 0, 1000));
    }

    #[test]
    fn a_forced_mate_is_certain() {
        for plies in [1, 2, 7, 40] {
            assert_eq!(wdl(mate_in(plies)), (1000, 0, 0));
            assert_eq!(wdl(mated_in(plies)), (0, 0, 1000));
        }
        assert_eq!(wdl(mated_in(0)), (0, 0, 1000));
    }
}
