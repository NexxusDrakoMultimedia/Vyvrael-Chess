# Results: Vyvræl 0.1.0

The measurement for the 0.1.0 release: a gauntlet of an `avx2` build of
the release's source against the same rungs of
[Ladder Baseline 2](ladder-Baseline-2.md) as the KI-8 measurement of
0.1.0-alpha.2 ([results-0.1.0-alpha.2.md](results-0.1.0-alpha.2.md)),
rated together with Baseline 2's games in one Ordo run. It is the rating
the 0.1.0 release notes cite.

> **Read this as an ordering, not a rating.** Vyvræl 0.1.0 scores 49.5%
> against Rodent IV-2000 and 48% against 0.1.0-alpha.2, and 30% against
> Stockfish 19-1800, and Ordo places it level with those two and below
> Stockfish 19-1800. The number is on Baseline 2's stretched scale
> (Stockfish 19 = 0), not CCRL, Lichess or FIDE Elo, and its error against
> the anchor is about ±225. Do not quote it as an Elo.

- **Build:** `Vyvrael 0.1.0` in the games: an `avx2` build
  (`-C target-cpu=x86-64-v3`, `--locked`, Rust 1.100.0 as pinned in
  `rust-toolchain.toml`, local GNU target; the workflow builds for Windows
  with MSVC) of commit `b048c21` on the branch
  `release/0.1.0`, built by hand on the Windows workstation as
  `vyvrael-0.1.0-avx2-b048c21.exe`, SHA-256
  `3166f9ec1310ea644653a97d4592e1d8adccefb6d4253325dd79b0ac29b14a7f`.
  It is the same source and build level as the released `avx2` file, not
  the workflow's binary itself. Its search is that of 0.1.0-alpha.2: the
  bench counts 16,611,416 nodes for both.
- **Date / host:** 2026-11-22, 19:57 to 20:54 UTC (57 minutes), the
  Windows workstation; `scripts/windows/match.ps1` from the clean commit
  `b048c21`
- **Games:** 1,000 Vyvræl games, merged with Baseline 2's 1,740 games for
  rating
- **Raw games:** not committed (`*.pgn` is git-ignored); kept locally with
  the run notes, log and the copy of the engine list
- **SHA-256 of the PGN** (`2026-11-23-gauntlet-Vyvrael-0.1.0-rel.pgn`,
  1,000 games, 2,037,360 bytes):
  `4e9eb0cbbfdc4cc58bf8c808a2484f0185e79b452a5491e27589a04d13a38495` (the
  same value is in the run notes)
- **Ordo output:** [`results-0.1.0-ratings.txt`](results-0.1.0-ratings.txt)
  (copied unchanged)

## Setup

The same as the KI-8 measurement, with two differences: the opening order
(`-Seed 20261123`, so different openings) and 0.1.0-alpha.2 in place of
0.1.0-alpha.1 among the opponents.

