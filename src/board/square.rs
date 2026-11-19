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

/// Defines one documented constant per square, numbered in the order given,
/// and a table of all of them for tests.
macro_rules! named_squares {
    ($($name:ident),* $(,)?) => {
        /// Named constants for all 64 squares, `A1` to `H8`.
        impl Square {
            named_squares!(@consts 0u8; $($name),*);

            /// Every named constant with its name, in index order.
            #[cfg(test)]
            const NAMED: [(Square, &'static str); 64] =
                [$((Square::$name, stringify!($name))),*];
        }
    };
    (@consts $index:expr; $name:ident $(, $rest:ident)*) => {
        #[doc = concat!("The square `", stringify!($name), "`.")]
        pub const $name: Square = Square($index);
        named_squares!(@consts $index + 1; $($rest),*);
    };
    (@consts $index:expr;) => {};
}

#[rustfmt::skip]
named_squares! {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
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

    /// Each of the 64 named constants has the index its position in the
    /// list implies and displays as its own name.
    #[test]
    fn named_constants_match_their_names() {
        for (index, (square, name)) in Square::NAMED.into_iter().enumerate() {
            assert_eq!(square.index(), index, "{name}");
            assert_eq!(square.to_string(), name.to_ascii_lowercase());
            assert_eq!(name.to_ascii_lowercase().parse(), Ok(square));
            let file = name.as_bytes()[0] - b'A';
            let rank = name.as_bytes()[1] - b'1';
            assert_eq!(square, Square::new(file, rank), "{name}");
        }
        assert_eq!(Square::E4, Square::new(4, 3));
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
