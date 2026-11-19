# Roadmap

Milestones are done in order, one branch per milestone. A finished milestone
is reviewed and merged into `main`, and the next one branches from there. At the start of a milestone its chunk
checklist is written here; items are ticked off in the commits that complete
them.

## Status

| # | Milestone | Branch | Status |
| --- | --- | --- | --- |
| 0 | Setup, licence, documentation set | `milestone/0-setup` | done, merged into `main` |
| 1 | Board | `milestone/1-board` | done, merged into `main` |
| 2 | Move generation | `milestone/2-movegen` | in progress |
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
- [x] `rng`: derived streams (base seed + stream index)
- [x] `rng`: unbiased bounded integers
- [x] `board`: `Color`
- [x] `board`: `Square` with coordinate parsing and display
- [x] `board`: `PieceKind` and `Piece` with FEN characters
- [x] `board`: `Bitboard` type, set operations and bit iteration
- [x] `board`: file/rank masks and directional shifts
- [x] `board`: `CastlingRights`
- [x] `board`: `Move` encoding with coordinate-notation display
- [x] `board`: `Position` piece placement (bitboards + mailbox)
- [x] `board`: FEN parsing (syntax)
- [x] `board`: FEN position validation (kings, pawns, castling, en passant)
- [x] `board`: FEN serialisation and round-trip tests
- [x] `board`: Zobrist keys from the seeded RNG
- [x] `board`: position hash (from scratch + incremental on placement)
- [x] `board`: make/unmake for quiet moves and captures
- [x] `board`: make/unmake for double pawn pushes and en passant
- [x] `board`: make/unmake for castling
- [x] `board`: make/unmake for promotions
- [x] `board`: coordinate-notation move parsing against a position
- [x] `board`: whole-game make/unmake regression test
- [x] End-of-milestone doc drift pass

Added during the milestone: a fix for the hash when pawns are edited beside
an en passant square, and three follow-ups from the end-of-milestone review
(documented and fully tested square constants, missing Black test cases, a
version test that can fail).

What milestone 1 leaves open is recorded under "Known issues" below.

## Known issues

Limitations in the code as it stands, each with what it affects and when it
is due to be fixed. An entry is removed in the commit that fixes it. The
design behind each one is explained in [ARCHITECTURE.md](ARCHITECTURE.md).

| # | Issue | Effect today | Fix planned |
| --- | --- | --- | --- |
| KI-1 | **En passant hashing ignores pins.** The en passant file is hashed when a pawn of the side to move attacks the en passant square, even if that capture is illegal because it would expose the mover's own king. | Two positions that are the same under the rules can hash differently, so a repetition could be missed or a transposition-table entry not reused. Nothing consumes the hash yet (no search, no repetition detection). | Milestone 2: hash the file only when an en passant capture is legal. |
| KI-2 | **FEN parsing rejects inconsistent positions and has no repair mode.** A castling right without its king and rook at home, or an en passant square that does not match a double push, is an error. | Sloppy FENs from other tools (stale castling rights are the usual case) fail to load instead of being corrected. | Later, milestone not fixed yet: an opt-in "repair and warn" mode that drops the bad right or square and reports what it changed. Strict parsing stays the default. To be revisited by milestone 7 at the latest, when third-party data is first read. |
| KI-3 | **`Position::parse_move` does not check piece geometry or legality.** It infers the move kind from the board but accepts, for example, a rook moving like a knight as a quiet move. | Safe for trusted input such as the test games. Unsafe for untrusted input: a nonsensical move can corrupt the game state or make `make_move` panic. | Milestone 5: UCI move input is matched against the legal move list, so only legal moves reach `make_move`. |
| KI-4 | **Make/unmake design is unmeasured.** `make_move` returns an `Undo` token and updates the position in place. This was chosen by reasoning, with no benchmark against copy-make. | None known. The choice may not be the fastest. | Milestone 2: compare with copy-make using perft node rates, record the numbers in `TESTING.md`, and keep or change the design on that evidence. |
| KI-5 | **FEN validation does not detect that the side not to move is in check.** Such a position cannot arise in a game. | An impossible position loads without error. | Milestone 2, once attack generation exists. |
| KI-6 | **No committed randomised make/unmake stress test.** The milestone 1 review ran one outside the repository. | Regressions are caught only by the fixed test cases. | Milestone 2: the perft suite fills this role. |

## Milestone 2: move generation

