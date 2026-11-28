// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Turning a line of UCI input into a command.
//!
//! Parsing is kept apart from acting: this file knows the grammar of the
//! protocol and nothing about positions or searching, so every line a GUI
//! can send is tested here without starting an engine.

/// The limits a `go` command sets. Times are in milliseconds, as the
/// protocol sends them. Anything not sent is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GoParams {
    /// White's remaining time.
    pub wtime: Option<u64>,
    /// Black's remaining time.
    pub btime: Option<u64>,
    /// White's increment per move.
    pub winc: Option<u64>,
    /// Black's increment per move.
    pub binc: Option<u64>,
    /// Moves left until the next time control.
    pub movestogo: Option<u32>,
    /// Search this many plies and no deeper.
    pub depth: Option<u32>,
    /// Search about this many positions.
    pub nodes: Option<u64>,
    /// Spend this long on the move.
    pub movetime: Option<u64>,
    /// Search for a mate in this many moves or fewer.
    pub mate: Option<u32>,
    /// Search until told to stop.
    pub infinite: bool,
    /// Search only these moves of the position, as sent (`searchmoves`).
    /// Empty when the parameter was not given: every move is searched.
    pub searchmoves: Vec<String>,
}

/// One command from the GUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `uci`: identify the engine and list its options.
    Uci,
    /// `isready`: answer `readyok`.
    IsReady,
    /// `ucinewgame`: the next position belongs to a new game.
    NewGame,
    /// `position`: set up a position and play moves from it.
    Position {
        /// The FEN to start from, or `None` for the start position.
        fen: Option<String>,
        /// The moves to play from there, as sent.
        moves: Vec<String>,
    },
    /// `go`: start searching.
    Go(GoParams),
    /// `stop`: end the search and report the best move.
    Stop,
    /// `setoption`: change an option.
    SetOption {
        /// The option's name as sent; it may contain spaces.
        name: String,
        /// The value, or `None` if the command had no `value` part.
        value: Option<String>,
    },
    /// `d`: show the current position. Not part of the protocol; a
    /// convenience for a person at a terminal.
    Display,
    /// `eval`: show the static evaluation of the current position. Not part
    /// of the protocol either.
    Eval,
    /// `quit`: leave.
    Quit,
    /// A known command that cannot be understood; the text says why.
    Malformed(String),
    /// A line with no command the engine knows.
    Unknown(String),
    /// A blank line.
    Empty,
}

/// Parses one line of input.
///
/// The protocol asks engines to be forgiving: words before the first known
/// command are skipped (`joho isready` is `isready`), any amount of white
/// space separates words, and `go` parameters the engine does not know are
/// ignored.
///
/// The two commands that are not part of the protocol, `d` and `eval`, are
/// recognised only as the whole line, so that they cannot hide a protocol
/// command on the same line (`d isready` is still `isready`) or be found by
/// accident in the noise before one.
#[must_use]
pub fn parse(line: &str) -> Command {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words[..] {
        [] => return Command::Empty,
        ["d"] => return Command::Display,
        ["eval"] => return Command::Eval,
        _ => {}
    }
    for (index, &word) in words.iter().enumerate() {
        let rest = &words[index + 1..];
        return match word {
            "uci" => Command::Uci,
            "isready" => Command::IsReady,
            "ucinewgame" => Command::NewGame,
            "position" => parse_position(rest),
            "go" => Command::Go(parse_go(rest)),
            "stop" => Command::Stop,
            "setoption" => parse_setoption(rest),
            "quit" => Command::Quit,
            _ => continue,
        };
    }
    Command::Unknown(words.join(" "))
}

fn parse_position(words: &[&str]) -> Command {
    let moves_at = words.iter().position(|&word| word == "moves");
    let (setup, moves) = match moves_at {
        Some(at) => (&words[..at], &words[at + 1..]),
        None => (words, &[][..]),
    };
    let fen = match setup {
        ["startpos"] => None,
        ["fen", fields @ ..] if !fields.is_empty() => Some(fields.join(" ")),
        _ => {
            return Command::Malformed(
                "position: expected `startpos` or `fen <fen>`, then optionally `moves ...`"
                    .to_string(),
            );
        }
    };
    Command::Position {
        fen,
        moves: moves.iter().map(ToString::to_string).collect(),
    }
}

