// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Pseudo-legal move generation: every move the pieces can make by the rules
//! of movement, without yet asking whether it leaves the mover's own king
//! attacked.

use super::MoveList;
use super::pawns::pawn_moves;
use crate::attacks::{bishop_attacks, king_attacks, knight_attacks, queen_attacks, rook_attacks};
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

/// King steps to the eight neighbouring squares. Castling is generated
/// separately.
fn king_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let own = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.flip());
    for from in pos.pieces_of(us, PieceKind::King) {
        push_piece_moves(list, from, king_attacks(from) & !own, enemies);
    }
}

/// Bishop, rook and queen moves: slide until the edge or the first piece,
/// capturing it if it is an enemy.
fn slider_moves(pos: &Position, list: &mut MoveList) {
    type Attacks = fn(Square, Bitboard) -> Bitboard;
    const SLIDERS: [(PieceKind, Attacks); 3] = [
        (PieceKind::Bishop, bishop_attacks),
        (PieceKind::Rook, rook_attacks),
        (PieceKind::Queen, queen_attacks),
    ];
    let us = pos.side_to_move();
    let own = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.flip());
    let occupied = pos.occupied();
    for (kind, attacks) in SLIDERS {
        for from in pos.pieces_of(us, kind) {
            push_piece_moves(list, from, attacks(from, occupied) & !own, enemies);
        }
    }
}

