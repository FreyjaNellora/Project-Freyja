// lib/stats.mjs — Multi-game statistical aggregation for self-play
//
// Aggregates metrics across N games:
//   - Win rate per seat
//   - Mean/stddev of game length, final scores, behavioral metrics
//   - 95% confidence intervals
//   - Chi-squared test for win rate uniformity
//
// Stage 12: Self-Play Framework

import { computeMetrics } from './metrics.mjs';

const PLAYERS = ['red', 'blue', 'yellow', 'green'];

// ---------------------------------------------------------------------------
// Statistical helpers
// ---------------------------------------------------------------------------

export function mean(arr) {
  if (arr.length === 0) return 0;
  return arr.reduce((a, b) => a + b, 0) / arr.length;
}

export function stddev(arr) {
  if (arr.length < 2) return 0;
  const m = mean(arr);
  const variance = arr.reduce((sum, x) => sum + (x - m) ** 2, 0) / (arr.length - 1);
  return Math.sqrt(variance);
}

export function confidenceInterval95(arr) {
  const m = mean(arr);
  const s = stddev(arr);
  const n = arr.length;
  if (n < 2) return { mean: m, lower: m, upper: m, stddev: s };
  const margin = 1.96 * s / Math.sqrt(n);
  return { mean: m, lower: m - margin, upper: m + margin, stddev: s };
}

/**
 * Chi-squared test for uniformity.
 * Tests if observed frequencies deviate significantly from expected uniform distribution.
 *
 * @param {number[]} observed - Observed counts per category
 * @param {number[]} expected - Expected counts per category
 * @returns {{ chi2: number, df: number, pValue: number, significant: boolean }}
 */
export function chiSquaredTest(observed, expected) {
  const df = observed.length - 1;
  let chi2 = 0;
  for (let i = 0; i < observed.length; i++) {
    if (expected[i] === 0) continue;
    chi2 += (observed[i] - expected[i]) ** 2 / expected[i];
  }
  // Approximate p-value using chi-squared CDF (Wilson-Hilferty approximation)
  const pValue = 1 - chi2CDF(chi2, df);
  return { chi2, df, pValue, significant: pValue < 0.05 };
}

/**
 * Two-sample t-test (Welch's t-test) for comparing means.
 *
 * @returns {{ t: number, df: number, pValue: number, significant: boolean }}
 */
export function tTest(arr1, arr2) {
  const n1 = arr1.length;
  const n2 = arr2.length;
  if (n1 < 2 || n2 < 2) return { t: 0, df: 0, pValue: 1, significant: false };

  const m1 = mean(arr1);
  const m2 = mean(arr2);
  const v1 = arr1.reduce((s, x) => s + (x - m1) ** 2, 0) / (n1 - 1);
  const v2 = arr2.reduce((s, x) => s + (x - m2) ** 2, 0) / (n2 - 1);
  const se = Math.sqrt(v1 / n1 + v2 / n2);
  if (se === 0) return { t: 0, df: n1 + n2 - 2, pValue: 1, significant: false };

  const t = (m1 - m2) / se;
  // Welch-Satterthwaite degrees of freedom
  const df = (v1 / n1 + v2 / n2) ** 2 /
    ((v1 / n1) ** 2 / (n1 - 1) + (v2 / n2) ** 2 / (n2 - 1));
  // Approximate p-value using t-distribution (two-tailed)
  const pValue = 2 * (1 - tCDF(Math.abs(t), df));
  return { t, df, pValue, significant: pValue < 0.05 };
}

// ---------------------------------------------------------------------------
// Approximate CDF functions
// ---------------------------------------------------------------------------

// Chi-squared CDF approximation using regularized incomplete gamma function
function chi2CDF(x, k) {
  if (x <= 0) return 0;
  return regGammaP(k / 2, x / 2);
}

// Regularized lower incomplete gamma function P(a, x)
// Using series expansion for P(a, x)
function regGammaP(a, x) {
  if (x < 0) return 0;
  if (x === 0) return 0;
  if (x < a + 1) {
    // Series expansion
    let sum = 1 / a;
    let term = 1 / a;
    for (let n = 1; n < 200; n++) {
      term *= x / (a + n);
      sum += term;
      if (Math.abs(term) < 1e-10 * Math.abs(sum)) break;
    }
    return sum * Math.exp(-x + a * Math.log(x) - logGamma(a));
  } else {
    // Continued fraction (complement)
    return 1 - regGammaQ(a, x);
  }
}

