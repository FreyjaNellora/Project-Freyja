# Downstream Log — Stage 12: Self-Play Framework

**Author:** Agent
**Date:** 2026-03-16
**Stage:** 12 (Self-Play Framework)

---

## Must-Know

1. **Self-play uses the Node.js observer pipeline** (`observer/`), not a Rust binary. All game running, stats, A/B comparison, and training data extraction are JavaScript (ESM).

2. **Single engine instance per game set.** The observer spawns one engine process and plays all 4 seats by sending `position startpos moves ...` then `go` for each ply.

3. **FEN4 captured per ply** via `d` protocol command after each `position` set. Stored in game JSON as `fen4` field on each ply.

4. **Game result determination:** Winner = player with highest final score. Reason: `last_standing` (1 player left), `max_ply` (hit ply limit), `max_rounds` (hit round limit).

5. **SPRT uses Gaussian model** on score differences (not binary win/loss). Adapted for 4-player chess where outcomes aren't binary. Each observation is `scoreB - scoreA` for a game pair.

6. **Training data is JSONL** — one JSON record per line. Each record: `{ fen4, eval_4vec, best_move, player, ply, round, depth, game_result }`. This is a VIEW of game JSON, not a separate format.

---

## API Contracts

### Observer CLI (`observer/observer.mjs`)

```
node observer.mjs [config.json]
```

Config:
```json
{
  "engine": "../target/release/freyja.exe",
  "games": 100,
  "depth": 2,              // or "movetime": 1000
  "max_ply": 400,
  "setoptions": {},         // optional: { "BeamWidth": "20" }
  "capture_raw": false,     // optional: include raw protocol lines
  "output_dir": "reports"
}
```

Outputs:
- `game_NNN.json` — per-game records
- `all_games.json` — all games array
- `summary.md` — human-readable summary
- `stats.json` — aggregate statistics
- `stats.md` — human-readable stats report

### Game JSON Schema (per ply)

```json
{
  "ply": 0,
  "round": 0,
  "player": "Red",
  "move": "d2d4",
  "fen4": "<FEN4 string>",
  "scores": { "red": 66, "blue": 66, "yellow": -287, "green": -287 },
  "depth": 2,
  "nodes": 156,
  "qnodes": null,
  "nps": 7800,
  "tthitrate": null,
  "killerhitrate": null,
  "pv": "d2d4 b9d9"
}
```

### A/B Runner (`observer/ab_runner.mjs`)

```
node ab_runner.mjs <ab_config.json>
```

Config:
```json
{
  "engine": "../target/release/freyja.exe",
  "games_per_config": 50,
  "max_ply": 400,
  "sprt": { "elo0": 0, "elo1": 20, "alpha": 0.05, "beta": 0.05 },
  "config_a": { "label": "baseline", "depth": 2, "setoptions": {} },
  "config_b": { "label": "candidate", "depth": 4, "setoptions": {} },
  "output_dir": "reports/ab_test_001"
}
```

Outputs: `games_a.json`, `games_b.json`, `comparison.json`, `comparison.md`

### Training Data Extraction (`observer/extract_training.mjs`)

```
node extract_training.mjs <games_json> [--min-depth N] [--min-ply N] [--output FILE]
```

### SPRT Class (`observer/lib/sprt.mjs`)

```javascript
import { SPRT } from './lib/sprt.mjs';
const sprt = new SPRT({ elo0: 0, elo1: 20, alpha: 0.05, beta: 0.05 });
const decision = sprt.update(scoreA, scoreB); // 'continue' | 'accept_h1' | 'accept_h0'
```

### Metrics (`observer/lib/metrics.mjs`)

```javascript
import { computeMetrics } from './lib/metrics.mjs';
const metrics = computeMetrics(gameRecord);
// Returns: { pawn_ratio, queen_activation_round, captures_per_10_rounds,
//            king_moves, shuffle_index, avg_score_delta_per_round, game_length_rounds }
```

### Stats (`observer/lib/stats.mjs`)

```javascript
import { aggregateStats, formatStatsReport } from './lib/stats.mjs';
const stats = aggregateStats(games);
// Returns: { total_games, win_rates, game_length, final_scores, metrics, per_game_metrics }
```

---

## Known Limitations

1. **Deterministic engine at same depth:** At depth 2, the engine always picks the same move from the same position (no randomization). This means identical-config games produce identical results. Differentiated results require different depths or beam widths.

2. **Score-based SPRT vs win-rate SPRT:** Standard chess SPRT uses pentanomial win/draw/loss model. Our Gaussian SPRT on score differences is an approximation suited to 4-player games but less well-studied. Use with appropriate skepticism.

3. **No persistent state across games:** Each game starts fresh (HybridSearcher created per `go`). No learning between games.

4. **Max ply cap affects results:** Games capped at 80 ply may not reach natural conclusions (checkmates/stalemates). Use higher caps (200+) for realistic game metrics.

5. **Training data lacks component breakdown:** Eval component scores (material, territory, mobility, etc.) are not in the game JSON. Adding them requires an engine-side `eval` protocol command (deferred to Stage 16 when NNUE needs it).

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| 3 games @ depth 2, 80 ply | ~15 seconds | Release build |
| A/B 3+3 games @ depth 1/2 | ~30 seconds | Including comparison |
| SPRT early stop | 2 pairs | Depth 1 vs 2 (large difference) |
| Training data extraction | <1 second | 3 games, 76 records |
| 100 games @ depth 2, 80 ply | ~8 minutes | Release build (estimate) |

---

## Open Questions

1. **Should training data include game outcome as a learning signal?** Currently yes (`game_result` included). Stage 16 will determine the NNUE training format.

2. **Is Gaussian SPRT appropriate for 4-player score distributions?** Seems to work empirically. Formal validation would require Monte Carlo simulation (deferred).

3. **How to handle deterministic identical-config games?** Add randomization (random opening moves, or random beam width perturbation) in Stage 13 tuning.

---

## Reasoning

- **Node.js over Rust for self-play tooling:** Self-play is not hot-path code. JSON manipulation is natural in JS. The observer already existed and was proven. Extending it avoided rewriting ~300 lines of protocol parsing.

- **FEN4 via `d` command:** Alternative was to compute FEN4 client-side from move replay. Using the engine's `d` command is authoritative and avoids duplicating board logic in JS.

- **SPRT on score differences:** Binary SPRT doesn't apply to 4-player games. Score-based Gaussian SPRT naturally handles the multi-player scoring model.

---

**Related:** [[MASTERPLAN]], [[AGENT_CONDUCT]], [[downstream_log_stage_11]]
