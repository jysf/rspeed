# AGENTS.md — Claude-Only Variant

Instructions for Claude working across all phases of this repository. Read this file first, every session.

> This variant assumes Claude plays every role: architect, implementer, reviewer. The context normally in a handoff document lives inside each spec's `## Implementation Context` section.

> This file contains conventions only. For rules/constraints, see `/guidance/constraints.yaml`. For architectural rationale, see `/decisions/`. For waves of work against this app, see `/projects/`.

---

## 1. Repo Overview

- **Repo (the app):** [REPLACE: My App]
- **Purpose:** [REPLACE: one sentence]
- **Primary stakeholders:** [REPLACE]
- **Active project:** [REPLACE: PROJ-001 — MVP]

See `.repo-context.yaml` for structured metadata.

---

## 2. Work Hierarchy

```
REPO (the app — persists across all projects)
 └─ PROJECT (a wave of work: "MVP", "improvements", "v2 redesign")
     └─ STAGE (a coherent chunk within a project)
         └─ SPEC (an individual task)
```

- The **repo** is the app. `AGENTS.md`, `/docs/`, `/guidance/`,
  `/decisions/` live at repo level because they accumulate across all
  projects.
- A **project** (`/projects/PROJ-*/`) is a bounded wave of work.
- A **stage** is an epic-sized chunk within a project (2–5 per project).
- A **spec** is a single implementable task. Belongs to one stage in
  one project.

In this variant, Claude plays architect and implementer in **separate
sessions**. The spec file itself carries all the context — see its
`## Implementation Context` section.

**Decisions persist at repo level.** A decision made during PROJ-001
binds PROJ-002 as well.

**Specs do not cross project boundaries.**

---

## 3. Business Value

Value structure exists at project and stage levels; specs link lightly.

**Project `value:` block** states the thesis — a testable claim about
what this wave of work delivers. Beneficiaries, success signals, and
risks to the thesis make it falsifiable, not marketing copy.

**Stage `value_contribution:` block** states what this coherent chunk
of work advances, what capabilities it delivers, and what it
explicitly doesn't try to do. Helps avoid stages that seem valuable
but don't contribute to the project thesis.

**Spec `value_link:`** is a one-sentence reference back to the
stage's value. Infrastructure specs may have
`value_link: "infrastructure enabling X"`. Optional but encouraged —
it surfaces specs that don't trace back to the thesis.

Reports (`just report-daily`, `just report-weekly`) aggregate these
signals: which stages advanced the thesis, which specs most directly
delivered it, and where value traceability broke down.

---

## 4. Cost Tracking Discipline

Every cycle on a spec appends a session entry to the spec's
`cost.sessions` list. Agents self-report so reports can aggregate AI
spend over time.

- **Claude Code:** the AI session writes a `cost.sessions` entry with
  null token fields during the cycle. After the session ends, the
  user runs `/cost` and backfills the numbers via:
  ```bash
  just record-cost SPEC-NNN cycle --tokens-input N --tokens-output N --usd N.NN
  ```
  The helper updates the most recent matching-cycle entry that has
  null tokens, converts the legacy `tokens_total: null` shorthand to
  canonical `tokens_input` + `tokens_output` (which `scripts/_lib.sh`
  aggregates), and recomputes `cost.totals`. Skipping the backfill is
  acceptable — null-numeric entries are honored throughout the
  reporting pipeline; the value then comes from `session_count` and
  `agent` fields rather than aggregated token counts.
- **API calls:** use the `usage` object in the API response. The
  `record-cost` helper accepts the same flags.
- **Claude.ai web:** estimate based on session length. Set
  `interface: claude-ai` so reports can distinguish estimates.
- **Third-party agents** (Ollama, Kilo, Factory, etc.): use whatever
  cost mechanism the agent provides. If none, enter null numeric
  values with a note.

Verify cycle flags specs missing cost entries for prior cycles (does
not block the PR — visibility only). Ship cycle computes `cost.totals`
from the session entries.

Reports aggregate cost by cycle, by interface, by spec, and by stage.

