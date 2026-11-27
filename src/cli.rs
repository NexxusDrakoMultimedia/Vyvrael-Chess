// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Command-line front end of the engine binary.
//!
//! The logic lives here, not in `main.rs`, so tests can run it against
//! in-memory buffers.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::bench::{BENCH_DEPTH, bench};
use crate::board::Position;
use crate::eval::{self, HandCrafted};
use crate::perft::{divide, perft, perft_copy_make};
use crate::search::{Limits, MAX_PLY, Report, Search, TranspositionTable, stats_lines};
use crate::suite::{self, SuiteLimit};
use crate::uci::{HASH_DEFAULT, HASH_MAX, THREADS_MAX, score_text};
use crate::{ENGINE_AUTHOR, ENGINE_NAME, ENGINE_VERSION};

/// Usage text for the binary.
pub const USAGE: &str = "\
Usage:
  vyvrael                                  speak UCI on standard input and output
  vyvrael perft <depth> [--divide | --copy-make] [FEN]
                                           count leaf nodes from a position
  vyvrael eval [FEN]                       show the static evaluation
  vyvrael search <depth> [--stats] [FEN]   search to a fixed depth
  vyvrael bench [depth [threads [hash]]]   a fixed search workload: nodes and speed
  vyvrael suite <file> (--depth N | --nodes N | --movetime MS)
                                           run an EPD test suite
  vyvrael help                             show this text

perft counts the positions reachable in exactly <depth> moves from FEN (the
start position if omitted; the FEN may be given unquoted). With --divide the
count is split by first move, for comparing with another engine. With
--copy-make the count is made by copying the position for each move instead
of making and unmaking it, to compare the speed of the two designs. The two
flags cannot be combined.

eval prints the hand-crafted evaluation of FEN (the start position if
omitted) in centipawns, with its parts. It does not search.

search runs the search to <depth> plies from FEN (the start position if
omitted) and prints one line per completed depth, then the best move. Scores
are in centipawns for the side to move, or shown as mate in N moves. With
--stats it then prints the nodes each depth took, the effective branching
factor and how often the first move tried gave the cutoff.

bench searches a fixed set of positions to a fixed depth (4 unless <depth> is
given) at full strength and prints `<nodes> nodes <nps> nps`. The node count
is the same on every run of the same build; the speed is not. <threads> (1
only) and <hash> (in MiB, 1 to 65536) are accepted in the form other engines
use, `bench <depth> <threads> <hash>`, and have no effect yet.

suite searches every position of an EPD file, each to the depth, node count
or time in milliseconds given, and says for each whether the move found is
one of its best moves (bm) and none of its moves to avoid (am), then how many
were solved. It is a quick check of the search, not a measure of strength.
";

/// Exit status for success.
pub const EXIT_OK: u8 = 0;
/// Exit status for a command that was understood and could not be carried
/// out: a file that cannot be read.
pub const EXIT_FAILURE: u8 = 1;
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
        // The binary starts the UCI loop itself when it has no arguments
        // (see `main.rs`), so this is reached only by calling `run` directly.
        None => {
            writeln!(err, "{ENGINE_NAME} {ENGINE_VERSION} by {ENGINE_AUTHOR}")?;
            write!(err, "{USAGE}")?;
            Ok(EXIT_USAGE)
        }
        Some("help" | "--help" | "-h") => {
            write!(out, "{USAGE}")?;
            Ok(EXIT_OK)
        }
        Some("perft") => run_perft(&args[1..], out, err),
        Some("eval") => run_eval(&args[1..], out, err),
        Some("search") => run_search(&args[1..], out, err),
        Some("bench") => run_bench(&args[1..], out, err),
        Some("suite") => run_suite(&args[1..], out, err),
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
    for line in eval::report(&pos) {
        writeln!(out, "{line}")?;
    }
    Ok(EXIT_OK)
}

/// One line of search progress, in the vocabulary of UCI `info` lines
/// (without the time, so that the output is the same on every run).
fn progress_line(report: &Report) -> String {
    let mut line = format!(
        "depth {} score {} nodes {}",
        report.depth,
        score_text(report.score),
        report.nodes
    );
    if !report.pv.is_empty() {
        line.push_str(" pv");
        for mv in &report.pv {
            line.push_str(&format!(" {mv}"));
        }
    }
    line
}

