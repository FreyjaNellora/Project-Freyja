# Project Freyja

A four-player chess engine built for evaluation accuracy. Pure Max^n search with NNUE-guided beam search models true multi-player dynamics at depth 7-8 — the engine searches deeper as the NNUE gets smarter. Paired with MCTS for strategic play.

---

## Architecture

```
MCTS (Max^n backpropagation, NNUE leaf eval)
  │
Max^n Search (depth 7-8, NNUE-guided beam search)
  │  Iterative deepening, shallow pruning
  │  Beam width adapts to NNUE maturity
  │  2 players remaining → negamax with full alpha-beta
  │
  ├── Quiescence (root-player captures, capped depth)
  ├── Move ordering (TT + killer + history + MVV-LVA)
  │
NNUE Evaluation
  ├── Material + Piece-Square Tables
  ├── Mobility (safe squares, piece activity)
  ├── Territory (BFS Voronoi partitioning)
  ├── Influence maps (piece-weighted decay)
  ├── King safety zones
  ├── Tension / vulnerability maps
  └── Tactical features (captures, threats, checks)
```

## How It Works

**Max^n** is the only multi-player search algorithm that correctly models all four players maximizing their own scores simultaneously. Traditional engines use approximations (BRS, Paranoid) that trade accuracy for speed. Freyja takes the opposite approach — accuracy first, with smart pruning to make it feasible.

**NNUE-guided beam search** is what makes deep Max^n possible. At every node, the NNUE ranks all legal moves and only the top K are expanded. As the NNUE improves through training, it ranks moves more accurately, the beam narrows, and the engine searches deeper on the same node budget.

| NNUE Maturity | Beam Width | Depth @ 7M Nodes |
|---------------|-----------|------------------|
| Bootstrap | 12-15 | 5-6 |
| Early NNUE | 8-10 | 6-7 |
| Mature NNUE | 5-8 | 7-8 |

**MCTS** handles strategic exploration at the top level, using Max^n backpropagation and NNUE as the leaf evaluator.

## The Board

14x14 grid with four 3x3 corners removed = 160 playable squares. Four players (Red, Blue, Yellow, Green) move clockwise.

```
    a   b   c   d   e   f   g   h   i   j   k   l   m   n
14  .   .   .   rR  yN  yB  yK  yQ  yB  yN  yR  .   .   .
13  .   .   .   yP  yP  yP  yP  yP  yP  yP  yP  .   .   .
12  .   .   .                                       .   .   .
11  bR  bP                                          gP  gR
10  bN  bP                                          gP  gN
 9  bB  bP                                          gP  gB
 8  bQ  bP                                          gP  gQ
 7  bK  bP                                          gP  gK
 6  bB  bP                                          gP  gB
 5  bN  bP                                          gP  gN
 4  bR  bP                                          gP  gR
 3  .   .   .                                       .   .   .
 2  .   .   .   rP  rP  rP  rP  rP  rP  rP  rP  .   .   .
 1  .   .   .   rR  rN  rB  rQ  rK  rB  rN  rR  .   .   .
    a   b   c   d   e   f   g   h   i   j   k   l   m   n
```

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Engine | Rust |
| UI | TypeScript + React |
| NNUE Training | Python + PyTorch |
| NNUE Inference | Rust (native) |

## Project Status

**Pre-Stage 0** — Scaffolding complete. 21 stages planned across 6 tiers:

| Tier | Stages | Focus |
|------|--------|-------|
| 1. Foundation | 0-5 | Board, movegen, game state, protocol, UI shell |
| 2. Core Search | 6-9 | Evaluation, Max^n, quiescence, TT |
| 3. Strategic | 10-11 | MCTS, integration |
| 4. Measurement | 12-13 | Self-play, beam tuning |
| 5. Intelligence | 14-17 | Zone control, NNUE |
| 6. Polish | 18-20 | Game modes, full UI, optimization |

## License

All rights reserved.
