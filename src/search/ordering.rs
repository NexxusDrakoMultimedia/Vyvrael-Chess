// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The order in which the search tries moves: captures first, the most
//! promising of them first.
//!
//! Alpha-beta returns the same value whatever order moves are tried in, but
//! the work it does depends heavily on the order: a good move tried first
//! cuts off the rest. In quiescence search this is not an optimisation but a
//! necessity. Trying captures in generation order, a busy middlegame
//! position takes tens of millions of nodes to resolve; trying the most
//! valuable victim first takes thousands.
//!
//! The main search uses the same order (since milestone 7): captures and
//! promotions by their gain, then the quiet moves as generated. A capture
//! is the likeliest move to refute the opponent's last one, and where it
//! does not, it is cheap to find that out.

use crate::board::{Move, MoveKind, PieceKind, Position, Square};
use crate::movegen::MoveList;

/// A rank for "how much is this piece worth", used only to compare captures.
const fn worth(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 1,
        PieceKind::Knight => 2,
        PieceKind::Bishop => 3,
        PieceKind::Rook => 4,
        PieceKind::Queen => 5,
        PieceKind::King => 6,
    }
}

/// How early `mv` should be tried: higher is earlier.
///
/// "Most valuable victim, least valuable attacker" (MVV-LVA): capturing a
/// queen comes before capturing a pawn, and among captures of the same
/// piece, the one made by the cheapest attacker comes first, since it risks
/// the least. A promotion counts as winning the difference to a queen.
/// Moves that capture nothing come last.
fn priority(pos: &Position, mv: Move) -> i32 {
    let attacker = pos.piece_at(mv.from()).map_or(0, |piece| worth(piece.kind));
    let victim = match mv.kind() {
        MoveKind::EnPassant => worth(PieceKind::Pawn),
        _ => pos.piece_at(mv.to()).map_or(0, |piece| worth(piece.kind)),
    };
    let promoted = mv
        .promotion()
        .map_or(0, |kind| worth(kind) - worth(PieceKind::Pawn));
    if victim == 0 && promoted == 0 {
        return -1;
    }
    // The victim (and any promotion gain) dominates; the attacker only
    // breaks ties, so it is weighted below the smallest victim difference.
    (victim + promoted) * 8 - attacker
}

/// The moves of `moves`, most promising capture first.
///
/// The sort is stable: moves of equal priority keep their generation order,
/// so the result is fully determined by the position. It is an insertion
/// sort on the stack, which suits the few dozen moves a position has and
/// allocates nothing.
pub(super) fn captures_first(pos: &Position, moves: &MoveList) -> MoveList {
    let filler = Move::new(Square::A1, Square::A1, MoveKind::Quiet);
    let mut sorted = [(0i32, filler); MoveList::CAPACITY];
    for (count, &mv) in moves.iter().enumerate() {
        let key = priority(pos, mv);
        let mut slot = count;
        while slot > 0 && sorted[slot - 1].0 < key {
            sorted[slot] = sorted[slot - 1];
            slot -= 1;
        }
        sorted[slot] = (key, mv);
    }
    let mut ordered = MoveList::new();
    for &(_, mv) in &sorted[..moves.len()] {
        ordered.push(mv);
    }
    ordered
}

