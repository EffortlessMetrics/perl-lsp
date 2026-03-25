#!/usr/bin/env python3

"""Compatibility shim for PR debt summary output."""

import json
import subprocess
import sys
from pathlib import Path


def _print_summary_from_json(raw: str) -> int:
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as err:
        print(f"Error: Invalid JSON input: {err}", file=sys.stderr)
        return 1

    summary = payload.get("summary", {})
    q = summary.get("quarantined_tests", {})
    k = summary.get("known_issues", {})
    t = summary.get("technical_debt", {})

    print("| Category | Count | Budget | Status |")
    print("|----------|-------|--------|--------|")
    print(f"| Quarantined Tests | {q.get('count', 0)} | {q.get('budget', 0)} | {q.get('status', 'unknown')} |")
    print(f"| Known Issues | {k.get('count', 0)} | {k.get('budget', 0)} | {k.get('status', 'unknown')} |")
    print(f"| Technical Debt | {t.get('count', 0)} | {t.get('budget', 0)} | {t.get('status', 'unknown')} |")

    if q.get("expired", 0) > 0:
        print("")
        print(f"**Warning:** {q['expired']} expired quarantine(s) need attention!")

    return 0


if __name__ == "__main__":
    raw = sys.stdin.read()
    if raw.strip():
        raise SystemExit(_print_summary_from_json(raw))

    repo_root = Path(__file__).resolve().parents[1]
    raise SystemExit(
        subprocess.call(["cargo", "xtask", "debt-report", "--summary"], cwd=repo_root)
    )
