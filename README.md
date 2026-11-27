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

**Version 0.1.0: milestones 1 to 6 (board representation, move
generation, hand-crafted evaluation, search core, UCI, strength testing) and
queued task 1 (the options `Seed` and `Variety`, and `bench`) and the
preparation for milestone 7 (search statistics, a test-suite runner, more
fuzzing, small UCI additions) are complete and
merged. Milestone 6 measured the engine on the project's ladder and
calibrated `UCI_Elo`, provisionally.** The engine plays chess: it
speaks UCI, so it can be used in a GUI and in match tools. It is still weak
and slow, because the main search orders only its captures and has no
transposition table yet (milestone 7, in progress). Its strength on the project's own opponent ladder, an
ordering rather than an Elo, is in
[docs/testing/results-0.1.0.md](docs/testing/results-0.1.0.md)
(measured for the release; the milestone 6 measurement is beside it). See [docs/ROADMAP.md](docs/ROADMAP.md) for the milestone plan
and [CHANGELOG.md](CHANGELOG.md) for what has landed.

## Build and run

You need the Rust toolchain that `rust-toolchain.toml` pins (a stable
release; `rustup` installs it automatically on first use).

```sh
cargo build --release        # binary at target/release/vyvrael
cargo test --workspace --release
```

Run with no arguments, the binary speaks UCI on standard input and output
(see [docs/UCI.md](docs/UCI.md)). It also has five subcommands, `perft`,
`eval`, `search`, `bench` and `suite`:

```sh
cargo run --release -- perft 5            # count positions 5 moves deep
cargo run --release -- perft 3 --divide   # the same, split by first move
cargo run --release -- perft 5 --copy-make  # same count, copy-make style
cargo run --release -- eval               # static evaluation, with its parts
cargo run --release -- search 4           # search 4 plies and name the best move
cargo run --release -- bench              # a fixed search workload: nodes and speed
cargo run --release -- suite wac.epd --depth 4  # run an EPD test suite (a file of your own)
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

Options of its own, besides `Hash`, `Threads`, `UCI_LimitStrength`,
`UCI_Elo`, `UCI_ShowWDL` and `Move Overhead`:

- **`SearchStats`** (off by default): a diagnostic that sends a search's
  statistics as `info string` lines; see [docs/UCI.md](docs/UCI.md).
- **`Seed`** (0 to 2147483647, default 0): the seed of the engine's
  strength-affecting randomness. 0 takes a new one from system entropy for
  every game and logs it; any other number fixes it, to replay a game.
- **`Variety`** (0 to 100 centipawns, default 0): in the first twelve
  moves, play now and then a move checked to be within this many
  centipawns of the best, so that games do not repeat. 0 always plays the
  best move, as matches and tests need.

Play is deterministic with one thread, a fixed `Seed`, `Variety` 0 and a
fixed depth or node count.

The protocol is tested by driving the binary over pipes (`tests/uci.rs`)
and by complete games under `cutechess-cli` (see
[docs/TESTING.md](docs/TESTING.md)). 0.1.0-alpha.1 has also played in the
Cute Chess GUI, and the releases run under lichess-bot, both set up by the
maintainer.

On Lichess the bot account `Vyvrael-Chess` plays **casual games only**: the
maintainer upgraded it to a bot on 2026-11-20 (UTC) and runs it with
0.1.0 (since 2026-11-23; the alphas before it). Rated games wait until the
engine's strength has been measured and its search heuristics tested
(after 0.2.0). The policy, the maintainer's checklist and the rules the bot
keeps are in [docs/LICHESS.md](docs/LICHESS.md), and what it has played with
each build in [docs/lichess-history.md](docs/lichess-history.md).

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): design and module layout
- [docs/UCI.md](docs/UCI.md): supported commands and options
- [docs/TESTING.md](docs/TESTING.md): tests, perft, strength testing
- [docs/testing/](docs/testing/): measured results, ladder baselines
- [docs/ROADMAP.md](docs/ROADMAP.md): milestones, status and known issues
- [docs/LICHESS.md](docs/LICHESS.md): the Lichess bot, its policy and rules
- [docs/lichess-history.md](docs/lichess-history.md): what the bot played
  with each build
- [docs/RELEASING.md](docs/RELEASING.md): how a release is made
- [docs/releases/](docs/releases/): release notes
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
