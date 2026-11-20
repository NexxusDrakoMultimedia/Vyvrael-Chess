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

pub use draw::{fifty_moves_reached, insufficient_material, is_repetition};
pub use limits::Limits;
pub use negamax::{Outcome, Report, Search};
pub use score::{
    DRAW, INFINITY, MATE, MATE_BOUND, MAX_PLY, is_mate_score, mate_in, mated_in, moves_to_mate,
};
