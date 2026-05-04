---
stage:
  id: STAGE-002
  status: active
  priority: high
  target_complete: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-27
shipped_at: null

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

## Stage-Level Reflection (DRAFT — finalized at Stage Ship)

### Did STAGE-002 deliver the stage's `value_contribution`?

- **Real download/upload/HTTP RTT measurements** — Yes. SPEC-008 delivered HTTP RTT + TCP
  fallback latency. SPEC-010 (Cloudflare) and SPEC-011 (GenericHttp) delivered parallel
  download/upload throughput via `download_parallel`/`upload_parallel`.
- **Valid `TestResult` JSON output** — Yes. SPEC-012 delivered `TestSession::run()` and
  `lib::run()` with `--format json` producing a fully populated `TestResult`.
- **Snapshot-fan-out seam (DEC-008)** — Yes. SPEC-007 delivered `MetricsAccumulator` +
  `watch::Sender<Snapshot>`; SPEC-012 wired it into the orchestrator and exposed `snapshot_rx()`.
- **Structured failure handling** — Yes. SPEC-012 established `TestError` variants and phase
  tagging; SPEC-013 proved all adversarial paths (stall, truncation, non-2xx) produce the
  correct typed variant end-to-end through the orchestrator stack.

### How many specs did it actually take?

7 (planned: 7). No splits, merges, or restructuring — the count held exactly. Each spec
shipped independently without needing to pull work forward or push work back.

### Are the three critical invariants intact?

1. **`MetricsAccumulator` decoupled from rendering (DEC-008 seam #1)** — Yes. SPEC-007 owns
   the accumulator; it emits `Snapshot` on a watch channel with no subscriber coupling.
   SPEC-012 consumes it via `snapshot_rx()`. The accumulator has no knowledge of how many
   subscribers exist or what they do with the data.
2. **Orchestrator is invocation-agnostic (DEC-008 seam #2)** — Yes. `TestSession::run(&self)`
   is callable in a loop; a future `MonitorSession` wraps it without measurement code changes.
   SPEC-013 added deadline fields without altering the seam.
3. **Failure modes return structured errors** — Yes. SPEC-013 closed the gap. Pre-SPEC-013,
   `BackendError::Timeout` was unreachable from download/upload paths. All six adversarial
   scenarios now produce correctly typed `TestError` variants.

### What changed between starting and shipping?

- **SPEC-008**: `Backend::latency_probe` return type changed from `Vec<Duration>` to
  `LatencyProbeOutcome` (richer struct carrying method + samples for JSON output).
- **SPEC-009**: Visibility pattern established — `pub` inside module, no top-level re-export
  from `lib.rs` unless needed by tests or CLI.
- **SPEC-010**: Upload RSS budget concern surfaced mid-build → DEC-005 amended with STAGE-004
  follow-up note.
- **SPEC-011**: `_parallel` naming convention adopted to disambiguate `download_parallel` /
  `upload_parallel` from identically-named trait methods.
- **SPEC-012**: `lib::run()` rewritten for full orchestration; eager `--server` URL validation
  added via `Config::validate()`.
- **SPEC-013**: Deadline enforcement placed at the orchestrator (the spec's "Files to modify"
  list pointed to `throughput.rs`; Build correctly followed DEC-003 instead). Second Build
  deviation in the stage that was architecturally correct.

### What did we defer?

- HTTP/2 stall question (`cloudflare-http2-stall-on-parallel-download`) → STAGE-004
  (recorded in `guidance/questions.yaml`)
- Upload RSS budget over-allocation → STAGE-004 (in DEC-005 Consequences)
- `Url::join` trailing-slash UX normalization → STAGE-004 (partial mitigation: `Config::validate()`
  in SPEC-012; full normalization deferred)
- `live`-feature Cloudflare integration tests → STAGE-004 (per SPEC-013 scope decision D)

### Lessons worth considering for AGENTS.md / templates / constraints?

Flag for Stage Ship to decide — not decided here:

1. **"Frame outcomes folded into Build" pattern** used in 5 of 7 specs. It's now load-bearing.
   Worth promoting from §15's reference to a first-class section in AGENTS.md?
2. **`pub` module + no-top-level-re-export visibility pattern** (established SPEC-008/009/010/011)
   is a real project convention now. Worth codifying in the rspeed-specific section?
3. **Build deviations have been correct twice** (SPEC-012's `lib::run()` placement;
   SPEC-013's `orchestrator.rs` vs `throughput.rs`). Both times: the prescriptive file list
   was wrong; the DEC rationale was right. Worth a paragraph on "when to trust a Build
   deviation"?

### What's the natural next stage?

STAGE-003 (Output & UX). All dependencies are satisfied: `TestResult` fully populated
(✓ SPEC-012), live `Snapshot` stream (✓ SPEC-007 + SPEC-012), `TestError` variants with
phase tags and `exit_code()` mapping (✓ SPEC-012 + SPEC-013).
