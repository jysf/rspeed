---
task:
  id: SPEC-004
  type: story
  cycle: build
  estimated_hours: 3-4
  blocked: false
  priority: high
  complexity: M

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
  decisions: [DEC-006]
  constraints:
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-002]

value_link: "delivers STAGE-001's `cargo run -- --help`-able binary surface; the public CLI contract enters review here"

cost:
  sessions:
    - cycle: frame
      agent: claude-opus-4-7
      interface: claude-ai
      tokens_total: null
      estimated_usd: null
      note: "Opus frame critique 2026-04-26; 10 inline edits approved"
    - cycle: build
      agent: claude-sonnet-4-6
      interface: claude-code
      tokens_total: null
      estimated_usd: null
      note: "Build session 2026-04-26; run /cost to fill in"
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 2
---

# SPEC-004: CLI surface with clap derive

## Context

Fourth spec under STAGE-001. With the Cargo skeleton (SPEC-002) and
CI (SPEC-003) in place, we define the full CLI flag matrix using
clap derive. Snapshot tests via `insta` lock the surface so future
drift is caught at PR review. No actual test logic yet — `run()` parses
args, prints the resolved configuration in human-readable form, and
exits 0.

The flag set is the public-API contract for v0.1.0. Resist adding
"convenience" flags now — each one is a maintenance burden.

## Goal

`src/cli.rs` defines a `Cli` struct (clap-derived) and `src/config.rs`
defines a `Config` struct (downstream-facing, with defaults applied
and conflicts resolved). `lib::run()` parses CLI args, builds `Config`,
debug-prints it, and exits 0. `--help` and `--version` snapshot tests
are committed.

## Inputs

- **Files to read:**
  - `DEC-006` (output formats — the `--format` flag values come from this)
  - `AGENTS.md` (exit codes, error handling)
  - `src/lib.rs` and `src/main.rs` from SPEC-002

## Outputs

- **Files created:** `src/cli.rs`, `src/config.rs`, `tests/cli.rs`
- **Files modified:** `src/lib.rs` (parses Cli, builds Config, debug-prints)
- **Snapshot files (insta):** `tests/snapshots/cli__help.snap`, etc.

## Acceptance Criteria

