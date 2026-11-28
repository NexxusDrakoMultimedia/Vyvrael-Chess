// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The history heuristic: which quiet moves have been cutting the search
//! off, wherever in the tree they were played.
//!
//! A quiet move that refutes many of the opponent's tries is probably a
//! good move in general in the position being searched: the knight
//! belongs on that outpost in most of the lines. The killer moves
//! remember two such moves per ply; the history table remembers all of
//! them, by the side that moves and the squares moved between, with a
//! number that says how well the move has done. The main search tries the
//! quiet moves that are not killers in the order of that number.

use crate::board::{Color, Move};

/// No history value ever leaves `-HISTORY_MAX..=HISTORY_MAX`.
pub(super) const HISTORY_MAX: i32 = 16_384;

/// The largest step a single cutoff makes.
const BONUS_MAX: i32 = 1_200;

/// How much a cutoff at `depth` plies from the horizon counts: more the
/// deeper it is, because a deep cutoff saved a large subtree and came from
/// a search that saw more.
pub(super) fn bonus(depth: u32) -> i32 {
    let depth = i32::try_from(depth).unwrap_or(i32::MAX);
    depth
        .saturating_mul(depth)
        .saturating_mul(16)
        .min(BONUS_MAX)
}

/// How well each quiet move has done in the current search: one number per
/// side to move, origin square and destination square (the "butterfly"
/// table).
///
/// It belongs to one search, through all its iterations, and is emptied
/// when the next begins, as the killer moves are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct History {
    values: Box<[[[i32; 64]; 64]; 2]>,
}

impl History {
    /// A table that knows nothing: every move is worth 0.
    pub(super) fn new() -> History {
        History {
            values: Box::new([[[0; 64]; 64]; 2]),
        }
    }

    /// Forgets everything.
    pub(super) fn clear(&mut self) {
        *self.values = [[[0; 64]; 64]; 2];
    }

    /// How well `mv` has done for `side`: positive for a move that cuts
    /// off, negative for one that was tried before the move that did.
    pub(super) fn value(&self, side: Color, mv: Move) -> i32 {
        self.values[side.index()][mv.from().index()][mv.to().index()]
    }

    /// Notes that `mv` cut off the search for `side`, by `step` (see
    /// [`bonus`]).
    pub(super) fn reward(&mut self, side: Color, mv: Move, step: i32) {
        self.add(side, mv, step);
    }

    /// Notes that `mv` was tried for `side` and did not cut off where a
    /// later quiet move did, by `step`.
    pub(super) fn punish(&mut self, side: Color, mv: Move, step: i32) {
        self.add(side, mv, -step);
    }

    /// Moves the value towards `HISTORY_MAX` (or its negative) by `step`,
    /// less the further it has already gone ("history gravity"): the value
    /// can approach the limit but not pass it, and what was learned long
    /// ago fades as new results come in.
    fn add(&mut self, side: Color, mv: Move, step: i32) {
        let value = &mut self.values[side.index()][mv.from().index()][mv.to().index()];
        *value += step - *value * step.abs() / HISTORY_MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{MoveKind, Square};

    fn quiet(from: Square, to: Square) -> Move {
        Move::new(from, to, MoveKind::Quiet)
    }

    #[test]
    fn the_bonus_grows_with_depth_up_to_its_limit() {
        assert_eq!(bonus(0), 0);
        assert_eq!(bonus(1), 16);
        assert_eq!(bonus(2), 64);
        assert_eq!(bonus(5), 400);
        assert_eq!(bonus(8), 1_024);
        assert_eq!(bonus(9), BONUS_MAX);
        assert_eq!(bonus(64), BONUS_MAX);
        assert_eq!(bonus(u32::MAX), BONUS_MAX);
    }

    #[test]
    fn a_move_is_known_by_side_and_squares() {
        let mut history = History::new();
        let mv = quiet(Square::G1, Square::F3);
        assert_eq!(history.value(Color::White, mv), 0);
        history.reward(Color::White, mv, 64);
        assert_eq!(history.value(Color::White, mv), 64);
        // Not the other side's move, the move back, or another move.
        assert_eq!(history.value(Color::Black, mv), 0);
        assert_eq!(
            history.value(Color::White, quiet(Square::F3, Square::G1)),
            0
        );
        assert_eq!(
            history.value(Color::White, quiet(Square::G1, Square::H3)),
            0
        );
        history.punish(Color::Black, mv, 64);
        assert_eq!(history.value(Color::Black, mv), -64);
        assert_eq!(history.value(Color::White, mv), 64);
        history.clear();
        assert_eq!(history, History::new());
    }

    #[test]
    fn a_reward_and_a_punishment_nearly_cancel() {
        let mut history = History::new();
        let mv = quiet(Square::E2, Square::E4);
        history.reward(Color::White, mv, 400);
        assert_eq!(history.value(Color::White, mv), 400);
        // 400 - 400 - 400 * 400 / 16384: the punishment weighs a little
        // more on a move that stood well.
        history.punish(Color::White, mv, 400);
        assert_eq!(history.value(Color::White, mv), -9);
    }

    /// However often a move is rewarded or punished, its value stays
    /// within the limit, and it gets there.
    #[test]
    fn values_approach_the_limit_and_never_pass_it() {
        let mut history = History::new();
        let (good, bad) = (quiet(Square::B1, Square::C3), quiet(Square::A2, Square::A3));
        let mut last = 0;
        for _ in 0..1_000 {
            history.reward(Color::White, good, BONUS_MAX);
            history.punish(Color::White, bad, BONUS_MAX);
            let value = history.value(Color::White, good);
            assert!(value >= last && value <= HISTORY_MAX);
            assert_eq!(history.value(Color::White, bad), -value);
            last = value;
        }
        assert!(last > HISTORY_MAX - BONUS_MAX, "{last}");
        // A move at the top can still come down.
        history.punish(Color::White, good, BONUS_MAX);
        assert!(history.value(Color::White, good) < last - BONUS_MAX);
    }
}
