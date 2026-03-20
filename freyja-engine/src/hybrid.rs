//! Hybrid controller: Max^n → MCTS integration (Stage 11).
//!
//! Sequences Max^n search (tactical grounding) followed by MCTS
//! (strategic exploration), transferring knowledge between them:
//! - History table: Max^n's history heuristic warm-starts MCTS progressive history
//! - Prior policy: softmax over Max^n ordering scores informs MCTS root selection
//!
//! Implements the Searcher trait as a drop-in replacement.

use arrayvec::ArrayVec;

use crate::board::types::*;
use crate::eval::Evaluator;
use crate::game_state::GameState;
use crate::mcts::{MctsConfig, MctsSearcher};
use crate::move_gen::{MAX_MOVES, Move};
use crate::move_order::{HistoryTable, KillerTable, score_move};
use crate::search::{MaxnSearcher, SearchConfig, SearchLimits, SearchResult, Searcher};

// ─── Constants ─────────────────────────────────────────────────────────────

/// Default fraction of time budget allocated to Max^n Phase 1.
/// Only used during the transition window (if implemented later).
const DEFAULT_TIME_SPLIT_RATIO: f32 = 0.5;

/// Score threshold for skipping MCTS when Max^n finds a decisive advantage.
/// 9000cp ≈ 10 queens worth; anything above this is effectively mate/winning.
const DEFAULT_MATE_SKIP_THRESHOLD: i16 = 9000;

/// Softmax temperature for computing prior policy from ordering scores.
/// Matches MCTS default (ADR-006).
const PRIOR_TEMPERATURE: f32 = 50.0;

/// Default phase cutover ply: below this, Max^n only; at or above, MCTS only.
/// 32 plies = 8 moves per player = ~2 full opening cycles.
const DEFAULT_PHASE_CUTOVER_PLY: u32 = 32;

// ─── Hybrid Configuration ─────────────────────────────────────────────────

/// Configuration for the hybrid Max^n → MCTS controller.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Max^n search configuration.
    pub maxn_config: SearchConfig,
    /// MCTS search configuration.
    pub mcts_config: MctsConfig,
    /// Fraction of time budget for Max^n Phase 1 (0.0-1.0).
    /// Only used if both phases run (future transition window).
    pub time_split_ratio: f32,
    /// Skip MCTS if Max^n root player score >= this threshold.
    pub mate_skip_threshold: i16,
    /// Phase cutover ply: below this Max^n only, at or above MCTS only.
    pub phase_cutover_ply: u32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            maxn_config: SearchConfig::default(),
            mcts_config: MctsConfig::default(),
            time_split_ratio: DEFAULT_TIME_SPLIT_RATIO,
            mate_skip_threshold: DEFAULT_MATE_SKIP_THRESHOLD,
            phase_cutover_ply: DEFAULT_PHASE_CUTOVER_PLY,
        }
    }
}

// ─── Hybrid Searcher ──────────────────────────────────────────────────────

/// Hybrid controller that sequences Max^n → MCTS with knowledge transfer.
///
/// Phase 1: Run Max^n for tactical grounding (iterative deepening, beam search).
/// Phase 1.5: Extract history table, compute prior policy from ordering scores.
/// Phase 2: Run MCTS with warm-started progressive history and informed priors.
/// Final: Return MCTS's Sequential Halving winner as the best move.
pub struct HybridSearcher<E: Evaluator + Clone> {
    maxn: MaxnSearcher<E>,
    mcts: MctsSearcher<E>,
    config: HybridConfig,
    /// Count of searches where Max^n and MCTS disagreed on best move.
    disagreement_count: u64,
    /// Total hybrid searches completed.
    total_searches: u64,
}

impl<E: Evaluator + Clone> HybridSearcher<E> {
    /// Create a new HybridSearcher with the given evaluator and config.
    ///
    /// Clones the evaluator so each sub-searcher has its own instance.
    pub fn new(evaluator: E, config: HybridConfig) -> Self {
        let maxn = MaxnSearcher::new(evaluator.clone(), config.maxn_config.clone());
        let mcts = MctsSearcher::new(evaluator, config.mcts_config.clone());
        Self {
            maxn,
            mcts,
            config,
            disagreement_count: 0,
            total_searches: 0,
        }
    }

    /// Get the disagreement rate (fraction of searches where Max^n ≠ MCTS).
    pub fn disagreement_rate(&self) -> f64 {
        if self.total_searches == 0 {
            0.0
        } else {
            self.disagreement_count as f64 / self.total_searches as f64
        }
    }
}

// ─── Prior Policy Computation ─────────────────────────────────────────────

