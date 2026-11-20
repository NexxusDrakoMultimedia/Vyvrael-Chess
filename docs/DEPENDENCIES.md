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

## External programs (run separately, never linked or vendored)

These are programs the project's scripts call. They are not Cargo
dependencies, are not linked into any binary and are not copied into the
repository. `scripts/install_tools.py` installs exactly these versions on
Linux; updating one means changing its pin there and its row here in the same
commit.

| Program | Pinned version | Licence | Used for |
| --- | --- | --- | --- |
| Cute Chess (`cutechess-cli`) | v1.5.1, release file checked by SHA-256 | GPL-3.0 | Engine-against-engine matches and SPRT (milestone 6). |
| Ordo | v1.2.6, commit `17eec774f2e4b9fdd2b1b38739f55ea221fb851a`, built from source | GPL-3.0 | Rating lists from match PGN (milestone 6). |
| lichess-bot | 2026.11.16.2, commit `06f4f10b5a5a70549b9fc5f8f759a532723b5dab` | AGPL-3.0 | Connecting the engine to Lichess (planned feature). |

Licences are as stated by the licence file in each project's own repository
(`COPYING`, `LICENSE.txt` and `LICENSE` respectively), checked on 2026-11-20.
Whether each is "or later" was not checked; it does not matter here because
none of them is linked with or distributed as part of Vyvræl. Running a
separate GPL or AGPL program does not affect Vyvræl's own licence.

lichess-bot installs its own Python packages (`chess`, `PyYAML`, `requests`,
`backoff`, `rich`) into a private virtual environment. Its `requirements.txt`
gives compatible-release ranges, not exact versions, so those packages are
not pinned by this project.

## Template for new entries

| Crate | Version | Licence | Used by | Why it is needed |
| --- | --- | --- | --- | --- |
| *name* | *x.y* | *SPDX expression* | *crate* | *reason, and why an in-house alternative was not chosen* |
