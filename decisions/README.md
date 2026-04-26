# Architecture Decisions

This directory holds the architecture decisions that constrain how
rspeed is built. Each DEC documents one decision in MADR-ish form:
**Context** (the situation), **Decision** (what we chose), **Consequences**
(what we accept by choosing this), **Status** (Accepted, Superseded,
Deprecated).

DECs are append-only. To change a decision, write a new DEC that
**Supersedes** the old one and update the old one's `superseded_by`
field plus its body Status line.

Each DEC also carries a ContextCore-style `insight.confidence` value.
Per [AGENTS.md §17](../AGENTS.md): decisions <0.7 emit a question to
`guidance/questions.yaml`, decisions <0.6 yellow-flag specs that
reference them in verify, and the weekly review surfaces all decisions
<0.8 with a strength/weakness trend.

## Index

| ID | Title | Status | Confidence |
|---|---|---|---|
| DEC-001 | Tokio feature set | Accepted | 0.90 |
| DEC-002 | HTTP client: reqwest with rustls | Accepted | 0.90 |
| DEC-003 | Backend abstraction with two implementations | Accepted | 0.80 |
| DEC-004 | Latency strategy: HTTP RTT primary, TCP-connect fallback | Accepted | 0.85 |
| DEC-005 | Buffer strategy: BytesMut pool with 256KB reads | Accepted | 0.75 |
| DEC-006 | Output formats: one struct, three renderers | Accepted | 0.90 |
| DEC-007 | Release tooling: cargo-dist + Homebrew tap + crates.io | Accepted | 0.70 |
| DEC-008 | TUI deferred to v2 monitor mode | Accepted (deferral) | 0.90 |
