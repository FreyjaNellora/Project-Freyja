# Downstream Log — Stage 02: Move Generation

**Date:** 2026-03-04
**Session:** 4

---

## Must-Know Facts

- Move generation uses pseudo-legal generation + make/unmake filtering for legality.
- All Board mutations go through Zobrist-correct methods — no raw field writes.
- `MoveUndo` is fixed-size (no heap). Stores: move, captured piece, prev castling rights, prev EP square/player, prev Zobrist, prev side to move.
- `update_piece_list_square()` is now used by make/unmake (was dead code in Stage 1).
- perft(2) = 395, not 400. Red moves interact with Blue's position (blocking double pushes, creating pins).

---

## API Contracts

### Public Types
- `Move` — u32 bitfield. Constructors: `new`, `capture`, `new_promotion`, `double_push`, `en_passant`, `castle`. Getters: `from_sq`, `to_sq`, `piece_type`, `captured`, `promotion`, `flags`, `is_capture`.
- `MoveFlags` — enum: Normal, DoublePush, Castle, EnPassant.
- `MoveUndo` — struct returned by `make_move`, consumed by `unmake_move`.

### Public Functions
- `generate_legal_moves(board: &mut Board) -> ArrayVec<Move, 256>` — returns all legal moves for side to move.
- `generate_legal_into(board: &mut Board, moves: &mut ArrayVec<Move, 256>)` — appends legal moves into caller-provided buffer.
- `make_move(board: &mut Board, mv: Move) -> MoveUndo` — applies move, returns undo info.
- `unmake_move(board: &mut Board, undo: &MoveUndo)` — reverses move exactly.
- `perft(board: &mut Board, depth: u32) -> u64` — recursive node count for move generation validation.

### Invariants
- `make_move` + `unmake_move` must leave board in identical state (including Zobrist hash).
- All generated moves target valid squares only.
- Perft values at depths 1-4 from starting position are permanent (see `tests/perft_depth.rs`).

---

## Known Limitations

- Legal filtering does make/unmake for every pseudo-legal move. No in-check shortcut or pinned piece optimization yet. Fine for Stage 2; may optimize in later stages if needed.
- No move ordering. Moves are generated in piece-list order, not by quality. Stage 3+ will add MVV-LVA, killer, history.
- `MAX_MOVES = 256` is generous. Actual max legal moves in 4PC is much lower, but ArrayVec overhead is negligible.
- Promotion generates `PromotedQueen` (index 7), not `Queen` (index 4). Downstream code must handle both when checking for queen-like pieces.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| perft(4) nodes | 152,050 | From starting position |
| perft(4) wall time | ~0.7s | Debug build, unoptimized |

Release build performance TBD in later stages.

---

## Open Questions

- None.

---

## Reasoning

- **Pseudo-legal + filter approach** chosen over direct legal generation because it's simpler, less error-prone, and the make/unmake infrastructure is needed anyway for search. Performance cost is acceptable at this stage.
- **Hardcoded castling table** rather than computed paths because 4PC castling has unique geometry per player that doesn't follow simple patterns. A table is clearer and less error-prone.
- **Board scan for EP** (ADR-009) rather than `player.prev()` because eliminated players would cause crashes. The scan finds the captured pawn by walking one step past the EP target in the pushing player's direction.
- **PromotedQueen vs Queen distinction** preserved because 4PC rules may treat promoted pieces differently (e.g., scoring). The attack generation already handles PromotedQueen identically to Queen.
