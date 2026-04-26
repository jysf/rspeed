---
insight:
  id: DEC-002
  type: decision
  confidence: 0.90
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
  - rust
  - http
  - tls
  - dependencies
---

# DEC-002: HTTP client — reqwest with rustls

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

rspeed makes HTTPS requests to Cloudflare and to user-supplied URLs.
We need an HTTP client that:

1. Streams response bodies (we measure throughput by counting bytes as
   they arrive, not by buffering full responses)
2. Supports HTTP/2 (Cloudflare prefers it; multiplexing is fine)
3. Uses TLS without requiring a system OpenSSL — we ship a single
   static binary on macOS, Linux, and Windows
4. Has a small, stable dependency graph

Three plausible options:

- **reqwest** — high-level, batteries-included, async, fork of hyper.
  Most common choice in the ecosystem.
- **hyper** directly — lower level, more control, more code to write.
- **ureq** — synchronous, won't fit our async + parallel-connection model.

## Decision

Use `reqwest` with `default-features = false`, opting into:

```toml
reqwest = { version = "0.12", default-features = false, features = [
    "rustls-tls",       # rustls instead of native-tls (no OpenSSL dep)
    "stream",           # body streaming for download throughput measurement
    "http2",            # HTTP/2 support
    "gzip",             # may be useful for some test endpoints
] }
```

Notably absent: `default-tls`, `native-tls`, `cookies`, `json` (we'll
serialize/deserialize directly with `serde_json`).

## Consequences

- Zero system TLS dependency — `cargo install` works on any reasonably
  modern Linux without `apt install libssl-dev` etc.
- Cross-compilation is straightforward (no OpenSSL cross-build issues).
- Slight binary size cost (rustls + ring/aws-lc-rs add ~1.5MB stripped),
  accepted for the deployability gain.
- We're tied to reqwest's API surface. If reqwest's streaming API
  changes between major versions, we adapt.
- Should we ever need to drop down for fine-grained connection control
  (e.g. for explicit HTTP/2 stream-per-connection counting), we can
  switch to `hyper` directly without much pain — the `Backend` trait
  isolates the HTTP client from the rest of the code.
