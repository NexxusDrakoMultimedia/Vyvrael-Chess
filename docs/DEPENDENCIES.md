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
| Rust toolchain (rustc, cargo, rustfmt, clippy) | MIT OR Apache-2.0 | Build, format, lint, test. Pinned to 1.100.0 in `rust-toolchain.toml` (since 2026-11-23), so local builds, CI and release binaries use the same compiler. |
| `actions/checkout` (GitHub Action, CI and release workflow) | MIT | Check out the repository. Both workflows use v7.0.1, pinned to commit `3d3c42e5aac5ba805825da76410c181273ba90b1`. |
| `actions/upload-artifact` (GitHub Action, release workflow only) | MIT | Pass each build job's binary and its `.sha256` file to the release job. v7.0.2, pinned to commit `cf430e030ddbb5b0abf93d22962f4752f3646cd9`. |
| `actions/download-artifact` (GitHub Action, release workflow only) | MIT | Collect those files in the release job. v8.0.2, pinned to commit `9000827ccba6bdab643e8b6fd33ac0654aef8333`. |
| GitHub CLI `gh` (preinstalled on GitHub's runners, release workflow only) | MIT | Create the draft release (`gh release create`). Not pinned: the runner's copy is used. |

The licences of the three actions and of `gh` are as GitHub reports them
from each repository's licence file (`actions/checkout`,
`actions/upload-artifact`, `actions/download-artifact`, `cli/cli`),
checked on 2026-11-23.

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
| lichess-bot | 2026.11.16.2, commit `06f4f10b5a5a70549b9fc5f8f759a532723b5dab` | AGPL-3.0 | Running the casual Lichess bot on the Linux machine (since 0.1.0-alpha.1; see [LICHESS.md](LICHESS.md)). |

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

## Opponent engines of the ladder (Linux and Windows)

