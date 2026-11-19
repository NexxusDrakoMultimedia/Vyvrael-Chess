# AGENTS.md

Canonical instructions for AI coding agents working on Vyvræl. `CLAUDE.md`
imports this file; keep this the single source of truth and under about 200
lines.

## Project overview

Vyvræl is a UCI-compatible chess engine written in Rust by Nexxus Drako
Multimedia. It has a dragon theme and an aggressive playing style. It is
licensed GPL-3.0-or-later.

The project is labelled **X** on the [0rwx scale](https://0rwx.com/): the
maintainer curates agent output rather than reading every line. Tests,
documentation and a clean commit history are therefore the source of trust.
Treat them as part of the product.

Layout:

- `src/lib.rs`, `src/main.rs`: the engine crate `vyvrael` (library + binary).
  It has no dependencies and must never depend on the tool crates.
  - `src/rng.rs`: the only source of randomness.
  - `src/attacks/`: attack tables per piece type.
  - `src/movegen/`: move generation.
  - `src/eval/`: static evaluation.
  - `src/perft.rs`: perft node counting.
  - `src/cli.rs`: the binary's command line (`main.rs` only calls it).
  - `src/board/`: board representation (types, position, FEN, Zobrist
    hashing, make/unmake, move parsing).
- `tests/`: integration tests that use only the public API.
- `tools/vyvrael-data`: data and database tooling (milestone 7).
- `tools/vyvrael-train`: NNUE training (milestone 8).
- `docs/`: architecture, UCI, testing, roadmap, dependencies, data, NNUE.

## Commands

| Task | Command |
| --- | --- |
| Format check | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace --release` |
| Build engine | `cargo build --release` (binary: `target/release/vyvrael`) |
| Perft | `cargo run --release -- perft <depth> [--divide \| --copy-make] [FEN]` |
| Evaluate a position | `cargo run --release -- eval [FEN]` |
| Match script | not implemented yet (milestone 6) |
| Data tools | `cargo run --release -p vyvrael-data` (stub until milestone 7) |
| Training | `cargo run --release -p vyvrael-train` (stub until milestone 8) |

Update this table in the same commit that adds or changes a command.

## Conventions

- Rust edition 2024, stable toolchain (see `rust-toolchain.toml`).
- Every source file starts with:
  `// SPDX-License-Identifier: GPL-3.0-or-later` and
  `// Copyright (C) <year> Nexxus Drako Multimedia`.
- Public items have `///` doc comments.
- No `unsafe` unless it measurably helps; comment every use.
- UCI stdout carries protocol output only. Debug output goes to stderr or
  `info string`. Flush stdout after every line.
- The UCI `id name` is `Vyvræl` (`Vyvrael` is the ASCII fallback and the crate
  name).
- British spelling in prose is fine; identifiers follow Rust conventions.

## Rules

- Never weaken, skip or delete a test to make it pass. Fix the code.
- Every commit is green (`fmt`, `clippy -D warnings`, `test --release`) and
  does one thing.
- Commit locally. At the end of a milestone, review it independently, fix
  what the review confirms, then merge its branch into `main` locally.
- Never push, force-push, tag or release on your own. Ask the maintainer when
  it is the right moment (for example after a merge into `main`) and act only
  on an explicit yes, each time. Do not rewrite history unless the maintainer
  asks.
- Stop and report at the end of each milestone before starting the next.
- Training and data work must be reproducible: fixed seeds, recorded
  parameters, and a manifest committed with any net's documentation (not the
  net itself unless it is small and the maintainer approves).
- A new net, eval change or search change is accepted only if SPRT against
  the previous version shows a gain. Record the result in the commit message
  or `docs/TESTING.md`.
- Before adding a dependency, check its licence is GPL-3.0-or-later
  compatible and record it in `docs/DEPENDENCIES.md`. One dependency per
  commit. Prefer none in the engine crate.
- Never commit large third-party artefacts (datasets, tablebases, nets,
  opening books). Record source URL, licence and date fetched in
  `docs/DATA.md`, confirmed on the official source page.
- Endgame tablebases (milestone 9) are optional: ask the maintainer first.

## Randomness

All randomness goes through the `rng` module of the engine crate (mirrored or
shared in the tool crates).

- **Default seed:** `DEFAULT_SEED: u32 = 0x31031996`, defined once. Never
  scatter the literal.
- **In-house PRNG:** no `rand` crate. Sequences must be identical across
  platforms and dependency upgrades, with golden-value tests.
- **No ambient entropy:** no system entropy, time-based seeding or unseeded
  RNG anywhere, except the single allowed path below.
- **Seed derivation:** independent streams (threads, games, subsystems) get
  seeds derived deterministically from the base seed and an index. Never share
  one RNG across threads.
