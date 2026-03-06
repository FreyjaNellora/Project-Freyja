# Pattern: 4PC Pawn Orientation

**Discovered:** 2026-03-05
**Stage:** Stage 2
**Category:** Correctness

---

## Problem

Four players have pawns that move in 4 different directions. Hardcoding one direction and applying it to all players causes pawns to move backward or sideways for 3 of the 4 players.

## Solution

Per-player direction tables indexed by `Player::index()`:

```rust
const PAWN_PUSH_DELTAS: [(i8, i8); 4] = [
    (1, 0),   // Red:    +rank (north)
    (0, 1),   // Blue:   +file (east)
    (-1, 0),  // Yellow: -rank (south)
    (0, -1),  // Green:  -file (west)
];

const PAWN_START_INDEX: [u8; 4] = [1, 1, 12, 12];  // starting rank/file
const PROMOTION_INDEX: [u8; 4] = [8, 8, 5, 5];     // promotion rank/file
```

Red/Yellow use rank for start/promotion checks. Blue/Green use file.

## When to Use

- Move generation (pawn pushes, double pushes, promotion detection)
- Evaluation (pawn structure, passed pawns) — Stage 6+
- Any code that references pawn advancement direction

## When NOT to Use

- Pawn captures use separate `PAWN_CAPTURE_DELTAS` (diagonal to the pawn's forward direction)
- Non-pawn pieces are orientation-independent

## Examples in Codebase

- `freyja-engine/src/move_gen.rs` — `PAWN_PUSH_DELTAS`, `is_start_rank()`, `is_promotion_rank()`
- `freyja-engine/src/board/mod.rs` — `PAWN_CAPTURE_DELTAS` for attack queries

---

**Related:** [[Component-MoveGen]], [[Component-Board]], [[4PC_RULES_REFERENCE]]
