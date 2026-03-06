# Component: MoveGen

**Stage Introduced:** Stage 2
**Last Updated:** 2026-03-06
**Module:** `freyja_engine::move_gen`

---

## Purpose

Generates legal moves for any player on the 14x14 four-player chess board. Handles all piece types (including PromotedQueen), special moves (castling, en passant, double push, promotion), and legality filtering (king not left in check). Provides make/unmake for board mutation and perft for verification.

## Public API

```rust
// Move generation
generate_legal_moves(board: &mut Board) -> ArrayVec<Move, 256>
generate_legal_into(board: &mut Board, moves: &mut ArrayVec<Move, 256>)

// Board mutation
make_move(board: &mut Board, mv: Move) -> MoveUndo
unmake_move(board: &mut Board, undo: &MoveUndo)

// Verification
perft(board: &mut Board, depth: u32) -> u64

// Move struct — compact u32 bitfield
Move::new(from, to, piece_type) -> Move
Move::capture(from, to, piece_type, captured) -> Move
Move::new_promotion(from, to, captured, promotion) -> Move
Move::double_push(from, to) -> Move
Move::en_passant(from, to) -> Move
Move::castle(from, to) -> Move

// Move accessors
mv.from_sq() -> Square
mv.to_sq() -> Square
mv.piece_type() -> PieceType
mv.captured() -> Option<PieceType>
mv.promotion() -> Option<PieceType>
mv.flags() -> MoveFlags
mv.is_capture() -> bool

// Display: long algebraic (d2d4, a10a11q)
impl Display for Move
```

## Internal Design

- **Move encoding:** u32 bitfield — from(8) | to(8) | piece(4) | captured(4) | promotion(4) | flags(2). 7 = "no capture"/"no promotion" sentinel.
- **Pseudo-legal generation:** Per piece type using direction tables. Pawns use `PAWN_PUSH_DELTAS` per player orientation.
- **Legality filtering:** Every pseudo-legal move is made, check tested via `is_in_check`, then unmade. Simple but correct.
- **MoveUndo:** Fixed-size struct storing captured piece, prior castling, prior EP, prior Zobrist, prior side_to_move.
- **Perft values (permanent invariants):** depth 1=20, 2=395, 3=7800, 4=152050.

## Performance Characteristics

- Zero heap allocation (`ArrayVec<Move, 256>`)
- Legal filtering: O(moves × is_in_check_cost) per position
- Perft(4): ~0.7s debug, ~0.1s release
- Move struct: Copy, 4 bytes

## Known Limitations

- No move ordering (Stage 9)
- PromotedQueen (index 6) is distinct from Queen (index 4) — downstream must handle both
- Legal filtering does full make/unmake per pseudo-legal move (no pin-aware skip optimization)

## Dependencies

- **Consumes:** [[Component-Board]] (piece_at, is_in_check, set_piece, remove_piece, Zobrist)
- **Consumed By:** [[Component-GameState]] (legal_moves, apply_move), [[Component-Protocol]] (notation parsing)

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_02]], [[downstream_log_stage_02]], [[Pattern-4PC-Pawn-Orientation]]
