// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Command-line front end of the engine binary.
//!
//! The logic lives here, not in `main.rs`, so tests can run it against
//! in-memory buffers.

use std::io::{self, Write};
use std::time::Instant;

use crate::board::Position;
use crate::eval::{Evaluator, HandCrafted, MAX_PHASE};
use crate::perft::{divide, perft, perft_copy_make};
use crate::{ENGINE_AUTHOR, ENGINE_NAME, ENGINE_VERSION};

/// Usage text for the binary.
pub const USAGE: &str = "\
Usage:
  vyvrael                                  identify the engine (UCI arrives later)
  vyvrael perft <depth> [--divide | --copy-make] [FEN]
                                           count leaf nodes from a position
  vyvrael eval [FEN]                       show the static evaluation
  vyvrael help                             show this text

perft counts the positions reachable in exactly <depth> moves from FEN (the
start position if omitted; the FEN may be given unquoted). With --divide the
count is split by first move, for comparing with another engine. With
--copy-make the count is made by copying the position for each move instead
of making and unmaking it, to compare the speed of the two designs. The two
flags cannot be combined.

eval prints the hand-crafted evaluation of FEN (the start position if
omitted) in centipawns, with its parts. It does not search.
";

/// Exit status for success.
pub const EXIT_OK: u8 = 0;
/// Exit status for a command line that could not be understood.
pub const EXIT_USAGE: u8 = 2;

/// Runs the command line `args` (without the program name), writing results
/// to `out` and diagnostics to `err`, and returns the process exit status.
///
/// # Errors
///
/// Returns an error only if writing to `out` or `err` fails.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    match args.first().map(String::as_str) {
        None => {
            writeln!(err, "{ENGINE_NAME} {ENGINE_VERSION} by {ENGINE_AUTHOR}")?;
            writeln!(
                err,
                "The UCI interface is not implemented yet; try `vyvrael help`."
            )?;
            Ok(EXIT_OK)
        }
        Some("help" | "--help" | "-h") => {
            write!(out, "{USAGE}")?;
            Ok(EXIT_OK)
        }
        Some("perft") => run_perft(&args[1..], out, err),
        Some("eval") => run_eval(&args[1..], out, err),
        Some(other) => {
            writeln!(err, "unknown command: {other}")?;
            write!(err, "{USAGE}")?;
            Ok(EXIT_USAGE)
        }
    }
}

/// Parses the FEN made of `fields` joined by spaces, or returns the start
/// position when there are none. On a bad FEN, reports it on `err` under
/// `command`'s name and returns `None`.
fn position_from(
    fields: &[&str],
    command: &str,
    err: &mut dyn Write,
) -> io::Result<Option<Position>> {
    if fields.is_empty() {
        return Ok(Some(Position::startpos()));
    }
    match Position::from_fen(&fields.join(" ")) {
        Ok(pos) => Ok(Some(pos)),
        Err(problem) => {
            writeln!(err, "{command}: invalid FEN: {problem}")?;
            Ok(None)
        }
    }
}

fn run_eval(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let fields: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(pos) = position_from(&fields, "eval", err)? else {
        return Ok(EXIT_USAGE);
    };
    let parts = HandCrafted::breakdown(&pos);
    writeln!(out, "position: {pos}")?;
    writeln!(out, "evaluator: {}", HandCrafted.name())?;
    writeln!(out, "material: {:+}", parts.material)?;
    writeln!(
        out,
        "piece-square: {:+} middlegame, {:+} endgame",
        parts.piece_square.mg, parts.piece_square.eg
    )?;
    writeln!(out, "phase: {} of {MAX_PHASE}", parts.phase)?;
    writeln!(out, "total for White: {:+}", parts.total)?;
    writeln!(
        out,
        "total for the side to move: {:+}",
        HandCrafted.evaluate(&pos)
    )?;
    Ok(EXIT_OK)
}

