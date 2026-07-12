#!/usr/bin/env python3
"""One-shot guarded note restoration for issue #9965."""

from pathlib import Path


REPO = "https://github.com/EffortlessMetrics/perl-lsp"


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def update_v0150() -> None:
    path = Path("docs/releases/v0.15.0.md")
    replace_once(
        path,
        '''---
version: "0.15.0"
tag: "v0.15.0"
release_date_utc: "2026-05-22"
previous_tag: "v0.14.0"
compare: "https://github.com/EffortlessMetrics/perl-lsp/compare/v0.14.0...v0.15.0"
notes_status: draft
release_track: public-alpha
release_kind: minor
channels:
  github_release: pending
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        f'''---
version: "0.15.0"
tag: "v0.15.0"
tag_commit: "ac8e281e73c6e14ae9d94ddf010ae0d45d1187d2"
release_date_utc: "2026-05-22"
previous_tag: "v0.14.0"
compare: "{REPO}/compare/v0.14.0...v0.15.0"
github_release: "{REPO}/releases/tag/v0.15.0"
notes_status: draft
release_track: public-alpha
release_kind: minor
channels:
  github_release: "published 2026-05-22; see canonical release page for asset inventory"
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        "v0.15.0 frontmatter",
    )
    replace_once(
        path,
        '''## Related

- Previous release: [v0.14.0](v0.14.0.md)
''',
        f'''## Related

- GitHub Release: [{REPO}/releases/tag/v0.15.0]({REPO}/releases/tag/v0.15.0)
- Previous release: [v0.14.0](v0.14.0.md)
''',
        "v0.15.0 related links",
    )


def update_v0151() -> None:
    path = Path("docs/releases/v0.15.1.md")
    replace_once(
        path,
        '''---
version: "0.15.1"
tag: "v0.15.1"
tag_commit: "15cbe7e6295a67ea0cba506c3cade628ee4847f6"
release_date_utc: "2026-05-26"
previous_tag: "v0.15.0"
compare: "https://github.com/EffortlessMetrics/perl-lsp/compare/v0.15.0...v0.15.1"
notes_status: draft
release_track: public-alpha
release_kind: patch
channels:
  github_release: pending
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        f'''---
version: "0.15.1"
tag: "v0.15.1"
tag_commit: "15cbe7e6295a67ea0cba506c3cade628ee4847f6"
release_date_utc: "2026-05-26"
previous_tag: "v0.15.0"
compare: "{REPO}/compare/v0.15.0...v0.15.1"
github_release: "{REPO}/releases/tag/v0.15.1"
notes_status: draft
release_track: public-alpha
release_kind: patch
channels:
  github_release: "published 2026-05-26; see canonical release page for asset inventory"
  crates_io: "published package is superseded by 0.15.2 for cargo install; no yank recorded"
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        "v0.15.1 frontmatter",
    )
    replace_once(
        path,
        '''## Install-surface caveat
''',
        f'''## Release publication

The [v0.15.1 GitHub Release]({REPO}/releases/tag/v0.15.1) is published and its
binaries remain usable. The restored [v0.15.2 closeout receipt](0.15.2-closeout-audit.md)
records that no yank was performed. That does not repair the separate crates.io
package-content defect described below.

## Install-surface caveat
''',
        "v0.15.1 publication section",
    )
    replace_once(
        path,
        '''## Related

- Previous release: [v0.15.0](v0.15.0.md)
- Packaging hotfix: [v0.15.2](v0.15.2.md)
''',
        f'''## Related

- GitHub Release: [{REPO}/releases/tag/v0.15.1]({REPO}/releases/tag/v0.15.1)
- Previous release: [v0.15.0](v0.15.0.md)
- Packaging hotfix: [v0.15.2](v0.15.2.md)
- Packaging closeout receipt: [0.15.2 closeout audit](0.15.2-closeout-audit.md)
''',
        "v0.15.1 related links",
    )


