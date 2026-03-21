# Project Freyja — DECISIONS

**Last Updated:** 2026-02-25

This document records all architectural decisions (ADRs) for the project. Each decision is permanent unless explicitly superseded by a new ADR that references the old one.

---

## ADR-001: Pure Max^n Search (No BRS, No Paranoid)

**Date:** 2026-02-25
**Status:** Accepted
**Stage:** Pre-implementation (architecture design)

**Context:**
Project Odin uses a BRS/Paranoid hybrid for tactical search. This is fast but introduces evaluation error — BRS assumes opponents cooperate, Paranoid assumes worst-case. Neither accurately models multi-player dynamics.

We considered several architectures:
1. Max^n → BRS/Paranoid filter → MCTS (three-stage pipeline)
2. Max^n for early game → BRS for mid-game (phase switch)
3. Max^n with paranoia dial (0.0-1.0 blend, like Athena)
4. **Pure Max^n with NNUE-guided beam search** ← chosen

**Decision:**
Use pure Max^n with no BRS or Paranoid components. The NNUE-guided beam search provides the tree reduction that traditionally required algorithmic shortcuts. As the NNUE improves, the beam narrows, and Max^n searches deeper — a virtuous cycle.

**Rationale:**
- Max^n is the only search algorithm that correctly models multi-player dynamics (each player maximizes their own score)
- BRS and Paranoid are useful lies — fast but inaccurate. Freyja prioritizes truth.
- The user already has Odin for the BRS/Paranoid approach. Freyja covers the other philosophy.
- Beam search solves the branching factor problem that traditionally makes Max^n infeasible at depth

**Consequences:**
- Cannot use alpha-beta pruning (fundamentally incompatible with Max^n except via paranoid assumption)
- Only proven pruning available: shallow pruning (Korf 1991)
- Depth limited by beam width, which is limited by NNUE quality
- Early stages (bootstrap eval, dumb NNUE) will search shallower than Odin

---

## ADR-002: NNUE-Guided Beam Search

**Date:** 2026-02-25
**Status:** Accepted
**Stage:** Pre-implementation (architecture design)

**Context:**
Pure Max^n at depth 8 with branching factor 30 = ~656 billion nodes. Infeasible. Need tree reduction.

We considered:
1. Speculative pruning (user rejected — "keep pruning to what is proven for Max^n")
2. Paranoia dial to enable alpha-beta (user rejected — wants pure Max^n)
3. **NNUE-guided beam search** ← chosen

**Decision:**
At each node, NNUE ranks all legal moves. Only the top K are expanded. Beam width adapts to NNUE maturity:

| NNUE Maturity | Beam Width | Effective Depth @ 7M nodes |
|---------------|-----------|---------------------------|
| Bootstrap (dumb) | 12-15 | 5-6 |
| Early NNUE | 8-10 | 6-7 |
| Mature NNUE | 5-8 | 7-8 |

**Rationale:**
- Beam width of 8 at depth 8 = 8^8 = ~16M nodes (feasible within 7-8M budget with iterative deepening)
- The engine literally gets deeper as it gets smarter
- No correctness compromise — beam search is a move ordering optimization, not a pruning assumption
- The NNUE IS the pruning strategy through better move ranking

**Consequences:**
- Beam width is a critical tunable parameter (Stage 13)
- Dumb NNUE = wide beam = shallow search (acceptable for early stages)
- Must verify: beam width 30 (all moves) = same result as no beam (correctness invariant)
- Beam width = 0 is an edge case that must be handled (minimum beam = 1)

---

## ADR-003: Independent Training Pipelines (Supersedes original ADR-003)

**Date:** 2026-03-02
**Status:** Accepted (supersedes "Freyja as Training Data Generator for Odin")
**Stage:** Pre-implementation (architecture design)

**Context:**
The original ADR-003 framed Freyja as a training data generator for Odin's NNUE. After further consideration, this coupling is undesirable — each engine should train independently on its own self-play data, then compete against each other when both are complete.

**Decision:**
Freyja and Odin maintain completely independent training pipelines:
1. Freyja trains its own NNUE via Max^n self-play data
2. Odin trains its own NNUE via BRS/Paranoid self-play data
3. When both engines are complete, they compete head-to-head to pressure-test both approaches

No training data is shared between engines.

