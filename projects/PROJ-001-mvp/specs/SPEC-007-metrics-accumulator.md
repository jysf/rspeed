---
task:
  id: SPEC-007
  type: story
  cycle: verify
  blocked: false
  priority: high
  complexity: M
  estimated_hours: 3

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: rspeed

agents:
  architect: claude-sonnet-4-6
  implementer: claude-opus-4-7
  created_at: 2026-04-27

references:
  decisions: [DEC-005, DEC-006, DEC-008]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-005, SPEC-006]

value_link: "foundation type layer enabling every other STAGE-002 measurement spec"

cost:
  sessions:
    - cycle: design
      date: 2026-04-27
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Spec authoring session; drafted types, MetricsAccumulator API, failing tests"
    - cycle: frame
      date: 2026-04-27
      agent: claude-opus-4-7
      interface: claude-ai
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Frame: punch list (5 items); architect resolved all; promoted to build"
    - cycle: build
      date: 2026-04-28
      agent: claude-opus-4-7
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Build: 10 integration tests + 4 unit tests pass; clippy/fmt clean. Tokio test-util added as dev-dep for paused-clock helpers."
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 3
---

# SPEC-007: `MetricsAccumulator` and result types

## Context

STAGE-001 shipped the `Backend` trait and a mock server. Those are seams; there
is no measurement yet. STAGE-002 fills them with real logic. Before any
measurement code can land, the *types* that measurement code produces must
exist: the result structs consumers will read, the snapshot type that drives
live progress display, and the accumulator that owns the per-connection byte
counters and emits snapshots on a timer.

DEC-006 specifies the shape of `TestResult`, `ThroughputResult`, `LatencyResult`,
and `Snapshot`. DEC-008 mandates three seams; **seam 1** is the subject of this
spec: `MetricsAccumulator` is decoupled from rendering, emits `Snapshot` via
`tokio::sync::watch`, and does not know what subscribes to it.

This spec defines everything *downstream* specs (SPEC-008 latency probe,
SPEC-009 buffer pool, SPEC-010/011 real backends, SPEC-012 orchestrator) will
import. It must ship before any of them can start.

## Goal

Define the canonical result types (`TestResult`, `ThroughputResult`,
`LatencyResult`, `Snapshot`, `Phase`) per DEC-006 and implement
`MetricsAccumulator` — a `Clone + Send + Sync` handle that collects byte
counters from parallel tasks, emits `Snapshot` snapshots on a configurable
timer, and returns a `ThroughputResult` on completion.

## Inputs

- **`decisions/DEC-006-output-formats.md`** — authoritative shapes for
  `TestResult`, `ThroughputResult`, `LatencyResult`, `Snapshot`
- **`decisions/DEC-008-deferred-tui.md`** — seam 1 definition; the watch-channel
  fan-out requirement; "accumulator does not know how many subscribers exist"
- **`decisions/DEC-005-buffer-strategy.md`** — buffer pool context; 2-second
  warm-up window rationale
