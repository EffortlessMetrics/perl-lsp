#!/usr/bin/env python3
"""List SRP-style microcrates in this workspace.

A crate is classified as an SRP microcrate when at least one of these signals is present:
- Cargo description contains "single responsibility" or "microcrate".
- README contains "single responsibility", "SRP", or "microcrate".
- src/lib.rs module docs contain "single responsibility" or "microcrate".

The output is grouped by crate family prefixes to make extraction and maintenance easier.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES_DIR = REPO_ROOT / "crates"

SIGNAL_PATTERNS = [
    re.compile(r"\bsingle responsibility\b", re.IGNORECASE),
    re.compile(r"\bmicrocrate\b", re.IGNORECASE),
    re.compile(r"\bSRP\b", re.IGNORECASE),
]

FAMILY_PREFIXES = [
    "perl-module-",
    "perl-lsp-feature-",
    "perl-lsp-",
    "perl-dap-",
    "perl-workspace-",
    "perl-ts-",
    "perl-",
]


@dataclass
class CrateMatch:
    name: str
    signals: list[str]


def _read_text(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8")


def _has_signal(text: str) -> bool:
    return any(pattern.search(text) for pattern in SIGNAL_PATTERNS)


def find_srp_microcrates() -> list[CrateMatch]:
    matches: list[CrateMatch] = []

    for crate_dir in sorted(p for p in CRATES_DIR.iterdir() if p.is_dir()):
        crate_name = crate_dir.name
        if not crate_name.startswith("perl-"):
            continue
        cargo_toml = _read_text(crate_dir / "Cargo.toml")
        readme = _read_text(crate_dir / "README.md")
        lib_rs = _read_text(crate_dir / "src/lib.rs")

        crate_signals: list[str] = []
        if _has_signal(cargo_toml):
            crate_signals.append("Cargo.toml")
        if _has_signal(readme):
            crate_signals.append("README.md")
        if _has_signal(lib_rs):
            crate_signals.append("src/lib.rs")

        if crate_signals:
            matches.append(CrateMatch(name=crate_name, signals=crate_signals))

    return matches


def family_key(crate_name: str) -> tuple[int, str]:
    for idx, prefix in enumerate(FAMILY_PREFIXES):
        if crate_name.startswith(prefix):
            return idx, crate_name
    return len(FAMILY_PREFIXES), crate_name


def main() -> None:
    matches = find_srp_microcrates()
    grouped: dict[str, list[CrateMatch]] = {}

    for match in sorted(matches, key=lambda m: family_key(m.name)):
        family = "core/misc"
        for prefix in FAMILY_PREFIXES:
            if prefix != "perl-" and match.name.startswith(prefix):
                family = prefix
                break
        grouped.setdefault(family, []).append(match)

    print("# SRP microcrate inventory")
    print()
    print(f"Detected {len(matches)} crates with SRP/microcrate signals.")
    print()

    for family in sorted(grouped.keys()):
        crates = grouped[family]
        print(f"## {family} ({len(crates)})")
        for crate in crates:
            sources = ", ".join(crate.signals)
            print(f"- {crate.name} ({sources})")
        print()


if __name__ == "__main__":
    main()
