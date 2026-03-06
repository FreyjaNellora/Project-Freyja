# Connection: GameState → Protocol

**Created:** 2026-03-06
**Last Updated:** 2026-03-06

---

## Overview

Protocol owns a GameState instance and uses its public API to implement the engine-UI communication. Protocol is the orchestration layer; GameState is the game logic layer.

## Interface

```rust
// Protocol reads from GameState:
gs.legal_moves() -> ArrayVec<Move, 256>
gs.current_player() -> Player
gs.is_game_over() -> bool
gs.scores() -> [u16; 4]
gs.player_status(player) -> PlayerStatus
gs.board() -> &Board  // for FEN4 output

// Protocol mutates GameState:
gs.apply_move(mv: Move)

// Protocol creates GameState:
GameState::new_standard_ffa()
GameState::new(Board::from_fen4(fen))
```

## Data Flow

1. `position` command → Protocol creates a new GameState, applies moves via `apply_move`
2. `go` command → Protocol calls `legal_moves()`, returns first move (stub) with `scores()`
3. After each `apply_move`, Protocol diffs `player_status` to detect eliminations
4. Protocol emits `info string nextturn {Color}` using `current_player()`

## Failure Modes

- Invalid FEN4 → `Board::from_fen4` returns `Err`, Protocol sends error message
- Illegal move string → `parse_move_str` fails to match against legal moves, Protocol sends error
- Game already over → Protocol returns `bestmove (none)` with info message

## Notes

- Protocol does NOT apply bestmove to GameState. The UI must send updated position with the move included.
- `legal_moves()` requires `&mut self` on GameState. Protocol has exclusive ownership so this is fine.
- Search (Stage 7+) will use Board directly, not GameState. Protocol will call search, which returns a Move, which Protocol formats as bestmove.

---

**Related:** [[Component-GameState]], [[Component-Protocol]], [[Pattern-Protocol-Status-Diffing]]
