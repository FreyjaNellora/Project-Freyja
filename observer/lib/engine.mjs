// lib/engine.mjs — Engine process wrapper and protocol parser for Freyja
//
// Adapted from Odin's observer/lib/engine.mjs for Freyja protocol.

import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

export const PLAYERS = ['Red', 'Blue', 'Yellow', 'Green'];

// ---------------------------------------------------------------------------
// Engine process wrapper — line-based stdin/stdout communication
// ---------------------------------------------------------------------------
export class Engine {
  #proc;
  #rl;
  #lineQueue = [];
  #lineResolve = null;
  #dead = false;
  #stderrBuf = '';

  constructor(enginePath) {
    this.#proc = spawn(enginePath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.#proc.on('error', (err) => {
      console.error(`Engine failed to start: ${err.message}`);
      process.exit(1);
    });
    this.#proc.on('exit', (code, signal) => {
      this.#dead = true;
      if (code !== 0 && code !== null) {
        console.error(`\n[ENGINE CRASH] exit code=${code} signal=${signal}`);
        if (this.#stderrBuf) console.error(`[ENGINE STDERR] ${this.#stderrBuf}`);
      }
      if (this.#lineResolve) {
        const r = this.#lineResolve;
        this.#lineResolve = null;
        r(null);
      }
    });
    this.#proc.stderr.on('data', (chunk) => {
      this.#stderrBuf += chunk.toString();
    });
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

  get dead() { return this.#dead; }
  get stderrOutput() { return this.#stderrBuf; }

  send(cmd) {
    if (this.#dead) return;
    this.#proc.stdin.write(cmd + '\n');
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

  async handshake() {
    this.send('freyja');
    await this.readUntil('freyja');
    this.send('isready');
    await this.readUntil('readyok');
  }

  close() {
    this.send('quit');
    setTimeout(() => this.#proc.kill(), 500);
  }
}

// ---------------------------------------------------------------------------
// Parse one Freyja protocol line into a typed object
// ---------------------------------------------------------------------------
export function parseLine(line) {
  if (line.startsWith('info string eliminated ')) {
    const rest = line.slice('info string eliminated '.length);
    const color = rest.split(' ')[0];
    const reason = rest.split(' ').slice(1).join(' ') || null;
    return { type: 'eliminated', color, reason, raw: line };
  }
  if (line.startsWith('info string nextturn ')) {
    return { type: 'nextturn', player: line.slice('info string nextturn '.length).trim(), raw: line };
  }
  if (line.startsWith('info string')) {
    return { type: 'info_string', raw: line };
  }
  if (line.startsWith('bestmove ')) {
    const move = line.slice('bestmove '.length).trim();
    return { type: 'bestmove', move: move === '(none)' ? null : move, raw: line };
  }
  if (line.startsWith('info ')) {
    const o = { type: 'search_info', raw: line };
    const t = line.split(' ');
    for (let i = 1; i < t.length; i++) {
      switch (t[i]) {
        case 'depth': o.depth = +t[++i]; break;
        case 'score':
          // "score red R blue B yellow Y green G"
          if (t[i+1] === 'red') {
            o.red = +t[i+2]; o.blue = +t[i+4];
            o.yellow = +t[i+6]; o.green = +t[i+8];
            i += 8;
          }
          break;
        case 'nodes': o.nodes = +t[++i]; break;
        case 'nps': o.nps = +t[++i]; break;
        case 'pv': o.pv = t.slice(i+1).join(' '); i = t.length; break;
      }
    }
    return o;
  }
  return { type: 'unknown', raw: line };
}
