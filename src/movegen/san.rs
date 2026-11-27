// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Nexxus Drako Multimedia

//! Reading a move written in standard algebraic notation (SAN), the notation
//! of game scores and of test suites: `Nf3`, `exd5`, `O-O`, `e8=Q+`.

use super::legal_moves;
use crate::board::{Move, MoveKind, PieceKind, Position, Square};

/// The legal move of `pos` that `text` names in standard algebraic
/// notation, or `None` if it names none or more than one.
///
/// The reading is forgiving about how the move is written, never about
/// which move it is: the text is taken apart into a piece, a destination,
/// whatever it says about the origin and the promotion piece, and the answer
/// is the one legal move that fits all of it. So the marks that files
/// disagree on do not matter (`+`, `#`, `!`, `?`, an `x` or its absence,
/// `e8=Q` or `e8Q`, `O-O` or `0-0`, more disambiguation than is needed, even
/// the long form `Ng1f3`), while a text that fits two legal moves, such as
/// `Nd2` with two knights able to go there, fits none.
///
/// Like [`find_legal_move`](super::find_legal_move) it only ever returns a
/// member of [`legal_moves`]: a move that would leave the king in check is
/// not found, and where one of two knights is pinned `Nd2` means the other.
#[must_use]
pub fn find_san_move(pos: &Position, text: &str) -> Option<Move> {
    let text = text.trim_end_matches(['+', '#', '!', '?']);
    let wanted = Wanted::read(text)?;
    let mut found = None;
    for &mv in &legal_moves(pos) {
        if wanted.fits(pos, mv) {
            if found.is_some() {
                return None;
            }
            found = Some(mv);
        }
    }
    found
}

/// What a SAN text says about its move.
enum Wanted {
    /// `O-O` (king's side) or `O-O-O`.
    Castle { king_side: bool },
    Move {
        piece: PieceKind,
        to: Square,
        /// The origin's file and rank (0 to 7), as far as the text gives them.
        file: Option<u8>,
        rank: Option<u8>,
        promotion: Option<PieceKind>,
    },
}

impl Wanted {
    fn read(text: &str) -> Option<Wanted> {
        match text {
            "O-O" | "0-0" => return Some(Wanted::Castle { king_side: true }),
            "O-O-O" | "0-0-0" => return Some(Wanted::Castle { king_side: false }),
            _ => {}
        }
        let mut letters: Vec<char> = text.chars().collect();
        // A promotion piece ends the text, with or without `=` before it.
        let promotion = match letters.last() {
            Some(&last) if last.is_ascii_uppercase() => {
                letters.pop();
                if letters.last() == Some(&'=') {
                    letters.pop();
                }
                Some(officer(last).filter(|kind| *kind != PieceKind::King)?)
            }
            _ => None,
        };
        // An upper-case letter in front is the piece; without one it is a
        // pawn (`bxc3` is a pawn's capture, `Bxc3` a bishop's).
        let piece = match letters.first() {
            Some(&first) if first.is_ascii_uppercase() => {
                letters.remove(0);
                officer(first)?
            }
            _ => PieceKind::Pawn,
        };
        letters.retain(|letter| !matches!(letter, 'x' | '-' | ':'));
        // The destination is the last two letters; what stands before it
        // is about the origin.
        let split = letters.len().checked_sub(2)?;
        let to: String = letters[split..].iter().collect();
        let to: Square = to.parse().ok()?;
        let (mut file, mut rank) = (None, None);
        for &letter in &letters[..split] {
            match letter {
                'a'..='h' if file.is_none() && rank.is_none() => file = Some(letter as u8 - b'a'),
                '1'..='8' if rank.is_none() => rank = Some(letter as u8 - b'1'),
                _ => return None,
            }
        }
        Some(Wanted::Move {
            piece,
            to,
            file,
            rank,
            promotion,
        })
    }

    fn fits(&self, pos: &Position, mv: Move) -> bool {
        match *self {
            Wanted::Castle { king_side } => {
                mv.kind()
                    == if king_side {
                        MoveKind::KingCastle
                    } else {
                        MoveKind::QueenCastle
                    }
            }
            Wanted::Move {
                piece,
                to,
                file,
                rank,
                promotion,
            } => {
                let from = mv.from();
                mv.to() == to
                    && pos.piece_at(from).map(|moving| moving.kind) == Some(piece)
                    && mv.promotion() == promotion
                    && file.is_none_or(|file| from.file() == file)
                    && rank.is_none_or(|rank| from.rank() == rank)
            }
        }
    }
}

