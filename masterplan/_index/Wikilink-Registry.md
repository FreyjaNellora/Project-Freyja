# Wikilink Registry

> Single source of truth for all `[[wikilink]]` targets in the vault.

**Last Updated:** 2026-03-16

---

## Core Documents

| Wikilink | File | Description |
|----------|------|-------------|
| [[MASTERPLAN]] | `masterplan/MASTERPLAN.md` | Stage specs, architecture, invariants |
| [[AGENT_CONDUCT]] | `masterplan/AGENT_CONDUCT.md` | Agent behavior rules |
| [[4PC_RULES_REFERENCE]] | `masterplan/4PC_RULES_REFERENCE.md` | Board geometry, game rules |
| [[DECISIONS]] | `masterplan/DECISIONS.md` | Architectural decision records |
| [[STATUS]] | `masterplan/STATUS.md` | Current project state |
| [[HANDOFF]] | `masterplan/HANDOFF.md` | Session handoff document |

---

## MOCs (Maps of Content)

| Wikilink | File | Description |
|----------|------|-------------|
| [[MOC-Project-Freyja]] | `masterplan/_index/MOC-Project-Freyja.md` | Top-level project map |
| [[MOC-Tier-1-Foundation]] | `masterplan/_index/MOC-Tier-1-Foundation.md` | Stages 0-5 |
| [[MOC-Tier-2-Core-Search]] | `masterplan/_index/MOC-Tier-2-Core-Search.md` | Stages 6-9 |
| [[MOC-Tier-3-Strategic]] | `masterplan/_index/MOC-Tier-3-Strategic.md` | Stages 10-11 |
| [[MOC-Tier-4-Measurement]] | `masterplan/_index/MOC-Tier-4-Measurement.md` | Stages 12-13 |
| [[MOC-Tier-5-Intelligence]] | `masterplan/_index/MOC-Tier-5-Intelligence.md` | Stages 14-17 |
| [[MOC-Tier-6-Polish]] | `masterplan/_index/MOC-Tier-6-Polish.md` | Stages 18-20 |
| [[MOC-Active-Issues]] | `masterplan/_index/MOC-Active-Issues.md` | Open issue tracker |
| [[MOC-Sessions]] | `masterplan/_index/MOC-Sessions.md` | Session history |
| [[Wikilink-Registry]] | `masterplan/_index/Wikilink-Registry.md` | This file |

---

## Sessions

| Wikilink | File | Description |
|----------|------|-------------|
| [[Session-001]] | `masterplan/sessions/Session-001.md` | Initial scaffolding session |
| [[Session-002]] | `masterplan/sessions/Session-002.md` | Stage 0 implementation |
| [[Session-003]] | `masterplan/sessions/Session-003.md` | Stage 0 closure + Stage 1 Board Representation |
| [[Session-005]] | `masterplan/sessions/Session-005.md` | Stage 2 closure + Stage 3 Game State |
| [[Session-006]] | `masterplan/sessions/Session-006.md` | Stage 3 closure + Stage 4 Protocol |
| [[Session-009]] | `masterplan/sessions/Session-009.md` | Stage 7 planning session |
| [[Session-010]] | `masterplan/sessions/Session-010.md` | Bug fixes, observer tool, UI feature gap analysis |
| [[Session-013]] | `masterplan/sessions/Session-013.md` | Stage 9 closure, TT + move ordering |
| [[Session-014]] | `masterplan/sessions/Session-014.md` | Stage 9 completion, Stage 10 MCTS start |
| [[Session-015]] | `masterplan/sessions/Session-015.md` | MCTS implementation |
| [[Session-016]] | `masterplan/sessions/Session-016.md` | Stage 10 closure, Stage 11 hybrid integration |
| [[Session-019]] | `masterplan/sessions/Session-019.md` | Stage 12 completion, qsearch discussion |
| [[Session-025]] | `masterplan/sessions/Session-025.md` | Stage 15 wrap-up, UI hang diagnosed |
| [[Session-026]] | `masterplan/sessions/Session-026.md` | Stage 15 closure, UI IPC fix |

---

## Audit & Downstream Logs

