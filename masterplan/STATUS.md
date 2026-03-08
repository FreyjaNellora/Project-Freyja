# Project Freyja -- STATUS

**Last Updated:** 2026-03-07
**Updated By:** Session 12

---

## Current Stage

**Stage:** 7 (Max^n Search) -- IN PROGRESS
**Build-Order Step:** 10/10 (engine complete, UI auto-play verified)
**Status:** Stage 7 COMPLETE. User green light granted Session 12. Engine plays all 4 sides via UI auto-play, analysis panel shows depth/nodes/NPS/scores/PV.

---

## Stage Completion Tracker

| Stage | Name | Status | Tag | Date |
|-------|------|--------|-----|------|
| 0 | Project Skeleton | Complete | `stage-00-complete` / `v1.0` | 2026-03-03 |
| 1 | Board Representation | Complete | `stage-01-complete` / `v1.1` | 2026-03-04 |
| 2 | Move Generation | Complete | `stage-02-complete` / `v1.2` | 2026-03-05 |
| 3 | Game State | Complete | `stage-03-complete` / `v1.3` | 2026-03-06 |
| 4 | Freyja Protocol | Complete | `stage-04-complete` / `v1.4` | 2026-03-06 |
| 5 | UI Shell | Complete | `stage-05-complete` / `v1.5` | 2026-03-06 |
| 6 | Bootstrap Evaluation | Complete | `stage-06-complete` / `v1.6` | 2026-03-07 |
| 7 | Max^n Search | Complete | `stage-07-complete` / `v1.7` | 2026-03-07 |
| 8 | Quiescence Search | Not Started | -- | -- |
| 9 | TT + Move Ordering | Not Started | -- | -- |
| 10 | MCTS | Not Started | -- | -- |
| 11 | Max^n -> MCTS Integration | Not Started | -- | -- |
| 12 | Self-Play Framework | Not Started | -- | -- |
| 13 | Time + Beam Tuning | Not Started | -- | -- |
| 14 | Zone Control Features | Not Started | -- | -- |
| 15 | NNUE Architecture | Not Started | -- | -- |
| 16 | NNUE Training Pipeline | Not Started | -- | -- |
| 17 | NNUE Integration | Not Started | -- | -- |
| 18 | Game Mode Tuning | Not Started | -- | -- |
| 19 | Full UI | Not Started | -- | -- |
| 20 | Optimization | Not Started | -- | -- |

---

## Blocking Issues

*None.*

---

## Warning Issues

- **[[Issue-UI-Feature-Gaps]]:** UI missing Debug Console, Engine Internals needed for Stage 8-10 development. Prioritized feature list with Odin source references. See `masterplan/issues/Issue-UI-Feature-Gaps.md`.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| perft(4) | 152,050 nodes | Starting position, debug build ~0.7s |
| Random playout avg | ~1004 half-moves | 1000 games, seeded LCG |
| Protocol startup | <1ms | Header output only |
| eval_4vec() | <100us debug, <50us release | Starting position |
| Observer: 3 games depth 1 | 198 ply each, stable | No crashes, no infinite loops |
| Search NPS (release) | ~84k depth 4 | Starting position, 2s budget |

---

## What the Next Session Should Do First

1. Get user green light on Stage 7 (watch engine play in UI, confirm acceptable)
2. Complete Stage 7 formalities (post-audit, tag `stage-07-complete` / `v1.7`)
3. Begin Stage 8 (Quiescence Search) planning

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7 and 8
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores 2s budget at higher depths (only affects debug, release works correctly)

---

## Key Metrics

| Metric | Value | Since |
|--------|-------|-------|
| Total stages | 21 (0-20) | -- |
| Stages complete | 8 (Stages 0-7) | 2026-03-07 |
| Open blocking issues | 0 | 2026-03-07 |
| Open warning issues | 1 | 2026-03-07 |
| NPS baseline | ~84k (release, depth 4) | 2026-03-07 |
