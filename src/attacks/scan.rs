// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Slider attacks by scanning along each ray.
//!
//! This is the slow, obviously-correct way to find what a bishop or rook
//! attacks: walk outwards square by square until the board edge or a piece.
//! It is the reference that the magic-bitboard tables are built from and
//! tested against; move generation itself uses the tables.

use crate::board::{Bitboard, Square};

/// The four (file, rank) steps a bishop slides along.
pub(super) const BISHOP_STEPS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, -1), (-1, 1)];

/// The four (file, rank) steps a rook slides along.
pub(super) const ROOK_STEPS: [(i8, i8); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

/// Walks from `square` along each step until the edge or the first occupied
/// square, which is included (it may be capturable).
pub(super) const fn scan(square: Square, occupied: Bitboard, steps: [(i8, i8); 4]) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let mut i = 0;
    while i < steps.len() {
        let (files, ranks) = steps[i];
        let mut current = square;
        while let Some(next) = current.offset(files, ranks) {
            attacks = attacks.with(next);
            if occupied.contains(next) {
                break;
            }
            current = next;
        }
        i += 1;
    }
    attacks
}

/// The squares a bishop on `square` attacks when `occupied` holds all
/// pieces, found by scanning. The first piece met on each diagonal is
/// included whatever its colour; squares behind it are not.
#[must_use]
pub const fn bishop_attacks_by_scan(square: Square, occupied: Bitboard) -> Bitboard {
    scan(square, occupied, BISHOP_STEPS)
}

/// The squares a rook on `square` attacks when `occupied` holds all pieces,
/// found by scanning. The first piece met on each line is included whatever
/// its colour; squares behind it are not.
#[must_use]
pub const fn rook_attacks_by_scan(square: Square, occupied: Bitboard) -> Bitboard {
    scan(square, occupied, ROOK_STEPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squares(list: &[Square]) -> Bitboard {
        list.iter().copied().collect()
    }

    #[test]
    fn rook_on_an_empty_board_sees_its_rank_and_file() {
        for square in Square::all() {
            let expected =
                (Bitboard::file(square.file()) | Bitboard::rank(square.rank())).without(square);
            assert_eq!(rook_attacks_by_scan(square, Bitboard::EMPTY), expected);
            assert_eq!(expected.count(), 14);
        }
    }

    #[test]
    fn bishop_on_an_empty_board() {
        let counts = [
            (Square::A1, 7),
            (Square::H1, 7),
            (Square::B2, 9),
            (Square::E4, 13),
        ];
        for (square, count) in counts {
            let attacks = bishop_attacks_by_scan(square, Bitboard::EMPTY);
            assert_eq!(attacks.count(), count, "{square}");
            // Every attacked square is on a diagonal: equal file and rank distance.
            for target in attacks {
                let files = square.file().abs_diff(target.file());
                let ranks = square.rank().abs_diff(target.rank());
                assert!(files == ranks && files > 0, "{square} {target}");
            }
        }
        assert_eq!(
            bishop_attacks_by_scan(Square::A1, Bitboard::EMPTY),
            squares(&[
                Square::B2,
                Square::C3,
                Square::D4,
                Square::E5,
                Square::F6,
                Square::G7,
                Square::H8
            ])
        );
    }

    #[test]
    fn rook_stops_at_the_first_piece_and_includes_it() {
        let occupied = squares(&[Square::E6, Square::E7, Square::C4, Square::E2, Square::H4]);
        let expected = squares(&[
            Square::E5,
            Square::E6,
            Square::E3,
            Square::E2,
            Square::D4,
            Square::C4,
            Square::F4,
            Square::G4,
            Square::H4,
        ]);
        assert_eq!(rook_attacks_by_scan(Square::E4, occupied), expected);
    }

    #[test]
    fn bishop_stops_at_the_first_piece_and_includes_it() {
        let occupied = squares(&[Square::F6, Square::G7, Square::B6, Square::B2]);
        let expected = squares(&[
            Square::E5,
            Square::F6,
            Square::C5,
            Square::B6,
            Square::C3,
            Square::B2,
            Square::E3,
            Square::F2,
            Square::G1,
        ]);
        assert_eq!(bishop_attacks_by_scan(Square::D4, occupied), expected);
    }

    #[test]
    fn a_piece_on_the_sliders_own_square_changes_nothing() {
        let occupied = squares(&[Square::E6, Square::C4]);
        for square in [Square::E4, Square::A1] {
            let with_self = occupied.with(square);
            assert_eq!(
                rook_attacks_by_scan(square, occupied),
                rook_attacks_by_scan(square, with_self)
            );
            assert_eq!(
                bishop_attacks_by_scan(square, occupied),
                bishop_attacks_by_scan(square, with_self)
            );
        }
    }

    #[test]
    fn a_full_board_leaves_only_the_neighbours() {
        let rook = rook_attacks_by_scan(Square::E4, Bitboard::FULL);
        assert_eq!(
            rook,
            squares(&[Square::E5, Square::E3, Square::D4, Square::F4])
        );
        let bishop = bishop_attacks_by_scan(Square::A1, Bitboard::FULL);
        assert_eq!(bishop, Bitboard::from_square(Square::B2));
    }
}
