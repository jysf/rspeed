---
task:
  id: SPEC-013
  type: story
  cycle: ship
  blocked: false
  priority: high
  complexity: S

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: rspeed

agents:
  architect: claude-sonnet-4-6
  implementer: null
  created_at: 2026-05-03

references:
  decisions: [DEC-001, DEC-002, DEC-003]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-008, SPEC-010, SPEC-011, SPEC-012]

value_link: "proves STAGE-002 invariant #3 (typed failure modes return structured errors) under adversarial conditions — without this spec the invariant exists only on the happy path"

cost:
  sessions:
    - cycle: design
      date: 2026-05-03
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 3617480
      tokens_output: 120878
      estimated_usd: 5.3045
      note: "Spec authoring + Frame critique in single Sonnet session"
    - cycle: build
      date: null
      agent: null
      interface: null
      tokens_input: 4934915
      tokens_output: 61133
      estimated_usd: 4.1413
      note: ""
    - cycle: verify
      date: null
      agent: null
      interface: null
      tokens_input: 2268051
      tokens_output: 26967
      estimated_usd: 2.8271
      note: ""
    - cycle: ship
      date: null
      agent: null
      interface: null
      tokens_input: 2853817
      tokens_output: 37460
      estimated_usd: 2.7201
      note: ""
  totals:
    tokens_total: 13920701
    estimated_usd: 14.993
    session_count: 4
---

# SPEC-013: Failure mode tests (timeout, reset, malformed)

## Context

Seventh and final spec under STAGE-002. SPEC-012 shipped the test
orchestrator (`TestSession::run()`), `TestError`, and end-to-end JSON
output. STAGE-002 critical invariant #3 states: "Every
external-network-induced failure is a typed variant of a `TestError`
enum so renderers can format it consistently." SPEC-012 establishes
this on the happy path and for connection-refused latency failures.
SPEC-013 proves the invariant holds under adversarial conditions:
stalled connections, mid-stream truncation, and non-2xx responses
verified end-to-end through the orchestrator stack.

This is the stage-closing spec. When it ships, STAGE-002 is complete
and STAGE-003 (output & UX) can begin.

## Goal

Add `tests/failure_modes.rs` with 6 integration tests that drive
`TestSession::run()` against a `MockServer` configured for adversarial
scenarios. Extend `MockOptions` with three new fields
(`download_delay`, `upload_delay`, `download_truncate_at`). Add
download/upload connection-establishment deadlines to `TestSession`
(constants + a `with_deadlines` builder method). Update
`guidance/questions.yaml` to defer the HTTP/2 stall question to
STAGE-004. Make no changes to `BackendError` or `TestError` variants.

## Scope decisions

### (A) Download/upload connection-establishment deadlines

**Decision: add them.** Rationale:

The latency probe has a 1s per-request timeout (SPEC-008).
`download_parallel`/`upload_parallel` have no corresponding timeout:
if the server accepts a TCP connection but never sends response headers,
`try_join_all(connections)` hangs indefinitely. The orchestrator's
existing phase-duration cutoff (`tokio::time::timeout(remaining,
stream.next())`) only applies to *body chunks after headers arrive* —
it does not fire if `download_parallel` itself stalls before returning.

Without deadlines:
1. A stalled server hangs the CLI forever, violating the "fast
   feedback" project thesis in `brief.md`.
2. The `BackendError::Timeout` variant is unreachable from download/upload
   code paths — making it dead in the typed-failure-mode taxonomy.

Adding connection-establishment deadlines:
- Closes the stall risk with ~10 LOC
- Gives timeout a testable entry point in CI
- Follows the latency probe precedent (symmetry)
- Constants at 60s are generous; STAGE-004 may tune from measurements

The B-1 `with_intervals` pattern is extended with a `with_deadlines`
builder method so tests can use short deadlines (500ms) without
changing production constants. No changes to `BackendError` or
`TestError` variants — `BackendError::Timeout(duration)` already
covers this.

### (B) "Malformed response" slot repurposed to truncation

The stage doc lists "malformed response tests" as one of the three
failure modes. Honest accounting:

- Download is raw bytes — no schema that can be "malformed."
- Upload response is `{"received": N}` JSON. `upload_one` checks
  HTTP status only; it does not parse the response body. A malformed
  JSON body is silently ignored — no failure surface exists.

**Substitution:** repurpose the "malformed response" slot for truncated
download body. The mock sends `Content-Length: N` but closes the
connection after streaming `N/2` bytes. reqwest surfaces this as a
`reqwest::Error` (IO-layer unexpected EOF), which maps to
`BackendError::Network` via `#[from] reqwest::Error`, and the
orchestrator surfaces `TestError::Download(BackendError::Network)`.
This is a real failure mode that no prior STAGE-002 spec tests. The
stage doc's "malformed response" intent is satisfied: the server is
sending an internally inconsistent response (Content-Length vs actual
body) that the client cannot recover from.

### (C) HTTP/2 stall question deferred to STAGE-004

The `cloudflare-http2-stall-on-parallel-download` question is
explicitly deferred to STAGE-004 (see `guidance/questions.yaml`
update). The download-timeout test added here (AC-2) provides a
regression guard: if any download phase stalls indefinitely, the
deadline fires and CI fails. That's the right CI protection for
STAGE-002; full HTTP/2 multiplexing investigation belongs in
STAGE-004 perf work.

### (D) No live-feature Cloudflare tests

All 6 tests run against `MockServer` (HTTP/1.1). The `live` cargo
feature can host real-Cloudflare failure tests in STAGE-004. SPEC-013
stays MockServer-only to keep scope tight and CI deterministic.

### (E) No new `BackendError` or `TestError` variants

