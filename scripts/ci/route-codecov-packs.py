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
NON_LCOV_SKIP_REASON = "non-LCOV CI policy/routing surface; covered by focused CI gates"
NON_SOURCE_LCOV_SKIP_REASON = "LCOV coverage pack matched only non-source files; covered by focused CI gates"


def crate_name_from_source_path(path: str) -> str | None:
    """Extract the crate directory name from a `crates/<name>/src/...` path."""
    if not path.startswith("crates/"):
        return None
    rest = path[len("crates/"):]
    slash = rest.find("/")
    if slash == -1:
        return None
    return rest[:slash]


def changed_crates(paths: list[str]) -> list[str]:
    """Return unique crate names owning changed LCOV source files, in order."""
    seen: set[str] = set()
    result: list[str] = []
    for path in paths:
        if is_lcov_source_path(path):
            name = crate_name_from_source_path(path)
            if name and name not in seen:
                seen.add(name)
                result.append(name)
    return result


def augment_rust_focused_commands(base_commands: list[str], paths: list[str]) -> list[str]:
    """Append per-crate integration-test commands to the rust-focused pack.

    The static pack command only runs ``--lib`` tests.  DAP-style crates
    (e.g. ``perl-dap``) prove patch coverage exclusively through integration
    tests in ``tests/``.  Without the extra ``--tests`` invocations those
    lines show 0 % patch coverage even though the tests exist and pass.

    ``-- --test-threads=1`` forces serial execution within the test binary.
    Integration tests in this workspace mutate global/process state (env vars,
    auto-ID counters, plenv PATH) without ``#[serial]`` guards.  Coverage does
    not benefit from parallelism -- deterministic instrumentation is more
    important.

    IMPORTANT: these commands are executed NON-FATALLY by
    ``generate-coverage-pack-commands.py`` (invoked from the
    ``coverage-proof-routed`` justfile recipe).  Assertion failures in
    integration tests do NOT abort the coverage lane -- the instrumented binary
    still writes LLVM coverage data before exiting, so ``cargo-llvm-cov``
    collects coverage regardless.  The quality-gate verdict is the patch
    coverage NUMBER, not test pass/fail.  Pre-existing test-debt (tracked in
    #1269) can no longer block PRs by surfacing in this lane.
    """
    commands = list(base_commands)
    for crate_name in changed_crates(paths):
        cmd = f"cargo test -p {crate_name} --tests --profile agent --locked -- --test-threads=1"
        if cmd not in commands:
            commands.append(cmd)
    return commands


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


def is_lcov_pack(pack: dict[str, object]) -> bool:
    return pack.get("lcov") is not False


def non_lcov_matches(packs: list[dict[str, object]], paths: list[str]) -> list[dict[str, object]]:
    return [
        pack
        for pack in packs
        if pack.get("id") != FALLBACK_PACK_ID
        and not is_lcov_pack(pack)
        and pack_matches(pack, paths)
    ]


def lcov_matches_without_source(
    packs: list[dict[str, object]], paths: list[str]
) -> list[dict[str, object]]:
    return [
        pack
        for pack in packs
        if pack.get("id") != FALLBACK_PACK_ID
        and is_lcov_pack(pack)
        and pack_matches(pack, paths)
        and not pack_matches_lcov_source(pack, paths)
    ]


def selected_packs(packs: list[dict[str, object]], paths: list[str]) -> list[dict[str, object]]:
    fallback = next((pack for pack in packs if pack.get("id") == FALLBACK_PACK_ID), None)
    selected = [
        pack
        for pack in packs
        if pack.get("id") != FALLBACK_PACK_ID
        and is_lcov_pack(pack)
        and pack_matches(pack, paths)
        and pack_matches_lcov_source(pack, paths)
    ]
    selected_needs_fallback = fallback is not None and any(
        is_lcov_source_path(path) and not any(pack_matches(pack, [path]) for pack in selected)
        for path in paths
    )
    if selected_needs_fallback:
        selected.append(fallback)
    if selected:
        return selected
    if non_lcov_matches(packs, paths):
        return []
    return []


def normalize_pack(
    pack: dict[str, object], paths: list[str] | None = None
) -> dict[str, object]:
    commands: list[str] = list(pack.get("commands") or [])
    if pack.get("id") == FALLBACK_PACK_ID and paths is not None:
        commands = augment_rust_focused_commands(commands, paths)
    return {
        "id": str(pack.get("id", "")),
        "files": list(pack.get("files") or []),
        "commands": commands,
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
            skipped = receipt.get("skipped_by_policy") or {}
            if skipped:
                handle.write("- skipped proof packs:\n")
                for pack_id, reason in skipped.items():
                    handle.write(f"  - `{pack_id}`: {reason}\n")


def main() -> int:
    args = parse_args()
    manifest = tomllib.loads(Path(args.manifest).read_text(encoding="utf-8"))
    packs = [pack for pack in manifest.get("pack", []) if isinstance(pack, dict)]
    paths = changed_files(args.base, args.head)
    coverage_packs = [normalize_pack(pack, paths) for pack in selected_packs(packs, paths)]
    coverage_pack_ids = [pack["id"] for pack in coverage_packs]
    skipped_by_policy = {
        str(pack.get("id", "")): NON_LCOV_SKIP_REASON for pack in non_lcov_matches(packs, paths)
    }
    skipped_by_policy.update(
        {
            str(pack.get("id", "")): NON_SOURCE_LCOV_SKIP_REASON
            for pack in lcov_matches_without_source(packs, paths)
        }
    )
    receipt = {
        "schema_version": "ci_route.v1",
        "provider_action": "changed_file_proof_routing",
        "claim_boundary": (
            "CI-enforced lightweight Codecov coverage-pack route; selected packs "
            "feed Codecov / Patch 95"
        ),
        "base": args.base,
        "head": args.head,
        "changed_files": paths,
        "changed_surfaces": coverage_pack_ids,
        "required_proof_packs": [],
        "skipped_by_policy": skipped_by_policy,
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
