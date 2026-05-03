---
task:
  id: SPEC-010
  type: story
  cycle: ship
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
  implementer: null
  created_at: 2026-05-02

references:
  decisions: [DEC-002, DEC-003, DEC-005]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-005, SPEC-006, SPEC-008, SPEC-009, SPEC-011]

value_link: "delivers download/upload throughput against Cloudflare — the second-and-third measurement phases of TestResult, after latency"

cost:
  sessions:
    - cycle: frame
      date: 2026-05-02
      agent: claude-opus-4-7
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: ""
    - cycle: design
      date: 2026-05-02
      agent: claude-opus-4-7
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Spec authoring + Frame critique in single Opus session (per SPEC-007/008/009 precedent)"
    - cycle: build
      date: 2026-05-02
      agent: null
      interface: claude-code
      tokens_input: 7415263
      tokens_output: 130133
      estimated_usd: 6.1813
      note: ""
    - cycle: verify
      date: 2026-05-02
      agent: null
      interface: claude-code
      tokens_input: 1091331
      tokens_output: 8499
      estimated_usd: 1.7029
      note: ""
    - cycle: ship
      date: 2026-05-02
      agent: null
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: ""
  totals:
    tokens_total: 8645226
    estimated_usd: 7.8842
    session_count: 5
---

# SPEC-010: Cloudflare backend — real download/upload

## Context

Third measurement spec under STAGE-002. SPEC-007 shipped the type
layer, SPEC-008 shipped the latency probe (the first real network
code), SPEC-009 shipped the buffer pool. SPEC-010 lands the second and
third real-network phases: download throughput and upload throughput
against Cloudflare's `/__down` and `/__up` endpoints.

Both download and upload are conceptually similar (issue parallel HTTP
requests, measure bytes/time) but differ in direction (server→client
vs client→server) and shape (download is a stream, upload is a
request-response). The mechanics are identical between Cloudflare and
the Generic backend (SPEC-011) — only the URLs differ. SPEC-010
extracts the shared HTTP mechanics into `src/backend/throughput.rs`
(matching the SPEC-008 pattern for `src/backend/latency.rs`), wires
`CloudflareBackend`, and leaves SPEC-011 to wire `GenericHttpBackend`
against the same shared module.

DEC-002 governs the HTTP client (`reqwest` with `rustls`, `no_proxy`,
`Accept-Encoding: identity`). DEC-003 specifies Cloudflare's endpoint
shape (`/__down?bytes=N`, `/__up`). DEC-005 specifies the buffer
strategy — see **Implementation Context** for the honest accounting of
where the SPEC-009 buffer pool fits and where it doesn't in the
reqwest-streaming path.

## Goal

Implement `Backend::download()` and `Backend::upload()` for
`CloudflareBackend`, both delegating to a new shared private module
`src/backend/throughput.rs`. Wire `connections > 1` parallel HTTP
requests via `futures::future::try_join_all` (start) and
`futures::stream::select_all` (merge for download). Return a
`DownloadStream` that fans bytes from N concurrent connections into
one merged stream; return an `UploadResult` whose `bytes_sent` and
`elapsed` reflect the parallel upload as a whole.

No `Backend` trait signature changes — the shape stabilized at SPEC-008.

## Inputs

- **`decisions/DEC-002-http-client.md`** — `reqwest` config:
  `default-features = false`, `["rustls", "stream", "http2"]`;
  `.no_proxy()`; `Accept-Encoding: identity` on all requests
- **`decisions/DEC-003-backend-abstraction.md`** — Cloudflare's
  endpoint contract: `GET /__down?bytes=N` returns N bytes;
  `POST /__up` consumes body, returns 200; protocol commitment
  semantics
- **`decisions/DEC-005-buffer-strategy.md`** — buffer pool sized for
  4 parallel connections; upload uses pre-allocated `Bytes` of zeros
  cloned per request (reference-counted, O(1))
- **`src/backend/mod.rs`** — `Backend` trait, `DownloadOpts`,
  `UploadOpts`, `UploadResult`, `DownloadStream`, `BackendError`
- **`src/backend/cloudflare.rs`** — current state: `latency_probe`
  implemented, `download`/`upload` return `NotImplemented`. This spec
  fills both
- **`src/backend/latency.rs`** — established pattern for shared
  backend logic in a `pub(crate)` module called by both backends
