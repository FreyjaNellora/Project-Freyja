# AGENT CONDUCT — AI/Agent Development Rules for Project Freyja

**Version:** 1.0
**Created:** 2026-02-25
**Status:** Active
**Heritage:** Derived from Project Odin AGENT_CONDUCT v1.0, with additions from 8 stages of lessons learned.

---

## 0. PREAMBLE

This document defines HOW AI agents behave while building Project Freyja. It is one of three core reference documents:

| Document | Defines | Authority Over |
|----------|---------|---------------|
| `MASTERPLAN.md` ([[MASTERPLAN]]) | WHAT each stage builds | Stage specs, acceptance criteria, architecture, tracing points |
| `4PC_RULES_REFERENCE.md` ([[4PC_RULES_REFERENCE]]) | The game rules | Board layout, piece movement, scoring, game modes |
| `AGENT_CONDUCT.md` (this) | HOW agents work | Behavior rules, audit procedures, code standards |

**Every agent that touches the codebase must read this document before beginning any work.**

This document does not duplicate the masterplan. It references it. If this document says "see MASTERPLAN Section 4" it means go read that section there, not that the content is copied here.

---

## 1. AGENT BEHAVIOR RULES

---

### 1.1 Stage Entry Protocol

Before writing a single line of code for any stage, follow these steps in order. Skipping steps causes cascading problems that compound across stages.

**Step 0: Orient yourself.** Read `STATUS.md` ([[STATUS]]) to know where the project is. Read `HANDOFF.md` ([[HANDOFF]]) to know what the previous session was doing. Read `DECISIONS.md` ([[DECISIONS]]) if you're new to the project or working on a stage where architectural decisions were made. This takes 5 minutes and prevents you from duplicating work or re-arguing settled decisions.

**Step 1: Read the stage specification** in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4. Understand:
- What this stage builds (deliverables)
- Build order (sequential steps)
- Acceptance criteria (definition of done)
- Tracing points (observation points to add)
- "What you DON'T need" (scope boundaries)

**Step 2: Read all upstream audit logs.** Trace the dependency chain from MASTERPLAN Section 3.1. For every stage this one depends on (direct and transitive), read `audit_log_stage_XX.md`. Look for:
- BLOCKING findings that affect this stage
- WARNING findings that might be relevant
- Known limitations flagged by prior auditors

**Step 3: Read all upstream downstream logs.** Same dependency chain. Read `downstream_log_stage_XX.md` for each. Look for:
- API contracts you must respect
- Known limitations you must work around
- Performance baselines you must not regress below
- Open questions that affect your stage

**Step 4: Build and test what exists.** Run:
```
cargo build
cargo test
```
If anything fails, STOP. Do not proceed with new work on a broken foundation. Record the failure in the pre-audit section of this stage's audit log.

**Step 5: Complete the pre-audit** section of `audit_log_stage_XX.md`. Record:
- Build state (compiles? tests pass?)
- Findings from upstream logs
- Risks identified for this stage

**Step 6: Begin implementation.**

---

### 1.2 The First Law: Do Not Break What Exists

Every commit must leave the project in a compilable, test-passing state. No exceptions. No "I'll fix it in the next commit."

**Permanent invariants** are defined in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4.1 (18 invariants, Stages 0-9). Consult that table for the full list. Key invariants for quick reference:

- **Stage 0:** Prior-stage tests never deleted.
- **Stage 2:** Perft values are forever. Zobrist make/unmake round-trip. Attack query API is the board boundary.
- **Stage 3:** Game playouts complete without crashes. Eliminated players never trigger movegen.
- **Stage 5:** UI owns zero game logic.
- **Stage 6:** Evaluator trait is the eval boundary. eval_scalar and eval_4vec agree.
- **Stage 7:** Searcher trait is the search boundary. Engine finds forced mates.
- **Stage 9:** NPS does not regress >15% between stages.

If a change in Stage N causes a test from Stage M (where M < N) to fail, that is a **blocking defect**. Fix it before proceeding.

If existing behavior genuinely needs to change (rare), the agent must:
1. Document exactly what changed and why in the audit log
2. Update all downstream consumers
3. Verify no test regressions
4. Flag it in the downstream log for future stages

---

### 1.3 Dependency Handling

**Consume APIs, do not peek at internals.** If Stage 7 needs evaluation, it calls `eval_4vec(state)` through the `Evaluator` trait. It does not reach into the eval module and read piece-square tables directly.

**Respect the "What you DON'T need" sections.** Each stage spec explicitly says what it should NOT build. Do not build things listed there.

