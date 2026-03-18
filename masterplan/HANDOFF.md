# Project Freyja -- HANDOFF

**Session Date:** 2026-03-18
**Session Number:** 20 (continued)

---

## What Stage Are We On?

**Stage 13: Time + Beam Tuning -- NEARLY COMPLETE**
All core features implemented. Phase separation added. Awaiting final testing.

---

## What Was Completed This Session

1. **All Stage 13 build order items** (Steps 1-7) — qsearch budget, option wiring, MoveNoise, beam schedule, opponent beam ratio, ID time management, Gumbel params
2. **A/B experiments completed:**
   - Opponent ratio 0.25 vs 0.5: 0.25 is stronger (Elo -28.6, p=0.04)
   - Beam 30 vs 15: no significant difference (p=0.59)
3. **Phase-separated hybrid controller:**
   - Opening (ply < 32): Max^n only, depth 4 cap, instant moves
   - Midgame (ply >= 32): MCTS only, full time budget
   - PhaseCutoverPly setoption (default 32)
4. **Depth 4 crash permanently fixed** via opponent beam ratio 0.25
5. **Game diversity** via MoveNoise + NoiseSeed per game

---

## What Was NOT Completed

1. **MCTS warmup at cutover** — When MCTS takes over at ply 32, it starts cold. Need to transfer Max^n's accumulated history table and compute prior policy at the cutover point (the `set_history_table()` and `set_prior_policy()` APIs already exist).
2. **Info output during MCTS** — UI looks frozen during MCTS thinking because no `info` lines are sent. Need periodic info output from MCTS simulations.
3. **Checkpoint visibility** — Need protocol output that shows Max^n thinking and MCTS thinking separately so the user can debug each phase.
4. **Depth 8 testing** — User wants to test depth 8 with beam schedule. Should be feasible with opponent beam ratio 0.25 + beam schedule (narrower at deeper depths).
5. **Gumbel parameter tuning** — Infrastructure ready, experiments not run.
6. **Stage 13 sign-off** — User hasn't given green light yet.

---

## What the Next Session Should Do First

1. **MCTS warmup at cutover:** In hybrid.rs, when `game_ply == phase_cutover_ply` (first MCTS move), run a quick Max^n depth 4 search, extract history table + priors, and pass to MCTS before its search. Only do this once at the transition.
2. **MCTS info output:** In mcts.rs, emit `info` lines every N simulations (sims, sps, best move so far). The protocol layer already handles `info` output.
3. **Checkpoint protocol messages:** Add `info string phase opening` and `info string phase midgame` so the UI knows which engine is thinking.
4. **Depth 8 test:** Try `go depth 8` with beam schedule `30,30,20,20,12,12,8,8` and opponent ratio 0.25. May need beam schedule wired through observer configs.
5. **Commit and get user sign-off for Stage 13.**

---

## Open Issues

- **[[Issue-Depth4-Engine-Crash]]:** RESOLVED by opponent beam ratio 0.25.
- **[[Issue-UI-Feature-Gaps]]:** Still open, not blocking.
- **MCTS freeze at cutover:** Not a crash — MCTS takes full 5s budget with no visible output. Fix: info output during MCTS + warmup for faster convergence.

---

## Key Decisions Made This Session

- **OpponentBeamRatio=0.25 validated** — A/B tested, statistically stronger than 0.5
- **Beam 30 vs 15 equivalent** — No significant difference with opponent pruning active
- **Phase separation** — Max^n opening only, MCTS midgame only (no blending)
- **Depth 4 cap on Max^n** — One full rotation, extra time to MCTS
- **PhaseCutoverPly=32** — 8 rounds of opening before MCTS takes over

---

## Files Modified This Session

| File | Action |
|------|--------|
| `freyja-engine/src/search.rs` | Qsearch budget, beam schedule, opponent ratio, MoveNoise, NoiseSeed, ID time mgmt, depth cap |
| `freyja-engine/src/hybrid.rs` | Phase separation (Max^n opening / MCTS midgame), PhaseCutoverPly |
| `freyja-engine/src/protocol/options.rs` | 15 new setoptions including PhaseCutoverPly |
| `freyja-engine/src/protocol/mod.rs` | Wire options into HybridConfig, game_ply in SearchLimits |
| `freyja-engine/src/main.rs` | 256MB stack thread |
| `observer/observer.mjs` | FEN4 positioning, NoiseSeed per game |
| `observer/ab_runner.mjs` | FEN4 positioning, NoiseSeed in SPRT path |
| `masterplan/audit_log_stage_13.md` | Pre-audit + post-audit |
| `masterplan/downstream_log_stage_13.md` | Full API docs + A/B results |
| `masterplan/STATUS.md` | Updated by user |
| `masterplan/sessions/Session-020.md` | Session note |

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18, 19
- Dead code: `apply_move_with_events` in `game_state.rs`
