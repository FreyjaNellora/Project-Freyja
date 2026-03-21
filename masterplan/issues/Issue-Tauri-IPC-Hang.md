# Issue: Tauri IPC Hang at Ply 30+

**Severity:** BLOCKING
**Stage:** Stage 15 (affects UI sign-off)
**Status:** Resolved
**Date Created:** 2026-03-21
**Date Resolved:** 2026-03-21
**Last Updated:** 2026-03-21

---

## Description

The Freyja UI freezes after ~30 plies of auto-play. The engine continues to run but the UI never receives the bestmove response. The Tauri `invoke('send_command')` promise never resolves when the position command contains 30+ moves.

## Symptoms

- Game plays 30 plies correctly with proper player labels and scores
- At ply 31, the UI calls `sendGoFromRef()` which sends `position startpos moves <30 moves>`
- The `engineSendCommand(posCmd)` invoke never resolves (promise hangs)
- The `go` command is never sent (chained after position via `.then()`)
- Watchdog fires after 30 seconds, recovering `awaitingBestmoveRef`
- Engine process is still alive and responsive via CLI

## Root Cause

**Primary: Undrained stderr pipe buffer.** The engine writes tracing output to stderr (`tracing_subscriber` configured to write to stderr in `main.rs`). The Tauri backend piped stderr (`Stdio::piped()`) but never spawned a reader thread. On Windows, the OS pipe buffer is ~64KB. Once the engine accumulated enough stderr output (~30 plies of search tracing), the buffer filled, and any subsequent `eprintln!` or tracing write blocked the engine process. This blocked the engine mid-search, preventing bestmove output.

**Secondary contributor: Growing position strings.** The `position startpos moves ...` command grew with every ply (~200+ chars at ply 30), increasing Tauri IPC serialization overhead. Not the primary cause, but switching to FEN4 provides defense-in-depth.

## Resolution

Two fixes applied in Session 26:

1. **Stderr drain thread** (`engine.rs` lines 69-81): Background thread reads and discards all stderr output, preventing pipe buffer deadlock.
2. **FEN4-based position commands** (`useGameState.ts` line 115-117): Replaced `position startpos moves ...` with `position fen4 <fen>` for all mid-game commands. Constant-size (~100 chars) regardless of game length.
3. **Watchdog timeout increased** to 10 minutes (from 30 seconds) to accommodate deep searches.

## Verification

User tested: game plays past ply 32 at depth 4 with all 4 engine players. No hang.

---

**Related:** [[HANDOFF]], [[Session-025]], [[Session-026]], [[audit_log_stage_15]]
