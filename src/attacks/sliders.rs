// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Bishop, rook and queen attacks by magic-bitboard lookup.
//!
//! The tables are built once, on first use. The magic numbers are found by
//! trial with the seeded generator (`rng::stream::MAGICS`), so every build
//! and every run finds exactly the same numbers and builds the same tables.

use std::sync::LazyLock;

use super::magic::{bishop_relevant_mask, rook_relevant_mask, subsets};
use super::scan::{bishop_attacks_by_scan, rook_attacks_by_scan};
use crate::board::{Bitboard, Square};
use crate::rng::{DEFAULT_SEED, Rng, stream};

/// How to find one square's slice of the shared attack table.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// The relevant-occupancy mask.
    mask: u64,
    /// The magic multiplier.
    magic: u64,
    /// `64 - mask bits`: keeps the top bits of the product as the index.
    shift: u32,
    /// Where this square's slice starts in `Tables::attacks`.
    offset: usize,
}

impl Entry {
    const EMPTY: Entry = Entry {
        mask: 0,
        magic: 0,
        shift: 0,
        offset: 0,
    };

    fn index(&self, occupied: Bitboard) -> usize {
        let hashed = (occupied.0 & self.mask).wrapping_mul(self.magic) >> self.shift;
        self.offset + hashed as usize
    }
}

struct Tables {
    bishop: [Entry; Square::COUNT],
    rook: [Entry; Square::COUNT],
    /// All squares' slices, bishops first: 5,248 + 102,400 entries.
    attacks: Vec<Bitboard>,
}

static TABLES: LazyLock<Tables> = LazyLock::new(build_tables);

/// Finds a magic number for one square and returns it with the filled slice.
///
/// Candidates with few set bits work best, so each is the AND of three
/// random numbers. A candidate is accepted when no two blocker arrangements
/// with different attacks land in the same slot.
fn find_magic(
    mask: Bitboard,
    attacks_of: impl Fn(Bitboard) -> Bitboard,
    rng: &mut Rng,
) -> (u64, Vec<Bitboard>) {
    let shift = 64 - mask.count();
    let size = 1usize << mask.count();
    let reference: Vec<(u64, Bitboard)> = subsets(mask)
        .map(|occupied| (occupied.0, attacks_of(occupied)))
        .collect();

    let mut slice = vec![Bitboard::EMPTY; size];
    // `filled_in[i]` is the attempt that last wrote slot `i`, so the slice
    // does not have to be cleared between attempts.
    let mut filled_in = vec![0u32; size];
    let mut attempt = 0u32;
    loop {
        let magic = rng.next_u64() & rng.next_u64() & rng.next_u64();
        // Cheap filter: a good magic spreads the mask into the top byte.
        if (mask.0.wrapping_mul(magic) >> 56).count_ones() < 6 {
            continue;
        }
        attempt += 1;
        let collision = reference.iter().any(|&(occupied, attacks)| {
            let slot = (occupied.wrapping_mul(magic) >> shift) as usize;
            if filled_in[slot] != attempt {
                filled_in[slot] = attempt;
                slice[slot] = attacks;
                false
            } else {
                slice[slot] != attacks
            }
        });
        if !collision {
            // Clear slots left over from failed attempts.
            for (slot, &written) in slice.iter_mut().zip(&filled_in) {
                if written != attempt {
                    *slot = Bitboard::EMPTY;
                }
            }
            return (magic, slice);
        }
    }
}

fn build_tables() -> Tables {
    let mut rng = Rng::stream(DEFAULT_SEED, stream::MAGICS);
    let mut tables = Tables {
        bishop: [Entry::EMPTY; Square::COUNT],
        rook: [Entry::EMPTY; Square::COUNT],
        attacks: Vec::with_capacity(5_248 + 102_400),
    };
    // Order matters for reproducibility: all bishop squares a1..h8, then
    // all rook squares, drawing from one generator.
    for square in Square::all() {
        let mask = bishop_relevant_mask(square);
        let scan = |occupied| bishop_attacks_by_scan(square, occupied);
        let (magic, slice) = find_magic(mask, scan, &mut rng);
        tables.bishop[square.index()] = Entry {
            mask: mask.0,
            magic,
            shift: 64 - mask.count(),
            offset: tables.attacks.len(),
        };
        tables.attacks.extend(slice);
    }
    for square in Square::all() {
        let mask = rook_relevant_mask(square);
        let scan = |occupied| rook_attacks_by_scan(square, occupied);
        let (magic, slice) = find_magic(mask, scan, &mut rng);
        tables.rook[square.index()] = Entry {
            mask: mask.0,
            magic,
            shift: 64 - mask.count(),
            offset: tables.attacks.len(),
        };
        tables.attacks.extend(slice);
    }
    tables
}

