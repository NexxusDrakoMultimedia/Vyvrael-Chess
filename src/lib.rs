// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Vyvræl: a dragon-flavoured UCI chess engine in Rust, built with AI and
//! playing for the attack.
//!
//! This library crate holds the whole engine (board representation, move
//! generation, evaluation, search and the UCI front end) so that the binary,
//! the integration tests and the tool crates all share one implementation.

pub mod attacks;
pub mod bench;
pub mod board;
pub mod cli;
pub mod eval;
pub mod movegen;
pub mod perft;
pub mod rng;
pub mod search;
pub mod uci;

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

    /// Whether `version` is three dot-separated numbers, optionally
    /// followed by `-` and a pre-release tag of dot-separated words made of
    /// letters, digits and hyphens (`0.1.0`, `0.1.0-alpha.1`), as Semantic
    /// Versioning writes versions. Build metadata (`+...`) is not used.
    fn is_version(version: &str) -> bool {
        let (core, pre_release) = match version.split_once('-') {
            Some((core, pre_release)) => (core, Some(pre_release)),
            None => (version, None),
        };
        let numbers: Vec<&str> = core.split('.').collect();
        let number = |part: &&str| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (*part == "0" || !part.starts_with('0'))
        };
        let word = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                // A word of digits only is a number and follows their rule.
                && (!part.bytes().all(|byte| byte.is_ascii_digit()) || number(&part))
        };
        numbers.len() == 3
            && numbers.iter().all(number)
            && pre_release.is_none_or(|tag| tag.split('.').all(word))
    }

    #[test]
    fn what_counts_as_a_version() {
        for good in [
            "0.1.0",
            "1.0.0",
            "12.34.56",
            "0.1.0-alpha.1",
            "0.2.0-rc-1",
            "0.1.0-0",
            "0.1.0-alpha.0a",
        ] {
            assert!(is_version(good), "{good}");
        }
        for bad in [
            "",
            "0.1",
            "0.1.0.0",
            "0.1.x",
            "01.1.0",
            "0.1.0-",
            "0.1.0-alpha..1",
            "0.1.0-alpha.",
            "0.1.0 alpha",
            "0.1.0+build",
            "v0.1.0",
            "0.1.0-alpha_1",
            "0.1.0-alpha.01",
            "0.1.0-01",
        ] {
            assert!(!is_version(bad), "{bad}");
        }
    }

    /// The version must be a version (see [`is_version`]) and must be the
    /// one written in the workspace manifest.
    #[test]
    fn version_is_the_manifest_version() {
        assert!(is_version(ENGINE_VERSION), "{ENGINE_VERSION}");

        let manifest = include_str!("../Cargo.toml");
        let line = format!("version = \"{ENGINE_VERSION}\"");
        assert!(
            manifest.lines().any(|l| l == line),
            "{line} not in Cargo.toml"
        );
    }
}
