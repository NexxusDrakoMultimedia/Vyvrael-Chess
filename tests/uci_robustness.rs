// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The UCI loop under malformed and unexpected input.
//!
//! A seeded generator writes streams of random bytes, scrambled and
//! truncated commands, oversized lines, broken FENs, long move lists, odd
//! option values and commands sent while a search runs. Each stream goes
//! through `uci::run`, and the test checks that nothing panicked on any
//! thread, that every `isready` was answered and that every `go` got
//! exactly one `bestmove`, and that each of those moves is legal in the
//! position the engine should have been searching.
//!
//! A second kind of stream is made of real sessions, as a GUI or a match
//! tool sends them (options, a new game, a growing move list with a search
//! after each position), some lines damaged by a byte or two, with games of
//! more than a thousand plies among them. There the engine is given each
//! line as it would arrive, and each search with a limit of its own is left
//! to run to that limit before the next line, so that the later depths, the
//! checks of a varied move and the passes of a weakened one are reached.
//!
//! A stream that the engine does not finish in time fails the test instead
//! of stalling it, and every stream's input is written to Cargo's directory
//! for test files, so that a failure can be looked at and replayed
//! (`vyvrael < uci-robustness-stream-<n>.txt`).
//!
//! This test is a binary of its own because it installs a panic hook, which
//! is global to the process.

use std::io::{self, Cursor, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::Duration;

use vyvrael::board::Position;
use vyvrael::movegen::{find_legal_move, legal_moves};
use vyvrael::rng::Rng;
use vyvrael::uci::{Command, Engine, GoParams, parse, run};

/// Input streams to try, each with a generator whose state is one draw from
/// the default seed's generator.
const STREAMS: u64 = 8;
/// Lines in each stream.
const LINES: usize = 400;

/// How long the engine may take over one stream. A stream takes a few
/// seconds; this is generous so that only a hang, or a machine far too
/// busy to test on, runs into it.
const WATCHDOG: Duration = Duration::from_secs(180);

/// Panics seen on any thread since the hook was installed.
static PANICS: AtomicUsize = AtomicUsize::new(0);

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

/// Positions to start from and to break.
const FENS: [&str; 6] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
    "4k3/8/8/8/8/8/8/4K2R w K - 99 80",
    "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
];

/// Words a command may be made of.
const WORDS: [&str; 40] = [
    "uci",
    "isready",
    "ucinewgame",
    "position",
    "startpos",
    "fen",
    "moves",
    "go",
    "stop",
    "setoption",
    "name",
    "value",
    "Hash",
    "Threads",
    "Seed",
    "Variety",
    "UCI_LimitStrength",
    "UCI_Elo",
    "Move",
    "Overhead",
    "depth",
    "nodes",
    "movetime",
    "wtime",
    "btime",
    "winc",
    "binc",
    "movestogo",
    "infinite",
    "ponder",
    "searchmoves",
    "mate",
    "true",
    "false",
    "e2e4",
    "a7a8q",
    "0000",
    "e1g1",
    "debug",
    "register",
];

/// Numbers a value may be: edges, overflows and things that only look like
/// numbers.
/// Some are not ASCII, so that a command cut short can end in the middle
/// of a character.
const NUMBERS: [&str; 19] = [
    "1500\u{e9}",
    "\u{ff11}\u{ff15}\u{ff10}\u{ff10}",
    "\u{3a9}",
    "0",
    "1",
    "-1",
    "5",
    "64",
    "1500",
    "2147483647",
    "2147483648",
    "-2147483648",
    "9223372036854775807",
    "18446744073709551616",
    "1e9",
    "NaN",
    "-0",
    "0x10",
    "",
];

struct Generator {
    rng: Rng,
    /// Commands cut short in the middle of a character so far.
    cut_characters: usize,
}

