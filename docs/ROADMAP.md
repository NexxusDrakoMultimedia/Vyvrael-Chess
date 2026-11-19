# Roadmap

Milestones are done in order, one branch per milestone, each branched from
the previous milestone's branch. At the start of a milestone its chunk
checklist is written here; items are ticked off in the commits that complete
them.

## Status

| # | Milestone | Branch | Status |
| --- | --- | --- | --- |
| 0 | Setup, licence, documentation set | `milestone/0-setup` | done (awaiting review) |
| 1 | Board | `milestone/1-board` | not started |
| 2 | Move generation | `milestone/2-movegen` | not started |
| 3 | Evaluation (hand-crafted) | `milestone/3-eval` | not started |
| 4 | Search | `milestone/4-search` | not started |
| 5 | UCI | `milestone/5-uci` | not started |
| 6 | Strength testing | `milestone/6-strength` | not started |
| 7 | Open data tooling | `milestone/7-data` | not started |
| 8 | Neural evaluation (NNUE) | `milestone/8-nnue` | not started |
| 9 | Endgame tablebases (optional, needs approval) | `milestone/9-tablebases` | not started |

## Milestone 0: setup

- [x] Cargo workspace skeleton (engine crate + two tool stubs)
- [x] `.gitignore` and `.gitattributes`
- [x] Toolchain pin and release profile
- [x] GitHub Actions CI
- [x] `LICENSE.md` (verbatim GPLv3)
- [x] `AGENTS.md` and `CLAUDE.md`
- [x] `README.md`
- [x] `CONTRIBUTING.md`
- [x] `CHANGELOG.md`
- [x] `docs/ARCHITECTURE.md`
- [x] `docs/UCI.md`
- [x] `docs/TESTING.md`
- [x] `docs/ROADMAP.md`
- [x] `docs/DEPENDENCIES.md` and `docs/DATA.md`
- [x] `docs/NNUE.md`

## Milestone 1: board

Scope: the `rng` module first, then bitboards, FEN parse/serialise,
make/unmake and Zobrist hashing (keys from the seeded RNG), with unit tests
for FEN round-trips and make/unmake restoring the exact position. The chunk
checklist is written at the start of the milestone.

## Milestones 2 to 9

Scope summaries live in the "Build order" section of
[AGENTS.md](../AGENTS.md). Each milestone's chunk checklist is added here when
that milestone starts.

## Ideas for later

Not commitments; candidates to evaluate (each needs SPRT evidence):

- Aggression tuning: king-safety and king-attack terms, mobility, initiative.
- Search: aspiration windows, principal variation search, singular
  extensions, futility and reverse futility pruning, SEE-based pruning,
  continuation history.
- Lazy SMP for `Threads > 1`.
- Time management tuned by self-play.
- Chess960 (Fischer random) support.
- Larger or bucketed NNUE architectures; training-data selection favouring
  attacking play.
- Opening book support for match play.
