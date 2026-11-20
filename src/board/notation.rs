// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Reading moves written in UCI coordinate notation.

use std::fmt;

use super::{Color, Move, MoveKind, PieceKind, Position, Square};

/// Why a move in coordinate notation could not be read for a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMoveError {
    /// The text is not two squares and an optional promotion letter
    /// (`n`, `b`, `r` or `q`).
    Syntax,
    /// There is no piece on the origin square.
    NoPiece,
    /// The piece on the origin square belongs to the side not to move.
    WrongSide,
    /// The destination square holds a piece of the side to move.
    OwnPieceOnDestination,
    /// A pawn reaches the last rank without a promotion letter, or a
    /// promotion letter is given for a move that is not a promotion.
    Promotion,
    /// A pawn moves diagonally to an empty square that is not the en passant
    /// square.
    PawnCapture,
}

impl fmt::Display for ParseMoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseMoveError::Syntax => "expected a move such as e2e4 or e7e8q",
            ParseMoveError::NoPiece => "no piece on the origin square",
            ParseMoveError::WrongSide => "the piece on the origin square is not to move",
            ParseMoveError::OwnPieceOnDestination => "the destination holds a piece of the mover",
            ParseMoveError::Promotion => "missing or unexpected promotion piece",
            ParseMoveError::PawnCapture => "a pawn moves diagonally only to capture",
        })
    }
}

impl std::error::Error for ParseMoveError {}

