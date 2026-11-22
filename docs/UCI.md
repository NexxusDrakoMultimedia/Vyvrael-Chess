# UCI interface

This document lists every UCI command and option Vyvræl supports, with
defaults and behaviour. It is updated in the same commit as any change to the
protocol surface.

## Current status

**Implemented.** Started without arguments, the binary speaks UCI on
standard input and output. A session looks like this (the seed, speeds and
times differ from run to run):

```text
$ vyvrael
uci
id name Vyvræl 0.1.0-alpha.2
id author Nexxus Drako Multimedia
option name Hash type spin default 16 min 1 max 65536
option name Threads type spin default 1 min 1 max 1
option name Seed type spin default 0 min 0 max 2147483647
option name Variety type spin default 0 min 0 max 100
option name UCI_LimitStrength type check default false
option name UCI_Elo type spin default 1360 min 1110 max 1630
option name Move Overhead type spin default 30 min 0 max 5000
info string rng seed 1463767669 (system entropy)
uciok
position startpos moves e2e4 e7e5
go depth 3
info depth 1 score cp 40 nodes 34 nps 1700000 time 0 pv d2d4
info depth 2 score cp 3 nodes 559 nps 2506726 time 0 pv d2d4 b8c6
info depth 3 score cp 37 nodes 7556 nps 3714847 time 2 pv d2d4 d7d5 g1f3
bestmove d2d4
quit
```

It has been tested by driving the binary over pipes (`tests/uci.rs`), by
seeded streams of malformed input (`tests/uci_robustness.rs`) and by
playing complete games under `cutechess-cli`; see "Complete games in
cutechess-cli" in [TESTING.md](TESTING.md). 0.1.0-alpha.1 has also played
in the Cute Chess GUI, and the releases run under lichess-bot (0.1.0-alpha.2
since 2026-11-22), both set up by the maintainer.

The "Status" columns below are the truth: only rows marked *implemented*
work. Started with arguments, the binary runs one of its other commands
(`perft`, `eval`, `search`, `bench`, `help`), which are not part of UCI; see
[TESTING.md](TESTING.md).

## Identity

| Field | Value |
| --- | --- |
| `id name` | `Vyvræl <version>` |
| `id author` | `Nexxus Drako Multimedia` |

The name is sent in UTF-8. `Vyvrael` is the ASCII form, used where a file or
package name needs one.

## Commands

| Command | Status | Behaviour |
| --- | --- | --- |
| `uci` | implemented | Prints the `id` lines, the option list, then `uciok`. |
| `isready` | implemented | Replies `readyok`, also while a search is running. The first one builds the attack tables (a few milliseconds), so the first search does not pay for them on the clock. |
| `ucinewgame` | implemented | Ends any search and returns to the start position with an empty game history. Starts the sequence of strength-affecting randomness afresh; without a chosen seed it takes a new one from system entropy and logs it. |
| `position [startpos \| fen <fen>] [moves ...]` | implemented | Ends any running search first (it reports its `bestmove`). Sets up the position and plays the listed moves. See "Bad input" below. The positions passed through become the game history, which the search uses to see repetitions. |
| `go` | implemented | Starts a search on its own thread and finishes with `bestmove`. Parameters below. A `go` sent while a search is running ends that search first (it reports its `bestmove`), then starts the new one. |
| `stop` | implemented | Ends the search as soon as possible; the search then prints `bestmove`. Without a search running it does nothing. The first depth is always finished first (see "The first depth" below). |
| `setoption name <name> [value <value>]` | implemented | Sets one of the options below. An unknown name is reported with `info string`. |
| `quit` | implemented | Ends any search (which reports its `bestmove`) and exits. |

Not offered: `debug`, `register`, `ponderhit`. They are answered with
`info string unknown command: <line>`, which echoes the line.

### `go` parameters

