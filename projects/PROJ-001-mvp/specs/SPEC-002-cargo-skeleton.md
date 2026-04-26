---
task:
  id: SPEC-002
  type: chore
  cycle: build
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
  decisions: [DEC-001, DEC-002]
  constraints:
    - no-secrets-in-code
    - test-before-implementation
    - no-new-top-level-deps-without-decision
  related_specs: [SPEC-001]

value_link: "infrastructure enabling STAGE-001 — without a buildable Cargo skeleton, no later spec can land code"

cost:
  sessions:
    - cycle: frame
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-26
      tokens_total: null
      estimated_usd: null
      notes: "Frame critique produced 5 decisions (School B deps, MSRV 1.91.0, version 0.0.1, DEC-002 inline reqwest 0.12→0.13, <1MB binary check) plus 3 bonus items (forbid unsafe_code, soft .gitignore AC, deferred clippy denies). Outcomes inlined into Build per SPEC-001 precedent. /cost not captured separately."
    - cycle: build
      agent: claude-opus-4-7
      interface: claude-code
      date: 2026-04-26
      tokens_total: null
      estimated_usd: null
      notes: "Build inlined Frame outcomes; landed Cargo skeleton (rust-toolchain.toml, Cargo.toml School B deps, src/lib.rs, src/main.rs, tests/version.rs, LICENSE-MIT, LICENSE-APACHE rename, README placeholder); DEC-002 inline refinement applied. Gates: fmt/clippy clean, debug+release build, 1 test passing, release binary 358K stripped. /cost not captured in-session."
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-002: Cargo project skeleton

## Context

Second spec under STAGE-001. With the DECs committed (SPEC-001), this
is the first spec where actual Rust code lands. It produces a
buildable, runnable `rspeed` binary that prints `--version` and exits,
with all dependencies pinned per DEC-001 (tokio features) and DEC-002
(reqwest+rustls), and the release profile tuned for size and start-up.

The crate name `rspeed` is already reserved on crates.io via a 0.0.0
placeholder publish (see git log). This spec moves the version forward
to something pre-1.0 (e.g., 0.0.1 or 0.1.0-alpha.1, decided in design).

## Goal

`cargo build --release` produces an `rspeed` binary on macOS arm64
that prints its version and exits 0. All DEC-mandated dependencies
and feature sets are pinned. The release profile is tuned (LTO,
single codegen unit, strip, panic=abort). `src/main.rs` and
`src/lib.rs` exist with the binary+library split that lets future
specs add integration tests against the lib API.

## Inputs

- **Files to read:**
  - `DEC-001` (tokio feature set)
  - `DEC-002` (reqwest+rustls)
  - `AGENTS.md` (rspeed-specific conventions, performance budgets)
  - Existing `Cargo.toml` (which currently has metadata only, no deps)

## Outputs

- **Files modified:** `Cargo.toml`, `.gitignore`, `README.md`
- **Files created:**
  - `src/lib.rs` — exports `pub fn run() -> anyhow::Result<i32>`
  - `rust-toolchain.toml` — pins the MSRV
  - `LICENSE-MIT`, `LICENSE-APACHE` — dual license per .repo-context.yaml
- **Files modified:** `src/main.rs` — calls `rspeed::run()` and propagates exit code

## Acceptance Criteria

- [ ] `Cargo.toml` declares the package as `rspeed`, edition `2024`,
      version `0.0.1`, with `rust-version = "1.91.0"` (MSRV)
- [ ] `Cargo.toml` `license` field is `"MIT OR Apache-2.0"` (replacing
      current `"MIT"`)
- [ ] `rust-toolchain.toml` exists and pins `channel = "1.91.0"`
- [ ] `Cargo.toml` declares `[lints.rust] unsafe_code = "forbid"`
- [ ] Dependencies (School B — only what SPEC-002 itself uses; every
      other dep moves to its first-consuming spec, see Frame outcomes
      below):
  ```toml
  [dependencies]
  anyhow = "1"

  [dev-dependencies]
  assert_cmd = "2"
  predicates = "3"
  ```
