#!/usr/bin/env python3
"""Validate verified release-channel actuals against release-note frontmatter.

This check is intentionally network-free. The JSON manifest records facts that
were independently audited; the checker prevents later source syncs from
silently reverting those notes to ``pending``.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
TAG_RE = re.compile(r"^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


class ChannelActualsError(RuntimeError):
    """Raised when a release-channel actuals file cannot be loaded."""


def _unquote(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def parse_frontmatter(path: Path) -> tuple[dict[str, str], dict[str, str]]:
    """Return top-level and ``channels`` values from simple YAML frontmatter.

    Release-note frontmatter uses scalar keys and one nested ``channels`` map.
    Parsing that constrained shape locally avoids adding a YAML dependency to a
    documentation integrity check.
    """

    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise ChannelActualsError(f"cannot read {path}: {exc}") from exc

    if not lines or lines[0] != "---":
        raise ChannelActualsError(f"{path} does not open with YAML frontmatter")

    try:
        end = lines.index("---", 1)
    except ValueError as exc:
        raise ChannelActualsError(f"{path} has no closing frontmatter delimiter") from exc

    top: dict[str, str] = {}
    channels: dict[str, str] = {}
    in_channels = False

    for line_number, raw in enumerate(lines[1:end], start=2):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue

        if raw.rstrip() == "channels:":
            in_channels = True
            continue

        if raw.startswith("  "):
            if not in_channels:
                raise ChannelActualsError(
                    f"{path}:{line_number}: unexpected nested frontmatter value"
                )
            item = raw.strip()
            if ":" not in item:
                raise ChannelActualsError(
                    f"{path}:{line_number}: malformed channel frontmatter"
                )
            key, value = item.split(":", 1)
            channels[key.strip()] = _unquote(value)
            continue

        in_channels = False
        if ":" not in raw:
            raise ChannelActualsError(
                f"{path}:{line_number}: malformed top-level frontmatter"
            )
        key, value = raw.split(":", 1)
        top[key.strip()] = _unquote(value)

    return top, channels


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ChannelActualsError(f"cannot read {path}: {exc}") from exc

    if not isinstance(data, dict):
        raise ChannelActualsError(f"{path} did not contain a JSON object")
    return data


def validate_manifest(data: dict[str, Any]) -> list[str]:
    errors: list[str] = []

    if data.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if not isinstance(data.get("repository"), str) or not data["repository"]:
        errors.append("repository must be a non-empty string")
    audited_at = data.get("audited_at")
    if not isinstance(audited_at, str) or DATE_RE.fullmatch(audited_at) is None:
        errors.append("audited_at must be an ISO date")

    records = data.get("github_releases")
    if not isinstance(records, list) or not records:
        errors.append("github_releases must be a non-empty array")
        return errors

    versions: set[str] = set()
    tags: set[str] = set()
    for index, raw in enumerate(records):
        prefix = f"github_releases[{index}]"
        if not isinstance(raw, dict):
            errors.append(f"{prefix} must be an object")
            continue

        version = raw.get("version")
        tag = raw.get("tag")
        sha = raw.get("tag_commit")
        published = raw.get("published_date_utc")
        release_url = raw.get("release_url")

        if not isinstance(version, str) or VERSION_RE.fullmatch(version) is None:
            errors.append(f"{prefix}.version must be unprefixed SemVer")
        elif version in versions:
            errors.append(f"duplicate release version: {version}")
        else:
            versions.add(version)

        if not isinstance(tag, str) or TAG_RE.fullmatch(tag) is None:
            errors.append(f"{prefix}.tag must be v-prefixed SemVer")
        elif tag in tags:
            errors.append(f"duplicate release tag: {tag}")
        else:
            tags.add(tag)

        if isinstance(version, str) and isinstance(tag, str) and tag != f"v{version}":
            errors.append(f"{prefix}.tag must equal v{version}")
        if not isinstance(sha, str) or SHA_RE.fullmatch(sha) is None:
            errors.append(f"{prefix}.tag_commit must be lowercase 40-hex")
        if not isinstance(published, str) or DATE_RE.fullmatch(published) is None:
            errors.append(f"{prefix}.published_date_utc must be an ISO date")
        repository = data.get("repository")
        if (
            not isinstance(release_url, str)
            or not isinstance(repository, str)
            or not isinstance(tag, str)
            or release_url != f"https://github.com/{repository}/releases/tag/{tag}"
        ):
            errors.append(f"{prefix}.release_url must be the canonical tag release URL")

        receipt = raw.get("closeout_receipt")
        if receipt is not None and (
            not isinstance(receipt, str) or not receipt.startswith("docs/releases/")
        ):
            errors.append(f"{prefix}.closeout_receipt must be a docs/releases path")

    return errors


def validate_notes(data: dict[str, Any], repo_root: Path) -> list[str]:
    errors: list[str] = []
    records = data.get("github_releases", [])
    if not isinstance(records, list):
        return ["cannot validate notes: github_releases is not an array"]

    for raw in records:
        if not isinstance(raw, dict):
            continue
        version = raw.get("version")
        if not isinstance(version, str):
            continue

        path = repo_root / "docs" / "releases" / f"v{version}.md"
        try:
            top, channels = parse_frontmatter(path)
        except ChannelActualsError as exc:
            errors.append(str(exc))
            continue

        expected = {
            "version": version,
            "tag": raw.get("tag"),
            "tag_commit": raw.get("tag_commit"),
            "release_date_utc": raw.get("published_date_utc"),
            "github_release": raw.get("release_url"),
        }
        for key, value in expected.items():
            if top.get(key) != value:
                errors.append(
                    f"v{version}.md {key} mismatch: expected {value!r}, found {top.get(key)!r}"
                )

        github_status = channels.get("github_release", "")
        if not github_status or github_status.lower() in {"pending", "n/a", "none"}:
            errors.append(
                f"v{version}.md regressed verified GitHub Release to {github_status or 'missing'}"
            )

        receipt = raw.get("closeout_receipt")
        if isinstance(receipt, str):
            receipt_path = repo_root / receipt
            if not receipt_path.is_file():
                errors.append(f"v{version} closeout receipt missing: {receipt}")
            if top.get("notes_status") != "canonical":
                errors.append(
                    f"v{version}.md with closeout receipt must have notes_status: canonical"
                )

    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("policy/release-channel-actuals.json"),
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path("."),
        help="repository root containing docs/releases",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        data = load_manifest(args.manifest)
    except ChannelActualsError as exc:
        print(f"release-channel actuals: ERROR: {exc}", file=sys.stderr)
        return 1

    errors = validate_manifest(data)
    if not errors:
        errors.extend(validate_notes(data, args.repo_root))

    if errors:
        print("release-channel actuals validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    count = len(data.get("github_releases", []))
    print(f"release-channel actuals OK: {count} verified GitHub Releases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
