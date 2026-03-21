"""Freyja NNUE PyTorch network definition.

Mirrors the Rust inference architecture exactly:
  Input:  4488 features per perspective (4480 piece-square + 8 zone)
  Hidden: 256 → 32 → 1 (per perspective, shared weights)
  Output: 4-vector of scores

All 4 perspectives share the same network weights.
The perspective-relative feature encoding makes each perspective
see the board from their own viewpoint.

Stage 16: NNUE Architecture + Training Pipeline
"""

import torch
import torch.nn as nn

from .data import NUM_PIECE_SQ_FEATURES, HIDDEN1_SIZE, HIDDEN2_SIZE


class FreyjaNet(nn.Module):
    """NNUE network for four-player chess.

    Architecture matches Rust inference exactly:
    - feature_layer: Linear(4480, 256) with ClippedReLU
    - hidden1: Linear(256, 32) with ClippedReLU
    - hidden2: Linear(32, 1)

    Zone features (8 per perspective) are deferred to later training runs.
    Initial training uses piece-square features only (4480).
    """

    def __init__(self, num_features=NUM_PIECE_SQ_FEATURES,
                 hidden1=HIDDEN1_SIZE, hidden2=HIDDEN2_SIZE):
        super().__init__()
        self.num_features = num_features
        self.feature_layer = nn.Linear(num_features, hidden1)
        self.hidden1 = nn.Linear(hidden1, hidden2)
        self.hidden2 = nn.Linear(hidden2, 1)

    def forward_perspective(self, features):
        """Forward pass for one perspective.

        Args:
            features: [batch, num_features] dense binary feature vector

        Returns:
            [batch, 1] raw score
        """
        x = torch.clamp(self.feature_layer(features), min=0.0)  # ReLU
        x = torch.clamp(self.hidden1(x), min=0.0)  # ReLU
        x = self.hidden2(x)
        return x

    def forward(self, features_4):
        """Forward pass for all 4 perspectives.

        Args:
            features_4: [batch, 4, num_features] feature vectors

        Returns:
            [batch, 4] score vector
        """
        outputs = []
        for i in range(4):
            out = self.forward_perspective(features_4[:, i])
            outputs.append(out.squeeze(-1))
        return torch.stack(outputs, dim=1)


def count_parameters(model):
    """Count total trainable parameters."""
    return sum(p.numel() for p in model.parameters() if p.requires_grad)
