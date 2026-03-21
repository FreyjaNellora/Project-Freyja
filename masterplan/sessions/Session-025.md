# Session 025 — Stage 15 Wrap-up, UI Hang Diagnosed

**Date:** 2026-03-21
**Stage:** 15 (engine complete, UI blocking)
**Focus:** Swarm validation, zero-centering, UI bug diagnosis + partial fix

---

## What Happened

### Swarm Model
- Implemented mutual defense, attack coordination, pawn chain detection
- Duel-tested: swarm+ray beats ray-only 9/15 (60%) across all seating arrangements
- ADR-021 accepted: swarm replaces BFS Voronoi

### Zero-Centered Eval
- Research: constant baselines break MCTS exploration, beam discrimination, NNUE training
- Fix: subtract mean across active players after computing all scores
- Scores now in normal centipawn range (-300 to +300 vs 2000+ before)

### Engine Binary Issue
- .cargo/config.toml (from research agent) redirected target-dir to C:/rust-target/freyja/
- Old binary in target/release/ was from March 19 (pre-Stage 15)
- Tauri UI spawned the old binary — explains why scores were inflated in UI
- Fixed: removed config.toml, rebuilt into target/

### UI Player Tracking Fix
- Root cause: nextturn arrives from BOTH position replay AND post-bestmove
- During auto-play, the double-update corrupted movingPlayer, shifting labels by 1
- Fix: playerWhenGoSentRef captures who's moving at send time
- Fix: nextturn suppressed while awaitingBestmoveRef is true
- Fix: advancePlayer used for chaining, never pendingNextTurnRef

### UI Ply-30 Hang (BLOCKING — NOT FIXED)
- Game plays 30 plies correctly, then freezes
- Console shows: sendGoFromRef called, but position command invoke never resolves
- Engine works fine via CLI at all ply counts
- Root cause: Tauri invoke('send_command') hangs with long position strings
- Recommended fix: use FEN4 (constant-size) or observer-based architecture (bypass Tauri IPC)

---

## Research Conducted

1. **Eval score normalization** — Multi-player game theory (Korf 1991, Sturtevant 2003), Stockfish normalization, NNUE training sigmoid saturation, MCTS Q-value balance. Conclusion: zero-center by subtracting mean.

2. **Tauri IPC issues** — Issue #8177 (event emission crash at high rate), #9453 (deadlock), chess programming stdout buffering best practices. Conclusion: Tauri's invoke/event system not designed for rapid engine IO.

3. **Voronoi correctness** — BFS Voronoi limitations for chess (circular not directional, all pieces equal, ignores obstacles). Led to ray-attenuation model, then swarm.

---

## Commits This Session

- `7ef8360` — Swarm mechanics implementation
- `82c1c5d` — Duel runner with random seatings
- `6867d1b` — Duel results (swarm wins 9/15)
- `801d2ad` — Session 24 docs
- `fc27afb` — Zero-center eval scores
- `339af9d` — UI player tracking fix + watchdog

---

## Test Results

- 441 engine lib tests pass, 0 fail
- Duel: 15 games, swarm+ray wins 60%
- UI: plays 30 plies correctly with proper player labels, hangs at ply 31
