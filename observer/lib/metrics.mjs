// lib/metrics.mjs — Behavioral metrics computation for self-play games
//
// Computes per-game metrics from structured game records:
//   - Pawn ratio (pawns remaining at end vs start)
//   - Queen activation round (first round a queen moves)
//   - Captures per 10 rounds (piece count delta windows)
//   - King moves count
//   - Piece shuffling index (moves to recently-visited squares)
//   - Game length in rounds
//
// Stage 12: Self-Play Framework

import { parseFEN4 } from './fen4_parser.mjs';

const PLAYERS = ['red', 'blue', 'yellow', 'green'];
const PAWNS_PER_PLAYER = 8;

// File/rank notation for move parsing (a=0..n=13, ranks 1-14)
function parseSquare(notation) {
  if (!notation || notation.length < 2) return null;
  const file = notation.charCodeAt(0) - 'a'.charCodeAt(0);
  const rankStr = notation.slice(1);
  const rank = parseInt(rankStr, 10) - 1;
  if (isNaN(rank) || file < 0 || file > 13 || rank < 0 || rank > 13) return null;
  return { file, rank };
}

// Extract from and to squares from a move string (e.g., "d2d4", "d14d12", "d7d8q")
function parseMoveSquares(moveStr) {
  if (!moveStr || moveStr.length < 4) return null;
  // From square: first char is file, then rank digits until next file letter
  const fromFile = moveStr.charCodeAt(0) - 'a'.charCodeAt(0);
  let i = 1;
  while (i < moveStr.length && moveStr[i] >= '0' && moveStr[i] <= '9') i++;
  const fromRank = parseInt(moveStr.slice(1, i), 10) - 1;

  const toFile = moveStr.charCodeAt(i) - 'a'.charCodeAt(0);
  let j = i + 1;
  while (j < moveStr.length && moveStr[j] >= '0' && moveStr[j] <= '9') j++;
  const toRank = parseInt(moveStr.slice(i + 1, j), 10) - 1;

  return { from: { file: fromFile, rank: fromRank }, to: { file: toFile, rank: toRank } };
}

/**
 * Compute behavioral metrics from a single game record.
 *
 * @param {Object} gameRecord - A game record from observer.mjs
 * @returns {Object} Computed metrics
 */
