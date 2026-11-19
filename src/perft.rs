// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Perft ("performance test"): counting the positions reachable in an exact
//! number of moves.
//!
//! The counts for standard positions are published and agreed between
//! engines, so perft is the standard proof that move generation and
//! make/unmake are correct. See `docs/TESTING.md`.

use crate::board::{Move, Position};
use crate::movegen::pseudo_legal_moves;

/// Counts the leaf nodes of the legal move tree of `pos` at exactly `depth`
/// plies. Depth 0 is the position itself and counts 1.
///
/// The position is used as a scratch board and is restored before returning.
#[must_use]
pub fn perft(pos: &mut Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut nodes = 0;
    for &mv in &pseudo_legal_moves(pos) {
        let undo = pos.make_move(mv);
        if !pos.can_capture_king() {
            nodes += perft(pos, depth - 1);
        }
        pos.unmake_move(mv, undo);
    }
    nodes
}

/// Perft split by first move: each legal move of `pos` with the number of
/// leaf nodes below it at `depth` plies in total (so `depth - 1` after the
/// move). The counts add up to `perft(pos, depth)`.
///
/// This is the debugging tool for a wrong perft count: compare the list with
/// a trusted engine's, follow the move whose count differs, and repeat one
/// ply deeper until the faulty position is found.
///
/// Returns an empty list when `depth` is 0 or there are no legal moves. The
/// moves come in generation order.
#[must_use]
pub fn divide(pos: &mut Position, depth: u32) -> Vec<(Move, u64)> {
    let mut counts = Vec::new();
    if depth == 0 {
        return counts;
    }
    for &mv in &pseudo_legal_moves(pos) {
        let undo = pos.make_move(mv);
        if !pos.can_capture_king() {
            counts.push((mv, perft(pos, depth - 1)));
        }
        pos.unmake_move(mv, undo);
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::legal_moves;

    #[test]
    fn shallow_counts_from_the_start_position() {
        let mut pos = Position::startpos();
        assert_eq!(perft(&mut pos, 0), 1);
        assert_eq!(perft(&mut pos, 1), 20);
        assert_eq!(perft(&mut pos, 2), 400);
        assert_eq!(perft(&mut pos, 3), 8_902);
        assert_eq!(pos, Position::startpos(), "perft must restore the position");
    }

    #[test]
    fn depth_one_equals_the_number_of_legal_moves() {
        for fen in [
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1",
        ] {
            let mut pos = Position::from_fen(fen).unwrap();
            let legal = legal_moves(&pos).len() as u64;
            assert_eq!(perft(&mut pos, 1), legal, "{fen}");
        }
    }

    #[test]
    fn divide_splits_perft_by_first_move() {
        let mut pos = Position::startpos();
        let split = divide(&mut pos, 3);
        assert_eq!(split.len(), 20);
        assert_eq!(split.iter().map(|&(_, n)| n).sum::<u64>(), 8_902);
        // Each first move is legal and listed once.
        let legal = legal_moves(&pos);
        assert!(split.iter().all(|(mv, _)| legal.contains(mv)));
        // 1. e4 opens lines for the queen and bishop: 600 replies-and-replies.
        let e4 = split
            .iter()
            .find(|(mv, _)| mv.to_string() == "e2e4")
            .unwrap();
        assert_eq!(e4.1, 600);
        let a3 = split
            .iter()
            .find(|(mv, _)| mv.to_string() == "a2a3")
            .unwrap();
        assert_eq!(a3.1, 380);
        assert_eq!(pos, Position::startpos());
    }

    #[test]
    fn divide_edge_cases() {
        let mut pos = Position::startpos();
        assert!(divide(&mut pos, 0).is_empty());
        let depth_one = divide(&mut pos, 1);
        assert_eq!(depth_one.len(), 20);
        assert!(depth_one.iter().all(|&(_, n)| n == 1));
        let mut mate = Position::from_fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1").unwrap();
        assert!(divide(&mut mate, 3).is_empty());
        assert_eq!(perft(&mut mate, 3), 0);
    }
}
