# Project Freyja -- HANDOFF

**Session Date:** 2026-03-20
**Session Number:** 23

---

## What Stage Are We On?

**Stage 15: Progressive Widening + Zone Control -- IN PROGRESS**

Part A (PW) complete. Part B (Zone Control) has working ray-attenuation implementation. Swarm model planned as alternative for A/B comparison.

---

## What Was Completed This Session

1. **Progressive Widening (Part A) -- COMPLETE**
   - PW at root-player nodes (paper-faithful, Baier & Kaisers 2020)
   - Children sorted by prior after expansion (best-first for PW window)
   - `root_decisions_total` renamed to `tree_moves_total` + new `root_player_decisions` counter
   - PW diagnostic tracking (`pw_limited_selections`)
   - Setoptions: `PWConstant` (k), `PWExponent` (alpha)
   - 9 new PW tests

2. **Zone Control (Part B) -- RAY-ATTENUATION MODEL IMPLEMENTED**
   - BFS territory enhanced with contested squares + frontier detection
   - Ray-attenuated influence maps: directional force projection with blocker degradation
   - Tension/vulnerability scoring from overlapping influence
   - King escape routes
   - Configurable zone weights via setoption
   - DKW players correctly skipped (`is_active_for_zones`)
   - 11 new zone tests
   - 441 total tests pass

3. **Research & Design Direction**
   - Extensive research into BFS Voronoi limitations, influence map models
   - User identified critical flaw: distance-decay is circular, pieces project along vectors
   - Pivoted from exponential distance-decay to ray-attenuation (directional, obstacle-aware)
   - User proposed swarm mechanics as next evolution (to replace BFS Voronoi)
   - Architecture: ray-attenuation = individual piece voice, swarm = collective chord

---

## What Was NOT Completed

- **Swarm model implementation** -- Designed but not coded. Next session.
- **A/B testing** -- No A/B tests run yet (PW or zone control)
- **Performance benchmark** -- Zone features not benchmarked yet (<5us target)
- **Stage 15 audit/downstream logs** -- Not written
- **k=2 vs k=4 A/B test** -- User approved test, not yet run

---

## What the Next Session Should Do First

1. **Implement swarm model** as alternative to BFS Voronoi territory
   - Layer on top of ray-attenuation: cluster cohesion, mutual defense, attack coordination
   - Separate setoption toggle to enable swarm vs Voronoi
2. **A/B test: swarm vs ray-attenuation vs baseline** (all zone features off)
3. **A/B test: PW enabled vs disabled** at movetime 5000 and 15000
4. **A/B test: k=2 vs k=4** for PW constant
5. **Performance benchmark** zone features (<5us target)
6. Stage 15 audit log, downstream log

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **MoveNoise in MCTS:** Still unresolved. Use hybrid mode for A/B diversity.
- **Crash at ply 32 in Tauri:** Not reproduced in unit tests. Monitor.
- **MASTERPLAN numbering error:** Two "Stage 15" headers (line 1001, 1030). Second should be Stage 16.

---

## Design Decisions Made This Session

- **PW at root-player nodes** (not opponent nodes). Paper-faithful approach. OMA handles opponents.
- **Ray-attenuation replaces distance-decay** for influence maps. Directional, obstacle-aware.
- **Swarm model planned** to replace BFS Voronoi. Will A/B test against ray-attenuation.
- **Friendly vs enemy attenuation asymmetry** in ray model: friendly blockers attenuate mildly (1.5x), enemy blockers attenuate strongly (2.0 + piece value scaled).

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 4, 7, 8, 11, 12, 17, 18, 19, 20, 21
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)

---

## Files Modified This Session (Session 23)

| File | Changes |
|------|---------|
| `freyja-engine/src/eval.rs` | Ray-attenuation influence, BFS territory enhanced, tension, king escape, ZoneWeights, tests |
| `freyja-engine/src/mcts.rs` | PW metrics, child sorting, PW diagnostics, tests |
| `freyja-engine/src/protocol/options.rs` | PW + zone weight setoptions |
| `freyja-engine/src/protocol/mod.rs` | Wire zone weights to evaluator |
| `freyja-engine/src/search.rs` | cargo fmt only |
| `freyja-engine/src/hybrid.rs` | cargo fmt only |
| `freyja-engine/src/move_gen.rs` | cargo fmt only |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/STATUS.md` | Updated |
| `masterplan/sessions/Session-023.md` | New |
