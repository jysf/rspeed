# SPEC-012 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-012-<cycle>.md`.

| # | Cycle | Status | Notes |
|---|---|---|---|
| 1 | design | [x] | Spec authored 2026-05-03 (claude-opus-4-7); Frame critique in same session — see `## Frame critique (2026-05-03)` in spec body. Architect resolved (A-2) + (B-1) inline; spec body amended same day. 5 mechanical patches (C–G) folded into Build prompt. |
| 2 | frame  | [x] | Frame critique inline (see spec body); GO. Architect approved A-2 (bind + abort forwarder/ticker handles at end-of-phase) + B-1 (`pub fn with_intervals` extension constructor). Patch (F) added a 5th `TestError::Backend(_)` variant for backend-init failures. |
| 3 | build  | [ ] | |
| 4 | verify | [ ] | |
| 5 | ship   | [ ] | |

## Cycle Log

### Design + Frame (2026-05-03, claude-opus-4-7)
- Combined spec-authoring + Frame critique session (Opus, 1M-context)
  per the SPEC-007/008/010/011 single-session precedent.
- Spec drafted with 17 ACs, 9 failing tests, and full Implementation
  Context including `TestSession` struct sketch, `TestError` enum
  sketch, `lib::run()` rewrite, `Config::validate()` shape, the
  internal-runtime rationale (worker_threads=2 vs `#[tokio::main]`),
  test sizing strategy, sender lifetime contract, and the
  `guidance/questions.yaml` entry update for
  `generic-backend-base-url-trailing-slash`.
- Frame verdict: GO. Architect approved A-2 + B-1 same session;
  spec body amended inline (AC-4 / AC-11 wording, code skeletons,
  exports list, frontmatter exports, Frame outcome). Item (A): bind
  forwarder + ticker handles, abort at end-of-phase (eliminates the
  previous-phase-snapshot race). Item (B): `pub fn with_intervals`
  extension constructor; production `new()` calls it with
  `DEFAULT_*` constants. 5 mechanical patches (C–G) folded into the
  Build prompt — patch (F) added a 5th `TestError::Backend(_)`
  variant for backend-init failures (TLS/URL parse).
- Honest scope: 7 hours, complexity L. The stage doc's 4hr estimate
  is light given orchestrator + TestError + `lib::run` rewrite +
  URL validation + integration tests. Splitting into SPEC-012a/b
  not recommended (orchestrator and `lib::run` rewrite are tightly
  coupled).
