# SPEC-002 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-002-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-26 — written critique produced 5 decisions + 3 bonus items; all folded into Build (rather than a separate Frame commit) per the SPEC-001 precedent

- [x] **design** — completed 2026-04-26 — n/a beyond the spec body; Frame outcomes serve as design (School B dep landing, MSRV pin, version bump, DEC-002 inline refinement, <1MB binary check)

- [x] **build** — completed 2026-04-26 — commit `<hash>` on `feat/spec-002-cargo-skeleton`; School B Cargo skeleton landed, gates clean, release binary 358K stripped

- [ ] **verify** — pending fresh-session re-verify

- [ ] **ship** — pending verify
