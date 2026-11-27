// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The transposition table: what the search has already found out about
//! positions, kept so that it need not be found out again.
//!
//! The same position is reached by many move orders, and by every iteration
//! of iterative deepening. The table remembers, for a position's hash, how
//! deep it was searched, what came out, and which move was best. A later
//! visit that needs no more depth than that can use the result at once.
//!
//! A result is not always the position's value. Alpha-beta stops looking as
//! soon as it knows enough, so what it knows is often only a bound, and the
//! table says which ([`Bound`]).
//!
//! The table is a flat array indexed by the low bits of the hash, one
//! entry to a slot. Two positions can want the same slot; the newer result
//! takes it, unless the older one is of the same position and searched
//! deeper. Each slot keeps the whole 64-bit hash, so a position is only
//! ever handed its own entry (up to the one-in-2^64 chance of two positions
//! sharing a hash, which the search tolerates like every engine does).

use crate::board::Move;
use crate::eval::Score;

use super::{MATE_BOUND, MAX_PLY};

/// What a stored score says about the position's true value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// The score is the value: the search looked at everything it needed.
    Exact,
    /// The value is at least the score: a move was good enough to end the
    /// search there (a beta cutoff), and another might be better still.
    Lower,
    /// The value is at most the score: no move reached the lower edge of
    /// the window (alpha), so all that is known is that none is better.
    Upper,
}

/// What the table knows about one position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// How many plies below the position the search went.
    pub depth: u32,
    /// The score, for the side to move, as the node that stored it saw it:
    /// see [`TranspositionTable::store`] about mate scores.
    pub score: Score,
    /// What the score says about the value.
    pub bound: Bound,
    /// The best move found, if one stood out.
    pub best: Option<Move>,
}

/// One slot. Sixteen bytes, so a mebibyte holds 65,536 of them.
#[derive(Clone, Copy)]
struct Slot {
    key: u64,
    best: Option<Move>,
    score: i16,
    depth: u8,
    /// `None` marks a slot that holds nothing: a hash of 0 is a hash like
    /// any other and cannot serve.
    bound: Option<Bound>,
}

const _: () = assert!(std::mem::size_of::<Slot>() == 16);
// Scores and depths fit the fields they are stored in.
const _: () = assert!(super::INFINITY <= i16::MAX as Score);
const _: () = assert!(MAX_PLY <= u8::MAX as usize);

const EMPTY: Slot = Slot {
    key: 0,
    best: None,
    score: 0,
    depth: 0,
    bound: None,
};

/// The table. See the module's description.
pub struct TranspositionTable {
    slots: Vec<Slot>,
}

impl TranspositionTable {
    /// A table of at most `mebibytes` MiB (and at least one slot), empty.
    /// The number of slots is a power of two, so a size that is not one is
    /// rounded down.
    ///
    /// Returns `None` if the memory cannot be had, so that asking for more
    /// than the machine has is an answer and not a crash.
    #[must_use]
    pub fn new(mebibytes: u32) -> Option<TranspositionTable> {
        let wanted = (u64::from(mebibytes) << 20) / std::mem::size_of::<Slot>() as u64;
        let count = usize::try_from(wanted.max(1)).ok()?;
        // The largest power of two that is not more than `count`.
        let count = 1usize << count.ilog2();
        let mut slots = Vec::new();
        slots.try_reserve_exact(count).ok()?;
        slots.resize(count, EMPTY);
        Some(TranspositionTable { slots })
    }

    /// How many positions the table can hold.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Never true: a table has at least one slot. Here because a type with
    /// `len` is expected to say.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Forgets everything, as at the start of a new game.
    pub fn clear(&mut self) {
        self.slots.fill(EMPTY);
    }

    fn index(&self, key: u64) -> usize {
        // The length is a power of two, so this keeps the low bits.
        (key as usize) & (self.slots.len() - 1)
    }

    /// What is known about the position with hash `key`, `ply` plies from
    /// the root of the search that asks.
    #[must_use]
    pub fn probe(&self, key: u64, ply: usize) -> Option<Entry> {
        let slot = &self.slots[self.index(key)];
        let bound = slot.bound?;
        (slot.key == key).then(|| Entry {
            depth: u32::from(slot.depth),
            score: from_table(Score::from(slot.score), ply),
            bound,
            best: slot.best,
        })
    }

    /// Records the result of searching the position with hash `key`
    /// `depth` plies deep, found `ply` plies from the root.
    ///
    /// A mate score counts plies from the root of the search that found it
    /// ("mate in 5 from the root"), which means something else in another
    /// search that meets the position at another distance. So it is stored
    /// as the distance from the position itself and turned back by
    /// [`probe`](Self::probe) for whoever asks.
    ///
    /// The slot is taken unless it already holds this same position
    /// searched deeper: the deeper result is worth more than the new one.
    pub fn store(
        &mut self,
        key: u64,
        ply: usize,
        depth: u32,
        score: Score,
        bound: Bound,
        best: Option<Move>,
    ) {
        let index = self.index(key);
        let slot = &mut self.slots[index];
        let depth = depth.min(u32::from(u8::MAX)) as u8;
        if slot.bound.is_some() && slot.key == key && slot.depth > depth {
            return;
        }
        *slot = Slot {
            key,
            // Keep the move already known for the position when the new
            // result has none to offer.
            best: best.or(if slot.key == key { slot.best } else { None }),
            score: to_table(score, ply) as i16,
            depth,
            bound: Some(bound),
        };
    }
}

