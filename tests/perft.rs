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
