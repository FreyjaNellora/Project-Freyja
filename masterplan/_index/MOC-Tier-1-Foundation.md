# MOC — Tier 1: Foundation (Stages 0-5)

> Everything that must be rock-solid before search begins.

---

## Stages

| Stage | Name | Status | Key Deliverable |
|-------|------|--------|-----------------|
| 0 | Project Skeleton | **Complete** | Compilable workspace, placeholder modules |
| 1 | Board Representation | **Complete** | Board struct, Zobrist, FEN4, attacks |
| 2 | Move Generation | **Complete** | Legal moves, make/unmake, perft |
| 3 | Game State | **Complete** | Turns, elimination, DKW, scoring |
| 4 | Freyja Protocol | **Awaiting Green Light** | Engine ↔ UI communication |
| 5 | UI Shell | Not Started | Visual board, click-to-move |

---

## Tier 1 Invariants (from MASTERPLAN Section 4.1)

- **Stage 0:** Prior-stage tests never deleted.
- **Stage 2:** Perft values are forever. Zobrist make/unmake round-trip. Attack query API is the board boundary.
- **Stage 3:** Game playouts complete without crashes. Eliminated players never trigger movegen.
- **Stage 5:** UI owns zero game logic.

---

## Tier Boundary Review

Before starting Tier 2 (Stage 6), a full tier boundary review is required (AGENT_CONDUCT 1.20):
1. Run ALL maintenance invariants
2. Review ALL open issues
3. Confirm all hot-path data structures are fixed-size
4. Get user sign-off

---

## Audit Logs

| Stage | Audit Log | Downstream Log |
|-------|-----------|---------------|
| 0 | `audit_log_stage_00.md` | `downstream_log_stage_00.md` |
| 1 | `audit_log_stage_01.md` | `downstream_log_stage_01.md` |
| 2 | `audit_log_stage_02.md` | `downstream_log_stage_02.md` |
| 3 | `audit_log_stage_03.md` | `downstream_log_stage_03.md` |
| 4 | `audit_log_stage_04.md` | `downstream_log_stage_04.md` |
| 5 | `audit_log_stage_05.md` | `downstream_log_stage_05.md` |

---

## Components (populated as built)

- [[Component-Board]] — Board representation, Zobrist hashing, FEN4, attack queries
- [[Component-MoveGen]] — Legal move generation, make/unmake, perft
- [[Component-GameState]] — Game state, turns, elimination, DKW, scoring
- [[Component-Protocol]] — Engine-UI protocol, command parsing, info output

## Connections (populated as built)

- [[Connection-Board-to-MoveGen]] — Board → MoveGen interface
- [[Connection-GameState-to-Protocol]] — GameState → Protocol interface

## Patterns

- [[Pattern-4PC-Pawn-Orientation]] — Per-player direction tables
- [[Pattern-Elimination-Chain]] — Loop-until-stable cascade
- [[Pattern-DKW-Processing]] — Dead King Walking random moves
- [[Pattern-Protocol-Status-Diffing]] — Before/after event detection

## Issues (populated as discovered)

*None open.*

---

**Related:** [[MOC-Project-Freyja]], [[MASTERPLAN]]
