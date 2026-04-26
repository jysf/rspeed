---
insight:
  id: DEC-007
  type: decision
  confidence: 0.70
  audience:
    - developer
    - agent

agent:
  id: claude-opus-4-7
  session_id: null

project:
  id: PROJ-001
repo:
  id: rspeed

created_at: 2026-04-25
supersedes: null
superseded_by: null

tags:
  - release
  - distribution
  - ci
---

# DEC-007: Release tooling — cargo-dist + Homebrew tap + crates.io

**Status:** Accepted
**Date:** 2026-04-25
**Supersedes:** —

## Context

We want releases to be one git-tag command. The matrix to produce on
each release:

- macOS arm64 binary (tarball)
- macOS x86_64 binary (tarball)
- Linux x86_64 musl static binary (tarball)
- Linux arm64 musl static binary (tarball)
- Windows x86_64 binary (zip)
- A crates.io publish of the source crate
- A Homebrew formula update pointing at the macOS tarballs

Doing this by hand on every release is error-prone. We want it
automated and declarative.

## Decision

Use [`cargo-dist`](https://github.com/axodotdev/cargo-dist) (or its
maintained successor at the time SPEC for Stage 5 is written) to:

- Generate a `release.yml` GitHub Actions workflow
- Build the matrix on git tag push (tag format: `v*`)
- Upload tarballs/zips to a GitHub Release
- Compute and write SHA256 checksums
- Optionally generate Homebrew formula

For Homebrew, create a separate repo `<owner>/homebrew-rspeed` with a
`Formula/rspeed.rb` file. cargo-dist can update this on release.

For crates.io, run `cargo publish` as a separate workflow step, gated
on the GitHub Release succeeding. Use a `CARGO_REGISTRY_TOKEN` secret.

If cargo-dist is unsuitable (changed APIs, abandoned, etc.) at the
time Stage 5 is executed, fall back to a hand-rolled GitHub Actions
matrix workflow. The deliverables stay the same.

## Consequences

- Release process is `git tag v0.1.0 && git push --tags`. No manual
  steps.
- A first-time setup cost of half a day in Stage 5 (cargo-dist init,
  Homebrew tap repo creation, crates.io token).
- We're somewhat coupled to cargo-dist's conventions; mitigation is
  the fallback above. Stage 5's spec evaluates the current cargo-dist
  state before committing.
- The Linux builds use musl static linking so we don't drag glibc
  versions into compatibility issues. That trades a few percent of
  performance (some allocator vs glibc differences) for portability.
- We document the JSON schema in the README and bump version per
  semver — this is enforced by Stage 5's release checklist.
