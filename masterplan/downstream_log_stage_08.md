# Downstream Log -- Stage 8: Quiescence Search

## Must-Know

1. **Quiescence runs at every leaf node.** Both `maxn()` and `negamax_2p()` call quiescence at depth 0 instead of static eval. Every leaf node is at minimum 1 qnode (stand-pat).
2. **Root-player captures only (4PC).** `qsearch()` only expands captures where the capturing or captured piece belongs to `root_player`. Opponent-vs-opponent captures are skipped.
3. **`MIN_SEARCH_DEPTH = 4` is enforced.** Time-based abort is suspended until depth 4 completes. This guarantees tactical quality.
4. **Default time budget is 5 seconds** (changed from 2s to accommodate quiescence overhead).
5. **`qnodes` is tracked separately** from main search nodes. Both are reported in info strings.

## API Contracts

### New Functions

```rust
// search.rs
pub const MAX_QSEARCH_DEPTH: u32 = 4;
pub const MIN_SEARCH_DEPTH: u32 = 4;

// move_gen.rs
pub fn generate_captures_only(board: &mut Board) -> ArrayVec<Move, MAX_MOVES>
```

### Changed Signatures

```rust
// maxn() now takes root_player
fn maxn(state: &mut GameState, depth: u32, ply: usize, root_player: Player, ss: &mut SearchState) -> Score4

// format_info() now takes qnodes
pub fn format_info(depth: u32, nodes: u64, qnodes: Option<u64>, nps: u64, ...) -> String
```

### SearchState Extensions

```rust
pub struct SearchState {
    // ... existing fields ...
    pub qnodes: u64,            // quiescence node count
    pub suspend_time_check: bool, // true during min depth guarantee
}
```

### SearchResult Extensions

```rust
pub struct SearchResult {
    // ... existing fields ...
    pub qnodes: u64,
}
```

## Known Limitations

1. **Quiescence overhead is significant.** At depth 4, qnodes can be 2.3x main nodes in developed positions. NPS drops from ~84k to ~33-60k.
2. **Root-player-only filter is conservative.** Misses opponent-vs-opponent captures including en passant. Acceptable tradeoff for 4PC performance.
3. **`MAX_QSEARCH_DEPTH = 4`** (reduced from spec's 8). Sufficient for most capture chains. Can increase if tactical quality is insufficient.
4. **Bootstrap eval causes pawn-heavy play.** This is an eval issue (territory/pawn advancement weighted too high), not quiescence. Addressed by NNUE in Stages 15-17.

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| NPS (release, depth 4) | ~33-60k | Down from ~84k pre-quiescence |
| Qnode overhead (starting pos, d2) | ~95% | Each leaf = 1 qnode (stand-pat) |
| Qnode overhead (developed, d4) | ~2.3x | Captures appear in developed positions |
| Min depth guarantee | depth 4 | Time abort suspended until complete |

## Open Questions

1. Should Stage 9 (TT + Move Ordering) apply to quiescence moves? MVV-LVA ordering of captures in qsearch would improve delta pruning effectiveness.
2. Should `generate_captures_only()` be replaced with a dedicated pseudo-legal capture generator in Stage 20 for performance?

## Reasoning

- **Filter pseudo-legal then validate:** Cheaper than full legal movegen + filter. Only captures need legality check, skipping all quiet moves.
- **Root-player captures only:** Full 4PC quiescence is O(captures³) per leaf — too expensive. Root-player filter keeps it manageable.
- **Stand-pat as lower bound:** Standard quiescence pattern. If no capture improves position, return static eval.
- **Delta pruning:** Skips captures that can't possibly raise score above current best even with optimistic capture value + margin.
- **Min depth guarantee:** Without it, quiescence overhead pushes iterative deepening below depth 4 within the time budget, degrading play quality.
