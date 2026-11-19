// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Attack tables for the pieces that jump a fixed pattern: knights, kings
//! and pawns. The squares they attack do not depend on other pieces, so one
//! lookup per square is enough. The tables are built at compile time.

use crate::board::{Bitboard, Square};

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
    fn knight_attacks_are_symmetric() {
        for a in Square::all() {
            for b in knight_attacks(a) {
                assert!(knight_attacks(b).contains(a), "{a} {b}");
            }
        }
    }
}