def update_v0152() -> None:
    path = Path("docs/releases/v0.15.2.md")
    replace_once(
        path,
        '''---
version: "0.15.2"
tag: "v0.15.2"
release_date_utc: "2026-05-26"
previous_tag: "v0.15.1"
compare: "https://github.com/EffortlessMetrics/perl-lsp/compare/v0.15.1...v0.15.2"
notes_status: draft
release_track: public-alpha
release_kind: patch
channels:
  github_release: pending
  crates_io: pending
  vscode_marketplace: n/a
  open_vsx: n/a
  docker: pending
---
''',
        f'''---
version: "0.15.2"
tag: "v0.15.2"
tag_commit: "746edcb78fe0fa8f48d87386fd4f110502588a87"
release_date_utc: "2026-05-26"
previous_tag: "v0.15.1"
compare: "{REPO}/compare/v0.15.1...v0.15.2"
github_release: "{REPO}/releases/tag/v0.15.2"
notes_status: canonical
release_track: public-alpha
release_kind: patch
channels:
  github_release: "published 2026-05-26; see canonical release page and closeout receipt"
  crates_io: "0.15.2 packages visible; cargo install perllsp/perl-dap smoke passed"
  vscode_marketplace: "0.15.2 verified"
  open_vsx: "0.15.2 verified"
  docker: "Docker Hub verified; GHCR workflow succeeded but public manifest verification returned denied"
---
''',
        "v0.15.2 frontmatter",
    )
    replace_once(
        path,
        '''## Claim boundary
''',
        f'''## Release verification

- The [v0.15.2 GitHub Release]({REPO}/releases/tag/v0.15.2) is published with
  platform binaries, checksums, SBOM, and VSIX artifact.
- crates.io reports `perllsp`, `perl-dap`, and `perl-lsp-rs-core` at `0.15.2`;
  fresh `cargo install` smokes for `perllsp` and `perl-dap` passed, and the
  installed `perllsp` binary passed the inline-completion stdio smoke.
- VS Code Marketplace and Open VSX report extension version `0.15.2`; both
  published-extension smokes passed after the GitHub Release artifact was
  available.
- Docker publish completed successfully. Docker Hub manifests were verified.
  GHCR public manifest verification returned `denied` with the available token,
  so that boundary remains explicit in the closeout receipt.

## Claim boundary
''',
        "v0.15.2 release verification",
    )
    replace_once(
        path,
        '''## Related

- Previous release: [v0.15.1](v0.15.1.md)
- Hotfix commit: `746edcb78`
''',
        f'''## Related

- GitHub Release: [{REPO}/releases/tag/v0.15.2]({REPO}/releases/tag/v0.15.2)
- Previous release: [v0.15.1](v0.15.1.md)
- Hotfix commit: `746edcb78fe0fa8f48d87386fd4f110502588a87`
- Closeout receipt: [0.15.2 closeout audit](0.15.2-closeout-audit.md)
''',
        "v0.15.2 related links",
    )


def update_v0160() -> None:
    path = Path("docs/releases/v0.16.0.md")
    replace_once(
        path,
        '''---
version: "0.16.0"
tag: "v0.16.0"
tag_commit: "b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d"
source_compare_classification: incomplete
release_date_utc: "pending"
previous_tag: "v0.15.2"
compare: "https://github.com/EffortlessMetrics/perl-lsp/compare/v0.15.2...v0.16.0"
notes_status: draft
release_track: public-alpha
release_kind: minor
channels:
  github_release: pending
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        f'''---
version: "0.16.0"
tag: "v0.16.0"
tag_commit: "b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d"
source_compare_classification: incomplete
release_date_utc: "2026-06-06"
previous_tag: "v0.15.2"
compare: "{REPO}/compare/v0.15.2...v0.16.0"
github_release: "{REPO}/releases/tag/v0.16.0"
notes_status: draft
release_track: public-alpha
release_kind: minor
channels:
  github_release: "published 2026-06-06; see canonical release page for asset inventory"
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        "v0.16.0 frontmatter",
    )
    replace_once(
        path,
        '''## Related

- Previous release: [v0.15.2](v0.15.2.md)
''',
        f'''## Related

- GitHub Release: [{REPO}/releases/tag/v0.16.0]({REPO}/releases/tag/v0.16.0)
- Previous release: [v0.15.2](v0.15.2.md)
''',
        "v0.16.0 related links",
    )


def update_v0170() -> None:
    path = Path("docs/releases/v0.17.0.md")
    replace_once(
        path,
        '''---
version: "0.17.0"
tag: "v0.17.0"
tag_commit: "ffee2824938f415e54923112c7b79e3f22040699"
source_compare_classification: inflated
release_date_utc: "pending"
previous_tag: "v0.16.0"
compare: "https://github.com/EffortlessMetrics/perl-lsp/compare/v0.16.0...v0.17.0"
notes_status: pending
release_track: public-alpha
release_kind: minor
channels:
  github_release: pending
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        f'''---
version: "0.17.0"
tag: "v0.17.0"
tag_commit: "ffee2824938f415e54923112c7b79e3f22040699"
source_compare_classification: inflated
release_date_utc: "2026-06-28"
previous_tag: "v0.16.0"
compare: "{REPO}/compare/v0.16.0...v0.17.0"
github_release: "{REPO}/releases/tag/v0.17.0"
notes_status: pending
release_track: public-alpha
release_kind: minor
channels:
  github_release: "published 2026-06-28; see canonical release page for asset inventory"
  crates_io: pending
  vscode_marketplace: pending
  open_vsx: pending
  docker: pending
---
''',
        "v0.17.0 frontmatter",
    )
    replace_once(
        path,
        '''## Related

- Previous release: [v0.16.0](v0.16.0.md)
''',
        f'''## Related

- GitHub Release: [{REPO}/releases/tag/v0.17.0]({REPO}/releases/tag/v0.17.0)
- Previous release: [v0.16.0](v0.16.0.md)
''',
        "v0.17.0 related links",
    )


def main() -> None:
    update_v0150()
    update_v0151()
    update_v0152()
    update_v0160()
    update_v0170()


if __name__ == "__main__":
    main()
