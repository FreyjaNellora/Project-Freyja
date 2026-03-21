"""Export trained PyTorch weights to .fnnue binary format.

The .fnnue format matches the Rust loader exactly:
  Header: magic + architecture_hash + layer sizes
  Body: i16 little-endian weights in order

Quantization: float32 → i16 via round(w * WEIGHT_SCALE), clamped.

Stage 16: NNUE Architecture + Training Pipeline
"""

import struct
import numpy as np
import torch

from .data import TOTAL_FEATURES, NUM_PIECE_SQ_FEATURES, HIDDEN1_SIZE, HIDDEN2_SIZE

# Must match Rust nnue::features::WEIGHT_SCALE
WEIGHT_SCALE = 64

# Magic bytes (must match Rust FNNUE_MAGIC)
FNNUE_MAGIC = b'FNNUE\x00\x01\x00'


def architecture_hash() -> int:
    """Compute architecture hash matching Rust NnueWeights::architecture_hash()."""
    h = 0x464E4E55  # "FNNU"
    h = (h * 31 + TOTAL_FEATURES) & 0xFFFFFFFF
    h = (h * 31 + HIDDEN1_SIZE) & 0xFFFFFFFF
    h = (h * 31 + HIDDEN2_SIZE) & 0xFFFFFFFF
    h = (h * 31 + 1) & 0xFFFFFFFF  # output_size
    return h


def quantize_weights(w_float: np.ndarray, scale: int = WEIGHT_SCALE) -> np.ndarray:
    """Quantize float32 weights to i16.

    Args:
        w_float: float32 weight array
        scale: quantization scale factor (default: 64 for Q6)

    Returns:
        i16 quantized weights
    """
    scaled = np.round(w_float * scale).astype(np.int32)
    clamped = np.clip(scaled, -32768, 32767)
    return clamped.astype(np.int16)


def export_fnnue(model, output_path: str):
    """Export a trained FreyjaNet model to .fnnue binary format.

    Args:
        model: trained FreyjaNet instance
        output_path: path to write the .fnnue file
    """
    state = model.state_dict()

    # Extract weights (PyTorch Linear stores weight as [out, in] and bias as [out])
    # Feature layer: [HIDDEN1_SIZE, num_features] → need [num_features, HIDDEN1_SIZE] for Rust
    feat_w = state['feature_layer.weight'].cpu().numpy()  # [256, num_features]
    feat_b = state['feature_layer.bias'].cpu().numpy()      # [256]

    # Hidden1: [HIDDEN2_SIZE, HIDDEN1_SIZE] → need [HIDDEN1_SIZE, HIDDEN2_SIZE]
    h1_w = state['hidden1.weight'].cpu().numpy()  # [32, 256]
    h1_b = state['hidden1.bias'].cpu().numpy()      # [32]

    # Hidden2: [1, HIDDEN2_SIZE] → need [HIDDEN2_SIZE]
    h2_w = state['hidden2.weight'].cpu().numpy()  # [1, 32]
    h2_b = state['hidden2.bias'].cpu().numpy()      # [1]

    num_features = feat_w.shape[1]

    # Pad feature weights to TOTAL_FEATURES if model uses fewer
    # (e.g., model uses NUM_PIECE_SQ_FEATURES=4480, format needs TOTAL_FEATURES=4488)
    if num_features < TOTAL_FEATURES:
        pad = np.zeros((HIDDEN1_SIZE, TOTAL_FEATURES - num_features), dtype=np.float32)
        feat_w = np.concatenate([feat_w, pad], axis=1)

    # Transpose to Rust layout: [TOTAL_FEATURES][HIDDEN1_SIZE]
    feat_w_rust = feat_w.T  # [TOTAL_FEATURES, HIDDEN1_SIZE]

    # Transpose hidden1 to Rust layout: [HIDDEN1_SIZE][HIDDEN2_SIZE]
    h1_w_rust = h1_w.T  # [HIDDEN1_SIZE, HIDDEN2_SIZE]

    # Hidden2 weights: flatten [1, HIDDEN2_SIZE] → [HIDDEN2_SIZE]
    h2_w_flat = h2_w.flatten()

    # Hidden2 bias: scalar
    h2_b_scalar = float(h2_b[0])

    # Quantize all weights
    q_feat_w = quantize_weights(feat_w_rust)
    q_feat_b = quantize_weights(feat_b)
    q_h1_w = quantize_weights(h1_w_rust)
    q_h1_b = quantize_weights(h1_b)
    q_h2_w = quantize_weights(h2_w_flat)
    q_h2_b = quantize_weights(np.array([h2_b_scalar]))

    # Write binary file
    with open(output_path, 'wb') as f:
        # Header
        f.write(FNNUE_MAGIC)
        f.write(struct.pack('<I', architecture_hash()))
        f.write(struct.pack('<I', TOTAL_FEATURES))
        f.write(struct.pack('<I', HIDDEN1_SIZE))
        f.write(struct.pack('<I', HIDDEN2_SIZE))
        f.write(struct.pack('<I', 1))  # output_size

        # Feature weights: [TOTAL_FEATURES][HIDDEN1_SIZE] as flat i16
        f.write(q_feat_w.tobytes())
        # Feature biases
        f.write(q_feat_b.tobytes())
        # Hidden1 weights: [HIDDEN1_SIZE][HIDDEN2_SIZE]
        f.write(q_h1_w.tobytes())
        # Hidden1 biases
        f.write(q_h1_b.tobytes())
        # Hidden2 weights
        f.write(q_h2_w.tobytes())
        # Hidden2 bias
        f.write(q_h2_b.tobytes())

    # Verify file size
    expected_size = (
        8 +  # magic
        4 +  # arch hash
        4 * 4 +  # 4 u32 layer sizes
        TOTAL_FEATURES * HIDDEN1_SIZE * 2 +  # feature weights
        HIDDEN1_SIZE * 2 +  # feature biases
        HIDDEN1_SIZE * HIDDEN2_SIZE * 2 +  # hidden1 weights
        HIDDEN2_SIZE * 2 +  # hidden1 biases
        HIDDEN2_SIZE * 2 +  # hidden2 weights
        2  # hidden2 bias
    )
    import os
    actual_size = os.path.getsize(output_path)
    assert actual_size == expected_size, \
        f"File size mismatch: got {actual_size}, expected {expected_size}"


