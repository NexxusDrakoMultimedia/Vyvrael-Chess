# Changelog

All notable changes to Vyvræl are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project intends to follow [Semantic Versioning](https://semver.org/)
once it has a first release.

## [Unreleased]

### Fixed

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
  only when an en passant capture is pseudo-legally possible.
- `Position::make_move` / `unmake_move` with an `Undo` token, for quiet moves,
  captures, double pawn pushes, en passant and castling.
- Documentation set: `README.md`, `AGENTS.md`, `CLAUDE.md`,
  `CONTRIBUTING.md`, this changelog and `docs/` (architecture, UCI, testing,
  roadmap, dependencies, data, NNUE).
