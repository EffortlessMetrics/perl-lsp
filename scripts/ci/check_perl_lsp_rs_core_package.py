#!/usr/bin/env python3
"""Validate that perl-lsp-rs-core is self-contained when packaged."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CRATE = "perl-lsp-rs-core"
REQUIRED_PACKAGE_FILE = "build_catalog.rs"


def run(args: list[str], *, capture: bool = False) -> str:
    print("+ " + " ".join(args), flush=True)
    if capture:
        completed = subprocess.run(
            args,
            cwd=ROOT,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        )
        return completed.stdout
    subprocess.run(args, cwd=ROOT, check=True)
    return ""


def workspace_package_version(crate: str) -> str:
    metadata = json.loads(run(["cargo", "metadata", "--format-version=1", "--no-deps"], capture=True))
    workspace_members = set(metadata["workspace_members"])
    for package in metadata["packages"]:
        if package["id"] in workspace_members and package["name"] == crate:
            return package["version"]
    raise SystemExit(f"workspace package not found: {crate}")


def workspace_patch_args(crate: str, *, include_dev_deps: bool) -> list[str]:
    metadata = json.loads(run(["cargo", "metadata", "--format-version=1", "--no-deps"], capture=True))
    workspace_members = set(metadata["workspace_members"])
    packages = {
        package["name"]: package for package in metadata["packages"] if package["id"] in workspace_members
    }
    if include_dev_deps:
        selected = {name for name in packages if name != crate}
    else:
        selected: set[str] = set()
        stack = [crate]
        while stack:
            package = packages[stack.pop()]
            for dependency in package["dependencies"]:
                if dependency.get("kind") == "dev":
                    continue
                dependency_name = dependency["name"]
                if dependency_name == crate or dependency_name not in packages:
                    continue
                if dependency_name in selected:
                    continue
                selected.add(dependency_name)
                stack.append(dependency_name)

    args: list[str] = []
    for name in sorted(selected):
        package = packages[name]
        package_dir = Path(package["manifest_path"]).parent.resolve().as_posix()
        args.append(f'--config=patch.crates-io.{package["name"]}.path="{package_dir}"')
    return args


def safe_extract(archive: Path, destination: Path) -> None:
    destination_resolved = destination.resolve()
    with tarfile.open(archive, "r:*") as tar:
        for member in tar.getmembers():
            target = (destination / member.name).resolve()
            try:
                target.relative_to(destination_resolved)
            except ValueError as error:
                raise SystemExit(f"refusing to extract path outside destination: {member.name}") from error
        tar.extractall(destination)


def strip_dev_dependencies(manifest: Path) -> None:
    text = manifest.read_text(encoding="utf-8")
    stripped = re.sub(
        r"^\[dev-dependencies[^\]]*\].*?(?=^\[(?!dev-dependencies)|\Z)",
        "",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    manifest.write_text(stripped, encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate that perl-lsp-rs-core is self-contained when packaged.",
    )
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="pass --allow-dirty to cargo package for local pre-commit validation",
    )
    return parser.parse_args()


def package_args(
    *extra: str,
    allow_dirty: bool,
    patch_args: list[str] | None = None,
    no_verify: bool = False,
) -> list[str]:
    patch_args = patch_args or []
    args = ["cargo", "package", "-p", CRATE, "--locked", *patch_args, *extra]
    if no_verify:
        args.append("--no-verify")
    if allow_dirty:
        args.append("--allow-dirty")
    return args


def main() -> int:
    args = parse_args()
    version = workspace_package_version(CRATE)
    package_patch_args = workspace_patch_args(CRATE, include_dev_deps=True)
    check_patch_args = workspace_patch_args(CRATE, include_dev_deps=False)

    package_listing = run(package_args("--list", allow_dirty=args.allow_dirty), capture=True)
    package_files = {line.strip().replace("\\", "/") for line in package_listing.splitlines()}
    if REQUIRED_PACKAGE_FILE not in package_files:
        print(
            f"ERROR: {CRATE} package is missing {REQUIRED_PACKAGE_FILE}",
            file=sys.stderr,
        )
        return 1

    print(f"OK: package list includes {REQUIRED_PACKAGE_FILE}")

    run(package_args(allow_dirty=args.allow_dirty, patch_args=package_patch_args, no_verify=True))

    crate_archive = ROOT / "target" / "package" / f"{CRATE}-{version}.crate"
    if not crate_archive.exists():
        print(f"ERROR: expected packaged crate not found: {crate_archive}", file=sys.stderr)
        return 1

    smoke_root = Path(tempfile.mkdtemp(prefix=f"{CRATE}-package-smoke-"))
    print(f"package smoke root: {smoke_root}")
    safe_extract(crate_archive, smoke_root)

    extracted = smoke_root / f"{CRATE}-{version}"
    if not (extracted / REQUIRED_PACKAGE_FILE).exists():
        print(
            f"ERROR: unpacked {CRATE} package is missing {REQUIRED_PACKAGE_FILE}",
            file=sys.stderr,
        )
        return 1

    strip_dev_dependencies(extracted / "Cargo.toml")
    run(
        [
            "cargo",
            "generate-lockfile",
            "--manifest-path",
            str(extracted / "Cargo.toml"),
            *check_patch_args,
        ],
    )
    run(
        [
            "cargo",
            "check",
            "--manifest-path",
            str(extracted / "Cargo.toml"),
            "--lib",
            "--locked",
            *check_patch_args,
        ],
    )

    print(f"OK: {CRATE}-{version}.crate is self-contained")
    shutil.rmtree(smoke_root, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
