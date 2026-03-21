"""Training data loader for Freyja NNUE.

Reads JSONL training records from the observer extract pipeline,
parses FEN4 positions, and converts to feature indices matching
the Rust NNUE architecture exactly.

Stage 16: NNUE Architecture + Training Pipeline
"""

import json
import numpy as np
import torch
from torch.utils.data import Dataset
from pathlib import Path

# ─── Board Constants (must match Rust board/types.rs) ──────────────────────

BOARD_SIZE = 14
TOTAL_SQUARES = 196
VALID_SQUARES = 160
CORNER_SIZE = 3

# ─── Architecture Constants (must match Rust nnue/features.rs) ─────────────

NUM_VALID_SQUARES = 160
NUM_PIECE_TYPES = 7
NUM_RELATIVE_PLAYERS = 4
NUM_PIECE_SQ_FEATURES = NUM_RELATIVE_PLAYERS * NUM_PIECE_TYPES * NUM_VALID_SQUARES  # 4480
NUM_ZONE_FEATURES = 8
TOTAL_FEATURES = NUM_PIECE_SQ_FEATURES + NUM_ZONE_FEATURES  # 4488
HIDDEN1_SIZE = 256
HIDDEN2_SIZE = 32

# ─── Piece Type Mapping ────────────────────────────────────────────────────

PIECE_TYPE_INDEX = {
    'P': 0,  # Pawn
    'N': 1,  # Knight
    'B': 2,  # Bishop
    'R': 3,  # Rook
    'Q': 4,  # Queen
    'K': 5,  # King
    # PromotedQueen (type 6) is represented as 'Q' in FEN4 — we can't
    # distinguish from Queen in the FEN string. Both map to index 4.
    # This is a known limitation; the training data only shows 'Q'.
}

PLAYER_INDEX = {
    'r': 0,  # Red
    'b': 1,  # Blue
    'y': 2,  # Yellow
    'g': 3,  # Green
}

# ─── Square Validity (must match Rust) ─────────────────────────────────────

def is_valid_square(rank: int, file: int) -> bool:
    """Check if a (rank, file) coordinate is a valid playable square."""
    if rank < 0 or rank >= BOARD_SIZE or file < 0 or file >= BOARD_SIZE:
        return False
    in_corner = (
        (rank > BOARD_SIZE - CORNER_SIZE - 1 or rank < CORNER_SIZE) and
        (file > BOARD_SIZE - CORNER_SIZE - 1 or file < CORNER_SIZE)
    )
    return not in_corner


# Precompute square-to-valid-index mapping (must match Rust SQUARE_TO_VALID_INDEX)
SQUARE_TO_VALID_INDEX = [255] * TOTAL_SQUARES
VALID_INDEX_TO_SQUARE = [0] * NUM_VALID_SQUARES
_valid_idx = 0
for _i in range(TOTAL_SQUARES):
    _rank = _i // BOARD_SIZE
    _file = _i % BOARD_SIZE
    if is_valid_square(_rank, _file):
        SQUARE_TO_VALID_INDEX[_i] = _valid_idx
        VALID_INDEX_TO_SQUARE[_valid_idx] = _i
        _valid_idx += 1
assert _valid_idx == NUM_VALID_SQUARES, f"Expected {NUM_VALID_SQUARES} valid squares, got {_valid_idx}"


# ─── Feature Index Computation (must match Rust exactly) ───────────────────

def relative_player(perspective: int, piece_player: int) -> int:
    """Compute relative player index (0=self, 1=next, 2=across, 3=prev)."""
    return (piece_player + 4 - perspective) % 4


def feature_index(perspective: int, sq_index: int, piece_type_idx: int, piece_player: int) -> int:
    """Compute piece-square feature index.

    Must match Rust nnue::features::feature_index exactly.
    Returns -1 if the square is invalid.
    """
    valid_idx = SQUARE_TO_VALID_INDEX[sq_index]
    if valid_idx == 255:
        return -1

    rel_player = relative_player(perspective, piece_player)
    idx = rel_player * (NUM_PIECE_TYPES * NUM_VALID_SQUARES) + \
          piece_type_idx * NUM_VALID_SQUARES + \
          valid_idx

    assert idx < NUM_PIECE_SQ_FEATURES, f"Feature index {idx} out of bounds"
    return idx


# ─── FEN4 Parser ───────────────────────────────────────────────────────────