- **Strength-affecting randomness** (move-choice randomisation, opening
  variety, eval noise, skill weakening, random book choice) is
  non-deterministic by default with an opt-in:
  - UCI check option `DeterministicRNG` (default `false`) and spin option
    `RngSeed` (default `0x31031996`);
  - `DeterministicRNG = false`: seed from system entropy at startup (the one
    allowed use) and log the seed with `info string`;
  - `DeterministicRNG = true`: seed from `RngSeed`.
- **Non-strength-affecting randomness** (Zobrist keys, magic search, test
  fixtures, property tests, datagen openings, shuffles) is always
  deterministic from the default seed or a derived stream. Tools may take
  `--seed`, defaulting to `0x31031996`.
- Classify every use in the table in `docs/ARCHITECTURE.md` with a one-line
  justification. If unsure, treat it as strength-affecting and tell the
  maintainer.
- Multithreaded search is not bit-for-bit reproducible; see
  `docs/TESTING.md` for reproducible-run settings.

## Chunk-and-commit workflow

- A chunk is one logical change that builds, passes tests and could be
  reverted on its own. If it needs "and" to describe it, split it.
- Aim for diffs reviewable in a few minutes; avoid commits over about 300
  changed lines. If unavoidable (generated tables), say why in the message.
- Tests land in the same commit as the code they cover.
- Docs travel with the code (see "Documentation upkeep").
- Conventional Commits: `feat(board): add FEN parsing`,
  `test(movegen): add Kiwipete perft`, `docs: ...`, `chore: ...`, `fix: ...`,
  `refactor: ...`. Short subject, then a body explaining what and why when it
  is not obvious. The 0rwx disclosure lives in the README only.
- Stage specific files. Never `git add -A` or `git add .` blindly; check
  `git status` and `git diff --staged` first.
- Plan before coding: at the start of each milestone, write its chunk
  checklist into `docs/ROADMAP.md` and commit it, then tick items off in the
  commits that complete them.
- Branching: one branch per milestone (`milestone/1-board`,
  `milestone/2-movegen`, ...), each started from the previous milestone's
  branch. The agent reviews and merges it into `main` (see "Rules").
- Reporting: at the end of each milestone report the branch name, its
  `git log --oneline`, the tests that cover it, and any doc/code mismatches.

## Build order

Milestones, in order, no skipping (details in `docs/ROADMAP.md`):

1. **Board:** `rng`, bitboards, FEN, make/unmake, Zobrist hashing.
2. **Move generation:** magic bitboards, legal moves, perft acceptance tests.
3. **Evaluation (hand-crafted):** material, tapered PSTs, behind an interface.
4. **Search core:** negamax/alpha-beta, iterative deepening, quiescence.
   Heuristics wait for 4b so each can be accepted by SPRT.
5. **UCI:** full protocol loop with `Hash`, `Threads`, `DeterministicRNG`,
   `RngSeed`, `UCI_LimitStrength`, `UCI_Elo`.
6. **Strength testing:** SPRT match script (fastchess or Cutechess);
   calibrate `UCI_Elo`.
   - **4b, search heuristics** (after 6): move ordering, transposition table,
     null move, LMR. One feature per commit, each with an SPRT result.
7. **Open data tooling:** Lichess PGN/puzzle/eval readers, opening EPDs.
8. **NNUE:** datagen, `bullet` training, quantised incremental inference.
9. **Tablebases (optional, ask first):** Syzygy probing.

Style goal: aggressive play (king attacks, initiative, piece activity), but
never at the cost of correctness.

## Documentation upkeep

Documentation is part of the definition of done. Every commit that changes
behaviour must also:

- update `CHANGELOG.md` (Unreleased section);
- update the affected docs: `docs/ARCHITECTURE.md` (structure, randomness
  table), `docs/UCI.md` (commands/options), `docs/TESTING.md` (tests, scripts),
  `docs/DATA.md` (datasets), `docs/NNUE.md` (networks), `docs/ROADMAP.md`
  (chunk status), and the README status line;
- update this file if a command, convention or the build order changed;
- record any limitation you knowingly leave behind in the "Known issues"
  table in `docs/ROADMAP.md`, and remove the entry in the commit that fixes
  it;
- keep `///` doc comments on public items.

At the end of each milestone, re-read the docs against the code and fix any
drift in a separate `docs:` commit before reporting. If a doc and the code
disagree, say so explicitly in the report.

For that re-read, or whenever a full documentation pass is asked for, use the
default prompt in `docs/prompts/documentation-pass.md` (`/docs-pass` in Claude
Code).

## Releases

Prepare a release only when the maintainer asks. Use the default prompt in
`docs/prompts/release.md` (`/release-prep` in Claude Code). Never tag, push or
publish.
