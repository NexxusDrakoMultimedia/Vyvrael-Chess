// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Negamax search with alpha-beta pruning.
//!
//! Negamax is minimax written once for both sides: a position's value for
//! the side to move is the best, over its moves, of the *negated* value of
//! the position that move leads to. Alpha-beta skips moves that cannot
//! change the result: `alpha` is the score the side to move is already sure
//! of, `beta` the score the opponent can already hold it to, and once a move
//! scores at least `beta` the opponent would never allow this position, so
//! the remaining moves need not be looked at.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::ordering::captures_first;
use super::{
    DRAW, INFINITY, Limits, MATE, MAX_PLY, fifty_moves_reached, insufficient_material,
    is_repetition, mated_in,
};
use crate::board::{Move, PieceKind, Position};
use crate::eval::{Evaluator, Score};
use crate::movegen::legal_moves;
use crate::movegen::pseudo_legal_moves;

/// What a search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The value of the position for the side to move.
    pub score: Score,
    /// The move that achieves it, or `None` when the side to move has no
    /// legal move (checkmate or stalemate).
    pub best_move: Option<Move>,
    /// The principal variation: the line of best play for both sides that
    /// the score comes from, starting with `best_move`. It is shorter than
    /// the search depth when the line ends early in a mate or a draw.
    pub pv: Vec<Move>,
}

/// Progress after one completed iteration of [`Search::iterate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The depth just completed, in plies.
    pub depth: u32,
    /// The score at that depth, for the side to move.
    pub score: Score,
    /// Positions visited since the search began, over all iterations.
    pub nodes: u64,
    /// Time since the search began.
    pub elapsed: Duration,
    /// The principal variation at that depth.
    pub pv: Vec<Move>,
}

/// A search in progress: the evaluator it uses and what it has counted.
pub struct Search<'a> {
    evaluator: &'a dyn Evaluator,
    nodes: u64,
    /// Hashes of the positions before the one being searched, oldest first:
    /// the game so far, then the path from the root. Used to see
    /// repetitions.
    keys: Vec<u64>,
    /// The best line found from each ply ("triangular" table): `pv[p]` is
    /// the line starting at ply `p`. A node that finds a new best move writes
    /// that move followed by its child's line.
    pv: Vec<Vec<Move>>,
    limits: Limits,
    /// Set from outside (another thread) to end the search early.
    stop_flag: Option<&'a AtomicBool>,
    /// Latched once any limit is hit: from then on every node returns at
    /// once and the search unwinds.
    stopped: bool,
    /// False while the first iteration runs, so that it always completes and
    /// there is always a move to play.
    limits_apply: bool,
}

/// The stop flag and the clock are looked at once per this many nodes:
/// reading them at every node would cost more than the search itself.
const CHECK_INTERVAL: u64 = 1024;

impl<'a> Search<'a> {
    /// Creates a search that scores positions with `evaluator`.
    #[must_use]
    pub fn new(evaluator: &'a dyn Evaluator) -> Search<'a> {
        Search {
            evaluator,
            nodes: 0,
            keys: Vec::new(),
            // One line per ply, plus one for the child of the deepest node.
            pv: vec![Vec::new(); MAX_PLY + 2],
            limits: Limits::NONE,
            stop_flag: None,
            stopped: false,
            limits_apply: true,
        }
    }

    /// Tells the search which positions the game has already been through,
    /// as their hashes, oldest first and **not** including the position that
    /// will be searched. Without this the search can only see repetitions
    /// inside its own lines.
    #[must_use]
    pub fn with_history(mut self, earlier: &[u64]) -> Search<'a> {
        self.keys = earlier.to_vec();
        self
    }

