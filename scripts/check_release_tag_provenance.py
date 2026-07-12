#!/usr/bin/env python3
"""Validate the release-tag provenance manifest.

The default mode is network-free and validates the manifest's shape and internal
contracts. `--verify-git` additionally resolves local tag refs and checks the
pinned commit and predecessor relationship. CI callers using that mode must
fetch tags and full history first.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ModuleNotFoundError:
        tomllib = None  # type: ignore[assignment]

if tomllib is None:
    TOMLDecodeError = ValueError
else:
    TOMLDecodeError = tomllib.TOMLDecodeError

TAG_RE = re.compile(r"^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
RECORDED_SHA_RE = re.compile(r"^[0-9a-f]{7,40}$")
RECORD_STATUSES = {"match", "stale", "unrecorded", "pending"}
LINEAGE_STATUSES = {"root", "linear", "diverged"}


class ManifestError(RuntimeError):
    """Raised when a provenance manifest cannot be loaded."""


def load_manifest(path: Path) -> dict[str, Any]:
    if tomllib is None:
        raise ManifestError(
            "Python 3.11+ or the 'tomli' package is required to parse TOML"
        )

    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except (OSError, TOMLDecodeError) as exc:
        raise ManifestError(f"cannot read {path}: {exc}") from exc

    if not isinstance(data, dict):
        raise ManifestError(f"{path} did not contain a TOML table")
    return data


def validate_manifest(data: dict[str, Any]) -> list[str]:
    errors: list[str] = []

    if data.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if not isinstance(data.get("repository"), str) or not data["repository"]:
        errors.append("repository must be a non-empty string")
    if not isinstance(data.get("audited_at"), str) or not data["audited_at"]:
        errors.append("audited_at must be a non-empty ISO date string")

    tags = data.get("tag")
    if not isinstance(tags, list) or not tags:
        errors.append("manifest must contain at least one [[tag]] record")
        return errors

    names: list[str] = []
    name_set: set[str] = set()

    for index, raw in enumerate(tags):
        prefix = f"tag[{index}]"
        if not isinstance(raw, dict):
            errors.append(f"{prefix} must be a table")
            continue

        name = raw.get("name")
        if not isinstance(name, str) or TAG_RE.fullmatch(name) is None:
            errors.append(f"{prefix}.name must be a v-prefixed SemVer tag")
            continue
        if name in name_set:
            errors.append(f"duplicate tag record: {name}")
        names.append(name)
        name_set.add(name)

        current_sha = raw.get("current_sha")
        if not isinstance(current_sha, str) or SHA_RE.fullmatch(current_sha) is None:
            errors.append(f"{name}.current_sha must be a lowercase 40-hex commit SHA")
            current_sha = ""

        record_status = raw.get("record_status")
        if record_status not in RECORD_STATUSES:
            errors.append(
                f"{name}.record_status must be one of {sorted(RECORD_STATUSES)}"
            )

        recorded_sha = raw.get("recorded_sha")
        recorded_reachable = raw.get("recorded_reachable")
        if record_status in {"match", "stale"}:
            if (
                not isinstance(recorded_sha, str)
                or RECORDED_SHA_RE.fullmatch(recorded_sha) is None
            ):
                errors.append(
                    f"{name}.recorded_sha must be 7-40 lowercase hex for {record_status}"
                )
            elif current_sha:
                is_prefix = current_sha.startswith(recorded_sha)
                if record_status == "match" and not is_prefix:
                    errors.append(
                        f"{name} is marked match but recorded_sha is not a current_sha prefix"
                    )
                if record_status == "stale" and is_prefix:
                    errors.append(
                        f"{name} is marked stale but recorded_sha matches current_sha"
                    )
            if not isinstance(recorded_reachable, bool):
                errors.append(
                    f"{name}.recorded_reachable must be boolean for {record_status}"
                )
            elif record_status == "match" and not recorded_reachable:
                errors.append(f"{name} match records must remain reachable")
        else:
            if recorded_sha is not None:
                errors.append(
                    f"{name}.recorded_sha must be omitted for {record_status} records"
                )
            if recorded_reachable is not None:
                errors.append(
                    f"{name}.recorded_reachable must be omitted for {record_status} records"
                )

        lineage = raw.get("lineage")
        if lineage not in LINEAGE_STATUSES:
            errors.append(f"{name}.lineage must be one of {sorted(LINEAGE_STATUSES)}")
        predecessor = raw.get("predecessor")
        if lineage == "root":
            if predecessor is not None:
                errors.append(f"{name} root records must not define predecessor")
        elif lineage in {"linear", "diverged"}:
            if not isinstance(predecessor, str) or TAG_RE.fullmatch(predecessor) is None:
                errors.append(
                    f"{name} {lineage} records require a v-prefixed predecessor"
                )
            elif predecessor == name:
                errors.append(f"{name} cannot be its own predecessor")

    positions = {name: index for index, name in enumerate(names)}
    for raw in tags:
        if not isinstance(raw, dict):
            continue
        name = raw.get("name")
        predecessor = raw.get("predecessor")
        if not isinstance(name, str) or not isinstance(predecessor, str):
            continue
        if predecessor not in name_set:
            errors.append(f"{name} references unknown predecessor {predecessor}")
        elif positions[predecessor] >= positions[name]:
            errors.append(f"{name} predecessor {predecessor} must appear earlier")

    missing = data.get("missing_tag", [])
    if not isinstance(missing, list):
        errors.append("missing_tag must be an array of tables")
    else:
        missing_versions: set[str] = set()
        for index, raw in enumerate(missing):
            prefix = f"missing_tag[{index}]"
            if not isinstance(raw, dict):
                errors.append(f"{prefix} must be a table")
                continue
            version = raw.get("version")
            if not isinstance(version, str) or VERSION_RE.fullmatch(version) is None:
                errors.append(f"{prefix}.version must be unprefixed SemVer")
                continue
            if version in missing_versions:
                errors.append(f"duplicate missing-tag record: {version}")
            missing_versions.add(version)
            if f"v{version}" in name_set:
                errors.append(f"missing-tag record {version} also has a live tag record")
            if not isinstance(raw.get("status"), str) or not raw["status"]:
                errors.append(f"{prefix}.status must be a non-empty string")

    return errors


def _git(repo_root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo_root), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def verify_git_refs(data: dict[str, Any], repo_root: Path) -> list[str]:
    """Compare the manifest against refs in a local full-history checkout."""

    if shutil.which("git") is None:
        return ["git executable not found on PATH"]

    errors: list[str] = []
    tags = data.get("tag", [])
    if not isinstance(tags, list):
        return ["cannot verify git refs: tag records are not a list"]

    manifest_names = {
        raw["name"]
        for raw in tags
        if isinstance(raw, dict) and isinstance(raw.get("name"), str)
    }
    list_result = _git(repo_root, "tag", "--list", "v*")
    if list_result.returncode != 0:
        errors.append(f"cannot list local tags: {list_result.stderr.strip()}")
    else:
        local_release_tags = {
            line.strip()
            for line in list_result.stdout.splitlines()
            if TAG_RE.fullmatch(line.strip()) is not None
        }
        for extra_tag in sorted(local_release_tags - manifest_names):
            errors.append(f"local release tag is missing from manifest: {extra_tag}")

    resolved: dict[str, str] = {}
    for raw in tags:
        if not isinstance(raw, dict):
            continue
        name = raw.get("name")
        expected = raw.get("current_sha")
        if not isinstance(name, str) or not isinstance(expected, str):
            continue

        result = _git(repo_root, "rev-parse", "--verify", f"{name}^{{commit}}")
        if result.returncode != 0:
            errors.append(f"{name} cannot be resolved locally: {result.stderr.strip()}")
            continue
        actual = result.stdout.strip().lower()
        resolved[name] = actual
        if actual != expected:
            errors.append(f"{name} drifted: manifest={expected} local={actual}")

    for raw in tags:
        if not isinstance(raw, dict):
            continue
        name = raw.get("name")
        predecessor = raw.get("predecessor")
        lineage = raw.get("lineage")
        if (
            not isinstance(name, str)
            or not isinstance(predecessor, str)
            or name not in resolved
            or predecessor not in resolved
        ):
            continue

        forward = _git(repo_root, "merge-base", "--is-ancestor", predecessor, name)
        if forward.returncode not in (0, 1):
            errors.append(
                f"git merge-base failed for {predecessor} and {name}: "
                f"{forward.stderr.strip()}"
            )
            continue
        reverse = _git(repo_root, "merge-base", "--is-ancestor", name, predecessor)
        if reverse.returncode not in (0, 1):
            errors.append(
                f"git merge-base failed for {name} and {predecessor}: "
                f"{reverse.stderr.strip()}"
            )
            continue

        predecessor_is_ancestor = forward.returncode == 0
        tag_is_ancestor = reverse.returncode == 0

        if lineage == "linear" and not predecessor_is_ancestor:
            errors.append(f"{name} is not linear from predecessor {predecessor}")
        elif lineage == "diverged" and (
            predecessor_is_ancestor or tag_is_ancestor
        ):
            errors.append(
                f"{name} is marked diverged from {predecessor}, but one ref is ancestral"
            )

    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("policy/release-tag-provenance.toml"),
    )
    parser.add_argument(
        "--verify-git",
        action="store_true",
        help="resolve local tags and verify pinned SHA and ancestry",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path("."),
        help="local git checkout used by --verify-git",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        data = load_manifest(args.manifest)
    except ManifestError as exc:
        print(f"release-tag provenance: ERROR: {exc}", file=sys.stderr)
        return 1

    errors = validate_manifest(data)
    if args.verify_git and not errors:
        errors.extend(verify_git_refs(data, args.repo_root))

    if errors:
        print("release-tag provenance validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    tag_count = len(data.get("tag", []))
    mode = "manifest + local git" if args.verify_git else "manifest"
    print(f"release-tag provenance OK: {tag_count} tags ({mode})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
