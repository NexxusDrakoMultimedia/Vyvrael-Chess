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
commit. On Windows the same script installs the same versions of Cute Chess
and Ordo from their official zip archives, pinned in
`scripts/windows_tools.py`: change both pins together.

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

## Opponent engines of the ladder (Linux)

The engines of Ladder Baseline 2, the reference ladder, as
`scripts/install_tools.py` is to install them on Linux. They are separate programs, run only as opponents in matches:
none is linked into Vyvræl, vendored or redistributed by this project. The
full pins (addresses, complete SHA-256 digests, build commands and notes)
are in `scripts/ladder_engines.py`, which is the place to change them.
Sources and licences were read on each project's own page or repository on
2026-11-21. The limited entries of the ladder (for example "Stockfish
19-1500") are the same binary with `UCI_LimitStrength` on.

| Engine | Version | Licence | How | Source | SHA-256 of the download |
| --- | --- | --- | --- | --- | --- |
| Stockfish 19 (also limited to 1320, 1500, 1800, 2000, 2200, 2400) | sf_19 | GPL-3.0 | official binary | <https://github.com/official-stockfish/Stockfish> | `9defc0d4e55d49c6...` |
| Arasan 26.1 (also limited to 1500, 1800) | v26.1 | MIT | official binary | <https://github.com/jdart1/arasan-chess> | `33c0c4f8f70c3ed5...` |
| Viridithas 20 | v20.0.0 | MIT | official binary | <https://github.com/cosmobobak/viridithas> | `eb53de2fde546f3b...` |
| Reckless 0.9.0 | v0.9.0 | AGPL-3.0 | official binary | <https://github.com/codedeliveryservice/Reckless> | `09ba1634faaffec5...` |
| Stash 37.0 | v37.0 | GPL-3.0 | official binary | <https://gitlab.com/mhouppin/stash-bot> | `70f8935f8e45cefe...` |
| Princhess 0.22.0 | 0.22.0 | GPL-3.0 | built from official source | <https://github.com/princesslana/princhess> | `a21d53cefdcad7b4...` |
| Weiss 2.0 | v2.0 | GPL-3.0 | built from official source | <https://github.com/TerjeKir/weiss> | `ddcd263e493987bd...` |
| Rodent IV (also limited to 1600, 2000) | 0.33 (commit e8d84c8) | GPL-3.0 | built from official source | <https://github.com/nescitus/rodent-iv> | `db372b48a8ccf22d...` |
| Vice 1.1 | 1.1 (commit 7d4fe19) | MIT | built from official source | <https://github.com/bluefeversoft/vice> | `4bcd7995ab932bd7...` |
| Fruit 2.1 | 2.1 (Debian's original tarball, 2.1.dfsg) | GPL-2.0 | built from source kept by Debian | <https://deb.debian.org/debian/pool/main/f/fruit/> | `9bf6364a15f697ed...` |
| Rustic Alpha 1.5 | 1.5.0 (branch release/alpha-1/1.5) | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| Rustic Alpha 2.4 | alpha-2.4.0 | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| Rustic Alpha 3.0.6 | alpha-3.0.6 | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| CT800 1.46 (also limited to 1200, 1500, 1800) | V1.46 | GPL-3.0-or-later | built from official source (zip) | <https://www.ct800.net/> | `61647c7146fd8b32...` |

Licences are as each repository's licence file states them. Arasan's is the
MIT licence with further terms for its GUI, which is not used here. All of
them, and Fruit's GPL-2.0, permit downloading the program or building it
from source for private testing.

Left out, with the reason:

- **Orion 1.1:** The official site (orionchess.com) offers Windows programs only and the source is not published, so there is nothing to download or build for Linux.
- **Ethereal 12.75:** No official Linux binary exists, and its source does not run when built with a current compiler: the engine crashes at the first `go` (gcc 15.2, 2026-11-21), because it allocates data that it declares 64-byte aligned with `calloc`. Building it would mean patching the engine.
- **Nostradamus 0.4** (<https://github.com/david-carteau/nostradamus>, MIT): not a compiled program but a Python program with a PyTorch model. Its only release file is a Windows executable, and the model's weights are not in the repository but in a zip on the author's site that no page there links to and no digest covers. Installing it would need a Python environment with PyTorch, which the installer does not set up (the maintainer's decision, 2026-11-21).

Fruit 2.1 is the one engine not taken from its project's own page: the
author's site is gone, so the source is the original tarball as Debian keeps
it, which the maintainer accepted on 2026-11-21. Arasan is installed with
its own opening book, as it was in Ladder Baseline 1's run. CT800 is the
first pin with options of its own in the engine list (book off, hash
cleared between games, 30 ms of move overhead), as in Ladder Baseline 2;
the other engines still get only Hash, Threads and their strength
settings.

CT800's zip also holds the Windows binary of Ladder Baseline 2, identical by
SHA-256, but no Linux binary: its readme says to build with
`make_ct800_linux.sh`, which compiles with `-march=native`. The installer
runs the same `gcc` command with `-march=haswell` instead and without
`-Werror` (the readme's own workaround for compiler warnings). No digest is
published for the zip; the one recorded is what ct800.net served on
2026-11-21.

Targets: x86-64 with AVX2 and BMI2. No AVX-512 build is used and nothing is
compiled with `-march=native`; a compiled engine names `haswell` or
`x86-64-v3`, except Vice 1.1, whose makefile names no architecture (plain
x86-64). Stockfish 19 ships one Linux binary that chooses its code path
at run time, and the Rustic Alpha binaries do not state their processor
level (they contain no AVX-512).
