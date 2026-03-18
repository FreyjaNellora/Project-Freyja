# Downstream Log — Stage 13: Time Management + Beam Width Tuning

**Author:** Agent (Session 20)
**Date:** 2026-03-17

---

## Must-Know

1. **Opponent beam ratio is the default.** All opponent nodes get `beam * 0.25` (minimum 3 moves). This is a BRS-inspired optimization that reduces the effective branching factor ~4x. Without it, depth 4 crashes from stack overflow.
2. **MoveNoise + NoiseSeed required for diverse self-play.** MoveNoise alone is deterministic per position. NoiseSeed must vary per game (observer does this automatically).
3. **Qsearch has a soft node budget (2M default).** When exceeded, qsearch returns stand-pat. Overshoot is expected (budget checked per-call, not per-subtree).
4. **ID time management uses 4x branching factor heuristic.** After each depth, estimates if `last_depth_ms * 4 > remaining_ms`. This is conservative; actual branching factor varies by position.
5. **Engine runs on 256MB stack thread.** Required for deep recursion on 14x14 board.

---

## API Contracts

### New SearchConfig Fields
```rust
pub struct SearchConfig {
    pub beam_width: usize,           // Default 30
    pub sum_bound: i32,              // Default 40_000
    pub tt_size_mb: usize,           // Default 16
    pub max_qnodes: u64,             // Default 2_000_000
    pub beam_schedule: Option<[usize; MAX_DEPTH]>,  // None = flat beam_width
    pub move_noise: u32,             // 0 = deterministic, 1-100 = probabilistic
    pub adaptive_beam: bool,         // false = off
    pub opponent_beam_ratio: f32,    // Default 0.25
    pub noise_seed: u64,             // 0 = default, varies per game
}
```

### New setoption Commands
| Option | Type | Default | Range | Description |
|--------|------|---------|-------|-------------|
| TimeSplitRatio | f32 | 0.5 | [0.0, 1.0] | DEPRECATED — phases no longer overlap |
| PhaseCutoverPly | u32 | 32 | >= 0 | Ply at which hybrid switches from Max^n to MCTS |
| MaxNodes | u64 | 0 (off) | >= 0 | Total node budget for search |
| MaxQnodes | u64 | 2000000 | >= 0 | Qsearch node budget |
| MoveNoise | u32 | 0 | [0, 100] | Probability of random top-3 move |
| NoiseSeed | u64 | 0 | >= 0 | Per-game randomization seed |
| BeamSchedule | csv | None | positive ints | Per-depth beam widths |
| AdaptiveBeam | bool | false | true/false | Complexity-based beam |
| OpponentBeamRatio | f32 | 0.25 | [0.0, 1.0] | Opponent beam fraction |
| GumbelK | usize | 16 | > 0 | MCTS root candidates |
| PriorTemperature | f32 | 50.0 | > 0.0 | Softmax temperature |
| PHWeight | f32 | 1.0 | >= 0.0 | Progressive history weight |
| CPrior | f32 | 1.5 | >= 0.0 | UCB prior coefficient |

### EngineOptions Helper Methods
```rust
impl EngineOptions {
    pub fn search_config(&self) -> SearchConfig;  // Build SearchConfig from options
    pub fn mcts_config(&self) -> MctsConfig;       // Build MctsConfig from options
}
```

### Key Functions Changed
```rust
// search.rs — beam width now depends on player role
fn beam_width_for(&self, remaining_depth: u32, is_root_player: bool, state: &mut GameState) -> usize;

// Qsearch soft budget check (both qsearch and qsearch_2p)
if ss.qnodes >= ss.max_qnodes { return stand_pat; }

// ID time management (in iterative_deepening)
if last_depth_ms * 4 > remaining_ms && depth >= min_depth { break; }

// MoveNoise (after iterative_deepening loop)
if move_noise > 0 { /* xorshift from hash ^ noise_seed, replace best with top-3 */ }
```

---

## Known Limitations