| Item | Value |
|---|---|
| Format | Gauntlet, Vyvræl against each opponent; 2 games per opening with sides swapped, 50 rounds, 100 games per opponent |
| Command | `match.ps1 gauntlet -Test <avx2 exe> -TestName 'Vyvrael 0.1.0' -Opponent <the ten below> -Rounds 50 -Seed 20261123 -Deterministic -Label rel`; the full `cutechess-cli` command is in the run notes |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd`, random order fixed by `-srand 20261123`; no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB for every engine (`-each`), Threads 1 where the engine has it; Vyvræl with `Seed=822286742` and `Variety=0` (`-Deterministic`), which only records the settings: at full strength with `Variety` 0 the engine draws no random numbers |
| Opponent options | The same engine list as the KI-8 measurement; as there, `-each` sends Nostradamus 0.4 a `Hash` of 64, outside its range of 0 to 0 |
| Concurrency | 6 games at once on 8 physical cores |
| Tools | `cutechess-cli` 1.5.1; Ordo 1.2.6 (`ordo-win64.exe`, SHA-256 `219be7081e63653c403ac4a0d8c0eecbab31116f4f67028d9d817b8290c9c6eb`), both installed by hand |
| Failures | Vyvræl: no illegal moves, crashes, stalls or time losses; no engine lost on time. `cutechess-cli` ended one engine process itself between games (CT800 1.46-1800), with no result affected. 654 games ended by adjudication, 295 by mate and 51 by a normal draw |

## Result: Vyvræl against each opponent

| Opponent | W | D | L | Score |
|---|--:|--:|--:|--:|
| CT800 1.46-1500 | 99 | 0 | 1 | 99.0/100 (99.0%) |
| CT800 1.46-1800 | 96 | 0 | 4 | 96.0/100 (96.0%) |
| Nostradamus 0.4 | 86 | 13 | 1 | 92.5/100 (92.5%) |
| Stockfish 19-1320 | 90 | 2 | 8 | 91.0/100 (91.0%) |
| Stockfish 19-1500 | 69 | 1 | 30 | 69.5/100 (69.5%) |
| Rodent IV-2000 | 44 | 11 | 45 | 49.5/100 (49.5%) |
| Vyvrael 0.1.0-a2 | 40 | 16 | 44 | 48.0/100 (48.0%) |
| Stockfish 19-1800 | 29 | 2 | 69 | 30.0/100 (30.0%) |
| Stockfish 19-2000 | 14 | 5 | 81 | 16.5/100 (16.5%) |
| Rustic Alpha 2.4 | 11 | 8 | 81 | 15.0/100 (15.0%) |
| **Total** | 578 | 58 | 364 | 607.0/1000 (60.7%) |

## Result (Ordo)

Command, through `scripts/windows/rate.ps1` on `Baseline2.pgn` and the
gauntlet's PGN: `ordo -a 0 -A "Stockfish 19" -W -s 500 -P <list> -o ... -c
...` (Ordo 1.2.6). Ratings are relative to Stockfish 19 = 0.

Rows near Vyvræl (the full table is in the ratings file):

| Engine | Rating | Error | Points | Played |
|---|--:|--:|--:|--:|
| Stockfish 19-2000 | -1452.6 | 229.1 | 136.5 | 216 |
| Rustic Alpha 2.4 | -1456.1 | 228.4 | 136.0 | 216 |
| Stockfish 19-2200 | -1457.5 | 236.4 | 54.0 | 116 |
| Rustic Alpha 3.0.6 | -1465.8 | 236.5 | 53.5 | 116 |
| Stockfish 19-1800 | -1547.8 | 226.0 | 121.5 | 216 |
| Rustic Alpha 1.5 | -1592.7 | 234.4 | 46.0 | 116 |
| Vyvrael 0.1.0-a2 | -1707.3 | 229.8 | 52.0 | 100 |
| **Vyvrael 0.1.0** | **-1721.4** | **226.1** | **607.0** | **1000** |
| Rodent IV-2000 | -1733.8 | 227.5 | 86.5 | 216 |
| Rodent IV-1600 | -1840.6 | 234.0 | 33.0 | 116 |
| Stockfish 19-1500 | -1880.1 | 230.2 | 60.0 | 216 |

On the `UCI_Elo` scale (Ordo + 3,500) that is 1,779.

## How to read this

- **The same search measured twice.** 0.1.0 searches exactly as
  0.1.0-alpha.2 does, which the KI-8 measurement rated at -1665.5 (1,835
  on the scale) and this one at -1721.4 (1,779), 56 points apart. The two
  runs differ in their openings and in one opponent, and each opponent
  plays only 100 games: against Stockfish 19-2000 the score went from 27%
  to 16.5%, against Stockfish 19-1800 from 41.5% to 30%. So the 56 points
  are the spread between two such runs, not a change in the engine, and
  a difference of that size between two builds is not evidence of
  anything without an SPRT.
- **Head to head with 0.1.0-alpha.2,** the same search scored 48% in 100
  games, as it should within the noise.
- **Use the order, not the magnitudes.** The scale is stretched by
  lopsided results; the ±225 is the error against the far-off anchor.
- **Speed.** Median depth 4 (mean 4.3) at a median 0.26 s a move, over
  36,614 moves: the same as 0.1.0-alpha.2. The `avx2` build is not
  measurably faster at this depth.
- **Not comparable with results from another machine or other settings.**
