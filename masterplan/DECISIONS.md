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

## ADR-003: Freyja as Training Data Generator for Odin

**Date:** 2026-02-25
**Status:** Accepted
**Stage:** Pre-implementation (architecture design)

**Context:**
Project Odin needs high-quality NNUE training data. Odin's own self-play data is generated via BRS/Paranoid search, which introduces evaluation bias. The NNUE trained on biased data perpetuates and amplifies that bias.

**Decision:**
Freyja serves a dual purpose:
1. Play four-player chess using the most accurate search available (Max^n)
2. Generate training data for Odin's NNUE using truthful multi-player evaluations

The training pipeline: Freyja self-play → export positions + 4-vector evaluations + zone control features → Odin NNUE training pipeline consumes the data.

**Rationale:**
- Max^n evaluations are the "ground truth" for multi-player positions
- Odin's BRS/Paranoid can be fast and approximate if it's approximating accurate targets
- Two engines covering both philosophies is strategically valuable
- The user explicitly wants to own both approaches

**Consequences:**
- Freyja must export evaluation data in a format Odin can consume (Stage 12 deliverable)
- Self-play infrastructure must support data export, not just win/loss tracking
- Freyja's NNUE and Odin's NNUE may diverge — this is expected (different training sources)

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

*New ADRs should be added below this line, following the same format.*