1. **Opponent beam ratio reduces play quality for opponents.** With ratio 0.25, opponents only consider ~7 moves (vs 30 for root). This is intentional (BRS tradeoff) but means the engine assumes opponents play suboptimally.
2. **MoveNoise distorts search.** At noise > 50, the engine frequently makes suboptimal moves. 30-40 is the sweet spot for diversity without major quality loss.
3. **Qsearch budget overshoot.** The 2M budget is soft — actual qnodes can reach 2.5-2.7M before the check fires in deeply recursive positions.
4. **ID time management is conservative.** The 4x branching factor heuristic may under-utilize the time budget in simple positions. Could be improved with per-depth timing history.
5. **BeamSchedule not yet empirically tuned.** The infrastructure exists but no optimal schedule has been determined via A/B testing.
6. **Depth 4 crashes without opponent beam ratio.** `OpponentBeamRatio=1.0` at depth 4 causes stack overflow in midgame. This is a fundamental limitation of the 4-player Max^n recursion depth.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| NPS (release, depth 5, opp ratio 0.25) | ~69k | Down from 89.7k with full beam |
| Depth 5 total nodes (opp ratio 0.25) | 409k | Down from 8M with full beam (20x reduction) |
| Depth 6 total nodes (opp ratio 0.25) | 2.6M | ~55 seconds from starting position |
| Depth 7 total nodes (opp ratio 0.25, 4M qnodes) | 18M | ~7.5 minutes from starting position |
| TT hit rate (depth 5, opp ratio 0.25) | 8.6% | Up from 4.4% with full beam |
| Self-play stability (depth 4, 20 games) | 0 crashes | 80 ply each, opp ratio 0.25 |
| Game diversity (MoveNoise=40, 5 games) | 4 unique winners | NoiseSeed per game |
| Unit tests | 398 pass | 17 new tests added in Stage 13 |

---

## A/B Experiment Results

### Experiment 1: Opponent Beam Ratio 0.25 vs 0.5 (depth 4, 6 pairs)
- **SPRT: accept_h0 after 6 pairs** (LLR=-3.732)
- Avg score per seat: 0.25 = 319.8 +/- 84.7, 0.5 = 224.5 +/- 36.7
- Elo difference: -28.6 (0.25 stronger), p=0.04 (significant)
- Pawn ratio: 0.25 = 0.80, 0.5 = 0.67 (0.25 preserves more material)
- Shuffle index: 0.25 = 0.085, 0.5 = 0.023 (p=0.003, 0.25 produces more varied play)
- **Conclusion: OpponentBeamRatio=0.25 validated as optimal default.**

### Experiment 2: Beam Width 30 vs 15 (depth 4, 10 pairs)
- **SPRT: inconclusive after 10 pairs** (LLR=-0.005)
- Avg score per seat: beam30 = 314.9 +/- 64.7, beam15 = 329.8 +/- 55.7
- Elo difference: +4.5 (beam15 marginally stronger), p=0.59 (NOT significant)
- Win distribution: beam30 skewed Yellow (6/10), beam15 more balanced (Red 4, Blue 4)
- **Conclusion: No significant difference. Beam 15 is a safe default — faster search with equivalent strength.**

### Cross-experiment Summary
- 32 games at depth 4, 0 crashes
- Opponent beam ratio is the dominant factor; root beam width is secondary
- MoveNoise=40 + NoiseSeed produces adequate diversity for SPRT

---

## Open Questions

1. **Beam schedule for depth 7-8?** No empirical data yet. Schedule could enable deeper search in practical time.
2. **Gumbel parameter sensitivity?** GumbelK, PriorTemperature, PHWeight all exposed but not yet tuned via A/B.
3. **Eval tuning via self-play?** Now possible with MoveNoise + A/B infrastructure. Deferred to Stage 14+.
4. **Beam 15 as new default?** A/B shows no strength loss vs 30. Would reduce node count and enable deeper search. Needs user sign-off.

---

## Reasoning

- **Opponent beam ratio** was chosen because BRS research (Schadd & Winands 2011, Baier & Kaisers 2020) shows narrowing opponent moves is the single biggest performance gain in multi-player search. The 0.25 default is aggressive but enables depth 4 stability.
- **Engine-side MoveNoise** was chosen over observer-side randomization because it's simpler, testable, and zero-cost when disabled. NoiseSeed was added after discovering that Zobrist hash alone gives deterministic noise per position.
- **256MB stack thread** was added after confirming the depth 4 crash was stack overflow (silent process death, no backtrace). Even with opponent beam ratio, deep positions can create very deep recursion.
- **Qsearch soft budget** (vs hard abort) was chosen to preserve search correctness — hard abort with SCORE4_MIN would corrupt the parent node's evaluation. Returning stand-pat is safe.
