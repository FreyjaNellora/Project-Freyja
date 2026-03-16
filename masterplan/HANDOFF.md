# Project Freyja -- HANDOFF

**Session Date:** 2026-03-16
**Session Number:** 18

---

## What Stage Are We On?

**Stage 12: Self-Play Framework -- AWAITING USER VERIFICATION**
**Next: Stage 13 (Time + Beam Tuning)**

All deliverables implemented and tested. No engine-side Rust changes — all work in the Node.js observer pipeline (`observer/`).

---

## What Was Completed This Session

1. **Tier 4 Boundary Review:**
   - Created `masterplan/tier_boundary_review_4.md`
   - All maintenance invariants pass, no blocking issues
   - Hot-path data structures verified fixed-size

2. **Stage 12 Full Implementation:**
   - Enhanced `observer/lib/engine.mjs` — FEN4 parsing, tthitrate/killerhitrate, sendOptions(), drainUntilReady()
   - Enhanced `observer/observer.mjs` — FEN4 capture per ply via `d` command, game_result computation, setoptions support, movetime support, MaxRounds handling
   - Created `observer/lib/fen4_parser.mjs` — Lightweight FEN4 parser (2-char `rP`/`bK`/`yQ`/`gN` encoding)
   - Created `observer/lib/metrics.mjs` — Pawn ratio, queen activation, captures per 10 rounds, king moves, shuffle index
   - Created `observer/lib/stats.mjs` — Mean/stddev/CI95, chi-squared, t-test, win rate aggregation
   - Created `observer/lib/ab.mjs` — A/B comparison logic with Elo estimation
   - Created `observer/ab_runner.mjs` — A/B runner with optional SPRT early stopping
   - Created `observer/lib/sprt.mjs` — Gaussian SPRT on score differences
   - Created `observer/lib/training_data.mjs` + `observer/extract_training.mjs` — JSONL training data extraction with filters

3. **Validation:**
   - 100 games @ depth 2: 0 errors, all FEN4 captured, metrics computed
   - A/B smoke: depth 1 vs depth 2 comparison works
   - SPRT smoke: accepted H1 after 2 pairs (depth 1 vs 2)
   - Training data: 76 unique valid records from 100 games

4. **Documentation:**
   - Created `masterplan/audit_log_stage_12.md` (pre-audit + post-audit)
   - Created `masterplan/downstream_log_stage_12.md`
   - Created `masterplan/tier_boundary_review_4.md`
   - Updated `masterplan/STATUS.md` and `masterplan/HANDOFF.md`

---

## What Was NOT Completed

- User verification of Stage 12
- Cargo test verification in release mode (test compilation was slow)
- 100 games at depth 4+ (takes hours — depth 2 was used for bulk stability)
- Session note for Session 18

---

## What the Next Session Should Do First

1. User verifies Stage 12 (run observer, check A/B, check training data)
2. If approved: tag `stage-12-complete` / `v1.12`
3. Begin Stage 13 (Time + Beam Tuning) — key need: opening randomization for non-deterministic A/B testing

---

## Open Issues / Discoveries

- **Deterministic self-play (NOTE):** At any fixed depth, all games produce identical results (same engine, same position). Need opening randomization for meaningful A/B comparisons. Deferred to Stage 13.
- **[[Issue-UI-Feature-Gaps]] (WARNING):** Stale since Session 10, reviewed Session 18. Still relevant but not blocking self-play work.
- **MCTS skipped with depth-only searches (NOTE):** `go depth N` only runs Max^n. Need `go movetime N` for full HybridSearcher. Documented in downstream_log_stage_12.

---

## Files Modified This Session

| File | Action |
|------|--------|
| `observer/lib/engine.mjs` | Modified — FEN4 parsing, tthitrate/killerhitrate, sendOptions, getFEN4, drainUntilReady |
| `observer/observer.mjs` | Modified — FEN4 capture, game_result, setoptions, movetime, MaxRounds handling |
| `observer/lib/fen4_parser.mjs` | **CREATED** — Lightweight FEN4 parser |
| `observer/lib/metrics.mjs` | **CREATED** — Behavioral metrics |
| `observer/lib/stats.mjs` | **CREATED** — Statistical aggregation |
| `observer/lib/ab.mjs` | **CREATED** — A/B comparison logic |
| `observer/ab_runner.mjs` | **CREATED** — A/B runner entry point |
| `observer/lib/sprt.mjs` | **CREATED** — SPRT implementation |
| `observer/lib/training_data.mjs` | **CREATED** — Training data extraction |
| `observer/extract_training.mjs` | **CREATED** — Training data CLI |
| `observer/config_smoke.json` | **CREATED** — Smoke test config |
| `observer/config_ab_smoke.json` | **CREATED** — A/B smoke test config |
| `observer/config_sprt_smoke.json` | **CREATED** — SPRT smoke test config |
| `observer/config_validation.json` | **CREATED** — Depth 3 validation config |
| `masterplan/tier_boundary_review_4.md` | **CREATED** |
| `masterplan/audit_log_stage_12.md` | **CREATED** |
| `masterplan/downstream_log_stage_12.md` | **CREATED** |
| `masterplan/STATUS.md` | Updated — Stage 12 status |
| `masterplan/HANDOFF.md` | Rewritten |

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores time budget at higher depths
- Eval suite systematic tuning (Stage 13)
- Opening randomization for non-deterministic self-play (Stage 13)
