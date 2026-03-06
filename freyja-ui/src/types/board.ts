// Board types mirroring freyja-engine's board module.
// These are display types only — no game logic.

/** Player colors in turn order. */
export type Player = 'Red' | 'Blue' | 'Yellow' | 'Green';

/** All players in turn order. */
export const PLAYERS: Player[] = ['Red', 'Blue', 'Yellow', 'Green'];

/** Piece types matching engine's PieceType enum. */
export type PieceType =
  | 'Pawn'
  | 'Knight'
  | 'Bishop'
  | 'Rook'
  | 'Queen'
  | 'King'
  | 'PromotedQueen';

/** A piece on the board. */
export interface Piece {
  pieceType: PieceType;
  owner: Player;
}

/** Player status in the game. */
export type PlayerStatus = 'Active' | 'DeadKingWalking' | 'Eliminated';

/** Board dimension. */
export const BOARD_SIZE = 14;

/** Total squares in the 14x14 grid. */
export const TOTAL_SQUARES = 196;

/** Number of valid (playable) squares. */
export const VALID_SQUARE_COUNT = 160;
