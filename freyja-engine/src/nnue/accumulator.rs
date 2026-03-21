//! NNUE accumulator: the first hidden layer computed from active features.
//!
//! The accumulator sums the weight rows for all active piece-square features
//! plus the bias vector. This is the most expensive part of NNUE evaluation,
//! but with ~50 active features it's manageable in scalar code.
//!
//! Stage 16: Full refresh on every eval call.
//! Stage 17/20: Incremental update via make/unmake.
//!
//! ADR-012: `#[repr(C, align(32))]` for future AVX2 SIMD.

use super::features::{self, HIDDEN1_SIZE, TOTAL_FEATURES};
use super::weights::NnueWeights;
use crate::board::Board;
use crate::board::types::Player;

// ─── Accumulator Struct ────────────────────────────────────────────────────

/// Per-perspective accumulator (first hidden layer values).
///
/// Aligned to 32 bytes for future AVX2 operations (ADR-012).
#[derive(Clone)]
#[repr(C, align(32))]
pub struct Accumulator {
    /// Current i16 values for this perspective's hidden layer.
    pub values: [i16; HIDDEN1_SIZE],
}

impl Accumulator {
    /// Create a zeroed accumulator.
    pub fn new() -> Self {
        Self {
            values: [0i16; HIDDEN1_SIZE],
        }
    }

    /// Refresh the accumulator from scratch for a given perspective.
    ///
    /// This computes: bias + sum(feature_weight[i] for each active feature i).
    /// Called on every eval in Stage 16 (full recompute, no incremental updates).
    pub fn refresh(&mut self, board: &Board, perspective: Player, weights: &NnueWeights) {
        // Start with bias
        self.values.copy_from_slice(&weights.feature_biases);

        // Add weight rows for each active piece feature
        for player in Player::all() {
            for (piece_type, sq) in board.pieces(player) {
                if let Some(idx) = features::feature_index(perspective, sq, piece_type, player) {
                    self.add_weight_row(idx as usize, weights);
                }
            }
        }
    }

    /// Add a feature's weight row to the accumulator.
    #[inline]
    fn add_weight_row(&mut self, feature_idx: usize, weights: &NnueWeights) {
        debug_assert!(feature_idx < TOTAL_FEATURES);
        let row = &weights.feature_weights[feature_idx];
        for i in 0..HIDDEN1_SIZE {
            self.values[i] = self.values[i].saturating_add(row[i]);
        }
    }

    /// Subtract a feature's weight row from the accumulator (for incremental update).
    #[inline]
    #[allow(dead_code)] // Used in Stage 17+ incremental updates
    fn sub_weight_row(&mut self, feature_idx: usize, weights: &NnueWeights) {
        debug_assert!(feature_idx < TOTAL_FEATURES);
        let row = &weights.feature_weights[feature_idx];
        for i in 0..HIDDEN1_SIZE {
            self.values[i] = self.values[i].saturating_sub(row[i]);
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::weights::NnueWeights;

    #[test]
    fn test_accumulator_alignment() {
        assert_eq!(std::mem::align_of::<Accumulator>(), 32);
    }

    #[test]
    fn test_accumulator_size() {
        // 256 × 2 bytes = 512 bytes, padded to align(32) boundary
        assert!(std::mem::size_of::<Accumulator>() >= 512);
    }

    #[test]
    fn test_refresh_deterministic() {
        let board = Board::starting_position();
        let weights = NnueWeights::random(42);

        let mut acc1 = Accumulator::new();
        let mut acc2 = Accumulator::new();
        acc1.refresh(&board, Player::Red, &weights);
        acc2.refresh(&board, Player::Red, &weights);

        assert_eq!(acc1.values, acc2.values, "Refresh should be deterministic");
    }

    #[test]
    fn test_refresh_different_perspectives() {
        let board = Board::starting_position();
        let weights = NnueWeights::random(42);

        let mut acc_red = Accumulator::new();
        let mut acc_blue = Accumulator::new();
        acc_red.refresh(&board, Player::Red, &weights);
        acc_blue.refresh(&board, Player::Blue, &weights);

        // Different perspectives should (almost certainly) produce different values
        // with random weights
        assert_ne!(
            acc_red.values, acc_blue.values,
            "Different perspectives should produce different accumulators"
        );
    }

    #[test]
    fn test_refresh_with_zero_weights_equals_bias() {
        let board = Board::starting_position();
        let mut weights = NnueWeights::zeros();
        // Set a distinctive bias pattern
        for (i, b) in weights.feature_biases.iter_mut().enumerate() {
            *b = (i as i16) * 3 - 100;
        }

        let mut acc = Accumulator::new();
        acc.refresh(&board, Player::Red, &weights);

        // With zero feature weights, result should be bias only
        assert_eq!(acc.values, weights.feature_biases);
    }
}
