# Issue: Tauri IPC Hang at Ply 30+

**Severity:** BLOCKING
**Stage:** Stage 15 (affects UI sign-off)
**Status:** Open
**Date Created:** 2026-03-21
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

## Reproduction

1. Launch UI (`npm run tauri dev` from freyja-ui/)
2. Set all players to Engine
3. Click Start
4. Wait ~30 plies (~2-5 minutes depending on depth)
5. Game freezes. Console shows watchdog firing.

## Root Cause Analysis

The `send_command` Tauri command acquires a mutex on `AppState`, writes to engine stdin via `BufWriter`, and flushes. The position command with 30+ moves is ~200+ characters. The invoke hangs — likely due to:

1. **Stdin pipe buffer contention:** The engine's stdout reader thread emits events via `app_handle.emit()`. If the emit blocks (Tauri issue #8177), and the main thread's invoke also needs the IPC bridge, there could be a deadlock.
2. **Mutex contention:** `send_command` holds the AppState mutex while writing to stdin. If another Tauri command also needs the mutex, or if the stdin write blocks, the invoke hangs.
3. **Tauri IPC serialization:** The position string with 30+ moves may exceed some internal buffer or serialization limit in Tauri's invoke mechanism.

## Proof Engine Works

```bash
printf "position startpos moves <30 moves>\ngo\n" | target/release/freyja.exe
```
Returns bestmove in ~3 seconds at ply 30, ~5 seconds at ply 32 (MCTS). No hang.

## Recommended Fixes

### Option A: Use FEN4 instead of move list (Quick Fix)
Replace `position startpos moves ...` with `position fen4 <fen>` for all mid-game commands. FEN4 is constant-size (~100 chars) regardless of game length. The engine already supports `position fen4`. The UI already retrieves FEN4 via the `d` command.

### Option B: Observer-based architecture (Architecture Fix)
Port Odin's observer-based engine communication. Bypass Tauri's invoke/event system entirely. Spawn the engine from the Rust backend, communicate via raw stdin/stdout in a dedicated thread, and forward parsed messages to the frontend via a simpler channel (e.g., Tauri events without the mutex-locked invoke path).

### Option C: Chunked position replay (Workaround)
Break the position command into smaller chunks: send `position startpos moves <first 15 moves>`, then `position fen4 <mid-fen> moves <remaining moves>`. Keeps each invoke below the hang threshold.

---

**Related:** [[HANDOFF]], [[Session-025]]
