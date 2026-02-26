# Project Freyja — MASTERPLAN v1.0

**Version:** 1.0
**Created:** 2026-02-25
**Status:** Active

---

## 1. VISION

Project Freyja is a four-player chess engine built for **evaluation accuracy**. Its purpose is twofold:

1. **Play four-player chess** using the most accurate multi-player search algorithm available (Max^n with NNUE-guided beam search).
2. **Generate high-quality training data** for Project Odin's NNUE, providing truthful multi-player position evaluations that Odin's BRS/Paranoid hybrid cannot produce on its own.

Freyja is the teacher. Odin is the student. Freyja prioritizes truth (accurate multi-player modeling via Max^n) over speed (BRS/Paranoid depth). The two engines together cover both philosophies of multi-player game tree search.

### 1.1 Core Architecture

```
MCTS (Max^n backpropagation, NNUE leaf eval)
  │  Strategic exploration + diverse training data generation
  │
Max^n Search (depth 7-8, NNUE-guided beam search)
  │  Iterative deepening, shallow pruning
  │  Beam width adapts to NNUE maturity
  │  2 players remaining → negamax with full alpha-beta
  │
  ├── Quiescence (root-player captures, capped depth)
  ├── Move ordering (TT + killer + history + MVV-LVA)
  │
NNUE Evaluation
  ├── Material + Piece-Square Tables
  ├── Mobility (safe squares, piece activity)
  ├── Territory (BFS Voronoi partitioning)
  ├── Influence maps (piece-weighted decay)
  ├── King safety zones (shelter, escape, zone integrity)
  ├── Tension / vulnerability maps
  └── Tactical features (captures, threats, checks)
```

### 1.2 Key Design Principles

1. **Accuracy over speed.** Max^n tells the truth about multi-player dynamics. BRS and Paranoid lie (useful lies, but lies). Freyja searches accurately; Odin searches fast. Together they cover both.

2. **NNUE is the pruning strategy.** A smarter evaluation means better move ordering, which means the beam search can be tighter at depth, which means Max^n goes deeper. The engine literally gets deeper as it gets smarter.

3. **Zone control is first-class.** Territory, influence, tension, and vulnerability are not afterthoughts — they are core evaluation features that distinguish 4-player chess from 2-player chess.

4. **Fixed-size data structures from day one.** No `Vec<T>` in hot-path structs. Odin learned this the hard way (clone cost timebomb deferred 8 stages). Freyja solves it upfront.

5. **Every 4PC rule verified for all 4 orientations.** No "works for Red, assumed for others." A verification matrix tracks: rule × player = tested/untested.

6. **Stages are not complete until the user gives the green light** from testing in the UI. Compilation + tests passing is necessary but not sufficient.

### 1.3 Node Budget

| Component | Nodes | Time @ 1M NPS |
|-----------|-------|---------------|
| Max^n depth 7-8 | 7-8M | ~7-8s |
| MCTS | remaining time budget | varies |
| **Total per move** | **7-8M + MCTS** | **~10-20s typical** |

### 1.4 Beam Width Schedule

The beam width (how many moves Max^n expands per node) adapts to NNUE quality:

| NNUE Maturity | Beam Width | Effective Depth @ 7M nodes |
|---------------|-----------|---------------------------|
| Bootstrap (dumb) | 12-15 | 5-6 |
| Early NNUE | 8-10 | 6-7 |
| Mature NNUE | 5-8 | 7-8 |

The engine gets deeper as the NNUE gets smarter. Beam width is a tunable parameter, not hardcoded.

### 1.5 Training Data Pipeline (Future: Odin Integration)

```
Freyja plays self-play games (Max^n, accurate multi-player evals)
  │
  ▼
Positions + evaluations + zone control features exported
  │
  ▼
Odin's NNUE training pipeline consumes Freyja's data
  │
  ▼
Odin becomes stronger, plays Freyja, generates new training signal
```

This integration is a future milestone, not an initial stage deliverable.

---

## 2. TECHNOLOGY STACK

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Engine | Rust | Memory safety, performance, ownership model forces correct clone/copy decisions |
| UI | TypeScript + React | Learned from Odin: ref-snapshotting patterns, protocol conformance tests |
| NNUE Training | Python + PyTorch | Standard ML tooling, separate from engine |
| NNUE Inference | Rust (native) | No Python dependency at runtime |
| Protocol | Custom text protocol (Freyja Protocol) | Learned from Odin: versioned, both sides updated in same commit |
| Observability | `tracing` crate | Day one. No custom telemetry. Odin learned this lesson (Huginn, ADR-015). |
| Build | Cargo workspace | `freyja-engine`, `freyja-ui`, `freyja-nnue` |

---

## 3. ARCHITECTURE LAYERS

```
┌──────────────────────────────────────────────────────────────┐
│  Tier 6: Polish                                               │
│  Stage 18: Game Mode Tuning   Stage 19: Full UI               │
│  Stage 20: Optimization                                       │
├──────────────────────────────────────────────────────────────┤
│  Tier 5: Intelligence                                         │
│  Stage 14: Zone Control    Stage 15: NNUE Architecture        │
│  Stage 16: NNUE Training   Stage 17: NNUE Integration         │
├──────────────────────────────────────────────────────────────┤
│  Tier 4: Measurement                                          │
│  Stage 12: Self-Play Framework   Stage 13: Time + Beam Tuning │
├──────────────────────────────────────────────────────────────┤
│  Tier 3: Strategic Layer                                      │
│  Stage 10: MCTS   Stage 11: Max^n → MCTS Integration          │
├──────────────────────────────────────────────────────────────┤
│  Tier 2: Core Search                                          │
│  Stage 6: Bootstrap Eval   Stage 7: Max^n Search              │
│  Stage 8: Quiescence       Stage 9: TT + Move Ordering        │
├──────────────────────────────────────────────────────────────┤
│  Tier 1: Foundation                                           │
│  Stage 0: Skeleton     Stage 1: Board     Stage 2: MoveGen   │
│  Stage 3: GameState    Stage 4: Protocol   Stage 5: UI Shell  │
└──────────────────────────────────────────────────────────────┘
```

### 3.1 Dependency Map

