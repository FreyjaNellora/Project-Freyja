//! FEN4 parsing and serialization for four-player chess.
//!
//! Format: `<ranks> <side> <castling> <ep_square> <ep_player>`
//!
//! - Ranks: 14 (top) to 1 (bottom), `/`-separated
//!   - Pieces: 2-char codes `rP`, `bK`, `yQ`, `gN`, `rD` (promoted queen)
//!   - Empty valid squares: digit counts (1-9)
//!   - Invalid corner squares: `x` per invalid square
//! - Side to move: `r`/`b`/`y`/`g`
//! - Castling: `RrBbYyGg` chars for active rights, `-` if none
//!   - R=Red KS, r=Red QS, B=Blue KS, b=Blue QS, Y=Yellow KS, y=Yellow QS, G=Green KS, g=Green QS
//! - EP square: notation like `d3`, or `-`
//! - EP player: `r`/`b`/`y`/`g`, or `-`

use std::fmt;

use super::Board;
use super::types::*;

/// Errors that can occur during FEN4 parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Fen4Error {
    InvalidFormat(String),
    InvalidPiece(String),
    InvalidPlayer(String),
    InvalidCastling(String),
    InvalidEnPassant(String),
    RankLengthMismatch {
        rank: usize,
        expected: usize,
        got: usize,
    },
    WrongRankCount {
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for Fen4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fen4Error::InvalidFormat(s) => write!(f, "Invalid FEN4 format: {}", s),
            Fen4Error::InvalidPiece(s) => write!(f, "Invalid piece: {}", s),
            Fen4Error::InvalidPlayer(s) => write!(f, "Invalid player: {}", s),
            Fen4Error::InvalidCastling(s) => write!(f, "Invalid castling: {}", s),
            Fen4Error::InvalidEnPassant(s) => write!(f, "Invalid en passant: {}", s),
            Fen4Error::RankLengthMismatch {
                rank,
                expected,
                got,
            } => write!(
                f,
                "Rank {} length mismatch: expected {} files, got {}",
                rank, expected, got
            ),
            Fen4Error::WrongRankCount { expected, got } => {
                write!(f, "Wrong rank count: expected {}, got {}", expected, got)
            }
        }
    }
}

impl std::error::Error for Fen4Error {}

