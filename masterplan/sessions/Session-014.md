# Session 014

**Date:** 2026-03-07 / 2026-03-08
**Stage:** 9 (TT + Move Ordering)

---

## What Was Done

### Stage 9: Transposition Table + Move Ordering (full implementation)

1. **`tt.rs` — Transposition Table** (~280 lines):
   - `TTEntry`: 20-byte compact struct (key_verify, best_move, scores[4], depth, flag, generation)
   - `TranspositionTable`: Vec-backed, power-of-2 sizing, 16MB default (~700K entries)
   - Replacement policy: empty > same position > deeper/equal > stale generation
   - Key verification via upper 32 bits of Zobrist hash
   - Stats tracking: probes, hits, hit_rate_pct()
   - 13 unit tests

2. **`move_order.rs` — Move Ordering Heuristics** (~460 lines):
   - `KillerTable`: 2 slots per ply per player, shift-insert, stats tracking
   - `HistoryTable`: 196×196 from-to, depth² bonus, age(), `raw()` accessor (ADR-007)
   - `mvv_lva_score()`: CAPTURE_BASE + victim*10 - attacker
   - `score_move()`: unified scoring — TT > captures > promotions > killers > history
   - `order_moves()`: scores and sorts in-place
   - `order_captures_mvv_lva()`: for quiescence
   - 10 unit tests

3. **`search.rs` — TT/Ordering Integration**:
   - `MaxnSearcher` gains `tt`, `killers`, `history` fields
   - All recursive functions changed to `&mut self`
   - `maxn()`: TT probe (exact hit), TT store, history update, killer store
   - `negamax_2p()`: Full alpha-beta TT with bound cutoffs, killer/history on beta cutoff
   - `beam_select()`: TT move always first, `score_move()` for pre-filtering
   - `qsearch()`/`qsearch_2p()`: MVV-LVA capture ordering
   - `iterative_deepening()`: `tt.new_search()`, `killers.clear()`, stats reporting
   - `history_table()` accessor (ADR-007)
   - 5 new unit tests

4. **`protocol/output.rs` — Stats Output**:
   - `format_info()` extended with `tt_hit_rate` and `killer_hit_rate` params
   - Info strings include `tthitrate` and `killerhitrate`

5. **Documentation**:
   - Audit log (`audit_log_stage_09.md`)
   - Downstream log (`downstream_log_stage_09.md`)

### Performance

- NPS improved to ~89.7k at depth 5 release (up from ~33-60k at depth 4 post-qsearch)
- TT hit rate ~4-5% at starting position (beam 30 = wide)
- 338 tests pass, 0 clippy warnings

---

## Commits

- `ce7f08f` — [Stage 09] Implement transposition table + move ordering

---

## What Was NOT Completed

- User UI verification of Stage 9 (requested, not yet confirmed)
- Stage 5 deferred debt (post-audit, downstream log, vault notes)
- Session notes for Sessions 7, 8, 11, 12
- Dead code: `apply_move_with_events`
- Debug build time abort bug

---

## Decisions Made

- Max^n TT uses TTFlag::Exact only (no alpha-beta bounds in 4-player search)
- Vec in TT acceptable per ADR-004 (allocated once, never cloned)
- History persists across ID iterations, cleared between moves
- Killers store on score improvement in Max^n (soft signal without beta cutoffs)
- Default TT size: 16MB (~700K entries)

---

## Open Issues

- TT hit rate below 30% acceptance criterion at starting position — expected with beam 30, will improve with NNUE beam narrowing
- Killer effectiveness limited in Max^n (no beta cutoffs) — real benefit in negamax_2p endgames
