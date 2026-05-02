---
task:
  id: SPEC-009
  type: story
  cycle: ship
  blocked: false
  priority: high
  complexity: S
  estimated_hours: 3

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
  decisions: [DEC-005]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-007, SPEC-010, SPEC-011]

value_link: "infrastructure enabling SPEC-010/011 download/upload to avoid per-chunk allocation on the hot path"

cost:
  sessions:
    - cycle: design
      date: 2026-05-02
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "Spec authoring + Frame critique in single Sonnet session"
    - cycle: build
      date: 2026-05-02
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 1643627
      tokens_output: 11925
      estimated_usd: 1.3534
      note: "Buffer pool implementation"
    - cycle: verify
      date: 2026-05-02
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 809300
      tokens_output: 8074
      estimated_usd: 0.9004
      note: "Buffer pool verification"
    - cycle: ship
      date: 2026-05-02
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: 4230244
      tokens_output: 28883
      estimated_usd: 2.8443
      note: "Buffer pool ship: merge, archive, stage backlog"
  totals:
    tokens_total: 6732053
    estimated_usd: 5.0981
    session_count: 4
---

# SPEC-009: Buffer pool implementation

## Context

STAGE-002 is 2 of 7 specs shipped: `MetricsAccumulator` (SPEC-007) and
the latency probe (SPEC-008). The next measurement specs — Cloudflare
download/upload (SPEC-010) and Generic HTTP download/upload (SPEC-011) —
need to read large chunks of data from network sockets without
allocating on every read. DEC-005 specifies the strategy: a pool of 8
`BytesMut` buffers, each 256KB, returned to the pool on drop via
`BytesMut::clear()`.

This spec implements the pool as a standalone crate-internal module
(`src/buffer_pool.rs`). It does not wire the pool into any download or
upload code — that's SPEC-010/011. SPEC-009's job is to get a correct,
well-tested pool in place so SPEC-010/011 can import it without having
to re-litigate the strategy.

## Goal

Implement `BufferPool` and `PooledBuffer` in `src/buffer_pool.rs`:

- `BufferPool::new(capacity, buf_size)` pre-allocates `capacity`
  `BytesMut` buffers, each with `buf_size` bytes of reserved capacity.
- `BufferPool::acquire()` pops a buffer non-blocking; returns `None` if
  the pool is empty.
- `PooledBuffer` wraps the leased buffer and implements
  `DerefMut<Target = BytesMut>` so callers can pass it directly to
  `tokio::io::AsyncReadExt::read_buf`.
- On `Drop`, `PooledBuffer` calls `BytesMut::clear()` on the buffer
  and returns it to the pool.

Both types are `pub(crate)` — the pool is an internal performance
mechanism, not part of the public `src/lib.rs` API.

## Inputs

- **`decisions/DEC-005-buffer-strategy.md`** — pool capacity (8),
  buffer size (256KB), return-on-drop via `BytesMut::clear()`, no
  allocation after warm-up
- **`Cargo.toml`** — `bytes = "1"` is already in `[dependencies]`;
  no new top-level dep is needed for this spec
- **`src/lib.rs`** — module declarations; this spec adds
  `pub(crate) mod buffer_pool;`

## Outputs

- **Files created:**
  - `src/buffer_pool.rs` — `BufferPool`, `PooledBuffer`,
    `DEFAULT_CAPACITY` (8), `DEFAULT_BUF_SIZE` (262144)
  - `tests/buffer_pool.rs` — integration tests (see **Failing Tests**)

- **Files modified:**
  - `src/lib.rs` — add `pub(crate) mod buffer_pool;`

- **Cargo.toml:** no changes. `bytes::BytesMut` is already in
  `[dependencies]`. `std::sync::Mutex` covers the pool implementation
  without a new dep (see **Implementation Context**).

## Acceptance Criteria

