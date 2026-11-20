// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The engine as the protocol sees it: a position, a search that may be
//! running, and the loop that reads commands and answers them.
//!
//! Two threads take part. The one that calls [`Engine::handle`] reads the
//! GUI's commands; `go` starts the search on a second thread, so that
//! `stop`, `isready` and `quit` are answered while the engine thinks. They
//! share the output (one line at a time, flushed) and a flag that ends the
//! search.

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::{Command, GoParams, bestmove_line, info_line, limits_for, parse};
use crate::attacks::rook_attacks;
use crate::board::{Bitboard, Position, Square};
use crate::eval::default_evaluator;
use crate::movegen::find_legal_move;
use crate::search::{Limits, Search};
use crate::{ENGINE_AUTHOR, ENGINE_NAME, ENGINE_VERSION};

/// Where protocol output goes, shared by the command thread and the search
/// thread.
type Output = Arc<Mutex<Box<dyn Write + Send>>>;

/// Stack for the search thread. The search recurses once per ply and keeps
/// a move list in every frame; this leaves a wide margin at the deepest
/// line it can reach.
const SEARCH_STACK_BYTES: usize = 16 * 1024 * 1024;

/// Writes one line and flushes it, as the protocol requires.
///
/// A write that fails means the GUI has gone away. There is nobody left to
/// tell, so the failure is ignored; the loop ends when its input closes.
fn send(out: &Output, line: &str) {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// A UCI engine: feed it lines with [`Engine::handle`].
pub struct Engine {
    out: Output,
    /// The position the next `go` searches.
    position: Position,
    /// Hashes of the positions the game went through before `position`,
    /// oldest first, for seeing repetitions.
    history: Vec<u64>,
    /// Raised to end the running search.
    stop: Arc<AtomicBool>,
    /// The search thread, if a `go` has been given. It may have finished.
    worker: Option<JoinHandle<()>>,
    /// Whether the running search reports only when told to stop.
    until_stopped: bool,
}

impl Engine {
    /// Creates an engine at the start position that writes protocol output
    /// to `out`.
    #[must_use]
    pub fn new(out: Box<dyn Write + Send>) -> Engine {
        Engine {
            out: Arc::new(Mutex::new(out)),
            position: Position::startpos(),
            history: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            until_stopped: false,
        }
    }

    /// Acts on one line of input. Returns `false` when the line was `quit`
    /// and the engine should end.
    pub fn handle(&mut self, line: &str) -> bool {
        match parse(line) {
            Command::Uci => {
                self.say(&format!("id name {ENGINE_NAME} {ENGINE_VERSION}"));
                self.say(&format!("id author {ENGINE_AUTHOR}"));
                self.say("uciok");
            }
            Command::IsReady => {
                // Build the slider tables now, so that the first search does
                // not pay for them on the clock.
                let _ = rook_attacks(Square::A1, Bitboard::EMPTY);
                self.say("readyok");
            }
            Command::NewGame => {
                self.end_search();
                self.position = Position::startpos();
                self.history.clear();
            }
            Command::Position { fen, moves } => self.set_position(fen.as_deref(), &moves),
            Command::Go(params) => self.go(&params),
            Command::Stop => self.end_search(),
            Command::SetOption { name, .. } => {
                self.say(&format!("info string unknown option: {name}"));
            }
            Command::Quit => {
                self.end_search();
                return false;
            }
            Command::Malformed(problem) => self.say(&format!("info string {problem}")),
            Command::Unknown(text) => self.say(&format!("info string unknown command: {text}")),
            Command::Empty => {}
        }
        true
    }

    /// Waits for the running search to finish by itself and report. A
    /// search that only reports when stopped (`go infinite`) is stopped.
    /// Used when the input ends without `quit`, so that a script piping
    /// commands in still gets its `bestmove`.
    pub fn wait(&mut self) {
        if self.until_stopped {
            self.stop.store(true, Ordering::Relaxed);
        }
        self.join();
    }

    fn say(&self, line: &str) {
        send(&self.out, line);
    }

    /// Stops the running search, if any, and waits until it has reported
    /// its best move.
    fn end_search(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.join();
    }

    fn join(&mut self) {
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            self.say("info string the search ended unexpectedly");
        }
        self.until_stopped = false;
    }

    /// `position`: sets up the position and plays the moves, each of which
    /// must be legal. If anything is wrong the command is refused as a
    /// whole and the position stays as it was.
    fn set_position(&mut self, fen: Option<&str>, moves: &[String]) {
        self.end_search();
        let start = match fen {
            None => Ok(Position::startpos()),
            Some(fen) => Position::from_fen(fen),
        };
        let mut position = match start {
            Ok(position) => position,
            Err(problem) => {
                self.say(&format!(
                    "info string position: invalid FEN ({problem}); position unchanged"
                ));
                return;
            }
        };
        let mut history = Vec::with_capacity(moves.len());
        for (index, text) in moves.iter().enumerate() {
            let Some(mv) = find_legal_move(&position, text) else {
                self.say(&format!(
                    "info string position: illegal move {text} (number {} in the list); \
                     position unchanged",
                    index + 1
                ));
                return;
            };
            history.push(position.hash());
            let _undo = position.make_move(mv);
        }
        self.position = position;
        self.history = history;
    }

    /// `go`: starts a search of the current position on its own thread. A
    /// search still running is ended first.
    fn go(&mut self, params: &GoParams) {
        self.end_search();
        self.stop.store(false, Ordering::Relaxed);
        let limits = limits_for(params, self.position.side_to_move(), Instant::now());
        // With nothing to end it, a search is `infinite` in all but name.
        let until_stopped = params.infinite || limits == Limits::NONE;

        let out = Arc::clone(&self.out);
        let stop = Arc::clone(&self.stop);
        let position = self.position.clone();
        let history = self.history.clone();
        let search = move || {
            let evaluator = default_evaluator();
            let mut search = Search::new(evaluator.as_ref())
                .with_history(&history)
                .with_limits(limits)
                .with_stop_flag(&stop);
            let outcome = search.iterate(&position, &mut |report| send(&out, &info_line(report)));
            // The protocol forbids answering an infinite search before
            // `stop`, even when there is nothing left to search.
            while until_stopped && !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(1));
            }
            send(&out, &bestmove_line(outcome.best_move));
        };
        let spawned = thread::Builder::new()
            .name("search".to_string())
            .stack_size(SEARCH_STACK_BYTES)
            .spawn(search);
        match spawned {
            Ok(worker) => {
                self.worker = Some(worker);
                self.until_stopped = until_stopped;
            }
            Err(problem) => {
                self.say(&format!(
                    "info string cannot start the search thread: {problem}"
                ));
                self.say(&bestmove_line(None));
            }
        }
    }
}

