// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Static exchange evaluation: what a move wins or loses on its destination
//! square if both sides go on capturing there, without searching.
//!
//! "Queen takes pawn" is a capture of a pawn, and if the pawn is defended
//! it is the loss of a queen for a pawn. Ordering captures by the victim
//! alone cannot tell the two apart; playing the exchange out can.
//!
//! The exchange is played on paper. Each side in turn captures on the
//! square with its least valuable piece, and the running balance is noted
//! after each capture (the "swap list"). Then the list is walked back:
//! either side may stop instead of making a capture that leaves it worse
//! off, so each entry is the better of stopping and going on.
//!
//! What it leaves out, as every static exchange evaluation does: pins (a
//! pinned piece is counted as able to capture), checks, and everything that
//! happens off the square. It is a quick estimate for ordering and pruning,
//! not a search.

use crate::attacks::{bishop_attacks, rook_attacks};
use crate::board::{Bitboard, Move, MoveKind, PieceKind, Position, Square};

/// What a piece counts for in an exchange. Not the evaluation's values:
/// knight and bishop are equal here, so that bishop takes defended knight
/// is an even trade and not a small loss, and the king is worth more than
/// everything else together, though it is never taken: it only captures
/// when nothing can take it back.
#[must_use]
pub const fn exchange_value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight | PieceKind::Bishop => 325,
        PieceKind::Rook => 500,
        PieceKind::Queen => 1_000,
        PieceKind::King => 20_000,
    }
}

/// The kinds in the order they are used to capture: cheapest first.
const BY_VALUE: [PieceKind; 6] = [
    PieceKind::Pawn,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Rook,
    PieceKind::Queen,
    PieceKind::King,
];

/// What promoting a pawn to `kind` adds to the balance.
const fn promotion_gain(kind: PieceKind) -> i32 {
    exchange_value(kind) - exchange_value(PieceKind::Pawn)
}

/// Whether a pawn arriving on `square` promotes.
const fn is_last_rank(square: Square) -> bool {
    square.rank() == 0 || square.rank() == 7
}

/// The material `mv` wins (positive) or loses (negative) for the side that
/// plays it in `pos`, in the units of [`exchange_value`], if both sides
/// then capture on its destination square for as long as it pays.
///
/// A move that captures nothing can still lose the piece that moves, so it
/// has a value too (0 or less, or the gain of a promotion). A pawn that
/// captures onto the last rank during the exchange is taken to promote to a
/// queen. Castling is 0. `mv` must be a move of the side to move in `pos`;
/// it need not be legal.
#[must_use]
pub fn see(pos: &Position, mv: Move) -> i32 {
    let (from, to) = (mv.from(), mv.to());
    let Some(mover) = pos.piece_at(from) else {
        return 0;
    };
    if mv.is_castle() {
        return 0;
    }
    let mut occupied = pos.occupied().without(from);
    let victim = match mv.kind() {
        MoveKind::EnPassant => {
            // The pawn taken stands beside the origin, on the file of the
            // destination; it leaves the board too.
            occupied = occupied.without(Square::new(to.file(), from.rank()));
            exchange_value(PieceKind::Pawn)
        }
        _ => pos
            .piece_at(to)
            .map_or(0, |piece| exchange_value(piece.kind)),
    };
    // What stands on the square now, to be taken next.
    let mut standing = mv.promotion().unwrap_or(mover.kind);
    // The balance after each capture, from the point of view of the side
    // that made it. Thirty-two pieces cannot capture more often than this.
    let mut balance = [0i32; 32];
    balance[0] = victim + mv.promotion().map_or(0, promotion_gain);
    let mut depth = 0;

    let diagonal = pos.pieces(PieceKind::Bishop) | pos.pieces(PieceKind::Queen);
    let straight = pos.pieces(PieceKind::Rook) | pos.pieces(PieceKind::Queen);
    let mut attackers = pos.attackers_to(to, occupied) & occupied;
    let mut side = mover.color.flip();
    loop {
        let ours = attackers & pos.occupied_by(side);
        let Some((kind, square)) = cheapest(pos, ours) else {
            break;
        };
        // A king cannot capture a piece that is defended. (Its value alone
        // would stop it, since the capture could be answered by taking the
        // king; saying so here ends the exchange where it ends.)
        if kind == PieceKind::King && !(attackers & pos.occupied_by(side.flip())).is_empty() {
            break;
        }
        depth += 1;
        balance[depth] = exchange_value(standing) - balance[depth - 1];
        standing = kind;
        if kind == PieceKind::Pawn && is_last_rank(to) {
            balance[depth] += promotion_gain(PieceKind::Queen);
            standing = PieceKind::Queen;
        }
        // The capturer has left its square: a slider behind it on the same
        // line now attacks the square too (an "x-ray").
        occupied = occupied.without(square);
        attackers |=
            (bishop_attacks(to, occupied) & diagonal) | (rook_attacks(to, occupied) & straight);
        attackers &= occupied;
        side = side.flip();
    }
    // Walk back: before each capture the side to make it could have
    // stopped, and does if the capture leaves it worse off.
    while depth > 0 {
        balance[depth - 1] = -(-balance[depth - 1]).max(balance[depth]);
        depth -= 1;
    }
    balance[0]
}