- **`src/buffer_pool.rs`** — SPEC-009's `BufferPool`. See
  **Implementation Context — Buffer pool accounting** for the honest
  story on why this spec doesn't directly use it
- **`tests/common/mod.rs`** — `MockServer` with `/download?bytes=N`
  and `/upload` routes already implemented per SPEC-006; SPEC-010
  extends `MockOptions` with `download_status` and `upload_status`
  fields for non-2xx error tests

## Outputs

- **Files created:**
  - `src/backend/throughput.rs` — `pub(crate)` module containing
    `download_one(client, url) -> impl Stream<Item = Result<Bytes, BackendError>>`
    and `upload_one(client, url, body) -> Result<Duration, BackendError>`.
    Visibility per **AC-3** below — `pub mod` from `backend/mod.rs` so
    integration tests can import directly, mirroring SPEC-009's
    visibility pattern.
  - `tests/throughput.rs` — integration tests (see **Failing Tests**)

- **Files modified:**
  - `src/backend/mod.rs`:
    - declare `pub mod throughput;` (visible to integration tests, not
      re-exported from `lib.rs`)
  - `src/backend/cloudflare.rs`:
    - add fields: `download_base_url: Url`, `upload_url: Url`
    - construct both in `new()` from hardcoded Cloudflare URLs
    - implement `download(&self, opts)` by spawning N
      `throughput::download_one()` futures via `try_join_all`,
      collecting the streams, merging with `select_all`
    - implement `upload(&self, opts)` by allocating one `Bytes` of
      zeros sized to `opts.bytes_per_request`, cloning it for each of
      N parallel `throughput::upload_one()` futures, and returning
      `UploadResult { bytes_sent: bytes_per_request * connections,
      elapsed: total_wall_clock }`
  - `tests/common/mod.rs`:
    - extend `MockOptions` with `download_status: StatusCode` (default
      200) and `upload_status: StatusCode` (default 200)
    - extend `AppState` and the `download`/`upload` handlers to honor
      the configured status codes (still serve the body on 200, return
      empty body on non-2xx)

- **`Cargo.toml`:** **no new top-level deps.**
  `futures::stream::select_all` and `futures::future::try_join_all`
  are both in `futures = "0.3"`, already a dep. `reqwest::Response::
  bytes_stream()` is available behind the `stream` feature, already
  enabled. `tokio::time::Instant` is already in the `time` feature.
  Pre-allocating `Bytes::from(vec![0u8; n])` uses `bytes` already in
  the dep graph.

- **No `lib.rs` re-exports added** — `throughput` is internal
  infrastructure, like `latency` and `buffer_pool`.

## Acceptance Criteria

- [ ] **AC-1: `throughput::download_one` issues a streaming GET.**
  Signature: `pub(crate) async fn download_one(client: &reqwest::
  Client, url: Url) -> Result<impl futures::Stream<Item = Result<Bytes,
  BackendError>> + Send + 'static, BackendError>`. Sends `GET url` with
  header `Accept-Encoding: identity`. On 2xx, returns the response's
  `bytes_stream()` with errors mapped from `reqwest::Error` to
  `BackendError::Network`. On non-2xx, returns `Err(BackendError::
  Protocol(format!("download returned HTTP {}", status)))`. On transport
  failure (connection refused, DNS, TLS), returns `Err(BackendError::
  Network(_))`.

- [ ] **AC-2: `throughput::upload_one` POSTs a `Bytes` body and
  measures elapsed time.** Signature: `pub(crate) async fn upload_one
  (client: &reqwest::Client, url: Url, body: Bytes) -> Result<Duration,
  BackendError>`. Records `Instant::now()` immediately before
  `.send().await`; computes elapsed immediately after the await
  resolves (and before the status check, so timeout/error cases still
  produce a meaningful elapsed if needed downstream — but currently we
  only return elapsed on success). Sets `Accept-Encoding: identity` and
  `Content-Length` headers. On non-2xx, returns `Err(BackendError::
  Protocol(...))`. On transport failure, returns `Err(BackendError::
  Network(_))`.

- [ ] **AC-3: `throughput` module visible to integration tests.**
  `src/backend/mod.rs` declares `pub mod throughput;` (not
  `pub(crate)`). Functions `download_one` and `upload_one` are `pub`.
  `src/lib.rs` does NOT re-export `throughput` from its top-level
  `pub use` block. Integration tests access via `rspeed::backend::
  throughput::download_one`. Same pattern as SPEC-009's
  `rspeed::buffer_pool::BufferPool`.

