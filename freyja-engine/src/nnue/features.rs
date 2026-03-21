//! NNUE feature encoding for four-player chess.
//!
//! Maps (perspective, square, piece_type, piece_player) → feature index.
//! Uses perspective-relative encoding: each player sees the board rotated
//! so that "self" is always relative_player 0.
//!
//! Stage 16: NNUE Architecture + Training Pipeline

use crate::board::types::*;

// ─── Architecture Constants ────────────────────────────────────────────────

/// Number of valid squares on the 14x14 board (excluding 36 corner squares).
pub const NUM_VALID_SQUARES: usize = VALID_SQUARES; // 160

/// Number of piece types in the feature encoding.
/// Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5, PromotedQueen=6.
pub const NUM_PIECE_TYPES: usize = 7;

/// Number of relative players (self=0, next=1, across=2, prev=3).
pub const NUM_RELATIVE_PLAYERS: usize = 4;

/// Total piece-square features per perspective.
/// 4 relative players × 7 piece types × 160 squares = 4,480.
pub const NUM_PIECE_SQ_FEATURES: usize = NUM_RELATIVE_PLAYERS * NUM_PIECE_TYPES * NUM_VALID_SQUARES;

/// Number of zone control summary features per perspective.
/// territory, frontier, net_influence, tension, defended, undefended, coordinated, pawn_chains.
pub const NUM_ZONE_FEATURES: usize = 8;

/// Total input features per perspective.
pub const TOTAL_FEATURES: usize = NUM_PIECE_SQ_FEATURES + NUM_ZONE_FEATURES; // 4488

/// First hidden layer size.
pub const HIDDEN1_SIZE: usize = 256;

/// Second hidden layer size.
pub const HIDDEN2_SIZE: usize = 32;

/// Output size per perspective (single scalar).
pub const OUTPUT_SIZE: usize = 1;

/// Quantization scale factor for weights (Q6 fixed-point).
pub const WEIGHT_SCALE: i32 = 64;

/// Combined scale for output conversion: WEIGHT_SCALE² = 4096.
pub const OUTPUT_SCALE: i32 = WEIGHT_SCALE * WEIGHT_SCALE;

// ─── Square-to-Valid-Index Mapping ─────────────────────────────────────────

/// Maps raw square index (0..195) to valid-square index (0..159).
/// Invalid squares map to 255.
pub static SQUARE_TO_VALID_INDEX: [u8; TOTAL_SQUARES] = {
    let mut table = [255u8; TOTAL_SQUARES];
    let mut valid_idx: u8 = 0;
    let mut i: usize = 0;
    while i < TOTAL_SQUARES {
        let rank = (i / BOARD_SIZE) as u8;
        let file = (i % BOARD_SIZE) as u8;
        if is_valid_square(rank, file) {
            table[i] = valid_idx;
            valid_idx += 1;
        }
        i += 1;
    }
    table
};

/// Maps valid-square index (0..159) back to raw square index.
pub static VALID_INDEX_TO_SQUARE: [u8; NUM_VALID_SQUARES] = {
    let mut table = [0u8; NUM_VALID_SQUARES];
    let mut valid_idx: usize = 0;
    let mut i: usize = 0;
    while i < TOTAL_SQUARES {
        let rank = (i / BOARD_SIZE) as u8;
        let file = (i % BOARD_SIZE) as u8;
        if is_valid_square(rank, file) {
            table[valid_idx] = i as u8;
            valid_idx += 1;
        }
        i += 1;
    }
    table
};

// ─── Feature Index Computation ─────────────────────────────────────────────

/// Compute the relative player index (0-3) for a piece owner seen from a perspective.
///
/// relative_player 0 = "self" (perspective player's own pieces)
/// relative_player 1 = "next" (clockwise neighbor)
/// relative_player 2 = "across" (opposite player)
/// relative_player 3 = "prev" (counter-clockwise neighbor)
#[inline]
pub fn relative_player(perspective: Player, piece_player: Player) -> usize {
    (piece_player.index() + PLAYERS - perspective.index()) % PLAYERS
}

/// Compute the piece-square feature index for a piece on a given square,
/// as seen from a specific perspective.
///
/// Returns `None` if the square is invalid.
///
/// Formula: relative_player * (7 * 160) + piece_type_index * 160 + valid_square_index
#[inline]
pub fn feature_index(
    perspective: Player,
    sq: Square,
    piece_type: PieceType,
    piece_player: Player,
) -> Option<u16> {
    let valid_idx = SQUARE_TO_VALID_INDEX[sq.index() as usize];
    if valid_idx == 255 {
        return None;
    }

    let rel_player = relative_player(perspective, piece_player);
    let pt_idx = piece_type as usize;

    let idx = rel_player * (NUM_PIECE_TYPES * NUM_VALID_SQUARES)
        + pt_idx * NUM_VALID_SQUARES
        + valid_idx as usize;

    debug_assert!(idx < NUM_PIECE_SQ_FEATURES);
    Some(idx as u16)
}

/// Compute the zone feature index (offset from piece-square features).
///
/// Zone feature indices start at NUM_PIECE_SQ_FEATURES (4480).
/// `zone_idx` must be in 0..NUM_ZONE_FEATURES.
#[inline]
pub fn zone_feature_index(zone_idx: usize) -> u16 {
    debug_assert!(zone_idx < NUM_ZONE_FEATURES);
    (NUM_PIECE_SQ_FEATURES + zone_idx) as u16
}

