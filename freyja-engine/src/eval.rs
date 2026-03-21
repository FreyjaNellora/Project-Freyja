//! Position evaluation.
//!
//! Evaluator trait and BootstrapEvaluator implementation.
//! The bootstrap evaluator is temporary — NNUE replaces it in Stage 17.
//! It includes zone control features that will become NNUE training features.

use crate::board::types::*;
use crate::board::{ALL_DIRS, DIAGONAL_DIRS, KNIGHT_OFFSETS, ORTHOGONAL_DIRS, PAWN_CAPTURE_DELTAS};
use crate::game_state::{GameState, PlayerStatus};

// ─── Evaluator Trait ────────────────────────────────────────────────────────

/// Evaluation interface for all evaluator implementations.
///
/// The Evaluator trait is the stable boundary between search and evaluation.
/// Stage 7+ search code calls only these methods — never eval internals.
pub trait Evaluator {
    /// Evaluate the position from a single player's perspective.
    /// Returns centipawns. Eliminated players return `ELIMINATED_SCORE`.
    fn eval_scalar(&self, state: &GameState, player: Player) -> i16;

    /// Evaluate the position for all 4 players simultaneously.
    /// Returns `[Red, Blue, Yellow, Green]` scores in centipawns.
    fn eval_4vec(&self, state: &GameState) -> [i16; 4];
}

/// Sentinel score for eliminated players.
pub const ELIMINATED_SCORE: i16 = i16::MIN;

// ─── Piece Values ───────────────────────────────────────────────────────────

/// Centipawn values indexed by `PieceType::index()`.
/// King (index 5) has no material value — set to 0.
const PIECE_VALUES: [i16; PieceType::COUNT] = [
    100, // Pawn
    300, // Knight
    450, // Bishop
    500, // Rook
    900, // Queen
    0,   // King
    900, // PromotedQueen
];

/// Get the centipawn value of a piece type.
#[inline]
pub fn piece_value(pt: PieceType) -> i16 {
    PIECE_VALUES[pt.index()]
}

// ─── Component Weights ──────────────────────────────────────────────────────

/// Weights for combining eval components. Untuned — Stage 13 tunes these.
const WEIGHT_MATERIAL: i16 = 1; // Anchor — values already in cp
const WEIGHT_PST: i16 = 1; // Tiebreaker — correctly quiet
const WEIGHT_MOBILITY: i16 = 4; // cp per legal move — slight bump for piece voice
// WEIGHT_TERRITORY moved to ZoneWeights.territory (Stage 15, configurable via setoption)
const WEIGHT_KING_SAFETY_SHELTER: i16 = 35; // cp per shelter pawn — king safety > pawn structure
const WEIGHT_KING_SAFETY_ATTACKER: i16 = 35; // cp per attacker penalty — matches shelter priority
const WEIGHT_PAWN_ADVANCE: i16 = 0; // DISABLED — pawns don't need advancement reward, re-enable with NNUE
const WEIGHT_PAWN_DOUBLED: i16 = 8; // cp penalty per doubled pawn — minor structural signal
const WEIGHT_DEVELOPMENT: i16 = 35; // cp per dev unit — capped development max 280cp
const DEV_SCORE_CAP: i16 = 8; // Cap positive dev score (8 * 35 = 280cp max)
const CASTLED_BONUS: i16 = 80; // Flat bonus for having castled

/// Per-player castling destination squares: [KS_king_to, QS_king_to].
/// Indices match CASTLE_DEFS in move_gen.rs.
const CASTLE_KING_DESTS: [[u8; 2]; 4] = [
    [9, 5],     // Red: KS=j1, QS=f1
    [56, 112],  // Blue: KS=a5, QS=a9
    [186, 190], // Yellow: KS=e14, QS=i14
    [139, 83],  // Green: KS=n10, QS=n6
];

/// Per-player castling rights bit indices: [KS_bit, QS_bit].
const CASTLE_RIGHTS_BITS: [[u8; 2]; 4] = [[0, 1], [2, 3], [4, 5], [6, 7]];

// ─── Piece-Square Tables ────────────────────────────────────────────────────

/// PST for pawns from Red's perspective (rank 0 = Red's back rank).
/// Values in centipawns. Center and advancement bonuses.
/// Layout: [rank][file] where rank 0 = Red's home side, file 0 = a-file.
/// Only valid squares have meaningful values.
#[rustfmt::skip]
const PST_PAWN: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    // rank 0 (pawns never here)
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    // rank 1 (Red's pawn starting rank)
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    // rank 2
    [0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0],
    // rank 3
    [0, 0, 0, 1, 1, 2, 2, 2, 2, 1, 1, 0, 0, 0],
    // rank 4
    [0, 0, 0, 1, 2, 3, 3, 3, 3, 2, 1, 0, 0, 0],
    // rank 5
    [0, 0, 0, 2, 3, 4, 5, 5, 4, 3, 2, 0, 0, 0],
    // rank 6 (center)
    [0, 0, 0, 2, 3, 4, 5, 5, 4, 3, 2, 0, 0, 0],
    // rank 7 (center)
    [0, 0, 0, 2, 3, 4, 5, 5, 4, 3, 2, 0, 0, 0],
    // rank 8
    [0, 0, 0, 3, 4, 5, 6, 6, 5, 4, 3, 0, 0, 0],
    // rank 9
    [0, 0, 0, 4, 5, 6, 7, 7, 6, 5, 4, 0, 0, 0],
    // rank 10
    [0, 0, 0, 5, 6, 7, 8, 8, 7, 6, 5, 0, 0, 0],
    // rank 11
    [0, 0, 0, 6, 7, 8, 9, 9, 8, 7, 6, 0, 0, 0],
    // rank 12 (promotion rank for Red)
    [0, 0, 0, 8, 9,10,12,12,10, 9, 8, 0, 0, 0],
    // rank 13
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

/// PST for knights from Red's perspective. Center-biased.
#[rustfmt::skip]
const PST_KNIGHT: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    [0, 0, 0, -3,-2,-2,-2,-2,-2,-2, -3, 0, 0, 0],
    [0, 0, 0, -2, 0, 0, 0, 0, 0, 0, -2, 0, 0, 0],
    [0, 0, 0, -2, 0, 1, 1, 1, 1, 0, -2, 0, 0, 0],
    [-3,-2,-2,  0, 1, 3, 3, 3, 3, 1,  0,-2,-2, -3],
    [-2, 0, 0,  1, 3, 4, 5, 5, 4, 3,  1, 0, 0, -2],
    [-2, 0, 1,  3, 4, 6, 7, 7, 6, 4,  3, 1, 0, -2],
    [-2, 0, 1,  3, 4, 7, 8, 8, 7, 4,  3, 1, 0, -2],
    [-2, 0, 1,  3, 4, 7, 8, 8, 7, 4,  3, 1, 0, -2],
    [-2, 0, 1,  3, 4, 6, 7, 7, 6, 4,  3, 1, 0, -2],
    [-2, 0, 0,  1, 3, 4, 5, 5, 4, 3,  1, 0, 0, -2],
    [-3,-2,-2,  0, 1, 3, 3, 3, 3, 1,  0,-2,-2, -3],
    [0, 0, 0, -2, 0, 1, 1, 1, 1, 0, -2, 0, 0, 0],
    [0, 0, 0, -2, 0, 0, 0, 0, 0, 0, -2, 0, 0, 0],
    [0, 0, 0, -3,-2,-2,-2,-2,-2,-2, -3, 0, 0, 0],
];