```
Stage 0: Skeleton
  └─ Stage 1: Board Representation
       └─ Stage 2: Move Generation
            └─ Stage 3: Game State
                 ├─ Stage 4: Protocol
                 │    └─ Stage 5: UI Shell
                 └─ Stage 6: Bootstrap Eval
                      └─ Stage 7: Max^n Search
                           └─ Stage 8: Quiescence
                                └─ Stage 9: TT + Move Ordering
                                     ├─ Stage 10: MCTS
                                     │    └─ Stage 11: Integration
                                     └─ Stage 12: Self-Play
                                          └─ Stage 13: Time + Beam Tuning
                                               └─ Stage 14: Zone Control
                                                    └─ Stage 15: NNUE Arch
                                                         └─ Stage 16: NNUE Training
                                                              └─ Stage 17: NNUE Integration
                                                                   └─ Stage 18: Game Modes
                                                                        └─ Stage 19: Full UI
                                                                             └─ Stage 20: Optimization
```

**Parallel opportunities:**
- Stages 5 and 6 can run in parallel (UI shell and bootstrap eval are independent)
- Stages 10 and 12 have minimal cross-dependency
- Stages 18 and 19 can run in parallel

---

## 4. STAGE DEFINITIONS

---

### Stage 0: Project Skeleton

**The problem:** Nothing exists yet. Need a compilable, testable project structure.

**What you're building:**
- Cargo workspace with `freyja-engine` (library + binary), `freyja-nnue` (future)
- React UI project (`freyja-ui`) with Vite
- Masterplan document structure
- CI configuration (`cargo fmt`, `cargo clippy`, `cargo test`)

**Build order:**
1. Cargo workspace manifest
2. `freyja-engine` crate with `lib.rs` and `main.rs`
3. UI project skeleton (`npm init`, Vite + React + TypeScript)
4. `.gitignore` (target/, node_modules/, dist/, *.onnue)
5. Placeholder modules: `board`, `movegen`, `gamestate`, `eval`, `search`, `mcts`, `protocol`

**Acceptance criteria:**
- `cargo build` succeeds with zero warnings
- `cargo test` runs (may have zero tests, but the harness works)
- `cargo fmt --check` passes
- `cargo clippy` passes
- `npm install && npm run dev` starts the UI dev server
- All placeholder modules exist and are importable

**What you DON'T need:**
- Any game logic
- Any UI components beyond the Vite default
- Any NNUE code

**Tracing points:**
- None yet. `tracing` crate added to `Cargo.toml` but no instrumentation.

---

### Stage 1: Board Representation

**The problem:** Need a correct, efficient representation of the 14x14 four-player chess board with 36 invalid corner squares.

**What you're building:**
- `Board` struct with fixed-size arrays (no `Vec`)
- Square indexing: `rank * 14 + file` → 196 total, 160 valid
- Piece representation: `Option<(PieceType, Player)>` per square
- Piece lists: `[[Option<(PieceType, Square)>; 32]; 4]` with piece counts per player
- Player enum: `Red`, `Blue`, `Yellow`, `Green` with index 0-3
- PieceType enum: `Pawn`, `Knight`, `Bishop`, `Rook`, `Queen`, `King`, `PromotedQueen`
- Square validity checking (the 36 invalid corners — see `4PC_RULES_REFERENCE.md`)
- FEN4 parsing and serialization (round-trip correctness)
- Zobrist hashing: 160 squares × 7 piece types × 4 players = 4,480 piece keys + 4 side-to-move keys + 8 castling keys + en passant keys
- `set_piece` / `remove_piece` with Zobrist incremental update
- Attack query API: `is_square_attacked_by(square, player) -> bool`, `attackers_of(square) -> AttackInfo`

**Key data structures:**

```rust
pub struct Board {
    squares: [Option<Piece>; TOTAL_SQUARES],        // 196
    piece_lists: [[Option<(PieceType, Square)>; 32]; 4],
    piece_counts: [u8; 4],
    king_squares: [u8; 4],                           // 255 = eliminated sentinel
    zobrist: u64,
    castling_rights: u8,                             // 8 bits: 2 per player
    en_passant: Option<Square>,
    en_passant_pushing_player: Option<Player>,
    side_to_move: Player,
}
```

**All fixed-size. No heap allocation. Cheaply cloneable.**

**Build order:**
1. Square and coordinate types, validity checking, corner enumeration
2. Piece and Player types
3. Board struct with squares array
4. Piece list management (add/remove, fixed-size arrays)
5. Zobrist key table generation (deterministic seed)
6. `set_piece` / `remove_piece` with Zobrist update
7. FEN4 parser and serializer
8. Attack query API (ray-based for sliders, lookup for knights/pawns/king)
9. Comprehensive tests for all 4 player orientations

**Acceptance criteria:**
- FEN4 round-trip: `parse(serialize(board)) == board` for 10+ test positions
- Zobrist consistency: `compute_full_hash(board) == board.zobrist` after any sequence of set/remove
- All 36 invalid squares correctly identified
- Attack queries correct for all piece types, all 4 player directions
- Piece list and squares array always in sync
- `Board` implements `Clone` with zero heap allocation
- **4PC Verification Matrix:** every attack pattern tested for Red, Blue, Yellow, Green independently

**What you DON'T need:**
- Move generation (Stage 2)
- Make/unmake (Stage 2)
- Game state management (Stage 3)
- Any evaluation

**Tracing points:**
- `tracing::debug!` on `set_piece` / `remove_piece`: what changed, where, what was there before

---

### Stage 2: Move Generation

**The problem:** Need to generate all legal moves for any player in any position, handling the unique geometry and rules of 4-player chess.

**What you're building:**
- Pseudo-legal move generation for all piece types, all 4 orientations
- Legal move filtering (king not in check after move, considering all 3 opponents)
- Special moves: castling (8 variants), en passant (4 orientations), pawn promotion (4 orientations), pawn double-step (4 orientations)
- `make_move` / `unmake_move` with `MoveUndo` struct
- Compact move encoding: `u32` bitfield
- Perft testing at depths 1-4

**Key data structures:**

```rust
// Move encoding (u32):
// Bits 0-7:   from square (0-195)
// Bits 8-15:  to square (0-195)
// Bits 16-19: piece type (moved)
// Bits 20-23: captured piece type (0 = no capture)
// Bits 24-27: promotion piece type (0 = no promotion)
// Bits 28-30: flags (castling, en passant, double push)

pub struct MoveUndo {
    captured_piece: Option<Piece>,
    prev_castling: u8,
    prev_en_passant: Option<Square>,
    prev_en_passant_player: Option<Player>,
    prev_zobrist: u64,
    prev_side_to_move: Player,
}
```

**Critical 4PC-specific rules (each must be independently verified for all 4 players):**

| Rule | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Pawn forward direction | +rank | +file | -rank | -file |
| Pawn starting ranks | rank 1 | file 1 | rank 12 | file 12 |
| Pawn promotion rank | rank 12 | file 12 | rank 1 | file 1 |
| Double-step from rank | rank 1 | file 1 | rank 12 | file 12 |
| En passant capture offset | -rank | -file | +rank | +file |
| Castling orientation | horizontal | vertical | horizontal | vertical |
| King starting square | exact per 4PC_RULES_REFERENCE | ... | ... | ... |

