# Pattern: Protocol Status Diffing

**Discovered:** 2026-03-06
**Stage:** Stage 4
**Category:** Architecture

---

## Problem

GameState::apply_move doesn't return elimination events. The protocol needs to emit `info string eliminated {Color} {reason}` messages when players are eliminated, but it can't know what changed without inspecting state before and after.

## Solution

Snapshot `player_status` for all 4 players before `apply_move`, then compare after:

```rust
fn apply_move_with_events(&mut self, mv: Move) {
    let statuses_before: [PlayerStatus; 4] = [
        self.game_state.player_status(Player::Red),
        self.game_state.player_status(Player::Blue),
        self.game_state.player_status(Player::Yellow),
        self.game_state.player_status(Player::Green),
    ];

    self.game_state.apply_move(mv);

    for (i, player) in Player::all().iter().enumerate() {
        if statuses_before[i] != PlayerStatus::Eliminated
            && self.game_state.player_status(*player) == PlayerStatus::Eliminated
        {
            let reason = match statuses_before[i] {
                PlayerStatus::DeadKingWalking => "dkw",
                _ => "checkmate",
            };
            self.send(&format_eliminated(*player, reason));
        }
    }
}
```

## When to Use

- Protocol layer when applying moves and needing to emit events
- Any observer/logger that needs to know what changed during a move

## When NOT to Use

- During search (no event emission needed)
- If GameState is enhanced to return elimination events directly

## Examples in Codebase

- `freyja-engine/src/protocol/mod.rs` — `apply_move_with_events()`

---

**Related:** [[Component-Protocol]], [[Component-GameState]]
