#!/usr/bin/env node
// ab_runner.mjs — A/B comparison runner for Project Freyja self-play
//
// Runs N games with config A, then N games with config B (or interleaved
// with SPRT), and produces a comparison report.
//
// Usage: node ab_runner.mjs <ab_config.json>
//
// Config format:
// {
//   "engine": "../target/release/freyja.exe",
//   "games_per_config": 50,
//   "max_ply": 400,
//   "sprt": { "elo0": 0, "elo1": 20, "alpha": 0.05, "beta": 0.05 },  // optional
//   "config_a": { "label": "depth_2", "depth": 2, "setoptions": {} },
//   "config_b": { "label": "depth_4", "depth": 4, "setoptions": {} },
//   "output_dir": "reports/ab_test_001"
// }
//
// Stage 12: Self-Play Framework

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';
import { compareConfigs, formatABReport } from './lib/ab.mjs';
import { SPRT } from './lib/sprt.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2];
if (!configPath) {
  console.error('Usage: node ab_runner.mjs <ab_config.json>');
  process.exit(1);
}

const config = JSON.parse(readFileSync(configPath, 'utf8'));

// ---------------------------------------------------------------------------
// Play one game with given settings — returns structured record
// ---------------------------------------------------------------------------
async function playGame(engine, gameNum, gameConfig) {
  const record = {
    game: gameNum,
    settings: {
      depth: gameConfig.depth ?? null,
      movetime: gameConfig.movetime ?? null,
      setoptions: gameConfig.setoptions ?? {},
    },
    plies: [],
    eliminations: [],
    game_result: null,
    total_ply: 0,
    total_rounds: 0,
    error: null,
  };

  const moveList = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let ply = 0;
  let lastFen4 = null;
  const maxPly = config.max_ply ?? 400;

  while (!gameOver && ply < maxPly) {
    const posCmd = lastFen4
      ? `position fen4 ${lastFen4}`
      : moveList.length === 0
        ? 'position startpos'
        : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);

    // Capture FEN4 — getFEN4 handles side-effect lines from position replay
    const fenResult = await engine.getFEN4();
    const fen4 = fenResult.fen4;
    for (const seLine of fenResult.sideEffects) {
      const se = parseLine(seLine);
      if (se.type === 'eliminated') {
        record.eliminations.push({ player: se.color, reason: se.reason, at_ply: ply });
      } else if (se.type === 'nextturn') {
        currentPlayer = se.player;
      } else if (se.type === 'info_string') {
        if (seLine.includes('game is over') || seLine.includes('no legal moves')) {
          gameOver = true;
        }
      }
    }
    if (gameOver) break;

    // Search
    const goCmd = gameConfig.movetime
      ? `go movetime ${gameConfig.movetime}`
      : `go depth ${gameConfig.depth}`;
    engine.send(goCmd);

    let bestmove = null;
    let lastSearch = null;

    while (true) {
      const line = await engine.readLine();
      if (line === null) {
        record.error = `Engine died at ply ${ply}`;
        gameOver = true;
        break;
      }
      const p = parseLine(line);

      if (p.type === 'eliminated') {
        record.eliminations.push({ player: p.color, reason: p.reason, at_ply: ply });
      } else if (p.type === 'nextturn') {
        currentPlayer = p.player;
      } else if (p.type === 'search_info') {
        lastSearch = p;
      } else if (p.type === 'bestmove') {
        bestmove = p.move;
        break;
      } else if (p.type === 'info_string') {
        if (p.raw.includes('game is over') || p.raw.includes('no legal moves')) {
          gameOver = true;
          break;
        }
        if (p.raw.includes('error:') || p.raw.includes('game stopped')) {
          record.error = p.raw;
          gameOver = true;
          break;
        }
      }
    }

    if (bestmove === null && !gameOver) {
      gameOver = true;
      break;
    }

    if (bestmove) {
      record.plies.push({
        ply,
        round: Math.floor(ply / 4),
        player: currentPlayer,
        move: bestmove,
        fen4,
        scores: lastSearch ? {
          red: lastSearch.red, blue: lastSearch.blue,
          yellow: lastSearch.yellow, green: lastSearch.green,
        } : null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        nps: lastSearch?.nps ?? null,
        pv: lastSearch?.pv ?? null,
      });

      moveList.push(bestmove);
      ply++;

      // Capture post-move FEN4 for next iteration
      if (fen4) {
        engine.send(`position fen4 ${fen4} moves ${bestmove}`);
        const postFenResult = await engine.getFEN4();
        lastFen4 = postFenResult.fen4;
        for (const seLine of postFenResult.sideEffects) {
          const se = parseLine(seLine);
          if (se.type === 'eliminated') {
            record.eliminations.push({ player: se.color, reason: se.reason, at_ply: ply });
          } else if (se.type === 'nextturn') {
            currentPlayer = se.player;
          } else if (se.type === 'info_string') {
            if (seLine.includes('game is over') || seLine.includes('no legal moves')) {
              gameOver = true;
            }
          }
        }
      }

      const eliminated = new Set(record.eliminations.map((e) => e.player));
      let next = PLAYERS[(PLAYERS.indexOf(currentPlayer) + 1) % 4];
      for (let i = 0; i < 3; i++) {
        if (!eliminated.has(next)) break;
        next = PLAYERS[(PLAYERS.indexOf(next) + 1) % 4];
      }
      currentPlayer = next;
    }

    if (gameOver) break;
  }

  record.total_ply = ply;
  record.total_rounds = Math.floor(ply / 4);

  // Compute game result
  const lastPly = record.plies[record.plies.length - 1];
  const scores = lastPly?.scores;
  if (scores) {
    const eliminated = new Set(record.eliminations.map((e) => e.player));
    const alive = PLAYERS.filter((p) => !eliminated.has(p));
    let reason = alive.length === 1 ? 'last_standing' : 'max_ply';
    let winner = null;
    let best = -Infinity;
    for (const p of PLAYERS) {
      const key = p.toLowerCase();
      if (scores[key] !== undefined && scores[key] > best) {
        best = scores[key];
        winner = p;
      }
    }
    record.game_result = { winner, final_scores: { ...scores }, reason };
  } else {
    record.game_result = { winner: null, final_scores: null, reason: 'unknown' };
  }

  return record;
}

