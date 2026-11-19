// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Forsyth-Edwards Notation (FEN): reading and writing positions as text.

use std::fmt;
use std::str::FromStr;

use super::{Bitboard, CastlingRights, Color, Piece, PieceKind, Position, Square};

/// The FEN of the standard chess starting position.
pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Why a FEN string was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenError {
    /// The string does not have 4 or 6 whitespace-separated fields.
    FieldCount {
        /// How many fields were found.
        found: usize,
    },
    /// The piece placement does not have 8 ranks separated by `/`.
    RankCount {
        /// How many ranks were found.
        found: usize,
    },
    /// A rank of the piece placement does not describe exactly 8 squares.
    RankLength {
        /// The rank number, 1 to 8.
        rank: u8,
    },
    /// Two digits in a row in the piece placement (for example `44`).
    ConsecutiveDigits {
        /// The rank number, 1 to 8.
        rank: u8,
    },
    /// A character in the piece placement is neither a piece nor a digit 1-8.
    InvalidPieceChar(char),
    /// The side to move is not `w` or `b`.
    InvalidSideToMove,
    /// The castling field is not `-` or distinct letters from `KQkq`.
    InvalidCastling,
    /// The en passant field is not `-` or a square.
    InvalidEnPassant,
    /// The halfmove clock is not a number that fits in 16 bits.
    InvalidHalfmoveClock,
    /// The fullmove number is not a number that fits in 16 bits.
    InvalidFullmoveNumber,
    /// A side does not have exactly one king.
    KingCount {
        /// The side with the wrong number of kings.
        color: Color,
        /// How many kings it has.
        found: u32,
    },
    /// A pawn stands on the first or eighth rank.
    PawnOnBackRank,
    /// A castling right is claimed, but the king or the rook it needs is not
    /// on its home square.
    CastlingWithoutPieces,
    /// The en passant square does not match a double pawn push by the side
    /// that just moved.
    EnPassantImpossible,
    /// The side not to move is in check, so the side to move could capture
    /// its king. No legal game reaches such a position.
    OpponentInCheck,
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FenError::FieldCount { found } => {
                write!(f, "expected 4 or 6 fields, found {found}")
            }
            FenError::RankCount { found } => {
                write!(f, "expected 8 ranks in the piece placement, found {found}")
            }
            FenError::RankLength { rank } => {
                write!(f, "rank {rank} does not describe exactly 8 squares")
            }
            FenError::ConsecutiveDigits { rank } => {
                write!(f, "rank {rank} has two digits in a row")
            }
            FenError::InvalidPieceChar(c) => {
                write!(f, "invalid character {c:?} in the piece placement")
            }
            FenError::InvalidSideToMove => f.write_str("side to move must be 'w' or 'b'"),
            FenError::InvalidCastling => f.write_str("invalid castling rights field"),
            FenError::InvalidEnPassant => f.write_str("invalid en passant field"),
            FenError::InvalidHalfmoveClock => f.write_str("invalid halfmove clock"),
            FenError::InvalidFullmoveNumber => f.write_str("invalid fullmove number"),
            FenError::KingCount { color, found } => {
                write!(f, "{color:?} must have exactly one king, found {found}")
            }
            FenError::PawnOnBackRank => f.write_str("a pawn is on the first or eighth rank"),
            FenError::CastlingWithoutPieces => {
                f.write_str("castling right without king and rook on their home squares")
            }
            FenError::EnPassantImpossible => {
                f.write_str("en passant square does not match a double pawn push")
            }
            FenError::OpponentInCheck => f.write_str("the side not to move is in check"),
        }
    }
}

impl std::error::Error for FenError {}

/// Parses an unsigned decimal number made only of ASCII digits.
///
/// Stricter than `str::parse`, which would also accept a leading `+`.
fn parse_counter(field: &str) -> Option<u16> {
    if field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    field.parse().ok()
}

