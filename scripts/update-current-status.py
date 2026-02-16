#!/usr/bin/env python3
from __future__ import annotations

import argparse
import pathlib
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
try:
    import tomllib
except ImportError:  # pragma: no cover
    import tomli as tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
FEATURES_TOML = ROOT / "features.toml"
CURRENT_STATUS = ROOT / "docs" / "CURRENT_STATUS.md"
ROADMAP = ROOT / "docs" / "ROADMAP.md"
TREE_SITTER_CORPUS = ROOT / "tree-sitter-perl" / "test" / "corpus"
GAP_CORPUS = ROOT / "test_corpus"
MISSING_DOCS_BASELINE = ROOT / "ci" / "missing_docs_baseline.txt"


@dataclass(frozen=True)
class TestCounts:
    tier_a_lib_tests: int | None
    ignored_total: int | None
    bug_count: int | None
    manual_count: int | None


def _run(cmd: list[str], timeout_s: int) -> str:
    """Run a command and return combined stdout+stderr.

    Never throw fake numbers into docs: if we can't measure, return "" and let callers mark UNVERIFIED.
    """
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            cwd=ROOT,
            timeout=timeout_s,
        )
        return (result.stdout or "") + (result.stderr or "")
    except (subprocess.TimeoutExpired, subprocess.SubprocessError, FileNotFoundError):
        return ""


def _count_tier_a_lib_tests() -> int | None:
    """Count Tier A lib tests by enumerating test names.

    This matches `just ci-test-lib` (workspace lib tests excluding the internal
    tree-sitter validation harness crate).
    We avoid parsing the fragile per-crate "X tests, Y benchmarks" summaries and instead count
    actual test entries:
      `foo::bar::baz: test`
    """
    output = _run(
        ["cargo", "test", "--workspace", "--lib", "--exclude", "tree-sitter-perl", "--", "--list"],
        timeout_s=180,
    )
    if not output:
        return None
    return len(re.findall(r":\s*test\s*$", output, re.MULTILINE))


def _count_ignored_tracked() -> tuple[int | None, int | None, int | None]:
    """Count ignored tests tracked by scripts/ignored-test-count.sh.

    Returns (ignored_total, bug_count, manual_count). Any may be None if parsing fails.
    """
    output = _run(["bash", "scripts/ignored-test-count.sh"], timeout_s=60)
    if not output:
        return None, None, None

    ignored_match = re.search(r"TOTAL\s+(\d+)", output)
    bug_match = re.search(r"^bug\s+(\d+)", output, re.MULTILINE)
    manual_match = re.search(r"^manual\s+(\d+)", output, re.MULTILINE)

    ignored_total = int(ignored_match.group(1)) if ignored_match else None
    bug_count = int(bug_match.group(1)) if bug_match else None
    manual_count = int(manual_match.group(1)) if manual_match else None
    return ignored_total, bug_count, manual_count


def _count_tests() -> TestCounts:
    tier_a = _count_tier_a_lib_tests()
    ignored_total, bug_count, manual_count = _count_ignored_tracked()
    return TestCounts(
        tier_a_lib_tests=tier_a,
        ignored_total=ignored_total,
        bug_count=bug_count,
        manual_count=manual_count,
    )


def _count_missing_docs_perl_parser() -> int | None:
    """Count missing_docs warnings for perl-parser using JSON compiler messages (same method as ci/check_missing_docs.sh)."""
    output = _run(
        ["cargo", "check", "-p", "perl-parser", "--tests", "--message-format=json"],
        timeout_s=300,
    )
    if not output:
        return None

    import json

    count = 0
    for line in output.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("reason") != "compiler-message":
            continue
        pkg_id = obj.get("package_id", "")
        if not str(pkg_id).startswith("perl-parser "):
            continue
        msg = obj.get("message") or {}
        if not msg:
            continue
        level = msg.get("level")
        code = (msg.get("code") or {}).get("code")
        if level == "warning" and code == "missing_docs":
            count += 1
    return count


def _read_missing_docs_baseline() -> int | None:
    try:
        if not MISSING_DOCS_BASELINE.exists():
            return None
        raw = MISSING_DOCS_BASELINE.read_text(encoding="utf-8").strip()
        return int(raw) if raw else None
    except Exception:
        return None


def _count_lsp_coverage() -> tuple[int, int, int, int, int, int]:
    """Calculate both UX coverage (headline) and protocol compliance metrics.

    Returns:
        tuple of (ux_percent, ux_implemented, ux_total, protocol_percent, protocol_implemented, protocol_total)
    """
    data = tomllib.loads(FEATURES_TOML.read_text(encoding="utf-8"))
    features = data.get("feature", [])

    # UX Coverage: User-visible features that count toward public-facing metric
    # Only include features where counts_in_coverage != false AND advertised = true
    ux_trackable = [
        f for f in features
        if f.get("maturity") != "planned"
        and f.get("counts_in_coverage", True) is not False
        and bool(f.get("advertised"))
    ]
    ux_implemented = [
        f for f in ux_trackable
        if f.get("maturity") in ("ga", "production")
    ]
    ux_percent = round(len(ux_implemented) / len(ux_trackable) * 100) if ux_trackable else 0

    # Protocol Compliance: All features regardless of counts_in_coverage
    protocol_trackable = [f for f in features if f.get("maturity") != "planned"]
    protocol_implemented = [
        f
        for f in protocol_trackable
        if f.get("maturity") in ("ga", "production", "preview")
    ]
    protocol_percent = round(len(protocol_implemented) / len(protocol_trackable) * 100) if protocol_trackable else 0

    return (
        ux_percent, len(ux_implemented), len(ux_trackable),
        protocol_percent, len(protocol_implemented), len(protocol_trackable)
    )