| Parameter | Status | Behaviour |
| --- | --- | --- |
| `wtime`, `btime`, `winc`, `binc` | implemented | The clocks and increments in milliseconds. The move gets `remaining / moves + 3/4 increment`, where `moves` is `movestogo` or 30, and never more than is on the clock less the `Move Overhead` (30 ms by default). A negative clock counts as zero. |
| `movestogo` | implemented | Moves to the next time control. |
| `movetime` | implemented | Think this long, less the `Move Overhead`. Takes precedence over the clocks. |
| `depth` | implemented | Search this many plies and no deeper. Reproducible. |
| `nodes` | implemented | Stop after about this many positions. Reproducible. The first depth is always completed, even beyond the limit, so that there is a move to play. |
| `infinite` | implemented | Search until `stop`. The clocks are ignored. `bestmove` is sent only after `stop`, even if the search has nothing left to look at. A `go` with no parameters at all behaves the same way. |
| `ponder`, `searchmoves`, `mate` | not offered | Ignored, with their arguments, as the protocol asks for unknown words. A `go` left with no limit once they are ignored (for example `go mate 3`) searches like `go infinite` and answers only after `stop`. |

Several limits may be combined; the search stops at whichever comes first.

### The first depth

**The first depth is always completed**, whatever the clock, the node limit
or `stop` say, so that there is always a move to play. Normally that takes
well under a millisecond. In a position with very many captures available
it can take much longer, because quiescence search there is not yet pruned:
tens of milliseconds in a middlegame with two queens a side, and seconds in
artificial positions with many queens. Until it is done `stop` and `quit`
are not obeyed and, because the engine waits for the search to end before
it reads on, nothing else is answered either. This is known issue KI-10 in
[ROADMAP.md](ROADMAP.md); it goes away with the search heuristics of
milestone 7.

### Output

- `info depth <d> score <cp x \| mate n> nodes <n> nps <n> time <ms> pv <moves>`
  after every completed depth. `nodes` and `time` count from the start of the
  search. `score` is from the side to move's point of view; `mate n` counts
  the engine's own moves and is negative when the engine is being mated.
- `bestmove <move>` at the end. A position with no legal move (checkmate or
  stalemate) gives `bestmove 0000`.
- `info string <text>` for anything the engine wants to say that is not
  part of the protocol, such as a refused command.

### Bad input

Nothing a GUI sends can make the engine play an illegal move or crash.

- Every move of a `position` command is looked up among the legal moves. If
  one is not legal, or is not a move, the **whole command is refused**: the
  engine says which move with `info string` and keeps the position it had.
- A FEN that does not describe a legal position is refused in the same way.
- Input need not even be text. Bytes that are not valid UTF-8 are replaced
  and the line is handled like any other, which normally means it is
  reported as not understood. They do not end the engine.
- A line with no known command, or a known command that cannot be
  understood, is reported with `info string` and otherwise ignored. Words
  before the first known command are skipped, as the protocol asks.
- When the input ends without `quit` (a script piping commands in), a
  running search is allowed to finish and report; an infinite one is
  stopped.

## Options

Option names are matched without regard to letter case. A value of the wrong
type or out of range is refused with an `info string`, and the option keeps
its old value. An option set while a search is running applies from the next
search.

