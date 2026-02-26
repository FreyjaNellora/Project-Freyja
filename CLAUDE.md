# Project Freyja -- Agent Orientation

A four-player chess engine: Max^n with NNUE-guided beam search + MCTS.

## Before You Start

Read these files in order:

1. `masterplan/STATUS.md` -- Where is the project? What stage? What's blocked?
2. `masterplan/HANDOFF.md` -- What was the last session doing? What's next?
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 -- Full stage entry protocol.

If you're new to the project or starting a new stage, also read:
4. `masterplan/DECISIONS.md` -- Why key architectural choices were made.
5. `masterplan/MASTERPLAN.md` -- Full spec (refer to specific sections as needed).
6. `masterplan/4PC_RULES_REFERENCE.md` -- Exact board geometry, piece positions, castling paths for all 4 players.

## Quick Understanding (Obsidian Vault)

For **fast lookup** on how the engine works, use the knowledge vault at `masterplan/`:

| You want to know... | Read |
|---|---|
| Big picture navigation | `masterplan/_index/MOC-Project-Freyja.md` |
| Tier 1 stages, logs, invariants | `masterplan/_index/MOC-Tier-1-Foundation.md` |
| Known issues | `masterplan/_index/MOC-Active-Issues.md` |
| Session history | `masterplan/_index/MOC-Sessions.md` |
| All wikilink targets | `masterplan/_index/Wikilink-Registry.md` |

Full vault instructions: `masterplan/AGENT_CONDUCT.md` Sections 1.12-1.13

## What Goes Where -- The Hard Line

| Content | Where | Rule |
|---|---|---|
| Stage specs, acceptance criteria | `masterplan/MASTERPLAN.md` | Authoritative. Never duplicate elsewhere. |
| Board rules, piece positions, castling paths | `masterplan/4PC_RULES_REFERENCE.md` | Authoritative for all game rules. |
| Agent behavior rules | `masterplan/AGENT_CONDUCT.md` | HOW agents work. |
| ADRs, architectural decisions | `masterplan/DECISIONS.md` | Why key choices were made. |
| Audit logs, downstream logs | `masterplan/` | Formal records per stage. |
| Project state, session handoff | `masterplan/STATUS.md` + `HANDOFF.md` | Update per AGENT_CONDUCT.md 1.14. |
| Implementation knowledge, component docs | `masterplan/components/` | How things actually work at code level. |
| Component relationships | `masterplan/connections/` | How things connect to each other. |
| Session history | `masterplan/sessions/` | Preserved history (HANDOFF gets overwritten). |
| Bugs, workarounds | `masterplan/issues/` | Runtime problems and resolutions. |
| Implementation patterns | `masterplan/patterns/` | Reusable approaches. |

## Core Architecture (Quick Reference)

```
MCTS (Max^n backpropagation, NNUE leaf eval)
  │
Max^n Search (depth 7-8, NNUE-guided beam search)
  │  Beam width adapts to NNUE maturity
  │  2 players remaining → negamax
  ├── Quiescence (root-player captures, capped depth)
  ├── Move ordering (TT + killer + history + MVV-LVA)
  │
NNUE Evaluation
  ├── Material + PST + mobility
  ├── Territory (BFS Voronoi)
  ├── Influence maps (decay)
  ├── King safety zones
  ├── Tension / vulnerability
  └── Tactical features
```

**Key difference from Odin:** No BRS. No Paranoid. Pure Max^n with NNUE-guided beam search. The NNUE IS the pruning strategy — smarter eval → tighter beam → deeper search.

## Critical Rules (Never Forget)

1. **Fixed-size data structures in hot paths.** No `Vec<T>` in Board, GameState, or MoveUndo.
2. **4PC Verification Matrix.** Every game rule tested for all 4 player orientations independently.
3. **Stages aren't done until the user says so** from testing in the UI.
4. **Debugging anti-spiral:** each analysis pass must cite something NEW or you're spiraling (AGENT_CONDUCT 1.15).
5. **Session-end protocol:** update HANDOFF.md, STATUS.md, create session note (AGENT_CONDUCT 1.14).

## Relationship to Project Odin

Freyja is the teacher. Odin is the student.

- Freyja generates accurate training data via Max^n (truthful multi-player evaluations)
- Odin consumes it for NNUE training
- Freyja prioritizes accuracy; Odin prioritizes speed
- Together they cover both philosophies of multi-player game tree search

## At Session End

1. Update `masterplan/HANDOFF.md` and `masterplan/STATUS.md` (per AGENT_CONDUCT.md 1.14).
2. Create vault notes per AGENT_CONDUCT.md 1.13 (issues, components, connections, patterns).
3. Create a session note in `masterplan/sessions/`.
