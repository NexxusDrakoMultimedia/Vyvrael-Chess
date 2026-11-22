# Ladder Baseline 2

Second opponent-ladder measurement for Vyvræl testing. It replaces
[Ladder Baseline 1](ladder-Baseline-1.md) as the reference ladder for
gauntlets from 0.1.0-alpha.2 on. Vyvræl itself was **not** in this run.

> **Vyvræl is not in this baseline and has no rating here.** This is a
> reference ladder of 30 *other* engines. The Ordo ratings below are
> relative to the anchor engine (Stockfish 19 = 0) and describe only those
> 30 engines. Vyvræl's strength (milestone 6, known issue KI-8) is measured
> by gauntlets against the lower rungs, in separate results files such as
> [`results-0.1.0-alpha.2.md`](results-0.1.0-alpha.2.md). It must not be
> inserted into this file or inferred from it.

- **Date / host:** 2026-11-21, the Windows workstation (Windows 11, Ryzen 7 5700X), `cutechess-cli` run by `Baseline2.ps1`, 05:56-07:21 AEST (85 min)
- **Raw games:** `Baseline2.pgn` (1,740 games, 3,060,010 bytes, **not committed**; kept locally in `chess-matches`)
- **SHA-256 of the PGN:** `a0dfaa6669902a059eca1f513a3dd24536a3d34111ea015f013f4bd601b3674e` (the same value is in the run notes)
- **Run record kept locally, not committed:** `Baseline2-notes.md` (command, tool versions, SHA-256 of every engine binary, start and end), `Baseline2-run.log` (the PowerShell transcript; it names the Windows user and machine, so it is not committed), `Baseline2-engines.json` (a copy of the Cute Chess engine list as it was for the run)
- **Ordo output:** [`ladder-Baseline-2-ratings.txt`](ladder-Baseline-2-ratings.txt) and `ladder-Baseline-2-ratings.csv`, the maintainer's Ordo 1.2.6 output, copied unchanged (see "Where the ratings come from")

## What changed since Baseline 1

- **Five engines added:** CT800 1.46 at full strength, three limited-strength CT800 entries (1800, 1500, 1200) and Nostradamus 0.4. 30 engines in all.
- **Two rounds** instead of one: 116 games per engine instead of 48, so the error bars are narrower (about 180 to 290 instead of 250 to 500).
- **`Move Overhead` is 30 ms** in every engine that has the option (Stockfish, Reckless, Ethereal, Stash, Arasan, Rodent `TimeBuffer`, CT800 `Move Overhead [ms]`). Baseline 1 used each engine's own default (10 to 100 ms). Engines without the option are unchanged.
- **Books and tablebases are off** in every engine, checked in the engine list: Arasan `OwnBook` and `Use tablebases`, Fruit `OwnBook`, Vice `Book`, Rodent `UseBook`, CT800 `OwnBook` and `Keep Hash Tables`, Weiss `NoobBook` and `OnlineSyzygy`; the other Syzygy paths are empty. Of the engines that report time in the PGN, no move was played instantly except forced moves and moves with a mate score (Nostradamus reports nothing, so this could not be checked for it; its options have no book).
- **The name "Stockfish 19-2400" has no trailing space.**

## Setup

