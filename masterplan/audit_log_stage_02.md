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

*(To be filled after implementation)*

---

## 4PC Verification Matrix — Stage 2

*(To be filled during implementation)*

| Rule | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Pawn single push | — | — | — | — |
| Pawn double push | — | — | — | — |
| Pawn capture | — | — | — | — |
| Promotion | — | — | — | — |
| En passant | — | — | — | — |
| Castling KS | — | — | — | — |
| Castling QS | — | — | — | — |
| Knight moves | — | — | — | — |
| Bishop/Rook/Queen | — | — | — | — |
| King moves | — | — | — | — |
| make/unmake round-trip | — | — | — | — |
