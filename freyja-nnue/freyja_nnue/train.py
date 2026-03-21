"""Freyja NNUE training loop.

Trains the network on self-play data with MSE loss on 4-vector eval output.

Usage:
    python -m freyja_nnue.train --data training.jsonl --output weights.fnnue

Stage 16: NNUE Architecture + Training Pipeline
"""

import argparse
import sys
import time
from pathlib import Path

import torch
import torch.nn as nn
from torch.utils.data import DataLoader

from .data import FreyjaDataset, load_jsonl, load_game_json, split_by_game
from .model import FreyjaNet, count_parameters
from .export import export_fnnue


def train(args):
    """Main training function."""
    print(f"Loading data from: {args.data}")

    # Load training records
    data_path = Path(args.data)
    if data_path.suffix == '.jsonl':
        records = load_jsonl(str(data_path))
    else:
        records = load_game_json(str(data_path))

    if not records:
        print("ERROR: No training records found")
        sys.exit(1)

    print(f"Loaded {len(records)} training records")

    # Filter records: require fen4 and eval_4vec
    records = [r for r in records if r.get('fen4') and r.get('eval_4vec')]
    print(f"After filtering: {len(records)} valid records")

    if len(records) < 10:
        print("ERROR: Need at least 10 valid training records")
        sys.exit(1)

    # Split into train/validation by game
    train_records, val_records = split_by_game(records, val_ratio=args.val_split)
    print(f"Train: {len(train_records)}, Validation: {len(val_records)}")

    # Create datasets
    train_dataset = FreyjaDataset(train_records, scale=args.scale)
    val_dataset = FreyjaDataset(val_records, scale=args.scale)

    train_loader = DataLoader(
        train_dataset,
        batch_size=args.batch_size,
        shuffle=True,
        num_workers=0,
        pin_memory=True,
    )
    val_loader = DataLoader(
        val_dataset,
        batch_size=args.batch_size,
        shuffle=False,
        num_workers=0,
    )

    # Create model
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    model = FreyjaNet().to(device)
    print(f"Model parameters: {count_parameters(model):,}")
    print(f"Device: {device}")

    # Optimizer
    optimizer = torch.optim.Adam(
        model.parameters(),
        lr=args.lr,
        weight_decay=args.weight_decay,
    )

    # Loss: MSE on 4-vector
    criterion = nn.MSELoss()

    # Training loop
    best_val_loss = float('inf')
    patience_counter = 0
    best_model_state = None

    print(f"\nTraining for up to {args.epochs} epochs (patience={args.patience})")
    print("-" * 60)

    for epoch in range(args.epochs):
        t0 = time.time()

        # Train
        model.train()
        train_loss = 0.0
        train_batches = 0
        for features, targets in train_loader:
            features = features.to(device)
            targets = targets.to(device)

            optimizer.zero_grad()
            outputs = model(features)
            loss = criterion(outputs, targets)
            loss.backward()
            optimizer.step()

            train_loss += loss.item()
            train_batches += 1

        avg_train_loss = train_loss / max(1, train_batches)

        # Validate
        model.eval()
        val_loss = 0.0
        val_batches = 0
        with torch.no_grad():
            for features, targets in val_loader:
                features = features.to(device)
                targets = targets.to(device)
                outputs = model(features)
                loss = criterion(outputs, targets)
                val_loss += loss.item()
                val_batches += 1

        avg_val_loss = val_loss / max(1, val_batches)
        elapsed = time.time() - t0

        print(f"Epoch {epoch+1:3d}/{args.epochs}  "
              f"train_loss={avg_train_loss:.6f}  "
              f"val_loss={avg_val_loss:.6f}  "
              f"time={elapsed:.1f}s")

        # Early stopping
        if avg_val_loss < best_val_loss:
            best_val_loss = avg_val_loss
            patience_counter = 0
            best_model_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}
        else:
            patience_counter += 1
            if patience_counter >= args.patience:
                print(f"\nEarly stopping at epoch {epoch+1} (patience={args.patience})")
                break

    print("-" * 60)
    print(f"Best validation loss: {best_val_loss:.6f}")

    # Restore best model
    if best_model_state is not None:
        model.load_state_dict(best_model_state)
        model.to(device)

    # Export weights
    output_path = args.output
    print(f"\nExporting weights to: {output_path}")
    export_fnnue(model, output_path)
    print(f"Done! Weights saved to {output_path}")

    # Also save PyTorch checkpoint for later use
    checkpoint_path = output_path.replace('.fnnue', '.pt')
    torch.save({
        'model_state_dict': model.state_dict(),
        'train_loss': avg_train_loss,
        'val_loss': best_val_loss,
        'epochs': epoch + 1,
        'args': vars(args),
    }, checkpoint_path)
    print(f"PyTorch checkpoint saved to {checkpoint_path}")


def main():
    parser = argparse.ArgumentParser(description='Train Freyja NNUE')
    parser.add_argument('--data', required=True, help='Path to training data (JSONL or game JSON)')
    parser.add_argument('--output', default='weights.fnnue', help='Output .fnnue file')
    parser.add_argument('--lr', type=float, default=1e-3, help='Learning rate')
    parser.add_argument('--batch-size', type=int, default=256, help='Batch size')
    parser.add_argument('--epochs', type=int, default=100, help='Max epochs')
    parser.add_argument('--patience', type=int, default=10, help='Early stopping patience')
    parser.add_argument('--scale', type=float, default=3000.0, help='Score normalization scale')
    parser.add_argument('--val-split', type=float, default=0.2, help='Validation split ratio')
    parser.add_argument('--weight-decay', type=float, default=1e-5, help='Weight decay')
    args = parser.parse_args()

    train(args)


if __name__ == '__main__':
    main()
