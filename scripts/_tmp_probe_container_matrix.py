#!/usr/bin/env python3
"""Read-only historical container registry probe. Temporary; do not merge."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import urllib.error
import urllib.request
from typing import Any

VERSIONS = ("0.14.0", "0.15.0", "0.15.1", "0.15.2", "0.16.0", "0.17.0")
DOCKER_HUB_REPOSITORY = "effortlessmetrics/perl-lsp"
GHCR_PACKAGES = ("perl-lsp", "perl-lsp-perl")


def emit(**record: Any) -> None:
    print(json.dumps(record, sort_keys=True, separators=(",", ":")), flush=True)


def actual_platforms(items: list[dict[str, Any]]) -> list[str]:
    platforms: set[str] = set()
    for item in items:
        platform = item.get("platform") if "platform" in item else item
        if not isinstance(platform, dict):
            continue
        os_name = str(platform.get("os", "unknown"))
        arch = str(platform.get("architecture", "unknown"))
        if os_name != "unknown" and arch != "unknown":
            platforms.add(f"{os_name}/{arch}")
    return sorted(platforms)


def fetch_json(url: str, *, token: str | None = None) -> Any:
    headers = {"User-Agent": "perl-lsp-release-audit"}
    if token:
        headers.update(
            {
                "Authorization": f"Bearer {token}",
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
            }
        )
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def probe_docker_hub() -> None:
    for version in VERSIONS:
        for flavor, tag in (("builder", version), ("runtime", f"{version}-perl")):
            url = (
                "https://hub.docker.com/v2/repositories/"
                f"{DOCKER_HUB_REPOSITORY}/tags/{tag}"
            )
            try:
                data = fetch_json(url)
            except urllib.error.HTTPError as exc:
                emit(
                    registry="dockerhub",
                    flavor=flavor,
                    version=version,
                    tag=tag,
                    status="absent-or-inaccessible",
                    http_status=exc.code,
                )
                continue
            except Exception as exc:  # noqa: BLE001 - audit should record failures
                emit(
                    registry="dockerhub",
                    flavor=flavor,
                    version=version,
                    tag=tag,
                    status="probe-error",
                    error=f"{type(exc).__name__}: {exc}",
                )
                continue

            images = data.get("images") if isinstance(data, dict) else []
            emit(
                registry="dockerhub",
                flavor=flavor,
                version=version,
                tag=tag,
                status="present",
                pushed_at=data.get("tag_last_pushed"),
                updated_at=data.get("last_updated"),
                digest=data.get("digest"),
                platforms=actual_platforms(images if isinstance(images, list) else []),
            )


def package_versions(package: str, token: str) -> dict[str, dict[str, Any]]:
    url = (
        "https://api.github.com/orgs/EffortlessMetrics/packages/container/"
        f"{package}/versions?per_page=100"
    )
    records = fetch_json(url, token=token)
    by_tag: dict[str, dict[str, Any]] = {}
    if not isinstance(records, list):
        return by_tag
    for record in records:
        if not isinstance(record, dict):
            continue
        metadata = record.get("metadata") or {}
        container = metadata.get("container") or {}
        tags = container.get("tags") or []
        if isinstance(tags, list):
            for tag in tags:
                if isinstance(tag, str):
                    by_tag[tag] = record
    return by_tag


def inspect_manifest(image: str) -> tuple[str, list[str], str | None]:
    process = subprocess.run(
        ["docker", "manifest", "inspect", image],
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if process.returncode != 0:
        error = process.stderr.strip().splitlines()
        return "absent-or-inaccessible", [], error[0] if error else None
    try:
        data = json.loads(process.stdout)
    except json.JSONDecodeError as exc:
        return "invalid-manifest", [], str(exc)
    manifests = data.get("manifests") if isinstance(data, dict) else []
    if not isinstance(manifests, list):
        manifests = []
    return "present", actual_platforms(manifests), None


def probe_ghcr() -> None:
    token = os.environ.get("GH_TOKEN")
    if not token:
        raise SystemExit("GH_TOKEN is required")

    for package in GHCR_PACKAGES:
        try:
            versions = package_versions(package, token)
        except Exception as exc:  # noqa: BLE001 - audit should record failures
            for version in VERSIONS:
                emit(
                    registry="ghcr",
                    package=package,
                    version=version,
                    status="package-api-error",
                    error=f"{type(exc).__name__}: {exc}",
                )
            continue

        for version in VERSIONS:
            record = versions.get(version)
            image = f"ghcr.io/effortlessmetrics/{package}:{version}"
            manifest_status, platforms, manifest_error = inspect_manifest(image)
            emit(
                registry="ghcr",
                package=package,
                version=version,
                status="present" if record is not None else "tag-not-found",
                version_id=record.get("id") if record else None,
                created_at=record.get("created_at") if record else None,
                updated_at=record.get("updated_at") if record else None,
                manifest_status=manifest_status,
                platforms=platforms,
                manifest_error=manifest_error,
            )


def main() -> int:
    probe_docker_hub()
    probe_ghcr()
    return 0


if __name__ == "__main__":
    sys.exit(main())
