// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Game phase: how far the game has moved from opening to endgame.

use crate::board::{PieceKind, Position};

/// The phase of a position with all pieces on the board: the opening and
/// middlegame end of the scale.
pub const MAX_PHASE: i32 = 24;

/// How much one piece of `kind` contributes to the phase. Pawns and kings
/// contribute nothing: the phase measures the pieces that make attacks on
/// the king possible.
#[must_use]
pub const fn phase_weight(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Knight | PieceKind::Bishop => 1,
        PieceKind::Rook => 2,
        PieceKind::Queen => 4,
        PieceKind::Pawn | PieceKind::King => 0,
    }
}

/// The game phase of `pos`, from `0` (only kings and pawns left: a pure
/// endgame) to [`MAX_PHASE`] (all pieces still on the board).
///
/// It is the sum of [`phase_weight`] over the pieces of both sides, capped
/// at [`MAX_PHASE`] so that promoted extra pieces cannot push it past the
/// opening value.
#[must_use]
pub fn game_phase(pos: &Position) -> i32 {
    let total: i32 = PieceKind::ALL
        .into_iter()
        .map(|kind| phase_weight(kind) * pos.pieces(kind).count() as i32)
        .sum();
    total.min(MAX_PHASE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(fen: &str) -> i32 {
        game_phase(&Position::from_fen(fen).unwrap())
    }

    #[test]
    fn full_board_is_max_phase() {
        // Per side: 2 knights + 2 bishops + 2 rooks x 2 + 1 queen x 4 = 12.
        assert_eq!(game_phase(&Position::startpos()), MAX_PHASE);
        assert_eq!(MAX_PHASE, 24);
    }

    #[test]
    fn kings_and_pawns_are_phase_zero() {
        assert_eq!(phase("4k3/8/8/8/8/8/8/4K3 w - - 0 1"), 0);
        assert_eq!(phase("4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1"), 0);
    }

    #[test]
    fn each_piece_adds_its_weight() {
        assert_eq!(phase("4k3/8/8/8/8/8/8/N3K3 w - - 0 1"), 1);
        assert_eq!(phase("4k3/8/8/8/8/8/8/B3K3 w - - 0 1"), 1);
        assert_eq!(phase("4k3/8/8/8/8/8/8/R3K3 w - - 0 1"), 2);
        assert_eq!(phase("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1"), 4);
        // Both colours count: queens off, everything else on, is 16.
        assert_eq!(
            phase("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1"),
            16
        );
        // A rook and a minor piece each.
        assert_eq!(phase("r3k1n1/8/8/8/8/8/8/R3K1B1 w - - 0 1"), 6);
    }

    #[test]
    fn promoted_pieces_cannot_exceed_max_phase() {
        // Nine white queens would be 36.
        assert_eq!(phase("7k/8/8/8/8/QQQ5/QQQ5/QQQ1K3 b - - 0 1"), MAX_PHASE);
    }
}