- [x] **AC-1: `BufferPool::new(capacity, buf_size)` pre-allocates.**
  After `new(8, 262144)`, the pool holds exactly 8 `BytesMut` values,
  each with `capacity() >= 262144`. Pre-allocation happens entirely in
  `new()` — no allocation occurs on the acquire/return hot path.

- [x] **AC-2: `acquire()` returns `None` when empty.**
  Calling `acquire()` exactly `capacity` times succeeds; the
  `(capacity + 1)`th call returns `None`. The pool itself may be queried
  via `available()` (see AC-5) to verify the count, but the primary
  invariant is behavioral: `None` on exhaustion.

- [x] **AC-3: `PooledBuffer` implements `DerefMut<Target = BytesMut>`.**
  A caller can do `pool.acquire().unwrap().read_buf(...)` where
  `read_buf` is the `tokio::io::AsyncReadExt` method that takes a
  `&mut impl BufMut`. `BytesMut` implements `BufMut`; exposing it
  through `DerefMut` makes `PooledBuffer` usable anywhere a `&mut
  BytesMut` is expected.

- [x] **AC-4: Buffer returns to pool on `Drop`; cleared on return.**
  Drop a `PooledBuffer`; immediately call `acquire()` again. It
  returns `Some(_)`. The re-acquired buffer has `len() == 0` (cleared)
  and `capacity() >= buf_size` (not reallocated). `clear()` is called
  by `PooledBuffer::drop`, not by the caller — callers do not need to
  manually clear before dropping.

- [x] **AC-5: `BufferPool::available()` reflects live pool state.**
  `available()` returns the number of buffers currently in the pool.
  After `new(8, 262144)`: `available() == 8`. After one `acquire()`:
  `available() == 7`. After the acquired `PooledBuffer` is dropped:
  `available() == 8`. This method is `pub(crate)` and used in tests.

- [x] **AC-6: No `unwrap`/`expect`/`panic` in library code except (a)
  `Mutex::lock().unwrap()` in `Drop`, which is documented, and (b)
  `Deref`/`DerefMut` accesses covered by `#![allow(clippy::unwrap_used)]`.**

- [x] **AC-7: `cargo clippy --all-targets -- -D warnings` and `cargo fmt
  --check` pass.** `tests/buffer_pool.rs` opens with
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` per the
  project test convention.

- [x] **AC-8: No new top-level deps.** `Cargo.toml` `[dependencies]`
  is unchanged. See **Implementation Context** for the dep decision.

- [x] **AC-9: Cloning a `BufferPool` produces a second handle to the
  same underlying pool.** `acquire()` on either handle draws from the
  same buffer set, and `available()` reflects the shared state.

## Failing Tests

Written during **design**. Build cycle makes these pass.

All live in `tests/buffer_pool.rs` which opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rspeed::buffer_pool::{BufferPool, DEFAULT_BUF_SIZE, DEFAULT_CAPACITY};
```

Wait — `buffer_pool` is `pub(crate)`, not `pub`. Integration tests in
`tests/` are a separate crate and cannot access `pub(crate)` items.
**Resolution:** expose `BufferPool`, `PooledBuffer`, `DEFAULT_CAPACITY`,
and `DEFAULT_BUF_SIZE` as `pub` from `src/buffer_pool.rs`, but do NOT
re-export them from `src/lib.rs` (omitting the re-export keeps them out
of the documented public API surface while remaining reachable from
integration tests via `rspeed::buffer_pool::BufferPool`). This is the
same pattern used for `rspeed::backend::latency` in SPEC-008.

The test file opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rspeed::buffer_pool::{BufferPool, DEFAULT_BUF_SIZE, DEFAULT_CAPACITY};
```

---

**`"capacity_respected"`** — `#[test]` (sync; no async needed). Constructs
`BufferPool::new(DEFAULT_CAPACITY, DEFAULT_BUF_SIZE)`. Calls `acquire()`
in a loop `DEFAULT_CAPACITY` times, collecting the handles. Asserts all
are `Some`. Calls `acquire()` one more time; asserts `None`.

