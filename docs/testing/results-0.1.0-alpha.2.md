# Results: Vyvræl 0.1.0-alpha.2

The milestone 6 measurement of Vyvræl (known issue KI-8): a gauntlet of
0.1.0-alpha.2 against the core and nearby rungs of
[Ladder Baseline 2](ladder-Baseline-2.md), rated together with Baseline 2's
games in one Ordo run. It is the build before any move ordering,
transposition table or pruning (known issues KI-9 and KI-10), and it will be
measured again after milestone 7.

> **Read this as an ordering, not a rating.** Vyvræl scores 41.5% against
> Stockfish 19-1800 and 60.5% against Rodent IV-2000, and is placed between
> them; against 0.1.0-alpha.1 it scores 54%, which is within the noise. The Ordo number is on Baseline 2's
> stretched scale (Stockfish 19 = 0), not CCRL, Lichess or FIDE Elo, and its
> error against the anchor is about ±220. Do not quote it as an Elo.

- **Build:** `Vyvrael 0.1.0-a2` in the games: the 0.1.0-alpha.2 release
  binary, `vyvrael-0.1.0-alpha.2.exe`, SHA-256
  `1b0d518f26fcbb876f747449422c9d32ea249db0223f3ddec319ac4297c81ad3`. It is
  also the current build of milestone 6: no file under `src/` has changed
  since the tag `v0.1.0-alpha.2`, and a build of the branch is byte for byte
  the same binary.
- **Date / host:** 2026-11-22, 05:25 to 06:23 UTC (58 minutes), the
  Windows workstation; `scripts/windows/match.ps1` from the clean commit
  `269a1f5`
- **Games:** 1,000 Vyvræl games, merged with Baseline 2's 1,740 games for
  rating
- **Raw games:** not committed (`*.pgn` is git-ignored); kept locally with
  the run notes, log and the copy of the engine list
- **SHA-256 of the PGN** (`2026-11-22-gauntlet-Vyvrael-0.1.0-a2-ki8.pgn`,
  1,000 games, 2,068,805 bytes):
  `e20b756c760333d15ec42844f2ef6ccde37ef02cb44edd209ef46845ca9f23fc` (the
  same value is in the run notes)
- **Ordo output:** [`results-0.1.0-alpha.2-ratings.txt`](results-0.1.0-alpha.2-ratings.txt)
  (copied unchanged)

## Setup

Baseline 2's fixed settings, so the games can be rated together.

