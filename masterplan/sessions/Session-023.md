# Session 023 — Stage 15 Progressive Widening + Zone Control (Partial)

**Date:** 2026-03-20
**Stage:** 15 (in progress)
**Focus:** PW implementation, zone control research + ray-attenuation model, swarm design

---

## What Happened

### Planning Phase
- Read AGENT_CONDUCT.md, STATUS.md, HANDOFF.md, MASTERPLAN Stage 15 spec
- Explored MCTS code (mcts.rs), eval code (eval.rs), options (options.rs)
- Researched Baier & Kaisers 2020 paper on OMA+PW in multiplayer MCTS
- Researched BFS Voronoi in game AI (Tron, Go, RTS)
- Researched influence maps, king safety (Stockfish model), tension detection
- Found Stage 15 spec is incomplete (missing build order, tracing points)
- Found MASTERPLAN has duplicate "Stage 15" header (numbering error)

### Key Design Decisions
1. **PW at root-player nodes** (paper-faithful, not opponent nodes as spec suggests)
   - OMA handles opponents, PW handles root-player width
   - User approved this approach
2. **k=2 vs k=4 to be tested empirically** via A/B
3. **Replaced distance-decay influence with ray-attenuation model**
   - User identified fundamental flaw: distance-decay is circular, pieces project along vectors
   - Ray-attenuation walks actual piece movement rays with blocker degradation
   - Friendly blockers: mild attenuation (1.5x)
   - Enemy blockers: strong attenuation (2.0 + piece value scaled)
4. **Swarm model planned** as next evolution (user's idea)
   - Replace BFS Voronoi territory with swarm-based group cohesion
   - Ray-attenuation = individual piece voice, swarm = collective chord
   - Will A/B test swarm vs ray-attenuation vs baseline

### Implementation
- Progressive Widening: setoptions, child sorting, metrics, diagnostics, 9 tests
- BFS territory: contested squares, frontier length, DKW handling
- Ray-attenuated influence: directional force projection for all piece types
- Tension/vulnerability: from overlapping influence
- King escape routes: safe adjacent squares
- Zone weights: configurable via setoption, wired through protocol
- 441 total tests pass (33 new)

### Issues Encountered
- Windows linker file-lock issue (OneDrive + stale test processes)
  - Workaround: alternate CARGO_TARGET_DIR
  - Root fix: .cargo/config.toml with target-dir off OneDrive
- DKW players not skipped by zone features (fixed with is_active_for_zones helper)
- BFS Voronoi mathematically flawed for chess influence (circular, not directional)
  - Fixed by pivoting to ray-attenuation model

---

## Open Questions for Next Session

1. **Does the engine actually play differently?** Zone weights may be too small to affect bestmove. Need seeded game comparison (zone on vs off) to verify.
2. **Swarm model design:** How to compute cluster cohesion, mutual defense, attack coordination efficiently within 5us budget?
3. **Performance:** Zone features not benchmarked yet. Ray-attenuation could be expensive (slider rays × all pieces).
4. **PW k=2 vs k=4:** Empirical test needed.

---

## Test Results

- 441 lib tests pass, 0 fail
- 1 integration test (1000 random playouts) passes
- 5 perft tests pass
- 25 protocol tests pass