- **`src/backend/mod.rs`** — `BackendError` variants (this spec does not introduce
  `TestError`; that's SPEC-012)
- **`src/lib.rs`** — current public API surface; this spec extends it

## Outputs

- **Files created:**
  - `src/result.rs` — `TestResult`, `ThroughputResult`, `LatencyResult`,
    `Snapshot`, `Phase`; `compute_latency_result()` helper; `Serde` derives on
    all public result types
  - `src/metrics.rs` — `MetricsAccumulator` and its private `AccumulatorState`
  - `tests/metrics.rs` — integration tests (see **Failing Tests**)

- **Files modified:**
  - `src/lib.rs` — `pub mod result; pub mod metrics;` + re-exports of
    `TestResult`, `ThroughputResult`, `LatencyResult`, `Snapshot`, `Phase`,
    `MetricsAccumulator`
  - `Cargo.toml` — move `serde` with `derive` feature to `[dependencies]`
    (currently dev-only; needed for `TestResult: Serialize`); add
    `chrono = { version = "0.4", features = ["serde"] }` to `[dependencies]`

- **New exports (from `src/lib.rs`):**
  ```rust
  pub use result::{
      TestResult, ThroughputResult, LatencyResult, Snapshot, Phase,
      compute_latency_result,
  };
  pub use metrics::MetricsAccumulator;
  ```

## Acceptance Criteria

- [ ] **AC-1:** `src/result.rs` defines `TestResult`, `ThroughputResult`,
  `LatencyResult`, `Snapshot`, `Phase` with the exact field names and types
  from DEC-006. All public result types derive `Serialize` (serde) and
  `Debug`. `TestResult` and both sub-structs also derive `Deserialize` (needed
  for test assertions and future use). All public result/snapshot types are
  `#[non_exhaustive]` (consistent with STAGE-001 public-API convention).

- [ ] **AC-2:** `compute_latency_result(method: &str, samples: &[Duration]) ->
  LatencyResult` is a public function in `src/result.rs`. It returns correct
  `median_ms`, `min_ms`, `max_ms`, `jitter_ms` (sample standard deviation)
  for any non-empty slice. It panics (with a descriptive message) on an empty
  slice — this is a programming error, not a user error; a latency probe that
  returns zero samples is a bug upstream.

- [ ] **AC-3:** `MetricsAccumulator` is `Clone + Send + Sync`. Creating an
  instance and cloning it returns two handles backed by the same shared state:
  `record_bytes` on one clone is immediately visible to `finish()` on another.

- [ ] **AC-4:** `MetricsAccumulator::new(interval: Duration, warmup: Duration) ->
  Self` constructs the accumulator; an initial snapshot (all-zero, phase
  `Phase::Latency`) is immediately available to any subscriber.

- [ ] **AC-5:** `MetricsAccumulator::subscribe(&self) -> watch::Receiver<Snapshot>`
  returns a receiver that sees every snapshot emitted after the call.

- [ ] **AC-6:** `MetricsAccumulator::start_ticking(&self) ->
  tokio::task::JoinHandle<()>` spawns a background task that calls
  `tx.send(self.compute_snapshot())` on every `interval` tick. After the
  handle is aborted, no more snapshots are emitted.

- [ ] **AC-7:** `record_bytes(n: u64)` is safe to call from multiple concurrent
  tasks. It increments `total_bytes` (cumulative since phase start) and
  `interval_bytes` (reset on each tick). It does NOT directly increment a
  `measurement_bytes` field. The warm-up boundary is captured by the tick
  handler via the baseline-snapshot pattern: on the first tick where
  `elapsed >= warmup`, the tick handler records `bytes_at_warmup_end =
  total_bytes` in state; `finish()` derives post-warmup bytes as
  `total_bytes - bytes_at_warmup_end`. Both counters are protected by the
  same `Mutex<AccumulatorState>`.

- [ ] **AC-8:** `current_mbps` in the emitted snapshot reflects bytes transferred
  in the *last* tick interval only (not cumulative). Formula:
  `(interval_bytes * 8) as f64 / interval_secs / 1e6`.

- [ ] **AC-9:** `bytes_so_far` in the snapshot is the total bytes since phase
  start (regardless of warm-up), so the progress bar in STAGE-003 can show
  cumulative transfer even during warm-up. The warm-up exclusion applies only
  to `ThroughputResult::bytes` and the Mbps statistics.

- [ ] **AC-10:** `finish(&self, connections_configured: usize, connections_active: usize) -> ThroughputResult` computes:
  - `mbps` — mean over the post-warmup per-interval Mbps samples
  - `mbps_p50` — median of the same samples
  - `mbps_p95` — 95th percentile of the same samples
  - `bytes` — `total_bytes - bytes_at_warmup_end` (baseline-snapshot pattern from AC-7)
  - `connections_configured` / `connections_active` — passed as arguments;
    `&self` is correct because clones share state via `Arc` — consuming `self`
    would only drop one clone while others remain live

- [ ] **AC-11:** `set_phase(phase: Phase)` updates the phase visible in subsequent
  snapshots. Bytes counters are NOT reset on `set_phase` — the orchestrator
  (SPEC-012) creates a fresh `MetricsAccumulator` per measurement phase.

- [ ] **AC-12:** `cargo test` passes. `cargo clippy --all-targets -- -D warnings`
  passes. `cargo fmt --check` passes. Tests in `tests/metrics.rs` carry
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` per AGENTS.md
  testing-discipline convention.

- [ ] **AC-13:** New deps (`serde` production + `chrono`) have inline justification
  in the spec body (this section) satisfying the `no-new-top-level-deps-without-decision`
  constraint (severity: warning; inline justification is sufficient per AGENTS.md).

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/metrics.rs` unless noted. File-level allow attrs required.

---

**`tests/metrics.rs`**

- `"snapshot_starts_in_latency_phase"` — creates `MetricsAccumulator::new`
  with any interval and warmup, subscribes, asserts initial snapshot has
  `phase == Phase::Latency` and `bytes_so_far == 0`.

- `"record_bytes_increments_bytes_so_far"` — records 1024 bytes on a fresh
  accumulator, calls `start_ticking`, awaits one tick, asserts snapshot
  `bytes_so_far >= 1024`.

- `"snapshot_emitted_on_interval"` — `#[tokio::test(start_paused = true)]`.
  Creates accumulator with 50ms interval, subscribes, calls `start_ticking`,
  advances time by 200ms (`tokio::time::advance`), drives `rx.changed().await`
  in a loop counting changes, asserts at least 3 snapshots received. Paused
  time eliminates scheduler jitter.

- `"current_mbps_reflects_last_interval_only"` — `#[tokio::test(start_paused =
  true)]`. Records 1_000_000 bytes, advances by one interval, asserts first
  snapshot `current_mbps > 0.0`. Records no more bytes, advances by another
  interval, asserts second snapshot `current_mbps == 0.0`.

- `"warmup_bytes_excluded_from_finish"` — `#[tokio::test(start_paused = true)]`.
  Creates accumulator with `warmup = 200ms`, `interval = 50ms`. Records
  1_000_000 bytes (during warmup). Advances time by 250ms (past warmup boundary,
  drives 5 ticks). Records 500_000 more bytes (post-warmup). Advances by 50ms
  (one more tick to flush `interval_bytes`). Calls `finish(1, 1)`. Asserts
  `result.bytes == 500_000`. Paused time eliminates the race between
  `record_bytes` and the warmup-boundary tick firing. This is the most critical
  test for result accuracy.

- `"finish_computes_mean_and_percentiles"` — `#[tokio::test(start_paused = true)]`.
  Creates accumulator with `warmup = 0ms` (no warm-up window), `interval = 50ms`.
  Records bytes in 5 rounds, each followed by `tokio::time::advance(50ms)` to
  fire a tick: rounds produce known per-interval throughputs
  `[100.0, 200.0, 300.0, 400.0, 500.0]` Mbps (computed from bytes = Mbps × interval_secs × 1e6 / 8).
  Calls `finish(1, 1)`. Asserts within ε = 0.01:
  - `mbps ≈ 300.0` (mean of 5 samples)
  - `mbps_p50 ≈ 300.0` (median, index 2)
  - `mbps_p95 ≈ 500.0` (index 4 of 5)

- `"abort_stops_ticking"` — `#[tokio::test(start_paused = true)]`. Creates
  accumulator, subscribes, calls `start_ticking`. Advances by one interval,
  drives `rx.changed().await` to confirm first snapshot received. Calls
  `handle.abort()`. Advances by 3 more intervals. Asserts
  `tokio::time::timeout(Duration::ZERO, rx.changed()).await` returns `Err(_)`
  (no further change after abort).

- `"multiple_subscribers_receive_same_snapshot"` — subscribes twice before
  `start_ticking`, records bytes, awaits one tick on each receiver, asserts
  both snapshots have equal `bytes_so_far` and `current_mbps`.

- `"set_phase_visible_in_next_snapshot"` — calls `set_phase(Phase::Download)`,
  awaits a tick, asserts snapshot `phase == Phase::Download`.

- `"clone_shares_state"` — creates accumulator, clones it, calls
  `record_bytes(9999)` on the clone, calls `start_ticking` on the original,
  awaits a tick on the original's subscriber, asserts `bytes_so_far >= 9999`.

---

**`src/result.rs`** (unit tests, `#[cfg(test)]` block)

- `"compute_latency_result_basic"` — passes `[100ms, 200ms, 300ms]`, asserts
  `median_ms ≈ 200.0`, `min_ms ≈ 100.0`, `max_ms ≈ 300.0`.

- `"compute_latency_result_jitter"` — passes a uniform slice, asserts
  `jitter_ms ≈ 0.0`.

- `"test_result_serializes_to_json"` — constructs a `TestResult` with all
  fields filled, calls `serde_json::to_string`, asserts the JSON string
  contains `"download"`, `"upload"`, `"latency"`, `"started_at"`.

- `"snapshot_default_is_all_zero"` — `Snapshot::default()` has `elapsed ==
  Duration::ZERO`, `current_mbps == 0.0`, `bytes_so_far == 0`.

## Implementation Context

### Decisions that apply

- **DEC-006** — specifies the exact field names and types of all result types.
  The build cycle must not rename or reorder public fields — `TestResult`
  Serialize output is the JSON contract. `started_at: DateTime<Utc>` (chrono)
  serializes as ISO 8601 via `chrono::serde::ts_seconds` or the default
  `chrono` serde impl (use the default; ISO 8601 is more readable than epoch).
- **DEC-008** — seam 1: `MetricsAccumulator` must own the `watch::Sender`,
  must not expose it publicly, and must not have any rendering logic. The
  accumulator does not know whether the subscriber is an indicatif bar, a
  future TUI dashboard, or an alerting hook.
- **DEC-005** — context only for SPEC-007; the buffer pool itself is SPEC-009.
  The warm-up window (2 seconds by default) is the shared contract with DEC-006.
  `MetricsAccumulator::new` takes `warmup: Duration` so callers can vary it in
  tests without waiting 2 real seconds.

### Constraints that apply

- **`test-before-implementation`** — the failing tests above are written first
  (in the design spec body); the build cycle makes them pass.
- **`no-new-top-level-deps-without-decision`** — two new prod-level deps land
  in this spec:
  - **`serde` (with `derive` feature)** promoted from `[dev-dependencies]` to
    `[dependencies]`. Justification: `TestResult` and sub-structs need
    `Serialize` in production code (the JSON renderer in STAGE-003 calls
    `serde_json::to_writer` on a live `TestResult`). No DEC required; this is
    the obvious serialization choice for the ecosystem and carries no
    architectural consequence.
  - **`chrono = { version = "0.4", features = ["serde"] }`** new to
    `[dependencies]`. Justification: DEC-006 specifies `started_at:
    DateTime<Utc>` by name. `std::time::SystemTime` serializes as epoch
    seconds by default (needs a custom serializer for ISO 8601), making
    the JSON output less human-readable. `chrono` is the de-facto standard
    for `DateTime<Utc>` in the Rust ecosystem; its `serde` feature gives
    us ISO 8601 JSON output with zero ceremony. It does not touch any hot
    path; the `TestResult` is constructed once per test run. No DEC required.

### Prior related work

- **SPEC-005** (shipped) — defines `BackendError` (`Network`, `Protocol`,
  `NotImplemented` variants). `MetricsAccumulator` does not depend on
  `BackendError`; that relationship arrives in SPEC-012 (orchestrator).
- **SPEC-006** (shipped) — axum mock server. SPEC-007's integration tests
  do not need the mock server (they test the accumulator in isolation using
  simulated byte recording and tokio timing). The mock server becomes relevant
  in SPEC-008+.

### Out of scope (for this spec specifically)

- `TestError` — the structured orchestrator-level error enum. Lands in
  SPEC-012 (orchestrator). `MetricsAccumulator` does not return `TestError`.
- The buffer pool (`BytesMut`/`ArrayQueue`) — SPEC-009.
- Any actual HTTP calls — SPEC-008 (latency probe) onward.
- Human/silent renderers consuming `Snapshot` — STAGE-003.
- `connections_configured` / `connections_active` values are passed into
  `finish()` as arguments; the accumulator does not track connection state.
  Connection bookkeeping is SPEC-010/011 (real backends) or SPEC-012.

### Design notes for the implementer

**MetricsAccumulator internals**

The shared state lives behind `Arc<Mutex<AccumulatorState>>`. Lock contention
is not a concern here — each `record_bytes` call is a tiny critical section
(two `u64` increments). If STAGE-004 profiling shows this is a bottleneck,
switch to `AtomicU64` for the counters; that's a contained refactor.

`watch::Sender<Snapshot>` must be wrapped in `Arc` to make
`MetricsAccumulator` derive `Clone`:

```rust
struct AccumulatorInner {
    state: Mutex<AccumulatorState>,
    tx: watch::Sender<Snapshot>,
}

#[derive(Clone)]
pub struct MetricsAccumulator {
    inner: Arc<AccumulatorInner>,
    interval: Duration,
    warmup: Duration,
}
```

The background task spawned by `start_ticking` holds an `Arc::clone` of
`self.inner` and loops on `tokio::time::interval`. On each tick it:
1. Locks `state`
2. Computes `current_mbps` from `state.interval_bytes` and `self.interval`
3. Resets `state.interval_bytes = 0`
4. Checks warmup: if `state.bytes_at_warmup_end.is_none()` and
   `state.started_at.elapsed() >= warmup`: sets `state.bytes_at_warmup_end =
   Some(state.total_bytes)` (baseline snapshot — from this point on,
   `finish()` derives post-warmup bytes as `total_bytes - bytes_at_warmup_end`)
5. If warmup has ended: pushes `current_mbps` to `state.samples`
6. Sends the snapshot; ignores send errors (no subscribers is fine)

`record_bytes` only increments `total_bytes` and `interval_bytes` — it never
touches warmup bookkeeping. This eliminates the race in
`warmup_bytes_excluded_from_finish`: no matter when `record_bytes` is called
relative to the tick, the warmup boundary is determined by elapsed time, not
by `record_bytes` call order.

`AccumulatorState` fields:
```rust
struct AccumulatorState {
    phase: Phase,
    started_at: std::time::Instant,
    interval_bytes: u64,
    total_bytes: u64,              // cumulative since phase start (bytes_so_far)
    bytes_at_warmup_end: Option<u64>, // baseline; None until warmup boundary fires
    samples: Vec<f64>,             // per-interval Mbps, post-warmup only
}
```

`elapsed` is derived from `started_at.elapsed()` inside the lock (no extra
field needed).

**p50/p95 computation in `finish()`**

Sort `samples` in-place and index:
- p50: `samples[n / 2]`
- p95: `samples[(n * 95) / 100]`

For a single sample, both percentiles equal that sample. For an empty samples
slice (warm-up longer than test duration), return `0.0` for all stats —
document this as a degenerate case in a code comment.

**`compute_latency_result` helper**

```rust
pub fn compute_latency_result(method: &str, samples: &[Duration]) -> LatencyResult {
    assert!(!samples.is_empty(), "latency probe returned zero samples — bug upstream");
    // median: sort a copy and take middle element
    // jitter: sample standard deviation (sqrt of mean of squared deviations from mean)
    ...
}
```

Jitter formula (sample stddev):
```
mean = sum(x) / n
variance = sum((x - mean)^2) / (n - 1)   // Bessel-corrected for n > 1
jitter = sqrt(variance)
```
For `n == 1`, stddev is 0 by convention.

**`TestResult` construction**

`TestResult` is assembled by the orchestrator (SPEC-012), not the accumulator.
This spec only *defines* the struct and ensures it round-trips through serde.

**`Snapshot::default()`**

`Phase` cannot derive `Default` naturally (no obvious default variant). Implement
`Default for Snapshot` manually with `phase: Phase::Latency`. Also implement
`Default for Phase` as `Phase::Latency` to make this consistent.

**`started_at` in `TestResult`**

Use `chrono::Utc::now()` at the point the test is started (in SPEC-012).
This spec defines the *type* (`DateTime<Utc>`) but SPEC-007 does not produce
a `TestResult` — it only defines the struct.

**`#[non_exhaustive]`**

Applies to: `TestResult`, `ThroughputResult`, `LatencyResult`, `Snapshot`,
`Phase` (the enum), `MetricsAccumulator` (struct — prevents external
construction, consistent with STAGE-001 pattern). This is the v0.1 public-API
convention established in SPEC-005.

---

## Build Completion

*Filled in at the end of the **build** cycle, before advancing to verify.*

- **Branch:** `feat/spec-007-metrics-accumulator`
- **PR (if applicable):** [#9](https://github.com/jysf/rspeed/pull/9)
- **All acceptance criteria met?** Yes — see below:
  - **AC-1:** ✅ `src/result.rs` defines all five public types with DEC-006 fields, `Serialize + Deserialize + Debug` on result types, `#[non_exhaustive]` on all five.
  - **AC-2:** ✅ `compute_latency_result` is public; computes median/min/max/jitter (Bessel-corrected); panics with descriptive message on empty slice.
  - **AC-3:** ✅ `MetricsAccumulator` is `Clone + Send + Sync` (verified by `clone_shares_state` test and the `Arc<AccumulatorInner>` wrapping).
  - **AC-4:** ✅ `new(interval, warmup)` returns initial all-zero `Phase::Latency` snapshot via the `watch::channel(Snapshot::default())` seed (test: `snapshot_starts_in_latency_phase`).
  - **AC-5:** ✅ `subscribe()` delegates to `watch::Sender::subscribe` (test: `multiple_subscribers_receive_same_snapshot`).
  - **AC-6:** ✅ `start_ticking` spawns a tokio task; `abort()` halts emissions (test: `abort_stops_ticking`).
  - **AC-7:** ✅ `record_bytes` is `&self` over `Arc<Mutex<…>>`, increments both counters, never touches warm-up bookkeeping. Baseline-snapshot pattern in tick handler (test: `warmup_bytes_excluded_from_finish`).
  - **AC-8:** ✅ `current_mbps = (interval_bytes * 8) / interval_secs / 1e6`; `interval_bytes` reset every tick (test: `current_mbps_reflects_last_interval_only`).
  - **AC-9:** ✅ `bytes_so_far = total_bytes` (cumulative, regardless of warm-up).
  - **AC-10:** ✅ `finish(&self, configured, active)` computes mean/p50/p95 over post-warm-up samples; `bytes = total - bytes_at_warmup_end` (test: `finish_computes_mean_and_percentiles`).
  - **AC-11:** ✅ `set_phase` updates phase only; bytes counters untouched (test: `set_phase_visible_in_next_snapshot`).
  - **AC-12:** ✅ `cargo test` (34 tests pass), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` all clean.
  - **AC-13:** ✅ `serde` (promoted) and `chrono` justified inline in spec (lines 300–314); `tokio` `test-util` feature added as dev-only is mechanical and does not enlarge the prod surface.
- **New decisions emitted:** None. All choices flow from DEC-005/006/008.
- **Deviations from spec:**
  - `Phase` derives `Default` via `#[derive(Default)] + #[default]` rather than a manual `impl Default` — clippy `derivable_impls` flagged the manual version. Same observable behavior (`Phase::default() == Phase::Latency`). The build prompt's literal "Implement `Default for Phase`" is satisfied.
  - Added `tokio = { version = "1", features = ["test-util"] }` to `[dev-dependencies]` to enable `tokio::time::advance` and `start_paused = true`. Dev-only — does not affect the prod dep surface and is in the spirit of `serde_json` being a test-only dep (per AGENTS.md "Style"). Not pre-listed in the spec because the design session didn't anticipate the feature gate.
- **Follow-up work identified:**
  - SPEC-008 onward: feed real bytes through `record_bytes` from the latency probe / download / upload streams.
  - STAGE-004 perf: if `Mutex<AccumulatorState>` shows up in profiling, swap counters for `AtomicU64` (contained refactor; spec design notes call this out).
  - DEC tracking: when the orchestrator (SPEC-012) lands, `connections_active` plumbing will need its own seam — out of scope here per AC-11.

### Build-phase reflection

1. **What was unclear in the spec that slowed you down?**
   The interaction between `MissedTickBehavior::Burst` (tokio default) and `tokio::time::advance` was not addressed. With the default Burst behavior, a single `advance(period)` can deliver multiple back-to-back ticks that the watch channel collapses to one observable change, which would silently break `current_mbps_reflects_last_interval_only` (the receiver would see `current_mbps == 0` instead of the high-throughput first sample). Switching to `MissedTickBehavior::Delay` made each `advance(period)` deliver exactly one tick, which is the implicit contract every paused-time test relies on. This was a build-time discovery, not a spec error — but a one-line note in the design's "MetricsAccumulator internals" section pointing at `MissedTickBehavior::Delay` would have saved 15 minutes of reasoning.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   The `tokio` `test-util` feature gate is not surfaced in DEC-001 (Tokio feature set). It is dev-only, so it doesn't actually expand the prod surface DEC-001 cares about, but the build prompt could have called it out preemptively (same way `serde_json` is noted as test-only in AGENTS.md §15.Style). Treating this as a dev-dep convention rather than a constraint violation; not emitting a DEC.

3. **If you did this task again, what would you do differently?**
   Verify `MissedTickBehavior` and `start_paused` interaction *before* writing all 10 tests — I wrote the test file expecting the default Burst behavior and had to reason through whether each test would still pass once I switched to Delay (they all do, but it required a second pass). Cheap to run a single 5-line probe test first.

---

## Reflection (Ship)

*Appended during the **ship** cycle.*

1. **What would I do differently next time?**
   —

2. **Does any template, constraint, or decision need updating?**
   —

3. **Is there a follow-up spec I should write now before I forget?**
   —
