//! Search algorithms: Max^n with beam search, negamax fallback.
//!
//! Searcher trait and MaxnSearcher implementation.
//! The Searcher trait is the stable boundary between protocol and search.

use std::time::Instant;

use arrayvec::ArrayVec;

use crate::board::Board;
use crate::board::types::*;
use crate::eval::{ELIMINATED_SCORE, Evaluator, piece_value};
use crate::game_state::GameState;
use crate::move_gen::{MAX_MOVES, Move, generate_legal_moves, make_move, unmake_move};

// ─── Constants ─────────────────────────────────────────────────────────────

/// Maximum search depth (ply).
pub const MAX_DEPTH: usize = 32;

/// Default beam width. 30 is effectively "all moves" for 4PC (~80 legal avg).
pub const DEFAULT_BEAM_WIDTH: usize = 30;

/// Number of candidates evaluated with full eval in hybrid beam ordering.
/// Must be >= beam_width for beam selection to work correctly.
pub const BEAM_CANDIDATES: usize = 15;

/// Default sum bound for Korf shallow pruning.
/// Conservative: 4 players * 10000cp max eval estimate.
pub const DEFAULT_SUM_BOUND: i32 = 40_000;

// ─── Score Type ────────────────────────────────────────────────────────────

/// Score vector for 4-player search: [Red, Blue, Yellow, Green] in centipawns.
pub type Score4 = [i16; 4];

/// Minimum score vector (used for initialization).
pub const SCORE4_MIN: Score4 = [i16::MIN; 4];

// ─── Search Limits ─────────────────────────────────────────────────────────

/// Time, depth, and node constraints for a search.
#[derive(Debug, Clone, Default)]
pub struct SearchLimits {
    /// Maximum search depth (ply).
    pub max_depth: Option<u32>,
    /// Maximum nodes to search.
    pub max_nodes: Option<u64>,
    /// Maximum time in milliseconds.
    pub max_time_ms: Option<u64>,
    /// Search until explicitly stopped.
    pub infinite: bool,
}

// ─── Search Result ─────────────────────────────────────────────────────────

/// Result returned by a search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Best move found, or None if no legal moves.
    pub best_move: Option<Move>,
    /// 4-vector scores at the root position.
    pub scores: Score4,
    /// Depth of the deepest completed iteration.
    pub depth: u32,
    /// Total nodes searched.
    pub nodes: u64,
    /// Principal variation (best line of play).
    pub pv: ArrayVec<Move, MAX_DEPTH>,
}

// ─── Search Configuration ──────────────────────────────────────────────────

/// Tunable search parameters.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Beam width: how many moves to expand per node.
    pub beam_width: usize,
    /// Sum bound for Korf shallow pruning.
    pub sum_bound: i32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            beam_width: DEFAULT_BEAM_WIDTH,
            sum_bound: DEFAULT_SUM_BOUND,
        }
    }
}

// ─── Searcher Trait ────────────────────────────────────────────────────────

/// Search interface. Stable boundary between protocol and search implementations.
///
/// All search algorithms (MaxnSearcher, future MctsSearcher, hybrid) implement
/// this trait. The protocol layer calls only `search()` — never search internals.
pub trait Searcher {
    /// Search the position and return the best move with evaluation.
    ///
    /// Takes `&mut GameState` because move generation requires `&mut Board`.
    /// The state is returned to its original position when search completes.
    fn search(&mut self, state: &mut GameState, limits: &SearchLimits) -> SearchResult;
}

// ─── MaxnSearcher ──────────────────────────────────────────────────────────

/// Max^n search with NNUE-guided beam search.
///
/// Each player maximizes their own component of the 4-vector score.
/// Beam search restricts expansion to top K moves ranked by evaluation.
/// Falls back to negamax with alpha-beta when only 2 players remain.
pub struct MaxnSearcher<E: Evaluator> {
    evaluator: E,
    config: SearchConfig,
}

impl<E: Evaluator> MaxnSearcher<E> {
    /// Create a new MaxnSearcher with the given evaluator and config.
    pub fn new(evaluator: E, config: SearchConfig) -> Self {
        Self { evaluator, config }
    }
}

// ─── Internal Search State ─────────────────────────────────────────────────

/// Mutable state passed through recursion.
///
/// Kept separate from MaxnSearcher to avoid &self vs &mut conflict.
struct SearchState {
    /// Total nodes visited.
    nodes: u64,
    /// Search start time.
    start_time: Instant,
    /// Limits for this search.
    limits: SearchLimits,
    /// Set to true when time/node limit hit.
    aborted: bool,
    /// Triangular PV table. pv_table[ply] holds the PV from that ply onward.
    pv_table: [[Option<Move>; MAX_DEPTH]; MAX_DEPTH],
    /// Length of PV at each ply.
    pv_length: [usize; MAX_DEPTH],
}

