---
task:
  id: SPEC-014
  type: bug
  cycle: ship
  blocked: false
  priority: high
  complexity: S

project:
  id: PROJ-001
  stage: STAGE-005
repo:
  id: rspeed

agents:
  architect: claude-sonnet-4-6
  implementer: claude-sonnet-4-6
  created_at: 2026-05-09

references:
  decisions: [DEC-002, DEC-003]
  constraints: [no-new-top-level-deps-without-decision]
  related_specs: [SPEC-010, SPEC-013]

value_link: "unblocks STAGE-005 release — Cloudflare backend must work before v0.1.0 ships"

cost:
  sessions:
    - cycle: design
      date: 2026-05-09
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "design+build+verify+ship in one session (bounded scope, explicit diagnostic decision tree)"
    - cycle: build
      date: 2026-05-09
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "combined session"
    - cycle: verify
      date: 2026-05-09
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "combined session"
    - cycle: ship
      date: 2026-05-09
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_input: null
      tokens_output: null
      estimated_usd: null
      note: "combined session"
  totals:
    tokens_total: null
    estimated_usd: null
    session_count: 1
---

# SPEC-014: Cloudflare download 403 fix

## Context

After STAGE-002 shipped the full measurement engine, `rspeed` returned
`HTTP 403` on the first download request against `speed.cloudflare.com`.
This blocked the binary from producing any useful output and is a
release blocker for STAGE-005.

An exhaustive diagnostic session ruled out five hypotheses before
identifying the root cause:

| Hypothesis | Tested via | Result |
|---|---|---|
| User-Agent absent | Added UA | 403 unchanged |
| `Accept-Encoding: identity` header | Removed header | 403 unchanged |
| Missing `during=download` query param | Added param | 403 unchanged |
| TLS fingerprinting (rustls JA3) | Switched to native-tls | 403 unchanged |
| HTTP/2 framing fingerprint | Forced HTTP/1.1 | 403 unchanged |
| **bytes param ≥ 100 MB** | **Tested with curl at boundary** | **403 → 200 at 99,999,999** |

Root cause: `DEFAULT_DOWNLOAD_BYTES_PER_REQUEST = 1_000_000_000` (1 GB)
exceeds Cloudflare's per-request limit. Requests for `bytes ≥ 100_000_000`
return 403; `bytes ≤ 99_999_999` return 200. Confirmed with curl at the
exact boundary.

Secondary issue (exposed by the fix): with `bytes_per_request = 25 MB`,
each round of 4 parallel connections drains in seconds on a fast link,
making the current single-round download design under-measure. The upload
phase already loops; this spec brings the download phase in line.

## Frame Critique (inline — design-through-ship pattern)

**A. Is 25 MB the right per-request value?**
Yes. 25 MB is well within the limit (75 MB of headroom), matches the
browser's observed file sizes for speed tests, and allows multiple rounds
within the default 10-second test duration on any link faster than 80
Mbps. Alternatives:
- 99 MB (max allowed): works but leaves only one round on fast links,
  producing a ≤7s measurement window after warmup on a 1 Gbps link.
- 10 MB: more rounds, slightly more connection overhead, no accuracy gain.

**B. Should the download loop mirror the upload loop exactly?**
Yes. The upload phase loops `while phase_start.elapsed() < duration`,
restarting connections when each round completes. The download phase
should do the same. The only difference: download reads a streaming
response (not a fire-and-forget POST), so the inner loop that reads
chunks remains; the outer `while` replaces the single-shot `loop`.

**C. Does the `during=download` param fix belong in this spec?**
Yes. This param is present in the browser's requests and is already in
the working tree. It causes no regressions and should ship with this fix.

**D. Does the UA change belong in this spec?**
Yes. Already in the working tree; harmless and good hygiene.

