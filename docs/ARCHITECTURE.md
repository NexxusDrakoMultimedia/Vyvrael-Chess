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
| `rng` | implemented | The only source of randomness: `Rng` (SplitMix64), `DEFAULT_SEED`, `mix64`. |
| board representation | planned (milestone 1) | Bitboards, position, FEN, make/unmake, Zobrist hashing. |
| move generation | planned (milestone 2) | Magic bitboards, legal moves, perft. |
| evaluation | planned (milestone 3) | Hand-crafted eval behind an interface a network can replace. |
| search | planned (milestone 4) | Alpha-beta search and its heuristics. |
| UCI | planned (milestone 5) | Protocol loop; search on its own thread. |

## How search, evaluation and UCI fit together (planned)

The UCI front end owns the current position and the option values. On `go`
it hands a copy of the position and the search limits to a search thread, so
the main thread keeps reading stdin and can act on `stop`. Search calls the
evaluation through a trait, so the hand-crafted evaluation and the NNUE
evaluation are interchangeable and can be compared by SPRT. Only the UCI
layer writes to stdout.

## RNG design

Implemented in `src/rng.rs`. Bullets marked *planned* are not in the code
yet.

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
- **Seed derivation (planned):** independent streams (threads, self-play games,
  subsystems) are created from a base seed and a stream index through a mixing
  function, never by sharing one generator across threads.
- **Golden tests:** known-answer tests pin the first outputs for seed
  `1234567` (the published SplitMix64 reference vector), seed `0` and
  `DEFAULT_SEED`. The `DEFAULT_SEED` values were computed with an independent
  implementation.
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
