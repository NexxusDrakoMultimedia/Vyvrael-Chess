# Dependencies

Every dependency is recorded here **before** it is added, with its licence
and the reason it is needed. Each dependency is added in its own commit.

## Policy

- The project is GPL-3.0-or-later. A dependency's licence must be compatible
  with that (for example MIT, BSD, Apache-2.0, Zlib, ISC, Unlicense, MPL-2.0,
  LGPL, GPL-3.0-or-later). Check the licence in the crate's own repository,
  not only its crates.io metadata.
- The engine crate (`vyvrael`) aims for **zero** dependencies. Anything heavy
  belongs in a tool crate.
- The `rand` crate is not used anywhere: randomness comes from the in-house
  `rng` module (see [ARCHITECTURE.md](ARCHITECTURE.md)).

## Engine crate: `vyvrael`

None.

## Tool crate: `vyvrael-data`

None.

## Tool crate: `vyvrael-train`

None.

## Build and CI tooling (not linked into any binary)

| Tool | Licence | Why |
| --- | --- | --- |
| Rust toolchain (rustc, cargo, rustfmt, clippy) | MIT OR Apache-2.0 | Build, format, lint, test. |
| `actions/checkout` (GitHub Action, CI only) | MIT | Check out the repository in CI. |

## Template for new entries

| Crate | Version | Licence | Used by | Why it is needed |
| --- | --- | --- | --- | --- |
| *name* | *x.y* | *SPDX expression* | *crate* | *reason, and why an in-house alternative was not chosen* |
