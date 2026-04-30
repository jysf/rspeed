---
task:
  id: SPEC-008
  type: story
  cycle: design
  blocked: false
  priority: high
  complexity: M
  estimated_hours: 4

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: rspeed

agents:
  architect: claude-opus-4-7
  implementer: claude-opus-4-7
  created_at: 2026-04-29

references:
  decisions: [DEC-002, DEC-003, DEC-004]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-005, SPEC-006, SPEC-007]

value_link: "delivers the latency phase of TestResult — the first STAGE-002 spec that exercises real network code"

cost:
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-008: Latency probe with HTTP RTT and TCP fallback

## Context

Second spec under STAGE-002. SPEC-007 shipped the type layer
(`MetricsAccumulator`, `Snapshot`, `TestResult`, `LatencyResult`,
`compute_latency_result`). SPEC-008 lands the first real network code in
the project: a latency probe that populates `LatencyResult` with actual
measurements against either Cloudflare or a Generic HTTP server.

DEC-004 locks the strategy: **HTTP RTT primary, TCP-connect fallback**.
The probe issues N+1 small `GET` requests to a designated ping endpoint
(discarding the first to exclude TCP+TLS+DNS warm-up), and falls back to
N+1 TCP-connect attempts if any HTTP request fails (HTTP-layer error or
non-2xx status). The fallback method is reported via `latency.method`
(`"http_rtt"` or `"tcp_connect"`) per DEC-006's JSON schema.

This spec also lands the `BackendError::Timeout` variant deferred at
SPEC-005 (`BackendError`'s doc comment explicitly anticipates it as a
non-breaking extension). It evolves the `Backend::latency_probe()`
return type from `Result<Vec<Duration>, _>` to
`Result<LatencyProbeOutcome, _>` so the method tag is preserved across
the trait boundary — SPEC-005's notes flagged this as a likely STAGE-002
refactor.

## Goal

Implement `Backend::latency_probe()` for `CloudflareBackend` and
`GenericHttpBackend`. Both delegate to a shared private helper in
`src/backend/latency.rs` that runs an HTTP RTT probe against the ping
endpoint and falls back to TCP-connect probes on failure. Both backends
gain a `reqwest::Client` field configured per DEC-002 (`no_proxy()`).
The trait return type changes to `LatencyProbeOutcome { method:
&'static str, samples: Vec<Duration> }`, and `BackendError::Timeout`
lands as a new non-breaking variant.

## Inputs

- **`decisions/DEC-004-latency-strategy.md`** — strategy: HTTP RTT
  primary, TCP-connect fallback; default 10 samples; `latency.method`
  tag in the JSON output
- **`decisions/DEC-002-http-client.md`** — `reqwest::Client` config:
  `default-features = false`, `["rustls", "stream", "http2"]`; disable
  HTTP proxy auto-detection via `.no_proxy()`; `Accept-Encoding:
  identity` on requests (irrelevant for `/ping` since responses are
  empty, but consistent with project posture)
- **`decisions/DEC-003-backend-abstraction.md`** — Generic protocol's
  `/ping` endpoint contract; Cloudflare uses `/__ping`
- **`src/backend/mod.rs`** — current trait shape and `BackendError`
  enum; the trait shape evolves here per SPEC-005's provisional-evolution
  note
- **`src/backend/cloudflare.rs`** + **`src/backend/generic.rs`** —
  current stubs returning `Err(NotImplemented)`; SPEC-008 fills in
  `latency_probe()` and gives both a `reqwest::Client` field
- **`tests/common/mod.rs`** — MockServer fixture; SPEC-008 extends it
  with a request counter on `/ping` and an alternate constructor for
  forcing HTTP failure / slow responses
- **`src/result.rs`** — `compute_latency_result(method, &samples)`
  helper; SPEC-008 does not call it (the orchestrator does in SPEC-012)
  but the test in AC-9 below threads outcome.samples through it as an
  integration check

## Outputs

- **Files created:**
  - `src/backend/latency.rs` — private module containing the shared
    HTTP-RTT-with-TCP-fallback helper; visibility `pub(crate)`
  - `tests/latency.rs` — integration tests for the probe (see
    **Failing Tests** below)
