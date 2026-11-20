// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The Universal Chess Interface: how a GUI or a match tool talks to the
//! engine over standard input and output.
//!
//! See `docs/UCI.md` for every command and option.

mod command;
mod engine;
mod options;
mod output;
mod time;

pub use command::{Command, GoParams, parse};
pub use engine::{Engine, run};
pub use options::{HASH_DEFAULT, HASH_MAX, Options, THREADS_MAX};
pub use output::{bestmove_line, info_line, score_text};
pub use time::{MOVE_OVERHEAD, MOVES_ASSUMED, limits_for, time_for_move};