Scope: attack tables (magic bitboards for sliders, found with the seeded
RNG), legal move generation, and perft against the published reference node
counts. Move generation is not done until every perft test passes. Also due
here: known issues KI-1, KI-4, KI-5 and KI-6.

Approach: generate pseudo-legal moves per piece type, then filter out the
ones that leave the mover's king attacked by making the move and testing.
This is the simplest correct design; a faster pin-aware generator can replace
it later behind the same interface, with perft as the safety net.

- [x] `attacks`: knight attack table
- [x] `attacks`: king attack table
- [x] `attacks`: pawn attack tables
- [ ] `attacks`: slider attacks by ray scanning (the reference implementation)
- [ ] `attacks`: relevant-occupancy masks and subset enumeration
- [ ] `attacks`: magic numbers found with the seeded RNG, bishop and rook tables
- [ ] `board`: attackers of a square and in-check detection
- [ ] `movegen`: `MoveList`
- [ ] `movegen`: pseudo-legal knight moves
- [ ] `movegen`: pseudo-legal king moves (without castling)
- [ ] `movegen`: pseudo-legal bishop, rook and queen moves
- [ ] `movegen`: pawn pushes and double pushes
- [ ] `movegen`: pawn captures
- [ ] `movegen`: promotions
- [ ] `movegen`: en passant
- [ ] `movegen`: castling
- [ ] `movegen`: legality filter and `legal_moves`
- [ ] `perft`: node counting with divide
- [ ] `perft`: `perft` subcommand of the binary
- [ ] perft test: start position
- [ ] perft test: Kiwipete (position 2)
- [ ] perft test: position 3
- [ ] perft test: position 4 and its mirror
- [ ] perft test: position 5
- [ ] perft test: position 6
- [ ] KI-6: perft-driven consistency check of make/unmake and hashing
- [ ] KI-5: FEN rejects positions where the side not to move is in check
- [ ] KI-1: hash the en passant file only when the capture is legal
- [ ] KI-4: measure make/unmake against copy-make with perft
- [ ] End-of-milestone review and doc drift pass

## Milestones 3 to 9

Scope summaries live in the "Build order" section of
[AGENTS.md](../AGENTS.md). Each milestone's chunk checklist is added here when
that milestone starts, and must include the known issues due in it:

- Milestone 2: KI-1, KI-4, KI-5, KI-6 (in its checklist above).
- Milestone 5: KI-3, plus the `UCI_LimitStrength` and `UCI_Elo` options
  (see "Planned features").
- Milestone 6: calibration of `UCI_Elo`.
- Not yet scheduled: KI-2.

## Planned features

Committed features that are not a milestone of their own. Each is scheduled
into the milestones named here and added to their chunk checklists.

### Strength limiting with the standard UCI options

Let a GUI or `lichess-bot` make Vyvræl play at a chosen, weaker rating using
the two options defined by the UCI protocol:

- `UCI_LimitStrength` (check, default `false`): when `true`, the engine
  plays at the rating in `UCI_Elo` instead of full strength.
- `UCI_Elo` (spin): the target rating. Its minimum, maximum and default are
  set from measured strength, not guessed.

Plan:

- **Milestone 5 (UCI):** expose both options and wire them to the weakening
  mechanism. One option per chunk, as for the other options.
- **Milestone 6 (strength testing):** calibrate. Play the limited engine at
  several `UCI_Elo` settings against opponents of known rating with the match
  script, fit the mapping from setting to measured Elo, and record the method,
  the opponents and the results in `TESTING.md`. Until this is done the
  options are documented as uncalibrated.

Constraints that already apply:

- Weakening is **strength-affecting randomness** (see the randomness rules in
  [AGENTS.md](../AGENTS.md)). It draws from the `rng` module, is seeded from
  system entropy by default, and is reproducible with `DeterministicRNG` and
  `RngSeed`. It gets its own row in the randomness table in
  [ARCHITECTURE.md](ARCHITECTURE.md).
- With `UCI_LimitStrength = false` (the default) play must be unchanged: no
  node, time or Elo difference. This is checked with a fixed-depth node-count
  comparison.
- The weakening mechanism (for example limiting nodes or depth, or choosing
  among near-best moves with some probability) is chosen in milestone 5 and
  recorded as a design decision. Weak play should still look like chess:
  plausible mistakes rather than random moves.

Decided: there will be no non-standard `Skill Level` option. Strength is
limited only through the two standard options above.

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
