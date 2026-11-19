// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl engine binary.
//!
//! The UCI loop arrives in milestone 5. Until then the binary only identifies
//! itself on stderr, because stdout is reserved for UCI protocol output.

use vyvrael::{ENGINE_AUTHOR, ENGINE_NAME, ENGINE_VERSION};

fn main() {
    eprintln!("{ENGINE_NAME} {ENGINE_VERSION} by {ENGINE_AUTHOR}");
    eprintln!("The UCI interface is not implemented yet; see docs/ROADMAP.md.");
}
