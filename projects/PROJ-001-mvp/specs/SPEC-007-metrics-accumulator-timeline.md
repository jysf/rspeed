# SPEC-007 Timeline

| # | Cycle | Status | Notes |
|---|---|---|---|
| 1 | design | [x] | Spec authored 2026-04-27 (claude-sonnet-4-6 design session) |
| 2 | frame  | [x] | 2026-04-27 punch list (Opus 4.7); all 5 items resolved by architect; promoted to build |
| 3 | build  | [x] | 2026-04-28 (claude-opus-4-7); 10 integration + 4 unit tests pass; clippy/fmt clean; branch `feat/spec-007-metrics-accumulator` |
| 4 | verify | [ ] | |
| 5 | ship   | [ ] | |

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
