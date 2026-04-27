# SPEC-005 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-005-<cycle>.md`.

## Instructions

- [x] **frame** — completed 2026-04-26 — Opus critique, 12 inline edits + DEC-003 update folded into Build per SPEC-001/002/003/004 precedent. agent: claude-opus-4-7

- [x] **design** — n/a — Frame outcomes serve as design

- [x] **build** — completed 2026-04-26 — commit `fab6af6` on `feat/spec-005-backend-trait`; 16 tests passing (2 unit + 13 cli + 1 version); 884K stripped binary (LTO dead-code-elim strips unused tokio/reqwest/rustls; will grow in STAGE-002). agent: claude-sonnet-4-6

- [x] **verify** — completed 2026-04-27 — ✅ APPROVED on PR #5; CI green on macos-15 + ubuntu-24.04 + windows-2025 (4m9s wall, cross-check x86_64-apple-darwin passed); all 16 tests pass; cargo fmt + clippy --all-targets -D warnings clean; 884K binary confirmed (vacuous AC pass — LTO/DCE strips unused tokio/reqwest/rustls; meaningful binary-size budget moves to STAGE-002 when download/upload/latency_probe wire in real reqwest); lint-scope finding (clippy --all-targets DOES lint tests/) handled with file-scope `#![allow]` in tests/cli.rs + tests/version.rs — already captured in MEMORY and worth a paragraph in AGENTS.md Style on next template revision; cross-spec consistency clean (DEC-001 tokio features, DEC-002 reqwest features, DEC-003 select() return type + #[non_exhaustive], SPEC-006 GenericHttpBackend::new(Url) + name()=="generic" surface all aligned; axum correctly absent from SPEC-005 Cargo.toml). agent: claude-opus-4-7

- [x] **ship** — completed 2026-04-26 — archived to specs/done/; PR #5 ready-for-review; stage backlog updated (5 of 6 STAGE-001 specs shipped). agent: claude-sonnet-4-6
