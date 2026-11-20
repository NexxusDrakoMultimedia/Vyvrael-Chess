// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Move generation.
//!
//! See `docs/ARCHITECTURE.md` ("Attacks and move generation") for the design.

mod castling;
mod legal;
mod list;
mod pawns;
mod pseudo;

pub use legal::{find_legal_move, legal_moves};
pub use list::MoveList;
pub use pseudo::pseudo_legal_moves;
