# Changelog

All notable changes to Vyvræl are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project intends to follow [Semantic Versioning](https://semver.org/)
once it has a first release.

## [Unreleased]

### Fixed

- `Position::from_fen` accepted positions with more material than a game can
  produce (for example 26 queens), which could generate more than 256 moves
  and abort in `MoveList`. Such positions are now rejected with
  `FenError::ImpossibleMaterial`. Found before any release.

- `Position::put_piece` and `remove_piece` left a stale hash when a pawn was
  added or removed beside an en passant square. Found before any release.

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
