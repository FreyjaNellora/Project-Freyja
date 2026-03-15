# Audit Log — Stage 10: MCTS (Gumbel MCTS)

**Date:** 2026-03-14
**Auditor:** Session 16

---

## Pre-Audit

### Build State
- `cargo build -p freyja-engine` — PASS
- `cargo test -p freyja-engine` — PASS (344 tests)
- `cargo clippy` — PASS (0 warnings)

### Upstream Logs Reviewed
- **Stage 9 audit:** TT + move ordering verified. TT hit rate ~4-5% at starting position with beam 30. Max^n TT is exact-only. NPS ~89.7k depth 5 release.
- **Stage 9 downstream:** All recursive search functions are `&mut self`. TT exact-only in Max^n. `beam_select()` accepts `tt_move` and `ply`. History table extractable via `pub fn history_table(&self) -> &HistoryTable` (ADR-007). Captures sorted by MVV-LVA in quiescence.
- **Stage 7 downstream:** `Searcher` trait is the search boundary. `Score4 = [i16; 4]`. Beam search, negamax fallback, PV tracking, iterative deepening.
- **Stage 8 downstream:** `generate_captures_only()`, `MIN_SEARCH_DEPTH = 4`, `qnodes` tracking.
- **Stage 6 downstream:** `Evaluator` trait with `eval_scalar` and `eval_4vec`. `ELIMINATED_SCORE = i16::MIN`.

### Findings from Upstream
- **S09-F01 (NOTE):** TT hit rate low with wide beam — MCTS does not use TT, so no impact.
- **S09-F02 (NOTE):** Max^n TT exact-only — MCTS has its own tree, no TT interaction.
- **S09 downstream open question:** "Should Stage 10 (MCTS) use the same TT, or a separate one?" — Answer: MCTS uses its own tree structure, no TT. Separate.

### Risks for This Stage
1. **Gumbel sampling correctness:** `g = -ln(-ln(u))` has singularity at u=0,1. Mitigated by clamping.
2. **Make/unmake correctness during MCTS traversal:** Multi-ply traversal then full unmake. Mitigated by debug zobrist assertions.
3. **Memory growth:** Progressive widening + 4 players = fast tree growth. Mitigated by configurable node cap (default 2M, tunable in Stage 13).
4. **Score vector sign convention:** Selection must use current player's Q, not root player's. Mitigated by explicit tests.
5. **Eliminated player handling during simulation:** Must skip eliminated players' turns. Reuse existing `is_player_eliminated_in_search()` pattern.

### Tier Boundary Review
- Tier 2→3 boundary review completed: `masterplan/tier_boundary_review_3.md`
- All 18 maintenance invariants verified
- No blocking issues
- User sign-off received

---

## Post-Audit

*(To be completed after Stage 10 implementation)*
