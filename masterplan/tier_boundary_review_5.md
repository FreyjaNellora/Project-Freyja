# Tier Boundary Review — Tier 5 (Intelligence, Stages 14-17)

**Date:** 2026-03-18
**Reviewer:** Agent (Session 21)
**Required by:** AGENT_CONDUCT 1.20

---

## 1. Maintenance Invariants (MASTERPLAN Section 4.1)

| Invariant | Status |
|-----------|--------|
| Stage 0: Prior-stage tests never deleted | PASS — 399 tests, all pass |
| Stage 2: Perft values permanent | PASS — perft tests exist in integration tests |
| Stage 2: Zobrist make/unmake round-trip | PASS — tested |
| Stage 2: Attack query API is the board boundary | PASS — is_square_attacked_by used, no internal access |
| Stage 3: Game playouts complete without crashes | PASS — self-play 100+ games at d4, 0 crashes |
| Stage 3: Eliminated players never trigger movegen | PASS — is_player_eliminated_in_search guards |
| Stage 5: UI owns zero game logic | PASS — UI is pure display |
| Stage 6: Evaluator trait is the eval boundary | PASS — BootstrapEvaluator implements Evaluator |
| Stage 6: eval_scalar and eval_4vec agree | PASS — tested |
| Stage 7: Searcher trait is the search boundary | PASS — MaxnSearcher, MctsSearcher, HybridSearcher all implement Searcher |
| Stage 9: NPS does not regress >15% | PASS — 69k NPS at d5 (opp ratio 0.25) |

**Result: ALL invariants pass.**

---

## 2. Open Issues (MOC-Active-Issues)

| Issue | Severity | Status | Impact on Tier 5 |
|-------|----------|--------|-------------------|
| Issue-UI-Feature-Gaps | WARNING | Open (stale, Session 10) | Not blocking. Debug Console needed for Stages 18+, not 14-17. |
| Issue-Depth4-Engine-Crash | NOTE | Resolved by opp beam ratio 0.25 | No impact. |

**Staleness update needed:** Issue-UI-Feature-Gaps last updated Session 10, now Session 21 (11 sessions). Per AGENT_CONDUCT 1.9, this needs a status comment. Will update.

---

## 3. Fixed-Size Data Structures in Hot Paths

| Struct | Vec usage? | Status |
|--------|-----------|--------|
| Board | No (tests/FEN4 parsing only) | PASS |
| GameState | No (ArrayVec only) | PASS |
| MoveUndo | No | PASS |
| MctsNode.children | Vec<MctsNode> | ACCEPTABLE — MCTS trees are dynamic by nature, not covered by ADR-004 |

**Result: No Vec in Board, GameState, or MoveUndo. ADR-004 satisfied.**

---

## 4. Build and Test State

- `cargo build`: PASS
- `cargo test --lib`: 399 passed, 0 failed
- `cargo clippy`: 1 warning (unused constant in hybrid.rs — cosmetic)

---

## 5. Summary

Tier 4 (Measurement) is complete. All invariants hold, no blocking issues, fixed-size constraints satisfied. Ready to begin Tier 5 (Intelligence) with Stage 14.

**User sign-off: APPROVED (2026-03-18, Session 21)**
