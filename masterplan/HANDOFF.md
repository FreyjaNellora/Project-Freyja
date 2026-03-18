# Project Freyja -- HANDOFF

**Session Date:** 2026-03-18
**Session Number:** 20

---

## What Stage Are We On?

**Stage 13: Time + Beam Tuning -- COMPLETE** (tagged `stage-13-complete` / `v1.13`)
**Next: Stage 14**

---

## What Was Completed This Session

1. **Qsearch node budget** (2M default) — fixes depth 4 crash
2. **EngineOptions → Searcher wiring** — fixed latent bug where BeamWidth was ignored
3. **15 new setoptions** — TimeSplitRatio, MaxNodes, MaxQnodes, MoveNoise, NoiseSeed, BeamSchedule, AdaptiveBeam, OpponentBeamRatio, PhaseCutoverPly, GumbelK, PriorTemperature, PHWeight, CPrior, NoiseSeed
4. **Opponent beam ratio** (0.25 default) — BRS-inspired, validated via A/B (Elo -28.6, p=0.04)
5. **MoveNoise + NoiseSeed** — per-game randomization for diverse self-play
6. **Beam width schedule** — per-depth array, BeamSchedule setoption
7. **ID time management** — 4x branching factor heuristic
8. **Phase-separated hybrid** — Max^n opening (ply < 32), MCTS midgame (ply >= 32)
9. **Depth rotation rounding** — search depth rounds to nearest multiple of active players
10. **256MB stack thread** — prevents stack overflow in deep recursion
11. **A/B experiments:** opponent ratio 0.25 > 0.5; beam 30 ≈ 15

### Depth Testing Results
- Depth 4: stable (20 games, 0 crashes)
- Depth 5: works from startpos (409k nodes)
- Depth 6: works from startpos (2.6M nodes, ~55s)
- Depth 7: works from startpos (18M nodes, ~7.5min)
- Depth 8+: too slow for practical play with bootstrap eval
- Depth 12: did not complete depth 8 in 10 minutes

---

## What the Next Session Should Do First

1. Read Stage 14 spec in MASTERPLAN
2. Key items carried forward from Stage 13:
   - **MCTS warmup at cutover:** Transfer Max^n history table + priors at ply 32
   - **MCTS info output:** Periodic info lines during MCTS thinking (UI looks frozen)
   - **Phase checkpoint messages:** `info string phase opening/midgame`
3. Depth 8+ requires NNUE (Stages 15-17) for tighter beam and faster search

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **[[Issue-Depth4-Engine-Crash]]:** RESOLVED by opponent beam ratio 0.25.

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18, 19
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried to Stage 14)
- MCTS info output during thinking (carried to Stage 14)
