# Audit Log — Stage 13: Time Management + Beam Width Tuning

**Date Started:** 2026-03-16
**Auditor:** Agent (Session 20)

---

## Pre-Audit

### Build State
- `cargo build`: PASS (compiles clean)
- `cargo test --lib`: PASS (380/381 tests pass; 1 slow test `test_eval_tuning_game_sim` skipped — hangs at depth 4 due to [[Issue-Depth4-Engine-Crash]])
- `cargo clippy`: not yet run (will verify after implementation)

### Upstream Logs Reviewed
- **[[audit_log_stage_09]]:** TT exact-only in Max^n, killer less effective, hit rate ~4-5% at beam 30. NPS improved to ~89.7k.
- **[[downstream_log_stage_09]]:** TT 20-byte entries, move ordering priority documented. History persists across ID iterations. Eval improvements: castling bonus, king exposure, attacker amplifier, dev cap.
- **[[audit_log_stage_12]]:** Self-play framework complete. 100 games @ depth 2 stable. SPRT working. Training data JSONL valid. NOTE: deterministic at same depth. WARNING: training data needs diverse configs.
- **[[downstream_log_stage_12]]:** Observer CLI, game JSON schema, A/B runner, SPRT class, metrics module all documented. Known limitation: deterministic engine, no randomization.

### Findings from Upstream
1. **[[Issue-Depth4-Engine-Crash]] (WARNING):** Qsearch explosion at depth 4 crashes engine. Root cause: no qsearch node budget. Fix is Step 1 of this stage.
2. **Deterministic self-play:** All games at same depth produce identical results. Fix is Step 4 (MoveNoise).
3. **Latent bug:** `options.beam_width` is parsed via `setoption` but `handle_go()` at `protocol/mod.rs:204` creates `HybridConfig::default()`, ignoring the option entirely. Fix is Step 2.
4. **TT hit rate low (4-5%):** Expected with beam 30. Should improve with narrower beam schedules (Step 5).

### Risks for This Stage
1. Qsearch node budget might cut off important capture chains — need to verify no strength regression
2. Beam schedule tuning is empirical — need diverse games (depends on MoveNoise working)
3. Time management branching factor estimate (4x) is a heuristic — may need per-depth calibration
4. Opening randomization must be zero-cost when MoveNoise=0

---

## Implementation Log

*(Filled during implementation)*

---

## Post-Audit

*(To be completed after implementation)*
