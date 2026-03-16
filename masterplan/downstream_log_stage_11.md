# Downstream Log -- Stage 11: Max^n → MCTS Integration (Hybrid Controller)

## Must-Know

1. **Protocol now uses HybridSearcher, not MaxnSearcher.** Every `go` command runs the hybrid pipeline: Max^n Phase 1 → knowledge transfer → MCTS Phase 2.
2. **50/50 time split is hardcoded.** Max^n gets 50% of the time budget, MCTS gets the remainder. Adaptive splitting deferred to Stage 13.
3. **Depth-only searches bypass MCTS entirely.** If `SearchLimits` has no `max_time_ms` and `infinite=false`, HybridSearcher delegates to Max^n only.
4. **Mate detection skips MCTS.** If Max^n's root player score >= 9000cp, MCTS Phase 2 is skipped and Max^n's result is returned directly.
5. **MCTS best move is the final choice.** The merged SearchResult uses MCTS's Sequential Halving winner as `best_move`, but Max^n's depth, PV, TT stats, and qnodes.
6. **Evaluator must implement Clone.** `HybridSearcher<E: Evaluator + Clone>` clones the evaluator for each sub-searcher. BootstrapEvaluator now derives Clone. Future evaluators (NNUE, Stage 17) must also implement Clone.
7. **HybridSearcher is created fresh per `go` command.** No persistent state across moves. History and TT are rebuilt each search. Persistence deferred to Stage 13.

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

| Field | Source | Notes |
|---|---|---|
| `best_move` | MCTS (Sequential Halving winner) | Falls back to Max^n if MCTS returns None |
| `scores` | MCTS | Q-vector from winner node |
| `depth` | Max^n | Meaningful iterative deepening depth |
| `nodes` | Max^n + MCTS | Combined node count from both phases |
| `qnodes` | Max^n | MCTS has no quiescence |
| `pv` | Max^n | Multi-move PV (MCTS only has single-move) |
| `tt_hit_rate` | Max^n | MCTS doesn't use TT |
| `killer_hit_rate` | Max^n | MCTS doesn't use killers |

### Knowledge Transfer Flow

```rust
// Phase 1: Max^n with 50% time
let maxn_result = self.maxn.search(state, &maxn_limits);

// Phase 1.5: Extract + compute
let history = self.maxn.history_table();           // &HistoryTable
let priors = compute_hybrid_priors(&moves, history, player, 50.0);  // Vec<f32>

// Phase 2: Inject + run MCTS with remaining time
self.mcts.set_history_table(history);
self.mcts.set_prior_policy(priors);
let mcts_result = self.mcts.search(state, &mcts_limits);
```

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

1. **Fixed 50/50 time split.** Not adaptive. Optimal ratio likely depends on position complexity. Stage 13 should tune this.
2. **No persistent history across moves.** History is rebuilt each search. Persisting Max^n history between moves could give MCTS a head start. Stage 13 should measure impact.
3. **AC8 not fully verified.** Warm-start vs cold-start comparison requires self-play A/B testing (Stage 12). Current test only verifies both phases run and produce combined results.
4. **No training data export.** Hybrid search data (disagreements, phase timings) could be valuable for NNUE training. Deferred to future work.
5. **Softmax prior computation duplicated.** `compute_hybrid_priors()` in hybrid.rs and `compute_prior_policy()` in mcts.rs implement the same algorithm. Could be extracted to `move_order.rs` if a third consumer appears.

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Hybrid tests | 21/21 pass | AC1-AC8 all covered |
| Hybrid overhead | <1ms | Softmax + 150KB memcpy per search |
| Phase 1 depth (starting pos, 2.5s debug) | ~4 | Debug build, MIN_SEARCH_DEPTH=4 |
| Combined nodes (3s debug) | >100 | Both phases contribute |

## Open Questions

1. Should Stage 12 (self-play) use HybridSearcher or allow choosing between MaxnSearcher/MctsSearcher/HybridSearcher per player?
2. Should disagreement data be exported for analysis in Stage 13?
3. Should the time split become a protocol option (`setoption name TimeSplit value 0.5`)?

## Reasoning

- **50/50 split over other ratios:** Simplest default. No empirical data yet to prefer any other ratio. Stage 13 will tune based on self-play results.
- **MCTS best move over Max^n best move:** MCTS with Gumbel Sequential Halving is designed for robust selection. Max^n provides tactical grounding that informs MCTS via priors and history, but MCTS's exploration covers strategic depth Max^n misses.
- **Fresh per `go` over persistent:** Avoids cross-move state bugs. Persistence requires careful cache invalidation (position changed, player eliminated, etc.). Simple and correct first; optimize later.
- **Clone bound over Arc<E>:** Clone is simpler and zero-cost for unit structs. Arc adds indirection and atomic reference counting overhead. Revisit in Stage 17 if NNUE evaluator is expensive to clone.
