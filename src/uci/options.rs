// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! The engine's UCI options: their names, types, limits and current values.
//!
//! Every option is declared in [`Options::declarations`] and set in
//! [`Options::set`], and nowhere else, so the list a GUI is shown and the
//! list the engine accepts cannot drift apart. `docs/UCI.md` documents each.

/// Default size of the transposition table, in mebibytes.
pub const HASH_DEFAULT: u32 = 16;
/// Largest transposition table that can be asked for, in mebibytes.
pub const HASH_MAX: u32 = 65_536;

/// The current value of every option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// `Hash`: transposition table size in mebibytes. Stored but not used
    /// yet: there is no transposition table before milestone 7.
    pub hash_mb: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            hash_mb: HASH_DEFAULT,
        }
    }
}

impl Options {
    /// The `option` lines sent in reply to `uci`, one per option, in the
    /// order a GUI should show them.
    #[must_use]
    pub fn declarations() -> Vec<String> {
        vec![spin("Hash", HASH_DEFAULT, 1, HASH_MAX)]
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
            _ => return Err(format!("unknown option: {name}")),
        }
        Ok(())
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
    /// declarations.
    fn declared() -> Vec<(String, String)> {
        Options::declarations()
            .iter()
            .map(|line| {
                let words: Vec<&str> = line.split(' ').collect();
                assert_eq!((words[0], words[1], words[3]), ("option", "name", "type"));
                (words[2].to_string(), words[4].to_string())
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
}