**Build order:**
1. Move encoding/decoding
2. Pawn move generation (all 4 directions, including double-step, promotion)
3. Knight move generation (L-shaped, corner-aware)
4. Sliding piece generation (bishop, rook, queen — ray-based, corner-aware)
5. King move generation (adjacent squares, no self-check)
6. Castling generation (8 variants, all preconditions)
7. En passant generation (all 4 orientations)
8. Legal move filtering (check detection from all 3 opponents)
9. `make_move` with Zobrist update, piece list update, castling rights update
10. `unmake_move` restoring exact prior state
11. Perft at depths 1-4
12. **4PC Verification Matrix: every move type × every player = tested**

**Acceptance criteria:**
- Perft values at depths 1-4 match (self-consistent, round-trip verified)
- Zobrist make/unmake round-trip: `hash_before == hash_after_make_then_unmake` for ALL moves in ALL perft positions
- Piece list synchronized with squares array after every make/unmake
- Castling correctly generates for all 4 players (8 variants)
- En passant correctly handles all 4 orientations
- Promotion correctly identifies promotion rank per player
- No move generated to/from invalid corner squares
- 1000+ random playouts complete without panic

**What you DON'T need:**
- Move ordering (Stage 9)
- Evaluation (Stage 6)
- Search (Stage 7)

**Tracing points:**
- `tracing::debug!` on `make_move` / `unmake_move`: from, to, piece, captured, flags
- `tracing::trace!` on pseudo-legal generation: move count per piece type

---

### Stage 3: Game State

**The problem:** Need to manage the full state of a 4-player chess game beyond just the board — turns, eliminations, scoring, DKW (Dead King Walking), game termination.

**What you're building:**
- `GameState` wrapping `Board` with game-level state
- Turn management: `side_to_move` rotation skipping eliminated players
- Player status tracking: `Active`, `Eliminated(reason)`, `Resigned`
- Elimination detection: checkmate, stalemate (awards 20 points)
- DKW (Dead King Walking): eliminated player's king makes random moves between turns
- Point scoring: captures award points per piece value, checkmate awards 20 points
- Position history for draw detection (Zobrist-based)
- Game termination conditions (1 player remaining, point threshold, etc.)

**Key data structures:**

```rust
pub struct GameState {
    board: Board,
    player_status: [PlayerStatus; 4],
    scores: [u16; 4],
    move_number: u16,
    half_move_clock: u16,
    position_history: [u64; MAX_GAME_LENGTH],  // Fixed-size, not Vec
    history_count: u16,
    game_mode: GameMode,
}

pub enum PlayerStatus {
    Active,
    Eliminated { reason: EliminationReason, move_number: u16 },
    Resigned { move_number: u16 },
}

pub enum EliminationReason {
    Checkmate { by: Player },
    Stalemate,
    Timeout,
    FlagFall,
}
```

**Critical: Eliminated player handling.**
From Odin's lesson — when a player is eliminated, EVERY function that calls `generate_legal_moves` must check `PlayerStatus` first. This includes search, quiescence, and any future component. Missing a single check = crash on kingless board.

**Build order:**
1. `GameState` struct wrapping `Board`
2. Turn management with eliminated-player skipping
3. Check detection (is player in check from any of 3 opponents?)
4. Checkmate detection (in check + no legal moves)
5. Stalemate detection (not in check + no legal moves)
6. Elimination chain processing
7. DKW move generation and execution (king-only random moves between turns)
8. Point scoring on captures and eliminations
9. Position history tracking (fixed-size array)
10. Game termination detection
11. `process_dkw_moves` BEFORE `check_elimination_chain` (order matters — Odin lesson)

**Acceptance criteria:**
- Eliminated players skipped in turn rotation without crash
- Checkmate correctly detected for all 4 players
- Stalemate awards 20 points to stalemated player
- DKW king makes only legal king moves
- DKW moves execute BEFORE elimination checks (ordering invariant)
- King square set to 255 sentinel on elimination (not stale value)
- 1000+ random playouts reach game termination without panic
- Position history never exceeds `MAX_GAME_LENGTH`

**What you DON'T need:**
- Protocol communication (Stage 4)
- UI (Stage 5)
- Terrain pieces (Stage 18, game mode specific)

**Tracing points:**
- `tracing::info!` on elimination: who, why, by whom, move number
- `tracing::info!` on game termination: reason, final scores
- `tracing::debug!` on DKW moves: king from/to

---

### Stage 4: Freyja Protocol

**The problem:** Need a communication protocol between the engine and UI. Learned from Odin: protocol must be versioned, changes must update both sides in the same commit, parsers must handle extensions gracefully.

**What you're building:**
- Text-based protocol over stdin/stdout (similar to UCI, adapted for 4PC)
- Command set: `freyja` (identify), `isready`/`readyok`, `position`, `go`, `stop`, `quit`
- Info output: `info depth`, `info score`, `info nodes`, `info pv`, `info string`
- Elimination events: `info string eliminated <color> <reason>` (parser extracts first token only — Odin lesson)
- Turn events: `info string nextturn <color>`
- Bestmove output: `bestmove <move>`
- Game mode configuration: `setoption name GameMode value <mode>`

**Protocol version header:**
```
freyja v1.0 maxn-beam-mcts
```

**Build order:**
1. Command parser (tokenizer, handles unknown commands gracefully)
2. Position command: `position fen4 <fen> moves <move1> <move2> ...`
3. Go command: `go depth <d>`, `go nodes <n>`, `go movetime <ms>`, `go infinite`
4. Bestmove output
5. Info string output (depth, score as 4-vector, nodes, nps, pv)
6. Option handling (game mode, beam width)
7. Protocol integration test: send position + go, receive bestmove

**Acceptance criteria:**
- Engine responds to `isready` with `readyok`
- Position command correctly sets board state
- Go command triggers search (stub: returns first legal move)
- Info strings correctly formatted
- Unknown commands produce `info string error: unknown command '<cmd>'`, not a crash
- Parser handles extra whitespace, trailing newlines
- **Protocol conformance test: every message format has a round-trip test**

**What you DON'T need:**
- Actual search (Stage 7 — go command returns first legal move as stub)
- Time management (Stage 13)
- Ponder mode

**Tracing points:**
- `tracing::debug!` on every command received
- `tracing::debug!` on every response sent

---

### Stage 5: UI Shell

