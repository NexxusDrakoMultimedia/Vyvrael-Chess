// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! A fixed-capacity list of moves that lives on the stack.

use std::fmt;
use std::ops::Deref;

use crate::board::{Move, MoveKind, Square};

/// A list of moves with room for [`MoveList::CAPACITY`] entries and no heap
/// allocation.
///
/// Move generation runs millions of times per second during search, so the
/// list is a plain array on the stack. The capacity is comfortably above the
/// largest number of moves any chess position can have (218 legal moves is
/// the known maximum).
///
/// It dereferences to a slice, so `len()`, `iter()`, `contains()` and
/// indexing all work.
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; MoveList::CAPACITY],
    len: usize,
}

impl MoveList {
    /// The most moves a list can hold.
    pub const CAPACITY: usize = 256;

    /// Filler for unused slots; never visible through the public interface.
    const FILLER: Move = Move::new(Square::A1, Square::A1, MoveKind::Quiet);

    /// Creates an empty list.
    #[must_use]
    pub const fn new() -> MoveList {
        MoveList {
            moves: [MoveList::FILLER; MoveList::CAPACITY],
            len: 0,
        }
    }

    /// Adds a move to the end of the list.
    ///
    /// # Panics
    ///
    /// Panics if the list is already full.
    pub fn push(&mut self, mv: Move) {
        assert!(self.len < MoveList::CAPACITY, "MoveList is full");
        self.moves[self.len] = mv;
        self.len += 1;
    }

    /// The moves as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

impl Default for MoveList {
    fn default() -> MoveList {
        MoveList::new()
    }
}

impl Deref for MoveList {
    type Target = [Move];

    fn deref(&self) -> &[Move] {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl fmt::Debug for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl PartialEq for MoveList {
    /// Lists are equal when they hold the same moves in the same order;
    /// unused slots are ignored.
    fn eq(&self, other: &MoveList) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for MoveList {}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::Quiet)
    }

    #[test]
    fn starts_empty() {
        let list = MoveList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.as_slice(), &[]);
        assert_eq!(MoveList::default(), list);
    }

    #[test]
    fn push_keeps_order_and_slice_methods_work() {
        let (a, b, c) = (
            mv(Square::E2, Square::E4),
            mv(Square::G1, Square::F3),
            mv(Square::A1, Square::A1),
        );
        let mut list = MoveList::new();
        list.push(a);
        list.push(b);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], a);
        assert_eq!(list.as_slice(), &[a, b]);
        assert!(list.contains(&b));
        // The filler value is a real move and can be stored like any other.
        assert!(!list.contains(&c));
        list.push(c);
        assert!(list.contains(&c));
        let collected: Vec<Move> = (&list).into_iter().copied().collect();
        assert_eq!(collected, [a, b, c]);
        assert_eq!(format!("{list:?}").matches("Move(").count(), 3);
    }

    #[test]
    fn equality_ignores_unused_slots() {
        let mut a = MoveList::new();
        let mut b = MoveList::new();
        a.push(mv(Square::E2, Square::E4));
        b.push(mv(Square::E2, Square::E4));
        assert_eq!(a, b);
        b.push(mv(Square::D2, Square::D4));
        assert_ne!(a, b);
    }

    #[test]
    fn holds_exactly_capacity_moves() {
        let mut list = MoveList::new();
        for _ in 0..MoveList::CAPACITY {
            list.push(mv(Square::E2, Square::E4));
        }
        assert_eq!(list.len(), MoveList::CAPACITY);
    }

    #[test]
    #[should_panic(expected = "MoveList is full")]
    fn pushing_past_capacity_panics() {
        let mut list = MoveList::new();
        for _ in 0..=MoveList::CAPACITY {
            list.push(mv(Square::E2, Square::E4));
        }
    }
}
