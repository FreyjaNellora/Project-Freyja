# Connection: Board → MoveGen

**Created:** 2026-03-06
**Last Updated:** 2026-03-06

---

## Overview

MoveGen consumes Board's piece query and mutation APIs to generate legal moves. Board provides the "what's on the board" data; MoveGen provides the "what can move where" logic.

## Interface

```rust
// MoveGen reads from Board:
board.piece_at(sq) -> Option<Piece>
board.king_square(player) -> u8
board.is_in_check(player) -> bool
board.castling_rights() -> u8
board.en_passant() -> Option<Square>
board.side_to_move() -> Player
board.pieces(player) -> Iterator<(PieceType, Square)>

// MoveGen mutates Board (make/unmake):
board.set_piece(sq, piece)
board.remove_piece(sq) -> Piece
board.set_castling_rights(u8)
board.set_en_passant(Option<Square>, Option<Player>)
board.set_side_to_move(Player)
```

## Data Flow

1. MoveGen iterates `board.pieces(player)` to find all pieces
2. For each piece, generates pseudo-legal target squares using direction tables
3. For legality: calls `make_move` (mutates board), checks `board.is_in_check(player)`, calls `unmake_move`
4. Zobrist is incrementally maintained through make/unmake

## Failure Modes

- Calling `generate_legal_moves` on a board with no king for the current player will panic (king_square returns SENTINEL). GameState guards against this by never calling movegen for eliminated players.

## Notes

- MoveGen uses `&mut Board` even for generation (because legality check does make/unmake). Board is always restored to original state.
- The attack query API (`is_in_check`, `is_square_attacked_by`) is the defined boundary — MoveGen never directly inspects attack tables.

---

**Related:** [[Component-Board]], [[Component-MoveGen]], [[MASTERPLAN]]
