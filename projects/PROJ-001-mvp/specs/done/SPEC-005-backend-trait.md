---
task:
  id: SPEC-005
  type: story
  cycle: ship
  blocked: false
  priority: high
  complexity: M
  estimated_hours: 3-4

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
  sessions:
    - cycle: frame
      date: 2026-04-26
      agent: claude-opus-4-7
      interface: claude-ai
      tokens_total: null
      estimated_usd: null
      note: "Frame critique with 12 inline edits; all approved and folded into Build"
    - cycle: build
      date: 2026-04-26
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "Build session; all AC met; 16 tests passing"
    - cycle: verify
      date: 2026-04-27
      agent: claude-opus-4-7
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "✅ APPROVED; CI green 3 OSes + cross-check; cross-spec consistency sweep clean; 884K binary flagged as vacuous AC pass (real budget check moves to STAGE-002)"
    - cycle: ship
      date: 2026-04-26
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "Ship session 2026-04-26; folded verify bookkeeping, answered Ship reflections, updated stage backlog and timeline, archived spec"
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 4
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
  - The `DownloadOpts`, `UploadOpts` input types (both `#[non_exhaustive]` with `pub fn new(...)` constructors)
  - The `DownloadStream`, `UploadResult` output types (`UploadResult` also `#[non_exhaustive]` with constructor)
  - The `BackendError` enum (initial variants: `NotImplemented`, `Network(reqwest::Error)`, `Protocol(String)`; marked `#[non_exhaustive]`)
- [ ] `src/backend/cloudflare.rs` defines `CloudflareBackend` with
      a hardcoded base URL `https://speed.cloudflare.com`. All trait
      methods return `Err(BackendError::NotImplemented)`. Implements
      `Default`.
- [ ] `src/backend/generic.rs` defines `GenericHttpBackend` with a
      `new(base_url: Url)` constructor. All trait methods return
      `Err(BackendError::NotImplemented)`.
- [ ] `src/backend/select.rs` defines:
  ```rust
  pub fn select(config: &Config) -> Box<dyn Backend + Send + Sync>;
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
- [ ] `Cargo.toml` adds the runtime deps:
  ```toml
  [dependencies]
  tokio       = { version = "1", default-features = false, features = ["rt-multi-thread", "net", "time", "macros", "io-util", "sync"] }
  reqwest     = { version = "0.13", default-features = false, features = ["rustls", "stream", "http2"] }
  bytes       = "1"
  futures     = "0.3"
  thiserror   = "2"
  async-trait = "0.1"
  ```
- [ ] `[lints.clippy]` block added to `Cargo.toml` with `unwrap_used`, `expect_used`, `panic`, `unreachable` all as `"warn"`
- [ ] `Cargo.lock` committed (project policy from SPEC-002)
- [ ] `select()` returns `Box<dyn Backend + Send + Sync>` (auto-trait bounds must be explicit on `dyn` types)
- [ ] `BackendError` is `#[non_exhaustive]`
- [ ] `DownloadOpts`, `UploadOpts`, `UploadResult` all `#[non_exhaustive]`
- [ ] Stripped release binary <5MB on macOS arm64 (expected 3.5–4.5MB after this dep wave)
- [ ] This is the v0.1 public library API surface; renames/removals post-v0.1.0 are breaking changes and require a major version bump

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

STAGE-002 must send `Accept-Encoding: identity` on download requests so servers don't compress (compressed bodies inflate the throughput count vs on-wire bytes — see DEC-002). SPEC-005 stubs don't make requests, but the requirement belongs here so the trait's first real consumer doesn't forget.

The Generic backend additionally **caps response size** (e.g.,
`response_max_size: 10GB`) when reading download streams, so a
misbehaving or hostile custom server cannot make rspeed run for an
hour by reporting an absurd `Content-Length`. Implementation lives
in the streaming reader, not the backend trait.

For SPEC-005 the stub `Client` is constructed but not actually used
(all methods return `NotImplemented`). STAGE-002 wires it up.

### Trait-shape evolution warning

The trait shape is provisional. STAGE-002 may add `connection_factory()`, may refactor `latency_probe` to return `LatencyResult` directly, and may convert `upload` to a stream. Document any such evolution by amending DEC-003 (preferred) or emitting a new DEC.

Stage 2 will likely need to add a method like `connection_factory()`
so the throughput layer can open new connections without going through
`download()`/`upload()`. **Don't preemptively add that here** — wait
until Stage 2 has a concrete need.

The Generic backend's URL contract (`/download?bytes=N`, `/upload`,
`/ping`) is documented in DEC-003 but not exercised by the stub.
That's fine — the documentation is the public contract; implementation
catches up in Stage 2.

