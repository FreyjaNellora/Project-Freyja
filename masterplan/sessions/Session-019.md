# Session 019 — Stage 12 Completion & Quiescence Discussion

**Date:** 2026-03-16
**Duration:** ~20 minutes
**Stage:** 12 (Self-Play Framework) → COMPLETE

---

## Summary

Marked Stage 12 complete with user approval. Tagged `stage-12-complete` / `v1.12`. Had an in-depth discussion about quiescence search balance in 4-player chess — how to handle the tension between deeper main search and bounded qsearch, and why "skip losing captures" is wrong as a blanket rule in 4PC.

## What Was Done

1. **Stage 12 closed:** User gave green light. Created tags `stage-12-complete` and `v1.12`.
2. **Session-end protocol:** Updated STATUS.md, HANDOFF.md, created this session note, updated MOCs and Wikilink Registry.
3. **Registered [[Issue-Depth4-Engine-Crash]]** in MOC-Active-Issues and Wikilink-Registry.

## Key Discussions

### Quiescence Search in 4-Player Chess

The user raised a fundamental question: how can the engine know to prune a capture without searching it? Key points from the discussion:

- **Qsearch isn't supposed to find brilliance** — it resolves hanging pieces so the static eval doesn't see unstable positions.
- **"Skip losing captures" is wrong in 4PC** — a bishop-for-pawn trade that weakens your strongest opponent may be strategically correct.
- **Main search finds strategy, qsearch resolves tactics.** Deep sacrifices are the main search's job.
- **Practical solutions for Stage 13:** Qsearch node budget (2M nodes), beam on captures (top 5-8 by MVV-LVA), adaptive qsearch depth based on position complexity.
- **NNUE (Stages 14-17)** is what eventually learns to assess sacrificial positions — the eval learns "this material loss leads to wins" so the search doesn't have to find it by brute force.

### Depth 4 Engine Crash

- Engine crashes at depth 4 from qsearch explosion — too many capture chains.
- Works fine at depth 2 and 3.
- Filed as [[Issue-Depth4-Engine-Crash]], to be addressed in Stage 13.

## Discoveries

- At shallow depth, Yellow wins 100% deterministically due to first-mover advantage. All 4 engines play identically (same position, same eval, same search), so the only differentiator is move order.

## Next Steps

- Stage 13: Time + Beam Tuning
- Priority: opening randomization, qsearch bounding, time management

---

**Related:** [[STATUS]], [[HANDOFF]], [[audit_log_stage_12]], [[downstream_log_stage_12]], [[Issue-Depth4-Engine-Crash]]
