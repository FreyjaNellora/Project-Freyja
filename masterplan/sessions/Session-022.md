# Session 022 — Stage 14 Post-Audit, Documentation, Stress Testing

**Date:** 2026-03-20
**Stage:** 14 (MCTS Opponent Move Abstraction) — COMPLETE (pending user sign-off)
**Focus:** Post-audit bug fixes, stress testing, documentation

---

## Summary

Completed Stage 14 post-audit work. Fixed a critical sigma transform saturation bug that had disabled Gumbel exploration since Stage 10, fixed observer player label rendering, added ply bounds safety guards to all search functions, and wrote extensive stress tests for edge cases (EP near cutouts, MCTS handoff boundaries, qsearch with eliminated players). A/B tested OMA on vs off — neutral result as expected pre-Progressive Widening.

## Key Achievements

### 1. Sigma Transform Fix (Critical — Stage 10 Bug)

The sigma function `sigma(x) = x / (1 + |x|)` compressed 4PC centipawn values (+/-2000) into +/-0.999, saturating the sigmoid. Sequential Halving's Gumbel noise (+/-2 to +5) and log-prior scores (+/-3 to 0) were completely overwhelmed. The engine was running pure exploitation since Stage 10 — no MCTS exploration at all.

Fixed with `sigma(x) = x / (c_scale + |x|)` where `c_scale = 200.0`. Values around +/-200cp now map to +/-0.5, preserving the gradient that Gumbel noise needs to influence selection.

### 2. OMA Implementation (Session 21, documented here)

- `SimStep` enum for mixed tree/OMA path tracking during MCTS simulation
- `OmaPolicy::select_move()` — lightweight opponent policy: checkmate > captures (MVV-LVA) > checks > history heuristic > random
- OMA moves stored at tree nodes (`oma_moves: ArrayVec<Move, 3>`, `oma_computed: bool`)
  - First visit: compute via policy, store at node
  - Revisit: replay stored moves for board state consistency
- `use_oma` flag in MctsConfig, `OpponentAbstraction` setoption
- Diagnostic metrics: oma_moves_total, root_decisions_total

### 3. Observer Bug Fixes

- Player label off-by-one in ab_runner.mjs and observer.mjs — double advancement of current_player (nextturn event + manual rotation). Fixed to single advancement.
- Discovered MoveNoise doesn't apply to MCTS phase (only Max^n iterative deepening).

### 4. Ply Bounds Guard

Added `ply >= MAX_DEPTH - 1` guard in qsearch, qsearch_2p, maxn, and negamax. When eliminated-player skip doesn't increment ply, deep recursion could overflow `pv_length[MAX_DEPTH]`. Safety net returns static eval at boundary.

### 5. Exhaustive EP Near-Cutout Testing

- 9 EP near-cutout tests for 4 corners (capturer and pusher combinations at board edges)
- 1 exhaustive test covering 64 positions (8 pairs x 8 edge positions)
- Validates that en passant works correctly near the 3x3 corner cutouts unique to 4PC

### 6. MCTS Handoff and Qsearch Stress Tests

- 2 MCTS handoff stress tests: ply 32 boundary (hybrid phase transition), consecutive searches (tree reuse)
- 3 qsearch elimination stress tests: 2 eliminated, 3 eliminated, midgame elimination

### 7. A/B Test Results

OMA on vs off: Elo -4.8 (not significant, p=0.993). 10 games per config, 2s movetime, hybrid mode. Winner distribution: Red 6, Blue 4, Yellow 9, Green 1. Zero crashes.

Expected neutral result at this stage. The Baier & Kaisers 2020 paper shows OMA benefit requires Progressive Widening (Stage 15) and longer time controls.

## Test Count

408 tests passing (24 new this stage):
- 9 OMA unit tests
- 10 EP near-cutout tests
- 2 MCTS handoff stress tests
- 3 qsearch elimination stress tests

## Commits

1. `9979720` — OMA: opponent move abstraction with stored moves per tree node
2. `61d05bf` — Fix sigma transform saturation + observer player label bug
3. `0e287d2` — Stage 14 post-audit, documentation, session note

## Decisions Made

- **c_scale=200.0 for sigma transform** — Tuned so that typical eval differences (~200cp) map to ~0.5 on the sigmoid, leaving room for Gumbel noise to influence selection. Will need recalibration when NNUE changes the centipawn range.
- **Stored OMA moves over deterministic RNG** — Zobrist-seeded RNG caused board state corruption after ~25 simulations. Stored moves guarantee consistency at ~13 bytes/node.
- **Include checkmate detection in OMA policy** — Not in original spec, but checkmate in 4PC eliminates a player (game-defining). OMA opponents that miss forced mates produce garbage simulations.

## What's Next

1. User UI testing for Stage 14 sign-off
2. Tag `stage-14-complete` / `v1.14` if approved
3. Stage 15: Progressive Widening + Zone Control
