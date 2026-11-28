# Results: Vyvræl 0.2.0

The measurement for the 0.2.0 release: an `avx2` build of the release's
source played in [Ladder Baseline 3](ladder-Baseline-3.md), a round robin
of 44 engines that also holds the three earlier Vyvræl builds. It is the
measurement the 0.2.0 release notes are to cite, and the measurement of
known issue KI-8 after milestone 7.

> **Read this as an ordering, not a rating.** Vyvræl 0.2.0 finished 19th
> of 44 with 92.5 points of 172 (54%), level with Tantabus 1.0.2,
> Stockfish 19-2400 and Stockfish 19-2200 and well above Vyvræl 0.1.0.
> The number is on Baseline 3's stretched scale (Stockfish 19 = 0), not
> CCRL, Lichess or FIDE Elo, and its error against the anchor is about
> ±200. Do not quote it as an Elo. It rests on 172 games, four against
> each opponent: far fewer than the 1,000 games of the earlier results
> files.

- **Build:** `Vyvrael 0.2.0` in the games: an `avx2` build
  (`-C target-cpu=x86-64-v3`, Rust as pinned in `rust-toolchain.toml`,
  local GNU target; the workflow builds for Windows with MSVC) of the
  source of commit `182f9ec` on the branch `release/0.2.0`, built by hand
  on the Windows workstation as `vyvrael-0.2.0-avx2.exe`, SHA-256
  `347a54f1fde36398a9d7d35181970963bf80fa3b7cbb2268cb131686567e095c`.
  It is the same source and build level as the `avx2` release file, not
  the workflow's binary itself. The bench counts 28,175 nodes.
- **Run:** Ladder Baseline 3, 2026-11-28, the Windows workstation. Its
  setup, tools, failures, engine binaries and the SHA-256 of its PGN are
  in [ladder-Baseline-3.md](ladder-Baseline-3.md); they are not repeated
  here.
- **Ordo output:**
  [`ladder-Baseline-3-ratings.txt`](ladder-Baseline-3-ratings.txt)
  (copied unchanged)

## Setup, in short

Round robin, 10+0.1, Hash 64 MB, one thread, openings from
`UHO_Lichess_4852_v1.epd` with sides swapped, the adjudication rules of
the ladder, six games at once. Vyvræl 0.2.0 played with `Seed=822286742`
and `Variety=0` and everything else at its default, as 0.1.0 and
0.1.0-alpha.2 did; 0.1.0-alpha.1 has neither option and played at its
defaults. No Vyvræl build made an illegal move, crashed, stalled or lost
on time in its 172 games.

## Result (Ordo)

Ratings are relative to Stockfish 19 = 0. Rows near the Vyvræl builds (the
full table is in the ratings file):

| # | Engine | Rating | Error | Points /172 |
|--:|---|--:|--:|--:|
| 17 | Blunder 7.1.0 | -1286.7 | 199.1 | 105.0 |
| 18 | MinimalChess 0.6 | -1362.1 | 198.4 | 98.0 |
| 19 | **Vyvrael 0.2.0** | **-1418.8** | **199.1** | **92.5** |
| 20 | Tantabus 1.0.2 | -1428.9 | 195.0 | 91.5 |
| 21 | Stockfish 19-2400 | -1434.0 | 197.8 | 91.0 |
| 22 | Stockfish 19-2200 | -1459.2 | 199.4 | 88.5 |
| 23 | MinimalChess 0.5 | -1474.2 | 196.3 | 87.0 |
| ... | | | | |
| 33 | Rodent IV-2000 | -1967.0 | 204.3 | 42.0 |
| 34 | **Vyvrael 0.1.0** | **-1973.7** | **200.6** | **41.5** |
| 35 | **Vyvrael 0.1.0-a1** | **-1993.9** | **202.4** | **40.0** |
| 36 | **Vyvrael 0.1.0-a2** | **-2007.6** | **203.5** | **39.0** |
| 37 | Stockfish 19-1500 | -2116.8 | 207.6 | 31.5 |

## Vyvræl 0.2.0 against each opponent

Four games each (two openings, sides swapped). In the order of the ladder:

