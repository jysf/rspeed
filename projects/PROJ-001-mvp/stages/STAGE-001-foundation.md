---
stage:
  id: STAGE-001
  status: shipped
  priority: high
  target_complete: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-25
shipped_at: 2026-04-27

value_contribution:
  advances: "establishes the buildable substrate so all subsequent stages can stop reinventing project structure, dependency choices, CI, and the backend seam"
  delivers:
    - "an `cargo run -- --help`-able binary with the full CLI flag matrix"
    - "all-green CI on macOS arm64, macOS x86_64, Linux x86_64, Windows x86_64"
    - "the eight DECs committed and indexed"
    - "a `Backend` trait with stub implementations and a `select()` factory"
    - "an axum-based mock server for integration tests"
  explicitly_does_not:
    - "send any real network traffic — that's STAGE-002"
    - "render any output beyond debug-printing the resolved config"
    - "verify performance budgets — that's STAGE-004"
---

# STAGE-001: Foundation

## What This Stage Is

Establish the substrate: a buildable Rust project with all dependencies
locked, CI on all four platforms, the CLI surface defined, the backend
trait stubbed, and an integration test harness ready for Stage 2 to
build on. No actual measurement code lands in this stage.

## Why Now

This is the first stage on rspeed and every other stage depends on it.
Without it, every subsequent spec would re-decide project structure,
dependency choices, and CI shape — and we'd get drift across stages.

## Success Criteria

- A developer can clone the repo, run `cargo test` and `cargo run -- --help`
  on macOS, see clean output, and CI is green on all four runners
- All eight DECs are committed and visible in `decisions/`
- The binary, when run, parses CLI args, selects a backend (Cloudflare
  or Generic based on `--server`), and prints "not yet implemented"
  when asked to actually run a test — that's a feature, not a bug
- A `tests/common/MockServer` exists that future stages can spin up
  in integration tests without thinking about ports or shutdowns

## Scope

### In scope

- Six specs, in dependency order:
  - SPEC-001: Architecture decision records
  - SPEC-002: Cargo project skeleton
  - SPEC-003: CI matrix on GitHub Actions
  - SPEC-004: CLI surface with clap derive
  - SPEC-005: Backend trait and concrete stubs
  - SPEC-006: Integration test harness with mock server

### Explicitly out of scope

- Any HTTP traffic to a real network — Stage 2
- Any rendering / output formatting — Stage 3
- Any performance work — Stage 4
- Any release pipeline — Stage 5

## Spec Backlog

- [x] SPEC-001 (shipped on 2026-04-25) — Architecture decision records
- [x] SPEC-002 (shipped on 2026-04-26) — Cargo project skeleton
- [x] SPEC-003 (shipped on 2026-04-26) — CI matrix on GitHub Actions
- [x] SPEC-004 (shipped on 2026-04-26) — CLI surface with clap derive
- [x] SPEC-005 (shipped on 2026-04-26) — Backend trait and concrete stubs
- [x] SPEC-006 (shipped on 2026-04-27) — Integration test harness with mock server

**Count:** 6 shipped / 0 active / 0 pending

## Dependency order

```
SPEC-001 (ADRs)
   ↓
SPEC-002 (Cargo skeleton)
   ↓
SPEC-003 (CI)  ←── runs concurrently with the rest after SPEC-002
   ↓
SPEC-004 (CLI surface)
   ↓
SPEC-005 (Backend trait)
   ↓
SPEC-006 (Test harness)
```

SPEC-003 (CI) can be picked up in parallel with SPEC-004 onwards —
having CI live early catches downstream mistakes. The hard ordering is
SPEC-001 → SPEC-002, then SPEC-003/4/5/6 in any reasonable order.

## Design Notes

- The DECs (DEC-001 through DEC-008) are the authoritative architecture
  reference; this stage's job is to commit them and start referencing
  them in code, not to relitigate.
- The Backend trait shape is intentionally provisional — Stage 2 will
  evolve it as concrete needs emerge. SPEC-005's Implementation Context
  flags the parts most likely to change.
- Avoid the temptation to wire up logging, env-var parsing, or any
  other "foundational nice-to-have" not on the spec list. Each addition
  is a future maintenance burden and most aren't needed for MVP.

## Dependencies

### Depends on

- None — this is the foundational stage.

### Enables

- STAGE-002 (Measurement core) replaces the `Backend` stubs with real
  impls, adds the `MetricsAccumulator` and `Snapshot` types, implements
  the latency probe, downloader, uploader, and uses the test harness
  from SPEC-006 in unit and integration tests.

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?** Yes. Buildable
  Rust project with `cargo test` and `cargo run -- --help` clean on
  macOS arm64 (cross-compile-validated for x86_64); CI green on macOS
  arm64 + Ubuntu 24.04 + Windows 2025 in <2 min wall-clock; all eight
  DECs committed and indexed; `Backend` trait with `CloudflareBackend` +
  `GenericHttpBackend` stubs + `select()` factory; axum-based `MockServer`
  integration test harness implementing DEC-003's Generic backend protocol.

- **How many specs did it actually take?** 6 specs as planned. No splits,
  no surprises requiring follow-on specs within STAGE-001.

- **What changed between starting and shipping?** Eight DEC inline
  refinements during Build cycles (none requiring superseding DECs):
  DEC-001 (`sync` rationale), DEC-002 (reqwest 0.12→0.13,
  `rustls-tls`→`rustls`, aws-lc-rs provider note, confidence 0.90→0.85),
  DEC-003 (`Send + Sync` on `dyn Backend`, `#[non_exhaustive]` Consequence),
  DEC-004 (JSON path `latency_method`→`latency.method`), DEC-005
  ("comfortable" headroom + RSS arithmetic), DEC-006 (`ip_version` field,
  `connections` split, throughput warm-up, additions-non-breaking note),
  DEC-007 (cargo-binstall mention). One scope reclassification: macOS
  x86_64 primary→secondary tier (paid Intel runner avoided;
  cross-compile-validated). One MSRV update (1.85.0→1.91.0 because the
  Frame critique caught the original was 14 months stale).

- **Lessons that should update AGENTS.md, templates, or constraints?**
  Eight lessons captured — most landing as AGENTS.md additions, one as a
  `guidance/questions.yaml` entry. The Frame-outcomes-inlined-into-Build
  pattern is now codified in §15. Fresh-session Verify discipline is
  reinforced in §16 with the 5-for-5 catch record.

- **Should any spec-level reflections be promoted to stage-level
  lessons?** All eight lessons were promoted from per-spec reflections (or
  from the verify session that flagged them); spec-level reflections
  themselves stay as-is in `specs/done/`.
