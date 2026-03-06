//! Move generation for all piece types and all four player orientations.
//!
//! Handles castling (8 variants), en passant, promotion, and legal filtering.
//! Uses ArrayVec for zero-allocation move buffers (ADR-010).

use arrayvec::ArrayVec;

use crate::board::types::*;
use crate::board::{
    ALL_DIRS, Board, DIAGONAL_DIRS, KNIGHT_OFFSETS, ORTHOGONAL_DIRS, PAWN_CAPTURE_DELTAS,
};

// ─── Constants ──────────────────────────────────────────────────────────────

/// Maximum legal moves in any position (generous upper bound).
pub const MAX_MOVES: usize = 256;

/// Pawn push direction deltas per player: (rank_delta, file_delta).
/// Red: North (+1,0), Blue: East (0,+1), Yellow: South (-1,0), Green: West (0,-1).
pub const PAWN_PUSH_DELTAS: [(i8, i8); PLAYERS] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// Pawn starting rank index per player (the rank/file where pawns begin).
/// Red: rank 1, Blue: file 1, Yellow: rank 12, Green: file 12.
const PAWN_START_INDEX: [u8; PLAYERS] = [1, 1, 12, 12];

/// Promotion rank/file index per player (FFA mode).
/// Red: rank 8, Blue: file 8, Yellow: rank 5, Green: file 5.
const PROMOTION_INDEX: [u8; PLAYERS] = [8, 8, 5, 5];

/// Promotion piece types (Queen, Rook, Bishop, Knight).
const PROMOTION_PIECES: [PieceType; 4] = [
    PieceType::PromotedQueen,
    PieceType::Rook,
    PieceType::Bishop,
    PieceType::Knight,
];

// ─── Move Encoding ──────────────────────────────────────────────────────────

/// Move flags for special move types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveFlags {
    Normal = 0,
    DoublePush = 1,
    Castle = 2,
    EnPassant = 3,
}

impl MoveFlags {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => MoveFlags::Normal,
            1 => MoveFlags::DoublePush,
            2 => MoveFlags::Castle,
            3 => MoveFlags::EnPassant,
            _ => MoveFlags::Normal,
        }
    }
}

/// Compact move encoding in a u32 bitfield.
///
/// ```text
/// Bits  0-7:  from square (0-195)
/// Bits  8-15: to square (0-195)
/// Bits 16-19: piece type moved (0-6)
/// Bits 20-23: captured piece type (7 = no capture)
/// Bits 24-27: promotion piece type (7 = no promotion)
/// Bits 28-29: flags (Normal, DoublePush, Castle, EnPassant)
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(pub u32);

/// Sentinel for "no capture" or "no promotion" in the bitfield.
const NO_PIECE: u8 = 7;

impl Move {
    /// Create a normal move.
    #[inline]
    pub fn new(from: Square, to: Square, piece_type: PieceType) -> Self {
        Self::encode(from, to, piece_type, NO_PIECE, NO_PIECE, MoveFlags::Normal)
    }

    /// Create a capture move.
    #[inline]
    pub fn capture(from: Square, to: Square, piece_type: PieceType, captured: PieceType) -> Self {
        Self::encode(
            from,
            to,
            piece_type,
            captured.index() as u8,
            NO_PIECE,
            MoveFlags::Normal,
        )
    }

    /// Create a promotion move (possibly with capture).
    #[inline]
    pub fn new_promotion(
        from: Square,
        to: Square,
        captured: Option<PieceType>,
        promotion: PieceType,
    ) -> Self {
        let cap = captured.map_or(NO_PIECE, |pt| pt.index() as u8);
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            cap,
            promotion.index() as u8,
            MoveFlags::Normal,
        )
    }

    /// Create a double pawn push.
    #[inline]
    pub fn double_push(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            NO_PIECE,
            NO_PIECE,
            MoveFlags::DoublePush,
        )
    }

    /// Create an en passant capture.
    #[inline]
    pub fn en_passant(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            PieceType::Pawn.index() as u8,
            NO_PIECE,
            MoveFlags::EnPassant,
        )
    }

    /// Create a castling move (king's from and to squares).
    #[inline]
    pub fn castle(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::King,
            NO_PIECE,
            NO_PIECE,
            MoveFlags::Castle,
        )
    }

    #[inline]
    fn encode(
        from: Square,
        to: Square,
        piece_type: PieceType,
        captured: u8,
        promotion: u8,
        flags: MoveFlags,
    ) -> Self {
        let bits = (from.index() as u32)
            | ((to.index() as u32) << 8)
            | ((piece_type.index() as u32) << 16)
            | ((captured as u32) << 20)
            | ((promotion as u32) << 24)
            | ((flags as u32) << 28);
        Self(bits)
    }

    /// Source square.
    #[inline]
    pub fn from_sq(self) -> Square {
        Square((self.0 & 0xFF) as u8)
    }

    /// Destination square.
    #[inline]
    pub fn to_sq(self) -> Square {
        Square(((self.0 >> 8) & 0xFF) as u8)
    }

    /// Piece type being moved.
    #[inline]
    pub fn piece_type(self) -> PieceType {
        PieceType::from_index(((self.0 >> 16) & 0xF) as u8).unwrap_or(PieceType::Pawn)
    }

    /// Captured piece type, if any.
    #[inline]
    pub fn captured(self) -> Option<PieceType> {
        let bits = ((self.0 >> 20) & 0xF) as u8;
        if bits == NO_PIECE {
            None
        } else {
            PieceType::from_index(bits)
        }
    }

    /// Promotion piece type, if any.
    #[inline]
    pub fn promotion(self) -> Option<PieceType> {
        let bits = ((self.0 >> 24) & 0xF) as u8;
        if bits == NO_PIECE {
            None
        } else {
            PieceType::from_index(bits)
        }
    }

    /// Move flags.
    #[inline]
    pub fn flags(self) -> MoveFlags {
        MoveFlags::from_bits(((self.0 >> 28) & 0x3) as u8)
    }

    /// Whether this move is a capture (including en passant).
    #[inline]
    pub fn is_capture(self) -> bool {
        self.captured().is_some()
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Move({}{} {:?}",
            self.from_sq(),
            self.to_sq(),
            self.piece_type()
        )?;
        if let Some(cap) = self.captured() {
            write!(f, " x{:?}", cap)?;
        }
        if let Some(promo) = self.promotion() {
            write!(f, " ={:?}", promo)?;
        }
        if self.flags() != MoveFlags::Normal {
            write!(f, " [{:?}]", self.flags())?;
        }
        write!(f, ")")
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from_sq(), self.to_sq())?;
        if let Some(promo) = self.promotion() {
            write!(f, "{}", promo.char().to_ascii_lowercase())?;
        }
        Ok(())
    }
}

