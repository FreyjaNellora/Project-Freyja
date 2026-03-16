# Project Freyja -- STATUS

**Last Updated:** 2026-03-15
**Updated By:** Session 17

---

## Current Stage

**Stage:** 11 (Max^n → MCTS Integration) -- COMPLETE
**Status:** HybridSearcher sequences Max^n → MCTS with history transfer and prior policy injection. 381 total tests (21 hybrid-specific), 0 clippy warnings. User-verified in UI: 28 ply of 4-player play, MCTS overriding Max^n on strategic moves, stable performance.

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
| 8 | Quiescence Search | Complete | `stage-08-complete` / `v1.8` | 2026-03-07 |
| 9 | TT + Move Ordering | Complete | `stage-09-complete` / `v1.9` | 2026-03-14 |
| 10 | MCTS | Complete | `stage-10-complete` / `v1.10` | 2026-03-15 |
| 11 | Max^n -> MCTS Integration | Complete | `stage-11-complete` / `v1.11` | 2026-03-15 |
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
| Search NPS (release, pre-qsearch) | ~84k depth 4 | Starting position, 2s budget |
| Search NPS (release, post-qsearch) | ~33-60k depth 4 | Starting position, 5s budget, min depth 4 |
| Search NPS (release, post-TT) | ~89.7k depth 5 | Starting position, TT + move ordering |
| Eval suite score | 17/39 (44%) at depth 2 | Baseline — systematic tuning deferred to Stage 13 |
| MCTS tests | 41/41 pass | All 9 acceptance criteria |
| Hybrid tests | 21/21 pass | AC1-AC8 coverage |

---

## What the Next Session Should Do First

1. Begin Stage 12 (Self-Play Framework) — read MASTERPLAN Stage 12 spec
2. Read `masterplan/downstream_log_stage_11.md` for HybridSearcher API contracts
3. Address deferred debt if time allows

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores 2s budget at higher depths (only affects debug, release works correctly)

---

## Eval Tuning (Deferred to Stage 13)

Observer eval suite infrastructure created in `observer/baselines/`. 25 tactical samples from 3000+ Elo games. Current score 17/39 (44%) at depth 2. Systematic weight tuning belongs in Stage 13 (Time + Beam Tuning) where self-play A/B testing is available. Key findings documented in `observer/baselines/CLAUDE_T_EVAL_TUNING_GUIDANCE.md` and `masterplan/downstream_log_stage_09.md`.

---

## Key Metrics

| Metric | Value | Since |
|--------|-------|-------|
| Total stages | 21 (0-20) | -- |
| Stages complete | 12 (Stages 0-11) | 2026-03-15 |
| Open blocking issues | 0 | 2026-03-15 |
| Open warning issues | 1 | 2026-03-07 |
| NPS baseline | ~89.7k (release, depth 5, TT+ordering) | 2026-03-08 |
