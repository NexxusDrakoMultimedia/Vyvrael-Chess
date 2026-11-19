// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Material: the value of the pieces each side has.

use super::{Evaluator, Score, relative_to};
use crate::board::{Color, PieceKind, Position};

/// The value of one piece of `kind` in centipawns.
///
/// These are the conventional values, with the bishop slightly above the
/// knight. The king has no material value: it is never exchanged.
#[must_use]
pub const fn piece_value(kind: PieceKind) -> Score {
    match kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 320,
        PieceKind::Bishop => 330,
        PieceKind::Rook => 500,
        PieceKind::Queen => 900,
        PieceKind::King => 0,
    }
}

/// The total value of `color`'s pieces.
#[must_use]
pub fn material_of(pos: &Position, color: Color) -> Score {
    PieceKind::ALL
        .into_iter()
        .map(|kind| piece_value(kind) * pos.pieces_of(color, kind).count() as Score)
        .sum()
}

/// White's material minus Black's.
#[must_use]
pub fn material_balance(pos: &Position) -> Score {
    material_of(pos, Color::White) - material_of(pos, Color::Black)
}

/// An evaluator that counts material and nothing else.
///
/// It is the simplest sound evaluation and the baseline that every later
/// evaluation term must beat.
#[derive(Debug, Clone, Copy, Default)]
pub struct Material;

impl Evaluator for Material {
    fn evaluate(&self, pos: &Position) -> Score {
        relative_to(pos.side_to_move(), material_balance(pos))
    }

    fn name(&self) -> &'static str {
        "material"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    #[test]
    fn piece_values_are_ordered_sensibly() {
        use PieceKind::{Bishop, King, Knight, Pawn, Queen, Rook};
        assert_eq!(piece_value(Pawn), 100);
        assert!(piece_value(Knight) > 3 * piece_value(Pawn));
        assert!(piece_value(Bishop) > piece_value(Knight));
        assert!(piece_value(Rook) > piece_value(Bishop));
        assert!(piece_value(Queen) > piece_value(Rook) + piece_value(Bishop));
        // Two rooks are worth a little more than a queen.
        assert!(2 * piece_value(Rook) > piece_value(Queen));
        // A minor piece is worth less than a rook: winning "the exchange".
        assert!(piece_value(Rook) > piece_value(Knight) + piece_value(Pawn));
        assert_eq!(piece_value(King), 0);
    }

    #[test]
    fn start_position_is_balanced_at_full_material() {
        let pos = Position::startpos();
        // 8 pawns, 2 knights, 2 bishops, 2 rooks and a queen each.
        let full = 8 * 100 + 2 * 320 + 2 * 330 + 2 * 500 + 900;
        assert_eq!(material_of(&pos, Color::White), full);
        assert_eq!(material_of(&pos, Color::Black), full);
        assert_eq!(material_balance(&pos), 0);
        assert_eq!(Material.evaluate(&pos), 0);
    }

    #[test]
    fn a_missing_piece_shows_up_with_the_right_sign() {
        // Black has no queen.
        let white_to_move = fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_eq!(material_balance(&white_to_move), 900);
        assert_eq!(Material.evaluate(&white_to_move), 900);
        // The same material with Black to move: bad for the side to move.
        let black_to_move = fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        assert_eq!(material_balance(&black_to_move), 900);
        assert_eq!(Material.evaluate(&black_to_move), -900);
    }

    #[test]
    fn imbalances() {
        // Rook against knight and pawn: 500 - 420.
        assert_eq!(
            material_balance(&fen("4k3/8/8/8/8/8/3np3/R3K3 w - - 0 1")),
            80
        );
        // Bishop against knight.
        assert_eq!(
            material_balance(&fen("4k1n1/8/8/8/8/8/8/2B1K3 w - - 0 1")),
            10
        );
        // Bare kings.
        assert_eq!(material_balance(&fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1")), 0);
    }

    #[test]
    fn evaluator_name() {
        assert_eq!(Material.name(), "material");
    }
}
