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

None yet.

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