/// Reads the piece placement field into `pos`.
fn parse_placement(pos: &mut Position, placement: &str) -> Result<(), FenError> {
    let ranks: Vec<&str> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::RankCount { found: ranks.len() });
    }
    // FEN lists rank 8 first.
    for (rank, text) in (0..8u8).rev().zip(ranks) {
        let mut file = 0u8;
        let mut after_digit = false;
        for c in text.chars() {
            if let Some(skip) = c.to_digit(10).filter(|d| (1..=8).contains(d)) {
                if after_digit {
                    return Err(FenError::ConsecutiveDigits { rank: rank + 1 });
                }
                after_digit = true;
                file += skip as u8;
            } else {
                let piece = Piece::from_char(c).ok_or(FenError::InvalidPieceChar(c))?;
                if file >= 8 {
                    return Err(FenError::RankLength { rank: rank + 1 });
                }
                pos.place(Square::new(file, rank), piece);
                after_digit = false;
                file += 1;
            }
            if file > 8 {
                return Err(FenError::RankLength { rank: rank + 1 });
            }
        }
        if file != 8 {
            return Err(FenError::RankLength { rank: rank + 1 });
        }
    }
    Ok(())
}

/// Checks that a syntactically valid position is one the rest of the engine
/// can rely on.
fn validate(pos: &Position) -> Result<(), FenError> {
    for color in Color::ALL {
        let found = pos.pieces_of(color, PieceKind::King).count();
        if found != 1 {
            return Err(FenError::KingCount { color, found });
        }
    }

    let back_ranks = Bitboard::RANK_1 | Bitboard::RANK_8;
    if !(pos.pieces(PieceKind::Pawn) & back_ranks).is_empty() {
        return Err(FenError::PawnOnBackRank);
    }

    let holds = |square, color, kind| pos.piece_at(square) == Some(Piece::new(color, kind));
    for color in Color::ALL {
        let (king, kingside_rook, queenside_rook) = match color {
            Color::White => (Square::E1, Square::H1, Square::A1),
            Color::Black => (Square::E8, Square::H8, Square::A8),
        };
        let sides = [
            (CastlingRights::kingside(color), kingside_rook),
            (CastlingRights::queenside(color), queenside_rook),
        ];
        for (right, rook) in sides {
            if pos.castling.contains(right)
                && !(holds(king, color, PieceKind::King) && holds(rook, color, PieceKind::Rook))
            {
                return Err(FenError::CastlingWithoutPieces);
            }
        }
    }

    if let Some(target) = pos.en_passant {
        // The side that is NOT to move has just played a double push: its
        // pawn passed over `target`, now stands one step beyond it, and the
        // pawn's starting square behind `target` is empty.
        let pusher = pos.side_to_move.flip();
        let (target_rank, forward) = match pusher {
            Color::White => (2, 1),
            Color::Black => (5, -1),
        };
        let pawn_now = target.offset(0, forward);
        let pawn_start = target.offset(0, -forward);
        let plausible = target.rank() == target_rank
            && pos.piece_at(target).is_none()
            && pawn_now.is_some_and(|sq| holds(sq, pusher, PieceKind::Pawn))
            && pawn_start.is_some_and(|sq| pos.piece_at(sq).is_none());
        if !plausible {
            return Err(FenError::EnPassantImpossible);
        }
    }

    // Last, because it relies on the checks above (exactly one king each).
    if pos.can_capture_king() {
        return Err(FenError::OpponentInCheck);
    }

    Ok(())
}

impl Position {
    /// The standard chess starting position.
    #[must_use]
    pub fn startpos() -> Position {
        Position::from_fen(START_FEN).expect("START_FEN is valid")
    }

    /// Parses a position from a FEN string.
    ///
    /// Fields are separated by any amount of whitespace. All six fields may
    /// be given, or only the first four (as in EPD), in which case the
    /// halfmove clock is 0 and the fullmove number is 1.
    ///
    /// Beyond syntax, the position must have exactly one king per side, no
    /// pawns on the first or eighth rank, castling rights only where king
    /// and rook are on their home squares, an en passant square only where
    /// the opponent's pawn could just have made a double push, and the side
    /// not to move must not be in check.
    ///
    /// # Errors
    ///
    /// Returns a [`FenError`] describing the first problem found.
    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() != 4 && fields.len() != 6 {
            return Err(FenError::FieldCount {
                found: fields.len(),
            });
        }