/// The words that begin a parameter of `go`. A list of `searchmoves` ends
/// at the next of them.
const GO_WORDS: [&str; 12] = [
    "searchmoves",
    "ponder",
    "wtime",
    "btime",
    "winc",
    "binc",
    "movestogo",
    "depth",
    "nodes",
    "mate",
    "movetime",
    "infinite",
];

fn parse_go(words: &[&str]) -> GoParams {
    /// A number for a `go` parameter. GUIs send negative clock times when a
    /// flag has fallen; those count as zero.
    fn number(word: Option<&&str>) -> Option<u64> {
        let value = word?.parse::<i64>().ok()?;
        Some(u64::try_from(value).unwrap_or(0))
    }
    let small = |word| number(word).map(|value| u32::try_from(value).unwrap_or(u32::MAX));

    let mut params = GoParams::default();
    let mut index = 0;
    while index < words.len() {
        let value = words.get(index + 1);
        let took_value = match words[index] {
            "wtime" => set(&mut params.wtime, number(value)),
            "btime" => set(&mut params.btime, number(value)),
            "winc" => set(&mut params.winc, number(value)),
            "binc" => set(&mut params.binc, number(value)),
            "movestogo" => set(&mut params.movestogo, small(value)),
            "depth" => set(&mut params.depth, small(value)),
            "nodes" => set(&mut params.nodes, number(value)),
            "movetime" => set(&mut params.movetime, number(value)),
            "mate" => set(&mut params.mate, small(value)),
            "infinite" => {
                params.infinite = true;
                false
            }
            "searchmoves" => {
                // Every word up to the next parameter is one of the moves.
                while let Some(word) = words.get(index + 1)
                    && !GO_WORDS.contains(word)
                {
                    params.searchmoves.push((*word).to_string());
                    index += 1;
                }
                false
            }
            _ => false,
        };
        index += if took_value { 2 } else { 1 };
    }
    params
}

/// Stores `value` in `slot` if there is one and says whether there was.
fn set<T>(slot: &mut Option<T>, value: Option<T>) -> bool {
    let present = value.is_some();
    if present {
        *slot = value;
    }
    present
}

