// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Attack tables: which squares a piece on a given square attacks.
//!
//! See `docs/ARCHITECTURE.md` ("Attacks and move generation") for the design.

mod leapers;

pub use leapers::{king_attacks, knight_attacks, pawn_attacks};
