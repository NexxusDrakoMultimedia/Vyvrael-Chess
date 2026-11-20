// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Limits on how long a search may run.

use std::time::Instant;

/// When a search should stop. Any combination may be set; the search stops
/// at whichever comes first. With nothing set it runs until it is told to
/// stop or reaches the maximum depth.
///
/// For reproducible results use `depth` or `nodes`, never `deadline`: how
/// far a search gets in a given time depends on the machine and on what else
/// it is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Limits {
    /// Search no deeper than this many plies.
    pub depth: Option<u32>,
    /// Visit no more than about this many positions. The search stops at the
    /// first position it would visit beyond the limit.
    pub nodes: Option<u64>,
    /// Stop once this moment has passed. Checked every 1,024 positions, so
    /// the search overruns it slightly.
    pub deadline: Option<Instant>,
}

impl Limits {
    /// No limits: search until stopped.
    pub const NONE: Limits = Limits {
        depth: None,
        nodes: None,
        deadline: None,
    };

    /// Search to exactly this depth.
    #[must_use]
    pub const fn depth(depth: u32) -> Limits {
        Limits {
            depth: Some(depth),
            ..Limits::NONE
        }
    }

    /// Search about this many positions.
    #[must_use]
    pub const fn nodes(nodes: u64) -> Limits {
        Limits {
            nodes: Some(nodes),
            ..Limits::NONE
        }
    }

    /// Search until `deadline`.
    #[must_use]
    pub const fn until(deadline: Instant) -> Limits {
        Limits {
            deadline: Some(deadline),
            ..Limits::NONE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_one_limit_each() {
        assert_eq!(Limits::default(), Limits::NONE);
        assert_eq!(Limits::depth(5).depth, Some(5));
        assert_eq!(Limits::depth(5).nodes, None);
        assert_eq!(Limits::nodes(1000).nodes, Some(1000));
        assert_eq!(Limits::nodes(1000).deadline, None);
        let soon = Instant::now();
        assert_eq!(Limits::until(soon).deadline, Some(soon));
        assert_eq!(Limits::until(soon).depth, None);
    }

    #[test]
    fn limits_can_be_combined() {
        let both = Limits {
            nodes: Some(10),
            ..Limits::depth(3)
        };
        assert_eq!(
            (both.depth, both.nodes, both.deadline),
            (Some(3), Some(10), None)
        );
    }
}
