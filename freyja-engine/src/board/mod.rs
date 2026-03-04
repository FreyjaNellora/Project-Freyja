//! Board representation for four-player chess.
//!
//! 14x14 grid with 36 invalid corner squares = 160 valid squares.
//! Fixed-size arrays only. No heap allocation in hot paths.

pub mod types;
pub mod zobrist;

mod attacks;
mod fen4;

pub use attacks::AttackInfo;
pub use fen4::Fen4Error;
pub use types::*;
pub use zobrist::zobrist_keys;

// ─── Direction Constants ────────────────────────────────────────────────────

/// Orthogonal direction deltas: (rank_delta, file_delta).
pub const ORTHOGONAL_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Diagonal direction deltas.
pub const DIAGONAL_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

/// All 8 direction deltas.
pub const ALL_DIRS: [(i8, i8); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// Knight move offsets: (rank_delta, file_delta).
pub const KNIGHT_OFFSETS: [(i8, i8); 8] = [
    (2, 1),
    (2, -1),
    (-2, 1),
    (-2, -1),
    (1, 2),
    (1, -2),
    (-1, 2),
    (-1, -2),
];

/// Pawn capture deltas FROM the pawn's position (directions the pawn attacks toward).
///
/// Per 4PC_RULES_REFERENCE Section 5.2:
/// - Red: NE (+1,+1) and NW (+1,-1)
/// - Blue: NE (+1,+1) and SE (-1,+1)
/// - Yellow: SE (-1,+1) and SW (-1,-1)
/// - Green: NW (+1,-1) and SW (-1,-1)
pub const PAWN_CAPTURE_DELTAS: [[(i8, i8); 2]; PLAYERS] = [
    [(1, 1), (1, -1)],   // Red: NE, NW
    [(1, 1), (-1, 1)],   // Blue: NE, SE
    [(-1, 1), (-1, -1)], // Yellow: SE, SW
    [(1, -1), (-1, -1)], // Green: NW, SW
];

// ─── Board Struct ───────────────────────────────────────────────────────────

/// The four-player chess board.
///
/// Fixed-size arrays only. No heap allocation. Cheaply cloneable.
/// Uses `rank * 14 + file` indexing with 36 invalid corner squares.
#[derive(Clone)]
pub struct Board {
    /// Piece occupancy. `None` means empty (or invalid corner).
    squares: [Option<Piece>; TOTAL_SQUARES],
    /// Piece lists per player. Each player can have up to 32 pieces.
    piece_lists: [[Option<(PieceType, Square)>; MAX_PIECES_PER_PLAYER]; PLAYERS],
    /// Count of active pieces per player.
    piece_counts: [u8; PLAYERS],
    /// Square index of each player's king. ELIMINATED_KING_SENTINEL (255) if eliminated.
    king_squares: [u8; PLAYERS],
    /// Zobrist hash of the current position.
    zobrist: u64,
    /// Castling rights: 8 bits (2 per player).
    castling_rights: u8,
    /// En passant target square, if any.
    en_passant: Option<Square>,
    /// The player whose pawn created the en passant opportunity.
    en_passant_pushing_player: Option<Player>,
    /// The player whose turn it is.
    side_to_move: Player,
}

// ─── Board Construction ─────────────────────────────────────────────────────

impl Board {
    /// Create an empty board with no pieces.
    pub fn empty() -> Self {
        Self {
            squares: [None; TOTAL_SQUARES],
            piece_lists: [[None; MAX_PIECES_PER_PLAYER]; PLAYERS],
            piece_counts: [0; PLAYERS],
            king_squares: [ELIMINATED_KING_SENTINEL; PLAYERS],
            zobrist: 0,
            castling_rights: 0,
            en_passant: None,
            en_passant_pushing_player: None,
            side_to_move: Player::Red,
        }
    }

    /// Create the standard starting position per 4PC_RULES_REFERENCE Section 3.
    pub fn starting_position() -> Self {
        let mut board = Self::empty();

        // Red (South): back rank 1, files d-k (indices 3-10)
        // Order: R N B Q K B N R
        let red_back = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in red_back.iter().enumerate() {
            let file = 3 + i as u8;
            let sq = Square::from_rank_file_unchecked(0, file);
            board.place_piece_no_hash(sq, Piece::new(pt, Player::Red));
        }
        // Red pawns: rank 2 (index 1), files d-k
        for file in 3..=10u8 {
            let sq = Square::from_rank_file_unchecked(1, file);
            board.place_piece_no_hash(sq, Piece::new(PieceType::Pawn, Player::Red));
        }

        // Blue (West): back file a (index 0), ranks 4-11 (indices 3-10)
        // Order: R N B K Q B N R (K-Q swapped vs Red)
        let blue_back = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::King,
            PieceType::Queen,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in blue_back.iter().enumerate() {
            let rank = 3 + i as u8;
            let sq = Square::from_rank_file_unchecked(rank, 0);
            board.place_piece_no_hash(sq, Piece::new(pt, Player::Blue));
        }
        // Blue pawns: file b (index 1), ranks 4-11
        for rank in 3..=10u8 {
            let sq = Square::from_rank_file_unchecked(rank, 1);
            board.place_piece_no_hash(sq, Piece::new(PieceType::Pawn, Player::Blue));
        }

        // Yellow (North): back rank 14 (index 13), files d-k
        // Order: R N B K Q B N R (K-Q swapped vs Red)
        let yellow_back = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::King,
            PieceType::Queen,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in yellow_back.iter().enumerate() {
            let file = 3 + i as u8;
            let sq = Square::from_rank_file_unchecked(13, file);
            board.place_piece_no_hash(sq, Piece::new(pt, Player::Yellow));
        }
        // Yellow pawns: rank 13 (index 12), files d-k
        for file in 3..=10u8 {
            let sq = Square::from_rank_file_unchecked(12, file);
            board.place_piece_no_hash(sq, Piece::new(PieceType::Pawn, Player::Yellow));
        }

        // Green (East): back file n (index 13), ranks 4-11
        // Order: R N B Q K B N R (same as Red)
        let green_back = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in green_back.iter().enumerate() {
            let rank = 3 + i as u8;
            let sq = Square::from_rank_file_unchecked(rank, 13);
            board.place_piece_no_hash(sq, Piece::new(pt, Player::Green));
        }
        // Green pawns: file m (index 12), ranks 4-11
        for rank in 3..=10u8 {
            let sq = Square::from_rank_file_unchecked(rank, 12);
            board.place_piece_no_hash(sq, Piece::new(PieceType::Pawn, Player::Green));
        }

        // All castling rights active
        board.castling_rights = 0xFF;

        // Side to move: Red
        board.side_to_move = Player::Red;

        // Compute Zobrist hash from scratch
        board.zobrist = board.compute_full_hash();

        board
    }

    /// Place a piece during board construction (no Zobrist update).
    fn place_piece_no_hash(&mut self, sq: Square, piece: Piece) {
        self.squares[sq.index() as usize] = Some(piece);
        self.add_to_piece_list(piece.player, piece.piece_type, sq);
        if piece.piece_type == PieceType::King {
            self.king_squares[piece.player.index()] = sq.index();
        }
    }
}

// ─── Accessors ──────────────────────────────────────────────────────────────

impl Board {
    /// Get the piece at a square (returns None for empty or invalid squares).
    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq.index() as usize]
    }

    /// Get the current side to move.
    #[inline]
    pub fn side_to_move(&self) -> Player {
        self.side_to_move
    }

    /// Get a player's king square index. Returns ELIMINATED_KING_SENTINEL if eliminated.
    #[inline]
    pub fn king_square(&self, player: Player) -> u8 {
        self.king_squares[player.index()]
    }

    /// Get the castling rights byte.
    #[inline]
    pub fn castling_rights(&self) -> u8 {
        self.castling_rights
    }

    /// Get the en passant target square, if any.
    #[inline]
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Get the player who created the en passant opportunity.
    #[inline]
    pub fn en_passant_pushing_player(&self) -> Option<Player> {
        self.en_passant_pushing_player
    }

    /// Get the Zobrist hash.
    #[inline]
    pub fn zobrist_hash(&self) -> u64 {
        self.zobrist
    }

    /// Get the piece count for a player.
    #[inline]
    pub fn piece_count(&self, player: Player) -> u8 {
        self.piece_counts[player.index()]
    }

    /// Iterate over a player's pieces.
    pub fn pieces(&self, player: Player) -> impl Iterator<Item = (PieceType, Square)> + '_ {
        let idx = player.index();
        let count = self.piece_counts[idx] as usize;
        self.piece_lists[idx][..count]
            .iter()
            .filter_map(|entry| *entry)
    }
}

