# Freyja Bootstrap Eval Tuning Method

## Overview

Curated Texel Tuning adapted for 4-player chess. Uses tactical positions extracted
from 3000+ Elo human games to validate and tune eval weights.

**This document is authoritative for eval weight changes. Any agent modifying
`eval.rs` weights MUST run the test suite and report results.**

## The Method

### Step 1 — Position Set

25 tactical samples in `tactical_samples.json`, each a 5-6 move window from
real games. Each sample has:
- Move sequence to replay (Freyja notation: `position startpos moves ...`)
- The human move at the decision point
- Expected move category (capture, development, castling, queen_activation, etc.)
- Whether it's a positive or negative example
- What happened as a consequence

### Step 2 — Run the Suite

For each sample where `moves_to_replay` is provided:

```
position startpos moves <replay_moves>
go depth 4
```

Record:
- `bestmove` — what Freyja chose
- `score` — eval for all 4 players from the `info` line
- Match result (see scoring below)

### Step 3 — Score

| Result | Points | Criteria |
|--------|--------|----------|
| Exact match | +3 | Engine plays the same move as the human |
| Category match | +2 | Different move, same type (any development when human developed, any capture when human captured, any castle when human castled) |
| Reasonable | +1 | Different category but not harmful (e.g., development when human captured — still productive) |
| Neutral | 0 | No clear positive or negative |
| Anti-pattern | -2 | Engine plays a move from the negative examples (pawn spam, knight retreat, queen shuffle, king walk) |
| Blunder | -3 | Engine hangs material or misses a free capture |

For negative examples (is_negative: true), scoring is inverted:
- Engine AVOIDS the bad move: +3
- Engine plays the bad move: -3

### Step 4 — Thresholds

| Score | Verdict | Action |
|-------|---------|--------|
| >= 50 (67%) | PASS | Weights are good. Proceed with other work. |
| 38-49 (50-66%) | MARGINAL | Identify which categories fail. Adjust those specific weights. |
| < 38 (< 50%) | FAIL | Fundamental weight problem. Do not merge. |

Maximum possible score: 75 (25 samples x 3 points each)

### Step 5 — Tuning Loop

When a sample fails:

1. Identify the category (development, castling, king_safety, capture, etc.)
2. Determine which weight in `eval.rs` controls that behavior
3. Adjust the weight by 5-10 units
4. Re-run the full suite
5. Check that the fix doesn't regress other samples
6. Repeat until score >= 50

## Weight Map

Which weight affects which behavior:

| Behavior | Primary Weight | Secondary |
|----------|---------------|-----------|
| Pieces sitting on back rank | `WEIGHT_DEVELOPMENT` | `WEIGHT_MOBILITY` |
| Queen not activating | `WEIGHT_DEVELOPMENT` (queen dev unit = 2) | PST_QUEEN |
| Too many pawn moves | `WEIGHT_PAWN_ADVANCE` (should be 0) | `WEIGHT_DEVELOPMENT` |
| Not castling | `king_safety_score()` shelter logic | PST_KING |
| Missing captures | Search/move ordering issue, not eval | `move_order.rs` |
| Queen overextension | King safety vs tactical bonus trade-off | `WEIGHT_KING_SAFETY_*` |
| Not promoting passed pawns | Pawn PST + search depth | PST_PAWN high ranks |
| Ignoring king danger | `WEIGHT_KING_SAFETY_ATTACKER` | `WEIGHT_KING_SAFETY_SHELTER` |

## Current Weights (Baseline)

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

## Universal Patterns (From Game Data)

These patterns hold across ALL 12 games in the dataset. They are non-negotiable
requirements for the eval — if the engine violates any of these, weights are wrong.

### Every winner did:
- Queen active by round 5
- Castled (or had deliberate reason not to)
- 2+ captures in first 20 moves
- Pawn ratio <= 35% in opening

### Every checkmated player did:
- Never castled (100% correlation)
- Queen overextended OR dormant
- Pawn ratio > 40% OR front-loaded all pawns before pieces

### Elo calibration targets:

| Metric | 3000+ target | Engine must beat |
|--------|-------------|-----------------|
| Pawn ratio (first 20) | 20-30% | <= 35% |
| Queen activation | Round 2-5 | <= Round 7 |
| Captures in first 20 | 4-5 | >= 2 |
| Knight undevelopment | 0 | 0 |
| Castling | Yes, by R18 | Yes |

## Game Sources

### Strong Games (tune TOWARD these)
- 95992584: All 3000+ (3054-3434). Red/Yellow tied 73pts.
- 96085550: Mixed (2541-3438). Blue 87pts, checkmate win.
- 93419919: Avg 2931 (2598-3465). Red 79pts.
- 93391655: Avg 3114 (2805-3476). Blue 70pts. Gangster_H wins.
- 93334455: Avg 3165 (2907-3456). Yellow 72pts, Red 79pts.
- 93333795: Avg 3202 (2913-3443). Red 60pts. Gangster_H wins.
- 93060137: Avg 3045 (2902-3422). Yellow 71pts. Gangster_H loses.

### Weak Games (tune AWAY from these)
- 96585003: Weak players show pawn spam, knight undevelopment.
- 96836735: King march to checkmate, queen shuffle.
- 96602063: 54% pawn ratio, queen never activated, 3pts.

### Mixed Games
- 93042775: Red 2803 dominates with bishop pair.
- 93042729: Green (EyeoftheTiger, #1 leaderboard) wins with patient style.
