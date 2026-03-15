# Downstream Log -- Stage 10: MCTS (Gumbel MCTS)

## Must-Know

1. **MCTS uses its own tree — no TT interaction.** MctsSearcher does not use TranspositionTable. Tree nodes store visit counts and score sums directly.
2. **Gumbel root selection, not UCB1.** Root candidates selected by `g(a) + log(pi(a))` where g ~ Gumbel(0,1). Sequential Halving eliminates weak candidates across rounds.
3. **Non-root selection uses Q + prior + PH formula.** `Q[player]/N + C_PRIOR * pi/(1+N) + PH(a)` where player = side-to-move at parent (NOT root player).
4. **Score sums are f64.** Accumulated as f64 for precision, converted to i16 only at output. Q-value = score_sums[player] / visits.
5. **Memory bounded by configurable node cap.** Default `max_nodes = 2_000_000`. When cap hit, stops expanding new nodes — searches through existing tree only (graceful degradation, no panic).
6. **Progressive widening at non-root.** `max_children = floor(pw_k * visits^pw_alpha)`. Default pw_k=2.0, pw_alpha=0.5 means sqrt(N)*2 children available.
7. **Prior policy from ordering scores.** `pi(a) = softmax(score_move(a) / PRIOR_TEMPERATURE)`. Reuses Max^n move ordering infrastructure (MVV-LVA, killers, history).

## API Contracts

### New Module: `mcts.rs`

```rust
// ── Configuration ──
pub struct MctsConfig {
    pub gumbel_k: usize,        // 16 — Top-k root candidates
    pub prior_temperature: f32,  // 50.0 — softmax temperature
    pub c_prior: f32,            // 1.5 — prior coefficient
    pub ph_weight: f32,          // 1.0 — progressive history weight
    pub pw_k: f32,               // 2.0 — progressive widening constant
    pub pw_alpha: f32,           // 0.5 — progressive widening exponent
    pub max_nodes: usize,        // 2_000_000 — memory bound
}

// ── Searcher ──
pub struct MctsSearcher<E: Evaluator> { /* ... */ }

impl<E: Evaluator> MctsSearcher<E> {
    pub fn new(evaluator: E, config: MctsConfig) -> Self;
    pub fn set_rng_seed(&mut self, seed: u64);
    pub fn set_prior_policy(&mut self, priors: Vec<f32>);      // Stage 11 injection
    pub fn set_history_table(&mut self, history: &HistoryTable); // Stage 11 injection (ADR-007)
}

impl<E: Evaluator> Searcher for MctsSearcher<E> {
    fn search(&mut self, state: &mut GameState, limits: &SearchLimits) -> SearchResult;
}
```

### Stage 11 Integration Points

```rust
// After Max^n Phase 1, the hybrid controller should:
// 1. Extract history table from Max^n
let history = maxn_searcher.history_table();

// 2. Compute prior policy from ordering scores
// (or use a standalone compute_prior_policy function)

// 3. Inject into MCTS before Phase 2
mcts_searcher.set_history_table(history);
mcts_searcher.set_prior_policy(priors);

// 4. Run MCTS with remaining time budget
let result = mcts_searcher.search(&mut state, &remaining_limits);
```

### SearchResult from MCTS

- `best_move`: Sequential Halving winner (most robust selection)
- `scores`: Q-vector from winner node (score_sums/visits → i16)
- `depth`: Always 0 (MCTS depth is not comparable to iterative deepening)
- `nodes`: Number of simulations completed
- `qnodes`: Always 0 (no quiescence in MCTS)
- `pv`: Single-move PV (just the best move — MCTS doesn't have a natural PV)
- `tt_hit_rate`, `killer_hit_rate`: Always 0.0 (MCTS doesn't use TT/killers)

## Known Limitations

1. **No parallel MCTS.** Single-threaded. Parallelism deferred to Stage 20.
2. **No persistent tree between moves.** Tree is rebuilt each search. Persistence deferred to Stage 13 measurement.
3. **PV is single move.** MCTS doesn't produce a natural principal variation. Could extract path of most-visited children, but not implemented yet.
4. **Prior policy uses empty killers/history.** Standalone MCTS computes priors with fresh (empty) killer and history tables. Full prior quality requires Stage 11 hybrid injection.
5. **No RAVE/AMAF.** Intentionally excluded — move permutation assumptions fail in multi-player games.
6. **Gumbel noise is LCG-based.** Not cryptographic quality, but sufficient for MCTS exploration.

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| MCTS unit tests | 36 tests pass | All acceptance criteria covered |
| Simulations per search (50-node limit) | ~50 | Starting position, debug build |
| Memory per node | ~80-100 bytes | MctsNode + Vec overhead for children |
| Max tree size (2M nodes) | ~200MB | Generous default, tunable via MctsConfig |

## Open Questions

1. Should Stage 11 hybrid controller use MctsConfig defaults or derive them from search time budget?
2. Should MCTS info output use a distinct format from Max^n info strings, or share the same protocol format?
3. Should `depth` in SearchResult report tree depth (max depth of any simulation path) instead of 0?

## Reasoning

- **f64 score sums over i16:** 10,000 visits × 10,000cp max = 100M, which overflows i16. f64 gives headroom for millions of visits without precision loss.
- **Vec<MctsNode> for children:** Arena allocation is more cache-friendly but adds complexity. Vec is simple and sufficient for Stage 10. Optimize in Stage 20 if profiling shows allocation bottleneck.
- **2M node cap (not 500K):** User feedback: "don't make strong bottlenecks this early on." 2M is generous; tune in Stage 13 with actual measurement.
- **No TT for MCTS:** MCTS identifies positions by tree location, not hash. Transposition detection in MCTS trees is complex (DAG merging) and deferred.
- **LCG for Gumbel:** Avoids adding `rand` crate dependency. LCG quality is sufficient — Gumbel sampling only needs roughly uniform values for exploration noise.
