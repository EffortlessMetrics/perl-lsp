#!/usr/bin/env python3
"""Read-only GHCR runtime-image release probe. Temporary; do not merge."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import urllib.request
from typing import Any

VERSIONS = ("0.14.0", "0.15.0", "0.15.1", "0.15.2", "0.16.0", "0.17.0")
PACKAGE = "perl-lsp-perl"


def emit(**record: Any) -> None:
    print(json.dumps(record, sort_keys=True, separators=(",", ":")), flush=True)


def fetch_versions(token: str) -> dict[str, dict[str, Any]]:
    url = (
        "https://api.github.com/orgs/EffortlessMetrics/packages/container/"
        f"{PACKAGE}/versions?per_page=100"
    )
    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "perl-lsp-release-audit",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        records = json.load(response)

    by_tag: dict[str, dict[str, Any]] = {}
    for record in records:
        tags = ((record.get("metadata") or {}).get("container") or {}).get("tags") or []
        for tag in tags:
            by_tag[tag] = record
    return by_tag


def inspect(version: str) -> tuple[str, list[str], str | None]:
    image = f"ghcr.io/effortlessmetrics/{PACKAGE}:{version}"
    process = subprocess.run(
        ["docker", "manifest", "inspect", image],
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if process.returncode != 0:
        lines = process.stderr.strip().splitlines()
        return "absent-or-inaccessible", [], lines[0] if lines else None
    data = json.loads(process.stdout)
    platforms = sorted(
        {
            f"{platform.get('os')}/{platform.get('architecture')}"
            for item in data.get("manifests") or []
            for platform in [item.get("platform") or {}]
            if platform.get("os") not in (None, "unknown")
            and platform.get("architecture") not in (None, "unknown")
        }
    )
    return "present", platforms, None


def main() -> int:
    token = os.environ.get("GH_TOKEN")
    if not token:
        raise SystemExit("GH_TOKEN is required")
    versions = fetch_versions(token)
    for version in VERSIONS:
        record = versions.get(version)
        manifest_status, platforms, error = inspect(version)
        emit(
            registry="ghcr",
            package=PACKAGE,
            version=version,
            status="present" if record else "tag-not-found",
            version_id=record.get("id") if record else None,
            created_at=record.get("created_at") if record else None,
            updated_at=record.get("updated_at") if record else None,
            manifest_status=manifest_status,
            platforms=platforms,
            manifest_error=error,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