- **Files modified:**
  - `src/backend/mod.rs`:
    - declare `mod latency;`
    - update `latency_probe` trait method return type to
      `Result<LatencyProbeOutcome, BackendError>`
    - add `LatencyProbeOutcome` struct (`#[non_exhaustive]`, public,
      with `pub fn new(...)` constructor)
    - add `BackendError::Timeout(Duration)` variant; update the
      orchestrator-translation doc comment to include Timeout → 3
  - `src/backend/cloudflare.rs`:
    - add `client: reqwest::Client` field
    - implement `Default::default()` to build the client with
      `.no_proxy()`
    - implement `latency_probe()` by delegating to
      `latency::probe(&self.client, ProbeConfig { ... })`
  - `src/backend/generic.rs`:
    - add `client: reqwest::Client` field
    - update `pub fn new(base_url: Url) -> Self` to also build the
      client with `.no_proxy()`
    - implement `latency_probe()` analogously
  - `src/lib.rs` — re-export `LatencyProbeOutcome`
  - `tests/common/mod.rs`:
    - add an `Arc<AtomicU64>` ping counter wired through axum `State`
    - expose `MockServer::ping_count() -> u64`
    - add `MockServer::start_with_options(opts: MockOptions)` (or
      equivalent named-options API) so tests can opt into a 404 `/ping`
      response or a `tokio::time::sleep`-delayed `/ping` handler;
      `start()` becomes a thin wrapper using `MockOptions::default()`
- **New exports (from `src/lib.rs`):**
  ```rust
  pub use backend::LatencyProbeOutcome;
  ```
  (`BackendError` is already re-exported; the new `Timeout` variant
  needs no extra export.)
- **`Cargo.toml`:** **no new top-level deps.** `tokio::time::timeout`,
  `tokio::time::error::Elapsed`, and `tokio::net::TcpStream` are all
  already available via the SPEC-005 `tokio` feature set
  (`["rt-multi-thread", "net", "time", "macros", "io-util", "sync"]`).
  `reqwest`, `bytes`, `futures` are already in the dep graph.

## Acceptance Criteria

- [ ] **AC-1: Trait shape evolved correctly.** `Backend::latency_probe`
  now returns `Result<LatencyProbeOutcome, BackendError>` instead of
  `Result<Vec<Duration>, BackendError>`. `LatencyProbeOutcome` is
  `#[non_exhaustive]` with public `method: &'static str` and `samples:
  Vec<Duration>` fields and a `pub fn new(method: &'static str,
  samples: Vec<Duration>) -> Self` constructor. SPEC-005's "trait
  shape provisional" caveat is honored — DEC-003 gets an inline
  refinement bullet documenting the change.

- [ ] **AC-2: `BackendError::Timeout` variant added.** Variant shape:
  `Timeout(Duration)`. `Display` impl renders as `"timed out after
  {duration:?}"`. Added below the existing variants so existing match
  arms with explicit variants still compile. The orchestrator-translation
  doc comment is updated to map `Timeout → 3` (network-class failure
  per AGENTS.md exit-code table).

- [ ] **AC-3: Shared `reqwest::Client` config matches DEC-002.** Both
  `CloudflareBackend::default()` and `GenericHttpBackend::new(url)`
  construct a `reqwest::Client` via `reqwest::Client::builder()
  .no_proxy() .build()?` (or `.expect("client builder")` only if the
  builder is provably infallible — prefer propagating the error if the
  signature permits, otherwise unwrap is forbidden by AGENTS.md style).
  Each backend stores the client in a private field; client builds run
  once per backend construction, not per probe.

- [ ] **AC-4: HTTP RTT probe issues exactly `samples + 1` requests.**
  Verified via `MockServer::ping_count()` after a successful probe of
  N samples (warm-up RTT discarded, N samples returned). On the happy
  path, `outcome.method == "http_rtt"`, `outcome.samples.len() ==
  samples`, and `mock.ping_count() == samples + 1`.

- [ ] **AC-5: Fallback triggers on first HTTP failure.** Any of the
  following during the probe (including the warm-up request) drops
  the probe to TCP-connect mode and returns
  `outcome.method == "tcp_connect"`:
  - HTTP-layer error (connection refused, reset, DNS failure, TLS
    failure)
  - Per-request `tokio::time::timeout` fires (returns
    `Err(Elapsed)`)
  - HTTP response status is non-2xx (`!status.is_success()`)

  Any successful HTTP samples gathered before the failure are
  discarded — the probe restarts cleanly under the TCP method to
  avoid a mixed-method `samples` slice. The fallback path runs
  `samples + 1` TCP connects (mirroring HTTP RTT's warm-up-discard
  pattern; first connect includes DNS resolution).

- [ ] **AC-6: Per-request timeout enforced via
  `tokio::time::timeout(Duration::from_secs(1), …)`.** Hard-coded 1s
  for both HTTP requests and TCP connects in the SPEC-008 build (DEC
  rationale: Cloudflare typical RTT < 50ms; 1s is generous enough that
  a loaded link doesn't false-positive, tight enough that fallback
  surfaces fast — the 10-sample cap means worst-case
  warm-up-then-N-failures wall-clock is bounded at `(samples + 1) ×
  1s` ≈ 11s before fallback, then up to another 11s if TCP also fails).
  Configurability is deferred to SPEC-012 (orchestrator) or a future
  flag.

- [ ] **AC-7: Both backends compile and pass clippy.**
  `CloudflareBackend::default()` and `GenericHttpBackend::new(url)`
  both produce backends whose `latency_probe()` method is reachable
  (no longer returns `NotImplemented`). `cargo clippy --all-targets
  -- -D warnings` passes; `cargo fmt --check` passes.