// ─── MoveUndo ───────────────────────────────────────────────────────────────

/// State saved before a move for unmake. Fixed-size, no heap allocation.
#[derive(Debug, Clone, Copy)]
pub struct MoveUndo {
    /// The move that was made.
    pub mv: Move,
    /// Captured piece (if any), including player info.
    pub captured_piece: Option<Piece>,
    /// Previous castling rights byte.
    pub prev_castling: u8,
    /// Previous en passant square.
    pub prev_en_passant: Option<Square>,
    /// Previous en passant pushing player.
    pub prev_ep_player: Option<Player>,
    /// Previous Zobrist hash.
    pub prev_zobrist: u64,
    /// Previous side to move.
    pub prev_side_to_move: Player,
}

// ─── Castling Table ─────────────────────────────────────────────────────────

/// Castling definition for one variant.
struct CastleDef {
    player: Player,
    rights_bit: u8,
    king_from: u8,
    king_to: u8,
    rook_from: u8,
    rook_to: u8,
    empty_sqs: &'static [u8],
    king_path: &'static [u8],
}

/// All 8 castling variants, per 4PC_RULES_REFERENCE Section 6.
static CASTLE_DEFS: [CastleDef; 8] = [
    // Red KS: h1->j1, k1->i1
    CastleDef {
        player: Player::Red,
        rights_bit: 0,
        king_from: 7,
        king_to: 9,
        rook_from: 10,
        rook_to: 8,
        empty_sqs: &[8, 9],
        king_path: &[7, 8, 9],
    },
    // Red QS: h1->f1, d1->g1
    CastleDef {
        player: Player::Red,
        rights_bit: 1,
        king_from: 7,
        king_to: 5,
        rook_from: 3,
        rook_to: 6,
        empty_sqs: &[4, 5, 6],
        king_path: &[7, 6, 5],
    },
    // Blue KS: a7->a5, a4->a6
    CastleDef {
        player: Player::Blue,
        rights_bit: 2,
        king_from: 84,
        king_to: 56,
        rook_from: 42,
        rook_to: 70,
        empty_sqs: &[56, 70],
        king_path: &[84, 70, 56],
    },
    // Blue QS: a7->a9, a11->a8
    CastleDef {
        player: Player::Blue,
        rights_bit: 3,
        king_from: 84,
        king_to: 112,
        rook_from: 140,
        rook_to: 98,
        empty_sqs: &[98, 112, 126],
        king_path: &[84, 98, 112],
    },
    // Yellow KS: g14->e14, d14->f14
    CastleDef {
        player: Player::Yellow,
        rights_bit: 4,
        king_from: 188,
        king_to: 186,
        rook_from: 185,
        rook_to: 187,
        empty_sqs: &[186, 187],
        king_path: &[188, 187, 186],
    },
    // Yellow QS: g14->i14, k14->h14
    CastleDef {
        player: Player::Yellow,
        rights_bit: 5,
        king_from: 188,
        king_to: 190,
        rook_from: 192,
        rook_to: 189,
        empty_sqs: &[189, 190, 191],
        king_path: &[188, 189, 190],
    },
    // Green KS: n8->n10, n11->n9
    CastleDef {
        player: Player::Green,
        rights_bit: 6,
        king_from: 111,
        king_to: 139,
        rook_from: 153,
        rook_to: 125,
        empty_sqs: &[125, 139],
        king_path: &[111, 125, 139],
    },
    // Green QS: n8->n6, n4->n7
    CastleDef {
        player: Player::Green,
        rights_bit: 7,
        king_from: 111,
        king_to: 83,
        rook_from: 55,
        rook_to: 97,
        empty_sqs: &[69, 83, 97],
        king_path: &[111, 97, 83],
    },
];

/// Rook starting squares for castling rights revocation.
static CASTLING_ROOK_SQUARES: [u8; 8] = [
    10,  // Red KS rook at k1
    3,   // Red QS rook at d1
    42,  // Blue KS rook at a4
    140, // Blue QS rook at a11
    185, // Yellow KS rook at d14
    192, // Yellow QS rook at k14
    153, // Green KS rook at n11
    55,  // Green QS rook at n4
];

/// King starting squares per player for castling rights revocation.
static CASTLING_KING_SQUARES: [u8; PLAYERS] = [7, 84, 188, 111];

// ─── Helper: try_square ────────────────────────────────────────────────────

/// Try to create a valid square from signed rank/file.
#[inline]
fn try_square(rank: i8, file: i8) -> Option<Square> {
    if rank >= 0 && rank < BOARD_SIZE as i8 && file >= 0 && file < BOARD_SIZE as i8 {
        let r = rank as u8;
        let f = file as u8;
        if is_valid_square(r, f) {
            return Some(Square::from_rank_file_unchecked(r, f));
        }
    }
    None
}

/// Check if a pawn is on its starting rank/file for a given player.
#[inline]
fn is_pawn_start(sq: Square, player: Player) -> bool {
    let idx = player.index();
    match idx {
        0 | 2 => sq.rank() == PAWN_START_INDEX[idx], // Red/Yellow: check rank
        1 | 3 => sq.file() == PAWN_START_INDEX[idx], // Blue/Green: check file
        _ => false,
    }
}

/// Check if a pawn has reached its promotion rank/file.
#[inline]
fn is_promotion_square(sq: Square, player: Player) -> bool {
    let idx = player.index();
    match idx {
        0 | 2 => sq.rank() == PROMOTION_INDEX[idx], // Red/Yellow: check rank
        1 | 3 => sq.file() == PROMOTION_INDEX[idx], // Blue/Green: check file
        _ => false,
    }
}

// ─── Pseudo-Legal Move Generation ───────────────────────────────────────────

/// Generate all pseudo-legal moves for the current side to move.
fn generate_pseudo_legal(board: &Board, moves: &mut ArrayVec<Move, MAX_MOVES>) {
    let player = board.side_to_move();

    for (piece_type, sq) in board.pieces(player) {
        match piece_type {
            PieceType::Pawn => gen_pawn_moves(board, player, sq, moves),
            PieceType::Knight => gen_knight_moves(board, player, sq, moves),
            PieceType::Bishop => gen_slider_moves(board, player, sq, &DIAGONAL_DIRS, moves),
            PieceType::Rook => gen_slider_moves(board, player, sq, &ORTHOGONAL_DIRS, moves),
            PieceType::Queen | PieceType::PromotedQueen => {
                gen_slider_moves(board, player, sq, &ALL_DIRS, moves);
            }
            PieceType::King => gen_king_moves(board, player, sq, moves),
        }
    }

    gen_castling_moves(board, player, moves);
    gen_en_passant_moves(board, player, moves);
}

