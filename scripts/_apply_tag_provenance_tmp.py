#!/usr/bin/env python3
"""One-shot guarded edits for the release tag provenance audit."""

from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def update_manifest_shas() -> None:
    path = Path("policy/release-tag-provenance.toml")
    replacements = {
        "cc801735bdfd81d79da056de6145c32381b081d1":
            "cc801735b004f89dc1c1b8789658f7abc73bf4aa",
        "181d2b2db1fb56c3c30135cd9ed7b5e9f0470a96":
            "181d2b2d2d8a5fc7e65fbceda648117eb04631a9",
        "4e4099cde6ba96d19b43d93f56e7bd5297116dfa":
            "4e4099cdd9f5e29f21412285224cfc95db4f9c53",
        "0e9c5d7864f4c8cb19e02887cfd76a8fb74020de":
            "0e9c5d789836938845dbe0921549cde881f47a21",
        "15cbe7e6cdb831eb4738ed8d5b7b14451ac24182":
            "15cbe7e6295a67ea0cba506c3cade628ee4847f6",
    }
    text = path.read_text(encoding="utf-8")
    for old, new in replacements.items():
        count = text.count(old)
        if count != 1:
            raise SystemExit(
                f"manifest SHA correction {old[:8]}: expected one match, found {count}"
            )
        text = text.replace(old, new, 1)
    path.write_text(text, encoding="utf-8")


def update_release_history() -> None:
    path = Path("RELEASE_HISTORY.md")
    replace_once(
        path,
        """The original `v0.13.4` asset count and channel cells are not evidence of a
standalone 0.13.4 publication and should not be used as such.

### Legend
""",
        """The original `v0.13.4` asset count and channel cells are not evidence of a
standalone 0.13.4 publication and should not be used as such.

### 2026-07-12 — live tag SHA and branch-line audit

The live ref audit found that several tag SHAs previously written in this ledger
or standalone release notes no longer match the commit reached by the named tag.
For multiple affected releases, the originally recorded full SHA no longer
resolves in the repository.

The complete immutable inventory is maintained in
[`policy/release-tag-provenance.toml`](policy/release-tag-provenance.toml), with
human guidance in [`docs/releases/TAG_PROVENANCE.md`](docs/releases/TAG_PROVENANCE.md).
The original rows above remain unchanged as historical evidence.

Key corrections:

- the current tags from `v0.1.0-pest` through `v0.8.5` are linear even though
  most of their recorded SHAs are stale;
- `v0.8.5` and `v0.9.1` are divergent, so their GitHub comparison is not a
  forward release range;
- `v0.11.0` descends from `v0.9.1`, not from the divergent `v0.8.5` line;
- the live refs for `v0.15.0`, `v0.16.0`, and `v0.17.0` are now pinned in the
  provenance manifest instead of remaining `pending` only;
- affected `Tag commit` cells in the original table are prior recorded values,
  not current live-ref truth. Use the provenance manifest for current SHAs.

No cause, actor, or rewrite date is inferred from the mismatch. The correction
records observable repository state and installs a drift guard.

### Legend
""",
        "release-history correction section",
    )
    replace_once(
        path,
        """[v0.11.0...v0.12.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.11.0...v0.12.0
[v0.8.5...v0.11.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.11.0
[v0.8.3...v0.8.5]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.3...v0.8.5
""",
        """[v0.11.0...v0.12.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.11.0...v0.12.0
[v0.9.1...v0.11.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.9.1...v0.11.0
[v0.8.5...v0.11.0]: docs/releases/TAG_PROVENANCE.md
[v0.8.3...v0.8.5]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.3...v0.8.5
""",
        "release-history compare links",
    )


def update_changelog() -> None:
    path = Path("CHANGELOG.md")
    replace_once(
        path,
        """Release notes: [v0.11.0](docs/releases/v0.11.0.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.11.0)

This release finalizes the 0.11.0 distribution pipeline across GitHub releases,
""",
        """Release notes: [v0.11.0](docs/releases/v0.11.0.md) · [GitHub Release](https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.11.0)

> **Tag provenance correction.** The live `v0.11.0` ref is
> `8dfa68860cdf8fc220b1345d3b943668d1393ad2`; the previously recorded
> `d22ac7346c832db6b92c41d354eb90099f8b5d53` no longer resolves. The current
> tagged predecessor is `v0.9.1`, because `0.10.0` was changelog-only and the
> current `v0.8.5` / `v0.9.1` refs are divergent. Use
> [`v0.9.1...v0.11.0`](https://github.com/EffortlessMetrics/perl-lsp/compare/v0.9.1...v0.11.0).

This release finalizes the 0.11.0 distribution pipeline across GitHub releases,
""",
        "0.11 changelog provenance",
    )
    replace_once(
        path,
        """Release notes: [v0.9.1](docs/releases/v0.9.1.md) (tag only — no GitHub Release)

### Added
""",
        """Release notes: [v0.9.1](docs/releases/v0.9.1.md) (tag only — no GitHub Release)

> **Tag provenance correction.** The live `v0.9.1` ref is
> `0e52877de7763d8654e0fb6d7afe6a257639e584`; the previously recorded
> `c82a1604987f315868973a4e5804112e031cec92` no longer resolves. The current
> tag is on a divergent line from `v0.8.5`, so no forward comparison from
> `v0.8.5` is claimed.

### Added
""",
        "0.9.1 changelog provenance",
    )


def update_release_process() -> None:
    path = Path("docs/RELEASE_PROCESS.md")
    replace_once(
        path,
        """### Step 5: Verify Release

After all workflows complete, verify:
""",
        """### Step 5: Verify Release

Before channel closeout, snapshot and verify the tag commit:

```bash
git fetch --force --tags origin
git rev-parse v<0.x.y>^{commit}
python3 scripts/check_release_tag_provenance.py --verify-git
```

Add the new tag, exact 40-character SHA, predecessor, and expected lineage to
`policy/release-tag-provenance.toml`. A release is not provenance-closed while
the manifest still says `pending` or the local-git verification fails. See
[`docs/releases/TAG_PROVENANCE.md`](releases/TAG_PROVENANCE.md) for the audit and
exception procedure.

After all workflows complete, verify:
""",
        "release process provenance step",
    )


def main() -> None:
    update_manifest_shas()
    update_release_history()
    update_changelog()
    update_release_process()


if __name__ == "__main__":
    main()
