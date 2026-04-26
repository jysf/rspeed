---
insight:
  id: DEC-005
  type: decision
  confidence: 0.75
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
  - performance
  - memory
  - io
---

# DEC-005: Buffer strategy — BytesMut pool with 256KB reads

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

To saturate 1 Gbps on modest hardware we need to avoid:

- Per-chunk allocation in the read loop
- Small reads that round-trip syscalls too often
- Copying bytes more than necessary in user space

Note that "zero-copy" in the strict sense (sendfile/splice) does not
apply on the *receiving* side, which is the dominant hot path for
download throughput. The kernel-to-userspace copy is unavoidable for
HTTPS reads (since the bytes get decrypted in user space by rustls).
What we *can* control is allocation pressure and read granularity.

For uploads, we generate the bytes (e.g. zeros from a pre-allocated
buffer) and stream them through reqwest's body API. Same buffer pool
applies.

## Decision

Use a `bytes::BytesMut`-based buffer pool:

- Pool capacity: 8 buffers × 256KB = 2MB. Sized so 4 parallel
  connections each have ≤ 2 buffers in flight without contention,
  with headroom for the upload generator and the latency probe.
- Read granularity: 256KB per `read_buf` call. Large enough to
  amortize syscall overhead, small enough to keep peak RSS predictable
  and to give the metrics accumulator timely updates.
- Pool implementation: a thin wrapper around a `crossbeam_queue::ArrayQueue`
  (or a tokio `Mutex<Vec<BytesMut>>`, profiled to pick the cheaper
  option in Stage 2).
- Buffers are returned to the pool on drop via `BytesMut::clear()`
  rather than freed.

For uploads we allocate one read-only `Bytes` of zeros at startup and
clone it (cheap — `Bytes` is reference-counted) for each request body.

## Consequences

- After warm-up, no allocations in the read loop.
- 2MB of fixed buffer pressure fits inside the 20MB RSS budget with
  comfortable (not generous) headroom: rough estimate is 9–13MB at
  idle and 12–15MB peak during a test, leaving 5–8MB of cushion. Pool
  sizing increases here eat directly into that cushion.
- The pool size is a tuning parameter we may adjust in Stage 4 perf
  work. If 1 Gbps requires more buffers in flight, we revisit.
- The terminology "zero-copy" is avoided in the codebase and the
  README — we say "buffer-pooled streaming" instead, since the
  receiving-side copy is real.
