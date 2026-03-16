# Audit Log — Stage 12: Self-Play Framework

**Auditor:** Agent
**Date Started:** 2026-03-15
**Stage Spec:** MASTERPLAN Section 4, Stage 12

---

## Pre-Audit

### Build State
- `cargo build` — PASS
- `cargo test` — PASS (381 tests, 0 failures)
- `cargo clippy` — 0 warnings

### Upstream Logs Reviewed

**`downstream_log_stage_11.md`:**
- HybridSearcher is the active searcher (Max^n Phase 1 → MCTS Phase 2)
- 50/50 time split hardcoded (adaptive tuning deferred to Stage 13)
- Depth-only searches bypass MCTS
- Mate detection skips MCTS (score >= 9000cp)
- MCTS best_move is final choice; falls back to Max^n if MCTS returns None
- Evaluator must implement Clone
- HybridSearcher created fresh per `go` command — no persistent state
- SearchResult fields: best_move (MCTS), scores (MCTS), depth (Max^n), nodes (combined), pv (Max^n), tt_hit_rate (Max^n)

**`audit_log_stage_11.md`:**
- 21/21 hybrid tests pass (AC1-AC8)
- Hybrid overhead: <1ms per search
- History transfer verified working
- Phase 1 depth ~4 in 2.5s debug build
- No blocking findings

### Upstream Findings Affecting Stage 12
- **None blocking.** Self-play uses the protocol layer (Stage 4) and HybridSearcher (Stage 11). Both are stable.
- The `d` command outputs `fen4 <string>` — used for per-ply FEN4 capture.
- LogFile toggle and MaxRounds auto-stop are functional (Stage 4).
- Observer pipeline (`observer/observer.mjs`) already runs games and captures JSON.

### Risks for Stage 12
1. **100-game stability:** Engine hasn't been tested at 100-game scale. Potential for memory leaks, accumulated state issues.
2. **FEN4 capture overhead:** Adding `d` command per ply increases protocol round-trips. Expected negligible vs search time.
3. **SPRT for 4-player games:** Standard SPRT assumes binary outcomes. Need Gaussian adaptation for multi-player score comparison.
4. **Game result determination:** In 4-player FFA, "winning" is ambiguous. Using highest final score as winner.

---

## Post-Audit

**Date:** 2026-03-16
**Build State:** `cargo build` PASS, `cargo build --release` PASS, no engine-side changes required.

### Deliverables Check

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Self-play runner (observer) | DONE | Enhanced `observer/observer.mjs` with FEN4 capture, setoptions, game_result |
| Structured game JSON | DONE | Per-ply: fen4, scores, depth, nodes, qnodes, nps, tthitrate, killerhitrate, pv |
| Behavioral metrics | DONE | `observer/lib/metrics.mjs`: pawn ratio, queen activation, captures, king moves, shuffle index |
| Statistics aggregation | DONE | `observer/lib/stats.mjs`: win rates, CI, chi-squared, mean/stddev |
| A/B comparison | DONE | `observer/lib/ab.mjs` + `observer/ab_runner.mjs`: paired comparison, t-test, Elo estimate |
| SPRT | DONE | `observer/lib/sprt.mjs`: Gaussian SPRT on score differences |
| Training data extraction | DONE | `observer/lib/training_data.mjs` + `observer/extract_training.mjs`: JSONL output with filters |

### Acceptance Criteria Verification

| Criterion | Result | Details |
|-----------|--------|---------|
| 100 games without crash | PASS | 100 games @ depth 2, 80 ply each, 0 errors, 0 null FEN4 |
| Behavioral metrics computed | PASS | Pawn ratio 0.75, shuffle index 0.05, queen activation, king moves all populated |
| Statistics match distribution | PARTIAL | Deterministic engine produces identical games at same depth (expected). Chi-squared correctly identifies non-uniform results. |
| A/B detects improvement | PASS | Depth 1 vs depth 2 shows higher avg score for depth 2 (263.3 vs 306.5) |
| Training data valid | PASS | 76 unique records from 100 games, 0 invalid, all FEN4 parseable |
| SPRT identifies improvement | PASS | Accepted H1 after 2 pairs (depth 1 vs 2, score diff ~43cp) |

### Code Quality

**2.3 Code Bloat:** No bloat. Each module has a single clear purpose. Minimal code for each function.

**2.4 Redundancy:** `computeGameResult` logic is in observer.mjs and duplicated in ab_runner.mjs. Acceptable — ab_runner needs self-contained game play. Could refactor to shared module later.

**2.5 Dead Code:** None. All functions are used by their entry points.

**2.8 Naming:** All functions follow camelCase JS convention consistently. File names follow kebab_case with `.mjs` extension.

**2.14 Performance:** FEN4 capture adds ~1ms overhead per ply (one `d` command + line parse). Negligible vs search time (100ms-4s per ply).

**2.22 Magic Numbers:** `PAWNS_PER_PLAYER = 8` is named. Statistical thresholds (1.96 for 95% CI, 0.05 alpha/beta) are standard and self-documenting.

### Findings

**NOTE-001:** Deterministic engine at same depth. At depth 2, all 100 games produce identical results because the engine makes the same decisions from the same positions. This means identical-config A/B tests can't show variance. Not a bug — it's deterministic search. Adding randomization (opening book, noise injection) is deferred to Stage 13.

**NOTE-002:** SPRT uses Gaussian model. Standard chess SPRT uses pentanomial model on game outcomes. Our Gaussian SPRT on score differences is an approximation. Works empirically but formal validation would require Monte Carlo simulation.

**NOTE-003:** Max^n at depth 4 takes ~3-4 seconds per move in release build. 100 games at depth 4 with 200 ply would take hours. Self-play at higher depths requires movetime budgets or lower game counts.

**WARNING-001:** Training data deduplication across identical games yields only unique positions from a single game. To build meaningful training data, games must differ (different depths, beam widths, or opening randomization). Stage 13 tuning will address this.

### Risk Assessment for Stage 13

Stage 13 (Time + Beam Tuning) will use this self-play framework extensively. Key dependencies:
- A/B runner with SPRT for comparing beam width / time configurations
- Metrics for measuring play quality at different settings
- Need opening randomization to break determinism for meaningful comparisons

---

**Related:** [[MASTERPLAN]], [[AGENT_CONDUCT]], [[downstream_log_stage_11]]