- [ ] **AC-8: Both methods return `&'static str` matching DEC-006's
  contract.** `outcome.method` is exactly `"http_rtt"` or
  `"tcp_connect"` (no other values). DEC-006's `LatencyResult.method:
  String` accepts these via `.to_string()` in the orchestrator. The
  JSON path is `latency.method` (single-segment dot path; not
  `latency_method`).

- [ ] **AC-9: Probe samples integrate with `compute_latency_result`.**
  Threading `outcome.samples` through `compute_latency_result(method,
  &samples)` produces a `LatencyResult` with non-zero `samples` count,
  positive `median_ms` / `min_ms` / `max_ms`, and `min_ms <= median_ms
  <= max_ms`. (This is an integration assertion, not a math
  re-test — `compute_latency_result`'s arithmetic is already covered
  by SPEC-007's unit tests.)

- [ ] **AC-10: All tests pass on all three primary CI runners.**
  macOS arm64 (`macos-15`), Linux x86_64 (`ubuntu-24.04`), Windows
  x86_64 (`windows-2025`). The `Cross-check x86_64-apple-darwin`
  step on macos-15 still succeeds. No `live`-feature-gated test in
  this spec — Cloudflare integration is deferred to SPEC-013.

- [ ] **AC-11: MockServer extended without breaking SPEC-006 smoke
  tests.** The four SPEC-006 smoke tests in `tests/smoke.rs` continue
  to compile against the modified `MockServer` API (no signature
  changes to `start()` / `base_url()`). `MockOptions::default()` reproduces
  the SPEC-006 happy-path behavior.

- [ ] **AC-12: No new top-level deps.** `Cargo.toml` `[dependencies]`
  block is unchanged. `[dev-dependencies]` may gain nothing if
  existing crates suffice — `tokio` `test-util` (already dev-only),
  `serde_json` (already), and `axum` (already) cover the new tests.

- [ ] **AC-13: Lib-side `unwrap`/`expect`/`panic` discipline preserved.**
  The HTTP-status check, fallback orchestration, DNS resolution, and
  client construction all propagate `BackendError` via `?` rather
  than panicking. `tests/latency.rs` carries
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` per
  STAGE-001's project-wide test convention.

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/latency.rs` unless noted; the file opens with
`#![allow(clippy::unwrap_used, clippy::expect_used)]` and `mod
common;`.

---

**`tests/latency.rs`**

- `"http_probe_happy_path_against_mock"` —
  `#[tokio::test]`. Starts MockServer (default opts); constructs
  `GenericHttpBackend::new(mock.base_url())`; calls
  `backend.latency_probe(5).await`; asserts:
  - `outcome.method == "http_rtt"`
  - `outcome.samples.len() == 5`
  - `outcome.samples.iter().all(|d| !d.is_zero())`

- `"http_probe_warmup_request_count"` —
  `#[tokio::test]`. Same setup as above; after the probe, asserts
  `mock.ping_count() == 6`. This is the load-bearing test that the
  warm-up RTT is actually being issued (not just skipped from the
  reported samples).

- `"http_probe_falls_back_on_404"` —
  `#[tokio::test]`. Starts MockServer with `MockOptions { ping_status:
  StatusCode::NOT_FOUND, ..Default::default() }`; constructs
  `GenericHttpBackend` against it; calls
  `backend.latency_probe(3).await`; asserts:
  - `outcome.method == "tcp_connect"` (404 trigger triggered fallback)
  - `outcome.samples.len() == 3`
  - `outcome.samples.iter().all(|d| !d.is_zero())`

- `"http_probe_falls_back_on_500"` —
  `#[tokio::test]`. Same but `ping_status: StatusCode::INTERNAL_SERVER_ERROR`;
  asserts `outcome.method == "tcp_connect"`. Validates that the fallback
  trigger is `!status.is_success()`, not just 404.

- `"http_probe_times_out_then_falls_back"` —
  `#[tokio::test(start_paused = true)]`. Starts MockServer with
  `MockOptions { ping_delay: Duration::from_secs(60),
  ..Default::default() }` so `/ping` sleeps 60s before responding;
  constructs `GenericHttpBackend`; spawns the probe future; advances
  `tokio::time::advance(Duration::from_secs(2))` (past the 1s
  per-request timeout); awaits the spawned future; asserts:
  - `outcome.method == "tcp_connect"` (HTTP timed out, TCP succeeded
    against the same listener)
  - `outcome.samples.len() == 4` for `latency_probe(4)`