impl Generator {
    fn below(&mut self, bound: usize) -> usize {
        self.rng.next_below(bound as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    /// A small number, or one of the awkward ones.
    fn number(&mut self) -> String {
        if self.below(2) == 0 {
            self.below(200).to_string()
        } else {
            self.pick(&NUMBERS).to_string()
        }
    }

    /// Bytes that need not be text, without a line break.
    fn noise(&mut self, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| match self.rng.next_below(256) as u8 {
                b'\n' => b' ',
                byte => byte,
            })
            .collect()
    }

    /// A FEN with a few characters changed, removed or repeated.
    fn broken_fen(&mut self) -> String {
        let mut fen: Vec<u8> = self.pick(&FENS).bytes().collect();
        for _ in 0..self.below(4) {
            if fen.is_empty() {
                break;
            }
            let at = self.below(fen.len());
            match self.below(3) {
                0 => fen[at] = b"pnbrqkPNBRQK12345678/ wb-0123456789abcdefgh9"[self.below(44)],
                1 => {
                    fen.remove(at);
                }
                _ => fen.insert(at, fen[at]),
            }
        }
        String::from_utf8_lossy(&fen).into_owned()
    }

    /// A random game from a position, as UCI moves, and sometimes a last
    /// move that is not legal.
    fn game(&mut self, mut position: Position, plies: usize) -> Vec<String> {
        let mut moves = Vec::new();
        for _ in 0..plies {
            let legal = legal_moves(&position);
            let legal = legal.as_slice();
            if legal.is_empty() {
                break;
            }
            let mv = legal[self.below(legal.len())];
            moves.push(mv.to_string());
            let _undo = position.make_move(mv);
        }
        if self.below(4) == 0 {
            moves.push(self.pick(&WORDS).to_string());
        }
        moves
    }

    fn go(&mut self) -> String {
        let mut line = String::from("go");
        for _ in 0..self.below(5) {
            let word = self.pick(&[
                "depth",
                "nodes",
                "movetime",
                "wtime",
                "btime",
                "winc",
                "binc",
                "movestogo",
                "infinite",
                "ponder",
                "mate",
                "searchmoves",
            ]);
            line.push(' ');
            line.push_str(word);
            if !matches!(word, "infinite" | "ponder") {
                line.push(' ');
                line.push_str(&self.number());
            }
        }
        line
    }

    fn setoption(&mut self) -> String {
        let name = self.pick(&[
            "Hash",
            "Threads",
            "Seed",
            "Variety",
            "UCI_LimitStrength",
            "UCI_Elo",
            "Move Overhead",
            "Clear Hash",
            "Vyvr\u{e6}l",
            "hash",
            "NoSuchOption",
            "",
        ]);
        let value = match self.below(3) {
            0 => self.pick(&["true", "false", "TRUE", "yes"]).to_string(),
            _ => self.number(),
        };
        match self.below(4) {
            0 => format!("setoption name {name}"),
            1 => format!("setoption name {name} value"),
            _ => format!("setoption name {name} value {value}"),
        }
    }

    fn position(&mut self) -> String {
        match self.below(3) {
            0 => format!("position fen {}", self.broken_fen()),
            1 => {
                let fen = self.pick(&FENS);
                let start = Position::from_fen(fen).expect("the listed FENs are valid");
                let plies = self.below(40);
                format!(
                    "position fen {fen} moves {}",
                    self.game(start, plies).join(" ")
                )
            }
            _ => {
                let plies = self.below(400);
                let moves = self.game(Position::startpos(), plies).join(" ");
                format!("position startpos moves {moves}")
            }
        }
    }

    /// `text` with one or two bytes changed, removed or repeated, which may
    /// leave it no longer text.
    fn damaged(&mut self, text: &str) -> Vec<u8> {
        const LETTERS: &[u8] = b"0123456789 abefgkopstuv\xff-";
        let mut bytes = text.as_bytes().to_vec();
        for _ in 0..1 + self.below(2) {
            if bytes.is_empty() {
                break;
            }
            let at = self.below(bytes.len());
            match self.below(3) {
                0 => bytes[at] = LETTERS[self.below(LETTERS.len())],
                1 => {
                    bytes.remove(at);
                }
                _ => bytes.insert(at, bytes[at]),
            }
        }
        bytes
    }

    /// A search with a small limit of its own, of every kind of limit.
    fn small_go(&mut self) -> String {
        match self.below(6) {
            0 => format!("go depth {}", 1 + self.below(2)),
            1 => format!("go movetime {}", 1 + self.below(8)),
            2 => {
                let clock = self.pick(&["0", "20", "40", "300"]);
                let other = self.pick(&["0", "20", "40", "300"]);
                format!("go wtime {clock} btime {other} winc 0 binc 0")
            }
            3 => format!("go depth 2 nodes {}", 100 + self.below(400)),
            _ => format!("go nodes {}", 200 + self.below(3_000)),
        }
    }

    /// One session as a GUI or a match tool sends it: the greeting, some
    /// options, a new game, then a game's positions in order with a search
    /// after each. With `long` the game has more than a thousand plies and
    /// only its last positions are searched.
    fn session(&mut self, long: bool) -> Vec<String> {
        let mut lines = vec!["uci".to_string(), "isready".to_string()];
        // A fixed seed, never 0: what the strength options draw is then
        // the same on every run.
        lines.push(format!(
            "setoption name Seed value {}",
            1 + self.below(1_000)
        ));
        for _ in 0..self.below(6) {
            let (name, value) = match self.below(7) {
                0 => ("UCI_ShowWDL", self.pick(&["true", "false"]).to_string()),
                1 => ("SearchStats", self.pick(&["true", "false"]).to_string()),
                2 => ("Variety", self.pick(&["0", "10", "60", "100"]).to_string()),
                3 => (
                    "UCI_LimitStrength",
                    self.pick(&["true", "false"]).to_string(),
                ),
                4 => ("UCI_Elo", (1_110 + self.below(521)).to_string()),
                5 => ("Move Overhead", self.pick(&["0", "30", "100"]).to_string()),
                _ => ("Hash", self.pick(&["1", "64"]).to_string()),
            };
            lines.push(format!("setoption name {name} value {value}"));
        }
        lines.push("ucinewgame".to_string());
        let start = match self.below(4) {
            // The fifty-move counter at its limit and beyond it, and the
            // move number at the largest a FEN can hold.
            0 => "fen 4k3/8/8/8/8/8/8/4K2R w K - 99 80",
            1 => "fen 4k3/8/8/8/8/8/8/4K2R w K - 120 65535",
            _ => "startpos",
        };
        let from = match start.strip_prefix("fen ") {
            Some(fen) => Position::from_fen(fen).expect("the FENs of a session are valid"),
            None => Position::startpos(),
        };
        let plies = if long {
            1_200 + self.below(600)
        } else {
            self.below(50)
        };
        let game = self.game(from, plies);
        let mut played = if long {
            game.len().saturating_sub(3)
        } else {
            0
        };
        while played <= game.len() {
            let mut line = format!("position {start}");
            if played > 0 {
                line.push_str(" moves ");
                line.push_str(&game[..played].join(" "));
            }
            lines.push(line);
            match self.below(8) {
                0 => lines.push("d".to_string()),
                1 => lines.push("eval".to_string()),
                2 => lines.push("isready".to_string()),
                _ => {}
            }
            lines.push(self.small_go());
            played += 1 + self.below(8);
        }
        lines
    }

    /// One line of input, without its line break.
    fn line(&mut self) -> Vec<u8> {
        let text = match self.below(12) {
            0 => {
                let length = self.below(120);
                return self.noise(length);
            }
            1 => {
                let count = 1 + self.below(12);
                (0..count)
                    .map(|_| self.pick(&WORDS))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            2 | 3 => self.go(),
            4 | 5 => self.position(),
            6 => self.setoption(),
            7 => "isready".to_string(),
            8 => self
                .pick(&["stop", "ucinewgame", "uci", "", "   ", "\t"])
                .to_string(),
            9 => {
                // A command cut short, perhaps in the middle of a character.
                let whole = match self.below(3) {
                    0 => self.go(),
                    1 => self.position(),
                    _ => self.setoption(),
                };
                let mut bytes = whole.into_bytes();
                bytes.truncate(self.below(bytes.len() + 1));
                if std::str::from_utf8(&bytes).is_err() {
                    self.cut_characters += 1;
                }
                return bytes;
            }
            10 => {
                // Oversized: a long run of one word or of digits.
                let word = self.pick(&["moves", "9", "e2e4", "go", "x"]);
                let count = 1_000 + self.below(50_000);
                format!("position startpos moves {}", vec![word; count].join(" "))
            }
            _ => {
                // A known command with noise in front, which UCI skips.
                let length = self.below(8);
                let mut bytes = self.noise(length);
                bytes.push(b' ');
                bytes.extend_from_slice(self.go().as_bytes());
                return bytes;
            }
        };
        text.into_bytes()
    }
}

/// What a stream should leave the engine thinking about, worked out beside
/// it: the position after each line, by the rule that a `position` command
/// with anything wrong in it changes nothing, and the position each `go`
/// was given.
struct Model {
    position: Position,
    /// The position of each `go`, in order.
    searched: Vec<Position>,
}

impl Model {
    fn new() -> Model {
        Model {
            position: Position::startpos(),
            searched: Vec::new(),
        }
    }

    /// Takes in one command as the engine understood it.
    fn follow(&mut self, command: &Command) {
        match command {
            Command::NewGame => self.position = Position::startpos(),
            Command::Position { fen, moves } => {
                let start = match fen {
                    None => Some(Position::startpos()),
                    Some(fen) => Position::from_fen(fen).ok(),
                };
                let reached = start.and_then(|mut position| {
                    for text in moves {
                        let mv = find_legal_move(&position, text)?;
                        let _undo = position.make_move(mv);
                    }
                    Some(position)
                });
                if let Some(position) = reached {
                    self.position = position;
                }
            }
            Command::Go(_) => self.searched.push(self.position.clone()),
            _ => {}
        }
    }

    /// Checks the engine's `bestmove` lines, in order, against the
    /// positions searched: a legal move of that position, or `0000` when
    /// and only when it has none. Returns how many were real moves and how
    /// many `0000`.
    fn check(&self, name: &str, lines: &[&str]) -> (usize, usize) {
        let moves: Vec<&str> = lines
            .iter()
            .filter_map(|line| line.strip_prefix("bestmove "))
            .collect();
        assert_eq!(
            moves.len(),
            self.searched.len(),
            "stream {name}: every go gets one bestmove"
        );
        let mut none = 0;
        for (number, (text, position)) in moves.iter().zip(&self.searched).enumerate() {
            let stuck = legal_moves(position).is_empty();
            let fine = if stuck {
                *text == "0000"
            } else {
                find_legal_move(position, text).is_some()
            };
            assert!(
                fine,
                "stream {name}: bestmove {text} (number {}) is not a move of {position}",
                number + 1
            );
            none += usize::from(stuck);
        }
        (moves.len() - none, none)
    }
}

/// Where the input of stream `name` is kept for looking at afterwards.
fn kept_input(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("uci-robustness-stream-{name}.txt"))
}

/// Feeds `input` to a new engine through `uci::run` and returns everything
/// it wrote. The engine runs on a thread of its own and is given
/// [`WATCHDOG`] to reach the end of the input.
///
/// # Panics
///
/// Panics, naming the file the input was kept in, if the engine is not done
/// in time or its loop ends by a panic. The thread is then left behind: a
/// hung engine cannot be stopped from outside, and the test fails anyway.
fn answer(name: &str, input: &[u8]) -> String {
    watched(name, input, false)
}

/// Whether a search with these limits ends by itself soon enough to be
/// waited for. A damaged line can turn a small limit into a large one, or
/// into none; such a search is told to stop.
fn ends_soon(go: &GoParams) -> bool {
    let small_clock = |clock: Option<u64>| clock.is_some_and(|ms| ms <= 10_000);
    !go.infinite
        && (go.depth.is_some_and(|depth| depth <= 3)
            || go.nodes.is_some_and(|nodes| nodes <= 1_000_000)
            || go.movetime.is_some_and(|ms| ms <= 1_000)
            || (small_clock(go.wtime) && small_clock(go.btime)))
}

/// Gives `input` to a new engine a line at a time, as [`Engine::handle`]
/// takes them, and after each `go` waits for its search: to its own limit
/// if it has a small one, or after a `stop` if it has not.
fn paced(input: &[u8], out: Captured) {
    let mut engine = Engine::new(Box::new(out));
    for line in input.split(|byte| *byte == b'\n') {
        let line = String::from_utf8_lossy(line);
        if !engine.handle(&line) {
            return;
        }
        if let Command::Go(go) = parse(&line) {
            if !ends_soon(&go) {
                engine.handle("stop");
            }
            engine.wait();
        }
    }
    engine.wait();
}

/// [`answer`], or the same with the input given a line at a time and each
/// search waited for (see [`paced`]).
fn watched(name: &str, input: &[u8], pace: bool) -> String {
    let kept = kept_input(name);
    std::fs::write(&kept, input).expect("the input can be kept");
    let out = Captured::default();
    let (done, finished) = mpsc::channel();
    let (theirs, copy) = (out.clone(), input.to_vec());
    thread::spawn(move || {
        let read = if pace {
            paced(&copy, theirs);
            true
        } else {
            run(Cursor::new(copy), Box::new(theirs)).is_ok()
        };
        let _ = done.send(read);
    });
    match finished.recv_timeout(WATCHDOG) {
        Ok(read) => assert!(read, "reading a buffer cannot fail"),
        Err(RecvTimeoutError::Timeout) => panic!(
            "stream {name}: the engine was not done after {} s: a hang, or a machine too \
             busy to test on. Its input is in {}",
            WATCHDOG.as_secs(),
            kept.display()
        ),
        Err(RecvTimeoutError::Disconnected) => panic!(
            "stream {name}: the engine's loop panicked. Its input is in {}",
            kept.display()
        ),
    }
    String::from_utf8(out.0.lock().unwrap().clone()).expect("output is text")
}

/// The watchdog itself: an engine that is done at once is not reported, and
/// the input is where the message would say.
#[test]
fn a_finished_stream_passes_the_watchdog_and_keeps_its_input() {
    let output = answer("watchdog", b"isready\nposition startpos\ngo depth 1\n");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.first(), Some(&"readyok"));
    assert!(lines.last().unwrap().starts_with("bestmove "));
    assert_eq!(
        std::fs::read(kept_input("watchdog")).unwrap(),
        b"isready\nposition startpos\ngo depth 1\n"
    );
}

