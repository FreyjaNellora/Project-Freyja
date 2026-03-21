#!/usr/bin/env node
// duel_runner.mjs — Head-to-head duel runner for Project Freyja
//
// Runs games where two engine configurations control different players:
//   Team A (config_a): Red + Yellow
//   Team B (config_b): Blue + Green
//
// Each team uses a separate engine instance with its own setoptions.
// Games are played in pairs (swap teams between games for fairness).
//
// Usage: node duel_runner.mjs <duel_config.json>
//
// Config format:
// {
//   "engine": "C:/rust-target/freyja/release/freyja.exe",
//   "game_pairs": 5,         // 5 pairs = 10 games total (swap colors each pair)
//   "max_ply": 160,
//   "config_a": { "label": "swarm_ray", "movetime": 2000, "setoptions": { ... } },
//   "config_b": { "label": "ray_only",  "movetime": 2000, "setoptions": { ... } },
//   "output_dir": "reports/duel_swarm_vs_ray"
// }

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2];
if (!configPath) {
  console.error('Usage: node duel_runner.mjs <duel_config.json>');
  process.exit(1);
}

const config = JSON.parse(readFileSync(configPath, 'utf8'));
const outDir = resolve(__dirname, config.output_dir);
if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

// All 3 distinct team pairings for 4 players (seating arrangements).
// Each pair assigns 2 players to team A and 2 to team B.
const TEAM_PAIRINGS = [
  { a: ['Red', 'Yellow'], b: ['Blue', 'Green'] },   // Opposite corners
  { a: ['Red', 'Blue'],   b: ['Yellow', 'Green'] },  // Adjacent (R+B vs Y+G)
  { a: ['Red', 'Green'],  b: ['Blue', 'Yellow'] },   // Adjacent (R+G vs B+Y)
];

// Get team assignment for a player given a pairing index
function getTeamConfig(playerName, pairingIdx) {
  const pairing = TEAM_PAIRINGS[pairingIdx % TEAM_PAIRINGS.length];
  return pairing.a.includes(playerName) ? 'a' : 'b';
}

// Get the pairing for display
function getPairing(pairingIdx) {
  return TEAM_PAIRINGS[pairingIdx % TEAM_PAIRINGS.length];
}

// ---------------------------------------------------------------------------
// Play one duel game with two engine instances
// ---------------------------------------------------------------------------
async function playDuelGame(gameNum, pairingIdx) {
  const engineA = new Engine(resolve(__dirname, config.engine));
  const engineB = new Engine(resolve(__dirname, config.engine));

  await engineA.handshake();
  await engineB.handshake();

  // Apply setoptions to each engine
  if (config.config_a.setoptions) await engineA.sendOptions(config.config_a.setoptions);
  if (config.config_b.setoptions) await engineB.sendOptions(config.config_b.setoptions);

  // Set unique NoiseSeed per game so MoveNoise produces different games
  const noiseSeed = String(gameNum * 7919 + 42);
  await engineA.sendOptions({ NoiseSeed: noiseSeed });
  await engineB.sendOptions({ NoiseSeed: noiseSeed });

  const pairing = getPairing(pairingIdx);
  const record = {
    game: gameNum,
    pairing: pairingIdx,
    team_a: { label: config.config_a.label, players: pairing.a },
    team_b: { label: config.config_b.label, players: pairing.b },
    plies: [],
    eliminations: [],
    scores: null,
    error: null,
  };

  const moveList = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let ply = 0;
  const maxPly = config.max_ply ?? 160;

  // Track eliminated players
  const eliminated = new Set();

  while (!gameOver && ply < maxPly) {
    // Determine which engine handles this ply
    const team = getTeamConfig(currentPlayer, pairingIdx);
    const engine = team === 'a' ? engineA : engineB;
    const gameConfig = team === 'a' ? config.config_a : config.config_b;

    // Both engines need to know the position
    const posCmd = moveList.length === 0
      ? 'position startpos'
      : `position startpos moves ${moveList.join(' ')}`;

    engine.send(posCmd);

    // Drain side effects from position replay
    const sideEffects = await engine.drainUntilReady();
    for (const line of sideEffects) {
      const p = parseLine(line);
      if (p.type === 'eliminated') {
        record.eliminations.push({ player: p.color, reason: p.reason, at_ply: ply });
        eliminated.add(p.color);
      } else if (p.type === 'nextturn') {
        currentPlayer = p.player;
      } else if (p.type === 'info_string') {
        if (line.includes('game is over') || line.includes('no legal moves')) {
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
        record.error = `Engine died at ply ${ply} (team ${team})`;
        gameOver = true;
        break;
      }
      const p = parseLine(line);

      if (p.type === 'eliminated') {
        record.eliminations.push({ player: p.color, reason: p.reason, at_ply: ply });
        eliminated.add(p.color);
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
        player: currentPlayer,
        team,
        move: bestmove,
        scores: lastSearch ? {
          red: lastSearch.red, blue: lastSearch.blue,
          yellow: lastSearch.yellow, green: lastSearch.green,
        } : null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        nps: lastSearch?.nps ?? null,
      });

      moveList.push(bestmove);
      ply++;

      // Advance to next active player
      let nextIdx = (PLAYERS.indexOf(currentPlayer) + 1) % 4;
      for (let i = 0; i < 3; i++) {
        if (!eliminated.has(PLAYERS[nextIdx])) break;
        nextIdx = (nextIdx + 1) % 4;
      }
      currentPlayer = PLAYERS[nextIdx];
    }
  }

  // Capture final scores from last search info
  if (record.plies.length > 0) {
    const last = record.plies[record.plies.length - 1];
    record.scores = last.scores;
  }

  record.total_ply = ply;

  // Clean up engines
  engineA.send('quit');
  engineB.send('quit');

  // Wait briefly for processes to exit
  await new Promise(r => setTimeout(r, 200));

  return record;
}

