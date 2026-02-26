# MOC — Project Freyja

> Map of Content: Top-level navigation for the entire project.

---

## Core Documents

- [[MASTERPLAN]] — Stage specifications, architecture, invariants
- [[AGENT_CONDUCT]] — Agent behavior rules, audit procedures
- [[4PC_RULES_REFERENCE]] — Board geometry, piece positions, game rules
- [[DECISIONS]] — Architectural decision records
- [[STATUS]] — Current project state
- [[HANDOFF]] — Session-to-session handoff

---

## Tier Maps

- [[MOC-Tier-1-Foundation]] — Stages 0-5: Skeleton, Board, MoveGen, GameState, Protocol, UI
- [[MOC-Tier-2-Core-Search]] — Stages 6-9: Eval, Max^n, Quiescence, TT+MoveOrdering
- [[MOC-Tier-3-Strategic]] — Stages 10-11: MCTS, Integration
- [[MOC-Tier-4-Measurement]] — Stages 12-13: Self-Play, Time+Beam Tuning
- [[MOC-Tier-5-Intelligence]] — Stages 14-17: Zone Control, NNUE Arch, Training, Integration
- [[MOC-Tier-6-Polish]] — Stages 18-20: Game Modes, Full UI, Optimization

---

## Active Tracking

- [[MOC-Active-Issues]] — All open bugs and concerns
- [[MOC-Sessions]] — Session history

---

## Knowledge Base

| Category | Folder | Contents |
|----------|--------|----------|
| Components | `components/` | How engine parts work |
| Connections | `connections/` | How parts connect |
| Patterns | `patterns/` | Reusable approaches |
| Issues | `issues/` | Bugs and resolutions |
| Sessions | `sessions/` | Session records |

---

## Quick Reference

- **Architecture:** Max^n + NNUE beam search + MCTS
- **Board:** 14x14, 160 valid squares, 4 players
- **Search target:** Depth 7-8, 7-8M nodes
- **Beam schedule:** 12-15 (bootstrap) → 5-8 (mature NNUE)
- **Relationship to Odin:** Freyja = teacher, Odin = student