/// Collect all active piece-square feature indices for a board position
/// from a given perspective.
///
/// Returns the number of features written into `out`.
pub fn collect_active_features(
    board: &crate::board::Board,
    perspective: Player,
    out: &mut [u16; 128], // max pieces on board = 4 * 32 = 128
) -> usize {
    let mut count = 0;
    for player in Player::all() {
        for (piece_type, sq) in board.pieces(player) {
            if let Some(idx) = feature_index(perspective, sq, piece_type, player) {
                out[count] = idx;
                count += 1;
            }
        }
    }
    count
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_square_mapping_count() {
        let count = SQUARE_TO_VALID_INDEX.iter().filter(|&&v| v != 255).count();
        assert_eq!(count, NUM_VALID_SQUARES);
    }

    #[test]
    fn test_valid_index_round_trip() {
        for valid_idx in 0..NUM_VALID_SQUARES {
            let raw_sq = VALID_INDEX_TO_SQUARE[valid_idx];
            assert_eq!(
                SQUARE_TO_VALID_INDEX[raw_sq as usize], valid_idx as u8,
                "Round-trip failed for valid_idx={valid_idx}"
            );
        }
    }

    #[test]
    fn test_relative_player_self_is_zero() {
        for p in Player::all() {
            assert_eq!(relative_player(p, p), 0, "Self should be relative 0");
        }
    }

    #[test]
    fn test_relative_player_next() {
        assert_eq!(relative_player(Player::Red, Player::Blue), 1);
        assert_eq!(relative_player(Player::Blue, Player::Yellow), 1);
        assert_eq!(relative_player(Player::Yellow, Player::Green), 1);
        assert_eq!(relative_player(Player::Green, Player::Red), 1);
    }

    #[test]
    fn test_relative_player_across() {
        assert_eq!(relative_player(Player::Red, Player::Yellow), 2);
        assert_eq!(relative_player(Player::Blue, Player::Green), 2);
    }

    #[test]
    fn test_relative_player_prev() {
        assert_eq!(relative_player(Player::Red, Player::Green), 3);
        assert_eq!(relative_player(Player::Blue, Player::Red), 3);
    }

    #[test]
    fn test_feature_index_bounds() {
        // Test all combinations produce valid indices
        for perspective in Player::all() {
            for &raw_sq in VALID_SQUARES_LIST.iter() {
                let sq = Square::from_index(raw_sq).unwrap();
                for player in Player::all() {
                    for pt_idx in 0..NUM_PIECE_TYPES {
                        let pt = match pt_idx {
                            0 => PieceType::Pawn,
                            1 => PieceType::Knight,
                            2 => PieceType::Bishop,
                            3 => PieceType::Rook,
                            4 => PieceType::Queen,
                            5 => PieceType::King,
                            6 => PieceType::PromotedQueen,
                            _ => unreachable!(),
                        };
                        let idx = feature_index(perspective, sq, pt, player).unwrap();
                        assert!(
                            (idx as usize) < NUM_PIECE_SQ_FEATURES,
                            "Feature index {idx} out of bounds for perspective={perspective}, sq={sq}, pt={pt_idx}, player={player}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_feature_index_uniqueness_for_different_perspectives() {
        // Same piece on same square should produce different feature indices
        // from different perspectives (because relative_player changes).
        let sq = Square::new(4, 4).unwrap();
        let pt = PieceType::Pawn;
        let owner = Player::Red;

        let idx_from_red = feature_index(Player::Red, sq, pt, owner).unwrap();
        let idx_from_blue = feature_index(Player::Blue, sq, pt, owner).unwrap();
        let idx_from_yellow = feature_index(Player::Yellow, sq, pt, owner).unwrap();
        let idx_from_green = feature_index(Player::Green, sq, pt, owner).unwrap();

        // Red sees Red as self (rel=0), Blue sees Red as prev (rel=3)
        assert_ne!(idx_from_red, idx_from_blue);
        assert_ne!(idx_from_red, idx_from_yellow);
        assert_ne!(idx_from_red, idx_from_green);
    }

    #[test]
    fn test_feature_index_symmetry() {
        // Red seeing Blue's piece should have the same relative offset
        // as Blue seeing Yellow's piece (both rel_player=1).
        let sq = Square::new(5, 5).unwrap();
        let pt = PieceType::Knight;

        let red_sees_blue = feature_index(Player::Red, sq, pt, Player::Blue).unwrap();
        let blue_sees_yellow = feature_index(Player::Blue, sq, pt, Player::Yellow).unwrap();

        // Same square, same piece type, same relative player (1) → same index
        assert_eq!(red_sees_blue, blue_sees_yellow);
    }

    #[test]
    fn test_feature_index_invalid_square() {
        // Corner square (0,0) is invalid
        let sq = Square(0); // rank=0, file=0 → invalid corner
        assert!(feature_index(Player::Red, sq, PieceType::Pawn, Player::Red).is_none());
    }

    #[test]
    fn test_zone_feature_index() {
        assert_eq!(zone_feature_index(0) as usize, NUM_PIECE_SQ_FEATURES);
        assert_eq!(
            zone_feature_index(NUM_ZONE_FEATURES - 1) as usize,
            TOTAL_FEATURES - 1
        );
    }

    #[test]
    fn test_collect_active_features_starting_position() {
        let board = crate::board::Board::starting_position();
        let mut features = [0u16; 128];
        let count = collect_active_features(&board, Player::Red, &mut features);
        // Starting position: 4 players × 16 pieces = 64 pieces
        assert_eq!(count, 64);
        // All indices should be in bounds
        for &idx in &features[..count] {
            assert!((idx as usize) < NUM_PIECE_SQ_FEATURES);
        }
    }

    #[test]
    fn test_all_perspectives_same_feature_count() {
        let board = crate::board::Board::starting_position();
        let mut features = [0u16; 128];
        for perspective in Player::all() {
            let count = collect_active_features(&board, perspective, &mut features);
            assert_eq!(count, 64, "All perspectives should see 64 pieces");
        }
    }
}