| Opponent | W | D | L | Score /4 |
|---|--:|--:|--:|--:|
| Stockfish 19 | 0 | 0 | 4 | 0.0 |
| Reckless 0.9.0 | 0 | 0 | 4 | 0.0 |
| Viridithas 20 | 0 | 0 | 4 | 0.0 |
| Arasan 26.1 | 0 | 0 | 4 | 0.0 |
| Ethereal 12.75 | 0 | 0 | 4 | 0.0 |
| Princhess 0.22.0 | 0 | 0 | 4 | 0.0 |
| Stash 37.0 | 0 | 0 | 4 | 0.0 |
| Weiss 2.0 | 0 | 0 | 4 | 0.0 |
| Orion 1.1 | 0 | 0 | 4 | 0.0 |
| Rodent IV | 0 | 0 | 4 | 0.0 |
| Inanis 1.0.1 | 0 | 0 | 4 | 0.0 |
| Fruit 2.1 | 0 | 0 | 4 | 0.0 |
| CT800 1.46 | 0 | 0 | 4 | 0.0 |
| Blunder 7.6.0 | 1 | 2 | 1 | 2.0 |
| Tantabus 2.0.0 | 1 | 0 | 3 | 1.0 |
| Velvet 1.0.0 | 1 | 1 | 2 | 1.5 |
| Blunder 7.1.0 | 1 | 1 | 2 | 1.5 |
| MinimalChess 0.6 | 0 | 3 | 1 | 1.5 |
| Tantabus 1.0.2 | 1 | 1 | 2 | 1.5 |
| Stockfish 19-2400 | 1 | 0 | 3 | 1.0 |
| Stockfish 19-2200 | 1 | 1 | 2 | 1.5 |
| MinimalChess 0.5 | 2 | 2 | 0 | 3.0 |
| Blunder 5.0.0 | 1 | 2 | 1 | 2.0 |
| Stockfish 19-2000 | 4 | 0 | 0 | 4.0 |
| Vice 1.1 | 3 | 0 | 1 | 3.0 |
| MinimalChess 0.4.1 | 2 | 1 | 1 | 2.5 |
| Rustic Alpha 3.0.6 | 4 | 0 | 0 | 4.0 |
| Rustic Alpha 2.4 | 4 | 0 | 0 | 4.0 |
| Stockfish 19-1800 | 3 | 0 | 1 | 3.0 |
| Rustic Alpha 1.5 | 4 | 0 | 0 | 4.0 |
| Blunder 4.0.0 | 3 | 1 | 0 | 3.5 |
| Rodent IV-2000 | 4 | 0 | 0 | 4.0 |
| Vyvrael 0.1.0 | 4 | 0 | 0 | 4.0 |
| Vyvrael 0.1.0-a1 | 4 | 0 | 0 | 4.0 |
| Vyvrael 0.1.0-a2 | 4 | 0 | 0 | 4.0 |
| Stockfish 19-1500 | 4 | 0 | 0 | 4.0 |
| Rodent IV-1600 | 4 | 0 | 0 | 4.0 |
| Nostradamus 0.4 | 4 | 0 | 0 | 4.0 |
| Stockfish 19-1320 | 4 | 0 | 0 | 4.0 |
| CT800 1.46-1500 | 4 | 0 | 0 | 4.0 |
| CT800 1.46-1800 | 4 | 0 | 0 | 4.0 |
| Arasan 26.1-1500 | 4 | 0 | 0 | 4.0 |
| CT800 1.46-1200 | 4 | 0 | 0 | 4.0 |
| **Total** | 85 | 15 | 72 | 92.5/172 (53.8%) |

## Against the earlier builds

| Build | Points /172 | Rating | Median depth | Median time a move |
|---|--:|--:|--:|--:|
| Vyvrael 0.2.0 | 92.5 | -1418.8 | 12 | 0.26 s |
| Vyvrael 0.1.0 | 41.5 | -1973.7 | 4 | 0.28 s |
| Vyvrael 0.1.0-a1 | 40.0 | -1993.9 | 4 | 0.28 s |
| Vyvrael 0.1.0-a2 | 39.0 | -2007.6 | 4 | 0.29 s |

- **0.2.0 won all twelve games against the three earlier builds,** and
  scored 92.5 points where they scored 39 to 41.5 against the same 40
  other engines and each other.
- **It searches three times as deep in the same time:** a median depth of
  12 (mean 12.9) over 6,443 moves, against 4 for each earlier build. That
  is the transposition table, move ordering and pruning of milestone 7;
  the evaluation is the same.
- **About 555 points on this ladder's scale** separate 0.2.0 from 0.1.0
  (-1418.8 against -1973.7). That is a distance on a stretched scale, with
  an error of about ±200 on each number: it says "clearly stronger", which
  the twelve games and the points agree with, and no more than that. It is
  not an Elo gain to quote.
- **The three earlier builds are level,** within 2.5 points of 172: 0.1.0
  and 0.1.0-alpha.2 have the same search, and 0.1.0-alpha.1 was level with
  alpha.2 in milestone 6's measurement.

## How to read this

- **Use the order, not the magnitudes.** The scale is stretched by
  lopsided results; the ±200 is the error against the far-off anchor.
- **Neighbours are not separated.** 0.2.0, Tantabus 1.0.2, Stockfish
  19-2400, Stockfish 19-2200 and MinimalChess 0.5 lie within 56 rating
  points and 5.5 game points. Which of them is "really" ahead this run
  does not say.
- **Four games against one opponent measure nothing about that pair.**
  0.2.0 scored 4-0 against Stockfish 19-2000 and 1-3 against Stockfish
  19-2400; read the table above as a picture of where the wins stop, not
  as 43 results.
- **Not on the scale of the earlier results files.** Those are rated with
  Ladder Baseline 2's games: 0.1.0 is at -1721.4 there
  ([results-0.1.0.md](results-0.1.0.md)) and at -1973.7 here, the same
  engine on two ladders. Compare builds within one ladder only.
- **`UCI_Elo` is not measured here.** Every Vyvræl build played at full
  strength. The settings were measured the same day with the same build
  and refitted on this ladder's scale (see "Calibration of `UCI_Elo`
  (0.2.0)" in [TESTING.md](../TESTING.md)); on that scale (Ordo + 3,500)
  full strength is about 2,080, far above the highest setting, 1380.
- **Not comparable with results from another machine or other settings.**
