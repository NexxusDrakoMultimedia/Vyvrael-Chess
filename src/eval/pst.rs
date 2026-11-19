// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Piece-square tables: a bonus or penalty for each piece on each square.
//!
//! The tables say where each kind of piece usually stands well. They were
//! written for this project from general chess principles, leaning towards
//! its style (active pieces, advanced pawns, pressure towards the enemy
//! king). They are a reasoned starting point, not tuned values; see known
//! issue KI-8 in `docs/ROADMAP.md`.

use super::{Score, Tapered};
use crate::board::{Color, Piece, PieceKind, Position, Square};

/// A table written the way a board is printed, from White's side: the first
/// row is rank 8 and the first column is the a-file.
type Visual = [[Score; 8]; 8];

/// Combines a middlegame and an endgame [`Visual`] table into one table
/// indexed by square (`a1 = 0`), from White's point of view.
const fn from_visual(mg: Visual, eg: Visual) -> [Tapered; Square::COUNT] {
    let mut table = [Tapered::ZERO; Square::COUNT];
    let mut row = 0;
    while row < 8 {
        let mut file = 0;
        while file < 8 {
            // Row 0 of the visual table is rank 8, which is rank index 7.
            let square = (7 - row) * 8 + file;
            table[square] = Tapered::new(mg[row][file], eg[row][file]);
            file += 1;
        }
        row += 1;
    }
    table
}

/// For piece kinds whose table has not been written yet.
const NO_TABLE: [Tapered; Square::COUNT] = [Tapered::ZERO; Square::COUNT];

/// Pawns. Middlegame: take the centre, and do not leave the d- and e-pawns
/// at home. Endgame: every step towards promotion is worth more than the
/// last, whatever the file.
#[rustfmt::skip]
const PAWN: [Tapered; Square::COUNT] = from_visual(
    [
        [  0,   0,   0,   0,   0,   0,   0,   0],
        [ 50,  50,  50,  50,  50,  50,  50,  50],
        [ 12,  16,  22,  30,  30,  22,  16,  12],
        [  6,   8,  14,  26,  26,  14,   8,   6],
        [  2,   4,  10,  24,  24,  10,   4,   2],
        [  4,   2,  -2,   4,   4,  -2,   2,   4],
        [  0,   4,   6, -16, -16,   6,   4,   0],
        [  0,   0,   0,   0,   0,   0,   0,   0],
    ],
    [
        [  0,   0,   0,   0,   0,   0,   0,   0],
        [ 90,  90,  90,  90,  90,  90,  90,  90],
        [ 56,  56,  56,  56,  56,  56,  56,  56],
        [ 32,  32,  32,  32,  32,  32,  32,  32],
        [ 18,  18,  18,  18,  18,  18,  18,  18],
        [ 10,  10,  10,  10,  10,  10,  10,  10],
        [  6,   6,   6,   6,   6,   6,   6,   6],
        [  0,   0,   0,   0,   0,   0,   0,   0],
    ],
);

/// Knights. A knight needs to be near the action: strong in the centre and on
/// advanced outposts, poor on the rim and worst in a corner.
#[rustfmt::skip]
const KNIGHT: [Tapered; Square::COUNT] = from_visual(
    [
        [-50, -30, -20, -20, -20, -20, -30, -50],
        [-30, -10,   5,  10,  10,   5, -10, -30],
        [-15,  10,  25,  30,  30,  25,  10, -15],
        [-10,  10,  22,  32,  32,  22,  10, -10],
        [-12,   5,  18,  25,  25,  18,   5, -12],
        [-18,   0,  12,  14,  14,  12,   0, -18],
        [-30, -15,   0,   2,   2,   0, -15, -30],
        [-50, -25, -20, -18, -18, -20, -25, -50],
    ],
    [
        [-40, -25, -15, -12, -12, -15, -25, -40],
        [-25,  -8,   0,   5,   5,   0,  -8, -25],
        [-15,   0,  12,  18,  18,  12,   0, -15],
        [-12,   5,  18,  24,  24,  18,   5, -12],
        [-12,   5,  18,  24,  24,  18,   5, -12],
        [-15,   0,  10,  16,  16,  10,   0, -15],
        [-25, -10,   0,   4,   4,   0, -10, -25],
        [-40, -25, -15, -12, -12, -15, -25, -40],
    ],
);

