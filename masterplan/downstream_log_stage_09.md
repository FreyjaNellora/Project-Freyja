# Downstream Log -- Stage 9: Transposition Table + Move Ordering

## Must-Know

1. **All recursive search functions are `&mut self`.** `maxn()`, `negamax_2p()`, `qsearch()`, `qsearch_2p()` all take `&mut self` to access TT, killers, and history tables.
2. **TT is exact-only in Max^n.** `TTFlag::Exact` used for all 4-player search stores. Full bound logic (LowerBound/UpperBound) only in `negamax_2p()`.
3. **`beam_select()` accepts `tt_move` and `ply`.** TT best move is always included in the candidate set and searched first. Other moves scored by `score_move()` for pre-filtering.
4. **History table is extractable.** `pub fn history_table(&self) -> &HistoryTable` on `MaxnSearcher` (ADR-007). Returns reference to `[[u32; 196]; 196]` via `.raw()`.
5. **Captures sorted by MVV-LVA in quiescence.** Both `qsearch()` and `qsearch_2p()` call `order_captures_mvv_lva()` before the capture loop.
6. **TT hit rate and killer hit rate reported in info strings.** Format: `tthitrate {:.1}` and `killerhitrate {:.1}`.

## API Contracts

### New Modules

```rust
// tt.rs
pub const DEFAULT_TT_SIZE_MB: usize = 16;
pub enum TTFlag { Exact, LowerBound, UpperBound }
pub struct TTEntry { /* 20 bytes */ }
pub struct TranspositionTable { /* Vec-backed, power-of-2 */ }

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self;
    pub fn clear(&mut self);
    pub fn new_search(&mut self);           // bump generation, reset stats
    pub fn probe(&mut self, hash: u64) -> Option<&TTEntry>;
    pub fn store(&mut self, hash: u64, depth: u8, flag: TTFlag, scores: Score4, best_move: Option<Move>);
    pub fn stats(&self) -> (u64, u64);      // (hits, probes)
    pub fn hit_rate_pct(&self) -> f64;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

impl TTEntry {
    pub fn depth(&self) -> u8;
    pub fn flag(&self) -> TTFlag;
    pub fn scores(&self) -> &Score4;
    pub fn best_move(&self) -> Option<Move>;
}

// move_order.rs
pub fn mvv_lva_score(mv: Move) -> i32;
pub fn score_move(mv, tt_move, killers, history, ply, player) -> i32;
pub fn order_moves(moves, tt_move, killers, history, ply, player);
pub fn order_captures_mvv_lva(captures: &mut ArrayVec<Move, MAX_MOVES>);

pub struct KillerTable { /* 2 slots × 4 players × MAX_DEPTH */ }
impl KillerTable {
    pub fn new() -> Self;
    pub fn clear(&mut self);
    pub fn is_killer(&mut self, ply, player, mv) -> bool;    // updates stats
    pub fn is_killer_no_stats(&self, ply, player, mv) -> bool;
    pub fn killer_slot(&self, ply, player, mv) -> Option<usize>;
    pub fn store(&mut self, ply, player, mv);
    pub fn probe_increment(&mut self);
    pub fn stats(&self) -> (u64, u64);
    pub fn hit_rate_pct(&self) -> f64;
}

pub struct HistoryTable { /* Box<[[u32; 196]; 196]> */ }
impl HistoryTable {
    pub fn new() -> Self;
    pub fn clear(&mut self);
    pub fn get(&self, from: u8, to: u8) -> u32;
    pub fn update(&mut self, from: u8, to: u8, depth: u32);  // depth² bonus
    pub fn age(&mut self);                                     // halve all entries
    pub fn raw(&self) -> &[[u32; 196]; 196];                  // ADR-007
}
```

### Changed Signatures

```rust
// search.rs — SearchConfig extended
pub struct SearchConfig {
    pub beam_width: usize,
    pub sum_bound: i32,
    pub tt_size_mb: usize,  // NEW: default 16MB
}

// search.rs — SearchResult extended
pub struct SearchResult {
    // ... existing fields ...
    pub tt_hit_rate: f64,       // NEW: 0.0 - 100.0
    pub killer_hit_rate: f64,   // NEW: 0.0 - 100.0
}

// search.rs — MaxnSearcher extended
pub struct MaxnSearcher<E: Evaluator> {
    evaluator: E,
    config: SearchConfig,
    tt: TranspositionTable,      // NEW
    killers: KillerTable,        // NEW
    history: HistoryTable,       // NEW
}

// protocol/output.rs — format_info extended
pub fn format_info(
    depth: Option<u32>,
    scores: Option<[i16; 4]>,
    nodes: Option<u64>,
    qnodes: Option<u64>,
    nps: Option<u64>,
    pv: Option<&[Move]>,
    tt_hit_rate: Option<f64>,       // NEW
    killer_hit_rate: Option<f64>,   // NEW
) -> String;
```