impl Board {
    /// Serialize the board to FEN4 string.
    pub fn to_fen4(&self) -> String {
        let mut parts = Vec::new();

        // Board section: ranks 14 (top) to 1 (bottom)
        let mut rank_strs = Vec::new();
        for rank in (0..BOARD_SIZE).rev() {
            let mut rank_str = String::new();
            let mut empty_count = 0u8;
            for file in 0..BOARD_SIZE {
                let idx = rank * BOARD_SIZE + file;
                if !VALID_SQUARE_TABLE[idx] {
                    // Flush empty count first
                    if empty_count > 0 {
                        rank_str.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    rank_str.push('x');
                } else if let Some(piece) = self.squares[idx] {
                    if empty_count > 0 {
                        rank_str.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    rank_str.push(piece.player.char());
                    rank_str.push(piece.piece_type.char());
                } else {
                    empty_count += 1;
                }
            }
            if empty_count > 0 {
                rank_str.push_str(&empty_count.to_string());
            }
            rank_strs.push(rank_str);
        }
        parts.push(rank_strs.join("/"));

        // Side to move
        parts.push(self.side_to_move.char().to_string());

        // Castling rights
        if self.castling_rights == 0 {
            parts.push("-".to_string());
        } else {
            let mut castling = String::new();
            // R=Red KS (bit 0), r=Red QS (bit 1)
            // B=Blue KS (bit 2), b=Blue QS (bit 3)
            // Y=Yellow KS (bit 4), y=Yellow QS (bit 5)
            // G=Green KS (bit 6), g=Green QS (bit 7)
            let chars = ['R', 'r', 'B', 'b', 'Y', 'y', 'G', 'g'];
            for (bit, &ch) in chars.iter().enumerate() {
                if self.castling_rights & (1 << bit) != 0 {
                    castling.push(ch);
                }
            }
            parts.push(castling);
        }

        // En passant
        match self.en_passant {
            Some(sq) => parts.push(sq.to_notation()),
            None => parts.push("-".to_string()),
        }

        // EP pushing player
        match self.en_passant_pushing_player {
            Some(player) => parts.push(player.char().to_string()),
            None => parts.push("-".to_string()),
        }

        parts.join(" ")
    }

    /// Parse a board from FEN4 string.
    pub fn from_fen4(fen: &str) -> Result<Self, Fen4Error> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(Fen4Error::InvalidFormat(format!(
                "Expected 5 parts, got {}",
                parts.len()
            )));
        }

        let board_str = parts[0];
        let side_str = parts[1];
        let castling_str = parts[2];
        let ep_str = parts[3];
        let ep_player_str = parts[4];

        // Parse ranks
        let ranks: Vec<&str> = board_str.split('/').collect();
        if ranks.len() != BOARD_SIZE {
            return Err(Fen4Error::WrongRankCount {
                expected: BOARD_SIZE,
                got: ranks.len(),
            });
        }

        let mut board = Board::empty();

        // Ranks are from 14 (top) to 1 (bottom)
        for (rank_idx_from_top, rank_str) in ranks.iter().enumerate() {
            let rank = (BOARD_SIZE - 1 - rank_idx_from_top) as u8;
            let mut file: u8 = 0;
            let chars: Vec<char> = rank_str.chars().collect();
            let mut i = 0;

            while i < chars.len() && (file as usize) < BOARD_SIZE {
                let ch = chars[i];
                if ch == 'x' {
                    // Invalid corner square — skip
                    file += 1;
                    i += 1;
                } else if ch.is_ascii_digit() {
                    // Empty squares count (may be multi-digit, e.g. "14")
                    let mut num = ch as u8 - b'0';
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        num = num * 10 + (chars[i] as u8 - b'0');
                        i += 1;
                    }
                    file += num;
                } else if ch.is_ascii_lowercase() {
                    // Piece: 2-char code (player_char + piece_char)
                    if i + 1 >= chars.len() {
                        return Err(Fen4Error::InvalidPiece(format!(
                            "Incomplete piece at rank {}, file {}",
                            rank + 1,
                            file
                        )));
                    }
                    let player_char = ch;
                    let piece_char = chars[i + 1];

                    let player = Player::from_char(player_char).ok_or_else(|| {
                        Fen4Error::InvalidPlayer(format!("Unknown player char '{}'", player_char))
                    })?;
                    let piece_type = PieceType::from_char(piece_char).ok_or_else(|| {
                        Fen4Error::InvalidPiece(format!("Unknown piece char '{}'", piece_char))
                    })?;

                    let sq = Square::new(rank, file).ok_or_else(|| {
                        Fen4Error::InvalidPiece(format!(
                            "Piece on invalid square rank={}, file={}",
                            rank, file
                        ))
                    })?;

                    board.place_piece_no_hash(sq, Piece::new(piece_type, player));
                    file += 1;
                    i += 2;
                } else {
                    return Err(Fen4Error::InvalidFormat(format!(
                        "Unexpected character '{}' at rank {}, file {}",
                        ch,
                        rank + 1,
                        file
                    )));
                }
            }

            if file as usize != BOARD_SIZE {
                return Err(Fen4Error::RankLengthMismatch {
                    rank: (rank + 1) as usize,
                    expected: BOARD_SIZE,
                    got: file as usize,
                });
            }
        }

        // Side to move
        if side_str.len() != 1 {
            return Err(Fen4Error::InvalidPlayer(format!(
                "Side to move '{}' not a single char",
                side_str
            )));
        }
        board.side_to_move =
            Player::from_char(side_str.chars().next().unwrap()).ok_or_else(|| {
                Fen4Error::InvalidPlayer(format!("Unknown side to move '{}'", side_str))
            })?;

        // Castling rights
        if castling_str == "-" {
            board.castling_rights = 0;
        } else {
            let mut rights = 0u8;
            let valid_chars = [
                ('R', 0),
                ('r', 1),
                ('B', 2),
                ('b', 3),
                ('Y', 4),
                ('y', 5),
                ('G', 6),
                ('g', 7),
            ];
            for ch in castling_str.chars() {
                let found = valid_chars.iter().find(|(c, _)| *c == ch);
                match found {
                    Some((_, bit)) => rights |= 1 << bit,
                    None => {
                        return Err(Fen4Error::InvalidCastling(format!(
                            "Unknown castling char '{}'",
                            ch
                        )));
                    }
                }
            }
            board.castling_rights = rights;
        }

