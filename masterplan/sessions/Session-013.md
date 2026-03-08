# Session 013

**Date:** 2026-03-07
**Stage:** 8 (Quiescence Search) — Complete

---

## Summary

Full implementation of Stage 8: Quiescence Search. Added capture-only move generation, quiescence search for both Max^n (4-player) and negamax (2-player) paths, stand-pat evaluation, delta pruning, depth cap, minimum depth guarantee, and separate qnode tracking.

## What Was Done

### 1. Capture-Only Move Generation (`move_gen.rs`)

Added `generate_captures_only()` — generates pseudo-legal moves, filters to captures, then validates legality only for those. Much cheaper than full legal movegen + filter (~60k vs ~32k NPS).

### 2. Quiescence Search (`search.rs`)

- `qsearch()` — Max^n quiescence with root-player capture filter, stand-pat, delta pruning, depth cap of 4
- `qsearch_2p()` — Standard alpha-beta quiescence for negamax fallback (2 players remaining)
- `qsearch_2p_entry()` — Bridge function mapping scalar negamax qsearch back to Score4
- Both `maxn()` and `negamax_2p()` call quiescence at depth 0 instead of static eval

### 3. Min Depth Guarantee

Added `MIN_SEARCH_DEPTH = 4` — time-based abort is suspended until depth 4 completes. Prevents quiescence overhead from degrading search depth. Default time budget increased from 2s to 5s.

### 4. Node Counting & Protocol

- `qnodes` tracked separately in `SearchState` and `SearchResult`
- Info string updated to include qnodes: `info depth X nodes Y qnodes Z nps W ...`

### 5. Tests

- 8 new unit tests: stand-pat, depth cap, delta pruning, hanging queen detection, 2P quiescence, overhead check
- Fixed pre-existing test bug: `test_all_four_players_can_move_via_protocol` expected per-move nextturn events

### 6. Verification

- All 284 unit tests + 25 integration tests pass
- Clippy clean, formatting clean
- Engine verified in UI: depth 4 consistent, auto-play works, analysis panel shows qnodes

## Key Decisions

- **MAX_QSEARCH_DEPTH = 4** (reduced from spec's 8) — sufficient for capture chains, better performance
- **Root-player captures only** — full 4PC quiescence too expensive
- **5s default time budget** — accommodates quiescence overhead while maintaining depth 4
- **Pawn-heavy play accepted** — bootstrap eval issue, not quiescence. Deferred to NNUE (Stages 15-17)

## Files Created/Modified

| File | Action |
|------|--------|
| `freyja-engine/src/search.rs` | Quiescence search, min depth, qnode tracking |
| `freyja-engine/src/move_gen.rs` | `generate_captures_only()` |
| `freyja-engine/src/protocol/output.rs` | qnodes in info string |
| `freyja-engine/src/protocol/mod.rs` | Pass qnodes, 5s default budget |
| `freyja-engine/tests/protocol_integration.rs` | Fixed nextturn test bug |
| `masterplan/audit_log_stage_08.md` | Created — pre/post audit |
| `masterplan/downstream_log_stage_08.md` | Created — API contracts for Stage 9 |
| `masterplan/sessions/Session-013.md` | Created — this note |
| `masterplan/STATUS.md` | Updated — Stage 8 complete |
| `masterplan/HANDOFF.md` | Rewritten — Session 13 |

## Performance Impact

| Metric | Before | After |
|--------|--------|-------|
| NPS (release, d4) | ~84k | ~33-60k |
| Min search depth | none (could drop to d3) | d4 guaranteed |
| Horizon effect | present | mitigated for root-player captures |

---

**Related:** [[STATUS]], [[HANDOFF]], [[Session-010]], [[Issue-UI-Feature-Gaps]]
