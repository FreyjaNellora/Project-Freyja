# MOC — Tier 2: Core Search (Stages 6-9)

> The search engine: evaluation, Max^n, quiescence, transposition tables.

---

## Stages

| Stage | Name | Status | Key Deliverable |
|-------|------|--------|-----------------|
| 6 | Bootstrap Evaluation | Not Started | Evaluator trait, material + PST + territory |
| 7 | Max^n Search | Not Started | Searcher trait, beam search, iterative deepening |
| 8 | Quiescence Search | Not Started | Capture extensions, stand-pat, delta pruning |
| 9 | TT + Move Ordering | Not Started | Hash table, killer/history, MVV-LVA |

---

## Tier 2 Invariants

- **Stage 6:** Evaluator trait is the eval boundary. eval_scalar and eval_4vec agree.
- **Stage 7:** Searcher trait is the search boundary. Engine finds forced mates.
- **Stage 9:** NPS does not regress >15% between stages.

---

*Populated as stages are completed.*

**Related:** [[MOC-Project-Freyja]], [[MASTERPLAN]]
