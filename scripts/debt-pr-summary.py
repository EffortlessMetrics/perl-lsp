#!/usr/bin/env python3
"""
Generate a markdown-formatted debt summary suitable for PR comments.

Reads JSON from stdin (from debt-report.py --json) and outputs markdown table.
"""

import json
import sys


def main():
    try:
        r = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
        sys.exit(1)

    s = r.get("summary", {})
    q = s.get("quarantined_tests", {})
    k = s.get("known_issues", {})
    t = s.get("technical_debt", {})

    print("| Category | Count | Budget | Status |")
    print("|----------|-------|--------|--------|")
    print(f"| Quarantined Tests | {q.get('count', 0)} | {q.get('budget', 0)} | {q.get('status', 'unknown')} |")
    print(f"| Known Issues | {k.get('count', 0)} | {k.get('budget', 0)} | {k.get('status', 'unknown')} |")
    print(f"| Technical Debt | {t.get('count', 0)} | {t.get('budget', 0)} | {t.get('status', 'unknown')} |")

    if q.get("expired", 0) > 0:
        print("")
        print(f"**Warning:** {q['expired']} expired quarantine(s) need attention!")


if __name__ == "__main__":
    main()
