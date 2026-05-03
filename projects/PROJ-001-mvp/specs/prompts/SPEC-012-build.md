# SPEC-012 Build Prompt

You are running the **Build** cycle for SPEC-012 in a **fresh session**
per AGENTS.md §16. Your job is to write code that makes the failing
tests pass. No new scope; no refactoring beyond what the spec requires.

This is the load-bearing integration spec for STAGE-002: it turns
the SPEC-007/008/010/011 pieces into a working `rspeed --format json`
command. Take it seriously.

## Read these first (in order)

1. `projects/PROJ-001-mvp/specs/SPEC-012-orchestrator-json.md` — the
   spec; pay close attention to `## Acceptance Criteria` (17 items),
   `## Failing Tests` (9 tests), `## Implementation Context`, and
   `## Frame critique` (Frame outcomes A-2, B-1, and patches C–G are
   pre-resolved — apply them as-described).
2. `AGENTS.md` — especially §4 (cost discipline), §15 (cost-capture
   reminder), §16 (fresh-session discipline), and the rspeed-specific
   section (style, error handling, exit codes, testing discipline).
3. `decisions/DEC-006-output-formats.md` — `TestResult` field shape;
   warm-up rule
4. `decisions/DEC-008-deferred-tui.md` — three seams. SPEC-012
   implements seams #2 and #3.
5. `decisions/DEC-005-buffer-strategy.md` — warm-up window default;
   upload-RSS Consequences note (SPEC-012's 10MB upload default
   reflects this; STAGE-004 polish)
6. `decisions/DEC-001-tokio-feature-set.md` — runtime features
   already in scope; no Cargo.toml feature changes
7. `src/lib.rs` — current stub `run()`; you rewrite it
8. `src/cli.rs` + `src/config.rs` — current CLI/Config; add
   `Config::validate()` + helper accessors
9. `src/result.rs` — `TestResult`, `LatencyResult`, `ThroughputResult`,
   `Snapshot`, `Phase`, `compute_latency_result` — populate, don't
   modify
10. `src/metrics.rs` — `MetricsAccumulator` API. **Construct a fresh
    instance per phase** (not reuse with `set_phase`) per SPEC-007 AC-11
11. `src/backend/mod.rs` — `Backend` trait, `BackendError`. Trait
    is unchanged in this spec.
12. `src/backend/select.rs` — `select(&Config) -> Result<Box<dyn Backend
    + Send + Sync>, BackendError>`
13. `src/main.rs` — stays unchanged (sync `main()` calling sync
    `rspeed::run()`)
14. `tests/common/mod.rs` — `MockServer` already exposes everything
    you need (`ping_count`, `download_count`, `upload_count`,
    `start_with_options`). **Do not extend it.**
15. `guidance/constraints.yaml` + `guidance/questions.yaml`

## What to build

Apply all Frame outcomes (A-2, B-1, C–G) as described in the spec
body's `### Promotion path`. None of them require architect
re-confirmation; they are pre-approved.

### 1. `Cargo.toml` — promote `serde_json`