    /// Sets when the search should stop by itself.
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Search<'a> {
        self.limits = limits;
        self
    }

    /// Gives the search a flag that another thread can set to stop it. This
    /// is how `stop` reaches a search that is already running.
    #[must_use]
    pub fn with_stop_flag(mut self, flag: &'a AtomicBool) -> Search<'a> {
        self.stop_flag = Some(flag);
        self
    }

    /// Whether the search was cut short by a limit or the stop flag. If so,
    /// the outcome of the search that was running is incomplete.
    #[must_use]
    pub const fn was_stopped(&self) -> bool {
        self.stopped
    }

    /// Whether the search must stop now. Once true it stays true.
    fn should_stop(&mut self) -> bool {
        if self.stopped {
            return true;
        }
        if !self.limits_apply {
            return false;
        }
        if self.limits.nodes.is_some_and(|limit| self.nodes >= limit) {
            self.stopped = true;
        } else if self.nodes.is_multiple_of(CHECK_INTERVAL) {
            let flagged = self
                .stop_flag
                .is_some_and(|flag| flag.load(Ordering::Relaxed));
            let out_of_time = self.limits.deadline.is_some_and(|at| Instant::now() >= at);
            self.stopped = flagged || out_of_time;
        }
        self.stopped
    }

    /// Whether a limit has been reached, checked right now instead of
    /// waiting for the next periodic check. Used between iterations.
    fn limit_reached_now(&self) -> bool {
        self.limits.nodes.is_some_and(|limit| self.nodes >= limit)
            || self
                .stop_flag
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            || self.limits.deadline.is_some_and(|at| Instant::now() >= at)
    }

    /// Searches `pos` by iterative deepening: depth 1, then 2, and so on,
    /// until a limit is reached, and returns the outcome of the deepest
    /// iteration that completed. `report` is called after each one.
    ///
    /// Deepening looks wasteful, since every iteration repeats the work of
    /// the one before, but each level of a chess tree is many times larger
    /// than the last, so the repeats are a small part of the total. In
    /// return the search can be stopped at any moment and still has a
    /// complete answer from the previous depth, which is what playing to a
    /// clock requires.
    ///
    /// The first iteration always completes, whatever the limits, so the
    /// outcome always names a move if one exists. An iteration that a limit
    /// interrupts is thrown away. The search also ends early once it has
    /// found a forced mate that fits inside the depth just searched, because
    /// searching deeper cannot find a shorter one.
    pub fn iterate(&mut self, pos: &Position, report: &mut dyn FnMut(&Report)) -> Outcome {
        let start = Instant::now();
        let deepest = self
            .limits
            .depth
            .map_or(MAX_PLY as u32, |depth| depth.clamp(1, MAX_PLY as u32));
        let mut best: Option<Outcome> = None;
        for depth in 1..=deepest {
            self.limits_apply = depth > 1;
            let outcome = self.fixed_depth(pos, depth);
            if self.stopped {
                break;
            }
            report(&Report {
                depth,
                score: outcome.score,
                nodes: self.nodes,
                elapsed: start.elapsed(),
                pv: outcome.pv.clone(),
            });
            let nothing_to_search = outcome.best_move.is_none();
            // A mate score counts plies from the root; within `depth` it was
            // found by the full-width search and is the shortest there is.
            let mate_found = MATE - outcome.score.abs() <= depth as Score;
            best = Some(outcome);
            if nothing_to_search || mate_found || self.limit_reached_now() {
                break;
            }
        }
        self.limits_apply = true;
        best.expect("the first iteration is never interrupted")
    }

    /// Whether `pos` is drawn by rule, so that searching it is pointless:
    /// too little material, a repeated position, or the fifty-move rule.
    fn is_draw(&self, pos: &Position) -> bool {
        if insufficient_material(pos) || is_repetition(pos, &self.keys) {
            return true;
        }
        // A mate delivered by the hundredth ply still counts as a mate.
        fifty_moves_reached(pos) && !(pos.in_check() && legal_moves(pos).is_empty())
    }

    /// Records `mv` as the best move at `ply`, followed by the best line
    /// already found for the position it leads to.
    fn update_pv(&mut self, ply: usize, mv: Move) {
        let (upper, lower) = self.pv.split_at_mut(ply + 1);
        let line = &mut upper[ply];
        line.clear();
        line.push(mv);
        line.extend_from_slice(&lower[0]);
    }

    /// How many positions have been visited so far.
    #[must_use]
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Searches `pos` exactly `depth` plies deep and returns the score and
    /// the best move. The position is not modified: each move is played on
    /// a copy ("copy-make"), which measured slightly faster here than making
    /// and unmaking moves in place.
    ///
    /// The node limit, deadline and stop flag apply (the depth limit does
    /// not: the depth is the argument). If one of them cuts the search short,
    /// [`Search::was_stopped`] is true and the outcome covers only the root
    /// moves that were searched completely, which may be none.
    ///
    /// # Panics
    ///
    /// Panics if `depth` is 0 or more than [`MAX_PLY`].
    pub fn fixed_depth(&mut self, pos: &Position, depth: u32) -> Outcome {
        assert!(
            depth >= 1 && depth as usize <= MAX_PLY,
            "search depth must be between 1 and MAX_PLY"
        );
        self.pv[0].clear();
        let mut alpha = -INFINITY;
        let mut best_move = None;
        let mut any_legal = false;
        if self.should_stop() {
            return Outcome {
                score: DRAW,
                best_move: None,
                pv: Vec::new(),
            };
        }
        self.nodes += 1;
        for &mv in &pseudo_legal_moves(pos) {
            self.keys.push(pos.hash());
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                self.keys.pop();
                continue;
            }
            any_legal = true;
            let score = -self.negamax(&next, depth - 1, 1, -INFINITY, -alpha);
            self.keys.pop();
            if self.stopped {
                // This move was not searched to the end; its score means
                // nothing.
                break;
            }
            // The first legal move is always taken, so a best move exists
            // whenever any legal move does.
            if best_move.is_none() || score > alpha {
                alpha = score;
                best_move = Some(mv);
                self.update_pv(0, mv);
            }
        }
        match best_move {
            Some(_) => Outcome {
                score: alpha,
                best_move,
                pv: self.pv[0].clone(),
            },
            // Stopped before any move was searched completely.
            None if any_legal => Outcome {
                score: DRAW,
                best_move: None,
                pv: Vec::new(),
            },
            None => Outcome {
                score: no_moves_score(pos, 0),
                best_move: None,
                pv: Vec::new(),
            },
        }
    }

    /// The value of `pos` for the side to move, searched `depth` plies
    /// deeper, given that only scores strictly between `alpha` and `beta`
    /// matter. `ply` is the distance from the root.
    ///
    /// "Fail-soft": the result may lie outside the window, in which case it
    /// is a bound on the true value, not the value itself.
    fn negamax(
        &mut self,
        pos: &Position,
        depth: u32,
        ply: usize,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.pv[ply].clear();
        if self.should_stop() {
            // The value is ignored: every caller checks `stopped` first.
            return DRAW;
        }
        // The root is never scored as a draw here: a move must still be
        // chosen there. Everywhere else a drawn position ends the line.
        if self.is_draw(pos) {
            self.nodes += 1;
            return DRAW;
        }
        if depth == 0 || ply >= MAX_PLY {
            // Out of depth: do not evaluate in the middle of an exchange.
            return self.quiesce(pos, ply, alpha, beta);
        }
        self.nodes += 1;

        let mut best = -INFINITY;
        let mut any_legal = false;
        for &mv in &pseudo_legal_moves(pos) {
            self.keys.push(pos.hash());
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                self.keys.pop();
                continue;
            }
            any_legal = true;
            let score = -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha);
            self.keys.pop();
            if self.stopped {
                return DRAW;
            }

            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    self.update_pv(ply, mv);
                    if alpha >= beta {
                        // The opponent has a better option elsewhere and
                        // will never let the game reach this position.
                        break;
                    }
                }
            }
        }

        if any_legal {
            best
        } else {
            no_moves_score(pos, ply)
        }
    }
}

