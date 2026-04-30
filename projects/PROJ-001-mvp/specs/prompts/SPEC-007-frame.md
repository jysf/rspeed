# SPEC-007 Frame Prompt

You are running the **Frame** cycle for SPEC-007. Frame is a go/no-go review:
your job is to sharpen the spec, surface any blockers, and either clear it for
Build or return it with a punch list. No code is written during Frame.

## Read these first (in order)

1. `projects/PROJ-001-mvp/specs/SPEC-007-metrics-accumulator.md` — the spec
2. `decisions/DEC-005-buffer-strategy.md`
3. `decisions/DEC-006-output-formats.md`
4. `decisions/DEC-008-deferred-tui.md`
5. `src/backend/mod.rs` — BackendError, Backend trait
6. `src/lib.rs` — current public API surface
7. `guidance/constraints.yaml`
8. `projects/PROJ-001-mvp/stages/STAGE-002-measurement-core.md` — stage context

## Frame checklist

Work through each question. Record your findings inline below or in a response.

**Scope**
- [ ] Is the spec bounded? Could a Build session complete it in ~3 hours?
- [ ] Does anything in scope belong in a later STAGE-002 spec instead?
- [ ] Is anything out-of-scope that Build will inevitably need to touch?

**Acceptance criteria**
- [ ] Are all 13 ACs testable as written?
- [ ] Is AC-2 (`compute_latency_result` panics on empty slice) the right contract,
      or should it return `Result`? Consider: an empty-samples latency probe is
      always a caller bug, not a runtime condition.
- [ ] AC-10: `finish(&self, connections_configured, connections_active)` — are
      connection counts the right thing to pass here, or should they be set
      earlier (e.g., via a `set_connections` call on the accumulator)?
- [ ] AC-11 says "orchestrator creates a fresh accumulator per phase." Is this
      the right seam, or should the accumulator support `reset()` and be reused?

**Failing tests**
- [ ] Do the 9 integration tests in `tests/metrics.rs` collectively cover all
      13 ACs? Are any ACs untested?
- [ ] `"warmup_bytes_excluded_from_finish"` test waits 110ms for a 100ms warmup.
      Is 10ms of margin enough for tokio test scheduling jitter, or should
      the test use a mock clock / longer margin?
- [ ] `"finish_computes_mean_and_percentiles"` — the spec says "manually
      populates with controlled per-interval samples bypasses ticking by calling
      `record_bytes` in bursts aligned with manual ticks." This is vague. Is
      there a cleaner test design?

**Dependencies**
- [ ] `serde` promoted to `[dependencies]` — correct, no concerns.
- [ ] `chrono = "0.4"` — any concern about binary size impact? DEC-005 notes the
      20MB RSS budget has 5–8MB of cushion; chrono adds ~200–400KB to the binary.
      Acceptable.
- [ ] Are there any missing deps (e.g., does `tokio::sync::watch` require a
      tokio feature flag not currently enabled)?

**Tokio feature check**
- [ ] `Cargo.toml` currently enables: `rt-multi-thread`, `net`, `time`, `macros`,
      `io-util`, `sync`. The `sync` feature covers `tokio::sync::watch`. Confirm
      no additional features needed.

**DEC alignment**
- [ ] Does the `MetricsAccumulator` design satisfy DEC-008 seam 1 exactly?
      (Decoupled from rendering, emits Snapshot on interval, consumers are
      watch::Receiver subscribers, accumulator does not know subscriber count.)
- [ ] Does `bytes_so_far` including warm-up bytes (AC-9) align with DEC-006's
      intent? DEC-006 says the warm-up exclusion applies to `mbps`/`bytes` in
      `ThroughputResult` — AC-9 is consistent with this.

**Open questions to resolve**
- [ ] Should `TestError` be defined in SPEC-007 or SPEC-012? (Architect flagged
      this. Recommendation: SPEC-012 — the accumulator has no error surface.)
- [ ] `finish(&self)` vs `finish(self)` — consuming vs shared-ref. Decide and
      update the spec if needed before Build.

## Output

End your Frame session with one of:

- **✅ GO** — spec is clear for Build. List any minor edits made inline to the
  spec file. Update `SPEC-007-metrics-accumulator-timeline.md` frame row to `[x]`.
- **⚠ PUNCH LIST** — spec needs revisions before Build. List each item; the
  architect resolves them and re-queues Frame (or promotes directly if minor).
- **❌ NO-GO** — structural problem requiring redesign. Explain why; return to
  architect.

## After Frame (if GO)

1. Edit `projects/PROJ-001-mvp/specs/SPEC-007-metrics-accumulator.md`:
   - Update `task.cycle` frontmatter from `design` to `build`
   - Record any AC clarifications or wording fixes made during Frame
2. Mark the frame row `[x]` in the timeline file with a one-line result.
3. End your response with the cost-capture reminder:

```
Cost capture — run `/cost` in this session, then paste:
just record-cost SPEC-007 frame --tokens-input <N> --tokens-output <N> --usd <N.NN>
```
