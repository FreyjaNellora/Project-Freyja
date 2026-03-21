# MOC — Active Issues

> All open bugs, concerns, and tracked items.

**Last scanned:** 2026-03-21 (Session 26)

---

## Blocking

*None.*

> [[Issue-Tauri-IPC-Hang]] resolved 2026-03-21 — moved to Recently Resolved.

---

## Warning

- [[Issue-UI-Feature-Gaps]] — UI missing Debug Console, Engine Internals needed for Stages 8-10. Prioritized list with Odin source references. (Session 10)

---

## Note

- [[Issue-Depth4-Engine-Crash]] — Engine crashes at depth 4 from qsearch explosion. Works at depth 2-3. To be addressed in Stage 13. (Session 18)

---

## Recently Resolved

- [[Issue-Tauri-IPC-Hang]] — Tauri IPC hung at ply 30+ due to undrained stderr pipe buffer. Fixed Session 26: stderr drain thread + FEN4 position commands. (Resolved 2026-03-21)
- [[Issue-UI-AutoPlay-Broken]] — Start button did nothing. Fixed Session 12: memoized `useEngine()`, destructured stable callbacks, reordered declarations, switched to release binary. (Resolved 2026-03-07)

---

## Staleness Check

Per AGENT_CONDUCT 1.9: At the start of every session, scan this list. Any Blocking or Warning issue whose `last_updated` is older than 3 sessions must be reviewed and updated.

---

**Related:** [[MOC-Project-Freyja]], [[AGENT_CONDUCT]]
