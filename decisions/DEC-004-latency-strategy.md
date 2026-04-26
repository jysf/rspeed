---
insight:
  id: DEC-004
  type: decision
  confidence: 0.85
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
  - latency
  - measurement
  - cross-platform
---

# DEC-004: Latency strategy — HTTP RTT primary, TCP-connect fallback

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

We need to measure latency to the test server. Three options:

- **ICMP ping.** Most accurate, but raw sockets need root on Linux
  (unless `net.ipv4.ping_group_range` is permissive), admin on Windows
  (or use `IcmpSendEcho`), and special handling on macOS (where
  unprivileged DGRAM ICMP works out of the box). For a tool whose
  pitch is "no system-wide deps, install with `cargo install`," ICMP
  is a privilege-escalation footgun.
- **TCP-connect RTT.** Open a TCP connection, measure SYN/ACK round
  trip, close. Gets through firewalls. Doesn't measure full
  application-path latency.
- **HTTP RTT.** Issue a tiny HTTP request (e.g. `HEAD /` or
  `GET /__health`) and measure the round trip. Includes TLS handshake
  effects on the first request (we exclude those from our number).
  This is the latency the throughput test will *actually* experience.

## Decision

Use **HTTP RTT as the primary measurement.** The probe:

1. Establish a TCP+TLS connection (warm-up — this RTT is discarded)
2. Issue N small `GET` requests to a designated low-cost endpoint
   (`/__ping` for Cloudflare, `/ping` for Generic backend) in series
3. Record per-request RTT
4. Compute median (reported as "latency"), mean, min, max, and
   sample standard deviation (reported as "jitter")

Default N = 10 samples. Configurable via `--latency-samples`.

**TCP-connect fallback** is used when HTTP RTT fails (e.g. server
returns 404 on the ping path, or HTTP layer error for any reason).
It opens N TCP connections in series, measures connect time, closes.
This is a degraded mode and is reflected in the JSON output at
`latency.method` (e.g., `"latency": {"method": "tcp_connect", ...}`)
— see DEC-006 for the canonical schema shape.

ICMP is **out of scope for MVP**. A future DEC may add it as an
opt-in `--icmp` flag with platform-specific handling.

## Consequences

- One less privilege concern. `cargo install rspeed && rspeed` works
  for an unprivileged user on all three OSes.
- The latency number reflects what the throughput test experiences,
  which is what most users actually care about.
- Latency is slightly inflated relative to ICMP because of HTTP and
  TLS overhead, but consistent across runs and across servers (good
  for relative comparison).
- The JSON schema includes `latency.method` (nested under
  `TestResult.latency` per DEC-006's `LatencyResult`) so monitoring
  scripts can detect when fallback was used.
- Future ICMP work is a clean addition: a third method, opt-in flag,
  no breakage to existing JSON consumers.
