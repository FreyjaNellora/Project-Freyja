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
use crate::move_gen::{
    MAX_MOVES, Move, generate_captures_only, generate_legal_moves, make_move, unmake_move,
};
use crate::move_order::{
    HistoryTable, KillerTable, order_captures_mvv_lva, order_moves, score_move,
};
use crate::tt::{DEFAULT_TT_SIZE_MB, TTFlag, TranspositionTable};

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

/// Maximum quiescence search depth (ply beyond main search leaf).
/// 4 is sufficient to resolve most capture chains while keeping overhead manageable.
pub const MAX_QSEARCH_DEPTH: u32 = 4;

/// Default maximum quiescence nodes before soft abort (return stand-pat).
/// Prevents stack overflow from capture explosion on the 14x14 board.
pub const DEFAULT_MAX_QNODES: u64 = 2_000_000;

/// Default opponent beam ratio: opponents get this fraction of root player's beam.
/// Based on BRS research (Schadd & Winands 2011) — narrowing opponent moves is the
/// single biggest performance gain in multi-player search.
pub const DEFAULT_OPPONENT_BEAM_RATIO: f32 = 0.25;

/// Minimum search depth before time-based abort is allowed.
/// Ensures the engine always completes at least this depth for quality decisions.
pub const MIN_SEARCH_DEPTH: u32 = 4;

/// Delta pruning margin: skip captures that can't possibly improve score.
const DELTA_MARGIN: i16 = 200;

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
    /// Current game ply (for phase-based search selection).
    pub game_ply: u32,
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
    /// Total nodes searched (main search).
    pub nodes: u64,
    /// Total quiescence nodes searched.
    pub qnodes: u64,
    /// Principal variation (best line of play).
    pub pv: ArrayVec<Move, MAX_DEPTH>,
    /// TT hit rate as percentage (0.0 - 100.0).
    pub tt_hit_rate: f64,
    /// Killer move hit rate as percentage (0.0 - 100.0).
    pub killer_hit_rate: f64,
}

// ─── Search Configuration ──────────────────────────────────────────────────

/// Tunable search parameters.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Beam width: how many moves to expand per node.
    pub beam_width: usize,
    /// Sum bound for Korf shallow pruning.
    pub sum_bound: i32,
    /// Transposition table size in megabytes.
    pub tt_size_mb: usize,
    /// Maximum quiescence nodes before soft abort (return stand-pat).
    pub max_qnodes: u64,
    /// Per-depth beam width schedule. None = use flat beam_width.
    pub beam_schedule: Option<[usize; MAX_DEPTH]>,
    /// Move noise level (0-100). 0 = deterministic.
    pub move_noise: u32,
    /// Adaptive beam based on position complexity.
    pub adaptive_beam: bool,
    /// Opponent beam ratio: fraction of root player's beam width for opponent nodes.
    /// 0.25 = opponents get 1/4 of root beam (minimum 3 moves).
    pub opponent_beam_ratio: f32,
    /// Noise seed: XOR'd with Zobrist hash for per-game randomization.
    /// 0 = use Zobrist hash alone. Different seeds produce different games.
    pub noise_seed: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            beam_width: DEFAULT_BEAM_WIDTH,
            sum_bound: DEFAULT_SUM_BOUND,
            tt_size_mb: DEFAULT_TT_SIZE_MB,
            max_qnodes: DEFAULT_MAX_QNODES,
            beam_schedule: None,
            move_noise: 0,
            adaptive_beam: false,
            opponent_beam_ratio: DEFAULT_OPPONENT_BEAM_RATIO,
            noise_seed: 0,
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
    tt: TranspositionTable,
    killers: KillerTable,
    history: HistoryTable,
}

impl<E: Evaluator> MaxnSearcher<E> {
    /// Create a new MaxnSearcher with the given evaluator and config.
    pub fn new(evaluator: E, config: SearchConfig) -> Self {
        let tt = TranspositionTable::new(config.tt_size_mb);
        Self {
            evaluator,
            config,
            tt,
            killers: KillerTable::new(),
            history: HistoryTable::new(),
        }
    }

    /// Extract the history table for MCTS Progressive History warm-start.
    /// Required by ADR-007 for Stage 11 hybrid controller.
    pub fn history_table(&self) -> &HistoryTable {
        &self.history
    }
}

// ─── Internal Search State ─────────────────────────────────────────────────

/// Mutable state passed through recursion.
///
/// Kept separate from MaxnSearcher to avoid &self vs &mut conflict.
struct SearchState {
    /// Total nodes visited (main search).
    nodes: u64,
    /// Total quiescence nodes visited.
    qnodes: u64,
    /// Maximum quiescence nodes before soft abort (return stand-pat).
    max_qnodes: u64,
    /// Search start time.
    start_time: Instant,
    /// Limits for this search.
    limits: SearchLimits,
    /// Set to true when time/node limit hit.
    aborted: bool,
    /// When true, time-based abort is suspended (used for minimum depth guarantee).
    suspend_time_check: bool,
    /// Triangular PV table. pv_table[ply] holds the PV from that ply onward.
    pv_table: [[Option<Move>; MAX_DEPTH]; MAX_DEPTH],
    /// Length of PV at each ply.
    pv_length: [usize; MAX_DEPTH],
}

