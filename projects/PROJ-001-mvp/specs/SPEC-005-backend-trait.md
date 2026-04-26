---
task:
  id: SPEC-005
  type: story
  cycle: frame
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
  related_specs: [SPEC-002, SPEC-004]

value_link: "delivers STAGE-001's backend seam — the trait Stage 2 fills with real measurement code"

cost:
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-005: Backend trait and concrete stubs

## Context

Fifth spec under STAGE-001. With CLI parsing producing a `Config`,
we define the `Backend` trait — the seam between "what we measure"
and "where we measure against" — with two concrete implementations
that return `Err(NotImplemented)` for now. Wire selection from
`Config`. This is structural code only; no measurement logic.

The trait shape is intentionally provisional and will evolve in
STAGE-002 as the throughput layer reveals concrete needs (e.g., a
`connection_factory()` method). Don't preemptively add anything.

## Goal

`src/backend/mod.rs` defines the `Backend` trait, request/response
types, and a `BackendError` enum. `cloudflare.rs` and `generic.rs`
provide stub implementations whose methods return
`Err(NotImplemented)`. `select.rs` provides a `select(&Config) ->
Box<dyn Backend>` factory. `lib::run()` calls `select()` after
building `Config` and prints the chosen backend's name.

## Inputs

- **Files to read:**
  - `DEC-003` (backend abstraction with two impls)
  - `src/config.rs` and `src/cli.rs` from SPEC-004
  - `src/lib.rs` from SPEC-004

## Outputs

- **Files created:**
  - `src/backend/mod.rs` — trait + types + re-exports
  - `src/backend/cloudflare.rs` — CloudflareBackend stub
  - `src/backend/generic.rs` — GenericHttpBackend stub
  - `src/backend/select.rs` — factory function
- **Files modified:** `src/lib.rs` (calls `select()`, prints `backend.name()`)

## Acceptance Criteria

- [ ] `src/backend/mod.rs` defines:
  - The `Backend` trait
  - The `DownloadOpts`, `UploadOpts` input types
  - The `DownloadStream`, `UploadResult` output types
  - The `BackendError` enum (initial variants: `NotImplemented`,
    `Network(reqwest::Error)`, `Protocol(String)`)
- [ ] `src/backend/cloudflare.rs` defines `CloudflareBackend` with
      a hardcoded base URL `https://speed.cloudflare.com`. All trait
      methods return `Err(BackendError::NotImplemented)`. Implements
      `Default`.
- [ ] `src/backend/generic.rs` defines `GenericHttpBackend` with a
      `new(base_url: Url)` constructor. All trait methods return
      `Err(BackendError::NotImplemented)`.
- [ ] `src/backend/select.rs` defines:
  ```rust
  pub fn select(config: &Config) -> Box<dyn Backend>;
  ```
  Returns `CloudflareBackend::default()` if `config.server.is_none()`,
  otherwise `GenericHttpBackend::new(url)`.
- [ ] `lib::run()` now calls `select()` after building `Config`,
      prints `Backend: {name}` (using `backend.name()`), and exits 0
- [ ] Unit tests in `src/backend/select.rs` (or a `tests` module)
      verify:
  - `select()` with no server returns a backend whose `name() == "cloudflare"`
  - `select()` with a server URL returns a backend whose `name() == "generic"`