function regGammaQ(a, x) {
  // Continued fraction representation
  let f = 1 + x - a;
  let c = 1 / 1e-30;
  let d = 1 / f;
  let h = d;
  for (let i = 1; i < 200; i++) {
    const an = -i * (i - a);
    const bn = 2 * i + 1 + x - a;
    d = bn + an * d;
    if (Math.abs(d) < 1e-30) d = 1e-30;
    c = bn + an / c;
    if (Math.abs(c) < 1e-30) c = 1e-30;
    d = 1 / d;
    const delta = d * c;
    h *= delta;
    if (Math.abs(delta - 1) < 1e-10) break;
  }
  return Math.exp(-x + a * Math.log(x) - logGamma(a)) * h;
}

function logGamma(x) {
  // Stirling's approximation with Lanczos coefficients
  const g = 7;
  const c = [
    0.99999999999980993, 676.5203681218851, -1259.1392167224028,
    771.32342877765313, -176.61502916214059, 12.507343278686905,
    -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7,
  ];
  if (x < 0.5) {
    return Math.log(Math.PI / Math.sin(Math.PI * x)) - logGamma(1 - x);
  }
  x -= 1;
  let a = c[0];
  for (let i = 1; i < g + 2; i++) {
    a += c[i] / (x + i);
  }
  const t = x + g + 0.5;
  return 0.5 * Math.log(2 * Math.PI) + (x + 0.5) * Math.log(t) - t + Math.log(a);
}

// Student's t-distribution CDF approximation
function tCDF(t, df) {
  if (df <= 0) return 0.5;
  const x = df / (df + t * t);
  return 1 - 0.5 * regBetaI(df / 2, 0.5, x);
}

// Regularized incomplete beta function I_x(a, b) using continued fraction
function regBetaI(a, b, x) {
  if (x <= 0) return 0;
  if (x >= 1) return 1;

  const lnBeta = logGamma(a) + logGamma(b) - logGamma(a + b);
  const prefix = Math.exp(a * Math.log(x) + b * Math.log(1 - x) - lnBeta);

  // Use continued fraction
  if (x < (a + 1) / (a + b + 2)) {
    return prefix * betaCF(a, b, x) / a;
  } else {
    return 1 - prefix * betaCF(b, a, 1 - x) / b;
  }
}

function betaCF(a, b, x) {
  let m, m2, d, h, aa, del;
  const qab = a + b;
  const qap = a + 1;
  const qam = a - 1;
  let c = 1;
  d = 1 - qab * x / qap;
  if (Math.abs(d) < 1e-30) d = 1e-30;
  d = 1 / d;
  h = d;
  for (m = 1; m <= 200; m++) {
    m2 = 2 * m;
    aa = m * (b - m) * x / ((qam + m2) * (a + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < 1e-30) d = 1e-30;
    c = 1 + aa / c;
    if (Math.abs(c) < 1e-30) c = 1e-30;
    d = 1 / d;
    h *= d * c;
    aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < 1e-30) d = 1e-30;
    c = 1 + aa / c;
    if (Math.abs(c) < 1e-30) c = 1e-30;
    d = 1 / d;
    del = d * c;
    h *= del;
    if (Math.abs(del - 1) < 1e-10) break;
  }
  return h;
}

// ---------------------------------------------------------------------------
// Aggregate stats across multiple games
// ---------------------------------------------------------------------------

/**
 * Compute aggregate statistics from an array of game records.
 *
 * @param {Object[]} games - Array of game records from observer.mjs
 * @returns {Object} Aggregated statistics
 */
