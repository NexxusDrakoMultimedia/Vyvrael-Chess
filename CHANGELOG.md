# Changelog

All notable changes to Vyvræl are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/). Before
1.0.0 a minor version may change anything.

## [Unreleased]

### Added

- **The installer fetches the opening file:** `install_tools.py
  --openings` downloads `UHO_Lichess_4852_v1.epd.zip` from the Stockfish
  books repository at a pinned commit, checks the zip and the `.epd` by
  SHA-256 (the `.epd` is the file of Ladder Baselines 1 and 2), puts it
  under the install prefix and prints the `OPENINGS=` line.
- **Eleven opponents for Ladder Baseline 3 in the engine installer:**
  Blunder 4.0.0, 5.0.0, 7.1.0 and 7.6.0, MinimalChess 0.4.1, 0.5 and 0.6,
  Velvet 1.0.0, Inanis 1.0.1, and Tantabus 1.0.2 and 2.0.0: full-strength
  engines rated from about 1600 to 2700 on the CCRL Blitz list (CCRL's
  ratings of them, not a scale Vyvræl has been measured on). They add
  full-strength rungs around and above the entries 0.1.0-alpha.2 placed
  between on Ladder Baseline 2, for when milestone 7 moves it. All are
  official Linux and Windows downloads under MIT, GPL-3.0 or MPL-2.0
  licences, pinned by SHA-256.
- **The match scripts' run notes say more.** `match.ps1` and `match.sh`
  record the repository's commit and whether its working tree was clean or
  dirty, the number of games in the PGN and its SHA-256; a gauntlet also
  keeps a copy of the engine list it read beside the notes.
- **The match scripts can give a build UCI options:** `--test-option
  NAME=VALUE` and `--base-option` in `match.sh`, `-TestOption` and
  `-BaseOption` in `match.ps1`, recorded in the run notes. Limited Vyvræl
  builds (`UCI_LimitStrength`, `UCI_Elo`) can now be measured.
- **A robustness test for the UCI loop:** seeded streams of malformed and
  unexpected input (random bytes, broken FENs, oversized and truncated
  lines, odd option values, commands during a search). It found no panic.
- **A first measurement of strength** (known issue KI-8): 1,000 games of
  0.1.0-alpha.2 against Ladder Baseline 2's rungs, rated with Baseline 2.
  It places Vyvræl between Stockfish 19-1800 and Rodent IV-2000 on that
  ladder's own scale, which is not an Elo (`docs/testing/results-0.1.0-alpha.2.md`).

### Changed

- **The ladder installer no longer provides Arasan 26.1-1800.** It scored
  13.5 of 116 in Ladder Baseline 2, too weak to tell Vyvræl builds apart,
  and it is not an opponent in the `UCI_Elo` calibration. CT800 1.46-1200
  and Arasan 26.1-1500 stay, as the calibration's lowest rungs. The name
  stays in Baseline 2's record and is left out from Ladder Baseline 3 on;
  `--only "Arasan 26.1-1800"` says why.
- **`UCI_Elo` is calibrated, provisionally.** Its numbers are now ratings
  on the scale of the reference ladder, measured for seven settings of the
  weakening: the range is 1110 to 1630 (was 400 to 1800, labels only), and
  the default 1360 (was 1200) plays exactly as the old default did.
  A GUI that sets a value outside the new range gets it refused, as for
  any out-of-range value. Ratings below 1110 cannot be played yet.
  Full-strength play is unchanged.
- **The Rust toolchain is pinned to 1.100.0** in `rust-toolchain.toml`
  (was the moving `stable`), so local builds, CI and release binaries use
  the same compiler. CI and the release workflow install the pinned
  version before building.
- **CI pins `actions/checkout` to a commit** (v7.0.1, as the release
  workflow does) instead of the moving tag `v4`.

### Fixed

- **The match scripts refuse a build name with a line break.** The
  plain-ASCII check let line breaks through (in `match.ps1` only one at
  the end).

## [0.1.0-alpha.2] - 2026-11-22

