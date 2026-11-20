# Data

Every dataset, tablebase set, opening book and network the project uses is
recorded here: source, licence, date fetched, size, how it is processed and
where it lives locally.

## Policy

- Large third-party artefacts are **never committed**. They live in the
  git-ignored directories `/data/`, `/nets/` and `/tablebases/`.
- Each artefact's licence is confirmed on its **official source page**, not a
  mirror, and the date it was checked is recorded.
- Anything derived from data (opening files, test suites, training sets) must
  be reproducible from the recorded source with a recorded command and seed.

## Artefacts in use

### UHO_Lichess_4852_v1 (opening positions)

| Field | Value |
| --- | --- |
| Kind | opening book (EPD positions, no moves) |
| Source URL | <https://github.com/official-stockfish/books>, file `UHO_Lichess_4852_v1.epd.zip` (download: <https://github.com/official-stockfish/books/raw/refs/heads/master/UHO_Lichess_4852_v1.epd.zip>, given by the maintainer) |
| Licence | CC0-1.0, as stated by the repository's `LICENSE` file |
| Licence checked on | 2026-11-21 |
| Date fetched | By the maintainer, for Ladder Baseline 1 (2026-11-20 or earlier; the day was not recorded) |
| File(s) and size | `UHO_Lichess_4852_v1.epd.zip`, 42,877,788 bytes; it unpacks to `UHO_Lichess_4852_v1.epd`, which the repository's README lists with 2,632,036 positions |
| Checksum | Not computed by the project: no agent has downloaded the file. Git's own identifier of the zip in that repository is `e439636101786177ece850d3607356891c1cc2cd` (a blob id, not a SHA-256) |
| Local path | Outside the repository; its location is the setting `OPENINGS` in `scripts/local.env`. It is never committed |
| Processing | None. The match scripts pass it to `cutechess-cli` as `-openings file=... format=epd order=random` |
| Used for | The openings of every match: Ladder Baseline 1, the results of 0.1.0-alpha.1, and the match scripts' fixed settings (see [TESTING.md](TESTING.md)) |

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
| Local path | under `/data/`, `/nets/` or `/tablebases/` |
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
