// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The chess position: where the pieces are and the game state around them.

use std::fmt;

use super::{Bitboard, CastlingRights, Color, Piece, PieceKind, Square};

/// A chess position.
///
/// Piece placement is stored twice, and the two views are always kept in
/// step: bitboards (one per piece kind and one per colour) answer "where are
/// all the white knights?" in one operation, while the square-indexed
/// `board` array answers "what is on e4?" without scanning bitboards.
///
/// Two positions compare equal only if every field is equal, including the
/// move counters.
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    /// Squares occupied by each piece kind, both colours together.
    pieces: [Bitboard; PieceKind::COUNT],
    /// Squares occupied by each colour.
    colors: [Bitboard; Color::COUNT],
    /// The piece on each square, if any.
    board: [Option<Piece>; Square::COUNT],
    side_to_move: Color,
    castling: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
}

impl Position {
    /// An empty board: no pieces, White to move, no castling rights, no en
    /// passant square, halfmove clock 0 and fullmove number 1.
    ///
    /// This is a starting point for building positions, not a legal chess
    /// position (it has no kings).
    #[must_use]
    pub const fn empty() -> Position {
        Position {
            pieces: [Bitboard::EMPTY; PieceKind::COUNT],
            colors: [Bitboard::EMPTY; Color::COUNT],
            board: [None; Square::COUNT],
            side_to_move: Color::White,
            castling: CastlingRights::NONE,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }

    /// The piece on `square`, if any.
    #[must_use]
    pub const fn piece_at(&self, square: Square) -> Option<Piece> {
        self.board[square.index()]
    }

    /// All squares holding a piece of `kind`, of either colour.
    #[must_use]
    pub const fn pieces(&self, kind: PieceKind) -> Bitboard {
        self.pieces[kind.index()]
    }

    /// All squares holding a piece of `color`.
    #[must_use]
    pub const fn occupied_by(&self, color: Color) -> Bitboard {
        self.colors[color.index()]
    }

    /// All occupied squares.
    #[must_use]
    pub const fn occupied(&self) -> Bitboard {
        Bitboard(self.colors[0].0 | self.colors[1].0)
    }

    /// All squares holding a piece of the given colour and kind.
    #[must_use]
    pub const fn pieces_of(&self, color: Color, kind: PieceKind) -> Bitboard {
        Bitboard(self.colors[color.index()].0 & self.pieces[kind.index()].0)
    }

    /// The square of `color`'s king, or `None` if that side has no king.
    ///
    /// If a side somehow has several kings, the lowest-indexed one is given.
    #[must_use]
    pub const fn king_square(&self, color: Color) -> Option<Square> {
        self.pieces_of(color, PieceKind::King).lsb()
    }

    /// The side whose turn it is.
    #[must_use]
    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    /// The castling rights still available.
    #[must_use]
    pub const fn castling_rights(&self) -> CastlingRights {
        self.castling
    }

    /// The en passant target square: the square a pawn passed over with a
    /// double push on the previous move, if any.
    #[must_use]
    pub const fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Halfmoves since the last pawn move or capture (for the fifty-move
    /// rule).
    #[must_use]
    pub const fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    /// The fullmove number: starts at 1 and increases after Black's move.
    #[must_use]
    pub const fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    /// Places `piece` on `square`.
    ///
    /// # Panics
    ///
    /// Panics if the square is already occupied.
    pub fn put_piece(&mut self, square: Square, piece: Piece) {
        assert!(
            self.board[square.index()].is_none(),
            "put_piece: {square} is already occupied"
        );
        self.board[square.index()] = Some(piece);
        self.pieces[piece.kind.index()].set(square);
        self.colors[piece.color.index()].set(square);
    }

    /// Removes and returns the piece on `square`.
    ///
    /// # Panics
    ///
    /// Panics if the square is empty.
    pub fn remove_piece(&mut self, square: Square) -> Piece {
        let Some(piece) = self.board[square.index()].take() else {
            panic!("remove_piece: {square} is empty");
        };
        self.pieces[piece.kind.index()].clear(square);
        self.colors[piece.color.index()].clear(square);
        piece
    }

    /// Whether the bitboards and the square array describe the same
    /// placement. This must hold after every operation; tests and debug
    /// assertions use it to catch a view going out of step.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        let white = self.colors[Color::White.index()];
        let black = self.colors[Color::Black.index()];
        if !(white & black).is_empty() {
            return false;
        }
        let mut union = Bitboard::EMPTY;
        for kind in PieceKind::ALL {
            if !(union & self.pieces(kind)).is_empty() {
                return false;
            }
            union |= self.pieces(kind);
        }
        if union != self.occupied() {
            return false;
        }
        Square::all().all(|square| match self.piece_at(square) {
            Some(piece) => self.pieces_of(piece.color, piece.kind).contains(square),
            None => !union.contains(square),
        })
    }
}

impl fmt::Debug for Position {
    /// Draws the board (rank 8 at the top, `.` for an empty square) followed
    /// by the rest of the state.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Position {{")?;
        for rank in (0..8).rev() {
            write!(f, "  {} ", rank + 1)?;
            for file in 0..8 {
                let c = self
                    .piece_at(Square::new(file, rank))
                    .map_or('.', Piece::to_char);
                write!(f, " {c}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "     a b c d e f g h")?;
        writeln!(
            f,
            "  side: {:?}, castling: {}, en passant: {}, halfmove: {}, fullmove: {}",
            self.side_to_move,
            self.castling,
            self.en_passant
                .map_or_else(|| "-".to_owned(), |sq| sq.to_string()),
            self.halfmove_clock,
            self.fullmove_number,
        )?;
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE_KING: Piece = Piece::new(Color::White, PieceKind::King);
    const BLACK_QUEEN: Piece = Piece::new(Color::Black, PieceKind::Queen);
    const BLACK_PAWN: Piece = Piece::new(Color::Black, PieceKind::Pawn);

    #[test]
    fn empty_position_defaults() {
        let pos = Position::empty();
        assert!(pos.occupied().is_empty());
        assert!(Square::all().all(|sq| pos.piece_at(sq).is_none()));
        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(pos.castling_rights(), CastlingRights::NONE);
        assert_eq!(pos.en_passant(), None);
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.fullmove_number(), 1);
        assert_eq!(pos.king_square(Color::White), None);
        assert!(pos.is_consistent());
    }

    #[test]
    fn put_updates_every_view() {
        let mut pos = Position::empty();
        pos.put_piece(Square::E1, WHITE_KING);
        pos.put_piece(Square::D8, BLACK_QUEEN);
        pos.put_piece(Square::A7, BLACK_PAWN);

        assert_eq!(pos.piece_at(Square::E1), Some(WHITE_KING));
        assert_eq!(pos.piece_at(Square::D8), Some(BLACK_QUEEN));
        assert_eq!(pos.piece_at(Square::E4), None);
        assert_eq!(pos.king_square(Color::White), Some(Square::E1));
        assert_eq!(pos.king_square(Color::Black), None);
        assert_eq!(
            pos.pieces(PieceKind::King),
            Bitboard::from_square(Square::E1)
        );
        assert_eq!(
            pos.occupied_by(Color::Black),
            Bitboard::EMPTY.with(Square::D8).with(Square::A7)
        );
        assert_eq!(
            pos.pieces_of(Color::Black, PieceKind::Pawn),
            Bitboard::from_square(Square::A7)
        );
        assert!(pos.pieces_of(Color::White, PieceKind::Pawn).is_empty());
        assert_eq!(pos.occupied().count(), 3);
        assert!(pos.is_consistent());
    }

    #[test]
    fn remove_restores_the_empty_position() {
        let mut pos = Position::empty();
        pos.put_piece(Square::E1, WHITE_KING);
        pos.put_piece(Square::D8, BLACK_QUEEN);
        assert_eq!(pos.remove_piece(Square::D8), BLACK_QUEEN);
        assert!(pos.is_consistent());
        assert_eq!(pos.remove_piece(Square::E1), WHITE_KING);
        assert_eq!(pos, Position::empty());
    }

    #[test]
    #[should_panic(expected = "e1 is already occupied")]
    fn put_on_occupied_square_panics() {
        let mut pos = Position::empty();
        pos.put_piece(Square::E1, WHITE_KING);
        pos.put_piece(Square::E1, BLACK_QUEEN);
    }

    #[test]
    #[should_panic(expected = "e4 is empty")]
    fn remove_from_empty_square_panics() {
        Position::empty().remove_piece(Square::E4);
    }

    #[test]
    fn is_consistent_detects_views_out_of_step() {
        let mut pos = Position::empty();
        pos.put_piece(Square::E1, WHITE_KING);

        let mut stray_bit = pos.clone();
        stray_bit.pieces[PieceKind::Rook.index()].set(Square::A1);
        assert!(!stray_bit.is_consistent());

        let mut both_colors = pos.clone();
        both_colors.colors[Color::Black.index()].set(Square::E1);
        assert!(!both_colors.is_consistent());

        let mut two_kinds = pos.clone();
        two_kinds.pieces[PieceKind::Queen.index()].set(Square::E1);
        assert!(!two_kinds.is_consistent());

        let mut ghost = pos.clone();
        ghost.board[Square::H8.index()] = Some(BLACK_QUEEN);
        assert!(!ghost.is_consistent());

        let mut wrong_piece = pos.clone();
        wrong_piece.board[Square::E1.index()] = Some(BLACK_QUEEN);
        assert!(!wrong_piece.is_consistent());
    }

    #[test]
    fn debug_draws_the_board() {
        let mut pos = Position::empty();
        pos.put_piece(Square::E1, WHITE_KING);
        pos.put_piece(Square::A7, BLACK_PAWN);
        let text = format!("{pos:?}");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[2], "  7  p . . . . . . .");
        assert_eq!(lines[8], "  1  . . . . K . . .");
        assert_eq!(lines[9], "     a b c d e f g h");
        assert!(lines[10].contains("side: White, castling: -, en passant: -"));
    }
}