The second alpha: games that no longer repeat, a reproducible bench, and
the tools to measure the engine on Windows and Linux. Release notes:
[docs/releases/0.1.0-alpha.2.md](docs/releases/0.1.0-alpha.2.md).

- **Breaking for scripts and GUI settings:** the options `DeterministicRNG`
  and `RngSeed` are replaced by `Seed`.
- **Full-strength play is unchanged** with the new options at their
  defaults: the bench node count is the same before and after them.
- **Strength is still unmeasured** and `UCI_Elo` is still not calibrated
  (milestone 6).

### Changed

- **The UCI option `Seed` replaces `DeterministicRNG` and `RngSeed`.** It is
  a spin from 0 to 2147483647, default 0. With 0 the seed of
  strength-affecting randomness comes from system entropy when the engine
  starts and at every `ucinewgame`, as with `DeterministicRNG` off before;
  any other value is the seed, as `RngSeed` with `DeterministicRNG` on was.
  The largest value is the largest signed 32-bit number, because Cute Chess
  stores spin values that way and reset `RngSeed` (up to 4294967295) to 0
  whenever it was edited. Entropy seeds now stay from 1 to 2147483647, so a
  logged seed can always be given back as `Seed`. The seed is logged with
  `info string` in reply to `uci` and now at every `ucinewgame`, also when
  it was given. A GUI or script that sets the old options gets "unknown
  option" for them.
- The match scripts' `--deterministic` (`-Deterministic`) now sets
  `Seed=822286742` and `Variety=0` on the Vyvræl builds instead of the old
  `DeterministicRNG=true`.

### Added

- UCI option `Variety` (spin, 0 to 100 centipawns, default 0). In the first
  12 full moves, at full strength, the engine plays at random among its
  best move and the moves a second search checks to be within `Variety` of
  it, weighted towards the best, so that its games do not repeat. Never
  when the best score is a mate; a move that could not be checked in time
  is never played; the search keeps two thirds of the move's budget. An
  `info string` says when a move other than the best was played. With 0,
  the default, play is exactly as before. It weakens play on purpose and is
  exempt from the SPRT rule; it must be 0 in strength matches.
- `vyvrael bench [depth]`: searches seven fixed positions to depth 4 (or the
  depth given) at full strength and prints `<nodes> nodes <nps> nps`. The
  node count is the same on every run of a build (16,611,416 for this one)
  and changes only when what the search does changes; the speed is for
  comparing builds on one machine.
- `scripts/local.env.example`: the settings the match and rating scripts
  read from a git-ignored `scripts/local.env` (tool locations, the opening
  file, the engine list, concurrency, priority).
- `scripts/linux/match.sh`: runs a `cutechess-cli` match in one of three
  modes, `sprt`, `gauntlet` or `h2h`, with the ladder's fixed settings.
  SPRT bounds and a round cap are required; `--dry-run` prints the command.
  Beside the PGN it writes a log of the progress and run notes (the exact
  command, engine names with the SHA-256 of each binary, the tool's
  version, start and end times), and it ends an SPRT by saying which
  hypothesis was accepted and at what LLR.
- `scripts/windows/match.ps1`: the same match script for Windows
  PowerShell 5.1 and PowerShell 7, with the same modes, checks and defaults
  (`-Rounds` for `--rounds`, and so on); low priority is BelowNormal.
- `scripts/linux/rate.sh`: runs Ordo over one or more PGN files with an
  anchor engine (by default "Stockfish 19" at 0, as in Ladder Baseline 1),
  writes a text table and a CSV, and prints the order with a reminder of
  what the numbers do not say.
- `scripts/windows/rate.ps1`: the same rating script for Windows.
- `scripts/install_tools.py` on Windows installs Cute Chess 1.5.1 and Ordo
  1.2.6 from their official zip archives, checked by SHA-256, without
  administrator rights, and prints the two lines to put in
  `scripts/local.env`. It reports a missing Visual C++ runtime and never
  installs it. lichess-bot is not installed on Windows.
