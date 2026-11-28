# Ladder Baseline 3

Third opponent-ladder measurement for Vyvræl testing, and the first with
Vyvræl in it: four Vyvræl builds played in the round robin beside 40 other
engines. It follows [Ladder Baseline 2](ladder-Baseline-2.md), which had 30
other engines and no Vyvræl.

> **Read this as an ordering, not a rating.** The Ordo numbers below are
> relative to the anchor engine (Stockfish 19 = 0) on a scale stretched by
> lopsided results. They are not CCRL, Lichess or FIDE Elo, and they are not
> on the same scale as Ladder Baseline 2's numbers. Each pairing is four
> games, each engine 172, and the error bars are about ±170 to ±260: close
> neighbours are not separated. What the run says about Vyvræl 0.2.0 is
> written up in [`results-0.2.0.md`](results-0.2.0.md).

- **Date / host:** 2026-11-28, 06:26 to 09:39 UTC (3 hours 12 minutes), the Windows workstation (Windows 11, Ryzen 7 5700X); `scripts/windows/match.ps1 roundrobin` from the clean commit `39ff647` on the branch `release/0.2.0`
- **Raw games:** `2026-11-28-roundrobin-baseline3.pgn` (3,784 games, 7,385,713 bytes, **not committed**: `*.pgn` is git-ignored; kept locally)
- **SHA-256 of the PGN:** `7877ceadc2e2bf6d4cc321dbb942e45e6f638de428363852403fdf8f414efd60` (the same value is in the run notes)
- **Run record kept locally, not committed:** the run notes (the full `cutechess-cli` command, tool version, SHA-256 of every engine binary, start and end), the log, and a copy of the engine list as it was for the run
- **Ordo output:** [`ladder-Baseline-3-ratings.txt`](ladder-Baseline-3-ratings.txt) and [`ladder-Baseline-3-ratings.csv`](ladder-Baseline-3-ratings.csv), copied unchanged

## What changed since Baseline 2

- **Four Vyvræl builds are in the run:** 0.1.0-alpha.1, 0.1.0-alpha.2, 0.1.0 and 0.2.0 (see "The Vyvræl builds"). Baseline 2 had none.
- **Eleven engines added:** Blunder 4.0.0, 5.0.0, 7.1.0 and 7.6.0, MinimalChess 0.4.1, 0.5 and 0.6, Velvet 1.0.0, Inanis 1.0.1, and Tantabus 1.0.2 and 2.0.0. They fill the middle of the ladder, where Baseline 2 was thin.
- **One engine dropped:** Arasan 26.1-1800, retired from the installer on 2026-11-22 as too weak to tell Vyvræl builds apart. Baseline 2's record keeps it.
- **44 engines** in all (29 of Baseline 2's, 11 new, 4 Vyvræl): 946 pairings and 172 games per engine, against 116 in Baseline 2.
- **Run by the project's script** (`match.ps1 roundrobin` with the installer's engine list and `-Build` for the Vyvræl builds), not by a hand-written one, and the opening order is fixed by `-Seed 20261128`.
- **Stash 37.0 is another file:** `stash-37.0-windows-x86_64-bmi2.exe`, the installer's `pext` build, where Baseline 2 used `stash-37.0-windows-64.exe`. Every other engine of Baseline 2 is the same file as there, by SHA-256.
- **Of the eleven new engines only Inanis has a move-overhead option** (set to 30 ms). The other ten play with their own margin. None of them lost on time.

## Setup

