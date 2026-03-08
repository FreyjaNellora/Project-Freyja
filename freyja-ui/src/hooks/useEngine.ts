// Engine lifecycle and IPC bridge.
// Manages spawning, command sending, and stdout event listening.
// Generation tagging prevents stale events from killed engine processes.

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { EngineMessage } from '../types/protocol';
import { parseEngineLine } from '../lib/protocol-parser';

const MAX_LOG_LINES = 1000;
const DROP_LINES = 200;

interface EngineOutputPayload {
  line: string;
  gen: number;
}

export interface UseEngineResult {
  isConnected: boolean;
  rawLog: string[];
  lastMessage: EngineMessage | null;
  spawnEngine: () => Promise<void>;
  sendCommand: (cmd: string) => Promise<void>;
  killEngine: () => Promise<void>;
  onMessage: (handler: (msg: EngineMessage) => void) => void;
}

export function useEngine(): UseEngineResult {
  const [isConnected, setIsConnected] = useState(false);
  const [rawLog, setRawLog] = useState<string[]>([]);
  const [lastMessage, setLastMessage] = useState<EngineMessage | null>(null);
  const messageHandlerRef = useRef<((msg: EngineMessage) => void) | null>(null);
  const engineGenRef = useRef(0);

  useEffect(() => {
    const unlisten = listen<EngineOutputPayload>('engine-output', (event) => {
      const { line, gen } = event.payload;

      // Discard output from a previous engine process
      if (gen !== engineGenRef.current) return;

      setRawLog((prev) => {
        const next = [...prev, line];
        if (next.length > MAX_LOG_LINES) {
          return next.slice(DROP_LINES);
        }
        return next;
      });

      const msg = parseEngineLine(line);
      setLastMessage(msg);

      if (messageHandlerRef.current) {
        messageHandlerRef.current(msg);
      }
    });

    const unlistenExit = listen<number>('engine-exit', (event) => {
      const gen = event.payload;
      if (gen === engineGenRef.current) {
        setIsConnected(false);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenExit.then((fn) => fn());
    };
  }, []);

  const spawnEngine = useCallback(async () => {
    try {
      const gen = await invoke<number>('spawn_engine');
      engineGenRef.current = gen;
      setIsConnected(true);
      // Freyja protocol init sequence
      await invoke('send_command', { cmd: 'freyja' });
      await invoke('send_command', { cmd: 'isready' });
    } catch (err) {
      console.error('[Freyja] Failed to spawn engine:', err);
      setIsConnected(false);
    }
  }, []);

  const sendCommand = useCallback(async (cmd: string) => {
    await invoke('send_command', { cmd });
  }, []);

  const killEngine = useCallback(async () => {
    try {
      await invoke('kill_engine');
    } finally {
      setIsConnected(false);
    }
  }, []);

  const onMessage = useCallback((handler: (msg: EngineMessage) => void) => {
    messageHandlerRef.current = handler;
  }, []);

  return useMemo(() => ({
    isConnected,
    rawLog,
    lastMessage,
    spawnEngine,
    sendCommand,
    killEngine,
    onMessage,
  }), [isConnected, rawLog, lastMessage, spawnEngine, sendCommand, killEngine, onMessage]);
}
