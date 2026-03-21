# Project Freyja -- HANDOFF

**Session Date:** 2026-03-21
**Session Number:** 27

---

## What Stage Are We On?

**Stage 16: NNUE Architecture + Training Pipeline -- COMPLETE (user signed off)**

Ready for Stage 17 (NNUE Integration).

---

## What Was Completed This Session

1. **Rust NNUE inference architecture** (`freyja-engine/src/nnue/`):
   - Feature encoding: 4488 features per perspective (4480 piece-square + 8 zone)
   - Accumulator: SIMD-ready (`align(32)`, ADR-012), full refresh per eval
   - Forward pass: quantized i16/i32 scalar (256→32→1 per perspective)
   - Weight format: `.fnnue` binary (little-endian i16, ~2.3 MB)
   - `NnueEvaluator` implementing `Evaluator` trait with `Arc<NnueWeights>` for cheap Clone
   - 36 new tests, all passing

2. **Protocol integration:**
   - `setoption name EvalMode value nnue` / `bootstrap`
   - `setoption name NnueWeights value <path>` — loads once, cached as `Arc`
   - Zone control functions made `pub` in eval.rs for direct NNUE access

3. **Python training pipeline** (`freyja-nnue/`):
   - FEN4 parser + feature extraction matching Rust encoding exactly
   - PyTorch `FreyjaNet` (shared weights across 4 perspectives)
   - Training loop: MSE loss on 4-vector, Adam optimizer, early stopping
   - Weight export: float32→i16 quantization, `.fnnue` format
   - Training result: loss 0.298 → 0.002 over 50 epochs on 1050 positions

4. **Round-trip verification:**
   - Trained `.fnnue` loads in Rust and produces non-zero differentiated scores
   - Trained NNUE produces position-sensitive evaluations vs random (all-zero)

5. **Tags:** `stage-15-complete` / `v1.15` (confirmed existing), `stage-16-complete` / `v1.16`

---

## What Was NOT Completed

- **SIMD forward pass** — Scalar only (~27-50us). AVX2 deferred to Stage 20 per plan.
- **Incremental accumulator** — Full refresh per eval. Incremental make/unmake deferred to Stage 17/20.
- **Large-scale training** — 1050 positions from existing games. Weights produce small scores (single-digit centipawns). More data + deeper search needed for Stage 17.
- **Formal A/B self-play** (trained vs random) — Verified qualitatively (differentiated vs flat scores), not via 100-game SPRT.

---

## What the Next Session Should Do First

1. Begin Stage 17: NNUE Integration
2. Wire `NnueEvaluator` as the default evaluator (replace bootstrap)
3. Generate more training data at higher depth with NNUE eval for iterative improvement
4. Beam width experiment: test tighter beam with NNUE ordering
5. Key concern: trained weights produce small scores — may need higher quantization scale or more training data before NNUE can actually beat bootstrap in self-play

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **NNUE score magnitude (NOTE):** Trained weights produce single-digit centipawn scores. Quantization scale (Q6=64) is conservative for the small learned weights. Stage 17 should investigate higher scale or more training epochs.
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

## Files Modified This Session (Session 27)

| File | Changes |
|------|---------|
| `freyja-engine/src/nnue/` (new dir) | Full NNUE inference module (5 files) |
| `freyja-engine/src/lib.rs` | Added `pub mod nnue` |
| `freyja-engine/src/eval.rs` | Made zone functions `pub` |
| `freyja-engine/src/protocol/options.rs` | EvalMode, NnueWeights options |
| `freyja-engine/src/protocol/mod.rs` | NNUE evaluator wiring, weight caching |
| `freyja-nnue/` (new dir) | Python training pipeline (6 files) |
| `freyja-nnue/weights.fnnue` | Trained NNUE weights |
| `freyja-nnue/training_combined.jsonl` | 1050 training records |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/STATUS.md` | Updated |
