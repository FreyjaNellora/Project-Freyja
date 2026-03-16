#!/usr/bin/env node
// extract_training.mjs — Training data extraction from self-play games
//
// Filters game JSON to produce NNUE training data in JSONL format.
//
// Usage:
//   node extract_training.mjs <games_json> [options]
//
// Options:
//   --min-depth N    Only include positions searched at depth >= N (default: 1)
//   --min-ply N      Skip first N plies (default: 8)
//   --max-ply N      Skip plies beyond N (default: unlimited)
//   --score-range LO HI  Only include positions with all scores in [LO, HI]
//   --no-dedup       Don't deduplicate by FEN4
//   --output FILE    Output file (default: training.jsonl)
//
// Stage 12: Self-Play Framework

import { readFileSync, writeFileSync } from 'node:fs';
import { extractTrainingData, validateTrainingData, toJSONL } from './lib/training_data.mjs';

// Parse CLI arguments
const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: node extract_training.mjs <games_json> [--min-depth N] [--min-ply N] [--output FILE]');
  process.exit(1);
}

const gamesPath = args[0];
const options = {
  minDepth: 1,
  minPly: 8,
  maxPly: Infinity,
  scoreRange: null,
  deduplicate: true,
};
let outputPath = 'training.jsonl';

for (let i = 1; i < args.length; i++) {
  switch (args[i]) {
    case '--min-depth': options.minDepth = parseInt(args[++i], 10); break;
    case '--min-ply': options.minPly = parseInt(args[++i], 10); break;
    case '--max-ply': options.maxPly = parseInt(args[++i], 10); break;
    case '--score-range':
      options.scoreRange = [parseInt(args[++i], 10), parseInt(args[++i], 10)];
      break;
    case '--no-dedup': options.deduplicate = false; break;
    case '--output': outputPath = args[++i]; break;
    default:
      console.error(`Unknown option: ${args[i]}`);
      process.exit(1);
  }
}

// Load games
console.log(`Loading games from: ${gamesPath}`);
const data = JSON.parse(readFileSync(gamesPath, 'utf8'));
const games = Array.isArray(data) ? data : [data];
console.log(`Loaded ${games.length} games`);

// Extract training data
console.log(`Filters: minDepth=${options.minDepth}, minPly=${options.minPly}, maxPly=${options.maxPly}, dedup=${options.deduplicate}`);
const records = extractTrainingData(games, options);
console.log(`Extracted ${records.length} training records`);

// Validate
const validation = validateTrainingData(records);
console.log(`Validation: ${validation.valid} valid, ${validation.invalid} invalid`);
if (validation.errors.length > 0) {
  console.error('Validation errors:');
  for (const err of validation.errors.slice(0, 10)) {
    console.error(`  ${err}`);
  }
  if (validation.errors.length > 10) {
    console.error(`  ... and ${validation.errors.length - 10} more`);
  }
}

// Write JSONL
const jsonl = toJSONL(records);
writeFileSync(outputPath, jsonl);
console.log(`Written ${records.length} records to ${outputPath}`);

// Summary statistics
if (records.length > 0) {
  const depths = records.map((r) => r.depth).filter((d) => d !== null);
  const plies = records.map((r) => r.ply);
  console.log('');
  console.log('Summary:');
  console.log(`  Depth range: ${Math.min(...depths)} - ${Math.max(...depths)}`);
  console.log(`  Ply range: ${Math.min(...plies)} - ${Math.max(...plies)}`);
  const players = {};
  for (const r of records) {
    players[r.player] = (players[r.player] || 0) + 1;
  }
  console.log(`  Per player: ${Object.entries(players).map(([k, v]) => `${k}=${v}`).join(', ')}`);
}
