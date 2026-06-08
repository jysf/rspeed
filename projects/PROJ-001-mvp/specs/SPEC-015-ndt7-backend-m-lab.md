---
task:
  id: SPEC-015
  type: story
  cycle: design
  blocked: false
  priority: high
  complexity: M

project:
  id: PROJ-001
  stage: STAGE-005
repo:
  id: rspeed

agents:
  architect: claude-sonnet-4-6
  implementer: null
  created_at: 2026-05-09

references:
  decisions: [DEC-002, DEC-003]
  constraints: [no-new-top-level-deps-without-decision]
  related_specs: [SPEC-008, SPEC-010, SPEC-011, SPEC-014]

value_link: "makes rspeed generally usable without user-managed infrastructure — the core release blocker"

cost:
  sessions:
    - cycle: design
      date: 2026-05-09
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: ""
  totals:
    tokens_total: null
    estimated_usd: null
    session_count: 0
---

# SPEC-015: NDT7 backend (M-Lab)

## Context

SPEC-014 confirmed that `speed.cloudflare.com` rate-limits programmatic
clients (HTTP 429 on repeated runs). The Cloudflare endpoint is a
browser-facing service, not a public API. `rspeed` cannot use it as a
reliable default.

[Measurement Lab (M-Lab)](https://www.measurementlab.net/) runs
NDT7 — an open WebSocket-based speed test protocol designed explicitly
for programmatic, repeated use. It's used by Google, Chrome, Mozilla,
and ISPs for network diagnostics. No rate limits. Global server
coverage. Free.

NDT7 replaces Cloudflare as the no-args default. Cloudflare stays in
the codebase (it works for occasional use) but is no longer the default.

## Goal

Implement an `Ndt7Backend` that uses M-Lab's locate API to find the
nearest server, then runs download and upload over WebSocket (NDT7
protocol). Make it the default when `--server` is not supplied.

## NDT7 protocol summary

**Server discovery:**
```
GET https://locate.measurementlab.net/v2/nearest/ndt/ndt7
→ JSON with list of nearest servers, each with download/upload WSS URLs
```

**Download:** `GET wss://<server>/ndt/v7/download`
- Server streams binary frames (random bytes) for ~10 seconds
- Client counts bytes as they arrive (same as HTTP streaming)
- Server also sends text frames (JSON measurement updates) — ignore for MVP

**Upload:** `GET wss://<server>/ndt/v7/upload`
- Client sends binary frames continuously
- Server sends text frames with measurement updates — ignore for MVP
- Client tracks bytes sent and elapsed time

**Latency:** TCP connect probe to the server (existing `latency::probe`).

## Frame Critique

**A. Make `select()` async?**
Yes — `Ndt7Backend` construction requires an async network call (locate
API). `select()` is called from `async_run()` which is already async;
the cascade is one `await` in `lib.rs`. No other callers.

**B. Remove Cloudflare backend?**
No. Keep it — it works for occasional use and has tests. A future
`--backend cloudflare` flag can expose it explicitly. Nothing to delete.

**C. What happens to `connections` and `bytes_per_request` for NDT7?**
NDT7 uses a single WebSocket connection per phase (the protocol is
designed to saturate the link from one connection). `connections` is
ignored. `bytes_per_request` controls how many bytes each upload call
sends before returning; the orchestrator loops. For download, the
server streams until the orchestrator's duration timeout fires.
No changes to the Backend trait or orchestrator.

**D. Parse NDT7 measurement messages?**
No — MVP only. The server sends JSON text frames mid-stream with
server-side throughput estimates. We ignore them and measure
client-side throughput the same way as all other backends.
A follow-up spec can surface server-side measurements.

**E. TLS for WebSocket?**
`tokio-tungstenite` with `rustls-tls-webpki-roots` feature — rustls
with bundled Mozilla root certs. Consistent with DEC-002's no-system-
OpenSSL principle. M-Lab uses Let's Encrypt certs (in Mozilla roots).
No system cert store required.

**F. Mock WebSocket server for CI tests?**
Yes. `axum` (already in dev-deps) supports WebSocket. A minimal mock
NDT7 server (~40 lines) streams N bytes then closes (download) and
accepts frames then closes (upload). This keeps the NDT7 tests
in the same CI-runnable pattern as all other backend tests, with no
live network needed for `cargo test`.

## Inputs

- `src/backend/mod.rs` — `Backend` trait, `BackendError`, opts/result types
- `src/backend/select.rs` — backend selection logic (becomes async)
- `src/lib.rs` — `async_run`: one `await` change
- `Cargo.toml` — add `tokio-tungstenite`
- `decisions/DEC-003-backend-abstraction.md` — add NDT7 backend entry
- New DEC for NDT7 choice (DEC-009 or next available)

## Outputs

- **Files created:**
  - `src/backend/ndt7.rs` — `Ndt7Backend` struct + `Backend` impl
  - `tests/ndt7_backend.rs` — integration tests with mock WebSocket server
  - `decisions/DEC-00N-ndt7-backend.md` — records the NDT7 choice
- **Files modified:**
  - `src/backend/select.rs` — `select()` becomes `async`, adds NDT7 arm
  - `src/backend/mod.rs` — export `Ndt7Backend`
  - `src/lib.rs` — `async_run` awaits `select()`
  - `Cargo.toml` — add `tokio-tungstenite`
  - `decisions/DEC-003-backend-abstraction.md` — add NDT7 entry
  - `tests/live_cloudflare.rs` — add a live NDT7 test alongside existing

## Acceptance Criteria

- [ ] AC-1: `rspeed --format json` (no `--server`) exits 0 and produces
        a JSON object with non-null `download.bytes` and `upload.bytes`,
        using M-Lab's nearest server.
- [ ] AC-2: `rspeed --format json` can be run 10 times in a row without
        a non-0 exit code (no rate limiting).
- [ ] AC-3: `rspeed --server <url> --format json` still works (generic
        backend unaffected).
- [ ] AC-4: All existing tests pass: `cargo test --all-targets`.
- [ ] AC-5: New mock-server tests in `tests/ndt7_backend.rs` pass in CI
        (no live network needed).
- [ ] AC-6: `cargo test --features live` includes a live NDT7 test that
        asserts `download.bytes > 0` and `upload.bytes > 0`.
- [ ] AC-7: `cargo clippy --all-targets -- -D warnings` clean.
- [ ] AC-8: `cargo fmt --check` clean.
- [ ] AC-9: A new DEC records the NDT7 choice (protocol, library, why
        M-Lab over alternatives).
- [ ] AC-10: `backend.name()` returns `"ndt7"` for `Ndt7Backend`.

## Failing Tests

Write these at the start of build, before writing `ndt7.rs`:

- **`tests/ndt7_backend.rs`**
  - `ndt7_locate_parses_nearest_server_urls` — unit test: given a
    sample locate API JSON response, `parse_locate_response` returns
    the correct download/upload URLs.
  - `ndt7_download_happy_path` — mock axum WebSocket server streams
    1 MB of binary data then closes; assert stream yields those bytes.
  - `ndt7_upload_happy_path` — mock axum WebSocket server accepts
    frames then closes; assert `upload()` returns `bytes_sent > 0`.
  - `ndt7_backend_name_is_ndt7` — `Ndt7Backend::name()` returns `"ndt7"`.

## Implementation Context

### Key types and structure

```rust
// src/backend/ndt7.rs

pub struct Ndt7Backend {
    download_url: String,   // wss://... (String not Url — tungstenite takes &str)
    upload_url: String,
    tcp_target: String,     // host:443 for TCP latency probe
}

impl Ndt7Backend {
    /// Calls M-Lab locate API, returns backend pointed at nearest server.
    pub async fn locate() -> Result<Self, BackendError> { ... }
}
```

```rust
// src/backend/select.rs (updated)

pub async fn select(config: &Config)
    -> Result<Box<dyn Backend + Send + Sync>, BackendError>
{
    match &config.server {
        Some(url) => Ok(Box::new(GenericHttpBackend::new(url.clone())?)),
        None      => Ok(Box::new(Ndt7Backend::locate().await?)),
    }
}
```

### Locate API response shape

```json
{
  "results": [{
    "machine": "...",
    "location": {"city": "Chicago", "country": "US"},
    "urls": {
      "wss:///ndt/v7/download": "wss://ndt-iupui-mlab3-ord05.mlab-oti.measurement-lab.org/ndt/v7/download",
      "wss:///ndt/v7/upload":   "wss://ndt-iupui-mlab3-ord05.mlab-oti.measurement-lab.org/ndt/v7/upload"
    }
  }]
}
```

Parse with `serde_json` (already in deps). Take `results[0]`.
The URL keys contain literal slashes — use a `HashMap<String, String>`
for the `urls` field, then look up by key `"wss:///ndt/v7/download"`.

### Download implementation sketch

```rust
async fn download(&self, _opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(&self.download_url)
        .await
        .map_err(|e| BackendError::Network(e.into()))?;
    let (_, read) = ws_stream.split();
    let stream = read.filter_map(|msg| async {
        match msg {
            Ok(tungstenite::Message::Binary(data)) =>
                Some(Ok(Bytes::from(data))),
            Ok(_) => None,           // text frames (measurements) — ignore
            Err(e) => Some(Err(BackendError::Network(e.into()))),
        }
    });
    Ok(Box::pin(stream))
}
```

### Upload implementation sketch

Each `upload()` call opens one WebSocket, sends `opts.bytes_per_request`
bytes in 64 KB chunks, then closes. The orchestrator loops until duration
expires — same pattern as the HTTP upload loop.

```rust
async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(&self.upload_url)
        .await
        .map_err(|e| BackendError::Network(e.into()))?;
    let (mut write, _read) = ws_stream.split();
    let chunk = Bytes::from(vec![0u8; 65_536]);
    let mut sent = 0u64;
    let start = Instant::now();
    while sent < opts.bytes_per_request {
        let remaining = (opts.bytes_per_request - sent) as usize;
        let this_len = chunk.len().min(remaining);
        write.send(tungstenite::Message::Binary(chunk[..this_len].to_vec()))
            .await
            .map_err(|e| BackendError::Network(e.into()))?;
        sent += this_len as u64;
    }
    let _ = write.send(tungstenite::Message::Close(None)).await;
    Ok(UploadResult::new(sent, start.elapsed()))
}
```

### Cargo.toml addition

```toml
tokio-tungstenite = { version = "0.24", default-features = false,
    features = ["rustls-tls-webpki-roots"] }
```

No `native-tls`; consistent with DEC-002.

### Mock WebSocket server for tests

`axum` (already in dev-deps) supports WebSocket via `axum::extract::ws`.
A minimal mock that streams N bytes for download and accepts frames for
upload is ~40 lines. Follow the `MockServer` pattern from
`tests/common/mod.rs` — bind a random port, spin up axum, return the
base URL.

### Decisions that apply

- `DEC-002` — stay with rustls; `tokio-tungstenite` uses
  `rustls-tls-webpki-roots`, consistent.
- `DEC-003` — add NDT7 as third backend impl; `select()` becomes async.
- New DEC (check `decisions/` for next available number) — records NDT7
  protocol + M-Lab choice and why over alternatives.

### Out of scope

- ❌ Parsing NDT7 JSON measurement messages (server-side throughput)
- ❌ Exposing `--backend cloudflare` CLI flag
- ❌ Multiple parallel NDT7 connections (protocol uses one)
- ❌ Removing the Cloudflare backend

## Notes for the Implementer

`tokio-tungstenite::connect_async` returns `(WebSocketStream, Response)`.
Split with `futures::StreamExt::split()`. The `Message::Binary` variant
in tungstenite 0.21+ holds `Bytes` directly — check the actual type
before writing `Bytes::from(data)` and adjust if it's already `Bytes`.

If the locate API fails (network error, malformed JSON, empty results),
return `BackendError::Protocol(...)` with a message like
`"M-Lab locate failed: <reason>; check connectivity or use --server"`.

`backend.name()` should return `"ndt7"`.

The `tcp_target` field should be the hostname from the download URL
plus `:443` — e.g. `"ndt-iupui-mlab3-ord05.mlab-oti.measurement-lab.org:443"`.

---

## Build Completion

*To be filled in at the end of the build cycle.*

---

## Reflection (Ship)

*To be filled in at the end of the ship cycle.*
