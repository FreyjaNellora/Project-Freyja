// Raw protocol log with color-coded lines and manual command input.

import { useState, useRef, useEffect } from 'react';

interface CommunicationLogProps {
  rawLog: string[];
  onSendCommand: (cmd: string) => void;
  isConnected: boolean;
}

function lineColor(line: string): string {
  if (line.startsWith('info string error:')) return '#cc0000';
  if (line.startsWith('bestmove')) return '#00aa44';
  if (line.startsWith('info ')) return '#4488cc';
  if (line === 'readyok' || line.startsWith('freyja ')) return '#666';
  return '#ccc';
}

export default function CommunicationLog({ rawLog, onSendCommand, isConnected }: CommunicationLogProps) {
  const [input, setInput] = useState('');
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [rawLog.length]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || !isConnected) return;
    onSendCommand(input.trim());
    setInput('');
  };

  return (
    <div className="comm-log">
      <h3>Protocol Log</h3>
      <div className="comm-log-lines" ref={logRef}>
        {rawLog.map((line, i) => (
          <div key={i} className="comm-line" style={{ color: lineColor(line) }}>
            {line}
          </div>
        ))}
      </div>
      <form className="comm-input" onSubmit={handleSubmit}>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={isConnected ? 'Send command...' : 'Not connected'}
          disabled={!isConnected}
        />
        <button type="submit" disabled={!isConnected}>Send</button>
      </form>
    </div>
  );
}
