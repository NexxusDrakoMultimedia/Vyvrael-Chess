// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Playing below full strength on purpose.
//!
//! A weakened engine should still look like a chess player: it should make
//! plausible mistakes, not random moves. So it searches every move of the
//! position to a shallow depth, as a weaker player sees less far, then
//! misjudges each move's value by a random amount and plays the one that
//! looks best to it. A small misjudgement picks the second-best move now
//! and then; it never prefers a move that loses a queen to one that does
//! not, unless the noise is as large as a queen.
//!
//! This is **strength-affecting randomness**: the generator is handed in by
//! the caller, which seeds it according to the UCI options
//! `DeterministicRNG` and `RngSeed`.

use std::sync::atomic::AtomicBool;

use super::{
    DRAW, Limits, Outcome, Search, fifty_moves_reached, insufficient_material, is_mate_score,
    is_repetition, mated_in,
};
use crate::board::{Move, Position};
use crate::eval::{Evaluator, Score};
use crate::movegen::legal_moves;
use crate::rng::Rng;

/// Lowest rating that can be asked for.
pub const ELO_MIN: u32 = 400;
/// Highest rating that can be asked for.
pub const ELO_MAX: u32 = 1800;
/// The rating used when none is chosen.
pub const ELO_DEFAULT: u32 = 1200;

/// How much to weaken play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weakening {
    /// How many plies to look ahead, at least 2.
    pub depth: u32,
    /// The largest error, in centipawns, made in judging a move. Errors are
    /// spread between minus and plus this, small ones being more likely.
    pub noise: Score,
}

impl Weakening {
    /// The weakening for a target rating, which is clamped to
    /// [`ELO_MIN`]`..=`[`ELO_MAX`].
    ///
    /// **The numbers are provisional.** They only order the settings: a
    /// higher rating looks further and errs less. What rating each setting
    /// really plays at is measured in milestone 6, and this mapping is
    /// refitted then. Until that is done the rating is a label.
    #[must_use]
    pub const fn for_elo(elo: u32) -> Weakening {
        let elo = if elo < ELO_MIN {
            ELO_MIN
        } else if elo > ELO_MAX {
            ELO_MAX
        } else {
            elo
        };
        // Position of the rating in its range, in thousandths.
        let level = (elo - ELO_MIN) * 1000 / (ELO_MAX - ELO_MIN);
        Weakening {
            depth: if level < 500 { 2 } else { 3 },
            noise: 300 - (level * 280 / 1000) as Score,
        }
    }
}

/// What a weakened search chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weakened {
    /// The move chosen, its true (not misjudged) score, and a one-move
    /// line. `best_move` is `None` only when there is no legal move.
    pub outcome: Outcome,
    /// Positions visited, over all the root moves.
    pub nodes: u64,
}

/// Chooses a move for `pos` at reduced strength.
///
/// `history` holds the hashes of the positions the game went through
/// before `pos`, as for [`Search::with_history`]. `limits` and `stop` end
/// the work early: the node limit and deadline are shared by all root
/// moves, the depth limit is ignored (the depth is `weakening.depth`). A
/// move whose search was cut short is not considered; if none was finished
/// the first legal move is played.
///
/// The generator is advanced in the same way for every root move searched,
/// in move order and whatever the move's score, so the choice is a function
/// of the position, the weakening and the generator's state.
pub fn weakened_move(
    pos: &Position,
    history: &[u64],
    evaluator: &dyn Evaluator,
    weakening: Weakening,
    limits: Limits,
    stop: &AtomicBool,
    rng: &mut Rng,
) -> Weakened {
    let moves = legal_moves(pos);
    let Some(&first) = moves.first() else {
        let score = if pos.in_check() { mated_in(0) } else { DRAW };
        return Weakened {
            outcome: Outcome {
                score,
                best_move: None,
                pv: Vec::new(),
            },
            nodes: 0,
        };
    };

    let mut keys = history.to_vec();
    keys.push(pos.hash());
    let child_depth = weakening.depth.max(2) - 1;
    let mut nodes = 0;
    // (score as misjudged, true score, move) of the best candidate so far.
    let mut chosen: Option<(Score, Score, Move)> = None;
    for &mv in &moves {
        let mut next = pos.clone();
        let _undo = next.make_move(mv);
        let budget = Limits {
            depth: None,
            nodes: limits.nodes.map(|limit| limit.saturating_sub(nodes)),
            deadline: limits.deadline,
        };
        let mut search = Search::new(evaluator)
            .with_history(&keys)
            .with_limits(budget)
            .with_stop_flag(stop);
        let score = if drawn_by_rule(&next, &keys) {
            DRAW
        } else {
            one_ply_further(-search.fixed_depth(&next, child_depth).score)
        };
        nodes += search.nodes();
        if search.was_stopped() {
            break;
        }
        let seen = score.saturating_add(error(score, weakening.noise, rng));
        if chosen.is_none_or(|(best, _, _)| seen > best) {
            chosen = Some((seen, score, mv));
        }
    }

    let (score, best) = chosen.map_or((DRAW, first), |(_, score, mv)| (score, mv));
    Weakened {
        outcome: Outcome {
            score,
            best_move: Some(best),
            pv: vec![best],
        },
        nodes,
    }
}