---

## Frame outcomes folded into Build (2026-04-26)

All 12 inline edits from the Frame critique (Opus, 2026-04-26) were approved and folded into this Build session:

1. **(A) async_trait kept** — AFIT for `dyn Backend + Send` still requires per-method `+ Send` annotations; `#[async_trait]` is cleaner for now. Migration is a one-PR refactor later.
2. **(B) Trait shape locked with provisional-evolution note** — Added to Trait-shape evolution warning section above.
3. **(C) `BackendError` `#[non_exhaustive]` + orchestrator-translation doc comment** — Variants `Timeout`/`Cancelled` deferred to STAGE-002/003.
4. **(D) `Accept-Encoding: identity` note for STAGE-002** — Added to Shared reqwest::Client configuration section above.
5. **(E) Public API surface + `#[non_exhaustive]` on opts/result types** — All opts/result types have `#[non_exhaustive]` and `pub fn new(...)` constructors. v0.1 surface acknowledged.
6. **(F) Dep versions verified** — tokio 1, reqwest 0.13, bytes 1, futures 0.3, thiserror 2, async-trait 0.1 (verified current at Frame 2026-04-26).
7. **(G) Lib-side clippy discipline as `warn`** — `[lints.clippy]` block added to `Cargo.toml`.
8. **(H) `select()` returns `Box<dyn Backend + Send + Sync>`** — Auto-trait bounds must be explicit on `dyn` types even when `Backend: Send + Sync`.
9. **(I) Cross-spec consistency** — Frame sweep was clean; no spec body edits needed beyond DEC-003 update.
10. **(J) Binary size AC with concrete range** — <5MB stripped on macOS arm64; expected 3.5–4.5MB. Recorded in Build Completion below.
11. **(K) AC enumeration** — Full checklist added to Acceptance Criteria above.
12. **(L) DEC-003 inline refinement** — `select()` return type updated; `#[non_exhaustive]` consequence bullet added.

---

## Build Completion

- **Branch:** `feat/spec-005-backend-trait`
- **PR:** pending (pushed for CI; PR created as draft)
- **All acceptance criteria met?** Yes — see checklist notes below
- **New decisions emitted:** DEC-003 inline refinement (select() return type + `#[non_exhaustive]` consequence bullet)
- **Deviations from spec:** Integration test files in `tests/` required `#![allow(clippy::unwrap_used, clippy::expect_used)]` to pass `cargo clippy --all-targets -- -D warnings`; the build prompt stated tests/ is exempt by default, which is incorrect — with `[lints.clippy]` + `-D warnings`, integration tests are also linted.
- **Follow-up work identified:** Binary size AC note — stripped release binary is 884K on macOS arm64, well below the expected 3.5–4.5MB range. This is because LTO + dead code elimination strips all tokio/reqwest/rustls code paths since the stubs never invoke them. The dep wave is present at type-check time; binary will grow substantially in STAGE-002 when real network code lands. Flag for Verify; STAGE-004 owns final size optimization.

### Build-phase reflection

1. **What was unclear that slowed you down?** The claim that `tests/` is exempt from clippy lints by default (in the build prompt) is incorrect when using `[lints.clippy]` + `-D warnings`. Took one iteration to discover and add `#![allow(...)]` to the two integration test files.
2. **Constraint or decision that should have been listed but wasn't?** The `[lints.clippy]` block applies to all targets including integration tests when combined with `-D warnings`. Should be noted in AGENTS.md: "integration test files in `tests/` should add `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top."
3. **If you did this task again, what would you do differently?** Pre-add the allow attribute to integration test files before the clippy run, saving one iteration.

---

## Verification Results (2026-04-27, claude-opus-4-7)

**Outcome: ✅ APPROVED — recommend Ship.**

### Acceptance criteria — all met

