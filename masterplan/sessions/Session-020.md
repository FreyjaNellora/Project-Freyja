# Session 020 — Stage 13 Implementation

**Date:** 2026-03-17
**Stage:** 13 (Time + Beam Tuning) — In Progress
**Focus:** Core implementation of all tuning parameters and crash fixes

---

## Summary

Implemented all 7 build-order items for Stage 13 plus critical bug fixes. The major achievement was fixing the depth 4 crash via opponent beam ratio (BRS-inspired pruning), which reduced the effective branching factor 4x and eliminated the stack overflow.

## Key Achievements

1. **Qsearch node budget** — Soft cap at 2M qnodes prevents capture explosion
2. **EngineOptions wiring** — Fixed latent bug where BeamWidth setoption was ignored
3. **Opponent beam ratio** — 0.25 default = opponents get 1/4 of root beam. This was the breakthrough that fixed depth 4 stability.
4. **MoveNoise + NoiseSeed** — Per-game randomization creates diverse self-play games (4 unique winners in 5 games)
5. **Beam schedule** — Per-depth beam width arrays for narrower beams at deeper depths
6. **ID time management** — Predicts whether next depth fits in remaining time budget
7. **14 new setoptions** — All tunable via protocol for A/B experimentation

## Depth Improvements

| Depth | Before (beam 30) | After (opp ratio 0.25) |
|-------|-------------------|------------------------|
| 4 | CRASH at ply 30 | 20 games stable |
| 5 | 8M nodes | 409k nodes (20x reduction) |
| 6 | untested | 2.6M nodes, ~55s |
| 7 | untested | 18M nodes, ~7.5min |

## Commits

1. `37b7136` — Qsearch node budget
2. `e859cac` — Wire EngineOptions, all setoption params
3. `2e6457b` — MoveNoise, beam schedule, adaptive beam, ID time management
4. `9092442` — Large stack thread, observer FEN4 positioning
5. `4bd78d0` — Opponent beam ratio (BRS-inspired)
6. `8ca0a2c` — NoiseSeed for per-game diversity

## Remaining Work

- A/B experiments: beam width, opponent ratio, Gumbel parameters
- Post-audit and downstream log
- Documentation of optimal settings
- User verification

## Decisions Made

- **Opponent beam ratio 0.25 as default** — Based on BRS research. Dramatically improves depth reach while maintaining play quality (needs A/B validation).
- **MoveNoise at engine level** — Simpler than observer-side randomization. NoiseSeed varies per game.
- **256MB stack thread** — Necessary for deep recursion on 14x14 board even with opponent beam ratio.
