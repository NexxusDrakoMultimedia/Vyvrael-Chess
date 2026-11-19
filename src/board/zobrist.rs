// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Zobrist keys: the random numbers a position's hash is built from.
//!
//! A position's hash is the XOR of one key per feature it has: each piece on
//! its square, the castling rights, the en passant file and the side to
//! move. XOR is its own inverse, so making a move updates the hash by XORing
//! out the features that disappear and XORing in the ones that appear.
//!
//! The 781 base keys are generated at compile time from the seeded [`Rng`],
//! stream [`stream::ZOBRIST`] of [`DEFAULT_SEED`], so they are identical in
//! every build on every platform. Generation order: piece keys (white then
//! black; pawn to king; a1 to h8), the four castling flag keys (`K`, `Q`,
//! `k`, `q`), the eight en passant file keys (a to h), then the side key.

use super::{CastlingRights, Piece, Square};
use crate::rng::{DEFAULT_SEED, Rng, stream};

/// The full key tables.
struct Keys {
    /// One key per `[colour][piece kind][square]`.
    piece: [[[u64; 64]; 6]; 2],
    /// One key per castling-rights value: the XOR of its flags' base keys.
    castling: [u64; CastlingRights::COUNT],
    /// One key per en passant file.
    en_passant: [u64; 8],
    /// XORed in when Black is to move.
    black_to_move: u64,
}

static KEYS: Keys = generate();

const fn generate() -> Keys {
    let mut rng = Rng::stream(DEFAULT_SEED, stream::ZOBRIST);

    let mut piece = [[[0u64; 64]; 6]; 2];
    let mut color = 0;
    while color < 2 {
        let mut kind = 0;
        while kind < 6 {
            let mut square = 0;
            while square < 64 {
                piece[color][kind][square] = rng.next_u64();
                square += 1;
            }
            kind += 1;
        }
        color += 1;
    }

    let mut flags = [0u64; 4];
    let mut i = 0;
    while i < 4 {
        flags[i] = rng.next_u64();
        i += 1;
    }
    // Composing each value's key from per-flag keys gives
    // `castling[a] ^ castling[b] == castling[a ^ b]` and `castling[0] == 0`.
    let mut castling = [0u64; CastlingRights::COUNT];
    let mut rights = 0;
    while rights < CastlingRights::COUNT {
        let mut bit = 0;
        while bit < 4 {
            if rights & (1 << bit) != 0 {
                castling[rights] ^= flags[bit];
            }
            bit += 1;
        }
        rights += 1;
    }

    let mut en_passant = [0u64; 8];
    let mut file = 0;
    while file < 8 {
        en_passant[file] = rng.next_u64();
        file += 1;
    }

    Keys {
        piece,
        castling,
        en_passant,
        black_to_move: rng.next_u64(),
    }
}

/// The key for `piece` standing on `square`.
#[must_use]
pub fn piece(piece: Piece, square: Square) -> u64 {
    KEYS.piece[piece.color.index()][piece.kind.index()][square.index()]
}

/// The key for a set of castling rights (zero for no rights).
#[must_use]
pub fn castling(rights: CastlingRights) -> u64 {
    KEYS.castling[rights.index()]
}

/// The key for an en passant target on `file` (`0..8`).
///
/// # Panics
///
/// Panics if `file` is 8 or more.
#[must_use]
pub fn en_passant(file: u8) -> u64 {
    KEYS.en_passant[usize::from(file)]
}

/// The key XORed into the hash when Black is to move.
#[must_use]
pub fn black_to_move() -> u64 {
    KEYS.black_to_move
}

#[cfg(test)]
mod tests {
    use super::super::{Color, PieceKind};
    use super::*;