impl SearchState {
    fn new(limits: SearchLimits) -> Self {
        Self {
            nodes: 0,
            start_time: Instant::now(),
            limits,
            aborted: false,
            pv_table: [[None; MAX_DEPTH]; MAX_DEPTH],
            pv_length: [0; MAX_DEPTH],
        }
    }

    /// Check if the search should abort due to time or node limits.
    #[inline]
    fn should_abort(&self) -> bool {
        if self.aborted {
            return true;
        }
        if let Some(max_nodes) = self.limits.max_nodes
            && self.nodes >= max_nodes
        {
            return true;
        }
        if let Some(max_time_ms) = self.limits.max_time_ms {
            // Only check time every 1024 nodes to reduce overhead
            if self.nodes & 1023 == 0 && self.start_time.elapsed().as_millis() as u64 >= max_time_ms
            {
                return true;
            }
        }
        false
    }
}

// ─── Score Utilities ───────────────────────────────────────────────────────

/// Check if `new_scores` is better than `old_scores` for the given player.
#[inline]
pub fn score4_is_better(new_scores: &Score4, old_scores: &Score4, player_idx: usize) -> bool {
    new_scores[player_idx] > old_scores[player_idx]
}

/// Sum of all components in a score vector (as i32 to avoid overflow).
#[inline]
pub fn score4_sum(s: &Score4) -> i32 {
    s[0] as i32 + s[1] as i32 + s[2] as i32 + s[3] as i32
}

/// Check if a player is eliminated during search.
///
/// During search, `make_move` removes captured kings from the board but does NOT
/// update `king_squares` to `ELIMINATED_KING_SENTINEL`. So we check two conditions:
/// 1. `king_square == ELIMINATED_KING_SENTINEL` (pre-search elimination via GameState)
/// 2. The piece at `king_square` is not this player's king (mid-search king capture)
#[inline]
pub fn is_player_eliminated_in_search(board: &Board, player: Player) -> bool {
    let ks = board.king_square(player);
    if ks == ELIMINATED_KING_SENTINEL {
        return true;
    }
    // Mid-search check: king was captured but king_squares not updated
    match board.piece_at(Square(ks)) {
        Some(piece) => piece.piece_type != PieceType::King || piece.player != player,
        None => true, // Square is empty — king was captured
    }
}

/// Count active (non-eliminated) players during search.
#[inline]
pub fn active_count_in_search(board: &Board) -> usize {
    let mut count = 0;
    for p in Player::all() {
        if !is_player_eliminated_in_search(board, p) {
            count += 1;
        }
    }
    count
}

// ─── Max^n Implementation ──────────────────────────────────────────────────

impl<E: Evaluator> MaxnSearcher<E> {
    /// Evaluate position with search-aware elimination detection.
    ///
    /// Wraps `evaluator.eval_4vec()` but substitutes `ELIMINATED_SCORE` for
    /// players whose kings were captured mid-search (which the evaluator can't
    /// detect because GameState.player_status is stale during search).
    fn eval_4vec_search(&self, state: &GameState) -> Score4 {
        let mut scores = self.evaluator.eval_4vec(state);
        let board = state.board();
        for p in Player::all() {
            if is_player_eliminated_in_search(board, p) {
                scores[p.index()] = ELIMINATED_SCORE;
            }
        }
        scores
    }

