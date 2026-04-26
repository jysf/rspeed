---
project:
  id: PROJ-001
  status: active
  priority: high
  target_ship: null

repo:
  id: rspeed

created_at: 2026-04-25
shipped_at: null

value:
  thesis: "A fast, lightweight Rust speedtest CLI that installs with a single command (`cargo install rspeed`, `brew install rspeed`, or a release tarball), starts in <50ms, and produces output usable by both humans and scripts — so users replace their existing speedtest with something that's faster, smaller, and pipes cleanly to jq."
  beneficiaries:
    - "developers who occasionally check their connection and want fast feedback (no Java app, no 30s startup)"
    - "ops engineers and homelab users who want to run scheduled speedtests in cron and pipe results to monitoring"
    - "Rust ecosystem users who prefer `cargo install`-able tools to system packages"
    - "rspeed maintainers (us) — a small, sharp codebase to maintain"
  success_signals:
    - "v0.1.0 shipped to crates.io, GitHub Releases, and a Homebrew tap within 2 weeks of project start"
    - "all three performance budgets met on a developer machine: cold-start <50ms, peak RSS <20MB, ≥1 Gbps download on wired link"
    - "first external download (someone who isn't us runs `cargo install rspeed`) within 30 days of release"
    - "first user-filed issue or PR within 60 days (signals reachability and discoverability)"
    - "JSON output is consumed by at least one third-party script we did not write (signals the schema is useful)"
  risks_to_thesis:
    - "performance budgets unmet on slower CI hardware or non-developer machines, undermining the 'fast' pitch"
    - "Cloudflare endpoints change and break the default backend, requiring a hot-fix release"
    - "the crowded speedtest space means 0 organic discovery and we never get a real user"
    - "macOS socket buffer ceiling caps throughput below 1 Gbps even with `socket2` tuning, forcing documentation of a `sysctl` workaround"
---

# PROJ-001: rspeed MVP

## What This Project Is

Ship a Rust CLI tool that measures download throughput, upload
throughput, and HTTP RTT against a configurable backend, in 1–2 weeks,
distributed via GitHub Releases, Homebrew, and crates.io.

## Why Now

This is the first project on rspeed. Nothing else can ship until v0.1.0
exists. The 1–2 week box is deliberate: every architecture decision
(the eight DECs) was made to fit a tight scope. Slipping the box means
either deferring features further or revisiting the budgets — both of
which weaken the thesis above.

## Definition of done

A user can:

- Run `cargo install rspeed` on macOS, Linux, or Windows and have a
  working binary
- Run `brew install <tap-owner>/rspeed/rspeed` on macOS and have a
  working binary
- Run `rspeed` and get a colored summary in <15 seconds
- Run `rspeed --json` and pipe the result to `jq`
- Run `rspeed --server https://my.server.example` against any HTTP
  server implementing the documented protocol
- Find the JSON schema and protocol spec documented in the README

The build passes:

- Cold-start to first network byte: <50ms (measured)
- Peak RSS during a test: <20MB (measured)
- Saturates 1 Gbps on a wired link with 4 parallel connections
  (measured on a developer machine, recorded in `reports/`)
- All-green CI on macOS arm64 (primary), macOS x86_64, Linux x86_64,
  and Windows x86_64

## Success Criteria

- v0.1.0 published to crates.io, GitHub Releases, and the Homebrew tap
- The three performance budgets are *measured*, not just *targeted*,
  and recorded in `reports/perf-baseline.md`
- README documents the JSON schema and the custom-server protocol so
  the public contract is plain
- A teammate (or a fresh agent) can `cargo install rspeed`, run it,
  and understand the output without consulting any other doc

## Scope

### In scope

- All five stages below (Foundation → Measurement → Output → Cross-platform → Release)
- The eight DECs already drafted (DEC-001 through DEC-008)
- macOS arm64 + x86_64 and Linux x86_64 as primary platforms
- HTTP RTT latency only (not ICMP)
- Cloudflare default backend + Generic HTTP backend
- Three output formats: human, json, silent

### Explicitly out of scope (this wave)

- ICMP latency measurement — deferred (see DEC-004)
- Packet loss measurement — needs UDP probe, deferred to v2
- Server discovery / built-in server list — Cloudflare is the default;
  custom URL otherwise
- A bundled server binary — users bring their own URL or use Cloudflare
- TUI / monitor mode — deferred to v2 (see DEC-008)
- Docker image — deferred indefinitely; semantics are weird inside
  Docker Desktop's VM and the static binary already covers most use
  cases
- Ookla / speedtest.net protocol support — out of scope, ever

## Constraints