**The problem:** Need a basic UI to visualize the board, make moves, and interact with the engine. Learned from Odin: React 18 batching timing bugs, ref-snapshotting pattern, parser conformance.

**What you're building:**
- 14x14 board display with 36 invalid corners visually distinct
- 4 color-coded player pieces
- Click-to-select, click-to-move interaction
- Engine communication via IPC (spawn engine process, pipe stdin/stdout)
- Move display and game log
- Basic controls: new game, auto-play, step mode
- Score display for all 4 players

**Critical UI rules (Odin lessons):**
1. **UI owns ZERO game logic.** UI never validates moves, detects check, evaluates. Engine is the authority.
2. **Snapshot refs before mutations.** Any value needed after setState must be captured in a local variable BEFORE the state update.
3. **Single point of entry for engine commands.** One function sends `go`, one guard checks if a search is already in flight. No dual code paths.
4. **Protocol parser handles extensions.** `eliminated Red checkmate` → extract first token `Red` only. Future-proof.

**Build order:**
1. Board rendering (14x14 grid, corners greyed out)
2. Piece rendering (SVG or Unicode, color-coded by player)
3. Square coordinate system matching engine's system exactly
4. Click-to-select, legal move highlighting (via engine query)
5. Engine process spawning and stdin/stdout piping
6. Protocol message parsing (with extension tolerance)
7. Move execution: send to engine, receive bestmove, update display
8. Game log display
9. Auto-play mode with `awaitingBestmoveRef` guard
10. **Test: all 4 players' pieces render correctly in starting position**

**Acceptance criteria:**
- Board renders with correct geometry (14x14, corners invalid)
- All 4 players' pieces visible in starting position
- King and Queen positions correct for all 4 players (per 4PC_RULES_REFERENCE)
- Click-select-move workflow completes a move
- Engine communication works (send position + go, receive bestmove)
- Auto-play mode doesn't double-send `go` (guard test)
- **4PC Verification: piece positions and square labels correct for all 4 orientations**

**What you DON'T need:**
- Move arrows, highlights, analysis features (Stage 19)
- Theme customization
- Self-play dashboard (Stage 12)
- Responsive design

**Tracing points:**
- N/A (UI uses browser console)

---

### Stage 6: Bootstrap Evaluation

**The problem:** Need an evaluation function to make the engine play chess. This is temporary — NNUE replaces it in Stage 17. Keep it simple, but include zone control features because they become NNUE training features later.

**What you're building:**
- `Evaluator` trait with two methods:
  - `eval_scalar(state: &GameState, player: Player) -> i16` — single perspective score
  - `eval_4vec(state: &GameState) -> [i16; 4]` — all 4 players' scores
- `BootstrapEvaluator` implementing the trait with:
  1. Material counting (with relative material advantage vs opponents)
  2. Piece-square tables (4 orientations — rotated per player)
  3. Basic mobility (legal move count approximation)
  4. Simple king safety (pawn shelter + attacker count)
  5. Basic territory (BFS Voronoi — who controls more squares?)
  6. Pawn structure (advancement bonus, doubled penalty)

**Piece values:**

| Piece | Eval (cp) | Capture (points) |
|-------|----------|-----------------|
| Pawn | 100 | 1 |
| Knight | 300 | 3 |
| Bishop | 350 | 3 |
| Rook | 500 | 5 |
| Queen | 900 | 9 |
| PromotedQueen | 900 (eval) | 1 (capture points) |
| King | N/A | 0 (DKW) |

**Critical lesson from Odin:** Material eval must be RELATIVE, not absolute. Count your material vs opponents' material. Capturing an opponent's rook must change your eval. Odin's original eval counted only own pieces — zero incentive to capture.

**Critical lesson from Odin:** Be careful with lead-penalty heuristics. A penalty for being ahead in material creates non-monotonic evaluation where capturing a queen can LOWER your score. If included, it must not override tactical correctness. For the bootstrap, omit the lead penalty entirely.

**Build order:**
1. `Evaluator` trait definition (stable interface for all future evaluators)
2. Material counting with relative advantage
3. Piece-square tables for all 4 orientations
4. Basic mobility (count of legal moves, approximated)
5. BFS Voronoi territory (multi-source BFS from all pieces, ~320 operations)
6. Simple king safety (pawn shelter count + attacker presence)
7. Pawn structure (advancement + doubled detection)
8. Combine into `eval_scalar` and `eval_4vec`
9. Eval symmetry test: position mirrored between players produces symmetric scores

**Acceptance criteria:**
- Materially different positions score differently
- Capturing an opponent piece improves your score
- `eval_scalar` and `eval_4vec` are consistent (Red's scalar matches Red's component in 4vec)
- Eval completes in < 50us per call (bootstrap, unoptimized is fine)
- BFS territory correctly assigns squares to nearest player
- Eval symmetry: swapping Red and Blue's pieces produces swapped scores
- Eliminated players return sentinel score

**What you DON'T need:**
- NNUE (Stages 15-17)
- Influence decay maps (Stage 14 — zone control enhancement)
- Tension/vulnerability maps (Stage 14)
- Tactical features beyond material (Stage 14)
- Tuned weights (Stage 13 — self-play tuning)

**Tracing points:**
- `tracing::debug!` on eval call: total score, component breakdown
- `tracing::trace!` on territory computation: squares per player

---

### Stage 7: Max^n Search

**The problem:** Need the core search algorithm — Max^n with NNUE-guided beam search. This is Freyja's defining component.

**What you're building:**
- `Searcher` trait with `search(state: &GameState, limits: SearchLimits) -> SearchResult`
- `MaxnSearcher` implementing the trait
- Max^n algorithm: each player maximizes their own score (4-tuple propagation)
- Score vector: `[i16; 4]` backed up through the tree, each player maximizes their component
- Beam search: at each node, only expand the top K moves ranked by eval
- Iterative deepening: depth 1 → 2 → 3 → ... → 8
- Shallow pruning (Korf 1991): using sum bound + individual bounds
- Negamax fallback: when only 2 active players remain
- PV (principal variation) tracking

**The Max^n Algorithm:**

```
fn maxn(state, depth, beam_width) -> [i16; 4]:
    if depth == 0 or state.is_terminal():
        return evaluator.eval_4vec(state)

    current_player = state.side_to_move()

    if state.player_status[current_player] != Active:
        // Skip eliminated player — DO NOT call generate_legal
        state.advance_turn()
        result = maxn(state, depth - 1, beam_width)
        state.undo_advance_turn()
        return result

    // NNUE-guided beam: rank all moves, only expand top K
    moves = generate_legal_moves(state)
    scored_moves = moves.map(|m| (m, evaluator.eval_scalar_after(state, m, current_player)))
    scored_moves.sort_descending()
    beam = scored_moves.take(beam_width)

    best_scores = [i16::MIN; 4]

    for move in beam:
        undo = state.make_move(move)
        child_scores = maxn(state, depth - 1, beam_width)
        state.unmake_move(move, undo)

        if child_scores[current_player] > best_scores[current_player]:
            best_scores = child_scores

    return best_scores
```

