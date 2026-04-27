# SPEC-004 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-004-<cycle>.md`.

## Instructions

- [x] **frame** — 2026-04-26 — Opus 4.7 critique; verdict GO; 10 inline edits folded into Build per SPEC-002/003 precedent. agent: claude-opus-4-7

- [x] **design** — n/a — Frame outcomes serve as design (spec body updated with all decisions inline)

- [x] **build** — 2026-04-26 — commit on `feat/spec-004-cli-surface`. agent: claude-sonnet-4-6
  - src/cli.rs, src/config.rs created; src/lib.rs updated
  - 12 tests: 11 in tests/cli.rs (6 exit-code + 5 snapshot) + 1 existing version test
  - cargo fmt --check: clean; cargo clippy -- -D warnings: clean
  - cargo build --release: 884K stripped binary

- [x] **verify** — 2026-04-26 — ✅ APPROVED; PR #4 promoted to ready; CI green (macos-15 ✅ ubuntu-24.04 ✅ windows-2025 ✅); all 10 Frame outcomes confirmed; no unwrap/expect in library code; Cli not exported; snapshots read cleanly. agent: claude-sonnet-4-6

- [x] **ship** — 2026-04-26 — archived to specs/done/; PR #4 ready-for-review; 9 CLI flags shipped via clap derive; Cli/Config two-struct split; 11 tests green on 3 OSes. agent: claude-sonnet-4-6

## Cost sessions

- frame: agent: claude-opus-4-7, interface: claude-ai, tokens_total: null, estimated_usd: null, note: "Opus frame critique session 2026-04-26"
- build: agent: claude-sonnet-4-6, interface: claude-code, tokens_total: null, estimated_usd: null, note: "Build session 2026-04-26; run /cost at end of session"