- `"tcp_fallback_warmup_request_count"` —
  `#[tokio::test]`. Same trigger as `falls_back_on_404`; asserts that
  TCP fallback also issued exactly N+1 connect attempts. Since
  TcpStream::connect is opaque (no per-connect counter at the
  application layer), this test verifies indirectly: it asserts
  `outcome.samples.len() == samples_requested` and that the wall-clock
  duration is plausibly larger than `samples_requested` — i.e., the
  warm-up was issued. Acceptable looseness; the strong test of "warm-up
  discarded" is the HTTP-side `http_probe_warmup_request_count`. If
  Frame disagrees on rigor here, propose tightening or removing.

- `"latency_method_strings_match_dec004_contract"` —
  `#[tokio::test]`. Runs both modes (one default mock probe, one
  404-mock probe); asserts `outcome.method` is `"http_rtt"` or
  `"tcp_connect"` exactly (string equality, not `.contains()`). This
  pins the DEC-006 `latency.method` value space.

- `"compute_latency_result_integrates_with_probe_output"` —
  `#[tokio::test]`. Runs probe; threads outcome through
  `compute_latency_result(outcome.method, &outcome.samples)`; asserts
  `result.samples == outcome.samples.len()`, `result.method ==
  outcome.method`, `result.min_ms <= result.median_ms <= result.max_ms`,
  `result.median_ms > 0.0`. Validates SPEC-008 → SPEC-007 boundary.

- `"both_http_and_tcp_fail_returns_timeout_error"` —
  `#[tokio::test(start_paused = true)]`. Constructs
  `GenericHttpBackend::new(Url::parse("http://127.0.0.1:1/").unwrap())`
  (port 1 is reserved/closed on all OSes); calls `latency_probe(3)`;
  advances time past the per-request timeout; asserts
  `result.is_err()` and the error is `BackendError::Timeout(_)` or
  `BackendError::Network(_)`. This covers the both-fail terminal
  state. Note: connecting to port 1 on `127.0.0.1` is reliably
  refused on Linux/macOS/Windows; if Windows behaves differently in
  CI, fall back to a kernel-bound-then-released port (a one-line
  helper).

  Frame-foldable alternative: if the both-fail path proves brittle
  on Windows, narrow the assertion to `result.is_err()` only and
  document the variant uncertainty in a code comment.

---

**`tests/common/mod.rs`** (extended)

The mock server gains:

```rust
#[derive(Default)]
pub struct MockOptions {
    /// Status code returned by /ping. Default: 200.
    pub ping_status: Option<axum::http::StatusCode>,
    /// Per-request delay applied to /ping (uses tokio::time::sleep,
    /// so paused-clock tests can advance over it).
    pub ping_delay: Option<Duration>,
}

impl MockServer {
    pub async fn start() -> Self {
        Self::start_with_options(MockOptions::default()).await
    }
    pub async fn start_with_options(opts: MockOptions) -> Self { ... }
    pub fn ping_count(&self) -> u64 { ... }
}
```

`ping_count` is backed by an `Arc<AtomicU64>` shared with the axum
handler via `State`. The handler increments before any optional
`tokio::time::sleep(delay).await`. The four SPEC-006 smoke tests in
`tests/smoke.rs` continue to pass against the new
`MockOptions::default()` path — confirmed by AC-11.

## Implementation Context

*Read this section (and the files it points to) before starting the
build cycle. It is the equivalent of a handoff document, folded into
the spec since there is no separate receiving agent.*

### Decisions that apply

- **DEC-004** — strategy: HTTP RTT primary, TCP-connect fallback;
  default 10 samples (caller-decided here; orchestrator passes 10);
  `latency.method` JSON tag values `"http_rtt"` / `"tcp_connect"`
- **DEC-002** — `reqwest::Client` config: `default-features = false`,
  features `["rustls", "stream", "http2"]`, `.no_proxy()` builder
  call. `Accept-Encoding: identity` is irrelevant for `/ping` (empty
  body) but the project posture is consistent
- **DEC-003** — Generic protocol's `/ping` endpoint; Cloudflare-side
  uses `/__ping` (already documented in DEC-003 / DEC-004; not
  exercised live in this spec — see AC-10 / Out of scope)

### Constraints that apply

- **`test-before-implementation`** — failing tests above are written
  first
- **`no-new-top-level-deps-without-decision`** — confirmed: zero new
  prod deps; existing tokio + reqwest + futures cover the surface

### Prior related work

