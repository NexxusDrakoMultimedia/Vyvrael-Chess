# Architecture

This document describes how Vyvræl is put together and why. It is updated in
the same commit as any structural change. Sections marked *planned* describe
intent for a later milestone, not code that exists.

## Workspace layout

```
Cargo.toml              workspace root + engine package `vyvrael`
src/lib.rs              engine library (all engine logic lives here)
src/main.rs             engine binary (thin wrapper over the library)
tools/vyvrael-data/     data and database tooling (stub; milestone 8)
tools/vyvrael-train/    NNUE training (stub; milestone 9)
tests/                  integration tests (public API only)
scripts/                tool installer, match and rating scripts, their tests
branding/               brand assets (all rights reserved, not under the GPL)
docs/                   documentation
```

The engine is a library plus a thin binary so that the binary and the
integration tests exercise one implementation, and the tool crates can do the
same once they depend on it (today they are empty stubs with no
dependencies).

**Dependency direction:** the tool crates may depend on the engine crate. The
engine crate must never depend on a tool crate, and aims for zero
dependencies, so the engine binary stays small and builds fast.

## Engine modules

| Module | Status | Responsibility |
| --- | --- | --- |
| crate root (`lib.rs`) | implemented | Engine identity constants (`ENGINE_NAME`, `ENGINE_NAME_ASCII`, `ENGINE_AUTHOR`, `ENGINE_VERSION`). |
| `rng` | implemented | The only source of randomness: `Rng` (SplitMix64) with `Rng::new`, `Rng::from_state`, `Rng::stream`, `next_u64`, `next_u32` and `next_below`; `DEFAULT_SEED`; the stream indices in `rng::stream`; `entropy_seed` and `SEED_MAX`; `mix64`. |
| `board` | implemented | Board representation: `Color`, `Square`, `PieceKind`, `Piece`, `Bitboard`, `CastlingRights`, `Move`, `Position`, FEN parsing, validation and serialisation, Zobrist keys (`board::zobrist`), the position hash, make/unmake, coordinate-notation move parsing and a text diagram (`Position::diagram`). |
| `attacks` | implemented | Attack tables: knight, king and pawn tables, slider attacks by ray scanning, and magic bitboards for bishops, rooks and queens. |
| `movegen` | implemented | Move generation: `MoveList`, `pseudo_legal_moves` (all pieces, pawns and castling) and `legal_moves`. `find_legal_move` and `find_san_move` turn outside text (UCI notation, standard algebraic notation) into a legal move or nothing. |
| `cli` | implemented | Command line of the binary: engine identification, `perft`, `eval`, `search`, `bench`, `help`. |
| `bench` | implemented | `bench(depth)`, `BenchResult`, `BENCH_POSITIONS`, `BENCH_DEPTH`: a fixed full-strength search workload whose node count is pinned. |
| `epd` | implemented | `EpdRecord::parse` reads one line of an EPD file (a position and its operations, such as `bm`, `am` and `id`), for test suites. The position is checked as strictly as a FEN. |
| `suite` | implemented | `suite::run` searches every record of an EPD test suite for a depth, a node count or a time (`SuiteLimit`) and reports whether the move played is a `bm` move and not an `am` move; `suite::solve` does one line. A quick check of the search, not a measure of strength. |
| `perft` | implemented | `perft`, `divide` and `perft_copy_make`: node counting for testing move generation. |
| `eval` | implemented | Static evaluation: the `Evaluator` trait, `Score`, `relative_to`; material (`Material`, `piece_value`, `material_balance`); `game_phase`; `Tapered` scores; piece-square tables (`piece_square`, `piece_square_balance`); the `HandCrafted` evaluator with its `Breakdown`; `default_evaluator`; `report`, the evaluation and its parts as lines of text. |
| `search` | implemented | Search core: score constants and mate-score arithmetic; fixed-depth negamax with alpha-beta; draw detection; the principal variation; limits and stopping; iterative deepening; quiescence search with ordered captures; weakened play for `UCI_LimitStrength` (`weakened_move`, `Weakening::for_elo`); the `Variety` choice (`varied_move`, `split_limits`). |
| `uci` | implemented | The UCI protocol: command parsing, output formatting, time management, options, and the engine session with the search on its own thread. |

## Board representation

Implemented in `src/board/`, one small file per type or concern, re-exported
from `board`.

- **`Color`** (`color.rs`): `White = 0`, `Black = 1`, so it indexes arrays
  directly. `flip()` and `!` give the other side.
- **`Square`** (`square.rs`): a `u8` newtype in `0..64` using little-endian
  rank-file mapping (`a1 = 0`, `h1 = 7`, `a8 = 56`, `h8 = 63`;
  `index = rank * 8 + file`). The index is also the square's bit position in
  a bitboard. Files and ranks are plain `u8` values in `0..8`. `offset` steps
  by files and ranks and returns `None` at the board edge, so it cannot wrap.
- **`PieceKind`** and **`Piece`** (`piece.rs`): `PieceKind` is
  `Pawn = 0 .. King = 5` for array indexing; `Piece` pairs a `Color` with a
  `PieceKind` and converts to and from FEN characters (`PNBRQK` / `pnbrqk`).
- **`Bitboard`** (`bitboard.rs`): a `u64` newtype holding a set of squares,
  bit `i` for square index `i`. It supports the set operators (`&`, `|`, `^`,
  `!`), membership edits, `lsb`/`msb`/`pop_lsb`, and iteration over member
  squares in ascending index order. Its `Debug` output is an 8x8 diagram with
  rank 8 at the top. `Bitboard::file` and `Bitboard::rank` give line masks,
  and the eight one-step shifts (`north`, `south`, `east`, `west` and the four
  diagonals) move a whole set at once; east/west shifts mask the edge file
  first so squares never wrap around the board.
- **`CastlingRights`** (`castling.rs`): four flag bits (`K = 1`, `Q = 2`,
  `k = 4`, `q = 8`), so a value is also an index in `0..16`. A right only
  records that king and rook are still on their home squares and unmoved;
  legality of castling is move generation's concern.
  `CastlingRights::revoked_by(square)` gives the rights lost when any piece
  moves from or to a square, which lets make-move update rights with two
  lookups (origin and destination) and covers king moves, rook moves and rook
  captures alike. Standard chess only: Chess960 castling is not supported.