export function aggregateStats(games) {
  if (games.length === 0) return null;

  // Win rate per player (seat)
  const wins = { red: 0, blue: 0, yellow: 0, green: 0 };
  const finalScores = { red: [], blue: [], yellow: [], green: [] };
  const gameLengths = [];
  const gameRounds = [];

  for (const g of games) {
    gameLengths.push(g.total_ply);
    gameRounds.push(g.total_rounds);

    if (g.game_result?.winner) {
      const w = g.game_result.winner.toLowerCase();
      if (wins[w] !== undefined) wins[w]++;
    }
    if (g.game_result?.final_scores) {
      for (const p of PLAYERS) {
        const score = g.game_result.final_scores[p];
        if (score !== undefined) finalScores[p].push(score);
      }
    }
  }

  // Win rate uniformity test
  const totalGames = games.length;
  const observed = PLAYERS.map((p) => wins[p]);
  const expected = PLAYERS.map(() => totalGames / 4);
  const winRateTest = chiSquaredTest(observed, expected);

  // Per-player score stats
  const scoreStats = {};
  for (const p of PLAYERS) {
    scoreStats[p] = confidenceInterval95(finalScores[p]);
  }

  // Compute per-game metrics and aggregate
  const allMetrics = games.map((g) => computeMetrics(g)).filter((m) => m !== null);

  const metricAggregates = {};
  if (allMetrics.length > 0) {
    // Pawn ratio
    metricAggregates.pawn_ratio = {};
    for (const p of PLAYERS) {
      const vals = allMetrics.map((m) => m.pawn_ratio[p]).filter((v) => v !== null);
      metricAggregates.pawn_ratio[p] = confidenceInterval95(vals);
    }
    const avgVals = allMetrics.map((m) => m.pawn_ratio.avg).filter((v) => v !== null);
    metricAggregates.pawn_ratio.avg = confidenceInterval95(avgVals);

    // Queen activation
    metricAggregates.queen_activation_round = {};
    for (const p of PLAYERS) {
      const vals = allMetrics.map((m) => m.queen_activation_round[p]).filter((v) => v !== null);
      metricAggregates.queen_activation_round[p] = confidenceInterval95(vals);
    }

    // Shuffle index
    const shuffleVals = allMetrics.map((m) => m.shuffle_index);
    metricAggregates.shuffle_index = confidenceInterval95(shuffleVals);

    // Score delta
    const deltaVals = allMetrics.map((m) => m.avg_score_delta_per_round);
    metricAggregates.avg_score_delta_per_round = confidenceInterval95(deltaVals);

    // King moves
    metricAggregates.king_moves = {};
    for (const p of PLAYERS) {
      const vals = allMetrics.map((m) => m.king_moves[p]);
      metricAggregates.king_moves[p] = confidenceInterval95(vals);
    }
  }

  return {
    total_games: totalGames,
    win_rates: {
      counts: wins,
      rates: Object.fromEntries(PLAYERS.map((p) => [p, wins[p] / totalGames])),
      uniformity_test: winRateTest,
    },
    game_length: {
      ply: confidenceInterval95(gameLengths),
      rounds: confidenceInterval95(gameRounds),
    },
    final_scores: scoreStats,
    metrics: metricAggregates,
    per_game_metrics: allMetrics,
  };
}

/**
 * Format a human-readable stats report.
 */
export function formatStatsReport(stats, numGames) {
  const lines = [];
  lines.push('# Self-Play Statistics Report');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push(`Games: ${numGames}`);
  lines.push('');

  // Win rates
  lines.push('## Win Rates');
  lines.push('| Player | Wins | Rate |');
  lines.push('|--------|------|------|');
  for (const p of PLAYERS) {
    const rate = (stats.win_rates.rates[p] * 100).toFixed(1);
    lines.push(`| ${p} | ${stats.win_rates.counts[p]} | ${rate}% |`);
  }
  lines.push('');
  const ut = stats.win_rates.uniformity_test;
  lines.push(`Chi-squared: ${ut.chi2.toFixed(2)}, df=${ut.df}, p=${ut.pValue.toFixed(4)} → ${ut.significant ? 'NON-UNIFORM' : 'uniform (expected)'}`);
  lines.push('');

  // Game length
  lines.push('## Game Length');
  const gl = stats.game_length;
  lines.push(`Ply: ${gl.ply.mean.toFixed(1)} +/- ${gl.ply.stddev.toFixed(1)} (95% CI: ${gl.ply.lower.toFixed(1)} - ${gl.ply.upper.toFixed(1)})`);
  lines.push(`Rounds: ${gl.rounds.mean.toFixed(1)} +/- ${gl.rounds.stddev.toFixed(1)}`);
  lines.push('');

  // Final scores
  lines.push('## Final Scores');
  lines.push('| Player | Mean | Stddev | 95% CI |');
  lines.push('|--------|------|--------|--------|');
  for (const p of PLAYERS) {
    const s = stats.final_scores[p];
    lines.push(`| ${p} | ${s.mean.toFixed(1)} | ${s.stddev.toFixed(1)} | ${s.lower.toFixed(1)} - ${s.upper.toFixed(1)} |`);
  }
  lines.push('');

  // Behavioral metrics
  if (stats.metrics.pawn_ratio) {
    lines.push('## Behavioral Metrics');
    lines.push('');
    lines.push('### Pawn Ratio (end/start)');
    for (const p of PLAYERS) {
      const v = stats.metrics.pawn_ratio[p];
      lines.push(`- ${p}: ${v.mean.toFixed(3)} +/- ${v.stddev.toFixed(3)}`);
    }
    lines.push(`- avg: ${stats.metrics.pawn_ratio.avg.mean.toFixed(3)} +/- ${stats.metrics.pawn_ratio.avg.stddev.toFixed(3)}`);
    lines.push('');

    lines.push('### Shuffle Index');
    const si = stats.metrics.shuffle_index;
    lines.push(`${si.mean.toFixed(4)} +/- ${si.stddev.toFixed(4)}`);
    lines.push('');
  }

  return lines.join('\n');
}