// ---------------------------------------------------------------------------
// Run N games with a given config
// ---------------------------------------------------------------------------
async function runGames(gameConfig, numGames, label) {
  const enginePath = resolve(__dirname, config.engine);
  const engine = new Engine(enginePath);
  await engine.handshake();

  if (gameConfig.setoptions && Object.keys(gameConfig.setoptions).length > 0) {
    await engine.sendOptions(gameConfig.setoptions);
  }

  const games = [];
  for (let i = 1; i <= numGames; i++) {
    engine.send('position startpos');
    engine.send('isready');
    await engine.readUntil('readyok');

    process.stdout.write(`[${label}] Game ${i}/${numGames} ... `);
    const record = await playGame(engine, i, gameConfig);
    games.push(record);

    const winner = record.game_result?.winner ?? '?';
    console.log(`${record.total_ply} ply, winner: ${winner}${record.error ? ' [ERR]' : ''}`);
  }

  engine.close();
  return games;
}

// ---------------------------------------------------------------------------
// Run interleaved games with SPRT early stopping
// ---------------------------------------------------------------------------
async function runWithSPRT(configA, configB, maxGames, sprtConfig) {
  const sprt = new SPRT(sprtConfig);
  const gamesA = [];
  const gamesB = [];

  console.log(`SPRT: elo0=${sprtConfig.elo0}, elo1=${sprtConfig.elo1}, alpha=${sprtConfig.alpha}, beta=${sprtConfig.beta}`);
  console.log('');

  for (let i = 0; i < maxGames; i++) {
    // Run one game with config A
    const enginePath = resolve(__dirname, config.engine);
    let engine = new Engine(enginePath);
    await engine.handshake();
    if (configA.setoptions) await engine.sendOptions(configA.setoptions);
    engine.send('position startpos');
    engine.send('isready');
    await engine.readUntil('readyok');

    process.stdout.write(`[A:${configA.label}] Game ${i + 1} ... `);
    const gameA = await playGame(engine, i + 1, configA);
    gamesA.push(gameA);
    console.log(`${gameA.total_ply} ply`);
    engine.close();

    // Run one game with config B
    engine = new Engine(enginePath);
    await engine.handshake();
    if (configB.setoptions) await engine.sendOptions(configB.setoptions);
    engine.send('position startpos');
    engine.send('isready');
    await engine.readUntil('readyok');

    process.stdout.write(`[B:${configB.label}] Game ${i + 1} ... `);
    const gameB = await playGame(engine, i + 1, configB);
    gamesB.push(gameB);
    console.log(`${gameB.total_ply} ply`);
    engine.close();

    // Compute average score for this pair
    const scoreA = avgGameScore(gameA);
    const scoreB = avgGameScore(gameB);
    const decision = sprt.update(scoreA, scoreB);

    console.log(`  SPRT: LLR=${sprt.llr.toFixed(3)}, bounds=[${sprt.lowerBound.toFixed(3)}, ${sprt.upperBound.toFixed(3)}], pairs=${sprt.gamesPlayed}`);

    if (decision !== 'continue') {
      console.log(`\nSPRT decision: ${decision} after ${sprt.gamesPlayed} pairs`);
      break;
    }
  }

  return { gamesA, gamesB, sprt };
}