**When a dependency is missing or insufficient:** STOP. Document this in the audit log as a blocking issue. Do not implement a workaround that duplicates or contradicts prior-stage responsibilities.

**Stub contracts.** Some stages create stubs for later stages to fill. The agent filling the stub must preserve the existing function signatures, parameter types, and return types. If the signature is wrong, document the needed change in the downstream log and get approval before modifying it.

---

### 1.4 Autonomy Boundaries

**Proceed autonomously when:**
- Implementing something explicitly described in the stage spec
- Writing tests for behavior described in the spec
- Fixing a bug that is clearly a defect (test fails, panic on valid input, wrong output for documented behavior)
- Adding tracing instrumentation at key boundaries
- Refactoring internal implementation without changing the public API

**Stop and ask when:**
- The stage spec is ambiguous or contradicts another document
- A change would modify a public API established by a prior stage
- A decision has long-term architectural implications not covered in the spec
- Performance is significantly worse than what the spec implies
- You discover a bug in a prior stage that requires non-trivial changes
- You want to add functionality not mentioned in the spec, even if it seems helpful
- Any change to `Cargo.toml` dependencies beyond what the spec requires

---

### 1.5 Code Standards

**Naming conventions are defined in MASTERPLAN Section 6. They are not optional.** Summary:

| Entity | Convention | Example |
|--------|-----------|---------|
| Rust modules | snake_case | `move_gen`, `board_repr` |
| Rust types | PascalCase | `GameState`, `MctsNode` |
| Rust functions | snake_case | `generate_legal_moves` |
| Rust constants | SCREAMING_SNAKE | `MAX_DEPTH`, `BEAM_WIDTH_DEFAULT` |
| UI components | PascalCase | `BoardDisplay`, `DebugConsole` |
| Protocol commands | lowercase | `bestmove`, `isready` |

**No mixed conventions within a module.**

**Term consistency across the entire codebase.** The project has a glossary (MASTERPLAN Section 7). Use those terms.

**Formatting and linting:**
- Run `cargo fmt` before every commit. No exceptions.
- Run `cargo clippy` and address all warnings before every commit. If a specific clippy lint is intentionally suppressed, add a comment explaining why.

**Visibility:**
- Default to private. Expose only what downstream stages need.
- Every `pub` item is a contract that future stages may depend on. Treat it as such.
- Document all `pub` items with a doc comment explaining what it does, not how.

**Constants over magic numbers.** If a number appears in logic code, give it a name:

| Literal | Named Constant |
|---------|---------------|
| `36` | `INVALID_CORNER_COUNT` |
| `196` | `TOTAL_SQUARES` |
| `160` | `VALID_SQUARES` |
| `14` | `BOARD_SIZE` |
| `8` | `MAX_QSEARCH_DEPTH` |
| `900` | `QUEEN_EVAL_VALUE` |

---

### 1.6 Commit Discipline

- Each commit should correspond to one item in the stage's build order, or one logically atomic change.
- Commit messages must reference the stage: `[Stage 02] Implement pseudo-legal pawn generation for all four directions`
- Never commit code that does not compile. Never commit code with failing tests.
- If a large change breaks things temporarily, use a feature branch and merge when stable.

---

### 1.7 Test Expectations

- Every acceptance criterion in the stage spec must have at least one corresponding test.
- Tests for prior stages must never be deleted or modified to make new code pass.
- Unit tests live in the module they test (`#[cfg(test)] mod tests`). Integration tests live in `freyja-engine/tests/`.
- Test names describe what they test: `test_checkmate_detected_when_no_legal_moves_and_in_check`, not `test_1`.
- Perft tests are integration tests that run in CI. They are never skipped.
- When a bug is found and fixed, add a regression test that reproduces the bug. Never fix a bug without a test.

---

### 1.8 Decision Principles

When trade-offs arise during implementation, these principles guide the decision. They are ordered by priority.

1. **Correctness before performance,** except where the spec defines explicit performance targets.
2. **"Reasonable" means within 10x of the target.** If the spec says "depth 7+ within 8 seconds" and you're at 80 seconds, that's a bug. If you're at 12 seconds, that's acceptable for now.
3. **If self-play shows regression, revert and document.** Minimum 100 games before declaring a change a regression or improvement. Use SPRT when available (Stage 12+).
4. **Under-engineer rather than over-engineer.** The spec is the scope. Do not build for hypothetical future requirements.
5. **When the spec is silent, prefer the simpler approach.**
6. **Record non-obvious decisions in `DECISIONS.md`.** If you chose approach A over approach B and someone might later wonder why, write it down.

