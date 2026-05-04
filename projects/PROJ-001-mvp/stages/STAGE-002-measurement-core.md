---
stage:
  id: STAGE-002
  status: shipped
  priority: high
  target_complete: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-27
shipped_at: 2026-05-03

value_contribution:
  advances: "delivers the actual speedtest engine — without this stage there is no product, only scaffolding"
  delivers:
    - "real download throughput, upload throughput, and HTTP RTT measurements"
    - "valid `TestResult` JSON output via `--format json`"
    - "the snapshot-fan-out seam DEC-008 requires for v2 monitor mode"
    - "structured failure handling for network/protocol errors"
  explicitly_does_not:
    - "render anything beyond JSON — human/silent renderers are STAGE-003"
    - "pin performance budgets — that's STAGE-004"
    - "package or release anything — that's STAGE-005"
---

# STAGE-002: Measurement core

## What This Stage Is

Implement the actual speedtest: latency probe, parallel-connection
download, parallel-connection upload, the buffer pool, statistics
computation, and snapshot fan-out. End-to-end this stage produces a
working `--format json` output against Cloudflare and against the
mock server from SPEC-006.

## Why Now

STAGE-001 establishes the seams; without this stage filling them in
there's no product. The order matters: rendering (STAGE-003) consumes
the `TestResult` and `Snapshot` types this stage produces, so it
cannot start until those are stable.

## Success Criteria

`rspeed --format json` produces a valid `TestResult` JSON object
populated with real measurements. The same is true with
`--server <url>` against any HTTP server implementing the documented
protocol. Tests cover happy paths and key failure modes (timeout,
connection reset, malformed response).

Performance budgets are not yet pinned in this stage — that's Stage 4.
But this stage must produce code that *plausibly* meets the budgets,
i.e., uses the buffer pool from DEC-005 and respects the seams from
DEC-008.

## Scope

### In scope (anticipated specs)

To be drafted when STAGE-001 is mid-build. Names are placeholders.

| ID | Title | Estimated |
|---|---|---|
| SPEC-007 | `MetricsAccumulator` and `Snapshot` types | 3 hr |
| SPEC-008 | Latency probe with HTTP RTT and TCP fallback | 4 hr |
| SPEC-009 | Buffer pool implementation | 3 hr |
| SPEC-010 | Cloudflare backend: real download/upload | 4 hr |
| SPEC-011 | Generic HTTP backend: real download/upload | 3 hr |
| SPEC-012 | Test orchestrator + headless JSON output | 4 hr |
| SPEC-013 | Failure mode tests (timeout, reset, malformed) | 3 hr |

Roughly 24 hours of focused work, padded for cycle overhead.

### Explicitly out of scope

- Human / silent renderers (STAGE-003)
- TTY detection, color handling, error rendering (STAGE-003)
- Performance tuning, socket buffer tuning, throughput budget
  verification (STAGE-004)

## Spec Backlog