def load_quantized_model(fnnue_path: str):
    """Load quantized weights from a .fnnue file for verification.

    Returns dict of numpy arrays matching the Rust weight layout.
    """
    with open(fnnue_path, 'rb') as f:
        magic = f.read(8)
        assert magic == FNNUE_MAGIC, f"Invalid magic: {magic!r}"

        arch_hash = struct.unpack('<I', f.read(4))[0]
        assert arch_hash == architecture_hash(), \
            f"Architecture hash mismatch: {arch_hash:#x} vs {architecture_hash():#x}"

        num_feat = struct.unpack('<I', f.read(4))[0]
        h1_size = struct.unpack('<I', f.read(4))[0]
        h2_size = struct.unpack('<I', f.read(4))[0]
        out_size = struct.unpack('<I', f.read(4))[0]

        assert num_feat == TOTAL_FEATURES
        assert h1_size == HIDDEN1_SIZE
        assert h2_size == HIDDEN2_SIZE
        assert out_size == 1

        # Read weights as i16 arrays
        feat_w = np.frombuffer(f.read(TOTAL_FEATURES * HIDDEN1_SIZE * 2), dtype='<i2')
        feat_w = feat_w.reshape(TOTAL_FEATURES, HIDDEN1_SIZE)

        feat_b = np.frombuffer(f.read(HIDDEN1_SIZE * 2), dtype='<i2')

        h1_w = np.frombuffer(f.read(HIDDEN1_SIZE * HIDDEN2_SIZE * 2), dtype='<i2')
        h1_w = h1_w.reshape(HIDDEN1_SIZE, HIDDEN2_SIZE)

        h1_b = np.frombuffer(f.read(HIDDEN2_SIZE * 2), dtype='<i2')

        h2_w = np.frombuffer(f.read(HIDDEN2_SIZE * 2), dtype='<i2')

        h2_b = np.frombuffer(f.read(2), dtype='<i2')

    return {
        'feature_weights': feat_w,
        'feature_biases': feat_b,
        'hidden1_weights': h1_w,
        'hidden1_biases': h1_b,
        'hidden2_weights': h2_w,
        'hidden2_bias': h2_b[0],
    }
