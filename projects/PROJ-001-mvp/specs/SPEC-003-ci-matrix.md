---
task:
  id: SPEC-003
  type: chore
  cycle: frame
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
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
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
  - `macos-14` (arm64)
  - `macos-13` (x86_64)
  - `ubuntu-22.04`
  - `windows-latest`
- [ ] Each matrix entry runs in this order:
  1. Checkout (`actions/checkout@v4`)
  2. Install Rust toolchain via `dtolnay/rust-toolchain@stable` pinned
     to the project's MSRV from `rust-toolchain.toml`
  3. Cache (`Swatinem/rust-cache@v2`) keyed on `Cargo.lock`
  4. `cargo fmt --check`
  5. `cargo clippy --all-targets -- -D warnings`
  6. `cargo test`
  7. `cargo build --release`
- [ ] A `concurrency` group cancels superseded runs on the same PR/branch
- [ ] A separate `.github/workflows/release.yml` file exists as a stub
      with a placeholder `workflow_dispatch:` trigger and a TODO
      comment pointing to STAGE-005 (this prevents "we forgot to add
      release CI" later)
- [ ] A test commit on a throwaway branch produces all-green CI in
      under 5 minutes (after cache warm-up; first run is 3–5 min on
      each runner type)
- [ ] README.md gains a CI status badge linking to the workflow

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

- **Branch:**
- **PR:**
- **All acceptance criteria met?** <not yet built>
- **New decisions emitted:**
- **Deviations from spec:**
- **Follow-up work identified:**

### Build-phase reflection

1. **What was unclear that slowed you down?** —
2. **Constraint or decision that should have been listed but wasn't?** —
3. **If you did this task again, what would you do differently?** —

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