- [ ] **AC-4: `CloudflareBackend::download()` runs N parallel
  connections.** With `opts.connections == n`, the implementation
  starts `n` `download_one()` futures concurrently via
  `futures::future::try_join_all` (so request-establishment overlaps,
  not sequential), then merges their result streams via
  `futures::stream::select_all`. The returned `DownloadStream` yields
  bytes from whichever connection has data ready. If any of the N
  initial requests fails to establish, the entire `download()` call
  returns `Err(_)` — partial success is not exposed at the trait
  boundary.

- [ ] **AC-5: `CloudflareBackend::upload()` runs N parallel
  connections.** With `opts.connections == n`, `upload()` allocates
  one `Bytes` of `opts.bytes_per_request` zero bytes (single
  allocation per `upload()` call, regardless of `n`), `clone()`s it
  per connection (O(1) reference-count bump), and runs `n`
  `upload_one()` futures via `futures::future::try_join_all`. Wall-
  clock `elapsed` is measured from immediately before `try_join_all`
  begins to immediately after it resolves successfully. `bytes_sent`
  is `opts.bytes_per_request * (opts.connections as u64)`. If any
  upload fails, the call returns `Err(_)`.

- [ ] **AC-6: Cloudflare URLs are constructed correctly.**
  `CloudflareBackend::new()` parses `"https://speed.cloudflare.com/
  __down"` into `download_base_url` and `"https://speed.cloudflare.
  com/__up"` into `upload_url`. The download URL per request is
  `{base}?bytes={n}` constructed via `Url::query_pairs_mut().
  append_pair("bytes", &n.to_string())`. URL parsing failures map to
  `BackendError::Protocol(parse_err.to_string())` (the existing
  pattern in `new()` for `ping_url`).

- [ ] **AC-7: `MockOptions` extended for non-2xx tests.** Adds
  `download_status: StatusCode` (default 200) and `upload_status:
  StatusCode` (default 200). Default behavior matches SPEC-006 exactly
  — existing `smoke.rs` and `latency.rs` tests continue to pass with
  `MockOptions::default()`. Handlers consult the state and return
  empty body on non-2xx (no need to stream zero bytes for an error
  response).

- [ ] **AC-8: `Accept-Encoding: identity` set on all requests.**
  Both `download_one` and `upload_one` set `Accept-Encoding:
  identity` explicitly. Verified by code inspection: `grep -n
  'Accept-Encoding' src/backend/throughput.rs` returns two matches,
  one in `download_one`, one in `upload_one`. The header-recording
  MockServer extension was judged too invasive for one assertion
  (Frame Item C).

- [ ] **AC-14: `connections == 0` returns a Protocol error.** Both
  `CloudflareBackend::download()` and `::upload()` return
  `Err(BackendError::Protocol(_))` immediately when
  `opts.connections == 0`, before any network call. Covered by
  `connections_zero_returns_error` in `tests/throughput.rs`
  (Frame Item F).

- [ ] **AC-9: All tests pass on all three primary CI runners.**
  macOS arm64 (`macos-15`), Linux x86_64 (`ubuntu-24.04`), Windows
  x86_64 (`windows-2025`). The `Cross-check x86_64-apple-darwin`
  step on macos-15 still succeeds.

- [ ] **AC-10: No new top-level deps.** `Cargo.toml` `[dependencies]`
  unchanged. `[dev-dependencies]` unchanged.

- [ ] **AC-11: Lib-side `unwrap`/`expect`/`panic` discipline
  preserved.** `src/backend/throughput.rs` and the new code in
  `src/backend/cloudflare.rs` contain no `unwrap()`, `expect()`, or
  `panic!()`. All fallible operations use `?` to propagate
  `BackendError`. `tests/throughput.rs` carries
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` per project
  test convention.

- [ ] **AC-12: `cargo clippy --all-targets -- -D warnings` and `cargo
  fmt --check` pass.**

- [ ] **AC-13: Existing tests continue to pass.** All seven
  `tests/buffer_pool.rs` tests, all `tests/latency.rs` tests, all
  `tests/smoke.rs` tests, and `tests/cli.rs` / `tests/version.rs` /
  `tests/metrics.rs` continue to pass without modification.

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/throughput.rs` unless noted. The file opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use bytes::Bytes;
use futures::StreamExt;
use rspeed::backend::throughput::{download_one, upload_one};
```

---

**`"download_one_happy_path_against_mock"`** — `#[tokio::test]`.
Starts MockServer (default opts). Constructs a `reqwest::Client` via
`Client::builder().no_proxy().build()?`. Builds a download URL
`{mock.base_url()}download?bytes=1048576` (1MB). Calls `download_one
(&client, url).await?`. Collects the stream into a `Vec<Bytes>` via
`while let Some(chunk) = stream.next().await { chunks.push(chunk?) }`.
Asserts:
- All chunks are `Ok(_)`
- `chunks.iter().map(|b| b.len()).sum::<usize>() == 1_048_576`

