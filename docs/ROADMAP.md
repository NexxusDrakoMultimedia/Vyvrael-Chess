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
| 5 | UCI | `milestone/5-uci` | in progress |
| 6 | Strength testing | `milestone/6-strength` | not started |
| 7 | Search heuristics (move ordering, transposition table, null move, late move reductions), each accepted by SPRT | `milestone/7-search-heuristics` | not started |
| 8 | Open data tooling | `milestone/8-data` | not started |
| 9 | Neural evaluation (NNUE) | `milestone/9-nnue` | not started |
| 10 | Endgame tablebases (optional, needs approval) | `milestone/10-tablebases` | not started |

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
| KI-9 | **Search is slow without move ordering.** The main search tries moves in generation order, so alpha-beta prunes poorly. On a busy middlegame (Kiwipete) a depth-3 search visits 4.5 million nodes and takes about 1.5 s on the development machine (see the measurement table in `docs/TESTING.md`). | The engine plays, but shallowly for the time it uses. Results are correct. | Milestone 7: move ordering, the transposition table and the other heuristics, each measured by SPRT. |
| KI-8 | **The evaluation's strength is unmeasured.** The hand-crafted evaluation is tested for correctness (symmetry, material, phase arithmetic), but its values and tables have never been tested in play. | Unknown playing strength; the numbers are reasoned, not tuned. | Milestone 6: SPRT the evaluation's terms and tune them, once search (milestone 4) and the match script exist. |

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
- [ ] `uci`: option `Threads`
- [ ] `rng`: a seed from system entropy and the stream for
  strength-affecting randomness
- [ ] `uci`: options `DeterministicRNG` and `RngSeed`
- [ ] `search`: weakened move choice for limited strength
- [ ] `uci`: option `UCI_LimitStrength`
- [ ] `uci`: option `UCI_Elo` (uncalibrated until milestone 6)
- [ ] Integration test: drive the binary over stdin and stdout
- [ ] End-of-milestone review and doc drift pass

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

## Milestones 5 to 10

Scope summaries live in the "Build order" section of
[AGENTS.md](../AGENTS.md). Each milestone's chunk checklist is added here when
that milestone starts, and must include the known issues due in it:

