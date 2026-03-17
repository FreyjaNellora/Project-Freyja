# Project Freyja -- STATUS

**Last Updated:** 2026-03-17
**Updated By:** Session 20

---

## Current Stage

**Stage:** 13 (Time + Beam Tuning) -- IN PROGRESS
**Status:** Build order items 1-7 implemented and tested. A/B experiments and documentation pending.
**Current Build-Order Step:** 8 (Self-play experiments)

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
| 12 | Self-Play Framework | Complete | `stage-12-complete` / `v1.12` | 2026-03-16 |
| 13 | Time + Beam Tuning | In Progress | -- | -- |
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

- **[[Issue-UI-Feature-Gaps]]:** UI missing Debug Console, Engine Internals. (Stale — reviewed Session 20, still relevant but not blocking.)

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| perft(4) | 152,050 nodes | Starting position, debug build ~0.7s |
| Random playout avg | ~1004 half-moves | 1000 games, seeded LCG |
| Protocol startup | <1ms | Header output only |
| eval_4vec() | <100us debug, <50us release | Starting position |
| Search NPS (release, post-TT, beam 30) | ~89.7k depth 5 | Starting position, beam 30 all players |
| Search NPS (release, opponent ratio 0.25) | ~69k depth 5 | Starting position, root beam 30 / opponent beam 7 |
| Depth 5 nodes (opponent ratio 0.25) | 409k total | 20x reduction from 8M with full beam |
| Depth 6 nodes (opponent ratio 0.25) | 2.6M total | ~55 seconds from starting position |
| Depth 7 nodes (opponent ratio 0.25, 4M qnodes) | 18M total | ~7.5 minutes from starting position |
| Self-play: 20 games @ d4 | 0 errors, 0 crashes | Opponent beam ratio 0.25, 80 ply each |
| Self-play diversity | 4 unique winners / 5 games | MoveNoise=40, NoiseSeed per game |
| Unit tests | 398 pass | All existing + 17 new Stage 13 tests |

---

## What the Next Session Should Do First

1. Run A/B experiments: beam width, opponent ratio, Gumbel parameters
2. Document optimal settings in downstream_log_stage_13.md
3. Complete post-audit
4. User verification for stage completion

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18, 19
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores time budget at higher depths
- Eval suite systematic tuning

---

## Eval Tuning (Deferred to Stage 13)

Observer eval suite infrastructure created in `observer/baselines/`. 25 tactical samples from 3000+ Elo games. Current score 17/39 (44%) at depth 2. Systematic weight tuning belongs in Stage 13 (Time + Beam Tuning) where self-play A/B testing is available. Key findings documented in `observer/baselines/CLAUDE_T_EVAL_TUNING_GUIDANCE.md` and `masterplan/downstream_log_stage_09.md`.

---

## Key Metrics

| Metric | Value | Since |
|--------|-------|-------|
| Total stages | 21 (0-20) | -- |
| Stages complete | 13 (Stages 0-12) | 2026-03-16 |
| Stages in progress | 1 (Stage 13) | 2026-03-17 |
| Open blocking issues | 0 | 2026-03-17 |
| Open warning issues | 1 | 2026-03-07 |
| NPS baseline (full beam) | ~89.7k (release, depth 5) | 2026-03-08 |
| NPS baseline (opp ratio 0.25) | ~69k (release, depth 5) | 2026-03-17 |

---

## New setoptions Added (Stage 13)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| TimeSplitRatio | float | 0.5 | Max^n vs MCTS time split |
| MaxNodes | u64 | 0 (off) | Total node budget |
| MaxQnodes | u64 | 2000000 | Qsearch node budget |
| MoveNoise | u32 | 0 | Opening randomization (0-100) |
| NoiseSeed | u64 | 0 | Per-game noise seed |
| BeamSchedule | csv | None | Per-depth beam widths |
| AdaptiveBeam | bool | false | Complexity-based beam |
| OpponentBeamRatio | float | 0.25 | Opponent beam fraction |
| GumbelK | usize | 16 | MCTS root candidates |
| PriorTemperature | float | 50.0 | Softmax temperature |
| PHWeight | float | 1.0 | Progressive history weight |
| CPrior | float | 1.5 | UCB prior coefficient |
