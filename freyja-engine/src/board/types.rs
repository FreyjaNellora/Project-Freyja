//! Core types for four-player chess board representation.
//!
//! Defines Square, Player, PieceType, Piece, and board constants.

use std::fmt;

// ─── Board Constants ────────────────────────────────────────────────────────

/// Board dimension (14x14 grid).
pub const BOARD_SIZE: usize = 14;

/// Total squares in the grid (14 * 14).
pub const TOTAL_SQUARES: usize = 196;

/// Number of valid (playable) squares.
pub const VALID_SQUARES: usize = 160;

/// Number of invalid corner squares (4 corners of 3x3).
pub const INVALID_CORNER_COUNT: usize = 36;

/// Size of each invalid corner (3x3).
pub const CORNER_SIZE: usize = 3;

/// Number of players.
pub const PLAYERS: usize = 4;

/// Starting pieces per player.
pub const PIECES_PER_PLAYER: usize = 16;

/// Maximum pieces per player (including all possible promotions).
pub const MAX_PIECES_PER_PLAYER: usize = 32;

/// Sentinel value for king_squares when player is eliminated.
pub const ELIMINATED_KING_SENTINEL: u8 = 255;

/// Number of castling rights bits (2 per player * 4 players).
pub const CASTLING_RIGHTS_BITS: usize = 8;

// ─── Square Validity ────────────────────────────────────────────────────────

/// Check if a (rank, file) coordinate is a valid playable square (const-compatible).
///
/// Invalid squares are the four 3x3 corners:
/// - SW: rank < 3 && file < 3
/// - SE: rank < 3 && file > 10
/// - NW: rank > 10 && file < 3
/// - NE: rank > 10 && file > 10
#[inline]
pub const fn is_valid_square(rank: u8, file: u8) -> bool {
    if rank >= BOARD_SIZE as u8 || file >= BOARD_SIZE as u8 {
        return false;
    }
    let in_corner = (rank > (BOARD_SIZE - CORNER_SIZE - 1) as u8 || rank < CORNER_SIZE as u8)
        && (file > (BOARD_SIZE - CORNER_SIZE - 1) as u8 || file < CORNER_SIZE as u8);
    !in_corner
}

/// Precomputed lookup: `VALID_SQUARE_TABLE[index]` is true if the square is valid.
pub static VALID_SQUARE_TABLE: [bool; TOTAL_SQUARES] = {
    let mut table = [false; TOTAL_SQUARES];
    let mut i: usize = 0;
    while i < TOTAL_SQUARES {
        let rank = (i / BOARD_SIZE) as u8;
        let file = (i % BOARD_SIZE) as u8;
        table[i] = is_valid_square(rank, file);
        i += 1;
    }
    table
};

/// All 160 valid square indices, precomputed.
pub static VALID_SQUARES_LIST: [u8; VALID_SQUARES] = {
    let mut list = [0u8; VALID_SQUARES];
    let mut count: usize = 0;
    let mut i: usize = 0;
    while i < TOTAL_SQUARES {
        let rank = (i / BOARD_SIZE) as u8;
        let file = (i % BOARD_SIZE) as u8;
        if is_valid_square(rank, file) {
            list[count] = i as u8;
            count += 1;
        }
        i += 1;
    }
    // count should be VALID_SQUARES (160) -- const assert not available, verified by test
    list
};

// ─── Square ─────────────────────────────────────────────────────────────────

/// A square index on the 14x14 board (0..195).
///
/// Computed as `rank * 14 + file` where rank and file are 0-indexed.
/// Not all indices are valid — 36 corner squares are invalid.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(pub u8);

impl Square {
    /// Create a square from rank and file (both 0-indexed).
    /// Returns `None` if out of bounds or in an invalid corner.
    pub fn new(rank: u8, file: u8) -> Option<Self> {
        if is_valid_square(rank, file) {
            Some(Self(rank * BOARD_SIZE as u8 + file))
        } else {
            None
        }
    }

    /// Create a square from rank and file without validity check.
    ///
    /// # Safety
    /// Caller must ensure the square is valid.
    pub fn from_rank_file_unchecked(rank: u8, file: u8) -> Self {
        debug_assert!(
            is_valid_square(rank, file),
            "Invalid square: rank={}, file={}",
            rank,
            file
        );
        Self(rank * BOARD_SIZE as u8 + file)
    }