- **`Move`** and **`MoveKind`** (`mv.rs`): a move is 16 bits: origin square
  (bits 0-5), destination square (bits 6-11) and a 4-bit flag code (bits
  12-15). Flag codes: `0` quiet, `1` double pawn push, `2` king-side castle,
  `3` queen-side castle, `4` capture, `5` en passant, `8-11` promotion to
  knight/bishop/rook/queen, `12-15` capturing promotion to the same four.
  Codes `6` and `7` are unused and cannot be constructed. Castling is encoded
  as the king's move (`e1g1`). A move does not record which piece moves; that
  comes from the position. `Display` gives UCI coordinate notation (`e2e4`,
  `e7e8q`). Sixteen bits keeps move lists and transposition-table entries
  small.
- **`Position`** (`position.rs`): piece placement is stored twice and kept in
  step. Six piece-kind bitboards plus two colour bitboards answer set
  questions ("all white knights") in one AND; a 64-entry `Option<Piece>` array
  (a "mailbox") answers "what is on e4?" without scanning bitboards, which
  make-move needs constantly. The rest of the state is the side to move,
  castling rights, the en passant target square, the halfmove clock and the
  fullmove number. `put_piece` and `remove_piece` are the public tools for
  editing placement; they update every view together and panic on misuse
  (placing on an occupied square, removing from an empty one). Make/unmake
  and the FEN parser change placement through the internal `place` and `lift`
  helpers instead, which also update every view. `is_consistent()` checks
  that the views agree. `Position::empty()` is a building block, not a legal
  position.
- **FEN parsing** (`fen.rs`): `Position::from_fen` (also `FromStr`) and
  `Position::startpos()`. Fields are split on any whitespace. Either all six
  fields or the first four (EPD style) are accepted; with four, the halfmove
  clock is 0 and the fullmove number 1. Parsing is strict about syntax: 8
  ranks of exactly 8 squares, no two digits in a row, `w`/`b`, a valid
  castling field, a valid en passant square or `-`, and counters made only of
  digits that fit in 16 bits (any value `0..=65535` is accepted, including a
  fullmove number of 0, which some tools emit). Errors are reported as a
  `FenError` variant naming the problem.
- **FEN validation** (`fen.rs`): after parsing, a position is accepted only if
  each side has exactly one king, no pawn is on the first or eighth rank, no
  side has more material than a game can produce (pawns plus pieces beyond the
  starting set must not exceed eight), every claimed castling right has its
  king and rook on their home squares, and any en passant square is consistent
  with a double push by the side that just moved (correct rank, target square
  and the pawn's starting square empty, that side's pawn directly beyond the
  target). These are the invariants make/unmake relies on. Rejecting rather
  than repairing means a bad input is noticed instead of silently changed; an
  opt-in repair mode is planned for later (known issue KI-2). A position where
  the side not to move is in check is rejected too
  (`FenError::OpponentInCheck`), since the side to move could capture the king
  and no legal game reaches that. An en passant square is kept whenever a
  double push happened, whether or not a capture is actually possible.
- **FEN serialisation** (`fen.rs`): `Position::to_fen` and `Display` write
  the canonical six-field form (single spaces, castling letters in `KQkq`
  order). A canonical FEN round-trips byte for byte. For any position that
  passes FEN validation (so every position that came from `from_fen`),
  parsing the output gives back an equal position. A hand-built position such
  as `Position::empty()` can serialise to text the parser rejects.
- **Zobrist keys** (`zobrist.rs`): 781 base keys taken in order from stream
  `stream::ZOBRIST` of the default seed: 768 piece keys
  (`[colour][kind][square]`, white first, pawn to king, a1 to h8), 4 castling
  flag keys (`K`, `Q`, `k`, `q`), 8 en passant file keys (a to h) and 1
  side-to-move key (XORed in when Black is to move). The table is a `static`
  built by a `const fn`, so it costs nothing at start-up and needs no lazy
  initialisation or `unsafe`. The key for a castling-rights value is the XOR
  of its flags' keys, so the key for no rights is 0 and losing a right is
  hashed by XORing out just that right's key.
- **Position hash** (`position.rs`): every `Position` stores its Zobrist hash
  and keeps it current. `put_piece` and `remove_piece` XOR the piece key in
  or out and also correct the en passant key whenever the edit changes
  whether an en passant capture is legal (a pawn beside the square, a pinning
  or shielding piece, the king); the FEN parser folds in the state
  fields. Internally they wrap `place` and `lift`, which touch only the piece
  keys and are what make/unmake and the FEN parser use. `Position::hash()` is
  constant time; `compute_hash()` rebuilds it from scratch and exists to
  initialise and to verify (`is_consistent()` compares the two). The hash
  covers placement, side to move, castling rights and the en passant file.
  The halfmove clock and fullmove number are not hashed.
- **En passant hashing rule:** the en passant file is hashed only when the
  side to move has a *legal* en passant capture. A double push that cannot be
  answered en passant then hashes the same as the identical position reached
  another way, which is what repetition detection and the transposition table
  need and how the rules of chess define a repeated position. Legality is
  decided without making the move: with the capturing pawn placed on the
  target square and the captured pawn removed from the occupancy, no remaining
  enemy piece may attack the king (`attackers_to` with a custom occupancy).
  That covers pins on either pawn, both pawns leaving the rank at once, and a
  check the capture does not answer. The test only runs when an en passant
  square exists and a pawn stands beside it, so it costs nothing in ordinary
  positions. `Position::en_passant()`
  itself always reports the square after a double push, as FEN does.
