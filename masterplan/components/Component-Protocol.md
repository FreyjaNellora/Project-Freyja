# Component: Protocol

**Stage Introduced:** Stage 4
**Last Updated:** 2026-03-06
**Module:** `freyja_engine::protocol`

---

## Purpose

Text-based stdin/stdout communication protocol for 4-player chess. Similar to UCI but with 4-vector scores and 4-player game state. Handles command parsing, position setup, search triggering (stub in Stage 4), info output, engine options, diagnostic logging, and move notation for the 14x14 board.

## Public API

```rust
pub struct Protocol<W: Write> { /* private */ }

impl<W: Write> Protocol<W> {
    pub fn new(output: W) -> Self;
    pub fn run(&mut self, input: impl BufRead);
}
```

## Submodules

| Module | Purpose |
|--------|---------|
| `commands.rs` | Command enum, GoParams struct |
| `parse.rs` | Tokenizer, `parse_command(line) -> Option<Command>` |
| `notation.rs` | Move string parsing, `parse_move_str(s, legal_moves) -> Result<Move>` |
| `output.rs` | Response formatting (bestmove, info, error) |
| `options.rs` | EngineOptions struct, setoption handling |
| `logfile.rs` | LogFile zero-cost toggle |

## Internal Design

- **Generic `Protocol<W: Write>`** for testability. Production: `BufWriter<StdoutLock>`. Tests: `Vec<u8>`.
- **Command dispatch:** parse → match → handler. Unknown commands produce error messages, never crash.
- **Position handling:** `startpos` or `fen4` + optional `moves` clause. Moves parsed via `parse_move_str` matching against legal moves.
- **Stub search:** Go returns first legal move. Real search added in Stage 7.
- **Elimination detection:** Snapshot `player_status[4]` before apply_move, diff after. Emit `info string eliminated {Color} {reason}`.
- **LogFile:** `#[derive(Default)]` enum — Disabled (zero-cost) or Enabled (BufWriter). Timestamps are Unix epoch.
- **MaxRounds:** Track ply_count, auto-stop when `ply_count / 4 >= max_rounds`.
- **Tracing:** All commands/responses logged via `tracing::debug!`. Subscriber writes to stderr in main.rs.

## Performance Characteristics

- Protocol overhead: negligible (<1ms per command)
- LogFile disabled: single branch prediction per message
- No allocations in hot path (protocol I/O is not hot path)

## Known Limitations

- `stop` is a no-op (synchronous search). Must be functional in Stage 7.
- Elimination reason inferred from status diff, not explicit. May not distinguish checkmate from stalemate.
- LogFile timestamps are Unix epoch, not human-readable.
- Go does not apply the bestmove to game state — UI must send updated position.

## Dependencies

- **Consumes:** [[Component-Board]] (from_fen4), [[Component-MoveGen]] (Move, Square), [[Component-GameState]] (all accessors)
- **Consumed By:** Stage 5 (UI Shell), main.rs

---

**Related:** [[MASTERPLAN]], [[audit_log_stage_04]], [[downstream_log_stage_04]]