/// The piece an upper-case SAN letter stands for. `P` is not one: a pawn is
/// written without a letter.
fn officer(letter: char) -> Option<PieceKind> {
    PieceKind::from_char(letter).filter(|kind| *kind != PieceKind::Pawn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::START_FEN;

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    /// The move `san` names in `fen`, in UCI notation, or `-` for none.
    fn read(fen: &str, san: &str) -> String {
        let pos = Position::from_fen(fen).unwrap();
        find_san_move(&pos, san).map_or_else(|| "-".to_owned(), |mv| mv.to_string())
    }

    #[test]
    fn pawn_and_piece_moves() {
        assert_eq!(read(START_FEN, "e4"), "e2e4");
        assert_eq!(read(START_FEN, "e3"), "e2e3");
        assert_eq!(read(START_FEN, "Nf3"), "g1f3");
        assert_eq!(read(START_FEN, "Na3"), "b1a3");
        for (san, uci) in [
            ("Bxa6", "e2a6"),
            ("dxe6", "d5e6"),
            ("Nxd7", "e5d7"),
            ("Qxf6", "f3f6"),
            ("gxh3", "g2h3"),
            ("Kd1", "e1d1"),
        ] {
            assert_eq!(read(KIWIPETE, san), uci, "{san}");
        }
        // A lower-case b is a file, an upper-case one a bishop.
        let both = "4k3/8/8/B7/8/2n5/1P6/4K3 w - - 0 1";
        assert_eq!(read(both, "bxc3"), "b2c3");
        assert_eq!(read(both, "Bxc3"), "a5c3");
        // En passant is a pawn capture like any other.
        assert_eq!(read("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2", "exd6"), "e5d6");
    }

    #[test]
    fn castling_either_way_it_is_written() {
        for (san, uci) in [
            ("O-O", "e1g1"),
            ("0-0", "e1g1"),
            ("O-O-O", "e1c1"),
            ("0-0-0", "e1c1"),
        ] {
            assert_eq!(read(KIWIPETE, san), uci, "{san}");
        }
        // Not when it is not legal: here the rook is not on h1.
        assert_eq!(read(START_FEN, "O-O"), "-");
        assert_eq!(read(KIWIPETE, "O-O-O-O"), "-");
    }

    #[test]
    fn promotions_need_their_piece() {
        let fen = "3r3k/4P3/8/8/8/8/8/4K3 w - - 0 1";
        assert_eq!(read(fen, "e8=Q"), "e7e8q");
        assert_eq!(read(fen, "e8Q"), "e7e8q");
        assert_eq!(read(fen, "e8=N+"), "e7e8n");
        assert_eq!(read(fen, "exd8=R#"), "e7d8r");
        // Without the piece four moves fit, so none is meant; a king is
        // not a piece to promote to.
        assert_eq!(read(fen, "e8"), "-");
        assert_eq!(read(fen, "e8=K"), "-");
    }

    #[test]
    fn disambiguation_and_its_absence() {
        // Knights on b1 and f3 can both go to d2; rooks on a1 and a5 to a3.
        let fen = "4k3/8/8/R7/8/5N2/8/RN2K3 w - - 0 1";
        assert_eq!(read(fen, "Nd2"), "-");
        assert_eq!(read(fen, "Nbd2"), "b1d2");
        assert_eq!(read(fen, "Nfd2"), "f3d2");
        assert_eq!(read(fen, "N1d2"), "b1d2");
        assert_eq!(read(fen, "Ra3"), "-");
        assert_eq!(read(fen, "R1a3"), "a1a3");
        assert_eq!(read(fen, "R5a3"), "a5a3");
        // More than is needed, up to the whole origin square, still fits.
        assert_eq!(read(fen, "Nb1d2"), "b1d2");
        assert_eq!(read(fen, "Ngh4"), "-");
        assert_eq!(read(fen, "Nfh4"), "f3h4");
        // Only legal moves count. Both knights could go to c2 or d5, but
        // the one on e3 is pinned by the bishop, so the other is meant.
        let pinned = "4k3/8/7b/8/1N6/4N3/8/2K5 w - - 0 1";
        assert_eq!(read(pinned, "Nc2"), "b4c2");
        assert_eq!(read(pinned, "Nd5"), "b4d5");
        assert_eq!(read(pinned, "Nec2"), "-");
        // Without the bishop the same text fits two moves.
        let free = "4k3/8/8/8/1N6/4N3/8/2K5 w - - 0 1";
        assert_eq!(read(free, "Nc2"), "-");
    }

    #[test]
    fn marks_are_ignored_and_nonsense_is_nothing() {
        for san in ["e4+", "e4#", "e4!?", "e4??", "e2e4", "e2-e4"] {
            assert_eq!(read(START_FEN, san), "e2e4", "{san}");
        }
        for san in [
            "", "x", "e", "4", "e9", "i4", "Zf3", "Pe4", "nf3", "Nf3g", "N3", "ee4e", "e4=", "+",
            "O-O+O", "Ke8", "e5", "Qd4", "\u{e6}4",
        ] {
            assert_eq!(read(START_FEN, san), "-", "{san:?}");
        }
    }

    /// Whatever the text, the answer is a legal move; and every legal move
    /// is found from its piece letter, origin, destination and promotion
    /// piece written out in full.
    #[test]
    fn every_legal_move_can_be_named() {
        for fen in [
            START_FEN,
            KIWIPETE,
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        ] {
            let pos = Position::from_fen(fen).unwrap();
            let legal = legal_moves(&pos);
            for &mv in &legal {
                let piece = pos.piece_at(mv.from()).unwrap().kind;
                let mut text = match piece {
                    PieceKind::Pawn => String::new(),
                    kind => kind.to_char().to_ascii_uppercase().to_string(),
                };
                text.push_str(&format!("{}{}", mv.from(), mv.to()));
                if let Some(kind) = mv.promotion() {
                    text.push('=');
                    text.push(kind.to_char().to_ascii_uppercase());
                }
                let found = find_san_move(&pos, &text);
                assert_eq!(found, Some(mv), "{fen}: {text}");
                assert!(legal.contains(&found.unwrap()));
            }
        }
    }
}
