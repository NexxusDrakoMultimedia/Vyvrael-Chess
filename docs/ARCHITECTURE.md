# Architecture

This document describes how Vyvræl is put together and why. It is updated in
the same commit as any structural change. Sections marked *planned* describe
intent for a later milestone, not code that exists.

## Workspace layout

```
Cargo.toml              workspace root + engine package `vyvrael`
src/lib.rs              engine library (all engine logic lives here)
src/main.rs             engine binary (thin wrapper over the library)
tools/vyvrael-data/     data and database tooling (stub; milestone 7)
tools/vyvrael-train/    NNUE training (stub; milestone 8)
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
| `rng` | implemented | The only source of randomness: `Rng` (SplitMix64) with `next_u64`, `next_u32`, `next_below` and `Rng::stream`; `DEFAULT_SEED`; `mix64`. |
| `board` | implemented | Board representation: `Color`, `Square`, `PieceKind`, `Piece`, `Bitboard`, `CastlingRights`, `Move`, `Position`, FEN parsing, validation and serialisation, Zobrist keys (`board::zobrist`), the position hash, make/unmake and coordinate-notation move parsing. |
| `attacks` | implemented | Attack tables: knight, king and pawn tables, slider attacks by ray scanning, and magic bitboards for bishops, rooks and queens. |
| `movegen` | implemented | Move generation: `MoveList`, `pseudo_legal_moves` (all pieces, pawns and castling) and `legal_moves`. |
| `cli` | implemented | Command line of the binary: engine identification, `perft`, `help`. |
| `perft` | implemented | `perft`, `divide` and `perft_copy_make`: node counting for testing move generation. |
| `eval` | in progress (milestone 3) | Static evaluation behind the `Evaluator` trait. |
| search | planned (milestone 4) | Alpha-beta search and its heuristics. |
| UCI | planned (milestone 5) | Protocol loop; search on its own thread. |

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
- **FEN validation** (`fen.rs`): after parsing, a position is accepted only
  if each side has exactly one king, no pawn is on the first or eighth rank,
  no side has more material than a game can produce (pawns plus pieces beyond
  the starting set must not exceed eight),
  every claimed castling right has its king and rook on their home squares,
  the side not to move is not in check, and any en passant square is consistent with a double push by the side that
  just moved (correct rank, target square and the pawn's starting square
  empty, that side's pawn directly beyond the target). These are the
  invariants make/unmake relies on. Rejecting rather than repairing means a
  bad input is noticed instead of silently changed; an opt-in repair mode is
  planned for later (known issue KI-2). A position where the side not to
  move is in check is rejected too (`FenError::OpponentInCheck`), since the
  side to move could capture the king and no legal game reaches that. An en passant
  square is kept whenever a double push happened, whether or not a capture is
  actually possible.
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
  value with no heap allocation; search will keep its own stack. `Undo` is
  `#[must_use]`. Both styles are supported: in-place make/unmake, and
  copy-make (clone the position, about 200 bytes, and make the move on the
  clone). Measured with perft in milestone 2, copy-make was about 5% faster
  (numbers in [TESTING.md](TESTING.md)). That is too small and too
  perft-specific to settle the question, so the choice for search is left to
  milestone 4, to be made on search speed.
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
  match the result against `movegen::legal_moves`. Doing that for UCI input is
  due in milestone 5 (known issue KI-3).

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
  - *When.* The tables are built on first use behind a `LazyLock`, not at
    compile time: the search is too heavy for `const` evaluation. It takes
    about 0.1 s in a release build (known issue KI-7).
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
  skip moves after which the king could be captured, so they exercise exactly
  the code search will use. `perft_copy_make` counts the same tree by cloning
  the position for each move; it exists only to measure the two styles
  against each other (see [TESTING.md](TESTING.md)).

- **Command line** (`cli.rs`): `cli::run(args, out, err)` implements the
  binary's commands against any pair of writers, so tests run it on in-memory
  buffers and `main.rs` is only a few lines. With no arguments the binary
  identifies itself on stderr, leaving stdout empty for the future UCI
  protocol. `perft <depth> [--divide | --copy-make] [FEN]` prints node counts on stdout and
  timing on stderr. Exit status 2 means the command line was not understood.

## Evaluation