fn run_search(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let with_stats = args.iter().any(|arg| arg == "--stats");
    let args: Vec<&String> = args.iter().filter(|arg| *arg != "--stats").collect();
    let depth = args.first().and_then(|text| text.parse::<u32>().ok());
    let Some(depth) = depth.filter(|depth| (1..=MAX_PLY as u32).contains(depth)) else {
        writeln!(err, "search: expected a depth from 1 to {MAX_PLY}")?;
        write!(err, "{USAGE}")?;
        return Ok(EXIT_USAGE);
    };
    let fields: Vec<&str> = args[1..].iter().map(|arg| arg.as_str()).collect();
    let Some(pos) = position_from(&fields, "search", err)? else {
        return Ok(EXIT_USAGE);
    };

    let start = Instant::now();
    let mut lines = Vec::new();
    let mut iterations = Vec::new();
    // As the engine searches the first move of a game: with an empty
    // table of its default size.
    let Some(mut table) = TranspositionTable::new(HASH_DEFAULT) else {
        writeln!(err, "search: no memory for the transposition table")?;
        return Ok(EXIT_FAILURE);
    };
    let mut searcher = Search::new(&HandCrafted)
        .with_limits(Limits::depth(depth))
        .with_table(&mut table);
    let outcome = searcher.iterate(&pos, &mut |report| {
        lines.push(progress_line(report));
        iterations.push((report.depth, report.nodes));
    });
    let elapsed = start.elapsed();
    for line in lines {
        writeln!(out, "{line}")?;
    }
    match outcome.best_move {
        Some(mv) => writeln!(out, "bestmove {mv}")?,
        None => writeln!(out, "bestmove (none)")?,
    }

    let nodes = searcher.nodes();
    if with_stats {
        for line in stats_lines(&iterations, nodes, &searcher.stats()) {
            writeln!(out, "{line}")?;
        }
    }
    let seconds = elapsed.as_secs_f64();
    let rate = if seconds > 0.0 {
        (nodes as f64 / seconds) as u64
    } else {
        0
    };
    writeln!(
        err,
        "time: {} ms, {nodes} nodes, {rate} nodes per second",
        elapsed.as_millis()
    )?;
    Ok(EXIT_OK)
}

fn run_bench(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    // Up to three numbers: the depth, then the threads and the hash size
    // in MiB. The last two are checked against the ranges of the UCI
    // options `Threads` and `Hash` and otherwise unused: there is one
    // search thread, and the bench's table has a fixed size for now.
    let ranges = [(1, MAX_PLY as u32), (1, THREADS_MAX), (1, HASH_MAX)];
    let in_range = |(text, (low, high)): (&String, &(u32, u32))| {
        text.parse::<u32>()
            .ok()
            .filter(|value| (*low..=*high).contains(value))
    };
    let numbers: Option<Vec<u32>> = if args.len() > ranges.len() {
        None
    } else {
        args.iter().zip(&ranges).map(in_range).collect()
    };
    let Some(numbers) = numbers else {
        writeln!(
            err,
            "bench: expected up to three numbers: a depth from 1 to {MAX_PLY}, threads (1 only) and a hash size from 1 to {HASH_MAX}"
        )?;
        write!(err, "{USAGE}")?;
        return Ok(EXIT_USAGE);
    };
    let depth = numbers.first().copied().unwrap_or(BENCH_DEPTH);
    let result = bench(depth);
    writeln!(
        out,
        "{} nodes {} nps",
        result.nodes,
        result.nodes_per_second()
    )?;
    Ok(EXIT_OK)
}

