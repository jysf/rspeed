---
insight:
  id: DEC-003
  type: decision
  confidence: 0.80
  audience:
    - developer
    - agent

agent:
  id: claude-opus-4-7
  session_id: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-25
supersedes: null
superseded_by: null

tags:
  - architecture
  - trait
  - abstraction
---

# DEC-003: Backend abstraction with two implementations

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

The "what we measure against" varies. The default backend is
Cloudflare's public speed-test endpoints (`speed.cloudflare.com/__down`
and `/__up`). Users may want to point rspeed at their own server or
a corporate test endpoint. The measurement code (downloader, uploader,
latency probe) should not care which backend it's using.

We also want to defend against Cloudflare changing its endpoint shape
or going away. A clean abstraction gives us a one-PR migration path.

## Decision

Define a `Backend` trait in `src/backend/mod.rs`:

```rust
#[async_trait::async_trait]  // or use AFIT once stable enough on MSRV
pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;

    async fn latency_probe(
        &self, samples: usize,
    ) -> Result<Vec<Duration>, BackendError>;

    async fn download(
        &self, opts: &DownloadOpts,
    ) -> Result<DownloadStream, BackendError>;

    async fn upload(
        &self, opts: &UploadOpts,
    ) -> Result<UploadResult, BackendError>;
}
```

Provide two impls:

- `CloudflareBackend` (`src/backend/cloudflare.rs`): hardcoded base URL
  `https://speed.cloudflare.com`. The endpoint shape (`/__down?bytes=N`,
  `/__up`) is the production contract — if Cloudflare changes it we
  ship a fixed minor version.

- `GenericHttpBackend` (`src/backend/generic.rs`): accepts any base URL
  via the `--server` flag. Implements a documented protocol (see the
  README under "Custom server protocol"):
  - `GET {base}/download?bytes=N` → returns N zero bytes
  - `POST {base}/upload` → consumes body, returns 200
  - `GET {base}/ping` → returns 200 quickly (for HTTP RTT)

Selection logic lives in `src/backend/select.rs`:

```rust
pub fn select(config: &Config) -> Box<dyn Backend + Send + Sync> {
    match &config.server {
        Some(url) => Box::new(GenericHttpBackend::new(url.clone())),
        None      => Box::new(CloudflareBackend::default()),
    }
}
```

## Consequences

- Measurement code (Stage 2) writes against `&dyn Backend`, never
  against a concrete type.
- Adding a new backend (e.g. M-Lab NDT, LibreSpeed protocol) is a new
  file plus a `select` arm — no changes to measurement.
- Test code uses a mock backend (the local axum server from SPEC-006)
  via the same trait.
- The trait shape may evolve in Stage 2 as we discover what download/
  upload need. That's fine — Stage 1 ships a stub trait, Stage 2
  refines it.
- The Generic backend's documented protocol is a public contract we
  commit to maintaining across minor versions. Any change requires a
  deprecation cycle.
- The trait + opts/result types use `#[non_exhaustive]` to keep STAGE-002 evolution semver-friendly. Adding a new `BackendError` variant or a new `DownloadOpts` field is non-breaking; renaming/removing existing items is breaking.