/// Counts, from now on, the panics of the engine's threads: the search
/// thread, which is named `search`, and the thread a stream's loop runs on,
/// which has no name. A test's own thread carries the test's name and is
/// left out, so that one test failing an assertion is not counted against
/// the other.
fn count_engine_panics() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if matches!(thread::current().name(), None | Some("search")) {
                PANICS.fetch_add(1, Ordering::SeqCst);
            }
            default_hook(info);
        }));
    });
}

#[test]
fn malformed_input_never_panics_and_is_always_answered() {
    count_engine_panics();

    // Across all streams: what the input reached, so that a generator that
    // stopped producing some kind of line cannot pass unnoticed.
    let mut seen = [0usize; 4];
    let mut cut_characters = 0;
    let (mut moves_played, mut no_move) = (0, 0);
    const REACHED: [&str; 4] = [
        "info depth ",
        "info string position: invalid FEN",
        "info string position: illegal move",
        "info string unknown command",
    ];
    // Not `Rng::stream`: its indices are registered for the engine's own
    // uses. The default generator is what the other randomised tests use.
    let mut states = Rng::default();
    for index in 0..STREAMS {
        let mut generator = Generator {
            rng: Rng::from_state(states.next_u64()),
            cut_characters: 0,
        };
        let mut input = Vec::new();
        let mut model = Model::new();
        let mut readies = 0;
        let mut written = 0;
        while written < LINES {
            let line = generator.line();
            let command = parse(&String::from_utf8_lossy(&line));
            // `quit` would end the stream early; the end is fixed below.
            if command == Command::Quit {
                continue;
            }
            readies += usize::from(command == Command::IsReady);
            model.follow(&command);
            input.extend_from_slice(&line);
            input.push(b'\n');
            written += 1;
        }
        // End any search still running, then ask once more.
        input.extend_from_slice(b"stop\nisready\n");
        readies += 1;

        let output = answer(&index.to_string(), &input);

        assert_eq!(PANICS.load(Ordering::SeqCst), 0, "stream {index} panicked");
        assert!(
            !output.contains("the search ended unexpectedly"),
            "stream {index}: a search thread failed"
        );
        let lines: Vec<&str> = output.lines().collect();
        let answered = lines.iter().filter(|line| **line == "readyok").count();
        assert_eq!(
            answered, readies,
            "stream {index}: every isready is answered"
        );
        let (played, none) = model.check(&index.to_string(), &lines);
        moves_played += played;
        no_move += none;
        assert_eq!(
            lines.last(),
            Some(&"readyok"),
            "stream {index} ends answered"
        );
        for (count, prefix) in seen.iter_mut().zip(REACHED) {
            *count += lines.iter().filter(|line| line.starts_with(prefix)).count();
        }
        cut_characters += generator.cut_characters;
    }
    assert!(
        cut_characters > 0,
        "no command was cut in the middle of a character"
    );
    // Both kinds of answer were checked: real moves, and `0000` from a
    // position without one (one of the listed FENs is a stalemate).
    assert!(moves_played > 100, "only {moves_played} moves were checked");
    assert!(no_move > 0, "no search of a position without a move");
    for (count, prefix) in seen.iter().zip(REACHED) {
        assert!(*count > 0, "no stream produced a line starting {prefix:?}");
    }
}

