// Protocol types for Freyja engine communication.

import type { Player } from './board';

/** Search info data from engine (all fields optional). */
export interface InfoData {
  depth?: number;
  scores?: [number, number, number, number];
  nodes?: number;
  nps?: number;
  pv?: string[];
}

/** Parsed engine message types. */
export type EngineMessage =
  | { type: 'header'; version: string }
  | { type: 'readyok' }
  | { type: 'bestmove'; move: string | null }
  | { type: 'info'; data: InfoData }
  | { type: 'eliminated'; player: Player; reason: string }
  | { type: 'nextturn'; player: Player }
  | { type: 'error'; message: string }
  | { type: 'info_string'; message: string }
  | { type: 'unknown'; raw: string };
