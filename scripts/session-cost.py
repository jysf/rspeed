#!/usr/bin/env python3
"""Read the most recent Claude Code session transcript and capture cost
into the spec's cost.sessions entry. Autopilot version of record-cost.

Usage:
    just session-cost                           # show breakdown
    just session-cost SPEC-NNN cycle            # show + suggest record-cost
    just session-cost SPEC-NNN cycle --apply    # parse + run record-cost
    just session-cost --transcript <path>       # explicit transcript path
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

# CALIBRATION
# Attempted calibration against SPEC-008 build (date: 2026-04-29, Sonnet,
# recorded /cost USD: $3.21). The SPEC-008 build transcript could not be
# unambiguously isolated (many sessions read latency_probe content as context).
# The nearest candidate (Sonnet, ~Apr 28, transcript 0bd45a85) gives $2.65
# with the prices below — ratio 0.826 (~17% under the recorded value).
#
# The discrepancy is likely because: (a) /cost may account for server-side
# factors not in the JSONL, (b) the stored $3.21 was manually entered and
# may include estimates, or (c) the calibration transcript is not the correct
# build session. Prices below match Anthropic's published Claude 4.x rates.
# Treat computed USD as a reasonable estimate; re-calibrate when a session
# with a precisely-known /cost figure and matching JSONL path is available.
PRICING: dict[str, dict[str, float]] = {
    "sonnet": {
        "input": 3.00,
        "output": 15.00,
        "cache_read": 0.30,
        "cache_create_5m": 3.00,
        "cache_create_1h": 6.00,
    },
    "opus": {
        "input": 15.00,
        "output": 75.00,
        "cache_read": 1.50,
        "cache_create_5m": 15.00,
        "cache_create_1h": 30.00,
    },
    "haiku": {
        "input": 0.80,
        "output": 4.00,
        "cache_read": 0.08,
        "cache_create_5m": 0.80,
        "cache_create_1h": 1.60,
    },
}


def model_family(model_name: str) -> str:
    n = model_name.lower()
    if "opus" in n:
        return "opus"
    if "haiku" in n:
        return "haiku"
    return "sonnet"


def encode_project_dir(cwd: str) -> str:
    """Encode CWD to Claude Code's project-dir format: /a/b/c → -a-b-c."""
    return cwd.replace("/", "-")


def find_transcript(cwd: str) -> Path:
    encoded = encode_project_dir(cwd)
    project_dir = Path.home() / ".claude" / "projects" / encoded
    if not project_dir.exists():
        sys.exit(
            f"ERROR: Claude Code project dir not found: {project_dir}\n"
            f"  Expected from CWD: {cwd}"
        )
    jsonl_files = list(project_dir.glob("*.jsonl"))
    if not jsonl_files:
        sys.exit(f"ERROR: no .jsonl transcript files found in {project_dir}")
    return max(jsonl_files, key=lambda p: p.stat().st_mtime)


def parse_transcript(path: Path) -> dict[str, dict[str, int]]:
    """Return per-model token totals from assistant messages in the transcript."""
    totals: dict[str, dict[str, int]] = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            msg = obj.get("message", {})
            if msg.get("role") != "assistant":
                continue
            usage = msg.get("usage")
            if not usage:
                continue
            model = msg.get("model", "unknown")
            cache_creation = usage.get("cache_creation") or {}
            entry = {
                "input": usage.get("input_tokens", 0),
                "output": usage.get("output_tokens", 0),
                "cache_read": usage.get("cache_read_input_tokens", 0),
                "cache_create_5m": cache_creation.get("ephemeral_5m_input_tokens", 0),
                "cache_create_1h": cache_creation.get("ephemeral_1h_input_tokens", 0),
            }
            if model not in totals:
                totals[model] = {k: 0 for k in entry}
            for k, v in entry.items():
                totals[model][k] += v
    return totals


def compute_usd(model: str, counts: dict[str, int]) -> float:
    p = PRICING[model_family(model)]
    return (
        counts["input"] * p["input"]
        + counts["output"] * p["output"]
        + counts["cache_read"] * p["cache_read"]
        + counts["cache_create_5m"] * p["cache_create_5m"]
        + counts["cache_create_1h"] * p["cache_create_1h"]
    ) / 1_000_000


def aggregate(per_model: dict[str, dict[str, int]]) -> tuple[int, int, float]:
    """Return (total_input, total_output, total_usd) across all models.

    total_input = regular_input + cache_reads + cache_creates (all billed as input).
    total_output = output_tokens only.
    """
    total_input = 0
    total_output = 0
    total_usd = 0.0
    for model, counts in per_model.items():
        total_input += (
            counts["input"]
            + counts["cache_read"]
            + counts["cache_create_5m"]
            + counts["cache_create_1h"]
        )
        total_output += counts["output"]
        total_usd += compute_usd(model, counts)
    return total_input, total_output, total_usd


def print_breakdown(
    transcript: Path,
    per_model: dict[str, dict[str, int]],
    total_input: int,
    total_output: int,
    total_usd: float,
) -> None:
    print(f"Transcript: {transcript}")
    print()
    for model, counts in per_model.items():
        usd = compute_usd(model, counts)
        family = model_family(model)
        print(f"  {model} ({family})")
        print(f"    input:            {counts['input']:>12,}")
        print(f"    output:           {counts['output']:>12,}")
        print(f"    cache_read:       {counts['cache_read']:>12,}")
        print(f"    cache_create_5m:  {counts['cache_create_5m']:>12,}")
        print(f"    cache_create_1h:  {counts['cache_create_1h']:>12,}")
        print(f"    subtotal:         ${usd:>11.4f}")
        print()
    print(f"  TOTAL input (for record-cost): {total_input:,}")
    print(f"  TOTAL output:                  {total_output:,}")
    print(f"  TOTAL estimated USD:           ${total_usd:.4f}")
    print()


def main() -> None:
    ap = argparse.ArgumentParser(
        description="Autopilot cost capture from Claude Code transcript.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument("spec_id", nargs="?", default=None, help="e.g., SPEC-008")
    ap.add_argument("cycle", nargs="?", default=None, help="e.g., build, verify, ship")
    ap.add_argument("--apply", action="store_true",
                    help="run record-cost immediately (requires spec_id + cycle)")
    ap.add_argument("--transcript", type=Path, default=None,
                    help="explicit transcript path (skips auto-discovery)")
    args = ap.parse_args()

    if args.apply and (not args.spec_id or not args.cycle):
        ap.error("--apply requires both spec_id and cycle")

    transcript = args.transcript or find_transcript(os.getcwd())
    per_model = parse_transcript(transcript)

    if not per_model:
        sys.exit("ERROR: no assistant messages with usage data found in transcript")

    total_input, total_output, total_usd = aggregate(per_model)
    print_breakdown(transcript, per_model, total_input, total_output, total_usd)

    if not args.spec_id or not args.cycle:
        return

    cmd = [
        "just", "record-cost",
        args.spec_id, args.cycle,
        "--tokens-input", str(total_input),
        "--tokens-output", str(total_output),
        "--usd", f"{total_usd:.4f}",
    ]

    if args.apply:
        print(f"Running: {' '.join(cmd)}")
        print()
        result = subprocess.run(cmd)
        sys.exit(result.returncode)
    else:
        print("Suggested command (copy/paste or re-run with --apply):")
        print(f"  {' '.join(cmd)}")


if __name__ == "__main__":
    main()
