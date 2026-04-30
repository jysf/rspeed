# SPEC-008 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-008-<cycle>.md`.

| # | Cycle | Status | Notes |
|---|---|---|---|
| 1 | design | [~] | Spec authored 2026-04-29 (claude-opus-4-7); Frame critique in same session — see `## Frame critique (2026-04-29)` in spec body. Awaiting architect resolution of substantive item (A) before promoting to Build. |
| 2 | frame  | [~] | Frame critique inline (see spec body); 1 substantive item flagged (backend construction fallibility) + 6 mechanical patches. GO conditional on architect ack of item (A). |
| 3 | build  | [ ] |  |
| 4 | verify | [ ] |  |
| 5 | ship   | [ ] |  |

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
- Awaiting architect: resolve item (A), then promote to Build.
