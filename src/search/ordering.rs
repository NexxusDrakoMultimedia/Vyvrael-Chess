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
//! promotions by their gain, then the quiet moves. A capture is the
//! likeliest move to refute the opponent's last one, and where it does
//! not, it is cheap to find that out.
//!
//! Among the quiet moves the **killer moves** come first: the quiet moves
//! that last cut off the search in another position at the same distance
//! from the root ([`Killers`]). The rest follow by how well they have done
//! in the search so far ([`History`]), and as generated where that does
//! not tell them apart.
//!
//! "By their gain" looks only at what is taken. A capture that loses
//! material once the opponent takes back (queen takes defended pawn) is
//! told apart by the static exchange evaluation ([`see`]): the main search
//! tries such captures last, after the quiet moves, and the quiescence
//! search does not try them at all.

use super::history::History;
use super::see::{exchange_value, see};
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

/// Whether `mv` loses material on its destination square once the
/// exchange there is played out: a static exchange evaluation below 0.
///
/// Taking a piece worth at least the one that takes cannot lose, since at
/// worst the taker is taken back. That settles most captures without
/// working the exchange out.
pub(super) fn loses_material(pos: &Position, mv: Move) -> bool {
    if mv.promotion().is_none() {
        let worth = |square| {
            pos.piece_at(square)
                .map_or(0, |piece| exchange_value(piece.kind))
        };
        let victim = match mv.kind() {
            MoveKind::EnPassant => exchange_value(PieceKind::Pawn),
            _ => worth(mv.to()),
        };
        if victim >= worth(mv.from()) {
            return false;
        }
    }
    see(pos, mv) < 0
}

/// Whether `mv` neither captures nor promotes: the kind of move that can be
/// a killer move.
pub(super) fn is_quiet(pos: &Position, mv: Move) -> bool {
    priority(pos, mv) < 0
}

/// The killer moves of one ply: the two quiet moves that most recently cut
/// off the search in a position that many plies from the root, the newer
/// one first.
///
/// A move that refutes one of the opponent's tries often refutes the next
/// one too, because the two positions differ by that one try only: the
/// knight fork is still there whichever pawn was pushed. Captures need no
/// such memory, since they are tried early anyway.
///
/// Two are kept so that one cutoff by another move does not make the
/// search forget a move that has worked many times.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Killers([Option<Move>; 2]);

impl Killers {
    /// No killer moves.
    pub(super) const NONE: Killers = Killers([None; 2]);

    /// Notes that the quiet move `mv` cut off the search. It becomes the
    /// first killer and the first until now becomes the second; if it is
    /// the first already, nothing changes, so the two are never the same
    /// move.
    pub(super) fn remember(&mut self, mv: Move) {
        if self.0[0] != Some(mv) {
            self.0[1] = self.0[0];
            self.0[0] = Some(mv);
        }
    }

    /// The two moves, the newer first.
    #[cfg(test)]
    pub(super) const fn moves(self) -> [Option<Move>; 2] {
        self.0
    }

    /// How early `mv` is tried for being a killer: 1 for the first, 0 for
    /// the second. `None` if it is not a killer.
    fn key(self, mv: Move) -> Option<i32> {
        if self.0[0] == Some(mv) {
            Some(1)
        } else if self.0[1] == Some(mv) {
            Some(0)
        } else {
            None
        }
    }
}

/// How far below the quiet moves a losing capture is put.
const LOSING: i32 = 1_000;

/// [`priority`] for the main search: a capture or promotion that loses
/// material comes after the quiet moves, the losing ones among themselves
/// in their usual order.
fn priority_by_exchange(pos: &Position, mv: Move) -> i32 {
    let key = priority(pos, mv);
    if key >= 0 && loses_material(pos, mv) {
        key - LOSING
    } else {
        key
    }
}

/// The moves of `moves`, most promising capture first, by what is taken
/// alone. The order of the quiescence search.
pub(super) fn captures_first(pos: &Position, moves: &MoveList) -> MoveList {
    sorted_by(moves, |mv| priority(pos, mv))
}

/// The moves of `moves` as the main search tries them when it knows no
/// move to try first: captures and promotions that do not lose material,
/// most promising first, then the quiet moves, then the captures that
/// lose material.
#[cfg(test)]
pub(super) fn losing_captures_last(pos: &Position, moves: &MoveList) -> MoveList {
    main_search_order(pos, moves, Killers::NONE, None)
}

