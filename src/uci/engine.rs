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

use super::{
    Command, GoParams, HASH_DEFAULT, Options, bestmove_line, info_line, limits_for, parse,
};
use crate::attacks::rook_attacks;
use crate::board::{Bitboard, Position, Square};
use crate::eval::{self, default_evaluator};
use crate::movegen::find_legal_move;
use crate::rng::{Rng, entropy_seed, stream};
use crate::search::{
    Limits, Outcome, Report, Search, TranspositionTable, Weakening, split_limits, stats_lines,
    varied_move, variety_applies, weakened_move,
};
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

/// Sends `lines` as `info string` lines under one lock, so that they
/// arrive together whatever the other thread is sending.
fn send_block(out: &Output, lines: &[String]) {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    for line in lines {
        let _ = writeln!(out, "info string {line}");
    }
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
    /// The values of the UCI options.
    options: Options,
    /// The transposition table, kept from one search of a game to the
    /// next. The search thread holds it for as long as a full-strength
    /// search runs.
    table: Arc<Mutex<TranspositionTable>>,
    /// The size the table was made with, in MiB: the option `Hash` as it
    /// was when a search last started.
    table_mb: u32,
    /// The seed of strength-affecting randomness: the option `Seed`, or
    /// one from system entropy while `Seed` is 0.
    seed: u32,
    /// The generator of strength-affecting randomness, started from `seed`.
    /// It stays on the command thread: each weakened search, and each varied
    /// choice when `Variety` applies, is given a generator of its own,
    /// seeded from this one.
    rng: Rng,
}

impl Engine {
    /// Creates an engine at the start position that writes protocol output
    /// to `out`.
    #[must_use]
    pub fn new(out: Box<dyn Write + Send>) -> Engine {
        // `Seed` is 0 by default, so the seed comes from system entropy.
        // This is the one permitted use of it.
        let seed = entropy_seed();
        Engine {
            out: Arc::new(Mutex::new(out)),
            position: Position::startpos(),
            history: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            until_stopped: false,
            options: Options::default(),
            // If even the default size cannot be had, a table of one slot
            // still lets the engine play.
            table: Arc::new(Mutex::new(
                TranspositionTable::new(HASH_DEFAULT)
                    .or_else(|| TranspositionTable::new(0))
                    .expect("memory for a table of one slot"),
            )),
            table_mb: HASH_DEFAULT,
            seed,
            rng: Rng::stream(seed, stream::STRENGTH),
        }
    }

    /// The seed strength-affecting randomness currently starts from.
    #[must_use]
    pub const fn rng_seed(&self) -> u32 {
        self.seed
    }

    /// Says which seed is in use, so that a run can be replayed: give the
    /// number as `Seed`.
    fn log_seed(&self) {
        let source = if self.options.seed == 0 {
            "system entropy"
        } else {
            "Seed"
        };
        self.say(&format!("info string rng seed {} ({source})", self.seed));
    }

    /// Chooses the seed again after `Seed` may have changed from
    /// `previous`, and logs it if it did.
    fn reseed(&mut self, previous: u32) {
        let seed = match (self.options.seed, previous) {
            // Still 0: keep the entropy seed this game already has, so that
            // setting another option does not reseed in the middle of it.
            (0, 0) => self.seed,
            // Just set back to 0: a fresh seed, not the one a user chose.
            (0, _) => entropy_seed(),
            // A seed was given: use it.
            (given, _) => given,
        };
        if seed != self.seed || (previous == 0) != (self.options.seed == 0) {
            self.seed = seed;
            self.rng = Rng::stream(seed, stream::STRENGTH);
            self.log_seed();
        }
    }

    /// The current values of the options.
    #[must_use]
    pub const fn options(&self) -> &Options {
        &self.options
    }

