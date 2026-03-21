#!/usr/bin/env node
// run_training_batches.mjs — Run N batches of self-play games for NNUE training data.
//
// Each batch runs 100 games at depth 4 with MoveNoise=40, then extracts
// training data from the batch. Batches run sequentially to stay within
// 8GB RAM limits.
//
// Usage:
//   node run_training_batches.mjs [--batches N] [--engine PATH] [--start-batch N]
//
// Defaults: 5 batches (500 games), engine from config.
// Use --start-batch to resume after a partial run (e.g., --start-batch 3 skips batches 1-2).
//
// Output:
//   reports/training_d4_batch_001/ ... reports/training_d4_batch_005/
//   freyja-nnue/training_d4_batch_001.jsonl ... (per-batch JSONL)
//   freyja-nnue/training_d4_all.jsonl (merged, deduplicated)
//
// Stage 17: NNUE Integration — Training Data Generation

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');

// ─── CLI args ────────────────────────────────────────────────────────────────
let numBatches = 5;
let enginePath = 'C:/rust-target/freyja/release/freyja.exe';
let startBatch = 1;
const gamesPerBatch = 100;
const depth = 4;
const maxPly = 80;
const moveNoise = 40;

const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  switch (args[i]) {
    case '--batches': numBatches = parseInt(args[++i], 10); break;
    case '--engine': enginePath = args[++i]; break;
    case '--start-batch': startBatch = parseInt(args[++i], 10); break;
    default:
      console.error(`Unknown option: ${args[i]}`);
      console.error('Usage: node run_training_batches.mjs [--batches N] [--engine PATH] [--start-batch N]');
      process.exit(1);
  }
}

const totalGames = numBatches * gamesPerBatch;
console.log(`\n=== Training Data Generation ===`);
console.log(`Batches: ${numBatches} (${gamesPerBatch} games each = ${totalGames} total)`);
console.log(`Depth: ${depth} | MaxPly: ${maxPly} | MoveNoise: ${moveNoise}`);
console.log(`Engine: ${enginePath}`);
console.log(`Start batch: ${startBatch}`);
console.log(`Estimated time: ~${Math.round(totalGames * 3.5 / 60)} hours (3.5 min/game at depth 4)\n`);

// ─── Run batches ─────────────────────────────────────────────────────────────
const batchJSONLs = [];

for (let b = startBatch; b <= numBatches; b++) {
  const batchLabel = String(b).padStart(3, '0');
  const reportDir = `reports/training_d4_batch_${batchLabel}`;
  const batchConfig = {
    engine: enginePath,
    games: gamesPerBatch,
    depth,
    max_ply: maxPly,
    setoptions: { MoveNoise: String(moveNoise) },
    output_dir: reportDir,
  };

  // Write temporary config
  const configPath = join(__dirname, `_temp_batch_${batchLabel}.json`);
  writeFileSync(configPath, JSON.stringify(batchConfig, null, 2));

  console.log(`\n--- Batch ${b}/${numBatches} (${gamesPerBatch} games) ---`);
  const batchStart = Date.now();

  try {
    execSync(`node "${join(__dirname, 'observer.mjs')}" "${configPath}"`, {
      stdio: 'inherit',
      cwd: __dirname,
      timeout: 3600000, // 1 hour max per batch
    });
  } catch (err) {
    console.error(`Batch ${b} failed: ${err.message}`);
    console.error('You can resume with: node run_training_batches.mjs --start-batch ' + b);
    process.exit(1);
  }

  const batchElapsed = ((Date.now() - batchStart) / 1000 / 60).toFixed(1);
  console.log(`Batch ${b} complete in ${batchElapsed} minutes.`);

  // Extract training data from this batch
  const allGamesPath = join(__dirname, reportDir, 'all_games.json');
  const jsonlPath = join(projectRoot, 'freyja-nnue', `training_d4_batch_${batchLabel}.jsonl`);

  if (existsSync(allGamesPath)) {
    try {
      execSync(
        `node "${join(__dirname, 'extract_training.mjs')}" "${allGamesPath}" --min-depth 3 --min-ply 8 --output "${jsonlPath}"`,
        { stdio: 'inherit', cwd: __dirname }
      );
      batchJSONLs.push(jsonlPath);
    } catch (err) {
      console.error(`Training extraction failed for batch ${b}: ${err.message}`);
    }
  }

  // Clean up temp config
  try { execSync(`rm "${configPath}"`, { stdio: 'ignore' }); } catch (_) {}
}

// ─── Merge all batch JSONLs ──────────────────────────────────────────────────
console.log(`\n--- Merging ${batchJSONLs.length} batch files ---`);
const mergedPath = join(projectRoot, 'freyja-nnue', 'training_d4_all.jsonl');
const seenFens = new Set();
let totalRecords = 0;
let dedupRecords = 0;
let mergedLines = [];

for (const jsonlPath of batchJSONLs) {
  if (!existsSync(jsonlPath)) continue;
  const lines = readFileSync(jsonlPath, 'utf8').trim().split('\n').filter(Boolean);
  for (const line of lines) {
    totalRecords++;
    try {
      const rec = JSON.parse(line);
      if (rec.fen4 && !seenFens.has(rec.fen4)) {
        seenFens.add(rec.fen4);
        mergedLines.push(line);
        dedupRecords++;
      }
    } catch (_) {
      mergedLines.push(line);
      dedupRecords++;
    }
  }
}

writeFileSync(mergedPath, mergedLines.join('\n') + '\n');
console.log(`Merged: ${totalRecords} total → ${dedupRecords} unique records`);
console.log(`Output: ${mergedPath}`);

// ─── Summary ─────────────────────────────────────────────────────────────────
console.log(`\n=== Training Data Generation Complete ===`);
console.log(`Total games: ${(numBatches - startBatch + 1) * gamesPerBatch}`);
console.log(`Unique training positions: ${dedupRecords}`);
console.log(`Output file: ${mergedPath}`);
console.log(`\nNext step: retrain NNUE with:`);
console.log(`  python -m freyja_nnue.train --data "${mergedPath}" --output weights_v2.fnnue --epochs 200 --batch-size 512`);
