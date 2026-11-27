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
    Bound, DRAW, INFINITY, Limits, MATE, MAX_PLY, SearchStats, TranspositionTable,
    fifty_moves_reached, insufficient_material, is_repetition, mated_in,
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
}

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
            stop_flag: None,
            stopped: false,
            limits_apply: true,
            seldepth: 0,
            stats: SearchStats::ZERO,
            table: None,
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
            self.limits_apply = depth > 1;
            let outcome = self.search_root(pos, depth);
            if self.stopped {
                break;
            }
            report(&Report {
                depth,
                seldepth: self.seldepth(),
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

    /// Starts a new search: forgets the node count and the stopped state of
    /// the one before. The game history and the limits are kept.
    const fn begin(&mut self) {
        self.nodes = 0;
        self.stopped = false;
        self.seldepth = 0;
        self.stats = SearchStats::ZERO;
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
        // Captures and promotions first, most valuable victim first: see
        // `ordering.rs`.
        for &mv in &captures_first(pos, &pseudo_legal_moves(pos)) {
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
        if let Some(table) = self.table.as_deref()
            && let Some(known) = table.probe(key, ply)
            && known.depth >= depth
            && pos.halfmove_clock() < FIFTY_MOVE_GUARD
        {
            let settles = match known.bound {
                Bound::Exact => true,
                Bound::Lower => known.score >= beta,
                Bound::Upper => known.score <= alpha,
            };
            if settles {
                return known.score;
            }
        }

        let alpha_at_first = alpha;
        let mut best = -INFINITY;
        let mut best_move = None;
        let mut any_legal = false;
        // Captures and promotions first, most valuable victim first: see
        // `ordering.rs`.
        for &mv in &captures_first(pos, &pseudo_legal_moves(pos)) {
            self.keys.push(pos.hash());
            let mut next = pos.clone();
            let _undo = next.make_move(mv);
            if next.can_capture_king() {
                self.keys.pop();
                continue;
            }
            let first = !any_legal;
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
                    best_move = Some(mv);
                    self.update_pv(ply, mv);
                    if alpha >= beta {
                        // The opponent has a better option elsewhere and
                        // will never let the game reach this position.
                        self.stats.cutoffs += 1;
                        self.stats.first_move_cutoffs += u64::from(first);
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
            if !in_check && !is_tactical(mv) {
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

    #[test]
    fn the_first_iteration_always_completes() {
        // A node limit smaller than one iteration, and a raised stop flag.
        let (outcome, reports, stopped) = iterate(KIWIPETE, Limits::nodes(1));
        assert_eq!(reports.len(), 1);
        assert_eq!(outcome, search(KIWIPETE, 1));
        // The limit is noticed between iterations, so no second one is
        // started only to be thrown away: nothing was interrupted.
        assert!(!stopped);

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
    fn with_and_without_a_table(text: &str, depth: u32) -> ((Outcome, u64), (Outcome, u64)) {
        let pos = fen(text);
        let mut table = TranspositionTable::new(1).unwrap();
        let mut helped = Search::new(&HandCrafted)
            .with_limits(Limits::depth(depth))
            .with_table(&mut table);
        let with = (helped.iterate(&pos, &mut |_| {}), helped.nodes());
        let mut plain = Search::new(&HandCrafted).with_limits(Limits::depth(depth));
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
