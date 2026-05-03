---
task:
  id: SPEC-011
  type: story
  cycle: ship
  blocked: false
  priority: high
  complexity: S
  estimated_hours: 2

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: rspeed

agents:
  architect: claude-sonnet-4-6
  implementer: null
  created_at: 2026-05-02

references:
  decisions: [DEC-002, DEC-003, DEC-005]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-009, SPEC-010, SPEC-012]

value_link: "wires GenericHttpBackend download/upload so --server works end-to-end and integration tests can drive the full trait surface without live Cloudflare traffic"

cost:
  sessions:
    - cycle: design
      date: 2026-05-02
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 3654552
      tokens_output: 73821
      estimated_usd: 4.0047
      note: "Spec authoring + Frame critique in single Sonnet session"
    - cycle: build
      date: null
      agent: null
      interface: claude-code
      tokens_input: 3421562
      tokens_output: 50544
      estimated_usd: 3.0914
      note: ""
    - cycle: verify
      date: null
      agent: null
      interface: claude-code
      tokens_input: 1192371
      tokens_output: 20093
      estimated_usd: 2.1804
      note: ""
    - cycle: ship
      date: null
      agent: null
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: ""
  totals:
    tokens_total: 8412943
    estimated_usd: 9.2765
    session_count: 3
---

# SPEC-011: Generic HTTP backend — real download/upload

## Context

Fifth measurement spec under STAGE-002. SPEC-010 shipped
`CloudflareBackend::download` and `::upload` plus the shared
`src/backend/throughput.rs` module (`download_one`, `upload_one`).
SPEC-011 wires `GenericHttpBackend` to the same shared module.

`GenericHttpBackend` currently returns `Err(BackendError::NotImplemented)`
for both `download` and `upload`. This means `--server <url>` is
silently broken for any user who tries it. SPEC-011 fixes that by
delegating to `throughput.rs` — the same per-connection HTTP mechanics
already proven in SPEC-010.

This is also the first spec that can do **full trait-level integration
testing** for download and upload: `GenericHttpBackend`'s URLs are
configurable, so tests can point it at `MockServer`. That testing
infrastructure is reusable by SPEC-012 (test orchestrator).

DEC-002 governs the HTTP client. DEC-003 specifies the Generic backend
protocol: `GET {base}/download?bytes=N`, `POST {base}/upload`. DEC-005
covers upload allocation — same reasoning as SPEC-010 (single
allocation, O(1) clone per connection; pool not applicable to the
reqwest streaming path).

## Goal

Wire `GenericHttpBackend::download()` and `::upload()` by extracting
the parallel-dispatch logic from `CloudflareBackend` into two new
`throughput.rs` helpers (`throughput::download_parallel` and
`throughput::upload_parallel`), reducing both backends to one-liner delegations
and eliminating ~30 lines of duplication. Add trait-level integration
tests in `tests/generic_backend.rs`.

No `Backend` trait signature changes. No new top-level dependencies.

## Inputs

- **`src/backend/throughput.rs`** — existing `download_one` / `upload_one`;
  SPEC-011 adds `build_download_url`, `download` (parallel orchestration),
  `upload` (parallel orchestration)
- **`src/backend/cloudflare.rs`** — SPEC-010's implementation; SPEC-011
  simplifies `download()` / `upload()` to single-line delegations and
  removes `build_download_url` (it moves to `throughput.rs`)
- **`src/backend/generic.rs`** — current state: stub `download`/`upload`
  returning `NotImplemented`; SPEC-011 fills both
- **`src/backend/mod.rs`** — `Backend` trait, opts/result types,
  `DownloadStream`
- **`tests/common/mod.rs`** — `MockServer` already has `download_count()`,
  `upload_count()`, `download_status`, `upload_status`; no extensions needed
- **`tests/throughput.rs`** — SPEC-010's 9 tests. **Invariant:** SPEC-011 must
  not require any modification to this file. The `CloudflareBackend` refactor
  (download/upload bodies become one-line delegations) preserves all behavior;
  the SPEC-010 test suite is the regression net for the extraction
- **`decisions/DEC-003-backend-abstraction.md`** — Generic protocol contract:
  `GET {base}/download?bytes=N`, `POST {base}/upload`
- **`decisions/DEC-002-http-client.md`** — `reqwest` config

## Outputs

- **Files created:**
  - `tests/generic_backend.rs` — trait-level integration tests (see
    **Failing Tests**)

