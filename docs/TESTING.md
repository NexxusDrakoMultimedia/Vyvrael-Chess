# Testing

Vyvræl is a 0rwx level X project, so tests are the main source of trust.
This document explains what is tested, how to run it and how to read the
results. It is updated in the same commit as any new test, suite or script.

## The three gates

Every commit must pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
```

CI (`.github/workflows/ci.yml`) runs exactly these.

## Unit tests

Unit tests live next to the code in `#[cfg(test)]` modules.

| Area | What is checked |
| --- | --- |
| crate root | The ASCII fallback name is ASCII; the version constant is three dot-separated numbers and is the version written in `Cargo.toml`. |
| `rng` | Golden values for seeds `1234567` (published SplitMix64 vector), `0` and `DEFAULT_SEED`; same seed gives the same sequence; different seeds differ; `next_u32` is the high half of `next_u64`; usable in `const` context; golden values for derived streams; 1000 streams and the base generator all start differently; `next_below` golden values, range, rough uniformity and the zero-bound panic. |
| `board::Color` | Indices match `Color::ALL`; `flip` is an involution; FEN character round-trip. |
| `board::Square` | Corner indices; each of the 64 named constants has the index, display name, parse result and file/rank its name implies; file/rank round-trip; `from_index` bounds; `offset` stops at edges without wrapping; malformed coordinates are rejected. |
| `board::PieceKind`, `board::Piece` | Indices match `PieceKind::ALL`; letter round-trips in both cases; all twelve FEN piece characters round-trip; non-piece characters are rejected. |
| `board::Bitboard` | Bit position equals square index; set/clear/toggle/with/without; set operators; `more_than_one`; `lsb`/`msb`/`pop_lsb` including the empty set; ascending iteration with exact length; `Debug` diagram orientation; file and rank masks against `Square::file`/`rank`; all eight shifts against `Square::offset` for every square (no wrap-around). |
| `board::CastlingRights` | Flags are distinct bits; with/without/contains; canonical FEN display; all 16 values round-trip; any letter order parses; malformed fields are rejected; `revoked_by` for all 64 squares. |
| `board::Move` | Size is 2 bytes; all 64 x 64 x 14 (from, to, kind) encodings round-trip; kinds encode distinctly; capture/promotion/castle predicates; UCI display; promoting to a pawn or king panics. |
| `board::Position` (placement) | Empty-position defaults; `put_piece` updates the mailbox and all bitboards; `remove_piece` restores the empty position exactly; misuse panics; `is_consistent` detects five kinds of corrupted placement; `Debug` diagram. |
| `board` FEN parsing | Start position bitboards and state; every field parsed; four-field (EPD) input; extra whitespace; each `FenError` syntax variant (field count, rank count, rank length, consecutive digits, bad piece character, bad side/castling/en passant/counters); counters accept the full 16-bit range. |
| `board` FEN validation | Wrong king counts; pawns on back ranks; castling rights without king or rook at home (nine cases); partial rights accepted; plausible en passant squares accepted and nine impossible ones rejected. |
| `board` FEN round-trip | 14 canonical FENs (start position, the standard perft reference positions, en passant for both sides, castling-right subsets, extreme counters) parse and serialise back byte for byte, and re-parse to an equal position; non-canonical input serialises canonically. |
| `board::zobrist` | The 781 base keys equal the Zobrist stream in the documented order; all are distinct and non-zero; ten golden key values from an independent implementation; castling keys compose by XOR and all 16 rights values hash differently. |
| `board::Position` (hash) | Golden hashes for the start position and after 1.e4 (independent implementation); incremental updates on placement equal a from-scratch recompute; side, castling and en passant each change the hash by exactly their key; counters are not hashed; en passant is hashed only when a pawn can capture (both colours, board edge, own pawns); editing pawns beside an en passant square keeps the hash right; parsed positions carry a correct hash. |
| `board` make/unmake | Each case plays a move, compares every field (including the hash) with a freshly parsed expected FEN, then unmakes and compares with the original. Covered so far: quiet piece and pawn moves, clock handling, captures, castling rights lost by king moves, rook moves and rook captures, clearing of the en passant square (hashed and unhashed), halfmove clock saturation, double pushes setting and replacing the en passant square (hashed when capturable), en passant captures by both colours in both directions and on the edge files, all four castling moves (rook placement, both rights spent, the other side's rights kept), promotions and capturing promotions to all four pieces for both colours (including capturing a rook on its home square), and an eight-move sequence unwound through every intermediate position. |
| `board` move parsing | Kind detection for pushes, double pushes, captures, en passant (needs the en passant square), castling (king from its home square only), promotions and capturing promotions; display round-trips; malformed text and moves that do not fit the position are rejected; the documented lack of geometry checking is pinned by a test. |

Rows are added as modules land.

During the milestone 1 review, make/unmake was also stress-tested outside the
repository against an independent mailbox model over about 5.6 million
pseudo-legal moves, and the FEN parser was fuzzed with a million mutated
inputs; neither found a defect. Those harnesses are not committed, because
they need a move generator: milestone 2's perft suite takes over that role.

## Integration tests

Integration tests live in `tests/` and use only the public API.

| File | What is checked |
| --- | --- |
| `tests/board_games.rs` | Two complete games played from the start position in UCI notation: the Opera Game (Morphy, Paris 1858; 33 plies, queenside castling) and a constructed 20-ply game covering en passant, capturing promotions to queen and knight, a rook captured on its home square and kingside castling by both sides. After every ply the position must be consistent, its hash must equal a from-scratch recompute, and its FEN must parse back to an identical position. Seven checkpoint FENs, including both final positions, are compared exactly. Then every move is unmade and each earlier position must be restored exactly. |

The Opera Game's final FEN is the widely published one, so that checkpoint is
independent of this code. The other checkpoint FENs were derived by hand.

## Perft (planned, milestone 2)

Perft ("performance test") counts the leaf nodes of the legal move tree to a
fixed depth from a given position. Because the correct counts for standard
positions are published, any disagreement proves a move generation bug, and
the *divide* mode (node count per root move) lets you bisect to the faulty
move by comparing against a reference engine.

Milestone 2 will add a perft integration test over the standard reference
positions (start position, Kiwipete and the other well-known positions that
stress castling, en passant, promotion and pins) and a `perft` subcommand
with divide mode. The positions and their expected node counts will be listed
here, with their source, when the tests land.

## Strength matches and SPRT (planned, milestone 6)

Search, evaluation and network changes are accepted only when a sequential
probability ratio test (SPRT) against the previous version shows a gain. A
match script using fastchess or Cutechess will be added in milestone 6 and
documented here, including how to read LLR, Elo estimates and error bars.
Until then, search changes need at least a fixed-depth node-count and
correctness check.

SPRT results are recorded in the commit message of the change or in a table
in this file.

## Tactical test suites (planned, milestone 7)

A tactical suite derived from the Lichess puzzle database will report the
share of puzzles solved at a fixed depth or time. Details will be added with
the tooling.

## Reproducible runs

- All non-strength-affecting randomness (Zobrist keys, magic numbers, test
  fixtures) is generated from the fixed seed `0x31031996` and is identical on
  every run and platform.
- Strength-affecting randomness is seeded from system entropy by default. For
  a reproducible run set the UCI options `DeterministicRNG = true` and, if
  wanted, `RngSeed` (planned, milestone 5). When `DeterministicRNG` is
  `false`, the engine logs the seed it chose with `info string`, so a run can
  be reproduced afterwards.
- **Multithreaded search is not bit-for-bit deterministic**, whatever the
  seed: thread scheduling changes the order in which shared tables are
  written. For reproducible results use **one thread, a fixed seed, and a
  fixed depth or node count** (never a time limit). Regression tests and
  performance comparisons use these settings.

## Reading results

- `cargo test` prints one `test result:` line per test binary; any `FAILED`
  line is a regression.
- A failing test is fixed by fixing the code. Tests are never weakened,
  skipped or deleted to get a green run.
