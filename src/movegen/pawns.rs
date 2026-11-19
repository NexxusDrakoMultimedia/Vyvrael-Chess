// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Pseudo-legal pawn moves. Pawns are the only pieces whose moves depend on
//! their colour, and the only ones that move and capture differently, so
//! they get their own generator.

use super::MoveList;
use crate::board::{Bitboard, Color, Move, MoveKind, PieceKind, Position, Square};

/// Every square of `squares` moved one rank in `color`'s forward direction.
const fn forward(color: Color, squares: Bitboard) -> Bitboard {
    match color {
        Color::White => squares.north(),
        Color::Black => squares.south(),
    }
}

/// The square `ranks` ranks behind `square` from `color`'s point of view:
/// where a pawn now on `square` came from.
fn behind(color: Color, square: Square, ranks: u8) -> Square {
    let rank = match color {
        Color::White => square.rank() - ranks,
        Color::Black => square.rank() + ranks,
    };
    Square::new(square.file(), rank)
}

/// Single and double pushes onto empty squares. Pushes onto the last rank
/// are promotions and are not generated here.
fn pushes(us: Color, pawns: Bitboard, empty: Bitboard, list: &mut MoveList) {
    let (last_rank, after_one_step) = match us {
        Color::White => (Bitboard::RANK_8, Bitboard::rank(2)),
        Color::Black => (Bitboard::RANK_1, Bitboard::rank(5)),
    };
    let single = forward(us, pawns) & empty;
    for to in single & !last_rank {
        list.push(Move::new(behind(us, to, 1), to, MoveKind::Quiet));
    }
    // A double push continues a single push from the starting rank, so both
    // squares in front of the pawn must be empty.
    let double = forward(us, single & after_one_step) & empty;
    for to in double {
        list.push(Move::new(behind(us, to, 2), to, MoveKind::DoublePush));
    }
}

/// Adds the pseudo-legal pawn moves of the side to move.
pub(super) fn pawn_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let pawns = pos.pieces_of(us, PieceKind::Pawn);
    pushes(us, pawns, !pos.occupied(), list);
}