/// Sessions as a GUI sends them, some lines damaged, long games among
/// them, with every limited search left to run to its limit.
#[test]
fn damaged_sessions_and_long_games_are_played_through() {
    count_engine_panics();
    const SESSION_STREAMS: u64 = 4;
    const REACHED: [&str; 5] = [
        "info string stats depth ",
        "info string + a b c d e f g h",
        "info string total for White: ",
        "info string variety: ",
        "info string position: illegal move",
    ];
    let mut seen = [0usize; 5];
    let (mut moves_played, mut with_chances, mut longest) = (0, 0, 0);
    // The streams of the test above take the first draws; these take the
    // ones after them.
    let mut states = Rng::default();
    for _ in 0..STREAMS {
        states.next_u64();
    }
    for index in 0..SESSION_STREAMS {
        let mut generator = Generator {
            rng: Rng::from_state(states.next_u64()),
            cut_characters: 0,
        };
        let mut input = Vec::new();
        let mut model = Model::new();
        let mut readies = 0;
        for (number, long) in [false, true, false, false, false, false, false, false]
            .into_iter()
            .enumerate()
        {
            let mut session = generator.session(long);
            if number == 0 {
                // One session of each stream is sure to play with the
                // widest `Variety`, at full strength.
                let new_game = session.iter().position(|line| line == "ucinewgame");
                let at = new_game.expect("every session starts a game");
                session.insert(at, "setoption name UCI_LimitStrength value false".into());
                session.insert(at, "setoption name Variety value 100".into());
            }
            for line in session {
                let line = if generator.below(8) == 0 {
                    generator.damaged(&line)
                } else {
                    line.into_bytes()
                };
                if line.contains(&b'\n') {
                    continue;
                }
                let command = parse(&String::from_utf8_lossy(&line));
                if command == Command::Quit {
                    continue;
                }
                readies += usize::from(command == Command::IsReady);
                model.follow(&command);
                longest = longest.max(line.len());
                input.extend_from_slice(&line);
                input.push(b'\n');
            }
        }
        input.extend_from_slice(b"stop\nisready\n");
        readies += 1;

        let name = format!("sessions-{index}");
        let output = watched(&name, &input, true);
        assert_eq!(PANICS.load(Ordering::SeqCst), 0, "stream {name} panicked");
        assert!(
            !output.contains("the search ended unexpectedly"),
            "stream {name}: a search thread failed"
        );
        let lines: Vec<&str> = output.lines().collect();
        let answered = lines.iter().filter(|line| **line == "readyok").count();
        assert_eq!(
            answered, readies,
            "stream {name}: every isready is answered"
        );
        assert_eq!(
            lines.last(),
            Some(&"readyok"),
            "stream {name} ends answered"
        );
        moves_played += model.check(&name, &lines).0;
        // Chances, where shown, are three numbers that add up to 1000.
        for line in lines.iter().filter(|line| line.starts_with("info depth ")) {
            let words: Vec<&str> = line.split(' ').collect();
            if let Some(at) = words.iter().position(|&word| word == "wdl") {
                let sum: u32 = words[at + 1..at + 4]
                    .iter()
                    .map(|word| word.parse::<u32>().unwrap())
                    .sum();
                assert_eq!(sum, 1000, "stream {name}: {line}");
                with_chances += 1;
            }
        }
        for (count, prefix) in seen.iter_mut().zip(REACHED) {
            *count += lines.iter().filter(|line| line.starts_with(prefix)).count();
        }
    }
    for (count, prefix) in seen.iter().zip(REACHED) {
        assert!(*count > 0, "no stream produced a line starting {prefix:?}");
    }
    assert!(moves_played > 100, "only {moves_played} moves were checked");
    assert!(with_chances > 0, "no info line had chances");
    // A game of more than a thousand plies was sent as one line.
    assert!(longest > 5_000, "the longest line had {longest} bytes");
}

