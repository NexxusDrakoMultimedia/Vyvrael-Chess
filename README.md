# Vyvrael-Chess
Dragon-flavoured, aggressive, vibecoded UCI-compatible chess engine written in Rust.

## What it is

Vyvræl is a chess engine by Nexxus Drako Multimedia. It speaks the
[UCI protocol](docs/UCI.md), so it plugs into chess GUIs and into
`lichess-bot`. It aims for an aggressive style: king attacks, initiative and
piece activity, built on a foundation that is correct first.

The engine is a single dependency-free Rust crate (`vyvrael`). Two tool
crates under `tools/` will handle open-data processing (`vyvrael-data`) and
NNUE training (`vyvrael-train`).

## AI disclosure: 0rwx level X

This project is labelled **X** on the [0rwx scale](https://0rwx.com/)
("AI made, I curate"). The code is written by AI coding agents and curated by
the maintainer, who does not read every line. Trust therefore rests on the
test suite, the documentation and a small-commit history. See
[CONTRIBUTING.md](CONTRIBUTING.md) and [docs/TESTING.md](docs/TESTING.md).

## Status

**Pre-alpha: milestone 1 (board representation) is complete and merged; milestone 2 (move generation) is complete and merged; milestone 3 (hand-crafted evaluation) is complete and merged; milestone 4 (search) has not started.** The engine generates legal moves, passes perft and can score a position, but cannot
play chess yet. See [docs/ROADMAP.md](docs/ROADMAP.md) for the
milestone plan and [CHANGELOG.md](CHANGELOG.md) for what has landed.

## Build and run

You need a stable Rust toolchain (installed automatically by `rustup` from
`rust-toolchain.toml`).

```sh
cargo build --release        # binary at target/release/vyvrael
cargo test --workspace --release
```

Running the binary with no arguments prints the engine name and version to
stderr and exits; the UCI loop arrives in milestone 5. Two subcommands exist
so far, `perft` and `eval`:

```sh
cargo run --release -- perft 5            # count positions 5 moves deep
cargo run --release -- perft 3 --divide   # the same, split by first move
cargo run --release -- perft 5 --copy-make  # same count, copy-make style
cargo run --release -- eval               # static evaluation, with its parts
cargo run --release -- help
```

`perft` takes an optional FEN after the depth, and `eval` takes an optional
FEN. See
[docs/TESTING.md](docs/TESTING.md) for what the numbers mean.

## Using it in a GUI or lichess-bot

Not possible yet: this needs the UCI interface (milestone 5). Once that
lands:

- **GUI** (Cute Chess, Arena, En Croissant, Banksia, ...): add
  `target/release/vyvrael` as a UCI engine.
- **lichess-bot:** set `engine.name` to the binary name and
  `engine.protocol` to `uci` in `config.yml`.

This section will be replaced with tested instructions when UCI works.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): design and module layout
- [docs/UCI.md](docs/UCI.md): supported commands and options
- [docs/TESTING.md](docs/TESTING.md): tests, perft, strength testing
- [docs/ROADMAP.md](docs/ROADMAP.md): milestones, status and known issues
- [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md): dependencies and licences
- [docs/DATA.md](docs/DATA.md): datasets, books, tablebases, networks
- [docs/NNUE.md](docs/NNUE.md): neural evaluation
- [AGENTS.md](AGENTS.md): instructions for AI coding agents

## Licence

Copyright (C) 2026 Nexxus Drako Multimedia.

Vyvræl is free software, licensed under the GNU General Public License,
version 3 or (at your option) any later version (GPL-3.0-or-later). See
[LICENSE.md](LICENSE.md). It comes with no warranty.
