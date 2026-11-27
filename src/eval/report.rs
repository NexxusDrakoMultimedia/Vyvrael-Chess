// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The evaluation of a position written out for a person to read.

use super::{Evaluator, HandCrafted, MAX_PHASE};
use crate::board::Position;

/// The hand-crafted evaluation of `pos` with its parts, one line of text
/// each: the position, the evaluator's name, the material balance, the
/// piece-square balance for the middlegame and the endgame, the game phase,
/// and the total from White's point of view and from the mover's.
///
/// It evaluates only; nothing is searched. The `eval` command of the binary
/// prints these lines as they are, and the UCI loop sends them as `info
/// string` lines.
#[must_use]
pub fn report(pos: &Position) -> Vec<String> {
    let parts = HandCrafted::breakdown(pos);
    vec![
        format!("position: {pos}"),
        format!("evaluator: {}", HandCrafted.name()),
        format!("material: {:+}", parts.material),
        format!(
            "piece-square: {:+} middlegame, {:+} endgame",
            parts.piece_square.mg, parts.piece_square.eg
        ),
        format!("phase: {} of {MAX_PHASE}", parts.phase),
        format!("total for White: {:+}", parts.total),
        format!(
            "total for the side to move: {:+}",
            HandCrafted.evaluate(pos)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_has_seven_lines_with_no_line_break_in_them() {
        let lines = report(&Position::startpos());
        assert_eq!(lines.len(), 7);
        assert!(lines.iter().all(|line| !line.contains('\n')));
        assert_eq!(
            lines[0],
            "position: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
        assert_eq!(lines[6], "total for the side to move: +0");
    }

    /// The last two lines differ only by the point of view.
    #[test]
    fn the_totals_are_for_white_and_for_the_mover() {
        let queen_up = "4k3/8/8/8/8/8/8/Q3K3 b - - 0 1";
        let lines = report(&Position::from_fen(queen_up).unwrap());
        let number = |line: &str| -> i32 { line.rsplit(' ').next().unwrap().parse().unwrap() };
        assert!(number(&lines[5]) > 800);
        assert_eq!(number(&lines[6]), -number(&lines[5]));
    }
}
