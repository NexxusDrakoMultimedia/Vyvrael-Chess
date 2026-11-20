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
| 6 | Strength testing | `milestone/6-strength-testing` | not started |
| 7 | Search heuristics (move ordering, transposition table, null move, late move reductions), each accepted by SPRT | `milestone/7-search-heuristics` | not started |
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
   command. See "Queued work".
3. **Milestone 6:** strength testing.
4. **Milestone 7:** search heuristics.
5. **Queued task 2:** benchmark and speed up.
6. **Milestone 8 onwards**, in numerical order.

**Where things run** (the maintainer, 2026-11-21): strength is measured on
the Windows workstation, where Ladder Baseline 1 and the results of
0.1.0-alpha.1 were played, so that later measurements can be compared with
them. lichess-bot runs on the Linux machine only. The scripts and the
installer still work on both systems.

Where the releases fall: **0.1.0-alpha.1** came before step 1, after
milestone 5; **0.1.0** follows step 3; **0.2.0** follows step 4. The
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
is due to be fixed. An entry is removed in the commit that fixes it. The
design behind each one is explained in [ARCHITECTURE.md](ARCHITECTURE.md).

| # | Issue | Effect today | Fix planned |
| --- | --- | --- | --- |
| KI-2 | **FEN parsing rejects inconsistent positions and has no repair mode.** A castling right without its king and rook at home, or an en passant square that does not match a double push, is an error. | Sloppy FENs from other tools (stale castling rights are the usual case) fail to load instead of being corrected. | Later, milestone not fixed yet: an opt-in "repair and warn" mode that drops the bad right or square and reports what it changed. Strict parsing stays the default. To be revisited by milestone 8 at the latest, when third-party data is first read. |
| KI-10 | **The first depth cannot be interrupted, and it can be slow.** The search always completes depth 1 so that there is a move to play, and ignores the clock and `stop` until then. Depth 1 includes an unpruned quiescence search, which in a position with very many captures takes long: about 25 ms with two queens a side, about 10 s in an artificial position with nine queens a side. | In such positions the engine overruns a short time budget and does not obey `stop` or `quit`, or answer anything else, until depth 1 is done. In ordinary positions depth 1 takes well under a millisecond. | Milestone 7: move ordering and pruning in quiescence search shrink depth 1; then let `stop` and the clock apply once the first root move is done. |
| KI-9 | **Search is slow without move ordering.** The main search tries moves in generation order, so alpha-beta prunes poorly. On a busy middlegame (Kiwipete) a depth-3 search visits 4.5 million nodes and takes about 1.5 s on the development machine (see the measurement table in `docs/TESTING.md`). | The engine plays, but shallowly for the time it uses. Results are correct. | Milestone 7: move ordering, the transposition table and the other heuristics, each measured by SPRT. |
| KI-8 | **The evaluation's strength is unmeasured.** The hand-crafted evaluation is tested for correctness (symmetry, material, phase arithmetic), but its values and tables have never been tested in play. | Unknown playing strength; the numbers are reasoned, not tuned. | Milestone 6: measure the engine against the opponent ladder with gauntlets and Ordo, and record the rating. Resolved in milestone 6 for the build measured, and re-measured after milestone 7. Tuning the terms by SPRT follows from milestone 7 on. |

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

- Milestone 2: none left.
- After the first release (0.1.0-alpha.1): a casual-only Lichess bot, run
  by the maintainer. Rated play only later, after milestone 7 and the
  branding (see "Playing on Lichess with lichess-bot").
- Milestone 5: none left (KI-3 and KI-7 were fixed in it).
- Milestone 6: calibration of `UCI_Elo`; KI-8.
- Milestone 7: KI-9, KI-10.
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

- **Milestone 5 (UCI), done:** both options exist and are wired to the
  weakening mechanism (`search::weakened_move`), uncalibrated.
- **Milestone 6 (strength testing):** calibrate. Play the limited engine at
  several `UCI_Elo` settings against opponents of known rating with the match
  script, fit the mapping from setting to measured Elo, and record the method,
  the opponents and the results in `TESTING.md`. Until this is done the
  options are documented as uncalibrated.

Constraints that already apply:

- Weakening is **strength-affecting randomness** (see the randomness rules in
  [AGENTS.md](../AGENTS.md)). It draws from the `rng` module, is seeded from
  system entropy by default, and is reproducible with `DeterministicRNG` and
  `RngSeed` (with the single option `Seed` once queued task 1 is done). It
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
of Ladder Baseline 1 and its Ordo anchor (Stockfish 19 = 0), not to
Lichess, CCRL or FIDE ratings. That scale is stretched by lopsided results,
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
| 0.1.0-alpha.1 | after milestone 5 | UCI conformance tests; complete games in `cutechess-cli` at 10+0.1 with no illegal move, crash, stall or time loss; KI-3 and KI-7 fixed; `Move Overhead`; `docs/LICHESS.md`. Strength unmeasured. |
| 0.1.0 | after milestone 6 | The scripts documented and smoke-tested; `UCI_Elo` calibrated; KI-8 resolved; a measured Ordo rating against the ladder in the release notes. The calibration and the rating are provisional: they are for the build measured. |
| 0.2.0 | after milestone 7 | The search heuristics, each accepted by SPRT. The first release with the `avx2` primary build, the optional `pext` and plain builds, and binaries built by a workflow (see "Release artifacts"). `UCI_Elo` and the Ordo rating measured again, and release notes that say the 0.1.0 calibration was provisional. They are measured once more after queued task 2. |

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
  builds.
- The binaries are built by a workflow, not by hand. The maintainer owns
  that workflow, because `.github/workflows` is a protected path that agents
  do not edit: an agent may propose its contents in a document for the
  maintainer to put in place.
- **The release of milestone 7 (0.2.0) is the first with these builds and
  with workflow-built binaries.** Until then a release ships plain builds
  made by hand, as 0.1.0-alpha.1 did.
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
real match or the real Ordo. The maintainer has since reported running the
tooling with 0.1.0-a1 on the Windows workstation and that it worked; which
parts, and with what result, is still to be written down here and in
`docs/TESTING.md`, whose status lines predate that.

## Recording results

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

## Queued work (requested 2026-11-21)