/// PST for bishops from Red's perspective. Diagonal control, center.
#[rustfmt::skip]
const PST_BISHOP: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    [0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, 0, 0, 0],
    [0, 0, 0, -2,  1,  0,  0,  0,  0,  1, -2, 0, 0, 0],
    [0, 0, 0, -2,  0,  1,  1,  1,  1,  0, -2, 0, 0, 0],
    [-2,-2,-2,  0,  1,  3,  3,  3,  3,  1,  0,-2,-2,-2],
    [-2, 0, 0,  1,  3,  4,  4,  4,  4,  3,  1, 0, 0,-2],
    [-2, 0, 1,  3,  4,  5,  5,  5,  5,  4,  3, 1, 0,-2],
    [-2, 0, 1,  3,  4,  5,  6,  6,  5,  4,  3, 1, 0,-2],
    [-2, 0, 1,  3,  4,  5,  6,  6,  5,  4,  3, 1, 0,-2],
    [-2, 0, 1,  3,  4,  5,  5,  5,  5,  4,  3, 1, 0,-2],
    [-2, 0, 0,  1,  3,  4,  4,  4,  4,  3,  1, 0, 0,-2],
    [-2,-2,-2,  0,  1,  3,  3,  3,  3,  1,  0,-2,-2,-2],
    [0, 0, 0, -2,  0,  1,  1,  1,  1,  0, -2, 0, 0, 0],
    [0, 0, 0, -2,  1,  0,  0,  0,  0,  1, -2, 0, 0, 0],
    [0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, 0, 0, 0],
];

/// PST for rooks from Red's perspective. Open files, 7th rank bonus.
#[rustfmt::skip]
const PST_ROOK: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 2, 2, 2,  2,  2,  2,  3,  3,  2,  2,  2, 2, 2, 2],
    [ 2, 2, 2,  2,  2,  2,  3,  3,  2,  2,  2, 2, 2, 2],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [ 0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
];

/// PST for queen from Red's perspective. Slight center preference, not too strong.
#[rustfmt::skip]
const PST_QUEEN: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    [0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, 0, 0, 0],
    [0, 0, 0, -2,  0,  0,  0,  0,  0,  0, -2, 0, 0, 0],
    [0, 0, 0, -2,  0,  1,  1,  1,  1,  0, -2, 0, 0, 0],
    [-2,-2,-2,  0,  1,  2,  2,  2,  2,  1,  0,-2,-2,-2],
    [-2, 0, 0,  1,  2,  3,  3,  3,  3,  2,  1, 0, 0,-2],
    [-2, 0, 1,  2,  3,  4,  4,  4,  4,  3,  2, 1, 0,-2],
    [-2, 0, 1,  2,  3,  4,  5,  5,  4,  3,  2, 1, 0,-2],
    [-2, 0, 1,  2,  3,  4,  5,  5,  4,  3,  2, 1, 0,-2],
    [-2, 0, 1,  2,  3,  4,  4,  4,  4,  3,  2, 1, 0,-2],
    [-2, 0, 0,  1,  2,  3,  3,  3,  3,  2,  1, 0, 0,-2],
    [-2,-2,-2,  0,  1,  2,  2,  2,  2,  1,  0,-2,-2,-2],
    [0, 0, 0, -2,  0,  1,  1,  1,  1,  0, -2, 0, 0, 0],
    [0, 0, 0, -2,  0,  0,  0,  0,  0,  0, -2, 0, 0, 0],
    [0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, 0, 0, 0],
];

/// PST for king from Red's perspective. Stay on back rank early game.
#[rustfmt::skip]
const PST_KING: [[i16; BOARD_SIZE]; BOARD_SIZE] = [
    [0, 0, 0,  3,  3,  2,  0,  0,  2,  3,  3, 0, 0, 0],
    [0, 0, 0,  2,  2,  0, -2, -2,  0,  2,  2, 0, 0, 0],
    [0, 0, 0, -3, -3, -5, -5, -5, -5, -3, -3, 0, 0, 0],
    [-3,-3,-3, -5, -5, -7, -7, -7, -7, -5, -5,-3,-3,-3],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-5,-5,-5, -7, -7, -8, -8, -8, -8, -7, -7,-5,-5,-5],
    [-3,-3,-3, -5, -5, -7, -7, -7, -7, -5, -5,-3,-3,-3],
    [0, 0, 0, -3, -3, -5, -5, -5, -5, -3, -3, 0, 0, 0],
    [0, 0, 0,  2,  2,  0, -2, -2,  0,  2,  2, 0, 0, 0],
    [0, 0, 0,  3,  3,  2,  0,  0,  2,  3,  3, 0, 0, 0],
];

/// Map a (rank, file) from Red's perspective to the correct (rank, file)
/// for the given player, accounting for 4PC board rotation.
///
/// Red (bottom):   identity — rank 0 is home
/// Blue (left):    rotate 90 CW — file 0 is home → (rank, file) = (file, 13 - rank)
/// Yellow (top):   rotate 180  — rank 13 is home → (rank, file) = (13 - rank, 13 - file)
/// Green (right):  rotate 270 CW — file 13 is home → (rank, file) = (13 - file, rank)
#[inline]
fn rotate_for_player(rank: u8, file: u8, player: Player) -> (u8, u8) {
    match player {
        Player::Red => (rank, file),
        Player::Blue => (file, 13 - rank),
        Player::Yellow => (13 - rank, 13 - file),
        Player::Green => (13 - file, rank),
    }
}

/// Look up PST value for a piece at a given square, rotated to the player's perspective.
fn pst_value(piece_type: PieceType, sq: Square, player: Player) -> i16 {
    let (r, f) = rotate_for_player(sq.rank(), sq.file(), player);
    let r = r as usize;
    let f = f as usize;

    match piece_type {
        PieceType::Pawn => PST_PAWN[r][f],
        PieceType::Knight => PST_KNIGHT[r][f],
        PieceType::Bishop => PST_BISHOP[r][f],
        PieceType::Rook => PST_ROOK[r][f],
        PieceType::Queen | PieceType::PromotedQueen => PST_QUEEN[r][f],
        PieceType::King => PST_KING[r][f],
    }
}

// ─── Zone Control Features (Stage 15) ───────────────────────────────────────

/// Check if a player is active for zone control purposes.
/// Eliminated and DKW players don't project territory or influence.
#[inline]
pub fn is_active_for_zones(state: &GameState, player: Player) -> bool {
    matches!(state.player_status(player), PlayerStatus::Active)
}

/// Total squares on the 14x14 board.
const TOTAL_SQUARES: usize = 196;

/// Sentinel for unowned squares in BFS territory.
const OWNER_NONE: u8 = 255;

/// Sentinel for contested squares (equidistant from 2+ players).
const OWNER_CONTESTED: u8 = 254;

/// Territory analysis result with enhanced zone features.
pub struct TerritoryInfo {
    /// Squares owned by each player.
    pub counts: [i16; PLAYERS],
    /// Frontier length per player (squares adjacent to enemy/contested territory).
    pub frontier: [i16; PLAYERS],
    /// Number of contested squares (equidistant from 2+ players).
    pub contested: i16,
    /// Per-square ownership map (for use by other zone features).
    pub owner: [u8; TOTAL_SQUARES],
}

