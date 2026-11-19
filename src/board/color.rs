// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Side colours.

use std::ops::Not;

/// One of the two sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Color {
    /// The side that moves first.
    White = 0,
    /// The side that moves second.
    Black = 1,
}

impl Color {
    /// Number of colours.
    pub const COUNT: usize = 2;

    /// Both colours, in index order.
    pub const ALL: [Color; Color::COUNT] = [Color::White, Color::Black];

    /// Index for array lookups: 0 for white, 1 for black.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The other colour.
    #[must_use]
    pub const fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    /// The FEN side-to-move character: `w` or `b`.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }

    /// Parses a FEN side-to-move character (`w` or `b`, case-sensitive).
    #[must_use]
    pub const fn from_char(c: char) -> Option<Color> {
        match c {
            'w' => Some(Color::White),
            'b' => Some(Color::Black),
            _ => None,
        }
    }
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Color {
        self.flip()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_match_all_order() {
        for (i, color) in Color::ALL.into_iter().enumerate() {
            assert_eq!(color.index(), i);
        }
    }

    #[test]
    fn flip_is_an_involution() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
        assert_eq!(!Color::White, Color::Black);
        for color in Color::ALL {
            assert_eq!(color.flip().flip(), color);
        }
    }

    #[test]
    fn char_round_trip() {
        for color in Color::ALL {
            assert_eq!(Color::from_char(color.to_char()), Some(color));
        }
        assert_eq!(Color::from_char('W'), None);
        assert_eq!(Color::from_char('x'), None);
    }
}