impl Search<'_> {
    /// Quiescence search: the value of `pos` once the captures have been
    /// played out, within the window `alpha..beta`.
    ///
    /// A static evaluation is only meaningful in a quiet position. Stopping
    /// the search at a fixed depth and evaluating there would, for example,
    /// count a queen that has just taken a defended pawn as a pawn won (the
    /// "horizon effect"). So at the horizon the search continues, but with
    /// captures and queen promotions only:
    ///
    /// - The side to move may always decline to capture, so the static
    ///   evaluation is a lower bound on its score ("standing pat").
    /// - When it is in check there is no standing pat: every legal move is
    ///   searched, and having none is checkmate.
    ///
    /// Captures cannot go on for ever, so this ends without a depth limit.
    ///
    /// `ply` 0 means the position is being valued on its own, not as part of
    /// a search from somewhere else; like the root of the main search, it is
    /// then not scored as a dead draw.
    fn quiesce(&mut self, pos: &Position, ply: usize, mut alpha: Score, beta: Score) -> Score {
        if self.should_stop() {
            return DRAW;
        }
        self.nodes += 1;
        // A capture can leave too little material to mate with. Repetition
        // and the fifty-move rule cannot arise here: captures are
        // irreversible.
        if ply > 0 && insufficient_material(pos) {
            return DRAW;
        }

        let in_check = pos.in_check();
        let mut best = -INFINITY;
        if !in_check || ply >= MAX_PLY {
            let stand_pat = self.evaluator.evaluate(pos);
            if stand_pat >= beta || ply >= MAX_PLY {
                return stand_pat;
            }
            best = stand_pat;
            alpha = alpha.max(stand_pat);
        }

        let mut any_legal = false;
        // Most valuable victim first: see `ordering.rs` for why this is not
        // optional here.
        for &mv in &captures_first(pos, &pseudo_legal_moves(pos)) {
            if !in_check && !is_tactical(mv) {
                continue;
            }
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                continue;
            }
            any_legal = true;
            let score = -self.quiesce(&next, ply + 1, -beta, -alpha);
            if self.stopped {
                return DRAW;
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }

        if in_check && !any_legal {
            mated_in(ply)
        } else {
            best
        }
    }
}

/// Whether quiescence search looks at `mv` when not in check: captures (en
/// passant and capturing promotions included) and promotions to a queen.
fn is_tactical(mv: Move) -> bool {
    mv.is_capture() || mv.promotion() == Some(PieceKind::Queen)
}

/// The score of a position whose side to move has no legal move: checkmate
/// if it is in check, stalemate otherwise.
fn no_moves_score(pos: &Position, ply: usize) -> Score {
    if pos.in_check() { mated_in(ply) } else { DRAW }
}

