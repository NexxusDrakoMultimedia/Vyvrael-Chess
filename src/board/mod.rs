// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Board representation: the primitive chess types and the position.
//!
//! See `docs/ARCHITECTURE.md` ("Board representation") for the design.

mod bitboard;
mod castling;
mod color;
mod mv;
mod piece;
mod position;
mod square;

pub use bitboard::{Bitboard, BitboardIter};
pub use castling::{CastlingRights, ParseCastlingError};
pub use color::Color;
pub use mv::{Move, MoveKind};
pub use piece::{Piece, PieceKind};
pub use position::Position;
pub use square::{ParseSquareError, Square};