- `scripts/ladder_engines.py`: the sources of the opponent engines of
  Ladder Baseline 2, the reference ladder, pinned for Linux and Windows.
  On Windows 17 official programs cover all 30 names, each the file of
  Ladder Baseline 2 by SHA-256 in the `pext` build (Stash aside). On Linux
  14 programs cover 27 names: Orion 1.1 exists for Windows only, Ethereal
  12.75's source crashes when built with a current compiler, and
  Nostradamus 0.4 is a Python program with a PyTorch model, which the
  installer does not set up.
- Two builds of the ladder's engines: `avx2` by default and `pext` with
  `--engine-build pext`, `VYVRAEL_ENGINE_BUILD` or `ENGINE_BUILD` in
  `scripts/local.env`, for the engines that offer a PEXT build (Arasan,
  Stash, Weiss, and Ethereal on Windows).
- `scripts/install_tools.py --engines` installs those engines on Linux and
  Windows under `<prefix>/engines`, each in a directory of its own: an
  official binary where there is one, otherwise (Linux only) a build from
  the official source (a tar.gz or zip archive). On Windows it also reads
  Rodent IV's 7-Zip archive with Windows' own `tar`, and fetches that one
  file over plain HTTP, which is all the author's site serves, with its
  digest checked.
  `--only "<name>"` installs one. It never uses sudo, names a missing build
  tool and carries on with the other engines, and leaves nothing behind of
  an engine whose download or build failed. It then writes
  `<prefix>/engines/engines.json`, the list of what is installed under the
  ladder's names, the limited entries (such as "Stockfish 19-1500")
  included, with Hash 64, one thread, the strength settings and the rest
  of Ladder Baseline 2's settings (books and tablebases off, 30 ms of move
  overhead where the engine has the option). Last, it
  starts every installed engine under each of its names, sends `uci`, its
  options, `isready` and `go depth 6`, and prints PASS or FAIL; every wait
  has a time limit. `--verify` runs that check alone.

## [0.1.0-alpha.1] - 2026-11-21

The first release: a complete UCI chess engine, in alpha. Release notes:
[docs/releases/0.1.0-alpha.1.md](docs/releases/0.1.0-alpha.1.md).

- **Strength is unmeasured** (known issue KI-8). No rating is claimed.
- **`UCI_Elo` is not calibrated**: its settings are ordered, not measured.
- **A casual-only Lichess bot is planned**; rated play waits for 0.2.0.
- **Built under the 0rwx X scale**: written by AI coding agents and curated
  by the maintainer, with trust resting on tests, documentation and small
  commits.

Entries marked "found before any release" describe defects that never
shipped; they are kept as a record of what the reviews caught.

### Changed

- The project's tagline is now "A dragon-flavoured UCI chess engine in Rust,
  built with AI and playing for the attack." It replaces "Dragon-flavoured,
  aggressive, vibecoded UCI-compatible chess engine written in Rust." in the
  README, the package description and the crate documentation. "Playing for
  the attack" is a style goal; the engine's strength is not measured yet.
- The time kept back on every move is no longer a fixed 30 ms: it is the
  new option `Move Overhead`. The constant `uci::MOVE_OVERHEAD` is gone;
  `uci::MOVE_OVERHEAD_DEFAULT` and `uci::MOVE_OVERHEAD_MAX` give the
  option's default and limit in milliseconds, and `uci::time_for_move` and
  `uci::limits_for` take the overhead as an argument. With the default,
  play is as before.
- The Lichess plan: a casual-only bot account (`Vyvrael-Chess`) after the
  first alpha release, and rated play only after 0.2.0. It replaces the
  earlier plan of a private test first. The branding requirement now gates
  rated play, not the first release.

### Fixed

- A line of input that was not valid UTF-8 ended the engine silently, with
  no `bestmove` for a running search. Such a line is now handled like any
  other line that is not understood. Found in the milestone 5 review, before
  any release.

