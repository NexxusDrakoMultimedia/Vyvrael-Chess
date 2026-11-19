// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Walks the legal move tree of several positions and checks the position's
//! invariants at every node.
//!
//! Perft proves the *number* of positions is right. This test proves each
//! position along the way is internally sound: the incrementally updated
//! state always equals a from-scratch recomputation, and unmaking a move
//! restores the previous position exactly. Release builds do not run debug
//! assertions, so these checks are made explicitly here.

use vyvrael::board::Position;
use vyvrael::movegen::{legal_moves, pseudo_legal_moves};

/// Visits every position within `depth` plies of `pos`, checking invariants,
/// and returns how many positions were checked.
fn walk(pos: &mut Position, depth: u32) -> u64 {
    // Bitboards and mailbox agree, and the stored hash equals a recompute.
    assert!(pos.is_consistent(), "inconsistent: {pos:?}");
    // The FEN round-trips to an identical position, hash included, so the
    // parser accepts every reachable position and incremental state (castling
    // rights, en passant square, clocks) matches what the FEN says.
    let fen = pos.to_fen();
    let reparsed = Position::from_fen(&fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
    assert_eq!(&reparsed, pos, "{fen}");
    // A reachable position never lets the side to move capture the king.
    assert!(!pos.can_capture_king(), "{fen}");

    if depth == 0 {
        return 1;
    }
    let legal = legal_moves(pos);
    let pseudo = pseudo_legal_moves(pos);
    assert!(legal.iter().all(|mv| pseudo.contains(mv)), "{fen}");

    let mut visited = 1;
    for &mv in &legal {
        // Parsing the move's own notation gives the move back, kind included.
        assert_eq!(pos.parse_move(&mv.to_string()), Ok(mv), "{fen} {mv}");

        let before = pos.clone();
        let undo = pos.make_move(mv);
        assert_eq!(undo.captured().is_some(), mv.is_capture(), "{fen} {mv}");
        visited += walk(pos, depth - 1);
        pos.unmake_move(mv, undo);
        assert_eq!(pos, &before, "unmaking {mv} from {fen}");
    }
    visited
}

#[test]
fn every_position_in_the_tree_is_sound() {
    // The perft reference positions plus two with en passant available at
    // the root, one of them with the capture pinned along the rank.
    let roots = [
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            3,
        ),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            3,
        ),
        ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 4),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            3,
        ),
        (
            "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1",
            3,
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            3,
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            3,
        ),
        (
            "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
            3,
        ),
        ("7k/8/8/K1pP3r/8/8/8/8 w - c6 0 2", 4),
    ];
    let mut total = 0;
    for (fen, depth) in roots {
        let original = Position::from_fen(fen).unwrap();
        let mut pos = original.clone();
        total += walk(&mut pos, depth);
        assert_eq!(pos, original, "{fen}");
    }
    // Guards against the walk silently visiting nothing.
    assert!(total > 300_000, "only {total} positions were checked");
}
