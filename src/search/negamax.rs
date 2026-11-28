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

use super::ordering::{Killers, captures_first, is_quiet, loses_material, table_move_first};
use super::{
    Bound, DRAW, INFINITY, Limits, MATE, MATE_BOUND, MAX_PLY, SearchStats, TranspositionTable,
    fifty_moves_reached, insufficient_material, is_repetition, mated_in, moves_to_mate,
};
use crate::attacks::knight_attacks;
use crate::board::{Color, Move, MoveKind, PieceKind, Position, Square};
use crate::eval::{Evaluator, Score};
use crate::movegen::{MoveList, legal_moves, pseudo_legal_moves};

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
    /// The selective depth: how many plies from the root the deepest
    /// position of this iteration was, quiescence search included. It is 0
    /// when the root has no legal move.
    pub seldepth: u32,
    /// The score at that depth, for the side to move.
    pub score: Score,
    /// Positions visited since the search began, over all iterations.
    pub nodes: u64,
    /// Time since the search began.
    pub elapsed: Duration,
    /// How full the transposition table is, in slots per thousand
    /// ([`TranspositionTable::hashfull`]), or `None` for a search without
    /// a table.
    pub hashfull: Option<u32>,
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
    /// The only moves searched at the root, if any are named (see
    /// [`Search::with_root_moves`]); empty for all of them.
    root_moves: Vec<Move>,
    /// Set from outside (another thread) to end the search early.
    stop_flag: Option<&'a AtomicBool>,
    /// Latched once any limit is hit: from then on every node returns at
    /// once and the search unwinds.
    stopped: bool,
    /// What the first iteration is spared, so that there is always a move
    /// to play.
    shield: Shield,
    /// The deepest ply visited since the last search from the root began.
    /// It is only ever written here and read by [`Search::seldepth`]: the
    /// search decides nothing by it.
    seldepth: usize,
    /// Counts of what the search did. Like `seldepth`, written here and
    /// read only from outside, by [`Search::stats`].
    stats: SearchStats,
    /// What earlier searching found out about positions, if the caller
    /// lent a table (see [`Search::with_table`]). Without one every
    /// position is searched afresh.
    table: Option<&'a mut TranspositionTable>,
    /// The killer moves of each ply: `killers[p]` holds the quiet moves
    /// that last cut off the search at ply `p`. They last for one search,
    /// through all its iterations, and start empty in the next.
    killers: Vec<Killers>,
    /// `no_null[p]` is set while the position at ply `p` must not try a
    /// null move: it was reached by one, or it is being searched to verify
    /// one.
    no_null: Vec<bool>,
    /// `verifying[p]` is set while the position at ply `p` is searched to
    /// verify a null move: its own moves must then really be searched, so
    /// nothing may cut it off from its evaluation alone.
    verifying: Vec<bool>,
    /// Whether null-move pruning is used at all. Always true outside the
    /// tests, which compare the search with and without it.
    null_moves: bool,
    /// What null-move pruning did in this search. Not part of the
    /// statistics that are shown; the tests read it.
    nulls: NullCounts,
    /// Whether reverse futility pruning is used. Always true outside the
    /// tests.
    futility: bool,
    /// How many positions it cut off in this search. For the tests.
    futile: u64,
    /// Whether late moves are searched less deep. Always true outside
    /// the tests, which compare the search with and without it.
    reductions: bool,
    /// How many moves were searched reduced in this search, and how many
    /// of them had to be searched again in full. For the tests, like
    /// `nulls`.
    reduced: (u64, u64),
}

/// Counts of null-move pruning in one search.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NullCounts {
    /// Null moves searched.
    tried: u64,
    /// Those whose reduced search came back at or above beta.
    failed_high: u64,
    /// Of those, the ones checked by a search without the null move
    /// because the side to move had a single piece left.
    verified: u64,
    /// Nodes cut off in the end.
    cutoffs: u64,
}

/// A move is reduced from this depth up: below it there is nothing to
/// take off without falling into quiescence search.
const LMR_MIN_DEPTH: u32 = 3;

/// The first moves of a position are never reduced: this many are
/// searched in full before any is.
const LMR_FULL_MOVES: u32 = 3;

/// The natural logarithm of 1 to 63, times 1,000 and rounded. A table of
/// whole numbers and not a call of `ln`, so that the reductions, and with
/// them every node count, are the same on every machine.
const LN_THOUSANDTHS: [u32; 63] = [
    0, 693, 1099, 1386, 1609, 1792, 1946, 2079, 2197, 2303, 2398, 2485, 2565, 2639, 2708, 2773,
    2833, 2890, 2944, 2996, 3045, 3091, 3135, 3178, 3219, 3258, 3296, 3332, 3367, 3401, 3434, 3466,
    3497, 3526, 3555, 3584, 3611, 3638, 3664, 3689, 3714, 3738, 3761, 3784, 3807, 3829, 3850, 3871,
    3892, 3912, 3932, 3951, 3970, 3989, 4007, 4025, 4043, 4060, 4078, 4094, 4111, 4127, 4143,
];

/// How many plies less a late quiet move is searched: the move that is
/// tried after `searched` others, at `depth`. It is
/// `0.75 + ln(depth) * ln(searched) / 2.25`, rounded down: one ply for
/// the first late moves at low depth, more the later the move and the
/// deeper the search, since both make it likelier that the move changes
/// nothing. Always at least one ply is left to search.
fn late_move_reduction(depth: u32, searched: u32) -> u32 {
    let ln = |n: u32| LN_THOUSANDTHS[n.clamp(1, 63) as usize - 1];
    let reduction = (1_687_500 + ln(depth) * ln(searched)) / 2_250_000;
    reduction.min(depth.saturating_sub(2))
}

/// Reverse futility pruning is used up to this depth: further from the
/// horizon too much can still happen for the evaluation to be trusted.
const RFP_MAX_DEPTH: u32 = 6;

/// How far above beta the evaluation must be, per ply of depth left, for
/// reverse futility pruning: about a pawn for each move the opponent has
/// to change things with.
const RFP_MARGIN: Score = 100;

/// A null move is tried from this depth up. Below it the reduced search
/// would be a quiescence search of a position hardly different from the
/// one whose evaluation already said "at least beta".
const NULL_MIN_DEPTH: u32 = 3;

/// How much shallower the search after a null move is than the search of
/// a real move would be, at `depth`: two plies, and one more for every
/// four plies of depth. The deeper the search, the more it can afford to
/// take on trust.
const fn null_reduction(depth: u32) -> u32 {
    2 + depth / 4
}

/// The pieces of the side to move that are not pawns or the king.
fn pieces_of_mover(pos: &Position) -> u32 {
    let side = pos.side_to_move();
    [
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ]
    .iter()
    .map(|&kind| pos.pieces_of(side, kind).count())
    .sum()
}

/// Which limits the search does not look at for the moment. The first
/// iteration of [`Search::iterate`] must leave a move to play, so nothing
/// may end it before one root move has been searched to the end; after
/// that the clock and the stop flag may, and the answer is then the best
/// of the root moves that were finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shield {
    /// Every limit applies: every search but the first iteration.
    Nothing,
    /// No limit applies: the first iteration, until its first legal root
    /// move has been searched.
    Everything,
    /// The node limit does not apply, and the stop flag and the clock are
    /// looked at only once per [`FIRST_DEPTH_CHECK_INTERVAL`] nodes: the
    /// rest of the first iteration. A node limit asks for a reproducible
    /// search, and what one gives stays as it was: the whole first depth.
    NodeLimit,
}

/// How often the stop flag and the clock are looked at during the first
/// iteration, once its first root move is done. A first depth is nearly
/// always far shorter than this (tens to a few thousand nodes) and then
/// completes whatever the clock says, as it always has, so that a move
/// played with no time left is still the best of all the moves at one
/// ply. Only a first depth that takes tens of milliseconds, in a position
/// with very many captures, can be cut short.
const FIRST_DEPTH_CHECK_INTERVAL: u64 = 1 << 16;

