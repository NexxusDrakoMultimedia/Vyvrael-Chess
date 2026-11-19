// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Perft acceptance tests for move generation.
//!
//! Each test counts the leaf nodes of the legal move tree from a standard
//! position and compares every depth with the published reference value
//! (Chess Programming Wiki, "Perft Results",
//! <https://www.chessprogramming.org/Perft_Results>). A single wrong, missing
//! or extra move anywhere in the tree changes a count, so these tests are the
//! acceptance criterion for move generation and for make/unmake.
//!
//! Depths are chosen so the whole file runs in a few seconds in a release
//! build. Deeper counts can be checked by hand with `vyvrael perft`.

use vyvrael::board::Position;
use vyvrael::perft::perft;

/// Checks `perft` at depths 1, 2, ... against `expected`, and that the
/// position is unchanged afterwards.
#[track_caller]
fn check(fen: &str, expected: &[u64]) {
    let original = Position::from_fen(fen).unwrap();
    let mut pos = original.clone();
    for (depth, &nodes) in (1..).zip(expected) {
        assert_eq!(perft(&mut pos, depth), nodes, "depth {depth} of {fen}");
    }
    assert_eq!(pos, original, "perft must restore the position");
}

/// Position 1: the start position.
#[test]
fn start_position() {
    let counts = [20, 400, 8_902, 197_281, 4_865_609];
    check(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &counts,
    );
}

/// Position 2, "Kiwipete": castling both ways for both sides, pins, en
/// passant and promotions in a crowded middlegame.
#[test]
fn kiwipete() {
    let counts = [48, 2_039, 97_862, 4_085_603];
    check(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &counts,
    );
}

/// Position 3: a rook endgame where en passant captures can expose the
/// king along the rank, and with many checks.
#[test]
fn position_3() {
    let counts = [14, 191, 2_812, 43_238, 674_624, 11_030_083];
    check("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", &counts);
}

/// Position 4 and its colour-swapped mirror, which must give identical
/// counts: White starts in check, with promotions (including captures and
/// under-promotions) and castling.
#[test]
fn position_4_and_its_mirror() {
    let counts = [6, 264, 9_467, 422_333, 15_833_292];
    check(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &counts,
    );
    check(
        "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1",
        &counts,
    );
}

/// Position 5: a position that caught a bug in many engines, with a pawn
/// about to promote by capture next to the enemy king.
#[test]
fn position_5() {
    let counts = [44, 1_486, 62_379, 2_103_487];
    check(
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &counts,
    );
}

/// Position 6: a quiet, symmetrical middlegame with many pieces, pins by
/// bishops and no special moves, testing the ordinary case at volume.
#[test]
fn position_6() {
    let counts = [46, 2_079, 89_890, 3_894_594];
    check(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &counts,
    );
}