- Milestone 2: none left.
- After milestone 5: private `lichess-bot` smoke test. Public only later,
  after milestone 7 and the branding (see "Playing on Lichess with
  lichess-bot").
- Milestone 5: KI-3, KI-7, plus the `UCI_LimitStrength` and `UCI_Elo` options
  (see "Planned features").
- Milestone 6: calibration of `UCI_Elo`; KI-8.
- Milestone 7: KI-9.
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

## Milestone 6: strength testing (Cute Chess + Ordo)

Scope as amended by the maintainer on 2026-11-20. It replaces the original
"fastchess or Cutechess" wording. The chunk checklist is written when the
milestone starts.

Use `cutechess-cli` (from Cute Chess) to run engine-against-engine matches,
and Ordo to compute ratings from the resulting PGN. Neither tool is vendored
into the repository or linked into the engine: they run as separate programs
and are installed by the user. `docs/TESTING.md` documents them: install
steps for Windows and Ubuntu, minimum versions, and how to confirm they work.

- **Match scripts:** `scripts/match.sh` (bash) and `scripts/match.ps1`
  (PowerShell) run a match between two engine binaries with SPRT. The two
  take the same parameters and behave the same on both systems.
  - Parameters, all with sensible defaults: engine paths, engine names, time
    control, hash, rounds, concurrency, opening file, output directory.
  - `cutechess-cli` settings: `-each tc=10+0.1 option.Hash=16
    option.Threads=1`; an EPD or PGN opening file with `order=random`;
    `-repeat` so each opening is played from both sides; resign and draw
    adjudication; `-ratinginterval`; `-recover`; `-pgnout`; and
    `-sprt elo0=0 elo1=5 alpha=0.05 beta=0.05`.
  - Pass `option.DeterministicRNG=true` where reproducibility matters, and
    say so in the script's output.
- **Rating scripts:** `scripts/rate.sh` and `scripts/rate.ps1` run Ordo over
  one or more PGN files with a named anchor engine (Ordo's `-A`) and write
  `ratings.txt` and `ratings.csv`. Same parameters on both systems.
- **Check the real flags.** Before writing either script, read
  `cutechess-cli --help` and `ordo --help` from the installed versions and
  use the flags they actually document. Do not assume a flag from memory or
  from this document; if a flag named here does not exist in the installed
  version, use the equivalent and say so in `docs/TESTING.md`.
- **Portability:** no hard-coded paths. Detect the tools on `PATH`, fail with
  a clear message if one is missing, and keep the scripts working on both
  Windows 11 and Ubuntu.
- **Concurrency:** default `-concurrency` to a conservative value (physical
  cores minus two) with one thread per engine, and document how to change it.
- **`docs/TESTING.md` sections:** how to read an SPRT result; how to read
  Ordo output; what a sound Elo comparison needs (same machine, same
  settings, enough games); the limits of results from different machines.
- **Acceptance rule:** only accept search and evaluation changes that SPRT
  shows to be a gain, and record the result in the commit message or in
  `docs/TESTING.md`.
- **Chunks:** the match script, then the rate script, then the docs. Each
  script is smoke-tested with a few games against a trivial engine (or at
  least a dry run) before it is committed.

Also due in this milestone: calibration of `UCI_Elo` (see "Planned
features") and known issue KI-8 (the evaluation's strength).

Notes for whoever starts it:

- `scripts/install_tools.py` already installs pinned versions of both tools
  (and lichess-bot) on Linux; see TESTING.md. It has unit tests but has not
  been run end to end. Windows install steps are still to be documented by
  hand, since the installer is Linux-only by the maintainer's choice.

- The tools' licences and pinned versions are recorded in
  [DEPENDENCIES.md](DEPENDENCIES.md). Re-check them if a pin changes.
- The match settings rely on the UCI options `Hash`, `Threads` and
  `DeterministicRNG`, which milestone 5 provides.
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
   on the Linux machine. It is kept only if the SPRT shows a gain.
4. **Delta pruning last**, tested separately by its own SPRT, so that its
   effect is not mixed up with SEE's.

Steps 3 and 4 change which moves the search looks at, so unlike plain
ordering they can change its results. That is why each gets its own SPRT.
The matches run on the Linux machine with the scripts from milestone 6;
results from different machines are not compared with each other.

The other milestone 7 heuristics (killer moves, history, null-move pruning,
late move reductions) are ordered when the milestone starts.

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
  SPRT needs the match script (milestone 6), which needs UCI (milestone 5).
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
- **KI-7 is fixed in milestone 5**, before the UCI loop, so the engine answers
  `isready` without a start-up delay.
- **Milestone 6 uses Cute Chess and Ordo** (the maintainer's choice, also
  2026-11-20). The full scope is under "Milestone 6: strength testing"
  above.

### Branding, required before going public

Required by the maintainer (2026-11-20): Vyvræl gets its branding before it
is published anywhere the public meets it, whether that is a Lichess bot
account, a rating list, a release or an announcement. Private testing does
not need it. **Going public is blocked until this is done.**

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
  README, the bot profile and any listing.
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
  Cargo dependency. Its licence is to be confirmed on its official source and
  recorded in [DEPENDENCIES.md](DEPENDENCIES.md) before first use.
- **A Lichess bot account and API token**, created and held by the
  maintainer. The token is a secret: it is never committed, never written
  into a tracked file and never handled by an AI agent. The repository will
  carry only an example configuration with a placeholder where the token
  goes, and the real configuration file will be git-ignored.
- **Documentation:** tested setup steps in the README (replacing the current
  placeholder) and in `docs/UCI.md`, including which engine options to set.

Plan, in this order (milestone 5 comes two milestones before milestone 7, so
the private test comes well before the bot is public):

- **First, after milestone 5**, a private smoke test that the engine
  plays complete games over the real protocol without losing on time,
  crashing or making illegal moves.
- **Later, going public: not before milestone 7**, which follows milestone
  6, **and not before the branding above is done**, so the bot that other people
  play has been through SPRT-tested search. Playing rated games with an
  untested engine would say little and could annoy opponents.
- Time management has to be sound first: `go wtime/btime/winc/binc` handling
  is part of milestone 5, and a bot that flags is worse than no bot.
- `UCI_LimitStrength` and `UCI_Elo` (above) let the bot offer weaker play once
  they are calibrated.

Open questions for the maintainer, to settle before going public: the
branding (see above), the bot's
account name, which time controls and variants to accept, whether to accept
rated games and challenges from humans, other bots or both, and where the bot
will run (it needs a machine that stays on).

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
