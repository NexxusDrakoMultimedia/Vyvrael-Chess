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

CI (`.github/workflows/ci.yml`) runs exactly these, and then the offline
tests of the tool installer (see "Installing the external tools" below).

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
| `board` FEN validation | Wrong king counts; pawns on back ranks; impossible material (nine pawns, extra pieces with all eight pawns, 26 queens) rejected and promotion-reachable material accepted; castling rights without king or rook at home (nine cases); partial rights accepted; plausible en passant squares accepted and nine impossible ones rejected; seven positions with the side not to move in check rejected, while check, checkmate and double check against the side to move are accepted. |
| `board` FEN round-trip | 14 canonical FENs (start position, the standard perft reference positions, en passant for both sides, castling-right subsets, extreme counters) parse and serialise back byte for byte, and re-parse to an equal position; non-canonical input serialises canonically. |
| `board::zobrist` | The 781 base keys equal the Zobrist stream in the documented order; all are distinct and non-zero; ten golden key values from an independent implementation; castling keys compose by XOR and all 16 rights values hash differently. |
| `board::Position` (hash) | Golden hashes for the start position and after 1.e4 (independent implementation); incremental updates on placement equal a from-scratch recompute; side, castling and en passant each change the hash by exactly their key; counters are not hashed; en passant is hashed only when a pawn can capture (both colours, board edge, own pawns) and only when that capture is legal (five illegal cases: rank exposure, file and diagonal pins, unanswered check, Black; four awkward legal cases); editing pawns beside an en passant square, or a pinning or shielding piece elsewhere, keeps the hash right; parsed positions carry a correct hash. |
| `board` make/unmake | Each case plays a move, compares every field (including the hash) with a freshly parsed expected FEN, then unmakes and compares with the original. Covered so far: quiet piece and pawn moves, clock handling, captures, castling rights lost by king moves, rook moves and rook captures, clearing of the en passant square (hashed and unhashed), halfmove clock saturation, double pushes setting and replacing the en passant square (hashed when capturable), en passant captures by both colours in both directions and on the edge files, all four castling moves (rook placement, both rights spent, the other side's rights kept), promotions and capturing promotions to all four pieces for both colours (including capturing a rook on its home square), and an eight-move sequence unwound through every intermediate position. |
| `board` move parsing | Kind detection for pushes, double pushes, captures, en passant (needs the en passant square), castling (king from its home square only), promotions and capturing promotions; display round-trips; malformed text and moves that do not fit the position are rejected; the documented lack of geometry checking is pinned by a test. |
| `attacks` knight | Table equals the eight-step definition built with `Square::offset` for all 64 squares; counts for corner, edge and centre squares and the total of 336; symmetry. |
| `attacks` king | Table equals the eight-neighbour definition for all 64 squares; counts for corner, edge and centre squares and the total of 420. |
| `attacks` pawn | Both colours equal the two-step definition for all 64 squares; edge files, last ranks and the total of 98 per colour; the reverse-lookup identity for every pair of squares. |
| `attacks` scan | Rook on an empty board equals rank plus file for all 64 squares; bishop counts and diagonal property; both stop at and include the first piece on each ray; a piece on the slider's own square changes nothing; a full board leaves only neighbours. |
| `attacks::magic` masks | Mask sizes for sample squares and the classic table totals (102,400 rook and 5,248 bishop entries); the rook mask of a1 square by square; for every square and every arrangement inside the mask, filling all squares outside it does not change the scanned attacks; `subsets` yields each subset exactly once. |
| `attacks::sliders` | Exhaustive: for all 64 squares and every relevant blocker arrangement (107,648 cases) the bishop and rook lookups equal the scan; 20,000 random full-board occupancies including queens; table layout and sizes; the seeded search, run twice, finds exactly the 128 embedded magic numbers and fills exactly the engine's table; a number that is not magic is refused when a slice is filled; four pinned magic numbers and a fold of all 128 (recorded from this implementation as regression pins). |
| `board` attack queries | Attackers of sample squares in the start position; check given by each piece kind, including pawns of both colours from both sides; positions that are not check (blocked sliders, pawns behind or straight ahead of the king); double check; only the side to move is asked about; a custom occupancy sees through a removed blocker; adjacent kings. |
| `movegen::MoveList` | Starts empty; push keeps order; slice methods work through `Deref`; equality ignores unused slots; holds exactly 256 moves and panics on the 257th. |
| `movegen` pseudo-legal | Expected move lists as sorted UCI strings with captures marked. Knights: start position for both colours, captures and own-piece blocking, a corner. King steps: both colours, captures, own-piece blocking, the centre, and that attacked squares are still generated at this stage. Sliders: bishop, rook (both colours) and queen stop at and capture the first enemy, stop before own pieces, and have no moves in the start position. Pawn pushes: 16 moves for each colour at the start, pushes blocked at the first or second square, double pushes only from the starting rank. Pawn captures: both diagonals for both colours; no capture of own pieces or straight ahead; no capture around the board edge, with the bait placed on the exact square a wrapping shift would reach, for both edge files and both colours. Promotions: four pieces for pushes and for captures, both colours; a blocked pawn cannot promote; moves onto the last rank are always promotions. En passant: both colours, two capturing pawns, none without the square, none without a capturer, no wrap around the edge for either colour. Castling: both ways for both colours; needs the right; needs empty squares between king and rook (own or enemy pieces, including b1); not out of check; not through or into an attacked square, with the crossed and destination squares each tested for both colours; an attacked b-file square is allowed. |
| `movegen::legal_moves` | The published depth-1 perft counts of the six reference positions (20, 48, 14, 6, 44, 46); pinned pieces (including moving along the pin line); answering check by capture, block or king move; the king may not step onto attacked squares or next to the other king; checkmate and stalemate give no moves; en passant that exposes the king along the rank is illegal; the position is left untouched; legal moves keep the pseudo-legal order. |
| `perft` | Start position depths 0 to 3 (1, 20, 400, 8,902); depth 1 equals the number of legal moves; `divide` sums to `perft`, lists each legal first move once and gives 600 for `e2e4` and 380 for `a2a3` at depth 3; depth 0 and checkmate edge cases; the position is restored. |
| `cli` `search` | One line per depth and then the best move, identical on repeated runs; the exact first line from the start position; mates reported in moves with the right sign, stopping at the depth that proves them; checkmate and stalemate at the root; bad depths and FENs are usage errors. |
| `cli` `eval` | The exact output for the start position; both points of view after 1.e4 (+40 for White, -40 for the side to move); quoted and unquoted FENs; material and phase lines for a lone queen; a bad FEN is a usage error. |
| `cli` | No arguments writes only to stderr; `help` prints the usage text; unknown commands exit with status 2; `perft` from the start position and from a quoted or unquoted FEN; `--divide` output format, order and total; bad depths and FENs are usage errors. |
| `perft` copy-make | `perft_copy_make` gives the same counts as `perft` at every depth for three positions; the `--copy-make` flag works and cannot be combined with `--divide`. |
| `eval` interface | `relative_to` flips the sign for Black; an evaluator works as a `Box<dyn Evaluator>` and reports from the side to move's view; an evaluator can be moved to another thread and shared by four threads at once. |
| `eval::Material` | Piece values keep the usual orderings (bishop above knight, two rooks above a queen, rook above knight plus pawn); the start position is balanced at full material; a missing queen scores +900 or -900 depending on the side to move; rook against knight and pawn, bishop against knight, bare kings. |
| `eval` game phase | The start position is 24; kings and pawns are 0; each piece kind adds its weight for both colours; nine queens are capped at 24. |
| `eval::Tapered` | `blend` equals `mg` at phase 24 and `eg` at phase 0, is a weighted average in between and monotonic; a flat pair blends to itself; `blend(-x) == -blend(x)` at every phase, including fractional averages; component-wise arithmetic; a phase outside 0 to 24 panics. |
| `eval` piece-square tables | Visual tables map to the right squares; Black uses each table flipped top to bottom; every table is left-right symmetric; the pawn table is zero on the back ranks, accelerates towards promotion in the endgame and prefers advanced centre pawns in the middlegame; the balance is White minus Black, zero at the start, and negates when colours are swapped and the board flipped; knights and bishops prefer the centre to the rim and development to the back rank; rooks prefer the seventh rank and central files; the queen's middlegame values are small and her endgame values favour the centre; the king scores best on g1 and b1 in the middlegame, both castled squares beat the home square, every step up the board is worse, and the centre is best in the endgame; every piece kind has a table. |
| `eval::HandCrafted` | Colour symmetry on eight positions (a position and its colour-swapped mirror score the same for the side to move, and every part of the breakdown negates); the side to move only changes the sign; the start position is exactly level; the breakdown after 1.e4 adds up by hand; material outweighs placement; the total equals material plus the blend at the position's own phase for twelve positions covering seven different phases; one advanced king scored exactly at three phases (-7 with queens and rooks on, +18 with queens only, +44 with bare kings); `default_evaluator` is the hand-crafted one. |
| `search` scores | The ranges nest (evaluations below `MATE_BOUND`, mates below `INFINITY`); `mated_in` and `mate_in` mirror each other; quicker mates and slower defeats score higher; ordinary scores are not mate scores; `moves_to_mate` counts full moves with the right sign. |
| `search` negamax | Alpha-beta returns exactly the score of an unpruned minimax on nine positions (to depth 3 on quiet ones, depth 2 on busy middlegames, because unpruned minimax with a quiescence search at every leaf is very expensive there), visits no more nodes, restores the position, and names a move that leads to that score; pruning more than halves the work at depth 4 in a rook endgame; a hanging queen is taken; mate in one and mate in two are found at the right depth with the right score, and not one ply early; the losing side sees mate coming; checkmate and stalemate at the root; the three stalemating moves of a queen ending are avoided at depths 1 to 3 and no mate is reported before it is there; node counting; depth 0 is rejected. |
| `search` draws | Bare kings and single minor pieces are insufficient, anything that might mate is not; knights going out and back repeat the start position; the same placement with the other side to move does not; with a made-up history, a matching key an odd number of plies back or beyond the halfmove clock's reach does not count; empty and short histories are safe; the fifty-move threshold is 100 plies. In search: the losing side takes a repetition from the game history that it cannot see without it; a perpetual check with no game history is a draw by the hashes the search records on its own way down, both when the line returns to the root and when it returns to a position one ply below it, and those hashes are all taken back afterwards; the fifty-move rule ends a line that had a mate, except when the mate lands on the hundredth ply; bare material is a draw inside the tree, at the first node of every line (checked by node count). |
| `search` principal variation | On five positions (to depth 3 or 4 on quiet ones, depth 2 on busy middlegames) the line starts with the best move, consists of legal moves, runs the full depth, and ends in a position whose quiescence value (seen from the root side) equals the reported score; the line of a mate in two is three plies and ends in checkmate; no line when the root has no legal move. |
| `search` limits | `Limits` constructors and combinations. A node limit stops the search exactly at the limit and leaves the repetition history as it was; a search cut short at three different limits answers only from the root moves it finished, never from the one it was working on; a generous limit changes nothing; a raised stop flag ends the search before the first node while an unraised one does not disturb it; a deadline already passed ends the search within one check interval. |
| `search` iterative deepening | Iterating to depth 3 equals a direct depth-3 search on three positions, with one report per depth that matches a direct search of that depth and with nodes and time increasing; iteration stops at the depth where a mate is proven (3 for mate in two, 2 for being mated in one) and not before: a mate in two already seen at depth 1 through quiescence is searched on to depth 3; it ends after one iteration when the root is checkmate or stalemate; a node limit that interrupts an iteration returns the last completed depth; the first iteration completes even with a one-node limit, a raised stop flag or a passed deadline, and a limit reached between iterations does not count as an interruption; a depth limit of 0 still searches one ply; a `Search` stopped by a limit can be used again and starts afresh. |
| `search` quiescence | A quiet position is worth its static evaluation; a winning capture is played out; a queen does not grab a defended pawn at depths 1 to 3 (the horizon effect); no standing pat in check (checkmate is scored by ply, a forced recapture is followed); queen promotions are considered; it terminates on a position full of captures in under a million nodes and restores it; bare kings after a capture are a draw; which moves count as tactical. Capture ordering: most valuable victim first, least valuable attacker breaking ties, promotions and en passant in their place, and the ordering only reorders (same moves, captures before quiet moves, generation order kept when nothing can be captured). The minimax-agreement and principal-variation tests now value their leaves with a full-window quiescence search. Stalemate at the horizon is a draw whatever the window, and a one-ply search does not walk into one; the two shortcuts that rule stalemate out without playing moves agree with full move generation on every position not in check of five trees (over 5,000 positions, with pins, en passant and stalemates), and a position whose only movable piece is pinned is recognised as stalemate. In check there is no standing pat (a skewered queen is not counted as material kept). A dead position is valued by its evaluation on its own, at ply 0, and as a draw inside a line. |
| `movegen` move input | `find_legal_move` returns every legal move of five positions from its text and agrees with the trusting parser on them; it refuses moves no piece can make (which the trusting parser accepts), the other side's moves, pinned pieces, moves that leave the king in check, promotions without or with a wrong letter, castling through an attacked square, and a dozen strings that are not moves. |
| `uci` command parsing | Each command is recognised; blank lines, extra white space, a trailing carriage return and unknown words before a command are forgiven; unknown lines are reported as such and commands are case-sensitive; `position` from the start or a FEN, with and without moves, and six malformed forms; `go` with clocks (a negative clock counts as zero), with depth, nodes, move time and `infinite`, ignoring `ponder`, `searchmoves` and `mate` and skipping parameters without a usable value; `setoption` with names and values containing spaces, without a value, with an empty value, and four malformed forms. |
| `uci` output | Scores as `cp` or as `mate` in the engine's own moves, negative when it is the one mated; `info` lines with depth, score, nodes, speed, time and line, without a `pv` word when there is no line, with no division by zero for an instant search, and with the speed taken from the full time; `bestmove` with a move and `0000` without one. |
| `uci` time management | No clock and no move time means no time limit, also when only the other side's clock is sent; `movetime` is used less the overhead and takes precedence over a clock; each side uses its own clock and increment; `movestogo` divides the time, and 1 or 0 allows all that is left less the overhead; the plan never exceeds the clock for 72 combinations of time, increment and moves to go, including a fallen flag and a large increment on an empty clock; limits combine the deadline with depth and nodes; `infinite` switches the clock off. |

Rows are added as modules land.

During the milestone 1 review, make/unmake was also stress-tested outside the
repository against an independent mailbox model over about 5.6 million
pseudo-legal moves, and the FEN parser was fuzzed with a million mutated
inputs; neither found a defect. Those harnesses were not committed. Their
role is now filled by `tests/perft.rs` and `tests/consistency.rs`.

## Looking at an evaluation

```sh
cargo run --release -- eval [FEN]
```

prints the hand-crafted evaluation of a position (the start position without
a FEN) and its parts: material, the piece-square balance for the middlegame
and the endgame, the game phase, and the total from White's point of view and
from the side to move's. It does not search, so it says how the position
looks, not what is about to happen in it. All numbers are centipawns.

## Pinned search results

A fixed-depth search uses no clock and no randomness, so everything it
reports is determined by the code. `tests/search_pins.rs` records the score,
node count and principal variation for seven positions.

These numbers were **recorded from this implementation**; they are not an
independent reference the way perft counts are. What they give is change
detection: an edit to move generation order, to the evaluation or to the
search that alters behaviour at fixed depth fails this test, including edits
that were not meant to alter anything.

This is the "fixed-depth node-count check" that search changes need while
SPRT is not yet available. When a deliberate change makes a pin fail:

1. Confirm the new behaviour is intended. For a search or evaluation change
   that means an SPRT result once the match script exists.
2. Update the numbers in the same commit and say so in its message.

Never update a pin just to get a green run.

| Position | Depth | Score | Nodes |
| --- | --- | --- | --- |
| Start position | 4 | 0 | 28,793 |
| Kiwipete | 2 | +44 | 391,308 |
| Position 3 | 5 | +107 | 63,892 |
| Position 4 | 2 | -418 | 23,562 |
| Position 5 | 2 | +541 | 5,857 |
| Position 6 | 2 | +22 | 27,964 |
| Queen against two pawns | 4 | +683 | 15,714 |

The Kiwipete figure shows the cost of searching without move ordering (known
issue KI-9): two plies there take more nodes than five in the rook endgame.

## Running a search by hand

```sh
cargo run --release -- search <depth> [FEN]
```

searches the position (the start position without a FEN) by iterative
deepening to the given depth. It prints one line per completed depth, in the
vocabulary of UCI `info` lines, and then the best move:

```
depth 1 score cp 40 nodes 21 pv d2d4
depth 2 score cp 0 nodes 243 pv d2d4 d7d5
bestmove d2d4
```

- `score cp N` is in centipawns for the side to move; `score mate N` means
  mate in N moves (negative: the side to move is being mated).
- `nodes` counts positions visited since the search began, quiescence search
  included.
- `pv` is the line of best play the score comes from.
- Time and speed go to stderr, so stdout is identical on every run and can be
  compared between versions.

A fixed-depth search uses no clock and no randomness, so it is fully
reproducible: the same position and depth always give the same lines.

## Integration tests

Integration tests live in `tests/` and use only the public API.

| File | What is checked |
| --- | --- |
| `scripts/tests/test_install_tools.py` | Python, offline. The tool installer's helpers: every tool is pinned to an exact version; downloads must be HTTPS, redirects included; archive extraction refuses paths and symlinks that escape, chained links included; a failed reinstall leaves the old install alone and an interrupted one is recovered; `--prefix` keeps everything under it; launchers and the systemd unit quote paths with spaces; the lichess-bot configuration gets the engine path and an empty token; token files are owner-only. |
| `tests/perft.rs` | Perft acceptance tests; see "Perft" below for the positions and counts. |
| `tests/search_pins.rs` | The score, total node count and principal variation of fixed-depth searches on seven positions (the six perft positions and one queen-against-pawns ending), and that searching twice gives the same result. See "Pinned search results" above. |
| `tests/consistency.rs` | Walks the legal move tree of nine positions (the perft reference positions and two with en passant at the root) three or four plies deep, more than 300,000 positions. At every node: `is_consistent()` (views agree, stored hash equals a recompute); the FEN parses back to an identical position; the side to move cannot capture the king; legal moves are a subset of pseudo-legal moves; each move's notation parses back to the same move; a capture is reported exactly when the move is a capture; the en passant square changes the hash exactly when an en passant capture is legal; unmaking restores the previous position exactly. |
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
cargo run --release -- perft <depth> [--divide | --copy-make] [FEN]
```

The FEN is optional (the start position is used without it) and may be given
quoted or unquoted. The node count goes to stdout as `nodes: N`; the time and
speed go to stderr. With `--divide`, each legal first move is listed as
`move: count` in alphabetical order before the total, which is the format
other engines print, so the two lists can be compared line by line.
`--copy-make` gives the same count by copying the position for each move (see
below); the two flags cannot be combined.

To find a move generation bug: run `--divide` here and in a trusted engine,
pick a move whose count differs, play it in both, and repeat at one depth
less until a position shows a move that one side has and the other lacks.

### Make/unmake versus copy-make

Known issue KI-4 asked for a measurement of the board's in-place make/unmake
against "copy-make" (copy the position, play the move on the copy, discard
it). `perft --copy-make` runs the same count that way.

Measured on 2026-11-20 on the development machine (Windows, x86-64, rustc
1.100.0, release profile, one thread), three runs each, median shown:

| Position | Depth | Nodes | Make/unmake | Copy-make | Difference |
| --- | --- | --- | --- | --- | --- |
| Start position | 6 | 119,060,324 | 4,450 ms (26.8 M nodes/s) | 4,235 ms (28.1 M nodes/s) | copy-make 4.8% faster |
| Kiwipete | 5 | 193,690,690 | 7,683 ms (25.2 M nodes/s) | 7,310 ms (26.5 M nodes/s) | copy-make 4.9% faster |

Both node counts equal the published reference values. To repeat:

```sh
cargo run --release -- perft 6
cargo run --release -- perft 6 --copy-make
```

Reading the result: copy-make is slightly faster in perft, by an amount
(about 5%) that is small and specific to this workload. Perft does nothing
but generate and make moves; search adds evaluation, hashing and ordering,
which dilute the difference, and later per-position state (for example NNUE
accumulators) changes the cost of copying. So the decision was deferred to a measurement
inside the real search, made in milestone 4.

Measured on 2026-11-20 on the same machine, by building the search both ways
and running `search` to a fixed depth, three runs each, median shown. The
node counts are identical, which confirms the two builds perform the same
search:

| Position | Depth | Nodes | Make/unmake | Copy-make | Difference |
| --- | --- | --- | --- | --- | --- |
| Start position | 6 | 3,001,490 | 629 ms | 598 ms | copy-make 4.9% faster |
| Kiwipete | 3 | 4,513,442 | 1,483 ms | 1,442 ms | copy-make 2.8% faster |
| Position 3 | 7 | 2,517,193 | 481 ms | 461 ms | copy-make 4.2% faster |

Copy-make was faster in all nine pairs of runs but one, where the two tied.
The margin is small but consistent, so **the search uses copy-make**. It
also makes the search simpler: there is no unmake step to get wrong, and the
search takes the position by shared reference. The make/unmake build used for
this comparison was the search as committed before the switch; the pinned
node counts in `tests/search_pins.rs` did not change across it.

### Reference positions

The integration tests in `tests/perft.rs` check **every depth from 1 to the
one shown** against the published value, and that perft leaves the position
unchanged.

| Position | FEN | Deepest depth tested | Nodes at that depth | What it stresses |
| --- | --- | --- | --- | --- |
| 1: start position | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 5 | 4,865,609 | Double pushes, the first en passant captures (depth 5) and the first checks. |
| 2: Kiwipete | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1` | 4 | 4,085,603 | Castling in all four directions, castling rights lost by captures, pins, en passant, promotions. |
| 3 | `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1` | 6 | 11,030,083 | En passant that is illegal because it uncovers a rook on the rank, discovered checks, promotions at depth. |
| 4 and its mirror | `r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1` | 5 | 15,833,292 | Starting in check, promotions with and without capture, castling for the side not to move; the mirror proves White and Black are treated alike. |
| 5 | `rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8` | 4 | 2,103,487 | Capturing promotions with check, castling while a knight attacks the castling rook (still legal). |
| 6 | `r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10` | 4 | 3,894,594 | Ordinary piece play at volume: bishop pins, many captures. |

The expected counts are the published reference values from the Chess
Programming Wiki ("Perft Results",
<https://www.chessprogramming.org/Perft_Results>), which engines agree on.
They were not produced by this engine.

## Installing the external tools (Linux)

Strength testing uses Cute Chess and Ordo, and playing on Lichess uses
lichess-bot. They are separate programs that are never part of this
repository. On Linux, `scripts/install_tools.py` installs pinned versions of
them for the current user, outside the repository. It needs only Python 3.9
or newer (lichess-bot itself needs 3.11); building Ordo needs `gcc` and
`make` (`sudo apt install build-essential`).

```sh
python3 scripts/install_tools.py --list       # pinned and installed versions
python3 scripts/install_tools.py --dry-run    # show what would be done
python3 scripts/install_tools.py              # Cute Chess and Ordo
python3 scripts/install_tools.py --tool lichess-bot --engine target/release/vyvrael
```

What to know before running it:

- **It downloads and runs third-party code:** the Cute Chess AppImage, Ordo's
  source (built with `make`), and lichess-bot with its Python packages.
- **Versions are pinned** in the `PINS` table at the top of the script and
  listed in [DEPENDENCIES.md](DEPENDENCIES.md). Cute Chess is checked against
  its published SHA-256. Ordo and lichess-bot are fetched at one exact commit;
  no digest is published for those archives, so the script prints the digest
  it saw. Record it in `PINS` and it is enforced from then on.
- **Where things go:** programs under `~/.local/share/vyvrael-tools`, launchers
  in `~/.local/bin`. With `--prefix DIR` everything, launchers included, stays
  under `DIR`. Re-running is safe; `--force` reinstalls.
- **A re-run skips what is already installed** (recorded at the pinned
  version, commit and digest, with its files still there) but still rewrites
  that tool's launchers, so a new `--bin-dir` or an added `--systemd` takes
  effect without `--force`. `--engine` is only used when `config.yml` is
  first written; after that, edit `config.yml`.
- **What a lichess-bot reinstall keeps:** `config.yml`, the virtual
  environment, and any top-level file or directory of yours that the release
  does not ship. Everything the release ships is replaced by the pinned
  version, so edits to its files and files added inside its directories
  (its `engines/`, for example) are lost. Without `--engine` the generated
  `config.yml` therefore points at `<prefix>/engines/vyvrael`, outside that
  directory; put the engine binary there.
- **`vyvrael-lichess-bot` runs from lichess-bot's own directory**, whatever
  directory you call it from, because lichess-bot reads `./config.yml`. A
  relative path on its command line is therefore read from that directory.
- **Your Lichess token is never written by the script.** `config.yml` is
  created with an empty token and readable only by you. The script does not
  upgrade the account to a bot account (that is irreversible) and does not
  start anything. With `--systemd` it also writes a user service and a token
  file under `~/.config`, without enabling them. The token file holds only a
  commented-out `LICHESS_BOT_TOKEN=` line: lichess-bot lets that variable
  replace the token in `config.yml` whenever it is set, even to nothing, so
  uncomment it only when you put the token there.
- After installing, it runs `cutechess-cli --version` and `ordo -h` once and
  prints the first line, which is the check that the tools work. A tool is
  recorded as installed only when that check could start the program. The
  check fails when the launcher cannot be run, times out, or exits with 126
  or 127 (the shell could not execute or find the program); then the run
  ends with an error, and the next run installs the tool again instead of
  reporting it as already installed. Any other exit status counts as started
  and is printed beside the first line.
- The pinned Cute Chess file is an x86_64 AppImage; on any other machine the
  script stops before downloading it.

Status: the script's helper functions are covered by offline unit tests
(`python3 -m unittest discover -s scripts/tests`, also run in CI), which
download and install nothing. **The installer itself has not yet been run
end to end.** Three details can only be confirmed by a real run: that this
Cute Chess AppImage runs `cutechess-cli` when given a leading `cli` argument,
which exit status `cutechess-cli --version` and `ordo -h` end with (until
that is known the check accepts any status except 126 and 127, so it cannot
yet tell a program that starts and then fails from one that works), and that
lichess-bot reads its token from `LICHESS_BOT_TOKEN`.

On Windows, and for the match and rating scripts themselves, see milestone 6
in [ROADMAP.md](ROADMAP.md): manual install steps are still to be written.

## Strength matches and SPRT (planned, milestone 6)

Search, evaluation and network changes are accepted only when a sequential
probability ratio test (SPRT) against the previous version shows a gain. A
match script using Cute Chess (`cutechess-cli`), with Ordo for rating
lists, will be added in milestone 6 (scope in [ROADMAP.md](ROADMAP.md)) and
documented here, including how to read LLR, Elo estimates and error bars.
Because of that, search is split (see "Decisions for milestones 4 to 6" in
[ROADMAP.md](ROADMAP.md)): the search core in milestone 4 is checked by
correctness tests and fixed-depth node counts, and the search heuristics wait
for milestone 7, after the match script exists, so each has an SPRT result.

SPRT results are recorded in the commit message of the change or in a table
in this file.

## Tactical test suites (planned, milestone 8)

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