/// The same kind of malformed stream given to the real binary over real
/// pipes: what the tests above show of the library holds for the program a
/// GUI starts, which reads bytes from a pipe, writes to one, and is built
/// to abort on a panic. It must read to the end, answer, write nothing to
/// standard error and leave with success.
#[test]
fn the_binary_survives_a_malformed_stream_over_pipes() {
    use std::io::Read;
    use std::process::{Command as Process, Stdio};

    // A generator of its own, after those of the other tests' streams.
    let mut states = Rng::default();
    for _ in 0..STREAMS + 4 {
        states.next_u64();
    }
    let mut generator = Generator {
        rng: Rng::from_state(states.next_u64()),
        cut_characters: 0,
    };
    let mut input = Vec::new();
    let mut model = Model::new();
    let mut readies = 0;
    let mut written = 0;
    while written < 200 {
        let line = generator.line();
        let command = parse(&String::from_utf8_lossy(&line));
        if command == Command::Quit {
            continue;
        }
        readies += usize::from(command == Command::IsReady);
        model.follow(&command);
        input.extend_from_slice(&line);
        input.push(b'\n');
        written += 1;
    }
    input.extend_from_slice(b"stop\nisready\nquit\n");
    readies += 1;
    let kept = kept_input("binary");
    std::fs::write(&kept, &input).expect("the input can be kept");

    let mut child = Process::new(env!("CARGO_BIN_EXE_vyvrael"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the engine binary starts");
    // Each pipe has a thread of its own: writing all the input before
    // reading any output would fill both pipes and stop both sides.
    let mut stdin = child.stdin.take().unwrap();
    let writer = thread::spawn(move || {
        // The engine may leave before taking everything; that is judged
        // by its answers, not here.
        let _ = stdin.write_all(&input);
    });
    let mut stderr = child.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let mut stdout = child.stdout.take().unwrap();
    let (done, finished) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        let _ = done.send(bytes);
    });
    let Ok(bytes) = finished.recv_timeout(WATCHDOG) else {
        let _ = child.kill();
        panic!(
            "the binary was not done after {} s; its input is in {}",
            WATCHDOG.as_secs(),
            kept.display()
        );
    };
    let status = child.wait().expect("the engine can be waited for");
    writer.join().unwrap();
    let errors = errors.join().unwrap();

    assert!(status.success(), "{status}; input in {}", kept.display());
    assert_eq!(errors, "", "the engine wrote to standard error");
    let output = String::from_utf8(bytes).expect("output is text");
    let lines: Vec<&str> = output.lines().collect();
    let answered = lines.iter().filter(|line| **line == "readyok").count();
    assert_eq!(answered, readies, "every isready is answered");
    assert_eq!(lines.last(), Some(&"readyok"), "nothing follows quit");
    let (played, _) = model.check("binary", &lines);
    assert!(played > 10, "only {played} moves were checked");
}
