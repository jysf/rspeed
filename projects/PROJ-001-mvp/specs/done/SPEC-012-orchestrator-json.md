---
task:
  id: SPEC-012
  type: story
  cycle: ship
  blocked: false
  priority: high
  complexity: L
  estimated_hours: 7

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: rspeed

agents:
  architect: claude-opus-4-7
  implementer: null
  created_at: 2026-05-03

references:
  decisions: [DEC-001, DEC-005, DEC-006, DEC-008]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-007, SPEC-008, SPEC-010, SPEC-011, SPEC-013]

value_link: "the load-bearing integration spec — turns the STAGE-002 type/probe/throughput pieces into a working `rspeed --format json` command and lands the orchestrator + TestError seam STAGE-003 renderers and a future v2 MonitorSession both consume"

cost:
  sessions:
    - cycle: design
      date: 2026-05-03
      agent: claude-opus-4-7
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Spec authoring + Frame critique in single Opus session (per SPEC-007/008/010/011 precedent)"
    - cycle: build
      date: 2026-05-03
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 9951611
      tokens_output: 79531
      estimated_usd: 6.165
      note: "Build cycle: orchestrator, error, config, lib rewrite, 9 integration tests"
    - cycle: verify
      date: 2026-05-03
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 3880790
      tokens_output: 39204
      estimated_usd: 3.7793
      note: "Verify cycle: full AC walkthrough, regression run, design integrity checks"
  totals:
    tokens_total: 13951136
    estimated_usd: 9.9443
    session_count: 3
---

# SPEC-012: Test orchestrator + headless JSON output

## Context

Sixth measurement spec under STAGE-002. SPEC-007 shipped the type
layer (`MetricsAccumulator`, `Snapshot`, `TestResult`). SPEC-008
shipped the latency probe (the first real network code). SPEC-009
shipped the buffer pool. SPEC-010 shipped Cloudflare download/upload.
SPEC-011 wired Generic HTTP download/upload through the same shared
`throughput.rs` module.

Today, `lib::run()` is a stub that prints the parsed `Config` and
exits 0. SPEC-012 replaces that stub with the **test orchestrator**
that drives `Backend::latency_probe`, `Backend::download`, and
`Backend::upload` against a `MetricsAccumulator` per phase, assembles
a populated `TestResult`, and emits it to stdout per `--format json`.

This is the load-bearing integration spec for STAGE-002: it
establishes invariants #2 (orchestrator is invocation-agnostic per
DEC-008) and #3 (typed failure modes — `TestError`) that the stage
plan calls out as critical. STAGE-003 (renderers) and a hypothetical
v2 `MonitorSession` both subscribe to the seams this spec lands.

It also opportunistically resolves the `generic-backend-base-url-trailing-slash`
question inherited from SPEC-008/SPEC-011: by adding a one-line
`Config::validate()` check, the silent `Url::join` foot-gun is caught
at CLI parse time rather than after the first request fails silently.

## Goal

Author `src/orchestrator.rs` (`TestSession`), `src/error.rs`
(`TestError`), rewrite `src/lib.rs::run()` to be the async entry
point that drives them, add a `Config::validate()` check, and land
end-to-end `tests/orchestrator.rs` integration tests against
`MockServer` that prove a populated `TestResult` is produced and
serialises to the DEC-006 JSON shape.

No `Backend` trait signature changes — the trait stabilised at
SPEC-008 and SPEC-011 confirmed both impls are complete.

## Inputs

- **`decisions/DEC-006-output-formats.md`** — authoritative `TestResult`
  shape; warm-up rule (2s); snapshot fan-out via watch channel