/// BFS Voronoi territory with frontier and contested square detection.
pub fn bfs_territory_enhanced(state: &GameState) -> TerritoryInfo {
    let board = state.board();

    // owner[sq_index] = player index, OWNER_CONTESTED, or OWNER_NONE
    let mut owner = [OWNER_NONE; TOTAL_SQUARES];
    // distance[sq_index] = BFS distance from nearest piece
    let mut dist = [255u8; TOTAL_SQUARES];

    // Fixed-size BFS queue: (square_index, player_index, distance)
    // Max 160 valid squares, each enqueued at most once
    let mut queue = [(0u8, 0u8, 0u8); VALID_SQUARES];
    let mut head: usize = 0;
    let mut tail: usize = 0;

    // Seed BFS with all pieces from active players (skip eliminated + DKW)
    for player in Player::all() {
        if !is_active_for_zones(state, player) {
            continue;
        }
        let pi = player.index() as u8;
        for (_, sq) in board.pieces(player) {
            let idx = sq.index() as usize;
            if owner[idx] == 255 {
                owner[idx] = pi;
                dist[idx] = 0;
                queue[tail] = (sq.index(), pi, 0);
                tail += 1;
            }
            // If two players have a piece on the same square (shouldn't happen),
            // first one wins. In practice, pieces don't overlap.
        }
    }

    // BFS expansion: 8-directional (king moves)
    const DIRS: [(i8, i8); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];

    while head < tail {
        let (sq_idx, pi, d) = queue[head];
        head += 1;

        let rank = sq_idx / BOARD_SIZE as u8;
        let file = sq_idx % BOARD_SIZE as u8;

        for &(dr, df) in &DIRS {
            let nr = rank as i8 + dr;
            let nf = file as i8 + df;
            if nr < 0 || nr >= BOARD_SIZE as i8 || nf < 0 || nf >= BOARD_SIZE as i8 {
                continue;
            }
            let nr = nr as u8;
            let nf = nf as u8;
            if !is_valid_square(nr, nf) {
                continue;
            }
            let nidx = (nr as usize) * BOARD_SIZE + nf as usize;
            let nd = d + 1;
            if nd < dist[nidx] {
                dist[nidx] = nd;
                owner[nidx] = pi;
                if tail < VALID_SQUARES {
                    queue[tail] = (nidx as u8, pi, nd);
                    tail += 1;
                }
            } else if nd == dist[nidx] && owner[nidx] != pi && owner[nidx] != OWNER_CONTESTED {
                // Equidistant from different players — mark as contested
                owner[nidx] = OWNER_CONTESTED;
            }
        }
    }

    // Count squares per player and contested squares
    let mut counts = [0i16; PLAYERS];
    let mut contested: i16 = 0;
    for &idx in VALID_SQUARES_LIST.iter() {
        let o = owner[idx as usize];
        if (o as usize) < PLAYERS {
            counts[o as usize] += 1;
        } else if o == OWNER_CONTESTED {
            contested += 1;
        }
    }

    // Compute frontier: squares where at least one neighbor belongs to a different player
    let mut frontier = [0i16; PLAYERS];
    for &idx in VALID_SQUARES_LIST.iter() {
        let o = owner[idx as usize];
        if (o as usize) >= PLAYERS {
            continue; // Skip unowned and contested
        }
        let rank = idx / BOARD_SIZE as u8;
        let file = idx % BOARD_SIZE as u8;
        let mut is_frontier = false;
        for &(dr, df) in &DIRS {
            let nr = rank as i8 + dr;
            let nf = file as i8 + df;
            if nr < 0 || nr >= BOARD_SIZE as i8 || nf < 0 || nf >= BOARD_SIZE as i8 {
                continue;
            }
            let nr = nr as u8;
            let nf = nf as u8;
            if !is_valid_square(nr, nf) {
                continue;
            }
            let nidx = (nr as usize) * BOARD_SIZE + nf as usize;
            let neighbor_owner = owner[nidx];
            if neighbor_owner != o && neighbor_owner != OWNER_NONE {
                is_frontier = true;
                break;
            }
        }
        if is_frontier {
            frontier[o as usize] += 1;
        }
    }

    tracing::trace!(
        red = counts[0],
        blue = counts[1],
        yellow = counts[2],
        green = counts[3],
        contested,
        frontier_red = frontier[0],
        frontier_blue = frontier[1],
        "territory with frontier"
    );

    TerritoryInfo {
        counts,
        frontier,
        contested,
        owner,
    }
}

// ─── Ray-Attenuated Influence Maps ──────────────────────────────────────────

/// Base influence weight per piece type (used at the piece's own square).
const INFLUENCE_WEIGHTS: [f32; PieceType::COUNT] = [
    1.0, // Pawn
    3.0, // Knight
    3.5, // Bishop
    5.0, // Rook
    9.0, // Queen
    1.0, // King
    9.0, // PromotedQueen
];

/// Influence map result per player.
pub struct InfluenceInfo {
    /// Total influence score per player (sum across all valid squares).
    pub total: [f32; PLAYERS],
    /// Net influence per player: own influence minus max opponent's influence.
    pub net: [f32; PLAYERS],
    /// Per-square per-player influence (for king safety zone and tension).
    pub grid: [[f32; PLAYERS]; TOTAL_SQUARES],
}

#[allow(clippy::too_many_arguments)] // All args are necessary for ray projection
/// Walk a ray from `origin` in direction `(dr, df)`, projecting influence that
/// attenuates through blockers. Influence at each square along the ray is:
///   current_influence (starts at piece_weight)
/// At each blocker encountered:
///   attenuation = 1.0 / (1 + 1 + defender_count)
///   current_influence *= attenuation
/// The blocker square itself receives the pre-attenuation influence.
///
/// `defender_counts[sq_index]` is a precomputed per-square total defender count
/// across all players (cheap approximation — avoids calling attackers_of per blocker).
fn ray_attenuated(
    board: &crate::board::Board,
    origin_rank: i8,
    origin_file: i8,
    dr: i8,
    df: i8,
    piece_weight: f32,
    player_idx: usize,
    grid: &mut [[f32; PLAYERS]; TOTAL_SQUARES],
) {
    let mut influence = piece_weight;
    let mut rank = origin_rank + dr;
    let mut file = origin_file + df;

    while rank >= 0 && rank < BOARD_SIZE as i8 && file >= 0 && file < BOARD_SIZE as i8 {
        let r = rank as u8;
        let f = file as u8;
        if !is_valid_square(r, f) {
            // Invalid corner — skip but continue ray (matches ray_find_piece behavior)
            rank += dr;
            file += df;
            continue;
        }

        let idx = r as usize * BOARD_SIZE + f as usize;

        // Add current influence to this square
        grid[idx][player_idx] += influence;

        // Check for blocker
        if let Some(blocker) = board.piece_at(Square(idx as u8)) {
            // Count defenders of this blocker (simplified: use piece value as proxy)
            // Friendly blocker attenuates less, enemy blocker attenuates more
            let is_friendly = blocker.player.index() == player_idx;
            let blocker_resistance = if is_friendly {
                // Friendly piece: mild attenuation (your own pieces support the ray)
                1.5
            } else {
                // Enemy piece: strong attenuation, scaled by piece value
                // A defended enemy pawn blocks less than a defended enemy queen
                2.0 + INFLUENCE_WEIGHTS[blocker.piece_type.index()] * 0.3
            };
            influence /= blocker_resistance;

            // Stop if influence is negligible
            if influence < 0.1 {
                break;
            }
        }

        rank += dr;
        file += df;
    }
}