- **Files modified:**
  - `src/backend/throughput.rs`:
    - Add `pub fn build_download_url(base: &Url, bytes: u64) -> Result<Url, BackendError>`
      (moved from `CloudflareBackend`; no behavior change)
    - Add `pub async fn download_parallel(client: &Client, download_base_url: &Url, opts: &DownloadOpts) -> Result<DownloadStream, BackendError>`
      (parallel orchestration extracted from `CloudflareBackend::download`)
    - Add `pub async fn upload_parallel(client: &Client, upload_url: &Url, opts: &UploadOpts) -> Result<UploadResult, BackendError>`
      (parallel orchestration extracted from `CloudflareBackend::upload`)
    - All three additions are `pub` (same visibility as `download_one` / `upload_one`)
  - `src/backend/cloudflare.rs`:
    - Replace `download()` body with `throughput::download_parallel(&self.client, &self.download_base_url, opts).await`
    - Replace `upload()` body with `throughput::upload_parallel(&self.client, &self.upload_url, opts).await`
    - Remove `build_download_url` associated function (it moves to `throughput.rs`)
  - `src/backend/generic.rs`:
    - Add `download_base_url: Url` and `upload_url: Url` fields
    - Populate both in `new()` via `base_url.join("download")?` and `base_url.join("upload")?`
    - Add `throughput` to `use super::` imports
    - Replace `download()` body with `throughput::download_parallel(&self.client, &self.download_base_url, opts).await`
    - Replace `upload()` body with `throughput::upload_parallel(&self.client, &self.upload_url, opts).await`

- **`Cargo.toml`:** no changes. All needed types and functions are in
  deps already on the graph (`futures`, `bytes`, `reqwest`, `tokio`).

- **`tests/common/mod.rs`:** no changes. `download_status`, `upload_status`,
  `download_count()`, `upload_count()` already exist from SPEC-010.

## Acceptance Criteria

- [ ] **AC-1: `throughput::build_download_url` is `pub` and moved from
  `cloudflare.rs`.** Identical behavior to the former `CloudflareBackend::
  build_download_url`: appends `?bytes=N` via `url.query_pairs_mut().
  append_pair("bytes", &bytes.to_string())`. `CloudflareBackend` calls
  it via `throughput::build_download_url(...)`.

- [ ] **AC-2: `throughput::download_parallel` contains the parallel orchestration
  logic.** Signature: `pub async fn download_parallel(client: &Client, download_base_url:
  &Url, opts: &DownloadOpts) -> Result<DownloadStream, BackendError>`.
  Implementation: `connections == 0` guard (returns `Protocol` error);
  builds per-connection URLs via `build_download_url`; issues N futures
  via `try_join_all`; merges streams via `select_all`; returns
  `Box::pin(merged)`. Identical logic to the SPEC-010 `CloudflareBackend::
  download` body, parameterized over URL.

- [ ] **AC-3: `throughput::upload_parallel` contains the parallel orchestration
  logic.** Signature: `pub async fn upload_parallel(client: &Client, upload_url:
  &Url, opts: &UploadOpts) -> Result<UploadResult, BackendError>`.
  Implementation: `connections == 0` guard; single `Bytes` allocation,
  `clone()` per connection; `try_join_all`; wall-clock `elapsed` wraps
  the join; returns `UploadResult::new(bytes_per * n, elapsed)`. Identical
  logic to the SPEC-010 `CloudflareBackend::upload` body, parameterized
  over URL.

- [ ] **AC-4: `CloudflareBackend::download` and `::upload` delegate to
  `throughput`.** Both method bodies become single-expression delegations.
  All existing `tests/throughput.rs` tests pass without modification,
  including `connections_zero_returns_error` (which now goes through
  `throughput::download_parallel`/`throughput::upload_parallel`).

- [ ] **AC-5: `GenericHttpBackend` has `download_base_url` and
  `upload_url` fields.** Both are `Url`, constructed in `new()` via
  `base_url.join("download")` and `base_url.join("upload")` respectively.
  URL join errors map to `BackendError::Protocol(e.to_string())`, same
  pattern as the existing `ping_url` construction.

- [ ] **AC-6: `GenericHttpBackend::download` issues parallel requests
  to `{base_url}download?bytes=N`.** With `opts.connections == n`,
  `n` parallel GET requests hit `download?bytes=N` on `MockServer`.
  Verified by `mock.download_count() == n` in the parallel-connections
  test.

