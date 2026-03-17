#!/usr/bin/env node
// observer.mjs — Automated gameplay observer for Project Freyja
//
// Spawns the engine, plays N games, captures all protocol output,
// writes structured reports. No analysis — just records what happened.
//
// Stage 12: Enhanced with FEN4 capture, configurable setoptions,
// movetime support, game_result computation, and richer ply schema.
//
// Usage: node observer.mjs [config.json]

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';
import { computeMetrics } from './lib/metrics.mjs';
import { aggregateStats, formatStatsReport } from './lib/stats.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2] || join(__dirname, 'config.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));

// ---------------------------------------------------------------------------
// Determine game result from final scores and eliminations
// ---------------------------------------------------------------------------
function computeGameResult(record) {
  const lastPly = record.plies[record.plies.length - 1];
  const scores = lastPly?.scores ?? null;

  if (!scores) {
    return { winner: null, final_scores: null, reason: 'unknown' };
  }

  const final_scores = { ...scores };

  // Check if only one player remains (last standing)
  const eliminated = new Set(record.eliminations.map((e) => e.player));
  const alive = PLAYERS.filter((p) => !eliminated.has(p));

  let reason = 'max_ply';
  if (alive.length === 1) {
    reason = 'last_standing';
  } else if (record.max_rounds_reached) {
    reason = 'max_rounds';
  }

  // Winner is the player with highest score
  let winner = null;
  let best = -Infinity;
  for (const p of PLAYERS) {
    const key = p.toLowerCase();
    if (final_scores[key] !== undefined && final_scores[key] > best) {
      best = final_scores[key];
      winner = p;
    }
  }

  return { winner, final_scores, reason };
}

