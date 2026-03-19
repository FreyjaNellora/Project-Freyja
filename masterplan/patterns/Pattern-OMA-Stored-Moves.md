---
tags: [pattern, mcts, oma, stage-14]
stage: 14
related: [[MCTS]], [[ADR-018]]
---

# Pattern: OMA Stored Moves for Tree Consistency

## Problem

Opponent Move Abstraction (OMA) in MCTS skips tree expansion for opponents, using a lightweight policy to select one move per opponent. But different simulations can produce different OMA moves at the same tree node (due to RNG, history changes, etc.), causing the board state at that node to vary. Tree node children become invalid for the "wrong" board state, leading to crashes (piece on empty square, zobrist mismatch).

Attempted fix of seeding the OMA RNG from the zobrist hash (making OMA deterministic per position) failed after ~25 simulations due to accumulated hash drift in deep trees.

## Solution

Store OMA moves at each tree node on first visit. Replay stored moves on subsequent visits.

```rust
struct MctsNode {
    // ... existing fields ...
    oma_moves: ArrayVec<Move, 3>,  // Up to 3 opponent moves (4-player game)
    oma_computed: bool,
}
```

**First visit:** Run OMA policy, store selected moves in `node.oma_moves`, set `oma_computed = true`.
**Revisit:** Replay `node.oma_moves` via `make_move` (skip OMA policy entirely).

## Key Subtlety

After replaying stored OMA moves, check if side_to_move is still an opponent (e.g., one was eliminated during first computation, breaking the OMA loop early). If so, `break` to evaluate rather than `continue` (which would infinite-loop back to the OMA check).

## Cost

~13 bytes per tree node (3 moves * 4 bytes + 1 bool). Negligible compared to existing node size (~100 bytes).

## Research Basis

Baier & Kaisers (IEEE CoG 2020): OMA is designed to generalize value estimates across different opponent moves. Tree nodes abstract over opponents — they don't require a fixed opponent sequence. Our stored-move approach is stricter (deterministic replay) but correct for engines where tree children must match the board state.

## When to Apply

Any MCTS implementation where opponent nodes are skipped (OMA, BRS-in-tree, or similar opponent abstraction). The tree must either:
1. Store opponent moves per node (our approach), or
2. Store full board state per node (expensive), or
3. Re-validate children's legality on revisit (complex)
