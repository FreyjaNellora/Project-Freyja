// Coordinate and starting position verification tests.
// Ensures UI board matches freyja-engine's board representation exactly.

import { describe, it, expect } from 'vitest';
import {
  isValidSquare,
  isValidSquareRF,
  squareFrom,
  squareName,
  parseSquare,
  fileOf,
  rankOf,
  startingPosition,
} from './board-constants';
import { BOARD_SIZE, TOTAL_SQUARES, VALID_SQUARE_COUNT } from '../types/board';

describe('isValidSquare', () => {
  it('counts exactly 160 valid squares', () => {
    let count = 0;
    for (let i = 0; i < TOTAL_SQUARES; i++) {
      if (isValidSquare(i)) count++;
    }
    expect(count).toBe(VALID_SQUARE_COUNT);
  });

  it('rejects SW corner (rank<3, file<3)', () => {
    for (let r = 0; r < 3; r++) {
      for (let f = 0; f < 3; f++) {
        expect(isValidSquareRF(r, f)).toBe(false);
      }
    }
  });

  it('rejects SE corner (rank<3, file>10)', () => {
    for (let r = 0; r < 3; r++) {
      for (let f = 11; f < 14; f++) {
        expect(isValidSquareRF(r, f)).toBe(false);
      }
    }
  });

  it('rejects NW corner (rank>10, file<3)', () => {
    for (let r = 11; r < 14; r++) {
      for (let f = 0; f < 3; f++) {
        expect(isValidSquareRF(r, f)).toBe(false);
      }
    }
  });

  it('rejects NE corner (rank>10, file>10)', () => {
    for (let r = 11; r < 14; r++) {
      for (let f = 11; f < 14; f++) {
        expect(isValidSquareRF(r, f)).toBe(false);
      }
    }
  });

  it('accepts center squares', () => {
    expect(isValidSquareRF(7, 7)).toBe(true);
    expect(isValidSquareRF(0, 7)).toBe(true);
    expect(isValidSquareRF(7, 0)).toBe(true);
  });
});

describe('coordinate helpers', () => {
  it('squareFrom and fileOf/rankOf round-trip', () => {
    for (let r = 0; r < BOARD_SIZE; r++) {
      for (let f = 0; f < BOARD_SIZE; f++) {
        const sq = squareFrom(f, r);
        expect(fileOf(sq)).toBe(f);
        expect(rankOf(sq)).toBe(r);
      }
    }
  });

  it('squareName matches expected notation', () => {
    expect(squareName(squareFrom(3, 0))).toBe('d1');
    expect(squareName(squareFrom(7, 0))).toBe('h1');
    expect(squareName(squareFrom(0, 6))).toBe('a7');
    expect(squareName(squareFrom(0, 7))).toBe('a8');
    expect(squareName(squareFrom(6, 13))).toBe('g14');
    expect(squareName(squareFrom(7, 13))).toBe('h14');
    expect(squareName(squareFrom(13, 6))).toBe('n7');
    expect(squareName(squareFrom(13, 7))).toBe('n8');
  });

  it('parseSquare handles multi-digit ranks', () => {
    expect(parseSquare('a10')).toBe(squareFrom(0, 9));
    expect(parseSquare('n14')).toBe(squareFrom(13, 13));
    expect(parseSquare('d1')).toBe(squareFrom(3, 0));
  });

  it('parseSquare returns -1 for invalid', () => {
    expect(parseSquare('')).toBe(-1);
    expect(parseSquare('z1')).toBe(-1);
    expect(parseSquare('a0')).toBe(-1);
    expect(parseSquare('a15')).toBe(-1);
  });
});

