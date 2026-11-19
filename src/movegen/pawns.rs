// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Pseudo-legal pawn moves. Pawns are the only pieces whose moves depend on
//! their colour, and the only ones that move and capture differently, so
//! they get their own generator.

use super::MoveList;
use crate::attacks::pawn_attacks;
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

/// The rank on which `color`'s pawns promote.
const fn last_rank(color: Color) -> Bitboard {
    match color {
        Color::White => Bitboard::RANK_8,
        Color::Black => Bitboard::RANK_1,
    }
}

/// Single and double pushes onto empty squares. Pushes onto the last rank
/// are promotions; see [`promotions`].
fn pushes(us: Color, pawns: Bitboard, empty: Bitboard, list: &mut MoveList) {
    let last_rank = last_rank(us);
    let after_one_step = match us {
        Color::White => Bitboard::rank(2),
        Color::Black => Bitboard::rank(5),
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

/// Diagonal captures of enemy pieces. Captures onto the last rank are
/// promotions; see [`promotions`].
fn captures(us: Color, pawns: Bitboard, enemies: Bitboard, list: &mut MoveList) {
    let targets = enemies & !last_rank(us);
    for from in pawns {
        for to in pawn_attacks(us, from) & targets {
            list.push(Move::new(from, to, MoveKind::Capture));
        }
    }
}

/// The pieces a pawn may promote to, most valuable first.
const PROMOTION_PIECES: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

/// Pushes and captures onto the last rank: one move per promotion piece.
fn promotions(us: Color, pawns: Bitboard, empty: Bitboard, enemies: Bitboard, list: &mut MoveList) {
    let last_rank = last_rank(us);
    for to in forward(us, pawns) & empty & last_rank {
        let from = behind(us, to, 1);
        for piece in PROMOTION_PIECES {
            list.push(Move::new(from, to, MoveKind::Promotion(piece)));
        }
    }
    // Only pawns one step from promoting can capture onto the last rank.
    for from in pawns {
        for to in pawn_attacks(us, from) & enemies & last_rank {
            for piece in PROMOTION_PIECES {
                list.push(Move::new(from, to, MoveKind::PromotionCapture(piece)));
            }
        }
    }
}

/// Adds the pseudo-legal pawn moves of the side to move.
pub(super) fn pawn_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let pawns = pos.pieces_of(us, PieceKind::Pawn);
    let empty = !pos.occupied();
    let enemies = pos.occupied_by(us.flip());
    pushes(us, pawns, empty, list);
    captures(us, pawns, enemies, list);
    promotions(us, pawns, empty, enemies, list);
}
