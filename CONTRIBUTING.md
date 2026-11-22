# Contributing to Vyvræl

Thanks for your interest. This file explains how to build, test and submit
changes. AI coding agents should also read [AGENTS.md](AGENTS.md).

## Build and test

```sh
cargo build --release
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
```

All three checks must pass on every commit, not only at the tip of a branch.
CI runs the same commands.

## Code style

- Rust edition 2024 on the stable toolchain pinned in `rust-toolchain.toml`
  (`rustup` installs it on first use); `rustfmt` defaults.
- No clippy warnings (`-D warnings`).
- Every source file starts with the SPDX header and copyright line:

  ```rust
  // SPDX-License-Identifier: GPL-3.0-or-later
  // Copyright (C) 2026 Nexxus Drako Multimedia
  ```

- Public items carry `///` doc comments.
- No `unsafe` unless it measurably helps, and every use is commented.
- UCI stdout is protocol output only; debug output goes to stderr or
  `info string`.

## Tests

- Add tests in the same commit as the code they cover.
- Never weaken, skip or delete a test to make it pass. Fix the code.
- Move generation changes must keep the perft suite passing (from
  milestone 2; see [docs/TESTING.md](docs/TESTING.md)).

## Strength changes need SPRT

Any change to search, evaluation or the network is accepted only if an SPRT
match against the previous version shows a gain. Record the SPRT result
(bounds, games, LLR, Elo estimate) in the commit message or in
`docs/TESTING.md`. Until the match scripts exist (planned before milestone 6), search changes
need at least a fixed-depth node-count and correctness check.

## Commits

- One logical change per commit. If the description needs "and", split it.
- Keep diffs reviewable in a few minutes; avoid more than about 300 changed
  lines. Explain in the message when that is unavoidable.
- Use [Conventional Commits](https://www.conventionalcommits.org/):
  `feat(board): add FEN parsing`, `fix: ...`, `test(movegen): ...`,
  `docs: ...`, `refactor: ...`, `chore: ...`.
- Behaviour changes update `CHANGELOG.md` and the affected docs in the same
  commit. Pure documentation changes get their own commits.
- Stage files deliberately; do not commit data files, nets, tablebases or
  books.

## Randomness

All randomness goes through the engine's `rng` module:

- one default seed, `DEFAULT_SEED = 0x31031996`, referenced by name;
- an in-house PRNG (no `rand` crate) with golden-value tests;
- no system entropy or time-based seeding, except for strength-affecting
  randomness when the UCI spin option `Seed` is 0 (the default), in which
  case the seed taken is logged with `info string`; any other `Seed` (1 to
  2147483647) is used as given;
- independent streams derive their seeds from the base seed and an index;
- non-strength-affecting randomness (Zobrist keys, magic search, tests,
  datagen, shuffles) is always deterministic; tools may take a `--seed`
  argument that defaults to `0x31031996`.

Every use of randomness must be added to the table in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and classified as
strength-affecting or not.

## Dependencies and data

- Check a dependency's licence for GPL-3.0-or-later compatibility before
  adding it, record it in [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md), and add
  it in its own commit. The engine crate should stay dependency-free.
- Large third-party artefacts are never committed. Record them in
  [docs/DATA.md](docs/DATA.md) with source URL, licence and date fetched.

## AI-assisted contributions

AI-assisted work is welcome; this project itself is 0rwx level **X**.

- Disclose the [0rwx](https://0rwx.com/) level of your contribution (0, R, W
  or X) in the pull request description.
- Whatever the level, tests must back the change. A change that cannot be
  verified by tests, perft or SPRT will not be merged on the strength of its
  description.
- You are responsible for the licence status of what you submit.

## Licence of contributions

Vyvræl is licensed GPL-3.0-or-later. By contributing you agree that your
contribution is licensed under the same terms.
