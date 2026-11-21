// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Variety: in the opening, sometimes a move a little below the best.
//!
//! An engine that always plays the best move it finds repeats its games:
//! the same position searched to the same depth gives the same move. With
//! the UCI option `Variety` above 0 the engine may instead play, at the
//! root and only in the first [`VARIETY_FULL_MOVES`] full moves, a move
//! whose score is within `Variety` centipawns of the best one, chosen at
//! random and weighted towards the best. It weakens play on purpose, which
//! is why it is off (0) by default and exempt from the SPRT rule.
//!
//! What keeps it safe:
//!
//! - It is never used when the best score is a mate, for or against.
//! - A move is only played after a **second check**: an exact search of it
//!   to the depth the best move was found at, whose score must be within
//!   `Variety` of the best move's. A move whose check a limit cuts short is
//!   never played. Before the check, a cheap quiescence-only score of every
//!   root move picks which moves are worth checking at all.
//! - The best move is always among the choices and is the most likely.
//!
//! This is **strength-affecting randomness**: the generator is handed in by
//! the caller, which seeds it according to the UCI option `Seed`.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use super::{Limits, Outcome, Search, is_mate_score};
use crate::board::{Move, Position};
use crate::eval::{Evaluator, Score};
use crate::movegen::legal_moves;
use crate::rng::Rng;

/// The largest `Variety` that can be asked for, in centipawns.
pub const VARIETY_MAX: u32 = 100;

/// `Variety` applies while the position's full-move number is at most this.
pub const VARIETY_FULL_MOVES: u16 = 12;

/// How far below the best a move's cheap score may be, beyond `Variety`,
/// for it to be checked at all. Quiescence alone misjudges quiet moves, so
/// the cut is generous; the check that follows is exact.
const PREFILTER_MARGIN: Score = 100;

/// Whether `Variety` set to `variety` applies in `pos`: it is above 0 and
/// the game is still in its first [`VARIETY_FULL_MOVES`] full moves.
#[must_use]
pub const fn variety_applies(pos: &Position, variety: u32) -> bool {
    variety > 0 && pos.fullmove_number() <= VARIETY_FULL_MOVES
}

/// Splits a search's limits between the ordinary search and the checks of
/// [`varied_move`], when `Variety` applies.
///
/// The ordinary search gets two thirds of the time left before the
/// deadline and two thirds of the node limit; the checks get what is left,
/// up to the original deadline and node limit. A depth limit stays with the
/// ordinary search, and the checks then run to completion, so a fixed-depth
/// search stays reproducible. Returns (search limits, check limits); the
/// check limits' node budget is the part of the node limit the search may
/// not use.
#[must_use]
pub fn split_limits(limits: Limits, now: Instant) -> (Limits, Limits) {
    let search_deadline = limits.deadline.map(|deadline| {
        let left = deadline.saturating_duration_since(now);
        now + left * 2 / 3
    });
    let search_nodes = limits.nodes.map(|nodes| nodes * 2 / 3);
    let search = Limits {
        depth: limits.depth,
        nodes: search_nodes,
        deadline: search_deadline,
    };
    let check = Limits {
        depth: None,
        nodes: limits.nodes.map(|nodes| nodes - nodes * 2 / 3),
        deadline: limits.deadline,
    };
    (search, check)
}

/// What [`varied_move`] chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Varied {
    /// The move to play: the best move or one within `Variety` of it.
    pub best_move: Move,
    /// That move's score from its check (the best move's own score if it is
    /// the best move), for the side to move.
    pub score: Score,
    /// How far that score is below the best move's, in centipawns: from 0
    /// to `Variety`.
    pub loss: Score,
    /// How many moves the choice was made among, the best move included.
    pub choices: usize,
    /// Positions visited by the cheap scores and the checks.
    pub nodes: u64,
}

