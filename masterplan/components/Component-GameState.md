# Component: GameState

**Stage Introduced:** Stage 3
**Last Updated:** 2026-03-06
**Module:** `freyja_engine::game_state`

---

## Purpose

Manages the full game state for a 4-player FFA chess game. Tracks player statuses (Active/DeadKingWalking/Eliminated), scores, turn management with eliminated player skipping, checkmate/stalemate detection, elimination chains, DKW random king processing, position history for threefold repetition, and game termination conditions.

## Public API

```rust
// Construction
GameState::new(board: Board) -> Self
GameState::new_standard_ffa() -> Self

// Accessors
board() -> &Board
board_mut() -> &mut Board
player_status(player) -> PlayerStatus
score(player) -> u16
scores() -> [u16; 4]
current_player() -> Player
is_game_over() -> bool
winner() -> Option<Player>
result() -> GameResult
is_active(player) -> bool
active_player_count() -> usize
half_move_clock() -> u16
history_count() -> u16
is_threefold_repetition() -> bool

// Mutable operations
legal_moves() -> ArrayVec<Move, 256>
apply_move(mv: Move)
resign_player(player: Player)
timeout_player(player: Player)
handle_no_legal_moves()

// Scoring functions
capture_points(piece_type) -> u16  // Pawn:1, Knight:3, Bishop:5, Rook:5, Queen:9, PromotedQueen:1
check_bonus_points(kings_checked) -> u16
```

## Internal Design

- **Fixed-size arrays only.** No heap allocation. Position history: `[u64; 1024]`.
- **3-state PlayerStatus:** Active → DeadKingWalking (resignation/timeout) → Eliminated (checkmate/stalemate/DKW stuck).
- **Elimination chain:** After any move, loops checking all players until no new eliminations occur. Handles cascading checkmates.
- **DKW processing:** LCG random king moves (no captures, no castling). If no legal king move, player is eliminated.
- **Turn management:** Skips Eliminated players. DKW players get auto-processed (not full turns).
- **King capture handling:** Player B can capture player C's king via make_move before elimination chain detects checkmate. Guarded in apply_move.
- **Game termination:** Last standing, claim win (21pt lead in 2-player), 50-move rule (200 half-moves), threefold repetition.

## Performance Characteristics

- GameState size: ~11KB (fixed-size, no heap)
- legal_moves: delegates to generate_legal_moves
- apply_move: O(legal_moves) for elimination chain check per remaining player
- 1000 random playouts avg ~1004 half-moves, no panics

## Known Limitations

- **Search boundary:** Search (Stage 7+) uses Board::make_move/unmake_move directly, NOT apply_move. GameState fields are stale during search — by design.
- Position history overflow at 1024 entries (silently stops recording; games should draw by 50-move rule before this).
- GameMode::FreeForAll is the only mode until Stage 18.
- EliminationReason is not stored on GameState after elimination.

## Dependencies

- **Consumes:** [[Component-Board]], [[Component-MoveGen]]
- **Consumed By:** [[Component-Protocol]] (apply_move, legal_moves, scores, player_status)

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_03]], [[downstream_log_stage_03]], [[Pattern-Elimination-Chain]], [[Pattern-DKW-Processing]]