The existing variants cover all failure modes this spec tests:
- `BackendError::Timeout` — stalled connection
- `BackendError::Network` — mid-stream truncation
- `BackendError::Protocol` — non-2xx response
- `TestError::Download` / `TestError::Upload` / `TestError::Latency` — phase tagging

No extensions needed.

## What this spec does NOT do

- Human/silent renderers for error output — STAGE-003
- Live Cloudflare failure-mode tests — STAGE-004
- HTTP/2 stall investigation — STAGE-004
- New `BackendError` or `TestError` variants
- Changes to `BackendError`, `TestError`, or `Backend` trait

**STAGE-002 ship cycle:** SPEC-013's Ship cycle is the last thing
before STAGE-002 ships. The Ship reflection below should draft the
stage-level reflection (see AGENTS.md §15) and run `just status` to
confirm the stage backlog is fully cleared before triggering the
Stage Ship prompt.

## Inputs

- **`src/orchestrator.rs`** — `TestSession`, `with_intervals` (B-1 extension
  point); `run_download_phase` wraps `self.backend.download(&opts).await`;
  `run_upload_phase` wraps `self.backend.upload(&opts).await` — these
  are the sites where connection-establishment deadlines land
- **`tests/common/mod.rs`** — `MockOptions`, `MockServer`; SPEC-013 adds
  three new `MockOptions` fields (all `Option<_>`, all default `None`)
- **`src/backend/mod.rs`** — `BackendError` variants; no changes
- **`src/error.rs`** — `TestError` variants; no changes
- **`tests/orchestrator.rs`** — template for `tests/failure_modes.rs`
  (same `build_config` helper shape, same `with_intervals` +
  `with_deadlines` pattern)
- **`guidance/questions.yaml`** — `cloudflare-http2-stall-on-parallel-download`:
  add `deferred_to: STAGE-004`, `deferral_rationale`, remove `blocks:
  SPEC-013`

## Outputs

### Files created

- **`tests/failure_modes.rs`** — 6 integration tests (see **Failing Tests**)

### Files modified

**`tests/common/mod.rs`:**

Add three fields to `MockOptions`:
```rust
/// Optional delay before /download sends response headers (for timeout tests).
pub download_delay: Option<Duration>,
/// Optional delay before /upload sends response (for timeout tests).
pub upload_delay: Option<Duration>,
/// If Some(n), /download streams only n bytes then closes the connection
/// mid-stream (for truncation/connection-reset tests). The advertised
/// Content-Length still reflects the full requested size, so the client
/// sees a premature EOF.
pub download_truncate_at: Option<u64>,
```

Update `MockOptions::default()` — all three fields to `None` (existing
behavior unchanged).

Add the three fields to `AppState` (internal struct).

Update `start_with_options` to propagate the new fields into `AppState`.

Update the `download` Axum handler:
```rust
async fn download(State(state): State<AppState>, Query(q): Query<DownloadQuery>) -> Response {
    state.download_counter.fetch_add(1, Ordering::Relaxed);
    if !state.download_status.is_success() {
        return Response::builder()
            .status(state.download_status)
            .body(Body::empty())
            .unwrap();
    }

    // Delay before headers (simulates stalled server for timeout tests).
    if let Some(delay) = state.download_delay {
        tokio::time::sleep(delay).await;
    }

    let n = q
        .bytes
        .unwrap_or(DOWNLOAD_DEFAULT_BYTES)
        .min(DOWNLOAD_MAX_BYTES);

    // For truncation tests: only stream this many bytes, but advertise
    // Content-Length = n so the client detects the premature EOF.
    let actual_send = state.download_truncate_at.map(|t| t.min(n)).unwrap_or(n);

    let chunk: Bytes = Bytes::from(vec![0u8; CHUNK_BYTES]);
    let full_chunks = actual_send / CHUNK_BYTES as u64;
    let tail = (actual_send % CHUNK_BYTES as u64) as usize;

    let chunks = stream::iter(
        std::iter::repeat_n(chunk.clone(), full_chunks as usize)
            .chain(if tail > 0 { Some(chunk.slice(0..tail)) } else { None })
            .map(Ok::<_, std::io::Error>),
    );

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, n) // advertises full n bytes
        .body(Body::from_stream(chunks))   // only delivers actual_send bytes
        .unwrap()
}
```

When `download_truncate_at` is set, the response advertises `n` bytes
but only streams `actual_send < n` bytes. HTTP/1.1 with an explicit
Content-Length causes hyper/reqwest to detect the premature connection
close and yield `Err(reqwest::Error)` on the next `stream.next()` call,
which maps to `BackendError::Network`.

Update the `upload` Axum handler to sleep before processing if
`upload_delay` is set:
```rust
async fn upload(State(state): State<AppState>, body: Bytes) -> Response {
    state.upload_counter.fetch_add(1, Ordering::Relaxed);

    // Delay before response (simulates stalled server for timeout tests).
    if let Some(delay) = state.upload_delay {
        tokio::time::sleep(delay).await;
    }

    if !state.upload_status.is_success() {
        return Response::builder()
            .status(state.upload_status)
            .body(Body::empty())
            .unwrap();
    }
    Json(UploadResponse {
        received: body.len() as u64,
    })
    .into_response()
}
```

Note: `upload_delay` is applied AFTER accepting the body (the TCP
receive happens before our handler runs). The delay simulates a slow
server-side handler, which means `client.post(...).body(...).send().await`
will block until after the delay. The timeout in `run_upload_phase`
wraps `self.backend.upload(&opts).await`, which includes this wait.

**`src/orchestrator.rs`:**

Add two constants:
```rust
pub const DEFAULT_DOWNLOAD_DEADLINE: Duration = Duration::from_secs(60);
pub const DEFAULT_UPLOAD_DEADLINE: Duration = Duration::from_secs(60);
```