---

### 1.9 Issue Lifecycle

**Issue staleness rule.** At the start of every session, scan [[MOC-Active-Issues]]. Any open **Blocking** or **Warning** issue whose `last_updated` field is older than 3 sessions without an update must be reviewed:
- If still relevant: update `last_updated`, add a status comment.
- If no longer relevant: resolve it with a note explaining why.
- If blocked: add a comment explaining what's blocking it.

**Note**-level issues are exempt from the 3-session staleness check.

**Issue creation checklist:**
1. Create the file in `issues/` using the template (`_templates/issue.md`).
2. Fill all fields — especially `severity`, `stage`, and `last_updated`.
3. Add the issue to [[MOC-Active-Issues]] under the correct severity heading.
4. Use existing [[wikilinks]] from the [[Wikilink-Registry]].
5. If a new wikilink target is needed, add it to [[Wikilink-Registry]] immediately.

**Verification gate.** An agent must NEVER claim a bug is fixed or mark an issue as `resolved` until the user has verified the fix through testing in the UI or self-play. Passing `cargo test` or a clean compile is necessary but not sufficient — runtime behavior must be confirmed by a human.

- After implementing a fix, tell the user what was changed and ask them to verify.
- Only after the user confirms: set `status: resolved`, set `date_resolved`, move to `## Recently Resolved` in [[MOC-Active-Issues]].
- If the user reports the fix doesn't work: update the `## Resolution` section with what was tried, set `status: open`, continue investigating.

**Stage completion gate.** A stage is NOT complete until the user gives the green light from testing in the UI. Compilation + tests passing + audit passing is necessary but not sufficient. The user must confirm that the engine behaves correctly through interactive testing.

---

### 1.10 Blocking Issue Resolution

When an audit finds a BLOCKING issue:

**BLOCKING found during pre-audit (before starting a new stage):**
1. Fix it before starting the current stage.
2. Record the fix in the prior stage's `audit_log_stage_XX.md` as an addendum.
3. Re-run all prior-stage tests.
4. Update `STATUS.md`.

**BLOCKING found during post-audit (after completing a stage):**
1. Fix it before marking the stage as complete.
2. Do NOT update `STATUS.md` to "complete" until resolved.
3. Re-run the full post-audit after the fix.

**BLOCKING found in a stage 3+ levels back:**
1. Escalate to human oversight. Do not attempt a fix autonomously.
2. Document the impact chain.
3. Record in `STATUS.md` as a blocking issue with full context.

---

### 1.11 Version Control

**Branching strategy:** Keep it simple.

- **Main branch** for all stable work. Every commit on main must compile and pass tests.
- **Feature branches** only when a large change breaks things temporarily. Name them `stage-XX-feature-name`. Merge back to main when stable.
- **No long-lived branches.**

**Tagging and versioning:**
- Tag each completed stage: `stage-00-complete` + `v1.0`, `stage-01-complete` + `v1.1`, etc.
- Tags are created AFTER the post-audit passes AND the user gives the green light.
- Tags are never moved or deleted.

**Version scheme:**
- `v1.0` = Stage 0 complete. Through `v1.20` = Stage 20 complete.
- **Major version bump** (`v2.0`): only if a rollback forces a rebuild from an earlier stage.

---

### 1.12 Wikilink Discipline

**The [[Wikilink-Registry]] is the single source of truth for all wikilink targets.**

**Rules:**
1. **Reuse before you create.** Check [[Wikilink-Registry]] for existing targets before creating new ones.
2. **New targets require registry updates.** Add immediately after creating a new file.
3. **No orphan links.** Every `[[target]]` must resolve to an actual file.
4. **No duplicate targets for the same concept.**
5. **Log entries get wikilinks.** Link to all relevant stage specs, components, decisions.
6. **Registry maintenance.** Update when files are renamed or deleted.

---

### 1.13 Vault Note Protocol

**Agents must create Obsidian vault notes to surface knowledge that would otherwise be buried in logs.**

| Trigger | Note type | Folder | Example |
|---|---|---|---|
| Every WARNING or BLOCKING audit finding | Issue | `issues/` | `Issue-EP-Representation-4PC.md` |
| Every component you implement or substantially modify | Component | `components/` | `Component-Board.md` |
| Every cross-component interaction | Connection | `connections/` | `Connection-Board-to-MoveGen.md` |
| Every non-obvious pattern or trick worth reusing | Pattern | `patterns/` | `Pattern-Pawn-Reverse-Lookup.md` |

