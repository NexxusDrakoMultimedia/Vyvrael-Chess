// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The UCI loop under malformed and unexpected input.
//!
//! A seeded generator writes streams of random bytes, scrambled and
//! truncated commands, oversized lines, broken FENs, long move lists, odd
//! option values and commands sent while a search runs. Each stream goes
//! through `uci::run`, and the test checks that nothing panicked on any
//! thread, that every `isready` was answered and that every `go` got
//! exactly one `bestmove`.
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
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vyvrael::board::Position;
use vyvrael::movegen::legal_moves;
use vyvrael::rng::Rng;
use vyvrael::uci::{Command, parse, run};

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
    let kept = kept_input(name);
    std::fs::write(&kept, input).expect("the input can be kept");
    let out = Captured::default();
    let (done, finished) = mpsc::channel();
    let (theirs, copy) = (out.clone(), input.to_vec());
    thread::spawn(move || {
        let read = run(Cursor::new(copy), Box::new(theirs));
        let _ = done.send(read.is_ok());
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

#[test]
fn malformed_input_never_panics_and_is_always_answered() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        PANICS.fetch_add(1, Ordering::SeqCst);
        default_hook(info);
    }));

    // Across all streams: what the input reached, so that a generator that
    // stopped producing some kind of line cannot pass unnoticed.
    let mut seen = [0usize; 4];
    let mut cut_characters = 0;
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
        let (mut readies, mut gos) = (0, 0);
        let mut written = 0;
        while written < LINES {
            let line = generator.line();
            let command = parse(&String::from_utf8_lossy(&line));
            // `quit` would end the stream early; the end is fixed below.
            if command == Command::Quit {
                continue;
            }
            readies += usize::from(command == Command::IsReady);
            gos += usize::from(matches!(command, Command::Go(_)));
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
        let best = lines
            .iter()
            .filter(|line| line.starts_with("bestmove "))
            .count();
        assert_eq!(best, gos, "stream {index}: every go gets one bestmove");
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
    for (count, prefix) in seen.iter().zip(REACHED) {
        assert!(*count > 0, "no stream produced a line starting {prefix:?}");
    }
}