- [ ] **AC-7: `GenericHttpBackend::upload` issues parallel POST requests
  to `{base_url}upload`.** With `opts.connections == n`, `n` parallel
  POST requests hit `upload` on `MockServer`. `UploadResult::bytes_sent
  == opts.bytes_per_request * n`. Verified by `mock.upload_count() == n`.

- [ ] **AC-8: `connections == 0` guard lives in `throughput::download_parallel`
  and `throughput::upload_parallel`, not in each backend.** Code inspection:
  `grep -n 'connections == 0' src/backend/throughput.rs` returns two
  matches (one per function); `grep -n 'connections == 0' src/backend/
  cloudflare.rs` and `...generic.rs` return zero matches.

- [ ] **AC-9: All prior tests pass without modification.** All tests in
  `tests/throughput.rs` (9), `tests/latency.rs`, `tests/smoke.rs`,
  `tests/buffer_pool.rs`, `tests/cli.rs`, `tests/version.rs`,
  `tests/metrics.rs` continue to pass.

- [ ] **AC-10: `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` pass.**

- [ ] **AC-11: Lib-side `unwrap`/`expect`/`panic` discipline preserved.**
  No `unwrap()`, `expect()`, or `panic!()` in `throughput.rs` new code,
  or in `generic.rs`. `tests/generic_backend.rs` carries
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`
  per project test convention.

- [ ] **AC-12: No new top-level deps.** `Cargo.toml` `[dependencies]`
  and `[dev-dependencies]` unchanged.

- [ ] **AC-13: All three CI runners.** macOS arm64, Linux x86_64, Windows
  x86_64.

- [ ] **AC-14: Multi-connection helpers named `download_parallel` /
  `upload_parallel`.** The two new public functions in `throughput.rs` are
  named `download_parallel` and `upload_parallel` (not `download` / `upload`).
  The `_parallel` suffix disambiguates from the trait methods `Backend::
  download` / `Backend::upload` at call sites, since backend impls call the
  helper from inside an `async fn download(...)` / `async fn upload(...)`
  body — identical names would invite misreads three months later. Verified
  by `grep -n 'fn download_parallel\|fn upload_parallel' src/backend/
  throughput.rs` returning two matches.

- [ ] **AC-15: `Url::join` trailing-slash gotcha logged in
  `guidance/questions.yaml`.** SPEC-011's `GenericHttpBackend::new` uses
  `base_url.join("download")` and `base_url.join("upload")`, which
  silently replaces the last path segment if `base_url` lacks a trailing
  slash (a user passing `--server http://server/api` would get requests
  to `http://server/download`, not `http://server/api/download`). This is
  inherited from SPEC-008's `ping_url` construction; SPEC-011 doesn't fix
  it, but adds an entry to `guidance/questions.yaml` so the latent issue
  is visible:

  ```yaml
  - id: generic-backend-base-url-trailing-slash
    question: "Should --server URL require a trailing slash, or should we
      normalize? Url::join replaces the last path segment if no trailing
      slash, which silently breaks user-supplied base URLs with a path
      component (e.g. http://server/api/)."
    severity: warning
    raised_at: 2026-05-02
    raised_by: SPEC-011 design (inherited from SPEC-008)
    blocks: null
  ```

  Resolution is deferred to STAGE-003 or a future polish spec.

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/generic_backend.rs`. The file opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use common::{MockOptions, MockServer};
use futures::StreamExt;
use rspeed::{Backend, BackendError, DownloadOpts, GenericHttpBackend, UploadOpts};

fn build_backend(mock: &MockServer) -> GenericHttpBackend {
    GenericHttpBackend::new(mock.base_url()).unwrap()
}
```

---

**`"generic_backend_download_happy_path"`** — `#[tokio::test]`.
Builds backend against `MockServer::start()`. Calls `backend.download(
&DownloadOpts::new(1_048_576, 1)).await.unwrap()`. Drains the stream.
Asserts total bytes == `1_048_576` and `mock.download_count() == 1`.

**`"generic_backend_upload_happy_path"`** — `#[tokio::test]`.
Calls `backend.upload(&UploadOpts::new(64 * 1024, 1)).await.unwrap()`.
Asserts `result.bytes_sent == 64 * 1024`, `!result.elapsed.is_zero()`,
and `mock.upload_count() == 1`.

