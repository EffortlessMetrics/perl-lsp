#!/usr/bin/env python3
"""
Generate PARSER_FEATURE_MATRIX.md from corpus_audit_report.json

Issue #180: Parser feature baseline infrastructure
"""

import json
import subprocess
from datetime import datetime
from pathlib import Path


def get_git_sha() -> str:
    """Get current git commit SHA (short form)."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, check=True
        )
        return result.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"


def get_crate_version(crate_name: str) -> str:
    """Get version from Cargo.toml for a crate."""
    cargo_path = Path(f"crates/{crate_name}/Cargo.toml")
    if not cargo_path.exists():
        return "unknown"
    try:
        content = cargo_path.read_text()
        for line in content.split("\n"):
            if line.startswith("version"):
                # Extract version from 'version = "x.y.z"'
                return line.split('"')[1]
    except (IndexError, IOError):
        pass
    return "unknown"


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

    # Get provenance info
    git_sha = get_git_sha()
    parser_version = get_crate_version("perl-parser")
    generated_at = report.get("metadata", {}).get("generated_at", datetime.now().isoformat())
    corpus_path = "test_corpus/"  # Default, could be extracted from report metadata

    # Generate markdown
    lines = [
        "# Parser Feature Matrix",
        "",
        "> **Issue #180**: This document tracks parser coverage and missing features.",
        "",
        "## Provenance",
        "",
        "| Field | Value |",
        "|-------|-------|",
        f"| Generated | {datetime.now().strftime('%Y-%m-%d %H:%M')} |",
        f"| Commit | `{git_sha}` |",
        f"| perl-parser | v{parser_version} |",
        f"| Corpus | `{corpus_path}` |",
        f"| Command | `just parser-audit && just parser-matrix-update` |",
        "",
        "## Summary",
        "",
        "| Metric | Current | Target | Status |",
        "|--------|---------|--------|--------|",
        f"| Parse Success Rate | {success_rate:.0f}% ({ok}/{total} files) | 100% | {'Passing' if errors == 0 else 'In Progress'} |",
        f"| Parse Errors | {errors} | 0 | {'Passing' if errors == 0 else 'Baseline Set'} |",
        f"| Timeouts | {timeouts} | 0 | {'Passing' if timeouts == 0 else 'Failed'} |",
        f"| Panics | {panics} | 0 | {'Passing' if panics == 0 else 'Failed'} |",
        f"| Test Corpus Inventory | {ga_coverage:.0f}% | 100% | {'Passing' if ga_coverage >= 80 else 'In Progress'} |",
    ]

    if baseline is not None:
        lines.append(f"| Baseline | {baseline} | 0 | Ratcheted |")

    lines.extend([
        "",
        "*Test Corpus Inventory* measures whether the test corpus contains examples of each",
        "GA (generally available) feature defined in `features.toml`. It does NOT measure",
        "whether those features parse successfully—that's what Parse Success Rate tracks.",
        "",
        "## Error Breakdown by Category",
        "",
        "Errors are categorized to help prioritize implementation work:",
        "",
        "| Category | Count | Priority | Description |",
        "|----------|-------|----------|-------------|",
    ])

    # Full taxonomy - show all categories even if count is 0
    category_taxonomy = {
        "ModernFeature": ("P1", "class/try/catch/field/method keywords"),
        "QuoteLike": ("P2", "q/qq/qw/qx/qr, heredocs, strings"),
        "Regex": ("P2", "m//, s///, tr///, patterns"),
        "ControlFlow": ("P2", "given/when/default"),
        "Dereference": ("P2", "->, postfix deref"),
        "Subroutine": ("P2", "Signatures, prototypes"),
        "General": ("P3", "Uncategorized"),
    }

    # Sort categories by count descending, then alphabetically for zero counts
    all_categories = {cat: error_by_category.get(cat, 0) for cat in category_taxonomy}
    sorted_categories = sorted(all_categories.items(), key=lambda x: (-x[1], x[0]))

    for category, count in sorted_categories:
        priority, desc = category_taxonomy[category]
        lines.append(f"| {category} | {count} | {priority} | {desc} |")

    lines.extend([
        "",
        "## Failing Files",
        "",
    ])

    if failing_files:
        for f in failing_files:
            path = f['path']
            category = f['category']

            # Location info
            line_num = f.get("line_number")
            col = f.get("column")
            location = ""
            if line_num:
                location = f"line {line_num}"
                if col:
                    location += f":{col}"

            # Token info
            found = f.get("found_token", "")
            expected = f.get("expected", "")
            token_info = ""
            if expected and found:
                token_info = f"expected `{expected}`, found `{found}`"
            elif expected:
                token_info = f"expected `{expected}`"

            # Code snippet
            snippet = f.get("code_snippet", "")

            # Build the entry
            lines.append(f"### `{path}`")
            lines.append("")
            lines.append(f"- **Category**: {category}")
            if location:
                lines.append(f"- **Location**: {location}")
            if token_info:
                lines.append(f"- **Error**: {token_info}")

            if snippet:
                lines.append("")
                lines.append("```perl")
                lines.append(snippet)
                lines.append("```")

            lines.append("")
    else:
        lines.append("*No failing files* ✅")

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
        "",
        "- Baseline stored in `ci/parse_errors_baseline.txt`",
        "- CI fails if parse errors **increase**",
        "- CI passes if parse errors stay same or decrease",
        "- When errors decrease, update baseline: `echo N > ci/parse_errors_baseline.txt`",
        "",
        "**Philosophy**: Baseline updates are only allowed when the parser actually improves",
        "(error count decreases), never to paper over regressions. The ratchet ensures the",
        "codebase only gets easier to reason about over time.",
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
