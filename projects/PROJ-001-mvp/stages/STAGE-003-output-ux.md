---
stage:
  id: STAGE-003
  status: proposed
  priority: high
  target_complete: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-25
shipped_at: null

value_contribution:
  advances: "moves the product from 'valid JSON appears' to 'rspeed feels nice to use', which is the difference between something users tolerate and something they keep installed"
  delivers:
    - "a polished `--format human` mode with progress bars and a colored summary"
    - "a `--format json` mode that pipes cleanly to `jq`"
    - "a `--format silent` mode whose exit code carries the verdict"
    - "consistent error rendering across all three formats"
  explicitly_does_not:
    - "implement a TUI dashboard — deferred to v2 (DEC-008)"
    - "tune performance — STAGE-004"
    - "package or release — STAGE-005"
---

# STAGE-003: Output & UX

## What This Stage Is

Wrap the working measurement engine in three polished output formats.
The user experience moves from "valid JSON appears" to "rspeed feels
nice to use."

## Why Now

The measurement engine produced by STAGE-002 emits `TestResult` and
`Snapshot` directly; the user-facing experience layer wraps that.
Performance optimization (STAGE-004) follows so the UX work doesn't
get torn up by perf-driven structural changes.

## Success Criteria

- `rspeed` (no flags): live progress bars during the test, colored
  summary block at the end. Looks good in iTerm2, Terminal.app, kitty,
  alacritty, and Ubuntu's gnome-terminal.
- `rspeed --format json`: single JSON object on stdout, nothing on
  stderr (under normal operation). Pipeable to `jq`.
- `rspeed --format silent`: no stdout, no stderr (under normal
  operation). Exit code conveys outcome.
- All three formats render error cases sensibly and use the exit codes
  from AGENTS.md.

## Scope

### In scope (anticipated specs)

| ID | Title | Estimated |
|---|---|---|
| SPEC-014 | Human renderer: indicatif progress bars | 4 hr |
| SPEC-015 | Human renderer: colored summary block | 3 hr |
| SPEC-016 | JSON renderer + schema documentation | 2 hr |
| SPEC-017 | Silent renderer + exit-code matrix | 1 hr |
| SPEC-018 | Error rendering across all three formats | 3 hr |
| SPEC-019 | TTY detection and color auto-disable | 1 hr |

Roughly 14 hours of focused work.

### Explicitly out of scope

- Performance tuning (STAGE-004)
- Release packaging (STAGE-005)
- A TUI / monitor-mode dashboard (deferred — see DEC-008)

### Notes for specs not yet written (carried forward from SPEC-001 Frame)

- **SPEC-014 (indicatif progress bars):** include the latency phase
  in the progress display, not just download/upload. ~1s of latency
  probing with no visible feedback feels like a hang.
- **SPEC-019 (TTY detection):** respect the `NO_COLOR` env var (any
  non-empty value disables color), and honor SPEC-004's
  `--color <auto|always|never>` override. owo-colors supports both.

## Spec Backlog

- [ ] SPEC-014 (not yet written) — Human renderer: indicatif progress bars
- [ ] SPEC-015 (not yet written) — Human renderer: colored summary block
- [ ] SPEC-016 (not yet written) — JSON renderer + schema documentation
- [ ] SPEC-017 (not yet written) — Silent renderer + exit-code matrix
- [ ] SPEC-018 (not yet written) — Error rendering across all three formats
- [ ] SPEC-019 (not yet written) — TTY detection and color auto-disable

**Count:** 0 shipped / 0 active / 6 pending

## Visual reference for human mode

The summary block (rough sketch — pin the exact style in SPEC-015):

```
  rspeed v0.1.0  ·  cloudflare  ·  10.0s

  Latency:   12.4 ms  (jitter 1.1 ms,  10 samples,  http_rtt)
  Download:  847 Mbps (p50 859,  p95 891,  4 connections)
  Upload:    312 Mbps (p50 318,  p95 339,  4 connections)
```

Colors used sparingly: dim gray for labels, default-bright for numbers,
green tick / red cross only where success/failure is binary.

## Dependencies

### Depends on

- STAGE-002 (Measurement core) — needs populated `TestResult` and
  live `Snapshot` stream.

### Enables

- STAGE-004 may reveal places where rendering is too eager (e.g.,
  redrawing progress bars too often hurts cold-start) — those get
  fixed inline rather than waiting for STAGE-004.

## Stage-Level Reflection

*To be filled in when this stage ships.*
