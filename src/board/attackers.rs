// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Attack queries on a position: who attacks a square, and is a king in
//! check.

use super::{Bitboard, Color, PieceKind, Position, Square};
use crate::attacks::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

impl Position {
    /// All pieces of both colours that attack `square`, given `occupied` as
    /// the set of squares that block sliders.
    ///
    /// Passing an occupancy other than [`Position::occupied`] answers "what
    /// if" questions, such as which sliders attack a square once a piece has
    /// moved out of the way.
    ///
    /// Works backwards from the target: a knight attacks `square` exactly
    /// when a knight on `square` would attack it, and likewise for the other
    /// pieces (for pawns, with the colours swapped).
    #[must_use]
    pub fn attackers_to(&self, square: Square, occupied: Bitboard) -> Bitboard {
        let queens = self.pieces(PieceKind::Queen);
        let diagonal = self.pieces(PieceKind::Bishop) | queens;
        let straight = self.pieces(PieceKind::Rook) | queens;
        let white_pawns = self.pieces_of(Color::White, PieceKind::Pawn);
        let black_pawns = self.pieces_of(Color::Black, PieceKind::Pawn);

        (pawn_attacks(Color::Black, square) & white_pawns)
            | (pawn_attacks(Color::White, square) & black_pawns)
            | (knight_attacks(square) & self.pieces(PieceKind::Knight))
            | (king_attacks(square) & self.pieces(PieceKind::King))
            | (bishop_attacks(square, occupied) & diagonal)
            | (rook_attacks(square, occupied) & straight)
    }

    /// Whether any piece of colour `by` attacks `square`.
    #[must_use]
    pub fn is_attacked_by(&self, square: Square, by: Color) -> bool {
        !(self.attackers_to(square, self.occupied()) & self.occupied_by(by)).is_empty()
    }

    /// The enemy pieces giving check to the side to move (empty when it is
    /// not in check, two pieces in a double check).
    ///
    /// Empty if the side to move has no king.
    #[must_use]
    pub fn checkers(&self) -> Bitboard {
        let us = self.side_to_move();
        match self.king_square(us) {
            Some(king) => self.attackers_to(king, self.occupied()) & self.occupied_by(us.flip()),
            None => Bitboard::EMPTY,
        }
    }

    /// Whether the side to move could capture the enemy king, which means
    /// the side that just moved left its own king attacked. Such a position
    /// cannot arise in a legal game.
    ///
    /// False if the side not to move has no king.
    #[must_use]
    pub fn can_capture_king(&self) -> bool {
        let us = self.side_to_move();
        self.king_square(us.flip())
            .is_some_and(|king| self.is_attacked_by(king, us))
    }