**Key design decisions:**
- Score vector `[i16; 4]` is the fundamental data unit, not a single scalar
- Each player maximizes their own component at their nodes
- Beam width is a parameter, not hardcoded
- Eliminated players are SKIPPED (check before generate_legal — Odin lesson)
- Shallow pruning uses: sum of all scores bounded, individual scores bounded by eval range

**Build order:**
1. `Searcher` trait definition
2. Score vector type `[i16; 4]` with comparison utilities
3. Basic Max^n without beam restriction (depth 1-3 only, all moves expanded)
4. Beam search integration (rank by eval, restrict to top K)
5. Iterative deepening wrapper
6. Shallow pruning (sum bound + individual bounds)
7. Negamax fallback for 2-player endgame
8. PV tracking and info string output
9. Time/node limits for iterative deepening termination
10. **Eliminated player skip at every depth** (check before generate_legal)

**Acceptance criteria:**
- Finds mate-in-1 from constructed positions
- Does not hang pieces within search depth
- Iterative deepening reaches depth 5+ in 5 seconds with bootstrap eval
- Beam width 30 (all moves) produces same result as no beam at depths 1-3
- Narrower beam (8-10) reaches deeper depths for same node count
- Negamax activates correctly when 2 players remain
- Eliminated players never trigger `generate_legal_moves`
- PV reported via info strings
- Node count within expected range for given depth and beam width

**What you DON'T need:**
- Transposition table (Stage 9)
- Killer/history heuristics (Stage 9)
- MCTS integration (Stage 11)
- Quiescence (Stage 8 — added next)

**Tracing points:**
- `tracing::info!` on search start/complete: depth, nodes, score, pv, beam width
- `tracing::debug!` on depth completion: depth N score, nodes at this depth
- `tracing::trace!` on node expansion: move, player, depth, score vector

---

### Stage 8: Quiescence Search

**The problem:** Without quiescence, Max^n can stop right before a queen capture and think the position is good. The horizon effect is worse in 4-player chess because 3 opponents can create tactical chaos between your moves.