- **Make/unmake** (`makemove.rs`): `Position::make_move(mv)` plays a move in
  place and returns an `Undo`; `Position::unmake_move(mv, undo)` takes it
  back and restores the position exactly, including counters and hash. The
  `Undo` holds only what a move destroys and cannot be recomputed: the
  captured piece, the previous castling rights, en passant square and
  halfmove clock, and the previous hash. Returning it to the caller (rather
  than keeping a history stack inside `Position`) keeps `Position` a plain
  value with no heap allocation; a caller that unmakes moves keeps its own
  stack. `Undo` is
  `#[must_use]`. Both styles are supported: in-place make/unmake, and
  copy-make (clone the position, about 200 bytes, and make the move on the
  clone). Measured with perft in milestone 2, copy-make was about 5% faster
  (numbers in [TESTING.md](TESTING.md)). Milestone 4 measured both inside the real
  search and copy-make was again 3 to 5% faster, with identical node counts,
  so **the search uses copy-make**. `make_move` and `unmake_move` remain: perft
  uses them, and they are the right tool wherever a move really must be taken
  back on the same position.
  - `make_move` trusts its input: the move must be pseudo-legal for the
    position. It panics on moves that do not fit at all (empty origin square,
    capture with nothing to capture) and checks consistency in debug builds.
    It does not test legality.
  - Hash updates are incremental: old en passant and castling keys out, piece
    keys via `place`/`lift`, new castling key in, side key toggled, then the
    new en passant key if the rule above applies to the new side to move.
    `unmake_move` simply restores the saved hash.
  - Castling rights are updated with `CastlingRights::revoked_by` on the
    origin and destination squares of every move.
  - The halfmove clock resets on pawn moves and captures and otherwise
    saturates at 65535. The fullmove number increases after Black's move
    using wrapping arithmetic, so unmake is exact even at the 16-bit limit.
  - A double push sets the en passant square to the square the pawn passed
    over. An en passant capture removes the pawn standing on the destination
    file and the origin rank, not on the destination square.
  - Castling is given as the king's move. The rook's squares follow from
    the king's destination: g-file means the h-rook goes to the f-file,
    c-file means the a-rook goes to the d-file. The king leaving its home
    square revokes both of that side's rights through the normal rule.
  - A promotion places the promoted piece instead of the pawn; unmake puts
    a pawn back on the origin square. A capturing promotion also saves and
    restores the captured piece.
- **Move parsing** (`notation.rs`): `Position::parse_move("e2e4")` reads UCI
  coordinate notation and works out the `MoveKind` from the board, because
  the notation does not say whether a move is a capture, a double push,
  castling or en passant. Castling is a king moving two files along the rank
  from its home square. It rejects malformed text, an empty or enemy origin
  square, an own piece on the destination, a missing or unexpected promotion
  letter, and a diagonal pawn move to an empty square that is not the en
  passant square. **It does not check piece geometry or legality** (a rook
  moving like a knight parses as a quiet move); where legality matters,
  use `movegen::find_legal_move` instead, which returns a move only if it is
  legal. The UCI layer does.

## Attacks and move generation

`src/attacks/` answers "which squares does a piece
on this square attack?".

- **Leapers** (`attacks/leapers.rs`): knights, kings and pawns attack a fixed
  pattern that does not depend on other pieces, so each has a 64-entry table
  built at compile time from the bitboard shifts (which cannot wrap around the
  board edge).
  - `knight_attacks(square)`.

  - `king_attacks(square)`: the eight neighbouring squares. Castling is a
    move, not an attack, and is not in the table.

  - `pawn_attacks(color, square)`: the two squares diagonally in front.
    It is defined on all 64 squares because it is also used in reverse: the
    pawns of one colour attacking a square stand on the other colour's attack
    set of that square.

- **Sliders by scanning** (`attacks/scan.rs`): `bishop_attacks_by_scan` and
  `rook_attacks_by_scan` walk outwards along each ray until the board edge or
  the first piece, which is included whatever its colour (the caller removes
  its own pieces). This is slow but obviously correct. It is the reference
  the magic-bitboard tables are built from and tested against; it is not used
  during search.

- **Magic bitboards** (`attacks/magic.rs`): constant-time slider attacks by
  table lookup.
  - *Relevant masks.* A slider's attacks depend only on the pieces on its
    rays, and not on the last square of each ray, since nothing stands behind
    it. `bishop_relevant_mask` and `rook_relevant_mask` give those squares
    (5 to 9 for a bishop, 10 to 12 for a rook).
  - *Subsets.* `subsets(mask)` enumerates every arrangement of pieces on a
    mask with the carry-rippler step `next = (current - mask) & mask`.

  - *Magic numbers and tables* (`attacks/sliders.rs`). For each square a
    magic multiplier maps every blocker arrangement to a slot:
    `index = ((occupied & mask) * magic) >> (64 - mask bits)`. Arrangements
    may share a slot only if they give the same attacks. All squares' slices
    live in one shared table of 5,248 bishop plus 102,400 rook entries (about
    841 KiB), "plain" magics with a per-square offset.
  - *Finding the magics.* Candidates are the AND of three outputs of the
    seeded generator (sparse numbers work best), drawn from
    `Rng::stream(DEFAULT_SEED, stream::MAGICS)` for the bishop squares a1 to
    h8 and then the rook squares. A candidate is kept when filling the slice
    from the scanned reference attacks produces no conflicting slot. Because
    the stream is fixed, every run finds the same numbers; a test pins them.
  - *Embedded, not searched for at start-up.* The 128 numbers the search
    finds are constants in `attacks/magics.rs`. The search itself is test
    code: a test runs it and requires exactly the embedded numbers and
    exactly the table the engine builds, so the numbers remain reproducible
    from the seed and cannot be edited by hand unnoticed.
  - *When.* The tables are built on first use behind a `LazyLock`, by
    filling each square's slice from its embedded number, which takes a few
    milliseconds. While filling, each number is checked to be a magic number
    for its mask; a wrong one stops the engine at once instead of giving
    wrong attacks.
  - `bishop_attacks`, `rook_attacks` and `queen_attacks(square, occupied)` are
    the lookups move generation uses.

- **Attack queries** (`board/attackers.rs`): `Position::attackers_to(square,
  occupied)` returns every piece of either colour attacking a square. It
  works backwards from the target: a knight attacks the square exactly when a
  knight standing on it would attack the knight, and the same holds for the
  other pieces (for pawns with the colours swapped), so six table lookups
  answer the question. The occupancy is a parameter so callers can ask "what
  if this piece were gone?". `is_attacked_by`, `checkers`, `in_check` and
  `can_capture_king` are built on it.

- **`MoveList`** (`movegen/list.rs`): a fixed array of 256 moves on the
  stack with a length, dereferencing to a slice. Generation runs millions of
  times per second in search, so it must not allocate. The largest known
  number of legal moves in any position with material a game can produce is
  218; 256 leaves room for pseudo-legal extras. FEN validation rejects
  positions with more material than that (see FEN validation), so a parsed
  position cannot overflow the list. Pushing past the capacity panics rather than
  corrupting memory.

- **Pseudo-legal generation** (`movegen/pseudo.rs`): `pseudo_legal_moves`
  lists every move that follows the pieces' movement rules, without yet
  asking whether it leaves the mover's king attacked. Each piece type has its
  own small generator. A move to an enemy-occupied square is a `Capture`,
  otherwise `Quiet`; squares holding the mover's own pieces are masked out
  first. Every piece type, all pawn moves and castling are generated.

