#!/usr/bin/env python3
"""Backfill cost numbers into a spec's cost.sessions entry.

Updates the most recent matching-cycle entry that has null token fields,
then recomputes cost.totals.

Usage:
    record-cost.py SPEC-NNN cycle [--tokens-input N] [--tokens-output N] \
                                  [--usd N.NN] [--note "text"]

Schema notes:
  - Per-session canonical fields are `tokens_input` and `tokens_output`
    (these are what scripts/_lib.sh aggregates into cost.totals.tokens_total).
  - Legacy entries used the `tokens_total: null` shorthand. When both
    --tokens-input and --tokens-output are provided AND the entry has
    `tokens_total: null`, this script converts the entry to the canonical
    schema (replaces the tokens_total line with separate input/output lines).
  - cost.totals.tokens_total = sum of (tokens_input + tokens_output) across
    sessions; cost.totals.estimated_usd = sum of estimated_usd; session_count
    = len(sessions).
  - The most recent matching-cycle entry with any null token field is the
    target. If you have multiple matching entries (e.g., SPEC-001 has two
    `cycle: verify` entries from a re-verify), the LAST one wins. Backfill
    each entry in chronological order.

No external dependencies — pure stdlib + regex.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


def find_spec(spec_id: str, repo_root: Path) -> Path:
    """Find the spec file (active or archived). Excludes timeline files."""
    candidates = [
        p for p in (repo_root / "projects").rglob(f"{spec_id}-*.md")
        if not p.name.endswith("-timeline.md")
        and "prompts" not in p.parts
    ]
    if not candidates:
        sys.exit(f"ERROR: spec file not found: {spec_id}")
    if len(candidates) > 1:
        listing = "\n  ".join(str(c) for c in candidates)
        sys.exit(f"ERROR: multiple files match {spec_id}:\n  {listing}")
    return candidates[0]


def find_cost_sessions_block(text: str) -> tuple[int, int]:
    """Return (start_offset, end_offset) of the sessions block content.

    Sessions block is everything between `  sessions:\\n` and the next
    sibling key under `cost:` (e.g., `  totals:`).
    """
    cost_match = re.search(r"^cost:\n", text, re.MULTILINE)
    if not cost_match:
        sys.exit("ERROR: cost: block not found in frontmatter")

    # Anchor sessions search after `cost:` start
    rest = text[cost_match.end():]
    sessions_match = re.search(r"^  sessions:\n", rest, re.MULTILINE)
    if not sessions_match:
        sys.exit("ERROR: cost.sessions: block not found")
    sessions_start = cost_match.end() + sessions_match.end()

    # Sessions block ends at next "  X:" sibling within cost: scope
    end_match = re.search(r"^  [a-zA-Z_]", text[sessions_start:], re.MULTILINE)
    if not end_match:
        sys.exit("ERROR: cost.sessions block has no end marker")
    sessions_end = sessions_start + end_match.start()

    return sessions_start, sessions_end


def split_entries(sessions_text: str) -> list[str]:
    """Split sessions block into a list of entry texts.

    Each entry begins with `    - cycle: ` and includes all following
    `      ` indented lines until the next entry start or block end.
    """
    # Use a lookahead split on the entry-start marker
    parts = re.split(r"(?m)(?=^    - cycle: )", sessions_text)
    # Drop any empty leading part
    return [p for p in parts if p.strip()]


def entry_cycle(entry: str) -> str | None:
    m = re.match(r"    - cycle: (\S+)", entry)
    return m.group(1) if m else None


def entry_has_null_tokens(entry: str) -> bool:
    """True if the entry has any null token field."""
    for field in ("tokens_input", "tokens_output", "tokens_total"):
        if re.search(rf"^      {field}: null\s*$", entry, re.MULTILINE):
            return True
    return False


def update_entry(
    entry: str,
    tokens_input: int | None,
    tokens_output: int | None,
    usd: float | None,
    note: str | None,
) -> str:
    """Apply field updates to a single entry."""

    has_legacy_total = bool(
        re.search(r"^      tokens_total: null\s*$", entry, re.MULTILINE)
    )
    has_canonical_input = bool(
        re.search(r"^      tokens_input: null\s*$", entry, re.MULTILINE)
    )
    has_canonical_output = bool(
        re.search(r"^      tokens_output: null\s*$", entry, re.MULTILINE)
    )

    # If legacy `tokens_total: null` and both input/output provided, convert
    if has_legacy_total and tokens_input is not None and tokens_output is not None:
        entry = re.sub(
            r"^      tokens_total: null\s*$",
            f"      tokens_input: {tokens_input}\n      tokens_output: {tokens_output}",
            entry,
            count=1,
            flags=re.MULTILINE,
        )
    else:
        # Canonical-schema updates (replace null with value)
        if tokens_input is not None and has_canonical_input:
            entry = re.sub(
                r"^      tokens_input: null\s*$",
                f"      tokens_input: {tokens_input}",
                entry,
                count=1,
                flags=re.MULTILINE,
            )
        if tokens_output is not None and has_canonical_output:
            entry = re.sub(
                r"^      tokens_output: null\s*$",
                f"      tokens_output: {tokens_output}",
                entry,
                count=1,
                flags=re.MULTILINE,
            )
        # If legacy total but only one of input/output provided, leave the
        # tokens_total line alone and warn (caller should provide both)

    if usd is not None:
        usd_str = format_usd(usd)
        entry = re.sub(
            r"^      estimated_usd: null\s*$",
            f"      estimated_usd: {usd_str}",
            entry,
            count=1,
            flags=re.MULTILINE,
        )

    if note:
        # Append to existing notes if present, else set
        notes_match = re.search(
            r'^      notes: "([^"]*)"', entry, re.MULTILINE
        )
        if notes_match:
            existing = notes_match.group(1)
            new_notes = f'{existing} [backfilled: {note}]'
            entry = re.sub(
                r'^      notes: "[^"]*"',
                f'      notes: "{new_notes}"',
                entry,
                count=1,
                flags=re.MULTILINE,
            )
        else:
            # Append a new notes line at end of entry
            entry = entry.rstrip("\n") + f'\n      notes: "{note}"\n'

    return entry


def format_usd(value: float) -> str:
    """Format USD with up to 4 decimal places, trimming trailing zeros."""
    s = f"{value:.4f}"
    if "." in s:
        s = s.rstrip("0").rstrip(".")
    return s if s else "0"


def sum_entry_tokens(entry: str) -> int:
    """Sum tokens for an entry: input+output if both present, else legacy total."""
    ti = re.search(r"^      tokens_input: (\d+)\s*$", entry, re.MULTILINE)
    to = re.search(r"^      tokens_output: (\d+)\s*$", entry, re.MULTILINE)
    if ti and to:
        return int(ti.group(1)) + int(to.group(1))
    if ti:
        return int(ti.group(1))
    if to:
        return int(to.group(1))
    tt = re.search(r"^      tokens_total: (\d+)\s*$", entry, re.MULTILINE)
    if tt:
        return int(tt.group(1))
    return 0


def sum_entry_usd(entry: str) -> float:
    m = re.search(r"^      estimated_usd: ([\d.]+)\s*$", entry, re.MULTILINE)
    return float(m.group(1)) if m else 0.0


def update_totals(text: str, total_tokens: int, total_usd: float, count: int) -> str:
    """Update cost.totals.{tokens_total,estimated_usd,session_count} in-place."""
    text = re.sub(
        r"^    tokens_total:\s*[\d.]+\s*$",
        f"    tokens_total: {total_tokens}",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    text = re.sub(
        r"^    estimated_usd:\s*[\d.]+\s*$",
        f"    estimated_usd: {format_usd(total_usd)}",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    text = re.sub(
        r"^    session_count:\s*\d+\s*$",
        f"    session_count: {count}",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    return text


def main() -> None:
    ap = argparse.ArgumentParser(
        description="Backfill cost numbers into a spec's cost.sessions entry.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("Schema notes:")[0],
    )
    ap.add_argument("spec_id", help="e.g., SPEC-007")
    ap.add_argument("cycle", help="e.g., frame, build, verify, ship")
    ap.add_argument("--tokens-input", type=int, default=None,
                    help="prompt token count from /cost")
    ap.add_argument("--tokens-output", type=int, default=None,
                    help="completion token count from /cost")
    ap.add_argument("--usd", type=float, default=None,
                    help="estimated USD cost from /cost")
    ap.add_argument("--note", type=str, default=None,
                    help="optional backfill note appended to existing notes")
    args = ap.parse_args()

    if all(v is None for v in (args.tokens_input, args.tokens_output, args.usd, args.note)):
        ap.error("provide at least one of --tokens-input, --tokens-output, --usd, --note")

    repo_root = Path.cwd()
    if not (repo_root / "AGENTS.md").exists():
        sys.exit("ERROR: must run from repo root (AGENTS.md not found)")

    spec_file = find_spec(args.spec_id, repo_root)
    text = spec_file.read_text()

    sessions_start, sessions_end = find_cost_sessions_block(text)
    sessions_text = text[sessions_start:sessions_end]

    entries = split_entries(sessions_text)

    # Find last matching-cycle entry with null tokens
    target_idx = None
    for i, entry in enumerate(entries):
        if entry_cycle(entry) == args.cycle and entry_has_null_tokens(entry):
            target_idx = i  # last match wins

    if target_idx is None:
        # Helpful diagnostic
        cycles = [entry_cycle(e) for e in entries]
        sys.exit(
            f"ERROR: no null-token entry for cycle '{args.cycle}'.\n"
            f"  cycles in this spec: {cycles}\n"
            f"  hint: only entries with `tokens_total: null` or `tokens_input: null` "
            f"are eligible targets."
        )

    # Apply update
    entries[target_idx] = update_entry(
        entries[target_idx],
        args.tokens_input,
        args.tokens_output,
        args.usd,
        args.note,
    )

    # Recompute totals
    total_tokens = sum(sum_entry_tokens(e) for e in entries)
    total_usd = sum(sum_entry_usd(e) for e in entries)
    count = len(entries)

    # Splice back
    new_sessions = "".join(entries)
    new_text = text[:sessions_start] + new_sessions + text[sessions_end:]
    new_text = update_totals(new_text, total_tokens, total_usd, count)

    if new_text == text:
        print(f"WARNING: no changes made to {spec_file}", file=sys.stderr)
        sys.exit(1)

    spec_file.write_text(new_text)
    rel = spec_file.relative_to(repo_root)
    print(f"✓ Updated {rel}")
    print(f"  cycle={args.cycle} entry (index {target_idx}):")
    if args.tokens_input is not None:
        print(f"    tokens_input:  {args.tokens_input}")
    if args.tokens_output is not None:
        print(f"    tokens_output: {args.tokens_output}")
    if args.usd is not None:
        print(f"    estimated_usd: {format_usd(args.usd)}")
    print(f"  cost.totals: tokens_total={total_tokens} "
          f"estimated_usd={format_usd(total_usd)} session_count={count}")


if __name__ == "__main__":
    main()
