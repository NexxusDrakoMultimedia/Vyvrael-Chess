// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The lines the engine sends: `info` during a search and `bestmove` at its
//! end.

use crate::board::Move;
use crate::eval::Score;
use crate::search::{Report, moves_to_mate};

/// A score as the protocol writes it: `cp <centipawns>` for an evaluation,
/// `mate <moves>` for a forced mate, negative when the engine is the one
/// being mated. Both are from the point of view of the side to move.
#[must_use]
pub fn score_text(score: Score) -> String {
    match moves_to_mate(score) {
        Some(moves) => format!("mate {moves}"),
        None => format!("cp {score}"),
    }
}

/// The `info` line for one completed iteration: depth, score, nodes, speed,
/// time in milliseconds, and the principal variation.
#[must_use]
pub fn info_line(report: &Report) -> String {
    let micros = report.elapsed.as_micros().max(1);
    let per_second = u128::from(report.nodes) * 1_000_000 / micros;
    let mut line = format!(
        "info depth {} score {} nodes {} nps {per_second} time {}",
        report.depth,
        score_text(report.score),
        report.nodes,
        report.elapsed.as_millis()
    );
    if !report.pv.is_empty() {
        line.push_str(" pv");
        for mv in &report.pv {
            line.push(' ');
            line.push_str(&mv.to_string());
        }
    }
    line
}

/// The `bestmove` line. A position with no legal move (checkmate or
/// stalemate) has no best move; the protocol's convention for that is the
/// null move `0000`.
#[must_use]
pub fn bestmove_line(best: Option<Move>) -> String {
    match best {
        Some(mv) => format!("bestmove {mv}"),
        None => "bestmove 0000".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::board::{MoveKind, PieceKind, Square};
    use crate::search::{mate_in, mated_in};

    #[test]
    fn scores() {
        assert_eq!(score_text(0), "cp 0");
        assert_eq!(score_text(37), "cp 37");
        assert_eq!(score_text(-1250), "cp -1250");
        // Mate is counted in the engine's own moves, not plies.
        assert_eq!(score_text(mate_in(1)), "mate 1");
        assert_eq!(score_text(mate_in(3)), "mate 2");
        assert_eq!(score_text(mate_in(4)), "mate 2");
        assert_eq!(score_text(mated_in(2)), "mate -1");
        assert_eq!(score_text(mated_in(0)), "mate 0");
    }

    fn report(nodes: u64, elapsed: Duration, pv: Vec<Move>) -> Report {
        Report {
            depth: 5,
            score: 31,
            nodes,
            elapsed,
            pv,
        }
    }

    #[test]
    fn info_lines() {
        let e2e4 = Move::new(Square::E2, Square::E4, MoveKind::DoublePush);
        let promote = Move::new(
            Square::E7,
            Square::E8,
            MoveKind::Promotion(PieceKind::Queen),
        );
        assert_eq!(
            info_line(&report(
                12_000,
                Duration::from_millis(250),
                vec![e2e4, promote]
            )),
            "info depth 5 score cp 31 nodes 12000 nps 48000 time 250 pv e2e4 e7e8q"
        );
        // No line to show: no `pv` word at all.
        assert_eq!(
            info_line(&report(7, Duration::from_millis(2), vec![])),
            "info depth 5 score cp 31 nodes 7 nps 3500 time 2"
        );
        // Faster than the clock can tell: no division by zero, and the speed
        // is counted as if one microsecond had passed.
        assert_eq!(
            info_line(&report(21, Duration::ZERO, vec![e2e4])),
            "info depth 5 score cp 31 nodes 21 nps 21000000 time 0 pv e2e4"
        );
        // Whole milliseconds, rounded down, but the speed uses the full time.
        assert_eq!(
            info_line(&report(1_000, Duration::from_micros(1_500), vec![])),
            "info depth 5 score cp 31 nodes 1000 nps 666666 time 1"
        );
    }

    #[test]
    fn best_moves() {
        let castle = Move::new(Square::E1, Square::G1, MoveKind::KingCastle);
        assert_eq!(bestmove_line(Some(castle)), "bestmove e1g1");
        assert_eq!(bestmove_line(None), "bestmove 0000");
    }
}
