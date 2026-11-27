// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Running a test suite: a file of EPD positions, each with the move to
//! find (`bm`) or to avoid (`am`), searched one after another.
//!
//! A suite is a quick look at whether the search still finds what it found
//! before a change, in seconds where a match takes hours. It measures
//! nothing about playing strength: positions are chosen, not sampled, and a
//! change that solves more of them can still lose games. Only an SPRT
//! decides whether a change is kept.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::board::Move;
use crate::epd::EpdRecord;
use crate::eval::default_evaluator;
use crate::movegen::find_san_move;
use crate::search::{Limits, Search, TranspositionTable};
use crate::uci::HASH_DEFAULT;

/// How long each position of a suite is searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteLimit {
    /// To this depth, in plies.
    Depth(u32),
    /// For about this many positions visited.
    Nodes(u64),
    /// For this long. Unlike the other two it does not give the same
    /// result on every run or machine.
    MoveTime(Duration),
}

/// How a suite went.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SuiteResult {
    /// Positions searched.
    pub positions: u32,
    /// Those where the engine's move was right.
    pub solved: u32,
    /// Lines that could not be used: not a record the engine accepts, no
    /// `bm` or `am`, or a move it could not read.
    pub skipped: u32,
    /// Positions visited, over all the searches.
    pub nodes: u64,
}

/// What became of one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The position was searched. `solved` is whether `played` is one of
    /// the best moves, if any are given, and none of the moves to avoid.
    Searched {
        /// The engine's move. `None` would mean a position without a
        /// legal move, which cannot come up: none of its `bm` or `am`
        /// moves could have been read.
        played: Option<Move>,
        /// Whether that is right.
        solved: bool,
        /// Positions the search visited.
        nodes: u64,
    },
    /// The line was not used; the text says why.
    Skipped(String),
}

/// Reads `line` as an EPD record and searches its position for `limit`, at
/// full strength, as a new search with the engine's own evaluation.
#[must_use]
pub fn solve(line: &str, limit: SuiteLimit) -> (Option<EpdRecord>, Verdict) {
    let record = match EpdRecord::parse(line) {
        Ok(record) => record,
        Err(problem) => return (None, Verdict::Skipped(problem.to_string())),
    };
    let mut wanted = [Vec::new(), Vec::new()];
    for (moves, opcode) in wanted.iter_mut().zip(["bm", "am"]) {
        for text in record.words(opcode) {
            let Some(mv) = find_san_move(&record.position, text) else {
                let reason = format!("{opcode} {text} is not one legal move of the position");
                return (Some(record), Verdict::Skipped(reason));
            };
            moves.push(mv);
        }
    }
    let [best, avoid] = wanted;
    if best.is_empty() && avoid.is_empty() {
        return (
            Some(record),
            Verdict::Skipped("no bm or am operation".to_owned()),
        );
    }
    let limits = match limit {
        SuiteLimit::Depth(depth) => Limits::depth(depth),
        SuiteLimit::Nodes(nodes) => Limits::nodes(nodes),
        SuiteLimit::MoveTime(time) => Limits::until(Instant::now() + time),
    };
    let evaluator = default_evaluator();
    // A new table for each position: one position's search must not
    // depend on the positions before it in the file.
    let mut table = TranspositionTable::new(HASH_DEFAULT);
    let mut search = Search::new(evaluator.as_ref()).with_limits(limits);
    if let Some(table) = table.as_mut() {
        search = search.with_table(table);
    }
    let played = search.iterate(&record.position, &mut |_| {}).best_move;
    let solved =
        played.is_some_and(|mv| (best.is_empty() || best.contains(&mv)) && !avoid.contains(&mv));
    let verdict = Verdict::Searched {
        played,
        solved,
        nodes: search.nodes(),
    };
    (Some(record), verdict)
}

