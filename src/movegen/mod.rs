// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Move generation.
//!
//! See `docs/ARCHITECTURE.md` ("Attacks and move generation") for the design.

mod list;
mod pawns;
mod pseudo;

pub use list::MoveList;
pub use pseudo::pseudo_legal_moves;
