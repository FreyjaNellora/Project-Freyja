# MOC — Tier 1: Foundation (Stages 0-5)

> Everything that must be rock-solid before search begins.

---

## Stages

| Stage | Name | Status | Key Deliverable |
|-------|------|--------|-----------------|
| 0 | Project Skeleton | **Complete** | Compilable workspace, placeholder modules |
| 1 | Board Representation | **Implementation Complete** (awaiting user green light) | Board struct, Zobrist, FEN4, attacks |
| 2 | Move Generation | Not Started | Legal moves, make/unmake, perft |
| 3 | Game State | Not Started | Turns, elimination, DKW, scoring |
| 4 | Freyja Protocol | Not Started | Engine ↔ UI communication |
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

## Connections (populated as built)

*None yet.*

## Issues (populated as discovered)

*None yet.*

---

**Related:** [[MOC-Project-Freyja]], [[MASTERPLAN]]
