# Issue: UI Start Button Does Nothing — Auto-Play Broken

**Severity:** Blocking
**Stage:** Stage 5 (UI Shell) / blocks Stage 7 UI testing
**Status:** resolved
**Created:** 2026-03-07
**Last Updated:** 2026-03-07 (Session 012)
**Date Resolved:** 2026-03-07

---

## Description

Clicking the Start button in the Freyja UI does nothing. The engine is connected (handshake visible in protocol log), all player slots show "Engine", the button toggles to "Stop" (indicating `autoPlay` state is true), but no `go` command is ever sent to the engine. The protocol log only shows the initial handshake (`freyja v1.0 maxn-beam-mcts`, `readyok`).

This completely blocks interactive Stage 7 testing via the UI.

## Root Cause Analysis

Investigated with 3 parallel agents. Confirmed NOT the issue: Tauri IPC parameter format, CSS overlays, `isGameOver` state, `sendGoRef` stub timing.

**Primary root cause:** `useEngine()` in `freyja-ui/src/hooks/useEngine.ts` returns a new object literal every render (lines 97-105). Even though the inner functions (`sendCommand`, `onMessage`, etc.) are stable `useCallback`s, the wrapper object has a new identity each render.

This causes a cascade:
1. `sendGoFromRef` in `useGameState.ts` depends on `[engine, setAutoPlay]` -- `engine` is new every render, so `sendGoFromRef` is recreated every render
2. `maybeChainEngineMove` depends on `[sendGoFromRef]` -- also recreated every render
3. The `onMessage` useEffect depends on `[engine, ..., maybeChainEngineMove]` -- re-runs every render, re-registering the message handler
4. Any transient failure in `engine.sendCommand` during this churn is silently caught (`.catch()` on line ~276 disables auto-play with no user feedback)

**Secondary cause:** Silent error swallowing in `sendGoFromRef`'s `.catch()` handler. If `invoke()` rejects for any reason, auto-play is silently disabled. The user sees "nothing happens" when in reality it started, failed, and silently stopped.

## Fix Plan

Full plan documented at `.claude/plans/binary-cuddling-rabin.md`. Summary:

### Step 1: Memoize `useEngine()` return value
File: `freyja-ui/src/hooks/useEngine.ts`
Wrap return object in `useMemo` so identity is stable when members haven't changed.

### Step 2: Destructure stable callbacks from `engine`
File: `freyja-ui/src/hooks/useGameState.ts`
Extract `const { sendCommand: engineSendCommand, onMessage: engineOnMessage } = engine;` at top. Replace all `engine.sendCommand(...)` with `engineSendCommand(...)`. Update dependency arrays.

### Step 3: Remove `sendGoRef` indirection
With `sendGoFromRef` now stable, the ref-to-function pattern is unnecessary. Call `sendGoFromRef()` directly in `setAutoPlay` and `togglePause`.

### Step 4: Stabilize `onMessage` useEffect
Use `engineOnMessage` instead of `engine.onMessage`. Effect runs once on mount.

### Step 5: Add error visibility
Add `console.error('[Freyja] sendGo failed:', err)` to catch handlers.

### Step 6: Clean up debug logs
Remove investigation console.logs, keep only error reporting.

## Reproduction

1. Launch `npx tauri dev` from `freyja-ui/`
2. Click "Connect Engine" in status bar
3. Set all player slots to Engine (or just click Start)
4. Click Start
5. Observe: button shows "Stop" but no moves happen, protocol log shows no `go` command

## Verification

After fix:
1. Start button fires `go` -- protocol log shows `position startpos` then `go`
2. Auto-play chains -- after each `bestmove`, next `position + go` follows automatically
3. Stop/Pause/Resume/New Game all work correctly
4. Console shows errors on engine failure (not silent)

---

**Related:** [[Issue-UI-Feature-Gaps]], [[Session-011]], [[Component-Protocol]]