    /// Acts on one line of input. Returns `false` when the line was `quit`
    /// and the engine should end.
    pub fn handle(&mut self, line: &str) -> bool {
        match parse(line) {
            Command::Uci => {
                self.say(&format!("id name {ENGINE_NAME} {ENGINE_VERSION}"));
                self.say(&format!("id author {ENGINE_AUTHOR}"));
                for declaration in Options::declarations() {
                    self.say(&declaration);
                }
                self.log_seed();
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
                // What was learnt about the last game's positions says
                // nothing reliable about the next one's, and a game must be
                // repeatable from its own moves. No search is running: it
                // was ended above.
                self.table
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clear();
                // Every game starts its random sequence afresh, so that any
                // game can be replayed by itself from one logged number.
                // With a chosen seed that is the same seed again; without
                // one it is a new seed from system entropy. Either way it is
                // logged here, so a match tool's log names it for each game.
                if self.options.seed == 0 {
                    self.seed = entropy_seed();
                }
                self.log_seed();
                self.rng = Rng::stream(self.seed, stream::STRENGTH);
            }
            Command::Position { fen, moves } => self.set_position(fen.as_deref(), &moves),
            Command::Go(params) => self.go(&params),
            Command::Stop => self.end_search(),
            Command::SetOption { name, value } => {
                // An option applies from the next search on; a running one
                // keeps the values it started with.
                let previous = self.options.seed;
                match self.options.set(&name, value.as_deref()) {
                    Ok(()) => self.reseed(previous),
                    Err(problem) => self.say(&format!("info string {problem}")),
                }
            }
            Command::Display => self.say_lines(&self.display()),
            Command::Eval => self.say_lines(&eval::report(&self.position)),
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

    /// Sends `lines` as `info string` lines, as one block: a search that is
    /// running cannot put a line of its own in between.
    fn say_lines(&self, lines: &[String]) {
        send_block(&self.out, lines);
    }

    /// What `d` shows: the board, the FEN, the hash key and the pieces
    /// giving check. It reads the position and changes nothing.
    fn display(&self) -> Vec<String> {
        let pos = &self.position;
        let checkers: Vec<String> = pos
            .checkers()
            .into_iter()
            .map(|sq| sq.to_string())
            .collect();
        let mut lines = pos.diagram();
        lines.push(format!("fen: {pos}"));
        lines.push(format!("key: {:016x}", pos.hash()));
        lines.push(if checkers.is_empty() {
            "checkers: -".to_owned()
        } else {
            format!("checkers: {}", checkers.join(" "))
        });
        lines
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
    /// Makes the table the size `Hash` asks for, if it is not. Called with
    /// no search running. A new table is empty; if there is no memory for
    /// it the old one stays, and `Hash` is put back to its size.
    fn fit_table(&mut self) {
        let wanted = self.options.hash_mb;
        if wanted == self.table_mb {
            return;
        }
        match TranspositionTable::new(wanted) {
            Some(table) => {
                *self.table.lock().unwrap_or_else(PoisonError::into_inner) = table;
                self.table_mb = wanted;
            }
            None => {
                self.say(&format!(
                    "info string Hash: no memory for {wanted} MiB; it stays at {} MiB",
                    self.table_mb
                ));
                self.options.hash_mb = self.table_mb;
            }
        }
    }

    fn go(&mut self, params: &GoParams) {
        self.end_search();
        self.fit_table();
        let table = Arc::clone(&self.table);
        self.stop.store(false, Ordering::Relaxed);
        let limits = limits_for(
            params,
            self.position.side_to_move(),
            Instant::now(),
            self.options.move_overhead(),
        );
        // With nothing to end it, a search is `infinite` in all but name.
        let until_stopped = params.infinite || limits == Limits::NONE;

        let out = Arc::clone(&self.out);
        let stop = Arc::clone(&self.stop);
        let position = self.position.clone();
        let history = self.history.clone();
        // Full strength touches the generator only when `Variety` applies
        // (below). A weakened search gets its own generator, seeded by one
        // draw here on the command thread, so the sequence of games does not
        // depend on how far each search got.
        let weakened = self.options.limit_strength.then(|| {
            (
                Weakening::for_elo(self.options.elo),
                Rng::from_state(self.rng.next_u64()),
            )
        });
        // `Variety` (full strength only): the search gets part of the budget
        // and the checks of a varied move the rest, with a generator of
        // their own, drawn only when it applies. With `Variety` 0 nothing
        // here changes the search or touches the generator.
        let variety = self.options.variety;
        let show_wdl = self.options.show_wdl;
        let search_stats = self.options.search_stats;
        let varying = !self.options.limit_strength && variety_applies(&self.position, variety);
        let (search_limits, check_limits) = if varying {
            split_limits(limits, Instant::now())
        } else {
            (limits, Limits::NONE)
        };
        let variety_rng = varying.then(|| Rng::from_state(self.rng.next_u64()));
        let search = move || {
            let evaluator = default_evaluator();
            let outcome = match weakened {
                None => {
                    let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
                    let mut search = Search::new(evaluator.as_ref())
                        .with_history(&history)
                        .with_limits(search_limits)
                        .with_stop_flag(&stop)
                        .with_table(&mut table);
                    let started = Instant::now();
                    let mut reached = 0;
                    let mut reached_seldepth = 0;
                    let mut iterations = Vec::new();
                    let outcome = search.iterate(&position, &mut |report| {
                        reached = report.depth;
                        reached_seldepth = report.seldepth;
                        iterations.push((report.depth, report.nodes));
                        send(&out, &info_line(report, show_wdl));
                    });
                    if search_stats {
                        // Of the search just done; the checks of a varied
                        // move, which follow, are not part of them.
                        let lines = stats_lines(&iterations, search.nodes(), &search.stats());
                        send_block(&out, &lines);
                    }
                    match variety_rng {
                        None => outcome,
                        Some(mut rng) => {
                            let varied = varied_move(
                                &position,
                                &history,
                                evaluator.as_ref(),
                                &outcome,
                                reached,
                                variety,
                                check_limits,
                                &stop,
                                &mut rng,
                            );
                            match (varied, outcome.best_move) {
                                (Some(varied), Some(best)) if varied.best_move != best => {
                                    send(
                                        &out,
                                        &format!(
                                            "info string variety: {} instead of {best}, {} cp \
                                             below it, one of {} moves",
                                            varied.best_move, varied.loss, varied.choices
                                        ),
                                    );
                                    // The last `info` line is for the move played,
                                    // so a GUI records its score and line.
                                    send(
                                        &out,
                                        &info_line(
                                            &Report {
                                                depth: reached,
                                                seldepth: reached_seldepth,
                                                score: varied.score,
                                                nodes: search.nodes() + varied.nodes,
                                                elapsed: started.elapsed(),
                                                pv: vec![varied.best_move],
                                            },
                                            show_wdl,
                                        ),
                                    );
                                    Outcome {
                                        score: varied.score,
                                        best_move: Some(varied.best_move),
                                        pv: vec![varied.best_move],
                                    }
                                }
                                _ => outcome,
                            }
                        }
                    }
                }
                Some((weakening, mut rng)) => {
                    let started = Instant::now();
                    let weak = weakened_move(
                        &position,
                        &history,
                        evaluator.as_ref(),
                        weakening,
                        limits,
                        &stop,
                        &mut rng,
                    );
                    send(
                        &out,
                        &info_line(
                            &Report {
                                depth: weak.depth,
                                // Not recorded for the passes of a weakened
                                // search.
                                seldepth: 0,
                                score: weak.outcome.score,
                                nodes: weak.nodes,
                                elapsed: started.elapsed(),
                                pv: weak.outcome.pv.clone(),
                            },
                            show_wdl,
                        ),
                    );
                    weak.outcome
                }
            };
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
/// Lines are read as bytes. One that is not valid UTF-8 (a file path in an
/// old code page, line noise) is not an error: the bad bytes are replaced
/// and the line is handled like any other, which usually means it is
/// reported as not understood.
///
/// # Errors
///
/// Returns an error if reading `input` fails. A search that was running is
/// allowed to finish and report first.
pub fn run(mut input: impl BufRead, out: Box<dyn Write + Send>) -> io::Result<()> {
    let mut engine = Engine::new(out);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match input.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {
                if !engine.handle(&String::from_utf8_lossy(&bytes)) {
                    return Ok(());
                }
            }
            Err(problem) => {
                engine.wait();
                return Err(problem);
            }
        }
    }
    engine.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::HandCrafted;
    use crate::search::{ELO_MAX, ELO_MIN, Outcome};

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
    fn uci_lists_the_options_before_uciok() {
        let lines = session(&["uci"]);
        let declared: Vec<&String> = lines
            .iter()
            .filter(|line| line.starts_with("option "))
            .collect();
        assert_eq!(declared.len(), Options::declarations().len());
        assert!(!declared.is_empty());
        let first = lines.iter().position(|line| line.starts_with("option "));
        assert!(first.unwrap() >= 2, "after the id lines");
        assert_eq!(lines.last().unwrap(), "uciok");
    }

    #[test]
    fn setoption_changes_an_option_and_says_nothing() {
        let (mut engine, captured) = engine();
        engine.handle("setoption name Hash value 128");
        assert_eq!(engine.options().hash_mb, 128);
        assert!(captured.lines().is_empty());
        // A refused value is reported and changes nothing.
        engine.handle("setoption name Hash value 0");
        assert_eq!(engine.options().hash_mb, 128);
        assert_eq!(
            captured.lines(),
            ["info string Hash: expected a whole number from 1 to 65536, got `0`"]
        );
    }

    /// No test here relies on two entropy seeds differing: an entropy seed
    /// is only checked to be a valid seed and to be the one logged.
    #[test]
    fn the_seed_comes_from_entropy_unless_told_otherwise() {
        let (mut engine, captured) = engine();
        // By default: system entropy, logged in reply to `uci`.
        engine.handle("uci");
        let entropy = engine.rng_seed();
        assert!((1..=crate::rng::SEED_MAX).contains(&entropy));
        let lines = captured.lines();
        assert_eq!(
            lines[lines.len() - 2],
            format!("info string rng seed {entropy} (system entropy)")
        );

        // A seed given: that is the seed, logged as such.
        engine.handle("setoption name Seed value 5");
        assert_eq!(engine.rng_seed(), 5);
        assert_eq!(
            captured.lines().last().unwrap(),
            "info string rng seed 5 (Seed)"
        );
        // Setting the same seed again changes nothing and says nothing.
        let before = captured.lines().len();
        engine.handle("setoption name Seed value 5");
        engine.handle("setoption name Hash value 8");
        assert_eq!(captured.lines().len(), before);
        // Another seed: used and logged.
        engine.handle("setoption name Seed value 77");
        assert_eq!(engine.rng_seed(), 77);
        assert_eq!(
            captured.lines().last().unwrap(),
            "info string rng seed 77 (Seed)"
        );
        // A refused value is reported and keeps the seed.
        engine.handle("setoption name Seed value 2147483648");
        assert_eq!(engine.rng_seed(), 77);

        // Back to 0: a seed from entropy again, logged as such. (Whether
        // it differs from 77 is left to chance and not tested.)
        engine.handle("setoption name Seed value 0");
        let fresh = engine.rng_seed();
        assert!((1..=crate::rng::SEED_MAX).contains(&fresh));
        assert_eq!(
            captured.lines().last().unwrap(),
            &format!("info string rng seed {fresh} (system entropy)")
        );
        // 0 again changes nothing and says nothing.
        let before = captured.lines().len();
        engine.handle("setoption name Seed value 0");
        assert_eq!(engine.rng_seed(), fresh);
        assert_eq!(captured.lines().len(), before);
    }

    /// The seed is logged at every `ucinewgame`, whether it was given or
    /// taken from entropy, and a given seed stays the same.
    #[test]
    fn every_new_game_logs_its_seed() {
        let (mut engine, captured) = engine();
        engine.handle("setoption name Seed value 1234");
        for _ in 0..2 {
            // Each `ucinewgame` adds a line of its own.
            let before = captured.lines().len();
            engine.handle("ucinewgame");
            let lines = captured.lines();
            assert_eq!(lines.len(), before + 1);
            assert_eq!(lines[before], "info string rng seed 1234 (Seed)");
        }
        engine.handle("setoption name Seed value 0");
        let before = captured.lines().len();
        engine.handle("ucinewgame");
        let seed = engine.rng_seed();
        let lines = captured.lines();
        assert_eq!(lines.len(), before + 1);
        assert_eq!(
            lines[before],
            format!("info string rng seed {seed} (system entropy)")
        );
    }

    /// The lines of a weakened game from the start position with a chosen
    /// seed: one `go` per entry of `commands` after the options are set.
    fn weakened(seed: u32, commands: &[&str]) -> Vec<String> {
        assert_ne!(seed, 0, "Seed 0 would mean a seed from entropy");
        let mut lines = vec![
            format!("setoption name Seed value {seed}"),
            "setoption name UCI_LimitStrength value true".to_string(),
        ];
        lines.extend(commands.iter().map(ToString::to_string));
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        played(&lines)
    }

    /// Feeds `lines` to a new engine as a GUI does, waiting for the move
    /// after every `go` before sending anything else (the next command
    /// would otherwise end the search early), and returns the output
    /// without speeds and times.
    fn played(lines: &[&str]) -> Vec<String> {
        let (mut engine, captured) = engine();
        for line in lines {
            assert!(engine.handle(line), "{line}");
            if line.starts_with("go") {
                engine.wait();
            }
        }
        stable(&captured.lines())
    }

    /// Waits until the engine has written something, for up to ten
    /// seconds. A fixed pause is not enough on a busy machine.
    fn wait_for_output(captured: &Captured) {
        let started = Instant::now();
        while captured.lines().is_empty() {
            assert!(started.elapsed() < Duration::from_secs(10), "no output");
            thread::sleep(Duration::from_millis(1));
        }
    }

    const QUEEN_ENDING_AFTER_MOVES: &str =
        "position fen 7k/8/8/8/8/2K5/8/1Q6 w - - 10 30 moves b1c1 h8g8 c1b1";
    /// Perft position 6, a quiet middlegame, with either side to move.
    const MIDDLEGAME: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1";

    /// Limited strength is still given the game's history and the limits
    /// of `go`.
    #[test]
    fn limited_strength_gets_the_history_and_the_limits() {
        // The repetition is found only with the history: a draw, where the
        // same position as a bare FEN is lost.
        let strongest = format!("setoption name UCI_Elo value {ELO_MAX}");
        let with_history = weakened(2, &[&strongest, QUEEN_ENDING_AFTER_MOVES, GO]);
        assert!(
            with_history[with_history.len() - 2].contains(" score cp 0 "),
            "{with_history:?}"
        );
        assert_eq!(with_history.last().unwrap(), "bestmove g8h8");
        let bare = weakened(
            2,
            &[
                &strongest,
                "position fen 6k1/8/8/8/8/2K5/8/1Q6 b - - 13 31",
                GO,
            ],
        );
        assert!(!bare[bare.len() - 2].contains(" score cp 0 "), "{bare:?}");

        // A node limit cuts the passes short: one ply, where the weakest
        // setting unhurried looks two.
        let kiwipete = format!("position fen {KIWIPETE}");
        let weakest = format!("setoption name UCI_Elo value {ELO_MIN}");
        let hurried = weakened(2, &[&weakest, &kiwipete, "go nodes 1"]);
        assert!(
            hurried[hurried.len() - 2].starts_with("info depth 1 "),
            "{hurried:?}"
        );
        let calm = weakened(2, &[&weakest, &kiwipete, GO]);
        assert!(
            calm[calm.len() - 2].starts_with("info depth 2 "),
            "{calm:?}"
        );
    }

    /// The side to move thinks on its own clock. Each `go` here gives the
    /// mover ten minutes and the opponent nothing, with a depth that takes
    /// far longer than a millisecond: reading the wrong clock would cut the
    /// search off after the first depth.
    #[test]
    fn go_uses_the_clock_of_the_side_to_move() {
        for (side, go) in [
            ("w", "go wtime 600000 btime 0 depth 2"),
            ("b", "go wtime 0 btime 600000 depth 2"),
        ] {
            let lines = played(&[&format!("position fen {MIDDLEGAME} {side} - - 0 10"), go]);
            assert_eq!(lines.len(), 3, "{side}: {lines:?}");
            assert!(lines[1].starts_with("info depth 2 "));
        }
        // And the other way round: with nothing on its own clock the mover
        // answers at once, however much the opponent has.
        for (side, go) in [
            ("w", "go wtime 0 btime 600000"),
            ("b", "go wtime 600000 btime 0"),
        ] {
            let started = Instant::now();
            let lines = played(&[&format!("position fen {MIDDLEGAME} {side} - - 0 10"), go]);
            assert!(started.elapsed() < Duration::from_secs(5), "{side}");
            assert!(lines.last().unwrap().starts_with("bestmove "));
        }
        // A clock within `Move Overhead` (30 ms) gets no time at all: the
        // move comes from the first depth alone, every time.
        for go in ["go wtime 30 btime 600000", "go wtime 1 btime 600000 winc 9"] {
            let lines = played(&[&format!("position fen {MIDDLEGAME} w - - 0 10"), go]);
            assert_eq!(lines.len(), 2, "{go}: {lines:?}");
            assert!(lines[0].starts_with("info depth 1 "));
            assert!(lines[1].starts_with("bestmove "));
        }
    }

    /// `Move Overhead` reaches the clock. With five seconds kept back from a
    /// five-second move time there is a millisecond left, so the answer
    /// comes at once; with the default 30 ms the same `go` would think for
    /// almost five seconds.
    #[test]
    fn move_overhead_is_kept_back_from_the_clock() {
        let (mut engine, captured) = engine();
        engine.handle("setoption name Move Overhead value 5000");
        assert_eq!(engine.options().move_overhead_ms, 5_000);
        assert!(captured.lines().is_empty());
        let started = Instant::now();
        engine.handle(&format!("position fen {MIDDLEGAME} w - - 0 10"));
        engine.handle("go movetime 5000");
        engine.wait();
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(captured.lines().last().unwrap().starts_with("bestmove "));
        // And the other way: with nothing kept back, a move time is used,
        // so the search gets past the first depth.
        let lines = played(&[
            "setoption name Move Overhead value 0",
            &format!("position fen {MIDDLEGAME} w - - 0 10"),
            "go movetime 3000 depth 2",
        ]);
        assert!(lines.iter().any(|line| line.starts_with("info depth 2 ")));
    }

    /// Full strength draws no random numbers: a full-strength move played
    /// first leaves the weakened move that follows exactly as it would
    /// have been.
    #[test]
    fn full_strength_does_not_draw_random_numbers() {
        for seed in 1..=6 {
            let seed_option = format!("setoption name Seed value {seed}");
            let after_full_strength = played(&[
                &seed_option,
                "go depth 2",
                "setoption name UCI_LimitStrength value true",
                GO,
            ]);
            let alone = weakened(seed, &[GO]);
            assert_eq!(after_full_strength.last(), alone.last(), "seed {seed}");
        }
    }

    /// Which stream the engine draws from, and how each search's generator
    /// is taken from it, is part of what makes a logged seed replayable:
    /// the engine's move is the one the library gives for the first number
    /// of the strength stream of that seed.
    #[test]
    fn weakened_play_draws_from_the_strength_stream() {
        for seed in 1..=6 {
            let mut base = Rng::stream(seed, stream::STRENGTH);
            let mut rng = Rng::from_state(base.next_u64());
            let stop = AtomicBool::new(false);
            let expected = weakened_move(
                &Position::startpos(),
                &[],
                &HandCrafted,
                Weakening::for_elo(crate::search::ELO_DEFAULT),
                Limits::NONE,
                &stop,
                &mut rng,
            );
            let lines = weakened(seed, &[GO]);
            assert_eq!(
                lines.last().unwrap(),
                &bestmove_line(expected.outcome.best_move),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn position_and_ucinewgame_end_a_running_search() {
        let (mut engine, captured) = engine();
        engine.handle(&format!("position fen {KIWIPETE}"));
        engine.handle("go infinite");
        engine.handle("position startpos");
        assert_eq!(
            best_moves(&captured),
            1,
            "ended before the position changed"
        );
        engine.handle("go infinite");
        engine.handle("ucinewgame");
        assert_eq!(best_moves(&captured), 2);
        // The second search was of the start position, not of Kiwipete.
        let last = captured.lines();
        let last = last
            .iter()
            .rev()
            .find(|line| line.starts_with("bestmove "))
            .unwrap();
        let best = last.strip_prefix("bestmove ").unwrap();
        assert!(find_legal_move(&Position::startpos(), best).is_some());
    }

    #[test]
    fn infinite_with_a_depth_still_waits_for_stop() {
        let (mut engine, captured) = engine();
        engine.handle("go infinite depth 1");
        wait_for_output(&captured);
        thread::sleep(Duration::from_millis(50));
        assert_eq!(best_moves(&captured), 0, "{:?}", captured.lines());
        // The depth limit was honoured: one depth and no more.
        assert_eq!(captured.lines().len(), 1);
        engine.handle("stop");
        assert_eq!(best_moves(&captured), 1);
    }

    /// A `go` whose limit is never reached, so the result does not depend
    /// on the machine: only depth and node limits are reproducible.
    const GO: &str = "go nodes 50000000";

    fn best_of(lines: &[String]) -> Vec<&String> {
        lines
            .iter()
            .filter(|line| line.starts_with("bestmove"))
            .collect()
    }

    #[test]
    fn limited_strength_plays_a_weakened_move() {
        let lines = weakened(1, &["go depth 9"]);
        // The seed line, then one `info` for the weakened choice (its own
        // shallow depth, whatever `go` asked for), then the move.
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0], "info string rng seed 1 (Seed)");
        assert!(
            lines[1].starts_with("info depth 3 score cp "),
            "{}",
            lines[1]
        );
        let best = lines[2].strip_prefix("bestmove ").unwrap();
        assert!(find_legal_move(&Position::startpos(), best).is_some());
        assert!(lines[1].ends_with(&format!(" pv {best}")));
    }

    #[test]
    fn a_chosen_seed_makes_weakened_play_reproducible() {
        let game = [GO, "position startpos moves e2e4", GO];
        assert_eq!(weakened(7, &game), weakened(7, &game));
        // Other seeds play other moves: of twenty, not all open the same.
        let openings: Vec<Vec<String>> = (1..=20).map(|seed| weakened(seed, &[GO])).collect();
        let first = best_of(&openings[0])[0].clone();
        assert!(
            openings.iter().any(|lines| *best_of(lines)[0] != first),
            "every seed played {first}"
        );
    }

    #[test]
    fn a_new_game_restarts_the_sequence_of_a_chosen_seed() {
        for seed in 1..=5 {
            let lines = weakened(seed, &[GO, "ucinewgame", GO]);
            let best = best_of(&lines);
            assert_eq!(best[0], best[1], "seed {seed}");
        }
        // Without a new game the sequence runs on: the same position is not
        // always answered with the same move.
        let differs = (1..=20).any(|seed| {
            let lines = weakened(seed, &[GO, GO]);
            let best = best_of(&lines);
            best[0] != best[1]
        });
        assert!(differs);
    }

    #[test]
    fn the_rating_sets_how_weakly_it_plays() {
        // The two ends of the range differ in how far they look, which the
        // `info` line shows.
        let weakest = format!("setoption name UCI_Elo value {ELO_MIN}");
        let low = weakened(3, &[&weakest, GO]);
        assert!(low[1].starts_with("info depth 2 "), "{}", low[1]);
        let strongest = format!("setoption name UCI_Elo value {ELO_MAX}");
        let high = weakened(3, &[&strongest, GO]);
        assert!(high[1].starts_with("info depth 3 "), "{}", high[1]);
        // A rating out of range is refused and the old one kept, the old
        // lowest setting included.
        for out in ["3000", "400"] {
            let refused = weakened(3, &[&format!("setoption name UCI_Elo value {out}"), GO]);
            assert_eq!(
                refused[1],
                format!(
                    "info string UCI_Elo: expected a whole number from 1110 to 1630, got `{out}`"
                )
            );
            assert_eq!(
                refused[2..],
                weakened(3, &[GO])[1..],
                "still the default rating"
            );
        }
        // Without `UCI_LimitStrength` the rating changes nothing.
        assert_eq!(
            stable(&session(&[&weakest, "go depth 3"])),
            stable(&session(&["go depth 3"]))
        );
    }

    /// The seed logged for a game, read back from an `info string`.
    fn logged_seed(line: &str) -> u32 {
        let rest = line.strip_prefix("info string rng seed ").unwrap();
        rest.split(' ').next().unwrap().parse().unwrap()
    }

    /// Without a chosen seed every game gets its own entropy seed, logged
    /// at `ucinewgame`. That one number is enough to replay the game, also
    /// the second game of a session, which is what a match tool plays: one
    /// engine process, `ucinewgame` between games.
    #[test]
    fn each_game_of_an_entropy_session_can_be_replayed() {
        let (mut engine, captured) = engine();
        engine.handle("setoption name UCI_LimitStrength value true");
        let mut games = Vec::new();
        for _ in 0..3 {
            engine.handle("ucinewgame");
            let seed = logged_seed(captured.lines().last().unwrap());
            assert_eq!(seed, engine.rng_seed());
            let mut moves = Vec::new();
            for position in ["position startpos", "position startpos moves e2e4 e7e5"] {
                engine.handle(position);
                engine.handle(GO);
                engine.wait();
                moves.push(captured.lines().last().unwrap().clone());
            }
            games.push((seed, moves));
        }
        // Whether the games had different seeds is left to chance and not
        // tested; what matters is that each can be replayed from its own.
        for (seed, moves) in games {
            let replay = weakened(
                seed,
                &[
                    "ucinewgame",
                    "position startpos",
                    GO,
                    "position startpos moves e2e4 e7e5",
                    GO,
                ],
            );
            let replayed: Vec<String> = best_of(&replay).into_iter().cloned().collect();
            assert_eq!(replayed, moves, "seed {seed}");
        }
    }

    #[test]
    fn full_strength_is_untouched_by_the_option_being_off() {
        let plain = stable(&session(&["go depth 3"]));
        let off = stable(&session(&[
            "setoption name UCI_LimitStrength value true",
            "setoption name UCI_LimitStrength value false",
            "go depth 3",
        ]));
        assert_eq!(off, plain);
        // And it is not the weakened search under another name.
        assert_ne!(weakened(1, &["go depth 3"])[2..], plain[..]);
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
        // The selective depths: quiescence goes deeper than the search.
        for (line, (depth, seldepth)) in lines.iter().zip([(1, 1), (2, 4), (3, 8)]) {
            assert!(
                line.starts_with(&format!("info depth {depth} seldepth {seldepth} score cp ")),
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

    /// The move of a session's last `bestmove` line.
    fn last_best(lines: &[String]) -> String {
        lines
            .iter()
            .rev()
            .find_map(|line| line.strip_prefix("bestmove "))
            .unwrap()
            .to_string()
    }

    /// `UCI_ShowWDL` adds the chances after the score of every `info` line
    /// of a search, weakened ones included, and changes nothing else: the
    /// lines without them, and the move, are those of the plain search.
    #[test]
    fn show_wdl_adds_chances_and_changes_nothing_else() {
        let on = "setoption name UCI_ShowWDL value true";
        let limited = "setoption name UCI_LimitStrength value true";
        for setup in [&[][..], &["setoption name Seed value 7", limited]] {
            let mut plain: Vec<&str> = setup.to_vec();
            plain.push("go depth 3");
            let mut shown = vec![on];
            shown.extend(&plain);
            let (plain, shown) = (stable(&session(&plain)), stable(&session(&shown)));
            assert_eq!(plain.len(), shown.len());
            assert!(plain.iter().all(|line| !line.contains(" wdl ")));
            let mut seen = 0;
            for (plain, shown) in plain.iter().zip(&shown) {
                if !plain.starts_with("info depth ") {
                    assert_eq!(plain, shown);
                    continue;
                }
                let words: Vec<&str> = shown.split(' ').collect();
                let at = words.iter().position(|&word| word == "wdl").unwrap();
                // Straight after `score cp <n>` or `score mate <n>`.
                assert_eq!(words[at - 3], "score", "{shown}");
                let chances: Vec<u32> = words[at + 1..at + 4]
                    .iter()
                    .map(|word| word.parse().unwrap())
                    .collect();
                assert_eq!(chances.iter().sum::<u32>(), 1000, "{shown}");
                let without = [&words[..at], &words[at + 4..]].concat().join(" ");
                assert_eq!(&without, plain);
                seen += 1;
            }
            assert!(seen >= 1, "{shown:?}");
        }
    }

    /// `SearchStats` adds the statistics of a full-strength search before
    /// its `bestmove`, as `info string` lines, and changes nothing else. A
    /// weakened search has none to show.
    #[test]
    fn search_stats_are_sent_before_the_best_move() {
        let plain = stable(&session(&["go depth 3"]));
        let on = "setoption name SearchStats value true";
        let shown = stable(&session(&[on, "go depth 3"]));
        assert_eq!(shown.len(), plain.len() + 5, "{shown:?}");
        assert_eq!(shown[..3], plain[..3]);
        assert_eq!(shown[3], "info string stats depth 1 nodes 21");
        assert_eq!(shown[4], "info string stats depth 2 nodes 222 ebf 10.57");
        assert_eq!(shown[5], "info string stats depth 3 nodes 2690 ebf 12.11");
        assert!(shown[6].starts_with("info string stats cutoffs "));
        assert!(shown[7].starts_with("info string stats qnodes "));
        assert!(shown[7].contains(" of 2933 ("));
        assert_eq!(shown[8], plain[3]);
        assert!(shown[8].starts_with("bestmove "));
        // The figures are those of the library's own search.
        let pos = Position::startpos();
        let mut search = Search::new(&HandCrafted).with_limits(Limits::depth(3));
        let _ = search.iterate(&pos, &mut |_| {});
        let own = stats_lines(&[], search.nodes(), &search.stats());
        assert_eq!(shown[6], format!("info string {}", own[0]));
        assert_eq!(shown[7], format!("info string {}", own[1]));
        // Limited strength: the option is accepted and nothing is added.
        let limited = [
            "setoption name Seed value 7",
            "setoption name UCI_LimitStrength value true",
            "go depth 3",
        ];
        let mut with_stats = vec![on];
        with_stats.extend(limited);
        assert_eq!(stable(&session(&with_stats)), stable(&session(&limited)));
    }

    /// The node count of the last `info depth` line of `lines`.
    fn last_nodes(lines: &[String]) -> u64 {
        let line = lines.iter().rfind(|line| line.starts_with("info depth "));
        let words: Vec<&str> = line.unwrap().split(' ').collect();
        let at = words.iter().position(|&word| word == "nodes").unwrap();
        words[at + 1].parse().unwrap()
    }

    /// The transposition table is kept from one search of a game to the
    /// next and emptied by `ucinewgame`: the same search again is quicker,
    /// and after a new game it is the first search again, line for line.
    #[test]
    fn the_table_lasts_a_game_and_a_new_game_empties_it() {
        let go = "go depth 5";
        let first = stable(&played(&[go]));
        let twice = stable(&played(&[go, go]));
        assert_eq!(twice[..first.len()], first[..]);
        let again = &twice[first.len()..];
        assert_eq!(again.last(), first.last());
        assert!(last_nodes(again) * 4 < last_nodes(&first), "{again:?}");
        // A new game in between: the seed line it logs, then the first
        // search once more.
        let renewed = stable(&played(&[
            "setoption name Seed value 5",
            go,
            "ucinewgame",
            go,
        ]));
        let seed = "info string rng seed 5 (Seed)";
        assert_eq!(renewed[0], seed);
        assert_eq!(renewed[1..=first.len()], first[..]);
        assert_eq!(renewed[first.len() + 1], seed);
        assert_eq!(renewed[first.len() + 2..], first[..]);
    }

    /// `Hash` sizes the table from the next search on. A new size is a new,
    /// empty table; the same size again keeps what the table holds.
    #[test]
    fn hash_sizes_the_table() {
        let (mut engine, captured) = engine();
        assert_eq!(engine.table_mb, HASH_DEFAULT);
        let slots = |engine: &Engine| engine.table.lock().unwrap().len();
        assert_eq!(slots(&engine), 1 << 20);
        engine.handle("setoption name Hash value 2");
        // Not yet: an option applies from the next search.
        assert_eq!(slots(&engine), 1 << 20);
        engine.handle("go depth 4");
        engine.wait();
        assert_eq!((engine.table_mb, slots(&engine)), (2, 1 << 17));
        let first = last_nodes(&captured.lines());
        // The same size named again: the table is kept, so the search is
        // quicker.
        engine.handle("setoption name Hash value 2");
        engine.handle("go depth 4");
        engine.wait();
        assert!(last_nodes(&captured.lines()) * 2 < first);
        // Another size: an empty table, and the first search's work again.
        engine.handle("setoption name Hash value 1");
        engine.handle("go depth 4");
        engine.wait();
        assert_eq!((engine.table_mb, slots(&engine)), (1, 1 << 16));
        assert_eq!(last_nodes(&captured.lines()), first);
        // A size that is refused changes nothing.
        engine.handle("setoption name Hash value 0");
        engine.handle("go depth 1");
        engine.wait();
        assert_eq!(engine.table_mb, 1);
    }

    /// With `Variety` 0, the default, the root move is always the search's
    /// best, with the same output, whatever the seed.
    #[test]
    fn variety_0_plays_the_best_move() {
        let plain = stable(&session(&["go depth 3"]));
        assert_eq!(
            stable(&session(&["setoption name Variety value 0", "go depth 3"])),
            plain
        );
        for seed in 1..=3 {
            let lines = stable(&session(&[
                &format!("setoption name Seed value {seed}"),
                "setoption name Variety value 0",
                "go depth 3",
            ]));
            assert_eq!(lines[1..], plain[..], "seed {seed}");
        }
        assert_eq!(
            last_best(&plain),
            direct(crate::board::START_FEN, 3)
                .0
                .best_move
                .unwrap()
                .to_string()
        );
    }

    /// With a fixed `Seed` and `go depth`, a varied game is reproducible;
    /// over several seeds some move other than the best is played, and each
    /// time the engine says so and stays within `Variety`.
    #[test]
    fn variety_with_a_fixed_seed_is_reproducible_and_near_the_best() {
        let plain = stable(&session(&["go depth 3"]));
        let best = last_best(&plain);
        let start = Position::startpos();
        let best_score = direct(crate::board::START_FEN, 3).0.score;
        let mut varied = 0;
        for seed in 1..=12 {
            let game = [
                format!("setoption name Seed value {seed}"),
                "setoption name Variety value 60".to_string(),
                "go depth 3".to_string(),
            ];
            let game: Vec<&str> = game.iter().map(String::as_str).collect();
            let lines = stable(&session(&game));
            assert_eq!(lines, stable(&session(&game)), "seed {seed}");
            // Root only: after the seed line, the search's own lines are
            // exactly those of the plain search; only what follows differs.
            assert_eq!(lines[1..4], plain[..3], "seed {seed}");
            let played = last_best(&lines);
            if played != best {
                varied += 1;
                // An independent exact search of the move played puts it
                // within 60 centipawns of the best move's score.
                let mv = find_legal_move(&start, &played).unwrap();
                let mut next = start.clone();
                let _undo = next.make_move(mv);
                let score = -Search::new(&HandCrafted)
                    .with_history(&[start.hash()])
                    .reply_value(&next, 2);
                assert!(
                    (0..=60).contains(&(best_score - score)),
                    "seed {seed}: {played}"
                );
                // The last `info` line is for the move played.
                // It keeps the selective depth of the search's last
                // iteration (8, as the third line of the plain search says).
                assert!(plain[2].starts_with("info depth 3 seldepth 8 "));
                let last_info = &lines[lines.len() - 2];
                assert!(
                    last_info.starts_with(&format!("info depth 3 seldepth 8 score cp {score} "))
                        && last_info.ends_with(&format!(" pv {played}")),
                    "{last_info}"
                );
                let note = lines
                    .iter()
                    .find(|line| line.starts_with("info string variety: "))
                    .unwrap();
                assert!(
                    note.starts_with(&format!(
                        "info string variety: {played} instead of {best}, "
                    )),
                    "{note}"
                );
                let loss: i32 = note
                    .split(", ")
                    .nth(1)
                    .unwrap()
                    .split(' ')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!((0..=60).contains(&loss), "{note}");
            }
        }
        assert!(varied > 0, "no seed varied the move");
    }

    /// After the twelfth full move, and under `UCI_LimitStrength`, `Variety`
    /// changes nothing.
    #[test]
    fn variety_is_only_for_the_opening_at_full_strength() {
        let late =
            "position fen r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 13";
        let plain = stable(&session(&[late, "go depth 3"]));
        for seed in 1..=4 {
            let lines = stable(&session(&[
                &format!("setoption name Seed value {seed}"),
                "setoption name Variety value 100",
                late,
                "go depth 3",
            ]));
            assert_eq!(lines[1..], plain[..], "seed {seed}");
        }
        // Limited strength draws exactly what it drew before: Variety adds
        // no draw of its own there. Over three moves an extra draw at the
        // first would shift the generators of the second and third.
        let moves = [
            GO,
            "position startpos moves e2e4",
            GO,
            "position startpos moves e2e4 e7e5",
            GO,
        ];
        for seed in 1..=3 {
            let mut with_variety = vec!["setoption name Variety value 100"];
            with_variety.extend(moves);
            assert_eq!(
                weakened(seed, &with_variety),
                weakened(seed, &moves),
                "seed {seed}"
            );
        }
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
            lines[1].starts_with("info depth 2 seldepth 3 score cp 0 "),
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
        assert!(lines[2].starts_with("info depth 3 seldepth 4 score mate 2 "));
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
        wait_for_output(&captured);
        assert!(captured.lines()[0].starts_with("info depth 1 seldepth 1 score mate 1 "));
        // The search is over (a mate is proven), yet no move is announced.
        thread::sleep(Duration::from_millis(50));
        assert_eq!(best_moves(&captured), 0, "{:?}", captured.lines());
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
        wait_for_output(&captured);
        thread::sleep(Duration::from_millis(50));
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

    /// `d` draws the position the next `go` would search.
    #[test]
    fn d_shows_the_board_the_fen_the_key_and_the_checkers() {
        let lines = session(&["position startpos moves e2e4", "d"]);
        let after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let key = Position::from_fen(after_e4).unwrap().hash();
        assert_eq!(lines.len(), 12, "{lines:?}");
        assert_eq!(lines[0], "info string 8 r n b q k b n r");
        assert_eq!(lines[4], "info string 4 . . . . P . . .");
        assert_eq!(lines[8], "info string + a b c d e f g h");
        assert_eq!(lines[9], format!("info string fen: {after_e4}"));
        assert_eq!(lines[10], format!("info string key: {key:016x}"));
        assert_eq!(lines[11], "info string checkers: -");
        // A double check: both checking pieces are named.
        let double = "position fen 4k3/8/3N4/8/8/8/8/4R1K1 b - - 0 1";
        let lines = session(&[double, "d"]);
        assert_eq!(lines[11], "info string checkers: e1 d6");
    }

    /// `eval` sends what the `eval` command of the binary prints.
    #[test]
    fn eval_shows_the_evaluation_of_the_current_position() {
        let after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let expected: Vec<String> = eval::report(&Position::from_fen(after_e4).unwrap())
            .iter()
            .map(|line| format!("info string {line}"))
            .collect();
        assert_eq!(expected.len(), 7);
        assert_eq!(session(&["position startpos moves e2e4", "eval"]), expected);
        assert!(expected.contains(&"info string total for White: +40".to_owned()));
    }

    /// Neither command ends a search or changes what it finds, and their
    /// blocks arrive whole.
    #[test]
    fn d_and_eval_leave_a_running_search_alone() {
        let plain = played(&["go depth 3"]);
        let (mut engine, captured) = engine();
        engine.handle("go depth 3");
        engine.handle("d");
        engine.handle("eval");
        engine.wait();
        let lines = captured.lines();
        let start = lines
            .iter()
            .position(|line| line == "info string 8 r n b q k b n r")
            .unwrap();
        assert_eq!(lines[start + 8], "info string + a b c d e f g h");
        assert_eq!(lines[start + 11], "info string checkers: -");
        let search: Vec<String> = lines
            .iter()
            .filter(|line| !line.starts_with("info string "))
            .cloned()
            .collect();
        assert_eq!(stable(&search), plain);
    }

    #[test]
    fn every_line_of_output_is_protocol() {
        let lines = session(&[
            "uci",
            "isready",
            "nonsense",
            "d",
            "eval",
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

    /// Bytes that are not UTF-8 spoil one line, not the session: the line
    /// is reported like anything else that is not understood, the commands
    /// after it are read, and a search that was running still reports.
    #[test]
    fn input_that_is_not_utf8_is_survived() {
        let captured = Captured::default();
        let mut input = b"go depth 2\n".to_vec();
        input.extend_from_slice(b"\xff\xfe junk\n");
        input.extend_from_slice(b"setoption name Hash value caf\xe9\n");
        input.extend_from_slice(b"isready\n");
        run(input.as_slice(), Box::new(captured.clone())).unwrap();
        let lines = captured.lines();
        assert_eq!(best_moves(&captured), 1, "{lines:?}");
        assert!(lines.contains(&"readyok".to_string()));
        assert!(
            lines.contains(&"info string unknown command: \u{fffd}\u{fffd} junk".to_string()),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("info string Hash: expected")),
            "{lines:?}"
        );
    }

    /// A reader that fails after its first line.
    struct Failing(bool);

    impl io::Read for Failing {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                return Err(io::Error::other("the pipe broke"));
            }
            self.0 = true;
            let line = b"go depth 2\n";
            buffer[..line.len()].copy_from_slice(line);
            Ok(line.len())
        }
    }

    #[test]
    fn a_read_error_ends_the_loop_after_the_search_has_reported() {
        let captured = Captured::default();
        let result = run(
            io::BufReader::new(Failing(false)),
            Box::new(captured.clone()),
        );
        assert_eq!(result.unwrap_err().to_string(), "the pipe broke");
        assert_eq!(best_moves(&captured), 1, "{:?}", captured.lines());
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
