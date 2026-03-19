# Audit Log — Stage 14: MCTS Opponent Move Abstraction (OMA)

## Pre-Audit

**Date:** 2026-03-18
**Auditor:** Agent (Session 21)

### Build State
- `cargo build`: PASS
- `cargo test --lib`: 399 passed, 0 failed
- `cargo clippy`: 1 warning (unused constant in hybrid.rs — cosmetic, pre-existing)

### Upstream Logs Reviewed

| Log | Key Findings |
|-----|-------------|
| downstream_log_stage_10.md | MCTS uses own tree (no TT). Gumbel root selection. Progressive widening at non-root. Prior policy from ordering scores. 2M node cap. |
| downstream_log_stage_11.md | HybridSearcher selects Max^n OR MCTS by phase. Opening < cutover ply → Max^n. Midgame+ → MCTS. No inter-phase transfer in current design. |
| downstream_log_stage_13.md | Opponent beam ratio 0.25 default. MoveNoise + NoiseSeed for diversity. Qsearch soft budget 2M. ID time management. 256MB stack thread. |
| audit_log_stage_13.md | (Not yet created — Stage 13 audit was informal) |

### Findings from Upstream

1. **MCTS progressive widening already exists at non-root nodes** (pw_k=2.0, pw_alpha=0.5). OMA replaces this for opponent nodes — opponent nodes get exactly 1 move via OMA policy instead of progressive widening. Root-player nodes keep full expansion + Gumbel selection.
2. **MctsNode.children is Vec** — dynamic tree. OMA won't add children for opponent moves (tree pointer stays put), so no memory impact.
3. **History table optionally shared** via `set_history_table()`. OMA policy can use this for move selection priority #3 (after captures and checks).

### Risks for This Stage

1. **Board state consistency after OMA moves.** OMA makes moves on the board without creating tree nodes. Must unmake ALL OMA moves (not just tree moves) during backpropagation. SimStep enum addresses this.
2. **Tree semantics change.** Tree nodes now represent root-player decisions, not all-player decisions. This changes what `expand()` sees — after OMA advancement, side-to-move should be root player. Edge case: if root player gets eliminated mid-OMA, need to handle gracefully.
3. **Check detection overhead.** Benchmarked at 1.9x per OMA decision (7.6µs vs 4.0µs), but only 0.2% of total search budget. Acceptable.
4. **Spec deviation: none.** Originally planned to skip checks, but benchmark showed acceptable overhead. Including checks per MASTERPLAN spec.

### Baseline Metrics

| Metric | Value |
|--------|-------|
| MCTS tests | 41 pass (within 399 total) |
| Self-play stability | 100+ games at d4, 0 crashes |
| MCTS sims/search | ~50 (debug, 50-node limit) |

### Critical Discovery: OMA Tree Consistency

**Issue:** The initial OMA implementation (zobrist-seeded deterministic RNG for opponent moves) failed after ~25 simulations with board state corruption ("Cannot remove piece from empty square" / zobrist hash mismatch). The root cause: OMA fundamentally abstracts over opponent moves (Baier & Kaisers 2020), meaning different simulations may produce different opponent outcomes at the same tree node. Using deterministic RNG didn't solve this because the hash drift accumulated over deep tree traversals.

**Resolution:** Store OMA moves at each tree node on first visit (`oma_moves: ArrayVec<Move, 3>`, `oma_computed: bool`). On subsequent visits, replay the stored moves instead of recomputing. This guarantees the same board state at each tree node across all simulations, at a cost of only ~13 bytes per node.

**Additional fix:** After replaying stored OMA moves, must check if side_to_move is still an opponent (e.g., eliminated player broke early during first computation). If so, break to evaluate instead of continuing (which would cause infinite loop).

**Research basis:** [Guiding Multiplayer MCTS by Focusing on Yourself](https://consensus.app/papers/details/bedf5fed7a73500f9e062fb6dc11001c/) (Baier & Kaisers, IEEE CoG 2020); [MultiTree MCTS in Tabletop Games](https://consensus.app/papers/details/ff88cdb30f23558a97fe5d33b6e4be5a/) (Goodman et al., 2022) — confirms poor opponent modelling cost increases with budget.

---

## Post-Audit

*(To be completed after Stage 14 implementation)*
