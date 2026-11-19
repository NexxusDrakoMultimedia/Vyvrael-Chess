// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Castling rights.

use std::fmt;
use std::str::FromStr;

use super::{Color, Square};

/// The set of castling rights still available, as four flag bits.
///
/// A right records only that the king and the relevant rook have not moved
/// (and the rook has not been captured). Whether castling is legal right now
/// also depends on the squares between them and on attacks; that is move
/// generation's job.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CastlingRights(u8);

impl CastlingRights {
    /// No castling rights.
    pub const NONE: CastlingRights = CastlingRights(0);
    /// White may castle kingside (FEN `K`).
    pub const WHITE_KINGSIDE: CastlingRights = CastlingRights(1);
    /// White may castle queenside (FEN `Q`).
    pub const WHITE_QUEENSIDE: CastlingRights = CastlingRights(2);
    /// Black may castle kingside (FEN `k`).
    pub const BLACK_KINGSIDE: CastlingRights = CastlingRights(4);
    /// Black may castle queenside (FEN `q`).
    pub const BLACK_QUEENSIDE: CastlingRights = CastlingRights(8);
    /// All four castling rights.
    pub const ALL: CastlingRights = CastlingRights(15);

    /// Number of distinct values, for tables indexed by [`Self::index`].
    pub const COUNT: usize = 16;

    /// The four single rights with their FEN letters, in FEN order.
    const FLAGS: [(CastlingRights, char); 4] = [
        (CastlingRights::WHITE_KINGSIDE, 'K'),
        (CastlingRights::WHITE_QUEENSIDE, 'Q'),
        (CastlingRights::BLACK_KINGSIDE, 'k'),
        (CastlingRights::BLACK_QUEENSIDE, 'q'),
    ];

    /// The kingside right of `color`.
    #[must_use]
    pub const fn kingside(color: Color) -> CastlingRights {
        match color {
            Color::White => CastlingRights::WHITE_KINGSIDE,
            Color::Black => CastlingRights::BLACK_KINGSIDE,
        }
    }

    /// The queenside right of `color`.
    #[must_use]
    pub const fn queenside(color: Color) -> CastlingRights {
        match color {
            Color::White => CastlingRights::WHITE_QUEENSIDE,
            Color::Black => CastlingRights::BLACK_QUEENSIDE,
        }
    }

    /// The value as an index in `0..16`, for table lookups.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Whether no right is left.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every right in `other` is present.
    #[must_use]
    pub const fn contains(self, other: CastlingRights) -> bool {
        self.0 & other.0 == other.0
    }

    /// These rights plus those in `other`.
    #[must_use]
    pub const fn with(self, other: CastlingRights) -> CastlingRights {
        CastlingRights(self.0 | other.0)
    }

    /// These rights minus those in `other`.
    #[must_use]
    pub const fn without(self, other: CastlingRights) -> CastlingRights {
        CastlingRights(self.0 & !other.0)
    }

    /// The rights that are lost when a piece moves from or to `square`.
    ///
    /// A move from a king's or rook's home square means that piece moved; a
    /// move to a rook's home square means the rook was captured (or the right
    /// was already gone). Applying this to both the origin and destination of
    /// every move keeps the rights correct.
    #[must_use]
    pub const fn revoked_by(square: Square) -> CastlingRights {
        match square {
            Square::A1 => CastlingRights::WHITE_QUEENSIDE,
            Square::H1 => CastlingRights::WHITE_KINGSIDE,
            Square::E1 => CastlingRights(3),
            Square::A8 => CastlingRights::BLACK_QUEENSIDE,
            Square::H8 => CastlingRights::BLACK_KINGSIDE,
            Square::E8 => CastlingRights(12),
            _ => CastlingRights::NONE,
        }
    }
}

impl fmt::Display for CastlingRights {
    /// Formats the FEN castling field: a subset of `KQkq` in that order, or
    /// `-` when no right is left.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("-");
        }
        for (flag, letter) in CastlingRights::FLAGS {
            if self.contains(flag) {
                write!(f, "{letter}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CastlingRights({self})")
    }
}

/// Error returned when a string is not a valid FEN castling field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCastlingError;

impl fmt::Display for ParseCastlingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid castling rights: expected '-' or distinct letters from 'KQkq'")
    }
}

impl std::error::Error for ParseCastlingError {}

impl FromStr for CastlingRights {
    type Err = ParseCastlingError;