impl Position {
    /// Reads a move in UCI coordinate notation (`e2e4`, `e7e8q`, castling as
    /// the king's move `e1g1`) and works out its [`MoveKind`] from this
    /// position.
    ///
    /// A [`Move`] carries its kind, but the notation does not, so the board
    /// is needed to tell a capture from a quiet move, a double push from a
    /// single one, castling from a king step, and en passant from an
    /// ordinary pawn capture.
    ///
    /// This checks only what is needed to choose the kind. It does **not**
    /// check that the piece can move that way or that the move is legal; a
    /// rook "moving" like a knight is accepted as a quiet move. It is meant
    /// for trusted input such as recorded games. **For input that is not
    /// trusted (a GUI, a file, a user), use `movegen::find_legal_move`
    /// instead**, which only ever returns a legal move.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseMoveError`] if the text is malformed or the move
    /// cannot be given a sensible kind on this position.
    pub fn parse_move(&self, text: &str) -> Result<Move, ParseMoveError> {
        if !text.is_ascii() || !(4..=5).contains(&text.len()) {
            return Err(ParseMoveError::Syntax);
        }
        let square = |range| -> Result<Square, ParseMoveError> {
            text.get(range)
                .and_then(|s: &str| s.parse().ok())
                .ok_or(ParseMoveError::Syntax)
        };
        let (from, to) = (square(0..2)?, square(2..4)?);
        let promotion = match text.as_bytes().get(4) {
            None => None,
            Some(b'n') => Some(PieceKind::Knight),
            Some(b'b') => Some(PieceKind::Bishop),
            Some(b'r') => Some(PieceKind::Rook),
            Some(b'q') => Some(PieceKind::Queen),
            Some(_) => return Err(ParseMoveError::Syntax),
        };

        let us = self.side_to_move();
        let mover = self.piece_at(from).ok_or(ParseMoveError::NoPiece)?;
        if mover.color != us {
            return Err(ParseMoveError::WrongSide);
        }
        let target = self.piece_at(to);
        if target.is_some_and(|piece| piece.color == us) {
            return Err(ParseMoveError::OwnPieceOnDestination);
        }
        let is_capture = target.is_some();
        let king_home = match us {
            Color::White => Square::E1,
            Color::Black => Square::E8,
        };

        let kind = if mover.kind == PieceKind::Pawn {
            let reaches_last_rank = to.rank() == 0 || to.rank() == 7;
            match (promotion, reaches_last_rank) {
                (Some(piece), true) if is_capture => MoveKind::PromotionCapture(piece),
                (Some(piece), true) => MoveKind::Promotion(piece),
                (Some(_), false) | (None, true) => return Err(ParseMoveError::Promotion),
                (None, false) if is_capture => MoveKind::Capture,
                (None, false) if from.file() != to.file() => {
                    if self.en_passant() == Some(to) {
                        MoveKind::EnPassant
                    } else {
                        return Err(ParseMoveError::PawnCapture);
                    }
                }
                (None, false) if from.rank().abs_diff(to.rank()) == 2 => MoveKind::DoublePush,
                (None, false) => MoveKind::Quiet,
            }
        } else if promotion.is_some() {
            return Err(ParseMoveError::Promotion);
        } else if is_capture {
            MoveKind::Capture
        } else if mover.kind == PieceKind::King
            && from == king_home
            && from.rank() == to.rank()
            && from.file().abs_diff(to.file()) == 2
        {
            // A king never steps two files, so from its home square this is
            // castling.
            if to.file() == 6 {
                MoveKind::KingCastle
            } else {
                MoveKind::QueenCastle
            }
        } else {
            MoveKind::Quiet
        };

        Ok(Move::new(from, to, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    #[track_caller]
    fn kind_of(pos: &Position, text: &str) -> MoveKind {
        let mv = pos.parse_move(text).unwrap();
        assert_eq!(mv.to_string(), text, "display must round-trip");
        mv.kind()
    }

    #[test]
    fn pawn_pushes() {
        let pos = Position::startpos();
        assert_eq!(kind_of(&pos, "e2e4"), MoveKind::DoublePush);
        assert_eq!(kind_of(&pos, "e2e3"), MoveKind::Quiet);
        let black = fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert_eq!(kind_of(&black, "d7d5"), MoveKind::DoublePush);
        assert_eq!(kind_of(&black, "d7d6"), MoveKind::Quiet);
    }

    #[test]
    fn piece_moves_and_captures() {
        let pos = fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2");
        assert_eq!(kind_of(&pos, "g1f3"), MoveKind::Quiet);
        assert_eq!(kind_of(&pos, "e4d5"), MoveKind::Capture);
        assert_eq!(kind_of(&pos, "e4e5"), MoveKind::Quiet);
        let pieces = fen("4k3/8/8/3n4/8/4N3/8/4K3 w - - 0 1");
        assert_eq!(kind_of(&pieces, "e3d5"), MoveKind::Capture);
    }

    #[test]
    fn en_passant_needs_the_en_passant_square() {
        let pos = fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2");
        assert_eq!(kind_of(&pos, "e5d6"), MoveKind::EnPassant);
        assert_eq!(kind_of(&pos, "e5e6"), MoveKind::Quiet);
        assert_eq!(pos.parse_move("e5f6"), Err(ParseMoveError::PawnCapture));

        let no_target = fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 2");
        assert_eq!(
            no_target.parse_move("e5d6"),
            Err(ParseMoveError::PawnCapture)
        );

        let black = fen("4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1");
        assert_eq!(kind_of(&black, "d4e3"), MoveKind::EnPassant);
    }

    #[test]
    fn castling_is_a_two_file_king_move_from_its_home_square() {
        let white = fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        assert_eq!(kind_of(&white, "e1g1"), MoveKind::KingCastle);
        assert_eq!(kind_of(&white, "e1c1"), MoveKind::QueenCastle);
        assert_eq!(kind_of(&white, "e1f1"), MoveKind::Quiet);
        assert_eq!(kind_of(&white, "e1d2"), MoveKind::Quiet);
        let black = fen("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
        assert_eq!(kind_of(&black, "e8g8"), MoveKind::KingCastle);
        assert_eq!(kind_of(&black, "e8c8"), MoveKind::QueenCastle);

        // Another piece making the same journey is not castling.
        let queen = fen("7k/8/8/8/8/8/8/K3Q3 w - - 0 1");
        assert_eq!(kind_of(&queen, "e1g1"), MoveKind::Quiet);
        // Nor is a king away from its home square, or on the enemy's.
        let wandering = fen("8/8/8/4k3/8/8/8/4K3 b - - 0 1");
        assert_eq!(kind_of(&wandering, "e5g5"), MoveKind::Quiet);
        let swapped = fen("4K3/8/8/8/8/8/8/4k3 w - - 0 1");
        assert_eq!(kind_of(&swapped, "e8g8"), MoveKind::Quiet);
    }

    #[test]
    fn promotions() {
        let pos = fen("1n2k3/P7/8/8/8/8/7p/4K1N1 w - - 0 1");
        let pieces = [
            ('n', PieceKind::Knight),
            ('b', PieceKind::Bishop),
            ('r', PieceKind::Rook),
            ('q', PieceKind::Queen),
        ];
        for (letter, piece) in pieces {
            let push = format!("a7a8{letter}");
            assert_eq!(kind_of(&pos, &push), MoveKind::Promotion(piece));
            let take = format!("a7b8{letter}");
            assert_eq!(kind_of(&pos, &take), MoveKind::PromotionCapture(piece));
        }
        let black = fen("1n2k3/P7/8/8/8/8/7p/4K1N1 b - - 0 1");
        let queen = MoveKind::Promotion(PieceKind::Queen);
        assert_eq!(kind_of(&black, "h2h1q"), queen);
        let knight = MoveKind::PromotionCapture(PieceKind::Knight);
        assert_eq!(kind_of(&black, "h2g1n"), knight);
    }

    #[test]
    fn promotion_letter_must_match_the_move() {
        let pos = fen("1n2k3/P7/8/8/8/8/4P3/4K3 w - - 0 1");
        for bad in ["a7a8", "a7b8", "e2e4q", "e2e3n", "e1d1q"] {
            assert_eq!(pos.parse_move(bad), Err(ParseMoveError::Promotion), "{bad}");
        }
    }

    #[test]
    fn rejects_malformed_text() {
        let pos = Position::startpos();
        for bad in [
            "", "e2", "e2e", "e2e4qq", "e2e9", "i2e4", "E2E4", "e2e4k", "e2e4p", "e2e4Q", "e2-e4",
            "é2e4", " e2e4", "0000",
        ] {
            assert_eq!(pos.parse_move(bad), Err(ParseMoveError::Syntax), "{bad:?}");
        }
    }

    #[test]
    fn rejects_moves_that_do_not_fit_the_position() {
        let pos = Position::startpos();
        assert_eq!(pos.parse_move("e4e5"), Err(ParseMoveError::NoPiece));
        assert_eq!(pos.parse_move("e7e5"), Err(ParseMoveError::WrongSide));
        assert_eq!(
            pos.parse_move("d1d2"),
            Err(ParseMoveError::OwnPieceOnDestination)
        );
        assert!(!ParseMoveError::WrongSide.to_string().is_empty());
    }

    #[test]
    fn does_not_check_how_pieces_move() {
        // Documented limitation: geometry and legality are not checked.
        let pos = Position::startpos();
        assert_eq!(kind_of(&pos, "a1a5"), MoveKind::Quiet);
    }
}
