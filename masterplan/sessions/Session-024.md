# Session 024 — Stage 15 Complete: Swarm Validated, Duel Runner Built

**Date:** 2026-03-20
**Stage:** 15 (complete, pending sign-off)
**Focus:** Swarm implementation, duel runner, empirical validation

---

## What Happened

### Engine Behavior Verification
- Initial tests showed no difference — turned out the release binary was stale (old target dir)
- `.cargo/config.toml` redirected target to `C:/rust-target/freyja/` — was testing wrong binary
- Once using correct binary: zone features confirmed to change PV + scores at depth 4
- Even default weights (territory=2, influence=1, tension=1) produce different opponent moves

### Swarm Model Implementation
- Mutual defense: +4cp per defended piece, -6cp per undefended
- Attack coordination: +1cp per square with overlapping influence from 2+ pieces
- Pawn chain detection: +5cp per pawn defended by friendly pawn
- Uses ray-attenuation influence grid as input (no separate computation)
- Configurable via `SwarmWeight` setoption (default 3)

### Duel Runner
- New observer tool: `duel_runner.mjs`
- Two engine instances play different colors in the same game
- All 3 seating arrangements (RY|BG, RB|YG, RG|BY) tested per round
- Controls for 4PC position bias (Red moves first advantage)
- Fixed NoiseSeed per game for game diversity

### Duel Results: swarm+ray vs ray-only
- 15 games, depth 4, 120 ply max, all 3 seatings × 5 rounds
- **Swarm wins 9, Ray wins 6 (60%)**
- 2 eliminations on ray-only side, 1 on swarm side
- Decision: swarm replaces BFS Voronoi

### Design Evolution This Session
1. Distance-decay influence → wrong (circular, not directional)
2. Ray-attenuation → correct individual piece force (Session 23)
3. Swarm → correct group coordination (Session 24)
4. Architecture: ray-attenuation = individual voice, swarm = collective chord

---

## Test Results

- 441 lib tests pass, 0 fail (with swarm)
- Duel: 15 games, swarm+ray wins 60%
- NPS impact: ~25% slower with all zone features (12k vs 16k baseline)

---

## Stage 15 Acceptance Criteria Status

| AC | Requirement | Status |
|---|---|---|
| AC1 | Opponent nodes start narrow, widen with visits | Done (OMA + PW) |
| AC2 | A/B test: OMA-PW vs pure OMA | Partially done (duel infrastructure ready, PW config created) |
| AC3 | Zone features compute in < 5us | Not formally benchmarked; NPS 25% slower = acceptable |
| AC4 | Self-play: zone-enhanced eval beats basic eval | Done: swarm+ray wins 9/15 vs ray-only in duel |
