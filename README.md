# Vyvrael-Chess
A dragon-flavoured UCI chess engine in Rust, built with AI and playing for the attack.

## What it is

Vyvræl is an Australian chess engine by Nexxus Drako Multimedia. It speaks the
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

**Pre-alpha: milestone 1 (board representation) is complete and merged; milestone 2 (move generation) is complete and merged; milestone 3 (hand-crafted evaluation) is complete and merged; milestone 4 (search core) is complete and merged; milestone 5 (UCI) is complete and merged.** The engine plays chess: it speaks UCI, so it can be used in a
GUI and in match tools. It is still weak and slow, because the search has no
move ordering or transposition table yet (milestone 7), and its strength has
not been measured (milestone 6). See [docs/ROADMAP.md](docs/ROADMAP.md) for the
milestone plan and [CHANGELOG.md](CHANGELOG.md) for what has landed.

## Build and run

You need a stable Rust toolchain (installed automatically by `rustup` from
`rust-toolchain.toml`).

```sh
cargo build --release        # binary at target/release/vyvrael
cargo test --workspace --release
```

Run with no arguments, the binary speaks UCI on standard input and output
(see [docs/UCI.md](docs/UCI.md)). It also has three subcommands, `perft`,
`eval` and `search`:

```sh
cargo run --release -- perft 5            # count positions 5 moves deep
cargo run --release -- perft 3 --divide   # the same, split by first move
cargo run --release -- perft 5 --copy-make  # same count, copy-make style
cargo run --release -- eval               # static evaluation, with its parts
cargo run --release -- search 4           # search 4 plies and name the best move
cargo run --release -- help
```

`perft` and `search` take an optional FEN after the depth, and `eval` takes an
optional FEN. See
[docs/TESTING.md](docs/TESTING.md) for what the numbers mean.

## Using it in a GUI or lichess-bot

Vyvræl speaks UCI when started without arguments. Every command and option
is in [docs/UCI.md](docs/UCI.md).

- **GUI** (Cute Chess, Arena, En Croissant, Banksia, ...): add
  `target/release/vyvrael` as a UCI engine.
- **lichess-bot:** set `engine.name` to the binary name and
  `engine.protocol` to `uci` in `config.yml`.

The protocol is tested by driving the binary over pipes (`tests/uci.rs`)
and by complete games under `cutechess-cli` (see
[docs/TESTING.md](docs/TESTING.md)). It has **not yet been tried in a GUI
or with lichess-bot**, so these instructions are unconfirmed.

The plan for Lichess is a bot account, `Vyvrael-Chess`, that plays **casual
games only** once the first alpha is released. Rated games wait until the
engine's strength has been measured and its search heuristics tested (after
0.2.0). The policy, the maintainer's checklist and the rules the bot keeps
are in [docs/LICHESS.md](docs/LICHESS.md).

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): design and module layout
- [docs/UCI.md](docs/UCI.md): supported commands and options
- [docs/TESTING.md](docs/TESTING.md): tests, perft, strength testing
- [docs/ROADMAP.md](docs/ROADMAP.md): milestones, status and known issues
- [docs/LICHESS.md](docs/LICHESS.md): the Lichess bot, its policy and rules
- [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md): dependencies and licences
- [docs/DATA.md](docs/DATA.md): datasets, books, tablebases, networks
- [docs/NNUE.md](docs/NNUE.md): neural evaluation
- [AGENTS.md](AGENTS.md): instructions for AI coding agents

## Licence

Copyright (C) 2026 Nexxus Drako Multimedia.

Vyvræl is free software, licensed under the GNU General Public License,
version 3 or (at your option) any later version (GPL-3.0-or-later). See
[LICENSE.md](LICENSE.md). It comes with no warranty.

The brand assets in [`branding/`](branding/README.md) are the exception: they
are all rights reserved and are not covered by the GPL.
