// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The chess position: where the pieces are and the game state around them.

use std::fmt;

use super::{Bitboard, CastlingRights, Color, Piece, PieceKind, Square, zobrist};

/// A chess position.
///
/// Piece placement is stored twice, and the two views are always kept in
/// step: bitboards (one per piece kind and one per colour) answer "where are
/// all the white knights?" in one operation, while the square-indexed
/// `board` array answers "what is on e4?" without scanning bitboards.
///
/// The position also carries its Zobrist hash, kept up to date by every
/// operation that changes it (see [`Position::hash`]).
///
/// Two positions compare equal only if every field is equal, including the
/// move counters.
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    /// Squares occupied by each piece kind, both colours together.
    pub(super) pieces: [Bitboard; PieceKind::COUNT],
    /// Squares occupied by each colour.
    pub(super) colors: [Bitboard; Color::COUNT],
    /// The piece on each square, if any.
    pub(super) board: [Option<Piece>; Square::COUNT],
    pub(super) side_to_move: Color,
    pub(super) castling: CastlingRights,
    pub(super) en_passant: Option<Square>,
    pub(super) halfmove_clock: u16,
    pub(super) fullmove_number: u16,
    /// Zobrist hash of the placement, side to move, castling rights and
    /// (when capturable) en passant file. Always equals `compute_hash()`.
    pub(super) hash: u64,
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
            hash: 0,
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

    /// The position's Zobrist hash.
    ///
    /// Two positions have the same hash when they have the same piece
    /// placement, side to move, castling rights and en passant possibility
    /// (barring a 64-bit collision). The halfmove clock and fullmove number
    /// are not part of the hash.
    ///
    /// The en passant file is hashed only when the side to move has a legal
    /// en passant capture, so that a double push that cannot be answered en
    /// passant does not make an otherwise identical position look different.
    /// This matters for detecting repetitions, and it is how the rules of
    /// chess define "the same position".
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }

    /// The en passant file to include in the hash, if any; see
    /// [`Position::hash`] for the rule.
    pub(super) fn hashed_en_passant_file(&self) -> Option<u8> {
        let target = self.en_passant?;
        let target_bb = Bitboard::from_square(target);
        let us = self.side_to_move;
        // The squares our capturing pawns stand on, and the square of the
        // pawn that just double-pushed (the one that would be captured).
        let (capture_from, victim) = match us {
            Color::White => (
                target_bb.south_east() | target_bb.south_west(),
                target_bb.south(),
            ),
            Color::Black => (
                target_bb.north_east() | target_bb.north_west(),
                target_bb.north(),
            ),
        };
        let capturers = capture_from & self.pieces_of(us, PieceKind::Pawn);
        let Some(king) = self.king_square(us) else {
            // Without a king nothing can be illegal (hand-built positions).
            return (!capturers.is_empty()).then(|| target.file());
        };
        // A capture is legal if, with the capturing pawn moved to the target
        // and the captured pawn gone, no remaining enemy piece attacks our
        // king. This covers pins on either pawn, the two-pawns-leave-the-rank
        // case, and being in check from something the capture does not fix.
        let enemies = self.occupied_by(us.flip()) & !victim;
        let legal = capturers.into_iter().any(|from| {
            let occupied = (self.occupied() & !victim).without(from).with(target);
            (self.attackers_to(king, occupied) & enemies).is_empty()
        });
        legal.then(|| target.file())
    }

    /// Computes the Zobrist hash from scratch.
    ///
    /// [`Position::hash`] returns the same value in constant time; this
    /// recomputation exists to initialise it and to verify it in tests.
    #[must_use]
    pub fn compute_hash(&self) -> u64 {
        let mut hash = zobrist::castling(self.castling);
        for square in self.occupied() {
            if let Some(piece) = self.piece_at(square) {
                hash ^= zobrist::piece(piece, square);
            }
        }
        if let Some(file) = self.hashed_en_passant_file() {
            hash ^= zobrist::en_passant(file);
        }
        if self.side_to_move == Color::Black {
            hash ^= zobrist::black_to_move();
        }
        hash
    }

    /// Places `piece` on `square`.
    ///
    /// This is a low-level editing tool for building positions. It keeps the
    /// placement views and the hash correct, but does not re-check that the
    /// position is still valid (for example that castling rights still have
    /// their rook).
    ///
    /// # Panics
    ///
    /// Panics if the square is already occupied.
    pub fn put_piece(&mut self, square: Square, piece: Piece) {
        let en_passant_before = self.hashed_en_passant_file();
        self.place(square, piece);
        self.rehash_en_passant(en_passant_before);
    }

    /// Removes and returns the piece on `square`.
    ///
    /// Like [`Position::put_piece`], this keeps the views and the hash
    /// correct but does not re-validate the position.
    ///
    /// # Panics
    ///
    /// Panics if the square is empty.
    pub fn remove_piece(&mut self, square: Square) -> Piece {
        let en_passant_before = self.hashed_en_passant_file();
        let piece = self.lift(square);
        self.rehash_en_passant(en_passant_before);
        piece
    }

    /// Corrects the hash after an edit that may have changed whether the en
    /// passant square counts for hashing (a pawn appearing or disappearing
    /// beside it).
    fn rehash_en_passant(&mut self, before: Option<u8>) {
        let after = self.hashed_en_passant_file();
        if before != after {
            for file in [before, after].into_iter().flatten() {
                self.hash ^= zobrist::en_passant(file);
            }
        }
    }

    /// Places `piece` on `square`, updating the placement views and the
    /// piece part of the hash only. Callers that can change whether the en
    /// passant square is hashed must account for that themselves.
    ///
    /// # Panics
    ///
    /// Panics if the square is already occupied.
    pub(super) fn place(&mut self, square: Square, piece: Piece) {
        assert!(
            self.board[square.index()].is_none(),
            "put_piece: {square} is already occupied"
        );
        self.board[square.index()] = Some(piece);
        self.pieces[piece.kind.index()].set(square);
        self.colors[piece.color.index()].set(square);
        self.hash ^= zobrist::piece(piece, square);
    }

    /// Removes and returns the piece on `square`, updating the placement
    /// views and the piece part of the hash only; see [`Position::place`].
    ///
    /// # Panics
    ///
    /// Panics if the square is empty.
    pub(super) fn lift(&mut self, square: Square) -> Piece {
        let Some(piece) = self.board[square.index()].take() else {
            panic!("remove_piece: {square} is empty");
        };
        self.pieces[piece.kind.index()].clear(square);
        self.colors[piece.color.index()].clear(square);
        self.hash ^= zobrist::piece(piece, square);
        piece
    }

    /// Whether the bitboards and the square array describe the same
    /// placement and the stored hash matches a from-scratch recomputation.
    /// This must hold after every operation; tests and debug assertions use
    /// it to catch a view going out of step.
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
        if union != self.occupied() || self.hash != self.compute_hash() {
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
            "  side: {:?}, castling: {}, en passant: {}, halfmove: {}, fullmove: {}, hash: {:#018x}",
            self.side_to_move,
            self.castling,
            self.en_passant
                .map_or_else(|| "-".to_owned(), |sq| sq.to_string()),
            self.halfmove_clock,
            self.fullmove_number,
            self.hash,
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

        let mut stale_hash = pos.clone();
        stale_hash.hash ^= 1;
        assert!(!stale_hash.is_consistent());

        let mut unhashed_state = pos.clone();
        unhashed_state.side_to_move = Color::Black;
        assert!(!unhashed_state.is_consistent());
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

#[cfg(test)]
mod hash_tests {
    use super::*;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    #[test]
    fn empty_position_hashes_to_zero() {
        assert_eq!(Position::empty().hash(), 0);
        assert_eq!(Position::empty().compute_hash(), 0);
    }

    /// Golden values computed with an independent implementation.
    #[test]
    fn golden_hashes() {
        assert_eq!(Position::startpos().hash(), 0x477F_24D5_D2C0_357F);
        let after_e4 = fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert_eq!(after_e4.hash(), 0x9903_B24C_0EC1_5C3C);
    }

    #[test]
    fn placement_updates_the_hash_incrementally() {
        let mut pos = Position::empty();
        let knight = Piece::new(Color::White, PieceKind::Knight);
        let king = Piece::new(Color::Black, PieceKind::King);
        pos.put_piece(Square::G1, knight);
        assert_eq!(pos.hash(), zobrist::piece(knight, Square::G1));
        pos.put_piece(Square::E8, king);
        assert_eq!(pos.hash(), pos.compute_hash());
        assert!(pos.is_consistent());
        pos.remove_piece(Square::G1);
        assert_eq!(pos.hash(), zobrist::piece(king, Square::E8));
        pos.remove_piece(Square::E8);
        assert_eq!(pos.hash(), 0);
    }

    #[test]
    fn each_hashed_feature_changes_the_hash_by_its_key() {
        let base = fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        let black = fen("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
        assert_eq!(base.hash() ^ black.hash(), zobrist::black_to_move());

        let fewer_rights = fen("r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1");
        let lost = CastlingRights::WHITE_QUEENSIDE.with(CastlingRights::BLACK_KINGSIDE);
        assert_eq!(base.hash() ^ fewer_rights.hash(), zobrist::castling(lost));

        let moved_rook = fen("r3k2r/8/8/8/8/8/8/1R2K2R w Kkq - 0 1");
        assert_ne!(base.hash(), moved_rook.hash());
    }

    #[test]
    fn counters_are_not_hashed() {
        let a = fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
        let b = fen("4k3/8/8/8/8/8/8/4K3 w - - 57 123");
        assert_ne!(a, b);
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn en_passant_is_hashed_only_when_a_pawn_can_capture() {
        // Black pawn on d5 just double-pushed; the white pawn on e5 attacks d6.
        let capturable = fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
        let without = fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1");
        assert_eq!(capturable.hashed_en_passant_file(), Some(3));
        assert_eq!(capturable.hash() ^ without.hash(), zobrist::en_passant(3));

        // No white pawn beside d5: the en passant square does not count.
        let idle = fen("4k3/8/8/3p4/8/8/4P3/4K3 w - d6 0 1");
        let idle_without = fen("4k3/8/8/3p4/8/8/4P3/4K3 w - - 0 1");
        assert_eq!(idle.en_passant(), Some(Square::D6));
        assert_eq!(idle.hashed_en_passant_file(), None);
        assert_eq!(idle.hash(), idle_without.hash());
        assert_ne!(idle, idle_without);

        // Black to move, with the capturer on either side of the pushed pawn.
        for board in ["4k3/8/8/8/3pP3/8/8/4K3", "4k3/8/8/8/4Pp2/8/8/4K3"] {
            let pos = fen(&format!("{board} b - e3 0 1"));
            assert_eq!(pos.hashed_en_passant_file(), Some(4), "{board}");
        }

        // A pawn on the h-file must not capture on a6 around the board edge.
        let edge = fen("4k3/8/8/p6P/8/8/8/4K3 w - a6 0 1");
        assert_eq!(edge.hashed_en_passant_file(), None);
        // A pawn of the side that just moved does not count as a capturer.
        let own = fen("4k3/8/8/2pp4/8/8/8/4K3 w - d6 0 1");
        assert_eq!(own.hashed_en_passant_file(), None);
    }

    /// Editing pawns beside an en passant square changes whether that
    /// square is hashed; the public editing methods must keep up.
    #[test]
    fn editing_pawns_beside_the_en_passant_square_keeps_the_hash_right() {
        let white_pawn = Piece::new(Color::White, PieceKind::Pawn);
        let mut pos = fen("4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1");
        assert_eq!(pos.hashed_en_passant_file(), None);

        pos.put_piece(Square::E5, white_pawn);
        assert_eq!(pos.hashed_en_passant_file(), Some(3));
        assert_eq!(pos.hash(), pos.compute_hash());
        assert_eq!(pos, fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1"));

        pos.put_piece(Square::C5, white_pawn);
        assert_eq!(pos.hash(), pos.compute_hash());

        pos.remove_piece(Square::E5);
        assert_eq!(pos.hashed_en_passant_file(), Some(3));
        assert_eq!(pos.hash(), pos.compute_hash());

        pos.remove_piece(Square::C5);
        assert_eq!(pos.hashed_en_passant_file(), None);
        assert_eq!(pos.hash(), pos.compute_hash());
        assert_eq!(pos, fen("4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1"));
    }

    /// An en passant capture that would be illegal does not count, so the
    /// position hashes the same as it would without the en passant square.
    #[test]
    fn illegal_en_passant_is_not_hashed() {
        let cases = [
            // Both pawns leave the fifth rank, exposing the king to the rook.
            "7k/8/8/K1pP3r/8/8/8/8 w - c6 0 2",
            // The capturing pawn is pinned on the e-file.
            "4r1k1/8/8/3pP3/8/8/8/4K3 w - d6 0 2",
            // The capturing pawn is pinned on a diagonal it would leave.
            "4k2b/8/8/3pP3/8/8/8/K7 w - d6 0 2",
            // In check from a rook; capturing en passant does not help.
            "6k1/8/8/3pP3/8/8/8/r3K3 w - d6 0 2",
            // The same idea for Black.
            "8/8/8/8/k1Pp3R/8/8/7K b - c3 0 1",
        ];
        for text in cases {
            let pos = fen(text);
            assert!(pos.en_passant().is_some(), "{text}");
            assert_eq!(pos.hashed_en_passant_file(), None, "{text}");
            let mut fields: Vec<&str> = text.split(' ').collect();
            fields[3] = "-";
            assert_eq!(pos.hash(), fen(&fields.join(" ")).hash(), "{text}");
        }
    }

    #[test]
    fn legal_en_passant_is_hashed_even_in_awkward_positions() {
        let cases = [
            // The double-pushed pawn gives check and capturing it is the cure.
            ("4k3/8/8/3pP3/2K5/8/8/8 w - d6 0 2", 3),
            // Two capturers, one pinned: the other can still capture.
            ("4r1k1/8/8/2PpP3/8/8/8/4K3 w - d6 0 2", 3),
            // A pawn pinned on the diagonal it captures along.
            ("1b4k1/8/8/3pP3/8/8/7K/8 w - d6 0 2", 3),
            // A rook on the rank, but another piece still shields the king.
            ("7k/8/8/K1pP2Nr/8/8/8/8 w - c6 0 2", 2),
        ];
        for (text, file) in cases {
            assert_eq!(fen(text).hashed_en_passant_file(), Some(file), "{text}");
        }
    }

    #[test]
    fn parsed_positions_have_a_correct_hash() {
        for text in [
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ] {
            let pos = fen(text);
            assert_eq!(pos.hash(), pos.compute_hash(), "{text}");
            assert!(pos.is_consistent(), "{text}");
        }
    }
}