- **SPEC-005** (shipped) — defined `Backend` trait, `BackendError`
  with `#[non_exhaustive]` and orchestrator-translation doc comment
  reserving `Timeout` as a future variant; defined the shared
  `reqwest::Client` configuration (see "Shared `reqwest::Client`
  configuration" subsection of SPEC-005's Implementation Context);
  flagged `latency_probe` return-type evolution as a likely
  STAGE-002 refactor
- **SPEC-006** (shipped) — MockServer fixture; this spec extends it
  with a request counter and a `MockOptions` struct
- **SPEC-007** (shipped) — `LatencyResult`, `compute_latency_result`,
  the `tokio::time` paused-clock pattern (`MissedTickBehavior::Delay`,
  `tokio::time::Instant`, `start_paused = true`). SPEC-008 reuses
  the paused-clock pattern for the timeout test only — the rest of
  the tests run in real time (network probes against MockServer are
  fast and unflaky)

### Out of scope (for this spec specifically)

- **Cloudflare live test.** Deferred to SPEC-013 (failure-mode tests).
  SPEC-008's tests all run against MockServer; the Cloudflare backend
  is exercised through unit-style construction (no live HTTP). Adding
  a `#[cfg(feature = "live")]` Cloudflare smoke is welcome there, not
  here.
- **ICMP probe.** DEC-004 explicitly excludes ICMP from MVP. A future
  spec adds it as a third method behind an opt-in `--icmp` flag.
- **`--latency-samples` CLI flag.** SPEC-004 owns CLI surface; SPEC-008
  only honors the trait method's `samples: usize` parameter. The
  orchestrator (SPEC-012) decides what to pass; default 10 per DEC-004.
- **Per-request timeout configurability.** Hard-coded 1s here.
  Configurability is a SPEC-012 / future-flag concern.
- **Streaming downloads/uploads.** SPEC-010/011 territory. SPEC-008
  touches `latency_probe` only; `download` and `upload` keep returning
  `NotImplemented`.
- **Connection-warm-without-request approach.** Issuing N+1 requests
  with discard-first is simpler and adequately models warm-up. The
  alternative (open a TCP+TLS connection without sending an HTTP
  request) is not exposed by reqwest's public API and would require
  dropping to hyper.
- **Detailed HTTP error introspection.** Reading reqwest's error type
  tree to distinguish DNS failure from connection-refused from TLS
  failure is interesting telemetry but not actionable for the
  fallback decision. Any error → fall back. SPEC-013 may add richer
  classification for renderer use.

### Notes for the implementer

#### Probe helper signature (private to crate::backend)

```rust
// src/backend/latency.rs

use std::time::Duration;
use tokio::time::Instant;
use url::Url;

use super::{BackendError, LatencyProbeOutcome};

pub(crate) struct ProbeConfig {
    pub samples: usize,
    pub per_request_timeout: Duration,
    pub ping_url: Url,
    /// host:port for TCP-connect fallback. Derived from the backend's
    /// base URL; passed in explicitly to keep the helper agnostic.
    pub tcp_target: TcpTarget,
}

pub(crate) enum TcpTarget {
    /// "host:port" (e.g., "speed.cloudflare.com:443" or
    /// "127.0.0.1:53421"). Resolved on first connect; subsequent
    /// connects benefit from OS DNS cache.
    HostPort(String),
}

pub(crate) async fn probe(
    client: &reqwest::Client,
    config: &ProbeConfig,
) -> Result<LatencyProbeOutcome, BackendError> {
    match http_probe(client, config).await {
        Ok(samples) => Ok(LatencyProbeOutcome::new("http_rtt", samples)),
        Err(_http_err) => {
            let samples = tcp_probe(config).await?;
            Ok(LatencyProbeOutcome::new("tcp_connect", samples))
        }
    }
}

async fn http_probe(
    client: &reqwest::Client,
    config: &ProbeConfig,
) -> Result<Vec<Duration>, BackendError> {
    let mut samples = Vec::with_capacity(config.samples);
    for i in 0..=config.samples {
        let start = Instant::now();
        let req = client
            .get(config.ping_url.clone())
            .header("Accept-Encoding", "identity");
        let resp = tokio::time::timeout(
            config.per_request_timeout,
            req.send(),
        )
        .await
        .map_err(|_| BackendError::Timeout(config.per_request_timeout))?
        .map_err(BackendError::Network)?;

        if !resp.status().is_success() {
            return Err(BackendError::Protocol(format!(
                "ping returned status {}",
                resp.status()
            )));
        }
        // Drain body to free the connection for the next sample.
        let _ = resp.bytes().await.map_err(BackendError::Network)?;

        let elapsed = start.elapsed();
        if i > 0 {
            samples.push(elapsed);
        }
    }
    Ok(samples)
}

async fn tcp_probe(config: &ProbeConfig) -> Result<Vec<Duration>, BackendError> {
    let TcpTarget::HostPort(addr) = &config.tcp_target;
    let mut samples = Vec::with_capacity(config.samples);
    for i in 0..=config.samples {
        let start = Instant::now();
        let connect_fut = tokio::net::TcpStream::connect(addr);
        let stream = tokio::time::timeout(
            config.per_request_timeout,
            connect_fut,
        )
        .await
        .map_err(|_| BackendError::Timeout(config.per_request_timeout))?
        .map_err(|e| BackendError::Protocol(format!("tcp connect: {e}")))?;
        drop(stream); // close immediately
        let elapsed = start.elapsed();
        if i > 0 {
            samples.push(elapsed);
        }
    }
    Ok(samples)
}
```

(Sketch; build cycle makes the precise types/error wrapping correct.)

#### `TcpTarget` derivation

For `CloudflareBackend`: hard-code `"speed.cloudflare.com:443"`.
For `GenericHttpBackend`: derive from `base_url` via
`format!("{}:{}", url.host_str()?, url.port_or_known_default()?)`,
with sensible error handling if either component is missing (return
`BackendError::Protocol("base URL missing host or port")`). Cache the
derived string on the backend struct alongside the `reqwest::Client`
so each probe doesn't re-derive.

#### `Instant` choice — `tokio::time::Instant`

Per SPEC-007's DEC-008 cadence note (and the SPEC-007 build reflection):
use `tokio::time::Instant`, not `std::time::Instant`. Under `#[tokio
::test(start_paused = true)]`, `tokio::time::Instant::elapsed()`
honors `tokio::time::advance`. In production the two types are
interchangeable — `tokio::time::Instant` is a thin wrapper. Without
this substitution the timeout test will deadlock under paused time.

#### `BackendError::Timeout` placement and translation

Add the variant *after* the existing variants so any external code
matching exhaustively on the (non-exhaustive) enum still compiles:

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("not yet implemented")]
    NotImplemented,
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("timed out after {0:?}")]
    Timeout(Duration),
}
```

Update the doc comment on the enum to extend the orchestrator
translation table:

```
/// Per AGENTS.md exit code table, the orchestrator (STAGE-002) is
/// responsible for translating variants to process exit codes:
/// `Network` → 3, `Protocol` → 4, `Timeout` → 3 (network-class).
```

We deliberately do **not** carry the `tokio::time::error::Elapsed`
source. `Elapsed` is opaque (zero-sized; no helpful payload). The
duration we attempted is more useful for both the user-facing message
and any future telemetry.

#### `MockServer` extension

`tests/common/mod.rs` adds:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use axum::extract::State;
use axum::http::StatusCode;

#[derive(Clone)]
struct AppState {
    ping_counter: Arc<AtomicU64>,
    ping_status: StatusCode,
    ping_delay: Option<Duration>,
}

#[derive(Default, Clone)]
pub struct MockOptions {
    pub ping_status: Option<StatusCode>,
    pub ping_delay: Option<Duration>,
}

async fn ping(State(s): State<AppState>) -> (StatusCode, &'static str) {
    s.ping_counter.fetch_add(1, Ordering::Relaxed);
    if let Some(d) = s.ping_delay {
        tokio::time::sleep(d).await;
    }
    (s.ping_status, "")
}
```