Add two fields to `TestSession`:
```rust
pub struct TestSession {
    // ... existing fields ...
    download_deadline: Duration,
    upload_deadline: Duration,
}
```

Update `with_intervals` to initialise both fields to their defaults:
```rust
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
        download_deadline: DEFAULT_DOWNLOAD_DEADLINE,
        upload_deadline: DEFAULT_UPLOAD_DEADLINE,
    }
}
```

Add new builder method (chained after `with_intervals`):
```rust
/// Override the connection-establishment deadlines for tests and bench
/// tools. Production callers use `new()` / `with_intervals()` to get
/// the `DEFAULT_*` constants.
pub fn with_deadlines(
    mut self,
    download_deadline: Duration,
    upload_deadline: Duration,
) -> Self {
    self.download_deadline = download_deadline;
    self.upload_deadline = upload_deadline;
    self
}
```

Update `run_download_phase` — wrap `self.backend.download(&opts).await`
with the deadline:
```rust
// Original:
let mut stream = self
    .backend
    .download(&opts)
    .await
    .map_err(TestError::Download)?;

// Replace with:
let mut stream = tokio::time::timeout(
    self.download_deadline,
    self.backend.download(&opts),
)
.await
.map_err(|_| TestError::Download(BackendError::Timeout(self.download_deadline)))?
.map_err(TestError::Download)?;
```

Update `run_upload_phase` — wrap `self.backend.upload(&opts).await`
with the deadline:
```rust
// Original:
let r = self
    .backend
    .upload(&opts)
    .await
    .map_err(TestError::Upload)?;

// Replace with:
let r = tokio::time::timeout(
    self.upload_deadline,
    self.backend.upload(&opts),
)
.await
.map_err(|_| TestError::Upload(BackendError::Timeout(self.upload_deadline)))?
.map_err(TestError::Upload)?;
```

**`src/lib.rs`:**

Add new constants to the `pub use orchestrator::{...}` block:
```rust
pub use orchestrator::{
    DEFAULT_DOWNLOAD_BYTES_PER_REQUEST, DEFAULT_DOWNLOAD_DEADLINE,
    DEFAULT_LATENCY_SAMPLES, DEFAULT_SNAPSHOT_INTERVAL,
    DEFAULT_UPLOAD_BYTES_PER_REQUEST, DEFAULT_UPLOAD_DEADLINE,
    DEFAULT_WARMUP, TestSession,
};
```

**`guidance/questions.yaml`:**

Update the `cloudflare-http2-stall-on-parallel-download` entry:
```yaml
- id: cloudflare-http2-stall-on-parallel-download
  question: "Does try_join_all + select_all stall on Cloudflare's HTTP/2 multiplexed connection when N download streams are awaited concurrently?"
  priority: high
  status: open
  raised_by: SPEC-010
  raised_at: 2026-05-02
  assigned_to: null
  deferred_to: STAGE-004
  deferral_rationale: "SPEC-013 scope is failure modes (timeout/reset/malformed), not perf-multiplexing investigation; SPEC-013 (STAGE-002) closes without resolving this question; STAGE-004 perf work owns it. The SPEC-013 download-deadline test provides a regression guard: if any download phase stalls indefinitely, CI fails."
  notes: |
    (prior notes unchanged)
    ...
```

Remove the `blocks: SPEC-013` line from this entry.

**`Cargo.toml`:** no new dependencies. All required types are already
in the dep graph.

## Acceptance Criteria

- [ ] **AC-1: `TestSession::with_deadlines` builder method exists.**
  Signature: `pub fn with_deadlines(mut self, download_deadline:
  Duration, upload_deadline: Duration) -> Self`. Chainable:
  `TestSession::with_intervals(...).with_deadlines(...)`.
  Production `new()` and `with_intervals()` initialise both to
  `DEFAULT_DOWNLOAD_DEADLINE` and `DEFAULT_UPLOAD_DEADLINE` (60s each).

- [ ] **AC-2: Download stall returns `TestError::Download(
  BackendError::Timeout)`.** When `backend.download(&opts)` does not
  return within `self.download_deadline`, the orchestrator returns
  `Err(TestError::Download(BackendError::Timeout(deadline)))`.
  Pinned by `download_timeout_surfaces_test_error_download`.

- [ ] **AC-3: Upload stall returns `TestError::Upload(
  BackendError::Timeout)`.** When `backend.upload(&opts)` does not
  return within `self.upload_deadline`, the orchestrator returns
  `Err(TestError::Upload(BackendError::Timeout(deadline)))`.
  Pinned by `upload_timeout_surfaces_test_error_upload`.

- [ ] **AC-4: Mid-stream truncation returns `TestError::Download(
  BackendError::Network)`.** When the server closes the connection
  after `N < Content-Length` bytes, reqwest surfaces the premature EOF
  as `reqwest::Error` → `BackendError::Network` → `TestError::Download`.
  Pinned by `download_mid_stream_truncation_surfaces_network_error`.

- [ ] **AC-5: Latency HTTP-RTT timeout triggers TCP-connect fallback.**
  When the ping endpoint delays longer than the 1s per-request timeout,
  `http_rtt_probe` fails and the probe falls back to TCP connect. The
  orchestrator receives `Ok(LatencyProbeOutcome { method: "tcp_connect",
  ... })`. Pinned by `latency_rtt_timeout_triggers_tcp_fallback`.

- [ ] **AC-6: Non-2xx download via orchestrator returns
  `TestError::Download(BackendError::Protocol)`.** Uses existing
  `MockOptions::download_status`. Verifies the error mapping chain
  through the full orchestrator stack, not just at the `throughput.rs`
  unit level. Pinned by `download_non_2xx_via_orchestrator`.