| Item | Value |
|---|---|
| Format | Round robin, 2 rounds, 2 games per encounter with sides swapped, each opening played twice |
| Games | 3,784 (44 engines, 946 pairings, 4 games per pairing, 172 games per engine) |
| Command | `match.ps1 roundrobin -Build "Vyvrael 0.1.0-a1=...","Vyvrael 0.1.0-a2=...","Vyvrael 0.1.0=...","Vyvrael 0.2.0=..." -Rounds 2 -Seed 20261128 -Deterministic -Label baseline3`; the full `cutechess-cli` command is in the run notes |
| Time control | 10+0.1, pondering off |
| Openings | `UHO_Lichess_4852_v1.epd` (SHA-256 `7a7f6470615a69c6cf23d565417701d38732876f480af90d67b42abade35644a`), random order fixed by `-srand 20261128`, FEN positions; 1,892 openings, each played twice; no book, no tablebases |
| Draw adjudication | move 34, 10 moves, within 20 cp |
| Resign adjudication | 3 moves, 700 cp, two-sided |
| Engine options | Hash 64 MB for every engine (`-each`; Blunder 4.0.0, MinimalChess 0.4.1 and Rustic Alpha 1.5 have no such option and Nostradamus 0.4 no size to set), Threads 1 where the engine has it, a move overhead of 30 ms where the engine list has the option (Stockfish, Arasan, Reckless, Stash, Ethereal, Inanis, Rodent `TimeBuffer`, CT800 `Move Overhead [ms]`), books and tablebases off |
| Vyvræl options | Threads 1, Hash 64 MB, `Seed=822286742` and `Variety=0` (`-Deterministic`), everything else at its default. 0.1.0-alpha.1 has neither `Seed` nor `Variety` and ignored them |
| Concurrency | 6 games at once on 8 physical cores |
| Tools | `cutechess-cli` 1.5.1; Ordo 1.2.6 (`ordo-win64.exe`, SHA-256 `219be7081e63653c403ac4a0d8c0eecbab31116f4f67028d9d817b8290c9c6eb`) |
| Outcomes | 1,822 White wins, 1,800 Black wins, 162 draws; 2,941 games ended by adjudication (2,898 wins, 43 draws), 723 by mate, 119 by a normal draw (77 repetition, 26 insufficient material, 13 fifty moves, 3 stalemate), 1 on time |
| Failures | One loss on time: Ethereal 12.75 as White against Weiss 2.0, game 2,533, counted as played. No illegal move, crash or stall. No Vyvræl build lost on time. `cutechess-cli` printed "Destroyed while process is still running" 120 times between games, with no result affected: it ends an engine process itself between games, as in earlier runs |

Strength-limited entries are one binary with a nominal `UCI_Elo` equal to the name suffix and `UCI_LimitStrength` on: Stockfish 19-1320/1500/1800/2000/2200/2400, Arasan 26.1-1500, Rodent IV-1600/2000, CT800 1.46-1200/1500/1800. The number is a label, not a measured rating.

## The Vyvræl builds

| Name in the games | Build | File | Search |
|---|---|---|---|
| Vyvrael 0.1.0-a1 | plain | the released Windows file of 0.1.0-alpha.1 | the plain search of before milestone 7 |
| Vyvrael 0.1.0-a2 | plain | the released Windows file of 0.1.0-alpha.2 | the same as 0.1.0 (bench 16,611,416 nodes) |
| Vyvrael 0.1.0 | `avx2` | the released `avx2` file of 0.1.0 | the plain search of before milestone 7: bench 16,611,416 nodes |
| Vyvrael 0.2.0 | `avx2` | built by hand from the source of commit `182f9ec` (`release/0.2.0`), as `vyvrael-0.2.0-avx2.exe` | milestone 7's search: bench 28,175 nodes |

The two alphas were released as plain builds only, so they are plain here; the other two are `avx2`. All three older builds search to the same depth in these games (median 4), so the build level makes no visible difference between them. Vyvræl 0.2.0 is not a released file: it is a hand build at the level of the `avx2` release file, from the source the release is made of. SHA-256 of each is under "Engine binaries".

## Result (Ordo)

Command, through `scripts/windows/rate.ps1`: `ordo -a 0 -A "Stockfish 19" -W -s 500 -p <the PGN> -o ... -c ...` (Ordo 1.2.6). Ratings are relative to Stockfish 19 = 0, not CCRL or FIDE Elo.

