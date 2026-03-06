// Freyja UI — Root component.
// 3-panel layout: Controls (left), Board (center), Analysis (right).

import { useEngine } from './hooks/useEngine';
import { useGameState } from './hooks/useGameState';
import StatusBar from './components/StatusBar';
import BoardDisplay from './components/BoardDisplay';
import GameControls from './components/GameControls';
import GameLog from './components/GameLog';
import AnalysisPanel from './components/AnalysisPanel';
import CommunicationLog from './components/CommunicationLog';
import PromotionDialog from './components/PromotionDialog';
import './App.css';

export default function App() {
  const engine = useEngine();
  const game = useGameState(engine);

  return (
    <div className="app">
      <StatusBar
        isConnected={engine.isConnected}
        onConnect={engine.spawnEngine}
      />

      <div className="app-layout">
        {/* Left panel: controls + game log */}
        <div className="panel panel-left">
          <GameControls
            currentPlayer={game.currentPlayer}
            scores={game.scores}
            playerStatus={game.playerStatus}
            slotConfig={game.slotConfig}
            autoPlay={game.autoPlay}
            engineDelay={game.engineDelay}
            isPaused={game.isPaused}
            isGameOver={game.isGameOver}
            isConnected={engine.isConnected}
            onSlotConfigChange={game.setSlotConfig}
            onAutoPlayChange={game.setAutoPlay}
            onEngineDelayChange={game.setEngineDelay}
            onNewGame={game.newGame}
            onRequestEngineMove={game.requestEngineMove}
            onTogglePause={game.togglePause}
            onUndo={game.undo}
          />
          <GameLog moveHistory={game.moveHistory} />
        </div>

        {/* Center: board */}
        <div className="panel panel-center">
          <BoardDisplay
            board={game.board}
            selectedSquare={game.selectedSquare}
            lastMoveFrom={game.lastMoveFrom}
            lastMoveTo={game.lastMoveTo}
            onSquareClick={game.onSquareClick}
          />
        </div>

        {/* Right panel: analysis + protocol log */}
        <div className="panel panel-right">
          <AnalysisPanel latestInfo={game.latestInfo} />
          <CommunicationLog
            rawLog={engine.rawLog}
            onSendCommand={engine.sendCommand}
            isConnected={engine.isConnected}
          />
        </div>
      </div>

      {/* Promotion dialog overlay */}
      {game.pendingPromotion && (
        <PromotionDialog
          onSelect={game.onPromotionSelect}
          onCancel={game.onPromotionCancel}
        />
      )}
    </div>
  );
}