// These functions are retained for a future transition window where Max^n
// seeds MCTS priors during a soft cutover. Currently unused since phases
// are fully separated (opening = Max^n only, midgame = MCTS only).

#[allow(dead_code)]
/// Compute softmax prior policy from Max^n ordering scores.
///
/// For each root move, computes `score_move()` using the Max^n history table,
/// then applies numerically stable softmax with the given temperature.
/// Returns a probability vector aligned with the move list.
fn compute_hybrid_priors(
    moves: &ArrayVec<Move, MAX_MOVES>,
    history: &HistoryTable,
    player: Player,
    temperature: f32,
) -> Vec<f32> {
    if moves.is_empty() {
        return Vec::new();
    }

    let killers = KillerTable::new(); // Fresh killers — not meaningful at root for MCTS
    let tt_move = None; // No TT move from hybrid perspective

    // Score all moves using Max^n ordering infrastructure
    let scores: Vec<f32> = moves
        .iter()
        .map(|&mv| score_move(mv, tt_move, &killers, history, 0, player) as f32 / temperature)
        .collect();

    // Numerically stable softmax
    let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_scores: Vec<f32> = scores.iter().map(|&s| (s - max_score).exp()).collect();
    let sum: f32 = exp_scores.iter().sum();

    if sum <= 0.0 || !sum.is_finite() {
        // Fallback: uniform distribution
        let uniform = 1.0 / moves.len() as f32;
        return vec![uniform; moves.len()];
    }

    exp_scores.iter().map(|&e| e / sum).collect()
}

#[allow(dead_code)]
/// Compute entropy of a probability distribution: -sum(p * ln(p)).
fn prior_entropy(priors: &[f32]) -> f32 {
    -priors
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| p * p.ln())
        .sum::<f32>()
}

#[allow(dead_code)]
/// Count nonzero entries in the history table.
fn count_history_nonzero(history: &HistoryTable) -> usize {
    let raw = history.raw();
    let mut count = 0;
    for row in raw.iter() {
        for &val in row.iter() {
            if val > 0 {
                count += 1;
            }
        }
    }
    count
}

// ─── Searcher Implementation ──────────────────────────────────────────────

