#!/usr/bin/env node
// extract_fen4.mjs — Replay game moves and extract FEN4 at each sample's test point.
// Outputs updated samples with fen4 fields.
//
// Usage: node extract_fen4.mjs <path-to-freyja-binary>

import { readFileSync, writeFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SAMPLES_PATH = join(__dirname, 'tactical_samples.json');

class Engine {
  #proc;
  #rl;
  #lineQueue = [];
  #lineResolve = null;
  #dead = false;

  constructor(enginePath) {
    this.#proc = spawn(enginePath, [], { stdio: ['pipe', 'pipe', 'pipe'] });
    this.#proc.on('exit', () => { this.#dead = true; });
    this.#proc.stderr.on('data', () => {});
    this.#rl = createInterface({ input: this.#proc.stdout });
    this.#rl.on('line', (line) => {
      if (this.#lineResolve) {
        const r = this.#lineResolve;
        this.#lineResolve = null;
        r(line);
      } else {
        this.#lineQueue.push(line);
      }
    });
  }

  send(cmd) {
    if (!this.#dead) this.#proc.stdin.write(cmd + '\n');
  }

  async readLine() {
    if (this.#lineQueue.length > 0) return this.#lineQueue.shift();
    if (this.#dead) return null;
    return new Promise((resolve) => { this.#lineResolve = resolve; });
  }

  async readUntil(prefix) {
    const lines = [];
    while (true) {
      const line = await this.readLine();
      if (line === null) break;
      lines.push(line);
      if (line.startsWith(prefix)) break;
    }
    return lines;
  }

  quit() { this.send('quit'); }
}

async function main() {
  const enginePath = process.argv[2];
  if (!enginePath) {
    console.error('Usage: node extract_fen4.mjs <path-to-freyja-binary>');
    process.exit(1);
  }

  const data = JSON.parse(readFileSync(SAMPLES_PATH, 'utf8'));
  const engine = new Engine(enginePath);
  await engine.readLine(); // banner
  engine.send('isready');
  await engine.readUntil('readyok');

  // Extract FEN4 for each sample that has moves_to_replay
  for (const sample of data.samples) {
    if (!sample.moves_to_replay || sample.moves_to_replay.startsWith('n/a')) {
      console.log(`[${sample.id}] SKIP — no replay moves`);
      continue;
    }

    // Set position and replay moves
    engine.send(`position startpos moves ${sample.moves_to_replay}`);

    // Dump FEN4
    engine.send('d');
    const lines = await engine.readUntil('fen4');
    const fen4Line = lines.find(l => l.startsWith('fen4 '));

    if (fen4Line) {
      const fen4 = fen4Line.substring(5); // strip "fen4 " prefix
      sample.fen4 = fen4;

      // Also capture nextturn
      const nextTurnLine = lines.find(l => l.includes('nextturn'));
      const nextTurn = nextTurnLine?.match(/nextturn\s+(\w+)/)?.[1] || null;
      if (nextTurn) sample.nextturn = nextTurn;

      console.log(`[${sample.id}] ${nextTurn || '?'} — ${fen4.substring(0, 60)}...`);
    } else {
      console.log(`[${sample.id}] ERROR — no fen4 output`);
    }
  }

  engine.quit();

  // Write updated JSON
  writeFileSync(SAMPLES_PATH, JSON.stringify(data, null, 2) + '\n');
  console.log(`\nUpdated ${SAMPLES_PATH}`);
}

main().catch(e => { console.error(e); process.exit(1); });
