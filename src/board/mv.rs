// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Move encoding. (The file is `mv.rs` because `move` is a Rust keyword.)

use std::fmt;

use super::{PieceKind, Square};

/// What a move does besides taking a piece from one square to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveKind {
    /// A plain move to an empty square.
    Quiet,
    /// A pawn advancing two squares from its starting rank.
    DoublePush,
    /// Castling on the king's side (the king moves two squares towards h).
    KingCastle,
    /// Castling on the queen's side (the king moves two squares towards a).
    QueenCastle,
    /// A capture of the piece on the destination square.
    Capture,
    /// An en passant capture: the captured pawn is beside the origin square,
    /// not on the destination square.
    EnPassant,
    /// A pawn reaching the last rank and becoming the given piece.
    Promotion(PieceKind),
    /// A pawn capturing onto the last rank and becoming the given piece.
    PromotionCapture(PieceKind),
}

/// A chess move packed into 16 bits.
///
/// Layout: bits 0-5 origin square, bits 6-11 destination square, bits 12-15
/// a flag code for the [`MoveKind`]. Castling is encoded as the king's move
/// (for example `e1` to `g1`).
///
/// A `Move` does not know which piece moves; that comes from the position it
/// is played on. It is the caller's job to pair a move with a position where
/// it makes sense.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(u16);

const FLAG_DOUBLE_PUSH: u16 = 1;
const FLAG_KING_CASTLE: u16 = 2;
const FLAG_QUEEN_CASTLE: u16 = 3;
const FLAG_CAPTURE: u16 = 4;
const FLAG_EN_PASSANT: u16 = 5;
/// Set on all promotions; the low two bits then select the piece.
const FLAG_PROMOTION: u16 = 8;

/// The two-bit promotion piece code.
const fn promotion_code(kind: PieceKind) -> u16 {
    match kind {
        PieceKind::Knight => 0,
        PieceKind::Bishop => 1,
        PieceKind::Rook => 2,
        PieceKind::Queen => 3,
        PieceKind::Pawn | PieceKind::King => {
            panic!("Move::new: cannot promote to a pawn or a king")
        }
    }
}

const fn promotion_piece(code: u16) -> PieceKind {
    match code & 3 {
        0 => PieceKind::Knight,
        1 => PieceKind::Bishop,
        2 => PieceKind::Rook,
        _ => PieceKind::Queen,
    }
}

impl Move {
    /// Creates a move.
    ///
    /// # Panics
    ///
    /// Panics if `kind` is a promotion to a pawn or a king.
    #[must_use]
    pub const fn new(from: Square, to: Square, kind: MoveKind) -> Move {
        let flags = match kind {
            MoveKind::Quiet => 0,
            MoveKind::DoublePush => FLAG_DOUBLE_PUSH,
            MoveKind::KingCastle => FLAG_KING_CASTLE,
            MoveKind::QueenCastle => FLAG_QUEEN_CASTLE,
            MoveKind::Capture => FLAG_CAPTURE,
            MoveKind::EnPassant => FLAG_EN_PASSANT,
            MoveKind::Promotion(piece) => FLAG_PROMOTION | promotion_code(piece),
            MoveKind::PromotionCapture(piece) => {
                FLAG_PROMOTION | FLAG_CAPTURE | promotion_code(piece)
            }
        };
        Move(from.index() as u16 | (to.index() as u16) << 6 | flags << 12)
    }

    /// The square the piece moves from.
    #[must_use]
    pub const fn from(self) -> Square {
        match Square::from_index((self.0 & 63) as u8) {
            Some(square) => square,
            None => unreachable!(),
        }
    }

    /// The square the piece moves to.
    #[must_use]
    pub const fn to(self) -> Square {
        match Square::from_index(((self.0 >> 6) & 63) as u8) {
            Some(square) => square,
            None => unreachable!(),
        }
    }

    /// What kind of move this is.
    #[must_use]
    pub const fn kind(self) -> MoveKind {
        let flags = self.0 >> 12;
        if flags & FLAG_PROMOTION != 0 {
            let piece = promotion_piece(flags);
            return if flags & FLAG_CAPTURE != 0 {
                MoveKind::PromotionCapture(piece)
            } else {
                MoveKind::Promotion(piece)
            };
        }
        match flags {
            0 => MoveKind::Quiet,
            FLAG_DOUBLE_PUSH => MoveKind::DoublePush,
            FLAG_KING_CASTLE => MoveKind::KingCastle,
            FLAG_QUEEN_CASTLE => MoveKind::QueenCastle,
            FLAG_CAPTURE => MoveKind::Capture,
            FLAG_EN_PASSANT => MoveKind::EnPassant,
            // `Move::new` is the only constructor and never produces the
            // two unused flag codes.
            _ => unreachable!(),
        }
    }

