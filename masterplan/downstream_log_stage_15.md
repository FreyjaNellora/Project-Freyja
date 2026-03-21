# Downstream Log — Stage 15: Progressive Widening + Zone Control

**Author:** Agent
**Date:** 2026-03-21
**Stage:** 15 (Progressive Widening + Zone Control)

---

## Must-Know

1. **Swarm model replaced BFS Voronoi** (ADR-021). Zone control uses ray-attenuation influence maps + swarm mechanics (mutual defense, attack coordination, pawn chains). BFS Voronoi code removed.

2. **Eval scores are zero-centered.** After computing all 4 player scores, the mean is subtracted so scores sum to ~0. This prevents MCTS exploration imbalance, improves beam discrimination, and produces better NNUE training data. Scores are now in normal centipawn range (-300 to +300 typical).

3. **Progressive widening at opponent nodes** controls MCTS exploration breadth: `max_children(visits) = floor(k * visits^exponent)`. Default k=2, exponent=0.5. Configurable via setoptions.

4. **UI uses FEN4-based position commands.** `position fen4 <fen>` replaces `position startpos moves <move_list>` for all mid-game commands. This is constant-size regardless of game length, preventing Tauri IPC issues.

5. **Stderr must be drained.** The engine writes tracing output to stderr. The Tauri backend must spawn a thread to drain stderr, or the OS pipe buffer fills up (~64KB on Windows) and the engine process deadlocks.

6. **Watchdog timeout is 10 minutes.** The UI watchdog recovers from stuck bestmove-await states. Set to 10 minutes to accommodate deep searches (depth 7+ takes 7.5+ minutes).

---

## API Contracts

### New Setoptions (Stage 15)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `PWExponent` | f64 | 0.5 | Progressive widening exponent |
| `PWK` | f64 | 2.0 | Progressive widening multiplier |
| `SwarmWeight` | i32 | 3 | Swarm eval weight (mutual defense, coordination, chains) |
| `ZoneWeight` | i32 | 1 | Zone control eval weight (ray attenuation, king safety) |

### Eval Output

Scores are zero-centered (sum to ~0 across active players). Individual player scores are relative, not absolute. A score of +150 means "150cp better than average across active players."

### UI-Engine Communication

The UI sends `position fen4 <fen>` followed by `go depth N`. The FEN4 string is obtained from the engine's `d` command response (the `fen4` line). The first move of the game uses `position startpos` since no FEN4 exists yet.

---

## Known Limitations

1. **Hand-tuned eval misses tactical patterns.** The engine makes suboptimal moves — misses obvious defensive positions and attack opportunities. This is the ceiling of hand-tuned eval. NNUE (Stages 16-17) is the architectural fix.

2. **Depth 8+ is impractical without NNUE.** Depth 7 takes ~7.5 minutes, depth 8 estimated at 30+ minutes per move. NNUE-guided tighter beam will enable deeper practical search.

3. **PW k=2 vs k=4 not formally A/B tested.** Config exists. Can be run, but unlikely to show significant difference at current eval quality.

4. **NPS ~25% slower than pre-zone baseline.** 12k NPS vs 16k at depth 4. Expected cost of richer evaluation.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| NPS (depth 4, release) | ~12k | With zone features, down from ~16k |
| Depth 4 total nodes | ~9-14k | Varies by position |
| Swarm vs ray-only duel | 9/15 (60%) | All seating arrangements |
| Unit tests | 441 pass | 33 new in Stage 15 |

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_15]], [[Session-025]], [[Session-026]]