- **Pawns** (`movegen/pawns.rs`) have their own generator because their
  moves depend on colour and they move and capture differently. Pushes are
  computed for all pawns at once with bitboard shifts: a single push is the
  pawn set shifted forward and masked with the empty squares; a double push
  continues those single pushes that landed on the third rank (sixth for
  Black), so both squares must be empty. Captures use the pawn attack table
  per pawn, masked with the enemy pieces. A push or capture onto the last rank
  becomes four moves, one per promotion piece (queen first); under-promotions
  are always generated because they are sometimes the only winning or drawing
  move. En passant is generated for each pawn attacking the position's en
  passant square (found by looking from the square with the other colour's
  attack table); whether the capture is legal is left to the legality filter,
  which matters because en passant removes two pieces from one rank at once.

- **Castling** (`movegen/castling.rs`) is generated only when it is fully
  legal: the right exists, the squares between king and rook are empty, the
  king is not in check, and neither the square it crosses nor the one it
  lands on is attacked. Unlike every other move, this cannot be checked after
  the fact by looking at where the king ended up, so it is done here. When
  castling queenside the b-file square must be empty but may be attacked.
  The four castling options are small constant records, so the generator is
  one loop with no colour-specific code.

- **Legality** (`movegen/legal.rs`): `legal_moves` plays each pseudo-legal
  move on a scratch copy and keeps it unless the opponent could then capture
  the mover's king (`Position::can_capture_king`). This single test covers
  pins, king steps onto attacked squares, unanswered checks and the en
  passant capture that uncovers an attack along the rank, with no special
  cases to get wrong. It costs a make/unmake and an attack query per move;
  a pin-aware generator that avoids this can replace it later behind the
  same function, with perft as the safety net. An empty result means
  checkmate when `in_check()` is true and stalemate otherwise.

- **Perft** (`perft.rs`): `perft(pos, depth)` counts the leaf nodes of the
  legal move tree, and `divide(pos, depth)` gives the count below each first
  move. They walk the pseudo-legal moves with make/unmake on one position and
  skip moves after which the king could be captured, so they exercise move
  generation, make and unmake together. `perft_copy_make` counts the same
  tree by cloning the position for each move, which is the style the search
  uses; the two were measured against each other (see
  [TESTING.md](TESTING.md)).

## Evaluation

`src/eval/` gives a position a score without searching. Milestone 3 covers
material and piece-square tables only; other terms (mobility, king safety,
pawn structure) are candidates for later, each to be accepted by SPRT.

- **Scores** are `eval::Score`, an `i32` in centipawns (100 is about one
  pawn).
- **`Evaluator`** (`eval/mod.rs`) is the trait search calls:
  `evaluate(&self, pos) -> Score` returns the score **from the side to move's
  point of view**, which is what negamax needs, and `name()` identifies the
  evaluator. Search holds it as a trait object, so the hand-crafted evaluation
  and a later neural network are interchangeable and can be played against
  each other. Evaluators add up "White minus Black" and convert with
  `eval::relative_to`.
