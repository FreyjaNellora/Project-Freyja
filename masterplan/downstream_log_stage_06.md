# Downstream Log -- Stage 6: Bootstrap Evaluation

## Must-Know

1. **Evaluator trait is the eval boundary.** All search code (Stage 7+) must call `eval_scalar()` or `eval_4vec()` through the `Evaluator` trait. Never access eval internals directly.
2. **Eliminated players return `ELIMINATED_SCORE` (i16::MIN = -32768).** Search must handle this sentinel — do not compare it with normal scores.
3. **Bishop is 450cp** (user override from MASTERPLAN's 350cp). Half a pawn less than rook.

## API Contracts

### Evaluator Trait (`eval.rs`)

```rust
pub trait Evaluator {
    fn eval_scalar(&self, state: &GameState, player: Player) -> i16;
    fn eval_4vec(&self, state: &GameState) -> [i16; 4];
}
```

### BootstrapEvaluator (`eval.rs`)

```rust
pub struct BootstrapEvaluator;
impl BootstrapEvaluator {
    pub fn new() -> Self;
}
impl Evaluator for BootstrapEvaluator { ... }
impl Default for BootstrapEvaluator { ... }
```

### Constants (`eval.rs`)

```rust
pub const ELIMINATED_SCORE: i16 = i16::MIN;
pub fn piece_value(pt: PieceType) -> i16;
```

Piece values: Pawn=100, Knight=300, Bishop=450, Rook=500, Queen=900, King=0, PromotedQueen=900.

## Known Limitations

1. **Mobility is approximate.** Uses piece-type heuristic (Pawn=2, Knight=4, etc.) not actual legal move count, because `generate_legal_moves` requires `&mut Board` but eval receives `&GameState`.
2. **Weights are untuned.** Material=1x, PST=1x, Mobility=3cp/move, Territory=5cp/sq, King shelter=15cp, King attacker=-20cp, Pawn advance=5cp/rank, Doubled pawn=-15cp. Stage 13 tunes these via self-play.
3. **No lead penalty.** Intentionally omitted per Odin lesson — lead penalties make capturing non-monotonic.
4. **eval_4vec calls eval for all 4 players.** Cannot compute a single player's eval without computing territory for all 4. The BFS is shared.

## Performance Baselines

| Metric | Value | Build |
|--------|-------|-------|
| eval_4vec() | <100us | Debug |
| eval_4vec() | <50us | Release (well under) |
| BFS territory | ~160 squares processed | Both |

## Open Questions

1. Should beam search (Stage 7) use `eval_scalar` or `eval_4vec` for move ordering? `eval_4vec` is more efficient when evaluating the same position for all players, but move ordering only needs the current player's perspective.
2. Should mobility switch to actual pseudo-legal count when the search provides `&mut Board`? This would be more accurate but may impact performance.

## Reasoning

- **Relative material (not absolute):** Odin's original eval counted only own pieces — zero incentive to capture. Relative eval ensures capturing always improves your score.
- **Approximate mobility:** Full legal movegen requires `&mut Board` for make/unmake legality testing. Since the evaluator trait takes `&GameState`, we'd need to clone the board. Heuristic is faster and adequate for bootstrap.
- **BFS Voronoi for territory:** Simple, fast, cache-friendly. Seeds from all pieces, expands in 8 directions. Produces a reasonable territory approximation in ~320 operations.
- **PST rotation:** One canonical table per piece type (Red's perspective), rotated at lookup time. Avoids storing 4 copies of each table.