| Item | Value |
|---|---|
| Format | Gauntlet, Vyvræl against each opponent; 2 games per opening with sides swapped, 50 rounds, 100 games per opponent |
| Command | `match.ps1 gauntlet -Test <alpha.2 exe> -TestName 'Vyvrael 0.1.0-a2' -Opponent <the ten below> -Rounds 50 -Seed 20261122 -Deterministic -Label ki8`; the full `cutechess-cli` command is in the run notes |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd`, random order fixed by `-srand 20261122`; no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB for every engine (`-each`), Threads 1 where the engine has it; Vyvræl with `Seed=822286742` and `Variety=0` (`-Deterministic`), which only records the settings: at full strength with `Variety` 0 the engine draws no random numbers, so the moves are the same without it |
| Opponent options | As in Baseline 2: every opponent entry in the engine list was the same as Baseline 2's copy, checked entry by entry before the run, and the SHA-256 of the nine Baseline 2 opponents' binaries in the run notes match Baseline 2's. One difference: Nostradamus 0.4's `Hash` option has the range 0 to 0, and `-each` sent it 64, where Baseline 2 left it at 0; what the engine does with a value outside its range is not known. `Vyvrael 0.1.0-a1` is the 0.1.0-alpha.1 release binary (SHA-256 `ed2209de016c80979bce46b3b9e094984e1b309f2d737e26efd30cde9411b046`), not a Baseline 2 engine |
| Concurrency | 6 games at once on 8 physical cores |
| Tools | `cutechess-cli` 1.5.1; Ordo 1.2.6 (`ordo-win64.exe`, SHA-256 `219be7081e63653c403ac4a0d8c0eecbab31116f4f67028d9d817b8290c9c6eb`, the binary Baseline 2 was rated with) |
| Failures | Vyvræl: no illegal moves, crashes, stalls or time losses. One time loss by an opponent (Nostradamus 0.4, game 574). See "Engine terminated after a game" below. 631 games ended by adjudication, 368 by mate (295) or a normal draw (73), and the one on time |

Strength-limited opponents are one binary with `UCI_LimitStrength` on and a
nominal `UCI_Elo` equal to the name suffix. The number is a label, not a
measured rating (see the baseline).

## Result: Vyvræl against each opponent

| Opponent | W | D | L | Score |
|---|--:|--:|--:|--:|
| CT800 1.46-1500 | 99 | 1 | 0 | 99.5/100 (99.5%) |
| CT800 1.46-1800 | 95 | 4 | 1 | 97.0/100 (97.0%) |
| Stockfish 19-1320 | 93 | 0 | 7 | 93.0/100 (93.0%) |
| Nostradamus 0.4 | 76 | 21 | 3 | 86.5/100 (86.5%) |
| Stockfish 19-1500 | 68 | 1 | 31 | 68.5/100 (68.5%) |
| Rodent IV-2000 | 56 | 9 | 35 | 60.5/100 (60.5%) |
| Vyvrael 0.1.0-a1 | 46 | 16 | 38 | 54.0/100 (54.0%) |
| Stockfish 19-1800 | 37 | 9 | 54 | 41.5/100 (41.5%) |
| Stockfish 19-2000 | 24 | 6 | 70 | 27.0/100 (27.0%) |
| Rustic Alpha 2.4 | 11 | 10 | 79 | 16.0/100 (16.0%) |
| **Total** | 605 | 77 | 318 | 643.5/1000 (64.4%) |

The total score depends on which opponents were chosen and says nothing on
its own.

## Result (Ordo)

Command, through `scripts/windows/rate.ps1` on `Baseline2.pgn` and the
gauntlet's PGN:
`ordo -a 0 -A "Stockfish 19" -W -s 500 -P <list of the two files> -o ... -c ...`
(Ordo 1.2.6). Ratings are relative to Stockfish 19 = 0. Ordo rated the
merged games without complaint (it refuses games that are not connected,
as in the smoke test), and no engine has a score of 0 or 100%. Ordo's
console output was not kept.

Rows near Vyvræl (the full table is in the ratings file):

| Engine | Rating | Error | Points | Played |
|---|--:|--:|--:|--:|
| Rustic Alpha 2.4 | -1428.7 | 226.3 | 135.0 | 216 |
| Stockfish 19-2000 | -1484.2 | 225.2 | 126.0 | 216 |
| Stockfish 19-1800 | -1571.6 | 224.1 | 110.0 | 216 |
| Rustic Alpha 1.5 | -1587.0 | 233.0 | 46.0 | 116 |
| **Vyvrael 0.1.0-a2** | **-1665.5** | **223.5** | **643.5** | **1000** |
| Vyvrael 0.1.0-a1 | -1693.7 | 229.0 | 46.0 | 100 |
| Rodent IV-2000 | -1746.7 | 226.9 | 75.5 | 216 |
| Rodent IV-1600 | -1820.2 | 230.6 | 33.0 | 116 |
| Stockfish 19-1500 | -1827.8 | 228.1 | 61.0 | 216 |
| Stockfish 19-1320 | -2050.8 | 232.2 | 32.0 | 216 |
| Nostradamus 0.4 | -2056.0 | 232.4 | 31.5 | 216 |

Rustic Alpha 1.5 and Rodent IV-1600 have no games against Vyvræl; their
places come from the baseline alone.

The merged run moves Baseline 2's own engines by up to about 90 points,
and changes some of their order: Ordo refits the whole pool, and the
opponents gain 100 games each. Stockfish 19-1800 was at -1493.5 in
Baseline 2 alone and Stockfish 19-1320 at -1969.6; Rustic Alpha 2.4 moves
up 73 points, past both Stockfish 19-2000 and -1800, and CT800 1.46-1500
drops 92 points, below Arasan 26.1-1800. Compare Vyvræl with the engines
of this table, not with the numbers in the baseline file.

## Engine terminated after a game

Once in the 1,000 games, `cutechess-cli` logged
`Terminating process of engine Vyvrael 0.1.0-a2(1931)`, just before the
result of game 971 (Vyvræl against CT800 1.46-1500, won by Vyvræl by
adjudication). The result was recorded normally and the other games ran as
usual. The message says that cutechess-cli ended the engine's process
itself, which it is expected to do when an engine does not exit after
`quit` (an interpretation of the tool, not checked in its source). The log
does not say why. In the `UCI_Elo` calibration runs the same message was
logged once each for a limited Vyvræl, CT800 1.46-1500 and Stockfish
19-1500, so it is not particular to Vyvræl (see "Calibration of `UCI_Elo`"
in [TESTING.md](../TESTING.md)). The soak test of milestone 6 looks for it
at other time controls.

## How to read this

- **Use the order, not the magnitudes.** The scale is stretched because
  many pairings in the baseline are lopsided. The ±220 is the error against
  the anchor, Stockfish 19, which is far above. Vyvræl's order against
  Stockfish 19-1800 and Rodent IV-2000 rests on its head-to-head scores
  (41.5% and 60.5% in 100 games each), not on that error; no error between
  neighbours was computed.
- **The informative opponents** are the ones Vyvræl scores 25 to 75%
  against: Stockfish 19-1500 (68%), Rodent IV-2000 (60%),
  0.1.0-alpha.1 (54%), Stockfish 19-1800 (42%) and Stockfish 19-2000 (27%).
  The CT800 entries, Stockfish 19-1320 and Nostradamus 0.4 (86 to 100%) and
  Rustic Alpha 2.4 (16%) add little.
- **Against 0.1.0-alpha.1,** 54% in 100 games is within the noise of a
  sample this size (about ±9% at 95%). It shows no gain or loss between
  the two alphas, and none is claimed.
- **No repeated games.** The engine is deterministic at full strength, so
  the same opening against the same opponent with the same colours could
  repeat a game. None did: the PGN has 500 different starting positions,
  each played exactly twice, as the colour-swapped pair of one encounter.
- **Speed.** Vyvræl searched to a median depth of 4 (mean 4.3) at a median
  0.26 s a move, over its 37,259 moves. It is slow for the time it uses
  (KI-9).
- **Not comparable with results from another machine or other settings.**
  Hash, threads, time control, openings and concurrency were fixed; this
  file and [`results-0.1.0-alpha.1.md`](results-0.1.0-alpha.1.md) are on
  different baselines and their numbers must not be compared.

## Opponent pool for the next gauntlets

From the scores above, for builds near this one (milestone 7 will need a
stronger pool):

| Role | Engines |
|---|---|
| Core | Stockfish 19-1500, Rodent IV-2000, Stockfish 19-1800, Stockfish 19-2000, Vyvrael 0.1.0-a2 |
| Floor | Nostradamus 0.4, Stockfish 19-1320 |
| Next up | Rustic Alpha 2.4 (16%); Vice 1.1 and Rustic Alpha 3.0.6 once Stockfish 19-2000 is near 50% |
| Drop for now | CT800 1.46-1500 and -1800 (97 to 100%) |

## What this is used for

- KI-8, for this build: the hand-crafted evaluation and the plain search
  together play at the level shown above.
- The baseline for milestone 7, whose heuristics are each accepted by SPRT;
  this file is what the next measurement is compared with on the same
  ladder.
- `UCI_Elo` is **not** calibrated by this file; that is its own measurement
  in milestone 6.