**`"generic_backend_download_parallel_connections"`** — `#[tokio::test]`.
Calls `backend.download(&DownloadOpts::new(256 * 1024, 4)).await.unwrap()`.
Drains merged stream. Asserts total bytes == `4 * 256 * 1024` and
`mock.download_count() == 4`.

**`"generic_backend_upload_parallel_connections"`** — `#[tokio::test]`.
Calls `backend.upload(&UploadOpts::new(16 * 1024, 4)).await.unwrap()`.
Asserts `result.bytes_sent == 4 * 16 * 1024` and `mock.upload_count() == 4`.

**`"generic_backend_non_2xx_download_returns_protocol_error"`** —
`#[tokio::test]`. Starts `MockServer::start_with_options(MockOptions {
download_status: StatusCode::INTERNAL_SERVER_ERROR, ..Default::default()
})`. Calls `backend.download(&DownloadOpts::new(1024, 1)).await`.
Asserts `Err(BackendError::Protocol(msg))` where `msg.contains("500")`.

**`"generic_backend_connection_refused_returns_network_error"`** —
`#[tokio::test]`. Constructs `GenericHttpBackend::new(
"http://127.0.0.1:1/".parse().unwrap()).unwrap()`. Calls `backend.download(
&DownloadOpts::new(1024, 1)).await`. Asserts `Err(BackendError::Network(_))`.
No timeout needed — kernel-level RST is fast.

**`"generic_backend_connections_zero_returns_error"`** — `#[tokio::test]`.
Uses default `MockServer`. Calls `backend.download(&DownloadOpts::new(
1024, 0)).await`. Asserts `Err(BackendError::Protocol(_))`. Calls
`backend.upload(&UploadOpts::new(1024, 0)).await`. Asserts
`Err(BackendError::Protocol(_))`. No network traffic expected (guard
fires before any request).

---

**Note on `Result<impl Stream, _>` Debug gap:** Per SPEC-010 build
surprise #2, `Result<impl Stream, BackendError>` does not implement
`Debug`, so tests that check error paths on `Backend::download()` must
use explicit `match` arms with `panic!()` rather than `.unwrap()` or
`assert!(matches!(...))`. The `#![allow(clippy::panic)]` on the test
file covers this. Tests that check `Backend::upload()` error paths can
use `unwrap()` since `Result<UploadResult, BackendError>` does implement
`Debug`.

## Implementation Context

### `throughput.rs` additions

The extracted `download_parallel` and `upload_parallel` helpers use the
same logic as the current `CloudflareBackend` methods, parameterized over
URL inputs:

```rust
pub fn build_download_url(base: &Url, bytes: u64) -> Result<Url, BackendError> {
    let mut url = base.clone();
    url.query_pairs_mut().append_pair("bytes", &bytes.to_string());
    Ok(url)
}

pub async fn download_parallel(
    client: &Client,
    download_base_url: &Url,
    opts: &DownloadOpts,
) -> Result<DownloadStream, BackendError> {
    if opts.connections == 0 {
        return Err(BackendError::Protocol("connections must be > 0".to_string()));
    }
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = client.clone();
            let url_result = build_download_url(download_base_url, bytes_per);
            async move {
                let url = url_result?;
                download_one(&client, url).await
            }
        })
        .collect();

    let streams = futures::future::try_join_all(futures_list).await?;
    let pinned: Vec<BoxStream<'static, Result<Bytes, BackendError>>> = streams
        .into_iter()
        .map(|s| -> BoxStream<'static, Result<Bytes, BackendError>> { Box::pin(s) })
        .collect();
    Ok(Box::pin(futures::stream::select_all(pinned)))
}

pub async fn upload_parallel(
    client: &Client,
    upload_url: &Url,
    opts: &UploadOpts,
) -> Result<UploadResult, BackendError> {
    if opts.connections == 0 {
        return Err(BackendError::Protocol("connections must be > 0".to_string()));
    }
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    // DEC-005: one allocation per upload() call, cloned per connection.
    let body = Bytes::from(vec![0u8; bytes_per as usize]);
    let start = Instant::now();

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = client.clone();
            let url = upload_url.clone();
            let body = body.clone();
            async move { upload_one(&client, url, body).await }
        })
        .collect();

    futures::future::try_join_all(futures_list).await?;
    let elapsed = start.elapsed();
    Ok(UploadResult::new(bytes_per * (n as u64), elapsed))
}
```

New imports needed in `throughput.rs`:

```rust
use futures::stream::BoxStream;

use super::{BackendError, DownloadOpts, DownloadStream, UploadOpts, UploadResult};
```

(`DownloadOpts`, `UploadOpts`, `UploadResult`, `DownloadStream` are
not currently imported in `throughput.rs`; `BoxStream` is needed for the
`map(|s| -> BoxStream<...>` cast.)

### `cloudflare.rs` after refactor

`CloudflareBackend::download` becomes:

```rust
async fn download(&self, opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
    throughput::download_parallel(&self.client, &self.download_base_url, opts).await
}
```

`CloudflareBackend::upload` becomes:

```rust
async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
    throughput::upload_parallel(&self.client, &self.upload_url, opts).await
}
```

`build_download_url` is deleted from `cloudflare.rs`; it moves to
`throughput.rs` and is now internal to `throughput::download_parallel`.

**Dead imports to remove from `cloudflare.rs`:** After the refactor,
`Bytes`, `Duration`, `Instant`, and `futures::stream::BoxStream` are no
longer referenced in `cloudflare.rs` (all three were used only in the
former `download`/`upload` bodies). Clippy (`-D warnings`) will flag
them as unused imports; remove them as part of this PR.

### `generic.rs` additions

New fields in `GenericHttpBackend`:

```rust
pub struct GenericHttpBackend {
    base_url: Url,
    client: reqwest::Client,
    ping_url: Url,
    tcp_target: String,
    download_base_url: Url,   // new
    upload_url: Url,           // new
}
```

Construction in `new()` (after existing `ping_url` construction):

```rust
let download_base_url = base_url
    .join("download")
    .map_err(|e| BackendError::Protocol(e.to_string()))?;
let upload_url = base_url
    .join("upload")
    .map_err(|e| BackendError::Protocol(e.to_string()))?;
```

`use super::` import gains `throughput`:

```rust
use super::{
    Backend, BackendError, DownloadOpts, DownloadStream, LatencyProbeOutcome,
    UploadOpts, UploadResult, throughput,
};
```

### URL join semantics

`Url::join` replaces the last path segment unless the base URL ends
with `/`. `MockServer::base_url()` always returns a trailing-slash URL
(`http://127.0.0.1:{port}/`), so `base.join("download")` →
`http://127.0.0.1:{port}/download`. For production use, the user must
pass a base URL with a trailing slash when using path prefixes. This is
the same constraint that already applies to the existing `ping_url`
construction — no new documentation needed.

### Rust 2024 RPIT and `use<>` — not applicable here

SPEC-010 required `+ use<>` on `download_one`'s return type because
the RPIT return captures `&Client` in the `'static` stream. The
extracted `throughput::download_parallel` helper has no RPIT — it returns the
concrete `Result<DownloadStream, BackendError>` where `DownloadStream`
is `BoxStream<'static, ...>`. The `&Client` and `&Url` references are
used to build futures but are never captured in the returned `BoxStream`.
No `use<>` annotation needed.

### Buffer pool accounting

Same as SPEC-010: the pool is not applicable to the reqwest streaming
path. SPEC-011 doesn't change this reasoning; see SPEC-010's
**Buffer pool accounting** section.

### No `tests/common/mod.rs` extensions needed

`MockOptions` already has `download_status`, `upload_status` (for
non-2xx tests). `MockServer` already has `download_count()`,
`upload_count()` (for parallel-connection tests). Both added in
SPEC-010. `MockOptions::default()` continues to reproduce SPEC-006
behavior, so existing test files are unaffected.

### What this spec does NOT do

- Live tests against Cloudflare's `--server`-mode URL — those are
  gated behind `live` feature (SPEC-013)
- Change the `Backend` trait signature
- Use the SPEC-009 buffer pool
- Implement the test orchestrator (SPEC-012)

## Frame Critique

### (A) Code duplication between backends — Decision: **Extract**

The parallel dispatch logic in `CloudflareBackend::download` and the
forthcoming `GenericHttpBackend::download` would be byte-for-byte
identical except for the URL input. The same is true for `upload`.
Extracting into `throughput::download_parallel` and `throughput::upload_parallel`:

- Reduces duplication of ~30 lines per backend (including the
  `connections == 0` guard, `try_join_all`, `select_all` wiring,
  and `BoxStream` pinning).