#[cfg(test)]
mod tests {
    use super::super::mate_in;
    use super::*;
    use crate::eval::HandCrafted;

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    fn search(text: &str, depth: u32) -> Outcome {
        Search::new(&HandCrafted).fixed_depth(&fen(text), depth)
    }

    fn best(text: &str, depth: u32) -> String {
        search(text, depth)
            .best_move
            .map(|mv| mv.to_string())
            .unwrap()
    }

    /// The exact quiescence value of a position: a full window leaves
    /// nothing to prune against.
    fn quiet_value(pos: &Position, ply: usize) -> Score {
        Search::new(&HandCrafted).quiesce(pos, ply, -INFINITY, INFINITY)
    }

    /// Plain minimax with no pruning at all: the definition that alpha-beta
    /// must agree with. Its leaves are valued by a full-window quiescence
    /// search, the same leaf value the real search uses. Returns the score
    /// and counts the nodes visited, quiescence nodes included.
    fn minimax(pos: &mut Position, depth: u32, ply: usize, nodes: &mut u64) -> Score {
        // The same draw rules as the real search, apart from repetition,
        // which cannot arise within three plies of these positions.
        if ply > 0 && Search::new(&HandCrafted).is_draw(pos) {
            *nodes += 1;
            return DRAW;
        }
        if depth == 0 {
            let mut leaf = Search::new(&HandCrafted);
            let value = leaf.quiesce(pos, ply, -INFINITY, INFINITY);
            *nodes += leaf.nodes();
            return value;
        }
        *nodes += 1;
        let moves = legal_moves(pos);
        if moves.is_empty() {
            return no_moves_score(pos, ply);
        }
        let mut best = -INFINITY;
        for &mv in &moves {
            let undo = pos.make_move(mv);
            best = best.max(-minimax(pos, depth - 1, ply + 1, nodes));
            pos.unmake_move(mv, undo);
        }
        best
    }