**`"download_one_non_2xx_returns_protocol_error"`** —
`#[tokio::test]`. Starts MockServer with `MockOptions {
download_status: StatusCode::INTERNAL_SERVER_ERROR, ..Default::default()
}`. Calls `download_one()`. Asserts the result is
`Err(BackendError::Protocol(msg))` where `msg.contains("500")`.

**`"download_one_connection_refused_returns_network_error"`** —
`#[tokio::test]`. Builds a URL pointing to `127.0.0.1:1` (port 1, very
unlikely to have a listener). Calls `download_one()`. Asserts the
result is `Err(BackendError::Network(_))`. Note: this test must not
hang — `reqwest`'s connection-refused path is fast (kernel-level
RST), so no timeout is needed at the test level.

**`"upload_one_happy_path_against_mock"`** — `#[tokio::test]`.
Starts MockServer (default opts). Builds a `Bytes` of 64KB zeros.
Calls `upload_one(&client, mock_upload_url, body).await?`. Asserts
the returned `Duration` is non-zero.

**`"upload_one_non_2xx_returns_protocol_error"`** —
`#[tokio::test]`. Starts MockServer with `upload_status:
StatusCode::INTERNAL_SERVER_ERROR`. Calls `upload_one()` with a 1KB
body. Asserts `Err(BackendError::Protocol(msg))` where
`msg.contains("500")`.

**`"parallel_downloads_via_select_all"`** — `#[tokio::test]`.
Starts MockServer. Spawns 4 `download_one()` futures via
`futures::future::try_join_all`, each requesting 256KB. Merges the
4 streams via `futures::stream::select_all`. Drains the merged
stream. Asserts total bytes received == `4 * 256 * 1024` and
`mock.download_count() == 4`. (`download_count` is a new counter
on MockServer, mirroring `ping_count`; needed for this and the
upload-parallel test.)

**`"parallel_uploads_via_try_join_all"`** — `#[tokio::test]`.
Starts MockServer. Builds a 16KB `Bytes` of zeros. Spawns 4
`upload_one()` futures via `try_join_all`, each posting `body.
clone()`. Awaits the result. Asserts all 4 succeed, all return
non-zero durations, and `mock.upload_count() == 4`.

**`"download_one_serializes_with_metrics_accumulator"`** —
`#[tokio::test]`. Integration smoke test combining SPEC-007 +
SPEC-010: starts MockServer, downloads 256KB via `download_one()`,
records each chunk's length into a `MetricsAccumulator` via
`record_bytes(chunk.len() as u64)`, calls `acc.finish()`, asserts the
returned `ThroughputResult.bytes` equals 256KB (or close — accumulator
counts post-warmup bytes; for this test set `warmup =
Duration::ZERO` so all bytes count).

---

The `tests/common/mod.rs` extension that this spec requires:

- Add `download_status: StatusCode` and `upload_status: StatusCode` to
  `MockOptions` (default `StatusCode::OK` for both).
- Add `download_counter: Arc<AtomicU64>` and `upload_counter:
  Arc<AtomicU64>` to `AppState` and `MockServer`. Increment in
  handlers. Expose `MockServer::download_count() -> u64` and
  `upload_count() -> u64`.
- Conditionally add `recorded_request_headers: Arc<Mutex<Vec<
  HeaderMap>>>` for the `Accept-Encoding` test (or skip per the
  resolution-alternative noted above).
- All extensions are additive to `MockOptions::default()`; existing
  test files compile and pass without modification.

## Implementation Context

### Module structure: `src/backend/throughput.rs`

