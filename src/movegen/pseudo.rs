// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Pseudo-legal move generation: every move the pieces can make by the rules
//! of movement, without yet asking whether it leaves the mover's own king
//! attacked.

use super::MoveList;
use crate::attacks::knight_attacks;
use crate::board::{Bitboard, Move, MoveKind, PieceKind, Position, Square};

/// Adds a move from `from` to each square in `targets`: a capture where an
/// enemy piece stands, a quiet move otherwise. `targets` must not contain
/// squares occupied by the mover's own pieces.
fn push_piece_moves(list: &mut MoveList, from: Square, targets: Bitboard, enemies: Bitboard) {
    for to in targets {
        let kind = if enemies.contains(to) {
            MoveKind::Capture
        } else {
            MoveKind::Quiet
        };
        list.push(Move::new(from, to, kind));
    }
}

fn knight_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let own = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.flip());
    for from in pos.pieces_of(us, PieceKind::Knight) {
        push_piece_moves(list, from, knight_attacks(from) & !own, enemies);
    }
}

/// Generates the pseudo-legal moves of the side to move.
///
/// Pseudo-legal means each move follows the piece's movement rules on this
/// board, but it may leave the mover's own king attacked. Use the legal move
/// generator when only legal moves are wanted.
///
/// **Incomplete while milestone 2 is in progress:** so far this generates
/// knight moves only. See the checklist in `docs/ROADMAP.md`.
#[must_use]
pub fn pseudo_legal_moves(pos: &Position) -> MoveList {
    let mut list = MoveList::new();
    knight_moves(pos, &mut list);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pseudo-legal moves made by pieces of `kind`, as sorted UCI
    /// strings with `x` appended to captures, so that expected lists are
    /// easy to read and the move kind is checked too.
    fn moves_of(fen: &str, kind: PieceKind) -> Vec<String> {
        let pos = Position::from_fen(fen).unwrap();
        let mut moves: Vec<String> = pseudo_legal_moves(&pos)
            .iter()
            .filter(|mv| pos.piece_at(mv.from()).unwrap().kind == kind)
            .map(|mv| match mv.kind() {
                MoveKind::Quiet => mv.to_string(),
                MoveKind::Capture => format!("{mv}x"),
                other => format!("{mv}:{other:?}"),
            })
            .collect();
        moves.sort();
        moves
    }

    #[test]
    fn knights_in_the_start_position() {
        let white = moves_of(crate::board::START_FEN, PieceKind::Knight);
        assert_eq!(white, ["b1a3", "b1c3", "g1f3", "g1h3"]);
        let black = moves_of(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            PieceKind::Knight,
        );
        assert_eq!(black, ["b8a6", "b8c6", "g8f6", "g8h6"]);
    }

    #[test]
    fn knight_captures_enemies_and_avoids_own_pieces() {
        // Knight e3: d5 holds a black pawn, f5 a white pawn.
        let moves = moves_of("4k3/8/8/3p1P2/8/4N3/8/4K3 w - - 0 1", PieceKind::Knight);
        assert_eq!(
            moves,
            ["e3c2", "e3c4", "e3d1", "e3d5x", "e3f1", "e3g2", "e3g4"]
        );
    }

    #[test]
    fn knight_in_the_corner() {
        let moves = moves_of("4k3/8/8/8/8/8/8/N3K3 w - - 0 1", PieceKind::Knight);
        assert_eq!(moves, ["a1b3", "a1c2"]);
    }
}
