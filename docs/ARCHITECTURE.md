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
| `attacks` | in progress (milestone 2) | Attack tables per piece type. |
| `movegen` | in progress (milestone 2) | Move generation. Implemented so far: `MoveList`, pseudo-legal moves for knights and kings. |
| perft | planned (milestone 2) | Node counting for testing move generation. |
| evaluation | planned (milestone 3) | Hand-crafted eval behind an interface a network can replace. |
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
  every claimed castling right has its king and rook on their home squares,
  and any en passant square is consistent with a double push by the side that
  just moved (correct rank, target square and the pawn's starting square
  empty, that side's pawn directly beyond the target). These are the
  invariants make/unmake relies on. Rejecting rather than repairing means a
  bad input is noticed instead of silently changed; an opt-in repair mode is
  planned for later (known issue KI-2). **Not validated yet:**
  that the side which just moved did not leave its own king in check; that
  needs attack generation and is planned for milestone 2. An en passant
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
  or out and also correct the en passant key when a pawn appears or
  disappears beside the en passant square; the FEN parser folds in the state
  fields. Internally they wrap `place` and `lift`, which touch only the piece
  keys and are what make/unmake and the FEN parser use. `Position::hash()` is
  constant time; `compute_hash()` rebuilds it from scratch and exists to
  initialise and to verify (`is_consistent()` compares the two). The hash
  covers placement, side to move, castling rights and the en passant file.
  The halfmove clock and fullmove number are not hashed.
- **En passant hashing rule:** the en passant file is hashed only when a pawn
  of the side to move attacks the en passant square. A double push that
  nobody can answer en passant then hashes the same as the identical position
  reached another way, which is what repetition detection and the
  transposition table need. The check is pseudo-legal: it ignores pins, so a
  capture that would expose the king still counts (known issue KI-1 in
  [ROADMAP.md](ROADMAP.md), to be tightened in milestone 2). `Position::en_passant()`
  itself always reports the square after a double push, as FEN does.
- **Make/unmake** (`makemove.rs`): `Position::make_move(mv)` plays a move in
  place and returns an `Undo`; `Position::unmake_move(mv, undo)` takes it
  back and restores the position exactly, including counters and hash. The
  `Undo` holds only what a move destroys and cannot be recomputed: the
  captured piece, the previous castling rights, en passant square and
  halfmove clock, and the previous hash. Returning it to the caller (rather
  than keeping a history stack inside `Position`) keeps `Position` a plain
  value with no heap allocation; search will keep its own stack. `Undo` is
  `#[must_use]`. In-place make/unmake was chosen over copy-make because the
  position is about 200 bytes and unmake touches only a few fields; this is
  unmeasured and will be decided on perft numbers in milestone 2 (known
  issue KI-4).
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
  moving like a knight parses as a quiet move); that needs move generation
  (milestone 2), after which UCI input should be matched against the legal
  move list; that is due in milestone 5 (known issue KI-3).

## Attacks and move generation

In progress (milestone 2). `src/attacks/` answers "which squares does a piece
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
    860 KiB), "plain" magics with a per-square offset.
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
  if this piece were gone?". `is_attacked_by`, `checkers` and `in_check` are
  built on it.

- **`MoveList`** (`movegen/list.rs`): a fixed array of 256 moves on the
  stack with a length, dereferencing to a slice. Generation runs millions of
  times per second in search, so it must not allocate. The largest known
  number of legal moves in any position is 218; 256 leaves room for
  pseudo-legal extras. Pushing past the capacity panics rather than
  corrupting memory.

- **Pseudo-legal generation** (`movegen/pseudo.rs`): `pseudo_legal_moves`
  lists every move that follows the pieces' movement rules, without yet
  asking whether it leaves the mover's king attacked. Each piece type has its
  own small generator. A move to an enemy-occupied square is a `Capture`,
  otherwise `Quiet`; squares holding the mover's own pieces are masked out
  first. **Generated so far:** knights, king steps (not castling).

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
| Test fixtures (golden values in `rng` and `zobrist` tests) | implemented | No | `DEFAULT_SEED` and its streams | Tests must give the same result on every run. |

Stream indices of the default seed are registered in `rng::stream` so that
subsystems never share or shift each other's numbers:

| Stream index | Constant | Used by |
| --- | --- | --- |
| 0 | `stream::ZOBRIST` | Zobrist keys |
| 1 | `stream::MAGICS` | Magic-number search |

Planned uses (each gets a real row when it lands): property tests (not strength-affecting), datagen opening randomisation (milestone 8, not
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