The engines of Ladder Baseline 2, the reference ladder, as
`scripts/install_tools.py --engines` installs them on Linux and on Windows
(the Windows table follows the Linux one). They are separate programs, run
only as opponents in matches:
none is linked into Vyvræl, vendored or redistributed by this project. The
full pins (addresses, complete SHA-256 digests, build commands and notes)
are in `scripts/ladder_engines.py`, which is the place to change them.
Sources and licences were read on each project's own page or repository on
2026-11-21, and for the eleven engines new in Ladder Baseline 3 (the last
rows of each table) on 2026-11-22. The limited entries of the ladder (for example "Stockfish
19-1500") are the same binary with `UCI_LimitStrength` on.

| Engine | Version | Licence | How | Source | SHA-256 of the download |
| --- | --- | --- | --- | --- | --- |
| Stockfish 19 (also limited to 1320, 1500, 1800, 2000, 2200, 2400) | sf_19 | GPL-3.0 | official binary | <https://github.com/official-stockfish/Stockfish> | `9defc0d4e55d49c6...` |
| Arasan 26.1 (also limited to 1500) | v26.1 | MIT | official binary | <https://github.com/jdart1/arasan-chess> | `33c0c4f8f70c3ed5...` |
| Viridithas 20 | v20.0.0 | MIT | official binary | <https://github.com/cosmobobak/viridithas> | `eb53de2fde546f3b...` |
| Reckless 0.9.0 | v0.9.0 | AGPL-3.0 | official binary | <https://github.com/codedeliveryservice/Reckless> | `09ba1634faaffec5...` |
| Stash 37.0 | v37.0 | GPL-3.0 | official binary (popcnt; bmi2 for `pext`) | <https://gitlab.com/mhouppin/stash-bot> | `8643547bfe9505d5...` (`pext`: `70f8935f8e45cefe...`) |
| Princhess 0.22.0 | 0.22.0 | GPL-3.0 | built from official source | <https://github.com/princesslana/princhess> | `a21d53cefdcad7b4...` |
| Weiss 2.0 | v2.0 | GPL-3.0 | built from official source | <https://github.com/TerjeKir/weiss> | `ddcd263e493987bd...` |
| Rodent IV (also limited to 1600, 2000) | 0.33 (commit e8d84c8) | GPL-3.0 | built from official source | <https://github.com/nescitus/rodent-iv> | `db372b48a8ccf22d...` |
| Vice 1.1 | 1.1 (commit 7d4fe19) | MIT | built from official source | <https://github.com/bluefeversoft/vice> | `4bcd7995ab932bd7...` |
| Fruit 2.1 | 2.1 (Debian's original tarball, 2.1.dfsg) | GPL-2.0 | built from source kept by Debian | <https://deb.debian.org/debian/pool/main/f/fruit/> | `9bf6364a15f697ed...` |
| Rustic Alpha 1.5 | 1.5.0 (branch release/alpha-1/1.5) | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| Rustic Alpha 2.4 | alpha-2.4.0 | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| Rustic Alpha 3.0.6 | alpha-3.0.6 | GPL-3.0 | official binary | <https://codeberg.org/mvanthoor/rustic> | `ac385c81fdc42a2f...` |
| CT800 1.46 (also limited to 1200, 1500, 1800) | V1.46 | GPL-3.0-or-later | built from official source (zip) | <https://www.ct800.net/> | `61647c7146fd8b32...` |
| Blunder 4.0.0 | v4.0.0 | MIT | official binary (zip) | <https://github.com/deanmchris/blunder> | `2c3ad87d3ed70158...` |
| Blunder 5.0.0 | v5.0.0 | MIT | official binary (zip) | <https://github.com/deanmchris/blunder> | `6d78c63bbc7eefa2...` |
| Blunder 7.1.0 | v7.1.0 | MIT | official binary (zip) | <https://github.com/deanmchris/blunder> | `59670c5290e00e8a...` |
| Blunder 7.6.0 | v7.6.0 | MIT | official binary (zip) | <https://github.com/deanmchris/blunder> | `b6e7a29b09df81be...` |
| MinimalChess 0.4.1 | v0.4 | MIT | official binary (zip, self-contained .NET) | <https://github.com/lithander/MinimalChessEngine> | `cf6ea66b0be29cec...` |
| MinimalChess 0.5 | v0.5 | MIT | official binary (zip, self-contained .NET) | <https://github.com/lithander/MinimalChessEngine> | `367baea84b0630c2...` |
| MinimalChess 0.6 | v0.6 | MIT | official binary (zip, self-contained .NET) | <https://github.com/lithander/MinimalChessEngine> | `b1dacf326ae50eb3...` |
| Velvet 1.0.0 | v1.0.0 | GPL-3.0-or-later | official binary (`modern`) | <https://github.com/mhonert/velvet-chess> | `712440e9103f2091...` |
| Inanis 1.0.1 | v1.0.1 | GPL-3.0 | official binary (zip; popcnt, popcnt_bmi2 for `pext`) | <https://github.com/Tearth/Inanis> | `e29d2e0ffad2dad4...` (`pext`: `397b2c76f5774b11...`) |
| Tantabus 1.0.2 | v1.0.2 | MPL-2.0 | official binary (x86-64-v3) | <https://github.com/analog-hors/tantabus> | `01ba05307ea56175...` |
| Tantabus 2.0.0 | v2.0.0 | MPL-2.0 | official binary (x86-64-v3) | <https://github.com/analog-hors/tantabus> | `3ef81dfde621b47f...` |

Licences are as each repository's licence file states them. Arasan's is the
MIT licence with further terms for its GUI, which is not used here. All of
them, and Fruit's GPL-2.0, permit downloading the program or building it
from source for private testing. Tantabus's MPL-2.0 is compatible with the
GPL through its section 3.3 ("Secondary License"), as no file of its
source carries the notice that would rule that out (checked at v1.0.2 and
v2.0.0 on 2026-11-22); the maintainer accepts GPL-3.0-compatible licences for
opponents (2026-11-22).

Left out on Linux, with the reason:

- **Orion 1.1:** The official site (orionchess.com) offers Windows programs only and the source is not published, so there is nothing to download or build for Linux.
- **Ethereal 12.75:** No official Linux binary exists, and its source does not run when built with a current compiler: the engine crashes at the first `go` (gcc 15.2, 2026-11-21), because it allocates data that it declares 64-byte aligned with `calloc`. Building it would mean patching the engine.
- **Nostradamus 0.4** (<https://github.com/david-carteau/nostradamus>, MIT): not a compiled program but a Python program with a PyTorch model. Its only release file is a Windows executable, and the model's weights are not in the repository but in a zip on the author's site that no page there links to and no digest covers. Installing it would need a Python environment with PyTorch, which the installer does not set up (the maintainer's decision, 2026-11-21).

On Windows every engine of the ladder is installed, all from official
Windows files except Fruit 2.1 (below), and nothing is compiled. Each file
was downloaded once on 2026-11-21 and its SHA-256 compared: the file of the
`pext` build is in every case the very file of Ladder Baseline 2, except
Stash, whose Baseline 2 file was its generic build. Where a project
publishes no digest, the recorded one is what it served that day.

| Engine | Windows file (`avx2`; `pext` if different) | Licence | From | SHA-256 of the program |
| --- | --- | --- | --- | --- |
| Stockfish 19 (and its six limited entries) | `stockfish-windows-x86-64-universal.exe` in the release zip | GPL-3.0 | GitHub release sf_19 | `45bc8e4969147db9...` |
| Arasan 26.1 (and its 1500 entry) | `arasanx-64-avx2.exe`; `arasanx-64-avx2-bmi2.exe`, in `arasan26.1.zip`, with its network and book beside it | MIT | GitHub release v26.1 | `67ff0441343bd5f6...`; `cca0e46c7a1d18d4...` |
| Viridithas 20 | `viridithas-20-win-x86-64-v3.exe` | MIT | GitHub release v20.0.0 | `c7fa86b2b46fe1c3...` |
| Reckless 0.9.0 | `reckless-windows-avx2.exe` | AGPL-3.0 | GitHub release v0.9.0 | `b74ead5648cfa7a7...` |
| Stash 37.0 | `stash-37.0-windows-x86_64-popcnt.exe`; `...-bmi2.exe` | GPL-3.0 | GitLab release v37.0 | `e86edfdafa35416c...`; `7cd0e8bb0bf48e81...` |
| Princhess 0.22.0 | `princhess.exe` | GPL-3.0 | GitHub release 0.22.0 | `7a95e41985483b86...` |
| Ethereal 12.75 | `Ethereal12.75-x64-popcnt-avx2.exe`; `...-pext-avx2.exe` | GPL-3.0 | GitHub release v12.75 | `5e891c6d5f0775ab...`; `22f12797fd43e220...` |
| Weiss 2.0 | `weiss-popcnt.exe`; `weiss-pext.exe`, in the Windows collection zip | GPL-3.0 | GitHub release v2.0 | `c56623c15c777442...`; `5bc2262139ae0712...` |
| Orion 1.1 | `orion64-v1.1-pop-avx.exe` in `Orion-v1.1.zip`, its network beside it | Freeware, closed source (free use, unmodified redistribution) | <https://www.orionchess.com/> | `712ba9848e84c9a7...` |
| Rodent IV (and its two) | `rodent-iv-x64.exe` in `FinalRodent.7z`, its personalities and books beside it | GPL-3.0 | the author's site, <http://pkoziol.cal24.pl/rodent/rodent.htm> (plain HTTP only) | `39c879b2dfdaf025...` |
| Vice 1.1 | `vice11_win32.exe` in `Vice11.zip` (a 32-bit program) | MIT | <https://www.bluefeversoft.com/> | `92827b547b32b7a7...` |
| Fruit 2.1 | `fruit_21.exe` | GPL-2.0 | <https://github.com/Warpten/Fruit-2.1>, commit `09627df` | `7f321a960887af6c...` |
| Rustic Alpha 1.5, 2.4, 3.0.6 | `rustic-alpha-<version>-win64.exe` in the same zip as on Linux | GPL-3.0 | Codeberg release alpha-3.0.6 | `2ee1c05f...`, `07803166...`, `fa8c6511...` |
| CT800 1.46 (and its three) | `CT800_V1.46_x64.exe` in the same zip as on Linux | GPL-3.0-or-later | <https://www.ct800.net/> | `dc0c1f6e49c56898...` |
| Nostradamus 0.4 | `nostradamus-v0.4.exe` (about 156 MB, its model inside) | MIT | GitHub release v0.4 | `485d3b606ad8784e...` |
| Blunder 4.0.0, 5.0.0, 7.1.0 | `blunder.exe` in the same zip as on Linux | MIT | GitHub releases | `3af0456d...`, `715d196e...`, `f9ac2cf2...` |
| Blunder 7.6.0 | `blunder-windows.exe` in the same zip as on Linux | MIT | GitHub release v7.6.0 | `15c76fe1bf16d1f3...` |
| MinimalChess 0.4.1, 0.5, 0.6 | `MinimalChessEngine.exe` in `MinimalChess.<version>.Windows.zip` | MIT | GitHub releases | `55147a2a...`, `bf1e8a54...`, `99623e8f...` |
| Velvet 1.0.0 | `velvet-v1.0.0-x86_64-modern.exe` | GPL-3.0-or-later | GitHub release v1.0.0 | `338d7d0606dc4009...` |
| Inanis 1.0.1 | `inanis.exe` in the popcnt zip; in the popcnt_bmi2 zip | GPL-3.0 | GitHub release v1.0.1 | `3be5b6cecebb64df...`; `4e3ed4dafd1c39a6...` |
| Tantabus 1.0.2 | `tantabus-windows-2019-x86-64-v3.exe` | MPL-2.0 | GitHub release v1.0.2 | `5da552a4459e52e0...` |
| Tantabus 2.0.0 | `tantabus-windows-2019-x86-64-v3.exe` | MPL-2.0 | GitHub release v2.0.0 | `8c343ed1797c923b...` |

Fruit 2.1 has no official Windows download (the author's site is gone);
its file is taken from `github.com/Warpten/Fruit-2.1` at a fixed commit, as
the maintainer chose on 2026-11-21. Rodent IV's Windows release is on the
author's own site, which serves plain HTTP only (its HTTPS certificate is
for another host name); the installer allows that for this one pin, which
says why, and still checks the digest. The fruitchess.com beta of
2005-11-03 is a possible later addition, not part of the ladder.

On Linux, Fruit 2.1 is the one engine not taken from its project's own
page: the author's site is gone, so the source is the original tarball as
Debian keeps it, which the maintainer accepted on 2026-11-21. (On Windows
its file comes from `github.com/Warpten/Fruit-2.1`, as said above.)

The engine list gives every engine Ladder Baseline 2's settings under the
engine's own option names, as read from each engine's `uci` answer on
2026-11-21: books off (Arasan `OwnBook`, Fruit `OwnBook`, Vice `Book`,
Rodent IV `UseBook`, Weiss `NoobBook`, CT800 `OwnBook`), tablebases off
(Arasan `Use tablebases`, Weiss `OnlineSyzygy`), CT800's `Keep Hash Tables`
off, and 30 ms of move overhead where there is the option (Stockfish `Move
Overhead`, Arasan `Move overhead`, Reckless, Stash and Ethereal
`MoveOverhead`, Rodent IV `TimeBuffer`, CT800 `Move Overhead [ms]`, Inanis
`Move Overhead`). Orion, Viridithas, Princhess, Vice, Fruit, Rustic Alpha
and Nostradamus have no move-overhead option, nor do Blunder, MinimalChess
Velvet and Tantabus of Ladder Baseline 3; Blunder 7.6.0's `UseBook` is
set off. Of
the new engines only Inanis and Tantabus 2.0.0 have a Threads option
(Tantabus's runs from 1 to 1). The book files that
Arasan, Fruit and Rodent IV ship are still installed beside them, unused.

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
level (they contain no AVX-512). Of Ladder Baseline 3's engines, Blunder's
Go programs name no processor level either; MinimalChess's are
self-contained .NET programs, which compile for the processor when they
run; Velvet 1.0.0 has no AVX2 file, so its `modern` build (popcnt, lzcnt,
BMI1) is used; Tantabus's files are its x86-64-v3 builds.

**Two builds.** The installer takes the `avx2` build by default and the
`pext` build with `--engine-build pext`, or else with
`VYVRAEL_ENGINE_BUILD=pext` in the environment, or else with
`ENGINE_BUILD=pext` in `scripts/local.env`, in that order of precedence.
Only engines that offer a PEXT build of their own differ: on Linux Arasan
(`arasanx-64-avx2` or `arasanx-64-avx2-bmi2` from the same archive), Stash
(its popcnt or its bmi2 file; it has no AVX2 build), Weiss (built
without or with `-DUSE_PEXT`) and Inanis (its popcnt or its popcnt_bmi2
zip; it has no AVX2 build); on Windows Arasan and Weiss (two files of one
archive each), Stash and Inanis (their popcnt or bmi2 files) and Ethereal
(`popcnt-avx2` or `pext-avx2`). Every other engine has one build for both.
PEXT is fast on Intel from Haswell and on AMD from Zen 3, and slow on AMD
Zen 1 and Zen 2.
