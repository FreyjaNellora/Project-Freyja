# Eval Tuning Guidance for Claude.T

**Date:** 2026-03-14
**From:** Observer baseline testing (Claude.S session)
**Context:** First full run of the bootstrap eval tuning suite against Freyja release build (v1.0, Stage 9 TT+MoveOrdering, ~89.7k NPS depth 5)

---

## Test Results

**Score: 17/39 = 44% — FAIL** (threshold: 50% MARGINAL, 67% PASS)

13 of 25 samples were testable (12 still need game data for replay strings).
Search depth: 2 (depth 4 is too slow per-position for iterative tuning).

### Per-Sample Results

| Sample | Name | Turn | Engine | Human | Score | Notes |
|--------|------|------|--------|-------|-------|-------|
| S01 | Four-Queen Pileup | Red | j2g5 | f6c6 | +1 | Missed capture |
| S02 | Three-Way Exchange | Blue | a6g12 | d7g4 | +1 | Missed capture |
| S03 | Three-Round Explosion | Red | e1f3 | j4j9 | +2 | Category match |
| S04 | Knight Wins Rook | Red | f1m8 | d3e5 | +2 | Category match |
| S05 | Queen Hangs (neg) | Red | h2f4 | j4g4 | +3 | Correctly avoided blunder |
| S06 | Bishop Sacrifice | Green | m8i12 | m8h3 | +1 | Bishop retreats instead of sacrificing |
| S07 | Queen Invasion | Blue | b5e2 | k7f2 | +2 | Category match (piece move, not exact) |
| S10 | Castle R10 | Blue | a9e5 | a7a5 | +1 | Skipped castling |
| S17 | Gangster_H Opening | Red | i1e5 | g1j4 | +2 | Category match |
| S18 | Queen Overext (neg) | Red | g9m9 | g9m9 | -3 | **Played the bad move** |
| S21 | Deep Queen Raid | Red | i1g3 | i5m5 | +1 | Missed capture |
| S22 | Early Queen Act | Yellow | f14k9 | h14f12 | +2 | Category match |
| S23 | Rook Lift R8 | Yellow | f14h12 | d14d12 | +2 | Category match |

### Category Breakdown

| Category | Score | Pct | Verdict |
|----------|-------|-----|---------|
| queen_blunder (avoidance) | 3/3 | 100% | PASS |
| queen_reposition | 2/3 | 67% | PASS |
| knight_advance | 2/3 | 67% | PASS |
| queen_invasion | 2/3 | 67% | PASS |
| queen_activation | 4/6 | 67% | PASS |
| rook_activation | 2/3 | 67% | PASS |
| **capture** | **3/9** | **33%** | **FAIL** |
| **sacrifice** | **1/3** | **33%** | **FAIL** |
| **castling** | **1/3** | **33%** | **FAIL** |
| **overextension** | **-3/3** | **-100%** | **FAIL** |

---

## Diagnosis

### Problem 1: Captures not prioritized (S01, S02, S21)

The engine sees captures in the move list (MVV-LVA ordering is correct) but the **eval** prefers piece development/mobility moves over material gain. At depth 2, the engine doesn't search deep enough to see the capture's downstream value — it relies on the eval to signal "capturing is good."

**Root cause:** `WEIGHT_MATERIAL = 1` is the anchor, but material changes from captures only register as the raw piece value difference. Meanwhile `WEIGHT_DEVELOPMENT = 35` and `WEIGHT_MOBILITY = 4` can easily outweigh a minor piece capture.

**Suggested fix:** This is subtle. The material weight is an anchor (piece values are already in centipawns), so don't inflate it. Instead, consider:
- Verify that MVV-LVA captures are searched first (they are — `CAPTURE_BASE_SCORE = 100_000`)
- The real issue may be that at depth 2, the engine evaluates the position *after* a quiet move and sees high development/mobility, vs after a capture it sees slightly less development. The development bonus may need a decay for positions where pieces are already developed (diminishing returns).
- Alternatively: add a small "hanging piece" bonus in eval — if an opponent's piece is attacked and undefended, add a bonus even before the capture is made.

### Problem 2: Castling not valued enough (S10)

Blue plays `a9e5` (bishop move) instead of `a7a5` (O-O). The engine prefers an active piece move over castling.

**Root cause:** Castling has no direct eval bonus. It only gets credit indirectly through `king_safety_score()` shelter pawns. But the shelter bonus (35cp per pawn) may not outweigh the mobility/development gain from a bishop sortie.

