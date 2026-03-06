// Game controls: slot config, new game, auto-play, step, scores, turn indicator.

import type { Player, PlayerStatus } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { SlotConfig } from '../hooks/useGameState';

interface GameControlsProps {
  currentPlayer: Player;
  scores: [number, number, number, number];
  playerStatus: Record<Player, PlayerStatus>;
  slotConfig: SlotConfig;
  autoPlay: boolean;
  engineDelay: number;
  isPaused: boolean;
  isGameOver: boolean;
  isConnected: boolean;
  onSlotConfigChange: (config: SlotConfig) => void;
  onAutoPlayChange: (on: boolean) => void;
  onEngineDelayChange: (ms: number) => void;
  onNewGame: () => void;
  onRequestEngineMove: () => void;
  onTogglePause: () => void;
  onUndo: () => void;
}

export default function GameControls({
  currentPlayer,
  scores,
  playerStatus,
  slotConfig,
  autoPlay,
  engineDelay,
  isPaused,
  isGameOver,
  isConnected,
  onSlotConfigChange,
  onAutoPlayChange,
  onEngineDelayChange,
  onNewGame,
  onRequestEngineMove,
  onTogglePause,
  onUndo,
}: GameControlsProps) {
  const toggleSlot = (player: Player) => {
    const newConfig = { ...slotConfig };
    newConfig[player] = newConfig[player] === 'human' ? 'engine' : 'human';
    onSlotConfigChange(newConfig);
  };

  return (
    <div className="game-controls">
      <div className="section">
        <h3>Turn</h3>
        <div className="turn-indicator" style={{ color: PLAYER_COLORS[currentPlayer] }}>
          {isGameOver ? 'Game Over' : currentPlayer}
        </div>
      </div>

      <div className="section">
        <h3>Scores</h3>
        <div className="scores-grid">
          {PLAYERS.map((p, i) => (
            <div
              key={p}
              className={`score-entry ${playerStatus[p] === 'Eliminated' ? 'eliminated' : ''} ${p === currentPlayer ? 'active' : ''}`}
              style={{ borderLeftColor: PLAYER_COLORS[p] }}
            >
              <span className="score-name">{p}</span>
              <span className="score-value">{scores[i]}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="section">
        <h3>Players</h3>
        <div className="slot-config">
          {PLAYERS.map((p) => (
            <div key={p} className="slot-row">
              <span style={{ color: PLAYER_COLORS[p] }}>{p}</span>
              <button className="slot-btn" onClick={() => toggleSlot(p)}>
                {slotConfig[p] === 'human' ? 'Human' : 'Engine'}
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="section">
        <h3>Controls</h3>
        <div className="control-buttons">
          <button onClick={onNewGame}>New Game</button>
          <button
            onClick={onRequestEngineMove}
            disabled={!isConnected || isGameOver}
          >
            Engine Move
          </button>
          <button onClick={onUndo}>Undo</button>
        </div>
        <div className="control-buttons">
          <button
            className={autoPlay ? 'active-btn' : ''}
            onClick={() => {
              if (!autoPlay) {
                // Set all players to engine and start
                onSlotConfigChange({ Red: 'engine', Blue: 'engine', Yellow: 'engine', Green: 'engine' });
                onAutoPlayChange(true);
              } else {
                onAutoPlayChange(false);
              }
            }}
            disabled={!isConnected || isGameOver}
          >
            {autoPlay ? 'Stop' : 'Start'}
          </button>
          <button onClick={onTogglePause} disabled={!autoPlay}>
            {isPaused ? 'Resume' : 'Pause'}
          </button>
        </div>
        <div className="delay-control">
          <label>Delay: {engineDelay}ms</label>
          <input
            type="range"
            min={0}
            max={5000}
            step={100}
            value={engineDelay}
            onChange={(e) => onEngineDelayChange(parseInt(e.target.value, 10))}
          />
        </div>
      </div>

      {!isConnected && (
        <div className="section warning">
          Engine not connected
        </div>
      )}
    </div>
  );
}