- [ ] An update or amendment documents the trait shape — either amend
      `DEC-003` (preferred for in-place evolution) or write
      `decisions/DEC-009-backend-trait-shape.md` (preferred if shape
      diverges meaningfully from DEC-003's sketch). Pick one in design.

## Failing Tests

- **`src/backend/select.rs` tests** (or a sibling file)
  - `"select chooses cloudflare with no server"` — `select(&Config{ server: None, .. })` returns backend with `name() == "cloudflare"`
  - `"select chooses generic with server"` — `select(&Config{ server: Some(url), .. })` returns backend with `name() == "generic"`
- **`tests/cli.rs`** (new snapshot)
  - `"prints backend with no server"` — `rspeed` stdout includes `"Backend: cloudflare"`
  - `"prints backend with server"` — `rspeed --server https://example.com` stdout includes `"Backend: generic"`

## Implementation Context

### Decisions that apply

- `DEC-003` — backend abstraction shape. Implement as sketched, with
  the caveat that STAGE-002 may extend the trait.

### Constraints that apply

- `test-before-implementation` — tests above are written first.
- `no-new-top-level-deps-without-decision` — `async-trait` is a new
  dep if used; justified by DEC-003. AFIT (return-position-impl-trait
  in traits) may avoid it but currently has friction with `Send` bounds
  on `dyn Trait` — discussed below.

### Prior related work

- SPEC-004 produced `Config` with optional `server: Option<Url>`. This
  spec consumes it to dispatch to a backend.

### Out of scope

- Any actual HTTP traffic — Stage 2
- The metrics accumulator and snapshot types — Stage 2
- Per-backend protocol details (request shapes, header handling) —
  Stage 2

## Notes for the Implementer

### Trait sketch

The shape will evolve in Stage 2. Start with:

```rust
use async_trait::async_trait;       // OR use AFIT if MSRV permits
use bytes::Bytes;
use futures::stream::BoxStream;
use std::time::Duration;

#[async_trait]
pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;

    async fn latency_probe(
        &self,
        samples: usize,
    ) -> Result<Vec<Duration>, BackendError>;

    async fn download(
        &self,
        opts: &DownloadOpts,
    ) -> Result<DownloadStream, BackendError>;

    async fn upload(
        &self,
        opts: &UploadOpts,
    ) -> Result<UploadResult, BackendError>;
}

pub type DownloadStream = BoxStream<'static, Result<Bytes, BackendError>>;

pub struct DownloadOpts {
    pub bytes_per_request: u64,
    pub connections: u8,
}

pub struct UploadOpts {
    pub bytes_per_request: u64,
    pub connections: u8,
}

pub struct UploadResult {
    pub bytes_sent: u64,
    pub elapsed: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("not yet implemented")]
    NotImplemented,
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}
```

### `async_trait` vs AFIT

Async functions in traits (AFIT) stabilized in Rust 1.75 and are the
modern approach. The friction point is `Send` bounds on the returned
futures — for a `dyn Backend`, you need `+ Send` bounds, which is
currently more ergonomic with `#[async_trait]` than with raw AFIT +
return-position-impl-trait syntax. **For Stage 1, use `async_trait`.**
We can migrate later in its own spec if AFIT ergonomics improve.

### Why `BoxStream` for download

The throughput meter in Stage 2 will consume bytes as they arrive.
Returning a stream gives the backend latitude to use any underlying
transport (HTTP/2 multiplex, multiple HTTP/1.1 connections, etc.) and
lets the consumer count bytes uniformly.

### Backend selection

Keep `select()` simple. Don't introduce a `BackendKind` enum yet —
the two concrete types and a function returning `Box<dyn Backend>` is
enough. Generalize when there's a third backend.

### File layout

```
src/backend/
├── mod.rs         # trait + types + re-exports
├── cloudflare.rs  # impl
├── generic.rs     # impl
└── select.rs      # factory function
```

`src/lib.rs` adds `pub mod backend;` and re-exports `Backend`,
`BackendError`.

### Shared `reqwest::Client` configuration

Both backends construct a `reqwest::Client` with one non-default
(per DEC-002):

```rust
reqwest::Client::builder()
    .no_proxy()             // ignore HTTP_PROXY/HTTPS_PROXY env vars
    .build()
```

We deliberately do **not** set `https_only(true)` even though the
default Cloudflare URL is HTTPS — the Generic backend may legitimately
target an internal `http://` test server (the SPEC-006 mock server is
plain HTTP for fixture simplicity). Protocol enforcement happens at
the URL level, not the client level.

The Generic backend additionally **caps response size** (e.g.,
`response_max_size: 10GB`) when reading download streams, so a
misbehaving or hostile custom server cannot make rspeed run for an
hour by reporting an absurd `Content-Length`. Implementation lives
in the streaming reader, not the backend trait.

For SPEC-005 the stub `Client` is constructed but not actually used
(all methods return `NotImplemented`). STAGE-002 wires it up.

### Trait-shape evolution warning

Stage 2 will likely need to add a method like `connection_factory()`
so the throughput layer can open new connections without going through
`download()`/`upload()`. **Don't preemptively add that here** — wait
until Stage 2 has a concrete need.

The Generic backend's URL contract (`/download?bytes=N`, `/upload`,
`/ping`) is documented in DEC-003 but not exercised by the stub.
That's fine — the documentation is the public contract; implementation
catches up in Stage 2.

---

## Build Completion

- **Branch:**
- **PR:**
- **All acceptance criteria met?** <not yet built>
- **New decisions emitted:**
- **Deviations from spec:**
- **Follow-up work identified:**

### Build-phase reflection

1. **What was unclear that slowed you down?** —
2. **Constraint or decision that should have been listed but wasn't?** —
3. **If you did this task again, what would you do differently?** —

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
