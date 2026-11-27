// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! `vyvrael bench`: a fixed amount of search work, for comparing builds.
//!
//! A fixed set of positions is searched to a fixed depth at full strength,
//! and the positions visited are counted. The count depends only on the
//! search itself, not on the machine or the clock, so it is the same on
//! every run: a change that leaves the count alone has not changed what the
//! search does, and one that changes it has. The speed is printed beside it
//! and is what a pure speed change should improve.
//!
//! Full-strength search draws no random numbers, so the bench needs no seed
//! and has nothing to vary: it is what the engine plays with `Seed` fixed and
//! `Variety` 0. Each position gets a search of its own, as a new game would.

use std::time::{Duration, Instant};

use crate::board::Position;
use crate::eval::default_evaluator;
use crate::search::{Limits, Search, TranspositionTable};
use crate::uci::HASH_DEFAULT;

/// The depth every bench position is searched to, in plies.
///
/// Chosen when the main search had no move ordering and the whole bench
/// took a few seconds at this depth. It now takes a fraction of a second;
/// the depth stays through milestone 7 so that the node counts of its
/// commits can be compared (see `docs/testing/bench-history.md`).
pub const BENCH_DEPTH: u32 = 4;

/// The positions of the bench, as FEN: an opening, middlegames with and
/// without tactics, and endgames. They are searched in this order.
pub const BENCH_POSITIONS: [&str; 7] = [
    // The start position.
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    // A rook endgame with pawns (perft position 3).
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    // Promotions, checks and a queen en prise (perft position 4).
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    // A pawn about to promote, with a knight fork on the board (perft
    // position 5).
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    // A quiet Queen's Gambit Declined middlegame.
    "r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 8",
    // An Italian Game middlegame with pins on both sides, Black to move.
    "r2q1rk1/ppp2ppp/2np1n2/2b1p1B1/2B1P1b1/2NP1N2/PPP2PPP/R2Q1RK1 b - - 0 8",
    // A king and pawn endgame.
    "8/5pk1/6p1/1p1P4/1P3PP1/8/5K2/8 w - - 0 45",
];

/// What a bench run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchResult {
    /// Positions visited, over all the bench positions.
    pub nodes: u64,
    /// How long the searches took, all together.
    pub elapsed: Duration,
}

impl BenchResult {
    /// Positions visited per second, or 0 if no time could be measured.
    #[must_use]
    pub fn nodes_per_second(&self) -> u64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds > 0.0 {
            (self.nodes as f64 / seconds) as u64
        } else {
            0
        }
    }
}

/// Searches every bench position to `depth` plies, with iterative deepening
/// as in a game, and counts the positions visited. Each position gets a
/// new transposition table of the engine's default size, [`HASH_DEFAULT`],
/// so the count belongs to the build and not to a setting.
///
/// # Panics
///
/// Panics if a bench position is not a valid FEN, which the tests rule out,
/// or if there is no memory for a table of the default size.
#[must_use]
pub fn bench(depth: u32) -> BenchResult {
    bench_with_hash(depth, HASH_DEFAULT).expect("memory for the bench's table")
}

/// [`bench`] with tables of `hash_mb` MiB, or `None` if there is no memory
/// for one. The node count can differ from one table size to another: a
/// smaller table forgets more.
///
/// # Panics
///
/// Panics if a bench position is not a valid FEN, which the tests rule out.
#[must_use]
pub fn bench_with_hash(depth: u32, hash_mb: u32) -> Option<BenchResult> {
    let evaluator = default_evaluator();
    let mut nodes = 0;
    let mut elapsed = Duration::ZERO;
    for fen in BENCH_POSITIONS {
        let pos = Position::from_fen(fen).expect("the bench positions are valid");
        // A new table for each position, as a new game has.
        let mut table = TranspositionTable::new(hash_mb)?;
        let mut search = Search::new(evaluator.as_ref())
            .with_limits(Limits::depth(depth))
            .with_table(&mut table);
        let start = Instant::now();
        let _ = search.iterate(&pos, &mut |_| {});
        elapsed += start.elapsed();
        nodes += search.nodes();
    }
    Some(BenchResult { nodes, elapsed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_position_is_valid() {
        for fen in BENCH_POSITIONS {
            assert!(Position::from_fen(fen).is_ok(), "{fen}");
        }
    }

    /// The count is the same however often the bench runs.
    #[test]
    fn the_node_count_is_the_same_every_run() {
        let first = bench(3).nodes;
        assert!(first > 0);
        for _ in 0..2 {
            assert_eq!(bench(3).nodes, first);
        }
    }

    /// The hash size reaches the tables: the default size is the plain
    /// bench, a table of one slot remembers next to nothing and so visits
    /// more, and a size no machine has is an answer.
    #[test]
    fn the_hash_size_is_used() {
        let plain = bench(4).nodes;
        assert_eq!(bench_with_hash(4, HASH_DEFAULT).unwrap().nodes, plain);
        assert!(bench_with_hash(4, 0).unwrap().nodes > plain);
        assert_eq!(bench_with_hash(1, u32::MAX), None);
    }

    #[test]
    fn the_rate_is_nodes_over_seconds() {
        let result = BenchResult {
            nodes: 3_000,
            elapsed: Duration::from_millis(1_500),
        };
        assert_eq!(result.nodes_per_second(), 2_000);
        let instant = BenchResult {
            nodes: 10,
            elapsed: Duration::ZERO,
        };
        assert_eq!(instant.nodes_per_second(), 0);
    }
}