/// The squares a bishop on `square` attacks when `occupied` holds all
/// pieces. The first piece on each diagonal is included whatever its colour.
#[must_use]
pub fn bishop_attacks(square: Square, occupied: Bitboard) -> Bitboard {
    let tables = &*TABLES;
    tables.attacks[tables.bishop[square.index()].index(occupied)]
}

/// The squares a rook on `square` attacks when `occupied` holds all pieces.
/// The first piece on each line is included whatever its colour.
#[must_use]
pub fn rook_attacks(square: Square, occupied: Bitboard) -> Bitboard {
    let tables = &*TABLES;
    tables.attacks[tables.rook[square.index()].index(occupied)]
}

/// The squares a queen on `square` attacks: a bishop's plus a rook's.
#[must_use]
pub fn queen_attacks(square: Square, occupied: Bitboard) -> Bitboard {
    bishop_attacks(square, occupied) | rook_attacks(square, occupied)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustive: for every square and every arrangement of relevant
    /// blockers (107,648 cases), the lookup equals the scan.
    #[test]
    fn lookups_equal_the_scan_for_every_relevant_occupancy() {
        for square in Square::all() {
            for occupied in subsets(bishop_relevant_mask(square)) {
                assert_eq!(
                    bishop_attacks(square, occupied),
                    bishop_attacks_by_scan(square, occupied),
                    "bishop {square}"
                );
            }
            for occupied in subsets(rook_relevant_mask(square)) {
                assert_eq!(
                    rook_attacks(square, occupied),
                    rook_attacks_by_scan(square, occupied),
                    "rook {square}"
                );
            }
        }
    }

    /// Random full-board occupancies, including pieces outside the masks and
    /// on the slider's own square.
    #[test]
    fn lookups_equal_the_scan_for_random_occupancies() {
        let mut rng = Rng::default();
        for _ in 0..20_000 {
            // AND-ing two draws gives a realistic density of about 16 pieces.
            let occupied = Bitboard(rng.next_u64() & rng.next_u64());
            let square = Square::from_index(rng.next_below(64) as u8).unwrap();
            assert_eq!(
                bishop_attacks(square, occupied),
                bishop_attacks_by_scan(square, occupied)
            );
            assert_eq!(
                rook_attacks(square, occupied),
                rook_attacks_by_scan(square, occupied)
            );
            assert_eq!(
                queen_attacks(square, occupied),
                bishop_attacks_by_scan(square, occupied) | rook_attacks_by_scan(square, occupied)
            );
        }
    }

    /// Pins the magic numbers the seeded search finds. These values were
    /// recorded from this implementation (they are regression pins, not an
    /// independent derivation): if they change, the search order, the
    /// generator or the stream changed, and the change must be deliberate.
    #[test]
    fn golden_magic_numbers() {
        let tables = &*TABLES;
        assert_eq!(tables.bishop[0].magic, 0x8402_0208_3104_0080);
        assert_eq!(tables.bishop[63].magic, 0x0110_1010_0840_4240);
        assert_eq!(tables.rook[0].magic, 0x0980_0058_A0C0_0080);
        assert_eq!(tables.rook[63].magic, 0x9087_0002_0088_2441);
        let fold = tables
            .bishop
            .iter()
            .chain(&tables.rook)
            .fold(0u64, |acc, entry| acc.rotate_left(7) ^ entry.magic);
        assert_eq!(fold, 0x24B1_B955_BE57_33BD);
    }

    #[test]
    fn table_layout() {
        let tables = &*TABLES;
        assert_eq!(tables.attacks.len(), 5_248 + 102_400);
        assert_eq!(tables.bishop[0].offset, 0);
        assert_eq!(tables.rook[0].offset, 5_248);
        for square in Square::all() {
            let bishop = tables.bishop[square.index()];
            assert_eq!(bishop.mask, bishop_relevant_mask(square).0);
            assert_eq!(bishop.shift, 64 - bishop.mask.count_ones());
            let rook = tables.rook[square.index()];
            assert_eq!(rook.mask, rook_relevant_mask(square).0);
            assert_eq!(rook.shift, 64 - rook.mask.count_ones());
        }
    }

    /// Building the tables again gives the same magic numbers: the search is
    /// a pure function of the seeded stream.
    #[test]
    fn magic_search_is_reproducible() {
        let again = build_tables();
        let tables = &*TABLES;
        for square in Square::all() {
            let i = square.index();
            assert_eq!(again.bishop[i].magic, tables.bishop[i].magic, "{square}");
            assert_eq!(again.rook[i].magic, tables.rook[i].magic, "{square}");
        }
        assert_eq!(again.attacks, tables.attacks);
    }
}
