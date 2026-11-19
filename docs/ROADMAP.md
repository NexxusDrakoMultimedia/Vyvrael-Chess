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
| 2 | Move generation | `milestone/2-movegen` | done, merged into `main` |
| 3 | Evaluation (hand-crafted) | `milestone/3-eval` | done, merged into `main` |
| 4 | Search core (negamax, alpha-beta, iterative deepening, quiescence) | `milestone/4-search` | not started |
| 5 | UCI | `milestone/5-uci` | not started |
| 6 | Strength testing | `milestone/6-strength` | not started |
| 4b | Search heuristics (move ordering, transposition table, null move, late move reductions), each accepted by SPRT | `milestone/4b-search-heuristics` | not started; follows milestone 6 |
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
| KI-2 | **FEN parsing rejects inconsistent positions and has no repair mode.** A castling right without its king and rook at home, or an en passant square that does not match a double push, is an error. | Sloppy FENs from other tools (stale castling rights are the usual case) fail to load instead of being corrected. | Later, milestone not fixed yet: an opt-in "repair and warn" mode that drops the bad right or square and reports what it changed. Strict parsing stays the default. To be revisited by milestone 7 at the latest, when third-party data is first read. |
| KI-3 | **`Position::parse_move` does not check piece geometry or legality.** It infers the move kind from the board but accepts, for example, a rook moving like a knight as a quiet move. | Safe for trusted input such as the test games. Unsafe for untrusted input: a nonsensical move can corrupt the game state or make `make_move` panic. | Milestone 5: UCI move input is matched against the legal move list, so only legal moves reach `make_move`. |
| KI-8 | **The evaluation's strength is unmeasured.** The hand-crafted evaluation is tested for correctness (symmetry, material, phase arithmetic), but its values and tables have never been tested in play. | Unknown playing strength; the numbers are reasoned, not tuned. | Milestone 6: SPRT the evaluation's terms and tune them, once search (milestone 4) and the match script exist. |
| KI-7 | **The magic-number search runs at start-up.** The slider tables are built on first use by searching for magic numbers, which takes about 0.1 s in a release build and about 1.5 s in a debug build. | A short delay before the first slider lookup. No effect on play. | Milestone 5, before the UCI loop: embed the 128 numbers the search finds as constants and keep the search as a test proving the constants are what the seed produces. |

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
- [x] `attacks`: slider attacks by ray scanning (the reference implementation)
- [x] `attacks`: relevant-occupancy masks and subset enumeration
- [x] `attacks`: magic numbers found with the seeded RNG, bishop and rook tables
- [x] `board`: attackers of a square and in-check detection
- [x] `movegen`: `MoveList`
- [x] `movegen`: pseudo-legal knight moves
- [x] `movegen`: pseudo-legal king moves (without castling)
- [x] `movegen`: pseudo-legal bishop, rook and queen moves
- [x] `movegen`: pawn pushes and double pushes
- [x] `movegen`: pawn captures
- [x] `movegen`: promotions
- [x] `movegen`: en passant
- [x] `movegen`: castling
- [x] `movegen`: legality filter and `legal_moves`
- [x] `perft`: node counting with divide
- [x] `perft`: `perft` subcommand of the binary
- [x] perft test: start position
- [x] perft test: Kiwipete (position 2)
- [x] perft test: position 3
- [x] perft test: position 4 and its mirror
- [x] perft test: position 5
- [x] perft test: position 6
- [x] KI-6: perft-driven consistency check of make/unmake and hashing
- [x] KI-5: FEN rejects positions where the side not to move is in check
- [x] KI-1: hash the en passant file only when the capture is legal
- [x] KI-4: measure make/unmake against copy-make with perft
- [x] End-of-milestone review and doc drift pass

Added during the milestone: known issue KI-7 (start-up cost of the magic
search), and from the end-of-milestone review a FEN check against impossible
material (which could overflow the move list) and several strengthened tests.
The review also ran perft on further published positions beyond the six in
the test suite and found no wrong count.

