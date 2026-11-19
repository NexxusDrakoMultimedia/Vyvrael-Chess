// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Making and unmaking moves on a [`Position`].

use super::{CastlingRights, Color, Move, MoveKind, Piece, PieceKind, Position, Square, zobrist};

/// What [`Position::unmake_move`] needs to restore the position that
/// [`Position::make_move`] changed: everything a move destroys that cannot be
/// worked out from the move itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "keep the Undo and pass it to `unmake_move` to take the move back"]
pub struct Undo {
    captured: Option<Piece>,
    castling: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    hash: u64,
}

impl Undo {
    /// The piece the move captured, if any.
    #[must_use]
    pub const fn captured(&self) -> Option<Piece> {
        self.captured
    }
}

/// The square of the pawn captured by an en passant move: on the file the
/// capturing pawn arrives on and the rank it leaves from.
const fn en_passant_victim(from: Square, to: Square) -> Square {
    Square::new(to.file(), from.rank())
}

/// Where the rook starts and ends in a castling move, given the square the
/// king lands on: the g-file means the h-rook goes to the f-file, otherwise
/// the a-rook goes to the d-file.
const fn castle_rook_squares(king_to: Square) -> (Square, Square) {
    let rank = king_to.rank();
    if king_to.file() == 6 {
        (Square::new(7, rank), Square::new(5, rank))
    } else {
        (Square::new(0, rank), Square::new(3, rank))
    }
}

impl Position {
    /// Plays `mv` for the side to move and returns the [`Undo`] that takes it
    /// back.
    ///
    /// The move must be at least pseudo-legal for this position: the origin
    /// square holds a piece of the side to move and the [`MoveKind`] matches
    /// what is on the board. This method does not check legality; generating
    /// only sensible moves is move generation's job.
    ///
    /// # Panics
    ///
    /// Panics if the move does not fit the position at all, for example when
    /// the origin square is empty or a capture has nothing to capture.
    pub fn make_move(&mut self, mv: Move) -> Undo {
        let us = self.side_to_move;
        let (from, to) = (mv.from(), mv.to());
        let mut undo = Undo {
            captured: None,
            castling: self.castling,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            hash: self.hash,
        };

        // Take the old en passant and castling keys out of the hash; the new
        // ones go back in once the move has been played.
        if let Some(file) = self.hashed_en_passant_file() {
            self.hash ^= zobrist::en_passant(file);
        }
        self.hash ^= zobrist::castling(self.castling);
        self.en_passant = None;

        let mover = self.lift(from);
        debug_assert_eq!(mover.color, us, "make_move: {mv} moves the wrong side");
        match mv.kind() {
            MoveKind::Quiet => self.place(to, mover),
            MoveKind::Capture => {
                undo.captured = Some(self.lift(to));
                self.place(to, mover);
            }
            MoveKind::DoublePush => {
                self.place(to, mover);
                // The en passant target is the square the pawn passed over.
                self.en_passant = Some(Square::new(from.file(), (from.rank() + to.rank()) / 2));
            }
            MoveKind::EnPassant => {
                undo.captured = Some(self.lift(en_passant_victim(from, to)));
                self.place(to, mover);
            }
            MoveKind::KingCastle | MoveKind::QueenCastle => {
                self.place(to, mover);
                let (rook_from, rook_to) = castle_rook_squares(to);
                let rook = self.lift(rook_from);
                self.place(rook_to, rook);
            }
            kind => unimplemented!("make_move: {kind:?} arrives in a later chunk"),
        }

        self.halfmove_clock = if mover.kind == PieceKind::Pawn || undo.captured.is_some() {
            0
        } else {
            self.halfmove_clock.saturating_add(1)
        };
        // Covers king moves, rook moves and captures of a rook at home.
        self.castling = self
            .castling
            .without(CastlingRights::revoked_by(from))
            .without(CastlingRights::revoked_by(to));
        self.hash ^= zobrist::castling(self.castling);

        self.side_to_move = us.flip();
        self.hash ^= zobrist::black_to_move();
        if us == Color::Black {
            self.fullmove_number = self.fullmove_number.wrapping_add(1);
        }
        // Must come after the side flips: the rule looks at the new mover.
        if let Some(file) = self.hashed_en_passant_file() {
            self.hash ^= zobrist::en_passant(file);
        }

        debug_assert!(
            self.is_consistent(),
            "make_move: {mv} corrupted the position"
        );
        undo
    }