    /// Parses a FEN castling field: `-`, or one to four distinct letters
    /// from `KQkq`. The letters may come in any order.
    fn from_str(s: &str) -> Result<CastlingRights, ParseCastlingError> {
        if s == "-" {
            return Ok(CastlingRights::NONE);
        }
        if s.is_empty() {
            return Err(ParseCastlingError);
        }
        let mut rights = CastlingRights::NONE;
        for c in s.chars() {
            let (flag, _) = CastlingRights::FLAGS
                .into_iter()
                .find(|&(_, letter)| letter == c)
                .ok_or(ParseCastlingError)?;
            if rights.contains(flag) {
                return Err(ParseCastlingError);
            }
            rights = rights.with(flag);
        }
        Ok(rights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_values() -> impl Iterator<Item = CastlingRights> {
        (0..16).map(CastlingRights)
    }

    #[test]
    fn flags_are_distinct_bits_that_make_up_all() {
        let union =
            CastlingRights::FLAGS
                .into_iter()
                .fold(CastlingRights::NONE, |acc, (flag, _)| {
                    assert!(!acc.contains(flag));
                    acc.with(flag)
                });
        assert_eq!(union, CastlingRights::ALL);
        assert_eq!(CastlingRights::default(), CastlingRights::NONE);
        assert!(CastlingRights::NONE.is_empty());
        assert!(!CastlingRights::ALL.is_empty());
    }

    #[test]
    fn per_colour_accessors() {
        use CastlingRights as C;
        assert_eq!(C::kingside(Color::White), C::WHITE_KINGSIDE);
        assert_eq!(C::queenside(Color::White), C::WHITE_QUEENSIDE);
        assert_eq!(C::kingside(Color::Black), C::BLACK_KINGSIDE);
        assert_eq!(C::queenside(Color::Black), C::BLACK_QUEENSIDE);
    }

    #[test]
    fn with_without_contains() {
        let rights = CastlingRights::WHITE_KINGSIDE.with(CastlingRights::BLACK_QUEENSIDE);
        assert!(rights.contains(CastlingRights::WHITE_KINGSIDE));
        assert!(rights.contains(CastlingRights::BLACK_QUEENSIDE));
        assert!(!rights.contains(CastlingRights::WHITE_QUEENSIDE));
        assert!(!rights.contains(CastlingRights::ALL));
        assert!(rights.contains(CastlingRights::NONE));
        assert_eq!(
            rights.without(CastlingRights::WHITE_KINGSIDE),
            CastlingRights::BLACK_QUEENSIDE
        );
        assert_eq!(rights.without(CastlingRights::BLACK_KINGSIDE), rights);
    }

    #[test]
    fn display_is_canonical_fen() {
        assert_eq!(CastlingRights::ALL.to_string(), "KQkq");
        assert_eq!(CastlingRights::NONE.to_string(), "-");
        assert_eq!(CastlingRights(0b1001).to_string(), "Kq");
        assert_eq!(CastlingRights(0b0110).to_string(), "Qk");
        assert_eq!(
            format!("{:?}", CastlingRights(0b0101)),
            "CastlingRights(Kk)"
        );
    }

    #[test]
    fn all_sixteen_values_round_trip_and_have_unique_indices() {
        for (i, rights) in all_values().enumerate() {
            assert_eq!(rights.index(), i);
            assert_eq!(rights.to_string().parse(), Ok(rights));
        }
    }

    #[test]
    fn parse_accepts_any_order() {
        assert_eq!("qkQK".parse(), Ok(CastlingRights::ALL));
        assert_eq!("kK".parse(), Ok(CastlingRights(0b0101)));
    }

    #[test]
    fn parse_rejects_bad_input() {
        for bad in [
            "", "KK", "KQkqK", "x", "K-", "-K", "--", "kqa", " ", "KQ kq",
        ] {
            assert_eq!(
                bad.parse::<CastlingRights>(),
                Err(ParseCastlingError),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn revoked_by_home_squares_only() {
        use CastlingRights as C;
        for square in Square::all() {
            let expected = match square {
                Square::A1 => C::WHITE_QUEENSIDE,
                Square::E1 => C::WHITE_KINGSIDE.with(C::WHITE_QUEENSIDE),
                Square::H1 => C::WHITE_KINGSIDE,
                Square::A8 => C::BLACK_QUEENSIDE,
                Square::E8 => C::BLACK_KINGSIDE.with(C::BLACK_QUEENSIDE),
                Square::H8 => C::BLACK_KINGSIDE,
                _ => C::NONE,
            };
            assert_eq!(C::revoked_by(square), expected, "{square}");
        }
    }
}