// ─── Piece List Management ──────────────────────────────────────────────────

impl Board {
    /// Add a piece to a player's piece list.
    fn add_to_piece_list(&mut self, player: Player, piece_type: PieceType, sq: Square) {
        let idx = player.index();
        let count = self.piece_counts[idx] as usize;
        debug_assert!(
            count < MAX_PIECES_PER_PLAYER,
            "Piece list overflow for {}",
            player
        );
        self.piece_lists[idx][count] = Some((piece_type, sq));
        self.piece_counts[idx] += 1;
    }

    /// Remove a piece from a player's piece list by square. Swap-removes for O(1).
    fn remove_from_piece_list(&mut self, player: Player, sq: Square) {
        let idx = player.index();
        let count = self.piece_counts[idx] as usize;
        let pos = self.find_in_piece_list(player, sq);
        debug_assert!(
            pos.is_some(),
            "Piece at {} not found in {}'s piece list",
            sq,
            player
        );
        if let Some(pos) = pos {
            // Swap with last element
            self.piece_lists[idx][pos] = self.piece_lists[idx][count - 1];
            self.piece_lists[idx][count - 1] = None;
            self.piece_counts[idx] -= 1;
        }
    }

    /// Update a piece's square in the piece list (used by make/unmake in Stage 2).
    #[allow(dead_code)]
    pub(crate) fn update_piece_list_square(&mut self, player: Player, from: Square, to: Square) {
        let idx = player.index();
        if let Some(pos) = self.find_in_piece_list(player, from)
            && let Some((pt, _)) = self.piece_lists[idx][pos]
        {
            self.piece_lists[idx][pos] = Some((pt, to));
        }
    }