    /// Takes back `mv`, which must be the most recent move made on this
    /// position, using the [`Undo`] that [`Position::make_move`] returned for
    /// it. The position is restored exactly, including counters and hash.
    ///
    /// # Panics
    ///
    /// May panic if `mv` and `undo` do not belong to the last move made.
    pub fn unmake_move(&mut self, mv: Move, undo: Undo) {
        let us = self.side_to_move.flip();
        let (from, to) = (mv.from(), mv.to());
        self.side_to_move = us;
        if us == Color::Black {
            self.fullmove_number = self.fullmove_number.wrapping_sub(1);
        }

        let mover = self.lift(to);
        match mv.kind() {
            MoveKind::Quiet => self.place(from, mover),
            MoveKind::Capture => {
                self.place(from, mover);
                let captured = undo.captured.expect("unmake_move: capture without a piece");
                self.place(to, captured);
            }
            MoveKind::DoublePush => self.place(from, mover),
            MoveKind::EnPassant => {
                self.place(from, mover);
                let captured = undo.captured.expect("unmake_move: capture without a piece");
                self.place(en_passant_victim(from, to), captured);
            }
            MoveKind::KingCastle | MoveKind::QueenCastle => {
                self.place(from, mover);
                let (rook_from, rook_to) = castle_rook_squares(to);
                let rook = self.lift(rook_to);
                self.place(rook_from, rook);
            }
            kind => unimplemented!("unmake_move: {kind:?} arrives in a later chunk"),
        }

        self.castling = undo.castling;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        // `place`/`lift` moved the hash around; the saved value is exact.
        self.hash = undo.hash;
        debug_assert!(
            self.is_consistent(),
            "unmake_move: {mv} corrupted the position"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::START_FEN;
    use super::*;

    /// Plays `mv` on `before`, checks the result is exactly `after` (every
    /// field, including the hash a fresh parse computes), then takes the
    /// move back and checks the original position is restored exactly.
    /// Returns the piece the move captured.
    #[track_caller]
    fn check(before: &str, mv: Move, after: &str) -> Option<Piece> {
        let original = Position::from_fen(before).unwrap();
        let expected = Position::from_fen(after).unwrap();
        let mut pos = original.clone();

        let undo = pos.make_move(mv);
        assert_eq!(pos.to_fen(), after, "after {mv}");
        assert!(pos.is_consistent(), "inconsistent after {mv}");
        assert_eq!(pos, expected, "after {mv}");

        pos.unmake_move(mv, undo);
        assert!(pos.is_consistent(), "inconsistent after unmaking {mv}");
        assert_eq!(pos, original, "after unmaking {mv}");
        undo.captured()
    }

    fn quiet(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::Quiet)
    }

    fn capture(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::Capture)
    }

    #[test]
    fn quiet_piece_moves_advance_the_clocks() {
        let after_nf3 = "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1";
        let captured = check(START_FEN, quiet(Square::G1, Square::F3), after_nf3);
        assert_eq!(captured, None);
        // Black's move increases the fullmove number.
        check(
            after_nf3,
            quiet(Square::B8, Square::C6),
            "r1bqkbnr/pppppppp/2n5/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 2 2",
        );
    }

    #[test]
    fn pawn_moves_reset_the_halfmove_clock() {
        check(
            "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
            quiet(Square::E7, Square::E6),
            "rnbqkbnr/pppp1ppp/4p3/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2",
        );
    }

