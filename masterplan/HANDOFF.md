# Project Freyja -- HANDOFF

**Session Date:** 2026-03-21
**Session Number:** 26

---

## What Stage Are We On?

**Stage 15: Progressive Widening + Zone Control -- COMPLETE (user signed off)**

Ready for tagging and Stage 16.

---

## What Was Completed This Session

1. **Fixed Tauri IPC ply-30 hang** — Two fixes applied:
   - **Stderr drain thread** in `engine.rs` — The engine writes tracing output to stderr. Without a reader thread, the 64KB Windows pipe buffer fills and the engine deadlocks. This was the actual root cause.
   - **FEN4-based position commands** in `useGameState.ts` — Replaced `position startpos moves <growing list>` with `position fen4 <constant-size>`. Defense-in-depth.

2. **Watchdog timeout 30s → 10min** — Depth 8 takes 7.5+ minutes per move. 30s was too aggressive.

3. **User sign-off obtained** — Game plays past ply 32 at depth 4 with all engine players.

4. **Stage 15 audit log + downstream log written.**

---

## What Was NOT Completed

- **Odin observer architecture port** — Not needed after stderr fix.
- **PW k=2 vs k=4 A/B test** — Config exists, not run.
- **Performance benchmark** — Zone features ~25% slower (12k vs 16k NPS), not formally benchmarked.

---

## What the Next Session Should Do First

1. Tag `stage-15-complete` / `v1.15`
2. Begin Stage 16: NNUE Training Pipeline
3. Address eval quality: engine misses obvious defensive/attack positions (e.g., allows queen to be captured when bishop could defend). This is the hand-tuned eval ceiling — NNUE is the fix.

---

## Open Issues

- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **Eval quality (NOTE):** Engine makes suboptimal moves — misses obvious defenses and attacks. Known limitation of hand-tuned eval at depth 4. NNUE (Stage 16-17) is the intended fix.
- **MoveNoise in MCTS:** Still unresolved. Hybrid mode provides diversity for testing.

---

## Key Observation: Eval Quality

User observed during testing: Red sacrifices its queen by letting Green take it ~move 8. Red could defend with its bishop but moves a knight instead. This is a clear example of the hand-tuned eval's tactical blindness at depth 4. The eval doesn't sufficiently penalize leaving high-value pieces hanging. NNUE training data from deeper searches (or from Odin) should teach proper piece safety.

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 4, 7, 8, 11, 12, 17, 18, 19, 20, 21
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)
- PW k=2 vs k=4 A/B test (config ready, not run)

---

## Files Modified This Session (Session 26)

| File | Changes |
|------|---------|
| `freyja-ui/src/hooks/useGameState.ts` | FEN4 position commands, watchdog 30s → 10min, depth 4 |
| `freyja-ui/src-tauri/src/engine.rs` | Stderr drain thread |
| `masterplan/audit_log_stage_15.md` | New |
| `masterplan/downstream_log_stage_15.md` | New |
| `masterplan/sessions/Session-026.md` | New |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/STATUS.md` | Updated |
| `masterplan/_index/MOC-Active-Issues.md` | Issue resolved |
| `masterplan/_index/Wikilink-Registry.md` | New entries |
| `masterplan/issues/Issue-Tauri-IPC-Hang.md` | Resolved |