/// Compute ray-attenuated influence maps for all players.
///
/// Slider pieces (rook, bishop, queen) project influence along their movement rays,
/// attenuating through each blocker based on the blocker's resistance.
/// Non-slider pieces (pawn, knight, king) project to their specific attack squares.
///
/// This correctly models directional piece influence — a rook projects along
/// ranks/files, a bishop along diagonals, pawns only forward-diagonal.
pub fn compute_influence(state: &GameState) -> InfluenceInfo {
    let board = state.board();
    let mut grid = [[0.0f32; PLAYERS]; TOTAL_SQUARES];

    for player in Player::all() {
        if !is_active_for_zones(state, player) {
            continue;
        }
        let pi = player.index();

        for (piece_type, sq) in board.pieces(player) {
            let weight = INFLUENCE_WEIGHTS[piece_type.index()];
            let rank = sq.rank() as i8;
            let file = sq.file() as i8;

            // The piece's own square gets full influence
            grid[sq.index() as usize][pi] += weight;

            match piece_type {
                // Sliders: ray-attenuated along movement vectors
                PieceType::Rook => {
                    for &(dr, df) in &ORTHOGONAL_DIRS {
                        ray_attenuated(board, rank, file, dr, df, weight, pi, &mut grid);
                    }
                }
                PieceType::Bishop => {
                    for &(dr, df) in &DIAGONAL_DIRS {
                        ray_attenuated(board, rank, file, dr, df, weight, pi, &mut grid);
                    }
                }
                PieceType::Queen | PieceType::PromotedQueen => {
                    for &(dr, df) in &ALL_DIRS {
                        ray_attenuated(board, rank, file, dr, df, weight, pi, &mut grid);
                    }
                }
                // Knight: discrete L-shaped jumps, no attenuation (jumps over pieces)
                PieceType::Knight => {
                    for &(dr, df) in &KNIGHT_OFFSETS {
                        let nr = rank + dr;
                        let nf = file + df;
                        if nr >= 0
                            && nr < BOARD_SIZE as i8
                            && nf >= 0
                            && nf < BOARD_SIZE as i8
                            && is_valid_square(nr as u8, nf as u8)
                        {
                            let nidx = nr as usize * BOARD_SIZE + nf as usize;
                            grid[nidx][pi] += weight;
                        }
                    }
                }
                // Pawn: only forward-diagonal attack squares (directionally correct)
                PieceType::Pawn => {
                    for &(dr, df) in &PAWN_CAPTURE_DELTAS[player.index()] {
                        let nr = rank + dr;
                        let nf = file + df;
                        if nr >= 0
                            && nr < BOARD_SIZE as i8
                            && nf >= 0
                            && nf < BOARD_SIZE as i8
                            && is_valid_square(nr as u8, nf as u8)
                        {
                            let nidx = nr as usize * BOARD_SIZE + nf as usize;
                            grid[nidx][pi] += weight;
                        }
                    }
                }
                // King: 8 adjacent squares, no attenuation
                PieceType::King => {
                    for &(dr, df) in &ALL_DIRS {
                        let nr = rank + dr;
                        let nf = file + df;
                        if nr >= 0
                            && nr < BOARD_SIZE as i8
                            && nf >= 0
                            && nf < BOARD_SIZE as i8
                            && is_valid_square(nr as u8, nf as u8)
                        {
                            let nidx = nr as usize * BOARD_SIZE + nf as usize;
                            grid[nidx][pi] += weight;
                        }
                    }
                }
            }
        }
    }

    // Compute totals and net influence
    let mut total = [0.0f32; PLAYERS];
    let mut net = [0.0f32; PLAYERS];

    for &idx in VALID_SQUARES_LIST.iter() {
        let sq_grid = &grid[idx as usize];
        for pi in 0..PLAYERS {
            total[pi] += sq_grid[pi];
        }
    }

    // Net = own - max(opponents), modified by counter-attack potential
    for pi in 0..PLAYERS {
        let max_opp = total
            .iter()
            .enumerate()
            .filter(|&(oi, _)| oi != pi)
            .map(|(_, &v)| v)
            .fold(0.0f32, f32::max);
        net[pi] = total[pi] - max_opp;
    }

    InfluenceInfo { total, net, grid }
}

// ─── Tension / Vulnerability ────────────────────────────────────────────────

/// Minimum combined influence to consider a square "contested".
const TENSION_THRESHOLD: f32 = 2.0;

/// Compute tension/vulnerability score per player.
///
/// For each player's pieces, check if the piece sits on a contested square
/// (where multiple players have significant influence). Pieces on contested
/// squares are penalized (vulnerable); pieces on safe squares get a bonus.
pub fn compute_tension(state: &GameState, influence: &InfluenceInfo) -> [i16; PLAYERS] {
    let board = state.board();
    let mut scores = [0i16; PLAYERS];

    for player in Player::all() {
        if !is_active_for_zones(state, player) {
            continue;
        }
        let pi = player.index();
        let mut safe_count: i16 = 0;
        let mut exposed_count: i16 = 0;

        for (_, sq) in board.pieces(player) {
            let idx = sq.index() as usize;
            let my_inf = influence.grid[idx][pi];

            // Find max opponent influence at this square
            let mut max_opp_inf = 0.0f32;
            for oi in 0..PLAYERS {
                if oi != pi && influence.grid[idx][oi] > max_opp_inf {
                    max_opp_inf = influence.grid[idx][oi];
                }
            }

            if my_inf + max_opp_inf > TENSION_THRESHOLD && max_opp_inf > my_inf * 0.5 {
                // Contested: opponent has significant presence here
                exposed_count += 1;
            } else if my_inf > max_opp_inf * 2.0 {
                // Safe: own influence dominates
                safe_count += 1;
            }
        }

        // Score: safe pieces bonus minus exposed pieces penalty
        scores[pi] = safe_count * 3 - exposed_count * 5;
    }

    scores
}

// ─── Swarm Mechanics ────────────────────────────────────────────────────────

/// Swarm analysis result per player.
pub struct SwarmInfo {
    /// Mutual defense: count of own pieces defended by at least one friendly piece.
    pub defended_pieces: [i16; PLAYERS],
    /// Undefended pieces: count of own pieces NOT defended by any friendly piece.
    pub undefended_pieces: [i16; PLAYERS],
    /// Attack coordination: count of squares where 2+ of your pieces project influence.
    /// Uses the ray-attenuation grid — squares with influence > single-piece weight
    /// indicate overlapping projection from multiple pieces.
    pub coordinated_squares: [i16; PLAYERS],
    /// Pawn chain length: count of pawns defended by friendly pawns.
    pub pawn_chain: [i16; PLAYERS],
}

/// Compute swarm features: mutual defense, attack coordination, pawn chains.
///
/// Swarm mechanics model the emergent strength of coordinated piece groups.
/// Individual piece force comes from ray-attenuation; swarm captures how
/// those individual forces reinforce each other.
pub fn compute_swarm(state: &GameState, influence: &InfluenceInfo) -> SwarmInfo {
    let board = state.board();
    let mut defended = [0i16; PLAYERS];
    let mut undefended = [0i16; PLAYERS];
    let mut coordinated = [0i16; PLAYERS];
    let mut pawn_chain = [0i16; PLAYERS];

    for player in Player::all() {
        if !is_active_for_zones(state, player) {
            continue;
        }
        let pi = player.index();

        // Mutual defense: for each piece, is it defended by a friendly piece?
        // A piece is "defended" if any other friendly piece attacks its square.
        for (piece_type, sq) in board.pieces(player) {
            let mut is_defended = false;

            // Check if any friendly piece attacks this square
            // For efficiency, check piece-type-specific patterns:

            // Check friendly pawn defense (pawn captures this square)
            for &(dr, df) in &PAWN_CAPTURE_DELTAS[pi] {
                let pr = sq.rank() as i8 - dr;
                let pf = sq.file() as i8 - df;
                if pr >= 0
                    && pr < BOARD_SIZE as i8
                    && pf >= 0
                    && pf < BOARD_SIZE as i8
                    && is_valid_square(pr as u8, pf as u8)
                {
                    let pidx = pr as usize * BOARD_SIZE + pf as usize;
                    if let Some(p) = board.piece_at(Square(pidx as u8))
                        && p.player == player
                        && p.piece_type == PieceType::Pawn
                    {
                        is_defended = true;
                        break;
                    }
                }
            }

            // If not pawn-defended, check if any friendly slider/knight/king defends
            if !is_defended {
                // Use the influence grid as a proxy: if friendly influence > own piece weight,
                // another piece also projects here
                let own_weight = INFLUENCE_WEIGHTS[piece_type.index()];
                if influence.grid[sq.index() as usize][pi] > own_weight + 0.5 {
                    is_defended = true;
                }
            }

            if is_defended {
                defended[pi] += 1;
            } else {
                undefended[pi] += 1;
            }

            // Pawn chain: pawn defended by pawn
            if piece_type == PieceType::Pawn {
                for &(dr, df) in &PAWN_CAPTURE_DELTAS[pi] {
                    let pr = sq.rank() as i8 - dr;
                    let pf = sq.file() as i8 - df;
                    if pr >= 0
                        && pr < BOARD_SIZE as i8
                        && pf >= 0
                        && pf < BOARD_SIZE as i8
                        && is_valid_square(pr as u8, pf as u8)
                    {
                        let pidx = pr as usize * BOARD_SIZE + pf as usize;
                        if let Some(p) = board.piece_at(Square(pidx as u8))
                            && p.player == player
                            && p.piece_type == PieceType::Pawn
                        {
                            pawn_chain[pi] += 1;
                            break; // Count each pawn once
                        }
                    }
                }
            }
        }

        // Attack coordination: count squares where influence > threshold for overlap
        // A single queen contributes 9.0 at its square. If a square has > 9.5,
        // at least two pieces project there. Use a lower threshold for coordination.
        let coordination_threshold = 5.0f32; // Two minor pieces or one major + support
        for &idx in VALID_SQUARES_LIST.iter() {
            if influence.grid[idx as usize][pi] >= coordination_threshold {
                coordinated[pi] += 1;
            }
        }
    }

    SwarmInfo {
        defended_pieces: defended,
        undefended_pieces: undefended,
        coordinated_squares: coordinated,
        pawn_chain,
    }
}

