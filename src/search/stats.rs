// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Counts of what a search did, for judging move ordering and pruning.
//!
//! The search only ever adds to these numbers. It never reads them, so they
//! cannot change which positions it visits: the node count of a search is
//! the same whether anyone looks at its statistics or not.

/// What one search counted, from its start to now. Every count starts at 0
/// with each new search.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchStats {
    /// Beta cutoffs in the main search: positions where a move was good
    /// enough to show that the opponent would avoid the position, so the
    /// remaining moves were skipped.
    pub cutoffs: u64,
    /// Those of them where the very first legal move tried did it. With
    /// perfect move ordering that would be all of them; the share is the
    /// usual measure of how good the ordering is.
    pub first_move_cutoffs: u64,
    /// Positions visited by the quiescence search, out of the search's
    /// total node count.
    pub quiescence_nodes: u64,
    /// Quiescence positions that were good enough as they stood ("standing
    /// pat" at or above beta), without trying a move.
    pub stand_pat_cutoffs: u64,
    /// Beta cutoffs by a move in the quiescence search.
    pub quiescence_cutoffs: u64,
    /// Those of them where the first legal move tried did it.
    pub quiescence_first_move_cutoffs: u64,
}

impl SearchStats {
    /// All counts at 0.
    pub const ZERO: SearchStats = SearchStats {
        cutoffs: 0,
        first_move_cutoffs: 0,
        quiescence_nodes: 0,
        stand_pat_cutoffs: 0,
        quiescence_cutoffs: 0,
        quiescence_first_move_cutoffs: 0,
    };

    /// How many in a thousand of the main search's cutoffs came from the
    /// first move tried, rounded down, or `None` if there was no cutoff.
    #[must_use]
    pub const fn first_move_rate(&self) -> Option<u64> {
        per_mille(self.first_move_cutoffs, self.cutoffs)
    }

    /// The same for the quiescence search's cutoffs by a move.
    #[must_use]
    pub const fn quiescence_first_move_rate(&self) -> Option<u64> {
        per_mille(self.quiescence_first_move_cutoffs, self.quiescence_cutoffs)
    }
}

/// `part` in thousandths of `whole`, rounded down, or `None` for a `whole`
/// of 0. Whole numbers only, so that printed figures are the same on every
/// platform.
#[must_use]
pub const fn per_mille(part: u64, whole: u64) -> Option<u64> {
    if whole == 0 {
        None
    } else {
        Some((part as u128 * 1000 / whole as u128) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_the_default() {
        assert_eq!(SearchStats::ZERO, SearchStats::default());
    }

    #[test]
    fn rates_are_thousandths_rounded_down() {
        assert_eq!(per_mille(0, 0), None);
        assert_eq!(per_mille(0, 7), Some(0));
        assert_eq!(per_mille(1, 3), Some(333));
        assert_eq!(per_mille(2, 3), Some(666));
        assert_eq!(per_mille(7, 7), Some(1000));
        // No overflow on the way.
        assert_eq!(per_mille(u64::MAX, u64::MAX), Some(1000));
        let stats = SearchStats {
            cutoffs: 8,
            first_move_cutoffs: 6,
            quiescence_cutoffs: 0,
            ..SearchStats::ZERO
        };
        assert_eq!(stats.first_move_rate(), Some(750));
        assert_eq!(stats.quiescence_first_move_rate(), None);
    }
}