**STAGE-001 history note:** the 24 cost.sessions entries shipped during
STAGE-001 were written before the `just record-cost` helper existed and
all carry `tokens_total: null`. Backfilling them retroactively isn't
useful (the original `/cost` numbers are gone). Going forward, run
`just record-cost` at the end of each session — the helper's per-spec
view confirms what landed.

---

## 5. Tech Stack

Replace with your actual stack. Be specific about versions.

- **Language:** [REPLACE]
- **Runtime:** [REPLACE]
- **Framework:** [REPLACE]
- **Database:** [REPLACE]
- **Testing:** [REPLACE]
- **Linter / Formatter:** [REPLACE]
- **Hosting:** [REPLACE]
- **CI:** [REPLACE]

---

## 6. Commands (exact)

These are the APP's commands. For template/workflow commands, see `justfile`.

```bash
[REPLACE: install command]
[REPLACE: dev command]
[REPLACE: test command]
[REPLACE: test single file command]
[REPLACE: lint command]
[REPLACE: typecheck command]
[REPLACE: build command]
```

---

## 7. Directory Structure

```
/
├── AGENTS.md                          # This file
├── CLAUDE.md                          # Pointer to AGENTS.md
├── README.md                          # Human-facing readme
├── GETTING_STARTED.md                 # First-project walkthrough
├── FIRST_SESSION_PROMPTS.md           # Phase prompts
├── .repo-context.yaml                 # Repo (app) metadata
├── .variant                           # "claude-only"
├── justfile                           # Commands: just status, just new-spec, etc.
├── scripts/                           # Shell scripts powering justfile
├── docs/                              # Architecture, data model, API contract
├── guidance/                          # Repo-level rules (across all projects)
│   ├── constraints.yaml
│   └── questions.yaml
├── decisions/                         # Repo-level DEC-* (across all projects)
├── feedback/                          # Downstream user feedback captures
├── reports/                           # Daily + weekly report outputs
├── projects/                          # Waves of work
│   ├── _templates/                    # Shared templates
│   │   ├── spec.md
│   │   ├── stage.md
│   │   └── project-brief.md
│   ├── PROJ-001-<slug>/
│   │   ├── brief.md
│   │   ├── stages/
│   │   └── specs/
│   │       └── done/
│   └── PROJ-002-<slug>/
└── src/                               # [REPLACE]
```

---

## 8. Cycle Model

Every spec moves through five cycles. **Cycles are tags, not gates**.

| Cycle | Purpose |
|---|---|
| **frame** | Go/no-go on the spec |
| **design** | Write the spec + failing tests + implementation context |
| **build** | Make failing tests pass |
| **verify** | Review + validation in one pass |
| **ship** | Merge, deploy, reflect, archive |

Valid transitions:
```
frame → design → build → verify → ship
                   ↑       │
                   └───────┘ (verify sends back on punch list)
```

**In this variant**, use **separate Claude sessions** for each cycle.
A fresh session prevents design-phase context from contaminating build
decisions, and a fresh verify session catches drift a continuation
session wouldn't.

Project and stage lifecycles are lighter:
- **Project status:** `proposed | active | shipped | cancelled`
- **Stage status:** `proposed | active | shipped | cancelled | on_hold`

---

## 9. Instruction Timeline

Every spec has a timeline file at
`projects/*/specs/SPEC-NNN-<slug>-timeline.md` listing cycle
instructions in order with status markers.

Status markers:

- `[ ]` not started — no one has picked this up yet
- `[~]` in progress — an executor is currently running this
- `[x]` complete — cycle finished; see the prompt file for what was run
- `[?]` blocked — needs a human decision or external unblock before
  proceeding. Include a one-line reason after the marker.

Cycle prompts live at `projects/*/specs/prompts/SPEC-NNN-<cycle>.md`.
The architect writes them; executors read and run them.

**Discipline for executors:**

- When you start a cycle, mark it `[~]`.
- When you finish, mark it `[x]` with a one-line result (PR number,
  cost, completion date).