| Item | Value |
|---|---|
| Format | Round Robin, 2 rounds, 2 games per encounter, swap sides, each opening played twice |
| Games | 1,740 (30 engines, 435 pairings, 116 games per engine) |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd` (SHA-256 `7a7f6470615a69c6cf23d565417701d38732876f480af90d67b42abade35644a`), random order, FEN positions; no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB and Threads 1 where the engine has them (Nostradamus has no hash size to set; Rustic Alpha 1.5 exposes no options), `Move Overhead` 30 ms where offered (from the engine list; not stored in the PGN) |
| Tools | Cute Chess 1.5.1 (Qt 6.8.3, Windows 11 26H2, x86_64), `cutechess-cli` the same version; Ordo 1.2.6 (`ordo-win64.exe`) |
| Outcomes | 846 White wins, 826 Black wins, 68 draws; 1,273 games ended by adjudication, 414 by mate |
| Failures | none (no crashes, timeouts, illegal moves or stalls; no parse errors in the PGN) |

Strength-limited entries are one binary with a nominal `UCI_Elo` equal to the name suffix and `UCI_LimitStrength` on: Stockfish 19-1320/1500/1800/2000/2200/2400, Arasan 26.1-1500/1800, Rodent IV-1600/2000, CT800 1.46-1200/1500/1800. The number is a label, not a measured rating. CT800's own range is 1000 to 2550; its full-strength entry has the limit off.

## Result (Ordo)

Command: `ordo -a 0 -A "Stockfish 19" -W -s 500 -p Baseline2.pgn -o ladder-Baseline-2-ratings.txt -c ladder-Baseline-2-ratings.csv`. Ratings are relative to Stockfish 19 = 0, not CCRL or FIDE Elo.

| # | Engine | Rating | Error | Points /116 |
|--:|---|--:|--:|--:|
| 1 | Stockfish 19 | 0.0 | - | 111.5 |
| 2 | Reckless 0.9.0 | -78.0 | 184.3 | 109.5 |
| 3 | Viridithas 20 | -96.1 | 178.9 | 109.0 |
| 4 | Arasan 26.1 | -262.0 | 194.1 | 104.0 |
| 5 | Princhess 0.22.0 | -468.4 | 190.9 | 97.0 |
| 6 | Ethereal 12.75 | -550.9 | 196.7 | 94.0 |
| 7 | Stash 37.0 | -578.0 | 206.8 | 93.0 |
| 8 | Weiss 2.0 | -645.4 | 198.1 | 90.5 |
| 9 | Orion 1.1 | -807.6 | 213.1 | 84.5 |
| 10 | Rodent IV | -1003.0 | 232.4 | 77.0 |
| 11 | Fruit 2.1 | -1085.8 | 232.5 | 73.5 |
| 12 | CT800 1.46 | -1141.0 | 236.6 | 71.0 |
| 13 | Stockfish 19-2400 | -1280.3 | 236.2 | 64.0 |
| 14 | Vice 1.1 | -1436.2 | 244.5 | 55.0 |
| 15 | Stockfish 19-2200 | -1452.6 | 242.1 | 54.0 |
| 16 | Rustic Alpha 3.0.6 | -1460.8 | 247.9 | 53.5 |
| 17 | Stockfish 19-2000 | -1469.0 | 237.7 | 53.0 |
| 18 | Stockfish 19-1800 | -1493.5 | 239.8 | 51.5 |
| 19 | Rustic Alpha 2.4 | -1501.7 | 245.2 | 51.0 |
| 20 | Rustic Alpha 1.5 | -1583.7 | 241.7 | 46.0 |
| 21 | Rodent IV-2000 | -1757.3 | 249.6 | 36.0 |
| 22 | Rodent IV-1600 | -1813.0 | 247.2 | 33.0 |
| 23 | Stockfish 19-1500 | -1880.1 | 249.6 | 29.5 |
| 24 | Stockfish 19-1320 | -1969.6 | 249.8 | 25.0 |
| 25 | Nostradamus 0.4 | -2116.9 | 260.0 | 18.0 |
| 26 | CT800 1.46-1800 | -2139.1 | 254.7 | 17.0 |
| 27 | CT800 1.46-1500 | -2184.6 | 261.7 | 15.0 |
| 28 | Arasan 26.1-1800 | -2220.1 | 261.1 | 13.5 |
| 29 | Arasan 26.1-1500 | -2431.3 | 273.2 | 6.0 |
| 30 | CT800 1.46-1200 | -2486.5 | 290.3 | 4.5 |

White advantage reported by Ordo: 4.03 ± 3.16. Observed draw rate: 3.9 % (68/1,740).

### Where the ratings come from

The table is the maintainer's Ordo output from 2026-11-21 07:32 AEST: `ordo-win64.exe` from the folder `ordo-1.2.6-win` (SHA-256 `219be7081e63653c403ac4a0d8c0eecbab31116f4f67028d9d817b8290c9c6eb`), run through `scripts/windows/rate.ps1` with the command above over `Baseline2.pgn`. The maintainer confirms the version is 1.2.6 (2026-11-21); the run notes' banner line was empty, so the hash identifies the binary. The files were renamed to `ladder-Baseline-2-ratings.txt` and `.csv` when copied. The assistant's own run of the same command with a different Ordo build, made before this one, gave the same ratings, errors and points to the printed precision; that run is not part of the record.

## Compared with Baseline 1

- **The order holds.** Across the 25 engines in both baselines the order agrees closely (Spearman rank correlation 0.978). Stockfish 19, Reckless and Viridithas are still the top three in a different order (indistinguishable within the error bars), then Arasan 26.1, Princhess, Ethereal and Stash, Weiss, Orion, Rodent IV and Fruit.
- **The scale is less stretched.** The five new engines gave the weakest engines opponents they could score against, so the low end is no longer so far below the rest: Stockfish 19-1320 is at about -1,970 here against -2,608 in Baseline 1. Magnitudes cannot be compared between the two ladders; only orders can.
- **The limited entries now come out in label order** within each family: Stockfish 19-2400 > 2200 > 2000 > 1800 > 1500 > 1320, Rodent IV-2000 > 1600, Arasan-1800 > 1500, CT800 1800 > 1500 > 1200, and the Rustic Alpha versions in version order (3.0.6, 2.4, 1.5). In Baseline 1 several of these were tied or reversed. The distances between them, though, are not the label differences: 2200, 2000 and 1800 are one cluster, and 1500 and 1320 sit well below it.

## How to read this

- **Use the order, not the magnitudes.** Many pairings were lopsided, so Ordo's scale is stretched: the total spread (about 2,500 points) says more about that than about the engines.
- **Close neighbours are not separated.** The error bars are ±180 to ±290. Stockfish 19-2200, -2000 and -1800, Vice 1.1 and Rustic Alpha 3.0.6 and 2.4 all lie within about 60 points of one another.
- **Nominal `UCI_Elo` is not strength.** At 10+0.1 the limited entries do not match their labels. CT800's three limited entries and the two limited Arasan entries are very weak: they sit at the bottom, and CT800 1.46-1200 scored 4.5 points of 116.
- **CT800 1.46 at full strength** is mid-table: 61 % and a rating of about -1,141, between Fruit 2.1 and Stockfish 19-2400.
- **Nostradamus 0.4 gives the PGN nothing to analyse:** it reports no score, depth or time, so its moves have no comments. None of its 116 games was adjudicated (adjudication needs the scores of both sides); every one ended in mate, repetition or stalemate. Its use of the clock cannot be read from the PGN. Treat it as a fixed rung.
- Ordo's "draw rate (equal opponents) = 50 %" line looks like a default, not a measurement; the observed draw rate was 3.9 %.
- Ordo's error values come from a random simulation (`-s 500`) and may differ slightly when re-run; the ratings themselves should not.

## Using the ladder for Vyvræl

Gauntlets use the lower and middle rungs. The pool as it stands, from the alpha.1 results and this ladder:

| Use | Opponents |
|---|---|
| Core | Rodent IV-2000, Stockfish 19-1800, Stockfish 19-1320, CT800 1.46-1800, CT800 1.46-1500, Nostradamus 0.4 |
| Optional | Stockfish 19-1500 |
| Hold (for later, once Vyvræl has moved up) | Stockfish 19-2000, Rustic Alpha 2.4 and 3.0.6, Vice 1.1, Rustic Alpha 1.5 |
| Probably too weak to separate builds at the level of 0.1.0-alpha.1 (CT800 1.46-1200 and the limited Arasan entries have not been played against Vyvræl) | CT800 1.46-1200, Arasan 26.1-1500 and -1800, Rodent IV-1600 |

Measure each Vyvræl build with a gauntlet against the core, then rate it together with Baseline 2's games in one Ordo run (`-a 0 -A "Stockfish 19"`), so its number sits on this ladder's scale. Keep earlier Vyvræl builds as opponents. Keep Hash, Threads, `Move Overhead`, time control and openings fixed between runs you want to compare, and do not merge games played with different overhead settings into one rating (Gauntlets 1 and 2 of 0.1.0-alpha.1 were played with Baseline 1's settings).

## Engine binaries

SHA-256 of each executable at the time of the run, from the run notes. Entries that share a binary differ only in options.

| Names | File | SHA-256 |
|---|---|---|
| Stockfish 19, Stockfish 19-2400, Stockfish 19-2200, Stockfish 19-2000, Stockfish 19-1800, Stockfish 19-1500, Stockfish 19-1320 | `stockfish-windows-x86-64-universal.exe` | `45bc8e4969147db9c2eb533810637994619bff0eacc81ccfd9854394901bcbd0` |
| Viridithas 20 | `viridithas-20-win-x86-64-v3.exe` | `c7fa86b2b46fe1c34b0ea3715ad3b4a61d3d984ec63d50b1d6b4f187332fbc51` |
| Reckless 0.9.0 | `reckless-windows-avx2.exe` | `b74ead5648cfa7a7a9f51d04566cf00f56dcf90dacd1252990906223bf1891b8` |
| Arasan 26.1, Arasan 26.1-1800, Arasan 26.1-1500 | `arasanx-64-avx2-bmi2.exe` | `cca0e46c7a1d18d4fe407827d96da2f02753e29ac62dc6748a6fbf83ca7985e0` |
| Stash 37.0 | `stash-37.0-windows-64.exe` | `71c401864303f15d29db10391f75d032ef1b28e7e2875675a8b281c94f79b818` |
| Princhess 0.22.0 | `princhess.exe` | `7a95e41985483b8689e77cb32ff4687f41f56f91317b9615fa5c2edeabc09b81` |
| Ethereal 12.75 | `Ethereal12.75-x64-pext-avx2.exe` | `22f12797fd43e220047b41c2f6373a20cabd893895a6e0a3c273339b911430e2` |
| Weiss 2.0 | `weiss-pext.exe` | `5bc2262139ae0712b8589c95b0cd343d536cf22a9a6ca4ab98e73c03ce8f0eed` |
| Orion 1.1 | `orion64-v1.1-pop-avx.exe` | `712ba9848e84c9a7e92eb84553f6e56e4a6241e0d2f7ade86f5a18ed9ecf9083` |
| Rodent IV, Rodent IV-2000, Rodent IV-1600 | `rodent-iv-x64.exe` | `39c879b2dfdaf0252e94db547397d95422fee76ea8bafa584bfdc03e4d648de5` |
| Fruit 2.1 | `fruit_21.exe` | `7f321a960887af6c52b5768a2c5b5a85720358398a8790fd8deb281fe2cb2565` |
| Vice 1.1 | `vice11_win32.exe` | `92827b547b32b7a71b28ec41c99bf1f6aaaf17fa5bb3265232bb015fb5f93a8a` |
| Rustic Alpha 3.0.6 | `rustic-alpha-3.0.6-win64.exe` | `fa8c651196c05bfd670c25fbad3712983eb85fc29d68593f7ee841ce5ec2f95e` |
| Rustic Alpha 2.4 | `rustic-alpha-2.4-win64.exe` | `0780316684b43af4666eec542b8e8670a79155ae0e6de87bb6e3c3b5d1600e53` |
| Rustic Alpha 1.5 | `rustic-alpha-1.5-win64.exe` | `2ee1c05f25380c9be12f4d2deeb3279fa66fe2e2b3f104f545a19aea7ed63c18` |
| CT800 1.46, CT800 1.46-1800, CT800 1.46-1500, CT800 1.46-1200 | `CT800_V1.46_x64.exe` | `dc0c1f6e49c56898815c5232b0d05db39412428597b66c14fc1b6c7b43d9405c` |
| Nostradamus 0.4 | `nostradamus-v0.4.exe` | `485d3b606ad8784e6435b08f32e9a1f789562c7abc6064ce807868e64d1cb5e2` |