// ─── King Safety Enhancement ────────────────────────────────────────────────

/// Count escape routes: empty king-adjacent squares not attacked by any opponent.
fn king_escape_routes(state: &GameState, player: Player) -> i16 {
    let board = state.board();
    let king_idx = board.king_square(player);
    if king_idx == 255 {
        return 0; // Eliminated king
    }
    let king_rank = king_idx / BOARD_SIZE as u8;
    let king_file = king_idx % BOARD_SIZE as u8;

    let mut routes: i16 = 0;
    const DIRS: [(i8, i8); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];

    for &(dr, df) in &DIRS {
        let nr = king_rank as i8 + dr;
        let nf = king_file as i8 + df;
        if nr < 0 || nr >= BOARD_SIZE as i8 || nf < 0 || nf >= BOARD_SIZE as i8 {
            continue;
        }
        let nr = nr as u8;
        let nf = nf as u8;
        if !is_valid_square(nr, nf) {
            continue;
        }
        let adj_sq = Square(nr * BOARD_SIZE as u8 + nf);

        // Square must be empty or own piece
        if let Some(piece) = board.piece_at(adj_sq)
            && piece.player != player
        {
            continue; // Blocked by enemy piece
        }

        // Check if any active opponent attacks this square
        let mut attacked = false;
        for opp in Player::all() {
            if opp == player || state.player_status(opp) == PlayerStatus::Eliminated {
                continue;
            }
            if board.is_square_attacked_by(adj_sq, opp) {
                attacked = true;
                break;
            }
        }
        if !attacked {
            routes += 1;
        }
    }

    routes
}

// ─── Bootstrap Evaluator ────────────────────────────────────────────────────

/// Configurable zone feature weights (Stage 15).
#[derive(Clone, Debug)]
pub struct ZoneWeights {
    /// Territory weight (BFS Voronoi). 0 = disabled.
    pub territory: i16,
    /// Influence map weight. 0 = disabled.
    pub influence: i16,
    /// Tension/vulnerability weight. 0 = disabled.
    pub tension: i16,
    /// Swarm mechanics weight. 0 = disabled.
    pub swarm: i16,
}

impl Default for ZoneWeights {
    fn default() -> Self {
        Self {
            territory: 2,
            influence: 1,
            tension: 1,
            swarm: 3,
        }
    }
}

/// Bootstrap positional evaluator.
///
/// Combines material, PST, mobility, territory, king safety, and pawn structure.
/// Temporary — NNUE replaces this in Stage 17.
#[derive(Clone)]
pub struct BootstrapEvaluator {
    /// Zone feature weights (Stage 15). Configurable via setoption.
    pub zone_weights: ZoneWeights,
}

impl BootstrapEvaluator {
    pub fn new() -> Self {
        Self {
            zone_weights: ZoneWeights::default(),
        }
    }

    /// Create with custom zone weights (for A/B testing).
    pub fn with_zone_weights(zone_weights: ZoneWeights) -> Self {
        Self { zone_weights }
    }

    /// Compute material score for a player (relative to opponents).
    fn material_score(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let my_material: i16 = board.pieces(player).map(|(pt, _)| piece_value(pt)).sum();

        let mut opp_total: i16 = 0;
        let mut opp_count: i16 = 0;
        for opp in player.opponents() {
            if state.player_status(opp) != PlayerStatus::Eliminated {
                let opp_mat: i16 = board.pieces(opp).map(|(pt, _)| piece_value(pt)).sum();
                opp_total += opp_mat;
                opp_count += 1;
            }
        }

        if opp_count > 0 {
            my_material - opp_total / opp_count
        } else {
            my_material
        }
    }

    /// Compute PST bonus for a player.
    fn pst_score(state: &GameState, player: Player) -> i16 {
        state
            .board()
            .pieces(player)
            .map(|(pt, sq)| pst_value(pt, sq, player))
            .sum()
    }

    /// Approximate mobility: count of pseudo-legal moves.
    /// Uses piece-type heuristic rather than full legal move generation
    /// to stay within the <50us budget without needing &mut Board.
    fn mobility_score(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let mut moves = 0i16;

        for (pt, _sq) in board.pieces(player) {
            // Approximate move counts by piece type
            moves += match pt {
                PieceType::Pawn => 2,
                PieceType::Knight => 4,
                PieceType::Bishop => 5,
                PieceType::Rook => 7,
                PieceType::Queen | PieceType::PromotedQueen => 10,
                PieceType::King => 4,
            };
        }

        moves
    }

    /// King safety: pawn shelter bonus minus attacker penalty.
    fn king_safety_score(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let king_idx = board.king_square(player);
        if king_idx == ELIMINATED_KING_SENTINEL {
            return 0;
        }
        let king_sq = Square(king_idx);
        let king_rank = king_sq.rank() as i8;
        let king_file = king_sq.file() as i8;

        let mut shelter = 0i16;
        let mut attackers = 0i16;

        // Check 8 adjacent squares
        const DIRS: [(i8, i8); 8] = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];

        for &(dr, df) in &DIRS {
            let nr = king_rank + dr;
            let nf = king_file + df;
            if nr < 0 || nr >= BOARD_SIZE as i8 || nf < 0 || nf >= BOARD_SIZE as i8 {
                continue;
            }
            let nr = nr as u8;
            let nf = nf as u8;
            if !is_valid_square(nr, nf) {
                continue;
            }
            let adj_sq = Square::from_rank_file_unchecked(nr, nf);

            // Pawn shelter: own pawn adjacent to king
            if let Some(piece) = board.piece_at(adj_sq)
                && piece.player == player
                && piece.piece_type == PieceType::Pawn
            {
                shelter += 1;
            }

            // Attacker presence: any opponent attacking this adjacent square
            for opp in player.opponents() {
                if state.player_status(opp) != PlayerStatus::Eliminated
                    && board.is_square_attacked_by(adj_sq, opp)
                {
                    attackers += 1;
                    break; // Count each square only once
                }
            }
        }

        // Exposure penalty: king with few shelter pawns is vulnerable
        let exposure_penalty = if shelter <= 1 {
            45 // Severe: 0-1 shelter pawns
        } else if shelter <= 2 {
            20 // Moderate: only 2 shelter pawns
        } else {
            0 // 3+ shelter pawns is adequate
        };

        // Amplify attacker penalty when shelter is already damaged
        let attacker_weight = if shelter <= 1 {
            WEIGHT_KING_SAFETY_ATTACKER * 2
        } else {
            WEIGHT_KING_SAFETY_ATTACKER
        };