// ---------------------------------------------------------------------------
// Play one game — returns a structured record
// ---------------------------------------------------------------------------
async function playGame(engine, gameNum) {
  const captureRaw = config.capture_raw ?? false;
  const record = {
    game: gameNum,
    settings: {
      depth: config.depth ?? null,
      movetime: config.movetime ?? null,
      setoptions: config.setoptions ?? {},
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
  let lastFen4 = null; // Track FEN4 for position-by-fen mode

  while (!gameOver && ply < (config.max_ply ?? 400)) {
    // Set position — use FEN4 when available to avoid replaying long move lists
    // (replaying 30+ moves from startpos causes crashes in deep search positions)
    const posCmd = lastFen4
      ? `position fen4 ${lastFen4}`
      : moveList.length === 0
        ? 'position startpos'
        : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);

    // Capture FEN4 before search — also process any side-effect lines
    // (nextturn, eliminated events from position replay)
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

    // Start search
    const goCmd = config.movetime
      ? `go movetime ${config.movetime}`
      : `go depth ${config.depth}`;
    engine.send(goCmd);

    const rawLines = captureRaw ? [] : undefined;
    let bestmove = null;
    let lastSearch = null;

    while (true) {
      const line = await engine.readLine();
      if (line === null) {
        record.error = `Engine died at ply ${ply}`;
        gameOver = true;
        break;
      }
      if (captureRaw) rawLines.push(line);
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
        if (p.raw.includes('game stopped') && p.raw.includes('max rounds')) {
          record.max_rounds_reached = true;
          gameOver = true;
          break;
        }
        if (p.raw.includes('error:')) {
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
      const plyRecord = {
        ply,
        round: Math.floor(ply / 4),
        player: currentPlayer,
        move: bestmove,
        fen4: fen4 ?? null,
        scores: lastSearch ? {
          red: lastSearch.red, blue: lastSearch.blue,
          yellow: lastSearch.yellow, green: lastSearch.green,
        } : null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        qnodes: lastSearch?.qnodes ?? null,
        nps: lastSearch?.nps ?? null,
        tthitrate: lastSearch?.tthitrate ?? null,
        killerhitrate: lastSearch?.killerhitrate ?? null,
        pv: lastSearch?.pv ?? null,
      };
      if (captureRaw) plyRecord.raw_lines = rawLines;
      record.plies.push(plyRecord);

      moveList.push(bestmove);
      ply++;

      // After making the move, capture the new FEN4 for next iteration.
      // Use fen4 + single move instead of replaying full move history,
      // which avoids crashes from long move list replays.
      if (fen4) {
        engine.send(`position fen4 ${fen4} moves ${bestmove}`);
        const postFenResult = await engine.getFEN4();
        lastFen4 = postFenResult.fen4;
        // Process any side effects
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

      // Advance to next non-eliminated player
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
  record.game_result = computeGameResult(record);
  return record;
}

// ---------------------------------------------------------------------------
// Generate a plain-text summary report
// ---------------------------------------------------------------------------
function summary(games) {
  const lines = [];
  lines.push('# Freyja Observer Report');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push(`Games: ${games.length} | Depth: ${config.depth ?? 'N/A'} | Movetime: ${config.movetime ?? 'N/A'}`);
  lines.push('');

  for (const g of games) {
    lines.push(`## Game ${g.game} — ${g.total_ply} ply (${g.total_rounds} rounds)`);
    if (g.error) lines.push(`ERROR: ${g.error}`);
    if (g.game_result) {
      lines.push(`Result: ${g.game_result.winner ?? 'N/A'} wins (${g.game_result.reason})`);
      if (g.game_result.final_scores) {
        const s = g.game_result.final_scores;
        lines.push(`Final scores: R:${s.red} B:${s.blue} Y:${s.yellow} G:${s.green}`);
      }
    }
    if (g.eliminations.length) {
      lines.push(`Eliminations: ${g.eliminations.map((e) => `${e.player} (${e.reason ?? '?'}, ply ${e.at_ply})`).join(', ')}`);
    }
    lines.push('');

    for (const p of PLAYERS) {
      const moves = g.plies.filter((m) => m.player === p);
      if (!moves.length) continue;
      const moveStr = moves.slice(0, 15).map((m) => m.move).join(' ');
      lines.push(`**${p}** (${moves.length} moves): \`${moveStr}\``);
    }
    lines.push('');

    lines.push('<details><summary>Full move log</summary>');
    lines.push('');
    for (const m of g.plies) {
      const s = m.scores ? ` [R:${m.scores.red} B:${m.scores.blue} Y:${m.scores.yellow} G:${m.scores.green}]` : '';
      lines.push(`${m.ply}. ${m.player}: ${m.move} (d${m.depth ?? '?'}, ${m.nodes ?? '?'}n)${s}`);
    }
    lines.push('</details>');
    lines.push('');
  }

  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const outputDir = resolve(__dirname, config.output_dir ?? 'reports');
  if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true });

  const enginePath = resolve(__dirname, config.engine);
  console.log(`Engine: ${enginePath}`);
  console.log(`Games: ${config.games} | Depth: ${config.depth ?? 'N/A'} | Movetime: ${config.movetime ?? 'N/A'}`);
  console.log('');

  const engine = new Engine(enginePath);
  await engine.handshake();

  // Apply setoptions from config
  if (config.setoptions && Object.keys(config.setoptions).length > 0) {
    await engine.sendOptions(config.setoptions);
    console.log(`Options: ${JSON.stringify(config.setoptions)}`);
  }

  console.log('Engine ready.\n');

  const games = [];
  for (let i = 1; i <= config.games; i++) {
    // Reset engine state between games
    engine.send('position startpos');
    engine.send('isready');
    await engine.readUntil('readyok');

    process.stdout.write(`Game ${i}/${config.games} ... `);
    const record = await playGame(engine, i);
    games.push(record);

    const winner = record.game_result?.winner ?? '?';
    console.log(`${record.total_ply} ply, winner: ${winner}${record.error ? ' [ERROR: ' + record.error + ']' : ''}`);

    writeFileSync(join(outputDir, `game_${String(i).padStart(3, '0')}.json`), JSON.stringify(record, null, 2));
  }

  engine.close();

  // Write all games JSON
  writeFileSync(join(outputDir, 'all_games.json'), JSON.stringify(games, null, 2));

  // Write text summary
  const summaryPath = join(outputDir, 'summary.md');
  writeFileSync(summaryPath, summary(games));

  // Compute and write aggregate stats
  try {
    const stats = aggregateStats(games);
    writeFileSync(join(outputDir, 'stats.json'), JSON.stringify(stats, null, 2));
    const statsReport = formatStatsReport(stats, games.length);
    writeFileSync(join(outputDir, 'stats.md'), statsReport);
    console.log(`\nStats written to ${join(outputDir, 'stats.json')}`);
  } catch (e) {
    console.error(`Stats computation failed: ${e.message}`);
  }

  console.log(`\nDone. Reports in ${outputDir}`);
  console.log(`Summary: ${summaryPath}`);
}

main().catch((err) => {
  console.error('Observer error:', err);
  process.exit(1);
});