/// Chooses the move to play in `pos` when `Variety` applies, given the
/// outcome of the ordinary search, `best`, which reached `depth` plies.
///
/// `history` holds the hashes of the positions before `pos`, as for
/// [`Search::with_history`]. `limits` and `stop` bound the cheap scores and
/// the checks (see [`split_limits`]); once one of them is reached no further
/// move is checked, and the moves not checked are not played.
///
/// Returns `None`, so that the best move is played as it is, when there is
/// no move, when `depth` is 0 or `variety` is 0, or when the best score is a
/// mate. Otherwise the generator is advanced exactly once, whatever the
/// position, so a game's random sequence depends on its moves and not on
/// how many moves were checked.
#[allow(clippy::too_many_arguments)]
pub fn varied_move(
    pos: &Position,
    history: &[u64],
    evaluator: &dyn Evaluator,
    best: &Outcome,
    depth: u32,
    variety: u32,
    limits: Limits,
    stop: &AtomicBool,
    rng: &mut Rng,
) -> Option<Varied> {
    let best_move = best.best_move?;
    if depth == 0 || variety == 0 || is_mate_score(best.score) {
        return None;
    }
    let variety = variety.min(VARIETY_MAX) as Score;
    let mut keys = history.to_vec();
    keys.push(pos.hash());
    let mut nodes: u64 = 0;
    let mut stopped = false;

    // The value of `pos` after `mv`, for the side to move in `pos`, searched
    // `plies` deep; `None` once a limit is reached.
    let mut score_after = |mv: Move, plies: u32, nodes: &mut u64| -> Option<Score> {
        if stopped {
            return None;
        }
        let mut next = pos.clone();
        let _undo = next.make_move(mv);
        let budget = Limits {
            depth: None,
            nodes: limits.nodes.map(|limit| limit.saturating_sub(*nodes)),
            deadline: limits.deadline,
        };
        let mut search = Search::new(evaluator)
            .with_history(&keys)
            .with_limits(budget)
            .with_stop_flag(stop);
        let score = -search.reply_value(&next, plies);
        *nodes += search.nodes();
        if search.was_stopped() {
            stopped = true;
            return None;
        }
        Some(score)
    };

    // The cheap scores: quiescence only, for every move.
    let mut cheap: Vec<(Move, Score)> = Vec::new();
    for &mv in &legal_moves(pos) {
        match score_after(mv, 0, &mut nodes) {
            Some(score) => cheap.push((mv, score)),
            None => break,
        }
    }
    let cheap_best = cheap
        .iter()
        .find(|(mv, _)| *mv == best_move)
        .map_or(Score::MIN, |&(_, score)| score);
    let top = cheap
        .iter()
        .map(|&(_, score)| score)
        .max()
        .unwrap_or(cheap_best);
    let threshold = top.saturating_sub(variety + PREFILTER_MARGIN);
    let mut worth_checking: Vec<(Move, Score)> = cheap
        .into_iter()
        .filter(|&(mv, score)| mv != best_move && score >= threshold)
        .collect();
    // Most promising first, so that a limit cuts off the least likely.
    worth_checking.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    // The second check: each candidate to the best move's depth.
    let mut choices: Vec<(Move, Score, Score)> = vec![(best_move, best.score, 0)];
    for (mv, _) in worth_checking {
        let Some(score) = score_after(mv, depth - 1, &mut nodes) else {
            break;
        };
        let loss = best.score.saturating_sub(score).max(0);
        if loss <= variety && !is_mate_score(score) {
            choices.push((mv, score, loss));
        }
    }

    let total: u64 = choices
        .iter()
        .map(|&(_, _, loss)| choice_weight(variety, loss))
        .sum();
    let mut pick = rng.next_below(total);
    let mut chosen = choices[0];
    for &choice in &choices {
        let w = choice_weight(variety, choice.2);
        if pick < w {
            chosen = choice;
            break;
        }
        pick -= w;
    }
    Some(Varied {
        best_move: chosen.0,
        score: chosen.1,
        loss: chosen.2,
        choices: choices.len(),
        nodes,
    })
}

