// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Bitboards: sets of squares stored as one 64-bit integer.

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use super::Square;

/// A set of squares, one bit per square.
///
/// Bit `i` is set when the square with index `i` is in the set, so `a1` is
/// the least significant bit and `h8` the most significant.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// The empty set.
    pub const EMPTY: Bitboard = Bitboard(0);

    /// The set of all 64 squares.
    pub const FULL: Bitboard = Bitboard(u64::MAX);

    /// The a-file. Shift left by `n` for the file `n` to its right.
    pub const FILE_A: Bitboard = Bitboard(0x0101_0101_0101_0101);

    /// The h-file.
    pub const FILE_H: Bitboard = Bitboard(0x8080_8080_8080_8080);

    /// The first rank. Shift left by `8 * n` for the rank `n` above it.
    pub const RANK_1: Bitboard = Bitboard(0xFF);

    /// The eighth rank.
    pub const RANK_8: Bitboard = Bitboard(0xFF00_0000_0000_0000);

    /// All squares of a file (`0..8`, where 0 is the a-file).
    ///
    /// # Panics
    ///
    /// Panics if `file` is 8 or more.
    #[must_use]
    pub const fn file(file: u8) -> Bitboard {
        assert!(file < 8, "Bitboard::file: file out of range");
        Bitboard(Bitboard::FILE_A.0 << file)
    }

    /// All squares of a rank (`0..8`, where 0 is the first rank).
    ///
    /// # Panics
    ///
    /// Panics if `rank` is 8 or more.
    #[must_use]
    pub const fn rank(rank: u8) -> Bitboard {
        assert!(rank < 8, "Bitboard::rank: rank out of range");
        Bitboard(Bitboard::RANK_1.0 << (rank * 8))
    }

    /// Every square moved one rank up (towards rank 8). Squares on rank 8
    /// fall off the board.
    #[must_use]
    pub const fn north(self) -> Bitboard {
        Bitboard(self.0 << 8)
    }

    /// Every square moved one rank down (towards rank 1).
    #[must_use]
    pub const fn south(self) -> Bitboard {
        Bitboard(self.0 >> 8)
    }

    /// Every square moved one file right (towards the h-file). Squares on
    /// the h-file fall off the board rather than wrapping to the a-file.
    #[must_use]
    pub const fn east(self) -> Bitboard {
        Bitboard((self.0 & !Bitboard::FILE_H.0) << 1)
    }

    /// Every square moved one file left (towards the a-file), without
    /// wrapping.
    #[must_use]
    pub const fn west(self) -> Bitboard {
        Bitboard((self.0 & !Bitboard::FILE_A.0) >> 1)
    }

    /// Every square moved one step diagonally up and right.
    #[must_use]
    pub const fn north_east(self) -> Bitboard {
        self.north().east()
    }

    /// Every square moved one step diagonally up and left.
    #[must_use]
    pub const fn north_west(self) -> Bitboard {
        self.north().west()
    }

    /// Every square moved one step diagonally down and right.
    #[must_use]
    pub const fn south_east(self) -> Bitboard {
        self.south().east()
    }

    /// Every square moved one step diagonally down and left.
    #[must_use]
    pub const fn south_west(self) -> Bitboard {
        self.south().west()
    }

    /// The set containing only `square`.
    #[must_use]
    pub const fn from_square(square: Square) -> Bitboard {
        Bitboard(1 << square.index())
    }

    /// Whether the set has no squares.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The number of squares in the set.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether the set has two or more squares.
    #[must_use]
    pub const fn more_than_one(self) -> bool {
        self.0 & self.0.wrapping_sub(1) != 0
    }

    /// Whether `square` is in the set.
    #[must_use]
    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square.index()) != 0
    }

    /// A copy of the set with `square` added.
    #[must_use]
    pub const fn with(self, square: Square) -> Bitboard {
        Bitboard(self.0 | (1 << square.index()))
    }

    /// A copy of the set with `square` removed.
    #[must_use]
    pub const fn without(self, square: Square) -> Bitboard {
        Bitboard(self.0 & !(1 << square.index()))
    }

    /// Adds `square` to the set.
    pub const fn set(&mut self, square: Square) {
        self.0 |= 1 << square.index();
    }

    /// Removes `square` from the set.
    pub const fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square.index());
    }

    /// Adds `square` if it is absent, removes it if it is present.
    pub const fn toggle(&mut self, square: Square) {
        self.0 ^= 1 << square.index();
    }

    /// The lowest-indexed square in the set, or `None` if it is empty.
    #[must_use]
    pub const fn lsb(self) -> Option<Square> {
        // An empty board has 64 trailing zeros, which `from_index` rejects.
        Square::from_index(self.0.trailing_zeros() as u8)
    }

    /// The highest-indexed square in the set, or `None` if it is empty.
    #[must_use]
    pub const fn msb(self) -> Option<Square> {
        if self.0 == 0 {
            None
        } else {
            Square::from_index(63 - self.0.leading_zeros() as u8)
        }
    }

    /// Removes and returns the lowest-indexed square, or `None` if empty.
    pub const fn pop_lsb(&mut self) -> Option<Square> {
        let square = self.lsb();
        self.0 &= self.0.wrapping_sub(1);
        square
    }
}

