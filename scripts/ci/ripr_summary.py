#!/usr/bin/env python3
"""Render a ripr JSON report into a GitHub step summary.

Defensive: tolerates schema variation between ripr versions, since this is the
first integration. Produces a markdown table sorted by classification severity.
Does not use runtime-mutation vocabulary (killed/survived).
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

# Order matters: most actionable classifications first.
SEVERITY_ORDER = [
    "exposed",
    "weakly_exposed",
    "reachable_unrevealed",
    "infection_unknown",
    "propagation_unknown",
    "no_static_path",
    "static_unknown",
]


def collect_findings(doc: Any) -> list[dict[str, Any]]:
    """Find a list of finding dicts in the ripr report regardless of layout."""
    if isinstance(doc, list):
        return [d for d in doc if isinstance(d, dict)]
    if isinstance(doc, dict):
        for key in ("findings", "results", "items", "report"):
            v = doc.get(key)
            if isinstance(v, list):
                return [d for d in v if isinstance(d, dict)]
    return []


def classify(finding: dict[str, Any]) -> str:
    for key in ("classification", "class", "category", "severity"):
        v = finding.get(key)
        if isinstance(v, str):
            return v.lower()
    return "static_unknown"


def render(findings: list[dict[str, Any]]) -> str:
    lines = ["# ripr (advisory)", ""]
    if not findings:
        lines.extend(
            [
                "No oracle-gap findings on the changed Rust diff.",
                "",
                "_ripr is advisory in this rollout. See "
                "[`docs/ci/ripr.md`](../blob/master/docs/ci/ripr.md)._",
                "",
            ]
        )
        return "\n".join(lines)

    counts: dict[str, int] = {}
    for f in findings:
        c = classify(f)
        counts[c] = counts.get(c, 0) + 1

    lines.append("## Counts")
    lines.append("")
    lines.append("| Classification | Count |")
    lines.append("|---|---:|")
    for cls in SEVERITY_ORDER:
        if cls in counts:
            lines.append(f"| `{cls}` | {counts[cls]} |")
    other = {k: v for k, v in counts.items() if k not in SEVERITY_ORDER}
    for cls, n in sorted(other.items()):
        lines.append(f"| `{cls}` | {n} |")
    lines.append("")

    lines.append("## Top findings")
    lines.append("")
    lines.append("| Classification | Location | Test signal |")
    lines.append("|---|---|---|")

    def sort_key(f: dict[str, Any]) -> tuple[int, str]:
        c = classify(f)
        return (
            SEVERITY_ORDER.index(c) if c in SEVERITY_ORDER else len(SEVERITY_ORDER),
            f.get("location", ""),
        )

    def _md_safe(s: str) -> str:
        # Normalize Windows separators and escape pipes so they don't break the
        # markdown table.
        return s.replace("\\", "/").replace("|", "\\|")

    for f in sorted(findings, key=sort_key)[:20]:
        cls = classify(f)
        loc_raw = f.get("location") or f.get("path") or "?"
        if isinstance(loc_raw, dict):
            file_part = str(loc_raw.get("file", "?"))
            line_part = loc_raw.get("line", "?")
            loc = f"{file_part}:{line_part}"
        else:
            loc = str(loc_raw)
        loc = _md_safe(loc)
        tests = f.get("related_tests") or f.get("tests") or []
        if isinstance(tests, list):
            test_str = ", ".join(_md_safe(str(t)) for t in tests[:3])
            if len(tests) > 3:
                test_str += f" (+{len(tests) - 3})"
        else:
            test_str = _md_safe(str(tests))
        lines.append(f"| `{cls}` | {loc} | {test_str or '—'} |")

    lines.append("")
    lines.append(
        "_ripr is **advisory** in this rollout. ripr is mutation-testing-lite at "
        "static-analysis prices; it does **not** run mutants and does **not** replace "
        "runtime mutation testing. See "
        "[`docs/ci/verification-ladder.md`](../blob/master/docs/ci/verification-ladder.md) "
        "for ripr's place on the verification ladder._"
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--report", type=Path, required=True)
    p.add_argument("--summary", type=str, required=True)
    args = p.parse_args()

    summary_path = Path(args.summary)
    summary_path.parent.mkdir(parents=True, exist_ok=True)

    if not args.report.exists() or args.report.stat().st_size == 0:
        with open(summary_path, "a", encoding="utf-8") as f:
            f.write("# ripr (advisory)\n\nReport file empty or missing.\n")
        return 0

    try:
        doc = json.loads(args.report.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        with open(summary_path, "a", encoding="utf-8") as f:
            f.write(f"# ripr (advisory)\n\nCould not parse report: {e}\n")
        return 0

    findings = collect_findings(doc)
    with open(summary_path, "a", encoding="utf-8") as f:
        f.write(render(findings))
    return 0


if __name__ == "__main__":
    sys.exit(main())