```rust
//! Shared download/upload HTTP mechanics. Called by both
//! CloudflareBackend and (in SPEC-011) GenericHttpBackend.

use bytes::Bytes;
use futures::StreamExt;
use reqwest::Client;
use std::time::{Duration, Instant};
use url::Url;

use super::BackendError;

pub async fn download_one(
    client: &Client,
    url: Url,
) -> Result<impl futures::Stream<Item = Result<Bytes, BackendError>> + Send + 'static, BackendError>
{
    let response = client
        .get(url)
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(BackendError::Network)?;

    let status = response.status();
    if !status.is_success() {
        return Err(BackendError::Protocol(format!(
            "download returned HTTP {}",
            status.as_u16()
        )));
    }

    Ok(response.bytes_stream().map(|r| r.map_err(BackendError::Network)))
}

pub async fn upload_one(
    client: &Client,
    url: Url,
    body: Bytes,
) -> Result<Duration, BackendError> {
    let bytes_len = body.len();
    let start = Instant::now();

    let response = client
        .post(url)
        .header("Accept-Encoding", "identity")
        .header("Content-Length", bytes_len)
        .body(body)
        .send()
        .await
        .map_err(BackendError::Network)?;

    let elapsed = start.elapsed();

    let status = response.status();
    if !status.is_success() {
        return Err(BackendError::Protocol(format!(
            "upload returned HTTP {}",
            status.as_u16()
        )));
    }

    Ok(elapsed)
}
```

Note the `+ Send + 'static` bounds on `download_one`'s returned
stream — required so `select_all` can merge it and so the returned
`DownloadStream = BoxStream<'static, ...>` is buildable.

### `CloudflareBackend::download` implementation

```rust
async fn download(&self, opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = self.client.clone();
            let url_result = Self::build_download_url(&self.download_base_url, bytes_per);
            async move {
                let url = url_result?;
                throughput::download_one(&client, url).await
            }
        })
        .collect();

    let streams = futures::future::try_join_all(futures_list).await?;

    let pinned: Vec<_> = streams
        .into_iter()
        .map(|s| Box::pin(s) as futures::stream::BoxStream<'static, _>)
        .collect();

    Ok(Box::pin(futures::stream::select_all(pinned)))
}

fn build_download_url(base: &Url, bytes: u64) -> Result<Url, BackendError> {
    let mut url = base.clone();
    url.query_pairs_mut().append_pair("bytes", &bytes.to_string());
    Ok(url)
}
```

`build_download_url` is fallible-typed for symmetry with the rest of
the codebase even though `query_pairs_mut` itself doesn't fail; this
keeps the downstream `?` chain uniform and lets us add fallible
operations later without a signature change.

### `CloudflareBackend::upload` implementation

```rust
async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    // DEC-005: one allocation, cloned per connection (Bytes is refcounted).
    let body = Bytes::from(vec![0u8; bytes_per as usize]);

    let start = Instant::now();

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = self.client.clone();
            let url = self.upload_url.clone();
            let body = body.clone();
            async move { throughput::upload_one(&client, url, body).await }
        })
        .collect();

    futures::future::try_join_all(futures_list).await?;

    let elapsed = start.elapsed();

    Ok(UploadResult::new(bytes_per * (n as u64), elapsed))
}
```

### Buffer pool accounting (the honest story)

DEC-005 specifies a `BytesMut` pool for "after warm-up, no allocations
in the read loop." SPEC-009 implemented that pool. SPEC-010 does NOT
directly use the pool because:

1. **The download path uses `reqwest::Response::bytes_stream()`,
   which yields `Bytes` chunks already owned by reqwest/hyper.**
   Inserting our pool here would require either copying bytes from
   reqwest's chunk into a `PooledBuffer` (wasted work) or replacing
   reqwest's high-level streaming with a low-level `tokio::io::
   AsyncRead` interface that the pool's `read_buf()` pattern fits.
   The latter is a significant refactor not warranted at STAGE-002,
   where the success criterion is "produces working measurements,"
   not "hits the perf budget" (that's STAGE-004).

2. **The upload path uses a single pre-allocated `Bytes` of zeros,
   cloned per request — exactly what DEC-005 specifies for uploads.**
   No `BytesMut` pool involvement; that pool was for the download
   read loop. The "one allocation per `upload()` call" cost is amortized
   across N parallel requests via `Bytes::clone`'s O(1) refcount bump.

3. **STAGE-004 is where the buffer pool earns its keep.** When we
   profile against the 1 Gbps budget, if `bytes_stream()`-driven
   chunk granularity (typically small — 8KB to 64KB depending on
   reqwest internals) causes the `MetricsAccumulator` to update too
   often or shows noticeable allocation pressure, STAGE-004 either
   wraps reqwest's stream with a pool-backed re-chunker, or drops
   to `hyper` directly to use `read_buf()`. Either change is
   contained to `src/backend/throughput.rs` because the trait
   surface (`DownloadStream = BoxStream<Bytes>`) hides the
   implementation choice.

This is documented honestly so SPEC-011 doesn't re-litigate the
question and STAGE-004 has a clear baseline to optimize from.

### `MockOptions` extension shape

```rust
// in tests/common/mod.rs
#[derive(Clone)]
pub struct MockOptions {
    pub ping_status: StatusCode,
    pub ping_delay: Option<Duration>,
    pub download_status: StatusCode,
    pub upload_status: StatusCode,
}

