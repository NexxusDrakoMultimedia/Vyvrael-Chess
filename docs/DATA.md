# Data

Every dataset, tablebase set, opening book and network the project uses is
recorded here: source, licence, date fetched, size, how it is processed and
where it lives locally.

## Policy

- Large third-party artefacts are **never committed**. They live in the
  git-ignored directories `/data/`, `/nets/`, `/books/` (opening books) and
  `/tablebases/`, or outside the repository: the matches' opening file goes
  under the installer's prefix (see its entry below). Git also ignores every `*.pgn` and `*.zst` file.
- Each artefact's licence is confirmed on its **official source page**, not a
  mirror, and the date it was checked is recorded.
- Anything derived from data (opening files, test suites, training sets) must
  be reproducible from the recorded source with a recorded command and seed.

## Artefacts in use

### UHO_Lichess_4852_v1 (opening positions)

| Field | Value |
| --- | --- |
| Kind | opening book (EPD positions, no moves) |
| Source URL | <https://github.com/official-stockfish/books>, file `UHO_Lichess_4852_v1.epd.zip` (download: <https://github.com/official-stockfish/books/raw/refs/heads/master/UHO_Lichess_4852_v1.epd.zip>, given by the maintainer). The installer (`install_tools.py --openings`, `scripts/openings.py`) fetches it at commit `37751b08db745aa02babfa74056dd1168b9da803` (the repository's `master` on 2026-11-23; the file itself was last changed in commit `426eca422c202ef381a1380ada1873a7f185c4b9`, 2023-10-21, and is the same at the pinned commit) from <https://raw.githubusercontent.com/official-stockfish/books/37751b08db745aa02babfa74056dd1168b9da803/UHO_Lichess_4852_v1.epd.zip> |
| Licence | CC0-1.0, as stated by the repository's `LICENSE` file |
| Licence checked on | 2026-11-21; again on 2026-11-23 at the pinned commit (`LICENSE`: CC0 1.0 Universal) |
| Date fetched | By the maintainer, for Ladder Baseline 1 (2026-11-20 or earlier; the day was not recorded). By an agent on 2026-11-23, to pin the installer's digests (from a clone of the repository at the pinned commit and from the raw address above; the two were identical) |
| File(s) and size | `UHO_Lichess_4852_v1.epd.zip`, 42,877,788 bytes; it unpacks to `UHO_Lichess_4852_v1.epd`, which the repository's README lists with 2,632,036 positions |
| Checksum | SHA-256 of the unpacked `.epd`, as recorded for Ladder Baseline 2: `7a7f6470615a69c6cf23d565417701d38732876f480af90d67b42abade35644a` (see [ladder-Baseline-2.md](testing/ladder-Baseline-2.md)). SHA-256 of the zip at the pinned commit (42,877,788 bytes, one member, the `.epd` of 174,889,110 bytes): `4e298f11e8acfa106babe02968f2e61582145e7874c59284690b20b9650e0e07`; its `.epd` has the SHA-256 above, so it is the file of the baselines. Git's own identifier of the zip in that repository is `e439636101786177ece850d3607356891c1cc2cd` (a blob id, not a SHA-256), the same at the pinned commit |
| Local path | Outside the repository; its location is the setting `OPENINGS` in `scripts/local.env`. The installer puts it at `<prefix>/openings/UHO_Lichess_4852_v1.epd` and prints that line. It is never committed |
| Processing | None. The match scripts pass it to `cutechess-cli` as `-openings file=... format=epd order=random`, and add `-srand <seed>` when given a seed (`--seed` in `match.sh`, `-Seed` in `match.ps1`), which fixes that random order |
| Used for | The openings of every match, as one of the match scripts' fixed settings (see [TESTING.md](TESTING.md)): Ladder Baselines 1 and 2, the results of 0.1.0-alpha.1 and 0.1.0-alpha.2, and the calibration of `UCI_Elo` and its check |

## Entry template

### *Name*

| Field | Value |
| --- | --- |
| Kind | dataset / tablebase / opening book / network |
| Source URL | official page |
| Licence | as stated on the official page |
| Licence checked on | YYYY-MM-DD |
| Date fetched | YYYY-MM-DD |
| File(s) and size | name, bytes |
| Checksum | SHA-256 if published or computed |
| Local path | under `/data/`, `/nets/`, `/books/` or `/tablebases/` |
| Processing | exact command, arguments and seed |
| Used for | what in the project depends on it |

## Planned sources (not yet used, licences not yet verified)

These are listed so the plan is visible. Nothing below has been downloaded,
and each licence will be verified on the official page before first use.

- Lichess open database (https://database.lichess.org/): standard game PGNs,
  puzzles and evaluations, for opening extraction, a tactical suite and
  possibly training data (milestone 8).
- Self-generated training data from Vyvræl's own `datagen` (milestone 9).
- Syzygy tablebases, only if milestone 10 is approved.
