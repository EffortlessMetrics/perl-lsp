#!/usr/bin/env python3
"""Check that article inline claims match values in PUBLICATION_FACTS_LEDGER.md.

Flags known-stale string patterns found in docs/articles/*.md.
This is a claim-match linter: it checks known-wrong values, not all numbers.

Exit code 0 = clean.
Exit code 1 = stale claims detected.
"""

from __future__ import annotations

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ARTICLES_DIR = ROOT / "docs" / "articles"

# Each entry: (stale_pattern, correct_replacement, description)
# These are patterns that are known to be wrong based on PUBLICATION_FACTS_LEDGER.md.
# Only scan docs/articles/*.md (not docs/articles/research/ — historical notes).
STALE_PATTERNS: list[tuple[str, str, str]] = [
    # Lines-of-Rust claims
    ("563,228 lines", "591,034 lines", "LOC claim (563K is stale; ledger: 591,034)"),
    ("563K lines", "591K lines", "LOC claim (563K is stale; ledger: 591K)"),
    ("546,000", "591,034", "LOC claim (546K is stale; ledger: 591,034)"),
    ("546K lines", "591K lines", "LOC claim (546K is stale; ledger: 591K)"),
    # Workspace crate counts
    ("131 crates", "133 crates", "Crate count (131 is stale; ledger: 133)"),
    ("131 workspace crates", "133 workspace crates", "Crate count (131 is stale; ledger: 133)"),
    ("132 workspace crates", "133 workspace crates", "Crate count (132 is stale; ledger: 133)"),
    ("132 crates", "133 crates", "Crate count (132 is stale; ledger: 133)"),
    # LSP feature counts
    ("97 LSP and DAP features", "98 LSP and DAP features", "Feature count (97 is stale; ledger: 98)"),
    ("97 features defined", "98 features defined", "Feature count (97 is stale; ledger: 98)"),
    ("97 features governed", "98 features governed", "Feature count (97 is stale; ledger: 98)"),
    # Commit counts
    ("2,700+ commits", "3,200+ commits", "Commit count (2,700+ is stale; ledger: 3,210)"),
    # PR counts
    ("2,200+ pull requests", "2,646+ pull requests", "PR count (2,200+ is stale; ledger: 2,646+)"),
    ("2,200+ PRs", "2,646+ PRs", "PR count (2,200+ is stale; ledger: 2,646+)"),
]


def scan_articles() -> list[tuple[pathlib.Path, int, str, str, str]]:
    """Scan docs/articles/*.md for stale patterns.

    Returns list of (file, line_number, matched_pattern, replacement, description).
    """
    hits: list[tuple[pathlib.Path, int, str, str, str]] = []

    # Only scan top-level articles, not research/ subdirectory (historical notes)
    for md_file in sorted(ARTICLES_DIR.glob("*.md")):
        text = md_file.read_text(encoding="utf-8")
        lines = text.splitlines()
        for lineno, line in enumerate(lines, start=1):
            for stale, replacement, description in STALE_PATTERNS:
                if stale in line:
                    hits.append((md_file, lineno, stale, replacement, description))

    return hits


def main() -> int:
    hits = scan_articles()

    if hits:
        print("DOC CLAIM VIOLATIONS:", file=sys.stderr)
        print("=" * 60, file=sys.stderr)
        for filepath, lineno, stale, replacement, description in hits:
            rel = filepath.relative_to(ROOT)
            print(f"  {rel}:{lineno}: {description}", file=sys.stderr)
            print(f"    found:    {stale!r}", file=sys.stderr)
            print(f"    expected: {replacement!r}", file=sys.stderr)
        print("=" * 60, file=sys.stderr)
        print(
            f"\n{len(hits)} stale claim(s) found in docs/articles/.",
            file=sys.stderr,
        )
        print(
            "\nTo fix: update the article to match docs/project/PUBLICATION_FACTS_LEDGER.md",
            file=sys.stderr,
        )
        return 1

    article_count = len(list(ARTICLES_DIR.glob("*.md")))
    pattern_count = len(STALE_PATTERNS)
    print(
        f"Doc claims OK: {article_count} articles scanned, "
        f"{pattern_count} stale patterns checked, 0 violations found"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
