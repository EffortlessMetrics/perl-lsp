#!/usr/bin/env python3
"""One-shot guarded correction for the 0.13 release lineage in CHANGELOG.md."""

from pathlib import Path

PATH = Path("CHANGELOG.md")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    return text.replace(old, new, 1)


def main() -> None:
    text = PATH.read_text(encoding="utf-8")

    text = replace_once(
        text,
        """## [0.14.0] - 2026-05-12

Release notes: [v0.14.0](docs/releases/v0.14.0.md)

### Added
""",
        """## [0.14.0] - 2026-05-12

Release notes: [v0.14.0](docs/releases/v0.14.0.md)

> **Release boundary correction.** No `v0.13.4` tag was cut. The previous
> actual tag is `v0.13.3`, so the valid cumulative comparison is
> [`v0.13.3...v0.14.0`](https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.3...v0.14.0).
> That range includes the source state prepared while the workspace carried
> version `0.13.4`; it is not a narrow 0.14-only logical ledger.
>
> <!-- lineage-correction:0.13 -->

### Added
""",
        "0.14.0 boundary",
    )

    text = replace_once(
        text,
        """## [0.13.4] - 2026-05-07

Release notes: [v0.13.4](docs/releases/v0.13.4.md)

### Fixed
""",
        """## [0.13.4] - 2026-05-07

Release notes: [0.13.4 prepared milestone](docs/releases/v0.13.4.md)

> **Prepared milestone, not a tagged release.** The repository has no
> `v0.13.4` ref. This section records changes staged while the workspace carried
> version `0.13.4`; those changes first appear in the later tagged `v0.14.0`
> tree. Do not infer a standalone asset, package, or compare boundary from this
> heading.

### Fixed
""",
        "0.13.4 status",
    )

    text = replace_once(
        text,
        """## [0.13.1] - 2026-05-01

Release notes: [v0.13.1](docs/releases/v0.13.1.md)

### Changed

- Hardened public-alpha release channels after the `v0.13.0` launch.
- Decoupled Open VSX publishing from VS Code Marketplace publishing.
- Clarified release naming: package versions use normal SemVer while product
  posture remains public alpha.
- Improved CI Gate timeout headroom and diagnostics for release runs.
- Corrected Homebrew/tap naming and formula generation around the `perllsp`
  binary.
""",
        """## [0.13.1] - 2026-05-01

Release notes: [v0.13.1](docs/releases/v0.13.1.md)

> **Release boundary correction.** No final `v0.13.0` tag was cut. The release
> line moved directly from `v0.13.0-rc1` to `v0.13.1`; use
> [`v0.13.0-rc1...v0.13.1`](https://github.com/EffortlessMetrics/perl-lsp/compare/v0.13.0-rc1...v0.13.1)
> for source comparison.

### Changed

- Hardened public-alpha release channels after the `v0.13.0-rc1` rehearsal.
- Decoupled Open VSX publishing from VS Code Marketplace publishing.
- Clarified release naming: package versions use normal SemVer while product
  posture remains public alpha.
- Improved CI Gate timeout headroom and diagnostics for release runs.
- Corrected Homebrew/tap naming and formula generation around the `perllsp`
  binary.
""",
        "0.13.1 predecessor",
    )

    text = replace_once(
        text,
        """## [0.13.0-rc1] - 2026-04-30

Release notes: [v0.13.0-rc1](docs/releases/v0.13.0-rc1.md)

### Fixed
""",
        """## [0.13.0-rc1] - 2026-04-30

Release notes: [v0.13.0-rc1](docs/releases/v0.13.0-rc1.md)

> **Historical outcome.** This remained the only `0.13.0` tag. No final
> `v0.13.0` source boundary was created; the next tagged release was
> `v0.13.1` on May 1, 2026.

### Fixed
""",
        "0.13.0-rc1 outcome",
    )

    if "## [0.13.0]" in text:
        raise SystemExit("unexpected final 0.13.0 heading present")

    PATH.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
