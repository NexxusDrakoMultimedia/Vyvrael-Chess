// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Time management: from what `go` says about the clock to how long the
//! search may think about this move.
//!
//! The scheme is deliberately plain. It gives every move an equal share of
//! the time left plus most of the increment, and never promises more than
//! is on the clock. Smarter schemes (stopping early when the best move is
//! stable, thinking longer when the score drops) change how strongly the
//! engine plays, so they wait for milestone 7 and an SPRT each.

use std::time::{Duration, Instant};

use super::GoParams;
use crate::board::Color;
use crate::search::Limits;

/// Time kept back on every move for what the engine cannot measure: the
/// GUI's own delay, the operating system, and the search noticing its
/// deadline only every thousand positions.
pub const MOVE_OVERHEAD: Duration = Duration::from_millis(30);

/// When the GUI does not say how many moves remain to the next time
/// control, the remaining time is spread over this many.
pub const MOVES_ASSUMED: u64 = 30;

/// How long to think about this move, or `None` when `go` set no clock and
/// no move time (the search then runs to its depth or node limit, or until
/// `stop`).
///
/// - `movetime` is used as given, less the overhead.
/// - With a clock, the share is `remaining / moves + 3/4 increment`, where
///   `moves` is `movestogo` or [`MOVES_ASSUMED`]. It is capped at what is on
///   the clock less the overhead, so the engine cannot plan to lose on time.
/// - There is always at least one millisecond: a move must be made even
///   with nothing left.
#[must_use]
pub fn time_for_move(params: &GoParams, side: Color) -> Option<Duration> {
    let floor = Duration::from_millis(1);
    if let Some(movetime) = params.movetime {
        let given = Duration::from_millis(movetime);
        return Some(given.saturating_sub(MOVE_OVERHEAD).max(floor));
    }
    let (remaining, increment) = match side {
        Color::White => (params.wtime?, params.winc.unwrap_or(0)),
        Color::Black => (params.btime?, params.binc.unwrap_or(0)),
    };
    let moves = params.movestogo.map_or(MOVES_ASSUMED, u64::from).max(1);
    let share = (remaining / moves).saturating_add(increment / 4 * 3);
    let on_the_clock = Duration::from_millis(remaining).saturating_sub(MOVE_OVERHEAD);
    Some(Duration::from_millis(share).min(on_the_clock).max(floor))
}