impl From<Square> for Bitboard {
    fn from(square: Square) -> Bitboard {
        Bitboard::from_square(square)
    }
}

impl BitAnd for Bitboard {
    type Output = Bitboard;

    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitOr for Bitboard {
    type Output = Bitboard;

    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitXor for Bitboard {
    type Output = Bitboard;

    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Bitboard;

    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Bitboard) {
        self.0 &= rhs.0;
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Bitboard) {
        self.0 |= rhs.0;
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Bitboard) {
        self.0 ^= rhs.0;
    }
}

/// Iterator over the squares of a [`Bitboard`], lowest index first.
#[derive(Debug, Clone)]
pub struct BitboardIter(Bitboard);

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Square> {
        self.0.pop_lsb()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for BitboardIter {}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    fn into_iter(self) -> BitboardIter {
        BitboardIter(self)
    }
}

impl FromIterator<Square> for Bitboard {
    fn from_iter<I: IntoIterator<Item = Square>>(iter: I) -> Bitboard {
        iter.into_iter()
            .fold(Bitboard::EMPTY, |bb, square| bb.with(square))
    }
}

impl fmt::Debug for Bitboard {
    /// Draws the set as an 8x8 diagram with rank 8 at the top, `X` for a
    /// member square and `.` for a non-member, followed by the hex value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Bitboard({:#018x})", self.0)?;
        for rank in (0..8).rev() {
            for file in 0..8 {
                let mark = if self.contains(Square::new(file, rank)) {
                    'X'
                } else {
                    '.'
                };
                write!(f, "{mark}")?;
                if file < 7 {
                    write!(f, " ")?;
                }
            }
            if rank > 0 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_full() {
        assert!(Bitboard::EMPTY.is_empty());
        assert_eq!(Bitboard::EMPTY.count(), 0);
        assert_eq!(Bitboard::FULL.count(), 64);
        assert_eq!(Bitboard::default(), Bitboard::EMPTY);
        assert_eq!(!Bitboard::EMPTY, Bitboard::FULL);
    }

    #[test]
    fn from_square_uses_the_square_index_as_bit() {
        assert_eq!(Bitboard::from_square(Square::A1).0, 1);
        assert_eq!(Bitboard::from_square(Square::H1).0, 0x80);
        assert_eq!(Bitboard::from_square(Square::A8).0, 1 << 56);
        assert_eq!(Bitboard::from_square(Square::H8).0, 1 << 63);
        for square in Square::all() {
            let bb = Bitboard::from(square);
            assert_eq!(bb.count(), 1);
            assert!(bb.contains(square));
            assert_eq!(bb.lsb(), Some(square));
            assert_eq!(bb.msb(), Some(square));
        }
    }

    #[test]
    fn set_clear_toggle_with_without() {
        let mut bb = Bitboard::EMPTY;
        bb.set(Square::E4);
        bb.set(Square::E4);
        assert_eq!(bb, Bitboard::from_square(Square::E4));
        bb.toggle(Square::D5);
        assert!(bb.contains(Square::D5));
        bb.toggle(Square::D5);
        assert!(!bb.contains(Square::D5));
        bb.clear(Square::E4);
        bb.clear(Square::E4);
        assert!(bb.is_empty());

        let two = Bitboard::EMPTY.with(Square::A1).with(Square::H8);
        assert_eq!(two.0, 0x8000_0000_0000_0001);
        assert_eq!(two.without(Square::A1), Bitboard::from_square(Square::H8));
        assert_eq!(two.without(Square::E4), two);
    }

    #[test]
    fn set_operations() {
        let a = Bitboard(0b1100);
        let b = Bitboard(0b1010);
        assert_eq!(a & b, Bitboard(0b1000));
        assert_eq!(a | b, Bitboard(0b1110));
        assert_eq!(a ^ b, Bitboard(0b0110));
        assert_eq!(!a & b, Bitboard(0b0010));

        let mut c = a;
        c &= b;
        assert_eq!(c, Bitboard(0b1000));
        c |= a;
        assert_eq!(c, a);
        c ^= a;
        assert!(c.is_empty());
    }

    #[test]
    fn more_than_one() {
        assert!(!Bitboard::EMPTY.more_than_one());
        assert!(!Bitboard::from_square(Square::H8).more_than_one());
        assert!(Bitboard(0b11).more_than_one());
        assert!(Bitboard::FULL.more_than_one());
    }

    #[test]
    fn lsb_msb_and_pop() {
        assert_eq!(Bitboard::EMPTY.lsb(), None);
        assert_eq!(Bitboard::EMPTY.msb(), None);

        let mut bb: Bitboard = [Square::C3, Square::A1, Square::H8].into_iter().collect();
        assert_eq!(bb.lsb(), Some(Square::A1));
        assert_eq!(bb.msb(), Some(Square::H8));
        assert_eq!(bb.pop_lsb(), Some(Square::A1));
        assert_eq!(bb.pop_lsb(), Some(Square::C3));
        assert_eq!(bb.pop_lsb(), Some(Square::H8));
        assert_eq!(bb.pop_lsb(), None);
        assert_eq!(bb.pop_lsb(), None);
        assert!(bb.is_empty());
    }

    #[test]
    fn iterates_in_ascending_order_with_exact_size() {
        let squares = [Square::B1, Square::E4, Square::A8, Square::H8];
        let bb: Bitboard = squares.into_iter().collect();
        let iter = bb.into_iter();
        assert_eq!(iter.len(), 4);
        assert_eq!(iter.collect::<Vec<_>>(), squares);
        assert_eq!(Bitboard::EMPTY.into_iter().next(), None);
        assert_eq!(Bitboard::FULL.into_iter().count(), 64);
        assert!(Bitboard::FULL.into_iter().eq(Square::all()));
    }

    #[test]
    fn file_and_rank_masks() {
        assert_eq!(Bitboard::file(0), Bitboard::FILE_A);
        assert_eq!(Bitboard::file(7), Bitboard::FILE_H);
        assert_eq!(Bitboard::rank(0), Bitboard::RANK_1);
        assert_eq!(Bitboard::rank(7), Bitboard::RANK_8);
        for square in Square::all() {
            for line in 0..8 {
                let on_file = Bitboard::file(line).contains(square);
                let on_rank = Bitboard::rank(line).contains(square);
                assert_eq!(on_file, square.file() == line);
                assert_eq!(on_rank, square.rank() == line);
            }
        }
        let mut union = Bitboard::EMPTY;
        for line in 0..8 {
            assert_eq!(Bitboard::file(line).count(), 8);
            assert_eq!(Bitboard::rank(line).count(), 8);
            union |= Bitboard::file(line);
        }
        assert_eq!(union, Bitboard::FULL);
    }

    /// Every shift must agree with `Square::offset` for every square,
    /// including dropping squares that would leave the board.
    #[test]
    fn shifts_agree_with_square_offset() {
        type Shift = fn(Bitboard) -> Bitboard;
        let shifts: [(Shift, i8, i8); 8] = [
            (Bitboard::north, 0, 1),
            (Bitboard::south, 0, -1),
            (Bitboard::east, 1, 0),
            (Bitboard::west, -1, 0),
            (Bitboard::north_east, 1, 1),
            (Bitboard::north_west, -1, 1),
            (Bitboard::south_east, 1, -1),
            (Bitboard::south_west, -1, -1),
        ];
        for (shift, files, ranks) in shifts {
            for square in Square::all() {
                let expected = square
                    .offset(files, ranks)
                    .map_or(Bitboard::EMPTY, Bitboard::from_square);
                let actual = shift(Bitboard::from_square(square));
                assert_eq!(actual, expected, "{square} by ({files}, {ranks})");
            }
        }
    }

    #[test]
    fn shifting_a_full_board_drops_one_line() {
        assert_eq!(Bitboard::FULL.north(), !Bitboard::RANK_1);
        assert_eq!(Bitboard::FULL.south(), !Bitboard::RANK_8);
        assert_eq!(Bitboard::FULL.east(), !Bitboard::FILE_A);
        assert_eq!(Bitboard::FULL.west(), !Bitboard::FILE_H);
    }

    #[test]
    fn debug_draws_rank_eight_first() {
        let bb = Bitboard::EMPTY.with(Square::A8).with(Square::H1);
        let text = format!("{bb:?}");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 9);
        assert_eq!(lines[0], "Bitboard(0x0100000000000080)");
        assert_eq!(lines[1], "X . . . . . . .");
        assert_eq!(lines[8], ". . . . . . . X");
    }
}
