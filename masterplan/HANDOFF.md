# Project Freyja -- HANDOFF

**Session Date:** 2026-03-21
**Session Number:** 28

---

## What Stage Are We On?

**Stage 17: NNUE Integration -- IN PROGRESS (data generation phase)**

Stage 17 has been planned and researched. Training data generation infrastructure is ready. Waiting on 1,000-game self-play run (split across 2 machines, ~29 hours each) before continuing with NNUE code changes.

---

## What Was Completed This Session

1. **Stage 17 research and planning:**
   - Read AGENT_CONDUCT.md, MASTERPLAN Stage 17 spec, all upstream logs
   - Diagnosed NNUE score magnitude problem (root cause: training normalization ÷3000 + Q6 quantization → single-digit centipawn outputs)
   - Identified fix: OUTPUT_MULTIPLIER in forward pass (~600-3000), tunable via NnueScale protocol option
   - Research on NNUE integration best practices (Stockfish, training data quality, SPRT validation)

2. **Hardware benchmarking:**
   - Measured: 3 games at depth 4 = 10m 34s → **~3.5 min/game**
   - Extrapolation: 100 games = ~5.8h, 500 = ~29h, 1000 = ~58h
   - PC specs documented: i5-10400F, 8GB RAM, GTX 1660 SUPER

3. **Training data generation infrastructure:**
   - `observer/config_training_d4_500.json` — config for 100 games at depth 4 with MoveNoise=40
   - `observer/run_training_batches.mjs` — batch runner: runs N batches of 100 games, extracts training data from each, merges and deduplicates into single JSONL file
   - Supports `--start-batch` for resuming partial runs
   - Estimated yield: ~60 positions per game → 1,000 games → ~60,000 training positions

4. **Stage 17 implementation plan** written at `.claude/plans/ethereal-honking-bee.md`

---

## What Was NOT Completed

- **NNUE code changes** (OUTPUT_MULTIPLIER, NnueScale option) — deferred until training data is collected
- **Training data generation** — infrastructure ready, run not started
- **NNUE retraining** — waiting on data
- **Self-play validation** (NNUE vs bootstrap duel) — waiting on retrained weights
- **Beam width experiments** — waiting on validated NNUE

---

## What the Next Session Should Do First

### IF training data generation is NOT yet running:
1. Start the 1K-game run split across 2 machines:
   ```bash
   cd observer
   # Machine A (500 games = 5 batches):
   node run_training_batches.mjs --batches 5 --engine "C:/rust-target/freyja/release/freyja.exe"

   # Machine B (500 games = 5 batches):
   node run_training_batches.mjs --batches 5 --engine "<path_to_freyja.exe>"
   ```
   Each takes ~29 hours. Output: `freyja-nnue/training_d4_all.jsonl`

### IF training data IS collected (merged JSONL exists):
1. **Phase 0: Fix score magnitude** — Add OUTPUT_MULTIPLIER (default ~600) to `nnue/features.rs`, apply in `nnue/forward.rs` line 64, add NnueScale protocol option
2. **Phase 2: Retrain NNUE** on expanded dataset:
   ```bash
   python -m freyja_nnue.train --data freyja-nnue/training_d4_all.jsonl --output weights_v2.fnnue --epochs 200 --batch-size 512
   ```
3. **Phase 3: Integration tests** — `cargo test`, 100 full-game lifecycle test
4. **Phase 4: Self-play validation** — NNUE vs bootstrap duel, 25 game pairs at depth 3
5. **Phase 5: Beam width experiments** (if Phase 4 passes)
6. **Phase 6: Ship decision** — make NNUE default if it wins >55% self-play

### Key technical details for the next agent:
- **Score magnitude root cause:** Training divides by 3000, Q6 quantization (×64), two ÷64 in forward pass → single-digit cp. Fix: multiply by OUTPUT_MULTIPLIER (~600) after forward pass, before the final ÷WEIGHT_SCALE division
- **Critical file:** `freyja-engine/src/nnue/forward.rs` line 64: `scores[i] = (raw / WEIGHT_SCALE) as i16` → needs `raw * OUTPUT_MULTIPLIER / WEIGHT_SCALE`
- **Training data depth:** User requires depth 4 ONLY. No depth 3.
- **Hardware limits:** 8GB RAM → 100-game batches max. Never run training + self-play simultaneously.
- **Full plan:** `.claude/plans/ethereal-honking-bee.md`

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **NNUE score magnitude (NOTE):** Root cause identified (arithmetic, not training bug). Fix planned but not implemented.
- **MoveNoise in MCTS:** Still unresolved. Hybrid mode provides diversity.

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 4, 7, 8, 11, 12, 17, 18, 19, 20, 21
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)
- PW k=2 vs k=4 A/B test (config ready, not run)

---

## Files Modified This Session (Session 28)

| File | Changes |
|------|---------|
| `observer/config_training_d4_500.json` (new) | Training data generation config (100 games, depth 4, MoveNoise 40) |
| `observer/run_training_batches.mjs` (new) | Batch runner: 5×100 games, extract, merge, dedup |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/STATUS.md` | Updated |
| `masterplan/sessions/session_028.md` (new) | Session note |
