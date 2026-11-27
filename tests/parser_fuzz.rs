// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The parsers that take outside input, fed with generated text.
//!
//! `tests/uci_robustness.rs` sends whole streams through the engine. Here
//! the four parsers behind it are called directly, many thousands of times,
//! with no engine and no search in the way: the UCI command parser, FEN
//! parsing, move text and option values. A panic in any of them fails the
//! test; beyond that, each has properties that must hold whatever the
//! input.
//!
//! The text comes from the in-house generator at its default seed, so every
//! run tries the same inputs.

use vyvrael::board::Position;
use vyvrael::movegen::{find_legal_move, legal_moves};
use vyvrael::rng::{Rng, SEED_MAX};
use vyvrael::search::{ELO_MAX, ELO_MIN, VARIETY_MAX};
use vyvrael::uci::{Command, HASH_MAX, MOVE_OVERHEAD_MAX, Options, THREADS_MAX, parse};

/// Positions to start from and to break.
const FENS: [&str; 7] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
    "4k3/8/8/8/8/8/8/4K2R w K - 99 80",
    "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
];

/// Commands as a GUI sends them, to be damaged.
const COMMANDS: [&str; 14] = [
    "uci",
    "isready",
    "ucinewgame",
    "position startpos moves e2e4 e7e5 g1f3",
    "position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1 moves e1g1",
    "go wtime 300000 btime 300000 winc 2000 binc 2000",
    "go depth 12 nodes 500000 movetime 1500 movestogo 40",
    "go infinite",
    "stop",
    "setoption name Move Overhead value 100",
    "setoption name UCI_LimitStrength value true",
    "d",
    "eval",
    "quit",
];

/// Text a number may be, sensible or not.
const NUMBERS: [&str; 16] = [
    "0",
    "1",
    "-1",
    "+7",
    "007",
    "64",
    "65536",
    "65537",
    "2147483647",
    "2147483648",
    "9223372036854775807",
    "18446744073709551616",
    "1e9",
    "3.5",
    "\u{ff11}\u{ff15}",
    "",
];

struct Generator(Rng);

impl Generator {
    /// The generator for one test: `index` draws on from the default seed's
    /// generator, so the tests do not share a sequence.
    fn new(index: u32) -> Generator {
        let mut states = Rng::default();
        let mut state = states.next_u64();
        for _ in 0..index {
            state = states.next_u64();
        }
        Generator(Rng::from_state(state))
    }