impl Default for MockOptions {
    fn default() -> Self {
        Self {
            ping_status: StatusCode::OK,
            ping_delay: None,
            download_status: StatusCode::OK,
            upload_status: StatusCode::OK,
        }
    }
}
```

`AppState` gains the matching fields. The `/download` handler:

```rust
async fn download(
    State(state): State<AppState>,
    Query(q): Query<DownloadQuery>,
) -> Response {
    state.download_counter.fetch_add(1, Ordering::Relaxed);
    if !state.download_status.is_success() {
        return Response::builder()
            .status(state.download_status)
            .body(Body::empty())
            .unwrap();
    }
    // existing happy-path body
}
```

The `/upload` handler analogously checks `state.upload_status`.

### Visibility recap

| Item | Where declared | Visible to |
|---|---|---|
| `mod throughput` | `src/backend/mod.rs` | `pub mod` — visible to integration tests |
| `download_one`, `upload_one` | `src/backend/throughput.rs` | `pub` |
| Re-export from `src/lib.rs` | none | not part of documented public API |

Same pattern as SPEC-009's `buffer_pool` and arguably better than
SPEC-008's `pub(crate) mod latency` (which is unreachable from
integration tests, working around it by testing through the
`Backend` trait against `GenericHttpBackend`). For SPEC-010 the
trait-test path isn't viable yet (CloudflareBackend's URLs are
hardcoded; GenericHttpBackend's download/upload aren't implemented
until SPEC-011), so direct `throughput` access from integration
tests is the cleanest seam.

### What this spec does NOT do

- Wire `GenericHttpBackend::download` / `::upload` — that's SPEC-011
  (one-spec scope). SPEC-011 will have a near-trivial Build because
  `throughput.rs` is already done.
- Implement the test orchestrator — that's SPEC-012, which will
  consume `Backend::download()` + `MetricsAccumulator` to populate
  `TestResult::download` and `TestResult::upload`.
- Live tests against actual Cloudflare endpoints — those are gated
  behind the `live` cargo feature (deferred to SPEC-013 per stage
  plan).
- Hit the 1 Gbps throughput budget — that's STAGE-004.
- Use the SPEC-009 buffer pool — see **Buffer pool accounting** above.

## Build Completion

**Date:** 2026-05-02
**Agent:** claude-sonnet-4-6

### Frame resolutions applied

| Item | Resolution |
|---|---|
| (A) `try_join_all` + HTTP/2 stall | `parallel_downloads_via_select_all` verified against MockServer (HTTP/1.1); HTTP/2 risk noted for SPEC-013 |
| (B) Large upload body RSS | DEC-005 Consequences amended with STAGE-004 follow-up note |
| (C) Accept-Encoding test | Dropped; AC-8 is code-review verification |
| (D) download/upload counters | Implemented as designed |
| (E) `select_all` semantics | Confirmed correct, no action |
| (F) `connections == 0` guard | Added early-return `Err(Protocol)` in both `download()` and `upload()`; covered by `connections_zero_returns_error` test |
| (G) Dep constraint | Confirmed clean, no new deps |
| (H) Trait unchanged | Confirmed |

### Implementation notes

- Rust 2024's RPIT lifetime capture rules required `+ use<>` on the
  `download_one` return type to prevent the `&Client` borrow from being
  captured in the `'static` stream. The stream itself doesn't borrow from
  the client (it takes ownership of `response`), so `use<>` is correct.
