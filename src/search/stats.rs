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

/// The statistics of a search as lines of text, each starting with
/// `stats`:
///
/// ```text
/// stats depth 1 nodes 21
/// stats depth 2 nodes 222 ebf 10.57
/// stats depth 3 nodes 2728 ebf 12.28
/// stats cutoffs 180 firstmove 126 rate 70.0%
/// stats qnodes 2500 of 2971 (84.1%) standpat 1900 qcutoffs 90 firstmove 81 rate 90.0%
/// ```
///
/// `iterations` holds each completed depth with the node count the search
/// had reached by then, as its reports give them. A `depth` line gives the
/// nodes that iteration took by itself, and `ebf` (the effective branching
/// factor) how many times more that was than the iteration before. The
/// last two lines are the counts of `stats` and the first-move rates;
/// `total_nodes` is the whole search's node count, which also includes an
/// iteration that a limit interrupted and that has no `depth` line.
///
/// Only whole-number arithmetic is used, so the text is the same on every
/// platform. A rate with nothing to divide by is shown as `-`.
#[must_use]
pub fn stats_lines(
    iterations: &[(u32, u64)],
    total_nodes: u64,
    stats: &SearchStats,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(iterations.len() + 2);
    let (mut before, mut previous) = (0, None);
    for &(depth, reached) in iterations {
        let nodes = reached.saturating_sub(before);
        let mut line = format!("stats depth {depth} nodes {nodes}");
        if let Some(hundredths) = previous.and_then(|earlier| ratio(nodes, earlier)) {
            line.push_str(&format!(
                " ebf {}.{:02}",
                hundredths / 100,
                hundredths % 100
            ));
        }
        lines.push(line);
        (before, previous) = (reached, Some(nodes));
    }
    lines.push(format!(
        "stats cutoffs {} firstmove {} rate {}",
        stats.cutoffs,
        stats.first_move_cutoffs,
        percent(stats.first_move_rate())
    ));
    lines.push(format!(
        "stats qnodes {} of {total_nodes} ({}) standpat {} qcutoffs {} firstmove {} rate {}",
        stats.quiescence_nodes,
        percent(per_mille(stats.quiescence_nodes, total_nodes)),
        stats.stand_pat_cutoffs,
        stats.quiescence_cutoffs,
        stats.quiescence_first_move_cutoffs,
        percent(stats.quiescence_first_move_rate())
    ));
    lines
}

/// `part` in hundredths of `whole`, rounded down, or `None` for a `whole`
/// of 0.
fn ratio(part: u64, whole: u64) -> Option<u128> {
    (whole != 0).then(|| u128::from(part) * 100 / u128::from(whole))
}

/// Thousandths as a percentage with one decimal, `-` for none.
fn percent(thousandths: Option<u64>) -> String {
    thousandths.map_or_else(
        || "-".to_owned(),
        |value| format!("{}.{}%", value / 10, value % 10),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_of_a_search() {
        let stats = SearchStats {
            cutoffs: 180,
            first_move_cutoffs: 126,
            quiescence_nodes: 2_500,
            stand_pat_cutoffs: 1_900,
            quiescence_cutoffs: 90,
            quiescence_first_move_cutoffs: 81,
        };
        // Nodes reached by the end of depths 1 to 3, and a total that
        // includes part of an interrupted fourth.
        let lines = stats_lines(&[(1, 21), (2, 243), (3, 2_971)], 3_000, &stats);
        assert_eq!(
            lines,
            [
                "stats depth 1 nodes 21",
                "stats depth 2 nodes 222 ebf 10.57",
                "stats depth 3 nodes 2728 ebf 12.28",
                "stats cutoffs 180 firstmove 126 rate 70.0%",
                "stats qnodes 2500 of 3000 (83.3%) standpat 1900 qcutoffs 90 firstmove 81 rate 90.0%",
            ]
        );
    }

    /// Nothing to divide by: no branching factor, and `-` for a rate.
    #[test]
    fn the_lines_of_a_search_that_found_nothing_to_do() {
        assert_eq!(
            stats_lines(&[(1, 1)], 1, &SearchStats::ZERO),
            [
                "stats depth 1 nodes 1",
                "stats cutoffs 0 firstmove 0 rate -",
                "stats qnodes 0 of 1 (0.0%) standpat 0 qcutoffs 0 firstmove 0 rate -",
            ]
        );
        assert_eq!(stats_lines(&[], 0, &SearchStats::ZERO).len(), 2);
        // An iteration that added no node gives the next one no factor.
        let flat = stats_lines(&[(1, 5), (2, 5), (3, 9)], 9, &SearchStats::ZERO);
        assert_eq!(flat[1], "stats depth 2 nodes 0 ebf 0.00");
        assert_eq!(flat[2], "stats depth 3 nodes 4");
    }

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
