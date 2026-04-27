# SPEC-005 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-005-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-26 — Opus critique, 12 inline edits + DEC-003 update folded into Build per SPEC-001/002/003/004 precedent. agent: claude-opus-4-7

- [x] **design** — n/a — Frame outcomes serve as design

- [x] **build** — completed 2026-04-26 — branch `feat/spec-005-backend-trait`; 16 tests passing (2 unit + 13 cli + 1 version); 884K stripped binary (LTO dead-code-elim strips unused tokio/reqwest/rustls; will grow in STAGE-002). agent: claude-sonnet-4-6

- [ ] **verify** — pending fresh-session verify (Opus recommended — substantive consistency sweep across trait + error + Send+Sync + cross-spec wires)

- [ ] **ship** — pending verify
