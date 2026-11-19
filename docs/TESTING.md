# Testing

Vyvræl is a 0rwx level X project, so tests are the main source of trust.
This document explains what is tested, how to run it and how to read the
results. It is updated in the same commit as any new test, suite or script.

## The three gates

Every commit must pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
```

CI (`.github/workflows/ci.yml`) runs exactly these.

## Unit tests

Unit tests live next to the code in `#[cfg(test)]` modules.

| Area | What is checked |
| --- | --- |
| crate root | The ASCII fallback name is ASCII; the version constant matches the manifest. |
| `rng` | Golden values for seeds `1234567` (published SplitMix64 vector), `0` and `DEFAULT_SEED`; same seed gives the same sequence; different seeds differ; `next_u32` is the high half of `next_u64`; usable in `const` context; golden values for derived streams; 1000 streams and the base generator all start differently. |

Rows are added as modules land.

## Perft (planned, milestone 2)

Perft ("performance test") counts the leaf nodes of the legal move tree to a
fixed depth from a given position. Because the correct counts for standard
positions are published, any disagreement proves a move generation bug, and
the *divide* mode (node count per root move) lets you bisect to the faulty
move by comparing against a reference engine.

Milestone 2 will add a perft integration test over the standard reference
positions (start position, Kiwipete and the other well-known positions that
stress castling, en passant, promotion and pins) and a `perft` subcommand
with divide mode. The positions and their expected node counts will be listed
here, with their source, when the tests land.

## Strength matches and SPRT (planned, milestone 6)

Search, evaluation and network changes are accepted only when a sequential
probability ratio test (SPRT) against the previous version shows a gain. A
match script using fastchess or Cutechess will be added in milestone 6 and
documented here, including how to read LLR, Elo estimates and error bars.
Until then, search changes need at least a fixed-depth node-count and
correctness check.

SPRT results are recorded in the commit message of the change or in a table
in this file.

## Tactical test suites (planned, milestone 7)

A tactical suite derived from the Lichess puzzle database will report the
share of puzzles solved at a fixed depth or time. Details will be added with
the tooling.

## Reproducible runs

- All non-strength-affecting randomness (Zobrist keys, magic numbers, test
  fixtures) is generated from the fixed seed `0x31031996` and is identical on
  every run and platform.
- Strength-affecting randomness is seeded from system entropy by default. For
  a reproducible run set the UCI options `DeterministicRNG = true` and, if
  wanted, `RngSeed` (planned, milestone 5). When `DeterministicRNG` is
  `false`, the engine logs the seed it chose with `info string`, so a run can
  be reproduced afterwards.
- **Multithreaded search is not bit-for-bit deterministic**, whatever the
  seed: thread scheduling changes the order in which shared tables are
  written. For reproducible results use **one thread, a fixed seed, and a
  fixed depth or node count** (never a time limit). Regression tests and
  performance comparisons use these settings.

## Reading results

- `cargo test` prints one `test result:` line per test binary; any `FAILED`
  line is a regression.
- A failing test is fixed by fixing the code. Tests are never weakened,
  skipped or deleted to get a green run.
