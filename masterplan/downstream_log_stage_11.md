# Downstream Log -- Stage 11: Phase-Separated Hybrid Controller

## Must-Know

1. **Protocol uses HybridSearcher, not MaxnSearcher.** The hybrid controller selects Max^n OR MCTS based on game phase — never both on the same move.
2. **Opening (ply < cutover): Max^n ONLY.** Full time budget goes to Max^n. MCTS is never called. Opening moves complete in <1s at depth 4.
3. **Midgame+ (ply >= cutover): MCTS ONLY.** Full time budget goes to MCTS. Max^n is never called. MCTS handles the chaotic multi-player branching through sampling.
4. **Phase cutover is configurable.** `setoption name PhaseCutoverPly value 32` (default 32 = 8 moves per player = ~2 full opening cycles).
5. **Mate detection in opening.** If Max^n found mate (score >= 9000cp), return immediately.
6. **Evaluator must implement Clone.** `HybridSearcher<E: Evaluator + Clone>` clones the evaluator for each sub-searcher. BootstrapEvaluator now derives Clone. Future evaluators (NNUE, Stage 17) must also implement Clone.
7. **HybridSearcher is created fresh per `go` command.** No persistent state across moves.

**CORRECTED (2026-03-18):** The original Stage 11 design ran both Max^n and MCTS sequentially on every move with a 50/50 time split. This was wrong — MCTS is weak at openings (wastes time on positions it doesn't understand), and Max^n is too shallow for midgame (can't search deep enough to matter). The corrected design uses phase separation: each algorithm runs alone in the phase where it excels.

## API Contracts

### New Module: `hybrid.rs`

```rust
// ── Configuration ──
pub struct HybridConfig {
    pub maxn_config: SearchConfig,      // Max^n settings (beam width, TT size, etc.)
    pub mcts_config: MctsConfig,        // MCTS settings (gumbel_k, temperature, etc.)
    pub time_split_ratio: f32,          // 0.5 — fraction of time for Max^n
    pub mate_skip_threshold: i16,       // 9000 — skip MCTS if score >= this
}

// ── Searcher ──
pub struct HybridSearcher<E: Evaluator + Clone> { /* ... */ }

impl<E: Evaluator + Clone> HybridSearcher<E> {
    pub fn new(evaluator: E, config: HybridConfig) -> Self;
    pub fn disagreement_rate(&self) -> f64;  // fraction where Max^n ≠ MCTS
}

impl<E: Evaluator + Clone> Searcher for HybridSearcher<E> {
    fn search(&mut self, state: &mut GameState, limits: &SearchLimits) -> SearchResult;
}
```

### SearchResult from Hybrid

The result comes from whichever phase handled the move:

| Game Phase | Source | Notes |
|---|---|---|
| Opening (ply < cutover) | Max^n | depth, scores, pv, nodes, qnodes, TT/killer stats |
| Midgame+ (ply >= cutover) | MCTS | best_move from Sequential Halving, depth=0, nodes from simulations |

### Phase Selection Flow

```rust
fn search(&mut self, state: &GameState, limits: &SearchLimits) -> SearchResult {
    let ply = state.ply_count();

    if ply < self.config.phase_cutover_ply {
        // Opening: Max^n only, full time budget
        self.maxn.search(state, limits)
    } else {
        // Midgame+: MCTS only, full time budget
        self.mcts.search(state, limits)
    }
}
```

**Note:** The original design included a "knowledge transfer flow" where Max^n's history table and prior policy were passed to MCTS via `set_history_table()` and `set_prior_policy()`. This is no longer needed because the phases don't overlap. MCTS uses its own lightweight move ordering (MVV-LVA, history from previous MCTS searches) for priors.

### Tracing Points

All tracing uses `tracing::info!`:
1. **Phase 1 complete:** depth, nodes, time_ms, best_move, scores
2. **Mate skip:** score, threshold (when triggered)
3. **History extracted:** nonzero_entries count
4. **Prior policy computed:** num_moves, num_priors, entropy
5. **Insufficient time:** remaining_ms (when Phase 2 skipped)
6. **Phase 2 complete:** sims, time_ms, best_move, scores
7. **Move selection:** maxn_move, mcts_move, disagree, disagreement_rate, total_searches

## Known Limitations

1. **Hard cutover between phases.** The transition from Max^n to MCTS is a sharp boundary at `phase_cutover_ply`. A future refinement could add a transition window where Max^n runs at reduced depth to seed MCTS priors, but the hard cutover is simpler and solves the immediate problem.
2. **MCTS starts cold in midgame.** Without Max^n warming it up, MCTS relies on its own history accumulation and lightweight move ordering for priors. This is acceptable — MCTS is designed to work from cold start.
3. **Cutover ply is fixed, not adaptive.** The optimal cutover point likely depends on position complexity and opening type. Future work could detect opening completion dynamically (e.g., when development score plateaus).

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Hybrid tests | 21/21 pass | AC1-AC8 all covered |
| Hybrid overhead | <1ms | Softmax + 150KB memcpy per search |
| Phase 1 depth (starting pos, 2.5s debug) | ~4 | Debug build, MIN_SEARCH_DEPTH=4 |
| Combined nodes (3s debug) | >100 | Both phases contribute |

## Open Questions

1. Should the phase cutover be adaptive (detect opening completion dynamically) rather than fixed ply count?
2. Should there be a brief transition window where both phases contribute, or is a hard cutover sufficient?

## Reasoning

- **Phase separation over sequential pipeline:** MCTS is weak at openings — it wastes time sampling positions it doesn't understand. Max^n is too shallow for midgame — depth 4 is one cycle, not enough to see strategic patterns. Each algorithm should run alone in the phase where it excels.
- **Hard cutover over soft transition:** Simpler implementation, easier to test, solves the immediate problem (5s MCTS burn on opening moves). Soft transition can be added later if A/B data shows a gap.
- **Default cutover at ply 32:** 8 moves per player = ~2 full opening cycles. By this point pieces are developed, the position is complex, and MCTS's sampling approach becomes more valuable than Max^n's exhaustive search.
- **Fresh per `go` over persistent:** Avoids cross-move state bugs. Persistence requires careful cache invalidation (position changed, player eliminated, etc.). Simple and correct first; optimize later.
- **Clone bound over Arc<E>:** Clone is simpler and zero-cost for unit structs. Arc adds indirection and atomic reference counting overhead. Revisit in Stage 17 if NNUE evaluator is expensive to clone.
