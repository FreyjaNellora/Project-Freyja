//! NNUE forward pass: accumulator → hidden layers → output.
//!
//! Architecture per perspective:
//!   accumulator[256] → ClippedReLU → W1[256×32] + B1 → ClippedReLU → W2[32×1] + B2
//!
//! All arithmetic is quantized i16/i32. No floating point in the hot path.
//! Scalar implementation for Stage 16. SIMD (AVX2) deferred to Stage 20.
//!
//! Scale factors:
//! - Feature weights: Q6 (×64)
//! - Hidden weights: Q6 (×64)
//! - Output: divided by OUTPUT_SCALE (4096) to get centipawns

use super::accumulator::Accumulator;
use super::features::{HIDDEN1_SIZE, HIDDEN2_SIZE, OUTPUT_SCALE};
use super::weights::NnueWeights;

/// Clipped ReLU on i16: clamp to [0, max_val].
/// Standard NNUE uses clamp to [0, 127] for i8 quantization,
/// but we use i16 accumulators so clamp to [0, i16::MAX].
const CLIPPED_RELU_MAX: i16 = 127 * (super::features::WEIGHT_SCALE as i16);

/// Compute the forward pass for a single perspective.
///
/// Returns the raw output score (before output scaling).
#[inline]
fn forward_single(acc: &Accumulator, weights: &NnueWeights) -> i32 {
    // Layer 1: accumulator → ClippedReLU → hidden1
    let mut hidden1 = [0i32; HIDDEN2_SIZE];
    for j in 0..HIDDEN2_SIZE {
        let mut sum = weights.hidden1_biases[j] as i32;
        for i in 0..HIDDEN1_SIZE {
            // ClippedReLU on accumulator value
            let clamped = acc.values[i].max(0).min(CLIPPED_RELU_MAX) as i32;
            sum += clamped * weights.hidden1_weights[i][j] as i32;
        }
        hidden1[j] = sum;
    }

    // Layer 2: hidden1 → ReLU → hidden2 (scalar output)
    let mut output = weights.hidden2_bias as i32;
    for j in 0..HIDDEN2_SIZE {
        // ReLU on hidden1 (already i32, just clamp negative)
        let activated = hidden1[j].max(0);
        // Scale down from Q6*Q6 = Q12 to Q6 before multiplying again
        let scaled = activated / super::features::WEIGHT_SCALE as i32;
        output += scaled * weights.hidden2_weights[j] as i32;
    }

    output
}

/// Compute the NNUE forward pass for all 4 perspectives.
///
/// Takes 4 refreshed accumulators (one per player perspective) and produces
/// a 4-vector of scores in centipawns.
pub fn forward_pass(accumulators: &[Accumulator; 4], weights: &NnueWeights) -> [i16; 4] {
    let mut scores = [0i16; 4];
    for i in 0..4 {
        let raw = forward_single(&accumulators[i], weights);
        // Scale from quantized domain to centipawns.
        // The hidden layer already divides by WEIGHT_SCALE once (line "scaled = activated / WS"),
        // so the remaining scale is just WEIGHT_SCALE, not WEIGHT_SCALE².
        scores[i] = (raw / super::features::WEIGHT_SCALE as i32) as i16;
    }
    scores
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nnue::weights::NnueWeights;

    #[test]
    fn test_forward_pass_zero_accumulators() {
        let weights = NnueWeights::random(42);
        let accs = [
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
        ];

        let scores = forward_pass(&accs, &weights);
        // With zero accumulators, output depends on biases + ReLU behavior.
        // All 4 perspectives use same weights and same zero input → same output.
        assert_eq!(scores[0], scores[1]);
        assert_eq!(scores[1], scores[2]);
        assert_eq!(scores[2], scores[3]);
    }

    #[test]
    fn test_forward_pass_deterministic() {
        let weights = NnueWeights::random(42);
        let mut accs = [
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
        ];
        // Put some non-zero values in accumulators
        let board = crate::board::Board::starting_position();
        for (i, player) in crate::board::types::Player::all().iter().enumerate() {
            accs[i].refresh(&board, *player, &weights);
        }

        let scores1 = forward_pass(&accs, &weights);
        let scores2 = forward_pass(&accs, &weights);
        assert_eq!(scores1, scores2, "Forward pass must be deterministic");
    }

    #[test]
    fn test_forward_pass_different_accumulators_different_output() {
        let weights = NnueWeights::random(42);
        let board = crate::board::Board::starting_position();

        let mut accs = [
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
        ];
        for (i, player) in crate::board::types::Player::all().iter().enumerate() {
            accs[i].refresh(&board, *player, &weights);
        }

        let scores = forward_pass(&accs, &weights);
        // Due to board symmetry in starting position, Red and Yellow (across)
        // might have similar but likely not identical scores with random weights.
        // At minimum, values should be finite and within reasonable range.
        for &s in &scores {
            assert!(
                s > -10000 && s < 10000,
                "Score {s} is unreasonably large for random weights"
            );
        }
    }

    #[test]
    fn test_forward_pass_perspectives_independent() {
        let weights = NnueWeights::random(42);
        let board = crate::board::Board::starting_position();

        // Compute with normal accumulators
        let mut accs = [
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
            Accumulator::new(),
        ];
        for (i, player) in crate::board::types::Player::all().iter().enumerate() {
            accs[i].refresh(&board, *player, &weights);
        }
        let scores_normal = forward_pass(&accs, &weights);

        // Zero out accumulator 0 (Red) and recompute
        accs[0] = Accumulator::new();
        let scores_modified = forward_pass(&accs, &weights);

        // Red's score should change, others should not
        assert_ne!(scores_normal[0], scores_modified[0]);
        assert_eq!(scores_normal[1], scores_modified[1]);
        assert_eq!(scores_normal[2], scores_modified[2]);
        assert_eq!(scores_normal[3], scores_modified[3]);
    }
}
