// lib/fen4_parser.mjs — Lightweight FEN4 parser for behavioral metrics
//
// Parses Freyja 4-player chess FEN4 strings to extract piece positions and counts.
//
// FEN4 piece encoding: 2-char codes — lowercase player + uppercase piece type
//   Players: r=Red, b=Blue, y=Yellow, g=Green
//   Pieces:  P=Pawn, N=Knight, B=Bishop, R=Rook, Q=Queen, K=King, D=PromotedQueen
//   Examples: rP=Red Pawn, bK=Blue King, yQ=Yellow Queen, gN=Green Knight
//   Empty valid squares: digit counts (1-14)
//   Invalid corner squares: 'x'

const PLAYER_CHARS = new Set(['r', 'b', 'y', 'g']);
const PIECE_CHARS = new Set(['P', 'N', 'B', 'R', 'Q', 'K', 'D']);

const PLAYER_NAMES = { r: 'red', b: 'blue', y: 'yellow', g: 'green' };
const PIECE_NAMES = {
  P: 'pawn', N: 'knight', B: 'bishop', R: 'rook',
  Q: 'queen', K: 'king', D: 'promoted_queen',
};

/**
 * Parse a FEN4 string and extract piece information.
 *
 * @param {string} fen4 - The FEN4 string from the engine
 * @returns {{ pieces: Array<{rank, file, player, type}>, counts: Object, pawnCounts: Object }}
 */
export function parseFEN4(fen4) {
  const parts = fen4.split(/\s+/);
  const boardStr = parts[0];
  const ranks = boardStr.split('/');

  const pieces = [];
  const counts = { red: 0, blue: 0, yellow: 0, green: 0 };
  const pawnCounts = { red: 0, blue: 0, yellow: 0, green: 0 };

  for (let rankIdx = 0; rankIdx < ranks.length; rankIdx++) {
    let file = 0;
    const rankStr = ranks[rankIdx];
    let i = 0;

    while (i < rankStr.length) {
      const ch = rankStr[i];

      // Digits = empty squares (can be multi-digit like "14")
      if (ch >= '0' && ch <= '9') {
        let numStr = ch;
        while (i + 1 < rankStr.length && rankStr[i + 1] >= '0' && rankStr[i + 1] <= '9') {
          numStr += rankStr[++i];
        }
        file += parseInt(numStr, 10);
        i++;
        continue;
      }

      // Invalid corner square
      if (ch === 'x') {
        file++;
        i++;
        continue;
      }

      // 2-char piece code: player_char + piece_char
      if (PLAYER_CHARS.has(ch) && i + 1 < rankStr.length && PIECE_CHARS.has(rankStr[i + 1])) {
        const playerChar = ch;
        const pieceChar = rankStr[i + 1];
        const player = PLAYER_NAMES[playerChar];
        const type = PIECE_NAMES[pieceChar];

        pieces.push({ rank: rankIdx, file, player, type });
        counts[player]++;
        if (type === 'pawn') pawnCounts[player]++;
        file++;
        i += 2;
        continue;
      }

      // Unknown character — skip
      file++;
      i++;
    }
  }

  return { pieces, counts, pawnCounts };
}

/**
 * Count total pieces on the board from a FEN4 string.
 */
export function totalPieceCount(fen4) {
  const { counts } = parseFEN4(fen4);
  return counts.red + counts.blue + counts.yellow + counts.green;
}

/**
 * Get piece at a specific square from FEN4.
 * @returns {{ player, type } | null}
 */
export function pieceAt(fen4, targetFile, targetRank) {
  const { pieces } = parseFEN4(fen4);
  return pieces.find((p) => p.file === targetFile && p.rank === targetRank) ?? null;
}