/// Bishops. Good on long diagonals and active central squares, including the
/// fianchetto squares b2 and g2; poor on the back rank where they are undeveloped.
#[rustfmt::skip]
const BISHOP: [Tapered; Square::COUNT] = from_visual(
    [
        [-20, -10, -10, -10, -10, -10, -10, -20],
        [ -8,   6,   4,   4,   4,   4,   6,  -8],
        [ -4,   8,  14,  12,  12,  14,   8,  -4],
        [ -2,  10,  12,  18,  18,  12,  10,  -2],
        [ -2,  12,  14,  18,  18,  14,  12,  -2],
        [ -4,  14,  12,  10,  10,  12,  14,  -4],
        [ -6,  16,   6,   4,   4,   6,  16,  -6],
        [-20,  -8, -14, -10, -10, -14,  -8, -20],
    ],
    [
        [-14,  -8,  -6,  -4,  -4,  -6,  -8, -14],
        [ -8,   0,   2,   4,   4,   2,   0,  -8],
        [ -6,   2,   8,  10,  10,   8,   2,  -6],
        [ -4,   4,  10,  14,  14,  10,   4,  -4],
        [ -4,   4,  10,  14,  14,  10,   4,  -4],
        [ -6,   2,   8,  10,  10,   8,   2,  -6],
        [ -8,   0,   2,   4,   4,   2,   0,  -8],
        [-14,  -8,  -6,  -4,  -4,  -6,  -8, -14],
    ],
);

/// Rooks. Strongest on the seventh rank, where they attack pawns and confine
/// the king, and better on the central files than on the edge. In the endgame
/// only how far up the board the rook stands matters.
#[rustfmt::skip]
const ROOK: [Tapered; Square::COUNT] = from_visual(
    [
        [  6,   8,  10,  14,  14,  10,   8,   6],
        [ 18,  22,  24,  26,  26,  24,  22,  18],
        [  0,   4,   8,  10,  10,   8,   4,   0],
        [ -4,   0,   4,   8,   8,   4,   0,  -4],
        [ -6,  -2,   2,   6,   6,   2,  -2,  -6],
        [ -8,  -4,   0,   4,   4,   0,  -4,  -8],
        [-10,  -4,   0,   4,   4,   0,  -4, -10],
        [ -6,  -2,   6,  12,  12,   6,  -2,  -6],
    ],
    [
        [  8,   8,   8,   8,   8,   8,   8,   8],
        [ 14,  14,  14,  14,  14,  14,  14,  14],
        [  6,   6,   6,   6,   6,   6,   6,   6],
        [  4,   4,   4,   4,   4,   4,   4,   4],
        [  2,   2,   2,   2,   2,   2,   2,   2],
        [  0,   0,   0,   0,   0,   0,   0,   0],
        [ -2,  -2,  -2,  -2,  -2,  -2,  -2,  -2],
        [ -4,  -4,  -4,  -4,  -4,  -4,  -4,  -4],
    ],
);

/// Queens. Mildly better in the centre and poor in the corners. The
/// middlegame values are kept small so the queen is not drawn out early; in
/// the endgame a centralised queen dominates the board.
#[rustfmt::skip]
const QUEEN: [Tapered; Square::COUNT] = from_visual(
    [
        [-10,  -4,   0,   2,   2,   0,  -4, -10],
        [ -4,   4,   8,   8,   8,   8,   4,  -4],
        [ -2,   6,  10,  12,  12,  10,   6,  -2],
        [  0,   6,  10,  14,  14,  10,   6,   0],
        [ -2,   4,   8,  12,  12,   8,   4,  -2],
        [ -4,   4,   6,   6,   6,   6,   4,  -4],
        [ -8,  -2,   4,   2,   2,   4,  -2,  -8],
        [-14, -10,  -6,   0,   0,  -6, -10, -14],
    ],
    [
        [-16,  -8,  -4,   0,   0,  -4,  -8, -16],
        [ -8,   0,   6,  10,  10,   6,   0,  -8],
        [ -4,   6,  14,  18,  18,  14,   6,  -4],
        [  0,  10,  18,  24,  24,  18,  10,   0],
        [  0,  10,  18,  24,  24,  18,  10,   0],
        [ -4,   6,  14,  18,  18,  14,   6,  -4],
        [ -8,   0,   6,  10,  10,   6,   0,  -8],
        [-16,  -8,  -4,   0,   0,  -4,  -8, -16],
    ],
);

