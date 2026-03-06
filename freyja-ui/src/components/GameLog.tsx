// Move history log with player colors and info snapshots.

import { useEffect, useRef } from 'react';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { MoveEntry } from '../hooks/useGameState';

interface GameLogProps {
  moveHistory: MoveEntry[];
}

export default function GameLog({ moveHistory }: GameLogProps) {
  const logRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [moveHistory.length]);

  return (
    <div className="game-log" ref={logRef}>
      <h3>Moves</h3>
      {moveHistory.length === 0 && (
        <div className="log-empty">No moves yet</div>
      )}
      {moveHistory.map((entry, i) => {
        const moveNum = Math.floor(i / 4) + 1;
        const infoStr = entry.info
          ? ` (d${entry.info.depth ?? '?'}, ${entry.info.nodes?.toLocaleString() ?? '?'} nodes)`
          : '';

        return (
          <div
            key={i}
            className="log-entry"
            style={{ borderLeftColor: PLAYER_COLORS[entry.player] }}
          >
            <span className="log-num">{moveNum}.</span>
            <span className="log-player" style={{ color: PLAYER_COLORS[entry.player] }}>
              {entry.player}:
            </span>
            <span className="log-move">{entry.move}</span>
            {infoStr && <span className="log-info">{infoStr}</span>}
          </div>
        );
      })}
    </div>
  );
}
