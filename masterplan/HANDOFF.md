# Project Freyja -- HANDOFF

**Session Date:** 2026-03-21
**Session Number:** 25

---

## What Stage Are We On?

**Stage 15: Progressive Widening + Zone Control -- COMPLETE (pending UI sign-off)**

All engine features implemented and tested. UI has a blocking IPC issue preventing play past ply 30.

---

## What Was Completed This Session

1. **Verified engine behavior** — Confirmed zone features change bestmove at depth 4 (h2h3 vs j1k3). Required finding and fixing stale binary issue (.cargo/config.toml had redirected target-dir to C:/rust-target/freyja/, leaving old binary in target/)

2. **Swarm model implemented** — Mutual defense (+4cp/defended, -6cp/undefended), attack coordination (+1cp/coordinated square), pawn chain (+5cp/chain pawn). Configurable via `SwarmWeight` setoption (default 3).

3. **Duel runner built** — `observer/duel_runner.mjs`: head-to-head per-color testing with all 3 seating arrangements (RY|BG, RB|YG, RG|BY). Unique NoiseSeed per game for diversity.

4. **Duel results: swarm+ray beats ray-only 9/15 (60%)** — Decision: swarm replaces BFS Voronoi.

5. **Zero-centered eval scores** — Subtracts mean across active players. Fixes MCTS exploration balance, beam discrimination, future NNUE training. Scores now in normal centipawn range.

6. **UI player tracking fixed** — `playerWhenGoSentRef` captures moving player before nextturn can corrupt it. `advancePlayer` used for chaining. Nextturn suppressed during go commands. Move labels now display correctly (Red, Blue, Yellow, Green order).

7. **Bestmove watchdog** — 30-second timer recovers from stuck `awaitingBestmoveRef`.

8. **Diagnosed UI ply-30 hang** — Root cause: **Tauri `invoke('send_command')` hangs when position command has 30+ moves**. The engine works fine via CLI at all ply counts. The Tauri IPC layer (mutex-locked stdin write → event emission) breaks under longer position strings. Needs observer-based architecture (bypass Tauri IPC) like Project Odin.

---

## What Was NOT Completed

- **UI play past ply 30** — Tauri IPC blocks. Needs architecture change (observer-based engine communication).
- **User UI sign-off for Stage 15** — Cannot test in UI due to the hang.
- **Stage 15 audit log + downstream log** — Not written.
- **PW k=2 vs k=4 A/B test** — Config exists (`config_ab_pw.json`), not run.
- **Performance benchmark** — Zone features not formally benchmarked. NPS ~25% slower (12k vs 16k).

---

## What the Next Session Should Do First

1. **Fix the Tauri IPC hang** — Two approaches:
   - **Quick fix:** Use FEN4 instead of replaying full move list. The engine supports `position fen4 <fen>` which is a constant-size command regardless of game length. The UI already gets FEN4 from `d` command.
   - **Architecture fix:** Port Odin's observer-based engine communication to Freyja. Bypass Tauri's invoke/event system. Spawn engine directly, communicate via raw stdin/stdout pipes from the Rust backend, forward to frontend via a simpler channel.
2. After UI fix: get user sign-off on Stage 15
3. Tag `stage-15-complete` / `v1.15`
4. Write audit_log_stage_15.md and downstream_log_stage_15.md

---

## Open Issues

- **[[Issue-UI-Tauri-IPC-Hang]] (BLOCKING):** Tauri invoke hangs when position command has 30+ moves. Engine works fine. UI architecture needs change.
- **[[Issue-UI-Feature-Gaps]] (WARNING):** Still open, not blocking.
- **MoveNoise in MCTS:** Still unresolved. Hybrid mode provides diversity for testing.

---

## Key Diagnosis: UI Ply-30 Hang

**Symptoms:** Game plays 30 plies fine, then freezes. No bestmove received. Watchdog fires after 30s.

**Console log at failure point:**
```
timeout: calling sendGoFromRef, awaiting=false
[sendGo: sending position (30 moves)]  ← THIS LINE NEVER APPEARS
WATCHDOG: No bestmove received in 30s, recovering
```

**Root cause:** `engineSendCommand(posCmd)` (which calls `invoke('send_command', { cmd })`) never resolves. The Tauri invoke acquires a mutex, writes to engine stdin, and flushes. With 30+ moves in the position command, the command string is ~200+ characters. The invoke hangs — possibly due to stdin pipe buffer contention, mutex deadlock with the stdout reader thread, or Tauri IPC serialization overhead.

**Proof engine works:** `printf "position startpos moves <30 moves>\ngo\n" | target/release/freyja.exe` returns bestmove in ~3 seconds. The engine is not the problem.

**Recommended fix:** Replace `position startpos moves ...` with `position fen4 <fen>` for all mid-game commands. FEN4 is constant-size (~100 chars) regardless of game length. The engine already supports this. The UI already retrieves FEN4 via the `d` command but doesn't use it for position setting.

---

## Design Decisions Made This Session

- **ADR-021 accepted:** Swarm replaces BFS Voronoi (duel: 9/15 wins)
- **Zero-centered eval:** Subtract mean across active players (research-backed)
- **Nextturn handling:** Suppress during go commands, use advancePlayer for chaining
- **playerWhenGoSentRef:** Capture moving player at send time, not receive time

---

## Deferred Debt

- Stage 5 post-audit, downstream log, vault notes
- Session notes for Sessions 4, 7, 8, 11, 12, 17, 18, 19, 20, 21
- Dead code: `apply_move_with_events` in `game_state.rs`
- MCTS warmup at phase cutover (carried from Stage 13)
- MCTS info output during thinking (carried from Stage 13)
- PW k=2 vs k=4 A/B test (config ready, not run)
- Stage 15 audit log + downstream log

---

## Files Modified This Session (Session 25)

| File | Changes |
|------|---------|
| `freyja-engine/src/eval.rs` | Swarm model, zero-centering, DKW fix |
| `freyja-engine/src/protocol/options.rs` | SwarmWeight setoption |
| `freyja-engine/src/protocol/mod.rs` | Wire swarm + zone weights |
| `freyja-ui/src/hooks/useGameState.ts` | Player tracking fix, watchdog, nextturn suppression, debug logging |
| `freyja-ui/src-tauri/src/engine.rs` | Emit error logging |
| `observer/duel_runner.mjs` | New + NoiseSeed fix |
| `observer/config_duel_*.json` | New duel configs |
| `observer/config_ab_*.json` | New A/B configs |
| `masterplan/DECISIONS.md` | ADR-019, 020, 021 |
| `masterplan/STATUS.md` | Updated |
| `masterplan/HANDOFF.md` | This file |
| `masterplan/sessions/Session-025.md` | New |
