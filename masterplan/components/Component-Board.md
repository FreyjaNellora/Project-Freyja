# Component: Board

**Stage Introduced:** Stage 1
**Last Updated:** 2026-03-03
**Module:** `freyja_engine::board`

---

## Purpose

The Board component represents the state of a 14x14 four-player chess board. It stores piece placement, tracks king positions, manages Zobrist hashing for transposition tables, provides attack queries for check/pin detection, and supports FEN4 serialization for position I/O.

## Public API

```rust
// Types
Square(u8)        // 0..195, rank*14+file
Player { Red=0, Blue=1, Yellow=2, Green=3 }
PieceType { Pawn, Knight, Bishop, Rook, Queen, King, PromotedQueen }
Piece { piece_type, player }
AttackInfo { count, attacker_squares, attacking_players }

// Construction
Board::empty() -> Board
Board::starting_position() -> Board
Board::from_fen4(&str) -> Result<Board, Fen4Error>
Board::to_fen4(&self) -> String

// Queries
board.piece_at(sq) -> Option<Piece>
board.king_square(player) -> Square
board.is_square_attacked_by(sq, player) -> bool
board.attackers_of(sq) -> AttackInfo
board.is_in_check(player) -> bool
board.pieces(player) -> impl Iterator<Item = (PieceType, Square)>

// Mutation (incremental Zobrist)
board.set_piece(sq, piece)
board.remove_piece(sq) -> Piece
board.set_castling_rights(u8)
board.set_en_passant(Option<Square>, Option<Player>)
board.set_side_to_move(Player)
```

## Internal Design

- **Storage:** `squares: [Option<Piece>; 196]` — full 14x14 grid, corners always `None`
- **Piece lists:** `piece_lists: [[Option<(PieceType, Square)>; 32]; 4]` — per-player, swap-remove for O(1)
- **Zobrist:** Deterministic xorshift64 PRNG seeded with `0x4652_4559_4A41_3450`. Keys for piece/square/player (4,480), side-to-move (4), castling (8), en passant (196). Incrementally updated on every mutation.
- **Attack queries:** Ray walks for sliders, L-jumps for knights, reverse capture deltas for pawns, adjacency for kings. All stop at board edges and invalid corners.

## Performance Characteristics

- Zero heap allocation (all fixed-size arrays)
- `Square(u8)` is Copy, 1 byte
- Piece list add/remove: O(1) amortized (swap-remove)
- Attack query: O(14) worst case per ray (board edge), O(1) per knight/pawn/king check
- Zobrist keys: one-time init via `OnceLock`, O(1) per mutation

## Known Limitations

- No make/unmake move (Stage 2)
- No move legality (Stage 2)
- No promotion execution (Stage 2)
- Pawn forward direction not stored (only capture deltas)

## Dependencies

- **Consumes:** `tracing` (debug logging in set/remove)
- **Consumed By:** Stage 2 (Move Generation), Stage 3 (Game State), Stage 7 (Search)

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_01]], [[downstream_log_stage_01]], [[Pattern-Fixed-Size-Piece-List]], [[Pattern-Zobrist-Incremental-Update]]
