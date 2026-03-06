# Pattern: Elimination Chain

**Discovered:** 2026-03-05
**Stage:** Stage 3
**Category:** Correctness

---

## Problem

In 4-player chess, eliminating one player can cause another player to be in checkmate (the eliminated king was blocking an attack). The chain can cascade: eliminating player C exposes player D to checkmate, which then exposes player B to stalemate, etc.

## Solution

After any move or elimination, loop through all active players checking for checkmate/stalemate until no new eliminations occur:

```rust
loop {
    let mut changed = false;
    for player in Player::all() {
        if self.player_status(player) != PlayerStatus::Active { continue; }
        let legal = generate_legal_moves(board);
        if legal.is_empty() {
            if is_in_check(player) {
                self.eliminate(player, Checkmate);
            } else {
                self.eliminate(player, Stalemate);
            }
            changed = true;
        }
    }
    if !changed { break; }
}
```

## When to Use

- After every `apply_move` in GameState
- After king captures (king removal can cascade)
- After DKW processing (DKW elimination can cascade)

## When NOT to Use

- During search (Stage 7+). Search uses Board::make_move directly and does not run elimination chains — it only needs to know if a move is legal.

## Examples in Codebase

- `freyja-engine/src/game_state.rs` — `apply_move()` orchestration, lines ~600-700

---

**Related:** [[Component-GameState]], [[Pattern-DKW-Processing]], [[MASTERPLAN]]
