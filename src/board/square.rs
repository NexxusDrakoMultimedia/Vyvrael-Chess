// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Board squares.

use std::fmt;
use std::str::FromStr;

/// A square of the board, numbered 0 to 63.
///
/// The mapping is "little-endian rank-file": `a1 = 0`, `b1 = 1`, ...,
/// `h1 = 7`, `a2 = 8`, ..., `h8 = 63`. So `index = rank * 8 + file`, and the
/// index of a square is also its bit position in a [`Bitboard`].
///
/// Files and ranks are plain `u8` values in `0..8` (file `0` is the a-file,
/// rank `0` is the first rank).
///
/// [`Bitboard`]: super::Bitboard
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Square(u8);

/// Named constants for all 64 squares, `A1` to `H8`.
#[rustfmt::skip]
impl Square {
    pub const A1: Square = Square(0); pub const B1: Square = Square(1); pub const C1: Square = Square(2); pub const D1: Square = Square(3);
    pub const E1: Square = Square(4); pub const F1: Square = Square(5); pub const G1: Square = Square(6); pub const H1: Square = Square(7);
    pub const A2: Square = Square(8); pub const B2: Square = Square(9); pub const C2: Square = Square(10); pub const D2: Square = Square(11);
    pub const E2: Square = Square(12); pub const F2: Square = Square(13); pub const G2: Square = Square(14); pub const H2: Square = Square(15);
    pub const A3: Square = Square(16); pub const B3: Square = Square(17); pub const C3: Square = Square(18); pub const D3: Square = Square(19);
    pub const E3: Square = Square(20); pub const F3: Square = Square(21); pub const G3: Square = Square(22); pub const H3: Square = Square(23);
    pub const A4: Square = Square(24); pub const B4: Square = Square(25); pub const C4: Square = Square(26); pub const D4: Square = Square(27);
    pub const E4: Square = Square(28); pub const F4: Square = Square(29); pub const G4: Square = Square(30); pub const H4: Square = Square(31);
    pub const A5: Square = Square(32); pub const B5: Square = Square(33); pub const C5: Square = Square(34); pub const D5: Square = Square(35);
    pub const E5: Square = Square(36); pub const F5: Square = Square(37); pub const G5: Square = Square(38); pub const H5: Square = Square(39);
    pub const A6: Square = Square(40); pub const B6: Square = Square(41); pub const C6: Square = Square(42); pub const D6: Square = Square(43);
    pub const E6: Square = Square(44); pub const F6: Square = Square(45); pub const G6: Square = Square(46); pub const H6: Square = Square(47);
    pub const A7: Square = Square(48); pub const B7: Square = Square(49); pub const C7: Square = Square(50); pub const D7: Square = Square(51);
    pub const E7: Square = Square(52); pub const F7: Square = Square(53); pub const G7: Square = Square(54); pub const H7: Square = Square(55);
    pub const A8: Square = Square(56); pub const B8: Square = Square(57); pub const C8: Square = Square(58); pub const D8: Square = Square(59);
    pub const E8: Square = Square(60); pub const F8: Square = Square(61); pub const G8: Square = Square(62); pub const H8: Square = Square(63);
}

impl Square {
    /// Number of squares on the board.
    pub const COUNT: usize = 64;

    /// Creates a square from a file and a rank, both in `0..8`.
    ///
    /// # Panics
    ///
    /// Panics if `file` or `rank` is 8 or more.
    #[must_use]
    pub const fn new(file: u8, rank: u8) -> Square {
        assert!(
            file < 8 && rank < 8,
            "Square::new: file or rank out of range"
        );
        Square(rank * 8 + file)
    }

