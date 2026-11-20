// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The UCI interface, tested on the real binary over real pipes.
//!
//! The unit tests in `src/uci/` cover the protocol in detail on an engine
//! object. These tests check what only a separate process can show: that
//! the binary starts the loop when it has no arguments, that every line
//! reaches the other side of the pipe as soon as it is written, that the
//! engine answers while it is thinking, and that the process ends when it
//! should.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use vyvrael::board::Position;
use vyvrael::eval::HandCrafted;
use vyvrael::movegen::find_legal_move;
use vyvrael::search::{Limits, Search};

/// A running engine process and both ends of its pipes.
struct Engine {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Engine {
    fn start() -> Engine {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vyvrael"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the engine binary starts");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Engine {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("input is still open");
        writeln!(stdin, "{line}").unwrap();
        stdin.flush().unwrap();
    }

    /// Reads one line. Blocks until the engine writes one, which is the
    /// point: an engine that did not flush would hang the test here.
    fn line(&mut self) -> String {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).unwrap();
        assert!(read > 0, "the engine closed its output");
        line.trim_end_matches(['\r', '\n']).to_string()
    }

    /// Reads lines up to and including the first that starts with `prefix`
    /// and returns them all.
    fn lines_until(&mut self, prefix: &str) -> Vec<String> {
        let mut lines = Vec::new();
        loop {
            let line = self.line();
            let found = line.starts_with(prefix);
            lines.push(line);
            if found {
                return lines;
            }
        }
    }

    /// Closes the engine's input and waits for it to end. Returns whether
    /// it ended successfully, what was left on its output, and its stderr.
    fn finish(mut self) -> (bool, Vec<String>, String) {
        drop(self.stdin.take());
        let rest: Vec<String> = (&mut self.stdout).lines().map(Result::unwrap).collect();
        let output = self.child.wait_with_output().unwrap();
        (
            output.status.success(),
            rest,
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

#[test]
fn a_whole_conversation() {
    let mut engine = Engine::start();
    engine.send("uci");
    let hello = engine.lines_until("uciok");
    // The name is sent in UTF-8, with its ligature.
    assert_eq!(
        hello[0],
        format!("id name Vyvræl {}", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(hello[1], "id author Nexxus Drako Multimedia");
    assert_eq!(
        hello
            .iter()
            .filter(|line| line.starts_with("option name "))
            .count(),
        6
    );

    engine.send("isready");
    assert_eq!(engine.line(), "readyok");

    engine.send("ucinewgame");
    engine.send("position startpos moves e2e4 e7e5");
    engine.send("go depth 3");
    let search = engine.lines_until("bestmove");

    // The same search, made directly through the library.
    let mut pos = Position::startpos();
    for text in ["e2e4", "e7e5"] {
        let mv = find_legal_move(&pos, text).unwrap();
        let _undo = pos.make_move(mv);
    }
    let mut direct = Search::new(&HandCrafted).with_limits(Limits::depth(3));
    let outcome = direct.iterate(&pos, &mut |_| {});
    assert_eq!(search.len(), 4, "{search:?}");
    assert!(search[2].contains(&format!(" nodes {} ", direct.nodes())));
    assert_eq!(
        search[3],
        format!("bestmove {}", outcome.best_move.unwrap())
    );

    engine.send("quit");
    let (ok, rest, stderr) = engine.finish();
    assert!(ok);
    assert!(rest.is_empty(), "nothing after quit: {rest:?}");
    assert!(stderr.is_empty(), "nothing on stderr: {stderr}");
}

#[test]
fn it_answers_while_thinking_and_stops_when_told() {
    let mut engine = Engine::start();
    engine
        .send("position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    engine.send("go infinite");
    // The first depth arrives while the search goes on.
    assert!(engine.line().starts_with("info depth 1 "));
    engine.send("isready");
    let seen = engine.lines_until("readyok");
    assert!(
        seen.iter().all(|line| !line.starts_with("bestmove")),
        "an infinite search does not answer by itself: {seen:?}"
    );

    let stopping = Instant::now();
    engine.send("stop");
    let last = engine.lines_until("bestmove").pop().unwrap();
    assert!(stopping.elapsed() < Duration::from_secs(5));
    assert_ne!(last, "bestmove 0000");

    // It is ready for the next command, and `quit` ends the process.
    engine.send("isready");
    assert_eq!(engine.line(), "readyok");
    engine.send("quit");
    assert!(engine.finish().0);
}

#[test]
fn quit_ends_a_running_search() {
    let mut engine = Engine::start();
    engine.send("go infinite");
    assert!(engine.line().starts_with("info depth 1 "));
    let quitting = Instant::now();
    engine.send("quit");
    let (ok, rest, _) = engine.finish();
    assert!(ok);
    assert!(quitting.elapsed() < Duration::from_secs(5));
    assert!(rest.last().unwrap().starts_with("bestmove "), "{rest:?}");
}

#[test]
fn the_end_of_input_lets_the_search_finish() {
    // What `printf 'go depth 3\n' | vyvrael` does: no `quit`, the input
    // simply ends.
    let mut engine = Engine::start();
    engine.send("go depth 3");
    let (ok, rest, stderr) = engine.finish();
    assert!(ok);
    assert_eq!(rest.len(), 4, "three depths and the move: {rest:?}");
    assert!(rest[2].starts_with("info depth 3 "));
    assert!(rest[3].starts_with("bestmove "));
    assert!(stderr.is_empty());
}

#[test]
fn bad_input_is_survived() {
    let mut engine = Engine::start();
    for line in [
        "",
        "hello",
        "position",
        "position fen not a fen at all",
        "position startpos moves e2e5",
        "position startpos moves e2e4 e2e4",
        "setoption name Hash value -5",
        "setoption name NoSuchOption value 1",
        "setoption",
        "go depth banana wtime x",
        "stop",
    ] {
        engine.send(line);
    }
    // `go depth banana wtime x` has no usable limit, so it is an infinite
    // search, and `stop` ended it. Everything before it was refused.
    let lines = engine.lines_until("bestmove");
    let refusals = lines
        .iter()
        .filter(|line| line.starts_with("info string "))
        .count();
    assert_eq!(refusals, 8, "{lines:?}");
    // The refused `position` commands left the start position in place.
    let best = lines.last().unwrap().strip_prefix("bestmove ").unwrap();
    assert!(find_legal_move(&Position::startpos(), best).is_some());

    // Still alive and well.
    engine.send("isready");
    assert_eq!(engine.line(), "readyok");
    engine.send("quit");
    let (ok, _, stderr) = engine.finish();
    assert!(ok);
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn with_arguments_it_is_not_a_uci_engine() {
    let output = Command::new(env!("CARGO_BIN_EXE_vyvrael"))
        .args(["perft", "1"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "nodes: 20"
    );
}
