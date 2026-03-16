// lib/ab.mjs — A/B comparison logic for self-play framework
//
// Compares two sets of game records (config A vs config B) and produces
// statistical comparisons of win rates, scores, and behavioral metrics.
//
// Stage 12: Self-Play Framework

import { mean, stddev, confidenceInterval95, tTest, chiSquaredTest } from './stats.mjs';
import { computeMetrics } from './metrics.mjs';

const PLAYERS = ['red', 'blue', 'yellow', 'green'];

/**
 * Approximate Elo difference from average score difference.
 * In 4-player context, uses score-based estimation rather than win/loss.
 *
 * @param {number} avgScoreA - Average total score for config A
 * @param {number} avgScoreB - Average total score for config B
 * @returns {number} Estimated Elo difference (B - A, positive means B is stronger)
 */
function eloDiffFromScores(avgScoreA, avgScoreB) {
  // Each 100cp difference ≈ ~30 Elo in 4-player (rough heuristic)
  return (avgScoreB - avgScoreA) * 0.3;
}

/**
 * Compare two sets of game records from different engine configurations.
 *
 * @param {Object[]} gamesA - Game records for config A
 * @param {Object[]} gamesB - Game records for config B
 * @param {string} labelA - Label for config A
 * @param {string} labelB - Label for config B
 * @returns {Object} Comparison results
 */
export function compareConfigs(gamesA, gamesB, labelA = 'A', labelB = 'B') {
  // Compute average total score per game (sum of all 4 players' scores from the winning player's perspective)
  // For A/B comparison: use average score across all seats as the measure
  function avgTotalScore(games) {
    const scores = [];
    for (const g of games) {
      if (g.game_result?.final_scores) {
        const s = g.game_result.final_scores;
        const total = (s.red ?? 0) + (s.blue ?? 0) + (s.yellow ?? 0) + (s.green ?? 0);
        scores.push(total / 4); // Average per seat
      }
    }
    return scores;
  }

  const scoresA = avgTotalScore(gamesA);
  const scoresB = avgTotalScore(gamesB);

  // Per-player scores
  const perPlayerScores = {};
  for (const p of PLAYERS) {
    const valsA = gamesA.map((g) => g.game_result?.final_scores?.[p] ?? 0);
    const valsB = gamesB.map((g) => g.game_result?.final_scores?.[p] ?? 0);
    perPlayerScores[p] = {
      a: confidenceInterval95(valsA),
      b: confidenceInterval95(valsB),
      test: tTest(valsA, valsB),
    };
  }

  // Overall score comparison
  const scoreTest = tTest(scoresA, scoresB);
  const eloDiff = eloDiffFromScores(mean(scoresA), mean(scoresB));

  // Win rate comparison
  const winsA = { red: 0, blue: 0, yellow: 0, green: 0 };
  const winsB = { red: 0, blue: 0, yellow: 0, green: 0 };
  for (const g of gamesA) {
    const w = g.game_result?.winner?.toLowerCase();
    if (w && winsA[w] !== undefined) winsA[w]++;
  }
  for (const g of gamesB) {
    const w = g.game_result?.winner?.toLowerCase();
    if (w && winsB[w] !== undefined) winsB[w]++;
  }

  // Game length comparison
  const lengthsA = gamesA.map((g) => g.total_ply);
  const lengthsB = gamesB.map((g) => g.total_ply);
  const lengthTest = tTest(lengthsA, lengthsB);

  // Behavioral metrics comparison
  const metricsA = gamesA.map((g) => computeMetrics(g)).filter((m) => m !== null);
  const metricsB = gamesB.map((g) => computeMetrics(g)).filter((m) => m !== null);

  const metricDiffs = {};

  // Pawn ratio
  if (metricsA.length > 0 && metricsB.length > 0) {
    const prA = metricsA.map((m) => m.pawn_ratio.avg).filter((v) => v !== null);
    const prB = metricsB.map((m) => m.pawn_ratio.avg).filter((v) => v !== null);
    metricDiffs.pawn_ratio = { a: mean(prA), b: mean(prB), test: tTest(prA, prB) };

    // Shuffle index
    const siA = metricsA.map((m) => m.shuffle_index);
    const siB = metricsB.map((m) => m.shuffle_index);
    metricDiffs.shuffle_index = { a: mean(siA), b: mean(siB), test: tTest(siA, siB) };

    // Game length (rounds)
    const rlA = metricsA.map((m) => m.game_length_rounds);
    const rlB = metricsB.map((m) => m.game_length_rounds);
    metricDiffs.game_length_rounds = { a: mean(rlA), b: mean(rlB), test: tTest(rlA, rlB) };
  }

  return {
    label_a: labelA,
    label_b: labelB,
    games_a: gamesA.length,
    games_b: gamesB.length,
    avg_score: {
      a: confidenceInterval95(scoresA),
      b: confidenceInterval95(scoresB),
      test: scoreTest,
      elo_diff: eloDiff,
    },
    per_player_scores: perPlayerScores,
    win_rates: { a: winsA, b: winsB },
    game_length: {
      a: confidenceInterval95(lengthsA),
      b: confidenceInterval95(lengthsB),
      test: lengthTest,
    },
    metric_diffs: metricDiffs,
    verdict: scoreTest.significant
      ? (mean(scoresB) > mean(scoresA) ? `${labelB} is stronger` : `${labelA} is stronger`)
      : 'no significant difference',
  };
}

