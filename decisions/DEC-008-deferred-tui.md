---
insight:
  id: DEC-008
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
  - deferral
  - scope
  - ui
---

# DEC-008: TUI deferred to v2 monitor mode

**Status:** Accepted (deferral)
**Date:** 2026-04-25
**Supersedes:** —

## Context

A live-updating ratatui dashboard during the test was considered for
MVP. Ratatui is the dominant Rust TUI library (a maintained fork of
tui-rs, used by BugStalker, claudectl, ATAC, and many others) and the
clear right tool for terminal dashboards.

But MVP is a one-shot 10-second test where the user fires the command,
watches output, and exits. Stacked indicatif progress bars with live
Mbps and a colored summary block satisfy the "real-time speed display"
requirement at near-zero cost (~150 lines, ~3ms overhead). Adding a
TUI would:

- Cost 5–15ms on cold start (terminal raw mode init/restore) — 10–30%
  of the 50ms budget for value users don't see
- Add 2–4MB to peak RSS, tightening the 20MB budget meaningfully
- Add ~15 transitive dependencies and 1–2MB to binary size
- Approximately double Stage 3's scope from 2 days to 4 days
- Conflict with the "fast and lightweight" pitch differentiating us
  from heavier alternatives

The genuinely valuable TUI use case is **monitor mode** — running the
test on an interval, persisting history, displaying time-series and
histograms over hours/days. That's a v2 feature, not MVP.

## Decision

**Defer TUI to a v2 project (`PROJ-002-monitor` or similar).** MVP
uses indicatif for the live human-mode display and produces a static
colored summary at completion.

To make the v2 add cheap, **MVP must preserve these design seams**:

1. `MetricsAccumulator` is decoupled from rendering. It emits
   `Snapshot` structs at a fixed cadence (default 100ms, configurable).
2. Snapshot fan-out uses `tokio::sync::watch` (or broadcast channel)
   so multiple subscribers can read without any subscriber blocking
   the source. MVP has one subscriber (the indicatif renderer).
3. The orchestrator (`TestSession::run`) is independent of "how the
   test is invoked." A future `MonitorSession` can wrap `TestSession`
   in a loop without touching measurement code.

These seams are codified as acceptance criteria on Stage 2 specs.

## Consequences

- MVP ships sooner and lighter.
- v2 monitor mode has a clear architectural runway: add a `monitor`
  subcommand, add a `ratatui` renderer subscribing to `Snapshot`,
  add a SQLite-backed history store. Estimated 1 week of work.
- We commit to not bypassing the snapshot abstraction even when it'd
  be tactically convenient (e.g., the indicatif renderer reading
  bytes-counter directly). PR review enforces this.
- Library: `ratatui` is the planned choice for v2. `iocraft` was
  considered (declarative React-like API) but ratatui's ecosystem
  maturity and immediate-mode model fit better for a real-time
  dashboard.