        let mut pos = Position::empty();
        parse_placement(&mut pos, fields[0])?;

        let mut side = fields[1].chars();
        pos.side_to_move = match (side.next().and_then(Color::from_char), side.next()) {
            (Some(color), None) => color,
            _ => return Err(FenError::InvalidSideToMove),
        };

        pos.castling = fields[2].parse().map_err(|_| FenError::InvalidCastling)?;

        pos.en_passant = match fields[3] {
            "-" => None,
            square => Some(square.parse().map_err(|_| FenError::InvalidEnPassant)?),
        };

        if fields.len() == 6 {
            pos.halfmove_clock = parse_counter(fields[4]).ok_or(FenError::InvalidHalfmoveClock)?;
            pos.fullmove_number =
                parse_counter(fields[5]).ok_or(FenError::InvalidFullmoveNumber)?;
        }

        // Pieces were hashed as they were placed; fold in the state fields.
        pos.hash = pos.compute_hash();
        validate(&pos)?;
        Ok(pos)
    }

    /// Serialises the position as a six-field FEN string.
    ///
    /// The output is canonical: single spaces, castling letters in `KQkq`
    /// order. For a position that [`Position::from_fen`] accepts, parsing
    /// the result gives back an equal position. A hand-built position (for
    /// example [`Position::empty`]) may serialise to text that `from_fen`
    /// rejects.
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Position {
    /// Formats the position as FEN; see [`Position::to_fen`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                match self.piece_at(Square::new(file, rank)) {
                    Some(piece) => {
                        if empty > 0 {
                            write!(f, "{empty}")?;
                            empty = 0;
                        }
                        write!(f, "{}", piece.to_char())?;
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                write!(f, "{empty}")?;
            }
            if rank > 0 {
                f.write_str("/")?;
            }
        }
        write!(f, " {} {} ", self.side_to_move.to_char(), self.castling)?;
        match self.en_passant {
            Some(square) => write!(f, "{square}")?,
            None => f.write_str("-")?,
        }
        write!(f, " {} {}", self.halfmove_clock, self.fullmove_number)
    }
}

impl FromStr for Position {
    type Err = FenError;

