// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Extended Position Description (EPD): one position to a line, followed by
//! operations that say something about it. Test suites are written in it:
//!
//! ```text
//! 6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+; id "back rank";
//! ```
//!
//! A record is the first four fields of a FEN (placement, side to move,
//! castling rights, en passant square) and then any number of operations,
//! each an opcode, its operand and a semicolon. This module reads a line
//! into the position and its operations; what the operations mean (`bm` is
//! the best move, `am` a move to avoid, `id` a name) is for its users.

use std::fmt;

use crate::board::{FenError, Position};

/// One line of an EPD file, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpdRecord {
    /// The position. EPD has no move counters, so the halfmove clock is 0
    /// and the fullmove number 1, whatever `hmvc` and `fmvn` operations say.
    pub position: Position,
    /// The operations in the order written: opcode and operand. A quoted
    /// operand is kept without its quotes; any other with single spaces
    /// between its words.
    pub operations: Vec<(String, String)>,
}

/// Why a line is not an EPD record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpdError {
    /// Fewer than the four fields of a position.
    TooShort,
    /// The four fields are not a position the engine accepts.
    Position(FenError),
    /// A quoted operand that is never closed.
    OpenQuote,
}

impl fmt::Display for EpdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EpdError::TooShort => write!(f, "fewer than the four fields of a position"),
            EpdError::Position(problem) => write!(f, "{problem}"),
            EpdError::OpenQuote => write!(f, "a quoted operand is not closed"),
        }
    }
}

impl std::error::Error for EpdError {}

impl EpdRecord {
    /// Reads one line.
    ///
    /// Operations are split at semicolons outside quotes, so an operand
    /// such as `"1. e4; then what?"` stays whole. The last semicolon may be
    /// missing, and an operation with no operand is kept with an empty one.
    /// The position is checked as strictly as any FEN (see
    /// [`Position::from_fen`]).
    ///
    /// # Errors
    ///
    /// Returns an [`EpdError`] if the line has no four fields, they are not
    /// an acceptable position, or a quote is left open.
    pub fn parse(line: &str) -> Result<EpdRecord, EpdError> {
        let line = line.trim();
        // The end of the fourth field: four runs of non-space.
        let mut end = 0;
        for _ in 0..4 {
            let rest = &line[end..];
            let start = rest.len() - rest.trim_start().len();
            let word = rest[start..]
                .find(char::is_whitespace)
                .unwrap_or(rest.len() - start);
            if word == 0 {
                return Err(EpdError::TooShort);
            }
            end += start + word;
        }
        let position = Position::from_fen(&line[..end]).map_err(EpdError::Position)?;

        let mut operations = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        for letter in line[end..].chars() {
            match letter {
                '"' => {
                    quoted = !quoted;
                    current.push(letter);
                }
                ';' if !quoted => operations.extend(operation(&std::mem::take(&mut current))),
                _ => current.push(letter),
            }
        }
        if quoted {
            return Err(EpdError::OpenQuote);
        }
        operations.extend(operation(&current));
        Ok(EpdRecord {
            position,
            operations,
        })
    }

    /// The operand of the first operation with this opcode, if there is one.
    #[must_use]
    pub fn operand(&self, opcode: &str) -> Option<&str> {
        self.operations
            .iter()
            .find(|(code, _)| code == opcode)
            .map(|(_, operand)| operand.as_str())
    }

    /// The words of the operand of `opcode`: the moves of `bm` or `am`, as
    /// written. Empty if there is no such operation.
    #[must_use]
    pub fn words(&self, opcode: &str) -> Vec<&str> {
        self.operand(opcode)
            .map(|operand| operand.split_whitespace().collect())
            .unwrap_or_default()
    }
}

/// One operation's text as opcode and operand, or nothing for blank text.
fn operation(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let (opcode, operand) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    if opcode.is_empty() {
        return None;
    }
    let operand = operand.trim();
    let operand = match operand
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inside) => inside.to_owned(),
        None => operand.split_whitespace().collect::<Vec<_>>().join(" "),
    };
    Some((opcode.to_owned(), operand))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_with_its_operations() {
        let record =
            EpdRecord::parse("6k1/5ppp/8/8/8/8/8/R5K1 w - - bm Ra8+; id \"back rank\";").unwrap();
        assert_eq!(
            record.position.to_string(),
            "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1"
        );
        assert_eq!(
            record.operations,
            [
                ("bm".to_owned(), "Ra8+".to_owned()),
                ("id".to_owned(), "back rank".to_owned())
            ]
        );
        assert_eq!(record.operand("id"), Some("back rank"));
        assert_eq!(record.words("bm"), ["Ra8+"]);
        assert_eq!(record.operand("am"), None);
        assert!(record.words("am").is_empty());
    }

    #[test]
    fn spacing_semicolons_and_quotes() {
        // Several moves, odd spacing, no last semicolon, an empty operation.
        let record = EpdRecord::parse(
            "  rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR   w  KQkq -  bm e4   d4  Nf3 ;; am   a4",
        )
        .unwrap();
        assert_eq!(record.words("bm"), ["e4", "d4", "Nf3"]);
        assert_eq!(record.operand("bm"), Some("e4 d4 Nf3"));
        assert_eq!(record.words("am"), ["a4"]);
        assert_eq!(record.operations.len(), 2);
        // A semicolon inside quotes belongs to the operand; an opcode alone
        // has an empty operand; move counters do not reach the position.
        let record = EpdRecord::parse(
            "4k3/8/8/8/8/8/8/4K2R w K - c0 \"castle; or not?\"; hmvc 12; fmvn 40; draw;",
        )
        .unwrap();
        assert_eq!(record.operand("c0"), Some("castle; or not?"));
        assert_eq!(record.operand("hmvc"), Some("12"));
        assert_eq!(record.operand("draw"), Some(""));
        assert_eq!(
            record.position.to_string(),
            "4k3/8/8/8/8/8/8/4K2R w K - 0 1"
        );
        // The position alone is a record with no operations.
        let bare = EpdRecord::parse("4k3/8/8/8/8/8/8/4K2R w K -").unwrap();
        assert!(bare.operations.is_empty());
    }

    #[test]
    fn what_is_not_a_record() {
        assert_eq!(EpdRecord::parse(""), Err(EpdError::TooShort));
        assert_eq!(
            EpdRecord::parse("4k3/8/8/8/8/8/8/4K3 w -"),
            Err(EpdError::TooShort)
        );
        // The position is checked like any FEN: no kings here.
        let no_kings = EpdRecord::parse("8/8/8/8/8/8/8/8 w - - bm e4;");
        assert!(
            matches!(no_kings, Err(EpdError::Position(_))),
            "{no_kings:?}"
        );
        // A full FEN is not EPD: its counters would be read as an opcode,
        // which is harmless.
        let fen = EpdRecord::parse("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(fen.operand("0"), Some("1"));
        assert_eq!(
            EpdRecord::parse("4k3/8/8/8/8/8/8/4K3 w - - id \"open"),
            Err(EpdError::OpenQuote)
        );
        let message = EpdError::OpenQuote.to_string();
        assert_eq!(message, "a quoted operand is not closed");
    }
}
