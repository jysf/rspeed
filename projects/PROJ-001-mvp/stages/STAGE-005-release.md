---
stage:
  id: STAGE-005
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
  advances: "puts a `cargo install`-able, `brew install`-able, downloadable v0.1.0 in users' hands — without this stage the project's success signals are unreachable"
  delivers:
    - "v0.1.0 published on crates.io, GitHub Releases, and a Homebrew tap"
    - "a polished README documenting usage, JSON schema, and the custom-server protocol"
    - "a CHANGELOG and dual MIT/Apache-2.0 LICENSE files"
    - "a one-command release process for future tags"
  explicitly_does_not:
    - "add new product features — those are v2 (PROJ-002, etc.)"
    - "set up Docker images, package managers other than Homebrew, or system installers"
---

# STAGE-005: Release & distribution

## What This Stage Is

`git tag v0.1.0 && git push --tags` produces a complete v0.1.0 release:
binaries on GitHub Releases, formula updated in the Homebrew tap,
crate published to crates.io, README polished, JSON schema documented.

## Why Now

This is the last stage. Everything before it produces working code;
this stage gets it into users' hands. Release tooling work has its
own learning curve (cargo-dist, Homebrew formula syntax, crates.io
auth) so it gets a dedicated stage rather than being squeezed onto
the end of STAGE-004.

## Success Criteria

- A user can `cargo install rspeed` and get a working binary
- A user can `brew tap <owner>/rspeed && brew install rspeed` (or
  `brew install <owner>/rspeed/rspeed`) and get a working binary
- A user can download the appropriate tarball/zip from
  `github.com/<owner>/rspeed/releases/latest` and extract a working
  binary
- The README at the repo root has:
  - 30-second pitch with one screenshot/asciicast
  - Install instructions for the three channels
  - Usage examples (default, json, custom server, silent)
  - JSON schema documentation with an example output
  - Custom server protocol documentation (see DEC-003)
  - Performance notes (the actual numbers from STAGE-004)
- `CHANGELOG.md` exists with `v0.1.0` entry
- A `LICENSE` file exists (MIT OR Apache-2.0 dual-license)

## Scope

### In scope (anticipated specs)

| ID | Title | Estimated |
|---|---|---|
| SPEC-027 | cargo-dist setup and release.yml | 3 hr |
| SPEC-028 | Homebrew tap repo + auto-update | 2 hr |
| SPEC-029 | crates.io publish workflow | 1 hr |
| SPEC-030 | README rewrite with screenshots/asciicast | 3 hr |
| SPEC-031 | CHANGELOG, LICENSE, contributor docs | 1 hr |

Roughly 10 hours; this is mostly mechanical setup.

### Explicitly out of scope

- New product features
- Docker images or other distribution channels
- Long-term release-cadence policy

### Stretch items (drop if running long)

- **Shell completions** (`clap_complete` generates bash/zsh/fish/PowerShell
  scripts from the existing CLI definitions). Homebrew formula installs
  them automatically. Half-day of work; high user-perceived polish.
- **Manpage** (`clap_mangen` generates from the same CLI definitions).
  Ships in the Homebrew formula and the GitHub Release tarballs. Half-day.

### Notes for specs not yet written (carried forward from SPEC-001 Frame)

- **SPEC-027 (cargo-dist setup):** verify `cargo-binstall` metadata is
  generated automatically (per DEC-007); confirm the cargo-dist
  freshness question in `guidance/questions.yaml` before locking the
  release pipeline.

## Spec Backlog

- [ ] (not yet written) — cargo-dist setup and release.yml
- [ ] (not yet written) — Homebrew tap repo + auto-update
- [ ] (not yet written) — crates.io publish workflow
- [ ] (not yet written) — README rewrite with screenshots/asciicast
- [ ] (not yet written) — CHANGELOG, LICENSE, contributor docs

**Count:** 0 shipped / 0 active / 5 pending

## Out-of-band setup required (one-time, before specs)

- A separate GitHub repo named `homebrew-rspeed` under your account
- A crates.io account with `rspeed` reserved (already done via 0.0.0
  reserve publish — see git log)
- A `CARGO_REGISTRY_TOKEN` repo secret
- A `HOMEBREW_TAP_TOKEN` repo secret (a fine-grained PAT with write
  access to the tap repo only)

## After v0.1.0

This stage closes PROJ-001-mvp. Wave 2 candidates (none committed,
just signposts):

- `PROJ-002-monitor`: monitor mode with TUI dashboard (DEC-008)
- `PROJ-003-loss`: UDP-based packet-loss probe
- `PROJ-004-icmp`: unprivileged ICMP (if not absorbed into STAGE-004's
  stretch)
- `PROJ-005-server`: a minimal `rspeed-server` binary implementing the
  Generic protocol, distributed alongside the client

Each gets its own project doc when picked up.

## Dependencies

### Depends on

- STAGE-004 (Cross-platform & performance) — perf and platform support
  must be stable before binaries ship.

### Enables

- v2 wave projects (PROJ-002, etc.) — the release pipeline is reused
  for subsequent tags.

## Stage-Level Reflection

*To be filled in when this stage ships.*