- [ ] **AC-7: Non-2xx upload via orchestrator returns
  `TestError::Upload(BackendError::Protocol)`.** Mirror of AC-6 for
  upload. Pinned by `upload_non_2xx_via_orchestrator`.

- [ ] **AC-8: All new `MockOptions` fields default to `None`.**
  `MockOptions::default()` preserves pre-SPEC-013 behavior on all
  existing tests. No test file outside `tests/failure_modes.rs` is
  modified (only `tests/common/mod.rs` gains the new fields/handler
  logic).

- [ ] **AC-9: `guidance/questions.yaml` updated.**
  `cloudflare-http2-stall-on-parallel-download` carries `deferred_to:
  STAGE-004`, a `deferral_rationale`, and the `blocks: SPEC-013` line
  is removed.

- [ ] **AC-10: `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` pass.**

- [ ] **AC-11: All three CI runners green.** macOS arm64 (`macos-15`),
  Linux x86_64 (`ubuntu-24.04`), Windows x86_64 (`windows-2025`).
  Cross-check x86_64-apple-darwin step still succeeds.

- [ ] **AC-12: All prior tests continue to pass.** No existing test
  file is modified except `tests/common/mod.rs` (additive changes
  only). `src/lib.rs` gains additive re-exports only.

- [ ] **AC-13: Lib-side `unwrap`/`expect`/`panic` discipline preserved.**
  No `unwrap()`, `expect()`, or `panic!()` in the new `src/orchestrator.rs`
  additions. `tests/failure_modes.rs` carries `#![allow(clippy::
  unwrap_used, clippy::expect_used, clippy::panic)]`.

## Failing Tests

All 6 live in `tests/failure_modes.rs`. The file opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Adversarial failure-mode integration tests for SPEC-013.
//! All tests drive TestSession::run() end-to-end against MockServer.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::{MockOptions, MockServer};
use rspeed::{
    BackendError, ColorWhen, Config, Format, GenericHttpBackend, TestError, TestSession,
};
use rspeed::config::IpVersion;

fn build_config(mock: &MockServer) -> Config {
    Config {
        duration_secs: 2,
        connections: 1,
        server: Some(mock.base_url()),
        do_download: true,
        do_upload: false,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    }
}
```

Note: `connections: 1` in the local `build_config`. Multi-connection
scenarios have more complex failure semantics (partial success with
`try_join_all`). For failure-mode tests, a single connection gives
deterministic, fast failure. Each test override `do_download`/`do_upload`
as needed.

---

**`"latency_rtt_timeout_triggers_tcp_fallback"`** — `#[tokio::test]`.
Proves that when HTTP RTT times out (ping_delay > 1s timeout), the
probe falls back to TCP connect and the orchestrator gets `Ok` with
`method == "tcp_connect"`. This exercises the HTTP-RTT → TCP fallback
path end-to-end through the orchestrator (SPEC-008 unit tests exercise
it at the probe level; this test confirms the orchestrator handles the
`Ok(LatencyProbeOutcome)` correctly after a fallback).

```rust
#[tokio::test]
async fn latency_rtt_timeout_triggers_tcp_fallback() {
    // ping_delay > 1s per-request timeout → HTTP RTT fails, TCP fallback runs.
    let mock = MockServer::start_with_options(MockOptions {
        ping_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: false,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();
    assert_eq!(result.latency.method, "tcp_connect",
        "expected TCP fallback after HTTP RTT timeout");
    assert!(result.latency.samples > 0);
}
```

**Wall-clock budget:** ≤ 2s. The HTTP RTT probe sends the first request
and it times out at 1s (the first request is the warm-up — but actually
the probe times out on the first request and immediately falls back to
TCP; it does not retry all N). TCP connect to the mock server is ~1ms
× 11 connects ≈ 50ms total for the fallback. Total: ~1.1s.

---

**`"download_timeout_surfaces_test_error_download"`** — `#[tokio::test]`.
Proves that a stalled download (server accepts TCP connection but delays
response headers) surfaces as `TestError::Download(BackendError::Timeout)`.
Uses `with_deadlines` to set a short download_deadline (500ms) so the
test runs fast; the mock's `download_delay` (2s) is longer than the
deadline.

```rust
#[tokio::test]
async fn download_timeout_surfaces_test_error_download() {
    let mock = MockServer::start_with_options(MockOptions {
        download_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    )
    .with_deadlines(Duration::from_millis(500), rspeed::DEFAULT_UPLOAD_DEADLINE);

    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Timeout(_)))),
        "expected Download(Timeout), got: {result:?}"
    );
}
```

**Wall-clock budget:** ≤ 1s. Latency phase: ~50ms. Download: 500ms
deadline fires. Total: ~600ms.

---

**`"upload_timeout_surfaces_test_error_upload"`** — `#[tokio::test]`.
Mirror of the above for upload. `do_upload: true`, `do_download: false`.
`upload_delay` (2s) > `upload_deadline` (500ms).

```rust
#[tokio::test]
async fn upload_timeout_surfaces_test_error_upload() {
    let mock = MockServer::start_with_options(MockOptions {
        upload_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: true,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    )
    .with_deadlines(rspeed::DEFAULT_DOWNLOAD_DEADLINE, Duration::from_millis(500));

    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Upload(BackendError::Timeout(_)))),
        "expected Upload(Timeout), got: {result:?}"
    );
}
```

**Wall-clock budget:** ≤ 1s. Latency: ~50ms. Upload: 500ms deadline.
Total: ~600ms.

---