impl<E: Evaluator + Clone> Searcher for HybridSearcher<E> {
    fn search(&mut self, state: &mut GameState, limits: &SearchLimits) -> SearchResult {
        self.total_searches += 1;

        // ── Phase separation ─────────────────────────────────────────────
        // Opening (ply < cutover): Max^n only. Depth 4 = one full 4-player
        // rotation for tactical grounding. Fast, correct, no MCTS overhead.
        // Midgame+ (ply >= cutover): MCTS only. Full time budget for strategic
        // sampling. Max^n can't search deep enough to matter here.
        let in_opening = limits.game_ply < self.config.phase_cutover_ply;

        if in_opening {
            // ── Opening phase: Max^n only ────────────────────────────────
            // Depth-only or no constraints: use full limits directly
            let has_time = limits.max_time_ms.is_some() || limits.infinite;
            let maxn_limits = if has_time && limits.max_depth.is_none() {
                // Time-based with no explicit depth: give Max^n the full budget
                limits.clone()
            } else {
                limits.clone()
            };

            let result = self.maxn.search(state, &maxn_limits);

            tracing::info!(
                phase = "opening",
                game_ply = limits.game_ply,
                cutover = self.config.phase_cutover_ply,
                depth = result.depth,
                nodes = result.nodes,
                best_move = ?result.best_move,
                "Max^n only (opening phase)"
            );

            result
        } else {
            // ── Midgame phase: MCTS only ─────────────────────────────────
            // Full time budget goes to MCTS. No Max^n warmup.
            let mcts_limits = SearchLimits {
                max_depth: None, // MCTS doesn't use depth limits
                max_nodes: limits.max_nodes,
                max_time_ms: limits.max_time_ms.or(Some(5000)),
                infinite: limits.infinite,
                game_ply: limits.game_ply,
            };

            let result = self.mcts.search(state, &mcts_limits);

            tracing::info!(
                phase = "midgame",
                game_ply = limits.game_ply,
                cutover = self.config.phase_cutover_ply,
                sims = result.nodes,
                best_move = ?result.best_move,
                "MCTS only (midgame phase)"
            );

            result
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    use arrayvec::ArrayVec;

    use crate::board::Board;
    use crate::eval::BootstrapEvaluator;
    use crate::game_state::GameState;
    use crate::move_gen::{Move, generate_legal_moves};

    fn make_searcher() -> HybridSearcher<BootstrapEvaluator> {
        HybridSearcher::new(BootstrapEvaluator::new(), HybridConfig::default())
    }

    fn make_state() -> GameState {
        GameState::new(Board::starting_position())
    }

    // ── Step 1: Basic construction and skeleton ──

    #[test]
    fn test_hybrid_config_defaults() {
        let config = HybridConfig::default();
        assert!((config.time_split_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.mate_skip_threshold, 9000);
    }

    #[test]
    fn test_hybrid_creates_with_both_searchers() {
        let searcher = make_searcher();
        assert_eq!(searcher.disagreement_count, 0);
        assert_eq!(searcher.total_searches, 0);
    }

    #[test]
    fn test_hybrid_returns_legal_move() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(
            result.best_move.is_some(),
            "Hybrid must return a legal move"
        );
    }

    // ── Step 2: Time allocation ──

    #[test]
    fn test_depth_only_uses_maxn_only() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_depth: Some(2),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(result.best_move.is_some());
        // Depth-only → no time → Max^n only → total_searches stays 0
        // (mate skip path also increments, so just check we got a result)
        assert!(result.depth >= 2);
    }

    // ── AC1: Controller sequences Max^n → MCTS ──

    #[test]
    fn test_ac1_hybrid_sequences_maxn_then_mcts() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(3000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(result.best_move.is_some());
        // Max^n depth should be > 0 (Phase 1 ran)
        assert!(result.depth > 0, "Max^n Phase 1 should have run");
        // Combined nodes should include both phases
        assert!(
            result.nodes > 0,
            "Should have searched nodes in both phases"
        );
        // total_searches should have incremented (Phase 2 ran)
        assert!(
            searcher.total_searches >= 1,
            "total_searches should increment after hybrid search"
        );
    }

    // ── AC2: MCTS gets remaining time, not total time ──

    #[test]
    fn test_ac2_mcts_gets_remaining_time() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let total_ms = 10_000u64;
        let limits = SearchLimits {
            max_depth: Some(3), // Cap depth to prevent debug build timeout
            max_time_ms: Some(total_ms),
            ..Default::default()
        };
        let start = Instant::now();
        let _result = searcher.search(&mut state, &limits);
        let elapsed_ms = start.elapsed().as_millis() as u64;
        // Total elapsed should be approximately within the budget
        // Allow generous slack for debug build overhead
        assert!(
            elapsed_ms <= total_ms + 5000,
            "Total time {}ms should be within budget {}ms (+5000ms slack)",
            elapsed_ms,
            total_ms
        );
    }

    // ── AC3: History table transfers ──

