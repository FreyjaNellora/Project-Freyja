# Component: Ray-Attenuated Influence Maps

**Stage Introduced:** Stage 15
**Last Updated:** 2026-03-20
**Module:** `freyja_engine::eval`

---

## Purpose

Computes per-player, per-square influence using directional ray projection along each piece's actual movement vectors, with attenuation through blockers. Replaces the flawed distance-decay (circular) influence model. Each piece type projects influence correctly: rooks along ranks/files, bishops along diagonals, queens in all 8 directions, knights to L-squares, pawns to forward-diagonal attack squares, kings to adjacent squares.

## Public API

```rust
pub struct InfluenceInfo {
    pub total: [f32; PLAYERS],     // Sum of influence per player
    pub net: [f32; PLAYERS],       // Own - max(opponents)
    pub grid: [[f32; PLAYERS]; 196], // Per-square per-player influence
}

fn compute_influence(state: &GameState) -> InfluenceInfo
```

## Internal Design

**Slider pieces (Rook, Bishop, Queen):** Walk each movement ray from the piece. At each square, add current influence value. At blockers, attenuate:
- Friendly blocker: `influence /= 1.5` (mild — friendly pieces support the ray)
- Enemy blocker: `influence /= (2.0 + piece_weight * 0.3)` (strong — enemy resistance)
- Stop when influence < 0.1

**Non-slider pieces:** Project to specific attack squares without attenuation:
- Knight: 8 L-shaped squares (jumps over pieces)
- Pawn: 2 forward-diagonal capture squares (orientation-specific per player)
- King: 8 adjacent squares

**Net influence:** Per-player `total - max(opponent totals)`.

DKW and eliminated players are skipped via `is_active_for_zones()`.

## Performance Characteristics

- ~32 active pieces × average ~7 ray squares per piece = ~224 ray steps
- Each step: 1 array lookup + 1 conditional + 1 multiply = ~3ns
- Estimated: <1us for ray computation, plus summary pass
- Not yet benchmarked — target <5us total with all zone features

## Known Limitations

- Friendly/enemy attenuation is heuristic (1.5x / 2.0+scale), not calibrated
- Does not account for pinned pieces (a pinned bishop still projects influence)
- Net influence uses total sums, not per-square comparison
- Planned replacement: swarm model will layer on top (ADR-021)

## Dependencies

- **Consumes:** `Board::piece_at()`, `Board::pieces()`, direction constants (ORTHOGONAL_DIRS, etc.)
- **Consumed By:** `BootstrapEvaluator::eval_4vec()`, `compute_tension()`, king safety zone ratio

---

**Related:** [[MASTERPLAN]], [[ADR-020]], [[ADR-021]], [[Component-BootstrapEvaluator]]
