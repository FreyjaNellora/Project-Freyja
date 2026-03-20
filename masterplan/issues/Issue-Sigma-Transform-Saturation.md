---
tags: [issue, mcts, gumbel, stage-14]
severity: Warning
stage: 10-14
status: resolved
date_created: 2026-03-19
date_resolved: 2026-03-19
last_updated: 2026-03-19
---

# Issue: Sigma Transform Saturation in Gumbel Sequential Halving

## Description

The sigma transform in Sequential Halving used `q / 100.0` to scale centipawn Q-values before applying `sigma(g + log_prior - q_scaled)`. With Q-values in the range [-10000, +10000], dividing by 100 gives [-100, +100], which completely saturates the sigmoid. Any Q-value difference > ~5cp dominates the Gumbel noise (~[-2, +5]) and log-prior (~[-3, 0]), effectively eliminating Gumbel exploration.

**Impact:** Sequential Halving always kept the highest-Q candidate regardless of Gumbel noise or prior quality. The policy improvement guarantee of Gumbel MCTS was not functioning. MCTS was pure exploitation with no principled exploration at the root.

**Present since:** Stage 10 (MCTS implementation).

## Resolution

Normalize Q-values to [0, 1] range across candidates using min/max normalization:

```rust
let q_norm = if q_range > 0.0 { (q - q_min) / q_range } else { 0.5 };
let x = g + log_prior - q_norm;
let sigma = 1.0 / (1.0 + (-x).exp());
```

This keeps Q on the same scale as Gumbel noise and log-prior, allowing meaningful exploration.

**Fixed in:** Session 21, commit 61d05bf.

## Related

- [[ADR-006]] Gumbel MCTS over UCB1
- [[MCTS]]
- [Gumbel AlphaZero paper](https://openreview.net/pdf?id=bERaNdoegnO)