    /// Hybrid beam selection: MVV-LVA pre-filter then eval_scalar on top candidates.
    ///
    /// 1. Generate all legal moves
    /// 2. Score cheaply with MVV-LVA (capture value + promotion bonus)
    /// 3. Pre-sort, take top BEAM_CANDIDATES
    /// 4. For those candidates: make_move, eval_scalar, unmake_move
    /// 5. Re-sort by eval, take top beam_width
    fn beam_select(
        &self,
        state: &mut GameState,
        player: Player,
        beam_width: usize,
    ) -> ArrayVec<Move, MAX_MOVES> {
        let board = state.board_mut();
        let all_moves = generate_legal_moves(board);

        if all_moves.is_empty() {
            return ArrayVec::new();
        }

        let effective_beam = beam_width.min(all_moves.len()).max(1);

        // If beam covers all moves, skip sorting
        if effective_beam >= all_moves.len() {
            return all_moves;
        }

        // Step 1: Cheap MVV-LVA scoring
        let mut scored: ArrayVec<(Move, i32), MAX_MOVES> = all_moves
            .iter()
            .map(|&mv| {
                let mut score: i32 = 0;
                if let Some(captured) = mv.captured() {
                    score += piece_value(captured) as i32 * 100;
                }
                if mv.promotion().is_some() {
                    score += 900; // Queen promotion bonus
                }
                (mv, score)
            })
            .collect();

        // Sort descending by cheap score
        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        // Step 2: Take top BEAM_CANDIDATES for eval-based re-ranking
        let candidates_count = BEAM_CANDIDATES.max(effective_beam).min(scored.len());

        // Step 3: Evaluate candidates with eval_scalar (make/eval/unmake)
        let mut eval_scored: ArrayVec<(Move, i16), MAX_MOVES> = ArrayVec::new();
        for i in 0..candidates_count {
            let mv = scored[i].0;
            let undo = make_move(state.board_mut(), mv);
            let eval = self.evaluator.eval_scalar(state, player);
            unmake_move(state.board_mut(), &undo);
            eval_scored.push((mv, eval));
        }

        // Sort candidates by eval descending
        eval_scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        // Take top beam_width
        eval_scored
            .iter()
            .take(effective_beam)
            .map(|&(mv, _)| mv)
            .collect()
    }

    /// Update the triangular PV table at the given ply.
    fn update_pv(ss: &mut SearchState, ply: usize, mv: Move) {
        ss.pv_table[ply][0] = Some(mv);
        let child_len = if ply + 1 < MAX_DEPTH {
            ss.pv_length[ply + 1]
        } else {
            0
        };
        for i in 0..child_len {
            ss.pv_table[ply][i + 1] = ss.pv_table[ply + 1][i];
        }
        ss.pv_length[ply] = child_len + 1;
    }

    /// Extract PV from the table at ply 0.
    fn extract_pv(ss: &SearchState) -> ArrayVec<Move, MAX_DEPTH> {
        let mut pv = ArrayVec::new();
        for i in 0..ss.pv_length[0] {
            if let Some(mv) = ss.pv_table[0][i] {
                pv.push(mv);
            } else {
                break;
            }
        }
        pv
    }

    /// Core Max^n recursive search.
    ///
    /// Returns the 4-vector score for the subtree rooted at this node.
    /// Each player maximizes their own component.
    fn maxn(&self, state: &mut GameState, depth: u32, ply: usize, ss: &mut SearchState) -> Score4 {
        ss.nodes += 1;

        // Abort check
        if ss.should_abort() {
            ss.aborted = true;
            return SCORE4_MIN;
        }

        let board = state.board();
        let current = board.side_to_move();
        let current_idx = current.index();

        // Eliminated player skip — do NOT decrement depth
        if is_player_eliminated_in_search(board, current) {
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let result = self.maxn(state, depth, ply, ss);
            state.board_mut().set_side_to_move(current); // restore
            return result;
        }

        // Terminal: game over (all opponents eliminated)
        if active_count_in_search(state.board()) <= 1 {
            ss.pv_length[ply] = 0;
            return self.eval_4vec_search(state);
        }

        // Leaf node: depth exhausted
        if depth == 0 {
            ss.pv_length[ply] = 0;
            return self.eval_4vec_search(state);
        }

        // Check for negamax activation (2 active players)
        if active_count_in_search(state.board()) == 2 {
            return self.negamax_2p_entry(state, depth, ply, ss);
        }

        // Generate moves with beam selection
        let moves = self.beam_select(state, current, self.config.beam_width);

        // No legal moves (stalemate/checkmate)
        if moves.is_empty() {
            ss.pv_length[ply] = 0;
            return self.eval_4vec_search(state);
        }

        // Max^n expansion
        let mut best_scores = SCORE4_MIN;
        let mut best_move = moves[0];

        for mv in &moves {
            let board = state.board_mut();
            let undo = make_move(board, *mv);
            let child_scores = self.maxn(state, depth - 1, ply + 1, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return SCORE4_MIN;
            }

            if child_scores[current_idx] > best_scores[current_idx] {
                best_scores = child_scores;
                best_move = *mv;
                Self::update_pv(ss, ply, *mv);
            }

            // Korf shallow pruning: if current player can't improve, prune
            if self.config.sum_bound > 0 {
                let sum_others = score4_sum(&best_scores) as i64 - best_scores[current_idx] as i64;
                if self.config.sum_bound as i64 - sum_others <= best_scores[current_idx] as i64 {
                    break;
                }
            }
        }

        let _ = best_move; // used via PV
        best_scores
    }