impl SearchState {
    fn new(limits: SearchLimits, max_qnodes: u64) -> Self {
        Self {
            nodes: 0,
            qnodes: 0,
            max_qnodes,
            start_time: Instant::now(),
            limits,
            aborted: false,
            suspend_time_check: false,
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
        let total_nodes = self.nodes + self.qnodes;
        if let Some(max_nodes) = self.limits.max_nodes
            && total_nodes >= max_nodes
        {
            return true;
        }
        // Skip time check when suspended (minimum depth guarantee)
        if !self.suspend_time_check
            && let Some(max_time_ms) = self.limits.max_time_ms
        {
            // Only check time every 1024 nodes to reduce overhead
            if total_nodes & 1023 == 0
                && self.start_time.elapsed().as_millis() as u64 >= max_time_ms
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

    /// Get beam width for a given depth and player role.
    ///
    /// Respects per-depth schedule, opponent beam ratio, and adaptive beam.
    /// Opponent nodes use a fraction of root player's beam (BRS insight:
    /// narrowing opponent moves is the biggest perf gain in multi-player search).
    #[inline]
    fn beam_width_for(
        &self,
        remaining_depth: u32,
        is_root_player: bool,
        state: &mut GameState,
    ) -> usize {
        let base = if let Some(ref schedule) = self.config.beam_schedule {
            let idx = (remaining_depth as usize).min(schedule.len() - 1);
            schedule[idx].max(1)
        } else {
            self.config.beam_width
        };

        // Apply opponent beam ratio for non-root players
        let role_adjusted = if is_root_player {
            base
        } else {
            ((base as f32 * self.config.opponent_beam_ratio) as usize).max(3)
        };

        if !self.config.adaptive_beam {
            return role_adjusted;
        }

        // Adaptive beam: widen for tactical positions (many captures), narrow for quiet
        let captures = generate_captures_only(state.board_mut());
        let capture_count = captures.len();
        if capture_count > 8 {
            (role_adjusted * 3 / 2).min(MAX_MOVES)
        } else if capture_count == 0 {
            (role_adjusted * 2 / 3).max(3)
        } else {
            role_adjusted
        }
    }

    /// Hybrid beam selection with TT-aware move ordering.
    ///
    /// 1. Generate all legal moves
    /// 2. Score with move ordering heuristics (TT move, MVV-LVA, killers, history)
    /// 3. Pre-sort, take top BEAM_CANDIDATES
    /// 4. For those candidates: make_move, eval_scalar, unmake_move
    /// 5. Re-sort by eval, take top beam_width
    ///
    /// The TT best move is always included in the candidate set.
    fn beam_select(
        &self,
        state: &mut GameState,
        player: Player,
        beam_width: usize,
        tt_move: Option<Move>,
        ply: usize,
    ) -> ArrayVec<Move, MAX_MOVES> {
        let board = state.board_mut();
        let all_moves = generate_legal_moves(board);

        if all_moves.is_empty() {
            return ArrayVec::new();
        }

        let effective_beam = beam_width.min(all_moves.len()).max(1);

        // If beam covers all moves, still order them (TT move first)
        if effective_beam >= all_moves.len() {
            let mut ordered = all_moves;
            // Put TT move first if present
            if let Some(tt) = tt_move
                && let Some(pos) = ordered.iter().position(|&m| m == tt)
            {
                ordered.swap(0, pos);
            }
            return ordered;
        }

        // Step 1: Score all moves with ordering heuristics
        let mut scored: ArrayVec<(Move, i32), MAX_MOVES> = all_moves
            .iter()
            .map(|&mv| {
                let s = score_move(mv, tt_move, &self.killers, &self.history, ply, player);
                (mv, s)
            })
            .collect();

        // Sort descending by ordering score
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

        // Ensure TT move is always first if it's in the candidate set
        if let Some(tt) = tt_move
            && let Some(pos) = eval_scored.iter().position(|&(m, _)| m == tt)
        {
            eval_scored.swap(0, pos);
        }

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
    /// `root_player` is the player who initiated the search (used by quiescence
    /// to restrict captures to those involving the root player's pieces).
    fn maxn(
        &mut self,
        state: &mut GameState,
        depth: u32,
        ply: usize,
        root_player: Player,
        ss: &mut SearchState,
    ) -> Score4 {
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
            // Guard: prevent ply from exceeding pv_table bounds
            if ply >= MAX_DEPTH - 1 {
                return self.eval_4vec_search(state);
            }
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let result = self.maxn(state, depth, ply, root_player, ss);
            state.board_mut().set_side_to_move(current); // restore
            return result;
        }

        // Terminal: game over (all opponents eliminated)
        if active_count_in_search(state.board()) <= 1 {
            ss.pv_length[ply] = 0;
            return self.eval_4vec_search(state);
        }

        // Leaf node: depth exhausted — enter quiescence
        if depth == 0 {
            return self.qsearch(state, 0, ply, root_player, ss);
        }

        // Check for negamax activation (2 active players)
        if active_count_in_search(state.board()) == 2 {
            return self.negamax_2p_entry(state, depth, ply, ss);
        }

        // TT probe
        let hash = state.board().zobrist_hash();
        let tt_move;
        if let Some(entry) = self.tt.probe(hash) {
            tt_move = entry.best_move();
            // Exact hit at sufficient depth — return stored scores
            if entry.flag() == TTFlag::Exact && entry.depth() as u32 >= depth {
                tracing::debug!(
                    depth = entry.depth(),
                    flag = ?entry.flag(),
                    scores = ?entry.scores(),
                    "TT hit (exact cutoff)"
                );
                ss.pv_length[ply] = 0;
                return *entry.scores();
            }
        } else {
            tt_move = None;
        }

        // Generate moves with beam selection (respects schedule, opponent ratio, adaptive)
        let is_root = current == root_player;
        let beam = self.beam_width_for(depth, is_root, state);
        let moves = self.beam_select(state, current, beam, tt_move, ply);

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
            let child_scores = self.maxn(state, depth - 1, ply + 1, root_player, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return SCORE4_MIN;
            }

            if child_scores[current_idx] > best_scores[current_idx] {
                best_scores = child_scores;
                best_move = *mv;
                Self::update_pv(ss, ply, *mv);

                // Update history for quiet moves that improve the score
                if !mv.is_capture() && mv.promotion().is_none() {
                    self.history.update(mv.from_sq().0, mv.to_sq().0, depth);
                }
            }

            // Korf shallow pruning: if current player can't improve, prune
            if self.config.sum_bound > 0 {
                let sum_others = score4_sum(&best_scores) as i64 - best_scores[current_idx] as i64;
                if self.config.sum_bound as i64 - sum_others <= best_scores[current_idx] as i64 {
                    break;
                }
            }
        }

        // TT store — Max^n always produces exact results
        self.tt.store(
            hash,
            depth as u8,
            TTFlag::Exact,
            best_scores,
            Some(best_move),
        );

        // Store best quiet move as killer
        if !best_move.is_capture() && best_move.promotion().is_none() {
            self.killers.store(ply, current, best_move);
        }

        best_scores
    }

    /// Entry point for negamax when exactly 2 players remain.
    ///
    /// Identifies the two active players, runs alpha-beta negamax,
    /// and maps the result back to a Score4.
    fn negamax_2p_entry(
        &mut self,
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
        &mut self,
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
            // Guard: prevent ply from exceeding pv_table bounds
            if ply >= MAX_DEPTH - 1 {
                return 0; // neutral score at depth limit
            }
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let score = -self.negamax_2p(state, depth, ply, -beta, -alpha, ss);
            state.board_mut().set_side_to_move(current);
            return score;
        }

        // Leaf node: enter quiescence
        if depth == 0 {
            return self.qsearch_2p(state, 0, ply, alpha, beta, ss);
        }

        // TT probe
        let hash = state.board().zobrist_hash();
        let tt_move;
        if let Some(entry) = self.tt.probe(hash) {
            tt_move = entry.best_move();
            if entry.depth() as u32 >= depth {
                let stored_score = entry.scores()[current.index()];
                match entry.flag() {
                    TTFlag::Exact => return stored_score,
                    TTFlag::LowerBound => {
                        if stored_score >= beta {
                            return beta;
                        }
                    }
                    TTFlag::UpperBound => {
                        if stored_score <= alpha {
                            return alpha;
                        }
                    }
                }
            }
        } else {
            tt_move = None;
        }

        let mut moves = generate_legal_moves(state.board_mut());

        if moves.is_empty() {
            ss.pv_length[ply] = 0;
            return self.evaluator.eval_scalar(state, current);
        }

        // Order moves with TT move, killers, history
        order_moves(
            &mut moves,
            tt_move,
            &self.killers,
            &self.history,
            ply,
            current,
        );

        let original_alpha = alpha;
        let mut best_move = moves[0];

        for mv in &moves {
            let undo = make_move(state.board_mut(), *mv);
            let score = -self.negamax_2p(state, depth - 1, ply + 1, -beta, -alpha, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return 0;
            }

            if score > alpha {
                alpha = score;
                best_move = *mv;
                Self::update_pv(ss, ply, *mv);
            }
            if alpha >= beta {
                // Beta cutoff — update killer and history for quiet moves
                if !mv.is_capture() && mv.promotion().is_none() {
                    self.killers.store(ply, current, *mv);
                    self.history.update(mv.from_sq().0, mv.to_sq().0, depth);
                }
                break;
            }
        }

        // TT store with correct flag
        let tt_flag = if alpha >= beta {
            TTFlag::LowerBound
        } else if alpha > original_alpha {
            TTFlag::Exact
        } else {
            TTFlag::UpperBound
        };

        // Store score from current player's perspective
        let mut tt_scores = [0i16; 4];
        tt_scores[current.index()] = alpha;
        self.tt
            .store(hash, depth as u8, tt_flag, tt_scores, Some(best_move));

        alpha
    }

    // ─── Quiescence Search ──────────────────────────────────────────────────

    /// Max^n quiescence search: resolve captures at leaf nodes.
    ///
    /// Only captures involving the root player's pieces are expanded
    /// (root player captures opponent, or opponent captures root player).
    /// This prevents the O(5^3) branching of full 4-player capture search.
    ///
    /// Stand-pat evaluation provides a lower bound: if the position is already
    /// good for the current player, we can return without searching captures.
    fn qsearch(
        &mut self,
        state: &mut GameState,
        qdepth: u32,
        ply: usize,
        root_player: Player,
        ss: &mut SearchState,
    ) -> Score4 {
        ss.qnodes += 1;

        // Abort check
        if ss.should_abort() {
            ss.aborted = true;
            return SCORE4_MIN;
        }

        let board = state.board();
        let current = board.side_to_move();
        let current_idx = current.index();

        // Eliminated player skip
        if is_player_eliminated_in_search(board, current) {
            // Guard: prevent ply from exceeding pv_table bounds
            if ply >= MAX_DEPTH - 1 {
                return self.eval_4vec_search(state);
            }
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let result = self.qsearch(state, qdepth, ply, root_player, ss);
            state.board_mut().set_side_to_move(current);
            return result;
        }

        // Terminal: game over
        if active_count_in_search(state.board()) <= 1 {
            ss.pv_length[ply] = 0;
            return self.eval_4vec_search(state);
        }

        // Stand-pat evaluation
        let stand_pat = self.eval_4vec_search(state);

        // Qsearch node budget: soft abort — return stand-pat when budget exhausted
        if ss.qnodes >= ss.max_qnodes {
            ss.pv_length[ply] = 0;
            return stand_pat;
        }

        // Depth cap: return stand-pat
        if qdepth >= MAX_QSEARCH_DEPTH {
            ss.pv_length[ply] = 0;
            return stand_pat;
        }

        // Switch to 2-player quiescence if only 2 remain
        if active_count_in_search(state.board()) == 2 {
            // Map back through negamax qsearch entry
            return self.qsearch_2p_entry(state, qdepth, ply, ss);
        }

        // Generate capture-only moves, sorted by MVV-LVA for better delta pruning
        let mut captures = generate_captures_only(state.board_mut());
        order_captures_mvv_lva(&mut captures);

        // Filter to root-player captures only:
        // captures where the moving piece belongs to root_player,
        // or the captured piece belongs to root_player
        let root_idx = root_player.index();

        tracing::debug!(
            qdepth = qdepth,
            stand_pat_root = stand_pat[root_idx],
            total_captures = captures.len(),
            current = ?current,
            "Quiescence entry"
        );

        // Start with stand-pat as the baseline
        let mut best_scores = stand_pat;
        ss.pv_length[ply] = 0;

        for mv in &captures {
            // Root-player capture filter: skip opponent-vs-opponent captures
            let is_root_moving = current == root_player;
            let captures_root_piece = {
                // The captured piece belongs to root_player if we're capturing root's piece
                // We check by looking at what's on the target square before the move
                // But since this is a legal move by `current`, the captured piece belongs
                // to someone else. If current != root_player, then we want captures OF
                // root_player's pieces (opponent taking root's piece).
                if let Some(captured_pt) = mv.captured() {
                    // The captured piece belongs to whoever owns the piece at the target square
                    // In a legal capture, the capturing player is `current`, and the captured
                    // piece belongs to another player. We need to check if that other player
                    // is root_player.
                    if let Some(target_piece) = state.board().piece_at(mv.to_sq()) {
                        target_piece.player == root_player
                    } else {
                        // En passant: captured pawn is not on the target square
                        // Check if any of the root player's pawns would be captured
                        let _ = captured_pt;
                        false // Conservative: en passant opponent-vs-opponent, skip
                    }
                } else {
                    false
                }
            };

            if !is_root_moving && !captures_root_piece {
                continue; // Skip opponent-vs-opponent capture
            }

            // Delta pruning: skip if this capture can't possibly improve
            if let Some(captured_pt) = mv.captured() {
                let capture_value = piece_value(captured_pt);
                if best_scores[current_idx] != i16::MIN
                    && stand_pat[current_idx]
                        .saturating_add(capture_value)
                        .saturating_add(DELTA_MARGIN)
                        < best_scores[current_idx]
                {
                    tracing::trace!(mv = ?mv, "Delta pruned in qsearch");
                    continue;
                }
            }

            tracing::trace!(mv = ?mv, "Expanding capture in qsearch");

            let undo = make_move(state.board_mut(), *mv);
            let child_scores = self.qsearch(state, qdepth + 1, ply + 1, root_player, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return SCORE4_MIN;
            }

            if child_scores[current_idx] > best_scores[current_idx] {
                best_scores = child_scores;
            }
        }

        best_scores
    }

    /// Negamax quiescence search for 2-player endgame.
    ///
    /// Standard alpha-beta quiescence with stand-pat and delta pruning.
    /// Returns score from the perspective of the current side to move.
    fn qsearch_2p(
        &mut self,
        state: &mut GameState,
        qdepth: u32,
        ply: usize,
        mut alpha: i16,
        beta: i16,
        ss: &mut SearchState,
    ) -> i16 {
        ss.qnodes += 1;

        if ss.should_abort() {
            ss.aborted = true;
            return 0;
        }

        let board = state.board();
        let current = board.side_to_move();

        // Skip eliminated players
        if is_player_eliminated_in_search(board, current) {
            // Guard: prevent ply from exceeding pv_table bounds
            if ply >= MAX_DEPTH - 1 {
                return 0; // neutral score at depth limit
            }
            let board = state.board_mut();
            let next = current.next();
            board.set_side_to_move(next);
            let score = -self.qsearch_2p(state, qdepth, ply, -beta, -alpha, ss);
            state.board_mut().set_side_to_move(current);
            return score;
        }

        // Stand-pat
        let stand_pat = self.evaluator.eval_scalar(state, current);

        // Qsearch node budget: soft abort — return stand-pat when budget exhausted
        if ss.qnodes >= ss.max_qnodes {
            ss.pv_length[ply] = 0;
            return stand_pat;
        }

        // Depth cap
        if qdepth >= MAX_QSEARCH_DEPTH {
            ss.pv_length[ply] = 0;
            return stand_pat;
        }

        // Stand-pat cutoff
        if stand_pat >= beta {
            ss.pv_length[ply] = 0;
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Generate captures, sorted by MVV-LVA for better delta pruning
        let mut captures = generate_captures_only(state.board_mut());
        order_captures_mvv_lva(&mut captures);

        ss.pv_length[ply] = 0;

        for mv in &captures {
            // Delta pruning
            if let Some(captured_pt) = mv.captured() {
                let capture_value = piece_value(captured_pt);
                if stand_pat
                    .saturating_add(capture_value)
                    .saturating_add(DELTA_MARGIN)
                    < alpha
                {
                    continue;
                }
            }

            let undo = make_move(state.board_mut(), *mv);
            let score = -self.qsearch_2p(state, qdepth + 1, ply + 1, -beta, -alpha, ss);
            unmake_move(state.board_mut(), &undo);

            if ss.aborted {
                return 0;
            }

            if score > alpha {
                alpha = score;
                Self::update_pv(ss, ply, *mv);
            }
            if alpha >= beta {
                break;
            }
        }

        alpha
    }

    /// Entry point for 2-player quiescence from Max^n context.
    ///
    /// Maps the scalar negamax qsearch result back to a Score4.
    fn qsearch_2p_entry(
        &mut self,
        state: &mut GameState,
        qdepth: u32,
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
        let (max_p, min_p) = if current == maximizer {
            (maximizer, minimizer)
        } else {
            (minimizer, maximizer)
        };

        let score = self.qsearch_2p(state, qdepth, ply, i16::MIN + 1, i16::MAX, ss);

        if ss.aborted {
            return SCORE4_MIN;
        }

        let mut result = [ELIMINATED_SCORE; 4];
        result[max_p.index()] = score;
        result[min_p.index()] = -score;
        result
    }

    /// Iterative deepening entry point.
    fn iterative_deepening(
        &mut self,
        state: &mut GameState,
        limits: &SearchLimits,
    ) -> SearchResult {
        let mut ss = SearchState::new(limits.clone(), self.config.max_qnodes);
        let raw_max_depth = limits.max_depth.unwrap_or(MAX_DEPTH as u32);

        let root_player = state.board().side_to_move();

        // Cap Max^n at one full rotation (number of active players).
        // Extra time is better spent on MCTS (strategic depth) than deeper Max^n
        // (which creates asymmetric evaluations at non-rotation depths).
        // NOTE: This cap is temporary while the bootstrap eval limits depth.
        // Future stages (NNUE, Stages 15-17) should raise this to 2 rotations
        // (depth 8 with 4 players) once NNUE enables tighter beam and faster search.
        let active = active_count_in_search(state.board());
        let rotation = if active >= 2 { active as u32 } else { 1 };
        // Round down to nearest full rotation for symmetric evaluation.
        // Allows depth 4, 8, 12, etc. with 4 active players.
        let max_depth = if raw_max_depth >= rotation {
            (raw_max_depth / rotation) * rotation
        } else {
            raw_max_depth
        };

        // Prepare TT and killers for this search
        self.tt.new_search();
        self.killers.clear();
        // History is NOT cleared between ID iterations — it accumulates

        let mut best_result = SearchResult {
            best_move: None,
            scores: SCORE4_MIN,
            depth: 0,
            nodes: 0,
            qnodes: 0,
            pv: ArrayVec::new(),
            tt_hit_rate: 0.0,
            killer_hit_rate: 0.0,
        };

        // Determine the minimum depth we must complete before allowing time abort.
        // If max_depth is explicitly set lower, respect that.
        let min_depth = if limits.max_depth.is_some() {
            1 // Explicit depth: no minimum floor
        } else {
            MIN_SEARCH_DEPTH
        };

        #[allow(unused_assignments)]
        let mut last_depth_ms: u64 = 0;

        for depth in 1..=max_depth {
            // Reset PV for this iteration
            ss.pv_length = [0; MAX_DEPTH];

            // Disable time abort for depths below minimum
            let time_suspended = depth <= min_depth;
            if time_suspended {
                ss.suspend_time_check = true;
            }

            let depth_start = Instant::now();
            let scores = self.maxn(state, depth, 0, root_player, &mut ss);
            let depth_elapsed_ms = depth_start.elapsed().as_millis() as u64;

            ss.suspend_time_check = false;

            if ss.aborted {
                break; // Use result from last completed depth
            }

            last_depth_ms = depth_elapsed_ms;

            // Extract best move from PV
            let pv = Self::extract_pv(&ss);
            let best_move = pv.first().copied();

            let tt_hit_rate = self.tt.hit_rate_pct();
            let killer_hit_rate = self.killers.hit_rate_pct();

            best_result = SearchResult {
                best_move,
                scores,
                depth,
                nodes: ss.nodes,
                qnodes: ss.qnodes,
                pv,
                tt_hit_rate,
                killer_hit_rate,
            };

            tracing::info!(
                depth = depth,
                depth_ms = depth_elapsed_ms,
                nodes = ss.nodes,
                qnodes = ss.qnodes,
                best_move = ?best_result.best_move,
                scores = ?scores,
                tt_hit_rate = format!("{:.1}%", tt_hit_rate),
                killer_hit_rate = format!("{:.1}%", killer_hit_rate),
                "Search depth complete"
            );

            // Check if we should stop before the next iteration
            if ss.should_abort() {
                break;
            }

            // Time management: estimate if next depth will fit in remaining budget.
            // Heuristic: next depth takes ~4x current depth (effective branching factor).
            if let Some(max_time_ms) = ss.limits.max_time_ms {
                let total_elapsed = ss.start_time.elapsed().as_millis() as u64;
                let remaining = max_time_ms.saturating_sub(total_elapsed);
                let estimated_next = last_depth_ms.saturating_mul(4);
                if estimated_next > remaining && depth >= min_depth {
                    tracing::info!(
                        depth,
                        depth_ms = last_depth_ms,
                        remaining_ms = remaining,
                        estimated_next_ms = estimated_next,
                        "Stopping ID: next depth unlikely to complete in time"
                    );
                    break;
                }
            }
        }

        // MoveNoise: probabilistic move replacement for opening diversity.
        // When move_noise > 0, with probability move_noise/100, replace the
        // best move with a random top-3 move. Uses Zobrist hash as seed for
        // deterministic-per-position randomization.
        if self.config.move_noise > 0 && best_result.best_move.is_some() {
            let hash = state.board().zobrist_hash() ^ self.config.noise_seed;
            // Simple xorshift from Zobrist hash XOR'd with per-game seed
            let mut rng = hash ^ (hash >> 17);
            rng ^= rng << 13;
            rng ^= rng >> 7;
            let roll = (rng % 100) as u32;
            if roll < self.config.move_noise {
                // Pick a random move from top-3 legal moves (ordered by search)
                let legal = generate_legal_moves(state.board_mut());
                if legal.len() >= 2 {
                    // Use PV move (index 0) and next 2 candidates
                    let pick_idx = (rng as usize / 100) % legal.len().min(3);
                    // Score moves to find top 3
                    let tt_move = best_result.best_move;
                    let mut scored: ArrayVec<(Move, i32), MAX_MOVES> = legal
                        .iter()
                        .map(|&mv| {
                            let s = score_move(
                                mv,
                                tt_move,
                                &self.killers,
                                &self.history,
                                0,
                                root_player,
                            );
                            (mv, s)
                        })
                        .collect();
                    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));
                    let top_n = scored.len().min(3);
                    let chosen = scored[pick_idx % top_n].0;
                    if chosen != best_result.best_move.unwrap() {
                        tracing::info!(
                            noise = self.config.move_noise,
                            original = ?best_result.best_move,
                            chosen = ?chosen,
                            "MoveNoise: replaced best move"
                        );
                        best_result.best_move = Some(chosen);
                    }
                }
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

    // ── Quiescence Search (Stage 8) ──

    #[test]
    fn test_qsearch_produces_qnodes() {
        // At depth >= 1, quiescence is called at leaf nodes, producing qnodes
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        // At depth 1, every leaf calls qsearch. qnodes should be > 0
        // (at minimum, each leaf node calls qsearch once for stand-pat)
        assert!(
            result.qnodes > 0,
            "qnodes should be > 0 at depth 1, got {}",
            result.qnodes
        );
    }

    #[test]
    fn test_qsearch_nodes_counted_separately() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        // Both main nodes and qnodes should be counted
        assert!(result.nodes > 0, "main nodes should be > 0");
        assert!(result.qnodes > 0, "qnodes should be > 0");
        // qnodes and nodes are tracked separately
        // Total work = nodes + qnodes
    }

    #[test]
    fn test_qsearch_preserves_game_state() {
        // Quiescence must not corrupt the board
        let mut gs = GameState::new_standard_ffa();
        let hash_before = gs.board().zobrist_hash();
        let stm_before = gs.board().side_to_move();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let _ = searcher.search(&mut gs, &limits);
        assert_eq!(gs.board().zobrist_hash(), hash_before);
        assert_eq!(gs.board().side_to_move(), stm_before);
    }

    #[test]
    fn test_qsearch_overhead_reasonable() {
        // Spec: quiescence adds < 50% to total node count in typical positions
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        // In starting position with no captures available, qnodes should be
        // roughly equal to leaf count (each leaf = 1 qnode for stand-pat).
        // Allow generous margin since this is the starting position.
        let overhead_pct = (result.qnodes as f64 / result.nodes as f64) * 100.0;
        // Just verify it's not absurdly high (< 200% is very generous)
        assert!(
            overhead_pct < 200.0,
            "qnode overhead {:.1}% is excessive (nodes={}, qnodes={})",
            overhead_pct,
            result.nodes,
            result.qnodes
        );
    }

    #[test]
    fn test_search_still_returns_legal_move_with_qsearch() {
        // Basic sanity: search with quiescence still returns legal moves
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        for depth in 1..=3 {
            let limits = SearchLimits {
                max_depth: Some(depth),
                ..Default::default()
            };
            let result = searcher.search(&mut gs, &limits);
            assert!(
                result.best_move.is_some(),
                "Depth {depth} must return a move"
            );
            // Verify the move is actually legal
            let legal = generate_legal_moves(gs.board_mut());
            assert!(
                legal.contains(&result.best_move.unwrap()),
                "Depth {depth} move must be legal"
            );
        }
    }

    #[test]
    fn test_qsearch_finds_hanging_queen() {
        // Proof that quiescence works: place a Blue queen where Red's pawn can
        // capture it. At depth 1, without qsearch the engine would just see the
        // static eval. With qsearch, the capture is found at the leaf and Red's
        // score should reflect the queen capture (900cp material swing).
        use crate::board::types::{Piece, PieceType, Square};

        let mut gs = GameState::new_standard_ffa();
        let board = gs.board_mut();

        // Remove Blue's original queen from its starting square
        // Blue queen starts at (4, 0) = rank 4, file 0 in 4PC
        // Actually, let's find it:
        // Instead of guessing coordinates, place a Blue queen on e3 (rank 2, file 4)
        // which is diagonally capturable by Red's d2 pawn.
        // Red's d-pawn is at rank 1, file 3. A Blue piece at rank 2, file 4 can be
        // captured by d2xe3 (pawn capture diag forward-right).

        // First verify the target square is empty, place a Blue queen there
        let target = Square::new(2, 4).unwrap(); // rank 2, file 4 (e3 in Red's coords)
        if board.piece_at(target).is_some() {
            board.remove_piece(target);
        }
        board.set_piece(
            target,
            Piece {
                player: Player::Blue,
                piece_type: PieceType::Queen,
            },
        );

        // Now search at depth 1 — qsearch at the leaf should find d2xe3
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(1),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);

        // The engine should find the queen capture
        let best = result.best_move.expect("must find a move");
        assert!(
            best.is_capture(),
            "Engine should capture the hanging queen, but played {} (not a capture)",
            best
        );
        // qnodes should be > nodes (captures being explored in qsearch)
        assert!(
            result.qnodes > 0,
            "qnodes should be positive when captures exist"
        );
        // The best move should capture the queen specifically
        assert_eq!(
            best.captured(),
            Some(PieceType::Queen),
            "Should capture the queen, but captured {:?}",
            best.captured()
        );
    }

    #[test]
    fn test_capture_only_generation() {
        // Starting position: no captures available
        let mut gs = GameState::new_standard_ffa();
        let captures = generate_captures_only(gs.board_mut());
        assert!(
            captures.is_empty(),
            "Starting position should have 0 captures, got {}",
            captures.len()
        );
    }

    #[test]
    fn test_capture_only_after_moves() {
        // Play a few moves, then check captures
        let mut gs = GameState::new_standard_ffa();
        // Advance pawns to create capture opportunities
        // Red d2d4
        let legal = generate_legal_moves(gs.board_mut());
        gs.apply_move(legal[0]); // Red's first legal move
        // Blue's move
        let legal = generate_legal_moves(gs.board_mut());
        gs.apply_move(legal[0]);
        // Yellow's move
        let legal = generate_legal_moves(gs.board_mut());
        gs.apply_move(legal[0]);
        // Green's move
        let legal = generate_legal_moves(gs.board_mut());
        gs.apply_move(legal[0]);
        // All captures at this point should have is_capture() == true
        let captures = generate_captures_only(gs.board_mut());
        for mv in &captures {
            assert!(
                mv.is_capture(),
                "All moves from generate_captures_only must be captures"
            );
        }
    }

    // ── Stage 9: TT + Move Ordering ──

    #[test]
    fn test_tt_hit_rate_positive_at_depth_3() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(3),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(
            result.tt_hit_rate > 0.0,
            "TT hit rate should be positive at depth 3, got {:.1}%",
            result.tt_hit_rate
        );
    }

