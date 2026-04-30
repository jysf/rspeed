# SPEC-007 Build Prompt

You are running the **Build** cycle for SPEC-007. Your job is to write code
that makes the failing tests pass. No new scope; no refactoring beyond what
the spec requires.

## Read these first (in order)

1. `projects/PROJ-001-mvp/specs/SPEC-007-metrics-accumulator.md` — the spec;
   pay close attention to `## Implementation Context` and `## Failing Tests`
2. `decisions/DEC-006-output-formats.md` — authoritative field names/types
3. `decisions/DEC-008-deferred-tui.md` — seam 1; the watch-channel constraint
4. `decisions/DEC-005-buffer-strategy.md` — warm-up window rationale
5. `src/backend/mod.rs` — BackendError (context only; not a dep of this spec)
6. `src/lib.rs` — current public API to extend
7. `Cargo.toml` — current deps; two changes land here
8. `guidance/constraints.yaml`

## What to build

### 1. `Cargo.toml` — dep changes

- Promote `serde = { version = "1", features = ["derive"] }` from
  `[dev-dependencies]` to `[dependencies]`
- Add `chrono = { version = "0.4", features = ["serde"] }` to `[dependencies]`
- Leave `serde_json` in `[dev-dependencies]` (test-only per AGENTS.md convention)

### 2. `src/result.rs` — new file

Define exactly per DEC-006:

```rust
pub struct TestResult { ... }       // started_at: DateTime<Utc>, backend, server_url,
                                    // ip_version, duration_secs, latency, download, upload
pub struct LatencyResult { ... }    // method, samples, median_ms, min_ms, max_ms, jitter_ms
pub struct ThroughputResult { ... } // mbps, mbps_p50, mbps_p95, bytes, connections_configured,
                                    // connections_active
pub struct Snapshot { ... }         // elapsed: Duration, phase: Phase, current_mbps, bytes_so_far
pub enum Phase { Latency, Download, Upload }
```

All public result types: `#[non_exhaustive]`, derive `Serialize + Deserialize + Debug`.
`Snapshot` and `Phase`: `#[non_exhaustive]`, derive `Debug + Clone + PartialEq`.
Implement `Default for Snapshot` (phase: Phase::Latency, all zeros) and
`Default for Phase` (Phase::Latency).

`pub fn compute_latency_result(method: &str, samples: &[Duration]) -> LatencyResult`
— panics with a descriptive message on empty slice; computes median (sort a
copy, take middle), min, max, jitter (sample stddev, Bessel-corrected for n>1,
0.0 for n==1). All values in milliseconds as f64.

Add `#[cfg(test)]` unit tests in this file:
- `compute_latency_result_basic`
- `compute_latency_result_jitter`
- `test_result_serializes_to_json`
- `snapshot_default_is_all_zero`

### 3. `src/metrics.rs` — new file

```rust
struct AccumulatorInner {
    state: Mutex<AccumulatorState>,
    tx: watch::Sender<Snapshot>,
}

#[non_exhaustive]
#[derive(Clone)]
pub struct MetricsAccumulator {
    inner: Arc<AccumulatorInner>,
    interval: Duration,
    warmup: Duration,
}

struct AccumulatorState {
    phase: Phase,
    started_at: std::time::Instant,
    interval_bytes: u64,
    total_bytes: u64,
    bytes_at_warmup_end: Option<u64>,  // baseline-snapshot pattern
    samples: Vec<f64>,                 // per-interval Mbps, post-warmup only
}
```

Public API (see spec AC-4 through AC-11 for exact contracts):
- `new(interval: Duration, warmup: Duration) -> Self`
- `subscribe(&self) -> watch::Receiver<Snapshot>`
- `start_ticking(&self) -> tokio::task::JoinHandle<()>`
- `record_bytes(&self, n: u64)`
- `set_phase(&self, phase: Phase)`
- `finish(&self, connections_configured: usize, connections_active: usize) -> ThroughputResult`

Key implementation notes:
- `record_bytes` only increments `total_bytes` and `interval_bytes` — never
  touches warmup bookkeeping
- Tick handler sets `bytes_at_warmup_end = Some(total_bytes)` once on the
  first tick where `started_at.elapsed() >= warmup`, then pushes `current_mbps`
  to `samples` on every post-warmup tick
- `finish()` computes `bytes = total_bytes - bytes_at_warmup_end.unwrap_or(total_bytes)`
- p50/p95: sort samples in-place, `samples[n/2]` and `samples[(n*95)/100]`;
  return 0.0 for all stats if samples is empty (warmup > test duration)
- `current_mbps` formula: `(interval_bytes * 8) as f64 / interval.as_secs_f64() / 1e6`

### 4. `tests/metrics.rs` — new file

Write the 10 failing tests from the spec (9 original + `abort_stops_ticking`).
All timing-sensitive tests use `#[tokio::test(start_paused = true)]` +
`tokio::time::advance`. File-level `#![allow(clippy::unwrap_used, clippy::expect_used)]`.

See spec `## Failing Tests` section for the exact test descriptions and
assertions — implement them precisely, including the ε-tolerance assertions
in `finish_computes_mean_and_percentiles`.

### 5. `src/lib.rs` — modify

Add:
```rust
pub mod metrics;
pub mod result;
pub use metrics::MetricsAccumulator;
pub use result::{
    compute_latency_result, LatencyResult, Phase, Snapshot, TestResult, ThroughputResult,
};
```

## Definition of done

```bash
cargo test                                    # all tests pass
cargo clippy --all-targets -- -D warnings     # clean
cargo fmt --check                             # clean
```

Check that `tests/metrics.rs` has file-scope `#![allow(...)]` for clippy lints
that apply to test code (per AGENTS.md testing-discipline convention).

## When done

1. Fill in `## Build Completion` in the spec:
   - Branch name, PR number (if opened)
   - All ACs met? (check each one)
   - Any deviations from spec
   - Follow-up work identified
2. Append a build cost session entry to `cost.sessions` in the spec frontmatter.
3. Run `just advance-cycle SPEC-007 verify`.
4. Open a PR targeting `main`. PR description must include:
   - Project: PROJ-001
   - Stage: STAGE-002
   - Spec: SPEC-007
   - Decisions referenced: DEC-005, DEC-006, DEC-008
   - Constraints checked: test-before-implementation,
     no-new-top-level-deps-without-decision

## End of session

End your final response with:

```
Cost capture — run `/cost` in this session, then paste:
just record-cost SPEC-007 build --tokens-input <N> --tokens-output <N> --usd <N.NN>
```
