# SPEC-007 Timeline

| # | Cycle | Status | Notes |
|---|---|---|---|
| 1 | design | [x] | Spec authored 2026-04-27 (claude-sonnet-4-6 design session) |
| 2 | frame  | [x] | 2026-04-27 punch list (Opus 4.7); all 5 items resolved by architect; promoted to build |
| 3 | build  | [x] | 2026-04-28 (claude-opus-4-7); 10 integration + 4 unit tests pass; clippy/fmt clean; branch `feat/spec-007-metrics-accumulator` |
| 4 | verify | [x] | 2026-04-28 (claude-opus-4-7) — ✅ APPROVED. PR #9 CI green (macos-15 / ubuntu-24.04 / windows-2025 + x86_64-apple-darwin cross-check); 1m25s. Recommended inline DEC-008 refinement (MissedTickBehavior::Delay) for Ship cycle; minor doc tweak on the tokio::time::Instant substitution. |
| 5 | ship   | [x] | 2026-04-28 (claude-sonnet-4-6) — DEC-008 Consequence paragraph + Build reflection augmented; archived to specs/done/ |

## Cycle Log

### Design (2026-04-27)
- Spec body authored by claude-sonnet-4-6 in design session
- Defined types per DEC-006, MetricsAccumulator API per DEC-008 seam 1
- 9 failing tests specified in `tests/metrics.rs` + 4 unit tests in `src/result.rs`
- Two new prod deps justified inline: serde (promoted) + chrono (new)
- **Awaiting:** architect review before Frame starts

### Frame (2026-04-27, claude-opus-4-7) — ⚠ PUNCH LIST
Spec is well-structured and bounded; checklist passed except for the items below. No structural redesign needed.

1. **AC-7 vs design notes inconsistent on `measurement_bytes` lifecycle.** AC-7 implies `record_bytes` increments `measurement_bytes` (reset once at warmup end); design notes step 4 has tick handler accumulate from `interval_bytes`. Different implementations; affects whether `warmup_bytes_excluded_from_finish` races. Recommended fix: baseline-snapshot pattern (`bytes_at_warmup_end = total_bytes` at warmup transition; `finish()` returns `total_bytes - bytes_at_warmup_end`).
2. **AC-10 parenthetical drops `&self`.** Prose says `finish(&self)`; inline drops it. Fix to `finish(&self, connections_configured: usize, connections_active: usize) -> ThroughputResult`. Resolves the open question — `&self` is correct (clones share state via Arc).
3. **Test timing brittleness in `warmup_bytes_excluded_from_finish`** (110ms wait for 100ms warmup) and `current_mbps_reflects_last_interval_only`. Recommend `#[tokio::test(start_paused = true)]` + `tokio::time::advance` for determinism.
4. **`finish_computes_mean_and_percentiles` is vague + weakly asserted.** "Bypasses ticking by calling record_bytes in bursts aligned with manual ticks" is undefined; the accumulator only pushes to `samples` inside the tick handler. Assertion `p95 >= p50` would pass even with broken percentile math. Rewrite using paused tokio time with known per-interval bytes and ε-tolerance assertions on expected mean/p50/p95.
5. **AC-6 abort half is untested.** Add `"abort_stops_ticking"` covering "After the handle is aborted, no more snapshots are emitted."

Items 1, 3, 4 are substantive (architect decision); 2 and 5 are mechanical. Once 1/3/4 are resolved and 2/5 patched in, promote directly to Build without another Frame round.

### Build (2026-04-28, claude-opus-4-7)
- Branch `feat/spec-007-metrics-accumulator`. Files: `src/result.rs`, `src/metrics.rs`, `tests/metrics.rs`, `src/lib.rs` re-exports, `Cargo.toml` (serde promoted, chrono added, tokio test-util dev-dep).
- All 13 ACs met. 10 integration tests + 4 unit tests + 20 pre-existing tests = 34 pass.
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- One spec-level note: switched ticker to `MissedTickBehavior::Delay` to keep each `tokio::time::advance(period)` mapped to exactly one tick — see Build-phase reflection.

### Verify (2026-04-28, claude-opus-4-7) — ✅ APPROVED
- All 13 ACs verified in code; 14 new tests (10 integration + 4 unit) + 20 pre-existing = 34 pass locally; clippy/fmt clean.
- PR #9 CI: Test (macos-15) ✅, Test (ubuntu-24.04) ✅, Test (windows-2025) ✅, x86_64-apple-darwin cross-check ✅; total wall-clock 1m25s (well under 5 min).
- DEC-008 seams 1 + 2 intact: accumulator owns `watch::Sender<Snapshot>`, no rendering refs in `src/metrics.rs`; subscribers fan out via `watch::Receiver`. Seam 3 (`TestSession`) correctly deferred to SPEC-012.
- Public API surface: `MetricsAccumulator`, `TestResult`, `ThroughputResult`, `LatencyResult`, `Snapshot`, `Phase`, `compute_latency_result` re-exported; all five public types `#[non_exhaustive]`; `AccumulatorInner` and `AccumulatorState` correctly private.
- Lib-side discipline: no `unwrap()` / `expect()` outside `#[cfg(test)]` blocks; Mutex poisoning handled via `lock_state` → `unwrap_or_else(|p| p.into_inner())`.
- Dep discipline: `serde` (promoted) and `chrono` justified inline in spec; `tokio` `test-util` is `[dev-dependencies]` only — no prod surface change beyond DEC-001.
- Recommended Ship-cycle additions (non-blocking; both already validated by CI):
  1. **Inline refinement to `decisions/DEC-008-deferred-tui.md`**: append a one-line Consequence noting "snapshot cadence uses `MissedTickBehavior::Delay` so subscribers see the latest aligned snapshot rather than a backlog of stale ticks; tests rely on this for `tokio::time::advance` determinism." Same pattern as DEC-002 reqwest-version refresh and DEC-003 Send+Sync addition — inline, not superseding.
  2. **Doc note on `tokio::time::Instant`**: the build prompt specified `started_at: std::time::Instant`, but the implementation uses `tokio::time::Instant` to make `Instant::elapsed()` honor `tokio::time::advance` under `start_paused = true`. In production, the two types are interchangeable (tokio's is a thin wrapper). Recommend adding a one-line comment near the `use tokio::time::Instant` import in `src/metrics.rs` and a sentence in the Build-phase reflection so the substitution is captured alongside the `MissedTickBehavior::Delay` discovery.
