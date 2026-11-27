// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The lines the engine sends: `info` during a search and `bestmove` at its
//! end.

use super::wdl;
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

/// The `info` line for one completed iteration: depth, selective depth,
/// score, nodes, speed, how full the transposition table is, time in
/// milliseconds, and the principal variation.
///
/// `hashfull` is left out for a search without a table.
///
/// `seldepth` is left out when it is 0: the root had no legal move, or the
/// line is for a move that was not found by one search from the root (a
/// weakened move), so there is no selective depth to give.
///
/// With `show_wdl` (the option `UCI_ShowWDL`) the score is followed by
/// `wdl <win> <draw> <loss>`, chances per thousand for the side to move
/// from the model of [`wdl`]: a fixed mapping of the score, not a measured
/// win rate.
#[must_use]
pub fn info_line(report: &Report, show_wdl: bool) -> String {
    let micros = report.elapsed.as_micros().max(1);
    let per_second = u128::from(report.nodes) * 1_000_000 / micros;
    let seldepth = match report.seldepth {
        0 => String::new(),
        plies => format!(" seldepth {plies}"),
    };
    let chances = if show_wdl {
        let (win, draw, loss) = wdl(report.score);
        format!(" wdl {win} {draw} {loss}")
    } else {
        String::new()
    };
    let hashfull = match report.hashfull {
        Some(per_mille) => format!(" hashfull {per_mille}"),
        None => String::new(),
    };
    let mut line = format!(
        "info depth {}{seldepth} score {}{chances} nodes {} nps {per_second}{hashfull} time {}",
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
            seldepth: 9,
            score: 31,
            nodes,
            elapsed,
            hashfull: None,
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
            info_line(
                &report(12_000, Duration::from_millis(250), vec![e2e4, promote]),
                false
            ),
            "info depth 5 seldepth 9 score cp 31 nodes 12000 nps 48000 time 250 pv e2e4 e7e8q"
        );
        // No line to show: no `pv` word at all.
        assert_eq!(
            info_line(&report(7, Duration::from_millis(2), vec![]), false),
            "info depth 5 seldepth 9 score cp 31 nodes 7 nps 3500 time 2"
        );
        // Faster than the clock can tell: no division by zero, and the speed
        // is counted as if one microsecond had passed.
        assert_eq!(
            info_line(&report(21, Duration::ZERO, vec![e2e4]), false),
            "info depth 5 seldepth 9 score cp 31 nodes 21 nps 21000000 time 0 pv e2e4"
        );
        // Whole milliseconds, rounded down, but the speed uses the full time.
        assert_eq!(
            info_line(&report(1_000, Duration::from_micros(1_500), vec![]), false),
            "info depth 5 seldepth 9 score cp 31 nodes 1000 nps 666666 time 1"
        );
    }

    /// Without a selective depth (no legal move, or a weakened move) the
    /// word is left out, not sent as 0.
    #[test]
    fn no_selective_depth_no_word() {
        let nothing = Report {
            seldepth: 0,
            ..report(1, Duration::from_millis(1), vec![])
        };
        assert_eq!(
            info_line(&nothing, false),
            "info depth 5 score cp 31 nodes 1 nps 1000 time 1"
        );
    }

    /// With a table, how full it is goes between the speed and the time.
    #[test]
    fn hashfull_when_there_is_a_table() {
        let plain = report(7, Duration::from_millis(2), vec![]);
        for (hashfull, text) in [
            (0, " hashfull 0"),
            (37, " hashfull 37"),
            (1000, " hashfull 1000"),
        ] {
            let with_table = Report {
                hashfull: Some(hashfull),
                ..plain.clone()
            };
            assert_eq!(
                info_line(&with_table, false),
                format!("info depth 5 seldepth 9 score cp 31 nodes 7 nps 3500{text} time 2")
            );
        }
    }

    /// With `UCI_ShowWDL` the chances follow the score, and nothing else
    /// about the line changes.
    #[test]
    fn chances_follow_the_score_when_asked_for() {
        let plain = report(7, Duration::from_millis(2), vec![]);
        assert_eq!(
            info_line(&plain, true),
            "info depth 5 seldepth 9 score cp 31 wdl 156 754 90 nodes 7 nps 3500 time 2"
        );
        let mate = Report {
            score: mate_in(3),
            ..plain.clone()
        };
        assert_eq!(
            info_line(&mate, true),
            "info depth 5 seldepth 9 score mate 2 wdl 1000 0 0 nodes 7 nps 3500 time 2"
        );
        assert_eq!(
            info_line(&plain, true).replace(" wdl 156 754 90", ""),
            info_line(&plain, false)
        );
    }

    #[test]
    fn best_moves() {
        let castle = Move::new(Square::E1, Square::G1, MoveKind::KingCastle);
        assert_eq!(bestmove_line(Some(castle)), "bestmove e1g1");
        assert_eq!(bestmove_line(None), "bestmove 0000");
    }
}
