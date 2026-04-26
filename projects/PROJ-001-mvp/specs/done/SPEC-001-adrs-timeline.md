# SPEC-001 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-001-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-25 — critique of all 8 DECs produced; outcomes inlined into Build (rather than separate commits) per the documentation-only nature of this spec

- [x] **design** — completed 2026-04-25 — n/a beyond the spec body itself; documentation-only spec, no failing tests required

- [x] **build** — completed 2026-04-25 — commit b07ac6d on feat/spec-001-adrs (14 files, +166/-25); Frame outcomes applied inline to DECs and Stage-1 specs

- [x] **verify** — completed 2026-04-25 — first pass produced PUNCH LIST (DEC-004↔DEC-006 JSON path; cost.sessions backfill; timeline staleness); fixes applied at commit 3cc4e01; ✅ approved on re-verify 2026-04-25

- [x] **ship** — completed 2026-04-25 — Build reflection backfilled; Ship reflection added; cost.totals computed (session_count: 6); stage backlog updated; spec archived to specs/done/
