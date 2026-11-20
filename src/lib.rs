// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl: a dragon-flavoured, aggressive UCI chess engine.
//!
//! This library crate holds the whole engine (board representation, move
//! generation, evaluation, search and the UCI front end) so that the binary,
//! the integration tests and the tool crates all share one implementation.

pub mod attacks;
pub mod board;
pub mod cli;
pub mod eval;
pub mod movegen;
pub mod perft;
pub mod rng;
pub mod search;

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

    /// The version must be three dot-separated numbers and must be the
    /// version written in the workspace manifest.
    #[test]
    fn version_is_the_manifest_version() {
        let parts: Vec<&str> = ENGINE_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "{ENGINE_VERSION}");
        assert!(parts.iter().all(|part| part.parse::<u32>().is_ok()));

        let manifest = include_str!("../Cargo.toml");
        let line = format!("version = \"{ENGINE_VERSION}\"");
        assert!(
            manifest.lines().any(|l| l == line),
            "{line} not in Cargo.toml"
        );
    }
}
