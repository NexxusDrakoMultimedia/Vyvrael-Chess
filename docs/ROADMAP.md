# Roadmap

Milestones are done in order, one branch per milestone, each branched from
the previous milestone's branch. At the start of a milestone its chunk
checklist is written here; items are ticked off in the commits that complete
them.

## Status

| # | Milestone | Branch | Status |
| --- | --- | --- | --- |
| 0 | Setup, licence, documentation set | `milestone/0-setup` | done (awaiting review) |
| 1 | Board | `milestone/1-board` | in progress |
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
for FEN round-trips and make/unmake restoring the exact position.

Zobrist hashing is ordered before make/unmake so that make/unmake maintains
the hash incrementally from its first commit and every test can compare it
with a from-scratch recompute.

- [x] `rng`: SplitMix64 generator, `DEFAULT_SEED`, golden-value tests
- [ ] `rng`: derived streams (base seed + stream index)
- [ ] `rng`: unbiased bounded integers
- [ ] `board`: `Color`
- [ ] `board`: `Square` with coordinate parsing and display
- [ ] `board`: `PieceKind` and `Piece` with FEN characters
- [ ] `board`: `Bitboard` type, set operations and bit iteration
- [ ] `board`: file/rank masks and directional shifts
- [ ] `board`: `CastlingRights`
- [ ] `board`: `Move` encoding with coordinate-notation display
- [ ] `board`: `Position` piece placement (bitboards + mailbox)
- [ ] `board`: FEN parsing with validation
- [ ] `board`: FEN serialisation and round-trip tests
- [ ] `board`: Zobrist keys from the seeded RNG
- [ ] `board`: position hash (from scratch + incremental on placement)
- [ ] `board`: make/unmake for quiet moves and captures
- [ ] `board`: make/unmake for double pawn pushes and en passant
- [ ] `board`: make/unmake for castling
- [ ] `board`: make/unmake for promotions
- [ ] `board`: coordinate-notation move parsing against a position
- [ ] `board`: whole-game make/unmake regression test
- [ ] End-of-milestone doc drift pass

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