fn gen_pawn_moves(
    board: &Board,
    player: Player,
    sq: Square,
    moves: &mut ArrayVec<Move, MAX_MOVES>,
) {
    let (dr, df) = PAWN_PUSH_DELTAS[player.index()];
    let rank = sq.rank() as i8;
    let file = sq.file() as i8;

    // Single push
    if let Some(target) = try_square(rank + dr, file + df)
        && board.piece_at(target).is_none()
    {
        if is_promotion_square(target, player) {
            for &promo in &PROMOTION_PIECES {
                moves.push(Move::new_promotion(sq, target, None, promo));
            }
        } else {
            moves.push(Move::new(sq, target, PieceType::Pawn));

            // Double push
            if is_pawn_start(sq, player)
                && let Some(double_target) = try_square(rank + 2 * dr, file + 2 * df)
                && board.piece_at(double_target).is_none()
            {
                moves.push(Move::double_push(sq, double_target));
            }
        }
    }

    // Captures
    for &(cr, cf) in &PAWN_CAPTURE_DELTAS[player.index()] {
        if let Some(target) = try_square(rank + cr, file + cf)
            && let Some(piece) = board.piece_at(target)
            && piece.player != player
        {
            if is_promotion_square(target, player) {
                for &promo in &PROMOTION_PIECES {
                    moves.push(Move::new_promotion(
                        sq,
                        target,
                        Some(piece.piece_type),
                        promo,
                    ));
                }
            } else {
                moves.push(Move::capture(sq, target, PieceType::Pawn, piece.piece_type));
            }
        }
    }
}

fn gen_knight_moves(
    board: &Board,
    player: Player,
    sq: Square,
    moves: &mut ArrayVec<Move, MAX_MOVES>,
) {
    let rank = sq.rank() as i8;
    let file = sq.file() as i8;

    for &(dr, df) in &KNIGHT_OFFSETS {
        if let Some(target) = try_square(rank + dr, file + df) {
            match board.piece_at(target) {
                None => moves.push(Move::new(sq, target, PieceType::Knight)),
                Some(piece) if piece.player != player => {
                    moves.push(Move::capture(
                        sq,
                        target,
                        PieceType::Knight,
                        piece.piece_type,
                    ));
                }
                _ => {}
            }
        }
    }
}

fn gen_slider_moves(
    board: &Board,
    player: Player,
    sq: Square,
    dirs: &[(i8, i8)],
    moves: &mut ArrayVec<Move, MAX_MOVES>,
) {
    let piece_type = board.piece_at(sq).unwrap().piece_type;
    let rank = sq.rank() as i8;
    let file = sq.file() as i8;

    for &(dr, df) in dirs {
        let mut r = rank + dr;
        let mut f = file + df;
        loop {
            match try_square(r, f) {
                None => break,
                Some(target) => match board.piece_at(target) {
                    None => {
                        moves.push(Move::new(sq, target, piece_type));
                    }
                    Some(piece) if piece.player != player => {
                        moves.push(Move::capture(sq, target, piece_type, piece.piece_type));
                        break;
                    }
                    _ => break,
                },
            }
            r += dr;
            f += df;
        }
    }
}

fn gen_king_moves(
    board: &Board,
    player: Player,
    sq: Square,
    moves: &mut ArrayVec<Move, MAX_MOVES>,
) {
    let rank = sq.rank() as i8;
    let file = sq.file() as i8;

    for &(dr, df) in &ALL_DIRS {
        if let Some(target) = try_square(rank + dr, file + df) {
            match board.piece_at(target) {
                None => moves.push(Move::new(sq, target, PieceType::King)),
                Some(piece) if piece.player != player => {
                    moves.push(Move::capture(sq, target, PieceType::King, piece.piece_type));
                }
                _ => {}
            }
        }
    }
}

fn gen_castling_moves(board: &Board, player: Player, moves: &mut ArrayVec<Move, MAX_MOVES>) {
    let rights = board.castling_rights();

    for def in &CASTLE_DEFS {
        if def.player != player {
            continue;
        }
        if rights & (1 << def.rights_bit) == 0 {
            continue;
        }

        // Check empty squares
        let mut path_clear = true;
        for &idx in def.empty_sqs {
            if board.piece_at(Square(idx)).is_some() {
                path_clear = false;
                break;
            }
        }
        if !path_clear {
            continue;
        }

        // Check king path not attacked by any opponent
        let mut path_safe = true;
        'path: for &idx in def.king_path {
            let sq = Square(idx);
            for &opp in &player.opponents() {
                if board.is_square_attacked_by(sq, opp) {
                    path_safe = false;
                    break 'path;
                }
            }
        }
        if !path_safe {
            continue;
        }

        moves.push(Move::castle(Square(def.king_from), Square(def.king_to)));
    }
}

fn gen_en_passant_moves(board: &Board, player: Player, moves: &mut ArrayVec<Move, MAX_MOVES>) {
    let ep_sq = match board.en_passant() {
        Some(sq) => sq,
        None => return,
    };

    for &(cr, cf) in &PAWN_CAPTURE_DELTAS[player.index()] {
        let pawn_rank = ep_sq.rank() as i8 - cr;
        let pawn_file = ep_sq.file() as i8 - cf;
        if let Some(pawn_sq) = try_square(pawn_rank, pawn_file)
            && let Some(piece) = board.piece_at(pawn_sq)
            && piece.player == player
            && piece.piece_type == PieceType::Pawn
        {
            moves.push(Move::en_passant(pawn_sq, ep_sq));
        }
    }
}

// ─── Find EP Captured Pawn (ADR-009: Board Scan) ───────────────────────────

/// Find EP captured pawn square using explicit parameters.
fn find_ep_captured_pawn_sq(ep_target: Square, pushing_player: Player) -> Square {
    let (dr, df) = PAWN_PUSH_DELTAS[pushing_player.index()];
    let pawn_rank = ep_target.rank() as i8 + dr;
    let pawn_file = ep_target.file() as i8 + df;
    try_square(pawn_rank, pawn_file).expect("EP captured pawn square invalid")
}

// ─── Make / Unmake ──────────────────────────────────────────────────────────

