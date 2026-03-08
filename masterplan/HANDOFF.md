# Project Freyja -- HANDOFF

**Session Date:** 2026-03-07
**Session Number:** 12

---

## What Stage Are We On?

**Stage 7: Max^n Search -- IN PROGRESS (awaiting user green light)**

Stage 7 engine search is COMPLETE and VERIFIED in UI. Auto-play works: engine plays all 4 sides, moves chain automatically, analysis panel shows depth/nodes/NPS/scores/PV.

---

## What Was Completed This Session

1. **Fixed UI auto-play bug ([[Issue-UI-AutoPlay-Broken]]):**
   - Memoized `useEngine()` return with `useMemo` (stops new object identity every render)
   - Destructured stable callbacks (`engineSendCommand`, `engineOnMessage`) from engine object
   - Moved `sendGoFromRef` definition above `setAutoPlay` (fixed const temporal dead zone)
   - Made all UI control setters update refs synchronously (prevents React batching stale reads)
   - Removed `sendGoRef` ref indirection (no longer needed with stable callbacks)
   - Added `console.error` to catch handlers for error visibility
   - Added try/catch to `spawnEngine` for visible spawn failures

2. **Fixed engine binary path resolution:**
   - Tauri backend now prefers release builds over debug (debug is too slow for search time budgets)
   - Release engine: ~84k NPS, depth 4 in ~1.5s with 2s budget
   - Debug engine: ~11k NPS, depth 4 takes 15+ seconds (search abort doesn't respect time limit)

3. **Updated project status:**
   - Marked [[Issue-UI-AutoPlay-Broken]] as resolved
   - Updated STATUS.md, MOC-Active-Issues.md
   - Zero blocking issues remaining

---

## What Was NOT Completed

- Stage 7 formal completion (post-audit, tagging, user green light)
- Git commits for changes
- Stage 5 deferred debt (post-audit, downstream_log, vault notes)
- Session notes for Sessions 7, 8, 11, 12
- Debug build search time abort bug (deferred — only affects debug, release works)

---

## What the Next Session Should Do First

1. Get user green light on Stage 7 (watch engine play in UI)
2. Complete Stage 7 formalities (post-audit, tag `stage-07-complete` / `v1.7`)
3. Begin Stage 8 (Quiescence Search) planning

---

## Open Issues / Discoveries

- **[[Issue-UI-Feature-Gaps]] (WARNING):** UI missing Debug Console, Engine Internals needed for Stages 8-10.
- **Search time abort bug (NOTE):** Debug build doesn't respect 2s time budget at depth 4+ (only matters for debug, release works correctly). Root cause: `should_abort()` uses `nodes & 1023 == 0` optimization that can skip time checks between depth iterations.

---

## Files Modified This Session

| File | Action |
|------|--------|
| `freyja-ui/src/hooks/useEngine.ts` | Memoized return, error handling on spawn |
| `freyja-ui/src/hooks/useGameState.ts` | Full auto-play fix (destructure, reorder, stable deps, ref sync) |
| `freyja-ui/src-tauri/src/engine.rs` | Prefer release binary path |
| `masterplan/issues/Issue-UI-AutoPlay-Broken.md` | Marked resolved |
| `masterplan/STATUS.md` | Updated (no blocking issues) |
| `masterplan/_index/MOC-Active-Issues.md` | Updated |
| `masterplan/HANDOFF.md` | Rewritten |

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 7, 8, 11, 12
- Remove dead code: `apply_move_with_events` in `game_state.rs`
- Debug build search time abort bug