/// The file and the limit of a `suite` command, or `None` if the arguments
/// are not one file and exactly one limit with a whole number above 0
/// (0 is allowed for a time).
fn suite_arguments(args: &[String]) -> Option<(&str, SuiteLimit)> {
    let [first, second, third] = args else {
        return None;
    };
    // The file may stand before or after the limit.
    let (file, flag, value) = if first.starts_with("--") {
        (third, first, second)
    } else {
        (first, second, third)
    };
    let limit = match flag.as_str() {
        "--depth" => {
            let depth = value
                .parse()
                .ok()
                .filter(|depth| (1..=MAX_PLY as u32).contains(depth));
            SuiteLimit::Depth(depth?)
        }
        "--nodes" => SuiteLimit::Nodes(value.parse().ok().filter(|nodes| *nodes > 0)?),
        "--movetime" => SuiteLimit::MoveTime(Duration::from_millis(value.parse().ok()?)),
        _ => return None,
    };
    (!file.starts_with("--")).then_some((file.as_str(), limit))
}

fn run_suite(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let Some((file, limit)) = suite_arguments(args) else {
        writeln!(
            err,
            "suite: expected a file and one of --depth (1 to {MAX_PLY}), --nodes or --movetime \
             (milliseconds), with a number"
        )?;
        write!(err, "{USAGE}")?;
        return Ok(EXIT_USAGE);
    };
    let text = match std::fs::read(file) {
        // Suite files are old and not always UTF-8; a stray byte in a
        // comment should not cost the whole file.
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(problem) => {
            writeln!(err, "suite: cannot read {file}: {problem}")?;
            return Ok(EXIT_FAILURE);
        }
    };
    let start = Instant::now();
    let result = suite::run(&text, limit, out)?;
    writeln!(
        err,
        "time: {} ms, {} nodes",
        start.elapsed().as_millis(),
        result.nodes
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
    fn no_arguments_is_not_a_command() {
        // Without arguments the binary speaks UCI and never calls `run`.
        // Called directly, `run` has no command to carry out.
        let (status, out, err) = run_args(&[]);
        assert_eq!(status, EXIT_USAGE);
        assert!(out.is_empty(), "stdout is reserved for protocol output");
        assert!(err.contains(ENGINE_NAME));
        assert!(err.contains(ENGINE_VERSION));
        assert!(err.contains(USAGE));
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
    fn search_prints_a_line_per_depth_then_the_best_move() {
        let (status, out, err) = run_args(&["search", "3"]);
        assert_eq!(status, EXIT_OK);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4);
        for (line, depth) in lines[..3].iter().zip(1..) {
            assert!(
                line.starts_with(&format!("depth {depth} score cp ")),
                "{line}"
            );
            assert!(line.contains(" nodes ") && line.contains(" pv "), "{line}");
        }
        assert_eq!(lines[0], "depth 1 score cp 40 nodes 21 pv d2d4");
        assert!(lines[3].starts_with("bestmove "));
        // The best move is the first move of the last line's variation.
        let best = lines[3].trim_start_matches("bestmove ");
        assert!(lines[2].contains(&format!(" pv {best}")));
        assert!(err.contains("nodes per second"));
        // The same command gives the same output: nothing depends on time.
        assert_eq!(run_args(&["search", "3"]).1, out);
    }

    /// `--stats` adds the statistics after the best move and changes
    /// nothing before it; it may stand anywhere after the command.
    #[test]
    fn search_with_stats_adds_the_statistics() {
        let (_, plain, _) = run_args(&["search", "3"]);
        let (status, out, err) = run_args(&["search", "3", "--stats"]);
        assert_eq!(status, EXIT_OK);
        assert!(err.starts_with("time: "));
        let added = out.strip_prefix(plain.as_str()).unwrap();
        let lines: Vec<&str> = added.lines().collect();
        assert_eq!(lines.len(), 5, "{added}");
        assert_eq!(lines[0], "stats depth 1 nodes 21");
        assert_eq!(lines[1], "stats depth 2 nodes 113 ebf 5.38");
        assert_eq!(lines[2], "stats depth 3 nodes 1114 ebf 9.85");
        assert!(lines[3].starts_with("stats cutoffs "), "{added}");
        assert!(lines[4].starts_with("stats qnodes "), "{added}");
        assert!(lines[4].contains(" of 1248 ("), "{added}");
        // The same on every run, and wherever the flag stands.
        assert_eq!(run_args(&["search", "--stats", "3"]).1, out);
        let kiwipete = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let (_, first, _) = run_args(&["search", "2", "--stats", kiwipete]);
        let (_, last, _) = run_args(&["search", "2", kiwipete, "--stats"]);
        assert_eq!(first, last);
        assert!(first.contains("\nstats depth 2 nodes "));
    }

    #[test]
    fn search_reports_mates_in_moves() {
        let (_, out, _) = run_args(&["search", "5", "7k/8/8/8/8/8/R7/1R4K1 w - - 0 1"]);
        let lines: Vec<&str> = out.lines().collect();
        // Mate in two is proven at depth 3, and the search stops there.
        assert_eq!(lines.len(), 4);
        assert!(
            lines[2].starts_with("depth 3 score mate 2 nodes "),
            "{}",
            lines[2]
        );
        assert!(lines[3] == "bestmove b1b7" || lines[3] == "bestmove a2a7");

        let (_, lost, _) = run_args(&["search", "4", "7k/1R6/8/8/8/8/R7/6K1 b - - 0 1"]);
        assert!(lost.contains("score mate -1 "), "{lost}");
        assert!(lost.ends_with("bestmove h8g8\n"));
    }

    #[test]
    fn search_with_no_legal_move() {
        let (status, out, _) = run_args(&["search", "3", "R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1"]);
        assert_eq!(status, EXIT_OK);
        assert_eq!(out, "depth 1 score mate 0 nodes 1\nbestmove (none)\n");
        let (_, stale, _) = run_args(&["search", "3", "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1"]);
        assert_eq!(stale, "depth 1 score cp 0 nodes 1\nbestmove (none)\n");
    }

    #[test]
    fn search_rejects_bad_input() {
        for args in [
            &["search"][..],
            &["search", "0"],
            &["search", "deep"],
            &["search", "129"],
        ] {
            let (status, out, err) = run_args(args);
            assert_eq!(status, EXIT_USAGE, "{args:?}");
            assert!(out.is_empty());
            assert!(err.contains("expected a depth from 1 to 128"));
        }
        let (status, _, err) = run_args(&["search", "2", "not", "a", "fen"]);
        assert_eq!(status, EXIT_USAGE);
        assert!(err.contains("search: invalid FEN"));
    }

    /// A file for the `suite` tests, in the system's temporary directory
    /// under a name of this process, removed when dropped.
    struct SuiteFile(std::path::PathBuf);

    impl SuiteFile {
        fn new(name: &str, bytes: &[u8]) -> SuiteFile {
            let file = format!("vyvrael-{}-{name}.epd", std::process::id());
            let path = std::env::temp_dir().join(file);
            std::fs::write(&path, bytes).unwrap();
            SuiteFile(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for SuiteFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn suite_runs_a_file_and_reports() {
        // A byte that is not UTF-8 in a comment does not stop the file.
        let file = SuiteFile::new(
            "runs",
            b"# by \xe9douard\n6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+; id \"back rank\";\n\
              4k3/8/8/3q4/8/8/8/3RK3 w - - bm Rd2;\nnot a record\n",
        );
        let (status, out, err) = run_args(&["suite", file.path(), "--depth", "2"]);
        assert_eq!(status, EXIT_OK);
        assert_eq!(
            out,
            "2 back rank: solved, played a1a8 (bm Ra8+)\n\
             3: failed, played d1d5 (bm Rd2)\n\
             4: skipped, fewer than the four fields of a position\n\
             solved 1 of 2 positions (1 line skipped)\n"
        );
        assert!(
            err.starts_with("time: ") && err.trim_end().ends_with(" nodes"),
            "{err}"
        );
        // The limit may come first, and the other limits work.
        assert_eq!(run_args(&["suite", "--depth", "2", file.path()]).1, out);
        for limit in [["--nodes", "200"], ["--movetime", "0"]] {
            let (status, out, _) = run_args(&["suite", file.path(), limit[0], limit[1]]);
            assert_eq!(status, EXIT_OK, "{limit:?}");
            assert!(
                out.ends_with("positions (1 line skipped)\n"),
                "{limit:?}: {out}"
            );
        }
    }

    #[test]
    fn suite_rejects_bad_input() {
        let file = SuiteFile::new("rejects", b"");
        let path = file.path();
        for args in [
            &["suite"][..],
            &["suite", path],
            &["suite", path, "--depth"],
            &["suite", path, "--depth", "0"],
            &["suite", path, "--depth", "129"],
            &["suite", path, "--depth", "deep"],
            &["suite", path, "--nodes", "0"],
            &["suite", path, "--movetime", "-5"],
            &["suite", path, "--time", "5"],
            &["suite", "--depth", "2", "--nodes"],
            &["suite", path, "--depth", "2", "--nodes", "5"],
        ] {
            let (status, out, err) = run_args(args);
            assert_eq!(status, EXIT_USAGE, "{args:?}");
            assert!(out.is_empty());
            assert!(
                err.starts_with("suite: expected a file and one of --depth"),
                "{err}"
            );
        }
        // A file that is not there is not a usage error.
        let (status, out, err) = run_args(&["suite", "no/such/file.epd", "--depth", "1"]);
        assert_eq!(status, EXIT_FAILURE);
        assert!(out.is_empty());
        assert!(
            err.starts_with("suite: cannot read no/such/file.epd: "),
            "{err}"
        );
        assert!(!err.contains("Usage:"));
    }

    /// The bench prints one line, `<nodes> nodes <nps> nps`, and the same
    /// node count every time. A shallow depth keeps the test quick.
    #[test]
    fn bench_prints_nodes_and_speed_the_same_every_run() {
        let (status, first, err) = run_args(&["bench", "2"]);
        assert_eq!(status, EXIT_OK);
        assert!(err.is_empty());
        let words: Vec<&str> = first.split_whitespace().collect();
        assert_eq!(words.len(), 4, "{first}");
        assert_eq!((words[1], words[3]), ("nodes", "nps"));
        assert!(words[0].parse::<u64>().unwrap() > 0);
        assert!(words[2].parse::<u64>().is_ok());
        assert!(first.ends_with(" nps\n"));
        let (_, second, _) = run_args(&["bench", "2"]);
        assert_eq!(second.split_whitespace().next(), Some(words[0]));
    }

    /// `bench <depth> <threads> <hash>`, the form other engines use: the
    /// threads and the hash size are accepted and change nothing.
    #[test]
    fn bench_accepts_threads_and_a_hash_size() {
        let nodes = |args: &[&str]| {
            let (status, out, err) = run_args(args);
            assert_eq!(status, EXIT_OK, "{args:?}");
            assert!(err.is_empty(), "{args:?}");
            assert_eq!(out.split_whitespace().count(), 4, "{args:?}");
            out.split_whitespace().next().unwrap().to_owned()
        };
        let plain = nodes(&["bench", "2"]);
        for args in [
            &["bench", "2", "1"][..],
            &["bench", "2", "1", "1"],
            &["bench", "2", "1", "16"],
            &["bench", "2", "1", "65536"],
        ] {
            assert_eq!(nodes(args), plain, "{args:?}");
        }
    }

    #[test]
    fn bench_rejects_bad_input() {
        for args in [
            &["bench", "0"][..],
            &["bench", "deep"],
            &["bench", "129"],
            // More than the one thread there is, or none.
            &["bench", "2", "3"],
            &["bench", "2", "0"],
            &["bench", "2", "many"],
            // A hash size outside the range of the option `Hash`.
            &["bench", "2", "1", "0"],
            &["bench", "2", "1", "65537"],
            &["bench", "2", "1", "big"],
            // A bad depth is not excused by what follows it.
            &["bench", "0", "1", "16"],
            // A fourth number.
            &["bench", "2", "1", "16", "4"],
        ] {
            let (status, out, err) = run_args(args);
            assert_eq!(status, EXIT_USAGE, "{args:?}");
            assert!(out.is_empty());
            assert!(err.contains(
                "bench: expected up to three numbers: a depth from 1 to 128, threads (1 only) and a hash size from 1 to 65536"
            ));
        }
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
