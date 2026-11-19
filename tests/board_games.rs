// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Whole-game regression tests for the board: FEN, move parsing, make/unmake
//! and hashing working together through the public API only.
//!
//! Each game is played move by move from the starting position. After every
//! move the position must be internally consistent and its FEN must parse
//! back to an identical position (which also proves the incrementally
//! maintained hash equals a from-scratch hash). Known positions along the way
//! and at the end are compared with their FENs. Then every move is taken
//! back and each earlier position must be restored exactly.

use vyvrael::board::{Move, Position, Undo};

/// Plays `moves` (UCI notation, space separated) from the starting position,
/// checking the position after each one, and returns the final position with
/// the history needed to unwind it.
fn play(moves: &str, checkpoints: &[(usize, &str)]) -> (Position, Vec<(Position, Move, Undo)>) {
    let mut pos = Position::startpos();
    let mut history = Vec::new();
    for (index, text) in moves.split_whitespace().enumerate() {
        let ply = index + 1;
        let mv = pos
            .parse_move(text)
            .unwrap_or_else(|e| panic!("ply {ply} ({text}): {e}"));
        assert_eq!(mv.to_string(), text, "ply {ply}");

        let before = pos.clone();
        let undo = pos.make_move(mv);
        history.push((before, mv, undo));

        assert!(pos.is_consistent(), "ply {ply} ({text})");
        assert_eq!(pos.hash(), pos.compute_hash(), "ply {ply} ({text})");
        let reparsed = Position::from_fen(&pos.to_fen())
            .unwrap_or_else(|e| panic!("ply {ply} ({text}) gives a FEN we reject: {e}"));
        assert_eq!(reparsed, pos, "ply {ply} ({text})");

        for &(at, fen) in checkpoints {
            if at == ply {
                assert_eq!(pos.to_fen(), fen, "after ply {ply} ({text})");
            }
        }
    }
    assert!(
        checkpoints.iter().all(|&(at, _)| at <= history.len()),
        "a checkpoint lies beyond the end of the game"
    );
    (pos, history)
}

/// Takes back every move, checking each earlier position is restored exactly.
fn unwind(mut pos: Position, mut history: Vec<(Position, Move, Undo)>) {
    while let Some((before, mv, undo)) = history.pop() {
        pos.unmake_move(mv, undo);
        assert_eq!(pos, before, "unmaking {mv}");
    }
    assert_eq!(pos, Position::startpos());
}

/// Morphy v Duke of Brunswick and Count Isouard, Paris 1858 ("the Opera
/// Game"): double pushes, captures, checks and queenside castling.
#[test]
fn opera_game() {
    let moves = "e2e4 e7e5 g1f3 d7d6 d2d4 c8g4 d4e5 g4f3 d1f3 d6e5 f1c4 g8f6 \
                 f3b3 d8e7 b1c3 c7c6 c1g5 b7b5 c3b5 c6b5 c4b5 b8d7 e1c1 a8d8 \
                 d1d7 d8d7 h1d1 e7e6 b5d7 f6d7 b3b8 d7b8 d1d8";
    let checkpoints = [
        (
            1,
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        ),
        // After 12. O-O-O Rd8: White has castled, Black lost the q right.
        (
            24,
            "3rkb1r/p2nqppp/5n2/1B2p1B1/4P3/1Q6/PPP2PPP/2KR3R w k - 3 13",
        ),
        // The final position: 17. Rd8#.
        (33, "1n1Rkb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2K5 b k - 1 17"),
    ];
    let (pos, history) = play(moves, &checkpoints);
    assert_eq!(history.len(), 33);
    unwind(pos, history);
}

/// A constructed game that exercises what the Opera Game does not: en
/// passant, promotion by capture (to queen and to knight), a rook captured
/// on its home square, and kingside castling by both sides.
#[test]
fn special_moves_game() {
    let moves = "e2e4 a7a5 e4e5 d7d5 e5d6 a5a4 d6c7 a4a3 c7b8q a3b2 \
                 g1f3 b2a1n f1e2 g8f6 e1g1 e7e6 b8a8 f8e7 d2d4 e8g8";
    let checkpoints = [
        // 2... d5 beside the e5 pawn: en passant is available.
        (
            4,
            "rnbqkbnr/1pp1pppp/8/p2pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        ),
        // 3. exd6 e.p.
        (
            5,
            "rnbqkbnr/1pp1pppp/3P4/p7/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3",
        ),
        // 6... bxa1=N: both promotions done, White lost the Q right.
        (
            12,
            "rQbqkbnr/1p2pppp/8/8/8/5N2/P1PP1PPP/nNBQKB1R w Kkq - 0 7",
        ),
        // 10... O-O: the final position.
        (
            20,
            "Q1bq1rk1/1p2bppp/4pn2/8/3P4/5N2/P1P1BPPP/nNBQ1RK1 w - - 1 11",
        ),
    ];
    let (pos, history) = play(moves, &checkpoints);
    assert_eq!(history.len(), 20);
    unwind(pos, history);
}