    #[test]
    fn captures_remove_the_piece_and_reset_the_halfmove_clock() {
        let captured = check(
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
            capture(Square::E4, Square::D5),
            "rnbqkbnr/ppp1pppp/8/3P4/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2",
        );
        let black_pawn = Piece::new(Color::Black, PieceKind::Pawn);
        assert_eq!(captured, Some(black_pawn));

        check(
            "4k3/8/8/3n4/8/4N3/8/4K3 w - - 17 30",
            capture(Square::E3, Square::D5),
            "4k3/8/8/3N4/8/8/8/4K3 b - - 0 30",
        );
    }

    #[test]
    fn moving_a_rook_or_king_loses_castling_rights() {
        let white = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        check(
            white,
            quiet(Square::A1, Square::B1),
            "r3k2r/8/8/8/8/8/8/1R2K2R b Kkq - 1 1",
        );
        check(
            white,
            quiet(Square::H1, Square::G1),
            "r3k2r/8/8/8/8/8/8/R3K1R1 b Qkq - 1 1",
        );
        check(
            white,
            quiet(Square::E1, Square::E2),
            "r3k2r/8/8/8/8/8/4K3/R6R b kq - 1 1",
        );
        check(
            "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
            quiet(Square::E8, Square::D8),
            "r2k3r/8/8/8/8/8/8/R3K2R w KQ - 1 2",
        );
    }

    #[test]
    fn capturing_a_rook_at_home_loses_both_sides_rights() {
        let black = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1";
        check(
            black,
            capture(Square::A8, Square::A1),
            "4k2r/8/8/8/8/8/8/r3K2R w Kk - 0 2",
        );
        check(
            black,
            capture(Square::H8, Square::H1),
            "r3k3/8/8/8/8/8/8/R3K2r w Qq - 0 2",
        );
    }

    #[test]
    fn any_move_clears_the_en_passant_square() {
        // Not capturable, so it was never in the hash.
        check(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            quiet(Square::G8, Square::F6),
            "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
        );
        // Capturable, so its key must leave the hash.
        check(
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
            quiet(Square::E1, Square::E2),
            "4k3/8/8/3pP3/8/8/4K3/8 b - - 1 1",
        );
    }

