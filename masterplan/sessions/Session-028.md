# Session 028

**Date:** 2026-03-21
**Stage:** 17 (NNUE Integration) — In Progress
**Focus:** Stage 17 research, planning, training data generation infrastructure

---

## Summary

Began Stage 17: NNUE Integration. Performed comprehensive research on NNUE integration best practices (Stockfish, training data quality, SPRT methodology, multi-player NNUE challenges). Diagnosed the root cause of the NNUE score magnitude problem: training normalization (÷3000) combined with Q6 quantization and two ÷64 divisions in the forward pass mathematically guarantees single-digit centipawn output. Fix identified: OUTPUT_MULTIPLIER (~600) in forward pass.

Benchmarked self-play performance on user's hardware (i5-10400F, 8GB RAM): 3 games at depth 4 = 10m 34s → ~3.5 min/game. Extrapolated: 500 games = ~29 hours, 1000 games = ~58 hours. User decided to split 1,000 games across two machines (500 each, ~29h).

Built training data generation infrastructure: config file for 100 games at depth 4, batch runner script that orchestrates multiple batches with extraction and merge. Ready to execute.

## Key Decisions

- **Depth 4 only for training data.** User explicitly rejected depth 3. Quality over speed.
- **1,000 games split across 2 machines.** ~60,000 positions target. ~29h per machine.
- **Score magnitude fix: OUTPUT_MULTIPLIER, not retraining.** The problem is arithmetic, not network quality. Apply post-forward-pass multiplier (~600) to rescale to centipawn range. Make it tunable via NnueScale protocol option.
- **Phase 0 code changes deferred.** User wanted infrastructure-only this session — code changes in next session after data is collected.

## Key Research Findings

1. **Score magnitude is arithmetic:** normalization(÷3000) → float[-1,+1] → Q6(×64) → ÷64 twice → ~1cp. Fix: multiply by OUTPUT_MULTIPLIER.
2. **1,050 positions is ~100x too few** for 1.15M-param network. Need ≥50K.
3. **Quiet position filtering** is the #1 training data quality improvement.
4. **4-player NNUE is uncharted.** No published research. Closest: Multiplayer AlphaZero (2019).
5. **SPRT for validation:** ~400 games for +100 Elo difference at 95% confidence.
6. **Incremental accumulator** deferred to Stage 20 — eval quality matters more than speed now.

## Files Created/Modified

- `observer/config_training_d4_500.json` (new) — 100 games, depth 4, MoveNoise 40
- `observer/run_training_batches.mjs` (new) — Batch runner with extract + merge
- `masterplan/HANDOFF.md` — Updated for Session 28
- `masterplan/STATUS.md` — Stage 17 In Progress
- `masterplan/sessions/Session-028.md` (new) — This file

## Open Issues

- NNUE score magnitude (NOTE) — root cause known, fix planned, not yet implemented
- [[Issue-UI-Feature-Gaps]] (WARNING) — still open

## Next Steps

1. User starts 1K-game run on 2 machines (~29h each)
2. Next agent: fix score magnitude (Phase 0), retrain NNUE (Phase 2), validate (Phase 4)
3. Full implementation plan at `.claude/plans/ethereal-honking-bee.md`