| # | Engine | Rating | Error | Points /172 |
|--:|---|--:|--:|--:|
| 1 | Stockfish 19 | 0.0 | - | 167.0 |
| 2 | Reckless 0.9.0 | -19.9 | 171.1 | 166.5 |
| 3 | Viridithas 20 | -76.3 | 175.5 | 165.0 |
| 4 | Arasan 26.1 | -258.8 | 172.4 | 159.5 |
| 5 | Ethereal 12.75 | -507.5 | 185.1 | 150.5 |
| 6 | Princhess 0.22.0 | -532.1 | 189.0 | 149.5 |
| 7 | Stash 37.0 | -544.3 | 189.9 | 149.0 |
| 8 | Weiss 2.0 | -626.8 | 198.6 | 145.5 |
| 9 | Orion 1.1 | -770.6 | 188.3 | 139.0 |
| 10 | Rodent IV | -862.3 | 196.0 | 134.5 |
| 11 | Inanis 1.0.1 | -1051.4 | 199.8 | 123.5 |
| 12 | Fruit 2.1 | -1115.8 | 198.9 | 119.0 |
| 13 | CT800 1.46 | -1115.8 | 195.5 | 119.0 |
| 14 | Blunder 7.6.0 | -1187.3 | 195.5 | 113.5 |
| 15 | Tantabus 2.0.0 | -1211.7 | 197.9 | 111.5 |
| 16 | Velvet 1.0.0 | -1269.9 | 198.3 | 106.5 |
| 17 | Blunder 7.1.0 | -1286.7 | 199.1 | 105.0 |
| 18 | MinimalChess 0.6 | -1362.1 | 198.4 | 98.0 |
| 19 | **Vyvrael 0.2.0** | **-1418.8** | **199.1** | **92.5** |
| 20 | Tantabus 1.0.2 | -1428.9 | 195.0 | 91.5 |
| 21 | Stockfish 19-2400 | -1434.0 | 197.8 | 91.0 |
| 22 | Stockfish 19-2200 | -1459.2 | 199.4 | 88.5 |
| 23 | MinimalChess 0.5 | -1474.2 | 196.3 | 87.0 |
| 24 | Blunder 5.0.0 | -1539.1 | 194.5 | 80.5 |
| 25 | Stockfish 19-2000 | -1554.1 | 199.0 | 79.0 |
| 26 | Vice 1.1 | -1599.3 | 199.5 | 74.5 |
| 27 | MinimalChess 0.4.1 | -1650.3 | 201.8 | 69.5 |
| 28 | Rustic Alpha 3.0.6 | -1686.7 | 198.9 | 66.0 |
| 29 | Rustic Alpha 2.4 | -1713.2 | 200.3 | 63.5 |
| 30 | Stockfish 19-1800 | -1740.2 | 201.9 | 61.0 |
| 31 | Rustic Alpha 1.5 | -1767.6 | 201.8 | 58.5 |
| 32 | Blunder 4.0.0 | -1773.2 | 198.4 | 58.0 |
| 33 | Rodent IV-2000 | -1967.0 | 204.3 | 42.0 |
| 34 | **Vyvrael 0.1.0** | **-1973.7** | **200.6** | **41.5** |
| 35 | **Vyvrael 0.1.0-a1** | **-1993.9** | **202.4** | **40.0** |
| 36 | **Vyvrael 0.1.0-a2** | **-2007.6** | **203.5** | **39.0** |
| 37 | Stockfish 19-1500 | -2116.8 | 207.6 | 31.5 |
| 38 | Rodent IV-1600 | -2172.4 | 209.6 | 28.0 |
| 39 | Nostradamus 0.4 | -2276.9 | 211.7 | 22.0 |
| 40 | Stockfish 19-1320 | -2334.7 | 212.4 | 19.0 |
| 41 | CT800 1.46-1500 | -2408.4 | 218.6 | 15.5 |
| 42 | CT800 1.46-1800 | -2466.5 | 225.2 | 13.0 |
| 43 | Arasan 26.1-1500 | -2670.5 | 244.5 | 6.0 |
| 44 | CT800 1.46-1200 | -2771.6 | 259.4 | 3.5 |

White advantage reported by Ordo: 2.04 ± 4.69. Observed draw rate: 4.3 % (162/3,784).

## Compared with Baseline 2

