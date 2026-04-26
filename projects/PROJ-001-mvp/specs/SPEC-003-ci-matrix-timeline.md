# SPEC-003 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-003-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-26. Written critique; 6 resolutions (runner versions refreshed, dtolnay/rust-toolchain dropped, cargo build --release dropped, timeout-minutes: 15, permissions: contents: read, Linux arm64 deferred) folded into Build per SPEC-001/SPEC-002 precedent. Prompt was inlined into the Build prompt by the architect.

- [x] **design** — n/a. Frame outcomes serve as design; this is a configuration-only spec with no code.

- [x] **build** — completed 2026-04-26. Branch feat/spec-003-ci-matrix. Created ci.yml (4-OS matrix), release.yml (stub), README badge, KNOWN_LIMITATIONS entries. Gates: fmt/clippy/test all clean locally. CI behavioral verification is a Verify-cycle gate.

- [ ] **verify** — pending fresh-session verify (includes observing the actual CI run go green on the pushed branch/PR)

- [ ] **ship** — pending verify