/// The distance between the groups of moves in [`main_search_order`]: far
/// more than any key inside a group, so that no history value can lift a
/// quiet move above a killer or drop it below a losing capture.
const BAND: i32 = 1 << 20;

/// [`losing_captures_last`] with the quiet moves put in order: captures
/// and promotions that do not lose material, then the killers, the newer
/// first, then the other quiet moves, the one with the best `history`
/// first and as generated among equals (all of them as generated without
/// a `history`), then the captures that lose material.
///
/// A killer comes from another position. It counts only if this position
/// has it and it is a quiet move here: one that is a capture here keeps
/// its place among the captures.
fn main_search_order(
    pos: &Position,
    moves: &MoveList,
    killers: Killers,
    history: Option<&History>,
) -> MoveList {
    let side = pos.side_to_move();
    sorted_by(moves, |mv| {
        let key = priority_by_exchange(pos, mv);
        if key >= 0 {
            BAND + key
        } else if key < -1 {
            key - BAND
        } else if let Some(killer) = killers.key(mv) {
            BAND / 2 + killer
        } else {
            history.map_or(0, |history| history.value(side, mv))
        }
    })
}

/// The moves of `moves`, highest `key` first.
///
/// The sort is stable: moves of equal priority keep their generation order,
/// so the result is fully determined by the position. It is an insertion
/// sort on the stack, which suits the few dozen moves a position has and
/// allocates nothing.
fn sorted_by(moves: &MoveList, key: impl Fn(Move) -> i32) -> MoveList {
    let filler = Move::new(Square::A1, Square::A1, MoveKind::Quiet);
    let mut sorted = [(0i32, filler); MoveList::CAPACITY];
    for (count, &mv) in moves.iter().enumerate() {
        let key = key(mv);
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
/// before all others, if it is one of them, then the captures and
/// promotions that do not lose material, the `killers`, the other quiet
/// moves by their `history` and the captures that lose material.
///
/// `first` is the move the transposition table remembers as best for the
/// position. It was best in an earlier, usually shallower, search, which
/// makes it the likeliest move to be best or to cut off now. It is only
/// moved to the front if the position really has it: a table entry of
/// another position that happens to share the hash cannot put a move here
/// that is not in the list.
pub(super) fn table_move_first(
    pos: &Position,
    moves: &MoveList,
    first: Option<Move>,
    killers: Killers,
    history: Option<&History>,
) -> MoveList {
    let sorted = main_search_order(pos, moves, killers, history);
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
        let plain = losing_captures_last(&pos, &moves);
        let quiet = *plain.iter().find(|mv| mv.to_string() == "e1e2").unwrap();
        let ordered = table_move_first(&pos, &moves, Some(quiet), Killers::NONE, None);
        assert_eq!(ordered.len(), plain.len());
        assert_eq!(ordered.as_slice()[0], quiet);
        let rest: Vec<Move> = plain.iter().copied().filter(|mv| *mv != quiet).collect();
        assert_eq!(ordered.as_slice()[1..], rest[..]);
        // The move that is first anyway stays first, once.
        let best = plain.as_slice()[0];
        assert_eq!(
            table_move_first(&pos, &moves, Some(best), Killers::NONE, None),
            plain
        );
        // No move, or a move the position does not have: the plain order.
        assert_eq!(
            table_move_first(&pos, &moves, None, Killers::NONE, None),
            plain
        );
        let foreign = Move::new(Square::A1, Square::H8, MoveKind::Quiet);
        assert_eq!(
            table_move_first(&pos, &moves, Some(foreign), Killers::NONE, None),
            plain
        );
    }

    #[test]
    fn killers_remember_the_two_newest_and_never_one_move_twice() {
        let mv = |from, to| Move::new(from, to, MoveKind::Quiet);
        let (a, b, c) = (
            mv(Square::A1, Square::A2),
            mv(Square::B1, Square::B2),
            mv(Square::C1, Square::C2),
        );
        let mut killers = Killers::NONE;
        assert_eq!(killers, Killers::default());
        assert_eq!(killers.key(a), None);
        killers.remember(a);
        assert_eq!(killers, Killers([Some(a), None]));
        killers.remember(a);
        assert_eq!(killers, Killers([Some(a), None]), "no move twice");
        killers.remember(b);
        assert_eq!(killers, Killers([Some(b), Some(a)]));
        assert_eq!(
            (killers.key(b), killers.key(a), killers.key(c)),
            (Some(1), Some(0), None)
        );
        // The second killer cutting off again changes places with the first.
        killers.remember(a);
        assert_eq!(killers, Killers([Some(a), Some(b)]));
        killers.remember(c);
        assert_eq!(
            killers,
            Killers([Some(c), Some(a)]),
            "the oldest is forgotten"
        );
    }

    /// The killers go between the captures that do not lose material and
    /// the other quiet moves, the newer first, and nothing else moves.
    #[test]
    fn killers_come_after_the_good_captures_and_before_the_other_quiet_moves() {
        // The queen can take a rook safely or a defended pawn at a loss.
        let pos = Position::from_fen("4k3/2p5/1p3r2/8/3Q4/8/8/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&pos);
        let plain = losing_captures_last(&pos, &moves);
        let named = |text: &str| *plain.iter().find(|mv| mv.to_string() == text).unwrap();
        let (wins, loses) = (named("d4f6"), named("d4b6"));
        assert_eq!(plain.as_slice()[0], wins);
        assert_eq!(plain.as_slice()[plain.len() - 1], loses);
        let (older, newer) = (named("e1e2"), named("d4h4"));
        let mut killers = Killers::NONE;
        killers.remember(older);
        killers.remember(newer);
        let ordered = table_move_first(&pos, &moves, None, killers, None);
        assert_eq!(ordered.as_slice()[..3], [wins, newer, older]);
        let rest: Vec<Move> = plain
            .iter()
            .copied()
            .filter(|mv| ![wins, newer, older].contains(mv))
            .collect();
        assert_eq!(ordered.as_slice()[3..], rest[..]);
        // The table's move still comes before everything, also when it is
        // a killer as well, and then it is tried once.
        let both = table_move_first(&pos, &moves, Some(older), killers, None);
        assert_eq!(both.as_slice()[..3], [older, wins, newer]);
        assert_eq!(both.len(), plain.len());
    }

    /// The quiet moves that are not killers go by their history, the best
    /// first, and those the history does not tell apart stay as generated.
    /// No history value moves a quiet move out of its place between the
    /// killers and the captures that lose material.
    #[test]
    fn the_other_quiet_moves_go_by_their_history() {
        use super::super::history::HISTORY_MAX;
        let pos = Position::from_fen("4k3/2p5/1p3r2/8/3Q4/8/8/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&pos);
        let plain = losing_captures_last(&pos, &moves);
        let named = |text: &str| *plain.iter().find(|mv| mv.to_string() == text).unwrap();
        let (wins, loses) = (named("d4f6"), named("d4b6"));
        let (killer, good, better, bad) =
            (named("e1e2"), named("d4h4"), named("d4a1"), named("e1d1"));
        let mut killers = Killers::NONE;
        killers.remember(killer);
        let side = pos.side_to_move();
        let mut history = History::new();
        // As far up and down as history goes.
        for _ in 0..1_000 {
            history.reward(side, better, 1_200);
            history.punish(side, bad, 1_200);
            // The killer and the captures are not ordered by history.
            history.punish(side, killer, 1_200);
            history.punish(side, wins, 1_200);
            history.reward(side, loses, 1_200);
        }
        history.reward(side, good, 16);
        assert!(history.value(side, better) > HISTORY_MAX - 1_200);
        // The other side's history does not count.
        history.reward(side.flip(), bad, 1_200);

        let ordered = table_move_first(&pos, &moves, None, killers, Some(&history));
        let last = ordered.len() - 1;
        assert_eq!(ordered.as_slice()[..4], [wins, killer, better, good]);
        assert_eq!(ordered.as_slice()[last - 1..], [bad, loses]);
        // Everything between has no history and stands as generated.
        let untouched: Vec<Move> = plain
            .iter()
            .copied()
            .filter(|mv| ![wins, killer, better, good, bad, loses].contains(mv))
            .collect();
        assert_eq!(ordered.as_slice()[4..last - 1], untouched[..]);
        // Without a history the quiet moves stand as generated.
        let mut plain_with_killer = Killers::NONE;
        plain_with_killer.remember(killer);
        let unsorted = table_move_first(&pos, &moves, None, plain_with_killer, None);
        let rest: Vec<Move> = plain
            .iter()
            .copied()
            .filter(|mv| ![wins, killer].contains(mv))
            .collect();
        assert_eq!(unsorted.as_slice()[2..], rest[..]);
    }

    /// A killer comes from another position: it counts only where it is a
    /// quiet move of this one.
    #[test]
    fn a_killer_that_is_no_quiet_move_here_changes_nothing() {
        let pos = Position::from_fen("4k3/2p5/1p3r2/8/3Q4/8/8/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&pos);
        let plain = losing_captures_last(&pos, &moves);
        let mut killers = Killers::NONE;
        // A move this position does not have (no piece stands on a1), and
        // the same squares as the losing capture, which another position
        // had as a quiet move: here the move is a capture, another move.
        killers.remember(Move::new(Square::A1, Square::H8, MoveKind::Quiet));
        killers.remember(Move::new(Square::D4, Square::B6, MoveKind::Quiet));
        assert_eq!(table_move_first(&pos, &moves, None, killers, None), plain);
        // Quiet means neither a capture nor a promotion.
        let named = |text: &str| *plain.iter().find(|mv| mv.to_string() == text).unwrap();
        assert!(is_quiet(&pos, named("e1e2")));
        assert!(!is_quiet(&pos, named("d4f6")));
        assert!(!is_quiet(&pos, named("d4b6")));
        let promotes = Position::from_fen("7k/2P5/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        for mv in &pseudo_legal_moves(&promotes) {
            assert_eq!(is_quiet(&promotes, *mv), mv.promotion().is_none(), "{mv}");
        }
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

    /// The main search's order: the queen takes the queen first, then come
    /// the quiet moves, and last the two captures that lose the queen (the
    /// rook and the pawn are both defended by the black queen), the rook
    /// before the pawn.
    #[test]
    fn captures_that_lose_material_come_last() {
        let pos = Position::from_fen("4k3/8/1q3r2/8/3Q4/8/1p6/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&pos);
        let ordered: Vec<String> = losing_captures_last(&pos, &moves)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(ordered.len(), moves.len());
        assert_eq!(ordered[0], "d4b6");
        assert_eq!(ordered[ordered.len() - 2..], ["d4f6", "d4b2"]);
        let quiet = &ordered[1..ordered.len() - 2];
        let by_victim = captures_first(&pos, &moves);
        let generated: Vec<String> = by_victim.as_slice()[3..]
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(quiet, generated);
        // Without the black queen nothing is defended and nothing loses:
        // the order is that of the victims.
        let free = Position::from_fen("4k3/8/5r2/8/3Q4/8/1p6/4K3 w - - 0 1").unwrap();
        let moves = pseudo_legal_moves(&free);
        assert_eq!(
            losing_captures_last(&free, &moves),
            captures_first(&free, &moves)
        );
    }

    #[test]
    fn losing_material_is_a_matter_of_the_whole_exchange() {
        let loses = |fen: &str, text: &str| {
            let pos = Position::from_fen(fen).unwrap();
            let moves = pseudo_legal_moves(&pos);
            let mv = *moves.iter().find(|mv| mv.to_string() == text).unwrap();
            loses_material(&pos, mv)
        };
        // Rook takes a defended pawn, and the same pawn undefended.
        assert!(loses("4k3/8/4p3/3p4/8/8/8/3RK3 w - - 0 1", "d1d5"));
        assert!(!loses("4k3/8/8/3p4/8/8/8/3RK3 w - - 0 1", "d1d5"));
        // An even trade and a winning one do not lose, defended or not.
        assert!(!loses("4k3/8/4p3/3n4/8/5B2/8/4K3 w - - 0 1", "f3d5"));
        assert!(!loses("4k3/8/4p3/3r4/8/4N3/8/4K3 w - - 0 1", "e3d5"));
        // A second attacker behind the first makes the capture good.
        assert!(!loses("4k3/8/4p3/3p4/4P3/5B2/8/4K3 w - - 0 1", "e4d5"));
        // A promotion on a square where the new queen is simply taken.
        assert!(loses("1r6/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7d8q"));
        assert!(!loses("8/3P4/8/8/8/8/8/k3K3 w - - 0 1", "d7d8q"));
        // En passant met by a recapture is even.
        assert!(!loses("4k3/2p5/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6"));
        // A quiet move is not asked about by the search, but has an answer.
        assert!(loses("4k3/8/4p3/8/8/8/8/3RK3 w - - 0 1", "d1d5"));
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
            let by_exchange = losing_captures_last(&pos, &generated);
            assert_eq!(by_exchange.len(), generated.len());
            assert!(generated.iter().all(|mv| by_exchange.contains(mv)));
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
        assert_eq!(losing_captures_last(&start, &generated), generated);
    }
}