- [ ] `[profile.release]` configured:
  ```toml
  lto           = "thin"
  codegen-units = 1
  strip         = true
  panic         = "abort"
  ```
- [ ] `src/main.rs` and `src/lib.rs` both exist (binary + library
      pattern)
- [ ] `src/main.rs` is a thin shim that calls `rspeed::run()` and
      propagates exit code via `ExitCode`
- [ ] `src/lib.rs` exports a `pub fn run() -> anyhow::Result<i32>`
      that prints `rspeed v<version>` and returns `Ok(0)`
- [ ] `cargo build --release` produces a binary on macOS arm64
- [ ] `./target/release/rspeed` prints `rspeed v<version>` and exits 0
      (the `--version` flag check waits for clap in SPEC-004; for now
      the binary unconditionally prints the version line)
- [ ] Stripped release binary is **under 1MB** (anyhow + std only —
      the meaningful `<5MB` budget check happens in SPEC-005 when
      reqwest+rustls land)
- [ ] `.gitignore` already ignores `/target` and `*.rs.bk` from the
      planning baseline; verify, no edit needed unless missing.
      `Cargo.lock` is committed (we are a binary crate)
- [ ] `README.md` updated to a placeholder with a one-line description
      and a `Status: under development` notice
- [ ] `LICENSE-MIT` and `LICENSE-APACHE` files at the repo root
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean

### Frame outcomes folded into Build (2026-04-26)

The Frame critique approved with five decisions plus three bonus items.
All inlined here rather than written as a separate Frame commit, per
the SPEC-001 precedent.

1. **School B — just-in-time dep landing.** SPEC-002 lands only
   `anyhow` (runtime) + `assert_cmd`, `predicates` (dev). Every other
   dep originally listed moves to its first-consuming spec:
   - `clap` (and `url` if used) → SPEC-004
   - `tokio`, `reqwest`, `bytes`, `futures`, `socket2` → SPEC-005
   - `serde`, `serde_json` → SPEC-005 or SPEC-006 (whichever serializes
     first)
   - `axum`, `tempfile`, `tokio-test` → SPEC-006
   - `indicatif`, `owo-colors` → STAGE-003 specs
   - `chrono` → STAGE-002 spec that lands `TestResult.started_at`
   - `thiserror` → STAGE-002 spec that lands `BackendError`

   Downstream specs are not edited in this Build; each picks up its
   deps when its own Build cycle runs.

2. **MSRV = 1.91.0.** Set in both `rust-toolchain.toml` (`channel`)
   and `Cargo.toml` (`rust-version`).

3. **Version = 0.0.1.** Bumped from the `0.0.0` reservation publish.
   Reserves `0.1.0` for the actual MVP ship tag.

4. **DEC-002 inline refinement (reqwest 0.12 → 0.13).** Frame caught
   that 0.13 renamed feature `rustls-tls` → `rustls` and switched the
   default TLS provider to `aws-lc-rs`. DEC-002 updated inline (not a
   superseding DEC — the *decision* "use reqwest with rustls TLS" is
   unchanged, only the version-specific feature name); confidence
   dropped 0.90 → 0.85 to reflect the version-pinning surface.

5. **`<1MB` binary check (vs the original `<5MB`).** Under School B,
   SPEC-002's binary is `anyhow` + std only; ~500KB–1MB stripped is
   expected. The meaningful `<5MB` budget check moves to SPEC-005,
   when reqwest + rustls land.

Bonus items folded in:

- **`[lints.rust] unsafe_code = "forbid"`** in `Cargo.toml`. Matches
  AGENTS.md "no unsafe in library code."
- **`.gitignore` AC softened** to a verify-only check. The post-
  planning-baseline `.gitignore` already covers `/target` and
  `*.rs.bk`.
- **Acceptance criterion language** for `--version` clarified. Without
  clap, the binary prints the version line unconditionally; the
  literal `--version` flag handling is SPEC-004's responsibility.

Bonus items deliberately **deferred**:

