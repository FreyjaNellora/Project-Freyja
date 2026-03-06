// Engine connection status indicator.

interface StatusBarProps {
  isConnected: boolean;
  onConnect: () => void;
}

export default function StatusBar({ isConnected, onConnect }: StatusBarProps) {
  return (
    <div className="status-bar">
      <span className="status-title">Project Freyja</span>
      <span className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`}>
        {isConnected ? 'Connected' : 'Disconnected'}
      </span>
      {!isConnected && (
        <button className="connect-btn" onClick={onConnect}>
          Connect Engine
        </button>
      )}
    </div>
  );
}