/// Make a move on the board, returning undo state.
pub fn make_move(board: &mut Board, mv: Move) -> MoveUndo {
    let mut undo = MoveUndo {
        mv,
        captured_piece: None,
        prev_castling: board.castling_rights(),
        prev_en_passant: board.en_passant(),
        prev_ep_player: board.en_passant_pushing_player(),
        prev_zobrist: board.zobrist_hash(),
        prev_side_to_move: board.side_to_move(),
    };

    let player = board.side_to_move();
    let from = mv.from_sq();
    let to = mv.to_sq();
    let flags = mv.flags();

    // Clear en passant
    if board.en_passant().is_some() {
        board.set_en_passant(None, None);
    }

    match flags {
        MoveFlags::Castle => {
            let def = CASTLE_DEFS
                .iter()
                .find(|d| {
                    d.player == player && d.king_from == from.index() && d.king_to == to.index()
                })
                .expect("Invalid castle move");

            // Move king
            board.remove_piece(from);
            board.set_piece(to, Piece::new(PieceType::King, player));
            board.update_piece_list_square(player, from, to);

            // Move rook
            let rook_from = Square(def.rook_from);
            let rook_to = Square(def.rook_to);
            board.remove_piece(rook_from);
            board.set_piece(rook_to, Piece::new(PieceType::Rook, player));
            board.update_piece_list_square(player, rook_from, rook_to);

            // Revoke both castling rights
            let mut rights = board.castling_rights();
            let base_bit = player.index() * 2;
            rights &= !(3 << base_bit);
            board.set_castling_rights(rights);
        }
        MoveFlags::EnPassant => {
            let ep_target = undo.prev_en_passant.expect("EP move without EP square");
            let pushing_player = undo.prev_ep_player.expect("EP move without pushing player");
            let captured_pawn_sq = find_ep_captured_pawn_sq(ep_target, pushing_player);

            // Remove captured pawn
            let captured = board.remove_piece(captured_pawn_sq);
            undo.captured_piece = Some(captured);

            // Move our pawn
            board.remove_piece(from);
            board.set_piece(to, Piece::new(PieceType::Pawn, player));
            board.update_piece_list_square(player, from, to);
        }
        MoveFlags::DoublePush => {
            // Move pawn
            board.remove_piece(from);
            board.set_piece(to, Piece::new(PieceType::Pawn, player));
            board.update_piece_list_square(player, from, to);

            // Set EP target (square the pawn passed through)
            let (dr, df) = PAWN_PUSH_DELTAS[player.index()];
            let ep_rank = from.rank() as i8 + dr;
            let ep_file = from.file() as i8 + df;
            if let Some(ep_sq) = try_square(ep_rank, ep_file) {
                board.set_en_passant(Some(ep_sq), Some(player));
            }
        }
        MoveFlags::Normal => {
            // Handle capture
            if mv.captured().is_some() {
                let captured = board.remove_piece(to);
                undo.captured_piece = Some(captured);
                revoke_castling_for_rook_capture(board, to);
            }

            // Move piece
            let piece_type = mv.piece_type();
            board.remove_piece(from);

            let placed_type = mv.promotion().unwrap_or(piece_type);
            board.set_piece(to, Piece::new(placed_type, player));
            board.update_piece_list_square(player, from, to);

            revoke_castling_for_piece_move(board, player, piece_type, from);
        }
    }

    // Advance side to move
    board.set_side_to_move(player.next());

    undo
}

fn revoke_castling_for_rook_capture(board: &mut Board, capture_sq: Square) {
    let mut rights = board.castling_rights();
    for (bit, &rook_sq) in CASTLING_ROOK_SQUARES.iter().enumerate() {
        if capture_sq.index() == rook_sq && rights & (1 << bit) != 0 {
            rights &= !(1 << bit);
            board.set_castling_rights(rights);
            return;
        }
    }
}

fn revoke_castling_for_piece_move(
    board: &mut Board,
    player: Player,
    piece_type: PieceType,
    from: Square,
) {
    let mut rights = board.castling_rights();
    let base_bit = player.index() * 2;

    if piece_type == PieceType::King && from.index() == CASTLING_KING_SQUARES[player.index()] {
        rights &= !(3 << base_bit);
        board.set_castling_rights(rights);
    } else if piece_type == PieceType::Rook {
        for bit in [base_bit, base_bit + 1] {
            if from.index() == CASTLING_ROOK_SQUARES[bit] && rights & (1 << bit) != 0 {
                rights &= !(1 << bit);
                board.set_castling_rights(rights);
                return;
            }
        }
    }
}

/// Unmake a move, restoring the board to its previous state.
pub fn unmake_move(board: &mut Board, undo: &MoveUndo) {
    let mv = undo.mv;
    let player = undo.prev_side_to_move;
    let from = mv.from_sq();
    let to = mv.to_sq();
    let flags = mv.flags();

    // Restore side to move
    board.set_side_to_move(player);

    match flags {
        MoveFlags::Castle => {
            let def = CASTLE_DEFS
                .iter()
                .find(|d| {
                    d.player == player && d.king_from == from.index() && d.king_to == to.index()
                })
                .expect("Invalid castle unmake");

            // Unmove king
            board.remove_piece(to);
            board.set_piece(from, Piece::new(PieceType::King, player));
            board.update_piece_list_square(player, to, from);

            // Unmove rook
            let rook_from = Square(def.rook_from);
            let rook_to = Square(def.rook_to);
            board.remove_piece(rook_to);
            board.set_piece(rook_from, Piece::new(PieceType::Rook, player));
            board.update_piece_list_square(player, rook_to, rook_from);
        }
        MoveFlags::EnPassant => {
            // Unmove our pawn
            board.remove_piece(to);
            board.set_piece(from, Piece::new(PieceType::Pawn, player));
            board.update_piece_list_square(player, to, from);

            // Restore captured pawn
            if let Some(captured) = undo.captured_piece {
                let ep_target = undo.prev_en_passant.expect("EP undo without EP square");
                let pusher = undo.prev_ep_player.expect("EP undo without pushing player");
                let pawn_sq = find_ep_captured_pawn_sq(ep_target, pusher);
                board.set_piece(pawn_sq, captured);
            }
        }
        MoveFlags::DoublePush | MoveFlags::Normal => {
            // Unmove piece
            let original_piece_type = mv.piece_type();
            board.remove_piece(to);
            board.set_piece(from, Piece::new(original_piece_type, player));
            board.update_piece_list_square(player, to, from);

            // Restore captured piece
            if let Some(captured) = undo.captured_piece {
                board.set_piece(to, captured);
            }
        }
    }

    // Restore castling rights
    board.set_castling_rights(undo.prev_castling);

    // Restore en passant
    board.set_en_passant(undo.prev_en_passant, undo.prev_ep_player);

    debug_assert_eq!(
        board.zobrist_hash(),
        undo.prev_zobrist,
        "Zobrist mismatch after unmake! Expected {:016x}, got {:016x}",
        undo.prev_zobrist,
        board.zobrist_hash()
    );
}

