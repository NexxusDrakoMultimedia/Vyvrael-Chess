// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Static evaluation: a number saying how good a position looks without
//! searching any further.
//!
//! See `docs/ARCHITECTURE.md` ("Evaluation") for the design.

mod material;
mod phase;
mod tapered;

pub use material::{Material, material_balance, material_of, piece_value};
pub use phase::{MAX_PHASE, game_phase, phase_weight};
pub use tapered::Tapered;

use crate::board::{Color, Position};

/// An evaluation in centipawns: 100 is roughly the value of a pawn.
///
/// Positive is good for whichever side the score is "relative to", which is
/// always stated where a score is produced.
pub type Score = i32;

/// Something that can evaluate a position.
///
/// Search talks to evaluation only through this trait, so the hand-crafted
/// evaluation and a neural network are interchangeable and can be compared
/// by playing them against each other.
pub trait Evaluator {
    /// The static evaluation of `pos` **from the point of view of the side
    /// to move**: positive means the side to move stands better.
    ///
    /// This is the convention negamax search needs. The position should be
    /// "quiet" for the number to mean much; resolving captures first is
    /// search's job (quiescence search), not the evaluator's.
    fn evaluate(&self, pos: &Position) -> Score;

    /// A short name for logs and for comparing evaluators.
    fn name(&self) -> &'static str;
}

/// Converts a score counted as "White minus Black" into the score from the
/// point of view of `side_to_move`.
///
/// Evaluators add up what each side has, which naturally gives a
/// White-relative number; this turns it into what [`Evaluator::evaluate`]
/// must return.
#[must_use]
pub const fn relative_to(side_to_move: Color, white_minus_black: Score) -> Score {
    match side_to_move {
        Color::White => white_minus_black,
        Color::Black => -white_minus_black,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in evaluator that always says White is a pawn up.
    struct WhiteIsAPawnUp;

    impl Evaluator for WhiteIsAPawnUp {
        fn evaluate(&self, pos: &Position) -> Score {
            relative_to(pos.side_to_move(), 100)
        }

        fn name(&self) -> &'static str {
            "white-is-a-pawn-up"
        }
    }

    #[test]
    fn relative_to_flips_the_sign_for_black() {
        assert_eq!(relative_to(Color::White, 35), 35);
        assert_eq!(relative_to(Color::Black, 35), -35);
        assert_eq!(relative_to(Color::Black, -120), 120);
        assert_eq!(relative_to(Color::White, 0), relative_to(Color::Black, 0));
    }

    /// Search will hold the evaluator as a trait object so it can be swapped
    /// at run time (hand-crafted or network).
    #[test]
    fn evaluators_can_be_used_as_trait_objects() {
        let evaluator: Box<dyn Evaluator> = Box::new(WhiteIsAPawnUp);
        let white = Position::startpos();
        let black =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();
        assert_eq!(evaluator.evaluate(&white), 100);
        assert_eq!(evaluator.evaluate(&black), -100);
        assert_eq!(evaluator.name(), "white-is-a-pawn-up");
    }
}