- Centralizes the guard so future backends cannot accidentally omit it.
- Makes `throughput.rs` the complete source of HTTP measurement
  mechanics (both per-connection and multi-connection), with all
  backends as thin URL-configuration wrappers.
- Touching SPEC-010's `cloudflare.rs` code is justified: this is DRY
  enforcement at the natural second-implementor boundary, not scope
  creep. The net delta to `cloudflare.rs` is a deletion (~25 lines
  removed, ~1 added). All SPEC-010 tests continue to pass unchanged.

Trade-off: `throughput.rs` now has two abstraction levels
(`download_one`/`upload_one` for per-connection, `download_parallel`/
`upload_parallel` for multi-connection orchestration). The `_parallel`
suffix on the multi-connection variants disambiguates them visually
from the trait methods (`Backend::download` / `Backend::upload`) at
call sites — important because backend impls read like `throughput::
download_parallel(&self.client, &self.download_base_url, opts).await`
inside `async fn download(...)`, and identical naming would invite
misreads.

### (B) URL construction — Decision: **Store `download_base_url`, construct per-request**

`bytes_per_request` comes from `DownloadOpts`, which arrives at call
time, so the full URL (`base?bytes=N`) cannot be stored at `new()` time.
Both backends store the bare base URL for download and the full upload
URL, identical to the Cloudflare pattern. No new design question.

### (C) Trait-level tests — Decision: **7 tests in `tests/generic_backend.rs`**

SPEC-010's `tests/throughput.rs` tests `download_one`/`upload_one`
directly. SPEC-011 adds trait-level tests that exercise the full
`Backend::download` → `throughput::download_parallel` → `download_one` path.
This is more valuable than duplicating module-level tests because:

- It validates the URL construction (`base_url.join("download")` +
  `?bytes=N`) end-to-end against a real HTTP server.
- It validates `UploadResult.bytes_sent` computation, which lives in
  `throughput::upload_parallel`, not `upload_one`.
- It proves the `GenericHttpBackend` wiring is correct (URL construction,
  field initialization, `throughput` import chain).
- It's the test infrastructure SPEC-012's orchestrator can use to
  drive `&dyn Backend` without Cloudflare traffic.

7 tests cover: download happy path, upload happy path, parallel download,
parallel upload, non-2xx, connection refused, `connections == 0`.

### (D) `connections == 0` guard — **Resolved by (A)**

Extracting into `throughput::download_parallel`/`throughput::upload_parallel` puts the
guard in one place. The SPEC-010 test `connections_zero_returns_error`
continues to pass (it fires the same code path via `CloudflareBackend`,
which now delegates). No duplication; no new AC needed beyond AC-8.

---

## Build Completion

*Filled in at the end of the **build** cycle, before advancing to verify.*

- **Branch:** feat/spec-011-generic-throughput
- **PR (if applicable):** pending
- **All acceptance criteria met?** yes
- **New decisions emitted:** none
- **Deviations from spec:** none
- **Follow-up work identified:** none

### Build-phase reflection (3 questions, short answers)

1. **What was unclear in the spec that slowed you down?**
   — Nothing. The Implementation Context code skeletons were exact; the carry-forwards from SPEC-010 (Debug gap, no `use<>` needed) were called out precisely.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No gaps. `cargo fmt` line-length differences between the spec skeleton and rustfmt's actual output were expected noise, not spec ambiguity.

3. **If you did this task again, what would you do differently?**
   — Run `cargo fmt` before `cargo clippy` in the same command to catch format drift earlier.

---

## Verification Results

*Filled in at the end of the **verify** cycle.*

### Refactor correctness

- ✅ `tests/throughput.rs` unchanged vs `main` (`git diff main -- tests/throughput.rs` empty)
- ✅ All 9 SPEC-010 throughput tests pass unchanged
- ✅ `CloudflareBackend::download` and `::upload` are literal one-liners — no for-loops, no `Vec::with_capacity`, no `try_join_all` in the method bodies
- ✅ `build_download_url` absent from `cloudflare.rs` (lives in `throughput.rs`)
- ✅ No dead imports in `cloudflare.rs` — `Bytes`, `Instant`, `BoxStream` all removed; clippy clean

### ACs

