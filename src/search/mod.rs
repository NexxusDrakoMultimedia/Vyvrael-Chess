// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Search: looking ahead to choose a move.
//!
//! See `docs/ARCHITECTURE.md` ("Search") for the design.

mod draw;
mod limits;
mod negamax;
mod ordering;
mod score;
mod variety;
mod weaken;

pub use draw::{fifty_moves_reached, insufficient_material, is_repetition};
pub use limits::Limits;
pub use negamax::{Outcome, Report, Search};
pub use score::{
    DRAW, INFINITY, MATE, MATE_BOUND, MAX_PLY, is_mate_score, mate_in, mated_in, moves_to_mate,
};
pub use variety::{
    VARIETY_FULL_MOVES, VARIETY_MAX, Varied, split_limits, varied_move, variety_applies,
};
pub use weaken::{ELO_DEFAULT, ELO_MAX, ELO_MIN, Weakened, Weakening, weakened_move};
