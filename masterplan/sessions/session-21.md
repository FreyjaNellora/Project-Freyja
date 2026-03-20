---
tags: [session, stage-14]
session: 21
date: 2026-03-19
stage: 14
---

# Session 21 — Stage 14: MCTS Opponent Move Abstraction (OMA)

## Summary

Implemented OMA for MCTS per ADR-018 and Baier & Kaisers (IEEE CoG 2020). Discovered and fixed a critical Stage 10 bug (sigma transform saturation) and two observer bugs. All 408 tests pass. A/B shows no significant strength difference at short time controls, as expected.

## Key Decisions

1. **Stored OMA moves per tree node** instead of deterministic RNG replay. Zobrist-seeded RNG caused board corruption after ~25 simulations. Storing 3 moves per node (~13 bytes) guarantees consistency.
2. **Check/mate detection included** in OMA policy despite 1.9x overhead per decision. Checkmate = elimination in 4PC, too important to skip.
3. **Sigma transform normalized to [0,1]** instead of `/100`. The old scaling killed Gumbel exploration since Stage 10.

## Bugs Found

| Bug | Severity | Location | Status |
|-----|----------|----------|--------|
| Sigma transform saturation | Blocking (Stage 10) | mcts.rs Sequential Halving | Fixed |
| Observer player label off-by-one | Warning | ab_runner.mjs, observer.mjs | Fixed |
| MoveNoise doesn't apply to MCTS | Note | search.rs only | Documented |
| OMA tree consistency (zobrist drift) | Blocking | mcts.rs run_simulation | Fixed (stored moves) |
| OMA infinite loop (eliminated player) | Blocking | mcts.rs OMA branch | Fixed |

## Files Modified

- `freyja-engine/src/mcts.rs` (+430 lines)
- `freyja-engine/src/protocol/options.rs` (+20 lines)
- `freyja-engine/src/board/mod.rs` (formatting)
- `observer/ab_runner.mjs` (bug fix)
- `observer/observer.mjs` (bug fix)

## Research References

- Baier & Kaisers, "Guiding Multiplayer MCTS by Focusing on Yourself," IEEE CoG 2020
- Goodman et al., "MultiTree MCTS in Tabletop Games," IEEE CoG 2022
- Roelofs, "Pitfalls and Solutions When Using MCTS for Strategy Games," GameAIPro 3
- Danihelka et al., "Policy Improvement by Planning with Gumbel," ICLR 2022

## Next Session

- User UI testing for Stage 14 sign-off
- If approved: tag v1.14, begin Stage 15