**`"download_mid_stream_truncation_surfaces_network_error"`** — `#[tokio::test]`.
Repurposes the "malformed response" failure-mode slot (see scope
decision B above). The server advertises `Content-Length: N` but
closes the connection after streaming only `CHUNK_BYTES` bytes. hyper
detects the premature EOF and reqwest surfaces it as `reqwest::Error`
→ `BackendError::Network` → `TestError::Download(BackendError::Network)`.

```rust
#[tokio::test]
async fn download_mid_stream_truncation_surfaces_network_error() {
    // Request 1MB but truncate after 64KB (one chunk). Content-Length: 1MB.
    let mock = MockServer::start_with_options(MockOptions {
        download_truncate_at: Some(64 * 1024),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Network(_)))),
        "expected Download(Network), got: {result:?}"
    );
}
```

**Wall-clock budget:** ≤ 500ms. The first chunk arrives quickly; the
stream then errors. The orchestrator's inner loop catches
`Ok(Some(Err(e))) => return Err(TestError::Download(e))` immediately.

---

**`"download_non_2xx_via_orchestrator"`** — `#[tokio::test]`.
Non-2xx already covered at `download_one` level in `tests/throughput.rs`.
This test verifies the full error-mapping chain: non-2xx → `BackendError::Protocol`
→ `TestError::Download(BackendError::Protocol)` end-to-end via the orchestrator.

```rust
#[tokio::test]
async fn download_non_2xx_via_orchestrator() {
    let mock = MockServer::start_with_options(MockOptions {
        download_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Protocol(_)))),
        "expected Download(Protocol), got: {result:?}"
    );
}
```

---

**`"upload_non_2xx_via_orchestrator"`** — `#[tokio::test]`. Mirror of
the above for upload.

```rust
#[tokio::test]
async fn upload_non_2xx_via_orchestrator() {
    let mock = MockServer::start_with_options(MockOptions {
        upload_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: true,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Upload(BackendError::Protocol(_)))),
        "expected Upload(Protocol), got: {result:?}"
    );
}
```

**Wall-clock budget for tests 5–6:** ≤ 200ms each (latency phase ~50ms +
immediate error from download/upload).

**Total test suite wall-clock:** ≤ 5s for all 6 tests running sequentially.
Tokio tests are concurrent; actual CI time is bounded by the longest
single test (~1.1s for the latency fallback test).

## Implementation Context

*Read this section before starting the build cycle.*

### Decisions that apply

- **DEC-001** — runtime feature set. `tokio::time::timeout` requires
  the `time` feature, already enabled.
- **DEC-002** — reqwest with rustls-tls. `reqwest::Error` wraps IO
  errors; the `#[from] reqwest::Error` mapping on `BackendError::Network`
  handles all network-layer failures including premature EOF.
- **DEC-003** — backend abstraction. Deadlines live in the orchestrator
  (`src/orchestrator.rs`), not inside `throughput.rs` functions — the
  orchestrator is the right place to enforce phase-level policies. The
  backend trait stays unchanged.

### Prior related work

- **SPEC-008** — latency probe uses `tokio::time::timeout(1s, req.send())`
  per sample; `BackendError::Timeout(duration)` already exists and is
  tested at the probe level. SPEC-013 tests the orchestrator's handling
  of fallback and does not re-test SPEC-008's logic.
- **SPEC-010/SPEC-011** — `download_one_non_2xx_returns_protocol_error`
  and `upload_one_non_2xx_returns_protocol_error` exist in
  `tests/throughput.rs`. SPEC-013 adds orchestrator-level versions
  (AC-6/AC-7). Duplication is intentional: different stack levels.
- **SPEC-012** — `orchestrator_latency_failure_returns_test_error_latency`
  tests connection-refused at the orchestrator level. SPEC-013 does
  not duplicate it; it tests different failure modes (stall, truncation).

### Notes for the implementer

#### Why `connections: 1` in failure tests

`download_parallel` with `connections > 1` uses `try_join_all`. If
*any* connection fails, `try_join_all` returns the first error and
cancels the rest. With `download_delay` and `connections: 4`, all 4
connections stall simultaneously and all 4 timeout futures fire at
the deadline. This works correctly — but testing with `connections: 1`
is simpler: exactly one failure, no ambiguity about which connection
errored first.

#### The `latency_rtt_timeout_triggers_tcp_fallback` wall-clock

The HTTP RTT probe sends `DEFAULT_LATENCY_SAMPLES + 1 = 11` requests
(N+1 with first discarded). If `ping_delay = 2s` and timeout = 1s,
the FIRST request (warm-up) times out after 1s. The probe does NOT
retry all 11 — it returns `Err(BackendError::Timeout)` immediately
after the first failure, then the function falls back to TCP. This
means the latency phase takes ~1s, not ~11s. Verified from the
`http_rtt_probe` logic in `src/backend/latency.rs`.

#### `download_delay` in the Axum handler

`tokio::time::sleep(download_delay).await` fires *after* the TCP
connection is accepted and the HTTP request is received, but *before*
the response headers are sent. From reqwest's perspective: `send().await`
blocks until headers arrive. With `download_delay = 2s`, `send().await`
blocks for 2s. Our `tokio::time::timeout(500ms, backend.download(&opts))`
fires after 500ms. The `BackendError::Timeout(500ms)` is constructed
from `self.download_deadline`.

#### `upload_delay` — note on body buffering

Axum with `DefaultBodyLimit::max(64MB)` buffers the request body
before calling the handler. So `upload_delay` fires AFTER the full
upload body is received by Axum (TCP receive completes), but BEFORE
Axum sends the response. From reqwest's `upload_one` perspective:
`client.post(url).body(body).send().await` blocks until the response
headers arrive — which happens after the delay. Our timeout wrapping
`backend.upload(&opts).await` fires correctly.

**Implication:** the upload test does NOT test a stall during body
upload (where TCP send buffers fill up). That scenario requires a
server that stops reading mid-body; out of scope for SPEC-013.