    /// Whether the side to move is in check.
    #[must_use]
    pub fn in_check(&self) -> bool {
        !self.checkers().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    fn squares(list: &[Square]) -> Bitboard {
        list.iter().copied().collect()
    }

    #[test]
    fn start_position_attackers() {
        let pos = Position::startpos();
        let occupied = pos.occupied();
        assert_eq!(
            pos.attackers_to(Square::F3, occupied),
            squares(&[Square::G1, Square::E2, Square::G2])
        );
        assert_eq!(
            pos.attackers_to(Square::D2, occupied),
            squares(&[Square::B1, Square::C1, Square::D1, Square::E1])
        );
        assert_eq!(
            pos.attackers_to(Square::C6, occupied),
            squares(&[Square::B8, Square::B7, Square::D7])
        );
        assert!(pos.attackers_to(Square::E4, occupied).is_empty());
        assert!(pos.is_attacked_by(Square::F3, Color::White));
        assert!(!pos.is_attacked_by(Square::F3, Color::Black));
        assert!(pos.is_attacked_by(Square::F6, Color::Black));
        assert!(!pos.in_check());
        assert!(pos.checkers().is_empty());
    }

    #[test]
    fn each_piece_kind_gives_check() {
        let cases = [
            ("4k3/8/8/8/8/8/4r3/4K3 w - - 0 1", Square::E2),
            ("4k3/8/8/8/1b6/8/8/4K3 w - - 0 1", Square::B4),
            ("4k3/8/8/8/8/8/8/q3K3 w - - 0 1", Square::A1),
            ("4k3/8/8/8/7q/8/8/4K3 w - - 0 1", Square::H4),
            ("4k3/8/8/8/8/3n4/8/4K3 w - - 0 1", Square::D3),
            ("4k3/8/8/8/8/8/3p4/4K3 w - - 0 1", Square::D2),
            ("4k3/8/8/8/8/8/5p2/4K3 w - - 0 1", Square::F2),
            ("4k3/3P4/8/8/8/8/8/4K3 b - - 0 1", Square::D7),
            ("4k3/5P2/8/8/8/8/8/4K3 b - - 0 1", Square::F7),
            ("4k3/8/5N2/8/8/8/8/4K3 b - - 0 1", Square::F6),
        ];
        for (text, checker) in cases {
            let pos = fen(text);
            assert!(pos.in_check(), "{text}");
            assert_eq!(pos.checkers(), Bitboard::from_square(checker), "{text}");
        }
    }

    #[test]
    fn positions_that_are_not_check() {
        for text in [
            // Slider blocked by a piece of either colour.
            "4k3/8/8/8/8/4r3/4P3/4K3 w - - 0 1",
            "4k3/8/8/8/1b6/2n5/8/4K3 w - - 0 1",
            // Pawns attack forwards only: these stand diagonally behind.
            "4k3/8/8/8/4K3/3p4/8/8 w - - 0 1",
            "8/3P4/4k3/8/8/8/8/4K3 b - - 0 1",
            // A pawn straight ahead does not attack.
            "4k3/8/8/8/8/8/4p3/4K3 w - - 0 1",
            "4k3/4P3/8/8/8/8/8/4K3 b - - 0 1",
        ] {
            assert!(!fen(text).in_check(), "{text}");
        }
    }

    #[test]
    fn double_check_reports_both_checkers() {
        let pos = fen("4k3/8/8/8/1b6/8/4r3/4K3 w - - 0 1");
        assert_eq!(pos.checkers(), squares(&[Square::B4, Square::E2]));
        assert!(pos.in_check());
    }

    #[test]
    fn only_the_side_to_move_is_asked_about() {
        // The white rook attacks the black king, but it is White's move.
        let pos = fen("4k3/8/8/8/8/8/8/4RK2 w - - 0 1");
        assert!(!pos.in_check());
        assert!(pos.is_attacked_by(Square::E8, Color::White));
    }

    #[test]
    fn custom_occupancy_sees_through_removed_pieces() {
        let pos = fen("4k3/8/8/8/8/4r3/4P3/4K3 w - - 0 1");
        let without_pawn = pos.occupied().without(Square::E2);
        assert!(pos.attackers_to(Square::E1, pos.occupied()).is_empty());
        assert_eq!(
            pos.attackers_to(Square::E1, without_pawn),
            Bitboard::from_square(Square::E3)
        );
    }

    #[test]
    fn kings_attack_adjacent_squares() {
        let pos = fen("8/8/8/3k4/8/3K4/8/8 w - - 0 1");
        assert!(pos.is_attacked_by(Square::D4, Color::White));
        assert!(pos.is_attacked_by(Square::D4, Color::Black));
        assert_eq!(
            pos.attackers_to(Square::D4, pos.occupied()),
            squares(&[Square::D3, Square::D5])
        );
    }

    #[test]
    fn can_capture_king_looks_at_the_side_not_to_move() {
        // The white rook attacks the black king and it is White's move.
        assert!(fen("4k3/8/8/8/8/8/8/4RK2 w - - 0 1").can_capture_king());
        // Black to move in the same position is simply in check.
        let black = fen("4k3/8/8/8/8/8/8/4RK2 b - - 0 1");
        assert!(!black.can_capture_king());
        assert!(black.in_check());
        assert!(!Position::startpos().can_capture_king());
        assert!(!Position::empty().can_capture_king());
    }

    #[test]
    fn a_position_without_a_king_is_not_in_check() {
        assert!(!Position::empty().in_check());
    }
}