- `Evaluator` requires `Send + Sync`, because search runs on its own thread
  and, with several search threads, they all share one evaluator. An
  evaluator that needs scratch state per search (a network's accumulators)
  must keep it outside the shared object.
- The trait takes the whole position each time. A network that updates
  incrementally on make/unmake (milestone 9) will need notification hooks;
  they will be added then as default methods, so existing evaluators do not
  change.

- **Material** (`eval/material.rs`): `piece_value` gives the conventional
  values in centipawns (pawn 100, knight 320, bishop 330, rook 500, queen 900,
  king 0). `material_balance` is White's total minus Black's, and the
  `Material` evaluator returns just that. It is the simplest sound evaluation
  and the baseline later terms are measured against.

- **Game phase** (`eval/phase.rs`): `game_phase` measures how much attacking
  material is left, from 0 (kings and pawns only) to `MAX_PHASE` = 24 (all
  pieces on). Knights and bishops count 1, rooks 2, queens 4; pawns and kings
  count 0. It is capped at 24 so promoted pieces cannot exceed the opening
  value. This is the usual scheme, chosen because it is cheap and well
  understood.

- **Tapered scores** (`eval/tapered.rs`): a `Tapered` is a pair of values for
  one feature, what it is worth in the middlegame (`mg`) and in the endgame
  (`eg`). `blend(phase)` gives `(mg * phase + eg * (24 - phase)) / 24`, which
  is exactly `mg` with all pieces on and exactly `eg` with none. Features
  change value as pieces come off (a central king is a liability with queens
  on and an asset in a pawn ending), and blending avoids a sudden jump at an
  arbitrary "the endgame starts here" line. The division rounds towards zero,
  so negating a pair negates the result exactly; that keeps the evaluation
  symmetric between the colours.

- **Piece-square tables** (`eval/pst.rs`): a `Tapered` bonus for each piece
  kind on each square, saying where that piece usually stands well.
  `piece_square(piece, square)` looks one up and `piece_square_balance(pos)`
  sums White's minus Black's.
  - The tables are written in the source the way a board is printed (rank 8
    at the top, from White's side) and converted at compile time to
    square-indexed arrays. A black piece uses the same table with the board
    flipped top to bottom, so the two sides are treated identically.
  - Every table is symmetric between the queenside and the kingside; a test
    enforces it.
  - The values were written for this project from general chess principles,
    not copied from another engine, and lean towards the project's style.
    They are untuned (known issue KI-8).
  - **The tables:** pawns (middlegame: take the centre and do not
    leave the d- and e-pawns at home; endgame: each step towards promotion is
    worth more than the last); knights (centre and outposts good, rim and
    corners bad); bishops (long diagonals and active squares good, back rank
    bad); rooks (seventh rank and central files); queens (small middlegame
    values so she is not drawn out early, centralised in the endgame); kings
    (middlegame: g1 and b1 best, both castled squares better than the home
    square, worse with every rank advanced; endgame: centre best).

- **The hand-crafted evaluator** (`eval/handcrafted.rs`): `HandCrafted`
  returns `material + piece_square.blend(phase)`, converted to the side to
  move's view. `HandCrafted::breakdown` exposes the parts (material,
  piece-square pair, phase, total) for inspection. It is the engine's
  baseline: it stays as the fallback and the yardstick once a network exists.
  `eval::default_evaluator()` returns the evaluator the engine uses; today
  that is always `HandCrafted`, and it is the single place where the choice
  between hand-crafted and network evaluation will be made (the planned
  `UseNNUE` option).
- **Symmetry is the main correctness test.** An evaluation has no reference
  answer to compare with, but it must not care which colour it looks at: a
  position and its colour-swapped mirror must score the same for the side to
  move. A sign error, a table read from the wrong side or a term applied to
  one colour only all break that, so the tests check it on a spread of
  positions.

## Search

Implemented in `src/search/`, which looks ahead to choose a move. This is
the core only: the heuristics that make a search fast (move ordering, a
transposition table, pruning) come in milestone 7, one at a time.

- **Scores** (`search/score.rs`) are the evaluator's `Score` (centipawns)
  extended with checkmate. `MATE` is 31,000 and `INFINITY`, the bound no score
  reaches, is 32,000. A position where the side to move is checkmated `ply`
  plies from the root scores `mated_in(ply) = -MATE + ply`, and the mirror
  image is `mate_in(ply)`. Counting the distance into the score makes the
  search prefer the quickest mate and, when lost, the longest defence, with no
  extra logic. Anything with absolute value at least `MATE_BOUND`
  (`MATE - MAX_PLY`) is a mate score; evaluations stay far below it.
  `moves_to_mate` converts a mate score to the signed move count UCI reports.
  `MAX_PLY` is 128.

- **Negamax with alpha-beta** (`search/negamax.rs`): `Search::fixed_depth`
  searches a position to an exact depth and returns an `Outcome` (score and
  best move). Negamax is minimax written once for both sides: a position is
  worth the best, over its moves, of the negated value of what each move leads
  to. Alpha-beta skips moves that cannot matter: `alpha` is what the side to
  move is already sure of and `beta` what the opponent can already hold it to,
  so once a move reaches `beta` the rest are not examined.
  - The search is **fail-soft**: a result outside the window is a bound on the
    true value. The root searches with the full window, so its score is exact.
  - Moves are taken straight from `pseudo_legal_moves`, played on a copy of
    the position, and skipped if they leave the king capturable. The search
    never modifies the position it is given ("copy-make"); see the
    make/unmake discussion under "Board representation" for why.
  - A node with no legal move scores `mated_in(ply)` if in check and `DRAW`
    otherwise, so stalemate is never mistaken for a win.
  - At depth 0 the quiescence search takes over (below), so positions are
    not judged in the middle of an exchange.
  - Moves are searched in generation order. Ordering them well is what makes
    alpha-beta efficient; it is deliberately left to milestone 7.
  - **Correctness is tested against plain minimax**, which has no pruning to
    get wrong: on nine positions, two or three plies deep, the two must return
    the same score, and the move alpha-beta names must lead to that score.

- **Draws** (`search/draw.rs`): inside the tree a position is scored `DRAW`
  at once when it is drawn by rule, which both saves work and stops the
  search walking in circles.
  - *Insufficient material:* bare kings, or one knight or bishop in total.
    Deliberately conservative: wrongly claiming a draw is far worse than
    missing one.
  - *Repetition:* the position's hash already occurred. `Search` keeps the
    hashes of the game so far (given with `with_history`) followed by the
    path from the root. One earlier occurrence counts, although the rules
    need three in all: a position that can be repeated once can be repeated
    again. Only positions with the same side to move and within reach of the
    halfmove clock are compared.
  - *Fifty-move rule:* a halfmove clock of 100, unless the side to move is
    checkmated, because a mate delivered on the hundredth ply still wins.
  - The root is never scored as drawn, since a move must still be returned
    there.

- **Principal variation.** `Outcome::pv` is the line of best play the score
  comes from, starting with the best move. `Search` keeps one line per ply (a
  "triangular" table): a node that finds a new best move stores that move
  followed by its child's line. The line is shorter than the depth when it
  ends in a mate or a draw. It is what UCI reports as `pv`, and it is also a
  check on the search: playing it out must reach a position whose quiescence
  value is the reported score (the line stops where quiescence search takes
  over and does not list the captures played beyond it).

- **Limits and stopping** (`search/limits.rs`). `Limits` holds an optional
  depth, node count and deadline; the search stops at whichever comes first.
  A stop flag (an `AtomicBool` given with `with_stop_flag`) lets another
  thread end a running search, which is how UCI `stop` works.
  - The node limit is exact: the search stops at the first node beyond it.
    The flag and the clock are read once every 1,024 nodes, because reading
    them at every node would cost more than the search.
  - Once any limit trips, a latch makes every node return immediately. The
    recursion unwinds normally, so the repetition history is restored exactly
    (the position itself is never modified); values returned on the way out
    are ignored.
  - A stopped search is incomplete. `fixed_depth` then reports only the root
    moves it finished. Iterative deepening (next) discards an unfinished
    iteration entirely and keeps the last complete one.
  - Use depth or node limits for anything that must be reproducible. Time
    limits depend on the machine.

- **Iterative deepening.** `Search::iterate` searches depth 1, then 2, and so
  on until a limit is reached, calling a `report` callback with a `Report`
  (depth, selective depth, score, nodes, elapsed time, principal variation)
  after each completed depth, and returns the outcome of the deepest one that finished.
  - The selective depth is the deepest ply the iteration visited,
    quiescence included. The search writes it and never reads it, so it
    cannot change what is searched.
- **Statistics** (`search/stats.rs`). The search counts its beta cutoffs
  and how many of them the first move tried gave, in the main search and
  in quiescence, the quiescence positions and those cut off as they
  stood (`SearchStats`, read with `Search::stats`). Like the selective
  depth they are written and never read by the search, so counting them
  cannot move a node count; they are always counted, and shown only when
  asked for. The share of cutoffs given by the first move is the measure
  of move ordering that milestone 7 works on.
  - It looks wasteful, but each level of the tree is many times larger than
    the last, so repeating the shallow levels costs little. In return the
    search can be stopped at any moment and still has a complete answer,
    which is what playing to a clock needs. Later, the earlier iterations
    will also feed move ordering (milestone 7).
  - The first iteration always completes, whatever the limits, so there is
    always a move to play. An interrupted iteration is discarded.
  - It ends early when the root has no legal move, and when a forced mate
    has been found within the depth searched, since a deeper search cannot
    find a shorter one.
  - With no ordering heuristics or tables yet, iterating to depth N gives
    exactly the result of a direct depth-N search; a test relies on that.

- **Quiescence search.** A static evaluation only means something in a quiet
  position. Evaluating at a fixed depth would count a queen that has just
  taken a defended pawn as a pawn won, because the recapture lies beyond the
  horizon. So when the main search runs out of depth it continues with
  captures and queen promotions only, until nothing is left to take.
  - The side to move may always decline to capture, so its static evaluation
    is a floor under its score ("standing pat"). If that floor already
    reaches `beta`, the node is cut off at once.
  - In check there is no standing pat: every legal move is searched, and
    having none is checkmate. This is also why a mate whose last move gives
    check is seen one ply earlier than the nominal depth.
  - It has no depth limit. Captures cannot continue for ever, and a ply cap
    (`MAX_PLY`) is the backstop.
  - Not in check and with no legal move at all, the position is stalemate
    and scores `DRAW`. This is tested before standing pat, because the side
    that cannot move is usually far behind and the evaluation would call the
    position won for the other side. Generating moves at every node only for
    this would cost about a fifth of the search's speed, so a glance at the
    bitboards comes first: when the king is not in check, a pawn or knight
    that shares no rank, file or diagonal with its own king cannot be pinned,
    so one with a square to go to proves there is a legal move. Moves are
    generated for the test only when no such piece exists.
  - Of the draw rules only insufficient material is checked here, because a
    capture can leave bare kings. Captures cannot repeat a position and they
    restart the fifty-move clock. The quiet moves that get out of a check can
    do neither, and quiescence does not test them for repetition or the
    fifty-move rule; the main search does, one ply deeper.
  - **Captures are tried most valuable victim first** (`search/ordering.rs`),
    with the least valuable attacker breaking ties and promotions counted as
    the material they gain. Alpha-beta returns the same value in any order,
    but the cost differs enormously: with captures in generation order one
    busy middlegame position (Kiwipete) took more than 20 million nodes to
    resolve, against 3,067 with this ordering, for the same value wherever
    both finished. So unlike the move ordering of the main search, which
    waits for milestone 7 and SPRT, this ordering is part of the core: without
    it the search does not finish.
  - No other pruning is used beyond alpha-beta and standing pat. Skipping
    captures that cannot help (delta pruning, static exchange evaluation) is
    left to milestone 7.

### Playing below full strength

`search/weaken.rs`, for the UCI option `UCI_LimitStrength`.

A weakened engine should make plausible mistakes, not random moves.
`weakened_move` therefore:

1. scores **every** root move, each with its own full-window search, to a
   shallow depth (a weaker player sees less far), which gives every move an
   exact score, not only the best one. This is done in passes, like
   iterative deepening: all moves one ply deep, then all moves two plies
   deep, up to the depth of the setting. The first pass always completes;
   the clock, a node limit or `stop` can only cut a later pass short, and a
   pass cut short is thrown away. So the choice is always made from scores
   that all look equally far ahead, however little time there is;
2. adds to each score a random error between minus and plus `noise`
   centipawns, the sum of two uniform draws, so small errors are common and
   large ones rare;
3. plays the move with the best misjudged score, and reports its true score.

The error is bounded, so a blunder larger than twice the noise cannot
happen: the engine may prefer the second-best quiet move, but it does not
leave a queen hanging unless the noise is that large. Forced mates are
judged exactly, for and against.

`Weakening::for_elo(elo)` maps a rating from `ELO_MIN` (1110) to
`ELO_MAX` (1630) to a depth and a noise through a table of seven
weakenings measured in milestone 6 (`CALIBRATION` in `search/weaken.rs`):
two plies from 1110 to 1349, three from 1350, with the noise interpolated
in a straight line between points of the same depth and held at the last
two-ply point (112 centipawns) from 1331 to 1349. `ELO_MIN` and `ELO_MAX`
are read from the table. The ratings are on the reference ladder's scale
(see [UCI.md](UCI.md), "Limited strength"). **The calibration is
provisional:** measured for one build, and measured again after milestone
7 and after queued task 2 (benchmark and speed-up; see
[ROADMAP.md](ROADMAP.md)).

The generator is handed in by the caller and advanced once per root move
before any searching, so the errors are a function of the position and the
seed alone. Which pass they are applied to depends on the limits: with a
node limit, or time that is never used up, a weakened game is reproducible
with a fixed `Seed`; with a clock that cuts passes short it is not.

Scoring the position after a root move uses `Search::reply_value`, which
treats that position as a node inside a search and not as a root: a
position drawn by rule is worth a draw, and a mate is counted from the
position before it.

### Variety in the opening

Implemented in `search/variety.rs`. At full strength the same position
searched to the same depth gives the same move, so self-play and a bot
repeat their games. `varied_move` lets the engine play, in the first 12 full
moves (`VARIETY_FULL_MOVES`), a move within `Variety` centipawns of the best
instead, at random and weighted towards the best. It runs after the
ordinary search, from its outcome and the depth D it reached:

1. **Cheap scores.** Every root move is scored by quiescence search alone
   (`reply_value` at depth 0). A move is worth checking if that is within
   `Variety` plus 100 centipawns of the best cheap score; quiescence
   misjudges quiet moves, so the margin is generous.
2. **The second check.** Each move worth checking, most promising first,
   is searched exactly to the best move's depth (`reply_value` at D − 1
   after the move). It is kept only if that score is within `Variety` of
   the best move's score and is not a mate. Once a limit or `stop` cuts a
   search short, no further move is checked, and an unchecked move is never
   played.