#### `download_truncate_at` — Content-Length mismatch

The handler sends `Content-Length: n` but only streams `truncate_at`
bytes. HTTP/1.1 with explicit Content-Length: hyper tracks expected
body size and raises an error when the connection closes before
that many bytes are read. reqwest surfaces this as a `reqwest::Error`
with an inner IO/decode error. This maps to `BackendError::Network`
via `#[from] reqwest::Error`.

Tested with `truncate_at = 64 * 1024` (one CHUNK_BYTES). The download
request asks for `DEFAULT_DOWNLOAD_BYTES_PER_REQUEST = 1GB` (but the
mock caps at `DOWNLOAD_MAX_BYTES = 1GB` anyway). The mock server
advertises `Content-Length: 1_000_000_000` but streams only 65536
bytes. reqwest detects the mismatch and errors.

**Important:** the test must use `connections: 1` (local `build_config`
default). With multiple connections, each connection independently
truncates, and all N produce `BackendError::Network`. `try_join_all`
returns the first error — still `BackendError::Network`. Works either
way, but `connections: 1` is simpler.

#### Dep audit

No new dependencies. All types used:
- `tokio::time::timeout` — `tokio` `time` feature, already enabled ✓
- `axum::extract::DefaultBodyLimit` — already in dev-deps ✓
- `tokio::time::sleep` — already used in ping_handler ✓
- `stream::iter`, `Bytes`, `StatusCode` — already in `tests/common/mod.rs` ✓

#### `guidance/questions.yaml` update — exact YAML change

Find the `cloudflare-http2-stall-on-parallel-download` entry and:
1. Add two keys after `assigned_to: null`:
   ```yaml
   deferred_to: STAGE-004
   deferral_rationale: "SPEC-013 scope is failure modes (timeout/reset/malformed), not perf-multiplexing investigation; SPEC-013 (STAGE-002) closes without resolving this question; STAGE-004 perf work owns it. The SPEC-013 download-deadline test provides a regression guard: if any download phase stalls indefinitely, CI fails."
   ```
2. Remove the line `blocks: SPEC-013`

---

## Frame Critique (2026-05-03, claude-sonnet-4-6)

**Verdict: ✅ GO** — all six Frame items have clear resolutions.
Scope is tight. The spec stays MockServer-only, adds no new variants,
and closes STAGE-002. The five mechanical items below are
inline-foldable at Build.

### Items (A)–(F)

**(A) Add download/upload deadlines or only test what exists?**

**Decision: ADD deadlines.** Confirmed. Four factors:

1. Without deadlines, `BackendError::Timeout` is unreachable from
   download/upload code paths — a typed variant that exists in the
   taxonomy but is never produced. That's a latent defect, not just
   a test gap.
2. The latency probe's 1s timeout is already symmetrically applied.
   Not extending the pattern to download/upload is an inconsistency
   a future maintainer would rightly question.
3. `with_deadlines` is a consuming builder that chains after
   `with_intervals`, following the established B-1 extension pattern.
   No existing callers change. No trait changes.
4. The 60s production default is generous — it won't affect any real
   test or production run. STAGE-004 may tighten it based on
   measurements.

The counterargument ("timeout path already exists via latency") would
leave download/upload with no timeout protection at all. Not acceptable
given the project's "fast feedback" thesis.

**(B) "Malformed" repurposed to truncation?**

**Confirmed.** The substitution is honest and clearly documented in
both the scope decision section and the `download_truncate_at`
implementation note. The stage doc's "malformed response tests" line
item is satisfied by the only meaningful "malformed" surface that
exists in the current protocol: Content-Length mismatch. Renderers
(STAGE-003) will receive `TestError::Download(BackendError::Network)`,
which they can format as "connection dropped mid-transfer" — exactly
the right user message for this failure mode.

**(C) MockOptions extensions — `download_delay`, `upload_delay`, `download_truncate_at`**

**Confirmed.** Three new `Option<_>` fields, all default `None`. The
`ping_delay` pattern is directly mirrored. Existing test behavior is
unchanged by construction (None = original code path). No public API
changes to `MockServer`'s counter methods.

One mechanical note for Build: the `download` handler currently
constructs `chunk` and stream iterators in one pass. After the change,
it computes `actual_send` first, then constructs the stream from
`actual_send` rather than `n`. The Build cycle should verify this
refactor doesn't change behavior when `download_truncate_at = None`
(i.e., `actual_send = n`, stream identical to before).

**(D) Connection-reset fixture choice — partial-body-then-drop**

**Confirmed.** The Content-Length-mismatch approach is cleaner than
both `panic!()` (non-deterministic TCP RST timing) and an explicit
`Err(...)` stream item (requires boxing and chaining two stream types
in the handler). The current approach: send fewer bytes than
Content-Length, let the stream iterator complete naturally, and let
hyper detect the mismatch. Hyper's Content-Length tracking does
exactly what we need. No `panic!()` in test code per AGENTS.md style.

**(E) Test count and shape — 6 tests in `tests/failure_modes.rs`**

**Confirmed, with one note.** Test 1 (`latency_rtt_timeout_triggers_tcp_fallback`)
is not strictly a "failure mode" test — it ends with `Ok(...)`. But it
belongs in this file because: (a) it is driven by an adversarial
condition (`ping_delay` > timeout), (b) it proves the orchestrator
correctly handles the latency probe's fallback path (a SPEC-008-level
test doesn't cover the orchestrator's `Ok(outcome)` handling after a
fallback), and (c) it provides CI evidence that the HTTP-RTT → TCP
fallback actually works end-to-end. Alternative name
`latency_rtt_timeout_triggers_tcp_fallback` is clearer than
`latency_timeout_surfaces_test_error_latency` (which would be
misleading since the test returns `Ok`). Use the name as written.