    /// Whether the move captures a piece (including en passant and capturing
    /// promotions).
    #[must_use]
    pub const fn is_capture(self) -> bool {
        matches!(
            self.kind(),
            MoveKind::Capture | MoveKind::EnPassant | MoveKind::PromotionCapture(_)
        )
    }

    /// The piece a pawn promotes to, if this move is a promotion.
    #[must_use]
    pub const fn promotion(self) -> Option<PieceKind> {
        match self.kind() {
            MoveKind::Promotion(piece) | MoveKind::PromotionCapture(piece) => Some(piece),
            _ => None,
        }
    }

    /// Whether the move is castling on either side.
    #[must_use]
    pub const fn is_castle(self) -> bool {
        matches!(self.kind(), MoveKind::KingCastle | MoveKind::QueenCastle)
    }
}

impl fmt::Display for Move {
    /// Formats the move in UCI coordinate notation: origin, destination and
    /// a lower-case promotion letter if any, for example `e2e4` or `e7e8q`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from(), self.to())?;
        if let Some(piece) = self.promotion() {
            write!(f, "{}", piece.to_char())?;
        }
        Ok(())
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Move({self}, {:?})", self.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMOTION_PIECES: [PieceKind; 4] = [
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ];

    fn all_kinds() -> Vec<MoveKind> {
        let mut kinds = vec![
            MoveKind::Quiet,
            MoveKind::DoublePush,
            MoveKind::KingCastle,
            MoveKind::QueenCastle,
            MoveKind::Capture,
            MoveKind::EnPassant,
        ];
        kinds.extend(PROMOTION_PIECES.map(MoveKind::Promotion));
        kinds.extend(PROMOTION_PIECES.map(MoveKind::PromotionCapture));
        kinds
    }

    #[test]
    fn move_is_two_bytes() {
        assert_eq!(size_of::<Move>(), 2);
    }

    /// Every (from, to, kind) combination must decode to itself: 64 x 64 x 14.
    #[test]
    fn every_encoding_round_trips() {
        let kinds = all_kinds();
        assert_eq!(kinds.len(), 14);
        for from in Square::all() {
            for to in Square::all() {
                for &kind in &kinds {
                    let mv = Move::new(from, to, kind);
                    assert_eq!((mv.from(), mv.to(), mv.kind()), (from, to, kind));
                }
            }
        }
    }

    #[test]
    fn distinct_kinds_give_distinct_moves() {
        let mut moves: Vec<Move> = all_kinds()
            .into_iter()
            .map(|kind| Move::new(Square::E2, Square::E4, kind))
            .collect();
        moves.sort_unstable_by_key(|mv| mv.0);
        moves.dedup();
        assert_eq!(moves.len(), 14);
    }

    #[test]
    fn predicates() {
        for kind in all_kinds() {
            let mv = Move::new(Square::A7, Square::B8, kind);
            let capture = matches!(
                kind,
                MoveKind::Capture | MoveKind::EnPassant | MoveKind::PromotionCapture(_)
            );
            let promotion = match kind {
                MoveKind::Promotion(piece) | MoveKind::PromotionCapture(piece) => Some(piece),
                _ => None,
            };
            let castle = matches!(kind, MoveKind::KingCastle | MoveKind::QueenCastle);
            assert_eq!(mv.is_capture(), capture, "{kind:?}");
            assert_eq!(mv.promotion(), promotion, "{kind:?}");
            assert_eq!(mv.is_castle(), castle, "{kind:?}");
        }
    }

    #[test]
    fn display_is_uci_coordinate_notation() {
        let quiet = Move::new(Square::E2, Square::E4, MoveKind::DoublePush);
        assert_eq!(quiet.to_string(), "e2e4");
        let castle = Move::new(Square::E1, Square::G1, MoveKind::KingCastle);
        assert_eq!(castle.to_string(), "e1g1");
        let letters = ["n", "b", "r", "q"];
        for (piece, letter) in PROMOTION_PIECES.into_iter().zip(letters) {
            let push = Move::new(Square::E7, Square::E8, MoveKind::Promotion(piece));
            assert_eq!(push.to_string(), format!("e7e8{letter}"));
            let take = Move::new(Square::B2, Square::A1, MoveKind::PromotionCapture(piece));
            assert_eq!(take.to_string(), format!("b2a1{letter}"));
        }
        assert_eq!(format!("{quiet:?}"), "Move(e2e4, DoublePush)");
    }

    #[test]
    #[should_panic(expected = "cannot promote")]
    fn promoting_to_a_king_panics() {
        let _ = Move::new(Square::E7, Square::E8, MoveKind::Promotion(PieceKind::King));
    }

    #[test]
    #[should_panic(expected = "cannot promote")]
    fn capture_promoting_to_a_pawn_panics() {
        let kind = MoveKind::PromotionCapture(PieceKind::Pawn);
        let _ = Move::new(Square::E7, Square::D8, kind);
    }
}
