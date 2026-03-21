//! NNUE (Efficiently Updatable Neural Network) for four-player chess.
//!
//! Architecture: 4-perspective network with shared weights.
//!   Input:  4488 features per perspective (4480 piece-square + 8 zone control)
//!   Hidden: 256 → 32 → 1 (per perspective)
//!   Output: [i16; 4] score vector (one per player)
//!
//! Stage 16: NNUE Architecture + Training Pipeline
//! Stage 17: Integration as default evaluator (swap with bootstrap)
//! Stage 20: SIMD optimization (AVX2 forward pass)

pub mod accumulator;
pub mod features;
pub mod forward;
pub mod weights;

use std::sync::Arc;

use crate::board::types::Player;
use crate::eval::{self, ELIMINATED_SCORE, Evaluator};
use crate::game_state::{GameState, PlayerStatus};

use accumulator::Accumulator;
use features::{HIDDEN1_SIZE, NUM_ZONE_FEATURES, WEIGHT_SCALE};
use weights::NnueWeights;

// ─── NnueEvaluator ────────────────────────────────────────────────────────

/// NNUE-based position evaluator for four-player chess.
///
/// Implements the `Evaluator` trait as a drop-in replacement for `BootstrapEvaluator`.
/// Weights are shared via `Arc` for cheap cloning (search clones evaluators frequently).
pub struct NnueEvaluator {
    weights: Arc<NnueWeights>,
}

impl NnueEvaluator {
    /// Create an evaluator from loaded weights.
    pub fn new(weights: Arc<NnueWeights>) -> Self {
        Self { weights }
    }

    /// Create an evaluator with random weights (for testing).
    pub fn with_random_weights(seed: u64) -> Self {
        Self {
            weights: Arc::new(NnueWeights::random(seed)),
        }
    }

    /// Create an evaluator from a .fnnue file.
    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let weights = NnueWeights::from_file(path)?;
        Ok(Self {
            weights: Arc::new(weights),
        })
    }

    /// Extract 8 zone control summary features for a given perspective.
    ///
    /// Calls the zone control functions from eval.rs directly (made pub for this purpose).
    /// Returns quantized i16 values scaled by WEIGHT_SCALE for the accumulator.
    fn zone_features(&self, state: &GameState, perspective: Player) -> [i16; NUM_ZONE_FEATURES] {
        let pi = perspective.index();

        // Compute zone features (these are the same functions used by BootstrapEvaluator)
        let territory = eval::bfs_territory_enhanced(state);
        let influence = eval::compute_influence(state);
        let tension = eval::compute_tension(state, &influence);
        let swarm = eval::compute_swarm(state, &influence);

        // Pack 8 summary features, scaled to Q6 range
        [
            territory.counts[pi],          // territory count
            territory.frontier[pi],        // frontier length
            influence.net[pi] as i16,      // net influence (float → i16)
            tension[pi],                   // tension score
            swarm.defended_pieces[pi],     // defended pieces
            swarm.undefended_pieces[pi],   // undefended pieces
            swarm.coordinated_squares[pi], // coordinated squares
            swarm.pawn_chain[pi],          // pawn chain count
        ]
    }

    /// Add zone features to an accumulator.
    fn apply_zone_features(&self, acc: &mut Accumulator, zone_feats: &[i16; NUM_ZONE_FEATURES]) {
        for (z_idx, &z_val) in zone_feats.iter().enumerate() {
            let feat_idx = features::zone_feature_index(z_idx) as usize;
            let row = &self.weights.feature_weights[feat_idx];
            // Scale zone value and add weighted contribution
            for i in 0..HIDDEN1_SIZE {
                // zone_val * weight_row[i] / WEIGHT_SCALE to keep in Q6 domain
                let contribution = (z_val as i32 * row[i] as i32) / WEIGHT_SCALE as i32;
                acc.values[i] = acc.values[i].saturating_add(contribution as i16);
            }
        }
    }
}

impl Clone for NnueEvaluator {
    fn clone(&self) -> Self {
        Self {
            weights: Arc::clone(&self.weights),
        }
    }
}

impl Evaluator for NnueEvaluator {
    fn eval_scalar(&self, state: &GameState, player: Player) -> i16 {
        self.eval_4vec(state)[player.index()]
    }

