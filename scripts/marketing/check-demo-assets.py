#!/usr/bin/env python3
"""Report the planned demo assets and validate the capture plan."""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11 fallback
    import tomli as tomllib  # type: ignore[no-redef]


@dataclass(frozen=True)
class Asset:
    name: str
    priority: str
    kind: str
    target: Path
    storyboard: Path
    recording: Path
    source_files: list[Path]
    notes: str


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def load_assets(manifest: Path) -> list[Asset]:
    with manifest.open("rb") as handle:
        payload = tomllib.load(handle)

    assets: list[Asset] = []
    for entry in payload.get("asset", []):
        assets.append(
            Asset(
                name=entry["name"],
                priority=entry["priority"],
                kind=entry["kind"],
                target=Path(entry["target"]),
                storyboard=Path(entry["storyboard"]),
                recording=Path(entry["recording"]),
                source_files=[Path(path) for path in entry.get("source_files", [])],
                notes=entry.get("notes", ""),
            )
        )
    return assets


def format_status(asset: Asset, root: Path) -> str:
    target = root / asset.target
    storyboard = root / asset.storyboard
    recording = root / asset.recording

    target_state = "present" if target.exists() else "missing"
    recording_state = "present" if recording.exists() else "missing"
    storyboard_state = "present" if storyboard.exists() else "missing"

    return (
        f"{asset.priority} {asset.name}: target={target_state}, "
        f"recording={recording_state}, storyboard={storyboard_state}"
    )


def validate_assets(assets: list[Asset], root: Path) -> list[str]:
    errors: list[str] = []
    expected = {"install-health", "find-references", "extract-variable"}

    names = {asset.name for asset in assets}
    if names != expected:
        errors.append(
            "manifest should contain exactly the three P0 walkthrough assets "
            f"{sorted(expected)}, found {sorted(names)}"
        )

    for asset in assets:
        if asset.kind != "gif":
            errors.append(f"{asset.name}: expected kind=gif, got {asset.kind!r}")
        if not (root / asset.storyboard).exists():
            errors.append(f"{asset.name}: missing storyboard {asset.storyboard}")
        for source in asset.source_files:
            if not (root / source).exists():
                errors.append(f"{asset.name}: missing source file {source}")

    return errors


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Report the current demo-asset plan and validate the P0 walkthrough tracker.",
    )
    parser.add_argument(
        "--manifest",
        default="docs/assets/demo-asset-plan.toml",
        help="Path to the asset-plan manifest (default: docs/assets/demo-asset-plan.toml).",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Validate the manifest and referenced source/storyboard files.",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    root = repo_root()
    manifest = root / args.manifest

    if not manifest.exists():
        print(f"error: manifest not found: {manifest}", file=sys.stderr)
        return 2

    assets = load_assets(manifest)

    for asset in assets:
        print(format_status(asset, root))
        print(f"  target:     {asset.target}")
        print(f"  storyboard: {asset.storyboard}")
        print(f"  recording:  {asset.recording}")
        if asset.notes:
            print(f"  notes:      {asset.notes}")

    if args.check:
        errors = validate_assets(assets, root)
        if errors:
            print("\nvalidation errors:", file=sys.stderr)
            for error in errors:
                print(f"- {error}", file=sys.stderr)
            return 1
        print("\nmanifest check passed")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