- A limited-strength game played with an entropy seed could not be replayed
  from the logged seed after the first game of a session. Every
  `ucinewgame` now takes and logs a seed of its own when no seed was chosen.
  Found in the milestone 5 review, before any release.

- Limited-strength play under a clock looked only at the first root moves
  in generation order, and with very little time played the first move
  unsearched with a made-up score of 0. It now scores every move in passes
  of increasing depth and chooses from the last pass that finished. Found in
  the milestone 5 review, before any release.

- A stalemate at the search horizon was scored as the static evaluation, so
  a one-ply search could play a stalemating move and report a won position.
  Quiescence search now scores stalemate as a draw. Found in the milestone 4
  review, before any release.

- A `Search` that had been stopped by a limit could not be used again:
  `iterate` panicked and `fixed_depth` returned no move. Each call now starts
  a new search. Found in the milestone 4 review, before any release.

- `Position::from_fen` accepted positions with more material than a game can
  produce (for example 26 queens), which could generate more than 256 moves
  and abort in `MoveList`. Such positions are now rejected with
  `FenError::ImpossibleMaterial`. Found before any release.

- `Position::put_piece` and `remove_piece` left a stale hash when a pawn was
  added or removed beside an en passant square. Found before any release.

- `scripts/install_tools.py`: the lichess-bot service it writes restarted
  the bot for ever, every ten seconds, whatever went wrong. It now restarts
  only after a failure, 30 seconds apart, and gives up after three failed
  starts in ten minutes, so a broken bot cannot abort game after game.

- `scripts/install_tools.py`: the next steps printed after installing
  lichess-bot no longer tell the user to enable the service at once. They
  now say to set `config.yml` up as `docs/LICHESS.md` describes before the
  first start, and to enable the service only after clean games from a
  terminal.

- `scripts/install_tools.py`: a `$` in the install path is now escaped in the
  systemd unit's `ExecStart`, where systemd would have expanded it.

- `scripts/install_tools.py`: the token file written with `--systemd` no
  longer sets an empty `LICHESS_BOT_TOKEN`, which lichess-bot would have used
  in place of a token kept in `config.yml`. The line is commented out.

- `scripts/install_tools.py`: a download that is redirected away from HTTPS
  is now refused; only the first address was checked before.

- `scripts/install_tools.py`: archive extraction now refuses paths that go
  through a link inside the archive, which closes an escape through chained
  symlinks on Python versions without `tarfile.data_filter`.

- `scripts/install_tools.py`: a reinstall interrupted while swapping
  directories no longer costs the previous install (and its `config.yml`) on
  the next run; it is restored from the `.old` directory first.

- `scripts/install_tools.py`: the `vyvrael-lichess-bot` launcher now changes
  into lichess-bot's directory first, so it finds `config.yml` from anywhere.

- `scripts/install_tools.py`: reinstalling lichess-bot no longer deletes the
  user's own top-level files, and the default engine path is now
  `<prefix>/engines/vyvrael`, outside the directory a reinstall replaces.

- `scripts/install_tools.py`: `--dry-run` now reports an already installed
  tool as such, as a real run does, instead of saying it would install it.

- `scripts/install_tools.py`: a re-run over an installed tool now rewrites
  its launchers and honours `--bin-dir` and `--systemd`, says when `--engine`
  was not applied, and no longer counts a tool as installed when its files
  are gone or the pinned commit or digest changed.

- `scripts/install_tools.py`: a tool that cannot be started by its
  post-install check (it cannot be run, times out, or exits with 126 or 127)
  is no longer recorded as installed and the run now exits with an error;
  Cute Chess is refused up front on machines that are not x86_64.

### Added

- Cargo workspace: engine crate `vyvrael` (library + binary) and stub tool
  crates `vyvrael-data` and `vyvrael-train`.
- Engine identity constants (`Vyvræl`, ASCII fallback `Vyvrael`).
- Stable toolchain pin, release profile (LTO, one codegen unit,
  `panic = "abort"`), `.gitignore` and `.gitattributes`.
- GitHub Actions CI running `cargo fmt --check`, `cargo clippy -D warnings`
  and `cargo test --release`.