### Move Ordering Priority

```
TT move:        1,000,000
Promotions:       500,000
Captures (MVV-LVA): 100,000 + victim*10 - attacker
Killer slot 0:     90,000
Killer slot 1:     80,000
History:           raw u32 value
Quiet:             0
```

## Known Limitations

1. **TT hit rate ~4-5% at starting position with beam 30.** Wide beam = few transpositions. NNUE narrowing beam will improve this significantly.
2. **Max^n TT is exact-only.** No bound cutoffs possible in 4-player search. TT benefit is transposition reuse + move ordering.
3. **Killers less effective in Max^n.** No beta cutoffs, so killers are stored on score improvement (soft signal). Full benefit in `negamax_2p()`.
4. **History not cleared between ID iterations.** Intentional — accumulates across iterations within a search. Cleared between moves.

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| NPS (release, depth 5) | ~89.7k | Up from ~33-60k pre-TT (depth 4) |
| TT hit rate (starting pos) | ~4-5% | Beam 30, opening — expected low |
| TT entries (16MB) | ~700K | 20 bytes per entry |
| Killer table size | ~1KB | 2 × 4 × MAX_DEPTH × sizeof(Option<Move>) |
| History table size | ~150KB | 196 × 196 × 4 bytes |

## Eval Improvements (Session 15)

Bootstrap eval (`eval.rs`) received four improvements:

1. **Castling bonus (80cp):** Flat bonus when king is on a castling destination square AND that side's rights are revoked. Proven +140cp advantage at depth 1 for castled vs un-castled positions. Uses `CASTLE_KING_DESTS` and `CASTLE_RIGHTS_BITS` constants (verified against `move_gen.rs` CASTLE_DEFS).

2. **King exposure penalty (45/20cp tiered):** Applied in `king_safety_score()` when shelter pawns ≤ 1 (severe: -45cp) or ≤ 2 (moderate: -20cp). Addresses the engine's tendency to ignore cracked king positions.

3. **Attacker amplifier (2x):** When shelter ≤ 1, `WEIGHT_KING_SAFETY_ATTACKER` is doubled. Makes attacking an already-exposed king doubly costly.

4. **Development score cap (8 units):** `development_score()` capped at 8 units (8 × 35 = 280cp max). Prevents development from drowning out 300-450cp captures at shallow depths.

**Cascading impacts:**
- **Stage 10 (MCTS):** MCTS leaf eval uses `eval_4vec` — these improvements apply automatically.
- **Stage 11 (Integration):** No impact — controller layer doesn't touch eval internals.
- **Stage 13 (Tuning):** Systematic weight tuning should re-evaluate these values with self-play A/B testing. The eval suite (`observer/baselines/`) provides a depth-4 tactical benchmark (25 positions, scoring rubric). Current baseline: 17/39 at depth 2.
- **Stage 14 (Zone Control):** New eval features will interact with castling bonus and exposure penalty. Zone control may subsume some of the king safety logic.
- **Stage 17 (NNUE):** All bootstrap eval features are replaced by NNUE. These improvements are temporary.

**Protocol extension:** `d` / `debug` command added (dumps FEN4). Used by observer eval suite. Downstream stages should be aware this command exists.

## Open Questions

1. Should Stage 10 (MCTS) use the same TT, or a separate one? Separate is cleaner — different search algorithms have different TT semantics.
2. Should `SearchConfig` be extended with `tt_enabled: bool` for comparison benchmarks? Currently TT is always active.
3. Should eval suite be run at depth 4 (more accurate but slower) or depth 2 (faster iteration)? User preference is depth 4.

## Reasoning

- **Vec in TT is acceptable per ADR-004:** Allocated once in MaxnSearcher, never cloned during search. Not in Board/GameState/MoveUndo.
- **Power-of-2 sizing:** `hash & mask` is faster than modulo. Wastes up to 50% of requested size but simplifies indexing.
- **Generation counter for replacement:** Avoids full TT clear between searches. Stale entries replaced opportunistically.
- **TT move always in beam:** Even if TT move would be filtered by beam_select's eval-based ranking, it's always included. TT move is the strongest signal from previous iterations.
- **`&mut self` for search functions:** Required because TT probe/store and killer/history updates need mutation. Alternative (interior mutability) is more complex for no benefit.