| Wikilink | File | Description |
|----------|------|-------------|
| [[audit_log_stage_00]] | `masterplan/audit_log_stage_00.md` | Stage 0 audit log |
| [[downstream_log_stage_00]] | `masterplan/downstream_log_stage_00.md` | Stage 0 downstream log |
| [[audit_log_stage_01]] | `masterplan/audit_log_stage_01.md` | Stage 1 audit log |
| [[downstream_log_stage_01]] | `masterplan/downstream_log_stage_01.md` | Stage 1 downstream log |
| [[audit_log_stage_02]] | `masterplan/audit_log_stage_02.md` | Stage 2 audit log |
| [[downstream_log_stage_02]] | `masterplan/downstream_log_stage_02.md` | Stage 2 downstream log |
| [[audit_log_stage_03]] | `masterplan/audit_log_stage_03.md` | Stage 3 audit log |
| [[downstream_log_stage_03]] | `masterplan/downstream_log_stage_03.md` | Stage 3 downstream log |
| [[audit_log_stage_04]] | `masterplan/audit_log_stage_04.md` | Stage 4 audit log |
| [[downstream_log_stage_04]] | `masterplan/downstream_log_stage_04.md` | Stage 4 downstream log |
| [[audit_log_stage_12]] | `masterplan/audit_log_stage_12.md` | Stage 12 audit log |
| [[downstream_log_stage_12]] | `masterplan/downstream_log_stage_12.md` | Stage 12 downstream log |
| [[audit_log_stage_15]] | `masterplan/audit_log_stage_15.md` | Stage 15 audit log |
| [[downstream_log_stage_15]] | `masterplan/downstream_log_stage_15.md` | Stage 15 downstream log |

---

## Components

| Wikilink | File | Description |
|----------|------|-------------|
| [[Component-Board]] | `masterplan/components/Component-Board.md` | Board representation (Stage 1) |
| [[Component-MoveGen]] | `masterplan/components/Component-MoveGen.md` | Move generation (Stage 2) |
| [[Component-GameState]] | `masterplan/components/Component-GameState.md` | Game state management (Stage 3) |
| [[Component-Protocol]] | `masterplan/components/Component-Protocol.md` | Engine-UI protocol (Stage 4) |
| [[Component-ProgressiveWidening]] | `masterplan/components/Component-ProgressiveWidening.md` | PW at root-player MCTS nodes (Stage 15) |
| [[Component-RayAttenuation]] | `masterplan/components/Component-RayAttenuation.md` | Ray-attenuated influence maps (Stage 15) |

---

## Connections

| Wikilink | File | Description |
|----------|------|-------------|
| [[Connection-Board-to-MoveGen]] | `masterplan/connections/Connection-Board-to-MoveGen.md` | Board → MoveGen interface |
| [[Connection-GameState-to-Protocol]] | `masterplan/connections/Connection-GameState-to-Protocol.md` | GameState → Protocol interface |

---

## Patterns

| Wikilink | File | Description |
|----------|------|-------------|
| [[Pattern-Fixed-Size-Piece-List]] | `masterplan/patterns/Pattern-Fixed-Size-Piece-List.md` | Swap-remove array for O(1) piece list ops |
| [[Pattern-Zobrist-Incremental-Update]] | `masterplan/patterns/Pattern-Zobrist-Incremental-Update.md` | XOR-based incremental hash updates |
| [[Pattern-4PC-Pawn-Orientation]] | `masterplan/patterns/Pattern-4PC-Pawn-Orientation.md` | Per-player direction tables for pawn movement |
| [[Pattern-Elimination-Chain]] | `masterplan/patterns/Pattern-Elimination-Chain.md` | Loop-until-stable elimination cascade |
| [[Pattern-DKW-Processing]] | `masterplan/patterns/Pattern-DKW-Processing.md` | Dead King Walking random move processing |
| [[Pattern-Protocol-Status-Diffing]] | `masterplan/patterns/Pattern-Protocol-Status-Diffing.md` | Before/after status diff for event detection |

---

## Issues

| Wikilink | File | Description |
|----------|------|-------------|
| [[Issue-UI-Feature-Gaps]] | `masterplan/issues/Issue-UI-Feature-Gaps.md` | UI missing components needed for Stages 7-10 |
| [[Issue-UI-AutoPlay-Broken]] | `masterplan/issues/Issue-UI-AutoPlay-Broken.md` | Start button does nothing — useEngine render cascade |
| [[Issue-Depth4-Engine-Crash]] | `masterplan/issues/Issue-Depth4-Engine-Crash.md` | Engine crashes at depth 4 from qsearch explosion |
| [[Issue-Tauri-IPC-Hang]] | `masterplan/issues/Issue-Tauri-IPC-Hang.md` | Tauri IPC hangs at ply 30+ (resolved) |

---

## Rules

- **Reuse before create.** Check this registry before creating new wikilinks.
- **New targets require immediate registry update.**
- **No orphan links.** Every `[[target]]` must resolve to an actual file.
- **No duplicates.** One wikilink per concept.
- **Update when files are renamed or deleted.**
