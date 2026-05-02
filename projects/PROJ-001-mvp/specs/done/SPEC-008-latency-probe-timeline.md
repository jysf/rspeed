# SPEC-008 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-008-<cycle>.md`.

| # | Cycle | Status | Notes |
|---|---|---|---|
| 1 | design | [x] | Spec authored 2026-04-29 (claude-opus-4-7); Frame critique in same session — see `## Frame critique (2026-04-29)` in spec body. Architect resolved item (A-1) inline (drop Default, fallible constructors + select cascade). |
| 2 | frame  | [x] | Frame critique inline (see spec body); 1 substantive item flagged (backend construction fallibility) + 6 mechanical patches. GO conditional on architect ack of item (A) — resolved A-1. |
| 3 | build  | [x] | claude-sonnet-4-6 — all 8 Frame outcomes applied; 9 new latency tests + 34 prior all pass; binary 4.3MB. PR #10 opened (draft). |
| 4 | verify | [x] | claude-opus-4-7 — APPROVED. PR #10 CI green on all 3 OSes (macos-15, ubuntu-24.04, windows-2025); 13/13 ACs verified; fallibility cascade clean; DEC-003 refinement captured; paused-clock deviation rationale sound. Convention check: build cost entry backfilled via `just record-cost`. Promoted PR draft → ready. |
| 5 | ship   | [x] | claude-sonnet-4-6 — reflection written, cost entry appended, archived. Shipped 2026-05-01. |

## Cycle Log

### Design + Frame (2026-04-29, claude-opus-4-7)
- Combined spec-authoring + Frame critique session (Opus, 1M-context).
- Spec drafted with full ACs (13), failing tests (9), and implementation
  context including the shared `src/backend/latency.rs` helper sketch,
  `BackendError::Timeout(Duration)` variant shape, MockServer extension
  (`MockOptions` + `ping_count()`), and the trait-shape evolution
  (`latency_probe` returns `LatencyProbeOutcome` per SPEC-005's
  provisional-evolution note).
- Frame critique applied in the same session; verdict GO conditional on
  architect resolution of item (A) — backend construction fallibility,
  which cascades up to `select()` and `lib::run()`. 6 mechanical patches
  (B–G) flagged for inline fold into Build.
- Architect resolved A-1: drop `Default`, fallible constructors, fallible
  `select()`, propagate via `?` in `lib::run()`. Promoted to Build.

### Build (2026-04-29, claude-sonnet-4-6)
- All 8 Frame outcomes (A-1 + B–G) applied in a single commit on
  `feat/spec-008-latency-probe`.
- 9 new latency integration tests + 4 SPEC-006 smoke tests + 1 paused-clock
  test = 14 file-tests; full suite 43 pass; clippy clean; fmt clean.
- Binary size 4.3MB (under 5MB budget).
- Deviation captured: `http_probe_times_out_then_falls_back` uses real-time
  (~1s) instead of paused-clock — `tokio::time::advance` fires the TCP
  fallback's 1s timer set inside the advance window, causing spurious
  `Timeout`. Real-time is reliable and exercises the documented behavior.
- DEC-003 Consequences updated inline (latency_probe return type + fallible
  constructors).
- PR #10 opened as draft; build cost session entry backfilled via
  `just record-cost`.

### Verify (2026-05-01, claude-opus-4-7)
- ✅ APPROVED — all 13 ACs verified end-to-end against the code.
- Fallibility cascade clean: both backends use `reqwest::Client::builder()
  .no_proxy().build()?`; `select()` returns `Result`; `lib::run()`
  propagates with one `?`; no `Default` impl remains; no `unwrap`/`expect`
  in lib code (only test modules in `select.rs::tests` and `result.rs::tests`).
- `latency::probe()` `Err(other) => Err(other)` correctly propagates
  `NotImplemented` without falling back to TCP — preserves STAGE-002
  diagnostic clarity.
- `LatencyProbeOutcome` is `pub`, `#[non_exhaustive]`, `Debug + Clone`,
  re-exported from `src/lib.rs`. The `&'static str`/`String` split between
  `LatencyProbeOutcome.method` (compile-time-known) and `LatencyResult.method`
  (owned for serialization) is intentional — orchestrator (SPEC-012) bridges
  via `.to_string()`.
- DEC-003 Consequences carries both refinements with rationale
  (lines 109–127). DEC-006 contract (`latency.method = "http_rtt" |
  "tcp_connect"`) pinned by `latency_method_strings_match_dec004_contract`.
- Paused-clock deviation: rationale sound (timer-fires-during-advance-window
  is a known tokio-time gotcha; SPEC-007's DEC-008 documents the inverse
  case). Test exercises documented behavior; ~1s wall-clock is bounded.
- AC-11 backward-compat verified: `tests/smoke.rs` SPEC-006 tests pass
  unmodified; `MockOptions::default()` reproduces SPEC-006 happy-path shape.
- CI green on macos-15, ubuntu-24.04, windows-2025 (PR #10 run id
  25151076129). Local `cargo test` 43/43 pass; clippy/fmt clean.
- §15 convention check: build cost entry has tokens populated and notes
  "tokens backfilled via just record-cost" — convention held this cycle
  (improvement from SPEC-007 ship drop).
- Promoted PR #10 from draft to ready.