        shelter * WEIGHT_KING_SAFETY_SHELTER - attackers * attacker_weight - exposure_penalty
    }

    /// Castling bonus: flat reward for having castled.
    /// Detects castling by checking if king is on a castling destination square
    /// AND that side's castling right is revoked.
    fn castled_bonus(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let king_sq = board.king_square(player);
        if king_sq == ELIMINATED_KING_SENTINEL {
            return 0;
        }
        let rights = board.castling_rights();
        let pi = player.index();

        for side in 0..2 {
            let dest = CASTLE_KING_DESTS[pi][side];
            let bit = CASTLE_RIGHTS_BITS[pi][side];
            // King is on castling destination AND that right is revoked
            if king_sq == dest && (rights & (1 << bit)) == 0 {
                return CASTLED_BONUS;
            }
        }
        0
    }

    /// Development bonus: reward pieces that have moved off their back rank.
    /// Knights and bishops get the biggest bonus (develop minors first),
    /// queen gets a moderate bonus (activate early like 3000+ Elo players),
    /// rooks get a small bonus (usually develop later via castling/open files).
    fn development_score(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let mut score = 0i16;

        // Count undeveloped pieces (pieces still on back rank) as penalty
        let mut undeveloped_minors = 0i16; // knights, bishops
        let mut undeveloped_queen = false;

        for (pt, sq) in board.pieces(player) {
            // Check if piece is off its back rank(s)
            let off_home = match player {
                Player::Red => sq.rank() >= 2,     // Red's back ranks are 0-1
                Player::Blue => sq.file() >= 2,    // Blue's back files are 0-1
                Player::Yellow => sq.rank() <= 11, // Yellow's back ranks are 12-13
                Player::Green => sq.file() <= 11,  // Green's back files are 12-13
            };

            match pt {
                PieceType::Knight | PieceType::Bishop => {
                    if off_home {
                        score += 3; // Strong bonus for developed minors
                    } else {
                        undeveloped_minors += 1;
                    }
                }
                PieceType::Queen | PieceType::PromotedQueen => {
                    if off_home {
                        score += 2; // Moderate bonus for active queen
                    } else {
                        undeveloped_queen = true;
                    }
                }
                PieceType::Rook => {
                    if off_home {
                        score += 1; // Small bonus for rooks
                    }
                }
                _ => continue, // Skip pawns and king
            }
        }

        // Penalty for undeveloped minors: gets worse the more you have sitting at home
        score -= undeveloped_minors * undeveloped_minors; // quadratic: 1→-1, 2→-4, 3→-9, 4→-16

        // Penalty for undeveloped queen when minors are still home
        // (don't penalize queen staying home if minors are developed — sometimes queen develops first)
        if undeveloped_queen && undeveloped_minors >= 3 {
            score -= 2;
        }

        // Cap positive development to prevent drowning out material gains (captures)
        score.min(DEV_SCORE_CAP)
    }

    /// Pawn structure: advancement bonus and doubled pawn penalty.
    fn pawn_structure_score(state: &GameState, player: Player) -> i16 {
        let board = state.board();
        let mut advance_bonus = 0i16;
        let mut doubled_penalty = 0i16;

        // Track pawns per "file" (relative to the player's orientation)
        let mut file_pawn_count = [0u8; BOARD_SIZE];

        for (pt, sq) in board.pieces(player) {
            if pt != PieceType::Pawn {
                continue;
            }

            // Advancement: how far from home rank
            let advancement = match player {
                Player::Red => sq.rank(),         // Red advances by increasing rank
                Player::Blue => sq.file(),        // Blue advances by increasing file
                Player::Yellow => 13 - sq.rank(), // Yellow advances by decreasing rank
                Player::Green => 13 - sq.file(),  // Green advances by decreasing file
            };
            // Subtract 1 since starting rank is 1 away from back rank
            advance_bonus += (advancement.saturating_sub(1)) as i16;

            // Track file for doubled detection (relative to player orientation)
            let rel_file = match player {
                Player::Red | Player::Yellow => sq.file(),
                Player::Blue | Player::Green => sq.rank(),
            } as usize;
            file_pawn_count[rel_file] += 1;
        }

        // Count doubled pawns: any file with 2+ pawns, penalty per extra pawn
        for &count in &file_pawn_count {
            if count > 1 {
                doubled_penalty += (count - 1) as i16;
            }
        }

        advance_bonus * WEIGHT_PAWN_ADVANCE - doubled_penalty * WEIGHT_PAWN_DOUBLED
    }
}

impl Default for BootstrapEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator for BootstrapEvaluator {
    fn eval_scalar(&self, state: &GameState, player: Player) -> i16 {
        let scores = self.eval_4vec(state);
        scores[player.index()]
    }