// ---------------------------------------------------------------------------
// Main: run game pairs and produce report
// ---------------------------------------------------------------------------
async function main() {
  const pairs = config.game_pairs ?? 5;
  // Each pair plays all 3 seating arrangements (6 games per pair for full coverage)
  // Or if fewer pairs requested, cycle through arrangements
  const gamesPerPair = TEAM_PAIRINGS.length; // 3 arrangements
  const totalGames = pairs * gamesPerPair;
  const results = [];

  console.log(`\n=== DUEL: ${config.config_a.label} vs ${config.config_b.label} ===`);
  console.log(`${pairs} rounds × ${gamesPerPair} seatings = ${totalGames} games, max_ply=${config.max_ply ?? 160}`);
  console.log(`Seatings: RY|BG, RB|YG, RG|BY (tests all seating biases)\n`);

  for (let round = 0; round < pairs; round++) {
    for (let seat = 0; seat < gamesPerPair; seat++) {
      const gameNum = round * gamesPerPair + seat + 1;
      const pairing = getPairing(seat);
      process.stdout.write(`  Game ${gameNum}/${totalGames} (A=${pairing.a.join('')}, B=${pairing.b.join('')})...`);
      const g = await playDuelGame(gameNum, seat);
      results.push(g);
      console.log(` ${g.total_ply} plies${g.error ? ' ERROR: ' + g.error : ''}`);
    }
  }

  // Compute summary: average scores for team A and team B across all games
  let teamAScoreSum = 0;
  let teamBScoreSum = 0;
  let gamesWithScores = 0;

  for (const r of results) {
    if (!r.scores) continue;
    gamesWithScores++;
    const teamAPlayers = r.team_a.players;
    const teamBPlayers = r.team_b.players;
    const scoreMap = { Red: r.scores.red, Blue: r.scores.blue, Yellow: r.scores.yellow, Green: r.scores.green };

    for (const p of teamAPlayers) teamAScoreSum += (scoreMap[p] ?? 0);
    for (const p of teamBPlayers) teamBScoreSum += (scoreMap[p] ?? 0);
  }

  const avgA = gamesWithScores > 0 ? teamAScoreSum / gamesWithScores : 0;
  const avgB = gamesWithScores > 0 ? teamBScoreSum / gamesWithScores : 0;

  // Count eliminations per team
  let elimA = 0, elimB = 0;
  for (const r of results) {
    for (const e of r.eliminations) {
      if (r.team_a.players.includes(e.player)) elimA++;
      else elimB++;
    }
  }

  const summary = {
    config_a: config.config_a.label,
    config_b: config.config_b.label,
    total_games: totalGames,
    games_with_scores: gamesWithScores,
    team_a_avg_score: Math.round(avgA),
    team_b_avg_score: Math.round(avgB),
    team_a_eliminations: elimA,
    team_b_eliminations: elimB,
    score_diff: Math.round(avgA - avgB),
  };

  console.log(`\n=== DUEL RESULTS ===`);
  console.log(`  ${config.config_a.label}: avg score ${summary.team_a_avg_score}, eliminations ${elimA}`);
  console.log(`  ${config.config_b.label}: avg score ${summary.team_b_avg_score}, eliminations ${elimB}`);
  console.log(`  Score diff (A-B): ${summary.score_diff}`);
  console.log();

  // Save results
  writeFileSync(join(outDir, 'duel_results.json'), JSON.stringify({ summary, games: results }, null, 2));

  const report = [
    `# Duel: ${config.config_a.label} vs ${config.config_b.label}`,
    '',
    `**Games:** ${totalGames} (${pairs} pairs, swapped colors)`,
    '',
    `| Team | Avg Score | Eliminations |`,
    `|------|-----------|--------------|`,
    `| ${config.config_a.label} | ${summary.team_a_avg_score} | ${elimA} |`,
    `| ${config.config_b.label} | ${summary.team_b_avg_score} | ${elimB} |`,
    '',
    `**Score diff (A-B):** ${summary.score_diff}`,
    '',
    `## Per-Game Results`,
    '',
    ...results.map(r =>
      `- Game ${r.game}: ${r.swapped ? 'SWAPPED' : 'NORMAL'} — ` +
      `${r.total_ply} plies, scores: R=${r.scores?.red ?? '?'} B=${r.scores?.blue ?? '?'} Y=${r.scores?.yellow ?? '?'} G=${r.scores?.green ?? '?'}` +
      (r.error ? ` ERROR: ${r.error}` : '')
    ),
  ].join('\n');

  writeFileSync(join(outDir, 'duel_report.md'), report);
  console.log(`Results saved to ${outDir}/`);
}

main().catch(console.error);