Two tasks asked for by the maintainer. Each gets a branch of its own from
`main`, small commits, and no version bump, tag or release. The first
follows the tooling branch, which is merged, so it is next; the second
waits until milestone 7 is done (the maintainer's decision of 2026-11-21).
See "Order of work".

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

- [ ] **One option `Seed`** replaces `DeterministicRNG` and `RngSeed`: 0
  takes a seed from system entropy (not the clock; the rule against
  time-based seeding stands), anything else is fixed. Spin, 0 to
  2147483647: Cute Chess keeps spin values as signed 32-bit numbers, and
  with today's maximum of 4294967295 its GUI resets `RngSeed` to 0 whenever
  it is edited. Seeds taken from entropy stay in that range so any logged
  seed can be typed back. The seed is logged with `info string` at start
  and at every `ucinewgame`, as now.
- [ ] **Option `Variety`** (spin, centipawns, 0 to 100, **default 0**). At
  the root only and only in the first 12 full moves, choose at random among
  the moves within `Variety` of the best, weighted towards the best. Never
  when the best score is a mate, and never a move that a second check shows
  to lose more than `Variety`. With 0 the engine plays exactly the best
  move, so matches and SPRT are untouched. It is exempt from the SPRT rule,
  like `UCI_LimitStrength`, because it weakens play on purpose; say so in
  `AGENTS.md`. The Lichess bot sets it (10) in `docs/LICHESS.md`.
- [ ] **`vyvrael bench`**: a fixed set of positions to a fixed depth with
  `Seed` fixed and `Variety` 0, printing `<nodes> nodes <nps> nps`. The
  node count must be the same on every run.
- [ ] Once `bench` exists, record the first bench node count and speed and
  the perft speed in `docs/testing/bench-history.md`, so that queued task 2
  has a baseline from before milestone 7.
- [ ] Tests: bench twice gives one node count; a fixed-depth search with a
  fixed `Seed` and `Variety` 0 is reproducible; with `Variety` 0 the root
  move is always the best one; the seed line is printed (no test may depend
  on two entropy seeds differing).
- [ ] Docs: the deterministic guarantee is one thread, a fixed `Seed` and
  `Variety` 0 (`docs/TESTING.md`, `docs/UCI.md`); the options in the README
  and `docs/LICHESS.md`; the randomness table in `docs/ARCHITECTURE.md`;
  the match scripts' `--deterministic`, which sets the old option; a
  changelog entry.
- [ ] Report: what made the engine deterministic, the bench node count
  before and after, the options added.

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

## Milestone 6: strength testing (Cute Chess + Ordo)

Scope as amended by the maintainer on 2026-11-20 and 2026-11-21. The chunk
checklist is written when the milestone starts. Milestone 6 is the
measuring: gauntlets against the ladder, the calibration of `UCI_Elo`, and
known issue KI-8 (the evaluation's strength). The scripts and the installer
it uses are built beforehand and described under "Testing tooling (before
milestone 6)" above.

**Opponent ladder reference:** Ladder Baseline 1,
[`docs/testing/ladder-Baseline-1.md`](testing/ladder-Baseline-1.md), a round
robin of 25 other engines run by the maintainer. Vyvræl is not in it. It
orders the opponents; Vyvræl's strength (KI-8) and the `UCI_Elo` calibration
are measured by gauntlets against its lower rungs, in separate results
files, with the same time control, hash, threads and openings.

Matches are run with `cutechess-cli` (from Cute Chess) and ratings computed
with Ordo. Neither tool is vendored into the repository or linked into the
engine: they run as separate programs and are installed by the user.

- **`docs/TESTING.md` sections:** how to read an SPRT result; how to read
  Ordo output; what a sound Elo comparison needs (same machine, same
  settings, enough games); the limits of results from different machines.
- **Acceptance rule:** only accept search and evaluation changes that SPRT
  shows to be a gain, and record the result in the commit message or in
  `docs/TESTING.md`.

To go into the chunk checklist when it is written:

- Record in `docs/TESTING.md` and `docs/UCI.md` which scale `UCI_Elo` is
  on (Ladder Baseline 1 and its Ordo anchor, not Lichess, CCRL or FIDE)
  and that the scale is stretched by lopsided results, most visibly at the
  low end.
- Robustness, wanted before 0.2.0 because the bot plays in public:
  - a test that feeds the UCI loop malformed and unexpected input and
    asserts that it never panics. The release profile uses
    `panic = "abort"`, so a panic ends the engine in the middle of a game;
  - a soak test through `cutechess-cli` at several time controls (1+0, 3+2
    and 60+0) with no illegal move, crash, stall or time loss.

Notes for whoever starts it:

- `scripts/install_tools.py` installs pinned versions of both tools (and
  lichess-bot) on Linux, and Cute Chess and Ordo on Windows; see
  TESTING.md. It has unit tests but has not been run end to end.
- The tools' licences and pinned versions are recorded in
  [DEPENDENCIES.md](DEPENDENCIES.md). Re-check them if a pin changes.
- The match settings rely on the UCI options `Hash` and `Threads`.
  Repeatable limited-strength runs also need a fixed seed: the options
  `DeterministicRNG` and `RngSeed` today, the single option `Seed` once
  queued task 1 is done.
- Calibrating `UCI_Elo` needs Vyvræl entries with `UCI_LimitStrength` on in
  the engine list: the match scripts give a build under test no options of
  its own.
- Match output (PGN, logs, rating files) is not committed; `.gitignore`
  already covers match output and `*.pgn`.

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

- **After the maintainer tags 0.1.0-alpha.1: a casual-only bot.** The
  maintainer upgrades the account `Vyvrael-Chess` to BOT and runs
  lichess-bot, on the Linux machine only, accepting casual challenges only. Casual games cost nobody
  rating points, and they show whether the engine plays complete games over
  the real protocol without losing on time, crashing or making illegal
  moves.
- **Rated play: not before 0.2.0 (milestone 7)**, which follows milestone
  6, **and not before the branding above is done**, so a bot that plays for
  rating has a measured strength and SPRT-accepted search. The maintainer
  decides when, and re-reads the Lichess Terms of Service and Bot API
  documentation first.
- Until the upgrade nobody plays a game on the account: one game makes the
  upgrade impossible.
- A bot that loses on time is worse than no bot: the option `Move Overhead`
  exists for this, and LICHESS.md gives a starting value.
- `UCI_LimitStrength` and `UCI_Elo` (above) let the bot offer weaker play
  once they are calibrated; on a rated account that would have to be said
  in the profile.

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
- Ladder Baseline 2: the round robin run again with Vyvræl builds in it.
- Submitting Vyvræl to rating lists. Blocked on the branding, like rated
  play.
- Non-regression SPRT bounds for pure refactorings and speed work, beside
  the bounds for a gain, written up in `docs/TESTING.md`. (It already gives
  example bounds for "no loss"; what is missing is the rule for when they
  apply.)
