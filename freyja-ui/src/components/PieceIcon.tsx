// Renders a chess piece as an SVG text element.
// Uses Unicode chess symbols colored by player.

import type { Piece } from '../types/board';
import { PLAYER_COLORS, pieceSymbol } from '../lib/board-constants';

interface PieceIconProps {
  piece: Piece;
  x: number;
  y: number;
  size: number;
}

export default function PieceIcon({ piece, x, y, size }: PieceIconProps) {
  return (
    <text
      x={x + size / 2}
      y={y + size * 0.72}
      textAnchor="middle"
      fontSize={size * 0.7}
      fill={PLAYER_COLORS[piece.owner]}
      stroke="#000"
      strokeWidth={0.3}
      style={{ pointerEvents: 'none', userSelect: 'none' }}
    >
      {pieceSymbol(piece.pieceType)}
    </text>
  );
}
