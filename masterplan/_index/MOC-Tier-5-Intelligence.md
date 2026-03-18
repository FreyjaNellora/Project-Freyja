# MOC — Tier 5: Intelligence (Stages 14-17)

> MCTS intelligence (OMA, progressive widening), zone control, NNUE architecture, training, and integration.

---

## Stages

| Stage | Name | Status | Key Deliverable |
|-------|------|--------|-----------------|
| 14 | MCTS Opponent Move Abstraction (OMA) | Not Started | Skip opponent expansion, focus simulations on root player |
| 15 | Progressive Widening + Zone Control | Not Started | PW at opponent nodes + territory/influence/tension features |
| 16 | NNUE Architecture + Training Pipeline | Not Started | Network design, inference, PyTorch training |
| 17 | NNUE Integration | Not Started | Replace bootstrap eval with NNUE |

---

## Critical Notes

- **Stage 14 makes MCTS smarter** by abstracting over opponent moves (Baier & Kaisers 2020). Each simulation reaches 3-4x deeper into root player's future turns.
- **Stage 15 Part A (PW)** layers on top of OMA — progressive widening at opponent nodes balances speed vs accuracy automatically.
- **Stage 15 Part B (Zone Control)** adds territorial evaluation features needed for NNUE input.
- **Stage 17 is the most dangerous swap.** Before-and-after audit mandatory.
- As NNUE improves, beam width should tighten and depth should increase. Measure this.

---

*Populated as stages are completed.*

**Related:** [[MOC-Project-Freyja]], [[MASTERPLAN]]
