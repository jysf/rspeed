---
task:
  id: SPEC-002
  type: chore
  cycle: frame
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
  sessions: []
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
      MSRV pinned to a specific stable Rust version (set to current
      stable at time of writing, e.g. `1.85.0`)
- [ ] `Cargo.toml` `license` field is `"MIT OR Apache-2.0"` (replacing
      current `"MIT"`)
- [ ] `rust-toolchain.toml` exists and pins `channel = "1.85.0"`
      (or whatever was chosen)
- [ ] Dependencies listed and feature-gated per DEC-001 and DEC-002:
  ```toml
  clap     = { version = "4", features = ["derive"] }
  tokio    = { version = "1", default-features = false, features = [
                  "rt-multi-thread", "net", "time", "macros",
                  "io-util", "sync"
              ] }
  reqwest  = { version = "0.12", default-features = false, features = [
                  "rustls-tls", "stream", "http2"
              ] }    # gzip intentionally omitted per DEC-002
  serde       = { version = "1", features = ["derive"] }
  serde_json  = "1"
  anyhow      = "1"
  thiserror   = "2"
  indicatif   = "0.17"
  owo-colors  = "4"
  bytes       = "1"
  socket2     = "0.5"
  futures     = "0.3"
  chrono      = { version = "0.4", default-features = false, features = ["serde", "clock"] }
  ```
- [ ] Dev-dependencies declared:
  ```toml
  assert_cmd  = "2"
  predicates  = "3"
  insta       = { version = "1", features = ["yaml"] }
  tokio-test  = "0.4"
  axum        = "0.8"
  tempfile    = "3"
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
- [ ] `./target/release/rspeed --version` prints `rspeed <version>` and
      exits 0
- [ ] Stripped binary is under 5MB (sanity check, not a perf budget
      commitment)
- [ ] `.gitignore` ignores `/target` and `**/*.rs.bk`. `Cargo.lock` is
      committed (we are a binary crate)
- [ ] `README.md` updated to a placeholder with a one-line description
      and a `Status: under development` notice
- [ ] `LICENSE-MIT` and `LICENSE-APACHE` files at the repo root
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean

## Failing Tests

Written in design, made to pass in build.

- **`tests/version.rs`** (integration test using `assert_cmd`)
  - `"prints version on --version"` — `rspeed --version` exits 0,
    stdout contains `"rspeed"` and the `CARGO_PKG_VERSION` value
  - `"unknown flag exits non-zero"` — `rspeed --not-a-flag` exits with
    a non-zero code

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