- Strict clippy denies (`unwrap_used`, `expect_used`, `panic`) — defer
  to the spec that first introduces lib-side fallibility (likely
  SPEC-005). `main.rs` is allowed to unwrap on unrecoverable startup
  per AGENTS.md, so blanket denies need a more nuanced setup.
- Explicit `[[bin]]` table — default works.
- `cargo audit` advisory check — that's SPEC-003 (CI matrix) territory.

## Failing Tests

Written in design, made to pass in build. Updated in Build per Frame
outcomes: clap moves to SPEC-004, so the unknown-flag rejection test
also moves to SPEC-004 (where flag parsing first exists). SPEC-002
ships only the version-print test.

- **`tests/version.rs`** (integration test using `assert_cmd`)
  - `"prints_version_on_version_flag"` — `rspeed --version` exits 0,
    stdout contains `"rspeed"` and the `CARGO_PKG_VERSION` value.
    (Without clap, the binary unconditionally prints its version and
    exits, so the `--version` flag is incidental — the assertion is
    that the binary runs and reports its version.)

## Implementation Context

### Decisions that apply

- `DEC-001` — Tokio feature set. Use the exact feature list. Adding a
  feature requires a new DEC.
- `DEC-002` — reqwest with rustls-tls. No native-tls.

### Constraints that apply

- `no-secrets-in-code` — none introduced here.
- `test-before-implementation` — the tests above are written first.
- `no-new-top-level-deps-without-decision` — every dep above is
  justified by a DEC or by SPEC-002 itself for stage-1 plumbing
  (clap, anyhow, etc.). The Frame phase confirms each.

### Prior related work

- Cargo.toml currently exists with metadata fields only (description,
  license, repository, readme, keywords, categories) and version
  0.0.0. This spec adds dependencies and tunes the release profile.

### Out of scope

- Any actual CLI parsing logic — that's SPEC-004
- The backend trait — SPEC-005
- CI configuration — SPEC-003
- Logging / env-var loading — out of scope for MVP entirely
  (decided at planning time)

## Notes for the Implementer

- **Binary + library split.** Even though MVP doesn't currently expose
  a public library API to other crates, structure the project as
  `src/main.rs` + `src/lib.rs` from day one. This lets us write
  integration tests in `tests/` against the lib API and lets us
  `cargo publish` a usable library in Stage 5 with no refactor. Costs
  essentially nothing now.
- **`main.rs` should be ~10 lines:**
  ```rust
  use std::process::ExitCode;
  fn main() -> ExitCode {
      match rspeed::run() {
          Ok(code) => ExitCode::from(code as u8),
          Err(err) => { eprintln!("error: {err:#}"); ExitCode::from(2) }
      }
  }
  ```
- **`lib.rs` exports `run()`** and re-exports public types as they're
  added in later specs. For SPEC-002, `run()` is just:
  ```rust
  pub fn run() -> anyhow::Result<i32> {
      println!("rspeed v{}", env!("CARGO_PKG_VERSION"));
      Ok(0)
  }
  ```
- **Dependency versions are aspirational.** Check actual current major
  versions for each crate at build time. The semver-major lines are
  what matter: clap 4, tokio 1, reqwest 0.12, serde 1.
- **MSRV.** Pick a specific stable version that's been out for at least
  a couple of months. Don't pin to bleeding-edge stable.
- **Crate name verification.** The name is already reserved on
  crates.io, but `cargo search rspeed` is still worth running to
  confirm no conflict has appeared.

---

## Build Completion

*Filled in at end of build.*

- **Branch:**
- **PR:**
- **All acceptance criteria met?** <not yet built>
- **New decisions emitted:**
- **Deviations from spec:**
- **Follow-up work identified:**

### Build-phase reflection

1. **What was unclear that slowed you down?** —
2. **Constraint or decision that should have been listed but wasn't?** —
3. **If you did this task again, what would you do differently?** —

---

## Reflection (Ship)

1. **What would I do differently next time?** — <not yet shipped>
2. **Does any template, constraint, or decision need updating?** — <not yet shipped>
3. **Is there a follow-up spec to write now?** — <not yet shipped>