Move `serde_json = "1"` from `[dev-dependencies]` to `[dependencies]`.
Inline justification per the `no-new-top-level-deps-without-decision`
warning constraint: SPEC-012 makes JSON output the production code
path; promoting `serde_json` is mechanical (already in dev graph;
obvious choice). No DEC required. Keep it in `[dev-dependencies]`
removed (don't double-list).

No other Cargo.toml changes (tokio runtime + sync features already
present per DEC-001).

### 2. `src/error.rs` — new file

```rust
use crate::backend::BackendError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("backend init failed: {0}")]
    Backend(#[source] BackendError),

    #[error("latency probe failed: {0}")]
    Latency(#[source] BackendError),

    #[error("download failed: {0}")]
    Download(#[source] BackendError),

    #[error("upload failed: {0}")]
    Upload(#[source] BackendError),
}

impl TestError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) => 2,
            Self::Backend(e) | Self::Latency(e) | Self::Download(e) | Self::Upload(e) => {
                match e {
                    BackendError::Network(_) | BackendError::Timeout(_) => 3,
                    BackendError::Protocol(_) | BackendError::NotImplemented => 4,
                    _ => 4, // BackendError is #[non_exhaustive]; default to protocol-class
                }
            }
        }
    }
}
```

Add a `#[cfg(test)] mod tests` block implementing
`test_error_exit_code_mapping` per the spec's Failing Tests section
(use easily-buildable variants only — no `Network` construction).

### 3. `src/orchestrator.rs` — new file

Implement per the `## Implementation Context — TestSession struct
shape` section verbatim. Specifically:

- `pub const DEFAULT_LATENCY_SAMPLES: usize = 10;`
- `pub const DEFAULT_DOWNLOAD_BYTES_PER_REQUEST: u64 = 1_000_000_000;`
- `pub const DEFAULT_UPLOAD_BYTES_PER_REQUEST: u64 = 10 * 1024 * 1024;`
- `pub const DEFAULT_SNAPSHOT_INTERVAL: Duration = Duration::from_millis(100);`
- `pub const DEFAULT_WARMUP: Duration = Duration::from_secs(2);`

`TestSession` fields:
```rust
pub struct TestSession {
    backend: Box<dyn Backend + Send + Sync>,
    config: Config,
    snapshot_tx: watch::Sender<Snapshot>,
    snapshot_interval: Duration,  // B-1: cached for per-phase MetricsAccumulator::new
    warmup: Duration,             // B-1: cached for per-phase MetricsAccumulator::new
}
```

Public API:
- `pub fn new(backend, config) -> Self` — calls `with_intervals` with `DEFAULT_*`
- `pub fn with_intervals(backend, config, snapshot_interval, warmup) -> Self` (B-1)
- `pub fn snapshot_rx(&self) -> watch::Receiver<Snapshot>`
- `pub async fn run(&self) -> Result<TestResult, TestError>`

Per-phase orchestration (from spec body, A-2 applied):
- Construct fresh `MetricsAccumulator::new(self.snapshot_interval, self.warmup)`
- `acc.set_phase(Phase::Download | Upload)` immediately
- **Bind** `let ticker = acc.start_ticking();`
- **Bind** `let forwarder = self.spawn_forwarder(acc.subscribe());`
- Run phase logic in inner `async { ... }.await` block; capture `Result`
- **`forwarder.abort(); ticker.abort();` always run** (after the
  inner block, before `Ok(...)` / propagation). `JoinHandle::abort()`
  is a no-op if the task already exited; idempotent.
- Return result

Helper:
```rust
fn measurement_window(elapsed: Duration, warmup: Duration) -> f64 {
    elapsed.saturating_sub(warmup).as_secs_f64()
}
```

### 4. `src/config.rs` — add validate + accessors

Add:

```rust
use crate::error::TestError;

impl Config {
    pub fn validate(&self) -> Result<(), TestError> {
        if let Some(url) = &self.server {
            if !url.path().ends_with('/') {
                return Err(TestError::Config(format!(
                    "--server URL must end with a trailing slash (got: {url})"
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn server_url_string(&self) -> String {
        self.server
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "https://speed.cloudflare.com/".to_string())
    }

    pub(crate) fn ip_version_string(&self) -> String {
        match self.ip_version {
            IpVersion::Auto => "auto",
            IpVersion::V4 => "ipv4",
            IpVersion::V6 => "ipv6",
        }
        .to_string()
    }
}
```

`config_validate_rejects_server_without_trailing_slash` may live as
a `#[cfg(test)]` unit test in this file (or in `tests/orchestrator.rs`
as the spec lists — your call; both are acceptable).

### 5. `src/lib.rs` — rewrite

Replace the current stub `run()` with the version from the spec's
`## Implementation Context — lib::run() rewrite` section. Key shape:

- Sync `pub fn run() -> anyhow::Result<i32>` (signature unchanged so
  `main.rs` stays sync)
- Builds tokio runtime with `worker_threads(2)` per AC-12
- Validates config via `config.validate()`; on `Err`, prints
  `error: {e:#}` to stderr and returns `Ok(e.exit_code())`
- Calls `runtime.block_on(async_run(config))`
- `async_run`: backend init wrapped in `TestError::Backend(_)` per
  patch (F); session.run() error → `eprintln!` + `Ok(e.exit_code())`;
  on success, render per `config.format`
- `Format::Json` → `serde_json::to_writer_pretty(stdout.lock(), &result)?`
  + trailing newline
- `Format::Human` → one-line stderr warning, then JSON fall-back
- `Format::Silent` → emit nothing

Add new module declarations + re-exports at top of `lib.rs`:

```rust
pub mod error;
pub mod orchestrator;

pub use error::TestError;
pub use orchestrator::{
    DEFAULT_DOWNLOAD_BYTES_PER_REQUEST, DEFAULT_LATENCY_SAMPLES,
    DEFAULT_SNAPSHOT_INTERVAL, DEFAULT_UPLOAD_BYTES_PER_REQUEST,
    DEFAULT_WARMUP, TestSession,
};
```

### 6. `tests/orchestrator.rs` — new file

Implement all 9 failing tests per the spec's `## Failing Tests`
section. File opens with:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! End-to-end orchestrator integration tests for SPEC-012.

mod common;
```

Use `TestSession::with_intervals(..., DEFAULT_SNAPSHOT_INTERVAL,
Duration::ZERO)` to bypass the 2s warmup so 1s test windows produce
non-zero `bytes`. Test budget: ≤ 3s wall-clock per full test.

For `orchestrator_snapshot_rx_observes_phase_transitions`, use the
canonical pattern from the spec's "Sender lifetime" section
(spawn collector, run session, drop session, await collector).

### 7. `guidance/questions.yaml` — mark trailing-slash question answered

Update the existing entry for `generic-backend-base-url-trailing-slash`
per the YAML shape in the spec body's `## Implementation Context —
guidance/questions.yaml update` section. Add `status: answered`,
`answered_at: 2026-05-03`, and the resolution notes block.

### 8. `guidance/questions.yaml` — add Format::Human follow-up (Patch D)

Add a new entry tracking the SPEC-012 Human-format JSON fall-back so
STAGE-003 doesn't forget to remove it:

```yaml
- id: human-format-json-fallback-cleanup
  question: "Remove SPEC-012's Human-format JSON fall-back when STAGE-003 ships the human renderer."
  priority: low
  status: open
  raised_by: SPEC-012 design
  raised_at: 2026-05-03
  assigned_to: null
  blocks: STAGE-003
  notes: |
    SPEC-012 falls back to JSON output for --format human with a
    one-line stderr warning (AC-9). STAGE-003's human renderer
    replaces this branch. Verify that the eprintln! and the
    JSON-fallback block in lib::run()'s match arm are removed when
    the renderer ships.
```

## Definition of done

```bash
cargo test                                    # all tests pass (existing + 9 new)
cargo clippy --all-targets -- -D warnings     # clean
cargo fmt --check                             # clean
cargo build --release                         # release build succeeds
./target/release/rspeed --format json --no-upload --no-download \
    --server http://127.0.0.1:NNNNN/          # smoke against an ad-hoc MockServer
```

## When done

1. Fill in `## Build Completion` in the spec:
   - Branch name (`feat/spec-012-orchestrator`)
   - PR number (when opened)
   - All 17 ACs met? (check each)
   - Frame outcomes applied (A-2, B-1, C–G)
   - Any deviations from spec — document with rationale
   - Follow-up work identified (e.g. STAGE-004 RSS/upload-streaming)
2. Append a build cost session entry to `cost.sessions` in the spec
   frontmatter (use `just session-cost SPEC-012 build --apply`).
3. Run `just advance-cycle SPEC-012 verify`.
4. Open a PR targeting `main`. PR description must include:
   - Project: PROJ-001
   - Stage: STAGE-002
   - Spec: SPEC-012
   - Decisions referenced: DEC-001, DEC-005, DEC-006, DEC-008
   - Constraints checked: test-before-implementation,
     no-new-top-level-deps-without-decision (justified inline for
     `serde_json` promotion)

## End of session

End your final response with:

```
Cost capture — when work is done, run:
just session-cost SPEC-012 build --apply
```