    /// Creates a square from its index, or `None` if `index` is 64 or more.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Square> {
        if index < 64 {
            Some(Square(index))
        } else {
            None
        }
    }

    /// The square's index in `0..64`, for array lookups and bit positions.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The square's file in `0..8` (0 is the a-file).
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    /// The square's rank in `0..8` (0 is the first rank).
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    /// The square `files` to the right and `ranks` up (from White's view),
    /// or `None` if that leaves the board.
    #[must_use]
    pub const fn offset(self, files: i8, ranks: i8) -> Option<Square> {
        let file = self.file() as i8 + files;
        let rank = self.rank() as i8 + ranks;
        if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
            Some(Square::new(file as u8, rank as u8))
        } else {
            None
        }
    }

    /// Iterates over all 64 squares in index order (a1, b1, ..., h8).
    pub fn all() -> impl Iterator<Item = Square> {
        (0..64).map(Square)
    }
}

impl fmt::Display for Square {
    /// Formats the square in coordinate notation, for example `e4`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = char::from(b'a' + self.file());
        let rank = char::from(b'1' + self.rank());
        write!(f, "{file}{rank}")
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// Error returned when a string is not a square in coordinate notation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSquareError;

impl fmt::Display for ParseSquareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid square: expected a file a-h followed by a rank 1-8")
    }
}

impl std::error::Error for ParseSquareError {}

impl FromStr for Square {
    type Err = ParseSquareError;

    /// Parses coordinate notation such as `e4` (lower-case file only).
    fn from_str(s: &str) -> Result<Square, ParseSquareError> {
        match *s.as_bytes() {
            [file @ b'a'..=b'h', rank @ b'1'..=b'8'] => Ok(Square::new(file - b'a', rank - b'1')),
            _ => Err(ParseSquareError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_and_centre_indices() {
        assert_eq!(Square::A1.index(), 0);
        assert_eq!(Square::H1.index(), 7);
        assert_eq!(Square::A8.index(), 56);
        assert_eq!(Square::H8.index(), 63);
        assert_eq!(Square::E4.index(), 28);
    }

    #[test]
    fn named_constants_match_their_names() {
        for square in Square::all() {
            let name = square.to_string();
            assert_eq!(name.parse::<Square>(), Ok(square));
        }
        assert_eq!(Square::E4, Square::new(4, 3));
        assert_eq!(Square::G7.to_string(), "g7");
        assert_eq!(Square::B2.to_string(), "b2");
    }

    #[test]
    fn file_and_rank_round_trip() {
        for square in Square::all() {
            assert_eq!(Square::new(square.file(), square.rank()), square);
            assert_eq!(
                square.index(),
                usize::from(square.rank() * 8 + square.file())
            );
        }
    }

    #[test]
    fn from_index_bounds() {
        assert_eq!(Square::from_index(0), Some(Square::A1));
        assert_eq!(Square::from_index(63), Some(Square::H8));
        assert_eq!(Square::from_index(64), None);
        assert_eq!(Square::from_index(255), None);
    }

    #[test]
    fn all_yields_64_distinct_squares_in_order() {
        let squares: Vec<Square> = Square::all().collect();
        assert_eq!(squares.len(), Square::COUNT);
        assert!(squares.iter().enumerate().all(|(i, sq)| sq.index() == i));
    }

    #[test]
    fn offset_moves_and_stops_at_edges() {
        assert_eq!(Square::E4.offset(0, 1), Some(Square::E5));
        assert_eq!(Square::E4.offset(-1, -1), Some(Square::D3));
        assert_eq!(Square::E4.offset(1, 2), Some(Square::F6));
        assert_eq!(Square::A1.offset(-1, 0), None);
        assert_eq!(Square::A1.offset(0, -1), None);
        assert_eq!(Square::H8.offset(1, 0), None);
        assert_eq!(Square::H8.offset(0, 1), None);
        assert_eq!(Square::H4.offset(1, 0), None, "must not wrap to a5");
    }

    #[test]
    fn parse_rejects_bad_input() {
        for bad in ["", "e", "e9", "i1", "E4", "e44", " e4", "4e", "e0", "é4"] {
            assert_eq!(bad.parse::<Square>(), Err(ParseSquareError), "{bad:?}");
        }
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn new_panics_out_of_range() {
        let _ = Square::new(8, 0);
    }
}
