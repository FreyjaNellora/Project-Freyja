// Protocol parser tests — verifies parsing matches freyja-engine output.rs formats.

import { describe, it, expect } from 'vitest';
import { parseEngineLine } from './protocol-parser';

describe('parseEngineLine', () => {
  it('parses header', () => {
    const msg = parseEngineLine('freyja v1.0 maxn-beam-mcts');
    expect(msg.type).toBe('header');
    if (msg.type === 'header') {
      expect(msg.version).toBe('v1.0 maxn-beam-mcts');
    }
  });

  it('parses readyok', () => {
    expect(parseEngineLine('readyok').type).toBe('readyok');
  });

  it('parses bestmove with move', () => {
    const msg = parseEngineLine('bestmove d2d4');
    expect(msg.type).toBe('bestmove');
    if (msg.type === 'bestmove') {
      expect(msg.move).toBe('d2d4');
    }
  });

  it('parses bestmove (none)', () => {
    const msg = parseEngineLine('bestmove (none)');
    expect(msg.type).toBe('bestmove');
    if (msg.type === 'bestmove') {
      expect(msg.move).toBeNull();
    }
  });

  it('parses info with all fields', () => {
    const msg = parseEngineLine(
      'info depth 5 score red 150 blue 120 yellow 100 green 140 nodes 50000 nps 100000'
    );
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBe(5);
      expect(msg.data.scores).toEqual([150, 120, 100, 140]);
      expect(msg.data.nodes).toBe(50000);
      expect(msg.data.nps).toBe(100000);
    }
  });

  it('parses info depth only', () => {
    const msg = parseEngineLine('info depth 3');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBe(3);
      expect(msg.data.scores).toBeUndefined();
    }
  });

  it('parses info with pv', () => {
    const msg = parseEngineLine('info depth 1 pv d2d3 b4b5');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.pv).toEqual(['d2d3', 'b4b5']);
    }
  });

  it('parses eliminated event (extension-tolerant)', () => {
    const msg = parseEngineLine('info string eliminated Red checkmate');
    expect(msg.type).toBe('eliminated');
    if (msg.type === 'eliminated') {
      expect(msg.player).toBe('Red');
      expect(msg.reason).toBe('checkmate');
    }
  });

  it('parses eliminated with extra tokens', () => {
    const msg = parseEngineLine('info string eliminated Blue stalemate extra tokens');
    expect(msg.type).toBe('eliminated');
    if (msg.type === 'eliminated') {
      expect(msg.player).toBe('Blue');
      expect(msg.reason).toBe('stalemate extra tokens');
    }
  });

  it('parses nextturn', () => {
    const msg = parseEngineLine('info string nextturn Blue');
    expect(msg.type).toBe('nextturn');
    if (msg.type === 'nextturn') {
      expect(msg.player).toBe('Blue');
    }
  });

  it('parses error', () => {
    const msg = parseEngineLine("info string error: unknown command 'bogus'");
    expect(msg.type).toBe('error');
    if (msg.type === 'error') {
      expect(msg.message).toBe("unknown command 'bogus'");
    }
  });

  it('parses generic info string', () => {
    const msg = parseEngineLine('info string some diagnostic');
    expect(msg.type).toBe('info_string');
    if (msg.type === 'info_string') {
      expect(msg.message).toBe('some diagnostic');
    }
  });

  it('parses unknown lines', () => {
    const msg = parseEngineLine('something unexpected');
    expect(msg.type).toBe('unknown');
  });

  it('handles empty string', () => {
    const msg = parseEngineLine('');
    expect(msg.type).toBe('unknown');
  });
});
