# Project Freyja -- HANDOFF

**Session Date:** 2026-03-20
**Session Number:** 24

---

## What Stage Are We On?

**Stage 15: Progressive Widening + Zone Control -- COMPLETE (pending user sign-off)**

All implementation done. Duel-tested. Awaiting UI testing for final sign-off.

---

## What Was Completed This Session

1. **Verified engine behavior** — Zone features confirmed to change PV and scores at depth 4 (was testing against stale binary initially — .cargo/config.toml redirected target dir)
2. **Swarm model implemented** — Mutual defense, attack coordination, pawn chain detection. Configurable via `SwarmWeight` setoption (default 3).
3. **Duel runner built** — New observer tool (`duel_runner.mjs`) enables head-to-head per-color testing. Two engine instances play different colors in the same game. All 3 seating arrangements tested (RY|BG, RB|YG, RG|BY) to control for position bias.
4. **Fixed NoiseSeed per game** — Duel runner sets unique NoiseSeed per game to ensure diverse games.
5. **Duel results: swarm+ray beats ray-only 9/15 (60%)** — Across 15 games with all seating arrangements and diverse noise seeds. Two eliminations on ray-only side vs one on swarm side.
6. **Design decision: swarm replaces BFS Voronoi** as the territory/zone control model.

---

## What Was NOT Completed

- **Performance benchmark** — Zone features not formally benchmarked (<5us target). NPS shows ~25% slowdown (12k vs 16k), within 30% tolerance.
- **PW A/B test** — k=2 vs k=4 not tested yet. Config created but not run.
- **Stage 15 audit log + downstream log** — Not written.
- **User UI testing** — Required for stage sign-off per AGENT_CONDUCT 1.9.

---

## What the Next Session Should Do First

1. **Get user sign-off on Stage 15** from UI testing
2. If approved, tag `stage-15-complete` / `v1.15`
3. Write audit_log_stage_15.md and downstream_log_stage_15.md
4. Run PW k=2 vs k=4 duel test (config exists: `config_ab_pw.json`)
5. Read Stage 16 spec (NNUE Training Pipeline)

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **MoveNoise in MCTS:** Still unresolved. Hybrid mode provides diversity for testing.
- **MASTERPLAN numbering error:** Two "Stage 15" headers (line 1001, 1030).

---

## Design Decisions Made This Session

- **ADR-020:** Ray-attenuation replaces distance-decay for influence maps (directional, obstacle-aware)
- **ADR-021:** Swarm model replaces BFS Voronoi — empirically validated via duel (9/15 wins)
- **Duel runner** with random seating controls for 4PC position bias

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 4, 7, 8, 11, 12, 17, 18, 19, 20, 21
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)
- PW k=2 vs k=4 A/B test (config ready, not run)

---

## Files Modified This Session (Session 24)

| File | Changes |
|------|---------|
| `freyja-engine/src/eval.rs` | Swarm model, DKW fix, zone weight tests |
| `freyja-engine/src/protocol/options.rs` | SwarmWeight setoption |
| `freyja-engine/src/protocol/mod.rs` | Wire swarm weight |
| `observer/duel_runner.mjs` | NEW — head-to-head per-color duel runner |
| `observer/config_duel_swarm_vs_ray.json` | NEW — duel config |
| `observer/config_duel_smoke.json` | NEW — smoke test config |
| `observer/config_ab_pw.json` | NEW — PW A/B config |
| `observer/config_ab_swarm_vs_ray.json` | NEW — standard A/B config |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/STATUS.md` | Updated |
| `masterplan/DECISIONS.md` | ADR-021 status → Accepted |
| `masterplan/sessions/Session-024.md` | New |