- GPL-3.0-or-later licence (`LICENSE.md`).
- `rng` module: in-house SplitMix64 generator, `DEFAULT_SEED = 0x31031996`
  and golden-value tests.
- `Rng::stream`: deterministic derived streams from a base seed and an index.
- `Rng::next_below`: unbiased bounded integers.
- `board` module with primitive types: `Color`, `Square`, `PieceKind`,
  `Piece`, `Bitboard` (with file/rank masks and directional shifts),
  `CastlingRights`, `Move` (16-bit encoding with UCI display).
- `Position`: piece placement as bitboards plus a square array, with the
  game-state fields and accessors.
- FEN parsing: `Position::from_fen`, `Position::startpos`, `FenError`, with
  validation of king counts, pawn ranks, castling rights and the en passant
  square.
- FEN serialisation: `Position::to_fen` and `Display`, with round-trip tests.
- Zobrist keys (`board::zobrist`), generated at compile time from stream
  `rng::stream::ZOBRIST` of the default seed.
- `Position::hash`: Zobrist hash maintained incrementally, with
  `compute_hash` as the from-scratch reference. The en passant file is hashed
  only when an en passant capture is legal.
- `Position::make_move` / `unmake_move` with an `Undo` token, for every move
  kind: quiet moves, captures, double pawn pushes, en passant, castling and
  promotions.
- `Position::parse_move`: reads UCI coordinate notation and infers the move
  kind from the position.
- Whole-game regression tests (`tests/board_games.rs`).
- Documentation set: `README.md`, `AGENTS.md`, `CLAUDE.md`,
  `CONTRIBUTING.md`, this changelog and `docs/` (architecture, UCI, testing,
  roadmap, dependencies, data, NNUE).
- `attacks` module: knight attack table.
- King attack table.
- Pawn attack tables for both colours.
- Slider attacks by ray scanning (`bishop_attacks_by_scan`,
  `rook_attacks_by_scan`), the reference for the magic tables.
- Relevant-occupancy masks and subset enumeration for magic bitboards.
- Magic bitboards: `bishop_attacks`, `rook_attacks`, `queen_attacks`, with
  magic numbers found by the seeded RNG (`rng::stream::MAGICS`).
- `Position::attackers_to`, `is_attacked_by`, `checkers` and `in_check`.
- `movegen` module with `MoveList`, a fixed-capacity stack-allocated list.
- `pseudo_legal_moves`: knight, bishop, rook, queen and king moves, all pawn
  moves (pushes, captures, promotions, en passant) and castling.
- `legal_moves` and `Position::can_capture_king`.
- `perft` module: `perft` and `divide`.
- `perft` subcommand of the binary, with `--divide`, and a `help` command.
- Perft acceptance tests against published node counts (`tests/perft.rs`).
- Tree-walking consistency test of make/unmake, hashing and FEN
  (`tests/consistency.rs`).
- FEN validation rejects positions where the side not to move is in check
  (`FenError::OpponentInCheck`).
- `perft_copy_make` and `perft --copy-make`, with a recorded speed comparison
  against make/unmake.
- `eval` module: the `Evaluator` trait, `Score` and `relative_to`.
- Material evaluation: `piece_value`, `material_balance` and the `Material`
  evaluator.
- Game phase (`game_phase`, 0 to 24).
- `Tapered` middlegame/endgame score pairs with phase blending.
- Piece-square tables (`piece_square`, `piece_square_balance`), for all six piece kinds.
- `HandCrafted` evaluator (material plus tapered piece-square tables), its
  `Breakdown`, and `default_evaluator`.
- `eval` subcommand of the binary, printing the evaluation and its parts.
- `scripts/install_tools.py`: a Linux installer for pinned versions of Cute
  Chess, Ordo and lichess-bot, with offline tests. Not yet run end to end.
- `Evaluator` now requires `Send + Sync`, so it can be used from the search
  thread.