`MockServer` gains `ping_counter: Arc<AtomicU64>` (clone-shared with
state) and `ping_count() -> u64`. `MockServer::start()` becomes a
one-liner over `start_with_options(MockOptions::default())`.

The four SPEC-006 smoke tests in `tests/smoke.rs` continue to pass
unmodified — `MockOptions::default()` reproduces the SPEC-006 shape:
`/ping` returns 200 with empty body, no delay, counter increments
silently.

#### Why fallback restarts from scratch

If the probe got 3/N HTTP samples and then sample 4 failed, returning
those 3 + 7 TCP samples would be a mixed-method `samples` slice that
breaks the DEC-006 contract (`latency.method` is a single value). The
clean call: any failure → discard partial HTTP samples, restart in
TCP mode. Both methods then issue `samples + 1` attempts independently.
Worst-case wall-clock is `(samples + 1) * 1s` ≈ 11s per method, ≈ 22s
total before terminal failure. Acceptable for a degraded path.

#### Trait shape evolution

Changing the return type from `Vec<Duration>` to `LatencyProbeOutcome`
is a breaking change to the `Backend` trait. SPEC-005 explicitly
flagged this as expected: "STAGE-002 may extend the trait" / "may
refactor `latency_probe` to return `LatencyResult` directly". The
chosen shape (`LatencyProbeOutcome` rather than `LatencyResult`)
preserves the SPEC-007 split: backends produce raw observations;
the orchestrator (SPEC-012) computes `LatencyResult` via
`compute_latency_result`. Document the change as an inline
refinement to DEC-003 (preferred per the SPEC-005 pattern), not as a
new DEC.

DEC-003 inline addition (sketch):

> **2026-04-29 (SPEC-008 refinement):** The trait method
> `latency_probe(&self, samples: usize)` returns
> `Result<LatencyProbeOutcome, BackendError>` where
> `LatencyProbeOutcome { method: &'static str, samples: Vec<Duration> }`.
> The change preserves the backend / result-computation split: backends
> produce raw observations; the orchestrator (SPEC-012) computes
> `LatencyResult` via `compute_latency_result`. The `method` tag carries
> `"http_rtt"` or `"tcp_connect"` per DEC-004.

#### Dep audit

Confirmed against current `Cargo.toml`:
- `tokio` features `["rt-multi-thread", "net", "time", "macros",
  "io-util", "sync"]` — `time` covers `tokio::time::timeout` +
  `Instant`; `net` covers `TcpStream`. ✓