**Every vault note must:**
1. Use the template from `_templates/` for its type
2. Link back to the relevant stage spec, audit log, and downstream log
3. Be added to [[Wikilink-Registry]] immediately
4. Be added to the relevant MOC

---

### 1.14 Session-End Protocol

Before ending any work session, complete these steps. This takes 5 minutes and saves the next session 30 minutes.

**Step 1: Update `HANDOFF.md`.**
Clear the file and rewrite with:
- What stage and build-order step you're on
- What was completed this session
- What was NOT completed
- Any open issues or discoveries
- Files modified
- What the next session should do first

**Step 2: Update `STATUS.md`.**
- Update "Current Stage" and "Current Build-Order Step"
- Update stage completion tracker if any stages were completed
- Update "What the Next Session Should Do First"
- Add any new blocking issues or regressions

**Step 3: Update `DECISIONS.md`** (if any architectural decisions were made).

**Step 4: Create a session note** in `masterplan/sessions/` documenting the session.

**Step 5: Commit management file updates.**
```
git add masterplan/STATUS.md masterplan/HANDOFF.md masterplan/DECISIONS.md
git commit -m "[Meta] Session-end status update"
```

This is the last commit of every session. No exceptions.

---

### 1.15 Debugging Discipline

When investigating a bug, agents must follow a structured process that prevents analysis paralysis. The goal: every analysis pass must either **narrow the hypothesis space** or **produce empirical evidence**. If it does neither, stop analyzing and start testing.

---

#### Rule 1: Maintain a Hypothesis Journal

After each analysis pass, state:
1. **Current hypothesis:** What you think the bug is. Must cite specific code locations (`file.rs:line`).
2. **What's new:** The specific code, value, path, or contradiction you discovered that you did not know before.
3. **What's eliminated:** Which prior hypotheses this pass ruled out.

**The spiral test:** If your hypothesis is identical to the previous pass and "what's new" is empty, you are spiraling. Stop analyzing and move to Rule 3.

**Rephrasing is not progress.** "The bug is in handle_go because it returns early" and "The issue is that handle_go has an early return path" are the same hypothesis.

---

#### Rule 2: Trust Hierarchy

When the user provides a diagnosis, implement their suggested fix FIRST. Test it. Only investigate alternatives if the fix fails.

The trust order:
1. **User's explicit diagnosis** — Implement it.
2. **Direct code reading** — What the code actually does, traced with specific values.
3. **Speculation** — Weakest form of evidence. Never spend more than one pass without converting to empirical evidence.

---

#### Rule 3: Empirical Escalation

**After two consecutive analysis passes where the hypothesis did not narrow, you must write a test.** Not "plan to write a test." Write it. Run it.

---

#### Rule 4: One Bug, One Focus

When investigating Bug A and you discover Bug B, **write Bug B down and continue working on Bug A.** Do not chase Bug B mid-investigation.

---

#### Rule 5: Scope Lock After Diagnosis

Once you have a diagnosis with a specific code location and a concrete fix plan, **stop analyzing and start implementing.** Implement → Test → Observe.

---

#### Recognizing the Spiral

You are spiraling if ANY of these are true:

| Signal | Example |
|---|---|
| Re-reading a function you already analyzed without a new question | Reading `maxn_search()` a third time "just to make sure" |
| Questioning evidence you already accepted | "But does the beam ordering really work?" after confirming it does |
| Exploring hypotheticals without tests | "What might happen if beam width is 1?" — write a test |
| Expanding scope | "While I'm here, let me also check territory scoring..." |
| Restating your conclusion | "So the issue is really that..." for the third time |

You are NOT spiraling if each pass cites a new code location, eliminates a hypothesis, or discovers a contradiction.

---

### 1.16 Deferred-Debt Escalation Rule

If any work item has been deferred for **2 or more consecutive stages**, it becomes a **mandatory escalation item**:

1. **Flag it loudly in HANDOFF.md** under a dedicated `## Deferred Debt` section. Include: what it is, how many stages deferred, WHY it is stuck, what would unblock it, whether the design itself might be flawed.
2. **Promote the issue severity.** NOTE → WARNING after 2 stages. WARNING → BLOCKING after 3, or explicitly record abandonment in DECISIONS.md.
3. **Tell the user directly.** Do not silently carry deferred debt.

---

### 1.17 4PC Verification Matrix (NEW — Freyja Addition)