export function computeMetrics(gameRecord) {
  const plies = gameRecord.plies;
  if (!plies || plies.length === 0) {
    return null;
  }

  // 1. Pawn ratio — compare first and last FEN4
  const pawnRatio = { red: null, blue: null, yellow: null, green: null, avg: null };
  const firstFen = plies[0]?.fen4;
  const lastFen = plies[plies.length - 1]?.fen4;
  if (firstFen && lastFen) {
    const startPawns = parseFEN4(firstFen).pawnCounts;
    const endPawns = parseFEN4(lastFen).pawnCounts;
    let total = 0;
    let count = 0;
    for (const p of PLAYERS) {
      const start = startPawns[p] || PAWNS_PER_PLAYER;
      pawnRatio[p] = start > 0 ? endPawns[p] / start : 0;
      total += pawnRatio[p];
      count++;
    }
    pawnRatio.avg = total / count;
  }

  // 2. Queen activation round — first round where a queen-move appears per player
  const queenActivation = { red: null, blue: null, yellow: null, green: null };
  for (const ply of plies) {
    const player = ply.player.toLowerCase();
    if (queenActivation[player] !== null) continue;
    if (!ply.fen4 || !ply.move) continue;

    const squares = parseMoveSquares(ply.move);
    if (!squares) continue;
    const { pieces } = parseFEN4(ply.fen4);
    const movingPiece = pieces.find(
      (p) => p.file === squares.from.file && p.rank === squares.from.rank && p.player === player,
    );
    if (movingPiece && (movingPiece.type === 'queen' || movingPiece.type === 'promoted_queen')) {
      queenActivation[player] = ply.round;
    }
  }

  // 3. Captures per 10 rounds — count piece count deltas
  const maxRound = plies[plies.length - 1]?.round ?? 0;
  const captureWindows = [];
  let prevTotal = null;
  for (let windowStart = 0; windowStart <= maxRound; windowStart += 10) {
    const windowEnd = Math.min(windowStart + 9, maxRound);
    // Find first and last ply in this window with FEN4
    const windowPlies = plies.filter((p) => p.round >= windowStart && p.round <= windowEnd && p.fen4);
    if (windowPlies.length === 0) {
      captureWindows.push({ window: `${windowStart}-${windowEnd}`, captures: 0 });
      continue;
    }
    const startCount = prevTotal ?? totalPiecesFromFen(windowPlies[0].fen4);
    const endCount = totalPiecesFromFen(windowPlies[windowPlies.length - 1].fen4);
    const captures = Math.max(0, startCount - endCount);
    captureWindows.push({ window: `${windowStart}-${windowEnd}`, captures });
    prevTotal = endCount;
  }

  // 4. King moves count
  const kingMoves = { red: 0, blue: 0, yellow: 0, green: 0 };
  for (const ply of plies) {
    const player = ply.player.toLowerCase();
    if (!ply.fen4 || !ply.move) continue;
    const squares = parseMoveSquares(ply.move);
    if (!squares) continue;
    const { pieces } = parseFEN4(ply.fen4);
    const movingPiece = pieces.find(
      (p) => p.file === squares.from.file && p.rank === squares.from.rank && p.player === player,
    );
    if (movingPiece && movingPiece.type === 'king') {
      kingMoves[player]++;
    }
  }

  // 5. Piece shuffling index — fraction of moves where piece returns to a recently-visited square
  const recentSquares = {}; // player -> Map<pieceKey, Set<squareKey>>
  let shuffleMoves = 0;
  let totalMoves = 0;
  for (const p of PLAYERS) recentSquares[p] = new Map();

  for (const ply of plies) {
    const player = ply.player.toLowerCase();
    if (!ply.move) continue;
    const squares = parseMoveSquares(ply.move);
    if (!squares) continue;

    totalMoves++;
    const toKey = `${squares.to.file},${squares.to.rank}`;
    const fromKey = `${squares.from.file},${squares.from.rank}`;

    // Check if destination was recently visited by this piece
    const playerHistory = recentSquares[player];
    if (playerHistory.has(fromKey)) {
      const visited = playerHistory.get(fromKey);
      if (visited.has(toKey)) {
        shuffleMoves++;
      }
    }

    // Update history: transfer from fromKey to toKey
    const history = playerHistory.get(fromKey) || new Set();
    history.add(fromKey); // Remember where we came from
    playerHistory.delete(fromKey);
    playerHistory.set(toKey, history);
  }

  const shuffleIndex = totalMoves > 0 ? shuffleMoves / totalMoves : 0;

  // 6. Average score delta per round
  let scoreDeltaSum = 0;
  let scoreDeltaCount = 0;
  for (let i = 4; i < plies.length; i++) {
    if (plies[i].scores && plies[i - 4].scores) {
      for (const p of PLAYERS) {
        const delta = Math.abs((plies[i].scores[p] ?? 0) - (plies[i - 4].scores[p] ?? 0));
        scoreDeltaSum += delta;
        scoreDeltaCount++;
      }
    }
  }

  return {
    pawn_ratio: pawnRatio,
    queen_activation_round: queenActivation,
    captures_per_10_rounds: captureWindows,
    king_moves: kingMoves,
    shuffle_index: shuffleIndex,
    avg_score_delta_per_round: scoreDeltaCount > 0 ? scoreDeltaSum / scoreDeltaCount : 0,
    game_length_rounds: gameRecord.total_rounds,
    game_length_ply: gameRecord.total_ply,
  };
}

function totalPiecesFromFen(fen4) {
  if (!fen4) return 0;
  const { counts } = parseFEN4(fen4);
  return counts.red + counts.blue + counts.yellow + counts.green;
}
