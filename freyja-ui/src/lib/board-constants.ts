// Board geometry constants mirroring freyja-engine/src/board/types.rs.
// Display logic only — no game rules.

import type { Piece, Player, PieceType } from '../types/board';
import { BOARD_SIZE, TOTAL_SQUARES } from '../types/board';

// --- Square validity ---
// Mirrors engine's is_valid_square exactly:
// Invalid corners: (rank > 10 || rank < 3) && (file > 10 || file < 3)

/** Check if a (rank, file) pair is a valid playable square. */
export function isValidSquareRF(rank: number, file: number): boolean {
  if (rank < 0 || rank >= BOARD_SIZE || file < 0 || file >= BOARD_SIZE) return false;
  const inCorner =
    (rank > 10 || rank < 3) && (file > 10 || file < 3);
  return !inCorner;
}

/** Check if a square index is a valid playable square. */
export function isValidSquare(index: number): boolean {
  if (index < 0 || index >= TOTAL_SQUARES) return false;
  const rank = Math.floor(index / BOARD_SIZE);
  const file = index % BOARD_SIZE;
  return isValidSquareRF(rank, file);
}

// --- Coordinate helpers ---

/** Extract file (column, 0-13) from a square index. */
export function fileOf(sq: number): number {
  return sq % BOARD_SIZE;
}

/** Extract rank (row, 0-13) from a square index. */
export function rankOf(sq: number): number {
  return Math.floor(sq / BOARD_SIZE);
}

/** Create a square index from file and rank. */
export function squareFrom(file: number, rank: number): number {
  return rank * BOARD_SIZE + file;
}

/** File letters a-n. */
const FILE_NAMES = 'abcdefghijklmn';

/** Convert a square index to algebraic notation (e.g., "d4"). */
export function squareName(sq: number): string {
  const file = fileOf(sq);
  const rank = rankOf(sq);
  return FILE_NAMES[file] + (rank + 1).toString();
}

/** Parse algebraic notation to a square index. Returns -1 if invalid. */
export function parseSquare(name: string): number {
  if (name.length < 2) return -1;
  const file = FILE_NAMES.indexOf(name[0]);
  if (file === -1) return -1;
  const rankStr = name.slice(1);
  if (!/^\d+$/.test(rankStr)) return -1;
  const rank = parseInt(rankStr, 10) - 1;
  if (isNaN(rank) || rank < 0 || rank >= BOARD_SIZE) return -1;
  return squareFrom(file, rank);
}

// --- Square coloring ---

/** Determine if a square should be light or dark (for alternating pattern). */
export function isLightSquare(file: number, rank: number): boolean {
  return (file + rank) % 2 === 0;
}

// --- Player colors ---

/** CSS color for each player. */
export const PLAYER_COLORS: Record<Player, string> = {
  Red: '#cc0000',
  Blue: '#0066cc',
  Yellow: '#ccaa00',
  Green: '#00aa44',
};

/** Light square color. */
export const LIGHT_SQUARE = '#f0d9b5';
/** Dark square color. */
export const DARK_SQUARE = '#b58863';
/** Selected square highlight. */
export const SELECTED_HIGHLIGHT = 'rgba(255, 255, 0, 0.45)';
/** Last move highlight. */
export const LAST_MOVE_HIGHLIGHT = 'rgba(0, 100, 255, 0.3)';

// --- Unicode piece symbols ---

const PIECE_SYMBOLS: Record<PieceType, string> = {
  King: '\u2654',
  Queen: '\u2655',
  Rook: '\u2656',
  Bishop: '\u2657',
  Knight: '\u2658',
  Pawn: '\u2659',
  PromotedQueen: '\u2655',
};

/** Get the display symbol for a piece type. */
export function pieceSymbol(pt: PieceType): string {
  return PIECE_SYMBOLS[pt];
}

// --- Starting position ---
// Mirrors freyja-engine starting position per 4PC_RULES_REFERENCE.md

/** Build the starting position board array (196 elements). */
export function startingPosition(): (Piece | null)[] {
  const board: (Piece | null)[] = new Array(TOTAL_SQUARES).fill(null);

  const place = (file: number, rank: number, pieceType: PieceType, owner: Player) => {
    board[squareFrom(file, rank)] = { pieceType, owner };
  };

  // Red: south side. Back rank d1-k1 (rank 0, files 3-10). R N B Q K B N R.
  const redBackRank: PieceType[] = ['Rook', 'Knight', 'Bishop', 'Queen', 'King', 'Bishop', 'Knight', 'Rook'];
  for (let i = 0; i < 8; i++) {
    place(3 + i, 0, redBackRank[i], 'Red');
    place(3 + i, 1, 'Pawn', 'Red');
  }

  // Blue: west side. Back file a4-a11 (file 0, ranks 3-10). R N B K Q B N R.
  const blueBackRank: PieceType[] = ['Rook', 'Knight', 'Bishop', 'King', 'Queen', 'Bishop', 'Knight', 'Rook'];
  for (let i = 0; i < 8; i++) {
    place(0, 3 + i, blueBackRank[i], 'Blue');
    place(1, 3 + i, 'Pawn', 'Blue');
  }

  // Yellow: north side. Back rank d14-k14 (rank 13, files 3-10). R N B K Q B N R.
  const yellowBackRank: PieceType[] = ['Rook', 'Knight', 'Bishop', 'King', 'Queen', 'Bishop', 'Knight', 'Rook'];
  for (let i = 0; i < 8; i++) {
    place(3 + i, 13, yellowBackRank[i], 'Yellow');
    place(3 + i, 12, 'Pawn', 'Yellow');
  }

  // Green: east side. Back file n4-n11 (file 13, ranks 3-10). R N B Q K B N R.
  const greenBackRank: PieceType[] = ['Rook', 'Knight', 'Bishop', 'Queen', 'King', 'Bishop', 'Knight', 'Rook'];
  for (let i = 0; i < 8; i++) {
    place(13, 3 + i, greenBackRank[i], 'Green');
    place(12, 3 + i, 'Pawn', 'Green');
  }

  return board;
}