/// Generates the pseudo-legal moves of the side to move.
///
/// Pseudo-legal means each move follows the piece's movement rules on this
/// board, but it may leave the mover's own king attacked. Use the legal move
/// generator when only legal moves are wanted.
///
/// **Incomplete while milestone 2 is in progress:** so far this generates
/// piece moves, king steps (no castling) and pawn pushes (no pawn captures,
/// promotions or en passant). See the checklist in `docs/ROADMAP.md`.
#[must_use]
pub fn pseudo_legal_moves(pos: &Position) -> MoveList {
    let mut list = MoveList::new();
    pawn_moves(pos, &mut list);
    knight_moves(pos, &mut list);
    king_moves(pos, &mut list);
    slider_moves(pos, &mut list);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pseudo-legal moves made by pieces of `kind`, as sorted UCI
    /// strings with `x` appended to captures and `+` to double pushes, so
    /// that expected lists are easy to read and the move kind is checked
    /// too.
    fn moves_of(fen: &str, kind: PieceKind) -> Vec<String> {
        let pos = Position::from_fen(fen).unwrap();
        let mut moves: Vec<String> = pseudo_legal_moves(&pos)
            .iter()
            .filter(|mv| pos.piece_at(mv.from()).unwrap().kind == kind)
            .map(|mv| match mv.kind() {
                MoveKind::Quiet => mv.to_string(),
                MoveKind::Capture => format!("{mv}x"),
                MoveKind::DoublePush => format!("{mv}+"),
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
    fn king_steps() {
        let alone = moves_of("4k3/8/8/8/8/8/8/4K3 w - - 0 1", PieceKind::King);
        assert_eq!(alone, ["e1d1", "e1d2", "e1e2", "e1f1", "e1f2"]);
        let black = moves_of("4k3/8/8/8/8/8/8/4K3 b - - 0 1", PieceKind::King);
        assert_eq!(black, ["e8d7", "e8d8", "e8e7", "e8f7", "e8f8"]);
        // d2 holds a black pawn, e2 a white pawn.
        let crowded = moves_of("4k3/8/8/8/8/8/3pP3/4K3 w - - 0 1", PieceKind::King);
        assert_eq!(crowded, ["e1d1", "e1d2x", "e1f1", "e1f2"]);
        let centre = moves_of("4k3/8/8/8/4K3/8/8/8 w - - 0 1", PieceKind::King);
        assert_eq!(centre.len(), 8);
    }

    /// Pseudo-legal on purpose: stepping onto an attacked square is still
    /// generated here and removed later by the legality filter.
    #[test]
    fn king_steps_ignore_attacks() {
        let moves = moves_of("4k3/8/8/8/8/8/8/r3K3 w - - 0 1", PieceKind::King);
        assert!(moves.contains(&"e1d1".to_owned()));
    }

    #[test]
    fn bishop_slides_and_stops_at_pieces() {
        // Bishop b2; a black pawn on c3 can be captured and blocks beyond.
        let moves = moves_of("4k3/8/8/8/8/2p5/1B6/4K3 w - - 0 1", PieceKind::Bishop);
        assert_eq!(moves, ["b2a1", "b2a3", "b2c1", "b2c3x"]);
    }

    #[test]
    fn rook_slides_and_stops_at_pieces() {
        // Rook a2; its own pawn on d2 blocks the rank from d2 on.
        let moves = moves_of("4k3/8/8/8/8/8/R2P4/4K3 w - - 0 1", PieceKind::Rook);
        assert_eq!(
            moves,
            [
                "a2a1", "a2a3", "a2a4", "a2a5", "a2a6", "a2a7", "a2a8", "a2b2", "a2c2"
            ]
        );
        let black = moves_of("r3k3/8/8/8/8/8/8/R3K3 b - - 0 1", PieceKind::Rook);
        assert_eq!(
            black,
            [
                "a8a1x", "a8a2", "a8a3", "a8a4", "a8a5", "a8a6", "a8a7", "a8b8", "a8c8", "a8d8"
            ]
        );
    }

    #[test]
    fn queen_moves_like_bishop_and_rook_together() {
        // Queen a1: 7 up the file, 3 along the rank before its own king, 7
        // on the long diagonal.
        let moves = moves_of("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1", PieceKind::Queen);
        assert_eq!(moves.len(), 17);
        for expected in ["a1a8", "a1d1", "a1h8", "a1b2"] {
            assert!(moves.contains(&expected.to_owned()), "{expected}");
        }
        assert!(!moves.iter().any(|mv| mv.starts_with("a1e1")));
    }

    #[test]
    fn sliders_are_boxed_in_at_the_start() {
        for kind in [PieceKind::Bishop, PieceKind::Rook, PieceKind::Queen] {
            assert!(moves_of(crate::board::START_FEN, kind).is_empty());
        }
    }

    #[test]
    fn pawn_pushes_in_the_start_position() {
        let white = moves_of(crate::board::START_FEN, PieceKind::Pawn);
        assert_eq!(white.len(), 16);
        assert_eq!(white.iter().filter(|mv| mv.ends_with('+')).count(), 8);
        assert!(white.contains(&"e2e3".to_owned()));
        assert!(white.contains(&"e2e4+".to_owned()));
        let black = moves_of(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            PieceKind::Pawn,
        );
        assert_eq!(black.len(), 16);
        assert!(black.contains(&"e7e6".to_owned()));
        assert!(black.contains(&"e7e5+".to_owned()));
    }

    #[test]
    fn pawn_pushes_need_empty_squares() {
        // Blocked directly in front: no push at all.
        let blocked = moves_of("4k3/8/8/8/8/4p3/4P3/4K3 w - - 0 1", PieceKind::Pawn);
        assert!(blocked.is_empty());
        // Second square blocked: single push only.
        let half = moves_of("4k3/8/8/8/4p3/8/4P3/4K3 w - - 0 1", PieceKind::Pawn);
        assert_eq!(half, ["e2e3"]);
        // The same for Black.
        let black = moves_of("4k3/4p3/8/4P3/8/8/8/4K3 b - - 0 1", PieceKind::Pawn);
        assert_eq!(black, ["e7e6"]);
        let black_blocked = moves_of("4k3/4p3/4P3/8/8/8/8/4K3 b - - 0 1", PieceKind::Pawn);
        assert!(black_blocked.is_empty());
    }

    #[test]
    fn only_pawns_on_their_starting_rank_double_push() {
        let white = moves_of("4k3/8/8/8/8/4P3/P7/4K3 w - - 0 1", PieceKind::Pawn);
        assert_eq!(white, ["a2a3", "a2a4+", "e3e4"]);
        let black = moves_of("4k3/7p/4p3/8/8/8/8/4K3 b - - 0 1", PieceKind::Pawn);
        assert_eq!(black, ["e6e5", "h7h5+", "h7h6"]);
    }

    #[test]
    fn knight_in_the_corner() {
        let moves = moves_of("4k3/8/8/8/8/8/8/N3K3 w - - 0 1", PieceKind::Knight);
        assert_eq!(moves, ["a1b3", "a1c2"]);
    }
}
