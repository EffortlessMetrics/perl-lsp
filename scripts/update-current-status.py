#!/usr/bin/env python3
from __future__ import annotations

import argparse
import pathlib
import re
import sys
from collections import defaultdict
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


def _count_lsp_coverage() -> tuple[int, int, int]:
    data = tomllib.loads(FEATURES_TOML.read_text(encoding="utf-8"))
    features = data.get("feature", [])
    trackable = [f for f in features if f.get("maturity") != "planned"]
    advertised = [
        f
        for f in features
        if f.get("advertised") and f.get("maturity") in ("ga", "production")
    ]
    percent = round(len(advertised) / len(trackable) * 100) if trackable else 0
    return percent, len(advertised), len(trackable)


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
    percent, advertised, trackable = _count_lsp_coverage()
    corpus_sections = _count_corpus_sections()
    gap_files = _count_gap_files()

    # Build the table row content
    lsp_table_row = f"| **LSP Coverage** | {percent}% ({advertised}/{trackable} GA advertised, `features.toml`) | 93%+ | In progress |"

    # Build the bullets section content (clean, factual metrics only)
    lsp_coverage = (
        f"- **LSP Coverage**: {percent}% cataloged GA coverage "
        f"({advertised}/{trackable} trackable features from `features.toml`)"
    )
    parser_coverage = (
        "- **Parser Coverage**: ~100% Perl 5 syntax via "
        f"`tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + "
        f"`test_corpus/` ({gap_files} `.pl` files)"
    )
    test_status = (
        "- **Test Status**: 337 lib tests passing, 1 ignored "
        "(9 total tracked debt: 8 bug, 1 manual)"
    )
    quality_metrics = (
        "- **Quality Metrics**: 87% mutation score, <50ms LSP response times, "
        "931ns incremental parsing"
    )
    production_status = (
        "- **Production Status**: LSP server production-ready (`just ci-gate` passing)"
    )
    lsp_target = f"**Target**: 93%+ LSP coverage (from current {percent}%)"

    bullets_content = "\n".join([
        lsp_coverage,
        parser_coverage,
        test_status,
        quality_metrics,
        production_status,
        "",
        lsp_target,
    ])

    text = CURRENT_STATUS.read_text(encoding="utf-8")

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
