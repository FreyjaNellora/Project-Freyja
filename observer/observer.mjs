#!/usr/bin/env node
// observer.mjs — Automated gameplay observer for Project Freyja
//
// Spawns the engine, plays N games, captures all protocol output,
// writes structured reports. No analysis — just records what happened.
//
// Usage: node observer.mjs [config.json]

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2] || join(__dirname, 'config.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));

// ---------------------------------------------------------------------------
// Play one game — returns a structured record
// ---------------------------------------------------------------------------
async function playGame(engine, gameNum) {
  const record = {
    game: gameNum,
    settings: { depth: config.depth },
    plies: [],
    eliminations: [],
    winner: null,
    total_ply: 0,
    error: null,
  };

  const moveList = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let ply = 0;

  while (!gameOver && ply < (config.max_ply ?? 400)) {
    const posCmd = moveList.length === 0
      ? 'position startpos'
      : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);
    engine.send(`go depth ${config.depth}`);

    const rawLines = [];
    let bestmove = null;
    let lastSearch = null;

    while (true) {
      const line = await engine.readLine();
      if (line === null) {
        record.error = `Engine died at ply ${ply}`;
        gameOver = true;
        break;
      }
      rawLines.push(line);
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
        // Check for game over / no legal moves
        if (p.raw.includes('game is over') || p.raw.includes('no legal moves')) {
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
      // bestmove (none) — game over
      gameOver = true;
      break;
    }

    if (bestmove) {
      record.plies.push({
        ply,
        player: currentPlayer,
        move: bestmove,
        scores: lastSearch ? { red: lastSearch.red, blue: lastSearch.blue, yellow: lastSearch.yellow, green: lastSearch.green } : null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        pv: lastSearch?.pv ?? null,
        raw_lines: rawLines,
      });

      moveList.push(bestmove);
      ply++;

      // Advance to next non-eliminated player (fallback if no nextturn event)
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
  return record;
}

// ---------------------------------------------------------------------------
// Generate a plain-text summary report
// ---------------------------------------------------------------------------
function summary(games) {
  const lines = [];
  lines.push('# Freyja Observer Report');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push(`Games: ${games.length} | Depth: ${config.depth}`);
  lines.push('');

  for (const g of games) {
    lines.push(`## Game ${g.game} — ${g.total_ply} ply`);
    if (g.error) lines.push(`ERROR: ${g.error}`);
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
  console.log(`Games: ${config.games} | Depth: ${config.depth}`);
  console.log('');

  const engine = new Engine(enginePath);
  await engine.handshake();
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
    console.log(`${record.total_ply} ply${record.error ? ' [ERROR: ' + record.error + ']' : ''}`);

    writeFileSync(join(outputDir, `game_${String(i).padStart(3, '0')}.json`), JSON.stringify(record, null, 2));
  }

  engine.close();

  writeFileSync(join(outputDir, 'all_games.json'), JSON.stringify(games, null, 2));
  const summaryPath = join(outputDir, 'summary.md');
  writeFileSync(summaryPath, summary(games));

  console.log(`\nDone. Reports in ${outputDir}`);
  console.log(`Summary: ${summaryPath}`);
}

main().catch((err) => {
  console.error('Observer error:', err);
  process.exit(1);
});