**Rationale:**
- Independent training avoids coupling the engines' development timelines
- Each engine's NNUE learns patterns natural to its own search algorithm
- Head-to-head competition is a stronger validation signal than shared training data
- Two engines covering both philosophies is strategically valuable

**Consequences:**
- Freyja's self-play infrastructure serves Freyja only — no cross-engine export format needed
- Each engine's NNUE evolves independently, reflecting its search algorithm's perspective
- Competition infrastructure needed when both engines are ready

---

## ADR-004: Fixed-Size Data Structures from Day One

**Date:** 2026-02-25
**Status:** Accepted
**Stage:** Pre-implementation (architecture design)

**Context:**
Project Odin had a "clone cost timebomb" — `Vec<T>` in `GameState` was cheap to clone initially, but as the game history grew, clone cost grew linearly. This wasn't detected until Stage 8+ because early stages had short games. Fixing it required touching almost every function.

**Decision:**
No `Vec<T>` in any hot-path struct. Period. Board, GameState, MoveUndo, and any struct that is cloned or copied during search must use fixed-size arrays.

```rust
// YES:
position_history: [u64; MAX_GAME_LENGTH],  // Fixed-size
piece_lists: [[Option<(PieceType, Square)>; 32]; 4],

// NO:
position_history: Vec<u64>,  // FORBIDDEN in hot paths
```

**Rationale:**
- Clone cost is O(1) for fixed-size, O(n) for Vec
- Search clones GameState thousands of times per second
- Detecting the regression requires long games, which don't happen until late stages
- The fix is cheap now (use arrays) and expensive later (change every function signature)

**Consequences:**
- `MAX_GAME_LENGTH` must be chosen carefully (1024 should be sufficient)
- Fixed-size arrays waste memory for short games (acceptable trade-off)
- Piece lists use `Option<T>` with capacity 32 instead of growing Vec

---

## ADR-005: Coordinate System — `rank * 14 + file` (No Padding)

**Date:** 2026-02-25
**Status:** Accepted
**Stage:** Pre-implementation (architecture design)

**Context:**
Athena uses `rank * 16 + file` (padded board, like 0x88 in standard chess). This wastes memory but simplifies boundary detection. Standard approaches for 4-player chess vary.

**Decision:**
Use `rank * 14 + file` with explicit validity checking. Internal 0-indexed (rank 0-13, file 0-13). Display 1-indexed (rank 1-14, file a-n).

**Rationale:**
- 14×14 = 196 squares. No wasted entries.
- Validity checking is cheap (a few comparisons per square, easily branchless)
- Padding saves one comparison per boundary check but wastes 44 entries (16×16 - 14×14 = 60 entries of overhead)
- Athena's coordinate system is not directly compatible — copying constants from Athena would be a source of bugs
- Simpler mental model: square index directly maps to (rank, file) via division and modulo

**Consequences:**
- Cannot use Athena's coordinate constants or lookup tables directly
- Must validate every square access (rank < 14 && file < 14 && not in corner)
- Zobrist table size: 196 entries (most compact)
- FEN4 conversion must map between display (a-n, 1-14) and internal (0-13, 0-13)

---

## ADR-006: Gumbel MCTS over UCB1

**Date:** 2026-02-26
**Status:** Accepted
**Stage:** Stage 10 (MCTS), Stage 11 (Integration), Stage 13 (Tuning)

**Context:**
Freyja's MCTS (Stage 10) was originally specified with UCB1 selection. UCB1 optimizes cumulative regret (overall exploration quality), but the engine only plays one move — simple regret (quality of the chosen move) matters more. UCB1 needs 16+ simulations to reliably converge; Gumbel MCTS works with as few as 2. In the Max^n → MCTS hybrid, MCTS gets a limited simulation budget (Phase 2 residual time), making this critical.

**Decision:**
Replace UCB1 at MCTS root with Gumbel-Top-k sampling + Sequential Halving. Non-root nodes use an improved policy formula with prior and progressive history terms.

- **Root selection:** Sample `g(a) ~ Gumbel(0,1)` per move, compute `g(a) + log(pi(a))` to select Top-k candidates (default k=16). Sequential Halving progressively eliminates candidates by comparing `sigma(g(a) + log(pi(a)) - Q(a))`.
- **Prior policy (pre-NNUE):** `pi(a) = softmax(ordering_score(a) / T)` where ordering scores come from Max^n's move ordering pipeline (TT hint, MVV-LVA, killers, history). Temperature T=50.
- **Non-root policy:** `Q(node)[player] / N(node) + C_PRIOR * pi(a) / (1 + N(node)) + PH(a)` where PH(a) is the progressive history term.

