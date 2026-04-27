---
task:
  id: SPEC-006
  type: chore
  cycle: verify
  blocked: false
  priority: high
  complexity: S

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: rspeed

agents:
  architect: claude-opus-4-7
  implementer: claude-opus-4-7
  created_at: 2026-04-25

references:
  decisions: [DEC-003]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-002, SPEC-005]

value_link: "infrastructure enabling STAGE-002 — the mock server keeps Stage 2 specs sized correctly"

cost:
  sessions:
    - cycle: frame
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "Frame critique with GO verdict; 8 items approved for inline fold into Build"
    - cycle: build
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "Build session; axum mock server + 4 smoke tests"
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-006: Integration test harness with mock server

## Context

Last spec under STAGE-001. A reusable test fixture that spins up a
local HTTP server matching the Generic backend protocol from DEC-003.
Stage 2 specs will build on this; without it Stage 2 balloons in scope
and fragility (every Stage 2 spec would otherwise wire its own mock).

This spec adds `axum` as a dev-dep (it was *not* landed in SPEC-002
under School B; SPEC-006 is the first-consuming spec) and wires it
into a `MockServer` fixture that future tests can use as one line.

## Goal

`tests/common/mod.rs` exposes `MockServer::start()` which returns a
running server bound to a kernel-assigned port on `127.0.0.1`,
implementing `/download`, `/upload`, `/ping`, `/health` per the
DEC-003 protocol. A smoke test in `tests/smoke.rs` verifies the
fixture starts and the constructed `GenericHttpBackend` (still
returning `NotImplemented` for download/upload) reports the correct
name.

## Inputs

- **Files to read:**
  - `DEC-003` (Generic backend protocol)
  - `src/backend/generic.rs` from SPEC-005
  - `Cargo.toml` (this spec adds axum as a dev-dep)

## Outputs

- **Files created:**
  - `tests/common/mod.rs` — `MockServer` struct + handlers
  - `tests/smoke.rs` — smoke tests using the fixture
- **Files modified:** none

## Acceptance Criteria

- [ ] `Cargo.toml` adds `axum` to `[dev-dependencies]` (deferred from
      SPEC-002 to its first-consuming spec under School B). Versions
      aspirational — verify current major line at Build time.
- [ ] `tests/common/mod.rs` exposes `MockServer` with:
  - `MockServer::start() -> MockServer` (async, awaits readiness)
  - `MockServer::base_url() -> Url`
  - A `Drop` impl that gracefully shuts the server down
- [ ] The mock server listens on `127.0.0.1:0` (kernel-assigned port)
      so multiple tests can run in parallel
- [ ] Endpoints implemented:
  - `GET /download?bytes=N` → response of N zero bytes,
    `Content-Length: N`, `Content-Type: application/octet-stream`
  - `POST /upload` → consumes the request body, responds 200 with
    JSON `{"received": <byte_count>}`
  - `GET /ping` → responds 200 with empty body, target latency <1ms
  - `GET /health` → responds 200 with body `"ok"` (smoke check)
