# Downstream Log — Stage 14: MCTS Opponent Move Abstraction (OMA)

**Author:** Agent (Sessions 21-22)
**Date:** 2026-03-20

---

## Must-Know

1. **OMA is on by default.** `MctsConfig::use_oma = true`. Opponent nodes in MCTS use a lightweight policy instead of full tree expansion. Toggle via `setoption name OpponentAbstraction value false`.
2. **OMA moves are stored per tree node.** First visit computes OMA moves, subsequent visits replay them. This guarantees board state consistency. `MctsNode` gained `oma_moves: ArrayVec<Move, 3>` and `oma_computed: bool` (~13 bytes per node).
3. **OMA policy priority: checkmate > capture > check > history > random.** Never calls the evaluator. Uses `generate_captures_only`, `generate_legal_moves`, `make_move`/`is_in_check`/`unmake_move` for check/mate detection.
4. **Sigma transform fixed.** `sigma(x) = x / (c_scale + |x|)` where `c_scale = 200.0`. Previously `sigma(x) = x / (1 + |x|)` which saturated on 4PC centipawn ranges (+/-2000) — Gumbel exploration was broken since Stage 10.
5. **Ply bounds guard added to all search functions.** `ply >= MAX_DEPTH - 1` check in qsearch, qsearch_2p, maxn, negamax prevents array overflow when eliminated-player skip doesn't increment ply.
6. **MoveNoise does NOT work in MCTS.** Only applies in Max^n iterative deepening. MCTS-only mode (PhaseCutoverPly=0) produces deterministic identical games. Use default hybrid mode for diverse self-play.

---

## API Contracts

### MctsConfig Changes
```rust
pub struct MctsConfig {
    // ... existing fields ...
    pub use_oma: bool,  // NEW — default true
}
```

### MctsNode Changes (internal)
```rust
pub struct MctsNode {
    // ... existing fields ...
    oma_moves: ArrayVec<Move, 3>,  // NEW — stored opponent moves
    oma_computed: bool,             // NEW — whether OMA was computed
}
```

### New setoption
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| OpponentAbstraction | bool | true | Enable/disable OMA in MCTS |

### New Diagnostic Metrics
- `oma_moves_total: u64` — total OMA moves across all simulations
- `root_decisions_total: u64` — total root-player tree decisions
- Info output: `oma_avg {:.1} root_avg {:.1}` in MCTS debug info lines

### SimStep Enum (internal to run_simulation)
```rust
enum SimStep {
    TreeMove { child_idx: usize, undo: MoveUndo },
    OmaMove { undo: MoveUndo },
}
```

### Sigma Transform Change
```rust
// BEFORE (Stage 10-13, broken):
fn sigma(x: f64) -> f64 { x / (1.0 + x.abs()) }

// AFTER (Stage 14+, correct):
const C_SCALE: f64 = 200.0;
fn sigma(x: f64) -> f64 { x / (C_SCALE + x.abs()) }
```

---

## Downstream Impacts by Stage

### Stage 15: Progressive Widening + Zone Control

- **OMA infrastructure is ready.** PW adds opponent tree nodes back selectively as visit count grows. The stored-moves-per-node design supports extending from 1 stored move to N stored moves.
- **Key question:** Should PW replace the fixed OMA moves, or extend them (keep the original OMA move and add alternatives)?
- **OMA diagnostic metrics** (oma_moves_total, root_decisions_total) provide the measurement infrastructure for validating PW's effect on root decision depth.
- **AC1 (3-4x deeper root decisions)** should be re-validated after PW is added, since OMA alone showed neutral Elo at 2s movetime.

### Stage 16+: NNUE Training Pipeline

- **Sigma c_scale=200.0 may need recalibration** when NNUE changes the centipawn value range. Currently tuned for bootstrap eval output (~+/-2000cp). If NNUE produces different magnitudes, c_scale should be adjusted proportionally.
- **OMA policy does NOT use the evaluator.** When NNUE replaces bootstrap eval, OMA is unaffected. However, Stage 15 PW may want to use NNUE scores for opponent move prioritization.

### General

- **Ply bounds guard is a safety net, not a fix.** The `ply >= MAX_DEPTH - 1` guard in qsearch/maxn/negamax prevents array overflow when eliminated players are skipped without ply increment. Deep search with many eliminations (3 players eliminated) will terminate early at the guard rather than searching to full depth. This is acceptable because 3-eliminated-player positions are near game end anyway.
- **MoveNoise in MCTS remains unresolved.** MCTS relies solely on Gumbel noise for exploration diversity. For self-play training data generation, this may produce insufficient game variety in MCTS-only mode. Hybrid mode (opening Max^n with MoveNoise + midgame MCTS) provides adequate diversity.

---

## Known Limitations

1. **OMA strength not measurable at short time controls.** A/B test at 2s movetime showed no significant difference (Elo -4.8, p=0.993). The Baier & Kaisers paper shows OMA benefits increase with longer time controls and Progressive Widening (Stage 15).
2. **Check/mate detection adds ~1.9x overhead per OMA decision.** Benchmarked at 7.6us vs 4.0us without. Total impact ~0.2% of search budget. Acceptable but could be optimized with geometric check detection in Stage 15.
3. **OMA moves are fixed after first visit.** The stored-moves approach prevents the "abstraction over different opponent outcomes" that the paper describes. Progressive Widening (Stage 15) will address this by allowing multiple stored moves per node.
4. **No MCTS-specific noise mechanism.** MoveNoise only works in Max^n. MCTS relies on Gumbel noise for exploration, which now works correctly after the sigma fix.
5. **AC1 (3-4x deeper root decisions) not fully verified.** Metrics are tracked but need longer time control testing to confirm the depth improvement.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| MCTS tests | 54 pass (13 new) | All OMA + pre-existing |
| Full test suite | 408 pass | 0 failures, 24 new tests total |
| A/B: OMA on vs off | Elo -4.8, p=0.993 | Not significant at 2s movetime |
| A/B: winner diversity | Red 6, Blue 4, Yellow 9, Green 1 | 20 games, no crashes |
| OMA check overhead | 1.9x per decision, 0.2% of search | Benchmarked from starting pos |
| OMA memory cost | ~13 bytes/node | ArrayVec<Move, 3> + bool |

---

## Open Questions

1. **Progressive Widening interaction.** Stage 15 adds PW at opponent nodes. Should PW replace stored OMA moves, or extend them (multiple stored sequences)?
2. **MCTS noise for self-play.** Without MoveNoise, MCTS-only games are deterministic. Should we add Gumbel noise seed variation per game?
3. **Sigma c_scale for non-root selection.** The c_scale=200.0 is tuned for Sequential Halving at root. `select_child` UCB uses raw Q-values — should it also use scaled sigma?
