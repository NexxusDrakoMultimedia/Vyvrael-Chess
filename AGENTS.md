# AGENTS.md

Canonical instructions for AI coding agents working on Vyvræl, imported by
`CLAUDE.md`. Keep this the single source of truth, under about 200 lines.

## Project overview

Vyvræl is a UCI-compatible chess engine written in Rust by Nexxus Drako
Multimedia, licensed GPL-3.0-or-later. The official tagline is "A
dragon-flavoured UCI chess engine in Rust, built with AI and playing for the
attack." Use it word for word wherever the project is described in one line.
"Playing for the attack" is a style goal, not a strength claim: never call
Vyvræl strong or give it a rating that has not been measured (milestone 6).

The project is labelled **X** on the [0rwx scale](https://0rwx.com/): the
maintainer curates agent output rather than reading every line, so tests, docs
and a clean commit history are the source of trust, and part of the product.

Layout:

- `src/lib.rs`, `src/main.rs`: the engine crate `vyvrael` (library + binary).
  It has no dependencies and must never depend on the tool crates. Under
  `src/`: `rng.rs` (the only source of randomness), `board/` (types, position,
  FEN, Zobrist hashing, make/unmake, move parsing), `attacks/` (attack tables
  per piece type), `movegen/` (move generation), `perft.rs` (perft node
  counting), `eval/` (static evaluation), `search/`, `uci/` (parsing, options,
  time, engine loop), `bench.rs` (a fixed search workload) and `cli.rs` (the
  binary's other commands: `perft`, `eval`, `search`, `bench`, `help`).
- `tests/`: integration tests that use only the public API.
- `tools/vyvrael-data`: data and database tooling (milestone 8).
- `tools/vyvrael-train`: NNUE training (milestone 9).
- `scripts/`: the tool and ladder-engine installer, the match and rating
  scripts (`linux/`, `windows/`), `local.env.example`, and their tests.
- `branding/`: brand assets, all rights reserved, not under the GPL.
- `docs/`: architecture, UCI, testing, roadmap, releasing, dependencies,
  data, NNUE, Lichess and the bot's build history. `docs/prompts/` holds the
  default agent prompts (run through `.claude/skills/`), `docs/testing/` the
  measured results and `docs/releases/` the release notes.

## Commands

| Task | Command |
| --- | --- |
| Format check | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace --release` |
| Script tests | `python3 -m unittest discover -s scripts/tests` (`python` on Windows; takes several minutes) |
| Install Cute Chess and Ordo (Linux, Windows; lichess-bot on Linux only) | `python3 scripts/install_tools.py --dry-run` first (`python` on Windows); see `docs/TESTING.md` |
| Install the ladder's engines (Linux, Windows) | `python3 scripts/install_tools.py --engines --dry-run` first (`python` on Windows); `--only "<name>"` for one; `--verify` to check them; `--engine-build pext` or `ENGINE_BUILD=pext` in `scripts/local.env` for PEXT builds |
| Build engine | `cargo build --release` (binary: `target/release/vyvrael`) |
| Perft | `cargo run --release -- perft <depth> [--divide \| --copy-make] [FEN]` |
| Evaluate or search a position | `cargo run --release -- eval [FEN]`; `cargo run --release -- search <depth> [FEN]` |
| Bench (node count and speed) | `cargo run --release -- bench [depth]` |
| Run as a UCI engine; usage | `cargo run --release` (no arguments; see `docs/UCI.md`); `cargo run --release -- help` lists the other commands |
| Run a match | Linux: `scripts/linux/match.sh sprt\|h2h\|gauntlet ... --dry-run` first, see `--help`; Windows: `scripts/windows/match.ps1 sprt\|h2h\|gauntlet ... -DryRun` first, see `-Help` |
| Rate PGN files | Linux: `scripts/linux/rate.sh [options] FILE.pgn ...`, see `--help`; Windows: `scripts/windows/rate.ps1 [options] FILE.pgn ...`, see `-Help` |
| Data tools; training | `cargo run --release -p vyvrael-data` (stub until milestone 8); `cargo run --release -p vyvrael-train` (stub until milestone 9) |

Update this table in the same commit that adds or changes a command.

## Conventions

- Rust edition 2024, stable toolchain (see `rust-toolchain.toml`).
- Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`
  and `// Copyright (C) <year> Nexxus Drako Multimedia`.
- Public items have `///` doc comments.
- No `unsafe` unless it measurably helps; comment every use.
- UCI stdout carries protocol output only. Debug output goes to stderr or
  `info string`. Flush stdout after every line.
- The UCI `id name` is `Vyvræl <version>` (`Vyvrael` is the ASCII fallback
  and the crate name).
- British spelling in prose is fine; identifiers follow Rust conventions.

## Rules

- Never weaken, skip or delete a test to make it pass. Fix the code.
- Every commit is green (`fmt`, `clippy -D warnings`, `test --release`) and
  does one thing.
- Commit locally. At the end of a milestone, review it independently, fix
  what the review confirms, then merge its branch into `main` locally. Then
  stop and report before starting the next milestone.
- Never push, force-push, tag or release on your own. Ask the maintainer when
  it is the right moment (for example after a merge into `main`) and act only
  on an explicit yes, each time. Do not rewrite history unless the maintainer
  asks.
- Training and data work must be reproducible: fixed seeds, recorded
  parameters, and a manifest committed with any net's documentation (not the
  net itself unless it is small and the maintainer approves).
- A new net, eval change or search change is accepted only if SPRT against
  the previous version shows a gain. Record the result in the commit message
  or `docs/TESTING.md`. Exempt: options that weaken play on purpose and are
  off by default, `UCI_LimitStrength` / `UCI_Elo` and `Variety`. With them
  at their defaults play must be unchanged, shown by an unchanged bench node
  count, and they stay at their defaults in every strength match.
- Before adding a dependency, check its licence is GPL-3.0-or-later
  compatible and record it in `docs/DEPENDENCIES.md`. One dependency per
  commit. Prefer none in the engine crate.
- Never commit large third-party artefacts (datasets, tablebases, nets,
  opening books). Record source URL, licence and date fetched in
  `docs/DATA.md`, confirmed on the official source page.
- Endgame tablebases (milestone 10) are optional: ask the maintainer first.
- Never touch Lichess: no API token, no API call, no account upgrade, no
  launching lichess-bot, not in CI either. See `docs/LICHESS.md`.

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
  non-deterministic by default, with an opt-in: the UCI spin option `Seed` (0
  to 2147483647, default `0`). `Seed` 0 takes a seed from 1 to 2147483647 from
  system entropy at startup and at every `ucinewgame` (the one allowed use)
  and logs it with `info string`; any other `Seed` is that seed. Determinism
  means one thread (multithreaded search is not bit-for-bit reproducible; see
  `docs/TESTING.md`), a fixed `Seed`, `Variety` 0 and a fixed depth or node
  count. Detail: `docs/ARCHITECTURE.md`, `docs/UCI.md`.
- **Non-strength-affecting randomness** (Zobrist keys, magic search, test
  fixtures, property tests, datagen openings, shuffles) is always
  deterministic from the default seed or a derived stream. Tools may take
  `--seed`, defaulting to `0x31031996`.
- Classify every use in the table in `docs/ARCHITECTURE.md` with a one-line
  justification. If unsure, treat it as strength-affecting and tell the
  maintainer.

## Chunk-and-commit workflow

- A chunk is one logical change that builds, passes tests and could be
  reverted on its own. If it needs "and" to describe it, split it.
- Aim for diffs reviewable in a few minutes; avoid commits over about 300
  changed lines. If unavoidable (generated tables), say why in the message.
- Tests and docs land in the commit of the code they cover (see below).
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
  `milestone/2-movegen`, ...), each started from `main` once the previous
  one is merged; the agent reviews and merges it (see "Rules"). Release
  preparation (`release/<version>`) and tooling (`tooling/<name>`) get
  branches of their own; the maintainer reviews a release branch and tags.