/// The search limits for a `go` command received at `now` with `side` to
/// move.
///
/// `depth` and `nodes` are passed on as they are. `infinite` switches the
/// clock off: only `stop` (or a depth or node limit sent with it) ends the
/// search.
#[must_use]
pub fn limits_for(params: &GoParams, side: Color, now: Instant) -> Limits {
    let deadline = if params.infinite {
        None
    } else {
        time_for_move(params, side).map(|time| now + time)
    };
    Limits {
        depth: params.depth,
        nodes: params.nodes,
        deadline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn clock(wtime: u64, btime: u64, winc: u64, binc: u64) -> GoParams {
        GoParams {
            wtime: Some(wtime),
            btime: Some(btime),
            winc: Some(winc),
            binc: Some(binc),
            ..GoParams::default()
        }
    }

    #[test]
    fn no_clock_means_no_time_limit() {
        let none = GoParams::default();
        assert_eq!(time_for_move(&none, Color::White), None);
        let depth_only = GoParams {
            depth: Some(5),
            ..GoParams::default()
        };
        assert_eq!(time_for_move(&depth_only, Color::Black), None);
        // Only the other side's clock was sent: nothing to go on.
        let white_only = GoParams {
            wtime: Some(1_000),
            ..GoParams::default()
        };
        assert_eq!(time_for_move(&white_only, Color::Black), None);
    }

    #[test]
    fn move_time_is_used_less_the_overhead() {
        let go = |movetime| GoParams {
            movetime: Some(movetime),
            ..GoParams::default()
        };
        assert_eq!(time_for_move(&go(1_000), Color::White), Some(ms(970)));
        assert_eq!(time_for_move(&go(1_000), Color::Black), Some(ms(970)));
        // Shorter than the overhead: still a millisecond to find a move.
        assert_eq!(time_for_move(&go(10), Color::White), Some(ms(1)));
        assert_eq!(time_for_move(&go(0), Color::White), Some(ms(1)));
        // It takes precedence over a clock sent with it.
        let both = GoParams {
            movetime: Some(500),
            ..clock(60_000, 60_000, 0, 0)
        };
        assert_eq!(time_for_move(&both, Color::White), Some(ms(470)));
    }

    /// Sudden death: clocks and no increments at all, which is how many
    /// GUIs send a game without increment.
    #[test]
    fn a_clock_without_increments() {
        let go = GoParams {
            wtime: Some(60_000),
            btime: Some(30_000),
            ..GoParams::default()
        };
        assert_eq!(time_for_move(&go, Color::White), Some(ms(2_000)));
        assert_eq!(time_for_move(&go, Color::Black), Some(ms(1_000)));
        // The increment of the other side is not borrowed.
        let lopsided = GoParams {
            winc: Some(9_000),
            ..go
        };
        assert_eq!(time_for_move(&lopsided, Color::Black), Some(ms(1_000)));
    }

    #[test]
    fn each_side_uses_its_own_clock() {
        let go = clock(300_000, 60_000, 2_000, 1_000);
        // 300 s / 30 + 3/4 of 2 s.
        assert_eq!(time_for_move(&go, Color::White), Some(ms(11_500)));
        // 60 s / 30 + 3/4 of 1 s.
        assert_eq!(time_for_move(&go, Color::Black), Some(ms(2_750)));
    }

    #[test]
    fn moves_to_go_divides_the_time() {
        let go = |movestogo| GoParams {
            movestogo: Some(movestogo),
            ..clock(60_000, 60_000, 0, 0)
        };
        assert_eq!(time_for_move(&go(40), Color::White), Some(ms(1_500)));
        assert_eq!(time_for_move(&go(2), Color::White), Some(ms(30_000)));
        // The last move before the control may use all that is left, less
        // the overhead; a GUI that sends 0 is treated the same way.
        assert_eq!(time_for_move(&go(1), Color::White), Some(ms(59_970)));
        assert_eq!(time_for_move(&go(0), Color::White), Some(ms(59_970)));
    }

    #[test]
    fn never_more_than_is_on_the_clock() {
        // Almost no time but a large increment: the increment is not on the
        // clock yet, so it cannot be spent.
        let go = clock(200, 200, 5_000, 5_000);
        assert_eq!(time_for_move(&go, Color::White), Some(ms(170)));
        // Less than the overhead left, or a fallen flag: one millisecond.
        assert_eq!(
            time_for_move(&clock(20, 20, 0, 0), Color::White),
            Some(ms(1))
        );
        assert_eq!(time_for_move(&clock(0, 0, 0, 0), Color::Black), Some(ms(1)));
        // Whatever the numbers, the plan fits the clock.
        for remaining in [1, 29, 30, 31, 500, 60_000, 3_600_000, u64::MAX / 4] {
            for increment in [0, 100, 30_000, u64::MAX / 4] {
                for movestogo in [None, Some(1), Some(40)] {
                    let go = GoParams {
                        movestogo,
                        ..clock(remaining, remaining, increment, increment)
                    };
                    let time = time_for_move(&go, Color::White).unwrap();
                    assert!(time >= ms(1));
                    assert!(
                        time <= ms(remaining).max(ms(1)),
                        "{remaining} {increment} {movestogo:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn limits_combine_the_clock_with_depth_and_nodes() {
        let now = Instant::now();
        let go = GoParams {
            depth: Some(7),
            nodes: Some(1_000),
            ..clock(30_000, 30_000, 0, 0)
        };
        assert_eq!(
            limits_for(&go, Color::White, now),
            Limits {
                depth: Some(7),
                nodes: Some(1_000),
                deadline: Some(now + ms(1_000)),
            }
        );
        // No clock: no deadline.
        let depth_only = GoParams {
            depth: Some(3),
            ..GoParams::default()
        };
        assert_eq!(limits_for(&depth_only, Color::Black, now), Limits::depth(3));
        assert_eq!(
            limits_for(&GoParams::default(), Color::White, now),
            Limits::NONE
        );
    }

    #[test]
    fn infinite_switches_the_clock_off() {
        let now = Instant::now();
        let go = GoParams {
            infinite: true,
            movetime: Some(100),
            depth: Some(9),
            ..clock(1_000, 1_000, 0, 0)
        };
        assert_eq!(limits_for(&go, Color::White, now), Limits::depth(9));
    }
}