In progress (milestone 3). `src/eval/` gives a position a score without
searching.

- **Scores** are `eval::Score`, an `i32` in centipawns (100 is about one
  pawn).
- **`Evaluator`** (`eval/mod.rs`) is the trait search will call:
  `evaluate(&self, pos) -> Score` returns the score **from the side to move's
  point of view**, which is what negamax needs, and `name()` identifies the
  evaluator. Search holds it as a trait object, so the hand-crafted evaluation
  and a later neural network are interchangeable and can be played against
  each other. Evaluators add up "White minus Black" and convert with
  `eval::relative_to`.
- The trait takes the whole position each time. A network that updates
  incrementally on make/unmake (milestone 8) will need notification hooks;
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
  - **Tables written so far:** pawns (middlegame: take the centre and do not
    leave the d- and e-pawns at home; endgame: each step towards promotion is
    worth more than the last).

## How search, evaluation and UCI fit together (planned)

The UCI front end owns the current position and the option values. On `go`
it hands a copy of the position and the search limits to a search thread, so
the main thread keeps reading stdin and can act on `stop`. Search calls the
evaluation through a trait, so the hand-crafted evaluation and the NNUE
evaluation are interchangeable and can be compared by SPRT. Only the UCI
layer writes to stdout.

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
  sharing one generator across threads. The stream's starting state is
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
  strength-affecting randomness when the UCI option `DeterministicRNG` is
  `false`. That path arrives with the UCI milestone and logs the seed it chose with `info string`.

### Table of randomness uses

Every use of randomness is listed here and classified. "Strength-affecting"
means the randomness can change how strongly or how differently the engine
plays; such uses are non-deterministic by default with `DeterministicRNG` /
`RngSeed` as the opt-in. Everything else is always deterministic.

| Use | Status | Strength-affecting? | Seed | Justification |
| --- | --- | --- | --- | --- |
| Zobrist keys (`board::zobrist`) | implemented | No | `Rng::stream(DEFAULT_SEED, stream::ZOBRIST)`, stream 0, at compile time | The keys only label positions. Any well-mixed set works equally well, and fixed keys keep hashes, tests and transposition-table behaviour reproducible. |
| Magic-number search (`attacks::sliders`) | implemented | No | `Rng::stream(DEFAULT_SEED, stream::MAGICS)`, stream 1, on first use | Any valid magic number gives exactly the same attack sets, so the choice cannot affect play. A fixed stream makes the tables identical in every run. |
| Randomised test inputs (`attacks::sliders` tests: 20,000 random occupancies and squares) | implemented | No | `Rng::default()`, the default seed | The samples only choose which cases a test compares against the scanning reference; a fixed seed makes the test reproducible. |
| Test fixtures (golden values in `rng` and `zobrist` tests) | implemented | No | `DEFAULT_SEED` and its streams | Tests must give the same result on every run. |

Stream indices of the default seed are registered in `rng::stream` so that
subsystems never share or shift each other's numbers:

| Stream index | Constant | Used by |
| --- | --- | --- |
| 0 | `stream::ZOBRIST` | Zobrist keys |
| 1 | `stream::MAGICS` | Magic-number search |

Planned uses (each gets a real row when it lands): datagen opening randomisation (milestone 8, not
strength-affecting; per-game derived seeds), strength limiting through
`UCI_LimitStrength` / `UCI_Elo` (milestone 5, **strength-affecting**:
non-deterministic by default, with `DeterministicRNG` / `RngSeed` as the
opt-in).

## Design decisions

| Decision | Reason |
| --- | --- |
| Library + thin binary | One implementation shared by the binary, integration tests and tools. |
| Zero-dependency engine crate | Small binary, fast builds, no licence or supply-chain surprises, stable generated tables. |
| Tool crates separate from the engine | Heavy dependencies (zstd, trainers) never reach the engine binary. |
| In-house SplitMix64 | Cross-platform, upgrade-proof determinism; see "RNG design". |
| `panic = "abort"` in release | Smaller, faster code; an engine cannot usefully recover from a panic mid-search. |
| `Vyvræl` in UCI, `vyvrael` as crate/binary name | Cargo package names must be ASCII; the UCI name can be UTF-8, with `Vyvrael` as fallback. |