- `Cargo.toml` deps + features match the Frame-locked table (tokio 1.52.1, reqwest 0.13.2 with `["rustls", "stream", "http2"]`, bytes 1.11.1, futures 0.3.32, thiserror 2.0.18, async-trait 0.1.89). DEC-001 + DEC-002 features verified verbatim.
- `[lints.clippy]` block present with `unwrap_used`/`expect_used`/`panic`/`unreachable` all `warn`.
- `src/backend/mod.rs`: trait + `#[async_trait]` attribute + four type definitions with `#[non_exhaustive]` + constructors for opts/result + `BackendError` (also `#[non_exhaustive]`, with orchestrator-translation doc comment mapping Network → 3 / Protocol → 4 per AGENTS.md exit code table).
- `src/backend/cloudflare.rs` + `src/backend/generic.rs`: stubs return `Err(BackendError::NotImplemented)`; `name()` returns `"cloudflare"` / `"generic"` respectively. Cloudflare impls `Default`. Generic exposes `new(base_url: Url)` and `base_url() -> &Url`.
- `src/backend/select.rs`: `pub fn select(config: &Config) -> Box<dyn Backend + Send + Sync>` with explicit auto-trait bounds — load-bearing for STAGE-002 task spawning.
- `src/lib.rs`: `pub mod backend;` + re-exports of full v0.1 surface (`Backend`, `BackendError`, `DownloadOpts/UploadOpts/UploadResult`, `DownloadStream`, `CloudflareBackend`, `GenericHttpBackend`); `Cli` remains private (rustdoc sidebar shows only `fn run`, `mod backend`, `mod config`).
- `lib::run()` calls `backend::select()` after Config debug and prints `Backend: {name}`; verified via integration tests `backend_cloudflare_default` + `backend_generic_with_server` and via the updated insta snapshots (`snapshot_default_config`, `snapshot_custom_server_no_upload`, `snapshot_json_format_with_duration` all include the `Backend: <name>` line).
- 16 tests pass locally and in CI (2 unit select + 13 cli + 1 version).
- `Cargo.lock` committed with substantial growth from the dep wave.

### Behavioral AC

- CI green on all three OSes (run `24973223341`): `Test (macos-15)`, `Test (ubuntu-24.04)`, `Test (windows-2025)` — total wall 4m9s. The `Cross-check x86_64-apple-darwin` step inside `Test (macos-15)` ran and succeeded, validating cross-compile under the new dep wave (rustls + aws-lc-rs).

### 12 Frame outcomes + DEC-003 update — all faithfully applied

1. `#[async_trait]` used (not AFIT) — `src/backend/mod.rs:21`. ✓
2. Trait method signatures match Frame sketch + provisional-evolution sentence in spec body. ✓
3. `BackendError` `#[non_exhaustive]` with orchestrator-translation doc comment present. ✓
4. `Accept-Encoding: identity` STAGE-002 forward note present in spec body. ✓
5. `DownloadOpts`/`UploadOpts`/`UploadResult` all `#[non_exhaustive]` with `pub fn new(...)`. ✓
6. Dep versions match the verified table (see above). ✓
7. `[lints.clippy]` uses `warn` (not `deny`). ✓
8. `select()` returns `Box<dyn Backend + Send + Sync>` — explicit auto-trait bounds on the `dyn` type. ✓
9. Cross-spec wires clean (see consistency sweep below). ✓
10. <5MB AC technically met at 884K (see "Vacuous binary-size pass" note below). ✓
11. AC checklist enumerates Frame additions. ✓
12. DEC-003 inline refinement landed (line 86 shows the `+ Send + Sync` return type; line 108 shows the `#[non_exhaustive]` Consequences bullet). ✓

### Cross-spec consistency sweep — clean

- **DEC-003 ↔ select.rs:** DEC-003 sketch (line 86) and `src/backend/select.rs:16` both declare `pub fn select(config: &Config) -> Box<dyn Backend + Send + Sync>`. No drift.
- **DEC-002 ↔ Cargo.toml reqwest features:** `["rustls", "stream", "http2"]` exact match; `gzip` correctly absent.
- **DEC-001 ↔ Cargo.toml tokio features:** `["rt-multi-thread", "net", "time", "macros", "io-util", "sync"]` exact match. SPEC-005 stubs don't actually exercise these features yet (they don't call into reqwest/tokio), but the project-wide commitment is in place.
- **AGENTS.md exit code table ↔ `BackendError` doc comment:** doc comment maps Network → 3, Protocol → 4. ✓
- **SPEC-006 ↔ SPEC-005 surface:** `GenericHttpBackend::new(base_url: Url)` and `name() == "generic"` confirmed — SPEC-006's smoke test will compile against the SPEC-005 surface unchanged.
- **axum absent from SPEC-005 Cargo.toml:** confirmed; SPEC-006 (still in Frame) will land it as a dev-dep per the deferred School B cascade.

### Lib-side `unwrap`/`expect`/`panic`/`unreachable!` discipline — preserved

