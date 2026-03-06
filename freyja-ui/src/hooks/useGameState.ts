// Core game state hook for Freyja UI.
// Manages board display, move history, auto-play, and engine interaction.
// UI owns ZERO game logic — engine is the sole authority.

import { useState, useCallback, useRef, useEffect } from 'react';
import type { Piece, Player, PlayerStatus } from '../types/board';
import { PLAYERS, BOARD_SIZE, TOTAL_SQUARES } from '../types/board';
import type { EngineMessage, InfoData } from '../types/protocol';
import {
  startingPosition,
  squareName,
  parseSquare,
  fileOf,
  rankOf,
  squareFrom,
} from '../lib/board-constants';
import type { UseEngineResult } from './useEngine';

export type SlotConfig = Record<Player, 'human' | 'engine'>;

export interface MoveEntry {
  move: string;
  player: Player;
  info: InfoData | null;
}

export interface UseGameStateResult {
  board: (Piece | null)[];
  currentPlayer: Player;
  scores: [number, number, number, number];
  moveHistory: MoveEntry[];
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  isGameOver: boolean;
  latestInfo: InfoData | null;
  playerStatus: Record<Player, PlayerStatus>;
  slotConfig: SlotConfig;
  autoPlay: boolean;
  engineDelay: number;
  isPaused: boolean;
  setSlotConfig: (config: SlotConfig) => void;
  setAutoPlay: (on: boolean) => void;
  setEngineDelay: (ms: number) => void;
  onSquareClick: (sq: number) => void;
  newGame: () => void;
  requestEngineMove: () => void;
  togglePause: () => void;
  undo: () => void;
  pendingPromotion: { from: number; to: number } | null;
  onPromotionSelect: (piece: 'q' | 'r' | 'b' | 'n') => void;
  onPromotionCancel: () => void;
}

