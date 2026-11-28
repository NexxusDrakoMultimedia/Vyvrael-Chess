// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Pinned results of fixed-depth searches.
//!
//! A fixed-depth search uses no clock and no randomness, so its score, best
//! move and node count are fully determined by the code. These values were
//! **recorded from this implementation** on 2026-11-20; they are not an
//! independent reference the way perft counts are. Their job is to notice
//! change: any edit to move generation order, evaluation or search that
//! alters what the engine does at fixed depth will fail here.
//!
//! When a deliberate change makes one of these fail, check that the new
//! behaviour is intended (for a search or evaluation change, that means an
//! SPRT result), then update the numbers in the same commit and say so in
//! its message. Never update them just to make the test pass.

use vyvrael::board::Position;
use vyvrael::eval::HandCrafted;
use vyvrael::search::{Limits, Search};

/// (FEN, depth, score in centipawns, total nodes, principal variation).
const PINS: [(&str, u32, i32, u64, &str); 7] = [
    (
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        4,
        0,
        25_541,
        "d2d4 d7d5 b1c3 b8c6",
    ),
    (
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        2,
        44,
        3_538,
        "e2a6 b4c3",
    ),
    (
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        5,
        107,
        15_159,
        "b4f4 h4g3 f4f7 c7c6 f7g7",
    ),
    (
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        2,
        -418,
        968,
        "c4c5 b6c5",
    ),
    (
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        2,
        541,
        476,
        "d7c8q d8c8",
    ),
    (
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        2,
        22,
        1_301,
        "c3d5 e7d8",
    ),
    (
        "4k3/8/3p4/4p3/8/8/4Q3/4K3 w - - 0 1",
        4,
        683,
        10_055,
        "e1d2 e8e7 e2c4 e7f6",
    ),
];

#[test]
fn fixed_depth_searches_give_the_recorded_results() {
    for (fen, depth, score, nodes, pv) in PINS {
        let pos = Position::from_fen(fen).unwrap();
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::depth(depth));
        let outcome = searcher.iterate(&pos, &mut |_| {});

        let line: Vec<String> = outcome.pv.iter().map(ToString::to_string).collect();
        assert_eq!(
            (outcome.score, searcher.nodes(), line.join(" ").as_str()),
            (score, nodes, pv),
            "{fen} at depth {depth}"
        );
        assert!(!searcher.was_stopped());
    }
}

/// The search is a pure function of the position and the depth.
#[test]
fn searching_twice_gives_the_same_result() {
    let (fen, depth, ..) = PINS[2];
    let run = || {
        let pos = Position::from_fen(fen).unwrap();
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::depth(depth));
        let outcome = searcher.iterate(&pos, &mut |_| {});
        (outcome, searcher.nodes())
    };
    assert_eq!(run(), run());
}
