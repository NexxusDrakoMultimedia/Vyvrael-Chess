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
| 4 | Search core (negamax, alpha-beta, iterative deepening, quiescence) | `milestone/4-search` | done, merged into `main` |
| 5 | UCI | `milestone/5-uci` | done, merged into `main` |
| 6 | Strength testing | `milestone/6-strength-testing` | done, merged into `main` |
| - | Preparation before milestone 7: diagnostics, a suite runner and small UCI features (the [A] items of "Extra features and QoL") | `task/pre-7-tooling-and-qol` | done, merged into `main`, but for the second half of KI-12, which waits on its side branch for the maintainer's matches |
| 7 | Search heuristics (move ordering, transposition table, null move, late move reductions), each accepted by SPRT | `milestone/7-search-heuristics` | in progress (started 2026-11-27) |
| - | Extra features with and after milestone 7 (the [B] items) | within milestone 7, or a branch each | not started |
| - | Larger extra features (the [C] items), asked about first | a branch each | not started |
| 8 | Open data tooling | `milestone/8-data` | not started |
| 9 | Neural evaluation (NNUE) | `milestone/9-nnue` | not started |
| 10 | Endgame tablebases (optional, needs approval) | `milestone/10-tablebases` | not started |

## Order of work

Decided by the maintainer on 2026-11-21. The milestones keep their numbers,
but two tasks and the tooling are done between them, so the order in which
work is actually done is this:

1. **The tooling branch** (`tooling/testing-scripts`): the match and rating
   scripts and the installer. See "Testing tooling (before milestone 6)".
   Done on 2026-11-21.
2. **Queued task 1:** the options `Seed` and `Variety` and the `bench`
   command. See "Queued work". Done on 2026-11-22.
3. **Milestone 6:** strength testing. Done on 2026-11-23.
4. **The preparation branch** (`task/pre-7-tooling-and-qol`, added by the
   maintainer on 2026-11-27; done on 2026-11-27 but for KI-12's second
   half): the small items of "Extra features and QoL"
   that need no SPRT, so that milestone 7 has diagnostics and a quick test
   runner to work with.
5. **Milestone 7:** search heuristics.
6. **Queued task 2:** benchmark and speed up.
7. **The rest of the extra features**, each on its own branch, in the order
   the maintainer confirms.
8. **Milestone 8 onwards**, in numerical order.

**Where things run** (the maintainer, 2026-11-21): strength is measured on
the Windows workstation, where Ladder Baselines 1 and 2 and the results of
0.1.0-alpha.1 were played, so that later measurements can be compared with
them. lichess-bot runs on the Linux machine only. The scripts and the
installer still work on both systems.

Where the releases fall: **0.1.0-alpha.1** came before step 1, after
milestone 5; **0.1.0-alpha.2** follows step 2; **0.1.0** follows step 3;
**0.2.0** follows step 5. The
casual-only Lichess bot starts once the maintainer has tagged
0.1.0-alpha.1, so it runs alongside all of the steps above; rated play
waits for 0.2.0 and the branding.

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
is due to be fixed. An entry is removed in the commit that fixes it. Where
the design behind an issue is explained elsewhere, it is in
[ARCHITECTURE.md](ARCHITECTURE.md) (KI-2, KI-8), [UCI.md](UCI.md) (KI-10) or
[TESTING.md](TESTING.md) (KI-8, KI-9, KI-11, KI-12, KI-13).