    /// Find a piece in the piece list by square.
    fn find_in_piece_list(&self, player: Player, sq: Square) -> Option<usize> {
        let idx = player.index();
        let count = self.piece_counts[idx] as usize;
        for i in 0..count {
            if let Some((_, s)) = self.piece_lists[idx][i]
                && s == sq
            {
                return Some(i);
            }
        }
        None
    }

    /// Verify piece list and squares array are in sync (debug assertion).
    #[cfg(debug_assertions)]
    pub fn assert_piece_list_sync(&self) {
        // Check: every piece list entry has a matching square
        for player in Player::all() {
            let idx = player.index();
            let count = self.piece_counts[idx] as usize;
            for i in 0..count {
                let (pt, sq) = self.piece_lists[idx][i]
                    .unwrap_or_else(|| panic!("Hole in piece list for {} at index {}", player, i));
                let square_piece = self.squares[sq.index() as usize].unwrap_or_else(|| {
                    panic!(
                        "Piece list says {} has {:?} at {} but square is empty",
                        player, pt, sq
                    )
                });
                assert_eq!(
                    square_piece.piece_type, pt,
                    "Piece type mismatch at {} for {}",
                    sq, player
                );
                assert_eq!(
                    square_piece.player, player,
                    "Player mismatch at {} for {}",
                    sq, player
                );
            }
            // Entries beyond count should be None
            for i in count..MAX_PIECES_PER_PLAYER {
                assert!(
                    self.piece_lists[idx][i].is_none(),
                    "Non-None entry beyond piece count for {} at index {}",
                    player,
                    i
                );
            }
        }
        // Check: every occupied square has a matching piece list entry
        for i in 0..TOTAL_SQUARES {
            if let Some(piece) = self.squares[i] {
                let player = piece.player;
                let sq = Square(i as u8);
                assert!(
                    self.find_in_piece_list(player, sq).is_some(),
                    "Square {} has {:?} but no matching piece list entry for {}",
                    sq,
                    piece,
                    player
                );
            }
        }
        // Check: king squares match
        for player in Player::all() {
            let ks = self.king_squares[player.index()];
            if ks != ELIMINATED_KING_SENTINEL {
                let sq = Square(ks);
                let piece = self.squares[ks as usize]
                    .unwrap_or_else(|| panic!("King square {} for {} is empty", sq, player));
                assert_eq!(piece.piece_type, PieceType::King);
                assert_eq!(piece.player, player);
            }
        }
    }

