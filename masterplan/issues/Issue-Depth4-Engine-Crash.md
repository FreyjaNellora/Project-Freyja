# Issue: Engine Crash at Depth 4 in Midgame Positions

**Severity:** Warning
**Stage:** Stage 8 (Quiescence Search) / discovered in Stage 12 self-play
**Status:** open
**Created:** 2026-03-16
**Last Updated:** 2026-03-16 (Session 018)
**Date Resolved:** —

---

## Description

Engine crashes with `exit code=1` when running `go depth 4` in midgame positions (ply 18-22). The crash occurs during depth 4 search when quiescence search node counts become very large (5-10 million qnodes). Likely a stack overflow from deep recursive quiescence search on the 14x14 board.

## Reproduction

1. Run `node observer.mjs` with depth 4 and max_ply 200
2. Engine crashes around ply 18-22 of game 1
3. Last logged search: depth 3 completes, then engine dies during depth 4

## Observed Behavior

- Opening (ply 0-8): depth 4 takes 3-5 seconds, ~200K nodes, ~400K-600K qnodes — OK
- Early midgame (ply 8-16): depth 4 takes 10-60 seconds, ~700K nodes, ~5M-10M qnodes — growing fast
- Crash point (ply 18-22): depth 4 search never completes, engine process exits with code 1
- qnodes growth: 381K → 1.5M → 3.5M → 6M → 10M → crash

## Root Cause Hypothesis

Stack overflow in recursive quiescence search. The 14x14 board with 4 players generates many more captures per position than 8x8 chess. Quiescence search depth is capped at 8, but with 4 players each having ~10 captures, the branching factor is enormous.

## Impact

- Depth 4 self-play games cannot complete past the opening
- A/B testing at depth 4 is not possible
- Depth 2-4 games work fine (depth 4 confirmed as minimum: 10 games, 0 crashes)

## Workarounds

1. Use `go movetime N` instead of `go depth 4` — time budget prevents explosion
2. Use depth 4 for self-play
3. Increase stack size (`RUST_MIN_STACK` env var)

## Suggested Fix

1. Check if quiescence search has a stack depth limit or node count limit
2. Add a qsearch node count cutoff (e.g., 5M qnodes → abort qsearch and return static eval)
3. Consider iterative quiescence instead of recursive

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_12]], [[Component-Search]]
