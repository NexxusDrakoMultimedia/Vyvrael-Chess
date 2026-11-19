# UCI interface

This document lists every UCI command and option Vyvræl supports, with
defaults and behaviour. It is updated in the same commit as any change to the
protocol surface.

## Current status

**No UCI command is implemented yet.** The UCI loop is milestone 5. Run with
no arguments, the binary prints its name and version to stderr, reads no
input and writes nothing to stdout. The only commands it has today are
`perft`, `eval` and `help`, which are not part of UCI; see
[TESTING.md](TESTING.md).

The tables below record the planned surface so that the plan is reviewable.
The "Status" column is the truth: only rows marked *implemented* work.

## Identity

| Field | Value |
| --- | --- |
| `id name` | `Vyvræl <version>` (ASCII fallback: `Vyvrael <version>`) |
| `id author` | `Nexxus Drako Multimedia` |

## Commands

| Command | Status | Behaviour |
| --- | --- | --- |
| `uci` | planned | Print `id` lines, the option list, then `uciok`. |
| `isready` | planned | Reply `readyok` once initialisation is complete. |
| `ucinewgame` | planned | Clear the transposition table and other per-game state. |
| `position [startpos \| fen <fen>] [moves ...]` | planned | Set up the position and play the listed moves. |
| `go` with `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite` | planned | Start a search on a separate thread; finish with `bestmove`. |
| `stop` | planned | Stop the search as soon as possible and print `bestmove`. |
| `setoption name <name> value <value>` | planned | Set one of the options below. |
| `quit` | planned | Stop any search and exit. |

## Options

| Option | Type | Default | Status | Behaviour |
| --- | --- | --- | --- | --- |
| `Hash` | spin | to be decided | planned | Transposition table size in MiB. |
| `Threads` | spin | `1` | planned | Number of search threads. |
| `DeterministicRNG` | check | `false` | planned | When `false`, strength-affecting randomness is seeded from system entropy at startup and the chosen seed is logged with `info string`. When `true`, it is seeded from `RngSeed`. |
| `RngSeed` | spin | `822286742` (`0x31031996`) | planned | Seed for strength-affecting randomness when `DeterministicRNG` is `true`. |
| `EvalFile` | string | to be decided | planned (milestone 8) | Path to an NNUE network file. |
| `UseNNUE` | check | to be decided | planned (milestone 8) | Select the NNUE or the hand-crafted evaluation. |
| `UCI_LimitStrength` | check | `false` | planned (milestone 5) | Standard UCI option. When `true`, play at the rating in `UCI_Elo` instead of full strength. When `false`, play is unchanged. |
| `UCI_Elo` | spin | to be decided by calibration | planned (milestone 5, calibrated in milestone 6) | Standard UCI option. Target rating used when `UCI_LimitStrength` is `true`. Minimum, maximum and default come from measured strength. |
| `SyzygyPath` | string | empty | optional (milestone 9) | Path to Syzygy tablebases, only if that milestone is approved. |

Strength limiting uses randomness to weaken play, so it is governed by
`DeterministicRNG` and `RngSeed`: non-deterministic by default, reproducible
when `DeterministicRNG` is `true`. See "Planned features" in
[ROADMAP.md](ROADMAP.md).

`DeterministicRNG` and `RngSeed` only govern *strength-affecting*
randomness. Zobrist keys, magic numbers and similar tables are always
generated from the fixed default seed and are not affected by these options.
See the randomness table in [ARCHITECTURE.md](ARCHITECTURE.md).

## Output rules

- stdout carries UCI protocol output only, flushed after every line.
- Diagnostics go to stderr or to `info string` lines.