    /// Create a square from a raw index (0..195).
    /// Returns `None` if the index is out of range or maps to an invalid corner.
    pub fn from_index(index: u8) -> Option<Self> {
        if (index as usize) < TOTAL_SQUARES && VALID_SQUARE_TABLE[index as usize] {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Get the rank (0-indexed).
    #[inline]
    pub fn rank(self) -> u8 {
        self.0 / BOARD_SIZE as u8
    }

    /// Get the file (0-indexed).
    #[inline]
    pub fn file(self) -> u8 {
        self.0 % BOARD_SIZE as u8
    }

    /// Get the raw index.
    #[inline]
    pub fn index(self) -> u8 {
        self.0
    }

    /// Check if this square index is valid (not in an invalid corner).
    pub fn is_valid(self) -> bool {
        (self.0 as usize) < TOTAL_SQUARES && VALID_SQUARE_TABLE[self.0 as usize]
    }

    /// Display file as letter (a-n).
    pub fn display_file(self) -> char {
        (b'a' + self.file()) as char
    }

    /// Display rank as number (1-14).
    pub fn display_rank(self) -> u8 {
        self.rank() + 1
    }

    /// Parse from display notation like "d1", "a7", "n11".
    pub fn from_notation(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.len() < 2 || s.len() > 3 {
            return None;
        }
        let bytes = s.as_bytes();
        let file_char = bytes[0];
        if !(b'a'..=b'n').contains(&file_char) {
            return None;
        }
        let file = file_char - b'a';
        let rank_str = &s[1..];
        let rank_display: u8 = rank_str.parse().ok()?;
        if !(1..=14).contains(&rank_display) {
            return None;
        }
        let rank = rank_display - 1;
        Self::new(rank, file)
    }

    /// Serialize to display notation.
    pub fn to_notation(self) -> String {
        format!("{}{}", self.display_file(), self.display_rank())
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.display_file(), self.display_rank())
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Square({}={})", self.0, self)
    }
}

// ─── Player ─────────────────────────────────────────────────────────────────

/// Players in four-player chess, in turn order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Player {
    Red = 0,
    Blue = 1,
    Yellow = 2,
    Green = 3,
}

impl Player {
    /// Get the player from an index (0-3).
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Player::Red),
            1 => Some(Player::Blue),
            2 => Some(Player::Yellow),
            3 => Some(Player::Green),
            _ => None,
        }
    }

    /// Get the player's index (0-3).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Get the next player in turn order (wraps Green -> Red).
    pub fn next(self) -> Self {
        match self {
            Player::Red => Player::Blue,
            Player::Blue => Player::Yellow,
            Player::Yellow => Player::Green,
            Player::Green => Player::Red,
        }
    }

    /// Get the previous player in turn order (wraps Red -> Green).
    pub fn prev(self) -> Self {
        match self {
            Player::Red => Player::Green,
            Player::Blue => Player::Red,
            Player::Yellow => Player::Blue,
            Player::Green => Player::Yellow,
        }
    }

    /// All players in turn order.
    pub fn all() -> [Player; PLAYERS] {
        [Player::Red, Player::Blue, Player::Yellow, Player::Green]
    }

    /// The three opponents of this player.
    pub fn opponents(self) -> [Player; 3] {
        match self {
            Player::Red => [Player::Blue, Player::Yellow, Player::Green],
            Player::Blue => [Player::Red, Player::Yellow, Player::Green],
            Player::Yellow => [Player::Red, Player::Blue, Player::Green],
            Player::Green => [Player::Red, Player::Blue, Player::Yellow],
        }
    }

    /// Single-character identifier for FEN4 and protocol.
    pub fn char(self) -> char {
        match self {
            Player::Red => 'r',
            Player::Blue => 'b',
            Player::Yellow => 'y',
            Player::Green => 'g',
        }
    }

    /// Parse from single character.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'r' => Some(Player::Red),
            'b' => Some(Player::Blue),
            'y' => Some(Player::Yellow),
            'g' => Some(Player::Green),
            _ => None,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Player::Red => write!(f, "Red"),
            Player::Blue => write!(f, "Blue"),
            Player::Yellow => write!(f, "Yellow"),
            Player::Green => write!(f, "Green"),
        }
    }
}

// ─── PieceType ──────────────────────────────────────────────────────────────

