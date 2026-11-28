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
id name Vyvræl 0.1.0
id author Nexxus Drako Multimedia
option name Hash type spin default 16 min 1 max 65536
option name Threads type spin default 1 min 1 max 1
option name Seed type spin default 0 min 0 max 2147483647
option name Variety type spin default 0 min 0 max 100
option name UCI_LimitStrength type check default false
option name UCI_Elo type spin default 1360 min 1110 max 1630
option name Move Overhead type spin default 30 min 0 max 5000
option name UCI_ShowWDL type check default false
option name SearchStats type check default false
option name Clear Hash type button
info string rng seed 1463767669 (system entropy)
uciok
position startpos moves e2e4 e7e5
go depth 3
info depth 1 seldepth 3 score cp 40 nodes 32 nps 1700000 hashfull 0 time 0 pv d2d4
info depth 2 seldepth 7 score cp 3 nodes 173 nps 2506726 hashfull 0 time 0 pv d2d4 b8c6
info depth 3 seldepth 7 score cp 37 nodes 1502 nps 3714847 hashfull 1 time 0 pv d2d4 d7d5 g1f3
bestmove d2d4
quit
```

It has been tested by driving the binary over pipes (`tests/uci.rs`), by
seeded streams of malformed input (`tests/uci_robustness.rs`) and by
playing complete games under `cutechess-cli`; see "Complete games in
cutechess-cli" in [TESTING.md](TESTING.md). 0.1.0-alpha.1 has also played
in the Cute Chess GUI, and the releases run under lichess-bot (0.1.0
since 2026-11-23), both set up by the maintainer.

The "Status" columns below are the truth: only rows marked *implemented*
work. Started with arguments, the binary runs one of its other commands
(`perft`, `eval`, `search`, `bench`, `suite`, `help`), which are not part of UCI; see
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
| `stop` | implemented | Ends the search as soon as possible; the search then prints `bestmove`. Without a search running it does nothing. The first depth is finished first, unless it is a very long one (see "The first depth" below). |
| `setoption name <name> [value <value>]` | implemented | Sets one of the options below. An unknown name is reported with `info string`. |
| `quit` | implemented | Ends any search (which reports its `bestmove`) and exits. |

Two commands are not part of the protocol and are there for a person at a
terminal. Both answer in `info string` lines, so that everything on
standard output stays protocol output; both read the current position
(the one the next `go` would search), change nothing, and are answered
while a search runs without disturbing it. They are commands only when
they are the whole line.

| Command | Behaviour |
| --- | --- |
| `d` | The board (ranks 8 to 1, FEN letters, `.` for an empty square), then `fen:`, `key:` (the position's hash, 16 hexadecimal digits) and `checkers:` (the squares of the pieces giving check, or `-`). |
| `eval` | The static evaluation with its parts: the seven lines that `vyvrael eval` prints (see "Command line and bench" in [ARCHITECTURE.md](ARCHITECTURE.md)). Nothing is searched. |

```text
d
info string 8 r n b q k b n r
info string 7 p p p p p p p p
info string 6 . . . . . . . .
info string 5 . . . . . . . .
info string 4 . . . . . . . .
info string 3 . . . . . . . .
info string 2 P P P P P P P P
info string 1 R N B Q K B N R
info string + a b c d e f g h
info string fen: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
info string key: <16 hexadecimal digits>
info string checkers: -
```

Not offered: `debug`, `register`, `ponderhit`. They are answered with
`info string unknown command: <line>`, which echoes the line.

### `go` parameters

| Parameter | Status | Behaviour |
| --- | --- | --- |
| `wtime`, `btime`, `winc`, `binc` | implemented | The clocks and increments in milliseconds. The move gets `remaining / moves + 3/4 increment`, where `moves` is `movestogo` or 30, and never more than is on the clock less the `Move Overhead` (30 ms by default). A clock at or below the overhead gets no time: the move comes from the first search depth alone. A negative clock counts as zero. |
| `movestogo` | implemented | Moves to the next time control. |
| `movetime` | implemented | Think this long, less the `Move Overhead`. Takes precedence over the clocks. |
| `depth` | implemented | Search this many plies and no deeper. Reproducible. |
| `nodes` | implemented | Stop after about this many positions. Reproducible. The first depth is always completed, even beyond the limit, so that there is a move to play. |
| `infinite` | implemented | Search until `stop`. The clocks are ignored. `bestmove` is sent only after `stop`, even if the search has nothing left to look at. A `go` with no parameters at all behaves the same way (also one left with none once what is not offered is ignored, such as `go ponder`). |
| `searchmoves` | implemented | Search only the moves that follow, up to the next parameter: `go depth 12 searchmoves e2e4 d2d4`. The answer is the best of them. A word that is not a legal move of the position is reported with `info string` and left out; if none is left, every move is searched. A restricted search does not record the position's result in the hash table, where it would pass for the result of all moves. Not used with `UCI_LimitStrength` (said with `info string`), and `Variety` does not apply to a restricted search. |
| `mate` | implemented | `go mate 3` searches for a mate in 3 moves or fewer for the side to move and answers as soon as an iteration shows one, however deep it had to look. It is one more limit: with `depth`, `nodes`, `movetime` or the clocks the search also ends where they say, and with none of them it goes on until that mate is found, until the search has proved a forced mate within its depth (a longer one than asked for, or one against the side to move: there is nothing more to find then, and every search ends there), or until `stop` comes. A mate that is merely seen against the side to move, or a longer one, ends nothing early. With `UCI_LimitStrength` no mate is looked for: the weakened search does its passes and answers, and a `go mate` alone does not wait for `stop`. The engine's search is selective, so it does not prove that there is no mate in that many moves. |
| `ponder` | not offered | Ignored, as the protocol asks for unknown words. |

Several limits may be combined; the search stops at whichever comes first.

### The first depth

**The first depth is completed** whatever the clock or `stop` say, so that
there is always a move to play, unless it is a very long one. Normally it
takes well under a millisecond and a few thousand positions at most. The
rules:

- Nothing ends the search before the **first root move** of the first
  depth has been searched: until then `stop`, `quit` and the clock are
  not obeyed and, because the engine waits for the search to end before
  it reads on, nothing else is answered either.
- After that first move `stop` and the clock are looked at once every
  65,536 positions (some tens of milliseconds) for the rest of the first
  depth. An ordinary first depth is over long before the first look. A
  long one is cut short there: the move played is the best of the root
  moves searched so far, and no `info depth 1` line is printed for it.
- A node limit never cuts the first depth short, so `go nodes` stays
  reproducible.

A first depth is long only where very many captures are available,
because the quiescence search plays them out: in an artificial position
with nine queens a side it takes about five seconds, and its first move
about 0.3 seconds. Up to 0.1.0 the whole first depth was always
completed. What is left is known issue KI-10 in [ROADMAP.md](ROADMAP.md):
the first root move, and the weakened search (`UCI_LimitStrength`), whose
first pass over all the moves still always completes.

### Output

- `info depth <d> seldepth <s> score <cp x \| mate n> nodes <n> nps <n> hashfull <n> time <ms> pv <moves>`
  after every completed depth. `nodes` and `time` count from the start of the
  search. `seldepth` is the selective depth: how many plies from the root
  the deepest position of that iteration was, the captures played out
  beyond the depth included. So it is usually at least the depth; it is
  less when every line ends sooner in a draw by rule (too little material,
  a repetition, the fifty-move rule) or a mate. It is left out
  when there is none to give: the position has no legal move, or the line
  is for a weakened move (see "Limited strength"). The extra line for a
  varied move (see `Variety`) repeats that of the search's last depth.
  `hashfull` is how full the transposition table is, in slots per
  thousand, counted over its first thousand slots. Nothing in the table
  ages yet, so the figure rises through a game and goes back to 0 at
  `ucinewgame` or with a new `Hash` size; it is not the share of entries
  that are of use to the current search. It is left out for a weakened
  move, whose search uses no table. `score` is from the side to move's point of view; `mate n` counts
  the engine's own moves and is negative when the engine is being mated.
- `bestmove <move>` at the end. A position with no legal move (checkmate or
  stalemate) gives `bestmove 0000`.
- `info string <text>` for anything the engine wants to say that is not
  part of the protocol, such as a refused command.

### Bad input

Nothing a GUI sends can make the engine play an illegal move or crash
(`tests/uci_robustness.rs` checks both on generated input, see
[TESTING.md](TESTING.md)).

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
| `Hash` | spin | `16` (1 to 65536) | implemented | Transposition table size in MiB. The table holds 65,536 positions to the MiB, and their number is a power of two, so a size that is not one is rounded down. A new size takes effect at the next search and starts with an empty table; if there is no memory for it, the engine says so with `info string` and keeps the table it has. The table is emptied by `ucinewgame` and by `Clear Hash`, and is used by full-strength play only. |
| `Clear Hash` | button | none | implemented | `setoption name Clear Hash` empties the transposition table, as `ucinewgame` does, without starting a new game: the next search knows nothing of the ones before it. Like every option it applies from the next search on, so pressed during a search it leaves that search alone. It has no value, and one sent with it is not looked at. Nothing is answered. |
| `Threads` | spin | `1` (1 to 1) | implemented | Number of search threads. The search is single-threaded, so 1 is the only value: `Threads=1` from a match tool is accepted, and a larger number is refused with an `info string`. The maximum rises when a parallel search exists. |
| `Seed` | spin | `0` (0 to 2147483647) | implemented | Seed of strength-affecting randomness. `0` takes a new seed from system entropy when the engine starts and at every `ucinewgame`; any other value is that seed. Setting it back to `0` takes a fresh entropy seed. The maximum is the largest signed 32-bit number, which Cute Chess can store; entropy seeds stay from 1 to it, so a logged seed can always be given back. It replaces `DeterministicRNG` and `RngSeed` (0.1.0-alpha.1). |
| `Variety` | spin | `0` (0 to 100) | implemented | Centipawns. In the first 12 full moves, at full strength, play at random among the best move and the moves a second search checks to be within this many centipawns of it, weighted towards the best. Never when the best score is a mate. `0` always plays the best move. It weakens play on purpose. See "Variety" below. |
| `EvalFile` | string | to be decided | planned (milestone 9) | Path to an NNUE network file. |
| `UseNNUE` | check | to be decided | planned (milestone 9) | Select the NNUE or the hand-crafted evaluation. |
| `UCI_LimitStrength` | check | `false` | implemented, **calibrated provisionally** | Standard UCI option. When `true`, play at the rating in `UCI_Elo` instead of full strength. When `false`, play is unchanged, down to the node count. See "Limited strength" below. |
| `UCI_Elo` | spin | `1360` (1110 to 1630) | implemented, **calibrated provisionally** | Standard UCI option. Target rating used when `UCI_LimitStrength` is `true`; ignored otherwise. A rating on the scale of the reference ladder (see "Limited strength" below), not a Lichess, CCRL or FIDE rating. The range is what the weakening was measured to cover; the mapping is fitted to seven measured settings. Provisional: measured for 0.1.0-alpha.2 and measured again after milestone 7 and after queued task 2. |
| `Move Overhead` | spin | `30` (0 to 5000) | implemented | Milliseconds kept back from every time limit, for delays the engine cannot see. See "Move Overhead" below. |
| `UCI_ShowWDL` | check | `false` | implemented, **a model, not a measurement** | Standard UCI option. When `true`, every `info` line of a search gives `wdl <win> <draw> <loss>` after the score: chances per thousand for the side to move. See "Win, draw and loss" below. It changes nothing about the search. |
| `SearchStats` | check | `false` | implemented, a diagnostic | When `true`, a full-strength search sends its statistics as `info string stats ...` lines just before `bestmove`: the nodes each depth took, the effective branching factor, how often the first move tried gave the cutoff, and how often the transposition table knew a position or settled it. They are explained under "Running a search by hand" in [TESTING.md](TESTING.md), where `vyvrael search --stats` prints the same lines. It changes nothing about the search, which counts these in any case; a weakened search (`UCI_LimitStrength`) sends none, and with `Variety` the checks of a varied move are not counted. Leave it off in matches: it only fills their logs. |
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

### Win, draw and loss

With `UCI_ShowWDL` on, an `info` line reads

```text
info depth 3 seldepth 6 score cp 37 wdl 164 751 85 nodes 746 nps 2500000 hashfull 0 time 1 pv d2d4 d7d5 b1c3
```

**The three numbers are a model, not a measurement.** They are a fixed
function of the centipawn score, of the shape engines usually use: one
logistic curve for a win, its mirror image for a loss, and the draw is
what is left.

```text
win  = 1000 / (1 + e^((200 - score) / 100))
loss = 1000 / (1 + e^((200 + score) / 100))
draw = 1000 - win - loss
```

each rounded to a whole number, for the side to move. So a level score
reads 119 762 119, a pawn up (100) reads 269 684 47, and at 200 a win is
given half the time. A forced mate is 1000 0 0, or 0 0 1000 for the side
being mated.

The constants 200 and 100 were chosen, not fitted. Nobody has counted how
Vyvræl's games end from each score, so the numbers are not a win rate of
this engine against any opponent, and a GUI that shows them as
percentages shows the model. They would also be off in scale after any
change to what a centipawn means in the evaluation (a tuned or neural
evaluation, for example). The option exists because GUIs and lichess-bot
can display the numbers; it is off by default. The figures are the same
on every platform.

The lines of a weakened move and of a varied move give the chances of
the score they report, like any other line.

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
- Once the clock is at or below the overhead no time is planned at all:
  the search completes its first depth (see "The first depth" above)
  and plays from that. The overhead is a reserve that the plan does not spend. (Up
  to 0.1.0 a move always got at least a millisecond, which spent it; see
  known issue KI-12 in [ROADMAP.md](ROADMAP.md).) A `movetime` at or
  below the overhead still gets one millisecond.
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
  completes, whatever the clock says (known issue KI-10).
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