**The problem this solves:** Odin had at least 8 critical bugs from assuming 2-player chess logic works for all 4 players. Pawn directions, en passant, castling paths, king/queen positions — all broke for specific players.

**The rule:** Every game rule, every move type, every special move, every board geometry check must have a test for ALL 4 player orientations independently. Not "works for Red, assumed for others."

**Verification matrix format:**

| Rule | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Pawn forward | ✅ | ✅ | ✅ | ✅ |
| Pawn double-step | ✅ | ✅ | ✅ | ✅ |
| En passant | ✅ | ✅ | ✅ | ✅ |
| Castling KS | ✅ | ✅ | ✅ | ✅ |
| Castling QS | ✅ | ✅ | ✅ | ✅ |
| Promotion rank | ✅ | ✅ | ✅ | ✅ |
| Attack directions | ✅ | ✅ | ✅ | ✅ |

**Every cell must have a dedicated test.** Not one test that loops through players — four explicit tests, one per player, each verifying the correct orientation-specific behavior.

The matrix is tracked in the stage's audit log and updated as new rules are implemented.

---

### 1.18 Protocol Change Procedure (NEW — Freyja Addition)

**The problem this solves:** Odin's protocol parser silently dropped extended messages (`eliminated Red checkmate` parsed as color `Red checkmate`, failed validation, silently discarded).

**The rule:** Any protocol change must:
1. Update engine output AND UI parser in the same commit
2. Add an integration test for the new/changed message format
3. The parser must extract tokens by position, not by assuming fixed format
4. Unknown or extended fields must be ignored gracefully, never cause a crash or silent drop

---

### 1.19 Performance Regression Bounds (NEW — Freyja Addition)

**The problem this solves:** Odin tracked NPS baselines but had no explicit threshold for when a regression becomes blocking.

**The rule:** NPS must not regress by more than 15% between stages. If it does:
1. Investigate the cause (new overhead, allocation pattern, etc.)
2. If the regression is inherent to the new stage's functionality (e.g., quiescence adds overhead): document and accept, but only up to 30% total from the post-Stage-9 baseline.
3. If the regression is accidental: fix before marking the stage complete.

Performance baselines are recorded in every stage's downstream log and tracked in STATUS.md.

---

### 1.20 Tier Boundary Review (NEW — Freyja Addition)

**The problem this solves:** Moving between tiers (Foundation → Search → Strategic → etc.) is a major phase transition. The foundation should be verified as rock-solid before building on it.

**The rule:** Before starting the first stage of a new tier, run a tier boundary review:
1. Run ALL maintenance invariants (MASTERPLAN Section 4.1)
2. Review ALL open issues in [[MOC-Active-Issues]]
3. Confirm ALL data structures in hot paths are fixed-size (no Vec in Board, GameState, or MoveUndo)
4. Record the review in a dedicated `tier_boundary_review_N.md` in masterplan/
5. Get user sign-off before proceeding

---

### 1.21 Context Management (NEW — Freyja Addition)

**The problem this solves:** An agent can burn its entire context window and produce a garbage handoff with 2% context remaining.

**The rule:** If context utilization is high and you are in the middle of a stage:
1. Complete the current build-order item if close to done
2. Begin session-end protocol (Section 1.14)
3. A clean handoff at 80% context is worth more than an extra 20% of work with a broken handoff
4. Never start a new build-order item if you cannot reasonably complete it within remaining context

---

### 1.22 UI/Engine Co-Evolution (NEW — Freyja Addition)

**The problem this solves:** Odin had two critical bugs from one-sided protocol changes — engine output changed but UI parser wasn't updated.

**The rule:** Any change to engine protocol output requires:
1. A matching UI parser update in the same commit
2. An integration test that sends the new format and verifies correct parsing
3. If the UI is not yet built (pre-Stage 5): create a protocol conformance test file that the UI must pass when built

---

### 1.23 Rollback Procedure (NEW — Freyja Addition)

**When to roll back:**
- Self-play shows >5% win rate regression over 200+ games
- A stage introduces a bug in a prior stage that cannot be fixed without reverting
- The user explicitly requests a rollback

**How to roll back:**
1. Create a branch from the current state: `abandoned-stage-XX-attempt-N`
2. Tag the abandoned state for reference
3. Reset to the last known-good stage tag: `git checkout stage-XX-complete`
4. Document in DECISIONS.md: what was attempted, why it was rolled back, what to try differently
5. Increment the major version if the rollback crosses 3+ stages

---

## 2. COMPREHENSIVE AUDIT CHECKLIST

