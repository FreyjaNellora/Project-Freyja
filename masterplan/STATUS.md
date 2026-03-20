# Project Freyja -- STATUS

**Last Updated:** 2026-03-20
**Updated By:** Session 22

---

## Current Stage

**Stage:** 14 -- COMPLETE (user signed off 2026-03-20)
**Status:** Complete. OMA implemented, sigma fix, stress tests, post-audit done.
**Next:** Stage 15 (Progressive Widening + Zone Control)

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
| 11 | Phase-Separated Hybrid Controller | Complete | `stage-11-complete` / `v1.11` | 2026-03-15 |
| 12 | Self-Play Framework | Complete | `stage-12-complete` / `v1.12` | 2026-03-16 |
| 13 | Time + Beam Tuning | Complete | `stage-13-complete` / `v1.13` | 2026-03-18 |
| 14 | MCTS Opponent Move Abstraction (OMA) | Complete | `stage-14-complete` / `v1.14` | 2026-03-20 |
| 15 | Progressive Widening + Zone Control | Not Started | -- | -- |
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

- **[[Issue-UI-Feature-Gaps]]:** UI missing Debug Console, Engine Internals. Stale but not blocking.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| perft(4) | 152,050 nodes | Starting position, debug build ~0.7s |
| Search NPS (release, opp ratio 0.25) | ~69k depth 5 | Opponent beam ratio active |
| Depth 4 total nodes | ~370k | Starting position, opp ratio 0.25 |
| Depth 5 total nodes | 409k | 20x reduction from 8M with full beam |
| Depth 6 total nodes | 2.6M | ~55 seconds |
| Depth 7 total nodes | 18M | ~7.5 minutes |
| Depth 8+ | Not practical | Needs NNUE for tighter beam |
| Self-play: 20 games @ d4 | 0 crashes | Opp ratio 0.25, 80 ply each |
| A/B: opp ratio 0.25 vs 0.5 | 0.25 stronger | Elo -28.6, p=0.04, 6 pairs |
| A/B: beam 30 vs 15 | No difference | p=0.59, 10 pairs |
| A/B: OMA on vs off | No difference | Elo -4.8, p=0.993, 10 pairs |
| Unit tests | 408 pass | 24 new in Stage 14 |

---

## Key Metrics

| Metric | Value | Since |
|--------|-------|-------|
| Total stages | 21 (0-20) | -- |
| Stages complete | 14 (Stages 0-13) | 2026-03-18 |
| Stage 14 status | Complete (pending sign-off) | 2026-03-20 |
| Open blocking issues | 0 | 2026-03-20 |
| Open warning issues | 1 | 2026-03-07 |