export function useGameState(engine: UseEngineResult): UseGameStateResult {
  // --- Display state ---
  const [board, setBoard] = useState<(Piece | null)[]>(() => startingPosition());
  const [currentPlayer, setCurrentPlayer] = useState<Player>('Red');
  const [scores, setScores] = useState<[number, number, number, number]>([0, 0, 0, 0]);
  const [moveHistory, setMoveHistory] = useState<MoveEntry[]>([]);
  const [selectedSquare, setSelectedSquare] = useState<number | null>(null);
  const [lastMoveFrom, setLastMoveFrom] = useState<number | null>(null);
  const [lastMoveTo, setLastMoveTo] = useState<number | null>(null);
  const [isGameOver, setIsGameOver] = useState(false);
  const [latestInfo, setLatestInfo] = useState<InfoData | null>(null);
  const [playerStatus, setPlayerStatus] = useState<Record<Player, PlayerStatus>>({
    Red: 'Active', Blue: 'Active', Yellow: 'Active', Green: 'Active',
  });
  const [pendingPromotion, setPendingPromotion] = useState<{ from: number; to: number } | null>(null);

  // --- Controls state ---
  const [slotConfig, setSlotConfig] = useState<SlotConfig>({
    Red: 'human', Blue: 'engine', Yellow: 'engine', Green: 'engine',
  });
  const [autoPlay, setAutoPlay] = useState(false);
  const [engineDelay, setEngineDelay] = useState(500);
  const [isPaused, setIsPaused] = useState(false);

  // --- Refs for async access ---
  const boardRef = useRef(board);
  const moveListRef = useRef<string[]>([]);
  const currentPlayerRef = useRef(currentPlayer);
  const awaitingBestmoveRef = useRef(false);
  const autoPlayRef = useRef(autoPlay);
  const engineDelayRef = useRef(engineDelay);
  const isPausedRef = useRef(isPaused);
  const ignoreNextBestmoveRef = useRef(false);
  const eliminatedPlayersRef = useRef(new Set<Player>());
  const latestInfoRef = useRef<InfoData | null>(null);
  const slotConfigRef = useRef(slotConfig);
  const gameGenRef = useRef(0);

  // Keep refs in sync
  useEffect(() => { boardRef.current = board; }, [board]);
  useEffect(() => { currentPlayerRef.current = currentPlayer; }, [currentPlayer]);
  useEffect(() => { autoPlayRef.current = autoPlay; }, [autoPlay]);
  useEffect(() => { engineDelayRef.current = engineDelay; }, [engineDelay]);
  useEffect(() => { isPausedRef.current = isPaused; }, [isPaused]);
  useEffect(() => { slotConfigRef.current = slotConfig; }, [slotConfig]);

  // --- Move string parser (greedy longest-match for multi-digit ranks) ---
  const parseMoveString = useCallback((moveStr: string): { from: number; to: number; promo: string } | null => {
    for (let fromLen = 3; fromLen >= 2; fromLen--) {
      const tryFrom = parseSquare(moveStr.slice(0, fromLen));
      if (tryFrom === -1) continue;
      const remaining = moveStr.slice(fromLen);
      for (let toLen = 3; toLen >= 2; toLen--) {
        if (toLen > remaining.length) continue;
        const tryTo = parseSquare(remaining.slice(0, toLen));
        if (tryTo === -1) continue;
        return { from: tryFrom, to: tryTo, promo: remaining.slice(toLen) };
      }
    }
    return null;
  }, []);

  // --- Board display update (heuristic — no game logic validation) ---
  const applyMoveToBoard = useCallback((moveStr: string, prev: (Piece | null)[]): (Piece | null)[] => {
    const next = [...prev];

    const parsed = parseMoveString(moveStr);
    if (!parsed) return next;
    const { from: fromSq, to: toSq, promo: promoChar } = parsed;

    const piece = next[fromSq];
    if (!piece) return next;

    // Basic move: remove from source, place at dest
    next[fromSq] = null;
    next[toSq] = piece;

    // Promotion: move string ends with q/r/b/n
    if (promoChar && piece.pieceType === 'Pawn') {
      const promoMap: Record<string, Piece['pieceType']> = {
        q: 'PromotedQueen', r: 'Rook', b: 'Bishop', n: 'Knight',
      };
      if (promoMap[promoChar]) {
        next[toSq] = { ...piece, pieceType: promoMap[promoChar] };
      }
    }

    // Castling detection: king moves 2+ squares on same rank or file
    if (piece.pieceType === 'King') {
      const fromFile = fileOf(fromSq);
      const fromRank = rankOf(fromSq);
      const toFile = fileOf(toSq);
      const toRank = rankOf(toSq);

      const fileDist = Math.abs(toFile - fromFile);
      const rankDist = Math.abs(toRank - fromRank);

      // Horizontal castling (Red/Yellow)
      if (fileDist >= 2 && rankDist === 0) {
        const dir = toFile > fromFile ? 1 : -1;
        // Find rook in that direction
        let rookFile = dir > 0 ? BOARD_SIZE - 1 : 0;
        for (let f = toFile + dir; f >= 0 && f < BOARD_SIZE; f += dir) {
          const sq = squareFrom(f, fromRank);
          if (next[sq]?.pieceType === 'Rook' && next[sq]?.owner === piece.owner) {
            rookFile = f;
            break;
          }
        }
        const rookSq = squareFrom(rookFile, fromRank);
        const rookDestSq = squareFrom(toFile - dir, fromRank);
        if (next[rookSq]?.pieceType === 'Rook') {
          next[rookDestSq] = next[rookSq];
          next[rookSq] = null;
        }
      }

      // Vertical castling (Blue/Green)
      if (rankDist >= 2 && fileDist === 0) {
        const dir = toRank > fromRank ? 1 : -1;
        let rookRank = dir > 0 ? BOARD_SIZE - 1 : 0;
        for (let r = toRank + dir; r >= 0 && r < BOARD_SIZE; r += dir) {
          const sq = squareFrom(fromFile, r);
          if (next[sq]?.pieceType === 'Rook' && next[sq]?.owner === piece.owner) {
            rookRank = r;
            break;
          }
        }
        const rookSq = squareFrom(fromFile, rookRank);
        const rookDestSq = squareFrom(fromFile, toRank - dir);
        if (next[rookSq]?.pieceType === 'Rook') {
          next[rookDestSq] = next[rookSq];
          next[rookSq] = null;
        }
      }
    }

    // En passant detection: pawn moves diagonally to empty square
    if (piece.pieceType === 'Pawn') {
      const fromFile = fileOf(fromSq);
      const fromRank = rankOf(fromSq);
      const toFile = fileOf(toSq);
      const toRank = rankOf(toSq);
      const isDiagonal = fromFile !== toFile && fromRank !== toRank;

      if (isDiagonal && prev[toSq] === null) {
        // Captured pawn is at the intersection of the mover's rank/file and the target's
        const cand1 = squareFrom(toFile, fromRank);
        const cand2 = squareFrom(fromFile, toRank);
        if (prev[cand1]?.pieceType === 'Pawn' && prev[cand1]?.owner !== piece.owner) {
          next[cand1] = null;
        } else if (prev[cand2]?.pieceType === 'Pawn' && prev[cand2]?.owner !== piece.owner) {
          next[cand2] = null;
        }
      }
    }

    return next;
  }, [parseMoveString]);

  // --- Turn advancement ---
  const advancePlayer = useCallback((from: Player): Player => {
    let idx = PLAYERS.indexOf(from);
    for (let i = 0; i < 4; i++) {
      idx = (idx + 1) % 4;
      if (!eliminatedPlayersRef.current.has(PLAYERS[idx])) {
        return PLAYERS[idx];
      }
    }
    return from; // All eliminated (shouldn't happen)
  }, []);

  // --- Send go command (single entry point with guard) ---
  const sendGoFromRef = useCallback(() => {
    if (awaitingBestmoveRef.current) {
      console.warn('go blocked: already awaiting bestmove');
      return;
    }
    awaitingBestmoveRef.current = true;

    const moves = [...moveListRef.current];
    const posCmd = moves.length > 0
      ? `position startpos moves ${moves.join(' ')}`
      : 'position startpos';

    engine.sendCommand(posCmd).then(() => {
      engine.sendCommand('go depth 1');
    }).catch(() => {
      awaitingBestmoveRef.current = false;
      autoPlayRef.current = false;
      setAutoPlay(false);
    });
  }, [engine]);

  // --- Public request to trigger engine move ---
  const requestEngineMove = useCallback(() => {
    if (isPausedRef.current) return;
    sendGoFromRef();
  }, [sendGoFromRef]);

  // --- Maybe chain next engine move ---
  // Always chains engine→engine. Stops at human unless autoPlay is on.
  const maybeChainEngineMove = useCallback((nextPlayer: Player) => {
    if (isPausedRef.current) return;
    if (slotConfigRef.current[nextPlayer] !== 'engine') return;

    const gen = gameGenRef.current;
    const delay = autoPlayRef.current ? engineDelayRef.current : 100;
    setTimeout(() => {
      // Bail if game was reset since this timeout was queued
      if (gen !== gameGenRef.current) return;
      if (!isPausedRef.current && slotConfigRef.current[nextPlayer] === 'engine') {
        sendGoFromRef();
      }
    }, delay);
  }, [sendGoFromRef]);

  // --- Handle engine messages ---
  useEffect(() => {
    engine.onMessage((msg: EngineMessage) => {
      if (msg.type === 'bestmove') {
        // Stale move discard
        if (ignoreNextBestmoveRef.current) {
          ignoreNextBestmoveRef.current = false;
          awaitingBestmoveRef.current = false;
          return;
        }

        awaitingBestmoveRef.current = false;
        const gen = gameGenRef.current;

        if (msg.move === null) {
          setIsGameOver(true);
          setAutoPlay(false);
          return;
        }

        // Snapshot before mutation
        const movingPlayer = currentPlayerRef.current;
        const prevBoard = boardRef.current;
        const infoSnapshot = latestInfoRef.current ? { ...latestInfoRef.current } : null;

        // Update move list
        moveListRef.current = [...moveListRef.current, msg.move];

        // Update display board
        const newBoard = applyMoveToBoard(msg.move, prevBoard);
        setBoard(newBoard);
        boardRef.current = newBoard;

        // Parse from/to for highlighting
        const moveParsed = parseMoveString(msg.move);
        setLastMoveFrom(moveParsed ? moveParsed.from : null);
        setLastMoveTo(moveParsed ? moveParsed.to : null);

        // Add to move history
        setMoveHistory((prev) => [...prev, {
          move: msg.move!,
          player: movingPlayer,
          info: infoSnapshot,
        }]);

        // Advance turn (use nextturn from engine if it comes, otherwise local)
        const nextPlayer = advancePlayer(movingPlayer);
        setCurrentPlayer(nextPlayer);
        currentPlayerRef.current = nextPlayer;

        setSelectedSquare(null);

        // Chain auto-play
        maybeChainEngineMove(nextPlayer);
      }

      if (msg.type === 'nextturn') {
        // Engine's authoritative turn — override local computation
        setCurrentPlayer(msg.player);
        currentPlayerRef.current = msg.player;
      }

      if (msg.type === 'info') {
        setLatestInfo(msg.data);
        latestInfoRef.current = msg.data;
        if (msg.data.scores) {
          setScores(msg.data.scores);
        }
      }

      if (msg.type === 'eliminated') {
        eliminatedPlayersRef.current.add(msg.player);
        setPlayerStatus((prev) => ({ ...prev, [msg.player]: 'Eliminated' as PlayerStatus }));
      }
    });
  }, [engine, applyMoveToBoard, advancePlayer, maybeChainEngineMove]);

  // --- Square click handler ---
  const onSquareClick = useCallback((sq: number) => {
    if (isGameOver || awaitingBestmoveRef.current) return;

    const currentBoard = boardRef.current;
    const player = currentPlayerRef.current;

    // If a human slot's turn
    if (slotConfigRef.current[player] !== 'human') return;

    if (selectedSquare === null) {
      // Select a piece
      const piece = currentBoard[sq];
      if (piece && piece.owner === player) {
        setSelectedSquare(sq);
      }
    } else {
      if (sq === selectedSquare) {
        // Deselect
        setSelectedSquare(null);
        return;
      }

      const piece = currentBoard[sq];
      if (piece && piece.owner === player) {
        // Select a different piece
        setSelectedSquare(sq);
        return;
      }

      // Attempt move from selectedSquare to sq
      const fromName = squareName(selectedSquare);
      const toName = squareName(sq);

      // Check for promotion
      const movingPiece = currentBoard[selectedSquare];
      if (movingPiece?.pieceType === 'Pawn') {
        const toRank = rankOf(sq);
        const toFile = fileOf(sq);
        // Freyja FFA promotion ranks per 4PC_RULES_REFERENCE:
        // Red promotes at rank 8 (display rank 9), Blue at file 8, Yellow at rank 5 (display rank 6), Green at file 5
        const isPromo =
          (movingPiece.owner === 'Red' && toRank === 8) ||
          (movingPiece.owner === 'Yellow' && toRank === 5) ||
          (movingPiece.owner === 'Blue' && toFile === 8) ||
          (movingPiece.owner === 'Green' && toFile === 5);

        if (isPromo) {
          setPendingPromotion({ from: selectedSquare, to: sq });
          return;
        }
      }

      // Send move to engine for validation
      submitMove(fromName + toName);
    }
  }, [selectedSquare, isGameOver]);

  const submitMove = useCallback((moveStr: string) => {
    const moves = [...moveListRef.current, moveStr];
    const posCmd = `position startpos moves ${moves.join(' ')}`;

    engine.sendCommand(posCmd).then(() => {
      return engine.sendCommand('isready');
    }).then(() => {
      // If we get readyok, the move was accepted
      // The move is now pending — we'll get readyok back
    }).catch(() => {
      // Move rejected
      setSelectedSquare(null);
    });

    // We need to listen for readyok to confirm the move was valid
    // This is handled via the engine message handler
    // For now, optimistically apply it
    moveListRef.current = moves;

    const prevBoard = boardRef.current;
    const newBoard = applyMoveToBoard(moveStr, prevBoard);
    setBoard(newBoard);
    boardRef.current = newBoard;

    const submitParsed = parseMoveString(moveStr);
    setLastMoveFrom(submitParsed ? submitParsed.from : null);
    setLastMoveTo(submitParsed ? submitParsed.to : null);
    setSelectedSquare(null);

    const movingPlayer = currentPlayerRef.current;
    setMoveHistory((prev) => [...prev, {
      move: moveStr,
      player: movingPlayer,
      info: null,
    }]);

    const nextPlayer = advancePlayer(movingPlayer);
    setCurrentPlayer(nextPlayer);
    currentPlayerRef.current = nextPlayer;

    // If next player is engine, trigger engine move
    if (slotConfigRef.current[nextPlayer] === 'engine') {
      const gen = gameGenRef.current;
      setTimeout(() => {
        if (gen === gameGenRef.current) sendGoFromRef();
      }, engineDelayRef.current);
    }
  }, [engine, applyMoveToBoard, advancePlayer, sendGoFromRef]);

  // --- Promotion handlers ---
  const onPromotionSelect = useCallback((piece: 'q' | 'r' | 'b' | 'n') => {
    if (!pendingPromotion) return;
    const fromName = squareName(pendingPromotion.from);
    const toName = squareName(pendingPromotion.to);
    setPendingPromotion(null);
    submitMove(fromName + toName + piece);
  }, [pendingPromotion, submitMove]);

  const onPromotionCancel = useCallback(() => {
    setPendingPromotion(null);
  }, []);

  // --- New game ---
  const newGame = useCallback(() => {
    // Increment game generation — all queued timeouts with old gen will bail out
    gameGenRef.current += 1;

    // Stop auto-play first to prevent queued timeouts from re-triggering
    setAutoPlay(false);
    autoPlayRef.current = false;

    // If a search is in flight, discard the response
    if (awaitingBestmoveRef.current) {
      ignoreNextBestmoveRef.current = true;
      engine.sendCommand('stop');
    }
    awaitingBestmoveRef.current = false;

    const initialBoard = startingPosition();
    setBoard(initialBoard);
    boardRef.current = initialBoard;
    setCurrentPlayer('Red');
    currentPlayerRef.current = 'Red';
    setScores([0, 0, 0, 0]);
    setMoveHistory([]);
    moveListRef.current = [];
    setSelectedSquare(null);
    setLastMoveFrom(null);
    setLastMoveTo(null);
    setIsGameOver(false);
    setLatestInfo(null);
    latestInfoRef.current = null;
    setPlayerStatus({ Red: 'Active', Blue: 'Active', Yellow: 'Active', Green: 'Active' });
    eliminatedPlayersRef.current = new Set();
    setSlotConfig({ Red: 'human', Blue: 'engine', Yellow: 'engine', Green: 'engine' });
    slotConfigRef.current = { Red: 'human', Blue: 'engine', Yellow: 'engine', Green: 'engine' };
    setPendingPromotion(null);
    setIsPaused(false);
    isPausedRef.current = false;

    engine.sendCommand('position startpos');
    engine.sendCommand('isready');
  }, [engine]);

  // --- Undo ---
  const undo = useCallback(() => {
    if (awaitingBestmoveRef.current) return;
    if (moveListRef.current.length === 0) return;

    // Pop last move
    const newMoves = moveListRef.current.slice(0, -1);
    moveListRef.current = newMoves;

    // Replay from startpos to rebuild board
    let replayBoard = startingPosition();
    for (const m of newMoves) {
      replayBoard = applyMoveToBoard(m, replayBoard);
    }
    setBoard(replayBoard);
    boardRef.current = replayBoard;

    // Remove last move from history
    setMoveHistory((prev) => prev.slice(0, -1));

    // Determine current player after undo
    // Simple: Red starts, each move advances
    let player: Player = 'Red';
    for (let i = 0; i < newMoves.length; i++) {
      player = advancePlayer(player);
    }
    setCurrentPlayer(player);
    currentPlayerRef.current = player;

    setSelectedSquare(null);
    setLastMoveFrom(null);
    setLastMoveTo(null);

    // Sync engine state
    if (newMoves.length > 0) {
      engine.sendCommand(`position startpos moves ${newMoves.join(' ')}`);
    } else {
      engine.sendCommand('position startpos');
    }
    engine.sendCommand('isready');
  }, [engine, applyMoveToBoard, advancePlayer]);

  // --- Pause toggle ---
  const togglePause = useCallback(() => {
    setIsPaused((prev) => {
      const next = !prev;
      isPausedRef.current = next;
      if (!next && autoPlayRef.current) {
        // Resuming — kick off next move
        const player = currentPlayerRef.current;
        if (slotConfigRef.current[player] === 'engine') {
          setTimeout(() => sendGoFromRef(), engineDelayRef.current);
        }
      }
      return next;
    });
  }, [sendGoFromRef]);

  // --- Auto-play start: trigger first engine move when turned on or slots change ---
  useEffect(() => {
    if (autoPlay && !isPaused && !awaitingBestmoveRef.current) {
      const player = currentPlayerRef.current;
      if (slotConfigRef.current[player] === 'engine') {
        sendGoFromRef();
      }
    }
  }, [autoPlay, isPaused, slotConfig, sendGoFromRef]);

  return {
    board,
    currentPlayer,
    scores,
    moveHistory,
    selectedSquare,
    lastMoveFrom,
    lastMoveTo,
    isGameOver,
    latestInfo,
    playerStatus,
    slotConfig,
    autoPlay,
    engineDelay,
    isPaused,
    setSlotConfig,
    setAutoPlay,
    setEngineDelay,
    onSquareClick,
    newGame,
    requestEngineMove,
    togglePause,
    undo,
    pendingPromotion,
    onPromotionSelect,
    onPromotionCancel,
  };
}