**Rationale:**
- Gumbel-Top-k provides provable policy improvement even with 2 simulations
- Sequential Halving concentrates budget on the best candidates
- Gumbel noise provides principled exploration without UCB1's over-exploration at low counts
- The weak ordering-score prior is compensated by Gumbel exploration and Q-value correction

**Consequences:**
- MctsNode struct needs `gumbel: f32` field (root nodes only)
- Need softmax computation over ordering scores for prior policy
- Sequential Halving adds implementation complexity at root
- New tunable parameters: GUMBEL_K (default 16), PRIOR_TEMPERATURE (default 50)

---

## ADR-007: Progressive History — Max^n History Table Shared with MCTS

**Date:** 2026-02-26
**Status:** Accepted
**Stage:** Stage 10 (MCTS), Stage 11 (Integration)

**Context:**
In the Max^n → MCTS hybrid, MCTS Phase 2 starts cold — it has no knowledge of which moves are promising beyond what the prior policy provides. Meanwhile, Max^n Phase 1 has already computed a history heuristic table tracking which moves produced cutoffs. This knowledge is thrown away.

**Decision:**
Share Max^n's history heuristic table with MCTS via Progressive History: `PH(a) = PROGRESSIVE_HISTORY_WEIGHT * H(a) / (N(a) + 1)` where H(a) is the history score and N(a) is MCTS visit count.

- The `1/(N(a)+1)` decay is key: history dominates when MCTS visits are low (warm start), fades as MCTS accumulates its own data
- History table extracted from Max^n after Phase 1, passed to MctsSearcher via `set_history_table()`
- History is NOT persistent across moves — extracted per search, consumed, discarded

**Rationale:**
- Free warm-start for MCTS using knowledge Max^n already computed
- Decay formula ensures MCTS is not permanently biased by Max^n's tactical perspective
- No API signature changes to `Searcher` trait — `set_history_table()` is implementation-specific
- Zero additional computation cost (history table already exists)

**Consequences:**
- Max^n searcher must expose history table via accessor method
- Hybrid controller extracts history after Phase 1, injects before Phase 2
- New tunable parameter: PROGRESSIVE_HISTORY_WEIGHT (default 1.0)
- Full inter-move persistence deferred to Stage 13 measurement

---

## ADR-008: Observer Protocol for Gameplay Data Collection

**Date:** 2026-02-27
**Status:** Accepted
**Stage:** Stage 4 (Protocol), Stage 12 (Self-Play)

**Context:**
Freyja's original self-play specification (Stage 12) described a vague "training data export format (position FEN4 + eval 4-vector + game result)" with format TBD. This was disconnected from any concrete infrastructure. Meanwhile, Odin v1 proved that a protocol-based observer pipeline (LogFile toggle + game runner + structured JSON) provides both diagnostic visibility AND training data extraction in one system.

**Decision:**
Adopt the observer protocol approach proven in Odin v1:
1. **Stage 4:** Engine protocol includes `setoption name LogFile value <path>` for per-line I/O logging, and `setoption name MaxRounds value <n>` for diagnostic auto-stop. Both are zero-cost when disabled.
2. **Stage 12:** Self-play runner uses the observer protocol (single engine instance, 4 seats via protocol). Captures structured game JSON with per-move FEN4, eval 4-vectors, depth, component breakdown.
3. **Training data is a VIEW of observer data**, not a separate export format. A filter extracts FEN4 + eval 4-vec + game result from the same game JSON that the observer produces.

**Rationale:**
- One system serves three purposes: diagnostics, behavioral testing, and training data generation
- Protocol logging is zero-cost when off, so it adds no overhead to normal play
- Structured game JSON already contains everything needed for NNUE training
- Behavioral metrics (pawn ratio, queen activation, captures) provide early-warning for eval/search regressions
- Proven in production: Odin v1's observer pipeline successfully captured 6 human baseline games and engine diagnostic games

**Consequences:**
- Stage 4 build order gains 2 items (LogFile toggle, MaxRounds)
- Stage 12 self-play runner builds on observer infrastructure instead of starting from scratch
- Stage 16 NNUE training pipeline reads observer game JSON, not a custom format
- Observer pipeline scaffolded in project structure from Stage 0

