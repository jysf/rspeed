---
insight:
  id: DEC-006
  type: decision
  confidence: 0.90
  audience:
    - developer
    - agent

agent:
  id: claude-opus-4-7
  session_id: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-25
supersedes: null
superseded_by: null

tags:
  - api
  - output
  - schema
---

# DEC-006: Output formats — one struct, three renderers

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

rspeed has three output formats:

- `human` (default): colored, progress bars during the test, tidy
  summary block at the end. For interactive use.
- `json`: a single JSON object printed to stdout when the test
  completes. For monitoring scripts and pipelines.
- `silent`: nothing printed; exit code communicates success/failure.
  For "run on a cron, only ping me if it failed" workflows.

A live snapshot also flows during the test (every 100ms by default)
to drive the human-mode progress bars and to fan out to any future
consumers (DEC-008's deferred TUI dashboard, an alerting hook, etc.).

We want the three renderers to be derived from a single source of
truth so that the JSON schema and the human-mode summary cannot drift.

## Decision

Define a single canonical result type in `src/result.rs`:

```rust
pub struct TestResult {
    pub started_at: DateTime<Utc>,
    pub backend: String,            // "cloudflare" | "generic"
    pub server_url: String,
    pub ip_version: String,         // "ipv4" | "ipv6" — which family was actually used
    pub duration_secs: f64,         // actual measurement window, excluding warm-up

    pub latency: LatencyResult,
    pub download: Option<ThroughputResult>,
    pub upload: Option<ThroughputResult>,
}

pub struct LatencyResult {
    pub method: String,             // "http_rtt" | "tcp_connect"
    pub samples: usize,
    pub median_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub jitter_ms: f64,             // sample stddev
}

pub struct ThroughputResult {
    pub mbps: f64,                  // mean over the measurement window (post-warm-up)
    pub mbps_p50: f64,              // sliding-window median
    pub mbps_p95: f64,              // sliding-window p95
    pub bytes: u64,                 // total transferred during the measurement window
    pub connections_configured: usize,  // count requested via --connections
    pub connections_active: usize,      // count still alive at end of test
}
```

`TestResult` derives `Serialize` and is written to stdout for `--format json`.

For live updates during the test, define `Snapshot`:

```rust
pub struct Snapshot {
    pub elapsed: Duration,
    pub phase: Phase,               // Latency | Download | Upload
    pub current_mbps: f64,
    pub bytes_so_far: u64,
}
```

A `tokio::sync::watch::Sender<Snapshot>` is owned by the orchestrator
and broadcast to subscribers (the human-mode progress bar in MVP;
future TUI dashboards; future alerting hooks).

Renderers live in `src/output/`:

- `output/human.rs` — indicatif progress bars driven by `Snapshot`,
  final summary block from `TestResult` with owo-colors styling
- `output/json.rs` — `serde_json::to_writer_pretty(stdout, &result)`
- `output/silent.rs` — does nothing; exit code conveys outcome

## Throughput warm-up window

TCP slow-start ramps the congestion window over the first few seconds
of a connection. If we report the mean throughput over the full test
duration, the warm-up undershoot drags the number down — a 1 Gbps
link reports ~600–800 Mbps for a 10s test. This is a known speedtest
pitfall.

The orchestrator therefore observes a **2-second warm-up window** at
the start of each throughput phase: bytes still flow and progress is
visible to the renderer, but the measurement window for `mbps`,
`mbps_p50`, `mbps_p95`, and `bytes` excludes those bytes. The JSON
output's `duration_secs` reflects the actual measurement window, not
the configured `--duration`. This is the dominant difference between
"fits in the family of 'roughly correct' speedtest tools" and "users
trust the number." STAGE-002's `MetricsAccumulator` owns this; the
exact warm-up duration may be tuned in STAGE-004 perf work.

## Consequences

- The JSON schema is exactly the `TestResult` Serialize output. This
  is the public contract; any field rename is a breaking change.
- *Field additions* are non-breaking: monitoring scripts using `jq`
  with named keys are forward-compatible by default. Adding a new
  optional field in a minor release is acceptable; renaming or
  removing requires a major version bump.
- We commit to documenting the JSON schema in the README and
  bumping major version on schema breaks.
- Renderers can never display data that's not in `TestResult` /
  `Snapshot`. If a renderer needs new data, it goes in those types
  first (and gets exposed in JSON automatically).
- A future ratatui dashboard (deferred per DEC-008) is a fourth
  renderer reading the same `Snapshot` stream. No coupling concerns.
