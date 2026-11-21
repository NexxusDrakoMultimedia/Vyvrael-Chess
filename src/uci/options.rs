// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The engine's UCI options: their names, types, limits and current values.
//!
//! Every option is declared in [`Options::declarations`] and set in
//! [`Options::set`], and nowhere else, so the list a GUI is shown and the
//! list the engine accepts cannot drift apart. `docs/UCI.md` documents each.

use std::time::Duration;

use crate::rng::SEED_MAX;
use crate::search::{ELO_DEFAULT, ELO_MAX, ELO_MIN};

/// Default size of the transposition table, in mebibytes.
pub const HASH_DEFAULT: u32 = 16;
/// Largest transposition table that can be asked for, in mebibytes.
pub const HASH_MAX: u32 = 65_536;

/// Most search threads that can be asked for. The search is single-threaded,
/// so the option is offered with one possible value; a match tool that sets
/// `Threads=1` is accepted and one that asks for more is told why not.
pub const THREADS_MAX: u32 = 1;

/// Default of `Move Overhead`, in milliseconds.
pub const MOVE_OVERHEAD_DEFAULT: u32 = 30;
/// Largest `Move Overhead` that can be asked for, in milliseconds.
pub const MOVE_OVERHEAD_MAX: u32 = 5_000;

/// The current value of every option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// `Hash`: transposition table size in mebibytes. Stored but not used
    /// yet: there is no transposition table before milestone 7.
    pub hash_mb: u32,
    /// `Threads`: number of search threads. Only 1 is possible until there
    /// is a parallel search.
    pub threads: u32,
    /// `Seed`: the seed of strength-affecting randomness, from 1 to
    /// [`SEED_MAX`], or 0 (the default) to take a new one from system
    /// entropy when the engine starts and at every `ucinewgame`.
    pub seed: u32,
    /// `UCI_LimitStrength`: whether to play below full strength.
    pub limit_strength: bool,
    /// `UCI_Elo`: the rating to play at when `UCI_LimitStrength` is on.
    /// Not calibrated yet: see `search::Weakening::for_elo`.
    pub elo: u32,
    /// `Move Overhead`: milliseconds kept back from every time limit for
    /// delays the engine cannot measure (GUI, network, operating system).
    pub move_overhead_ms: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            hash_mb: HASH_DEFAULT,
            threads: 1,
            seed: 0,
            limit_strength: false,
            elo: ELO_DEFAULT,
            move_overhead_ms: MOVE_OVERHEAD_DEFAULT,
        }
    }
}

impl Options {
    /// The `option` lines sent in reply to `uci`, one per option, in the
    /// order a GUI should show them.
    #[must_use]
    pub fn declarations() -> Vec<String> {
        vec![
            spin("Hash", HASH_DEFAULT, 1, HASH_MAX),
            spin("Threads", 1, 1, THREADS_MAX),
            spin("Seed", 0, 0, SEED_MAX),
            check("UCI_LimitStrength", false),
            spin("UCI_Elo", ELO_DEFAULT, ELO_MIN, ELO_MAX),
            spin("Move Overhead", MOVE_OVERHEAD_DEFAULT, 0, MOVE_OVERHEAD_MAX),
        ]
    }

    /// The time kept back from every time limit: `Move Overhead` as a
    /// duration.
    #[must_use]
    pub fn move_overhead(&self) -> Duration {
        Duration::from_millis(u64::from(self.move_overhead_ms))
    }

    /// Sets the option called `name` (letter case does not matter, as the
    /// protocol says) to `value`.
    ///
    /// # Errors
    ///
    /// Returns a message for the GUI if there is no such option or the
    /// value is missing, of the wrong type or out of range. Nothing is
    /// changed in that case.
    pub fn set(&mut self, name: &str, value: Option<&str>) -> Result<(), String> {
        match name.to_ascii_lowercase().as_str() {
            "hash" => self.hash_mb = parse_spin("Hash", value, 1, HASH_MAX)?,
            "threads" => self.threads = parse_spin("Threads", value, 1, THREADS_MAX)?,
            "seed" => self.seed = parse_spin("Seed", value, 0, SEED_MAX)?,
            "uci_limitstrength" => {
                self.limit_strength = parse_check("UCI_LimitStrength", value)?;
            }
            "uci_elo" => self.elo = parse_spin("UCI_Elo", value, ELO_MIN, ELO_MAX)?,
            "move overhead" => {
                self.move_overhead_ms = parse_spin("Move Overhead", value, 0, MOVE_OVERHEAD_MAX)?;
            }
            _ => return Err(format!("unknown option: {name}")),
        }
        Ok(())
    }
}

/// The declaration of an on/off option.
fn check(name: &str, default: bool) -> String {
    format!("option name {name} type check default {default}")
}

/// Reads the value of an on/off option: `true` or `false`.
fn parse_check(name: &str, value: Option<&str>) -> Result<bool, String> {
    match value {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(format!("{name}: expected true or false, got `{other}`")),
        None => Err(format!("{name}: expected true or false, got no value")),
    }
}

