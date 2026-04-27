# SPEC-006 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-006-<cycle>.md`.

## Instructions

- [x] **frame** — 2026-04-27 — Sonnet critique, GO with 8 inline AC folds per SPEC-001..005 precedent; agent: claude-sonnet-4-6

- [x] **design** — n/a (design folded into spec during Frame)

- [x] **build** — 2026-04-27 — commit `c9155b1` on `feat/spec-006-test-harness`; 20 tests passing (16 prior + 4 new smoke); binary still 884K; agent: claude-sonnet-4-6

- [x] **verify** — 2026-04-27 — ✅ APPROVED; PR #6 CI all-green (macos-15, ubuntu-24.04, windows-2025 + x86_64-apple-darwin cross-check); all 8 Frame outcomes confirmed; serde/serde_json dev-dep deviation defensible (School B-aligned); agent: claude-sonnet-4-6

- [x] **ship** — 2026-04-27 — archived to specs/done/; STAGE-001 complete (6/6 specs shipped); Stage Ship cycle next
