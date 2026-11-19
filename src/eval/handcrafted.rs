// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The hand-crafted evaluation: material plus piece-square tables, tapered
//! by game phase.

use super::{
    Evaluator, Score, Tapered, game_phase, material_balance, piece_square_balance, relative_to,
};
use crate::board::Position;

/// The parts of a hand-crafted evaluation, for inspection and debugging.
/// Every score here is "White minus Black".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakdown {
    /// Material balance in centipawns.
    pub material: Score,
    /// Piece-square balance, before blending.
    pub piece_square: Tapered,
    /// Game phase, from 0 (endgame) to 24 (all pieces on).
    pub phase: i32,
    /// The final evaluation from White's point of view: material plus the
    /// piece-square balance blended at `phase`.
    pub total: Score,
}

/// The hand-crafted evaluator: material plus piece-square tables, with the
/// tables blended between their middlegame and endgame values by the game
/// phase.
///
/// This is the engine's baseline evaluation. It stays available as the
/// fallback and as the yardstick once a neural network exists.
#[derive(Debug, Clone, Copy, Default)]
pub struct HandCrafted;

impl HandCrafted {
    /// Evaluates `pos` and returns each part of the result.
    #[must_use]
    pub fn breakdown(pos: &Position) -> Breakdown {
        let material = material_balance(pos);
        let piece_square = piece_square_balance(pos);
        let phase = game_phase(pos);
        Breakdown {
            material,
            piece_square,
            phase,
            total: material + piece_square.blend(phase),
        }
    }
}

impl Evaluator for HandCrafted {
    fn evaluate(&self, pos: &Position) -> Score {
        relative_to(pos.side_to_move(), HandCrafted::breakdown(pos).total)
    }

    fn name(&self) -> &'static str {
        "hand-crafted"
    }
}

