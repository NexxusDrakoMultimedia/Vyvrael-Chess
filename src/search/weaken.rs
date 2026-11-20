// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Playing below full strength on purpose.
//!
//! A weakened engine should still look like a chess player: it should make
//! plausible mistakes, not random moves. So it scores every move of the
//! position to a shallow depth, as a weaker player sees less far, then
//! misjudges each move's value by a random amount and plays the one that
//! looks best to it. Every move is scored to the same depth, in passes of
//! increasing depth, so this holds however little time there is. A small misjudgement picks the second-best move now
//! and then; it never prefers a move that loses a queen to one that does
//! not, unless the noise is as large as a queen.
//!
//! This is **strength-affecting randomness**: the generator is handed in by
//! the caller, which seeds it according to the UCI options
//! `DeterministicRNG` and `RngSeed`.

use std::sync::atomic::AtomicBool;

use super::{DRAW, Limits, Outcome, Search, is_mate_score, mated_in};
use crate::board::Position;
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
    /// How many plies to look ahead, at least 1.
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
    /// Positions visited, over all the root moves and all the passes.
    pub nodes: u64,
    /// How many plies the scores the choice was made from look ahead: the
    /// weakening's depth, or less if the limits cut the work short.
    pub depth: u32,
}

/// Chooses a move for `pos` at reduced strength.
///
/// `history` holds the hashes of the positions the game went through
/// before `pos`, as for [`Search::with_history`].
///
/// The work is done in passes, like iterative deepening: every root move is
/// scored one ply deep, then every root move two plies deep, and so on up
/// to `weakening.depth`. The choice is made from the **last pass that
/// finished**, so every move has been looked at to the same depth whatever
/// the clock did. The first pass always completes, whatever the limits, as
/// the first iteration of the ordinary search does; `limits` (node limit
/// and deadline, shared by all moves and passes; the depth limit is
/// ignored) and `stop` can only cut the later ones short.
///
/// The generator is advanced once per root move before any searching, in
/// move order, so the random errors are a function of the position and the
/// generator's state alone. Which pass they are applied to depends on the
/// limits; with a depth or node limit that is reproducible, with a clock it
/// is not.
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
    if moves.is_empty() {
        let score = if pos.in_check() { mated_in(0) } else { DRAW };
        return Weakened {
            outcome: Outcome {
                score,
                best_move: None,
                pv: Vec::new(),
            },
            nodes: 0,
            depth: 0,
        };
    }
    let errors: Vec<Score> = moves.iter().map(|_| error(weakening.noise, rng)).collect();

    let mut keys = history.to_vec();
    keys.push(pos.hash());
    let mut nodes = 0;
    let mut scores: Vec<Score> = Vec::new();
    let mut reached = 0;
    for depth in 1..=weakening.depth.max(1) {
        let mut pass = Vec::with_capacity(moves.len());
        for &mv in &moves {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            let mut search = Search::new(evaluator).with_history(&keys);
            if depth > 1 {
                // Only the first pass is exempt from the limits.
                let budget = Limits {
                    depth: None,
                    nodes: limits.nodes.map(|limit| limit.saturating_sub(nodes)),
                    deadline: limits.deadline,
                };
                search = search.with_limits(budget).with_stop_flag(stop);
            }
            let score = -search.reply_value(&next, depth - 1);
            nodes += search.nodes();
            if search.was_stopped() {
                break;
            }
            pass.push(score);
        }
        if pass.len() < moves.len() {
            // Cut short: this pass is thrown away, like an interrupted
            // iteration, and the choice is made from the one before.
            break;
        }
        scores = pass;
        reached = depth;
    }

    // The move that looks best once each score is misjudged; the first of
    // equals. Forced mates are judged exactly: a weakened player may miss
    // the best quiet move, but it still takes a mate it has seen and still
    // knows when it is being mated.
    let seen = |index: usize| {
        let score = scores[index];
        if is_mate_score(score) {
            score
        } else {
            score.saturating_add(errors[index])
        }
    };
    let mut chosen = 0;
    for index in 1..moves.len() {
        if seen(index) > seen(chosen) {
            chosen = index;
        }
    }
    Weakened {
        outcome: Outcome {
            score: scores[chosen],
            best_move: Some(moves[chosen]),
            pv: vec![moves[chosen]],
        },
        nodes,
        depth: reached,
    }
}