    /// No-op in release builds.
    #[cfg(not(debug_assertions))]
    pub fn assert_piece_list_sync(&self) {}
}

// ─── Piece Mutation with Zobrist ────────────────────────────────────────────

impl Board {
    /// Place a piece on a square, updating squares array, piece list, king_squares, and Zobrist hash.
    ///
    /// Panics if the square is invalid or already occupied.
    pub fn set_piece(&mut self, sq: Square, piece: Piece) {
        debug_assert!(sq.is_valid(), "Cannot place piece on invalid square {}", sq);
        debug_assert!(
            self.squares[sq.index() as usize].is_none(),
            "Square {} already occupied by {:?}",
            sq,
            self.squares[sq.index() as usize]
        );

        tracing::debug!(
            square = %sq,
            piece = %piece,
            "set_piece: placing {} on {}",
            piece,
            sq
        );

        let keys = zobrist_keys();

        // Update squares array
        self.squares[sq.index() as usize] = Some(piece);

        // Update Zobrist hash
        self.zobrist ^=
            keys.piece[piece.piece_type.index()][piece.player.index()][sq.index() as usize];

        // Update piece list
        self.add_to_piece_list(piece.player, piece.piece_type, sq);

        // Update king square if placing a king
        if piece.piece_type == PieceType::King {
            self.king_squares[piece.player.index()] = sq.index();
        }
    }

    /// Remove a piece from a square, updating squares array, piece list, and Zobrist hash.
    ///
    /// Returns the removed piece. Panics if the square is empty.
    pub fn remove_piece(&mut self, sq: Square) -> Piece {
        let piece = self.squares[sq.index() as usize]
            .unwrap_or_else(|| panic!("Cannot remove piece from empty square {}", sq));

        tracing::debug!(
            square = %sq,
            piece = %piece,
            "remove_piece: removing {} from {}",
            piece,
            sq
        );

        let keys = zobrist_keys();

        // Update squares array
        self.squares[sq.index() as usize] = None;

        // Update Zobrist hash (XOR again to remove)
        self.zobrist ^=
            keys.piece[piece.piece_type.index()][piece.player.index()][sq.index() as usize];

        // Update piece list
        self.remove_from_piece_list(piece.player, sq);

        piece
    }

    /// Set castling rights, updating Zobrist hash.
    pub fn set_castling_rights(&mut self, rights: u8) {
        let keys = zobrist_keys();
        // XOR out old rights
        for bit in 0..CASTLING_RIGHTS_BITS {
            if self.castling_rights & (1 << bit) != 0 {
                self.zobrist ^= keys.castling[bit];
            }
        }
        self.castling_rights = rights;
        // XOR in new rights
        for bit in 0..CASTLING_RIGHTS_BITS {
            if self.castling_rights & (1 << bit) != 0 {
                self.zobrist ^= keys.castling[bit];
            }
        }
    }