- [x] SPEC-007 (shipped 2026-04-28) — `MetricsAccumulator` and result types
- [x] SPEC-008 (shipped 2026-05-01) — Latency probe with HTTP RTT and TCP fallback
- [x] SPEC-009 (shipped 2026-05-02, PR #12) — Buffer pool implementation
- [x] SPEC-010 (shipped 2026-05-03, PR #13) — Cloudflare backend: real download/upload
- [x] SPEC-011 (shipped 2026-05-02, PR #14) — Generic HTTP backend: real download/upload
- [x] SPEC-012 (shipped 2026-05-03, PR #17) — Test orchestrator + headless JSON output
- [x] SPEC-013 (shipped 2026-05-03, PR #18) — Failure mode tests

**Count:** 7 shipped / 0 active / 0 pending — STAGE COMPLETE

## Critical invariants this stage establishes

These are the seams that downstream stages and v2 work depend on:

1. **`MetricsAccumulator` is decoupled from rendering.** It owns the
   counters; it emits `Snapshot` on a tokio interval; consumers are
   `watch::Receiver` subscribers. The accumulator does not know how
   many subscribers exist or what they do with the data.
2. **The orchestrator (`TestSession::run`) is invocation-agnostic.**
   A future `MonitorSession` (v2) wraps this in a loop with no
   measurement code changes.
3. **Failure modes return structured errors.** Every
   external-network-induced failure is a typed variant of a
   `TestError` enum so renderers can format it consistently.

## Dependencies

### Depends on

- STAGE-001 (Foundation) — `Backend` trait from SPEC-005, mock server
  from SPEC-006, and DECs 0001–0008 (especially DEC-005 buffer
  strategy and DEC-006 output struct).

### Enables

- STAGE-003 (Output & UX) — needs a populated `TestResult`, a live
  `Snapshot` stream, and error types that distinguish user-fixable
  from environment errors.

## Stage-Level Reflection

### Did STAGE-002 deliver the stage's `value_contribution`?

- **Real download/upload/HTTP RTT measurements** — Yes. SPEC-008 delivered HTTP RTT + TCP
  fallback latency. SPEC-010 (Cloudflare) and SPEC-011 (GenericHttp) delivered parallel
  download/upload throughput via `download_parallel`/`upload_parallel`. Verified end-to-end
  by `tests/orchestrator.rs` and `tests/failure_modes.rs` against the mock backend; live
  Cloudflare validation deferred to STAGE-004 per SPEC-013 scope decision.
- **Valid `TestResult` JSON output** — Yes. SPEC-012 delivered `TestSession::run()` and
  `lib::run()` with `--format json` producing a fully populated `TestResult` (all fields
  populated: `started_at`, `backend`, `server_url`, `ip_version`, `duration_secs`, `latency`,
  `download`, `upload`). Confirmed at `src/lib.rs:74` and `src/result.rs:14`. Note the
  brief's "JSON output consumed by a third-party script we did not write" success signal
  is **reachable in source** today; it becomes **reachable to external users** post-STAGE-005
  release.
- **Snapshot-fan-out seam (DEC-008)** — Yes. SPEC-007 delivered `MetricsAccumulator` +
  `watch::Sender<Snapshot>`; SPEC-012 wired it into the orchestrator and exposed
  `snapshot_rx()` at `src/orchestrator.rs:78`. Per-phase forwarders (one accumulator per
  phase, forwarded into the outer `watch` sender) keep the seam intact across phase
  boundaries with no subscriber coupling.
- **Structured failure handling** — Yes. SPEC-012 established `TestError` variants
  (`Config`, `Backend`, `Latency`, `Download`, `Upload`) and phase tagging; SPEC-013 proved
  all adversarial paths (stall, truncation, non-2xx) produce the correct typed variant
  end-to-end through the orchestrator stack. Six failure-mode tests in
  `tests/failure_modes.rs` cover the adversarial matrix.

### How many specs did it actually take?

7 (planned: 7). No splits, merges, or restructuring — the count held exactly. Each spec
shipped independently without needing to pull work forward or push work back.

### Are the three critical invariants intact?

1. **`MetricsAccumulator` decoupled from rendering (DEC-008 seam #1)** — Yes. Verified at
   `src/metrics.rs` (no rendering imports) and `src/orchestrator.rs:208` (the
   `spawn_forwarder` task forwards `Snapshot` values to the outer `watch::Sender` without
   the accumulator knowing how many subscribers exist).
2. **Orchestrator is invocation-agnostic (DEC-008 seam #2)** — Yes. `TestSession::run(&self)`
   at `src/orchestrator.rs:82` takes no per-invocation parameters and returns a `TestResult`,
   so a future `MonitorSession` can wrap it in a loop without measurement-code changes.
   SPEC-013 added `download_deadline`/`upload_deadline` fields and a `with_deadlines()`
   builder without altering the run signature.
3. **Failure modes return structured errors** — Yes. SPEC-013 closed the gap. Pre-SPEC-013,
   `BackendError::Timeout` was unreachable from download/upload paths because the
   orchestrator awaited the backend without a deadline. After SPEC-013, the
   `tokio::time::timeout(...)` wrappers in `run_download_phase`/`run_upload_phase` map
   elapsed deadlines to `TestError::Download(BackendError::Timeout(_))` and
   `TestError::Upload(BackendError::Timeout(_))` respectively. All six adversarial scenarios
   in `tests/failure_modes.rs` produce correctly typed variants.

### What changed between starting and shipping?

Six design corrections during the stage. Each was Frame-discovered and folded into Build
in the same commit (the pattern this stage's reflection is now codifying — see lessons
below):

- **SPEC-008**: `Backend::latency_probe` return type changed from `Vec<Duration>` to
  `LatencyProbeOutcome` (richer struct carrying `method` + `samples`); fallibility cascade
  applied to `CloudflareBackend::new()`, `GenericHttpBackend::new()`, and `select()`. DEC-003
  amended with both refinements rather than spawning new DECs.
- **SPEC-009**: Visibility pattern established — `pub` inside module, no top-level re-export
  from `lib.rs` unless needed by canonical API surface. `Clone` added to `BufferPool` so
  SPEC-010/011 can hand the pool to async tasks.
- **SPEC-010**: Upload RSS budget concern surfaced mid-build (single `Bytes::from(vec![0u8;
  25MB])` × 4 connections > 20MB RSS budget). DEC-005 Consequences amended with STAGE-004
  follow-up note. Rust 2024 RPIT lifetime capture rule (`+ use<>`) discovered and applied.
- **SPEC-011**: `_parallel` naming convention adopted (`throughput::download_parallel` /
  `upload_parallel`) to disambiguate from identically-named trait methods. No surprises in
  Build — SPEC-010's pattern transferred verbatim.
- **SPEC-012**: `lib::run()` rewritten for full orchestration; eager `--server` URL validation
  added via `Config::validate()` to fail loudly at CLI parse rather than silently on first
  request. `with_intervals(...)` extension point added to keep CI suite under 3s/test.
- **SPEC-013**: Deadline enforcement placed at the orchestrator (the spec's "Files to modify"
  list pointed to `throughput.rs`; Build correctly followed DEC-003's trait-boundary
  rationale instead — second Build deviation in the stage that was architecturally correct).
  Chainable `.with_deadlines(...)` builder followed the SPEC-012 B-1 pattern.

**Process learnings (not design):**

- **Test-count grep bug** (SPEC-011 → SPEC-012): `grep "running [0-9]+ tests"` silently
  skips single-test binaries (Cargo prints "running 1 test" singular). Counting from
  `test result:` summary lines fixes it. Verified Build Completion sections in subsequent
  specs include the net count (added − removed), not just the added total.
- **URL crate normalization**: bare-host URLs (`http://example.com`) silently get a
  trailing slash from `url::Url`. Validation test fixtures must use an explicit path
  component (`http://example.com/api`) to exercise rejection branches. Documented
  inline in SPEC-012's reflection.

### What did we defer?

- **HTTP/2 stall** (`cloudflare-http2-stall-on-parallel-download`, status: open, priority:
  high) → STAGE-004. SPEC-013's download-deadline test serves as a regression guard: any
  indefinite stall fails CI within the deadline.
- **Upload RSS budget over-allocation** → STAGE-004. DEC-005 Consequences carries the
  follow-up: replace one-shot `Bytes::from(vec![0u8; N])` with `reqwest::Body::wrap_stream()`
  yielding 256KB chunks.
- **`Url::join` trailing-slash UX normalization** → STAGE-004. Partial mitigation:
  `Config::validate()` rejects URLs without trailing-slash paths in SPEC-012. Full
  user-friendly normalization (auto-append, warn) deferred.
- **`live`-feature Cloudflare integration tests** → STAGE-004 (per SPEC-013 scope decision
  D). All STAGE-002 tests run against the mock backend; live-network validation belongs
  with the perf budgets work.
- **`throughput-warmup-duration`** (status: open, priority: low) → STAGE-004. The fixed 2s
  warm-up may be wasteful on fast links or insufficient on high-RTT links. Defer until
  STAGE-004 has measurements.
- **`human-format-json-fallback-cleanup`** (status: open, priority: low, blocks STAGE-003).
  `lib::run()`'s `Format::Human` arm currently falls through to JSON with a stderr warning
  (`src/lib.rs:81`). SPEC-014 (the human renderer) replaces this branch.

### Lessons codified at Stage Ship

The draft surfaced three candidates; all three landed in AGENTS.md in this session:

1. **Frame-outcomes-folded-into-Build pattern** — 6 of 7 specs followed it (SPEC-007
   through SPEC-013, with SPEC-011 trivially N/A — no Frame items to fold). AGENTS.md §15
   subsection expanded with: when it applies (tractable refinements), when it doesn't
   (structural rework → NO-GO Frame verdict), and how to mark it (resolution-letter table
   in spec body, cited in commit messages).
2. **`pub` module + no-top-level-re-export visibility convention** — codified as a new
   "Module visibility convention" subsection in AGENTS.md's rspeed-specific section. Two
   tiers (canonical API surface vs internal-but-test-reachable) with the default being
   internal-but-test-reachable.
3. **"When to trust a Build deviation" paragraph** — added to AGENTS.md §15. Frames
   deviations as signal (the spec's prescriptive file list misled but the underlying DEC
   rationale was right), with a ratify-or-reject discipline in Verify rather than reflexive
   reversion. Caveat included that this is not a license to under-invest in Frame.

A fourth candidate from SPEC-007's reflection — adding a "tokio paused-clock pattern"
checklist item to the spec template — is **not codified yet**. STAGE-002 had only one
new async-cadence spec (SPEC-007 itself); a second one would confirm or invalidate the
pattern. Reconsider if STAGE-003's progress-bar/animation work introduces a second
async-cadence spec, otherwise revisit at STAGE-005's template-revision pass.

### What's the natural next stage?

STAGE-003 (Output & UX). All dependencies are satisfied: `TestResult` fully populated
(✓ SPEC-012), live `Snapshot` stream via `snapshot_rx()` (✓ SPEC-007 + SPEC-012),
`TestError` variants with `exit_code()` mapping (✓ SPEC-012 + SPEC-013). The first STAGE-003
spec (SPEC-014, indicatif progress bars) inherits the `human-format-json-fallback-cleanup`
follow-up and will remove the SPEC-012 stub branch.