def parse_fen4(fen4_str: str):
    """Parse a FEN4 string and extract pieces.

    FEN4 format: ranks / active_player castling ep_square ep_player
    Ranks are separated by '/', from rank 14 (top) to rank 1 (bottom).

    Piece codes: lowercase player prefix + uppercase piece type
    e.g., rP = Red Pawn, bK = Blue King, yQ = Yellow Queen, gN = Green Knight
    'xxx' = invalid corner square, digits = empty squares.

    Returns: list of (square_index, piece_type_idx, player_idx) tuples.
    """
    parts = fen4_str.strip().split()
    rank_str = parts[0]
    ranks = rank_str.split('/')

    pieces = []

    for rank_idx_from_top, rank_data in enumerate(ranks):
        # FEN4 goes from rank 14 (index 13) at top to rank 1 (index 0) at bottom
        rank = BOARD_SIZE - 1 - rank_idx_from_top
        file = 0

        i = 0
        while i < len(rank_data):
            ch = rank_data[i]

            if ch == 'x':
                # Skip invalid corner markers (xxx = 3 invalid squares)
                # Count consecutive x's
                x_count = 0
                while i < len(rank_data) and rank_data[i] == 'x':
                    x_count += 1
                    i += 1
                file += x_count
                continue

            if ch.isdigit():
                # Empty squares: could be 1 or 2 digit number
                num_str = ch
                if i + 1 < len(rank_data) and rank_data[i + 1].isdigit():
                    num_str += rank_data[i + 1]
                    i += 1
                file += int(num_str)
                i += 1
                continue

            # Piece: player prefix (r/b/y/g) + piece type (P/N/B/R/Q/K)
            if ch in PLAYER_INDEX and i + 1 < len(rank_data):
                player_char = ch
                piece_char = rank_data[i + 1]

                if piece_char in PIECE_TYPE_INDEX:
                    player_idx = PLAYER_INDEX[player_char]
                    piece_type_idx = PIECE_TYPE_INDEX[piece_char]
                    sq_index = rank * BOARD_SIZE + file

                    if 0 <= sq_index < TOTAL_SQUARES:
                        pieces.append((sq_index, piece_type_idx, player_idx))

                file += 1
                i += 2
                continue

            # Unknown character — skip
            i += 1
            file += 1

    return pieces


# ─── Feature Extraction ────────────────────────────────────────────────────

def extract_features(fen4_str: str, perspective: int):
    """Extract active feature indices for a position from a given perspective.

    Returns a list of active feature indices (ints in 0..4479).
    """
    pieces = parse_fen4(fen4_str)
    features = []

    for sq_index, piece_type_idx, player_idx in pieces:
        idx = feature_index(perspective, sq_index, piece_type_idx, player_idx)
        if idx >= 0:
            features.append(idx)

    return features


def extract_all_perspectives(fen4_str: str):
    """Extract feature indices for all 4 perspectives.

    Returns: list of 4 lists of active feature indices.
    """
    return [extract_features(fen4_str, p) for p in range(4)]


# ─── PyTorch Dataset ───────────────────────────────────────────────────────

class FreyjaDataset(Dataset):
    """PyTorch dataset for NNUE training.

    Each sample contains:
    - features_per_perspective: 4 lists of active feature indices
    - target_4vec: [Red, Blue, Yellow, Green] eval scores (normalized)
    """

    def __init__(self, records, scale=3000.0):
        """
        Args:
            records: list of dicts with 'fen4' and 'eval_4vec' keys
            scale: score normalization factor (divide by this)
        """
        self.records = records
        self.scale = scale

    def __len__(self):
        return len(self.records)

    def __getitem__(self, idx):
        rec = self.records[idx]
        fen4 = rec['fen4']
        target = np.array(rec['eval_4vec'], dtype=np.float32) / self.scale

        # Extract features for all 4 perspectives
        all_features = extract_all_perspectives(fen4)

        # Convert to dense binary vectors (0/1) for each perspective
        # Shape: [4, TOTAL_FEATURES] — mostly zeros, sparse
        dense = np.zeros((4, NUM_PIECE_SQ_FEATURES), dtype=np.float32)
        for p in range(4):
            for idx_feat in all_features[p]:
                dense[p, idx_feat] = 1.0

        return torch.tensor(dense), torch.tensor(target)


def load_jsonl(path: str):
    """Load training records from a JSONL file."""
    records = []
    with open(path, 'r') as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def load_game_json(path: str):
    """Load training records from a game JSON file (observer format).

    Extracts per-ply records with fen4 + scores.
    """
    with open(path, 'r') as f:
        data = json.load(f)

    games = data if isinstance(data, list) else [data]
    records = []

    for game in games:
        game_result = game.get('game_result')
        for ply in game.get('plies', []):
            if not ply.get('fen4') or not ply.get('scores'):
                continue
            scores = ply['scores']
            records.append({
                'fen4': ply['fen4'],
                'eval_4vec': [
                    scores.get('red', 0),
                    scores.get('blue', 0),
                    scores.get('yellow', 0),
                    scores.get('green', 0),
                ],
                'best_move': ply.get('move', ''),
                'player': ply.get('player', '').lower(),
                'ply': ply.get('ply', 0),
                'depth': ply.get('depth'),
                'game_result': game_result,
            })

    return records


def split_by_game(records, val_ratio=0.2):
    """Split records into train/val by game (not position) to avoid data leakage.

    Groups records by detecting game boundaries (ply decreases or resets),
    then splits at the game level.
    """
    # Group by detecting game boundaries: when ply decreases, a new game started
    games = []
    current_game = []
    prev_ply = -1

    for rec in records:
        ply = rec.get('ply', 0)
        if current_game and ply <= prev_ply:
            # Ply decreased or reset → new game
            games.append(current_game)
            current_game = [rec]
        else:
            current_game.append(rec)
        prev_ply = ply
    if current_game:
        games.append(current_game)

    # Ensure we have at least 2 games for split
    if len(games) < 2:
        # Fallback: simple position-level split
        n_val = max(1, int(len(records) * val_ratio))
        return records[:-n_val], records[-n_val:]

    # Split games
    n_val = max(1, int(len(games) * val_ratio))
    n_train = len(games) - n_val

    train_records = []
    for game in games[:n_train]:
        train_records.extend(game)

    val_records = []
    for game in games[n_train:]:
        val_records.extend(game)

    return train_records, val_records