---

## ADR-009: EP Board Scan (Not player.prev())

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 2-3 (Move Generation, Game State)
**Carried from:** Odin v1 Stage 19 stress test finding

**Decision:** En passant captured pawn location is determined by scanning the board for the actual pawn, not by using `player.prev()` to infer the pusher.

**Rationale:** In 4-player chess, `player.prev()` can return an eliminated player. Odin v1 crashed at ~10% rate in games with eliminations + EP. The `find_ep_captured_pawn_sq()` board scan pattern is immune to elimination state.

**Consequences:** Slightly more expensive EP handling (board scan vs arithmetic). Zero crash risk.

---

## ADR-010: ArrayVec Movegen from Stage 2

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 2 (Move Generation)
**Carried from:** Odin v1 Stage 19 optimization

**Decision:** Move generation uses `generate_legal_into(&mut ArrayVec<Move, 256>)` via a `MoveBuffer` trait. No `Vec<Move>` return in hot paths.

**Rationale:** Odin v1 allocated a new `Vec<Move>` per search node. The retrofit to ArrayVec contributed to a 2.46x BRS depth-6 speedup. Max^n with beam search still generates moves per node — same heap pressure applies.

**Consequences:** `arrayvec` dependency from Stage 2.

---

## ADR-011: Release Profile from Stage 0

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 0 (Project Skeleton)
**Carried from:** Odin v1 Stage 19

**Decision:** Workspace `Cargo.toml` includes `[profile.release]` with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1` from Stage 0.

**Rationale:** Free 10-20% release performance. Odin v1 didn't add these until Stage 19.

---

## ADR-012: SIMD-Ready NNUE Architecture

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 14 (NNUE)
**Carried from:** Odin v1 Stage 19 SIMD retrofit

**Decision:** NNUE architecture includes: `#[repr(C, align(32))]` on accumulator arrays, hidden weight transpose at load time, runtime AVX2 detection via `OnceLock<bool>`, scalar fallback for all SIMD operations.

**Rationale:** Odin v1 retrofitted AVX2 at Stage 19, achieving 40.8x forward_pass speedup. Designing for SIMD from the start avoids touching accumulator, forward pass, and weight loading simultaneously in a late-stage retrofit.

---

## ADR-013: Bitboards — Skip

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** N/A (architecture decision)
**Carried from:** Odin v1 Stage 19 profiling

**Decision:** No bitboard representation. The 14x14 board uses array representation with the attack query API abstraction (ADR-005 coordinate system).

**Rationale:** 14x14 boards require u256 or custom wide-int types for bitboards. Odin v1 profiled this at Stage 19 after SIMD + memory optimization and found board scanning is not the bottleneck. The attack query API provides a clean abstraction boundary without the complexity.

---

## ADR-014: Strategy Profiles for Training Data Diversity

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 8 (Max^n Hybrid), Stage 12 (Self-Play)
**Carried from:** Odin v1 post-engine planning

**Decision:** Opponent modeling supports multiple strategy profiles across two independent axes: target selection (Vulture — lowest material, Predator — lowest king safety, Assassin — closest to elimination) and play style (Fortress — defensive, Territorial — space control). Self-play data generation uses all profiles for diverse training positions.

**Rationale:** Strategy diversity in self-play training data is critical — the NNUE must learn to evaluate positions arising from many different play patterns. Diverse profiles create diverse board states that a single "Standard" profile would never reach.

**Consequences:** Self-play datagen runs games across all profile combinations. More diverse but higher quality training data.

---

## ADR-015: Stress Testing — Volume Over Depth

**Date:** 2026-03-02
**Status:** Accepted
**Stage:** Stage 20 (Optimization & Hardening)
**Carried from:** Odin v1 Stage 19

**Decision:** Stress testing uses high game volume at low search depth (10K games at depth 2) rather than few games at high depth.

**Rationale:** Odin v1 found that depth only affects how many nodes are explored, not which code paths are hit. The EP-after-elimination crash was a game state bug triggered by board conditions, not search depth. 10K shallow games cover far more unique board states than 500 deep ones. Each game is a different elimination/EP/castling/promotion sequence.

**Consequences:** Stress tests complete in hours instead of days. Higher coverage of edge cases.