- ✅ **AC-1** — `throughput::build_download_url` is `pub`; appends `?bytes=N` via `query_pairs_mut`; called by `download_parallel`
- ✅ **AC-2** — `download_parallel` signature matches spec; `connections == 0` guard at line 75; `try_join_all` + `select_all` wiring present
- ✅ **AC-3** — `upload_parallel` signature matches spec; `connections == 0` guard at line 107; single `Bytes` allocation + `clone()` per connection; wall-clock `elapsed` wraps `try_join_all`
- ✅ **AC-4** — Cloudflare delegations are one-liners; all 9 SPEC-010 tests pass
- ✅ **AC-5** — `GenericHttpBackend` has `download_base_url: Url` and `upload_url: Url`; both constructed via `base_url.join(...)` in `new()`; errors mapped to `BackendError::Protocol`
- ✅ **AC-6** — `generic_backend_download_parallel_connections` asserts `mock.download_count() == 4`; passes
- ✅ **AC-7** — `generic_backend_upload_parallel_connections` asserts `bytes_sent == 4 * 16 * 1024` and `mock.upload_count() == 4`; passes
- ✅ **AC-8** — `grep 'connections == 0' src/backend/throughput.rs` → 2 matches (lines 75, 107); same grep on `cloudflare.rs` and `generic.rs` → 0 matches
- ✅ **AC-9** — All 66 tests pass (`cargo test --all-targets`). The spec predicted 76; the actual count was an overshoot — all named test files pass with zero failures. New total is 59 prior + 7 new generic_backend tests = 66.
- ✅ **AC-10** — `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both pass clean
- ✅ **AC-11** — No `unwrap`/`expect`/`panic` in `throughput.rs` new code or `generic.rs`; `tests/generic_backend.rs` opens with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`
- ✅ **AC-12** — `Cargo.toml` unchanged (confirmed via source inspection; no new deps)
- ✅ **AC-13** — CI green on all three runners: macOS arm64, Linux x86_64, Windows x86_64
- ✅ **AC-14** — `grep 'fn download_parallel\|fn upload_parallel' src/backend/throughput.rs` → 2 matches (lines 70, 102)
- ✅ **AC-15** — `guidance/questions.yaml` has `generic-backend-base-url-trailing-slash` entry with exact YAML shape from spec

### Test file structure

- ✅ Opens with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`
- ✅ Exactly 7 tests, all names match spec's Failing Tests section
- ✅ Error-path download tests use explicit `match` arms with `panic!()` (not `unwrap`) — covers `non_2xx`, `connection_refused`, and `connections_zero` download arm
- ✅ Error-path upload tests also use explicit `match` (more conservative than required; fine)

### Honest concerns (non-blocking)

- HTTP/2 stall risk from SPEC-010 Frame (A) is unchanged — same code path, still tracked in `guidance/questions.yaml` for SPEC-013. No visibility decay.
- Upload-RSS budget (DEC-005 amend) unchanged. No regression.

---

## ✅ APPROVED

---

## Reflection (Ship)

*Appended during the **ship** cycle.*

1. **What went well or was easier than expected?**
   — The extract-on-second-implementor pattern worked exactly as designed. SPEC-010 gave us the working implementation; SPEC-011 was mechanical: lift the parallel dispatch bodies into `throughput.rs`, add URL parameters, wire both backends to the helpers. `MockServer` needed zero extensions. The spec's Implementation Context code skeletons were exact — the build-phase reflection noted no surprises at all.

2. **What was harder, surprising, or required correction?**
   — The test-count discrepancy: the PR description (written during build) predicted 76 tests (69 prior + 7 new), but `cargo test --all-targets` actually reports 66. The root cause is that Cargo outputs "running 1 test" (singular) for single-test binaries, which a grep for `running [0-9]+ tests` silently misses. Verify caught this and AC-9 documents the correct count (59 prior + 7 new = 66). Lesson: count from `test result:` summary lines, not `running N tests` lines. No other corrections needed.

3. **What should SPEC-012/013 know?**
   — `Backend::download` and `Backend::upload` are fully implemented for both backends; SPEC-012's orchestrator can drive `&dyn Backend` against either one without conditional logic.
   — HTTP/2 stall risk (SPEC-010 Frame A) remains SPEC-013's problem; tracked in `guidance/questions.yaml`.
   — The `Url::join` trailing-slash gotcha (AC-15) is in `guidance/questions.yaml`; SPEC-012 should consider validating `--server` URLs eagerly so the issue surfaces before the first request fails silently.
   — The `_parallel` naming convention in `throughput.rs` is the established pattern: if a third backend ever lands, call `throughput::download_parallel` / `throughput::upload_parallel` directly — no new pattern needed.