    fn all_base_keys() -> Vec<u64> {
        let mut keys = Vec::new();
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                for square in Square::all() {
                    keys.push(piece(Piece::new(color, kind), square));
                }
            }
        }
        keys.extend([
            castling(CastlingRights::WHITE_KINGSIDE),
            castling(CastlingRights::WHITE_QUEENSIDE),
            castling(CastlingRights::BLACK_KINGSIDE),
            castling(CastlingRights::BLACK_QUEENSIDE),
        ]);
        keys.extend((0..8).map(en_passant));
        keys.push(black_to_move());
        keys
    }

    /// The base keys are exactly the first 781 outputs of the Zobrist
    /// stream, in the documented order.
    #[test]
    fn keys_are_the_zobrist_stream_in_order() {
        let mut rng = Rng::stream(DEFAULT_SEED, stream::ZOBRIST);
        let expected: Vec<u64> = (0..781).map(|_| rng.next_u64()).collect();
        assert_eq!(all_base_keys(), expected);
    }

    #[test]
    fn base_keys_are_distinct_and_non_zero() {
        let mut keys = all_base_keys();
        assert_eq!(keys.len(), 781);
        assert!(keys.iter().all(|&key| key != 0));
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 781);
    }

    /// Golden values computed with an independent implementation of the
    /// generator, pinning both the keys and their order.
    #[test]
    fn golden_keys() {
        let white_pawn = Piece::new(Color::White, PieceKind::Pawn);
        let black_king = Piece::new(Color::Black, PieceKind::King);
        assert_eq!(piece(white_pawn, Square::A1), 0xCCC9_1CE0_CC50_1A61);
        assert_eq!(piece(white_pawn, Square::B1), 0x0A28_A258_6506_921E);
        assert_eq!(piece(black_king, Square::H8), 0xF33E_30C3_C8F3_D90E);
        assert_eq!(
            castling(CastlingRights::WHITE_KINGSIDE),
            0x21F2_BE2A_CB32_EFB6
        );
        assert_eq!(
            castling(CastlingRights::WHITE_QUEENSIDE),
            0x7F0D_2DE1_04CA_6606
        );
        assert_eq!(
            castling(CastlingRights::BLACK_KINGSIDE),
            0xE334_9ACB_5C35_49E7
        );
        assert_eq!(
            castling(CastlingRights::BLACK_QUEENSIDE),
            0xC62B_8997_7ECF_B56F
        );
        assert_eq!(en_passant(0), 0xDAB7_B245_FFF8_247C);
        assert_eq!(en_passant(7), 0x048F_A3E8_6F14_7824);
        assert_eq!(black_to_move(), 0x7C40_8179_CF90_6E11);
    }

    /// Builds a rights value from four flag bits (bit 0 = `K` ... bit 3 = `q`).
    fn rights_from_bits(bits: u8) -> CastlingRights {
        let flags = [
            CastlingRights::WHITE_KINGSIDE,
            CastlingRights::WHITE_QUEENSIDE,
            CastlingRights::BLACK_KINGSIDE,
            CastlingRights::BLACK_QUEENSIDE,
        ];
        let mut rights = CastlingRights::NONE;
        for (i, flag) in flags.into_iter().enumerate() {
            if bits & (1 << i) != 0 {
                rights = rights.with(flag);
            }
        }
        rights
    }

    /// The key of a union of disjoint rights is the XOR of their keys, so
    /// losing a right can be hashed by XORing out just that right's key.
    #[test]
    fn castling_keys_compose_by_xor() {
        assert_eq!(castling(CastlingRights::NONE), 0);
        for a in 0..16u8 {
            for b in (0..16u8).filter(|b| a & b == 0) {
                let (ra, rb) = (rights_from_bits(a), rights_from_bits(b));
                assert_eq!(castling(ra.with(rb)), castling(ra) ^ castling(rb));
            }
        }
        let mut keys: Vec<u64> = (0..16)
            .map(|bits| castling(rights_from_bits(bits)))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 16, "all 16 rights values must hash differently");
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn en_passant_file_out_of_range_panics() {
        let _ = en_passant(8);
    }
}