    fn eval_4vec(&self, state: &GameState) -> [i16; 4] {
        let mut accumulators = [
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
        ];

        // Refresh accumulators for each perspective
        for player in Player::all() {
            let pi = player.index();
            if matches!(state.player_status(player), PlayerStatus::Eliminated) {
                // Skip eliminated players — they get ELIMINATED_SCORE
                continue;
            }
            accumulators[pi].refresh(state.board(), player, &self.weights);

            // Add zone control features
            let zone_feats = self.zone_features(state, player);
            self.apply_zone_features(&mut accumulators[pi], &zone_feats);
        }

        // Forward pass
        let mut scores = forward::forward_pass(&accumulators, &self.weights);

        // Eliminated players get sentinel score
        for player in Player::all() {
            if matches!(state.player_status(player), PlayerStatus::Eliminated) {
                scores[player.index()] = ELIMINATED_SCORE;
            }
        }

        // Zero-center active player scores (same as bootstrap eval)
        let active_count = Player::all()
            .iter()
            .filter(|&&p| !matches!(state.player_status(p), PlayerStatus::Eliminated))
            .count();

        if active_count > 1 {
            let sum: i32 = Player::all()
                .iter()
                .filter(|&&p| !matches!(state.player_status(p), PlayerStatus::Eliminated))
                .map(|&p| scores[p.index()] as i32)
                .sum();
            let mean = (sum / active_count as i32) as i16;
            for player in Player::all() {
                if !matches!(state.player_status(player), PlayerStatus::Eliminated) {
                    scores[player.index()] = scores[player.index()].saturating_sub(mean);
                }
            }
        }

        scores
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::game_state::GameState;

    #[test]
    fn test_nnue_evaluator_implements_evaluator_trait() {
        let eval: Box<dyn Evaluator> = Box::new(NnueEvaluator::with_random_weights(42));
        let state = GameState::new(Board::starting_position());
        let scores = eval.eval_4vec(&state);
        // Should produce values for all 4 players
        for &s in &scores {
            assert_ne!(s, ELIMINATED_SCORE);
        }
    }

    #[test]
    fn test_nnue_evaluator_deterministic() {
        let eval = NnueEvaluator::with_random_weights(42);
        let state = GameState::new(Board::starting_position());
        let scores1 = eval.eval_4vec(&state);
        let scores2 = eval.eval_4vec(&state);
        assert_eq!(scores1, scores2);
    }

    #[test]
    fn test_nnue_evaluator_clone_independent() {
        let eval1 = NnueEvaluator::with_random_weights(42);
        let eval2 = eval1.clone();
        let state = GameState::new(Board::starting_position());
        // Both should produce identical results (shared weights)
        assert_eq!(eval1.eval_4vec(&state), eval2.eval_4vec(&state));
    }

    #[test]
    fn test_nnue_evaluator_scalar_matches_4vec() {
        let eval = NnueEvaluator::with_random_weights(42);
        let state = GameState::new(Board::starting_position());
        let vec = eval.eval_4vec(&state);
        for player in Player::all() {
            assert_eq!(
                eval.eval_scalar(&state, player),
                vec[player.index()],
                "eval_scalar should match eval_4vec component for {player}"
            );
        }
    }

    #[test]
    fn test_nnue_evaluator_zero_centered() {
        let eval = NnueEvaluator::with_random_weights(42);
        let state = GameState::new(Board::starting_position());
        let scores = eval.eval_4vec(&state);
        // Sum of all 4 scores should be near 0 (zero-centered)
        let sum: i32 = scores.iter().map(|&s| s as i32).sum();
        assert!(
            sum.abs() < 4,
            "Scores should be zero-centered, sum={sum}, scores={scores:?}"
        );
    }

    #[test]
    fn test_nnue_evaluator_reasonable_range() {
        let eval = NnueEvaluator::with_random_weights(42);
        let state = GameState::new(Board::starting_position());
        let scores = eval.eval_4vec(&state);
        for &s in &scores {
            assert!(
                s > -10000 && s < 10000,
                "Score {s} out of reasonable range for random weights"
            );
        }
    }

    #[test]
    fn test_nnue_different_seeds_different_output() {
        let eval1 = NnueEvaluator::with_random_weights(42);
        let eval2 = NnueEvaluator::with_random_weights(99);
        let state = GameState::new(Board::starting_position());
        assert_ne!(
            eval1.eval_4vec(&state),
            eval2.eval_4vec(&state),
            "Different weight seeds should produce different outputs"
        );
    }
}
