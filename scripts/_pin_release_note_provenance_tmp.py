#!/usr/bin/env python3
"""One-shot frontmatter provenance pins for PR #9956."""

from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one exact match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    Path("docs/releases/v0.15.1.md"),
    'tag: "v0.15.1"\nrelease_date_utc:',
    'tag: "v0.15.1"\ntag_commit: "15cbe7e6295a67ea0cba506c3cade628ee4847f6"\nrelease_date_utc:',
)
replace_once(
    Path("docs/releases/v0.16.0.md"),
    'tag: "v0.16.0"\nrelease_date_utc:',
    'tag: "v0.16.0"\ntag_commit: "b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d"\nsource_compare_classification: incomplete\nrelease_date_utc:',
)
replace_once(
    Path("docs/releases/v0.17.0.md"),
    'tag: "v0.17.0"\nrelease_date_utc:',
    'tag: "v0.17.0"\ntag_commit: "ffee2824938f415e54923112c7b79e3f22040699"\nsource_compare_classification: inflated\nrelease_date_utc:',
)