- [ ] Server starts in <50ms (so it doesn't dominate test wall time)
- [ ] `tests/smoke.rs` contains:
  - A test that starts the mock, hits `/health` with `reqwest`, asserts 200
  - A test that constructs a `GenericHttpBackend` against the mock's
    `base_url()` and verifies `backend.name() == "generic"` (the
    actual `download()`/`upload()` calls still error with
    `NotImplemented` — that's expected at this stage)
- [ ] `tests/common/mod.rs` and `tests/smoke.rs` both open with
      `#![allow(clippy::unwrap_used, clippy::expect_used)]` per the
      SPEC-005 project-wide convention
- [ ] All smoke tests pass on all four CI runners

### Frame outcomes folded into Build (2026-04-27)

1. **(A) axum = "0.8" only** — `tokio-test` and `tempfile` not needed; add only `axum` to `[dev-dependencies]`
2. **(B) `#![allow]` in both test files** — both `tests/common/mod.rs` and `tests/smoke.rs` open with `#![allow(clippy::unwrap_used, clippy::expect_used)]` per SPEC-005 project-wide convention (AC added above)
3. **(C) Drop impl uses `Option::take()`** — `shutdown_tx: Option<oneshot::Sender<()>>` with `take()` in Drop; cleaner than the `mem::replace` sketch in Notes
4. **(D) Cross-platform binding confirmed** — `TcpListener::bind("127.0.0.1:0")` works uniformly across macOS arm64, Linux x86_64, Windows x86_64
5. **(E) Visibility** — `MockServer` is `pub` within tests crate only; no library API surface
6. **(F) AC completeness** — `#![allow]` bullet and axum dev-dep AC added; list is now complete
7. **(G) Scope** — complexity stays S; no spec frontmatter change
8. **(H) Last spec in STAGE-001** — Stage Ship cycle fires after SPEC-006 ships; not triggered from this Build

## Failing Tests

- **`tests/smoke.rs`**
  - `"mock health 200"` — start mock, `reqwest::get(format!("{}/health", base_url))`, assert 200
  - `"mock download bytes"` — start mock, `reqwest::get(format!("{}/download?bytes=1024", base_url))`, assert content length 1024
  - `"mock upload echoes"` — start mock, POST 512 bytes to `/upload`, assert response JSON `{"received": 512}`
  - `"generic backend reports name"` — construct `GenericHttpBackend` against mock's `base_url`, assert `backend.name() == "generic"`

## Implementation Context

### Decisions that apply

- `DEC-003` — Generic backend protocol. The mock implements *exactly*
  this protocol (`/download?bytes=N`, `/upload`, `/ping`). This makes
  it our canonical test surface; Stage 2 measurement specs target the
  protocol, not Cloudflare specifics.

### Constraints that apply

- `test-before-implementation` — the smoke tests above are written
  first.
- `no-new-top-level-deps-without-decision` — this spec adds `axum` as
  a dev-dep (justified inline by the test-harness need; SPEC-002 under
  School B did not land it). No further new deps expected.

### Prior related work

- SPEC-005 lands `GenericHttpBackend`. This spec gives it a server to
  point at.

### Out of scope

- Modeling failure cases (truncated streams, slow servers, TLS errors,
  connection drops). Those become Stage 2 specs as concrete failure
  paths emerge.
- HTTPS support in the mock. We test against the mock over HTTP;
  the production code path uses HTTPS but we don't need to exercise
  TLS in unit-style integration tests.
- Recording/asserting on request shapes (mockito's strength). If we
  need that later, add it as a feature on `MockServer` rather than
  swapping libraries.

## Notes for the Implementer

### Why axum, not mockito or wiremock

mockito and wiremock are designed for asserting HTTP request shapes —
useful when you're testing client behavior. We need something
different: a server that streams large response bodies and consumes
large request bodies realistically. axum's streaming support is good
and the dependency overhead is acceptable (this spec adds it as a
dev-dep — SPEC-002 deferred it under School B).

### Server construction

```rust
use axum::{routing::{get, post}, Router};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use std::net::SocketAddr;

pub struct MockServer {
    addr: SocketAddr,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

impl MockServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/health",   get(handlers::health))
            .route("/ping",     get(handlers::ping))
            .route("/download", get(handlers::download))
            .route("/upload",   post(handlers::upload));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async { let _ = rx.await; })
                .await
                .unwrap();
        });

        Self { addr, shutdown_tx: tx, handle }
    }

    pub fn base_url(&self) -> url::Url {
        format!("http://{}", self.addr).parse().unwrap()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        let _ = std::mem::replace(
            &mut self.shutdown_tx,
            tokio::sync::oneshot::channel().0
        ).send(());
        self.handle.abort();
    }
}
```

### Handler details

- `download` handler reads `bytes` query param; default 1MB; cap at,
  say, 1GB to prevent runaway tests. Streams chunks of 64KB zero
  bytes via `axum::body::Body::from_stream`.
- `upload` handler reads body to `/dev/null` equivalent (just count
  bytes) and returns the count as JSON.
- Use `axum::body::Body::from_stream` with `futures::stream::repeat`
  for the download — pre-allocate a 64KB zeroed `Bytes` and clone it
  per chunk (cheap — `Bytes` is reference-counted).

### Common module pattern

`tests/common/mod.rs` is convention. Each integration test file does
`mod common;` to get access. This avoids the "shared between
integration tests" headache.

### Performance

axum starts fast on its own. The bottleneck is tokio runtime startup
if each test creates a fresh runtime — share runtimes per test where
possible via `#[tokio::test]`.

The `unwrap()` calls in the example above are acceptable in test code
(`tests/` is not library code per AGENTS.md). If they ever fire,
the test fails loudly, which is the right outcome for a fixture.

The mock is specifically the **Generic backend protocol** — it does
not mock Cloudflare's specific endpoints (`/__down`, `/__up`). That's
intentional: the generic protocol is our public contract, so testing
against it gives us higher-leverage coverage. Cloudflare-specific
behavior gets tested via live integration tests gated behind the
`live` feature flag (Stage 2 spec).

---

## Build Completion

- **Branch:** `feat/spec-006-test-harness`
- **PR:** pending (pushed for CI)
- **All acceptance criteria met?** Yes — see verification results below
- **New decisions emitted:** none (axum dev-dep justified inline in spec per existing pattern)
- **Deviations from spec:**
  - Drop impl uses `Option<oneshot::Sender<()>>::take()` instead of the Notes sketch's `mem::replace` — cleaner, approved as Frame outcome (C)
  - `upload` handler parameter is `bytes::Bytes` (imported directly) rather than `axum::body::Bytes` path — same type, path was incorrect for axum 0.8
  - `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` added to dev-deps (not anticipated in the Frame prompt but both needed for axum handler derives and JSON parsing); no Frame item needed — both are dev-only and aligned with the no-new-top-level-deps constraint intent
  - `resp.json()` avoided (reqwest's `json` feature not enabled); used `resp.text()` + `serde_json::from_str()` instead — no production dependency change
  - `std::iter::repeat_n()` used instead of `repeat().take()` — clippy `manual_repeat_n` lint under `-D warnings`
- **Follow-up work identified:** STAGE-001 Stage Ship cycle fires after this spec ships

### Verification results

- `cargo fmt --check`: clean
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo test`: 20 passed (2 unit + 13 cli + 4 smoke + 1 version); 0 failed
- `cargo build --release`: succeeds; binary 884K (unchanged — mock is dev-only)

### Build-phase reflection

1. **What was unclear that slowed you down?** The `axum::body::Bytes` path doesn't exist in axum 0.8 as a standalone extractor alias; `bytes::Bytes` (already imported) is the correct type. Minor compile-error cycle.
2. **Constraint or decision that should have been listed but wasn't?** `no-new-top-level-deps-without-decision` technically applies to `serde` and `serde_json` as dev-deps; they're obviously justified by the handler code but weren't called out in the Frame prompt. Worth noting for future specs that axum handlers require `serde` directly.
3. **If you did this task again, what would you do differently?** Pre-check which serde features axum exposes transitively vs. which need direct listing — would have saved one compile cycle.

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