| Option | Type | Default | Status | Behaviour |
| --- | --- | --- | --- | --- |
| `Hash` | spin | `16` (1 to 65536) | implemented, **no effect yet** | Transposition table size in MiB. The value is checked and stored, but there is no transposition table before milestone 7, so it changes nothing. It is offered now because match tools set it. |
| `Threads` | spin | `1` (1 to 1) | implemented | Number of search threads. The search is single-threaded, so 1 is the only value: `Threads=1` from a match tool is accepted, and a larger number is refused with an `info string`. The maximum rises when a parallel search exists. |
| `Seed` | spin | `0` (0 to 2147483647) | implemented | Seed of strength-affecting randomness. `0` takes a new seed from system entropy when the engine starts and at every `ucinewgame`; any other value is that seed. Setting it back to `0` takes a fresh entropy seed. The maximum is the largest signed 32-bit number, which Cute Chess can store; entropy seeds stay from 1 to it, so a logged seed can always be given back. It replaces `DeterministicRNG` and `RngSeed` (0.1.0-alpha.1). |
| `Variety` | spin | `0` (0 to 100) | implemented | Centipawns. In the first 12 full moves, at full strength, play at random among the best move and the moves a second search checks to be within this many centipawns of it, weighted towards the best. Never when the best score is a mate. `0` always plays the best move. It weakens play on purpose. See "Variety" below. |
| `EvalFile` | string | to be decided | planned (milestone 9) | Path to an NNUE network file. |
| `UseNNUE` | check | to be decided | planned (milestone 9) | Select the NNUE or the hand-crafted evaluation. |
| `UCI_LimitStrength` | check | `false` | implemented, **calibrated provisionally** | Standard UCI option. When `true`, play at the rating in `UCI_Elo` instead of full strength. When `false`, play is unchanged, down to the node count. See "Limited strength" below. |
| `UCI_Elo` | spin | `1360` (1110 to 1630) | implemented, **calibrated provisionally** | Standard UCI option. Target rating used when `UCI_LimitStrength` is `true`; ignored otherwise. A rating on the scale of the reference ladder (see "Limited strength" below), not a Lichess, CCRL or FIDE rating. The range is what the weakening was measured to cover; the mapping is fitted to seven measured settings. Provisional: measured for 0.1.0-alpha.2 and measured again after milestone 7 and after queued task 2. |
| `Move Overhead` | spin | `30` (0 to 5000) | implemented | Milliseconds kept back from every time limit, for delays the engine cannot see. See "Move Overhead" below. |
| `SyzygyPath` | string | empty | optional (milestone 10) | Path to Syzygy tablebases, only if that milestone is approved. |

Strength limiting uses randomness to weaken play, so it is governed by
`Seed`: non-deterministic by default (`Seed` 0), reproducible with any other
value. See "Planned features" in [ROADMAP.md](ROADMAP.md).

**Deterministic play** needs one thread, a fixed `Seed`, `Variety` 0 and a
search that does not depend on the clock (`go depth` or `go nodes`).
Full-strength play with `Variety` 0 draws no random numbers at all, so at a
fixed depth it is the same with any `Seed`; the seed matters for limited
strength and for `Variety`.

### Variety

An engine that always plays its best move repeats its games: the same
position searched to the same depth gives the same move. `Variety` above 0
breaks that in the opening:

- It applies at full strength, in positions whose full-move number is at
  most 12, and never when the best score is a mate.
- The search runs as usual, on two thirds of the move's time or node
  budget. Then every other move gets a quick score, those near the best are
  searched again to the same depth, and only a move whose score that second
  search puts within `Variety` of the best is a choice. The rest of the
  budget is for that check; a move it could not check in time is not
  played. With `go depth` alone (no clock, `movetime` or `nodes`, and no
  `stop`) the check always completes.
- The engine plays one of the choices at random, the best move included
  and the most likely, and says so in an `info string` when it is not the
  best: `info string variety: d7d6 instead of g8f6, 9 cp below it, one of 6
  moves`. The `info` lines before it are those of the search. After it
  comes one more `info` line for the move played: its score from the check
  and a `pv` of that move alone, so that a GUI records the move actually
  played.
- It draws from the same seeded generator as limited strength, so with a
  fixed `Seed` and `go depth` a varied game is reproducible.

What "within `Variety` centipawns" means depends on the evaluation: at shallow
depths the hand-crafted evaluation rates many quiet opening moves within a
few centipawns of each other, so a large `Variety` plays odd but legal
openings. `Variety` weakens play on purpose, so it is exempt from the rule
that changes must pass an SPRT, and it must be 0 in any match that measures
strength.

### Move Overhead

The engine only knows how long it thinks. It does not see the time a move
spends on its way: the GUI or match tool handling it, the operating system
scheduling the process, and for an online game the network. `Move Overhead`
is the number of milliseconds kept back for that.

- `movetime` is used less the overhead.
- With a clock, the overhead caps what one move may take: never more than
  what is on the clock less the overhead. An ordinary share of the time is
  far below that cap and is not shortened, so the option matters when the
  clock is nearly empty and on the last move before a time control.
- There is always at least one millisecond to move in. Without an
  increment, a long game at a very short time control can still lose on
  time; this is known issue KI-12 in [ROADMAP.md](ROADMAP.md).
- It does not change which move is chosen at a fixed depth or node count,
  only how long a search on the clock may run.

