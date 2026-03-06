# Pattern: DKW (Dead King Walking) Processing

**Discovered:** 2026-03-05
**Stage:** Stage 3
**Category:** Correctness

---

## Problem

When a player resigns or times out in 4PC FFA, their pieces become "dead" (worth 0 capture points) but their king continues making random legal moves until stuck or captured. This must happen between turns, not as a full turn.

## Solution

DKW players are automatically processed when their turn comes up:

1. Generate king-only legal moves (no captures, no castling)
2. If no legal moves: eliminate the DKW player (king stuck)
3. If legal moves exist: pick one via LCG deterministic random, apply it
4. Skip to next player (DKW does not get a "real" turn)

```rust
// LCG for deterministic random: seed from Zobrist hash
let seed = board.zobrist_hash();
let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
let index = (random >> 33) as usize % king_moves.len();
```

## When to Use

- In GameState turn management, after advancing current_player
- Only for players with `PlayerStatus::DeadKingWalking`

## When NOT to Use

- Active players (they get full turns with all piece types)
- Eliminated players (they are skipped entirely)
- During search (search does not process DKW)

## Examples in Codebase

- `freyja-engine/src/game_state.rs` — DKW processing in `apply_move()`
- `4PC_RULES_REFERENCE.md` — DKW kings cannot capture

---

**Related:** [[Component-GameState]], [[Pattern-Elimination-Chain]], [[4PC_RULES_REFERENCE]]
