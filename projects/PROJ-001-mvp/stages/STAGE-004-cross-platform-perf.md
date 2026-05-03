---
stage:
  id: STAGE-004
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
  advances: "turns the performance budgets from aspirations into measured-and-recorded facts, which is the load-bearing claim in the project thesis"
  delivers:
    - "cold-start ≤50ms p95, peak RSS ≤20MB, ≥1 Gbps download — all measured and committed in `reports/perf-baseline.md`"
    - "all primary platforms (macOS arm64+x86_64, Linux x86_64) hit the budgets"
    - "Windows runs cleanly at 'best-effort' tier"
    - "a perf regression bench in CI so future PRs don't silently degrade"
  explicitly_does_not:
    - "ship the binary — STAGE-005"
    - "implement ICMP (stretch only — drop without ceremony if running long)"
---

# STAGE-004: Cross-platform & performance

## What This Stage Is

Hit the three performance budgets on the three primary platforms, fix
any cross-platform behavior gaps, and verify Windows reaches
"best-effort works" status.

## Why Now

The project thesis depends on rspeed being measurably fast and
lightweight. Until this stage ships, the budgets are aspirational.
This stage also locks the platform support story before STAGE-005
packages the binary.

## Success Criteria

Measured on the developer's primary machine and recorded in
`reports/perf-baseline.md`:

- Cold-start to first network byte: ≤ 50ms (p95 across 20 runs)
- Peak RSS: ≤ 20MB (measured with `/usr/bin/time -v` or `gtime`)
- Sustained download throughput: ≥ 1 Gbps on a wired link
- Sustained upload throughput: ≥ 800 Mbps on a wired link
  (asymmetric ISP service is typical; this is a "if your link allows"
  number, not a hard floor)

Cross-platform verifications:

- macOS arm64 and x86_64: all of the above met
- Linux x86_64 (musl static): all of the above met
- Windows x86_64: builds, runs, basic test completes; perf budget
  measured-and-documented but not required to be hit

## Scope

### In scope (anticipated specs)

| ID | Title | Estimated |
|---|---|---|
| SPEC-020 | Cold-start budget verification + tuning | 4 hr |
| SPEC-021 | RSS budget verification + tuning | 3 hr |
| SPEC-022 | Throughput budget on macOS (primary) | 4 hr |
| SPEC-023 | Throughput budget on Linux | 3 hr |
| SPEC-024 | Windows "best-effort" verification | 2 hr |
| SPEC-025 | (optional) Unprivileged ICMP on mac/linux | 4 hr |
| SPEC-026 | Performance regression bench in CI | 3 hr |

Roughly 18–22 hours; 4 hr buffer.

### Explicitly out of scope

- Release packaging (STAGE-005)
- Per-platform installer polish (STAGE-005)
- Docker images (deferred indefinitely)

## Spec Backlog

- [ ] SPEC-020 (not yet written) — Cold-start budget verification + tuning
- [ ] SPEC-021 (not yet written) — RSS budget verification + tuning
- [ ] SPEC-022 (not yet written) — Throughput budget on macOS (primary)
- [ ] SPEC-023 (not yet written) — Throughput budget on Linux
- [ ] SPEC-024 (not yet written) — Windows "best-effort" verification
- [ ] SPEC-025 (not yet written, stretch) — Unprivileged ICMP on mac/linux
- [ ] SPEC-026 (not yet written) — Performance regression bench in CI

**Count:** 0 shipped / 0 active / 7 pending

## Tuning levers we expect to use

These are hypotheses. The actual fixes get pinned in their respective
specs:

- **Socket buffer sizes** via `socket2`: `SO_RCVBUF` and `SO_SNDBUF`
  bumped to 4MB. macOS may need explicit `kern.ipc.maxsockbuf`
  documentation if the OS default ceiling is hit.
- **Connection count**: 4 is the default per the spec; profile whether
  6 or 8 helps on macOS.
- **Buffer pool size**: 8 × 256KB is the DEC-005 starting point; may
  need to grow to 16 buffers if connections starve on the pool.
- **HTTP/2 vs HTTP/1.1 with multiple connections**: profile both.
  HTTP/2 multiplexing through a single connection may underperform
  separate HTTP/1.1 connections for raw throughput.
- **rustls vs aws-lc-rs as TLS provider**: aws-lc-rs is generally
  faster on AES-NI hardware; check whether reqwest exposes it cleanly.

## Stretch goal

If everything else is comfortably done, SPEC-025 adds unprivileged
ICMP via `socket2` SOCK_DGRAM (mac/linux) and `IcmpSendEcho` (Windows).
Surfaced via a `--icmp` flag, opt-in, with HTTP RTT remaining the
default. Drop without ceremony if Stage 4 runs long.

## Cross-platform code hygiene

- Each `#[cfg(target_os = ...)]` arm has a comment explaining *why*
  it's conditional
- Platform-specific tests are gated with `#[cfg(target_os = ...)]`
  and run on the appropriate matrix entry in CI
- No "works on my machine" — if perf is met only on the developer's
  laptop and not on the slowest CI runner, document the gap

## Dependencies

### Depends on

- STAGE-003 (Output & UX) — UX is locked before perf tuning so
  rendering work doesn't tear up perf measurements.

### Enables

- STAGE-005 (Release) packages and ships once perf and platform
  support are stable.

## Stage-Level Reflection

*To be filled in when this stage ships.*
