#!/usr/bin/env python3
"""One-shot removal of a dead release-history link definition."""

from pathlib import Path

path = Path("RELEASE_HISTORY.md")
text = path.read_text(encoding="utf-8")
line = "[gh-0.13.4]: #lineage-corrections\n"
count = text.count(line)
if count != 1:
    raise SystemExit(f"expected one unused gh-0.13.4 reference, found {count}")
path.write_text(text.replace(line, "", 1), encoding="utf-8")
