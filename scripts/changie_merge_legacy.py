#!/usr/bin/env python3
"""Insert one Changie version ahead of the immutable legacy changelog tail.

The repository adopted Changie after a curated historical backfill. Re-rendering
all older releases from a new structured archive would create unnecessary drift,
so Changie owns new fragments and version files while this small adapter keeps
pre-adoption history byte-for-byte intact from its first release heading onward.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re

UNRELEASED_HEADING = b"## [Unreleased]"
SEMVER = re.compile(r"^v?(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")


class ChangelogMergeError(RuntimeError):
    """Raised when the version file or changelog violates the merge contract."""


def normalize_version(raw: str) -> tuple[str, str]:
    """Return `(version_without_prefix, version_with_prefix)`."""

    match = SEMVER.fullmatch(raw.strip())
    if match is None:
        raise ChangelogMergeError(f"invalid release version: {raw!r}")
    version = match.group("version")
    return version, f"v{version}"


def _newline_for_insertion(separator: bytes, changelog: bytes) -> bytes:
    """Choose a newline for new content without rewriting existing bytes."""

    if separator.startswith(b"\r\n"):
        return b"\r\n"
    if separator.startswith(b"\n"):
        return b"\n"
    if separator.startswith(b"\r"):
        return b"\r"
    if b"\r\n" in changelog:
        return b"\r\n"
    if b"\n" in changelog:
        return b"\n"
    if b"\r" in changelog:
        return b"\r"
    return b"\n"


def _with_newline_style(content: bytes, newline: bytes) -> bytes:
    """Normalize only newly inserted content to the changelog's local style."""

    normalized = content.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    return normalized.replace(b"\n", newline)


def merge_version(
    *,
    changelog_path: Path,
    changes_dir: Path,
    raw_version: str,
) -> None:
    """Insert the batched version immediately after `[Unreleased]`.

    Everything from the first non-newline byte after `[Unreleased]` onward is
    treated as the legacy tail and copied byte-for-byte. The function fails
    closed on duplicate headings, malformed version files, or ambiguous markers.
    """

    version, prefixed = normalize_version(raw_version)
    version_path = changes_dir / f"{prefixed}.md"
    if not version_path.is_file():
        raise ChangelogMergeError(f"missing Changie version file: {version_path}")

    release_text = version_path.read_bytes().strip(b"\r\n")
    expected_heading = f"## [{version}] - ".encode("ascii")
    if not release_text.startswith(expected_heading):
        raise ChangelogMergeError(
            f"{version_path} must begin with {expected_heading.decode('ascii')!r}"
        )

    changelog = changelog_path.read_bytes()
    marker_count = changelog.count(UNRELEASED_HEADING)
    if marker_count != 1:
        heading = UNRELEASED_HEADING.decode("ascii")
        raise ChangelogMergeError(
            f"expected one {heading!r} heading, found {marker_count}"
        )

    release_heading = re.compile(
        rb"(?m)^## \["
        + re.escape(version.encode("ascii"))
        + rb"\](?: -|\r?$)"
    )
    if release_heading.search(changelog) is not None:
        raise ChangelogMergeError(f"CHANGELOG already contains release {version}")

    marker_start = changelog.index(UNRELEASED_HEADING)
    marker_end = marker_start + len(UNRELEASED_HEADING)
    prefix = changelog[:marker_end]
    suffix = changelog[marker_end:]

    separator_length = len(suffix) - len(suffix.lstrip(b"\r\n"))
    separator = suffix[:separator_length]
    legacy_tail = suffix[separator_length:]
    newline = _newline_for_insertion(separator, changelog)
    if not separator:
        separator = newline + newline

    release_text = _with_newline_style(release_text, newline)
    rendered = prefix + separator + release_text
    if legacy_tail:
        rendered += separator + legacy_tail
    elif not rendered.endswith(newline):
        rendered += newline

    if legacy_tail and not rendered.endswith(legacy_tail):
        raise ChangelogMergeError("legacy changelog tail changed during render")

    changelog_path.write_bytes(rendered)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Insert a batched Changie release before legacy history"
    )
    parser.add_argument("version", help="Release version, with or without v prefix")
    parser.add_argument(
        "--changelog",
        type=Path,
        default=Path("CHANGELOG.md"),
        help="Rendered changelog path",
    )
    parser.add_argument(
        "--changes-dir",
        type=Path,
        default=Path(".changes"),
        help="Changie changes directory",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        merge_version(
            changelog_path=args.changelog,
            changes_dir=args.changes_dir,
            raw_version=args.version,
        )
    except ChangelogMergeError as exc:
        raise SystemExit(f"changelog merge failed: {exc}") from exc

    print(f"Inserted {args.version} into {args.changelog}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
