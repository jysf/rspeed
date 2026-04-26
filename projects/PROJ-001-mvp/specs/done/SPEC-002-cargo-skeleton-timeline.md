# SPEC-002 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-002-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-26 — written critique produced 5 decisions + 3 bonus items; all folded into Build (rather than a separate Frame commit) per the SPEC-001 precedent

- [x] **design** — completed 2026-04-26 — n/a beyond the spec body; Frame outcomes serve as design (School B dep landing, MSRV pin, version bump, DEC-002 inline refinement, <1MB binary check)

- [x] **build** — completed 2026-04-26 — commit 559445f on `feat/spec-002-cargo-skeleton`; School B Cargo skeleton landed, gates clean, release binary 358K stripped

- [x] **verify** — completed 2026-04-26 — APPROVED; all AC met, all gates green, release binary 358K stripped, Frame outcomes correctly applied, DEC-002 inline refinement landed, constraint sweep clean. Three downstream cross-spec drift items flagged (stale "axum dev-dep" refs in SPEC-006, missing unknown-flag test in SPEC-004, unenumerated deps in SPEC-005) — not SPEC-002 blockers, but should be addressed before those specs' Build cycles.

- [x] **ship** — completed 2026-04-26 — PR opened on `feat/spec-002-cargo-skeleton`; Build reflection backfilled, Ship reflection appended, cost.totals computed (4 sessions, null-numeric per AGENTS.md §4 convention), three downstream cross-spec drift items (SPEC-004 unknown-flag test, SPEC-005 Cargo.toml deps enumerated, SPEC-006 'axum dev-dep' refs rewritten) folded inline since SPEC-002's School B Frame outcome caused them; stage backlog bumped 1→2 shipped; spec archived to `specs/done/`.