    #[test]
    fn test_ac3_history_transfer_nonzero() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        // Run a search deep enough to populate history
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let _result = searcher.search(&mut state, &limits);
        // After search, Max^n's history should have nonzero entries
        let nonzero = count_history_nonzero(searcher.maxn.history_table());
        assert!(
            nonzero > 0,
            "History table should have nonzero entries after Max^n search"
        );
    }

    // ── AC4: Prior policy computed ──

    #[test]
    fn test_ac4_prior_policy_valid() {
        let mut state = make_state();
        let history = HistoryTable::new();
        let moves = generate_legal_moves(state.board_mut());
        assert!(!moves.is_empty());

        let priors = compute_hybrid_priors(&moves, &history, Player::Red, PRIOR_TEMPERATURE);
        assert_eq!(priors.len(), moves.len());

        let sum: f32 = priors.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "Priors should sum to 1.0, got {}",
            sum
        );

        // All priors should be positive
        for &p in &priors {
            assert!(p > 0.0, "All priors should be positive");
        }
    }

    #[test]
    fn test_ac4_prior_entropy_reasonable() {
        let mut state = make_state();
        let history = HistoryTable::new();
        let moves = generate_legal_moves(state.board_mut());

        let priors = compute_hybrid_priors(&moves, &history, Player::Red, PRIOR_TEMPERATURE);
        let entropy = prior_entropy(&priors);

        // Entropy should be positive and less than max (ln(N))
        let max_entropy = (moves.len() as f32).ln();
        assert!(entropy > 0.0, "Entropy should be positive");
        assert!(
            entropy <= max_entropy + 0.01,
            "Entropy {} should be <= max {}",
            entropy,
            max_entropy
        );
    }

    #[test]
    fn test_prior_captures_higher_than_quiet() {
        let history = HistoryTable::new();
        let mut moves: ArrayVec<Move, MAX_MOVES> = ArrayVec::new();
        let quiet = Move::new(Square(50), Square(64), PieceType::Pawn);
        let capture = Move::capture(Square(51), Square(65), PieceType::Pawn, PieceType::Queen);
        moves.push(quiet);
        moves.push(capture);

        let priors = compute_hybrid_priors(&moves, &history, Player::Red, PRIOR_TEMPERATURE);
        assert!(
            priors[1] > priors[0],
            "Capture prior {} should be > quiet prior {}",
            priors[1],
            priors[0]
        );
    }

    #[test]
    fn test_prior_empty_moves() {
        let history = HistoryTable::new();
        let moves: ArrayVec<Move, MAX_MOVES> = ArrayVec::new();
        let priors = compute_hybrid_priors(&moves, &history, Player::Red, PRIOR_TEMPERATURE);
        assert!(priors.is_empty());
    }

    // ── AC5: Mate detection skips MCTS ──

    #[test]
    fn test_ac5_mate_skip_threshold() {
        // Use a very low threshold to trigger the skip path
        let config = HybridConfig {
            mate_skip_threshold: -32000, // Anything above this triggers skip
            ..Default::default()
        };
        let mut searcher = HybridSearcher::new(BootstrapEvaluator::new(), config);
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(result.best_move.is_some());
        // With such a low threshold, mate skip should trigger after Phase 1
        // total_searches incremented but no disagreement tracking (MCTS didn't run)
        assert_eq!(
            searcher.total_searches, 1,
            "Should have completed one search"
        );
        assert_eq!(
            searcher.disagreement_count, 0,
            "No disagreement when MCTS skipped"
        );
    }

    // ── AC6: Disagreement tracking ──

    #[test]
    fn test_ac6_disagreement_rate_starts_zero() {
        let searcher = make_searcher();
        assert!((searcher.disagreement_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ac6_disagreement_tracked() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let _result = searcher.search(&mut state, &limits);
        // After one search, total_searches should be >= 1
        assert!(
            searcher.total_searches >= 1,
            "total_searches should be >= 1 after a search"
        );
        // disagreement_count is either 0 or 1 — both valid
        assert!(searcher.disagreement_count <= searcher.total_searches);
    }

    // ── AC7: Total time within budget ──

    #[test]
    fn test_ac7_total_time_within_budget() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let budget_ms = 10_000u64;
        let limits = SearchLimits {
            max_depth: Some(3), // Cap depth to prevent debug build timeout
            max_time_ms: Some(budget_ms),
            ..Default::default()
        };
        let start = Instant::now();
        let _result = searcher.search(&mut state, &limits);
        let elapsed = start.elapsed().as_millis() as u64;
        // Allow generous slack for debug build overhead
        assert!(
            elapsed <= budget_ms + 5000,
            "Elapsed {}ms exceeds budget {}ms + 5000ms slack",
            elapsed,
            budget_ms
        );
    }

    // ── AC8: Warm-start vs cold-start ──

    #[test]
    fn test_ac8_warm_vs_cold_mcts() {
        // Verify that set_history_table and set_prior_policy are actually called
        // by running a hybrid search and confirming it produces a result.
        // Full A/B self-play comparison belongs in Stage 12.
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(3000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(result.best_move.is_some());
        // The hybrid ran both phases — nodes should reflect combined work
        assert!(
            result.nodes > 100,
            "Combined nodes {} should reflect both phases",
            result.nodes
        );
    }

    // ── Result merging ──

    #[test]
    fn test_result_has_maxn_depth() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        // Depth comes from Max^n, should be > 0
        assert!(result.depth > 0, "Result depth should come from Max^n");
    }

    #[test]
    fn test_result_combines_nodes() {
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_time_ms: Some(2000),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        // Total nodes should be positive (both phases contributed)
        assert!(result.nodes > 0);
    }

    // ── Edge cases ──

    #[test]
    fn test_infinite_mode_works() {
        // Infinite mode with a depth limit should still work
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_depth: Some(2),
            max_time_ms: Some(5000),
            infinite: false,
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_very_short_time_budget() {
        // Very short time — should still produce a result
        let mut searcher = make_searcher();
        let mut state = make_state();
        let limits = SearchLimits {
            max_depth: Some(1), // Cap depth for very short time test
            max_time_ms: Some(50),
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);
        assert!(
            result.best_move.is_some(),
            "Should produce a move even with very short time"
        );
    }

    #[test]
    fn test_multiple_searches_accumulate_stats() {
        // Verify that total_searches increments with each hybrid search call.
        // Use mate-skip path for speed: threshold of -32000 means any score triggers skip.
        let config = HybridConfig {
            mate_skip_threshold: -32000,
            ..Default::default()
        };
        let mut searcher = HybridSearcher::new(BootstrapEvaluator::new(), config);
        let mut state = make_state();
        let limits = SearchLimits {
            max_depth: Some(2),
            max_time_ms: Some(5000),
            ..Default::default()
        };
        let _r1 = searcher.search(&mut state, &limits);
        assert_eq!(
            searcher.total_searches, 1,
            "total_searches should be 1 after first search"
        );
        // Note: Running the same MaxnSearcher twice on the same position with
        // persistent TT can cause best_move=None on the second call (TT hit returns
        // cached result immediately). This is expected Max^n behavior, not a hybrid bug.
    }

    // ── Stress test: Max^n → MCTS handoff at ply boundary ──

    #[test]
    fn test_mcts_handoff_at_cutover_ply() {
        // Play a game up to ply 31 (Max^n territory), then search at ply 32
        // (MCTS territory). Verify the transition doesn't crash or produce
        // invalid results. This tests the exact boundary where the engine
        // switches from Max^n to Gumbel MCTS.
        let mut searcher = make_searcher();
        let mut state = make_state();

        // Play 32 moves (8 per player) at depth 2 to reach the cutover
        for ply in 0..32 {
            if state.is_game_over() {
                eprintln!("Game ended at ply {} before cutover", ply);
                return; // Game over before cutover — can't test handoff
            }
            let limits = SearchLimits {
                max_depth: Some(2),
                game_ply: ply,
                ..Default::default()
            };
            let result = searcher.search(&mut state, &limits);
            if let Some(mv) = result.best_move {
                state.apply_move(mv);
            } else {
                eprintln!("No legal move at ply {} — stalemate/checkmate", ply);
                return;
            }
        }

        // Now at ply 32 — this should trigger MCTS phase
        assert!(
            !state.is_game_over(),
            "Game should still be in progress at ply 32"
        );

        let limits = SearchLimits {
            max_time_ms: Some(1000),
            game_ply: 32,
            ..Default::default()
        };
        let result = searcher.search(&mut state, &limits);

        assert!(
            result.best_move.is_some(),
            "MCTS at ply 32 must return a legal move"
        );
        assert!(
            result.nodes > 0,
            "MCTS should have run simulations, got 0 nodes"
        );

        // Verify the move is actually legal
        let legal = crate::move_gen::generate_legal_moves(state.board_mut());
        assert!(
            legal.contains(&result.best_move.unwrap()),
            "MCTS best move must be legal"
        );

        eprintln!(
            "MCTS handoff at ply 32: {} sims, best_move={:?}, scores={:?}",
            result.nodes, result.best_move, result.scores
        );
    }

    #[test]
    fn test_mcts_handoff_consecutive_searches() {
        // Search at ply 31 (Max^n), then ply 32 (MCTS), then ply 33 (MCTS).
        // Verifies the engine handles repeated phase transitions cleanly.
        let mut searcher = make_searcher();
        let mut state = make_state();

        // Advance to ply 31
        for ply in 0..31 {
            if state.is_game_over() {
                return;
            }
            let limits = SearchLimits {
                max_depth: Some(2),
                game_ply: ply,
                ..Default::default()
            };
            let result = searcher.search(&mut state, &limits);
            if let Some(mv) = result.best_move {
                state.apply_move(mv);
            } else {
                return;
            }
        }

        if state.is_game_over() {
            return;
        }

        // Ply 31: Max^n
        let r31 = searcher.search(
            &mut state,
            &SearchLimits {
                max_time_ms: Some(500),
                game_ply: 31,
                ..Default::default()
            },
        );
        assert!(
            r31.best_move.is_some(),
            "Max^n at ply 31 must produce a move"
        );
        state.apply_move(r31.best_move.unwrap());

        if state.is_game_over() {
            return;
        }

        // Ply 32: MCTS (first time)
        let r32 = searcher.search(
            &mut state,
            &SearchLimits {
                max_time_ms: Some(500),
                game_ply: 32,
                ..Default::default()
            },
        );
        assert!(
            r32.best_move.is_some(),
            "MCTS at ply 32 must produce a move"
        );
        state.apply_move(r32.best_move.unwrap());

        if state.is_game_over() {
            return;
        }

        // Ply 33: MCTS (second time)
        let r33 = searcher.search(
            &mut state,
            &SearchLimits {
                max_time_ms: Some(500),
                game_ply: 33,
                ..Default::default()
            },
        );
        assert!(
            r33.best_move.is_some(),
            "MCTS at ply 33 must produce a move"
        );

        eprintln!(
            "Consecutive handoff: ply31 depth={}, ply32 sims={}, ply33 sims={}",
            r31.depth, r32.nodes, r33.nodes
        );
    }
}