- `#![allow(clippy::panic)]` added to `tests/throughput.rs` alongside the
  existing `unwrap_used` / `expect_used` allowances, because match-arm
  `panic!()` calls are the idiomatic test failure path when `Result<impl
  Stream, ...>` doesn't implement `Debug`.
- `download_one_serializes_with_metrics_accumulator` uses
  `#[tokio::test(start_paused = true)]` to deterministically fire warmup
  and sample ticks without real-time sleeps; real localhost I/O works
  fine with the paused clock since reqwest has no connect timeout set.

### Reflection

Spec-to-code fidelity was high — the Implementation Context was accurate
enough to write the code directly. The two surprises: Rust 2024 RPIT
capture rules (resolved with `use<>`), and the Debug bound gap on
`Result<impl Stream, ...>` forcing explicit Err/Ok split in match arms.
Both are minor. Frame critique quality was excellent: items (A), (B), (F)
were real issues; all three resolved cleanly in Build.

## Verification Results

**Date:** 2026-05-02
**Agent:** claude-sonnet-4-6
**Verdict:** ✅ **APPROVED**

### ACs

| AC | Status | Notes |
|---|---|---|
| AC-1 `download_one` signature/headers/error mapping | ✅ | `throughput.rs:9-34`; `Accept-Encoding: identity` at line 18; `BackendError::Network` and `Protocol` mapped correctly |
| AC-2 `upload_one` timing/headers/error mapping | ✅ | `Instant::now()` before `.send().await` (line 38); `elapsed` captured immediately after await (line 49), before status check; both headers set |
| AC-3 `pub mod throughput` in `backend/mod.rs`; `pub fn`s; no re-export from `lib.rs` | ✅ | `mod.rs:10` is `pub mod throughput;`; both functions are `pub`; `lib.rs` `pub use` block has no `throughput` |
| AC-4 `download()` uses `try_join_all` then `select_all` | ✅ | `cloudflare.rs:91-98` — `try_join_all` to establish, `select_all` to merge; `Err` on any failure |
| AC-5 `upload()` single allocation, cloned per connection, correct `bytes_sent`/`elapsed` | ✅ | One `Bytes::from(vec![0u8;...])` at line 114; `body.clone()` per connection; `elapsed` wraps `try_join_all`; `bytes_sent = bytes_per * n` |
| AC-6 Cloudflare URLs via `query_pairs_mut` | ✅ | `__down` and `__up` parsed in `new()`; `build_download_url` uses `query_pairs_mut().append_pair` |
| AC-7 `MockOptions::default()` matches SPEC-006; non-2xx tests work | ✅ | `common/mod.rs:47-56`; handlers check status before serving body; download/upload counters always increment |
| AC-8 `Accept-Encoding: identity` both functions | ✅ | `grep` returns two matches: lines 18 and 42 of `throughput.rs` |
| AC-14 `connections == 0` guard before network | ✅ | `cloudflare.rs:71-75` (download) and `101-105` (upload); both before any allocation or network call |
| AC-9 All three CI runners | ✅ (pending) | Not verified locally; CI status TBD at ship time |
| AC-10 No new top-level deps | ✅ | `Cargo.toml` `[dependencies]` and `[dev-dependencies]` unchanged |
| AC-11 No `unwrap`/`expect`/`panic` in lib code | ✅ | `grep` returns no matches in `throughput.rs` or new `cloudflare.rs` code |
| AC-12 `clippy` and `fmt --check` clean | ✅ | Both pass with zero warnings |
| AC-13 All prior tests pass | ✅ | buffer_pool (7), latency (9), smoke (4), cli (13), version (1), metrics (10) — all pass |

### Frame outcomes

| Item | Status | Notes |
|---|---|---|
| (A) HTTP/2 stall risk for SPEC-013 | ✅ | Noted in Build Completion notes; `parallel_downloads_via_select_all` uses HTTP/1.1 MockServer — limitation acknowledged |
| (B) DEC-005 upload RSS follow-up | ✅ | `DEC-005-buffer-strategy.md` Consequences section has explicit STAGE-004 follow-up paragraph |
| (C) `Accept-Encoding` test dropped; AC-8 is code inspection | ✅ | No header-recording test; AC-8 verified by grep |
| (F) `connections == 0` guard added; test exists | ✅ | Both guards present before body allocation; `connections_zero_returns_error` test covers both paths |

### Build surprises

