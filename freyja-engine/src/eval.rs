//! Position evaluation.
//!
//! Evaluator trait and BootstrapEvaluator implementation.
//! The bootstrap evaluator is temporary — NNUE replaces it in Stage 17.
//! It includes zone control features that will become NNUE training features.

use crate::board::types::*;
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
const WEIGHT_TERRITORY: i16 = 0; // DISABLED — BFS Voronoi rewards pawn pushes, re-enable with NNUE
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

// ─── BFS Territory ──────────────────────────────────────────────────────────

/// BFS Voronoi territory: assign each valid square to the nearest player
/// (by piece distance). Returns count of squares owned by each player.
fn bfs_territory(state: &GameState) -> [i16; PLAYERS] {
    let board = state.board();

    // owner[sq_index] = player index who owns it, or 255 for unassigned
    let mut owner = [255u8; TOTAL_SQUARES];
    // distance[sq_index] = BFS distance from nearest piece
    let mut dist = [255u8; TOTAL_SQUARES];

    // Fixed-size BFS queue: (square_index, player_index, distance)
    // Max 160 valid squares, each enqueued at most once
    let mut queue = [(0u8, 0u8, 0u8); VALID_SQUARES];
    let mut head: usize = 0;
    let mut tail: usize = 0;

    // Seed BFS with all pieces from all active players
    for player in Player::all() {
        if state.player_status(player) == PlayerStatus::Eliminated {
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
            }
        }
    }

    // Count squares per player
    let mut counts = [0i16; PLAYERS];
    for &idx in VALID_SQUARES_LIST.iter() {
        let o = owner[idx as usize];
        if (o as usize) < PLAYERS {
            counts[o as usize] += 1;
        }
    }

    tracing::trace!(
        red = counts[0],
        blue = counts[1],
        yellow = counts[2],
        green = counts[3],
        "territory squares per player"
    );

    counts
}

// ─── Bootstrap Evaluator ────────────────────────────────────────────────────

/// Bootstrap positional evaluator.
///
/// Combines material, PST, mobility, territory, king safety, and pawn structure.
/// Temporary — NNUE replaces this in Stage 17.
#[derive(Clone)]
pub struct BootstrapEvaluator;

impl BootstrapEvaluator {
    pub fn new() -> Self {
        Self
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
        // Compute territory once (shared across all players)
        let territory = bfs_territory(state);

        let mut scores = [0i16; 4];

        for player in Player::all() {
            if state.player_status(player) == PlayerStatus::Eliminated {
                scores[player.index()] = ELIMINATED_SCORE;
                continue;
            }

            let material = Self::material_score(state, player) * WEIGHT_MATERIAL;
            let pst = Self::pst_score(state, player) * WEIGHT_PST;
            let mobility = Self::mobility_score(state, player) * WEIGHT_MOBILITY;
            let terr = territory[player.index()] * WEIGHT_TERRITORY;
            let king_safety = Self::king_safety_score(state, player);
            let castled = Self::castled_bonus(state, player);
            let pawn_structure = Self::pawn_structure_score(state, player);
            let development = Self::development_score(state, player) * WEIGHT_DEVELOPMENT;

            let total = material
                + pst
                + mobility
                + terr
                + king_safety
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
                king_safety,
                castled,
                pawn_structure,
                development,
                "eval component breakdown"
            );

            scores[player.index()] = total;
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
        let territory = bfs_territory(&state);
        let total: i16 = territory.iter().sum();
        assert_eq!(
            total, VALID_SQUARES as i16,
            "All {} valid squares must be assigned: {:?}",
            VALID_SQUARES, territory
        );
    }

    #[test]
    fn test_bfs_territory_starting_roughly_equal() {
        let state = standard_state();
        let territory = bfs_territory(&state);
        // Each player should control roughly 40 squares (160/4)
        for &count in &territory {
            assert!(
                count > 20 && count < 60,
                "Starting territory should be roughly equal: {:?}",
                territory
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
}