---

## ADR-016: Snake_Case Module Naming Consistency

**Date:** 2026-03-04
**Status:** Accepted
**Stage:** Stage 2 (Move Generation)

**Context:**
Stage 0 created stub modules with inconsistent naming: `movegen` and `gamestate` are compound words without underscores, while Rust convention and MASTERPLAN Section 6 specify snake_case for modules (e.g., `move_gen`, `board_repr`). Single-word modules (`board`, `eval`, `mcts`, `protocol`, `search`) are already correct.

**Decision:**
Rename compound modules to snake_case:
- `movegen.rs` → `move_gen.rs`
- `gamestate.rs` → `game_state.rs`

All module names now follow Rust snake_case convention uniformly.

**Rationale:**
- Consistency: all modules follow the same naming convention
- MASTERPLAN Section 6 explicitly uses `move_gen` as an example
- Rust standard convention (clippy recommends snake_case for modules)
- Early rename is cheap; late rename would require updating all imports across many stages

**Consequences:**
- `mod movegen` → `mod move_gen` in lib.rs
- `mod gamestate` → `mod game_state` in lib.rs
- No downstream impact (both modules are stubs with no consumers yet)

---

## ADR-017: Phase-Separated Hybrid Controller (Supersedes original Stage 11 design)

**Date:** 2026-03-18
**Status:** Accepted (supersedes "Max^n → MCTS sequential pipeline on every move")
**Stage:** Stage 11 (corrected), Stage 13 (implementation)

**Context:**
The original Stage 11 design ran Max^n Phase 1 → knowledge transfer → MCTS Phase 2 on every single move with a 50/50 time split. In practice, this caused two problems:

1. **MCTS burned 5 seconds on opening moves** where Max^n depth 4 already had the right answer. MCTS is weak at openings — it samples positions it doesn't understand and can override Max^n's correct tactical answer.
2. **Max^n wasted time in the midgame** where it can't search deep enough to matter. Depth 4 is one 4-player cycle — not enough to see strategic patterns. The time spent on Max^n could go entirely to MCTS.

**Decision:**
Phase separation. The hybrid controller selects Max^n OR MCTS based on game phase — never both on the same move:

- **Opening (ply < phase_cutover_ply):** Max^n only, full time budget
- **Midgame+ (ply >= phase_cutover_ply):** MCTS only, full time budget
- Default cutover at ply 32 (8 moves per player = ~2 full opening cycles)
- Configurable via `setoption name PhaseCutoverPly value N`

**Rationale:**
- Each algorithm excels at different game phases. Running both wastes time.
- Max^n handles structured openings where tactical precision matters (captures, development, immediate threats).
- MCTS handles chaotic midgame where sampling handles multi-player branching better than exhaustive search.
- Opening moves now complete in <1s instead of ~6s.
- MCTS gets the FULL time budget in the midgame instead of sharing with Max^n.

**Consequences:**
- History transfer from Max^n to MCTS (ADR-007) is no longer used during normal play. The APIs (`set_history_table`, `set_prior_policy`) remain for potential future use but are not called in the phase-separated flow.
- `TimeSplitRatio` setoption is deprecated (kept for backward compatibility).
- MCTS must work from cold start in the midgame — uses its own lightweight move ordering (MVV-LVA, killer moves) for priors.
- Disagreement tracking between Max^n and MCTS is no longer applicable.

---

## ADR-018: MCTS Opponent Move Abstraction (OMA) for Stage 14

**Date:** 2026-03-18
**Status:** Accepted (replaces original "Zone Control Features" as Stage 14)
**Stage:** Stage 14

**Context:**
The original Stage 14 was "Zone Control Features" — territorial evaluation enhancements. While valuable, this was reprioritized after analyzing Freyja's identity: Phase 1 (Max^n) stays dumb and fast, Phase 2 (MCTS) is where strategic intelligence lives. The highest-impact improvement to MCTS is reducing opponent branching, not enhancing eval.

**Decision:**
Stage 14 implements Opponent Move Abstraction (OMA) per Baier & Kaisers (IEEE CoG 2020). During MCTS simulations, root player nodes expand fully while opponent nodes pick ONE move via a lightweight policy. This lets MCTS search 3-4x deeper into root player's decision space.

Zone Control features move to Stage 15 (combined with Progressive Widening).