**`"buffer_returns_on_drop"`** — `#[test]`. Constructs pool with
capacity 1. Acquires the one buffer (`assert!(handle.is_some())`). Drops
it (`drop(handle)`). Acquires again; asserts `Some`. Verifies
`pool.available() == 1` after the second drop.

**`"buffer_cleared_on_return"`** — `#[test]`. Acquires a buffer, writes
a known byte pattern into it via `buf.extend_from_slice(b"hello")`,
drops it. Re-acquires; asserts `buf.len() == 0` and
`buf.capacity() >= DEFAULT_BUF_SIZE`.

**`"buffer_size_at_least_default"`** — `#[test]`. Constructs a pool
with `DEFAULT_CAPACITY` and `DEFAULT_BUF_SIZE`. Acquires one buffer;
asserts `buf.capacity() >= DEFAULT_BUF_SIZE` and `buf.len() == 0`.
Verifies the buffer is usable: `buf.extend_from_slice(&[0u8; 1024])`;
assert `buf.len() == 1024`.

**`"available_tracks_acquire_and_drop"`** — `#[test]`. Constructs
`BufferPool::new(4, 4096)`. Asserts `available() == 4`. Acquires two
handles. Asserts `available() == 2`. Drops one handle; asserts
`available() == 3`. Drops the other; asserts `available() == 4`.

**`"deref_mut_allows_extend"`** — `#[test]`. Acquires a buffer. Uses
`*buf` (DerefMut) to call `buf.extend_from_slice(b"test")` directly
(i.e., calls it as if it were a `BytesMut` reference). Asserts
`buf.len() == 4`.

## Implementation Context

### Pool implementation: `std::sync::Mutex<Vec<BytesMut>>`

DEC-005 names two candidates: `crossbeam_queue::ArrayQueue<BytesMut>`
(lock-free, fixed-capacity) and `tokio::sync::Mutex<Vec<BytesMut>>`
(async-aware lock). A third option — `std::sync::Mutex<Vec<BytesMut>>`
— is strictly better for SPEC-009:

- **No new dep.** `bytes` is already in `[dependencies]`; `std::sync`
  is the standard library. Adding `crossbeam-queue = "0.3"` would require
  inline dep justification (constraint `no-new-top-level-deps-without-decision`,
  severity: warning) and adds a crate to the dep graph permanently.
- **No async overhead.** `tokio::sync::Mutex` is designed for futures
  that hold the guard across an `.await` point. We never hold the pool
  lock across an await — acquire is a single `Vec::pop()` under lock,
  and return is a single `Vec::push()` under lock. Using tokio's async
  mutex here is paying for async wakeup machinery we don't need.
- **Uncontended cost is identical.** An uncontended `std::sync::Mutex`
  lock is a single atomic CAS — the same order as `crossbeam_queue`'s
  `pop()`. With 8 buffers and 4 connections, the pool is rarely
  contended.
- **No capacity enforcement bug surface.** `ArrayQueue` enforces
  capacity at the queue level (push returns `Err(T)` if full). With
  `Vec`, we enforce capacity by never pushing more than we popped: the
  `Drop` impl pushes back exactly one buffer per acquired handle. The
  invariant is upheld by construction, not by the container.

If Stage 4 profiling shows Mutex contention is measurable (unlikely
given pool sizing), swapping to `crossbeam_queue::ArrayQueue` is a
contained change to `src/buffer_pool.rs` only.

**`tokio::sync::Mutex` explicitly rejected** — no benefit over
`std::sync::Mutex` for a non-async acquire pattern; adds the tokio
dep's async-mutex overhead for nothing.

### `PooledBuffer` internal structure

```rust
pub struct PooledBuffer {
    inner: Option<BytesMut>,
    pool: Arc<Mutex<Vec<BytesMut>>>,
}
```

