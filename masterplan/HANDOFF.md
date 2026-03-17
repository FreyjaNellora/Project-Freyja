# Project Freyja -- HANDOFF

**Session Date:** 2026-03-17
**Session Number:** 20

---

## What Stage Are We On?

**Stage 13: Time + Beam Tuning -- IN PROGRESS**
Build order items 1-7 complete. Experiments partially done. Post-audit pending.

---

## What Was Completed This Session

1. **Qsearch node budget** — `max_qnodes` field in SearchConfig (default 2M). Soft abort in qsearch() and qsearch_2p() when budget exhausted. Fixes Issue-Depth4-Engine-Crash.
2. **EngineOptions → Searcher wiring** — Fixed latent bug where `options.beam_width` was parsed but ignored. `handle_go()` now constructs HybridConfig from options via `search_config()` and `mcts_config()` helpers.
3. **Node budget enforcement** — `MaxNodes` setoption flows through to SearchLimits. `max_nodes` in `should_abort()` was already checking it.
4. **MoveNoise + NoiseSeed** — Opening randomization. MoveNoise (0-100) replaces best move with random top-3 using xorshift from Zobrist hash XOR NoiseSeed. Observer sets NoiseSeed per game automatically.
5. **Beam width schedule** — Per-depth array in SearchConfig. `beam_width_for()` replaces flat `beam_width`. BeamSchedule setoption accepts comma-separated values.
6. **Opponent beam ratio** — BRS-inspired pruning. Opponents get `beam * opponent_beam_ratio` (default 0.25 = 1/4 beam, minimum 3 moves). **This fixed the depth 4 crash** by reducing branching factor enough to prevent stack overflow.
7. **ID time management** — Per-depth timing with 4x branching factor heuristic to predict if next depth fits in remaining budget.
8. **Gumbel parameters** — GumbelK, PriorTemperature, PHWeight, CPrior all exposed via setoption.
9. **Adaptive beam** — AdaptiveBeam setoption widens beam 50% for tactical positions, narrows 33% for quiet.
10. **Large stack thread** — Engine protocol runs on 256MB stack thread.
11. **Observer FEN4 positioning** — Uses `position fen4` instead of replaying full move history.

### Key Results
- **Depth 4: STABLE** — 20 games, 80 plies each, zero crashes (with opponent beam ratio 0.25)
- **Depth 5: Works** — 172k + 236k nodes from starting position (vs 8M without opponent ratio)
- **Depth 6: Works** — 1.3M + 1.3M nodes, ~55 seconds from starting position
- **Depth 7: Works** — 8.8M + 9.2M nodes, ~7.5 minutes from starting position
- **Game diversity: Confirmed** — 5 games at depth 3 produce 4 different winners with MoveNoise=40

---

## What Was NOT Completed

- Beam width A/B experiments (need MoveNoise working — now fixed)
- Gumbel parameter tuning experiments
- Adaptive beam experiments
- Post-audit and downstream log
- Documentation of optimal beam schedule
- Vault notes (Component-Search update, patterns)
- Session note

---

## What the Next Session Should Do First

1. **Run beam width experiments:** A/B test beam 30 vs 15 vs schedule at depth 4 with MoveNoise=40 + NoiseSeed
2. **Run opponent beam ratio experiments:** A/B test ratio 0.25 vs 0.5 vs 1.0
3. **Run Gumbel parameter experiments:** A/B test GumbelK 8 vs 16, PriorTemperature 25 vs 50
4. **Document findings** in downstream_log_stage_13.md and DECISIONS.md
5. **Complete post-audit** in audit_log_stage_13.md
6. **User verification** — user tests in UI, confirms stage complete

---

## Open Issues / Discoveries

- **Depth 4 crash root cause:** Stack overflow from deep recursion in 4-player Max^n + qsearch. Fixed by opponent beam ratio (0.25) which reduces effective branching factor 4x. Pure beam 30 at depth 4 still crashes in midgame.
- **NPS with opponent ratio:** 69k at depth 5 (was 100k). Lower NPS but 20x fewer nodes for same depth due to tighter pruning.
- **[[Issue-Depth4-Engine-Crash]]:** Can be updated to RESOLVED — opponent beam ratio is the permanent fix.
- **[[Issue-UI-Feature-Gaps]]:** Still open, not blocking.

---

## Files Modified This Session

| File | Action |
|------|--------|
| `freyja-engine/src/search.rs` | **MODIFIED** — qsearch budget, beam schedule, opponent ratio, MoveNoise, NoiseSeed, ID time mgmt |
| `freyja-engine/src/protocol/options.rs` | **MODIFIED** — All new setoptions (14 new options) |
| `freyja-engine/src/protocol/mod.rs` | **MODIFIED** — Wire EngineOptions into HybridConfig |
| `freyja-engine/src/main.rs` | **MODIFIED** — 256MB stack thread |
| `observer/observer.mjs` | **MODIFIED** — FEN4 positioning, NoiseSeed per game |
| `observer/ab_runner.mjs` | **MODIFIED** — FEN4 positioning, NoiseSeed per game |
| `observer/config_d3_*.json` | **CREATED** — Experiment configs |
| `observer/config_d4_*.json` | **CREATED** — Experiment configs |
| `observer/config_mt2s.json` | **CREATED** — Movetime config |
| `observer/config_ab_beam.json` | **CREATED** — A/B beam width config |
| `masterplan/audit_log_stage_13.md` | **CREATED** — Pre-audit complete |

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18, 19
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores time budget at higher depths
- Eval suite systematic tuning
- Stage 13 post-audit and downstream log (in progress)