**What you're building:**
- Quiescence search extending from Max^n leaf nodes
- Restricted to **root-player captures only** (opponent-vs-opponent captures too expensive)
- Stand-pat evaluation (can return eval without searching if it's already good)
- Delta pruning (skip captures that can't possibly improve alpha)
- Capped depth (MAX_QSEARCH_DEPTH = 8, prevents explosion)

**Why root-player captures only:**
In 4-player chess, if each opponent has ~5 captures, full quiescence is 5³ = 125 opponent capture sequences per root-player move. Restricting to captures that directly affect the root player keeps the tree manageable.

**Build order:**
1. Capture-only move generation function
2. Stand-pat evaluation at quiescence entry
3. Root-player capture search (only captures of/by root player's pieces)
4. Delta pruning (skip if `stand_pat + capture_value + margin < alpha`)
5. Depth cap enforcement
6. Integration with Max^n (call quiescence at depth 0 instead of static eval)
7. Node counting (quiescence nodes counted separately)

**Acceptance criteria:**
- Engine does not miss hanging pieces at search horizon
- Quiescence resolves capture chains (queen takes pawn, rook retakes, etc.)
- Stand-pat allows early cutoff in quiet positions
- Depth cap prevents quiescence explosion (test with tactical positions)
- Node count overhead: quiescence adds < 50% to total node count in typical positions
- Tactical suite: engine finds all captures that the bootstrap eval values correctly

**What you DON'T need:**
- Check extensions in quiescence (keep it simple: captures only)
- SEE (Static Exchange Evaluation) — can add in Stage 20 optimization

**Tracing points:**
- `tracing::debug!` on quiescence entry: stand_pat score, number of captures available
- `tracing::trace!` on quiescence capture: move, delta, pruned or expanded

---

### Stage 9: Transposition Table + Move Ordering

**The problem:** Max^n revisits the same positions through different move orders. TT caches results. Move ordering makes the beam search more effective (best move first = tighter beam is safer).

**What you're building:**
- Transposition table: fixed-size hash table, replace-if-deeper
- TT entry stores: Zobrist key, depth, score vector `[i16; 4]`, best move, flag (exact/lower/upper)
- Move ordering priority: TT best move → captures (MVV-LVA) → promotions → checks → killer moves → history heuristic → quiet moves
- Killer move table: 2 per depth per player
- History heuristic table: `[u32; 196][196]` (from-to scoring)

**Key difference from 2-player TT:**
The score is a 4-vector, not a scalar. TT flag semantics:
- Exact: all 4 scores are known
- Lower/Upper bounds: apply only to the current player's component
- Shallow pruning can use TT bounds from other players' perspectives

**Build order:**
1. TT data structure (fixed-size array, power-of-2 size for fast modulo)
2. TT probe and store logic
3. TT integration with Max^n search
4. MVV-LVA capture scoring
5. Killer move table (per depth, per player)
6. History heuristic table (updated on cutoffs)
7. Move ordering function combining all heuristics
8. Beam search now uses TT best move + ordering for move ranking (not just raw eval)

**Acceptance criteria:**
- TT hit rate > 30% at depth 5+ (measured via tracing)
- TT produces no correctness regressions (same result with and without TT at shallow depths)
- Move ordering: TT best move searched first when available
- Killer moves produce cutoffs (measured: killer move hit rate > 10%)
- History table populated after iterative deepening (nonzero entries)
- No hash collision corruption (key verification on probe)
- Performance: NPS improvement of at least 30% over no-TT baseline

**What you DON'T need:**
- MCTS integration (Stage 10)
- Multi-threaded TT access (Stage 20)

**Tracing points:**
- `tracing::debug!` on TT hit: depth, flag, score vector
- `tracing::debug!` on TT store: depth, flag, best move
- `tracing::info!` on search complete: TT hit rate, killer hit rate

---

### Stage 10: MCTS

**The problem:** Max^n handles tactical depth (7-8 plies). MCTS handles strategic breadth — exploring diverse continuations and finding moves that lead to better long-term positions.

**What you're building:**
- Monte Carlo Tree Search with Max^n-style backpropagation
- Each MCTS node stores: visit count, score sums `[f64; 4]`, children, prior move
- Selection: UCB1 adapted for 4 players — each player maximizes their own UCB component
- Expansion: create child nodes for legal moves
- Evaluation: NNUE (or bootstrap eval) at leaf nodes — NOT a full Max^n search (too slow per simulation)
- Backpropagation: update all 4 score components up the tree
- Progressive widening at deeper nodes (optional: limit children to K initially, add more as visits increase)

**UCB1 for multi-player:**
```
UCB(node, player) = Q(node)[player] / N(node) + C * sqrt(ln(N(parent)) / N(node))
```
Where `Q(node)[player]` is the accumulated score for `player` and `C` is the exploration constant.

At each node, the current player selects the child with the highest UCB for themselves.

**Build order:**
1. MCTS node struct (arena-allocated or Vec-based tree)
2. Selection phase (UCB1 traversal)
3. Expansion phase (add one child per simulation)
4. Evaluation phase (call eval_4vec at leaf)
5. Backpropagation phase (update score sums and visit counts)
6. Root move selection (highest visit count, not highest average — more robust)
7. Search time management (run simulations until time limit)
8. Info output (simulations/second, top moves by visit count)
9. Progressive widening (optional enhancement)

**Acceptance criteria:**
- MCTS converges: with enough simulations, consistently picks the same best move
- MCTS finds mate-in-1 (even without full search, simulations should discover it)
- Simulations per second > 10,000 with bootstrap eval (fast playouts)
- Memory bounded: tree does not grow without limit (node recycling or cap)
- Handles eliminated players correctly in simulations
- Score vectors backpropagate correctly (each player's component updated independently)

**What you DON'T need:**
- Max^n search integration (Stage 11)
- RAVE or AMAF (not applicable to multi-player — move permutation assumptions fail)
- Parallel MCTS (Stage 20)

**Tracing points:**
- `tracing::info!` on MCTS complete: simulations, top 3 moves with visit counts
- `tracing::debug!` on selection: path from root to leaf
- `tracing::trace!` on backpropagation: score update per node

---

### Stage 11: Max^n → MCTS Integration

**The problem:** Max^n and MCTS are separate systems. Need a controller that runs Max^n first (for tactical grounding), then MCTS (for strategic exploration).

**What you're building:**
- Hybrid controller implementing the `Searcher` trait
- Phase 1: Run Max^n to depth 7-8 with iterative deepening
- Phase 2: Run MCTS with remaining time budget
- MCTS uses NNUE (not Max^n) as leaf evaluator — Max^n is too slow per simulation
- MCTS root move ordering informed by Max^n results (most-visited children initialized with Max^n scores)
- Final move selection: MCTS's highest visit count, but if MCTS and Max^n disagree on best move, flag for review (tracing)

**Build order:**
1. Hybrid controller struct holding both `MaxnSearcher` and `MctsSearcher`
2. Time allocation: Max^n gets N seconds, MCTS gets the rest
3. Max^n runs, produces best move + score vector + PV
4. MCTS runs, initialized with Max^n's move ordering at root
5. Final move selection logic
6. Info output: report both Max^n and MCTS perspectives
7. Disagreement detection and logging

**Acceptance criteria:**
- Controller correctly sequences Max^n → MCTS
- MCTS receives remaining time (not total time)
- MCTS root moves ordered by Max^n scores
- If Max^n found mate, MCTS skipped (no need to explore)
- Disagreement rate logged (Max^n best ≠ MCTS best)
- Total time usage within budget

**What you DON'T need:**
- Adaptive time splitting (Stage 13)
- Training data export (future Odin integration)

---

### Stage 12: Self-Play Framework

**The problem:** Can't improve what you can't measure. Need automated self-play to validate changes.

**What you're building:**
- Self-play runner: engine plays against itself in all 4 seats
- Game recording: full move history + positions + evaluations
- Statistics: win rate, average score, average game length
- A/B comparison: play version A vs version B, measure win rate difference
- SPRT (Sequential Probability Ratio Test) for statistical significance
- Training data export: positions + eval vectors for NNUE training

**Build order:**
1. Self-play game loop (4 instances of engine, one per player seat)
2. Game result recording
3. Statistics computation (win rate, score distribution)
4. A/B framework (two different engine configs, measure head-to-head)
5. SPRT implementation for early stopping
6. Training data export format (position FEN4 + eval 4-vector + game result)

**Acceptance criteria:**
- Self-play completes 100 games without crash
- Statistics match expected distribution (roughly equal win rates for identical engines)
- A/B comparison detects a known-good improvement (e.g., deeper search beats shallower)
- Training data export produces valid, parseable files
- SPRT correctly identifies improvement vs regression at 95% confidence

---

### Stage 13: Time Management + Beam Width Tuning

**The problem:** Need to allocate time between Max^n and MCTS, and tune beam width for optimal depth within the node budget.

**What you're building:**
- Time allocation: Max^n gets a configurable fraction (default: 50%), MCTS gets the rest
- Beam width auto-tuning: measure NNUE move-ordering accuracy, adjust beam width
- Adaptive beam: wider in tactically complex positions, narrower in quiet positions
- Node budget enforcement: hard cap at 7-8M nodes for Max^n
- Iterative deepening time management: stop deepening when time/nodes nearly exhausted

**Build order:**
1. Time allocation parameters
2. Node budget enforcement in Max^n
3. Beam width configuration (per-depth schedule)
4. Self-play experiments: measure depth achieved vs beam width
5. Adaptive beam width based on position complexity (number of captures available)
6. Document optimal beam width schedule for bootstrap eval

**Acceptance criteria:**
- Max^n stays within 7-8M node budget
- Beam width tuning documented with self-play results
- Depth 7-8 achieved with mature beam settings
- Time allocation between Max^n and MCTS is configurable

---

### Stage 14: Zone Control Features

**The problem:** Standard chess eval (material + PST + mobility) misses the territorial dynamics that dominate 4-player chess. Zone control features are cheap to compute on 160 squares and provide rich positional information.

**What you're building:**
- Enhanced zone control features (upgrading bootstrap eval's basic territory):
  1. **Territory (BFS Voronoi):** Multi-source BFS from all pieces, assigns territory per player. Count, frontier length, encirclement ratio.
  2. **Influence maps:** Per-player influence with distance decay (exponential or linear). Piece-weighted (queen emits more influence than pawn).
  3. **King safety zones:** King zone influence ratio (friendly vs enemy), pawn shelter integrity, escape route count, virtual mobility (queen-replacement attack vectors).
  4. **Tension maps:** Sum of all players' influence per square. High tension = active combat zone. Low tension = quiet backwater.
  5. **Vulnerability maps:** Enemy influence minus friendly influence per square. Highlights danger areas.
  6. **Square control scoring:** Weighted by reciprocal piece value (pawn controlling > queen controlling).
- All features output as numeric vectors suitable for NNUE input

**Computational budget:** All features combined < 5us on 160 squares.

**Build order:**
1. Enhanced BFS Voronoi with frontier and encirclement metrics
2. Influence map with configurable decay function
3. King safety zone computation
4. Tension and vulnerability derivation from influence maps
5. Square control scoring with reciprocal piece weights
6. Integration into bootstrap evaluator (replacing basic territory)
7. Feature vector export (for future NNUE training)
8. Self-play validation: zone control features improve eval quality

**Acceptance criteria:**
- Territory count distinguishes between expanding and contracting players
- Influence maps correctly show piece projection patterns
- King safety detects exposed kings (low shelter, many attackers)
- Tension maps highlight contested zones
- All features compute in < 5us combined
- Self-play: zone-control-enhanced eval beats basic eval at 95% confidence

---

### Stage 15: NNUE Architecture

**The problem:** Hand-tuned eval has ceiling. NNUE learns optimal feature weights from data. Need the inference architecture in Rust — training is Stage 16 (Python).

**What you're building:**
- NNUE network architecture for 4-player chess:
  - Input: 160 squares × 7 piece types × 4 relative players = 4,480 features per perspective
  - Plus zone control features (territory count, influence summary, king safety, tension)
  - 4 accumulators (one per player perspective)
  - Hidden layers: 256 → 32 → 1 (per perspective), then combine for 4-vector output
- Accumulator: incrementally updated on make/unmake (add/remove piece features)
- Weight file format: `.fnnue` (Freyja NNUE binary format)
- Random-weight inference for pipeline verification

**Key difference from 2-player NNUE:**
4 accumulators, one per player perspective. Each sees the board from their own viewpoint. The network produces 4 scores independently, then they're combined.

**Build order:**
1. Feature encoding (perspective-relative piece-square mapping)
2. Accumulator struct (fixed-size, stack-allocated)
3. Accumulator push/pop for make/unmake (incremental update)
4. Forward pass: accumulator → hidden → output
5. 4-perspective combination into `[i16; 4]`
6. Zone control features as additional network inputs
7. Weight loading from `.fnnue` file
8. Random-weight verification: inference produces values, accumulator round-trips

**Acceptance criteria:**
- Inference produces values for all 4 players
- Accumulator make/unmake round-trip: same output before and after
- Forward pass completes in < 5us (fast enough for beam search eval)
- Weight file loads and produces deterministic output
- `NnueEvaluator` implements `Evaluator` trait (drop-in replacement for bootstrap)

---

### Stage 16: NNUE Training Pipeline

**The problem:** Need to train the NNUE on self-play data. This is Python + PyTorch.

**What you're building:**
- Python training pipeline in `freyja-nnue/`
- Training data loader (reads self-play export from Stage 12)
- Network architecture in PyTorch (mirrors Rust inference architecture)
- Loss function: MSE on 4-vector eval output vs search result
- Weight export to `.fnnue` binary format
- Training script with configurable hyperparameters

**Build order:**
1. Training data format parser (Python)
2. PyTorch network definition (matching Rust architecture exactly)
3. Training loop with MSE loss
4. Validation on held-out positions
5. Weight export to `.fnnue` format
6. Round-trip test: train → export → load in Rust → verify same outputs

**Acceptance criteria:**
- Training loss decreases over epochs
- Exported weights load correctly in Rust engine
- Rust inference matches PyTorch inference on test positions (within floating-point tolerance)
- Trained NNUE beats random-weight NNUE in self-play

---

### Stage 17: NNUE Integration

**The problem:** Swap bootstrap eval for trained NNUE. This is the most dangerous swap in the project.

**What you're building:**
- `NnueEvaluator` as the default evaluator
- Bootstrap eval retained as fallback
- Beam width tightening: with smarter NNUE, beam can narrow, depth increases
- Self-play validation: NNUE engine vs bootstrap engine
- Before-and-after audit: perft, NPS, eval values, self-play results

**Build order:**
1. Wire `NnueEvaluator` into Max^n search (trait swap — mechanical)
2. Verify accumulator lifecycle through full games (no corruption)
3. Self-play: NNUE vs bootstrap, measure win rate
4. Beam width experiment: measure optimal beam with NNUE ordering
5. If NNUE wins: make it default. If not: investigate training data quality.

**Acceptance criteria:**
- NNUE evaluator produces sane values (materially better positions score higher)
- Accumulator round-trip verified through 100+ full games
- NNUE engine beats bootstrap engine in self-play (>55% win rate)
- Beam width can be tightened by at least 2 (e.g., 10→8) while maintaining depth
- No perft regressions, no crash in 1000 random playouts

---

### Stage 18: Game Mode Tuning

**The problem:** Different game modes (FFA, Teams, Last King Standing, DKW variants) require different evaluation profiles.

**What you're building:**
- Game mode enum: `FFA`, `Teams`, `LastKingStanding`, `DKWVariant`
- Mode-specific eval adjustments (e.g., teammate pieces valued differently in Teams)
- Mode-specific search adjustments (e.g., more aggressive beam in LKS)
- Terrain pieces (for terrain mode): inert pieces that block movement, cannot be captured

**Deferred to this stage intentionally.** Get the core engine working on standard FFA first.

---

### Stage 19: Full UI

**The problem:** The shell UI from Stage 5 is minimal. Full UI adds analysis features, self-play visualization, and training data inspection.

**UI still owns ZERO game logic.** All analysis comes from the engine.

---

### Stage 20: Optimization

**The problem:** Profile-first optimization. Never optimize without measurement.

**Candidates:**
- Arena allocation for MCTS nodes
- SIMD for NNUE inference
- Bitboard representation for attack maps (256-bit for 196 squares)
- Parallel MCTS (tree parallelism)
- Lockless TT for future multi-threaded search

---

## 4.1 MAINTENANCE INVARIANTS

Once established, these must pass after every stage, forever.

| # | Invariant | Introduced | Description |
|---|-----------|-----------|-------------|
| 1 | Prior-stage tests never deleted | Stage 0 | Tests accumulate, never shrink |
| 2 | Board representation round-trips | Stage 1 | FEN4 parse → serialize → parse = identical |
| 3 | Zobrist make/unmake round-trip | Stage 2 | `hash_before == hash_after_make_then_unmake` |
| 4 | Perft values are forever | Stage 2 | Any change to perft values is a bug |
| 5 | Attack query API is board boundary | Stage 2 | Nothing above Board reads `squares[]` directly |
| 6 | Piece lists sync with squares | Stage 2 | Always in agreement after any operation |
| 7 | Game playouts complete | Stage 3 | 1000+ random playouts without crash |
| 8 | Eliminated players never trigger movegen | Stage 3 | PlayerStatus checked before generate_legal everywhere |
| 9 | DKW before elimination checks | Stage 3 | process_dkw_moves runs before check_elimination_chain |
| 10 | Protocol conformance | Stage 4 | Every message format has a round-trip integration test |
| 11 | UI owns zero game logic | Stage 5 | UI never validates moves, detects check, evaluates |
| 12 | Evaluator trait is eval boundary | Stage 6 | All search calls through trait, never direct implementation |
| 13 | Eval consistency | Stage 6 | eval_scalar and eval_4vec agree for same position |
| 14 | Searcher trait is search boundary | Stage 7 | Hybrid controller composes through trait |
| 15 | Engine finds forced mates | Stage 7 | Finds mate-in-1, doesn't hang pieces within depth |
| 16 | TT produces no correctness regressions | Stage 9 | With TT disabled, same results at shallow depth |
| 17 | NPS does not regress > 15% between stages | Stage 9 | Performance baselines tracked and enforced |
| 18 | 4PC verification matrix complete | Stage 2 | Every rule × every player = independently tested |

### 4.2 Tactical Position Suite

**Format:** One position per line in `freyja-engine/tests/positions/tactical_suite.txt`:
```
<fen4> | <best_move> | <category> | <description>
```

**Categories:** `mate`, `fork`, `capture`, `defense`, `quiet`, `territory`

**Growth plan:**
- Stage 7: 10 positions minimum (5 mate, 3 capture, 2 fork)
- Stage 9: 20 positions (add defense, quiet, territory)
- Stage 14: 30 positions (zone-control-specific positions)
- Stage 17: 50+ positions (NNUE validation suite)

**Rules:**
- Positions never removed, only added
- Best moves verified by deep search before acceptance
- Each position tested for all relevant player perspectives
- Run as part of CI after Stage 7

---

## 5. AUDIT PROTOCOL

See `AGENT_CONDUCT.md` Section 2 for the comprehensive 26-point audit checklist. It is carried forward from Project Odin with Freyja-specific additions.

---

## 6. NAMING CONVENTIONS

| Entity | Convention | Example |
|--------|-----------|---------|
| Rust modules | snake_case | `move_gen`, `board_repr`, `zone_control` |
| Rust types | PascalCase | `GameState`, `MaxnSearcher`, `MctsNode` |
| Rust functions | snake_case | `generate_legal_moves`, `eval_4vec` |
| Rust constants | SCREAMING_SNAKE | `MAX_DEPTH`, `BEAM_WIDTH_DEFAULT`, `MAX_QSEARCH_DEPTH` |
| UI components | PascalCase | `BoardDisplay`, `GameLog` |
| Protocol commands | lowercase | `bestmove`, `isready`, `position` |

---

## 7. GLOSSARY

| Term | Definition |
|------|-----------|
| **Max^n** | Multi-player minimax where each player maximizes their own score component in an n-tuple |
| **Beam search** | Restricting the search at each node to the top K moves ranked by evaluation |
| **Beam width** | The number of moves expanded per node (K). Wider = slower but safer. Tighter = faster but may miss moves. |
| **Paranoia dial** | A blend parameter (0.0-1.0) between Max^n and Paranoid. 0.0 = pure Max^n, 1.0 = pure Paranoid. Currently NOT used in Freyja (pure Max^n), but the dial exists in Athena's heritage for reference. |
| **Shallow pruning** | The proven Max^n pruning technique (Korf 1991) using sum bounds and individual bounds |
| **BFS Voronoi** | Multi-source BFS assigning board squares to the nearest player's pieces. Produces territory counts. |
| **Influence map** | Per-square, per-player influence score based on piece proximity with distance decay |
| **Tension** | Sum of all players' influence at a square. High = contested combat zone. |
| **Vulnerability** | Enemy influence minus friendly influence at a square. Positive = dangerous. |
| **DKW** | Dead King Walking. An eliminated player's king continues to make random moves. |
| **NNUE** | Efficiently Updatable Neural Network. Evaluation function that incrementally updates an accumulator on each move. |
| **4-vector / 4-tuple** | A score array `[i16; 4]` with one component per player. The fundamental unit of Max^n evaluation. |
| **Score vector** | Synonym for 4-vector. |
| **Accumulator** | The NNUE's hidden layer state, incrementally updated by adding/removing piece features. |
| **PV** | Principal Variation. The expected best sequence of moves. |
| **TT** | Transposition Table. Hash table caching search results to avoid re-searching identical positions. |
| **MVV-LVA** | Most Valuable Victim - Least Valuable Attacker. Capture ordering heuristic. |
| **SPRT** | Sequential Probability Ratio Test. Statistical test for determining if a change is an improvement. |
| **EBF** | Effective Branching Factor. The actual average branching observed in search, after pruning and beam restriction. |
| **Bootstrap eval** | The temporary hand-tuned evaluation function used before NNUE is trained. |

---

## APPENDIX A: DEPENDENCY MAP

See Section 3.1.

## APPENDIX B: RISK REGISTER

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|-----------|
| 1 | NNUE training doesn't converge | Medium | High | Bootstrap eval carries the engine. Investigate training data quality and network architecture. |
| 2 | Beam search misses critical moves | Medium | High | Start with wide beams. Quiescence catches tactical misses. Measure with tactical suite. |
| 3 | Max^n too slow for depth 7-8 | Low | Medium | Beam width is tunable. Depth 6 with wider beam is still valuable for training data. |
| 4 | Zone control features add noise, not signal | Low | Medium | Self-play A/B test zone control on vs off. Remove features that don't help. |
| 5 | 4PC rule implementation errors | High | High | 4PC verification matrix: every rule × every player = tested. Odin's lessons applied. |
| 6 | MCTS memory growth unbounded | Medium | Medium | Node count cap, tree recycling between moves. |
| 7 | React UI timing bugs | Medium | Low | Ref-snapshotting pattern, single entry point for engine commands. Odin lessons. |
| 8 | Freyja ↔ Odin training data format mismatch | Low | Medium | Define format in Stage 12, document thoroughly. |
| 9 | Clone cost in hot paths | Low (mitigated by design) | High | Fixed-size data structures from Stage 1. No Vec in Board or GameState. |
| 10 | Deferred debt accumulation | Medium | Medium | 2-stage escalation rule from Odin's AGENT_CONDUCT (Section 1.16). |

---

*End of MASTERPLAN v1.0*
