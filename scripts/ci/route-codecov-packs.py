#!/usr/bin/env python3
"""Emit a lightweight coverage-pack route for the Codecov workflow."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - CI uses Python 3.11+.
    print("tomllib is required; use Python 3.11 or newer", file=sys.stderr)
    raise


FALLBACK_PACK_ID = "patch-coverage-rust-focused"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Route changed files to Codecov coverage proof packs."
    )
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--manifest", default=".ci/coverage-packs.toml")
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--summary", required=True)
    return parser.parse_args()


def changed_files(base: str, head: str) -> list[str]:
    output = subprocess.check_output(
        ["git", "diff", "--name-only", f"{base}...{head}"],
        text=True,
    )
    return [line.strip().replace("\\", "/") for line in output.splitlines() if line.strip()]


def matches_pattern(path: str, pattern: str) -> bool:
    pattern = pattern.replace("\\", "/")
    if pattern.startswith("*."):
        return path.endswith(pattern[1:])
    if pattern.endswith("/"):
        return path.startswith(pattern)
    return path == pattern or path.startswith(pattern)


def pack_matches(pack: dict[str, object], paths: list[str]) -> bool:
    patterns = pack.get("files") or []
    if not isinstance(patterns, list):
        return False
    return any(
        isinstance(pattern, str) and matches_pattern(path, pattern)
        for path in paths
        for pattern in patterns
    )


def is_lcov_source_path(path: str) -> bool:
    if not path.endswith(".rs"):
        return False
    if path.startswith("xtask/tests/") or "/tests/" in path:
        return False
    return path.startswith("xtask/src/") or path.startswith("crates/")


def pack_matches_lcov_source(pack: dict[str, object], paths: list[str]) -> bool:
    patterns = pack.get("files") or []
    if not isinstance(patterns, list):
        return False
    return any(
        is_lcov_source_path(path)
        and isinstance(pattern, str)
        and matches_pattern(path, pattern)
        for path in paths
        for pattern in patterns
    )


def selected_packs(packs: list[dict[str, object]], paths: list[str]) -> list[dict[str, object]]:
    fallback = next((pack for pack in packs if pack.get("id") == FALLBACK_PACK_ID), None)
    selected = [
        pack
        for pack in packs
        if pack.get("id") != FALLBACK_PACK_ID
        and pack_matches(pack, paths)
        and pack_matches_lcov_source(pack, paths)
    ]
    if selected:
        return selected
    if fallback is not None and any(is_lcov_source_path(path) for path in paths):
        return [fallback]
    return []


def normalize_pack(pack: dict[str, object]) -> dict[str, object]:
    return {
        "id": str(pack.get("id", "")),
        "files": list(pack.get("files") or []),
        "commands": list(pack.get("commands") or []),
        "coverage_filters": list(pack.get("coverage_filters") or []),
    }


def write_summary(path: Path, receipt: dict[str, object]) -> None:
    packs = receipt["coverage_proof_packs"]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# Changed-File Coverage Route\n\n")
        handle.write(f"- base: `{receipt['base']}`\n")
        handle.write(f"- head: `{receipt['head']}`\n")
        handle.write(f"- changed files: `{len(receipt['changed_files'])}`\n")
        if packs:
            handle.write("- coverage proof packs:\n")
            for pack in packs:
                handle.write(f"  - `{pack['id']}`\n")
        else:
            handle.write("- coverage proof packs: skipped-by-policy\n")


def main() -> int:
    args = parse_args()
    manifest = tomllib.loads(Path(args.manifest).read_text(encoding="utf-8"))
    packs = [pack for pack in manifest.get("pack", []) if isinstance(pack, dict)]
    paths = changed_files(args.base, args.head)
    coverage_packs = [normalize_pack(pack) for pack in selected_packs(packs, paths)]
    coverage_pack_ids = [pack["id"] for pack in coverage_packs]
    receipt = {
        "schema_version": "ci_route.v1",
        "provider_action": "changed_file_proof_routing",
        "claim_boundary": "lightweight Codecov coverage-pack route; full proof routing remains owned by xtask",
        "base": args.base,
        "head": args.head,
        "changed_files": paths,
        "changed_surfaces": coverage_pack_ids,
        "required_proof_packs": [],
        "skipped_by_policy": {},
        "coverage_pack_selector": coverage_pack_ids,
        "coverage_proof_packs": coverage_packs,
        "estimated_lem": 1,
    }
    receipt_path = Path(args.receipt)
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    write_summary(Path(args.summary), receipt)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
