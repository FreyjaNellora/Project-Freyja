# Downstream Log — Stage 14: MCTS Opponent Move Abstraction (OMA)

**Author:** Agent (Session 21)
**Date:** 2026-03-19

---

## Must-Know

1. **OMA is on by default.** `MctsConfig::use_oma = true`. Opponent nodes in MCTS use a lightweight policy instead of full tree expansion. Toggle via `setoption name OpponentAbstraction value false`.
2. **OMA moves are stored per tree node.** First visit computes OMA moves, subsequent visits replay them. This guarantees board state consistency. `MctsNode` gained `oma_moves: ArrayVec<Move, 3>` and `oma_computed: bool` (~13 bytes per node).
3. **OMA policy priority: checkmate > capture > check > history > random.** Never calls the evaluator. Uses `generate_captures_only`, `generate_legal_moves`, `make_move`/`is_in_check`/`unmake_move` for check/mate detection.
4. **Sigma transform fixed.** Q-values in Sequential Halving are now normalized to [0,1] across candidates. Previously `/100` which saturated the sigmoid — Gumbel exploration was broken since Stage 10.
5. **MoveNoise does NOT work in MCTS.** Only applies in Max^n iterative deepening. MCTS-only mode (PhaseCutoverPly=0) produces deterministic identical games. Use default hybrid mode for diverse self-play.

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

### Board::recompute_zobrist removed
Was added during debugging, removed in cleanup. Not needed.

---

## Known Limitations

1. **OMA strength not measurable at short time controls.** A/B test at 2s movetime showed no significant difference (Elo -4.8, p=0.993). The Baier & Kaisers paper shows OMA benefits increase with longer time controls and Progressive Widening (Stage 15).
2. **Check/mate detection adds ~1.9x overhead per OMA decision.** Benchmarked at 7.6µs vs 4.0µs without. Total impact ~0.2% of search budget. Acceptable but could be optimized with geometric check detection in Stage 15.
3. **OMA moves are fixed after first visit.** The stored-moves approach prevents the "abstraction over different opponent outcomes" that the paper describes. Progressive Widening (Stage 15) will address this by allowing multiple stored moves per node.
4. **No MCTS-specific noise mechanism.** MoveNoise only works in Max^n. MCTS relies on Gumbel noise for exploration, which now works correctly after the sigma fix.
5. **AC1 (3-4x deeper root decisions) not fully verified.** Metrics are tracked but need longer time control testing to confirm the depth improvement.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| MCTS tests | 54 pass (13 new) | All OMA + pre-existing |
| Full test suite | 408 pass | 0 failures |
| A/B: OMA on vs off | Elo -4.8, p=0.993 | Not significant at 2s movetime |
| A/B: winner diversity | Red 6, Blue 4, Yellow 9, Green 1 | 20 games, no crashes |
| OMA check overhead | 1.9x per decision, 0.2% of search | Benchmarked from starting pos |
| OMA memory cost | ~13 bytes/node | ArrayVec<Move, 3> + bool |

---

## Open Questions

1. **Progressive Widening interaction.** Stage 15 adds PW at opponent nodes. Should PW replace stored OMA moves, or extend them (multiple stored sequences)?
2. **MCTS noise for self-play.** Without MoveNoise, MCTS-only games are deterministic. Should we add Gumbel noise seed variation per game?
3. **Sigma transform for non-root selection.** The `/100` scaling in `select_child` is crude but functional. Should it also use min/max normalization?

---

## Reasoning

- **Stored moves over deterministic RNG:** Zobrist-seeded RNG for OMA seemed elegant but hash drift after ~25 simulations caused board corruption. Stored moves guarantee consistency at ~13 bytes/node cost.
- **Check/mate detection included despite cost:** In 4PC, checkmate eliminates a player — game-defining. OMA opponents that can't play checkmates produce garbage simulations. The 0.2% overhead is worth realistic opponent behavior.
- **Sigma transform normalized to [0,1]:** The original `/100` scaling was a Stage 10 implementation shortcut that accidentally killed Gumbel exploration. Min/max normalization matches the paper's intent.