- **The order holds.** Across the 29 engines in both baselines the order agrees closely (Spearman rank correlation 0.995). The first four and the last two are in the same places; no engine moved more than two places among the 29.
- **The scale is more stretched at the low end.** The eleven new engines sit in the middle and beat the weak entries, which pushes those further down: Rodent IV-2000 is at about -1,967 here against -1,757 in Baseline 2, Stockfish 19-1320 at -2,335 against -1,970. Magnitudes cannot be compared between the ladders; only orders can.
- **The cluster of limited Stockfish entries has spread out.** In Baseline 2 Stockfish 19-2200, -2000 and -1800 were within about 40 points; here they are in label order over about 280 points, with other engines between them. CT800 1.46-1800 and -1500 are in the wrong order here by 2.5 points of 172, well inside the noise.

## How to read this

- **Use the order, not the magnitudes.** Many pairings were lopsided (Stockfish 19 scored 97 %), so Ordo's scale is stretched: the total spread of about 2,770 points says more about that than about the engines.
- **Close neighbours are not separated.** The error bars are ±170 to ±260 against a far-off anchor. Vyvrael 0.2.0, Tantabus 1.0.2, Stockfish 19-2400, Stockfish 19-2200 and MinimalChess 0.5 lie within 56 points and 5.5 game points of one another.
- **The three older Vyvræl builds are level,** as they should be: 0.1.0 and 0.1.0-alpha.2 have the same search, and 0.1.0-alpha.1 was level with alpha.2 within the noise when milestone 6 measured them. They finished within 2.5 points of 172, and among themselves the pairings ended 2-2, 3-1 and 2.5-1.5.
- **Four games per pairing say nothing about a single pairing.** The table is built from 172 games per engine spread over 43 opponents. A result such as 4-0 between two neighbours is not a measurement of that pair.
- **Nominal `UCI_Elo` is not strength.** At 10+0.1 the limited entries do not match their labels, and the limited CT800 and Arasan entries are at the bottom.
- **Nostradamus 0.4 reports no score, depth or time,** so its games cannot be adjudicated and its use of the clock cannot be read from the PGN.
- Ordo's "draw rate (equal opponents) = 50 %" line is a default, not a measurement; the observed draw rate was 4.3 %.
- Ordo's error values come from a random simulation (`-s 500`) and may differ slightly when re-run; the ratings themselves should not.
- **Not comparable with results from another machine or other settings.**

## Using the ladder for Vyvræl

From 0.2.0 on, the opponents near Vyvræl are other ones than before. By their places in the table, not by the four games of any one pairing:

| Use | Opponents |
|---|---|
| Near Vyvræl 0.2.0 (within about 200 points) | Velvet 1.0.0, Blunder 7.1.0, MinimalChess 0.6 and 0.5, Tantabus 1.0.2, Stockfish 19-2400, -2200 and -2000, Blunder 5.0.0, Vice 1.1 |
| Above, to grow into | Tantabus 2.0.0, Blunder 7.6.0, CT800 1.46, Fruit 2.1, Inanis 1.0.1 |
| Below, probably still of use | MinimalChess 0.4.1, the Rustic Alpha versions, Stockfish 19-1800, Blunder 4.0.0 |
| Far below (more than 500 points and 50 game points lower) | every entry from Rodent IV-2000 down, and the earlier Vyvræl builds |

A gauntlet for a later full-strength build can be rated on this ladder's scale: it keeps Hash, Threads, the move overhead, the time control and the openings of this run, and is rated together with this run's games in one Ordo run (`-a 0 -A "Stockfish 19"`). Do not merge games played with other settings into one rating. Ladder Baseline 2 stays the scale of `UCI_Elo` until the maintainer decides otherwise (see "Opponent ladder" in [TESTING.md](../TESTING.md)).

## Engine binaries

SHA-256 of each executable at the time of the run, from the run notes. Entries that share a binary differ only in options.