    /// Parses a FEN string; see [`Position::from_fen`].
    fn from_str(s: &str) -> Result<Position, FenError> {
        Position::from_fen(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(fen: &str) -> FenError {
        Position::from_fen(fen).expect_err(fen)
    }

    #[test]
    fn startpos_fields() {
        let pos = Position::startpos();
        assert!(pos.is_consistent());
        assert_eq!(pos.occupied().count(), 32);
        assert_eq!(pos.occupied_by(Color::White).0, 0x0000_0000_0000_FFFF);
        assert_eq!(pos.occupied_by(Color::Black).0, 0xFFFF_0000_0000_0000);
        assert_eq!(pos.pieces(PieceKind::Pawn).0, 0x00FF_0000_0000_FF00);
        assert_eq!(pos.pieces(PieceKind::Knight).0, 0x4200_0000_0000_0042);
        assert_eq!(pos.pieces(PieceKind::Bishop).0, 0x2400_0000_0000_0024);
        assert_eq!(pos.pieces(PieceKind::Rook).0, 0x8100_0000_0000_0081);
        assert_eq!(pos.pieces(PieceKind::Queen).0, 0x0800_0000_0000_0008);
        assert_eq!(pos.pieces(PieceKind::King).0, 0x1000_0000_0000_0010);
        assert_eq!(pos.king_square(Color::White), Some(Square::E1));
        assert_eq!(pos.king_square(Color::Black), Some(Square::E8));
        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(pos.castling_rights(), CastlingRights::ALL);
        assert_eq!(pos.en_passant(), None);
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.fullmove_number(), 1);
    }

    #[test]
    fn parses_every_field() {
        let fen = "r3k2r/8/8/3pP3/8/8/8/R3K2R w Kq d6 12 34";
        let pos = Position::from_fen(fen).unwrap();
        assert!(pos.is_consistent());
        let white_rook = Piece::new(Color::White, PieceKind::Rook);
        let black_pawn = Piece::new(Color::Black, PieceKind::Pawn);
        assert_eq!(pos.piece_at(Square::A1), Some(white_rook));
        assert_eq!(pos.piece_at(Square::D5), Some(black_pawn));
        assert_eq!(pos.piece_at(Square::D6), None);
        assert_eq!(
            pos.pieces_of(Color::White, PieceKind::Pawn),
            Bitboard::from_square(Square::E5)
        );
        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(
            pos.castling_rights(),
            CastlingRights::WHITE_KINGSIDE.with(CastlingRights::BLACK_QUEENSIDE)
        );
        assert_eq!(pos.en_passant(), Some(Square::D6));
        assert_eq!(pos.halfmove_clock(), 12);
        assert_eq!(pos.fullmove_number(), 34);
    }

    #[test]
    fn black_to_move_and_from_str() {
        let pos: Position = "4k3/8/8/8/8/8/8/4K3 b - - 0 1".parse().unwrap();
        assert_eq!(pos.side_to_move(), Color::Black);
        assert_eq!(pos.occupied().count(), 2);
    }

    #[test]
    fn four_fields_default_the_counters() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - -").unwrap();
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.fullmove_number(), 1);
    }

    #[test]
    fn extra_whitespace_is_ignored() {
        let spaced = "  rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR   w  KQkq\t-  0 1\n";
        assert_eq!(Position::from_fen(spaced).unwrap(), Position::startpos());
    }

    #[test]
    fn rejects_wrong_field_counts() {
        assert_eq!(err(""), FenError::FieldCount { found: 0 });
        assert_eq!(
            err("8/8/8/8/8/8/8/8 w -"),
            FenError::FieldCount { found: 3 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/8/4K3 w - - 0"),
            FenError::FieldCount { found: 5 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/8/4K3 w - - 0 1 extra"),
            FenError::FieldCount { found: 7 }
        );
    }

    #[test]
    fn rejects_bad_placement() {
        assert_eq!(
            err("4k3/8/8/8/8/8/4K3 w - - 0 1"),
            FenError::RankCount { found: 7 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/8/8/4K3 w - - 0 1"),
            FenError::RankCount { found: 9 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/8/4K2 w - - 0 1"),
            FenError::RankLength { rank: 1 }
        );
        assert_eq!(
            err("4k4/8/8/8/8/8/8/4K3 w - - 0 1"),
            FenError::RankLength { rank: 8 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/PPPPPPPPP/4K3 w - - 0 1"),
            FenError::RankLength { rank: 2 }
        );
        assert_eq!(
            err("4k3/8/8/8/8/8//4K3 w - - 0 1"),
            FenError::RankLength { rank: 2 }
        );
        assert_eq!(
            err("4k3/8/8/44/8/8/8/4K3 w - - 0 1"),
            FenError::ConsecutiveDigits { rank: 5 }
        );
        for c in ['x', '9', '0', '-', 'é'] {
            let fen = format!("4k3/8/8/3{c}4/8/8/8/4K3 w - - 0 1");
            assert_eq!(err(&fen), FenError::InvalidPieceChar(c));
        }
    }

    #[test]
    fn rejects_bad_state_fields() {
        let board = "4k3/8/8/8/8/8/8/4K3";
        for side in ["W", "x", "wb", "white"] {
            let fen = format!("{board} {side} - - 0 1");
            assert_eq!(err(&fen), FenError::InvalidSideToMove, "{side}");
        }
        for castling in ["KK", "x", "kqz"] {
            let fen = format!("{board} w {castling} - 0 1");
            assert_eq!(err(&fen), FenError::InvalidCastling, "{castling}");
        }
        for ep in ["e9", "x", "e", "E3"] {
            let fen = format!("{board} w - {ep} 0 1");
            assert_eq!(err(&fen), FenError::InvalidEnPassant, "{ep}");
        }
        for clock in ["x", "-1", "+1", "65536", "1.5"] {
            let fen = format!("{board} w - - {clock} 1");
            assert_eq!(err(&fen), FenError::InvalidHalfmoveClock, "{clock}");
            let fen = format!("{board} w - - 0 {clock}");
            assert_eq!(err(&fen), FenError::InvalidFullmoveNumber, "{clock}");
        }
    }

    #[test]
    fn counters_accept_the_full_u16_range() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 65535 65535").unwrap();
        assert_eq!(pos.halfmove_clock(), u16::MAX);
        assert_eq!(pos.fullmove_number(), u16::MAX);
    }

    #[test]
    fn rejects_wrong_king_counts() {
        let kings = |color, found| FenError::KingCount { color, found };
        assert_eq!(err("4k3/8/8/8/8/8/8/8 w - - 0 1"), kings(Color::White, 0));
        assert_eq!(err("8/8/8/8/8/8/8/4K3 w - - 0 1"), kings(Color::Black, 0));
        assert_eq!(
            err("4k2k/8/8/8/8/8/8/4K3 w - - 0 1"),
            kings(Color::Black, 2)
        );
        assert_eq!(
            err("4k3/8/8/8/8/8/8/K3K2K w - - 0 1"),
            kings(Color::White, 3)
        );
        assert_eq!(err("8/8/8/8/8/8/8/8 w - - 0 1"), kings(Color::White, 0));
    }

    #[test]
    fn rejects_pawns_on_back_ranks() {
        for fen in [
            "P3k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/4K2p w - - 0 1",
            "4k2p/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/P3K3 w - - 0 1",
        ] {
            assert_eq!(err(fen), FenError::PawnOnBackRank, "{fen}");
        }
    }

    #[test]
    fn rejects_castling_rights_without_pieces() {
        for fen in [
            // King off its home square.
            "r3k2r/8/8/8/8/8/8/R2K3R w K - 0 1",
            "r3k2r/8/8/8/8/8/8/R2K3R w Q - 0 1",
            "r2k3r/8/8/8/8/8/8/R3K2R w k - 0 1",
            // Rook missing or replaced by another piece.
            "r3k2r/8/8/8/8/8/8/R3K3 w K - 0 1",
            "r3k2r/8/8/8/8/8/8/4K2R w Q - 0 1",
            "r3k3/8/8/8/8/8/8/R3K2R w k - 0 1",
            "4k2r/8/8/8/8/8/8/R3K2R w q - 0 1",
            "r3k2r/8/8/8/8/8/8/R3K2N w K - 0 1",
            // A rook of the other colour on the home square does not count.
            "r3k2r/8/8/8/8/8/8/R3K2r w K - 0 1",
        ] {
            assert_eq!(err(fen), FenError::CastlingWithoutPieces, "{fen}");
        }
    }

    #[test]
    fn accepts_partial_castling_rights() {
        let pos = Position::from_fen("r3k3/8/8/8/8/8/8/4K2R w Kq - 0 1").unwrap();
        assert_eq!(pos.castling_rights().to_string(), "Kq");
    }

    #[test]
    fn accepts_plausible_en_passant_squares() {
        for fen in [
            "4k3/8/8/8/4P3/8/8/4K3 b - e3 0 1",
            "4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        ] {
            assert!(Position::from_fen(fen).is_ok(), "{fen}");
        }
    }

    #[test]
    fn rejects_impossible_en_passant_squares() {
        for fen in [
            // Wrong rank for the side to move.
            "4k3/8/8/8/4P3/8/8/4K3 w - e3 0 1",
            "4k3/8/8/3p4/8/8/8/4K3 b - d6 0 1",
            "4k3/8/8/8/4P3/8/8/4K3 b - e4 0 1",
            // No pawn beyond the target square.
            "4k3/8/8/8/8/8/8/4K3 b - e3 0 1",
            "4k3/8/8/8/4N3/8/8/4K3 b - e3 0 1",
            // The pawn there is the wrong colour.
            "4k3/8/8/8/4p3/8/8/4K3 b - e3 0 1",
            // The target square is occupied.
            "4k3/8/8/8/4P3/4N3/8/4K3 b - e3 0 1",
            // The starting square of the pawn is occupied.
            "4k3/8/8/8/4P3/8/4N3/4K3 b - e3 0 1",
            "4k3/3n4/8/3p4/8/8/8/4K3 w - d6 0 1",
        ] {
            assert_eq!(err(fen), FenError::EnPassantImpossible, "{fen}");
        }
    }

    /// Canonical FENs covering castling-right subsets, en passant squares for
    /// both sides, promotions-in-waiting, sparse and crowded boards, and
    /// extreme counters. Includes the standard perft reference positions.
    const ROUND_TRIP_FENS: [&str; 14] = [
        START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        "r3k2r/8/8/8/8/8/8/R3K2R b Kq - 3 20",
        "r3k2r/8/8/8/8/8/8/R3K2R w Qk - 0 1",
        "4k3/8/8/8/8/8/8/4K3 b - - 99 150",
        "8/8/8/8/8/8/8/K6k w - - 65535 65535",
        "k7/8/8/8/8/8/8/7K w - - 0 0",
        "1n1q1b1r/P1P1P1P1/1k6/8/8/6K1/p1p1p1p1/R1B1Q1N1 b - - 7 42",
    ];

    #[test]
    fn fen_round_trips_exactly() {
        for fen in ROUND_TRIP_FENS {
            let pos = Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
            assert!(pos.is_consistent(), "{fen}");
            assert_eq!(pos.to_fen(), fen);
            assert_eq!(pos.to_string(), fen);
            assert_eq!(Position::from_fen(&pos.to_fen()).unwrap(), pos, "{fen}");
        }
    }

    #[test]
    fn startpos_serialises_to_start_fen() {
        assert_eq!(Position::startpos().to_fen(), START_FEN);
    }

    #[test]
    fn serialisation_is_canonical() {
        let messy = " r3k2r/8/8/8/8/8/8/R3K2R\tb   qkQK  -  5   9 ";
        let pos = Position::from_fen(messy).unwrap();
        assert_eq!(pos.to_fen(), "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 5 9");
    }

    #[test]
    fn four_field_input_serialises_with_default_counters() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - -").unwrap();
        assert_eq!(pos.to_fen(), "4k3/8/8/8/8/8/8/4K3 w - - 0 1");
    }

    #[test]
    fn empty_position_serialises() {
        assert_eq!(Position::empty().to_fen(), "8/8/8/8/8/8/8/8 w - - 0 1");
    }

    #[test]
    fn rejects_positions_where_the_side_not_to_move_is_in_check() {
        for fen in [
            // White to move while the black king is attacked.
            "4k3/8/8/8/8/8/8/4RK2 w - - 0 1",
            "4k3/3P4/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/5N2/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/7B/8/8/8/4K3 w - - 0 1",
            // Black to move while the white king is attacked.
            "4k3/8/8/8/8/8/8/q3K3 b - - 0 1",
            "4k3/8/8/8/8/8/3p4/4K3 b - - 0 1",
            // Kings side by side: each attacks the other.
            "8/8/8/8/8/8/8/3kK3 w - - 0 1",
        ] {
            assert_eq!(err(fen), FenError::OpponentInCheck, "{fen}");
        }
    }

    #[test]
    fn accepts_positions_where_the_side_to_move_is_in_check() {
        for fen in [
            "4k3/8/8/8/8/8/8/4RK2 b - - 0 1",
            "4k3/8/8/8/8/8/8/q3K3 w - - 0 1",
            // Checkmate and double check are still valid positions.
            "R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1",
            "4k3/8/8/8/1b6/8/4r3/4K3 w - - 0 1",
        ] {
            let pos = Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
            assert!(pos.in_check(), "{fen}");
        }
    }

    #[test]
    fn errors_have_messages() {
        let message = FenError::RankLength { rank: 3 }.to_string();
        assert_eq!(message, "rank 3 does not describe exactly 8 squares");
        assert!(!FenError::InvalidCastling.to_string().is_empty());
    }
}