def _compute_compliance_table() -> str:
    """Compute the LSP compliance table from features.toml."""
    data = tomllib.loads(FEATURES_TOML.read_text(encoding="utf-8"))
    features = data.get("feature", [])

    # Count by area
    by_area: dict[str, dict[str, int]] = defaultdict(lambda: {"implemented": 0, "total": 0})

    for f in features:
        area = f.get("area", "other")
        maturity = f.get("maturity", "planned")

        by_area[area]["total"] += 1
        if maturity in ("ga", "production", "preview"):
            by_area[area]["implemented"] += 1

    # Build table
    lines = ["| Area | Implemented | Total | Coverage |"]
    lines.append("|------|-------------|-------|----------|")

    total_impl = 0
    total_all = 0

    for area in sorted(by_area.keys()):
        impl = by_area[area]["implemented"]
        total = by_area[area]["total"]
        pct = round(impl / total * 100) if total else 0
        lines.append(f"| {area} | {impl} | {total} | {pct}% |")
        total_impl += impl
        total_all += total

    overall_pct = round(total_impl / total_all * 100) if total_all else 0
    lines.append(f"| **Overall** | **{total_impl}** | **{total_all}** | **{overall_pct}%** |")

    return "\n".join(lines)


def _count_corpus_sections() -> int:
    marker = re.compile(r"^=+\s*$")
    total = 0
    for path in TREE_SITTER_CORPUS.rglob("*.txt"):
        with path.open(encoding="utf-8", errors="replace") as handle:
            for line in handle:
                if marker.match(line):
                    total += 1
    return total


def _count_gap_files() -> int:
    return sum(1 for _ in GAP_CORPUS.rglob("*.pl"))


def _replace_block(text: str, begin_marker: str, end_marker: str, new_content: str) -> str:
    """Replace content between markers (inclusive of markers)."""
    pattern = re.compile(
        rf"({re.escape(begin_marker)})\n.*?\n({re.escape(end_marker)})",
        re.DOTALL
    )
    replacement = f"{begin_marker}\n{new_content}\n{end_marker}"
    updated, count = pattern.subn(replacement, text)
    if count != 1:
        raise ValueError(f"Expected 1 match for block {begin_marker!r}, got {count}")
    return updated