- `reqwest` features `["rustls", "stream", "http2"]` — covers HTTPS +
  HTTP/2 to Cloudflare; rustls means no system OpenSSL. ✓
- No `serde_json` in prod (still dev-only); SPEC-008 doesn't need it.
- `async-trait` 0.1 — already powers the `Backend` trait. ✓

No `Cargo.toml` changes anticipated.

---

## Frame critique (2026-04-29, claude-opus-4-7)

**Verdict: ✅ GO** — conditional on architect resolution of item (A).
6 mechanical patches (B–G) are inline-foldable at Build with no
structural rework. No NO-GO conditions; the spec is well-bounded and
the technical approach is sound.

### Confirmations (architect choices that survived critique)

- **Helper-module placement.** Shared `pub(crate)` helper in
  `src/backend/latency.rs`. Cloudflare and Generic differ only in
  ping URL and TCP target string; duplicating ~80 lines per backend
  would be churn for no isolation gain. Same module pattern as
  `src/backend/select.rs`. Confirmed.
- **`BackendError::Timeout(Duration)` shape.** Carrying the timeout
  duration we attempted, not `tokio::time::error::Elapsed` (which is
  zero-sized and opaque). Display string "timed out after {0:?}" gives
  users an actionable number; future telemetry can index on it.
  Confirmed.
- **Per-request timeout 1s.** Worst-case wall-clock budget:
  `(samples + 1) * 1s` ≈ 11s per method, ≈ 22s total before terminal
  failure. 5s would push that to 110s — user-visible "rspeed seems
  hung". 500ms gets close to false-positive on slow links (mobile,
  satellite RTT ≈ 200–400ms). 1s is the right balance. Confirmed.
