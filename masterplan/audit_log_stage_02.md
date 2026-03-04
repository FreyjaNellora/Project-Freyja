# Audit Log — Stage 02: Move Generation

## Pre-Audit

**Date:** 2026-03-04
**Session:** 4

### Build State
- `cargo build`: PASSES
- `cargo test`: PASSES (89 tests, 0 failures)
- `cargo fmt --check`: PASSES
- `cargo clippy`: PASSES

### Upstream Audit Logs Reviewed

- **`audit_log_stage_00.md`:** No blocking/warning findings. User green light received.
- **`audit_log_stage_01.md`:** No blocking/warning findings. 3 NOTE-level:
  - S01-F01: `update_piece_list_square()` is dead code — Stage 2 make/unmake will use it. ✅ Ready.
  - S01-F02: Module naming (`movegen` vs `move_gen`) — resolved in this session (ADR-016). Renamed to `move_gen`.
  - S01-F03: FEN4 format is custom. Not a Stage 2 concern.

### Upstream Downstream Logs Reviewed

- **`downstream_log_stage_00.md`:** Workspace structure confirmed. `arrayvec` added per ADR-010.
- **`downstream_log_stage_01.md`:** Full API contract reviewed. Key items for Stage 2:
  - Board mutation methods (`set_piece`, `remove_piece`, `set_castling_rights`, `set_en_passant`, `set_side_to_move`) are Zobrist-correct
  - `update_piece_list_square()` exists for make/unmake
  - Attack queries (`is_square_attacked_by`, `is_in_check`) available for legal filtering and castling validation
  - Direction constants (`ORTHOGONAL_DIRS`, `DIAGONAL_DIRS`, `ALL_DIRS`, `KNIGHT_OFFSETS`, `PAWN_CAPTURE_DELTAS`) available
  - Pawn forward direction NOT in board — movegen must encode push directions

### Risks for This Stage

1. **Pawn direction reversal:** Each player pushes in a different direction. Red +rank, Blue +file, Yellow -rank, Green -file. Sign errors = backward pawns. Mitigated by per-player tests.
2. **Castling complexity (8 variants):** 4 players x 2 sides = 8 castling variants with different squares, paths, and attack checks. Mitigated by hardcoded table from 4PC_RULES_REFERENCE.
3. **En passant with eliminations:** Must use board scan (ADR-009), not `player.prev()`. EP target square from board state.
4. **Promotion rank per player:** Each player promotes on a different rank/file. Red rank 8, Blue file 8, Yellow rank 5, Green file 5 (0-indexed).
5. **Corner square move generation:** Sliding pieces and knights near corners must not generate moves to/from invalid squares.
6. **make/unmake Zobrist round-trip:** The hardest invariant — hash must be identical after make+unmake for every possible move.

---

## Post-Audit

**Date:** 2026-03-04
**Session:** 4

### Build State
- `cargo build`: PASSES
- `cargo test`: PASSES (149 tests: 144 unit + 5 integration, 0 failures)
- `cargo fmt --check`: PASSES
- `cargo clippy`: PASSES (0 warnings)

### Implementation Summary

**File:** `freyja-engine/src/move_gen.rs` (~1650 lines)

- Move encoding: `Move` as u32 bitfield (from, to, piece_type, captured, promotion, flags)
- `MoveFlags`: Normal, DoublePush, Castle, EnPassant
- `MoveUndo`: fixed-size struct (no heap allocation) storing all state needed for unmake
- Piece generators: pawn (push, double push, capture, promotion), knight, slider (bishop/rook/queen), king
- Castling: 8 variants via hardcoded `CASTLE_DEFS` table from 4PC_RULES_REFERENCE
- En passant: board scan pattern (ADR-009) via `find_ep_captured_pawn_sq()`
- Legal filtering: pseudo-legal → make → is_in_check → unmake
- Public API: `generate_legal_moves()`, `generate_legal_into()`, `make_move()`, `unmake_move()`, `perft()`
- All Board mutations via Zobrist-correct methods (set_piece, remove_piece, etc.)

### Perft Values (Permanent Invariants)

| Depth | Nodes |
|-------|-------|
| 1 | 20 |
| 2 | 395 |
| 3 | 7,800 |
| 4 | 152,050 |

**Note:** perft(2) = 395 (not 400) because Red moves can block Blue double pushes and open pin lines.

### Risk Mitigation Results

1. **Pawn direction reversal:** ✅ All 4 players tested (push, double push, capture)
2. **Castling complexity:** ✅ All 8 variants tested individually
3. **En passant with eliminations:** ✅ All 4 players tested with board scan pattern
4. **Promotion rank per player:** ✅ All 4 players tested
5. **Corner square generation:** ✅ Validated no moves to/from invalid squares in starting position
6. **make/unmake Zobrist round-trip:** ✅ All starting moves + EP + castle + promotion round-trips verified

### Findings

- **S02-F01 (NOTE):** `update_piece_list_square()` from Stage 1 is now used by make/unmake. Dead code resolved.
- **S02-F02 (NOTE):** `PromotedQueen` maps to index 7 in piece type. Promotion generates PromotedQueen (not Queen) to distinguish from original queen per 4PC rules.
- No BLOCK or WARN findings.

---

## 4PC Verification Matrix — Stage 2

All tests in `freyja-engine/src/move_gen.rs` (tests module).

| Rule | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Pawn single push | ✅ `test_pawn_push_direction_red` | ✅ `test_pawn_push_direction_blue` | ✅ `test_pawn_push_direction_yellow` | ✅ `test_pawn_push_direction_green` |
| Pawn double push | ✅ (same tests) | ✅ (same tests) | ✅ (same tests) | ✅ (same tests) |
| Pawn capture | ✅ `test_pawn_capture_red` | ✅ `test_pawn_capture_blue` | ✅ `test_pawn_capture_yellow` | ✅ `test_pawn_capture_green` |
| Promotion | ✅ `test_promotion_red` | ✅ `test_promotion_blue` | ✅ `test_promotion_yellow` | ✅ `test_promotion_green` |
| En passant | ✅ `test_en_passant_red` | ✅ `test_en_passant_blue` | ✅ `test_en_passant_yellow` | ✅ `test_en_passant_green` |
| Castling KS | ✅ `test_castling_red_kingside` | ✅ `test_castling_blue_kingside` | ✅ `test_castling_yellow_kingside` | ✅ `test_castling_green_kingside` |
| Castling QS | ✅ `test_castling_red_queenside` | ✅ `test_castling_blue_queenside` | ✅ `test_castling_yellow_queenside` | ✅ `test_castling_green_queenside` |
| Knight moves | ✅ `test_knight_red` | ✅ `test_knight_blue` | ✅ `test_knight_yellow` | ✅ `test_knight_green` |
| Bishop/Rook/Queen | ✅ `test_rook_red` | ✅ `test_bishop_blue` | ✅ `test_queen_yellow` | ✅ `test_rook_green` |
| King moves | ✅ `test_king_red` | ✅ `test_king_blue` | ✅ `test_king_yellow` | ✅ `test_king_green` |
| make/unmake round-trip | ✅ `test_make_unmake_all_starting_moves` + EP/castle/promo roundtrips | ✅ (perft depth 4 covers all) | ✅ (perft depth 4 covers all) | ✅ (perft depth 4 covers all) |
