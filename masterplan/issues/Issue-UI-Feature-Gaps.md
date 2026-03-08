# Issue: UI Feature Gaps — Missing Components for Stages 7-10

**Severity:** Warning
**Stage:** Stage 5 (UI Shell) / affects Stages 7-10
**Status:** open
**Created:** 2026-03-07
**Last Updated:** 2026-03-07 (Session 010)
**Date Resolved:** —

---

## Description

Freyja's UI Shell (Stage 5) is minimal compared to Odin's mature UI. Several features are needed to effectively develop and debug Stages 7-10 (Max^n Search, Quiescence, TT + Move Ordering, MCTS). Without these, debugging relies on observer JSON dumps or the game log alone — neither gives real-time feedback during development.

Feature gap identified by comparing Freyja UI against Odin UI in Session 010.

## Impact

- **Stage 7 (Max^n Search):** Cannot see search depth, NPS, 4-player scores, or PV in real-time. Makes tuning and debugging search behavior difficult.
- **Stage 8 (Quiescence):** Cannot see seldepth or quiescence behavior.
- **Stage 9 (TT + Move Ordering):** Cannot see TT hit rates, move ordering quality.
- **Stage 10 (MCTS):** Cannot see MCTS simulations, top moves by visit count, search phase.
- **All stages:** No raw protocol log or manual command input in UI. No self-play dashboard for batch testing.

## Feature List

### Priority 1 — Port before/during Stage 7

| Feature | Odin Source | What It Shows | Why Needed |
|---------|-------------|---------------|------------|
| **Analysis Panel** | `AnalysisPanel.tsx` | NPS, depth, seldepth, score, nodes, time, PV | Primary search debugging tool. See depth progression, NPS, best line. |
| **Debug Console** | `DebugConsole.tsx` | Raw protocol log (color-coded) + manual command input | Send manual `go depth 5`, `position ... moves ...` commands. Catch protocol errors. |
| **Engine Internals** | `EngineInternals.tsx` | Per-player eval scores, search phase, stop reason | Freyja outputs `score red R blue B yellow Y green G` — need 4-player score grid, not single cp value. |

### Priority 2 — Port before Stage 10

| Feature | Odin Source | What It Shows | Why Needed |
|---------|-------------|---------------|------------|
| **Redo** | `useGameState.ts` | Complement to undo — restore undone moves | Replay specific positions during testing without restart. |
| **Self-Play Dashboard** | `SelfPlayDashboard.tsx` + `useSelfPlay.ts` | Automated N-game batches, win rates, avg game length/duration | Stress-test search without CLI observer. Stage 7 acceptance requires depth 5+ in 5s across many games. |
| **Max Rounds Slider** | `GameControls.tsx` | Cap game length (0=unlimited, N rounds = 4N ply) | Prevent runaway games during testing. Observer has `max_ply` but UI doesn't. |
| **Status Bar** | `StatusBar.tsx` | Engine name, connection status, manual connect button | Confirm which binary is loaded and whether it's alive. |
| **Mouse-Wheel Zoom** | `BoardDisplay.tsx` | Zoom in/out on board | Inspect crowded endgame positions. |

### Priority 3 — Stage 12+ (defer)

| Feature | When Needed | Notes |
|---------|-------------|-------|
| Self-play config save/restore | Stage 12 (Self-Play Framework) | Part of self-play dashboard maturation |
| Beam candidates display | Stage 9 (TT + Move Ordering) | Equivalent of Odin's "BRS surviving moves" |
| MCTS top moves + sims | Stage 10 (MCTS) | Direct port from Odin's EngineInternals |
| Game mode toggles (FFA/LKS) | Stage 18 (Game Mode Tuning) | Plumbing easier to add incrementally |
| Chess960 toggle | Stage 18+ | Not applicable until Fischer Random support |
| Terrain mode toggle | Stage 14+ (Zone Control) | Not applicable until terrain eval |
| Eval profile selector | Stage 18+ | Not applicable until multiple eval profiles |

## Reproduction

Launch Freyja UI → observe that there is no search info display, no raw protocol log, no manual command input, no self-play batch runner.

## Root Cause

Stage 5 spec was intentionally minimal ("UI Shell"). Full UI is Stage 19. However, development of Stages 7-10 requires real-time search feedback that the current UI does not provide.

## Resolution

Port Priority 1 features (Analysis Panel, Debug Console, Engine Internals) before or during Stage 7 implementation. Port Priority 2 features before Stage 10. This is development tooling, not premature Stage 19 work.

**Odin source files for reference:**
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\components\AnalysisPanel.tsx`
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\components\DebugConsole.tsx`
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\components\EngineInternals.tsx`
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\components\SelfPlayDashboard.tsx`
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\hooks\useSelfPlay.ts`
- `C:\Users\N8_Da\OneDrive\Desktop\Project_Odin\odin-ui\src\components\StatusBar.tsx`

## Verification

Each feature verified by launching the UI and confirming the component renders with live engine data.

---

**Related:** [[MASTERPLAN]], [[Session-010]], [[Component-Protocol]]