    fn below(&mut self, bound: usize) -> usize {
        self.0.next_below(bound as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    /// `text` with up to `most` bytes changed, removed, repeated or
    /// inserted, read back as text the way the engine's loop reads input.
    fn damaged(&mut self, text: &str, most: usize) -> String {
        const ALPHABET: &[u8] = b"pnbrqkPNBRQK0123456789/ -wbKQkqabcdefgh\t\0\xc3\xa6\xff+=#x";
        let mut bytes = text.as_bytes().to_vec();
        for _ in 0..self.below(most + 1) {
            let letter = ALPHABET[self.below(ALPHABET.len())];
            if bytes.is_empty() {
                bytes.push(letter);
                continue;
            }
            let at = self.below(bytes.len());
            match self.below(4) {
                0 => bytes[at] = letter,
                1 => {
                    bytes.remove(at);
                }
                2 => bytes.insert(at, bytes[at]),
                _ => bytes.insert(at, letter),
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Bytes that need not be text, read back as text.
    fn noise(&mut self, length: usize) -> String {
        let bytes: Vec<u8> = (0..length).map(|_| self.0.next_below(256) as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// The command parser never panics, reads the same line the same way
/// twice, calls only white space empty, and echoes an unknown line whole.
#[test]
fn the_command_parser_takes_anything() {
    let mut generator = Generator::new(0);
    let (mut unknown, mut malformed, mut understood) = (0, 0, 0);
    for round in 0..30_000 {
        let line = match round % 3 {
            0 => {
                let length = generator.below(80);
                generator.noise(length)
            }
            1 => {
                let command = generator.pick(&COMMANDS);
                generator.damaged(command, 6)
            }
            _ => {
                let command = generator.pick(&COMMANDS);
                let number = generator.pick(&NUMBERS);
                // A number where a word belongs, or the other way round.
                let mut words: Vec<&str> = command.split(' ').collect();
                let at = generator.below(words.len());
                words[at] = number;
                words.join(" ")
            }
        };
        let command = parse(&line);
        assert_eq!(parse(&line), command, "{line:?}");
        let words: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(command == Command::Empty, words.is_empty(), "{line:?}");
        match command {
            Command::Unknown(text) => {
                assert_eq!(text, words.join(" "), "{line:?}");
                unknown += 1;
            }
            Command::Malformed(_) => malformed += 1,
            _ => understood += 1,
        }
    }
    // All three kinds came up: the generator is not stuck on one.
    assert!(unknown > 1_000 && malformed > 100 && understood > 5_000);
}

/// FEN parsing never panics. What it accepts is a consistent position that
/// writes itself as a FEN it reads back as the same position, and whose
/// moves can be generated and played.
#[test]
fn the_fen_parser_takes_anything() {
    let mut generator = Generator::new(1);
    let (mut accepted, mut refused) = (0, 0);
    for round in 0..30_000 {
        let fen = generator.pick(&FENS);
        let text = if round % 10 == 0 {
            let length = generator.below(90);
            generator.noise(length)
        } else {
            generator.damaged(fen, 3)
        };
        let Ok(position) = Position::from_fen(&text) else {
            refused += 1;
            continue;
        };
        accepted += 1;
        assert!(position.is_consistent(), "{text:?}");
        let written = position.to_string();
        assert_eq!(
            Position::from_fen(&written).as_ref(),
            Ok(&position),
            "{text:?}"
        );
        for &mv in legal_moves(&position).as_slice() {
            let mut next = position.clone();
            let _undo = next.make_move(mv);
            assert!(next.is_consistent(), "{text:?} {mv}");
        }
    }
    assert!(
        accepted > 2_000,
        "only {accepted} damaged FENs were accepted"
    );
    assert!(refused > 10_000, "only {refused} damaged FENs were refused");
}

/// Move text never panics, and whatever it is, the answer is a legal move
/// of the position or none at all.
#[test]
fn move_text_gives_a_legal_move_or_none() {
    const LETTERS: &[u8] = b"abcdefgh12345678qrbnkQRBN0-xO+=# ";
    let mut generator = Generator::new(2);
    let mut found = 0;
    for fen in FENS {
        let position = Position::from_fen(fen).unwrap();
        let legal = legal_moves(&position);
        let legal = legal.as_slice();
        for round in 0..6_000 {
            let text = match round % 3 {
                0 => {
                    let length = generator.below(7);
                    let bytes: Vec<u8> = (0..length)
                        .map(|_| LETTERS[generator.below(LETTERS.len())])
                        .collect();
                    String::from_utf8(bytes).unwrap()
                }
                1 if !legal.is_empty() => {
                    let mv = legal[generator.below(legal.len())].to_string();
                    generator.damaged(&mv, 2)
                }
                _ => {
                    let length = generator.below(12);
                    generator.noise(length)
                }
            };
            if let Some(mv) = find_legal_move(&position, &text) {
                assert!(legal.contains(&mv), "{fen}: {text:?} gave {mv}");
                found += 1;
            }
        }
    }
    assert!(found > 500, "only {found} texts were read as a move");
}

/// Setting an option never panics. A refusal changes nothing and is
/// reported in one line, and whatever is accepted leaves every value inside
/// its declared range.
#[test]
fn option_values_are_refused_or_within_range() {
    const NAMES: [&str; 13] = [
        "Hash",
        "Threads",
        "Seed",
        "Variety",
        "UCI_LimitStrength",
        "UCI_Elo",
        "Move Overhead",
        "UCI_ShowWDL",
        "SearchStats",
        "move  overhead",
        "Vyvr\u{e6}l",
        "NoSuchOption",
        "",
    ];
    const WORDS: [&str; 6] = ["true", "false", "TRUE", "yes", "on", " true"];
    let mut generator = Generator::new(3);
    let mut options = Options::default();
    let (mut accepted, mut refused) = (0, 0);
    for round in 0..30_000 {
        let name = generator.pick(&NAMES);
        let name = if round % 4 == 0 {
            generator.damaged(name, 2)
        } else {
            name.to_owned()
        };
        let value = match generator.below(5) {
            0 => None,
            1 => Some(generator.pick(&WORDS).to_owned()),
            2 => Some(generator.below(70_000).to_string()),
            3 => Some(generator.pick(&NUMBERS).to_owned()),
            _ => Some(generator.noise(6)),
        };
        // As the command parser hands them over: words with one space
        // between them, so no line break or tab can be inside.
        let words = |text: String| text.split_whitespace().collect::<Vec<_>>().join(" ");
        let (name, value) = (words(name), value.map(words));
        let before = options.clone();
        match options.set(&name, value.as_deref()) {
            Ok(()) => accepted += 1,
            Err(message) => {
                assert_eq!(options, before, "{name:?} {value:?}");
                assert!(!message.contains('\n'), "{message:?}");
                refused += 1;
            }
        }
        assert!((1..=HASH_MAX).contains(&options.hash_mb));
        assert!((1..=THREADS_MAX).contains(&options.threads));
        assert!(options.seed <= SEED_MAX);
        assert!(options.variety <= VARIETY_MAX);
        assert!((ELO_MIN..=ELO_MAX).contains(&options.elo));
        assert!(options.move_overhead_ms <= MOVE_OVERHEAD_MAX);
    }
    assert!(accepted > 1_000 && refused > 10_000, "{accepted} {refused}");
    // The options did move: this was not thirty thousand refusals of the
    // defaults.
    assert_ne!(options, Options::default());
}