---

### 2.0 Audit Philosophy

An audit is not a rubber stamp. It is adversarial review. The auditor assumes bugs exist and tries to find them.

**Severity levels:**
- **BLOCKING** — Must fix before the next stage begins.
- **WARNING** — Should fix. Will likely cause problems in a future stage.
- **NOTE** — Observation for the record. No action required now.

---

### 2.1 through 2.26: Audit Categories

The following categories are carried forward from Project Odin. Each is described briefly here; see the full Odin AGENT_CONDUCT Section 2 for detailed guidance on what to look for.

| # | Category | Key Focus |
|---|----------|-----------|
| 2.1 | Cascading Issues | Changed APIs → all callers updated? |
| 2.2 | Iterative Degradation | Functions growing beyond original purpose? |
| 2.3 | Code Bloat | More code than necessary? |
| 2.4 | Redundancy | Two implementations of the same thing? |
| 2.5 | Dead Code | Functions defined but never called? |
| 2.6 | Broken Code | Compiles but produces wrong results? |
| 2.7 | Stale References | References to things that changed or were removed? |
| 2.8 | Naming Inconsistencies | Mixed casing, abbreviation drift? |
| 2.9 | Conflicting Code | Two pieces that contradict each other? |
| 2.10 | Before-and-After Audit | Compare metrics before/after stage work |
| 2.11 | Trait and Interface Contract Violations | Clone, PartialEq, Evaluator, Searcher contracts |
| 2.12 | Unsafe Unwraps and Panics | `.unwrap()` in engine code? |
| 2.13 | Test Coverage Gaps | Acceptance criteria without tests? |
| 2.14 | Performance Regressions | NPS dropped? Allocations in hot paths? |
| 2.15 | Memory Concerns | Unbounded growth? MCTS tree size? |
| 2.16 | Feature Flag Contamination | Feature-gated code leaking into default builds? |
| 2.17 | Board Geometry Errors | Corner squares, ray generation, 4-player geometry |
| 2.18 | Zobrist Hash Correctness | Make/unmake round-trip, incremental vs full recompute |
| 2.19 | Thread Safety Preparation | Global mutable state, Rc usage |
| 2.20 | Import and Dependency Bloat | Unnecessary crates, glob imports |
| 2.21 | Circular Dependencies | Module A imports B, B imports A |
| 2.22 | Magic Numbers | Literal numbers without names |
| 2.23 | Error Handling Gaps | Protocol input, FEN4 parsing, NNUE weight loading |
| 2.24 | API Surface Area Creep | More `pub` than necessary? |
| 2.25 | Documentation/Code Drift | Comments describing old behavior? |
| 2.26 | Semantic Correctness | Evaluation symmetry, move generation completeness |

---

### 2.27 Beam Search Correctness (NEW — Freyja Addition)

Code that implements beam search but violates its contract.

**What to look for:**
- Beam width correctly limits the number of expanded children at each node
- Beam width 30+ (all moves) produces the same result as no beam at shallow depths
- Move ranking for beam selection uses the correct evaluator (not a stale or wrong eval)
- Beam selection does not accidentally filter captures or checks that quiescence should see
- Beam width parameter is propagated correctly through all recursive calls
- Iterative deepening correctly reuses TT best move as first candidate regardless of beam

---

### 2.28 4PC Verification Matrix Completeness (NEW — Freyja Addition)

**What to look for:**
- Does the 4PC verification matrix have a ✅ for every rule × every player?
- Are orientation-specific values correct (pawn directions, castling paths, promotion ranks)?
- Are there tests that exercise each cell independently (not just a loop)?
- Do tests verify INCORRECT orientations are rejected (not just correct ones accepted)?

---

## 3. OBSERVABILITY

The engine uses the `tracing` crate from day one. No custom telemetry. Odin learned this lesson (Huginn system deferred 8 stages, then retired — ADR-015).

### 3.1 Tracing Usage

- Use `tracing::debug!` for search/eval diagnostic output
- Use `tracing::info!` for high-level events (search start/complete, position set)
- Use `tracing::trace!` for verbose per-node data (only in development)
- All tracing calls are zero-cost when filtered out at runtime

### 3.2 Environment Configuration

```
RUST_LOG=freyja_engine=debug    # Development
RUST_LOG=freyja_engine=info     # Normal operation
RUST_LOG=freyja_engine=trace    # Verbose debugging
```

### 3.3 Diagnostic Gameplay Observer Protocol