3. **The choice.** Among the best move and the kept ones, each is weighted
   by `(Variety + 1 − loss)²`, so the best move has the largest weight and a
   move exactly `Variety` worse the least. One number is drawn from the
   generator per choice, however many moves were checked.

It is not used when the best score is a mate, for or against, or when
there is no move. `split_limits` shares the move's budget: when `Variety`
applies, the ordinary search gets two thirds of the time and node limit and
the checks the rest, so that a varied move is checked as deeply as the best
one was found; a depth limit stays with the search and the checks then run
to completion, which keeps a fixed-depth run reproducible.

This is strength-affecting randomness, from the same generator as weakened
play (see the randomness table below), and it weakens play on purpose, so
it is off by default and exempt from the SPRT rule.

## UCI

Implemented in `src/uci/`, the protocol layer; every command
and option is listed in [UCI.md](UCI.md).

- **Parsing** (`uci/command.rs`): `parse(line)` turns a line into a
  `Command`. It knows the grammar and nothing about chess, so it is tested
  without an engine. It is forgiving where the protocol asks for it: words
  before the first known command are skipped and unknown `go` words ignored.
- **Output** (`uci/output.rs`): `info_line`, `bestmove_line` and
  `score_text` build the lines the engine sends.
- **Win, draw and loss** (`uci/wdl.rs`): `wdl` turns a score into three
  chances per thousand by a fixed model (two logistic curves, constants
  `WDL_HALF` and `WDL_SPREAD`), not fitted to any games. Its exponential
  uses only additions, multiplications and divisions, so the figures are
  the same on every platform.
