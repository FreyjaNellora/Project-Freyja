# Downstream Log — Stage 00: Project Skeleton

## Must-Know

- **Workspace root:** `C:\Users\N8_Da\OneDrive\Desktop\Project_Freyja\`
- **freyja-engine** is the only Cargo workspace member. `freyja-nnue` is NOT a member yet (empty directory, reserved for future NNUE crate).
- **freyja-ui** is a separate npm project (Vite + React + TypeScript), NOT part of the Cargo workspace.
- **Rust edition:** 2024 (requires Rust 1.85+, currently on 1.93.1)
- **tracing** is a dependency but NOT instrumented yet. No `tracing::info!` calls. First instrumentation in Stage 1 (`set_piece`/`remove_piece`).
- **Release profile** configured per ADR-011: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`. Use `cargo build --release` for performance builds.
- **Binary name:** `freyja` (not `freyja-engine`). Configured via `[[bin]]` in `freyja-engine/Cargo.toml`.

## API Contracts

- **Library crate** `freyja-engine` exports 7 public modules via `lib.rs`:
  - `board`, `eval`, `gamestate`, `mcts`, `movegen`, `protocol`, `search`
- **All modules are empty stubs.** No types, no functions, no traits defined yet.
- **Binary** prints `freyja v1.0 maxn-beam-mcts` and exits. No protocol handling yet (Stage 4).
- **Workspace version** inherited from root: `0.1.0`. Tagged version `v1.0` assigned after user green light.

## Known Limitations

- All 7 modules are empty — doc comments only, no types or functions.
- `main.rs` only prints protocol header — no stdin/stdout loop, no command parsing.
- UI is Vite default — no chess-specific components, no board rendering, no engine communication.
- No game logic whatsoever. First game logic arrives in Stage 1 (board representation).
- No tests beyond the test harness scaffold. First tests arrive in Stage 1.

## Performance Baselines

N/A — no engine logic to measure. First baseline set after Stage 2 (perft NPS).

## Open Questions

- **Module naming: `movegen` vs `move_gen`.** MASTERPLAN Section 6 example says `move_gen`, but Stage 0 build order says `movegen`. Currently using `movegen` per the stage spec. If a rename is needed, do it in Stage 2 when the module gets populated.

## Reasoning

- **freyja-nnue deliberately excluded from workspace:** MASTERPLAN says "(future)". Adding an empty crate as a workspace member would be over-engineering per AGENT_CONDUCT 1.8 rule 4.
- **Release profile from Stage 0:** Per ADR-011, free 10-20% release performance. Odin didn't add these until Stage 19.
- **Cargo.lock committed:** Removed from `.gitignore`. Freyja is an end-user binary, not a library consumed by third parties. Reproducible builds require committed lock file.
- **Protocol header in main.rs:** Establishes the pattern for Stage 4's protocol implementation. Provides meaningful binary output for verification.
