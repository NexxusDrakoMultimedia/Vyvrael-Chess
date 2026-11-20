# UCI interface

This document lists every UCI command and option Vyvræl supports, with
defaults and behaviour. It is updated in the same commit as any change to the
protocol surface.

## Current status

**The protocol loop works; the options are being added.** Started without
arguments, the binary speaks UCI on standard input and output:

```text
$ vyvrael
uci
id name Vyvræl 0.1.0
id author Nexxus Drako Multimedia
uciok
position startpos moves e2e4 e7e5
go movetime 300
...
info depth 4 score cp 3 nodes 113628 nps 3384001 time 33 pv d2d4 b8c6 g1f3 g8f6
...
bestmove d2d4
quit
```

The "Status" columns below are the truth: only rows marked *implemented*
work. Started with arguments, the binary runs one of its other commands
(`perft`, `eval`, `search`, `help`), which are not part of UCI; see
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
| `ucinewgame` | implemented | Ends any search and returns to the start position with an empty game history. |
| `position [startpos \| fen <fen>] [moves ...]` | implemented | Sets up the position and plays the listed moves. See "Bad input" below. The positions passed through become the game history, which the search uses to see repetitions. |
| `go` | implemented | Starts a search on its own thread and finishes with `bestmove`. Parameters below. A `go` sent while a search is running ends that search first (it reports its `bestmove`), then starts the new one. |
| `stop` | implemented | Ends the search as soon as possible; the search then prints `bestmove`. Without a search running it does nothing. |
| `setoption name <name> [value <value>]` | implemented | Sets one of the options below. An unknown name is reported with `info string`. |
| `quit` | implemented | Ends any search (which reports its `bestmove`) and exits. |

Not offered: `debug`, `register`, `ponderhit`. They are answered with
`info string unknown command`.

### `go` parameters

| Parameter | Status | Behaviour |
| --- | --- | --- |
| `wtime`, `btime`, `winc`, `binc` | implemented | The clocks and increments in milliseconds. The move gets `remaining / moves + 3/4 increment`, where `moves` is `movestogo` or 30, and never more than is on the clock less a 30 ms overhead. A negative clock counts as zero. |
| `movestogo` | implemented | Moves to the next time control. |
| `movetime` | implemented | Think this long, less the 30 ms overhead. Takes precedence over the clocks. |
| `depth` | implemented | Search this many plies and no deeper. Reproducible. |
| `nodes` | implemented | Stop after about this many positions. Reproducible. The first depth is always completed, even beyond the limit, so that there is a move to play. |
| `infinite` | implemented | Search until `stop`. The clocks are ignored. `bestmove` is sent only after `stop`, even if the search has nothing left to look at. A `go` with no parameters at all behaves the same way. |
| `ponder`, `searchmoves`, `mate` | not offered | Ignored, with their arguments, as the protocol asks for unknown words. |

Several limits may be combined; the search stops at whichever comes first.

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
| `DeterministicRNG` | check | `false` | implemented | When `false`, strength-affecting randomness is seeded from system entropy when the engine starts. When `true`, it is seeded from `RngSeed`. Switching it off again takes a fresh entropy seed. |
| `RngSeed` | spin | `822286742` (`0x31031996`), 0 to 4294967295 | implemented | Seed for strength-affecting randomness when `DeterministicRNG` is `true`. Ignored while it is `false`. |
| `EvalFile` | string | to be decided | planned (milestone 9) | Path to an NNUE network file. |
| `UseNNUE` | check | to be decided | planned (milestone 9) | Select the NNUE or the hand-crafted evaluation. |
| `UCI_LimitStrength` | check | `false` | implemented, **uncalibrated** | Standard UCI option. When `true`, play at the rating in `UCI_Elo` instead of full strength. When `false`, play is unchanged, down to the node count. See "Limited strength" below. |
| `UCI_Elo` | spin | to be decided by calibration | planned (milestone 5, calibrated in milestone 6) | Standard UCI option. Target rating used when `UCI_LimitStrength` is `true`. Minimum, maximum and default come from measured strength. |
| `SyzygyPath` | string | empty | optional (milestone 10) | Path to Syzygy tablebases, only if that milestone is approved. |

Strength limiting uses randomness to weaken play, so it is governed by
`DeterministicRNG` and `RngSeed`: non-deterministic by default, reproducible
when `DeterministicRNG` is `true`. See "Planned features" in
[ROADMAP.md](ROADMAP.md).

### Limited strength

With `UCI_LimitStrength` on, `go` does not run the ordinary search. It
searches every legal move to a shallow depth, misjudges each move's score by
a bounded random amount, and plays the move that then looks best (see
"Playing below full strength" in [ARCHITECTURE.md](ARCHITECTURE.md)). What
this means for a GUI:

- There is one `info` line, sent when the move is chosen. Its `depth` is the
  shallow depth used, its `score` the true score of the move played, and its
  `pv` that one move.
- `go depth` is ignored: the depth belongs to the strength setting. `nodes`,
  `movetime`, the clocks and `stop` end the work early as usual; a move that
  was not fully looked at is then not considered.
- The moves are random, so two games differ unless `DeterministicRNG` is on.
  With it on, `ucinewgame` starts the random sequence again, so each game
  can be replayed by itself, given the same commands and limits that do not
  depend on the clock (`nodes`, or time that is never used up).
- **The rating is not calibrated.** The settings are ordered (a higher
  rating looks further and errs less) but nothing has been measured yet.
  Calibration is milestone 6.

The seed in use is always logged, so any run can be replayed:

```text
info string rng seed 3141592653 (system entropy)
info string rng seed 822286742 (RngSeed)
```

The line is sent in reply to `uci`, just before `uciok`, and again whenever
one of the two options changes the seed. To replay a game that used an
entropy seed, set `RngSeed` to the logged number and `DeterministicRNG` to
`true`.

`DeterministicRNG` and `RngSeed` only govern *strength-affecting*
randomness. Zobrist keys, magic numbers and similar tables are always
generated from the fixed default seed and are not affected by these options.
See the randomness table in [ARCHITECTURE.md](ARCHITECTURE.md).

## Output rules

- stdout carries UCI protocol output only, flushed after every line.
- Diagnostics go to stderr or to `info string` lines.