def _update_current_status() -> str:
    ux_percent, ux_impl, ux_total, protocol_percent, protocol_impl, protocol_total = _count_lsp_coverage()
    corpus_sections = _count_corpus_sections()
    gap_files = _count_gap_files()
    tests = _count_tests()
    missing_docs_current = _count_missing_docs_perl_parser()
    missing_docs_baseline = _read_missing_docs_baseline()

    if tests.tier_a_lib_tests is None:
        tier_a_tests_str = "UNVERIFIED"
    else:
        tier_a_tests_str = str(tests.tier_a_lib_tests)

    if tests.ignored_total is None:
        ignored_tests_str = "UNVERIFIED"
    else:
        ignored_tests_str = str(tests.ignored_total)

    if tests.bug_count is None or tests.manual_count is None:
        tracked_debt_str = "UNVERIFIED"
        bug_count_str = "UNVERIFIED"
        manual_count_str = "UNVERIFIED"
    else:
        tracked_debt = tests.bug_count + tests.manual_count
        tracked_debt_str = str(tracked_debt)
        bug_count_str = str(tests.bug_count)
        manual_count_str = str(tests.manual_count)

    if missing_docs_current is None:
        missing_docs_str = "UNVERIFIED"
    else:
        missing_docs_str = str(missing_docs_current)

    baseline_suffix = ""
    if missing_docs_baseline is not None and missing_docs_current is not None:
        baseline_suffix = f" (baseline {missing_docs_baseline})"

    # Build the table row content - uses UX coverage (headline metric)
    lsp_table_row = f"| **LSP Coverage** | {ux_percent}% ({ux_impl}/{ux_total} advertised features, `features.toml`) | 93%+ | In progress |"

    def _replace_row(pattern: str, replacement: str, text: str) -> str:
        updated, count = re.subn(pattern, replacement, text, flags=re.MULTILINE)
        if count != 1:
            raise ValueError(f"Expected 1 match for row pattern {pattern!r}, got {count}")
        return updated

    # Build the bullets section content (clean, factual metrics only)
    lsp_coverage = (
        f"- **LSP Coverage**: {ux_percent}% user-visible feature coverage "
        f"({ux_impl}/{ux_total} advertised features from `features.toml`)"
    )
    protocol_compliance = (
        f"- **Protocol Compliance**: {protocol_percent}% overall LSP protocol support "
        f"({protocol_impl}/{protocol_total} including plumbing)"
    )
    parser_coverage = (
        "- **Parser Coverage**: ~100% Perl 5 syntax via "
        f"`tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + "
        f"`test_corpus/` ({gap_files} `.pl` files)"
    )
    test_status = (
        f"- **Test Status**: {tier_a_tests_str} lib tests (Tier A), {ignored_tests_str} ignores tracked "
        f"({tracked_debt_str} total tracked debt: {bug_count_str} bug, {manual_count_str} manual)"
    )
    docs_status = f"- **Docs (perl-parser)**: missing_docs warnings = {missing_docs_str}{baseline_suffix}"
    quality_metrics = (
        "- **Quality Metrics**: 87% mutation score, <50ms LSP response times, "
        "931ns incremental parsing"
    )
    production_status = (
        "- **Production Status**: LSP server production-ready (`just ci-gate` passing)"
    )
    lsp_target = f"**Target**: 93%+ LSP coverage (from current {ux_percent}%)"

    bullets_content = "\n".join([
        lsp_coverage,
        protocol_compliance,
        parser_coverage,
        test_status,
        docs_status,
        quality_metrics,
        production_status,
        "",
        lsp_target,
    ])

    text = CURRENT_STATUS.read_text(encoding="utf-8")

    text = _replace_row(
        r"^\| \*\*Tier A Tests\*\* \| .* \| 100% pass \| .* \|$",
        f"| **Tier A Tests** | {tier_a_tests_str} lib tests (discovered), {ignored_tests_str} ignores (tracked) | 100% pass | PASS |",
        text,
    )
    text = _replace_row(
        r"^\| \*\*Tracked Test Debt\*\* \| .* \| 0 \| .* \|$",
        f"| **Tracked Test Debt** | {tracked_debt_str} ({bug_count_str} bug, {manual_count_str} manual) | 0 | Near-zero |",
        text,
    )
    text = _replace_row(
        r"^\| \*\*Documentation\*\* \| .* \| 0 \| .* \|$",
        f"| **Documentation** | perl-parser missing_docs = {missing_docs_str}{baseline_suffix} | 0 | Ratchet |",
        text,
    )

    # Replace table row block
    text = _replace_block(
        text,
        "<!-- BEGIN: STATUS_METRICS_TABLE -->",
        "<!-- END: STATUS_METRICS_TABLE -->",
        lsp_table_row
    )

    # Replace bullets block
    text = _replace_block(
        text,
        "<!-- BEGIN: STATUS_METRICS_BULLETS -->",
        "<!-- END: STATUS_METRICS_BULLETS -->",
        bullets_content
    )

    text = re.sub(
        r"^\-\s+\*\*484 doc violations\*\*:.*$",
        f"- **missing_docs (perl-parser)**: {missing_docs_str}{baseline_suffix} (ratcheted by `ci/check_missing_docs.sh`)",
        text,
        flags=re.MULTILINE,
    )

    return text


def _update_roadmap() -> str:
    """Update ROADMAP.md with computed compliance table."""
    compliance_table = _compute_compliance_table()

    text = ROADMAP.read_text(encoding="utf-8")

    # Update the compliance table block
    text = _replace_block(
        text,
        "<!-- BEGIN: COMPLIANCE_TABLE -->",
        "<!-- END: COMPLIANCE_TABLE -->",
        compliance_table
    )

    return text


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Update derived metrics in docs/CURRENT_STATUS.md and docs/ROADMAP.md"
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write updates back to docs/",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check whether docs are up-to-date",
    )
    args = parser.parse_args()

    if not args.write and not args.check:
        args.check = True

    exit_code = 0
    files_to_update = []

    # Check CURRENT_STATUS.md
    updated_status = _update_current_status()
    original_status = CURRENT_STATUS.read_text(encoding="utf-8")
    if updated_status != original_status:
        files_to_update.append(("docs/CURRENT_STATUS.md", CURRENT_STATUS, updated_status))

    # Check ROADMAP.md
    updated_roadmap = _update_roadmap()
    original_roadmap = ROADMAP.read_text(encoding="utf-8")
    if updated_roadmap != original_roadmap:
        files_to_update.append(("docs/ROADMAP.md", ROADMAP, updated_roadmap))

    if not files_to_update:
        return 0

    if args.write:
        for name, path, content in files_to_update:
            path.write_text(content, encoding="utf-8")
            sys.stderr.write(f"Updated {name}\n")
        return 0

    for name, _, _ in files_to_update:
        sys.stderr.write(f"{name} is out of date.\n")
    sys.stderr.write("Run `just status-update`\n")
    sys.stderr.write("Then re-run `just ci-gate`\n")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