## Milestone 3: evaluation (hand-crafted)

Scope: a static evaluation of a position in centipawns, material first and
then piece-square tables with tapered middlegame/endgame blending, behind an
interface that a neural network can implement later. The hand-crafted
evaluation stays as fallback and baseline.

Evaluation cannot be judged by SPRT yet: that needs search (milestone 4) and
the match script (milestone 6). Until then it is checked for correctness
only (symmetry, known material balances, phase arithmetic), and its strength
is recorded as unmeasured in known issue KI-8.

The piece-square tables are written for this project from general chess
principles (they are not copied from another engine), leaning towards the
project's style: active pieces, advanced pawns and pressure towards the
enemy king. They are a starting point to be tuned once SPRT exists.

- [x] `eval`: `Evaluator` interface and score conventions
- [x] `eval`: material evaluator
- [x] `eval`: game phase
- [x] `eval`: tapered scores (middlegame/endgame pairs and blending)
- [x] `eval`: piece-square table framework with pawn tables
- [x] `eval`: knight and bishop tables
- [x] `eval`: rook and queen tables
- [x] `eval`: king tables
- [x] `eval`: hand-crafted evaluator (material plus tables, tapered)
- [x] `cli`: `eval` subcommand with a breakdown
- [x] End-of-milestone review and doc drift pass

The end-of-milestone review found no defect in the evaluation code. It
strengthened the tests (the blended total is now pinned at phases between
the endpoints) and corrected two comments that overstated what the tables do.

## Milestones 4 to 9

Scope summaries live in the "Build order" section of
[AGENTS.md](../AGENTS.md). Each milestone's chunk checklist is added here when
that milestone starts, and must include the known issues due in it:

- Milestone 2: none left.
- Milestone 5: KI-3, KI-7, plus the `UCI_LimitStrength` and `UCI_Elo` options
  (see "Planned features").
- Milestone 6: calibration of `UCI_Elo`; KI-8.
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

## Decisions for milestones 4 to 6

Decided by the maintainer on 2026-11-20.

- **Search is split in two so that heuristics can be proven.** The project
  rule is that a search change is accepted only if SPRT shows a gain, but
  SPRT needs the match script (milestone 6), which needs UCI (milestone 5).
  So milestone 4 builds only the core that everything else depends on:
  negamax with alpha-beta, iterative deepening and quiescence search. UCI and
  the match script follow. Milestone 4b then adds move ordering (hash move,
  MVV-LVA, killers, history), the transposition table, null-move pruning and
  late move reductions, one feature per commit, each with a recorded SPRT
  result. The core has nothing to be compared against, so it is checked by
  correctness tests (mates found, known node counts at fixed depth).
- **`Evaluator` gets `Send + Sync` bounds** as the first chunk of milestone 4,
  so the evaluator can be handed to the search thread. A later network
  evaluator must therefore keep any mutable per-search state outside the
  trait object.
- **Make/unmake or copy-make is chosen by measuring search.** Milestone 4
  starts with make/unmake, then measures both styles in the real search loop
  and keeps the faster. Perft showed copy-make about 5% faster (see
  TESTING.md), which is not decisive on its own.
- **KI-7 is fixed in milestone 5**, before the UCI loop, so the engine answers
  `isready` without a start-up delay.
- **Milestone 6 uses Cute Chess and Ordo** (the maintainer's choice, also
  2026-11-20). `cutechess-cli` plays the matches and runs SPRT; Ordo turns
  the resulting PGN into a rating list with error bars, which is also what
  the `UCI_Elo` calibration needs. Both are external programs the scripts
  call: they are not linked into the engine and are not Cargo dependencies.
  At the start of milestone 6, before first use, confirm each tool's licence
  on its official source and record it, with the version used, in
  [DEPENDENCIES.md](DEPENDENCIES.md). Match output (PGN, logs) is not
  committed.

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