/// The moves of `moves` in the order the main search tries them: `first`
/// before all others, if it is one of them, then the rest as
/// [`captures_first`] has them.
///
/// `first` is the move the transposition table remembers as best for the
/// position. It was best in an earlier, usually shallower, search, which
/// makes it the likeliest move to be best or to cut off now. It is only
/// moved to the front if the position really has it: a table entry of
/// another position that happens to share the hash cannot put a move here
/// that is not in the list.
pub(super) fn table_move_first(pos: &Position, moves: &MoveList, first: Option<Move>) -> MoveList {
    let sorted = captures_first(pos, moves);
    let Some(first) = first.filter(|mv| sorted.contains(mv)) else {
        return sorted;
    };
    let mut ordered = MoveList::new();
    ordered.push(first);
    for &mv in sorted.iter().filter(|mv| **mv != first) {
        ordered.push(mv);
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::pseudo_legal_moves;

    #[test]
    fn the_table_move_goes_to_the_front_and_nothing_else_moves() {
        // The queen can take a queen, a rook or a pawn; a quiet king move
        // is the table's move.
        let pos = Position::from_fen("4k3/8/1q3r2/8/3Q4/8/1p6/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&pos);
        let plain = captures_first(&pos, &moves);
        let quiet = *plain.iter().find(|mv| mv.to_string() == "e1e2").unwrap();
        let ordered = table_move_first(&pos, &moves, Some(quiet));
        assert_eq!(ordered.len(), plain.len());
        assert_eq!(ordered.as_slice()[0], quiet);
        let rest: Vec<Move> = plain.iter().copied().filter(|mv| *mv != quiet).collect();
        assert_eq!(ordered.as_slice()[1..], rest[..]);
        // The move that is first anyway stays first, once.
        let best = plain.as_slice()[0];
        assert_eq!(table_move_first(&pos, &moves, Some(best)), plain);
        // No move, or a move the position does not have: the plain order.
        assert_eq!(table_move_first(&pos, &moves, None), plain);
        let foreign = Move::new(Square::A1, Square::H8, MoveKind::Quiet);
        assert_eq!(table_move_first(&pos, &moves, Some(foreign)), plain);
    }

    fn ordered(fen: &str) -> Vec<String> {
        let pos = Position::from_fen(fen).unwrap();
        captures_first(&pos, &pseudo_legal_moves(&pos))
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn most_valuable_victim_first() {
        // The queen on d4 can take a queen, a rook or a pawn.
        let moves = ordered("4k3/8/1q3r2/8/3Q4/8/1p6/4K3 w - - 0 1");
        let captures: Vec<&str> = moves.iter().take(3).map(String::as_str).collect();
        assert_eq!(captures, ["d4b6", "d4f6", "d4b2"]);
    }

    #[test]
    fn least_valuable_attacker_breaks_ties() {
        // A pawn, a knight and a queen can all take the rook on d5.
        let moves = ordered("4k3/8/8/3r4/2P2N2/8/8/3QK3 w - - 0 1");
        let captures: Vec<&str> = moves.iter().take(3).map(String::as_str).collect();
        assert_eq!(captures, ["c4d5", "f4d5", "d1d5"]);
    }

    #[test]
    fn promotions_and_en_passant_have_their_place() {
        // Taking the knight and promoting to a queen wins the most, so it
        // comes first. A queen promotion comes before the lesser promotions
        // on the same square, and every promotion before the king's moves.
        let moves = ordered("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1");
        let place = |text: &str| moves.iter().position(|mv| mv == text).unwrap();
        assert_eq!(place("a7b8q"), 0);
        assert!(place("a7b8q") < place("a7b8r") && place("a7b8r") < place("a7b8n"));
        assert!(place("a7a8q") < place("a7a8r") && place("a7a8r") < place("a7a8n"));
        assert!(place("a7a8n") < place("e1e2"));
        // En passant is a pawn capture and is not left among the quiet moves.
        let moves = ordered("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2");
        assert_eq!(moves[0], "e5d6");
    }

    #[test]
    fn it_only_reorders() {
        for fen in [
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ] {
            let pos = Position::from_fen(fen).unwrap();
            let generated = pseudo_legal_moves(&pos);
            let sorted = captures_first(&pos, &generated);
            assert_eq!(sorted.len(), generated.len());
            assert!(generated.iter().all(|mv| sorted.contains(mv)));
            // Captures come before every quiet move.
            let first_quiet = sorted.iter().position(|mv| priority(&pos, *mv) < 0);
            if let Some(index) = first_quiet {
                assert!(sorted[index..].iter().all(|mv| priority(&pos, *mv) < 0));
            }
        }
        // With nothing to capture the generation order is kept exactly.
        let start = Position::startpos();
        let generated = pseudo_legal_moves(&start);
        assert_eq!(captures_first(&start, &generated), generated);
    }
}
