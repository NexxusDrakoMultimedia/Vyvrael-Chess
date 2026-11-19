// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Board representation: the primitive chess types and the position.
//!
//! See `docs/ARCHITECTURE.md` ("Board representation") for the design.

mod color;
mod piece;
mod square;

pub use color::Color;
pub use piece::{Piece, PieceKind};
pub use square::{ParseSquareError, Square};
