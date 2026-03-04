# Downstream Log — Stage 01: Board Representation

## Must-Know

1. **Board is `[Option<Piece>; 196]`, not 160.** The full 14x14 = 196 array is used. Invalid corner squares are always `None`. Use `is_valid_square(rank, file)` or `VALID_SQUARE_TABLE[index]` to check validity before accessing.

2. **Square is `Square(u8)`.** The inner value is `rank * 14 + file`, 0-indexed. Display notation is 1-indexed: `Square(7)` displays as `"h1"` (rank 0, file 7 → display rank 1, file 'h'). Use `Square::new(rank, file)` which returns `None` for invalid squares.

3. **Piece lists are fixed-size `[Option<(PieceType, Square)>; 32]` per player.** `piece_counts[player]` tracks how many entries are valid. Uses swap-remove for O(1) deletion — order is NOT preserved. Stage 2 must not assume ordering.

4. **Zobrist updates are incremental.** Every mutation method (`set_piece`, `remove_piece`, `set_castling_rights`, `set_en_passant`, `set_side_to_move`) XORs keys in/out of `board.zobrist`. Stage 2 make/unmake must use these methods to keep the hash consistent.

5. **King squares are tracked in `king_squares[4]`.** Updated automatically by `set_piece()`/`remove_piece()`. Use `board.king_square(player)` to read. Eliminated king sentinel = `Square(255)`.

6. **Player turn order:** Red(0) → Blue(1) → Yellow(2) → Green(3) → Red. Use `Player::next()` for turn advancement. `Player::opponents()` returns the other 3 players.

7. **Pawn capture directions are player-specific:**
   - Red: NE (+1,+1), NW (+1,-1)
   - Blue: NE (+1,+1), SE (-1,+1)
   - Yellow: SE (-1,+1), SW (-1,-1)
   - Green: NW (+1,-1), SW (-1,-1)
   Constants: `PAWN_CAPTURE_DELTAS[player_index]` in `board/mod.rs`.

8. **Blue/Yellow have King-Queen swapped** compared to Red/Green. This is already correct in `Board::starting_position()`. Stage 2 castling must use the correct king files.

## API Contracts

### Board construction
- `Board::empty()` — empty board, Red to move, all castling, no EP
- `Board::starting_position()` — all 64 pieces placed, Zobrist computed
- `Board::from_fen4(s) -> Result<Board, Fen4Error>` — parse FEN4 string
- `Board::to_fen4() -> String` — serialize to FEN4

### Board queries (read-only)
- `board.piece_at(sq) -> Option<Piece>`
- `board.side_to_move() -> Player`
- `board.king_square(player) -> Square`
- `board.castling_rights() -> u8` — bitmask, 2 bits per player (KS, QS)
- `board.en_passant() -> (Option<Square>, Option<Player>)`
- `board.zobrist_hash() -> u64`
- `board.piece_count(player) -> u8`
- `board.pieces(player) -> impl Iterator<Item = (PieceType, Square)>`

### Board mutation
- `board.set_piece(sq, piece)` — places piece, updates lists + Zobrist
- `board.remove_piece(sq) -> Piece` — removes piece, updates lists + Zobrist
- `board.set_castling_rights(u8)` — updates Zobrist
- `board.set_en_passant(Option<Square>, Option<Player>)` — updates Zobrist
- `board.set_side_to_move(Player)` — updates Zobrist
- `board.compute_full_hash() -> u64` — from-scratch hash for verification

### Attack queries
- `board.is_square_attacked_by(sq, player) -> bool`
- `board.attackers_of(sq) -> AttackInfo` — all attackers from all players
- `board.is_in_check(player) -> bool` — king attacked by any opponent

### Piece list management (pub(crate))
- `board.add_to_piece_list(player, piece_type, sq)`
- `board.remove_from_piece_list(player, sq) -> (PieceType, Square)`
- `board.update_piece_list_square(player, from, to)` — for move execution
- `board.find_in_piece_list(player, sq) -> Option<usize>`
- `board.assert_piece_list_sync()` — debug assertion

## Known Limitations

1. **No make/unmake move.** Board can set/remove individual pieces but has no concept of chess moves. Stage 2 builds this.
2. **No move legality checking.** Attack queries tell you IF a square is attacked, not whether a move is legal. Stage 2 uses these for pin/check detection.
3. **No promotion tracking in piece lists.** `PieceType::PromotedQueen` exists as a type but promotion logic is Stage 2.
4. **`update_piece_list_square()` is currently dead code.** It exists for Stage 2 make/unmake to use.
5. **FEN4 format is custom.** No interop with external tools unless they implement the same format.
6. **Pawn forward direction is not encoded.** Only capture deltas are stored. Stage 2 movegen must encode pawn push directions separately.

## Performance Baselines

No benchmarks yet. Baseline measurements will be set after Stage 2 perft.

Key design choices for performance:
- All fixed-size arrays, zero heap allocation in Board
- `Square(u8)` is Copy, 1 byte
- Zobrist keys cached in `OnceLock` (one-time init)
- Swap-remove piece list: O(1) add/remove
- Attack queries: ray walk stops at first piece or invalid square

## Open Questions

1. **`movegen` vs `move_gen` naming:** Carried from Stage 0. Currently `movegen`. Decide before Stage 2.

## Reasoning

1. **Module split over monolith:** `board.rs` was split into `board/mod.rs` + `types.rs` + `zobrist.rs` + `fen4.rs` + `attacks.rs` (total ~1,600 lines). Keeps each concern isolated while sharing the `Board` struct.
2. **`Option<Piece>` over enum with Empty variant:** More idiomatic Rust. `None` = empty, `Some(Piece)` = occupied. Compiler optimizes `Option<Piece>` to same size as `Piece` + tag.
3. **Piece list size 32 (not 16):** Promotions can increase piece count beyond 16 per player (up to 8 extra queens from pawns + initial pieces). 32 is conservative upper bound.
4. **FEN4 custom format:** No standard exists for 4-player chess. Format uses `x` for invalid corners, 2-char piece codes (`rP`, `bK`), and explicit EP player field. Designed for round-trip correctness over compactness.
5. **`PromotedQueen` as separate PieceType:** Distinguishes original queens from promoted ones for evaluation purposes (Stage 6+). Moves identically to Queen.