/**
 * Format a human-readable A/B comparison report.
 */
export function formatABReport(comparison) {
  const lines = [];
  lines.push('# A/B Comparison Report');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push(`Config A: ${comparison.label_a} (${comparison.games_a} games)`);
  lines.push(`Config B: ${comparison.label_b} (${comparison.games_b} games)`);
  lines.push('');

  lines.push(`## Verdict: ${comparison.verdict}`);
  lines.push('');

  // Score comparison
  const sa = comparison.avg_score.a;
  const sb = comparison.avg_score.b;
  lines.push('## Average Score (per seat)');
  lines.push(`- ${comparison.label_a}: ${sa.mean.toFixed(1)} +/- ${sa.stddev.toFixed(1)}`);
  lines.push(`- ${comparison.label_b}: ${sb.mean.toFixed(1)} +/- ${sb.stddev.toFixed(1)}`);
  lines.push(`- Elo difference: ${comparison.avg_score.elo_diff.toFixed(1)} (${comparison.avg_score.elo_diff > 0 ? comparison.label_b + ' stronger' : comparison.label_a + ' stronger'})`);
  lines.push(`- p-value: ${comparison.avg_score.test.pValue.toFixed(4)} (${comparison.avg_score.test.significant ? 'SIGNIFICANT' : 'not significant'})`);
  lines.push('');

  // Win rates
  lines.push('## Win Rates');
  lines.push(`| Player | ${comparison.label_a} | ${comparison.label_b} |`);
  lines.push('|--------|------|------|');
  for (const p of PLAYERS) {
    lines.push(`| ${p} | ${comparison.win_rates.a[p]} | ${comparison.win_rates.b[p]} |`);
  }
  lines.push('');

  // Game length
  lines.push('## Game Length');
  lines.push(`- ${comparison.label_a}: ${comparison.game_length.a.mean.toFixed(1)} ply`);
  lines.push(`- ${comparison.label_b}: ${comparison.game_length.b.mean.toFixed(1)} ply`);
  lines.push(`- p-value: ${comparison.game_length.test.pValue.toFixed(4)}`);
  lines.push('');

  // Metric diffs
  if (Object.keys(comparison.metric_diffs).length > 0) {
    lines.push('## Behavioral Metrics');
    for (const [metric, data] of Object.entries(comparison.metric_diffs)) {
      const sig = data.test.significant ? ' *' : '';
      lines.push(`- ${metric}: A=${data.a.toFixed(4)}, B=${data.b.toFixed(4)}, p=${data.test.pValue.toFixed(4)}${sig}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}
