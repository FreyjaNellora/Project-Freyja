# Project Freyja -- HANDOFF

**Session Date:** 2026-03-16
**Session Number:** 19

---

## What Stage Are We On?

**Stage 12: Self-Play Framework -- COMPLETE** (tagged `stage-12-complete` / `v1.12`)
**Next: Stage 13 (Time + Beam Tuning)**

---

## What Was Completed This Session

1. **Stage 12 marked complete** — user gave green light
2. **Tagged:** `stage-12-complete` / `v1.12`
3. **Discussion:** Quiescence search explosion at depth 4, how to balance qsearch depth vs main search depth in 4-player chess. Key insight: qsearch isn't supposed to find brilliance — it resolves hanging pieces. Deep sacrifices are the main search's job (and eventually NNUE's).

---

## What Was NOT Completed

- Stage 13 implementation (not started)
- Depth 4 qsearch crash fix (filed as Issue-Depth4-Engine-Crash)

---

## What the Next Session Should Do First

1. Read Stage 13 spec in MASTERPLAN
2. Read upstream audit/downstream logs (Stage 9 + Stage 12)
3. Key priorities for Stage 13:
   - Opening randomization (needed for non-deterministic A/B testing)
   - Qsearch node budget or beam-on-captures to bound explosion
   - Time management (movetime → depth allocation)
   - Beam width tuning experiments with data
4. Investigate depth 4 crash — likely qsearch explosion, see [[Issue-Depth4-Engine-Crash]]

---

## Open Issues / Discoveries

- **[[Issue-Depth4-Engine-Crash]] (WARNING):** Engine crashes at depth 4 from qsearch explosion. Games run fine at depth 2 and 3. Qsearch generates too many capture chains at depth 4. Possible fixes: node budget, beam on captures, adaptive qsearch depth.
- **Deterministic self-play (NOTE):** At any fixed depth, all games produce identical results. Need opening randomization for meaningful A/B comparisons. Stage 13 priority.
- **[[Issue-UI-Feature-Gaps]] (WARNING):** Stale since Session 10. Still relevant but not blocking.
- **First-mover advantage (NOTE):** Yellow wins 100% at shallow depth due to move order. Expected to diminish with deeper search and better eval.

---

## Files Modified This Session

| File | Action |
|------|--------|
| `masterplan/STATUS.md` | Updated — Stage 12 complete |
| `masterplan/HANDOFF.md` | Rewritten |
| `masterplan/sessions/Session-019.md` | **CREATED** |
| `masterplan/_index/MOC-Sessions.md` | Updated |
| `masterplan/_index/MOC-Active-Issues.md` | Updated |
| `masterplan/_index/Wikilink-Registry.md` | Updated |

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12, 17, 18
- Dead code: `apply_move_with_events` in `game_state.rs`
- Search time abort bug: debug build ignores time budget at higher depths
- Eval suite systematic tuning (Stage 13)
- Opening randomization for non-deterministic self-play (Stage 13)
