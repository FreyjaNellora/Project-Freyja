// Protocol parser for Freyja engine output.
// Extension-tolerant: extracts known tokens, ignores trailing.
// Matches formats from freyja-engine/src/protocol/output.rs.

import type { Player } from '../types/board';
import type { EngineMessage, InfoData } from '../types/protocol';

const VALID_PLAYERS: Player[] = ['Red', 'Blue', 'Yellow', 'Green'];

function isPlayer(s: string): s is Player {
  return VALID_PLAYERS.includes(s as Player);
}

/** Parse a single engine output line into a typed message. */
export function parseEngineLine(raw: string): EngineMessage {
  const trimmed = raw.trim();
  if (!trimmed) return { type: 'unknown', raw: '' };

  const tokens = trimmed.split(/\s+/);
  const first = tokens[0];

  // Header: "freyja v1.0 maxn-beam-mcts"
  if (first === 'freyja' && tokens.length >= 2) {
    return { type: 'header', version: tokens.slice(1).join(' ') };
  }

  // Ready acknowledgment
  if (first === 'readyok') {
    return { type: 'readyok' };
  }

  // Best move: "bestmove d2d4" or "bestmove (none)"
  if (first === 'bestmove') {
    const moveStr = tokens[1];
    if (!moveStr || moveStr === '(none)') {
      return { type: 'bestmove', move: null };
    }
    return { type: 'bestmove', move: moveStr };
  }

  // Info lines
  if (first === 'info') {
    // "info string ..." sub-types
    if (tokens[1] === 'string') {
      return parseInfoString(tokens.slice(2), trimmed);
    }
    // "info depth D score red R blue B yellow Y green G nodes N nps NPS pv ..."
    return parseInfoData(tokens.slice(1));
  }

  return { type: 'unknown', raw: trimmed };
}

function parseInfoString(tokens: string[], raw: string): EngineMessage {
  if (tokens.length === 0) return { type: 'info_string', message: '' };

  const keyword = tokens[0];

  // "eliminated Red checkmate" — extract first token after "eliminated" as player
  if (keyword === 'eliminated' && tokens.length >= 2) {
    const playerStr = tokens[1];
    if (isPlayer(playerStr)) {
      const reason = tokens.slice(2).join(' ') || 'unknown';
      return { type: 'eliminated', player: playerStr, reason };
    }
  }

  // "nextturn Blue" — extract first token after "nextturn" as player
  if (keyword === 'nextturn' && tokens.length >= 2) {
    const playerStr = tokens[1];
    if (isPlayer(playerStr)) {
      return { type: 'nextturn', player: playerStr };
    }
  }

  // "error: some message"
  if (keyword === 'error:') {
    return { type: 'error', message: tokens.slice(1).join(' ') };
  }

  // Generic info string
  return { type: 'info_string', message: tokens.join(' ') };
}

function parseInfoData(tokens: string[]): EngineMessage {
  const data: InfoData = {};
  let i = 0;

  while (i < tokens.length) {
    const key = tokens[i];

    if (key === 'depth' && i + 1 < tokens.length) {
      data.depth = parseInt(tokens[i + 1], 10);
      i += 2;
    } else if (key === 'score' && i + 8 < tokens.length) {
      // "score red R blue B yellow Y green G"
      // tokens[i+1]="red", [i+2]=R, [i+3]="blue", [i+4]=B, ...
      const r = parseInt(tokens[i + 2], 10);
      const b = parseInt(tokens[i + 4], 10);
      const y = parseInt(tokens[i + 6], 10);
      const g = parseInt(tokens[i + 8], 10);
      if (!isNaN(r) && !isNaN(b) && !isNaN(y) && !isNaN(g)) {
        data.scores = [r, b, y, g];
      }
      i += 9;
    } else if (key === 'nodes' && i + 1 < tokens.length) {
      data.nodes = parseInt(tokens[i + 1], 10);
      i += 2;
    } else if (key === 'nps' && i + 1 < tokens.length) {
      data.nps = parseInt(tokens[i + 1], 10);
      i += 2;
    } else if (key === 'pv') {
      // PV: all remaining tokens are moves (until end or next known keyword)
      const pvMoves: string[] = [];
      i += 1;
      while (i < tokens.length) {
        const t = tokens[i];
        // Stop at known keywords (future-proof)
        if (['depth', 'score', 'nodes', 'nps', 'pv'].includes(t)) break;
        pvMoves.push(t);
        i++;
      }
      data.pv = pvMoves;
    } else {
      // Unknown token — skip (extension tolerant)
      i++;
    }
  }

  return { type: 'info', data };
}