| Item | Status | Notes |
|---|---|---|
| Rust 2024 `use<>` capture annotation | ✅ | `throughput.rs:13` — `+ use<>` present on `download_one` return type; correct (prevents `&Client` capture in `'static` stream) |
| `#![allow(clippy::panic)]` justification | ✅ | `tests/throughput.rs:1` — justified by `Result<impl Stream, ...>` not implementing `Debug`, requiring explicit match-arm `panic!()` |

### Test run

- **Total:** 69 tests, 0 failures, 0 ignored
- **`tests/throughput.rs`:** 9 tests (matches spec's post-drop count)
- All test files pass without modification

## Reflection (Ship)

**What went well or was easier than expected?**

The Implementation Context in the Design spec was accurate enough to write
the code directly — the `download_one`/`upload_one` signatures, the
`try_join_all` + `select_all` wiring pattern, and the `MockOptions`
extension shape all transferred to working code with minimal adjustment.
Frame critique quality was high: items (A) (HTTP/2 stall), (B) (upload
RSS), and (F) (`connections == 0` guard) were all genuine issues, and all
three resolved cleanly in Build without re-opening the Frame. The
`latency.rs` precedent made `throughput.rs` straightforward to structure.

**What was harder, surprising, or required a correction?**

Two Build surprises, both worth carrying into SPEC-011:

1. **Rust 2024 RPIT lifetime capture rules.** `download_one` returns
   `-> Result<impl Stream + Send + 'static, _>`, but `&Client` would
   normally be captured in the `'static` bound because the function
   takes `client: &Client`. Rust 2024's conservative default (capture
   all named lifetimes) rejects this. The fix is `+ use<>` on the
   return type — the stream doesn't actually borrow from `client`
   (it takes ownership of `response`), so `use<>` is correct. This
   will affect SPEC-011's `GenericHttpBackend` wiring the same way;
   both backends call the same `throughput::download_one`, so SPEC-011
   can copy the pattern verbatim.

2. **`Result<impl Stream, _>` doesn't implement `Debug`.** When testing
   error paths with `assert!(matches!(...))`, you can't use `?` or
   `.unwrap()` on an opaque `impl Stream` result because the `Debug`
   bound isn't satisfied. The idiomatic fix is explicit `match` arms
   that `panic!()` on the wrong variant. This adds `#![allow(clippy::
   panic)]` to `tests/throughput.rs` alongside the existing `unwrap_used`
   / `expect_used` allowances. Any future spec that tests `download_one`
   error paths directly will need the same allowance.

**What should SPEC-011 (and SPEC-012/013) know?**

- **`throughput.rs` is the shared module SPEC-011 delegates to** —
  `GenericHttpBackend::download` and `::upload` will be near-trivial
  wires: construct URL from `opts`, call `throughput::download_one` /
  `upload_one`, apply the same `try_join_all` + `select_all` pattern.
  No new module needed; Build should be short.

- **HTTP/2 stall risk on `try_join_all` is real and SPEC-013 must
  verify.** `parallel_downloads_via_select_all` tests against
  HTTP/1.1 MockServer (separate connections). Against Cloudflare's
  HTTP/2 endpoint, all N connections may be multiplexed onto a single
  TCP stream; if the server-side flow control window stalls, all N
  futures block together. Frame item (A) was deferred because MockServer
  doesn't reproduce HTTP/2 multiplexing — SPEC-013's live-network tests
  are the first opportunity to observe this. Track via
  `guidance/questions.yaml` id `cloudflare-http2-stall-on-parallel-download`.

- **Upload RSS concern punted to STAGE-004.** A single `Bytes::from(vec!
  [0u8; N])` is one allocation; with N=25MB and 4 connections that's
  100MB RSS floor before reqwest/hyper buffering. DEC-005's Consequences
  section was amended during Build with this note. STAGE-004 should
  profile RSS against the budget before shipping to users.

- **`connections == 0` guard placement matters.** Both guards sit before
  `Bytes::from(vec![0u8; ...])` in `upload()` and before any `try_join_
  all` in `download()`. If the guard is added after the allocation,
  the zero-length body case becomes an empty `try_join_all` which returns
  `Ok(())` rather than an error — subtle difference. The current placement
  is correct; don't move it.

- **`MockOptions` extension shape.** `tests/common/mod.rs` now carries
  `download_status`, `upload_status`, `download_counter`, and
  `upload_counter`. Any future spec extending `MockOptions` should follow
  this additive pattern (`Default::default()` must match the existing
  passing tests).