/// The declaration of a whole-number option.
fn spin(name: &str, default: u32, min: u32, max: u32) -> String {
    format!("option name {name} type spin default {default} min {min} max {max}")
}

/// Reads the value of a whole-number option, which must lie in `min..=max`.
fn parse_spin(name: &str, value: Option<&str>, min: u32, max: u32) -> Result<u32, String> {
    let problem =
        |got: &str| format!("{name}: expected a whole number from {min} to {max}, got {got}");
    let text = value.ok_or_else(|| problem("no value"))?;
    text.parse::<u32>()
        .ok()
        .filter(|number| (min..=max).contains(number))
        .ok_or_else(|| problem(&format!("`{text}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every declared option, as (name, type) pairs read back from the
    /// declarations. A name may be several words: it runs up to `type`.
    fn declared() -> Vec<(String, String)> {
        Options::declarations()
            .iter()
            .map(|line| {
                let words: Vec<&str> = line.split(' ').collect();
                assert_eq!((words[0], words[1]), ("option", "name"));
                let at = words.iter().position(|&word| word == "type").unwrap();
                assert!(at > 2, "{line}");
                (words[2..at].join(" "), words[at + 1].to_string())
            })
            .collect()
    }

    /// The default written in a declaration.
    fn declared_default(name: &str) -> String {
        let line = Options::declarations()
            .into_iter()
            .find(|line| line.starts_with(&format!("option name {name} ")))
            .unwrap();
        let words: Vec<&str> = line.split(' ').collect();
        let at = words.iter().position(|&word| word == "default").unwrap();
        words[at + 1].to_string()
    }

    #[test]
    fn every_declared_option_can_be_set_to_its_default() {
        for (name, _) in declared() {
            let default = declared_default(&name);
            let mut options = Options::default();
            assert_eq!(options.set(&name, Some(&default)), Ok(()), "{name}");
            // Setting the declared default changes nothing: the declaration
            // and the real default agree.
            assert_eq!(options, Options::default(), "{name}");
        }
    }

    #[test]
    fn option_names_ignore_letter_case() {
        let mut options = Options::default();
        for name in ["Hash", "hash", "HASH", "hAsH"] {
            assert_eq!(options.set(name, Some("32")), Ok(()), "{name}");
        }
        assert_eq!(options.hash_mb, 32);
        // Every option, in three spellings, accepts its declared default.
        for (name, _) in declared() {
            let default = declared_default(&name);
            let alternating: String = name
                .chars()
                .enumerate()
                .map(|(index, letter)| {
                    if index % 2 == 0 {
                        letter.to_ascii_uppercase()
                    } else {
                        letter.to_ascii_lowercase()
                    }
                })
                .collect();
            for spelling in [name.to_uppercase(), name.to_lowercase(), alternating] {
                let mut options = Options::default();
                assert_eq!(options.set(&spelling, Some(&default)), Ok(()), "{spelling}");
            }
        }
    }

    /// Names must match in full, and values are not as forgiving as names:
    /// `true` and `false` are lower case, as GUIs send them.
    #[test]
    fn near_misses_are_refused() {
        let mut options = Options::default();
        for name in [
            "elo",
            "limitstrength",
            "seeds",
            "Hash ",
            " Hash",
            "Hash1",
            "UCI_",
            // The options `Seed` replaced are gone.
            "DeterministicRNG",
            "RngSeed",
        ] {
            assert_eq!(
                options.set(name, Some("1")),
                Err(format!("unknown option: {name}")),
                "{name:?}"
            );
        }
        for name in ["UCI_LimitStrength"] {
            for value in ["TRUE", "FALSE", "False", "True", "1", "0", " true"] {
                assert!(options.set(name, Some(value)).is_err(), "{name} {value:?}");
            }
        }
        assert_eq!(options, Options::default());
    }

    #[test]
    fn an_unknown_option_is_refused() {
        let mut options = Options::default();
        assert_eq!(
            options.set("Nonsense", Some("1")),
            Err("unknown option: Nonsense".to_string())
        );
        assert_eq!(options.set("", None), Err("unknown option: ".to_string()));
        assert_eq!(options, Options::default());
    }

    #[test]
    fn hash() {
        assert_eq!(
            Options::declarations()[0],
            "option name Hash type spin default 16 min 1 max 65536"
        );
        let mut options = Options::default();
        assert_eq!(options.hash_mb, 16);
        for (text, value) in [("1", 1), ("64", 64), ("65536", 65_536)] {
            assert_eq!(options.set("Hash", Some(text)), Ok(()));
            assert_eq!(options.hash_mb, value);
        }
        // Out of range, not a number, or missing: refused, value kept.
        for bad in ["0", "65537", "-1", "16.5", "lots", "", " 8", "99999999999"] {
            assert_eq!(
                options.set("Hash", Some(bad)),
                Err(format!(
                    "Hash: expected a whole number from 1 to 65536, got `{bad}`"
                )),
                "{bad}"
            );
        }
        assert_eq!(
            options.set("Hash", None),
            Err("Hash: expected a whole number from 1 to 65536, got no value".to_string())
        );
        assert_eq!(options.hash_mb, 65_536);
    }

    #[test]
    fn threads() {
        assert_eq!(
            Options::declarations()[1],
            "option name Threads type spin default 1 min 1 max 1"
        );
        let mut options = Options::default();
        assert_eq!(options.threads, 1);
        assert_eq!(options.set("Threads", Some("1")), Ok(()));
        assert_eq!(options.threads, 1);
        for bad in ["0", "2", "8", "many"] {
            assert_eq!(
                options.set("threads", Some(bad)),
                Err(format!(
                    "Threads: expected a whole number from 1 to 1, got `{bad}`"
                ))
            );
        }
        assert_eq!(options, Options::default());
    }

    /// `Seed`: 0 by default, meaning a seed from entropy; any value up to the
    /// largest signed 32-bit number, which Cute Chess can still store.
    #[test]
    fn seed() {
        assert_eq!(
            Options::declarations()[2],
            "option name Seed type spin default 0 min 0 max 2147483647"
        );
        let mut options = Options::default();
        assert_eq!(options.seed, 0, "a seed from entropy unless asked");
        for (text, value) in [
            ("1", 1),
            ("822286742", crate::rng::DEFAULT_SEED),
            ("2147483647", SEED_MAX),
            ("0", 0),
        ] {
            assert_eq!(options.set("Seed", Some(text)), Ok(()));
            assert_eq!(options.seed, value);
        }
        assert_eq!(options.set("seed", Some("77")), Ok(()));
        for bad in ["-1", "2147483648", "4294967295", "0x31031996", "seed", ""] {
            assert_eq!(
                options.set("Seed", Some(bad)),
                Err(format!(
                    "Seed: expected a whole number from 0 to 2147483647, got `{bad}`"
                )),
                "{bad}"
            );
        }
        assert_eq!(options.seed, 77);
    }

    #[test]
    fn limit_strength() {
        assert_eq!(
            Options::declarations()[3],
            "option name UCI_LimitStrength type check default false"
        );
        let mut options = Options::default();
        assert!(!options.limit_strength, "full strength unless asked");
        assert_eq!(options.set("UCI_LimitStrength", Some("true")), Ok(()));
        assert!(options.limit_strength);
        assert_eq!(options.set("uci_limitstrength", Some("false")), Ok(()));
        assert!(!options.limit_strength);
        assert!(options.set("UCI_LimitStrength", Some("1")).is_err());
        assert!(options.set("UCI_LimitStrength", None).is_err());
    }

    #[test]
    fn elo() {
        assert_eq!(
            Options::declarations()[4],
            "option name UCI_Elo type spin default 1200 min 400 max 1800"
        );
        let mut options = Options::default();
        assert_eq!(options.elo, 1200);
        for (text, value) in [("400", 400), ("1800", 1800), ("1350", 1350)] {
            assert_eq!(options.set("UCI_Elo", Some(text)), Ok(()));
            assert_eq!(options.elo, value);
        }
        for bad in ["399", "1801", "0", "2800", "strong"] {
            assert_eq!(
                options.set("uci_elo", Some(bad)),
                Err(format!(
                    "UCI_Elo: expected a whole number from 400 to 1800, got `{bad}`"
                ))
            );
        }
        assert_eq!(options.elo, 1350);
        // Setting the rating does not switch limiting on.
        assert!(!options.limit_strength);
    }

    /// The whole list, in order: what a GUI is shown.
    #[test]
    fn the_declared_options() {
        let names: Vec<String> = declared().into_iter().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "Hash",
                "Threads",
                "Seed",
                "UCI_LimitStrength",
                "UCI_Elo",
                "Move Overhead"
            ]
        );
    }

    #[test]
    fn move_overhead() {
        assert_eq!(
            Options::declarations()[5],
            "option name Move Overhead type spin default 30 min 0 max 5000"
        );
        let mut options = Options::default();
        assert_eq!(options.move_overhead_ms, 30);
        assert_eq!(options.move_overhead(), Duration::from_millis(30));
        for (text, value) in [("0", 0), ("250", 250), ("5000", 5_000)] {
            assert_eq!(options.set("Move Overhead", Some(text)), Ok(()));
            assert_eq!(options.move_overhead_ms, value);
            assert_eq!(
                options.move_overhead(),
                Duration::from_millis(u64::from(value))
            );
        }
        for bad in ["-1", "5001", "30ms", "0.5", "", "lots"] {
            assert_eq!(
                options.set("move overhead", Some(bad)),
                Err(format!(
                    "Move Overhead: expected a whole number from 0 to 5000, got `{bad}`"
                )),
                "{bad}"
            );
        }
        assert_eq!(
            options.set("Move Overhead", None),
            Err("Move Overhead: expected a whole number from 0 to 5000, got no value".to_string())
        );
        assert_eq!(options.move_overhead_ms, 5_000);
        // The name is two words with one space: nothing else will do.
        for name in [
            "MoveOverhead",
            "Move  Overhead",
            "Move_Overhead",
            "Overhead",
        ] {
            assert_eq!(
                options.set(name, Some("100")),
                Err(format!("unknown option: {name}")),
                "{name:?}"
            );
        }
    }
}
