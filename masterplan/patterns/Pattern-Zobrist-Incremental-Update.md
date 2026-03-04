# Pattern: Zobrist Incremental Update

**Discovered:** 2026-03-03
**Stage:** Stage 1
**Category:** Performance | Correctness

---

## Problem

Transposition tables require a hash of the board position. Computing the hash from scratch after every move is O(n) where n = number of pieces. For a 4-player board with up to 64 pieces, this is expensive at millions of nodes/second.

## Solution

Zobrist hashing uses XOR, which is self-inverse: `h ^ k ^ k = h`. Maintain a running hash and XOR in/out only the changed keys when the board is mutated.

```rust
fn set_piece(&mut self, sq: Square, piece: Piece) {
    let keys = zobrist_keys();
    // XOR in the new piece
    self.zobrist ^= keys.piece[piece.piece_type.index()][piece.player.index()][sq.index()];
    self.squares[sq.index()] = Some(piece);
    // ... update piece lists, king squares
}

fn remove_piece(&mut self, sq: Square) -> Piece {
    let piece = self.squares[sq.index()].take().expect("no piece");
    let keys = zobrist_keys();
    // XOR out the removed piece (same operation as XOR in)
    self.zobrist ^= keys.piece[piece.piece_type.index()][piece.player.index()][sq.index()];
    // ... update piece lists, king squares
    piece
}
```

Key insight: XOR is both add and remove. `set_piece` and `remove_piece` use the same key lookup.

## When to Use

- Any time the board state changes (move, capture, promotion, castling, en passant)
- Side-to-move, castling rights, and en passant also have Zobrist keys

## When NOT to Use

- Initial position setup (use `compute_full_hash()` once, then incremental from there)
- FEN4 parsing (compute full hash after all pieces are placed)

## Verification Pattern

Always test that incremental hash matches from-scratch hash:

```rust
let incremental = board.zobrist_hash();
let from_scratch = board.compute_full_hash();
assert_eq!(incremental, from_scratch);
```

## Examples in Codebase

- `Board::set_piece()`, `Board::remove_piece()` in `freyja-engine/src/board/mod.rs`
- `Board::set_castling_rights()`, `Board::set_en_passant()`, `Board::set_side_to_move()` in same file
- `Board::compute_full_hash()` for verification

---

**Related:** [[MASTERPLAN]], [[Component-Board]], [[DECISIONS]] (ADR-005)