- If you hit a real blocker — constraint ambiguous, dependency
  missing, verify surfaced something needing architect judgment —
  mark `[?]` with a one-line reason. Do NOT use `[?]` as a "I don't
  know what to do" dumping ground. Blocked means the next move
  requires someone else; everything else is in-progress or a
  question to resolve in the current session.

This is a convention, not a mechanism. No tooling enforces it; the
discipline lives in the prompt set. Skip it and nothing breaks, but
you lose the history artifact and the next executor has to hunt for
the right prompt.

---

## 10. Cross-Reference Rules

Every spec has these relationships, encoded in front-matter:
- `project.id` → the project it belongs to
- `project.stage` → the stage within that project
- `references.decisions` → DEC-* it was designed against
- `references.constraints` → constraints that apply

DECs are stable; specs come and go. DECs don't reciprocally list specs.

---

## 11. Coding Conventions

- **Naming:** [REPLACE]
- **File organization:** [REPLACE]
- **Imports:** [REPLACE]
- **Error handling:** [REPLACE]
- **Logging:** [REPLACE]
- **Comments:** Explain *why*, not *what*.
- **No dead code.** Delete, don't comment out.

---

## 12. Testing Conventions

- Every new function gets at least one test.
- Test file naming: [REPLACE]
- Coverage expectations: [REPLACE]
- **TDD:** Tests live in the spec's `## Failing Tests` section, written
  during **design**, made to pass during **build**.

---

## 13. Git and PR Conventions

- **Branch:** `feat/spec-NNN-<slug>`, etc.
- **One spec per branch, one PR per branch.**
- **Commits:** [REPLACE]
- **PR description must include:**
  - Project: `PROJ-NNN`
  - Stage: `STAGE-NNN`
  - Spec: `SPEC-NNN`
  - Decisions referenced, constraints checked, new `DEC-*` files

---

## 14. Domain Glossary

- **[REPLACE: Term]** — [REPLACE: Definition]

---

## 15. Cycle-Specific Rules

### During **build**

Start a **new Claude session**. Do not continue from the design session.

Before writing code:
1. Read the spec's `## Implementation Context` section.
2. Read every `DEC-*` it references.
3. Read the parent `STAGE-*.md` and project `brief.md`.
4. Read `/guidance/constraints.yaml`.
5. If anything is ambiguous, add to `/guidance/questions.yaml` and stop.

When done:
1. Fill in spec's `## Build Completion` (including reflection).
2. Append a build cost session entry to `cost.sessions`.
3. `just advance-cycle SPEC-NNN verify`.
4. Create `DEC-*` files for non-trivial build decisions.
5. Open PR.

### During **verify**

Start **another new Claude session**. Do not reuse build session.