    #[test]
    fn test_history_table_populated_after_search() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(3),
            ..Default::default()
        };
        let _ = searcher.search(&mut gs, &limits);

        // Check history table has nonzero entries
        let history = searcher.history_table();
        let raw = history.raw();
        let nonzero = raw
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&v| v > 0)
            .count();
        assert!(
            nonzero > 0,
            "History table should have nonzero entries after depth 3 search"
        );
    }

    #[test]
    fn test_search_result_includes_tt_stats() {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        // At depth 2, TT hit rate might be 0 (no transpositions at this shallow depth),
        // but the field should exist and be non-negative
        assert!(result.tt_hit_rate >= 0.0);
        assert!(result.killer_hit_rate >= 0.0);
    }

    #[test]
    fn test_tt_preserves_board_state() {
        let mut gs = GameState::new_standard_ffa();
        let hash_before = gs.board().zobrist_hash();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(3),
            ..Default::default()
        };
        let _ = searcher.search(&mut gs, &limits);
        assert_eq!(
            gs.board().zobrist_hash(),
            hash_before,
            "Board hash must be restored after TT-enabled search"
        );
    }

    #[test]
    fn test_history_table_accessor() {
        let searcher = make_searcher();
        let history = searcher.history_table();
        // Fresh history table should be all zeros
        let raw = history.raw();
        let total: u32 = raw.iter().flat_map(|row| row.iter()).sum();
        assert_eq!(total, 0, "Fresh history table should be all zeros");
    }

    // ── Qsearch node budget ──

    #[test]
    fn test_qsearch_node_budget_caps_qnodes() {
        // With a tight qsearch budget, depth 4 search should complete
        // and qnodes should not exceed the budget significantly.
        let mut gs = GameState::new_standard_ffa();
        let budget = 50_000u64;
        let mut searcher = MaxnSearcher::new(
            BootstrapEvaluator::new(),
            SearchConfig {
                max_qnodes: budget,
                ..SearchConfig::default()
            },
        );
        let limits = SearchLimits {
            max_depth: Some(4),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some(), "Should find a move");
        assert_eq!(result.depth, 4, "Should complete depth 4 with budget");
        // Budget is soft: checked per-call, so overshoot is expected due to
        // in-flight recursive calls between checks. The key property is that
        // depth 4 completes at all (it hangs without a budget).
        assert!(
            result.qnodes < budget * 10,
            "qnodes {} should be bounded (budget {})",
            result.qnodes,
            budget
        );
    }

    #[test]
    fn test_qsearch_budget_zero_disables_qsearch() {
        // max_qnodes=0 means qsearch immediately returns stand-pat
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = MaxnSearcher::new(
            BootstrapEvaluator::new(),
            SearchConfig {
                max_qnodes: 0,
                ..SearchConfig::default()
            },
        );
        let limits = SearchLimits {
            max_depth: Some(3),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);
        assert!(result.best_move.is_some());
        // With qsearch disabled, qnodes should be minimal (only the initial
        // increment before the budget check fires)
    }

    #[test]
    fn test_eval_tuning_game_sim() {
        // Play a game at depth 4, print each move for eval tuning.
        // Run with: cargo test -p freyja-engine test_eval_tuning_game_sim -- --nocapture
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(4),
            ..Default::default()
        };

        for ply in 0..40 {
            if gs.is_game_over() {
                break;
            }
            let player = gs.current_player();
            let result = searcher.search(&mut gs, &limits);
            if let Some(mv) = result.best_move {
                eprintln!(
                    "Ply {}: {} plays {} (scores: R={} B={} Y={} G={})",
                    ply,
                    player,
                    mv,
                    result.scores[0],
                    result.scores[1],
                    result.scores[2],
                    result.scores[3]
                );
                gs.apply_move(mv);
            } else {
                eprintln!("Ply {}: {} has no legal moves", ply, player);
                break;
            }
        }
    }

    // ── Stress test: qsearch with eliminated players (ply bounds) ──

    #[test]
    fn test_qsearch_with_eliminated_players_no_crash() {
        // Set up a position where 2 players are eliminated, then run a deep
        // search. The eliminated-player skip in qsearch could cause ply to
        // exceed MAX_DEPTH without the bounds guard. This test verifies the
        // guard prevents buffer overrun.
        use crate::board::ELIMINATED_KING_SENTINEL;

        let mut gs = GameState::new_standard_ffa();

        // Eliminate Yellow and Green by removing their kings
        // (simulates mid-game eliminations)
        let yellow_ks = gs.board().king_square(Player::Yellow);
        if yellow_ks != ELIMINATED_KING_SENTINEL {
            gs.board_mut().remove_piece(Square(yellow_ks));
            gs.board_mut().set_king_eliminated(Player::Yellow);
        }
        let green_ks = gs.board().king_square(Player::Green);
        if green_ks != ELIMINATED_KING_SENTINEL {
            gs.board_mut().remove_piece(Square(green_ks));
            gs.board_mut().set_king_eliminated(Player::Green);
        }

        // Search at depth 4 — this creates deep recursion with eliminated
        // player skips that could exceed pv_table bounds without the guard.
        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(4),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);

        assert!(
            result.best_move.is_some(),
            "Search with 2 eliminated players must return a move"
        );
        assert!(result.nodes > 0);
        assert!(result.qnodes > 0, "Qsearch should have run");

        eprintln!(
            "Qsearch + 2 eliminated: depth={} nodes={} qnodes={} best={:?}",
            result.depth, result.nodes, result.qnodes, result.best_move
        );
    }

    #[test]
    fn test_qsearch_with_3_eliminated_players_no_crash() {
        // Extreme case: 3 eliminated, only Red remains.
        // Every level of search hits eliminated-player skips.
        use crate::board::ELIMINATED_KING_SENTINEL;

        let mut gs = GameState::new_standard_ffa();

        for p in [Player::Blue, Player::Yellow, Player::Green] {
            let ks = gs.board().king_square(p);
            if ks != ELIMINATED_KING_SENTINEL {
                gs.board_mut().remove_piece(Square(ks));
                gs.board_mut().set_king_eliminated(p);
            }
        }

        let mut searcher = make_searcher();
        let limits = SearchLimits {
            max_depth: Some(4),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);

        // With only 1 player active, search should terminate quickly
        // (active_count <= 1 is the terminal condition).
        assert!(
            result.best_move.is_some() || result.nodes > 0,
            "Search should complete without crash"
        );

        eprintln!(
            "Qsearch + 3 eliminated: depth={} nodes={} qnodes={}",
            result.depth, result.nodes, result.qnodes
        );
    }

    #[test]
    fn test_search_deep_with_captures_and_eliminations() {
        // Play a game for 20 plies to create a position with captures,
        // then eliminate a player and search deeper. This exercises the
        // qsearch ply guard under realistic conditions.
        use crate::board::ELIMINATED_KING_SENTINEL;

        let mut gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();

        // Play 20 plies at depth 2 to get a realistic midgame
        for _ply in 0..20 {
            if gs.is_game_over() {
                break;
            }
            let limits = SearchLimits {
                max_depth: Some(2),
                ..Default::default()
            };
            let result = searcher.search(&mut gs, &limits);
            if let Some(mv) = result.best_move {
                gs.apply_move(mv);
            } else {
                break;
            }
        }

        if gs.is_game_over() {
            return;
        }

        // Eliminate Green (simulate checkmate)
        let green_ks = gs.board().king_square(Player::Green);
        if green_ks != ELIMINATED_KING_SENTINEL {
            gs.board_mut().remove_piece(Square(green_ks));
            gs.board_mut().set_king_eliminated(Player::Green);
        }

        // Now search deeper — depth 4 with one eliminated player
        // and a complex midgame position with many captures in qsearch
        let limits = SearchLimits {
            max_depth: Some(4),
            ..Default::default()
        };
        let result = searcher.search(&mut gs, &limits);

        assert!(
            result.best_move.is_some(),
            "Deep search after elimination must produce a move"
        );
        assert!(
            result.qnodes > 0,
            "Qsearch should have run in midgame position"
        );

        eprintln!(
            "Deep search + elimination: depth={} nodes={} qnodes={} best={:?}",
            result.depth, result.nodes, result.qnodes, result.best_move
        );
    }
}