`inner` is `Option` so the `Drop` impl can `take()` it without
a partial move. On `Drop`:

```rust
fn drop(&mut self) {
    if let Some(mut buf) = self.inner.take() {
        buf.clear();
        // PoisonError means the holding thread already panicked; re-panic.
        self.pool.lock().unwrap().push(buf);
    }
}
```

`Deref` and `DerefMut` delegate to `inner.as_ref().unwrap()` /
`inner.as_mut().unwrap()`. These unwraps are safe: `inner` is always
`Some` while the `PooledBuffer` is live (it's only `None` during
`drop()`). A debug-mode assert can document the invariant if desired.

### `BufferPool` structure

```rust
pub struct BufferPool {
    pool: Arc<Mutex<Vec<BytesMut>>>,
    buf_size: usize,
}
```

`new(capacity, buf_size)` pre-allocates:

```rust
let mut v = Vec::with_capacity(capacity);
for _ in 0..capacity {
    let mut buf = BytesMut::with_capacity(buf_size);
    // len == 0, capacity == buf_size; ready for read_buf calls
    v.push(buf);
}
```

`acquire()` locks, pops, and wraps:

```rust
pub fn acquire(&self) -> Option<PooledBuffer> {
    self.pool.lock().unwrap().pop().map(|buf| PooledBuffer {
        inner: Some(buf),
        pool: Arc::clone(&self.pool),
    })
}
```

`available()`:

```rust
pub fn available(&self) -> usize {
    self.pool.lock().unwrap().len()
}
```

### Constants

```rust
pub const DEFAULT_CAPACITY: usize = 8;
pub const DEFAULT_BUF_SIZE: usize = 256 * 1024; // 256KB per DEC-005
```

### Visibility note

`BufferPool` and `PooledBuffer` are `pub` in `src/buffer_pool.rs` but
`src/lib.rs` does NOT re-export them. This makes them reachable from
integration tests (`rspeed::buffer_pool::BufferPool`) and from
crate-internal modules (`crate::buffer_pool::BufferPool`) without
appearing in the public-facing API docs or `pub use` surface. This is
the same pattern as `src/backend/latency.rs` in SPEC-008.

### Performance note (DEC-005 Consequences, §1)

After warm-up (the first `TestSession::run()` invocation), no
allocations occur in the read loop: `acquire()` pops a pre-allocated
`BytesMut`, `read_buf()` fills it in-place (no grow needed since
`capacity >= buf_size`), and `Drop` clears and returns it. The 2MB of
fixed buffer pressure is accounted for in DEC-005's RSS estimate
(9–13MB idle, 12–15MB peak, 5–8MB headroom to the 20MB budget).

## Build Completion

**Date:** 2026-05-02  
**Agent:** claude-sonnet-4-6

Implemented `src/buffer_pool.rs` and `tests/buffer_pool.rs` exactly per the
Implementation Context. The `std::sync::Mutex<Vec<BytesMut>>` approach kept
the dep graph clean (no new crates needed). The `Option<BytesMut>` inner field
for `PooledBuffer` made the `Drop` impl straightforward — `take()` avoids the
partial-move problem cleanly. Added `impl Clone for BufferPool` (Frame E)
delegating to `Arc::clone`, enabling the `clone_shares_pool` test to verify
shared state across handles. All 7 integration tests pass; clippy and fmt clean.

## Verification Results

**Date:** 2026-05-02
**Agent:** claude-sonnet-4-6
**Cycle:** verify

### Checklist