    /// Entry point for negamax when exactly 2 players remain.
    ///
    /// Identifies the two active players, runs alpha-beta negamax,
    /// and maps the result back to a Score4.
    fn negamax_2p_entry(
        &self,
        state: &mut GameState,
        depth: u32,
        ply: usize,
        ss: &mut SearchState,
    ) -> Score4 {
        let board = state.board();
        let mut active = ArrayVec::<Player, 4>::new();
        for p in Player::all() {
            if !is_player_eliminated_in_search(board, p) {
                active.push(p);
            }
        }
        debug_assert_eq!(active.len(), 2);

        let current = board.side_to_move();
        let maximizer = active[0];
        let minimizer = active[1];
        // Make sure we know which is which relative to current player
        let (max_p, min_p) = if current == maximizer {
            (maximizer, minimizer)
        } else {
            (minimizer, maximizer)
        };

        let score = self.negamax_2p(state, depth, ply, i16::MIN + 1, i16::MAX, ss);

        if ss.aborted {
            return SCORE4_MIN;
        }

        // Map scalar score back to Score4
        let mut result = [ELIMINATED_SCORE; 4];
        result[max_p.index()] = score;
        result[min_p.index()] = -score;
        result
    }

    /// Negamax with alpha-beta for 2-player endgame.
    ///
    /// Returns score from the perspective of the current side to move.
    fn negamax_2p(
        &self,
        state: &mut GameState,
        depth: u32,
        ply: usize,
        mut alpha: i16,
        beta: i16,
        ss: &mut SearchState,
    ) -> i16 {
        ss.nodes += 1;

        if ss.should_abort() {
            ss.aborted = true;
            return 0;
        }

        let board = state.board();
        let current = board.side_to_move();

        // Skip eliminated players (shouldn't happen in 2p but be safe)
        if is_player_eliminated_in_search(board, current) {
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let score = -self.negamax_2p(state, depth, ply, -beta, -alpha, ss);
            state.board_mut().set_side_to_move(current);
            return score;
        }

        // Leaf node
        if depth == 0 {
            ss.pv_length[ply] = 0;
            return self.evaluator.eval_scalar(state, current);
        }

        let moves = generate_legal_moves(state.board_mut());

        if moves.is_empty() {
            ss.pv_length[ply] = 0;
            return self.evaluator.eval_scalar(state, current);
        }

        for mv in &moves {
            let undo = make_move(state.board_mut(), *mv);
            let score = -self.negamax_2p(state, depth - 1, ply + 1, -beta, -alpha, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return 0;
            }

            if score > alpha {
                alpha = score;
                Self::update_pv(ss, ply, *mv);
            }
            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        alpha
    }

    /// Iterative deepening entry point.
    fn iterative_deepening(
        &mut self,
        state: &mut GameState,
        limits: &SearchLimits,
    ) -> SearchResult {
        let mut ss = SearchState::new(limits.clone());
        let max_depth = limits.max_depth.unwrap_or(MAX_DEPTH as u32);

        let mut best_result = SearchResult {
            best_move: None,
            scores: SCORE4_MIN,
            depth: 0,
            nodes: 0,
            pv: ArrayVec::new(),
        };

        for depth in 1..=max_depth {
            // Reset PV for this iteration
            ss.pv_length = [0; MAX_DEPTH];

            let scores = self.maxn(state, depth, 0, &mut ss);

            if ss.aborted {
                break; // Use result from last completed depth
            }

            // Extract best move from PV
            let pv = Self::extract_pv(&ss);
            let best_move = pv.first().copied();

            best_result = SearchResult {
                best_move,
                scores,
                depth,
                nodes: ss.nodes,
                pv,
            };

            tracing::info!(
                depth = depth,
                nodes = ss.nodes,
                best_move = ?best_result.best_move,
                scores = ?scores,
                "Search depth complete"
            );

            // Check if we should stop before the next iteration
            if ss.should_abort() {
                break;
            }
        }

        best_result
    }
}

impl<E: Evaluator> Searcher for MaxnSearcher<E> {
    fn search(&mut self, state: &mut GameState, limits: &SearchLimits) -> SearchResult {
        self.iterative_deepening(state, limits)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::BootstrapEvaluator;

    fn make_searcher() -> MaxnSearcher<BootstrapEvaluator> {
        MaxnSearcher::new(BootstrapEvaluator::new(), SearchConfig::default())
    }

    fn make_searcher_with_beam(beam: usize) -> MaxnSearcher<BootstrapEvaluator> {
        MaxnSearcher::new(
            BootstrapEvaluator::new(),
            SearchConfig {
                beam_width: beam,
                ..SearchConfig::default()
            },
        )
    }

    // ── Step 1: Types compile ──

    #[test]
    fn test_search_types_exist() {
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let config = SearchConfig::default();
        assert_eq!(config.beam_width, DEFAULT_BEAM_WIDTH);
        assert_eq!(limits.max_depth, Some(1));
    }

    // ── Step 2: Score utilities ──

    #[test]
    fn test_score4_is_better() {
        let a: Score4 = [100, 200, 300, 400];
        let b: Score4 = [150, 180, 300, 400];
        assert!(score4_is_better(&b, &a, 0)); // Red: 150 > 100
        assert!(!score4_is_better(&b, &a, 1)); // Blue: 180 < 200
    }

    #[test]
    fn test_score4_sum() {
        let s: Score4 = [100, 200, -50, 300];
        assert_eq!(score4_sum(&s), 550);
    }

    #[test]
    fn test_score4_sum_with_eliminated() {
        let s: Score4 = [100, ELIMINATED_SCORE, 200, 300];
        // i16::MIN = -32768, sum = 100 + (-32768) + 200 + 300 = -32168
        assert_eq!(score4_sum(&s), 100 + ELIMINATED_SCORE as i32 + 200 + 300);
    }

    #[test]
    fn test_elimination_detection_starting_position() {
        let gs = GameState::new_standard_ffa();
        let board = gs.board();
        for p in Player::all() {
            assert!(
                !is_player_eliminated_in_search(board, p),
                "{:?} should not be eliminated at start",
                p
            );
        }
        assert_eq!(active_count_in_search(board), 4);
    }

    #[test]
    fn test_elimination_detection_sentinel() {
        let board = Board::empty();
        // Empty board: all king_squares are ELIMINATED_KING_SENTINEL
        for p in Player::all() {
            assert!(is_player_eliminated_in_search(&board, p));
        }
        assert_eq!(active_count_in_search(&board), 0);
    }

    // ── Step 3: Basic Max^n ──

    #[test]
    fn test_depth_1_returns_legal_move() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some(), "Depth 1 must return a move");
        assert_eq!(result.depth, 1);
        assert!(result.nodes > 0);
    }