/// From this many half-moves without a capture or a pawn move, a result in
/// the table is not taken on trust: it was found with another count, and
/// the fifty-move rule is about to make the count matter.
const FIFTY_MOVE_GUARD: u16 = 90;

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
            root_moves: Vec::new(),
            stop_flag: None,
            stopped: false,
            shield: Shield::Nothing,
            seldepth: 0,
            stats: SearchStats::ZERO,
            table: None,
            killers: vec![Killers::NONE; MAX_PLY],
            no_null: vec![false; MAX_PLY + 2],
            verifying: vec![false; MAX_PLY + 2],
            null_moves: true,
            nulls: NullCounts::default(),
            futility: true,
            futile: 0,
            reductions: true,
            reduced: (0, 0),
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

    /// Restricts the search to these moves of the position searched (UCI
    /// `searchmoves`): the outcome is the best of them, and the others are
    /// not looked at. An empty list restricts nothing.
    ///
    /// The moves must be legal in that position. A restricted search does
    /// not record the root in the table, where its result would pass for
    /// the value of the position.
    #[must_use]
    pub fn with_root_moves(mut self, moves: &[Move]) -> Search<'a> {
        self.root_moves = moves.to_vec();
        self
    }

    /// Lends the search a transposition table: it uses what the table
    /// knows and adds what it finds. The table outlives the search, so the
    /// next search, of the next move of the game, starts with it.
    ///
    /// A search with a table is no longer a function of the position alone:
    /// what it visits depends on what the table held. It is still
    /// repeatable, since the same searches in the same order fill the table
    /// the same way, and a new table makes it a function of the position
    /// again.
    #[must_use]
    pub fn with_table(mut self, table: &'a mut TranspositionTable) -> Search<'a> {
        self.table = Some(table);
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
        if self.shield == Shield::Everything {
            return false;
        }
        let first_depth = self.shield == Shield::NodeLimit;
        let counted_out =
            !first_depth && self.limits.nodes.is_some_and(|limit| self.nodes >= limit);
        let interval = if first_depth {
            FIRST_DEPTH_CHECK_INTERVAL
        } else {
            CHECK_INTERVAL
        };
        if counted_out {
            self.stopped = true;
        } else if self.nodes.is_multiple_of(interval) {
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
    /// The outcome always names a move if one exists: no limit ends the
    /// first iteration before its first legal root move has been searched,
    /// and a node limit never ends it at all. The stop flag and the clock
    /// may end it after that first move, where it is a long one (they are
    /// looked at rarely during it); the outcome is then the best of
    /// the root moves that were finished, [`Search::was_stopped`] is true
    /// and nothing is reported. A later iteration that a limit interrupts
    /// is thrown away. The search also ends early once it has found a
    /// forced mate that fits inside the depth just searched, because
    /// searching deeper cannot find a shorter one.
    ///
    /// Each call is a new search: the node count and the stopped state start
    /// afresh, so one `Search` can be used for move after move.
    pub fn iterate(&mut self, pos: &Position, report: &mut dyn FnMut(&Report)) -> Outcome {
        let start = Instant::now();
        self.begin();
        let deepest = self
            .limits
            .depth
            .map_or(MAX_PLY as u32, |depth| depth.clamp(1, MAX_PLY as u32));
        let mut best: Option<Outcome> = None;
        for depth in 1..=deepest {
            self.shield = if depth == 1 {
                Shield::Everything
            } else {
                Shield::Nothing
            };
            let outcome = self.search_root(pos, depth);
            if self.stopped {
                // Only the first iteration can be cut short with nothing
                // before it, and then it has finished at least one move.
                best = best.or(Some(outcome));
                break;
            }
            report(&Report {
                depth,
                seldepth: self.seldepth(),
                score: outcome.score,
                nodes: self.nodes,
                elapsed: start.elapsed(),
                hashfull: self.table.as_deref().map(TranspositionTable::hashfull),
                pv: outcome.pv.clone(),
            });
            let nothing_to_search = outcome.best_move.is_none();
            // A mate score counts plies from the root; within `depth` it was
            // found by the full-width search and is the shortest there is.
            let mate_found = MATE - outcome.score.abs() <= depth as Score;
            // The mate that was asked for (`Limits::mate`), however deep
            // the search had to look to see it.
            let mate_wanted = self.limits.mate.is_some_and(|within| {
                moves_to_mate(outcome.score)
                    .is_some_and(|moves| moves > 0 && moves.unsigned_abs() <= within)
            });
            best = Some(outcome);
            if nothing_to_search || mate_found || mate_wanted || self.limit_reached_now() {
                break;
            }
        }
        self.shield = Shield::Nothing;
        best.expect("the first iteration is not interrupted before its first move")
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

    /// Starts a new search: forgets the node count and the stopped state of
    /// the one before. The game history and the limits are kept.
    fn begin(&mut self) {
        self.nodes = 0;
        self.stopped = false;
        self.seldepth = 0;
        self.stats = SearchStats::ZERO;
        self.killers.fill(Killers::NONE);
        self.no_null.fill(false);
        self.verifying.fill(false);
        self.nulls = NullCounts::default();
        self.reduced = (0, 0);
        self.futile = 0;
    }

    /// What the current or most recent search has counted: cutoffs, how
    /// many of them the first move gave, and the quiescence search's share.
    /// An interrupted iteration's work is included, as it is in
    /// [`Search::nodes`].
    #[must_use]
    pub const fn stats(&self) -> SearchStats {
        self.stats
    }

    /// The selective depth of the current or most recent search from the
    /// root: how many plies from it the deepest position visited was,
    /// quiescence search included. Under [`Search::iterate`] it is that of
    /// the iteration last started, which a limit may have interrupted; the
    /// value of each completed iteration is in its [`Report`].
    #[must_use]
    pub const fn seldepth(&self) -> u32 {
        self.seldepth as u32
    }

    /// How many positions the current or most recent search has visited.
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
    /// Each call is a new search, as with [`Search::iterate`].
    ///
    /// # Panics
    ///
    /// Panics if `depth` is 0 or more than [`MAX_PLY`].
    pub fn fixed_depth(&mut self, pos: &Position, depth: u32) -> Outcome {
        assert!(
            depth >= 1 && depth as usize <= MAX_PLY,
            "search depth must be between 1 and MAX_PLY"
        );
        self.begin();
        self.search_root(pos, depth)
    }

    /// The value of `pos` for its side to move, searched `depth` plies deep
    /// (0 means quiescence search only), when `pos` is **one move away from
    /// the position a move is being chosen for**. The history given with
    /// [`Search::with_history`] must then end with that position's hash.
    ///
    /// Unlike [`Search::fixed_depth`], this treats `pos` as a node inside a
    /// search and not as its root: a position drawn by rule is worth
    /// [`DRAW`], and a mate is counted from the position before it. It is
    /// what scoring the root moves one at a time needs.
    ///
    /// Each call is a new search. If a limit cuts it short,
    /// [`Search::was_stopped`] is true and the value means nothing.
    pub fn reply_value(&mut self, pos: &Position, depth: u32) -> Score {
        self.begin();
        let depth = depth.min(MAX_PLY as u32 - 1);
        self.negamax(pos, depth, 1, -INFINITY, INFINITY)
    }

    /// One fixed-depth search from the root, as part of the search already
    /// begun: [`Search::iterate`] calls this once per depth and the node
    /// count carries on from one depth to the next.
    fn search_root(&mut self, pos: &Position, depth: u32) -> Outcome {
        self.pv[0].clear();
        self.seldepth = 0;
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
        // The best move of the iteration before, which the table holds,
        // then captures and promotions: see `ordering.rs`. The root is
        // never settled from the table, only ordered by it.
        let key = pos.hash();
        let table_move = self
            .table
            .as_deref()
            .and_then(|table| table.probe(key, 0))
            .and_then(|known| known.best);
        // No killer moves at the root: nothing cuts off there.
        let moves = table_move_first(pos, &pseudo_legal_moves(pos), table_move, Killers::NONE);
        let restricted = !self.root_moves.is_empty();
        for &mv in &moves {
            if restricted && !self.root_moves.contains(&mv) {
                continue;
            }
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
            if self.shield == Shield::Everything {
                // There is a move to play now: the stop flag and the clock
                // may end the first iteration from here on.
                self.shield = Shield::NodeLimit;
            }
            // The first legal move is always taken, so a best move exists
            // whenever any legal move does.
            if best_move.is_none() || score > alpha {
                alpha = score;
                best_move = Some(mv);
                self.update_pv(0, mv);
            }
        }
        // Remember the root's best move for the next iteration and the next
        // search of the game. Not after a stop: the move may be the best
        // of only the moves that were searched.
        if let (Some(table), false) = (self.table.as_deref_mut(), self.stopped || restricted)
            && best_move.is_some()
        {
            table.store(key, 0, depth, alpha, Bound::Exact, best_move);
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
        self.seldepth = self.seldepth.max(ply);
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

        // Has this position been searched at least this deep before? Then
        // what was found may settle it: its value, or a bound on the value
        // that already lies outside the window.
        let key = pos.hash();
        let mut table_move = None;
        let mut known = None;
        if let Some(table) = self.table.as_deref() {
            self.stats.table_probes += 1;
            known = table.probe(key, ply);
        }
        if let Some(known) = known {
            self.stats.table_hits += 1;
            // Whatever the depth it was found at, the move that was best
            // before is the one to try first now.
            table_move = known.best;
            if known.depth >= depth && pos.halfmove_clock() < FIFTY_MOVE_GUARD {
                let settles = match known.bound {
                    Bound::Exact => true,
                    Bound::Lower => known.score >= beta,
                    Bound::Upper => known.score <= alpha,
                };
                if settles {
                    self.stats.table_cutoffs += 1;
                    return known.score;
                }
            }
        }

        if !self.verifying[ply]
            && let Some(score) = self.reverse_futility_cutoff(pos, depth, beta)
        {
            self.futile += 1;
            return score;
        }
        if let Some(score) = self.null_move_cutoff(pos, depth, ply, beta) {
            if let Some(table) = self.table.as_deref_mut() {
                table.store(key, ply, depth, score, Bound::Lower, table_move);
            }
            return score;
        }
        if self.stopped {
            return DRAW;
        }

        let alpha_at_first = alpha;
        let mut best = -INFINITY;
        let mut best_move = None;
        let mut any_legal = false;
        // The table's move first, then captures and promotions, most
        // valuable victim first, then this ply's killer moves: see
        // `ordering.rs`.
        let killers = self.killers[ply];
        let moves = table_move_first(pos, &pseudo_legal_moves(pos), table_move, killers);
        // Late moves may be reduced, but not to get out of a check.
        let may_reduce = self.reductions && depth >= LMR_MIN_DEPTH && !pos.in_check();
        let mut searched = 0;
        for &mv in &moves {
            self.keys.push(pos.hash());
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                self.keys.pop();
                continue;
            }
            let first = !any_legal;
            any_legal = true;
            let reduce =
                may_reduce && searched >= LMR_FULL_MOVES && is_quiet(pos, mv) && !next.in_check();
            let score = if reduce {
                self.search_reduced(&next, depth, ply, alpha, beta, searched)
            } else {
                -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha)
            };
            searched += 1;
            self.keys.pop();
            if self.stopped {
                return DRAW;
            }

            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    best_move = Some(mv);
                    self.update_pv(ply, mv);
                    if alpha >= beta {
                        // The opponent has a better option elsewhere and
                        // will never let the game reach this position.
                        self.stats.cutoffs += 1;
                        self.stats.first_move_cutoffs += u64::from(first);
                        if is_quiet(pos, mv) {
                            self.killers[ply].remember(mv);
                        }
                        break;
                    }
                }
            }
        }

        if !any_legal {
            return no_moves_score(pos, ply);
        }
        // Remember the result. It is the value if some move raised alpha
        // without reaching beta; a move that reached beta shows only that
        // the value is at least that; and if no move raised alpha, only
        // that it is at most the best seen.
        if let Some(table) = self.table.as_deref_mut() {
            let bound = if best >= beta {
                Bound::Lower
            } else if best > alpha_at_first {
                Bound::Exact
            } else {
                Bound::Upper
            };
            table.store(key, ply, depth, best, bound, best_move);
        }
        best
    }
}

