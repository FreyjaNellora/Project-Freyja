# Session 027 — Stage 16: NNUE Architecture + Training Pipeline

**Date:** 2026-03-21
**Stage:** 16 (complete)
**Tags:** `stage-16-complete` / `v1.16`

---

## Summary

Built the full NNUE pipeline end-to-end: Rust inference architecture, Python training pipeline, `.fnnue` binary format, and round-trip verification. This combined what the MASTERPLAN originally specified as Stage 15 (NNUE Architecture) and Stage 16 (Training Pipeline), since ADR-018's reshuffling displaced the Rust inference work.

## Key Decisions

- **Combined scope:** Stage 16 covers both Rust inference + Python training (original Stage 15 NNUE Architecture was displaced by ADR-018).
- **8 zone summary features** included in NNUE input from the start (total input = 4488 features per perspective).
- **Scalar forward pass only** (~27-50us). SIMD deferred to Stage 20 per ADR-012.
- **Output scale fix:** Changed from WEIGHT_SCALE² (4096) to WEIGHT_SCALE (64) because the hidden layer already divides once.
- **Heap allocation for feature weights:** `Box::new()` caused stack overflow (~2.3 MB array). Fixed with `vec![].into_boxed_slice()` conversion.

## Metrics

| Metric | Value |
|--------|-------|
| New Rust code | ~1,350 lines |
| New Python code | ~650 lines |
| New tests | 36 |
| Training records | 1,050 (d3/d4 self-play) |
| Training loss | 0.298 → 0.002 (50 epochs) |
| .fnnue file size | 2.3 MB |
| NNUE NPS (depth 1) | ~7k |

## Files Created

- `freyja-engine/src/nnue/` — 5 Rust files (features, accumulator, forward, weights, mod)
- `freyja-nnue/freyja_nnue/` — 6 Python files (data, model, train, export, verify, __init__)
- `freyja-nnue/weights.fnnue` — Trained NNUE weights
- `freyja-nnue/training_combined.jsonl` — Combined training data

## Files Modified

- `freyja-engine/src/lib.rs` — Added `pub mod nnue`
- `freyja-engine/src/eval.rs` — Made 5 zone functions `pub`
- `freyja-engine/src/protocol/options.rs` — EvalMode, NnueWeights options
- `freyja-engine/src/protocol/mod.rs` — NNUE evaluator wiring

## Known Issues for Stage 17

- NNUE scores are small (single-digit centipawns) — needs more training data or higher quantization scale
- No incremental accumulator updates — full refresh per eval is ~27-50us
- Random NNUE produces all-zero scores (quantized Xavier init too small)