/// The least valuable piece among `pieces`, with its square; of several of
/// one kind, the one on the lowest square, so that the choice depends on
/// the position alone.
fn cheapest(pos: &Position, pieces: Bitboard) -> Option<(PieceKind, Square)> {
    if pieces.is_empty() {
        return None;
    }
    BY_VALUE.into_iter().find_map(|kind| {
        (pieces & pos.pieces(kind))
            .lsb()
            .map(|square| (kind, square))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::{MoveList, legal_moves, pseudo_legal_moves};

    const P: i32 = exchange_value(PieceKind::Pawn);
    const N: i32 = exchange_value(PieceKind::Knight);
    const B: i32 = exchange_value(PieceKind::Bishop);
    const R: i32 = exchange_value(PieceKind::Rook);
    const Q: i32 = exchange_value(PieceKind::Queen);

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    /// The value of the move written `text` (as in UCI) in the position.
    fn value(position: &str, text: &str) -> i32 {
        let pos = fen(position);
        let moves = pseudo_legal_moves(&pos);
        let mv = moves
            .iter()
            .find(|mv| mv.to_string() == text)
            .unwrap_or_else(|| panic!("{position}: no move {text}"));
        see(&pos, *mv)
    }

    #[test]
    fn simple_exchanges() {
        // A pawn that nothing defends.
        assert_eq!(value("4k3/8/8/3p4/8/8/8/3RK3 w - - 0 1", "d1d5"), P);
        // A pawn defended by a pawn: the rook is lost for it.
        assert_eq!(value("4k3/8/4p3/3p4/8/8/8/3RK3 w - - 0 1", "d1d5"), P - R);
        // Pawn takes pawn, pawn takes back: even.
        assert_eq!(value("4k3/8/4p3/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5"), 0);
        // Bishop takes a knight that a pawn defends: an even trade here.
        assert_eq!(value("4k3/8/4p3/3n4/8/5B2/8/4K3 w - - 0 1", "f3d5"), N - B);
        assert_eq!(N - B, 0);
        // Knight takes a rook that a pawn defends: the exchange is won.
        assert_eq!(value("4k3/8/4p3/3r4/8/4N3/8/4K3 w - - 0 1", "e3d5"), R - N);
        // Queen takes a pawn that a rook defends.
        assert_eq!(value("3rk3/8/8/3p4/8/8/8/3QK3 w - - 0 1", "d1d5"), P - Q);
    }

    /// A slider behind the capturing piece joins the exchange once the
    /// piece in front of it has captured.
    #[test]
    fn x_rays() {
        // Rook takes pawn, rook takes rook, the second rook takes back.
        let doubled = "3rk3/8/8/3p4/8/8/3R4/3RK3 w - - 0 1";
        assert_eq!(value(doubled, "d2d5"), P);
        // Without the second rook it is a rook for a pawn.
        assert_eq!(value("3rk3/8/8/3p4/8/8/3R4/4K3 w - - 0 1", "d2d5"), P - R);
        // A queen behind a bishop on the diagonal, against a pawn defender:
        // bishop takes pawn, pawn takes bishop, queen takes pawn. A bishop
        // for two pawns, which is still better than a bishop for one.
        let battery = "4k3/8/4p3/3p4/8/5B2/6Q1/4K3 w - - 0 1";
        assert_eq!(value(battery, "f3d5"), 2 * P - B);
        assert_eq!(value("4k3/8/4p3/3p4/8/5B2/8/4K3 w - - 0 1", "f3d5"), P - B);
        // A pawn in front of a bishop: pawn takes, pawn takes back, bishop
        // takes.
        let behind_pawn = "4k3/8/4p3/3p4/4P3/5B2/8/4K3 w - - 0 1";
        assert_eq!(value(behind_pawn, "e4d5"), P);
        // Black's x-ray counts as well: two rooks defend through each other.
        let both = "3rk3/3r4/8/3p4/8/8/3R4/3RK3 w - - 0 1";
        assert_eq!(value(both, "d2d5"), P - R);
    }

    #[test]
    fn kings_capture_only_what_is_not_defended() {
        // The king is the only defender: it takes back.
        assert_eq!(value("8/8/4k3/3p4/8/8/8/3RK3 w - - 0 1", "d1d5"), P - R);
        // The king would step into the second rook's line: it does not.
        assert_eq!(value("8/8/4k3/3p4/8/8/3R4/3RK3 w - - 0 1", "d2d5"), P);
        // A king may make the first capture of an undefended piece.
        assert_eq!(value("4k3/8/8/8/8/8/4p3/4K3 w - - 0 1", "e1e2"), P);
    }

    #[test]
    fn quiet_moves_promotions_and_en_passant() {
        // A quiet move to a safe square is worth nothing, and one onto an
        // attacked square loses the piece.
        assert_eq!(value("4k3/8/8/8/8/8/8/3RK3 w - - 0 1", "d1d4"), 0);
        assert_eq!(value("4k3/8/4p3/8/8/8/8/3RK3 w - - 0 1", "d1d5"), -R);
        // Promotion: the gain of the new piece, or the pawn if it is taken.
        assert_eq!(value("8/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7d8q"), Q - P);
        assert_eq!(value("8/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7d8n"), N - P);
        assert_eq!(value("1r6/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7d8q"), -P);
        // Taking while promoting, and being taken back.
        assert_eq!(
            value("2r5/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7c8q"),
            R + Q - P
        );
        assert_eq!(value("1rr5/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7c8q"), R - P);
        // A pawn that recaptures on the last rank becomes a queen: rook
        // takes knight, pawn takes rook and promotes.
        assert_eq!(
            value("4k3/8/8/8/8/8/2p5/1N1RK3 b - - 0 1", "c2b1q"),
            N + Q - P - Q,
            "the rook takes the new queen"
        );
        // En passant: a pawn for nothing, or for a pawn.
        let free = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
        assert_eq!(value(free, "e5d6"), P);
        let defended = "4k3/2p5/8/3pP3/8/8/8/4K3 w - d6 0 1";
        assert_eq!(value(defended, "e5d6"), 0);
        // The pawn taken en passant no longer blocks the file behind it.
        let opened = "3rk3/8/8/3pP3/8/8/8/3RK3 w - d6 0 1";
        assert_eq!(value(opened, "e5d6"), P);
        // Castling is not an exchange.
        assert_eq!(value("4k3/8/8/8/8/8/8/4K2R w K - 0 1", "e1g1"), 0);
    }

    // The oracle: the same exchange played out on copies of the board.

    /// The capture the side to move makes on `square` with its cheapest
    /// piece (lowest origin among equals), if it has one. A king does not
    /// capture into an attack.
    fn cheapest_capture(pos: &Position, square: Square) -> Option<Move> {
        let rank = |mv: &Move| {
            let kind = pos.piece_at(mv.from()).unwrap().kind;
            let order = BY_VALUE.iter().position(|&each| each == kind).unwrap();
            (order, mv.from().index())
        };
        let mv = pseudo_legal_moves(pos)
            .iter()
            .copied()
            .filter(|mv| mv.to() == square && mv.is_capture())
            .filter(|mv| mv.kind() != MoveKind::EnPassant)
            .filter(|mv| matches!(mv.promotion(), None | Some(PieceKind::Queen)))
            .min_by_key(rank)?;
        if pos.piece_at(mv.from()).unwrap().kind == PieceKind::King {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                return None;
            }
        }
        Some(mv)
    }

    /// What `mv` takes off the board and adds by promoting.
    fn gain(pos: &Position, mv: Move) -> i32 {
        let taken = match mv.kind() {
            MoveKind::EnPassant => P,
            _ => pos
                .piece_at(mv.to())
                .map_or(0, |piece| exchange_value(piece.kind)),
        };
        taken + mv.promotion().map_or(0, promotion_gain)
    }

    /// The most the side to move can win on `square` from here on, where
    /// winning nothing (stopping) is always possible.
    fn best_reply(pos: &Position, square: Square) -> i32 {
        let Some(mv) = cheapest_capture(pos, square) else {
            return 0;
        };
        let mut next = pos.clone();
        let _undo = next.make_move(mv);
        (gain(pos, mv) - best_reply(&next, square)).max(0)
    }

    fn played_out(pos: &Position, mv: Move) -> i32 {
        let mut next = pos.clone();
        let _undo = next.make_move(mv);
        gain(pos, mv) - best_reply(&next, mv.to())
    }

    /// Checks every move of `pos` but castling, then the positions after
    /// its legal moves, `depth` plies deep. Returns how many moves it
    /// checked.
    fn check_tree(pos: &Position, depth: u32) -> u64 {
        let moves: MoveList = pseudo_legal_moves(pos);
        let mut checked = 0;
        for &mv in moves.iter().filter(|mv| !mv.is_castle()) {
            assert_eq!(see(pos, mv), played_out(pos, mv), "{} {mv}", pos.to_fen());
            checked += 1;
        }
        if depth > 0 {
            for &mv in &legal_moves(pos) {
                let mut next = pos.clone();
                let _undo = next.make_move(mv);
                checked += check_tree(&next, depth - 1);
            }
        }
        checked
    }

    /// The evaluation agrees with the exchange really played out, for every
    /// move, capture or not, of every position two plies around a set of
    /// busy positions: the perft reference positions and some with
    /// promotions, en passant and batteries.
    #[test]
    fn it_agrees_with_the_exchange_played_out() {
        let positions = [
            crate::board::START_FEN,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1",
            "1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1",
            "2r1r1k1/pp1bppbp/3p1np1/q3P3/2P2P2/1P2B3/P1N1B1PP/2RQ1RK1 b - - 0 1",
            "6k1/1pp4p/p1pb4/6q1/3P1pRr/2P4P/PP1Br1P1/5RKN w - - 0 1",
            "4k3/1P1p2P1/8/2pPp3/8/8/1p4p1/R3K2R w KQ c6 0 1",
            // En passant that opens a line onto the square, for each side.
            "3rk3/8/8/3pP3/8/8/8/3RK3 w - d6 0 1",
            "3rk3/2p5/8/4P3/2Pp4/8/1B6/3RK3 b - c3 0 1",
        ];
        let mut checked = 0;
        for position in positions {
            checked += check_tree(&fen(position), 2);
        }
        assert!(checked > 400_000, "{checked}");
    }
}