- One `.unwrap()` exists at `src/backend/select.rs:52` inside `#[cfg(test)] mod tests { ... }`, gated by an `#[allow(clippy::unwrap_used)]` attribute on the test module. The unwrap is on a hard-coded `Url::parse("https://example.com")` literal that cannot fail. The cfg-gate means it never compiles into the production library, preserving the AGENTS.md spirit ("no unwrap in lib code"). Acceptable and idiomatic — consistent with `tests/*.rs` integration tests that opt out via file-scope `#![allow]`.
- No other `unwrap`/`expect` and no `panic!`/`unreachable!` in lib code (`src/lib.rs`, `src/cli.rs`, `src/config.rs`, `src/backend/*.rs`).

### Vacuous binary-size pass — STAGE-002 awareness

The 884K stripped binary on macOS arm64 is well under the <5MB AC, but the AC effectively passes vacuously: LTO + dead-code elimination strips all tokio/reqwest/rustls code paths because the stubs never invoke them. Confirmed by inspection of `cloudflare.rs` and `generic.rs` (every method returns `Err(NotImplemented)`; no reqwest/tokio call sites exist yet). The meaningful binary-size budget check moves to **STAGE-002**, when `download()`/`upload()`/`latency_probe()` actually exercise reqwest. Not flagged as a punch-list item — LTO is doing its job; just documenting the implication so STAGE-002 doesn't get blindsided when the binary jumps from <1MB to ~3-5MB.

### Lint-scope finding — project-wide convention candidate

Build correctly handled an incorrect Frame assumption: `[lints.clippy] unwrap_used = "warn"` + `cargo clippy --all-targets -- -D warnings` **does** lint integration tests in `tests/*.rs`. Build's fix added `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of `tests/cli.rs` and `tests/version.rs` (file scope, preserving lib-side discipline). This is the right shape — lib code stays constrained; tests opt out at file boundary. Recommend adding a one-paragraph note to AGENTS.md's Style section on the next template revision pass so future specs don't rediscover this. Question proposal in MEMORY captures the rationale; not adding to `guidance/questions.yaml` to avoid noise.

### Constraint sweep

- `no-secrets-in-code`: ✓ trivial.
- `test-before-implementation`: ✓ 2 unit tests + 2 integration tests (backend selection) written; stubs return `NotImplemented` so tests assert on `name()` not implementation. Defensible.
- `no-new-top-level-deps-without-decision`: ✓ — tokio justified by DEC-001; reqwest by DEC-002; bytes/futures/thiserror/async-trait first-consuming-spec under School B with DEC-003 covering the trait shape that needs them. Spec body's "Frame outcomes" section enumerates each. No new DEC required.
- `one-spec-per-pr`: ✓ branch only touches SPEC-005-related files (DEC-003 inline refinement is in scope per spec AC).

### Cost discipline

`cost.sessions` had `frame` (Opus) and `build` (Sonnet) entries. Verify entry appended (Opus, claude-code).

### Recommendation

Promote PR #5 from draft to ready (`gh pr ready 5`). Bookkeeping edits (this section + timeline + cost) should be folded into Ship's commit per the SPEC-001/002/003/004 pattern.

---

## Reflection (Ship)

1. **What would I do differently next time?** — Frame's binary-size estimate of 3.5–4.5MB missed LTO+DCE behavior on stub-only implementations: all of tokio/reqwest/rustls was stripped to 884K because no code paths actually invoke those deps. Future stubs-only specs that land heavy deps should treat the binary-size AC as "won't grow until code paths are reachable" rather than predicting a post-dep-wave size. The meaningful <5MB check doesn't fire until STAGE-002 when `download()`/`upload()`/`latency_probe()` wire in real reqwest calls.

2. **Does any template, constraint, or decision need updating?** — Three updates worth capturing on the next AGENTS.md template pass: (a) Style section: a paragraph stating that `[lints.clippy]` combined with `cargo clippy --all-targets -- -D warnings` **does** apply to `tests/*.rs` integration test files; add `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the file scope in `tests/*.rs` to preserve lib-side discipline while allowing idiomatic test assertions. (b) Performance Budgets: a note that binary-size ACs pass vacuously in stubs-only specs — LTO + dead-code elim strips unreachable deps; the meaningful budget check is STAGE-002. (c) `#[non_exhaustive]` on public-API opts, result, and error types is now an established v0.1 contract for the project — worth a convention bullet in the spec template or the relevant AGENTS.md section so future specs follow the pattern without re-deriving it.

3. **Is there a follow-up spec to write now?** — No. SPEC-006 (mock server with axum as a dev-dep) is the natural next spec and is already drafted in Frame. When SPEC-006 ships, **STAGE-001 ship cycle fires**: write the stage-level reflection in STAGE-001-foundation.md, declare the foundational substrate complete, and STAGE-002 (Measurement core) unblocks.