describe('key square indices', () => {
  it('d1 = index 3', () => expect(squareFrom(3, 0)).toBe(3));
  it('h1 = index 7 (Red King)', () => expect(squareFrom(7, 0)).toBe(7));
  it('g1 = index 6 (Red Queen)', () => expect(squareFrom(6, 0)).toBe(6));
  it('a7 = index 84 (Blue King)', () => expect(squareFrom(0, 6)).toBe(84));
  it('a8 = index 98 (Blue Queen)', () => expect(squareFrom(0, 7)).toBe(98));
  it('g14 = index 188 (Yellow King)', () => expect(squareFrom(6, 13)).toBe(188));
  it('h14 = index 189 (Yellow Queen)', () => expect(squareFrom(7, 13)).toBe(189));
  it('n7 = index 97 (Green Queen)', () => expect(squareFrom(13, 6)).toBe(97));
  it('n8 = index 111 (Green King)', () => expect(squareFrom(13, 7)).toBe(111));
});

describe('startingPosition', () => {
  const board = startingPosition();

  it('has 196 elements', () => {
    expect(board.length).toBe(TOTAL_SQUARES);
  });

  it('has exactly 64 pieces (16 per player)', () => {
    const count = board.filter((p) => p !== null).length;
    expect(count).toBe(64);
  });

  // Red pieces
  it('Red King at h1', () => {
    const p = board[squareFrom(7, 0)];
    expect(p?.pieceType).toBe('King');
    expect(p?.owner).toBe('Red');
  });

  it('Red Queen at g1', () => {
    const p = board[squareFrom(6, 0)];
    expect(p?.pieceType).toBe('Queen');
    expect(p?.owner).toBe('Red');
  });

  it('Red Rooks at d1 and k1', () => {
    expect(board[squareFrom(3, 0)]?.pieceType).toBe('Rook');
    expect(board[squareFrom(10, 0)]?.pieceType).toBe('Rook');
  });

  it('Red Pawns on rank 2 (d2-k2)', () => {
    for (let f = 3; f <= 10; f++) {
      const p = board[squareFrom(f, 1)];
      expect(p?.pieceType).toBe('Pawn');
      expect(p?.owner).toBe('Red');
    }
  });

  // Blue pieces
  it('Blue King at a7', () => {
    const p = board[squareFrom(0, 6)];
    expect(p?.pieceType).toBe('King');
    expect(p?.owner).toBe('Blue');
  });

  it('Blue Queen at a8', () => {
    const p = board[squareFrom(0, 7)];
    expect(p?.pieceType).toBe('Queen');
    expect(p?.owner).toBe('Blue');
  });

  it('Blue Pawns on file b (b4-b11)', () => {
    for (let r = 3; r <= 10; r++) {
      const p = board[squareFrom(1, r)];
      expect(p?.pieceType).toBe('Pawn');
      expect(p?.owner).toBe('Blue');
    }
  });

  // Yellow pieces
  it('Yellow King at g14', () => {
    const p = board[squareFrom(6, 13)];
    expect(p?.pieceType).toBe('King');
    expect(p?.owner).toBe('Yellow');
  });

  it('Yellow Queen at h14', () => {
    const p = board[squareFrom(7, 13)];
    expect(p?.pieceType).toBe('Queen');
    expect(p?.owner).toBe('Yellow');
  });

  it('Yellow Pawns on rank 13 (d13-k13)', () => {
    for (let f = 3; f <= 10; f++) {
      const p = board[squareFrom(f, 12)];
      expect(p?.pieceType).toBe('Pawn');
      expect(p?.owner).toBe('Yellow');
    }
  });

  // Green pieces
  it('Green King at n8', () => {
    const p = board[squareFrom(13, 7)];
    expect(p?.pieceType).toBe('King');
    expect(p?.owner).toBe('Green');
  });

  it('Green Queen at n7', () => {
    const p = board[squareFrom(13, 6)];
    expect(p?.pieceType).toBe('Queen');
    expect(p?.owner).toBe('Green');
  });

  it('Green Pawns on file m (m4-m11)', () => {
    for (let r = 3; r <= 10; r++) {
      const p = board[squareFrom(12, r)];
      expect(p?.pieceType).toBe('Pawn');
      expect(p?.owner).toBe('Green');
    }
  });
});