/// Chess piece types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    PromotedQueen = 6,
}

impl PieceType {
    /// Number of distinct piece types.
    pub const COUNT: usize = 7;

    /// Get piece type from index (0-6).
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(PieceType::Pawn),
            1 => Some(PieceType::Knight),
            2 => Some(PieceType::Bishop),
            3 => Some(PieceType::Rook),
            4 => Some(PieceType::Queen),
            5 => Some(PieceType::King),
            6 => Some(PieceType::PromotedQueen),
            _ => None,
        }
    }

    /// Get the index of this piece type (0-6).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Whether this piece is a slider (bishop, rook, queen, promoted queen).
    pub fn is_slider(self) -> bool {
        matches!(
            self,
            PieceType::Bishop | PieceType::Rook | PieceType::Queen | PieceType::PromotedQueen
        )
    }

    /// Single-character representation for FEN4.
    pub fn char(self) -> char {
        match self {
            PieceType::Pawn => 'P',
            PieceType::Knight => 'N',
            PieceType::Bishop => 'B',
            PieceType::Rook => 'R',
            PieceType::Queen => 'Q',
            PieceType::King => 'K',
            PieceType::PromotedQueen => 'D',
        }
    }

    /// Parse from single character.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'P' => Some(PieceType::Pawn),
            'N' => Some(PieceType::Knight),
            'B' => Some(PieceType::Bishop),
            'R' => Some(PieceType::Rook),
            'Q' => Some(PieceType::Queen),
            'K' => Some(PieceType::King),
            'D' => Some(PieceType::PromotedQueen),
            _ => None,
        }
    }
}

// ─── Piece ──────────────────────────────────────────────────────────────────

/// A piece on the board (type + owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub piece_type: PieceType,
    pub player: Player,
}

impl Piece {
    pub fn new(piece_type: PieceType, player: Player) -> Self {
        Self { piece_type, player }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.player.char(), self.piece_type.char())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Square validity ──

    #[test]
    fn test_total_valid_squares_count() {
        let count = (0..TOTAL_SQUARES)
            .filter(|&i| VALID_SQUARE_TABLE[i])
            .count();
        assert_eq!(count, VALID_SQUARES);
    }

    #[test]
    fn test_all_36_invalid_squares() {
        // Exhaustive list from 4PC_RULES_REFERENCE Section 1.3
        let invalid_indices: [u8; 36] = [
            // SW corner (a1-c3)
            0, 1, 2, 14, 15, 16, 28, 29, 30, // SE corner (l1-n3)
            11, 12, 13, 25, 26, 27, 39, 40, 41, // NW corner (a12-c14)
            154, 155, 156, 168, 169, 170, 182, 183, 184, // NE corner (l12-n14)
            165, 166, 167, 179, 180, 181, 193, 194, 195,
        ];
        for &idx in &invalid_indices {
            assert!(
                !VALID_SQUARE_TABLE[idx as usize],
                "Square index {} should be invalid",
                idx
            );
        }
        let invalid_count = (0..TOTAL_SQUARES)
            .filter(|&i| !VALID_SQUARE_TABLE[i])
            .count();
        assert_eq!(invalid_count, INVALID_CORNER_COUNT);
    }

    #[test]
    fn test_valid_squares_list_count() {
        // Verify VALID_SQUARES_LIST has exactly 160 entries and all are valid
        let mut count = 0;
        for &idx in VALID_SQUARES_LIST.iter() {
            assert!(
                VALID_SQUARE_TABLE[idx as usize],
                "Index {} in list should be valid",
                idx
            );
            count += 1;
        }
        assert_eq!(count, VALID_SQUARES);
    }

    // ── Square construction and roundtrip ──

    #[test]
    fn test_square_rank_file_roundtrip() {
        for rank in 0..14u8 {
            for file in 0..14u8 {
                if is_valid_square(rank, file) {
                    let sq = Square::new(rank, file).unwrap();
                    assert_eq!(sq.rank(), rank);
                    assert_eq!(sq.file(), file);
                    assert_eq!(sq.index(), rank * 14 + file);
                }
            }
        }
    }

    #[test]
    fn test_square_invalid_corners_return_none() {
        assert!(Square::new(0, 0).is_none()); // a1
        assert!(Square::new(0, 13).is_none()); // n1
        assert!(Square::new(13, 0).is_none()); // a14
        assert!(Square::new(13, 13).is_none()); // n14
    }

