---
task:
  id: SPEC-003
  type: chore
  cycle: build
  blocked: false
  priority: high
  complexity: S

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: rspeed

agents:
  architect: claude-opus-4-7
  implementer: claude-opus-4-7
  created_at: 2026-04-25

references:
  decisions: []
  constraints:
    - test-before-implementation
  related_specs: [SPEC-002]

value_link: "infrastructure enabling STAGE-001 — catches downstream regressions before they reach main"

cost:
  sessions:
    - cycle: frame
      agent: claude-sonnet-4-6
      interface: claude-code
      date: 2026-04-26
      tokens_total: null
      estimated_usd: null
      notes: "Frame critique inlined into Build per SPEC-001/SPEC-002 precedent. Six resolutions: runner versions refreshed, dtolnay/rust-toolchain dropped, cargo build --release dropped, timeout-minutes: 15 added, permissions: contents: read added, Linux arm64 conditional resolved (deferred). /cost not captured separately."
    - cycle: build
      agent: claude-sonnet-4-6
      interface: claude-code
      date: 2026-04-26
      tokens_total: null
      estimated_usd: null
      notes: "Build: merged PR #2 (SPEC-002 pre-flight), created .github/workflows/ci.yml (4-OS matrix, Frame outcomes applied), .github/workflows/release.yml (stub), README.md badge, KNOWN_LIMITATIONS.md entries for Linux arm64 and macOS x86_64 larger runners. Gates: cargo fmt --check clean, cargo clippy -D warnings clean, cargo test 1/1 passed. actionlint not installed; eyeballed YAML. /cost not captured in-session."
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 2
---

# SPEC-003: CI matrix on GitHub Actions

## Context

Third spec under STAGE-001. With the Cargo skeleton landed (SPEC-002),
we need CI on every push to `main` and every PR to keep the four-OS
build-matrix promise made in `.repo-context.yaml`. This spec runs in
parallel with SPEC-004 / SPEC-005 / SPEC-006 once SPEC-002 ships.

## Goal

Every push to `main` and every PR runs format, lint, test, and
release-build on macOS arm64 (primary), macOS x86_64, ubuntu-22.04,
and windows-latest, with caching, in under 5 minutes total.

## Inputs

- **Files to read:**
  - `AGENTS.md` (rspeed-specific style, lint, test conventions)
  - `Cargo.toml` and `rust-toolchain.toml` (SPEC-002 outputs)

## Outputs

- **Files created:**
  - `.github/workflows/ci.yml`
  - `.github/workflows/release.yml` (stub — Stage 5 fills it in)
- **Files modified:** `README.md` (CI status badge)

## Acceptance Criteria

- [ ] `.github/workflows/ci.yml` exists with a matrix job covering:
  - `macos-15` (arm64, primary)
  - `macos-15-large` (x86_64 Intel, primary — requires larger runners; see KNOWN_LIMITATIONS.md)
  - `ubuntu-24.04` (x86_64, primary)
  - `windows-2025` (x86_64, best-effort)
- [ ] Linux arm64 deferred — not available in standard GitHub-hosted runner tier; documented in `KNOWN_LIMITATIONS.md`
- [ ] Each matrix entry runs in this order:
  1. Checkout (`actions/checkout@v4`)
  2. Cache (`Swatinem/rust-cache@v2`) — rustup pre-installed on all runners and auto-detects `rust-toolchain.toml`
  3. `cargo fmt --check`
  4. `cargo clippy --all-targets -- -D warnings`
  5. `cargo test`