Check: acceptance criteria met? tests pass? no decision drift? no
constraint violations? non-trivial choices have DEC-*? build reflection
answered honestly? `cost.sessions` has entries for prior cycles
(flag if missing, don't block)?

Append a verify cost session entry to `cost.sessions`.

Output: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

### During **ship**

Append `## Reflection` to spec. Three answers. Append a ship cost
session entry, then compute `cost.totals`. Then
`just archive-spec SPEC-NNN`. If stage backlog is complete, run the
Stage Ship prompt.

### Frame-outcomes-inlined-into-Build pattern

STAGE-001 specs followed a "Frame outcomes folded into Build" pattern:
the Frame critique produces a written list of resolutions (typically
5-12 items), and the Build cycle inlines them in the same commit
rather than emitting separate Frame-only artifacts. Works for specs
where Frame outcomes are tractable refinements (the common case). Does
NOT work for specs requiring structural rework — those should produce a
NO-GO Frame verdict and return to design before Build starts.

---

## 16. Session Hygiene (claude-only specific)

Because one agent plays multiple roles, context contamination is a real
risk. Four habits keep it at bay:

1. **New session per cycle where possible.** Especially design → build
   and build → verify.
2. **Never reference "as I said earlier"** in later cycles. The spec
   is the source of truth.
3. **Weekly review is non-optional.** Without a second agent pushing
   back, drift compounds silently. Run `just weekly-review`.
4. **Honest confidence values on decisions.** See Section 17.
5. **Fresh-session Verify is worth its cost.** STAGE-001 saw
   fresh-session Verify catch real bugs in 5 of 6 specs: DEC-004↔006
   path mismatch (SPEC-001), cross-spec drift across SPEC-004/005/006
   (SPEC-002), `rust-toolchain.toml` components gap (SPEC-003),
   Windows `bin_name` rendering (SPEC-004), `[lints.clippy]` scope in
   tests (SPEC-005). Do not relax the cycle-context-fresh discipline
   on substantive specs (Rust code, public API surface, concurrency)
   even when continuation feels cheaper.

---

## 17. Confidence Discipline

Decisions have an `insight.confidence` field (0.0–1.0). Honest values drive:

- **Design:** decisions at confidence < 0.7 also create a question in
  `/guidance/questions.yaml`.
- **Verify:** specs referencing decisions at confidence < 0.6 get a
  yellow flag.
- **Weekly review:** all decisions < 0.8 are listed with strength/weakness trend.

Most decisions should land between 0.7 and 0.95. 1.0 only for truly locked choices.

---

## 18. Pointers

- Constraints: `/guidance/constraints.yaml`
- Open questions: `/guidance/questions.yaml`
- Decisions: `/decisions/`
- Projects: `/projects/`
- Templates: `/projects/_templates/`
- Architecture: `/docs/architecture.md`
- Feedback: `/feedback/`
- Reports: `/reports/` (daily, weekly)
- Timelines: `/projects/*/specs/SPEC-NNN-*-timeline.md` (per-spec)
- Cycle prompts: `/projects/*/specs/prompts/`
- Phase prompts: `/FIRST_SESSION_PROMPTS.md`
- First walkthrough: `/GETTING_STARTED.md`
- Daily commands: run `just --list`

---

## rspeed-specific conventions

### Performance budgets are non-negotiable

Every spec that touches the hot path must verify against these budgets
in its acceptance criteria:

- **Cold-start latency:** ≤ 50ms from process exec to first network byte
  sent. Measured with `hyperfine` or a custom integration test that uses
  `Instant::now()` checkpoints.
- **Peak RSS:** ≤ 20MB during a full test (download + upload + latency).
  Measured via `/usr/bin/time -v` on Linux or `gtime` on macOS.
- **Throughput:** ≥ 1 Gbps sustained download on a wired link with default
  flags (4 connections, 10s duration). Measured against Cloudflare on the
  developer's machine; numbers tracked in `reports/perf-baseline.md`.

If a spec would push us past a budget, that's a Frame-phase blocker.
Either redesign or escalate to revise the budget in
`.repo-context.yaml` (which requires a DEC).

Binary-size budgets only meaningfully exercise once code is reachable.
Stub-only specs (returning `Err(NotImplemented)` from trait methods)
pass the budget vacuously because LTO+DCE strips unreachable transitive
deps. Verify the meaningful check at the spec that first wires the deps
into actual call paths — for rspeed, that's STAGE-002 measurement code.

### Style

- `cargo fmt` runs in CI with `--check`. PRs that don't fmt-pass don't merge.
- `cargo clippy --all-targets -- -D warnings` runs in CI. Same.
- No `unwrap()` or `expect()` in library code (`src/lib.rs` and below).
  `main.rs` may unwrap during startup if the failure is genuinely
  unrecoverable and well-reported.
- No `panic!()` or `unreachable!()` outside of test code unless the
  invariant is documented with a comment that survives review.
- When using clap derive, set `bin_name` explicitly on `#[command(...)]`
  to ensure consistent `--help` rendering across platforms (Windows
  otherwise shows `argv[0]` which includes the `.exe` extension;
  macOS/Linux drop it). One-line preemptive fix; otherwise discovered
  via Windows CI snapshot diff.
- When test code needs JSON parsing, prefer `serde_json` as a
  `[dev-dependencies]` over enabling reqwest's `json` feature in
  production. Keeps the production dep surface minimal; tests get JSON
  parsing via `resp.text()` + `serde_json::from_str()` (one extra line
  per test, zero production impact).

### Error handling

- The library surface (`src/lib.rs` and re-exports) uses `thiserror`
  enums with descriptive variants. Errors carry enough context that a
  caller can distinguish a network failure from a configuration error
  from an upstream protocol violation.
- The binary (`src/bin/rspeed.rs` or `src/main.rs`) uses `anyhow` and
  context-attaching with `.context()` / `.with_context()`. The final
  user-facing error message is rendered by `main` via the human-mode
  renderer when format=human, or as a JSON object with an `"error"` key
  when format=json.
- Exit codes:
  - 0: success
  - 1: test ran but failed a threshold (reserved for future
    `--fail-under` flag — not used in MVP, but the slot is reserved)
  - 2: configuration error (bad flags, invalid URL, etc.)
  - 3: network error during test
  - 4: backend error (server returned malformed response)

### Testing discipline

- Every spec lands at least one test. Specs that add new public
  functions land unit tests AND at least one integration test.
- Integration tests against Cloudflare are gated behind the `live` cargo
  feature and skipped in default CI runs. They run on a nightly schedule
  workflow.
- Snapshot tests via `insta` for any human-readable output (help text,
  summary block, error messages). Snapshot updates require explicit
  reviewer approval.
- Benches via `criterion`, stored in `benches/`. Bench results
  recorded in `reports/` per the template.
- Project-wide `[lints.clippy]` warnings (e.g., `unwrap_used`,
  `expect_used`) apply to integration tests under
  `cargo clippy --all-targets`. Use file-scope
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of
  `tests/*.rs` files to preserve the lib-vs-test distinction. Lib code
  stays constrained; fixture code can fail loudly via `unwrap()`.

### Dependency discipline

Each spec lands its own deps as it first consumes them ("School B" —
just-in-time). The `no-new-top-level-deps-without-decision` constraint
(severity: warning) is satisfied by inline justification in the spec
body when the dep doesn't warrant a full DEC. Avoid School A (landing
all anticipated deps in one early spec): it concentrates dep churn but
obscures the per-spec rationale that makes future review tractable.