/// Whether `pos`, reached by a move from the root, is a draw by rule. A
/// search started at `pos` would not notice, since a search never scores
/// its own root as drawn.
fn drawn_by_rule(pos: &Position, earlier: &[u64]) -> bool {
    if insufficient_material(pos) || is_repetition(pos, earlier) {
        return true;
    }
    fifty_moves_reached(pos) && !(pos.in_check() && legal_moves(pos).is_empty())
}

/// A score found by searching the position after a move, restated for the
/// position before it: a mate is one ply further away.
const fn one_ply_further(score: Score) -> Score {
    if is_mate_score(score) {
        score - score.signum()
    } else {
        score
    }
}

/// The error made in judging a move worth `score`: between `-noise` and
/// `+noise`, the sum of two uniform draws, so that small errors are the
/// common ones. Forced mates are judged exactly: a weakened player may miss
/// the best quiet move, but it still takes a mate it has seen and still
/// knows when it is being mated.
///
/// Draws from `rng` even when the result is not used, so that the sequence
/// of numbers does not depend on the scores.
fn error(score: Score, noise: Score, rng: &mut Rng) -> Score {
    let span = noise.max(0) as u64 + 1;
    let drawn = (rng.next_below(span) + rng.next_below(span)) as Score - noise.max(0);
    if is_mate_score(score) { 0 } else { drawn }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::HandCrafted;
    use crate::rng::DEFAULT_SEED;
    use crate::search::mate_in;

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const ROOK_ENDGAME: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    fn choose(text: &str, weakening: Weakening, seed: u32) -> Weakened {
        let stop = AtomicBool::new(false);
        let mut rng = Rng::new(seed);
        weakened_move(
            &fen(text),
            &[],
            &HandCrafted,
            weakening,
            Limits::NONE,
            &stop,
            &mut rng,
        )
    }

    const fn exact(depth: u32) -> Weakening {
        Weakening { depth, noise: 0 }
    }

    /// Without noise, scoring the root moves one by one finds exactly what
    /// the ordinary search finds at the same depth: the same move and the
    /// same score.
    #[test]
    fn without_noise_it_is_the_ordinary_search() {
        for (text, depth) in [
            (crate::board::START_FEN, 3),
            (KIWIPETE, 2),
            (ROOK_ENDGAME, 3),
            ("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 2),
            // Mates, for and against the side to move.
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 4),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 3),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 2),
        ] {
            let plain = Search::new(&HandCrafted).fixed_depth(&fen(text), depth);
            let weak = choose(text, exact(depth), DEFAULT_SEED);
            assert_eq!(weak.outcome.score, plain.score, "{text}");
            assert_eq!(weak.outcome.best_move, plain.best_move, "{text}");
            assert_eq!(weak.outcome.pv, [plain.best_move.unwrap()]);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_move() {
        let noisy = Weakening {
            depth: 2,
            noise: 200,
        };
        for seed in [0, 1, DEFAULT_SEED] {
            assert_eq!(
                choose(crate::board::START_FEN, noisy, seed),
                choose(crate::board::START_FEN, noisy, seed)
            );
        }
    }

    /// With noise the choice varies from seed to seed, which is the point.
    #[test]
    fn noise_varies_the_choice() {
        let noisy = Weakening {
            depth: 2,
            noise: 200,
        };
        let mut seen = Vec::new();
        for seed in 0..40 {
            let mv = choose(crate::board::START_FEN, noisy, seed)
                .outcome
                .best_move
                .unwrap();
            if !seen.contains(&mv) {
                seen.push(mv);
            }
        }
        assert!(seen.len() >= 5, "only {seen:?}");
        // Every one of them is a legal move.
        let legal = legal_moves(&Position::startpos());
        assert!(seen.iter().all(|mv| legal.contains(mv)));
    }

    /// The error is bounded, so a mistake larger than twice the noise is
    /// never made: with 100 centipawns of noise the hanging queen is always
    /// taken, and the reported score is the true one, not the misjudged one.
    #[test]
    fn mistakes_stay_within_the_noise() {
        let text = "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1";
        let plain = Search::new(&HandCrafted).fixed_depth(&fen(text), 2);
        let noisy = Weakening {
            depth: 2,
            noise: 100,
        };
        for seed in 0..60 {
            let weak = choose(text, noisy, seed);
            assert_eq!(weak.outcome.best_move.unwrap().to_string(), "d1d5");
            assert_eq!(weak.outcome.score, plain.score);
        }
    }

    #[test]
    fn a_mate_is_never_misjudged() {
        let huge = Weakening {
            depth: 2,
            noise: 30_000,
        };
        for seed in 0..40 {
            // Mate in one is played whatever the noise.
            let weak = choose("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", huge, seed);
            assert_eq!(weak.outcome.best_move.unwrap().to_string(), "a1a8");
            assert_eq!(weak.outcome.score, mate_in(1));
        }
    }

    #[test]
    fn errors_are_bounded_and_centred() {
        let mut rng = Rng::default();
        let draws: Vec<Score> = (0..20_000).map(|_| error(0, 50, &mut rng)).collect();
        assert_eq!(draws.iter().min(), Some(&-50));
        assert_eq!(draws.iter().max(), Some(&50));
        let mean = draws.iter().sum::<Score>() / draws.len() as Score;
        assert!(mean.abs() <= 1, "{mean}");
        // Small errors are the common ones: more than half lie within half
        // the range (a triangular spread puts three quarters there).
        let small = draws.iter().filter(|draw| draw.abs() <= 25).count();
        assert!(small > draws.len() * 7 / 10, "{small}");
        // No noise, no error; and a mate is exempt but still draws numbers.
        assert!((0..100).all(|_| error(0, 0, &mut rng) == 0));
        let mut one = Rng::new(7);
        let mut other = Rng::new(7);
        assert_eq!(error(mate_in(3), 500, &mut one), 0);
        error(10, 500, &mut other);
        assert_eq!(one, other, "the same numbers were drawn");
    }

    #[test]
    fn no_legal_move() {
        let mated = choose("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", exact(2), 1);
        assert_eq!(
            (mated.outcome.score, mated.outcome.best_move),
            (mated_in(0), None)
        );
        let stalemate = choose("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", exact(2), 1);
        assert_eq!(
            (stalemate.outcome.score, stalemate.outcome.best_move),
            (DRAW, None)
        );
    }

    /// A queen down, Black can repeat a position from the game. The moves
    /// are scored one by one, so the draw has to be recognised for the
    /// position right after the move.
    #[test]
    fn the_game_history_is_used() {
        let mut pos = fen("7k/8/8/8/8/2K5/8/1Q6 w - - 10 30");
        let mut history = Vec::new();
        for text in ["b1c1", "h8g8", "c1b1"] {
            history.push(pos.hash());
            let mv = pos.parse_move(text).unwrap();
            let _undo = pos.make_move(mv);
        }
        let stop = AtomicBool::new(false);
        let mut rng = Rng::default();
        let weak = weakened_move(
            &pos,
            &history,
            &HandCrafted,
            exact(2),
            Limits::NONE,
            &stop,
            &mut rng,
        );
        assert_eq!(weak.outcome.score, DRAW);
        assert_eq!(weak.outcome.best_move.unwrap().to_string(), "g8h8");
    }

    #[test]
    fn it_can_be_stopped() {
        // Stopped before anything is searched: the first legal move.
        let stop = AtomicBool::new(true);
        let mut rng = Rng::default();
        let pos = fen(KIWIPETE);
        let weak = weakened_move(
            &pos,
            &[],
            &HandCrafted,
            exact(3),
            Limits::NONE,
            &stop,
            &mut rng,
        );
        assert_eq!(weak.outcome.best_move, legal_moves(&pos).first().copied());

        // A node limit shared by all root moves: the work ends close to it,
        // and the move comes from those that were finished.
        let calm = AtomicBool::new(false);
        let limited = weakened_move(
            &pos,
            &[],
            &HandCrafted,
            exact(2),
            Limits::nodes(3_000),
            &calm,
            &mut rng,
        );
        assert!(limited.nodes <= 3_000, "{}", limited.nodes);
        assert!(legal_moves(&pos).contains(&limited.outcome.best_move.unwrap()));
        let full = weakened_move(
            &pos,
            &[],
            &HandCrafted,
            exact(2),
            Limits::NONE,
            &calm,
            &mut rng,
        );
        assert!(full.nodes > limited.nodes * 10);
    }

    #[test]
    fn a_higher_rating_looks_further_and_errs_less() {
        assert_eq!(
            Weakening::for_elo(ELO_MIN),
            Weakening {
                depth: 2,
                noise: 300
            }
        );
        assert_eq!(
            Weakening::for_elo(ELO_MAX),
            Weakening {
                depth: 3,
                noise: 20
            }
        );
        // Out of range is clamped.
        assert_eq!(Weakening::for_elo(0), Weakening::for_elo(ELO_MIN));
        assert_eq!(Weakening::for_elo(9_999), Weakening::for_elo(ELO_MAX));
        assert!((ELO_MIN..=ELO_MAX).contains(&ELO_DEFAULT));

        let mut previous = Weakening::for_elo(ELO_MIN);
        for elo in ELO_MIN..=ELO_MAX {
            let now = Weakening::for_elo(elo);
            assert!(now.depth >= previous.depth, "{elo}");
            assert!(now.noise <= previous.noise, "{elo}");
            assert!(now.depth >= 2 && now.noise > 0);
            previous = now;
        }
    }
}