**Who runs diagnostics:** ONLY the top-level orchestrating agent may start the engine, build the project, or run diagnostic games. Subagents MUST NOT independently start/stop the engine, run `cargo build`, spawn the engine binary, or modify engine state while another agent is working.

**Engine protocol logging (Stage 4+):**
- Enable: `setoption name LogFile value <path>`
- Disable: `setoption name LogFile value none`
- Format: `> incoming_command` and `< outgoing_response` per line, timestamped
- Zero overhead when disabled

**Max Rounds auto-stop (Stage 4+):**
- `setoption name MaxRounds value <n>` stops the game after N rounds
- 20 rounds (80 ply) is usually enough to see behavioral patterns

**Diagnostic workflow:**
1. Build engine: `cargo build --release`
2. Configure observer: set depth, game mode, eval profile, ply limit
3. Run observer: spawns engine, plays game via protocol, captures outputs
4. Review outputs: protocol log, structured game JSON, summary markdown
5. Compare behavioral metrics against baselines (pawn ratio, queen activation, captures)
6. If regression detected: create issue in `masterplan/issues/`, investigate, fix, re-run

**Log naming convention:** `{mode}_{profile}_d{depth}_{games}games_{timestamp}.log`

**When to run diagnostics:**
- After any eval change (zone control features, weight adjustments)
- After any search change (beam width, pruning, MCTS parameters)
- Before and after stage completion (before/after baselines)
- When a user reports unexpected behavior

---

## 4. WHAT AUTOMATED TRACING CANNOT CATCH

These categories require human or agent judgment.

| Category | What Goes Wrong | How to Catch |
|---|---|---|
| Architectural Drift | Codebase deviates from MASTERPLAN architecture | Compare actual dependency graph against MASTERPLAN Section 3 at every stage boundary |
| Wrong Abstractions | Abstraction makes the right things hard | When downstream work feels like fighting upstream abstractions, flag it |
| Over-Engineering | Building for hypothetical futures | "Does the MASTERPLAN require this?" If no, don't build it. |
| Under-Engineering | Shortcuts that cost more later | Cross-reference stage spec's key types. Divergence = flag. |
| Algorithmic Correctness | Max^n sign convention, beam selection, UCB1 formula | Manual review against MASTERPLAN spec. Hand-computed test cases. |
| Performance Pathology | Correct but 100x slower than necessary | Profile with `cargo flamegraph`. Performance baselines in downstream logs. |
| Design Intent Violations | Works but doesn't serve project goals | Test against quantitative targets, not just correctness. |
| Readability | Next agent can't understand the code | "Read it cold" test: understandable within 60 seconds per function? |

---

## 5. TIER-SPECIFIC CONDUCT NOTES

### 5.1 Tier 1: Foundation (Stages 0-5)

- **Correctness over speed, clarity over cleverness.**
- **Establish invariants with exhaustive tests.** Board validity, move legality, Zobrist correctness.
- **Do not optimize.** If perft is slow, that's fine.
- **GameState must be cheaply cloneable.** Fixed-size arrays, no Vec.
- **The Freyja Protocol is the contract between engine and UI.** Treat it as a stable interface.
- **The UI owns ZERO game logic.**
- **4PC Verification Matrix must be complete** before leaving Tier 1.

### 5.2 Tier 2: Core Search (Stages 6-9)

- `Evaluator` and `Searcher` traits are interface contracts that persist through the entire project.
- **Keep the bootstrap eval simple.** NNUE replaces it. But include zone control basics (territory) because they become NNUE features.
- **Max^n must be fully working before Stage 8.** Quiescence layers on top.
- **Beam width = 30 (all moves) must produce correct results.** Narrower beams are an optimization that must not change correctness at shallow depths.
- Track the **beam width ↔ depth ↔ node count** relationship empirically.

### 5.3 Tier 3: Strategic Layer (Stages 10-11)

- MCTS implements the same `Searcher` trait. It must work standalone before integration.
- **Stage 11 must not change Max^n or MCTS internals.** It is a controller layer.
- Log disagreements between Max^n and MCTS (when they pick different best moves).

### 5.4 Tier 4: Measurement (Stages 12-13)

- **Self-play is infrastructure, not a feature.** Every subsequent stage uses it.
- From this point forward, significant changes validated by self-play.
- Document beam width tuning experiments with data.

### 5.5 Tier 5: Intelligence (Stages 14-17)

- **Stage 15 is architecture and inference only.** No training. Random weights to verify pipeline.
- **Stage 17 is the most dangerous swap.** Before-and-after audit mandatory.
- As NNUE improves, beam width should tighten and depth should increase. Measure this.