    #[test]
    fn test_depth_2_returns_legal_move() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some());
        assert_eq!(result.depth, 2);
    }

    #[test]
    fn test_search_preserves_game_state() {
        let mut gs = GameState::new_standard_ffa();
        let board_before = gs.board().clone();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let _ = searcher.search(&mut gs, &limits);
        // Board must be unchanged after search
        assert_eq!(
            gs.board().side_to_move(),
            board_before.side_to_move(),
            "side_to_move must be restored"
        );
        assert_eq!(
            gs.board().zobrist_hash(),
            board_before.zobrist_hash(),
            "zobrist must be restored"
        );
    }

    // ── Step 4: Beam search ──

    #[test]
    fn test_beam_width_gt_moves_no_panic() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher_with_beam(999);
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_beam_width_1_returns_move() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher_with_beam(1);
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some());
    }

    // ── Step 5: Iterative deepening ──

    #[test]
    fn test_iterative_deepening_depth_3() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(3),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert_eq!(result.depth, 3);
        assert!(result.nodes > 0);
    }

    #[test]
    fn test_node_limit() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_nodes: Some(500),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        // Node count should be in the ballpark (may slightly exceed due to check granularity)
        assert!(
            result.nodes < 2000,
            "Node count {} should be bounded near 500",
            result.nodes
        );
    }

    // ── Step 8: PV tracking ──

    #[test]
    fn test_pv_not_empty() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(!result.pv.is_empty(), "PV should not be empty");
        assert_eq!(
            result.pv[0],
            result.best_move.unwrap(),
            "PV[0] should be best_move"
        );
    }

    #[test]
    fn test_pv_moves_are_legal() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);

        // Verify each PV move is legal by playing them
        let board = gs.board_mut();
        let mut undos = Vec::new();
        for mv in &result.pv {
            let legal = generate_legal_moves(board);
            assert!(
                legal.contains(mv),
                "PV move {} is not legal in position",
                mv
            );
            let undo = make_move(board, *mv);
            undos.push((*mv, undo));
        }
        // Restore
        for (_mv, undo) in undos.iter().rev() {
            unmake_move(board, undo);
        }
    }
}