/// All tables, indexed by [`PieceKind::index`], from White's point of view.
static TABLES: [[Tapered; Square::COUNT]; PieceKind::COUNT] =
    [PAWN, KNIGHT, BISHOP, ROOK, QUEEN, NO_TABLE];

/// The piece-square value of `piece` standing on `square`, as a bonus for
/// the piece's own side.
///
/// The tables are written for White. A black piece uses the same table with
/// the board flipped top to bottom, so both sides are treated identically.
#[must_use]
pub fn piece_square(piece: Piece, square: Square) -> Tapered {
    let index = match piece.color {
        Color::White => square.index(),
        // XOR with 56 flips the rank and keeps the file: a1 <-> a8.
        Color::Black => square.index() ^ 56,
    };
    TABLES[piece.kind.index()][index]
}

/// The piece-square total of White's pieces minus that of Black's.
#[must_use]
pub fn piece_square_balance(pos: &Position) -> Tapered {
    let mut total = Tapered::ZERO;
    for square in pos.occupied() {
        if let Some(piece) = pos.piece_at(square) {
            match piece.color {
                Color::White => total += piece_square(piece, square),
                Color::Black => total -= piece_square(piece, square),
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white(kind: PieceKind, square: Square) -> Tapered {
        piece_square(Piece::new(Color::White, kind), square)
    }

    fn black(kind: PieceKind, square: Square) -> Tapered {
        piece_square(Piece::new(Color::Black, kind), square)
    }

    fn fen(text: &str) -> Position {
        Position::from_fen(text).unwrap()
    }

    #[test]
    fn visual_tables_read_like_a_board() {
        // The second row of the pawn table is rank 7; the seventh is rank 2.
        assert_eq!(white(PieceKind::Pawn, Square::A7), Tapered::new(50, 90));
        assert_eq!(white(PieceKind::Pawn, Square::E2), Tapered::new(-16, 6));
        assert_eq!(white(PieceKind::Pawn, Square::E4), Tapered::new(24, 18));
        assert_eq!(white(PieceKind::Pawn, Square::C3), Tapered::new(-2, 10));
    }

    /// Black sees the same table from its own side of the board.
    #[test]
    fn black_uses_the_table_flipped_top_to_bottom() {
        for kind in PieceKind::ALL {
            for square in Square::all() {
                let mirrored = Square::new(square.file(), 7 - square.rank());
                assert_eq!(
                    black(kind, square),
                    white(kind, mirrored),
                    "{kind:?} {square}"
                );
            }
        }
        assert_eq!(black(PieceKind::Pawn, Square::E5), Tapered::new(24, 18));
        assert_eq!(black(PieceKind::Pawn, Square::A2), Tapered::new(50, 90));
    }

    /// Every table treats the queenside and the kingside alike.
    #[test]
    fn tables_are_symmetric_left_to_right() {
        for kind in PieceKind::ALL {
            for square in Square::all() {
                let mirrored = Square::new(7 - square.file(), square.rank());
                assert_eq!(
                    white(kind, square),
                    white(kind, mirrored),
                    "{kind:?} {square}"
                );
            }
        }
    }

    #[test]
    fn pawn_table_shape() {
        // No pawn ever stands on the first or last rank.
        for file in 0..8 {
            assert_eq!(white(PieceKind::Pawn, Square::new(file, 0)), Tapered::ZERO);
            assert_eq!(white(PieceKind::Pawn, Square::new(file, 7)), Tapered::ZERO);
        }
        // In the endgame each step forward is worth more than the last.
        for file in 0..8 {
            let mut previous_gain = 0;
            for rank in 2..7 {
                let here = white(PieceKind::Pawn, Square::new(file, rank)).eg;
                let below = white(PieceKind::Pawn, Square::new(file, rank - 1)).eg;
                assert!(here - below > previous_gain, "file {file} rank {rank}");
                previous_gain = here - below;
            }
        }
        // In the middlegame the centre pawns want to advance two squares.
        let mg = |square| white(PieceKind::Pawn, square).mg;
        assert!(mg(Square::E4) > mg(Square::E3) && mg(Square::E3) > mg(Square::E2));
        assert!(mg(Square::D4) > mg(Square::A4));
    }

    #[test]
    fn knights_and_bishops_want_to_be_developed_and_central() {
        for kind in [PieceKind::Knight, PieceKind::Bishop] {
            let at = |square| white(kind, square);
            // Centre beats rim beats corner, in both phases.
            for value in [|t: Tapered| t.mg, |t: Tapered| t.eg] {
                assert!(value(at(Square::E5)) > value(at(Square::A5)), "{kind:?}");
                assert!(value(at(Square::A5)) > value(at(Square::A1)), "{kind:?}");
            }
            // Every home square is worse than a natural developing square.
            let home = if kind == PieceKind::Knight {
                Square::G1
            } else {
                Square::F1
            };
            let developed = if kind == PieceKind::Knight {
                Square::F3
            } else {
                Square::C4
            };
            assert!(at(developed).mg > at(home).mg + 20, "{kind:?}");
        }
        // A knight is worse off in the corner than a bishop is.
        assert!(white(PieceKind::Knight, Square::A1).mg < white(PieceKind::Bishop, Square::A1).mg);
        // The fianchetto square is one of the bishop's best second-rank squares.
        assert!(white(PieceKind::Bishop, Square::G2).mg > white(PieceKind::Bishop, Square::E2).mg);
    }

    #[test]
    fn rooks_want_the_seventh_rank_and_central_files() {
        let at = |square| white(PieceKind::Rook, square);
        for file in 0..8 {
            let seventh = at(Square::new(file, 6));
            for rank in (0..8).filter(|&rank| rank != 6) {
                assert!(
                    seventh.mg > at(Square::new(file, rank)).mg,
                    "file {file} rank {rank}"
                );
                assert!(
                    seventh.eg > at(Square::new(file, rank)).eg,
                    "file {file} rank {rank}"
                );
            }
        }
        assert!(at(Square::D1).mg > at(Square::A1).mg);
        // Black's seventh rank is the second rank of the board.
        assert_eq!(black(PieceKind::Rook, Square::D2), at(Square::D7));
    }

    #[test]
    fn the_queen_is_not_drawn_out_early_but_centralises_late() {
        let at = |square| white(PieceKind::Queen, square);
        // Leaving d1 for the centre gains little in the middlegame...
        let early_gain = at(Square::D4).mg - at(Square::D1).mg;
        assert!(early_gain > 0 && early_gain < 15, "{early_gain}");
        // ...and less than developing a knight does.
        let knight_gain =
            white(PieceKind::Knight, Square::F3).mg - white(PieceKind::Knight, Square::G1).mg;
        assert!(early_gain < knight_gain);
        // In the endgame the centre is clearly best and the corner worst.
        assert!(at(Square::D4).eg > at(Square::D1).eg + 20);
        assert!(at(Square::A1).eg < at(Square::D1).eg);
        assert!(Square::all().all(|square| at(square).mg.abs() <= 14));
    }

    #[test]
    fn balance_is_white_minus_black() {
        assert_eq!(piece_square_balance(&Position::startpos()), Tapered::ZERO);
        // One white pawn on e4 against one black pawn on e7 (its home square).
        let pos = fen("4k3/4p3/8/8/4P3/8/8/4K3 w - - 0 1");
        let expected = white(PieceKind::Pawn, Square::E4) - black(PieceKind::Pawn, Square::E7);
        assert_eq!(piece_square_balance(&pos), expected);
        assert_eq!(expected, Tapered::new(24 + 16, 18 - 6));
    }

    /// Swapping the colours and flipping the board negates the balance.
    #[test]
    fn balance_is_antisymmetric_under_colour_swap() {
        let pairs = [
            (
                "4k3/4p3/8/8/4P3/8/8/4K3 w - - 0 1",
                "4k3/8/8/4p3/8/8/4P3/4K3 w - - 0 1",
            ),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                "r3k2r/pppbbppp/2n2q1P/1P2p3/3pn3/BN2PNP1/P1PPQPB1/R3K2R w KQkq - 0 1",
            ),
        ];
        for (position, swapped) in pairs {
            assert_eq!(
                piece_square_balance(&fen(position)),
                -piece_square_balance(&fen(swapped)),
                "{position}"
            );
        }
    }
}
