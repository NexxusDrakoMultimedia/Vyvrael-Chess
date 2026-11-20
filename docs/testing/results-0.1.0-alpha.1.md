# Results: Vyvræl 0.1.0-alpha.1

First strength measurement of Vyvræl, played against the lower rungs of
[Ladder Baseline 1](ladder-Baseline-1.md). It is a pre-milestone-6 snapshot
of the first alpha, before any move ordering, transposition table or
pruning (known issues KI-9 and KI-10), and it will be superseded by the
milestone 6 measurement.

> **Read this as an ordering, not a rating.** Vyvræl sits about level with
> Rodent IV-2000 and clearly below Stockfish 19-1800, Vice 1.1 and the
> Rustic Alpha versions. The Ordo number is on the baseline's stretched
> scale (Stockfish 19 = 0), not CCRL, Lichess or FIDE Elo, and its error is
> about ±400. Do not quote it as an Elo.

- **Build:** the engine named `Vyvrael 0.1.0-a1` in the games. Gauntlet 1
  is the gauntlet of the release check (see "Complete games in
  cutechess-cli" in [TESTING.md](../TESTING.md)): it ran the build of commit
  `1dbe4a1`, which plays as the release does, because the commits between
  it and the tag changed no engine code, only the version it reports.
  Gauntlet 2 used the binary configured in Cute Chess under that name; its
  commit is not recorded in the PGN, where the engine name is the only
  identifier.
- **Date / host:** 2026-11-20 and 2026-11-21, the Windows workstation;
  gauntlet 1 with `cutechess-cli`, gauntlet 2 in Cute Chess
- **Games:** 320 Vyvræl games (two gauntlets), merged with the 600 games of
  the baseline for rating
- **Raw games:** not committed (`*.pgn` is git-ignored)
- **SHA-256 of the merged PGN** (`merged-alpha1.pgn`, 920 games, 1,848,848
  bytes): `aede6b0c3e2e16fd126d9d17335dcb99c08da1e6b46f7c27fa52d5a104071cec`
- **Ordo output:** [`results-0.1.0-alpha.1-ratings.txt`](results-0.1.0-alpha.1-ratings.txt)

## Setup

The same fixed defaults as Ladder Baseline 1, so the two can be combined.

| Item | Value |
|---|---|
| Format | Gauntlet, Vyvræl seeded against the opponents; 2 games per encounter, swap sides, each opening played twice |
| Gauntlet 1 | 2026-11-20, 40 games: Arasan 26.1-1500, Rodent IV-1600, Stockfish 19-1320, Vice 1.1, Rustic Alpha 1.5 (8 games each) |
| Gauntlet 2 | 2026-11-21, 280 games: Rodent IV-2000, Rustic Alpha 3.0.6, Stockfish 19-1800, Stockfish 19-2000, Rustic Alpha 2.4, Vice 1.1, Stockfish 19-1320 (40 games each) |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd`, random order, FEN positions; no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB, Threads 1 |
| Tools | Cute Chess 1.5.1 (Qt 6.8.3, Windows 11 26H2, x86_64; GUI version from its About box; `cutechess-cli` is the same version, confirmed by the maintainer); Ordo 1.2.6 |
| Failures | none recorded: no illegal moves, crashes, time losses or stalls; 248 of the 320 games ended by adjudication, the rest by mate or a normal draw |

Strength-limited opponents are one binary with `UCI_LimitStrength` on and a
nominal `UCI_Elo` equal to the name suffix. The number is a label, not a
measured rating (see the baseline).

## Result: Vyvræl against each opponent

| Opponent | W | D | L | Score |
|---|--:|--:|--:|--:|
| Arasan 26.1-1500 | 8 | 0 | 0 | 8.0/8 (100%) |
| Rodent IV-1600 | 8 | 0 | 0 | 8.0/8 (100%) |
| Stockfish 19-1320 | 41 | 1 | 6 | 41.5/48 (86%) |
| Rodent IV-2000 | 19 | 5 | 16 | 21.5/40 (54%) |
| Stockfish 19-1800 | 11 | 1 | 28 | 11.5/40 (29%) |
| Stockfish 19-2000 | 6 | 1 | 33 | 6.5/40 (16%) |
| Rustic Alpha 2.4 | 4 | 3 | 33 | 5.5/40 (14%) |
| Rustic Alpha 1.5 | 0 | 2 | 6 | 1.0/8 (12%) |
| Rustic Alpha 3.0.6 | 2 | 3 | 35 | 3.5/40 (9%) |
| Vice 1.1 | 3 | 0 | 45 | 3.0/48 (6%) |
| **Total** | 102 | 16 | 202 | 110.0/320 (34%) |

The total score depends on which opponents were chosen and says nothing on
its own.

## Result (Ordo)

Command, on the baseline merged with both gauntlets:
`ordo -a 0 -A "Stockfish 19" -W -s 500 -p merged-alpha1.pgn -o ratings-alpha1.txt -c ratings-alpha1.csv`
(Ordo 1.2.6). Ratings are relative to Stockfish 19 = 0. The baseline's
`Stockfish 19-2400` is written without the trailing space it had in the
original run, in this merged file.

Rows near Vyvræl (the full table is in the ratings file):

| Engine | Rating | Error | Points | Played |
|---|--:|--:|--:|--:|
| Vice 1.1 | -1895.8 | 412.9 | 66.0 | 96 |
| Rustic Alpha 1.5 | -2035.3 | 418.5 | 25.0 | 56 |
| Rustic Alpha 2.4 | -2093.4 | 418.1 | 50.5 | 88 |
| Rustic Alpha 3.0.6 | -2093.4 | 407.1 | 50.5 | 88 |
| Stockfish 19-2000 | -2140.0 | 411.3 | 48.0 | 88 |
| Stockfish 19-1800 | -2214.2 | 405.2 | 43.5 | 88 |
| Rodent IV-2000 | -2383.3 | 410.3 | 31.5 | 88 |
| **Vyvrael 0.1.0-a1** | **-2390.9** | **402.6** | **110.0** | **320** |
| Rodent IV-1600 | -2664.1 | 421.9 | 8.0 | 56 |
| Stockfish 19-1500 | -2704.8 | 447.8 | 6.0 | 48 |
| Stockfish 19-1320 | -2713.0 | 416.8 | 12.5 | 96 |

Stockfish 19-1500 has no games against Vyvræl; its place comes from the
baseline alone.

## How to read this

- **Use the order, not the magnitudes.** The ladder's scale is stretched
  because many pairings were lopsided. Vyvræl's position relative to its
  neighbours is better determined than the ±400 suggests, since that error
  is mostly the baseline's own spread.
- **Only Rodent IV-2000 gave a balanced result (54%).** Opponents Vyvræl
  beats almost always (Arasan-1500, Rodent IV-1600) or loses to almost always
  (Vice 1.1, the Rustic Alpha versions) say little about its strength.
- **Sample size.** Most opponents have 40 games or fewer; the 8-game
  opponents are too few to rate. The first gauntlet's 8-game rows are
  listed for completeness only.
- **Repeated games.** The engine searches deterministically at full
  strength, so the same opening against the same opponent can repeat a game.
  The effective number of independent games is therefore somewhat lower than
  the count above (see queued task 1, `Seed` and `Variety`).
- **Speed.** In these games Vyvræl searched to a median depth of 4 at about
  0.26 s a move. The engine is slow for the time it uses (KI-9).
- **Not comparable with later results unless the settings match.** Hash,
  threads, time control, openings and machine were fixed; results from other
  machines are not directly comparable.

## Opponent pool for the next gauntlets

Chosen from the scores above: an opponent is useful when Vyvræl scores
roughly 25 to 75% against it. Same settings as this measurement (10+0.1,
Hash 64, one thread, `UHO_Lichess_4852_v1.epd`, 40 games per opponent or
more).

| Role | Engine | Why |
|---|---|---|
| Core | Rodent IV-2000 | The only balanced result (54%) |
| Core | Stockfish 19-1800 | Close above (29%); the main rung to watch for gains |
| Optional | Stockfish 19-1500 | Not yet played. On the baseline it is tied with Stockfish 19-1320 within error, so it adds little; include it only for a second floor |
| Core | Stockfish 19-1320 | A floor that is still informative (86%, 6 losses in 48) |
| Hold | Stockfish 19-2000, Rustic Alpha 2.4, Rustic Alpha 3.0.6 | 9 to 16%: too strong for now; bring them back when Vyvræl scores about 25% against them |
| Hold | Vice 1.1, Rustic Alpha 1.5 | 6 to 12%: too strong for now; bring them back after milestone 7 |
| Drop | Arasan 26.1-1500, Rodent IV-1600 | 100% in 8 games each; no information |

Keep earlier Vyvræl builds as opponents too (for example `Vyvrael 0.1.0-a1`
against a later build), as the baseline's own advice says. Re-pick the pool
after each build from its own scores; a pool chosen for this build will be
too weak for the next.

## What this is used for

- A pre-heuristics baseline: milestone 7 and the speed task will change the
  engine's strength a lot, so the rating here is the number to compare them
  with, not a number to keep.
- Choosing opponents for later gauntlets (see the pool above).
- `UCI_Elo` is **not** calibrated by this file (milestone 6).