    fn double_push(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::DoublePush)
    }

    fn en_passant(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::EnPassant)
    }

    #[test]
    fn double_pushes_set_the_en_passant_square() {
        let after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        check(START_FEN, double_push(Square::E2, Square::E4), after_e4);
        // A second double push replaces the previous en passant square.
        check(
            after_e4,
            double_push(Square::D7, Square::D5),
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
        );
    }

    /// When an enemy pawn can capture, the en passant key enters the hash;
    /// `check` proves it by comparing with a freshly parsed position.
    #[test]
    fn double_push_beside_an_enemy_pawn_is_hashed() {
        let before = "4k3/3p4/8/4P3/8/8/8/4K3 b - - 0 1";
        let after = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2";
        check(before, double_push(Square::D7, Square::D5), after);

        let mut pos = Position::from_fen(before).unwrap();
        let _undo = pos.make_move(double_push(Square::D7, Square::D5));
        assert_eq!(pos.hashed_en_passant_file(), Some(3));
    }

    #[test]
    fn en_passant_captures_remove_the_pawn_beside_the_origin() {
        let black_pawn = Piece::new(Color::Black, PieceKind::Pawn);
        let white_pawn = Piece::new(Color::White, PieceKind::Pawn);

        let captured = check(
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2",
            en_passant(Square::E5, Square::D6),
            "4k3/8/3P4/8/8/8/8/4K3 b - - 0 2",
        );
        assert_eq!(captured, Some(black_pawn));

        // Capturing towards the other side.
        check(
            "4k3/8/8/2Pp4/8/8/8/4K3 w - d6 0 1",
            en_passant(Square::C5, Square::D6),
            "4k3/8/3P4/8/8/8/8/4K3 b - - 0 1",
        );

        let captured = check(
            "4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1",
            en_passant(Square::D4, Square::E3),
            "4k3/8/8/8/8/4p3/8/4K3 w - - 0 2",
        );
        assert_eq!(captured, Some(white_pawn));
    }

    #[test]
    fn en_passant_on_the_edge_files() {
        check(
            "4k3/p7/8/1P6/8/8/8/4K3 b - - 0 1",
            double_push(Square::A7, Square::A5),
            "4k3/8/8/pP6/8/8/8/4K3 w - a6 0 2",
        );
        check(
            "4k3/8/8/pP6/8/8/8/4K3 w - a6 0 2",
            en_passant(Square::B5, Square::A6),
            "4k3/8/P7/8/8/8/8/4K3 b - - 0 2",
        );
        check(
            "4k3/8/8/8/6p1/8/7P/4K3 w - - 0 1",
            double_push(Square::H2, Square::H4),
            "4k3/8/8/8/6pP/8/8/4K3 b - h3 0 1",
        );
        check(
            "4k3/8/8/8/6pP/8/8/4K3 b - h3 0 1",
            en_passant(Square::G4, Square::H3),
            "4k3/8/8/8/8/7p/8/4K3 w - - 0 2",
        );
    }

    #[test]
    fn castling_moves_king_and_rook_and_spends_both_rights() {
        let king_side = |from, to| Move::new(from, to, MoveKind::KingCastle);
        let queen_side = |from, to| Move::new(from, to, MoveKind::QueenCastle);

        let white = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        check(
            white,
            king_side(Square::E1, Square::G1),
            "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1",
        );
        check(
            white,
            queen_side(Square::E1, Square::C1),
            "r3k2r/8/8/8/8/8/8/2KR3R b kq - 1 1",
        );

        let black = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1";
        check(
            black,
            king_side(Square::E8, Square::G8),
            "r4rk1/8/8/8/8/8/8/R3K2R w KQ - 1 2",
        );
        check(
            black,
            queen_side(Square::E8, Square::C8),
            "2kr3r/8/8/8/8/8/8/R3K2R w KQ - 1 2",
        );
    }

    #[test]
    fn castling_keeps_the_other_sides_rights_and_the_clock_running() {
        check(
            "r3k2r/8/8/8/8/8/8/R3K2R w Kq - 5 10",
            Move::new(Square::E1, Square::G1, MoveKind::KingCastle),
            "r3k2r/8/8/8/8/8/8/R4RK1 b q - 6 10",
        );
        // On a crowded board (the "Kiwipete" perft position).
        check(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            Move::new(Square::E1, Square::C1, MoveKind::QueenCastle),
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/2KR3R b kq - 1 1",
        );
    }

    #[test]
    fn halfmove_clock_saturates_and_is_restored() {
        check(
            "4k3/8/8/8/8/8/8/4K3 w - - 65535 9",
            quiet(Square::E1, Square::E2),
            "4k3/8/8/8/8/8/4K3/8 b - - 65535 9",
        );
    }

    #[test]
    fn a_sequence_unwinds_through_every_intermediate_position() {
        let moves = [
            quiet(Square::G1, Square::F3),
            quiet(Square::G8, Square::F6),
            quiet(Square::F3, Square::E5),
            quiet(Square::F6, Square::E4),
            capture(Square::E5, Square::F7),
            capture(Square::E4, Square::F2),
            capture(Square::F7, Square::H8),
            capture(Square::F2, Square::H1),
        ];
        let mut pos = Position::startpos();
        let mut history = Vec::new();
        for mv in moves {
            history.push((pos.clone(), mv, pos.make_move(mv)));
            assert!(pos.is_consistent(), "after {mv}");
        }
        assert_eq!(
            pos.to_fen(),
            "rnbqkb1N/ppppp1pp/8/8/8/8/PPPPP1PP/RNBQKB1n w Qq - 0 5"
        );
        while let Some((snapshot, mv, undo)) = history.pop() {
            pos.unmake_move(mv, undo);
            assert_eq!(pos, snapshot, "unmaking {mv}");
        }
        assert_eq!(pos, Position::startpos());
    }

    #[test]
    #[should_panic(expected = "e4 is empty")]
    fn moving_from_an_empty_square_panics() {
        let _ = Position::startpos().make_move(quiet(Square::E4, Square::E5));
    }
}