- `search` module: score constants and mate-score arithmetic.
- Fixed-depth negamax search with alpha-beta pruning (`search::Search`).
- Draw detection in search: insufficient material, repetition (with game
  history through `Search::with_history`) and the fifty-move rule.
- The principal variation (`Outcome::pv`).
- Search limits (`Limits`: depth, nodes, deadline) and an external stop flag.
- Iterative deepening (`Search::iterate`) with a progress `Report` per depth.
- Quiescence search: captures and queen promotions are played out before a
  position is evaluated, most valuable victim first.
- `search` subcommand of the binary: fixed-depth search with a line per depth.
- Pinned fixed-depth search results (`tests/search_pins.rs`).
- The search plays moves on copies of the position (copy-make) after
  measuring it 3 to 5% faster than make/unmake in the search loop.
- Brand assets in `branding/`: the engine's avatar and the NXD fractal
  background, all rights reserved and outside the code's GPL.
- The 128 magic numbers are embedded as constants (`attacks/magics.rs`), so
  the engine no longer searches for them at start-up. A test proves they are
  exactly what the seeded search finds.
- `movegen::find_legal_move`: turns a move written in UCI notation into a
  move only if it is legal. This is the entry point for input that is not
  trusted; `Position::parse_move` stays for trusted input such as recorded
  games.
- `uci`: parsing of UCI command lines into commands (`uci::parse`).
- `uci`: formatting of `info` and `bestmove` lines and of scores
  (`cp` and `mate`).
- `uci`: time management. A move gets an equal share of the remaining time
  (over `movestogo` or 30 moves) plus three quarters of the increment, never
  more than is on the clock less a 30 ms overhead.
- **The engine speaks UCI.** Started without arguments, the binary reads
  UCI commands (`uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`,
  `quit`) and searches on its own thread.
- UCI option `Hash` (spin, default 16, 1 to 65536 MiB). It is accepted and
  stored but has no effect until there is a transposition table
  (milestone 7).
- UCI option `Threads` (spin, default 1, minimum 1, maximum 1): the search
  is single-threaded, so 1 is the only value accepted.
- `rng::entropy_seed`, the one permitted reader of system entropy, and
  `rng::stream::STRENGTH`, the stream for randomness that affects playing
  strength, used by the UCI layer.
- UCI options `DeterministicRNG` (check, default `false`) and `RngSeed`
  (spin, default 822286742, which is `0x31031996`). The seed of
  strength-affecting randomness comes from system entropy unless
  `DeterministicRNG` is on, and is logged with `info string` in reply to
  `uci`, at every `ucinewgame` without a chosen seed, and whenever an
  option changes it.
- `search::weakened_move`: chooses a move at reduced strength by searching
  every root move to a shallow depth and misjudging each score by a bounded
  random amount. Used when `UCI_LimitStrength` is on.
- UCI option `UCI_LimitStrength` (check, default `false`). When on, `go`
  plays a weakened move at the level set by `UCI_Elo`. When off, play is
  exactly as before.
- UCI option `UCI_Elo` (spin, default 1200, 400 to 1800): the rating to play
  at when `UCI_LimitStrength` is on. **Not calibrated**: the settings are
  ordered, but what rating each really plays at is measured in milestone 6.
- Integration tests that drive the engine binary over pipes
  (`tests/uci.rs`).
- Added Ladder Baseline 1 reference results (docs/testing/)
- `docs/LICHESS.md`: the Lichess bot policy, the facts and rules it rests
  on, a checklist for the maintainer, and the anti-abuse rules.
- UCI option `Move Overhead` (spin, default 30, 0 to 5000 ms): time kept
  back from every time limit for delays the engine cannot measure, such as
  the network between a Lichess game and the engine.

[Unreleased]: https://github.com/NexxusDrakoMultimedia/Vyvrael-Chess/compare/v0.1.0-alpha.2...HEAD
[0.1.0-alpha.2]: https://github.com/NexxusDrakoMultimedia/Vyvrael-Chess/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/NexxusDrakoMultimedia/Vyvrael-Chess/releases/tag/v0.1.0-alpha.1
