# Audit Log — Stage 01: Board Representation

## Pre-Audit

**Date:** 2026-03-03
**Session:** 3

### Build State
- `cargo build`: PASSES (0 warnings)
- `cargo test`: PASSES (0 tests, harness runs)
- `cargo fmt --check`: PASSES
- `cargo clippy`: PASSES

### Upstream Audit Logs Reviewed
- **`audit_log_stage_00.md`:** No BLOCKING or WARNING findings. 4 NOTE-level observations, all resolved. User green light received and recorded.

### Upstream Downstream Logs Reviewed
- **`downstream_log_stage_00.md`:** Key facts:
  - `freyja-engine` is the only Cargo workspace member
  - All 7 modules are empty stubs — no types, functions, or traits
  - Binary prints protocol header and exits
  - Edition 2024, Rust 1.93.1
  - `tracing` dependency present but not instrumented
  - Open question: `movegen` vs `move_gen` naming (NOTE severity, deferred)

### Findings from Upstream
- No blocking issues from Stage 0.
- Module naming question (`movegen` vs `move_gen`) is NOTE severity — does not affect Stage 1 (`board` module).

### Risks for This Stage
1. **Corner square off-by-one errors:** The 4 corners use ranks/files 0-2 and 11-13. Off-by-one in boundary checks (e.g., `< 3` vs `<= 2`, `> 10` vs `>= 11`) is the most likely bug. Mitigated by exhaustive test of all 36 invalid indices from 4PC_RULES_REFERENCE.
2. **Pawn attack direction reversal:** Each player has unique capture diagonals. Blue captures NE/SE, not NE/NW like Red. Easy to confuse. Mitigated by per-player tests with explicit expected squares.
3. **Blue/Yellow King-Queen swap:** These players have K-Q in swapped positions vs Red/Green. Starting position tests must verify exact squares per 4PC_RULES_REFERENCE Section 3.5.
4. **FEN4 format undefined:** No standard FEN4 exists. Must design a format and ensure round-trip correctness. Risk: ambiguous encoding. Mitigated by explicit format definition and 10+ round-trip tests.
5. **Zobrist key quality:** 4,688 keys from a simple PRNG. Risk: accidental duplicates or zeros. Mitigated by uniqueness and nonzero tests.

---

## Post-Audit

**Date:** 2026-03-03
**Session:** 3

### Build State
- `cargo build`: PASSES (0 warnings)
- `cargo test`: PASSES (89 tests, 0 failures)
- `cargo fmt --check`: PASSES
- `cargo clippy`: PASSES (0 warnings)

### Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `Square(u8)` with rank/file accessors | PASS | `types.rs`: `Square(u8)`, `rank()`, `file()`, `from_index()`, `from_notation()`, `to_notation()` |
| 36 invalid corners identified | PASS | `test_all_36_invalid_squares`: all 36 indices verified, `VALID_SQUARES = 160` |
| `Board` struct with fixed-size arrays | PASS | `mod.rs`: `squares: [Option<Piece>; 196]`, `piece_lists: [[(Option<(PieceType, Square)>); 32]; 4]`, no `Vec<T>` |
| Piece list management (O(1) add/remove) | PASS | `add_to_piece_list()`, `remove_from_piece_list()` (swap-remove), `test_piece_list_sync_starting_position` |
| Starting position correct for all 4 players | PASS | 4 dedicated tests verify all piece placements per 4PC_RULES_REFERENCE Section 3 |
| King squares tracked | PASS | `king_squares[4]` updated in `set_piece()`/`remove_piece()`, verified for all 4 players |
| Zobrist hashing (incremental) | PASS | `set_piece()`/`remove_piece()` XOR keys; `test_zobrist_starting_position_matches_full_hash`, `test_zobrist_consistency_after_many_operations` (20 pieces) |
| Zobrist deterministic, no duplicates, nonzero | PASS | `test_zobrist_deterministic`, `test_zobrist_no_duplicates`, `test_zobrist_all_nonzero` |
| FEN4 parse and serialize | PASS | 14 round-trip tests covering: starting position, empty board, single piece, all 4 sides to move, EP, partial/no castling, promoted queen, minimal, all piece types |
| FEN4 Zobrist matches after parse | PASS | `test_fen4_zobrist_matches_after_parse` |
| Attack query: `is_square_attacked_by()` | PASS | Tests for rook, bishop, queen, knight, king, pawn attacks |
| Attack query: `attackers_of()` | PASS | `test_attackers_of_multiple_pieces` |
| Attack query: `is_in_check()` | PASS | `test_is_in_check_by_rook`, `test_is_in_check_three_opponent_check`, `test_is_in_check_eliminated_returns_false` |
| Pawn attacks for all 4 player orientations | PASS | 8 directional tests + 4 negative tests (Red NE/NW, Blue NE/SE, Yellow SE/SW, Green NW/SW) |
| Knight/slider near corners | PASS | `test_knight_near_ne/nw/se_corner`, `test_knight_attack_near_sw_corner`, `test_slider_blocked_by_ne/sw_corner` |
| Starting position: no player in check | PASS | `test_starting_position_no_player_in_check` |

### 4PC Verification Matrix — Stage 1

| Test | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Starting position pieces correct | PASS | PASS | PASS | PASS |
| King square index | PASS (7) | PASS (84) | PASS (188) | PASS (111) |
| Piece count = 16 | PASS | PASS | PASS | PASS |
| Pawn capture direction 1 | PASS (NE) | PASS (NE) | PASS (SE) | PASS (NW) |
| Pawn capture direction 2 | PASS (NW) | PASS (SE) | PASS (SW) | PASS (SW) |
| Pawn negative direction | PASS (no S) | PASS (no W) | PASS (no N) | PASS (no E) |
| Zobrist side-to-move distinct | PASS | PASS | PASS | PASS |

### Findings

| ID | Severity | Description |
|----|----------|-------------|
| S01-F01 | NOTE | `update_piece_list_square()` is defined but not yet called — marked `#[allow(dead_code)]`. Will be used by Stage 2 make/unmake. |
| S01-F02 | NOTE | `movegen` vs `move_gen` naming question carried from Stage 0. Does not affect Stage 1. |
| S01-F03 | NOTE | FEN4 format is custom-designed (no standard exists). Format documented in `fen4.rs` module doc. Round-trip tested with 14 positions. |

### Risks Retrospective
1. **Corner off-by-one:** Mitigated successfully. Simplified boolean expression passes clippy, all 36 invalid squares verified.
2. **Pawn direction reversal:** Mitigated. Per-player tests with explicit squares catch direction errors.
3. **Blue/Yellow K-Q swap:** Mitigated. Starting position tests verify exact squares for all 4 players.
4. **FEN4 format:** Multi-digit empty count parsing bug found and fixed during implementation. Now covered by round-trip tests.
5. **Zobrist quality:** All 4,688 keys verified unique and nonzero.
