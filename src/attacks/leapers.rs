// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Attack tables for the pieces that jump a fixed pattern: knights, kings
//! and pawns. The squares they attack do not depend on other pieces, so one
//! lookup per square is enough. The tables are built at compile time.

use crate::board::{Bitboard, Color, Square};

/// The squares a knight on the single-square set `from` attacks.
const fn knight_pattern(from: Bitboard) -> Bitboard {
    let up = from.north().north();
    let down = from.south().south();
    let right = from.east().east();
    let left = from.west().west();
    Bitboard(
        up.east().0
            | up.west().0
            | down.east().0
            | down.west().0
            | right.north().0
            | right.south().0
            | left.north().0
            | left.south().0,
    )
}

static KNIGHT_ATTACKS: [Bitboard; Square::COUNT] = {
    let mut table = [Bitboard::EMPTY; Square::COUNT];
    let mut index = 0;
    while index < Square::COUNT {
        table[index] = knight_pattern(Bitboard(1 << index));
        index += 1;
    }
    table
};

/// The squares a knight on `square` attacks.
#[must_use]
pub fn knight_attacks(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square.index()]
}

/// The squares a king on the single-square set `from` attacks.
const fn king_pattern(from: Bitboard) -> Bitboard {
    Bitboard(
        from.north().0
            | from.south().0
            | from.east().0
            | from.west().0
            | from.north_east().0
            | from.north_west().0
            | from.south_east().0
            | from.south_west().0,
    )
}

static KING_ATTACKS: [Bitboard; Square::COUNT] = {
    let mut table = [Bitboard::EMPTY; Square::COUNT];
    let mut index = 0;
    while index < Square::COUNT {
        table[index] = king_pattern(Bitboard(1 << index));
        index += 1;
    }
    table
};

/// The squares a king on `square` attacks (castling is not an attack).
#[must_use]
pub fn king_attacks(square: Square) -> Bitboard {
    KING_ATTACKS[square.index()]
}

static PAWN_ATTACKS: [[Bitboard; Square::COUNT]; Color::COUNT] = {
    let mut table = [[Bitboard::EMPTY; Square::COUNT]; Color::COUNT];
    let mut index = 0;
    while index < Square::COUNT {
        let from = Bitboard(1 << index);
        table[Color::White.index()][index] = Bitboard(from.north_east().0 | from.north_west().0);
        table[Color::Black.index()][index] = Bitboard(from.south_east().0 | from.south_west().0);
        index += 1;
    }
    table
};

/// The squares a pawn of `color` on `square` attacks: the two squares
/// diagonally in front of it. Pushes are moves, not attacks.
///
/// Defined for every square, including ranks a pawn never stands on, because
/// the table is also used backwards: the pawns of `color` that attack a
/// square `s` stand on `pawn_attacks(color.flip(), s)`.
#[must_use]
pub fn pawn_attacks(color: Color, square: Square) -> Bitboard {
    PAWN_ATTACKS[color.index()][square.index()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the expected attack set from (file, rank) steps with
    /// `Square::offset`, which cannot wrap around the board.
    fn from_steps(square: Square, steps: &[(i8, i8)]) -> Bitboard {
        steps
            .iter()
            .filter_map(|&(files, ranks)| square.offset(files, ranks))
            .collect()
    }

    const KNIGHT_STEPS: [(i8, i8); 8] = [
        (1, 2),
        (2, 1),
        (2, -1),
        (1, -2),
        (-1, -2),
        (-2, -1),
        (-2, 1),
        (-1, 2),
    ];

    #[test]
    fn knight_attacks_match_the_step_definition() {
        for square in Square::all() {
            let expected = from_steps(square, &KNIGHT_STEPS);
            assert_eq!(knight_attacks(square), expected, "{square}");
        }
    }

    #[test]
    fn knight_attack_counts() {
        assert_eq!(knight_attacks(Square::A1).count(), 2);
        assert_eq!(knight_attacks(Square::H8).count(), 2);
        assert_eq!(knight_attacks(Square::B1).count(), 3);
        assert_eq!(knight_attacks(Square::A4).count(), 4);
        assert_eq!(knight_attacks(Square::B2).count(), 4);
        assert_eq!(knight_attacks(Square::E4).count(), 8);
        let total: u32 = Square::all().map(|sq| knight_attacks(sq).count()).sum();
        assert_eq!(total, 336);
    }

    #[test]
    fn king_attacks_match_the_step_definition() {
        let steps: Vec<(i8, i8)> = (-1..=1)
            .flat_map(|files| (-1..=1).map(move |ranks| (files, ranks)))
            .filter(|&step| step != (0, 0))
            .collect();
        assert_eq!(steps.len(), 8);
        for square in Square::all() {
            assert_eq!(king_attacks(square), from_steps(square, &steps), "{square}");
        }
    }

    #[test]
    fn king_attack_counts() {
        assert_eq!(king_attacks(Square::A1).count(), 3);
        assert_eq!(king_attacks(Square::H8).count(), 3);
        assert_eq!(king_attacks(Square::E1).count(), 5);
        assert_eq!(king_attacks(Square::A5).count(), 5);
        assert_eq!(king_attacks(Square::E4).count(), 8);
        let total: u32 = Square::all().map(|sq| king_attacks(sq).count()).sum();
        assert_eq!(total, 420);
    }

    #[test]
    fn pawn_attacks_match_the_step_definition() {
        for square in Square::all() {
            let white = from_steps(square, &[(-1, 1), (1, 1)]);
            let black = from_steps(square, &[(-1, -1), (1, -1)]);
            assert_eq!(pawn_attacks(Color::White, square), white, "{square}");
            assert_eq!(pawn_attacks(Color::Black, square), black, "{square}");
        }
    }

    #[test]
    fn pawn_attack_examples() {
        let e5 = pawn_attacks(Color::White, Square::E4);
        assert_eq!(e5, Bitboard::EMPTY.with(Square::D5).with(Square::F5));
        let a_file = pawn_attacks(Color::White, Square::A2);
        assert_eq!(a_file, Bitboard::from_square(Square::B3));
        let h_file = pawn_attacks(Color::Black, Square::H7);
        assert_eq!(h_file, Bitboard::from_square(Square::G6));
        assert!(pawn_attacks(Color::White, Square::E8).is_empty());
        assert!(pawn_attacks(Color::Black, Square::E1).is_empty());
        for color in Color::ALL {
            let total: u32 = Square::all()
                .map(|sq| pawn_attacks(color, sq).count())
                .sum();
            assert_eq!(total, 98, "{color:?}");
        }
    }

    /// The reverse lookup: `a` attacks `b` exactly when `a` is in the
    /// opposite colour's attack set of `b`.
    #[test]
    fn pawn_attacks_reverse_with_the_other_colour() {
        for color in Color::ALL {
            for a in Square::all() {
                for b in Square::all() {
                    let forward = pawn_attacks(color, a).contains(b);
                    let backward = pawn_attacks(color.flip(), b).contains(a);
                    assert_eq!(forward, backward, "{color:?} {a} {b}");
                }
            }
        }
    }

    #[test]
    fn knight_attacks_are_symmetric() {
        for a in Square::all() {
            for b in knight_attacks(a) {
                assert!(knight_attacks(b).contains(a), "{a} {b}");
            }
        }
    }
}
