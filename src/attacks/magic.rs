// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Magic bitboards: constant-time slider attacks by table lookup.
//!
//! A slider's attacks depend only on the pieces standing on its rays, and
//! not even on the last square of each ray (a piece there cannot block
//! anything behind it). Those "relevant" squares form a mask of at most 12
//! bits, so there are at most 4096 blocker arrangements per square. A magic
//! number is a multiplier that maps every arrangement to its own slot (or a
//! slot shared only with arrangements giving the same attacks):
//!
//! ```text
//! index = ((occupied & mask) * magic) >> (64 - bits)
//! ```
//!
//! See `docs/ARCHITECTURE.md` ("Attacks and move generation").

use super::scan::{BISHOP_STEPS, ROOK_STEPS, scan};
use crate::board::{Bitboard, Square};

/// The board edges that cannot block a slider on `square`: the outer ranks
/// and files, except the ones the slider itself stands on.
const fn irrelevant_edges(square: Square) -> Bitboard {
    let ranks = (Bitboard::RANK_1.0 | Bitboard::RANK_8.0) & !Bitboard::rank(square.rank()).0;
    let files = (Bitboard::FILE_A.0 | Bitboard::FILE_H.0) & !Bitboard::file(square.file()).0;
    Bitboard(ranks | files)
}

/// The squares whose occupancy can change what a bishop on `square`
/// attacks: its diagonals without the board edge.
#[must_use]
pub const fn bishop_relevant_mask(square: Square) -> Bitboard {
    let rays = scan(square, Bitboard::EMPTY, BISHOP_STEPS);
    Bitboard(rays.0 & !irrelevant_edges(square).0)
}

/// The squares whose occupancy can change what a rook on `square` attacks:
/// its rank and file without the last square of each ray.
#[must_use]
pub const fn rook_relevant_mask(square: Square) -> Bitboard {
    let rays = scan(square, Bitboard::EMPTY, ROOK_STEPS);
    Bitboard(rays.0 & !irrelevant_edges(square).0)
}

/// Iterator over every subset of a set of squares, the empty set first.
///
/// Uses the "carry-rippler" trick: `next = (current - mask) & mask` steps
/// through all `2^n` subsets of an `n`-square mask.
#[derive(Debug, Clone)]
pub struct Subsets {
    mask: u64,
    next: Option<u64>,
}

/// All subsets of `mask`: every arrangement of pieces on those squares.
#[must_use]
pub const fn subsets(mask: Bitboard) -> Subsets {
    Subsets {
        mask: mask.0,
        next: Some(0),
    }
}

impl Iterator for Subsets {
    type Item = Bitboard;

    fn next(&mut self) -> Option<Bitboard> {
        let current = self.next?;
        let following = current.wrapping_sub(self.mask) & self.mask;
        self.next = (following != 0).then_some(following);
        Some(Bitboard(current))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{bishop_attacks_by_scan, rook_attacks_by_scan};
    use super::*;

    #[test]
    fn relevant_mask_sizes() {
        assert_eq!(rook_relevant_mask(Square::A1).count(), 12);
        assert_eq!(rook_relevant_mask(Square::E4).count(), 10);
        assert_eq!(rook_relevant_mask(Square::A4).count(), 11);
        assert_eq!(bishop_relevant_mask(Square::A1).count(), 6);
        assert_eq!(bishop_relevant_mask(Square::E4).count(), 9);
        assert_eq!(bishop_relevant_mask(Square::B1).count(), 5);
        // The well-known total table sizes for plain magic bitboards.
        let size = |mask: fn(Square) -> Bitboard| -> u32 {
            Square::all().map(|sq| 1 << mask(sq).count()).sum()
        };
        assert_eq!(size(rook_relevant_mask), 102_400);
        assert_eq!(size(bishop_relevant_mask), 5_248);
    }

    #[test]
    fn rook_mask_example() {
        let expected: Bitboard = [
            Square::B1,
            Square::C1,
            Square::D1,
            Square::E1,
            Square::F1,
            Square::G1,
            Square::A2,
            Square::A3,
            Square::A4,
            Square::A5,
            Square::A6,
            Square::A7,
        ]
        .into_iter()
        .collect();
        assert_eq!(rook_relevant_mask(Square::A1), expected);
    }

    /// The defining property of a relevant mask: pieces outside it never
    /// change the attack set. Checked for every arrangement inside the mask
    /// with every square outside it first empty and then occupied.
    #[test]
    fn occupancy_outside_the_mask_is_irrelevant() {
        for square in Square::all() {
            let rook = rook_relevant_mask(square);
            let bishop = bishop_relevant_mask(square);
            assert!(!rook.contains(square) && !bishop.contains(square));
            for occupied in subsets(rook) {
                assert_eq!(
                    rook_attacks_by_scan(square, occupied | !rook),
                    rook_attacks_by_scan(square, occupied),
                    "{square}"
                );
            }
            for occupied in subsets(bishop) {
                assert_eq!(
                    bishop_attacks_by_scan(square, occupied | !bishop),
                    bishop_attacks_by_scan(square, occupied),
                    "{square}"
                );
            }
        }
    }

    #[test]
    fn subsets_enumerates_each_subset_once() {
        let mask = rook_relevant_mask(Square::E4);
        let all: Vec<Bitboard> = subsets(mask).collect();
        assert_eq!(all.len(), 1 << mask.count());
        assert_eq!(all[0], Bitboard::EMPTY);
        assert!(all.contains(&mask));
        assert!(all.iter().all(|&subset| subset & !mask == Bitboard::EMPTY));
        let mut values: Vec<u64> = all.iter().map(|bb| bb.0).collect();
        values.sort_unstable();
        values.dedup();
        assert_eq!(values.len(), all.len());
    }

    #[test]
    fn subsets_of_small_masks() {
        assert_eq!(
            subsets(Bitboard::EMPTY).collect::<Vec<_>>(),
            [Bitboard::EMPTY]
        );
        let two = Bitboard(0b101);
        let expected = [
            Bitboard(0),
            Bitboard(0b001),
            Bitboard(0b100),
            Bitboard(0b101),
        ];
        assert_eq!(subsets(two).collect::<Vec<_>>(), expected);
        assert_eq!(subsets(Bitboard::FULL).take(3).count(), 3);
    }
}