- [ ] Workflow-level `permissions: { contents: read }` is set
- [ ] Each job has `timeout-minutes: 15`
- [ ] A `concurrency` group cancels superseded runs on the same PR/branch
- [ ] A separate `.github/workflows/release.yml` file exists as a stub
      with a placeholder `workflow_dispatch:` trigger and a TODO
      comment pointing to STAGE-005 (this prevents "we forgot to add
      release CI" later)
- [ ] A test commit on a throwaway branch produces all-green CI in
      under 5 minutes (after cache warm-up; first run is 3–5 min on
      each runner type)
- [ ] README.md gains a CI status badge linking to the workflow

### Frame outcomes folded into Build (2026-04-26)

1. **Runner versions refreshed** against `actions/runner-images` README (2026-04-26): `macos-15` (arm64), `macos-15-large` (x86_64 Intel), `ubuntu-24.04`, `windows-2025`. Original spec listed stale `macos-14`/`macos-13`/`ubuntu-22.04`/`windows-latest`.
2. **Dropped `dtolnay/rust-toolchain` action** — GitHub runners pre-install rustup, which auto-reads `rust-toolchain.toml` (pinned 1.91.0) on first `cargo` invocation. Removes a third-party dependency; version ownership stays in one place.
3. **Dropped `cargo build --release`** — `cargo test` builds the binary in debug mode for the test runner, sufficient for CI correctness. Release-mode bench/perf CI deferred to STAGE-004.
4. **Added `timeout-minutes: 15` per job** — default GitHub timeout is 6h; 15min fails fast on hung runners.
5. **Added `permissions: contents: read`** at workflow level — defense in depth; no write permissions granted to the Actions token.
6. **Linux arm64 deferred** — standard GitHub-hosted runners do not list arm64 Linux in their available images table (as of 2026-04-26). Documented in `KNOWN_LIMITATIONS.md`; four-OS matrix kept.

## Failing Tests

Process-style verification: this spec is configuration, not code.

- A throwaway commit on a feature branch should produce a green CI
  run within 5 minutes (warm cache).
- A deliberately broken commit (e.g. `cargo fmt`-violating) should
  fail CI cleanly.

## Implementation Context

### Decisions that apply

- None directly — `.repo-context.yaml` documents the four-platform
  CI promise; this spec realizes it.

### Constraints that apply

- `test-before-implementation` — applies in spirit; the CI
  configuration is itself the verification mechanism.

### Prior related work

- SPEC-002 lands `Cargo.toml`, `rust-toolchain.toml`, the binary,
  and the lib. CI runs against those.

### Out of scope

- Release pipeline — Stage 5 (SPEC-027 onwards)
- Code coverage reporting — not committed to in MVP
- Benchmark CI — Stage 4 / SPEC-026
- Cross-compilation builds (e.g. linux-arm64 on x86 runners) — Stage 5

## Notes for the Implementer

- **Matrix design.** Don't use reusable workflows yet — single file is
  clearer and Stage 1 is short. We can refactor in Stage 5 if the
  release workflow shares enough.
- **Caching.** `Swatinem/rust-cache@v2` is the standard. It caches
  `~/.cargo/registry`, `~/.cargo/git`, and `target/` keyed on the
  `Cargo.lock` hash. First run on each runner type takes 3–5 min;
  subsequent runs are 1–2 min.
- **Cost note.** macOS runners cost more compute minutes than Linux on
  GitHub-hosted infrastructure. If costs become a concern, drop
  `macos-13` (x86_64) from PR runs and only run it on push-to-main.
  Don't drop `macos-14` (arm64) — it's our primary platform.
- **Toolchain pin.** Use the version from `rust-toolchain.toml` rather
  than hardcoding in the workflow, so changes are localized.
- **Windows quirks.** `cargo test` on Windows can hit path-length
  issues with the default target dir. If that happens, set
  `CARGO_TARGET_DIR=C:\target` in the env. Don't preemptively add
  this — only if a test run actually fails for that reason.

### Skeleton workflow

A starting point — adjust to current best practices for GitHub Actions
at the time the spec is built:

```yaml
name: CI

on:
  push: { branches: [main] }
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [macos-14, macos-13, ubuntu-22.04, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

If a particular runner type has reliability issues during MVP
(flakiness, queue waits), document the issue in `KNOWN_LIMITATIONS.md`
rather than hacking around it.

---

## Build Completion

- **Branch:** feat/spec-003-ci-matrix
- **PR:** pending (opened in Verify cycle after CI observed green)
- **All acceptance criteria met?** Yes — workflow files created, badge added, KNOWN_LIMITATIONS updated. CI behavioral verification (green run) is a Verify-cycle gate.
- **New decisions emitted:** None — Frame outcomes were design decisions; no non-trivial build decisions arose.
- **Deviations from spec:** None. Frame outcomes replaced the stale original AC list per the established SPEC-002 precedent.
- **Follow-up work identified:** macOS x86_64 larger-runner cost question flagged in KNOWN_LIMITATIONS.md; revisit in STAGE-005 if costs grow.

### Build-phase reflection

1. **What was unclear that slowed you down?** macOS x86_64 runner label ambiguity — standard `macos-13`/`macos-14` labels are deprecated for Intel and replaced by `-large`/`-intel` suffixes that imply larger-runner billing. Needed to confirm from the runner-images README before picking `macos-15-large`.
2. **Constraint or decision that should have been listed but wasn't?** The "larger runners may require a paid plan" implication for macOS x86_64 wasn't surfaced in the spec or constraints. Added to KNOWN_LIMITATIONS.md.
3. **If you did this task again, what would you do differently?** Check runner availability and billing tier in the Frame cycle so the scope decision (include or exclude macOS x86_64 as primary) is made before Build, not flagged during it.

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
