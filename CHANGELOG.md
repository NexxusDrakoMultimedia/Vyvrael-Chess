# Changelog

All notable changes to Vyvræl are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project intends to follow [Semantic Versioning](https://semver.org/)
once it has a first release.

## [Unreleased]

### Fixed

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
- `uci`: parsing of UCI command lines into commands (`uci::parse`). Nothing
  acts on them yet.
- `uci`: formatting of `info` and `bestmove` lines and of scores
  (`cp` and `mate`).
- `uci`: time management. A move gets an equal share of the remaining time
  (over `movestogo` or 30 moves) plus three quarters of the increment, never
  more than is on the clock less a 30 ms overhead.
- **The engine speaks UCI.** Started without arguments, the binary reads
  UCI commands (`uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`,
  `quit`) and searches on its own thread. No options yet.
- UCI option `Hash` (spin, default 16, 1 to 65536 MiB). It is accepted and
  stored but has no effect until there is a transposition table
  (milestone 7).
- UCI option `Threads` (spin, default 1, minimum 1, maximum 1): the search
  is single-threaded, so 1 is the only value accepted.
- `rng::entropy_seed`, the one permitted reader of system entropy, and
  `rng::stream::STRENGTH`, the stream for randomness that affects playing
  strength. Nothing uses them yet.
- UCI options `DeterministicRNG` (check, default `false`) and `RngSeed`
  (spin, default 822286742, which is `0x31031996`). The seed of
  strength-affecting randomness comes from system entropy unless
  `DeterministicRNG` is on, and is logged with `info string` in reply to
  `uci` and whenever it changes. No randomness is drawn yet.
- `search::weakened_move`: chooses a move at reduced strength by searching
  every root move to a shallow depth and misjudging each score by a bounded
  random amount. Not reachable from UCI yet.
- UCI option `UCI_LimitStrength` (check, default `false`). When on, `go`
  plays a weakened move, at a fixed provisional level until `UCI_Elo`
  exists. When off, play is exactly as before.
- UCI option `UCI_Elo` (spin, default 1200, 400 to 1800): the rating to play
  at when `UCI_LimitStrength` is on. **Not calibrated**: the settings are
  ordered, but what rating each really plays at is measured in milestone 6.
- Integration tests that drive the engine binary over pipes
  (`tests/uci.rs`).
- Added Ladder Baseline 1 reference results (docs/testing/)
