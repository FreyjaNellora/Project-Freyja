# Session 026: Stage 15 Closure — UI IPC Fix

**Date:** 2026-03-21
**Stage:** Stage 15
**Duration:** ~1h

---

## Goals

Fix the Tauri UI ply-30 hang so the engine can play full games in the UI, enabling user sign-off for Stage 15.

## Completed

- **FEN4-based position commands** — Replaced `position startpos moves ...` (grows with every ply) with `position fen4 <fen>` (constant-size) in `useGameState.ts`. The engine already supported `position fen4`. The UI already retrieved FEN4 via the `d` command.
- **Stderr drain thread** — Added background thread in `engine.rs` to drain the engine's stderr pipe. The engine writes tracing output to stderr; without draining, the 64KB Windows pipe buffer fills up and the engine process deadlocks. This was the actual root cause of the ply-30 hang.
- **Watchdog timeout increased** — Changed from 30 seconds to 10 minutes to accommodate deep searches.
- **Depth tested at 4 and 8** — Depth 4 works smoothly. Depth 8 is too slow (~7.5+ min/move) but the engine doesn't hang.
- **User sign-off obtained** — Game plays past ply 32 at depth 4 with all engine players.
- **Stage 15 audit log and downstream log written.**

## Not Completed

- PW k=2 vs k=4 A/B test — deferred, config ready
- Odin observer architecture port — not needed after stderr fix

## Discoveries

- **The FEN4 fix alone was not sufficient.** After switching to FEN4, the hang still occurred at ply 31. The actual root cause was the undrained stderr pipe buffer. The FEN4 fix is still valuable as defense-in-depth (constant-size IPC messages).
- **Stderr pipe deadlock is a Windows-specific footgun.** On Windows, if a child process writes to a piped stderr that nobody reads, the process blocks once the ~64KB OS buffer fills. This manifests as a hang at a consistent ply count regardless of other changes.

## Decisions Made

- ADR-022 (proposed): Use FEN4-based position commands in all UI-engine communication. Constant-size, no accumulation.

## Issues Created/Resolved

| Issue | Action | Severity |
|-------|--------|----------|
| [[Issue-Tauri-IPC-Hang]] | Resolved | BLOCKING |

## Files Modified

| File | Changes |
|------|---------|
| `freyja-ui/src/hooks/useGameState.ts` | FEN4 position commands, watchdog 30s → 10min, depth 4 |
| `freyja-ui/src-tauri/src/engine.rs` | Stderr drain thread |
| `masterplan/audit_log_stage_15.md` | New |
| `masterplan/downstream_log_stage_15.md` | New |
| `masterplan/sessions/Session-026.md` | This file |
| `masterplan/HANDOFF.md` | Updated |
| `masterplan/STATUS.md` | Updated |

## Next Session Should

1. Tag `stage-15-complete` / `v1.15`
2. Begin Stage 16: NNUE Training Pipeline
3. Consider running PW A/B test if time permits

---

**Related:** [[HANDOFF]], [[STATUS]], [[Issue-Tauri-IPC-Hang]], [[Session-025]]
