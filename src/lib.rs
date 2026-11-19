// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl: a dragon-flavoured, aggressive UCI chess engine.
//!
//! This library crate holds the whole engine (board representation, move
//! generation, evaluation, search and the UCI front end) so that the binary,
//! the integration tests and the tool crates all share one implementation.

/// Engine name as reported to UCI front ends (`id name`).
pub const ENGINE_NAME: &str = "Vyvræl";

/// ASCII fallback of [`ENGINE_NAME`] for front ends that cannot display `æ`.
pub const ENGINE_NAME_ASCII: &str = "Vyvrael";

/// Engine author as reported to UCI front ends (`id author`).
pub const ENGINE_AUTHOR: &str = "Nexxus Drako Multimedia";

/// Engine version, taken from the package manifest.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_fallback_is_ascii() {
        assert!(ENGINE_NAME_ASCII.is_ascii());
        assert!(!ENGINE_NAME.is_ascii());
    }

    #[test]
    fn version_matches_manifest() {
        assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!ENGINE_VERSION.is_empty());
    }
}