- Reporting: at the end of each milestone report the branch name, its
  `git log --oneline`, the tests that cover it, and any doc/code mismatches.

## Build order

Milestones, in order, no skipping (details in `docs/ROADMAP.md`):

1. **Board:** `rng`, bitboards, FEN, make/unmake, Zobrist hashing.
2. **Move generation:** magic bitboards, legal moves, perft acceptance tests.
3. **Evaluation (hand-crafted):** material, tapered PSTs, behind an interface.
4. **Search core:** negamax/alpha-beta, iterative deepening, quiescence.
   Heuristics wait for milestone 7 so each can be accepted by SPRT.
5. **UCI:** the protocol loop and its options (see `docs/UCI.md`).
6. **Strength testing:** measure Vyvræl against the opponent ladder with
   the match and rating scripts (Cute Chess, Ordo); calibrate `UCI_Elo`.
7. **Search heuristics:** move ordering, transposition table, null move,
   LMR. One feature per commit, each with an SPRT result.
8. **Open data tooling:** Lichess PGN/puzzle/eval readers, opening EPDs.
9. **NNUE:** datagen, `bullet` training, quantised incremental inference.
10. **Tablebases (optional, ask first):** Syzygy probing.

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
  table in `docs/ROADMAP.md`; remove it in the commit that fixes it.

At the end of each milestone, re-read the docs against the code, fix any drift
in a separate `docs:` commit before reporting, and say explicitly in the
report where a doc and the code disagree. For that re-read, or any full
documentation pass, use the default prompt in
`docs/prompts/documentation-pass.md` (`/docs-pass` in Claude Code).

## Releases

Prepare a release only when the maintainer asks, following `docs/RELEASING.md`
(default prompt: `docs/prompts/release.md`, `/release-prep` in Claude Code).
Never tag, push or publish.
