# Ladder Baseline 1

First opponent-ladder measurement for Vyvræl testing. Vyvræl itself was **not** in this run; the goal was to check the tooling and order the opponents.

> **Vyvræl is not in this baseline and has no rating here.** This is a
> reference ladder of 25 *other* engines, run before Vyvræl had a UCI
> interface. The Ordo ratings below are relative to the anchor engine
> (Stockfish 19 = 0) and describe only those 25 engines. Vyvræl's own
> strength will be measured later (milestone 6, known issue KI-8) by
> gauntlets against the lower rungs, and recorded in a separate results
> file. It must not be inserted into this file or inferred from it.

- **Date / host:** 2026-11-20, the Windows workstation (Windows 11, Ryzen 7 5700X), Cute Chess GUI, 13:44-14:30 AEST (about 46 min)
- **Raw games:** `Baseline1.pgn` (600 games, 1.1 MB, **not committed**; kept locally in `matches/`)
- **SHA-256 of the PGN:** `a9cb2fbb3cab148125f1c92db7bc500863931248aa8ecc0c41953a186b409c14` (of the maintainer's local copy of `Baseline1.pgn`, which is not committed; `*.pgn` is git-ignored)
- **Ordo output:** [`ladder-Baseline-1-ratings.txt`](ladder-Baseline-1-ratings.txt), [`ladder-Baseline-1-ratings.csv`](ladder-Baseline-1-ratings.csv)

## Setup

| Item | Value |
|---|---|
| Format | Round Robin, 1 round, 2 games per encounter, swap sides, each opening played twice |
| Games | 600 (25 engines, 300 pairings, 48 games per engine) |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd`, random order, 1 ply (FEN positions); no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB, Threads 1 (as configured; not stored in the PGN) |
| Outcomes | 295 White wins, 287 Black wins, 18 draws; 559 games ended by adjudication |
| Failures | none (no crashes, timeouts, illegal moves or stalls) |

Strength-limited entries are one binary with a nominal `UCI_Elo` equal to the name suffix and `UCI_LimitStrength` on: Stockfish 19-1320/1500/1800/2000/2200/2400, Arasan 26.1-1500/1800, Rodent IV-1600/2000. The number is a label, not a measured rating.

TSCP and Micro-Max were left out: they are XBoard-only and cannot start from FEN openings, so an earlier run (`smoke1.pgn`) was invalid and discarded.

## Result (Ordo)

Command: `ordo -a 0 -A "Stockfish 19" -W -s 500 -p Baseline1.pgn -o ladder-Baseline-1-ratings.txt -c ladder-Baseline-1-ratings.csv` (Ordo 1.2.6). Ratings are relative to Stockfish 19 = 0, not CCRL or FIDE Elo.

| # | Engine | Rating | Error | Points /48 |
|--:|---|--:|--:|--:|
| 1 | Viridithas 20 | 75.4 | 268.6 | 45.5 |
| 2 | Reckless 0.9.0 | 36.6 | 247.9 | 45.0 |
| 3 | Stockfish 19 | 0.0 | - | 44.5 |
| 4 | Arasan 26.1 | -166.8 | 255.1 | 42.0 |
| 5 | Stash 37.0 | -385.4 | 277.8 | 38.5 |
| 6 | Princhess 0.22.0 | -385.4 | 278.4 | 38.5 |
| 7 | Ethereal 12.75 | -449.0 | 274.7 | 37.5 |
| 8 | Weiss 2.0 | -586.5 | 305.9 | 35.5 |
| 9 | Orion 1.1 | -936.1 | 341.3 | 31.5 |
| 10 | Rodent IV | -1080.9 | 364.8 | 30.0 |
| 11 | Fruit 2.1 | -1181.9 | 403.8 | 29.0 |
| 12 | Stockfish 19-2400 | -1725.0 | 410.1 | 23.5 |
| 13 | Stockfish 19-2200 | -1725.0 | 398.3 | 23.5 |
| 14 | Vice 1.1 | -1876.4 | 422.9 | 21.0 |
| 15 | Rustic Alpha 1.5 | -2025.1 | 422.7 | 18.0 |
| 16 | Rustic Alpha 2.4 | -2114.7 | 437.4 | 16.0 |
| 17 | Stockfish 19-1800 | -2158.5 | 424.7 | 15.0 |
| 18 | Stockfish 19-2000 | -2180.3 | 436.2 | 14.5 |
| 19 | Rustic Alpha 3.0.6 | -2202.2 | 437.4 | 14.0 |
| 20 | Rodent IV-2000 | -2246.5 | 435.7 | 13.0 |
| 21 | Rodent IV-1600 | -2491.8 | 454.0 | 8.0 |
| 22 | Stockfish 19-1500 | -2608.3 | 458.6 | 6.0 |
| 23 | Stockfish 19-1320 | -2608.3 | 462.7 | 6.0 |
| 24 | Arasan 26.1-1800 | -2821.4 | 488.9 | 3.0 |
| 25 | Arasan 26.1-1500 | -3025.4 | 493.8 | 1.0 |

White advantage reported by Ordo: 4.67 ± 15.72.

## How to read this

- **Use the order, not the magnitudes.** Many pairings were lopsided, so Ordo's total spread (about 3000 points) is exaggerated.
- **Many neighbours are tied.** With 48 games per engine the error bars are ±250 to ±500. Stockfish 19, Reckless and Viridithas are indistinguishable; so are Stockfish 19-2200 and -2400, and the Rustic versions are not in version order.
- **Nominal `UCI_Elo` is not strength.** At 10+0.1 the Stockfish entries do not match their labels.
- **Orion 1.1 is mid-table**, not a weak engine.
- Ordo's "draw rate (equal opponents) = 50 %" line looks like a default, not a measurement; the observed draw rate was 3 % (18/600).
- Ordo's error values come from a random simulation (`-s 500`) and may differ slightly when re-run; the ratings themselves should not.

## Using the ladder for Vyvræl

Start with a gauntlet against the lower rungs (Arasan-1500/1800, Stockfish 19-1320/1500, Rodent IV-1600/2000, Vice, Rustic Alpha versions), then move up as Vyvræl improves. Keep earlier Vyvræl builds as opponents. Keep Hash, Threads, time control and openings fixed between runs you want to compare.
