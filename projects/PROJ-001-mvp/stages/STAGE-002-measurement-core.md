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
- [ ] (not yet written) — Latency probe with HTTP RTT and TCP fallback
- [ ] (not yet written) — Buffer pool implementation
- [ ] (not yet written) — Cloudflare backend: real download/upload
- [ ] (not yet written) — Generic HTTP backend: real download/upload
- [ ] (not yet written) — Test orchestrator + headless JSON output
- [ ] (not yet written) — Failure mode tests

**Count:** 1 shipped / 0 active / 6 pending

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

*To be filled in when this stage ships.*
