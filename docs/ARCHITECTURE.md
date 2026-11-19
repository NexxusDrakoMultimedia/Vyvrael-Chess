# Architecture

This document describes how Vyvræl is put together and why. It is updated in
the same commit as any structural change. Sections marked *planned* describe
intent for a later milestone, not code that exists.

## Workspace layout

```
Cargo.toml              workspace root + engine package `vyvrael`
src/lib.rs              engine library (all engine logic lives here)
src/main.rs             engine binary (thin wrapper over the library)
tools/vyvrael-data/     data and database tooling (stub; milestone 7)
tools/vyvrael-train/    NNUE training (stub; milestone 8)
docs/                   documentation
```

The engine is a library plus a thin binary so that integration tests, the
binary and the tool crates all exercise one implementation.

**Dependency direction:** the tool crates may depend on the engine crate. The
engine crate must never depend on a tool crate, and aims for zero
dependencies, so the engine binary stays small and builds fast.

## Engine modules

| Module | Status | Responsibility |
| --- | --- | --- |
| crate root (`lib.rs`) | implemented | Engine identity constants (`ENGINE_NAME`, `ENGINE_NAME_ASCII`, `ENGINE_AUTHOR`, `ENGINE_VERSION`). |
| `rng` | implemented | The only source of randomness: `Rng` (SplitMix64) with `next_u64`, `next_u32`, `next_below` and `Rng::stream`; `DEFAULT_SEED`; `mix64`. |
| `board` | in progress (milestone 1) | Board representation. Implemented so far: `Color`, `Square`, `PieceKind`, `Piece`, `Bitboard`, `CastlingRights`, `Move`. Still planned: position, FEN, make/unmake, Zobrist hashing. |
| move generation | planned (milestone 2) | Magic bitboards, legal moves, perft. |
| evaluation | planned (milestone 3) | Hand-crafted eval behind an interface a network can replace. |
| search | planned (milestone 4) | Alpha-beta search and its heuristics. |
| UCI | planned (milestone 5) | Protocol loop; search on its own thread. |

## Board representation

Implemented in `src/board/`, one small file per type, re-exported from
`board`. This section grows as milestone 1 lands.

- **`Color`** (`color.rs`): `White = 0`, `Black = 1`, so it indexes arrays
  directly. `flip()` and `!` give the other side.
- **`Square`** (`square.rs`): a `u8` newtype in `0..64` using little-endian
  rank-file mapping (`a1 = 0`, `h1 = 7`, `a8 = 56`, `h8 = 63`;
  `index = rank * 8 + file`). The index is also the square's bit position in
  a bitboard. Files and ranks are plain `u8` values in `0..8`. `offset` steps
  by files and ranks and returns `None` at the board edge, so it cannot wrap.
- **`PieceKind`** and **`Piece`** (`piece.rs`): `PieceKind` is
  `Pawn = 0 .. King = 5` for array indexing; `Piece` pairs a `Color` with a
  `PieceKind` and converts to and from FEN characters (`PNBRQK` / `pnbrqk`).
- **`Bitboard`** (`bitboard.rs`): a `u64` newtype holding a set of squares,
  bit `i` for square index `i`. It supports the set operators (`&`, `|`, `^`,
  `!`), membership edits, `lsb`/`msb`/`pop_lsb`, and iteration over member
  squares in ascending index order. Its `Debug` output is an 8x8 diagram with
  rank 8 at the top. `Bitboard::file` and `Bitboard::rank` give line masks,
  and the eight one-step shifts (`north`, `south`, `east`, `west` and the four
  diagonals) move a whole set at once; east/west shifts mask the edge file
  first so squares never wrap around the board.
- **`CastlingRights`** (`castling.rs`): four flag bits (`K = 1`, `Q = 2`,
  `k = 4`, `q = 8`), so a value is also an index in `0..16`. A right only
  records that king and rook are still on their home squares and unmoved;
  legality of castling is move generation's concern.
  `CastlingRights::revoked_by(square)` gives the rights lost when any piece
  moves from or to a square, which lets make-move update rights with two
  lookups (origin and destination) and covers king moves, rook moves and rook
  captures alike. Standard chess only: Chess960 castling is not supported.
- **`Move`** and **`MoveKind`** (`mv.rs`): a move is 16 bits: origin square
  (bits 0-5), destination square (bits 6-11) and a 4-bit flag code (bits
  12-15). Flag codes: `0` quiet, `1` double pawn push, `2` king-side castle,
  `3` queen-side castle, `4` capture, `5` en passant, `8-11` promotion to
  knight/bishop/rook/queen, `12-15` capturing promotion to the same four.
  Codes `6` and `7` are unused and cannot be constructed. Castling is encoded
  as the king's move (`e1g1`). A move does not record which piece moves; that
  comes from the position. `Display` gives UCI coordinate notation (`e2e4`,
  `e7e8q`). Sixteen bits keeps move lists and transposition-table entries
  small.

