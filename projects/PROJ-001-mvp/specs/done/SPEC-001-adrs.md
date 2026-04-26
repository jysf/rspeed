---
task:
  id: SPEC-001
  type: chore
  cycle: ship
  blocked: false
  priority: high
  complexity: S

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: rspeed

agents:
  architect: claude-opus-4-7
  implementer: claude-opus-4-7
  created_at: 2026-04-25

references:
  decisions: [DEC-001, DEC-002, DEC-003, DEC-004, DEC-005, DEC-006, DEC-007, DEC-008]
  constraints: []
  related_specs: []

value_link: "infrastructure enabling STAGE-001's foundational substrate — every later spec references DEC numbers"

cost:
  sessions:
    - cycle: frame
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "Frame ran in same session as planning baseline integration; /cost not captured separately. Backfilled during punch-list fix per AGENTS.md §4 null-allowed convention."
    - cycle: build
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "Build inlined Frame outcomes (commit b07ac6d on feat/spec-001-adrs). Same session as Frame; /cost not captured separately."
    - cycle: verify
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "/cost not captured in-session; Frame and Build entries are missing (yellow per AGENTS.md §4)."
    - cycle: verify-punchlist-fix
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "Punch-list resolution: DEC-004 latency_method→latency.method; backfilled cost.sessions; updated timeline. Awaiting re-verify."
    - cycle: verify
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "Re-verify of punch-list fix (commit 3cc4e01). Confirmed DEC-004 latency.method nesting + DEC-006 cross-ref, four-entry cost backfill, and timeline status. ✅ APPROVED. /cost not captured in-session."
    - cycle: ship
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-25
      tokens_total: null
      estimated_usd: null
      notes: "Ship: backfilled Build reflection, appended Ship reflection, computed cost.totals, updated stage backlog + timeline, archived spec to specs/done/. /cost not captured in-session."
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 6
---

# SPEC-001: Architecture decision records

## Context

First spec under STAGE-001 (Foundation) in PROJ-001 (MVP). The eight
architecture decisions for rspeed were drafted during pre-project
planning and need to be committed to `decisions/` in the repo's
canonical format before any code references them.

This spec is documentation, not deliberation — the decisions have been
made. Frame phase exists to give us a chance to question or refine
them before they're locked.

## Goal

Eight DEC files exist in `decisions/`, plus an updated `decisions/README.md`
indexing them, with the template's ContextCore-aligned frontmatter
(`insight.id`, `insight.confidence`, `audience`, `agent`, `project`,
`repo`, `created_at`, `tags`) and MADR-style bodies (Status, Context,
Decision, Consequences).

## Why this is first

Subsequent specs (SPEC-002 through SPEC-006) reference DEC numbers in
their justifications. Having the DECs land first means PRs can link
to them rather than re-arguing the choice in PR descriptions.

## Inputs

