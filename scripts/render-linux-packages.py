#!/usr/bin/env python3
"""Render Linux package manager templates from the repo metadata file.

This script turns the checked-in Linux packaging templates into a concrete
rendered tree for a specific release version, download base URL, checksum, and
target architecture. It is intentionally small and bounded: it renders the
three first-party package manager templates in `distribution/linux/` and stops
if any placeholder tokens remain.
"""

from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path
from typing import Mapping

try:
    import tomllib
except ModuleNotFoundError as exc:  # pragma: no cover - Python 3.10 fallback
    raise SystemExit("Python 3.11+ is required for tomllib") from exc


REPO_ROOT = Path(__file__).resolve().parents[1]
TEMPLATE_ROOT = REPO_ROOT / "distribution" / "linux"
DEFAULT_METADATA = TEMPLATE_ROOT / "package-metadata.toml"
TOKEN_PATTERN = re.compile(r"__[A-Z0-9_]+__")

ARCH_MAP = {
    "x86_64": {"deb": "amd64", "rpm": "x86_64", "pacman": "x86_64"},
    "aarch64": {"deb": "arm64", "rpm": "aarch64", "pacman": "aarch64"},
}

TEMPLATE_TARGETS = {
    TEMPLATE_ROOT / "apt" / "control.in": "apt/control",
    TEMPLATE_ROOT / "dnf" / "perl-lsp.spec.in": "dnf/perl-lsp.spec",
    TEMPLATE_ROOT / "pacman" / "PKGBUILD.in": "pacman/PKGBUILD",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render Linux package templates from package-metadata.toml.",
    )
    parser.add_argument(
        "--metadata",
        type=Path,
        default=DEFAULT_METADATA,
        help="Path to package-metadata.toml.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Directory to write rendered package files into.",
    )
    parser.add_argument(
        "--version",
        required=True,
        help="Release version to stamp into the rendered files.",
    )
    parser.add_argument(
        "--download-base",
        required=True,
        help="Release download base URL, e.g. https://.../releases/download/v0.12.0",
    )
    parser.add_argument(
        "--download-sha256",
        required=True,
        help="SHA256 for the selected release archive.",
    )
    parser.add_argument(
        "--arch",
        choices=sorted(ARCH_MAP),
        default="x86_64",
        help="Architecture to render (affects package-manager-specific arch names).",
    )
    return parser.parse_args()


def load_metadata(path: Path) -> Mapping[str, str]:
    with path.open("rb") as fh:
        data = tomllib.load(fh)

    required = [
        "package",
        "display_name",
        "summary",
        "homepage",
        "license",
        "maintainer",
        "linux_gnu_x86_64_asset",
        "linux_gnu_aarch64_asset",
    ]
    missing = [key for key in required if key not in data]
    if missing:
        raise SystemExit(f"{path}: missing required keys: {', '.join(missing)}")

    return data


def render_asset_name(metadata: Mapping[str, str], arch: str, version: str) -> str:
    asset_key = f"linux_gnu_{arch}_asset"
    template = metadata[asset_key]
    return template.replace("__RELEASE_VERSION__", version)


def build_replacements(
    metadata: Mapping[str, str],
    arch: str,
    version: str,
    download_base: str,
    sha256: str,
) -> dict[str, str]:
    asset = render_asset_name(metadata, arch, version)
    source_dir = asset.removesuffix(".tar.gz")
    package_arch = ARCH_MAP[arch]
    download_url = f"{download_base.rstrip('/')}/{asset}"

    return {
        "__PACKAGE_NAME__": metadata["package"],
        "__PACKAGE_DISPLAY_NAME__": metadata["display_name"],
        "__PACKAGE_SUMMARY__": metadata["summary"],
        "__PACKAGE_HOMEPAGE__": metadata["homepage"],
        "__PACKAGE_LICENSE__": metadata["license"],
        "__PACKAGE_MAINTAINER__": metadata["maintainer"],
        "__PACKAGE_DESCRIPTION_LINE_1__": "perl-lsp is a native Perl language server written in Rust.",
        "__PACKAGE_DESCRIPTION_LINE_2__": "This package installs the perl-lsp and perl-dap executables from the release artifact.",
        "__RELEASE_VERSION__": version,
        "__SOURCE_TARBALL__": asset,
        "__SOURCE_DIR__": source_dir,
        "__DOWNLOAD_URL__": download_url,
        "__DOWNLOAD_SHA256__": sha256,
        "__DEB_ARCH__": package_arch["deb"],
        "__RPM_ARCH__": package_arch["rpm"],
        "__PACMAN_ARCH__": package_arch["pacman"],
    }


def render_text(text: str, replacements: Mapping[str, str], source: Path) -> str:
    rendered = text
    for token, value in replacements.items():
        rendered = rendered.replace(token, value)

    unresolved = sorted(set(TOKEN_PATTERN.findall(rendered)))
    if unresolved:
        raise SystemExit(f"{source}: unresolved tokens remain: {', '.join(unresolved)}")

    return rendered


def render_templates(
    template_root: Path,
    output_dir: Path,
    replacements: Mapping[str, str],
) -> list[Path]:
    rendered_paths: list[Path] = []
    for template_path, relative_output in TEMPLATE_TARGETS.items():
        source = template_root / template_path.relative_to(TEMPLATE_ROOT)
        target = output_dir / relative_output
        target.parent.mkdir(parents=True, exist_ok=True)
        rendered = render_text(source.read_text(encoding="utf-8"), replacements, source)
        target.write_text(rendered, encoding="utf-8", newline="\n")
        rendered_paths.append(target)
    return rendered_paths


def main() -> int:
    args = parse_args()
    metadata = load_metadata(args.metadata)
    replacements = build_replacements(
        metadata=metadata,
        arch=args.arch,
        version=args.version,
        download_base=args.download_base,
        sha256=args.download_sha256,
    )

    if args.output_dir.exists():
        if args.output_dir.is_file():
            raise SystemExit(f"{args.output_dir}: output path exists and is not a directory")
    else:
        args.output_dir.mkdir(parents=True, exist_ok=True)

    rendered = render_templates(TEMPLATE_ROOT, args.output_dir, replacements)
    print(f"Rendered {len(rendered)} files into {args.output_dir}")
    for path in rendered:
        print(f"- {path.relative_to(args.output_dir)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
