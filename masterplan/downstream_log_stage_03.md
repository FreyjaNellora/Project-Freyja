# Downstream Log -- Stage 03: Game State

## API Contracts

### GameState Public API

```rust
// Construction
GameState::new(board: Board) -> Self
GameState::new_standard_ffa() -> Self

// Accessors (immutable)
board() -> &Board
board_mut() -> &mut Board
player_status(player: Player) -> PlayerStatus
score(player: Player) -> u16
scores() -> [u16; 4]
current_player() -> Player
is_game_over() -> bool
winner() -> Option<Player>
result() -> GameResult
is_active(player: Player) -> bool
active_player_count() -> usize
half_move_clock() -> u16
history_count() -> u16
is_threefold_repetition() -> bool

// Mutable operations
legal_moves() -> ArrayVec<Move, MAX_MOVES>   // generates for current_player
apply_move(mv: Move)                          // central game loop method
resign_player(player: Player)                 // triggers DKW
timeout_player(player: Player)                // same as resign
handle_no_legal_moves()                       // when engine detects 0 moves
```

### Enums

```rust
pub enum PlayerStatus { Active, DeadKingWalking, Eliminated }
pub enum EliminationReason { Checkmate, Stalemate, Timeout, Resignation, DkwKingStuck }
pub enum GameMode { FreeForAll }  // Teams added Stage 18
pub enum GameResult { Ongoing, Decisive { winner: Player }, Draw }
```

### Scoring Functions (public)

```rust
pub fn capture_points(piece_type: PieceType) -> u16
pub fn check_bonus_points(kings_checked: usize) -> u16
```

### Constants (public)

```rust
pub const MAX_GAME_LENGTH: usize = 1024;
pub const CHECKMATE_POINTS: u16 = 20;
pub const STALEMATE_POINTS: u16 = 20;
pub const DRAW_POINTS: u16 = 10;
pub const CLAIM_WIN_THRESHOLD: u16 = 21;
pub const FIFTY_MOVE_THRESHOLD: u16 = 200;
```

## Known Limitations

1. **Search uses Board, not GameState:** Search (Stage 7+) should use `make_move`/`unmake_move` on `Board` directly, not `apply_move`. GameState fields are stale during search. This is by design (Odin W5).

2. **GameState clone cost:** ~11KB per clone (8KB position history + Board). Avoid cloning in hot paths. Search should work on Board.

3. **DKW halfmove_clock:** DKW moves through make_move increment halfmove_clock. This is a rules grey area; documented but may need revisiting.

4. **game_mode field:** Currently dead code (`#[allow(dead_code)]`). Will be used in Stage 18 for Teams mode.

5. **King capture scoring:** When a king is captured via make_move (before elimination chain), the capturer gets CHECKMATE_POINTS (20). This duplicates the elimination chain's normal checkmate scoring but is correct because the chain won't re-award.

6. **Position history overflow:** After MAX_GAME_LENGTH (1024) positions, new positions are silently not recorded. Games this long should have been drawn by 50-move rule (200 half-moves).

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Random playout avg length | ~1004 half-moves | 1000 games, seeded LCG |
| Random playout shortest | 107 half-moves | |
| Random playout longest | 1672 half-moves | |
| GameState size | ~11KB | Fixed-size, no heap |
| Unit tests | 187 | 0 failures |
| Integration tests | 6 | 5 perft + 1 playout |

## Board Additions for This Stage

- `Board::set_king_eliminated(player)`: Sets king_squares[player] = ELIMINATED_KING_SENTINEL. `pub(crate)`.
- `Player::prev()`: Returns previous player in turn order (Red->Green, Blue->Red, etc.).