/// Runs the suite in `text`, one record to a line, writing a line per
/// record and a last line with the totals to `out`:
///
/// ```text
/// 1 back rank: solved, played a1a8 (bm Ra8+)
/// 2 quiet move: failed, played e2e4 (bm Nf3; am e4)
/// 4: skipped, no bm or am operation
/// solved 1 of 2 positions (1 line skipped)
/// ```
///
/// Each line starts with its number in the file and the record's `id`, if
/// it has one. Blank lines and lines starting with `#` are passed over
/// without a word. Nothing that depends on the clock is written, so with a
/// depth or a node limit the output is the same on every run.
///
/// # Errors
///
/// Returns an error only if writing to `out` fails.
pub fn run(text: &str, limit: SuiteLimit, out: &mut dyn Write) -> io::Result<SuiteResult> {
    let mut result = SuiteResult::default();
    // A byte-order mark, which some editors put at the start of a file, is
    // not white space and would spoil the first line.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (record, verdict) = solve(line, limit);
        let operand = |opcode| record.as_ref().and_then(|record| record.operand(opcode));
        let name = match operand("id") {
            Some(id) if !id.is_empty() => format!("{} {id}", index + 1),
            _ => (index + 1).to_string(),
        };
        match verdict {
            Verdict::Skipped(reason) => {
                result.skipped += 1;
                writeln!(out, "{name}: skipped, {reason}")?;
            }
            Verdict::Searched {
                played,
                solved,
                nodes,
            } => {
                result.positions += 1;
                result.solved += u32::from(solved);
                result.nodes += nodes;
                let asked: Vec<String> = ["bm", "am"]
                    .into_iter()
                    .filter_map(|opcode| Some(format!("{opcode} {}", operand(opcode)?)))
                    .collect();
                let played = played.map_or_else(|| "no move".to_owned(), |mv| mv.to_string());
                let word = if solved { "solved" } else { "failed" };
                writeln!(
                    out,
                    "{name}: {word}, played {played} ({})",
                    asked.join("; ")
                )?;
            }
        }
    }
    let lines = if result.skipped == 1 { "line" } else { "lines" };
    writeln!(
        out,
        "solved {} of {} positions ({} {lines} skipped)",
        result.solved, result.positions, result.skipped
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small suite written for this test: two mates in one, a hanging
    /// queen, a move to avoid, a best move the engine does not play at this
    /// depth, and lines that cannot be used.
    const SUITE: &str = "\
# A comment, then a blank line.

6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+; id \"back rank\";
7k/5Q2/6K1/8/8/8/8/8 w - - bm Qf8# Qg7#; id \"two mates\";
4k3/8/8/3q4/8/8/8/3RK3 w - - bm Rxd5; id \"hanging queen\";
4k3/8/8/3q4/8/8/8/3RK3 w - - am Rd2 Rd3; id \"not a quiet rook move\";
4k3/8/8/3q4/8/8/8/3RK3 w - - bm Rd2; id \"wrong on purpose\";
4k3/8/8/3q4/8/8/8/3RK3 w - - id \"nothing asked\";
4k3/8/8/3q4/8/8/8/3RK3 w - - bm Qd4;
8/8/8/8/8/8/8/8 w - - bm e4;
7k/5Q2/6K1/8/8/8/8/8 b - - am Kg8; id \"stalemate\";
";

    fn output(limit: SuiteLimit) -> (SuiteResult, String) {
        let mut out = Vec::new();
        let result = run(SUITE, limit, &mut out).unwrap();
        (result, String::from_utf8(out).unwrap())
    }

    #[test]
    fn a_suite_line_by_line() {
        let (result, text) = output(SuiteLimit::Depth(2));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[..4],
            [
                "3 back rank: solved, played a1a8 (bm Ra8+)",
                "4 two mates: solved, played f7g7 (bm Qf8# Qg7#)",
                "5 hanging queen: solved, played d1d5 (bm Rxd5)",
                "6 not a quiet rook move: solved, played d1d5 (am Rd2 Rd3)",
            ]
        );
        assert_eq!(lines[4], "7 wrong on purpose: failed, played d1d5 (bm Rd2)");
        assert_eq!(lines[5], "8 nothing asked: skipped, no bm or am operation");
        assert_eq!(
            lines[6],
            "9: skipped, bm Qd4 is not one legal move of the position"
        );
        assert!(lines[7].starts_with("10: skipped, "), "{}", lines[7]);
        // A position without a legal move cannot be asked about: no move
        // of its `bm` or `am` can be read, so it is skipped like the rest.
        assert_eq!(
            lines[8],
            "11 stalemate: skipped, am Kg8 is not one legal move of the position"
        );
        assert_eq!(lines[9], "solved 4 of 5 positions (4 lines skipped)");
        assert_eq!(lines.len(), 10);
        assert_eq!((result.positions, result.solved, result.skipped), (5, 4, 4));
        assert!(result.nodes > 0);
    }

    /// A depth or a node limit gives the same output every time; so does a
    /// node limit as a whole, and the limits reach the search.
    #[test]
    fn the_same_every_run_and_the_limit_is_used() {
        let (first, text) = output(SuiteLimit::Depth(2));
        assert_eq!(output(SuiteLimit::Depth(2)), (first, text));
        // The smallest node limit there is: each search ends as soon as
        // its first depth is done.
        let (few, _) = output(SuiteLimit::Nodes(1));
        assert_eq!(output(SuiteLimit::Nodes(1)).0, few);
        let (deeper, _) = output(SuiteLimit::Depth(3));
        assert!(few.nodes < first.nodes && first.nodes < deeper.nodes);
        // A move time is a limit too: the first depth alone finds these.
        let (timed, _) = output(SuiteLimit::MoveTime(Duration::ZERO));
        assert_eq!((timed.positions, timed.skipped), (5, 4));
    }

    #[test]
    fn one_line_alone() {
        let (record, verdict) = solve(
            "6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+;",
            SuiteLimit::Depth(1),
        );
        assert_eq!(record.unwrap().words("bm"), ["Ra8+"]);
        assert!(
            matches!(verdict, Verdict::Searched { solved: true, .. }),
            "{verdict:?}"
        );
        let (record, verdict) = solve("not a position", SuiteLimit::Depth(1));
        assert_eq!(record, None);
        assert!(matches!(verdict, Verdict::Skipped(_)));
        // A byte-order mark in front of the file does not cost its first
        // line, whether that is a record or a comment.
        for first in ["6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+;", "# a comment"] {
            let mut out = Vec::new();
            let text = format!("\u{feff}{first}\n");
            let result = run(&text, SuiteLimit::Depth(1), &mut out).unwrap();
            assert_eq!(result.skipped, 0, "{first}");
            assert_eq!(result.positions, u32::from(!first.starts_with('#')));
        }
        // An empty suite is zero of zero, with the plural.
        let mut out = Vec::new();
        let none = run("\n# only this\n", SuiteLimit::Depth(1), &mut out).unwrap();
        assert_eq!(none, SuiteResult::default());
        assert_eq!(out, b"solved 0 of 0 positions (0 lines skipped)\n");
    }
}