/// The evaluator the engine uses unless told otherwise.
///
/// Today that is always [`HandCrafted`]. When the neural network arrives
/// (milestone 9) this is where the choice between the two is made.
#[must_use]
pub fn default_evaluator() -> Box<dyn Evaluator> {
    Box::new(HandCrafted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::START_FEN;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    fn eval(text: &str) -> Score {
        HandCrafted.evaluate(&fen(text))
    }

    /// The same position with the colours swapped: the board flipped top to
    /// bottom, every piece recoloured, and the side to move, castling rights
    /// and en passant square swapped to match.
    fn swap_colours(text: &str) -> String {
        let swap_case = |c: char| {
            if c.is_ascii_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c.to_ascii_uppercase()
            }
        };
        let fields: Vec<&str> = text.split(' ').collect();
        let board: Vec<String> = fields[0]
            .split('/')
            .rev()
            .map(|rank| rank.chars().map(swap_case).collect())
            .collect();
        let side = if fields[1] == "w" { "b" } else { "w" };
        let castling = if fields[2] == "-" {
            "-".to_owned()
        } else {
            // Keep the canonical KQkq order after swapping case.
            let swapped: String = fields[2].chars().map(swap_case).collect();
            "KQkq".chars().filter(|&c| swapped.contains(c)).collect()
        };
        let en_passant = match fields[3].as_bytes() {
            [file, b'3'] => format!("{}6", *file as char),
            [file, b'6'] => format!("{}3", *file as char),
            _ => "-".to_owned(),
        };
        format!(
            "{} {side} {castling} {en_passant} {} {}",
            board.join("/"),
            fields[4],
            fields[5]
        )
    }

    const POSITIONS: [&str; 8] = [
        START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        "8/8/8/4k3/8/8/4P3/K7 b - - 3 40",
    ];

    #[test]
    fn swap_colours_helper_is_an_involution_that_mirrors() {
        for position in POSITIONS {
            let swapped = swap_colours(position);
            assert_eq!(swap_colours(&swapped), position);
            assert!(Position::from_fen(&swapped).is_ok(), "{swapped}");
        }
        assert_eq!(
            swap_colours("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"),
            "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1"
        );
    }

    /// The evaluation must not care which colour it is looking at: the side
    /// to move sees the same score in a position and in its colour-swapped
    /// mirror.
    #[test]
    fn evaluation_is_colour_symmetric() {
        for position in POSITIONS {
            let swapped = swap_colours(position);
            assert_eq!(eval(position), eval(&swapped), "{position}");
            // And the White-relative total changes sign.
            let a = HandCrafted::breakdown(&fen(position));
            let b = HandCrafted::breakdown(&fen(&swapped));
            assert_eq!(a.total, -b.total, "{position}");
            assert_eq!(a.phase, b.phase);
            assert_eq!(a.material, -b.material);
            assert_eq!(a.piece_square, -b.piece_square);
        }
    }

    /// Handing the move to the other side negates the score.
    #[test]
    fn side_to_move_only_changes_the_sign() {
        assert_eq!(
            eval("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1"),
            -eval("4k3/8/8/8/3Q4/8/8/4K3 b - - 0 1")
        );
    }

    #[test]
    fn the_start_position_is_level() {
        let breakdown = HandCrafted::breakdown(&Position::startpos());
        assert_eq!(breakdown.material, 0);
        assert_eq!(breakdown.piece_square, Tapered::ZERO);
        assert_eq!(breakdown.phase, 24);
        assert_eq!(breakdown.total, 0);
        assert_eq!(eval(START_FEN), 0);
    }

    #[test]
    fn breakdown_adds_up() {
        // After 1. e4: the pawn left e2 (-16 mg, 6 eg) for e4 (24 mg, 18 eg).
        let pos = fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        let breakdown = HandCrafted::breakdown(&pos);
        assert_eq!(breakdown.material, 0);
        assert_eq!(breakdown.piece_square, Tapered::new(40, 12));
        assert_eq!(breakdown.phase, 24);
        assert_eq!(breakdown.total, 40);
        // Black is to move, so the evaluator reports it as bad for Black.
        assert_eq!(HandCrafted.evaluate(&pos), -40);
    }

    #[test]
    fn material_dominates_placement() {
        // A knight up is clearly winning wherever the pieces stand.
        assert!(eval("4k3/8/8/8/8/8/8/N3K3 w - - 0 1") > 200);
        assert!(eval("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1") > 800);
        // A knight in the corner with an extra pawn still beats a lone
        // centralised knight: placement does not outweigh a pawn here.
        assert!(eval("4k3/8/8/8/8/8/P7/N3K3 w - - 0 1") > eval("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1"));
    }

    #[test]
    fn tapering_changes_what_a_square_is_worth() {
        // The same advanced white king on e4 against a black king at home,
        // with the other pieces placed symmetrically so only the kings count.
        // The king balance is -33 in the middlegame and +44 in the endgame.
        let kings = Tapered::new(-33, 44);
        let heavy = HandCrafted::breakdown(&fen("r2qk2r/8/8/8/4K3/8/8/R2Q3R w - - 0 1"));
        let queens = HandCrafted::breakdown(&fen("3qk3/8/8/8/4K3/8/8/3Q4 w - - 0 1"));
        let bare = HandCrafted::breakdown(&fen("4k3/8/8/8/4K3/8/8/8 w - - 0 1"));
        for parts in [heavy, queens, bare] {
            assert_eq!(parts.material, 0);
            assert_eq!(parts.piece_square, kings);
        }
        // Queens and rooks on (phase 16): the exposed king is a liability.
        // (-33 * 16 + 44 * 8) / 24 = -176 / 24, which rounds towards zero.
        assert_eq!((heavy.phase, heavy.total), (16, -7));
        // Queens only (phase 8): (-33 * 8 + 44 * 16) / 24 = 440 / 24.
        assert_eq!((queens.phase, queens.total), (8, 18));
        // Bare kings (phase 0): only the endgame half counts.
        assert_eq!((bare.phase, bare.total), (0, 44));
        assert!(heavy.total < 0 && queens.total < bare.total);
    }

    /// The total is always material plus the piece-square pair blended at
    /// the position's own phase. The sample positions are mostly full boards,
    /// so four more are added to cover the phases in between.
    #[test]
    fn total_is_material_plus_the_blend_at_the_positions_phase() {
        let in_between = [
            "r2qk2r/8/8/8/4K3/8/8/R2Q3R w - - 0 1",
            "3qk3/8/8/8/4K3/8/8/3Q4 w - - 0 1",
            "r1bqk2r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 0 1",
            "2r3k1/5ppp/8/8/8/8/5PPP/2R1N1K1 w - - 0 1",
        ];
        let mut phases = Vec::new();
        for position in POSITIONS.into_iter().chain(in_between) {
            let parts = HandCrafted::breakdown(&fen(position));
            assert_eq!(
                parts.total,
                parts.material + parts.piece_square.blend(parts.phase),
                "{position}"
            );
            phases.push(parts.phase);
        }
        // The positions really do cover different phases.
        phases.sort_unstable();
        phases.dedup();
        assert_eq!(phases, [0, 4, 5, 8, 16, 22, 24]);
    }

    #[test]
    fn default_evaluator_is_the_hand_crafted_one() {
        let evaluator = default_evaluator();
        assert_eq!(evaluator.name(), "hand-crafted");
        assert_eq!(HandCrafted.name(), "hand-crafted");
        for position in POSITIONS {
            let pos = fen(position);
            assert_eq!(evaluator.evaluate(&pos), HandCrafted.evaluate(&pos));
        }
    }
}