// ─── Legal Move Generation ─────────────────────────────────────────────────

/// Generate all legal moves for the current side to move.
pub fn generate_legal_moves(board: &mut Board) -> ArrayVec<Move, MAX_MOVES> {
    let mut pseudo = ArrayVec::<Move, MAX_MOVES>::new();
    generate_pseudo_legal(board, &mut pseudo);

    let player = board.side_to_move();
    let mut legal = ArrayVec::<Move, MAX_MOVES>::new();

    for mv in pseudo {
        let undo = make_move(board, mv);
        let in_check = board.is_in_check(player);
        unmake_move(board, &undo);

        if !in_check {
            legal.push(mv);
        }
    }

    legal
}

/// Generate legal moves into a provided buffer (ADR-010 MoveBuffer pattern).
pub fn generate_legal_into(board: &mut Board, moves: &mut ArrayVec<Move, MAX_MOVES>) {
    let result = generate_legal_moves(board);
    moves.extend(result);
}

// ─── Perft ──────────────────────────────────────────────────────────────────

/// Perft: count leaf nodes at a given depth.
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal_moves(board);

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0u64;
    for mv in moves {
        let undo = make_move(board, mv);

        #[cfg(debug_assertions)]
        board.assert_piece_list_sync();

        nodes += perft(board, depth - 1);
        unmake_move(board, &undo);

        #[cfg(debug_assertions)]
        board.assert_piece_list_sync();
    }

    nodes
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Move encoding roundtrip ──

    #[test]
    fn test_move_encoding_normal() {
        let from = Square::new(3, 5).unwrap();
        let to = Square::new(5, 5).unwrap();
        let mv = Move::new(from, to, PieceType::Rook);
        assert_eq!(mv.from_sq(), from);
        assert_eq!(mv.to_sq(), to);
        assert_eq!(mv.piece_type(), PieceType::Rook);
        assert_eq!(mv.captured(), None);
        assert_eq!(mv.promotion(), None);
        assert_eq!(mv.flags(), MoveFlags::Normal);
    }

    #[test]
    fn test_move_encoding_capture() {
        let from = Square::new(3, 5).unwrap();
        let to = Square::new(4, 6).unwrap();
        let mv = Move::capture(from, to, PieceType::Pawn, PieceType::Knight);
        assert_eq!(mv.from_sq(), from);
        assert_eq!(mv.to_sq(), to);
        assert_eq!(mv.piece_type(), PieceType::Pawn);
        assert_eq!(mv.captured(), Some(PieceType::Knight));
        assert!(mv.is_capture());
    }

    #[test]
    fn test_move_encoding_promotion() {
        let from = Square::new(7, 5).unwrap();
        let to = Square::new(8, 5).unwrap();
        let mv = Move::new_promotion(from, to, None, PieceType::PromotedQueen);
        assert_eq!(mv.piece_type(), PieceType::Pawn);
        assert_eq!(mv.promotion(), Some(PieceType::PromotedQueen));
        assert_eq!(mv.captured(), None);
    }

    #[test]
    fn test_move_encoding_promotion_capture() {
        let from = Square::new(7, 5).unwrap();
        let to = Square::new(8, 6).unwrap();
        let mv = Move::new_promotion(from, to, Some(PieceType::Rook), PieceType::PromotedQueen);
        assert_eq!(mv.promotion(), Some(PieceType::PromotedQueen));
        assert_eq!(mv.captured(), Some(PieceType::Rook));
    }

    #[test]
    fn test_move_encoding_double_push() {
        let from = Square::new(1, 5).unwrap();
        let to = Square::new(3, 5).unwrap();
        let mv = Move::double_push(from, to);
        assert_eq!(mv.flags(), MoveFlags::DoublePush);
        assert_eq!(mv.piece_type(), PieceType::Pawn);
    }

    #[test]
    fn test_move_encoding_en_passant() {
        let from = Square::new(4, 5).unwrap();
        let to = Square::new(5, 6).unwrap();
        let mv = Move::en_passant(from, to);
        assert_eq!(mv.flags(), MoveFlags::EnPassant);
        assert!(mv.is_capture());
        assert_eq!(mv.captured(), Some(PieceType::Pawn));
    }

    #[test]
    fn test_move_encoding_castle() {
        let from = Square(7);
        let to = Square(9);
        let mv = Move::castle(from, to);
        assert_eq!(mv.flags(), MoveFlags::Castle);
        assert_eq!(mv.piece_type(), PieceType::King);
    }

    // ── Starting position move generation ──

    #[test]
    fn test_starting_position_red_move_count() {
        let mut board = Board::starting_position();
        let moves = generate_legal_moves(&mut board);
        // 8 single pushes + 8 double pushes + 2 knights * 2 moves each = 20
        assert_eq!(moves.len(), 20, "Red should have 20 legal moves");
    }

    // ── make/unmake Zobrist round-trip ──

    #[test]
    fn test_make_unmake_zobrist_roundtrip_all_starting_moves() {
        let mut board = Board::starting_position();
        let original_hash = board.zobrist_hash();
        let original_board = board.clone();

        let moves = generate_legal_moves(&mut board);
        for mv in moves {
            let undo = make_move(&mut board, mv);
            unmake_move(&mut board, &undo);
            assert_eq!(
                board.zobrist_hash(),
                original_hash,
                "Zobrist mismatch after make/unmake of {:?}",
                mv
            );
            assert_eq!(
                board, original_board,
                "Board mismatch after make/unmake of {:?}",
                mv
            );
            board.assert_piece_list_sync();
        }
    }

    // ── Pawn push direction per player ──

    #[test]
    fn test_pawn_push_direction_red() {
        let mut board = Board::empty();
        let sq = Square::new(1, 4).unwrap();
        board.set_piece(sq, Piece::new(PieceType::Pawn, Player::Red));
        let king_sq = Square::new(0, 7).unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Red));
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let targets: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn)
            .map(|m| m.to_sq())
            .collect();
        assert!(
            targets.contains(&Square::new(2, 4).unwrap()),
            "Red pawn should push to e3"
        );
        assert!(
            targets.contains(&Square::new(3, 4).unwrap()),
            "Red pawn should double push to e4"
        );
    }

    #[test]
    fn test_pawn_push_direction_blue() {
        let mut board = Board::empty();
        let sq = Square::new(4, 1).unwrap();
        board.set_piece(sq, Piece::new(PieceType::Pawn, Player::Blue));
        let king_sq = Square::new(6, 0).unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Blue));
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let targets: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn)
            .map(|m| m.to_sq())
            .collect();
        assert!(
            targets.contains(&Square::new(4, 2).unwrap()),
            "Blue pawn should push east"
        );
        assert!(
            targets.contains(&Square::new(4, 3).unwrap()),
            "Blue pawn should double push east"
        );
    }

    #[test]
    fn test_pawn_push_direction_yellow() {
        let mut board = Board::empty();
        let sq = Square::new(12, 4).unwrap();
        board.set_piece(sq, Piece::new(PieceType::Pawn, Player::Yellow));
        let king_sq = Square::new(13, 7).unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Yellow));
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let targets: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn)
            .map(|m| m.to_sq())
            .collect();
        assert!(
            targets.contains(&Square::new(11, 4).unwrap()),
            "Yellow pawn should push south"
        );
        assert!(
            targets.contains(&Square::new(10, 4).unwrap()),
            "Yellow pawn should double push south"
        );
    }

    #[test]
    fn test_pawn_push_direction_green() {
        let mut board = Board::empty();
        let sq = Square::new(4, 12).unwrap();
        board.set_piece(sq, Piece::new(PieceType::Pawn, Player::Green));
        let king_sq = Square::new(7, 13).unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Green));
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let targets: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn)
            .map(|m| m.to_sq())
            .collect();
        assert!(
            targets.contains(&Square::new(4, 11).unwrap()),
            "Green pawn should push west"
        );
        assert!(
            targets.contains(&Square::new(4, 10).unwrap()),
            "Green pawn should double push west"
        );
    }

    // ── Castling per player ──

    #[test]
    fn test_castling_red_kingside() {
        let mut board = Board::empty();
        board.set_piece(Square(7), Piece::new(PieceType::King, Player::Red));
        board.set_piece(Square(10), Piece::new(PieceType::Rook, Player::Red));
        board.set_castling_rights(1);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(9));
    }

    #[test]
    fn test_castling_red_queenside() {
        let mut board = Board::empty();
        board.set_piece(Square(7), Piece::new(PieceType::King, Player::Red));
        board.set_piece(Square(3), Piece::new(PieceType::Rook, Player::Red));
        board.set_castling_rights(2);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(5));
    }

    #[test]
    fn test_castling_blue_kingside() {
        let mut board = Board::empty();
        board.set_piece(Square(84), Piece::new(PieceType::King, Player::Blue));
        board.set_piece(Square(42), Piece::new(PieceType::Rook, Player::Blue));
        board.set_castling_rights(4);
        board.set_side_to_move(Player::Blue);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(56));
    }

    #[test]
    fn test_castling_blue_queenside() {
        let mut board = Board::empty();
        board.set_piece(Square(84), Piece::new(PieceType::King, Player::Blue));
        board.set_piece(Square(140), Piece::new(PieceType::Rook, Player::Blue));
        board.set_castling_rights(8);
        board.set_side_to_move(Player::Blue);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(112));
    }

    #[test]
    fn test_castling_yellow_kingside() {
        let mut board = Board::empty();
        board.set_piece(Square(188), Piece::new(PieceType::King, Player::Yellow));
        board.set_piece(Square(185), Piece::new(PieceType::Rook, Player::Yellow));
        board.set_castling_rights(16);
        board.set_side_to_move(Player::Yellow);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(186));
    }

    #[test]
    fn test_castling_yellow_queenside() {
        let mut board = Board::empty();
        board.set_piece(Square(188), Piece::new(PieceType::King, Player::Yellow));
        board.set_piece(Square(192), Piece::new(PieceType::Rook, Player::Yellow));
        board.set_castling_rights(32);
        board.set_side_to_move(Player::Yellow);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(190));
    }

    #[test]
    fn test_castling_green_kingside() {
        let mut board = Board::empty();
        board.set_piece(Square(111), Piece::new(PieceType::King, Player::Green));
        board.set_piece(Square(153), Piece::new(PieceType::Rook, Player::Green));
        board.set_castling_rights(64);
        board.set_side_to_move(Player::Green);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(139));
    }

    #[test]
    fn test_castling_green_queenside() {
        let mut board = Board::empty();
        board.set_piece(Square(111), Piece::new(PieceType::King, Player::Green));
        board.set_piece(Square(55), Piece::new(PieceType::Rook, Player::Green));
        board.set_castling_rights(128);
        board.set_side_to_move(Player::Green);

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 1);
        assert_eq!(castles[0].to_sq(), Square(83));
    }

    // ── En passant per player ──

    #[test]
    fn test_en_passant_red() {
        let mut board = Board::empty();
        let pawn_sq = Square::new(5, 5).unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        let ep_sq = Square::new(6, 6).unwrap();
        board.set_en_passant(Some(ep_sq), Some(Player::Yellow));
        // Yellow pushes south (-1,0), pawn at (6+(-1),6+0) = (5,6)
        board.set_piece(
            Square::new(5, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::EnPassant)
            .collect();
        assert_eq!(ep_moves.len(), 1, "Red should have 1 EP move");
    }

    #[test]
    fn test_en_passant_blue() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        board.set_piece(
            Square::new(6, 0).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        let ep_sq = Square::new(6, 6).unwrap();
        board.set_en_passant(Some(ep_sq), Some(Player::Green));
        // Green pushes west (0,-1), pawn at (6+0,6+(-1)) = (6,5)
        board.set_piece(
            Square::new(6, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Green),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::EnPassant)
            .collect();
        assert_eq!(ep_moves.len(), 1, "Blue should have 1 EP move");
    }

    #[test]
    fn test_en_passant_yellow() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_piece(
            Square::new(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        let ep_sq = Square::new(4, 6).unwrap();
        board.set_en_passant(Some(ep_sq), Some(Player::Red));
        // Red pushes north (+1,0), pawn at (4+1,6+0) = (5,6)
        board.set_piece(
            Square::new(5, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::EnPassant)
            .collect();
        assert_eq!(ep_moves.len(), 1, "Yellow should have 1 EP move");
    }

    #[test]
    fn test_en_passant_green() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Green),
        );
        board.set_piece(
            Square::new(7, 13).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        let ep_sq = Square::new(6, 4).unwrap();
        board.set_en_passant(Some(ep_sq), Some(Player::Blue));
        // Blue pushes east (0,+1), pawn at (6+0,4+1) = (6,5)
        board.set_piece(
            Square::new(6, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::EnPassant)
            .collect();
        assert_eq!(ep_moves.len(), 1, "Green should have 1 EP move");
    }

    // ── Knight center moves ──

    #[test]
    fn test_knight_center_8_moves() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(6, 6).unwrap(),
            Piece::new(PieceType::Knight, Player::Red),
        );
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let knight_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Knight)
            .collect();
        assert_eq!(
            knight_moves.len(),
            8,
            "Knight in center should have 8 moves"
        );
    }

    // ── Promotion per player ──

    #[test]
    fn test_promotion_red() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(7, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let promos: Vec<_> = moves.iter().filter(|m| m.promotion().is_some()).collect();
        assert_eq!(promos.len(), 4, "Red should have 4 promotion choices");
    }

    #[test]
    fn test_promotion_blue() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(4, 7).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        board.set_piece(
            Square::new(6, 0).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let promos: Vec<_> = moves.iter().filter(|m| m.promotion().is_some()).collect();
        assert_eq!(promos.len(), 4, "Blue should have 4 promotion choices");
    }

    #[test]
    fn test_promotion_yellow() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(6, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_piece(
            Square::new(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let promos: Vec<_> = moves.iter().filter(|m| m.promotion().is_some()).collect();
        assert_eq!(promos.len(), 4, "Yellow should have 4 promotion choices");
    }

    #[test]
    fn test_promotion_green() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(4, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Green),
        );
        board.set_piece(
            Square::new(7, 13).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let promos: Vec<_> = moves.iter().filter(|m| m.promotion().is_some()).collect();
        assert_eq!(promos.len(), 4, "Green should have 4 promotion choices");
    }

    // ── Pawn captures per player ──

    #[test]
    fn test_pawn_capture_red() {
        let mut board = Board::empty();
        let pawn_sq = Square::new(4, 5).unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        // Enemy pieces on Red's capture diagonals: (+1,+1) and (+1,-1)
        board.set_piece(
            Square::new(5, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        board.set_piece(
            Square::new(5, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let captures: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn && m.is_capture())
            .collect();
        assert_eq!(
            captures.len(),
            2,
            "Red pawn should capture on both diagonals"
        );
    }

    #[test]
    fn test_pawn_capture_blue() {
        let mut board = Board::empty();
        let pawn_sq = Square::new(5, 4).unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Blue));
        board.set_piece(
            Square::new(6, 0).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        // Blue captures: (+1,+1) and (-1,+1)
        board.set_piece(
            Square::new(6, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(4, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let captures: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn && m.is_capture())
            .collect();
        assert_eq!(
            captures.len(),
            2,
            "Blue pawn should capture on both diagonals"
        );
    }

    #[test]
    fn test_pawn_capture_yellow() {
        let mut board = Board::empty();
        let pawn_sq = Square::new(9, 5).unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Yellow));
        board.set_piece(
            Square::new(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        // Yellow captures: (-1,+1) and (-1,-1)
        board.set_piece(
            Square::new(8, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(8, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Green),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let captures: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn && m.is_capture())
            .collect();
        assert_eq!(
            captures.len(),
            2,
            "Yellow pawn should capture on both diagonals"
        );
    }

    #[test]
    fn test_pawn_capture_green() {
        let mut board = Board::empty();
        let pawn_sq = Square::new(5, 9).unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Green));
        board.set_piece(
            Square::new(7, 13).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        // Green captures: (+1,-1) and (-1,-1)
        board.set_piece(
            Square::new(6, 8).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(4, 8).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let captures: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Pawn && m.is_capture())
            .collect();
        assert_eq!(
            captures.len(),
            2,
            "Green pawn should capture on both diagonals"
        );
    }

    // ── Knight moves per player ──

    #[test]
    fn test_knight_red() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(3, 5).unwrap(),
            Piece::new(PieceType::Knight, Player::Red),
        );
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let knight_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Knight)
            .collect();
        assert_eq!(
            knight_moves.len(),
            8,
            "Red knight at (3,5) should have 8 moves"
        );
    }

    #[test]
    fn test_knight_blue() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 3).unwrap(),
            Piece::new(PieceType::Knight, Player::Blue),
        );
        board.set_piece(
            Square::new(6, 0).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let knight_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Knight)
            .collect();
        assert_eq!(
            knight_moves.len(),
            8,
            "Blue knight at (5,3) should have 8 moves"
        );
    }

    #[test]
    fn test_knight_yellow() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(10, 8).unwrap(),
            Piece::new(PieceType::Knight, Player::Yellow),
        );
        board.set_piece(
            Square::new(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let knight_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Knight)
            .collect();
        assert_eq!(
            knight_moves.len(),
            8,
            "Yellow knight at (10,8) should have 8 moves"
        );
    }

    #[test]
    fn test_knight_green() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(8, 10).unwrap(),
            Piece::new(PieceType::Knight, Player::Green),
        );
        board.set_piece(
            Square::new(7, 13).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let knight_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Knight)
            .collect();
        assert_eq!(
            knight_moves.len(),
            8,
            "Green knight at (8,10) should have 8 moves"
        );
    }

    // ── Slider orientation per player ──

    #[test]
    fn test_rook_red() {
        let mut board = Board::empty();
        let rook_sq = Square::new(4, 7).unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));
        board.set_piece(
            Square::new(0, 3).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let rook_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Rook)
            .collect();
        // Rook on open board at (4,7): can slide in 4 directions
        assert!(
            rook_moves.len() > 10,
            "Red rook should have many moves on open board"
        );
    }

    #[test]
    fn test_bishop_blue() {
        let mut board = Board::empty();
        let bishop_sq = Square::new(7, 4).unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Blue));
        board.set_piece(
            Square::new(6, 0).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let bishop_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Bishop)
            .collect();
        assert!(
            bishop_moves.len() > 5,
            "Blue bishop should have many moves on open board"
        );
    }

    #[test]
    fn test_queen_yellow() {
        let mut board = Board::empty();
        let queen_sq = Square::new(7, 7).unwrap();
        board.set_piece(queen_sq, Piece::new(PieceType::Queen, Player::Yellow));
        board.set_piece(
            Square::new(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let queen_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Queen)
            .collect();
        // Queen at center of board should have many moves in all 8 directions
        assert!(
            queen_moves.len() > 20,
            "Yellow queen at center should have many moves"
        );
    }

    #[test]
    fn test_rook_green() {
        let mut board = Board::empty();
        let rook_sq = Square::new(7, 10).unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Green));
        board.set_piece(
            Square::new(7, 13).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let rook_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::Rook)
            .collect();
        assert!(
            rook_moves.len() > 10,
            "Green rook should have many moves on open board"
        );
    }

    // ── King moves per player ──

    #[test]
    fn test_king_red() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let king_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::King)
            .collect();
        assert_eq!(king_moves.len(), 8, "King in center should have 8 moves");
    }

    #[test]
    fn test_king_blue() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_side_to_move(Player::Blue);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let king_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::King)
            .collect();
        assert_eq!(king_moves.len(), 8, "King in center should have 8 moves");
    }

    #[test]
    fn test_king_yellow() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(8, 8).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.set_side_to_move(Player::Yellow);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let king_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::King)
            .collect();
        assert_eq!(king_moves.len(), 8, "King in center should have 8 moves");
    }

    #[test]
    fn test_king_green() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(8, 8).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Green);
        board.set_castling_rights(0);

        let moves = generate_legal_moves(&mut board);
        let king_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.piece_type() == PieceType::King)
            .collect();
        assert_eq!(king_moves.len(), 8, "King in center should have 8 moves");
    }

    // ── make/unmake stress test: all move types from starting position ──

    #[test]
    fn test_make_unmake_all_starting_moves() {
        let mut board = Board::starting_position();
        let original = board.clone();
        let original_hash = board.zobrist_hash();

        let moves = generate_legal_moves(&mut board);
        for mv in &moves {
            let undo = make_move(&mut board, *mv);
            unmake_move(&mut board, &undo);
            assert_eq!(
                board, original,
                "Board not restored after make/unmake of {:?}",
                mv
            );
            assert_eq!(
                board.zobrist_hash(),
                original_hash,
                "Zobrist not restored after {:?}",
                mv
            );
            board.assert_piece_list_sync();
        }
    }

    // ── Perft ──

    #[test]
    fn test_perft_depth_1() {
        let mut board = Board::starting_position();
        assert_eq!(perft(&mut board, 1), 20);
    }

    #[test]
    fn test_perft_depth_2() {
        let mut board = Board::starting_position();
        // Not 20*20=400: some Red moves block Blue double pushes or open lines.
        // Verified via perft divide: 395 is correct.
        assert_eq!(perft(&mut board, 2), 395);
    }

    #[test]
    fn test_perft_preserves_board_state() {
        let mut board = Board::starting_position();
        let original = board.clone();
        let original_hash = board.zobrist_hash();

        let _ = perft(&mut board, 3);

        assert_eq!(board, original, "Board should be unchanged after perft");
        assert_eq!(
            board.zobrist_hash(),
            original_hash,
            "Zobrist should be unchanged after perft"
        );
        board.assert_piece_list_sync();
    }

    // ── Safety checks ──

    #[test]
    fn test_no_castling_when_in_check() {
        let mut board = Board::empty();
        board.set_piece(Square(7), Piece::new(PieceType::King, Player::Red));
        board.set_piece(Square(10), Piece::new(PieceType::Rook, Player::Red));
        board.set_castling_rights(1);
        // Enemy rook attacks h1
        board.set_piece(
            Square::new(7, 7).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );

        let moves = generate_legal_moves(&mut board);
        let castles: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlags::Castle)
            .collect();
        assert_eq!(castles.len(), 0, "Cannot castle while in check");
    }

    #[test]
    fn test_no_moves_to_invalid_corners() {
        let mut board = Board::starting_position();
        let moves = generate_legal_moves(&mut board);
        for mv in &moves {
            assert!(
                mv.to_sq().is_valid(),
                "Move {:?} targets invalid square",
                mv
            );
            assert!(mv.from_sq().is_valid(), "Move {:?} from invalid square", mv);
        }
    }

    // ── make/unmake EP roundtrip ──

    #[test]
    fn test_make_unmake_en_passant_roundtrip() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        let ep_sq = Square::new(6, 6).unwrap();
        board.set_en_passant(Some(ep_sq), Some(Player::Yellow));
        board.set_piece(
            Square::new(5, 6).unwrap(),
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        board.set_castling_rights(0);

        let original = board.clone();
        let original_hash = board.zobrist_hash();

        let moves = generate_legal_moves(&mut board);
        let ep_mv = moves
            .iter()
            .find(|m| m.flags() == MoveFlags::EnPassant)
            .unwrap();

        let undo = make_move(&mut board, *ep_mv);
        // After EP: our pawn moved to (6,6), enemy pawn at (5,6) removed
        assert!(
            board.piece_at(Square::new(5, 6).unwrap()).is_none(),
            "Captured pawn should be gone"
        );
        assert!(
            board.piece_at(Square::new(6, 6).unwrap()).is_some(),
            "Our pawn should be at EP target"
        );

        unmake_move(&mut board, &undo);
        assert_eq!(board, original, "Board should be restored after EP unmake");
        assert_eq!(
            board.zobrist_hash(),
            original_hash,
            "Zobrist should be restored"
        );
    }

    // ── make/unmake castle roundtrip ──

    #[test]
    fn test_make_unmake_castle_roundtrip() {
        let mut board = Board::empty();
        board.set_piece(Square(7), Piece::new(PieceType::King, Player::Red));
        board.set_piece(Square(10), Piece::new(PieceType::Rook, Player::Red));
        board.set_castling_rights(1);

        let original = board.clone();
        let original_hash = board.zobrist_hash();

        let moves = generate_legal_moves(&mut board);
        let castle_mv = moves
            .iter()
            .find(|m| m.flags() == MoveFlags::Castle)
            .unwrap();

        let undo = make_move(&mut board, *castle_mv);
        assert!(board.piece_at(Square(9)).is_some(), "King should be at j1");
        assert!(board.piece_at(Square(8)).is_some(), "Rook should be at i1");

        unmake_move(&mut board, &undo);
        assert_eq!(
            board, original,
            "Board should be restored after castle unmake"
        );
        assert_eq!(
            board.zobrist_hash(),
            original_hash,
            "Zobrist should be restored"
        );
    }

    // ── make/unmake promotion roundtrip ──

    #[test]
    fn test_make_unmake_promotion_roundtrip() {
        let mut board = Board::empty();
        board.set_piece(
            Square::new(7, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.set_piece(
            Square::new(0, 7).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_castling_rights(0);

        let original = board.clone();
        let original_hash = board.zobrist_hash();

        let moves = generate_legal_moves(&mut board);
        let promo_mv = moves.iter().find(|m| m.promotion().is_some()).unwrap();

        let undo = make_move(&mut board, *promo_mv);
        unmake_move(&mut board, &undo);
        assert_eq!(board, original);
        assert_eq!(board.zobrist_hash(), original_hash);
    }
}