**(F) Stage-closing posture**

**Confirmed.** `## What this spec does NOT do` explicitly calls out
STAGE-002 ship as a separate cycle anticipated immediately after
SPEC-013 ships. The Ship cycle reflection should:
1. Fill in the SPEC-013 Ship reflection (three questions)
2. Draft the STAGE-002 stage-level reflection in a separate section
   (or as a note): what the stage delivered, what changed from the
   stage plan, what STAGE-003 inherits
3. Run the Stage Ship prompt (`just status` to confirm backlog clear,
   then trigger Stage Ship per AGENTS.md §15)

### Mechanical notes for Build

1. **`upload_delay` timing.** The upload handler should apply
   `upload_delay` AFTER `fetch_add(1, ...)` (so the counter increments
   even on delayed responses) and BEFORE the status check. This
   matches the natural read order and ensures the counter is always
   accurate regardless of delay.

2. **`with_deadlines` must be pub.** The method is used in
   `tests/failure_modes.rs` which is integration test code accessing
   the public API surface. Ensure `pub fn with_deadlines` is in the
   `impl TestSession` block in `src/orchestrator.rs`.

3. **Re-export new constants.** `DEFAULT_DOWNLOAD_DEADLINE` and
   `DEFAULT_UPLOAD_DEADLINE` need to be added to `lib.rs`'s
   `pub use orchestrator::{...}` block so test code can reference them
   as `rspeed::DEFAULT_DOWNLOAD_DEADLINE` without reaching into the
   module.

4. **`#![allow]` in `tests/failure_modes.rs`.** Add `#![allow(clippy::
   unwrap_used, clippy::expect_used, clippy::panic)]` at the top per
   project convention (AGENTS.md testing discipline section).

5. **`#![allow(dead_code)]` in `tests/common/mod.rs`.** The existing
   file already has `#![allow(..., dead_code)]` — verify the new
   fields and handler code don't trigger clippy warnings under
   `--all-targets`.

### Scope check

The stage doc estimates SPEC-013 at 3 hours. This spec frontmatter
records `complexity: S`. Honest breakdown:

- `MockOptions` extensions (3 fields + handler changes): ~30min
- `TestSession::with_deadlines` + deadline fields + `with_intervals`
  update: ~20min
- Deadline wrapping in `run_download_phase`/`run_upload_phase`: ~15min
- `lib.rs` re-exports: ~5min
- 6 failing tests in `tests/failure_modes.rs`: ~45min
- `guidance/questions.yaml` update: ~5min
- Clippy/fmt/CI iteration: ~30min
- Reflection + spec frontmatter completion: ~20min

Total: ~2h50m. Fits comfortably in the 3hr estimate. No split needed.

---

## Build Completion

- **Branch:** `feat/spec-013-failure-modes`
- **PR (if applicable):** #18
- **All acceptance criteria met?** Yes (AC-1 through AC-13)
- **Test count:** 72 prior + 6 new = 78 total tests passing
- **New decisions emitted:** None — all build choices aligned with existing DEC-001/002/003
- **Deviations from spec:** One intentional:
  - The spec's "Files to modify" listed `src/backend/throughput.rs` for constants and
    timeout wrapping. The Implementation Context's DEC-003 and the spec's code examples
    both place these in `src/orchestrator.rs` instead. The orchestrator approach was
    followed: constants in `orchestrator.rs`, timeouts wrap `self.backend.download/upload()`
    at the orchestrator level. This satisfies architect refinement #2 (per-phase not
    per-connection) because `backend.download()` internally calls `try_join_all` on all N
    connections. `throughput.rs` was not modified.
  - `guidance/questions.yaml` was already updated during design (deferred_to +
    deferral_rationale present, blocks: SPEC-013 absent). No change needed at build.

---

## Verification Results

**Date:** 2026-05-03  
**Agent:** claude-sonnet-4-6  
**Branch:** `feat/spec-013-failure-modes`  
**PR:** #18

### Architectural deviation — ratified

The Build placed `DEFAULT_DOWNLOAD_DEADLINE` / `DEFAULT_UPLOAD_DEADLINE` constants and
`tokio::time::timeout` wrapping in `src/orchestrator.rs`, not `src/backend/throughput.rs`.
This is the correct placement. `git diff main -- src/backend/throughput.rs` returns empty —
`throughput.rs` is untouched. The orchestrator placement enforces the deadline at the
trait-call boundary (`self.backend.download/upload`), meaning any future `Backend`
implementation gets the deadline for free. Placing it inside `throughput.rs` would have
left a hypothetical second backend unprotected. Rationale is sound; deviation is the
better architecture.

### `with_deadlines` builder

Signature confirmed: `pub fn with_deadlines(mut self, download_deadline: Duration, upload_deadline: Duration) -> Self`.
Both `with_intervals` and `with_deadlines` consume `self` and return `Self` — chaining is
`TestSession::with_intervals(...).with_deadlines(...)`. Verified by reading: `with_deadlines`
only modifies `download_deadline` / `upload_deadline`; `snapshot_interval` and `warmup` are
untouched. The two timeout tests chain both builders and pass — structurally confirms
coexistence. `TestSession::new()` delegates to `with_intervals`, which initializes both
deadline fields to the constants. ✅

### Six failure-mode tests — all pass (1.02s total)

