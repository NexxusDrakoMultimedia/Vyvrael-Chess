// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The bench's node count, pinned.
//!
//! The count changes only when what the search does changes: a pure speed
//! change must leave it alone. When a search change is made on purpose (a
//! heuristic of milestone 7, say), the new count goes here in the same
//! commit, and into `docs/testing/bench-history.md`.

use vyvrael::bench::{BENCH_DEPTH, BENCH_POSITIONS, bench};

/// The count of `vyvrael bench` at the default depth. It was 16,611,416
/// from 2026-11-22 until milestone 7 began to change the search;
/// `docs/testing/bench-history.md` has every value since.
const BENCH_NODES: u64 = 60_070;

#[test]
fn the_bench_node_count_is_pinned() {
    assert_eq!(BENCH_DEPTH, 4);
    assert_eq!(BENCH_POSITIONS.len(), 7);
    assert_eq!(bench(BENCH_DEPTH).nodes, BENCH_NODES);
}