## How search, evaluation and UCI fit together (planned)

The UCI front end owns the current position and the option values. On `go`
it hands a copy of the position and the search limits to a search thread, so
the main thread keeps reading stdin and can act on `stop`. Search calls the
evaluation through a trait, so the hand-crafted evaluation and the NNUE
evaluation are interchangeable and can be compared by SPRT. Only the UCI
layer writes to stdout.

## RNG design

Implemented in `src/rng.rs`.

All randomness in the engine goes through one module, `rng`. The tool crates
use it through their dependency on the engine crate (or mirror it exactly if
that dependency is ever undesirable).

- **Algorithm:** SplitMix64 (Steele, Lea and Flood, 2014; the reference
  implementation by Sebastiano Vigna is public domain). The state is one
  `u64`. Each output adds the constant `0x9E3779B97F4A7C15` to the state and
  returns a bijective mix of the new state (xor-shift by 30, multiply by
  `0xBF58476D1CE4E5B9`, xor-shift by 27, multiply by `0x94D049BB133111EB`,
  xor-shift by 31). It has a full period of 2^64, passes BigCrush, and is a
  few lines of integer arithmetic with identical results on every platform.
- **Why in-house:** generated tables (Zobrist keys, magic numbers) and test
  fixtures must never change because a dependency was upgraded. Owning the
  dozen lines of PRNG removes that risk and keeps the engine dependency-free.
- **Default seed:** `DEFAULT_SEED: u32 = 0x31031996`, defined once in `rng`
  and referenced by name everywhere. A 32-bit seed is zero-extended to the
  64-bit state.
- **Seed derivation:** independent streams (threads, self-play games,
  subsystems) are created with `Rng::stream(base_seed, index)`, never by
  sharing one generator across threads. The stream's starting state is
  `mix64(k ^ mix64(index + 0x9E3779B97F4A7C15))`, where `k` is the first
  output of `Rng::new(base_seed)`. Because `mix64` is a bijection, distinct
  indices always give distinct starting states for the same base seed. Streams
  are different offsets into SplitMix64's single 2^64 cycle; with
  well-mixed offsets, an overlap within any practical run length is
  vanishingly unlikely.
- **Bounded integers:** `next_below(bound)` returns a uniform integer in
  `0..bound` using Lemire's multiply-and-reject method, so there is no modulo
  bias for any bound. Use it for all sampling and shuffling.
- **Golden tests:** known-answer tests pin the first outputs for seed
  `1234567` (the published SplitMix64 reference vector), seed `0` and
  `DEFAULT_SEED`, plus four derived streams. The `DEFAULT_SEED` and stream
  values were computed with an independent implementation.
- **Compile-time use:** every method is a `const fn`, so tables can be
  generated at compile time with no start-up cost.
- **Ambient entropy:** forbidden everywhere except one path: seeding
  strength-affecting randomness when the UCI option `DeterministicRNG` is
  `false`. That path arrives with the UCI milestone and logs the seed it chose.

### Table of randomness uses

Every use of randomness is listed here and classified. "Strength-affecting"
means the randomness can change how strongly or how differently the engine
plays; such uses are non-deterministic by default with `DeterministicRNG` /
`RngSeed` as the opt-in. Everything else is always deterministic.

| Use | Status | Strength-affecting? | Seed | Justification |
| --- | --- | --- | --- | --- |
| *(none yet)* | | | | No code uses randomness yet. |

Planned uses (each gets a real row when it lands): Zobrist keys (milestone 1,
not strength-affecting), magic-number search (milestone 2, not
strength-affecting), test fixtures and property tests (not
strength-affecting), datagen opening randomisation (milestone 8, not
strength-affecting; per-game derived seeds).

## Design decisions

| Decision | Reason |
| --- | --- |
| Library + thin binary | One implementation shared by the binary, integration tests and tools. |
| Zero-dependency engine crate | Small binary, fast builds, no licence or supply-chain surprises, stable generated tables. |
| Tool crates separate from the engine | Heavy dependencies (zstd, trainers) never reach the engine binary. |
| In-house SplitMix64 | Cross-platform, upgrade-proof determinism; see "RNG design". |
| `panic = "abort"` in release | Smaller, faster code; an engine cannot usefully recover from a panic mid-search. |
| `Vyvræl` in UCI, `vyvrael` as crate/binary name | Cargo package names must be ASCII; the UCI name can be UTF-8, with `Vyvrael` as fallback. |