- Cold-start <50ms (CLI parse → first socket connect)
- Peak RSS <20MB
- 1 Gbps saturation on modest wired hardware with default settings
- Tokio with minimal feature set (DEC-001)
- TLS via rustls (DEC-002), no native-tls
- Cross-platform priority: macOS first-class, Linux first-class,
  Windows best-effort (see AGENTS.md)
- No `unwrap()` / `expect()` / `panic!()` in library code
  (see AGENTS.md)

## Architecture decisions

All decisions live in `/decisions/`. Relevant for this project:

- DEC-001: Tokio feature set (minimal)
- DEC-002: HTTP client = reqwest with rustls-tls
- DEC-003: Backend abstraction with two implementations
- DEC-004: Latency = HTTP RTT primary, TCP-connect fallback
- DEC-005: Buffer strategy = BytesMut pool, 256KB reads
- DEC-006: Output formats = single struct, three renderers
- DEC-007: Release = cargo-dist + Homebrew tap + crates.io
- DEC-008: TUI deferred to v2 monitor mode

## Stage Plan

| ID | Title | Estimated days | Depends on |
|---|---|---|---|
| STAGE-001 | Foundation | 1.5 | — |
| STAGE-002 | Measurement core | 3–4 | STAGE-001 |
| STAGE-003 | Output & UX | 2 | STAGE-002 |
| STAGE-004 | Cross-platform & performance | 2–3 | STAGE-003 |
| STAGE-005 | Release & distribution | 1 | STAGE-004 |

Total: ~10–12 working days. Buffer of 2–4 days for spillover, polish,
docs, and unforeseen issues — fits the 2-week box.

- [ ] STAGE-001 (active) — Foundation: ADRs, Cargo skeleton, CI, CLI surface, backend trait stubs, mock server
- [ ] STAGE-002 (proposed) — Measurement core: latency probe, parallel-connection download/upload, snapshot fan-out
- [ ] STAGE-003 (proposed) — Output & UX: human/json/silent renderers, error rendering, TTY detection
- [ ] STAGE-004 (proposed) — Cross-platform & performance: hit budgets on macOS/Linux, Windows works
- [ ] STAGE-005 (proposed) — Release: cargo-dist, Homebrew tap, crates.io publish

**Count:** 0 shipped / 1 active / 4 proposed

## Risks

- **Cloudflare changes endpoints.** The backend trait isolates this;
  the contract is documented in DEC-003. Mitigation: single fixed-version
  release if it happens.
- **macOS socket buffer ceiling limits 1Gbps.** macOS caps `SO_RCVBUF`
  at `kern.ipc.maxsockbuf` (often 8MB). Mitigation: explicit socket2
  buffer tuning in Stage 4, document any required `sysctl` for users
  who need more.
- **rustls TLS handshake pushes cold-start over 50ms.** Mitigation:
  the budget is "first socket connect," not "first TLS byte." Handshake
  is measured separately and reported as part of the latency probe's
  warm-up.
- **1 Gbps unrealistic on M-series macs over WiFi.** "Modest hardware"
  in our constraint means *wired*. Document this in the README.

## Out-of-band dependencies

- A Homebrew tap repo (`homebrew-rspeed`) — created in Stage 5
- `rspeed` name reservation on crates.io — already done via 0.0.0
  reserve publish (see git log)
- A `CARGO_REGISTRY_TOKEN` secret in GitHub Actions — added in Stage 5

## Dependencies

### Depends on

- None. This is the first project on rspeed.

### Enables

- PROJ-002-monitor (potential v2): monitor mode with TUI dashboard
- PROJ-002-bufferbloat (potential v2; could fold into monitor):
  measure latency *during* download/upload to surface queueing delay
  introduced by saturating the link. This is the #1 thing modern
  speedtest users care about that classic speedtests miss; strong
  differentiator if v0.2 ships it.
- PROJ-003-loss (potential v2): UDP-based packet-loss probe
- PROJ-004-icmp (potential v2): unprivileged ICMP
- PROJ-005-server (potential v2): a minimal `rspeed-server` binary

## Success metrics post-ship

Not blockers, but worth tracking:

- crates.io download count over the first 30 days
- GitHub stars (vanity, but a useful adoption signal)
- A handful of issues filed by real users (if zero, the README is
  underselling and/or distribution is broken)
- One contributor PR within 60 days (signals the codebase is readable)

## Project-Level Reflection

*To be filled in when this project ships.*

- **Did we deliver the outcome in "What This Project Is"?** <not yet>
- **How many stages did it actually take?** <not yet>
- **What changed between starting and shipping?** <not yet>
- **Lessons that should update AGENTS.md, templates, or constraints?** <not yet>
- **What did we defer to the next project?** <not yet>
