#!/usr/bin/env python3
"""Validate audited container actuals against release-note frontmatter.

The manifest is network-free evidence captured from Docker Hub and authenticated
GHCR probes. This check prevents later source syncs from turning verified or
known-defective container states back into ``pending``.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any

from check_release_channel_actuals import ChannelActualsError, parse_frontmatter


VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
TIMESTAMP_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
DOCKER_PLATFORMS = {"linux/amd64", "linux/arm64"}
GHCR_PLATFORMS = {"linux/arm64"}


class ContainerActualsError(RuntimeError):
    """Raised when the container actuals manifest cannot be loaded."""


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ContainerActualsError(f"cannot read {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise ContainerActualsError(f"{path} did not contain a JSON object")
    return data


def _validate_platforms(
    value: Any, expected: set[str], prefix: str, errors: list[str]
) -> None:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        errors.append(f"{prefix}.platforms must be an array of strings")
        return
    actual = set(value)
    if len(value) != len(actual):
        errors.append(f"{prefix}.platforms must not contain duplicates")
    if actual != expected:
        errors.append(
            f"{prefix}.platforms must equal {sorted(expected)}, found {sorted(actual)}"
        )


def _validate_docker_flavor(
    raw: Any,
    *,
    version: str,
    flavor: str,
    prefix: str,
    errors: list[str],
) -> None:
    if not isinstance(raw, dict):
        errors.append(f"{prefix} must be an object")
        return
    expected_tag = version if flavor == "builder" else f"{version}-perl"
    if raw.get("tag") != expected_tag:
        errors.append(f"{prefix}.tag must equal {expected_tag}")
    pushed_at = raw.get("pushed_at")
    if not isinstance(pushed_at, str) or TIMESTAMP_RE.fullmatch(pushed_at) is None:
        errors.append(f"{prefix}.pushed_at must be an ISO UTC timestamp")
    digest = raw.get("digest")
    if not isinstance(digest, str) or DIGEST_RE.fullmatch(digest) is None:
        errors.append(f"{prefix}.digest must be a sha256 digest")
    _validate_platforms(raw.get("platforms"), DOCKER_PLATFORMS, prefix, errors)


def _validate_ghcr_flavor(
    raw: Any,
    *,
    flavor: str,
    prefix: str,
    errors: list[str],
) -> None:
    if not isinstance(raw, dict):
        errors.append(f"{prefix} must be an object")
        return
    expected_package = "perl-lsp" if flavor == "builder" else "perl-lsp-perl"
    if raw.get("package") != expected_package:
        errors.append(f"{prefix}.package must equal {expected_package}")
    created_at = raw.get("created_at")
    if not isinstance(created_at, str) or TIMESTAMP_RE.fullmatch(created_at) is None:
        errors.append(f"{prefix}.created_at must be an ISO UTC timestamp")
    _validate_platforms(raw.get("platforms"), GHCR_PLATFORMS, prefix, errors)


def validate_manifest(data: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if data.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    repository = data.get("repository")
    if not isinstance(repository, str) or not repository:
        errors.append("repository must be a non-empty string")
    audited_at = data.get("audited_at")
    if not isinstance(audited_at, str) or DATE_RE.fullmatch(audited_at) is None:
        errors.append("audited_at must be an ISO date")
    evidence_runs = data.get("evidence_runs")
    if (
        not isinstance(evidence_runs, list)
        or not evidence_runs
        or not all(isinstance(run, int) and run > 0 for run in evidence_runs)
    ):
        errors.append("evidence_runs must be a non-empty array of positive integers")

    records = data.get("releases")
    if not isinstance(records, list) or not records:
        errors.append("releases must be a non-empty array")
        return errors

    versions: set[str] = set()
    for index, raw in enumerate(records):
        prefix = f"releases[{index}]"
        if not isinstance(raw, dict):
            errors.append(f"{prefix} must be an object")
            continue
        version = raw.get("version")
        if not isinstance(version, str) or VERSION_RE.fullmatch(version) is None:
            errors.append(f"{prefix}.version must be unprefixed SemVer")
            continue
        if version in versions:
            errors.append(f"duplicate release version: {version}")
        versions.add(version)

        note_value = raw.get("note_channel_value")
        if (
            not isinstance(note_value, str)
            or not note_value.strip()
            or note_value.strip().lower() in {"pending", "n/a", "none"}
        ):
            errors.append(f"{prefix}.note_channel_value must be a resolved string")

        docker_hub = raw.get("docker_hub")
        if not isinstance(docker_hub, dict):
            errors.append(f"{prefix}.docker_hub must be an object")
        else:
            for flavor in ("builder", "runtime"):
                _validate_docker_flavor(
                    docker_hub.get(flavor),
                    version=version,
                    flavor=flavor,
                    prefix=f"{prefix}.docker_hub.{flavor}",
                    errors=errors,
                )

        ghcr = raw.get("ghcr")
        if not isinstance(ghcr, dict):
            errors.append(f"{prefix}.ghcr must be an object")
        else:
            for flavor in ("builder", "runtime"):
                _validate_ghcr_flavor(
                    ghcr.get(flavor),
                    flavor=flavor,
                    prefix=f"{prefix}.ghcr.{flavor}",
                    errors=errors,
                )

    return errors


def validate_notes(data: dict[str, Any], repo_root: Path) -> list[str]:
    errors: list[str] = []
    records = data.get("releases", [])
    if not isinstance(records, list):
        return ["cannot validate notes: releases is not an array"]

    for raw in records:
        if not isinstance(raw, dict):
            continue
        version = raw.get("version")
        expected = raw.get("note_channel_value")
        if not isinstance(version, str) or not isinstance(expected, str):
            continue
        path = repo_root / "docs" / "releases" / f"v{version}.md"
        try:
            top, channels = parse_frontmatter(path)
        except ChannelActualsError as exc:
            errors.append(str(exc))
            continue
        if top.get("version") != version:
            errors.append(
                f"v{version}.md version mismatch: found {top.get('version')!r}"
            )
        actual = channels.get("docker", "")
        if actual != expected:
            errors.append(
                f"v{version}.md docker channel mismatch: expected {expected!r}, "
                f"found {actual or 'missing'!r}"
            )
    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("policy/release-container-actuals.json"),
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
    except ContainerActualsError as exc:
        print(f"release-container actuals: ERROR: {exc}", file=sys.stderr)
        return 1

    errors = validate_manifest(data)
    if not errors:
        errors.extend(validate_notes(data, args.repo_root))
    if errors:
        print("release-container actuals validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "release-container actuals OK: "
        f"{len(data.get('releases', []))} audited releases"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
