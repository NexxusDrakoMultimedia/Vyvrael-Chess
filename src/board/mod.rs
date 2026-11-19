// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Board representation: the primitive chess types and the position.
//!
//! See `docs/ARCHITECTURE.md` ("Board representation") for the design.

mod attackers;
mod bitboard;
mod castling;
mod color;
mod fen;
mod makemove;
mod mv;
mod notation;
mod piece;
mod position;
mod square;
pub mod zobrist;

pub use bitboard::{Bitboard, BitboardIter};
pub use castling::{CastlingRights, ParseCastlingError};
pub use color::Color;
pub use fen::{FenError, START_FEN};
pub use makemove::Undo;
pub use mv::{Move, MoveKind};
pub use notation::ParseMoveError;
pub use piece::{Piece, PieceKind};
pub use position::Position;
pub use square::{ParseSquareError, Square};