    /// Set en passant state, updating Zobrist hash.
    pub fn set_en_passant(&mut self, ep_sq: Option<Square>, pushing_player: Option<Player>) {
        let keys = zobrist_keys();
        // XOR out old EP
        if let Some(old_sq) = self.en_passant {
            self.zobrist ^= keys.en_passant[old_sq.index() as usize];
        }
        self.en_passant = ep_sq;
        self.en_passant_pushing_player = pushing_player;
        // XOR in new EP
        if let Some(new_sq) = self.en_passant {
            self.zobrist ^= keys.en_passant[new_sq.index() as usize];
        }
    }

    /// Set side to move, updating Zobrist hash.
    pub fn set_side_to_move(&mut self, player: Player) {
        let keys = zobrist_keys();
        // XOR out old side
        self.zobrist ^= keys.side_to_move[self.side_to_move.index()];
        self.side_to_move = player;
        // XOR in new side
        self.zobrist ^= keys.side_to_move[self.side_to_move.index()];
    }

    /// Compute the Zobrist hash from scratch (for verification).
    pub fn compute_full_hash(&self) -> u64 {
        let keys = zobrist_keys();
        let mut hash = 0u64;

        // Piece contributions
        for &idx in VALID_SQUARES_LIST.iter() {
            if let Some(piece) = self.squares[idx as usize] {
                hash ^= keys.piece[piece.piece_type.index()][piece.player.index()][idx as usize];
            }
        }

        // Side to move
        hash ^= keys.side_to_move[self.side_to_move.index()];

        // Castling rights
        for bit in 0..CASTLING_RIGHTS_BITS {
            if self.castling_rights & (1 << bit) != 0 {
                hash ^= keys.castling[bit];
            }
        }

        // En passant
        if let Some(ep_sq) = self.en_passant {
            hash ^= keys.en_passant[ep_sq.index() as usize];
        }

        hash
    }
}

// ─── PartialEq ──────────────────────────────────────────────────────────────

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.squares == other.squares
            && self.castling_rights == other.castling_rights
            && self.en_passant == other.en_passant
            && self.en_passant_pushing_player == other.en_passant_pushing_player
            && self.side_to_move == other.side_to_move
    }
}

impl Eq for Board {}

