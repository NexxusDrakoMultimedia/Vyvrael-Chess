// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Legal move generation: pseudo-legal moves with the ones that leave the
//! mover's own king attacked removed.

use super::{MoveList, pseudo_legal_moves};
use crate::board::{Move, Position};

/// Generates the legal moves of the side to move.
///
/// Each pseudo-legal move is played on a scratch copy of the position; it is
/// kept if the opponent could not then capture the mover's king. This one
/// test covers every way a move can be illegal: moving a pinned piece off its
/// line, stepping the king onto an attacked square, failing to answer a
/// check, and the en passant capture that uncovers an attack along the rank.
///
/// An empty list means checkmate if [`Position::in_check`] is true and
/// stalemate otherwise.
#[must_use]
pub fn legal_moves(pos: &Position) -> MoveList {
    let mut scratch = pos.clone();
    let mut legal = MoveList::new();
    for &mv in &pseudo_legal_moves(pos) {
        let undo = scratch.make_move(mv);
        let king_safe = !scratch.can_capture_king();
        scratch.unmake_move(mv, undo);
        if king_safe {
            legal.push(mv);
        }
    }
    legal
}

/// The legal move of `pos` written `text` in UCI notation (`e2e4`, `e7e8q`,
/// castling as the king's move `e1g1`), or `None` if there is no such legal
/// move.
///
/// This is the way to turn outside input into a move. Unlike
/// [`Position::parse_move`], it cannot return a move that is illegal or that
/// the piece cannot make, because it only ever returns a member of
/// [`legal_moves`]. Anything else, from a pinned piece's move to text that
/// is not a move at all, gives `None`.
#[must_use]
pub fn find_legal_move(pos: &Position, text: &str) -> Option<Move> {
    legal_moves(pos)
        .iter()
        .copied()
        .find(|mv| mv.to_string() == text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{MoveKind, START_FEN};

    #[test]
    fn find_legal_move_finds_every_legal_move_by_its_text() {
        for text in [
            START_FEN,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2",
        ] {
            let pos = Position::from_fen(text).unwrap();
            let moves = legal_moves(&pos);
            for &mv in &moves {
                assert_eq!(find_legal_move(&pos, &mv.to_string()), Some(mv), "{mv}");
                // For legal moves it agrees with the trusting parser.
                assert_eq!(pos.parse_move(&mv.to_string()), Ok(mv), "{mv}");
            }
            // The texts are distinct, so each names one move.
            let mut texts: Vec<String> = moves.iter().map(ToString::to_string).collect();
            texts.sort();
            texts.dedup();
            assert_eq!(texts.len(), moves.len());
        }
    }

    #[test]
    fn find_legal_move_refuses_everything_else() {
        let start = Position::startpos();
        // Moves the trusting parser accepts but no piece can make.
        for text in ["a1b3", "a1a5", "e1e3", "d1h5", "b1b3"] {
            assert!(start.parse_move(text).is_ok(), "{text}");
            assert_eq!(find_legal_move(&start, text), None, "{text}");
        }
        // The other side's move, an empty square, and things that are not
        // moves at all.
        for text in [
            "e7e5",
            "e4e5",
            "",
            "e2",
            "e2e4q",
            "e2e9",
            "E2E4",
            "e2e4 ",
            " e2e4",
            "0000",
            "e2-e4",
            "castle",
            "e2e4e5",
            "\u{e9}2e4",
        ] {
            assert_eq!(find_legal_move(&start, text), None, "{text:?}");
        }

        // A pinned knight may not move, and a king may not stay in check.
        let pinned = Position::from_fen("4r1k1/8/8/8/8/8/4N3/4K3 w - - 0 1").unwrap();
        assert!(pinned.parse_move("e2c3").is_ok());
        assert_eq!(find_legal_move(&pinned, "e2c3"), None);
        assert!(find_legal_move(&pinned, "e1d1").is_some());
        let checked = Position::from_fen("4r1k1/8/8/8/8/8/3N4/4K3 w - - 0 1").unwrap();
        assert_eq!(find_legal_move(&checked, "d2b3"), None);
        assert!(
            find_legal_move(&checked, "d2e4").is_some(),
            "blocks the check"
        );

        // A promotion needs its letter, in lower case, and only on the
        // last rank.
        let promo = Position::from_fen("8/4P1k1/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(find_legal_move(&promo, "e7e8"), None);
        assert_eq!(find_legal_move(&promo, "e7e8Q"), None);
        assert_eq!(find_legal_move(&promo, "e7e8k"), None);
        let queen = find_legal_move(&promo, "e7e8q").unwrap();
        assert_eq!(queen.promotion(), Some(crate::board::PieceKind::Queen));
        assert!(find_legal_move(&promo, "e7e8n").is_some());

        // Castling is written as the king's move, and only when legal.
        let castle = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(
            find_legal_move(&castle, "e1g1").map(Move::kind),
            Some(MoveKind::KingCastle)
        );
        let attacked = Position::from_fen("r3k2r/8/8/8/8/5r2/8/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(find_legal_move(&attacked, "e1g1"), None, "f1 is attacked");
        assert!(find_legal_move(&attacked, "e1c1").is_some());
    }

    fn legal(fen: &str) -> Vec<String> {
        let pos = Position::from_fen(fen).unwrap();
        let mut moves: Vec<String> = legal_moves(&pos).iter().map(ToString::to_string).collect();
        moves.sort();
        moves
    }

    /// The published perft depth-1 counts for the standard test positions.
    #[test]
    fn move_counts_of_the_reference_positions() {
        let positions = [
            (START_FEN, 20),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                48,
            ),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 14),
            (
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
                6,
            ),
            (
                "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
                44,
            ),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                46,
            ),
        ];
        for (fen, count) in positions {
            assert_eq!(legal(fen).len(), count, "{fen}");
        }
    }

    #[test]
    fn a_pinned_piece_may_not_leave_the_pin_line() {
        // The knight on e2 shields the king from the rook on e3.
        let moves = legal("4k3/8/8/8/8/4r3/4N3/4K3 w - - 0 1");
        assert_eq!(moves, ["e1d1", "e1d2", "e1f1", "e1f2"]);
        // A pinned rook may still move along the pin line, including
        // capturing the pinning piece.
        let along = legal("4k3/8/8/8/4r3/8/4R3/4K3 w - - 0 1");
        assert!(along.contains(&"e2e3".to_owned()));
        assert!(along.contains(&"e2e4".to_owned()));
        assert!(!along.contains(&"e2d2".to_owned()));
    }

    #[test]
    fn check_must_be_answered() {
        // Rook e2 gives check from an adjacent square and is undefended:
        // capture it or step off the e-file and second rank.
        let moves = legal("4k3/8/8/8/8/8/4r3/4K3 w - - 0 1");
        assert_eq!(moves, ["e1d1", "e1e2", "e1f1"]);
        // Check from a distance: block, capture or move the king.
        let blocks = legal("4k3/8/8/8/8/8/3B4/r3K3 w - - 0 1");
        assert_eq!(blocks, ["d2c1", "e1e2", "e1f2"]);
    }

    #[test]
    fn the_king_may_not_step_onto_an_attacked_square() {
        // Away from a rook along its line is still attacked ("x-ray").
        let moves = legal("4k3/8/8/8/8/8/8/r3K3 w - - 0 1");
        assert_eq!(moves, ["e1d2", "e1e2", "e1f2"]);
        // Kings may not stand next to each other.
        let kings = legal("8/8/8/3k4/8/3K4/8/8 w - - 0 1");
        assert_eq!(kings, ["d3c2", "d3c3", "d3d2", "d3e2", "d3e3"]);
    }

    #[test]
    fn checkmate_and_stalemate_have_no_legal_moves() {
        let mate = Position::from_fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1").unwrap();
        assert!(legal_moves(&mate).is_empty());
        assert!(mate.in_check());
        let stalemate = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(legal_moves(&stalemate).is_empty());
        assert!(!stalemate.in_check());
    }

    /// Capturing en passant removes two pawns from the fifth rank at once,
    /// which here would expose the king to the rook.
    #[test]
    fn en_passant_that_exposes_the_king_is_illegal() {
        let pinned = legal("7k/8/8/K1pP3r/8/8/8/8 w - c6 0 2");
        assert!(!pinned.contains(&"d5c6".to_owned()));
        assert!(pinned.contains(&"d5d6".to_owned()));
        // Without the rook the capture is fine.
        let free = legal("7k/8/8/K1pP4/8/8/8/8 w - c6 0 2");
        assert!(free.contains(&"d5c6".to_owned()));
    }

    #[test]
    fn legal_moves_keep_their_kinds_and_leave_the_position_untouched() {
        let pos = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let before = pos.clone();
        let moves = legal_moves(&pos);
        assert_eq!(pos, before);
        assert!(moves.iter().any(|mv| mv.kind() == MoveKind::KingCastle));
        assert!(moves.iter().any(|mv| mv.kind() == MoveKind::QueenCastle));
        // Legal moves are a subset of the pseudo-legal moves, in order.
        let pseudo = pseudo_legal_moves(&pos);
        let filtered: Vec<_> = pseudo
            .iter()
            .copied()
            .filter(|mv| moves.contains(mv))
            .collect();
        assert_eq!(filtered, moves.as_slice());
    }
}