- **Warm-up RTT: N+1 issue, discard first.** The connection-warm-
  without-request alternative requires dropping to hyper (reqwest
  doesn't expose it). The N+1 approach is simpler and adequately
  models the real warm-up cost (TCP+TLS+DNS). Confirmed.
- **Sample count 10 (caller-decided).** DEC-004 fixes 10. n=20 cuts
  jitter standard-error from ~30% to ~22% — real but doubles probe
  time. MVP doesn't need it. Spec just honors the trait parameter;
  default 10 lives in SPEC-012 (orchestrator). Confirmed.
- **DEC-006 path `latency.method`** (single-segment dot path) used
  consistently in spec body and AC-8. SPEC-001 verify punch-list fix
  is honored. No `latency_method` anywhere. Confirmed.
- **MockServer cascade bundled in this spec.** `tests/common/mod.rs`
  is fixture code; the extension (ping counter + MockOptions) is
  small and motivated by the test that requires it. Bundling here
  beats spawning a SPEC-006-cascade spec for a one-file fixture
  change. AC-11 pins backward compat for the four SPEC-006 smoke
  tests. Confirmed.
- **Tokio paused-clock for the timeout test only.** Other tests run
  in real time against MockServer (probes are fast and unflaky).
  Reuses SPEC-007's pattern — `tokio::time::Instant`, `start_paused
  = true`, explicit `tokio::time::advance`. Confirmed.

### Substantive item (architect decision needed)

**(A) Backend construction is fallible — `Default` impl conflicts with
fallible `reqwest::ClientBuilder::build()`.** AC-3 currently says both
`CloudflareBackend::default()` and `GenericHttpBackend::new(url)`
construct a `reqwest::Client` via `Client::builder().no_proxy().build()`
— but `ClientBuilder::build()` returns `Result<Client, reqwest::Error>`
(can fail on TLS init, system resolver init). The current
`CloudflareBackend: Default` shape from SPEC-005 expects an infallible
constructor; `Default::default` cannot return `Result`.

Two paths:

- **(A-1) Drop `Default` from `CloudflareBackend`.** Replace with
  `pub fn new() -> Result<Self, BackendError>`. Cascade:
  `select(&Config) -> Result<Box<dyn Backend + Send + Sync>,
  BackendError>` (also fallible); `lib::run()` propagates via `?`
  (already returns `anyhow::Result<i32>`, so the BackendError gets
  wrapped automatically). One additional code touch in
  `tests/cli.rs` insta snapshots — the `Backend: <name>` print
  happens after `select()` succeeds, so the snapshots don't change
  shape, but the `cli.rs` integration tests now have a `.unwrap()`
  on the new `select()?` path that's already covered by file-scope
  `#![allow(clippy::unwrap_used)]`.

  Cost: ~5 lines of cascade churn (select.rs sig, lib.rs `?`).
  Benefit: lib-side `unwrap`/`expect`/`panic` discipline preserved
  end-to-end. Production code never panics on TLS init failure;
  the user gets a structured error and exit code 3.

- **(A-2) Keep `Default`; build the client via `Client::new()` (which
  panics on init failure) or `Client::builder().build().expect(...)`.**
  AGENTS.md explicitly allows panics in startup paths *in main.rs*,
  but the construction is in `lib::run()` indirectly via `select()`.
  Pushing `expect` into lib code is a discipline regression and a
  precedent for "well, this one panic is fine."

**Architect recommendation: A-1.** It's the right shape and the cascade
is small (~5 lines). A-2 saves a one-line `?` propagation at the cost of
the project's "no panics in lib code" invariant. The latter is one of
the few hard rules in AGENTS.md; trading it for a one-liner is a bad
deal. The cascade also incidentally makes `select()` honest about its
fallibility — useful for STAGE-002 onward.

If A-1 is approved, AC-3 is amended to:

> **AC-3 (revised):** Both `CloudflareBackend::new()` and
> `GenericHttpBackend::new(url)` construct a `reqwest::Client` via
> `reqwest::Client::builder().no_proxy().build()` and return
> `Result<Self, BackendError>`. Each backend stores the client in a
> private field; client builds run once per backend construction, not
> per probe. `Backend::select(config)` is updated to
> `Result<Box<dyn Backend + Send + Sync>, BackendError>`; `lib::run()`
> propagates via `?`. The `CloudflareBackend: Default` impl is removed
> per SPEC-005's "trait shape provisional" caveat.

If A-2 is preferred (architect-level escape-hatch decision), AC-3
stays as-is; add a one-line note that the `expect()` is justified by
"TLS init failure at process startup is unrecoverable" with a code
comment citing AGENTS.md.

### Mechanical patches (inline-foldable into Build)

- **(B) Drop the `TcpTarget` enum; use `tcp_target: String`.** Only one
  variant (`HostPort`); the enum is premature abstraction and Rust's
  irrefutable-let-pattern restriction on single-variant enums makes
  the destructure awkward. Reintroduce as an enum if/when ICMP arrives
  with a `RawSocket(IpAddr)` variant.

- **(C) Test 9 name + assertion too narrow.** Rename
  `both_http_and_tcp_fail_returns_timeout_error` →
  `both_http_and_tcp_fail_returns_error`. Widen assertion to
  `result.is_err()` only with a code comment that the variant lands
  as `Timeout`, `Network`, or `Protocol` depending on the OS's
  port-1-refusal behavior (Linux/macOS typically `ECONNREFUSED` →
  `Network`/`Protocol`; Windows may differ).

- **(D) Tighten AC-12 dev-dep wording.** Change
  "[dev-dependencies] may gain nothing if existing crates suffice" to
  "[dev-dependencies] block is also unchanged." Verified: tokio
  test-util, axum, serde_json all already cover the new tests.

- **(E) `tcp_fallback_warmup_request_count` test rigor.** Drop the
  wall-clock heuristic; assert only `outcome.samples.len() ==
  samples_requested`. Add a code comment that the strong warm-up-
  discard verification lives on the HTTP side
  (`http_probe_warmup_request_count`); the TCP side mirrors the loop
  structure, so sample-count parity is sufficient.

- **(F) Connection-pool note in implementation context.** Add a
  sentence: "HTTP RTT measurement assumes reqwest's default HTTP/2
  multiplex (or HTTP/1.1 keep-alive) reuses the TCP+TLS connection
  across the N+1 ping requests. Without pooling, every request would
  include handshake overhead and the warm-up discard would be
  meaningless. reqwest's default pool max is large enough for our
  N+1=11 case; no `pool_max_idle_per_host` tuning needed."

- **(G) Mock handler refactor scope.** Clarify in the implementation
  context that the `MockServer` extension moves only `/ping` into a
  stateful (`State<AppState>`) handler; `/health`, `/download`, and
  `/upload` remain stateless. Build should not refactor the
  unaffected handlers.

### Cascade fix identified

**MockServer extension (bundled in SPEC-008's PR):** `tests/common/
mod.rs` gains:
1. `Arc<AtomicU64>` ping counter (clone-shared with axum `State`)
2. `MockOptions` struct (default = current SPEC-006 behavior)
3. `MockServer::start_with_options(opts)` constructor; `start()`
   becomes a thin wrapper
4. `MockServer::ping_count() -> u64` accessor
5. The `/ping` handler reads `State<AppState>` to access counter +
   optional status-override + optional `tokio::time::sleep` delay

The four SPEC-006 smoke tests in `tests/smoke.rs` are NOT modified;
`MockOptions::default()` reproduces the SPEC-006 happy-path behavior
exactly. AC-11 codifies this.

### Promotion path

If architect approves A-1: amend AC-3 inline (one-line edit), apply
patches B–G inline at Build, advance to Build. No second Frame round
needed.

If architect prefers A-2: amend AC-3 with the `expect()` justification
note, apply patches B–G inline at Build, advance to Build.

Either path is GO. The substantive item is a public-API-surface
question that needs an explicit yes from the architect; the mechanical
patches are non-controversial.

---
