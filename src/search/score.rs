// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Search scores: the range of values and how checkmate is encoded in it.

use crate::eval::Score;

/// The deepest the search can go, in plies from the root. Recursion and the
/// principal-variation table are sized by it.
pub const MAX_PLY: usize = 128;

/// A bound no real score reaches; the initial alpha-beta window is
/// `-INFINITY..INFINITY`.
pub const INFINITY: Score = 32_000;

/// The score for delivering checkmate at the root. A mate found deeper is
/// worth a little less, one point per ply, so the search prefers the
/// quickest mate and, when losing, the slowest defeat.
pub const MATE: Score = 31_000;

/// The score of a drawn position.
pub const DRAW: Score = 0;

/// Scores at least this large (in absolute value) are mate scores, not
/// evaluations. Evaluations stay far below it.
pub const MATE_BOUND: Score = MATE - MAX_PLY as Score;

// Checked when the crate is compiled: the ranges nest, and evaluations (a few
// thousand centipawns at most) stay far below mate scores.
const _: () = assert!(MATE < INFINITY);
const _: () = assert!(MATE_BOUND > 20_000);

/// The score of a position in which the side to move is checkmated, `ply`
/// plies from the root.
#[must_use]
pub const fn mated_in(ply: usize) -> Score {
    -MATE + ply as Score
}

/// The score of a position from which the side to move gives checkmate,
/// reaching the mated position `ply` plies from the root.
#[must_use]
pub const fn mate_in(ply: usize) -> Score {
    MATE - ply as Score
}

/// Whether `score` says one side is being checkmated by force.
#[must_use]
pub const fn is_mate_score(score: Score) -> bool {
    score.abs() >= MATE_BOUND
}

/// For a mate score, the number of **moves** (not plies) until mate, as UCI
/// reports it: positive when the side to move mates, negative when it is
/// mated. `None` for an ordinary score.
///
/// "Mate in 1" means the side to move mates with its next move.
#[must_use]
pub const fn moves_to_mate(score: Score) -> Option<i32> {
    if !is_mate_score(score) {
        return None;
    }
    let plies = MATE - score.abs();
    // The winner moves on odd plies, so round up; the loser is mated after
    // an even number of plies.
    let moves = (plies + 1) / 2;
    Some(if score > 0 { moves } else { -moves })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_nest_with_room_to_spare() {
        assert!(mate_in(MAX_PLY) >= MATE_BOUND);
        assert!(mated_in(MAX_PLY) <= -MATE_BOUND);
        assert_eq!(DRAW, 0);
    }

    #[test]
    fn mate_scores_mirror_each_other() {
        for ply in 0..MAX_PLY {
            assert_eq!(mated_in(ply), -mate_in(ply));
            assert!(is_mate_score(mate_in(ply)) && is_mate_score(mated_in(ply)));
        }
    }

    /// A quicker mate scores higher, and a slower defeat scores higher.
    #[test]
    fn sooner_mates_and_later_defeats_are_preferred() {
        assert!(mate_in(1) > mate_in(3));
        assert!(mate_in(3) > mate_in(5));
        assert!(mated_in(4) > mated_in(2));
        assert!(mated_in(2) > mated_in(0));
        assert!(mate_in(MAX_PLY - 1) > 10_000);
    }

    #[test]
    fn ordinary_scores_are_not_mate_scores() {
        for score in [
            0,
            1,
            -1,
            900,
            -2_500,
            20_000,
            MATE_BOUND - 1,
            -(MATE_BOUND - 1),
        ] {
            assert!(!is_mate_score(score), "{score}");
            assert_eq!(moves_to_mate(score), None, "{score}");
        }
    }

    #[test]
    fn moves_to_mate_counts_full_moves() {
        // The side to move mates with its next move: the mated position is 1
        // ply away.
        assert_eq!(moves_to_mate(mate_in(1)), Some(1));
        // Move, reply, mating move.
        assert_eq!(moves_to_mate(mate_in(3)), Some(2));
        assert_eq!(moves_to_mate(mate_in(5)), Some(3));
        // The side to move is already checkmated.
        assert_eq!(moves_to_mate(mated_in(0)), Some(0));
        // Any move it makes is answered by mate.
        assert_eq!(moves_to_mate(mated_in(2)), Some(-1));
        assert_eq!(moves_to_mate(mated_in(4)), Some(-2));
    }
}