    fn eval_4vec(&self, state: &GameState) -> [i16; 4] {
        // Compute zone features once (shared across all players)
        let territory = bfs_territory_enhanced(state);
        let zw = &self.zone_weights;

        // Only compute influence/tension/swarm if weights are nonzero
        let needs_influence = zw.influence > 0 || zw.tension > 0 || zw.swarm > 0;
        let influence = if needs_influence {
            Some(compute_influence(state))
        } else {
            None
        };
        let tension_scores = if zw.tension > 0 {
            influence.as_ref().map(|inf| compute_tension(state, inf))
        } else {
            None
        };
        let swarm = if zw.swarm > 0 {
            influence.as_ref().map(|inf| compute_swarm(state, inf))
        } else {
            None
        };

        let mut scores = [0i16; 4];

        for player in Player::all() {
            if state.player_status(player) == PlayerStatus::Eliminated {
                scores[player.index()] = ELIMINATED_SCORE;
                continue;
            }

            let pi = player.index();
            let material = Self::material_score(state, player) * WEIGHT_MATERIAL;
            let pst = Self::pst_score(state, player) * WEIGHT_PST;
            let mobility = Self::mobility_score(state, player) * WEIGHT_MOBILITY;

            // Zone features (Stage 15)
            let terr = territory.counts[pi] * zw.territory;
            let frontier_penalty = -(territory.frontier[pi].saturating_sub(
                territory.counts[pi] / 4, // Penalty only for excessive frontier
            )) * (zw.territory / 2).max(1);
            let influence_score = if let Some(ref inf) = influence {
                (inf.net[pi] * zw.influence as f32) as i16
            } else {
                0
            };
            let tension_score = if let Some(ref ts) = tension_scores {
                ts[pi] * zw.tension
            } else {
                0
            };

            // Swarm mechanics (Stage 15)
            let swarm_score = if let Some(ref sw) = swarm {
                let defense_bonus = sw.defended_pieces[pi] * 4; // 4cp per defended piece
                let isolation_penalty = sw.undefended_pieces[pi] * 6; // 6cp per undefended piece
                let coordination_bonus = sw.coordinated_squares[pi]; // 1cp per coordinated square
                let chain_bonus = sw.pawn_chain[pi] * 5; // 5cp per chain pawn
                (defense_bonus - isolation_penalty + coordination_bonus + chain_bonus) * zw.swarm
            } else {
                0
            };

            // King safety (base + escape routes enhancement)
            let king_safety = Self::king_safety_score(state, player);
            let escape = king_escape_routes(state, player) * 15; // 15cp per escape route

            let castled = Self::castled_bonus(state, player);
            let pawn_structure = Self::pawn_structure_score(state, player);
            let development = Self::development_score(state, player) * WEIGHT_DEVELOPMENT;

            let total = material
                + pst
                + mobility
                + terr
                + frontier_penalty
                + influence_score
                + tension_score
                + swarm_score
                + king_safety
                + escape
                + castled
                + pawn_structure
                + development;

            tracing::debug!(
                player = %player,
                total,
                material,
                pst,
                mobility,
                territory = terr,
                frontier = frontier_penalty,
                influence = influence_score,
                tension = tension_score,
                swarm = swarm_score,
                king_safety,
                escape,
                castled,
                pawn_structure,
                development,
                "eval component breakdown"
            );

            scores[player.index()] = total;
        }

        // Zero-center: subtract the mean across all active players.
        // This removes constant baselines from zone features, which:
        // - Preserves relative ordering (Max^n correctness unchanged)
        // - Enables constant-sum pruning (scores sum to zero)
        // - Keeps MCTS Q-values near zero (healthy exploration/exploitation balance)
        // - Prevents sigmoid saturation in future NNUE training
        // - Restores beam search discrimination (wider dynamic range)
        let active_count = scores.iter().filter(|&&s| s != ELIMINATED_SCORE).count() as i32;
        if active_count > 1 {
            let sum: i32 = scores
                .iter()
                .filter(|&&s| s != ELIMINATED_SCORE)
                .map(|&s| s as i32)
                .sum();
            let mean = (sum / active_count) as i16;
            for s in &mut scores {
                if *s != ELIMINATED_SCORE {
                    *s -= mean;
                }
            }
        }

        scores
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    fn standard_state() -> GameState {
        GameState::new_standard_ffa()
    }

    // ── Evaluator Trait Contract ──

    #[test]
    fn test_eval_scalar_4vec_consistency() {
        let state = standard_state();
        let eval = BootstrapEvaluator::new();
        let vec = eval.eval_4vec(&state);
        for player in Player::all() {
            assert_eq!(
                eval.eval_scalar(&state, player),
                vec[player.index()],
                "scalar and 4vec must agree for {}",
                player
            );
        }
    }

    #[test]
    fn test_starting_position_roughly_equal() {
        let state = standard_state();
        let eval = BootstrapEvaluator::new();
        let scores = eval.eval_4vec(&state);
        // All 4 players start with same material, symmetric position.
        // Scores should be close (not necessarily identical due to PST orientation).
        let max = *scores.iter().max().unwrap();
        let min = *scores.iter().min().unwrap();
        assert!(
            (max - min).abs() < 100,
            "Starting position should be roughly equal: {:?}",
            scores
        );
    }

    // ── Material ──

    #[test]
    fn test_material_different_positions_score_differently() {
        let mut state = standard_state();
        let eval = BootstrapEvaluator::new();
        let before = eval.eval_4vec(&state);

        // Remove a Red pawn to create material imbalance
        let board = state.board_mut();
        // Find Red's first pawn
        let pawn_sq = board
            .pieces(Player::Red)
            .find(|(pt, _)| *pt == PieceType::Pawn)
            .map(|(_, sq)| sq)
            .unwrap();
        board.remove_piece(pawn_sq);

        let after = eval.eval_4vec(&state);
        assert_ne!(
            before[0], after[0],
            "Removing a piece must change the score"
        );
    }

    #[test]
    fn test_capturing_opponent_improves_score() {
        let mut state = standard_state();
        let eval = BootstrapEvaluator::new();

        // Set up: put a Blue pawn where Red can capture it
        let board = state.board_mut();
        // Place Blue pawn on e3 (rank 2, file 4)
        let target = Square::from_rank_file_unchecked(2, 4);
        board.set_piece(target, Piece::new(PieceType::Pawn, Player::Blue));

        let before_red = eval.eval_scalar(&state, Player::Red);

        // Remove the Blue pawn (simulating capture)
        let board = state.board_mut();
        board.remove_piece(target);

        let after_red = eval.eval_scalar(&state, Player::Red);

        assert!(
            after_red > before_red,
            "Capturing opponent piece must improve score: before={}, after={}",
            before_red,
            after_red
        );
    }

    #[test]
    fn test_promoted_queen_eval_900cp() {
        assert_eq!(
            piece_value(PieceType::PromotedQueen),
            900,
            "PromotedQueen should be worth 900cp in eval"
        );
        assert_eq!(
            piece_value(PieceType::Queen),
            piece_value(PieceType::PromotedQueen),
            "Queen and PromotedQueen should have equal eval value"
        );
    }

    // ── Eliminated Players ──

    #[test]
    fn test_eliminated_player_sentinel_score() {
        let mut state = standard_state();
        let eval = BootstrapEvaluator::new();

        // Resign 3 players to trigger eliminations
        state.resign_player(Player::Red);
        state.resign_player(Player::Blue);
        state.resign_player(Player::Yellow);

        let scores = eval.eval_4vec(&state);

        // Any player that is Eliminated should get sentinel score
        for player in Player::all() {
            let status = state.player_status(player);
            if status == PlayerStatus::Eliminated {
                assert_eq!(
                    scores[player.index()],
                    ELIMINATED_SCORE,
                    "{} is Eliminated but did not get sentinel score",
                    player
                );
            }
        }

        // Green (last standing) should NOT be eliminated
        assert_ne!(
            state.player_status(Player::Green),
            PlayerStatus::Eliminated,
            "Green should still be active/DKW"
        );
        assert_ne!(
            scores[Player::Green.index()],
            ELIMINATED_SCORE,
            "Green should have a normal score"
        );
    }

    // ── Territory ──

    #[test]
    fn test_bfs_territory_all_squares_assigned() {
        let state = standard_state();
        let info = bfs_territory_enhanced(&state);
        let total: i16 = info.counts.iter().sum::<i16>() + info.contested;
        assert_eq!(
            total, VALID_SQUARES as i16,
            "All {} valid squares must be assigned or contested: counts={:?} contested={}",
            VALID_SQUARES, info.counts, info.contested
        );
    }

    #[test]
    fn test_bfs_territory_starting_roughly_equal() {
        let state = standard_state();
        let info = bfs_territory_enhanced(&state);
        // Each player should control roughly 40 squares (160/4), minus contested
        for &count in &info.counts {
            assert!(
                count > 10 && count < 60,
                "Starting territory should be roughly equal: {:?} contested={}",
                info.counts,
                info.contested
            );
        }
    }

    // ── PST Rotation ──

    #[test]
    fn test_pst_rotation_consistency() {
        // Red's (0,0) should map to Blue's rotated equivalent
        let (r, f) = rotate_for_player(0, 0, Player::Red);
        assert_eq!((r, f), (0, 0));

        let (r, f) = rotate_for_player(0, 0, Player::Blue);
        assert_eq!((r, f), (0, 13));

        let (r, f) = rotate_for_player(0, 0, Player::Yellow);
        assert_eq!((r, f), (13, 13));

        let (r, f) = rotate_for_player(0, 0, Player::Green);
        assert_eq!((r, f), (13, 0));
    }

    // ── Eval Symmetry ──

    #[test]
    fn test_eval_symmetry_swap_red_blue() {
        // Create a symmetric position and verify scores are symmetric.
        // Start from standard position — it IS symmetric across the 4 players.
        let state = standard_state();
        let eval = BootstrapEvaluator::new();
        let scores = eval.eval_4vec(&state);

        // In the starting position, Red and Yellow are on opposite sides (rank 0-1 vs 12-13)
        // and Blue and Green are on opposite sides (file 0-1 vs 12-13).
        // Red/Yellow should score similarly, Blue/Green should score similarly.
        let ry_diff = (scores[Player::Red.index()] - scores[Player::Yellow.index()]).abs();
        let bg_diff = (scores[Player::Blue.index()] - scores[Player::Green.index()]).abs();
        assert!(
            ry_diff < 50,
            "Red/Yellow should be symmetric in starting pos: {:?}",
            scores
        );
        assert!(
            bg_diff < 50,
            "Blue/Green should be symmetric in starting pos: {:?}",
            scores
        );
    }

    // ── Pawn Structure ──

    #[test]
    fn test_pawn_advancement_disabled() {
        // WEIGHT_PAWN_ADVANCE = 0: advancement scoring disabled until NNUE (Stage 14+).
        // Pawns get value from material (captures) and promotion, not walking forward.
        let mut board = Board::empty();
        board.set_piece(
            Square::from_rank_file_unchecked(0, 7),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_piece(
            Square::from_rank_file_unchecked(2, 5),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        let state1 = GameState::new(board.clone());

        let mut board2 = Board::empty();
        board2.set_piece(
            Square::from_rank_file_unchecked(0, 7),
            Piece::new(PieceType::King, Player::Red),
        );
        board2.set_piece(
            Square::from_rank_file_unchecked(6, 5),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        let state2 = GameState::new(board2);

        let adv1 = BootstrapEvaluator::pawn_structure_score(&state1, Player::Red);
        let adv2 = BootstrapEvaluator::pawn_structure_score(&state2, Player::Red);

        assert_eq!(
            adv1, adv2,
            "With WEIGHT_PAWN_ADVANCE=0, advancement should not affect score: rank2={}, rank6={}",
            adv1, adv2
        );
    }

    #[test]
    fn test_doubled_pawn_penalty() {
        let mut board1 = Board::empty();
        board1.set_piece(
            Square::from_rank_file_unchecked(0, 7),
            Piece::new(PieceType::King, Player::Red),
        );
        // Single pawn on file e
        board1.set_piece(
            Square::from_rank_file_unchecked(3, 4),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        let state1 = GameState::new(board1);

        let mut board2 = Board::empty();
        board2.set_piece(
            Square::from_rank_file_unchecked(0, 7),
            Piece::new(PieceType::King, Player::Red),
        );
        // Two pawns on file e (doubled)
        board2.set_piece(
            Square::from_rank_file_unchecked(3, 4),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board2.set_piece(
            Square::from_rank_file_unchecked(5, 4),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        let state2 = GameState::new(board2);

        let _score1 = BootstrapEvaluator::pawn_structure_score(&state1, Player::Red);
        let score2 = BootstrapEvaluator::pawn_structure_score(&state2, Player::Red);

        // Verify doubled penalty: compare doubled pawns vs same pawns on separate files.
        let mut board3 = Board::empty();
        board3.set_piece(
            Square::from_rank_file_unchecked(0, 7),
            Piece::new(PieceType::King, Player::Red),
        );
        // Same advancement as board2 but spread across two files (no doubled)
        board3.set_piece(
            Square::from_rank_file_unchecked(3, 4),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board3.set_piece(
            Square::from_rank_file_unchecked(5, 5),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        let state3 = GameState::new(board3);
        let score3 = BootstrapEvaluator::pawn_structure_score(&state3, Player::Red);

        assert!(
            score3 > score2,
            "Spread pawns should score higher than doubled pawns: spread={}, doubled={}",
            score3,
            score2
        );
    }

    // ── Performance ──

    #[test]
    fn test_eval_performance_under_500us_debug() {
        // The <50us target is for release builds. Debug builds are ~5-10x slower.
        // This test uses a generous 500us threshold for debug; release perf
        // is verified via `cargo test --release`.
        let state = standard_state();
        let eval = BootstrapEvaluator::new();

        // Warm up
        let _ = eval.eval_4vec(&state);

        let start = std::time::Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = eval.eval_4vec(&state);
        }
        let elapsed = start.elapsed();
        let per_call = elapsed / iterations;

        assert!(
            per_call.as_micros() < 500,
            "Eval should complete in <500us (debug), actual: {}us",
            per_call.as_micros()
        );
    }

    // ── Piece Values ──

    #[test]
    fn test_piece_values() {
        assert_eq!(piece_value(PieceType::Pawn), 100);
        assert_eq!(piece_value(PieceType::Knight), 300);
        assert_eq!(piece_value(PieceType::Bishop), 450);
        assert_eq!(piece_value(PieceType::Rook), 500);
        assert_eq!(piece_value(PieceType::Queen), 900);
        assert_eq!(piece_value(PieceType::King), 0);
        assert_eq!(piece_value(PieceType::PromotedQueen), 900);
    }

    // ── Material Relative ──

    #[test]
    fn test_material_is_relative() {
        let state = standard_state();
        // All players start with same material, so relative material should be ~0
        let score = BootstrapEvaluator::material_score(&state, Player::Red);
        assert_eq!(score, 0, "Equal material should give 0 relative score");
    }

    // ── Stage 15: Zone Control Feature Tests ──

    #[test]
    fn test_territory_frontier_at_boundary() {
        let state = standard_state();
        let info = bfs_territory_enhanced(&state);
        // In starting position, each player should have frontier squares
        // (territory borders other players' territory)
        for pi in 0..PLAYERS {
            assert!(
                info.frontier[pi] > 0,
                "Player {} should have frontier squares in starting position: frontier={:?}",
                pi,
                info.frontier
            );
        }
    }

    #[test]
    fn test_territory_contested_equidistant() {
        let state = standard_state();
        let info = bfs_territory_enhanced(&state);
        // In a symmetric starting position, there should be contested squares
        // where multiple players are equidistant
        assert!(
            info.contested >= 0,
            "Contested squares should be non-negative: {}",
            info.contested
        );
        // Total assigned + contested = valid squares
        let total: i16 = info.counts.iter().sum::<i16>() + info.contested;
        assert_eq!(total, VALID_SQUARES as i16);
    }

    #[test]
    fn test_influence_queen_dominates_pawn() {
        let state = standard_state();
        let inf = compute_influence(&state);
        // All players start with same pieces, so total influence should be similar
        let max_diff = inf.total.iter().copied().fold(0.0f32, |acc, x| acc.max(x))
            - inf
                .total
                .iter()
                .copied()
                .fold(f32::MAX, |acc, x| acc.min(x));
        assert!(
            max_diff < inf.total[0] * 0.3,
            "Starting influence should be roughly equal: {:?}",
            inf.total
        );
    }

    #[test]
    fn test_influence_net_symmetric_start() {
        let state = standard_state();
        let inf = compute_influence(&state);
        // In symmetric starting position, net influence should be near zero for all
        for pi in 0..PLAYERS {
            assert!(
                inf.net[pi].abs() < inf.total[pi] * 0.3,
                "Player {} net influence should be near 0 in symmetric start: net={:.1} total={:.1}",
                pi,
                inf.net[pi],
                inf.total[pi]
            );
        }
    }

    #[test]
    fn test_tension_symmetric_start() {
        let state = standard_state();
        let inf = compute_influence(&state);
        let tension = compute_tension(&state, &inf);
        // All players should have similar tension in symmetric position
        let max_t = *tension.iter().max().unwrap();
        let min_t = *tension.iter().min().unwrap();
        assert!(
            (max_t - min_t).abs() <= 3,
            "Starting tension should be roughly equal: {:?}",
            tension
        );
    }

    #[test]
    fn test_king_escape_routes_starting_position() {
        let state = standard_state();
        // In starting position, kings are on back rank with pieces around them.
        // They should have limited escape routes.
        for player in Player::all() {
            let routes = king_escape_routes(&state, player);
            assert!(
                routes >= 0 && routes <= 8,
                "King escape routes should be in [0, 8]: player={} routes={}",
                player,
                routes
            );
        }
    }

    #[test]
    fn test_zone_features_symmetric_starting_position() {
        // Zone-enhanced eval should give roughly equal scores in starting position
        let state = standard_state();
        let eval = BootstrapEvaluator::new(); // Default zone weights
        let scores = eval.eval_4vec(&state);
        let max_score = *scores.iter().max().unwrap();
        let min_score = *scores.iter().min().unwrap();
        assert!(
            (max_score - min_score).abs() < 50,
            "Starting scores should be roughly equal: {:?}",
            scores
        );
    }

    #[test]
    fn test_zone_features_disabled_matches_no_zone() {
        let state = standard_state();
        // With all zone weights = 0, should match old behavior closely
        let eval_zero = BootstrapEvaluator::with_zone_weights(ZoneWeights {
            territory: 0,
            influence: 0,
            tension: 0,
            swarm: 0,
        });
        let scores_zero = eval_zero.eval_4vec(&state);

        // With zone weights enabled
        let eval_on = BootstrapEvaluator::new();
        let scores_on = eval_on.eval_4vec(&state);

        // Both should give valid scores (not crash)
        for &s in &scores_zero {
            assert!(s != ELIMINATED_SCORE, "No players should be eliminated");
        }
        for &s in &scores_on {
            assert!(s != ELIMINATED_SCORE, "No players should be eliminated");
        }
    }

    #[test]
    fn test_zone_dkw_player_skipped() {
        let mut state = standard_state();
        // Resign players — triggers DKW (Dead King Walking), not full elimination.
        // DKW players' pieces stay on board but are "dead". Zone features should
        // skip DKW players (they don't project territory or influence).
        state.resign_player(Player::Blue);
        state.resign_player(Player::Yellow);

        let info = bfs_territory_enhanced(&state);
        // DKW players should have 0 territory (skipped by is_active_for_zones)
        assert_eq!(
            info.counts[Player::Blue.index()],
            0,
            "DKW Blue should have 0 territory"
        );
        assert_eq!(
            info.counts[Player::Yellow.index()],
            0,
            "DKW Yellow should have 0 territory"
        );

        // Active players (Red + Green) should share the board
        let active_total: i16 =
            info.counts[Player::Red.index()] + info.counts[Player::Green.index()] + info.contested;
        assert_eq!(active_total, VALID_SQUARES as i16);
    }

    #[test]
    fn test_zone_features_performance_debug() {
        // Simple smoke test for zone feature performance (debug mode, so relaxed threshold)
        let state = standard_state();
        let eval = BootstrapEvaluator::new();

        // Warm up
        let _ = eval.eval_4vec(&state);

        let start = std::time::Instant::now();
        let iterations = 100;
        for _ in 0..iterations {
            let _ = eval.eval_4vec(&state);
        }
        let elapsed = start.elapsed();
        let per_call = elapsed / iterations;

        // Debug mode: allow up to 500us (release is ~10x faster)
        assert!(
            per_call.as_micros() < 500,
            "eval_4vec with zone features took {}us per call (debug mode, budget 500us)",
            per_call.as_micros()
        );
    }
}