- **`download_timeout_surfaces_test_error_download`**: `Err(TestError::Download(BackendError::Timeout(_)))`. 500ms deadline, 2s server delay. ✅
- **`upload_timeout_surfaces_test_error_upload`**: `Err(TestError::Upload(BackendError::Timeout(_)))`. 500ms deadline, 2s upload delay. ✅
- **`download_mid_stream_truncation_surfaces_network_error`**: `Err(TestError::Download(BackendError::Network(_)))`. No flakiness risk — deadline is DEFAULT_DOWNLOAD_DEADLINE (60s), truncation surfaces in milliseconds. ✅
- **`download_non_2xx_via_orchestrator`**: Drives `TestSession::run()`, not `download_one()`. `Err(TestError::Download(BackendError::Protocol(_)))`. ✅
- **`upload_non_2xx_via_orchestrator`**: Same orchestrator-level discipline. `Err(TestError::Upload(BackendError::Protocol(_)))`. ✅
- **`latency_rtt_timeout_triggers_tcp_fallback`**: Asserts `result.latency.method == "tcp_connect"` AND `result.latency.samples > 0`. The `method` assertion would catch a silently broken fallback. Returns `Ok`, finishes in ~1.0s. ✅

### MockServer extensions

Three new `Option<_>` fields all default to `None` — pre-SPEC-013 behavior preserved.
`download_truncate_at` handler correctly advertises `Content-Length: n` (full size)
while streaming only `actual_send` bytes — confirmed by reading: `header::CONTENT_LENGTH, n`
with `Body::from_stream(chunks)` built from `actual_send`. `download_delay` and
`upload_delay` use `tokio::time::sleep`. Upload counter increments before the delay
(mechanical note F-1 honored). ✅

### Regression net

All prior tests pass without modification. `git diff main -- 'tests/*'` shows changes
only to `tests/common/mod.rs` (additive) and new `tests/failure_modes.rs`. Test count:
78 total across all targets (72 prior + 6 new; the Build Completion's "78 prior" is a
documentation off-by-six — the actual prior count is 72). All 78 pass. ✅

### `guidance/questions.yaml` deferral

`cloudflare-http2-stall-on-parallel-download` carries `deferred_to: STAGE-004`,
`deferral_rationale` present, no `blocks: SPEC-013` line. `just status` parses cleanly
with no errors. ✅

### Lints / build

- `cargo clippy --all-targets -- -D warnings` ✅ clean
- `cargo fmt --check` ✅ clean
- `cargo build --release` ✅ succeeds (7.59s)
- CI: macOS arm64 ✅ / Linux x86_64 ✅ / Windows x86_64 ✅

### Stage-closing readiness check

All six prior STAGE-002 specs' invariants are intact:
- **Invariant #1** (MetricsAccumulator decoupled from rendering): SPEC-007 delivered; the
  accumulator emits `Snapshot` on a watch channel with no subscriber coupling. Unchanged.
- **Invariant #2** (orchestrator invocation-agnostic): SPEC-012 delivered `TestSession::run()`;
  SPEC-013 adds deadline fields without altering the trait seam. Unchanged.
- **Invariant #3** (typed failure modes return structured errors): SPEC-013 closes the gap.
  Pre-SPEC-013, `BackendError::Timeout` was unreachable from download/upload paths — the
  variant existed in the taxonomy but was never produced. SPEC-013 proves all six adversarial
  paths return the correct typed variant: `Timeout` for stalls, `Network` for mid-stream
  truncation, `Protocol` for non-2xx, all properly tagged with `TestError::Download` or
  `TestError::Upload`. Every external-network-induced failure is now a typed variant of
  `TestError`. Invariant #3 holds end-to-end. ✅

Nothing looks half-shipped at the stage level. STAGE-002 is ready to close.

---

✅ **APPROVED** — STAGE-002 is one Ship away from closing.

---

## Reflection (Ship)

### 1. What went well or was easier than expected?

The architectural deviation proved immediately sound in Verify: placing deadlines in
`orchestrator.rs` rather than `throughput.rs` means any future `Backend` implementation
inherits timeout protection at the trait boundary for free — a point the Verify rationale
surfaced clearly. The chainable `with_intervals(...).with_deadlines(...)` builder also came
together cleanly; following the established B-1 pattern meant zero existing callers changed
and the test DSL reads clearly. The Content-Length mismatch approach for mid-stream truncation
was the right tool — no `panic!()`, no boxing two stream types, just hyper's built-in body-size
tracking surfacing the premature EOF as a `reqwest::Error`.

### 2. What was harder, surprising, or required correction?

The spec's "Files to modify" listed `src/backend/throughput.rs` for the deadline constants
and wrapping, but the Implementation Context's DEC-003 note and all code examples pointed to
`orchestrator.rs`. The Build had to reconcile the file list against the spec's own rationale
and correctly chose the orchestrator — this is the second time a Build deviation beat the
prescriptive file list (SPEC-012 had a similar moment with `lib::run()`). The PR description
also carried a test-count error ("84 tests total (78 prior + 6 new)") that conflicted with
the actual 78 total (72 prior + 6 new); caught in Verify and fixed in this Ship commit.

### 3. What should STAGE-003 know?

The `TestSession` snapshot fan-out (DEC-008 seam #1) is accessed via `session.snapshot_rx()`
— the watch receiver STAGE-003's human renderer subscribes to for live progress. The
`Format::Human` branch in `lib::run()` currently falls through to JSON (SPEC-012 interim
stub); STAGE-003 fills this in. Error rendering must distinguish all five `TestError`
variants (`Network`, `Timeout`, `Protocol`, `Backend`, `Config`) — each needs a distinct
user-facing message so operators can differentiate a stalled connection from a misconfigured
URL from a server-side 5xx. The HTTP/2 stall question is deferred to STAGE-004; do not let
it bleed into STAGE-003 scope. The MockServer extensions accumulated across STAGE-002
(delays, truncation, status overrides, 64MB body limit) are stable infrastructure STAGE-003
inherits.
