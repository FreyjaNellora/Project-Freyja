# Downstream Log — Stage 4: Freyja Protocol

**Author:** Session 6
**Date:** 2026-03-06

---

## Must-Know

1. **Protocol output goes to stdout ONLY.** Tracing/debug goes to stderr via `tracing-subscriber`. Never write debug info to stdout.
2. **`Protocol<W: Write>` is generic** over the output writer. Tests use `Vec<u8>`, production uses `BufWriter<StdoutLock>`.
3. **`stop` is a no-op** in Stage 4. When search becomes async (Stage 7), this must be made functional.
4. **Go command returns first legal move** as a stub. Stage 7 replaces this with actual search.
5. **Elimination events are detected via status diffing** — snapshot `player_status` for all 4 players before `apply_move`, compare after. This happens in `apply_move_with_events()`.

---

## API Contracts

### Protocol Struct (protocol/mod.rs)
```rust
pub struct Protocol<W: Write> {
    // Private fields — consumers interact via run()
}

impl<W: Write> Protocol<W> {
    pub fn new(output: W) -> Self;
    pub fn run(&mut self, input: impl BufRead);
}
```

### Protocol Version
```
freyja v1.0 maxn-beam-mcts
```
Sent on startup and in response to `freyja` command.

### Command Set (Incoming)
```
freyja                                    → identify
isready                                   → readyok
position startpos [moves m1 m2 ...]       → set position
position fen4 <fen> [moves m1 m2 ...]     → set position from FEN4
go [depth D] [nodes N] [movetime MS] [infinite] → trigger search
stop                                      → halt search (no-op Stage 4)
quit                                      → exit
setoption name <Name> value <Value>       → set option
```

### Options
| Name | Values | Default | Notes |
|------|--------|---------|-------|
| GameMode | FreeForAll | FreeForAll | Only mode until Stage 18 |
| BeamWidth | positive integer | 15 | Placeholder until Stage 7 |
| MaxRounds | 0 (unlimited) or positive | unlimited | Diagnostic auto-stop |
| LogFile | file path or "none" | none (disabled) | Zero-cost when disabled |

### Response Set (Outgoing)
```
freyja v1.0 maxn-beam-mcts               → identification
readyok                                   → ready response
bestmove <move>                           → best move found
bestmove (none)                           → no legal moves / game over
info depth D score red R blue B yellow Y green G nodes N nps NPS pv M1 M2 ...
info string <text>                        → diagnostic message
info string eliminated <Color> <reason>   → elimination event
info string nextturn <Color>              → turn change
info string error: <text>                 → error message
```

### Move String Format
- Long algebraic: `{from}{to}[promo]`
- Files: `a`-`n`, Ranks: `1`-`14`
- Examples: `d2d4`, `a7a10`, `d7d8q`
- Promotion chars: `q` (PromotedQueen), `r` (Rook), `b` (Bishop), `n` (Knight)
- Parsing: `parse_move_str(s, &legal_moves) -> Result<Move, String>`

### LogFile Format
```
[epoch_secs.millis] > incoming_command
[epoch_secs.millis] < outgoing_response
```

---

## Known Limitations

1. **No actual search.** Go returns first legal move. Stage 7 adds Max^n.
2. **No time management.** `movetime` parameter is parsed but ignored. Stage 13.
3. **No ponder mode.** Not planned.
4. **`stop` is a no-op.** Search is synchronous in Stage 4.
5. **Elimination reason inference** is approximate — uses status diff, not explicit reason tracking from GameState. May not distinguish checkmate from stalemate in all cases.
6. **LogFile timestamps** are Unix epoch, not human-readable.
7. **MaxRounds** uses `ply_count / 4` which slightly over-counts when players are eliminated. Sufficient for diagnostics.

---

## Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Protocol startup | <1ms | Header output only |
| isready→readyok | <1ms | No computation |
| position startpos | <1ms | Board construction |
| go depth 1 (stub) | <1ms | Returns first legal move |
| LogFile overhead (disabled) | ~0ns | Single branch prediction |

---

## Open Questions

1. **How should Stage 5 (UI) handle elimination events?** Currently `info string eliminated Red checkmate` — UI should extract the color as the first token after "eliminated" and ignore additional tokens (Odin lesson).
2. **Should go command apply the bestmove to game state?** Currently it does NOT — it only reports the move. The UI must send `position ... moves ... <bestmove>` to advance the game. This is the correct UCI-like pattern.

---

## Reasoning

- **Generic `Protocol<W>`** chosen over trait objects for zero-cost abstraction in production and easy testing.
- **Separate notation.rs** for move parsing because 14x14 variable-length notation is complex enough to warrant isolation and dedicated tests.
- **`apply_move_with_events`** wraps GameState::apply_move with status diffing to emit elimination/nextturn events. This keeps event detection in the protocol layer where it belongs (GameState should not know about protocol messages).
- **LogFile as enum** (not Option<BufWriter>) for clearer semantics and `#[derive(Default)]`.
- **tracing-subscriber added** (not just tracing) because main.rs needs to initialize the subscriber on stderr. This is the standard approach.
