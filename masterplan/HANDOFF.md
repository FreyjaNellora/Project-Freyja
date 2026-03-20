# Project Freyja -- HANDOFF

**Session Date:** 2026-03-19
**Session Number:** 21

---

## What Stage Are We On?

**Stage 14: MCTS Opponent Move Abstraction (OMA) -- IMPLEMENTATION COMPLETE, AWAITING USER SIGN-OFF**
**Next: Stage 15 (Progressive Widening + Zone Control)**

---

## What Was Completed This Session

1. **Tier 5 boundary review** — all invariants pass, no blocking issues, user signed off
2. **OMA core implementation:**
   - `SimStep` enum for mixed tree/OMA path tracking
   - `OmaPolicy`: checkmate > capture > check > history > random (no evaluator calls)
   - OMA branch in `run_simulation` with **stored moves per tree node** (first visit: compute, revisit: replay)
   - `use_oma: bool` in MctsConfig (default true)
   - `OpponentAbstraction` setoption
   - OMA diagnostic counters (oma_moves_total, root_decisions_total)
   - 9 dedicated OMA unit tests, all 408 tests pass
3. **Critical bug fix: Sigma transform saturation (Stage 10 bug)**
   - Sequential Halving Q-values normalized to [0,1] instead of `/100`
   - Gumbel exploration was effectively disabled since Stage 10
4. **Observer bug fixes:**
   - Player label off-by-one (double advancement) in ab_runner.mjs and observer.mjs
   - Discovered MoveNoise doesn't apply to MCTS (only Max^n)
5. **A/B test:** OMA on vs off, 10 games each, 2s movetime — no significant difference (Elo -4.8, p=0.993). Diverse winners, 0 crashes.
6. **Research findings** from Baier & Kaisers 2020, GameAIPro MCTS pitfalls, Gumbel AlphaZero paper — documented in audit log and pattern notes.

---

## What Was NOT Completed

- **AC1 verification at scale:** "3-4x deeper root decisions" needs longer time control testing
- **MCTS noise mechanism:** MoveNoise doesn't work in MCTS phase. Needs investigation for diverse MCTS-only self-play.

---

## What the Next Session Should Do First

1. **Get user sign-off on Stage 14** from UI testing
2. If approved, tag `stage-14-complete` / `v1.14`
3. Read Stage 15 spec (Progressive Widening + Zone Control)

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **[[Issue-Sigma-Transform-Saturation]] (RESOLVED):** Fixed in this session.
- **MoveNoise in MCTS:** Not yet addressed. MCTS-only mode produces identical games.

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18, 19, 20
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)

---

## Files Modified This Session

| File | Changes |
|------|---------|
| `freyja-engine/src/mcts.rs` | SimStep, OmaPolicy, OMA branch, stored moves, sigma fix, metrics, 9 tests |
| `freyja-engine/src/protocol/options.rs` | OpponentAbstraction setoption |
| `freyja-engine/src/board/mod.rs` | Minor formatting (cargo fmt) |
| `observer/ab_runner.mjs` | Player label fix, NoiseSeed investigation |
| `observer/observer.mjs` | Player label fix |
| `observer/config_ab_oma.json` | A/B test configuration |
| `masterplan/tier_boundary_review_5.md` | NEW — Tier 5 boundary review |
| `masterplan/audit_log_stage_14.md` | NEW — pre-audit + post-audit |
| `masterplan/downstream_log_stage_14.md` | NEW — API contracts, limitations |
| `masterplan/patterns/Pattern-OMA-Stored-Moves.md` | NEW — pattern note |
| `masterplan/issues/Issue-Sigma-Transform-Saturation.md` | NEW — resolved issue |