// ─── Debug ──────────────────────────────────────────────────────────────────

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Board {{")?;
        writeln!(f, "  side_to_move: {}", self.side_to_move)?;
        writeln!(f, "  castling: {:08b}", self.castling_rights)?;
        if let Some(ep) = self.en_passant {
            writeln!(
                f,
                "  en_passant: {} (by {:?})",
                ep, self.en_passant_pushing_player
            )?;
        }
        writeln!(f, "  zobrist: {:016x}", self.zobrist)?;
        // Board diagram
        for rank in (0..BOARD_SIZE).rev() {
            write!(f, "  {:>2} ", rank + 1)?;
            for file in 0..BOARD_SIZE {
                let idx = rank * BOARD_SIZE + file;
                if !VALID_SQUARE_TABLE[idx] {
                    write!(f, " . ")?;
                } else if let Some(piece) = self.squares[idx] {
                    write!(f, "{} ", piece)?;
                } else {
                    write!(f, " _ ")?;
                }
            }
            writeln!(f)?;
        }
        write!(f, "     ")?;
        for file in 0..BOARD_SIZE {
            write!(f, " {} ", (b'a' + file as u8) as char)?;
        }
        writeln!(f)?;
        write!(f, "}}")
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Board construction ──

    #[test]
    fn test_board_clone_is_equal() {
        let board = Board::starting_position();
        let clone = board.clone();
        assert_eq!(board, clone);
    }

    #[test]
    fn test_board_zero_heap_allocation() {
        let board = Board::starting_position();
        let _cloned = board.clone();
        let size = std::mem::size_of::<Board>();
        assert!(size > 0);
        assert!(
            size < 4096,
            "Board struct is {} bytes, expected < 4096",
            size
        );
    }

    // ── Starting position ──

    #[test]
    fn test_starting_position_piece_counts() {
        let board = Board::starting_position();
        for player in Player::all() {
            assert_eq!(
                board.piece_count(player),
                16,
                "{} should have 16 pieces",
                player
            );
        }
    }

    #[test]
    fn test_starting_position_red_king_square() {
        let board = Board::starting_position();
        assert_eq!(board.king_square(Player::Red), 7); // h1
    }

    #[test]
    fn test_starting_position_blue_king_square() {
        let board = Board::starting_position();
        assert_eq!(board.king_square(Player::Blue), 84); // a7
    }

    #[test]
    fn test_starting_position_yellow_king_square() {
        let board = Board::starting_position();
        assert_eq!(board.king_square(Player::Yellow), 188); // g14
    }

    #[test]
    fn test_starting_position_green_king_square() {
        let board = Board::starting_position();
        assert_eq!(board.king_square(Player::Green), 111); // n8
    }

    #[test]
    fn test_starting_position_all_castling_rights() {
        let board = Board::starting_position();
        assert_eq!(board.castling_rights(), 0xFF);
    }

    #[test]
    fn test_starting_position_side_to_move() {
        let board = Board::starting_position();
        assert_eq!(board.side_to_move(), Player::Red);
    }

    #[test]
    fn test_starting_position_no_en_passant() {
        let board = Board::starting_position();
        assert!(board.en_passant().is_none());
    }

    // 4PC Verification Matrix: "Starting position correct" — Red
    #[test]
    fn test_starting_position_red_pieces() {
        let board = Board::starting_position();
        // Back rank: d1=R, e1=N, f1=B, g1=Q, h1=K, i1=B, j1=N, k1=R
        assert_eq!(
            board.piece_at(Square(3)),
            Some(Piece::new(PieceType::Rook, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(4)),
            Some(Piece::new(PieceType::Knight, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(5)),
            Some(Piece::new(PieceType::Bishop, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(6)),
            Some(Piece::new(PieceType::Queen, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(7)),
            Some(Piece::new(PieceType::King, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(8)),
            Some(Piece::new(PieceType::Bishop, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(9)),
            Some(Piece::new(PieceType::Knight, Player::Red))
        );
        assert_eq!(
            board.piece_at(Square(10)),
            Some(Piece::new(PieceType::Rook, Player::Red))
        );
        // Pawns: d2-k2
        for file in 3..=10u8 {
            let sq = Square(1 * 14 + file);
            assert_eq!(
                board.piece_at(sq),
                Some(Piece::new(PieceType::Pawn, Player::Red)),
                "Red pawn expected at file {}",
                file
            );
        }
    }

    // 4PC Verification Matrix: "Starting position correct" — Blue
    #[test]
    fn test_starting_position_blue_pieces() {
        let board = Board::starting_position();
        // Back file a: a4=R, a5=N, a6=B, a7=K, a8=Q, a9=B, a10=N, a11=R
        assert_eq!(
            board.piece_at(Square(42)),
            Some(Piece::new(PieceType::Rook, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(56)),
            Some(Piece::new(PieceType::Knight, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(70)),
            Some(Piece::new(PieceType::Bishop, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(84)),
            Some(Piece::new(PieceType::King, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(98)),
            Some(Piece::new(PieceType::Queen, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(112)),
            Some(Piece::new(PieceType::Bishop, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(126)),
            Some(Piece::new(PieceType::Knight, Player::Blue))
        );
        assert_eq!(
            board.piece_at(Square(140)),
            Some(Piece::new(PieceType::Rook, Player::Blue))
        );
        // Pawns: b4-b11
        for rank in 3..=10u8 {
            let sq = Square(rank * 14 + 1);
            assert_eq!(
                board.piece_at(sq),
                Some(Piece::new(PieceType::Pawn, Player::Blue)),
                "Blue pawn expected at rank {}",
                rank
            );
        }
    }

    // 4PC Verification Matrix: "Starting position correct" — Yellow
    #[test]
    fn test_starting_position_yellow_pieces() {
        let board = Board::starting_position();
        // Back rank 14: d14=R, e14=N, f14=B, g14=K, h14=Q, i14=B, j14=N, k14=R
        assert_eq!(
            board.piece_at(Square(185)),
            Some(Piece::new(PieceType::Rook, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(186)),
            Some(Piece::new(PieceType::Knight, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(187)),
            Some(Piece::new(PieceType::Bishop, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(188)),
            Some(Piece::new(PieceType::King, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(189)),
            Some(Piece::new(PieceType::Queen, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(190)),
            Some(Piece::new(PieceType::Bishop, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(191)),
            Some(Piece::new(PieceType::Knight, Player::Yellow))
        );
        assert_eq!(
            board.piece_at(Square(192)),
            Some(Piece::new(PieceType::Rook, Player::Yellow))
        );
        // Pawns: d13-k13 (rank 12)
        for file in 3..=10u8 {
            let sq = Square(12 * 14 + file);
            assert_eq!(
                board.piece_at(sq),
                Some(Piece::new(PieceType::Pawn, Player::Yellow)),
                "Yellow pawn expected at file {}",
                file
            );
        }
    }

    // 4PC Verification Matrix: "Starting position correct" — Green
    #[test]
    fn test_starting_position_green_pieces() {
        let board = Board::starting_position();
        // Back file n: n4=R, n5=N, n6=B, n7=Q, n8=K, n9=B, n10=N, n11=R
        assert_eq!(
            board.piece_at(Square(55)),
            Some(Piece::new(PieceType::Rook, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(69)),
            Some(Piece::new(PieceType::Knight, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(83)),
            Some(Piece::new(PieceType::Bishop, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(97)),
            Some(Piece::new(PieceType::Queen, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(111)),
            Some(Piece::new(PieceType::King, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(125)),
            Some(Piece::new(PieceType::Bishop, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(139)),
            Some(Piece::new(PieceType::Knight, Player::Green))
        );
        assert_eq!(
            board.piece_at(Square(153)),
            Some(Piece::new(PieceType::Rook, Player::Green))
        );
        // Pawns: m4-m11 (file 12)
        for rank in 3..=10u8 {
            let sq = Square(rank * 14 + 12);
            assert_eq!(
                board.piece_at(sq),
                Some(Piece::new(PieceType::Pawn, Player::Green)),
                "Green pawn expected at rank {}",
                rank
            );
        }
    }

    // ── Piece list sync ──

    #[test]
    fn test_piece_list_sync_starting_position() {
        let board = Board::starting_position();
        board.assert_piece_list_sync();
    }

    #[test]
    fn test_piece_list_iteration_red() {
        let board = Board::starting_position();
        let red_pieces: Vec<_> = board.pieces(Player::Red).collect();
        assert_eq!(red_pieces.len(), 16);
        let pawns = red_pieces
            .iter()
            .filter(|(pt, _)| *pt == PieceType::Pawn)
            .count();
        assert_eq!(pawns, 8);
        let kings = red_pieces
            .iter()
            .filter(|(pt, _)| *pt == PieceType::King)
            .count();
        assert_eq!(kings, 1);
    }

    // ── Zobrist ──

    #[test]
    fn test_zobrist_starting_position_matches_full_hash() {
        let board = Board::starting_position();
        assert_eq!(board.zobrist_hash(), board.compute_full_hash());
    }

    #[test]
    fn test_set_remove_piece_zobrist_consistency() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();

        let sq = Square::new(5, 5).unwrap();
        let piece = Piece::new(PieceType::Rook, Player::Red);

        board.set_piece(sq, piece);
        assert_eq!(board.zobrist, board.compute_full_hash());

        board.remove_piece(sq);
        assert_eq!(board.zobrist, board.compute_full_hash());
    }

    #[test]
    fn test_set_piece_remove_piece_roundtrip() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        let initial_hash = board.zobrist;

        let sq = Square::new(7, 7).unwrap();
        let piece = Piece::new(PieceType::Queen, Player::Blue);

        board.set_piece(sq, piece);
        assert_ne!(board.zobrist, initial_hash);
        assert_eq!(board.piece_at(sq), Some(piece));

        let removed = board.remove_piece(sq);
        assert_eq!(removed, piece);
        assert_eq!(board.zobrist, initial_hash);
        assert!(board.piece_at(sq).is_none());
    }

    #[test]
    fn test_zobrist_consistency_after_many_operations() {
        let mut board = Board::starting_position();
        let sq1 = Square::new(0, 3).unwrap(); // d1 = Red Rook
        let removed = board.remove_piece(sq1);
        assert_eq!(board.zobrist, board.compute_full_hash());

        let sq2 = Square::new(5, 5).unwrap(); // f6
        board.set_piece(sq2, removed);
        assert_eq!(board.zobrist, board.compute_full_hash());

        board.assert_piece_list_sync();
    }

    #[test]
    fn test_castling_rights_zobrist() {
        let mut board = Board::starting_position();
        let original_hash = board.zobrist;
        board.set_castling_rights(0xF0);
        assert_ne!(board.zobrist, original_hash);
        assert_eq!(board.zobrist, board.compute_full_hash());
    }

    #[test]
    fn test_en_passant_zobrist() {
        let mut board = Board::starting_position();
        let original_hash = board.zobrist;
        let ep_sq = Square::new(2, 3).unwrap(); // d3
        board.set_en_passant(Some(ep_sq), Some(Player::Red));
        assert_ne!(board.zobrist, original_hash);
        assert_eq!(board.zobrist, board.compute_full_hash());
        board.set_en_passant(None, None);
        assert_eq!(board.zobrist, original_hash);
    }

    #[test]
    fn test_side_to_move_zobrist() {
        let mut board = Board::starting_position();
        let red_hash = board.zobrist;
        board.set_side_to_move(Player::Blue);
        assert_ne!(board.zobrist, red_hash);
        assert_eq!(board.zobrist, board.compute_full_hash());
    }

    // 4PC Verification Matrix: "Zobrist side-to-move (4 values)"
    #[test]
    fn test_zobrist_side_to_move_all_different() {
        let mut hashes = [0u64; 4];
        for (i, &player) in Player::all().iter().enumerate() {
            let mut board = Board::starting_position();
            board.set_side_to_move(player);
            hashes[i] = board.zobrist_hash();
        }
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(
                    hashes[i],
                    hashes[j],
                    "Zobrist collision for {} and {}",
                    Player::all()[i],
                    Player::all()[j]
                );
            }
        }
    }

    // ── Stress test ──

    #[test]
    fn test_zobrist_consistency_stress() {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();

        let valid_sqs: Vec<Square> = VALID_SQUARES_LIST.iter().map(|&idx| Square(idx)).collect();

        // Place 20 pieces on non-overlapping squares
        let mut placed = Vec::new();
        for i in 0..20 {
            let sq = valid_sqs[i * 7 % valid_sqs.len()];
            if board.piece_at(sq).is_none() {
                let pt = PieceType::from_index((i % 6) as u8).unwrap();
                let player = Player::from_index((i % 4) as u8).unwrap();
                board.set_piece(sq, Piece::new(pt, player));
                assert_eq!(
                    board.zobrist,
                    board.compute_full_hash(),
                    "Zobrist mismatch after set_piece step {}",
                    i
                );
                placed.push(sq);
            }
        }

        // Remove half
        for (i, &sq) in placed.iter().take(placed.len() / 2).enumerate() {
            board.remove_piece(sq);
            assert_eq!(
                board.zobrist,
                board.compute_full_hash(),
                "Zobrist mismatch after remove_piece step {}",
                i
            );
        }

        board.assert_piece_list_sync();
    }
}