fn parse_setoption(words: &[&str]) -> Command {
    let malformed =
        || Command::Malformed("setoption: expected `name <name> [value <value>]`".to_string());
    let Some((&"name", rest)) = words.split_first() else {
        return malformed();
    };
    let value_at = rest.iter().position(|&word| word == "value");
    let (name, value) = match value_at {
        Some(at) => (&rest[..at], Some(rest[at + 1..].join(" "))),
        None => (rest, None),
    };
    if name.is_empty() {
        return malformed();
    }
    Command::SetOption {
        name: name.join(" "),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(line: &str) -> GoParams {
        match parse(line) {
            Command::Go(params) => params,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn the_simple_commands() {
        assert_eq!(parse("uci"), Command::Uci);
        assert_eq!(parse("isready"), Command::IsReady);
        assert_eq!(parse("ucinewgame"), Command::NewGame);
        assert_eq!(parse("stop"), Command::Stop);
        assert_eq!(parse("quit"), Command::Quit);
    }

    #[test]
    fn white_space_and_leading_noise_are_forgiven() {
        assert_eq!(parse(""), Command::Empty);
        assert_eq!(parse("   \t "), Command::Empty);
        assert_eq!(parse("  isready\r"), Command::IsReady);
        assert_eq!(parse("\tuci  \t"), Command::Uci);
        // The protocol's own example: unknown words before a command.
        assert_eq!(parse("joho isready"), Command::IsReady);
        assert_eq!(parse("xyz 12 quit"), Command::Quit);
    }

    /// `d` and `eval` are commands only when they are the whole line.
    #[test]
    fn display_and_eval_are_whole_lines() {
        assert_eq!(parse("d"), Command::Display);
        assert_eq!(parse("  eval 	"), Command::Eval);
        assert_eq!(parse("d isready"), Command::IsReady);
        assert_eq!(parse("eval stop"), Command::Stop);
        for line in ["hello d", "d d", "eval e2e4", "D", "Eval", "debug eval"] {
            assert_eq!(parse(line), Command::Unknown(line.to_owned()), "{line}");
        }
    }

    #[test]
    fn unknown_lines_are_reported_not_guessed() {
        assert_eq!(parse("hello"), Command::Unknown("hello".to_string()));
        assert_eq!(
            parse("  perft   5 "),
            Command::Unknown("perft 5".to_string())
        );
        // Commands are lower case; this is not `uci`.
        assert_eq!(parse("UCI"), Command::Unknown("UCI".to_string()));
        // `ucinewgame` is one word, not `uci` followed by something.
        assert_eq!(parse("ucinewgame"), Command::NewGame);
    }

    #[test]
    fn position_from_the_start() {
        assert_eq!(
            parse("position startpos"),
            Command::Position {
                fen: None,
                moves: vec![]
            }
        );
        assert_eq!(
            parse("position startpos moves e2e4 e7e5  g1f3"),
            Command::Position {
                fen: None,
                moves: vec!["e2e4".into(), "e7e5".into(), "g1f3".into()],
            }
        );
        // `moves` with nothing after it is an empty list.
        assert_eq!(
            parse("position startpos moves"),
            Command::Position {
                fen: None,
                moves: vec![]
            }
        );
    }

    #[test]
    fn position_from_a_fen() {
        let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
        assert_eq!(
            parse(&format!("position fen {fen}")),
            Command::Position {
                fen: Some(fen.to_string()),
                moves: vec![]
            }
        );
        assert_eq!(
            parse(&format!("position   fen  {fen}   moves b4f4 h4g3")),
            Command::Position {
                fen: Some(fen.to_string()),
                moves: vec!["b4f4".into(), "h4g3".into()],
            }
        );
        // The FEN is passed on as sent; judging it is not the parser's job.
        assert_eq!(
            parse("position fen nonsense"),
            Command::Position {
                fen: Some("nonsense".to_string()),
                moves: vec![]
            }
        );
    }

    #[test]
    fn malformed_position_commands() {
        for line in [
            "position",
            "position moves e2e4",
            "position fen",
            "position fen moves e2e4",
            "position startpos extra",
            "position e2e4",
        ] {
            assert!(
                matches!(parse(line), Command::Malformed(text) if text.starts_with("position")),
                "{line}"
            );
        }
    }

    #[test]
    fn go_with_clocks() {
        assert_eq!(
            go("go wtime 300000 btime 295000 winc 2000 binc 2000"),
            GoParams {
                wtime: Some(300_000),
                btime: Some(295_000),
                winc: Some(2_000),
                binc: Some(2_000),
                ..GoParams::default()
            }
        );
        assert_eq!(
            go("go wtime 60000 btime 60000 movestogo 40"),
            GoParams {
                wtime: Some(60_000),
                btime: Some(60_000),
                movestogo: Some(40),
                ..GoParams::default()
            }
        );
        // A fallen flag is sent as a negative time. Any negative number
        // counts as zero, and what follows it is still read.
        assert_eq!(go("go wtime -150 btime 1000").wtime, Some(0));
        assert_eq!(
            go("go wtime 1000 btime -150 winc -5 binc -7 movestogo -1 depth 4"),
            GoParams {
                wtime: Some(1_000),
                btime: Some(0),
                winc: Some(0),
                binc: Some(0),
                movestogo: Some(0),
                depth: Some(4),
                ..GoParams::default()
            }
        );
        assert_eq!(go("go movetime -20 nodes -3").movetime, Some(0));
        assert_eq!(go("go movetime -20 nodes -3").nodes, Some(0));
    }

    #[test]
    fn go_with_fixed_limits() {
        assert_eq!(go("go depth 6").depth, Some(6));
        assert_eq!(go("go nodes 100000").nodes, Some(100_000));
        assert_eq!(go("go movetime 250").movetime, Some(250));
        assert!(go("go infinite").infinite);
        assert_eq!(go("go"), GoParams::default());
        let both = go("go depth 3 nodes 500");
        assert_eq!((both.depth, both.nodes), (Some(3), Some(500)));
        // Too large for the field: the largest value it holds.
        assert_eq!(go("go depth 99999999999").depth, Some(u32::MAX));
    }

    #[test]
    fn go_reads_the_moves_to_search() {
        let moves = |line: &str| go(line).searchmoves;
        assert!(moves("go depth 4").is_empty());
        assert_eq!(moves("go searchmoves e2e4 d2d4"), ["e2e4", "d2d4"]);
        // The list ends at the next parameter, wherever it stands.
        assert_eq!(
            go("go searchmoves e2e4 d2d4 depth 4"),
            GoParams {
                depth: Some(4),
                searchmoves: vec!["e2e4".to_string(), "d2d4".to_string()],
                ..GoParams::default()
            }
        );
        let mixed = go("go wtime 1000 searchmoves a7a8q infinite");
        assert_eq!(mixed.searchmoves, ["a7a8q"]);
        assert!(mixed.infinite && mixed.wtime == Some(1000));
        // Whatever stands there is kept as sent: the engine checks it
        // against the position. An empty list is no list.
        assert_eq!(moves("go searchmoves e2e4 nonsense"), ["e2e4", "nonsense"]);
        assert!(moves("go searchmoves depth 2").is_empty());
        assert!(moves("go searchmoves").is_empty());
        // Given twice, the lists add up.
        assert_eq!(
            moves("go searchmoves a2a3 depth 1 searchmoves b2b3"),
            ["a2a3", "b2b3"]
        );
    }

    #[test]
    fn go_ignores_what_it_does_not_know() {
        // Not offered: pondering.
        assert_eq!(go("go ponder depth 4").depth, Some(4));
        assert_eq!(go("go ponder"), GoParams::default());
        // A mate search, alone and with other limits.
        assert_eq!(go("go mate 3").mate, Some(3));
        assert_eq!(
            go("go mate 2 movetime 500"),
            GoParams {
                mate: Some(2),
                movetime: Some(500),
                ..GoParams::default()
            }
        );
        assert_eq!(go("go mate"), GoParams::default());
        assert_eq!(go("go mate x depth 2").mate, None);
        // A parameter without a usable value is skipped, and what follows
        // it is still read.
        assert_eq!(go("go depth").depth, None);
        assert_eq!(go("go depth x nodes 9").depth, None);
        assert_eq!(go("go depth x nodes 9").nodes, Some(9));
        assert_eq!(go("go depth infinite"), go("go infinite"));
        // The last value wins.
        assert_eq!(go("go depth 2 depth 5").depth, Some(5));
    }

    #[test]
    fn setoption() {
        assert_eq!(
            parse("setoption name Hash value 64"),
            Command::SetOption {
                name: "Hash".into(),
                value: Some("64".into())
            }
        );
        // Names and values may contain spaces.
        assert_eq!(
            parse("setoption name Some Long Name value a b  c"),
            Command::SetOption {
                name: "Some Long Name".into(),
                value: Some("a b c".into())
            }
        );
        // A button has no value; an empty value is not the same thing.
        assert_eq!(
            parse("setoption name Clear Hash"),
            Command::SetOption {
                name: "Clear Hash".into(),
                value: None
            }
        );
        assert_eq!(
            parse("setoption name SyzygyPath value"),
            Command::SetOption {
                name: "SyzygyPath".into(),
                value: Some(String::new())
            }
        );
        for line in [
            "setoption",
            "setoption Hash 64",
            "setoption name",
            "setoption name value 3",
        ] {
            assert!(
                matches!(parse(line), Command::Malformed(text) if text.starts_with("setoption")),
                "{line}"
            );
        }
    }
}