        // En passant
        if ep_str == "-" {
            board.en_passant = None;
        } else {
            board.en_passant = Some(Square::from_notation(ep_str).ok_or_else(|| {
                Fen4Error::InvalidEnPassant(format!("Invalid EP square '{}'", ep_str))
            })?);
        }

        // EP pushing player
        if ep_player_str == "-" {
            board.en_passant_pushing_player = None;
        } else {
            if ep_player_str.len() != 1 {
                return Err(Fen4Error::InvalidPlayer(format!(
                    "EP player '{}' not a single char",
                    ep_player_str
                )));
            }
            board.en_passant_pushing_player = Some(
                Player::from_char(ep_player_str.chars().next().unwrap()).ok_or_else(|| {
                    Fen4Error::InvalidPlayer(format!("Unknown EP player '{}'", ep_player_str))
                })?,
            );
        }

        // Compute Zobrist hash
        board.zobrist = board.compute_full_hash();

        Ok(board)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FEN4 round-trip tests (10+ required by acceptance criteria) ──

    #[test]
    fn test_fen4_roundtrip_starting_position() {
        let board = Board::starting_position();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
        assert_eq!(parsed.zobrist_hash(), parsed.compute_full_hash());
    }

    #[test]
    fn test_fen4_roundtrip_empty_board() {
        let board = Board::empty();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_single_piece() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        let sq = Square::new(5, 5).unwrap();
        board.set_piece(sq, Piece::new(PieceType::King, Player::Red));
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_blue_to_move() {
        let mut board = Board::starting_position();
        board.set_side_to_move(Player::Blue);
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
        assert_eq!(parsed.side_to_move(), Player::Blue);
    }

    #[test]
    fn test_fen4_roundtrip_yellow_to_move() {
        let mut board = Board::starting_position();
        board.set_side_to_move(Player::Yellow);
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_green_to_move() {
        let mut board = Board::starting_position();
        board.set_side_to_move(Player::Green);
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_no_castling_rights() {
        let mut board = Board::starting_position();
        board.set_castling_rights(0);
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
        assert_eq!(parsed.castling_rights(), 0);
    }

    #[test]
    fn test_fen4_roundtrip_partial_castling_rights() {
        let mut board = Board::starting_position();
        board.set_castling_rights(0b10100101); // Red KS, Red QS off, Blue KS, etc.
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_with_en_passant() {
        let mut board = Board::starting_position();
        let ep_sq = Square::new(2, 3).unwrap(); // d3
        board.set_en_passant(Some(ep_sq), Some(Player::Red));
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
        assert_eq!(parsed.en_passant(), Some(ep_sq));
        assert_eq!(parsed.en_passant_pushing_player(), Some(Player::Red));
    }

    #[test]
    fn test_fen4_roundtrip_promoted_queen() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        let sq = Square::new(6, 6).unwrap();
        board.set_piece(sq, Piece::new(PieceType::PromotedQueen, Player::Yellow));
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_all_piece_types() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        let pieces = [
            (PieceType::Pawn, Player::Red, 3, 3),
            (PieceType::Knight, Player::Blue, 4, 4),
            (PieceType::Bishop, Player::Yellow, 5, 5),
            (PieceType::Rook, Player::Green, 6, 6),
            (PieceType::Queen, Player::Red, 7, 7),
            (PieceType::King, Player::Blue, 8, 8),
            (PieceType::PromotedQueen, Player::Green, 9, 9),
        ];
        for (pt, player, rank, file) in pieces {
            let sq = Square::new(rank, file).unwrap();
            board.set_piece(sq, Piece::new(pt, player));
        }
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_roundtrip_minimal_pieces() {
        // Just two kings
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_piece(
            Square::new(13, 6).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_fen4_parse_invalid_format() {
        assert!(Board::from_fen4("garbage").is_err());
        assert!(Board::from_fen4("").is_err());
    }

    #[test]
    fn test_fen4_zobrist_matches_after_parse() {
        let board = Board::starting_position();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).unwrap();
        assert_eq!(parsed.zobrist_hash(), parsed.compute_full_hash());
    }
}