- **Time management** (`uci/time.rs`): `time_for_move` gives a move an equal
  share of the remaining time (over `movestogo`, or 30 moves) plus three
  quarters of the increment, capped at what is on the clock less the
  overhead, which is the UCI option `Move Overhead` (30 ms by default). A
  clock at or below the overhead gets no time, so the move comes from the
  first depth, which the search always completes. `limits_for` combines it with `depth`, `nodes` and `infinite`
  into the search's `Limits`. The scheme is deliberately plain: schemes that
  react to the search change playing strength and wait for milestone 7.
- **Options** (`uci/options.rs`): `Options` holds the value of every UCI
  option. `Options::declarations()` gives the `option` lines for `uci` and
  `Options::set(name, value)` checks and stores a value; an option exists
  only in those two places, so what the GUI is shown and what the engine
  accepts cannot drift apart. A test sets every declared option to its
  declared default and requires that nothing changes.
- **The engine** (`uci/engine.rs`): `Engine` owns the current position, the
  hashes of the positions the game went through, and the running search.
  `Engine::handle(line)` acts on one line; `run(input, out)` is the loop the
  binary uses.
  - **Two threads.** The thread that calls `handle` reads commands. `go`
    copies the position and history and starts the search on a second
    thread, so `stop`, `isready` and `quit` are answered while the engine
    thinks. They share two things: the output, behind a mutex, written one
    whole line at a time and flushed; and an `AtomicBool` that the search
    reads every 1,024 nodes.
  - **One search at a time.** `stop`, `quit`, `ucinewgame`, `position` and a
    second `go` all end the running search and wait for its `bestmove`
    before doing anything else, so there are never two.
  - **Untrusted input.** A move in a `position` command is looked up with
    `movegen::find_legal_move`; an illegal one, or a bad FEN, refuses the
    whole command and leaves the position as it was. Nothing illegal reaches
    `make_move`.
  - **`go infinite`** (and a `go` with no limit at all) answers only after
    `stop`, as the protocol requires, even when the search itself ended at
    once.
  - **Limited strength.** With `UCI_LimitStrength` on, `go` runs
    `search::weakened_move` instead of the ordinary search. The engine keeps
    one generator, `Rng::stream(seed, stream::STRENGTH)`, on the command
    thread. Each weakened search gets a generator of its own, seeded by one
    draw from it, so the random sequence of a game depends on the commands
    and not on how far each search got. Full-strength play draws from it
    only when `Variety` applies, one draw per `go`, for the choice's own
    generator (see "Variety in the opening"); with `Variety` 0 it never
    does. `ucinewgame` restarts the generator and logs its seed: the chosen
    `Seed`, or with `Seed` 0 a new entropy seed. Either way one logged
    number replays one game, including the later games of a session, which
    is how match tools play.
  - **Bytes, not text.** `run` reads lines as bytes and replaces what is
    not valid UTF-8, so a stray byte spoils one line and not the session.
  - **End of input** without `quit` lets a running search finish and report,
    so a script that pipes commands in gets its answer.

## Command line and bench

- **Command line** (`cli.rs`): `cli::run(args, out, err)` implements the
  binary's commands against any pair of writers, so tests run it on in-memory
  buffers and `main.rs` is only a few lines. With no arguments the binary
  does not call it at all: `main.rs` starts the UCI loop instead (see "UCI"
  above). `perft <depth> [--divide | --copy-make] [FEN]` prints node counts
  on stdout and timing on stderr. `eval [FEN]` prints the hand-crafted
  evaluation and its parts. `search <depth> [--stats] [FEN]` runs iterative deepening
  to a fixed depth and prints one line per depth and the best move, with
  timing on stderr so that stdout is identical on every run; `--stats`
  adds the lines of `search::stats_lines`. `bench [depth [threads [hash]]]`
  runs `bench::bench` and prints `<nodes> nodes <nps> nps`; the threads
  and the hash size are checked and have no effect yet. Exit status 2
  means the command line was not understood.

- **Bench** (`bench.rs`): `bench(depth)` searches the fixed positions of
  `BENCH_POSITIONS` to `depth` at full strength, each as a new search with
  iterative deepening and no time limit, and returns the nodes visited and
  the time taken. Nothing random is involved, so the node count is a
  function of the search alone; `tests/bench.rs` pins it at
  `BENCH_DEPTH`.

## How search, evaluation and UCI fit together

The UCI layer owns the current position and the option values. On `go` it
hands a copy of the position and the search limits to a search thread, so
the command thread keeps reading stdin and can act on `stop`. Search calls
the evaluation through a trait, so the hand-crafted evaluation and the NNUE
evaluation are interchangeable and can be compared by SPRT. Only the UCI
layer writes to stdout; the other commands of the binary (`perft`, `eval`,
`search`, `bench`, `help`) are separate and never run in the same process as the loop.
The loop's own `d` and `eval` reuse `Position::diagram` and `eval::report`
and send them as `info string` lines.

## RNG design

Implemented in `src/rng.rs`.

All randomness in the engine goes through one module, `rng`. The tool crates
are stubs with no dependencies today; when they need randomness they will use
this module through a dependency on the engine crate (or mirror it exactly if
that dependency is ever undesirable). Tools may take a `--seed` argument,
which defaults to `DEFAULT_SEED`.

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
- **Seed derivation:** independent streams (threads, self-play games,
  subsystems) are created with `Rng::stream(base_seed, index)`, never by
  sharing one generator across threads. Its indices are registered for the
  engine's own uses (below); a test that needs several generators of its
  own draws their states from `Rng::default()` instead, as
  `tests/uci_robustness.rs` does. The stream's starting state is
  `mix64(k ^ mix64(index + 0x9E3779B97F4A7C15))`, where `k` is the first
  output of `Rng::new(base_seed)`. Because `mix64` is a bijection, distinct
  indices always give distinct starting states for the same base seed. Streams
  are different offsets into SplitMix64's single 2^64 cycle; with
  well-mixed offsets, an overlap within any practical run length is
  vanishingly unlikely.
