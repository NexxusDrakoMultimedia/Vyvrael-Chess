// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Castling move generation.
//!
//! Castling is the one move whose legality cannot be checked afterwards by
//! looking at where the king ended up: the king may not castle out of check
//! or across an attacked square either. So all of its conditions are checked
//! here, and a generated castling move is fully legal.

use super::MoveList;
use crate::board::{Bitboard, CastlingRights, Color, Move, MoveKind, Position, Square};

/// One way of castling: what must hold for it and the move it produces.
struct Castle {
    right: CastlingRights,
    /// Squares between king and rook, which must be empty.
    between: Bitboard,
    /// Squares the king crosses and lands on, which must not be attacked.
    king_path: [Square; 2],
    king_from: Square,
    king_to: Square,
    kind: MoveKind,
}

const fn squares(list: &[Square]) -> Bitboard {
    let mut set = Bitboard::EMPTY;
    let mut i = 0;
    while i < list.len() {
        set = set.with(list[i]);
        i += 1;
    }
    set
}

const WHITE: [Castle; 2] = [
    Castle {
        right: CastlingRights::WHITE_KINGSIDE,
        between: squares(&[Square::F1, Square::G1]),
        king_path: [Square::F1, Square::G1],
        king_from: Square::E1,
        king_to: Square::G1,
        kind: MoveKind::KingCastle,
    },
    Castle {
        right: CastlingRights::WHITE_QUEENSIDE,
        between: squares(&[Square::B1, Square::C1, Square::D1]),
        king_path: [Square::D1, Square::C1],
        king_from: Square::E1,
        king_to: Square::C1,
        kind: MoveKind::QueenCastle,
    },
];

const BLACK: [Castle; 2] = [
    Castle {
        right: CastlingRights::BLACK_KINGSIDE,
        between: squares(&[Square::F8, Square::G8]),
        king_path: [Square::F8, Square::G8],
        king_from: Square::E8,
        king_to: Square::G8,
        kind: MoveKind::KingCastle,
    },
    Castle {
        right: CastlingRights::BLACK_QUEENSIDE,
        between: squares(&[Square::B8, Square::C8, Square::D8]),
        king_path: [Square::D8, Square::C8],
        king_from: Square::E8,
        king_to: Square::C8,
        kind: MoveKind::QueenCastle,
    },
];

/// Adds the castling moves available to the side to move.
///
/// A castling move is generated when the side still has the right, the
/// squares between king and rook are empty, the king is not in check, and
/// neither the square it crosses nor the square it lands on is attacked.
/// When castling queenside, the b-file square only has to be empty; it may
/// be attacked, because the king does not cross it.
pub(super) fn castling_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let them = us.flip();
    let castles = match us {
        Color::White => &WHITE,
        Color::Black => &BLACK,
    };
    let rights = pos.castling_rights();
    // Cheap exits first: most positions have no rights left for the mover.
    if !castles.iter().any(|castle| rights.contains(castle.right)) || pos.in_check() {
        return;
    }
    let occupied = pos.occupied();
    for castle in castles {
        if rights.contains(castle.right)
            && (occupied & castle.between).is_empty()
            && !castle
                .king_path
                .iter()
                .any(|&square| pos.is_attacked_by(square, them))
        {
            list.push(Move::new(castle.king_from, castle.king_to, castle.kind));
        }
    }
}
