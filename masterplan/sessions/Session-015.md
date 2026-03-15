# Session 015

**Date:** 2026-03-14
**Stage:** 9 → Complete
**Focus:** Stage 9 verification, eval improvements, observer infrastructure

---

## Summary

Verified Stage 9 (TT + Move Ordering) meets all acceptance criteria. User gave greenlight. Tagged `stage-09-complete` / `v1.9`.

Attempted eval tuning based on observer baseline test results (17/39, 44% FAIL). Added four eval improvements: castling bonus, king exposure penalty, attacker amplifier, development cap. Testing revealed that depth-2 search is too shallow for eval changes to measurably affect bestmove selection — the improvements show at depth 1 (+140cp for castling) but opponent responses at depth 2 dominate. Systematic tuning deferred to Stage 13.

Created observer eval suite infrastructure (`observer/baselines/`) with 25 tactical positions from 3000+ Elo human games, scoring rubric, and diagnostic tooling.

## Key Decisions

- **Eval tuning deferred to Stage 13:** Depth-2 eval suite can't validate weight changes. Needs self-play A/B testing (Stage 12) and deeper search to be meaningful.
- **Keep eval improvements:** Castling bonus, king exposure, dev cap, attacker amplifier are structurally correct and improve eval quality at all depths. No harm keeping them in.
- **Eval suite depth 4:** User preference. Updated from depth 2 to depth 4 in `run_eval_suite.mjs`.
- **No `has_castled` flag:** Castling detection inferred from king position + revoked rights. Avoids invasive changes to Board/make_move/unmake_move for a bootstrap eval feature.

## Files Modified

- `freyja-engine/src/eval.rs` — castling bonus, king exposure, dev cap, attacker amplifier
- `freyja-engine/src/search.rs` — eval tuning game sim test
- `freyja-engine/src/protocol/{commands,mod,parse}.rs` — `d`/`debug` command
- `observer/baselines/*` — eval suite infrastructure (new)
- `tools/observer.sh` — legacy observer (new)
- `masterplan/STATUS.md`, `HANDOFF.md`, `downstream_log_stage_09.md` — updated