    #[test]
    fn test_square_from_notation() {
        assert_eq!(Square::from_notation("d1"), Some(Square(3)));
        assert_eq!(Square::from_notation("h1"), Some(Square(7)));
        assert_eq!(Square::from_notation("a7"), Some(Square(84)));
        assert_eq!(Square::from_notation("n8"), Some(Square(111)));
        assert_eq!(Square::from_notation("g14"), Some(Square(188)));
        assert_eq!(Square::from_notation("a1"), None); // Invalid corner
        assert_eq!(Square::from_notation("n14"), None); // Invalid corner
    }

    #[test]
    fn test_square_display_notation_roundtrip() {
        for &idx in VALID_SQUARES_LIST.iter() {
            let sq = Square(idx);
            let notation = sq.to_notation();
            let parsed = Square::from_notation(&notation).unwrap();
            assert_eq!(sq, parsed, "Roundtrip failed for {}", notation);
        }
    }

    #[test]
    fn test_specific_square_indices_from_rules_reference() {
        // Red king h1 = index 7
        assert_eq!(Square::from_notation("h1").unwrap().index(), 7);
        // Blue king a7 = index 84
        assert_eq!(Square::from_notation("a7").unwrap().index(), 84);
        // Yellow king g14 = index 188
        assert_eq!(Square::from_notation("g14").unwrap().index(), 188);
        // Green king n8 = index 111
        assert_eq!(Square::from_notation("n8").unwrap().index(), 111);
    }

    // ── Player ──

    #[test]
    fn test_player_turn_order() {
        assert_eq!(Player::Red.next(), Player::Blue);
        assert_eq!(Player::Blue.next(), Player::Yellow);
        assert_eq!(Player::Yellow.next(), Player::Green);
        assert_eq!(Player::Green.next(), Player::Red);
    }

    #[test]
    fn test_player_from_index() {
        assert_eq!(Player::from_index(0), Some(Player::Red));
        assert_eq!(Player::from_index(1), Some(Player::Blue));
        assert_eq!(Player::from_index(2), Some(Player::Yellow));
        assert_eq!(Player::from_index(3), Some(Player::Green));
        assert_eq!(Player::from_index(4), None);
    }

    #[test]
    fn test_player_opponents() {
        let opps = Player::Red.opponents();
        assert!(!opps.contains(&Player::Red));
        assert_eq!(opps.len(), 3);
        assert!(opps.contains(&Player::Blue));
        assert!(opps.contains(&Player::Yellow));
        assert!(opps.contains(&Player::Green));
    }

    #[test]
    fn test_player_char_roundtrip() {
        for player in Player::all() {
            let c = player.char();
            assert_eq!(Player::from_char(c), Some(player));
        }
    }

    // ── PieceType ──

    #[test]
    fn test_piece_type_count() {
        assert_eq!(PieceType::COUNT, 7);
    }

    #[test]
    fn test_piece_type_is_slider() {
        assert!(!PieceType::Pawn.is_slider());
        assert!(!PieceType::Knight.is_slider());
        assert!(PieceType::Bishop.is_slider());
        assert!(PieceType::Rook.is_slider());
        assert!(PieceType::Queen.is_slider());
        assert!(!PieceType::King.is_slider());
        assert!(PieceType::PromotedQueen.is_slider());
    }

    #[test]
    fn test_piece_type_from_index_roundtrip() {
        for i in 0..7u8 {
            let pt = PieceType::from_index(i).unwrap();
            assert_eq!(pt.index(), i as usize);
        }
        assert!(PieceType::from_index(7).is_none());
    }

    #[test]
    fn test_piece_type_char_roundtrip() {
        for i in 0..7u8 {
            let pt = PieceType::from_index(i).unwrap();
            let c = pt.char();
            assert_eq!(PieceType::from_char(c), Some(pt));
        }
    }

    // ── Piece ──

    #[test]
    fn test_piece_display() {
        let piece = Piece::new(PieceType::King, Player::Red);
        assert_eq!(format!("{}", piece), "rK");
        let piece = Piece::new(PieceType::Queen, Player::Blue);
        assert_eq!(format!("{}", piece), "bQ");
        let piece = Piece::new(PieceType::PromotedQueen, Player::Yellow);
        assert_eq!(format!("{}", piece), "yD");
    }
}