- **Files to read (already-drafted bundle content, now landed at):**
  - `decisions/DEC-001-tokio-feature-set.md` through
    `decisions/DEC-008-deferred-tui.md`
  - `decisions/_template.md` (template's MADR-ish ContextCore form)
  - `AGENTS.md` (§17 Confidence Discipline — drives our confidence values)

## Outputs

- **Files created (already in place after planning-baseline commit):**
  - `decisions/DEC-001-tokio-feature-set.md`
  - `decisions/DEC-002-http-client.md`
  - `decisions/DEC-003-backend-abstraction.md`
  - `decisions/DEC-004-latency-strategy.md`
  - `decisions/DEC-005-buffer-strategy.md`
  - `decisions/DEC-006-output-formats.md`
  - `decisions/DEC-007-release-tooling.md`
  - `decisions/DEC-008-deferred-tui.md`
  - `decisions/README.md`

## Acceptance Criteria

- [ ] Eight DEC files exist in `decisions/`, named
      `DEC-NNN-kebab-case-title.md`
- [ ] Each DEC has YAML frontmatter with `insight.id`,
      `insight.confidence` (honest 0.0–1.0 value per AGENTS.md §17),
      `insight.type: decision`, `audience`, `agent`, `project`, `repo`,
      `created_at`, `supersedes`, `superseded_by`, `tags`
- [ ] Each DEC body has sections: **Status**, **Context**, **Decision**,
      **Consequences**. Status options: Accepted, Superseded,
      Deprecated. DEC-008 is "Accepted (deferral)."
- [ ] `decisions/README.md` exists and indexes the eight DECs in a
      table with ID, Title, Status, Confidence
- [ ] Frame phase produces a written critique of all eight DECs,
      flagging anything that should be questioned or refined; no DEC
      bodies are silently revised — any change goes through Frame
      output and reviewer signoff
- [ ] If any decision lands at confidence < 0.7, it gets a
      corresponding entry in `guidance/questions.yaml`

### Frame outcomes folded into Build (2026-04-25)

Frame critique surfaced refinements that land in DECs / specs as part
of this spec's Build cycle (rather than as superseding DECs):

- DEC-001: `sync` feature rationale broadened to mention `oneshot`
- DEC-002: `gzip` feature dropped (throughput needs on-wire bytes);
  HTTP proxy auto-detection disabled by default
- DEC-003: trait sketch uses explicit `Result<_, BackendError>`
- DEC-005: "ample headroom" tempered to "comfortable headroom" with
  RSS budget arithmetic
- DEC-006: `ip_version` field added to `TestResult`; `connections`
  split into `connections_configured` + `connections_active`;
  forward-compat note for field additions; throughput warm-up window
  documented
- DEC-007: `cargo-binstall` compatibility called out as zero-cost
  win, queued for STAGE-005 verification
- SPEC-002: gzip removed from reqwest features
- SPEC-004: `--color <auto|always|never>` flag added (with `NO_COLOR`
  env var respected when `auto`)
- SPEC-005: shared client config (`no_proxy()`, `https_only(true)`)
  + 10GB response size cap on Generic backend reads
- STAGE-003 stage doc: notes for SPEC-014 (latency-phase progress)
  and SPEC-019 (NO_COLOR env var)
- STAGE-005 stage doc: shell completions + manpage as stretch
- PROJ-001 brief: `PROJ-002-bufferbloat` added as Wave 2 candidate
- `guidance/questions.yaml`: cargo-dist-freshness question raised for
  STAGE-005

Build cycle applies these inline; no DECs are superseded.

## Failing Tests

This spec ships markdown only. No tests to write.

The Verify cycle should confirm:

- `ls decisions/DEC-*.md | wc -l` returns 8
- `decisions/README.md` renders as a clean table on GitHub
- A teammate (or Claude with fresh eyes) can read DEC-003 in 60 seconds
  and explain what the backend abstraction is

## Implementation Context

*Read this section (and the files it points to) before starting
the build cycle.*

### Decisions that apply

This is the spec that *creates* the DECs, so they don't apply yet —
but the decisions to be committed are listed in `references.decisions`
above. The Frame critique should treat each as if it could still be
revised; the build phase locks them.

### Constraints that apply

- None at the constraint level — this is documentation. AGENTS.md §17
  drives the confidence-discipline acceptance criterion.

### Prior related work

- The bundle drafts at `rspeed-planning-bundle/decisions/` contain
  the source content (the bundle is removed in a follow-up commit
  after the planning baseline lands).

### Out of scope (for this spec specifically)

- Decisions we haven't made yet — JSON schema field names, error type
  variants, exact bench harness shape. Those become DECs in their
  parent stages when the relevant work happens.
- Filing the DECs in any external system. They live as markdown in the
  repo.
- Re-running the underlying analysis that produced each DEC. If the
  Frame critique surfaces a decision that needs reopening, that's a
  *new* DEC superseding the old one, not an inline rewrite.

## Notes for the Implementer

- DECs are append-only. To change one post-ship, write a new DEC that
  Supersedes it and update `superseded_by` on the original.
- The two DECs most likely to provoke rethinking in Frame:
  - **DEC-007 (release tooling).** cargo-dist's status changes
    fast; verify it's still healthy before committing. If it's
    abandoned, write a hand-rolled GH Actions matrix in its place
    and update the DEC to reflect.
  - **DEC-001 (tokio features).** Verify the named features are still
    exposed under those names in the current `tokio` 1.x. Tokio is very
    stable on this front but worth a 30-second sanity check against the
    current docs.
- Confidence values were assigned during planning baseline:
  - DEC-001: 0.90, DEC-002: 0.90, DEC-003: 0.80, DEC-004: 0.85,
  - DEC-005: 0.75, DEC-006: 0.90, DEC-007: 0.70, DEC-008: 0.90
  - These are starting points; Frame may revise them.

---

## Build Completion

*Filled in at the end of the **build** cycle, before advancing to verify.*

- **Branch:**
- **PR (if applicable):**
- **All acceptance criteria met?** <not yet built>
- **New decisions emitted:**
- **Deviations from spec:**
- **Follow-up work identified:**

### Build-phase reflection

1. **What was unclear in the spec that slowed you down?** — Build was unusual: Frame outcomes were applied inline rather than via a separate Design cycle, because this spec is documentation-only. The "Frame outcomes folded into Build" subsection in the spec body was added during Build to record that, but the *expectation* of inlining wasn't stated upfront — a future doc-only spec template should make that explicit.
2. **Was there a constraint or decision that should have been listed but wasn't?** — DEC-004's JSON-path inconsistency with DEC-006 (`latency_method` vs `latency.method`) was caught by Verify, not by the spec's own acceptance criteria. The Verify checklist (or the spec template) should grow a "cross-DEC consistency" check item so the same mistake doesn't recur on multi-DEC specs.
3. **If you did this task again, what would you do differently?** — Run the cross-DEC consistency sweep at the end of Build, not at Verify; the fix was small but cost a punch-list round-trip. Also: capture `/cost` at the end of each session — five of six entries here are null because no one ran `/cost` before the session ended.

---

## Reflection (Ship)

*Appended during the **ship** cycle.*

1. **What would I do differently next time?** — Run Verify in a fresh session before declaring Build complete. We caught one real cross-DEC bug (DEC-004 ↔ DEC-006 JSON path) doing exactly that on this spec; the discipline pays. Also capture `/cost` at the end of every session — five of six entries here are null-numeric.
2. **Does any template, constraint, or decision need updating?** — Yes, two small additions: (a) add a "cross-DEC consistency check" line item to the Verify checklist (or to the spec template's acceptance criteria), so multi-DEC specs sweep for shared-field naming drift before declaring Build done; (b) the spec template's Build-phase reflection should make explicit when inlining Frame outcomes (instead of superseding DECs) is acceptable, since this spec did so without an upfront convention.
3. **Is there a follow-up spec to write now?** — No new spec. SPEC-002 (Cargo project skeleton) is already drafted and is the natural next step. One small ergonomic candidate worth noting in passing: a `just record-cost SPEC-NNN <cycle>` helper that appends a `cost.sessions` entry from a `/cost` clipboard paste — would cut the friction that left five entries null on this spec. Not blocking; capture it as a future tooling tweak if it keeps coming up.
