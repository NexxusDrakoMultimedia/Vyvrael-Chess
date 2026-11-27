// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! A position drawn as text, for a person to read.

use super::{Piece, Position, Square};

impl Position {
    /// The board as nine lines of text: ranks 8 down to 1, each starting
    /// with its number, then the file letters. Pieces are their FEN letters
    /// (upper case for White) and an empty square is a `.`.
    ///
    /// ```text
    /// 8 r n b q k b n r
    /// 7 p p p p p p p p
    /// 6 . . . . . . . .
    /// 5 . . . . . . . .
    /// 4 . . . . . . . .
    /// 3 . . . . . . . .
    /// 2 P P P P P P P P
    /// 1 R N B Q K B N R
    /// + a b c d e f g h
    /// ```
    ///
    /// No line is empty or starts with a space, and squares are one space
    /// apart, so the drawing survives being sent a line at a time behind a
    /// prefix (the UCI loop sends it as `info string` lines) and programs
    /// that trim or collapse spaces.
    #[must_use]
    pub fn diagram(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(9);
        for rank in (0..8).rev() {
            let mut line = (rank + 1).to_string();
            for file in 0..8 {
                line.push(' ');
                line.push(
                    self.piece_at(Square::new(file, rank))
                        .map_or('.', Piece::to_char),
                );
            }
            lines.push(line);
        }
        lines.push("+ a b c d e f g h".to_owned());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_position() {
        assert_eq!(
            Position::startpos().diagram(),
            [
                "8 r n b q k b n r",
                "7 p p p p p p p p",
                "6 . . . . . . . .",
                "5 . . . . . . . .",
                "4 . . . . . . . .",
                "3 . . . . . . . .",
                "2 P P P P P P P P",
                "1 R N B Q K B N R",
                "+ a b c d e f g h",
            ]
        );
    }

    /// Files run from a on the left, ranks from 8 at the top: a piece on
    /// g1 is next to the end of the eighth line and one on a8 at the start of
    /// the first.
    #[test]
    fn squares_are_where_a_board_has_them() {
        let pos = Position::from_fen("q6k/8/8/8/3n4/8/8/K5R1 w - - 0 1").unwrap();
        let lines = pos.diagram();
        assert_eq!(lines[0], "8 q . . . . . . k");
        assert_eq!(lines[4], "4 . . . n . . . .");
        assert_eq!(lines[7], "1 K . . . . . R .");
    }

    #[test]
    fn every_line_is_as_wide_as_the_others_and_starts_with_a_mark() {
        let kiwipete = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        for fen in [
            super::super::START_FEN,
            kiwipete,
            "8/8/8/8/8/8/8/K6k w - - 0 1",
        ] {
            let lines = Position::from_fen(fen).unwrap().diagram();
            assert_eq!(lines.len(), 9, "{fen}");
            for line in &lines {
                assert_eq!(line.len(), 17, "{fen}: {line}");
                assert!(!line.starts_with(' ') && !line.contains("  "), "{line}");
            }
        }
    }
}