The default of 30 ms suits a GUI or match tool on the same machine. Raise it
when moves travel over a network: see [LICHESS.md](LICHESS.md) for a starting
value with lichess-bot. The name is two words with one space, as other
engines and tools spell it: `setoption name Move Overhead value 100`.

### Limited strength

With `UCI_LimitStrength` on, `go` does not run the ordinary search. It
searches every legal move to a shallow depth, misjudges each move's score by
a bounded random amount, and plays the move that then looks best (see
"Playing below full strength" in [ARCHITECTURE.md](ARCHITECTURE.md)). What
this means for a GUI:

- There is one `info` line, sent when the move is chosen. Its `depth` is how
  far the scores the choice was made from look ahead, its `score` the true
  score of the move played, and its `pv` that one move.
- `go depth` is ignored: the depth belongs to the strength setting. `nodes`,
  `movetime`, the clocks and `stop` end the work early as usual. The moves
  are scored in passes of increasing depth and the choice is made from the
  last pass that finished, so short of time the engine looks less far, but
  it has always looked at every move. The first pass (one ply) always
  completes, as the first depth of the ordinary search does.
- The moves are random, so two games differ unless `Seed` is fixed.
  `ucinewgame` starts the random sequence again, from the chosen seed or
  from a new logged entropy seed, so each game can be replayed by itself,
  given the same commands and limits that do not depend on the clock
  (`nodes`, or time that is never used up).
- **The rating is calibrated provisionally.** Seven settings of the
  weakening were measured against the reference ladder in milestone 6, and
  `UCI_Elo` is mapped through them:

  | `UCI_Elo` | Plies | Largest misjudgement (cp) |
  | --: | --: | --: |
  | 1110 | 2 | 300 |
  | 1150 | 2 | 230 |
  | 1220 | 2 | 171 |
  | 1330 | 2 | 112 |
  | 1350 | 3 | 151 |
  | 1410 | 3 | 90 |
  | 1630 | 3 | 20 |

  Between two of these at the same depth the misjudgement is interpolated
  in a straight line. From 1331 to 1349, where the depth changes, the
  engine plays as 1330: up to 20 points weak for the number, never
  stronger than a higher setting. The default, 1360, plays exactly as the default did
  before the calibration. Nothing below 1110 can be played: the weakest
  setting already beats the two lowest rungs of the ladder in most games.
  The measurement and its limits are in [TESTING.md](TESTING.md),
  "Calibration of `UCI_Elo`". It holds for the build measured and is
  measured again after milestone 7 and after queued task 2.

**What scale `UCI_Elo` is on.** Its numbers are ratings on the scale of the
reference ladder, Ladder Baseline 2, as computed by Ordo with Stockfish 19
as the anchor, shifted by a fixed 3,500 points so that Stockfish 19 is
3,500 and every setting is a positive number. They are not Lichess, CCRL or
FIDE ratings and cannot be converted to them. The scale is stretched by
lopsided results, most visibly at the low end: the limited engines of the
ladder rate far below the numbers in their names (Stockfish 19-1320 is
about 1,530 on this scale). A setting of 1,200 means about as strong as an
engine rated 1,200 on this ladder, not a 1,200 player. The calibration of
milestone 6 maps the settings onto this scale (see [TESTING.md](TESTING.md),
"The scale of `UCI_Elo`").

The seed in use is always logged, so any run can be replayed:

```text
info string rng seed 1141592653 (system entropy)
info string rng seed 822286742 (Seed)
```

The line is sent in reply to `uci`, just before `uciok`, whenever `Seed`
changes the seed, and at every `ucinewgame`: with `Seed` 0 **every game takes
a seed of its own** from system entropy, and with a fixed `Seed` every game
starts from that same seed. To replay a game, set `Seed` to the number
logged at its `ucinewgame`, then send the same commands.

`Seed` only governs *strength-affecting* randomness. Zobrist keys, magic
numbers and similar tables are always generated from the fixed default seed
and are not affected by it.
See the randomness table in [ARCHITECTURE.md](ARCHITECTURE.md).

## Output rules

- stdout carries UCI protocol output only, flushed after every line.
- Diagnostics go to stderr or to `info string` lines.
