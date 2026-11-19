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
| `attacks` knight | Table equals the eight-step definition built with `Square::offset` for all 64 squares; counts for corner, edge and centre squares and the total of 336; symmetry. |
| `attacks` king | Table equals the eight-neighbour definition for all 64 squares; counts for corner, edge and centre squares and the total of 420. |
| `attacks` pawn | Both colours equal the two-step definition for all 64 squares; edge files, last ranks and the total of 98 per colour; the reverse-lookup identity for every pair of squares. |
| `attacks` scan | Rook on an empty board equals rank plus file for all 64 squares; bishop counts and diagonal property; both stop at and include the first piece on each ray; a piece on the slider's own square changes nothing; a full board leaves only neighbours. |
| `attacks::magic` masks | Mask sizes for sample squares and the classic table totals (102,400 rook and 5,248 bishop entries); the rook mask of a1 square by square; for every square and every arrangement inside the mask, filling all squares outside it does not change the scanned attacks; `subsets` yields each subset exactly once. |
| `attacks::sliders` | Exhaustive: for all 64 squares and every relevant blocker arrangement (107,648 cases) the bishop and rook lookups equal the scan; 20,000 random full-board occupancies including queens; table layout and sizes; rebuilding the tables reproduces the same magic numbers and table; four pinned magic numbers and a fold of all 128 (recorded from this implementation as regression pins). |
| `board` attack queries | Attackers of sample squares in the start position; check given by each piece kind, including pawns of both colours from both sides; positions that are not check (blocked sliders, pawns behind or straight ahead of the king); double check; only the side to move is asked about; a custom occupancy sees through a removed blocker; adjacent kings. |
| `movegen::MoveList` | Starts empty; push keeps order; slice methods work through `Deref`; equality ignores unused slots; holds exactly 256 moves and panics on the 257th. |
| `movegen` pseudo-legal | Expected move lists as sorted UCI strings with captures marked. Knights: start position for both colours, captures and own-piece blocking, a corner. King steps: both colours, captures, own-piece blocking, the centre, and that attacked squares are still generated at this stage. Sliders: bishop, rook (both colours) and queen stop at and capture the first enemy, stop before own pieces, and have no moves in the start position. Pawn pushes: 16 moves for each colour at the start, pushes blocked at the first or second square, double pushes only from the starting rank. Pawn captures: both diagonals for both colours; no capture of own pieces, straight ahead, or around the board edge. Promotions: four pieces for pushes and for captures, both colours; a blocked pawn cannot promote; moves onto the last rank are always promotions. En passant: both colours, two capturing pawns, none without the square, none without a capturer, no wrap around the edge. Castling: both ways for both colours; needs the right; needs empty squares between king and rook (own or enemy pieces, including b1); not out of check; not through or into an attacked square; an attacked b-file square is allowed. |
| `movegen::legal_moves` | The published depth-1 perft counts of the six reference positions (20, 48, 14, 6, 44, 46); pinned pieces (including moving along the pin line); answering check by capture, block or king move; the king may not step onto attacked squares or next to the other king; checkmate and stalemate give no moves; en passant that exposes the king along the rank is illegal; the position is left untouched. |
| `perft` | Start position depths 0 to 3 (1, 20, 400, 8,902); depth 1 equals the number of legal moves; `divide` sums to `perft`, lists each legal first move once and gives 600 for `e2e4` and 380 for `a2a3` at depth 3; depth 0 and checkmate edge cases; the position is restored. |
| `cli` | No arguments writes only to stderr; `help` prints the usage text; unknown commands exit with status 2; `perft` from the start position and from a quoted or unquoted FEN; `--divide` output format, order and total; bad depths and FENs are usage errors. |

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
| `tests/perft.rs` | Perft acceptance tests; see "Perft" below for the positions and counts. |
| `tests/board_games.rs` | Two complete games played from the start position in UCI notation: the Opera Game (Morphy, Paris 1858; 33 plies, queenside castling) and a constructed 20-ply game covering en passant, capturing promotions to queen and knight, a rook captured on its home square and kingside castling by both sides. After every ply the position must be consistent, its hash must equal a from-scratch recompute, and its FEN must parse back to an identical position. Seven checkpoint FENs, including both final positions, are compared exactly. Then every move is unmade and each earlier position must be restored exactly. |

The Opera Game's final FEN is the widely published one, so that checkpoint is
independent of this code. The other checkpoint FENs were derived by hand.

## Perft

Perft ("performance test") counts the leaf nodes of the legal move tree to a
fixed depth from a given position. Because the correct counts for standard
positions are published, any disagreement proves a move generation bug, and
the *divide* mode (node count per root move) lets you bisect to the faulty
move by comparing against a reference engine.

### Running perft by hand

```sh
cargo run --release -- perft <depth> [--divide] [FEN]
```

The FEN is optional (the start position is used without it) and may be given
quoted or unquoted. The node count goes to stdout as `nodes: N`; the time and
speed go to stderr. With `--divide`, each legal first move is listed as
`move: count` in alphabetical order before the total, which is the format
other engines print, so the two lists can be compared line by line.

To find a move generation bug: run `--divide` here and in a trusted engine,
pick a move whose count differs, play it in both, and repeat at one depth
less until a position shows a move that one side has and the other lacks.

### Reference positions

The integration tests in `tests/perft.rs` check **every depth from 1 to the
one shown** against the published value, and that perft leaves the position
unchanged.

| Position | FEN | Deepest depth tested | Nodes at that depth | What it stresses |
| --- | --- | --- | --- | --- |
| 1: start position | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 5 | 4,865,609 | Double pushes, the first en passant captures (depth 5) and the first checks. |
| 2: Kiwipete | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1` | 4 | 4,085,603 | Castling in all four directions, castling rights lost by captures, pins, en passant, promotions. |

The expected counts are the published reference values from the Chess
Programming Wiki ("Perft Results",
<https://www.chessprogramming.org/Perft_Results>), which engines agree on.
They were not produced by this engine.

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