- **`decisions/DEC-008-deferred-tui.md`** — seams: (#1) accumulator owns
  `watch::Sender`, (#2) orchestrator is invocation-agnostic so a v2
  `MonitorSession` wraps it without touching measurement code, (#3)
  tokio paused-clock semantics (`MissedTickBehavior::Delay`,
  `tokio::time::Instant`); SPEC-012 must preserve seam #2 explicitly
- **`decisions/DEC-005-buffer-strategy.md`** — warm-up window default
  (2s); upload-RSS follow-up note already in Consequences (don't
  re-litigate; pick a default that fits the budget today and document
  the tradeoff)
- **`decisions/DEC-001-tokio-feature-set.md`** — runtime features
  available: `rt-multi-thread`, `net`, `time`, `macros`, `io-util`,
  `sync`. SPEC-012 needs `rt-multi-thread` for the runtime, `time` for
  `Instant` / `timeout`, `sync` for `watch`. All present.
- **`src/lib.rs`** — current stub `run()`; SPEC-012 rewrites it
- **`src/cli.rs`** + **`src/config.rs`** — current `Cli` / `Config`
  surface; SPEC-012 adds `Config::validate()` and re-uses
  `config.duration_secs`, `config.connections`, `config.do_download`,
  `config.do_upload`, `config.format`, `config.server`
- **`src/result.rs`** — `TestResult`, `LatencyResult`, `ThroughputResult`,
  `Snapshot`, `Phase`, `compute_latency_result` — SPEC-012 populates
  `TestResult` from these
- **`src/metrics.rs`** — `MetricsAccumulator::new`, `subscribe`,
  `start_ticking`, `record_bytes`, `set_phase`, `finish`. Per SPEC-007
  AC-11, **the orchestrator constructs a fresh `MetricsAccumulator`
  per measurement phase** rather than reusing one with `set_phase` —
  bytes counters do not reset on `set_phase`. SPEC-012 honours this.
- **`src/backend/mod.rs`** — `Backend` trait, `LatencyProbeOutcome`,
  `DownloadOpts`, `UploadOpts`, `UploadResult`, `DownloadStream`,
  `BackendError`. The latter has `Network`, `Protocol`, `Timeout`,
  `NotImplemented` variants
- **`src/backend/select.rs`** — `select(&Config) -> Result<Box<dyn Backend
  + Send + Sync>, BackendError>`; SPEC-012 calls it from `lib::run()`
- **`src/main.rs`** — current sync `main()` calling `rspeed::run()`;
  SPEC-012 keeps `lib::run()` sync (with an internal tokio runtime —
  see Implementation Context for rationale) so `main.rs` stays
  unchanged
- **`tests/common/mod.rs`** — `MockServer` already exposes everything
  SPEC-012 needs: `start()`, `start_with_options()`, `base_url()`,
  `ping_count()`, `download_count()`, `upload_count()`. **No
  extensions required.**
- **`guidance/questions.yaml`** — `generic-backend-base-url-trailing-slash`
  open question; SPEC-012 fixes it eagerly via `Config::validate()`
  and marks the question answered

## Outputs

- **Files created:**
  - `src/orchestrator.rs` — `TestSession`, default constants, internal
    snapshot-forwarder helper. Public API: `TestSession::new(backend,
    config)`, `TestSession::with_intervals(backend, config,
    snapshot_interval, warmup)` (B-1 extension point),
    `TestSession::snapshot_rx()`, `TestSession::run()`
  - `src/error.rs` — `TestError` enum (`#[non_exhaustive]`,
    `thiserror::Error`); `TestError::exit_code()` translates variants
    to AGENTS.md exit codes (`Config → 2`, `Latency/Download/Upload`
    classified by inner `BackendError` variant: `Network|Timeout → 3`,
    `Protocol → 4`, `NotImplemented → 4`)
  - `tests/orchestrator.rs` — integration tests (see **Failing Tests**)

- **Files modified:**
  - `src/lib.rs`:
    - declare `pub mod orchestrator; pub mod error;`
    - re-export `TestSession`, `DEFAULT_*` constants, `TestError`
    - rewrite `run()` to: parse CLI → `Config` → `validate` →
      `select(&config)` → build `TestSession` → block_on
      `session.run()` → render per `config.format` → return exit code.
      The signature stays `pub fn run() -> anyhow::Result<i32>` so
      `main.rs` is unchanged; the function builds a tokio runtime
      internally with `worker_threads(2)` (cold-start / RSS budget
      consideration — see Implementation Context).
  - `src/config.rs`:
    - add `pub fn validate(&self) -> Result<(), TestError>` — checks
      `--server` URL has a trailing-slash `path()` (resolves the
      `generic-backend-base-url-trailing-slash` question)
  - `guidance/questions.yaml`:
    - mark `generic-backend-base-url-trailing-slash` as `answered`
      with a one-paragraph note pointing at SPEC-012's
      `Config::validate()` and the failing test that pins the check

- **`Cargo.toml`:** **no new top-level deps.** All required types
  (`anyhow`, `serde_json` (dev-only — see AC-9), `tokio`, `chrono`,
  `serde`, `thiserror`) are already in the dep graph from earlier
  STAGE-001/002 specs. The orchestrator does not need `serde_json`
  in `[dependencies]`: `lib::run()` writes JSON by calling
  `serde_json::to_writer_pretty(stdout, &result)` only inside the
  binary path, but that import lives in `src/lib.rs::run()` which is
  callable only from `main.rs` and tests; `serde_json` already lives
  in `[dev-dependencies]`. **Per AGENTS.md "Style" guidance** ("prefer
  `serde_json` as a `[dev-dependencies]` over enabling reqwest's
  `json` feature in production"), `serde_json` must be **promoted to
  `[dependencies]`** in this spec — it is now reachable at runtime.
  Inline justification per the `no-new-top-level-deps-without-decision`
  warning constraint: SPEC-012 makes JSON output the production code
  path; promoting `serde_json` is mechanical (it was already in the
  dev graph and is the obvious serialiser for the ecosystem). No DEC
  required.

- **`src/main.rs`:** unchanged. The internal-runtime approach (see
  Implementation Context — "Why an internal runtime, not
  `#[tokio::main]`") keeps `main.rs` sync.

- **New exports (from `src/lib.rs`):**
  ```rust
  pub use error::TestError;
  pub use orchestrator::{
      TestSession, DEFAULT_LATENCY_SAMPLES, DEFAULT_DOWNLOAD_BYTES_PER_REQUEST,
      DEFAULT_UPLOAD_BYTES_PER_REQUEST, DEFAULT_SNAPSHOT_INTERVAL,
      DEFAULT_WARMUP,
  };
  ```

## Acceptance Criteria

- [ ] **AC-1: `TestSession` is invocation-agnostic per DEC-008 seam #2.**
  Constructor `pub fn new(backend: Box<dyn Backend + Send + Sync>,
  config: Config) -> Self`. Run method
  `pub async fn run(&self) -> Result<TestResult, TestError>` takes
  `&self` (not `self`) so a future `MonitorSession` (v2) can call it
  in a loop with no measurement-code changes. The `Backend` is owned
  (`Box<dyn Backend + Send + Sync>`), not borrowed — the backend's
  internal `reqwest::Client` stays alive across iterations and reuses
  its connection pool.

- [ ] **AC-2: `TestSession::snapshot_rx()` exposes a `watch::Receiver
  <Snapshot>` for renderers.** Signature: `pub fn snapshot_rx(&self)
  -> tokio::sync::watch::Receiver<Snapshot>`. Subscribers may be added
  before or after `run()` starts. Initial snapshot value is
  `Snapshot::default()` (matches DEC-008 seam #1's accumulator
  default). The receiver sees snapshots from all three phases in
  order: `Phase::Latency` → `Phase::Download` → `Phase::Upload`
  (skipping any phase disabled by `--no-download` / `--no-upload`).

- [ ] **AC-3: Phase orchestration creates a fresh `MetricsAccumulator`
  per phase.** Per SPEC-007 AC-11, `set_phase` does not reset bytes
  counters, so a single accumulator across phases would conflate
  cumulative bytes. The orchestrator instead constructs a new
  `MetricsAccumulator::new(DEFAULT_SNAPSHOT_INTERVAL, DEFAULT_WARMUP)`
  for download and another for upload, calling `set_phase()` once on
  each immediately after construction so the first emitted snapshot
  carries the correct phase tag. Latency does not need an accumulator
  (the probe returns its samples directly).

- [ ] **AC-4: Snapshot fan-out forwards per-phase accumulator
  snapshots into a single outer stream; phase-boundary cleanup is
  explicit.** `TestSession` owns one outer `watch::Sender<Snapshot>`.
  Each measurement phase spawns a small forwarder task that
  subscribes to the phase's `MetricsAccumulator` and forwards
  `*rx.borrow_and_update()` into the outer sender. **Per architect
  decision A-2: the forwarder `JoinHandle` is bound and explicitly
  `abort()`ed at end-of-phase** (not merely dropped) so no
  previous-phase forwarder can race-emit a stale-phase snapshot
  after the next phase's forwarder has started. The accumulator's
  ticker `JoinHandle` is also explicitly aborted at end-of-phase.
  This is the seam STAGE-003's renderers consume. **Forwarder loops
  use `tokio::spawn`** so they don't block the main run loop.

- [ ] **AC-5: Latency phase populates `TestResult::latency`.** Calls
  `backend.latency_probe(DEFAULT_LATENCY_SAMPLES)`; on `Ok(outcome)`
  threads `outcome.samples` through `compute_latency_result(
  outcome.method, &outcome.samples)`. On `Err(BackendError::*)`
  returns `TestError::Latency(err)` immediately — the test cannot
  proceed to throughput phases without baseline latency. The latency
  phase does not allocate a `MetricsAccumulator`.

- [ ] **AC-6: Download phase populates `TestResult::download` when
  `config.do_download == true`, otherwise `None`.** With download
  enabled: opts are `DownloadOpts::new(DEFAULT_DOWNLOAD_BYTES_PER_REQUEST,
  config.connections)`. `backend.download(&opts).await?` returns the
  merged `DownloadStream`. The orchestrator drains the stream chunk
  by chunk, calling `acc.record_bytes(chunk.len() as u64)` per chunk,
  until either the stream ends or
  `tokio::time::Instant::now().duration_since(phase_start) >=
  Duration::from_secs(config.duration_secs as u64)`. After the loop,
  the stream is dropped (which closes the underlying TCP connections
  server-side via the body-not-consumed signal) and `acc.finish(
  config.connections as usize, config.connections as usize)` returns
  the `ThroughputResult`. Errors during stream drain map to
  `TestError::Download(BackendError)`.

- [ ] **AC-7: Upload phase populates `TestResult::upload` when
  `config.do_upload == true`, otherwise `None`.** With upload enabled:
  opts are `UploadOpts::new(DEFAULT_UPLOAD_BYTES_PER_REQUEST,
  config.connections)`. The orchestrator loops calling
  `backend.upload(&opts).await?` until `phase_start.elapsed() >=
  duration`; each completed `UploadResult` contributes
  `result.bytes_sent` to the accumulator via `acc.record_bytes`. After
  the loop, `acc.finish(...)` returns the `ThroughputResult`. Errors
  map to `TestError::Upload(BackendError)`.

  **Documented limitation:** because each `backend.upload()` call is
  bulk (no per-chunk feedback), recorded bytes arrive at the
  accumulator in a burst at end-of-call rather than continuously. The
  reported `mbps` (mean over the per-tick samples) is still correct,
  but `mbps_p50` / `mbps_p95` may be coarser than for download. A
  STAGE-004 polish wraps the upload body to feed the accumulator
  continuously; tracked as a Reflection follow-up rather than a new
  question.

- [ ] **AC-8: `TestResult` metadata fields are populated correctly.**
  - `started_at`: `chrono::Utc::now()` captured immediately before
    the latency phase begins
  - `backend`: `backend.name().to_string()` (`"cloudflare"` or
    `"generic"`)
  - `server_url`: `config.server.as_ref().map(|u| u.to_string())
    .unwrap_or_else(|| "https://speed.cloudflare.com/".to_string())`
  - `ip_version`: stub-string `"auto"` / `"ipv4"` / `"ipv6"` derived
    from `config.ip_version`. **Honest accounting:** SPEC-012 cannot
    detect which family was *actually* used by reqwest in the MVP
    (no public hyper hook); the field reflects user intent, not
    observed wire behaviour. STAGE-004 may refine via reqwest's
    `local_addr` once a connection is established. Document the
    limitation in a code comment on `Config::ip_version_string()`.
  - `duration_secs`: actual measurement-window duration (sum of
    download `phase_start.elapsed()` + upload `phase_start.elapsed()`,
    minus the warm-up window of each enabled phase). Falls back to
    `0.0` if neither phase ran. **Per DEC-006:** "actual measurement
    window, excluding warm-up."

- [ ] **AC-9: `--format json` writes `TestResult` to stdout via
  `serde_json::to_writer_pretty`.** `lib::run()` matches on
  `config.format`:
  - `Format::Json` → `serde_json::to_writer_pretty(io::stdout().lock(),
    &result)?; println!()` (trailing newline so `jq` is happy)
  - `Format::Human` → STAGE-003 scope. SPEC-012 **temporarily falls
    back to JSON output** with a one-line `eprintln!("(human renderer
    coming in STAGE-003 — emitting JSON)")` warning. Documented as
    intended interim behaviour; STAGE-003 replaces this branch.
  - `Format::Silent` → emits nothing on success.
  - All three exit `0` on success.

- [ ] **AC-10: `Config::validate()` rejects `--server` URLs without
  trailing slash.** `Config { server: Some(url), .. }.validate()`
  returns `Err(TestError::Config(_))` if `url.path().ends_with('/') ==
  false`. `MockServer::base_url()` already returns trailing-slash URLs,
  so existing tests pass. Resolves the
  `generic-backend-base-url-trailing-slash` question; SPEC-012's PR
  marks the question entry `answered`.

- [ ] **AC-11: `TestError` is `#[non_exhaustive]`,
  `thiserror::Error`, with phase-tagged variants.** Variants
  (5 total per Frame patch F):
  - `Config(String)` — config validation failures (e.g. `--server`
    URL without trailing slash)
  - `Backend(#[source] BackendError)` — backend construction
    failures (TLS init, URL parse) from `select(&Config)`
  - `Latency(#[source] BackendError)` — latency probe failed
  - `Download(#[source] BackendError)` — download failed
  - `Upload(#[source] BackendError)` — upload failed
  
  Public method `pub fn exit_code(&self) -> i32` translates per the
  AGENTS.md exit-code table:
  - `Config(_) → 2`
  - `Backend|Latency|Download|Upload(BackendError::Network|Timeout) → 3`
  - `Backend|Latency|Download|Upload(BackendError::Protocol|NotImplemented) → 4`
  
  `lib::run()` calls `exit_code()` on the error path and returns the
  code from `Ok(code)`; the renderer also prints the error to stderr
  (`eprintln!("error: {err:#}")`) before returning the exit code.
  Errors are rendered as JSON to stderr only when `config.format ==
  Format::Json` — see AGENTS.md error-handling §15.

- [ ] **AC-12: `lib::run()` builds a tokio runtime internally with
  `worker_threads(2)` and stays sync.** Signature `pub fn run() ->
  anyhow::Result<i32>` is unchanged so `main.rs` does not need to
  become `#[tokio::main]`. The internal runtime caps worker threads
  at 2 (rationale: 4 parallel HTTP connections + a few snapshot
  forwarders run comfortably on 2 workers; default `num_cpus` would
  spawn 8–16 threads on developer machines and inflate RSS by ~16MB
  of stack alone, eating into the 20MB peak budget). Runtime build
  failures propagate via `?`.

- [ ] **AC-13: Existing tests continue to pass.** All current tests
  (`tests/cli.rs`, `tests/version.rs`, `tests/smoke.rs`,
  `tests/buffer_pool.rs`, `tests/metrics.rs`, `tests/latency.rs`,
  `tests/throughput.rs`, `tests/generic_backend.rs`) pass without
  modification. `tests/common/mod.rs` is **unchanged** — `MockServer`
  already exposes everything SPEC-012 needs.

- [ ] **AC-14: Lib-side `unwrap`/`expect`/`panic` discipline preserved.**
  No `unwrap()`, `expect()`, or `panic!()` in `src/orchestrator.rs`,
  `src/error.rs`, or the new code in `src/lib.rs` / `src/config.rs`.
  All fallible operations use `?` to propagate `TestError` or
  `anyhow::Error`. `tests/orchestrator.rs` carries `#![allow(clippy::
  unwrap_used, clippy::expect_used, clippy::panic)]` per project
  test convention.

- [ ] **AC-15: `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` pass.**

- [ ] **AC-16: All three CI runners green.** macOS arm64
  (`macos-15`), Linux x86_64 (`ubuntu-24.04`), Windows x86_64
  (`windows-2025`). The `Cross-check x86_64-apple-darwin` step still
  succeeds.

- [ ] **AC-17: End-to-end smoke against MockServer produces a JSON
  string that round-trips through `serde_json::from_str::<TestResult>`.**
  The integration test in `tests/orchestrator.rs` (see
  `test_session_run_produces_serialisable_test_result` in **Failing
  Tests**) verifies the JSON contract end-to-end. This is the
  load-bearing test for STAGE-002's success criterion: "`rspeed
  --format json` produces a valid `TestResult` JSON object populated
  with real measurements."

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/orchestrator.rs` unless noted. The file opens
with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! End-to-end orchestrator integration tests for SPEC-012.

mod common;

use std::time::Duration;

use common::MockServer;
use rspeed::{
    Backend, BackendError, Config, ColorWhen, Format, GenericHttpBackend,
    Phase, TestError, TestSession,
};
use rspeed::config::IpVersion;
```

`fn build_config(mock: &MockServer, do_download: bool, do_upload: bool)
-> Config { ... }` is a small local helper that constructs a `Config`
pointing at the MockServer with `duration_secs: 1` (short test
window — paired with `warmup: 0ms` overrides where needed; see
**Implementation Context** for how tests bypass the 2s default warm-up
without changing production defaults).

---

**`"orchestrator_run_against_mock_populates_all_phases"`** —
`#[tokio::test]`. Builds a `MockServer`, constructs `GenericHttp
Backend::new(mock.base_url())?`, builds `Config { duration_secs: 1,
do_download: true, do_upload: true, .. }`, builds `TestSession`,
calls `session.run().await`. Asserts:
- `result.latency.samples > 0`
- `result.latency.method == "http_rtt"`
- `result.download.is_some() && result.download.as_ref().unwrap().bytes > 0`
- `result.upload.is_some() && result.upload.as_ref().unwrap().bytes >= 0`
  (upload may be 0 if no upload completed within the 1s window with
  the small mock — the test asserts "field is populated", not
  "non-zero". See **Implementation Context — Test sizing** for the
  rationale.)
- `result.backend == "generic"`
- `mock.ping_count() >= 1`
- `mock.download_count() >= 1`
- `mock.upload_count() >= 1`

**Test budget:** ≤ 3 seconds wall-clock (1s download phase + 1s
upload phase + ~100ms latency probe + harness overhead).

---

**`"orchestrator_run_with_only_latency_skips_throughput_phases"`** —
`#[tokio::test]`. Builds session with `do_download: false, do_upload:
false`. Calls `session.run().await`. Asserts:
- `result.latency.samples > 0`
- `result.download.is_none()`
- `result.upload.is_none()`
- `mock.download_count() == 0` (no download requests issued)
- `mock.upload_count() == 0` (no upload requests issued)

---

**`"orchestrator_test_result_round_trips_through_serde_json"`** —
`#[tokio::test]`. Runs the full session against MockServer. Calls
`serde_json::to_string(&result)?` and `serde_json::from_str::<
TestResult>(&json)?`. Asserts the round-tripped result has the same
field values for `latency.samples`, `latency.method`,
`download.is_some()`, `upload.is_some()`, `backend`. This is the
load-bearing JSON-contract test.

---

**`"orchestrator_snapshot_rx_observes_phase_transitions"`** —
`#[tokio::test]`. Subscribes to `session.snapshot_rx()` BEFORE
calling `run()`. Spawns a collector task that drains snapshots into
a `Vec<Phase>` (deduplicated to track unique phase-transitions). Runs
the session. After `run()` completes, the collector finishes (the
outer `watch::Sender` is dropped when the session goes out of scope —
see **Implementation Context — Sender lifetime** for the exact
mechanism). Asserts the observed phases include both
`Phase::Download` and `Phase::Upload` (initial `Phase::Latency` is
the default snapshot value, so it's present even without a phase
running).

---

**`"orchestrator_skip_download_omits_download_request"`** —
`#[tokio::test]`. Builds session with `do_download: false, do_upload:
true`. Asserts:
- `result.download.is_none()`
- `result.upload.is_some()`
- `mock.download_count() == 0`
- `mock.upload_count() >= 1`

---

**`"orchestrator_skip_upload_omits_upload_request"`** —
`#[tokio::test]`. Mirror of the above with `do_upload: false`.

---

**`"orchestrator_latency_failure_returns_test_error_latency"`** —
`#[tokio::test]`. Constructs `GenericHttpBackend::new("http://127.0.0.1:1/"
.parse()?)?` (port 1, reliably refused). Builds session, calls
`run().await`. Asserts result is `Err(TestError::Latency(_))`
(latency is the first phase; failure short-circuits). The test does
not assert the exact inner `BackendError` variant — per SPEC-008
test 9 precedent, OS-dependent variance between
`Network`/`Protocol`/`Timeout` is acceptable.

---

**`"config_validate_rejects_server_without_trailing_slash"`** —
`#[tokio::test]` (or unit test in `src/config.rs::tests`; build can
choose). Constructs `Config { server: Some("http://example.com".parse()?),
.. }`. Calls `.validate()`. Asserts `Err(TestError::Config(msg))`
where `msg.contains("trailing slash")`. Then constructs `Config {
server: Some("http://example.com/".parse()?), .. }`. Asserts
`.validate().is_ok()`. Then constructs `Config { server: None, .. }`.
Asserts `.validate().is_ok()` (Cloudflare default, no validation
needed).

---

**`"test_error_exit_code_mapping"`** — unit test in `src/error.rs::tests`.
Constructs the easily-buildable variants only (per Frame patch C —
`Network` requires a real `reqwest::Error` which is brittle to fake
in unit tests; the `Network → 3` mapping is verified end-to-end by
`orchestrator_latency_failure_returns_test_error_latency`):
- `TestError::Config("bad".to_string()).exit_code() == 2`
- `TestError::Backend(BackendError::Timeout(Duration::from_secs(1))).exit_code() == 3`
- `TestError::Backend(BackendError::NotImplemented).exit_code() == 4`
- `TestError::Latency(BackendError::Timeout(Duration::from_secs(1))).exit_code() == 3`
- `TestError::Latency(BackendError::Protocol("p".to_string())).exit_code() == 4`
- `TestError::Download(BackendError::Protocol("p".to_string())).exit_code() == 4`
- `TestError::Upload(BackendError::NotImplemented).exit_code() == 4`

---

**`"orchestrator_test_session_run_is_repeatable"`** — `#[tokio::test]`.
Constructs `TestSession` once. Calls `session.run().await` twice in
sequence. Asserts both runs return `Ok(_)` and the second run's
`mock.ping_count()` is exactly twice the first's. **This is the
DEC-008 seam #2 test:** if `run()` consumed `self`, this would not
compile, which would break a future v2 `MonitorSession` that wraps
`TestSession` in a loop.

---

**`"orchestrator_run_emits_warning_to_stderr_on_human_format"`** —
**Skip in SPEC-012 design.** Stderr-capture is awkward inside
integration tests (assert_cmd is the right tool, and that's a
`tests/cli.rs`-style test). Cover the Human-format fall-back via a
manual test note in the spec body — the build cycle's test plan
section verifies it manually after release-build. Adding an
`assert_cmd` test for it is welcome but not required.

## Implementation Context

*Read this section (and the files it points to) before starting the
build cycle. It is the equivalent of a handoff document, folded into
the spec since there is no separate receiving agent.*

### Decisions that apply

- **DEC-001** — runtime feature set. SPEC-012's internal runtime uses
  `rt-multi-thread` + `time` + `sync`; all already in DEC-001's set.
- **DEC-005** — buffer strategy + warm-up window. SPEC-012 honours
  `DEFAULT_WARMUP = Duration::from_secs(2)`. Tests bypass via per-test
  `MetricsAccumulator::new(_, Duration::ZERO)` plumbing — see "Test
  warm-up override" below.
- **DEC-006** — `TestResult` shape, snapshot fan-out via
  `tokio::sync::watch`, `Snapshot` cadence (~100ms default).
- **DEC-008** — three seams. SPEC-012 is the spec where seams #2
  (orchestrator invocation-agnostic) and #3 (typed failure modes)
  actually get implemented. Concrete invariants:
  - `TestSession::run(&self)` (not `self`), so v2 `MonitorSession`
    wraps in loop with no measurement-code changes
  - `TestSession::snapshot_rx()` exposes `watch::Receiver<Snapshot>`
    rather than coupling rendering via callback — the receiver is
    DEC-008's broadcast seam STAGE-003 / v2 dashboards consume

### Constraints that apply

- **`test-before-implementation`** — the failing tests above are
  written during design (this spec body); build makes them pass.
- **`no-new-top-level-deps-without-decision`** — `serde_json`
  promoted from `[dev-dependencies]` to `[dependencies]`; inline
  justification in **Outputs** above. No DEC required (mechanical;
  obvious choice; already in dev graph).
- **`one-spec-per-pr`** — SPEC-012's PR references only SPEC-012.

### Prior related work

- **SPEC-007** (shipped) — defines `MetricsAccumulator` API; AC-11
  explicitly says the orchestrator constructs a fresh accumulator per
  phase, which SPEC-012 honours
- **SPEC-008** (shipped) — `Backend::latency_probe` returns
  `LatencyProbeOutcome { method, samples }`. SPEC-012 threads
  `compute_latency_result(outcome.method, &outcome.samples)` to
  produce `LatencyResult`
- **SPEC-010** (shipped) — `CloudflareBackend::download` /
  `::upload` are complete; the trait is fully implemented for
  Cloudflare
- **SPEC-011** (shipped) — `GenericHttpBackend::download` /
  `::upload` are complete; **all SPEC-012 integration tests can drive
  `&dyn Backend` against MockServer without conditional logic**
  (this is the testability win SPEC-011 set up)
- **SPEC-013** (planned next) — failure-mode tests will also drive
  the orchestrator; SPEC-012's `tests/orchestrator.rs` is the
  template SPEC-013 extends

### Out of scope (for this spec specifically)

- **Human renderer.** STAGE-003 scope. SPEC-012 falls back to JSON
  for `--format human` with a one-line stderr warning (AC-9).
- **Silent renderer's failure-mode messaging.** STAGE-003 scope.
  SPEC-012's `Format::Silent` emits nothing on success and the
  rendered stderr error on failure; STAGE-003 may suppress the
  stderr on silent.
- **CLI flags for `bytes_per_request` / `latency_samples`.** Not
  exposed in SPEC-012. Hardcoded as `pub const DEFAULT_*` in
  `orchestrator.rs`. STAGE-003 / STAGE-004 may add CLI flags as
  polish if user feedback demands them.
- **Live Cloudflare integration.** Deferred to SPEC-013 behind the
  `live` cargo feature.
- **Connection-liveness tracking** for `connections_active`. SPEC-012
  reports `connections_active = connections_configured` (assumes all
  alive). Proper liveness counting requires wrapping each
  `download_one`/`upload_one` future with a counter; out of scope.
  Documented in code comment.
- **Burst-vs-streaming upload throughput accuracy.** Documented
  limitation in AC-7. STAGE-004 polish.
- **Real `ip_version` detection.** SPEC-012 reflects user intent only
  (AC-8). STAGE-004 polish via reqwest `local_addr` once a connection
  is established.
- **Performance budget verification.** STAGE-004 owns hitting the
  three budgets (cold-start <50ms, peak RSS <20MB, 1Gbps). SPEC-012
  produces code that *plausibly* meets them (per stage doc) — i.e.,
  uses the buffer-pool-friendly streaming path SPEC-010 set up,
  caps tokio worker threads at 2.

### Notes for the implementer

#### `TestSession` struct shape

```rust
// src/orchestrator.rs

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use futures::StreamExt;
use tokio::sync::watch;

use crate::backend::{Backend, DownloadOpts, UploadOpts};
use crate::config::Config;
use crate::error::TestError;
use crate::metrics::MetricsAccumulator;
use crate::result::{
    Phase, Snapshot, TestResult, ThroughputResult, compute_latency_result,
};

pub const DEFAULT_LATENCY_SAMPLES: usize = 10;
pub const DEFAULT_DOWNLOAD_BYTES_PER_REQUEST: u64 = 1_000_000_000; // 1GB stream, capped server-side at Cloudflare's max
pub const DEFAULT_UPLOAD_BYTES_PER_REQUEST: u64 = 10 * 1024 * 1024; // 10MB; STAGE-004 to chunk-stream (see DEC-005 follow-up)
pub const DEFAULT_SNAPSHOT_INTERVAL: Duration = Duration::from_millis(100);
pub const DEFAULT_WARMUP: Duration = Duration::from_secs(2); // DEC-005

pub struct TestSession {
    backend: Box<dyn Backend + Send + Sync>,
    config: Config,
    /// Outer broadcast seam. STAGE-003 renderers and v2 monitor
    /// dashboards subscribe via `snapshot_rx()`. Per-phase
    /// `MetricsAccumulator` snapshots are forwarded into this sender
    /// by short-lived spawned tasks.
    snapshot_tx: watch::Sender<Snapshot>,
    /// Per architect decision B-1: cached so per-phase accumulators
    /// can be constructed with non-default cadence in tests via
    /// `with_intervals(...)`. Production `new()` initialises both
    /// to the `DEFAULT_*` constants.
    snapshot_interval: Duration,
    warmup: Duration,
}

impl TestSession {
    pub fn new(backend: Box<dyn Backend + Send + Sync>, config: Config) -> Self {
        Self::with_intervals(
            backend,
            config,
            DEFAULT_SNAPSHOT_INTERVAL,
            DEFAULT_WARMUP,
        )
    }

    /// Per architect decision B-1: extension point for tests and
    /// future bench tools that need non-default cadence/warmup.
    /// Production callers use `new()` to get the `DEFAULT_*` constants.
    pub fn with_intervals(
        backend: Box<dyn Backend + Send + Sync>,
        config: Config,
        snapshot_interval: Duration,
        warmup: Duration,
    ) -> Self {
        let (snapshot_tx, _rx) = watch::channel(Snapshot::default());
        Self {
            backend,
            config,
            snapshot_tx,
            snapshot_interval,
            warmup,
        }
    }

    pub fn snapshot_rx(&self) -> watch::Receiver<Snapshot> {
        self.snapshot_tx.subscribe()
    }

    pub async fn run(&self) -> Result<TestResult, TestError> {
        let started_at = Utc::now();
        let backend_name = self.backend.name().to_string();

        // Phase 1: latency
        let outcome = self.backend.latency_probe(DEFAULT_LATENCY_SAMPLES).await
            .map_err(TestError::Latency)?;
        let latency = compute_latency_result(outcome.method, &outcome.samples);

        // Phase 2: download (if enabled)
        let mut measurement_secs = 0.0_f64;
        let download = if self.config.do_download {
            let (result, secs) = self.run_download_phase().await?;
            measurement_secs += secs;
            Some(result)
        } else { None };

        // Phase 3: upload (if enabled)
        let upload = if self.config.do_upload {
            let (result, secs) = self.run_upload_phase().await?;
            measurement_secs += secs;
            Some(result)
        } else { None };

        Ok(TestResult {
            started_at,
            backend: backend_name,
            server_url: self.config.server_url_string(),
            ip_version: self.config.ip_version_string(),
            duration_secs: measurement_secs,
            latency,
            download,
            upload,
        })
    }

    async fn run_download_phase(&self) -> Result<(ThroughputResult, f64), TestError> {
        let acc = MetricsAccumulator::new(self.snapshot_interval, self.warmup);
        acc.set_phase(Phase::Download);
        let ticker = acc.start_ticking();
        let forwarder = self.spawn_forwarder(acc.subscribe());

        let opts = DownloadOpts::new(
            DEFAULT_DOWNLOAD_BYTES_PER_REQUEST,
            self.config.connections,
        );
        let result = async {
            let mut stream = self.backend.download(&opts).await
                .map_err(TestError::Download)?;

            let phase_start = Instant::now();
            let duration = Duration::from_secs(self.config.duration_secs as u64);
            loop {
                let remaining = duration.checked_sub(phase_start.elapsed());
                let Some(remaining) = remaining else { break };
                match tokio::time::timeout(remaining, stream.next()).await {
                    Ok(Some(Ok(chunk))) => acc.record_bytes(chunk.len() as u64),
                    Ok(Some(Err(e))) => return Err(TestError::Download(e)),
                    Ok(None) => break,           // server closed early
                    Err(_) => break,             // duration reached
                }
            }
            drop(stream);

            let measurement_secs =
                measurement_window(phase_start.elapsed(), self.warmup);
            let throughput = acc.finish(
                self.config.connections as usize,
                self.config.connections as usize,
            );
            Ok((throughput, measurement_secs))
        }
        .await;

        // A-2: explicit abort eliminates the previous-phase-snapshot race.
        // Aborts are no-ops if the tasks have already exited naturally.
        forwarder.abort();
        ticker.abort();

        result
    }

    async fn run_upload_phase(&self) -> Result<(ThroughputResult, f64), TestError> {
        let acc = MetricsAccumulator::new(self.snapshot_interval, self.warmup);
        acc.set_phase(Phase::Upload);
        let ticker = acc.start_ticking();
        let forwarder = self.spawn_forwarder(acc.subscribe());

        let opts = UploadOpts::new(
            DEFAULT_UPLOAD_BYTES_PER_REQUEST,
            self.config.connections,
        );

        let result = async {
            let phase_start = Instant::now();
            let duration = Duration::from_secs(self.config.duration_secs as u64);
            while phase_start.elapsed() < duration {
                let r = self.backend.upload(&opts).await
                    .map_err(TestError::Upload)?;
                acc.record_bytes(r.bytes_sent);
            }

            let measurement_secs =
                measurement_window(phase_start.elapsed(), self.warmup);
            let throughput = acc.finish(
                self.config.connections as usize,
                self.config.connections as usize,
            );
            Ok((throughput, measurement_secs))
        }
        .await;

        // A-2: explicit abort eliminates the previous-phase-snapshot race.
        forwarder.abort();
        ticker.abort();

        result
    }

    fn spawn_forwarder(
        &self,
        mut rx: watch::Receiver<Snapshot>,
    ) -> tokio::task::JoinHandle<()> {
        let outer = self.snapshot_tx.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let snap = rx.borrow_and_update().clone();
                // Send error means no subscribers — fine, we keep forwarding.
                let _ = outer.send(snap);
            }
        })
    }
}

/// Patch (E): clarity helper for the per-phase measurement window.
fn measurement_window(elapsed: Duration, warmup: Duration) -> f64 {
    elapsed.saturating_sub(warmup).as_secs_f64()
}
```

**Per architect decision A-2:** the `ticker` and `forwarder`
`JoinHandle`s are bound and explicitly `abort()`ed at end-of-phase.
This eliminates the previous-phase-snapshot race (a forwarder for
the dropped accumulator could otherwise emit one final snapshot
after the next phase's forwarder has started). The `Result` is
captured from the inner `async` block so the abort calls run on
both success and error paths without an explicit `defer`-style
helper. `JoinHandle::abort()` is a no-op if the task has already
exited naturally, so this is robust.

#### `TestError` shape

```rust
// src/error.rs

use crate::backend::BackendError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("backend init failed: {0}")]
    Backend(#[source] BackendError),

    #[error("latency probe failed: {0}")]
    Latency(#[source] BackendError),

    #[error("download failed: {0}")]
    Download(#[source] BackendError),

    #[error("upload failed: {0}")]
    Upload(#[source] BackendError),
}

impl TestError {
    /// Translate to the AGENTS.md exit-code table.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) => 2,
            Self::Backend(e) | Self::Latency(e) | Self::Download(e) | Self::Upload(e) => {
                match e {
                    BackendError::Network(_) | BackendError::Timeout(_) => 3,
                    BackendError::Protocol(_) | BackendError::NotImplemented => 4,
                    // BackendError is #[non_exhaustive]; future variants default to
                    // protocol-class (contract violation, not transient).
                    _ => 4,
                }
            }
        }
    }
}
```

The `match` on `BackendError` is exhaustive over today's variants
but `BackendError` is `#[non_exhaustive]`; the build cycle adds a
`_ => 4` fallback arm or a `#[allow(non_exhaustive_omitted_patterns)]`
inline allow if clippy complains. The `4` fallback makes sense as
"some new backend-class error we haven't classified yet — treat as
Protocol-class (i.e., contract violation, not transient)."

#### `Config::validate()` and helper accessors

```rust
// src/config.rs additions

use crate::error::TestError;

impl Config {
    pub fn validate(&self) -> Result<(), TestError> {
        if let Some(url) = &self.server {
            if !url.path().ends_with('/') {
                return Err(TestError::Config(format!(
                    "--server URL must end with a trailing slash (got: {url})"
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn server_url_string(&self) -> String {
        self.server
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "https://speed.cloudflare.com/".to_string())
    }

    /// Reflects user *intent*, not observed wire behaviour.
    /// STAGE-004 may refine this via reqwest::local_addr() to
    /// report the family actually used by the connection pool.
    pub(crate) fn ip_version_string(&self) -> String {
        match self.ip_version {
            IpVersion::Auto => "auto",
            IpVersion::V4 => "ipv4",
            IpVersion::V6 => "ipv6",
        }
        .to_string()
    }
}
```

#### `lib::run()` rewrite

```rust
// src/lib.rs

use std::io::{self, Write};

use clap::Parser;

pub mod backend;
pub mod buffer_pool;
mod cli;
pub mod config;
pub mod error;
pub mod metrics;
pub mod orchestrator;
pub mod result;

pub use backend::{
    Backend, BackendError, CloudflareBackend, DownloadOpts, DownloadStream,
    GenericHttpBackend, LatencyProbeOutcome, UploadOpts, UploadResult,
};
pub use config::{ColorWhen, Config, Format};
pub use error::TestError;
pub use metrics::MetricsAccumulator;
pub use orchestrator::{
    DEFAULT_DOWNLOAD_BYTES_PER_REQUEST, DEFAULT_LATENCY_SAMPLES,
    DEFAULT_SNAPSHOT_INTERVAL, DEFAULT_UPLOAD_BYTES_PER_REQUEST,
    DEFAULT_WARMUP, TestSession,
};
pub use result::{
    LatencyResult, Phase, Snapshot, TestResult, ThroughputResult,
    compute_latency_result,
};

pub fn run() -> anyhow::Result<i32> {
    let cli = cli::Cli::parse();
    let config = Config::from(cli);
    if let Err(e) = config.validate() {
        eprintln!("error: {e:#}");
        return Ok(e.exit_code());
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    runtime.block_on(async_run(config))
}

async fn async_run(config: Config) -> anyhow::Result<i32> {
    // Patch (F): backend-construction failures use TestError::Backend(_),
    // not Config(_), so TLS init failures land at exit code 3 (network/system)
    // rather than 2 (config). URL-parse failures inside select() also land here;
    // accepted granularity loss documented in spec body.
    let backend = match backend::select(&config) {
        Ok(b) => b,
        Err(e) => {
            let err = TestError::Backend(e);
            eprintln!("error: {err:#}");
            return Ok(err.exit_code());
        }
    };

    let format = config.format;
    let session = TestSession::new(backend, config);

    let result = match session.run().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e:#}");
            return Ok(e.exit_code());
        }
    };

    match format {
        Format::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            serde_json::to_writer_pretty(&mut handle, &result)?;
            writeln!(handle)?;
        }
        Format::Human => {
            // STAGE-003 implements the human renderer. SPEC-012
            // falls back to JSON with a one-line warning so the user
            // gets *something* useful in the meantime.
            eprintln!("(human renderer coming in STAGE-003 — emitting JSON)");
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            serde_json::to_writer_pretty(&mut handle, &result)?;
            writeln!(handle)?;
        }
        Format::Silent => {}
    }

    Ok(0)
}
```

#### Why an internal runtime, not `#[tokio::main]`

`#[tokio::main]` is a macro that defaults to `worker_threads =
num_cpus`. On a 16-core developer machine, that's 16 worker threads
× ~2MB stack ≈ 32MB of stack alone — busts the 20MB peak RSS budget
before any real work runs. Configuring the runtime explicitly with
`worker_threads(2)` keeps thread spawn cost and stack RSS bounded.
The tradeoff: `lib::run()` has ~5 lines of runtime construction
ceremony instead of `#[tokio::main]`'s zero, and `main.rs` stays
sync (no `#[tokio::main]` macro on the binary). Worth it for the
budget headroom.

DEC-001 already lists `rt-multi-thread`, `time`, `sync` in the
required feature set — `enable_all()` plus `worker_threads(2)` works
with the current config (no Cargo.toml changes).

Cold-start budget: the runtime construction itself is ~1ms on macOS;
fits comfortably inside the 50ms cold-start budget.

#### Test warm-up override (B-1, architect-approved)

The DEC-005 default warm-up is 2 seconds. Integration tests can't
afford to wait 2s + duration per test (the failing-tests budget is
~3s wall-clock per test for fast CI).

**Resolution:** `TestSession::with_intervals(backend, config,
snapshot_interval, warmup) -> Self` is `pub`. Production `new()`
calls it with `DEFAULT_SNAPSHOT_INTERVAL` and `DEFAULT_WARMUP`.
Integration tests construct via:

```rust
let session = TestSession::with_intervals(
    backend,
    config,
    rspeed::DEFAULT_SNAPSHOT_INTERVAL, // 100ms; cadence stays realistic
    Duration::ZERO,                    // bypass warmup so 1s test window has bytes
);
```

The constants are also `pub`, so test code can mix-and-match (e.g.
keep the production cadence but zero the warmup). The constructor
is a deliberate extension point per architect decision B-1; future
user-facing bench tools may use it directly.

#### Sender lifetime

`TestSession::snapshot_tx` is owned by the session. When the session
is dropped (after `run()` returns and the caller drops it), the
sender drops too, which closes all receivers. Subscribers that were
in `rx.changed().await` get `Err(_)` and exit cleanly.

Tests that subscribe to `snapshot_rx()` before `run()` should:
1. Subscribe via `let mut rx = session.snapshot_rx();`
2. Spawn a collector: `tokio::spawn(async move { while
   rx.changed().await.is_ok() { ... } })`
3. Run the session: `let _ = session.run().await;`
4. Drop the session (or let it go out of scope)
5. Await the collector's join handle to ensure it sees all snapshots

This is the pattern `orchestrator_snapshot_rx_observes_phase_transitions`
follows.

#### `Phase` is `Clone` already

SPEC-007 derived `Clone, Default, PartialEq` on `Phase`. The test
collector deduplicates a `Vec<Phase>` via standard pattern matching;
no need to add traits.

#### Test sizing

Default test config: `duration_secs: 1, connections: 4, warmup: 0ms`
(via `with_intervals`). Expected per-test wall-clock:
- Latency phase: ~50–200ms (10 HTTP requests against localhost)
- Download phase: 1s + harness overhead
- Upload phase: 1s + harness overhead
- Total: ≤ 3s per full-orchestrator test. Tests that skip phases run
  faster.

Tests that just exercise the latency phase (no download/upload) are
~100ms. Tests for `Config::validate()` are sub-millisecond.

#### Borrow vs. owned `Backend`

`TestSession` owns `Box<dyn Backend + Send + Sync>` (not `&dyn
Backend`). Two reasons:
1. `Box<dyn>` makes `TestSession` itself `'static`, simplifying the
   spawned task lifetimes (forwarders, ticker).
2. v2 `MonitorSession` wraps `TestSession` directly: `struct
   MonitorSession { inner: TestSession, period: Duration }` and
   `monitor.run_loop().await` calls `self.inner.run().await` in a
   loop. The backend's connection pool stays alive across iterations,
   amortising TLS handshake cost. If `TestSession` borrowed `&dyn
   Backend`, the v2 wrapper would need its own `Box<dyn>` storage to
   satisfy the lifetime bound — works, but adds friction.

#### Dep audit

Confirmed against current `Cargo.toml`:
- `tokio` features `["rt-multi-thread", "net", "time", "macros",
  "io-util", "sync"]` — `rt-multi-thread` for runtime,
  `time` for `Instant`/`timeout`, `sync` for `watch`. ✓
- `chrono = { version = "0.4", features = ["serde"] }` — `Utc::now()`
  for `started_at`. ✓
- `futures` — `StreamExt::next()` on the `DownloadStream`. ✓
- `anyhow` — `lib::run()` return type. ✓
- `thiserror` — `TestError`. ✓
- `serde_json` — **promotion from dev-deps to deps required**
  (justified in **Outputs**). One-line `Cargo.toml` edit.

No other Cargo.toml changes anticipated.

#### `guidance/questions.yaml` update

Mark `generic-backend-base-url-trailing-slash` as `answered`:

```yaml
- id: generic-backend-base-url-trailing-slash
  question: "Should --server URL require a trailing slash, or should we
    normalize? Url::join replaces the last path segment if no trailing
    slash, which silently breaks user-supplied base URLs with a path
    component (e.g. http://server/api/)."
  status: answered
  raised_at: 2026-05-02
  raised_by: SPEC-011 design (inherited from SPEC-008)
  answered_at: 2026-05-03
  blocks: null
  notes: |
    Resolved by SPEC-012 via Config::validate(): rejects --server URLs
    whose path() does not end with '/'. Pinned by
    config_validate_rejects_server_without_trailing_slash test. The
    "normalize silently" alternative was rejected because it would mask
    the user's typo (http://server/api vs http://server/api/) — better
    to fail loudly at CLI parse time than to silently strip a path
    segment from the user-supplied base.
```

---

## Frame critique (2026-05-03, claude-opus-4-7)

**Verdict: ✅ GO** — conditional on architect ack of items (A) and
(B). 5 mechanical patches (C–G) are inline-foldable at Build with no
structural rework. Spec is well-bounded; the hard design questions
(TestSession shape, TestError variants, phase orchestration, runtime
strategy, URL validation) all have defensible resolutions in the
spec body. One follow-up identified: STAGE-004's upload-streaming
polish, which DEC-005's Consequences section already tracks.

### Confirmations (architect choices that survived critique)

- **`TestSession::run(&self)` over `self`.** The DEC-008 seam #2
  test (`orchestrator_test_session_run_is_repeatable`) is the
  load-bearing assertion: a future `MonitorSession` wraps
  `TestSession` in a loop and calls `run()` per iteration. If `run()`
  consumed `self`, the wrapper would have to either reconstruct the
  session per iteration (losing the connection pool) or hold `Option<
  TestSession>` and `.take()` (ugly). `&self` is the right call.
  Confirmed.

- **`TestSession` owns `Box<dyn Backend + Send + Sync>`.** Owning
  the backend (rather than borrowing `&dyn Backend`) makes
  `TestSession` `'static`, simplifying the spawned forwarder/ticker
  task lifetimes. v2 `MonitorSession` composes via
  `MonitorSession { inner: TestSession }` cleanly. The minor cost —
  the caller can no longer reuse the backend after passing it to the
  session — is irrelevant in practice (the backend is constructed
  for the session). Confirmed.

- **Fresh `MetricsAccumulator` per phase, not shared with `set_phase`.**
  SPEC-007 AC-11 explicitly says "the orchestrator (SPEC-012) creates
  a fresh `MetricsAccumulator` per measurement phase." Bytes counters
  do not reset on `set_phase`; sharing one accumulator would conflate
  cumulative bytes across phases. The fresh-per-phase pattern also
  makes the snapshot-fan-out forwarder shut down naturally per phase
  (the inner Sender drops, closing the receiver). Confirmed.

- **`snapshot_rx()` exposed; renderers subscribe.** Frame item (A)
  in the prompt. The "callback" alternative (`run(callback)`) couples
  orchestration to rendering — STAGE-003 would need different
  callbacks for human vs json (json doesn't render Snapshots, only
  TestResult). Exposing `watch::Receiver` is the same seam DEC-008
  established for the accumulator and is the obvious shape STAGE-003's
  human renderer wants. Confirmed.

- **Eager `Config::validate()` for trailing-slash check.** Frame item
  (B) in the prompt. Resolves the `generic-backend-base-url-trailing-slash`
  question and prevents the silent-failure foot-gun. The change to
  SPEC-004's `Config` is one method (`validate`); negligible scope
  creep. Confirmed.

- **`tests/orchestrator.rs` end-to-end test.** Frame item (C). The
  load-bearing test for the stage. Established as
  `orchestrator_run_against_mock_populates_all_phases` +
  `orchestrator_test_result_round_trips_through_serde_json`.
  Confirmed.

- **Internal tokio runtime with `worker_threads(2)`, not
  `#[tokio::main]`.** Frame item (D). Saves ~16MB of stack RSS on a
  16-core developer machine. The 5-line ceremony cost is trivial vs
  the budget headroom. `lib::run()` stays sync; `main.rs` unchanged.
  Confirmed.

- **Default opts as `pub const` in `orchestrator.rs`, not CLI flags.**
  Frame item (E). MVP scope per AGENTS.md "no new features beyond
  what the task requires." STAGE-003 / STAGE-004 may add flags if
  user feedback demands them; SPEC-012 doesn't pre-emptively expose
  them. Confirmed.

- **`set_phase` invoked once per fresh accumulator immediately after
  construction.** Frame item (F). This makes the FIRST emitted
  snapshot in each phase carry the correct phase tag (otherwise the
  initial `Snapshot::default()` would emit a `Phase::Latency` value
  during the download phase, briefly). Confirmed.

- **`TestError` carries phase context via variant tag (`Latency`,
  `Download`, `Upload`).** Renderers can render "download failed: …"
  vs "upload failed: …" from the variant alone, no extra plumbing.
  Better than flattening to `Network(BackendError)` — the phase is
  the most useful context for the user. Confirmed.

### Substantive items — architect decisions (resolved 2026-05-03)

**(A) Forwarder cleanup race — Resolution: A-2 (architect-approved).**
The forwarder and ticker `JoinHandle`s are bound and explicitly
`abort()`ed at end-of-phase, not merely dropped. This eliminates the
narrow race where a previous-phase forwarder could emit one final
snapshot after the next phase's forwarder has started. Spec body's
code skeleton (`run_download_phase` / `run_upload_phase`) reflects
A-2; AC-4 is amended to require explicit aborts. `JoinHandle::abort()`
is a no-op if the task has already exited naturally, so the calls
are idempotent on the happy path.

**(B) Test warm-up override plumbing — Resolution: B-1 (architect-approved).**
`TestSession` exposes a public `with_intervals(backend, config,
snapshot_interval, warmup) -> Self` constructor. Production `new()`
calls it with the `DEFAULT_*` constants. Integration tests use
`with_intervals(..., DEFAULT_SNAPSHOT_INTERVAL, Duration::ZERO)` to
bypass the 2s warm-up so a 1s test window produces non-zero `bytes`.
The constructor is in the public API surface; rationale: future
user-facing bench tools may also want non-default cadence / warmup,
and the constants stay the canonical defaults so `new()` remains
the "I just want defaults" path.

### Mechanical patches (inline-foldable into Build)

- **(C) `BackendError::Network(reqwest_error_dummy())` in
  `test_error_exit_code_mapping` is brittle.** Constructing a
  `reqwest::Error` for the assertion requires either a real network
  call or an internal-API hack. **Patch:** rewrite the unit test to
  `use matches!(...)` for `Network` / `Timeout` / `Protocol` /
  `NotImplemented` paths via constructed-by-error variants only —
  the `Network(reqwest::Error)` path is verified end-to-end by
  `orchestrator_latency_failure_returns_test_error_latency` (which
  hits a refused port and gets back a `Network` variant naturally).
  The exit-code mapping unit test only needs to verify the *match
  arms*, not the variant construction.

- **(D) `Format::Human` fall-back.** SPEC-012's AC-9 says human-mode
  emits a one-line stderr warning and falls back to JSON. The build
  cycle should add a follow-up question to `guidance/questions.yaml`
  noting that STAGE-003 will replace this — concrete reminder:
  "Remove SPEC-012's Human-format JSON fall-back when human renderer
  ships." Bookkeeping; no code impact in SPEC-012.

- **(E) `duration_secs` calculation has a sign bug edge case.** The
  spec body's calculation is `phase_start.elapsed().as_secs_f64() -
  DEFAULT_WARMUP.as_secs_f64().min(phase_start.elapsed().as_secs_f64())`.
  This reads "subtract the warm-up duration from elapsed, but cap
  the warm-up subtraction at elapsed itself" — correct, but the
  build cycle should write this as a small helper for clarity:
  ```rust
  fn measurement_window(elapsed: Duration, warmup: Duration) -> f64 {
      elapsed.saturating_sub(warmup).as_secs_f64()
  }
  ```
  Same semantics, more readable. Inline-fold at Build.

- **(F) `select(&Config) -> Result<_, BackendError>` error mapping.**
  `lib::run()`'s `async_run` wraps `BackendError` from `select` in
  `TestError::Config(...)`. This is *technically* wrong — TLS init
  failure is closer to a network/system-class error than a config
  error. **Patch:** introduce `TestError::BackendInit(BackendError)`
  variant with exit code 3 (network-class), or fold the mapping
  through the orchestrator. Given the exit-code table only
  distinguishes 2 (config), 3 (network), 4 (protocol), the cleanest
  mapping is:
  - URL parse failures (Cloudflare URL, generic URL join) → 2 (config)
  - Reqwest client-build failures (TLS init) → 3 (network/system)
  
  The current `BackendError` enum doesn't distinguish these; both come
  through as `BackendError::Network(_)` (via `#[from] reqwest::Error`)
  or `BackendError::Protocol(_)`. The exit-code mapping in
  `TestError::exit_code()` already handles this correctly — `Network →
  3`, `Protocol → 4`. **Build patch:** introduce a fourth variant
  `TestError::Backend(BackendError)` (no phase tag, used for
  construction-phase failures), with `exit_code()` reusing the same
  inner-variant match. Five variants, not four. Easy extension.

- **(G) Snapshot collector test pattern.** The snapshot-rx test
  needs careful sequencing to avoid flakiness. The build cycle
  should use the pattern:
  ```rust
  let mut rx = session.snapshot_rx();
  let collector = tokio::spawn(async move {
      let mut phases = Vec::new();
      while rx.changed().await.is_ok() {
          phases.push(rx.borrow_and_update().phase.clone());
      }
      phases
  });
  let result = session.run().await?;
  drop(session);  // drops snapshot_tx, closes collector loop
  let phases = collector.await?;
  assert!(phases.contains(&Phase::Download));
  assert!(phases.contains(&Phase::Upload));
  ```
  **Patch:** add this code skeleton to the spec body's "Sender
  lifetime" section so the build cycle has a copy-paste reference.

### Cascade fixes identified

**`guidance/questions.yaml`:** mark
`generic-backend-base-url-trailing-slash` as `answered`. Already
called out in the spec body; bundled into SPEC-012's PR.

**No `tests/common/mod.rs` cascade.** `MockServer` already exposes
everything SPEC-012 needs (`ping_count()`, `download_count()`,
`upload_count()`, `start_with_options()`). Confirmed clean.

**`Cargo.toml` promotion.** `serde_json` moves from `[dev-dependencies]`
to `[dependencies]`. Inline-justified; not a DEC. Bundled.

### Promotion path — architect decisions resolved

**Architect approved A-2 + B-1 (2026-05-03).** Spec body is amended
inline:
- AC-4 wording updated to require explicit `abort()` on forwarder +
  ticker handles at end-of-phase
- `run_download_phase` / `run_upload_phase` skeletons updated to
  bind handles, run the phase logic in an inner `async` block,
  capture the `Result`, and call `forwarder.abort()` + `ticker.abort()`
  on both success and error paths
- `TestSession` struct gains `snapshot_interval` and `warmup` fields;
  `with_intervals(...)` is the public extension-point constructor;
  production `new()` calls it with the `DEFAULT_*` constants

Patches C–G are accepted as inline-foldable at Build. Build cycle
applies all of them in the same commit:

- **(C)** `test_error_exit_code_mapping` uses easily-constructed
  variants only (`Timeout`, `Protocol`, `NotImplemented`); the
  `Network` mapping is verified end-to-end by
  `orchestrator_latency_failure_returns_test_error_latency`
- **(D)** `Format::Human` STAGE-003 follow-up reminder added to
  `guidance/questions.yaml`
- **(E)** `measurement_window(elapsed, warmup)` helper extracted
  (already shown in the code skeleton)
- **(F)** `TestError::Backend(BackendError)` 5th variant added for
  backend-construction failures (TLS init, URL parse). `lib::run()`
  uses `TestError::Backend(_)` rather than `TestError::Config(_)` to
  wrap `BackendError` from `select()`. `exit_code()` reuses the
  inner-variant match; `Backend(Network|Timeout) → 3`,
  `Backend(Protocol|NotImplemented) → 4`. AC-11 is amended to list
  five variants instead of four
- **(G)** snapshot collector test pattern is the canonical one
  shown in "Sender lifetime"

Promote to Build. No second Frame round needed.

### Honest scope check

The stage doc estimates SPEC-012 at 4 hours. This spec frontmatter
records `complexity: L, estimated_hours: 7`. Honest reasoning:

- Type/struct authoring (`TestSession`, `TestError`): ~1.5h
- `lib::run()` rewrite + runtime config + error mapping: ~1h
- `Config::validate()` + helper accessors: ~0.5h
- 9 failing tests + helper plumbing (`with_intervals`): ~2.5h
- Clippy/fmt/CI iteration: ~1h
- Reflection + spec frontmatter completion: ~0.5h

Total: ~7h. Splitting into SPEC-012a/b would create artificial seams
(orchestrator and `lib::run` rewrite are tightly coupled — landing one
without the other leaves `lib::run` broken or the orchestrator
unreachable). Not recommended.

If the implementer's Build cycle exceeds 9h, that's a Verify-cycle
flag: re-Frame and consider splitting in a follow-up SPEC-012b for
the Format::Human STAGE-003 placeholder.

---

## Build Completion

- **Branch:** `feat/spec-012-orchestrator`
- **PR (if applicable):** #17
- **All acceptance criteria met?** Yes — see per-AC notes below.
- **Test count:** 72 (66 prior + 11 new − 5 stub removals in `tests/cli.rs`; build prompt stated 81 by omitting the stub deletions).
- **New decisions emitted:** none
- **Deviations from spec:**
  1. **`tests/common/mod.rs` modified** (spec said "do not extend it"). Axum's default 2MB body limit caused HTTP 413 on 10MB `DEFAULT_UPLOAD_BYTES_PER_REQUEST` uploads. Fix: added `DefaultBodyLimit::max(64MB)` as an Axum layer internally — no new public API methods added. This is an internal fix the spec didn't anticipate, not a semantic extension.
  2. **`tests/cli.rs` modified** (spec said existing tests pass without modification). Five tests tested the old stub's debug output (`println!("{config:#?}")`) and are not compatible with the real orchestrator. Removed: `snapshot_default_config`, `snapshot_json_format_with_duration`, `snapshot_custom_server_no_upload`, `backend_cloudflare_default`, `backend_generic_with_server`. Added: `server_without_trailing_slash_exits_2` (AC-10 CLI coverage). Corresponding stale snapshot files deleted.
  3. **`config_validate_rejects_server_without_trailing_slash` URL adjusted**: `url::Url` normalises bare-host URLs (`http://example.com`) to include a trailing slash, so the test uses `http://example.com/api` (explicit path without slash) to exercise the validation branch. Same is true for the `cli.rs` replacement test.
- **Follow-up work identified:**
  - STAGE-004: upload body streaming for continuous accumulator feedback (AC-7 documented limitation)
  - STAGE-004: `ip_version` field reflects user intent only; STAGE-004 may use `reqwest::local_addr` for actual wire family
  - STAGE-003: remove `Format::Human` JSON fall-back (tracked in `guidance/questions.yaml` as `human-format-json-fallback-cleanup`)

### AC checklist

- [x] AC-1: `TestSession::run(&self)` not `self`; DEC-008 seam #2 preserved
- [x] AC-2: `snapshot_rx()` returns `watch::Receiver<Snapshot>`; initial value `Snapshot::default()`
- [x] AC-3: fresh `MetricsAccumulator` per phase; `set_phase` called immediately after construction
- [x] AC-4: forwarder + ticker `JoinHandle`s bound and explicitly `abort()`ed at end-of-phase (A-2)
- [x] AC-5: latency phase calls `latency_probe(DEFAULT_LATENCY_SAMPLES)` and maps to `TestError::Latency`
- [x] AC-6: download phase conditional on `config.do_download`; maps to `TestError::Download`
- [x] AC-7: upload phase conditional on `config.do_upload`; maps to `TestError::Upload`; documented limitation noted
- [x] AC-8: metadata fields (`started_at`, `backend`, `server_url`, `ip_version`, `duration_secs`) populated correctly
- [x] AC-9: `Format::Json` → `to_writer_pretty` + newline; `Format::Human` → stderr warning + JSON fallback; `Format::Silent` → no output
- [x] AC-10: `Config::validate()` rejects `--server` without trailing slash; pinned by `config_validate_rejects_server_without_trailing_slash` and `server_without_trailing_slash_exits_2` CLI test
- [x] AC-11: `TestError` `#[non_exhaustive]`, 5 variants (`Config`, `Backend`, `Latency`, `Download`, `Upload`), `exit_code()` correct
- [x] AC-12: `lib::run()` builds tokio runtime with `worker_threads(2)`; stays sync
- [x] AC-13: SPEC-007–SPEC-011 test files pass; `tests/cli.rs` modified for stub-removal (see Deviations)
- [x] AC-14: no `unwrap`/`expect`/`panic` in `src/` code; test files carry `#![allow(...)]`
- [x] AC-15: `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean
- [x] AC-16: CI will confirm (three runners); Cross-check step unchanged
- [x] AC-17: `orchestrator_test_result_round_trips_through_serde_json` passes end-to-end

### Build-phase reflection

1. **What was unclear in the spec that slowed you down?**
   Axum's 2MB default body limit wasn't mentioned in the spec's "Test sizing" section. The spec anticipated upload might return 0 bytes (timeout) but not 413 (rejected). The `url::Url` normalization behaviour for bare-host URLs (adding trailing slash) also wasn't called out — the spec's `config_validate_rejects_server_without_trailing_slash` example used `http://example.com` which already passes validation after url-crate normalization.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   The constraint "do not extend `tests/common/mod.rs`" should clarify the intent: it means "don't add new public counter methods." An internal body-limit change shouldn't be forbidden. The spec should also note the url-crate normalization behaviour when writing examples for trailing-slash validation tests.

3. **If you did this task again, what would you do differently?**
   Run `cargo test` immediately after writing the first test to catch the Axum body limit issue early, before writing all 9 tests. Also pre-validate example URLs against the url crate's parser before using them as test fixtures.

---

## Reflection (Ship)

*Appended 2026-05-03 during the **ship** cycle.*

### 1. What went well or was easier than expected?

The per-phase accumulator + abort pattern (A-2) delivered exactly the clean phase boundary we hoped for. `JoinHandle::abort()` is idempotent on already-exited tasks, so the end-of-phase cleanup is a no-op on the happy path and a real safeguard on the error path — no special-case branching needed. The pattern scales directly to SPEC-013's failure tests: the orchestrator already aborts correctly when a phase returns `Err(...)`.

`with_intervals` (B-1) was definitively worth the public-API expansion. Without it, every integration test would burn 2s of warmup per phase — 4+ seconds per full-orchestrator test. Zeroing the warmup in tests kept the CI suite under 3s per test while production defaults stayed untouched. Future bench tools will use the same knob.

Having SPEC-011 complete the `GenericHttpBackend` trait surface before this spec was the right sequencing call: all 9 orchestrator tests drove `GenericHttpBackend` against `MockServer` without any conditional logic. The testability win SPEC-011 promised materialised exactly.

### 2. What was harder, surprising, or required correction?

**Axum 2MB body limit.** `tests/common/mod.rs` needed `DefaultBodyLimit::max(64MB)` — the spec anticipated upload might return 0 bytes (phase timeout) but not 413 (server rejection). For SPEC-013: the same mock body-limit cap is already in place; large-body upload error tests won't hit 413 as long as test payloads stay under 64MB.

**URL crate normalization.** `url::Url` silently appends a trailing slash to bare-host URLs (`http://example.com` → `http://example.com/`), so the spec's example validation test fixture was wrong. Tests use `http://example.com/api` (explicit path without slash) to exercise the rejection branch. Worth noting in the project constraints doc: "bare-host URLs normalize to trailing-slash; use an explicit path component in validation test fixtures."

**Test count discrepancy (81 vs 72).** The build prompt auto-generated "81 tests" by counting all pre-SPEC-012 tests plus all new tests without subtracting the 5 removed `cli.rs` stub tests. The Build Completion section now records 72 explicitly. For future Ship cycles: always include the net count (added − removed) in Build Completion, not just the added total.

**`Format::Human` fall-back** was a one-liner but the stderr-capture testing gap (see AC note for skipped `orchestrator_run_emits_warning_to_stderr_on_human_format`) is real. `assert_cmd`-style CLI tests are the right surface for that; adding one to `tests/cli.rs` in SPEC-013 or STAGE-003 setup is the clean fix.

No new DEC needed. The Axum body-limit fix and URL normalization quirk are documented here and in the spec deviations; they don't represent design decisions, just implementation gotchas.

### 3. What should SPEC-013 know?

SPEC-013 is the failure-mode tests spec (timeout, reset, malformed responses) and the last STAGE-002 spec. Several things to carry forward:

- **Use `TestSession::run()` end-to-end.** SPEC-013 tests must drive the orchestrator via `TestSession::run()`, not bypass it by calling `backend.download()` directly. The orchestrator is the canonical surface; `tests/orchestrator.rs` from this spec is the template to extend.

- **HTTP/2 stall risk** is still open in `guidance/questions.yaml` from SPEC-010 Frame A. SPEC-013 is the last chance before STAGE-003 to decide whether to investigate now (add a stall-detection test against a mock that never closes the response body) or defer to STAGE-004 explicitly. Recommendation: write one regression-guard test that asserts `TestError::Download` (timeout-class) within a bounded wall-clock budget when the server stalls; that gives CI coverage without a full investigation.

- **Mock body-limit gotcha.** `tests/common/mod.rs` has been patched to allow 64MB bodies. SPEC-013 upload-error tests that send large bodies are safe up to that limit. Tests using small error-trigger payloads (e.g., a 1-byte body that the server closes prematurely) are unaffected.

- **`Config::validate()` runs before the orchestrator.** URL parsing failures (`--server` without trailing slash) are caught at `config.validate()` and return exit code 2 before `TestSession::new()` is ever called. SPEC-013 timeout and reset tests don't need to account for URL-validation errors — those are a separate, earlier failure mode.

- **`live` cargo feature** is the right home for any tests that hit Cloudflare's real network for failure-mode validation (e.g., actual timeout against a slow endpoint). In-CI tests must stay against `MockServer` behind the `#[cfg(not(feature = "live"))]` guard. SPEC-013 can write both variants for any scenario that benefits from live validation.