/// Runs the protocol loop: reads commands from `input` until `quit` or the
/// end of the input, writing protocol output to `out`.
///
/// # Errors
///
/// Returns an error if reading `input` fails.
pub fn run(input: impl BufRead, out: Box<dyn Write + Send>) -> io::Result<()> {
    let mut engine = Engine::new(out);
    for line in input.lines() {
        if !engine.handle(&line?) {
            return Ok(());
        }
    }
    engine.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::HandCrafted;
    use crate::search::Outcome;

    /// An output the test can read back.
    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Captured {
        fn lines(&self) -> Vec<String> {
            let bytes = self.0.lock().unwrap().clone();
            String::from_utf8(bytes)
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    fn engine() -> (Engine, Captured) {
        let captured = Captured::default();
        (Engine::new(Box::new(captured.clone())), captured)
    }

    /// Feeds `lines` to a new engine, lets the search finish, and returns
    /// everything it wrote.
    fn session(lines: &[&str]) -> Vec<String> {
        let (mut engine, captured) = engine();
        for line in lines {
            assert!(engine.handle(line), "{line}");
        }
        engine.wait();
        captured.lines()
    }

    fn direct(fen: &str, depth: u32) -> (Outcome, u64) {
        let pos = Position::from_fen(fen).unwrap();
        let mut search = Search::new(&HandCrafted).with_limits(Limits::depth(depth));
        let outcome = search.iterate(&pos, &mut |_| {});
        (outcome, search.nodes())
    }

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    #[test]
    fn uci_identifies_the_engine() {
        let lines = session(&["uci"]);
        assert_eq!(lines[0], format!("id name Vyvræl {ENGINE_VERSION}"));
        assert_eq!(lines[1], "id author Nexxus Drako Multimedia");
        assert_eq!(lines.last().unwrap(), "uciok");
    }

    #[test]
    fn isready_is_answered() {
        assert_eq!(session(&["isready"]), ["readyok"]);
        assert_eq!(session(&["isready", "isready"]), ["readyok", "readyok"]);
    }

    #[test]
    fn go_depth_searches_the_start_position() {
        let lines = session(&["position startpos", "go depth 3"]);
        let (outcome, nodes) = direct(crate::board::START_FEN, 3);
        assert_eq!(lines.len(), 4, "{lines:?}");
        for (line, depth) in lines.iter().zip(1..=3) {
            assert!(
                line.starts_with(&format!("info depth {depth} score cp ")),
                "{line}"
            );
        }
        // Exactly the search a direct call makes: same nodes, same line.
        assert!(
            lines[2].contains(&format!(" nodes {nodes} ")),
            "{}",
            lines[2]
        );
        let pv: Vec<String> = outcome.pv.iter().map(ToString::to_string).collect();
        assert!(lines[2].ends_with(&format!(" pv {}", pv.join(" "))));
        assert_eq!(lines[3], bestmove_line(outcome.best_move));
    }

    #[test]
    fn a_new_engine_is_at_the_start_position() {
        assert_eq!(
            stable(&session(&["go depth 2"])),
            stable(&session(&["position startpos", "go depth 2"]))
        );
    }

    /// An `info` line without its speed and time, which vary from run to
    /// run.
    fn strip_time(line: &str) -> String {
        let words: Vec<&str> = line.split(' ').collect();
        let mut kept = Vec::new();
        let mut index = 0;
        while index < words.len() {
            if words[index] == "nps" || words[index] == "time" {
                index += 2;
            } else {
                kept.push(words[index]);
                index += 1;
            }
        }
        kept.join(" ")
    }

    fn stable(lines: &[String]) -> Vec<String> {
        lines.iter().map(|line| strip_time(line)).collect()
    }

    #[test]
    fn position_plays_the_moves_it_is_given() {
        // After 1. e4 e5 2. Nf3 the position is this one.
        let after = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let by_moves = session(&["position startpos moves e2e4 e7e5 g1f3", "go depth 2"]);
        let by_fen = session(&[&format!("position fen {after}"), "go depth 2"]);
        assert_eq!(stable(&by_moves), stable(&by_fen));
        assert_eq!(
            by_fen.last().unwrap(),
            &bestmove_line(direct(after, 2).0.best_move)
        );
    }

    #[test]
    fn position_from_a_fen_with_moves() {
        let lines = session(&[
            &format!("position fen {KIWIPETE} moves e2a6 b4c3"),
            "go depth 1",
        ]);
        let after = "r3k2r/p1ppqpb1/Bn2pnp1/3PN3/4P3/2p2Q1p/PPPB1PPP/R3K2R w KQkq - 0 2";
        assert_eq!(
            lines.last().unwrap(),
            &bestmove_line(direct(after, 1).0.best_move)
        );
    }

    #[test]
    fn an_illegal_move_refuses_the_whole_command() {
        // After 1. e4 it is Black's move: a rook jumping over its pawn, a
        // queen jumping over hers, a move for the wrong side, and text that
        // is not a move.
        for bad in ["a8a5", "d8d5", "d2d4", "banana", "e7e5q"] {
            let lines = session(&[
                &format!("position fen {KIWIPETE}"),
                &format!("position startpos moves e2e4 {bad} e7e5"),
                "go depth 1",
            ]);
            assert_eq!(
                lines[0],
                format!(
                    "info string position: illegal move {bad} (number 2 in the list); \
                     position unchanged"
                )
            );
            // Still Kiwipete, not the start position after 1. e4.
            assert_eq!(
                lines.last().unwrap(),
                &bestmove_line(direct(KIWIPETE, 1).0.best_move),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_bad_fen_is_reported_and_changes_nothing() {
        let lines = session(&[
            &format!("position fen {KIWIPETE}"),
            "position fen 8/8/8/8/8/8/8/8 w - - 0 1",
            "go depth 1",
        ]);
        assert!(
            lines[0].starts_with("info string position: invalid FEN (")
                && lines[0].ends_with("); position unchanged"),
            "{}",
            lines[0]
        );
        assert_eq!(
            lines.last().unwrap(),
            &bestmove_line(direct(KIWIPETE, 1).0.best_move)
        );
    }

    /// The moves of a `position` command are the game's history: a queen
    /// down, Black finds the repetition only because the engine remembers
    /// the positions the game went through.
    #[test]
    fn the_game_history_reaches_the_search() {
        let lines = session(&[
            "position fen 7k/8/8/8/8/2K5/8/1Q6 w - - 10 30 moves b1c1 h8g8 c1b1",
            "go depth 2",
        ]);
        assert!(
            lines[1].starts_with("info depth 2 score cp 0 "),
            "{lines:?}"
        );
        assert_eq!(lines.last().unwrap(), "bestmove g8h8");
        // The same position given as a FEN has no history: Black is lost.
        let lines = session(&[
            "position fen 6k1/8/8/8/8/2K5/8/1Q6 b - - 13 31",
            "go depth 2",
        ]);
        assert!(!lines[1].contains("score cp 0 "), "{lines:?}");
    }

    #[test]
    fn ucinewgame_returns_to_the_start_position() {
        let lines = session(&[
            &format!("position fen {KIWIPETE}"),
            "ucinewgame",
            "go depth 1",
        ]);
        assert_eq!(
            lines.last().unwrap(),
            &bestmove_line(direct(crate::board::START_FEN, 1).0.best_move)
        );
    }

    #[test]
    fn no_legal_move_gives_the_null_move() {
        let mated = session(&[
            "position fen R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1",
            "go depth 3",
        ]);
        assert!(
            mated[0].starts_with("info depth 1 score mate 0 "),
            "{mated:?}"
        );
        assert_eq!(mated.last().unwrap(), "bestmove 0000");
        let stalemate = session(&["position fen 7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", "go depth 3"]);
        assert!(stalemate[0].starts_with("info depth 1 score cp 0 "));
        assert_eq!(stalemate.last().unwrap(), "bestmove 0000");
    }

    #[test]
    fn mate_scores_are_reported_in_moves() {
        let lines = session(&["position fen 7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", "go depth 5"]);
        // Proven at depth 3, where the search stops by itself.
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!(lines[2].starts_with("info depth 3 score mate 2 "));
    }

    #[test]
    fn a_node_limit_ends_the_search() {
        let lines = session(&[&format!("position fen {KIWIPETE}"), "go nodes 2000"]);
        let best = lines.last().unwrap();
        assert!(best.starts_with("bestmove ") && best != "bestmove 0000");
        // Depth 1 always completes; the limit stops the search soon after.
        assert!(lines.len() >= 2 && lines.len() <= 3, "{lines:?}");
    }

    #[test]
    fn a_move_time_ends_the_search() {
        let started = Instant::now();
        let lines = session(&[&format!("position fen {KIWIPETE}"), "go movetime 100"]);
        assert!(lines.last().unwrap().starts_with("bestmove "));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// The number of `bestmove` lines written so far.
    fn best_moves(captured: &Captured) -> usize {
        captured
            .lines()
            .iter()
            .filter(|line| line.starts_with("bestmove"))
            .count()
    }

    #[test]
    fn an_infinite_search_answers_only_when_stopped() {
        // Mate in one: the search itself is over in an instant, yet the
        // answer must wait for `stop`.
        let (mut engine, captured) = engine();
        engine.handle("position fen 6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1");
        engine.handle("go infinite");
        thread::sleep(Duration::from_millis(100));
        assert_eq!(best_moves(&captured), 0, "{:?}", captured.lines());
        assert!(captured.lines()[0].starts_with("info depth 1 score mate 1 "));
        // It still answers other commands meanwhile.
        engine.handle("isready");
        assert_eq!(captured.lines().last().unwrap(), "readyok");
        engine.handle("stop");
        assert_eq!(captured.lines().last().unwrap(), "bestmove a1a8");
        assert_eq!(best_moves(&captured), 1);
    }

    #[test]
    fn a_bare_go_is_infinite() {
        let (mut engine, captured) = engine();
        engine.handle("position fen 6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1");
        engine.handle("go");
        thread::sleep(Duration::from_millis(100));
        assert_eq!(best_moves(&captured), 0);
        engine.handle("stop");
        assert_eq!(captured.lines().last().unwrap(), "bestmove a1a8");
    }

    #[test]
    fn stop_ends_a_long_search_promptly() {
        let (mut engine, captured) = engine();
        engine.handle(&format!("position fen {KIWIPETE}"));
        engine.handle("go infinite");
        thread::sleep(Duration::from_millis(50));
        let stopping = Instant::now();
        engine.handle("stop");
        assert!(stopping.elapsed() < Duration::from_secs(2));
        let last = captured.lines().last().unwrap().clone();
        assert!(last.starts_with("bestmove ") && last != "bestmove 0000");
        assert_eq!(best_moves(&captured), 1);
    }

    #[test]
    fn stop_without_a_search_says_nothing() {
        assert!(session(&["stop", "stop"]).is_empty());
        // And after a search has finished it does not report a second time.
        let lines = session(&["go depth 1", "stop"]);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("bestmove"))
                .count(),
            1
        );
    }

    #[test]
    fn a_second_go_ends_the_first() {
        let (mut engine, captured) = engine();
        engine.handle(&format!("position fen {KIWIPETE}"));
        engine.handle("go infinite");
        engine.handle("go depth 1");
        engine.wait();
        // One answer from each search, never two searches at once.
        assert_eq!(best_moves(&captured), 2);
    }

    #[test]
    fn quit_ends_the_engine_and_any_search() {
        let (mut engine, captured) = engine();
        assert!(engine.handle("isready"));
        engine.handle(&format!("position fen {KIWIPETE}"));
        engine.handle("go infinite");
        assert!(!engine.handle("quit"));
        assert_eq!(best_moves(&captured), 1, "the search was ended first");
    }

    #[test]
    fn what_is_not_understood_is_said_so() {
        assert_eq!(
            session(&["hello there"]),
            ["info string unknown command: hello there"]
        );
        assert_eq!(
            session(&["position e2e4"]),
            [
                "info string position: expected `startpos` or `fen <fen>`, then optionally `moves ...`"
            ]
        );
        assert_eq!(
            session(&["setoption name Nonsense value 3"]),
            ["info string unknown option: Nonsense"]
        );
        assert!(session(&["", "   "]).is_empty());
    }

    #[test]
    fn every_line_of_output_is_protocol() {
        let lines = session(&[
            "uci",
            "isready",
            "nonsense",
            "position startpos moves e2e4",
            "go depth 2",
        ]);
        for line in &lines {
            let first = line.split(' ').next().unwrap();
            assert!(
                ["id", "uciok", "readyok", "info", "bestmove", "option"].contains(&first),
                "{line}"
            );
        }
    }

    #[test]
    fn the_loop_runs_until_quit() {
        let captured = Captured::default();
        let input = "uci\nisready\nposition startpos\ngo depth 2\nquit\nisready\n";
        run(input.as_bytes(), Box::new(captured.clone())).unwrap();
        let lines = captured.lines();
        assert!(lines.contains(&"uciok".to_string()));
        // `quit` stops the search, which still reports, and nothing after
        // `quit` is read.
        assert_eq!(lines.iter().filter(|line| *line == "readyok").count(), 1);
        assert!(lines.last().unwrap().starts_with("bestmove "));
    }

    #[test]
    fn at_the_end_of_input_the_search_is_allowed_to_finish() {
        let captured = Captured::default();
        run(
            "position startpos\ngo depth 3\n".as_bytes(),
            Box::new(captured.clone()),
        )
        .unwrap();
        let lines = captured.lines();
        // All three depths, not a search cut off at the first.
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert_eq!(
            lines[3],
            bestmove_line(direct(crate::board::START_FEN, 3).0.best_move)
        );

        // An infinite search cannot finish by itself, so it is stopped.
        let captured = Captured::default();
        run("go infinite\n".as_bytes(), Box::new(captured.clone())).unwrap();
        assert!(captured.lines().last().unwrap().starts_with("bestmove "));
    }
}