**Suggested fix:** Add a **castling bonus** in eval. When a player has castled, give a flat bonus (e.g., 40-60cp). This is justified by the dataset: every winner castled, every checkmated player didn't. Alternatively, increase `WEIGHT_KING_SAFETY_SHELTER` from 35 to 50+ to make the shelter bonus from castling more compelling.

### Problem 3: Doesn't defend cracked king shelter (S18)

This is the worst result. S18 is a negative example where Red's king shelter is cracked (h-file open after Green's Bm8xh3). The human played `Qg9xm9+` (queen overextends to grab a pawn with check, abandoning defense). The engine plays the exact same bad move.

**Root cause:** The engine sees `g9m9` as a queen capture with check — very attractive tactically. It doesn't weigh the strategic cost of moving the queen far from a cracked king shelter. `WEIGHT_KING_SAFETY_ATTACKER = 35` penalizes attackers near the king, but doesn't penalize *leaving* the king undefended.

**Suggested fix:**
- Add a "king exposure" term: when shelter pawns are missing (especially on the file the king is on), apply a penalty that scales with the number of missing shelter pawns. Currently shelter is a positive bonus; also need a negative penalty for *missing* shelter.
- Consider a "queen distance to own king" factor — if king shelter is damaged and queen is far away, apply a penalty. This would make `Qg9→m9` (queen moves even further from cracked king) evaluate worse.
- This is the hardest fix and may require depth 4+ to fully resolve. At depth 2, tactical captures with check will always look attractive.

### Problem 4: Bishop sacrifice not seen (S06)

Engine plays `m8i12` (bishop retreats) instead of `m8h3` (bishop captures pawn, cracking Red's king shelter permanently). This is a strategic sacrifice that pays off 10 moves later.

**Reality check:** At depth 2, no engine will see a 10-move payoff. This sample tests whether the eval gives enough credit for capturing a shelter pawn. The fact that the engine retreats the bishop suggests the eval doesn't see "capturing a pawn adjacent to the enemy king" as particularly valuable.

**Suggested fix:** Consider a "pawn storm / shelter destruction" bonus in `king_safety_score()`. When evaluating a capture, if the captured pawn was part of an opponent's king shelter, give extra credit beyond the raw pawn value. This is a longer-term improvement.

---

## Priority Order

1. **Castling bonus** (easiest, clearest signal from data, fixes S10)
2. **King exposure penalty** (fixes S18, prevents overextension)
3. **Capture awareness** (hardest — may need a hanging-piece detector or development diminishing returns)
4. **Shelter destruction bonus** (S06, long-term, may resolve naturally with depth)

---

## How to Test

```bash
cd observer/baselines
node run_eval_suite.mjs <path-to-freyja-binary>
```

The suite loads FEN4 positions directly (no replay needed for testable samples). Depth 2 by default. Results include per-sample and per-category breakdowns.

To re-extract FEN4 after position changes:
```bash
node extract_fen4.mjs <path-to-freyja-binary>
```

**Target:** Score >= 38/75 (50%) for MARGINAL, >= 50/75 (67%) for PASS.
Current testable subset: >= 20/39 for MARGINAL, >= 26/39 for PASS.

---

## Current Weights (for reference)

```rust
const WEIGHT_MATERIAL: i16 = 1;              // Anchor
const WEIGHT_PST: i16 = 1;                   // Tiebreaker
const WEIGHT_MOBILITY: i16 = 4;              // Per-move bonus
const WEIGHT_TERRITORY: i16 = 0;             // DISABLED
const WEIGHT_KING_SAFETY_SHELTER: i16 = 35;  // Per shelter pawn
const WEIGHT_KING_SAFETY_ATTACKER: i16 = 35; // Per attacker penalty
const WEIGHT_PAWN_ADVANCE: i16 = 0;          // DISABLED
const WEIGHT_PAWN_DOUBLED: i16 = 8;          // Per doubled pawn
const WEIGHT_DEVELOPMENT: i16 = 35;          // Per dev unit
```

---

## What's Working

The engine is solid on:
- **Piece activation** — development, queen activation, rook lifts all passing (67%)
- **Blunder avoidance** — correctly avoids hanging the queen (100%)
- **Move variety** — doesn't spam pawns, plays real piece moves

The foundation is sound. The gaps are in tactical awareness (captures, castling, king safety response). These are weight-tunable without architectural changes.