/// One error in judging a move: between `-noise` and `+noise`, the sum of
/// two uniform draws, so that small errors are the common ones.
fn error(noise: Score, rng: &mut Rng) -> Score {
    let noise = noise.max(0);
    let span = noise as u64 + 1;
    (rng.next_below(span) + rng.next_below(span)) as Score - noise
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
            // The fifty-move rule: the mate is too late with the clock at
            // 98, and just in time at 97.
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 98 80", 4),
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 97 80", 4),
            // Every move here reaches the hundredth ply; Black's pawn could
            // reset the clock afterwards, but the game is already drawn.
            ("7k/7p/8/8/8/8/8/K7 w - - 99 80", 2),
            // Mate on the hundredth ply still counts.
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 99 80", 2),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 3),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 2),
        ] {
            let plain = Search::new(&HandCrafted).fixed_depth(&fen(text), depth);
            let weak = choose(text, exact(depth), DEFAULT_SEED);
            assert_eq!(weak.outcome.score, plain.score, "{text}");
            assert_eq!(weak.outcome.best_move, plain.best_move, "{text}");
            assert_eq!(weak.outcome.pv, [plain.best_move.unwrap()]);
            assert_eq!(weak.depth, depth);
        }
        // The two fifty-move cases really differ: a draw and a mate.
        let late = choose("7k/8/8/8/8/8/R7/1R4K1 w - - 98 80", exact(4), 1);
        assert_eq!(late.outcome.score, DRAW);
        let in_time = choose("7k/8/8/8/8/8/R7/1R4K1 w - - 97 80", exact(4), 1);
        assert_eq!(in_time.outcome.score, mate_in(3));
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
        let draws: Vec<Score> = (0..20_000).map(|_| error(50, &mut rng)).collect();
        assert_eq!(draws.iter().min(), Some(&-50));
        assert_eq!(draws.iter().max(), Some(&50));
        let mean = draws.iter().sum::<Score>() / draws.len() as Score;
        assert!(mean.abs() <= 1, "{mean}");
        // Small errors are the common ones: more than half lie within half
        // the range (a triangular spread puts three quarters there).
        let small = draws.iter().filter(|draw| draw.abs() <= 25).count();
        assert!(small > draws.len() * 7 / 10, "{small}");
        // No noise, no error.
        assert!((0..100).all(|_| error(0, &mut rng) == 0));
    }

    /// The numbers drawn do not depend on what the search finds: whether
    /// the position holds a mate in one or nothing at all, the generator
    /// ends exactly one error per legal move further on.
    #[test]
    fn the_generator_advances_once_per_root_move() {
        let noisy = Weakening {
            depth: 2,
            noise: 500,
        };
        let stop = AtomicBool::new(false);
        for text in [
            "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1",
            "6k1/5ppp/8/8/8/8/8/R4K2 w - - 0 1",
            crate::board::START_FEN,
            KIWIPETE,
        ] {
            let pos = fen(text);
            let mut rng = Rng::new(7);
            weakened_move(
                &pos,
                &[],
                &HandCrafted,
                noisy,
                Limits::NONE,
                &stop,
                &mut rng,
            );
            let mut expected = Rng::new(7);
            for _ in 0..legal_moves(&pos).len() {
                error(500, &mut expected);
            }
            assert_eq!(rng, expected, "{text}");
        }
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

    /// Black's queen on g5 can be taken for nothing by the bishop on d2.
    const HANGING_QUEEN: &str =
        "r3k2r/p1pp1pb1/bn2pnp1/3PN1q1/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    fn limited(
        text: &str,
        weakening: Weakening,
        limits: Limits,
        stopped: bool,
        seed: u32,
    ) -> Weakened {
        let stop = AtomicBool::new(stopped);
        let mut rng = Rng::new(seed);
        weakened_move(
            &fen(text),
            &[],
            &HandCrafted,
            weakening,
            limits,
            &stop,
            &mut rng,
        )
    }

    /// However little time there is, every move has been looked at: with
    /// the deadline already passed, or told to stop before it began, the
    /// engine still takes the hanging queen, and the choice is exactly the
    /// one-ply choice for the same seed. It does not fall back on whichever
    /// move happens to be generated first.
    #[test]
    fn out_of_time_it_still_looks_at_every_move() {
        let noisy = Weakening {
            depth: 3,
            noise: 100,
        };
        let one_ply = Weakening {
            depth: 1,
            noise: 100,
        };
        let first_generated = legal_moves(&fen(HANGING_QUEEN))[0];
        for seed in 0..25 {
            let reference = limited(HANGING_QUEEN, one_ply, Limits::NONE, false, seed);
            assert_eq!(reference.depth, 1);
            assert_eq!(reference.outcome.best_move.unwrap().to_string(), "d2g5");
            assert!(reference.outcome.score > 500, "a real score, not a guess");
            assert_ne!(reference.outcome.best_move, Some(first_generated));

            let late = limited(
                HANGING_QUEEN,
                noisy,
                Limits::until(std::time::Instant::now()),
                false,
                seed,
            );
            assert_eq!(late.outcome, reference.outcome, "seed {seed}");
            assert_eq!(late.depth, 1);
            let stopped = limited(HANGING_QUEEN, noisy, Limits::NONE, true, seed);
            assert_eq!(stopped.outcome, reference.outcome, "seed {seed}");
        }
    }

    /// A pass that the limits cut short is thrown away whole: the answer is
    /// the one the previous pass gives for the same seed, never a mixture
    /// of moves looked at to different depths.
    #[test]
    fn an_interrupted_pass_is_discarded() {
        let noisy = |depth| Weakening { depth, noise: 60 };
        for seed in 0..10 {
            let two = limited(ROOK_ENDGAME, noisy(2), Limits::NONE, false, seed);
            let three = limited(ROOK_ENDGAME, noisy(3), Limits::NONE, false, seed);
            assert_eq!((two.depth, three.depth), (2, 3));
            // A limit that falls in the middle of the third pass, after
            // several of its moves have been scored and before the rest.
            let halfway = two.nodes + (three.nodes - two.nodes) / 2;
            let cut = limited(ROOK_ENDGAME, noisy(3), Limits::nodes(halfway), false, seed);
            assert_eq!(cut.depth, 2, "seed {seed}");
            assert_eq!(cut.outcome, two.outcome, "seed {seed}");
            assert_eq!(cut.nodes, halfway, "the third pass ran up to the limit");
        }
        // The two depths do not always agree, or the test would show
        // nothing: for some seed the full third pass chooses differently.
        assert!((0..10).any(|seed| {
            let two = limited(ROOK_ENDGAME, noisy(2), Limits::NONE, false, seed);
            let three = limited(ROOK_ENDGAME, noisy(3), Limits::NONE, false, seed);
            two.outcome != three.outcome
        }));
        // A node limit smaller than the first pass does not stop the first
        // pass, and nothing after it is begun.
        let one = limited(KIWIPETE, noisy(1), Limits::NONE, false, 3);
        let tiny = limited(KIWIPETE, noisy(3), Limits::nodes(1), false, 3);
        assert_eq!((tiny.depth, &tiny.outcome), (1, &one.outcome));
        assert_eq!(tiny.nodes, one.nodes);
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