| Names | File | SHA-256 |
|---|---|---|
| Vyvrael 0.1.0-a1 | `vyvrael-0.1.0-alpha.1-windows-x86_64.exe` | `ed2209de016c80979bce46b3b9e094984e1b309f2d737e26efd30cde9411b046` |
| Vyvrael 0.1.0-a2 | `vyvrael-0.1.0-alpha.2.exe` | `1b0d518f26fcbb876f747449422c9d32ea249db0223f3ddec319ac4297c81ad3` |
| Vyvrael 0.1.0 | `vyvrael-0.1.0-avx2.exe` | `85e7b42458953470bdb0315e5c038fd5d973db27638c6256b9b314c2e4ff7174` |
| Vyvrael 0.2.0 | `vyvrael-0.2.0-avx2.exe` (hand build) | `347a54f1fde36398a9d7d35181970963bf80fa3b7cbb2268cb131686567e095c` |
| Stockfish 19, Stockfish 19-2400, Stockfish 19-2200, Stockfish 19-2000, Stockfish 19-1800, Stockfish 19-1500, Stockfish 19-1320 | `stockfish-windows-x86-64-universal.exe` | `45bc8e4969147db9c2eb533810637994619bff0eacc81ccfd9854394901bcbd0` |
| Viridithas 20 | `viridithas-20-win-x86-64-v3.exe` | `c7fa86b2b46fe1c34b0ea3715ad3b4a61d3d984ec63d50b1d6b4f187332fbc51` |
| Reckless 0.9.0 | `reckless-windows-avx2.exe` | `b74ead5648cfa7a7a9f51d04566cf00f56dcf90dacd1252990906223bf1891b8` |
| Arasan 26.1, Arasan 26.1-1500 | `arasanx-64-avx2-bmi2.exe` | `cca0e46c7a1d18d4fe407827d96da2f02753e29ac62dc6748a6fbf83ca7985e0` |
| Stash 37.0 | `stash-37.0-windows-x86_64-bmi2.exe` | `7cd0e8bb0bf48e819e4acf3fe9323a51e9e5b1813d073ff295d802fcd8fd7bb3` |
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
| Blunder 4.0.0 | `blunder.exe` | `3af0456db698d937d9982770b6b1980671cddb60441705f3d7db62562b51b648` |
| Blunder 5.0.0 | `blunder.exe` | `715d196e0bb1cb04be8693c97c3f1680a8b16a8e2e9e7a74cda16c008e0ae923` |
| Blunder 7.1.0 | `blunder.exe` | `f9ac2cf20040afb626d526e2e71683163abf1f1707e29ea7cb655115ff61fc23` |
| Blunder 7.6.0 | `blunder-windows.exe` | `15c76fe1bf16d1f35ce1173c74788588dcd01dac59b5bc5353c67de1c2c19ac6` |
| MinimalChess 0.4.1 | `MinimalChessEngine.exe` | `55147a2ae5c2e8d3dfe86e9d392740cc091c112b21f16b57329341a67e30bfc6` |
| MinimalChess 0.5 | `MinimalChessEngine.exe` | `bf1e8a542e1040771f72d0dc53d6ebe4198b3a9bce1af2f35640a63d18ac6496` |
| MinimalChess 0.6 | `MinimalChessEngine.exe` | `99623e8fbf07fe91586cd612b3dc3041dabf1652e8954a6dcf68eaa6ee58d4e2` |
| Velvet 1.0.0 | `velvet-v1.0.0-x86_64-modern.exe` | `338d7d0606dc40097e373251e414f9d9b56c6ab9095367cd92f62ec18d41b163` |
| Inanis 1.0.1 | `inanis.exe` | `4e3ed4dafd1c39a66f873f2a7f0ac7f4bb99f1fb0db7cb0aeeb8b33939493bda` |
| Tantabus 1.0.2 | `tantabus-windows-2019-x86-64-v3.exe` | `5da552a4459e52e0df37186cf0f7298e8f1bab936be2a1dcf5bf8c129074cfd0` |
| Tantabus 2.0.0 | `tantabus-windows-2019-x86-64-v3.exe` | `8c343ed1797c923b34870fb0b52d1f88699a68e329c38135b211782fa20f864e` |
