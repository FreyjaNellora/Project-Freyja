"""Round-trip verification: compare quantized Python inference vs Rust inference.

Usage:
    python -m freyja_nnue.verify --weights weights.fnnue --test-fen4 "..."

This script loads a .fnnue file, runs the quantized forward pass in Python
(matching Rust's i16/i32 arithmetic exactly), and prints the output scores.
Compare these against the Rust engine's output for the same position.

Stage 16: NNUE Architecture + Training Pipeline
"""

import argparse
import numpy as np

from .data import extract_all_perspectives, NUM_PIECE_SQ_FEATURES, TOTAL_FEATURES
from .export import load_quantized_model, WEIGHT_SCALE


def quantized_forward_single(accumulator, weights):
    """Quantized forward pass for one perspective, matching Rust exactly.

    All arithmetic is i16/i32, matching nnue/forward.rs::forward_single.
    """
    h1_weights = weights['hidden1_weights']  # [256, 32]
    h1_biases = weights['hidden1_biases']    # [32]
    h2_weights = weights['hidden2_weights']  # [32]
    h2_bias = int(weights['hidden2_bias'])

    clipped_relu_max = 127 * WEIGHT_SCALE  # 8128

    # Layer 1: accumulator → ClippedReLU → hidden1
    hidden1 = np.zeros(32, dtype=np.int32)
    for j in range(32):
        s = int(h1_biases[j])
        for i in range(256):
            clamped = max(0, min(int(accumulator[i]), clipped_relu_max))
            s += clamped * int(h1_weights[i, j])
        hidden1[j] = s

    # Layer 2: hidden1 → ReLU → output
    output = h2_bias
    for j in range(32):
        activated = max(0, int(hidden1[j]))
        scaled = activated // WEIGHT_SCALE
        output += scaled * int(h2_weights[j])

    return output


def quantized_eval(fen4, weights):
    """Full quantized evaluation matching Rust NnueEvaluator::eval_4vec.

    Returns [i16; 4] scores in centipawns.
    """
    # The hidden layer already divides by WEIGHT_SCALE once,
    # so remaining scale is just WEIGHT_SCALE, not WEIGHT_SCALE².
    output_scale = WEIGHT_SCALE  # 64

    feat_weights = weights['feature_weights']  # [4488, 256]
    feat_biases = weights['feature_biases']    # [256]

    scores = []
    for perspective in range(4):
        # Refresh accumulator
        accumulator = np.array(feat_biases, dtype=np.int16).copy()
        active_features = extract_all_perspectives(fen4)[perspective]

        for feat_idx in active_features:
            row = feat_weights[feat_idx]
            for i in range(256):
                # Saturating add (match Rust behavior)
                val = int(accumulator[i]) + int(row[i])
                accumulator[i] = max(-32768, min(32767, val))

        # Forward pass
        raw = quantized_forward_single(accumulator, weights)
        score = raw // output_scale
        scores.append(int(np.clip(score, -32768, 32767)))

    # Zero-center
    mean = sum(scores) // 4
    scores = [s - mean for s in scores]

    return scores


def main():
    parser = argparse.ArgumentParser(description='Verify NNUE round-trip')
    parser.add_argument('--weights', required=True, help='Path to .fnnue file')
    parser.add_argument('--fen4', required=True, help='FEN4 position string')
    args = parser.parse_args()

    print(f"Loading weights from: {args.weights}")
    weights = load_quantized_model(args.weights)

    print(f"Position: {args.fen4[:80]}...")
    scores = quantized_eval(args.fen4, weights)

    print(f"\nQuantized Python eval:")
    print(f"  Red:    {scores[0]}")
    print(f"  Blue:   {scores[1]}")
    print(f"  Yellow: {scores[2]}")
    print(f"  Green:  {scores[3]}")
    print(f"  Sum:    {sum(scores)}")
    print(f"\nCompare these values against 'go depth 1' with EvalMode=nnue in the engine.")


if __name__ == '__main__':
    main()