**Rationale:**
- MCTS wastes simulation budget expanding all 4 players equally
- OMA focuses simulations on root player decisions, which is what matters
- Research shows MCTS-OMA outperforms Paranoid and BRS+ in multi-player games
- Zone Control is still valuable but less impactful than fixing the branching factor problem

**Consequences:**
- Stage 14 is now MCTS-focused, not eval-focused
- Stage 15 combines Progressive Widening (OMA enhancement) with Zone Control
- NNUE stages shift to 16-17

---

## ADR-019: Progressive Widening at Root-Player Nodes (Paper-Faithful)

**Date:** 2026-03-20
**Status:** Accepted
**Stage:** Stage 15

**Context:**
The Stage 15 spec says "Progressive widening at opponent nodes." However, research (Baier & Kaisers 2020) applies PW at root-player nodes, with OMA handling opponents entirely. PW at opponent nodes would fight OMA's design (fixed stored moves per node).

**Decision:**
PW applies at root-player tree nodes only. OMA stays fixed at opponent nodes. They are complementary: OMA shrinks tree depth (skipping opponents), PW shrinks tree width (limiting root-player children). Formula: `available_children = floor(k * visits^alpha)`, children sorted by prior descending so PW exposes best moves first.

**Rationale:**
- Paper-faithful: proven in literature
- Simpler: no OMA-to-tree transition complexity
- Children sorted by prior ensures PW window contains highest-quality moves
- Configurable via setoption (PWConstant, PWExponent) for A/B testing

**Consequences:**
- PW naturally applies only to root-player nodes (OMA intercepts opponents)
- k=2 vs k=4 to be empirically tested via A/B

---

## ADR-020: Ray-Attenuated Influence Over Distance-Decay

**Date:** 2026-03-20
**Status:** Accepted
**Stage:** Stage 15

**Context:**
Initial influence map implementation used exponential distance decay in a Chebyshev radius-3 circle around each piece. This is fundamentally wrong for chess: pieces project force along specific vectors (rays), not in circles. A rook controls a cross, a bishop an X, a pawn two forward diagonals — none of these are circles.

**Decision:**
Replace distance-decay with ray-attenuated influence. Each piece projects influence along its actual movement vectors. Influence attenuates through blockers: `influence /= blocker_resistance`, where resistance depends on whether the blocker is friendly (mild: 1.5x) or enemy (strong: 2.0 + piece_value_scaled). Non-slider pieces (knight, pawn, king) project to their specific attack squares without attenuation.

**Rationale:**
- Directionally correct for all piece types
- Obstacle-aware: blocked lines reduce influence
- Friendly/enemy blocker asymmetry captures real chess dynamics
- Builds on existing ray-tracing infrastructure in attacks.rs

**Consequences:**
- Influence maps are now per-ray, not per-circle
- Performance may be higher than distance-decay (more ray traversals)
- Tension/vulnerability scoring uses the same grid interface

---

## ADR-021: Swarm Model Replaces BFS Voronoi Territory

**Date:** 2026-03-20
**Status:** Accepted (implemented + duel-tested: swarm wins 9/15)
**Stage:** Stage 15

**Context:**
BFS Voronoi territory assigns squares to the nearest player by piece distance. This has known limitations: all pieces seed equally (pawn = queen), ignores obstacles, and the result is a geometric partition that doesn't capture how chess piece groups actually project collective influence. User proposed swarm mechanics as a more organic model.

**Decision:**
Implement a swarm-based zone control model as an alternative to BFS Voronoi. Swarm features: cluster cohesion (how tightly pieces are grouped), mutual defense density (pieces protecting each other), attack coordination (squares attacked by 2+ pieces), pawn chain backbone. Ray-attenuation (ADR-020) provides the individual force projection; swarm captures the collective reinforcement.

Both models will be A/B tested against each other and against a baseline (no zone features). The empirically stronger model wins.

**Rationale:**
- Chess IS about coordinated piece groups, not geometric territory
- Swarm captures emergent group strength from overlapping force vectors
- In 4PC, the 3-opponent dynamic makes group cohesion even more important
- A/B testing provides empirical answer rather than theoretical argument

**Consequences:**
- BFS Voronoi may be removed if swarm wins A/B decisively
- Or both may coexist as complementary NNUE features
- Implementation and testing deferred to next session

---

*New ADRs should be added below this line, following the same format.*