/// A score as the table keeps it: a mate score counted from the position
/// instead of from the root.
const fn to_table(score: Score, ply: usize) -> Score {
    if score >= MATE_BOUND {
        score + ply as Score
    } else if score <= -MATE_BOUND {
        score - ply as Score
    } else {
        score
    }
}

/// The reverse of [`to_table`] for a search that is `ply` plies from its
/// root.
const fn from_table(score: Score, ply: usize) -> Score {
    if score >= MATE_BOUND {
        score - ply as Score
    } else if score <= -MATE_BOUND {
        score + ply as Score
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{MoveKind, Square};
    use crate::search::{mate_in, mated_in};

    fn mv(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::Quiet)
    }

    #[test]
    fn sizes() {
        // 65,536 slots to the mebibyte, and a power of two always.
        assert_eq!(TranspositionTable::new(1).unwrap().len(), 65_536);
        assert_eq!(TranspositionTable::new(16).unwrap().len(), 1 << 20);
        assert_eq!(TranspositionTable::new(3).unwrap().len(), 2 * 65_536);
        // Nothing asked for is still a table.
        let least = TranspositionTable::new(0).unwrap();
        assert_eq!(least.len(), 1);
        assert!(!least.is_empty());
        // More than any machine has is refused, not died of.
        assert!(TranspositionTable::new(u32::MAX).is_none());
    }

    #[test]
    fn what_is_stored_is_found_and_nothing_else() {
        let mut table = TranspositionTable::new(1).unwrap();
        assert_eq!(table.probe(42, 0), None);
        // A hash of 0 is not mistaken for an empty slot, nor found in one.
        assert_eq!(table.probe(0, 0), None);
        let best = Some(mv(Square::E2, Square::E4));
        table.store(42, 3, 5, 17, Bound::Lower, best);
        let entry = Entry {
            depth: 5,
            score: 17,
            bound: Bound::Lower,
            best,
        };
        assert_eq!(table.probe(42, 9), Some(entry));
        table.store(0, 0, 1, -3, Bound::Exact, None);
        assert_eq!(table.probe(0, 0).unwrap().score, -3);
        // Another position that wants the same slot is not given this one.
        let same_slot = 42 + table.len() as u64;
        assert_eq!(table.probe(same_slot, 0), None);
        table.clear();
        assert_eq!(table.probe(42, 0), None);
        assert_eq!(table.probe(0, 0), None);
    }

    #[test]
    fn replacement() {
        let mut table = TranspositionTable::new(1).unwrap();
        let (first, second) = (mv(Square::E2, Square::E4), mv(Square::D2, Square::D4));
        table.store(7, 0, 6, 10, Bound::Exact, Some(first));
        // The same position searched less deep does not displace it.
        table.store(7, 0, 4, 99, Bound::Upper, Some(second));
        assert_eq!(table.probe(7, 0).unwrap().score, 10);
        // As deep or deeper does, and a result without a move keeps the
        // move that was known.
        table.store(7, 0, 6, 20, Bound::Upper, None);
        let entry = table.probe(7, 0).unwrap();
        assert_eq!(
            (entry.score, entry.bound, entry.best),
            (20, Bound::Upper, Some(first))
        );
        table.store(7, 0, 8, 30, Bound::Lower, Some(second));
        assert_eq!(table.probe(7, 0).unwrap().best, Some(second));
        // Another position takes the slot whatever the depths, and does
        // not inherit the move.
        let other = 7 + table.len() as u64;
        table.store(other, 0, 1, -5, Bound::Exact, None);
        assert_eq!(table.probe(7, 0), None);
        assert_eq!(table.probe(other, 0).unwrap().best, None);
    }

    /// A mate found three plies below a position that is four plies from
    /// the root is a mate seven plies from the root. Met again two plies
    /// from another root, it is a mate five plies from that one.
    #[test]
    fn mate_scores_are_kept_as_distances_from_the_position() {
        let mut table = TranspositionTable::new(1).unwrap();
        table.store(1, 4, 9, mate_in(7), Bound::Exact, None);
        assert_eq!(table.probe(1, 4).unwrap().score, mate_in(7));
        assert_eq!(table.probe(1, 2).unwrap().score, mate_in(5));
        table.store(2, 4, 9, mated_in(8), Bound::Exact, None);
        assert_eq!(table.probe(2, 10).unwrap().score, mated_in(14));
        // Ordinary scores are not touched, and the deepest ply fits.
        table.store(3, MAX_PLY, MAX_PLY as u32, -250, Bound::Upper, None);
        let entry = table.probe(3, 0).unwrap();
        assert_eq!((entry.score, entry.depth), (-250, MAX_PLY as u32));
        table.store(4, MAX_PLY, 1, mated_in(MAX_PLY), Bound::Exact, None);
        assert_eq!(table.probe(4, MAX_PLY).unwrap().score, mated_in(MAX_PLY));
    }
}