- [ ] `src/cli.rs` exists, defining a `Cli` struct with
      `#[derive(Parser)]` and these flags:
  - `-d, --duration <SECONDS>` — test duration in seconds. Default: `10`. Type: `u32`.
  - `-c, --connections <N>` — parallel connections. Default: `4`. Type: `u8` (range-validated **1..=32**; Frame outcome #3).
  - `-s, --server <URL>` — custom server URL. Optional. Type: `url::Url` (Frame outcome #1 — free URL validation at parse time; clap rejects malformed URLs with exit 2).
  - `--no-upload` — skip the upload phase.
  - `--no-download` — skip the download phase. Conflicts with `--no-upload`.
  - `-f, --format <FORMAT>` — output format. Default: `human`. Values: `human`, `json`, `silent`.
  - `--color <WHEN>` — color output. Default: `auto`. Values: `auto`, `always`, `never`.
    `auto`: enables color iff stdout is a TTY **and** `NO_COLOR` env var is unset.
    `always`: forces color, overrides `NO_COLOR`.
    `never`: disables color, overrides TTY detection.
    (SPEC-004 stores the resolved enum; TTY/`NO_COLOR` resolution at runtime lands in STAGE-003 SPEC-019.)
  - `-4, --ipv4` — force IPv4. Conflicts with `-6, --ipv6`.
  - `-6, --ipv6` — force IPv6.
  - `-v, --verbose` — count flag (`-v`, `-vv`, `-vvv`) for log level.
  - (Implicit) `-h, --help` and `-V, --version` from clap.
- [ ] `Cli` is private to the binary (`mod cli;` not `pub mod cli;`); library consumers use `Config` directly without going through clap (Frame outcome #4).
- [ ] `From<Cli> for Config` impl converts the parsed `Cli` into a flat `Config` struct in `src/config.rs`; `Format` and `ColorWhen` re-exported via `config` module so library consumers don't import from `cli`.
- [ ] `Config` is the type passed around the rest of the codebase; `Cli` is parser-only.
- [ ] `lib::run()` now calls `Cli::parse()`, builds `Config`, debug-prints it, exits 0.
- [ ] **Cargo.toml `[dependencies]`:** `anyhow = "1"`, `clap = { version = "4", features = ["derive"] }`, `url = "2"` (Frame outcome #1).
- [ ] **Cargo.toml `[dev-dependencies]`:** `assert_cmd = "2"`, `predicates = "3"`, `insta = "1"` (no `yaml` feature — text snapshots only; Frame outcome #2).
- [ ] **`Cargo.lock`** is committed (project policy from SPEC-002).
- [ ] **`tests/snapshots/*.snap`** files are committed to git; `.gitignore` is not modified (Frame outcome #6).
- [ ] Snapshot tests via `insta` for:
  - `rspeed --help` output
  - `rspeed --version` output
  - 3 flag combinations (default config; `-f json -d 30`; `-s https://example.com --no-upload`) — assert resolved `Config` structure via debug snapshot.
  - Snapshot tests pass on macOS arm64, Linux x86_64, and Windows x86_64 in CI. If Windows produces CRLF differences in `--help` output, normalize line endings in test setup (`text.replace("\r\n", "\n")` before snapshotting), not in the `.snap` files (Frame outcome #7).
- [ ] Integration tests via `assert_cmd` covering:
  - Exit **2** (exact code) for unknown flag (Frame outcome #5).
  - Exit **2** (exact code) for conflicting flags (`--ipv4 --ipv6`, `--no-upload --no-download`).
  - Exit **2** (exact code) for `--connections 0` and `--connections 33` (out-of-range).
  - Exit **2** (exact code) for invalid server URL.
- [ ] `--help` output reads cleanly to a fresh user (subjective; review in Verify cycle).

### Binary size note

Stripped release binary may exceed SPEC-002's `<1MB` target after clap and url land (expect ~800KB–1.2MB). The meaningful budget check remains SPEC-005 (`<5MB` once reqwest+rustls land). Do not gate SPEC-004 on the 1MB number (Frame outcome #9).

## Failing Tests

- **`tests/cli.rs`**
  - `"prints help"` — snapshot of `rspeed --help` stdout
  - `"prints version"` — snapshot of `rspeed --version` stdout
  - `"resolves default config"` — debug-snapshot of `Config` from
    no-flag invocation
  - `"resolves with json format and duration"` — debug-snapshot of
    `Config` from `-f json -d 30`
  - `"rejects connections out of range"` — `rspeed -c 0` exits 2
  - `"rejects ipv4 ipv6 conflict"` — `rspeed -4 -6` exits 2
  - `"rejects no-upload no-download conflict"` — `rspeed --no-upload --no-download` exits 2
  - `"unknown flag exits nonzero"` — `rspeed --not-a-real-flag` exits
    nonzero with clap's "unrecognized argument" stderr (test deferred
    here from SPEC-002 under School B — clap doesn't exist until this
    spec, so the unknown-flag rejection contract first becomes
    testable in this spec)

## Implementation Context

### Decisions that apply

- `DEC-006` — Output formats. The three `--format` values (`human`,
  `json`, `silent`) are fixed; renderers come in STAGE-003.

### Constraints that apply

- `test-before-implementation` — the snapshot tests above are written
  first and fail until implementation lands.
- `no-new-top-level-deps-without-decision` — adding `url` (if chosen)
  is justified inline by SPEC-004; if Frame disagrees, use `String`
  validated at use site.

### Prior related work

- SPEC-002 produced `lib::run()` returning `Ok(0)`. This spec evolves
  it to parse args first.

### Out of scope

- Any network I/O
- Color output (Stage 3)
- The actual backend dispatch — SPEC-005 takes the `Config` and
  creates a `Backend`
- Logger initialization — out of scope for MVP

## Notes for the Implementer

### File layout

```
src/
├── lib.rs        # exports run(); pub use cli::Cli, config::Config;
├── main.rs       # unchanged from SPEC-002
├── cli.rs        # the Cli struct (clap-derived)
└── config.rs     # the Config struct (downstream-facing)
```

**Why two structs?** `Cli` is shaped by clap's needs (derive
attributes, `Option<T>` for unset flags, etc.). `Config` is shaped
by the rest of the code's needs (defaults applied, conflicts
resolved, types normalized — e.g., `Format` is an enum, not a
string). Keeping them separate keeps clap's quirks out of the
measurement code.

**`Format` enum:**

```rust
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Format {
    Human,
    Json,
    Silent,
}
```

**Conflict handling.** Use clap's `conflicts_with` attribute, not
runtime checks. clap's error message for conflicts is good enough.

**URL validation.** If you import `url::Url` and use it as the clap
type via `value_parser`, you get URL validation for free (clap will
reject malformed URLs). Cost: a small dependency. Acceptable.

**Snapshot tests.** Use `insta::assert_snapshot!()` for `--help` and
`--version`, `insta::assert_debug_snapshot!()` for resolved `Config`.
Initial snapshots are accepted via `cargo insta accept`. Future PRs
that change CLI surface will fail the test, prompting reviewer to
explicitly accept the change.

**Verbose flag implementation.** `#[arg(short, long, action = ArgAction::Count)]`
gives you a `u8` count. Map to log level: 0=Warn (default), 1=Info,
2=Debug, 3+=Trace. Logging integration itself is deferred — for now
`Config` just stores the count.

The flag matrix above is the MVP set. Resist the urge to add
"convenience" flags now (e.g., `--quiet` as alias for `--format silent`,
or `--full-duplex`). Each flag is a public API surface we have to
maintain.

---

### Frame outcomes folded into Build (2026-04-26)

All 10 outcomes from the Frame critique (Opus 4.7, 2026-04-26) are incorporated:

1. **`url::Url` for `--server`** — cross-spec alignment with SPEC-005 which already commits to `Url`; picking `String` here would force a bridge later. Free URL validation; clap rejects malformed URLs with exit 2.
2. **`insta` without `yaml` feature** — `yaml` feature drags in serde + serde_yaml; not needed for text snapshots. SPEC-004 only uses `assert_snapshot!` (text) and `assert_debug_snapshot!` (Debug trait).
3. **Connections ceiling 64 → 32** — DEC-005's buffer pool sizing assumes ~4-8 connections; 32 matches librespeed precedent and leaves room to relax non-breakingly later.
4. **`Cli` kept private; `Config` exposed** — `mod cli;` (private); library consumers construct `Config` directly without going through clap, so clap is not a transitive library dep.
5. **Exit-code tests assert code 2 exactly** — not just nonzero; aligns with AGENTS.md exit code table (2 = configuration error).
6. **AC enumerates Cargo.toml deps, Cargo.lock policy, .snap files committed** — added to Acceptance Criteria above.
7. **Windows snapshot stability noted** — handle CRLF only if CI flags it; clap usually renders the same across platforms.
8. **`--color` semantics tightened** — explicit flag overrides `NO_COLOR`; TTY/`NO_COLOR` resolution at runtime deferred to STAGE-003 SPEC-019.
9. **SPEC-002 `<1MB` binary check relaxed** — real check moves to SPEC-005; expect ~800KB–1.2MB after clap+url.
10. **Estimate adjusted 2h → 3-4h** — complexity stays M.

## Build Completion

- **Branch:** feat/spec-004-cli-surface
- **PR:**
- **All acceptance criteria met?** <pending verification>
- **New decisions emitted:** none (all covered by existing DECs)
- **Deviations from spec:** none
- **Follow-up work identified:** none

### Build-phase reflection

1. **What was unclear that slowed you down?** — Nothing significant; the spec and Frame outcomes were clear.
2. **Constraint or decision that should have been listed but wasn't?** — None identified.
3. **If you did this task again, what would you do differently?** — No changes; the two-struct split (Cli/Config) kept things clean.

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