### 5.6 Tier 6: Polish (Stages 18-20)

- **Variant tuning isolated behind `GameMode` enum,** not scattered through search code.
- **Full UI still owns ZERO game logic.**
- **Optimization is profile-first.** Never optimize without measurement.

---

## 6. AUDIT LOG AND DOWNSTREAM LOG PROCEDURES

### 6.1 How to Fill the Audit Log

**Pre-Audit:** Build state, previous downstream flags reviewed, findings, risks for this stage.

**Post-Audit:** Deliverables check, code quality (uniformity, bloat, efficiency, dead code, broken code, temporary code), search/eval integrity, future conflict analysis, unaccounted concerns, reasoning & methods, issue resolution.

**Specific observations are required.** Do not write "looks fine." Write: "Checked all 12 public functions for naming consistency per Section 2.8 — all follow snake_case convention."

### 6.2 How to Fill the Downstream Log

**Must-Know:** Critical facts that would cause problems if missed.
**API Contracts:** Every public function/type downstream stages will use.
**Known Limitations:** What this stage does NOT do that someone might expect.
**Performance Baselines:** Timing and throughput numbers that must not regress.
**Open Questions:** Unresolved design questions.
**Reasoning:** Why decisions were made.

---

## 7. APPENDICES

### Appendix A: Quick-Reference Card

**Before starting any stage (Section 1.1):**
0. Orient: read STATUS.md, HANDOFF.md, DECISIONS.md
1. Read stage spec in MASTERPLAN
2. Read upstream audit logs
3. Read upstream downstream logs
4. `cargo build && cargo test` — all must pass
5. Fill pre-audit section
6. Begin work

**Decision priority order (Section 1.8):**
1. Correctness > Performance > Elegance
2. Spec-defined > Agent-inferred > Unspecified
3. Measured improvement > Theoretical improvement
4. Simple + working > Clever + fragile
5. Existing patterns > Novel patterns
6. Reversible > Irreversible

**When debugging (Section 1.15):**
1. User gave diagnosis? → Implement it first.
2. Each pass must cite something NEW. No new citation = spiral.
3. Two passes without narrowing → write a test.
4. One bug at a time. Write Bug B down. Fix Bug A.
5. Have a fix plan? Stop analyzing. Implement → Test → Observe.

**Before ending any session (Section 1.14):**
1. Update HANDOFF.md
2. Update STATUS.md
3. Update DECISIONS.md (if applicable)
4. Create session note
5. Commit: `[Meta] Session-end status update`

---

### Appendix B: Common Freyja-Specific Pitfalls

| Pitfall | What Goes Wrong | Where It Matters |
|---------|----------------|-----------------|
| **Pawn direction reversal** | Red +rank, Blue +file, Yellow -rank, Green -file. Sign error = backward pawns. | Stage 2 (movegen), Stage 6 (PST) |
| **Corner square validity** | 36 specific squares invalid. Four 3x3 corners. | Stage 1, Stage 2 |
| **Three-way check** | Must check attacks from 3 opponents, not 1. | Stage 2, Stage 3 |
| **Promoted queen dual value** | 1 point capture, 900cp eval. Two systems. | Stage 3, Stage 6 |
| **DKW timing** | Dead king moves happen between turns, not as a full turn. | Stage 3 |
| **Castling for 4 players** | 8 rights, each player has own back rank and orientation. | Stage 2, Stage 1 |
| **En passant with 4 players** | Clear on next move, even if another player's turn. | Stage 2, Stage 1 |
| **Eliminated player movegen crash** | Calling generate_legal on a kingless board = panic. | Stage 3, Stage 7 |
| **Beam width = 0** | Edge case: no moves pass beam filter = panic/empty result. | Stage 7 |
| **Score vector comparison** | `[i16; 4]` — each player maximizes THEIR component, not the whole vector. | Stage 7, Stage 10 |
| **Zobrist side-to-move** | 4 players, not 2. Hash depends on which of 4 players moves next. | Stage 1, Stage 2 |
| **Stalemate scoring** | 20 points in FFA. Not a draw. Not zero. | Stage 3, Stage 6 |
| **React ref mutation timing** | Snapshot refs BEFORE mutations. setState updaters run asynchronously. | Stage 5 |
| **Protocol extension parsing** | Extract first token only. `eliminated Red checkmate` → color is `Red`. | Stage 4, Stage 5 |

---

*End of Agent Conduct v1.0*