impl Search<'_> {
    /// Late move reductions: the value, for the side that moved, of the
    /// position `next` after a late quiet move, searched less deep than
    /// the `depth - 1` plies an early move gets (see
    /// [`late_move_reduction`]).
    ///
    /// The moves are tried best first as far as the search can tell: the
    /// table's move, the captures that win, the killers. A quiet move that
    /// comes after `searched` others is unlikely to be the best, so it is
    /// first only asked, at reduced depth and with a window of one point,
    /// whether it beats alpha. Usually it does not and the rest of its
    /// subtree is saved. If it does, the doubt was wrong and the move is
    /// searched again at full depth with the whole window, so a good late
    /// move costs time but is not missed for being late.
    ///
    /// The caller does not reduce captures and promotions, moves that
    /// give check, moves out of check, the first moves of a position, or
    /// anything at low depth.
    fn search_reduced(
        &mut self,
        next: &Position,
        depth: u32,
        ply: usize,
        alpha: Score,
        beta: Score,
        searched: u32,
    ) -> Score {
        let reduction = late_move_reduction(depth, searched);
        if reduction > 0 {
            self.reduced.0 += 1;
            let shallow = depth - 1 - reduction;
            let score = -self.negamax(next, shallow, ply + 1, -alpha - 1, -alpha);
            if self.stopped || score <= alpha {
                return score;
            }
            self.reduced.1 += 1;
        }
        -self.negamax(next, depth - 1, ply + 1, -beta, -alpha)
    }

    /// Reverse futility pruning: near the horizon, a position whose static
    /// evaluation is far above beta is cut off without being searched.
    /// Returns the score to cut off with, or `None`.
    ///
    /// "Far" is [`RFP_MARGIN`] for every ply of depth left: the side to
    /// move is so well off that the few moves left to search are not
    /// expected to bring it below beta, whatever they are. It is the
    /// mirror image of futility pruning, which gives up on moves that
    /// cannot lift a bad position to alpha.
    ///
    /// - **Only up to [`RFP_MAX_DEPTH`]:** with more moves to come, more
    ///   than the margin can change.
    /// - **Not in check:** the evaluation of a position in check says
    ///   little, and there may be a mate.
    /// - **Not when beta is a mate score:** an evaluation proves no mate.
    ///
    /// The score returned is the evaluation less the margin: still at
    /// least beta, and not more than the pruning itself vouches for.
    fn reverse_futility_cutoff(&self, pos: &Position, depth: u32, beta: Score) -> Option<Score> {
        if !self.futility || depth > RFP_MAX_DEPTH || beta.abs() >= MATE_BOUND || pos.in_check() {
            return None;
        }
        let margin = RFP_MARGIN * depth as Score;
        let cautious = self.evaluator.evaluate(pos) - margin;
        (cautious >= beta).then_some(cautious)
    }

    /// Whether a null move may be tried in `pos`: see
    /// [`Search::null_move_cutoff`] for what each condition is for.
    fn null_move_applies(&self, pos: &Position, depth: u32, ply: usize, beta: Score) -> bool {
        self.null_moves
            && depth >= NULL_MIN_DEPTH
            && !self.no_null[ply]
            && beta.abs() < MATE_BOUND
            && !pos.in_check()
            && pieces_of_mover(pos) > 0
            && self.evaluator.evaluate(pos) >= beta
    }

    /// Null-move pruning: if the side to move could pass and still be at
    /// least as well off as `beta`, a real move will nearly always do at
    /// least as well, so the position is not searched. Returns the score
    /// to cut off with, or `None` to search the position as usual (also
    /// after a stop: the caller checks `stopped`).
    ///
    /// "Could pass" is found out by giving the opponent the move and
    /// searching [`null_reduction`] plies shallower than usual, with a
    /// window of one point at beta, since only "at least beta or not" is
    /// asked. It is the largest saving in the search and the one place
    /// where it takes something on trust, so it is hedged:
    ///
    /// - **Not in check:** passing would leave the king to be taken.
    /// - **Not when the evaluation is below beta:** a side that stands
    ///   worse than beta at rest will seldom prove beta by passing.
    /// - **Not twice running, and not at low depth or with a mate score
    ///   as beta:** two passes search the same position again shallower,
    ///   and a mate is not proved by a reduced search.
    /// - **Not with pawns and king alone.** The reasoning fails exactly in
    ///   zugzwang, where every move makes things worse and passing would
    ///   be best, and pawn endgames are full of it.
    /// - **With one piece left, only after verification.** Zugzwang is
    ///   still common there, so the fail-high is checked by searching the
    ///   position itself at the reduced depth, at least one ply, with
    ///   real moves only: no null move there, and no reverse futility
    ///   pruning, which would answer from the same evaluation that
    ///   allowed the null move. (Below that position the search is the
    ///   ordinary one.)
    fn null_move_cutoff(
        &mut self,
        pos: &Position,
        depth: u32,
        ply: usize,
        beta: Score,
    ) -> Option<Score> {
        if !self.null_move_applies(pos, depth, ply, beta) {
            return None;
        }
        let reduced = depth.saturating_sub(1 + null_reduction(depth));
        let mut next = pos.clone();
        next.make_null_move();
        self.nulls.tried += 1;
        self.keys.push(pos.hash());
        self.no_null[ply + 1] = true;
        let score = -self.negamax(&next, reduced, ply + 1, -beta, -beta + 1);
        self.no_null[ply + 1] = false;
        self.keys.pop();
        if self.stopped || score < beta {
            return None;
        }
        self.nulls.failed_high += 1;
        if pieces_of_mover(pos) == 1 {
            self.nulls.verified += 1;
            (self.no_null[ply], self.verifying[ply]) = (true, true);
            // At least a ply: at depth 0 it would be a quiescence search,
            // which stands pat on the evaluation and tries no quiet move.
            let verified = self.negamax(pos, reduced.max(1), ply, beta - 1, beta);
            (self.no_null[ply], self.verifying[ply]) = (false, false);
            if self.stopped || verified < beta {
                return None;
            }
        }
        self.nulls.cutoffs += 1;
        // A mate seen behind a null move is not a mate: the opponent did
        // not really pass.
        Some(if score >= MATE_BOUND { beta } else { score })
    }

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
    /// - When it is not in check and has no legal move at all, that is
    ///   stalemate and a draw, whatever the material says.
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
        self.seldepth = self.seldepth.max(ply);
        self.nodes += 1;
        self.stats.quiescence_nodes += 1;
        // A capture can leave too little material to mate with. Repetition
        // and the fifty-move rule are not looked for here: captures cannot
        // lead to either, and the quiet moves that get out of a check are
        // left to the main search, which tests for both one ply deeper.
        if ply > 0 && insufficient_material(pos) {
            return DRAW;
        }

        let in_check = pos.in_check();
        // Stalemate has to be seen before standing pat: the side that
        // cannot move is usually far behind, and the evaluation would call
        // the position won for the other side. Generating moves at every
        // node just for this would be costly, so a glance at the pawns and
        // knights rules it out first, which it nearly always does.
        let mut generated = None;
        if !in_check && !has_free_piece(pos) {
            let moves = pseudo_legal_moves(pos);
            if !has_legal_move(pos, &moves) {
                return DRAW;
            }
            generated = Some(moves);
        }
        let mut best = -INFINITY;
        if !in_check || ply >= MAX_PLY {
            let stand_pat = self.evaluator.evaluate(pos);
            if stand_pat >= beta || ply >= MAX_PLY {
                self.stats.stand_pat_cutoffs += u64::from(stand_pat >= beta);
                return stand_pat;
            }
            best = stand_pat;
            alpha = alpha.max(stand_pat);
        }

        let mut any_legal = false;
        let moves = generated.unwrap_or_else(|| pseudo_legal_moves(pos));
        // Most valuable victim first: see `ordering.rs` for why this is not
        // optional here.
        for &mv in &captures_first(pos, &moves) {
            // Out of check, only captures and queen promotions, and of
            // those only the ones that do not lose material on their
            // square: a capture that loses is not a way to show that the
            // position is better than it stands. In check every move is
            // tried, since one of them has to be played.
            if !in_check && (!is_tactical(mv) || loses_material(pos, mv)) {
                continue;
            }
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                continue;
            }
            let first = !any_legal;
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
                        self.stats.quiescence_cutoffs += 1;
                        self.stats.quiescence_first_move_cutoffs += u64::from(first);
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

/// Whether the side to move, which must **not be in check**, has a pawn or
/// a knight that is certain to have a legal move. `false` means "not
/// certain", not "no legal move".
///
/// When the king is not in check, a move can only be illegal by exposing
/// it: a king move onto an attacked square, or a piece leaving a line it
/// shared with its king. A piece that stands on no rank, file or diagonal
/// with its own king therefore moves freely. So a pawn off the king's lines
/// with an empty square ahead, or a knight off them with a square to go to,
/// proves the position is not stalemate, from the bitboards alone and
/// without generating a move.
fn has_free_piece(pos: &Position) -> bool {
    let side = pos.side_to_move();
    let Some(king) = pos.king_square(side) else {
        return false;
    };
    let (ahead, back) = match side {
        Color::White => (pos.pieces_of(side, PieceKind::Pawn).north(), -1),
        Color::Black => (pos.pieces_of(side, PieceKind::Pawn).south(), 1),
    };
    let mut pushes = ahead & !pos.occupied();
    while let Some(to) = pushes.pop_lsb() {
        if to
            .offset(0, back)
            .is_some_and(|from| !on_a_line(from, king))
        {
            return true;
        }
    }
    let mut knights = pos.pieces_of(side, PieceKind::Knight);
    while let Some(from) = knights.pop_lsb() {
        let targets = knight_attacks(from) & !pos.occupied_by(side);
        if !on_a_line(from, king) && !targets.is_empty() {
            return true;
        }
    }
    false
}

/// Whether at least one of the pseudo-legal `moves` of `pos` is legal, for
/// a side to move that is **not in check**.
///
/// Playing a move to test it is costly, so the same reasoning as in
/// [`has_free_piece`] comes first: any move by a piece off the king's lines
/// is legal (en passant aside, which takes a second pawn off a rank). Only
/// when there is no such move are moves played out.
fn has_legal_move(pos: &Position, moves: &MoveList) -> bool {
    let Some(king) = pos.king_square(pos.side_to_move()) else {
        return !moves.is_empty();
    };
    let cannot_be_pinned = |mv: &Move| {
        mv.kind() != MoveKind::EnPassant && mv.from() != king && !on_a_line(mv.from(), king)
    };
    moves.iter().any(cannot_be_pinned)
        || moves.iter().any(|&mv| {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            !next.can_capture_king()
        })
}