    /// Pruning must never change the answer, only the work.
    #[test]
    fn alpha_beta_agrees_with_minimax() {
        // Unpruned minimax with a full quiescence search at every leaf is
        // very expensive on busy positions, so those go one ply less deep.
        let positions = [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                3,
            ),
            (KIWIPETE, 2),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 3),
            (
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
                2,
            ),
            (
                "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
                2,
            ),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 b - - 0 10",
                2,
            ),
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 3),
            ("4k3/8/8/3q4/8/8/8/3RK3 b - - 0 1", 3),
            (
                "r1bqk2r/pppp1ppp/2n2n2/4p3/1bB1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 0 1",
                2,
            ),
        ];
        for (text, deepest) in positions {
            for depth in 1..=deepest {
                let mut pos = fen(text);
                let mut minimax_nodes = 0;
                let expected = minimax(&mut pos, depth, 0, &mut minimax_nodes);

                let mut searcher = Search::new(&HandCrafted);
                let outcome = searcher.fixed_depth(&pos, depth);
                assert_eq!(outcome.score, expected, "{text} depth {depth}");
                assert!(searcher.nodes() <= minimax_nodes, "{text} depth {depth}");
                assert_eq!(pos, fen(text), "the position must be restored");

                // The move it names really leads to that score.
                let mv = outcome.best_move.unwrap();
                let undo = pos.make_move(mv);
                let reply = minimax(&mut pos, depth - 1, 1, &mut 0);
                pos.unmake_move(mv, undo);
                assert_eq!(-reply, expected, "{text} depth {depth} move {mv}");
            }
        }
    }

    #[test]
    fn pruning_saves_work_at_depth() {
        let text = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
        let mut minimax_nodes = 0;
        minimax(&mut fen(text), 4, 0, &mut minimax_nodes);
        let mut searcher = Search::new(&HandCrafted);
        searcher.fixed_depth(&fen(text), 4);
        assert!(searcher.nodes() * 2 < minimax_nodes);
    }

    #[test]
    fn takes_a_hanging_queen() {
        assert_eq!(best("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 1), "d1d5");
        assert_eq!(best("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 3), "d1d5");
        // And does not put its own queen where it can be taken for nothing.
        let outcome = search("4k3/8/8/3q4/8/8/8/3RK3 b - - 0 1", 2);
        assert!(outcome.score > 300, "Black keeps the queen: {outcome:?}");
    }

    #[test]
    fn finds_mate_in_one() {
        let text = "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1";
        let outcome = search(text, 2);
        assert_eq!(outcome.score, mate_in(1));
        assert_eq!(best(text, 2), "a1a8");
        assert_eq!(
            search(text, 1).score,
            mate_in(1),
            "seen at depth 1 through the check"
        );
        // Deeper searches still report the same, quickest, mate.
        assert_eq!(search(text, 4).score, mate_in(1));
    }

    #[test]
    fn finds_mate_in_two() {
        // Two rooks against a bare king in the corner: cut off the seventh
        // rank, then mate on the eighth. Either rook can go first.
        let text = "7k/8/8/8/8/8/R7/1R4K1 w - - 0 1";
        let outcome = search(text, 4);
        assert_eq!(outcome.score, mate_in(3));
        assert!(["b1b7", "a2a7"].contains(&best(text, 4).as_str()));
        // One ply shallower the mating move is the last one searched; the
        // quiescence search sees that the reply to its check does not exist.
        assert_eq!(search(text, 3).score, mate_in(3));
        // Shallower than that the mate is beyond the horizon.
        assert!(!crate::search::is_mate_score(search(text, 2).score));
    }

    #[test]
    fn sees_its_own_defeat_coming() {
        // After 1. Rb7 Black's only move is Kg8, and Ra8 is mate.
        let outcome = search("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 3);
        assert_eq!(outcome.score, mated_in(2));
        assert_eq!(outcome.best_move.unwrap().to_string(), "h8g8");
    }

    #[test]
    fn checkmate_and_stalemate_at_the_root() {
        let mated = search("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", 3);
        assert_eq!(mated.score, mated_in(0));
        assert_eq!(mated.best_move, None);
        let stalemate = search("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 3);
        assert_eq!(stalemate.score, DRAW);
        assert_eq!(stalemate.best_move, None);
    }

    /// A queen up with two tempting moves, Qf7 and Qg6, that leave the bare
    /// king without a legal move and not in check. A search that scored
    /// "no legal moves" as a win would play one of them; this one must count
    /// them as draws and keep the queen instead.
    #[test]
    fn stalemate_is_scored_as_a_draw_inside_the_tree() {
        let text = "7k/8/4QK2/8/8/8/8/8 w - - 0 1";
        for stalemating in ["e6f7", "e6g6"] {
            let mut pos = fen(text);
            let mv = pos.parse_move(stalemating).unwrap();
            let _undo = pos.make_move(mv);
            assert!(
                legal_moves(&pos).is_empty() && !pos.in_check(),
                "{stalemating}"
            );
        }
        let outcome = search(text, 2);
        let chosen = best(text, 2);
        assert!(chosen != "e6f7" && chosen != "e6g6", "{chosen}");
        assert!(outcome.score > 500, "{outcome:?}");
    }

    /// The principal variation must be a line of legal moves that really
    /// leads to the reported score.
    #[test]
    fn the_principal_variation_explains_the_score() {
        // Busy middlegames are searched less deep: without move ordering
        // (milestone 7) every extra ply there costs a great deal.
        let positions = [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                3,
            ),
            (KIWIPETE, 2),
            (ROOK_ENDGAME, 4),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 b - - 0 10",
                2,
            ),
            ("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 4),
        ];
        for (text, deepest) in positions {
            for depth in 1..=deepest {
                let outcome = search(text, depth);
                assert_eq!(outcome.pv.first().copied(), outcome.best_move, "{text}");
                assert!(outcome.pv.len() <= depth as usize, "{text} depth {depth}");

                let mut pos = fen(text);
                for mv in &outcome.pv {
                    assert!(legal_moves(&pos).contains(mv), "{text}: {mv} is not legal");
                    let _undo = pos.make_move(*mv);
                }
                // These positions have no mate or draw within these depths, so
                // the line runs the full depth and ends in a position whose
                // quiescence value, seen from the root side, is the score.
                assert_eq!(outcome.pv.len(), depth as usize, "{text} depth {depth}");
                let from_root_side = if depth % 2 == 0 { 1 } else { -1 };
                let leaf = quiet_value(&pos, depth as usize) * from_root_side;
                assert_eq!(leaf, outcome.score, "{text} depth {depth}");
            }
        }
    }

    #[test]
    fn the_principal_variation_of_a_mate_ends_in_checkmate() {
        let outcome = search("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 4);
        assert_eq!(
            outcome.pv.len(),
            3,
            "mate in two is three plies: {:?}",
            outcome.pv
        );
        let mut pos = fen("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1");
        for mv in &outcome.pv {
            assert!(legal_moves(&pos).contains(mv));
            let _undo = pos.make_move(*mv);
        }
        assert!(pos.in_check() && legal_moves(&pos).is_empty());

        // No legal move at the root: nothing to play.
        assert!(search("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", 3).pv.is_empty());
    }

    /// Plays `moves` from `pos` and returns the hashes of the positions
    /// passed through, for use as game history.
    fn play(pos: &mut Position, moves: &[&str]) -> Vec<u64> {
        let mut keys = Vec::new();
        for text in moves {
            keys.push(pos.hash());
            let mv = pos.parse_move(text).unwrap();
            let _undo = pos.make_move(mv);
        }
        keys
    }

    /// A queen down, Black can still draw by returning to a position the
    /// game has already been through. The search sees that only when it is
    /// told the game's history.
    #[test]
    fn the_losing_side_takes_a_repetition_from_the_game_history() {
        let mut pos = fen("7k/8/8/8/8/2K5/8/1Q6 w - - 10 30");
        let history = play(&mut pos, &["b1c1", "h8g8", "c1b1"]);
        assert_eq!(pos.to_fen(), "6k1/8/8/8/8/2K5/8/1Q6 b - - 13 31");

        let blind = Search::new(&HandCrafted).fixed_depth(&pos.clone(), 2);
        assert!(blind.score < -500, "without history Black is just lost");

        let informed = Search::new(&HandCrafted)
            .with_history(&history)
            .fixed_depth(&pos, 2);
        assert_eq!(informed.score, DRAW);
        assert_eq!(informed.best_move.unwrap().to_string(), "g8h8");
    }

    /// The winning side must not walk into a repetition inside its own
    /// search line, with no game history at all.
    #[test]
    fn the_winning_side_avoids_repeating_within_the_search() {
        let text = "7k/8/8/8/8/2K5/8/1Q6 w - - 10 30";
        // b1c1, h8g8, c1b1, g8h8 returns to the root position.
        let outcome = search(text, 5);
        assert!(outcome.score > 500, "{outcome:?}");
    }

    #[test]
    fn the_fifty_move_rule_ends_the_line() {
        // Mate in two is on the board, but every move brings the clock to
        // 100 and nothing can reset it.
        assert_eq!(search("7k/8/8/8/8/8/R7/1R4K1 w - - 99 80", 4).score, DRAW);
        // With one ply less on the clock the mate is still too late.
        assert_eq!(search("7k/8/8/8/8/8/R7/1R4K1 w - - 98 80", 4).score, DRAW);
        // Delivered on exactly the hundredth ply, the mate counts.
        assert_eq!(
            search("7k/8/8/8/8/8/R7/1R4K1 w - - 97 80", 4).score,
            mate_in(3)
        );
        assert_eq!(
            search("7k/8/8/8/8/8/R7/1R4K1 w - - 0 80", 4).score,
            mate_in(3)
        );
    }

    #[test]
    fn bare_material_is_a_draw_inside_the_tree() {
        // After the rooks come off there are only kings: White must
        // recapture (the alternatives stay a rook down) and that is a draw.
        let outcome = search("4k3/8/8/8/8/8/8/3rK3 w - - 0 1", 2);
        assert_eq!(outcome.score, DRAW);
        assert_eq!(outcome.best_move.unwrap().to_string(), "e1d1");
        // King and knight against king: every line is drawn at once.
        assert_eq!(search("4k3/8/8/8/8/8/8/3NK3 w - - 0 1", 3).score, DRAW);
    }

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    /// Perft position 3: few pieces, so deeper searches stay quick.
    const ROOK_ENDGAME: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";

    #[test]
    fn a_node_limit_stops_the_search_and_restores_everything() {
        let original = fen(KIWIPETE);
        let pos = original.clone();
        let mut searcher = Search::new(&HandCrafted)
            .with_history(&[1, 2, 3])
            .with_limits(Limits::nodes(500));
        let outcome = searcher.fixed_depth(&pos, 4);
        assert!(searcher.was_stopped());
        assert_eq!(searcher.nodes(), 500, "stops exactly at the limit");
        // Unwinding took every move back.
        assert_eq!(pos, original);
        assert_eq!(searcher.keys, [1, 2, 3]);
        // Whatever move it names was searched completely and is legal.
        if let Some(mv) = outcome.best_move {
            assert!(legal_moves(&original).contains(&mv));
        }
    }

    #[test]
    fn a_generous_node_limit_changes_nothing() {
        let unlimited = search(ROOK_ENDGAME, 3);
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::nodes(u64::MAX));
        let limited = searcher.fixed_depth(&fen(ROOK_ENDGAME), 3);
        assert!(!searcher.was_stopped());
        assert_eq!(limited, unlimited);
    }

    #[test]
    fn a_raised_stop_flag_ends_the_search_at_the_next_check() {
        let flag = AtomicBool::new(true);
        let pos = fen(KIWIPETE);
        let mut searcher = Search::new(&HandCrafted).with_stop_flag(&flag);
        let outcome = searcher.fixed_depth(&pos, 5);
        assert!(searcher.was_stopped());
        // Seen before the first node, since zero is a multiple of the interval.
        assert_eq!(searcher.nodes(), 0);
        assert_eq!(outcome.best_move, None);
        assert_eq!(pos, fen(KIWIPETE));

        // A flag that is never raised does not disturb the search.
        let calm = AtomicBool::new(false);
        let mut searcher = Search::new(&HandCrafted).with_stop_flag(&calm);
        assert_eq!(searcher.fixed_depth(&pos, 2), search(KIWIPETE, 2));
        assert!(!searcher.was_stopped());
    }

    #[test]
    fn a_passed_deadline_ends_the_search() {
        let pos = fen(KIWIPETE);
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::until(Instant::now()));
        searcher.fixed_depth(&pos, 6);
        assert!(searcher.was_stopped());
        assert!(searcher.nodes() <= CHECK_INTERVAL);
        assert_eq!(pos, fen(KIWIPETE));
    }

    /// Runs iterative deepening and collects the reports.
    fn iterate(text: &str, limits: Limits) -> (Outcome, Vec<Report>, bool) {
        let mut reports = Vec::new();
        let mut searcher = Search::new(&HandCrafted).with_limits(limits);
        let outcome = searcher.iterate(&fen(text), &mut |report| reports.push(report.clone()));
        (outcome, reports, searcher.was_stopped())
    }

    /// With no ordering tricks or tables, deepening to depth N must give
    /// exactly what a direct depth-N search gives.
    #[test]
    fn iterating_to_a_depth_equals_searching_that_depth() {
        for text in [
            crate::board::START_FEN,
            "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1",
            ROOK_ENDGAME,
        ] {
            let (outcome, reports, stopped) = iterate(text, Limits::depth(3));
            assert!(!stopped);
            assert_eq!(outcome, search(text, 3), "{text}");
            // One report per depth, in order, each matching that depth.
            assert_eq!(reports.len(), 3);
            for (report, depth) in reports.iter().zip(1..) {
                assert_eq!(report.depth, depth);
                let direct = search(text, depth);
                assert_eq!((report.score, &report.pv), (direct.score, &direct.pv));
            }
            assert!(reports.windows(2).all(|pair| pair[0].nodes < pair[1].nodes));
            assert!(
                reports
                    .windows(2)
                    .all(|pair| pair[0].elapsed <= pair[1].elapsed)
            );
        }
    }

    #[test]
    fn iteration_stops_once_a_mate_is_proven() {
        // Mate in two (three plies) is proven at depth 3; nothing deeper is
        // searched.
        let (outcome, reports, _) = iterate("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", Limits::depth(12));
        assert_eq!(outcome.score, mate_in(3));
        assert_eq!(reports.last().unwrap().depth, 3);
        // Being mated in one (two plies) is proven at depth 2.
        let (lost, reports, _) = iterate("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", Limits::depth(12));
        assert_eq!(lost.score, mated_in(2));
        assert_eq!(reports.last().unwrap().depth, 2);
    }

    #[test]
    fn iteration_ends_at_once_when_there_is_nothing_to_search() {
        let (mated, reports, _) = iterate("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", Limits::depth(9));
        assert_eq!((mated.score, mated.best_move), (mated_in(0), None));
        assert_eq!(reports.len(), 1);
        let (stale, reports, _) = iterate("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", Limits::depth(9));
        assert_eq!((stale.score, stale.best_move), (DRAW, None));
        assert_eq!(reports.len(), 1);
    }

    /// A limit that interrupts an iteration discards it: the answer is the
    /// last depth that finished, which is always a complete, legal answer.
    #[test]
    fn an_interrupted_iteration_is_discarded() {
        // Find how many nodes three and four iterations take, then set the
        // limit between them so the fourth is cut off partway.
        let (_, three, _) = iterate(ROOK_ENDGAME, Limits::depth(3));
        let (_, four, _) = iterate(ROOK_ENDGAME, Limits::depth(4));
        let (after_three, after_four) = (three[2].nodes, four[3].nodes);
        let limit = after_three + (after_four - after_three) / 2;

        let (outcome, reports, stopped) = iterate(ROOK_ENDGAME, Limits::nodes(limit));
        assert!(stopped);
        assert_eq!(reports.last().unwrap().depth, 3);
        assert_eq!(outcome, search(ROOK_ENDGAME, 3));
    }

    #[test]
    fn the_first_iteration_always_completes() {
        // A node limit smaller than one iteration, and a raised stop flag.
        let (outcome, reports, _) = iterate(KIWIPETE, Limits::nodes(1));
        assert_eq!(reports.len(), 1);
        assert_eq!(outcome, search(KIWIPETE, 1));

        let flag = AtomicBool::new(true);
        let mut searcher = Search::new(&HandCrafted).with_stop_flag(&flag);
        let mut count = 0;
        let outcome = searcher.iterate(&fen(KIWIPETE), &mut |_| count += 1);
        assert_eq!(count, 1);
        assert!(legal_moves(&fen(KIWIPETE)).contains(&outcome.best_move.unwrap()));

        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::until(Instant::now()));
        let outcome = searcher.iterate(&fen(KIWIPETE), &mut |_| {});
        assert_eq!(outcome, search(KIWIPETE, 1));
    }

    #[test]
    fn a_depth_limit_of_zero_still_searches_one_ply() {
        let (outcome, reports, _) = iterate(KIWIPETE, Limits::depth(0));
        assert_eq!(reports.len(), 1);
        assert_eq!(outcome, search(KIWIPETE, 1));
    }

    #[test]
    fn a_quiet_position_is_worth_its_static_evaluation() {
        for text in [
            crate::board::START_FEN,
            "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
            "r4rk1/ppp2ppp/8/8/8/8/PPP2PPP/R4RK1 w - - 0 1",
        ] {
            let pos = fen(text);
            assert_eq!(quiet_value(&pos, 0), HandCrafted.evaluate(&pos), "{text}");
            assert_eq!(pos, fen(text));
        }
    }

    #[test]
    fn quiescence_plays_out_a_winning_capture() {
        // White is a queen for a rook down on the board, but takes the queen.
        let pos = fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1");
        assert!(HandCrafted.evaluate(&pos) < -300);
        assert!(quiet_value(&pos, 0) > 300);
    }

    /// The horizon effect: at depth 1 a search that evaluated straight away
    /// would take the pawn with the queen, not seeing the recapture.
    #[test]
    fn does_not_grab_a_defended_pawn_with_the_queen() {
        let text = "4k3/8/3p4/4p3/8/8/4Q3/4K3 w - - 0 1";
        // The capture really is available and really loses the queen.
        let mut pos = fen(text);
        let grab = pos.parse_move("e2e5").unwrap();
        assert!(legal_moves(&pos).contains(&grab));
        let _undo = pos.make_move(grab);
        // After dxe5 White has a bare king against a pawn.
        assert!(-quiet_value(&pos, 1) < 0, "the queen is lost after dxe5");

        for depth in 1..=3 {
            assert_ne!(best(text, depth), "e2e5", "depth {depth}");
            assert!(search(text, depth).score > 500, "depth {depth}");
        }
    }

    #[test]
    fn quiescence_does_not_stand_pat_in_check() {
        // Checkmated: no standing pat, no moves.
        let mated = fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1");
        assert_eq!(quiet_value(&mated, 0), mated_in(0));
        assert_eq!(quiet_value(&mated, 5), mated_in(5));
        // In check with two ways out, taking the queen or stepping aside:
        // the value is what the best of them leads to, not the static
        // evaluation of the checked position.
        let checked = fen("4k3/8/8/8/8/8/3q4/4K3 w - - 0 1");
        assert_eq!(legal_moves(&checked).len(), 2);
        // King takes queen: bare kings, which is level.
        assert_eq!(quiet_value(&checked, 0), 0);
        assert!(HandCrafted.evaluate(&checked) < -800);
    }

    #[test]
    fn quiescence_considers_queen_promotions() {
        let pos = fen("8/4P1k1/8/8/8/8/8/4K3 w - - 0 1");
        let before = HandCrafted.evaluate(&pos);
        assert!(quiet_value(&pos, 0) > before + 500);
    }

    #[test]
    fn quiescence_terminates_on_a_position_full_of_captures() {
        let mut searcher = Search::new(&HandCrafted);
        let pos = fen(KIWIPETE);
        let value = searcher.quiesce(&pos, 0, -INFINITY, INFINITY);
        assert!(value.abs() < 2_000, "{value}");
        assert!(searcher.nodes() < 1_000_000, "{}", searcher.nodes());
        assert_eq!(pos, fen(KIWIPETE));
    }

    #[test]
    fn tactical_moves() {
        use crate::board::{MoveKind, Square};
        let mv = |kind| Move::new(Square::E7, Square::E8, kind);
        assert!(is_tactical(mv(MoveKind::Capture)));
        assert!(is_tactical(mv(MoveKind::EnPassant)));
        assert!(is_tactical(mv(MoveKind::Promotion(PieceKind::Queen))));
        assert!(is_tactical(mv(MoveKind::PromotionCapture(
            PieceKind::Knight
        ))));
        assert!(!is_tactical(mv(MoveKind::Promotion(PieceKind::Rook))));
        assert!(!is_tactical(mv(MoveKind::Quiet)));
        assert!(!is_tactical(mv(MoveKind::DoublePush)));
        assert!(!is_tactical(mv(MoveKind::KingCastle)));
    }

    #[test]
    fn counts_nodes() {
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.nodes(), 0);
        searcher.fixed_depth(&Position::startpos(), 1);
        // The root plus its 20 children.
        assert_eq!(searcher.nodes(), 21);
    }

    #[test]
    #[should_panic(expected = "between 1 and MAX_PLY")]
    fn depth_zero_is_rejected() {
        Search::new(&HandCrafted).fixed_depth(&Position::startpos(), 0);
    }
}