/// How likely a move is to be chosen, relative to the others: weighted
/// towards the best, falling with the square of the move's loss, from
/// (`variety` + 1)^2 for the best move (loss 0) to 1 for a move exactly
/// `variety` worse. A loss outside 0 to `variety` (never offered) weighs 1.
const fn choice_weight(variety: Score, loss: Score) -> u64 {
    if loss < 0 || loss > variety {
        return 1;
    }
    let room = (variety - loss + 1) as u64;
    room * room
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::eval::HandCrafted;
    use crate::movegen::find_legal_move;
    use crate::search::mate_in;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    /// The ordinary search to `depth`, then the varied choice with no
    /// limits and the generator of `seed`.
    fn vary(pos: &Position, depth: u32, variety: u32, seed: u32) -> (Outcome, Option<Varied>) {
        let best = Search::new(&HandCrafted).fixed_depth(pos, depth);
        let stop = AtomicBool::new(false);
        let mut rng = Rng::new(seed);
        let varied = varied_move(
            pos,
            &[],
            &HandCrafted,
            &best,
            depth,
            variety,
            Limits::NONE,
            &stop,
            &mut rng,
        );
        (best, varied)
    }

    /// The exact score of `mv` in `pos` at `depth`, as the check computes it.
    fn checked_score(pos: &Position, mv: Move, depth: u32) -> Score {
        let mut next = pos.clone();
        let _undo = next.make_move(mv);
        -Search::new(&HandCrafted)
            .with_history(&[pos.hash()])
            .reply_value(&next, depth - 1)
    }

    #[test]
    fn it_applies_only_above_0_and_in_the_first_twelve_moves() {
        let start = Position::startpos();
        assert!(!variety_applies(&start, 0));
        assert!(variety_applies(&start, 1));
        let twelve = fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 12");
        let thirteen = fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 13");
        assert!(variety_applies(&twelve, 100));
        assert!(!variety_applies(&thirteen, 100));
    }

    #[test]
    fn nothing_varies_at_0_or_with_a_mate() {
        let start = Position::startpos();
        assert_eq!(vary(&start, 2, 0, 1).1, None);
        // White mates in one: the best score is a mate.
        let mate = fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1");
        let (best, varied) = vary(&mate, 2, 100, 1);
        assert_eq!(best.score, mate_in(1));
        assert_eq!(varied, None);
        // Black is mated whatever it does: no variety either.
        let lost = fen("7k/1R6/R7/8/8/8/8/6K1 b - - 0 1");
        let (best, varied) = vary(&lost, 3, 100, 1);
        assert!(is_mate_score(best.score));
        assert_eq!(varied, None);
    }

    /// Whatever the seed, the move played is checked to be within `Variety`
    /// of the best, and its score is what an exact search of it gives.
    #[test]
    fn every_choice_is_within_variety_of_the_best() {
        let positions = [
            crate::board::START_FEN,
            "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
            "rnbqkb1r/pp2pppp/3p1n2/8/3NP3/8/PPP2PPP/RNBQKB1R w KQkq - 1 5",
        ];
        for text in positions {
            let pos = fen(text);
            for variety in [10, 40, 100] {
                for seed in 1..=8 {
                    let (best, varied) = vary(&pos, 3, variety, seed);
                    let varied = varied.unwrap();
                    assert!(
                        varied.loss >= 0 && varied.loss <= variety as Score,
                        "{text}"
                    );
                    assert_eq!(varied.loss, best.score - varied.score, "{text}");
                    assert!(varied.choices >= 1);
                    if Some(varied.best_move) != best.best_move {
                        assert_eq!(varied.score, checked_score(&pos, varied.best_move, 3));
                    }
                }
            }
        }
    }

    /// A move that the second check shows to lose more than `Variety` is
    /// never played: here every move but one leaves the queen to be taken.
    #[test]
    fn a_losing_move_is_never_chosen() {
        // White's queen on d5 is attacked by the pawn on e6; only moving it
        // away keeps it, and the rest lose far more than 100 centipawns.
        let pos = fen("rnbqkbnr/pppp1ppp/4p3/3Q4/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 3");
        let mut other_moves = 0;
        for seed in 1..=20 {
            let (best, varied) = vary(&pos, 2, 100, seed);
            let varied = varied.unwrap();
            assert!(varied.loss <= 100);
            if Some(varied.best_move) != best.best_move {
                other_moves += 1;
            }
            let mut next = pos.clone();
            let _undo = next.make_move(varied.best_move);
            // The queen is not left on d5.
            let left = find_legal_move(&next, "e6d5").is_some();
            assert!(
                !left,
                "seed {seed} played {} (best {:?})",
                varied.best_move, best.best_move
            );
        }
        // Variety was at work: other queen moves were played too.
        assert!(other_moves > 0);
    }

    /// The second check is what stops a move whose loss the cheap score
    /// cannot see: moving the rook off the back rank allows Re1 mate, a
    /// quiet move that quiescence does not look at. Such a move passes the
    /// cheap filter, and only the exact search rejects it.
    #[test]
    fn the_second_check_rejects_what_the_cheap_score_misses() {
        let pos = fen("4r1k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1");
        let cheap = |mv: Move| {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            -Search::new(&HandCrafted)
                .with_history(&[pos.hash()])
                .reply_value(&next, 0)
        };
        let top = legal_moves(&pos).iter().map(|&mv| cheap(mv)).max().unwrap();
        let losing = ["a1a2", "a1a3", "a1a4", "a1a5", "a1a6", "a1a7"];
        // The cheap score of each losing rook move gets it past the filter.
        for text in losing {
            let mv = find_legal_move(&pos, text).unwrap();
            assert!(cheap(mv) >= top - 100 - PREFILTER_MARGIN, "{text}");
        }
        let mut other_moves = 0;
        for seed in 1..=20 {
            let (best, varied) = vary(&pos, 3, 100, seed);
            assert!(!is_mate_score(best.score));
            let varied = varied.unwrap();
            if Some(varied.best_move) != best.best_move {
                other_moves += 1;
            }
            let played = varied.best_move.to_string();
            assert!(
                !losing.contains(&played.as_str()),
                "seed {seed} played {played}"
            );
        }
        // Variety was at work: safe moves other than the best were played.
        assert!(other_moves > 0);
    }

    #[test]
    fn the_same_seed_gives_the_same_choice() {
        let pos = Position::startpos();
        for seed in 1..=5 {
            assert_eq!(vary(&pos, 3, 50, seed), vary(&pos, 3, 50, seed));
        }
    }

    /// With a `Variety` wide enough to offer several moves, different seeds
    /// play different moves.
    #[test]
    fn the_choice_varies_with_the_seed() {
        let pos = Position::startpos();
        let mut played: Vec<Move> = Vec::new();
        for seed in 1..=20 {
            let (_, varied) = vary(&pos, 3, 100, seed);
            let varied = varied.unwrap();
            assert!(varied.choices > 1);
            if !played.contains(&varied.best_move) {
                played.push(varied.best_move);
            }
        }
        assert!(played.len() > 1, "{played:?}");
    }

    /// The weighting favours the best move and falls as the loss grows: per
    /// centipawn lost a move is less likely, and one exactly `Variety` worse
    /// has the least weight. (Over many near-equal moves the best need not
    /// be the single most frequent choice.)
    #[test]
    fn the_weight_falls_with_the_loss() {
        for variety in [1, 10, 100] {
            assert_eq!(
                choice_weight(variety, 0),
                ((variety + 1) * (variety + 1)) as u64
            );
            assert_eq!(choice_weight(variety, variety), 1);
            for loss in 0..variety {
                assert!(choice_weight(variety, loss) > choice_weight(variety, loss + 1));
            }
        }
        assert_eq!(choice_weight(10, 11), 1);
        assert_eq!(choice_weight(10, -1), 1);
    }

    /// The generator is advanced exactly once per call that varies, however
    /// many moves were checked.
    #[test]
    fn one_number_is_drawn_per_choice() {
        for (text, depth) in [
            (crate::board::START_FEN, 2),
            ("8/8/4k3/8/8/4K3/4P3/8 w - - 0 1", 3),
        ] {
            let pos = fen(text);
            let best = Search::new(&HandCrafted).fixed_depth(&pos, depth);
            let stop = AtomicBool::new(false);
            let mut rng = Rng::new(9);
            let _ = varied_move(
                &pos,
                &[],
                &HandCrafted,
                &best,
                depth,
                30,
                Limits::NONE,
                &stop,
                &mut rng,
            );
            let mut expected = Rng::new(9);
            let _ = expected.next_u64();
            assert_eq!(rng.next_u64(), expected.next_u64(), "{text}");
        }
    }

    /// When the stop flag is already up, nothing can be checked, so the best
    /// move is played: no unchecked move ever is.
    #[test]
    fn with_no_time_to_check_the_best_move_is_played() {
        let pos = Position::startpos();
        let best = Search::new(&HandCrafted).fixed_depth(&pos, 3);
        let stop = AtomicBool::new(true);
        for seed in 1..=10 {
            let mut rng = Rng::new(seed);
            let varied = varied_move(
                &pos,
                &[],
                &HandCrafted,
                &best,
                3,
                100,
                Limits::NONE,
                &stop,
                &mut rng,
            )
            .unwrap();
            assert_eq!(Some(varied.best_move), best.best_move);
            assert_eq!(varied.choices, 1);
        }
        // A node budget that runs out also stops the checks.
        let mut rng = Rng::new(1);
        let free = AtomicBool::new(false);
        let varied = varied_move(
            &pos,
            &[],
            &HandCrafted,
            &best,
            3,
            100,
            Limits::nodes(1),
            &free,
            &mut rng,
        )
        .unwrap();
        assert_eq!(varied.choices, 1);
    }

    #[test]
    fn the_limits_are_split_two_thirds_to_one_third() {
        let now = Instant::now();
        let limits = Limits {
            depth: Some(5),
            nodes: Some(9_000),
            deadline: Some(now + Duration::from_millis(900)),
        };
        let (search, check) = split_limits(limits, now);
        assert_eq!(search.depth, Some(5));
        assert_eq!(search.nodes, Some(6_000));
        assert_eq!(search.deadline, Some(now + Duration::from_millis(600)));
        assert_eq!(check.depth, None);
        assert_eq!(check.nodes, Some(3_000));
        assert_eq!(check.deadline, limits.deadline);
        // No limits: nothing to split.
        assert_eq!(
            split_limits(Limits::NONE, now),
            (Limits::NONE, Limits::NONE)
        );
        // A depth alone stays with the search; the checks are unbounded.
        assert_eq!(
            split_limits(Limits::depth(4), now),
            (Limits::depth(4), Limits::NONE)
        );
    }
}
