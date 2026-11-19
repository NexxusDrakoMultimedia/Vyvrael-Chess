# Changelog

All notable changes to Vyvræl are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project intends to follow [Semantic Versioning](https://semver.org/)
once it has a first release.

## [Unreleased]

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
- FEN parsing: `Position::from_fen`, `Position::startpos`, `FenError`.
- Documentation set: `README.md`, `AGENTS.md`, `CLAUDE.md`,
  `CONTRIBUTING.md`, this changelog and `docs/` (architecture, UCI, testing,
  roadmap, dependencies, data, NNUE).
