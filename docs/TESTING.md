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
tests of the helper scripts in `scripts/tests/`: the installer, the
ladder's engines, and the match and rating scripts (see "Integration tests"
below).

## Unit tests

Unit tests live next to the code in `#[cfg(test)]` modules.

| Area | What is checked |
| --- | --- |
| crate root | The ASCII fallback name is ASCII; the version constant is three dot-separated numbers, optionally with a pre-release tag such as `-alpha.1` (seven accepted and fourteen refused spellings pin the rule, among them a pre-release number with a leading zero, which Cargo also refuses), and is the version written in `Cargo.toml`. |
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
| `board` make/unmake | Each case plays a move, compares every field (including the hash) with a freshly parsed expected FEN, then unmakes and compares with the original. Covered so far: quiet piece and pawn moves, clock handling, captures, castling rights lost by king moves, rook moves and rook captures, clearing of the en passant square (hashed and unhashed), halfmove clock saturation, double pushes setting and replacing the en passant square (hashed when capturable), en passant captures by both colours in both directions and on the edge files, all four castling moves (rook placement, both rights spent, the other side's rights kept), promotions and capturing promotions to all four pieces for both colours (including capturing a rook on its home square), and an eight-move sequence unwound through every intermediate position. A null move gives exactly the position a fresh parse of the expected FEN gives, hash included (White and Black passing, an en passant square in the hash and one not in it); two null moves give the first hash back; the halfmove clock stops at its limit. |
| `board` move parsing | Kind detection for pushes, double pushes, captures, en passant (needs the en passant square), castling (king from its home square only), promotions and capturing promotions; display round-trips; malformed text and moves that do not fit the position are rejected; the documented lack of geometry checking is pinned by a test. |
| `attacks` knight | Table equals the eight-step definition built with `Square::offset` for all 64 squares; counts for corner, edge and centre squares and the total of 336; symmetry. |
| `attacks` king | Table equals the eight-neighbour definition for all 64 squares; counts for corner, edge and centre squares and the total of 420. |
| `attacks` pawn | Both colours equal the two-step definition for all 64 squares; edge files, last ranks and the total of 98 per colour; the reverse-lookup identity for every pair of squares. |
| `attacks` scan | Rook on an empty board equals rank plus file for all 64 squares; bishop counts and diagonal property; both stop at and include the first piece on each ray; a piece on the slider's own square changes nothing; a full board leaves only neighbours. |
| `attacks::magic` masks | Mask sizes for sample squares and the classic table totals (102,400 rook and 5,248 bishop entries); the rook mask of a1 square by square; for every square and every arrangement inside the mask, filling all squares outside it does not change the scanned attacks; `subsets` yields each subset exactly once. |
| `attacks::sliders` | Exhaustive: for all 64 squares and every relevant blocker arrangement (107,648 cases) the bishop and rook lookups equal the scan; 20,000 random full-board occupancies including queens; table layout and sizes; the seeded search, run twice, finds exactly the 128 embedded magic numbers and fills exactly the engine's table; a number that is not magic is refused when a slice is filled; four pinned magic numbers and a fold of all 128 (recorded from this implementation as regression pins). Which lookup the build uses is checked by the slot of one blocker. With the feature `pext` (run with `RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test --release --features pext`): the slot equals a bit-by-bit definition of PEXT for every square and relevant arrangement, ignores pieces off the mask, and uses every slot of a slice exactly once; the two tests that are about the magic numbers as a lookup (a wrong number is refused, the table equals the one the seeded search fills) apply to the default build only, while the seeded search still has to find the embedded numbers. |
| `board` attack queries | Attackers of sample squares in the start position; check given by each piece kind, including pawns of both colours from both sides; positions that are not check (blocked sliders, pawns behind or straight ahead of the king); double check; only the side to move is asked about; a custom occupancy sees through a removed blocker; adjacent kings. |
| `board` diagram | The start position drawn line for line; pieces on a8, d4, g1 and h8 land where a board has them; every line of three positions is 17 characters wide, starts with its mark and has no double space. |
| `movegen::MoveList` | Starts empty; push keeps order; slice methods work through `Deref`; equality ignores unused slots; holds exactly 256 moves and panics on the 257th. |
| `movegen` pseudo-legal | Expected move lists as sorted UCI strings with captures marked. Knights: start position for both colours, captures and own-piece blocking, a corner. King steps: both colours, captures, own-piece blocking, the centre, and that attacked squares are still generated at this stage. Sliders: bishop, rook (both colours) and queen stop at and capture the first enemy, stop before own pieces, and have no moves in the start position. Pawn pushes: 16 moves for each colour at the start, pushes blocked at the first or second square, double pushes only from the starting rank. Pawn captures: both diagonals for both colours; no capture of own pieces or straight ahead; no capture around the board edge, with the bait placed on the exact square a wrapping shift would reach, for both edge files and both colours. Promotions: four pieces for pushes and for captures, both colours; a blocked pawn cannot promote; moves onto the last rank are always promotions. En passant: both colours, two capturing pawns, none without the square, none without a capturer, no wrap around the edge for either colour. Castling: both ways for both colours; needs the right; needs empty squares between king and rook (own or enemy pieces, including b1); not out of check; not through or into an attacked square, with the crossed and destination squares each tested for both colours; an attacked b-file square is allowed. |
| `movegen::legal_moves` | The published depth-1 perft counts of the six reference positions (20, 48, 14, 6, 44, 46); pinned pieces (including moving along the pin line); answering check by capture, block or king move; the king may not step onto attacked squares or next to the other king; checkmate and stalemate give no moves; en passant that exposes the king along the rank is illegal; the position is left untouched; legal moves keep the pseudo-legal order. |
| `perft` | Start position depths 0 to 3 (1, 20, 400, 8,902); depth 1 equals the number of legal moves; `divide` sums to `perft`, lists each legal first move once and gives 600 for `e2e4` and 380 for `a2a3` at depth 3; depth 0 and checkmate edge cases; the position is restored. |
| `cli` `search` | One line per depth and then the best move, identical on repeated runs; the exact first line from the start position; mates reported in moves with the right sign, stopping at the depth that proves them; checkmate and stalemate at the root; bad depths and FENs are usage errors. |
| `cli` `eval` | The exact output for the start position; both points of view after 1.e4 (+40 for White, -40 for the side to move); quoted and unquoted FENs; material and phase lines for a lone queen; a bad FEN is a usage error. |
| `epd` | A record with `bm` and a quoted `id`; several moves, odd spacing, an empty operation and no last semicolon; a semicolon inside quotes stays in the operand; an opcode alone has an empty operand; `hmvc` and `fmvn` are kept as operations and do not reach the position; a bare position has no operations. Refused: fewer than four fields, a position the FEN rules refuse, an open quote. |
| `suite` | A suite of eleven lines written for the test, pinned line for line at depth 2: two mates in one (either of two mating moves counts), a hanging queen, a move to avoid, a best move that is wrong on purpose (failed), and lines skipped with their reason (no `bm` or `am`, a move that is not one legal move, a position the FEN rules refuse, a stalemate, whose moves cannot be read); comments and blank lines are passed over; the total line. The output is the same on every run with a depth or a node limit, fewer nodes are used with a smaller limit, a move time works as a limit, and an empty suite is zero of zero. |
| `cli` | No arguments writes only to stderr; `help` prints the usage text; unknown commands exit with status 2; `perft` from the start position and from a quoted or unquoted FEN; `--divide` output format, order and total; bad depths and FENs are usage errors; `bench` prints `<nodes> nodes <nps> nps` with the same count on every run, accepts `bench <depth> <threads> <hash>`, where the default hash size gives the count of the bench without arguments and other sizes run, and refuses a bad depth, more than one thread, a hash size outside the option's range and a fourth argument. `search --stats` adds five lines after the best move of a depth-3 search (21, 113 and 1,114 nodes, factors 5.38 and 9.85), leaves what comes before them unchanged, and gives the same output wherever the flag stands. `suite` runs a file (with a byte that is not UTF-8 in a comment) and prints a line per record and the total, with the limit before or after the file and with each of the three limits; eleven wrong command lines (no file, no limit, two limits, a bad number, an unknown flag) are usage errors; a file that is not there gives status 1 and no usage text. |
| `bench` | Every bench position is a valid FEN; the node count is the same over three runs; the speed is nodes over seconds, and 0 when no time was measured. At depth 5 the default hash size gives the plain count, a table of one slot a larger one, and a size no machine has is refused. |
| `perft` copy-make | `perft_copy_make` gives the same counts as `perft` at every depth for three positions; the `--copy-make` flag works and cannot be combined with `--divide`. |
| `eval` interface | `relative_to` flips the sign for Black; an evaluator works as a `Box<dyn Evaluator>` and reports from the side to move's view; an evaluator can be moved to another thread and shared by four threads at once. |
| `eval::Material` | Piece values keep the usual orderings (bishop above knight, two rooks above a queen, rook above knight plus pawn); the start position is balanced at full material; a missing queen scores +900 or -900 depending on the side to move; rook against knight and pawn, bishop against knight, bare kings. |
| `eval` game phase | The start position is 24; kings and pawns are 0; each piece kind adds its weight for both colours; nine queens are capped at 24. |
| `eval::Tapered` | `blend` equals `mg` at phase 24 and `eg` at phase 0, is a weighted average in between and monotonic; a flat pair blends to itself; `blend(-x) == -blend(x)` at every phase, including fractional averages; component-wise arithmetic; a phase outside 0 to 24 panics. |
| `eval` piece-square tables | Visual tables map to the right squares; Black uses each table flipped top to bottom; every table is left-right symmetric; the pawn table is zero on the back ranks, accelerates towards promotion in the endgame and prefers advanced centre pawns in the middlegame; the balance is White minus Black, zero at the start, and negates when colours are swapped and the board flipped; knights and bishops prefer the centre to the rim and development to the back rank; rooks prefer the seventh rank and central files; the queen's middlegame values are small and her endgame values favour the centre; the king scores best on g1 and b1 in the middlegame, both castled squares beat the home square, every step up the board is worse, and the centre is best in the endgame; every piece kind has a table. |
| `eval::HandCrafted` | Colour symmetry on eight positions (a position and its colour-swapped mirror score the same for the side to move, and every part of the breakdown negates); the side to move only changes the sign; the start position is exactly level; the breakdown after 1.e4 adds up by hand; material outweighs placement; the total equals material plus the blend at the position's own phase for twelve positions covering seven different phases; one advanced king scored exactly at three phases (-7 with queens and rooks on, +18 with queens only, +44 with bare kings); `default_evaluator` is the hand-crafted one. |
| `search` scores | The ranges nest (evaluations below `MATE_BOUND`, mates below `INFINITY`); `mated_in` and `mate_in` mirror each other; quicker mates and slower defeats score higher; ordinary scores are not mate scores; `moves_to_mate` counts full moves with the right sign. |
| `search` negamax | Alpha-beta returns exactly the score of an unpruned minimax on nine positions (to depth 3 on quiet ones, depth 2 on busy middlegames, because unpruned minimax with a quiescence search at every leaf is very expensive there), visits no more nodes, restores the position, and names a move that leads to that score; pruning more than halves the work at depth 4 in a rook endgame; a hanging queen is taken; mate in one and mate in two are found at the right depth with the right score, and not one ply early; the losing side sees mate coming; checkmate and stalemate at the root; the three stalemating moves of a queen ending are avoided at depths 1 to 3 and no mate is reported before it is there; node counting; depth 0 is rejected. Killer moves: after a depth-5 search of the start position the root and the plies from the horizon down have none, plies 1 to 4 hold at least four between them, none a capture or a promotion; the same search finds the same ones, and the next search starts without any. Null-move pruning: the reduction is 2 plus a quarter of the depth; the pieces of the side to move are counted without pawns and king; a null move applies a queen up at depth 3 and not below depth 3, with the evaluation below beta, with a mate score as beta, after a null move or during a verification, in check, with pawns and king alone, or switched off; against the same search without it, it saves over a quarter of the nodes on the start position at depth 6 and on two middlegames at depth 5 with no verification; in three pawn endgames at depth 7 no null move is tried and the search is the same node for node; in the rook endgame every fail-high is verified; four mates are found with the same score and move. Late move reductions: the reduction at seven points worked out by hand; from depth 3 and the fourth move on it is at least one ply, never less for a later move or a deeper search, and always leaves a ply; none below depth 3; the table of logarithms is within one thousandth of `ln`; against the same search without them (both without null moves) they save over a quarter of the nodes on four positions, with fewer than half of the reduced moves searched again; four mates are found with the same score and move. Reverse futility pruning: at each depth from 1 to 6 an evaluation exactly the margin (100 a ply) above beta cuts off with the evaluation less the margin and one point less does not; not above depth 6, not against a mate score, not in check, not switched off; against the same search without it (both without null moves and reductions) it saves nodes on four positions; four mates are found with the same score and move. Root moves: restricted to two quiet moves where a queen can be taken, the search answers with the better of them and its own value, below the free search, leaves the root out of the table, answers within the list at every depth when iterating, and with an empty list is the free search. The verification of a null move: with a rook far above beta, where reverse futility pruning would answer at once, it visits more than three positions beyond the null move's own search, agrees with a plain search of the position at the reduced depth, and leaves no flag set. |
| `search` draws | Bare kings and single minor pieces are insufficient, anything that might mate is not; knights going out and back repeat the start position; the same placement with the other side to move does not; with a made-up history, a matching key an odd number of plies back or beyond the halfmove clock's reach does not count; empty and short histories are safe; the fifty-move threshold is 100 plies. In search: the losing side takes a repetition from the game history that it cannot see without it; a perpetual check with no game history is a draw by the hashes the search records on its own way down, both when the line returns to the root and when it returns to a position one ply below it, and those hashes are all taken back afterwards; the fifty-move rule ends a line that had a mate, except when the mate lands on the hundredth ply; bare material is a draw inside the tree, at the first node of every line (checked by node count). |
| `search` principal variation | On five positions (to depth 3 or 4 on quiet ones, depth 2 on busy middlegames) the line starts with the best move, consists of legal moves, runs the full depth, and ends in a position whose quiescence value (seen from the root side) equals the reported score; the line of a mate in two is three plies and ends in checkmate; no line when the root has no legal move. |
| `search` limits | `Limits` constructors and combinations. A node limit stops the search exactly at the limit and leaves the repetition history as it was; a search cut short at three different limits answers only from the root moves it finished, never from the one it was working on; a generous limit changes nothing; a raised stop flag ends the search before the first node while an unraised one does not disturb it; a deadline already passed ends the search within one check interval. `Limits::mate` sets only the mate limit. |
| `search` iterative deepening | Iterating to depth 3 equals a direct depth-3 search on three positions, with one report per depth that matches a direct search of that depth and with nodes and time increasing; iteration stops at the depth where a mate is proven (3 for mate in two, 2 for being mated in one) and not before: a mate in two already seen at depth 1 through quiescence is searched on to depth 3; it ends after one iteration when the root is checkmate or stalemate; a node limit that interrupts an iteration returns the last completed depth; a first iteration of ordinary length (Kiwipete's, longer than the ordinary interval between looks at the clock) completes even with a one-node limit, a raised stop flag or a passed deadline, and a limit reached between iterations does not count as an interruption; a long first iteration (eight queens a side, some 600,000 nodes) is ended by a raised stop flag and by a passed deadline once its first root move is done and at most 65,536 nodes later, unreported, with a legal move worth at least the first move's exact value and the path unwound, and the search can be used again; a one-node limit does not end that long first iteration; a depth limit of 0 still searches one ply; a `Search` stopped by a limit can be used again and starts afresh. The selective depth is the deepest ply visited: 1 after one move from the start position, more than 2 after two (quiescence goes below the depth), 3 for a queen trade below a depth of one, 0 with no legal move, and afresh for each search from the root; every report carries the value of its own iteration, which in the rook endgame tested is not below the depth (it can be, where every line ends sooner in a draw by rule). A mate limit ends the iteration at the first depth that shows a mate that short (depth 1 for a mate in two seen through quiescence, where the unlimited search goes on to depth 3); a shorter mate asked for than there is, a mate against the side to move, or no mate at all end nothing early. |
| `search` statistics | Rates are thousandths rounded down, with none for a count of zero and no overflow. Depth 1 from the start position: 20 quiescence nodes of 21, no cutoff in the main search or by a move in quiescence, between 1 and 19 stand-pat cutoffs; one quiescence position good enough as it stands counts exactly one stand-pat cutoff; queen takes queen is one cutoff by the first move. On three positions the first-move cutoffs are among the cutoffs and the quiescence nodes among the nodes, and the same search again counts the same, not double. The lines of text: nodes per depth as differences, the branching factor in hundredths, rates as percentages with one decimal, `-` where there is nothing to divide by, and no factor after an iteration that added no node. |
| `search` transposition table | The table: 65,536 slots to the MiB, always a power of two, one slot at least, and a size no machine has is refused, not died of; what is stored is found and nothing else (a hash of 0 included; another position wanting the same slot is not given it); the same position searched less deep does not displace a deeper result, deeper or equal does, a result without a move keeps the known move, another position takes the slot; mate scores are kept as distances from the position. In the search: on five positions the search with a table gives the score and move of the search without, in fewer nodes; four mates are found at the same distance; the same position searched again takes less than a quarter of the nodes, and after `clear` the first search's count again; a table filled from a position with a low fifty-move count does not hide the draw at a count of 98. The table's move goes to the front of the order and nothing else moves; a move that is first anyway stays first once; no move, or one the position does not have, leaves the order as it was. `hashfull` counts the used slots per thousand over the first thousand slots, or over all of a smaller table, and is 0 again after `clear`. The statistics count no probe without a table; with one, a probe for every main-search position below the root, the hits among the probes and the cutoffs among the hits, and a higher hit rate when the search is made again. The comparison of a search with a table and one without is made on exact searches, with null moves and reductions off. |
| `search` static exchange evaluation | Hand-worked exchanges: free and defended pieces, even trades, a recapture that is not made because it would cost more, x-rays through rooks, a bishop and a pawn for both sides, kings that capture only what is not defended, quiet moves onto safe and attacked squares, promotions with and without a capture, a pawn that promotes while recapturing, en passant (also opening a file), castling. Against an oracle: for every move but castling of every position two plies around thirteen busy positions (more than 400,000 moves), the value equals that of the same exchange played out move by move on copies of the board. The oracle was checked by breaking the evaluation on purpose (x-rays, promotions, en passant, the right to stop): each break makes it fail. |
| `search` quiescence | A quiet position is worth its static evaluation; a winning capture is played out; a queen does not grab a defended pawn at depths 1 to 3 (the horizon effect); no standing pat in check (checkmate is scored by ply, a forced recapture is followed); queen promotions are considered; it terminates on a position full of captures in under a million nodes and restores it; bare kings after a capture are a draw; which moves count as tactical. Capture ordering: most valuable victim first, least valuable attacker breaking ties, promotions and en passant in their place, and the ordering only reorders (same moves, captures before quiet moves, generation order kept when nothing can be captured). The minimax-agreement and principal-variation tests now value their leaves with a full-window quiescence search. Stalemate at the horizon is a draw whatever the window, and a one-ply search does not walk into one; the two shortcuts that rule stalemate out without playing moves agree with full move generation on every position not in check of five trees (over 5,000 positions, with pins, en passant and stalemates), and a position whose only movable piece is pinned is recognised as stalemate. In check there is no standing pat (a skewered queen is not counted as material kept). A dead position is valued by its evaluation on its own, at ply 0, and as a draw inside a line. Out of check a capture that loses material is left out (a rook facing a pawn defended by a pawn is judged as it stands, in one node, and takes the same pawn undefended); in check it is tried like every move, and a position whose only way out of check is such a capture is not called mate. The main search's order: a capture that loses material comes after the quiet moves (queen takes queen, the quiet moves as generated, then the two captures that lose the queen, in the order of their victims), and where nothing is defended the order is that of the victims; whether a capture loses is a matter of the whole exchange (a defended pawn, an even trade, a second attacker behind the first, a promotion that is taken, en passant met by a recapture); both orders only reorder. |
| `movegen` move input | `find_legal_move` returns every legal move of five positions from its text and agrees with the trusting parser on them; it refuses moves no piece can make (which the trusting parser accepts), the other side's moves, pinned pieces, moves that leave the king in check, promotions without or with a wrong letter, castling through an attacked square, and a dozen strings that are not moves. |
| `movegen` SAN input | `find_san_move` reads pawn and piece moves, captures and en passant, tells a pawn on the b-file from a bishop by the letter's case, and castling written with letters or zeros (not when it is not legal); a promotion needs its piece, with or without `=`, and not a king; a text that fits two legal moves gives none, and file, rank or whole-square disambiguation picks one; a pinned knight does not count, so the text means the other; check and annotation marks and the long form are accepted and 19 kinds of nonsense give nothing; on five positions every legal move is found from its piece, origin, destination and promotion written in full. |
| `uci` command parsing | Each command is recognised; blank lines, extra white space, a trailing carriage return and unknown words before a command are forgiven; unknown lines are reported as such and commands are case-sensitive; `position` from the start or a FEN, with and without moves, and six malformed forms; `go` with clocks (any negative number counts as zero and what follows it is still read), with depth, nodes, move time and `infinite`, ignoring `ponder`, `searchmoves` and `mate` and skipping parameters without a usable value; `setoption` with names and values containing spaces, without a value, with an empty value, and four malformed forms. `d` and `eval` are commands only as the whole line: with a protocol command after them that command is read, and anywhere else they are unknown. `go searchmoves` reads the words up to the next parameter, wherever it stands and whatever they are; an empty list is no list; given twice, the lists add up. `go mate` reads its number, alone and with other limits, and is skipped without one. |
| `uci` output | Scores as `cp` or as `mate` in the engine's own moves, negative when it is the one mated; `info` lines with depth, score, nodes, speed, time and line, without a `pv` word when there is no line, with no division by zero for an instant search, and with the speed taken from the full time; `bestmove` with a move and `0000` without one. `seldepth` follows the depth, and is left out when it is 0. `hashfull` goes between the speed and the time when the search has a table, and is left out when it has none. When asked for, `wdl` and the model's three figures follow the score (also a mate score) and nothing else about the line changes. |
| `uci` win, draw and loss | The module's exponential agrees with the standard one to twelve digits from -40 to 40; the model's figures at six scores, worked out by hand; for every score from -3000 to 3000 the three add up to 1000, mirror those of the opposite score, and the win never falls as the score rises; a forced mate is a certain win or loss. |
| `uci` time management | No clock and no move time means no time limit, also when only the other side's clock is sent; `movetime` is used less the overhead and takes precedence over a clock; the overhead is the one passed in (six values from 0 to 60 s, for a move time and for the last move before a time control), caps the time without shortening an ordinary share, and moves the deadline; each side uses its own clock and increment, and a clock sent without increments works and does not borrow the other side's; `movestogo` divides the time, and 1 or 0 allows all that is left less the overhead; the plan never exceeds the clock for 96 combinations of time, increment and moves to go, including a fallen flag and a large increment on an empty clock; limits combine the deadline with depth and nodes; `infinite` switches the clock off. A clock at or below the overhead gets no time, whatever the increment, and one millisecond more gets that millisecond. For known issue KI-12 a game without increment is played out against the plan alone: the flag never falls (before the fix it fell on move 150 from one second and on move 77 from a tenth of a second), and with half a millisecond a move unaccounted for it falls on moves 162 and 94 (before: 123 and 55) |
| `uci` engine | `uci` and `isready` are answered; `go depth` makes exactly the search a direct call makes (same nodes, line and best move), which is the check that the protocol layer changes nothing; a new engine is at the start position; `position` with moves equals the same position as a FEN; an illegal move (five kinds) or a bad FEN refuses the whole command and keeps the position; the game history reaches the search (a repetition found only with it); `ucinewgame` resets; no legal move gives `bestmove 0000`; mates are reported in moves; node and move-time limits end the search; an infinite search and a bare `go` answer only after `stop`, and `isready` is answered meanwhile; `stop` ends a long search within two seconds, and says nothing without a search; `position` and `ucinewgame` end a running search before they act; `go infinite depth 1` searches one depth and still waits for `stop`; `go` thinks on the clock of the side to move, for both sides; `Move Overhead` reaches the clock (five seconds kept back from a five-second move time answers at once, and none kept back lets the search past the first depth); limited strength is given the game's history and the limits of `go`; full strength draws no random numbers; weakened play draws from the strength stream of the seed, checked against the library for six seeds; a second `go` ends the first; `quit` ends the search and the loop; what is not understood is reported with `info string`; every output line is a protocol line; at the end of input a search is allowed to finish and an infinite one is stopped; lines that are not valid UTF-8 are reported and the commands after them still obeyed, with the running search still reporting; a read error ends the loop only after the search has reported. `d` sends the board, FEN, hash key and checking pieces (both pieces of a double check) and `eval` the lines of `eval::report`, every line as `info string`; sent during a search they arrive as whole blocks and the search prints what it prints without them. With `UCI_ShowWDL` on, every `info` line of a full-strength and of a weakened search has `wdl` and three numbers adding up to 1000 straight after the score, and without them the output is the plain search's, move included. With `SearchStats` on, a depth-3 search of the start position sends five `info string stats` lines between its last `info` line and `bestmove` (21, 113 and 1,114 nodes; the two count lines equal those of the library's own search) and is otherwise unchanged; a weakened search sends none. The table lasts a game: the same `go depth 5` again takes less than a quarter of the nodes with the same move, and after `ucinewgame` the output is the first search's line for line. `Hash` sizes the table from the next search on (2 MiB is 131,072 slots); the same size named again keeps the table, another size empties it, a refused value changes nothing. `Clear Hash` makes the next search the first search of the game again, line for line, in any letter case and with a value sent along, once per press; it says nothing and changes no option; pressed during a search, it leaves that search its table and empties it before the next. `go searchmoves`: the answer and the first move of every line shown are among the moves named, in either order of the parameters, and the next search is free again; a word that is no legal move (unknown, of the other side, leaving the king in check, or a repeat) is reported and left out, and if none is left all moves are searched; with `UCI_LimitStrength` it is not used and says so. |
| `uci` options | Every declared option can be set to its declared default, which changes nothing (so the declaration and the real default agree); option names ignore letter case, for every option in three spellings, but must match in full, and `true` and `false` must be lower case; an unknown option is refused; `Hash` accepts 1 to 65536 and refuses eight kinds of bad value and a missing one, keeping the old value; `Threads` accepts 1 and refuses anything else; `Seed` is 0 by default, accepts 0 to 2147483647 in decimal and refuses larger numbers, negative ones and hexadecimal; the old `DeterministicRNG` and `RngSeed` are unknown options; `Variety` is 0 by default, accepts 0 to 100 and refuses five kinds of bad value; `UCI_LimitStrength` is off by default; `UCI_Elo` is 1360 by default, accepts 1110 to 1630, refuses the old ends 400 and 1800, and does not switch limiting on; `Move Overhead` accepts 0 to 5000, refuses six kinds of bad value and a missing one, and is not known under four near-miss spellings of its two-word name; the declared options are exactly the nine, in order; `UCI_ShowWDL` is declared off, reads `true` and `false` in any letter case of its name and refuses anything else. In the engine: `uci` lists the options after the `id` lines and before `uciok`; `setoption` changes the option silently, and a refused value is reported with `info string`; the seed comes from system entropy by default, lies from 1 to 2147483647 and is logged in reply to `uci`; `Seed` sets the seed with a log line, the same value again says nothing, a refused value keeps the seed, and 0 goes back to an entropy seed with a log line; every `ucinewgame` logs the seed, given or from entropy. No test relies on two entropy seeds differing. `Variety`: at 0, explicit or default, the output is exactly the plain search's for every seed, and the move is the best; at 60 with a fixed `Seed` and `go depth` each game is reproducible, the search's own `info` lines are exactly the plain search's (root only), some of twelve seeds play another move than the best, each says so in an `info string`, an independent exact search puts the move played within 60 of the best, and the last `info` line is for the move played; after the twelfth full move it changes nothing; under `UCI_LimitStrength` it changes nothing over three moves, so it draws nothing; every `ucinewgame` adds a seed line of its own. With `UCI_LimitStrength` on: one `info` line at the weakened depth and a legal move, whatever depth `go` asked for; a chosen seed makes a two-move game reproducible; twenty seeds do not all open the same way; `ucinewgame` restarts the sequence of a chosen seed, while without it the same position is not always answered the same; in a session without a chosen seed each of three games logs its seed, and replaying with that seed reproduces that game's moves, the second and third games included; switching the option on and off again leaves full-strength play exactly as it was; `UCI_Elo` 1110 and 1630 look 2 and 3 plies ahead, a rating out of range (3000, or the old lowest 400) is refused and the default kept, and without `UCI_LimitStrength` the rating changes nothing. `SearchStats` is declared off and reads `true` and `false` only. `Clear Hash` is declared as a button without a default, is recognised in any letter case and only in full, and cannot be set like a value; the options with a value are not taken for buttons. |
| `rng` entropy and streams | The stream indices are 0, 1 and 2 and are never renumbered; the first three outputs of the strength stream of the default seed are pinned (computed with an independent implementation); every entropy seed is a valid `Seed`, from 1 to 2147483647 and never 0, and any 64 random bits map to such a seed, the same every time; no test relies on two entropy seeds differing, which chance could break. |
| `search` variety | It applies only above 0 and up to the twelfth full move; never with a mate for or against; over three opening positions, three `Variety` values and eight seeds every choice is within `Variety` of the best and has the score an exact search of it gives; a move that leaves the queen to be taken is never chosen over twenty seeds, while other queen moves are; a rook move that allows a back-rank mate passes the cheap filter (checked) and the second check still never lets it be played, while other safe moves are; the same seed gives the same choice and twenty seeds play more than one move; the weight is largest for the best move and falls with every centipawn of loss; one number is drawn per choice; with `stop` already raised, or a budget of one node, only the best move is offered; the budget is split two thirds to the search, a depth limit stays with the search. |
| `search` weakening | Without noise it finds exactly the move and score of the ordinary search at the same depth, on seven positions including mates for and against; the same seed gives the same move; with noise the choice varies over seeds (at least five different legal first moves in forty); a mistake larger than twice the noise is never made (a hanging queen is always taken, and the reported score is the true one); a mate in one is played whatever the noise; errors are bounded, centred and mostly small, and a mate draws the same random numbers as any other move; no legal move; the game history is used (a repetition); with the deadline already passed or `stop` already raised it still takes a hanging queen and makes exactly the one-ply choice for the same seed, on 25 seeds, never the first generated move; a pass cut short by a node limit is discarded whole and the answer is the previous pass's, and a limit smaller than the first pass does not stop it; the generator advances exactly once per root move whatever the position holds; the fifty-move rule is honoured (a mate too late is a draw, one just in time is a mate); the seven measured settings of the calibration and its check give exactly their measured weakenings, the range is 1110 to 1630 and out-of-range ratings (the old 400 included) are clamped, the default 1360 gives three plies and 141 centipawns; between points of the same depth the misjudgement is interpolated, rounded towards more noise, and 1331 to 1349, where the depth changes, play as 1330 (two plies, 112 centipawns); a higher rating never looks less far, and at the same depth never errs more, over the whole range. |

Rows are added as modules land.

During the milestone 1 review, make/unmake was also stress-tested outside the
repository against an independent mailbox model over about 5.6 million
pseudo-legal moves, and the FEN parser was fuzzed with a million mutated
inputs; neither found a defect. Those harnesses were not committed. Their
role is now filled by `tests/perft.rs` and `tests/consistency.rs` for the
moves, and by `tests/parser_fuzz.rs`, which feeds the FEN parser 30,000
damaged inputs on every run.

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

This is the "fixed-depth node-count check": change detection that sits
alongside the SPRT, not in place of it. When a deliberate change makes a
pin fail:

1. Confirm the new behaviour is intended. For a search or evaluation change
   that means an SPRT result (see "Strength matches, SPRT and ratings").
2. Update the numbers in the same commit and say so in its message.

Never update a pin just to get a green run.

| Position | Depth | Score | Nodes |
| --- | --- | --- | --- |
| Start position | 4 | 0 | 2,419 |
| Kiwipete | 2 | +44 | 3,538 |
| Position 3 | 5 | +107 | 2,844 |
| Position 4 | 2 | -418 | 968 |
| Position 5 | 2 | +541 | 187 |
| Position 6 | 2 | +22 | 817 |
| Queen against two pawns | 4 | +689 | 3,533 |

These are the counts since reverse futility pruning (milestone 7, item
13), which changes three of them and no score or line; before it they
were 2,419, 3,538, 3,152, 968, 476, 817 and 3,551. (Position 3, the rook
endgame, is the one search here in which null moves are verified; it
counted 2,559 until the verification was made to search real moves.) Late move reductions
(item 10) reach only the three searches of depth 4 and 5. Six scores and
lines are what they have been since milestone 4. The seventh, the queen
against two pawns, is the first pinned score to change: +689 by
`e2g4 e8e7 g4f5 e7d8`, from +683 by `e1d2 e8e7 e2e4 e7e6` (and by
`e1d2 e8e7 e2c4 e7f6` before the killer moves of item 6, the same value
by another line). A search that looks less deep at late moves is not
exact, and neither is one that prunes by null moves (item 9): both can
change a score, which the heuristics before them could not, and their
SPRTs are what says the change is for the better. Before the reductions
the counts were 4,106, 3,538, 4,876, 968, 476, 817 and 3,642; before
null-move pruning 4,154, 3,538, 8,487, 968, 476, 817 and 3,642; before
the killer moves 25,541, 3,538, 15,159, 968, 476, 1,301 and 10,055; before
captures that lose
material were tried last in the main search and left out of quiescence
(item 3) 27,614, 13,161, 20,743, 2,322, 637, 4,563 and 11,155; and
before the main search tried captures first (item 1) 28,793, 391,308,
63,892, 23,562, 5,857, 27,964 and 15,714, the Kiwipete figure showing
what searching captures in generation order cost.

## The PEXT build

The Cargo feature `pext` replaces the magic multiplication in the slider
lookup by the PEXT instruction (see "Magic bitboards" in
[ARCHITECTURE.md](ARCHITECTURE.md)). It must find the same moves, so the
whole test suite is run on it as on the default build, the perft
acceptance tests and the pinned bench count included:

```sh
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test --release --features pext
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo clippy --all-targets --features pext -- -D warnings
```

CI does not run these: the workflows build the default features only.
Run them by hand after a change to `src/attacks/` or to move generation.
The feature is refused at compile time for a target without BMI2.

## Running a search by hand

```sh
cargo run --release -- search <depth> [--stats] [FEN]
```

searches the position (the start position without a FEN) by iterative
deepening to the given depth. It prints one line per completed depth, in the
vocabulary of UCI `info` lines, and then the best move:

```
depth 1 score cp 40 nodes 21 pv d2d4
depth 2 score cp 0 nodes 97 pv d2d4 d7d5
bestmove d2d4
```

- `score cp N` is in centipawns for the side to move; `score mate N` means
  mate in N moves (negative: the side to move is being mated).
- `nodes` counts positions visited since the search began, quiescence search
  included.
- `pv` is the line of best play the score comes from.
- Time and speed go to stderr, so stdout is identical on every run and can be
  compared between versions.

**Search statistics.** With `--stats` the search's statistics follow the
best move, each line starting with `stats`:

```
stats depth 1 nodes 21
stats depth 2 nodes 76 ebf 3.61
stats depth 3 nodes 649 ebf 8.53
stats depth 4 nodes 860 ebf 1.32
stats cutoffs 337 firstmove 309 rate 91.6%
stats qnodes 1143 of 1606 (71.1%) standpat 696 qcutoffs 36 firstmove 36 rate 100.0%
stats table probes 459 hits 129 (28.1%) cutoffs 73 (15.9%)
```

- A `depth` line gives the nodes that iteration took by itself (the
  search's lines above count from its start), and `ebf`, the effective
  branching factor: how many times more nodes it took than the iteration
  before. Good pruning makes it small.
- `cutoffs` are the beta cutoffs of the main search, `firstmove` those
  given by the first legal move tried, and `rate` their share. It is the
  usual measure of move ordering: the better the ordering, the nearer to
  100%. The 92% above is the start position, where there is little to
  capture, so it is the quiet moves that have to be tried in a good
  order: the transposition table's move first, then the killer moves.
  It was 22% before the table's move was tried first, 58% while
  captures that lose material were still tried before the quiet moves,
  and 62% before the killer moves. At the end of milestone 7, searched
  deeper, it is 93.8% from the start position at depth 6, and at depth
  8 97.5% in Kiwipete and 90.3% in a quiet middlegame (Position 6 of
  the pins). This is the measurement that known issue KI-9, "the search
  is slow for lack of move ordering", was closed on.
- `qnodes` are the positions the quiescence search visited, out of the
  search's total; `standpat` those good enough as they stood; `qcutoffs`
  and the `firstmove` after it the cutoffs by a move there, which is
  ordered most valuable victim first (hence 100.0%).
- The figures are whole-number arithmetic on counts, so they too are the
  same on every run. They are a quick look at what a change did to the
  tree, not a measure of strength: that is what an SPRT is for.
- The counts are always kept and cost nothing measurable; the flag only
  shows them. They cannot change the search, which never reads them: the
  bench node count is the same with and without.
- `table` is the transposition table. `probes` are the times the main
  search asked it about a position: once for every position it visits
  below the root, so the probes, the quiescence positions and the root
  add up to the nodes. `hits` are the probes that found the position, at
  whatever depth (a hit gives at least a move to try first), and
  `cutoffs` the hits that settled the position at once. Both percentages
  are shares of the probes. They grow with depth, as more of the tree is
  a position seen before: at depth 6 from the start position 25.9% of
  the probes hit and 18.3% cut off. The quiescence search does not ask
  the table.
- In UCI mode the option `SearchStats` sends the same lines as `info
  string` just before `bestmove` (see [UCI.md](UCI.md)), for looking at a
  search on the clock or in a GUI's log. It stays off in strength matches.

A fixed-depth search uses no clock and no randomness, so it is fully
reproducible: the same position and depth always give the same lines.

## Bench

```sh
cargo run --release -- bench [depth [threads [hash]]]
```

searches the seven positions of `src/bench.rs` (an opening, middlegames
with and without tactics, endgames) to depth 4, or the depth given, at full
strength, each as a new search with iterative deepening and a new, empty
transposition table of the default size (16 MiB), and prints one line:

```
28175 nodes 3141200 nps
```

- **The node count is the fingerprint of the search.** It depends on what
  the search does, not on the machine or the clock, so it is the same on
  every run of the same build: full-strength search draws no random
  numbers, which is what playing with `Seed` fixed and `Variety` 0 means.
  A pure speed change must leave it unchanged; a change that alters it is a
  search change and needs its SPRT. `tests/bench.rs` pins it.
- **The speed varies** with the machine and with what else it is doing.
  Compare speeds only on one machine, idle, over several runs.
- The figures for each measured build, with perft speeds, are kept in
  [`testing/bench-history.md`](testing/bench-history.md).

- **`bench <depth> <threads> <hash>`** is the form several other engines
  share, accepted so that tools and habits carry over. The threads must be
  1, since there is one search thread. The hash size, in MiB and within
  the range of the option `Hash`, is the size of the transposition table
  each position gets. The node count is the same on every run *with the
  same arguments*: a smaller table forgets more, so another size can give
  another count (at depth 6, 127,170 with 1 MiB against 126,953 with 16). The bench
  without arguments always uses 16 MiB, the engine's default, and that is
  the count that is pinned. If there is no memory for the size asked for,
  the command says so and ends with status 1.
- **OpenBench** runs `./binary bench` with no arguments and reads the last
  `<n> nodes` and `<n> nps` it finds in the output (its "Requirements for
  Public Engines" page and `Client/bench.py`, read on 2026-11-27). The
  line above is what it expects. It also needs a makefile that builds the
  engine as `make EXE=<name>`, which this repository does not have.

The positions and the depth were chosen when the main search had no move
ordering and Kiwipete's depth 4 alone took about 25 seconds. With captures
tried first the whole bench takes a fraction of a second. The set and the
depth stay as they are through milestone 7, so that the counts of its
commits can be compared; queued task 2 widens the set and deepens it.

## Integration tests

Integration tests live in `tests/` and use only the public API. The table
also lists the Python tests of the helper scripts in `scripts/tests/`, run
with `python3 -m unittest discover -s scripts/tests`.

| File | What is checked |
| --- | --- |
| `scripts/tests/test_install_tools.py` | Python, offline. The tool installer's helpers: every tool is pinned to an exact version; downloads must be HTTPS, redirects included, except plain HTTP for a download that is allowed it, which then needs a digest, is still checked and may only be redirected to HTTP or HTTPS; archive extraction refuses paths and symlinks that escape, chained links included; a 7-Zip archive is listed and refused for escaping paths, links or special files before Windows' tar unpacks it, refused without that tar, and unpacked for real where it exists; the build of the ladder's engines comes from the argument, then the environment, then `local.env` (found through `VYVRAEL_LOCAL_ENV` too, read with or without a byte-order mark or as UTF-16), then `avx2`, and an unknown build is refused; a failed reinstall leaves the old install alone and an interrupted one is recovered; `--prefix` keeps everything under it; launchers and the systemd unit quote paths with spaces; the unit restarts only on failure and gives up after three failed starts in ten minutes; the lichess-bot configuration gets the engine path and an empty token; token files are owner-only. |
| `scripts/tests/test_vyvrael_release.py` | Python, offline; the symbolic-link cases are skipped where the user cannot make links, and the cases that run a shell script as the engine are skipped on Windows. `install_tools.py --vyvrael`: versions with or without `v` are accepted and anything else is refused; the file names and HTTPS addresses follow the release artifacts; a `.sha256` line must name its file; an existing link given with `--engine` is not followed; a release is checked and linked relatively, with no staging left behind; an upgrade re-points the link and keeps the older file; a plain file at the engine path is kept as a copy, and stays in place if the link cannot be swapped; a binary that names another version, a wrong digest or a failed download changes nothing; a missing release mentions drafts and the plain build, while no network or a rate limit is an error message without that hint; an `--engine` named like the release file is refused; a dry run through `main` fetches and writes nothing; `--verify` with `--vyvrael` is refused; `--list` shows the link. On the Windows side, tested on any system: the `.exe` and its `.sha256` are fetched and the file kept in `enginesyvrael-VERSION` with no link and no staging left behind; both builds share that folder; another version, a wrong digest or a missing file keeps nothing, with the same hint; `--list` names the releases kept; through `main` on Windows it installs no tool, a dry run fetches and writes nothing, and `--engine` and `--verify` are refused. |
| `scripts/tests/test_match_sh.py` | Python driving bash, offline; skipped without bash. `scripts/linux/match.sh` is given a stand-in `cutechess-cli` that only answers `-help`, and the command it would run is read from `--dry-run`. The exact command of each mode (`sprt`, `h2h`, `gauntlet`, `roundrobin`), with names containing spaces kept as one argument; in `roundrobin`, the whole engine list or the named engines after the `--build` builds, and its refusals (fewer than two engines, a name twice among builds and list engines, a bad or missing build, an unknown or missing list engine, the other modes' options, `--build` in the other modes, a `cutechess-cli` without `-tournament`), checked in the same order as `match.ps1`; gauntlet opponents taken from an engine list with their options; every optional setting; settings read from the local file, then the environment, then arguments; refusals for missing SPRT bounds, a missing or bad round cap, missing engine files, colliding or non-ASCII names, unknown or missing opponents, more games at once than physical cores, a missing opening file, a missing `cutechess-cli` (a warning only in a dry run), a `cutechess-cli` without a needed option, bad usage, and an existing PGN. With a stand-in that prints a result instead of playing: the progress is shown and logged; the run notes hold the command, names, files, digests, tool version, times, status and outcome, for a gauntlet with every opponent; the last SPRT line decides between H1 accepted, H0 accepted and undecided, and a log without one concludes nothing; a failing `cutechess-cli` fails the script with its status and is recorded. Also, for bash only: the default games at once are the physical cores minus two, and a core count given in `PHYSICAL_CORES` is used; an `lscpu` that fails means the cores are unknown, not an error; a byte-order mark and CRLF in the local file; numbers with a leading zero are decimal; a bad `LOW_PRIORITY` setting is refused unless an argument replaces it; plain ASCII build names are accepted, and an accented one refused, under five locales. |
| `scripts/tests/test_ladder_engines.py` | Python, offline. The pins of the ladder's opponent engines, for each system and build: on Linux the names provided, the names left out and the names retired are exactly the 30 of Ladder Baseline 2's ratings file, with no engine invented, the three left out are Orion 1.1, Ethereal 12.75 and Nostradamus 0.4, and the one retired is Arasan 26.1-1800; besides them only the eleven engines new in Ladder Baseline 3 are provided, none of them a Baseline 2 name; on Windows all 40 names that are not retired are provided, by 28 downloads and no builds; an engine missing on a system has a reason there; no generated name carries a stray space; the limited entries are variants of 4 binaries, 12 in all, each inside its engine's `UCI_Elo` range; every licence is GPL, AGPL, MIT, MPL-2.0 or stated freeware terms; every address is HTTPS with a SHA-256, or plain HTTP only for a pin that says why; a closed-source engine's notes state its terms; paths stay inside the install; nothing is built with `native` or for AVX-512, and every build names its tools and its architecture (Vice 1.1 excepted: its makefile names none); no `avx2` pin mentions PEXT or BMI2, and on Linux exactly Arasan, Inanis, Stash and Weiss differ in the `pext` build; a pin resolves per system and build without one system's fields leaking into the other's; fixed options never repeat Hash, Threads or the strength settings. |
| `scripts/tests/test_ladder_install.py` | Python, offline. Made-up engines installed from archives the tests write, with the download replaced by a local copy and the build by this Python: a bare binary, a binary in a tar archive with files and directories beside it (and nothing else from the archive), one member of a zip shared by two engines and fetched once; a zip member with the files it needs beside it, and nothing else from the zip; plain HTTP allowed only for a pin that says why; a build from source, from a tar.gz or a zip, and a zip source archive that escapes refused; a missing build tool named, with nothing fetched; a failing build, or one that makes no binary, leaves nothing behind and does not stop the other engines; a wrong digest of the download or of a zip member is refused; paths may not leave the archive; a second run installs nothing again unless forced, the pin changed or the program is gone; `--only` by ladder name, a limited entry selecting its engine, and unknown, uninstallable or retired names explained; a dry run changes nothing, and only notes a missing tool; an engine without a digest is refused; a download that fails does not stop the other engines; a failed reinstall leaves the working install as it was, on disk and in the record; an engine missing a file it needs beside it is installed again. The engine list: only what is installed, a limited engine three times under the ladder's names with one program, options only where the engine has them, and a pin's fixed options in every entry; from the real Linux table, the 37 names in order with Ladder Baseline 2's values (books off, 30 ms of move overhead under each engine's own option name, CT800's settings), Rodent IV told not to limit itself, and the same values for the engines new in Baseline 3 (Blunder 7.6.0's book off, Inanis's move overhead). |
| `scripts/tests/test_ladder_verify.py` | Python, offline, with small UCI engines written by the tests. A working engine passes and is sent `uci`, its options, `isready`, `ucinewgame`, `position startpos`, `go depth 6` and `quit` in that order; an engine that says nothing, exits, never becomes ready, thinks for ever or names no move fails with a message naming the step, and none of them holds the check up; one that ignores `quit` is killed; a program that is not there is reported; every name of an installed engine is checked with its own options and reported as PASS, FAIL or SKIP. |
| `scripts/tests/test_openings.py` | Python, offline. `--openings` with the download replaced by a zip the tests write: the `.epd` is installed under `<prefix>/openings`, recorded, and its `OPENINGS=` line printed, and nothing else is installed, on Linux and on Windows; a second run fetches nothing unless forced or the file was changed; a wrong `.epd` digest, or a zip without the `.epd`, is refused and leaves nothing behind; a swap that fails leaves no `.new` file and says why; a failure, or a network error, is reported without a traceback and does not stop the engines; it combines with `--tool` and `--verify`; a dry run fetches and writes nothing; `--list` shows it (on Linux). The real pin is one commit over HTTPS, CC0-1.0, and its `.epd` digest is Ladder Baseline 2's. |
| `scripts/tests/test_windows_tools.py` | Python, offline, on any system. The Windows pins (HTTPS, a SHA-256, the same versions as the Linux pins); both tools unpacked whole from zips the tests write, with their `local.env` lines printed; a second run installs nothing again unless forced; a tool that does not start is not recorded; a wrong digest, an archive without the program and unsafe archive paths are refused; a missing Visual C++ runtime is reported, not installed; lichess-bot is refused on Windows; `--engines` there takes the Windows pins of the chosen build (the `avx2` or `pext` file of Ethereal) and installs no tool, and its dry run downloads nothing; a dry run changes nothing; the check fails a program that ends with an error and no output; `--tool all` means both tools, and the whole list is checked before anything is fetched; `--list` shows the Windows tools. |
| `scripts/tests/test_match_ps1.py` | Python driving PowerShell, offline; skipped without PowerShell. The cases of `test_match_sh.py` except those it lists as bash only, against `scripts/windows/match.ps1` with a stand-in `cutechess-cli` written in PowerShell: the command of each mode, `roundrobin` included with its refusals, opponents from the engine list (a bare command joined to its working directory), optional settings, the order of local file, environment and arguments, every refusal, and the notes, log and SPRT outcome of a run; and, on Windows only, a real program that writes warnings to stderr under Windows PowerShell 5.1, which must not end the match. The two scripts are meant to behave alike, and the two test files are kept in step to show it. |
| `scripts/tests/test_rate_sh.py` | Python driving bash, offline; skipped without bash. `scripts/linux/rate.sh` is given a stand-in Ordo. The default command is the baseline's (`-a 0 -A "Stockfish 19" -W -s 500`); each option; several PGN files go through Ordo's list file, or one joined file for an Ordo without that option, and the helper file is removed afterwards; a real run prints the order and the reminder and writes both output files; refusals when the anchor played in none of the games (naming the players found), for a missing PGN, bad values, a missing Ordo (a warning only in a dry run), an Ordo without a needed option, and an Ordo that fails; a table left by an earlier run does not pass for this one's; Ordo's location read from the local file. |
| `scripts/tests/test_rate_ps1.py` | Python driving PowerShell, offline; skipped without PowerShell. The same cases as `test_rate_sh.py`, against `scripts/windows/rate.ps1` with a stand-in Ordo written in PowerShell. |
| `scripts/tests/test_record_sprt.py` | `record_sprt.py` on notes and logs written by the tests in both scripts' forms (line feeds or carriage returns, a byte-order mark, `%q` or quoted command lines). It reads the builds and their SHA-256, the options, bounds, `alpha`, `beta`, time control, hash, round cap and games at once from the notes, and the last score, Elo and SPRT lines from the log; an undecided run is read as undecided. Refused: a run that is not an SPRT, one that was interrupted or failed, a log that is missing or has no score or SPRT line, notes and a log that disagree on the outcome, a command without bounds. The row is pinned cell for cell (white space in the change folded, a `|` escaped, no path of the notes in it); a row is appended with line feeds only and once only; a dry run writes nothing; an undecided run is written as `undecided`, without emphasis, and said not to be an acceptance; a history file without the heading or with text after the table, a missing file, an empty change and a run that is not an SPRT are refused with status 2 and nothing written; the committed history file takes a row. Two guards against drift run the real `match.sh` and `match.ps1` (where bash and PowerShell exist) in SPRT mode against the stand-in `cutechess-cli` and read back the notes and log they left: the builds with the digests of their files, the bounds, the conditions, the score and the outcome. |
| `tests/perft.rs` | Perft acceptance tests; see "Perft" below for the positions and counts. |
| `tests/uci.rs` | The real binary over real pipes. A whole conversation (`uci`, `isready`, `ucinewgame`, `position`, `go depth 3`, `quit`) gives the same node count and move as a direct library search, the name arrives in UTF-8, ten options are listed, nothing follows `quit` and nothing is written to stderr; during `go infinite` the first depth arrives at once, `isready` is answered and no `bestmove` is sent until `stop`, which is obeyed within five seconds; `quit` ends a running search and the process; the end of input lets a search finish; twelve lines of bad input (one blank, two not valid UTF-8, one `go` with no usable limit) and a `stop` produce ten refusals, leave the position unchanged and leave the engine alive; with arguments the binary is not a UCI engine. Reading blocks until a line arrives, so an engine that did not flush every line would hang these tests. `go searchmoves` answers with one of the moves named and reports a word that is no legal move. `go mate 2` answers by itself with `score mate 2`; `go mate 1 movetime 300` ends by the move time without such a line; `go mate 3` from the start position searches on, answering `isready`, until `stop`. |
| `tests/uci_robustness.rs` | The UCI loop (`uci::run`, in the test process) under malformed and unexpected input from a seeded generator: eight streams of 400 lines, each from a generator drawn from the default seed's, of random bytes, scrambled words, `go` with odd limits, broken FENs, long random games and illegal moves, odd option values, commands cut short (some in the middle of a character), lines of up to about 300 KB and noise before a command. Nothing may panic on any thread, every `isready` is answered and every `go` gets exactly one `bestmove`; searches, refused FENs, illegal moves and unknown commands must each have occurred, so a generator that stopped producing them would fail. Each stream runs on a thread of its own with 180 seconds to finish: a hang fails the test and names the file the stream was kept in (every stream is written to Cargo's directory for test files, to be replayed with `vyvrael < file`); a second test shows that a stream that finishes passes and that its input is kept as sent. Beside each stream the test keeps a model of the position (a `position` command with anything wrong in it changes nothing, `ucinewgame` returns to the start) and checks every `bestmove`, in order, against the position of its `go`: a legal move of it, or `0000` when and only when it has none; more than 100 real moves and at least one `0000` are checked. A third test sends four streams of real sessions (the greeting, options with a fixed non-zero `Seed`, a new game, then a game's positions in order with a small search after each, by depth, nodes, move time or clock; `d`, `eval` and `isready` in between; the fifty-move counter at 99 and 120 and move number 65535 among the starts), an eighth of the lines damaged by a byte or two, one game in each stream of more than 1,200 plies. The engine is given a line at a time and each search with a small limit is waited for, others are stopped, so later depths, varied moves and weakened passes are reached. It checks the same things (no panic, every `isready` answered, every `bestmove` legal, more than 100 of them), that chances shown add up to 1000, and that statistics, a diagram, an evaluation, a varied move and a refused move list all occurred. Panics are counted on the engine's threads only, so one test failing does not fail another. A fourth test gives a 200-line malformed stream to the real binary over pipes, each pipe served by a thread of its own and the whole under the same watchdog (the process is killed if it is not done): it must leave with success, write nothing to standard error, answer every `isready`, end on `readyok`, and its moves are checked for legality like the others. |
| `tests/parser_fuzz.rs` | The four parsers behind the UCI loop, called directly with generated text from the default seed, no engine in the way. Commands (30,000 lines of noise, damaged real commands and numbers in the wrong place): no panic, the same reading twice, only white space is empty, an unknown line is echoed whole, and all three of unknown, malformed and understood come up. FENs (30,000, damaged by up to three bytes or pure noise): what is accepted is consistent, writes itself as a FEN that reads back as the same position, and every legal move can be played; thousands are accepted and more than ten thousand refused. Move text (6,000 per position on seven positions): the answer is a legal move of the position or none. Option values (30,000, set one after another): a refusal changes nothing and is one line, and every value stays in its declared range. |
| `tests/bench.rs` | The node count of `vyvrael bench` at its default depth, pinned (28,175 at depth 4 over seven positions); see "Bench". |
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

## Installing the external tools

Strength testing uses Cute Chess and Ordo, and playing on Lichess uses
lichess-bot. They are separate programs that are never part of this
repository. `scripts/install_tools.py` installs pinned versions of them for
the current user, outside the repository: all three on Linux, described
first, and Cute Chess and Ordo on Windows, described at the end of this
section. It also fetches the matches' opening file (`--openings`) and the
ladder's opponent engines (`--engines`), each in a section of its own
below. It needs only Python 3.9
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
  effect without `--force`. `--engine` is only written into `config.yml`
  when that file is first created; after that, edit `config.yml` (or use
  `--vyvrael`, below, which keeps the engine path fixed).
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
download and install nothing. **The maintainer has installed Cute Chess and
Ordo with it on the Linux machine (2026-11-21), and played a real match and
made a rating list with what it installed.** That confirms that this Cute
Chess AppImage runs `cutechess-cli` when given a leading `cli` argument.
Two details are still open: which exit status `cutechess-cli --version` and
`ordo -h` end with (the check accepts any status except 126 and 127, so it
cannot yet tell a program that starts and then fails from one that works),
and that lichess-bot reads its token from `LICHESS_BOT_TOKEN`. The
lichess-bot part has one known fault: see KI-11 in
[ROADMAP.md](ROADMAP.md).

#### Upgrading the bot's engine

```sh
python3 scripts/install_tools.py --vyvrael 0.1.0 --dry-run
python3 scripts/install_tools.py --vyvrael 0.1.0                    # avx2 file
python3 scripts/install_tools.py --vyvrael 0.1.0-alpha.2 --vyvrael-build plain
```

`--vyvrael VERSION` (Linux only) downloads the Linux binary of that GitHub
release, `vyvrael-VERSION-avx2` or with `--vyvrael-build plain`
`vyvrael-VERSION` (releases before 0.1.0 have only that one), together with
its `.sha256` file, over HTTPS only. It checks the digest and checks that the
binary answers `uci` with `id name Vyvræl VERSION`. Only then does it store
the file under its own name next to the engine path and point that path at
it with a relative symbolic link. The engine path is
`<prefix>/engines/vyvrael`, or the path given with `--engine`, so it matches
what the installer wrote into `config.yml`. As a result `config.yml` never
changes for an upgrade, and going back means pointing the link at the older
file, which is kept: `ln -sfn vyvrael-0.1.0-alpha.2 vyvrael`.

- A plain file already at the engine path (a binary copied there by hand)
  is copied to `vyvrael.before-link` before the link replaces it, so there
  is an engine at that path at every moment. With `--engine` pointing at a
  build output such as `target/release/vyvrael`, that file becomes the
  link and the release lands beside it in `target/release/`; give the path
  `config.yml` really uses.
- A failed download, a wrong digest or a wrong version leaves the link and
  the engine directory as they were and ends with an error message.
- A release that is still a draft cannot be downloaded this way (the error
  says so). Publish it first.
- `--vyvrael` with `--verify` is refused: the install already checks the
  binary.
- It never stops, starts or restarts the bot and never edits `config.yml`.
  Stop the bot first (after the pre-release PGN export, see
  [lichess-history.md](lichess-history.md)) and start it again afterwards.
- `--list` shows where the engine path points.

#### A release on the Windows workstation

```powershell
python scripts/install_tools.py --vyvrael 0.1.0 --dry-run
python scripts/install_tools.py --vyvrael 0.1.0                     # avx2 .exe
python scripts/install_tools.py --vyvrael 0.1.0 --vyvrael-build plain
```

On Windows there is no bot, so `--vyvrael VERSION` downloads the release's
Windows file, `vyvrael-VERSION-avx2.exe` or with `--vyvrael-build plain`
`vyvrael-VERSION.exe`, with its `.sha256` file, makes the same two checks
(the digest, and `id name Vyvræl VERSION` in reply to `uci`) and keeps it
in `<prefix>\engines\vyvrael-VERSION\`, beside the ladder's engines. Both
builds of one release share that folder. Nothing is linked: Windows lets few
users make symbolic links, and a match names the version it plays. A failed
download, a wrong digest or a wrong version keeps nothing. `--engine` is
refused there (it is lichess-bot's engine path), and so is `--verify` with
`--vyvrael`. `--list` names the releases kept. To use one in matches, add
it to the Cute Chess engine list.

The match and rating scripts that use these tools are described under
"Strength matches, SPRT and ratings" below.

### The opening file (Linux and Windows)

```sh
python3 scripts/install_tools.py --openings --dry-run
python3 scripts/install_tools.py --openings
```

`--openings` fetches `UHO_Lichess_4852_v1.epd.zip` from the Stockfish
project's books repository at one pinned commit (`scripts/openings.py`;
source, licence and digests in [DATA.md](DATA.md)), checks the zip and the
`.epd` inside it by SHA-256, and puts the `.epd` at
`<prefix>/openings/UHO_Lichess_4852_v1.epd`. The `.epd` is the very file of
Ladder Baselines 1 and 2, by SHA-256. The script prints the `OPENINGS=`
line for `scripts/local.env`. With no `--tool`, it installs nothing else;
it can be combined with `--tool`, `--engines` or `--verify` (with
`--force`, everything in the run is reinstalled). A failure to fetch it is
reported at the end and does not stop the rest of the run. A second run
fetches nothing unless `--force` is given, the pin has changed, or the file
or its record is gone or changed. The file is put in place only once both
digests match, by a rename next to it, so a failed run leaves no partial
file; if the swap fails (for example while a match has the file open on
Windows), the script says so and leaves the old file. `--list` shows
whether it is installed. Status: tested offline, and fetched for real once
on Linux on 2026-11-23 (both digests verified, file placed and recorded);
not yet run on Windows.

### The ladder's opponent engines (Linux and Windows)

```sh
python3 scripts/install_tools.py --engines --dry-run   # show what would be done
python3 scripts/install_tools.py --engines             # all of them
python3 scripts/install_tools.py --only "Vice 1.1"     # one, by its ladder name
python3 scripts/install_tools.py --verify              # check what is installed
python3 scripts/install_tools.py --list                # pins and what is installed
```

On Windows the commands are the same with `python` for `python3`.

`--engines` installs the opponents of Ladder Baseline 2, less Arasan
26.1-1800 (retired on 2026-11-22: too weak to tell Vyvræl builds apart,
and not a calibration opponent), and eleven engines new in Ladder Baseline
3 (Blunder 4.0.0, 5.0.0, 7.1.0 and 7.6.0, MinimalChess 0.4.1, 0.5 and 0.6,
Velvet 1.0.0, Inanis 1.0.1, Tantabus 1.0.2 and 2.0.0; official downloads,
added on 2026-11-22), under
`<prefix>/engines`, each in a directory of its own, outside the repository.
**Vyvræl itself is not part of the installer**: build it with
`cargo build --release` and keep released builds somewhere of your own.

- **What is installed** is fixed by `scripts/ladder_engines.py`; the table
  of engines, versions, sources, licences and methods is in
  [DEPENDENCIES.md](DEPENDENCIES.md). An entry such as "Stockfish 19-1500"
  is Stockfish 19 with `UCI_LimitStrength` on.
  - **Windows: 28 programs for the 40 names**, every one an official
    Windows binary (or, for Fruit 2.1, the distribution the maintainer
    chose). For Baseline 2's engines each is the very file of Ladder
    Baseline 2 by SHA-256 in the `pext` build, except Stash (see below). Nothing is compiled on Windows.
  - **Linux: 25 programs for 37 of the 40 names.** Three are left out:
    Orion 1.1, which exists for Windows only; Ethereal 12.75, whose source
    crashes when built with a current compiler; and Nostradamus 0.4, a
    Python program with a PyTorch model, which the installer does not set
    up.
- **Supported machines:** x86-64 Linux with AVX2 and BMI2 (any Intel
  processor from Haswell, 2013, or AMD from Excavator or Zen). No AVX-512
  build is used and nothing is compiled for "this machine"
  (`-march=native`); a compiled engine names `haswell` or `x86-64-v3`,
  except Vice 1.1, whose makefile names no architecture and so builds for
  plain x86-64.
- **Two builds, `avx2` (the default) and `pext`.** `--engine-build pext`,
  `VYVRAEL_ENGINE_BUILD=pext` or `ENGINE_BUILD=pext` in `scripts/local.env`
  installs the PEXT build of the engines that have one (Arasan, Stash,
  Weiss and Inanis, and Ethereal on Windows); the others have a single
  build. Inanis has no AVX2 build either: its default is its popcnt file,
  its `pext` build the popcnt_bmi2 file. Stash
  has no AVX2 build: its default is the popcnt file, and Ladder Baseline 2
  used its generic file, which neither build installs. Use `pext` on Intel from Haswell
  and AMD from Zen 3, where PEXT is fast; both of the maintainer's machines
  do. Changing the build reinstalls those engines on the next run. See
  [DEPENDENCIES.md](DEPENDENCIES.md).
- **Downloads and builds.** Every file is checked by SHA-256. On Windows
  all are downloads: bare programs or members of zip archives, and Rodent
  IV's 7-Zip archive, which Windows' own `tar` unpacks (Windows 10 or
  later). Rodent IV's site serves plain HTTP only, so that one download is
  over HTTP, which its pin explains; its digest is checked like any other.
  On Linux 19 engines are official binaries (eight of Baseline 2's and the eleven new in Baseline 3). Six are built from source
  (Princhess, Weiss, Rodent IV, Vice, Fruit, CT800) with the project's own
  build command and need `gcc`,
  `g++` and `make` (`sudo apt install build-essential`); Princhess also
  needs `cargo` and `clang`. **The script never runs sudo.** A tool that is
  missing is named and that engine is skipped; the others are still
  installed, and the run ends with an error listing what failed.
- **Safe to run again.** An engine already installed is skipped; `--force`
  reinstalls. An engine is put in place only when its download or build is
  complete, so a failure leaves nothing half-installed.
- **The engine list.** Afterwards the script writes
  `<prefix>/engines/engines.json` with every installed engine under its
  ladder name, Hash 64 and one thread where the engine has the option, the
  strength settings of the limited entries, and the rest of Ladder
  Baseline 2's settings under each engine's own option names: books and
  tablebases off and 30 ms of move overhead where the engine has the option
  (the list is in [DEPENDENCIES.md](DEPENDENCIES.md)). Put its path in
  `scripts/local.env` as `ENGINES_JSON` for `match.sh gauntlet`.
- **The check.** Last, each engine is started under each of its names and
  sent `uci`, its options, `isready` and `go depth 6`. The script prints
  `PASS` or `FAIL` for each name with what happened. Every wait has a time
  limit (20, 30 and 60 seconds), and an engine that will not quit is
  killed, so a stuck engine cannot hold the script up. `--verify` runs the
  check alone.

Status: covered by offline tests with made-up engines, and **run for real
once, by the maintainer on the Linux machine on 2026-11-21**:

| Result | Engines |
| --- | --- |
| Installed and passing the check (21 names) | Stockfish 19 and its six limited entries, Arasan 26.1 and its two, Viridithas 20, Reckless 0.9.0, Stash 37.0, Rodent IV and its two, Vice 1.1, Fruit 2.1, Rustic Alpha 1.5, 2.4 and 3.0.6 |
| Built, but failing the check | Ethereal 12.75: it crashed at the first `go`. The cause is in its source (data declared 64-byte aligned, allocated with `calloc`), which a current gcc no longer tolerates. It has since been taken out of the installer. |
| Build failed, since fixed | Weiss 2.0: a current gcc warns about sign comparisons, and the makefile's `-Werror` made that fatal. The pin now builds without `-Werror`, and the maintainer confirmed that it then installs and works. |
| Skipped, since installed | Princhess 0.22.0: `clang` was not installed. With it installed, the maintainer confirmed that Princhess builds and works. |

Every download matched its SHA-256, the shared Rustic Alpha archive was
fetched once, a failing engine did not stop the others, and the engine list
was written with the 22 names installed. All 13 engines the installer then
provided have therefore been installed on Linux at least once. What came
later has not been run for real yet: CT800 1.46 on Linux, the `avx2` builds
of Arasan, Stash and Weiss on Linux, and the whole Windows engine installer
(only its files' digests were checked, by downloading each once on
2026-11-21). The eleven engines of Ladder Baseline 3 have not been started
either: their files' digests, and those of the programs inside the zips,
were checked by downloading each once on 2026-11-22, and their options
were read from each project's source at its tag. A real match
and a rating run with the bash scripts followed (see "Strength matches,
SPRT and ratings"), with the `cutechess-cli` and Ordo that this script
installed.

On the Windows workstation the maintainer installed Cute Chess, Ordo and
the ladder's engines by hand (confirmed 2026-11-23). The Windows matches of
milestone 6 (the KI-8 gauntlet, the `UCI_Elo` calibration and its check,
and the soak test) ran with those, not with copies this script installed.

### Cute Chess and Ordo on Windows

```powershell
python scripts/install_tools.py --dry-run
python scripts/install_tools.py
```

On Windows the same script installs the same versions of Cute Chess and
Ordo from their official zip archives, checked by SHA-256, under the
install prefix. It needs no administrator rights and does not touch `PATH`:
it prints the `CUTECHESS_CLI` and `ORDO` lines to put in
`scripts/local.env`. `cutechess-cli.exe` needs the Microsoft Visual C++
runtime; if it is missing the script says so and names the installer that
ships in the archive, which asks for administrator rights and is never run
by the script. A Cute Chess that is already installed works just as well:
point `CUTECHESS_CLI` at its `cutechess-cli.exe`.

lichess-bot is not installed on Windows (it runs on the Linux machine
only). The ladder's engines are: see "The ladder's opponent engines (Linux
and Windows)" above. Status of the tools part: tested offline; the real
archives have not been fetched by the script. The Cute Chess and Ordo on
the Windows workstation were installed by hand by the maintainer
(confirmed 2026-11-23).

## Opponent ladder (Ladder Baseline 2)

The reference ladder is **Ladder Baseline 2 (current); Ladder Baseline 1 is
kept as history.**

- [`docs/testing/ladder-Baseline-2.md`](testing/ladder-Baseline-2.md)
  records a round robin between 30 engines of very different strength,
  played by the maintainer on 2026-11-21, with Ordo's ratings in
  [`ladder-Baseline-2-ratings.txt`](testing/ladder-Baseline-2-ratings.txt)
  and
  [`ladder-Baseline-2-ratings.csv`](testing/ladder-Baseline-2-ratings.csv).
  It is the reference list of opponents for measuring Vyvræl, from
  0.1.0-alpha.2 on; its section "Using the ladder for Vyvræl" names the
  core opponents of a gauntlet.
- [`docs/testing/ladder-Baseline-1.md`](testing/ladder-Baseline-1.md), with
  [`ladder-Baseline-1-ratings.txt`](testing/ladder-Baseline-1-ratings.txt)
  and
  [`ladder-Baseline-1-ratings.csv`](testing/ladder-Baseline-1-ratings.csv),
  is the first ladder: 25 engines, one round. It is kept as history;
  0.1.0-alpha.1 was measured against it.

**Vyvræl is in neither and has no rating in them.** The ladders are of other
engines only. Their Ordo ratings are relative to the anchor engine
(Stockfish 19 = 0) and describe only those engines. Vyvræl's own strength is
measured (milestone 6, known issue KI-8) by gauntlets against the lower
rungs, with a separate results file; it must not be inserted into a baseline
file or inferred from it. A first snapshot of 0.1.0-alpha.1, from before
milestone 6 and against Ladder Baseline 1, is in
[`results-0.1.0-alpha.1.md`](testing/results-0.1.0-alpha.1.md); the
milestone 6 measurement of 0.1.0-alpha.2, against Ladder Baseline 2, is in
[`results-0.1.0-alpha.2.md`](testing/results-0.1.0-alpha.2.md).

- **Fixed settings of Ladder Baseline 2:** round robin, two rounds of two
  games per pairing with sides swapped (four games per pairing, 1,740 games,
  116 per engine); time
  control 10+0.1, pondering off; openings from `UHO_Lichess_4852_v1.epd` in
  random order; Hash 64 MB and one thread per engine; `Move Overhead` 30 ms
  in every engine that has the option (Ladder Baseline 1 left each engine at
  its own default); draw adjudication from move 34 (10 moves within 20
  centipawns) and resign adjudication (3 moves, 700 centipawns, two-sided);
  no book and no tablebases. Keep these the same in any run that is to be
  compared with it.
- **Read the order, not the spread.** With 116 games per engine the error
  bars are about 180 to 290 points (250 to 500 in Ladder Baseline 1, with 48
  games each), close neighbours are not separated, and lopsided pairings
  stretch Ordo's scale to about 2,500 points (about 3,000 in Ladder Baseline
  1). The order of the rungs is the useful result; the Elo distances between
  them are not, and the magnitudes of the two ladders cannot be compared.
- The games themselves (`Baseline1.pgn`, `Baseline2.pgn`) are not in the
  repository: `*.pgn` is git-ignored. The SHA-256 in each baseline file is
  of the maintainer's local copy.

## Complete games in cutechess-cli (release check)

Before a release the engine must play complete games in `cutechess-cli`
with no illegal move, crash, stall or loss on time. Run them with the
match scripts (see "Strength matches, SPRT and ratings") after
`cargo build --release`: `h2h` with the build on both sides under two
names for self-play, and `gauntlet` against opponents of the reference
ladder, Ladder Baseline 2. For example, twenty games of self-play and eight
against each of two opponents:

```sh
scripts/linux/match.sh h2h \
  --test target/release/vyvrael --test-name "Vyvrael dev-1a2b3c4" \
  --base target/release/vyvrael --base-name "Vyvrael dev-1a2b3c4 (2)" \
  --rounds 10 --dry-run
scripts/linux/match.sh gauntlet \
  --test target/release/vyvrael --test-name "Vyvrael dev-1a2b3c4" \
  --opponent "Stockfish 19-1320" --opponent "Rodent IV-2000" \
  --rounds 4 --dry-run
```

0.1.0-alpha.1 was checked before the scripts existed, with a plain
`cutechess-cli` command at the settings of Ladder Baseline 1, run from the
repository root. It still works where the scripts cannot run. Give the
opening file's location in `BOOK`; the output folder `matches/` is
git-ignored.

```sh
cutechess-cli \
  -engine name="Vyvrael 0.1.0-a1" cmd=target/release/vyvrael proto=uci \
          option.Hash=64 option.Threads=1 \
  -engine name="Vyvrael 0.1.0-a1 (2)" cmd=target/release/vyvrael proto=uci \
          option.Hash=64 option.Threads=1 \
  -each tc=10+0.1 \
  -openings file="$BOOK" format=epd order=random \
  -games 2 -repeat -rounds 10 \
  -resign movecount=3 score=700 twosided=true \
  -draw movenumber=34 movecount=10 score=20 \
  -recover -concurrency 6 -srand 822286742 -ratinginterval 10 \
  -pgnout matches/selfplay.pgn
```

For games against other engines, add `-tournament gauntlet`, keep Vyvræl as
the first engine, and replace the second with one `-engine conf="<name>"`
per opponent, where the name is the one in Cute Chess's own `engines.json`
(the names of the reference ladder, Ladder Baseline 2). `-rounds 4` then
gives eight games against each. Every flag above was checked against
`cutechess-cli --help` of version 1.5.1.

How to read it: the PGN's last comment of each game says how it ended.
"mates", a draw by rule, and "wins by adjudication" are complete games.
"loses on time", "makes an illegal move", "disconnects" or "connection
stalls" are failures, and so is any game missing from the count.

**Result for 0.1.0-alpha.1**, run on 2026-11-20 on the Windows workstation
(Ryzen 7 5700X, six games at a time) with cutechess-cli 1.5.1, on the commit
that added `Move Overhead`:

| Run | Games | Mate | Draw by rule | Adjudicated | Illegal moves, crashes, stalls, time losses |
| --- | --- | --- | --- | --- | --- |
| Self-play | 20 | 2 | 2 | 16 | 0 |
| Gauntlet against Arasan 26.1-1500, Stockfish 19-1320, Rodent IV-1600, Vice 1.1 and Rustic Alpha 1.5 | 40 | 6 | 2 | 32 | 0 |

The longest game ran 238 plies. In the gauntlet log `cutechess-cli` warned
four times, "EngineProcess: Destroyed while process is still running"; each
time between games, after one against Arasan, and no game was affected. The
games were not kept in the repository.

**Result for 0.1.0-alpha.2**, run on 2026-11-22 on the Windows workstation
(six games at a time) with cutechess-cli 1.5.1 and the release's Windows
binary, built from commit `105d8d5`. The gauntlet used `match.ps1` against
the six core opponents of Ladder Baseline 2; the self-play gave both sides
`Variety` 10 with `Seed` 0, as the Lichess bot will play.

| Run | Games | Mate | Draw by rule | Adjudicated | Illegal moves, crashes, stalls, time losses |
| --- | --- | --- | --- | --- | --- |
| Gauntlet against Rodent IV-2000, Stockfish 19-1800, Stockfish 19-1320, CT800 1.46-1800, CT800 1.46-1500 and Nostradamus 0.4 | 48 | 20 | 4 | 24 | 0 |
| Self-play with `Variety` 10 | 20 | 3 | 3 | 14 | 0 |

`cutechess-cli` printed no warning in either log. In the self-play none of
the ten opening pairs was played move for move twice: eight diverged at
the first move after the opening position, the others at moves 3 and 7.
Without `Variety`, three of ten pairs repeated in the self-play of
0.1.0-alpha.1. The games were not kept in the repository.

**Result for 0.1.0**, run on 2026-11-22 (19:57 to 20:54 UTC) on the
Windows workstation (six games at a time) with cutechess-cli 1.5.1 and an
`avx2` build of the release's source (commit `b048c21`; nothing after
it on `release/0.1.0` touches the engine's source). It is the 1,000-game
gauntlet of the release's measurement, against ten rungs of Ladder
Baseline 2 ([results-0.1.0.md](testing/results-0.1.0.md)), so the check
and the rating come from the same games.

| Run | Games | Mate | Draw by rule | Adjudicated | Illegal moves, crashes, stalls, time losses |
| --- | --- | --- | --- | --- | --- |
| Gauntlet against ten rungs of Ladder Baseline 2 | 1,000 | 295 | 51 | 654 | 0 |

`cutechess-cli` ended one opponent's process by force between games
(CT800 1.46-1800), with no result affected.

**The alpha checks are checks that games complete, not measurements of
strength.** Forty or forty-eight games against five or six opponents say
nothing reliable about a rating; the measurements are separate (see
[results-0.1.0-alpha.2.md](testing/results-0.1.0-alpha.2.md)). From 0.1.0
the release check uses the games of the release's measurement
([results-0.1.0.md](testing/results-0.1.0.md)).

## Strength matches, SPRT and ratings

Search, evaluation and network changes are accepted only when a sequential
probability ratio test (SPRT) against the previous version shows a gain.
The result (bounds, games, LLR, outcome) is recorded in
[`testing/sprt-history.md`](testing/sprt-history.md) by
`scripts/record_sprt.py` and quoted in the commit message of the change
(see "Recording an SPRT result"). That is why search is split (see
"Decisions for milestones 4 to 6" in [ROADMAP.md](ROADMAP.md)): the search
core is checked by correctness tests and fixed-depth node counts, and each
search heuristic of milestone 7 gets its own SPRT.

Four scripts do the work. The two for each system behave alike; only the
spelling of the options differs (`--rounds` in bash, `-Rounds` in
PowerShell), and PowerShell takes several opponents or options as one
comma- or semicolon-separated list where bash repeats the flag (see
"Pitfalls").

| Task | Linux | Windows |
| --- | --- | --- |
| Run a match | `scripts/linux/match.sh` | `scripts/windows/match.ps1` |
| Make a rating list | `scripts/linux/rate.sh` | `scripts/windows/rate.ps1` |

They need `cutechess-cli` and Ordo, which are separate programs (see
"Installing the external tools"). In gauntlet and round-robin mode `match.sh` also needs
`python3` (or the program named in `VYVRAEL_PYTHON`) to read the engine list.
Each script reads the tool's own help before using it and stops with a clear
message if the tool, or an option it needs, is missing. `--help` (`-Help`)
prints every option. **Start every run with `--dry-run` (`-DryRun`)**: it
prints the exact command and runs nothing.

**Status:** the scripts are covered by offline tests with stand-in tools.
On the Windows workstation `match.ps1` and `rate.ps1` have been run for
real in every mode (see "Smoke test of the scripts", 2026-11-22), and
`match.ps1` played the KI-8 gauntlet, the `UCI_Elo` calibration and the
soak test. The maintainer has also run the bash scripts on the
Linux machine (2026-11-21): a dry run, a real gauntlet with `match.sh` and
a rating with `rate.sh` all completed, after a fault in `match.sh`'s name
check that the first attempt met was fixed.

### Settings that differ between machines

Paths and machine sizes are not in the repository. Copy
`scripts/local.env.example` to `scripts/local.env` (git-ignored) and set
what applies: where `cutechess-cli` and Ordo are if not on `PATH`, the
opening file (`install_tools.py --openings` fetches it and prints the
line), the engine list, the output folder, the number of games at
once, whether to run at low priority, and the physical core count where
the scripts cannot detect it (`PHYSICAL_CORES`). Each setting can also be an
environment variable with the prefix `VYVRAEL_`, and most an argument. The
order is argument, environment, local file, built-in default. Set
`VYVRAEL_LOCAL_ENV` to read another file in place of `scripts/local.env`.

Games at once (`CONCURRENCY`) defaults to the physical cores minus two (at
least one) and may never exceed the physical cores: engines sharing a core
steal each other's time and lose games on the clock. On the maintainer's
machines it is 6 on the Windows workstation and 3, at low priority, on the
Linux machine.

### The four kinds of match

Build names are plain ASCII, for example `Vyvrael 0.1.0-a1` or
`Vyvrael dev-1a2b3c4`. A round is two games: one opening, each side once.

**SPRT**, to decide whether a change is a gain:

```sh
scripts/linux/match.sh sprt \
  --test target/release/vyvrael --test-name "Vyvrael dev-1a2b3c4" \
  --base ../builds/vyvrael-0.1.0-a1 --base-name "Vyvrael 0.1.0-a1" \
  --elo0 0 --elo1 5 --alpha 0.05 --beta 0.05 --rounds 5000 --dry-run
```

The bounds, `alpha`, `beta` and the round cap are required; the scripts
have no default for them, so a test's terms are always written down. Bounds
to start from:

| Purpose | `elo0` | `elo1` | Use it when |
| --- | --- | --- | --- |
| A gain | 0 | 5 | A new heuristic or evaluation term: accept only if it is better. |
| No loss | -5 | 0 | A simplification or refactoring that should play the same. |
| A large gain, quickly | 0 | 10 | Early on, when changes are worth tens of Elo and games are scarce. |

`alpha` and `beta` of 0.05 each allow a 5% chance of accepting a change
that is not a gain and a 5% chance of rejecting one that is. The script
ends by naming the outcome: **H1 accepted** (keep the change), **H0
accepted** (reject it), or **undecided** when the round cap came first,
which is not an acceptance. The LLR runs from the lower to the upper bound
(about -2.94 to 2.94 with those values); the test stops when it crosses
one.

**Gauntlet**, to measure a build against the ladder:

```sh
scripts/linux/match.sh gauntlet \
  --test target/release/vyvrael --test-name "Vyvrael 0.1.0-a1" \
  --opponent "Stockfish 19-1320" --opponent "Vice 1.1" \
  --opponent "Rustic Alpha 1.5" --rounds 20 --dry-run
```

Opponents are named exactly as in the engine list (`ENGINES_JSON`, in Cute
Chess's `engines.json` format), which carries each one's file and options,
`UCI_LimitStrength` and `UCI_Elo` for the limited entries included. In
PowerShell: `-Opponent "Stockfish 19-1320","Vice 1.1"`.

**Head-to-head**, a fixed number of rounds between two builds: as `sprt`
without the four SPRT options, with the mode `h2h`.

**Round robin**, for a ladder baseline: every engine against every other.

```sh
scripts/linux/match.sh roundrobin \
  --build "Vyvrael 0.1.0=../builds/vyvrael-0.1.0" \
  --build "Vyvrael 0.2.0=../builds/vyvrael-0.2.0" \
  --rounds 2 --seed 20261201 --deterministic --label baseline3 --dry-run
```

```powershell
scripts\windows\match.ps1 roundrobin `
  -Build "Vyvrael 0.1.0=C:\builds\vyvrael-0.1.0.exe","Vyvrael 0.2.0=C:\builds\vyvrael-0.2.0.exe" `
  -Rounds 2 -Seed 20261201 -Deterministic -Label baseline3 -DryRun
```

With no `--opponent` (`-Opponent`), every engine in the engine list plays,
in the list's order; with it, only those named. `--build NAME=PATH`
(`-Build`, repeatable) adds a Vyvræl build by its path, with the same
settings a test build gets (one thread, and `Seed` and `Variety` fixed by
`--deterministic`); builds play first in the command. Names must be
unique, and at least two engines must play. The settings are those of
every match (below), which are Ladder Baseline 2's: `--rounds 2` gives
each engine two games against every other engine in each of two rounds,
116 games each with 30 engines, as in Baseline 2. The files are named
`<date>-roundrobin[-label]`, and the run notes list every engine with its
SHA-256, with a copy of the engine list beside them. `--test`,
`--test-name`, `--base`, `--base-name`, `--test-option`, `--base-option`
and the SPRT options belong to the other modes and are refused here, as
`--build` is refused there. Rate the PGN with `rate.sh` / `rate.ps1` and
the baseline's Ordo command (anchor Stockfish 19).

Ladder Baseline 3, once milestone 7 and 0.2.0 are ready (see
[ROADMAP.md](ROADMAP.md)): the installer's engine list, which already
leaves out Arasan 26.1-1800 and adds Baseline 3's new engines, plus
`--build` for 0.1.0-alpha.1, 0.1.0-alpha.2, 0.1.0 and 0.2.0, `--rounds 2`
and a fixed `--seed`. On Windows every engine of the list installs; on
Linux Orion 1.1, Ethereal 12.75 and Nostradamus 0.4 do not, so a Linux run
would not be the same baseline.

### What every match uses, and why

| Setting | Value | Why |
| --- | --- | --- |
| Time control | 10+0.1 | As Ladder Baselines 1 and 2; short enough for thousands of games. |
| Hash, threads | 64 MB, 1 | As the baselines; one thread keeps results reproducible in kind. |
| Pondering | off | Engines must not use each other's time. |
| Openings | `UHO_Lichess_4852_v1.epd`, random order | Unbalanced human openings reduce draws between unequal engines. |
| Games per opening | 2, colours swapped | Neither side gets the better half of an opening. |
| Resign | 3 moves at 700 centipawns, both engines agreeing | Ends decided games early. |
| Draw | from move 34, 10 moves within 20 centipawns | Ends dead games early. |
| `-recover` | on | One crashed engine does not end a long run. |

`--tc` and `--hash` can change the first two. The opening file is the one
given with the match script's `--openings PATH` (or `OPENINGS`; the
table's is the one the ladder used, which `install_tools.py --openings`
fetches, see "The opening file" above), and its
order is random unless `--seed` fixes it. A build's own UCI options come
from `--test-option` and `--base-option` (an opponent's from the engine
list). The rest is fixed in the scripts. **Keep everything the same between two runs you mean to compare**,
on the same machine with the same number of games at once: results from
different machines or settings are not comparable.

Each run leaves three files in the output folder (`matches/`, git-ignored),
named `<date>-<mode>-<build>`: the `.pgn`, a `.log` of the progress, and
`.notes.txt` with the exact command, each engine's name, file and SHA-256, the
`cutechess-cli` version, the start and end times and the outcome. The notes
also give the repository the script ran from (its commit, and whether the
working tree was clean or dirty, read with `git --no-optional-locks`, which
writes nothing in the repository, before the run writes any file; set
`VYVRAEL_GIT` to use another `git`), and at the end the number of games in the
PGN and the PGN's SHA-256. `VYVRAEL_MATCH_DATE` replaces the date in the file
names. A gauntlet or a round robin also leaves a fourth file, `.engines.json`: a copy of the
engine list it read, since that list carries the opponents' options and may
change later. Keep the notes (and that copy) with any result you record. A
script refuses to add games to a PGN that already exists; use `--label` for a
second run on one day.

### Rating lists with Ordo

```sh
scripts/linux/rate.sh matches/2026-11-21-gauntlet-Vyvrael-0.1.0-a1.pgn
```

writes `...-ratings.txt` and `...-ratings.csv` beside the PGN, with the
baseline's settings: the anchor "Stockfish 19" fixed at 0, the white
advantage estimated from the games, and 500 simulations for the error
bars. Several PGN files may be given. The anchor must have played in the
games; for a gauntlet without Stockfish 19 give another, for example
`--anchor "Vice 1.1" --anchor-rating -1436.2` to place it where the
reference ladder, Ladder Baseline 2, has it.

### Reading an SPRT result

While an SPRT runs, `cutechess-cli` prints every ten games, and at the
end, lines like these (the match script keeps them in the `.log`):

```text
Score of Vyvrael dev-1a2b3c4 vs Vyvrael 0.1.0-a2: 412 - 371 - 217  [0.521] 1000
Elo difference: 14.3 +/- 19.0, LOS: 92.9 %, DrawRatio: 21.7 %
SPRT: llr 1.83 (62.3%), lbound -2.94, ubound 2.94
```

- **`Score of A vs B: W - L - D [p] N`**: A's wins, losses and draws against
  B, A's score as a fraction (0.521 is 52.1%) and the games played.
- **`Elo difference: x +/- y`**: the difference that score implies, with a
  95% margin. It is a running estimate, not the test's answer: early on
  the margin is wide, and an SPRT does not stop when the margin excludes 0.
  `LOS` is the likelihood that A is stronger at all; `DrawRatio` the share
  of draws.
- **`SPRT: llr L (P%), lbound a, ubound b`** is the test itself. LLR, the
  log-likelihood ratio, weighs the evidence for H1 (the change gains at
  least `elo1`) against H0 (it gains no more than `elo0`). It drifts up
  when A does well, down when it does not, and the percentage says how far
  it is towards the bound it is heading for. The bounds come from `alpha`
  and `beta`: with 0.05 each, about ±2.94.
- **The test ends when the LLR crosses a bound**, and the last `SPRT:` line
  says which: `- H1 was accepted` (keep the change) or `- H0 was accepted`
  (reject it). The script repeats it as "SPRT outcome". If the round cap
  comes first the result is **undecided**: not an acceptance, whatever the
  Elo line says.
- **What an accepted H1 means:** with the chosen error rates, the change
  is better by at least `elo0`, against this opponent, at this time
  control, on this machine. It does not give the size of the gain: the Elo
  line at the end is a rough estimate of that, with its margin.
- Record the result as the next section says: a row in the history file,
  and its one-line summary in the commit message of the change
  (AGENTS.md).

### Recording an SPRT result

```sh
python3 scripts/record_sprt.py <run>.notes.txt --change "MVV-LVA in the main search" --dry-run
```

(`python` on Windows.) The script reads the run's notes and the log beside
them and appends one row to
[`testing/sprt-history.md`](testing/sprt-history.md): the date, the change
in your words, the two builds with the start of their SHA-256 and any
options they were given, the bounds, `alpha` and `beta`, the time control,
hash size, games at once and round cap, the last score and Elo line, the
final LLR with its bounds, the outcome, and the start of the PGN's SHA-256,
which identifies the run. It then prints the row and a one-line summary to
paste into the commit message. So a result is written down the same way
every time, from the files, and never retyped.

- `--dry-run` shows the row and writes nothing. `--log FILE` names the
  log if it is not beside the notes; `--file FILE` another history file.
- **A run is recorded once:** a second call for the same run (the same
  PGN) changes nothing and says so.
- **Every outcome is recorded,** not only the passes: H0 accepted and
  undecided get their rows too. An undecided run is written as
  `undecided` and the script says that it is not an acceptance.
- **What is not a finished SPRT is refused,** with status 2 and nothing
  written: another kind of match, a run that was interrupted or failed, a
  log without a score or an SPRT line, notes and a log that disagree
  about the outcome.
- The games and the score are those of the last line `cutechess-cli`
  printed. With several games at once a few more may be in the PGN, since
  games still running when the test stopped are finished and saved.
- Nothing of the notes' paths goes into the row. The row says nothing
  about the machine: the history file states that its rows are from the
  Windows workstation.
- The script never creates or repairs the history file: the file must
  hold the table's heading and end with the table.

### Reading Ordo's output

`rate.sh` and `rate.ps1` write Ordo's table as text and as CSV; the
ladders' files under `docs/testing/` are examples:

```text
   # PLAYER                :   RATING  ERROR  POINTS  PLAYED   (%)
   1 Stockfish 19          :      0.0   ----   111.5     116    96
  13 Stockfish 19-2400     :  -1280.3  236.2    64.0     116    55
White advantage = 4.03 +/- 3.16
Draw rate (equal opponents) = 50.00 % +/- 0.00
```

- **`RATING`** is relative to the anchor (`-A`), whose rating is fixed
  (`-a`, 0 by default): Stockfish 19 is 0 and everything weaker is negative.
  The anchor has no error (`----`) because it is fixed, not measured.
- **`ERROR`** is the margin, from Ordo's simulations (`-s 500`): roughly the
  95% range either side of the rating. It changes a little between runs of
  Ordo; the ratings do not. Two engines whose ranges overlap are not
  separated.
- **`POINTS`, `PLAYED` and `(%)`** are the raw results: a win is one point,
  a draw half.
- **`White advantage`** is estimated from the games (`-W`), so that the
  colours do not bias the ratings. The **`Draw rate (equal opponents)`**
  line shows 50% when Ordo did not estimate it; it is not a measurement.
- **The order is the result.** The scripts print it again on its own.
- **The distances are not.** When one engine wins nearly every game the
  scale stretches: Ladder Baseline 2 spans about 2,500 points, and Ladder
  Baseline 1 about 3,000, between engines that are not that far apart.
- **Ratings belong to one Ordo run.** They exist only relative to the other
  players and games of that run: a rating from one list cannot be put next
  to a rating from another, unless both runs share the anchor and enough of
  the same games. That is why a Vyvræl build is rated **together with
  Baseline 2's games** in one run.
- **Ratings are not CCRL, FIDE or Lichess ratings**, and a nominal
  `UCI_Elo` in an engine's name is a label, not its strength.

### What a sound Elo comparison needs

Two builds, or a build and a ladder, can only be compared when everything
but the engine is the same:

- **The same machine**, and the same number of games at once. A faster
  machine searches deeper at the same time control, and that changes the
  result.
- **The same settings:** time control, hash, threads, `Move Overhead`, the
  opening file and its order, adjudication. The scripts fix threads and
  adjudication; the time control, hash, opening file and any option given
  with `--test-option` or `--base-option` (`Move Overhead` among them)
  come from the command, so keep them as the ladder's games had them (see
  "What every match uses"). Games played with other settings (Ladder Baseline 1's own `Move
  Overhead`, for example) are not merged into one rating.
- **The same opponents**, or enough games against shared ones: a build that
  only met weak engines and one that only met strong ones cannot be
  ranked against each other.
- **Enough games.** The margin shrinks with the square root of the games.
  For evenly matched engines the 95% margin is about ±140 Elo at 20
  games, ±90 at 50, ±45 at 200 and ±23 at 800 (20 games of self-play gave
  ±143.7 in `cutechess-cli`); when the results are lopsided it is far
  wider, which is why the ladder's ratings carry ±180 to ±290 after 116
  games each. A difference smaller
  than the margins is no difference. For deciding a change, use an SPRT,
  which runs as long as the evidence needs.
- **Full-strength builds with `Variety` 0**, unless the point is to measure
  a limited setting. With `UCI_LimitStrength`, a fixed `Seed` makes the run
  repeatable.
- **A quiet machine.** Other work steals time from the engines and shows up
  as time losses and weaker play; run long matches on an otherwise idle
  machine.

### The scale of `UCI_Elo`

Decided by the maintainer on 2026-11-22, in this wording: `UCI_Elo`'s
numbers are ratings on the scale of the reference ladder, Ladder Baseline
2, as computed by Ordo with Stockfish 19 as the anchor, shifted by a fixed
3,500 points so that Stockfish 19 is 3,500 and every setting is a positive
number. They are not Lichess, CCRL or FIDE ratings and cannot be converted
to them. The scale is stretched by lopsided results, most visibly at the
low end: the limited engines of the ladder rate far below the numbers in
their names (Stockfish 19-1320 is about 1,530 on this scale). A setting of
1,200 means about as strong as an engine rated 1,200 on this ladder, not a
1,200 player.

In practice: a Vyvræl build at a given `UCI_Elo` is rated in one Ordo run
with Baseline 2's games, anchored at Stockfish 19 = 0 as always, and 3,500
is added to its rating. Some points of the ladder on this scale:

| Engine (Ladder Baseline 2) | Ordo rating | On the `UCI_Elo` scale |
| --- | --: | --: |
| Stockfish 19 | 0.0 | 3,500 |
| Stockfish 19-1800 | -1,493.5 | 2,007 |
| Rodent IV-2000 | -1,757.3 | 1,743 |
| Stockfish 19-1320 | -1,969.6 | 1,530 |
| CT800 1.46-1800 | -2,139.1 | 1,361 |
| CT800 1.46-1200 | -2,486.5 | 1,014 |

The settings were labels until the calibration below; since its refit
`UCI_Elo` (1110 to 1630) is on this scale.

### Calibration of `UCI_Elo` (0.1.0-alpha.2, 2026-11-22)

**Provisional:** it holds for the build measured and is measured again
after milestone 7 and after queued task 2.

- **Build:** the 0.1.0-alpha.2 release binary (SHA-256
  `1b0d518f26fcbb876f747449422c9d32ea249db0223f3ddec319ac4297c81ad3`), then
  the same as the current build of milestone 6, with
  `UCI_LimitStrength=true` and six settings of the uncalibrated `UCI_Elo`.
  `-Deterministic` fixed `Seed` at 822286742, so each game can be replayed.
- **Games, played by the maintainer on the Windows workstation** with
  `match.ps1` from the clean commit `c16c466`, 06:49 to 07:55 UTC:
  - one gauntlet per setting, 20 rounds (40 games) against each of
    CT800 1.46-1200, Arasan 26.1-1500, CT800 1.46-1500, Nostradamus 0.4,
    Stockfish 19-1320, Stockfish 19-1500, Rodent IV-2000 and
    `Vyvrael 0.1.0-a2` at full strength (`-Seed 20261123`, the same
    openings for every setting); 320 games each;
  - five head-to-head runs of 40 games between neighbouring settings
    (`-Seed 20261124`), so that Ordo can rate the settings even if one of
    them lost every game to the ladder (none did);
  - Baseline 2's fixed settings otherwise (10+0.1, Hash 64, one thread,
    `UHO_Lichess_4852_v1.epd`). The seven ladder entries were the same as
    Baseline 2's copy of the engine list, checked entry by entry.
- **Rating:** one Ordo run (`rate.ps1`, Ordo 1.2.6, `-a 0 -A "Stockfish 19"
  -W -s 500`) over `Baseline2.pgn`, the KI-8 gauntlet (which rates
  `Vyvrael 0.1.0-a2` with 1,000 games) and the eleven calibration files,
  4,860 games in all. Ordo's table:
  [`calibration-0.1.0-alpha.2-ratings.txt`](testing/calibration-0.1.0-alpha.2-ratings.txt),
  copied unchanged.
- **Failures:** no illegal move, crash, stall or time loss by any engine.
  `cutechess-cli` ended three engine processes itself, once each for
  Vyvræl L1150, CT800 1.46-1500 and Stockfish 19-1500, between games; no
  result was affected. So the message seen once in the KI-8 gauntlet is not
  particular to Vyvræl.
- **SHA-256 of the PGNs** (not committed; kept locally with their notes):

  | File (`2026-11-22-...-cal.pgn`) | Games | SHA-256 |
  | --- | --: | --- |
  | `gauntlet-Vyvrael-0.1.0-a2-L400` | 320 | `1d33066a6a37c7a5ed741048742267f9021ae0b924a946f14867abe9efb409ac` |
  | `gauntlet-Vyvrael-0.1.0-a2-L750` | 320 | `70728859fe0a721eebf92a7e742f46c1ab53935e79ac181a32ba38b4dbb3cb2c` |
  | `gauntlet-Vyvrael-0.1.0-a2-L1050` | 320 | `d01004244e56f06d406dcac3da56f2ebd0821faea6c3bb555ac0be7072b11a77` |
  | `gauntlet-Vyvrael-0.1.0-a2-L1150` | 320 | `52485058be958bbb742ad43e05226b92011a0eace5d5de6c7014b8ece16aa526` |
  | `gauntlet-Vyvrael-0.1.0-a2-L1450` | 320 | `b5d3d2119214e93a63194cb8f8914998d4a5f54eddd37e1b5b69abc03703ddcf` |
  | `gauntlet-Vyvrael-0.1.0-a2-L1800` | 320 | `25cb3651432caa62620c97d62634cb9d79611c402df611e3cd407de912934ee1` |
  | `h2h-Vyvrael-0.1.0-a2-L400` (against L750) | 40 | `e5d298df4dfe1e64aa5090669388d3001910be7fad1f23603863c8e7ce357a32` |
  | `h2h-Vyvrael-0.1.0-a2-L750` (against L1050) | 40 | `adeca5956f7ecbcdd793fa5f0cc17a4a44194ad772af531450dc700d2e237722` |
  | `h2h-Vyvrael-0.1.0-a2-L1050` (against L1150) | 40 | `1e069780c8dc23586a75b6b6c1fad7bbf6dea77a2b6d272fea6527d05fb18820` |
  | `h2h-Vyvrael-0.1.0-a2-L1150` (against L1450) | 40 | `af42c6fe8782caade4259553b140de637e3d765a6693db74a12bc8fcdccd64f1` |
  | `h2h-Vyvrael-0.1.0-a2-L1450` (against L1800) | 40 | `b0a6f5d134ad4d1eb29fcd7f3bf429a134a23d58012ee6d723702dcbe315dcb8` |

**Result.** What each old setting really plays at, on the `UCI_Elo`
scale (Ordo rating + 3,500). The error against the anchor is about ±245
for every row; what matters for the fit is the order and the gaps, which
the head-to-head runs confirm (each higher setting won its link, 25.0 to
35.0 of 40).

| Old setting | Plies | Largest misjudgement (cp) | Ordo rating | On the scale |
| --- | --: | --: | --: | --: |
| 400 | 2 | 300 | -2,393.8 | 1,106 |
| 750 | 2 | 230 | -2,351.0 | 1,149 |
| 1050 | 2 | 171 | -2,282.7 | 1,217 |
| 1150 | 3 | 151 | -2,146.4 | 1,354 |
| 1450 | 3 | 90 | -2,095.4 | 1,405 |
| 1800 | 3 | 20 | -1,870.4 | 1,630 |
| Full strength | | | -1,660.6 | 1,839 |

Scores against the ladder, out of 40 for each opponent:

| Setting | CT800 1.46-1200 | Arasan 26.1-1500 | CT800 1.46-1500 | Nostradamus 0.4 | Stockfish 19-1320 | Stockfish 19-1500 | Rodent IV-2000 | Vyvrael 0.1.0-a2 |
| --- | --: | --: | --: | --: | --: | --: | --: | --: |
| 400 | 32.0 | 35.0 | 12.5 | 13.5 | 9.0 | 2.0 | 0.0 | 1.0 |
| 750 | 40.0 | 38.0 | 16.0 | 12.5 | 6.0 | 0.0 | 0.0 | 0.0 |
| 1050 | 39.0 | 36.0 | 20.5 | 18.5 | 8.0 | 5.0 | 0.0 | 0.0 |
| 1150 | 39.5 | 39.0 | 29.0 | 17.5 | 17.0 | 10.0 | 0.5 | 0.0 |
| 1450 | 40.0 | 39.0 | 33.0 | 23.5 | 12.0 | 10.5 | 3.5 | 1.0 |
| 1800 | 40.0 | 39.0 | 37.5 | 32.0 | 33.0 | 15.0 | 11.5 | 9.0 |

What it shows:

- **The settings span only about 525 points,** from 1,106 to 1,630. The
  misjudgement costs less than it was meant to, and the weakest setting
  still beats the two lowest rungs of the ladder (CT800 1.46-1200 and
  Arasan 26.1-1500) in most games. Ratings below about 1,100 cannot be
  played with the weakening as it is; reaching them needs a change to the
  weakening itself, which is not part of this calibration.
- **The step from two to three plies is worth about 137 points** (1,217 at
  setting 1050 to 1,354 at 1150), more than the whole span of the
  two-ply settings (111 points).
- **The misjudgement matters most when it is small:** at three plies,
  going from 90 to 20 centipawns gains 225 points; from 300 to 171 at two
  plies gains 111.
- **The ladder's lowest rungs move down** when rated with these games
  (Arasan 26.1-1500 from -2,431.3 in Baseline 2 alone to -2,712.9,
  CT800 1.46-1200 from -2,486.5 to -2,780.4), because the limited settings beat them far more
  often than Baseline 2's engines suggested. As with KI-8, compare within
  this one table.

**The first refit (`a874077`).** `search::Weakening::for_elo` mapped
`UCI_Elo` through the six measured points, their ratings rounded to tens
(1110, 1150, 1220, 1350, 1410, 1630), interpolating the misjudgement
between points of the same depth. The range became 1110 to 1630. Between
1220 and 1350 there was no measurement: the line through the last two
two-ply points was extended to 1349 (62 centipawns), so that stretch
stayed at two plies; the second refit below replaces that. The default is
1360 (three plies, 141 centipawns), exactly the weakening the old default
(1200) gave. The current table is in `docs/UCI.md`, "Limited strength".
Full-strength play is unchanged: the bench node count is the same before
and after.

**Check of the refit.** The refitted build (commit `a874077`,
`vyvrael-dev-a874077.exe`, SHA-256
`8682cac2711dc703ca9ecc19f1b9499a4b0c4a4c69be1fe5e3ef9e573ce26019`) was
played at three new settings, one in each kind of stretch, against the
same eight opponents, 40 games each (`-Seed 20261125`). The maintainer ran
it from the clean commit `7a36db0`, whose engine code is that of
`a874077`, 11:56 to 12:25 UTC. It was rated in one Ordo run with
Baseline 2, the KI-8 gauntlet and the calibration's games (5,820 games;
table:
[`calibration-check-a874077-ratings.txt`](testing/calibration-check-a874077-ratings.txt),
copied unchanged). No illegal move, crash, stall or time loss;
`cutechess-cli` ended two Vyvræl processes itself between games, with no
result affected.

| `UCI_Elo` | Stretch | Plies, misjudgement (cp) | Ordo rating | On the scale | Off target |
| --: | --- | --- | --: | --: | --: |
| 1180 | two plies, interpolated | 2, 205 | -2,348.4 | 1,152 | -28 |
| 1290 | two plies, extrapolated | 2, 112 | -2,189.2 | 1,311 | +21 |
| 1520 | three plies, interpolated | 3, 55 | -2,020.4 | 1,480 | -40 |

PGN SHA-256: E1180
`93a63e54f6777fff3e39e0d251788e552d75c7efa3cf52d9fcfaf4c20fc1d896`,
E1290 `c98c99bf7b8bbcdb3941e54c472ff6207763f3a087e29140e08042c8a85fff08`,
E1520 `eb3aaebe99d4646942d0322ec8c316413e18004ab6dfd04a3d83ec2c2c21d939`
(320 games each, not committed).

Every setting landed within 40 points of its number, in the right order,
which is about the error of 320 games. The merged run also moves the six
calibration settings 9 to 27 points down (L400 by 26, L1800 by 9; L400 at
1,080 and L1800 at 1,621 in this table), so part of each offset is the
scale shifting, not the fit. Against this table's own calibration
points, 1180 lands exactly where the interpolation puts it (1,153), and
1520 is 25 points below the midpoint of its neighbours, whose own shift
there is only 9 to 17 points.

**The extrapolated stretch was wrong, and is replaced.** Against its
measured neighbours in the same table, 1290 (two plies, 112 cp) plays
118 points above L1050 (two plies, 171 cp) and only 23 below L1150 (three
plies, 151 cp): two plies gains about 2 points for every centipawn of
misjudgement taken away. Continuing the line down to 62 cp at 1349, as the
first refit did, would most likely have made 1349 play stronger than 1350
to 1410, so a higher setting would have played weaker. This was found by
the end-of-milestone review. The second refit (below) drops the
extrapolated point.

**The second refit.** The check's 1290 setting (two plies, 112 cp) is
itself a measured weakening. Placed on the calibration's scale by its
distance from its neighbours in the check table (22.6 below L1150, 118.0
above L1050, which give 1,331 and 1,335 on the calibration's scale), it is
about 1,333, rounded to 1330. `for_elo` now maps `UCI_Elo` through seven
measured points (1110, 1150, 1220, 1330, 1350, 1410, 1630) with nothing
extrapolated. Between 1330 and 1350 the depth changes, so 1331 to 1349
play as 1330: up to 20 points weak for the number, but never stronger than
a higher setting. Range, default and full-strength play are unchanged (the
bench node count is the same). The second refit is not checked by games
of its own: every weakening it uses was measured.

### Soak test (Windows workstation, 2026-11-22)

A check that games run cleanly at very different speeds: no illegal move,
crash, stall or loss on time by Vyvræl. It is not a strength match, and
its scores are not quoted.

- **Time controls in seconds,** as `cutechess-cli` counts them: 1+0 (one
  second a side), 3+2 (three seconds a side plus two a move) and 60+0 (one
  minute a side).
- **Build:** the then-current build, `vyvrael-dev-a874077.exe` (engine code
  of `a874077`), run by the maintainer with `match.ps1` from the clean commit
  `a39ecf3`, 12:37 to 13:29 UTC. Set up as the Lichess bot runs it:
  `Variety=10`, with `Seed=20261126` fixed so that the `Variety` choices
  repeat (the games themselves are not exactly reproducible: the depth
  each move reaches depends on the clock); `Move Overhead` left at its default of 30 ms, harsher than the
  bot's 100.
- **Opponents:** Stockfish 19-1800, Rodent IV-2000 and `Vyvrael 0.1.0-a2`,
  Hash 64, one thread, `UHO_Lichess_4852_v1.epd` (`-Seed 20261126`).

| Time control | Games | PGN SHA-256 (not committed) | Vyvræl: illegal moves, crashes, stalls | Vyvræl: lost on time | Others: lost on time |
| --- | --: | --- | --: | --: | --: |
| 1+0 | 204 | `03d0db8331e2bc14191b97bc2aa9e63dd4eb4383d458d1b361a3860a1cc384a5` | 0 | **1** | 14 (all Rodent IV-2000) |
| 3+2 | 60 | `c5a81e52b298b216eae901717a3808974e4033506a3821ca2b7e5265b7fc2cee` | 0 | 0 | 0 |
| 60+0 | 60 | `ffdfc5c83a1c66f108a4f05bb4907f6713f7a475c8808a3443dbcc6cc54052f3` | 0 | 0 | 0 |

`cutechess-cli` ended no engine process by force in these runs.

**The one time loss** (1+0, Stockfish 19-1800 against Vyvræl, which
played Black) came in a 124-move game that never reached a draw
adjudication: Vyvræl's score was within the rule's 20 cp only on isolated
moves after move 34, never ten in a row, and +0.3 or more for its last
45 moves. Vyvræl spent its whole second over its 123 moves, the last
dozen at about a millisecond each (depth 2 or 3), and ran out. It is not
known issue KI-10: no move was slow. It is the time allocation: without an
increment the engine keeps spending at least a millisecond a move, so a
long enough game at a tiny time control runs the clock out. This is
recorded as known issue KI-12. The Lichess bot is not exposed to it: its
games all have an increment of at least two seconds (see LICHESS.md).

**Result:** no illegal move, crash or stall at any time control. One loss
on time at 1+0, explained above and recorded as KI-12, none at 3+2 or
60+0.

### Smoke test of the scripts (Windows workstation, 2026-11-22)

`match.ps1` and `rate.ps1` were run for real once in each mode, at the
settings above with `-Seed 1` and `-Label smoke`, from the clean commit
`d7f05b3`. The current build was byte for byte 0.1.0-alpha.2 (SHA-256
`1b0d518f…c81ad3`, the release asset), since no engine code had changed
since then, so the published exe stood in for it.

| Run | Games | What it showed |
| --- | --- | --- |
| `sprt`, alpha.2 against alpha.1, 10 rounds | 20 | Ended undecided at the round cap (LLR −0.129), as 20 games must; the notes say so in words. |
| `h2h`, alpha.2 with `-TestOption UCI_LimitStrength=true, UCI_Elo=1500, Seed=1` against alpha.1, 5 rounds | 10 | The options reached the limited build only, and the notes list them for each build. |
| `gauntlet`, alpha.2 against Vice 1.1 and Rustic Alpha 1.5, 5 rounds | 20 | Opponents and their options came from the engine list, which was copied beside the notes byte for byte. |
| `rate.ps1` over the `sprt` and `h2h` games, anchored on alpha.1 | 30 | A table, a CSV and the order. |

Every run's notes had the commit marked clean, each engine's file and
SHA-256, the right game count and the PGN's SHA-256. No game was lost on
time or by a crash; every game ended by mate, adjudication or a drawing
rule. The only warnings in the logs were cutechess-cli's "Rustic Alpha
1.5 doesn't have option Hash", which is harmless: `-each` offers a hash
size to every engine, and that version has none. These are too few games to say anything about
strength, and no rating from them is quoted anywhere.

Rating all three files together failed, and rightly: Vice 1.1 won all ten
of its games, so Ordo found a group linked to the rest by wins only (see
"Pitfalls"). `rate.ps1` stopped with Ordo's status. Ordo had already
created the two output files, which stay behind empty; that is recorded as
known issue KI-13 in `ROADMAP.md`.

### Results from different machines

Results from the Windows workstation and the Linux machine are **never
compared**, and are never merged into one rating. The same build reaches
different depths on them (the Linux machine is several times slower per
core), so its rating would differ although the engine is the same, and
the difference says nothing about the engine. Strength is measured on the
Windows workstation, where the ladders were played; the Linux machine runs
the Lichess bot and checks that builds work there. What does carry across
machines: the bench node count (it must be equal everywhere), perft
counts, test results, and whether games complete without failures.

### Pitfalls

- **XBoard-only engines cannot start from FEN openings.** TSCP and
  Micro-Max were dropped from the baseline for this reason: every game
  began from an EPD position they could not set up, so the run was invalid.
  Only UCI engines belong in a match that uses the opening file.
- **Ordo cannot rate across one-way results.** If a player (or a group)
  only won, or only lost, against the rest, Ordo reports "Database is not
  well connected" and stops, whichever anchor is given, even after it
  drops the player with a perfect score. `-g FILE` shows the groups. Give
  such a pairing more games, choose opponents closer in strength, or
  leave those games out of the rating; do not force it with `-G`.
- **An undecided SPRT is not a pass.** Raise the round cap and run again,
  or treat the change as unproven.
- **Do not compare across machines.** A build measured on the workstation
  and one measured on the Linux machine have not been compared.
- **Watch the log for time losses and crashes.** `-recover` keeps the match
  going, so a build that crashes still produces a result; the PGN's last
  comment of each game says how it ended.
- **`--deterministic` (`-Deterministic`)** sets `Seed=822286742` (the
  project's default seed, `0x31031996`) and `Variety=0` on the Vyvræl
  builds. Both are already what a match needs: `Variety` is 0 by default and
  full-strength play then draws no random numbers, so the switch only
  writes that down in the command. Never run a strength match with
  `Variety` above 0. (`--seed N` is a different thing: the seed of the
  opening order, `-srand`.)
- **`--test-option NAME=VALUE` (`-TestOption`)** gives the build under
  test a UCI option, `option.NAME=VALUE` in the command; repeat it for more
  (in PowerShell: `-TestOption "UCI_LimitStrength=true","UCI_Elo=1500"`).
  `--base-option` (`-BaseOption`) does the same for the base build in
  `sprt` and `h2h`; a gauntlet opponent's options come from the engine
  list. This is how a limited Vyvræl is measured, for example when
  calibrating `UCI_Elo`. `Threads` and `Hash` are refused (the script sets
  one thread and `--hash`), as are `Seed` and `Variety` next to
  `--deterministic`, an option given twice, anything but printable ASCII
  (a line break would put a second command into the engine's input),
  quotes, and a value ending with a backslash (Windows PowerShell 5.1
  passes one on wrongly). `match.sh` refuses `;` in an option;
  `match.ps1` splits a string at `;` into several options, as it does
  opponents. The notes list each build's options.

## Test suites

```sh
cargo run --release -- suite <file> (--depth N | --nodes N | --movetime MS)
```

searches every position of an EPD file and says whether the move found is
the one the file asks for:

```
1 back rank: solved, played a1a8 (bm Ra8+)
2 quiet move: failed, played e2e4 (bm Nf3; am e4)
4: skipped, no bm or am operation
solved 1 of 2 positions (1 line skipped)
```

- A line of the file is a position (the first four fields of a FEN) and
  operations: `bm` lists the best moves, `am` moves to avoid, `id` names
  the position. A position is **solved** when the move played is one of
  the `bm` moves, if there are any, and none of the `am` moves. Moves are
  in standard algebraic notation, read forgivingly (see
  `movegen::find_san_move`).
- Each position gets a new full-strength search with exactly one limit: a
  depth, a node count, or a time in milliseconds. **With a depth or a node
  count the output is the same on every run and machine**, so two builds
  can be compared line by line; a time is what published results use, but
  varies with the machine, and a first depth of ordinary length is
  finished whatever the time.
- **A line that cannot be used is skipped, said so, and counted:** not a
  position the engine accepts (FEN parsing is strict and repairs nothing,
  known issue KI-2), no `bm` or `am`, or a move that is not exactly one
  legal move of the position. Results are of the positions that loaded.
  Blank lines and lines starting with `#` are passed over.
- Time and nodes go to stderr. Status 1 means the file could not be read,
  2 that the command line was not understood.
- Scores some suites give per move (in a comment operation) are not read:
  a position is solved or not.

**What a suite is for.** It is a sanity check after a change to the
search, in seconds where a match takes hours: it shows that the search
still finds what it found, and which positions changed. **It is not a
measure of strength and replaces no SPRT.** Positions are chosen, not
sampled; a change that solves more of them can lose games, and the other
way round. A count of solved positions is never quoted as how strong
Vyvræl is.

Suites are third-party files: they are never committed (git ignores
`*.epd`), and the ones used are recorded in [DATA.md](DATA.md) with
their source, licence and date. The command takes a path, so they can
live anywhere.

## Tactical test suites (planned, milestone 8)

A tactical suite derived from the Lichess puzzle database will report the
share of puzzles solved at a fixed depth or time, run with the `suite`
command above. Details will be added with the tooling.

## Reproducible runs

- All non-strength-affecting randomness (Zobrist keys, magic numbers, test
  fixtures) is generated from the fixed seed `0x31031996` and is identical on
  every run and platform.
- Strength-affecting randomness is seeded from system entropy by default
  (`Seed` 0). For a reproducible run set the UCI option `Seed` to any number
  from 1 to 2147483647. The engine logs the seed in use with `info string`
  in reply to `uci` and at every `ucinewgame`, so a run with an entropy seed
  can be reproduced afterwards by giving that number as `Seed`. Two things
  use this randomness: limited-strength play (`UCI_LimitStrength`) and, at
  full strength, `Variety` above 0 in the first 12 full moves. Full-strength
  play with `Variety` 0, the default, uses none.
- **The deterministic guarantee:** one thread, a fixed `Seed`, `Variety` 0,
  and a fixed depth or node count (never a time limit). With these, the
  same commands give the same moves. (With `Variety` above 0 a fixed `Seed`
  and `go depth` still reproduce a game.) `vyvrael bench` is the check that
  full-strength search stays deterministic: its node count is the same on
  every run.
- **Multithreaded search is not bit-for-bit deterministic**, whatever the
  seed: thread scheduling changes the order in which shared tables are
  written. Regression tests and performance comparisons therefore use one
  thread.

## Reading results

- `cargo test` prints one `test result:` line per test binary; any `FAILED`
  line is a regression.
- A failing test is fixed by fixing the code. Tests are never weakened,
  skipped or deleted to get a green run.
