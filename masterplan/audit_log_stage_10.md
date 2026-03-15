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

### Build State (Final)
- `cargo build -p freyja-engine` — PASS
- `cargo test -p freyja-engine --lib` — PASS (355 unit tests)
- `cargo clippy` — PASS (0 warnings)
- MCTS-specific tests: 41/41 pass (36 original + 5 additional AC tests)

### Implementation Verification

| Build Step | Status | Verified By |
|---|---|---|
| 1. Node struct + config | Complete | `test_mcts_config_defaults`, `test_mcts_node_root`, `test_mcts_node_child`, `test_q_value_*` |
| 2. Prior policy | Complete | `test_prior_policy_sums_to_one`, `test_prior_policy_captures_higher`, `test_prior_policy_empty_moves` |
| 3. Gumbel sampling | Complete | `test_lcg_*`, `test_gumbel_samples_finite`, `test_gumbel_topk_*` |
| 4. Sequential Halving | Complete | `test_halving_*`, `test_ac9_halving_reduces_to_winner` |
| 5. Non-root tree policy | Complete | `test_select_child_unvisited_high_prior_first`, `test_select_child_q_dominates_after_visits` |
| 6. Expansion | Complete | `test_single_simulation_updates_visits` |
| 7. Evaluation | Complete | `test_score_vector_per_player_independent` |
| 8. Backpropagation | Complete | `test_backpropagation_accumulates_scores`, `test_ac7_score_vector_components_independent` |
| 9. Root move selection | Complete | `test_mcts_returns_legal_move` |
| 10. Progressive widening | Complete | `test_progressive_widening_limits_children`, `test_progressive_widening_increases_with_visits` |
| 11. Searcher trait + time mgmt | Complete | `test_mcts_respects_node_limit`, `test_mcts_board_preserved_after_search` |
| 12. Stage 11 injection | Complete | `test_set_prior_policy`, `test_set_history_table_enables_ph`, `test_ph_decays_with_visits` |
| 13. Info output | Complete | tracing::info + tracing::debug in mcts_search() |

### Acceptance Criteria

| AC | Description | Status | Test |
|---|---|---|---|
| AC1 | 2-sim quality | PASS | `test_ac1_two_simulations_returns_legal_move` |
| AC2 | 100+ sim quality | PASS | `test_ac2_100_sims_returns_reasonable_move` |
| AC3 | Mate-in-1 | PARTIAL | `test_ac3_finds_capture_when_obvious` — full mate-in-1 needs FEN4 position setup (not yet available in test harness) |
| AC4 | SPS > 10K | PASS (release) | `test_ac4_sps_performance` — debug mode verifies ≥10 sims in 500ms; release target verified manually |
| AC5 | Memory bounded | PASS | `test_ac5_memory_cap_graceful_degradation` — cap hit + graceful degradation verified |
| AC6 | Eliminated players | PASS | `test_ac6_eliminated_player_search` — search with 2 eliminated players completes without panic |
| AC7 | Score vector backprop | PASS | `test_ac7_score_vector_components_independent` |
| AC8 | PH warm-start | PASS | `test_ac8_ph_warmstart_affects_search` |
| AC9 | Sequential Halving | PASS | `test_ac9_halving_reduces_to_winner` |

### Findings

- **S10-F01 (NOTE):** AC3 (mate-in-1) is only partially testable without a FEN4 test setup helper. The current test verifies MCTS produces legal moves and doesn't corrupt state with 200 sims. A true mate-in-1 test requires constructing a specific board position, which will be straightforward once FEN4 parsing matures.
- **S10-F02 (NOTE):** The `is_multiple_of` method used in the simulation loop requires nightly Rust or a recent stable version. If this causes issues on older toolchains, replace with `% TIME_CHECK_INTERVAL == 0`.
- **S10-F03 (NOTE):** Pre-existing uncommitted changes in `attacks.rs` and `move_gen.rs` (from another session) change slider corner handling from `break` to `continue`. These are orthogonal to MCTS and should be reviewed/committed separately.

### Maintenance Invariants Check

| Invariant | Status |
|---|---|
| Prior-stage tests not deleted | PASS — 314 non-MCTS tests unmodified |
| Perft values unchanged | PASS — perft tests not modified |
| Board state preserved after search | PASS — `test_mcts_board_preserved_after_search` |
| Evaluator trait boundary respected | PASS — calls `eval_4vec()` via trait |
| Searcher trait boundary respected | PASS — `impl Searcher for MctsSearcher<E>` |
| No Vec in Board/GameState/MoveUndo | PASS — `Vec<MctsNode>` only in tree nodes (acceptable per ADR) |
| NPS regression check | N/A — MCTS uses SPS metric, not NPS |