| Check | Result |
|---|---|
| AC-1: pre-allocates in `new()` | ✅ |
| AC-2: `acquire()` returns `None` when empty | ✅ |
| AC-3: `DerefMut<Target = BytesMut>` | ✅ |
| AC-4: buffer returns on drop, cleared | ✅ |
| AC-5: `available()` reflects live state | ✅ |
| AC-6: only two documented unwrap sites | ✅ |
| AC-7: clippy + fmt clean | ✅ |
| AC-8: no new top-level deps | ✅ |
| AC-9: clone shares pool | ✅ |
| Frame A: types `pub`; module `pub mod`; no re-export | ✅ |
| Frame B: `#![allow(clippy::unwrap_used)]` + invariant comment | ✅ |
| Frame D: AC-6 enumerates both allowed unwrap sites | ✅ |
| Frame E: `impl Clone` + `clone_shares_pool` test | ✅ |
| `Drop` calls `buf.clear()` before push | ✅ |
| `new()` pre-allocates all buffers (no lazy alloc) | ✅ |
| `DEFAULT_CAPACITY == 8`, `DEFAULT_BUF_SIZE == 262144` | ✅ |
| 7 required tests present and named correctly | ✅ |
| `cargo test --all-targets` — all pass | ✅ (60 tests total) |
| Test file opens with `#![allow(...)]` | ✅ |
| `cargo clippy --all-targets -- -D warnings` — clean | ✅ |
| `cargo fmt --check` — clean | ✅ |
| `Cargo.toml [dependencies]` unchanged | ✅ |
| No unwrap/expect/panic outside documented exceptions | ✅ |
| No re-export from `lib.rs` top-level | ✅ |
| Standalone — no wiring into download/upload | ✅ |
| No modifications to out-of-scope files | ✅ |

### Verdict

✅ APPROVED — all ACs met, all 7 tests pass, lints clean, no issues.

## Reflection (Ship)

**What went well or was easier than expected?**

The `Option<BytesMut>` inner field pattern for `PooledBuffer` was exactly the right choice — it made the `Drop` impl trivial (`take()` avoids partial-move entirely) and the Deref/DerefMut impls a one-liner each. The decision to use `std::sync::Mutex<Vec<BytesMut>>` instead of `crossbeam_queue` or `tokio::sync::Mutex` eliminated a dependency and kept the implementation in stdlib. The spec's Implementation Context was detailed enough that Build had no ambiguity about structure, making the cycle fast and clean.

**What was harder, surprising, or required a Frame correction?**

The visibility mismatch was the key design correction the spec had to document (Frame A): `pub(crate)` cannot be accessed from integration tests in `tests/`, which live in a separate crate. The resolution — make types `pub` in `buffer_pool.rs` but omit them from `lib.rs`'s `pub use` block — keeps them off the documented API surface while remaining reachable from `tests/`. This same pattern was already established by SPEC-008 (`rspeed::backend::latency`), so no new convention was needed. The Frame critique also added `impl Clone for BufferPool` (not in the original AC list), which SPEC-010/011 will need to clone the pool handle into async tasks — a genuine omission caught early.

**What should SPEC-010/011 know about the buffer pool API?**

- **`acquire()` is non-blocking and returns `None` immediately if the pool is exhausted.** Callers must handle the `None` case — the design intent is to back-pressure or fall back to a smaller read, not to block waiting for a buffer. SPEC-010/011 should decide their exhaustion strategy before implementing.
- **`clone()` shares the same underlying pool.** Clone the `BufferPool` once per task/connection at startup; each clone draws from and returns to the same `Arc<Mutex<Vec<BytesMut>>>`. Do not call `BufferPool::new()` multiple times — that creates independent pools and defeats the budget.
- **The buffer comes back with `len() == 0` but `capacity() >= DEFAULT_BUF_SIZE`.** Callers can pass it directly to `AsyncReadExt::read_buf` without any setup. Do not call `clear()` before use — it's already been cleared on return.
- **Do not hold a `PooledBuffer` across an `.await` if it can be avoided.** The `Mutex` inside is a `std::sync::Mutex` (not async-aware). Holding the lock across an await point is not a concern (the lock is only held during `acquire`/`drop`, not while the buffer is in use), but holding the `PooledBuffer` itself across a long await ties up a pool slot for the duration. Drop it as soon as the data is consumed.
