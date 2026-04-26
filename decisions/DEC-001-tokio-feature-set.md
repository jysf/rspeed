---
insight:
  id: DEC-001
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
  - async
  - dependencies
  - performance
---

# DEC-001: Tokio feature set

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —
**Superseded by:** —

## Context

Tokio is the async runtime for rspeed. Its default `full` feature set
pulls in everything (process spawning, signal handling, fs, sync
primitives, etc.) which inflates compile times, binary size, and
startup work. rspeed has a strict 50ms cold-start budget and a 20MB
peak RSS budget, so we want the minimum runtime that does our job.

The job: spawn tasks for parallel HTTP connections, read/write sockets,
hold timers for test duration, and use macros for `#[tokio::main]` and
`tokio::select!`. We do not need filesystem ops in the hot path,
process management, signals, or a `current_thread` runtime.

## Decision

Depend on `tokio` with these features only:

```toml
tokio = { version = "1", default-features = false, features = [
    "rt-multi-thread",  # multi-threaded runtime for parallel connections
    "net",              # TCP/UDP primitives (UDP reserved for future v2 ICMP/loss probe)
    "time",             # sleep, timeout, interval — needed for test duration
    "macros",           # #[tokio::main] and tokio::select!
    "io-util",          # AsyncReadExt / AsyncWriteExt traits
    "sync",             # watch/broadcast for snapshot fan-out (DEC-006), oneshot for graceful test-server shutdown (SPEC-006)
] }
```

If a future spec needs `signal` (graceful Ctrl-C handling) or `fs`,
that spec adds the feature in its own change with justification.

## Consequences

- Compile times stay short; cold start work is minimized.
- Binary size benefits modestly (~200-400KB savings vs `full`).
- Adding a tokio feature is now a deliberate decision visible in PR
  diffs, not a default.
- We accept the small ceremony of justifying each feature addition in
  the PR that introduces it.
- If we later need a single-threaded runtime variant (e.g. for very
  small environments), `rt` (without `multi-thread`) is a feature flag
  away.
