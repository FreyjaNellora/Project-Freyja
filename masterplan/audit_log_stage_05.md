# Audit Log — Stage 5: UI Shell

## Pre-Audit

**Date:** 2026-03-06
**Baseline:** Stage 4 complete (275 engine tests pass), `cargo build` clean, `cargo test` green.

### Scope

Build a Tauri v2 desktop UI that:
- Renders the 14×14 4PC board with SVG (46px squares, corner exclusion)
- Displays all 64 starting pieces with correct K/Q placement per 4PC_RULES_REFERENCE
- Communicates with freyja-engine via child process IPC (stdin/stdout)
- Supports click-to-move with engine-side validation
- Auto-play with configurable engine delay and slot config (human/engine per player)
- Analysis panel (depth, scores, nodes, NPS, PV)
- Communication log with manual command input
- Game controls (new game, undo, pause/resume)

### Architecture

```
React UI (display only)  ←→  useEngine hook  ←→  Tauri Backend (Rust)  ←→  freyja engine process
```

Three layers: React components own zero game logic. Protocol parser translates engine stdout. Tauri Rust backend manages engine child process with generation tagging.

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| SVG board (not CSS grid) | Proven in Odin, supports zoom, coordinate labels, highlights |
| Generation tagging on engine events | Prevents ghost moves from killed processes |
| Dual state (useState + useRef) | React batching means refs needed for current values in async callbacks |
| Single `go` entry point with guard | Prevents double-send bugs |
| Full move history replay | Every `position` command sends ALL moves — no desync possible |
| Extension-tolerant parsing | Future-proof: extract needed tokens, ignore trailing |

### Files Created

**Tauri Backend (Rust):**
- `freyja-ui/src-tauri/src/main.rs` — Windows console suppression, entry point
- `freyja-ui/src-tauri/src/lib.rs` — Tauri command handlers (spawn_engine, send_command, kill_engine)
- `freyja-ui/src-tauri/src/engine.rs` — Engine process manager with generation tagging
- `freyja-ui/src-tauri/Cargo.toml` — Tauri v2 dependencies
- `freyja-ui/src-tauri/tauri.conf.json` — Window config (1280×800), dev server
- `freyja-ui/src-tauri/capabilities/default.json` — Permissions
- `freyja-ui/src-tauri/build.rs` — Tauri build script

**TypeScript Types:**
- `freyja-ui/src/types/board.ts` — Player, PieceType, Piece, PlayerStatus, board constants
- `freyja-ui/src/types/protocol.ts` — EngineMessage union, InfoData

**Libraries:**
- `freyja-ui/src/lib/board-constants.ts` — Square math, validity, starting position, colors, symbols
- `freyja-ui/src/lib/protocol-parser.ts` — Engine output line parser

**Hooks:**
- `freyja-ui/src/hooks/useEngine.ts` — Engine IPC with generation filtering
- `freyja-ui/src/hooks/useGameState.ts` — Core game state (~350 lines)

**Components:**
- `freyja-ui/src/components/BoardDisplay.tsx` — SVG 14×14 board with zoom
- `freyja-ui/src/components/BoardSquare.tsx` — Individual square with highlights
- `freyja-ui/src/components/PieceIcon.tsx` — Unicode pieces with player colors
- `freyja-ui/src/components/PromotionDialog.tsx` — Promotion piece selector
- `freyja-ui/src/components/GameControls.tsx` — Turn, scores, slots, controls
- `freyja-ui/src/components/GameLog.tsx` — Move history display
- `freyja-ui/src/components/AnalysisPanel.tsx` — Search info display
- `freyja-ui/src/components/CommunicationLog.tsx` — Raw protocol log
- `freyja-ui/src/components/StatusBar.tsx` — Connection indicator

**Tests:**
- `freyja-ui/src/lib/board-constants.test.ts` — 34 tests: coordinates, validity, starting position
- `freyja-ui/src/lib/protocol-parser.test.ts` — 14 tests: all message types

**Layout & Styles:**
- `freyja-ui/src/App.tsx` — 3-panel layout wiring all components
- `freyja-ui/src/App.css` — Dark theme (~400 lines)
- `freyja-ui/src/index.css` — Global reset

### Verification

| Check | Result |
|-------|--------|
| `cargo build -p freyja-ui` | Clean |
| `cargo build -p freyja-engine` | Clean |
| `cargo test -p freyja-engine` | 275 tests pass |
| `npx vitest run` (freyja-ui) | 48 tests pass (34 board + 14 parser) |
| `npx vite build` (freyja-ui) | Clean (214 KB JS, 6.5 KB CSS) |

### Upstream Dependencies

- Stage 4 protocol: `freyja` init command, `position`/`go`/`stop`/`isready` commands
- Engine response formats from `output.rs`: bestmove, info, eliminated, nextturn, error
- Board geometry from `types.rs`: `is_valid_square`, 14×14 grid, corner exclusion

---

## Post-Audit

*To be filled after user testing and green light.*