fn run_perft(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let split = args.iter().any(|arg| arg == "--divide");
    let copy_make = args.iter().any(|arg| arg == "--copy-make");
    if split && copy_make {
        writeln!(err, "perft: --divide and --copy-make cannot be combined")?;
        return Ok(EXIT_USAGE);
    }
    let mut rest = args
        .iter()
        .filter(|arg| *arg != "--divide" && *arg != "--copy-make");

    let Some(depth) = rest.next().and_then(|text| text.parse::<u32>().ok()) else {
        writeln!(err, "perft: expected a depth (a non-negative whole number)")?;
        write!(err, "{USAGE}")?;
        return Ok(EXIT_USAGE);
    };
    let fen: Vec<&str> = rest.map(String::as_str).collect();
    let Some(mut pos) = position_from(&fen, "perft", err)? else {
        return Ok(EXIT_USAGE);
    };

    let start = Instant::now();
    let nodes = if split {
        let mut counts = divide(&mut pos, depth);
        counts.sort_by_key(|(mv, _)| mv.to_string());
        for (mv, count) in &counts {
            writeln!(out, "{mv}: {count}")?;
        }
        writeln!(out)?;
        // At depth 0 `divide` lists nothing, but the count is still 1.
        if depth == 0 {
            1
        } else {
            counts.iter().map(|&(_, count)| count).sum()
        }
    } else if copy_make {
        perft_copy_make(&pos, depth)
    } else {
        perft(&mut pos, depth)
    };
    let elapsed = start.elapsed();
    writeln!(out, "nodes: {nodes}")?;

    let seconds = elapsed.as_secs_f64();
    let rate = if seconds > 0.0 {
        (nodes as f64 / seconds) as u64
    } else {
        0
    };
    writeln!(
        err,
        "time: {} ms, {rate} nodes per second",
        elapsed.as_millis()
    )?;
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the command line and returns (exit status, stdout, stderr).
    fn run_args(args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|&arg| arg.to_owned()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = run(&args, &mut out, &mut err).unwrap();
        (
            status,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn no_arguments_identifies_the_engine_on_stderr_only() {
        let (status, out, err) = run_args(&[]);
        assert_eq!(status, EXIT_OK);
        assert!(out.is_empty(), "stdout is reserved for protocol output");
        assert!(err.contains(ENGINE_NAME));
        assert!(err.contains(ENGINE_VERSION));
    }

    #[test]
    fn help_prints_usage_on_stdout() {
        for flag in ["help", "--help", "-h"] {
            let (status, out, err) = run_args(&[flag]);
            assert_eq!(status, EXIT_OK);
            assert_eq!(out, USAGE);
            assert!(err.is_empty());
        }
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        let (status, out, err) = run_args(&["frobnicate"]);
        assert_eq!(status, EXIT_USAGE);
        assert!(out.is_empty());
        assert!(err.contains("unknown command: frobnicate"));
    }

    #[test]
    fn perft_from_the_start_position() {
        let (status, out, err) = run_args(&["perft", "3"]);
        assert_eq!(status, EXIT_OK);
        assert_eq!(out, "nodes: 8902\n");
        assert!(err.contains("nodes per second"));
        assert_eq!(run_args(&["perft", "0"]).1, "nodes: 1\n");
    }

    #[test]
    fn perft_accepts_a_fen_quoted_or_unquoted() {
        let kiwipete = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let quoted = run_args(&["perft", "2", kiwipete]);
        assert_eq!(quoted.1, "nodes: 2039\n");
        let mut unquoted = vec!["perft", "2"];
        unquoted.extend(kiwipete.split(' '));
        assert_eq!(run_args(&unquoted).1, "nodes: 2039\n");
    }

    #[test]
    fn perft_divide_lists_sorted_moves_then_the_total() {
        let (status, out, _) = run_args(&["perft", "2", "--divide"]);
        assert_eq!(status, EXIT_OK);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 22, "20 moves, a blank line, the total");
        assert_eq!(lines[0], "a2a3: 20");
        assert_eq!(lines[19], "h2h4: 20");
        assert_eq!(lines[20], "");
        assert_eq!(lines[21], "nodes: 400");
        let sorted = {
            let mut moves = lines[..20].to_vec();
            moves.sort_unstable();
            moves
        };
        assert_eq!(sorted, lines[..20]);
        // The flag may come before the depth, and depth 0 still counts 1.
        assert_eq!(run_args(&["perft", "--divide", "2"]).1, out);
        assert_eq!(run_args(&["perft", "0", "--divide"]).1, "\nnodes: 1\n");
    }

    #[test]
    fn perft_copy_make_gives_the_same_count() {
        let (status, out, _) = run_args(&["perft", "3", "--copy-make"]);
        assert_eq!(status, EXIT_OK);
        assert_eq!(out, "nodes: 8902\n");
        let (status, out, err) = run_args(&["perft", "2", "--copy-make", "--divide"]);
        assert_eq!(status, EXIT_USAGE);
        assert!(out.is_empty());
        assert!(err.contains("cannot be combined"));
    }

    #[test]
    fn eval_prints_a_breakdown() {
        let (status, out, err) = run_args(&["eval"]);
        assert_eq!(status, EXIT_OK);
        assert!(err.is_empty());
        let expected = "\
position: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
evaluator: hand-crafted
material: +0
piece-square: +0 middlegame, +0 endgame
phase: 24 of 24
total for White: +0
total for the side to move: +0
";
        assert_eq!(out, expected);
    }

    #[test]
    fn eval_reports_both_points_of_view() {
        let after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let (status, out, _) = run_args(&["eval", after_e4]);
        assert_eq!(status, EXIT_OK);
        assert!(out.contains("piece-square: +40 middlegame, +12 endgame\n"));
        assert!(out.contains("total for White: +40\n"));
        assert!(out.contains("total for the side to move: -40\n"));
        // The FEN may be given unquoted.
        let mut unquoted = vec!["eval"];
        unquoted.extend(after_e4.split(' '));
        assert_eq!(run_args(&unquoted).1, out);
        // A queen up shows in the material line.
        let (_, out, _) = run_args(&["eval", "4k3/8/8/8/8/8/8/Q3K3 w - - 0 1"]);
        assert!(out.contains("material: +900\n"));
        assert!(out.contains("phase: 4 of 24\n"));
    }

    #[test]
    fn eval_rejects_a_bad_fen() {
        let (status, out, err) = run_args(&["eval", "not", "a", "fen"]);
        assert_eq!(status, EXIT_USAGE);
        assert!(out.is_empty());
        assert!(err.contains("eval: invalid FEN"));
    }

    #[test]
    fn perft_rejects_bad_input() {
        for args in [&["perft"][..], &["perft", "deep"], &["perft", "-1"]] {
            let (status, out, err) = run_args(args);
            assert_eq!(status, EXIT_USAGE, "{args:?}");
            assert!(out.is_empty());
            assert!(err.contains("expected a depth"));
        }
        let (status, out, err) = run_args(&["perft", "1", "not", "a", "fen"]);
        assert_eq!(status, EXIT_USAGE);
        assert!(out.is_empty());
        assert!(err.contains("invalid FEN"));
    }
}