| # | Issue | Effect today | Fix planned |
| --- | --- | --- | --- |
| KI-2 | **FEN parsing rejects inconsistent positions and has no repair mode.** A castling right without its king and rook at home, or an en passant square that does not match a double push, is an error. | Sloppy FENs from other tools (stale castling rights are the usual case) fail to load instead of being corrected. | Later, milestone not fixed yet: an opt-in "repair and warn" mode that drops the bad right or square and reports what it changed. Strict parsing stays the default. To be revisited by milestone 8 at the latest, when third-party data is first read. |
| KI-10 | **The first root move of the first depth cannot be interrupted, and neither can the weakened search's first pass.** The search ignores the clock and `stop` until one root move of depth 1 has been searched, so that there is a move to play. That move includes a quiescence search, which in a position with very many captures takes long: about 0.3 s in an artificial position with nine queens a side. (The whole first depth takes about 5 s there; since milestone 7 `stop` and the clock cut it short after the first move.) With `UCI_LimitStrength` on, the first pass over all the root moves still always completes: over two minutes in that position. | In such positions the engine overruns a short time budget and does not obey `stop` or `quit`, or answer anything else, meanwhile. In ordinary positions depth 1 takes well under a millisecond. | Not planned. Pruning in quiescence search shrinks it (delta pruning is parked in milestone 7); the weakened search's first pass could be given the ordinary search's rule. |
| KI-9 | **Search is slow for lack of move ordering.** Narrowed in milestone 7: the main search now tries captures first (Kiwipete at depth 3 takes 46,307 nodes where it took 4.5 million), and has a transposition table whose move is tried first, but the other quiet moves are still tried in generation order, so from a quiet position only a little over half of the cutoffs come from the first move (`vyvrael search --stats`). | The engine plays, but shallowly for the time it uses. Results are correct. | Milestone 7: move ordering, the transposition table and the other heuristics, each measured by SPRT. |
| KI-8 | **The evaluation is untuned, and its strength is measured for one build only.** The hand-crafted evaluation is tested for correctness (symmetry, material, phase arithmetic); its values and tables are reasoned, not tuned. Milestone 6 measured 0.1.0-alpha.2 on Ladder Baseline 2 (full-strength play did not change from then until milestone 7 began: the bench counted 16,611,416 nodes throughout): between Stockfish 19-1800 and Rodent IV-2000, level with 0.1.0-alpha.1 within the noise ([results-0.1.0-alpha.2.md](testing/results-0.1.0-alpha.2.md)). The same search was measured again for the 0.1.0 release, as an `avx2` build with other openings: level with Rodent IV-2000 and 0.1.0-alpha.2, 56 points lower on the scale, which is the spread between such runs ([results-0.1.0.md](testing/results-0.1.0.md)). | The measurement holds for that build only: any change to search or evaluation makes it out of date. **It is out of date now:** milestone 7 has begun to change the search. | Measured again after milestone 7 and after queued task 2. Tuning the terms by SPRT follows from milestone 7 on. |
| KI-11 | **The installer does not repair a half-made virtual environment for lichess-bot.** If the first run fails while creating it (seen on the Linux machine when Python's `venv` package was missing), the folder is left without `pip`, and later runs reuse it and fail with "No module named pip". | Installing lichess-bot fails again and again until the folder is removed by hand: delete `venv` inside the lichess-bot folder under the install prefix and run the installer again. Nothing else is lost; `config.yml` is kept elsewhere. | Not scheduled: the maintainer declined a fix for now (2026-11-21). The fix would be to check that `pip` works in an existing environment and recreate it if not. |
| KI-12 | **A long game without an increment can run the clock out.** The time for a move is `remaining / 30 + 3/4 increment`, capped at the clock less `Move Overhead`, with a floor of one millisecond. With no increment the engine kept spending at least a millisecond a move however little was left, so a long enough game at a tiny time control lost on time. **Narrowed on the preparation branch:** a clock within `Move Overhead` now gets no time, so the plan no longer spends the reserve; what the plan cannot see (the first depth, the GUI) still comes off the clock. | Seen once in the soak test (TESTING.md): 1 of 204 games at 1+0 seconds, a 124-move game. None at 3+2 or 60+0 seconds, and none in games with an increment. The Lichess bot's games all have an increment of at least two seconds. | The preparation branch before milestone 7: next, the overhead is taken off the clock before it is shared out; the entry goes once runs at 1+0 and 0.1+0 on the Windows workstation show the effect. |
| KI-13 | **A failed rating leaves Ordo's empty output files behind.** When Ordo fails (for example on games that are not connected), `rate.ps1` and `rate.sh` stop with Ordo's status, but the text and CSV files Ordo had already created stay in place, empty. | A stray pair of empty `-ratings.txt` and `.csv` files next to the PGNs after a failed rating; the next run with the same `-Out` overwrites them. Seen in the smoke test (TESTING.md). | Not scheduled: the maintainer decides. A fix would remove the two files when Ordo fails, if the script created them. |

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
the match scripts. Until then it is checked for correctness
only (symmetry, known material balances, phase arithmetic), and its strength
was recorded as unmeasured in known issue KI-8 (first measured in
milestone 6).

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

## Milestone 4: search core

Scope: the part of search that everything else depends on. Negamax with
alpha-beta pruning, iterative deepening and quiescence search, plus what a
search needs to be correct in a real game: mate scores, draw detection, a
principal variation, and a way to stop. Move ordering, the transposition
table, null-move pruning and late move reductions are milestone 7, where
each can be accepted by SPRT.

There is no earlier search to compare against, so this milestone is checked
by correctness, not strength: forced mates are found at the right depth,
alpha-beta returns exactly the score of a plain minimax search, draws are
recognised, and fixed-depth node counts are pinned so that any change in
behaviour is noticed.

- [x] `eval`: `Send + Sync` bounds on `Evaluator`
- [x] `search`: score constants and mate-score arithmetic
- [x] `search`: negamax with alpha-beta at fixed depth
- [x] `search`: draw detection (fifty-move rule, repetition, bare material)
- [x] `search`: principal variation
- [x] `search`: limits and stopping (depth, nodes, time, stop flag)
- [x] `search`: iterative deepening with progress reports
- [x] `search`: quiescence search, with captures ordered most valuable victim
  first
- [x] `cli`: `search` subcommand
- [x] `search`: pinned fixed-depth node counts
- [x] `search`: measure make/unmake against copy-make and choose
- [x] End-of-milestone review and doc drift pass

## Milestone 5: UCI

Chunk checklist. One commit per item unless noted.

- [x] `attacks`: embed the magic numbers as constants, with the search kept
  as a test (fixes KI-7)
- [x] `movegen`: find a move given as text among the legal moves (fixes KI-3)
- [x] `uci`: command parsing (`uci`, `isready`, `ucinewgame`, `position`,
  `go`, `stop`, `setoption`, `quit`)
- [x] `uci`: output formatting (`info`, scores, `bestmove`)
- [x] `uci`: time management (from `wtime`, `btime`, increments,
  `movestogo`, `movetime` to a deadline)
- [x] `uci`: the engine session and protocol loop, with the search on its
  own thread; the binary runs it when started without arguments
- [x] `uci`: option `Hash`
- [x] `uci`: option `Threads`
- [x] `rng`: a seed from system entropy and the stream for
  strength-affecting randomness
- [x] `uci`: options `DeterministicRNG` and `RngSeed`
- [x] `search`: weakened move choice for limited strength
- [x] `uci`: option `UCI_LimitStrength`
- [x] `uci`: option `UCI_Elo` (uncalibrated until milestone 6)
- [x] Integration test: drive the binary over stdin and stdout
- [x] End-of-milestone review and doc drift pass

Design decisions taken at the start of the milestone:

- **Illegal input never reaches `make_move`.** A move in a `position`
  command is looked up among the legal moves. If one is not found the whole
  command is refused with an `info string` and the previous position stays.
- **One search thread.** The loop reads commands on the main thread and the
  search runs on a second one, so `stop`, `isready` and `quit` are answered
  while it thinks. `Threads` is offered with a minimum and maximum of 1 until
  a parallel search exists.
- **`Hash` is accepted and stored but has no effect yet.** There is no
  transposition table before milestone 7. The option exists now because
  match tools set it.
- **No pondering, `searchmoves` or `go mate`.** They are not offered;
  unknown `go` words are ignored, as the protocol asks.
- **Weakening (for `UCI_LimitStrength`)**: search every root move to a
  shallow depth that grows with `UCI_Elo`, add random noise that shrinks with
  it to each move's score, and play the best noisy score. Mistakes are then
  moves that looked nearly as good, not random ones. The numbers are
  provisional until calibration in milestone 6.

## Milestones 6 to 10

Scope summaries live in the "Build order" section of
[AGENTS.md](../AGENTS.md). Each milestone's chunk checklist is added here when
that milestone starts, and must include the known issues due in it:

- Milestone 6: calibration of `UCI_Elo` (KI-8 was measured for
  0.1.0-alpha.2).
- Milestone 7: KI-9, KI-10; KI-8 measured again.
- The preparation branch before milestone 7: KI-12.
- Not yet scheduled: KI-2, KI-11, KI-13.

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

- **Milestone 5 (UCI), done:** both options exist and are wired to the
  weakening mechanism (`search::weakened_move`), uncalibrated.
- **Milestone 6 (strength testing), done (provisional):** calibrated. The
  limited engine played at several `UCI_Elo` settings against opponents of
  known rating, the mapping from setting to measured Elo was fitted, and
  `UCI_Elo` now runs from 1110 to 1630 with a default of 1360. The method,
  the opponents and the results are in `TESTING.md` ("Calibration of
  `UCI_Elo`"). The calibration holds for the build measured, on the
  ladder's scale.

Constraints that already apply:

- Weakening is **strength-affecting randomness** (see the randomness rules in
  [AGENTS.md](../AGENTS.md)). It draws from the `rng` module, is seeded from
  system entropy by default, and is reproducible with a fixed `Seed`. It
  gets its own row in the randomness table in
  [ARCHITECTURE.md](ARCHITECTURE.md).
- With `UCI_LimitStrength = false` (the default) play must be unchanged: no
  node, time or Elo difference. This is checked with a fixed-depth node-count
  comparison.
- The weakening mechanism was chosen in milestone 5: search every root move
  to a shallow depth, misjudge each score by a bounded random amount, play
  the best. It is described under "Playing below full strength" in
  [ARCHITECTURE.md](ARCHITECTURE.md). Weak play should still look like
  chess: plausible mistakes rather than random moves.

**The calibration is re-measured.** `UCI_Elo` and the engine's Ordo rating
are measured again after milestone 7 and after queued task 2, because both
change the engine's strength. The calibration of 0.1.0 is provisional until
then, and the release notes of 0.2.0 say so.

**What scale `UCI_Elo` is on.** The calibration maps `UCI_Elo` to the scale
of the reference ladder, Ladder Baseline 2 (current; Ladder Baseline 1 is
kept as history), and its Ordo anchor (Stockfish 19 = 0), shifted by a
fixed 3,500 points so that every setting is positive (Stockfish 19 =
3,500; the maintainer's decision of 2026-11-22, worded in `TESTING.md`,
"The scale of `UCI_Elo`"), not to Lichess, CCRL or FIDE ratings. That scale is stretched by lopsided results,
most visibly at the low end: the limited-strength engines of the baseline
rate far below the numbers in their names. So documentation and the bot's profile must say which scale a number
refers to, and must not state a Lichess rating that the measurements do not
support.

Decided: there will be no non-standard `Skill Level` option. Strength is
limited only through the two standard options above.

## Releases

Decided by the maintainer on 2026-11-20. Versions follow Semantic
Versioning; before 1.0.0 a minor version may change anything. Tags are
annotated, `vX.Y.Z`, and made by the maintainer only.

| Version | When | What it needs |
| --- | --- | --- |
| 0.1.0-alpha.1 | after milestone 5 | UCI conformance tests; complete games in `cutechess-cli` at 10+0.1 with no illegal move, crash, stall or time loss; KI-3 and KI-7 fixed; `Move Overhead`; `docs/LICHESS.md`. Strength unmeasured. **Released 2026-11-21.** |
| 0.1.0-alpha.2 | after queued task 1 | The options `Seed` and `Variety` and the `bench` command, with their tests and docs; the three gates green; complete games in `cutechess-cli` as for alpha.1. Still an alpha: `UCI_Elo` is not calibrated, and the only measurement is the snapshot of alpha.1, which its notes may cite as such because full-strength play is unchanged. Plain builds made by hand. **Released 2026-11-22.** |
| 0.1.0 | after milestone 6 | The scripts documented and smoke-tested; `UCI_Elo` calibrated; KI-8 measured for the build released; a measured Ordo rating against the ladder in the release notes. The calibration and the rating are provisional: they are for the build measured. The first release with binaries built by a workflow, in the `avx2` primary build and the plain build (see "Release artifacts" and [RELEASING.md](RELEASING.md)). **Released 2026-11-23**, built by the release workflow ([results-0.1.0.md](testing/results-0.1.0.md)). |
| 0.2.0 | after milestone 7 | The search heuristics, each accepted by SPRT. The first release with the optional `pext` build beside `avx2` and plain (see "Release artifacts"). `UCI_Elo` and the Ordo rating measured again, and release notes that say the 0.1.0 calibration was provisional. They are measured once more after queued task 2. |

Later minor versions follow milestones 8, 9 and 10. When 1.0.0 comes, and
whether there are further alphas, is the maintainer's call.

### Release artifacts

Decided by the maintainer (all but the first point on 2026-11-21):

- A release ships binaries for Windows and Linux on x86-64, each with a
  SHA-256 checksum file beside it. 0.1.0-alpha.1 did.
- **Three builds for each system:**

  | Build | Windows file | What it is | Standing |
  | --- | --- | --- | --- |
  | `avx2` | `vyvrael-<version>-avx2.exe` | Built for the `x86-64-v3` level: AVX2 and BMI2, on Intel from Haswell (2013) and AMD from Excavator and Zen. Sliding-piece attacks by magic bitboards, which are fast on all of those. | **The primary build**, and the least the project supports and tests. |
  | `pext` | `vyvrael-<version>-pext.exe` | The same level, with sliding-piece attacks by the BMI2 instruction PEXT instead. Faster where PEXT is fast (Intel, and AMD from Zen 3); slow on AMD Zen 1 and Zen 2, where PEXT is slow. | Optional. |
  | plain | `vyvrael-<version>.exe` | A build for any x86-64 processor, for machines older than the primary build runs on. | Optional, for backward compatibility; not the supported minimum. |

  None is ever built with `target-cpu=native`, and there is no AVX-512
  build.
- **`avx2` is the bare minimum** the project targets: the build to measure,
  to test with and to recommend. The plain build is a courtesy and gets no
  measurements of its own.
- **File names:** `vyvrael-<version>-<build>.exe` for Windows and
  `vyvrael-<version>-<build>` for Linux, where `<build>` is `avx2` or
  `pext`. The plain build has no build word: `vyvrael-<version>.exe` and
  `vyvrael-<version>`. The names are short on purpose; the table above says
  what each means.
- **Every release says which file to take.** Because the file without a
  build word looks like the default download and is not, the release notes
  state near the top that `avx2` is the modern, preferred build, that `pext`
  is for processors where PEXT is fast, and that the plain file is only for
  older machines.
- The files of 0.1.0-alpha.1 were named before this was decided and keep
  their names (`...-windows-x86_64.exe`, `...-linux-x86_64`); they are plain
  builds. 0.1.0-alpha.2 shipped plain builds under the names above.
- The binaries are built by a workflow, not by hand. The maintainer owns
  that workflow, because `.github/workflows` is a protected path that agents
  do not edit: an agent may propose its contents in a document for the
  maintainer to put in place. The workflow was installed on 2026-11-22 and
  passed a trial on the test tag `v0.1.0-rc.0`; see
  [RELEASING.md](RELEASING.md).
- **0.1.0 is the first release with workflow-built binaries**, in the
  `avx2` and plain builds; **the `pext` build joins in 0.2.0**, when
  milestone 7 adds sliding-piece attacks by PEXT. (Amended by the
  maintainer on 2026-11-21; first planned as "0.2.0 for all of it".)
  0.1.0-alpha.1 and 0.1.0-alpha.2 were the last releases with plain builds
  made by hand. The procedure is in [RELEASING.md](RELEASING.md).
- Agents prepare a release; the maintainer tags and publishes it.
- Binaries are attached to the release and never committed.

What the builds depend on:

- The `pext` build needs sliding-piece attacks by PEXT in the
  engine, which is to be added in milestone 7 behind a feature, with magic
  bitboards staying the default and perft required to match on both paths.
  Until that exists there is no such build.
- The benchmark of queued task 2 measures the three builds against each
  other on both of the maintainer's machines.

## Testing tooling (before milestone 6)

Decided by the maintainer on 2026-11-20. The match and rating scripts and
the installer work were built on a branch of their own,
`tooling/testing-scripts`, before milestone 6 uses them. **Done: reviewed
and merged into `main` on 2026-11-21.**

- **Scripts, one set per system, same behaviour and flags:**
  `scripts/windows/match.ps1` and `scripts/windows/rate.ps1` (PowerShell),
  `scripts/linux/match.sh` and `scripts/linux/rate.sh` (bash).
- **Match modes:** `sprt` (a test build against a base build), `gauntlet`
  (the build under test against opponents from the ladder) and `h2h` (two
  Vyvræl builds, a fixed number of rounds).
- **Fixed defaults, those of Ladder Baseline 1:** 10+0.1, Hash 64, one
  thread, pondering off, openings from `UHO_Lichess_4852_v1.epd` in random
  order with each played twice, colours swapped; resign after 3 moves at 700
  centipawns, two-sided; draw from move 34 after 10 moves within 20
  centipawns; `-recover`.
- **SPRT bounds, alpha, beta and a round cap are required arguments.** There
  are no silent defaults; `TESTING.md` gives example bounds.
- **No named machine profiles** (the maintainer, 2026-11-21): no machine
  name appears in a script or a flag. Defaults for concurrency, priority and
  tool locations come from a git-ignored local file, of which the
  repository carries an example, and every one can be overridden by an
  argument. Concurrency, confirmed by the maintainer: 6 on the Windows
  workstation, 3 at low priority on the Linux machine, never more than the
  physical cores.
- **No personal paths** in scripts or docs: tools come from `PATH`,
  arguments, environment variables or a git-ignored local file.
- **Installer:** it also installs Linux builds of the ladder opponents and
  both tools on both systems. This replaces the earlier choice of a
  Linux-only installer. Vyvræl itself is not part of the installer.

Chunk checklist (branch `tooling/testing-scripts`, started 2026-11-21). One
commit per item unless noted; tests and docs travel with each.

Part B, the match and rating scripts:

- [x] Local settings: a git-ignored file read by all four scripts, with a
  committed example
- [x] `scripts/linux/match.sh`: arguments, safety checks and `--dry-run`
  for the modes `sprt`, `gauntlet` and `h2h`
- [x] `scripts/linux/match.sh`: run notes (command, names, SHA-256s, tool
  versions, times) and the progress log
- [x] `scripts/linux/rate.sh`: Ordo with an anchor, one or more PGN files
- [x] `scripts/windows/match.ps1`: the same behaviour and flags
- [x] `scripts/windows/rate.ps1`: the same behaviour and flags
- [x] `docs/TESTING.md`: running the scripts, the fixed defaults, example
  SPRT bounds, pitfalls

Part A, the installer:

- [x] The ladder's opponent engines as pins: name, version, official source,
  method, SHA-256, licence
- [x] Install the Linux engines: download or build, `--only`, idempotent
- [x] Write the `engines.json` fragment with the baseline's exact names and
  options
- [x] Verify each installed engine (`uci`, `isready`, `go depth 6`) with
  timeouts
- [x] Install Cute Chess and Ordo on Windows from the official releases
- [x] `docs/TESTING.md` and `docs/DEPENDENCIES.md`: the engine table and the
  Linux and Windows instructions

When this was merged, no agent had run it against a real ladder engine, a
real match or the real Ordo. **The maintainer has since run the tooling on
the Windows workstation with 0.1.0-a1 and confirmed that it works**
(2026-11-21). Which scripts and modes that covered is not recorded. On the
Linux machine the engine installer has been run once (2026-11-21): 12 of
the then 14 engines installed and 21 names passed the check; see "The
ladder's opponent engines" in `docs/TESTING.md` for the three that did
not. One of them, Ethereal 12.75, has since been taken out of the
installer; the other two, Weiss and Princhess, now install and work. The
bash match and rating scripts have made a real run there too: a gauntlet
with `match.sh` and a rating with `rate.sh` (2026-11-21).

## Recording results

SPRT results are not in the files described here: each test is a row of
[`testing/sprt-history.md`](testing/sprt-history.md), appended by
`scripts/record_sprt.py` (TESTING.md, "Recording an SPRT result").

Every measured build gets one file under `docs/testing/`, named after the
build (for example `results-0.1.0-alpha.1.md`), with:

- the build's name as it appears in the games, and its commit;
- the opponents, the number of games and the scores;
- the Ordo rating and its error, with the anchor it is relative to;
- the settings, where they differ from the fixed ones;
- the baseline's caveats: small samples, and a scale stretched by lopsided
  results.

Ordo's text and CSV output may sit beside it. PGN files, logs and run notes
stay out of git.

Results recorded so far:

- [`results-0.1.0-alpha.1.md`](testing/results-0.1.0-alpha.1.md): the first
  alpha against the lower rungs of the ladder, a snapshot from before
  milestone 6.
- [`results-0.1.0-alpha.2.md`](testing/results-0.1.0-alpha.2.md): the
  milestone 6 measurement (KI-8), 1,000 games against Ladder Baseline 2's
  rungs, rated with Baseline 2.
- [`results-0.1.0.md`](testing/results-0.1.0.md): the measurement for the
  0.1.0 release, an `avx2` build of its source, against the same rungs
  (the same search as alpha.2, measured again with other openings).
- The `UCI_Elo` calibration and its check are recorded in TESTING.md
  ("Calibration of `UCI_Elo`") rather than in a results file, with Ordo's
  output in
  [`calibration-0.1.0-alpha.2-ratings.txt`](testing/calibration-0.1.0-alpha.2-ratings.txt)
  and
  [`calibration-check-a874077-ratings.txt`](testing/calibration-check-a874077-ratings.txt).

## Queued work (requested 2026-11-21)

Two tasks asked for by the maintainer. Each gets a branch of its own from
`main` and small commits. Neither task bumps the version, tags or releases
by itself; the first is followed by the release of 0.1.0-alpha.2, prepared
as a separate step (see "Releases"). The first
follows the tooling branch, which is merged, so it is next; the second
waits until milestone 7 is done (the maintainer's decision of 2026-11-21).
See "Order of work". A third group of requests, of 2026-11-27, follows
them as section 3.

### 1. Optional determinism: `Seed`, `Variety`, `bench`

Why: self-play of 0.1.0-a1 repeats games. Investigated on 2026-11-21: at
full strength nothing is random (the search draws no random numbers, there
is no transposition table, and moves are tried in a fixed order), so the
same position searched to the same depth gives the same move. The only
time-dependence is which depth finishes before the deadline: the clock is
read every 1,024 nodes and an interrupted iteration is discarded. In the
release check's 20 self-play games, 3 of 10 opening pairs were identical
move for move and 7 diverged.

Decided by the maintainer:

- [x] **One option `Seed`** replaces `DeterministicRNG` and `RngSeed`: 0
  takes a seed from system entropy (not the clock; the rule against
  time-based seeding stands), anything else is fixed. Spin, 0 to
  2147483647: Cute Chess keeps spin values as signed 32-bit numbers, and
  with today's maximum of 4294967295 its GUI resets `RngSeed` to 0 whenever
  it is edited. Seeds taken from entropy stay in that range so any logged
  seed can be typed back. The seed is logged with `info string` at start
  and at every `ucinewgame`, as now.
- [x] **Option `Variety`** (spin, centipawns, 0 to 100, **default 0**). At
  the root only and only in the first 12 full moves, choose at random among
  the moves within `Variety` of the best, weighted towards the best. Never
  when the best score is a mate, and never a move that a second check shows
  to lose more than `Variety`. With 0 the engine plays exactly the best
  move, so matches and SPRT are untouched. It is exempt from the SPRT rule,
  like `UCI_LimitStrength`, because it weakens play on purpose; say so in
  `AGENTS.md`. The Lichess bot sets it (10) in `docs/LICHESS.md`.
- [x] **`vyvrael bench`**: a fixed set of positions to a fixed depth with
  `Seed` fixed and `Variety` 0, printing `<nodes> nodes <nps> nps`. The
  node count must be the same on every run.
- [x] Once `bench` exists, record the first bench node count and speed and
  the perft speed in `docs/testing/bench-history.md`, so that queued task 2
  has a baseline from before milestone 7.
- [x] Tests: bench twice gives one node count; a fixed-depth search with a
  fixed `Seed` and `Variety` 0 is reproducible; with `Variety` 0 the root
  move is always the best one; the seed line is printed (no test may depend
  on two entropy seeds differing).
- [x] Docs: the deterministic guarantee is one thread, a fixed `Seed` and
  `Variety` 0 (`docs/TESTING.md`, `docs/UCI.md`); the options in the README
  and `docs/LICHESS.md`; the randomness table in `docs/ARCHITECTURE.md`;
  the match scripts' `--deterministic`, which sets the old option; a
  changelog entry.
- [x] Report: what made the engine deterministic, the bench node count
  before and after, the options added.

Chunk order (branch `task/1-seed-variety-bench`, started 2026-11-22), one
commit each with its tests and docs:

1. `bench` first, so that its node count before `Seed` and `Variety` can
   be compared with the count after them: the two must be equal, because
   neither option may change the full-strength search.
2. `rng`: entropy seeds from 1 to 2147483647.
3. `uci`: the option `Seed` in place of `DeterministicRNG` and `RngSeed`,
   and no test that needs two entropy seeds to differ.
4. `search`: the choice of a varied move, on its own and tested.
5. `uci`: the option `Variety`, using it.
6. The match scripts: `--deterministic` / `-Deterministic` set `Seed`.
7. `AGENTS.md`: `Variety` is exempt from the SPRT rule.
8. Docs: README, `UCI.md`, `TESTING.md`, `LICHESS.md`, `ARCHITECTURE.md`'s
   randomness table, the changelog.
9. `docs/testing/bench-history.md`: the first bench and perft figures.

Design decisions taken for it, confirmed by the maintainer on 2026-11-22:

- **Entropy seeds avoid 0.** `Seed` 0 means "take one from entropy", so an
  entropy seed is drawn from 1 to 2147483647: any logged seed can be typed
  back as `Seed` and means the same thing.
- **The second check of `Variety`.** After the ordinary search reaches depth
  D, each other root move is first scored cheaply (quiescence only) and
  kept as a candidate if that is within `Variety` plus a margin of the best;
  each candidate is then searched exactly to the best move's depth (D) and
  kept only if that score is within `Variety` of the best move's. A move
  whose check is cut short by a limit is never played. The pick is weighted
  by the square of `Variety + 1 −` its loss, so the best move is the most
  likely and a move `Variety` worse the least.
- **`Variety` gets a share of the time.** When it applies, the ordinary
  search gets two thirds of the move's time or node budget and the checks
  the rest, so that a varied move has been checked to the same depth as the
  best one. With `go depth` there is no budget to share and the checks run
  to completion, which keeps a fixed-depth run reproducible. With `Variety`
  0 nothing changes: the search gets the whole budget, as now.

### 2. Benchmark and speed up (after milestone 7)

Why: in a 10+0.1 gauntlet 0.1.0-a1 reached only depth 3 to 6 (median 4) at
about 0.28 s a move. Measure before changing anything; do not assume the
cause. **It starts after milestone 7**, so the order is: tooling, task 1,
milestone 6, milestone 7, then this. Task 1 provides `bench`. By then move
ordering, the transposition table and the other search heuristics exist and
have each passed an SPRT, so this task measures and speeds up the search as
it will really be, not one that is slow mainly for lack of them.

- [ ] **Baseline, reported before any change:** perft speed on the
  reference positions; `bench` over 30 to 50 positions (opening,
  middlegame, endgame, tactical); the build settings in use (wanted:
  opt-level 3, `lto = "fat"`, one codegen unit, `panic = "abort"`; never
  `target-cpu=native` in committed configuration; the three release builds
  of "Release artifacts" compared, with how to make each described in the
  docs); a profile of `bench` on Linux
  with the ten hottest functions.
- [ ] **The usual causes, in this order, as the profile points:**
  allocation or cloning in the hot path; copy-make on a large position;
  move generation; evaluation recomputed at every node instead of updated;
  the transposition table (there is none yet); how often the clock is read;
  quiescence search; debug assertions or bounds checks left in release.
- [ ] **One change per commit, with bench nodes and speed before and
  after** in the message. A pure speed change keeps the bench node count
  identical; if the count changes it is a search change: stop and explain.
- [ ] **Search changes** (move ordering, pruning, reductions, evaluation)
  go in separate, clearly labelled commits and are not called stronger.
  The heuristics of milestone 7 are done by then; anything further found
  here follows the same rule, one feature per commit, each accepted only by
  an SPRT.
- [ ] `docs/testing/bench-history.md`: a row per commit (commit, change,
  nodes, speed, perft speed). Bench and perft stay fast enough for CI.
- [ ] Report: speed before and after, the hotspots and what changed for
  each, which commits are pure speed and which change the search, and what
  to SPRT next.

### 3. Extra features and QoL (requested 2026-11-27)

Asked for by the maintainer, in three groups. **[A]** is small, needs no
SPRT and is done before milestone 7 on the preparation branch below.
**[B]** is done with or after milestone 7 and may need an SPRT or careful
testing. **[C]** is larger: a branch each, asked about first.

**[A], the preparation branch:** the fix for KI-12; search statistics
behind a flag; an EPD test-suite runner; more fuzzing of the UCI parser;
`bench <depth> <threads> <hash>`; a script that records an SPRT result;
`seldepth` and `UCI_ShowWDL` in `info` (`nps` was there already); the
commands `d` and `eval` in UCI mode.

**[B], with and after milestone 7:**

- [x] A `Clear Hash` button option and `hashfull` in `info`, with the
  transposition table (milestone 7, item 2c).
- [ ] Further search features, each its own commit and its own SPRT, after
  the transposition table, MVV-LVA, killers and history, null move and
  late move reductions: principal variation search, aspiration windows,
  reverse futility pruning, check extensions, internal iterative
  reduction, countermove and continuation history. Their order is
  proposed when milestone 7 starts.
- [x] `go mate` and `searchmoves`: protocol features with no effect on
  strength, tested in `tests/uci.rs`, bench node count unchanged.
- [ ] Pondering (`Ponder`, `go ponder`, `ponderhit`), tested by the
  integration tests and by real games in `cutechess-cli`, then described
  for lichess-bot in `LICHESS.md`.
- [ ] A `tune` tool in `tools/` (SPSA or Texel style) for the piece-square
  tables and the evaluation's terms, king-attack terms among them, with
  fixed seeds, recorded parameters and a manifest. Each tuned set needs
  its SPRT. Where it fits (possibly after milestone 8, whose tooling
  provides the data) is the maintainer's to say.

**[C], larger work, asked about first:**

- [ ] Chess960 (`UCI_Chess960`), accepted by perft against the published
  Chess960 counts with standard perft unchanged.
- [ ] A short opening book for the Lichess bot through lichess-bot's
  polyglot support: documentation and configuration only, no book file
  committed.
- Rated play and submission to rating lists stay blocked on the branding.

#### Chunk checklist of the preparation branch

Branch `task/pre-7-tooling-and-qol`, started 2026-11-27 from `main`. One
commit per item unless noted; tests and docs travel with each. **Nothing
here may change what the search does at its default settings: the bench
counts 16,611,416 nodes after every commit.** CI does not run on pushes to
this branch (the workflow names `main` and `milestone/**`), so the three
gates and the script tests are run locally.

Decided by the maintainer on 2026-11-27: the statistics are counted always
and only shown behind a flag; `d` and `eval` answer in `info string`
lines, so the rule that stdout carries protocol output only has no
exception; `UCI_ShowWDL` uses a fixed, documented mapping that is a model
and not a measurement; SPRT results go into
`docs/testing/sprt-history.md`; the suite runner is a command of the
engine binary; `bench` refuses more than one thread.

- [x] This plan, and the `task/` branch prefix in `AGENTS.md`
- [x] Docs: fix the drift found on `main` while planning
- **KI-12** (the maintainer runs the matches on the Windows workstation):
  - [x] A test that plays out a clock without increment against the time
    formula and pins how it runs out today
  - [x] No time is planned once the clock is within `Move Overhead`: the
    move comes from the first depth alone
  - [ ] `Move Overhead` is taken off the clock before it is shared out
    (moves every budget by about a millisecond, so it needs a
    non-regression SPRT). **Written and green on the side branch
    `task/pre-7-ki12-overhead`, not merged:** it waits for its SPRT
    (bounds -5 and 0) on the Windows workstation, and is dropped if that
    does not accept it.
  - [ ] The runs at 1+0 and 0.1+0 before and after, recorded; KI-12
    removed. **Waits for the maintainer's runs.** Until then KI-12 stays
    in the table, narrowed.
- [x] `bench [depth] [threads] [hash]`
- **`d` and `eval` in UCI mode:**
  - [x] The evaluation's report built in the `eval` module
  - [x] A text diagram of a position
  - [x] The two commands
- **`info` fields:**
  - [x] The search records its selective depth
  - [x] `seldepth` in the `info` lines
  - [x] The score-to-WDL mapping
  - [x] The option `UCI_ShowWDL`
- **Search statistics:**
  - [x] Cutoffs and first-move cutoffs counted in the search
  - [x] `search --stats`: nodes per depth, branching factor, cutoff rates
  - [x] The option `SearchStats` and its `info string` line (the hit rate
    of the transposition table joins in milestone 7)
- **UCI fuzzing** (tests only; a defect found gets a fix commit of its own):
  - [x] A stream that hangs fails the test instead of stalling it
  - [x] Every `bestmove` of a malformed stream is legal
  - [x] The command, FEN, move and option parsers fuzzed directly
  - [x] Streams of mutated real sessions and long games
  - [x] The release binary survives a fuzzed stream over pipes
- **SPRT recorder:**
  - [x] `scripts/record_sprt.py` reads a run's notes and log
  - [x] It appends the result to `docs/testing/sprt-history.md`
  - [x] A test that it reads what the match scripts really write
- **EPD suite runner:**
  - [x] `*.epd` ignored by git
  - [x] A legal move found from its SAN text
  - [x] EPD records read
  - [x] Positions scored by `bm` and `am`
  - [x] The command `suite`
  - [x] `DATA.md`: the WAC and STS suites, confirmed on their official pages
- [x] End-of-branch doc drift pass, with an independent review of the
  branch: no defect in the search, the time management, the protocol or
  the fuzz tests; fixed from it, a byte-order mark at the start of a
  suite file, three gaps in the SPRT recorder and ten slips in the docs

## Milestone 6: strength testing (Cute Chess + Ordo)

Scope as amended by the maintainer on 2026-11-20 and 2026-11-21. The chunk
checklist below was written when the milestone started. Milestone 6 is the
measuring: gauntlets against the ladder, the calibration of `UCI_Elo`, and
known issue KI-8 (the evaluation's strength). The scripts and the installer
it uses are built beforehand and described under "Testing tooling (before
milestone 6)" above.

**Opponent ladder reference:** Ladder Baseline 2 (current); Ladder Baseline
1 is kept as history. Ladder Baseline 2,
[`docs/testing/ladder-Baseline-2.md`](testing/ladder-Baseline-2.md), is a
round robin of 30 other engines run by the maintainer on 2026-11-21; it
replaced Ladder Baseline 1,
[`docs/testing/ladder-Baseline-1.md`](testing/ladder-Baseline-1.md) (25
engines), against which 0.1.0-alpha.1 was measured. Vyvræl is in neither.
The reference ladder orders the opponents; Vyvræl's strength (KI-8) and
the `UCI_Elo` calibration are measured by gauntlets against its lower
rungs, in separate results files, with the same time control, hash,
threads and openings.

Matches are run with `cutechess-cli` (from Cute Chess) and ratings computed
with Ordo. Neither tool is vendored into the repository or linked into the
engine: they run as separate programs and are installed by the user.

- **`docs/TESTING.md` sections:** how to read an SPRT result; how to read
  Ordo output; what a sound Elo comparison needs (same machine, same
  settings, enough games); the limits of results from different machines.
- **Acceptance rule:** only accept search and evaluation changes that SPRT
  shows to be a gain, and record the result in the commit message or in
  `docs/TESTING.md`.

Taken into the chunk checklist below:

- Record in `docs/TESTING.md` and `docs/UCI.md` which scale `UCI_Elo` is
  on: the reference ladder, Ladder Baseline 2 (current; Ladder Baseline 1
  is kept as history), and its Ordo anchor, not Lichess, CCRL or FIDE;
  and that the scale is stretched by lopsided results, most visibly at the
  low end.
- Robustness, wanted before 0.2.0 because the bot plays in public:
  - a test that feeds the UCI loop malformed and unexpected input and
    asserts that it never panics. The release profile uses
    `panic = "abort"`, so a panic ends the engine in the middle of a game;
  - a soak test through `cutechess-cli` at several time controls (1+0, 3+2
    and 60+0) with no illegal move, crash, stall or time loss.

Notes for whoever starts it:

- `scripts/install_tools.py` installs pinned versions of Cute Chess, Ordo,
  the matches' opening file and the ladder's opponent engines on Linux and
  Windows, and lichess-bot on Linux; see TESTING.md. The maintainer has installed both tools with it on the Linux
  machine and used them for a real match and rating (2026-11-21).
- The tools' licences and pinned versions are recorded in
  [DEPENDENCIES.md](DEPENDENCIES.md). Re-check them if a pin changes.
- The match settings rely on the UCI options `Hash` and `Threads`.
  Repeatable limited-strength runs also need a fixed seed: the option
  `Seed`.
- Calibrating `UCI_Elo` needs limited Vyvræl entries: since milestone 6
  the match scripts give a build its own options (`--test-option`,
  `--base-option`), or the engine list can hold them.
- Match output (PGN, logs, rating files) is not committed; `.gitignore`
  already covers match output and `*.pgn`.

### Chunk checklist

Branch `milestone/6-strength-testing`, started 2026-11-22. One commit per
item unless noted; tests and docs travel with each. Part A is the agent's;
Part B is measuring, where the maintainer runs the long matches on the
Windows workstation and sends the files, and the agent writes the records.
Results from different machines are not compared.

Part A, documents, tools and tests:

- [x] `ROADMAP.md`: the release plan as amended by the maintainer
  (2026-11-21): 0.1.0, not 0.2.0, is the first release with binaries built
  by a workflow, in the `avx2` and plain builds; the `pext` build joins in
  0.2.0, when milestone 7 adds PEXT attacks. The "Releases" table and
  "Release artifacts" change; the three-build design, the file names and
  "never `target-cpu=native`" do not.
- [x] `docs/proposals/release-workflow.yml` and `release-workflow.md`: the
  proposed release workflow (tag-triggered, gates first, Windows and Linux
  `avx2` and plain builds, SHA-256 files, a smoke test, a **draft** release;
  `contents: write` on one job only; actions pinned by SHA). A proposal: the
  maintainer installs it under `.github/workflows/`, and it is not called
  working until it has run once on a test tag.
- [x] `docs/RELEASING.md`: the release procedure from 0.1.0 on, as a
  checklist (prepare, gates, merge and tag by the maintainer, the workflow's
  draft, checks and publishing, the results file). `docs/prompts/release.md`
  and `AGENTS.md` "Releases" point to it instead of holding the steps.
- [x] `TESTING.md`: how to read an SPRT result.
- [x] `TESTING.md`: how to read Ordo output.
- [x] `TESTING.md`: what a sound Elo comparison needs (same machine, same
  settings, enough games) and the limits of results from different machines.
- [x] `TESTING.md` and `UCI.md`: which scale `UCI_Elo` is on, in the wording
  the maintainer approved (2026-11-22): Ordo ratings on the scale of
  Ladder Baseline 2 with Stockfish 19 as the anchor, **shifted by a fixed
  3,500 points** so that Stockfish 19 is 3,500 and every setting is
  positive; not Lichess, CCRL or FIDE ratings; stretched by lopsided
  results, most visibly at the low end (Stockfish 19-1320 is about 1,530).
- [x] Match scripts, run notes (`match.ps1` and `match.sh` alike, with
  tests): at the start the git commit and whether the working tree is
  clean or dirty (read-only `git` in the repository, as the maintainer
  allowed on 2026-11-22; "unknown" without git or a repository), and a copy
  of the engine list beside the notes; at the end the number of games in
  the PGN and its SHA-256. Nothing written outside the output folder; an
  existing PGN is still refused. (`rate.ps1` and
  `rate.sh` are left alone; a bug found in them is reported, not fixed
  silently.)
- [x] Match scripts: `--test-option NAME=VALUE` (`-TestOption`), and
  `--base-option` (`-BaseOption`) for `sprt` and `h2h`, repeatable, giving
  that build `option.NAME=VALUE` and recorded in the run notes (the
  maintainer's choice, 2026-11-22), so that limited Vyvræl builds
  (`UCI_LimitStrength`, `UCI_Elo`, `Seed`) can be measured.
- [x] Robustness: a test that feeds the UCI loop malformed and unexpected
  input (random bytes, truncated and oversized commands, bad values, odd
  orders, commands during a search) from a seeded generator and asserts
  that it never panics and keeps answering `isready`. Any panic it finds is
  fixed in its own commit.

Part B, measuring (the maintainer plays, the agent records):

- [x] The scripts smoke-tested for real on the Windows workstation (a short
  run of each mode and a rating) and the result recorded in `TESTING.md`.
- [x] KI-8: a gauntlet of the current build against the core opponents of
  Ladder Baseline 2 ("Using the ladder for Vyvræl"), rated together with
  Baseline 2's games in one Ordo run anchored on Stockfish 19, recorded in
  `docs/testing/results-<build>.md` with Ordo's output. KI-8 is updated for
  the build measured.
- [x] `UCI_Elo` calibration: gauntlets of limited builds at several
  settings (for example 400, 800, 1200, 1500 and 1800) with a fixed `Seed`,
  rated with Baseline 2; the mapping in `search::Weakening::for_elo` and the
  option's range and default refitted to the measurements, on the approved
  scale; method, opponents and results recorded in `TESTING.md`. Marked
  provisional: it is measured again after milestone 7 and after queued
  task 2.
- [x] Soak test through `cutechess-cli` at 1+0, 3+2 and 60+0: no illegal
  move, crash, stall or time loss, recorded in `TESTING.md`. One time
  loss at 1+0 seconds, accepted by the maintainer as known issue KI-12,
  deferred (2026-11-22).

Closing:

- [x] End-of-milestone review and doc drift pass, then the report.

The release of 0.1.0 follows as its own step, with `docs/RELEASING.md`.

## Milestone 7: search heuristics (order of work for captures)

Set by the maintainer on 2026-11-20. The chunk checklist is written when the
milestone starts; this fixes the order in which capture ordering and
quiescence pruning are built, so that each step has a clean baseline to be
measured against.

1. **MVV-LVA with promotions.** Simple, and the baseline for everything
   after it. The quiescence search already orders captures this way
   (milestone 4, `search/ordering.rs`); what remains is to use it for the
   captures of the main search as well.
2. **The transposition-table move first**, when the table has one for the
   position. This needs the transposition table, which is also milestone 7.
3. **Static exchange evaluation (SEE)**, compared with the baseline by SPRT
   on the Windows workstation. It is kept only if the SPRT shows a gain.
4. **Delta pruning last**, tested separately by its own SPRT, so that its
   effect is not mixed up with SEE's.

Steps 3 and 4 change which moves the search looks at, so unlike plain
ordering they can change its results. That is why each gets its own SPRT.
The matches run on the Windows workstation with the match scripts;
results from different machines are not compared with each other.

The other milestone 7 heuristics (killer moves, history, null-move pruning,
late move reductions) are ordered when the milestone starts.

Also in milestone 7: sliding-piece attacks by the BMI2 instruction PEXT,
behind a feature, for the optional `pext` release build (see
"Release artifacts"). Magic bitboards stay the default, and perft must give
the same counts on both paths. It is a speed change that leaves the search
tree alone, so it is checked by perft and by an unchanged node count, not by
SPRT.

### Chunk checklist

Branch `milestone/7-search-heuristics`, started 2026-11-27 from `main`.

Items 1 to 4 are the order the maintainer set on 2026-11-20; the rest was
proposed by the agent and confirmed by the maintainer on 2026-11-27, with
the reason for each place. The maintainer also agreed that up to three
features may be built ahead of their SPRTs, each on the one before, and
tested in order: one that fails is dropped and what was built on it is
rebuilt.

How every search change is done:

- **One feature, one commit, one SPRT** against the commit before it, on
  the Windows workstation with `match.ps1 sprt`, both builds `avx2`,
  10+0.1, Hash 64, `alpha` and `beta` 0.05, a cap of 5,000 rounds. Bounds
  `[0, 10]` while changes are worth tens of Elo (items 1 to 9), `[0, 5]`
  after. The agent gives the command and the two builds; the maintainer
  runs it and sends the notes and the log back.
- **The result is a row of
  [`testing/sprt-history.md`](testing/sprt-history.md)**, written by
  `scripts/record_sprt.py`, and its summary line goes into the commit
  message. H1 accepted: the change stays. H0 accepted: it is reverted.
  Undecided is not a pass: the cap is raised and the test run again, or
  the change is dropped.
- **The bench node count moves with every one of them.** The commit
  updates the pin in `tests/bench.rs` and adds a row to
  [`testing/bench-history.md`](testing/bench-history.md). `vyvrael search
  --stats` before and after goes into the commit message: the first-move
  cutoff rate and the branching factor are what these changes are for.
- A change that is not meant to alter the search (a speed change, a
  protocol feature, a diagnostic) keeps the bench count and says so.

**Capture ordering and the table (the maintainer's order):**

- [x] 1. MVV-LVA with promotions for the captures of the main search. The
  baseline for everything after it. SPRT [0, 10]: H1 accepted in 120
  games (run `f38fad1831cb`).
- [x] 2a. The transposition table: the table itself, sized by `Hash`,
  cleared at `ucinewgame`, probed and stored in the main search, with its
  cutoffs. SPRT [0, 10]: H1 accepted in 344 games (run `ad16d90abf11`).
- [x] 2b. The table's move tried first, at the root too. SPRT [0, 10]:
  H1 accepted in 242 games (run `9710d85acf32`).
- [x] 2c. With the table, no SPRT (the bench count did not move, 206,097):
  the button `Clear Hash`, `hashfull` in `info`, the table's hit rate in
  the search statistics, and `bench <depth> <threads> <hash>` using its
  hash size (the bench without arguments keeps a fixed one).
- [x] 3. Static exchange evaluation, for ordering captures and for
  leaving losing ones out of quiescence. Kept only if its SPRT shows a
  gain. SPRT [0, 10]: H1 accepted in 1,160 games (run `9c8de5ceed4c`).
- [ ] 4. Delta pruning in quiescence, last of the four, with its own SPRT.
  **Parked.** Built as `4bfd377` (margin 200, captures that give check
  exempt; bench 64,509 from 77,407) and reverted in `2bdb660`: its SPRT
  [0, 10] was undecided at the cap, 3898 - 3748 - 2354 in 10,000 games,
  LLR 0.229 (run `d35a397d5e43`). The figures point at a gain of about
  5 Elo, which those bounds cannot settle. Deferred beyond this
  milestone with the other small items (see below).

**KI-10, once quiescence is pruned:**

- [x] 5. `stop` and the clock apply once the first root move of depth 1
  is done. Not a change to what a fixed-depth search finds, so the bench
  count stays; tested with the many-queens positions of KI-10. Done with
  two refinements. During the rest of the first depth they are looked at
  only once per 65,536 nodes, so that an ordinary first depth still
  always completes and a move played with no time left is as good as it
  was; and a node limit still never ends the first depth. Nine queens a
  side with 50 ms to move: an answer after about 0.3 s, from 4.8 s.
  KI-10 is not removed but narrowed: the first root move itself (that
  0.3 s) and the weakened search's first pass remain.

**Quiet-move ordering** (before the pruning that relies on it):

- [x] 6. Killer moves: two per ply, kept for one search. SPRT
  [0, 10]: H1 accepted in 372 games (run `f1b57a770a57`).
- [ ] 7. The history heuristic. SPRT.
  **Parked.** Built as `db0d976` (by side, origin and destination, with
  punishment of the quiet moves tried before the one that cut off; kept
  for one search; not used at the root; bench 60,070 from 61,760, and
  899,248 from 943,287 at depth 6) and reverted. Its SPRT [0, 10] was
  stopped by hand after 155 games (57 - 65 - 33, LLR -0.265; PGN
  `33583770c654`), which says nothing either way: it was stopped
  because a saving of about 5% of the nodes means a gain of a few Elo
  at most, which those bounds cannot settle short of the cap, as item 4
  showed. An interrupted run has no row in `sprt-history.md`. Put back
  on top of late move reductions (item 10) and measured, it does not
  save nodes there (the bench at depth 9 takes 9% more), which settles
  nothing since it changes which moves are reduced. Deferred beyond
  this milestone with the other small items (see below).

**Windows, then pruning and reductions.** Principal variation search is
proposed before null move and late move reductions, where the request
listed it after them: reductions are searched with a zero window and
re-searched on a fail high, which is the machinery of principal
variation search, so building it first keeps the reductions' commit to
the reductions.

- [ ] 8. Principal variation search. SPRT. **Deferred**, built and
  measured but not committed: by itself it is a small and uneven gain
  (the bench falls from 51,272 nodes to 47,073 and `bench 7` by 14%,
  but the start position at depth 6 takes 22,549 nodes with it against
  22,073 without, and without a table it costs nodes), which bounds of
  [0, 10] would not settle (see items 4 and 7). Late move reductions
  turned out not to need it: a reduced move is searched with a window
  of one point and searched again in full, which is all of its
  machinery they use. Deferred beyond this milestone with the other
  small items (see below).
- [x] 9. Null-move pruning, with its guards (not in check, not in a pawn
  endgame, a verification where zugzwang threatens). Built before item
  8: it does not need principal variation search, and it is the largest
  gain left, where item 8 alone is a small one that bounds of [0, 10]
  would not settle quickly (see items 4 and 7). From depth 3, reduction
  2 + depth / 4, only with the evaluation at or above beta, verified
  with one piece left. SPRT [0, 10]: H1 accepted in 198 games (run
  `023be622299e`).
- [x] 10. Late move reductions: quiet moves from the fourth on, from
  depth 3, by `0.75 + ln(depth) * ln(move) / 2.25` plies; without
  principal variation search (see item 8) and without history (see
  item 7). SPRT [0, 5]: H1 accepted in 620 games (run `00e8619ba2cf`).
- [ ] 11. Aspiration windows at the root. SPRT. Deferred (see below).
- [ ] 12. Check extensions. SPRT. Deferred (see below).
- [x] 13. Reverse futility pruning: up to depth 6, margin 100 a ply, not
  in check. Built before items 11 and 12, as the largest gain left
  among the items not yet built. SPRT: awaited; if it does not show a
  gain, its commit is reverted.
- [ ] 14. Internal iterative reduction. SPRT. Deferred (see below).
- [ ] 15. The countermove heuristic. SPRT. Deferred (see below).
- [ ] 16. Continuation history. SPRT. Deferred (see below).

**The small items are deferred** (the maintainer's decision, 2026-11-28):
items 4, 7, 8, 11, 12, 14, 15 and 16 are not part of this milestone any
more. Each is expected to be worth a few Elo by itself. The accepted
items were all worth 28 Elo or more and took 120 to 1,160 games to
show it; a change worth about 5 Elo needs some 20,000 games at bounds
of [0, 5], about a day of the Windows workstation each, and item 4
spent eleven hours to stay undecided. They stay on this list, unticked,
for a later pass over the search, where they can be tested at a faster
time control or together. Nothing of them is in the code: the two that
were committed (items 4 and 7) were reverted.

**Not search changes:**

- [ ] 17. Sliding-piece attacks by PEXT behind the Cargo feature `pext`.
  Magic bitboards stay the default; perft matches on both paths and the
  bench count is the same. No SPRT.
- [x] 18. `go mate` and `searchmoves` (the [B] list): protocol features,
  bench count unchanged, tested in `tests/uci.rs`. Neither is used by a
  weakened search (`UCI_LimitStrength`).

**At the end:**

- [ ] KI-9: the measurement table of `TESTING.md` taken again, and the
  entry removed if the search is no longer slow for lack of ordering.
- [ ] KI-8: the commands for measuring strength and `UCI_Elo` again
  against Ladder Baseline 2; the maintainer runs them, and the results
  are written up under `docs/testing/`.
- [ ] End-of-milestone doc drift pass, an independent review, the merge
  into `main`.

Pondering and the `tune` tool of the [B] list are not in this milestone:
pondering touches time management and the search thread and gets a
branch of its own, and the tuner waits for the maintainer's word on
where it fits.

## Decisions for milestones 4 to 6

Decided by the maintainer on 2026-11-20.

**Milestones are done in numerical order.** The search heuristics were
first recorded as "milestone 4b", which read as if it came before milestone
5; on 2026-11-20 it was renumbered to milestone 7. The milestones after it
therefore sit one higher than in the original brief: open data tooling is 8,
NNUE is 9 and tablebases are 10.

- **One ordering heuristic had to come early.** Approved by the maintainer
  on 2026-11-20. Quiescence search orders captures most valuable victim first
  in milestone 4, not milestone 7.
  Measured without it, one test position needed more than 20 million nodes
  where 3,067 suffice, so the search did not finish. Ordering captures does
  not change the value the search returns, only its cost, so there is
  nothing for SPRT to compare beyond "finishes" and "does not". Move
  ordering in the main search still waits for milestone 7.
- **Search is split in two so that heuristics can be proven.** The project
  rule is that a search change is accepted only if SPRT shows a gain, but
  SPRT needs the match script (before milestone 6), which needs UCI
  (milestone 5).
  So milestone 4 builds only the core that everything else depends on:
  negamax with alpha-beta, iterative deepening and quiescence search. UCI and
  the match script follow. Milestone 7 then adds move ordering (hash move,
  MVV-LVA, killers, history), the transposition table, null-move pruning and
  late move reductions, one feature per commit, each with a recorded SPRT
  result. The core has nothing to be compared against, so it is checked by
  correctness tests (mates found, known node counts at fixed depth).
- **`Evaluator` gets `Send + Sync` bounds** (done) as the first chunk of milestone 4,
  so the evaluator can be handed to the search thread. A later network
  evaluator must therefore keep any mutable per-search state outside the
  trait object.
- **Make/unmake or copy-make is chosen by measuring search** (done: the
  search uses copy-make). Milestone 4 started with make/unmake, measured both
  styles in the real search loop and kept the faster. Copy-make was 3 to 5%
  faster with identical node counts; the numbers are in TESTING.md.
- **KI-7 was fixed in milestone 5**, before the UCI loop, so the engine
  answers `isready` without a start-up delay.
- **Milestone 6 uses Cute Chess and Ordo** (the maintainer's choice, also
  2026-11-20). The full scope is under "Milestone 6: strength testing"
  above.

### Branding, required before rated play

Required by the maintainer (2026-11-20, amended the same day): Vyvræl gets
its branding before it plays rated games on Lichess or is entered in a
rating list. **Rated play is blocked until this is done.** The first alpha
release and the casual-only bot do not wait for it: they use what is already
decided (the name, the tagline and the avatar below).

Vyvræl is an **Australian** engine and is to be presented as one wherever a
country is asked for or shown: the Lichess bot profile's flag, rating lists
and engine directories that record a nationality, and the project's own
description.

What the branding has to cover is the maintainer's to decide. The list below
is a starting point, not a commitment:

- Name and spelling rules: `Vyvræl`, with `Vyvrael` only where ASCII is
  required, and how the name is written in prose and in file names.
- A logo or avatar (the dragon theme), in the sizes the places it appears
  need, with its licence stated.
- A one-line description and a short paragraph, used consistently in the
  README, the bot profile and any listing. The one line is decided
  (2026-11-20): "A dragon-flavoured UCI chess engine in Rust, built with AI
  and playing for the attack." It states a style goal, not a strength.
- The bot profile itself: name, flag, bio, links.
- What the engine says about itself over UCI (`id name`, `id author`).
- Credit and the 0rwx level X disclosure, which belongs wherever the engine
  is presented, not only in the README.

Assets offered by the maintainer for the branding (2026-11-20). Both are
Nexxus Drako Multimedia's own work, not third-party material:

- **The NXD fractal background** (`NXDFractal.svg`): a 500 by 500 low-poly
  field of triangles running from dark red (`#7f0000`) through orange to gold
  (`#f3c601`). It also gives the branding its palette.
- **Renders of Nexxus Drako**, the dragon VTuber who owns Nexxus Drako
  Multimedia (red hair, red wings). The avatar shows the engine's maker as
  well as the engine. Among them are a square render already set on the fractal
  background and a round one with a gold ring on dark red, which are the
  shapes an avatar needs.

**Licence (decided by the maintainer, 2026-11-20): all rights reserved.** The
brand assets are not covered by the code's GPL-3.0-or-later. They live in
their own directory, `branding/`, with a notice saying so, and the README's
licence section makes clear that the GPL covers the code only.

**Avatar (decided by the maintainer, 2026-11-20): the square render of Nexxus
Drako already set on the fractal background.** It is the image for the bot
profile and wherever else one picture stands for the engine.

In the repository: the avatar (`branding/vyvrael-avatar.png`, the
maintainer's 1920 by 1920 original) and the fractal background
(`branding/nxd-fractal.svg`). The other renders are not, and are added only
if a use for them is decided.

Artwork and other brand assets follow the project's rules for third-party
material: nothing is used without a clear licence, and its source and licence
are recorded.

### Playing on Lichess with lichess-bot

Wanted by the maintainer (2026-11-20). Run Vyvræl as a Lichess bot account
through `lichess-bot`, the bridge program that connects a UCI engine to the
Lichess API.

What it needs:

- **A working UCI engine.** `lichess-bot` speaks UCI to the engine, so this
  cannot start before milestone 5.
- **`lichess-bot` itself**, installed and run by the maintainer as a separate
  program. Like Cute Chess and Ordo it is not vendored, not linked and not a
  Cargo dependency. Its pinned version and licence (AGPL-3.0) are recorded
  in [DEPENDENCIES.md](DEPENDENCIES.md).
- **A Lichess bot account and API token**, created and held by the
  maintainer. The token is a secret: it is never committed, never written
  into a tracked file and never handled by an AI agent. The repository
  carries no lichess-bot configuration and no token placeholder: the
  maintainer's `config.yml` is copied from lichess-bot's own
  `config.yml.default`, lives outside this repository, and is git-ignored
  here in case it ever lands in it.
- **Documentation:** [LICHESS.md](LICHESS.md) holds the policy, the
  maintainer's checklist and the anti-abuse rules.

Plan, as amended by the maintainer on 2026-11-20. It replaces the earlier
plan of a private test first and a public bot only after milestone 7:

- **After the maintainer tagged 0.1.0-alpha.1: a casual-only bot.** The
  maintainer upgraded the account `Vyvrael-Chess` to BOT and ran
  lichess-bot on the Linux machine, accepting casual challenges only.
  Casual games cost nobody rating points, and they show whether the engine
  plays complete games over the real protocol without losing on time,
  crashing or making illegal moves.
- **Rated play: not before 0.2.0 (milestone 7)**, which follows milestone
  6, **and not before the branding above is done**, so a bot that plays for
  rating has a measured strength and SPRT-accepted search. The maintainer
  decides when, and re-reads the Lichess Terms of Service and Bot API
  documentation first.
- Before the upgrade nobody played a game on the account, because one game
  would have made the upgrade impossible.
- A bot that loses on time is worse than no bot: the option `Move Overhead`
  exists for this, and LICHESS.md gives a starting value.
- `UCI_LimitStrength` and `UCI_Elo` (above) let the bot offer weaker play
  now that they are calibrated (provisionally, on the ladder's scale); on a
  rated account that would have to be said in the profile.

What the bot has played with each build is recorded in
[lichess-history.md](lichess-history.md).

**KI-10 applies to the casual bot**, because the bot starts before milestone
7, which fixes it. The first search depth cannot be interrupted, so in a
position with very many captures the engine can overrun its time for the
move. The mitigation until then is `Move Overhead` and watching the bot's
games for losses on time (see [LICHESS.md](LICHESS.md)).

Still open for the maintainer, before rated play: the rest of the branding
(see above), which time controls and variants to accept, and whether to
accept challenges from humans, other bots or both.

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
- Ladder Baseline 3: the round robin run again with Vyvræl builds in it.
  (Ladder Baseline 2, run on 2026-11-21, has none.) It is run once
  milestone 7 is done and 0.2.0 is ready, not before (the maintainer,
  2026-11-22). Vyvræl 0.1.0 and 0.2.0 go in, beside 0.1.0-alpha.1 and
  0.1.0-alpha.2. Without
  Arasan 26.1-1800, retired from the installer on 2026-11-22; Ladder
  Baseline 2's record keeps it. With eleven new engines, in the installer
  since 2026-11-22 (`NEW_IN_BASELINE_3` in `scripts/ladder_engines.py`):
  Blunder 4.0.0, 5.0.0, 7.1.0 and 7.6.0, MinimalChess 0.4.1, 0.5 and 0.6,
  Velvet 1.0.0, Inanis 1.0.1, and Tantabus 1.0.2 and 2.0.0. Of them only Inanis has a move-overhead
  option, so the others play with their own margin; the run's notes must
  say so. The run is `match.ps1 roundrobin` (or `match.sh roundrobin`)
  with the installer's engine list and `-Build` for the four Vyvræl builds,
  on the Windows workstation, where every engine of the list installs (see
  "Round robin" in TESTING.md).
- Submitting Vyvræl to rating lists. Blocked on the branding, like rated
  play.
- Non-regression SPRT bounds for pure refactorings and speed work, beside
  the bounds for a gain, written up in `docs/TESTING.md`. (It already gives
  example bounds for "no loss"; what is missing is the rule for when they
  apply.)
