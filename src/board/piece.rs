// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Piece kinds and coloured pieces.

use super::Color;

/// A kind of piece, without a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceKind {
    /// Pawn.
    Pawn = 0,
    /// Knight.
    Knight = 1,
    /// Bishop.
    Bishop = 2,
    /// Rook.
    Rook = 3,
    /// Queen.
    Queen = 4,
    /// King.
    King = 5,
}

impl PieceKind {
    /// Number of piece kinds.
    pub const COUNT: usize = 6;

    /// All piece kinds, in index order.
    pub const ALL: [PieceKind; PieceKind::COUNT] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    /// Index for array lookups, `0..6` in the order of [`PieceKind::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The lower-case letter for this kind: `p`, `n`, `b`, `r`, `q` or `k`.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    /// Parses a piece letter of either case.
    #[must_use]
    pub const fn from_char(c: char) -> Option<PieceKind> {
        match c.to_ascii_lowercase() {
            'p' => Some(PieceKind::Pawn),
            'n' => Some(PieceKind::Knight),
            'b' => Some(PieceKind::Bishop),
            'r' => Some(PieceKind::Rook),
            'q' => Some(PieceKind::Queen),
            'k' => Some(PieceKind::King),
            _ => None,
        }
    }
}

/// A piece of a particular colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    /// The side the piece belongs to.
    pub color: Color,
    /// What kind of piece it is.
    pub kind: PieceKind,
}

impl Piece {
    /// Creates a piece.
    #[must_use]
    pub const fn new(color: Color, kind: PieceKind) -> Piece {
        Piece { color, kind }
    }

    /// The FEN character: upper case for White (`PNBRQK`), lower case for
    /// Black (`pnbrqk`).
    #[must_use]
    pub const fn to_char(self) -> char {
        let c = self.kind.to_char();
        match self.color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }

    /// Parses a FEN piece character; the case selects the colour.
    #[must_use]
    pub const fn from_char(c: char) -> Option<Piece> {
        let Some(kind) = PieceKind::from_char(c) else {
            return None;
        };
        let color = if c.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece::new(color, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_indices_match_all_order() {
        for (i, kind) in PieceKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), i);
        }
    }

    #[test]
    fn kind_char_round_trip() {
        for kind in PieceKind::ALL {
            let c = kind.to_char();
            assert!(c.is_ascii_lowercase());
            assert_eq!(PieceKind::from_char(c), Some(kind));
            assert_eq!(PieceKind::from_char(c.to_ascii_uppercase()), Some(kind));
        }
        assert_eq!(PieceKind::from_char('x'), None);
        assert_eq!(PieceKind::from_char('1'), None);
    }

    #[test]
    fn piece_char_round_trip_covers_all_twelve() {
        let mut chars = String::new();
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                let piece = Piece::new(color, kind);
                assert_eq!(Piece::from_char(piece.to_char()), Some(piece));
                chars.push(piece.to_char());
            }
        }
        assert_eq!(chars, "PNBRQKpnbrqk");
    }

    #[test]
    fn piece_from_char_rejects_non_pieces() {
        for c in ['x', ' ', '/', '8', 'é', 'w'] {
            assert_eq!(Piece::from_char(c), None, "{c:?}");
        }
        assert_eq!(
            Piece::from_char('K'),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            Piece::from_char('n'),
            Some(Piece::new(Color::Black, PieceKind::Knight))
        );
    }
}