/// Whether two squares share a rank, a file or a diagonal.
const fn on_a_line(a: Square, b: Square) -> bool {
    let files = a.file().abs_diff(b.file());
    let ranks = a.rank().abs_diff(b.rank());
    files == 0 || ranks == 0 || files == ranks
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

    /// Whether the side to move in `pos` is stalemated.
    fn is_stalemate(pos: &Position) -> bool {
        legal_moves(pos).is_empty() && !pos.in_check()
    }

    /// A queen up with three tempting moves, Qf7, Qg6 and Kg6, that leave
    /// the bare king without a legal move and not in check. A search that
    /// scored "no legal moves" as a win would play one of them and report a
    /// mate; this one must count them as draws and keep the win instead.
    #[test]
    fn stalemate_is_scored_as_a_draw_inside_the_tree() {
        let text = "7k/8/4QK2/8/8/8/8/8 w - - 0 1";
        let stalemating = ["e6f7", "e6g6", "f6g6"];
        for text_move in stalemating {
            let mut pos = fen(text);
            let mv = pos.parse_move(text_move).unwrap();
            let _undo = pos.make_move(mv);
            assert!(is_stalemate(&pos), "{text_move}");
        }
        // Depth 1 as well: there the stalemates sit at the horizon, where
        // quiescence search has to recognise them.
        for depth in 1..=3 {
            let outcome = search(text, depth);
            let chosen = outcome.best_move.unwrap();
            assert!(
                !stalemating.contains(&chosen.to_string().as_str()),
                "depth {depth}: {chosen}"
            );
            let mut pos = fen(text);
            let _undo = pos.make_move(chosen);
            assert!(!is_stalemate(&pos), "depth {depth}: {chosen}");
            assert!(outcome.score > 500, "depth {depth}: {outcome:?}");
            // Mate is three plies away, so only depth 3 may report one.
            assert_eq!(
                crate::search::is_mate_score(outcome.score),
                depth == 3,
                "depth {depth}: {outcome:?}"
            );
        }
    }

    /// At the horizon a stalemate is a draw, not the static evaluation,
    /// which would call a position a queen down lost.
    #[test]
    fn quiescence_scores_stalemate_as_a_draw() {
        let stalemate = fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
        assert!(is_stalemate(&stalemate));
        assert!(HandCrafted.evaluate(&stalemate) < -800);
        assert_eq!(quiet_value(&stalemate, 0), DRAW);
        assert_eq!(quiet_value(&stalemate, 3), DRAW);
        // Even when the window says the evaluation would be good enough.
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.quiesce(&stalemate, 1, -2_000, -1_000), DRAW);

        // A one-ply search must not walk into one: after Kb2 the black king
        // on d1 has no move.
        let text = "8/8/8/8/8/8/5Q2/K2k4 w - - 0 1";
        let mut pos = fen(text);
        let trap = pos.parse_move("a1b2").unwrap();
        let _undo = pos.make_move(trap);
        assert!(is_stalemate(&pos));
        assert_ne!(best(text, 1), "a1b2");
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

    /// A perpetual check, with no game history at all: a queen and a rook
    /// down, White checks on e8 and h5 for ever, and Black's king has one
    /// square each time. The fourth ply returns to the root position, which
    /// the search can only know from the hashes it recorded on its own way
    /// down.
    #[test]
    fn a_repetition_inside_the_search_line_is_a_draw() {
        let text = "6k1/6p1/8/7Q/8/7K/1r6/q7 w - - 0 1";
        let root = fen(text);
        let mut pos = root.clone();
        for (check, only_reply) in [("h5e8", "g8h7"), ("e8h5", "h7g8")] {
            let mv = pos.parse_move(check).unwrap();
            let _undo = pos.make_move(mv);
            let replies = legal_moves(&pos);
            assert_eq!(replies.len(), 1, "after {check}");
            assert_eq!(replies[0].to_string(), only_reply);
            let _undo = pos.make_move(replies[0]);
        }
        assert_eq!(pos.hash(), root.hash(), "four plies return to the root");

        // Three plies do not reach the repetition: White is simply lost.
        assert!(search(text, 3).score < -300);
        // Four do, and the draw is the best White can get.
        let mut searcher = Search::new(&HandCrafted).with_history(&[1, 2, 3]);
        let outcome = searcher.fixed_depth(&root, 4);
        assert_eq!(outcome.score, DRAW);
        let line: Vec<String> = outcome.pv.iter().map(ToString::to_string).collect();
        assert_eq!(line, ["h5e8", "g8h7", "e8h5", "h7g8"]);
        // The search took back every hash it recorded, including those of
        // the many illegal moves it tried while in check.
        assert_eq!(searcher.keys, [1, 2, 3]);

        // The same perpetual entered from outside: the queen starts on e2,
        // so the root position never comes back. The position that repeats
        // is the one after Qe8+, one ply down, which only the hashes
        // recorded inside the tree can show. It comes back at ply 5.
        let text = "6k1/6p1/8/8/8/7K/1r2Q3/q7 w - - 0 1";
        assert!(search(text, 4).score < -300);
        let outcome = search(text, 5);
        assert_eq!(outcome.score, DRAW);
        let line: Vec<String> = outcome.pv.iter().map(ToString::to_string).collect();
        assert_eq!(line, ["e2e8", "g8h7", "e8h5", "h7g8", "h5e8"]);
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
        // King and knight against king: every line is drawn at once, so the
        // search visits the root and one node per root move, however deep it
        // was asked to go.
        let dead = fen("4k3/8/8/8/8/8/8/3NK3 w - - 0 1");
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.fixed_depth(&dead, 3).score, DRAW);
        assert_eq!(searcher.nodes(), 1 + legal_moves(&dead).len() as u64);
    }

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    /// Perft position 3: few pieces, so deeper searches stay quick.
    const ROOK_ENDGAME: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";

    #[test]
    fn a_node_limit_stops_the_search_and_restores_the_key_history() {
        let pos = fen(KIWIPETE);
        let mut searcher = Search::new(&HandCrafted)
            .with_history(&[1, 2, 3])
            .with_limits(Limits::nodes(500));
        let outcome = searcher.fixed_depth(&pos, 4);
        assert!(searcher.was_stopped());
        assert_eq!(searcher.nodes(), 500, "stops exactly at the limit");
        // Unwinding took back every hash recorded on the way down. (The
        // position itself cannot change: the search only borrows it.)
        assert_eq!(searcher.keys, [1, 2, 3]);
        // Whatever move it names is legal; the next test checks that it was
        // searched completely.
        if let Some(mv) = outcome.best_move {
            assert!(legal_moves(&pos).contains(&mv));
        }
    }

    /// A search cut short answers from the root moves it finished and from
    /// nothing else. White is a queen down here, so every move scores far
    /// below zero; the half-searched move comes back as zero and would win
    /// the comparison if it were allowed to take part.
    #[test]
    fn an_interrupted_search_ignores_the_move_it_was_working_on() {
        let pos = fen("4k3/8/8/3q4/8/8/P7/4K3 w - - 0 1");
        let depth = 3;
        // The exact value of each root move, in the order they are tried.
        let mut exact = Vec::new();
        for &mv in &legal_moves(&pos) {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            let mut child = Search::new(&HandCrafted);
            child.keys.push(pos.hash());
            let score = -child.negamax(&next, depth - 1, 1, -INFINITY, INFINITY);
            assert!(score < -300, "{mv}: {score}");
            exact.push((score, mv));
        }
        // What the search may answer: nothing, or the best of the first one,
        // two, three... root moves, the earliest of equals.
        let mut allowed = vec![(DRAW, None)];
        for done in 1..=exact.len() {
            let best = exact[..done].iter().map(|&(score, _)| score).max().unwrap();
            let first = exact[..done].iter().find(|&&(score, _)| score == best);
            allowed.push((best, first.map(|&(_, mv)| mv)));
        }

        let mut full = Search::new(&HandCrafted);
        let complete = full.fixed_depth(&pos, depth);
        assert_eq!(allowed.last(), Some(&(complete.score, complete.best_move)));
        let total = full.nodes();

        let mut named_a_move = 0;
        for quarter in 1..=3 {
            let limit = total * quarter / 4;
            let mut searcher = Search::new(&HandCrafted).with_limits(Limits::nodes(limit));
            let outcome = searcher.fixed_depth(&pos, depth);
            assert!(searcher.was_stopped(), "limit {limit}");
            assert!(
                allowed.contains(&(outcome.score, outcome.best_move)),
                "limit {limit}: {outcome:?}"
            );
            named_a_move += u32::from(outcome.best_move.is_some());
        }
        assert!(named_a_move >= 2, "the limits must fall between root moves");
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
        // A mate score alone does not end the search. Here depth 1 already
        // reports mate in two (Re8+ Qxe8, Qxe8#), seen through the checks
        // and captures of quiescence search, which does not look at every
        // move and so cannot prove it the shortest. The search goes on until
        // the mate fits inside the depth, at depth 3.
        let (outcome, reports, _) =
            iterate("3q2k1/5ppp/8/8/Q7/8/6K1/4R3 w - - 0 1", Limits::depth(12));
        assert_eq!(reports[0].score, mate_in(3), "seen at depth 1");
        assert_eq!(reports.len(), 3, "but only proven at depth 3");
        assert_eq!(outcome.score, mate_in(3));
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

    /// A first iteration of ordinary length completes even with a one-node
    /// limit, a raised stop flag or a passed deadline. Kiwipete's is on the
    /// long side of ordinary: longer than the interval between looks at
    /// the clock in the rest of the search.
    #[test]
    fn an_ordinary_first_iteration_always_completes() {
        let whole = search(KIWIPETE, 1);
        let mut direct = Search::new(&HandCrafted);
        direct.fixed_depth(&fen(KIWIPETE), 1);
        assert!(direct.nodes() > CHECK_INTERVAL);
        assert!(direct.nodes() < FIRST_DEPTH_CHECK_INTERVAL);

        let (outcome, reports, stopped) = iterate(KIWIPETE, Limits::nodes(1));
        assert_eq!(reports.len(), 1);
        assert_eq!(outcome, whole);
        // The limit is noticed between iterations, so no second one is
        // started only to be thrown away: nothing was interrupted.
        assert!(!stopped);

        let flag = AtomicBool::new(true);
        let mut searcher = Search::new(&HandCrafted).with_stop_flag(&flag);
        let mut count = 0;
        let outcome = searcher.iterate(&fen(KIWIPETE), &mut |_| count += 1);
        assert_eq!((count, &outcome), (1, &whole));
        assert!(!searcher.was_stopped());

        let (outcome, reports, stopped) = iterate(KIWIPETE, Limits::until(Instant::now()));
        assert_eq!((reports.len(), &outcome), (1, &whole));
        assert!(!stopped);
    }

    /// Eight queens a side: the first depth takes some 600,000 positions,
    /// nearly all of them in quiescence search. (With nine a side it takes
    /// millions, and seconds: that was known issue KI-10.)
    const MANY_QUEENS: &str = "qqqqkqqq/q7/8/8/8/8/Q7/QQQQKQQQ w - - 0 1";

    /// Where the first iteration is long, the stop flag and the clock end
    /// it once the first root move has been searched: the answer is then
    /// that move with its exact value, or the best of the first few, and
    /// no report is made.
    #[test]
    fn a_long_first_iteration_can_be_stopped_after_its_first_move() {
        let pos = fen(MANY_QUEENS);
        // The first root move and what searching it alone costs and finds.
        let first = table_move_first(&pos, &pseudo_legal_moves(&pos), None, Killers::NONE)
            .iter()
            .copied()
            .find(|&mv| legal_moves(&pos).contains(&mv))
            .unwrap();
        let mut next = pos.clone();
        let _undo = next.make_move(first);
        let mut child = Search::new(&HandCrafted);
        child.keys.push(pos.hash());
        let first_score = -child.negamax(&next, 0, 1, -INFINITY, INFINITY);
        let first_nodes = child.nodes() + 1;
        let mut direct = Search::new(&HandCrafted);
        direct.fixed_depth(&pos, 1);
        assert!(direct.nodes() > first_nodes + 2 * FIRST_DEPTH_CHECK_INTERVAL);

        let flag = AtomicBool::new(true);
        let mut by_flag = Search::new(&HandCrafted).with_stop_flag(&flag);
        let late = Limits::until(Instant::now());
        let mut by_clock = Search::new(&HandCrafted).with_limits(late);
        for searcher in [&mut by_flag, &mut by_clock] {
            let mut count = 0;
            let outcome = searcher.iterate(&pos, &mut |_| count += 1);
            assert!(searcher.was_stopped());
            assert_eq!(count, 0, "an interrupted iteration is not reported");
            // It went on to the end of the first move and at most to the
            // next look at the flag and the clock.
            assert!(searcher.nodes() >= first_nodes);
            assert!(searcher.nodes() <= first_nodes + FIRST_DEPTH_CHECK_INTERVAL);
            assert!(legal_moves(&pos).contains(&outcome.best_move.unwrap()));
            assert!(outcome.score >= first_score);
            if outcome.best_move == Some(first) {
                assert_eq!(outcome.score, first_score);
            }
            assert_eq!(outcome.pv.first(), outcome.best_move.as_ref());
            assert_eq!(searcher.keys, [], "the path was unwound");
        }
        // And the search can be used again: stopped is not a lasting state.
        flag.store(false, Ordering::Relaxed);
        by_flag.limits = Limits::depth(1);
        let mut count = 0;
        let whole = by_flag.iterate(&fen(ROOK_ENDGAME), &mut |_| count += 1);
        assert_eq!((count, whole), (1, search(ROOK_ENDGAME, 1)));
        assert!(!by_flag.was_stopped());
    }

    /// A node limit asks for a reproducible search: it does not end the
    /// first iteration even where that is long.
    #[test]
    fn a_node_limit_never_ends_the_first_iteration() {
        let mut direct = Search::new(&HandCrafted);
        let whole = direct.fixed_depth(&fen(MANY_QUEENS), 1);
        assert!(direct.nodes() > 2 * FIRST_DEPTH_CHECK_INTERVAL);
        let (outcome, reports, stopped) = iterate(MANY_QUEENS, Limits::nodes(1));
        assert_eq!((reports.len(), outcome), (1, whole));
        assert!(!stopped);
    }

    /// A `Search` is not used up by a search that was cut short: the next
    /// call starts afresh, and its first iteration completes as promised.
    #[test]
    fn a_stopped_search_can_be_used_again() {
        let pos = fen(ROOK_ENDGAME);
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::nodes(500));
        let first = searcher.iterate(&pos, &mut |_| {});
        assert!(searcher.was_stopped());
        let first_nodes = searcher.nodes();

        let second = searcher.iterate(&pos, &mut |_| {});
        assert_eq!(second, first, "the same search gives the same answer");
        assert_eq!(searcher.nodes(), first_nodes, "the count starts afresh");

        // The same for fixed-depth searches: a stopped one, then a full one.
        let mut searcher = Search::new(&HandCrafted).with_limits(Limits::nodes(500));
        searcher.fixed_depth(&pos, 6);
        assert!(searcher.was_stopped());
        searcher.limits = Limits::NONE;
        assert_eq!(searcher.fixed_depth(&pos, 2), search(ROOK_ENDGAME, 2));
        assert!(!searcher.was_stopped());
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
        }
        // Valued on its own (ply 0), a dead position is not called a draw,
        // just as the root of a search is not; inside a line it is.
        let dead = fen("4k3/8/8/8/8/8/8/3NK3 w - - 0 1");
        assert_ne!(HandCrafted.evaluate(&dead), DRAW);
        assert_eq!(quiet_value(&dead, 0), HandCrafted.evaluate(&dead));
        assert_eq!(quiet_value(&dead, 1), DRAW);
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

    /// Out of check, a capture that loses material is not tried: the rook
    /// could take a pawn that a pawn defends, and the position is simply
    /// judged as it stands, in one node. In check it is tried like every
    /// other move: here rook takes knight is the only way out, and leaving
    /// it out would call the position mate.
    #[test]
    fn quiescence_leaves_out_captures_that_lose_material() {
        let pos = fen("4k3/8/4p3/3p4/8/8/8/3RK3 w - - 0 1");
        let mut searcher = Search::new(&HandCrafted);
        let value = searcher.quiesce(&pos, 0, -INFINITY, INFINITY);
        assert_eq!(value, HandCrafted.evaluate(&pos));
        assert_eq!(searcher.nodes(), 1);
        // The same pawn without its defender is taken.
        let free = fen("4k3/8/8/3p4/8/8/8/3RK3 w - - 0 1");
        let mut searcher = Search::new(&HandCrafted);
        let value = searcher.quiesce(&free, 0, -INFINITY, INFINITY);
        assert!(value > HandCrafted.evaluate(&free));
        assert!(searcher.nodes() > 1);
        let checked = fen("k4r2/8/8/8/8/8/5nPP/5RNK w - - 0 1");
        assert!(checked.in_check());
        assert_eq!(crate::movegen::legal_moves(&checked).len(), 1);
        let mut searcher = Search::new(&HandCrafted);
        let value = searcher.quiesce(&checked, 0, -INFINITY, INFINITY);
        assert!(!crate::search::is_mate_score(value), "{value}");
        assert!(searcher.nodes() > 1);
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
        // A skewer: White is a queen for a rook up, but the king is in
        // check, must step aside, and the queen behind it falls. Standing
        // pat here would claim the material; the truth is a draw at best
        // (the king retakes the rook).
        let skewered = fen("4r2k/8/8/8/8/8/4K3/4Q3 w - - 0 1");
        assert!(skewered.in_check());
        assert!(HandCrafted.evaluate(&skewered) > 300);
        assert_eq!(quiet_value(&skewered, 0), DRAW);
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
    }

    /// The shortcut in `has_legal_move` must agree with real legality on
    /// every position of a tree that is not in check, pins, en passant and
    /// stalemates included.
    #[test]
    fn has_legal_move_agrees_with_move_generation() {
        fn walk(pos: &mut Position, depth: u32, seen: &mut u64, free: &mut u64) {
            let legal = legal_moves(pos);
            if !pos.in_check() {
                *seen += 1;
                let fast = has_legal_move(pos, &pseudo_legal_moves(pos));
                assert_eq!(fast, !legal.is_empty(), "{}", pos.to_fen());
                // The glance at pawns and knights may only say "yes" when
                // there really is a legal move.
                if has_free_piece(pos) {
                    *free += 1;
                    assert!(!legal.is_empty(), "{}", pos.to_fen());
                }
            }
            if depth == 0 {
                return;
            }
            for &mv in &legal {
                let undo = pos.make_move(mv);
                walk(pos, depth - 1, seen, free);
                pos.unmake_move(mv, undo);
            }
        }
        let (mut seen, mut free) = (0, 0);
        for (text, depth) in [
            (KIWIPETE, 2),
            (ROOK_ENDGAME, 3),
            // En passant that would expose the king along the rank.
            ("8/8/8/KPp4r/8/8/8/7k w - c6 0 2", 2),
            // Queen and king against king, close to stalemate.
            ("7k/8/4QK2/8/8/8/8/8 w - - 0 1", 3),
            ("8/8/8/8/8/8/5Q2/K2k4 w - - 0 1", 3),
        ] {
            walk(&mut fen(text), depth, &mut seen, &mut free);
        }
        assert!(seen > 5_000, "{seen}");
        // The glance settles many positions, but not all of them.
        assert!(free > seen / 4 && free < seen, "{free} of {seen}");

        // The rook is pinned by the bishop on c3 and the king's two squares
        // are covered by the other bishop and the knight: stalemate,
        // although there are pseudo-legal moves.
        let pinned = fen("k7/8/4b3/8/8/2b5/1R1n4/K7 w - - 0 1");
        assert!(!pseudo_legal_moves(&pinned).is_empty());
        assert!(is_stalemate(&pinned));
        assert!(!has_legal_move(&pinned, &pseudo_legal_moves(&pinned)));
        assert!(!has_free_piece(&pinned));
        // A pinned pawn and a pinned knight are not counted as free; ones
        // off the king's lines are.
        assert!(!has_free_piece(&fen("4k3/4r3/8/8/8/8/4P3/4K3 w - - 0 1")));
        assert!(!has_free_piece(&fen("4k3/4r3/8/8/8/8/4N3/4K3 w - - 0 1")));
        assert!(has_free_piece(&fen("4k3/4r3/8/8/8/8/1P6/4K3 w - - 0 1")));
        assert!(has_free_piece(&fen("4k3/4r3/8/8/8/1N6/8/4K3 w - - 0 1")));
        // A blocked pawn is not free, for either colour.
        assert!(!has_free_piece(&fen("4k3/8/8/8/1p6/1P6/8/4K3 w - - 0 1")));
        assert!(!has_free_piece(&fen("4k3/8/8/8/1p6/1P6/8/4K3 b - - 0 1")));
        assert!(has_free_piece(&fen("4k3/8/8/8/1p6/8/1P6/4K3 b - - 0 1")));
        assert_eq!(quiet_value(&pinned, 1), DRAW);
    }

    #[test]
    fn squares_on_a_line() {
        use crate::board::Square;
        assert!(on_a_line(Square::A1, Square::A8));
        assert!(on_a_line(Square::A1, Square::H1));
        assert!(on_a_line(Square::A1, Square::H8));
        assert!(on_a_line(Square::H1, Square::A8));
        assert!(on_a_line(Square::E4, Square::E4));
        assert!(!on_a_line(Square::A1, Square::B3));
        assert!(!on_a_line(Square::E4, Square::F6));
        assert!(!on_a_line(Square::A1, Square::H7));
    }

    #[test]
    fn tactical_moves() {
        use crate::board::MoveKind;
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

    /// The selective depth is the deepest ply visited. In a quiet position
    /// every line ends at the horizon, so it equals the depth; captures
    /// played out in quiescence take it further.
    #[test]
    fn the_selective_depth_is_the_deepest_ply_visited() {
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.seldepth(), 0);
        // After one move from the start position nothing can be captured.
        searcher.fixed_depth(&Position::startpos(), 1);
        assert_eq!(searcher.seldepth(), 1);
        // After two there can be (1. e4 d5, and the pawns trade): the
        // quiescence search goes below the depth asked for.
        searcher.fixed_depth(&Position::startpos(), 2);
        assert!(searcher.seldepth() > 2);
        // Queen takes queen, king takes queen: two plies below a depth of
        // one, both in quiescence.
        let exchange = fen("3qk3/8/8/8/8/8/8/3QK3 w - - 0 1");
        searcher.fixed_depth(&exchange, 1);
        assert_eq!(searcher.seldepth(), 3);
        // Each search from the root starts again from nothing.
        searcher.fixed_depth(&Position::startpos(), 1);
        assert_eq!(searcher.seldepth(), 1);
        // No legal move: nothing below the root was visited.
        searcher.fixed_depth(&fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1"), 3);
        assert_eq!(searcher.seldepth(), 0);
    }

    /// Every report carries the selective depth of its own iteration. In
    /// this endgame it is never less than the depth; it can be where every
    /// line ends sooner in a draw by rule.
    #[test]
    fn each_report_has_the_selective_depth_of_its_iteration() {
        let (_, reports, _) = iterate(ROOK_ENDGAME, Limits::depth(4));
        assert_eq!(reports.len(), 4);
        for report in &reports {
            assert!(report.seldepth >= report.depth, "{report:?}");
            assert!((report.seldepth as usize) <= MAX_PLY);
        }
        let (_, mated, _) = iterate("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", Limits::depth(3));
        assert_eq!(mated[0].seldepth, 0);
    }

    /// Killer moves are quiet moves of the plies between the root and the
    /// horizon, and they last for one search.
    #[test]
    fn killers_are_quiet_moves_kept_for_one_search() {
        let mut searcher = Search::new(&HandCrafted);
        searcher.fixed_depth(&fen(crate::board::START_FEN), 5);
        // Nothing cuts off at the root, and below ply 4 there is only
        // quiescence search, which keeps none.
        assert_eq!(searcher.killers[0], Killers::NONE);
        assert!(searcher.killers[5..].iter().all(|at| *at == Killers::NONE));
        let kept: Vec<Move> = searcher.killers[1..5]
            .iter()
            .flat_map(|at| at.moves())
            .flatten()
            .collect();
        assert!(kept.len() >= 4, "{kept:?}");
        for mv in kept {
            assert!(!mv.is_capture() && mv.promotion().is_none(), "{mv}");
        }
        // They are found again by the same search, and the next search of
        // anything starts without them.
        let first = searcher.killers.clone();
        searcher.fixed_depth(&fen(crate::board::START_FEN), 5);
        assert_eq!(searcher.killers, first);
        searcher.fixed_depth(&fen(ROOK_ENDGAME), 1);
        assert!(searcher.killers.iter().all(|at| *at == Killers::NONE));
    }

    #[test]
    fn the_null_move_reduction_grows_with_depth() {
        assert_eq!(null_reduction(3), 2);
        assert_eq!(null_reduction(4), 3);
        assert_eq!(null_reduction(7), 3);
        assert_eq!(null_reduction(8), 4);
        assert_eq!(null_reduction(16), 6);
    }

    #[test]
    fn the_pieces_of_the_mover_are_counted_without_pawns_and_king() {
        let count = |text: &str| pieces_of_mover(&fen(text));
        assert_eq!(count(crate::board::START_FEN), 7);
        assert_eq!(count("4k3/pppp4/8/8/8/8/PPPP4/4K3 w - - 0 1"), 0);
        assert_eq!(count("4k3/pppp4/8/8/8/8/PPPP4/R3K3 w - - 0 1"), 1);
        assert_eq!(count("4k3/pppp4/8/8/8/8/PPPP4/R3K3 b - - 0 1"), 0);
        assert_eq!(count("qnb1k3/8/8/8/8/8/8/R3K3 b - - 0 1"), 3);
    }

    /// Each guard of the null move, one at a time, on a position where
    /// White is a queen up and a null move is in order.
    #[test]
    fn a_null_move_is_tried_only_where_it_is_safe() {
        let ahead = fen("r3k3/8/8/8/8/8/8/R2QK3 w - - 0 1");
        let mut searcher = Search::new(&HandCrafted);
        let stands = HandCrafted.evaluate(&ahead);
        assert!(stands > 500);
        assert!(searcher.null_move_applies(&ahead, 3, 1, 0));
        assert!(searcher.null_move_applies(&ahead, 9, 4, stands));
        // Too shallow; the evaluation below beta; a mate score as beta.
        assert!(!searcher.null_move_applies(&ahead, 2, 1, 0));
        assert!(!searcher.null_move_applies(&ahead, 3, 1, stands + 1));
        assert!(!searcher.null_move_applies(&ahead, 3, 1, -MATE_BOUND));
        assert!(!searcher.null_move_applies(&ahead, 3, 1, -MATE));
        // Reached by a null move, or being verified.
        searcher.no_null[1] = true;
        assert!(!searcher.null_move_applies(&ahead, 3, 1, 0));
        assert!(searcher.null_move_applies(&ahead, 3, 2, 0));
        searcher.no_null[1] = false;
        // In check, however far ahead.
        let checked = fen("4r1k1/8/8/8/8/8/8/R2QK3 w - - 0 1");
        assert!(checked.in_check());
        assert!(!searcher.null_move_applies(&checked, 3, 1, 0));
        // Pawns and king alone, however far ahead.
        let pawns = fen("4k3/8/8/8/8/8/PPPPPPPP/4K3 w - - 0 1");
        assert!(HandCrafted.evaluate(&pawns) > 500);
        assert!(!searcher.null_move_applies(&pawns, 3, 1, 0));
        // Switched off.
        searcher.null_moves = false;
        assert!(!searcher.null_move_applies(&ahead, 3, 1, 0));
    }

    /// The verification of a null move searches the position's own moves:
    /// neither the null move nor reverse futility pruning may answer it.
    /// White has a rook and stands far above beta, so that reverse
    /// futility pruning would cut the verification off at once if it were
    /// allowed to.
    #[test]
    fn a_null_move_is_verified_by_searching_real_moves() {
        let pos = fen("8/8/8/4k3/8/8/4K3/R7 w - - 0 1");
        let (depth, ply, beta) = (6, 1, 0);
        let reduced = depth - 1 - null_reduction(depth);
        assert!((1..=RFP_MAX_DEPTH).contains(&reduced));
        let stands = HandCrafted.evaluate(&pos);
        assert!(stands - RFP_MARGIN * reduced as Score >= beta, "{stands}");
        assert_eq!(pieces_of_mover(&pos), 1);
        // What the null move's own search costs.
        let mut alone = Search::new(&HandCrafted);
        let mut passed = pos.clone();
        passed.make_null_move();
        alone.keys.push(pos.hash());
        alone.no_null[ply + 1] = true;
        let score = -alone.negamax(&passed, reduced, ply + 1, -beta, -beta + 1);
        assert!(score >= beta);
        let null_nodes = alone.nodes();

        let mut searcher = Search::new(&HandCrafted);
        let cutoff = searcher.null_move_cutoff(&pos, depth, ply, beta);
        assert!(searcher.nulls.verified >= 1);
        // The verification visited the position and at least one position
        // after each of its legal moves, unless one of them cut it off:
        // far more than the single node of an answer by evaluation.
        let verification_nodes = searcher.nodes() - null_nodes;
        assert!(verification_nodes > 3, "{verification_nodes}");
        // And it agrees with a plain search of the same thing.
        let mut plain = Search::new(&HandCrafted);
        (plain.null_moves, plain.futility) = (false, false);
        let real = plain.negamax(&pos, reduced, ply, beta - 1, beta);
        assert_eq!(cutoff.is_some(), real >= beta);
        // The flags are down again afterwards.
        assert!(searcher.verifying.iter().all(|&set| !set));
        assert!(searcher.no_null.iter().all(|&set| !set));
    }

    /// A fixed-depth search with and without null-move pruning, both
    /// without late move reductions, so that null moves are measured by
    /// themselves.
    fn with_and_without_null_moves(text: &str, depth: u32) -> [(Outcome, u64, NullCounts); 2] {
        [true, false].map(|null_moves| {
            let mut searcher = Search::new(&HandCrafted);
            (searcher.reductions, searcher.futility) = (false, false);
            searcher.null_moves = null_moves;
            let outcome = searcher.fixed_depth(&fen(text), depth);
            (outcome, searcher.nodes(), searcher.nulls)
        })
    }

    /// Null-move pruning saves a large part of the work in ordinary
    /// positions. It is not exact, so the move and score may differ from
    /// the search without it; what it costs or gains in play is for its
    /// SPRT to say.
    #[test]
    fn null_moves_save_work() {
        for (text, depth) in [
            (crate::board::START_FEN, 6),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                5,
            ),
            (
                "r1bq1rk1/pp2ppbp/2np1np1/8/3NP3/2N1BP2/PPPQ2PP/R3KB1R w KQ - 0 9",
                5,
            ),
        ] {
            let [(with, fewer, nulls), (without, nodes, none)] =
                with_and_without_null_moves(text, depth);
            assert_eq!(none, NullCounts::default(), "{text}");
            assert!(nulls.tried > 0 && nulls.cutoffs > 0, "{text}: {nulls:?}");
            assert!(nulls.cutoffs <= nulls.failed_high && nulls.failed_high <= nulls.tried);
            // With every piece on the board nothing needs verifying.
            assert_eq!(nulls.verified, 0, "{text}");
            assert!(
                fewer * 4 < nodes * 3,
                "{text}: {fewer} with, {nodes} without"
            );
            assert!(with.best_move.is_some() && without.best_move.is_some());
        }
    }

    /// With pawns and kings alone no null move is tried, so the search is
    /// the same search, node for node; with a single piece, every null
    /// move that fails high is verified.
    #[test]
    fn null_moves_keep_out_of_the_endgames_where_zugzwang_lives() {
        for text in [
            "8/8/8/3k4/8/3K4/3P4/8 w - - 0 1",
            "8/5p2/5k2/8/5K2/8/4P3/8 b - - 0 1",
            "8/pp3k2/8/8/8/8/PP3K2/8 w - - 0 1",
        ] {
            let [(with, nodes, nulls), (without, plain, _)] = with_and_without_null_moves(text, 7);
            assert_eq!(nulls, NullCounts::default(), "{text}");
            assert_eq!((with, nodes), (without, plain), "{text}");
        }
        // A rook each: the side to move has one piece wherever it is not
        // the lone king's turn.
        let [(_, _, nulls), _] = with_and_without_null_moves(ROOK_ENDGAME, 7);
        assert!(nulls.failed_high > 0, "{nulls:?}");
        assert_eq!(nulls.verified, nulls.failed_high);
        assert!(nulls.cutoffs <= nulls.verified);
    }

    /// Mates are found at the same depth and distance with null moves: a
    /// mate is never taken from behind a null move.
    #[test]
    fn null_moves_do_not_change_what_mates_are_found() {
        for (text, depth) in [
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 5),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 4),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3),
            ("3q2k1/5ppp/8/8/Q7/8/6K1/4R3 w - - 0 1", 5),
        ] {
            let [(with, _, _), (without, _, _)] = with_and_without_null_moves(text, depth);
            assert!(crate::search::is_mate_score(without.score), "{text}");
            assert_eq!(with.score, without.score, "{text}");
            assert_eq!(with.best_move, without.best_move, "{text}");
        }
    }

    #[test]
    fn late_moves_are_reduced_more_the_later_and_the_deeper() {
        // The formula, at a few points worked out by hand.
        assert_eq!(late_move_reduction(3, 3), 1);
        assert_eq!(late_move_reduction(4, 10), 2);
        assert_eq!(late_move_reduction(6, 6), 2);
        assert_eq!(late_move_reduction(8, 4), 2);
        assert_eq!(late_move_reduction(10, 10), 3);
        assert_eq!(late_move_reduction(16, 40), 5);
        assert_eq!(late_move_reduction(32, 40), 6);
        // Never less the later the move or the deeper the search, always
        // something from the first late move on, and always a ply left.
        for depth in LMR_MIN_DEPTH..=MAX_PLY as u32 {
            for searched in LMR_FULL_MOVES..=250 {
                let reduction = late_move_reduction(depth, searched);
                assert!(reduction >= 1, "{depth} {searched}");
                assert!(depth - 1 - reduction >= 1, "{depth} {searched}");
                assert!(reduction >= late_move_reduction(depth, searched - 1));
                assert!(reduction >= late_move_reduction(depth - 1, searched).min(depth - 3));
            }
        }
        // Past the end of the table the last entry counts.
        assert_eq!(late_move_reduction(63, 63), late_move_reduction(200, 200));
        // Too shallow to take anything off.
        assert_eq!(late_move_reduction(2, 30), 0);
        assert_eq!(late_move_reduction(1, 30), 0);
        // The table is the logarithm it says it is.
        for (index, &entry) in LN_THOUSANDTHS.iter().enumerate() {
            let exact = (f64::from(index as u32 + 1).ln() * 1_000.0).round();
            assert!((f64::from(entry) - exact).abs() <= 1.0, "{index}");
        }
    }

    /// A fixed-depth search with and without late move reductions, both
    /// without null moves: the outcome, the nodes, and the moves reduced
    /// and searched again.
    fn with_and_without_reductions(text: &str, depth: u32) -> [(Outcome, u64, (u64, u64)); 2] {
        [true, false].map(|reductions| {
            let mut searcher = Search::new(&HandCrafted);
            (searcher.null_moves, searcher.futility) = (false, false);
            searcher.reductions = reductions;
            let outcome = searcher.fixed_depth(&fen(text), depth);
            (outcome, searcher.nodes(), searcher.reduced)
        })
    }

    /// Late move reductions save a large part of the work, and only some
    /// of the reduced moves have to be searched again.
    #[test]
    fn late_move_reductions_save_work() {
        for (text, depth) in [
            (crate::board::START_FEN, 6),
            (KIWIPETE, 5),
            (ROOK_ENDGAME, 7),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                5,
            ),
        ] {
            let [(with, fewer, (reduced, again)), (without, nodes, none)] =
                with_and_without_reductions(text, depth);
            assert_eq!(none, (0, 0), "{text}");
            assert!(reduced > 0, "{text}");
            assert!(again * 2 < reduced, "{text}: {again} of {reduced}");
            assert!(
                fewer * 4 < nodes * 3,
                "{text}: {fewer} with, {nodes} without"
            );
            assert!(with.best_move.is_some() && without.best_move.is_some());
        }
    }

    /// Mates are found with the same score and move: the moves that give
    /// check, and the moves out of check, are never reduced.
    #[test]
    fn late_move_reductions_do_not_change_what_mates_are_found() {
        for (text, depth) in [
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 5),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 4),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3),
            ("3q2k1/5ppp/8/8/Q7/8/6K1/4R3 w - - 0 1", 5),
        ] {
            let [(with, _, _), (without, _, _)] = with_and_without_reductions(text, depth);
            assert!(crate::search::is_mate_score(without.score), "{text}");
            assert_eq!(with.score, without.score, "{text}");
            assert_eq!(with.best_move, without.best_move, "{text}");
        }
    }

    /// Each condition of reverse futility pruning, on a position where
    /// White is a queen up.
    #[test]
    fn reverse_futility_pruning_needs_a_margin_for_every_ply() {
        let ahead = fen("r3k3/8/8/8/8/8/8/R2QK3 w - - 0 1");
        let mut searcher = Search::new(&HandCrafted);
        let stands = HandCrafted.evaluate(&ahead);
        let cut = |searcher: &Search, depth: u32, beta: Score| {
            searcher.reverse_futility_cutoff(&ahead, depth, beta)
        };
        // Exactly the margin above beta is enough; one point less is not.
        for depth in 1..=RFP_MAX_DEPTH {
            let margin = RFP_MARGIN * depth as Score;
            assert_eq!(
                cut(&searcher, depth, stands - margin),
                Some(stands - margin)
            );
            assert_eq!(cut(&searcher, depth, stands - margin + 1), None);
            // Further above beta, the score is still the cautious one.
            assert_eq!(cut(&searcher, depth, -400), Some(stands - margin));
        }
        // Too deep, however far ahead; and no mate is claimed.
        assert_eq!(cut(&searcher, RFP_MAX_DEPTH + 1, -5_000), None);
        assert_eq!(cut(&searcher, 1, -MATE_BOUND), None);
        assert_eq!(cut(&searcher, 1, -MATE), None);
        // In check.
        let checked = fen("4r1k1/8/8/8/8/8/8/R2QK3 w - - 0 1");
        assert!(checked.in_check() && HandCrafted.evaluate(&checked) > 300);
        assert_eq!(searcher.reverse_futility_cutoff(&checked, 1, -5_000), None);
        // Switched off.
        searcher.futility = false;
        assert_eq!(cut(&searcher, 1, -5_000), None);
    }

    /// A fixed-depth search with and without reverse futility pruning,
    /// both without null moves and reductions: the outcome, the nodes and
    /// the positions it cut off.
    fn with_and_without_futility(text: &str, depth: u32) -> [(Outcome, u64, u64); 2] {
        [true, false].map(|futility| {
            let mut searcher = Search::new(&HandCrafted);
            (searcher.null_moves, searcher.reductions) = (false, false);
            searcher.futility = futility;
            let outcome = searcher.fixed_depth(&fen(text), depth);
            (outcome, searcher.nodes(), searcher.futile)
        })
    }

    #[test]
    fn reverse_futility_pruning_saves_work() {
        for (text, depth) in [
            (crate::board::START_FEN, 5),
            (KIWIPETE, 4),
            (ROOK_ENDGAME, 6),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                4,
            ),
        ] {
            let [(with, fewer, cut), (without, nodes, none)] =
                with_and_without_futility(text, depth);
            assert_eq!(none, 0, "{text}");
            assert!(cut > 0, "{text}");
            assert!(fewer < nodes, "{text}: {fewer} with, {nodes} without");
            assert!(with.best_move.is_some() && without.best_move.is_some());
        }
    }

    /// Mates are found with the same score and move: nothing is pruned in
    /// check or against a mate score.
    #[test]
    fn reverse_futility_pruning_does_not_change_what_mates_are_found() {
        for (text, depth) in [
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 5),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 4),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3),
            ("3q2k1/5ppp/8/8/Q7/8/6K1/4R3 w - - 0 1", 5),
        ] {
            let [(with, _, _), (without, _, _)] = with_and_without_futility(text, depth);
            assert!(crate::search::is_mate_score(without.score), "{text}");
            assert_eq!(with.score, without.score, "{text}");
            assert_eq!(with.best_move, without.best_move, "{text}");
        }
    }

    /// With root moves named, the search answers with the best of them
    /// and its value, looks at no other, and leaves the root out of the
    /// table.
    #[test]
    fn a_search_can_be_restricted_to_some_root_moves() {
        // The queen can be taken; restricted to two quiet moves, the
        // search must make do with the better of those.
        let pos = fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1");
        let free = search("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 3);
        assert_eq!(free.best_move.unwrap().to_string(), "d1d5");
        let named: Vec<Move> = legal_moves(&pos)
            .iter()
            .copied()
            .filter(|mv| ["e1e2", "d1a1"].contains(&mv.to_string().as_str()))
            .collect();
        assert_eq!(named.len(), 2);
        // What each is worth by itself.
        let worth = |mv: Move| {
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            let mut child = Search::new(&HandCrafted);
            child.keys.push(pos.hash());
            -child.negamax(&next, 2, 1, -INFINITY, INFINITY)
        };
        let best = named.iter().copied().max_by_key(|&mv| worth(mv)).unwrap();

        let mut table = TranspositionTable::new(1).unwrap();
        let mut searcher = Search::new(&HandCrafted)
            .with_root_moves(&named)
            .with_table(&mut table);
        let outcome = searcher.fixed_depth(&pos, 3);
        assert_eq!(outcome.best_move, Some(best));
        assert_eq!(outcome.score, worth(best));
        assert!(outcome.score < free.score);
        assert_eq!(outcome.pv.first(), Some(&best));
        drop(searcher);
        assert_eq!(table.probe(pos.hash(), 0), None, "the root is not recorded");
        // One move named: that move, whatever it is worth.
        let only = [named[0]];
        let mut searcher = Search::new(&HandCrafted).with_root_moves(&only);
        assert_eq!(searcher.fixed_depth(&pos, 2).best_move, Some(named[0]));
        // Iterating, every depth answers within the list.
        let mut searcher = Search::new(&HandCrafted)
            .with_root_moves(&named)
            .with_limits(Limits::depth(4));
        let mut firsts = Vec::new();
        let outcome = searcher.iterate(&pos, &mut |report| firsts.push(report.pv[0]));
        assert_eq!(firsts.len(), 4);
        assert!(firsts.iter().all(|mv| named.contains(mv)));
        assert!(named.contains(&outcome.best_move.unwrap()));
        // No list is no restriction.
        let mut searcher = Search::new(&HandCrafted).with_root_moves(&[]);
        assert_eq!(searcher.fixed_depth(&pos, 3), free);
    }

    /// A mate limit ends the iteration as soon as a mate that short is
    /// seen, and only then.
    #[test]
    fn a_mate_limit_ends_the_search_when_the_mate_is_found() {
        // Depth 1 already sees a mate in two here, through quiescence
        // search; unlimited, the search goes on to depth 3 to prove it.
        let seen_early = "3q2k1/5ppp/8/8/Q7/8/6K1/4R3 w - - 0 1";
        let mate = |within| Limits {
            depth: Some(12),
            ..Limits::mate(within)
        };
        let (outcome, reports, stopped) = iterate(seen_early, mate(2));
        assert_eq!((reports.len(), outcome.score), (1, mate_in(3)));
        assert!(!stopped);
        // More moves allowed: the same.
        assert_eq!(iterate(seen_early, mate(5)).1.len(), 1);
        // A mate in one was asked for and there is none: the search goes
        // on as it would have, to where the mate in two is proved.
        let (outcome, reports, _) = iterate(seen_early, mate(1));
        assert_eq!((reports.len(), outcome.score), (3, mate_in(3)));
        // Being mated is not the mate that was asked for.
        let lost = "7k/1R6/8/8/8/8/R7/6K1 b - - 0 1";
        let (outcome, reports, _) = iterate(lost, mate(1));
        assert_eq!(outcome.score, mated_in(2));
        assert_eq!(
            reports.last().unwrap().depth,
            2,
            "ended as without the limit"
        );
        // No mate anywhere: the other limits end the search.
        let (outcome, reports, _) = iterate(
            ROOK_ENDGAME,
            Limits {
                depth: Some(3),
                ..Limits::mate(4)
            },
        );
        assert_eq!(reports.len(), 3);
        assert!(!crate::search::is_mate_score(outcome.score));
    }

    /// The statistics in cases small enough to check by hand.
    #[test]
    fn the_statistics_count_cutoffs_and_quiescence_nodes() {
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.stats(), SearchStats::ZERO);
        // Depth 1 from the start: the root, then one quiescence position
        // for each of the 20 moves. The root's window is open, so the main
        // search has no cutoff; no capture is possible, so quiescence has
        // none by a move.
        searcher.fixed_depth(&Position::startpos(), 1);
        let stats = searcher.stats();
        assert_eq!(searcher.nodes(), 21);
        assert_eq!(stats.quiescence_nodes, 20);
        assert_eq!((stats.cutoffs, stats.first_move_cutoffs), (0, 0));
        assert_eq!(stats.quiescence_cutoffs, 0);
        // A reply that is no better for White than the best so far is cut
        // off as it stands: all 19 others at most.
        assert!((1..=19).contains(&stats.stand_pat_cutoffs), "{stats:?}");
        // One quiescence position that is good enough as it stands.
        let mut alone = Search::new(&HandCrafted);
        alone.quiesce(&Position::startpos(), 0, -1_000, -500);
        let expected = SearchStats {
            quiescence_nodes: 1,
            stand_pat_cutoffs: 1,
            ..SearchStats::ZERO
        };
        assert_eq!(alone.stats(), expected);
        // Queen takes queen refutes at once: a cutoff by the first move
        // tried, since captures are tried most valuable victim first.
        let mut capture = Search::new(&HandCrafted);
        capture.quiesce(&fen("3qk3/8/8/8/8/8/8/3QK3 w - - 0 1"), 0, -100, 100);
        let stats = capture.stats();
        assert_eq!(
            (
                stats.quiescence_cutoffs,
                stats.quiescence_first_move_cutoffs
            ),
            (1, 1)
        );
        assert_eq!(stats.quiescence_first_move_rate(), Some(1000));
    }

    /// The table's counts: none without a table; with one, a probe for
    /// every position of the main search below the root, the hits among
    /// the probes and the cutoffs among the hits; and a table that already
    /// holds the search's work knows more of what it is asked.
    #[test]
    fn the_table_is_counted() {
        let pos = fen(KIWIPETE);
        let mut plain = Search::new(&HandCrafted);
        plain.fixed_depth(&pos, 3);
        let none = plain.stats();
        assert_eq!(
            (none.table_probes, none.table_hits, none.table_cutoffs),
            (0, 0, 0)
        );
        let mut table = TranspositionTable::new(1).unwrap();
        let mut searcher = Search::new(&HandCrafted).with_table(&mut table);
        searcher.fixed_depth(&pos, 3);
        let (first, nodes) = (searcher.stats(), searcher.nodes());
        // Every node is the root, a quiescence position or a probe.
        assert_eq!(first.table_probes, nodes - 1 - first.quiescence_nodes);
        assert!(first.table_probes > 0);
        assert!(first.table_cutoffs <= first.table_hits);
        assert!(first.table_hits <= first.table_probes);
        searcher.fixed_depth(&pos, 3);
        let again = searcher.stats();
        assert!(
            again.table_hits * first.table_probes > first.table_hits * again.table_probes,
            "{first:?} {again:?}"
        );
        assert!(again.table_cutoffs > 0);
    }

    /// What holds whatever the position: the first-move cutoffs are among
    /// the cutoffs, the quiescence nodes among the nodes, and each search
    /// starts counting from nothing.
    #[test]
    fn the_statistics_are_consistent_and_start_afresh() {
        let mut searcher = Search::new(&HandCrafted);
        for position in [KIWIPETE, ROOK_ENDGAME, crate::board::START_FEN] {
            let pos = fen(position);
            searcher.fixed_depth(&pos, 2);
            let (stats, nodes) = (searcher.stats(), searcher.nodes());
            assert!(stats.cutoffs > 0, "{position}");
            assert!(stats.first_move_cutoffs <= stats.cutoffs);
            assert!(stats.quiescence_first_move_cutoffs <= stats.quiescence_cutoffs);
            assert!(stats.quiescence_nodes < nodes);
            assert!(stats.stand_pat_cutoffs + stats.quiescence_cutoffs <= stats.quiescence_nodes);
            // The same search again counts the same again, not twice as much.
            searcher.fixed_depth(&pos, 2);
            assert_eq!((searcher.stats(), searcher.nodes()), (stats, nodes));
        }
    }

    /// A search with a table and one without, each iterating to `depth`.
    /// Both are exact searches, without null moves and reductions: only
    /// of an exact search can it be asked that a table changes no answer.
    fn with_and_without_a_table(text: &str, depth: u32) -> ((Outcome, u64), (Outcome, u64)) {
        let pos = fen(text);
        let mut table = TranspositionTable::new(1).unwrap();
        let mut helped = Search::new(&HandCrafted)
            .with_limits(Limits::depth(depth))
            .with_table(&mut table);
        (helped.null_moves, helped.reductions, helped.futility) = (false, false, false);
        let with = (helped.iterate(&pos, &mut |_| {}), helped.nodes());
        let mut plain = Search::new(&HandCrafted).with_limits(Limits::depth(depth));
        (plain.null_moves, plain.reductions, plain.futility) = (false, false, false);
        let without = (plain.iterate(&pos, &mut |_| {}), plain.nodes());
        (with, without)
    }

    /// The table saves work and changes no answer: the same score and the
    /// same first move as the search without it, in fewer positions.
    #[test]
    fn a_table_saves_work_and_changes_no_answer() {
        for (text, depth) in [
            (crate::board::START_FEN, 5),
            (KIWIPETE, 4),
            (ROOK_ENDGAME, 6),
            (
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                4,
            ),
            ("4k3/8/3p4/4p3/8/8/4Q3/4K3 w - - 0 1", 5),
        ] {
            let ((with, fewer), (without, nodes)) = with_and_without_a_table(text, depth);
            assert_eq!(with.score, without.score, "{text}");
            assert_eq!(with.best_move, without.best_move, "{text}");
            assert!(fewer < nodes, "{text}: {fewer} with, {nodes} without");
        }
    }

    /// Mates are found at the same distance with a table, where the scores
    /// it hands back were stored at other distances from the root.
    #[test]
    fn a_table_keeps_mate_distances_right() {
        for (text, depth) in [
            ("7k/8/8/8/8/8/R7/1R4K1 w - - 0 1", 5),
            ("7k/1R6/8/8/8/8/R7/6K1 b - - 0 1", 4),
            ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3),
            ("8/8/8/8/8/5k2/8/3q1K2 w - - 0 1", 6),
        ] {
            let ((with, _), (without, _)) = with_and_without_a_table(text, depth);
            assert_eq!(with.score, without.score, "{text}");
            assert_eq!(with.best_move, without.best_move, "{text}");
        }
    }

    /// The table outlives a search. The same position searched again finds
    /// its answer almost at once, and after `clear` the work is the first
    /// search's again, node for node.
    #[test]
    fn a_table_is_kept_between_searches_until_it_is_cleared() {
        let pos = fen(KIWIPETE);
        let mut table = TranspositionTable::new(1).unwrap();
        let search = |table: &mut TranspositionTable| {
            let mut searcher = Search::new(&HandCrafted)
                .with_limits(Limits::depth(3))
                .with_table(table);
            (searcher.iterate(&pos, &mut |_| {}), searcher.nodes())
        };
        let (first, nodes) = search(&mut table);
        let (again, fewer) = search(&mut table);
        assert_eq!(
            (again.score, again.best_move),
            (first.score, first.best_move)
        );
        assert!(fewer * 4 < nodes, "{fewer} against {nodes}");
        table.clear();
        assert_eq!(search(&mut table), (first, nodes));
    }

    /// Near the fifty-move rule the table is not believed: a value found
    /// with a low count would hide the draw that the count is about to
    /// force. Rook against bare king, 98 half-moves in: a mate in two that
    /// the hundredth half-move turns into a draw.
    #[test]
    fn a_table_does_not_hide_the_fifty_move_rule() {
        let mut table = TranspositionTable::new(1).unwrap();
        let won = |clock: u32| fen(&format!("7k/8/5K2/8/8/8/8/R7 w - - {clock} 60"));
        // Fill the table from the position with a count of 0.
        let mut filling = Search::new(&HandCrafted)
            .with_limits(Limits::depth(5))
            .with_table(&mut table);
        let fresh = filling.iterate(&won(0), &mut |_| {});
        assert!(crate::search::is_mate_score(fresh.score), "{}", fresh.score);
        // The count is not part of the hash: the same table, the same
        // squares, and a search that must still see the draw.
        let late = won(98);
        assert_eq!(late.hash(), won(0).hash());
        let mut helped = Search::new(&HandCrafted)
            .with_limits(Limits::depth(5))
            .with_table(&mut table);
        let with = helped.iterate(&late, &mut |_| {});
        let without = Search::new(&HandCrafted)
            .with_limits(Limits::depth(5))
            .iterate(&late, &mut |_| {});
        assert_eq!(with.score, without.score);
        assert!(!crate::search::is_mate_score(with.score), "{}", with.score);
    }

    #[test]
    #[should_panic(expected = "between 1 and MAX_PLY")]
    fn depth_zero_is_rejected() {
        Search::new(&HandCrafted).fixed_depth(&Position::startpos(), 0);
    }
}
