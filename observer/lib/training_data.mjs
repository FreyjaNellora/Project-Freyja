// lib/training_data.mjs — Training data extraction from game JSON
//
// Extracts training records from self-play game records. Each record contains:
//   - FEN4 position (before the move)
//   - Eval 4-vector (engine's evaluation at search time)
//   - Best move chosen
//   - Game result (who won, final scores)
//   - Search depth
//
// This is a VIEW of observer game data, not a separate export format.
// Output: JSONL (one JSON record per line) for streaming/appending.
//
// Stage 12: Self-Play Framework

import { parseFEN4 } from './fen4_parser.mjs';

/**
 * Extract training records from an array of game records.
 *
 * @param {Object[]} games - Array of game records from observer
 * @param {Object} options - Filter options
 * @param {number} [options.minDepth=1] - Minimum search depth to include
 * @param {number} [options.minPly=0] - Skip first N plies (opening noise)
 * @param {number} [options.maxPly=Infinity] - Skip plies beyond this
 * @param {number[]} [options.scoreRange] - [min, max] for all scores (skip extremes)
 * @param {boolean} [options.deduplicate=true] - Remove duplicate FEN4 positions
 * @returns {Object[]} Array of training records
 */
export function extractTrainingData(games, options = {}) {
  const {
    minDepth = 1,
    minPly = 0,
    maxPly = Infinity,
    scoreRange = null,
    deduplicate = true,
  } = options;

  const records = [];
  const seen = deduplicate ? new Set() : null;

  for (const game of games) {
    const gameResult = game.game_result ?? null;

    for (const ply of game.plies) {
      // Apply filters
      if (ply.ply < minPly) continue;
      if (ply.ply > maxPly) continue;
      if (ply.depth !== null && ply.depth < minDepth) continue;
      if (!ply.fen4) continue;
      if (!ply.scores) continue;

      // Score range filter
      if (scoreRange) {
        const [lo, hi] = scoreRange;
        const scores = ply.scores;
        if (scores.red < lo || scores.red > hi ||
            scores.blue < lo || scores.blue > hi ||
            scores.yellow < lo || scores.yellow > hi ||
            scores.green < lo || scores.green > hi) {
          continue;
        }
      }

      // Deduplicate by FEN4 (board portion only, first space-separated field)
      if (deduplicate) {
        const boardKey = ply.fen4.split(/\s+/)[0];
        if (seen.has(boardKey)) continue;
        seen.add(boardKey);
      }

      records.push({
        fen4: ply.fen4,
        eval_4vec: [ply.scores.red, ply.scores.blue, ply.scores.yellow, ply.scores.green],
        best_move: ply.move,
        player: ply.player.toLowerCase(),
        ply: ply.ply,
        round: ply.round,
        depth: ply.depth,
        game_result: gameResult ? {
          winner: gameResult.winner?.toLowerCase() ?? null,
          final_scores: gameResult.final_scores,
          reason: gameResult.reason,
        } : null,
      });
    }
  }

  return records;
}

/**
 * Validate extracted training records.
 *
 * @param {Object[]} records - Training records to validate
 * @returns {{ valid: number, invalid: number, errors: string[] }}
 */
export function validateTrainingData(records) {
  let valid = 0;
  let invalid = 0;
  const errors = [];

  for (let i = 0; i < records.length; i++) {
    const r = records[i];
    const errs = [];

    // Check FEN4 parseable
    if (!r.fen4 || typeof r.fen4 !== 'string') {
      errs.push('missing or invalid fen4');
    } else {
      try {
        const parsed = parseFEN4(r.fen4);
        const total = parsed.counts.red + parsed.counts.blue +
          parsed.counts.yellow + parsed.counts.green;
        if (total === 0) errs.push('FEN4 parsed but no pieces found');
      } catch (e) {
        errs.push(`FEN4 parse error: ${e.message}`);
      }
    }

    // Check eval vector
    if (!Array.isArray(r.eval_4vec) || r.eval_4vec.length !== 4) {
      errs.push('eval_4vec must be array of 4 numbers');
    } else if (r.eval_4vec.some((v) => typeof v !== 'number' || isNaN(v))) {
      errs.push('eval_4vec contains non-numeric values');
    }

    // Check move
    if (!r.best_move || typeof r.best_move !== 'string') {
      errs.push('missing best_move');
    }

    if (errs.length > 0) {
      invalid++;
      errors.push(`Record ${i}: ${errs.join('; ')}`);
    } else {
      valid++;
    }
  }

  return { valid, invalid, errors };
}

/**
 * Convert training records to JSONL format (one JSON per line).
 */
export function toJSONL(records) {
  return records.map((r) => JSON.stringify(r)).join('\n') + '\n';
}
