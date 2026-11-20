// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Recognising drawn positions without searching them.

use crate::board::{PieceKind, Position};

/// Whether neither side has enough material to ever deliver checkmate: bare
/// kings, or a single knight or bishop on the board.
///
/// Deliberately conservative. Other dead positions exist (for example
/// bishops on one colour only), but claiming a draw wrongly is far worse
/// than missing one, so only the certain cases are covered.
#[must_use]
pub fn insufficient_material(pos: &Position) -> bool {
    let can_mate_alone =
        pos.pieces(PieceKind::Pawn) | pos.pieces(PieceKind::Rook) | pos.pieces(PieceKind::Queen);
    let minors = pos.pieces(PieceKind::Knight) | pos.pieces(PieceKind::Bishop);
    can_mate_alone.is_empty() && minors.count() <= 1
}

/// Whether `pos` has already occurred among `earlier`, the hashes of the
/// positions that came before it, oldest first.
///
/// One earlier occurrence is enough. The rules ask for three occurrences in
/// total before a draw can be claimed, but a position that can be repeated
/// once can be repeated again, so search treats the first repetition as a
/// draw. That is the usual practice and keeps the search from walking in
/// circles.
///
/// Only positions since the last capture or pawn move can match, which the
/// halfmove clock counts, and only those with the same side to move, which
/// are an even number of plies back.
#[must_use]
pub fn is_repetition(pos: &Position, earlier: &[u64]) -> bool {
    let reach = usize::from(pos.halfmove_clock()).min(earlier.len());
    earlier[earlier.len() - reach..]
        .iter()
        .rev()
        .skip(1)
        .step_by(2)
        .any(|&key| key == pos.hash())
}

/// Whether the fifty-move rule applies: a hundred plies without a capture or
/// a pawn move.
///
/// A checkmate delivered by the hundredth ply still wins; callers must check
/// for that before treating the position as drawn.
#[must_use]
pub const fn fifty_moves_reached(pos: &Position) -> bool {
    pos.halfmove_clock() >= 100
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    /// Plays `moves` from `pos`, returning the hash of each position passed
    /// through (not including the final one).
    fn play(pos: &mut Position, moves: &[&str]) -> Vec<u64> {
        let mut keys = Vec::new();
        for text in moves {
            keys.push(pos.hash());
            let mv = pos.parse_move(text).unwrap();
            let _undo = pos.make_move(mv);
        }
        keys
    }

    #[test]
    fn material_that_cannot_mate() {
        for text in [
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/3NK3 w - - 0 1",
            "4k3/8/8/8/8/8/8/3BK3 w - - 0 1",
            "3nk3/8/8/8/8/8/8/4K3 b - - 0 1",
            "3bk3/8/8/8/8/8/8/4K3 b - - 0 1",
        ] {
            assert!(insufficient_material(&fen(text)), "{text}");
        }
    }

    #[test]
    fn material_that_might_mate_is_not_claimed() {
        for text in [
            "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/3RK3 w - - 0 1",
            "4k3/8/8/8/8/8/8/3QK3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2NNK3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2BNK3 w - - 0 1",
            "3bk3/8/8/8/8/8/8/3BK3 w - - 0 1",
            "3nk3/8/8/8/8/8/8/3NK3 w - - 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ] {
            assert!(!insufficient_material(&fen(text)), "{text}");
        }
    }

    #[test]
    fn knights_out_and_back_repeat_the_position() {
        let mut pos = Position::startpos();
        let keys = play(&mut pos, &["g1f3", "g8f6", "f3g1", "f6g8"]);
        assert_eq!(pos.hash(), Position::startpos().hash());
        assert!(is_repetition(&pos, &keys));
        // Two plies earlier nothing had repeated yet.
        let mut partway = Position::startpos();
        let keys = play(&mut partway, &["g1f3", "g8f6"]);
        assert!(!is_repetition(&partway, &keys));
    }

    #[test]
    fn the_other_side_to_move_is_a_different_position() {
        // Same placement as the start, but reached with Black to move, by
        // White losing a tempo with the knight. An odd number of plies back
        // never counts.
        let mut pos = Position::startpos();
        let keys = play(&mut pos, &["g1f3", "g8f6", "f3g1"]);
        assert!(!is_repetition(&pos, &keys));
        assert!(!keys.contains(&pos.hash()));
    }

    #[test]
    fn a_pawn_move_or_capture_cuts_off_earlier_positions() {
        // A key equal to the current hash sits further back than the
        // halfmove clock reaches, so it cannot be a repetition.
        let mut pos = Position::startpos();
        let mut keys = play(&mut pos, &["g1f3", "g8f6", "f3g1", "f6g8"]);
        assert!(is_repetition(&pos, &keys));
        keys.extend(play(
            &mut pos,
            &["e2e3", "e7e6", "g1f3", "g8f6", "f3g1", "f6g8"],
        ));
        assert_eq!(pos.halfmove_clock(), 4);
        // The position after 1...e6 recurs: that is within reach.
        assert!(is_repetition(&pos, &keys));
        // One ply short of the repeat, with the clock at 3: not yet.
        let mut short = Position::startpos();
        let keys = play(&mut short, &["e2e3", "e7e6", "g1f3", "g8f6", "f3g1"]);
        assert!(!is_repetition(&short, &keys));
    }

    #[test]
    fn no_history_means_no_repetition() {
        assert!(!is_repetition(&Position::startpos(), &[]));
        let quiet = fen("4k3/8/8/8/8/8/8/R3K3 w - - 40 60");
        assert!(!is_repetition(&quiet, &[]));
        // A clock larger than the history must not read out of bounds.
        assert!(!is_repetition(&quiet, &[1, 2, 3]));
    }

    #[test]
    fn fifty_move_rule_counts_plies() {
        assert!(!fifty_moves_reached(&fen(
            "4k3/8/8/8/8/8/8/R3K3 w - - 99 80"
        )));
        assert!(fifty_moves_reached(&fen(
            "4k3/8/8/8/8/8/8/R3K3 w - - 100 80"
        )));
        assert!(fifty_moves_reached(&fen(
            "4k3/8/8/8/8/8/8/R3K3 w - - 140 99"
        )));
    }
}
