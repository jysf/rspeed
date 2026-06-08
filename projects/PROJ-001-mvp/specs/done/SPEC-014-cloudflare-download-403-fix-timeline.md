# SPEC-014 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-014-<cycle>.md`.

## Instructions

Combined design-through-ship in one session (bounded scope, explicit diagnostic
decision tree). Per AGENTS.md §15 Frame-outcomes-folded-into-Build pattern.

- [x] design — 2026-05-09. Root cause identified (bytes ≥ 100 MB → 403), spec authored,
      implementation planned.
- [x] build — 2026-05-09. `DEFAULT_DOWNLOAD_BYTES_PER_REQUEST` → 25 MB, download loop added,
      live feature + test added, DECs amended. 79/79 tests pass.
- [x] verify — 2026-05-09. All 10 ACs verified. Live test passes against Cloudflare.
- [x] ship — 2026-05-09. PR opened, branch feat/spec-014-cloudflare-download-fix.
