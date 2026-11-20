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

use super::{
    DRAW, INFINITY, MAX_PLY, fifty_moves_reached, insufficient_material, is_repetition, mated_in,
};
use crate::board::{Move, Position};
use crate::eval::{Evaluator, Score};
use crate::movegen::legal_moves;
use crate::movegen::pseudo_legal_moves;

/// What a search found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// The value of the position for the side to move.
    pub score: Score,
    /// The move that achieves it, or `None` when the side to move has no
    /// legal move (checkmate or stalemate).
    pub best_move: Option<Move>,
}

/// A search in progress: the evaluator it uses and what it has counted.
pub struct Search<'a> {
    evaluator: &'a dyn Evaluator,
    nodes: u64,
    /// Hashes of the positions before the one being searched, oldest first:
    /// the game so far, then the path from the root. Used to see
    /// repetitions.
    keys: Vec<u64>,
}

impl<'a> Search<'a> {
    /// Creates a search that scores positions with `evaluator`.
    #[must_use]
    pub fn new(evaluator: &'a dyn Evaluator) -> Search<'a> {
        Search {
            evaluator,
            nodes: 0,
            keys: Vec::new(),
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

    /// Whether `pos` is drawn by rule, so that searching it is pointless:
    /// too little material, a repeated position, or the fifty-move rule.
    fn is_draw(&self, pos: &Position) -> bool {
        if insufficient_material(pos) || is_repetition(pos, &self.keys) {
            return true;
        }
        // A mate delivered by the hundredth ply still counts as a mate.
        fifty_moves_reached(pos) && !(pos.in_check() && legal_moves(pos).is_empty())
    }

    /// How many positions have been visited so far.
    #[must_use]
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Searches `pos` exactly `depth` plies deep and returns the score and
    /// the best move. `pos` is used as a scratch board and restored.
    ///
    /// # Panics
    ///
    /// Panics if `depth` is 0 or more than [`MAX_PLY`].
    pub fn fixed_depth(&mut self, pos: &mut Position, depth: u32) -> Outcome {
        assert!(
            depth >= 1 && depth as usize <= MAX_PLY,
            "search depth must be between 1 and MAX_PLY"
        );
        self.nodes += 1;
        let mut alpha = -INFINITY;
        let mut best_move = None;
        for &mv in &pseudo_legal_moves(pos) {
            self.keys.push(pos.hash());
            let undo = pos.make_move(mv);
            if pos.can_capture_king() {
                pos.unmake_move(mv, undo);
                self.keys.pop();
                continue;
            }
            let score = -self.negamax(pos, depth - 1, 1, -INFINITY, -alpha);
            pos.unmake_move(mv, undo);
            self.keys.pop();
            // The first legal move is always taken, so a best move exists
            // whenever any legal move does.
            if best_move.is_none() || score > alpha {
                alpha = score;
                best_move = Some(mv);
            }
        }
        match best_move {
            Some(_) => Outcome {
                score: alpha,
                best_move,
            },
            None => Outcome {
                score: no_moves_score(pos, 0),
                best_move: None,
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
        pos: &mut Position,
        depth: u32,
        ply: usize,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.nodes += 1;
        // The root is never scored as a draw here: a move must still be
        // chosen there. Everywhere else a drawn position ends the line.
        if self.is_draw(pos) {
            return DRAW;
        }
        if depth == 0 {
            return self.evaluator.evaluate(pos);
        }

        let mut best = -INFINITY;
        let mut any_legal = false;
        for &mv in &pseudo_legal_moves(pos) {
            self.keys.push(pos.hash());
            let undo = pos.make_move(mv);
            if pos.can_capture_king() {
                pos.unmake_move(mv, undo);
                self.keys.pop();
                continue;
            }
            any_legal = true;
            let score = -self.negamax(pos, depth - 1, ply + 1, -beta, -alpha);
            pos.unmake_move(mv, undo);
            self.keys.pop();

            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
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
        Search::new(&HandCrafted).fixed_depth(&mut fen(text), depth)
    }

    fn best(text: &str, depth: u32) -> String {
        search(text, depth)
            .best_move
            .map(|mv| mv.to_string())
            .unwrap()
    }

    /// Plain minimax with no pruning at all: the definition that alpha-beta
    /// must agree with. Returns the score and the number of nodes visited.
    fn minimax(pos: &mut Position, depth: u32, ply: usize, nodes: &mut u64) -> Score {
        *nodes += 1;
        // The same draw rules as the real search, apart from repetition,
        // which cannot arise within three plies of these positions.
        if ply > 0 && Search::new(&HandCrafted).is_draw(pos) {
            return DRAW;
        }
        if depth == 0 {
            return HandCrafted.evaluate(pos);
        }
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
        let positions = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 b - - 0 10",
            "7k/8/8/8/8/8/R7/1R4K1 w - - 0 1",
            "4k3/8/8/3q4/8/8/8/3RK3 b - - 0 1",
        ];
        for text in positions {
            for depth in 1..=3 {
                let mut pos = fen(text);
                let mut minimax_nodes = 0;
                let expected = minimax(&mut pos, depth, 0, &mut minimax_nodes);

                let mut searcher = Search::new(&HandCrafted);
                let outcome = searcher.fixed_depth(&mut pos, depth);
                assert_eq!(outcome.score, expected, "{text} depth {depth}");
                assert!(searcher.nodes() <= minimax_nodes, "{text} depth {depth}");
                assert_eq!(pos, fen(text), "the position must be restored");

                // The move it names really leads to that score.
                let mv = outcome.best_move.unwrap();
                let undo = pos.make_move(mv);
                let reply = if depth == 1 {
                    HandCrafted.evaluate(&pos)
                } else {
                    minimax(&mut pos, depth - 1, 1, &mut 0)
                };
                pos.unmake_move(mv, undo);
                assert_eq!(-reply, expected, "{text} depth {depth} move {mv}");
            }
        }
    }

    #[test]
    fn pruning_saves_work_at_depth() {
        let text = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let mut minimax_nodes = 0;
        minimax(&mut fen(text), 3, 0, &mut minimax_nodes);
        let mut searcher = Search::new(&HandCrafted);
        searcher.fixed_depth(&mut fen(text), 3);
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
        // Not visible one ply too shallow.
        assert!(!crate::search::is_mate_score(search(text, 3).score));
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

        let blind = Search::new(&HandCrafted).fixed_depth(&mut pos.clone(), 2);
        assert!(blind.score < -500, "without history Black is just lost");

        let informed = Search::new(&HandCrafted)
            .with_history(&history)
            .fixed_depth(&mut pos, 2);
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
        // Black can take the last white piece that could ever mate.
        let outcome = search("4k3/8/8/8/8/8/3r4/3RK3 b - - 0 1", 2);
        assert_eq!(outcome.score, DRAW);
        // King and knight against king: every line is drawn at once.
        assert_eq!(search("4k3/8/8/8/8/8/8/3NK3 w - - 0 1", 3).score, DRAW);
    }

    #[test]
    fn counts_nodes() {
        let mut searcher = Search::new(&HandCrafted);
        assert_eq!(searcher.nodes(), 0);
        searcher.fixed_depth(&mut Position::startpos(), 1);
        // The root plus its 20 children.
        assert_eq!(searcher.nodes(), 21);
    }

    #[test]
    #[should_panic(expected = "between 1 and MAX_PLY")]
    fn depth_zero_is_rejected() {
        Search::new(&HandCrafted).fixed_depth(&mut Position::startpos(), 0);
    }
}
