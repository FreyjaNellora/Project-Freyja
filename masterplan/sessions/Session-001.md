# Session 1: Initial Scaffolding & Architecture Design

**Date:** 2026-02-25
**Stage:** Pre-Stage 0 (Scaffolding)
**Duration:** ~3h

---

## Goals

- Create Project Freyja folder structure
- Research and design architecture (pure Max^n + NNUE beam search + MCTS)
- Write all foundational documents (MASTERPLAN, AGENT_CONDUCT, 4PC_RULES_REFERENCE)
- Write project management docs (CLAUDE.md, STATUS, HANDOFF, DECISIONS)
- Create Obsidian vault structure (templates, MOCs, indexes)

## Completed

- Project folder structure created with all subdirectories
- `cargo init --name freyja` — basic Rust project compiles
- MASTERPLAN.md v1.0 — 21 stages across 6 tiers
- AGENT_CONDUCT.md v1.0 — all Odin rules + 7 new Freyja additions
- 4PC_RULES_REFERENCE.md v1.0 — exhaustive board spec with exact square indices
- CLAUDE.md — agent orientation
- STATUS.md — project state tracker
- HANDOFF.md — session handoff
- DECISIONS.md — 5 initial ADRs
- Vault templates (issue, component, connection, pattern, session)
- Vault MOCs (Project Freyja, Tier 1, Active Issues, Sessions)
- Wikilink Registry

## Not Completed

- Git repository initialization (deferred to next session start)
- Stage 0 implementation (next session)
- Tier 2-6 MOC placeholders (created but minimal)

## Discoveries

1. **Athena coordinate system is incompatible:** Uses `rank * 16 + file` (padded) with 1-indexed coords. Freyja uses `rank * 14 + file` with 0-indexed internals. Cannot copy Athena constants directly.
2. **Blue/Yellow K-Q swap:** These two players have King before Queen in arrangement. Red/Green have Queen before King. This affects castling "kingside" designation and is a classic bug source.
3. **Capture point values:** Odin documented knight=3, bishop=5, rook=5. Should be verified against chess.com when UI is ready.
4. **EP timing in 4PC:** EP expires after 1 ply (next player only). This means most EP opportunities are missed because the player who could capture often isn't the one moving next.

## Decisions Made

- ADR-001: Pure Max^n (no BRS, no Paranoid)
- ADR-002: NNUE-guided beam search for tree reduction
- ADR-003: Freyja as training data generator for Odin
- ADR-004: Fixed-size data structures from day one
- ADR-005: `rank * 14 + file` coordinate system

## Issues Created/Resolved

| Issue | Action | Severity |
|-------|--------|----------|
| — | — | — |

No issues — project hasn't started implementation.

## Files Modified

All files are new (project creation). See HANDOFF.md for complete list.

## Next Session Should

1. Initialize git repo with `.gitignore` and initial commit
2. Begin Stage 0: Project Skeleton (Cargo workspace, placeholder modules, UI project)
3. Get user green light on Stage 0 before proceeding

---

**Related:** [[HANDOFF]], [[STATUS]], [[MASTERPLAN]]