- **Bounded integers:** `next_below(bound)` returns a uniform integer in
  `0..bound` using Lemire's multiply-and-reject method, so there is no modulo
  bias for any bound. Use it for all sampling and shuffling.
- **Stream registry:** `rng::stream` holds one named index per subsystem
  (listed under the randomness table below). Indices are never reused or
  renumbered, so adding a consumer cannot change the numbers an existing one
  receives.
- **Golden tests:** known-answer tests pin the first outputs for seed
  `1234567` (the published SplitMix64 reference vector), seed `0` and
  `DEFAULT_SEED`, plus four derived streams. The `DEFAULT_SEED` and stream
  values were computed with an independent implementation.
- **Compile-time use:** every method is a `const fn`, so tables can be
  generated at compile time with no start-up cost.
- **Ambient entropy:** forbidden everywhere except one path: seeding
  strength-affecting randomness when the UCI option `Seed` is 0, its
  default. `rng::entropy_seed()` is that path and the only code that reads
  system entropy; it takes its randomness from the standard library's
  `RandomState`, so the engine stays free of dependencies, and maps it to a
  seed from 1 to `rng::SEED_MAX` (2147483647). Its caller logs the seed with
  `info string`, so even such a run can be replayed by giving the logged
  value as `Seed`.

### Table of randomness uses

Every use of randomness is listed here and classified. "Strength-affecting"
means the randomness can change how strongly or how differently the engine
plays; such uses are non-deterministic by default (`Seed` 0), with a fixed
`Seed` as the opt-in. Everything else is always deterministic.

| Use | Status | Strength-affecting? | Seed | Justification |
| --- | --- | --- | --- | --- |
| Zobrist keys (`board::zobrist`) | implemented | No | `Rng::stream(DEFAULT_SEED, stream::ZOBRIST)`, stream 0, at compile time | The keys only label positions. Any well-mixed set works equally well, and fixed keys keep hashes, tests and transposition-table behaviour reproducible. |
| Magic-number search (`attacks::sliders`, test code; the engine uses the numbers it found, embedded in `attacks/magics.rs`) | implemented | No | `Rng::stream(DEFAULT_SEED, stream::MAGICS)`, stream 1, in the test that checks the embedded numbers | Any valid magic number gives exactly the same attack sets, so the choice cannot affect play. A fixed stream makes the tables identical in every run. |
| Randomised test inputs (`attacks::sliders` tests: 20,000 random occupancies and squares) | implemented | No | `Rng::default()`, the default seed | The samples only choose which cases a test compares against the scanning reference; a fixed seed makes the test reproducible. |
| Malformed UCI input (`tests/uci_robustness.rs`) | implemented | No | `Rng::default()`, the default seed; each of the eight streams of malformed lines, then each of the four streams of damaged sessions, then the one stream for the binary, gets a generator whose state is the next draw from it; the sessions fix a non-zero `Seed`, so what the strength options draw is the same on every run | The input only exercises the protocol loop; a fixed seed makes a failure replayable from its stream number. |
| Generated input for the parsers (`tests/parser_fuzz.rs`) | implemented | No | `Rng::default()`, the default seed; each of the four tests takes a later draw from it as its generator's state | Test input only; a fixed seed means every run tries the same text and a failure can be reproduced. |
| Seed of strength-affecting randomness (`uci::Engine`) | implemented | **Yes** | `rng::entropy_seed()` (1 to 2147483647) when the engine starts, at every `ucinewgame` while `Seed` is 0, and when `Seed` is set back to 0; or the `Seed` given. Logged with `info string` in reply to `uci`, at every `ucinewgame` and whenever `setoption` changes the seed | This seed decides weakened play and the `Variety` choice, which change how the engine plays, so by the project's rule it is not reproducible unless the user asks. Logging it keeps every run replayable. |
| Weakened move choice (`search::weakened_move`, used when `UCI_LimitStrength` is on) | implemented | **Yes** | A generator handed in by the caller: the UCI layer's, from the seed above and `stream::STRENGTH` | The random error decides which move is played, so it changes playing strength by design. |
| Varied opening move (`search::varied_move`, used when `Variety` is above 0, at full strength, in the first 12 full moves) | implemented | **Yes** | The same as weakened play: one draw from the UCI layer's `stream::STRENGTH` generator per `go` where it applies, giving the choice its own generator | It decides which of the checked moves is played, so it changes how the engine plays by design. With `Variety` 0 nothing is drawn. |
| Weakened-play and `Variety` tests (`search::weaken`, `search::variety`) | implemented | No | `rng::test_stream(case)`: stream `2^32 + case` of `DEFAULT_SEED`, far above the engine's own stream indices; some tests also use `Rng::default()` | The tests only need known, repeatable generators; streams derived from the default seed give each case its own sequence, the same on every run. |
| `uci::engine` tests of the `Seed` option | implemented | No | `Rng::stream(1..=6, stream::STRENGTH)`, the seeds the tests give the option | They check that a chosen `Seed` reproduces the engine's own draws, so they must use the option's values, not the default seed. |
| Test fixtures (golden values in `rng` and `zobrist` tests) | implemented | No | `DEFAULT_SEED` and its streams | Tests must give the same result on every run. |

Stream indices of the default seed are registered in `rng::stream` so that
subsystems never share or shift each other's numbers:

| Stream index | Constant | Used by |
| --- | --- | --- |
| 0 | `stream::ZOBRIST` | Zobrist keys |
| 1 | `stream::MAGICS` | Magic-number search |
| 2 | `stream::STRENGTH` | Strength-affecting randomness. Taken from the seed the UCI option `Seed` selects (system entropy, or the seed given), not necessarily the default seed. |

Planned uses (each gets a real row when it lands): datagen opening
randomisation (milestone 9, not strength-affecting; per-game derived seeds).

## Design decisions

| Decision | Reason |
| --- | --- |
| Library + thin binary | One implementation shared by the binary, integration tests and tools. |
| Zero-dependency engine crate | Small binary, fast builds, no licence or supply-chain surprises, stable generated tables. |
| Tool crates separate from the engine | Heavy dependencies (zstd, trainers) never reach the engine binary. |
| In-house SplitMix64 | Cross-platform, upgrade-proof determinism; see "RNG design". |
| `panic = "abort"` in release | Smaller, faster code; an engine cannot usefully recover from a panic mid-search. |
| `Vyvræl` in UCI, `vyvrael` as crate/binary name | Cargo package names must be ASCII; the UCI name can be UTF-8, with `Vyvrael` as fallback. |
