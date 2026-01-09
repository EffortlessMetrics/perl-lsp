#!/usr/bin/env python3
"""
Generate PARSER_FEATURE_MATRIX.md from corpus_audit_report.json

Issue #180: Parser feature baseline infrastructure
"""

import json
from datetime import datetime
from pathlib import Path


def main():
    report_path = Path("corpus_audit_report.json")
    output_path = Path("docs/PARSER_FEATURE_MATRIX.md")
    baseline_path = Path("ci/parse_errors_baseline.txt")

    if not report_path.exists():
        print(f"Error: {report_path} not found. Run 'just parser-audit' first.")
        return 1

    with open(report_path) as f:
        report = json.load(f)

    # Read baseline if it exists
    baseline = None
    if baseline_path.exists():
        baseline = int(baseline_path.read_text().strip())

    po = report["parse_outcomes"]
    total = po["total"]
    ok = po["ok"]
    errors = po["error"]
    timeouts = po["timeout"]
    panics = po["panic"]

    success_rate = (ok / total * 100) if total > 0 else 0
    error_by_category = po.get("error_by_category", {})
    failing_files = po.get("failing_files", [])
    ga_coverage = report.get("ga_coverage", {}).get("coverage_percentage", 0)

    # Generate markdown
    lines = [
        "# Parser Feature Matrix",
        "",
        "> **Issue #180**: This document tracks parser coverage and missing features.",
        f"> Auto-generated from `corpus_audit_report.json` on {datetime.now().strftime('%Y-%m-%d %H:%M')}",
        "",
        "## Summary",
        "",
        "| Metric | Current | Target | Status |",
        "|--------|---------|--------|--------|",
        f"| Parse Success Rate | {success_rate:.0f}% ({ok}/{total} files) | 100% | {'Passing' if errors == 0 else 'In Progress'} |",
        f"| Parse Errors | {errors} | 0 | {'Passing' if errors == 0 else 'Baseline Set'} |",
        f"| Timeouts | {timeouts} | 0 | {'Passing' if timeouts == 0 else 'Failed'} |",
        f"| Panics | {panics} | 0 | {'Passing' if panics == 0 else 'Failed'} |",
        f"| GA Feature Coverage | {ga_coverage:.0f}% | 100% | {'Passing' if ga_coverage >= 80 else 'In Progress'} |",
    ]

    if baseline is not None:
        lines.append(f"| Baseline | {baseline} | 0 | Ratcheted |")

    lines.extend([
        "",
        "## Error Breakdown by Category",
        "",
        "Errors are categorized to help prioritize improvements:",
        "",
        "| Category | Count | Priority | Description |",
        "|----------|-------|----------|-------------|",
    ])

    # Priority mapping
    priority_map = {
        "ModernFeature": ("P1", "class/try/catch/field/method keywords"),
        "QuoteLike": ("P2", "q/qq/qw/qx/qr, heredocs, strings"),
        "Regex": ("P2", "m//, s///, tr///, patterns"),
        "ControlFlow": ("P2", "given/when/default"),
        "Dereference": ("P2", "->, postfix deref"),
        "Subroutine": ("P2", "Signatures, prototypes"),
        "General": ("P3", "Uncategorized"),
    }

    if error_by_category:
        # Sort by count descending
        sorted_categories = sorted(error_by_category.items(), key=lambda x: -x[1])
        for category, count in sorted_categories:
            priority, desc = priority_map.get(category, ("P3", category))
            lines.append(f"| {category} | {count} | {priority} | {desc} |")
    else:
        lines.append("| *No errors* | 0 | - | - |")

    lines.extend([
        "",
        "## Failing Files",
        "",
        "| File | Category | Error Summary |",
        "|------|----------|---------------|",
    ])

    if failing_files:
        for f in failing_files:
            # Truncate error message for readability
            error_msg = f.get("error_message", "")
            if len(error_msg) > 50:
                error_msg = error_msg[:47] + "..."
            lines.append(f"| {f['path']} | {f['category']} | {error_msg} |")
    else:
        lines.append("| *No failing files* | - | - |")

    lines.extend([
        "",
        "## Coverage Roadmap",
        "",
        "### Phase 1: Stabilize Core (Current)",
        "- [x] Establish baseline ratchet (Issue #180)",
        "- [x] Add error categorization",
        f"- {'[x]' if errors == 0 else '[ ]'} Reduce parse errors to 0",
        "",
        "### Phase 2: Modern Perl Features",
        "- [ ] `class` keyword (Perl 5.38+, Corinna)",
        "- [ ] `try`/`catch`/`finally` blocks",
        "- [ ] `field` and `method` declarations",
        "- [ ] `builtin::` functions",
        "",
        "### Phase 3: Edge Cases",
        "- [ ] Complex heredoc scenarios",
        "- [ ] Unicode in quote delimiters",
        "- [ ] Recursive regex patterns",
        "",
        "## How to Use",
        "",
        "```bash",
        "# View current parse status",
        "just parser-audit",
        "",
        "# Check against baseline (CI mode)",
        "just ci-parser-features-check",
        "",
        "# Update this document from latest audit",
        "just parser-matrix-update",
        "```",
        "",
        "## Baseline Ratchet",
        "",
        "The parse error count uses a ratchet mechanism:",
        "- Baseline stored in `ci/parse_errors_baseline.txt`",
        "- CI fails if parse errors **increase**",
        "- CI passes if parse errors stay same or decrease",
        "- When errors decrease, update baseline: `echo N > ci/parse_errors_baseline.txt`",
        "",
        "## Related Documentation",
        "",
        "- [CLAUDE.md](../CLAUDE.md) - Project overview and commands",
        "- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture",
        "- [features.toml](../features.toml) - LSP feature catalog",
    ])

    with open(output_path, "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"Updated {output_path}")
    print(f"  Parse success: {ok}/{total} ({success_rate:.0f}%)")
    print(f"  Errors: {errors} ({len(error_by_category)} categories)")

    return 0


if __name__ == "__main__":
    exit(main())
