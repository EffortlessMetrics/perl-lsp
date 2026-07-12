#!/usr/bin/env python3
"""One-shot correction for the published v0.13.0-rc1 prerelease ledger row."""

from pathlib import Path

path = Path("RELEASE_HISTORY.md")
text = path.read_text(encoding="utf-8")

old_row = "| [0.13.0-rc1] | `v0.13.0-rc1` | unreconciled | 2026-04-30 (tag) | `4e4099cd` | [v0.12.4...v0.13.0-rc1] | pending verification | pending verification | pending verification | [v0.13.0-rc1][n-0.13.0-rc1] |"
new_row = "| [0.13.0-rc1] | `v0.13.0-rc1` | [yes][gh-0.13.0-rc1] (prerelease) | 2026-04-30 | `4e4099cd` | [v0.12.4...v0.13.0-rc1] | 11 | pending verification | pending verification | [v0.13.0-rc1][n-0.13.0-rc1] |"
if text.count(old_row) != 1:
    raise SystemExit(f"expected one RC ledger row, found {text.count(old_row)}")
text = text.replace(old_row, new_row, 1)

anchor = "[gh-0.13.1]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.13.1\n"
addition = anchor + "[gh-0.13.0-rc1]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.13.0-rc1\n"
if text.count(anchor) != 1:
    raise SystemExit(f"expected one GitHub release link anchor, found {text.count(anchor)}")
text = text.replace(anchor, addition, 1)

path.write_text(text, encoding="utf-8")