### Cross-platform priorities

| Tier | Platforms | Standard |
|---|---|---|
| Primary | macOS arm64, Linux x86_64 | All tests pass on CI; perf budget met; bug-fix priority |
| Secondary | macOS x86_64¹, Linux arm64² | See footnotes |
| Best-effort | Windows x86_64 | Builds clean; basic functionality works; no perf guarantees |

¹ **macOS x86_64:** compile-validated via `cargo check --target x86_64-apple-darwin` on the arm64 CI runner (paid Intel runner avoided). Test execution and perf validation depend on user reports.
² **Linux arm64:** tests pass when run locally; no GitHub-hosted arm64 Linux runner available for CI yet (see `KNOWN_LIMITATIONS.md`).

Specs that introduce platform-conditional code (`#[cfg(target_os = ...)]`)
must justify the conditional in the spec's Implementation Context section
and add a test for each conditional branch where feasible.

### Cycle phase reminders for Claude Code

- **Frame:** clarify *what* and *why*. No code. Output: an updated spec
  with a sharper acceptance criterion if needed, plus open questions.
- **Design:** decide *how*. Sketch the key types, the file layout, the
  edge cases. Update the Implementation Context section. Still no code.
- **Build:** write the code. Land tests as you go. Keep diffs small —
  if a spec is sprawling, that's a sign it should have been split.
- **Verify:** run the tests, run the lints, manually verify acceptance
  criteria, run perf checks if the spec touches the hot path. Document
  the results in the spec under a `## Verification Results` section.
- **Ship:** open the PR, link the spec, get review, merge. Update the
  spec's `task.cycle` to `shipped` and move it to `done/` per the
  template's `just archive-spec` workflow.