function avgGameScore(game) {
  const s = game.game_result?.final_scores;
  if (!s) return 0;
  return ((s.red ?? 0) + (s.blue ?? 0) + (s.yellow ?? 0) + (s.green ?? 0)) / 4;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const outputDir = resolve(__dirname, config.output_dir ?? 'reports/ab_test');
  if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true });

  const configA = config.config_a;
  const configB = config.config_b;
  const numGames = config.games_per_config ?? 50;

  console.log(`A/B Comparison: ${configA.label} vs ${configB.label}`);
  console.log(`Games per config: ${numGames}`);
  console.log(`Engine: ${config.engine}`);
  console.log('');

  let gamesA, gamesB, sprtResult = null;

  if (config.sprt) {
    // Interleaved with SPRT early stopping
    const result = await runWithSPRT(configA, configB, numGames, config.sprt);
    gamesA = result.gamesA;
    gamesB = result.gamesB;
    sprtResult = {
      decision: result.sprt.gamesPlayed > 0 ? result.sprt.decision : 'inconclusive',
      llr: result.sprt.llr,
      lower_bound: result.sprt.lowerBound,
      upper_bound: result.sprt.upperBound,
      pairs_played: result.sprt.gamesPlayed,
    };
  } else {
    // Sequential: all A games, then all B games
    console.log(`--- Config A: ${configA.label} ---`);
    gamesA = await runGames(configA, numGames, configA.label);
    console.log('');

    console.log(`--- Config B: ${configB.label} ---`);
    gamesB = await runGames(configB, numGames, configB.label);
    console.log('');
  }

  // Save raw game data
  writeFileSync(join(outputDir, 'games_a.json'), JSON.stringify(gamesA, null, 2));
  writeFileSync(join(outputDir, 'games_b.json'), JSON.stringify(gamesB, null, 2));

  // Compare
  const comparison = compareConfigs(gamesA, gamesB, configA.label, configB.label);
  if (sprtResult) comparison.sprt = sprtResult;

  writeFileSync(join(outputDir, 'comparison.json'), JSON.stringify(comparison, null, 2));
  const report = formatABReport(comparison);
  writeFileSync(join(outputDir, 'comparison.md'), report);

  console.log('');
  console.log(report);
  console.log(`\nResults saved to ${outputDir}`);
}

main().catch((err) => {
  console.error('A/B runner error:', err);
  process.exit(1);
});
