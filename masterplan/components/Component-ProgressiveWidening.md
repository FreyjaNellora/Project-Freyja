# Component: Progressive Widening (PW)

**Stage Introduced:** Stage 10 (infrastructure), Stage 15 (activated)
**Last Updated:** 2026-03-20
**Module:** `freyja_engine::mcts`

---

## Purpose

Limits the number of children considered at root-player MCTS nodes, growing with visit count. Combined with OMA (which handles opponent nodes), PW reduces tree width while OMA reduces tree depth. Together they allow MCTS to reach deeper root-player decisions within the same simulation budget.

## Public API

```rust
// MctsConfig fields
pub pw_k: f32,      // Widening constant (default 2.0)
pub pw_alpha: f32,   // Widening exponent (default 0.5)

// MctsNode method
fn available_children(&self, pw_k: f32, pw_alpha: f32) -> usize
// Returns: floor(pw_k * visits^pw_alpha), clamped to [1, children.len()]

// Setoptions
// PWConstant: positive float (default 2.0)
// PWExponent: float in [0.0, 1.0] (default 0.5)
```

## Internal Design

**Formula:** `available = floor(k * N^alpha)` where N = visit count.

**Children sorted by prior descending** after expansion (`expand()`), so the PW window always exposes the highest-prior (best-ranked) moves first.

**Flow:** With OMA on, opponent nodes never reach `select_child()` (they use the OMA path). PW naturally applies only to root-player tree nodes. With OMA off, PW applies to all expanded nodes.

**Diagnostics:**
- `tree_moves_total`: all tree moves across simulations
- `root_player_decisions`: only moves where side_to_move == root player
- `pw_limited_selections`: times PW restricted selection (available < total children)

## Performance Characteristics

- Zero overhead: `available_children()` is a single `powf` + clamp per selection
- Child sort in `expand()`: one-time O(n log n) per node expansion

## Known Limitations

- k=2 may be too conservative (only 2 children at first visit). A/B test k=2 vs k=4 pending.
- PW ordering depends on prior quality (softmax over MVV-LVA + killers + history). Bad priors = bad PW.
- No PW at root (root uses Gumbel Top-k, which is effectively PW with different selection).

## Dependencies

- **Consumes:** `compute_prior_policy()` for child ordering, `MctsConfig` for parameters
- **Consumed By:** `MctsSearcher::select_child()`, `MctsSearcher::run_simulation()`

---

**Related:** [[MASTERPLAN]], [[ADR-019]], [[ADR-018]], [[Component-MCTS]]