**E. Should `Accept-Encoding: identity` be restored?**
No. DEC-002's concern was that the `gzip` reqwest feature would enable
compression, inflating throughput numbers. Since we don't enable `gzip`,
reqwest sends no Accept-Encoding by default and servers don't compress.
The explicit header was defensive and is not needed. DEC-002's
Consequences section is updated to reflect this.

**F. What's the `live` feature shape?**
Minimal: one `[features]` entry in `Cargo.toml` (`live = []`), one test
file `tests/live_cloudflare.rs` gated on `#[cfg(feature = "live")]`.
One test: asserts `download.bytes > 0` and `upload.bytes > 0` against
the real Cloudflare endpoint. No CI job (nightly live tests are
mentioned in AGENTS.md; the test runs on-demand with
`cargo test --features live`).

## Goal

Fix `rspeed`'s default Cloudflare backend: change `bytes_per_request`
to 25 MB (below Cloudflare's 100 MB per-request cap), loop the download
phase like the upload phase, and add a `live`-gated integration test that
validates a real Cloudflare run.

## Inputs

- `src/orchestrator.rs` — `DEFAULT_DOWNLOAD_BYTES_PER_REQUEST` constant, `run_download_phase`
- `src/backend/cloudflare.rs` — client builder (UA, working-tree changes to keep)
- `src/backend/throughput.rs` — `build_download_url` (during=download, working-tree change to keep)
- `decisions/DEC-002-http-client.md` — Consequences section to amend
- `decisions/DEC-003-backend-abstraction.md` — Cloudflare backend notes to amend
- `guidance/questions.yaml` — add resolved entry
- `projects/PROJ-001-mvp/stages/STAGE-005-release.md` — update Cloudflare status

## Outputs

- **Files modified:**
  - `src/orchestrator.rs` — constant + download loop
  - `src/backend/cloudflare.rs` — keep UA; no TLS changes
  - `src/backend/throughput.rs` — keep `during=download`; no other changes
  - `Cargo.toml` — add `live = []` feature; no new runtime deps
  - `decisions/DEC-002-http-client.md` — Consequences amendment
  - `decisions/DEC-003-backend-abstraction.md` — bytes-limit note
  - `guidance/questions.yaml` — resolved cloudflare-403 entry
  - `projects/PROJ-001-mvp/stages/STAGE-005-release.md` — status update
- **Files created:**
  - `tests/live_cloudflare.rs` — live integration test
  - `projects/PROJ-001-mvp/specs/SPEC-014-cloudflare-download-403-fix.md` (this file)

## Acceptance Criteria

- [ ] AC-1: `cargo build --release && ./target/release/rspeed --format json` exits 0
        and produces a JSON object with non-null `download.bytes` and `upload.bytes`
        against the default Cloudflare endpoint.
- [ ] AC-2: `DEFAULT_DOWNLOAD_BYTES_PER_REQUEST = 25_000_000` (25 MB).
- [ ] AC-3: `run_download_phase` loops while `phase_start.elapsed() < duration`,
        restarting connections when the server closes the stream after sending
        bytes_per_request bytes.
- [ ] AC-4: All 78 existing tests pass: `cargo test --all-targets`.
- [ ] AC-5: `cargo clippy --all-targets -- -D warnings` clean.
- [ ] AC-6: `cargo fmt --check` clean.
- [ ] AC-7: `cargo test --features live` passes a new test in
        `tests/live_cloudflare.rs` that asserts `download.bytes > 0`
        and `upload.bytes > 0`.
- [ ] AC-8: DEC-002 Consequences section amended with a note that the
        `Accept-Encoding: identity` header was removed (gzip feature
        absent, servers don't compress test payloads).
- [ ] AC-9: DEC-003 amended with the Cloudflare `bytes < 100_000_000`
        limit and the `during=download` query param.
- [ ] AC-10: `guidance/questions.yaml` has a resolved entry for the 403 investigation.

## Failing Tests

Written during design (this is a combined session; tests were written
before build started and drove the implementation):

- **`tests/live_cloudflare.rs`**
  - `cloudflare_download_and_upload_return_nonzero_bytes` — asserts
    `result.download.unwrap().bytes > 0` and
    `result.upload.unwrap().bytes > 0` against `CloudflareBackend::new()`.
    Gated on `#[cfg(feature = "live")]`.

No new unit tests needed for the constant change; existing integration
tests (78 total) exercise the download loop through the mock server.

## Implementation Context

### Decisions that apply

- `DEC-002` — HTTP client: rustls, no gzip, no native-tls. This spec
  keeps rustls; no TLS change. Updates the Accept-Encoding note.
- `DEC-003` — Cloudflare backend protocol. Updates with bytes limit and
  `during=download` param.

### Constraints that apply

- `no-new-top-level-deps-without-decision` — no new runtime deps.
  `live = []` feature flag adds no deps.

### Prior related work

- `SPEC-010` (shipped) — Cloudflare real download/upload; established
  `download_parallel` / `upload_parallel` and `build_download_url`.
- `SPEC-013` (shipped) — Failure mode tests; established
  `DEFAULT_DOWNLOAD_DEADLINE` and `with_deadlines`.

### Out of scope

- ❌ TLS backend changes (native-tls, BoringSSL, rquest)
- ❌ Switching `GenericHttpBackend` to match any of these changes
- ❌ Fixing `/__ping` 404 (TCP fallback already works)
- ❌ `measId` query param for `__up` (separate investigation)
- ❌ Upload `mbps_p50 = 0.0` anomaly (pre-existing, separate spec)

## Notes for the Implementer

The core change is in `run_download_phase`. The original code had a single
`let mut stream = ...` then an inner `loop` reading chunks. The fix wraps
that in `while phase_start.elapsed() < duration`, hoisting the `phase_start`
declaration above the `while`. The inner `Ok(None) => break` now breaks
the inner loop to restart the outer `while`, rather than breaking all the
way out. No change to `Err(_)` (duration reached) or `Ok(Some(Err(e)))`
(network error) arms — both still terminate the phase.

---

## Build Completion

- **Branch:** `feat/spec-014-cloudflare-download-fix`
- **PR:** TBD
- **All acceptance criteria met?** yes
- **New decisions emitted:** none (DEC-002 and DEC-003 amended, not new)
- **Deviations from spec:**
  - None. The implementation matched the spec design exactly.
- **Follow-up work identified:**
  - Upload `mbps_p50 = 0.0` anomaly observed in live output — pre-existing, needs investigation in separate spec
  - `/__ping` returns 404 on Cloudflare; TCP fallback works but adding a `guidance/questions.yaml` note for STAGE-005

### Build-phase reflection

1. **What was unclear in the spec that slowed you down?**
   — Nothing unclear. The diagnostic decision tree in the context section
   was precise enough that the root cause was found before any code was written.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No. All relevant DECs were listed.

3. **If you did this task again, what would you do differently?**
   — Would have tested `curl /__down?bytes=1GB` in the initial diagnostic
   before trying TLS/UA/header fixes. A 60-second curl test would have
   saved the TLS investigation entirely.

---

## Reflection (Ship)

1. **What would I do differently next time?**
   — Start diagnostics by testing the exact production URL+params with curl
   before investigating TLS fingerprinting. A boundary-search with curl took
   under 5 seconds; the TLS investigation took multiple build cycles.

2. **Does any template, constraint, or decision need updating?**
   — DEC-002 and DEC-003 are updated. The diagnostic runbook pattern
   (test simplest hypothesis first with curl before code changes) is worth
   adding to the rspeed-specific section of AGENTS.md — but that's a
   template pass, not an urgent spec.

3. **Is there a follow-up spec I should write now before I forget?**
   — Upload `mbps_p50 = 0.0` with non-zero mean needs investigation.
   Add to `guidance/questions.yaml` rather than a spec for now.
