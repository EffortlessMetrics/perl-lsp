#!/usr/bin/env python3
from __future__ import annotations

import argparse
import pathlib
import re
import sys
try:
    import tomllib
except ImportError:  # pragma: no cover
    import tomli as tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
FEATURES_TOML = ROOT / "features.toml"
CURRENT_STATUS = ROOT / "docs" / "CURRENT_STATUS.md"
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


def _replace_line(text: str, pattern: str, replacement) -> str:
    updated, count = re.subn(pattern, replacement, text, flags=re.M)
    if count != 1:
        raise ValueError(f"Expected 1 match for pattern {pattern!r}, got {count}")
    return updated


def _update_current_status() -> str:
    percent, advertised, trackable = _count_lsp_coverage()
    corpus_sections = _count_corpus_sections()
    gap_files = _count_gap_files()

    lsp_value = f"{percent}% ({advertised}/{trackable} GA advertised, `features.toml`)"
    lsp_strength = (
        f"- **Solid LSP foundation**: {percent}% cataloged GA coverage "
        f"({advertised}/{trackable} trackable features), production-ready (`just ci-gate`)"
    )
    lsp_target = f"**Target**: 93%+ LSP coverage (from {percent}% catalog)"
    lsp_status = (
        f"- **Status**: 🟢 **Production** - {percent}% LSP 3.18 cataloged feature coverage"
    )
    lsp_competitive = f"- {percent}% LSP coverage vs ~40-70%"

    parsing_line = (
        "- **Comprehensive parsing**: broad Perl 5 coverage via "
        f"`tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + "
        f"`test_corpus/` ({gap_files} `.pl` files)"
    )
    corpus_status = (
        "- **Status**: 🟢 **Production** - Corpus sources: "
        f"`tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + "
        f"`test_corpus/` ({gap_files} `.pl` files)"
    )

    text = CURRENT_STATUS.read_text(encoding="utf-8")

    text = _replace_line(
        text,
        r"^(\| \*\*LSP Coverage\*\* \|) [^|]*(\| .*\|)$",
        lambda m: f"{m.group(1)} {lsp_value} {m.group(2)}",
    )
    text = _replace_line(text, r"^- \*\*Comprehensive parsing\*\*: .*", parsing_line)
    text = _replace_line(text, r"^- \*\*Solid LSP foundation\*\*: .*", lsp_strength)
    text = _replace_line(text, r"^\*\*Target\*\*: 93%\+ LSP coverage .*", lsp_target)
    text = _replace_line(
        text,
        r"^- \*\*Status\*\*: 🟢 \*\*Production\*\* - .*LSP .*feature coverage$",
        lsp_status,
    )
    text = _replace_line(
        text,
        r"^- \*\*Status\*\*: 🟢 \*\*Production\*\* - Corpus sources: .*",
        corpus_status,
    )
    text = _replace_line(text, r"^- \d+% LSP coverage vs ~40-70%", lsp_competitive)

    return text


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Update derived metrics in docs/CURRENT_STATUS.md"
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write updates back to docs/CURRENT_STATUS.md",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check whether docs/CURRENT_STATUS.md is up-to-date",
    )
    args = parser.parse_args()

    if not args.write and not args.check:
        args.check = True

    updated = _update_current_status()
    original = CURRENT_STATUS.read_text(encoding="utf-8")

    if updated == original:
        return 0

    if args.write:
        CURRENT_STATUS.write_text(updated, encoding="utf-8")
        return 0

    sys.stderr.write("docs/CURRENT_STATUS.md is out of date.\n")
    sys.stderr.write("Run `just status-update`\n")
    sys.stderr.write("Then re-run `just ci-gate`\n")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
